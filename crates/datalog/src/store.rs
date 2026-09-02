// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The columnar [`RelationStore`]: ONE arity-4 relation
//! `triple(subject, predicate, object, graph)`, physically partitioned by
//! `(predicate, graph)`.
//!
//! # One relation, partitioned — not one relation per predicate
//!
//! The store holds a single logical relation whose four positions are all ordinary terms,
//! interned in ONE [`TermInterner`]. That is what lets a clause variable bind a predicate
//! in one atom and a subject in another (`prp-dom`'s `?p`): a predicate is a term, so its
//! id space is the term id space, not a separate symbol table.
//!
//! Physically the rows are partitioned on the `(predicate, graph)` positions, because those
//! two are constants in the overwhelming majority of atoms. The partitioning is what keeps
//! the common case fast and the general case possible at once:
//!
//! * a **constant (or already-bound) predicate and graph** address exactly one partition
//!   through [`RelationStore::partition`] — one ordered-map probe on a pair of `u32` ids —
//!   and then reach the very same galloping `(subject, object)` index the store has always
//!   had. Carrying the predicate as data costs such an atom nothing;
//! * a **free predicate or graph** sweeps the matching partitions through
//!   [`RelationStore::partitions`], in LEXICAL `(predicate surface, graph surface)` order,
//!   and each partition is still probed through its own `(subject, object)` index. An
//!   unbound predicate therefore degrades to "one indexed probe per predicate", never to a
//!   scan of every row in the store.
//!
//! # The default graph
//!
//! [`RelationStore::DEFAULT_GRAPH`] is the EMPTY surface. RDF's default graph has no name
//! and PurRDF mints no vocabulary, so the store denotes "no name" by no name rather than by
//! a fabricated IRI; no IRI surface (`<…>`) and no literal surface (`"…"`) is empty, so the
//! denotation cannot collide with a caller's term. It is the same denotation
//! [`ClauseTerm::DefaultGraph`](crate::clause::ClauseTerm::DefaultGraph) renders to.
//!
//! # The arrangement shape
//!
//! One partition is the `(subject, object)` rows sharing a `(predicate, graph)` key, held
//! as a **shared arrangement** — a log of sorted immutable batches plus a small mutable
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
//!   through a `BTreeSet`, [`RelationStore::partitions`] through the maintained lexical
//!   partition order, [`RelationStore::facts_sorted`] through an explicit sort — never by
//!   mint order and never by hash-table order.
//! - The interner holds a `hashbrown::HashTable` for O(1) borrowed-key probes. That
//!   table is **never iterated**: it is keyed by a fixed-key `ahash` and is only ever
//!   asked "which id, if any, carries this surface". Insertion order lives in the
//!   parallel `Vec` side arena, which is what every sweep reads.
//! - The partition table is a `BTreeMap` keyed by `(predicate id, graph id)`. It is
//!   probed, never iterated: id order is mint order, so every partition SWEEP reads the
//!   separately maintained lexical order instead.
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
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use hashbrown::HashTable;

use crate::cursor::{RowCursor, VALUE_OBJECT, VALUE_SUBJECT, ValueCursor};
use crate::id::{PartitionId, RowId, TermId};

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
    /// Running total of `surfaces`' bytes, maintained on intern so the dictionary's
    /// footprint is an O(1) read rather than an O(n) sweep. See [`Self::byte_len`].
    bytes: usize,
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
        self.bytes += surface.len();
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

    /// The total bytes of the interned surfaces — the dictionary's term-arena footprint.
    ///
    /// Counts each distinct surface ONCE (a repeated term is stored once), and counts the
    /// surface bytes only, not the `Vec`/`HashTable` bookkeeping around them. This is the
    /// quantity the evaluator's arena ceiling is expressed against, so the ceiling is a
    /// property of the data rather than of an allocator's growth policy.
    pub fn byte_len(&self) -> usize {
        self.bytes
    }

    /// Whether the dictionary holds no terms.
    pub fn is_empty(&self) -> bool {
        self.surfaces.is_empty()
    }
}

// ── Bounds ──────────────────────────────────────────────────────────────────────

/// A position-pattern over one partition's `(subject, object)` columns.
///
/// The predicate and graph positions are NOT here: they choose the partition, and the
/// partition's arrangement is indexed on `(subject, object)`. That separation is what lets
/// an atom with a free predicate keep its subject/object index — see the module docs.
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
#[non_exhaustive]
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

/// A single partition of the store's arity-4 relation: the `(subject, object)` rows
/// sharing one `(predicate, graph)` key, held as a shared arrangement — a log of sorted
/// immutable [`Batch`]es plus a mutable tail.
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

/// One projected row of the store: a `(subject, predicate, object, graph)` quad of
/// lexical surfaces.
///
/// The derived [`Ord`] is `(subject, predicate, object, graph)` lexical order — the
/// emission order [`RelationStore::facts_sorted`] establishes, and the only fact ordering
/// this crate ever produces. The graph sorts LAST so a dataset confined to one graph keeps
/// exactly the fact order it had before the position existed.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Fact {
    /// The subject term's lexical surface.
    pub subject: String,
    /// The predicate term's lexical surface — an IRI predicate renders as `<iri>`, the
    /// same bytes it would render to in any other position.
    pub predicate: String,
    /// The object term's lexical surface.
    pub object: String,
    /// The graph term's lexical surface; EMPTY for the default graph
    /// ([`RelationStore::DEFAULT_GRAPH`]).
    pub graph: String,
}

