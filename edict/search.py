import os.path
import re
from typing import Iterator

# default filenames
default_edict = os.path.join(os.path.dirname(__file__), 'edict2')
default_enamdict = os.path.join(os.path.dirname(__file__), 'enamdict')

# pre-compile regular expressions
edict_line_pattern = re.compile(r'(?m)^(\S*) (?:\[(\S*?)\] )?/(.*)/')
common_marker = re.compile(r'\([^)]*\)')


class EdictEntry:
    def __init__(
        self,
        edict_entry: str,  # full Edict entry corresponding to the word
        type_: int,
    ):
        self.edict_entry = edict_entry
        self.type_ = type_

    def __repr__(self) -> str:
        return f'<{self.kanji}>'

    @staticmethod
    def get_type(glosses: str) -> int:
        """Return type mask for deinflections"""
        type_ = 1 << 7
        if re.search(r'\bv1\b', glosses):
            type_ |= 1 << 0
        if re.search(r'\bv5.\b', glosses):
            type_ |= 1 << 1
        if re.search(r'\badj-i\b', glosses):
            type_ |= 1 << 2
        if re.search(r'\bvk\b', glosses):
            type_ |= 1 << 3
        if re.search(r'\bvs\b', glosses):
            type_ |= 1 << 4
        return type_


class Edict:
    def __init__(self, filename: str = default_edict):
        self.words = {}
        with open(filename, encoding='euc_jp') as f:
            for line in f:
                match = edict_line_pattern.match(line)
                if not match:
                    continue

                # gather information for new word
                swritings, sreadings, glosses = match.groups()
                entry = EdictEntry(line.strip(), EdictEntry.get_type(glosses))

                # map writings and reading to entry
                writings = common_marker.sub('', swritings).split(';')
                readings = common_marker.sub('', sreadings).split(';') if sreadings else []
                for key in writings + readings:
                    try:
                        self.words[key].add(entry)
                    except KeyError:
                        self.words[key] = {entry}

    def search(self, word: str) -> Iterator[EdictEntry]:
        try:
            yield from self.words[word]
        except KeyError:
            return


edict = Edict(default_edict)
enamdict = Edict(default_enamdict)
