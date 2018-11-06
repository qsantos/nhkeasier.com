"""nhkeasier URL Configuration

The `urlpatterns` list routes URLs to views. For more information please see:
    https://docs.djangoproject.com/en/1.10/topics/http/urls/
Examples:
Function views
    1. Add an import:  from my_app import views
    2. Add a URL to urlpatterns:  url(r'^$', views.home, name='home')
Class-based views
    1. Add an import:  from other_app.views import Home
    2. Add a URL to urlpatterns:  url(r'^$', Home.as_view(), name='home')
Including another URLconf
    1. Import the include() function: from django.conf.urls import url, include
    2. Add a URL to urlpatterns:  url(r'^blog/', include('blog.urls'))
"""
from django.conf.urls import url, include
from django.contrib import admin
from django.conf import settings
from django.conf.urls.static import static
import django.views.defaults
from django.views.generic.base import RedirectView
from django.http import Http404
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
