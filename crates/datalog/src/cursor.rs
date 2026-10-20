// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The arrangement's native cursors: a zero-allocation lending cursor over a
//! relation's shared arrangement, and a globally value-ordered trie cursor over the
//! same runs.
//!
//! # What a [`RowCursor`] yields, and in what order
//!
//! The per-atom scan of a semi-naive join never materialises a
//! `Vec<(TermId, TermId, RowId)>`. A [`RowCursor`] yields the
//! `(subject_id, object_id, row_id)` id rows selected by a [`Bound`] one at a time,
//! borrowing the relation's columns. It CONCATENATES each batch's bound-run — located
//! by GALLOPING the sorted `(subject_id, object_id)` columns (subject-bound: gallop the
//! `subj` column to the term's contiguous run; object-bound: walk the lazily-built
//! `(object, subject)` permutation) — followed by a linear scan of the small tail.
//!
//! # Order-freedom (why batch-then-tail is sound)
//!
//! Enumeration is batch-then-tail, NOT a global merge sort across runs. A cursor's
//! order is an INTERNAL storage order: the fixpoint's per-round commit re-sorts the
//! derived rows lexically before anything is emitted or charged, so two runs that
//! enumerate the same rows in different orders still produce byte-identical output.
//! The cursor therefore never introduces an observable ordering, and never has to pay
//! for a merge it does not need.
//!
//! # Galloping is the primitive
//!
//! Run location is a galloping lower bound over the sorted columns — an exponential
//! probe, then a binary search inside the bracket; never a linear scan and never a hash
//! probe. That is the exact primitive a multiway leapfrog / worst-case-optimal join
//! composes, which is what [`ValueCursor`] exposes: the same sorted runs viewed as one
//! globally value-ordered stream with a `seek`. The `(subject_id, object_id)` key is
//! unique per relation, so a [`Bound::Both`] yields at most one row across the whole
//! arrangement.

use core::cmp::{Ordering, Reverse};
use core::fmt;
use std::collections::BinaryHeap;

use crate::dictionary::TermIdentity;
use crate::id::{RowId, TermId};
use crate::store::{Batch, Bound, Relation, Tail};

/// The sealing supertrait: only types in THIS module can implement
/// [`LendingIterator`], so the cursor contract cannot be re-implemented — or its `next`
/// invariant weakened — from outside the crate.
mod sealed {
    pub trait Sealed {}

    impl<I: super::TermIdentity> Sealed for super::RowCursor<'_, I> {}
}

/// A lending iterator over a relation's id rows: it yields borrowed views tied to the
/// `&mut self` borrow (through the generic associated type `Item<'a>`), so a driver
/// never collects a `Vec`.
///
/// Sealed (only [`RowCursor`] implements it). The single method is the whole contract:
/// advance and yield over the galloped runs.
pub trait LendingIterator: sealed::Sealed {
    /// The lent item — a `(subject_id, object_id, row_id)` id row. All three are `Copy`
    /// niche integers, so the item borrows nothing beyond the cursor's own reborrow and
    /// yielding it copies three words rather than cloning a term.
    type Item<'a>
    where
        Self: 'a;

    /// Yield the id row at the cursor and advance past it, or `None` at the end.
    fn next(&mut self) -> Option<Self::Item<'_>>;
}

/// The within-source iteration state for the current arrangement leg.
#[derive(Debug, Clone, Copy)]
enum Inner {
    /// A contiguous column-position range `[pos, end)` of the current batch — an `Any`
    /// full scan (`0..len`) or a subject-bound run.
    Range {
        /// The next column position to yield.
        pos: usize,
        /// One past the last column position of the run.
        end: usize,
    },
    /// An absolute range in the current batch's original object permutation.
    Perm {
        /// The next index into the original permutation.
        pos: usize,
        /// One past the last selected permutation index.
        end: usize,
    },
    /// At most one column position (a `Both` bound over the unique `(s, o)` key).
    One(Option<usize>),
    /// A linear scan of the relation's tail from index `pos`, filtered by the bound.
    Tail {
        /// The next tail index to test.
        pos: usize,
    },
    /// No more rows.
    Done,
}

/// A lending cursor over one relation's shared arrangement, in batch-then-tail order.
///
/// Borrows the relation for the cursor's lifetime; every yielded row is resolved
/// through the batch columns or the tail, so nothing is cloned or allocated per row.
/// The bound shape drives the per-source run.
pub struct RowCursor<'a, I: TermIdentity = TermId> {
    /// The arrangement being scanned.
    rel: &'a Relation<I>,
    /// Owned indices into that same arrangement; no retained slice borrows.
    position: RowPosition<I>,
}

/// The native row scan's owned position, retained between source borrows.
///
/// The enclosing execution state must keep the original arrangement unchanged.
/// This is crate-private: it is not a token callers may attach to another source.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RowPosition<I: TermIdentity = TermId> {
    /// The selection this cursor enumerates.
    bound: Bound<I>,
    /// The current arrangement leg: `0..batches.len()` selects that batch, an index
    /// equal to `batches.len()` selects the tail, and anything greater is exhausted.
    src: usize,
    /// The within-source state of the current leg.
    inner: Inner,
}

