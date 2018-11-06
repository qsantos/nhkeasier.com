from django.db import models
from django.utils.deconstruct import deconstructible
from django.urls import reverse


def webpage_filename(instance, filename):
    return 'html/{}.html'.format(instance.story_id)


def image_filename(instance, filename):
    return 'jpg/{}.jpg'.format(instance.story_id)


def voice_filename(instance, filename):
    return 'mp3/{}.mp3'.format(instance.story_id)


def video_original_filename(instance, filename):
    return 'mp4/{}.mp4'.format(instance.story_id)


def video_reencoded_filename(instance, filename):
    return 'mp4/{}.reencoded.mp4'.format(instance.story_id)


# kept for migrations
@deconstructible
class NameByStoryID:
    def __init__(self, extension):
        self.extension = extension

    def __call__(self, instance, filename):
        return '{1}/{0}.{1}'.format(instance.story_id, self.extension)


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

    class Meta:
        verbose_name_plural = 'stories'

    def __str__(self):
        return '{}: {}'.format(self.id, self.title)

    def get_absolute_url(self):
        return reverse('nhkstories:story', kwargs={'id': self.id})
