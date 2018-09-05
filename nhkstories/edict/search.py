# encoding: utf-8
import re
import os.path

from .furigana import furigana_from_kanji_kana

# default filenames
default_edict = os.path.join(os.path.dirname(__file__), 'edict2')
default_enamdict = os.path.join(os.path.dirname(__file__), 'enamdict')
default_edict_index = os.path.join(os.path.dirname(__file__), 'edict2_index')
default_enamdict_index = os.path.join(os.path.dirname(__file__), 'enamdict_index')

# pre-compile regular expressions
edict_line_pattern = re.compile(r'(?m)(^(\S*) (?:\[(\S*?)\] )?/(.*)/.*$)')
common_marker = re.compile(r'\([^)]*\)')
gloss_pattern = re.compile(r'^(?:\(([^0-9]\S*)\) )?(?:\(([0-9]+)\) )?(.*)')


class Word:
    def __init__(self, writings, readings, glosses, edict_entry, edict_offset=None):
        self.writings = writings
        self.readings = readings
        self.glosses = glosses
        self.edict_entry = edict_entry
        self.edict_offset = edict_offset

        self.kanji = self.writings[0]
        self.kana = self.readings[0] if self.readings else self.kanji

        self._furigana = None

    def __repr__(self):
        r = u'<{}>'.format(self.kanji)
        if isinstance(r, str):  # Python 3
            return r
        else:  # Python 2
            return r.encode('utf-8')

    def get_sequence_number(self):
        last_gloss = self.glosses.split(u'/')[-1]
        return last_gloss if last_gloss[:4] == u'EntL' else None

    def get_furigana(self):
        if self._furigana is None:
            kanji = self.kanji
            kana = self.kana
            self._furigana = furigana_from_kanji_kana(kanji, kana)
        return self._furigana

    def get_meanings(self):
        # pre-parse glosses
        glosses = self.glosses.split(u'/')
        glosses = [
            gloss for gloss in glosses
            if gloss != u'(P)' and not gloss.startswith(u'EntL')
        ]

        # regroup meanings
        meanings = []
        last_meaning = []
        for gloss in glosses:
            match = gloss_pattern.match(gloss)
            nature, meaning_id, meaning = match.groups()
            if meaning_id and last_meaning:
                meanings.append(u'; '.join(last_meaning))
                last_meaning = []
            last_meaning.append(meaning)
        meanings.append(u'; '.join(last_meaning))
        return meanings

    def get_meanings_html(self):
        meanings = self.get_meanings()
        if len(meanings) == 1:
            return meanings[0]
        items = (u'<li>%s</li>' % meaning for meaning in meanings)
        list_ = u'<ol>%s</ol>' % u''.join(items)
        return list_

    def get_type(self):
        """Return type mask for deinflections"""
        type_ = 1<<7
        if re.search(r'\bv1\b', self.glosses):
            type_ |= 1<<0
        if re.search(r'\bv5.\b', self.glosses):
            type_ |= 1<<1
        if re.search(r'\badj-i\b', self.glosses):
            type_ |= 1<<2
        if re.search(r'\bvk\b', self.glosses):
            type_ |= 1<<3
        if re.search(r'\bvs\b', self.glosses):
            type_ |= 1<<4
        return type_


def search_index(word, filename=default_edict_index):
    word = word.encode('utf-8')
    with open(filename, mode='rb') as f:
        low = 0
        f.seek(0, 2)
        high = f.tell()
        last_line = None
        while high - low > 1:
            middle = (high + low) // 2
            f.seek(middle, 0)
            f.readline()  # get to end of current line

            line = f.readline()
            if not line:
                break
            last_line = line

            current_word, offsets = line.split(b' ', 1)
            if current_word == word:
                for offset in offsets.split(b' '):
                    yield int(offset)
                return
            if word < current_word:
                high = middle
            elif word > current_word:
                low = middle


def search_edict(word, edict_filename=default_edict, index_filename=default_edict_index):
    with open(edict_filename, mode='rb') as f:
        for offset in search_index(word, index_filename):
            f.seek(offset, 0)
            line = f.readline().decode('euc_jp')
            match = edict_line_pattern.match(line)
            if match is None:
                continue
            line, writings, readings, glosses = match.groups()
            writings = common_marker.sub(u'', writings).split(u';')
            readings = common_marker.sub(u'', readings).split(u';') if readings else []
            yield Word(writings, readings, glosses, line)


def search_enamdict(word, enamdict_filename=default_enamdict, index_filename=default_enamdict_index):
    return search_edict(word, enamdict_filename, index_filename)
