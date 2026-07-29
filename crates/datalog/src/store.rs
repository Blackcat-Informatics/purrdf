// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The columnar [`RelationStore`]: one shared arrangement per predicate.
//!
//! # The arrangement shape
//!
//! A relation is the `(subject, object)` rows of ONE predicate IRI, held as a
//! **shared arrangement** — a log of sorted immutable batches plus a small mutable
//! tail (the columnar LSM discipline):
//!
//! - A `Batch` is flat dense-id columns (`subj`, `obj`, `row_id`) in canonical
//!   `(subject_id, object_id)` order, so a subject-bound probe GALLOPS the sorted
//!   `subj` column to the term's contiguous run — there is no eager `by_subject`
//!   map, because subject grouping falls out of the sort. The `(object, subject)`
//!   access path is a lazily-built permutation, materialised only on the first
//!   object-bound probe (never eagerly, never at all for a subject-only relation).
//! - The mutable **tail** absorbs the current epoch's inserts unsorted; it is
//!   sealed into a sorted batch once it reaches `TAIL_SEAL_THRESHOLD`, and
//!   adjacent batches consolidate by a streaming merge that is geometric
//!   (size-tiered), so the live batch count stays logarithmic. A tiny relation
//!   never seals — it stays a single small tail `Vec`, allocation-light.
//! - Dedup on insert is a GALLOPING probe of every sorted batch plus a linear scan
//!   of the small tail: **no per-row hashing and no postings-list maintenance**.
//! - The single sorted representation is generic over an abelian [`Weight`] monoid,
//!   instantiated `W = ()` in production. The same consolidation merge compiles for
//!   `W = i64` (Z-set signed multiplicities), so signed-weight consolidation — and
//!   hence incremental maintenance with retraction — falls out of one representation
//!   as a compiled fact rather than a promise.
//!
//! # Determinism
//!
//! - Term ids are minted by the store's single [`TermInterner`], keyed on the term's
//!   **lexical surface**, so two terms share an id exactly when their surfaces are
//!   byte-equal. A batch's internal `(subject_id, object_id)` sort is by mint order —
//!   an INTERNAL storage order, never an emission order.
//! - A join probe translates a ground surface to an id via [`RelationStore::term_id`]
//!   (non-inserting): a miss means the term has never entered the store, so the
//!   selection is empty. That is the single place probe-miss semantics lives.
//! - Every consumer-facing sweep is sorted LEXICALLY — [`RelationStore::predicates`]
//!   through a `BTreeSet`, [`RelationStore::facts_sorted`] through an explicit sort —
//!   never by mint order and never by hash-table order.
//! - The interners hold a `hashbrown::HashTable` for O(1) borrowed-key probes. That
//!   table is **never iterated**: it is keyed by a fixed-key `ahash` and is only ever
//!   asked "which id, if any, carries this surface". Insertion order lives in the
//!   parallel `Vec` side arena, which is what every sweep reads.
//!
//! # Terms are lexical surfaces
//!
//! The store interns a term's already-rendered lexical surface (`&str`), not a
//! structured RDF term: the surface IS the dedup key, so carrying a structured term
//! would add a type dependency without changing a single identity. Callers render
//! once, at their own boundary, and the store hands the surface back through
//! [`TermInterner::resolve`].

use core::cmp::Ordering;
use core::convert::Infallible;
use core::fmt;
use core::hash::Hasher;
use std::collections::BTreeSet;
use std::sync::OnceLock;

use hashbrown::HashTable;

use crate::cursor::{RowCursor, VALUE_OBJECT, VALUE_SUBJECT, ValueCursor};
use crate::id::{PredId, RowId, TermId};

// ── Interners ───────────────────────────────────────────────────────────────────

/// Fixed-key hash of a borrowed surface, for every borrowed-key probe in this crate.
///
/// The key is fixed (`ahash`'s default, seeded from constants — never from ambient
/// entropy, which does not exist on `wasm32-unknown-unknown`) and is never persisted:
/// determinism comes from insertion order and the sorted sweeps, never from this hash.
#[inline]
fn surface_hash(surface: &str) -> u64 {
    let mut hasher = ahash::AHasher::default();
    hasher.write(surface.as_bytes());
    hasher.finish()
}

/// A per-store term dictionary: lexical surface → dense [`TermId`].
///
/// Ids are assigned in insertion order and are meaningless outside the store that
/// minted them; they are never serialized and never hashed into a stable identity.
///
/// The surface bytes live ONCE, in the `surfaces` side arena. The probe table holds
/// the [`TermId`] only, so a borrowed `&str` probe resolves a candidate id to its
/// slice without allocating an owned key.
#[derive(Debug, Clone, Default)]
pub struct TermInterner {
    /// Surface → id, for O(1) intern/lookup. Never iterated (see the module docs).
    by_surface: HashTable<TermId>,
    /// First-seen surface per id, in insertion order (slot = id index).
    surfaces: Vec<String>,
}

impl TermInterner {
    /// A fresh, empty dictionary.
    pub fn new() -> Self {
        Self::default()
    }

    /// Intern `surface`, minting a new insertion-ordered id if it is new, else
    /// returning the existing id.
    pub fn intern(&mut self, surface: &str) -> TermId {
        let hash = surface_hash(surface);
        let surfaces = &self.surfaces;
        if let Some(&id) = self
            .by_surface
            .find(hash, |&id| surfaces[id.index()] == surface)
        {
            return id;
        }
        let id = TermId::from_index(self.surfaces.len());
        self.surfaces.push(surface.to_owned());
        let surfaces = &self.surfaces;
        self.by_surface
            .insert_unique(hash, id, |&id| surface_hash(&surfaces[id.index()]));
        id
    }

    /// The id of the term with this surface, if already interned; never inserts.
    pub fn lookup(&self, surface: &str) -> Option<TermId> {
        let hash = surface_hash(surface);
        self.by_surface
            .find(hash, |&id| self.surfaces[id.index()] == surface)
            .copied()
    }

    /// The lexical surface `id` was interned from.
    ///
    /// # Panics
    ///
    /// Panics if `id` was not minted by this dictionary. Ids are per-store handles,
    /// so a foreign id is a programming error, never a data state.
    pub fn resolve(&self, id: TermId) -> &str {
        self.surfaces.get(id.index()).map_or_else(
            || {
                panic!(
                    "TermId {id:?} was not minted by this interner (len {}): \
                     term ids are per-store handles and must never cross store boundaries",
                    self.surfaces.len()
                )
            },
            String::as_str,
        )
    }

    /// The number of distinct terms interned.
    pub fn len(&self) -> usize {
        self.surfaces.len()
    }

    /// Whether the dictionary holds no terms.
    pub fn is_empty(&self) -> bool {
        self.surfaces.is_empty()
    }
}

/// A per-store predicate dictionary: predicate IRI surface → dense [`PredId`].
///
/// The same borrowed-key discipline as [`TermInterner`], so the relation table can be
/// keyed by a `Copy` niche integer instead of an owned `String`.
#[derive(Debug, Clone, Default)]
struct PredInterner {
    /// Predicate surface → id. Never iterated (see the module docs).
    by_name: HashTable<PredId>,
    /// First-seen predicate IRI per id, in insertion order (slot = id index).
    names: Vec<String>,
}

