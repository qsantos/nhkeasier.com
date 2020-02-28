import os.path
from collections import defaultdict
from typing import DefaultDict, Iterator, List, NamedTuple, Set

from .search import Word, search_edict

default_deinflect = os.path.join(os.path.dirname(__file__), 'deinflect.dat')


class Rule(NamedTuple):
    from_: str
    to: str
    type_: int
    reason: str


class Candidate(NamedTuple):
    word: str
    type_: int
    reasons: List[str]


# deinflect.dat countains instructions to remove inflections from words
# the first line is a header
# the next few lines (without '\t') are a string array refereced to later
# the rest are made of four fields separated by '\t'
#   * first field incdicates the suffix to look for in a candidate
#   * second field indicates when the suffix should be remplaced with
#   * third field helps narrowing down the grammatical class of the candidate
#   * fourth field points to the array string and gives a user-friendly
#     explanation of the removed suffix
# type is a bit field where:
#   * bit 0 hints at a 一段 verb ('v1' marker)
#   * bit 1 hints at a 五段 verb (markers starting with 'v5')
#   * bit 2 hints at a い-adjective (marker 'adj-i')
#   * bit 3 hints at a くる verb (marker 'vk')
#   * bit 4 hints at a す or する verb (markers starting with 'vs-')
#   * bit 7 should always be set for words (so that 0xff & wtype != 0 always)
# for a word, type gives a hint of the expected grammatical class of the word
# for a rule, type[0:8] gives the required grammatical class of original word
# for a rule, type[8:16] gives the grammatical class of the resulting word
# thus, the new word has type wtype = rtyle >> 8
class Deinflector:
    """A Deinflector instance applies deinflection rules to normalize a word"""
    def __init__(self, deinflect_data_filename: str = default_deinflect):
        """Populate deinflecting rules from given file"""
        with open(deinflect_data_filename, 'rb') as f:
            lines = iter(f)
            next(lines)  # skip header
            reasons = []  # collect the string array for later resolution
            self.rules: List[Rule] = []
            for byte_line in lines:
                line = byte_line.decode()
                fields = line.strip().split('\t')
                # the header does not indicate the size of the array string; it
                # is simplest to differentiate between the array string and the
                # actual rules by counting the numbers of fields
                if len(fields) == 1:
                    # string array
                    reasons.append(fields[0])
                else:
                    # rule
                    from_, to, stype_, reason = fields
                    type_ = int(stype_)
                    reason = reasons[int(reason)]  # resolve string
                    self.rules.append(Rule(from_, to, type_, reason))

    def __call__(self, word: str) -> List[Candidate]:
        """Iterate through possible deinflections of word (including word)

        Each value is a triplet whose first element is the deinflected word,
        the second element is a mask of possible grammatical classes for the
        word, and the third element is the corresponding reasonning for the
        inflection"""
        candidates = [Candidate(word, 0xff, [])]
        i = 0
        while i < len(candidates):
            candidate = candidates[i]
            for rule in self.rules:
                # check types match
                if candidate.type_ & rule.type_ == 0:
                    continue
                # check suffix matches
                if not candidate.word.endswith(rule.from_):
                    continue
                # append new candidate
                candidates.append(Candidate(
                    word=candidate.word[:-len(rule.from_)] + rule.to,  # replace suffix,
                    type_=rule.type_ >> 8,
                    reasons=candidate.reasons + [rule.reason],
                ))
                # NOTE: could check that new_word is already in candidates
                # Rikaikun merges with previous candidate; if this candidate
                # has already been processed, the new type is ignored
                # Rikaichamp only combines candidates of identical types
            i += 1
        return candidates

    def search_edict(self, fragment: str) -> Iterator[Word]:
        candidates = list(self(fragment))
        subedict: DefaultDict[str, Set[Word]] = defaultdict(set)
        for candidate, _, _ in candidates:
            for word in search_edict(candidate):
                for k in word.readings + word.writings:
                    subedict[k].add(word)
        for candidate, type_, reason in candidates:
            for word in subedict[candidate]:
                if word.get_type() & type_:
                    yield word
