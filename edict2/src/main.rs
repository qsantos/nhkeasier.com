use regex::Regex;

use edict2::{Deinflector, Edict};

fn main() {
    let data = std::fs::read_to_string("edict2").unwrap();
    let edict2 = Edict::parse(&data);

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
                    if let Some(entries) = edict2.lookup(&candidate.word as &str) {
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