/// One addressed partition of the store's arity-4 relation: the `(subject, object)`
/// arrangement of a single `(predicate, graph)` key.
///
/// A `PartitionRef` is the unit a join actually scans. Obtaining one is where the
/// predicate/graph positions are resolved; everything after it is the store's ordinary
/// galloping `(subject, object)` access, so an atom whose predicate happens to be a
/// variable pays for the extra partitions it visits and for nothing else.
#[derive(Debug, Clone, Copy)]
pub struct PartitionRef<'a> {
    /// The partition's dense slot handle.
    id: PartitionId,
    /// The interned predicate surface this partition is keyed by.
    predicate: TermId,
    /// The interned graph surface this partition is keyed by.
    graph: TermId,
    /// The partition's arrangement.
    relation: &'a Relation,
}

impl<'a> PartitionRef<'a> {
    /// The partition's dense slot handle.
    pub fn id(self) -> PartitionId {
        self.id
    }

    /// The interned predicate this partition is keyed by — the value a free predicate
    /// position binds to.
    pub fn predicate(self) -> TermId {
        self.predicate
    }

    /// The interned graph this partition is keyed by — the value a free graph position
    /// binds to.
    pub fn graph(self) -> TermId {
        self.graph
    }

    /// The number of distinct `(subject, object)` rows in this partition.
    pub fn row_count(self) -> usize {
        self.relation.row_count()
    }

    /// A galloping lending [`RowCursor`] over the id rows selected by `bound`.
    pub fn select(self, bound: Bound) -> RowCursor<'a> {
        self.relation.select(bound)
    }

    /// A globally subject-value-ordered trie-level cursor; `other` optionally fixes the
    /// object position.
    pub fn values_subject(self, other: Option<TermId>) -> ValueCursor<'a, VALUE_SUBJECT> {
        ValueCursor::new(self.relation, other)
    }

    /// The object-ordered sibling of [`Self::values_subject`]; `other` optionally fixes
    /// the subject position.
    pub fn values_object(self, other: Option<TermId>) -> ValueCursor<'a, VALUE_OBJECT> {
        ValueCursor::new(self.relation, other)
    }
}

/// How a [`Partitions`] sweep is being served.
enum PartitionsCursor<'a> {
    /// Both key positions are known, so at most ONE partition can match and it was
    /// located by an ordered-map probe. This is the constant-predicate hot path: no
    /// sweep, no filter, straight to the same arrangement a predicate-keyed store would
    /// have handed back.
    Single(Option<PartitionRef<'a>>),
    /// At least one key position is free: walk the store's lexical partition order,
    /// keeping the partitions whose key agrees with the known positions.
    Sweep {
        /// The store being swept.
        store: &'a RelationStore,
        /// The required predicate, or `None` if the position is free.
        predicate: Option<TermId>,
        /// The required graph, or `None` if the position is free.
        graph: Option<TermId>,
        /// The next index into the store's lexical partition order.
        next: usize,
    },
}

/// The partitions matching a `(predicate, graph)` selection, in LEXICAL
/// `(predicate surface, graph surface)` order.
///
/// Allocation-free in both modes, so an atom's partition resolution never materialises a
/// `Vec` — see [`RelationStore::partitions`].
pub struct Partitions<'a>(PartitionsCursor<'a>);

impl fmt::Debug for Partitions<'_> {
    /// Prints the sweep's shape, never the borrowed store.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            PartitionsCursor::Single(found) => f
                .debug_struct("Partitions")
                .field("mode", &"single")
                .field("found", &found.is_some())
                .finish(),
            PartitionsCursor::Sweep {
                predicate,
                graph,
                next,
                ..
            } => f
                .debug_struct("Partitions")
                .field("mode", &"sweep")
                .field("predicate", predicate)
                .field("graph", graph)
                .field("next", next)
                .finish(),
        }
    }
}

impl<'a> Iterator for Partitions<'a> {
    type Item = PartitionRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            PartitionsCursor::Single(found) => found.take(),
            PartitionsCursor::Sweep {
                store,
                predicate,
                graph,
                next,
            } => {
                while let Some(&slot) = store.order.get(*next) {
                    *next += 1;
                    let (partition_predicate, partition_graph) = store.keys[slot];
                    if predicate.is_none_or(|wanted| wanted == partition_predicate)
                        && graph.is_none_or(|wanted| wanted == partition_graph)
                    {
                        return Some(store.partition_at(slot));
                    }
                }
                None
            }
        }
    }
}

/// The store's ONE arity-4 relation `triple(subject, predicate, object, graph)`,
/// physically partitioned by `(predicate, graph)`.
///
/// Every position is an ordinary term interned in ONE [`TermInterner`], so a clause
/// variable that binds a predicate in one atom can be compared against a subject in
/// another. The ids the interner mints are meaningless outside this store — probes obtain
/// them through [`Self::term_id`]. See the module docs for how the partitioning keeps a
/// constant-predicate atom exactly as fast as it was when predicates were relation
/// symbols.
#[derive(Debug, Clone, Default)]
pub struct RelationStore {
    /// The store's term dictionary, shared by every position of every partition. This is
    /// the persistent term arena: never reset within the store's lifetime, because a
    /// [`TermId`] handed out in round 1 must still resolve in round 40.
    interner: TermInterner,
    /// The partitions, indexed by dense [`PartitionId`] slot.
    relations: Vec<Relation>,
    /// `keys[slot]` is `relations[slot]`'s `(predicate, graph)` key.
    keys: Vec<(TermId, TermId)>,
    /// Key → slot. Probed, NEVER iterated: its order is term mint order, which carries no
    /// lexical meaning. Every sweep reads `order` instead.
    by_key: BTreeMap<(TermId, TermId), usize>,
    /// The partition slots in LEXICAL `(predicate surface, graph surface)` order — the
    /// single sweep order, maintained on partition creation so no sweep has to sort.
    order: Vec<usize>,
    /// The number of rows inserted so far across ALL partitions — equivalently, the next
    /// dense [`RowId`] slot to assign. Row ids are minted `0, 1, 2, …` in store-wide
    /// insertion order, so at any point the live rows are exactly `0..row_count`. This
    /// is the single row-id source, and the id never enters a provenance identity.
    row_count: usize,
    /// A permanently-empty partition handed to [`select`](Self::select) on a partition
    /// miss, so an unknown key yields an empty [`RowCursor`] with NO `Option` branch on
    /// the per-row scan. Never inserted into.
    empty: Relation,
}

