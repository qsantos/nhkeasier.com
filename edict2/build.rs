//! Index the dictionaries at compile time (see edict2-index)

use std::path::Path;

use edict2_index::{build_index, could_be_name};

fn write_index(out_dir: &Path, name: &str, keep_key: impl Fn(&str) -> bool) {
    let path = format!("data/{name}");
    println!("cargo::rerun-if-changed={path}");
    let data = std::fs::read_to_string(&path).expect("failed to read dictionary");
    let index =
        build_index(&data, keep_key).unwrap_or_else(|e| panic!("failed to parse {path}: {e:?}"));
    std::fs::write(out_dir.join(format!("{name}.records")), index.records)
        .expect("failed to write records");
    std::fs::write(out_dir.join(format!("{name}.entries")), index.entries)
        .expect("failed to write entries");
    std::fs::write(out_dir.join(format!("{name}.slots")), index.slots)
        .expect("failed to write slots");
    std::fs::write(out_dir.join(format!("{name}.tags")), index.tags).expect("failed to write tags");
}

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    let out_dir = std::env::var("OUT_DIR").expect("missing OUT_DIR");
    let out_dir = Path::new(&out_dir);
    write_index(out_dir, "edict2", |_| true);
    // only index the names that can be looked up (see SubEnamdictCreator::from)
    write_index(out_dir, "enamdict", could_be_name);
}
