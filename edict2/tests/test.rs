use std::io::{BufWriter, Write, stdout};
use std::sync::LazyLock;

use edict2::{SubEdictCreator, SubEnamdictCreator};

static SUB_EDICT_CREATOR: LazyLock<SubEdictCreator> =
    LazyLock::new(|| SubEdictCreator::from_files().expect("failed to load EDICT2"));
static SUB_ENAMDICT_CREATOR: LazyLock<SubEnamdictCreator> =
    LazyLock::new(|| SubEnamdictCreator::from_files().expect("failed to load ENAMDICT"));

fn edict_headwords(content: &str) -> Vec<String> {
    SUB_EDICT_CREATOR
        .from(content)
        .into_iter()
        .map(|line| line.split('\t').take(2).collect::<Vec<_>>().join("\t"))
        .collect()
}

fn enamdict_headwords(content: &str) -> Vec<String> {
    SUB_ENAMDICT_CREATOR
        .from(content, &SUB_EDICT_CREATOR)
        .into_iter()
        .map(|line| line.split('\t').take(2).collect::<Vec<_>>().join("\t"))
        .collect()
}

#[test]
fn test() {
    let data = include_str!("test-input");
    let lines = SUB_EDICT_CREATOR.from(data);
    let mut writer = BufWriter::with_capacity(8192, stdout().lock());
    for line in lines {
        let _ = writer.write(line.as_bytes()).expect("failed to write line");
        let _ = writer.write(b"\n").expect("failed to write newline");
    }
}

#[test]
fn test_deinflection() {
    assert!(edict_headwords("書いた").contains(&"書く\tかく".to_string()));
    // v5k-s
    for text in ["行った", "行って", "行ったら", "行ったり"] {
        let headwords = edict_headwords(text);
        assert!(headwords.iter().any(|h| h.starts_with("行く\t")), "{text}");
    }
    // the adj-i tag of たい is not on its first sense
    assert!(edict_headwords("行きたかった").contains(&"\tたい".to_string()));
}

#[test]
fn test_lone_kana() {
    let headwords = edict_headwords("します");
    assert!(!headwords.iter().any(|h| h.starts_with("氏\t")));
    assert!(headwords.iter().any(|h| h.ends_with("\tする")));
}

#[test]
fn test_names() {
    let headwords = enamdict_headwords("カムチャツカ半島");
    assert!(headwords.contains(&"カムチャツカ半島\tカムチャツカはんとう".to_string()));
    assert!(!headwords.iter().any(|h| h.starts_with("ツカ\t")));
    // words are not looked up as names
    assert!(enamdict_headwords("森").is_empty());
}
