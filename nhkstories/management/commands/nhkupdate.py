import re
import os
import json
from datetime import datetime, timedelta, date, timezone
from tempfile import mkstemp
from subprocess import run, DEVNULL
from urllib.request import urlopen, HTTPError

from django.core.management.base import BaseCommand
from django.conf import settings

from nhkstories.models import Story
from nhkstories.edict.subedict import create_subedict, create_subenamdict, save_subedict


class DuplicateStoryIDType(Exception):
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


def fetch_story_list():
    '''Return a dictionary mapping days to stories published this day'''
    with urlopen(story_list_url) as f:
        data = f.read()
    return json.loads(data.decode('utf-8-sig'))[0]


def fetch_replace_voice():
    '''Return a dictionary mapping story_id to amended voice filename'''
    with urlopen(replace_voice_url) as f:
        data = f.read()
    amendments = json.loads(data.decode())
    return {
        amendment['news_id']: amendment['voice_id']
        for amendment in amendments
    }


def set_voice_id(info, replace_voice):
    news_id = info['news_id']
    info['voice_id'] = replace_voice.get(news_id, news_id)


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
    story, created = Story.objects.get_or_create(story_id=info['news_id'])
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
        return
    webpage_url = webpage_url_pattern.format(**info)
    print('Download %s' % webpage_url)
    with urlopen(webpage_url) as f:
        story.webpage.save('', f)


def fetch_story_image(story, info):
    if story.image:
        return

    assert not (info['has_news_web_image'] and info['has_news_easy_image'])

    if info['has_news_web_image']:
        image_url = info['news_web_image_uri']
    elif info['has_news_easy_image']:  # rare
        image_url = image_url_pattern.format(**info)
    else:
        return

    print('Download image %s' % image_url)
    try:
        with urlopen(image_url) as f:
            story.image.save('', f)
    except HTTPError:
        print('Failed')
        return

    run(['mogrify', '-interlace', 'plane', story.image.file.name], check=True)


def fetch_story_voice(story, info):
    if story.voice:
        return

    if info['has_news_easy_voice']:
        voice_url = voice_url_pattern.format(**info)
    else:
        return

    if voice_url.endswith('.mp4'):
        # fragmented MP4 using HTTP Live Streaming
        voice_url = fragmented_voice_url_pattern.format(**info)
        print('Download voice (fragmented MP4) %s' % voice_url)
        _, temp_name = mkstemp(suffix='.mp3')
        res = run(['ffmpeg', '-y', '-i', voice_url, temp_name], stderr=DEVNULL)
        if res.returncode == 0:
            with open(temp_name, 'rb') as f:
                story.voice.save('', f)
        else:
            print('Failed')
        os.remove(temp_name)
    else:
        print('Download voice %s' % voice_url)
        try:
            with urlopen(voice_url) as f:
                story.voice.save('', f)
        except HTTPError:
            print('Failed')


def fetch_story_video(story, info):
    if story.video_original:
        return

    if info['has_news_web_movie']:
        video_url = video_url_pattern.format(**info)
    else:
        return

    print('Download video %s' % video_url)
    _, temp = mkstemp()
    # some download complete partially, so we try several times
    for _ in range(2):
        res = run(['rtmpdump', '-r', video_url, '-o', temp], stderr=DEVNULL)
        if res.returncode != 2:
            break
    # some videos always trigger a partial download so we keep what we have
    if res.returncode in (0, 2):
        with open(temp, 'rb') as f:
            story.video_original.save('', f)
    else:
        print('Failed')
    os.remove(temp)


def extract_story_content(story):
    data = story.webpage.read().decode()
    story.webpage.seek(0)  # the webpage might be read when updating story
    m = re.search(r'(?s)<div class="article-main__body article-body" id="js-article-body">(.*?)            </div>', data)
    if m is None:
        m = re.search(r'(?s)<div id="newsarticle">(.*?)</div>', data)
    raw_content = m.group(1)
    story.content_with_ruby = clean_up_content(raw_content)
    story.content = remove_ruby(story.content_with_ruby)


def convert_story_video(story):
    if story.video_reencoded:
        return

    if not story.video_original:
        return
    print('Converting %s' % story.video_original.name)
    original = story.video_original.file.name
    _, temp = mkstemp(suffix='.mp4')
    run(['ffmpeg', '-y', '-i', original, '-b:v', '500k', temp], stderr=DEVNULL, check=True)
    with open(temp, 'rb') as f:
        story.video_reencoded.save('', f)
    os.remove(temp)


def fetch_story(info, replace_voice):
    set_voice_id(info, replace_voice)
    story, created = story_from_info(info)
    fetch_story_webpage(story, info)
    fetch_story_voice(story, info)
    if (date.today() - story.published.date()).days <= 7:
        fetch_story_image(story, info)
        fetch_story_video(story, info)
    extract_story_content(story)
    convert_story_video(story)
    story.save()
    return created


def fetch_stories():
    stories_per_day = fetch_story_list()
    replace_voice = fetch_replace_voice()
    new_stories_count = sum(
        fetch_story(story, replace_voice)
        for day in sorted(stories_per_day)
        for story in stories_per_day[day]
    )

    if new_stories_count == 0:
        print('No new stories')
    elif new_stories_count == 1:
        print('1 new story')
    else:
        print('%i new stories' % new_stories_count)


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
    stories = Story.objects.filter(subedict_created=False)

    # create sub EDICT files for stories and list days that must be updated
    new_days = set()
    for story in stories:
        new_days.add(story.published.date())
        filename = '{:05}.dat'.format(story.id)
        subedict_from_content(filename, story.content)
        subenamdict_from_content(filename, story.content)
        print(filename)
    print('Story-wise sub EDICT files updated')

    # update sub EDICT files for days
    for day in sorted(new_days):
        day_stories = Story.objects.filter(published__date=day)
        content = ''.join(story.content for story in day_stories)
        filename = '{}.dat'.format(day)
        subedict_from_content(filename, content)
        subenamdict_from_content(filename, content)
        print(filename)
    print('Day-wise sub EDICT files updated')

    # note that the subedict have been generated for those stories
    stories.update(subedict_created=True)


class Command(BaseCommand):
    def handle(self, *args, **options):
        fetch_stories()
        create_subedicts()
