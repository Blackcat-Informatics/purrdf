// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Fixed-work cache hit, eviction, pin and final-owner controls.

use super::{Arc, SegmentedReadLimits, SegmentedSession, TermValue, lock_read_state};
use crate::{DatasetView, SegmentedBuildLimits, SegmentedBuilder};

#[test]
fn cache_hits_preserve_full_recency_and_pinned_owners() {
    const HITS: u64 = 262_144;
    let mut builder =
        SegmentedBuilder::new(SegmentedBuildLimits::new(16_384, 2_000_000, 0, 512, 4).unwrap())
            .unwrap();
    let values: Vec<_> = (0..16_384)
        .map(|index| TermValue::iri(format!("urn:cache-control:{index:05}")))
        .collect();
    builder.intern_batch(&values, |_, _| {}).unwrap();
    let image = builder.seal().unwrap();
    for entries in [64, 4096] {
        let session = SegmentedSession::open(
            Arc::new(image.provider()),
            image.receipt(),
            SegmentedReadLimits::new(256 * 1024 * 1024, entries, 200_000, 256 * 1024 * 1024, 32),
        )
        .unwrap();
        let first = session.header.dictionary.first;
        for offset in 0..u64::from(entries) {
            drop(session.block(first + offset).unwrap());
        }
        let before = session.evidence();
        for index in 0..HITS {
            let slot = first + index * 8191 % u64::from(entries);
            let block = session.block(slot).unwrap();
            std::hint::black_box(&block.encoded);
        }
        let after = session.evidence();
        assert_eq!(after.request_count(), before.request_count());
        assert_eq!(after.io_bytes(), before.io_bytes());
        assert_eq!(after.live_bytes(), before.live_bytes());
        assert_eq!(after.bookkeeping_bytes(), before.bookkeeping_bytes());
        assert_eq!(after.evictions(), before.evictions());

        // The final full permutation leaves `first` oldest. Touching/pinning it
        // makes the next permutation member the oldest evictable owner.
        let pinned = session.block(first).unwrap();
        let bytes = pinned.encoded.clone();
        assert!(lock_read_state(&session.state).evict_one().unwrap());
        drop(session.block(first).unwrap());
        assert_eq!(session.evidence().request_count(), before.request_count());
        let evicted = first + 8191 % u64::from(entries);
        drop(session.block(evicted).unwrap());
        assert!(session.evidence().request_count() > before.request_count());
        assert_eq!(pinned.encoded, bytes);
        assert_eq!(session.read_error(), None);

        let budget = Arc::downgrade(&session.budget);
        drop(before);
        drop(after);
        drop(session);
        assert!(budget.upgrade().is_some());
        assert_eq!(pinned.encoded, bytes);
        drop(pinned);
        assert!(budget.upgrade().is_none());
    }
}
