use std::borrow::Cow;
use std::sync::LazyLock;

use regex::Regex;

static REMOVE_RUBY_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new("<rp>.*?</rp>|<rt>.*?</rt>|<rtc>.*?</rtc>|<ruby>|</ruby>").unwrap()
});

pub fn remove_ruby(s: &str) -> Cow<'_, str> {
    REMOVE_RUBY_REGEX.replace_all(s, "")
}

#[test]
fn test() {
    assert_eq!(remove_ruby("no ruby"), "no ruby");
    assert_eq!(remove_ruby("あいうえお"), "あいうえお");
    assert_eq!(
        remove_ruby(
            "ベツレヘム　ガザ<ruby>地区<rt>ちく</rt></ruby>で<ruby>亡<rt>な</rt></ruby>くなった<ruby>人<rt>ひと</rt></ruby>のために<ruby>祈<rt>いの</rt></ruby>る"
        ),
        "ベツレヘム　ガザ地区で亡くなった人のために祈る",
    );
}
