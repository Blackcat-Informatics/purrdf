// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! One-shot allocation probes for deterministic PURREMB workloads.
//!
//! This executable is deliberately separate from the timed Criterion harness:
//! its global allocator performs atomic accounting on every allocation, which
//! would otherwise contaminate latency and throughput measurements.

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

use purrdf_core::{EmbeddingView, verify_embedding};

#[allow(
    dead_code,
    reason = "the shared fixture module also exposes accessors used only by the timed process"
)]
#[path = "support/purremb.rs"]
mod fixture;

use fixture::{build_catalog_fixture, build_f32_fixture, build_f64_fixture};

static ALLOCATION_COUNT: AtomicU64 = AtomicU64::new(0);
static ALLOCATION_BYTES: AtomicU64 = AtomicU64::new(0);
static LIVE_BYTES: AtomicI64 = AtomicI64::new(0);
static PEAK_BYTES: AtomicI64 = AtomicI64::new(0);

struct CountingAllocator;

fn usize_to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn record_allocation(size: usize) {
    ALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);
    ALLOCATION_BYTES.fetch_add(usize_to_u64(size), Ordering::Relaxed);
    let size = usize_to_i64(size);
    let live = LIVE_BYTES
        .fetch_add(size, Ordering::Relaxed)
        .saturating_add(size);
    let mut peak = PEAK_BYTES.load(Ordering::Relaxed);
    while live > peak {
        match PEAK_BYTES.compare_exchange_weak(peak, live, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(observed) => peak = observed,
        }
    }
}

fn record_deallocation(size: usize) {
    LIVE_BYTES.fetch_sub(usize_to_i64(size), Ordering::Relaxed);
}

// SAFETY: every operation delegates to `System` with the exact incoming
// pointer/layout. The atomic accounting does not affect allocator ownership.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record_allocation(layout.size());
        // SAFETY: delegated with the exact layout supplied by the caller.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        record_deallocation(layout.size());
        // SAFETY: delegated with the exact pointer/layout supplied by the caller.
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        record_deallocation(layout.size());
        record_allocation(new_size);
        // SAFETY: delegated with the exact pointer/layout and requested size.
        unsafe { System.realloc(pointer, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

#[derive(Clone, Copy)]
struct AllocationSnapshot {
    count: u64,
    requested: u64,
    live: i64,
    peak: i64,
}

fn reset_peak() -> AllocationSnapshot {
    let live = LIVE_BYTES.load(Ordering::Relaxed);
    PEAK_BYTES.store(live, Ordering::Relaxed);
    AllocationSnapshot {
        count: ALLOCATION_COUNT.load(Ordering::Relaxed),
        requested: ALLOCATION_BYTES.load(Ordering::Relaxed),
        live,
        peak: live,
    }
}

fn snapshot() -> AllocationSnapshot {
    AllocationSnapshot {
        count: ALLOCATION_COUNT.load(Ordering::Relaxed),
        requested: ALLOCATION_BYTES.load(Ordering::Relaxed),
        live: LIVE_BYTES.load(Ordering::Relaxed),
        peak: PEAK_BYTES.load(Ordering::Relaxed),
    }
}

fn report(label: &str, before: AllocationSnapshot, after: AllocationSnapshot) {
    println!(
        "[purremb_alloc] {label}: allocations={} requested_bytes={} retained_bytes={} peak_working_bytes={}",
        after.count.saturating_sub(before.count),
        after.requested.saturating_sub(before.requested),
        after.live.saturating_sub(before.live),
        after.peak.saturating_sub(before.peak)
    );
}

fn main() {
    let before = reset_peak();
    let f32_fixture = build_f32_fixture();
    report("f32_fixture_build", before, snapshot());
    println!(
        "[purremb_alloc] f32_fixture artifact_bytes={}",
        f32_fixture.bytes.len()
    );

    let before = reset_peak();
    {
        let mut view = EmbeddingView::from_bytes(&f32_fixture.bytes).expect("f32 view");
        black_box(verify_embedding(&mut view).expect("verified f32 fixture"));
    }
    report("f32_full_verify", before, snapshot());

    let before = reset_peak();
    let streamed = f32_fixture.stream_once();
    let after = snapshot();
    assert_eq!(streamed, f32_fixture.bytes, "streaming output is canonical");
    report("f32_streaming_write", before, after);
    drop(streamed);

    let before = reset_peak();
    let f64_fixture = build_f64_fixture();
    report("f64_fixture_build", before, snapshot());
    println!(
        "[purremb_alloc] f64_fixture artifact_bytes={}",
        f64_fixture.bytes.len()
    );

    let before = reset_peak();
    let catalog = build_catalog_fixture();
    report("chunk_catalog_build", before, snapshot());
    println!(
        "[purremb_alloc] chunk_catalog chunks={} artifact_bytes={}",
        catalog.chunk_count,
        catalog.bytes.len()
    );

    let before = reset_peak();
    {
        let mut view = EmbeddingView::from_bytes(&catalog.bytes).expect("catalog view");
        black_box(verify_embedding(&mut view).expect("verified catalog"));
    }
    report("chunk_catalog_full_verify", before, snapshot());
}
