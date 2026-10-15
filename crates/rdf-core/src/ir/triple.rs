// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Explicit graph-free resident RDF 1.2 storage.

use std::cmp::Ordering;
use std::sync::OnceLock;

use purrdf_lex::allocation::{Memory, Resident, StorageError};

use super::builder::LocalCapacity;
use super::dataset::{QuadHandle, TermRef, build_ordinal_permutation};
use super::frozen_terms::FrozenTerms;
use super::term::{TermId, TermValue};
use crate::{RdfLocation, RdfStoreCapabilities};

mod read;
pub use read::{TripleRead, TripleReadError, TripleReservation};

/// An ID-native triple. There is no graph field or graph sentinel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TripleIds<Id = TermId> {
    /// Subject, or reifier for an annotation row.
    pub s: Id,
    /// Predicate.
    pub p: Id,
    /// Object.
    pub o: Id,
}

/// A graph-free RDF 1.2 reifier binding.
pub type TripleReifierRow = (TermId, TermId);

const _: () = assert!(size_of::<TripleIds>() == 12);
const _: () = assert!(size_of::<TripleReifierRow>() == 8);

/// Actual retained row and lazy-index payload bytes, excluding the shared term
/// dictionary and fixed container controls. Each count reads its physical slice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TripleStorageLayout {
    /// Asserted SPO rows, exactly twelve bytes per row.
    pub triples: usize,
    /// Reifier bindings, exactly eight bytes per row.
    pub reifiers: usize,
    /// Annotation SPO rows, exactly twelve bytes per row.
    pub annotations: usize,
    /// Asserted POS and OSP ordinal payloads; zero until each is materialized.
    pub triple_orders: [usize; 2],
    /// Annotation POS and OSP ordinal payloads.
    pub annotation_orders: [usize; 2],
    /// Reifier bindings ordered by target, zero until first target probe.
    pub reifier_targets: usize,
}

fn cached_permutation<'a, T>(
    cell: &'a OnceLock<Box<[u32]>>,
    rows: &[T],
    compare: impl FnMut(&T, &T) -> Ordering,
) -> Result<&'a [u32], StorageError> {
    if let Some(ordinals) = cell.get() {
        return Ok(ordinals);
    }
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    let ordinals = build_ordinal_permutation(
        rows,
        LocalCapacity::Native,
        &mut memory,
        Memory::reserve,
        Memory::boxed_slice,
        compare,
    )?;
    // A competing successful publication owns the same deterministic order.
    // Refusal never poisons the immutable payload's cache.
    let _ = cell.set(ordinals);
    Ok(cell
        .get()
        .expect("a successful permutation publication initialized the cell"))
}

fn permutation_bytes(cell: &OnceLock<Box<[u32]>>) -> usize {
    cell.get().map_or(0, |rows| size_of_val(rows.as_ref()))
}

/// The three physical access orders of a graph-free table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TripleOrder {
    /// The stored subject/predicate/object order; no permutation allocation.
    Spo,
    /// Predicate/object/subject, held as lazy row ordinals.
    Pos,
    /// Object/subject/predicate, held as lazy row ordinals.
    Osp,
}

impl TripleOrder {
    fn ordered<T>(self, [s, p, o]: [T; 3]) -> [T; 3] {
        match self {
            Self::Spo => [s, p, o],
            Self::Pos => [p, o, s],
            Self::Osp => [o, s, p],
        }
    }

    fn selected(bound: [bool; 3]) -> Self {
        match bound {
            [_, false, true] => Self::Osp,
            [false, true, _] => Self::Pos,
            _ => Self::Spo,
        }
    }
}

/// Immutable compact rows and their two non-identity permutations. The same
/// physical home serves asserted triples and annotation triples.
#[derive(Debug)]
pub(super) struct TripleTable {
    pub(super) rows: Box<[TripleIds]>,
    pos: OnceLock<Box<[u32]>>,
    osp: OnceLock<Box<[u32]>>,
}

impl TripleTable {
    pub(super) fn new(rows: Box<[TripleIds]>) -> Self {
        Self {
            rows,
            pos: OnceLock::new(),
            osp: OnceLock::new(),
        }
    }

