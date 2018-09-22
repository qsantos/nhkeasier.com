import re
from datetime import date

from django.http import HttpResponse
from django.urls import reverse
from django.utils import timezone
from django.core.paginator import Paginator, PageNotAnInteger, EmptyPage
from django.shortcuts import render, get_object_or_404
from django.core.mail import send_mail

from .models import Story


def handler400(request):
    url = request.build_absolute_uri()
    return render(request, 'nhkstories/400.html', {
        'url': url,
        'title': 'Bad Request',
        'header': 'Bad Request',
    }, status=400)


def handler403(request):
    url = request.build_absolute_uri()
    return render(request, 'nhkstories/403.html', {
        'url': url,
        'title': 'Forbidden',
        'header': 'Forbidden',
    }, status=400)


def handler404(request):
    url = request.build_absolute_uri()
    return render(request, 'nhkstories/404.html', {
        'url': url,
        'title': 'Page Not Found',
        'header': 'Page Fot Found',
    }, status=404)


def handler500(request):
    url = request.build_absolute_uri()
    return render(request, 'nhkstories/500.html', {
        'url': url,
        'title': 'Server Error',
        'header': 'Server Error',
    }, status=500)


def external_error(request, code):
    if request.META.get('REQUEST_URI') == '/.well-known/assetlinks.json':
        return handler404(request)

    email_from = 'bugs@nhkeasier.com'
    email_to = 'contact@nhkeasier.com'
    email_subject = '[Django] Broken EXTERNAL link ({}) on {}'.format(code, request.META.get('SERVER_NAME'))
    email_body = (
        'Referrer: {}\r\n'
        'Requested URL: {}\r\n'
        'User agent: {}\r\n'
        'IP address: {}\r\n'.format(
        request.META.get('HTTP_REFERER'),
        request.META.get('REQUEST_URI'),
        request.META.get('HTTP_USER_AGENT'),
        request.META.get('REMOTE_ADDR'),
    ))
    send_mail(email_subject, email_body, email_from, [email_to])
    url = request.build_absolute_uri()
    error = {
        '400': 'Bad Request',
        '403': 'Forbidden',
        '404': 'Page Not Found',
        '500': 'Server Error',
    }.get(code, 'Unknown error')
    return render(request, 'nhkstories/{}.html'.format(code), {
        'url': url,
        'title': error,
        'header': error,
    }, status=int(code))



def archive(request, year=None, month=None, day=None):
    if year is not None and month is not None and day is not None:
        try:
            day = date(int(year), int(month), int(day))
        except ValueError:
            return handler400(request)
        header = 'Stories on {}'.format(day)
    else:
        day = Story.objects.order_by('-published').first().published.date()
        header = 'Latest Stories'

    stories = Story.objects.exclude(content_with_ruby=None).filter(published__date=day).order_by('-published', '-id')
    if not stories:
        return handler404(request)

    # information for links (canonical URL, links to previous and next days)
    previous_day = Story.objects.filter(published__date__lt=day).order_by('-published').first()
    next_day = Story.objects.filter(published__date__gt=day).order_by('published').first()
    date_info = (day.year, '{:02}'.format(day.month), '{:02}'.format(day.day))
    url = request.build_absolute_uri(reverse('nhkstories:archive', args=date_info))

    # take the image of one of the story as page illustration, if any
    illustrated_story = stories.exclude(image='').first()
    if illustrated_story is not None:
        image = request.build_absolute_uri(illustrated_story.image.url)
    else:
        image = None

    return render(request, 'nhkstories/index.html', {
        'url': url,
        'title': 'Easier Japanese Practice',
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
    url = request.build_absolute_uri(reverse('nhkstories:story', args=(id,)))

    # take the image of the story as page illustration, if any
    image = request.build_absolute_uri(story.image.url) if story.image else None

    return render(request, 'nhkstories/story.html', {
        'url': url,
        'title': story.title,
        'header': 'Single Story',
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
