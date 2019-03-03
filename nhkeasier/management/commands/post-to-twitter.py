import os
import json
import logging
from time import sleep
from tempfile import NamedTemporaryFile
from subprocess import run, check_output, DEVNULL

from authlib.client import OAuth1Session
from django.urls import reverse
from django.core.management.base import BaseCommand

from nhkeasier import settings
from ...logging import init_logging
from ...models import Story

logger = logging.getLogger(__name__)

NHKEASIER_BASE = 'https://nhkeasier.com'
POST_TWEET_URL = 'https://api.twitter.com/1.1/statuses/update.json'
UPLOAD_MEDIA_URL = 'https://upload.twitter.com/1.1/media/upload.json'

STORY_URL_PATTERN = r'https?://www3.nhk.or.jp/news/easy/(k[0-9]*)/k[0-9]*.html'


def filesize(f):
    old_pos = f.tell()
    f.seek(0, os.SEEK_END)
    size = f.tell()
    f.seek(old_pos, os.SEEK_SET)
    return size


def video_duration(filename):
    # inspired from https://stackoverflow.com/a/54158915/4457767
    res = check_output([
        'ffprobe',
        '-v', 'error',
        '-show_entries', 'format=duration',
        '-of', 'default=noprint_wrappers=1:nokey=1',
        filename
    ])
    return float(res.decode())


def escape_ffmpeg_option(option):
    option = option.replace('\\', '\\\\')
    option = option.replace(':', '\\:')
    option = option.replace('|', '\\|')
    option = option.replace(' ', '\\ ')
    return option


def video_truncate(input_filename, output_filename, duration):
    logger.debug('Truncating video')

    message = 'Truncated for Twitter. Visit NHKEasier for full video.'
    drawtext_options = {
        'text': escape_ffmpeg_option(message),
        'x': '(main_w-text_w)/2',
        'y': 'main_h-text_h*2',
        'fontcolor': 'white',
        'fontsize': '20',
        'box': 1,
        'boxcolor': 'black@.5',
        'boxborderw': '5',
    }
    drawtext_options = ': '.join(
        '{}={}'.format(key, value)
        for key, value in drawtext_options.items()
    )

    run([
        'ffmpeg',
        '-y',
        '-i', input_filename,
        '-c:a', 'copy',
        '-t', str(duration),
        '-vf', 'drawtext={}'.format(drawtext_options),
        output_filename,
    ], stderr=DEVNULL, check=True)


def upload(client, f, mimetype, category):
    size = filesize(f)
    logger.debug('Uploading {:,} bytes ({})'.format(size, mimetype))

    # INIT
    res = client.post(UPLOAD_MEDIA_URL, data={
        'command': 'INIT',
        'total_bytes': size,
        'media_type': mimetype,
        'media_category': category,
    })
    if res.status_code != 202:
        logger.error('INIT returned {}: {}'.format(res, res.text))
        exit(1)
    media_id = res.json()['media_id_string']
    logger.debug('Created media {}'.format(media_id))

    sent_bytes = 0
    chunks = iter(lambda: f.read(2**22), b'')
    for segment_index, chunk in enumerate(chunks):
        # APPEND
        res = client.post(UPLOAD_MEDIA_URL, data={
            'command': 'APPEND',
            'media_id': media_id,
            'segment_index': segment_index,
        }, files={
            'media': chunk,
        })
        if res.status_code != 204:
            logger.error('APPEND {} returned {}: {}'.format(segment_index, res, res.text))
            exit(1)

        sent_bytes += len(chunk)
        progress_bar_width = 30
        progress = int(sent_bytes / size * progress_bar_width)
        progress_bar = '*'*progress + '_'*(progress_bar_width - progress)
        print('\r\x1b[2K{} {}'.format(media_id, progress_bar), end='')

    # FINALIZE
    res = client.post(UPLOAD_MEDIA_URL, data={
        'command': 'FINALIZE',
        'media_id': media_id,
    })
    if res.status_code not in (200, 201):
        logger.error('FINALIZE returned {}: {}'.format(res, res.text))
        exit(1)

    while res.status_code != 201:
        info = res.json()['processing_info']
        if info['state'] == 'succeeded':
            break
        elif info['state'] == 'failed':
            logger.error('\nMedia upload failed: {}'.format(res.text))
            exit(1)
        print('\r\x1b[2K{} {} s...'.format(media_id, info['check_after_secs']), end='')
        sleep(info['check_after_secs'])

        # STATUS
        res = client.get(UPLOAD_MEDIA_URL, params={
            'command': 'STATUS',
            'media_id': media_id,
        })
        if res.status_code != 200:
            logger.error('STATUS returned {}: {}'.format(res, res.text))
            exit(1)

    print('\r\x1b[2K')
    logger.debug('Uploaded media {}'.format(media_id))
    return media_id


def tweet(client, message, media_ids=None):
    params = {'status': message}
    if media_ids:
        params['media_ids'] = ','.join(media_ids),

    res = client.post(POST_TWEET_URL, data=params)
    if res.status_code != 200:
        logger.error('TWEET returned {}: {}'.format(res, res.text))
        exit(1)
    id = res.json()['id']
    logger.info('Tweeted https://twitter.com/NHKEasier/status/{}'.format(id))
    return id


def post_story_to_twitter(story, client):
    logger.info('Posting story {} to Twitter'.format(story.id))

    # status
    url = NHKEASIER_BASE + reverse('story', args=[story.id])
    refs = '#nhk_easier #nhk_news #nhk_easy #日本 #日本語'
    status = '{}\n（{}） [{:%Y-%m-%d}] {{{}}}'.format(story.title, url, story.published, refs)

    # media
    if story.video_reencoded:
        logger.debug('Joining video')
        if video_duration(story.video_reencoded.file.name) >= 139:
            with NamedTemporaryFile(suffix='.mp4') as f:
                video_truncate(story.video_reencoded.file.name, f.name, 139)
                media_ids = [
                    upload(client, f, 'video/mp4', 'tweet_video'),
                ]
        else:
            media_ids = [
                upload(client, story.video_reencoded, 'video/mp4', 'tweet_video'),
            ]
    elif story.image:
        logger.debug('Joining image')
        media_ids = [
            upload(client, story.image, 'image/jpeg', 'tweet_image'),
        ]
    else:
        logger.debug('Joining no media')
        media_ids = []

    story.twitter_post_id = tweet(client, status, media_ids)
    story.save()
    logger.debug('Story {} posted to Twitter'.format(story.id))


def post_stories_to_twitter(stories):
    # auth
    with open(settings.BASE_DIR + '/twitter-oauth.json') as f:
        oauth_settings = json.load(f)
    client = OAuth1Session(**oauth_settings)

    for story in stories:
        post_story_to_twitter(story, client)


def main(archive):
    if archive:
        stories = Story.objects.all().order_by('published')
    else:
        stories = Story.objects.filter(twitter_post_id=None).order_by('published')
    post_stories_to_twitter(stories)


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
