// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! One-shot allocation probes for the pre-change expanded JSON-LD codec.
//!
//! This process is separate from Criterion because allocator atomics would contaminate
//! latency measurements. It reports allocation count, requested bytes, retained bytes,
//! peak working bytes, and output bytes for the exact fixtures used by the timed bench.

#![allow(missing_docs)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

use purrdf_rdf::native_codecs::jsonld::{
    CompiledJsonLdContext, JsonLdSerializeOptions, parse_jsonld, serialize_dataset_to_jsonld,
    serialize_dataset_to_jsonld_with_options,
};

#[path = "support/jsonld.rs"]
mod fixture;

use fixture::{LARGE_ROWS, SMALL_ROWS, build_dataset};

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
// pointer/layout. Atomic accounting does not affect allocator ownership.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: delegated with the exact layout supplied by the caller.
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            record_allocation(layout.size());
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        record_deallocation(layout.size());
        // SAFETY: delegated with the exact pointer/layout supplied by the caller.
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: delegated with the exact pointer/layout and requested size.
        let resized = unsafe { System.realloc(pointer, layout, new_size) };
        if !resized.is_null() {
            record_deallocation(layout.size());
            record_allocation(new_size);
        }
        resized
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

#[derive(Clone, Copy)]
struct Snapshot {
    count: u64,
    requested: u64,
    live: i64,
    peak: i64,
}

fn reset_peak() -> Snapshot {
    let live = LIVE_BYTES.load(Ordering::Relaxed);
    PEAK_BYTES.store(live, Ordering::Relaxed);
    Snapshot {
        count: ALLOCATION_COUNT.load(Ordering::Relaxed),
        requested: ALLOCATION_BYTES.load(Ordering::Relaxed),
        live,
        peak: live,
    }
}

fn snapshot() -> Snapshot {
    Snapshot {
        count: ALLOCATION_COUNT.load(Ordering::Relaxed),
        requested: ALLOCATION_BYTES.load(Ordering::Relaxed),
        live: LIVE_BYTES.load(Ordering::Relaxed),
        peak: PEAK_BYTES.load(Ordering::Relaxed),
    }
}

fn report(label: &str, before: Snapshot, after: Snapshot, artifact_bytes: usize) {
    println!(
        "[jsonld_alloc] {label}: allocations={} requested_bytes={} retained_bytes={} peak_working_bytes={} artifact_bytes={artifact_bytes}",
        after.count.saturating_sub(before.count),
        after.requested.saturating_sub(before.requested),
        after.live.saturating_sub(before.live),
        after.peak.saturating_sub(before.peak),
    );
}

fn probe(rows: usize) {
    let dataset = build_dataset(rows);
    let before = reset_peak();
    let json = serialize_dataset_to_jsonld(&dataset).expect("expanded JSON-LD serialization");
    let after = snapshot();
    report(
        &format!("serialize_expanded_rows_{rows}"),
        before,
        after,
        json.len(),
    );

    let before = reset_peak();
    let parsed = parse_jsonld(json.as_bytes()).expect("expanded JSON-LD parse");
    let after = snapshot();
    report(
        &format!("parse_expanded_rows_{rows}"),
        before,
        after,
        json.len(),
    );
    black_box(parsed);
    black_box(json);

    let caller = JsonLdSerializeOptions::compiled(std::sync::Arc::new(
        CompiledJsonLdContext::from_prefixes([
            ("ex", "https://example.org/"),
            ("p", "https://example.org/p/"),
            ("o", "https://example.org/o/"),
        ])
        .expect("compile context"),
    ));
    for (mode, options) in [
        ("caller", caller),
        ("derived", JsonLdSerializeOptions::derived()),
    ] {
        let before = reset_peak();
        let json = serialize_dataset_to_jsonld_with_options(&dataset, &options)
            .expect("configured JSON-LD serialization");
        let after = snapshot();
        report(
            &format!("serialize_{mode}_rows_{rows}"),
            before,
            after,
            json.len(),
        );

        let before = reset_peak();
        let parsed = parse_jsonld(json.as_bytes()).expect("configured JSON-LD parse");
        let after = snapshot();
        report(
            &format!("parse_{mode}_rows_{rows}"),
            before,
            after,
            json.len(),
        );
        black_box(parsed);
        black_box(json);
    }
}

fn main() {
    probe(SMALL_ROWS);
    probe(LARGE_ROWS);
}