impl RelationStore {
    /// The lexical surface of the DEFAULT GRAPH: the EMPTY surface.
    ///
    /// RDF's default graph has no name, and PurRDF mints no vocabulary, so the store says
    /// "no name" rather than inventing one. No IRI surface (`<…>`) and no literal surface
    /// (`"…"`) is empty, so this cannot collide with a caller's term; it is the same
    /// denotation [`ClauseTerm::DefaultGraph`](crate::clause::ClauseTerm::DefaultGraph)
    /// renders to, which is what makes a clause constant and stored data comparable.
    pub const DEFAULT_GRAPH: &'static str = "";

    /// A fresh, empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert the quad `(subject, predicate, object, graph)`, all four as lexical
    /// surfaces.
    ///
    /// Returns `Some((subject_id, object_id, row_id))` with the subject/object interned
    /// ids and the newly-assigned store-global [`RowId`] if the quad was newly inserted,
    /// or `None` if it was already present (dedup).
    ///
    /// The predicate and graph surfaces are interned as ordinary terms and their pair
    /// addresses the partition; a successful insert stamps the row with the next dense row
    /// id (insertion order across the whole store), which is the identity the semi-naive
    /// delta span is keyed on.
    pub fn insert(
        &mut self,
        subject: &str,
        predicate: &str,
        object: &str,
        graph: &str,
    ) -> Option<(TermId, TermId, RowId)> {
        let slot = self.partition_slot(predicate, graph);
        let row_id = RowId::from_index(self.row_count);
        self.relations[slot]
            .insert(&mut self.interner, subject, object, row_id)
            .map(|(s_id, o_id)| {
                self.row_count += 1;
                (s_id, o_id, row_id)
            })
    }

    /// The slot of the `(predicate, graph)` partition, creating it if it is new.
    ///
    /// A new partition is spliced into the lexical `order` at the position a binary search
    /// names, so the sweep order stays lexical without ever being re-sorted.
    fn partition_slot(&mut self, predicate: &str, graph: &str) -> usize {
        let predicate_id = self.interner.intern(predicate);
        let graph_id = self.interner.intern(graph);
        if let Some(&slot) = self.by_key.get(&(predicate_id, graph_id)) {
            return slot;
        }
        let slot = self.relations.len();
        let position = {
            let interner = &self.interner;
            let keys = &self.keys;
            self.order
                .binary_search_by(|&other| {
                    let (other_predicate, other_graph) = keys[other];
                    (
                        interner.resolve(other_predicate),
                        interner.resolve(other_graph),
                    )
                        .cmp(&(predicate, graph))
                })
                .unwrap_or_else(|position| position)
        };
        self.relations.push(Relation::default());
        self.keys.push((predicate_id, graph_id));
        self.by_key.insert((predicate_id, graph_id), slot);
        self.order.insert(position, slot);
        slot
    }

    /// The number of rows currently in the store across all partitions — equivalently,
    /// the exclusive upper bound of the live dense [`RowId`]s (`0..row_count`).
    ///
    /// The semi-naive fixpoint sizes its round-1 delta from this: every accumulated row is
    /// "new" in round 1, so the seed is the contiguous span `[0, row_count)`.
    pub fn row_count(&self) -> usize {
        self.row_count
    }

    /// The number of live `(predicate, graph)` partitions.
    ///
    /// This is the sweep length an atom with a free predicate or graph pays; an atom with
    /// both constant never touches it.
    pub fn partition_count(&self) -> usize {
        self.relations.len()
    }

    /// The store's term dictionary — for resolving an id back to its lexical surface at
    /// the point a caller renders.
    pub fn interner(&self) -> &TermInterner {
        &self.interner
    }

    /// The bytes of interned term surfaces held by this store's dictionary — the
    /// term-arena footprint the evaluator's arena ceiling is measured against.
    ///
    /// Predicate and graph surfaces are terms too, so they are counted here exactly like a
    /// subject or an object: the ceiling measures the whole dictionary, not a subset of it.
    pub fn term_bytes(&self) -> usize {
        self.interner.byte_len()
    }

    /// The interned id of the term with this lexical surface, if the term has ever entered
    /// this store in ANY position. Never inserts.
    ///
    /// This is the SINGLE place probe-miss semantics lives: `None` means the term has
    /// never been seen, so any selection or membership bound on it is empty / false —
    /// callers short-circuit to the empty result exactly where a surface-keyed index
    /// would have produced zero matches.
    pub fn term_id(&self, surface: &str) -> Option<TermId> {
        self.interner.lookup(surface)
    }

    /// The dense handle of the `(predicate, graph)` partition, if it exists; never
    /// creates one.
    pub fn partition_id(&self, predicate: &str, graph: &str) -> Option<PartitionId> {
        let (predicate, graph) = (self.term_id(predicate)?, self.term_id(graph)?);
        self.by_key
            .get(&(predicate, graph))
            .copied()
            .map(PartitionId::from_index)
    }

