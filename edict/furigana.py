from collections import deque
from typing import Deque, Iterator, List, Tuple

from .kanji import load_kanjidic

FuriganaMatch = List[Tuple[str, str]]


def furigana_from_kanji_kana(kanji: str, kana: str) -> str:
    matches = list(match_from_kanji_kana(kanji, kana))
    if len(matches) == 0:
        return f'{kanji}[{kana}]'
    return furigana_from_match(matches[0])


def match_from_kanji_kana(kanji: str, kana: str) -> Iterator[FuriganaMatch]:
    """Match kanji against kana

    Return a generators that yields all possible matches of kanji with the kana
    based on their known readings. For instance, for '牛肉' and 'ぎゅうにく',
    it yields the single match [('牛', 'ぎゅう'), ('肉', 'にく')].
    """
    kanjidic = load_kanjidic()

    q: Deque[Tuple[FuriganaMatch, str, str]] = deque([([], kanji, kana)])
    while q:
        match_prefix, kanji, kana = q.popleft()
        if not kanji and not kana:
            yield match_prefix
        if not kanji or not kana:
            continue
        c = kanji[0]

        if c == '々' and match_prefix:
            readings = {match_prefix[-1][1]}  # TODO: dakuten
        elif c in kanjidic:
            readings = kanjidic[c].readings
        else:
            readings = {c}

        for reading in readings:
            if kana.startswith(reading):
                new_prefixes = [*match_prefix, (c, reading)]
                new_kanji = kanji[1:]
                new_kana = kana[len(reading):]
                new_element = (new_prefixes, new_kanji, new_kana)
                q.append(new_element)


def furigana_from_match(match: FuriganaMatch) -> str:
    """Transform a kanji-kana match into Anki-compatible furigana

    For instance, for [('牛', 'ぎゅう'), ('肉', 'にく')], it returns
    '牛[ぎゅう]肉[にく]'.
    """
    def _() -> Iterator[str]:
        last_was_kana = False
        for kanji, kana in match:
            if kanji == kana:
                yield kana
            else:
                if last_was_kana:
                    yield ' '
                yield f'{kanji}[{kana}]'
            last_was_kana = kanji == kana
    return ''.join(_())
