use gxhash::HashMap;

use crate::Error;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct EdictEntry<'a> {
    pub line: &'a str,
    pub type_: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Edict<'a> {
    // most keys have a single entry, so store all the entries in a single vector, rather than one
    // allocation per key, to save memory
    entries: Vec<EdictEntry<'a>>,
    // for each key, the range of its entries in entries, as (start, len)
    keys: HashMap<&'a str, (u32, u32)>,
}

impl<'a> Edict<'a> {
    pub fn parse(data: &'a str) -> Result<Self, Error> {
        Self::parse_with_key_filter(data, |_| true)
    }

    /// Only index the entries at the keys for which keep_key returns true
    pub fn parse_with_key_filter(
        data: &'a str,
        keep_key: impl Fn(&str) -> bool,
    ) -> Result<Self, Error> {
        // (key, entry) pairs; using a single vector rather than a vector per key avoids many small
        // allocations, whose memory would not be returned to the system after parsing
        let mut pairs = Vec::new();
        // NOTE: skip(1) for header on first line
        for (lineno, line) in data.lines().enumerate().skip(1) {
            let lineno = lineno + 1;
            let (_, meanings) = line.split_once(" /").ok_or(Error::ParseError {
                lineno,
                expected: " /",
            })?;
            // each sense may start with its own part-of-speech tags; for example:
            // たい /(aux-adj) (1) want to ... do something/…/(suf,adj-i) (3) very .../(P)/EntL2017560X/
            let mut type_ = 1 << 7;
            for sense in meanings.split('/') {
                if let Some(rest) = sense.strip_prefix('(') {
                    let (glosses, _) = rest.split_once(')').ok_or(Error::ParseError {
                        lineno,
                        expected: ")",
                    })?;
                    type_ |= type_from_glosses(glosses);
                }
            }

            if let Some((writings, rest)) = line.split_once(" [") {
                // both writings, and readings (within brackets); for example:
                // 日本 [にほん(P);にっぽん] /(n) Japan/(P)/EntL1582710X/
                let (readings, _) = rest.split_once("] /").ok_or(Error::ParseError {
                    lineno,
                    expected: "] /",
                })?;
                insert_line_at_keys(&mut pairs, &keep_key, writings.split(';'), line, type_);
                insert_line_at_keys(&mut pairs, &keep_key, readings.split(';'), line, type_);
            } else {
                // only writings; for example:
                // あやかし /(n) (1) ghost that appears at sea during a shipwreck/(2) something strange or suspicious/(3) idiot/fool/(4) noh mask for roles involving dead or ghost characters/EntL2143630X/
                let (writings, _) = line.split_once(' ').ok_or(Error::ParseError {
                    lineno,
                    expected: " ",
                })?;
                insert_line_at_keys(&mut pairs, &keep_key, writings.split(';'), line, type_);
            };
        }
        // group the entries by key, keeping the order of the lines
        pairs.sort_by_key(|&(key, _)| key);
        let mut entries = Vec::with_capacity(pairs.len());
        let mut keys = HashMap::default();
        keys.reserve(pairs.chunk_by(|(key1, _), (key2, _)| key1 == key2).count());
        for chunk in pairs.chunk_by(|(key1, _), (key2, _)| key1 == key2) {
            let start = entries.len() as u32;
            entries.extend(chunk.iter().map(|(_, entry)| entry.clone()));
            keys.insert(chunk[0].0, (start, chunk.len() as u32));
        }
        Ok(Edict { entries, keys })
    }

    /// Return the entries for the given word, along with the matching key in the dictionary
    pub fn lookup(&self, word: &str) -> Option<(&'a str, &[EdictEntry<'a>])> {
        self.keys.get_key_value(word).map(|(key, &(start, len))| {
            let start = start as usize;
            (*key, &self.entries[start..start + len as usize])
        })
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
            // the deinflection rules for 行く (った→く, etc.) only target this type
            if gloss == "v5k-s" {
                type_ |= 1 << 6;
            }
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
    pairs: &mut Vec<(&'a str, EdictEntry<'a>)>,
    keep_key: &impl Fn(&str) -> bool,
    keys: I,
    line: &'a str,
    type_: u32,
) {
    for key in keys {
        // strip common marker; for example “(P)”:
        // あの人(P);彼の人 [あのひと] /(pn) (1) (sometimes of one's spouse or partner) he/she/that person/(2) (arch) you/(P)/EntL1000440X/
        let key = key.split_once('(').map(|(key, _)| key).unwrap_or(key);
        if !keep_key(key) {
            continue;
        }
        pairs.push((key, EdictEntry { line, type_ }));
    }
}
