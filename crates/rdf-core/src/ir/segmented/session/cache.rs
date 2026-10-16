// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Source-qualified block lookup and recency links in the admitted cache table.

use super::{
    Arc, CacheEntry, CacheKey, CachedBlock, DictionaryPosition, SegmentedError, State, format,
};
use crate::hash::hash_of;

impl State {
    /// A hit changes only recency links, never shifts or reallocates cache entries.
    pub(super) fn cache_hit(&mut self, key: CacheKey) -> Option<Arc<CachedBlock>> {
        let entry = self.cache.find(hash_of(&key), |entry| entry.key == key)?;
        let block = Arc::clone(&entry.block);
        let (older, newer) = (entry.older, entry.newer);
        if self.newest != Some(key) {
            self.detach_cache_links(older, newer);
            self.append_cache_links(key);
        }
        Some(block)
    }

    /// The caller has admitted table capacity before reading the new block.
    pub(super) fn cache_insert(
        &mut self,
        key: CacheKey,
        block: Arc<CachedBlock>,
        retain_route: bool,
    ) {
        debug_assert!(self.cache.len() < self.cache.capacity());
        if retain_route && let Some((first, end)) = block.dictionary_range {
            let width = u64::from(format::MAX_RECORDS_PER_BLOCK);
            for bucket in first / width..=(end - 1) / width {
                if self
                    .dictionary_positions
                    .find(hash_of(&(key.snapshot, bucket)), |position| {
                        position.key == key && position.bucket == bucket
                    })
                    .is_some()
                {
                    continue;
                }
                debug_assert!(
                    self.dictionary_positions.len() < self.dictionary_positions.capacity()
                );
                self.dictionary_positions.insert_unique(
                    hash_of(&(key.snapshot, bucket)),
                    DictionaryPosition {
                        key,
                        bucket,
                        first,
                        end,
                    },
                    |entry| hash_of(&(entry.key.snapshot, entry.bucket)),
                );
            }
        }
        self.cache.insert_unique(
            hash_of(&key),
            CacheEntry {
                key,
                block,
                older: None,
                newer: None,
            },
            |entry| hash_of(&entry.key),
        );
        self.append_cache_links(key);
    }

    /// Skip lexical pins in recency order; lookup and unlink are indexed.
    /// Iterators can retain row bytes outside the cache: their original block
    /// reservation stays live until the last owner releases it. Freeing their
    /// cache slot lets nested scans proceed without reacquiring the outer block.
    pub(super) fn evict_one(&mut self) -> Result<bool, SegmentedError> {
        let mut next = self.oldest;
        while let Some(key) = next {
            let entry = self
                .cache
                .find(hash_of(&key), |entry| entry.key == key)
                .expect("recency link names a retained block");
            next = entry.newer;
            if Arc::strong_count(&entry.block) != 1 && entry.block.encoded[0] != format::QUADS {
                continue;
            }
            let evictions = self
                .evictions
                .checked_add(1)
                .ok_or(SegmentedError::AddressExhausted)?;
            let entry = self
                .cache
                .find_entry(hash_of(&key), |entry| entry.key == key)
                .expect("selected cache block exists")
                .remove()
                .0;
            self.detach_cache_links(entry.older, entry.newer);
            self.evictions = evictions;
            return Ok(true);
        }
        Ok(false)
    }

    fn detach_cache_links(&mut self, older: Option<CacheKey>, newer: Option<CacheKey>) {
        if let Some(key) = older {
            self.cache
                .find_mut(hash_of(&key), |entry| entry.key == key)
                .expect("older recency link exists")
                .newer = newer;
        } else {
            self.oldest = newer;
        }
        if let Some(key) = newer {
            self.cache
                .find_mut(hash_of(&key), |entry| entry.key == key)
                .expect("newer recency link exists")
                .older = older;
        } else {
            self.newest = older;
        }
    }

    fn append_cache_links(&mut self, key: CacheKey) {
        if let Some(previous) = self.newest {
            self.cache
                .find_mut(hash_of(&previous), |entry| entry.key == previous)
                .expect("newest recency link exists")
                .newer = Some(key);
        } else {
            self.oldest = Some(key);
        }
        let entry = self
            .cache
            .find_mut(hash_of(&key), |entry| entry.key == key)
            .expect("appended cache block exists");
        entry.older = self.newest;
        entry.newer = None;
        self.newest = Some(key);
    }
}
