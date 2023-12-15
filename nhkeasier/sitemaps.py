from django.contrib.sitemaps import GenericSitemap, Sitemap
from django.db import models
from django.urls import reverse

from .models import Story


class StaticSitemap(Sitemap):
    changefreq = 'daily'

    def items(self) -> list[str]:
        return ['about']

    def location(self, item: models.Model) -> str:
        assert isinstance(item, str)
        return reverse(item)


StorySitemap = GenericSitemap({
    'queryset': Story.objects.all(),
    'date_field': 'published',
})

sitemaps = {
    'static': StaticSitemap,
    'stories': StorySitemap,
}