impl PredInterner {
    /// Intern `name`, minting a new insertion-ordered id if it is new.
    fn intern(&mut self, name: &str) -> PredId {
        let hash = surface_hash(name);
        let names = &self.names;
        if let Some(&id) = self.by_name.find(hash, |&id| names[id.index()] == name) {
            return id;
        }
        let id = PredId::from_index(self.names.len());
        self.names.push(name.to_owned());
        let names = &self.names;
        self.by_name
            .insert_unique(hash, id, |&id| surface_hash(&names[id.index()]));
        id
    }

    /// The id of the predicate with this surface, if already interned; never inserts.
    fn lookup(&self, name: &str) -> Option<PredId> {
        let hash = surface_hash(name);
        self.by_name
            .find(hash, |&id| self.names[id.index()] == name)
            .copied()
    }

    /// Every interned predicate surface, in mint order (slot order).
    ///
    /// Mint order is NEVER an emission order: callers that sweep resolve and sort
    /// lexically (see [`RelationStore::predicates`]).
    fn names(&self) -> &[String] {
        &self.names
    }
}

// ── Bounds ──────────────────────────────────────────────────────────────────────

/// A position-pattern over a binary relation's `(subject, object)` columns.
///
/// The [`TermId`] payloads are handles minted by the interner of the SAME
/// [`RelationStore`] the bound is probed against (obtain them through
/// [`RelationStore::term_id`]), so a join probes a relation without re-rendering or
/// re-hashing a term surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bound {
    /// No position bound — every tuple of the relation.
    Any,
    /// Subject bound to this interned term.
    Subject(TermId),
    /// Object bound to this interned term.
    Object(TermId),
    /// Both positions bound, `(subject, object)`, to these interned terms.
    Both(TermId, TermId),
}

// ── The weight monoid: the Z-set seam ───────────────────────────────────────────

/// The failure a signed weight can report during consolidation.
///
/// Saturation is not a ring operation (and is not associative across mixed-sign
/// updates), so an overflowing combine must hard-fail rather than silently change the
/// Z-set it is maintaining.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeightError {
    /// A signed-weight addition left the `i64` ring.
    Overflow {
        /// The left operand of the combine.
        lhs: i64,
        /// The right operand of the combine.
        rhs: i64,
    },
}

impl fmt::Display for WeightError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Overflow { lhs, rhs } => write!(
                f,
                "signed weight addition overflow: {lhs} + {rhs} leaves the i64 ring"
            ),
        }
    }
}

impl std::error::Error for WeightError {}

/// An abelian weight monoid over relation rows — the Z-set seam.
///
/// # The contract
///
/// [`combine`](Self::combine) is the operation applied when two runs carry the SAME
/// `(subject, object)` key during consolidation. It must be **associative** and
/// **commutative** and must treat [`UNIT`](Self::UNIT) consistently, because
/// consolidation merges runs pairwise in an order fixed by batch geometry, not by any
/// caller-visible sequence: a non-abelian combine would make a relation's contents
/// depend on when its batches happened to seal, which would break the crate's
/// determinism contract outright.
///
/// # Why the generic exists
///
/// Production set semantics instantiate `W = ()` — the unit monoid, a zero-sized type,
/// so `Vec<()>` allocates nothing and the weight column costs zero live bytes. The
/// SAME merge compiles for `W = i64`, a Z-set with signed multiplicities, where
/// `combine` sums weights and an annihilated (zero) row drops. Retraction — and with it
/// incremental (backward/forward, DRed-style) maintenance — is therefore already a
/// compiled property of the representation: the lever is one type parameter, never a
/// redesign of the store.
pub trait Weight: Copy {
    /// Structured failure type for consolidation. Set weights are infallible; signed
    /// weights report checked-ring overflow.
    type Error;
    /// The multiplicity of a freshly inserted row.
    const UNIT: Self;
    /// The abelian combine applied to two rows sharing a `(subject, object)` key.
    ///
    /// # Errors
    ///
    /// Returns the monoid's error when the combine leaves its ring; `W = ()` is
    /// infallible and its error type is uninhabited.
    fn combine(self, rhs: Self) -> Result<Self, Self::Error>;
    /// Whether a combined weight annihilates the row, so consolidation drops it.
    fn is_annihilated(self) -> bool;
}

impl Weight for () {
    type Error = Infallible;
    const UNIT: Self = ();
    #[inline]
    fn combine(self, _rhs: Self) -> Result<Self, Self::Error> {
        Ok(())
    }
    #[inline]
    fn is_annihilated(self) -> bool {
        // Set semantics: every live row has unit weight and never consolidates away.
        false
    }
}

impl Weight for i64 {
    type Error = WeightError;
    const UNIT: Self = 1;
    #[inline]
    fn combine(self, rhs: Self) -> Result<Self, Self::Error> {
        self.checked_add(rhs)
            .ok_or(WeightError::Overflow { lhs: self, rhs })
    }
    #[inline]
    fn is_annihilated(self) -> bool {
        self == 0
    }
}

// ── Galloping ───────────────────────────────────────────────────────────────────

/// The first position `>= from` in the strictly-ascending run `xs` whose value is
/// `>= key`, found by GALLOPING — an exponential probe to bracket the answer, then a
/// binary search inside the bracket. Never a linear scan and never a hash probe.
///
/// This is the sorted-run lower bound the whole arrangement leans on (subject-run
/// location, object-run location, dedup), and the exact primitive a multiway leapfrog
/// triejoin composes.
fn gallop_lower_bound(xs: &[TermId], from: usize, key: TermId) -> usize {
    let len = xs.len();
    if from >= len {
        return len;
    }
    if xs[from] >= key {
        return from;
    }
    // Exponential probe: keep `xs[lo] < key`, doubling the stride until `hi` brackets a
    // value `>= key` (or runs off the end).
    let mut lo = from;
    let mut step = 1usize;
    let hi = loop {
        let probe = lo.saturating_add(step);
        if probe >= len {
            break len;
        }
        if xs[probe] >= key {
            break probe;
        }
        lo = probe;
        step = step.saturating_mul(2);
    };
    // The first position `>= key` lies in `(lo, hi]`; binary-search it.
    let (mut left, mut right) = (lo + 1, hi);
    while left < right {
        let mid = left + (right - left) / 2;
        if xs[mid] >= key {
            right = mid;
        } else {
            left = mid + 1;
        }
    }
    left
}

// ── Batches ─────────────────────────────────────────────────────────────────────

/// The lazily-built secondary access path for one [`Batch`]: the batch's row positions
/// in `(object_id, subject_id)` order, so an object-bound probe gallops to its run.
///
/// Built ON FIRST object-bound demand — never eagerly, never at all for a subject-only
/// relation — and memoized in a [`OnceLock`], which is write-once and `Sync`, so a
/// parallel delta-partition firing that shares `&Batch` across threads initialises it
/// cleanly. A permutation of `u32` positions, never a hash map and never a per-key
/// `Vec`: four bytes per row, materialised only when an object bound is actually
/// probed.
#[derive(Debug, Clone, Default)]
struct ObjectIndex {
    /// Row positions of the batch, sorted by `(object_id, subject_id)`.
    perm: Box<[u32]>,
}

/// One immutable sorted batch: a relation's `(subject, object)` rows in canonical
/// `(subject_id, object_id)` order, stored as flat dense-id columns.
///
/// The primary sort is subject-major, so a subject-bound probe gallops the `subj`
/// column to the term's contiguous run with NO secondary structure. The
/// `(object, subject)` access path is the lazily-built [`ObjectIndex`]. Generic over
/// the weight monoid `W` (the Z-set seam); the production instantiation is `W = ()`.
#[derive(Debug, Clone)]
pub(crate) struct Batch<W: Weight = ()> {
    /// Subject column, ascending (subject-major within the `(subject, object)` sort).
    subj: Vec<TermId>,
    /// Object column, ascending within each subject run.
    obj: Vec<TermId>,
    /// Store-global dense [`RowId`] per row, parallel to the columns.
    row_id: Vec<RowId>,
    /// Multiplicity per row; `Vec<()>` is zero-sized under set semantics.
    weight: Vec<W>,
    /// The lazily-built `(object, subject)` access path (built on first object probe).
    object_index: OnceLock<ObjectIndex>,
}

