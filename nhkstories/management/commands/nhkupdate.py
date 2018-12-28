import re
import os
import json
import logging
from datetime import datetime, timedelta, date, timezone
from tempfile import mkstemp
from subprocess import run, DEVNULL
from urllib.request import urlopen, HTTPError

from django.core.management.base import BaseCommand
from django.conf import settings

from nhkstories.logging import init_logging
from nhkstories.models import Story
from nhkstories.edict.subedict import create_subedict, create_subenamdict, save_subedict

logger = logging.getLogger(__name__)


class DuplicateStoryIDType(Exception):
    pass


class ContentNotFound(Exception):
    pass


class RegularAndNHKVideos(Exception):
    pass


DuplicateStoryID = DuplicateStoryIDType()

BASE_URL = 'http://www3.nhk.or.jp/news/easy/'
story_list_url = BASE_URL + 'news-list.json'
replace_voice_url = BASE_URL + 'player/voice_replace/voice_replace.json'
webpage_url_pattern = BASE_URL + '{news_id}/{news_id}.html'
image_url_pattern = BASE_URL + '{news_id}/{news_easy_image_uri}'
voice_url_pattern = BASE_URL + '{voice_id}/{news_easy_voice_uri}'
fragmented_voice_url_pattern = 'https://nhks-vh.akamaihd.net/i/news/easy/{voice_id}.mp4/master.m3u8'
video_url_pattern = 'rtmp://flv.nhk.or.jp/ondemand/flv/news/{news_web_movie_uri}'
nhk_contents = 'https://www3.nhk.or.jp/news/contents/easy/'


def fetch_story_list():
    '''Return a dictionary mapping days to stories published this day'''
    logger.debug('Fetching list of stories')
    with urlopen(story_list_url) as f:
        data = f.read()
    stories_per_day = json.loads(data.decode('utf-8-sig'))[0]

    n_stories = sum(len(stories_per_day[day]) for day in stories_per_day)
    n_days = len(stories_per_day)
    logger.debug('{} stories over {} days found'.format(n_stories, n_days))
    return stories_per_day


def fetch_replace_voice():
    '''Return a dictionary mapping story_id to amended voice filename'''
    logger.debug('Fetching voice amendments')
    with urlopen(replace_voice_url) as f:
        data = f.read()
    amendments = json.loads(data.decode())
    logger.debug('{} voice amendments found'.format(len(amendments)))
    return {
        amendment['news_id']: amendment['voice_id']
        for amendment in amendments
    }


def set_voice_id(info, replace_voice):
    news_id = info['news_id']
    if news_id in replace_voice:
        info['voice_id'] = replace_voice[news_id]
        logger.debug('Amending voice_id ({})'.format(info['voice_id']))
    else:
        info['voice_id'] = news_id
        logger.debug('Copying voice_id ({})'.format(info['voice_id']))


def clean_up_content(content):
    content = re.sub("<a.*?>", '', content)
    content = re.sub('<span.*?>', '', content)
    content = content.replace('</a>', '')
    content = content.replace('</span>', '')
    content = content.replace('<p></p>', '')
    content = content.strip()
    return content


def remove_ruby(content):
    content = re.sub('<rp>.*?</rp>', '', content)
    content = re.sub('<rt>.*?</rt>', '', content)
    content = re.sub('<rtc>.*?</rtc>', '', content)
    content = content.replace('<rb>', '')
    content = content.replace('</rb>', '')
    content = content.replace('</ruby>', '')
    content = content.replace('<ruby>', '')
    return content


def parse_datetime_nhk(s):
    dt = datetime.strptime(s, '%Y-%m-%d %H:%M:%S')
    jst = timezone(timedelta(hours=9))
    return dt.replace(tzinfo=jst)


def story_from_info(info):
    logger.debug('Extracting story info')
    story, created = Story.objects.get_or_create(story_id=info['news_id'])
    if created:
        logger.debug('Inserted into database (id={})'.format(story.id))
    else:
        logger.debug('Retrieved from database (id={})'.format(story.id))

    published = parse_datetime_nhk(info['news_prearranged_time'])
    if story.published and abs(story.published - published).days > 2:
        # probably a reused story_id, not implemented yet
        raise DuplicateStoryID
    story.published = published
    story.title = info['title']
    story.title_with_ruby = info['title_with_ruby']
    assert remove_ruby(story.title_with_ruby) == story.title
    return story, created


def fetch_story_webpage(story, info):
    if story.webpage:
        logger.debug('Webpage already present')
        return
    logger.debug('Fetching webpage')
    webpage_url = webpage_url_pattern.format(**info)
    logger.info('Download %s' % webpage_url)
    with urlopen(webpage_url) as f:
        story.webpage.save('', f)


