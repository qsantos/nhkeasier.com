# encoding: utf-8
import os.path
from collections import defaultdict

from .search import search_edict

default_deinflect = os.path.join(os.path.dirname(__file__), 'deinflect.dat')


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
    def __init__(self, deinflect_data_filename=default_deinflect):
        """Populate deinflecting rules from given file"""
        with open(deinflect_data_filename, 'rb') as f:
            lines = iter(f)
            next(lines)  # skip header
            reasons = []  # collect the string array for later resolution
            self.rules = []
            for line in lines:
                line = line.decode('utf-8')
                fields = line.strip().split(u'\t')
                # the header does not indicate the size of the array string; it
                # is simplest to differentiate between the array string and the
                # actual rules by counting the numbers of fields
                if len(fields) == 1:
                    # string array
                    reasons.append(fields[0])
                else:
                    # rule
                    from_, to, type_, reason = fields
                    type_ = int(type_)
                    reason = reasons[int(reason)]  # resolve string
                    self.rules.append((from_, to, type_, reason))

    def __call__(self, word):
        """Iterate through possible deinflections of word (including word)

        Each value is a triplet whose first element is the deinflected word,
        the second element is a mask of possible grammatical classes for the
        word, and the third element is the corresponding reasonning for the
        inflection"""
        candidates = [(word, 0xff, [])]
        i = 0
        while i < len(candidates):
            word, wtype, wreason = candidates[i]
            for rfrom, rto, rtype, rreason in self.rules:
                # check types match
                if wtype & rtype == 0:
                    continue
                # check suffix matches
                if not word.endswith(rfrom):
                    continue
                # append new candidate
                new_word = word[:-len(rfrom)] + rto  # replace suffix
                new_type = rtype >> 8
                new_reason = wreason + [rreason]
                candidates.append((new_word, new_type, new_reason))
                # NOTE: could check that new_word is already in candidates
                # Rikaikun merges with previous candidate; if this candidate
                # has already been processed, the new type is ignored
                # Rikaichamp only combines candidates of identical types
            i += 1
        return candidates

    def search_edict(self, fragment):
        candidates = list(self(fragment))
        subedict = defaultdict(set)
        for candidate, _, _ in candidates:
            for word in search_edict(candidate):
                for k in word.readings + word.writings:
                    subedict[k].add(word)
        for candidate, type_, reason in candidates:
            for word in subedict[candidate]:
                if word.get_type() & type_:
                    yield word