impl<W: Weight> Batch<W> {
    /// Build a batch from rows ALREADY sorted ascending by `(subject_id, object_id)`
    /// and free of duplicate keys. Weights default to [`Weight::UNIT`].
    fn from_sorted(rows: &[(TermId, TermId, RowId)]) -> Self {
        let mut subj = Vec::with_capacity(rows.len());
        let mut obj = Vec::with_capacity(rows.len());
        let mut row_id = Vec::with_capacity(rows.len());
        let mut weight = Vec::with_capacity(rows.len());
        for &(s, o, r) in rows {
            subj.push(s);
            obj.push(o);
            row_id.push(r);
            weight.push(W::UNIT);
        }
        Self {
            subj,
            obj,
            row_id,
            weight,
            object_index: OnceLock::new(),
        }
    }

    /// The number of rows in the batch.
    #[inline]
    pub(crate) fn len(&self) -> usize {
        self.row_id.len()
    }

    /// The `(subject_id, object_id, row_id)` id row at column position `p`.
    #[inline]
    pub(crate) fn row_at(&self, p: usize) -> (TermId, TermId, RowId) {
        (self.subj[p], self.obj[p], self.row_id[p])
    }

    /// The `[lo, hi)` column-position run whose subject is `s`, located by galloping
    /// the sorted `subj` column (subject grouping is contiguous in the primary sort).
    pub(crate) fn subject_run(&self, s: TermId) -> (usize, usize) {
        let lo = gallop_lower_bound(&self.subj, 0, s);
        // `hi` is the first position past `s`'s contiguous run — a binary search of the
        // sorted suffix, so a `Both` probe stays O(log n) rather than O(run length).
        let hi = lo + self.subj[lo..].partition_point(|&x| x <= s);
        (lo, hi)
    }

    /// The single column position of the unique `(s, o)` row, if present: gallop the
    /// subject run, then binary-search its ascending `obj` sub-column for `o`.
    pub(crate) fn both_pos(&self, s: TermId, o: TermId) -> Option<usize> {
        let (lo, hi) = self.subject_run(s);
        let run = &self.obj[lo..hi];
        run.binary_search(&o).ok().map(|off| lo + off)
    }

    /// Whether the unique `(s, o)` key is present in this batch.
    fn contains(&self, s: TermId, o: TermId) -> bool {
        self.both_pos(s, o).is_some()
    }

    /// The batch positions whose object is `o`, via the lazily-built [`ObjectIndex`].
    /// A subslice of the `(object, subject)`-sorted permutation.
    pub(crate) fn object_positions(&self, o: TermId) -> &[u32] {
        let perm = self.object_order();
        let lo = perm.partition_point(|&p| self.obj[p as usize] < o);
        let hi = perm.partition_point(|&p| self.obj[p as usize] <= o);
        &perm[lo..hi]
    }

    /// Every batch position in `(object_id, subject_id)` order.
    ///
    /// The same lazy, memoized permutation backs object-bound binary probes and
    /// object-major trie levels; it is built once and shared by both operators.
    pub(crate) fn object_order(&self) -> &[u32] {
        &self
            .object_index
            .get_or_init(|| {
                let mut perm: Vec<u32> = (0..self.len() as u32).collect();
                // Sort positions by (object_id, subject_id) — the secondary access
                // order. `(object, subject)` keys are unique within a batch (the
                // primary sort is key-disjoint), so no equal elements exist whose order
                // would need preserving: the unstable sort is a pure win (no scratch
                // allocation, lower constants) and cannot introduce nondeterminism.
                perm.sort_unstable_by(|&a, &b| {
                    let (a, b) = (a as usize, b as usize);
                    (self.obj[a], self.subj[a]).cmp(&(self.obj[b], self.subj[b]))
                });
                ObjectIndex {
                    perm: perm.into_boxed_slice(),
                }
            })
            .perm
    }
}

/// Merge two sorted batches into one sorted batch.
///
/// A streaming two-way merge over the `(subject_id, object_id)` key: O(1) scratch
/// beyond the output, never a whole-relation re-sort, so there is no transient
/// allocation spike. On a key COLLISION — only reachable for a signed weight monoid,
/// since set-semantics inserts keep batches key-disjoint — the weights
/// [`combine`](Weight::combine) and the surviving row keeps the LOWER [`RowId`]
/// (deterministic, independent of which run it came from); an annihilated weight drops
/// the row. For `W = ()` the collision arm is dead and this is a plain interleave.
fn merge_batches<W: Weight>(left: &Batch<W>, right: &Batch<W>) -> Result<Batch<W>, W::Error> {
    let cap = left.len() + right.len();
    let mut subj = Vec::with_capacity(cap);
    let mut obj = Vec::with_capacity(cap);
    let mut row_id = Vec::with_capacity(cap);
    let mut weight = Vec::with_capacity(cap);
    let (mut i, mut j) = (0usize, 0usize);
    let push = |subj: &mut Vec<TermId>,
                obj: &mut Vec<TermId>,
                row_id: &mut Vec<RowId>,
                weight: &mut Vec<W>,
                b: &Batch<W>,
                p: usize| {
        subj.push(b.subj[p]);
        obj.push(b.obj[p]);
        row_id.push(b.row_id[p]);
        weight.push(b.weight[p]);
    };
    while i < left.len() && j < right.len() {
        let lk = (left.subj[i], left.obj[i]);
        let rk = (right.subj[j], right.obj[j]);
        match lk.cmp(&rk) {
            Ordering::Less => {
                push(&mut subj, &mut obj, &mut row_id, &mut weight, left, i);
                i += 1;
            }
            Ordering::Greater => {
                push(&mut subj, &mut obj, &mut row_id, &mut weight, right, j);
                j += 1;
            }
            Ordering::Equal => {
                // Key collision (signed-weight only): combine, keep the lower RowId,
                // drop the row if annihilated. Never reached under `W = ()`.
                let w = left.weight[i].combine(right.weight[j])?;
                if !w.is_annihilated() {
                    subj.push(left.subj[i]);
                    obj.push(left.obj[i]);
                    row_id.push(left.row_id[i].min(right.row_id[j]));
                    weight.push(w);
                }
                i += 1;
                j += 1;
            }
        }
    }
    while i < left.len() {
        push(&mut subj, &mut obj, &mut row_id, &mut weight, left, i);
        i += 1;
    }
    while j < right.len() {
        push(&mut subj, &mut obj, &mut row_id, &mut weight, right, j);
        j += 1;
    }
    Ok(Batch {
        subj,
        obj,
        row_id,
        weight,
        object_index: OnceLock::new(),
    })
}

// ── Relations ───────────────────────────────────────────────────────────────────

/// The size a mutable tail may reach before it is sealed into a sorted batch.
///
/// A tiny relation never reaches it — it stays a single small tail `Vec`,
/// allocation-light. Chosen small so the tail scans (dedup on insert, and the cursor's
/// tail leg) stay cheap between seals.
pub(crate) const TAIL_SEAL_THRESHOLD: usize = 64;

