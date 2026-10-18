// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Owned native journals retained by the existing whole-refutation checker.

use crate::owl_dl::graph::changes::Journal;
use purrdf_lex::allocation::{Memory, Resident, StorageError};

pub(crate) struct Clash {
    pub(crate) index: usize,
    pub(crate) journal: Journal,
    /// Terminal clause whose bound derivation was checked in this source phase.
    pub(crate) schema: Option<usize>,
}

impl std::fmt::Debug for Clash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Clash")
            .field("index", &self.index)
            .field("journal", &self.journal)
            .field("schema", &self.schema)
            .finish()
    }
}

#[derive(Default)]
pub(crate) struct Capture {
    pub(crate) clashes: Vec<Clash>,
    storage: Resident,
    live: usize,
}

impl std::fmt::Debug for Capture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Capture")
            .field("clashes", &self.clashes)
            .field("storage", &self.storage)
            .field("live", &self.live)
            .finish()
    }
}

impl Capture {
    pub(super) fn push(
        &mut self,
        index: usize,
        journal: Journal,
        schema: Option<super::CheckedSchema<'_>>,
    ) -> Result<(), StorageError> {
        let mut memory = Memory::resume(&mut self.storage, self.live);
        let result = memory.push(
            &mut self.clashes,
            Clash {
                index,
                journal,
                schema: schema.map(|checked| checked.clause),
            },
        );
        self.live = memory.admitted_bytes();
        result
    }
}
