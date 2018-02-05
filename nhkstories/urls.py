from django.conf.urls import url

from . import views
from . import feed

app_name = 'nhkstories'
urlpatterns = [
    url(r'^story/(?P<story_id>k[0-9]*)(?:-(?P<story_title>.*))?.html', views.story, name='story'),
    url(r'^feed/', feed.LatestStoriesFeed(), name='feed'),
    url(r'^$', views.index, name='index'),
]
