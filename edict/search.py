import os.path
import re
from typing import Iterator, List, Optional

from .furigana import furigana_from_kanji_kana

# default filenames
default_edict = os.path.join(os.path.dirname(__file__), 'edict2')
default_enamdict = os.path.join(os.path.dirname(__file__), 'enamdict')

# pre-compile regular expressions
edict_line_pattern = re.compile(r'(?m)(^(\S*) (?:\[(\S*?)\] )?/(.*)/.*$)')
common_marker = re.compile(r'\([^)]*\)')
gloss_pattern = re.compile(r'^(?:\(([^0-9]\S*)\) )?(?:\(([0-9]+)\) )?(.*)')


class Word:
    def __init__(
        self,
        writings: List[str],
        readings: List[str],
        glosses: str,
        edict_entry: str,  # full Edict entry corresponding to the word
        edict_offset: int = 0,  # offset position in bytes of the entry
    ):
        self.writings = writings
        self.readings = readings
        self.glosses = glosses
        self.edict_entry = edict_entry
        self.edict_offset = edict_offset

        self.kanji = self.writings[0]
        self.kana = self.readings[0] if self.readings else self.kanji

        self._furigana: Optional[str] = None

    def __repr__(self) -> str:
        return f'<{self.kanji}>'

    def get_sequence_number(self) -> Optional[str]:
        last_gloss = self.glosses.split('/')[-1]
        return last_gloss if last_gloss[:4] == 'EntL' else None

    def get_furigana(self) -> str:
        if self._furigana is None:
            kanji = self.kanji
            kana = self.kana
            self._furigana = furigana_from_kanji_kana(kanji, kana)
        return self._furigana

    def get_meanings(self) -> List[str]:
        # pre-parse glosses
        glosses = self.glosses.split('/')
        glosses = [
            gloss for gloss in glosses
            if gloss != '(P)' and not gloss.startswith('EntL')
        ]

        # regroup meanings
        meanings = []
        last_meaning: List[str] = []
        for gloss in glosses:
            match = gloss_pattern.match(gloss)
            assert match is not None
            nature, meaning_id, meaning = match.groups()
            if meaning_id and last_meaning:
                meanings.append('; '.join(last_meaning))
                last_meaning = []
            last_meaning.append(meaning)
        meanings.append('; '.join(last_meaning))
        return meanings

    def get_meanings_html(self) -> str:
        meanings = self.get_meanings()
        if len(meanings) == 1:
            return meanings[0]
        return '<ol>' + ''.join(f'<li>{meaning}</li>' for meaning in meanings) + '</ol>'

    def get_type(self) -> int:
        """Return type mask for deinflections"""
        type_ = 1 << 7
        if re.search(r'\bv1\b', self.glosses):
            type_ |= 1 << 0
        if re.search(r'\bv5.\b', self.glosses):
            type_ |= 1 << 1
        if re.search(r'\badj-i\b', self.glosses):
            type_ |= 1 << 2
        if re.search(r'\bvk\b', self.glosses):
            type_ |= 1 << 3
        if re.search(r'\bvs\b', self.glosses):
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
                entry, swritings, sreadings, glosses = match.groups()
                writings = common_marker.sub('', swritings).split(';')
                readings = common_marker.sub('', sreadings).split(';') if sreadings else []
                word = Word(writings, readings, glosses, entry, 0)

                # map writings and reading to word
                for key in writings + readings:
                    try:
                        self.words[key].add(word)
                    except KeyError:
                        self.words[key] = {word}

    def search(self, word: str) -> Iterator[Word]:
        try:
            yield from self.words[word]
        except KeyError:
            return


edict = Edict(default_edict)
enamdict = Edict(default_enamdict)
