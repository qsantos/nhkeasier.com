use std::include_str;
use std::sync::LazyLock;
use std::time::Instant;

use gxhash::HashSet;
use ouroboros::self_referencing;
use regex::Regex;

use crate::{Deinflector, Edict, Error, iter_fragments};

static KATAKANA_RUN_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("[ァ-ヺー]+").expect("invalid KATAKANA_RUN_PATTERN regex"));

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

    pub fn from(&self, content: &str) -> Vec<String> {
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
                            matches.push((entry.line, key, entry.type_));
                        }
                    }
                }
            }

            let lines = format_lines(matches, is_lone_kana_of_kanji_word);
            tracing::debug!(
                "sub-edict with {} entries generated in {:?}",
                lines.len(),
                start.elapsed()
            );
            lines
        })
    }
}

impl SubEdictCreator {
    /// Whether the word is a key of EDICT (without deinflection)
    pub fn is_word(&self, word: &str) -> bool {
        self.with_edict2(|edict2| edict2.lookup(word).is_some())
    }
}

/// Whether a string from the text looks like a name, as opposed to a fragment of a word
///
/// Single characters and strings with hiragana mostly match given names by their readings (e.g.
/// のあ for 乃亜, 乃愛, etc.), and katakana names must not be part of a longer katakana word
/// (e.g. ツカ in カムチャツカ).
fn is_plausible_name(fragment: &str, katakana_runs: &HashSet<&str>) -> bool {
    let mut chars = fragment.chars();
    if chars.next().is_none() || chars.next().is_none() {
        return false;
    }
    if fragment
        .chars()
        .any(|c| matches!(c, '\u{3040}'..='\u{309f}'))
    {
        return false;
    }
    let is_katakana = fragment.chars().all(|c| matches!(c, 'ァ'..='ヺ' | 'ー'));
    !is_katakana || katakana_runs.contains(fragment)
}

/// Whether a single kana matched the reading of a word usually written in kanji; for example, し
/// for 氏 [し]; in the text, such a kana is almost always part of a longer word, or grammar
fn is_lone_kana_of_kanji_word(line: &str, key: &str) -> bool {
    let mut chars = key.chars();
    let (Some(c), None) = (chars.next(), chars.next()) else {
        return false;
    };
    let is_kana = matches!(c, '\u{3040}'..='\u{30ff}');
    is_kana && line.contains(" [") && !line.contains("(uk)")
}

/// Group the (line, key, type) matches by line, and format the lines for the client (see
/// format_line)
///
/// Lines whose matches are all unlikely are dropped.
fn format_lines<'a>(
    mut matches: Vec<(&'a str, &'a str, u32)>,
    is_unlikely: impl Fn(&str, &str) -> bool,
) -> Vec<String> {
    matches.sort_unstable();
    matches.dedup();
    let mut keys = Vec::new();
    matches
        .chunk_by(|(line1, _, _), (line2, _, _)| line1 == line2)
        .filter(|chunk| !chunk.iter().all(|(line, key, _)| is_unlikely(line, key)))
        .map(|chunk| {
            let (line, _, type_) = chunk[0];
            keys.clear();
            keys.extend(chunk.iter().map(|(_, key, _)| *key));
            format_line(line, &keys, type_)
        })
        .collect()
}

