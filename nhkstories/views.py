from datetime import datetime, timedelta

from django.http import HttpResponse
from django.utils import timezone
from django.core.paginator import Paginator, PageNotAnInteger, EmptyPage
from django.shortcuts import render, get_object_or_404

from .models import Story

def index(request):
    stories = Story.objects.order_by('-id')
    paginator = Paginator(stories, 10)

    page = request.GET.get('page')
    try:
        stories = paginator.page(page)
    except PageNotAnInteger:
        stories = paginator.page(1)
    except EmptyPage:
        stories = paginator.page(paginator.num_pages)

    return render(request, 'nhkstories/index.html', {
        'stories': stories,
    })

def story(request, id):
    story = get_object_or_404(Story, pk=id)
    if story.published > timezone.now() - timedelta(days=30):
        url = 'https://www3.nhk.or.jp/news/easy/{0}/{0}.html'.format(
            story.story_id
        )
    else:
        url = None
    return render(request, 'nhkstories/story.html', {
        'story': story,
        'original_url': url,
    })
