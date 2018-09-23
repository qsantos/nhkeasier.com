import re
from datetime import date

from django.http import HttpResponse
from django.urls import reverse
from django.utils import timezone
from django.core.paginator import Paginator, PageNotAnInteger, EmptyPage
from django.shortcuts import render, get_object_or_404, redirect
from django.core.mail import send_mail

from .models import Story
from .forms import ContactForm


def simple_message(request, title, message, status=200):
    return render(request, 'nhkstories/message.html', {
        'title': title,
        'header': title,
        'message': message,
    }, status=status)


def handler400(request):
    return simple_message(request, 'Bad Request', 'Sorry, we were not able to handle the request you sent us. Please check that it is formatted correctly.', 400)


def handler403(request):
    return simple_message(request, 'Forbidden', 'Sorry, the permissions of this document are not configured properly to let you access it.', 403)


def handler404(request):
    return simple_message(request, 'Page Not Found', 'Sorry, we could not find the page you requested. Maybe the URL you followed is incomplete, or the document has been moved.', 404)


def handler500(request):
    return simple_message(request, 'Server Error', 'Sorry, something went very wrong on the server and we were not able to display the requested document.', 500)


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
    return {
        '400': handler400,
        '403': handler403,
        '404': handler404,
    }.get(code, handler500)(request)


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

    # take the image of one of the story as page illustration, if any
    illustrated_story = stories.exclude(image='').first()
    if illustrated_story is not None:
        image = request.build_absolute_uri(illustrated_story.image.url)
    else:
        image = None

    return render(request, 'nhkstories/index.html', {
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

    # take the image of the story as page illustration, if any
    image = request.build_absolute_uri(story.image.url) if story.image else None

    return render(request, 'nhkstories/story.html', {
        'title': story.title,
        'header': 'Single Story',
        'description': remove_all_html(story.content),
        'image': image,
        'story': story,
        'previous_story': previous_story,
        'next_story': next_story,
    })


def about(request):
    return render(request, 'nhkstories/about.html', {
        'title': 'About',
        'header': 'About',
    })


def contact(request):
    if request.method == 'GET':
        form = ContactForm()
    else:
        form = ContactForm(request.POST)
    if form.is_valid():
        from_email = form.cleaned_data['from_email']
        subject = '[NHKEasier] {}'.format(form.cleaned_data['subject'])
        message = form.cleaned_data['message']
        if send_mail(subject, message, from_email, ['contact@nhkeasier.com']) != 1:
            return simple_message(request, 'Message Not Sent', 'Sorry, there was en error while sending your message. Please try again later. You should be return to the form using the “Back” button of your web browser without', 500)
        return redirect('nhkstories:contact_sent')
    else:
        return render(request, 'nhkstories/contact.html', {
            'title': 'Contact',
            'header': 'Contact',
            'form': form,
        })

def contact_sent(request):
    return simple_message(request, 'Message Sent', 'Thank you for your feedback. We will take your message under consideration as soon as possible.')
