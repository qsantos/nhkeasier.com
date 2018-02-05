import re
import sqlite3
import datetime

from django.core.management.base import BaseCommand
from django.core.files import File
from nhkstories.models import Story


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


class Command(BaseCommand):
    def add_arguments(self, parser):
        parser.add_argument('base_path', help='Old database file')

    def handle(self, *args, **options):
        base = options['base_path']
        with sqlite3.connect('{}/stories.db'.format(base)) as db:
            stories = []
            c = db.execute("""
                SELECT story_id, published, title, title_with_ruby, content,
                story_url, image_url, voice_url
                FROM stories ORDER BY published""")
            for (story_id, published, title, title_with_ruby, raw_content,
                story_url, image_url, voice_url) in c:

                assert remove_ruby(title_with_ruby) == title
                published = parse_datetime_nhk(published)
                content_with_ruby = clean_up_content(raw_content)
                content = remove_ruby(content_with_ruby)

                try:
                    html = File(open('{}/files/{}.html'.format(base, story_id), 'rb'))
                except FileNotFoundError:
                    html = None

                try:
                    jpg = File(open('{}/files/{}.jpg'.format(base, story_id), 'rb'))
                except FileNotFoundError:
                    jpg = None

                try:
                    mp3 = File(open('{}/files/{}.mp3'.format(base, story_id), 'rb'))
                except FileNotFoundError:
                    mp3 = None

                Story.objects.create(
                    story_id=story_id,
                    published=published,
                    title_with_ruby=title_with_ruby,
                    title=title,
                    content_with_ruby=content_with_ruby,
                    content=content,
                    webpage=html,
                    image=jpg,
                    voice=mp3,
                )

                if html is not None:
                    html.close()
                if jpg is not None:
                    jpg.close()
                if mp3 is not None:
                    mp3.close()
            #Story.objects.bulk_create(stories)
