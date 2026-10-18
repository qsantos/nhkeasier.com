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

            // (line, key) pairs, to know through which keys each line was matched
            let mut matches = Vec::new();
            for candidate in candidates {
                if let Some((key, entries)) = fields.edict2.lookup(&candidate.word as &str) {
                    for entry in entries {
                        if entry.type_ & candidate.type_ != 0 {
                            matches.push((entry.line, key));
                        }
                    }
                }
            }

            let lines = keep_displayed_lines(matches, strip_hidden_glosses);
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

/// Group the (line, key) matches by line, and only keep the headwords that can be displayed
fn keep_displayed_lines<'a>(
    mut matches: Vec<(&'a str, &'a str)>,
    transform: impl Fn(&'a str) -> &'a str,
) -> Vec<Cow<'a, str>> {
    matches.sort_unstable();
    matches.dedup();
    let mut keys = Vec::new();
    matches
        .chunk_by(|(line1, _), (line2, _)| line1 == line2)
        .map(|chunk| {
            keys.clear();
            keys.extend(chunk.iter().map(|(_, key)| *key));
            keep_displayed_headwords(transform(chunk[0].0), &keys)
        })
        .collect()
}

/// Keep only the headwords that the client can display, without their markers
///
/// When hovering a writing, the client displays it with all the readings; when hovering a reading,
/// it displays all the writings with it; so other headwords are never displayed. For example, when
/// the text only contains 行く:
/// 行く(P);往く(rK) [いく(P);ゆく(P)] /… → 行く [いく;ゆく] /…
fn keep_displayed_headwords<'a>(line: &'a str, keys: &[&str]) -> Cow<'a, str> {
    let (headwords, glosses) = line
        .split_once(" /")
        .expect("Edict::parse should have checked for \" /\"");
    if !headwords.contains([';', '(']) {
        // single writing (and single reading), without markers: nothing to remove
        return Cow::Borrowed(line);
    }
    let mut ret = String::with_capacity(line.len());
    if let Some((writings, readings)) = headwords.split_once(" [") {
        let readings = readings.trim_end_matches(']');
        let writing_matched = iter_forms(writings).any(|form| keys.contains(&form));
        let reading_matched = iter_forms(readings).any(|form| keys.contains(&form));
        push_forms(&mut ret, writings, keys, reading_matched);
        ret.push_str(" [");
        push_forms(&mut ret, readings, keys, writing_matched);
        ret.push(']');
    } else {
        push_forms(&mut ret, headwords, keys, false);
    }
    ret.push_str(" /");
    ret.push_str(glosses);
    Cow::Owned(ret)
}

/// Iterate over the forms of a list of headwords, without their markers (like Edict::parse)
fn iter_forms(headwords: &str) -> impl Iterator<Item = &str> {
    headwords
        .split(';')
        .map(|form| form.split_once('(').map_or(form, |(form, _)| form))
}

fn push_forms(ret: &mut String, headwords: &str, keys: &[&str], keep_all: bool) {
    let forms = iter_forms(headwords).filter(|form| keep_all || keys.contains(form));
    for (i, form) in forms.enumerate() {
        if i != 0 {
            ret.push(';');
        }
        ret.push_str(form);
    }
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

    pub fn from(&self, content: &str) -> Vec<Cow<'_, str>> {
        self.with(|fields| {
            let start = Instant::now();
            let fragments: HashSet<&str> = iter_fragments(content).collect();

            // (line, key) pairs, to know through which keys each line was matched
            let mut matches = Vec::new();
            for fragment in fragments {
                if let Some((key, entries)) = fields.enamdict.lookup(fragment as &str) {
                    for entry in entries.iter() {
                        matches.push((entry.line, key));
                    }
                }
            }

            let lines = keep_displayed_lines(matches, |line| line);
            tracing::debug!(
                "sub-enamdict with {} entries generated in {:?}",
                lines.len(),
                start.elapsed()
            );
            lines
        })
    }
}
