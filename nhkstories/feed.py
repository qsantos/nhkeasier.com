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

    def item_description(self, item):
        return '<img src="{}"><br><audio src="{}" alt="Story illustration" controls></audio>{}'.format(
            item.image.url, item.voice.url, item.content_with_ruby,
        )

    def item_link(self, item):
        return reverse('nhkstories:story', args=[item.id])
