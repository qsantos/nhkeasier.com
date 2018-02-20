import re
from datetime import datetime, timedelta

from django.http import HttpResponse
from django.urls import reverse
from django.utils import timezone
from django.core.paginator import Paginator, PageNotAnInteger, EmptyPage
from django.shortcuts import render, get_object_or_404

from .models import Story

def index(request):
    stories = Story.objects.order_by('-published', '-id')
    paginator = Paginator(stories, 10)

    page = request.GET.get('page')
    try:
        stories = paginator.page(page)
    except PageNotAnInteger:
        stories = paginator.page(1)
    except EmptyPage:
        stories = paginator.page(paginator.num_pages)

    url = request.build_absolute_uri(reverse('nhkstories:index'))
    return render(request, 'nhkstories/index.html', {
        'url': url,
        'title': 'Easier Japanese practice',
        'header': 'Latest stories',
        'description': 
            'Come practice reading and listening to Japanese with recent news '
            'stories! Simple vocabulary, simple kanji and simple sentence '
            'structures, as well as kanji readings (furigana) and an '
            'integrated dictionary will let you train until you get more '
            'comfortable for harder materials.',
        'stories': stories,
    })


def remove_all_html(content):
    content = re.sub("<.*?>", '', content)
    return content


def story(request, id):
    story = get_object_or_404(Story, pk=id)
    url = request.build_absolute_uri(reverse('nhkstories:story', args=(id,))),
    image = request.build_absolute_uri(story.image.url) if story.image else None
    return render(request, 'nhkstories/story.html', {
        'url': url,
        'title': story.title,
        'header': story.title,
        'description': remove_all_html(story.content),
        'image': image,
        'story': story,
    })


def about(request):
    url = request.build_absolute_uri(reverse('nhkstories:about'))
    return render(request, 'nhkstories/about.html', {
        'url': url,
        'title': 'About',
        'header': 'About',
    })

def tools(request):
    url = request.build_absolute_uri(reverse('nhkstories:tools'))
    return render(request, 'nhkstories/tools.html', {
        'url': url,
        'title': 'Tools',
        'header': 'Tools',
    })