impl<I: TermIdentity> fmt::Debug for RowCursor<'_, I> {
    /// Prints the cursor's position, never the borrowed columns: a cursor is a
    /// position, and dumping a relation's whole arrangement into a diagnostic would be
    /// both enormous and useless.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let leg = match self.position.inner {
            Inner::Range { .. } => "range",
            Inner::Perm { .. } => "perm",
            Inner::One(_) => "one",
            Inner::Tail { .. } => "tail",
            Inner::Done => "done",
        };
        f.debug_struct("RowCursor")
            .field("bound", &self.position.bound)
            .field("source", &self.position.src)
            .field("leg", &leg)
            .finish()
    }
}

impl<'a, I: TermIdentity> RowCursor<'a, I> {
    /// A cursor over the rows of `rel` selected by `bound`, positioned at the first
    /// source (batch 0, or the tail, or exhausted).
    pub(crate) fn new(rel: &'a Relation<I>, bound: Bound<I>) -> Self {
        Self {
            rel,
            position: RowPosition::new(rel, bound),
        }
    }

    /// Whether the cursor has at least one remaining row, without materializing
    /// an intermediate collection.
    pub fn any_remaining(mut self) -> bool {
        self.next().is_some()
    }

    /// Count the remaining matches from indexed run lengths and the bounded tail.
    /// Sealed rows are never enumerated and no result buffer is allocated.
    pub(crate) fn remaining_count(self) -> usize {
        self.position.remaining_count(self.rel)
    }
}

impl<I: TermIdentity> RowPosition<I> {
    /// Start at the first original batch, then its bounded mutable tail.
    pub(crate) fn new(rel: &Relation<I>, bound: Bound<I>) -> Self {
        let mut cursor = Self {
            bound,
            src: 0,
            inner: Inner::Done,
        };
        cursor.enter(rel, 0);
        cursor
    }

    /// Position the cursor at source `src`, computing its bound-run. A batch source
    /// gallops the sorted columns for the run; the tail source is a linear scan; past
    /// the tail is exhausted.
    fn enter(&mut self, rel: &Relation<I>, src: usize) {
        self.src = src;
        let batches = rel.batches();
        self.inner = match src.cmp(&batches.len()) {
            Ordering::Less => {
                let b: &Batch<(), I> = &batches[src];
                match self.bound {
                    Bound::Any => Inner::Range {
                        pos: 0,
                        end: b.len(),
                    },
                    Bound::Subject(s) => {
                        let (lo, hi) = b.subject_run(s);
                        Inner::Range { pos: lo, end: hi }
                    }
                    Bound::Object(o) => {
                        let (pos, end) = b.object_run(o);
                        Inner::Perm { pos, end }
                    }
                    Bound::Both(s, o) => Inner::One(b.both_pos(s, o)),
                }
            }
            Ordering::Equal => Inner::Tail { pos: 0 },
            Ordering::Greater => Inner::Done,
        };
    }

    /// Whether the tail row `(s, o)` satisfies `bound`.
    #[inline]
    fn tail_matches(bound: Bound<I>, s: I, o: I) -> bool {
        match bound {
            Bound::Any => true,
            Bound::Subject(bs) => s == bs,
            Bound::Object(bo) => o == bo,
            Bound::Both(bs, bo) => s == bs && o == bo,
        }
    }

    /// Count the remaining matches from indexed run lengths and the bounded tail.
    /// Sealed rows are never enumerated and no result buffer is allocated.
    fn remaining_count(mut self, rel: &Relation<I>) -> usize {
        let mut count = 0;
        loop {
            count += match &self.inner {
                Inner::Range { pos, end } | Inner::Perm { pos, end } => end - pos,
                Inner::One(position) => usize::from(position.is_some()),
                Inner::Tail { pos } => rel
                    .tail()
                    .iter()
                    .skip(*pos)
                    .filter(|&(subject, object, _)| Self::tail_matches(self.bound, subject, object))
                    .count(),
                Inner::Done => return count,
            };
            self.enter(rel, self.src + 1);
        }
    }

    /// Continue directly from the retained indices in the original arrangement.
    pub(crate) fn next(&mut self, rel: &Relation<I>) -> Option<(I, I, RowId)> {
        let bound = self.bound;
        loop {
            match &mut self.inner {
                Inner::Range { pos, end } => {
                    if *pos < *end {
                        let p = *pos;
                        *pos += 1;
                        return Some(rel.batches()[self.src].row_at(p));
                    }
                }
                Inner::Perm { pos, end } => {
                    if *pos < *end {
                        let p = rel.batches()[self.src].object_order()[*pos];
                        *pos += 1;
                        return Some(rel.batches()[self.src].row_at(p));
                    }
                }
                Inner::One(slot) => {
                    if let Some(p) = slot.take() {
                        return Some(rel.batches()[self.src].row_at(p));
                    }
                }
                Inner::Tail { pos } => {
                    let tail = rel.tail();
                    while *pos < tail.len() {
                        let (s, o, r) = tail.row(*pos);
                        *pos += 1;
                        if Self::tail_matches(bound, s, o) {
                            return Some((s, o, r));
                        }
                    }
                }
                Inner::Done => return None,
            }
            // The current source is exhausted; advance to the next.
            let next_src = self.src + 1;
            self.enter(rel, next_src);
        }
    }
}

