use std::sync::LazyLock;

use regex::Regex;

static FRAGMENT_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        "[",
        "々",                // IDEOGRAPHIC ITERATION MARK (U+3005)
        "\u{3040}-\u{30ff}", // Hiragana, Katakana
        "\u{3400}-\u{4dbf}", // CJK Unified Ideographs Extension A
        "\u{4e00}-\u{9fff}", // CJK Unified Ideographs
        "\u{f900}-\u{faff}", // CJK Compatibility Ideographs
        "\u{ff66}-\u{ff9f}", // Halfwidth and Fullwidth Forms Block (hiragana and katakana)
        "]+",
    ))
    .expect("invalid FRAGMENT_PATTERN regex")
});

pub fn iter_fragments(data: &str) -> impl Iterator<Item = &str> {
    // iter fragments
    FRAGMENT_PATTERN.find_iter(data).flat_map(|m| {
        let fragment = m.as_str();
        fragment.char_indices().flat_map(|(start, _)| {
            let suffix = &fragment[start..];
            suffix
                .char_indices()
                .map(|(end, c)| &suffix[..end + c.len_utf8()])
        })
    })
}
