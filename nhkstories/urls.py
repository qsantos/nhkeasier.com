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
    url(r'^$', views.archive, name='index'),
]
