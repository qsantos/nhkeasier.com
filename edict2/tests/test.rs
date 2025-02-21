use std::io::{BufWriter, Write, stdout};

use edict2::SubEdictCreator;

#[test]
fn test() {
    let sub_edict_creator = SubEdictCreator::from_files().expect("failed to load EDICT2");

    let data = include_str!("test-input");
    let lines = sub_edict_creator.from(data);
    let mut writer = BufWriter::with_capacity(8192, stdout().lock());
    for line in lines {
        let _ = writer.write(line.as_bytes()).expect("failed to write line");
        let _ = writer.write(b"\n").expect("failed to write newline");
    }
}