    /// The partition keyed by these interned positions, if it exists.
    ///
    /// One ordered-map probe on a pair of dense ids — the addressing step of every atom
    /// whose predicate and graph are constants or already-bound variables, and the reason
    /// carrying the predicate as data costs such an atom nothing.
    pub fn partition(&self, predicate: TermId, graph: TermId) -> Option<PartitionRef<'_>> {
        self.by_key
            .get(&(predicate, graph))
            .map(|&slot| self.partition_at(slot))
    }

    /// Every partition matching the given positions, in LEXICAL
    /// `(predicate surface, graph surface)` order; `None` means the position is FREE.
    ///
    /// With both positions known this is the single [`Self::partition`] probe wrapped as a
    /// one-element iterator — no sweep at all. With either free it walks the maintained
    /// lexical order, which is why an unbound predicate is deterministic and is still a
    /// sequence of indexed probes rather than one undifferentiated scan.
    pub fn partitions(&self, predicate: Option<TermId>, graph: Option<TermId>) -> Partitions<'_> {
        match (predicate, graph) {
            (Some(predicate), Some(graph)) => {
                Partitions(PartitionsCursor::Single(self.partition(predicate, graph)))
            }
            _ => Partitions(PartitionsCursor::Sweep {
                store: self,
                predicate,
                graph,
                next: 0,
            }),
        }
    }

    /// The partition at a dense slot.
    fn partition_at(&self, slot: usize) -> PartitionRef<'_> {
        let (predicate, graph) = self.keys[slot];
        PartitionRef {
            id: PartitionId::from_index(slot),
            predicate,
            graph,
            relation: &self.relations[slot],
        }
    }

    /// Whether the quad `(subject, predicate, object, graph)` is present, as lexical
    /// surfaces.
    ///
    /// Membership for negation-as-failure and downstream dedup: every surface must resolve
    /// to an interned id ([`Self::term_id`]) or the quad cannot be present.
    pub fn contains(&self, subject: &str, predicate: &str, object: &str, graph: &str) -> bool {
        let (Some(s), Some(o)) = (self.term_id(subject), self.term_id(object)) else {
            return false;
        };
        self.relation(predicate, graph)
            .is_some_and(|r| r.contains(s, o))
    }

    /// A galloping lending [`RowCursor`] over the id rows of the `(predicate, graph)`
    /// partition selected by `bound`, addressed by lexical surfaces.
    ///
    /// The surface-addressed convenience: the join's hot path resolves ids once and goes
    /// through [`Self::partition`] instead. An unknown partition yields an empty cursor
    /// over the shared empty relation, with NO `Vec` materialised.
    pub fn select(&self, predicate: &str, graph: &str, bound: Bound) -> RowCursor<'_> {
        self.relation(predicate, graph)
            .unwrap_or(&self.empty)
            .select(bound)
    }

    /// The number of distinct quads under this `(predicate, graph)` key (0 if unknown).
    pub fn len_for(&self, predicate: &str, graph: &str) -> usize {
        self.relation(predicate, graph)
            .map_or(0, Relation::row_count)
    }

    /// The partition arrangement for a surface-addressed key, if interned.
    fn relation(&self, predicate: &str, graph: &str) -> Option<&Relation> {
        let (predicate, graph) = (self.term_id(predicate)?, self.term_id(graph)?);
        self.by_key
            .get(&(predicate, graph))
            .and_then(|&slot| self.relations.get(slot))
    }

    /// Every predicate surface that carries at least one quad, DISTINCT and in LEXICAL
    /// order.
    ///
    /// Resolved out of the partition keys and sorted through a `BTreeSet` — never mint
    /// order, which is insertion order and carries no lexical meaning — so any "all
    /// predicates" sweep is byte-deterministic.
    pub fn predicates(&self) -> impl Iterator<Item = &str> {
        self.keys
            .iter()
            .map(|&(predicate, _)| self.interner.resolve(predicate))
            .collect::<BTreeSet<_>>()
            .into_iter()
    }

    /// Every graph surface that carries at least one quad, DISTINCT and in LEXICAL order.
    ///
    /// The default graph appears as the empty surface ([`Self::DEFAULT_GRAPH`]), which
    /// sorts first.
    pub fn graphs(&self) -> impl Iterator<Item = &str> {
        self.keys
            .iter()
            .map(|&(_, graph)| self.interner.resolve(graph))
            .collect::<BTreeSet<_>>()
            .into_iter()
    }

    /// Project every live row back to a [`Fact`] quad of lexical surfaces, in sorted
    /// `(subject, predicate, object, graph)` order.
    ///
    /// This is the single columnar-to-lexical bridge. Keeping it in one place prevents
    /// consumers from growing subtly different seed ordering or term-resolution rules,
    /// and it is the surface a determinism test compares across permuted insertion
    /// orders.
    pub fn facts_sorted(&self) -> Vec<Fact> {
        let mut facts = Vec::with_capacity(self.row_count);
        for &slot in &self.order {
            let (predicate, graph) = self.keys[slot];
            let predicate = self.interner.resolve(predicate);
            let graph = self.interner.resolve(graph);
            let mut cursor = self.relations[slot].select(Bound::Any);
            while let Some((s_id, o_id, _row)) = crate::cursor::LendingIterator::next(&mut cursor) {
                facts.push(Fact {
                    subject: self.interner.resolve(s_id).to_owned(),
                    predicate: predicate.to_owned(),
                    object: self.interner.resolve(o_id).to_owned(),
                    graph: graph.to_owned(),
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
        let mut cursor = s.select(predicate, RelationStore::DEFAULT_GRAPH, bound);
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

    /// Predicate surfaces: a predicate is an ordinary term, so it is stored bracketed
    /// exactly like any other IRI.
    const KNOWS: &str = "<https://example.org/knows>";
    const LIKES: &str = "<https://example.org/likes>";

    /// The `knows`/`likes` corpus: `knows` = (a,b), (a,c), (b,c); `likes` = (a,c).
    fn sample_tuples() -> Vec<(&'static str, &'static str, &'static str)> {
        vec![
            (KNOWS, "a", "b"),
            (KNOWS, "a", "c"),
            (KNOWS, "b", "c"),
            (LIKES, "a", "c"),
        ]
    }

    /// Build a store from `tuples` in the DEFAULT graph, asserting every insert is new.
    fn store_of(tuples: &[(&str, &str, &str)]) -> RelationStore {
        let mut s = RelationStore::new();
        for &(p, sub, obj) in tuples {
            assert!(
                s.insert(&iri(sub), p, &iri(obj), RelationStore::DEFAULT_GRAPH)
                    .is_some(),
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
        assert_eq!(select_rows(&s, KNOWS, Bound::Both(b, b)), [] as [_; 0]);
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
        assert!(
            s.insert(&iri("a"), KNOWS, &iri("b"), RelationStore::DEFAULT_GRAPH)
                .is_some()
        );
        // Re-inserting the same quad is a no-op that reports None (no new row id).
        assert!(
            s.insert(&iri("a"), KNOWS, &iri("b"), RelationStore::DEFAULT_GRAPH)
                .is_none()
        );
        assert_eq!(s.len_for(KNOWS, RelationStore::DEFAULT_GRAPH), 1);
        assert_eq!(
            resolved_set(&s, &select_rows(&s, KNOWS, Bound::Any)),
            [pair("a", "b")].into(),
        );
    }

    /// The SAME `(subject, predicate, object)` in two different graphs is two distinct
    /// quads: the graph is part of the key, so neither dedups the other away and each
    /// lives in its own partition.
    #[test]
    fn store_separates_the_same_triple_in_two_graphs() {
        let g1 = iri("g1");
        let g2 = iri("g2");
        let mut s = RelationStore::new();
        assert!(s.insert(&iri("a"), KNOWS, &iri("b"), &g1).is_some());
        assert!(s.insert(&iri("a"), KNOWS, &iri("b"), &g2).is_some());
        assert!(
            s.insert(&iri("a"), KNOWS, &iri("b"), &g1).is_none(),
            "a repeat within one graph still dedups"
        );
        assert_eq!(s.row_count(), 2);
        assert_eq!(
            s.partition_count(),
            2,
            "one partition per (predicate, graph)"
        );
        assert_eq!(s.len_for(KNOWS, &g1), 1);
        assert_eq!(s.len_for(KNOWS, &g2), 1);
        assert_eq!(
            s.len_for(KNOWS, RelationStore::DEFAULT_GRAPH),
            0,
            "the default graph is a graph of its own, and it is empty here"
        );
        assert!(s.contains(&iri("a"), KNOWS, &iri("b"), &g1));
        assert!(!s.contains(&iri("a"), KNOWS, &iri("b"), RelationStore::DEFAULT_GRAPH));
        assert_eq!(
            s.graphs().collect::<Vec<_>>(),
            vec![g1.as_str(), g2.as_str()]
        );
        assert_eq!(s.predicates().collect::<Vec<_>>(), vec![KNOWS]);
    }

    /// The default graph is the EMPTY surface, and it partitions like any other graph
    /// name while sorting before every named one.
    #[test]
    fn store_default_graph_is_the_empty_surface() {
        assert_eq!(RelationStore::DEFAULT_GRAPH, "");
        let named = iri("g");
        let mut s = RelationStore::new();
        assert!(
            s.insert(&iri("a"), KNOWS, &iri("b"), RelationStore::DEFAULT_GRAPH)
                .is_some()
        );
        assert!(s.insert(&iri("a"), KNOWS, &iri("b"), &named).is_some());
        assert_eq!(
            s.graphs().collect::<Vec<_>>(),
            vec![RelationStore::DEFAULT_GRAPH, named.as_str()],
            "the empty surface sorts first"
        );
        let facts = s.facts_sorted();
        assert_eq!(facts.len(), 2);
        assert_eq!(facts[0].graph, RelationStore::DEFAULT_GRAPH);
        assert_eq!(facts[1].graph, named);
        assert_eq!(
            facts[0].predicate, KNOWS,
            "a predicate is a bracketed IRI term"
        );
    }

    /// `insert` stamps each newly-inserted row with a dense [`RowId`] in store-wide
    /// insertion order — `0, 1, 2, …` ACROSS partitions, not per partition — and `select`
    /// hands each selected row that same id. A dedup consumes no id, so the id space
    /// stays gap-free and `row_count` counts exactly the live rows. The ids are asserted
    /// as SETS: the arrangement stores rows value-sorted, so a selection enumerates them
    /// in storage order, not insertion order.
    #[test]
    fn store_insert_assigns_dense_cross_partition_row_ids() {
        let default = RelationStore::DEFAULT_GRAPH;
        let mut s = RelationStore::new();
        // Interleave predicates so a per-partition index would NOT match the global id.
        let r0 = s
            .insert(&iri("a"), KNOWS, &iri("b"), default)
            .map(|(_, _, r)| r);
        let r1 = s
            .insert(&iri("a"), LIKES, &iri("c"), default)
            .map(|(_, _, r)| r);
        let r2 = s
            .insert(&iri("a"), KNOWS, &iri("c"), default)
            .map(|(_, _, r)| r);
        assert_eq!(r0, Some(RowId::from_index(0)));
        assert_eq!(r1, Some(RowId::from_index(1)));
        assert_eq!(
            r2,
            Some(RowId::from_index(2)),
            "row ids span partitions in insertion order"
        );
        // A dedup consumes no row id — the space stays dense and `row_count` is exact.
        assert_eq!(s.insert(&iri("a"), KNOWS, &iri("b"), default), None);
        assert_eq!(s.row_count(), 3, "three distinct rows ⇒ row ids 0..3");
        // `select` hands each row its store-global id (never a per-partition index):
        // knows carries {0, 2}, likes carries {1} — the interleaved likes took id 1.
        let knows_ids: BTreeSet<RowId> = select_rows(&s, KNOWS, Bound::Any)
            .iter()
            .map(|&(_, _, r)| r)
            .collect();
        assert_eq!(
            knows_ids,
            [RowId::from_index(0), RowId::from_index(2)].into(),
            "selected rows carry their store-global row id, not a per-partition index",
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
    /// contiguous within a partition) confirms every selected row carries its dense global
    /// id and that `row_count` stays exact across partitions.
    #[test]
    fn store_sealed_batches_preserve_row_set_and_dense_ids() {
        let default = RelationStore::DEFAULT_GRAPH;
        let (p, q) = ("<https://example.org/p>", "<https://example.org/q>");
        let mut s = RelationStore::new();
        // Interleave p and q for more than twice the threshold so BOTH partitions seal
        // batches and neither partition's row ids are the contiguous 0, 1, 2, ….
        let n = TAIL_SEAL_THRESHOLD * 3;
        for i in 0..n {
            let pred = if i % 2 == 0 { p } else { q };
            assert!(
                s.insert(&iri("s"), pred, &iri(&format!("o{i:04}")), default)
                    .is_some()
            );
        }
        assert_eq!(s.row_count(), n, "every distinct row is counted, gap-free");
        // Each partition returns exactly its half of the rows, each with the global row id
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
            s.insert(&iri("s"), p, &iri("o0000"), default).is_none(),
            "a row already sealed into a batch is deduped by the galloping probe"
        );
    }

    #[test]
    fn store_contains_on_lexical_surfaces() {
        let s = sample_store();
        let default = RelationStore::DEFAULT_GRAPH;
        assert!(s.contains(&iri("a"), KNOWS, &iri("b"), default));
        // A never-seen term surface fails the lookup, so containment is false.
        assert!(!s.contains(&iri("a"), KNOWS, &iri("z"), default));
        // An unknown predicate is a clean miss, not a panic.
        assert!(!s.contains(&iri("a"), "<https://example.org/nope>", &iri("b"), default));
        // So is an unknown graph.
        assert!(!s.contains(&iri("a"), KNOWS, &iri("b"), &iri("never-a-graph")));
    }

    #[test]
    fn store_term_id_lookup_never_inserts() {
        let s = sample_store();
        assert!(s.term_id(&iri("a")).is_some());
        assert_eq!(s.term_id(&iri("never-seen")), None);
        // The miss did not insert: a second lookup still misses.
        assert_eq!(s.term_id(&iri("never-seen")), None);
        assert_eq!(
            s.interner().len(),
            6,
            "a, b, c, the two predicate surfaces and the default-graph surface"
        );
        assert!(!s.interner().is_empty());
    }

    /// A PREDICATE is an ordinary term in the one dictionary, so the same variable can
    /// bind it in predicate position and compare it in subject position — which is
    /// exactly what `prp-dom`'s `?p` does.
    #[test]
    fn store_predicates_are_terms_in_the_one_dictionary() {
        let default = RelationStore::DEFAULT_GRAPH;
        let domain = "<https://example.org/domain>";
        let mut s = RelationStore::new();
        // `knows domain Person` — the predicate surface KNOWS appears as a SUBJECT here…
        assert!(s.insert(KNOWS, domain, &iri("Person"), default).is_some());
        // …and as the predicate of an ordinary quad here.
        assert!(s.insert(&iri("a"), KNOWS, &iri("b"), default).is_some());
        let knows = id_of(&s, KNOWS);
        assert_eq!(
            s.partition(knows, id_of(&s, default))
                .expect("the knows partition exists")
                .row_count(),
            1
        );
        // One id, both roles: the subject occurrence and the partition key are equal.
        let subjects: BTreeSet<TermId> = select_rows(&s, domain, Bound::Any)
            .iter()
            .map(|&(subject, _, _)| subject)
            .collect();
        assert_eq!(subjects, [knows].into());
    }

    #[test]
    fn store_interner_is_shared_across_partitions() {
        // The same term inserted under two predicates mints ONE id (store-level
        // dictionary), and a Bound built from that id probes either partition.
        let s = sample_store();
        let a = id_of(&s, &iri("a"));
        assert_eq!(
            resolved_set(&s, &select_rows(&s, LIKES, Bound::Subject(a))),
            [pair("a", "c")].into(),
        );
        assert_eq!(
            s.partition_id(KNOWS, RelationStore::DEFAULT_GRAPH),
            Some(PartitionId::from_index(0))
        );
        assert_eq!(
            s.partition_id("<https://example.org/absent>", RelationStore::DEFAULT_GRAPH),
            None
        );
        assert_eq!(s.partition_id(KNOWS, &iri("absent-graph")), None);
    }

    /// The partition sweep is the store's answer to a FREE predicate or graph: it visits
    /// exactly the matching partitions, in lexical order, and each one is still probed
    /// through its own `(subject, object)` index rather than scanned as raw rows.
    #[test]
    fn store_partition_sweep_is_lexical_and_selective() {
        let default = RelationStore::DEFAULT_GRAPH;
        let g = iri("g");
        let mut s = RelationStore::new();
        // Insert in deliberately anti-lexical predicate order.
        for (subject, predicate, object, graph) in [
            ("a", "<https://example.org/zeta>", "b", default),
            ("a", "<https://example.org/mu>", "b", g.as_str()),
            ("a", "<https://example.org/alpha>", "b", default),
            ("c", "<https://example.org/alpha>", "d", g.as_str()),
        ] {
            assert!(
                s.insert(&iri(subject), predicate, &iri(object), graph)
                    .is_some()
            );
        }
        let key = |s: &RelationStore, p: PartitionRef<'_>| {
            (
                s.interner().resolve(p.predicate()).to_owned(),
                s.interner().resolve(p.graph()).to_owned(),
            )
        };

        // Everything free: every partition, lexical by (predicate, graph).
        let all: Vec<(String, String)> = s.partitions(None, None).map(|p| key(&s, p)).collect();
        assert_eq!(
            all,
            vec![
                ("<https://example.org/alpha>".to_owned(), String::new()),
                ("<https://example.org/alpha>".to_owned(), g.clone()),
                ("<https://example.org/mu>".to_owned(), g.clone()),
                ("<https://example.org/zeta>".to_owned(), String::new()),
            ]
        );

        // A free predicate with a FIXED graph visits only that graph's partitions.
        let in_g: Vec<(String, String)> = s
            .partitions(None, Some(id_of(&s, &g)))
            .map(|p| key(&s, p))
            .collect();
        assert_eq!(
            in_g,
            vec![
                ("<https://example.org/alpha>".to_owned(), g.clone()),
                ("<https://example.org/mu>".to_owned(), g.clone()),
            ]
        );

        // A fixed predicate with a free graph visits that predicate across graphs.
        let alpha = id_of(&s, "<https://example.org/alpha>");
        assert_eq!(
            s.partitions(Some(alpha), None)
                .map(|p| key(&s, p))
                .collect::<Vec<_>>(),
            vec![
                ("<https://example.org/alpha>".to_owned(), String::new()),
                ("<https://example.org/alpha>".to_owned(), g.clone()),
            ]
        );

        // BOTH fixed: at most one partition, and it is the one `partition` probes for —
        // the constant-predicate path, which never sweeps.
        let both: Vec<PartitionRef<'_>> = s
            .partitions(Some(alpha), Some(id_of(&s, default)))
            .collect();
        assert_eq!(both.len(), 1);
        assert_eq!(
            both[0].id(),
            s.partition(alpha, id_of(&s, default))
                .expect("the partition exists")
                .id()
        );
        assert!(format!("{:?}", s.partitions(Some(alpha), Some(alpha))).contains("single"));
        assert!(format!("{:?}", s.partitions(None, None)).contains("sweep"));

        // A never-interned position selects nothing at all.
        assert_eq!(s.partitions(Some(alpha), None).count(), 2);
        assert_eq!(s.partition_count(), 4);
    }

    /// The term-arena footprint counts each DISTINCT surface once, grows only on a fresh
    /// intern, and is the same whichever order the terms arrive in — so the evaluator's
    /// arena ceiling is a property of the data, not of an insertion sequence. Predicate
    /// and graph surfaces are counted too: they are terms.
    #[test]
    fn store_term_bytes_counts_each_distinct_surface_once() {
        let default = RelationStore::DEFAULT_GRAPH;
        let mut s = RelationStore::new();
        assert_eq!(s.term_bytes(), 0);
        assert!(s.interner().is_empty());

        s.insert(&iri("a"), KNOWS, &iri("b"), default);
        let after_first = s.term_bytes();
        assert_eq!(
            after_first,
            iri("a").len() + iri("b").len() + KNOWS.len(),
            "the default graph's surface is empty, so it adds no bytes"
        );

        // A repeat of both terms under a second predicate interns only the predicate.
        s.insert(&iri("a"), LIKES, &iri("b"), default);
        assert_eq!(s.term_bytes(), after_first + LIKES.len());
        let after_second = s.term_bytes();

        // A new term adds exactly its own bytes.
        s.insert(&iri("a"), KNOWS, &iri("c"), default);
        assert_eq!(s.term_bytes(), after_second + iri("c").len());
        assert_eq!(s.term_bytes(), s.interner().byte_len());

        // The same quads in the opposite order reach the same total.
        let mut reversed = RelationStore::new();
        reversed.insert(&iri("a"), KNOWS, &iri("c"), default);
        reversed.insert(&iri("a"), LIKES, &iri("b"), default);
        reversed.insert(&iri("a"), KNOWS, &iri("b"), default);
        assert_eq!(reversed.term_bytes(), s.term_bytes());
    }

    /// Resolving an id no dictionary minted is a programming error, reported as a panic
    /// rather than a silent wrong surface.
    #[test]
    #[should_panic(expected = "must never cross store boundaries")]
    fn store_resolving_a_foreign_term_id_panics() {
        let s = sample_store();
        let _ = s.interner().resolve(TermId::from_index(999));
    }

    /// Emission-order guard: the partition table is a slot-indexed `Vec` and the key map
    /// is keyed by mint-ordered ids, so neither may leak. The consumer-facing
    /// enumerations — [`RelationStore::predicates`], [`RelationStore::graphs`] and the
    /// partition sweep — must all be lexical. Insert in deliberately anti-lexical order
    /// and assert the output is lexical.
    #[test]
    fn store_predicates_never_leak_mint_or_hash_order() {
        let mut s = RelationStore::new();
        for pred in [
            "<https://example.org/zeta>",
            "<https://example.org/mu>",
            "<https://example.org/alpha>",
        ] {
            assert!(
                s.insert(&iri("x"), pred, &iri("y"), RelationStore::DEFAULT_GRAPH)
                    .is_some()
            );
        }
        let preds: Vec<&str> = s.predicates().collect();
        assert_eq!(
            preds,
            vec![
                "<https://example.org/alpha>",
                "<https://example.org/mu>",
                "<https://example.org/zeta>"
            ],
            "predicates() must be lexical — partition mint order must never leak"
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

    /// `facts_sorted` is the lexical projection: sorted
    /// `(subject, predicate, object, graph)`, covering every partition, with no duplicate
    /// row.
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
                graph: RelationStore::DEFAULT_GRAPH.to_owned(),
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

    /// The determinism contract, property style: the ORDER in which quads are inserted
    /// cannot affect any lexical observable of the store. Over many deterministic
    /// permutations of a corpus large enough to force several batch seals and
    /// consolidations — and spread over several GRAPHS as well as several predicates, so
    /// the arity-4 partitioning is what is being permuted — `facts_sorted`, `predicates`,
    /// `graphs`, the partition sweep, `row_count`, `len_for` and every membership answer
    /// are identical.
    ///
    /// Row ids are deliberately NOT compared: a row id is mint order, which is a
    /// function of insertion order by construction and is never an emission order.
    #[test]
    fn store_insertion_order_does_not_affect_lexical_output() {
        let preds = ["<https://example.org/p>", "<https://example.org/q>"];
        let graphs = [
            RelationStore::DEFAULT_GRAPH.to_owned(),
            iri("g1"),
            iri("g2"),
        ];
        // 150 quads over two predicates and three graphs: past the seal threshold, so
        // batches seal and consolidate at different points under different orders.
        let corpus: Vec<(usize, usize, usize, usize)> = (0..150)
            .map(|i| (i % 2, i % 3, i % 7, i % 23))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let quads: Vec<(String, String, String, String)> = corpus
            .iter()
            .map(|&(p, g, sub, obj)| {
                (
                    iri(&format!("s{sub}")),
                    preds[p].to_owned(),
                    iri(&format!("o{obj}")),
                    graphs[g].clone(),
                )
            })
            .collect();

        let build = |order: &[(String, String, String, String)]| {
            let mut s = RelationStore::new();
            for (sub, p, obj, g) in order {
                s.insert(sub, p, obj, g);
            }
            s
        };

        let partition_keys = |s: &RelationStore| -> Vec<(String, String)> {
            s.partitions(None, None)
                .map(|p| {
                    (
                        s.interner().resolve(p.predicate()).to_owned(),
                        s.interner().resolve(p.graph()).to_owned(),
                    )
                })
                .collect()
        };

        let reference = build(&quads);
        let reference_facts = reference.facts_sorted();
        let reference_preds: Vec<String> = reference.predicates().map(ToOwned::to_owned).collect();
        let reference_graphs: Vec<String> = reference.graphs().map(ToOwned::to_owned).collect();
        let reference_partitions = partition_keys(&reference);
        assert!(
            reference.row_count() >= TAIL_SEAL_THRESHOLD,
            "the corpus must be large enough to seal batches"
        );
        assert!(reference_partitions.len() >= 4, "several partitions");

        for seed in 0..24u64 {
            // Duplicated quads are absorbed whatever the order, so append a second
            // shuffled copy of the whole corpus.
            let mut order = permute(&quads, seed);
            order.extend(permute(&quads, seed ^ 0x5EED));
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
            assert_eq!(
                permuted.graphs().collect::<Vec<_>>(),
                reference_graphs,
                "seed {seed}: the graph sweep is insertion-order independent"
            );
            assert_eq!(
                partition_keys(&permuted),
                reference_partitions,
                "seed {seed}: the partition sweep is lexical, never mint-ordered"
            );
            assert_eq!(permuted.row_count(), reference.row_count(), "seed {seed}");
            assert_eq!(
                permuted.partition_count(),
                reference.partition_count(),
                "seed {seed}"
            );
            for p in &preds {
                for g in &graphs {
                    assert_eq!(
                        permuted.len_for(p, g),
                        reference.len_for(p, g),
                        "seed {seed}"
                    );
                }
            }
            for fact in &reference_facts {
                assert!(
                    permuted.contains(&fact.subject, &fact.predicate, &fact.object, &fact.graph),
                    "seed {seed}: every reference fact is present"
                );
            }
            assert!(
                !permuted.contains(
                    &iri("s0"),
                    preds[0],
                    &iri("never-seen"),
                    RelationStore::DEFAULT_GRAPH
                ),
                "seed {seed}: an absent quad stays absent"
            );
        }
    }

    /// Every access path answers the same question: for each bound shape the selected
    /// row SET is identical whatever the insertion order, including after seals.
    #[test]
    fn store_every_bound_shape_is_insertion_order_independent() {
        let pred = "<https://example.org/p>";
        let tuples: Vec<(String, String)> = (0..90)
            .map(|i| (iri(&format!("s{}", i % 5)), iri(&format!("o{}", i % 17))))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();

        let build = |order: &[(String, String)]| {
            let mut s = RelationStore::new();
            for (sub, obj) in order {
                s.insert(sub, pred, obj, RelationStore::DEFAULT_GRAPH);
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
