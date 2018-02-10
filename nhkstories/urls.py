from django.conf.urls import url

from . import views
from . import feed

app_name = 'nhkstories'
urlpatterns = [
    url(r'^story/(?P<id>[0-9]*)/', views.story, name='story'),
    url(r'^about/', views.about, name='about'),
    url(r'^feed/', feed.LatestStoriesFeed(), name='feed'),
    url(r'^$', views.index, name='index'),
]
