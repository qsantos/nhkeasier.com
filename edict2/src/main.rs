use edict2::{Deinflector, Edict, iter_fragments};

fn main() {
    let data = std::fs::read_to_string("edict2").unwrap();
    let edict2 = Edict::parse(&data);

    let data = std::fs::read_to_string("deinflect.dat").unwrap();
    let deinflector = Deinflector::parse(&data);

    let data = std::fs::read_to_string("test-input").unwrap();
    for fragment in iter_fragments(&data) {
        for candidate in deinflector.deinflect(fragment) {
            if let Some(entries) = edict2.lookup(&candidate.word as &str) {
                for entry in entries {
                    if entry.type_ & candidate.type_ != 0 {
                        println!("{}", entry.line);
                    }
                }
            }
        }
    }
}
