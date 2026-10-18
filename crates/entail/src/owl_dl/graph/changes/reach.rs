// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Immutable witnesses produced by the native transitive walk/cache operations.

use super::super::super::concept::Role;
use purrdf_lex::allocation::{Memory, OrderedTable, Resident, StorageError};
#[cfg(test)]
mod tests;

/// A step from the original edge scanner, including its actual orientation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Step {
    pub(crate) edge: usize,
    pub(crate) from: usize,
    pub(crate) to: usize,
    pub(crate) letter: Role,
}

/// Children always precede a composition; no recursive ownership/drop is needed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Path {
    Step(Step),
    Join { first: usize, second: usize },
}

/// Current cache membership points into immutable historical witness nodes.
#[derive(Default)]
pub(crate) struct Witnesses {
    members: OrderedTable<(usize, usize, usize), usize>,
    pub(crate) paths: Vec<Path>,
}

impl std::fmt::Debug for Witnesses {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Witnesses")
            .field("members", &self.members.len())
            .field("paths", &self.paths)
            .finish()
    }
}

impl Witnesses {
    /// Equality invalidates lookup, while previously selected paths remain immutable.
    pub(super) fn clear(&mut self) {
        self.members.clear();
    }

    pub(super) fn path(&self, pattern: usize, root: usize, target: usize) -> usize {
        *self
            .members
            .get(&(pattern, root, target))
            .expect("native reach member has a retained witness")
    }

    fn join(
        &mut self,
        first: Option<usize>,
        second: usize,
        memory: &mut Memory<'_, Resident>,
    ) -> Result<usize, StorageError> {
        let Some(first) = first else {
            return Ok(second);
        };
        let id = self.paths.len();
        memory.push(&mut self.paths, Path::Join { first, second })?;
        Ok(id)
    }

    /// A newly discovered endpoint extends the predecessor actually used by DFS.
    pub(super) fn step(
        &mut self,
        pattern: usize,
        root: usize,
        step: Step,
        memory: &mut Memory<'_, Resident>,
    ) -> Result<(), StorageError> {
        let prefix = (step.from != root).then(|| self.path(pattern, root, step.from));
        let id = self.paths.len();
        memory.push(&mut self.paths, Path::Step(step))?;
        let id = self.join(prefix, id, memory)?;
        self.members.insert((pattern, root, step.to), id, memory)?;
        Ok(())
    }

    /// The cache's own append rule: owner→source, new edge, destination→target.
    pub(super) fn extend(
        &mut self,
        pattern: usize,
        owner: usize,
        step: Step,
        target: usize,
        memory: &mut Memory<'_, Resident>,
    ) -> Result<(), StorageError> {
        let prefix = (owner != step.from).then(|| self.path(pattern, owner, step.from));
        let suffix = (target != step.to).then(|| self.path(pattern, step.to, target));
        let edge = self.paths.len();
        memory.push(&mut self.paths, Path::Step(step))?;
        let joined = self.join(prefix, edge, memory)?;
        let joined = if let Some(suffix) = suffix {
            self.join(Some(joined), suffix, memory)?
        } else {
            joined
        };
        self.members
            .insert((pattern, owner, target), joined, memory)?;
        Ok(())
    }
}
