import re
from typing import Iterator, Set

from .deinflect import Deinflector

ranges = [
    '々',  # IDEOGRAPHIC ITERATION MARK (U+3005)
    '\u3040-\u30ff',  # Hiragana, Katakana
    '\u3400-\u4dbf',  # CJK Unified Ideographs Extension A
    '\u4e00-\u9fff',  # CJK Unified Ideographs
    '\uf900-\ufaff',  # CJK Compatibility Ideographs
    '\uff66-\uff9f',  # Halfwidth and Fullwidth Forms Block (hiragana and katakana)
]
fragment_pattern = re.compile('[' + ''.join(ranges) + ']+')


def japanese_text_substrings(text: str) -> Iterator[str]:
    for match in fragment_pattern.finditer(text):
        fragment = match.group()
        for start in range(len(fragment)):
            for stop in reversed(range(start + 1, len(fragment) + 1)):
                yield fragment[start:stop]


def create_subedict(text: str) -> Set[bytes]:
    """List EDICT items that might be present in text"""
    from .search import edict
    deinflector = Deinflector()
    candidates = {
        (candidate, type_)
        for substring in japanese_text_substrings(text)
        for candidate, type_ in deinflector(substring)
    }
    return {
        entry
        for (candidate, type_) in candidates
        for (entry, entry_type_) in edict.search(candidate)
        if entry_type_ & type_
    }


def create_subenamdict(text: str) -> Set[bytes]:
    """List EDICT items that might be present in text"""
    from .search import enamdict
    return {
        entry
        for substring in japanese_text_substrings(text)
        for (entry, _) in enamdict.search(substring)
    }


def export_subedict(subedict: Set[bytes], sort: bool = False) -> str:
    lines = [line.decode() for line in subedict]
    if sort:
        lines.sort()
    return '\n'.join(lines) + '\n'


def save_subedict(subedict: Set[bytes], filename: str) -> None:
    with open(filename, 'w') as f:
        f.write(export_subedict(subedict, sort=True))


def main() -> None:
    import argparse

    # parse arguments
    parser = argparse.ArgumentParser()
    parser.description = 'Reduce EDICT and ENAMDICT to cover given text'
    parser.add_argument('text_file')
    parser.add_argument('subedict_file')
    parser.add_argument('subenamdict_file')
    args = parser.parse_args()

    # read text file
    with open(args.text_file, 'rb') as f:
        text = f.read().decode('utf-8')

    # actually do the work
    subedict = create_subedict(text)
    subenamdict = create_subenamdict(text)
    save_subedict(subedict, args.subedict_file)
    save_subedict(subenamdict, args.subenamdict_file)


if __name__ == '__main__':
    main()