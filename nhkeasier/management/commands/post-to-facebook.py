import json
import logging
from time import sleep

import requests
from django.urls import reverse
from django.db.utils import OperationalError
from django.core.management.base import BaseCommand

from nhkeasier import settings
from ...logging import init_logging
from ...models import Story

logger = logging.getLogger(__name__)

NHKEASIER_BASE = 'https://nhkeasier.com'

PAGE_BASE_URL = 'https://graph.facebook.com/{}'.format(1040161449507630)
PAGE_FEED_URL = PAGE_BASE_URL + '/feed'
PAGE_PHOTOS_URL = PAGE_BASE_URL + '/photos'
PAGE_VIDEOS_URL = PAGE_BASE_URL + '/videos'


def post_story_to_facebook(story, page_access_token):
    logger.info('Posting story {} to Facebook'.format(story.id))

    # status
    url = NHKEASIER_BASE + reverse('story', args=[story.id])
    refs = '#nhk_easier #nhk_news #nhk_easy #日本 #日本語'
    status = '{}\n{}\n{:%Y-%m-%d}\n{}'.format(story.title, url, story.published, refs)

    # media
    if story.video_reencoded:
        logger.debug('Posting as video')
        res = requests.post(PAGE_VIDEOS_URL, data={
            'access_token': page_access_token,
            'description': status,
        }, files={'video.mp4': story.video_reencoded})
    elif story.image:
        logger.debug('Posting as photo')
        res = requests.post(PAGE_PHOTOS_URL, data={
            'access_token': page_access_token,
            'caption': status,
        }, files={'image.jpg': story.image})
    else:
        logger.debug('Posting as link')
        res = requests.post(PAGE_FEED_URL, params={
            'access_token': page_access_token,
            'message': status,
            'link': url,
        })
    if res.status_code != 200:
        logger.error('POST returned {}: {}'.format(res, res.text))
        exit(1)

    story.facebook_post_id = res.json()['id']
    while True:
        try:
            story.save()
        except OperationalError:
            logger.debug('Failed to update story.facebook_post_if')
            sleep(1)
        else:
            break
    logger.debug('Story {} posted to Facebook'.format(story.id))


def post_stories_to_facebook(stories):
    # auth
    with open(settings.BASE_DIR + '/facebook-oauth.json') as f:
        oauth_settings = json.load(f)
    page_access_token = oauth_settings['access_token']

    for story in stories:
        post_story_to_facebook(story, page_access_token)


def main(archive):
    if archive:
        stories = Story.objects.all().order_by('published')
    else:
        stories = Story.objects.filter(facebook_post_id=None).order_by('published')
    post_stories_to_facebook(stories)


class Command(BaseCommand):
    help = 'Post stories to Twitter'

    def add_arguments(self, parser):
        group = parser.add_mutually_exclusive_group(required=True)
        group.add_argument('--new', action='store_true')
        group.add_argument('--archive', action='store_true')

    def handle(self, archive, *args, **kwargs):
        init_logging()
        try:
            main(archive)
        except Exception:
            logger.exception('NHKUPDATE GENERAL FAILURE')
