from __future__ import annotations

from django.db import models
from django.urls import reverse


def webpage_filename(instance: Story, _filename: str) -> str:
    return f'html/{instance.story_id}.html'


def image_filename(instance: Story, _filename: str) -> str:
    return f'jpg/{instance.story_id}.jpg'


def voice_filename(instance: Story, _filename: str) -> str:
    return f'mp3/{instance.story_id}.mp3'


def video_original_filename(instance: Story, _filename: str) -> str:
    return f'mp4/{instance.story_id}.mp4'


def video_reencoded_filename(instance: Story, _filename: str) -> str:
    return f'mp4/{instance.story_id}.reencoded.mp4'


class Story(models.Model):
    id = models.AutoField(primary_key=True)
    story_id = models.CharField(max_length=200)
    published = models.DateTimeField()
    title_with_ruby = models.CharField(max_length=200)
    title = models.CharField(max_length=200)
    content_with_ruby = models.TextField()
    content = models.TextField()
    webpage = models.FileField(upload_to=webpage_filename)
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
        return f'{self.id}: {self.title}'

    def get_absolute_url(self) -> str:
        return reverse('story', kwargs={'id': self.id})
