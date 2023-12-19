use std::collections::HashMap;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct Rule {
    from: String,
    to: String,
    type_: u32,
    reason: String,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Candidate {
    pub word: String,
    pub type_: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SuffixToRules(HashMap<char, (Vec<Rule>, SuffixToRules)>);

// deinflect.dat countains instructions to remove inflections from words
// the first line is a header
// the next few lines (without '\t') are a string array refereced to later
// the rest are made of four fields separated by '\t'
//   * first field incdicates the suffix to look for in a candidate
//   * second field indicates when the suffix should be remplaced with
//   * third field helps narrowing down the grammatical class of the candidate
//   * fourth field points to the array string and gives a user-friendly
//     explanation of the removed suffix
// type is a bit field where:
//   * bit 0 hints at a 一段 verb ('v1' marker)
//   * bit 1 hints at a 五段 verb (markers starting with 'v5')
//   * bit 2 hints at a い-adjective (marker 'adj-i')
//   * bit 3 hints at a くる verb (marker 'vk')
//   * bit 4 hints at a す or する verb (markers starting with 'vs-')
//   * bit 7 should always be set for words (so that 0xff & wtype != 0 always)
// for a word, type gives a hint of the expected grammatical class of the word
// for a rule, type[0:8] gives the required grammatical class of original word
// for a rule, type[8:16] gives the grammatical class of the resulting word
// thus, the new word has type wtype = rtyle >> 8
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Deinflector {
    suffix_to_rules: SuffixToRules,
}

impl Deinflector {
    pub fn parse(data: &str) -> Self {
        fn aux<I: Iterator<Item = char>>(
            chars: &mut I,
            rule: Rule,
            cur_suffix_to_rules: &mut SuffixToRules,
            cur_rules: Option<&mut Vec<Rule>>,
        ) {
            if let Some(c) = chars.next() {
                if let Some((rules, suffix_to_rules)) = cur_suffix_to_rules.0.get_mut(&c) {
                    aux(chars, rule, suffix_to_rules, Some(rules));
                } else {
                    let mut rules = Vec::new();
                    let mut suffix_to_rules = SuffixToRules(HashMap::new());
                    aux(chars, rule, &mut suffix_to_rules, Some(&mut rules));
                    cur_suffix_to_rules.0.insert(c, (rules, suffix_to_rules));
                }
            } else {
                cur_rules.unwrap().push(rule);
            }
        }

        // NOTE: skip(1) for header on first line
        let mut suffix_to_rules = SuffixToRules(HashMap::new());
        let mut reasons = Vec::new();
        for line in data.lines().skip(1) {
            let fields: Vec<&str> = line.split('\t').collect();
            match fields[..] {
                [_] => reasons.push(line),
                [from, to, type_, reason] => {
                    let type_: u32 = type_.parse().unwrap();
                    let reason: usize = reason.parse().unwrap();
                    let reason = reasons[reason];
                    let rule = Rule {
                        from: from.to_string(),
                        to: to.to_string(),
                        type_,
                        reason: reason.to_string(),
                    };
                    aux(&mut from.chars().rev(), rule, &mut suffix_to_rules, None)
                }
                _ => panic!("unexpected line {line}"),
            }
        }
        Deinflector { suffix_to_rules }
    }

    pub fn deinflect(&self, word: &str) -> Iter<'_> {
        Iter {
            deinflector: self,
            candidates: vec![Candidate {
                word: word.to_string(),
                type_: 0xff,
            }],
        }
    }
}

pub struct Iter<'a> {
    deinflector: &'a Deinflector,
    candidates: Vec<Candidate>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = Candidate;
    fn next(&mut self) -> Option<Self::Item> {
        // iter deinflections
        if let Some(candidate) = self.candidates.pop() {
            let mut cur_suffix_to_rules = &self.deinflector.suffix_to_rules;
            for c in candidate.word.chars().rev() {
                if let Some((rules, suffix_to_rules)) = cur_suffix_to_rules.0.get(&c) {
                    cur_suffix_to_rules = suffix_to_rules;
                    for rule in rules {
                        if candidate.type_ & rule.type_ == 0 {
                            continue;
                        }
                        let prefix = candidate.word.strip_suffix(&rule.from).unwrap();
                        self.candidates.push(Candidate {
                            word: format!("{}{}", prefix, rule.to),
                            type_: rule.type_ >> 8,
                        })
                    }
                } else {
                    break;
                }
            }
            Some(candidate)
        } else {
            None
        }
    }
}
