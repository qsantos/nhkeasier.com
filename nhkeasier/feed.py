import datetime

from django.contrib.syndication.views import Feed
from django.db import models
from django.db.models.manager import BaseManager
from django.http import HttpRequest
from django.urls import reverse
from django.utils.html import escape
from django.utils.safestring import SafeString

from .models import Story


class LatestStoriesFeed(Feed):
    title = 'NHK News Web Easier'
    link = '/'
    description = 'Latest stories from NHK News Web easy'
    furiganas = False

    def get_object(self, request: HttpRequest):
        self.furiganas = request.META['QUERY_STRING'] != 'no-furiganas'

    def items(self) -> BaseManager[models.Model]:
        return Story.objects.filter(subedict_created=True).order_by('-published', '-id')[:50]  # type: ignore

    def item_title(self, item: models.Model) -> SafeString:
        assert isinstance(item, Story)
        return escape(item.title)

    def item_pubdate(self, item: models.Model) -> datetime.date:
        assert isinstance(item, Story)
        return item.published

    def item_description(self, story: models.Model) -> str:
        assert isinstance(story, Story)
        html = ''

        # content
        video_src = story.video_reencoded.url if story.video_reencoded else None
        img_src = story.image.url if story.image else None
        if video_src and img_src:
            html += f'<video src="{video_src}" poster="{img_src}" controls preload="none""></video>'
        elif video_src:
            html += f'<video src="{video_src}" controls preload="poster""></video>'
        elif img_src:
            html += f'<img src="{img_src}" alt="Story illustration">'
        if self.furiganas:
            html += story.content_with_ruby
        else:
            html += story.content
        if story.voice:
            html += f'<audio src="{story.voice.url}" controls preload="none"></audio>'

        # links
        html += '<ul>'
        if story.published.date() >= datetime.date(2017, 12, 5):
            html += '<li><a href="https://www3.nhk.or.jp/news/easy/{0}/{0}.html">Original</a></li>'.format(story.story_id)
        html += '<li><a href="{}" class="permalink">Permalink</a></li>'.format(reverse('story', args=[story.id]))
        html += '</ul>'

        return html

    def item_link(self, item: models.Model) -> str:
        assert isinstance(item, Story)
        return reverse('story', args=[item.id])
