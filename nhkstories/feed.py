from datetime import date

from django.contrib.syndication.views import Feed
from django.urls import reverse

from .models import Story


class LatestStoriesFeed(Feed):
    title = "NHK News Web Easier"
    link = "/"
    description = "Latest stories from NHK News Web easy"

    def items(self):
        return Story.objects.exclude(content_with_ruby=None).order_by('-published', '-id')[:50]

    def item_title(self, item):
        return item.title

    def item_pubdate(self, item):
        return item.published

    def item_description(self, story):
        html = ''

        # content
        if story.video_reencoded and story.image:
            html += '<video src="{}" poster="{}" controls preload="none""></video>'.format(story.video_reencoded.url, story.image.url)
        elif story.video_reencoded:
            html += '<video src="{}" controls preload="poster""></video>'.format(story.video_reencoded.url)
        elif story.image:
            html += '<img src="{}" alt="Story illustration">'.format(story.image.url)
        html += story.content_with_ruby
        if story.voice:
            html += '<audio src="{}" controls preload="none"></audio>'.format(story.voice.url)

        # links
        html += '<ul>'
        if story.r_nhkeasynews_link:
            html += '<li><a href="{}">/r/NHKEasyNews</a></li>'.format(story.r_nhkeasynews_link)
        if story.published.date() >= date(2017, 12, 5):
            html += '<li><a href="https://www3.nhk.or.jp/news/easy/{0}/{0}.html">Original</a></li>'.format(story.story_id)
        html += '<li><a href="{}" class="permalink">Permalink</a></li>'.format(reverse('nhkstories:story', args=[story.id]))
        html += '</ul>'

        return html

    def item_link(self, item):
        return reverse('nhkstories:story', args=[item.id])
