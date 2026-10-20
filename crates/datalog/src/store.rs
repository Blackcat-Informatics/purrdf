// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
//!   table is **never iterated**: it is keyed by the fixed-key `FixedHasher` and is only ever
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

mod columns;
mod index;
pub use columns::RowColumns;
pub use index::{RowIndex, RowIndexError};

use hashbrown::HashTable;

use crate::cursor::{LendingIterator, RowCursor, VALUE_OBJECT, VALUE_SUBJECT, ValueCursor};
use crate::dictionary::{
    DictionaryError, DictionaryTerm, TermDictionary, TermIdentity, try_map_row,
};
use crate::id::{PartitionId, RowId, TermId};
use purrdf_lex::allocation::{Memory, Resident, StorageError};

// ── Interners ───────────────────────────────────────────────────────────────────

/// Fixed-key hash of a borrowed surface, for every borrowed-key probe in this crate.
///
/// The key is fixed (`FixedHasher`'s compile-time keys — never ambient
/// entropy, which does not exist on `wasm32-unknown-unknown`) and is never persisted:
/// determinism comes from insertion order and the sorted sweeps, never from this hash.
#[inline]
fn surface_hash(surface: &str) -> u64 {
    let mut hasher = purrdf_hash::fixed::FixedHasher::default();
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
    /// Reduced-width owner model, absent from the shipping representation.
    #[cfg(test)]
    term_limit: Option<u64>,
}

impl TermInterner {
    /// Intern `surface`, minting a new insertion-ordered id if it is new, else
    /// returning the existing id.
    pub fn intern(&mut self, surface: &str) -> Result<TermId, StorageError> {
        let hash = surface_hash(surface);
        let surfaces = &self.surfaces;
        if let Some(&id) = self
            .by_surface
            .find(hash, |&id| surfaces[id.resident_index()] == surface)
        {
            return Ok(id);
        }
        let limit = u64::MAX;
        #[cfg(test)]
        let limit = self.term_limit.unwrap_or(limit);
        let id = TermId::from_address_with_limit(self.surfaces.len(), limit)?;
        let bytes = self
            .bytes
            .checked_add(surface.len())
            .ok_or(StorageError::SizeOverflow)?;
        self.surfaces
            .try_reserve(1)
            .map_err(|_| StorageError::AllocationFailed)?;
        let surfaces = &self.surfaces;
        self.by_surface
            .try_reserve(1, |&id| surface_hash(&surfaces[id.resident_index()]))
            .map_err(|_| StorageError::AllocationFailed)?;
        let mut resident = Resident;
        let value = Memory::new(&mut resident).string(surface)?;
        self.surfaces.push(value);
        self.bytes = bytes;
        let surfaces = &self.surfaces;
        self.by_surface
            .insert_unique(hash, id, |&id| surface_hash(&surfaces[id.resident_index()]));
        Ok(id)
    }

    /// Copy the original insertion order through the checked mint home.
    fn try_clone(&self) -> Result<Self, StorageError> {
        let mut copy = Self::default();
        #[cfg(test)]
        {
            copy.term_limit = self.term_limit;
        }
        for surface in &self.surfaces {
            copy.intern(surface)?;
        }
        Ok(copy)
    }

    /// The id of the term with this surface, if already interned; never inserts.
    pub fn lookup(&self, surface: &str) -> Option<TermId> {
        let hash = surface_hash(surface);
        self.by_surface
            .find(hash, |&id| self.surfaces[id.resident_index()] == surface)
            .copied()
    }

    /// Undo only the new lexical identities of a failed enclosing insertion.
    fn rollback_to(&mut self, checkpoint: usize) {
        while self.surfaces.len() > checkpoint {
            let index = self.surfaces.len() - 1;
            let value = &self.surfaces[index];
            let entry = self
                .by_surface
                .find_entry(surface_hash(value), |id| id.resident_index() == index)
                .expect("every retained lexical identity has its original index entry");
            entry.remove();
            self.bytes -= value.len();
            self.surfaces.pop();
        }
    }

