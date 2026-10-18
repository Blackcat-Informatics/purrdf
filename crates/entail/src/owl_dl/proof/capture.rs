// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Owned dependencies selected in the original whole-refutation checker's phase.

use crate::owl_dl::graph::changes::{Journal, slice};
use purrdf_lex::allocation::{Memory, Resident};

pub(crate) struct Clash {
    pub(crate) index: usize,
    pub(crate) merge: bool,
    /// Terminal clause whose bound derivation was checked in this source phase.
    pub(crate) schema: Option<usize>,
    pub(crate) support: Result<slice::Support, slice::Error>,
}

impl std::fmt::Debug for Clash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Clash")
            .field("index", &self.index)
            .field("merge", &self.merge)
            .field("schema", &self.schema)
            .field("support", &self.support)
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
        journal: &Journal,
        schema: Option<super::CheckedSchema<'_>>,
        kb: &crate::owl_dl::Kb,
    ) -> Result<(), super::DlProofError> {
        self.push_leaf(index, false, journal, schema, kb)
    }

    pub(super) fn push_merge(
        &mut self,
        index: usize,
        journal: &Journal,
        kb: &crate::owl_dl::Kb,
    ) -> Result<(), super::DlProofError> {
        self.push_leaf(index, true, journal, None, kb)
    }

    fn push_leaf(
        &mut self,
        index: usize,
        merge: bool,
        journal: &Journal,
        schema: Option<super::CheckedSchema<'_>>,
        kb: &crate::owl_dl::Kb,
    ) -> Result<(), super::DlProofError> {
        let support = journal
            .normalized_slice_with_schema(kb, schema)
            .and_then(slice::Slice::into_support);
        match &support {
            Err(
                slice::Error::Storage(original)
                | slice::Error::RoleLicence(crate::owl_dl::roles::LicenceError::Storage(original)),
            ) => {
                return Err(super::DlProofError::Storage(*original));
            }
            Err(
                slice::Error::Stopped
                | slice::Error::RoleLicence(crate::owl_dl::roles::LicenceError::Stopped),
            ) => {
                return Err(super::DlProofError::Stopped);
            }
            _ => {}
        }
        let mut memory = Memory::resume(&mut self.storage, self.live);
        let result = memory.push(
            &mut self.clashes,
            Clash {
                index,
                merge,
                schema: schema.map(|checked| checked.clause),
                support,
            },
        );
        self.live = memory.admitted_bytes();
        result.map_err(super::DlProofError::Storage)
    }
}
