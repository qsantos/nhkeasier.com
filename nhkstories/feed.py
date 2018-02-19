from django.contrib.syndication.views import Feed
from django.urls import reverse
from django.contrib.staticfiles.templatetags.staticfiles import static
from .models import Story


class LatestStoriesFeed(Feed):
    title = "NHK News Web Easier"
    link = "/"
    description = "Latest stories from NHK News Web easy"

    def items(self):
        return Story.objects.order_by('-published', '-id')[:20]

    def item_title(self, item):
        return item.title

    def item_pubdate(self, item):
        return item.published

    def item_description(self, story):
        html = ''
        if story.video_reencoded:
            html += '<video src="{}" poster="{}" controls preload="none""></video>'.format(story.video_reencoded.url, story.image.url)
        elif story.image:
            html += '<img src="{}" alt="Story illustration">'.format(story.image.url)
        html += story.content_with_ruby
        if story.voice:
            html += '<audio src={} controls preload="none"></audio>'.format(story.voice.url)
        html += '<a href="https://www3.nhk.or.jp/news/easy/{0}/{0}.html">See on NHK News Web Easy</a> (if still available)'.format(story.story_id)
        return html

    def item_link(self, item):
        return reverse('nhkstories:story', args=[item.id])