    /// The lexical surface `id` was interned from.
    ///
    /// # Panics
    ///
    /// Panics if `id` was not minted by this dictionary. Ids are per-store handles,
    /// so a foreign id is a programming error, never a data state.
    pub fn resolve(&self, id: TermId) -> &str {
        self.surfaces.get(id.resident_index()).map_or_else(
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

impl TermDictionary for TermInterner {
    type Id = TermId;
    type Error = Infallible;
    type Lexical<'a> = &'a str;
    fn lookup_term(&self, surface: &str) -> Result<Option<TermId>, Infallible> {
        Ok(self.lookup(surface))
    }
    fn resolve_term(&self, id: TermId) -> Result<&str, Infallible> {
        Ok(self.resolve(id))
    }
    fn compare_terms(
        &self,
        left: DictionaryTerm<'_, TermId>,
        right: DictionaryTerm<'_, TermId>,
    ) -> Result<Ordering, Infallible> {
        crate::dictionary::compare_lexical_terms(self, left, right)
    }
    fn with_interned<T, const N: usize>(
        &mut self,
        terms: [DictionaryTerm<'_, TermId>; N],
        validate: impl FnOnce(&Self, [TermId; N]) -> Result<T, DictionaryError<Infallible>>,
    ) -> Result<T, DictionaryError<Infallible>> {
        let checkpoint = self.len();
        let result = (|| {
            let ids = try_map_row(terms, |term| {
                Ok::<_, StorageError>(match term {
                    DictionaryTerm::Id(id) => id,
                    DictionaryTerm::Surface(surface) => self.intern(surface)?,
                })
            })?;
            validate(self, ids)
        })();
        if result.is_err() {
            self.rollback_to(checkpoint);
        }
        result
    }
    fn check_read(&self) -> Result<(), Infallible> {
        Ok(())
    }
    fn term_count(&self) -> u64 {
        u64::try_from(self.len()).expect("resident term count fits native64")
    }
    fn resident_bytes(&self) -> usize {
        self.byte_len()
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
pub enum Bound<I: TermIdentity = TermId> {
    /// No position bound — every tuple of the relation.
    Any,
    /// Subject bound to this interned term.
    Subject(I),
    /// Object bound to this interned term.
    Object(I),
    /// Both positions bound, `(subject, object)`, to these interned terms.
    Both(I, I),
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

impl purrdf_hash::error::StructuredError for WeightError {
    purrdf_hash::error_visit! {
        Overflow { lhs, rhs } => "datalog-weight-overflow" { "lhs" => Signed(i128::from(*lhs)), "rhs" => Signed(i128::from(*rhs)) },
    }
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
fn gallop_lower_bound<I: TermIdentity>(xs: &[I], from: usize, key: I) -> usize {
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
/// cleanly. A permutation of resident `usize` positions, never a hash map or per-key
/// `Vec`: one address per row, materialised only when an object bound is actually
/// probed.
#[derive(Debug, Clone, Default)]
struct ObjectIndex {
    /// Row positions of the batch, sorted by `(object_id, subject_id)`.
    perm: Box<[usize]>,
}

/// One immutable sorted batch: a relation's `(subject, object)` rows in canonical
/// `(subject_id, object_id)` order, stored as flat dense-id columns.
///
/// The primary sort is subject-major, so a subject-bound probe gallops the `subj`
/// column to the term's contiguous run with NO secondary structure. The
/// `(object, subject)` access path is the lazily-built [`ObjectIndex`]. Generic over
/// the weight monoid `W` (the Z-set seam); the production instantiation is `W = ()`.
#[derive(Debug, Clone)]
pub(crate) struct Batch<W: Weight = (), I: TermIdentity = TermId> {
    /// Subject column, ascending (subject-major within the `(subject, object)` sort).
    subj: Vec<I>,
    /// Object column, ascending within each subject run.
    obj: Vec<I>,
    /// Store-global dense [`RowId`] per row, parallel to the columns.
    row_id: Vec<RowId>,
    /// Multiplicity per row; `Vec<()>` is zero-sized under set semantics.
    weight: Vec<W>,
    /// The lazily-built `(object, subject)` access path (built on first object probe).
    object_index: OnceLock<ObjectIndex>,
}

impl<W: Weight, I: TermIdentity> Batch<W, I> {
    /// Build a batch from rows ALREADY sorted ascending by `(subject_id, object_id)`
    /// and free of duplicate keys. Weights default to [`Weight::UNIT`].
    fn from_sorted(rows: &[(I, I, RowId)]) -> Self {
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
    pub(crate) fn row_at(&self, p: usize) -> (I, I, RowId) {
        (self.subj[p], self.obj[p], self.row_id[p])
    }

    /// The `[lo, hi)` column-position run whose subject is `s`, located by galloping
    /// the sorted `subj` column (subject grouping is contiguous in the primary sort).
    pub(crate) fn subject_run(&self, s: I) -> (usize, usize) {
        let lo = gallop_lower_bound(&self.subj, 0, s);
        // `hi` is the first position past `s`'s contiguous run — a binary search of the
        // sorted suffix, so a `Both` probe stays O(log n) rather than O(run length).
        let hi = lo + self.subj[lo..].partition_point(|&x| x <= s);
        (lo, hi)
    }

    /// The single column position of the unique `(s, o)` row, if present: gallop the
    /// subject run, then binary-search its ascending `obj` sub-column for `o`.
    pub(crate) fn both_pos(&self, s: I, o: I) -> Option<usize> {
        let (lo, hi) = self.subject_run(s);
        let run = &self.obj[lo..hi];
        run.binary_search(&o).ok().map(|off| lo + off)
    }

    /// Whether the unique `(s, o)` key is present in this batch.
    fn contains(&self, s: I, o: I) -> bool {
        self.both_pos(s, o).is_some()
    }

    /// The selected range in the original object permutation, without borrowing
    /// its slice into a cursor's retained position.
    pub(crate) fn object_run(&self, o: I) -> (usize, usize) {
        let perm = self.object_order();
        let lo = perm.partition_point(|&p| self.obj[p] < o);
        let hi = perm.partition_point(|&p| self.obj[p] <= o);
        (lo, hi)
    }

    /// Every batch position in `(object_id, subject_id)` order.
    ///
    /// The same lazy, memoized permutation backs object-bound binary probes and
    /// object-major trie levels; it is built once and shared by both operators.
    pub(crate) fn object_order(&self) -> &[usize] {
        &self
            .object_index
            .get_or_init(|| {
                let mut perm: Vec<usize> = (0..self.len()).collect();
                // Sort positions by (object_id, subject_id) — the secondary access
                // order. `(object, subject)` keys are unique within a batch (the
                // primary sort is key-disjoint), so no equal elements exist whose order
                // would need preserving: the unstable sort is a pure win (no scratch
                // allocation, lower constants) and cannot introduce nondeterminism.
                index::sort_row_positions(&mut perm, &[&self.subj, &self.obj], &[1, 0]);
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
fn merge_batches<W: Weight, I: TermIdentity>(
    left: &Batch<W, I>,
    right: &Batch<W, I>,
) -> Result<Batch<W, I>, W::Error> {
    let cap = left.len() + right.len();
    let mut subj = Vec::with_capacity(cap);
    let mut obj = Vec::with_capacity(cap);
    let mut row_id = Vec::with_capacity(cap);
    let mut weight = Vec::with_capacity(cap);
    let (mut i, mut j) = (0usize, 0usize);
    let push = |subj: &mut Vec<I>,
                obj: &mut Vec<I>,
                row_id: &mut Vec<RowId>,
                weight: &mut Vec<W>,
                b: &Batch<W, I>,
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

/// The mutable tail of a [`Relation`]: the current epoch's rows in insertion
/// order, held as three parallel columns.
///
/// The key columns are separate so the duplicate probe on insert
/// ([`Self::contains`]) reads two contiguous identity columns rather than
/// `(subject, object, row)` tuples: over the tuples the compares stride three
/// identities apart and the loop stays scalar, over the columns they pack.
/// Row `i` is `(subjects[i], objects[i], rows[i])`; the three columns
/// always have the same length.
#[derive(Debug, Clone)]
pub(crate) struct Tail<I: TermIdentity = TermId> {
    data: RowColumns<I, 2>,
}

impl<I: TermIdentity> Default for Tail<I> {
    fn default() -> Self {
        Self {
            data: RowColumns::default(),
        }
    }
}

impl<I: TermIdentity> Tail<I> {
    /// Reserve all three columns before publishing any part of a row.
    fn reserve_row(&mut self) -> Result<(), StorageError> {
        self.data.reserve_rows(1)
    }

    /// Publish row `(subject, object, row)` after [`Self::reserve_row`] succeeds.
    fn push(&mut self, subject: I, object: I, row: RowId) {
        self.data.push_reserved([subject, object], row);
    }

    /// The number of rows.
    #[inline]
    pub(crate) fn len(&self) -> usize {
        self.data.len()
    }

    /// Whether the tail holds no row.
    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Row `position`, as `(subject, object, row)`.
    ///
    /// # Panics
    ///
    /// When `position` is not below [`Self::len`].
    #[inline]
    pub(crate) fn row(&self, position: usize) -> (I, I, RowId) {
        let ([subject, object], row) = self
            .data
            .row(position)
            .expect("tail row position is occupied");
        (subject, object, row)
    }

    /// The rows in insertion order, as `(subject, object, row)`.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (I, I, RowId)> + '_ {
        self.data
            .iter()
            .map(|([subject, object], row)| (subject, object, row))
    }

    /// Whether some row has the key `(subject, object)`.
    ///
    /// Every row is tested and the answers are counted, with no exit on the
    /// first hit: a loop that can leave early is not vectorized, and the tail is
    /// bounded by [`TAIL_SEAL_THRESHOLD`], so on a target with vector compares
    /// the whole scan is a handful of packed compares.
    #[allow(
        clippy::needless_bitwise_bool,
        reason = "the non-short-circuit `&` is the point: a lazy `&&` puts a branch in the \
                  lane loop, and the compares of both key columns must be evaluated every row"
    )]
    fn contains(&self, subject: I, object: I) -> bool {
        let subjects = self.data.column(0);
        let objects = self.data.column(1);
        let len = subjects.len().min(objects.len());
        // The tail seals at 64 rows, so its match count fits in one u32 even
        // when each key's identity is wider than a machine word.
        let mut matches = 0u32;
        for position in 0..len {
            matches += u32::from((subjects[position] == subject) & (objects[position] == object));
        }
        matches != 0
    }

    /// Empty the tail, returning its rows in insertion order.
    fn take_rows(&mut self) -> Vec<(I, I, RowId)> {
        let rows = self.iter().collect();
        self.data.clear();
        rows
    }
}

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
#[derive(Debug, Clone)]
pub(crate) struct Relation<I: TermIdentity = TermId> {
    /// Immutable sorted batches (each `(subject_id, object_id)`-ordered and
    /// key-disjoint), newest last. Empty for a tail-only (never-sealed) relation.
    batches: Vec<Batch<(), I>>,
    /// The mutable tail: `(subject_id, object_id, row_id)` rows of the current epoch,
    /// in insertion order, sealed once it reaches [`TAIL_SEAL_THRESHOLD`].
    tail: Tail<I>,
    /// The number of rows across batches + tail (the dense per-relation row count).
    len: usize,
    /// The store-global id of the newest row, or `None` for an empty relation. Row ids are
    /// minted in ascending order, so this is the largest id the relation holds.
    newest_row: Option<RowId>,
}

impl<I: TermIdentity> Default for Relation<I> {
    fn default() -> Self {
        Self {
            batches: Vec::new(),
            tail: Tail::default(),
            len: 0,
            newest_row: None,
        }
    }
}

impl<I: TermIdentity> Relation<I> {
    /// Publish an already-admitted, absent `(subject_id, object_id)` row with
    /// its original store-assigned `row_id`.
    ///
    /// Dedup is a GALLOPING probe of every sorted batch plus a linear scan of the small
    /// tail — no per-row hashing, no postings maintenance. A new row is appended to the
    /// unsorted tail; when the tail reaches [`TAIL_SEAL_THRESHOLD`] it is sealed into a
    /// sorted batch and the batch log consolidates.
    fn insert(&mut self, s_id: I, o_id: I, row_id: RowId) {
        self.tail.push(s_id, o_id, row_id);
        self.len += 1;
        self.newest_row = Some(self.newest_row.map_or(row_id, |newest| newest.max(row_id)));
        if self.tail.len() >= TAIL_SEAL_THRESHOLD {
            self.seal();
        }
    }

    /// Whether a tuple with these interned terms is present — a galloping probe of each
    /// sorted batch plus a linear scan of the tail (no hashing).
    fn contains(&self, subject: I, object: I) -> bool {
        self.batches.iter().any(|b| b.contains(subject, object))
            || self.tail.contains(subject, object)
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
        let mut rows = self.tail.take_rows();
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
    fn select(&self, bound: Bound<I>) -> RowCursor<'_, I> {
        RowCursor::new(self, bound)
    }

    /// The batches of this relation, newest last — the cursor's per-batch sub-runs.
    #[inline]
    pub(crate) fn batches(&self) -> &[Batch<(), I>] {
        &self.batches
    }

    /// The unsorted tail rows — the cursor's final (linear-scanned) leg.
    #[inline]
    pub(crate) fn tail(&self) -> &Tail<I> {
        &self.tail
    }
}

mod arity_seal {
    pub trait Sealed {}
    impl Sealed for super::Arity<3> {}
    impl Sealed for super::Arity<4> {}
}

/// Closed compile-time row arity, with no run-time shape tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Arity<const N: usize>;

/// Physical key and row coordinates of the supported triple and quad stores.
///
/// Three-position keys hold one predicate identity. Four-position keys retain
/// the predicate and graph pair. Both use the same subject/object arrangements.
pub trait StoreShape<const N: usize>: arity_seal::Sealed {
    /// The physically retained partition key.
    type Key<I: TermIdentity>: TermIdentity;
    /// No field storage for triples; the original owned graph surface for quads.
    type Graph: fmt::Debug + Clone + Eq + Ord + Send + Sync;
    /// RDF positions that identify a partition, in lexical sweep order.
    const KEY_POSITIONS: &'static [usize];
    /// Original dictionary mint order, expressed as RDF position indices.
    const MINT_ORDER: [usize; N];
    /// Project a complete row to its physical key.
    fn key<I: TermIdentity>(row: &[I; N]) -> Self::Key<I>;
    /// Obtain a key only when every partition coordinate is bound.
    fn known_key<I: TermIdentity>(row: &[Option<I>; N]) -> Option<Self::Key<I>>;
    /// Read a key coordinate by its key-order position.
    fn key_term<I: TermIdentity>(key: Self::Key<I>, position: usize) -> I;
    /// Reconstruct a row from its original key and subject/object identities.
    fn row<I: TermIdentity>(subject: I, key: Self::Key<I>, object: I) -> [I; N];
    /// Own exactly the shape's lexical positions at publication.
    fn fact(terms: [String; N]) -> Fact<Self::Graph>;
    /// Borrow exactly the shape's lexical positions for proof and model identities.
    fn fact_terms(fact: &Fact<Self::Graph>) -> [&str; N];
}

impl StoreShape<3> for Arity<3> {
    type Key<I: TermIdentity> = I;
    type Graph = ();
    const KEY_POSITIONS: &'static [usize] = &[1];
    const MINT_ORDER: [usize; 3] = [1, 0, 2];
    fn key<I: TermIdentity>(row: &[I; 3]) -> I {
        row[1]
    }
    fn known_key<I: TermIdentity>(row: &[Option<I>; 3]) -> Option<I> {
        row[1]
    }
    fn key_term<I: TermIdentity>(key: I, _position: usize) -> I {
        key
    }
    fn row<I: TermIdentity>(subject: I, key: I, object: I) -> [I; 3] {
        [subject, key, object]
    }
    fn fact([subject, predicate, object]: [String; 3]) -> ShapeFact<3> {
        Fact {
            subject,
            predicate,
            object,
            graph: (),
        }
    }
    fn fact_terms(fact: &ShapeFact<3>) -> [&str; 3] {
        [&fact.subject, &fact.predicate, &fact.object]
    }
}

impl StoreShape<4> for Arity<4> {
    type Key<I: TermIdentity> = (I, I);
    type Graph = String;
    const KEY_POSITIONS: &'static [usize] = &[1, 3];
    const MINT_ORDER: [usize; 4] = [1, 3, 0, 2];
    fn key<I: TermIdentity>(row: &[I; 4]) -> (I, I) {
        (row[1], row[3])
    }
    fn known_key<I: TermIdentity>(row: &[Option<I>; 4]) -> Option<(I, I)> {
        row[1].zip(row[3])
    }
    fn key_term<I: TermIdentity>(key: (I, I), position: usize) -> I {
        if position == 0 { key.0 } else { key.1 }
    }
    fn row<I: TermIdentity>(subject: I, key: (I, I), object: I) -> [I; 4] {
        [subject, key.0, object, key.1]
    }
    fn fact([subject, predicate, object, graph]: [String; 4]) -> ShapeFact<4> {
        Fact {
            subject,
            predicate,
            object,
            graph,
        }
    }
    fn fact_terms(fact: &ShapeFact<4>) -> [&str; 4] {
        [&fact.subject, &fact.predicate, &fact.object, &fact.graph]
    }
}

/// The exact key type of an admitted static row shape.
pub type StoreKey<I, const N: usize> = <Arity<N> as StoreShape<N>>::Key<I>;

/// A lexical fact with exactly the selected shape's owned coordinates.
pub type ShapeFact<const N: usize> = Fact<<Arity<N> as StoreShape<N>>::Graph>;

// ── The store ───────────────────────────────────────────────────────────────────

/// One projected row of the store: a `(subject, predicate, object, graph)` quad of
/// lexical surfaces.
///
/// The derived [`Ord`] is `(subject, predicate, object, graph)` lexical order — the
/// emission order [`RelationStore::facts_sorted`] establishes, and the only fact ordering
/// this crate ever produces. The graph sorts LAST so a dataset confined to one graph keeps
/// exactly the fact order it had before the position existed.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Fact<G = String> {
    /// The subject term's lexical surface.
    pub subject: String,
    /// The predicate term's lexical surface — an IRI predicate renders as `<iri>`, the
    /// same bytes it would render to in any other position.
    pub predicate: String,
    /// The object term's lexical surface.
    pub object: String,
    /// The graph term's lexical surface; EMPTY for the default graph
    /// ([`RelationStore::DEFAULT_GRAPH`]).
    pub graph: G,
}

impl<G: purrdf_hash::error::StructuredError> purrdf_hash::error::StructuredError for Fact<G> {
    purrdf_hash::error_visit! {
        @record self { subject, predicate, object, graph } => "datalog-fact" {
            "subject" => Text(&self.subject), "predicate" => Text(&self.predicate),
            "object" => Text(&self.object), "graph" => Record(&self.graph)
        }
    }
}

/// One addressed partition of the store's arity-4 relation: the `(subject, object)`
/// arrangement of a single `(predicate, graph)` key.
///
/// A `PartitionRef` is the unit a join actually scans. Obtaining one is where the
/// predicate/graph positions are resolved; everything after it is the store's ordinary
/// galloping `(subject, object)` access, so an atom whose predicate happens to be a
/// variable pays for the extra partitions it visits and for nothing else.
#[derive(Debug, Clone, Copy)]
pub struct PartitionRef<'a, I: TermIdentity = TermId, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    /// The partition's dense slot handle.
    id: PartitionId,
    /// The interned predicate surface this partition is keyed by.
    key: StoreKey<I, N>,
    /// The partition's arrangement.
    relation: &'a Relation<I>,
}

impl<'a, I: TermIdentity, const N: usize> PartitionRef<'a, I, N>
where
    Arity<N>: StoreShape<N>,
{
    /// The partition's dense slot handle.
    pub fn id(self) -> PartitionId {
        self.id
    }

    /// The interned predicate this partition is keyed by — the value a free predicate
    /// position binds to.
    pub fn predicate(self) -> I {
        Arity::<N>::key_term(self.key, 0)
    }

    /// Reconstruct the original row without a graph coordinate on triple stores.
    pub fn row(self, subject: I, object: I) -> [I; N] {
        Arity::<N>::row(subject, self.key, object)
    }

    /// The number of distinct `(subject, object)` rows in this partition.
    pub fn row_count(self) -> usize {
        self.relation.row_count()
    }

    /// Count an indexed pattern without constructing a new historical access path.
    /// A cold object permutation is unknown; subject/both bounds use primary runs,
    /// and the mutable tail remains bounded by the sealing threshold.
    pub(crate) fn indexed_count(self, bound: Bound<I>) -> Option<usize> {
        if matches!(bound, Bound::Object(_))
            && self
                .relation
                .batches()
                .iter()
                .any(|batch| batch.object_index.get().is_none())
        {
            return None;
        }
        Some(match bound {
            Bound::Any => self.row_count(),
            _ => self.select(bound).remaining_count(),
        })
    }

    /// Whether this partition holds a row whose store-global id is at least `index` —
    /// one O(1) comparison against the newest row id, so a semi-naive round can tell that
    /// a partition gained nothing since row `index` without scanning it.
    pub fn has_row_from(self, index: usize) -> bool {
        self.relation
            .newest_row
            .is_some_and(|newest| newest.resident_index() >= index)
    }

    /// A galloping lending [`RowCursor`] over the id rows selected by `bound`.
    pub fn select(self, bound: Bound<I>) -> RowCursor<'a, I> {
        self.relation.select(bound)
    }

    /// The original immutable arrangement used by an owned native scan frame.
    pub(crate) fn arrangement(self) -> &'a Relation<I> {
        self.relation
    }

    /// A globally subject-value-ordered trie-level cursor; `other` optionally fixes the
    /// object position.
    pub fn values_subject(self, other: Option<I>) -> ValueCursor<'a, VALUE_SUBJECT, I> {
        ValueCursor::new(self.relation, other)
    }

    /// The object-ordered sibling of [`Self::values_subject`]; `other` optionally fixes
    /// the subject position.
    pub fn values_object(self, other: Option<I>) -> ValueCursor<'a, VALUE_OBJECT, I> {
        ValueCursor::new(self.relation, other)
    }
}

impl<I: TermIdentity> PartitionRef<'_, I, 4> {
    /// The interned graph this partition is keyed by.
    pub fn graph(self) -> I {
        self.key.1
    }
}

/// How a [`Partitions`] sweep is being served.
#[derive(Debug, Clone, Copy)]
enum PartitionMode<I: TermIdentity, const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    /// Both key positions are known, so at most ONE partition can match and it was
    /// located by an ordered-map probe. This is the constant-predicate hot path: no
    /// sweep, no filter, straight to the same arrangement a predicate-keyed store would
    /// have handed back.
    Single(Option<PartitionId>),
    /// At least one key position is free: walk the store's lexical partition order,
    /// keeping the partitions whose key agrees with the known positions.
    Sweep {
        /// Bound RDF positions; only the shape's key coordinates select partitions.
        positions: [Option<I>; N],
        /// The next index into the store's lexical partition order.
        next: usize,
    },
}

/// The partitions matching a `(predicate, graph)` selection, in LEXICAL
/// `(predicate surface, graph surface)` order.
///
/// Allocation-free in both modes, so an atom's partition resolution never materialises a
/// `Vec` — see [`RelationStore::partitions`].
pub struct Partitions<'a, D: TermDictionary = TermInterner, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    store: &'a RelationStore<D, N>,
    delta: Option<&'a DeltaRun<D::Id>>,
    position: PartitionPosition<D::Id, N>,
}

/// Owned selection progress over the original store and optional delta run.
/// The execution owner retains both unchanged; this is not a foreign-source token.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PartitionPosition<I: TermIdentity, const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    mode: PartitionMode<I, N>,
    #[cfg(test)]
    visited: usize,
}

impl<D: TermDictionary, const N: usize> fmt::Debug for Partitions<'_, D, N>
where
    Arity<N>: StoreShape<N>,
{
    /// Prints the sweep's shape, never the borrowed store.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.position.mode {
            PartitionMode::Single(found) => f
                .debug_struct("Partitions")
                .field("mode", &"single")
                .field("found", &found.is_some())
                .finish(),
            PartitionMode::Sweep {
                positions, next, ..
            } => f
                .debug_struct("Partitions")
                .field("mode", &"sweep")
                .field("predicate", &positions[1])
                .field("graph", &positions.get(3).copied().flatten())
                .field("next", next)
                .finish(),
        }
    }
}

