import re
import os
import json
import datetime
import tempfile
import subprocess
from urllib.request import urlopen

from django.core.management.base import BaseCommand
from django.core.files import File
from nhkstories.models import Story

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
    content = re.sub('<rt>.*?</rt>', '', content)
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

    story.published = parse_datetime_nhk(info['news_prearranged_time'])
    story.title = info['title']
    story.title_with_ruby = info['title_with_ruby']
    assert remove_ruby(story.title_with_ruby) == story.title

    # webpage
    if not story.webpage:
        webpage_url = '%s/%s/%s.html' % (base_url, story_id, story_id)
        print('Download %s' % webpage_url)
        with urlopen(webpage_url) as req:
            data = req.read()
        temp = tempfile.TemporaryFile()
        temp.write(data)
        temp.seek(0)
        story.webpage = File(temp)

    # extract content from webpage
    data = story.webpage.read().decode()
    story.webpage.seek(0)  # the webpage might be read when updating story
    m = re.search(r'(?s)<div id="newsarticle">\n(.*?)\n\s*</div>', data)
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
        print('Download %s' % image_url)
        temp = tempfile.TemporaryFile()
        temp.write(urlopen(image_url).read())
        temp.seek(0)
        story.image = File(temp)

    # voice
    if info['has_news_easy_voice']:
        filename = info['news_easy_voice_uri']
        voice_url = '/'.join([base_url, story_id, filename])
    else:
        voice_url = None
    if not story.voice and voice_url is not None:
        print('Download %s' % voice_url)
        temp = tempfile.TemporaryFile()
        temp.write(urlopen(voice_url).read())
        temp.seek(0)
        story.voice = File(temp)

    # video
    stream_base_url = 'rtmp://flv.nhk.or.jp/ondemand/flv/news/'
    if info['has_news_web_movie']:
        filename = info['news_web_movie_uri']
        video_url = stream_base_url + filename
    else:
        video_url = None
    if not story.video and video_url is not None:
        print('Download %s' % video_url)
        _, temp_name = tempfile.mkstemp()
        res = subprocess.run(['rtmpdump', '-r', video_url, '-o', temp_name],
                             stderr=subprocess.DEVNULL)
        if res.returncode == 0:
            temp = open(temp_name, 'rb')
            story.video = File(temp)
        else:
            print('Failed')

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


class Command(BaseCommand):
    def handle(self, *args, **options):
        new_stories_count = 0
        news_list = get_news_list()
        for stories_of_the_day in news_list[0].values():
            for story in stories_of_the_day:
                new_stories_count += save_story(story)

        if new_stories_count == 0:
            print('No new stories')
        elif new_stories_count == 1:
            print('1 new story')
        else:
            print('%i new stories' % new_stories_count)
