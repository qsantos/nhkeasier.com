# encoding: utf-8
import re
import sys

from .search import search_enamdict
from .deinflect import Deinflector

ranges = [
    u'々',  # IDEOGRAPHIC ITERATION MARK (U+3005)
    u'\u3040-\u30ff',  # Hiragana, Katakana
    u'\u3400-\u4dbf',  # CJK Unified Ideographs Extension A
    u'\u4e00-\u9fff',  # CJK Unified Ideographs
    u'\uf900-\ufaff',  # CJK Compatibility Ideographs
    u'\uff66-\uff9f',  # Halfwidth and Fullwidth Forms Block (hiragana and katakana)
]
fragment_pattern = re.compile(u'[{}]+'.format(u''.join(ranges)))


def japanese_text_substrings(text):
    for fragment in fragment_pattern.finditer(text):
        fragment = fragment.group()
        for start in range(0, len(fragment)):
            for stop in reversed(range(start+1, len(fragment)+1)):
                yield fragment[start:stop]


def create_subedict(text):
    """List EDICT items that might be present in text"""
    deinflector = Deinflector()
    # we directly store the set of EDICT entries (text lines), so that
    # duplicates are naturally eliminated
    items = set()
    for substring in japanese_text_substrings(text):
        for candidate, type_, reason in deinflector(substring):
            for word in deinflector.search_edict(candidate):
                if word.get_type() & type_:
                    items.add(word.edict_entry)
    return items


def create_subenamdict(text):
    """List EDICT items that might be present in text"""
    items = set()
    for substring in japanese_text_substrings(text):
        for word in search_enamdict(substring):
            items.add(word.edict_entry)
    return items


def save_subedict(subedict, filename):
    with open(filename, 'wb') as f:
        content = u'\n'.join(sorted(subedict)) + u'\n'
        f.write(content.encode('utf-8'))


def main():
    import argparse

    # parse arguments
    parser = argparse.ArgumentParser()
    parser.description = 'Reduce EDICT and ENAMDICT to cover given text'
    parser.add_argument('text_file')
    parser.add_argument('subedict_file')
    parser.add_argument('subenamdict_file')
    args = parser.parse_args()

    # read text file
    with open(args.text_file, 'rb') as f:
        text = f.read().decode('utf-8')

    # actually do the work
    subedict = create_subedict(text)
    subenamdict = create_subenamdict(text)
    save_subedict(subedict, args.subedict_file)
    save_subedict(subenamdict, args.subenamdict_file)


if __name__ == '__main__':
    main()