/// A single binary relation: the `(subject, object)` rows of ONE predicate IRI, held
/// as a shared arrangement — a log of sorted immutable [`Batch`]es plus a mutable tail.
///
/// Term interning lives at the [`RelationStore`] level (one dictionary shared by every
/// relation), so `insert` borrows the store's interner. Production set semantics fix
/// the weight monoid at `W = ()`.
#[derive(Debug, Clone, Default)]
pub(crate) struct Relation {
    /// Immutable sorted batches (each `(subject_id, object_id)`-ordered and
    /// key-disjoint), newest last. Empty for a tail-only (never-sealed) relation.
    batches: Vec<Batch>,
    /// The mutable tail: `(subject_id, object_id, row_id)` rows of the current epoch,
    /// in insertion order, sealed once it reaches [`TAIL_SEAL_THRESHOLD`].
    tail: Vec<(TermId, TermId, RowId)>,
    /// The number of rows across batches + tail (the dense per-relation row count).
    len: usize,
}

impl Relation {
    /// Insert `(subject, object)` if its `(subject_id, object_id)` key is not already
    /// present, stamping it with the store-assigned `row_id`; return
    /// `Some((subject_id, object_id))` if newly inserted, or `None` on a duplicate.
    ///
    /// Dedup is a GALLOPING probe of every sorted batch plus a linear scan of the small
    /// tail — no per-row hashing, no postings maintenance. A new row is appended to the
    /// unsorted tail; when the tail reaches [`TAIL_SEAL_THRESHOLD`] it is sealed into a
    /// sorted batch and the batch log consolidates.
    fn insert(
        &mut self,
        interner: &mut TermInterner,
        subject: &str,
        object: &str,
        row_id: RowId,
    ) -> Option<(TermId, TermId)> {
        let s_id = interner.intern(subject);
        let o_id = interner.intern(object);
        if self.contains(s_id, o_id) {
            return None;
        }
        self.tail.push((s_id, o_id, row_id));
        self.len += 1;
        if self.tail.len() >= TAIL_SEAL_THRESHOLD {
            self.seal();
        }
        Some((s_id, o_id))
    }

    /// Whether a tuple with these interned terms is present — a galloping probe of each
    /// sorted batch plus a linear scan of the tail (no hashing).
    fn contains(&self, subject: TermId, object: TermId) -> bool {
        self.batches.iter().any(|b| b.contains(subject, object))
            || self
                .tail
                .iter()
                .any(|&(s, o, _)| s == subject && o == object)
    }

    /// Seal the mutable tail into a new sorted immutable batch, then consolidate.
    ///
    /// Sorting the tail by `(subject_id, object_id)` establishes the canonical batch
    /// order; the tail is dedup-free by construction (insert rejects duplicate keys), so
    /// the sort is a plain columnar build with no combine. Consolidation then merges the
    /// batch log geometrically. Row ids are already stamped, so sealing is a pure
    /// storage reorganisation — it never changes the row set, the row ids, or the count.
    fn seal(&mut self) {
        if self.tail.is_empty() {
            return;
        }
        let mut rows = core::mem::take(&mut self.tail);
        rows.sort_unstable_by_key(|&(s, o, _)| (s, o));
        self.batches.push(Batch::from_sorted(&rows));
        self.consolidate();
    }

    /// Geometric (size-tiered) consolidation: while the newest two batches are within a
    /// factor of two in size, merge them into one sorted batch. This bounds the live
    /// batch count logarithmically, so a probe gallops O(log n) runs.
    fn consolidate(&mut self) {
        while self.batches.len() >= 2 {
            let n = self.batches.len();
            let (a, b) = (self.batches[n - 2].len(), self.batches[n - 1].len());
            if b * 2 < a {
                break;
            }
            let right = self.batches.pop().expect("len >= 2");
            let left = self.batches.pop().expect("len >= 2");
            let merged = match merge_batches(&left, &right) {
                Ok(batch) => batch,
                // `W = ()` is infallible: its error type is uninhabited, so this arm is
                // a compile-time proof that set-semantics consolidation cannot fail.
                Err(never) => match never {},
            };
            self.batches.push(merged);
        }
    }

    /// The number of rows in this relation (batches + tail).
    #[inline]
    pub(crate) fn row_count(&self) -> usize {
        self.len
    }

    /// A lending [`RowCursor`] over the `(subject_id, object_id, row_id)` id rows
    /// selected by `bound`, borrowing this relation's columns — no per-stage `Vec` is
    /// materialised.
    ///
    /// The cursor concatenates each batch's bound-run (galloped over the sorted columns)
    /// with a linear scan of the tail. Enumeration order is batch-then-tail, NOT a
    /// global merge sort; that is sound because the cursor's order never reaches an
    /// output path (see the [`crate::cursor`] module docs). The `(s, o)` key is unique,
    /// so a `Both` bound yields at most one row across the whole relation.
    fn select(&self, bound: Bound) -> RowCursor<'_> {
        RowCursor::new(self, bound)
    }

    /// The batches of this relation, newest last — the cursor's per-batch sub-runs.
    #[inline]
    pub(crate) fn batches(&self) -> &[Batch] {
        &self.batches
    }

    /// The unsorted tail rows — the cursor's final (linear-scanned) leg.
    #[inline]
    pub(crate) fn tail(&self) -> &[(TermId, TermId, RowId)] {
        &self.tail
    }
}

// ── The store ───────────────────────────────────────────────────────────────────

/// One projected row of the store: a `(subject, predicate, object)` triple of lexical
/// surfaces.
///
/// The derived [`Ord`] is `(subject, predicate, object)` lexical order — the emission
/// order [`RelationStore::facts_sorted`] establishes, and the only fact ordering this
/// crate ever produces.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Fact {
    /// The subject term's lexical surface.
    pub subject: String,
    /// The predicate IRI surface.
    pub predicate: String,
    /// The object term's lexical surface.
    pub object: String,
}

/// A columnar set of binary relations keyed by predicate IRI.
///
/// One `Relation` per predicate, all sharing ONE [`TermInterner`]; this is the
/// evaluator's working EDB/IDB form. The ids the interner mints are meaningless outside
/// this store — probes obtain them through [`Self::term_id`].
#[derive(Debug, Clone, Default)]
pub struct RelationStore {
    /// The store's term dictionary, shared by every relation. This is the persistent
    /// term arena: never reset within the store's lifetime, because a [`TermId`] handed
    /// out in round 1 must still resolve in round 40.
    interner: TermInterner,
    /// The store's predicate dictionary, interned once at first insert. Keeps
    /// [`relations`](Self::relations) keyed by a `Copy` niche integer rather than an
    /// owned `String`.
    predicates: PredInterner,
    /// Binary relations indexed by [`PredId`] slot (`relations[pid.index()]`).
    ///
    /// Predicate ids are minted densely (0, 1, 2, …), so a new predicate's slot is
    /// always the vector's current length; there are never gap / empty relations.
    relations: Vec<Relation>,
    /// The number of rows inserted so far across ALL relations — equivalently, the next
    /// dense [`RowId`] slot to assign. Row ids are minted `0, 1, 2, …` in store-wide
    /// insertion order, so at any point the live rows are exactly `0..row_count`. This
    /// is the single row-id source, and the id never enters a provenance identity.
    row_count: usize,
    /// A permanently-empty relation handed to [`select`](Self::select) on a predicate
    /// miss, so an unknown predicate yields an empty [`RowCursor`] with NO `Option`
    /// branch on the per-row scan. Never inserted into.
    empty: Relation,
}

