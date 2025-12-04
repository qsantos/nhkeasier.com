use std::collections::hash_map::Entry;

use gxhash::HashMap;

use crate::Error;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct EdictEntry<'a> {
    pub line: &'a str,
    pub type_: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Edict<'a> {
    entries: HashMap<&'a str, Box<[EdictEntry<'a>]>>,
}

impl<'a> Edict<'a> {
    pub fn parse(data: &'a str) -> Result<Self, Error> {
        let mut entries = HashMap::default();
        // NOTE: skip(1) for header on first line
        for (lineno, line) in data.lines().enumerate().skip(1) {
            let lineno = lineno + 1;
            let (_, meanings) = line.split_once(" /").ok_or(Error::ParseError {
                lineno,
                expected: " /",
            })?;
            let type_ = if let Some(rest) = meanings.strip_prefix('(') {
                let (glosses, _) = rest.split_once(')').ok_or(Error::ParseError {
                    lineno,
                    expected: ")",
                })?;
                type_from_glosses(glosses)
            } else {
                1 << 7
            };

            if let Some((writings, rest)) = line.split_once(" [") {
                // both writings, and readings (within brackets); for example:
                // 日本 [にほん(P);にっぽん] /(n) Japan/(P)/EntL1582710X/
                let (readings, _) = rest.split_once("] /").ok_or(Error::ParseError {
                    lineno,
                    expected: "] /",
                })?;
                insert_line_at_keys(&mut entries, writings.split(';'), line, type_);
                insert_line_at_keys(&mut entries, readings.split(';'), line, type_);
            } else {
                // only writings; for example:
                // あやかし /(n) (1) ghost that appears at sea during a shipwreck/(2) something strange or suspicious/(3) idiot/fool/(4) noh mask for roles involving dead or ghost characters/EntL2143630X/
                let (writings, _) = line.split_once(' ').ok_or(Error::ParseError {
                    lineno,
                    expected: " ",
                })?;
                insert_line_at_keys(&mut entries, writings.split(';'), line, type_);
            };
        }
        let entries = entries
            .into_iter()
            .map(|(key, entries)| (key, entries.into_boxed_slice()))
            .collect();
        Ok(Edict { entries })
    }

    pub fn lookup(&self, word: &str) -> Option<&[EdictEntry<'_>]> {
        self.entries.get(word).map(|b| b.as_ref())
    }
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
