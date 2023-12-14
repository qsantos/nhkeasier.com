from datetime import date

from django.contrib.syndication.views import Feed
from django.urls import reverse

from .models import Story


class LatestStoriesFeed(Feed):
    title = 'NHK News Web Easier'
    link = '/'
    description = 'Latest stories from NHK News Web easy'

    def items(self):
        return Story.objects.filter(subedict_created=True).order_by('-published', '-id')[:50]

    def item_title(self, item):
        return item.title

    def item_pubdate(self, item):
        return item.published

    def item_description(self, story):
        html = ''

        # content
        video_src = story.video_reencoded.url
        img_src = story.image.url if story.image else None
        if video_src and img_src:
            html += f'<video src="{video_src}" poster="{img_src}" controls preload="none""></video>'
        elif video_src:
            html += f'<video src="{video_src}" controls preload="poster""></video>'
        elif img_src:
            html += f'<img src="{img_src}" alt="Story illustration">'
        html += story.content_with_ruby
        if story.voice:
            html += f'<audio src="{story.voice.url}" controls preload="none"></audio>'

        # links
        html += '<ul>'
        if story.r_nhkeasynews_link:
            html += f'<li><a href="{story.r_nhkeasynews_link}">/r/NHKEasyNews</a></li>'
        if story.published.date() >= date(2017, 12, 5):
            html += '<li><a href="https://www3.nhk.or.jp/news/easy/{0}/{0}.html">Original</a></li>'.format(story.story_id)
        html += '<li><a href="{}" class="permalink">Permalink</a></li>'.format(reverse('story', args=[story.id]))
        html += '</ul>'

        return html

    def item_link(self, item):
        return reverse('story', args=[item.id])
