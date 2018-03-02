import re
from datetime import date

from django.http import HttpResponse
from django.urls import reverse
from django.utils import timezone
from django.core.paginator import Paginator, PageNotAnInteger, EmptyPage
from django.shortcuts import render, get_object_or_404

from .models import Story

def archive(request, year=None, month=None, day=None):
    if year is not None and month is not None and day is not None:
        day = date(int(year), int(month), int(day))
        header = 'Stories on {}'.format(day)
    else:
        day = Story.objects.order_by('-published').first().published.date()
        header = 'Latest stories'

    stories = Story.objects.filter(published__date=day).order_by('-published', '-id')

    # information for links (canonical URL, links to previous and next days)
    previous_day = Story.objects.filter(published__date__lt=day).order_by('-published').first()
    next_day = Story.objects.filter(published__date__gt=day).order_by('published').first()
    url = request.build_absolute_uri(reverse('nhkstories:index'))

    # take the image of one of the story as page illustration, if any
    illustrated_story = stories.exclude(image='').first()
    if illustrated_story is not None:
        image = request.build_absolute_uri(illustrated_story.image.url)
    else:
        image = None

    return render(request, 'nhkstories/index.html', {
        'url': url,
        'title': 'Easier Japanese practice',
        'header': header,
        'description': 
            'Come practice reading and listening to Japanese with recent news '
            'stories! Simple vocabulary, simple kanji and simple sentence '
            'structures, as well as kanji readings (furigana) and an '
            'integrated dictionary will let you train until you get more '
            'comfortable for harder materials.',
        'image': image,
        'stories': stories,
        'previous_day': previous_day,
        'day': day,
        'next_day': next_day,
    })


def remove_all_html(content):
    content = re.sub("<.*?>", '', content)
    return content


def story(request, id):
    story = get_object_or_404(Story, pk=id)

    # information for links (canonical URL, links to previous and next stories)
    previous_stories = Story.objects.filter(published__date=story.published.date(), id__lt=story.id) | Story.objects.filter(published__date__lt=story.published.date())
    previous_story = previous_stories.order_by('-published', '-id').first()
    next_stories = Story.objects.filter(published__date=story.published.date(), id__gt=story.id) | Story.objects.filter(published__date__gt=story.published.date())
    next_story = next_stories.order_by('published', 'id').first()
    url = request.build_absolute_uri(reverse('nhkstories:story', args=(id,))),

    # take the image of the story as page illustration, if any
    image = request.build_absolute_uri(story.image.url) if story.image else None

    return render(request, 'nhkstories/story.html', {
        'url': url,
        'title': story.title,
        'header': story.title,
        'description': remove_all_html(story.content),
        'image': image,
        'story': story,
        'previous_story': previous_story,
        'next_story': next_story,
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