impl RelationStore {
    /// A fresh, empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert `(subject, object)` under `predicate`, all three as lexical surfaces.
    ///
    /// Returns `Some((subject_id, object_id, row_id))` with the terms' interned ids and
    /// the newly-assigned store-global [`RowId`] if the tuple was newly inserted, or
    /// `None` if it was already present (dedup).
    ///
    /// The predicate IRI is interned to a [`PredId`] once through a borrowed-key probe —
    /// no owned-key clone per call. A successful insert stamps the row with the next
    /// dense row id (insertion order across the whole store), which is the identity the
    /// semi-naive delta bitset is keyed on. The interned subject/object ids are returned
    /// alongside it so a commit-path caller threads them onward without a redundant
    /// second dictionary lookup.
    pub fn insert(
        &mut self,
        predicate: &str,
        subject: &str,
        object: &str,
    ) -> Option<(TermId, TermId, RowId)> {
        let idx = self.predicates.intern(predicate).index();
        if idx >= self.relations.len() {
            // A newly-minted PredId's slot is always the current length (dense mint),
            // so this resize adds exactly one default relation — never an empty gap.
            self.relations.resize_with(idx + 1, Relation::default);
        }
        let row_id = RowId::from_index(self.row_count);
        self.relations[idx]
            .insert(&mut self.interner, subject, object, row_id)
            .map(|(s_id, o_id)| {
                self.row_count += 1;
                (s_id, o_id, row_id)
            })
    }

    /// The number of rows currently in the store across all relations — equivalently,
    /// the exclusive upper bound of the live dense [`RowId`]s (`0..row_count`).
    ///
    /// The semi-naive fixpoint sizes its round-1 delta bitset from this: every
    /// accumulated row is "new" in round 1, so the seed is
    /// [`DenseBitset::all_set`](crate::bitset::DenseBitset::all_set) over this count,
    /// with no per-key materialisation.
    pub fn row_count(&self) -> usize {
        self.row_count
    }

    /// The store's term dictionary — for resolving a selected id row's `(subject,
    /// object)` back to their lexical surfaces at the point a caller renders.
    pub fn interner(&self) -> &TermInterner {
        &self.interner
    }

    /// The interned [`PredId`] for `predicate`, if any relation of this store carries
    /// it; never inserts. `None` means no relation, so any selection on it is empty.
    pub fn pred_id(&self, predicate: &str) -> Option<PredId> {
        self.predicates.lookup(predicate)
    }

    /// The interned id of the term with this lexical surface, if the term has ever been
    /// inserted into ANY relation of this store. Never inserts.
    ///
    /// This is the SINGLE place probe-miss semantics lives: `None` means the term has
    /// never been seen, so any selection or membership bound on it is empty / false —
    /// callers short-circuit to the empty result exactly where a surface-keyed index
    /// would have produced zero matches.
    pub fn term_id(&self, surface: &str) -> Option<TermId> {
        self.interner.lookup(surface)
    }

    /// Whether `(subject, predicate, object)` is present, as lexical surfaces.
    ///
    /// Membership for negation-as-failure and downstream dedup: both term surfaces must
    /// resolve to interned ids ([`Self::term_id`]) or the tuple cannot be present.
    pub fn contains(&self, predicate: &str, subject: &str, object: &str) -> bool {
        let (Some(s), Some(o)) = (self.term_id(subject), self.term_id(object)) else {
            return false;
        };
        self.relation(predicate).is_some_and(|r| r.contains(s, o))
    }

