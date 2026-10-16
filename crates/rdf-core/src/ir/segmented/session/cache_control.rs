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

#[derive(Debug)]
struct ScanProvider {
    bytes: crate::SegmentedBytes,
    snapshots: std::sync::atomic::AtomicUsize,
    stopped: std::sync::atomic::AtomicBool,
}
impl crate::SegmentedProvider for ScanProvider {
    fn snapshot(&self) -> crate::SegmentedSnapshot {
        self.snapshots
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.bytes.snapshot()
    }
    fn byte_len(&self) -> u64 {
        self.bytes.byte_len()
    }
    fn read_at(&self, position: u64, output: &mut [u8]) -> Result<(), super::SegmentedError> {
        if self.stopped.load(std::sync::atomic::Ordering::Relaxed) {
            Err(super::SegmentedError::Stopped(crate::StopCause::Cancelled))
        } else {
            self.bytes.read_at(position, output)
        }
    }
}

fn scan_image(rows: u32) -> crate::SegmentedImage {
    let mut builder = SegmentedBuilder::new(
        SegmentedBuildLimits::new(rows * 2 + 16, 2_000_000, rows * 3, 1024, 8).unwrap(),
    )
    .unwrap();
    let mut values: Vec<_> = (0..rows.max(3))
        .map(|index| TermValue::iri(format!("urn:scan:{index:05}")))
        .collect();
    values.push(TermValue::Triple {
        s: crate::TermBox::new(values[0].clone()),
        p: crate::TermBox::new(values[1].clone()),
        o: crate::TermBox::new(values[2].clone()),
    });
    let mut ids = Vec::new();
    builder.intern_batch(&values, |_, id| ids.push(id)).unwrap();
    for &id in &ids[..usize::try_from(rows).unwrap()] {
        let row = crate::QuadIds {
            s: id,
            p: ids[1],
            o: ids[2],
            g: None,
        };
        builder
            .push_quad(crate::QuadIds { p: ids[0], ..row })
            .unwrap();
        builder
            .push_reifier(id, *ids.last().unwrap(), None)
            .unwrap();
        builder.push_annotation(row).unwrap();
    }
    builder.seal().unwrap()
}

#[test]
fn scan_bookkeeping_and_pins_follow_blocks_across_all_streams() {
    use std::sync::atomic::Ordering::Relaxed;
    const ROWS_PER_BLOCK: u32 = (1024 - 9) / 32;
    for rows in [0, ROWS_PER_BLOCK, ROWS_PER_BLOCK * 2 + 5, 257] {
        let image = scan_image(rows);
        let provider = Arc::new(ScanProvider {
            bytes: image.provider(),
            snapshots: std::sync::atomic::AtomicUsize::new(0),
            stopped: std::sync::atomic::AtomicBool::new(false),
        });
        // A single cache slot makes an old pin immediately observable at the
        // next summary/payload admission, including the partially filled tail.
        let session = SegmentedSession::open(
            provider.clone(),
            image.receipt(),
            SegmentedReadLimits::new(2_000_000, 1, 20_000, 10_000_000, 0),
        )
        .unwrap();
        for table in 0..3 {
            let before = provider.snapshots.load(Relaxed);
            let mut previous = before;
            let mut scan = session.stream(table, (None, None, None, crate::GraphMatch::Any));
            for index in 0..rows {
                let row = scan.next().expect("every certified row is present");
                assert_eq!(row.s.index(), u64::from(index));
                let current = provider.snapshots.load(Relaxed);
                if index % ROWS_PER_BLOCK == 0 {
                    assert_eq!(current - previous, 3, "one summary and one payload lookup");
                } else {
                    assert_eq!(current, previous, "no readiness/cache work within a block");
                }
                previous = current;
            }
            assert_eq!(scan.next(), None);
            assert_eq!(
                provider.snapshots.load(Relaxed) - before,
                3 * usize::try_from(rows.div_ceil(ROWS_PER_BLOCK)).unwrap() + 1,
            );
            assert_eq!(session.read_error(), None);
            assert!(session.evidence().peak_bytes() <= 2_000_000);
        }
        if rows != 0 {
            let mut scan = session.quads();
            assert!(scan.next().is_some());
            let (pinned, charge) = {
                let state = lock_read_state(&session.state);
                let block = &state.cache.iter().next().unwrap().block;
                assert_eq!(Arc::strong_count(block), 2);
                let pinned = Arc::downgrade(block);
                let charge = block.reservation.bytes();
                drop(state);
                (pinned, charge)
            };
            let live = session.evidence().live_bytes();
            assert!(lock_read_state(&session.state).evict_one().unwrap());
            assert!(pinned.upgrade().is_some());
            assert_eq!(session.evidence().live_bytes(), live);
            let before = provider.snapshots.load(Relaxed);
            assert_eq!(scan.next().unwrap().s.index(), 1);
            assert_eq!(provider.snapshots.load(Relaxed), before);
            drop(scan);
            assert!(
                pinned.upgrade().is_none(),
                "early drop releases the original block grant"
            );
            assert_eq!(session.evidence().live_bytes(), live - charge);
        }
    }
}

#[test]
fn scan_block_boundary_keeps_the_first_fault_and_refuses_completion() {
    use std::sync::atomic::Ordering::Relaxed;
    let image = scan_image(67);
    let provider = Arc::new(ScanProvider {
        bytes: image.provider(),
        snapshots: std::sync::atomic::AtomicUsize::new(0),
        stopped: std::sync::atomic::AtomicBool::new(false),
    });
    let session = SegmentedSession::open(
        provider.clone(),
        image.receipt(),
        SegmentedReadLimits::new(2_000_000, 1, 2000, 10_000_000, 0),
    )
    .unwrap();
    let mut scan = session.quads();
    assert!(scan.next().is_some());
    provider.stopped.store(true, Relaxed);
    assert_eq!(
        scan.by_ref().count(),
        30,
        "already authenticated rows finish their block"
    );
    let first = session.read_error().unwrap();
    assert_eq!(
        first,
        super::SegmentedError::Stopped(crate::StopCause::Cancelled)
    );
    let requests = session.evidence().request_count();
    provider.stopped.store(false, Relaxed);
    assert_eq!(scan.next(), None);
    assert_eq!(session.read_error(), Some(first.clone()));
    assert_eq!(session.evidence().request_count(), requests);
    assert_eq!(
        session.checked_read(|view| view.quads().count()),
        Err(first)
    );
}
