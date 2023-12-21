import datetime
import re

from django.core.mail import mail_admins
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


def handler400(request: HttpRequest, exception: Exception | None) -> HttpResponse:
    # the argument really need to be named "exception"
    _ = exception
    return simple_message(
        request,
        'Bad Request',
        (
            'Sorry, we were not able to handle the request you sent us. '
            'Please check that it is formatted correctly.'
        ),
        400,
    )


def handler403(request: HttpRequest, exception: Exception | None) -> HttpResponse:
    # the argument really need to be named "exception"
    _ = exception
    return simple_message(
        request,
        'Forbidden',
        (
            'Sorry, the permissions of this document are not configured '
            'properly to let you access it.'
        ),
        403,
    )


def handler404(request: HttpRequest, exception: Exception | None) -> HttpResponse:
    # the argument really need to be named "exception"
    _ = exception
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
    else:
        # TODO: avoid doing a separate query for this
        story = stories.order_by('-published').first()
        if story is None:
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
        date = story.published.date()
        header = 'Latest Stories'

    # information for links (canonical URL, links to previous and next days)
    # NOTE: Django's ORM is way too slow at generating the SQL queries
    dt = str(date)
    try:
        previous_day = stories.raw(
            """
                SELECT "nhkeasier_story"."id", "nhkeasier_story"."published"
                FROM "nhkeasier_story"
                WHERE date("nhkeasier_story"."published") < %s
                ORDER BY "nhkeasier_story"."published" DESC
                LIMIT 1
            """,
            [dt],
        )[0]
    except IndexError:
        previous_day = None
    try:
        next_day = stories.raw(
            """
                SELECT "nhkeasier_story"."id", "nhkeasier_story"."published"
                FROM "nhkeasier_story"
                WHERE date("nhkeasier_story"."published") > %s
                ORDER BY "nhkeasier_story"."published" ASC
                LIMIT 1
            """,
            [dt],
        )[0]
    except IndexError:
        next_day = None

    midnight = datetime.datetime.combine(date, datetime.time(0))
    one_day = datetime.timedelta(days=1)
    stories = stories.filter(published__gte=midnight, published__lt=midnight + one_day)

    stories = stories.order_by('-published', '-id')
    if not stories:
        return handler404(request, None)

    # select interesting story
    try:
        story = next(story for story in stories if story.video_reencoded)
    except StopIteration:
        try:
            story = next(story for story in stories if story.image)
        except StopIteration:
            story = None
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
    # NOTE: Django's ORM is way too slow at generating the SQL queries
    dt = story.published.strftime('%Y-%m-%d %H:%M:%S')
    try:
        previous_story = Story.objects.raw(
            """
                SELECT id
                FROM nhkeasier_story
                WHERE (published, id) < (%s, %s)
                ORDER BY published DESC, id DESC
                LIMIT 1
            """,
            [dt, id],
        )[0]
    except IndexError:
        previous_story = None
    try:
        next_story = Story.objects.raw(
            """
                SELECT id
                FROM nhkeasier_story
                WHERE (published, id) > (%s, %s)
                ORDER BY published ASC, id ASC
                LIMIT 1
            """,
            [dt, id],
        )[0]
    except IndexError:
        next_story = None

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
        subject = '[NHKEasier] {}'.format(form.cleaned_data['subject'])
        message = form.cleaned_data['message']
        from_email = form.cleaned_data['from_email']
        mail_admins(subject, f'From: {from_email}\n\n{message}')
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
