// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original input owners, recorded at the native parser/derivation boundary.
//! Anchors are not complete expression syntax or key-inference proofs.

use super::AxiomSource;
use purrdf_lex::allocation::{Memory, OrderedTable, Resident, StorageError};
#[cfg(test)]
mod tests;

/// Vector ordinals or exact normalized role-rule keys; never source term names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Input {
    Type(usize),
    Role(usize),
    Equality(usize),
    Distinct(usize),
    Key(usize),
    Inverse(u32, u32),
    SubRole { sub: u32, sup: u32 },
    Transitive(u32),
    Asymmetric(u32),
    DisjointRole(u32, u32),
}

impl Input {
    fn ordinal(self) -> Option<(usize, usize)> {
        match self {
            Self::Type(index) => Some((0, index)),
            Self::Role(index) => Some((1, index)),
            Self::Equality(index) => Some((2, index)),
            Self::Distinct(index) => Some((3, index)),
            Self::Key(index) => Some((4, index)),
            _ => None,
        }
    }
}

/// Both input owners and their shared syntax references use the same dense door.
struct Entries<T> {
    ordinals: [Vec<Option<T>>; 5],
    rules: OrderedTable<Input, T>,
}

impl<T> Default for Entries<T> {
    fn default() -> Self {
        Self {
            ordinals: std::array::from_fn(|_| Vec::new()),
            rules: OrderedTable::default(),
        }
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Entries<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Entries")
            .field("ordinals", &self.ordinals)
            .field("rule_count", &self.rules.len())
            .finish()
    }
}

impl<T> Entries<T> {
    fn get(&self, input: Input) -> Option<&T> {
        match input.ordinal() {
            Some((family, index)) => self.ordinals[family].get(index)?.as_ref(),
            None => self.rules.get(&input),
        }
    }

    fn record(
        &mut self,
        input: Input,
        value: T,
        memory: &mut Memory<'_, Resident>,
    ) -> Result<(), StorageError> {
        let Some((family, index)) = input.ordinal() else {
            return self.rules.insert(input, value, memory).map(|_| ());
        };
        let rows = &mut self.ordinals[family];
        let required = index.checked_add(1).ok_or(StorageError::SizeOverflow)?;
        if required > rows.capacity() {
            let capacity =
                purrdf_lex::allocation::growth_capacity::<Option<T>>(rows.capacity(), required)?;
            memory.reserve(rows, capacity)?;
        }
        if rows.len() < required {
            rows.resize_with(required, || None);
        }
        rows[index] = Some(value);
        Ok(())
    }
}

/// Preserve the actual producer rather than interpreting every prepared row as RDF.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Source {
    Anchor(AxiomSource),
    LiteralValue(u32),
    KeyInference { key: usize, left: u32, right: u32 },
}

impl std::fmt::Debug for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Anchor(anchor) => f.debug_tuple("Anchor").field(anchor).finish(),
            Self::LiteralValue(term) => f.debug_tuple("LiteralValue").field(term).finish(),
            Self::KeyInference { key, left, right } => f
                .debug_struct("KeyInference")
                .field("key", key)
                .field("left", left)
                .field("right", right)
                .finish(),
        }
    }
}

/// Missing entries are explicit missing provenance, including synthetic Kbs.
#[derive(Default)]
pub(crate) struct Inputs {
    sources: Entries<Source>,
    syntax: Entries<std::num::NonZeroUsize>,
    pending: Vec<Input>,
    active: bool,
    complete: bool,
    storage: Resident,
    live: usize,
    #[cfg(test)]
    refusal: Option<(usize, StorageError)>,
}

impl std::fmt::Debug for Inputs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut value = f.debug_struct("Inputs");
        value
            .field("sources", &self.sources)
            .field("syntax", &self.syntax)
            .field("pending", &self.pending)
            .field("active", &self.active)
            .field("complete", &self.complete)
            .field("storage", &self.storage)
            .field("live", &self.live);
        #[cfg(test)]
        value.field("refusal", &self.refusal);
        value.finish()
    }
}

impl Inputs {
    pub(crate) fn get(&self, input: Input) -> Option<Source> {
        self.sources.get(input).copied()
    }

    pub(crate) fn syntax(&self, input: Input) -> Option<super::syntax::Capture> {
        use super::syntax::Capture;
        if let Some(root) = self.syntax.get(input) {
            return Some(Capture::Root(root.get() - 1));
        }
        (self.complete
            && matches!(
                self.get(input),
                Some(Source::Anchor(_) | Source::LiteralValue(_))
            ))
        .then_some(Capture::Empty)
    }

    pub(crate) fn finish_parse(&mut self) {
        assert!(!self.active, "input capture finished at an axiom boundary");
        self.complete = true;
    }

    pub(crate) fn begin_scope(&mut self) {
        assert!(!self.active, "parser input scopes are not nested");
        self.pending.clear();
        self.active = true;
    }

    pub(crate) fn end_scope(&mut self, root: Option<usize>) -> Result<(), StorageError> {
        self.active = false;
        if root.is_some() {
            for &input in &self.pending {
                assert!(
                    self.syntax(input).is_none(),
                    "one syntax owner per parsed input"
                );
            }
        }
        let mut memory = Memory::resume(&mut self.storage, self.live);
        let result = (|| {
            if let Some(root) = root {
                let root = std::num::NonZeroUsize::new(
                    root.checked_add(1).ok_or(StorageError::SizeOverflow)?,
                )
                .expect("a syntax ordinal is one-based");
                for &input in &self.pending {
                    self.syntax.record(input, root, &mut memory)?;
                }
            }
            Ok(())
        })();
        self.live = memory.admitted_bytes();
        self.pending.clear();
        result
    }

    /// A set-valued native rule retains its original first insertion owner.
    /// Vector facts have distinct ordinal keys, even when their values coincide.
    pub(crate) fn record(&mut self, input: Input, source: Source) -> Result<(), StorageError> {
        assert!(
            self.get(input).is_none(),
            "native input provenance is recorded once at insertion"
        );
        #[cfg(test)]
        if let Some((remaining, original)) = self.refusal.as_mut() {
            if *remaining == 0 {
                let original = *original;
                self.refusal = None;
                return Err(original);
            }
            *remaining -= 1;
        }
        let mut memory = Memory::resume(&mut self.storage, self.live);
        let result = (|| {
            self.sources.record(input, source, &mut memory)?;
            if self.active {
                memory.push(&mut self.pending, input)?;
            }
            Ok(())
        })();
        self.live = memory.admitted_bytes();
        result
    }

    /// Admit a complete derived-equality batch before changing semantic rows.
    /// Failed publication or stopping removes only this batch's provenance.
    pub(crate) fn record_key_equalities(
        &mut self,
        start: usize,
        forced: &[(usize, u32, u32)],
        mut poll: impl FnMut() -> Result<(), crate::EntailError>,
    ) -> Result<(), crate::EntailError> {
        start
            .checked_add(forced.len())
            .ok_or(StorageError::SizeOverflow)?;
        let baseline = self.sources.ordinals[2].len();
        assert!(
            baseline <= start,
            "key publication appends equality provenance"
        );
        for (offset, &(key, left, right)) in forced.iter().enumerate() {
            let outcome = poll().and_then(|()| {
                self.record(
                    Input::Equality(start + offset),
                    Source::KeyInference { key, left, right },
                )
                .map_err(crate::EntailError::from)
            });
            if let Err(original) = outcome {
                self.sources.ordinals[2].truncate(baseline);
                return Err(original);
            }
        }
        Ok(())
    }
}