impl<I: TermIdentity> LendingIterator for RowCursor<'_, I> {
    type Item<'a>
        = (I, I, RowId)
    where
        Self: 'a;

    fn next(&mut self) -> Option<Self::Item<'_>> {
        self.position.next(self.rel)
    }
}

// ── Value-ordered trie cursor ───────────────────────────────────────────────────

/// Trie adornment: order by the SUBJECT column. See [`ValueCursor`].
pub const VALUE_SUBJECT: u8 = 0;
/// Trie adornment: order by the OBJECT column. See [`ValueCursor`].
pub const VALUE_OBJECT: u8 = 1;

/// One sorted arrangement run viewed in the requested trie orientation.
#[derive(Debug)]
enum OrderedSource {
    /// A contiguous primary-column range: a whole subject-major batch, or one
    /// subject-bound object run.
    Range {
        /// The original batch's position in the unchanged arrangement.
        batch: usize,
        /// The next column position.
        pos: usize,
        /// One past the last column position.
        end: usize,
    },
    /// A secondary-order permutation: a whole object-major batch, or one object-bound
    /// subject run.
    Perm {
        /// The original batch's position in the unchanged arrangement.
        batch: usize,
        /// The next absolute index into its original object permutation.
        pos: usize,
        /// One past the last selected permutation index.
        end: usize,
    },
    /// The mutable tail, which is bounded below the seal threshold. Its matching
    /// positions are sorted ONCE per cursor; the rows themselves are never copied.
    Tail {
        /// The matching tail indices, in projected `(value, row)` order.
        positions: Box<[u32]>,
        /// The next index into `positions`.
        pos: usize,
    },
}

impl OrderedSource {
    /// The column this orientation orders by. Resolved at monomorphisation, never
    /// re-branched per tuple.
    #[inline]
    fn project<const COLUMN: u8, I: TermIdentity>(subject: I, object: I) -> I {
        match COLUMN {
            VALUE_SUBJECT => subject,
            VALUE_OBJECT => object,
            _ => unreachable!("COLUMN is VALUE_SUBJECT or VALUE_OBJECT"),
        }
    }

    /// Whether the row's OTHER column equals the fixed value.
    #[inline]
    fn other_matches<const COLUMN: u8, I: TermIdentity>(other: I, subject: I, object: I) -> bool {
        match COLUMN {
            VALUE_SUBJECT => object == other,
            VALUE_OBJECT => subject == other,
            _ => unreachable!("COLUMN is VALUE_SUBJECT or VALUE_OBJECT"),
        }
    }

    /// The projected `(value, row)` pair at a batch column position.
    #[inline]
    fn batch_row<const COLUMN: u8, I: TermIdentity>(
        batch: &Batch<(), I>,
        position: usize,
    ) -> (I, RowId) {
        let (subject, object, row) = batch.row_at(position);
        (Self::project::<COLUMN, I>(subject, object), row)
    }

    /// The projected `(value, row)` pair at a tail index.
    #[inline]
    fn tail_row<const COLUMN: u8, I: TermIdentity>(tail: &Tail<I>, position: usize) -> (I, RowId) {
        let (subject, object, row) = tail.row(position);
        (Self::project::<COLUMN, I>(subject, object), row)
    }

    /// The run's current pair, without advancing.
    fn peek<const COLUMN: u8, I: TermIdentity>(&self, rel: &Relation<I>) -> Option<(I, RowId)> {
        match self {
            Self::Range { batch, pos, end } => {
                (*pos < *end).then(|| Self::batch_row::<COLUMN, I>(&rel.batches()[*batch], *pos))
            }
            Self::Perm { batch, pos, end } => (*pos < *end).then(|| {
                let batch = &rel.batches()[*batch];
                Self::batch_row::<COLUMN, I>(batch, batch.object_order()[*pos])
            }),
            Self::Tail { positions, pos } => positions
                .get(*pos)
                .map(|&position| Self::tail_row::<COLUMN, I>(rel.tail(), position as usize)),
        }
    }

    /// The run's current pair, advancing past it.
    fn pop<const COLUMN: u8, I: TermIdentity>(&mut self, rel: &Relation<I>) -> Option<(I, RowId)> {
        let row = self.peek::<COLUMN, I>(rel)?;
        match self {
            Self::Range { pos, .. } | Self::Perm { pos, .. } | Self::Tail { pos, .. } => {
                *pos += 1;
            }
        }
        Some(row)
    }

    /// Seek this sorted run to its first projected value `>= target`.
    fn seek<const COLUMN: u8, I: TermIdentity>(&mut self, rel: &Relation<I>, target: I) {
        match self {
            Self::Range { batch, pos, end } => {
                let batch = &rel.batches()[*batch];
                let (mut lo, mut hi) = (*pos, *end);
                while lo < hi {
                    let mid = lo + (hi - lo) / 2;
                    if Self::batch_row::<COLUMN, I>(batch, mid).0 < target {
                        lo = mid + 1;
                    } else {
                        hi = mid;
                    }
                }
                *pos = lo;
            }
            Self::Perm { batch, pos, end } => {
                let batch = &rel.batches()[*batch];
                let offset = batch.object_order()[*pos..*end].partition_point(|&position| {
                    Self::batch_row::<COLUMN, I>(batch, position).0 < target
                });
                *pos += offset;
            }
            Self::Tail { positions, pos } => {
                let offset = positions[*pos..].partition_point(|&position| {
                    Self::tail_row::<COLUMN, I>(rel.tail(), position as usize).0 < target
                });
                *pos += offset;
            }
        }
    }
}

