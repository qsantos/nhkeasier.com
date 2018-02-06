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


def save_story(story):
    # general information
    story_id = story['news_id']
    try:
        Story.objects.get(pk=story_id)
    except Story.DoesNotExist:
        pass
    else:
        return 0

    published = parse_datetime_nhk(story['news_prearranged_time'])
    title = story['title']
    title_with_ruby = story['title_with_ruby']
    assert remove_ruby(title_with_ruby) == title

    # webpage
    webpage_url = '%s/%s/%s.html' % (base_url, story_id, story_id)
    with urlopen(webpage_url) as req:
        data = req.read()
    temp = tempfile.TemporaryFile()
    temp.write(data)
    temp.seek(0)
    webpage = File(temp)

    # extract content from webpage
    data = data.decode()
    m = re.search(r'(?s)<div id="newsarticle">\n(.*?)\n\s*</div>', data)
    raw_content = m.group(1)
    content_with_ruby = clean_up_content(raw_content)
    content = remove_ruby(content_with_ruby)

    # image
    if story['has_news_web_image']:
        image_url = story['news_web_image_uri']
    elif story['has_news_easy_image']:  # rare
        filename = story['news_easy_image_uri']
        image_url = '/'.join([base_url, story_id, filename])
    else:
        image_url = None
    temp = tempfile.TemporaryFile()
    temp.write(urlopen(image_url).read())
    temp.seek(0)
    image = File(temp)

    # voice
    if story['has_news_easy_voice']:
        filename = story['news_easy_voice_uri']
        voice_url = '/'.join([base_url, story_id, filename])
    else:
        voice_url = None
    temp = tempfile.TemporaryFile()
    temp.write(urlopen(voice_url).read())
    temp.seek(0)
    voice = File(temp)

    # video
    stream_base_url = 'rtmp://flv.nhk.or.jp/ondemand/flv/news/'
    if story['has_news_web_movie']:
        filename = story['news_web_movie_uri']
        video_url = stream_base_url + filename
    else:
        video_url = None
    _, temp_name = tempfile.mkstemp()
    subprocess.call(['rtmpdump', '-r', video_url, '-o', temp_name])
    temp = open(temp_name, 'rb')
    video = File(temp)

    Story.objects.create(
        story_id=story_id,
        published=published,
        title_with_ruby=title_with_ruby,
        title=title,
        content_with_ruby=content_with_ruby,
        content=content,
        webpage=webpage,
        image=image,
        voice=voice,
        video=video,
    )
    video.close()
    os.remove(temp_name)
    voice.close()
    image.close()
    webpage.close()
    return 1


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
