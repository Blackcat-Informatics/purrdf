// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Explicit carrier retention admission and non-identity work counters.
//!
//! Two accounting scopes live here and they answer different questions.
//!
//! * **Per view.** [`ViewStats`] is one view's own snapshot: what *this* view
//!   would retain and construct if it were the only view in the process, checked
//!   against *this* view's [`ViewLimits`]. It is deliberately unconditional — a
//!   base counted twice by one composite is charged twice — because admission is
//!   a statement about a single view's ceilings, not about process residency.
//! * **Across carriers.** [`RetentionLedger`] is the shared scope: several
//!   carriers that hold the SAME `Arc` base retain ONE copy of it between them,
//!   so the ledger reports that base's bytes ONCE and releases them when the last
//!   reader drops. Nothing here changes admission; the two scopes compose through
//!   [`ViewAccountingReport`], which keeps deduplicated RETAINED bytes and
//!   per-view INCREMENTAL bytes in separate fields so a consumer never adds one
//!   category into the other.

use core::any::Any;
use core::convert::Infallible;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::content_store::ContentDigest;
use crate::{RdfDataset, RdfDiagnostic};

/// Finite admission ceilings for one immutable view. Owners may select larger
/// ceilings explicitly; exceeding any ceiling fails before publishing a view.
#[derive(Debug, Clone, Copy)]
pub struct ViewLimits {
    /// Maximum number of retained source handles.
    pub max_sources: usize,
    /// Maximum aggregate source term count, including aliases.
    pub max_terms: usize,
    /// Maximum aggregate RDF record count across all three tables.
    pub max_rows: usize,
    /// Maximum retained native RDF payload bytes (not allocator/index/sidecar bytes).
    pub max_payload_bytes: usize,
    /// Maximum conservative charge for view construction and retained bookkeeping,
    /// including transformed literal text and caller-owned graph placement.
    pub max_auxiliary_bytes: usize,
}

impl Default for ViewLimits {
    fn default() -> Self {
        Self {
            max_sources: 64,
            max_terms: 16_777_216,
            max_rows: 67_108_864,
            max_payload_bytes: 1_073_741_824,
            max_auxiliary_bytes: 536_870_912,
        }
    }
}

/// Successful copy/publication work. Counters never participate in RDF identity.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ViewWork {
    /// Distinct terms in newly published dictionaries or rebound literals.
    pub copied_terms: usize,
    /// RDF records replayed at a dataset materialization boundary.
    pub copied_rows: usize,
    /// Published UTF-8 payload bytes in new dictionaries or rebound literals.
    /// Temporary importer buffers and allocator overhead are not counted.
    pub copied_text_bytes: usize,
    /// ID payload bytes written into view mappings and suppression sets.
    /// This excludes hash-table capacity and source-owned indexes.
    pub copied_index_bytes: usize,
    /// Successful native dataset freezes.
    pub freezes: usize,
    /// Successful complete view materializations.
    pub materializations: usize,
}

/// Retained payload and view-owned bookkeeping, separate from operational work.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ViewStats {
    /// Retained source handles. The view owns each handle until its final owner drops it.
    pub retained_sources: usize,
    /// Aggregate source term count, before cross-source aliasing.
    pub retained_terms: usize,
    /// Aggregate source RDF record count, before suppression and deduplication.
    pub retained_rows: usize,
    /// Immutable RDF payload bytes; source indexes and sidecars remain source-owned.
    pub retained_payload_bytes: usize,
    /// Conservative admission charge for retained and construction bookkeeping.
    /// Hash entries use four times their payload size for table slack; transient
    /// lookup maps are included. This is not an allocator or RSS measurement.
    pub auxiliary_bytes: usize,
    /// Work accumulated through this view and its clones.
    pub work: ViewWork,
}

impl ViewStats {
    pub(crate) fn retain(&mut self, dataset: &RdfDataset) {
        self.retained_sources = self.retained_sources.saturating_add(1);
        self.retained_terms = self.retained_terms.saturating_add(dataset.term_count());
        self.retained_rows = self.retained_rows.saturating_add(dataset.rdf_row_count());
        self.retained_payload_bytes = self
            .retained_payload_bytes
            .saturating_add(dataset.rdf_payload_bytes());
    }
}