    fn permutation(&self, order: TripleOrder) -> Result<Option<&[u32]>, StorageError> {
        let cell = match order {
            TripleOrder::Spo => return Ok(None),
            TripleOrder::Pos => &self.pos,
            TripleOrder::Osp => &self.osp,
        };
        cached_permutation(cell, &self.rows, |a, b| {
            order
                .ordered([a.s, a.p, a.o])
                .cmp(&order.ordered([b.s, b.p, b.o]))
        })
        .map(Some)
    }

    fn probe(&self, pattern: [Option<TermId>; 3]) -> Result<TripleRows<'_>, StorageError> {
        self.probe_ordered(
            TripleOrder::selected(pattern.map(|value| value.is_some())),
            pattern,
        )
    }

    fn probe_ordered(
        &self,
        order: TripleOrder,
        pattern: [Option<TermId>; 3],
    ) -> Result<TripleRows<'_>, StorageError> {
        let ordinals = self.permutation(order)?;
        let ordered = order.ordered(pattern);
        let prefix = ordered.iter().take_while(|value| value.is_some()).count();
        let compare = |row: TripleIds| {
            for (actual, expected) in order
                .ordered([row.s, row.p, row.o])
                .into_iter()
                .zip(ordered)
                .take(prefix)
            {
                if let Some(expected) = expected {
                    let result = actual.cmp(&expected);
                    if result != Ordering::Equal {
                        return result;
                    }
                }
            }
            Ordering::Equal
        };
        let (lower, upper) = match ordinals {
            Some(ordinals) => (
                ordinals
                    .partition_point(|&index| compare(self.rows[index as usize]) == Ordering::Less),
                ordinals.partition_point(|&index| {
                    compare(self.rows[index as usize]) != Ordering::Greater
                }),
            ),
            None => (
                self.rows
                    .partition_point(|&row| compare(row) == Ordering::Less),
                self.rows
                    .partition_point(|&row| compare(row) != Ordering::Greater),
            ),
        };
        Ok(TripleRows {
            rows: &self.rows,
            ordinals,
            next: lower,
            end: upper,
            pattern,
        })
    }
}

/// Borrowed ID-native matching rows, with no per-row lexical allocation.
#[derive(Debug)]
pub struct TripleRows<'a> {
    rows: &'a [TripleIds],
    ordinals: Option<&'a [u32]>,
    next: usize,
    end: usize,
    pattern: [Option<TermId>; 3],
}

impl Iterator for TripleRows<'_> {
    type Item = TripleIds;

    fn next(&mut self) -> Option<Self::Item> {
        while self.next < self.end {
            let ordinal = self
                .ordinals
                .map_or(self.next, |values| values[self.next] as usize);
            self.next += 1;
            let row = self.rows[ordinal];
            if [row.s, row.p, row.o]
                .into_iter()
                .zip(self.pattern)
                .all(|(actual, expected)| expected.is_none_or(|expected| actual == expected))
            {
                return Some(row);
            }
        }
        None
    }
}

/// Frozen, graph-free RDF 1.2 payload. Its operation-local read session, rather
/// than the payload, implements the query view protocol.
#[derive(Debug)]
pub struct TripleDataset {
    pub(super) dictionary: FrozenTerms,
    pub(super) triples: TripleTable,
    pub(super) reifiers: Box<[TripleReifierRow]>,
    pub(super) reifier_targets: OnceLock<Box<[u32]>>,
    pub(super) annotations: TripleTable,
    pub(super) locations: Box<[(QuadHandle, RdfLocation)]>,
    pub(super) caps: RdfStoreCapabilities,
}

impl TripleDataset {
    /// Inspect concrete immutable row and currently populated permutation slices.
    #[must_use]
    pub fn storage_layout(&self) -> TripleStorageLayout {
        TripleStorageLayout {
            triples: size_of_val(self.triples.rows.as_ref()),
            reifiers: size_of_val(self.reifiers.as_ref()),
            annotations: size_of_val(self.annotations.rows.as_ref()),
            triple_orders: [
                permutation_bytes(&self.triples.pos),
                permutation_bytes(&self.triples.osp),
            ],
            annotation_orders: [
                permutation_bytes(&self.annotations.pos),
                permutation_bytes(&self.annotations.osp),
            ],
            reifier_targets: permutation_bytes(&self.reifier_targets),
        }
    }
    /// Number of distinct asserted triples, excluding the statement layer.
    #[must_use]
    pub fn len(&self) -> usize {
        self.triples.rows.len()
    }

