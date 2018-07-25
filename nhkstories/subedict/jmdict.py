import re
import gzip
import os.path

from .deinflect import Deinflector


default_edict = os.path.join(os.path.dirname(__file__), 'edict.gz')
default_enamdict = os.path.join(os.path.dirname(__file__), 'enamdict.gz')
default_deinflect = os.path.join(os.path.dirname(__file__), 'deinflect.dat')

line_pattern = re.compile(r'(\S*)\s+(?:\[(.*?)\])?')
gloss_pattern = re.compile(r'/((?:\(.*?\) )*)')
fragment_pattern = re.compile(r'[\u25cb\u3004-\u30ff\u3400-\u4dbf\u4e00-\u9fff\uf900-\ufaff\uff66-\uff9f]+')


def load(f):
    """Parse file in EDICT format"""
    # see <http://ftp.monash.edu/pub/nihongo/edict_doc.html>
    # first line is a header
    # other lines are of the form
    #    KANJI [KANA] /(general information) gloss/gloss/.../
    # or
    #    KANA /(general information) gloss/gloss/.../
    # "general information" may appear for each sense of a word: (1), (2), etc
    # general information notably indicates the grammatical class (Part of Speech)
    dictionary = {}
    for line in f:
        m = line_pattern.match(line)
        if not m:
            continue
        word, kana = m.groups()

        # determine type mask for deinflections (see below)
        type = 0
        for match in gloss_pattern.finditer(line):
            if not match:
                continue
            general_information = match.group(1)
            if 'v1' in general_information:
                type |= 1<<8
            if 'v5' in general_information:
                type |= 1<<9
            if 'adj-i' in general_information:
                type |= 1<<10
            if 'vk' in general_information:
                type |= 1<<11
            if 'vs' in general_information:
                type |= 1<<12

        # insert both kanji and kana version when relevant
        dictionary[word] = type, line
        if kana:
            dictionary[kana] = type, line
    return dictionary


def load_from_filename(filename):
    # open as gzipped or raw file, as UTf-8 or EUC-JP
    opener = gzip.open if filename.endswith('.gz') else open
    try:
        with opener(filename, mode='rt') as f:
            return load(f)
    except UnicodeDecodeError:
        with opener(filename, mode='rt', encoding='euc-jp') as f:
            return load(f)


def _iter_subfragments(text):
    for fragment in fragment_pattern.finditer(text):
        fragment = fragment.group()
        for start in range(0, len(fragment)):
            for stop in reversed(range(start+1, len(fragment)+1)):
                yield fragment[start:stop]


def subedict(dictionary, text):
    """List EDICT items that might be present in text"""
    deinflector = Deinflector(default_deinflect)
    items = set()
    for subfragment in _iter_subfragments(text):
        for candidate, type1, reason in deinflector(subfragment):
            if candidate not in dictionary:
                continue
            type2, line = dictionary[candidate]
            if type1 and not (type1 & type2):
                continue
            # we don't care about the reason, and it may generate duplicate
            items.add(line)
    return items


def save_subedict(subedict, filename):
    with open(filename, 'w') as f:
        f.write(''.join(sorted(subedict)))