impl ViewLimits {
    pub(crate) fn check(self, stats: &ViewStats) -> Result<(), RdfDiagnostic> {
        for (name, actual, limit) in [
            ("sources", stats.retained_sources, self.max_sources),
            ("terms", stats.retained_terms, self.max_terms),
            ("rows", stats.retained_rows, self.max_rows),
            (
                "payload bytes",
                stats.retained_payload_bytes,
                self.max_payload_bytes,
            ),
            (
                "auxiliary bytes",
                stats.auxiliary_bytes,
                self.max_auxiliary_bytes,
            ),
        ] {
            if actual > limit {
                return Err(RdfDiagnostic::error(
                    "view-retention-limit",
                    format!("view retains {actual} {name}, limit is {limit}"),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
pub(crate) struct WorkCounter {
    terms: AtomicUsize,
    rows: AtomicUsize,
    text: AtomicUsize,
    indexes: AtomicUsize,
    freezes: AtomicUsize,
    materializations: AtomicUsize,
}

impl WorkCounter {
    pub(crate) fn add(&self, work: ViewWork) {
        for (counter, value) in [
            (&self.terms, work.copied_terms),
            (&self.rows, work.copied_rows),
            (&self.text, work.copied_text_bytes),
            (&self.indexes, work.copied_index_bytes),
            (&self.freezes, work.freezes),
            (&self.materializations, work.materializations),
        ] {
            let mut old = counter.load(Ordering::Relaxed);
            loop {
                match counter.compare_exchange_weak(
                    old,
                    old.saturating_add(value),
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(actual) => old = actual,
                }
            }
        }
    }

    pub(crate) fn get(&self) -> ViewWork {
        ViewWork {
            copied_terms: self.terms.load(Ordering::Relaxed),
            copied_rows: self.rows.load(Ordering::Relaxed),
            copied_text_bytes: self.text.load(Ordering::Relaxed),
            copied_index_bytes: self.indexes.load(Ordering::Relaxed),
            freezes: self.freezes.load(Ordering::Relaxed),
            materializations: self.materializations.load(Ordering::Relaxed),
        }
    }
}

// ---------------------------------------------------------------------------
// Shared retention: one owner, one charge, released by its last reader
// ---------------------------------------------------------------------------

/// The identity of a retained owner *within one ledger*: the address of the
/// `Arc` allocation the owner lives in.
///
/// # Why an address is sound here
///
/// Pointer identity NEVER touches RDF identity. Two structurally equal datasets
/// at two addresses are two owners to the ledger and it is right that they are:
/// the ledger answers "how many bytes are resident", and two allocations occupy
/// two allocations' worth of memory whatever their content says. Conversely the
/// ledger never concludes anything about RDF *equality* from a shared address —
/// canonical identity is [`crate::ir::canon`]'s job and stays there.
///
/// The address is also stable for exactly as long as it is used as a key. Every
/// [`RetentionGuard`] holds a strong, type-erased handle on the owner it keys,
/// so the allocation cannot be freed — and therefore cannot be recycled under a
/// second, unrelated owner — while any guard names it. When the last guard for a
/// key drops, the ledger erases that key (and every memo entry filed under it)
/// BEFORE releasing its strong handle, so a later allocation that happens to
/// land on the same address starts from an empty slate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OwnerKey(usize);

impl OwnerKey {
    /// The raw address, for diagnostics and stable test ordering only. It is not
    /// an identifier a caller may persist, compare across processes, or resolve
    /// back to an owner.
    #[must_use]
    pub const fn addr(self) -> usize {
        self.0
    }
}

/// Whether a registered owner's content is immutable for as long as its guards
/// live. Only a [`Frozen`](OwnerMutability::Frozen) owner may be memoized.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OwnerMutability {
    /// The owner's content cannot change while a guard names it, so an identity
    /// analysis computed once stays true for every later reader. An
    /// `Arc<RdfDataset>` is frozen by construction.
    Frozen,
    /// The owner may change behind the guard. Retention is still accounted; the
    /// digest memo refuses to STORE for such an owner and recomputes on every
    /// call, which shows up as a miss that never becomes a hit.
    Mutable,
}

/// What one distinct owner contributes to a ledger's retained totals.
///
/// Charged ONCE per distinct owner per ledger, no matter how many guards name
/// it. The figures are native RDF payload, exactly as in [`ViewStats`]: source
/// indexes, sidecars and allocator overhead are not counted.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RetainedCharge {
    /// Retained source handles this owner accounts for (one, for a dataset base).
    pub sources: usize,
    /// Interned term count.
    pub terms: usize,
    /// RDF record count across all three tables.
    pub rows: usize,
    /// Immutable RDF payload bytes.
    pub payload_bytes: usize,
}

impl RetainedCharge {
    /// The charge of one frozen dataset base: one source handle plus its terms,
    /// rows and payload bytes — the same three figures a view charges into its
    /// own [`ViewStats`] per source.
    #[must_use]
    pub fn of_dataset(dataset: &RdfDataset) -> Self {
        Self {
            sources: 1,
            terms: dataset.term_count(),
            rows: dataset.rdf_row_count(),
            payload_bytes: dataset.rdf_payload_bytes(),
        }
    }
}

/// A ledger-wide reading of deduplicated retention plus digest-memo occupancy.
///
/// Every figure is recomputed from the live owner and memo tables at
/// [`RetentionLedger::snapshot`] time rather than carried as a running total, so
/// no sequence of registrations and drops can leave a residue behind: when the
/// last guard drops, the tables are empty and every figure except the monotonic
/// hit/miss counters is zero by construction.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RetentionSnapshot {
    /// Distinct owners currently registered (deduplicated by allocation).
    pub distinct_owners: usize,
    /// Live guards across all owners; at least `distinct_owners` while any exist.
    pub live_guards: usize,
    /// Deduplicated retained source handles.
    pub retained_sources: usize,
    /// Deduplicated retained term count.
    pub retained_terms: usize,
    /// Deduplicated retained RDF record count.
    pub retained_rows: usize,
    /// Deduplicated retained RDF payload bytes.
    pub retained_payload_bytes: usize,
    /// Stored `(owner, graph) -> digest` memo entries.
    pub memo_entries: usize,
    /// Payload bytes the memo itself occupies: per entry, the graph IRI's UTF-8
    /// bytes plus the fixed-size owner key and 32-byte digest. B-tree node and
    /// allocator overhead are not counted, exactly as elsewhere in this module.
    pub memo_bytes: usize,
    /// Memo lookups served from a stored entry, since the ledger was created.
    pub memo_hits: usize,
    /// Memo lookups that had to compute, since the ledger was created. Monotonic:
    /// unlike the retention figures, hits and misses are a history, not an
    /// occupancy, so dropping every guard does not reset them.
    pub memo_misses: usize,
}

/// The two accounting categories a consumer reports side by side, with no figure
/// counted in both.
///
/// * `retained` is ledger-wide and deduplicated: the bases themselves, charged
///   once however many carriers share them.
/// * `incremental_*` is one view's own construction charge — overlays, alias
///   maps, suppression sets, rebound literal text and caller-owned graph
///   placement — which is genuinely private to that view and is therefore
///   additive across views.
///
/// [`ViewStats::retained_payload_bytes`] and its sibling retained figures are
/// deliberately NOT carried over: they are that view's private restatement of
/// base bytes the ledger already reports once, and adding them is precisely the
/// double count this type exists to prevent.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ViewAccountingReport {
    /// Deduplicated, ledger-wide retention.
    pub retained: RetentionSnapshot,
    /// This view's own bookkeeping charge ([`ViewStats::auxiliary_bytes`]).
    pub incremental_auxiliary_bytes: usize,
    /// Work accumulated through this view, carried verbatim.
    pub incremental_work: ViewWork,
}

impl ViewAccountingReport {
    /// Pair a ledger snapshot with one view's stats.
    #[must_use]
    pub fn new(retained: RetentionSnapshot, stats: &ViewStats) -> Self {
        Self {
            retained,
            incremental_auxiliary_bytes: stats.auxiliary_bytes,
            incremental_work: stats.work,
        }
    }

    /// Retained payload bytes + memo bytes + this view's incremental bytes, with
    /// each byte counted exactly once.
    #[must_use]
    pub fn total_accounted_bytes(&self) -> usize {
        self.retained
            .retained_payload_bytes
            .saturating_add(self.retained.memo_bytes)
            .saturating_add(self.incremental_auxiliary_bytes)
    }
}

/// One registered owner: its charge, and how many guards currently name it.
#[derive(Debug)]
struct OwnerEntry {
    charge: RetainedCharge,
    mutability: OwnerMutability,
    /// Live guard count. Never zero while the entry exists: the entry is removed
    /// in the same locked section that takes the count to zero.
    guards: usize,
}

/// The ledger's whole mutable state, behind one lock.
#[derive(Debug, Default)]
struct LedgerState {
    owners: BTreeMap<OwnerKey, OwnerEntry>,
    /// `owner -> graph IRI -> canonical digest`. Nested rather than tuple-keyed
    /// so an owner's entire memo is erased with one `remove` when its last guard
    /// drops, and so lookups borrow `&str` without allocating.
    memo: BTreeMap<OwnerKey, BTreeMap<String, ContentDigest>>,
    memo_hits: usize,
    memo_misses: usize,
}

impl LedgerState {
    fn snapshot(&self) -> RetentionSnapshot {
        let mut out = RetentionSnapshot {
            memo_hits: self.memo_hits,
            memo_misses: self.memo_misses,
            ..RetentionSnapshot::default()
        };
        for entry in self.owners.values() {
            out.distinct_owners = out.distinct_owners.saturating_add(1);
            out.live_guards = out.live_guards.saturating_add(entry.guards);
            out.retained_sources = out.retained_sources.saturating_add(entry.charge.sources);
            out.retained_terms = out.retained_terms.saturating_add(entry.charge.terms);
            out.retained_rows = out.retained_rows.saturating_add(entry.charge.rows);
            out.retained_payload_bytes = out
                .retained_payload_bytes
                .saturating_add(entry.charge.payload_bytes);
        }
        for graphs in self.memo.values() {
            for graph in graphs.keys() {
                out.memo_entries = out.memo_entries.saturating_add(1);
                out.memo_bytes = out.memo_bytes.saturating_add(memo_entry_bytes(graph));
            }
        }
        out
    }
}

/// The payload charge of one memo entry: the graph IRI's own bytes plus the
/// fixed-size key and digest it is filed against.
fn memo_entry_bytes(graph: &str) -> usize {
    graph
        .len()
        .saturating_add(size_of::<OwnerKey>())
        .saturating_add(size_of::<ContentDigest>())
}

/// The shared scope: retention that is reported once per distinct owner and
/// released when that owner's last reader drops, plus a digest memo the owner's
/// readers share.
///
/// Construct with [`new`](Self::new) and share the resulting `Arc` — a ledger is
/// only meaningful when more than one carrier reports into it. Register an owner
/// with [`retain_dataset`](Self::retain_dataset) or [`retain`](Self::retain) and
/// keep the returned [`RetentionGuard`] for as long as the owner is held.
///
/// # What it does not do
///
/// It does not admit, refuse, or resize anything. [`ViewLimits`] remains the sole
/// admission gate and remains per view: a view that would exceed its own
/// ceilings is refused whether or not its bases are already resident on a ledger.
/// The ledger only reports, and it reports the residency question admission
/// cannot see.
///
/// # Locking
///
/// State lives behind a `std::sync::Mutex` — deterministic, dependency-free and
/// available on `wasm32-unknown-unknown`. No caller code ever runs while the lock
/// is held: [`RetentionGuard::memoized_graph_digest`] releases the lock before
/// calling its closure and re-takes it to store, so a closure may freely touch
/// the same ledger without deadlocking, and a panic inside one cannot leave the
/// state half-updated. Because of that, a poisoned lock is recovered rather than
/// propagated: the state is plain counters, and releasing a dropped owner's
/// accounting must not be blocked by an unrelated panic elsewhere.
#[derive(Debug, Default)]
pub struct RetentionLedger {
    state: Mutex<LedgerState>,
}

impl RetentionLedger {
    /// A new, empty ledger, ready to be shared between carriers.
    #[must_use]
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Register a frozen dataset base, charging [`RetainedCharge::of_dataset`].
    ///
    /// Registering the same `Arc` again — from another carrier, another clone, or
    /// the same carrier twice — adds a guard but NOT a second charge.
    #[must_use = "retention is released as soon as the guard drops"]
    pub fn retain_dataset(self: &Arc<Self>, base: &Arc<RdfDataset>) -> RetentionGuard {
        let charge = RetainedCharge::of_dataset(base);
        self.retain(base, charge, OwnerMutability::Frozen)
    }

    /// Register any shared owner under an explicit charge.
    ///
    /// The owner is deduplicated by the address of `owner`'s allocation, and the
    /// guard holds a strong handle on it so that address stays this owner's for
    /// the guard's whole life. A second registration of an owner already present
    /// keeps the FIRST charge and mutability: re-registering the same allocation
    /// under a different charge is a caller bug and is caught by a debug
    /// assertion rather than silently changing the reported total.
    #[must_use = "retention is released as soon as the guard drops"]
    pub fn retain<T: Any + Send + Sync>(
        self: &Arc<Self>,
        owner: &Arc<T>,
        charge: RetainedCharge,
        mutability: OwnerMutability,
    ) -> RetentionGuard {
        let key = OwnerKey(Arc::as_ptr(owner).cast::<()>().addr());
        let mut state = self.state();
        let entry = state.owners.entry(key).or_insert(OwnerEntry {
            charge,
            mutability,
            guards: 0,
        });
        debug_assert_eq!(
            entry.charge, charge,
            "one allocation was registered under two different charges"
        );
        debug_assert_eq!(
            entry.mutability, mutability,
            "one allocation was registered as both frozen and mutable"
        );
        entry.guards = entry.guards.saturating_add(1);
        let mutability = entry.mutability;
        let charge = entry.charge;
        drop(state);
        RetentionGuard {
            ledger: Arc::clone(self),
            key,
            charge,
            mutability,
            owner: Arc::clone(owner) as Arc<dyn Any + Send + Sync>,
        }
    }

    /// The current ledger-wide reading.
    #[must_use]
    pub fn snapshot(&self) -> RetentionSnapshot {
        self.state().snapshot()
    }

    /// [`snapshot`](Self::snapshot) paired with one view's per-view stats, as the
    /// two-category [`ViewAccountingReport`].
    #[must_use]
    pub fn report(&self, stats: &ViewStats) -> ViewAccountingReport {
        ViewAccountingReport::new(self.snapshot(), stats)
    }

    /// The lock, with poisoning recovered (see the type docs).
    fn state(&self) -> MutexGuard<'_, LedgerState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Take the guard count for `key` down by one, erasing the owner and its
    /// whole memo when it reaches zero. Called only from [`RetentionGuard::drop`].
    fn release(&self, key: OwnerKey) {
        let mut state = self.state();
        let Some(entry) = state.owners.get_mut(&key) else {
            debug_assert!(false, "released an owner the ledger never registered");
            return;
        };
        let Some(remaining) = entry.guards.checked_sub(1) else {
            debug_assert!(false, "released more guards than were registered");
            state.owners.remove(&key);
            state.memo.remove(&key);
            return;
        };
        entry.guards = remaining;
        if remaining == 0 {
            state.owners.remove(&key);
            // Erase the memo in the SAME locked section, and therefore before the
            // guard releases its strong handle on the allocation. No later owner
            // can inherit this one's memoized digests by landing on its address.
            state.memo.remove(&key);
        }
    }
}

/// A registered owner's RAII receipt: its charge is on the ledger for exactly as
/// long as at least one guard for that owner lives.
///
/// Cloning a guard adds a reader of the SAME owner — the charge does not grow.
/// Drop order is irrelevant: whichever guard happens to be last releases the
/// charge, and the release is exact (never partial, never negative, never left
/// behind).
pub struct RetentionGuard {
    ledger: Arc<RetentionLedger>,
    key: OwnerKey,
    charge: RetainedCharge,
    mutability: OwnerMutability,
    /// The strong, type-erased handle that keeps [`Self::key`] meaningful. Held
    /// for its side effect only; it is dropped AFTER [`Drop::drop`] has erased
    /// the ledger entry, which is what makes address reuse harmless.
    owner: Arc<dyn Any + Send + Sync>,
}

impl RetentionGuard {
    /// The ledger this guard reports into.
    #[must_use]
    pub fn ledger(&self) -> &Arc<RetentionLedger> {
        &self.ledger
    }

    /// This owner's ledger-local key.
    #[must_use]
    pub const fn owner_key(&self) -> OwnerKey {
        self.key
    }

    /// What this owner contributes to the ledger — once, however many guards
    /// name it.
    #[must_use]
    pub const fn charge(&self) -> RetainedCharge {
        self.charge
    }

    /// Whether this owner was registered as frozen.
    #[must_use]
    pub const fn mutability(&self) -> OwnerMutability {
        self.mutability
    }

    /// Get-or-compute this owner's digest for one named graph, shared with every
    /// other guard on the same owner and ledger.
    ///
    /// `compute` is the CALLER's canonicalization — typically
    /// [`graph_digest_view`](crate::ir::canon::graph_digest_view) over whatever
    /// view the caller holds. The ledger never canonicalizes anything itself and
    /// has no dependency on the canonicalizer; it stores 32 bytes under a name.
    ///
    /// A stored entry is returned without calling `compute` and counts a hit; a
    /// computed one counts a miss and is stored, so a second carrier over the
    /// same frozen base pays for the analysis once between them. For an owner
    /// registered [`Mutable`](OwnerMutability::Mutable) nothing is stored and
    /// every call is a miss — a memo would be a claim about content the ledger
    /// has no right to make.
    ///
    /// The lock is released around `compute`, so `compute` may itself use this
    /// ledger. If another thread stores the same key first, ITS value is kept and
    /// returned; for a frozen owner the two agree by construction.
    pub fn memoized_graph_digest<F>(&self, graph: &str, compute: F) -> ContentDigest
    where
        F: FnOnce() -> ContentDigest,
    {
        self.try_memoized_graph_digest::<_, Infallible>(graph, || Ok(compute()))
            .unwrap_or_else(|never| match never {})
    }

    /// [`memoized_graph_digest`](Self::memoized_graph_digest) for a fallible
    /// canonicalization, e.g.
    /// [`try_graph_digest_view`](crate::ir::canon::try_graph_digest_view).
    ///
    /// A failed computation stores nothing and counts neither a hit nor a miss:
    /// no analysis was completed, so there is nothing to have shared.
    ///
    /// # Errors
    /// Exactly what `compute` returns, unchanged.
    pub fn try_memoized_graph_digest<F, E>(
        &self,
        graph: &str,
        compute: F,
    ) -> Result<ContentDigest, E>
    where
        F: FnOnce() -> Result<ContentDigest, E>,
    {
        if self.mutability == OwnerMutability::Mutable {
            let digest = compute()?;
            let mut state = self.ledger.state();
            state.memo_misses = state.memo_misses.saturating_add(1);
            drop(state);
            return Ok(digest);
        }
        let mut state = self.ledger.state();
        if let Some(hit) = state
            .memo
            .get(&self.key)
            .and_then(|graphs| graphs.get(graph))
            .copied()
        {
            state.memo_hits = state.memo_hits.saturating_add(1);
            drop(state);
            return Ok(hit);
        }
        // The lock is released for the whole of the caller's canonicalization.
        drop(state);
        let digest = compute()?;
        let mut state = self.ledger.state();
        state.memo_misses = state.memo_misses.saturating_add(1);
        // Only file the entry while the owner is still registered: a guard that is
        // mid-drop on another thread must not have its erased memo resurrected.
        let stored = if state.owners.contains_key(&self.key) {
            *state
                .memo
                .entry(self.key)
                .or_default()
                .entry(graph.to_owned())
                .or_insert(digest)
        } else {
            digest
        };
        drop(state);
        Ok(stored)
    }
}

impl Clone for RetentionGuard {
    fn clone(&self) -> Self {
        let mut state = self.ledger.state();
        if let Some(entry) = state.owners.get_mut(&self.key) {
            entry.guards = entry.guards.saturating_add(1);
        } else {
            debug_assert!(false, "cloned a guard whose owner is no longer registered");
        }
        drop(state);
        Self {
            ledger: Arc::clone(&self.ledger),
            key: self.key,
            charge: self.charge,
            mutability: self.mutability,
            owner: Arc::clone(&self.owner),
        }
    }
}

impl core::fmt::Debug for RetentionGuard {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // The owner is deliberately not formatted: it is held for its lifetime
        // effect, and rendering a whole dataset into a guard's Debug output is
        // never what a caller wanted.
        f.debug_struct("RetentionGuard")
            .field("key", &self.key)
            .field("charge", &self.charge)
            .field("mutability", &self.mutability)
            .finish_non_exhaustive()
    }
}

impl Drop for RetentionGuard {
    fn drop(&mut self) {
        self.ledger.release(self.key);
        // `self.owner` drops after this body returns, so the allocation this
        // guard keyed is still alive while the ledger erases the key.
    }
}
