use std::collections::HashSet;
use std::io::{stdout, BufWriter, Write};

use edict2::{iter_fragments, Deinflector, Edict};

fn main() {
    let data = std::fs::read_to_string("edict2").unwrap();
    let edict2 = Edict::parse(&data);

    let data = std::fs::read_to_string("deinflect.dat").unwrap();
    let deinflector = Deinflector::parse(&data);

    let data = std::fs::read_to_string("test-input").unwrap();
    let fragments: HashSet<&str> = iter_fragments(&data).collect();

    let mut candidates = HashSet::new();
    for fragment in fragments  {
        candidates.extend(deinflector.deinflect(fragment));
    }

    let mut lines = HashSet::new();
    for candidate in candidates {
        if let Some(entries) = edict2.lookup(&candidate.word as &str) {
            for entry in entries {
                if entry.type_ & candidate.type_ != 0 {
                    lines.insert(entry.line);
                }
            }
        }
    }

    let mut lines: Vec<&str> = lines.into_iter().collect();
    lines.sort();
    let mut writer = BufWriter::with_capacity(8192, stdout().lock());
    for line in lines {
        let _ = writer.write(line.as_bytes()).unwrap();
        let _ = writer.write(b"\n").unwrap();
    }
}
