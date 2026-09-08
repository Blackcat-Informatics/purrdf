// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

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
