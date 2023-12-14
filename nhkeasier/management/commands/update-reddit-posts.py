import json
import logging
import re
from urllib.parse import urlencode
from urllib.request import Request, urlopen

from django.core.management.base import BaseCommand

from nhkeasier.logging import init_logging
from nhkeasier.models import Story

logger = logging.getLogger(__name__)

STORY_URL_PATTERN = r'https?://www3.nhk.or.jp/news/easy/(k[0-9]*)/k[0-9]*.html'
SUBREDDIT_NEW_URL = 'https://www.reddit.com/r/NHKEasyNews/new.json'


def fetch(url):
    request = Request(url, headers={'User-Agent': 'NHKEasier Crawler'})
    with urlopen(request) as f:
        return f.read()


def submissions_from_pushshift():
    base = 'https://api.pushshift.io/reddit/search/submission/'
    params = {
        'subreddit': 'NHKEasyNews',
        'sort': 'asc',
        'sort_type': 'created_utc',
        'size': 1000,
    }
    while True:
        result = fetch(base + '?' + urlencode(params)).decode()
        submissions = json.loads(result)['data']
        yield from submissions
        if len(submissions) < 1000:
            break
        params['after'] = submissions[-1]['created_utc']


def submissions_from_reddit():
    result = fetch(SUBREDDIT_NEW_URL).decode()
    result = json.loads(result)
    assert result['kind'] == 'Listing'
    for thing in result['data']['children']:
        assert thing['kind'] == 't3'
        data = thing['data']
        data['full_link'] = data['url']
        yield data


def stories_with_urls(submissions):
    for submission in submissions:
        try:
            selftext = submission['selftext']
        except KeyError:
            continue

        m = re.search(STORY_URL_PATTERN, selftext)
        if m is None:
            # no case where story_id could be retrieved from URL
            continue
        story_id = m.group(1)

        yield story_id, submission['full_link']


def main(archive):
    stories_by_story_id = {
        story.story_id: story
        for story in Story.objects.all()
    }

    submissions = submissions_from_pushshift() if archive else submissions_from_reddit()
    for story_id, url in stories_with_urls(submissions):
        try:
            story = stories_by_story_id[story_id]
        except KeyError:
            continue

        if story.r_nhkeasynews_link is not None:
            continue

        print(story_id)
        story.r_nhkeasynews_link = url
        story.save()


class Command(BaseCommand):
    help = 'Search NHKEasyNews posts'

    def add_arguments(self, parser):
        group = parser.add_mutually_exclusive_group(required=True)
        group.add_argument('--new', action='store_true')
        group.add_argument('--archive', action='store_true')

    def handle(self, archive, *_args, **_kwargs):
        init_logging()
        try:
            main(archive)
        except Exception:
            logger.exception('NHKUPDATE GENERAL FAILURE')
