from django.conf import settings
from django.urls import include, re_path
from django.conf.urls.static import static
from django.contrib import admin
from django.contrib.sitemaps.views import sitemap
from django.views.generic.base import RedirectView

from . import feed, views
from .sitemaps import sitemaps

urlpatterns = [
    re_path(r'^admin/', admin.site.urls),
    re_path(r'^favicon.ico$', RedirectView.as_view(url=settings.STATIC_URL + 'favicon.ico')),
    re_path(r'^robots.txt$', RedirectView.as_view(url=settings.STATIC_URL + 'robots.txt')),
    re_path(r'^sitemap\.xml$', sitemap, {'sitemaps': sitemaps}, name='django.contrib.sitemaps.views.sitemap'),

    re_path(r'^story/(?P<id>[0-9]*)/$', views.story, name='story'),
    re_path(r'^player/(?P<id>[0-9]*)/$', views.player, name='player'),
    re_path(r'^about/$', views.about, name='about'),
    re_path(r'^tools/$', views.tools, name='tools'),
    re_path(r'^feed/$', feed.LatestStoriesFeed(), name='feed'),
    re_path(r'^(?P<year>\d{4})/(?P<month>\d{2})/(?P<day>\d{2})/$', views.archive, name='archive'),
    re_path(r'^external-error/(?P<code>\d{3})/$', views.external_error, name='external_error'),
    re_path(r'^400/$', views.handler400),
    re_path(r'^403/$', views.handler403),
    re_path(r'^404/$', views.handler404),
    re_path(r'^500/$', views.handler500),
    re_path(r'^$', views.archive, name='home'),
]
handler400 = views.handler400
handler403 = views.handler403
handler404 = views.handler404
handler500 = views.handler500

if settings.DEBUG:
    urlpatterns += static(settings.MEDIA_URL, document_root=settings.MEDIA_ROOT)

    import debug_toolbar
    urlpatterns = [
        re_path(r'^__debug__/', include(debug_toolbar.urls)),
    ] + urlpatterns
