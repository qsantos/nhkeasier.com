import datetime
import re

from django.core.mail import send_mail
from django.http import HttpRequest, HttpResponse
from django.shortcuts import get_object_or_404, redirect, render
from django.urls import reverse
from django.views.decorators.clickjacking import xframe_options_exempt

from .forms import ContactForm
from .models import Story


def simple_message(request: HttpRequest, title: str, message: str, status: int = 200) -> HttpResponse:
    return render(request, 'message.html', {
        'title': title,
        'header': title,
        'message': message,
    }, status=status)


def handler400(request: HttpRequest, _exception: Exception | None) -> HttpResponse:
    return simple_message(
        request,
        'Bad Request',
        (
            'Sorry, we were not able to handle the request you sent us. '
            'Please check that it is formatted correctly.'
        ),
        400,
    )


def handler403(request: HttpRequest, _exception: Exception | None) -> HttpResponse:
    return simple_message(
        request,
        'Forbidden',
        (
            'Sorry, the permissions of this document are not configured '
            'properly to let you access it.'
        ),
        403,
    )


def handler404(request: HttpRequest, _exception: Exception | None) -> HttpResponse:
    return simple_message(
        request,
        'Page Not Found',
        (
            'Sorry, we could not find the page you requested. Maybe the URL '
            'you followed is incomplete, or the document has been moved.'
        ),
        404,
    )


def handler500(request: HttpRequest) -> HttpResponse:
    return simple_message(
        request,
        'Server Error',
        (
            'Sorry, something went very wrong on the server and we were not '
            'able to display the requested document.'
        ),
        500,
    )


def external_error(request: HttpRequest, code: str) -> HttpResponse:
    try:
        handler = {
            '400': handler400,
            '403': handler403,
            '404': handler404,
        }[code]
    except KeyError:
        return handler500(request)
    else:
        return handler(request, None)


def archive(
    request: HttpRequest,
    year: str | None = None,
    month: str | None = None,
    day: str | None = None,
) -> HttpResponse:
    stories = Story.objects.filter(subedict_created=True)

    if year is not None and month is not None and day is not None:
        try:
            date = datetime.date(int(year), int(month), int(day))
        except ValueError as e:
            return handler400(request, e)
        header = f'Stories on {date}'
    elif stories.count():
        story = stories.order_by('-published').first()
        if story is None:
            return handler404(request, None)
        date = story.published.date()
        header = 'Latest Stories'
    else:
        return render(request, 'index.html', {
            'title': 'Easier Japanese Practice',
            'header': 'Japanese stories here soon!',
            'description':
                'Come practice reading and listening to Japanese with recent news '
                'stories! Simple vocabulary, simple kanji and simple sentence '
                'structures, as well as kanji readings (furigana) and an '
                'integrated dictionary will let you train until you get more '
                'comfortable for harder materials.',
        })

    # information for links (canonical URL, links to previous and next days)
    previous_day = stories.filter(published__date__lt=date).order_by('-published').first()
    next_day = stories.filter(published__date__gt=date).order_by('published').first()

    stories = stories.filter(published__date=date).order_by('-published', '-id')
    if not stories:
        return handler404(request, None)

    # select interesting story
    story = stories.exclude(video_reencoded='').first()
    if story is None:
        story = stories.exclude(image='').first()
    # media for OpenGraph and such
    if story is not None and story.video_reencoded:
        player = request.build_absolute_uri(reverse('player', args=[story.id]))
    else:
        player = None
    image = request.build_absolute_uri(story.image.url) if story is not None and story.image else None

    return render(request, 'index.html', {
        'title': 'Easier Japanese Practice',
        'header': header,
        'description':
            'Come practice reading and listening to Japanese with recent news '
            'stories! Simple vocabulary, simple kanji and simple sentence '
            'structures, as well as kanji readings (furigana) and an '
            'integrated dictionary will let you train until you get more '
            'comfortable for harder materials.',
        'image': image,
        'player': player,
        'stories': stories,
        'previous_day': previous_day,
        'day': date,
        'next_day': next_day,
    })


def remove_all_html(content: str) -> str:
    return re.sub('<.*?>', '', content)


def story(request: HttpRequest, id: str) -> HttpResponse:
    story = get_object_or_404(Story, pk=id)
    if not story.subedict_created:
        return handler404(request, None)

    # information for links (canonical URL, links to previous and next stories)
    # previous story
    stores_previous_ids = Story.objects.filter(published__date=story.published.date(), id__lt=story.id)
    stories_previous_days = Story.objects.filter(published__date__lt=story.published.date())
    previous_stories = stores_previous_ids | stories_previous_days
    previous_story = previous_stories.order_by('-published', '-id').first()
    # next story
    stories_next_ids = Story.objects.filter(published__date=story.published.date(), id__gt=story.id)
    stories_next_days = Story.objects.filter(published__date__gt=story.published.date())
    next_stories = stories_next_ids | stories_next_days
    next_story = next_stories.order_by('published', 'id').first()

    # media for OpenGraph and such
    image = request.build_absolute_uri(story.image.url) if story.image else None
    player = request.build_absolute_uri(reverse('player', args=[story.id])) if story.video_reencoded else None

    return render(request, 'story.html', {
        'title': story.title,
        'header': 'Single Story',
        'description': remove_all_html(story.content),
        'image': image,
        'player': player,
        'story': story,
        'previous_story': previous_story,
        'next_story': next_story,
    })


@xframe_options_exempt
def player(request: HttpRequest, id: str) -> HttpResponse:
    story = get_object_or_404(Story, pk=id)
    if not story.video_reencoded:
        return handler404(request, None)

    autoplay = bool(request.GET.get('autoplay'))
    return render(request, 'player.html', {
        'story': story,
        'autoplay': autoplay,
    })


def about(request: HttpRequest) -> HttpResponse:
    return render(request, 'about.html', {
        'title': 'About',
        'header': 'About',
    })


def tools(_request: HttpRequest) -> HttpResponse:
    return redirect('about')


def contact(request):
    form = ContactForm() if request.method == 'GET' else ContactForm(request.POST)
    if form.is_valid():
        from_email = form.cleaned_data['from_email']
        subject = '[NHKEasier] {}'.format(form.cleaned_data['subject'])
        message = form.cleaned_data['message']
        if send_mail(subject, message, from_email, ['contact@nhkeasier.com']) != 1:
            return simple_message(
                request,
                'Message Not Sent',
                (
                    'Sorry, there was en error while sending your message. '
                    'Please try again later. You should be return to the form '
                    'using the “Back” button of your web browser without'
                ),
                500,
            )
        return redirect('contact_sent')
    return render(request, 'contact.html', {
        'title': 'Contact',
        'header': 'Contact',
        'form': form,
    })


def contact_sent(request):
    return simple_message(
        request,
        'Message Sent',
        (
            'Thank you for your feedback. We will take your message under '
            'consideration as soon as possible.'
        ),
    )
