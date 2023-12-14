import json
import logging
import os
import re
from datetime import datetime, timedelta, timezone
from subprocess import DEVNULL, run
from tempfile import mkstemp
from typing import Any, Dict, List, NewType, Tuple
from urllib.error import HTTPError
from urllib.parse import urlparse, urlunparse
from urllib.request import Request, urlopen

from django.conf import settings
from django.core.files.base import ContentFile
from django.core.management.base import BaseCommand

from nhkeasier.edict.subedict import create_subedict, create_subenamdict, save_subedict
from nhkeasier.logging import init_logging
from nhkeasier.models import Story

logger = logging.getLogger(__name__)
StoryInfo = NewType('StoryInfo', Dict)


class DuplicateStoryIDError(Exception):
    pass


class ContentNotFoundError(Exception):
    pass


class RegularAndNHKVideosError(Exception):
    pass


BASE_URL = 'http://www3.nhk.or.jp/news/easy/'
story_list_url = BASE_URL + 'news-list.json'
replace_voice_url = BASE_URL + 'player/voice_replace/voice_replace.json'
webpage_url_pattern = BASE_URL + '{news_id}/{news_id}.html'
image_url_pattern = BASE_URL + '{news_id}/{news_easy_image_uri}'
voice_url_pattern = BASE_URL + '{voice_id}/{news_easy_voice_uri}'
fragmented_voice_url_pattern = 'https://nhks-vh.akamaihd.net/i/news/easy/{voice_id}.mp4/master.m3u8'
video_url_pattern = 'rtmp://flv.nhk.or.jp/ondemand/flv/news/{news_web_movie_uri}'
nhk_contents = 'https://www3.nhk.or.jp/news/contents/easy/'


def remove_extra_dots(url: str) -> str:
    # inspired from https://stackoverflow.com/a/27950825
    parsed = urlparse(url)
    dirs: List[str] = []
    for name in parsed.path.split('/'):
        if name == '..':
            if len(dirs) > 1:
                dirs.pop()
        else:
            dirs.append(name)
    new_path = '/'.join(dirs)
    return urlunparse(parsed._replace(path=new_path))


assert remove_extra_dots('http://example.com/a/../b') == 'http://example.com/b'
assert remove_extra_dots('http://example.com/a/../b/') == 'http://example.com/b/'
assert remove_extra_dots('http://example.com/a/../b/c.jpg') == 'http://example.com/b/c.jpg'
assert remove_extra_dots('http://example.com/a/../b/c.jpg?aze') == 'http://example.com/b/c.jpg?aze'


def fetch(url: str) -> bytes:
    url = remove_extra_dots(url)
    request = Request(url, headers={'User-Agent': 'NHKEasier Crawler'})
    with urlopen(request) as f:
        return f.read()  # type: ignore


def fetch_story_list() -> Dict[str, List[StoryInfo]]:
    """Return a dictionary mapping days to stories published this day"""
    logger.debug('Fetching list of stories')
    data = fetch(story_list_url)
    stories_per_day = json.loads(data.decode('utf-8-sig'))[0]

    n_stories = sum(len(stories_per_day[day]) for day in stories_per_day)
    n_days = len(stories_per_day)
    logger.debug(f'{n_stories} stories over {n_days} days found')
    return stories_per_day  # type: ignore


def fetch_replace_voice() -> Dict[str, str]:
    """Return a dictionary mapping story_id to amended voice filename"""
    logger.debug('Fetching voice amendments')
    data = fetch(replace_voice_url)
    amendments = json.loads(data.decode())
    logger.debug(f'{len(amendments)} voice amendments found')
    return {
        amendment['news_id']: amendment['voice_id']
        for amendment in amendments
    }


def set_voice_id(info: StoryInfo, replace_voice: Dict[str, str]) -> None:
    news_id = info['news_id']
    if news_id in replace_voice:
        info['voice_id'] = replace_voice[news_id]
        logger.debug(f'Amending voice_id ({info["voice_id"]})')
    else:
        info['voice_id'] = news_id
        logger.debug(f'Copying voice_id ({info["voice_id"]})')


def clean_up_content(content: str) -> str:
    content = re.sub('<a.*?>', '', content)
    content = re.sub('<span.*?>', '', content)
    content = content.replace('</a>', '')
    content = content.replace('</span>', '')
    content = content.replace('<p></p>', '')
    return content.strip()


def remove_ruby(content: str) -> str:
    content = re.sub('<rp>.*?</rp>', '', content)
    content = re.sub('<rt>.*?</rt>', '', content)
    content = re.sub('<rtc>.*?</rtc>', '', content)
    return re.sub('<.*?>', '', content)


def parse_datetime_nhk(s: str) -> datetime:
    jst = timezone(timedelta(hours=9))
    return datetime.strptime(s, '%Y-%m-%d %H:%M:%S').replace(tzinfo=jst)