    /// A galloping lending [`RowCursor`] over the id rows under `predicate` selected by
    /// `bound`.
    ///
    /// Yields interned `(subject_id, object_id, row_id)` rows (`Copy` — nothing is
    /// cloned) one at a time: the term ids for lazy surface resolution through
    /// [`interner`](Self::interner) where you render, and the store-global [`RowId`] for
    /// a one-word delta-bitset probe. Picks the cheapest access path for the bound
    /// positions; an unknown predicate yields an empty cursor over the shared empty
    /// relation, with NO `Vec` materialised.
    pub fn select(&self, predicate: &str, bound: Bound) -> RowCursor<'_> {
        self.relation(predicate)
            .unwrap_or(&self.empty)
            .select(bound)
    }

    /// A globally subject-value-ordered trie-level cursor over one predicate relation.
    ///
    /// `other` optionally fixes the object position. An unknown predicate uses the
    /// permanent empty relation, matching [`Self::select`]'s probe-miss semantics.
    pub fn values_subject(
        &self,
        predicate: &str,
        other: Option<TermId>,
    ) -> ValueCursor<'_, VALUE_SUBJECT> {
        ValueCursor::new(self.relation(predicate).unwrap_or(&self.empty), other)
    }

    /// The object-value-ordered sibling of [`Self::values_subject`]; `other` optionally
    /// fixes the subject position.
    pub fn values_object(
        &self,
        predicate: &str,
        other: Option<TermId>,
    ) -> ValueCursor<'_, VALUE_OBJECT> {
        ValueCursor::new(self.relation(predicate).unwrap_or(&self.empty), other)
    }

    /// The number of distinct tuples stored under `predicate` (0 if unknown).
    pub fn len_for(&self, predicate: &str) -> usize {
        self.relation(predicate).map_or(0, Relation::row_count)
    }

    /// The relation for `predicate`, if interned (resolves `PredId` → slot).
    fn relation(&self, predicate: &str) -> Option<&Relation> {
        self.predicates
            .lookup(predicate)
            .and_then(|pid| self.relations.get(pid.index()))
    }

    /// Every predicate IRI surface that has at least one tuple, in LEXICAL order.
    ///
    /// Resolves every interned [`PredId`] back to its surface and sorts through a
    /// `BTreeSet` — NEVER by mint order, which is insertion order and carries no lexical
    /// meaning — so any "all relations" sweep is byte-deterministic. Every interned
    /// predicate has at least one tuple, because a [`PredId`] is minted only by
    /// [`insert`](Self::insert), which then adds the row.
    pub fn predicates(&self) -> impl Iterator<Item = &str> {
        self.predicates
            .names()
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>()
            .into_iter()
    }

    /// Project every live row back to a [`Fact`] triple of lexical surfaces, in sorted
    /// `(subject, predicate, object)` order.
    ///
    /// This is the single columnar-to-lexical bridge. Keeping it in one place prevents
    /// consumers from growing subtly different seed ordering or term-resolution rules,
    /// and it is the surface a determinism test compares across permuted insertion
    /// orders.
    pub fn facts_sorted(&self) -> Vec<Fact> {
        let mut facts = Vec::with_capacity(self.row_count);
        for predicate in self.predicates() {
            let mut cursor = self.select(predicate, Bound::Any);
            while let Some((s_id, o_id, _row)) = crate::cursor::LendingIterator::next(&mut cursor) {
                facts.push(Fact {
                    subject: self.interner.resolve(s_id).to_owned(),
                    predicate: predicate.to_owned(),
                    object: self.interner.resolve(o_id).to_owned(),
                });
            }
        }
        facts.sort_unstable();
        facts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cursor::LendingIterator;
    use crate::test_support::permute;

    /// The lexical surface of an example IRI, as a caller would render it.
    fn iri(local: &str) -> String {
        format!("<https://example.org/{local}>")
    }

    /// Drain a [`RowCursor`] into a `Vec` of id rows — a test-only helper for asserting
    /// a selection's full sequence (`select` returns a lending cursor, so the production
    /// hot path never collects).
    fn select_rows(
        s: &RelationStore,
        predicate: &str,
        bound: Bound,
    ) -> Vec<(TermId, TermId, RowId)> {
        let mut cursor = s.select(predicate, bound);
        let mut rows = Vec::new();
        while let Some(row) = cursor.next() {
            rows.push(row);
        }
        rows
    }

    /// The interned id for a surface, asserting it is present.
    fn id_of(s: &RelationStore, surface: &str) -> TermId {
        s.term_id(surface)
            .unwrap_or_else(|| panic!("term {surface:?} must be interned"))
    }

    /// Resolve selected id rows to an ORDER-INDEPENDENT `(subject, object)` surface set.
    ///
    /// The arrangement enumerates batch-then-tail — an internal storage order, not an
    /// emission order — so a store test asserts the row SET, never a sequence.
    fn resolved_set(
        s: &RelationStore,
        rows: &[(TermId, TermId, RowId)],
    ) -> BTreeSet<(String, String)> {
        rows.iter()
            .map(|&(si, oi, _row)| {
                (
                    s.interner().resolve(si).to_owned(),
                    s.interner().resolve(oi).to_owned(),
                )
            })
            .collect()
    }

    fn pair(a: &str, b: &str) -> (String, String) {
        (iri(a), iri(b))
    }

    const KNOWS: &str = "https://example.org/knows";
    const LIKES: &str = "https://example.org/likes";

    /// The `knows`/`likes` corpus: `knows` = (a,b), (a,c), (b,c); `likes` = (a,c).
    fn sample_tuples() -> Vec<(&'static str, &'static str, &'static str)> {
        vec![
            (KNOWS, "a", "b"),
            (KNOWS, "a", "c"),
            (KNOWS, "b", "c"),
            (LIKES, "a", "c"),
        ]
    }

    /// Build a store from `tuples`, asserting every insert is new.
    fn store_of(tuples: &[(&str, &str, &str)]) -> RelationStore {
        let mut s = RelationStore::new();
        for &(p, sub, obj) in tuples {
            assert!(
                s.insert(p, &iri(sub), &iri(obj)).is_some(),
                "fixture tuples are distinct"
            );
        }
        s
    }

    fn sample_store() -> RelationStore {
        store_of(&sample_tuples())
    }

    #[test]
    fn store_select_subject_bound() {
        let s = sample_store();
        let a = id_of(&s, &iri("a"));
        let got = select_rows(&s, KNOWS, Bound::Subject(a));
        assert_eq!(
            resolved_set(&s, &got),
            [pair("a", "b"), pair("a", "c")].into()
        );
    }

    #[test]
    fn store_select_object_bound() {
        let s = sample_store();
        let c = id_of(&s, &iri("c"));
        let got = select_rows(&s, KNOWS, Bound::Object(c));
        assert_eq!(
            resolved_set(&s, &got),
            [pair("a", "c"), pair("b", "c")].into()
        );
    }

    #[test]
    fn store_select_both_bound() {
        let s = sample_store();
        let a = id_of(&s, &iri("a"));
        let b = id_of(&s, &iri("b"));
        let c = id_of(&s, &iri("c"));
        let got = select_rows(&s, KNOWS, Bound::Both(a, c));
        assert_eq!(resolved_set(&s, &got), [pair("a", "c")].into());

        // A both-bound miss (b is interned but (b,b) is not a tuple) yields nothing.
        assert!(select_rows(&s, KNOWS, Bound::Both(b, b)).is_empty());
    }

    #[test]
    fn store_select_any_yields_every_row() {
        let s = sample_store();
        let got = select_rows(&s, KNOWS, Bound::Any);
        assert_eq!(
            resolved_set(&s, &got),
            [pair("a", "b"), pair("a", "c"), pair("b", "c")].into()
        );
    }

    #[test]
    fn store_dedup_reports_none_and_stores_one_row() {
        let mut s = RelationStore::new();
        assert!(s.insert(KNOWS, &iri("a"), &iri("b")).is_some());
        // Re-inserting the same (s, p, o) is a no-op that reports None (no new row id).
        assert!(s.insert(KNOWS, &iri("a"), &iri("b")).is_none());
        assert_eq!(s.len_for(KNOWS), 1);
        assert_eq!(
            resolved_set(&s, &select_rows(&s, KNOWS, Bound::Any)),
            [pair("a", "b")].into(),
        );
    }

    /// `insert` stamps each newly-inserted row with a dense [`RowId`] in store-wide
    /// insertion order — `0, 1, 2, …` ACROSS relations, not per relation — and `select`
    /// hands each selected row that same id. A dedup consumes no id, so the id space
    /// stays gap-free and `row_count` counts exactly the live rows. The ids are asserted
    /// as SETS: the arrangement stores rows value-sorted, so a selection enumerates them
    /// in storage order, not insertion order.
    #[test]
    fn store_insert_assigns_dense_cross_relation_row_ids() {
        let mut s = RelationStore::new();
        // Interleave predicates so a per-relation index would NOT match the global id.
        let r0 = s.insert(KNOWS, &iri("a"), &iri("b")).map(|(_, _, r)| r);
        let r1 = s.insert(LIKES, &iri("a"), &iri("c")).map(|(_, _, r)| r);
        let r2 = s.insert(KNOWS, &iri("a"), &iri("c")).map(|(_, _, r)| r);
        assert_eq!(r0, Some(RowId::from_index(0)));
        assert_eq!(r1, Some(RowId::from_index(1)));
        assert_eq!(
            r2,
            Some(RowId::from_index(2)),
            "row ids span relations in insertion order"
        );
        // A dedup consumes no row id — the space stays dense and `row_count` is exact.
        assert_eq!(s.insert(KNOWS, &iri("a"), &iri("b")), None);
        assert_eq!(s.row_count(), 3, "three distinct rows ⇒ row ids 0..3");
        // `select` hands each row its store-global id (never a per-relation index):
        // knows carries {0, 2}, likes carries {1} — the interleaved likes took id 1.
        let knows_ids: BTreeSet<RowId> = select_rows(&s, KNOWS, Bound::Any)
            .iter()
            .map(|&(_, _, r)| r)
            .collect();
        assert_eq!(
            knows_ids,
            [RowId::from_index(0), RowId::from_index(2)].into(),
            "selected rows carry their store-global row id, not a per-relation index",
        );
        let likes_ids: BTreeSet<RowId> = select_rows(&s, LIKES, Bound::Any)
            .iter()
            .map(|&(_, _, r)| r)
            .collect();
        assert_eq!(likes_ids, [RowId::from_index(1)].into());
    }

    /// The arrangement seals its tail into sorted batches past the threshold and still
    /// returns the exact row SET with the exact store-global row ids — the galloping
    /// batch path, not just the tail leg. A heavily-interleaved build (row ids NOT
    /// contiguous within a relation) confirms every selected row carries its dense global
    /// id and that `row_count` stays exact across relations.
    #[test]
    fn store_sealed_batches_preserve_row_set_and_dense_ids() {
        let (p, q) = ("https://example.org/p", "https://example.org/q");
        let mut s = RelationStore::new();
        // Interleave p and q for more than twice the threshold so BOTH relations seal
        // batches and neither relation's row ids are the contiguous 0, 1, 2, ….
        let n = TAIL_SEAL_THRESHOLD * 3;
        for i in 0..n {
            let pred = if i % 2 == 0 { p } else { q };
            assert!(
                s.insert(pred, &iri("s"), &iri(&format!("o{i:04}")))
                    .is_some()
            );
        }
        assert_eq!(s.row_count(), n, "every distinct row is counted, gap-free");
        // Each relation returns exactly its half of the rows, each with the global row id
        // it was stamped with at insert (the even indices went to p, odd to q).
        let p_ids: BTreeSet<RowId> = select_rows(&s, p, Bound::Any)
            .iter()
            .map(|&(_, _, r)| r)
            .collect();
        let expect_p: BTreeSet<RowId> = (0..n).step_by(2).map(RowId::from_index).collect();
        assert_eq!(p_ids, expect_p, "p carries exactly the even-index row ids");
        // A subject-bound gallop over the sealed batches finds every one of s's edges.
        let subj = id_of(&s, &iri("s"));
        assert_eq!(
            select_rows(&s, p, Bound::Subject(subj)).len(),
            n / 2,
            "subject gallop over sealed batches finds all rows"
        );
        // Dedup still holds across sealed batches: re-inserting a sealed row is a no-op.
        assert!(
            s.insert(p, &iri("s"), &iri("o0000")).is_none(),
            "a row already sealed into a batch is deduped by the galloping probe"
        );
    }

    #[test]
    fn store_contains_on_lexical_surfaces() {
        let s = sample_store();
        assert!(s.contains(KNOWS, &iri("a"), &iri("b")));
        // A never-seen term surface fails the lookup, so containment is false.
        assert!(!s.contains(KNOWS, &iri("a"), &iri("z")));
        // An unknown predicate is a clean miss, not a panic.
        assert!(!s.contains("https://example.org/nope", &iri("a"), &iri("b")));
    }

    #[test]
    fn store_term_id_lookup_never_inserts() {
        let s = sample_store();
        assert!(s.term_id(&iri("a")).is_some());
        assert_eq!(s.term_id(&iri("never-seen")), None);
        // The miss did not insert: a second lookup still misses.
        assert_eq!(s.term_id(&iri("never-seen")), None);
        assert_eq!(s.interner().len(), 3, "exactly a, b, c are interned");
        assert!(!s.interner().is_empty());
    }

    #[test]
    fn store_interner_is_shared_across_relations() {
        // The same term inserted under two predicates mints ONE id (store-level
        // dictionary), and a Bound built from that id probes either relation.
        let s = sample_store();
        let a = id_of(&s, &iri("a"));
        assert_eq!(
            resolved_set(&s, &select_rows(&s, LIKES, Bound::Subject(a))),
            [pair("a", "c")].into(),
        );
        assert_eq!(s.pred_id(KNOWS), Some(PredId::from_index(0)));
        assert_eq!(s.pred_id("https://example.org/absent"), None);
    }

    /// Resolving an id no dictionary minted is a programming error, reported as a panic
    /// rather than a silent wrong surface.
    #[test]
    #[should_panic(expected = "must never cross store boundaries")]
    fn store_resolving_a_foreign_term_id_panics() {
        let s = sample_store();
        let _ = s.interner().resolve(TermId::from_index(999));
    }

    /// Emission-order guard: the relation table is a `PredId`-indexed `Vec`, so its slot
    /// order is mint order. The only consumer-facing enumeration — [`predicates`] — must
    /// still be lexical, sorted through the `BTreeSet` sweep, never leaking mint order.
    /// Insert in deliberately anti-lexical order and assert the output is lexical.
    #[test]
    fn store_predicates_never_leak_mint_or_hash_order() {
        let mut s = RelationStore::new();
        for pred in [
            "https://example.org/zeta",
            "https://example.org/mu",
            "https://example.org/alpha",
        ] {
            assert!(s.insert(pred, &iri("x"), &iri("y")).is_some());
        }
        let preds: Vec<&str> = s.predicates().collect();
        assert_eq!(
            preds,
            vec![
                "https://example.org/alpha",
                "https://example.org/mu",
                "https://example.org/zeta"
            ],
            "predicates() must be lexical — predicate mint order must never leak"
        );
    }

    #[test]
    fn store_predicates_are_sorted_and_repeatable() {
        let s = sample_store();
        let preds: Vec<&str> = s.predicates().collect();
        assert_eq!(preds, vec![KNOWS, LIKES]);

        // Repeated builds give identical select output.
        let s2 = sample_store();
        assert_eq!(
            select_rows(&s, KNOWS, Bound::Any),
            select_rows(&s2, KNOWS, Bound::Any),
        );
        assert_eq!(preds, s2.predicates().collect::<Vec<_>>());
    }

    /// `facts_sorted` is the lexical projection: sorted `(subject, predicate, object)`,
    /// covering every relation, with no duplicate row.
    #[test]
    fn store_facts_sorted_is_lexical_and_complete() {
        let s = sample_store();
        let facts = s.facts_sorted();
        assert_eq!(facts.len(), 4);
        let mut expected: Vec<Fact> = sample_tuples()
            .into_iter()
            .map(|(p, sub, obj)| Fact {
                subject: iri(sub),
                predicate: p.to_owned(),
                object: iri(obj),
            })
            .collect();
        expected.sort();
        assert_eq!(facts, expected);
        assert!(
            facts.windows(2).all(|w| w[0] < w[1]),
            "facts_sorted is strictly ascending"
        );
    }

    // ── Determinism ─────────────────────────────────────────────────────────────

    /// The determinism contract, property style: the ORDER in which tuples are inserted
    /// cannot affect any lexical observable of the store. Over many deterministic
    /// permutations of a corpus large enough to force several batch seals and
    /// consolidations, `facts_sorted`, `predicates`, `row_count`, `len_for` and every
    /// membership answer are identical.
    ///
    /// Row ids are deliberately NOT compared: a row id is mint order, which is a
    /// function of insertion order by construction and is never an emission order.
    #[test]
    fn store_insertion_order_does_not_affect_lexical_output() {
        let preds = ["https://example.org/p", "https://example.org/q"];
        // 150 tuples over two predicates: past the seal threshold in both, so batches
        // seal and consolidate at different points under different orders.
        let corpus: Vec<(usize, usize, usize)> = (0..150)
            .map(|i| (i % 2, i % 7, i % 23))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let tuples: Vec<(String, String, String)> = corpus
            .iter()
            .map(|&(p, sub, obj)| {
                (
                    preds[p].to_owned(),
                    iri(&format!("s{sub}")),
                    iri(&format!("o{obj}")),
                )
            })
            .collect();

        let build = |order: &[(String, String, String)]| {
            let mut s = RelationStore::new();
            for (p, sub, obj) in order {
                s.insert(p, sub, obj);
            }
            s
        };

        let reference = build(&tuples);
        let reference_facts = reference.facts_sorted();
        let reference_preds: Vec<String> = reference.predicates().map(ToOwned::to_owned).collect();
        assert!(
            reference.row_count() >= TAIL_SEAL_THRESHOLD,
            "the corpus must be large enough to seal batches"
        );

        for seed in 0..24u64 {
            // Duplicated tuples are absorbed whatever the order, so append a second
            // shuffled copy of the whole corpus.
            let mut order = permute(&tuples, seed);
            order.extend(permute(&tuples, seed ^ 0x5EED));
            let permuted = build(&order);
            assert_eq!(
                permuted.facts_sorted(),
                reference_facts,
                "seed {seed}: facts_sorted is insertion-order independent"
            );
            assert_eq!(
                permuted.predicates().collect::<Vec<_>>(),
                reference_preds,
                "seed {seed}: the predicate sweep is insertion-order independent"
            );
            assert_eq!(permuted.row_count(), reference.row_count(), "seed {seed}");
            for p in &preds {
                assert_eq!(permuted.len_for(p), reference.len_for(p), "seed {seed}");
            }
            for fact in &reference_facts {
                assert!(
                    permuted.contains(&fact.predicate, &fact.subject, &fact.object),
                    "seed {seed}: every reference fact is present"
                );
            }
            assert!(
                !permuted.contains(preds[0], &iri("s0"), &iri("never-seen")),
                "seed {seed}: an absent tuple stays absent"
            );
        }
    }

    /// Every access path answers the same question: for each bound shape the selected
    /// row SET is identical whatever the insertion order, including after seals.
    #[test]
    fn store_every_bound_shape_is_insertion_order_independent() {
        let pred = "https://example.org/p";
        let tuples: Vec<(String, String)> = (0..90)
            .map(|i| (iri(&format!("s{}", i % 5)), iri(&format!("o{}", i % 17))))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();

        let build = |order: &[(String, String)]| {
            let mut s = RelationStore::new();
            for (sub, obj) in order {
                s.insert(pred, sub, obj);
            }
            s
        };
        let reference = build(&tuples);
        let subjects: Vec<String> = (0..5).map(|i| iri(&format!("s{i}"))).collect();
        let objects: Vec<String> = (0..17).map(|i| iri(&format!("o{i}"))).collect();

        let observe = |s: &RelationStore| {
            let mut out: Vec<BTreeSet<(String, String)>> = Vec::new();
            out.push(resolved_set(s, &select_rows(s, pred, Bound::Any)));
            for sub in &subjects {
                let id = id_of(s, sub);
                out.push(resolved_set(s, &select_rows(s, pred, Bound::Subject(id))));
            }
            for obj in &objects {
                let id = id_of(s, obj);
                out.push(resolved_set(s, &select_rows(s, pred, Bound::Object(id))));
            }
            for sub in &subjects {
                for obj in &objects {
                    let (si, oi) = (id_of(s, sub), id_of(s, obj));
                    out.push(resolved_set(s, &select_rows(s, pred, Bound::Both(si, oi))));
                }
            }
            out
        };
        let expected = observe(&reference);

        for seed in 0..16u64 {
            let permuted = build(&permute(&tuples, seed));
            assert_eq!(
                observe(&permuted),
                expected,
                "seed {seed}: every bound shape selects the same row set"
            );
        }
    }

    // ── The Z-set seam: signed-weight consolidation ─────────────────────────────

    /// Build a single-row `Batch<i64>` with an explicit signed weight — the seam a
    /// signed delta rides. Set-semantics `insert` never mints a non-unit weight, so the
    /// seam is exercised here by constructing weighted batches directly.
    fn weighted_row(s: TermId, o: TermId, r: RowId, w: i64) -> Batch<i64> {
        Batch {
            subj: vec![s],
            obj: vec![o],
            row_id: vec![r],
            weight: vec![w],
            object_index: OnceLock::new(),
        }
    }

    /// The consolidation merge is generic over the [`Weight`] monoid and compiles for
    /// `W = i64` (a Z-set): a `+1` and a `-1` on the SAME key combine to `0`, which
    /// annihilates and DROPS the row — retraction falls out of the same merge, with no
    /// special deletion pass. This makes "the representation admits signed weights" a
    /// compiled, exercised fact rather than a promise; production stays at `W = ()`.
    #[test]
    fn batch_merge_is_a_z_set_over_signed_weights() {
        let s = TermId::from_index(0);
        let o = TermId::from_index(1);

        // (+1) + (-1) = 0 ⇒ the row annihilates and is dropped (retraction).
        let plus = weighted_row(s, o, RowId::from_index(5), 1);
        let minus = weighted_row(s, o, RowId::from_index(2), -1);
        let retracted = merge_batches(&plus, &minus).expect("signed retraction combines");
        assert_eq!(retracted.len(), 0, "(+1)+(-1)=0 annihilates the shared key");

        // (+1) + (+2) = 3 ⇒ one surviving row, weights summed, lower row id kept.
        let two = weighted_row(s, o, RowId::from_index(2), 2);
        let summed = merge_batches(&plus, &two).expect("signed addition combines");
        assert_eq!(summed.len(), 1, "a non-annihilating combine keeps one row");
        assert_eq!(summed.weight[0], 3, "weights sum: 1 + 2 = 3");
        assert_eq!(
            summed.row_id[0],
            RowId::from_index(2),
            "the lower row id deterministically survives a key collision"
        );

        // Disjoint keys interleave with NO combine — the set-semantics `W = ()` shape.
        let o2 = TermId::from_index(2);
        let a = weighted_row(s, o, RowId::from_index(0), 1);
        let b = weighted_row(s, o2, RowId::from_index(1), 1);
        let disjoint = merge_batches(&a, &b).expect("disjoint signed batches interleave");
        assert_eq!(disjoint.len(), 2, "disjoint keys interleave, no combine");
    }

    /// The abelian law the [`Weight`] contract demands, exercised on the signed monoid:
    /// merging in either run order yields the same weights, and the same surviving row
    /// id, so consolidation cannot depend on batch geometry.
    #[test]
    fn signed_weight_combine_is_commutative_across_merge_order() {
        let s = TermId::from_index(0);
        let o = TermId::from_index(1);
        let left = weighted_row(s, o, RowId::from_index(7), 4);
        let right = weighted_row(s, o, RowId::from_index(3), -1);
        let forward = merge_batches(&left, &right).expect("combines");
        let backward = merge_batches(&right, &left).expect("combines");
        assert_eq!(forward.weight, backward.weight, "combine commutes");
        assert_eq!(forward.row_id, backward.row_id, "the lower row id wins");
        assert_eq!(forward.weight[0], 3);
    }

    /// Saturation is not a ring operation (and is not associative across mixed-sign
    /// updates), so overflow must hard-fail instead of silently changing the Z-set.
    #[test]
    fn signed_weight_overflow_never_saturates() {
        let err = i64::MAX
            .combine(1)
            .expect_err("signed overflow must be a structured failure");
        let text = err.to_string();
        assert!(text.contains("overflow"), "{text}");
        assert!(text.contains("addition"), "{text}");
        assert_eq!(
            err,
            WeightError::Overflow {
                lhs: i64::MAX,
                rhs: 1
            }
        );
        // The unit monoid is infallible and never annihilates.
        assert_eq!(<() as Weight>::UNIT.combine(()), Ok(()));
        assert!(!<() as Weight>::UNIT.is_annihilated());
        assert!(0i64.is_annihilated());
    }

    /// The galloping lower bound is exactly `partition_point` over the ascending run,
    /// from every start offset — the property the whole arrangement leans on.
    #[test]
    fn gallop_lower_bound_matches_the_linear_lower_bound() {
        // A run with gaps, so keys both present and absent are exercised.
        let run: Vec<TermId> = (0..64).map(|i| TermId::from_index(i * 3)).collect();
        for key_slot in 0..200usize {
            let key = TermId::from_index(key_slot);
            for from in [0usize, 1, 7, 63, 64] {
                let expected = from + run[from.min(run.len())..].partition_point(|&x| x < key);
                assert_eq!(
                    gallop_lower_bound(&run, from, key),
                    expected,
                    "key slot {key_slot} from {from}"
                );
            }
        }
        // An empty run and an out-of-range start both answer with the length.
        assert_eq!(gallop_lower_bound(&[], 0, TermId::from_index(0)), 0);
        assert_eq!(gallop_lower_bound(&run, 999, TermId::from_index(0)), 64);
    }
}
