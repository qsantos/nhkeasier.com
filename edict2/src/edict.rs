use hashbrown::HashTable;

use crate::Error;

/// A line of the dictionary, as a range of the data, and the type of the word (see
/// type_from_glosses); this is more compact than a &str, to save memory
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct EdictEntry {
    offset: u32,
    len: u16,
    type_: u8,
}

/// A key of the dictionary, as a range of the data, and the range of its entries in Edict::entries;
/// this is more compact than a &str and a range, to save memory
#[derive(Clone, Copy, Debug)]
struct Key {
    offset: u32,
    len: u8,
    entries_start: u32,
    entries_len: u16,
}

#[derive(Clone, Debug)]
pub struct Edict<'a> {
    data: &'a str,
    // most keys have a single entry, so store all the entries in a single vector, rather than one
    // allocation per key, to save memory
    entries: Vec<EdictEntry>,
    keys: HashTable<Key>,
}

fn hash_key(key: &str) -> u64 {
    gxhash::gxhash64(key.as_bytes(), 0)
}

/// The offset of a slice of data in data
fn offset_in(data: &str, slice: &str) -> u32 {
    u32::try_from(slice.as_ptr() as usize - data.as_ptr() as usize)
        .expect("dictionary should be smaller than 4 GiB")
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
            let mut type_: u8 = 1 << 7;
            for sense in meanings.split('/') {
                if let Some(rest) = sense.strip_prefix('(') {
                    let (glosses, _) = rest.split_once(')').ok_or(Error::ParseError {
                        lineno,
                        expected: ")",
                    })?;
                    type_ |= type_from_glosses(glosses);
                }
            }

            let entry = EdictEntry {
                offset: offset_in(data, line),
                len: u16::try_from(line.len()).map_err(|_| Error::ParseError {
                    lineno,
                    expected: "line shorter than 64 KiB",
                })?,
                type_,
            };

            if let Some((writings, rest)) = line.split_once(" [") {
                // both writings, and readings (within brackets); for example:
                // 日本 [にほん(P);にっぽん] /(n) Japan/(P)/EntL1582710X/
                let (readings, _) = rest.split_once("] /").ok_or(Error::ParseError {
                    lineno,
                    expected: "] /",
                })?;
                insert_line_at_keys(&mut pairs, &keep_key, writings.split(';'), entry);
                insert_line_at_keys(&mut pairs, &keep_key, readings.split(';'), entry);
            } else {
                // only writings; for example:
                // あやかし /(n) (1) ghost that appears at sea during a shipwreck/(2) something strange or suspicious/(3) idiot/fool/(4) noh mask for roles involving dead or ghost characters/EntL2143630X/
                let (writings, _) = line.split_once(' ').ok_or(Error::ParseError {
                    lineno,
                    expected: " ",
                })?;
                insert_line_at_keys(&mut pairs, &keep_key, writings.split(';'), entry);
            };
        }
        // group the entries by key, keeping the order of the lines
        pairs.sort_by_key(|&(key, _)| key);
        let mut entries = Vec::with_capacity(pairs.len());
        let mut keys =
            HashTable::with_capacity(pairs.chunk_by(|(key1, _), (key2, _)| key1 == key2).count());
        let key_str = |key: &Key| {
            let offset = key.offset as usize;
            &data[offset..offset + key.len as usize]
        };
        for chunk in pairs.chunk_by(|(key1, _), (key2, _)| key1 == key2) {
            let key = chunk[0].0;
            let entries_start = entries.len() as u32;
            entries.extend(chunk.iter().map(|&(_, entry)| entry));
            let key = Key {
                offset: offset_in(data, key),
                len: u8::try_from(key.len()).expect("keys should be shorter than 256 bytes"),
                entries_start,
                entries_len: u16::try_from(chunk.len())
                    .expect("keys should have fewer than 65536 entries"),
            };
            keys.insert_unique(hash_key(key_str(&key)), key, |key| hash_key(key_str(key)));
        }
        Ok(Edict {
            data,
            entries,
            keys,
        })
    }

    /// Return the entries for the given word, along with the matching key in the dictionary
    pub fn lookup(
        &self,
        word: &str,
    ) -> Option<(&'a str, impl Iterator<Item = (&'a str, u32)> + '_)> {
        let data = self.data;
        let key = self.keys.find(hash_key(word), |key| {
            let offset = key.offset as usize;
            &data[offset..offset + key.len as usize] == word
        })?;
        let offset = key.offset as usize;
        let key_str = &data[offset..offset + key.len as usize];
        let start = key.entries_start as usize;
        let entries = self.entries[start..start + key.entries_len as usize]
            .iter()
            .map(move |entry| {
                let offset = entry.offset as usize;
                let line = &data[offset..offset + entry.len as usize];
                (line, u32::from(entry.type_))
            });
        Some((key_str, entries))
    }
}

fn type_from_glosses(glosses: &str) -> u8 {
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
    pairs: &mut Vec<(&'a str, EdictEntry)>,
    keep_key: &impl Fn(&str) -> bool,
    keys: I,
    entry: EdictEntry,
) {
    for key in keys {
        // strip common marker; for example “(P)”:
        // あの人(P);彼の人 [あのひと] /(pn) (1) (sometimes of one's spouse or partner) he/she/that person/(2) (arch) you/(P)/EntL1000440X/
        let key = key.split_once('(').map(|(key, _)| key).unwrap_or(key);
        if !keep_key(key) {
            continue;
        }
        pairs.push((key, entry));
    }
}
