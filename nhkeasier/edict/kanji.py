import os.path
import re
from functools import lru_cache
from typing import Dict, Iterable, NamedTuple, Set

default_kanjidic = os.path.join(os.path.dirname(__file__), 'kanjidic')


class Kanji(NamedTuple):
    character: str
    readings: Set[str]
    meanings: Set[str]

    def __repr__(self) -> str:
        return f'.{self.character}.'


hiragana = [chr(i) for i in range(0x3040, 0x30A0)]
katakana = [chr(i) for i in range(0x30A0, 0x3100)]


def hiragana_to_katakana(s: str) -> str:
    return ''.join(
        katakana[hiragana.index(c)] if c in hiragana else c
        for c in s
    )


def katakana_to_hiragana(s: str) -> str:
    # NOTE: ignores ヷ ヸ ヹ ヺ
    return ''.join(
        hiragana[katakana.index(c)] if c in katakana else c
        for c in s
    )


assert hiragana_to_katakana('くぼ.む') == 'クボ.ム'
assert katakana_to_hiragana('クボ.ム') == 'くぼ.む'

dakutens = {
    'か': 'が', 'き': 'ぎ', 'く': 'ぐ', 'け': 'げ', 'こ': 'ご',
    'さ': 'ざ', 'し': 'じ', 'す': 'ず', 'せ': 'ぜ', 'そ': 'ぞ',
    'た': 'だ', 'ち': 'ぢ', 'つ': 'づ', 'て': 'で', 'と': 'ど',
    'は': 'ばぱ', 'ひ': 'びぴ', 'ふ': 'ぶぷ', 'へ': 'べぺ', 'ほ': 'ぼぽ',
}


def normalize_readings(readings: Iterable[str]) -> Set[str]:
    # strip okurigana
    readings = {
        reading.split('.')[0] if '.' in reading else reading
        for reading in readings
    }
    # remove "-"
    readings = {reading.replace('-', '') for reading in readings}
    # convert to hiragana
    readings = {katakana_to_hiragana(reading) for reading in readings}
    # make ず and づ equivalent readings
    if readings & {'ず', 'づ'}:
        readings |= {'ず', 'づ'}
    return readings


def compound_readings(readings: Set[str]) -> Set[str]:
    gemination = {reading[:-1] + 'っ' for reading in readings}
    rendaku = {
        dakuten + reading[1:]
        for reading in readings
        for dakuten in dakutens.get(reading[0], set())
    }
    return gemination | rendaku


@lru_cache
def load_kanjidic(filename: str = default_kanjidic) -> Dict[str, Kanji]:
    with open(filename, mode='rb') as f:
        edict_data = f.read().decode('euc_jp')

    kanjidic = {}
    line_pattern = re.compile(
        r'(?m)^(.) (?:[0-9A-F]{4}) (?:(?:[A-Z]\S*) )*([^{]*?) (?:T[^{]*?)?((?:\{.*?\} )*\{.*?\})',
    )
    meaning_pattern = re.compile(r'{(.*?)}')
    for character, readings_str, meanings_str in line_pattern.findall(edict_data):
        # gather kanji information
        meanings = meaning_pattern.findall(meanings_str)
        readings = normalize_readings(readings_str.split())
        readings |= compound_readings(readings)
        kanji = Kanji(character, readings, meanings)

        # map character to kanji
        kanjidic[character] = kanji
    return kanjidic
