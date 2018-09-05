# encoding: utf-8
import re
import os.path

default_kanjidic = os.path.join(os.path.dirname(__file__), 'kanjidic')


class Kanji:
    def __init__(self, character, readings, meanings):
        self.character = character
        self.readings = readings
        self.meanings = meanings

    def __repr__(self):
        r = u'.{}.'.format(self.character)
        if isinstance(r, str):  # Python 3
            return r
        else:  # Python 2
            return r.encode('utf-8')


try:  # Python 2
    chr = unichr
except NameError:  # Python 3
    pass

hiragana = [chr(i) for i in range(0x3040, 0x30A0)]
katakana = [chr(i) for i in range(0x30A0, 0x3100)]

def hiragana_to_katakana(s):
    return u''.join(katakana[hiragana.index(c)] if c in hiragana else c for c in s)

def katakana_to_hiragana(s):
    # NOTE: ignores ヷ ヸ ヹ ヺ
    return u''.join(hiragana[katakana.index(c)] if c in katakana else c for c in s)

assert hiragana_to_katakana(u'くぼ.む') == u'クボ.ム'
assert katakana_to_hiragana(u'クボ.ム') == u'くぼ.む'

dakutens = {
    u'か': u'が', u'き': u'ぎ', u'く': u'ぐ', u'け': u'げ', u'こ': u'ご',
    u'さ': u'ざ', u'し': u'じ', u'す': u'ず', u'せ': u'ぜ', u'そ': u'ぞ',
    u'た': u'だ', u'ち': u'ぢ', u'つ': u'づ', u'て': u'で', u'と': u'ど',
    u'は': u'ばぱ', u'ひ': u'びぴ', u'ふ': u'ぶぷ', u'へ': u'べぺ', u'ほ': u'ぼぽ',
}


def normalize_readings(readings):
    # strip okurigana
    readings = {
        reading.split(u'.')[0] if u'.' in reading else reading
        for reading in readings
    }
    # remove "-"
    readings = {reading.replace(u'-', u'') for reading in readings}
    # convert to hirgana
    readings = {katakana_to_hiragana(reading) for reading in readings}
    return readings


def compound_readings(readings):
    gemination = {reading[:-1] + u'っ' for reading in readings}
    rendaku = {
        dakuten + reading[1:]
        for reading in readings
        for dakuten in dakutens.get(reading[0], set())
    }
    return gemination | rendaku


def load_kanjidic(filename=default_kanjidic):
    with open(filename, mode='rb') as f:
        edict_data = f.read().decode('euc_jp')

    kanjidic = {}
    line_pattern = re.compile(r'(?m)^(.) (?:[0-9A-F]{4}) (?:(?:[A-Z]\S*) )*([^{]*?) (?:T[^{]*?)?((?:\{.*?\} )*\{.*?\})')
    meaning_pattern = re.compile(r'{(.*?)}')
    for character, readings, meanings in line_pattern.findall(edict_data):
        # gather kanji information
        meanings = meaning_pattern.findall(meanings)
        readings = normalize_readings(readings.split())
        readings |= compound_readings(readings)
        kanji = Kanji(character, readings, meanings)

        # map character to kanji
        kanjidic[character] = kanji
    return kanjidic
