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
    types: Vec<Option<Source>>,
    roles: Vec<Option<Source>>,
    equalities: Vec<Option<Source>>,
    distinct: Vec<Option<Source>>,
    keys: Vec<Option<Source>>,
    rules: OrderedTable<Input, Source>,
    storage: Resident,
    live: usize,
    #[cfg(test)]
    refusal: Option<(usize, StorageError)>,
}

impl std::fmt::Debug for Inputs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut value = f.debug_struct("Inputs");
        value
            .field("types", &self.types)
            .field("roles", &self.roles)
            .field("equalities", &self.equalities)
            .field("distinct", &self.distinct)
            .field("keys", &self.keys)
            .field("rule_count", &self.rules.len())
            .field("storage", &self.storage)
            .field("live", &self.live);
        #[cfg(test)]
        value.field("refusal", &self.refusal);
        value.finish()
    }
}

impl Inputs {
    pub(crate) fn get(&self, input: Input) -> Option<Source> {
        let (rows, index) = match input {
            Input::Type(index) => (&self.types, index),
            Input::Role(index) => (&self.roles, index),
            Input::Equality(index) => (&self.equalities, index),
            Input::Distinct(index) => (&self.distinct, index),
            Input::Key(index) => (&self.keys, index),
            _ => return self.rules.get(&input).copied(),
        };
        rows.get(index).copied().flatten()
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
        let result = match input {
            Input::Type(index) => Self::row(&mut self.types, index, source, &mut memory),
            Input::Role(index) => Self::row(&mut self.roles, index, source, &mut memory),
            Input::Equality(index) => Self::row(&mut self.equalities, index, source, &mut memory),
            Input::Distinct(index) => Self::row(&mut self.distinct, index, source, &mut memory),
            Input::Key(index) => Self::row(&mut self.keys, index, source, &mut memory),
            _ => self.rules.insert(input, source, &mut memory).map(|_| ()),
        };
        self.live = memory.admitted_bytes();
        result
    }

    fn row(
        rows: &mut Vec<Option<Source>>,
        index: usize,
        source: Source,
        memory: &mut Memory<'_, Resident>,
    ) -> Result<(), StorageError> {
        let required = index.checked_add(1).ok_or(StorageError::SizeOverflow)?;
        if required > rows.capacity() {
            let capacity = purrdf_lex::allocation::growth_capacity::<Option<Source>>(
                rows.capacity(),
                required,
            )?;
            memory.reserve(rows, capacity)?;
        }
        if rows.len() < required {
            rows.resize(required, None);
        }
        rows[index] = Some(source);
        Ok(())
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
        let baseline = self.equalities.len();
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
                self.equalities.truncate(baseline);
                return Err(original);
            }
        }
        Ok(())
    }
}
