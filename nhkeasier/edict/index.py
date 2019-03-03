#!/usr/bin/env python
from collections import defaultdict

from .search import *

byte_line_pattern = re.compile(b'(?m)^(.*)$')


def load_edict(filename):
    with open(filename, mode='rb') as f:
        edict_data = f.read()

    edict = defaultdict(set)
    for match in byte_line_pattern.finditer(edict_data):
        # get byte offset of line
        offset = match.start()

        # parse line
        line = match.group(0).decode('euc_jp')
        match = edict_line_pattern.match(line)
        if not match:
            continue

        # gather information for new word
        line, writings, readings, glosses = match.groups()
        writings = common_marker.sub(u'', writings).split(u';')
        readings = common_marker.sub(u'', readings).split(u';') if readings else []
        word = Word(writings, readings, glosses, line, offset)

        # map writings and reading to word
        for key in writings + readings:
            edict[key].add(word)
    return edict


def build_index(input_filename, output_filename):
    edict = load_edict(input_filename)
    with open(output_filename, 'wb') as f:
        for key in sorted(edict):
            words = edict[key]
            offsets = sorted(word.edict_offset for word in words)
            offsets = u' '.join(str(offset) for offset in offsets)
            line = u'{} {}\n'.format(key, offsets)
            f.write(line.encode('utf-8'))


if __name__ == '__main__':
    build_index(default_edict, default_edict_index)
    build_index(default_enamdict, default_enamdict_index)
