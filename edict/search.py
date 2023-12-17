import os.path
import re
from typing import Iterator, Tuple

# default filenames
default_edict = os.path.join(os.path.dirname(__file__), 'edict2')
default_enamdict = os.path.join(os.path.dirname(__file__), 'enamdict')

# pre-compile regular expressions
edict_line_pattern = re.compile(rb'(?m)^(\S*) (?:\[(\S*?)\] )?/(.*)/')
common_marker = re.compile(rb'\([^)]*\)')


def type_from_glosses(glosses: bytes) -> int:
    """Return type mask for deinflections"""
    type_ = 1 << 7
    if re.search(rb'\bv1\b', glosses):
        type_ |= 1 << 0
    if re.search(rb'\bv5.\b', glosses):
        type_ |= 1 << 1
    if re.search(rb'\badj-i\b', glosses):
        type_ |= 1 << 2
    if re.search(rb'\bvk\b', glosses):
        type_ |= 1 << 3
    if re.search(rb'\bvs\b', glosses):
        type_ |= 1 << 4
    return type_


class Edict:
    def __init__(self, filename: str = default_edict):
        self.words = {}
        with open(filename, mode='rb') as f:
            for line in f:
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
                    decoded_key = key.decode('euc-jp')
                    try:
                        entries = self.words[decoded_key]
                    except KeyError:
                        self.words[decoded_key] = entry
                    else:
                        if isinstance(entries, list):
                            entries.append(entry)
                        else:
                            self.words[decoded_key] = [entries, entry]

    def search(self, word: str) -> Iterator[Tuple[str, int]]:
        try:
            entries = self.words[word]
        except KeyError:
            return
        else:
            if isinstance(entries, list):
                yield from self.words[word]
            else:
                yield entries


edict = Edict(default_edict)
enamdict = Edict(default_enamdict)
