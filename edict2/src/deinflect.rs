use std::collections::hash_map::Entry;
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
pub struct Deinflector<'a> {
    rules: HashMap<&'a str, Vec<Rule>>,
}

impl<'a> Deinflector<'a> {
    pub fn parse(data: &'a str) -> Self {
        // NOTE: skip(1) for header on first line
        let mut rules: HashMap<&str, Vec<Rule>> = HashMap::new();
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
                    match rules.entry(from) {
                        Entry::Occupied(mut e) => e.get_mut().push(rule),
                        Entry::Vacant(e) => drop(e.insert(vec![rule])),
                    }
                }
                _ => panic!("unexpected line {line}"),
            }
        }
        Deinflector { rules }
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
    deinflector: &'a Deinflector<'a>,
    candidates: Vec<Candidate>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = Candidate;
    fn next(&mut self) -> Option<Self::Item> {
        // iter deinflections
        if let Some(candidate) = self.candidates.pop() {
            for (start, _) in candidate.word.char_indices().rev().take(9) {
                let suffix = &candidate.word[start..];
                if let Some(rules) = self.deinflector.rules.get(suffix) {
                    for rule in rules {
                        if candidate.type_ & rule.type_ == 0 {
                            continue;
                        }
                        self.candidates.push(Candidate {
                            word: format!("{}{}", &candidate.word[..start], rule.to),
                            type_: rule.type_ >> 8,
                        })
                    }
                }
            }
            Some(candidate)
        } else {
            None
        }
    }
}