/// Format a line of EDICT for the client, as four fields separated by tabs: the writings, the
/// readings, the type (without bit 7, empty if zero), and the meanings, as displayed; for example:
/// 行く(P);往く(rK) [いく(P);ゆく(P)] /(v5k-s,vi) (1) to go/to move/(v5k-s,vi) (2) to leave/…
/// → 行く\tいく;ゆく\t66\tto go; to move/to leave/…
///
/// Only the headwords that the client can display are kept: when hovering a writing, the client
/// displays it with all the readings; when hovering a reading, it displays all the writings with
/// it; so other headwords are never displayed.
fn format_line(line: &str, keys: &[&str], type_: u32) -> String {
    let (headwords, glosses) = line
        .split_once(" /")
        .expect("Edict::parse should have checked for \" /\"");
    let mut ret = String::with_capacity(line.len());
    if let Some((writings, readings)) = headwords.split_once(" [") {
        let readings = readings.trim_end_matches(']');
        let writing_matched = iter_forms(writings).any(|form| keys.contains(&form));
        let reading_matched = iter_forms(readings).any(|form| keys.contains(&form));
        push_forms(&mut ret, writings, keys, reading_matched);
        ret.push('\t');
        push_forms(&mut ret, readings, keys, writing_matched);
    } else {
        // only readings
        ret.push('\t');
        push_forms(&mut ret, headwords, keys, false);
    }
    ret.push('\t');
    let type_ = type_ & !(1 << 7);
    if type_ != 0 {
        ret.push_str(&type_.to_string());
    }
    ret.push('\t');
    push_meanings(&mut ret, glosses.strip_suffix('/').unwrap_or(glosses));
    ret
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

/// Group glosses into meanings separated by '/', skipping markers and the tags of each gloss; for
/// example: (n) (1) dot/spot/(n) (2) mark/(P)/EntL1234X → dot; spot/mark
fn push_meanings(ret: &mut String, glosses: &str) {
    let mut meaning_is_empty = true;
    // NOTE: the client used to replace '/' with "; ", then split on "; "; this gives the same result
    for gloss in glosses.split('/').flat_map(split_semicolon_space) {
        if gloss == "(P)" || gloss.starts_with("EntL") {
            continue;
        }
        let (starts_meaning, gloss) = split_gloss_tags(gloss);
        if !meaning_is_empty {
            ret.push_str(if starts_meaning { "/" } else { "; " });
        }
        ret.push_str(gloss);
        meaning_is_empty = false;
    }
}

/// Same as s.split("; "), but faster for short strings
fn split_semicolon_space(s: &str) -> impl Iterator<Item = &str> {
    let mut rest = Some(s);
    std::iter::from_fn(move || {
        let s = rest?;
        let mut start = 0;
        while let Some(i) = s[start..].find(';') {
            let i = start + i;
            if s[i + 1..].starts_with(' ') {
                rest = Some(&s[i + 2..]);
                return Some(&s[..i]);
            }
            start = i + 1;
        }
        rest = None;
        Some(s)
    })
}

/// Split the tags at the start of a gloss: an optional part-of-speech (one or two characters), an
/// optional meaning number, and any other tags; return whether there was a meaning number (which
/// starts a new meaning) and the rest of the gloss
///
/// This is equivalent to /^(?:\([^0-9]\S?\) )?(?:\(([0-9]+)\) )?(?:\(.*?\) )*(.*)/, which the
/// client used, but much faster.
fn split_gloss_tags(gloss: &str) -> (bool, &str) {
    let mut rest = gloss;

    // optional part-of-speech
    if let Some(after) = rest.strip_prefix('(') {
        let mut chars = after.chars();
        if let Some(c1) = chars.next()
            && !c1.is_ascii_digit()
        {
            let after_c1 = chars.as_str();
            let mut chars = after_c1.chars();
            if let Some(c2) = chars.next()
                && !c2.is_whitespace()
                && let Some(after_tag) = chars.as_str().strip_prefix(") ")
            {
                rest = after_tag;
            } else if let Some(after_tag) = after_c1.strip_prefix(") ") {
                rest = after_tag;
            }
        }
    }

    // optional meaning number
    let mut has_number = false;
    if let Some(after) = rest.strip_prefix('(') {
        let digits = after.bytes().take_while(u8::is_ascii_digit).count();
        if digits != 0
            && let Some(after_tag) = after[digits..].strip_prefix(") ")
        {
            rest = after_tag;
            has_number = true;
        }
    }

    // other tags
    while rest.starts_with('(') {
        let Some(i) = rest[1..].find(") ") else {
            break;
        };
        rest = &rest[1 + i + ") ".len()..];
    }

    (has_number, rest)
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

    /// Names are only looked up for strings that are not also words in EDICT, since those are
    /// much more likely to be meant as words (e.g. 森 is “forest” more often than the surname),
    /// and that look like names (see is_plausible_name)
    pub fn from(&self, content: &str, edict: &SubEdictCreator) -> Vec<String> {
        self.with(|fields| {
            let start = Instant::now();
            let fragments: HashSet<&str> = iter_fragments(content).collect();
            let katakana_runs: HashSet<&str> = KATAKANA_RUN_PATTERN
                .find_iter(content)
                .map(|m| m.as_str())
                .collect();

            // (line, key) pairs, to know through which keys each line was matched
            let mut matches = Vec::new();
            for fragment in fragments {
                if !is_plausible_name(fragment, &katakana_runs) || edict.is_word(fragment) {
                    continue;
                }
                if let Some((key, entries)) = fields.enamdict.lookup(fragment as &str) {
                    for entry in entries.iter() {
                        matches.push((entry.line, key, 0));
                    }
                }
            }

            let lines = format_lines(matches, |_, _| false);
            tracing::debug!(
                "sub-enamdict with {} entries generated in {:?}",
                lines.len(),
                start.elapsed()
            );
            lines
        })
    }
}
