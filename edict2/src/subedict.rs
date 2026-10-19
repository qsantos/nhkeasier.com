use std::include_str;
use std::sync::LazyLock;
use std::time::Instant;

use edict2_index::{Entry, FLAG_KANJI_WORD, Index, could_be_name};
use gxhash::HashSet;
use regex::Regex;

use crate::{Deinflector, Error, iter_fragments};

static KATAKANA_RUN_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("[ァ-ヺー]+").expect("invalid KATAKANA_RUN_PATTERN regex"));

/// The index of a dictionary in data/, built by build.rs
macro_rules! index {
    ($name:literal) => {
        Index {
            records: include_str!(concat!(env!("OUT_DIR"), "/", $name, ".records")),
            entries: include_bytes!(concat!(env!("OUT_DIR"), "/", $name, ".entries")),
            slots: include_bytes!(concat!(env!("OUT_DIR"), "/", $name, ".slots")),
            tags: include_bytes!(concat!(env!("OUT_DIR"), "/", $name, ".tags")),
        }
    };
}

// NOTE: statics, rather than constants, so that the data is not duplicated where it is used
static EDICT2_INDEX: Index<'static> = index!("edict2");
static ENAMDICT_INDEX: Index<'static> = index!("enamdict");

pub struct SubEdictCreator {
    edict2: Index<'static>,
    deinflector: Deinflector<'static>,
}

/// Deinflection rules used by SubEdictCreator, to be served to the client so that both agree
pub const DEINFLECT_DATA: &str = include_str!("../data/deinflect");

impl SubEdictCreator {
    pub fn from_files() -> Result<Self, Error> {
        Ok(SubEdictCreator {
            edict2: EDICT2_INDEX,
            deinflector: Deinflector::parse(DEINFLECT_DATA)?,
        })
    }

    pub fn from(&self, content: &str) -> Vec<String> {
        let start = Instant::now();
        let fragments: HashSet<&str> = iter_fragments(content).collect();

        let mut candidates = Vec::new();
        let mut buffer = Vec::new();
        for fragment in fragments {
            candidates.extend(self.deinflector.deinflect(fragment, &mut buffer));
        }

        // (entry, key) pairs, to know through which keys each entry was matched
        let mut matches = Vec::new();
        for candidate in candidates {
            if let Some((key, entries)) = self.edict2.lookup(&candidate.word) {
                for entry in entries {
                    if u32::from(entry.type_) & candidate.type_ != 0 {
                        matches.push((entry, key));
                    }
                }
            }
        }

        let lines = format_entries(matches, is_lone_kana_of_kanji_word);
        tracing::debug!(
            "sub-edict with {} entries generated in {:?}",
            lines.len(),
            start.elapsed()
        );
        lines
    }

    /// Whether the word is a key of EDICT (without deinflection)
    pub fn is_word(&self, word: &str) -> bool {
        self.edict2.lookup(word).is_some()
    }
}

/// Whether a string from the text looks like a name, as opposed to a fragment of a word
///
/// Single characters and strings with hiragana mostly match given names by their readings (see
/// could_be_name), and katakana names must not be part of a longer katakana word (e.g. ツカ in
/// カムチャツカ).
fn is_plausible_name(fragment: &str, katakana_runs: &HashSet<&str>) -> bool {
    if !could_be_name(fragment) {
        return false;
    }
    let is_katakana = fragment.chars().all(|c| matches!(c, 'ァ'..='ヺ' | 'ー'));
    !is_katakana || katakana_runs.contains(fragment)
}

/// Whether a single kana matched the reading of a word usually written in kanji; for example, し
/// for 氏 [し]; in the text, such a kana is almost always part of a longer word, or grammar
fn is_lone_kana_of_kanji_word(entry: &Entry, key: &str) -> bool {
    let mut chars = key.chars();
    let (Some(c), None) = (chars.next(), chars.next()) else {
        return false;
    };
    let is_kana = matches!(c, '\u{3040}'..='\u{30ff}');
    is_kana && entry.flags & FLAG_KANJI_WORD != 0
}

/// Group the (entry, key) matches by entry, and format the entries for the client (see
/// format_entry)
///
/// Entries whose matches are all unlikely are dropped.
fn format_entries<'a>(
    mut matches: Vec<(Entry<'a>, &'a str)>,
    is_unlikely: impl Fn(&Entry, &str) -> bool,
) -> Vec<String> {
    matches.sort_unstable();
    matches.dedup();
    let mut keys = Vec::new();
    matches
        .chunk_by(|(entry1, _), (entry2, _)| entry1 == entry2)
        .filter(|chunk| !chunk.iter().all(|(entry, key)| is_unlikely(entry, key)))
        .map(|chunk| {
            keys.clear();
            keys.extend(chunk.iter().map(|(_, key)| *key));
            format_entry(chunk[0].0.record, &keys)
        })
        .collect()
}

