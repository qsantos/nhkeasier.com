#!/usr/bin/env python
import re
from collections import defaultdict
from typing import DefaultDict, Dict, Set

from .search import (
    Word,
    common_marker,
    default_edict,
    default_edict_index,
    default_enamdict,
    default_enamdict_index,
    edict_line_pattern,
)

byte_line_pattern = re.compile(b'(?m)^(.*)$')

Edict = Dict[str, Set[Word]]


def load_edict(filename: str) -> Edict:
    with open(filename, mode='rb') as f:
        edict_data = f.read()

    edict: DefaultDict[str, Set[Word]] = defaultdict(set)
    for match in byte_line_pattern.finditer(edict_data):
        # get byte offset of line
        offset = match.start()

        # parse line
        line = match.group(0).decode('euc_jp')
        match2 = edict_line_pattern.match(line)
        if not match2:
            continue

        # gather information for new word
        line, swritings, sreadings, glosses = match2.groups()
        writings = common_marker.sub('', swritings).split(';')
        readings = common_marker.sub('', sreadings).split(';') if sreadings else []
        word = Word(writings, readings, glosses, line, offset)

        # map writings and reading to word
        for key in writings + readings:
            edict[key].add(word)
    return edict


def build_index(input_filename: str, output_filename: str) -> None:
    edict = load_edict(input_filename)
    with open(output_filename, 'wb') as f:
        for key in sorted(edict):
            words = edict[key]
            offsets = sorted(word.edict_offset for word in words)
            offsets_string = ' '.join(str(offset) for offset in offsets)
            f.write(f'{key} {offsets_string}\n'.encode())


if __name__ == '__main__':
    build_index(default_edict, default_edict_index)
    build_index(default_enamdict, default_enamdict_index)
