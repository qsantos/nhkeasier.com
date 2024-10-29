use std::borrow::Cow;
use std::collections::HashMap;

use crate::Error;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct Rule<'a> {
    from: &'a str,
    to: &'a str,
    type_: u32,
    reason: &'a str,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Candidate<'a> {
    pub word: Cow<'a, str>,
    pub type_: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SuffixToRules<'a>(HashMap<char, (Vec<Rule<'a>>, SuffixToRules<'a>)>);

// deinflect.dat contains instructions to remove inflections from words
// the first line is a header
// the next few lines (without '\t') are a string array referenced to later
// the rest are made of four fields separated by '\t'
//   * first field indicates the suffix to look for in a candidate
//   * second field indicates when the suffix should be replaced with
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
pub struct Deinflector<'a> {
    suffix_to_rules: SuffixToRules<'a>,
}

impl<'a> Deinflector<'a> {
    pub fn parse(data: &'a str) -> Result<Self, Error> {
        fn aux<'a, I: Iterator<Item = char>>(
            chars: &mut I,
            rule: Rule<'a>,
            cur_suffix_to_rules: &mut SuffixToRules<'a>,
            cur_rules: Option<&mut Vec<Rule<'a>>>,
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
                cur_rules.expect("rules should not be None").push(rule);
            }
        }

        // NOTE: skip(1) for header on first line
        let mut suffix_to_rules = SuffixToRules(HashMap::new());
        let mut reasons = Vec::new();
        for (lineno, line) in data.lines().enumerate().skip(1) {
            let lineno = lineno + 1;
            let fields: Vec<&str> = line.split('\t').collect();
            match fields[..] {
                [_] => reasons.push(line),
                [from, to, type_, reason] => {
                    let type_: u32 = type_.parse().map_err(|_| Error::ParseInteger { lineno })?;
                    let reason: usize =
                        reason.parse().map_err(|_| Error::ParseInteger { lineno })?;
                    let reason = reasons[reason];
                    let rule = Rule {
                        from,
                        to,
                        type_,
                        reason,
                    };
                    aux(&mut from.chars().rev(), rule, &mut suffix_to_rules, None)
                }
                _ => panic!("unexpected line {line}"),
            }
        }
        Ok(Deinflector { suffix_to_rules })
    }

    pub fn deinflect<'b, 'c>(
        &self,
        word: &'c str,
        vec: &'b mut Vec<Candidate<'c>>,
    ) -> Iter<'_, 'b, 'c> {
        vec.clear();
        vec.push(Candidate {
            word: Cow::Borrowed(word),
            type_: 0xff,
        });
        Iter {
            deinflector: self,
            candidates: vec,
        }
    }
}

pub struct Iter<'a, 'b, 'c> {
    deinflector: &'a Deinflector<'a>,
    candidates: &'b mut Vec<Candidate<'c>>,
}

impl<'a, 'b, 'c: 'a> Iterator for Iter<'a, 'b, 'c> {
    type Item = Candidate<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        let candidate = self.candidates.pop()?;
        let mut cur_suffix_to_rules = &self.deinflector.suffix_to_rules;
        for c in candidate.word.chars().rev() {
            if let Some((rules, suffix_to_rules)) = cur_suffix_to_rules.0.get(&c) {
                cur_suffix_to_rules = suffix_to_rules;
                for rule in rules {
                    if candidate.type_ & rule.type_ == 0 {
                        continue;
                    }
                    let prefix_len = candidate.word.bytes().len() - rule.from.bytes().len();
                    let prefix = &candidate.word[..prefix_len];
                    let mut word = String::with_capacity(prefix_len + rule.to.as_bytes().len());
                    word.push_str(prefix);
                    word.push_str(rule.to);
                    self.candidates.push(Candidate {
                        word: Cow::Owned(word),
                        type_: rule.type_ >> 8,
                    })
                }
            } else {
                break;
            }
        }
        Some(candidate)
    }
}