    /// Whether the asserted table is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.triples.rows.is_empty()
    }

    /// Number of terms in the original local namespace.
    #[must_use]
    pub fn term_count(&self) -> usize {
        self.dictionary.terms.len()
    }

    /// Resolve one validated local term without owning its lexical bytes.
    #[must_use]
    pub fn resolve(&self, id: TermId) -> TermRef<'_> {
        self.dictionary.resolve(id)
    }

    /// Reverse lookup in the original store-once dictionary.
    #[must_use]
    pub fn term_id_by_value(&self, value: &TermValue) -> Option<TermId> {
        self.dictionary.term_id_by_value(value)
    }

    /// The original caller-selected content-recognition scheme.
    #[must_use]
    pub fn content_id_scheme(&self) -> Option<&crate::ContentIdScheme> {
        self.dictionary.content_scheme.as_ref()
    }

    /// Derived content IDs in deterministic local-ID order.
    pub fn content_ids(&self) -> impl Iterator<Item = (TermId, crate::Blake3ContentId)> + '_ {
        self.dictionary.content_ids()
    }

    /// Original configured derivation predicate, even if never interned.
    #[must_use]
    pub fn derivation_predicate_iri(&self) -> Option<&str> {
        self.dictionary.derivation_predicate_iri.as_deref()
    }

    /// Every asserted row in physical SPO order.
    pub fn triples(&self) -> impl ExactSizeIterator<Item = TripleIds> + '_ {
        self.triples.rows.iter().copied()
    }

    /// Match through the three real graph-free access orders.
    ///
    /// # Errors
    /// Returns physical allocation or typed ordinal-capacity refusal.
    pub fn triples_for_pattern(
        &self,
        s: Option<TermId>,
        p: Option<TermId>,
        o: Option<TermId>,
    ) -> Result<TripleRows<'_>, StorageError> {
        self.triples.probe([s, p, o])
    }

    /// Every reifier/quoted-triple binding, with no graph slot.
    pub fn reifiers(&self) -> impl ExactSizeIterator<Item = TripleReifierRow> + '_ {
        self.reifiers.iter().copied()
    }

    /// Match a binding using the physical reifier order or lazy target order.
    ///
    /// # Errors
    /// Returns the original ordinal capacity or allocation refusal.
    pub fn reifiers_for_pattern(
        &self,
        reifier: Option<TermId>,
        target: Option<TermId>,
    ) -> Result<impl Iterator<Item = TripleReifierRow> + '_, StorageError> {
        let ordinals = if reifier.is_none() && target.is_some() {
            Some(cached_permutation(
                &self.reifier_targets,
                &self.reifiers,
                |a, b| (a.1, a.0).cmp(&(b.1, b.0)),
            )?)
        } else {
            None
        };
        let at = move |index: usize| {
            self.reifiers[ordinals.map_or(index, |order| order[index] as usize)]
        };
        let bound = reifier.or(target);
        let (start, end) = match (bound, ordinals) {
            (None, _) => (0, self.reifiers.len()),
            (Some(bound), None) => (
                self.reifiers.partition_point(|row| row.0 < bound),
                self.reifiers.partition_point(|row| row.0 <= bound),
            ),
            (Some(bound), Some(order)) => (
                order.partition_point(|&index| self.reifiers[index as usize].1 < bound),
                order.partition_point(|&index| self.reifiers[index as usize].1 <= bound),
            ),
        };
        Ok((start..end).map(at).filter(move |&(r, t)| {
            reifier.is_none_or(|bound| bound == r) && target.is_none_or(|bound| bound == t)
        }))
    }

    /// Every annotation as `(reifier, predicate, object)`.
    pub fn annotations(&self) -> impl ExactSizeIterator<Item = TripleIds> + '_ {
        self.annotations.rows.iter().copied()
    }

    /// Match annotations through their actual three compact access orders.
    ///
    /// # Errors
    /// Returns the original ordinal capacity or allocation refusal.
    pub fn annotations_for_pattern(
        &self,
        reifier: Option<TermId>,
        predicate: Option<TermId>,
        object: Option<TermId>,
    ) -> Result<TripleRows<'_>, StorageError> {
        self.annotations.probe([reifier, predicate, object])
    }

    super::dataset::source_location_method! {
        /// The original sparse source location for a frozen triple ordinal.
        #[must_use]
    }

    /// Observed RDF features; named graphs are structurally absent.
    #[must_use]
    pub const fn capabilities(&self) -> RdfStoreCapabilities {
        self.caps
    }
}