impl<'a, D: TermDictionary, const N: usize> Iterator for Partitions<'a, D, N>
where
    Arity<N>: StoreShape<N>,
{
    type Item = PartitionRef<'a, D::Id, N>;

    fn next(&mut self) -> Option<Self::Item> {
        self.position.next(self.store, self.delta)
    }
}

impl<I: TermIdentity, const N: usize> PartitionPosition<I, N>
where
    Arity<N>: StoreShape<N>,
{
    /// Resolve a fixed key once; free keys retain their original selection bounds.
    pub(crate) fn new<D: TermDictionary<Id = I>>(
        store: &RelationStore<D, N>,
        positions: [Option<I>; N],
    ) -> Self {
        let mode = if Arity::<N>::known_key(&positions).is_some() {
            PartitionMode::Single(store.partition_for(&positions).map(PartitionRef::id))
        } else {
            PartitionMode::Sweep { positions, next: 0 }
        };
        Self {
            mode,
            #[cfg(test)]
            visited: 0,
        }
    }

    /// Continue without replaying previously visited partition keys.
    pub(crate) fn next<'a, D: TermDictionary<Id = I>>(
        &mut self,
        store: &'a RelationStore<D, N>,
        delta: Option<&'a DeltaRun<I>>,
    ) -> Option<PartitionRef<'a, I, N>> {
        match &mut self.mode {
            PartitionMode::Single(found) => {
                let partition = store.partition_at(found.take()?.resident_index());
                delta.map_or(Some(partition), |run| run.project(partition))
            }
            PartitionMode::Sweep { positions, next } => {
                let order = delta.map_or(store.order.as_slice(), |run| run.order.as_slice());
                let predicates = delta.map_or(&store.by_predicate, |run| &run.by_predicate);
                let order = positions[1].map_or(order, |predicate| {
                    predicates.get(&predicate).map_or(&[], Vec::as_slice)
                });
                while let Some(&slot) = order.get(*next) {
                    *next += 1;
                    #[cfg(test)]
                    {
                        self.visited += 1;
                    }
                    let (_, key) = store.keys[slot];
                    if Arity::<N>::KEY_POSITIONS.iter().enumerate().all(
                        |(coordinate, &position)| {
                            positions[position].is_none_or(|wanted| {
                                wanted == Arity::<N>::key_term(key, coordinate)
                            })
                        },
                    ) {
                        let partition = store.partition_at(slot);
                        return Some(delta.map_or(partition, |run| {
                            run.project(partition)
                                .expect("delta order names an admitted arrangement")
                        }));
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
#[derive(Debug, Clone)]
pub struct RelationStore<D: TermDictionary = TermInterner, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    /// The store's term dictionary, shared by every position of every partition. This is
    /// the persistent term arena: never reset within the store's lifetime, because a
    /// [`TermDictionary::Id`] handed out in round 1 must still resolve in round 40.
    interner: D,
    /// Explicit immutable native base; capture strips any older base, so depth is one.
    base: Option<Box<Self>>,
    /// The partitions, indexed by dense [`PartitionId`] slot.
    relations: Vec<Relation<D::Id>>,
    /// `keys[slot]` retains the original partition id and `(predicate, graph)` key.
    keys: Vec<(PartitionId, StoreKey<D::Id, N>)>,
    /// Key → slot. Probed, NEVER iterated: its order is term mint order, which carries no
    /// lexical meaning. Every sweep reads `order` instead.
    by_key: BTreeMap<StoreKey<D::Id, N>, usize>,
    /// The partition slots in LEXICAL `(predicate surface, graph surface)` order — the
    /// single sweep order, maintained on partition creation so no sweep has to sort.
    order: Vec<usize>,
    /// Predicate identity to its graph partitions, in the same lexical order.
    /// Never enumerate the hash table; only probe it by an original identity.
    by_predicate: purrdf_core::FastMap<D::Id, Vec<usize>>,
    /// The number of rows inserted so far across ALL partitions — equivalently, the next
    /// dense [`RowId`] slot to assign. Row ids are minted `0, 1, 2, …` in store-wide
    /// insertion order, so at any point the live rows are exactly `0..row_count`. This
    /// is the single row-id source, and the id never enters a provenance identity.
    row_count: usize,
    /// Every row's original `(row id, partition id, subject, object)` — the row
    /// ORDER the arrangements do not keep, so [`Self::rows_from`] reads the rows a round
    /// added without scanning a partition.
    row_log: Vec<(RowId, PartitionId, D::Id, D::Id)>,
    /// A permanently-empty partition handed to [`select`](Self::select) on a partition
    /// miss, so an unknown key yields an empty [`RowCursor`] with NO `Option` branch on
    /// the per-row scan. Never inserted into.
    empty: Relation<D::Id>,
    /// Reduced-width owner model, absent from shipping partition storage.
    #[cfg(test)]
    partition_limit: Option<u64>,
}

/// Storage reservations held before publishing one new row.
struct RowAdmission<I: TermIdentity> {
    row_id: RowId,
    partition_id: PartitionId,
    new_relation: Relation<I>,
    /// An admitted directory bucket if this is the predicate's first partition.
    predicate_bucket: Option<Vec<usize>>,
}

/// An indexed slice of the append log, retaining original partition and row IDs.
/// It uses the same native arrangements as the accumulated store and owns no terms.
#[derive(Debug)]
pub(crate) struct DeltaRun<I: TermIdentity> {
    relations: purrdf_core::FastMap<PartitionId, Relation<I>>,
    order: Vec<usize>,
    by_predicate: purrdf_core::FastMap<I, Vec<usize>>,
}

impl<I: TermIdentity> DeltaRun<I> {
    pub(crate) fn project<'a, const N: usize>(
        &'a self,
        partition: PartitionRef<'a, I, N>,
    ) -> Option<PartitionRef<'a, I, N>>
    where
        Arity<N>: StoreShape<N>,
    {
        self.relations
            .get(&partition.id)
            .map(|relation| PartitionRef {
                relation,
                ..partition
            })
    }
}

/// Original subject/object identities and the newly published row, if not a duplicate.
pub type InsertedRow<I> = Option<(I, I, RowId)>;

impl<D: TermDictionary, const N: usize> RelationStore<D, N>
where
    Arity<N>: StoreShape<N>,
{
    /// Select a declared read authority. Base is checked by the executor before work.
    pub(crate) fn read_source(&self, source: crate::clause::ReadSource) -> &Self {
        match source {
            crate::clause::ReadSource::Evaluation => self,
            crate::clause::ReadSource::Base => self
                .base
                .as_deref()
                .expect("the evaluator admitted the rule's base read authority"),
        }
    }

    /// Whether an explicit immutable base was captured.
    pub(crate) const fn has_base(&self) -> bool {
        self.base.is_some()
    }

    /// All retained rows, including the immutable base's separate arrangements.
    pub(crate) fn retained_rows(&self) -> usize {
        self.row_count
            .saturating_add(self.base.as_ref().map_or(0, |base| base.row_count))
    }

    /// Construct an empty store using the original dictionary and identity space.
    /// No term is read, copied or remapped by construction.
    pub fn with_dictionary(interner: D) -> Self {
        Self {
            interner,
            base: None,
            relations: Vec::new(),
            keys: Vec::new(),
            by_key: BTreeMap::new(),
            order: Vec::new(),
            by_predicate: purrdf_core::FastMap::default(),
            row_count: 0,
            row_log: Vec::new(),
            empty: Relation::default(),
            #[cfg(test)]
            partition_limit: None,
        }
    }

    /// Insert original IDs from this dictionary without interning lexical copies.
    ///
    /// Only a newly created partition resolves its predicate and graph to maintain
    /// lexical sweep order. Row probes and joins use the supplied IDs unchanged.
    ///
    /// # Errors
    ///
    /// Preserves the original dictionary failure or storage refusal. Neither a
    /// source failure nor admission failure publishes a partial row or partition.
    pub fn insert_ids(
        &mut self,
        ids: [D::Id; N],
    ) -> Result<InsertedRow<D::Id>, DictionaryError<D::Error>> {
        self.insert_terms(ids.map(DictionaryTerm::Id))
    }

    /// Insert a mixed original-ID/fresh-surface quad through one admitted publication.
    ///
    /// # Errors
    /// Original dictionary errors and native storage refusals leave the row absent.
    /// New lexical identities are rolled back if their dependent validation refuses.
    pub fn insert_terms(
        &mut self,
        terms: [DictionaryTerm<'_, D::Id>; N],
    ) -> Result<InsertedRow<D::Id>, DictionaryError<D::Error>> {
        self.interner
            .check_read()
            .map_err(DictionaryError::Source)?;
        let mut known = [None; N];
        for (slot, term) in known.iter_mut().zip(terms) {
            *slot = match term {
                DictionaryTerm::Id(id) => Some(id),
                DictionaryTerm::Surface(surface) => self
                    .interner
                    .lookup_term(surface)
                    .map_err(DictionaryError::Source)?,
            };
        }
        self.interner
            .check_read()
            .map_err(DictionaryError::Source)?;
        let (subject, object) = (known[0], known[2]);
        let existing = Arity::<N>::known_key(&known).and_then(|key| self.by_key.get(&key).copied());
        if let (Some(slot), Some(subject), Some(object)) = (existing, subject, object)
            && self.relations[slot].contains(subject, object)
        {
            return Ok(None);
        }
        let admission = self.admit_row(existing, known[1])?;
        let ordered = Arity::<N>::MINT_ORDER
            .map(|position| known[position].map_or(terms[position], DictionaryTerm::Id));
        let keys = &self.keys;
        let order = &self.order;
        let (ids, position) = self.interner.with_interned(ordered, |dictionary, minted| {
            let mut ids = minted;
            for (index, position) in Arity::<N>::MINT_ORDER.into_iter().enumerate() {
                ids[position] = minted[index];
            }
            dictionary.check_read().map_err(DictionaryError::Source)?;
            let position = if existing.is_some() {
                0
            } else {
                partition_position::<D, N>(dictionary, keys, order, Arity::<N>::key(&ids))
                    .map_err(DictionaryError::Source)?
            };
            dictionary.check_read().map_err(DictionaryError::Source)?;
            Ok((ids, position))
        })?;
        Ok(Some(self.publish_row(ids, existing, position, admission)))
    }

    fn admit_row(
        &mut self,
        existing: Option<usize>,
        predicate: Option<D::Id>,
    ) -> Result<RowAdmission<D::Id>, StorageError> {
        let row_id = RowId::from_address(self.row_count)?;
        let partition_id = match existing {
            Some(slot) => self.keys[slot].0,
            None => {
                let limit = u64::MAX;
                #[cfg(test)]
                let limit = self.partition_limit.unwrap_or(limit);
                PartitionId::from_address_with_limit(self.relations.len(), limit)?
            }
        };
        self.row_log
            .try_reserve(1)
            .map_err(|_| StorageError::AllocationFailed)?;
        let mut new_relation = Relation::default();
        let mut predicate_bucket = None;
        if let Some(slot) = existing {
            self.relations[slot].tail.reserve_row()?;
        } else {
            self.relations
                .try_reserve(1)
                .map_err(|_| StorageError::AllocationFailed)?;
            self.keys
                .try_reserve(1)
                .map_err(|_| StorageError::AllocationFailed)?;
            self.order
                .try_reserve(1)
                .map_err(|_| StorageError::AllocationFailed)?;
            new_relation.tail.reserve_row()?;
            if let Some(slots) = predicate.and_then(|id| self.by_predicate.get_mut(&id)) {
                slots
                    .try_reserve(1)
                    .map_err(|_| StorageError::AllocationFailed)?;
            } else {
                self.by_predicate
                    .try_reserve(1)
                    .map_err(|_| StorageError::AllocationFailed)?;
                let mut slots = Vec::new();
                slots
                    .try_reserve(1)
                    .map_err(|_| StorageError::AllocationFailed)?;
                predicate_bucket = Some(slots);
            }
        }
        Ok(RowAdmission {
            row_id,
            partition_id,
            new_relation,
            predicate_bucket,
        })
    }

    fn publish_row(
        &mut self,
        ids: [D::Id; N],
        existing: Option<usize>,
        position: usize,
        admission: RowAdmission<D::Id>,
    ) -> (D::Id, D::Id, RowId) {
        let (subject_id, object_id) = (ids[0], ids[2]);
        let key = Arity::<N>::key(&ids);
        let RowAdmission {
            row_id,
            partition_id,
            new_relation,
            predicate_bucket,
        } = admission;
        let slot = if let Some(slot) = existing {
            slot
        } else {
            let predicate = Arity::<N>::key_term(key, 0);
            let previous = position
                .checked_sub(1)
                .and_then(|index| self.order.get(index))
                .copied()
                .filter(|&slot| Arity::<N>::key_term(self.keys[slot].1, 0) == predicate);
            let slot = self.relations.len();
            self.relations.push(new_relation);
            self.keys.push((partition_id, key));
            self.by_key.insert(key, slot);
            self.order.insert(position, slot);
            let slots = self.by_predicate.entry(predicate).or_insert_with(|| {
                predicate_bucket.expect("a new predicate's directory bucket was admitted")
            });
            let position = previous.map_or(0, |previous| {
                if slots.last() == Some(&previous) {
                    slots.len()
                } else {
                    slots
                        .iter()
                        .position(|&slot| slot == previous)
                        .expect("the preceding partition belongs to this predicate")
                        + 1
                }
            });
            slots.insert(position, slot);
            slot
        };
        self.relations[slot].insert(subject_id, object_id, row_id);
        self.row_count += 1;
        self.row_log
            .push((row_id, partition_id, subject_id, object_id));
        (subject_id, object_id, row_id)
    }
}

/// Lexical position of an absent partition in the original dictionary order.
fn partition_position<D: TermDictionary, const N: usize>(
    dictionary: &D,
    keys: &[(PartitionId, StoreKey<D::Id, N>)],
    order: &[usize],
    key: StoreKey<D::Id, N>,
) -> Result<usize, D::Error>
where
    Arity<N>: StoreShape<N>,
{
    let (mut lo, mut hi) = (0, order.len());
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        let (_, existing) = keys[order[mid]];
        let mut ordering = Ordering::Equal;
        for coordinate in 0..Arity::<N>::KEY_POSITIONS.len() {
            ordering = dictionary.compare_terms(
                DictionaryTerm::Id(Arity::<N>::key_term(existing, coordinate)),
                DictionaryTerm::Id(Arity::<N>::key_term(key, coordinate)),
            )?;
            if !ordering.is_eq() {
                break;
            }
        }
        if ordering.is_lt() {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    Ok(lo)
}

/// Pin every physical key coordinate before a lexical comparison.
fn resolve_key<D: TermDictionary, const N: usize>(
    dictionary: &D,
    key: StoreKey<D::Id, N>,
) -> Result<[Option<D::Lexical<'_>>; 2], D::Error>
where
    Arity<N>: StoreShape<N>,
{
    let mut terms = [None, None];
    for (coordinate, slot) in terms
        .iter_mut()
        .enumerate()
        .take(Arity::<N>::KEY_POSITIONS.len())
    {
        *slot = Some(dictionary.resolve_term(Arity::<N>::key_term(key, coordinate))?);
    }
    Ok(terms)
}

purrdf_hash::default_from_new!(RelationStore);

impl RelationStore<TermInterner, 3> {
    /// Capture the current triples as the immutable base for ordered rules.
    ///
    /// Later insertions and retractions affect only the evaluation model. The snapshot
    /// retains the same dictionary IDs; no graph term or partition is introduced.
    /// A second explicit capture replaces the old base instead of nesting snapshots.
    ///
    /// # Errors
    ///
    /// Returns the original typed dictionary, row-storage or owner-allocation refusal.
    pub fn with_base_snapshot(mut self) -> Result<Self, StorageError> {
        let mut base = Self::with_dictionary(self.interner.try_clone()?);
        for (_, row) in self.rows_from(0) {
            base.insert_row(row.map(|id| self.interner.resolve(id)))?;
        }
        let mut resident = Resident;
        self.base = Some(Memory::new(&mut resident).boxed(base)?);
        Ok(self)
    }
}

impl<const N: usize> RelationStore<TermInterner, N>
where
    Arity<N>: StoreShape<N>,
{
    /// Preserve IDs referenced by the immutable base across an ordered retraction.
    pub(crate) fn retraction_dictionary(&self) -> Result<TermInterner, StorageError> {
        if self.has_base() {
            self.interner.try_clone()
        } else {
            Ok(TermInterner::default())
        }
    }

    /// Move the immutable read authority to a rebuilt model without another copy.
    pub(crate) fn transfer_base(&mut self, destination: &mut Self) {
        destination.base = self.base.take();
    }
}

impl RelationStore {
    /// A fresh, empty store; [`Default`] delegates here.
    #[must_use]
    pub fn new() -> Self {
        Self::with_dictionary(TermInterner::default())
    }

    /// The lexical surface of the DEFAULT GRAPH: the EMPTY surface.
    ///
    /// RDF's default graph has no name, and PurRDF mints no vocabulary, so the store says
    /// "no name" rather than inventing one. No IRI surface (`<…>`) and no literal surface
    /// (`"…"`) is empty, so this cannot collide with a caller's term; it is the same
    /// denotation [`ClauseTerm::DefaultGraph`](crate::clause::ClauseTerm::DefaultGraph)
    /// renders to, which is what makes a clause constant and stored data comparable.
    pub const DEFAULT_GRAPH: &'static str = "";

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
    ) -> Result<Option<(TermId, TermId, RowId)>, StorageError> {
        self.insert_row([subject, predicate, object, graph])
    }
}

impl<D: TermDictionary, const N: usize> RelationStore<D, N>
where
    Arity<N>: StoreShape<N>,
{
    /// The rows from row `since` on, in row order, as `(subject, predicate, object,
    /// graph)` interned ids — the facts the store gained since it held `since` rows,
    /// read in time proportional to their number.
    pub fn rows_from(&self, since: usize) -> impl Iterator<Item = (RowId, [D::Id; N])> + '_ {
        let since = since.min(self.row_log.len());
        self.row_log[since..]
            .iter()
            .map(|&(row, partition, subject, object)| {
                let (_, key) = self.keys[partition.resident_index()];
                (row, Arity::<N>::row(subject, key, object))
            })
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
    pub fn interner(&self) -> &D {
        &self.interner
    }

    /// The bytes of interned term surfaces held by this store's dictionary — the
    /// term-arena footprint the evaluator's arena ceiling is measured against.
    ///
    /// Predicate and graph surfaces are terms too, so they are counted here exactly like a
    /// subject or an object: the ceiling measures the whole dictionary, not a subset of it.
    pub fn term_bytes(&self) -> usize {
        self.interner.resident_bytes().saturating_add(
            self.base
                .as_ref()
                .map_or(0, |base| base.interner.resident_bytes()),
        )
    }
}

impl RelationStore {
    /// The dense handle of the `(predicate, graph)` partition, if it exists; never
    /// creates one.
    pub fn partition_id(&self, predicate: &str, graph: &str) -> Option<PartitionId> {
        let (predicate, graph) = (self.term_id(predicate)?, self.term_id(graph)?);
        self.by_key
            .get(&(predicate, graph))
            .copied()
            .map(|slot| self.keys[slot].0)
    }
}

impl<D: TermDictionary> RelationStore<D> {
    /// The partition keyed by these interned positions, if it exists.
    ///
    /// One ordered-map probe on a pair of dense ids — the addressing step of every atom
    /// whose predicate and graph are constants or already-bound variables, and the reason
    /// carrying the predicate as data costs such an atom nothing.
    pub fn partition(&self, predicate: D::Id, graph: D::Id) -> Option<PartitionRef<'_, D::Id>> {
        self.partition_for(&[None, Some(predicate), None, Some(graph)])
    }

    /// Every partition matching the given positions, in LEXICAL
    /// `(predicate surface, graph surface)` order; `None` means the position is FREE.
    ///
    /// With both positions known this is the single [`Self::partition`] probe wrapped as a
    /// one-element iterator — no sweep at all. With either free it walks the maintained
    /// lexical order, which is why an unbound predicate is deterministic and is still a
    /// sequence of indexed probes rather than one undifferentiated scan.
    pub fn partitions(&self, predicate: Option<D::Id>, graph: Option<D::Id>) -> Partitions<'_, D> {
        self.partitions_for([None, predicate, None, graph])
    }
}

impl<D: TermDictionary, const N: usize> RelationStore<D, N>
where
    Arity<N>: StoreShape<N>,
{
    /// Resolve a fully bound partition key; subject/object coordinates do not select keys.
    pub fn partition_for(
        &self,
        positions: &[Option<D::Id>; N],
    ) -> Option<PartitionRef<'_, D::Id, N>> {
        self.by_key
            .get(&Arity::<N>::known_key(positions)?)
            .map(|&slot| self.partition_at(slot))
    }

    /// Match the shape's actual partition coordinates in lexical order, without allocation.
    pub fn partitions_for(&self, positions: [Option<D::Id>; N]) -> Partitions<'_, D, N> {
        self.select_partitions(positions, None)
    }

    /// Select the accumulated arrangements or the round's native indexed run.
    pub(crate) fn select_partitions<'a>(
        &'a self,
        positions: [Option<D::Id>; N],
        delta: Option<&'a DeltaRun<D::Id>>,
    ) -> Partitions<'a, D, N> {
        Partitions {
            store: self,
            delta,
            position: PartitionPosition::new(self, positions),
        }
    }

    /// Build one arrangement from the append-log slice, never from old rows.
    pub(crate) fn delta_run(
        &self,
        range: core::ops::Range<usize>,
    ) -> Result<DeltaRun<D::Id>, DictionaryError<D::Error>> {
        self.interner
            .check_read()
            .map_err(DictionaryError::Source)?;
        let mut run = DeltaRun {
            relations: purrdf_core::FastMap::default(),
            order: Vec::new(),
            by_predicate: purrdf_core::FastMap::default(),
        };
        for &(row, partition, subject, object) in &self.row_log[range] {
            if !run.relations.contains_key(&partition) {
                run.relations
                    .try_reserve(1)
                    .map_err(|_| StorageError::AllocationFailed)?;
                run.order
                    .try_reserve(1)
                    .map_err(|_| StorageError::AllocationFailed)?;
                let slot = partition.resident_index();
                run.order.push(slot);
                run.relations.insert(partition, Relation::default());
            }
            let relation = run
                .relations
                .get_mut(&partition)
                .expect("the delta arrangement was admitted");
            relation.tail.reserve_row()?;
            relation.insert(subject, object, row);
        }
        // This private physical run uses original partition ordinals. It never
        // resolves a term; public store traversal and sorted commits retain
        // their lexical ordering.
        run.order.sort_unstable();
        for &slot in &run.order {
            let predicate = Arity::<N>::key_term(self.keys[slot].1, 0);
            run.by_predicate
                .try_reserve(1)
                .map_err(|_| StorageError::AllocationFailed)?;
            let slots = run.by_predicate.entry(predicate).or_default();
            slots
                .try_reserve(1)
                .map_err(|_| StorageError::AllocationFailed)?;
            slots.push(slot);
        }
        self.interner
            .check_read()
            .map_err(DictionaryError::Source)?;
        Ok(run)
    }

    /// Original rows for one predicate, without scanning other predicates or
    /// rendering terms. Partitions are lexical; rows use their arrangement order.
    pub fn rows_for_predicate(
        &self,
        predicate: D::Id,
    ) -> impl Iterator<Item = (RowId, [D::Id; N])> + '_ {
        let mut positions = [None; N];
        positions[1] = Some(predicate);
        self.partitions_for(positions).flat_map(|partition| {
            let mut cursor = partition.select(Bound::Any);
            std::iter::from_fn(move || {
                let (subject, object, row) = cursor.next()?;
                Some((row, partition.row(subject, object)))
            })
        })
    }

    /// The partition at a dense slot.
    pub(crate) fn partition_at(&self, slot: usize) -> PartitionRef<'_, D::Id, N> {
        let (id, key) = self.keys[slot];
        PartitionRef {
            id,
            key,
            relation: &self.relations[slot],
        }
    }
}

