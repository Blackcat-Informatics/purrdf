// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Witnesses retained by the original Tarjan condensation and sparse closure.

use super::{Role, source::Input};
use purrdf_lex::allocation::{Admission, Memory, OrderedTable, StorageError};

#[derive(Clone, Copy)]
pub(super) enum Step {
    Input(Input),
    TopInverse,
    Join {
        first: usize,
        second: usize,
        via: Option<usize>,
    },
}

impl std::fmt::Debug for Step {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Input(input) => f.debug_tuple("Input").field(input).finish(),
            Self::TopInverse => f.write_str("TopInverse"),
            Self::Join { first, second, via } => f
                .debug_tuple("Join")
                .field(first)
                .field(second)
                .field(via)
                .finish(),
        }
    }
}

pub(super) struct Witnesses {
    pub(super) components: Vec<Option<usize>>,
    pub(super) steps: Vec<Step>,
    pairs: OrderedTable<(usize, usize), usize>,
}

impl std::fmt::Debug for Witnesses {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Witnesses")
            .field("components", &self.components)
            .field("steps", &self.steps)
            .field("pair_count", &self.pairs.len())
            .finish()
    }
}

impl Witnesses {
    pub(super) fn new<S: Admission + ?Sized>(
        count: usize,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, StorageError> {
        Ok(Self {
            components: memory.collect(std::iter::repeat_n(None, count))?,
            steps: Vec::new(),
            pairs: OrderedTable::default(),
        })
    }

    fn step<S: Admission + ?Sized>(
        &mut self,
        step: Step,
        memory: &mut Memory<'_, S>,
    ) -> Result<usize, StorageError> {
        let id = self.steps.len();
        memory.push(&mut self.steps, step)?;
        Ok(id)
    }

    pub(super) fn edge<S: Admission + ?Sized>(
        &mut self,
        components: (usize, usize),
        roles: (Role, Role),
        top: Option<Role>,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        let (from, to) = components;
        if from != to && self.pairs.get(&(from, to)).is_some() {
            return Ok(());
        }
        let (sub, sup) = roles;
        let step = if top.is_some_and(|top| {
            (sub == top && sup == top.inverse()) || (sup == top && sub == top.inverse())
        }) {
            Step::TopInverse
        } else {
            Step::Input(Input::Inclusion(sub, sup))
        };
        let leaf = self.step(step, memory)?;
        if from == to {
            let root = match self.components[from] {
                None => leaf,
                Some(first) => self.step(
                    Step::Join {
                        first,
                        second: leaf,
                        via: None,
                    },
                    memory,
                )?,
            };
            self.components[from] = Some(root);
        } else {
            self.pairs.insert((from, to), leaf, memory)?;
        }
        Ok(())
    }

    pub(super) fn extend<S: Admission + ?Sized>(
        &mut self,
        from: usize,
        via: usize,
        to: usize,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        if self.pairs.get(&(from, to)).is_some() {
            return Ok(());
        }
        let first = *self
            .pairs
            .get(&(from, via))
            .expect("original direct condensation arc");
        let second = *self
            .pairs
            .get(&(via, to))
            .expect("completed lower component closure");
        let root = self.step(
            Step::Join {
                first,
                second,
                via: Some(via),
            },
            memory,
        )?;
        self.pairs.insert((from, to), root, memory)?;
        Ok(())
    }

    pub(super) fn release<S: Admission + ?Sized>(
        self,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        self.pairs.release(memory)?;
        memory.release_vec(self.components)?;
        memory.release_vec(self.steps)
    }

    #[cfg(test)]
    pub(super) fn pair(&self, from: usize, to: usize) -> Option<usize> {
        self.pairs.get(&(from, to)).copied()
    }
}
