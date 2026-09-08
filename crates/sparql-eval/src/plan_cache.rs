// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Bounded deterministic LRU storage for prepared plans and join orders.

use std::collections::BTreeMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex};

use crate::DetHashMap;

/// Retention ceilings for a query-plan or join-order cache.
///
/// Zero in either field disables retention, never execution. Oversize entries
/// execute normally without evicting useful smaller entries. These ceilings
/// bound cache-owned payload accounting and entry count, not allocator overhead,
/// process RSS, temporary compilation, or plans retained by a caller's `Arc`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CacheLimits {
    /// Maximum retained entries.
    pub entries: usize,
    /// Maximum retained payload bytes (keys and owned plan storage).
    pub bytes: usize,
}

impl Default for CacheLimits {
    fn default() -> Self {
        Self {
            entries: 4096,
            bytes: 64 * 1024 * 1024,
        }
    }
}

/// Observable cache retention and lifetime counters. Counters saturate.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CacheStats {
    /// Currently retained entries.
    pub entries: usize,
    /// Currently charged payload bytes; see [`CacheLimits`].
    pub bytes: usize,
    /// Successful lookups.
    pub hits: u64,
    /// Unsuccessful lookups, including requests whose later preparation fails.
    pub misses: u64,
    /// Entries removed to admit newer entries within the ceilings.
    pub evictions: u64,
    /// Successful preparations not retained because of the ceilings.
    pub unretained: u64,
}

#[derive(Debug)]
struct Entry<V> {
    value: V,
    bytes: usize,
    stamp: u64,
}

#[derive(Debug)]
pub(crate) struct BoundedCache<K, V> {
    entries: DetHashMap<K, Entry<V>>,
    recency: BTreeMap<u64, K>,
    limits: CacheLimits,
    stats: CacheStats,
    clock: u64,
}

pub(crate) type BoundedOrderCache = Mutex<BoundedCache<(u64, u64), Arc<[usize]>>>;

#[derive(Clone, Copy)]
pub(crate) enum OrderCacheRef<'a> {
    Legacy(&'a crate::eval::BgpOrderCache),
    Bounded(&'a BoundedOrderCache),
}

impl<K: Clone + Eq + Hash, V: Clone> Default for BoundedCache<K, V> {
    fn default() -> Self {
        Self::new(CacheLimits::default())
    }
}

impl<K: Clone + Eq + Hash, V: Clone> BoundedCache<K, V> {
    pub(crate) fn new(limits: CacheLimits) -> Self {
        Self {
            entries: DetHashMap::default(),
            recency: BTreeMap::new(),
            limits,
            stats: CacheStats::default(),
            clock: 0,
        }
    }

    pub(crate) fn stats(&self) -> CacheStats {
        self.stats
    }

    fn tick(&mut self) -> u64 {
        if self.clock == u64::MAX {
            // Preserve LRU order at counter rollover; never introduce a timestamp
            // collision or a hash-iteration tie break.
            let old = std::mem::take(&mut self.recency);
            self.clock = 0;
            for (_, key) in old {
                self.entries
                    .get_mut(&key)
                    .expect("recency entry exists")
                    .stamp = self.clock;
                self.recency.insert(self.clock, key);
                self.clock += 1;
            }
        }
        let stamp = self.clock;
        self.clock += 1;
        stamp
    }

    pub(crate) fn get(&mut self, key: &K) -> Option<V> {
        if !self.entries.contains_key(key) {
            self.stats.misses = self.stats.misses.saturating_add(1);
            return None;
        }
        let stamp = self.tick();
        let entry = self.entries.get_mut(key).expect("checked cache entry");
        self.recency.remove(&entry.stamp);
        entry.stamp = stamp;
        self.recency.insert(stamp, key.clone());
        self.stats.hits = self.stats.hits.saturating_add(1);
        Some(entry.value.clone())
    }

    pub(crate) fn insert(&mut self, key: K, value: V, bytes: usize) {
        if self.limits.entries == 0
            || self.limits.bytes == 0
            || bytes == usize::MAX
            || bytes > self.limits.bytes
        {
            self.stats.unretained = self.stats.unretained.saturating_add(1);
            return;
        }
        if let Some(previous) = self.entries.remove(&key) {
            self.recency.remove(&previous.stamp);
            self.stats.bytes -= previous.bytes;
        }
        while self.entries.len() >= self.limits.entries
            || self.stats.bytes > self.limits.bytes - bytes
        {
            let (_, oldest) = self.recency.pop_first().expect("nonempty cache to evict");
            let entry = self.entries.remove(&oldest).expect("recency entry exists");
            self.stats.bytes -= entry.bytes;
            self.stats.evictions = self.stats.evictions.saturating_add(1);
        }
        let stamp = self.tick();
        self.recency.insert(stamp, key.clone());
        self.entries.insert(
            key,
            Entry {
                value,
                bytes,
                stamp,
            },
        );
        self.stats.bytes += bytes;
        self.stats.entries = self.entries.len();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recency_rollover_preserves_lru_order_and_byte_accounting() {
        let mut cache = BoundedCache::new(CacheLimits {
            entries: 2,
            bytes: 20,
        });
        cache.insert(1, 10, 5);
        cache.insert(2, 20, 10);
        cache.clock = u64::MAX;
        assert_eq!(cache.get(&1), Some(10));
        cache.insert(3, 30, 8);
        assert_eq!(cache.get(&2), None);
        assert_eq!(cache.get(&1), Some(10));
        assert_eq!(cache.stats().bytes, 13);
        assert_eq!(cache.stats().evictions, 1);
    }
}