impl RelationStore {
    /// Whether the quad `(subject, predicate, object, graph)` is present, as lexical
    /// surfaces.
    ///
    /// Membership for negation-as-failure and downstream dedup: every surface must resolve
    /// to an interned id ([`Self::term_id`]) or the quad cannot be present.
    pub fn contains(&self, subject: &str, predicate: &str, object: &str, graph: &str) -> bool {
        self.contains_row([subject, predicate, object, graph])
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
            .map(|&(_, (predicate, _))| self.interner.resolve(predicate))
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
            .map(|&(_, (_, graph))| self.interner.resolve(graph))
            .collect::<BTreeSet<_>>()
            .into_iter()
    }
}

impl<const N: usize> RelationStore<TermInterner, N>
where
    Arity<N>: StoreShape<N>,
{
    /// The interned id of a surface in any position. Never inserts.
    ///
    /// An unknown surface constrains a selection or membership probe to no rows.
    pub fn term_id(&self, surface: &str) -> Option<TermId> {
        self.interner.lookup(surface)
    }

    /// Whether this shape's complete row is present, without interning any surface.
    pub fn contains_row(&self, terms: [&str; N]) -> bool {
        let (Some(subject), Some(object)) = (self.term_id(terms[0]), self.term_id(terms[2])) else {
            return false;
        };
        let mut ids = [None; N];
        for &position in Arity::<N>::KEY_POSITIONS {
            let Some(id) = self.term_id(terms[position]) else {
                return false;
            };
            ids[position] = Some(id);
        }
        self.partition_for(&ids)
            .is_some_and(|partition| partition.relation.contains(subject, object))
    }

    /// Insert one native row in the store's declared shape.
    ///
    /// # Errors
    ///
    /// Returns the original typed dictionary or row-storage refusal before publication.
    pub fn insert_row(
        &mut self,
        terms: [&str; N],
    ) -> Result<Option<(TermId, TermId, RowId)>, StorageError> {
        self.insert_terms(terms.map(DictionaryTerm::Surface))
            .map_err(|error| match error {
                DictionaryError::Storage(error) => error,
                DictionaryError::Source(never) => match never {},
            })
    }

    /// Project native rows through the original lexical owner, pinning each key once.
    fn projected_rows(&self) -> impl Iterator<Item = (RowId, ShapeFact<N>)> + '_ {
        self.order.iter().flat_map(move |&slot| {
            let (_, key) = self.keys[slot];
            let key_terms = match resolve_key::<TermInterner, N>(&self.interner, key) {
                Ok(terms) => terms,
                Err(never) => match never {},
            };
            let mut cursor = self.relations[slot].select(Bound::Any);
            core::iter::from_fn(move || {
                let (subject, object, row) = LendingIterator::next(&mut cursor)?;
                let mut terms = core::array::from_fn(|_| String::new());
                self.interner.resolve(subject).clone_into(&mut terms[0]);
                self.interner.resolve(object).clone_into(&mut terms[2]);
                for (&position, surface) in Arity::<N>::KEY_POSITIONS
                    .iter()
                    .zip(key_terms.iter().flatten())
                {
                    (*surface).clone_into(&mut terms[position]);
                }
                Some((row, Arity::<N>::fact(terms)))
            })
        })
    }

    /// Every live row in insertion order, for deterministic rebuilds after retraction.
    pub(crate) fn facts_in_row_order(&self) -> Vec<(RowId, ShapeFact<N>)> {
        let mut rows = Vec::with_capacity(self.row_count);
        rows.extend(self.projected_rows());
        rows.sort_unstable_by_key(|(row, _)| row.index());
        rows
    }

    /// Project the model to lexical facts in subject/predicate/object[/graph] order.
    pub fn facts_sorted(&self) -> Vec<ShapeFact<N>> {
        let mut facts = Vec::with_capacity(self.row_count);
        facts.extend(self.projected_rows().map(|(_, fact)| fact));
        facts.sort_unstable();
        facts
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::{HostDictionary, SourceLost};

    #[test]
    fn cardinality_advice_never_builds_a_cold_historical_object_index() {
        let mut store = RelationStore::new();
        for index in 0..1000 {
            store
                .insert(&format!("s{index}"), "<p>", "marked", "")
                .unwrap();
        }
        let predicate = store.term_id("<p>").unwrap();
        let graph = store.term_id("").unwrap();
        let subject = store.term_id("s0").unwrap();
        let object = store.term_id("marked").unwrap();
        let partition = store.partition(predicate, graph).unwrap();
        assert!(!partition.relation.batches().is_empty());
        assert_eq!(partition.indexed_count(Bound::Object(object)), None);
        assert_eq!(partition.indexed_count(Bound::Subject(subject)), Some(1));
        assert_eq!(
            partition.indexed_count(Bound::Both(subject, object)),
            Some(1)
        );
        assert!(
            partition
                .relation
                .batches()
                .iter()
                .all(|batch| batch.object_index.get().is_none())
        );
        assert_eq!(
            partition.select(Bound::Object(object)).remaining_count(),
            1000
        );
        assert_eq!(partition.indexed_count(Bound::Object(object)), Some(1000));
    }

    #[test]
    fn predicate_directory_skips_other_predicates_and_preserves_graph_order() {
        for reverse in [false, true] {
            let mut store = RelationStore::default();
            let mut rows = (0..100)
                .flat_map(|predicate| {
                    ["<z>", "<a>", "<m>"].map(move |graph| {
                        [
                            "<s>".to_owned(),
                            format!("<p{predicate:03}>"),
                            "<o>".to_owned(),
                            graph.to_owned(),
                        ]
                    })
                })
                .collect::<Vec<_>>();
            if reverse {
                rows.reverse();
            }
            for row in rows {
                store.insert(&row[0], &row[1], &row[2], &row[3]).unwrap();
            }
            let predicate = store.term_id("<p050>").unwrap();
            let mut partitions = store.partitions(Some(predicate), None);
            let graphs = partitions
                .by_ref()
                .map(|partition| store.interner().resolve(partition.graph()).to_owned())
                .collect::<Vec<_>>();
            assert_eq!(graphs, ["<a>", "<m>", "<z>"]);
            assert_eq!(
                partitions.position.visited, 3,
                "unrelated predicates must never be examined"
            );
            let rows = store.rows_for_predicate(predicate).collect::<Vec<_>>();
            assert_eq!(rows.len(), 3);
            assert!(rows.iter().all(|(_, row)| row[1] == predicate));
            let unknown = store.term_id("<s>").unwrap();
            let mut absent = store.partitions(Some(unknown), None);
            assert!(absent.next().is_none());
            assert_eq!(absent.position.visited, 0);
            assert_eq!(
                store
                    .clone()
                    .rows_for_predicate(predicate)
                    .collect::<Vec<_>>(),
                rows
            );
        }
    }

    fn check_partition_position_cuts<const N: usize>(
        store: &RelationStore<TermInterner, N>,
        positions: [Option<TermId>; N],
        delta: Option<&DeltaRun<TermId>>,
    ) where
        Arity<N>: StoreShape<N>,
    {
        let mut reference = store.select_partitions(positions, delta);
        let expected: Vec<_> = reference.by_ref().map(PartitionRef::id).collect();
        let total_visits = reference.position.visited;
        for cut in 0..=expected.len() {
            let mut cursor = store.select_partitions(positions, delta);
            let mut actual = Vec::new();
            for _ in 0..cut {
                actual.push(cursor.next().unwrap().id());
            }
            let prior_visits = cursor.position.visited;
            let Partitions {
                store,
                delta,
                position,
            } = cursor;
            let mut resumed = Partitions {
                store,
                delta,
                position,
            };
            assert_eq!(resumed.position.visited, prior_visits);
            actual.extend(resumed.by_ref().map(PartitionRef::id));
            assert_eq!(
                actual, expected,
                "width {N}, bounds {positions:?}, cut {cut}"
            );
            assert_eq!(resumed.position.visited, total_visits);
        }
    }

    #[test]
    fn owned_partition_positions_resume_full_and_delta_quad_selections() {
        let mut store = RelationStore::new();
        for (predicate, graph) in [
            ("<z>", "<g2>"),
            ("<a>", "<g1>"),
            ("<z>", "<g1>"),
            ("<m>", "<g3>"),
        ] {
            store.insert("<s>", predicate, "<o>", graph).unwrap();
        }
        let delta = store.delta_run(1..store.row_count()).unwrap();
        let z = store.term_id("<z>").unwrap();
        let m = store.term_id("<m>").unwrap();
        let g1 = store.term_id("<g1>").unwrap();
        let g2 = store.term_id("<g2>").unwrap();
        let g3 = store.term_id("<g3>").unwrap();
        let absent = store.term_id("<s>").unwrap();
        for (predicate, graph) in [
            (None, None),
            (Some(z), None),
            (None, Some(g1)),
            (Some(z), Some(g1)),
            (Some(z), Some(g2)),
            (Some(m), Some(g3)),
            (Some(absent), None),
            (Some(absent), Some(g1)),
        ] {
            for run in [None, Some(&delta)] {
                check_partition_position_cuts(&store, [None, predicate, None, graph], run);
            }
        }
        let full_keys: Vec<_> = store
            .partitions(None, None)
            .map(|partition| {
                (
                    store.interner().resolve(partition.predicate()),
                    store.interner().resolve(partition.graph()),
                )
            })
            .collect();
        assert_eq!(
            full_keys,
            [
                ("<a>", "<g1>"),
                ("<m>", "<g3>"),
                ("<z>", "<g1>"),
                ("<z>", "<g2>"),
            ]
        );
        let delta_ids: Vec<_> = store
            .select_partitions([None; 4], Some(&delta))
            .map(PartitionRef::id)
            .collect();
        assert_eq!(
            delta_ids,
            [
                store
                    .partition(store.term_id("<a>").unwrap(), g1)
                    .unwrap()
                    .id(),
                store.partition(z, g1).unwrap().id(),
                store.partition(m, g3).unwrap().id(),
            ]
        );
    }

    #[test]
    fn owned_partition_positions_resume_full_and_delta_triple_selections() {
        let mut store = RelationStore::<TermInterner, 3>::with_dictionary(TermInterner::default());
        for predicate in ["<z>", "<a>", "<m>"] {
            store.insert_row(["<s>", predicate, "<o>"]).unwrap();
        }
        let delta = store.delta_run(1..store.row_count()).unwrap();
        for predicate in [None, Some("<z>"), Some("<a>"), Some("<s>")] {
            let predicate = predicate.map(|surface| store.term_id(surface).unwrap());
            for run in [None, Some(&delta)] {
                check_partition_position_cuts(&store, [None, predicate, None], run);
            }
        }
    }

    #[test]
    fn host_store_preserves_ids_and_original_source_refusal() {
        use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering as AtomicOrdering};
        const P: u64 = 0xf000_0000_0000_0007;
        const Q: u64 = 0x1000_0000_0000_0003;
        const GRAPH: u64 = 0x8000_0000_0000_0000;
        const TRIPLE: u64 = 0xa000_0000_1234_5678;
        let dictionary = HostDictionary {
            surfaces: [
                (P, "<https://example.org/a>".into()),
                (Q, "<https://example.org/z>".into()),
                (GRAPH, String::new()),
            ]
            .into_iter()
            .collect(),
            resolves: AtomicUsize::new(0),
            lookups: AtomicUsize::new(0),
            lost: AtomicBool::new(false),
            lose_after_lookup: AtomicBool::new(false),
            pins: AtomicUsize::new(0),
            extra_terms: 0,
        };
        let mut store = RelationStore::with_dictionary(dictionary);
        assert_eq!(store.interner().resolves.load(AtomicOrdering::Relaxed), 0);
        store.insert_ids([TRIPLE, Q, 0, GRAPH]).unwrap().unwrap();
        store.insert_ids([0, P, TRIPLE, GRAPH]).unwrap().unwrap();
        assert_eq!(
            store
                .partitions(None, None)
                .map(PartitionRef::predicate)
                .collect::<Vec<_>>(),
            vec![P, Q],
            "lexical sweep differs from raw-ID order"
        );
        let before = store.interner().resolves.load(AtomicOrdering::Relaxed);
        let left = store.partition(P, GRAPH).unwrap();
        let right = store.partition(Q, GRAPH).unwrap();
        let mut outer = left.select(Bound::Any);
        let mut joined = Vec::new();
        while let Some((s, middle, _)) = outer.next() {
            let mut inner = right.select(Bound::Subject(middle));
            while let Some((_, o, _)) = inner.next() {
                joined.push((s, middle, o));
            }
        }
        assert_eq!(joined, vec![(0, TRIPLE, 0)]);
        assert_eq!(
            store.interner().resolves.load(AtomicOrdering::Relaxed),
            before,
            "actual indexed cursor join performs no lexical access"
        );
        assert_eq!(store.rows_for_predicate(Q).count(), 1);
        assert_eq!(
            store.interner().resolves.load(AtomicOrdering::Relaxed),
            before
        );
        assert_eq!(store.interner().lookups.load(AtomicOrdering::Relaxed), 0);
        store.partition_limit = Some(2);
        assert_eq!(store.insert_ids([TRIPLE, Q, 0, GRAPH]).unwrap(), None);
        store
            .insert_terms([
                DictionaryTerm::Id(TRIPLE),
                DictionaryTerm::Surface("<https://example.org/z>"),
                DictionaryTerm::Id(1),
                DictionaryTerm::Id(GRAPH),
            ])
            .unwrap()
            .unwrap();
        let rows = store.rows_from(0).collect::<Vec<_>>();
        assert_eq!(
            rows.iter().map(|(_, ids)| *ids).collect::<Vec<_>>(),
            vec![
                [TRIPLE, Q, 0, GRAPH],
                [0, P, TRIPLE, GRAPH],
                [TRIPLE, Q, 1, GRAPH]
            ]
        );
        assert!(matches!(
            store.insert_ids([0, 17, 1, GRAPH]),
            Err(DictionaryError::Storage(StorageError::Capacity(_)))
        ));
        store.partition_limit = None;
        assert_eq!(
            store.insert_ids([0, 17, 1, GRAPH]),
            Err(DictionaryError::Source(SourceLost(17)))
        );
        assert_eq!(store.rows_from(0).collect::<Vec<_>>(), rows);
        assert_eq!(store.rows_for_predicate(Q).count(), 2);
        assert_eq!(store.partition_count(), 2);
        store.interner().lost.store(true, AtomicOrdering::Relaxed);
        assert_eq!(
            store.insert_ids([TRIPLE, Q, 0, GRAPH]),
            Err(DictionaryError::Source(SourceLost(99))),
            "even duplicate ingress checks original source lifetime"
        );
        assert_eq!(store.rows_from(0).collect::<Vec<_>>(), rows);
    }
    #[test]
    fn arrangements_preserve_tagged_host_ids_across_batches_and_tail() {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        struct HostId {
            tag: u8,
            payload: u64,
        }
        let subject = |i| HostId {
            tag: 7,
            payload: u64::MAX - i,
        };
        let object = |i| HostId {
            tag: 19,
            payload: (1_u64 << 48) + i,
        };
        let mut relation = Relation::<HostId>::default();
        let mut expected = Vec::new();
        for i in 0..145_u64 {
            let row = RowId::from_index(i).unwrap();
            relation.tail.reserve_row().unwrap();
            relation.insert(subject(i), object(i % 11), row);
            expected.push((subject(i), object(i % 11), row));
        }
        assert!(!relation.batches().is_empty());
        assert!(!relation.tail().is_empty());
        for &(s, o, _) in &expected {
            assert!(relation.contains(s, o));
            assert!(!relation.contains(HostId { tag: 8, ..s }, o));
            assert!(!relation.contains(s, HostId { tag: 20, ..o }));
            assert!(!relation.contains(
                HostId {
                    payload: s.payload ^ (1_u64 << 60),
                    ..s
                },
                o,
            ));
            assert!(!relation.contains(
                s,
                HostId {
                    payload: o.payload ^ (1_u64 << 60),
                    ..o
                },
            ));
        }
        for bound in [
            Bound::Any,
            Bound::Subject(subject(73)),
            Bound::Object(object(3)),
            Bound::Both(subject(135), object(3)),
        ] {
            let mut cursor = relation.select(bound);
            let mut actual = Vec::new();
            while let Some(row) = cursor.next() {
                actual.push(row);
            }
            let mut wanted: Vec<_> = expected
                .iter()
                .copied()
                .filter(|&(s, o, _)| match bound {
                    Bound::Any => true,
                    Bound::Subject(value) => s == value,
                    Bound::Object(value) => o == value,
                    Bound::Both(left, right) => s == left && o == right,
                })
                .collect();
            actual.sort_unstable();
            wanted.sort_unstable();
            assert_eq!(actual, wanted);
        }
        let mut wanted: Vec<_> = expected.iter().map(|&(s, _, row)| (s, row)).collect();
        wanted.sort_unstable();
        let mut subjects = ValueCursor::<VALUE_SUBJECT, HostId>::new(&relation, None);
        subjects.seek(subject(90));
        wanted.retain(|&(s, _)| s >= subject(90));
        assert_eq!(subjects.collect::<Vec<_>>(), wanted);
        let mut wanted: Vec<_> = expected.iter().map(|&(_, o, row)| (o, row)).collect();
        wanted.sort_unstable();
        let mut actual =
            ValueCursor::<VALUE_OBJECT, HostId>::new(&relation, None).collect::<Vec<_>>();
        assert!(actual.windows(2).all(|rows| rows[0].0 <= rows[1].0));
        // Equal values retain arrangement order, not row-id order.
        actual.sort_unstable();
        assert_eq!(actual, wanted);
        let rows = ValueCursor::<VALUE_OBJECT, HostId>::new(&relation, Some(subject(135)))
            .collect::<Vec<_>>();
        assert_eq!(rows, vec![(object(3), RowId::from_index(135).unwrap())]);
    }
    #[test]
    fn partition_capacity_keeps_existing_partition_growth_and_refuses_new_keys() {
        use purrdf_lex::allocation::{CapacityError, CapacitySpace};
        let mut store = RelationStore::new();
        store.insert("s", "p", "o", "").unwrap().unwrap();
        let partition = store.partition_id("p", "").unwrap();
        store.partition_limit = Some(1);
        store.insert("s2", "p", "o2", "").unwrap().unwrap();
        assert_eq!(store.partition_id("p", ""), Some(partition));
        let original_terms = store.interner.len();
        assert_eq!(
            store.insert("s", "new-p", "o", ""),
            Err(StorageError::Capacity(CapacityError::new(
                CapacitySpace::DatalogPartitions,
                2,
                1
            )))
        );
        assert_eq!(store.interner.len(), original_terms);
        assert_eq!(store.row_count(), 2);
        assert_eq!(store.partition_count(), 1);
        store.partition_limit = None;
        store.insert("s", "new-p", "o", "").unwrap().unwrap();
        assert_eq!(store.partition_id("new-p", "").unwrap().index(), 1);
    }

    #[test]
    fn dependent_term_capacity_rolls_back_lexical_identity_and_original_rows() {
        use purrdf_lex::allocation::{CapacityError, CapacitySpace};
        let mut store = RelationStore::new();
        store.insert("s", "p", "o", "").unwrap().unwrap();
        let original_rows = store.row_log.clone();
        let original_terms = store.interner.len();
        let original_bytes = store.interner.byte_len();
        let known = store.term_id("p").unwrap();
        store.interner.term_limit = Some(original_terms as u64);
        assert_eq!(store.interner.intern("p"), Ok(known));
        // Predicate and graph succeed; the subject refuses after both were minted.
        store.interner.term_limit = Some(original_terms as u64 + 2);
        assert_eq!(
            store.insert("new-s", "new-p", "new-o", "new-g"),
            Err(StorageError::Capacity(CapacityError::new(
                CapacitySpace::DatalogTerms,
                original_terms as u128 + 3,
                original_terms as u128 + 2
            )))
        );
        assert_eq!(store.row_log, original_rows);
        assert_eq!(store.interner.len(), original_terms);
        assert_eq!(store.interner.byte_len(), original_bytes);
        assert_eq!(store.partition_count(), 1);
        for term in ["new-s", "new-p", "new-o", "new-g"] {
            assert!(store.term_id(term).is_none());
        }
        store.interner.term_limit = None;
        assert_eq!(
            store
                .insert("new-s", "new-p", "new-o", "new-g")
                .unwrap()
                .unwrap()
                .2
                .index(),
            1
        );
        assert_eq!(
            store.term_id("new-p").unwrap().index(),
            original_terms as u64
        );
    }

    #[test]
    fn mixed_row_admission_preserves_ids_and_rolls_back_fresh_terms() {
        let mut store = RelationStore::new();
        store.insert("s", "p", "o", "").unwrap().unwrap();
        let predicate = store.term_id("p").unwrap();
        let graph = store.term_id("").unwrap();
        let before = (
            store.interner.len(),
            store.interner.byte_len(),
            store.row_log.clone(),
        );
        store.interner.term_limit = Some(before.0 as u64 + 1);
        let terms = [
            DictionaryTerm::Surface("new-s"),
            DictionaryTerm::Id(predicate),
            DictionaryTerm::Surface("new-o"),
            DictionaryTerm::Id(graph),
        ];
        for _ in 0..2 {
            assert!(matches!(
                store.insert_terms(terms),
                Err(DictionaryError::Storage(StorageError::Capacity(_)))
            ));
            assert_eq!(
                (
                    store.interner.len(),
                    store.interner.byte_len(),
                    store.row_log.clone()
                ),
                before
            );
            assert_eq!(store.term_id("new-s"), None);
            assert_eq!(store.term_id("new-o"), None);
        }
        store.interner.term_limit = None;
        store.insert_terms(terms).unwrap().unwrap();
        let (_, ids) = store.rows_from(1).next().unwrap();
        assert_eq!(ids[1], predicate);
        assert_eq!(ids[3], graph);
        assert_eq!(ids[0], store.term_id("new-s").unwrap());
        assert_eq!(ids[2], store.term_id("new-o").unwrap());
        let before = (store.interner.len(), store.interner.byte_len());
        let refusal: Result<(), DictionaryError<Infallible>> = store.interner.with_interned(
            [
                DictionaryTerm::Id(predicate),
                DictionaryTerm::Id(graph),
                DictionaryTerm::Surface("unpublished-s"),
                DictionaryTerm::Surface("unpublished-o"),
            ],
            |dictionary, ids| {
                assert_eq!(dictionary.resolve(ids[2]), "unpublished-s");
                Err(DictionaryError::Storage(StorageError::SizeOverflow))
            },
        );
        assert_eq!(
            refusal,
            Err(DictionaryError::Storage(StorageError::SizeOverflow))
        );
        assert_eq!((store.interner.len(), store.interner.byte_len()), before);
        assert_eq!(store.term_id("unpublished-s"), None);
        assert_eq!(store.term_id("unpublished-o"), None);
        store.interner.term_limit = Some(before.0 as u64);
        assert_eq!(store.insert_terms(terms), Ok(None));
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn native_row_capacity_refuses_before_terms_and_preserves_duplicate_hits() {
        use purrdf_lex::allocation::{CapacityError, CapacitySpace};
        let mut store = RelationStore::new();
        store.insert("s", "p", "o", "").unwrap().unwrap();
        let original_rows = store.row_log.clone();
        let original_terms = store.interner.len();
        let original_bytes = store.interner.byte_len();
        let original_partitions = store.partition_count();
        // Exercise the real insertion counter without allocating an impossible arena.
        store.row_count = usize::MAX;
        assert_eq!(store.insert("s", "p", "o", ""), Ok(None));
        assert_eq!(
            store.insert("fresh-s", "fresh-p", "fresh-o", "fresh-g"),
            Err(StorageError::Capacity(CapacityError::new(
                CapacitySpace::DatalogRows,
                u128::from(u64::MAX) + 1,
                u128::from(u64::MAX)
            )))
        );
        assert_eq!(store.row_count, usize::MAX);
        assert_eq!(store.row_log, original_rows);
        assert_eq!(store.interner.len(), original_terms);
        assert_eq!(store.interner.byte_len(), original_bytes);
        assert_eq!(store.partition_count(), original_partitions);
        assert!(store.term_id("fresh-s").is_none());
        assert!(store.term_id("fresh-p").is_none());
        assert!(store.term_id("fresh-o").is_none());
        assert!(store.term_id("fresh-g").is_none());
    }

    use super::*;
    use crate::cursor::LendingIterator;
    use crate::test_support::permute;

    #[test]
    fn consolidated_object_positions_preserve_wide_row_ids() {
        use crate::cursor::{RowCursor, VALUE_OBJECT, ValueCursor};
        let mut relation = Relation::default();
        let count = 517_u64;
        let object = TermId::from_index(0).unwrap();
        for index in 0..count {
            relation.tail.reserve_row().unwrap();
            relation.insert(
                TermId::from_index(index + 1).unwrap(),
                object,
                RowId::from_index(u64::from(u32::MAX) + index).unwrap(),
            );
        }
        assert!(
            relation
                .batches
                .iter()
                .any(|batch| batch.len() > usize::from(u8::MAX))
        );
        assert!(
            relation
                .batches
                .iter()
                .all(|batch| batch.object_index.get().is_none())
        );
        let mut cursor = RowCursor::new(&relation, Bound::Object(object));
        let mut rows = Vec::new();
        while let Some(row) = cursor.next() {
            rows.push(row.2);
        }
        rows.sort_unstable();
        let expected: Vec<_> = (0..count)
            .map(|index| RowId::from_index(u64::from(u32::MAX) + index).unwrap())
            .collect();
        assert_eq!(rows, expected);
        let ordered: Vec<_> = ValueCursor::<VALUE_OBJECT, _>::new(&relation, None)
            .map(|(value, row)| {
                assert_eq!(value, object);
                row
            })
            .collect();
        assert_eq!(ordered, expected);
        for batch in &relation.batches {
            let positions: &[usize] = batch.object_order();
            assert_eq!(positions.len(), batch.len());
            assert_eq!(size_of_val(positions), batch.len() * size_of::<usize>());
        }
    }

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
                    .unwrap()
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
                .unwrap()
                .is_some()
        );
        // Re-inserting the same quad is a no-op that reports None (no new row id).
        assert!(
            s.insert(&iri("a"), KNOWS, &iri("b"), RelationStore::DEFAULT_GRAPH)
                .unwrap()
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
        assert!(
            s.insert(&iri("a"), KNOWS, &iri("b"), &g1)
                .unwrap()
                .is_some()
        );
        assert!(
            s.insert(&iri("a"), KNOWS, &iri("b"), &g2)
                .unwrap()
                .is_some()
        );
        assert!(
            s.insert(&iri("a"), KNOWS, &iri("b"), &g1)
                .unwrap()
                .is_none(),
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
                .unwrap()
                .is_some()
        );
        assert!(
            s.insert(&iri("a"), KNOWS, &iri("b"), &named)
                .unwrap()
                .is_some()
        );
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
            .unwrap()
            .map(|(_, _, r)| r);
        let r1 = s
            .insert(&iri("a"), LIKES, &iri("c"), default)
            .unwrap()
            .map(|(_, _, r)| r);
        let r2 = s
            .insert(&iri("a"), KNOWS, &iri("c"), default)
            .unwrap()
            .map(|(_, _, r)| r);
        assert_eq!(r0, Some(RowId::from_index(0).unwrap()));
        assert_eq!(r1, Some(RowId::from_index(1).unwrap()));
        assert_eq!(
            r2,
            Some(RowId::from_index(2).unwrap()),
            "row ids span partitions in insertion order"
        );
        // A dedup consumes no row id — the space stays dense and `row_count` is exact.
        assert_eq!(
            s.insert(&iri("a"), KNOWS, &iri("b"), default).unwrap(),
            None
        );
        assert_eq!(s.row_count(), 3, "three distinct rows ⇒ row ids 0..3");
        // `select` hands each row its store-global id (never a per-partition index):
        // knows carries {0, 2}, likes carries {1} — the interleaved likes took id 1.
        let knows_ids: BTreeSet<RowId> = select_rows(&s, KNOWS, Bound::Any)
            .iter()
            .map(|&(_, _, r)| r)
            .collect();
        assert_eq!(
            knows_ids,
            [RowId::from_index(0).unwrap(), RowId::from_index(2).unwrap()].into(),
            "selected rows carry their store-global row id, not a per-partition index",
        );
        let likes_ids: BTreeSet<RowId> = select_rows(&s, LIKES, Bound::Any)
            .iter()
            .map(|&(_, _, r)| r)
            .collect();
        assert_eq!(likes_ids, [RowId::from_index(1).unwrap()].into());
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
                    .unwrap()
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
        let expect_p: BTreeSet<RowId> = (0..n)
            .step_by(2)
            .map(|index| RowId::from_address(index).unwrap())
            .collect();
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
            s.insert(&iri("s"), p, &iri("o0000"), default)
                .unwrap()
                .is_none(),
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
        assert!(
            s.insert(KNOWS, domain, &iri("Person"), default)
                .unwrap()
                .is_some()
        );
        // …and as the predicate of an ordinary quad here.
        assert!(
            s.insert(&iri("a"), KNOWS, &iri("b"), default)
                .unwrap()
                .is_some()
        );
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
            Some(PartitionId::from_index(0).unwrap())
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
                    .unwrap()
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
    fn rows_from_reads_the_rows_added_since_in_row_order() {
        let mut s = RelationStore::new();
        s.insert("<a>", "<p>", "<b>", RelationStore::DEFAULT_GRAPH)
            .unwrap();
        s.insert("<b>", "<q>", "<c>", "<g>").unwrap();
        assert!(
            s.insert("<a>", "<p>", "<b>", RelationStore::DEFAULT_GRAPH)
                .unwrap()
                .is_none(),
            "a duplicate adds no row"
        );
        s.insert("<c>", "<p>", "<d>", RelationStore::DEFAULT_GRAPH)
            .unwrap();
        let surfaces = |since: usize| -> Vec<(u64, [&str; 4])> {
            s.rows_from(since)
                .map(|(row, ids)| (row.index(), ids.map(|id| s.interner().resolve(id))))
                .collect()
        };
        assert_eq!(
            surfaces(1),
            vec![
                (1, ["<b>", "<q>", "<c>", "<g>"]),
                (2, ["<c>", "<p>", "<d>", RelationStore::DEFAULT_GRAPH]),
            ]
        );
        assert_eq!(surfaces(0).len(), s.row_count());
        assert_eq!(surfaces(3), Vec::new());
        assert_eq!(surfaces(7), Vec::new());
    }

    #[test]
    fn store_term_bytes_counts_each_distinct_surface_once() {
        let default = RelationStore::DEFAULT_GRAPH;
        let mut s = RelationStore::new();
        assert_eq!(s.term_bytes(), 0);
        assert!(s.interner().is_empty());

        s.insert(&iri("a"), KNOWS, &iri("b"), default).unwrap();
        let after_first = s.term_bytes();
        assert_eq!(
            after_first,
            iri("a").len() + iri("b").len() + KNOWS.len(),
            "the default graph's surface is empty, so it adds no bytes"
        );

        // A repeat of both terms under a second predicate interns only the predicate.
        s.insert(&iri("a"), LIKES, &iri("b"), default).unwrap();
        assert_eq!(s.term_bytes(), after_first + LIKES.len());
        let after_second = s.term_bytes();

        // A new term adds exactly its own bytes.
        s.insert(&iri("a"), KNOWS, &iri("c"), default).unwrap();
        assert_eq!(s.term_bytes(), after_second + iri("c").len());
        assert_eq!(s.term_bytes(), s.interner().byte_len());

        // The same quads in the opposite order reach the same total.
        let mut reversed = RelationStore::new();
        reversed
            .insert(&iri("a"), KNOWS, &iri("c"), default)
            .unwrap();
        reversed
            .insert(&iri("a"), LIKES, &iri("b"), default)
            .unwrap();
        reversed
            .insert(&iri("a"), KNOWS, &iri("b"), default)
            .unwrap();
        assert_eq!(reversed.term_bytes(), s.term_bytes());
    }

    /// Resolving an id no dictionary minted is a programming error, reported as a panic
    /// rather than a silent wrong surface.
    #[test]
    #[should_panic(expected = "must never cross store boundaries")]
    fn store_resolving_a_foreign_term_id_panics() {
        let s = sample_store();
        let _ = s.interner().resolve(TermId::from_index(999).unwrap());
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
                    .unwrap()
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
                s.insert(sub, p, obj, g).unwrap();
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
                s.insert(sub, pred, obj, RelationStore::DEFAULT_GRAPH)
                    .unwrap();
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
        let s = TermId::from_index(0).unwrap();
        let o = TermId::from_index(1).unwrap();

        // (+1) + (-1) = 0 ⇒ the row annihilates and is dropped (retraction).
        let plus = weighted_row(s, o, RowId::from_index(5).unwrap(), 1);
        let minus = weighted_row(s, o, RowId::from_index(2).unwrap(), -1);
        let retracted = merge_batches(&plus, &minus).expect("signed retraction combines");
        assert_eq!(retracted.len(), 0, "(+1)+(-1)=0 annihilates the shared key");

        // (+1) + (+2) = 3 ⇒ one surviving row, weights summed, lower row id kept.
        let two = weighted_row(s, o, RowId::from_index(2).unwrap(), 2);
        let summed = merge_batches(&plus, &two).expect("signed addition combines");
        assert_eq!(summed.len(), 1, "a non-annihilating combine keeps one row");
        assert_eq!(summed.weight[0], 3, "weights sum: 1 + 2 = 3");
        assert_eq!(
            summed.row_id[0],
            RowId::from_index(2).unwrap(),
            "the lower row id deterministically survives a key collision"
        );

        // Disjoint keys interleave with NO combine — the set-semantics `W = ()` shape.
        let o2 = TermId::from_index(2).unwrap();
        let a = weighted_row(s, o, RowId::from_index(0).unwrap(), 1);
        let b = weighted_row(s, o2, RowId::from_index(1).unwrap(), 1);
        let disjoint = merge_batches(&a, &b).expect("disjoint signed batches interleave");
        assert_eq!(disjoint.len(), 2, "disjoint keys interleave, no combine");
    }

    /// The abelian law the [`Weight`] contract demands, exercised on the signed monoid:
    /// merging in either run order yields the same weights, and the same surviving row
    /// id, so consolidation cannot depend on batch geometry.
    #[test]
    fn signed_weight_combine_is_commutative_across_merge_order() {
        let s = TermId::from_index(0).unwrap();
        let o = TermId::from_index(1).unwrap();
        let left = weighted_row(s, o, RowId::from_index(7).unwrap(), 4);
        let right = weighted_row(s, o, RowId::from_index(3).unwrap(), -1);
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
        let run: Vec<TermId> = (0..64)
            .map(|i| TermId::from_index(i * 3).unwrap())
            .collect();
        for key_slot in 0..200usize {
            let key = TermId::from_address(key_slot).unwrap();
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
        assert_eq!(
            gallop_lower_bound(&[], 0, TermId::from_index(0).unwrap()),
            0
        );
        assert_eq!(
            gallop_lower_bound(&run, 999, TermId::from_index(0).unwrap()),
            64
        );
    }

    /// The tail's duplicate probe as it was written over `(subject, object, row)`
    /// tuples: a short-circuiting scan that stops at the first matching key.
    /// Kept as the oracle of the column fold.
    fn reference_tail_contains(
        rows: &[(TermId, TermId, RowId)],
        subject: TermId,
        object: TermId,
    ) -> bool {
        rows.iter().any(|&(s, o, _)| s == subject && o == object)
    }

    /// A tail of `len` rows over a small id space (so keys collide across rows
    /// in either column), with the tuples it was built from.
    fn random_tail(
        len: usize,
        rng: &mut purrdf_testkit::rng::SplitMix64,
    ) -> (Tail, Vec<(TermId, TermId, RowId)>) {
        let mut tail = Tail::default();
        let mut rows = Vec::with_capacity(len);
        for i in 0..len {
            let s = TermId::from_address(rng.below_usize(24)).unwrap();
            let o = TermId::from_address(rng.below_usize(24)).unwrap();
            let r = RowId::from_address(i).unwrap();
            tail.reserve_row().unwrap();
            tail.push(s, o, r);
            rows.push((s, o, r));
        }
        (tail, rows)
    }

    /// The column fold answers every probe as the tuple scan does, at every tail
    /// length up to and including the seal threshold: each row's own key (a hit
    /// at every position), each row's key with one column changed (a miss unless
    /// another row happens to hold it), and keys from outside the id space (a
    /// miss everywhere).
    #[test]
    fn tail_contains_matches_the_tuple_scan() {
        let mut rng = purrdf_testkit::rng::SplitMix64::new(0x5EED_0F7A_11C0);
        let outside = TermId::from_index(1_000).unwrap();
        for len in 0..=TAIL_SEAL_THRESHOLD {
            for _ in 0..4 {
                let (tail, rows) = random_tail(len, &mut rng);
                assert_eq!(tail.len(), len);
                assert_eq!(tail.is_empty(), len == 0);
                let mut probes = vec![(outside, outside)];
                for &(s, o, _) in &rows {
                    probes.push((s, o));
                    probes.push((s, outside));
                    probes.push((outside, o));
                    probes.push((o, s));
                }
                for (s, o) in probes {
                    assert_eq!(
                        tail.contains(s, o),
                        reference_tail_contains(&rows, s, o),
                        "len {len}, probe ({s:?}, {o:?})"
                    );
                }
                for (position, &(s, o, _)) in rows.iter().enumerate() {
                    assert!(tail.contains(s, o), "len {len}: row {position} is found");
                }
                assert!(!tail.contains(outside, outside), "len {len}: absent key");
            }
        }
    }

    /// A hit in exactly one position, with every other row a near miss that
    /// shares one column with the probe: the fold must see the one row, wherever
    /// it sits, and must not take a one-column match for a hit.
    #[test]
    fn tail_contains_finds_a_lone_hit_at_every_position() {
        let subject = TermId::from_index((1_u64 << 48) + 7).unwrap();
        let object = TermId::from_index((1_u64 << 40) + 9).unwrap();
        let subject_low = TermId::from_index(7).unwrap();
        let object_low = TermId::from_index(9).unwrap();
        for len in 1..=TAIL_SEAL_THRESHOLD {
            for hit in 0..len {
                let mut tail = Tail::default();
                let mut rows = Vec::with_capacity(len);
                for i in 0..len {
                    let row = if i == hit {
                        (subject, object)
                    } else if i % 2 == 0 {
                        (subject, object_low)
                    } else {
                        (subject_low, object)
                    };
                    tail.reserve_row().unwrap();
                    tail.push(row.0, row.1, RowId::from_address(i).unwrap());
                    rows.push((row.0, row.1, RowId::from_address(i).unwrap()));
                }
                assert!(tail.contains(subject, object), "len {len}, hit {hit}");
                assert!(reference_tail_contains(&rows, subject, object));
                assert!(
                    !tail.contains(object, subject),
                    "len {len}, hit {hit}: the swapped key is absent"
                );
                assert!(!reference_tail_contains(&rows, object, subject));
                assert!(!tail.contains(subject_low, object_low));
                assert!(!reference_tail_contains(&rows, subject_low, object_low));
            }
            // The same near misses with no hit at all.
            let mut tail = Tail::default();
            for i in 0..len {
                let (s, o) = if i % 2 == 0 {
                    (subject, object_low)
                } else {
                    (subject_low, object)
                };
                tail.reserve_row().unwrap();
                tail.push(s, o, RowId::from_address(i).unwrap());
            }
            assert!(!tail.contains(subject, object), "len {len}: no hit");
        }
    }

    /// The columns read back as the tuples they were pushed as, in insertion
    /// order, by position and by iteration, and draining the tail returns them
    /// in that order and leaves it empty.
    #[test]
    fn tail_columns_round_trip_rows_in_insertion_order() {
        let mut rng = purrdf_testkit::rng::SplitMix64::new(0xC011);
        for len in [0, 1, 7, 8, 9, TAIL_SEAL_THRESHOLD - 1, TAIL_SEAL_THRESHOLD] {
            let (mut tail, rows) = random_tail(len, &mut rng);
            let by_position: Vec<_> = (0..tail.len()).map(|i| tail.row(i)).collect();
            assert_eq!(by_position, rows);
            assert_eq!(tail.iter().collect::<Vec<_>>(), rows);
            assert_eq!(tail.take_rows(), rows);
            assert!(tail.is_empty());
            assert_eq!(tail.len(), 0);
            assert_eq!(tail.iter().count(), 0);
        }
    }
}
