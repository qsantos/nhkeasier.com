use std::collections::HashSet;
use std::include_str;
use std::time::Instant;

use ouroboros::self_referencing;

use crate::{Deinflector, Edict, Error, iter_fragments};

#[self_referencing]
pub struct SubEdictCreator {
    edict2_data: &'static str,
    deinflector_data: &'static str,
    #[borrows(edict2_data)]
    #[not_covariant]
    edict2: Edict<'this>,
    #[borrows(deinflector_data)]
    #[not_covariant]
    deinflector: Deinflector<'this>,
}

impl SubEdictCreator {
    pub fn from_files() -> Result<Self, Error> {
        SubEdictCreator::try_new(
            include_str!("../data/edict2"),
            include_str!("../data/deinflect"),
            |data| Edict::parse(data),
            |data| Deinflector::parse(data),
        )
    }

    pub fn from(&self, content: &str) -> Vec<&str> {
        self.with(|fields| {
            let start = Instant::now();
            let fragments: HashSet<&str> = iter_fragments(content).collect();

            let mut candidates = Vec::new();
            let mut buffer = Vec::new();
            for fragment in fragments {
                candidates.extend(fields.deinflector.deinflect(fragment, &mut buffer));
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

            let lines: Vec<&str> = lines.into_iter().collect();
            tracing::debug!(
                "sub-edict with {} entries generated in {:?}",
                lines.len(),
                start.elapsed()
            );
            lines
        })
    }
}

#[self_referencing]
pub struct SubEnamdictCreator {
    enamdict_data: &'static str,
    #[borrows(enamdict_data)]
    #[not_covariant]
    enamdict: Edict<'this>,
}

impl SubEnamdictCreator {
    pub fn from_files() -> Result<Self, Error> {
        SubEnamdictCreator::try_new(include_str!("../data/enamdict"), |data| Edict::parse(data))
    }

    pub fn from(&self, content: &str) -> Vec<&str> {
        self.with(|fields| {
            let start = Instant::now();
            let fragments: HashSet<&str> = iter_fragments(content).collect();

            let mut lines = HashSet::new();
            for fragment in fragments {
                if let Some(entries) = fields.enamdict.lookup(fragment as &str) {
                    for entry in entries.iter() {
                        lines.insert(entry.line);
                    }
                }
            }

            let lines: Vec<&str> = lines.into_iter().collect();
            tracing::debug!(
                "sub-enamdict with {} entries generated in {:?}",
                lines.len(),
                start.elapsed()
            );
            lines
        })
    }
}