/// A globally value-ordered cursor over one relation trie level.
///
/// Immutable batches already carry the two orders this needs: `(subject, object)` in
/// their primary columns and `(object, subject)` in the lazy permutation. This cursor
/// k-way merges those sorted runs plus the bounded tail, yielding projected
/// `(I, RowId)` pairs without materialising or re-sorting relation rows, and
/// supports the [`seek`](Self::seek) a leapfrog intersection drives.
///
/// The order is the identity carrier's `Ord`, shared by every relation of one
/// dictionary. Native [`TermId`] uses mint order; host identities retain their own
/// internal order. Neither certifies lexical or value order. Equal values retain
/// arrangement order, not global row-id order. This is never an emission order.
pub struct ValueCursor<'a, const COLUMN: u8, I: TermIdentity = TermId> {
    /// The original unchanged arrangement.
    rel: &'a Relation<I>,
    /// Owned run indices, tail selection and merge heap.
    position: ValuePosition<COLUMN, I>,
}

/// Owned traversal state for a trie level, without a self-referential source.
/// The enclosing execution retains the unchanged original arrangement.
#[derive(Debug)]
pub(crate) struct ValuePosition<const COLUMN: u8, I: TermIdentity = TermId> {
    /// The non-exhausted sorted runs, in arrangement order.
    sources: Vec<OrderedSource>,
    /// One current row from every non-exhausted source. `Reverse` turns the standard
    /// max-heap into a min-frontier; the source index in the key makes the pop order
    /// total, so equal `(value, row)` pairs cannot tie.
    frontier: BinaryHeap<Reverse<(I, RowId, usize)>>,
}

impl<const COLUMN: u8, I: TermIdentity> fmt::Debug for ValueCursor<'_, COLUMN, I> {
    /// Prints the merge shape, never the borrowed columns.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ValueCursor")
            .field("column", &COLUMN)
            .field("sources", &self.position.sources.len())
            .field("frontier", &self.position.frontier.len())
            .finish()
    }
}

impl<'a, const COLUMN: u8, I: TermIdentity> ValueCursor<'a, COLUMN, I> {
    /// A cursor over `rel`'s trie level in this orientation, optionally fixing the
    /// OTHER column to `other`.
    pub(crate) fn new(rel: &'a Relation<I>, other: Option<I>) -> Self {
        Self {
            rel,
            position: ValuePosition::new(rel, other),
        }
    }

    /// Seek every sorted run to its first value `>= target`.
    pub fn seek(&mut self, target: I) {
        self.position.seek(self.rel, target);
    }
}

impl<const COLUMN: u8, I: TermIdentity> ValuePosition<COLUMN, I> {
    /// Build the existing merge frontier once against the original arrangement.
    pub(crate) fn new(rel: &Relation<I>, other: Option<I>) -> Self {
        let mut sources = Vec::with_capacity(rel.batches().len() + 1);
        for (index, batch) in rel.batches().iter().enumerate() {
            let source = match (COLUMN, other) {
                (VALUE_SUBJECT, None) => OrderedSource::Range {
                    batch: index,
                    pos: 0,
                    end: batch.len(),
                },
                (VALUE_SUBJECT, Some(object)) => {
                    let (pos, end) = batch.object_run(object);
                    OrderedSource::Perm {
                        batch: index,
                        pos,
                        end,
                    }
                }
                (VALUE_OBJECT, None) => OrderedSource::Perm {
                    batch: index,
                    pos: 0,
                    end: batch.object_order().len(),
                },
                (VALUE_OBJECT, Some(subject)) => {
                    let (lo, hi) = batch.subject_run(subject);
                    OrderedSource::Range {
                        batch: index,
                        pos: lo,
                        end: hi,
                    }
                }
                _ => unreachable!("COLUMN is VALUE_SUBJECT or VALUE_OBJECT"),
            };
            if source.peek::<COLUMN, I>(rel).is_some() {
                sources.push(source);
            }
        }

        if !rel.tail().is_empty() {
            let mut positions: Vec<u32> = rel
                .tail()
                .iter()
                .enumerate()
                .filter_map(|(position, (subject, object, _))| {
                    other
                        .is_none_or(|bound| Self::other_matches(bound, subject, object))
                        .then_some(position as u32)
                })
                .collect();
            positions.sort_unstable_by_key(|&position| {
                let (subject, object, row) = rel.tail().row(position as usize);
                (Self::project(subject, object), row)
            });
            if !positions.is_empty() {
                sources.push(OrderedSource::Tail {
                    positions: positions.into_boxed_slice(),
                    pos: 0,
                });
            }
        }

        let mut cursor = Self {
            sources,
            frontier: BinaryHeap::new(),
        };
        cursor.rebuild_frontier(rel);
        cursor
    }

    /// The column this orientation orders by.
    #[inline]
    fn project(subject: I, object: I) -> I {
        OrderedSource::project::<COLUMN, I>(subject, object)
    }

    /// Whether the row's OTHER column equals the fixed value.
    #[inline]
    fn other_matches(other: I, subject: I, object: I) -> bool {
        OrderedSource::other_matches::<COLUMN, I>(other, subject, object)
    }

