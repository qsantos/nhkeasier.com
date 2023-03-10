from __future__ import annotations

from django.db import models
from django.urls import reverse


def webpage_filename(instance: Story, filename: str) -> str:
    return 'html/{}.html'.format(instance.story_id)


def image_filename(instance: Story, filename: str) -> str:
    return 'jpg/{}.jpg'.format(instance.story_id)


def voice_filename(instance: Story, filename: str) -> str:
    return 'mp3/{}.mp3'.format(instance.story_id)


def video_original_filename(instance: Story, filename: str) -> str:
    return 'mp4/{}.mp4'.format(instance.story_id)


def video_reencoded_filename(instance: Story, filename: str) -> str:
    return 'mp4/{}.reencoded.mp4'.format(instance.story_id)


class Story(models.Model):
    story_id = models.CharField(max_length=200)
    published = models.DateTimeField(null=True)
    title_with_ruby = models.CharField(max_length=200, null=True)
    title = models.CharField(max_length=200, null=True)
    content_with_ruby = models.TextField(null=True)
    content = models.TextField(null=True)
    webpage = models.FileField(upload_to=webpage_filename, null=True)
    image = models.FileField(upload_to=image_filename, null=True)
    voice = models.FileField(upload_to=voice_filename, null=True)
    video_original = models.FileField(upload_to=video_original_filename, null=True)
    video_reencoded = models.FileField(upload_to=video_reencoded_filename, null=True)
    subedict_created = models.BooleanField(default=False)
    r_nhkeasynews_link = models.CharField(max_length=200, null=True)
    twitter_post_id = models.CharField(max_length=200, null=True)
    facebook_post_id = models.CharField(max_length=200, null=True)

    class Meta:
        verbose_name_plural = 'stories'

    def __str__(self) -> str:
        return '{}: {}'.format(self.id, self.title)

    def get_absolute_url(self) -> str:
        return reverse('story', kwargs={'id': self.id})  # type: ignore
