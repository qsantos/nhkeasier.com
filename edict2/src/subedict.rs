use std::collections::HashSet;
use std::fs::read_to_string;

use ouroboros::self_referencing;

use crate::{iter_fragments, Deinflector, Edict};

#[self_referencing]
pub struct SubEdictCreator {
    edict2_data: String,
    deinflector_data: String,
    #[borrows(edict2_data)]
    #[not_covariant]
    edict2: Edict<'this>,
    #[borrows(deinflector_data)]
    #[not_covariant]
    deinflector: Deinflector<'this>,
}

impl SubEdictCreator {
    pub fn from_files() -> Self {
        SubEdictCreatorBuilder {
            edict2_data: read_to_string("edict2").unwrap(),
            deinflector_data: read_to_string("deinflect.dat").unwrap(),
            edict2_builder: |data| Edict::parse(data),
            deinflector_builder: |data| Deinflector::parse(data),
        }
        .build()
    }

    pub fn from(&self, content: &str) -> Vec<&str> {
        self.with(|fields| {
            let fragments: HashSet<&str> = iter_fragments(content).collect();

            let mut candidates = Vec::new();
            for fragment in fragments {
                candidates.extend(fields.deinflector.deinflect(fragment));
            }

            let mut lines = HashSet::new();
            for candidate in candidates {
                if let Some(entries) = fields.edict2.lookup(&candidate.word as &str) {
                    for entry in entries {
                        if entry.type_ & candidate.type_ != 0 {
                            lines.insert(entry.line);
                        }
                    }
                }
            }

            let mut lines: Vec<&str> = lines.into_iter().collect();
            lines.sort();
            lines
        })
    }
}

#[self_referencing]
pub struct SubEnamdictCreator {
    enamdict_data: String,
    #[borrows(enamdict_data)]
    #[not_covariant]
    enamdict: Edict<'this>,
}

impl SubEnamdictCreator {
    pub fn from_files() -> Self {
        SubEnamdictCreatorBuilder {
            enamdict_data: read_to_string("enamdict").unwrap(),
            enamdict_builder: |data| Edict::parse(data),
        }
        .build()
    }

    pub fn from(&self, content: &str) -> Vec<&str> {
        self.with(|fields| {
            let fragments: HashSet<&str> = iter_fragments(content).collect();

            let mut lines = HashSet::new();
            for fragment in fragments {
                if let Some(entries) = fields.enamdict.lookup(&fragment as &str) {
                    for entry in entries {
                        lines.insert(entry.line);
                    }
                }
            }

            let mut lines: Vec<&str> = lines.into_iter().collect();
            lines.sort();
            lines
        })
    }
}
