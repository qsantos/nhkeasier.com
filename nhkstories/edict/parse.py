import re
import gzip
import os.path
from collections import defaultdict


default_edict = os.path.join(os.path.dirname(__file__), 'edict.gz')
default_enamdict = os.path.join(os.path.dirname(__file__), 'enamdict.gz')


def load_edict(filename=default_edict):
    """Parse file in EDICT format"""
    # see <http://ftp.monash.edu/pub/nihongo/edict_doc.html>
    # first line is a header
    # other lines are of the form
    #    KANJI [KANA] /(general information) gloss/gloss/.../
    # or
    #    KANA /(general information) gloss/gloss/.../
    # "general information" may appear for each sense of a word: (1), (2), etc
    # general information notably indicates the grammatical class (Part of Speech)
    line_pattern = re.compile(r'(\S*)\s+(?:\[(.*?)\])?')
    gloss_pattern = re.compile(r'/((?:\(.*?\) )*)')
    edict = defaultdict(set)
    with gzip.open(filename, mode='r') as f:
        for line in f:
            line = line.decode('euc_jp')
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
            edict[word].add((type, line))
            if kana:
                edict[kana].add((type, line))
    return edict


def load_enamdict(filename=default_enamdict):
    return load_edict(filename)
