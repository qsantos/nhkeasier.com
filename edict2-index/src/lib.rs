//! Index of EDICT-formatted dictionaries, built at compile time (see edict2/build.rs), so that the
//! original dictionaries do not need to be parsed, or even kept in memory, at runtime
//!
//! An index is made of three parts:
//! - records: one line per entry, already formatted for the client (see format_record);
//! - entries: for each entry, the range of its record, its type and flags (ENTRY_SIZE bytes);
//! - slots: an open-addressing hash table mapping each key (a form of a headword, stored as a range
//!   of records) to the range of its entries (SLOT_SIZE bytes per slot);
//! - tags: for each slot, 0 if it is empty, or 7 bits of the hash of its key otherwise; like in
//!   SwissTable, probing these first avoids reading the larger slots in most cases.

use std::fmt::Write;

/// Flag of an entry: the word has writings (usually in kanji) and is not usually written in kana
pub const FLAG_KANJI_WORD: u8 = 1 << 0;

const ENTRY_SIZE: usize = 8;
const SLOT_SIZE: usize = 11;
/// Maximum ratio of used slots; lower is faster for missing keys, but takes more memory
const MAX_LOAD_FACTOR: f64 = 0.7;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct ParseError {
    pub lineno: usize,
    pub expected: &'static str,
}

/// An index, as built by build_index
#[derive(Clone, Debug, Default)]
pub struct IndexData {
    pub records: String,
    pub entries: Vec<u8>,
    pub slots: Vec<u8>,
    pub tags: Vec<u8>,
}

impl IndexData {
    pub fn as_index(&self) -> Index<'_> {
        Index {
            records: &self.records,
            entries: &self.entries,
            slots: &self.slots,
            tags: &self.tags,
        }
    }
}

/// A view of an index, as built by build_index
#[derive(Clone, Copy, Debug)]
pub struct Index<'a> {
    pub records: &'a str,
    pub entries: &'a [u8],
    pub slots: &'a [u8],
    pub tags: &'a [u8],
}

/// An entry of the index
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Entry<'a> {
    /// writings, readings, type and meanings, separated by tabs (see format_record)
    pub record: &'a str,
    pub type_: u8,
    pub flags: u8,
}

impl<'a> Index<'a> {
    /// Return the matching key in the index, and its entries
    pub fn lookup(&self, word: &str) -> Option<(&'a str, impl Iterator<Item = Entry<'a>> + 'a)> {
        if word.is_empty() || word.len() > usize::from(u8::MAX) {
            return None;
        }
        let records = self.records;
        let entries = self.entries;
        let slots = self.slots;
        let tags = self.tags;
        let n_slots = tags.len();
        let hash = hash(word);
        let tag = tag(hash);
        let mut i = slot_index(hash, n_slots);
        loop {
            if tags[i] == 0 {
                return None;
            }
            let slot = &slots[i * SLOT_SIZE..(i + 1) * SLOT_SIZE];
            let key_len = usize::from(slot[10]);
            if tags[i] == tag && key_len == word.len() {
                let key_offset = read_u32(&slot[0..4]);
                let key = &records[key_offset..key_offset + key_len];
                if key == word {
                    let start = read_u32(&slot[4..8]);
                    let len = usize::from(read_u16(&slot[8..10]));
                    let entries = entries[start * ENTRY_SIZE..(start + len) * ENTRY_SIZE]
                        .as_chunks::<ENTRY_SIZE>()
                        .0
                        .iter()
                        .map(move |entry| {
                            let offset = read_u32(&entry[0..4]);
                            let len = usize::from(read_u16(&entry[4..6]));
                            Entry {
                                record: &records[offset..offset + len],
                                type_: entry[6],
                                flags: entry[7],
                            }
                        });
                    return Some((key, entries));
                }
            }
            i += 1;
            if i == n_slots {
                i = 0;
            }
        }
    }
}

fn read_u32(bytes: &[u8]) -> usize {
    u32::from_le_bytes(bytes.try_into().expect("4 bytes")) as usize
}

fn read_u16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes(bytes.try_into().expect("2 bytes"))
}

