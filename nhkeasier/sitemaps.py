from typing import List

from django.contrib.sitemaps import GenericSitemap, Sitemap
from django.urls import reverse

from .models import Story


class StaticSitemap(Sitemap):
    changefreq = 'daily'

    def items(self) -> List[str]:
        return ['about']

    def location(self, item):
        return reverse(item)


StorySitemap = GenericSitemap({
    'queryset': Story.objects.all(),
    'date_field': 'published',
})

sitemaps = {
    'static': StaticSitemap,
    'stories': StorySitemap,
}
