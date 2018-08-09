import re
import os.path

from .deinflect import Deinflector


fragment_pattern = re.compile(r'[\u25cb\u3004-\u30ff\u3400-\u4dbf\u4e00-\u9fff\uf900-\ufaff\uff66-\uff9f]+')

default_deinflect = os.path.join(os.path.dirname(__file__), 'deinflect.dat')

def _iter_subfragments(text):
    for fragment in fragment_pattern.finditer(text):
        fragment = fragment.group()
        for start in range(0, len(fragment)):
            for stop in reversed(range(start+1, len(fragment)+1)):
                yield fragment[start:stop]


def filter_edict(dictionary, text):
    """List EDICT items that might be present in text"""
    deinflector = Deinflector(default_deinflect)
    items = set()
    for subfragment in _iter_subfragments(text):
        for candidate, type1, reason in deinflector(subfragment):
            for type2, line in dictionary[candidate]:
                if type1 and not (type1 & type2):
                    continue
                # we don't care about the particular reason
                items.add(line)
    return items


def save_subedict(subedict, filename):
    with open(filename, 'w') as f:
        f.write(''.join(sorted(subedict)))


def main():
    import argparse
    from .parse import load_edict

    # parse arguments
    parser = argparse.ArgumentParser()
    parser.description = 'Reduce EDICT file to cover given text'
    parser.add_argument('edict_file')
    parser.add_argument('text_file')
    args = parser.parse_args()

    # load EDICT file
    edict = load_edict(args.edict_file)

    # read text file
    with open(args.text_file) as f:
        text = f.read()

    # actually do the work
    subedict = filter_edict(edict, text)

    # sort and output
    output = ''.join(sorted(subedict))
    print(output, end='')


if __name__ == '__main__':
    main()
