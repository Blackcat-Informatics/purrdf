// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Explicit graph-free resident RDF 1.2 storage.

use std::cmp::Ordering;
use std::ops::Range;
use std::sync::OnceLock;

use purrdf_lex::allocation::StorageError;

use super::builder::LocalCapacity;
use super::dataset::{QuadHandle, TermRef, cached_permutation};
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
    /// Component-mask ordinal payloads in mask order (S=1, P=2, O=4).
    pub reifier_components: [usize; 7],
}

fn permutation_bytes(cell: &OnceLock<Box<[u32]>>) -> usize {
    cell.get().map_or(0, |rows| size_of_val(rows.as_ref()))
}

pub(super) fn component_mask(pattern: [Option<TermId>; 3]) -> usize {
    usize::from(pattern[0].is_some())
        | (usize::from(pattern[1].is_some()) << 1)
        | (usize::from(pattern[2].is_some()) << 2)
}

fn masked_key(components: [TermId; 3], mask: usize) -> [Option<TermId>; 3] {
    std::array::from_fn(|index| (mask & (1 << index) != 0).then_some(components[index]))
}

pub(super) fn compare_masked<T: Ord>(
    a: &T,
    b: &T,
    mask: usize,
    components: &impl Fn(&T) -> [TermId; 3],
) -> Ordering {
    masked_key(components(a), mask)
        .cmp(&masked_key(components(b), mask))
        .then_with(|| a.cmp(b))
}

/// One mask order preserves the frozen row order within every matching key.
pub(super) fn masked_permutation<'a, T: Ord>(
    cell: &'a OnceLock<Box<[u32]>>,
    rows: &[T],
    capacity: LocalCapacity,
    mask: usize,
    components: impl Fn(&T) -> [TermId; 3],
) -> Result<&'a [u32], StorageError> {
    cached_permutation(cell, rows, capacity, |a, b| {
        compare_masked(a, b, mask, &components)
    })
}

/// Borrowed physical rows addressed by an identity or ordinal range.
#[derive(Debug)]
pub(super) struct OrdinalRows<'a, T> {
    rows: &'a [T],
    ordinals: Option<&'a [u32]>,
    range: Range<usize>,
}

impl<T: Copy> Iterator for OrdinalRows<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.range
            .next()
            .map(|index| self.rows[self.ordinals.map_or(index, |order| order[index] as usize)])
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.range.size_hint()
    }
}

impl<T: Copy> ExactSizeIterator for OrdinalRows<'_, T> {}

pub(super) fn ordinal_range<'a, T>(
    rows: &'a [T],
    ordinals: Option<&'a [u32]>,
    compare: impl Fn(&T) -> Ordering,
) -> OrdinalRows<'a, T> {
    let (lower, upper) = match ordinals {
        Some(order) => (
            order.partition_point(|&index| compare(&rows[index as usize]) == Ordering::Less),
            order.partition_point(|&index| compare(&rows[index as usize]) != Ordering::Greater),
        ),
        None => (
            rows.partition_point(|row| compare(row) == Ordering::Less),
            rows.partition_point(|row| compare(row) != Ordering::Greater),
        ),
    };
    OrdinalRows {
        rows,
        ordinals,
        range: lower..upper,
    }
}

pub(super) fn masked_rows<'a, T: Copy + Ord>(
    rows: &'a [T],
    cell: Option<&'a OnceLock<Box<[u32]>>>,
    capacity: LocalCapacity,
    pattern: [Option<TermId>; 3],
    components: impl Fn(&T) -> [TermId; 3],
) -> Result<OrdinalRows<'a, T>, StorageError> {
    let mask = component_mask(pattern);
    let ordinals = cell
        .map(|cell| masked_permutation(cell, rows, capacity, mask, &components))
        .transpose()?;
    Ok(ordinal_range(rows, ordinals, |row| {
        masked_key(components(row), mask).cmp(&pattern)
    }))
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
        cached_permutation(cell, &self.rows, LocalCapacity::Native, |a, b| {
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
        let compare = |row: &TripleIds| {
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
        Ok(TripleRows {
            rows: ordinal_range(&self.rows, ordinals, compare),
            pattern,
        })
    }
}

/// Borrowed ID-native matching rows, with no per-row lexical allocation.
#[derive(Debug)]
pub struct TripleRows<'a> {
    rows: OrdinalRows<'a, TripleIds>,
    pattern: [Option<TermId>; 3],
}

impl Iterator for TripleRows<'_> {
    type Item = TripleIds;

    fn next(&mut self) -> Option<Self::Item> {
        let pattern = self.pattern;
        self.rows.find(|row| {
            [row.s, row.p, row.o]
                .into_iter()
                .zip(pattern)
                .all(|(actual, expected)| expected.is_none_or(|expected| actual == expected))
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.rows.len()))
    }
}

/// Frozen, graph-free RDF 1.2 payload. Its operation-local read session, rather
/// than the payload, implements the query view protocol.
#[derive(Debug)]
pub struct TripleDataset {
    pub(super) dictionary: FrozenTerms,
    pub(super) statistics: super::statistics::LocalStatistics,
    pub(super) snapshot_identity: purrdf_hash::hex::Digest32,
    pub(super) triples: TripleTable,
    pub(super) reifiers: Box<[TripleReifierRow]>,
    pub(super) reifier_targets: OnceLock<Box<[u32]>>,
    pub(super) reifier_components: [OnceLock<Box<[u32]>>; 7],
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
            reifier_components: self.reifier_components.each_ref().map(permutation_bytes),
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
                LocalCapacity::Native,
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

    /// Bindings whose target's components match the supplied local IDs.
    /// Matching bindings retain frozen reifier order without lexical allocation.
    ///
    /// # Errors
    /// Returns typed ordinal-capacity or physical allocation refusal.
    pub fn reifiers_for_components(
        &self,
        pattern: [Option<TermId>; 3],
    ) -> Result<impl ExactSizeIterator<Item = TripleReifierRow> + '_, StorageError> {
        let mask = component_mask(pattern);
        masked_rows(
            &self.reifiers,
            mask.checked_sub(1)
                .map(|index| &self.reifier_components[index]),
            LocalCapacity::Native,
            pattern,
            |row| self.dictionary.triple_components(row.1),
        )
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
