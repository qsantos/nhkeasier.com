from django.db import models
from django.utils.deconstruct import deconstructible


@deconstructible
class NameByStoryID:
    def __init__(self, extension):
        self.extension = extension

    def __call__(self, instance, filename):
        return '{1}/{0}.{1}'.format(instance.story_id, self.extension)


class Story(models.Model):
    story_id = models.CharField(max_length=200, primary_key=True)
    published = models.DateTimeField()
    title_with_ruby = models.CharField(max_length=200)
    title = models.CharField(max_length=200)
    content_with_ruby = models.TextField()
    content = models.TextField()
    webpage = models.FileField(upload_to=NameByStoryID('html'), null=True)
    image = models.FileField(upload_to=NameByStoryID('jpg'), null=True)
    voice = models.FileField(upload_to=NameByStoryID('mp3'), null=True)

    class Meta:
        verbose_name_plural = 'stories'

    def __str__(self):
        return self.title
