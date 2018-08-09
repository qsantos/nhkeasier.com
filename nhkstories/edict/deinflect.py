#!/usr/bin/env python

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
#   * bit  8 hints at a 一段 verb ('v1' marker)
#   * bit  9 hints at a 五段 verb (markers starting with 'v5')
#   * bit 10 hints at a い-adjective (marker 'adj-i')
#   * bit 11 hints at a くる verb (marker 'vk')
#   * bit 12 hints at a す or する verb (markers starting with 'vs-')
class Deinflector:
    """A Deinflector instance applies deinflection rules to normalize a word"""
    def __init__(self, deinflect_data_filename):
        """Populate deinflecting rules from given file"""
        with open(deinflect_data_filename) as f:
            lines = iter(f)
            next(lines)  # skip header
            reasons = []  # collect the string array for later resolution
            self.rules = []
            for line in lines:
                fields = line.strip().split('\t')
                # the header does not indicate the size of the array string; it
                # is simplest to differentiate between the array string and the
                # actual rules by counting the numbers of fields
                if len(fields) == 1:
                    # string array
                    reasons.append(fields[0])
                else:
                    # rule
                    from_, to, type, reason = fields
                    type = int(type)
                    reason = reasons[int(reason)]  # resolve string
                    self.rules.append((from_, to, type, reason))

    def __call__(self, word):
        """Iterate through possible deinflections of word (including word)

        Each value is a triplet whose first element is the deinflected word,
        the second element is a mask of possible grammatical classes for the
        word, and the third element is the corresponding reasonning for the
        inflection"""
        yield word, 0, None
        for from_, to, type, reason in self.rules:
            if word.endswith(from_):
                candidate = word[:-len(from_)] + to  # replace suffix
                yield candidate, type, reason
