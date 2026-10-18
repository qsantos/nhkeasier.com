use std::borrow::Cow;
use std::include_str;
use std::time::Instant;

use gxhash::HashSet;
use ouroboros::self_referencing;

use crate::{Deinflector, Edict, Error, iter_fragments};

#[self_referencing]
pub struct SubEdictCreator {
    edict2_data: &'static str,
    deinflector_data: &'static str,
    #[borrows(edict2_data)]
    #[not_covariant]
    edict2: Edict<'this>,
    #[borrows(deinflector_data)]
    #[not_covariant]
    deinflector: Deinflector<'this>,
}

/// Deinflection rules used by SubEdictCreator, to be served to the client so that both agree
pub const DEINFLECT_DATA: &str = include_str!("../data/deinflect");

impl SubEdictCreator {
    pub fn from_files() -> Result<Self, Error> {
        SubEdictCreator::try_new(
            include_str!("../data/edict2"),
            DEINFLECT_DATA,
            |data| Edict::parse(data),
            |data| Deinflector::parse(data),
        )
    }

    pub fn from(&self, content: &str) -> Vec<Cow<'_, str>> {
        self.with(|fields| {
            let start = Instant::now();
            let fragments: HashSet<&str> = iter_fragments(content).collect();

            let mut candidates = Vec::new();
            let mut buffer = Vec::new();
            for fragment in fragments {
                candidates.extend(fields.deinflector.deinflect(fragment, &mut buffer));
            }

            let mut lines = HashSet::default();
            for candidate in candidates {
                if let Some(entries) = fields.edict2.lookup(&candidate.word as &str) {
                    for entry in entries {
                        if entry.type_ & candidate.type_ != 0 {
                            lines.insert(entry.line);
                        }
                    }
                }
            }

            let lines: Vec<Cow<str>> = lines
                .into_iter()
                .map(|line| strip_headword_markers(strip_hidden_glosses(line)))
                .collect();
            tracing::debug!(
                "sub-edict with {} entries generated in {:?}",
                lines.len(),
                start.elapsed()
            );
            lines
        })
    }
}

/// Remove the trailing glosses that are never displayed; for example:
/// 日本 [にほん(P);にっぽん] /(n) Japan/(P)/EntL1582710X/ → 日本 [にほん(P);にっぽん] /(n) Japan/
fn strip_hidden_glosses(mut line: &str) -> &str {
    if let Some(i) = line.rfind("/EntL") {
        line = &line[..=i];
    }
    if line.ends_with("/(P)/") {
        line = &line[..line.len() - "(P)/".len()];
    }
    line
}

/// Remove the markers of headwords, which the client strips anyway; for example:
/// 行く(P);往く(rK) [いく(P);ゆく(P)] /… → 行く;往く [いく;ゆく] /…
fn strip_headword_markers(line: &str) -> Cow<'_, str> {
    let Some((headwords, glosses)) = line.split_once(" /") else {
        return Cow::Borrowed(line);
    };
    if !headwords.contains('(') {
        return Cow::Borrowed(line);
    }
    let mut ret = String::with_capacity(line.len());
    let mut in_marker = false;
    for c in headwords.chars() {
        match c {
            '(' => in_marker = true,
            ')' => in_marker = false,
            _ if !in_marker => ret.push(c),
            _ => {}
        }
    }
    ret.push_str(" /");
    ret.push_str(glosses);
    Cow::Owned(ret)
}

#[self_referencing]
pub struct SubEnamdictCreator {
    enamdict_data: &'static str,
    #[borrows(enamdict_data)]
    #[not_covariant]
    enamdict: Edict<'this>,
}

impl SubEnamdictCreator {
    pub fn from_files() -> Result<Self, Error> {
        SubEnamdictCreator::try_new(include_str!("../data/enamdict"), |data| Edict::parse(data))
    }

    pub fn from(&self, content: &str) -> Vec<&str> {
        self.with(|fields| {
            let start = Instant::now();
            let fragments: HashSet<&str> = iter_fragments(content).collect();

            let mut lines = HashSet::default();
            for fragment in fragments {
                if let Some(entries) = fields.enamdict.lookup(fragment as &str) {
                    for entry in entries.iter() {
                        lines.insert(entry.line);
                    }
                }
            }

            let lines: Vec<&str> = lines.into_iter().collect();
            tracing::debug!(
                "sub-enamdict with {} entries generated in {:?}",
                lines.len(),
                start.elapsed()
            );
            lines
        })
    }
}
