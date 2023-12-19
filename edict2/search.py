import os.path
import re
from typing import Iterator, Tuple

# default filenames
default_edict = os.path.join(os.path.dirname(__file__), 'edict2')
default_enamdict = os.path.join(os.path.dirname(__file__), 'enamdict')

# pre-compile regular expressions
edict_line_pattern = re.compile(rb'(?m)^(\S*) (?:\[(\S*?)\] )?/(?:\(([^)]*?)\))?')
common_marker = re.compile(rb'\([^)]*\)')

EdictEntry = Tuple[bytes, int]


def type_from_glosses(glosses: bytes) -> int:
    """Return type mask for deinflections"""
    type_ = 1 << 7
    if glosses:
        for gloss in glosses.split(b','):
            if gloss == b'v1':
                type_ |= 1 << 0
            elif gloss.startswith(b'v5'):
                type_ |= 1 << 1
            elif gloss == b'adj-i':
                type_ |= 1 << 2
            elif gloss == b'vk':
                type_ |= 1 << 3
            elif gloss == b'vs' or gloss.startswith(b'vs-'):
                type_ |= 1 << 4
    return type_


class Edict:
    def __init__(self, filename: str = default_edict):
        self.words: dict[str, EdictEntry | list[EdictEntry]] = {}
        with open(filename, mode='rb') as f:
            lines = iter(f)
            next(lines)  # skip header
            for line in lines:
                match = edict_line_pattern.match(line)
                if not match:
                    continue

                # gather information for new word
                swritings, sreadings, glosses = match.groups()
                entry = (line.strip(), type_from_glosses(glosses))

                # map writings and reading to entry
                writings = common_marker.sub(b'', swritings).split(b';')
                readings = common_marker.sub(b'', sreadings).split(b';') if sreadings else []
                for key in writings + readings:
                    decoded_key = key.decode()
                    try:
                        entries = self.words[decoded_key]
                    except KeyError:
                        self.words[decoded_key] = entry
                    else:
                        if isinstance(entries, list):
                            entries.append(entry)
                        else:
                            self.words[decoded_key] = [entries, entry]

    def search(self, word: str) -> Iterator[EdictEntry]:
        try:
            entries = self.words[word]
        except KeyError:
            return
        else:
            if isinstance(entries, list):
                yield from entries
            else:
                yield entries


edict = Edict(default_edict)
enamdict = Edict(default_enamdict)
