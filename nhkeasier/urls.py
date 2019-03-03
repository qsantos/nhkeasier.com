from django.conf.urls import url, include
from django.contrib import admin
from django.conf import settings
from django.conf.urls.static import static
from django.views.generic.base import RedirectView
from django.contrib.sitemaps.views import sitemap

from . import views
from . import feed
from .sitemaps import sitemaps


urlpatterns = [
    url(r'^admin/', admin.site.urls),
    url(r'^favicon.ico$', RedirectView.as_view(url=settings.STATIC_URL + 'favicon.ico')),
    url(r'^robots.txt$', RedirectView.as_view(url=settings.STATIC_URL + 'robots.txt')),
    url(r'^sitemap\.xml$', sitemap, {'sitemaps': sitemaps}, name='django.contrib.sitemaps.views.sitemap'),

    url(r'^story/(?P<id>[0-9]*)/$', views.story, name='story'),
    url(r'^player/(?P<id>[0-9]*)/$', views.player, name='player'),
    url(r'^contact/$', views.contact, name='contact'),
    url(r'^contact/sent/$', views.contact_sent, name='contact_sent'),
    url(r'^about/$', views.about, name='about'),
    url(r'^tools/$', views.tools, name='tools'),
    url(r'^feed/$', feed.LatestStoriesFeed(), name='feed'),
    url(r'^(?P<year>\d{4})/(?P<month>\d{2})/(?P<day>\d{2})/$', views.archive, name='archive'),
    url(r'^external-error/(?P<code>\d{3})/$', views.external_error, name='external_error'),
    url(r'^400/$', views.handler400),
    url(r'^403/$', views.handler403),
    url(r'^404/$', views.handler404),
    url(r'^500/$', views.handler500),
    url(r'^$', views.archive, name='home'),
]
handler400 = views.handler400
handler403 = views.handler403
handler404 = views.handler404
handler500 = views.handler500

if settings.DEBUG:
    urlpatterns += static(settings.MEDIA_URL, document_root=settings.MEDIA_ROOT)

    import debug_toolbar
    urlpatterns = [
        url(r'^__debug__/', include(debug_toolbar.urls)),
    ] + urlpatterns
