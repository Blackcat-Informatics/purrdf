// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Plan lifetime accounting independent of cache retention and caller handle count.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Live admitted plan allocations associated with one preparation owner.
///
/// Bytes are each plan's conservative payload charge at admission, counted once regardless of
/// `Arc` clones. They exclude cache keys, allocator overhead and shared tracker
/// storage. Shared strings are charged per occurrence, as in algebra accounting.
/// Public caller mutation cannot be intercepted: use
/// [`crate::PreparedQuery::retained_size_bytes`] to inspect that plan's current
/// payload. These counters never claim to measure arbitrary caller mutations or RSS.
/// Public totals saturate at `usize::MAX`; accounting and subtraction remain exact.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlanMemoryStats {
    /// All still-live admitted plans, retained or detached.
    pub live_plans: usize,
    /// Admitted payload bytes of all live plans.
    pub live_bytes: usize,
    /// Live plans still owned by their preparing cache.
    pub retained_plans: usize,
    /// Admitted payload bytes still owned by that cache, excluding its keys.
    pub retained_bytes: usize,
    /// Live plans held exclusively outside their preparing cache.
    pub detached_plans: usize,
    /// Admitted payload bytes held exclusively outside the cache.
    pub detached_bytes: usize,
}

/// A lifetime-bounded accounting observer that retains no query plans.
///
/// Keep an observer across cache eviction, policy replacement or destruction to
/// observe the final caller-held plan being released. Its fixed-size counters use
/// no registry of plan pointers, weak handles or query keys.
#[derive(Clone, Debug, Default)]
pub struct PlanMemoryObserver(Arc<Mutex<AllocationTotals>>);

// Each live allocation can contribute at most usize::MAX bytes. Even on 64-bit
// hosts, the address space cannot hold enough PlanCharges to overflow u128.
#[derive(Debug, Default)]
struct AllocationTotals {
    live_plans: u128,
    live_bytes: u128,
    retained_plans: u128,
    retained_bytes: u128,
}

fn projected(total: u128) -> usize {
    usize::try_from(total).unwrap_or(usize::MAX)
}

impl PlanMemoryObserver {
    /// A coherent snapshot of this preparation owner's admitted allocations.
    #[must_use]
    pub fn stats(&self) -> PlanMemoryStats {
        let stats = self.0.lock().expect("plan accounting lock poisoned");
        PlanMemoryStats {
            live_plans: projected(stats.live_plans),
            live_bytes: projected(stats.live_bytes),
            retained_plans: projected(stats.retained_plans),
            retained_bytes: projected(stats.retained_bytes),
            detached_plans: projected(stats.live_plans - stats.retained_plans),
            detached_bytes: projected(stats.live_bytes - stats.retained_bytes),
        }
    }
}

#[derive(Debug)]
pub(crate) struct PlanCharge {
    observer: PlanMemoryObserver,
    bytes: usize,
    retained: AtomicBool,
}

impl PlanCharge {
    pub(crate) fn new(observer: &PlanMemoryObserver, bytes: usize) -> Self {
        {
            let mut stats = observer.0.lock().expect("plan accounting lock poisoned");
            stats.live_plans += 1;
            stats.live_bytes += bytes as u128;
        }
        Self {
            observer: observer.clone(),
            bytes,
            retained: AtomicBool::new(false),
        }
    }

    pub(crate) fn observer(&self) -> PlanMemoryObserver {
        self.observer.clone()
    }

    pub(crate) fn retain(&self) {
        let mut stats = self
            .observer
            .0
            .lock()
            .expect("plan accounting lock poisoned");
        if !self.retained.swap(true, Ordering::Relaxed) {
            stats.retained_plans += 1;
            stats.retained_bytes += self.bytes as u128;
        }
    }

    pub(crate) fn detach(&self) {
        let mut stats = self
            .observer
            .0
            .lock()
            .expect("plan accounting lock poisoned");
        if self.retained.swap(false, Ordering::Relaxed) {
            stats.retained_plans -= 1;
            stats.retained_bytes -= self.bytes as u128;
        }
    }
}

impl Drop for PlanCharge {
    fn drop(&mut self) {
        let mut stats = self
            .observer
            .0
            .lock()
            .expect("plan accounting lock poisoned");
        stats.live_plans -= 1;
        stats.live_bytes -= self.bytes as u128;
        if *self.retained.get_mut() {
            stats.retained_plans -= 1;
            stats.retained_bytes -= self.bytes as u128;
        }
    }
}