def fetch_story_image(story, info):
    if story.image:
        logger.debug('Image already present')
        return

    logger.debug('Fetching image')
    assert not (info['has_news_web_image'] and info['has_news_easy_image'])

    if info['has_news_web_image']:
        logger.debug('News Web image')
        image_url = info['news_web_image_uri']
    elif info['has_news_easy_image']:  # rare
        logger.debug('News Web Easy image')
        image_url = image_url_pattern.format(**info)
    else:
        logger.debug('No image')
        return

    logger.info('Download image %s' % image_url)
    try:
        with urlopen(image_url) as f:
            story.image.save('', f)
    except HTTPError:
        logger.warning('Failed to fetch image')
        return
    logger.debug('Image saved')

    logger.debug('Converting image')
    run(['mogrify', '-interlace', 'plane', story.image.file.name], check=True)


def fetch_story_voice(story, info):
    if story.voice:
        logger.debug('Voice already present')
        return

    if not info['has_news_easy_voice']:
        logger.debug('No voice')
        return

    voice_url = voice_url_pattern.format(**info)
    logger.debug('Voice found')
    if voice_url.endswith('.mp4'):
        # fragmented MP4 using HTTP Live Streaming
        voice_url = fragmented_voice_url_pattern.format(**info)
        logger.info('Download voice (fragmented MP4) %s' % voice_url)
        _, temp_name = mkstemp(suffix='.mp3')
        res = run(['ffmpeg', '-y', '-i', voice_url, temp_name], stderr=DEVNULL)
        if res.returncode == 0:
            logger.debug('Fragmented voice fetched successfully')
            with open(temp_name, 'rb') as f:
                story.voice.save('', f)
            logger.debug('Voice saved')
        else:
            logger.warning('Failed to download fragmented voice')
        os.remove(temp_name)
    else:
        logger.info('Download voice %s' % voice_url)
        try:
            with urlopen(voice_url) as f:
                story.voice.save('', f)
        except HTTPError:
            logger.warning('Failed to download voice')
        else:
            logger.debug('Voice saved')


def fetch_story_video(story, info):
    if story.video_original:
        logger.debug('Original video already present')
        return

    if not info['has_news_web_movie']:
        logger.debug('Story has no video')
        return

    video_url = video_url_pattern.format(**info)
    logger.info('Download video %s' % video_url)
    _, temp = mkstemp()
    # some download complete partially, so we try several times
    for _ in range(2):
        logger.debug('Trying to read RTMP stream')
        res = run(['rtmpdump', '-r', video_url, '-o', temp], stderr=DEVNULL)
        if res.returncode != 2:
            break
    # some videos always trigger a partial download so we keep what we have
    if res.returncode in (0, 2):
        logger.debug('Stream read successfully')
        with open(temp, 'rb') as f:
            story.video_original.save('', f)
        logger.debug('Video saved')
    else:
        logger.warning('Failed to fetch video')
    os.remove(temp)


def extract_story_content(story):
    if story.content_with_ruby:
        logger.debug('Content already present')
        return

    logger.debug('Extracting content')
    data = story.webpage.read().decode()
    story.webpage.seek(0)  # the webpage might be read when updating story

    logger.debug('Parsing {} characters'.format(len(data)))
    m = re.search(r'(?s)<div class="article-main__body article-body" id="js-article-body">(.*?)            </div>', data)
    if m is None:
        logger.error('Could not find content')
        raise ContentNotFound

    raw_content = m.group(1)
    logger.debug('Parsed content ({} characters)'.format(len(raw_content)))
    story.content_with_ruby = clean_up_content(raw_content)
    logger.debug('Cleaned up ({} characters)'.format(len(story.content_with_ruby)))
    story.content = remove_ruby(story.content_with_ruby)
    logger.debug('Removed ruby ({} characters)'.format(len(story.content)))


def fetch_story_nhk_video(story):
    if story.video_reencoded:
        logger.debug('Web video already present')
        return

    logger.debug('Fetching NHK video')
    content = story.content_with_ruby

    # extract iframe URL
    html_match = re.search(r'(?s)<div.*?src="(.*?)".*?</div>\s*', content)
    if not html_match:
        logger.debug('No NHK video found')
        return
    iframe_url = html_match.group(1)
    logger.debug('Found iframe (URL={})', iframe_url)

    if story.video_original:
        logger.error('Story has both regular and NHK videos')
        raise RegularAndNHKVideos

    # extract JSON URL
    with urlopen(iframe_url) as f:
        data = f.read()
    json_match = re.search(r'player\("(.*?)"\)', data.decode())
    if not json_match:
        logger.error('Failed to find JSON filename of NHK video')
        return
    json_filename = json_match.group(1)
    logger.debug('Found JSON filename ({})'.format(json_filename))

    # fetch JSON
    json_url = nhk_contents + json_filename
    with urlopen(json_url) as f:
        data = f.read()
    info = json.loads(data.decode())
    video_url = info['mediaResource']['url']
    logger.debug('Found NHK video URL ({})'.format(video_url))

    # fetch video
    logger.info('Fetching NHK video {}'.format(video_url))
    _, temp_name = mkstemp(suffix='.mp4')
    res = run(['ffmpeg', '-y', '-i', video_url, temp_name], stderr=DEVNULL)
    if res.returncode != 0:
        logger.warning('Failed to fetch NHK video')
        os.remove(temp_name)
        return
    logger.debug('Fetched video')

    # save video
    with open(temp_name, 'rb') as f:
        story.video_reencoded.save('', f)
    logger.debug('Video saved')

    # remove HTML object
    logger.debug('Replacing NHK video iframe')
    new_ruby = content[:html_match.start()] + content[html_match.end():]
    delta = len(new_ruby) - len(story.content_with_ruby)
    logger.debug('Updating content ({:+} characters)'.format(delta))
    story.content_with_ruby = new_ruby
    new_content = remove_ruby(story.content_with_ruby)
    delta2 = len(new_content) - len(story.content)
    assert delta == delta2
    story.content = remove_ruby(story.content_with_ruby)
    story.save()
    logger.debug('Content updated')