    /// Refill the min-frontier from every source's current pair.
    fn rebuild_frontier(&mut self, rel: &Relation<I>) {
        self.frontier.clear();
        for (source, run) in self.sources.iter().enumerate() {
            if let Some((value, row)) = run.peek::<COLUMN, I>(rel) {
                self.frontier.push(Reverse((value, row, source)));
            }
        }
    }

    /// Seek every sorted run, positioning the merged cursor at its first value
    /// `>= target`.
    pub(crate) fn seek(&mut self, rel: &Relation<I>, target: I) {
        for source in &mut self.sources {
            source.seek::<COLUMN, I>(rel, target);
        }
        self.rebuild_frontier(rel);
    }

    /// Yield a row at the globally-smallest remaining value and advance its run.
    pub(crate) fn next(&mut self, rel: &Relation<I>) -> Option<(I, RowId)> {
        let Reverse((expected_value, expected_row, source)) = self.frontier.pop()?;
        let row = self.sources[source]
            .pop::<COLUMN, I>(rel)
            .expect("a frontier entry always names a live source row");
        debug_assert_eq!(row, (expected_value, expected_row));
        if let Some((value, row)) = self.sources[source].peek::<COLUMN, I>(rel) {
            self.frontier.push(Reverse((value, row, source)));
        }
        Some(row)
    }
}

