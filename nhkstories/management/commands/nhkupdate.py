import re
import os
import json
import datetime
import tempfile
import subprocess
from urllib.request import urlopen, HTTPError

from django.core.management.base import BaseCommand
from django.core.files import File
from django.core.files.base import ContentFile
from nhkstories.models import Story
from nhkstories.edict.parse import load_edict, load_enamdict
from nhkstories.edict.filter import filter_edict, save_subedict


class DuplicateStoryIDType(Exception):
    pass


DuplicateStoryID = DuplicateStoryIDType()
base_url = 'http://www3.nhk.or.jp/news/easy'


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
    dt = datetime.datetime.strptime(s, '%Y-%m-%d %H:%M:%S')
    jst = datetime.timezone(datetime.timedelta(hours=9))
    return dt.replace(tzinfo=jst)


def save_story(info):
    # general information
    story_id = info['news_id']
    story, created = Story.objects.get_or_create(story_id=story_id)
    published = parse_datetime_nhk(info['news_prearranged_time'])

    if story.published and abs(story.published - published).days > 2:
        # probably a reused story_id, not implemented yet
        raise DuplicateStoryID

    story.published = published
    story.title = info['title']
    story.title_with_ruby = info['title_with_ruby']
    assert remove_ruby(story.title_with_ruby) == story.title

    # webpage
    if not story.webpage:
        webpage_url = '%s/%s/%s.html' % (base_url, story_id, story_id)
        print('Download %s' % webpage_url)
        with urlopen(webpage_url) as f:
            story.webpage.save('', f)

    # extract content from webpage
    data = story.webpage.read().decode()
    story.webpage.seek(0)  # the webpage might be read when updating story
    m = re.search(r'(?s)<div class="article-main__body article-body" id="js-article-body">(.*?)</div>', data)
    if m is None:
        m = re.search(r'(?s)<div id="newsarticle">(.*?)</div>', data)
    raw_content = m.group(1)
    story.content_with_ruby = clean_up_content(raw_content)
    story.content = remove_ruby(story.content_with_ruby)

    # image
    if info['has_news_web_image']:
        image_url = info['news_web_image_uri']
    elif info['has_news_easy_image']:  # rare
        filename = info['news_easy_image_uri']
        image_url = '/'.join([base_url, story_id, filename])
    else:
        image_url = None
    if not story.image and image_url is not None:
        print('Download image %s' % image_url)
        try:
            with urlopen(image_url) as f:
                story.image.save('', f)
        except HTTPError:
            print('Failed')
        else:
            subprocess.run(
                ['mogrify', '-interlace', 'plane', story.image.file.name],
                check=True
            )

    # voice
    if info['has_news_easy_voice']:
        filename = info['news_easy_voice_uri']
        voice_url = '/'.join([base_url, story_id, filename])
    else:
        voice_url = None
    if not story.voice and voice_url is not None:
        if voice_url.endswith('.mp4'):
            # fragmented MP4 using HTTP Live Streaming
            voice_url = 'https://nhks-vh.akamaihd.net/i/news/easy/' + \
                story_id + '.mp4/master.m3u8'
            print('Download voice (fragmented MP4) %s' % voice_url)
            _, temp_name = tempfile.mkstemp(suffix='.mp3')
            res = subprocess.run(
                ['ffmpeg', '-y', '-i', voice_url, temp_name],
                stderr=subprocess.DEVNULL
            )
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

    # video
    stream_base_url = 'rtmp://flv.nhk.or.jp/ondemand/flv/news/'
    if info['has_news_web_movie']:
        filename = info['news_web_movie_uri']
        video_url = stream_base_url + filename
    else:
        video_url = None
    if not story.video_original and video_url is not None:
        print('Download video %s' % video_url)
        _, temp_name = tempfile.mkstemp()
        res = subprocess.run(['rtmpdump', '-r', video_url, '-o', temp_name],
                             stderr=subprocess.DEVNULL)
        if res.returncode == 0:
            with open(temp_name, 'rb') as f:
                story.video_original.save('', f)
        else:
            print('Failed')
        os.remove(temp_name)

    if story.video_original and not story.video_reencoded:
        print('Converting %s' % story.video_original.name)
        _, temp_name = tempfile.mkstemp(suffix='.mp4')
        subprocess.run(
            ['ffmpeg', '-y', '-i', story.video_original.file.name, '-b:v', '500k', temp_name],
            stderr=subprocess.DEVNULL, check=True
        )
        with open(temp_name, 'rb') as f:
            story.video_reencoded.save('', f)
        os.remove(temp_name)

    story.save()
    return created


def get_news_list():
    news_url = 'http://www3.nhk.or.jp/news/easy/news-list.json'
    with urlopen(news_url) as f:
        data = f.read()
    return json.loads(data.decode('utf-8-sig'))


def fetch_story(story_id, story_url, image_url, voice_url):
    success = True
    success &= fetch(story_url, pathlib.Path('files', story_id + '.html'))
    success &= fetch(image_url, pathlib.Path('files', story_id + '.jpg'))
    success &= fetch(voice_url, pathlib.Path('files', story_id + '.mp3'))
    return success


def create_subedicts():
    stories = Story.objects.filter(subedict_created=False)

    if not stories:
        return

    # load EDICT files
    edict = load_edict()
    enamdict = load_enamdict()

    # ensure the directories exist
    subedict_dir = os.path.join('media', 'subedict')
    subenamdict_dir = os.path.join('media', 'subenamdict')
    os.makedirs(subedict_dir, exist_ok=True)
    os.makedirs(subenamdict_dir, exist_ok=True)

    # create sub EDICT files for stories and list days that must be updated
    new_days = set()
    for story in stories:
        new_days.add(story.published.date())
        # create sub EDICT files for story
        subedict = filter_edict(edict, story.content)
        subenamdict = filter_edict(enamdict, story.content)
        filename = '{:05}.dat'.format(story.id)
        save_subedict(subedict, os.path.join(subedict_dir, filename))
        save_subedict(subenamdict, os.path.join(subenamdict_dir, filename))
        print(filename)
    print('Story-wise sub EDICT files updated')

    # update sub EDICT files for days
    for day in sorted(new_days):
        # aggregate stories of the day
        day_stories = Story.objects.filter(published__date=day)
        text = ''.join(story.content for story in day_stories)
        # update sub EDICT files for day
        subedict = filter_edict(edict, text)
        subenamdict = filter_edict(enamdict, text)
        filename = '{}.dat'.format(day)
        save_subedict(subedict, os.path.join(subedict_dir, filename))
        save_subedict(subenamdict, os.path.join(subenamdict_dir, filename))
        print(filename)
    print('Day-wise sub EDICT files updated')

    # note that the subedict have been generated for those stories
    stories.update(subedict_created=True)


class Command(BaseCommand):
    def handle(self, *args, **options):
        new_stories_count = 0
        news_list = get_news_list()
        stories_per_day = news_list[0]  # day -> stories
        for day in sorted(stories_per_day):
            for story in stories_per_day[day]:
                new_stories_count += save_story(story)

        if new_stories_count == 0:
            print('No new stories')
        elif new_stories_count == 1:
            print('1 new story')
        else:
            print('%i new stories' % new_stories_count)

        create_subedicts()