def story_from_info(info: StoryInfo) -> Tuple[Story, bool]:
    logger.debug('Extracting story info')
    story, created = Story.objects.get_or_create(story_id=info['news_id'])
    if created:
        logger.debug(f'Inserted into database (id={story.id})')
    else:
        logger.debug(f'Retrieved from database (id={story.id})')

    published = parse_datetime_nhk(info['news_prearranged_time'])
    if story.published and abs(story.published - published).days > 2 and story.title != info['title']:
        # probably a reused story_id, not implemented yet
        raise DuplicateStoryIDError
    story.published = published
    story.title = info['title']
    story.title_with_ruby = info['title_with_ruby']
    assert remove_ruby(story.title_with_ruby) == story.title
    return story, created


def fetch_story_webpage(story: Story, info: StoryInfo) -> None:
    if story.webpage:
        logger.debug('Webpage already present')
        return
    logger.debug('Fetching webpage')
    webpage_url = webpage_url_pattern.format(**info)
    logger.info(f'Download {webpage_url}')
    story.webpage.save('', ContentFile(fetch(webpage_url)))


def fetch_story_image(story: Story, info: StoryInfo) -> None:
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

    logger.info(f'Download image {image_url}')
    try:
        story.image.save('', ContentFile(fetch(image_url)))
    except HTTPError:
        logger.warning('Failed to fetch image', exc_info=True)
        return
    logger.debug('Image saved')

    logger.debug('Converting image')
    run(['mogrify', '-interlace', 'plane', story.image.file.name], check=True)


def fetch_story_voice(story: Story, info: StoryInfo) -> None:
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
        logger.info(f'Download voice (fragmented MP4) {voice_url}')
        _, temp_name = mkstemp(suffix='.mp3')
        res = run(['ffmpeg', '-y', '-i', voice_url, temp_name], stderr=DEVNULL, check=False)
        if res.returncode == 0:
            logger.debug('Fragmented voice fetched successfully')
            with open(temp_name, 'rb') as f:
                story.voice.save('', f)  # type: ignore
            logger.debug('Voice saved')
        else:
            logger.warning('Failed to download fragmented voice')
        os.remove(temp_name)
    else:
        logger.info(f'Download voice {voice_url}')
        try:
            story.voice.save('', ContentFile(fetch(voice_url)))
        except HTTPError:
            logger.warning('Failed to download voice')
        else:
            logger.debug('Voice saved')


def fetch_story_video(story: Story, info: StoryInfo) -> None:
    if story.video_original:
        logger.debug('Original video already present')
        return

    if not info['has_news_web_movie']:
        logger.debug('Story has no video')
        return

    video_url = video_url_pattern.format(**info)
    logger.info(f'Download video {video_url}')
    _, temp = mkstemp()
    # some download complete partially, so we try several times
    for _ in range(2):
        logger.debug('Trying to read RTMP stream')
        res = run(['rtmpdump', '-r', video_url, '-o', temp], stderr=DEVNULL, check=False)
        if res.returncode != 2:
            break
    # some videos always trigger a partial download so we keep what we have
    if res.returncode in (0, 2):
        logger.debug('Stream read successfully')
        with open(temp, 'rb') as f:
            story.video_original.save('', f)  # type: ignore
        logger.debug('Video saved')
    else:
        logger.info('Failed to fetch video')
    os.remove(temp)


def extract_story_content(story: Story) -> None:
    if story.content_with_ruby:
        logger.debug('Content already present')
        return

    logger.debug('Extracting content')
    data = story.webpage.read().decode()
    story.webpage.seek(0)  # the webpage might be read when updating story

    logger.debug(f'Parsing {len(data)} characters')
    m = re.search(
        r'(?s)<div class="article-main__body article-body" id="js-article-body">(.*?)            </div>',
        data,
    )
    if m is None:
        logger.error('Could not find content')
        raise ContentNotFoundError

    raw_content = m.group(1)
    logger.debug(f'Parsed content ({len(raw_content)} characters)')
    story.content_with_ruby = clean_up_content(raw_content)
    logger.debug(f'Cleaned up ({len(story.content_with_ruby)} characters)')
    story.content = remove_ruby(story.content_with_ruby)
    logger.debug(f'Removed ruby ({len(story.content)} characters)')