/// Format an entry for the client, as four fields separated by tabs: the writings, the readings,
/// the type, and the meanings (see edict2_index::build_index); for example:
/// 行く;往く\tいく;ゆく\t66\tto go; to move/to leave → 行く\tいく;ゆく\t66\tto go; to move/to leave
///
/// Only the headwords that the client can display are kept: when hovering a writing, the client
/// displays it with all the readings; when hovering a reading, it displays all the writings with
/// it; so other headwords are never displayed.
fn format_entry(record: &str, keys: &[&str]) -> String {
    let mut fields = record.splitn(3, '\t');
    let (Some(writings), Some(readings), Some(rest)) =
        (fields.next(), fields.next(), fields.next())
    else {
        panic!("invalid record: {record:?}");
    };
    let mut ret = String::with_capacity(record.len());
    if writings.is_empty() {
        ret.push('\t');
        push_forms(&mut ret, readings, keys, false);
    } else {
        let writing_matched = writings.split(';').any(|form| keys.contains(&form));
        let reading_matched = readings.split(';').any(|form| keys.contains(&form));
        push_forms(&mut ret, writings, keys, reading_matched);
        ret.push('\t');
        push_forms(&mut ret, readings, keys, writing_matched);
    }
    ret.push('\t');
    ret.push_str(rest);
    ret
}

fn push_forms(ret: &mut String, forms: &str, keys: &[&str], keep_all: bool) {
    let forms = forms
        .split(';')
        .filter(|form| keep_all || keys.contains(form));
    for (i, form) in forms.enumerate() {
        if i != 0 {
            ret.push(';');
        }
        ret.push_str(form);
    }
}

pub struct SubEnamdictCreator {
    enamdict: Index<'static>,
}

impl SubEnamdictCreator {
    pub fn from_files() -> Result<Self, Error> {
        Ok(SubEnamdictCreator {
            enamdict: ENAMDICT_INDEX,
        })
    }

    /// Names are only looked up for strings that are not also words in EDICT, since those are
    /// much more likely to be meant as words (e.g. 森 is “forest” more often than the surname),
    /// and that look like names (see is_plausible_name)
    pub fn from(&self, content: &str, edict: &SubEdictCreator) -> Vec<String> {
        let start = Instant::now();
        let fragments: HashSet<&str> = iter_fragments(content).collect();
        let katakana_runs: HashSet<&str> = KATAKANA_RUN_PATTERN
            .find_iter(content)
            .map(|m| m.as_str())
            .collect();

        // (entry, key) pairs, to know through which keys each entry was matched
        let mut matches = Vec::new();
        for fragment in fragments {
            if !is_plausible_name(fragment, &katakana_runs) || edict.is_word(fragment) {
                continue;
            }
            if let Some((key, entries)) = self.enamdict.lookup(fragment) {
                matches.extend(entries.map(|entry| (entry, key)));
            }
        }

        let lines = format_entries(matches, |_, _| false);
        tracing::debug!(
            "sub-enamdict with {} entries generated in {:?}",
            lines.len(),
            start.elapsed()
        );
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_entry() {
        let record = "行く;往く\tいく;ゆく\t66\tto go; to move/to leave";
        // matched writing: all the readings
        assert_eq!(
            format_entry(record, &["行く"]),
            "行く\tいく;ゆく\t66\tto go; to move/to leave"
        );
        // matched reading: all the writings
        assert_eq!(
            format_entry(record, &["ゆく"]),
            "行く;往く\tゆく\t66\tto go; to move/to leave"
        );
        // only readings
        assert_eq!(
            format_entry("\tあの;あれ\t\tthat", &["あの"]),
            "\tあの\t\tthat"
        );
    }

    #[test]
    fn test_is_lone_kana_of_kanji_word() {
        let entry = |flags| Entry {
            record: "",
            type_: 1 << 7,
            flags,
        };
        assert!(is_lone_kana_of_kanji_word(&entry(FLAG_KANJI_WORD), "し"));
        assert!(!is_lone_kana_of_kanji_word(&entry(FLAG_KANJI_WORD), "氏"));
        assert!(!is_lone_kana_of_kanji_word(&entry(FLAG_KANJI_WORD), "しま"));
        assert!(!is_lone_kana_of_kanji_word(&entry(0), "し"));
    }

    #[test]
    fn test_is_plausible_name() {
        let katakana_runs = HashSet::from_iter(["カムチャツカ"]);
        assert!(is_plausible_name("カムチャツカ", &katakana_runs));
        assert!(!is_plausible_name("ツカ", &katakana_runs));
        assert!(is_plausible_name("穴水町", &katakana_runs));
        assert!(!is_plausible_name("森", &katakana_runs));
        assert!(!is_plausible_name("のあ", &katakana_runs));
    }
}
