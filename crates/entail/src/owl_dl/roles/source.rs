// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original hierarchy inputs retained during the native reader's one pass.

use super::Role;
use purrdf_lex::allocation::{Admission, Memory, OrderedTable, StorageError};
use std::ops::Range;
#[cfg(test)]
mod tests;

pub(crate) type Triple = (u32, u32, u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Input {
    Chain(usize),
    Inclusion(Role, Role),
    Reflexive(Role),
}

pub(crate) struct Origin {
    pub(crate) anchor: Triple,
    pub(crate) shared: Range<usize>,
    pub(crate) reads: Range<usize>,
}

impl std::fmt::Debug for Origin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Origin")
            .field("anchor", &self.anchor)
            .field("shared", &self.shared)
            .field("reads", &self.reads)
            .finish()
    }
}

#[derive(Default)]
pub(crate) struct Sources {
    owners: OrderedTable<Input, usize>,
    pub(crate) origins: Vec<Origin>,
    pub(crate) reads: Vec<Triple>,
}

impl std::fmt::Debug for Sources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sources")
            .field("owner_count", &self.owners.len())
            .field("origins", &self.origins)
            .field("reads", &self.reads)
            .finish()
    }
}

impl Sources {
    pub(crate) fn publish<S: Admission + ?Sized>(
        &mut self,
        inputs: &[Input],
        anchor: Triple,
        shared: Range<usize>,
        start: usize,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        if inputs.iter().all(|input| self.owners.get(input).is_some()) {
            self.reads.truncate(start);
            return Ok(());
        }
        let id = self.origins.len();
        memory.push(
            &mut self.origins,
            Origin {
                anchor,
                shared,
                reads: start..self.reads.len(),
            },
        )?;
        for &input in inputs {
            if self.owners.get(&input).is_none() {
                self.owners.insert(input, id, memory)?;
            }
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn get(&self, input: Input) -> Option<&Origin> {
        self.origins.get(*self.owners.get(&input)?)
    }

    pub(super) fn release<S: Admission + ?Sized>(
        self,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        self.owners.release(memory)?;
        memory.release_vec(self.origins)?;
        memory.release_vec(self.reads)
    }
}