def fetch_story_nhk_video(story: Story) -> None:
    if story.video_reencoded:
        logger.debug('Web video already present')
        return

    logger.debug('Fetching NHK video')

    # extract iframe URL
    html_match = re.search(
        r'(?s)<div[^>]*?src="(.*?)".*?</div>\s*',
        story.content_with_ruby,
    )
    if not html_match:
        logger.debug('No NHK video found')
        return
    iframe_url = html_match.group(1)
    logger.debug(f'Found iframe (URL={iframe_url})')

    if story.video_original:
        logger.error('Story has both regular and NHK videos')
        raise RegularAndNHKVideosError

    # extract JSON URL
    data = fetch(iframe_url)
    json_match = re.search(r'[^"\']*.json', data.decode())
    if not json_match:
        logger.error('Failed to find JSON filename of NHK video')
        return
    json_filename = json_match.group()
    logger.debug(f'Found JSON filename ({json_filename})')

    # fetch JSON
    json_url = nhk_contents + json_filename
    info = json.loads(fetch(json_url).decode())
    video_url = info['mediaResource']['url']
    logger.debug(f'Found NHK video URL ({video_url})')

    # fetch video
    logger.info(f'Fetching NHK video {video_url}')
    _, temp_name = mkstemp(suffix='.mp4')
    res = run(['ffmpeg', '-y', '-i', video_url, temp_name], stderr=DEVNULL, check=False)
    if res.returncode != 0:
        logger.warning('Failed to fetch NHK video')
        os.remove(temp_name)
        return
    logger.debug('Fetched video')

    # save video
    with open(temp_name, 'rb') as f:
        story.video_reencoded.save('', f)  # type: ignore
    logger.debug('Video saved')

    # remove HTML object
    logger.debug('Replacing NHK video iframe')
    old_ruby = story.content_with_ruby
    new_ruby = old_ruby[:html_match.start()] + old_ruby[html_match.end():]
    logger.debug('Updating content')
    story.content_with_ruby = new_ruby
    story.content = remove_ruby(new_ruby)
    story.save()
    logger.debug('Content updated')


def convert_story_video(story: Story) -> None:
    if story.video_reencoded:
        logger.debug('Web video already present')
        return

    if not story.video_original:
        logger.debug('No video')
        return

    logger.info(f'Converting {story.video_original.name}')
    original = story.video_original.file.name
    _, temp = mkstemp(suffix='.mp4')
    run(['ffmpeg', '-y', '-i', original, '-b:v', '500k', temp], stderr=DEVNULL, check=True)
    logger.debug('Video converted')
    with open(temp, 'rb') as f:
        story.video_reencoded.save('', f)  # type: ignore
    logger.debug('Video saved')
    os.remove(temp)


def fetch_story(info: StoryInfo, replace_voice: Dict[str, str]) -> bool:
    logger.debug(f'Fetching story {info["news_id"]} ({info["title"]})')
    set_voice_id(info, replace_voice)
    story, created = story_from_info(info)
    fetch_story_webpage(story, info)
    fetch_story_voice(story, info)
    today = datetime.now(tz=timezone.utc).date()
    if (today - story.published.date()).days <= 7:
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


def fetch_stories() -> None:
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
        logger.info(f'{new_stories_count} new stories')


def subedict_from_content(filename: str, content: str) -> None:
    subedict_dir = os.path.join(settings.BASE_DIR, 'media', 'subedict')
    os.makedirs(subedict_dir, exist_ok=True)
    path = os.path.join(subedict_dir, filename)
    save_subedict(create_subedict(content), path)


def subenamdict_from_content(filename: str, content: str) -> None:
    subenamdict_dir = os.path.join(settings.BASE_DIR, 'media', 'subenamdict')
    os.makedirs(subenamdict_dir, exist_ok=True)
    path = os.path.join(subenamdict_dir, filename)
    save_subedict(create_subenamdict(content), path)


def create_subedicts() -> None:
    logger.debug('Creating subedicts')
    stories = Story.objects.filter(subedict_created=False)
    logger.debug(f'Stories without subedicts: {len(stories)}')

    # create sub EDICT files for stories and list days that must be updated
    logger.debug('Creating story subedicts')
    new_days = set()
    for story in stories:
        logger.debug(f'Considering story id={story.id}')
        new_days.add(story.published.date())
        filename = f'{story.id:05}.dat'
        subedict_from_content(filename, story.content)
        subenamdict_from_content(filename, story.content)
        logger.info(filename)
    logger.info('Story-wise sub EDICT files updated')

    # update sub EDICT files for days
    logger.debug('Creating day subedicts')
    for day in sorted(new_days):
        logger.debug(f'Considering day {day}')
        day_stories = Story.objects.filter(published__date=day)
        content = ''.join(story.content for story in day_stories)
        filename = f'{day}.dat'
        subedict_from_content(filename, content)
        subenamdict_from_content(filename, content)
        logger.info(filename)
    logger.info('Day-wise sub EDICT files updated')

    # note that the subedict have been generated for those stories
    stories.update(subedict_created=True)
    logger.debug('Subedicts created')


def main() -> None:
    init_logging()
    logger.debug('Start of NHKUpdate command')
    fetch_stories()
    create_subedicts()
    logger.debug('End of NHKUpdate command')


class Command(BaseCommand):
    def handle(self, *_args: Any, **_options: Any) -> None:
        try:
            main()
        except Exception:
            logger.exception('NHKUPDATE GENERAL FAILURE')