/// A hash function that gives the same results at compile time and at runtime, whatever the CPU
fn hash(key: &str) -> u64 {
    // FxHash, with a final mixing step (from MurmurHash3) since the high bits are used for the
    // index, and the low bits for the tag
    const K: u64 = 0xf135_7aea_2e62_a9c5;
    let mut hash: u64 = key.len() as u64;
    let (chunks, remainder) = key.as_bytes().as_chunks::<8>();
    for &chunk in chunks {
        hash = (hash.rotate_left(5) ^ u64::from_le_bytes(chunk)).wrapping_mul(K);
    }
    let mut last = [0; 8];
    last[..remainder.len()].copy_from_slice(remainder);
    hash = (hash.rotate_left(5) ^ u64::from_le_bytes(last)).wrapping_mul(K);
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(0xff51_afd7_ed55_8ccd);
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    hash ^ (hash >> 33)
}

/// The tag of a used slot (never 0, which marks empty slots), using the low bits of the hash
fn tag(hash: u64) -> u8 {
    0x80 | (hash as u8 & 0x7f)
}

/// Map a hash to a slot, using the high bits of the hash
fn slot_index(hash: u64, n_slots: usize) -> usize {
    ((u128::from(hash) * n_slots as u128) >> 64) as usize
}

/// Build the index of a dictionary in the EDICT format; only index the entries at the keys for
/// which keep_key returns true, and drop the entries without any such key
pub fn build_index(data: &str, keep_key: impl Fn(&str) -> bool) -> Result<IndexData, ParseError> {
    let mut records = String::new();
    let mut entries: Vec<(usize, usize, u8, u8)> = Vec::new();
    // (key offset, key length, entry index) for each key of each entry
    let mut pairs: Vec<(usize, usize, usize)> = Vec::new();
    // NOTE: skip(1) for header on first line
    for (lineno, line) in data.lines().enumerate().skip(1) {
        let lineno = lineno + 1;
        let (headwords, glosses) = line.split_once(" /").ok_or(ParseError {
            lineno,
            expected: " /",
        })?;
        // each sense may start with its own part-of-speech tags; for example:
        // たい /(aux-adj) (1) want to ... do something/…/(suf,adj-i) (3) very .../(P)/EntL2017560X/
        let mut type_: u8 = 1 << 7;
        for sense in glosses.split('/') {
            if let Some(rest) = sense.strip_prefix('(') {
                let (tags, _) = rest.split_once(')').ok_or(ParseError {
                    lineno,
                    expected: ")",
                })?;
                type_ |= type_from_tags(tags);
            }
        }

        let (writings, readings) = if let Some((writings, readings)) = headwords.split_once(" [") {
            // both writings, and readings (within brackets); for example:
            // 日本 [にほん(P);にっぽん] /(n) Japan/(P)/EntL1582710X/
            let readings = readings.strip_suffix(']').ok_or(ParseError {
                lineno,
                expected: "] /",
            })?;
            (writings, readings)
        } else {
            // only readings; for example:
            // あやかし /(n) (1) ghost that appears at sea during a shipwreck/…/EntL2143630X/
            ("", headwords)
        };
        if !iter_forms(writings)
            .chain(iter_forms(readings))
            .any(&keep_key)
        {
            continue;
        }

        let mut flags = 0;
        if !writings.is_empty() && !line.contains("(uk)") {
            flags |= FLAG_KANJI_WORD;
        }
        let entry_index = entries.len();
        let record_start = records.len();
        for (field, forms) in [writings, readings].into_iter().enumerate() {
            if field != 0 {
                records.push('\t');
            }
            for (i, form) in iter_forms(forms).enumerate() {
                if i != 0 {
                    records.push(';');
                }
                if keep_key(form) {
                    pairs.push((records.len(), form.len(), entry_index));
                }
                records.push_str(form);
            }
        }
        records.push('\t');
        let output_type = type_ & !(1 << 7);
        if output_type != 0 {
            write!(records, "{output_type}").expect("writing to a String should not fail");
        }
        records.push('\t');
        push_meanings(&mut records, glosses.strip_suffix('/').unwrap_or(glosses));
        entries.push((record_start, records.len() - record_start, type_, flags));
        records.push('\n');
    }

    // group the entries by key, keeping the order of the lines
    let key_str = |&(offset, len, _): &(usize, usize, usize)| &records[offset..offset + len];
    pairs.sort_by(|a, b| key_str(a).cmp(key_str(b)));
    let n_keys = pairs.chunk_by(|a, b| key_str(a) == key_str(b)).count();
    // NOTE: at least one slot must stay empty, so that looking up a missing key stops
    let n_slots = (n_keys as f64 / MAX_LOAD_FACTOR).ceil() as usize + 1;
    let mut slots = vec![0; n_slots * SLOT_SIZE];
    let mut tags = vec![0; n_slots];
    let mut index_entries = Vec::with_capacity(pairs.len() * ENTRY_SIZE);
    let mut n_index_entries = 0;
    for chunk in pairs.chunk_by(|a, b| key_str(a) == key_str(b)) {
        let (key_offset, key_len, _) = chunk[0];
        let key = key_str(&chunk[0]);
        let hash = hash(key);
        let mut i = slot_index(hash, n_slots);
        while tags[i] != 0 {
            i += 1;
            if i == n_slots {
                i = 0;
            }
        }
        tags[i] = tag(hash);
        let slot = &mut slots[i * SLOT_SIZE..(i + 1) * SLOT_SIZE];
        slot[0..4].copy_from_slice(&to_u32(key_offset).to_le_bytes());
        slot[4..8].copy_from_slice(&to_u32(n_index_entries).to_le_bytes());
        let n_entries =
            u16::try_from(chunk.len()).expect("keys should have fewer than 65536 entries");
        slot[8..10].copy_from_slice(&n_entries.to_le_bytes());
        slot[10] = u8::try_from(key_len).expect("keys should be shorter than 256 bytes");
        for &(_, _, entry_index) in chunk {
            let (offset, len, type_, flags) = entries[entry_index];
            index_entries.extend_from_slice(&to_u32(offset).to_le_bytes());
            let len = u16::try_from(len).expect("records should be shorter than 64 KiB");
            index_entries.extend_from_slice(&len.to_le_bytes());
            index_entries.push(type_);
            index_entries.push(flags);
        }
        n_index_entries += chunk.len();
    }

    Ok(IndexData {
        records,
        entries: index_entries,
        slots,
        tags,
    })
}

