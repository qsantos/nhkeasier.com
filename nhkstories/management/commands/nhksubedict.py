from django.core.management.base import BaseCommand
from nhkstories.models import Story
from nhkstories.subedict import jmdict


def update_subedict(subedict_filename, edict_filename, stories):
    # load current subedict
    with open(subedict_filename) as f:
        cur_subedict = set(f.readlines())

    # load full EDICT file
    edict = jmdict.load_from_filename(edict_filename)
    print('{} loaded'.format(edict_filename))

    # find subedict of selected stories
    text = ''.join(story.content for story in stories)
    new_subedict = jmdict.subedict(edict, text)

    # update subedict and print how many words were added
    dif_subedict = new_subedict - cur_subedict
    cur_subedict |= new_subedict
    print('{} new words ({} total)'.format(len(dif_subedict), len(cur_subedict)))

    # save new subedict
    with open(subedict_filename, 'w') as f:
        f.write(''.join(sorted(cur_subedict)))


class Command(BaseCommand):
    def add_arguments(self, parser):
        parser.add_argument('since', help='Starting date for articles to parse')

    def handle(self, *args, **options):
        since = options['since']
        stories = Story.objects.filter(published__date__gte=since)
        update_subedict('media/subedict.dat', jmdict.default_edict, stories)
        update_subedict('media/subenamdict.dat', jmdict.default_enamdict, stories)
