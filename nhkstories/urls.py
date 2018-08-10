from django.conf.urls import url

from . import views
from . import feed

app_name = 'nhkstories'
urlpatterns = [
    url(r'^story/(?P<id>[0-9]*)/', views.story, name='story'),
    url(r'^about/', views.about, name='about'),
    url(r'^tools/', views.tools, name='tools'),
    url(r'^feed/', feed.LatestStoriesFeed(), name='feed'),
    url(r'^(?P<year>\d{4})/(?P<month>\d{2})/(?P<day>\d{2})/', views.archive, name='archive'),
    url(r'^external-error/(?P<code>\d{3})/', views.external_error, name='external_error'),
    url(r'^400/$', views.handler400),
    url(r'^403/$', views.handler403),
    url(r'^404/$', views.handler404),
    url(r'^500/$', views.handler500),
    url(r'^$', views.archive, name='index'),
]
