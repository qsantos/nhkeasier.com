use std::collections::hash_map::Entry;
use std::collections::HashMap;

use regex::Regex;

use edict2::Deinflector;

struct EdictEntry<'a> {
    line: &'a str,
    type_: u32,
}

fn type_from_glosses(glosses: &str) -> u32 {
    // the false positive are few enough that we just look in the line, without word boundaries
    let mut type_ = 1 << 7;
    for gloss in glosses.split(',') {
        if gloss == "v1" {
            type_ |= 1 << 0;
        } else if gloss.starts_with("v5") {
            type_ |= 1 << 1;
        } else if gloss == "adj-i" {
            type_ |= 1 << 2;
        } else if gloss == "vk" {
            type_ |= 1 << 3;
        } else if gloss == "vs" || gloss.starts_with("vs-") {
            type_ |= 1 << 4;
        }
    }
    type_
}

#[inline]
fn insert_line_at_keys<'a, I: Iterator<Item = &'a str>>(
    entries: &mut HashMap<&'a str, Vec<EdictEntry<'a>>>,
    keys: I,
    line: &'a str,
    type_: u32,
) {
    for key in keys {
        // strip common marker; for example “(P)”:
        // あの人(P);彼の人 [あのひと] /(pn) (1) (sometimes of one's spouse or partner) he/she/that person/(2) (arch) you/(P)/EntL1000440X/
        let key = key.split_once('(').map(|(key, _)| key).unwrap_or(key);
        let entry = EdictEntry { line, type_ };
        match entries.entry(key) {
            Entry::Occupied(mut e) => e.get_mut().push(entry),
            Entry::Vacant(e) => drop(e.insert(vec![entry])),
        }
    }
}

fn parse_edict(data: &str) -> HashMap<&str, Vec<EdictEntry<'_>>> {
    let mut entries = HashMap::new();
    // NOTE: skip(1) for header on first line
    for line in data.lines().skip(1) {
        let (_, meanings) = line.split_once(" /").unwrap();
        let type_ = if let Some(rest) = meanings.strip_prefix('(') {
            let (glosses, _) = rest.split_once(')').unwrap();
            type_from_glosses(glosses)
        } else {
            1 << 7
        };

        if let Some((writings, rest)) = line.split_once(" [") {
            // both writings, and readings (within brackets); for example:
            // 日本 [にほん(P);にっぽん] /(n) Japan/(P)/EntL1582710X/
            let (readings, _) = rest.split_once("] /").unwrap();
            insert_line_at_keys(&mut entries, writings.split(';'), line, type_);
            insert_line_at_keys(&mut entries, readings.split(';'), line, type_);
        } else {
            // only writings; for example:
            // あやかし /(n) (1) ghost that appears at sea during a shipwreck/(2) something strange or suspicious/(3) idiot/fool/(4) noh mask for roles involving dead or ghost characters/EntL2143630X/
            let (writings, _) = line.split_once(' ').unwrap();
            insert_line_at_keys(&mut entries, writings.split(';'), line, type_);
        };
    }
    entries
}

fn main() {
    let data = std::fs::read_to_string("edict2").unwrap();
    let edict2 = parse_edict(&data);

    let data = std::fs::read_to_string("deinflect.dat").unwrap();
    let deinflector = Deinflector::parse(&data);

    // iter fragments
    let data = std::fs::read_to_string("test-input").unwrap();
    let fragment_pattern = Regex::new(concat!(
        "[",
        "々",                // IDEOGRAPHIC ITERATION MARK (U+3005)
        "\u{3040}-\u{30ff}", // Hiragana, Katakana
        "\u{3400}-\u{4dbf}", // CJK Unified Ideographs Extension A
        "\u{4e00}-\u{9fff}", // CJK Unified Ideographs
        "\u{f900}-\u{faff}", // CJK Compatibility Ideographs
        "\u{ff66}-\u{ff9f}", // Halfwidth and Fullwidth Forms Block (hiragana and katakana)
        "]+",
    ))
    .unwrap();
    for m in fragment_pattern.find_iter(&data) {
        let fragment = m.as_str();
        for (start, _) in fragment.char_indices() {
            let suffix = &fragment[start..];
            for (end, c) in suffix.char_indices() {
                let word = &suffix[..end + c.len_utf8()];
                for candidate in deinflector.deinflect(word) {
                    // iter search
                    if let Some(entries) = edict2.get(&candidate.word as &str) {
                        for entry in entries {
                            if entry.type_ & candidate.type_ != 0 {
                                println!("{}", entry.line);
                            }
                        }
                    }
                    // end search
                }
            }
        }
    }
}