fn to_u32(value: usize) -> u32 {
    u32::try_from(value).expect("dictionary should be smaller than 4 GiB")
}

/// Iterate over the forms of a list of headwords, without their markers; for example:
/// 行く(P);往く(rK) → 行く, 往く
fn iter_forms(forms: &str) -> impl Iterator<Item = &str> {
    forms
        .split(';')
        .filter(|form| !form.is_empty())
        .map(|form| form.split_once('(').map_or(form, |(form, _)| form))
}

/// The type of a word from the part-of-speech tags of a sense, matching the type of deinflection
/// rules (see Deinflector)
fn type_from_tags(tags: &str) -> u8 {
    let mut type_ = 1 << 7;
    for tag in tags.split(',') {
        if tag == "v1" {
            type_ |= 1 << 0;
        } else if tag.starts_with("v5") {
            type_ |= 1 << 1;
            // the deinflection rules for 行く (った→く, etc.) only target this type
            if tag == "v5k-s" {
                type_ |= 1 << 6;
            }
        } else if tag == "adj-i" {
            type_ |= 1 << 2;
        } else if tag == "vk" {
            type_ |= 1 << 3;
        } else if tag == "vs" || tag.starts_with("vs-") {
            type_ |= 1 << 4;
        }
    }
    type_
}