impl<const COLUMN: u8, I: TermIdentity> Iterator for ValueCursor<'_, COLUMN, I> {
    type Item = (I, RowId);

    fn next(&mut self) -> Option<Self::Item> {
        self.position.next(self.rel)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::store::{PartitionRef, RelationStore};
    use crate::test_support::permute;

    /// A predicate is an ordinary term, so its store key is its bracketed surface.
    const P: &str = "<https://example.org/p>";

    fn iri(local: &str) -> String {
        format!("<https://example.org/{local}>")
    }

    /// Drain a lending cursor into a `Vec` — a test-only convenience for asserting a
    /// cursor's full row set (the production hot path never collects).
    fn drain(mut c: RowCursor<'_>) -> Vec<(TermId, TermId, RowId)> {
        let mut out = Vec::new();
        while let Some(row) = c.next() {
            out.push(row);
        }
        out
    }

    /// Resolve selected id rows to `(subject, object)` surface pairs as an
    /// ORDER-INDEPENDENT set: the cursor enumerates batch-then-tail, not sorted, so
    /// tests compare sets — the round commit, not cursor order, fixes output.
    fn resolved_set(
        s: &RelationStore,
        rows: &[(TermId, TermId, RowId)],
    ) -> BTreeSet<(String, String)> {
        rows.iter()
            .map(|&(si, oi, _)| {
                (
                    s.interner().resolve(si).to_owned(),
                    s.interner().resolve(oi).to_owned(),
                )
            })
            .collect()
    }

    fn pair(sub: &str, obj: &str) -> (String, String) {
        (iri(sub), iri(obj))
    }

    /// The big-store tuples: `p` holds `(a, o_i)` for 200 objects plus a second subject
    /// `z` with one edge — large enough to force several batch seals, so the galloping
    /// batch runs are exercised, not just the tail leg.
    fn big_tuples() -> Vec<(String, String)> {
        let mut tuples: Vec<(String, String)> = (0..200)
            .map(|i| (iri("a"), iri(&format!("o{i:03}"))))
            .collect();
        tuples.push((iri("z"), iri("o000")));
        tuples
    }

    fn store_of(tuples: &[(String, String)]) -> RelationStore {
        let mut s = RelationStore::new();
        for (sub, obj) in tuples {
            assert!(
                s.insert(sub, P, obj, RelationStore::DEFAULT_GRAPH)
                    .unwrap()
                    .is_some(),
                "fixture rows are distinct"
            );
        }
        s
    }

    /// The fixture's single `(predicate, graph)` partition — the unit a cursor scans.
    fn partition_of<'a>(s: &'a RelationStore, predicate: &str) -> Option<PartitionRef<'a>> {
        let predicate = s.term_id(predicate)?;
        let graph = s.term_id(RelationStore::DEFAULT_GRAPH)?;
        s.partition(predicate, graph)
    }

    /// A row cursor over the fixture's partition, or an empty one when it does not exist.
    fn select<'a>(s: &'a RelationStore, predicate: &str, bound: Bound) -> RowCursor<'a> {
        s.select(predicate, RelationStore::DEFAULT_GRAPH, bound)
    }

    fn big_store() -> RelationStore {
        store_of(&big_tuples())
    }

    #[test]
    fn indexed_remaining_counts_cover_batches_tail_and_consumed_rows() {
        let store = big_store();
        let subject = store.term_id(&iri("a")).unwrap();
        let other = store.term_id(&iri("z")).unwrap();
        let object = store.term_id(&iri("o000")).unwrap();
        let missing_pair = store.term_id(&iri("o199")).unwrap();
        for (bound, expected) in [
            (Bound::Any, 201),
            (Bound::Subject(subject), 200),
            (Bound::Object(object), 2),
            (Bound::Both(subject, object), 1),
            (Bound::Both(other, missing_pair), 0),
        ] {
            assert_eq!(select(&store, P, bound).remaining_count(), expected);
            let mut cursor = select(&store, P, bound);
            let consumed = usize::from(cursor.next().is_some());
            assert_eq!(cursor.remaining_count(), expected - consumed);
        }
    }

    #[test]
    fn owned_row_positions_resume_at_every_selected_boundary() {
        let store = big_store();
        let subject = store.term_id(&iri("a")).unwrap();
        let other = store.term_id(&iri("z")).unwrap();
        let object = store.term_id(&iri("o000")).unwrap();
        let later_object = store.term_id(&iri("o005")).unwrap();
        let absent_pair = store.term_id(&iri("o199")).unwrap();
        for bound in [
            Bound::Any,
            Bound::Subject(subject),
            Bound::Object(object),
            Bound::Object(later_object),
            Bound::Object(subject),
            Bound::Both(subject, object),
            Bound::Both(other, absent_pair),
        ] {
            let expected = drain(select(&store, P, bound));
            for cut in 0..=expected.len() {
                let mut cursor = select(&store, P, bound);
                let mut prefix = Vec::new();
                for _ in 0..cut {
                    prefix.push(cursor.next().unwrap());
                }
                // Retain only the original source and integer position, then
                // attach a fresh lending borrow without seeking or skipping.
                let RowCursor { rel, position } = cursor;
                assert_eq!(position.remaining_count(rel), expected.len() - cut);
                let resumed = RowCursor { rel, position };
                prefix.extend(drain(resumed));
                assert_eq!(prefix, expected, "bound {bound:?}, cut {cut}");
            }
        }
    }

    #[test]
    fn cursor_any_yields_every_row_as_a_set() {
        let s = big_store();
        let got = resolved_set(&s, &drain(select(&s, P, Bound::Any)));
        assert_eq!(got.len(), 201, "200 a-edges + 1 z-edge, deduped");
        assert!(got.contains(&pair("a", "o000")));
        assert!(got.contains(&pair("z", "o000")));
        assert!(got.contains(&pair("a", "o199")));
    }

    #[test]
    fn cursor_subject_bound_gallops_batches() {
        let s = big_store();
        let a = s.term_id(&iri("a")).expect("a interned");
        let got = resolved_set(&s, &drain(select(&s, P, Bound::Subject(a))));
        assert_eq!(got.len(), 200, "exactly a's 200 edges");
        assert!(got.iter().all(|(sub, _)| *sub == iri("a")));

        let z = s.term_id(&iri("z")).expect("z interned");
        let zrows = resolved_set(&s, &drain(select(&s, P, Bound::Subject(z))));
        assert_eq!(zrows, [pair("z", "o000")].into());
    }

    #[test]
    fn cursor_object_bound_uses_the_lazy_permutation() {
        let s = big_store();
        let o0 = s.term_id(&iri("o000")).expect("o000 interned");
        // o000 is the object of BOTH a and z.
        let got = resolved_set(&s, &drain(select(&s, P, Bound::Object(o0))));
        assert_eq!(got, [pair("a", "o000"), pair("z", "o000")].into());
        // A distinct object appears once.
        let o5 = s.term_id(&iri("o005")).expect("o005 interned");
        let g5 = resolved_set(&s, &drain(select(&s, P, Bound::Object(o5))));
        assert_eq!(g5, [pair("a", "o005")].into());
        // An interned term that is never an object of p selects nothing.
        let a = s.term_id(&iri("a")).expect("a interned");
        assert_eq!(drain(select(&s, P, Bound::Object(a))), [] as [_; 0]);
    }

    #[test]
    fn cursor_both_bound_is_unique() {
        let s = big_store();
        let a = s.term_id(&iri("a")).expect("a interned");
        let o7 = s.term_id(&iri("o007")).expect("o007 interned");
        assert_eq!(drain(select(&s, P, Bound::Both(a, o7))).len(), 1);
        // A subject/object that never co-occur select nothing.
        let z = s.term_id(&iri("z")).expect("z interned");
        assert!(
            drain(select(&s, P, Bound::Both(z, o7))).is_empty(),
            "z only links o000, never o007"
        );
    }

    #[test]
    fn cursor_tail_only_small_relation() {
        // A relation below the seal threshold is a pure tail (no batches) — the
        // allocation-light regime — and still selects correctly on every bound.
        let k = "<https://example.org/k>";
        let mut s = RelationStore::new();
        for (sub, obj) in [("a", "b"), ("a", "c"), ("b", "c")] {
            assert!(
                s.insert(&iri(sub), k, &iri(obj), RelationStore::DEFAULT_GRAPH)
                    .unwrap()
                    .is_some()
            );
        }
        let a = s.term_id(&iri("a")).expect("a interned");
        let got = resolved_set(&s, &drain(select(&s, k, Bound::Subject(a))));
        assert_eq!(got, [pair("a", "b"), pair("a", "c")].into());
        let c = s.term_id(&iri("c")).expect("c interned");
        assert_eq!(
            resolved_set(&s, &drain(select(&s, k, Bound::Object(c)))),
            [pair("a", "c"), pair("b", "c")].into()
        );
        assert!(s.contains(&iri("b"), k, &iri("c"), RelationStore::DEFAULT_GRAPH));
        assert!(!s.contains(&iri("a"), k, &iri("z"), RelationStore::DEFAULT_GRAPH));
    }

    #[test]
    fn cursor_unknown_predicate_is_an_empty_cursor() {
        let s = big_store();
        assert_eq!(
            drain(select(&s, "<https://example.org/absent>", Bound::Any)),
            [] as [_; 0]
        );
        assert_eq!(
            partition_of(&s, "<https://example.org/absent>")
                .map_or(0, |p| p.values_subject(None).count()),
            0
        );
    }

    #[test]
    fn cursor_any_remaining_probes_without_collecting() {
        let s = big_store();
        let a = s.term_id(&iri("a")).expect("a interned");
        assert!(select(&s, P, Bound::Subject(a)).any_remaining());
        let o0 = s.term_id(&iri("o000")).expect("o000 interned");
        assert!(select(&s, P, Bound::Both(a, o0)).any_remaining());
        let z = s.term_id(&iri("z")).expect("z interned");
        let o7 = s.term_id(&iri("o007")).expect("o007 interned");
        assert!(!select(&s, P, Bound::Both(z, o7)).any_remaining());
    }

    /// The `Debug` surface reports the cursor's position, not the borrowed arrangement.
    #[test]
    fn cursor_debug_prints_position_not_columns() {
        let s = big_store();
        let text = format!("{:?}", select(&s, P, Bound::Any));
        assert!(text.starts_with("RowCursor"), "{text}");
        assert!(text.contains("leg"), "{text}");
        let values = format!(
            "{:?}",
            partition_of(&s, P)
                .expect("the fixture partition exists")
                .values_object(None)
        );
        assert!(values.starts_with("ValueCursor"), "{values}");
    }

    /// The trie cursor globally merges several immutable batches plus the tail in either
    /// orientation, and fixing the opposite column narrows the sorted stream.
    #[test]
    fn value_cursor_is_globally_sorted_in_both_orientations() {
        let s = big_store();

        let subject_rows: Vec<_> = partition_of(&s, P)
            .expect("the fixture partition exists")
            .values_subject(None)
            .collect();
        assert_eq!(subject_rows.len(), 201);
        assert!(subject_rows.windows(2).all(|rows| rows[0].0 <= rows[1].0));

        let object_rows: Vec<_> = partition_of(&s, P)
            .expect("the fixture partition exists")
            .values_object(None)
            .collect();
        assert_eq!(object_rows.len(), 201);
        assert!(object_rows.windows(2).all(|rows| rows[0].0 <= rows[1].0));

        let o0 = s.term_id(&iri("o000")).expect("o000 interned");
        let subjects_at_o0: Vec<_> = partition_of(&s, P)
            .expect("the fixture partition exists")
            .values_subject(Some(o0))
            .collect();
        assert_eq!(subjects_at_o0.len(), 2, "a and z point at o000");
        assert!(subjects_at_o0.windows(2).all(|rows| rows[0].0 <= rows[1].0));

        let a = s.term_id(&iri("a")).expect("a interned");
        let objects_at_a: Vec<_> = partition_of(&s, P)
            .expect("the fixture partition exists")
            .values_object(Some(a))
            .collect();
        assert_eq!(objects_at_a.len(), 200);
        assert!(objects_at_a.windows(2).all(|rows| rows[0].0 <= rows[1].0));
    }

    /// Seek applies to every sorted run before the k-way merge, so no value below the
    /// requested trie frontier can reappear from an older batch or the mutable tail.
    #[test]
    fn value_cursor_seek_advances_all_runs() {
        let s = big_store();
        let target = s.term_id(&iri("o150")).expect("o150 interned");
        let mut cursor = partition_of(&s, P)
            .expect("the fixture partition exists")
            .values_object(None);
        cursor.seek(target);
        let rows: Vec<_> = cursor.collect();
        assert_ne!(rows, [] as [_; 0]);
        assert!(rows.iter().all(|&(value, _)| value >= target));
        assert_eq!(rows[0].0, target);
    }

    #[test]
    fn value_cursor_frontier_removes_exhausted_batch_runs() {
        let s = big_store();
        let mut cursor = partition_of(&s, P)
            .expect("the fixture partition exists")
            .values_object(None);
        let source_count = cursor.position.sources.len();
        assert!(source_count > 1, "fixture must span multiple sorted runs");

        let mut previous = None;
        let mut count = 0;
        while let Some((value, _row)) = cursor.next() {
            if let Some(previous) = previous {
                assert!(previous <= value);
            }
            previous = Some(value);
            count += 1;
            assert!(cursor.position.frontier.len() <= source_count);
        }

        assert_eq!(count, 201);
        assert!(cursor.position.frontier.is_empty());
        assert!(
            cursor
                .position
                .sources
                .iter()
                .all(|source| source.peek::<VALUE_OBJECT, _>(cursor.rel).is_none())
        );
    }

    fn check_value_position_cuts<const COLUMN: u8>(
        store: &RelationStore,
        other: Option<TermId>,
        initial_seek: Option<TermId>,
        resumed_seek: Option<TermId>,
    ) {
        let rel = select(store, P, Bound::Any).rel;
        let start = || {
            let mut cursor = ValueCursor::<COLUMN>::new(rel, other);
            if let Some(target) = initial_seek {
                cursor.seek(target);
            }
            cursor
        };
        let full: Vec<_> = start().collect();
        for cut in 0..=full.len() {
            let mut cursor = start();
            let mut uninterrupted = start();
            for expected in &full[..cut] {
                assert_eq!(cursor.next(), Some(*expected));
                assert_eq!(uninterrupted.next(), Some(*expected));
            }
            let heap_pointer = cursor.position.frontier.as_slice().as_ptr();
            let heap_contents = cursor.position.frontier.as_slice().to_vec();
            let runs_pointer = cursor.position.sources.as_ptr();
            let ValueCursor { rel, position } = cursor;
            let mut resumed = ValueCursor { rel, position };
            assert_eq!(resumed.position.frontier.as_slice().as_ptr(), heap_pointer);
            assert_eq!(resumed.position.frontier.as_slice(), heap_contents);
            assert_eq!(resumed.position.sources.as_ptr(), runs_pointer);
            if let Some(target) = resumed_seek {
                uninterrupted.seek(target);
                resumed.seek(target);
            }
            assert_eq!(
                resumed.collect::<Vec<_>>(),
                uninterrupted.collect::<Vec<_>>(),
                "column {COLUMN}, other {other:?}, cut {cut}, seeks {initial_seek:?}/{resumed_seek:?}"
            );
        }
    }

    #[test]
    fn owned_value_positions_retain_frontiers_at_every_cut_and_seek() {
        let store = big_store();
        let subject = store.term_id(&iri("a")).unwrap();
        let object = store.term_id(&iri("o005")).unwrap();
        let later = store.term_id(&iri("o150")).unwrap();
        let absent = store.term_id(P).unwrap();
        for other in [None, Some(subject), Some(object), Some(absent)] {
            for (initial, resumed) in [
                (None, None),
                (Some(object), None),
                (None, Some(later)),
                (Some(object), Some(later)),
            ] {
                check_value_position_cuts::<VALUE_SUBJECT>(&store, other, initial, resumed);
                check_value_position_cuts::<VALUE_OBJECT>(&store, other, initial, resumed);
            }
        }
    }

    /// Determinism: neither cursor's ANSWER depends on insertion order. Whatever order
    /// the same rows arrive in — and however the batches consequently seal — every bound
    /// shape selects the same surface set, and both trie orientations yield the same
    /// surface multiset in an order that is always ascending in the store's own id
    /// order.
    #[test]
    fn cursor_answers_are_insertion_order_independent() {
        let tuples = big_tuples();
        let reference = store_of(&tuples);
        let reference_any = resolved_set(&reference, &drain(select(&reference, P, Bound::Any)));

        // The surfaces a trie orientation yields, resolved out of id space so two
        // differently-ordered stores are comparable at all.
        let subject_surfaces = |s: &RelationStore| -> Vec<String> {
            partition_of(s, P)
                .expect("the fixture partition exists")
                .values_subject(None)
                .map(|(value, _)| s.interner().resolve(value).to_owned())
                .collect()
        };
        let object_surfaces = |s: &RelationStore| -> Vec<String> {
            partition_of(s, P)
                .expect("the fixture partition exists")
                .values_object(None)
                .map(|(value, _)| s.interner().resolve(value).to_owned())
                .collect()
        };
        let mut reference_subjects = subject_surfaces(&reference);
        let mut reference_objects = object_surfaces(&reference);
        reference_subjects.sort();
        reference_objects.sort();

        for seed in 0..12u64 {
            let permuted = store_of(&permute(&tuples, seed));
            assert_eq!(
                resolved_set(&permuted, &drain(select(&permuted, P, Bound::Any))),
                reference_any,
                "seed {seed}: the full scan selects the same rows"
            );
            for local in ["a", "z"] {
                let want = resolved_set(
                    &reference,
                    &drain(select(
                        &reference,
                        P,
                        Bound::Subject(reference.term_id(&iri(local)).expect("interned")),
                    )),
                );
                let got = resolved_set(
                    &permuted,
                    &drain(select(
                        &permuted,
                        P,
                        Bound::Subject(permuted.term_id(&iri(local)).expect("interned")),
                    )),
                );
                assert_eq!(got, want, "seed {seed}: subject {local}");
            }
            let want_o = resolved_set(
                &reference,
                &drain(select(
                    &reference,
                    P,
                    Bound::Object(reference.term_id(&iri("o000")).expect("interned")),
                )),
            );
            let got_o = resolved_set(
                &permuted,
                &drain(select(
                    &permuted,
                    P,
                    Bound::Object(permuted.term_id(&iri("o000")).expect("interned")),
                )),
            );
            assert_eq!(got_o, want_o, "seed {seed}: object o000");

            // The trie stream is always ascending in the store's id order, and carries
            // exactly the same surfaces whatever that order happens to be.
            let subjects: Vec<_> = partition_of(&permuted, P)
                .expect("the fixture partition exists")
                .values_subject(None)
                .collect();
            assert!(
                subjects.windows(2).all(|w| w[0].0 <= w[1].0),
                "seed {seed}: the subject trie stream is sorted"
            );
            let mut got_subjects = subject_surfaces(&permuted);
            got_subjects.sort();
            assert_eq!(got_subjects, reference_subjects, "seed {seed}");
            let mut got_objects = object_surfaces(&permuted);
            got_objects.sort();
            assert_eq!(got_objects, reference_objects, "seed {seed}");
        }
    }
}