/// The [`PlanMemoryObserver`] one worker's per-thread template/layout interner
/// tables charge into — `crate::substitute::INTERNED_VARIABLES` and
/// `crate::solution::INTERNED_SCHEMAS`.
///
/// Thread-local, matching the tables it serves: both are per-worker state with
/// no staleness dimension (see their own doc comments — `NativeSparqlEngine`
/// itself is held in a thread-local by every parallel caller, so per-worker is
/// what an engine-owned observer would have given anyway). Each interner
/// table is bounded individually by `INTERNED_VARIABLE_CAP`/`INTERNED_SCHEMA_CAP`,
/// but that entry cap alone leaves its bytes invisible to the SAME
/// [`PlanMemoryStats`] a caller already reads to bound one worker's
/// [`crate::PlanCache`], so a deployment's `CacheLimits` could look satisfied
/// while this memory grew unbounded beside it — this observer is what closes
/// that gap. Returns a cheap `Arc` clone of the
/// calling thread's observer, not a fresh one — every caller on one thread reads
/// and charges the SAME totals.
pub(crate) fn interner_memory_observer() -> PlanMemoryObserver {
    thread_local! {
        static OBSERVER: PlanMemoryObserver = PlanMemoryObserver::default();
    }
    OBSERVER.with(Clone::clone)
}

/// Bulk-cleared thread-local interner charge tracking.
///
/// `crate::substitute::INTERNED_VARIABLES` and `crate::solution::INTERNED_SCHEMAS`
/// both clear their whole table in one shot on reaching their cap, rather than
/// evicting entry-by-entry (see their own doc comments: both memoize a pure
/// function, so losing the table costs one reconstruction, never a wrong
/// answer). This tracks that same granularity — one running byte total, charged
/// against [`interner_memory_observer`] and recharged on every insert — rather
/// than one [`PlanCharge`] (and its `Mutex` lock) per entry, which a table that
/// only ever grows-then-clears has no use for.
#[derive(Debug, Default)]
pub(crate) struct InternerCharge {
    bytes: usize,
    charge: Option<PlanCharge>,
}

impl InternerCharge {
    /// An unfunded charge: the const-constructible zero state, so a
    /// `thread_local!` table can still initialize without an allocation until
    /// its first insert — matching the tables' own "never allocates a table until
    /// used" invariant.
    pub(crate) const fn new() -> Self {
        Self {
            bytes: 0,
            charge: None,
        }
    }

    /// Charge `delta` additional bytes — one freshly-inserted entry's estimated
    /// size — recharging [`interner_memory_observer`] with the new running total.
    /// The old charge (if any) is dropped, crediting its bytes back, in the same
    /// statement the new one is retained, so the observer never double-counts nor
    /// under-counts between the two.
    pub(crate) fn add(&mut self, delta: usize) {
        self.bytes = self.bytes.saturating_add(delta);
        let observer = interner_memory_observer();
        let charge = PlanCharge::new(&observer, self.bytes);
        charge.retain();
        self.charge = Some(charge);
    }

    /// Credit every byte charged so far back to the observer — call exactly when
    /// the table itself is bulk-cleared.
    pub(crate) fn clear(&mut self) {
        self.bytes = 0;
        self.charge = None;
    }
}

#[cfg(test)]
mod tests {
    use super::{PlanCharge, PlanMemoryObserver};

    #[test]
    fn saturated_public_totals_keep_exact_detached_and_drop_accounting() {
        let observer = PlanMemoryObserver::default();
        let retained = PlanCharge::new(&observer, usize::MAX);
        retained.retain();
        let detached = PlanCharge::new(&observer, 31);
        assert_eq!(observer.stats().live_bytes, usize::MAX);
        assert_eq!(observer.stats().retained_bytes, usize::MAX);
        assert_eq!(observer.stats().detached_bytes, 31);
        drop(retained);
        assert_eq!(observer.stats().live_bytes, 31);
        assert_eq!(observer.stats().detached_bytes, 31);
        drop(detached);
        assert_eq!(observer.stats(), super::PlanMemoryStats::default());
    }
}
