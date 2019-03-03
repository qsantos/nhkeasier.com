# encoding: utf-8
from collections import deque

from .kanji import load_kanjidic

kanjidic = None


def furigana_from_kanji_kana(kanji, kana, make_default=True):
    matches = list(match_from_kanji_kana(kanji, kana))
    if len(matches) == 0:
        return u'{}[{}]'.format(kanji, kana) if make_default else None
    return furigana_from_match(matches[0])


def match_from_kanji_kana(kanji, kana):
    """Match kanji against kana

    Return a generators that yields all possible matches of kanji with the kana
    based on their known readings. For instance, for u'牛肉' and u'ぎゅうにく',
    it yields the single match [(u'牛', u'ぎゅう'), (u'肉', u'にく')].
    """
    global kanjidic
    if kanjidic is None:
        kanjidic = load_kanjidic()

    q = deque([([], kanji, kana)])
    while q:
        match_prefix, kanji, kana = q.popleft()
        if not kanji and not kana:
            yield match_prefix
        if not kanji or not kana:
            continue
        c = kanji[0]
        if c == u'々' and match_prefix:
            readings = [match_prefix[-1][1]]  # TODO: dakuten
        else:
            try:
                kanjiinfo = kanjidic[c]
            except KeyError:
                readings = c
            else:
                readings = kanjiinfo.readings
        for reading in readings:
            if kana.startswith(reading):
                new_prefixes = match_prefix + [(c, reading)]
                new_kanji = kanji[1:]
                new_kana = kana[len(reading):]
                new_element = (new_prefixes, new_kanji, new_kana)
                q.append(new_element)


def furigana_from_match(match):
    """Transform a kanji-kana match into Anki-compatible furigana

    For instance, for [(u'牛', u'ぎゅう'), (u'肉', u'にく')], it returns
    u'牛[ぎゅう]肉[にく]'.
    """
    def _():
        last_was_kana = False
        for kanji, kana in match:
            if kanji == kana:
                yield kana
            else:
                if last_was_kana:
                    yield u' '
                yield u'{}[{}]'.format(kanji, kana)
            last_was_kana = kanji == kana
    return u''.join(_())