/// Whether a string can be the key of a name: at least two characters, and no hiragana (names are
/// mostly matched through their readings otherwise; e.g. のあ for 乃亜, 乃愛, etc.)
pub fn could_be_name(key: &str) -> bool {
    let mut chars = key.chars();
    if chars.next().is_none() || chars.next().is_none() {
        return false;
    }
    !key.chars().any(|c| matches!(c, '\u{3040}'..='\u{309f}'))
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

/// Split the tags at the start of a gloss (part-of-speech, field, meaning number, etc.); return
/// whether one of them is a meaning number (which starts a new meaning) and the rest of the gloss;
/// for example: (v5k-s,vi) (2) to leave → (true, "to leave")
fn split_gloss_tags(gloss: &str) -> (bool, &str) {
    let mut rest = gloss;
    let mut has_number = false;
    while rest.starts_with('(') {
        let Some(i) = rest[1..].find(") ") else {
            break;
        };
        let tag = &rest[1..1 + i];
        has_number |= !tag.is_empty() && tag.bytes().all(|b| b.is_ascii_digit());
        rest = &rest[1 + i + ") ".len()..];
    }
    (has_number, rest)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DATA: &str = "header
行く(P);往く(rK) [いく(P);ゆく(P)] /(v5k-s,vi) (1) to go/to move/(v5k-s,vi) (2) to leave/(P)/EntL1578850X/
あの /(adj-pn) that/EntL1/
燐 [りん;リン] /(n) (uk) phosphorus (P)/EntL1555530X/
氏 [し] /(n) Mr/EntL2/
";

    fn entries<'a>(index: &Index<'a>, word: &str) -> Vec<Entry<'a>> {
        index
            .lookup(word)
            .map(|(_, entries)| entries.collect())
            .unwrap_or_default()
    }

    #[test]
    fn test_build_index() {
        let data = build_index(DATA, |_| true).unwrap();
        let index = data.as_index();
        let iku = Entry {
            record: "行く;往く\tいく;ゆく\t66\tto go; to move/to leave",
            type_: 1 << 7 | 1 << 6 | 1 << 1,
            flags: FLAG_KANJI_WORD,
        };
        assert_eq!(entries(&index, "行く"), [iku]);
        assert_eq!(entries(&index, "ゆく"), [iku]);
        assert_eq!(index.lookup("ゆく").unwrap().0, "ゆく");
        assert_eq!(entries(&index, "あの")[0].record, "\tあの\t\tthat");
        assert_eq!(entries(&index, "あの")[0].flags, 0);
        let rin = entries(&index, "リン");
        assert_eq!(rin[0].record, "燐\tりん;リン\t\tphosphorus (P)");
        assert_eq!(rin[0].flags, 0);
        assert_eq!(entries(&index, "し")[0].flags, FLAG_KANJI_WORD);
        assert!(index.lookup("行").is_none());
        assert!(index.lookup("").is_none());
    }

    #[test]
    fn test_build_index_key_filter() {
        let data = build_index(DATA, could_be_name).unwrap();
        let index = data.as_index();
        assert!(index.lookup("リン").is_some());
        assert!(index.lookup("燐").is_none());
        assert!(index.lookup("行く").is_none());
        // entries without any key are dropped
        assert!(!data.records.contains("to go"));
        assert!(!data.records.contains("Mr"));
    }

    #[test]
    fn test_split_gloss_tags() {
        assert_eq!(split_gloss_tags("dot"), (false, "dot"));
        assert_eq!(split_gloss_tags("(n) dot"), (false, "dot"));
        assert_eq!(split_gloss_tags("(n) (1) (uk) tooth"), (true, "tooth"));
        assert_eq!(
            split_gloss_tags("(v5k-s,vi) (2) to leave"),
            (true, "to leave")
        );
        assert_eq!(split_gloss_tags("(See 来る・1) to go"), (false, "to go"));
        assert_eq!(split_gloss_tags("(unclosed tag"), (false, "(unclosed tag"));
        assert_eq!(
            split_gloss_tags("phosphorus (P)"),
            (false, "phosphorus (P)")
        );
    }

    #[test]
    fn test_split_semicolon_space() {
        for s in ["", "a", "a; b", "a;b; c", "a; ", "; a", ";", "a;;  b"] {
            let expected: Vec<&str> = s.split("; ").collect();
            assert_eq!(
                split_semicolon_space(s).collect::<Vec<_>>(),
                expected,
                "{s:?}"
            );
        }
    }

    #[test]
    fn test_could_be_name() {
        assert!(could_be_name("穴水町"));
        assert!(could_be_name("ツカ"));
        assert!(!could_be_name("森"));
        assert!(!could_be_name("のあ"));
    }
}
