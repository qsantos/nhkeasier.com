from django.conf.urls import url, include
from django.contrib import admin
from django.conf import settings
from django.conf.urls.static import static
from django.views.generic.base import RedirectView
from django.contrib.sitemaps.views import sitemap

import nhkstories.views
from nhkstories.sitemaps import sitemaps


urlpatterns = [
    url(r'^admin/', admin.site.urls),
    url(r'^favicon.ico$', RedirectView.as_view(url=settings.STATIC_URL + '/nhkstories/favicon.ico')),
    url(r'^sitemap\.xml$', sitemap, {'sitemaps': sitemaps}, name='django.contrib.sitemaps.views.sitemap'),
    url(r'^', include('nhkstories.urls')),
]
handler400 = nhkstories.views.handler400
handler403 = nhkstories.views.handler403
handler404 = nhkstories.views.handler404
handler500 = nhkstories.views.handler500

if settings.DEBUG:
    urlpatterns += static(settings.MEDIA_URL, document_root=settings.MEDIA_ROOT)

    import debug_toolbar
    urlpatterns = [
        url(r'^__debug__/', include(debug_toolbar.urls)),
    ] + urlpatterns
