use std::io::{stdout, BufWriter, Write};

use edict2::SubEdictCreator;

#[test]
fn test() {
    let sub_edict_creator = SubEdictCreator::from_files();

    let data = std::fs::read_to_string("test-input").unwrap();
    let lines = sub_edict_creator.from(&data);
    let mut writer = BufWriter::with_capacity(8192, stdout().lock());
    for line in lines {
        let _ = writer.write(line.as_bytes()).unwrap();
        let _ = writer.write(b"\n").unwrap();
    }
}
