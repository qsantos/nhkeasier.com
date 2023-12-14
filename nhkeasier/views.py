import re
from datetime import date

from django.shortcuts import get_object_or_404, redirect, render
from django.urls import reverse
from django.views.decorators.clickjacking import xframe_options_exempt

from .models import Story


def simple_message(request, title: str, message: str, status: int = 200):
    return render(request, 'message.html', {
        'title': title,
        'header': title,
        'message': message,
    }, status=status)


def handler400(request, exception):
    return simple_message(request, 'Bad Request', 'Sorry, we were not able to handle the request you sent us. Please check that it is formatted correctly.', 400)


def handler403(request, exception):
    return simple_message(request, 'Forbidden', 'Sorry, the permissions of this document are not configured properly to let you access it.', 403)


def handler404(request, exception):
    return simple_message(request, 'Page Not Found', 'Sorry, we could not find the page you requested. Maybe the URL you followed is incomplete, or the document has been moved.', 404)


def handler500(request):
    return simple_message(request, 'Server Error', 'Sorry, something went very wrong on the server and we were not able to display the requested document.', 500)


def external_error(request, code):
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


def archive(request, year=None, month=None, day=None):
    stories = Story.objects.filter(subedict_created=True)

    if year is not None and month is not None and day is not None:
        try:
            day = date(int(year), int(month), int(day))
        except ValueError as e:
            return handler400(request, e)
        header = f'Stories on {day}'
    elif stories.count():
        day = stories.order_by('-published').first().published.date()
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
    previous_day = stories.filter(published__date__lt=day).order_by('-published').first()
    next_day = stories.filter(published__date__gt=day).order_by('published').first()

    stories = stories.filter(published__date=day).order_by('-published', '-id')
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
        'day': day,
        'next_day': next_day,
    })


def remove_all_html(content):
    return re.sub('<.*?>', '', content)


def story(request, id):
    story = get_object_or_404(Story, pk=id)
    if not story.subedict_created:
        return handler404(request, None)

    # information for links (canonical URL, links to previous and next stories)
    previous_stories = Story.objects.filter(published__date=story.published.date(), id__lt=story.id) | Story.objects.filter(published__date__lt=story.published.date())
    previous_story = previous_stories.order_by('-published', '-id').first()
    next_stories = Story.objects.filter(published__date=story.published.date(), id__gt=story.id) | Story.objects.filter(published__date__gt=story.published.date())
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
def player(request, id):
    story = get_object_or_404(Story, pk=id)
    if not story.video_reencoded:
        return handler404(request, None)

    autoplay = bool(request.GET.get('autoplay'))
    return render(request, 'player.html', {
        'story': story,
        'autoplay': autoplay,
    })


def about(request):
    return render(request, 'about.html', {
        'title': 'About',
        'header': 'About',
    })


def tools(request):
    return redirect('about')