def convert_story_video(story):
    if story.video_reencoded:
        logger.debug('Web video already present')
        return

    if not story.video_original:
        logger.debug('No video')
        return

    logger.info('Converting %s' % story.video_original.name)
    original = story.video_original.file.name
    _, temp = mkstemp(suffix='.mp4')
    run(['ffmpeg', '-y', '-i', original, '-b:v', '500k', temp], stderr=DEVNULL, check=True)
    logger.debug('Video converted')
    with open(temp, 'rb') as f:
        story.video_reencoded.save('', f)
    logger.debug('Video saved')
    os.remove(temp)


def fetch_story(info, replace_voice):
    logger.debug('Fetching story {} ({})'.format(info['news_id'], info['title']))
    set_voice_id(info, replace_voice)
    story, created = story_from_info(info)
    fetch_story_webpage(story, info)
    fetch_story_voice(story, info)
    if (date.today() - story.published.date()).days <= 7:
        logger.debug('Recent story, fetching media')
        fetch_story_image(story, info)
        fetch_story_video(story, info)
    else:
        logger.debug('Old story, skipping media')
    extract_story_content(story)
    fetch_story_nhk_video(story)
    convert_story_video(story)
    story.save()
    if created:
        logger.debug('Story created')
    else:
        logger.debug('Story updated')
    return created


def fetch_stories():
    logger.debug('Fetching stories')
    stories_per_day = fetch_story_list()
    replace_voice = fetch_replace_voice()
    new_stories_count = sum(
        fetch_story(story, replace_voice)
        for day in sorted(stories_per_day)
        for story in stories_per_day[day]
    )

    if new_stories_count == 0:
        logger.info('No new stories')
    elif new_stories_count == 1:
        logger.info('1 new story')
    else:
        logger.info('%i new stories' % new_stories_count)


def subedict_from_content(filename, content):
    subedict_dir = os.path.join(settings.BASE_DIR, 'media', 'subedict')
    os.makedirs(subedict_dir, exist_ok=True)
    path = os.path.join(subedict_dir, filename)
    save_subedict(create_subedict(content), path)


def subenamdict_from_content(filename, content):
    subenamdict_dir = os.path.join(settings.BASE_DIR, 'media', 'subenamdict')
    os.makedirs(subenamdict_dir, exist_ok=True)
    path = os.path.join(subenamdict_dir, filename)
    save_subedict(create_subenamdict(content), path)


def create_subedicts():
    logger.debug('Creating subedicts')
    stories = Story.objects.filter(subedict_created=False)
    logger.debug('Stories without subedicts: {}'.format(len(stories)))

    # create sub EDICT files for stories and list days that must be updated
    logger.debug('Creating story subedicts')
    new_days = set()
    for story in stories:
        logger.debug('Considering story id={}'.format(story.id))
        new_days.add(story.published.date())
        filename = '{:05}.dat'.format(story.id)
        subedict_from_content(filename, story.content)
        subenamdict_from_content(filename, story.content)
        logger.info(filename)
    logger.info('Story-wise sub EDICT files updated')

    # update sub EDICT files for days
    logger.debug('Creating day subedicts')
    for day in sorted(new_days):
        logger.debug('Considering day {}'.format(day))
        day_stories = Story.objects.filter(published__date=day)
        content = ''.join(story.content for story in day_stories)
        filename = '{}.dat'.format(day)
        subedict_from_content(filename, content)
        subenamdict_from_content(filename, content)
        logger.info(filename)
    logger.info('Day-wise sub EDICT files updated')

    # note that the subedict have been generated for those stories
    stories.update(subedict_created=True)
    logger.debug('Subedicts created')


def main():
    init_logging()
    logger.debug('Start of NHKUpdate command')
    fetch_stories()
    create_subedicts()
    logger.debug('End of NHKUpdate command')


class Command(BaseCommand):
    def handle(self, *args, **options):
        try:
            main()
        except Exception:
            logger.exception('NHKUPDATE GENERAL FAILURE')
