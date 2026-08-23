// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// This bench is a plain `main`, but the workspace `missing_docs` lint applies to its
// items just the same; the reporting helpers below are internal probes, not API.
#![allow(missing_docs)]

//! Peak-allocation evidence for the CLI pack path, in a separate process from the
//! timed `pack_input` bench so the tracking allocator's atomics do not skew latency.
//!
//! For each phase it reports two DISTINCT numbers, on purpose:
//!
//! * **`peak_allocated_bytes`** — the high-water mark of bytes requested through the
//!   global allocator (heap: `Vec`, interner tables, the closure). Memory-mapped pages
//!   never pass through the allocator, so this does NOT include an `mmap`.
//! * **`rss_delta_kb`** — the change in the process resident set (`/proc/self/statm`),
//!   which DOES include memory-mapped pages the allocator never sees.
//!
//! The two together tell the real story: the mmap-backed / owned pack paths have small
//! *allocated bytes* but the mapping shows up in RSS, while the owned-`RdfDataset`
//! rebuild has large *allocated bytes*. Conflating the two would hide exactly the
//! trade-off this benchmark exists to quantify.

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicI64, Ordering};

use purrdf_cli::immutable::ImmutableInput;
use purrdf_core::{DatasetView as _, PackView, dataset_from_view};
use purrdf_entail::{Materialization, materialize};

#[path = "support/pack.rs"]
mod support;

static LIVE_BYTES: AtomicI64 = AtomicI64::new(0);
static PEAK_BYTES: AtomicI64 = AtomicI64::new(0);

fn to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn record_allocation(size: usize) {
    let size = to_i64(size);
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
    LIVE_BYTES.fetch_sub(to_i64(size), Ordering::Relaxed);
}

struct CountingAllocator;

// SAFETY: every operation delegates to `System` with the exact incoming pointer and
// layout; the atomic accounting does not affect allocator ownership.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: delegated with the caller's exact layout.
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            record_allocation(layout.size());
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        record_deallocation(layout.size());
        // SAFETY: delegated with the caller's exact pointer/layout.
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: delegated with the caller's exact pointer/layout and requested size.
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

/// Set the peak high-water mark to the current live bytes, returning that baseline.
fn reset_peak() -> i64 {
    let live = LIVE_BYTES.load(Ordering::Relaxed);
    PEAK_BYTES.store(live, Ordering::Relaxed);
    live
}

/// Peak allocated bytes since `baseline` (the value [`reset_peak`] returned).
fn peak_since(baseline: i64) -> i64 {
    PEAK_BYTES.load(Ordering::Relaxed).saturating_sub(baseline)
}

/// The process resident set size in KiB, read from `/proc/self/statm` (field 2 is the
/// resident page count). Linux-only; on any other platform this reports `0` and only
/// the allocator figures carry the evidence.
fn rss_kb() -> u64 {
    #[cfg(target_os = "linux")]
    {
        let statm = std::fs::read_to_string("/proc/self/statm").unwrap_or_default();
        let resident_pages: u64 = statm
            .split_whitespace()
            .nth(1)
            .and_then(|field| field.parse().ok())
            .unwrap_or(0);
        // 4 KiB pages on every Linux target this runs on; a report-only figure.
        resident_pages * 4
    }
    #[cfg(not(target_os = "linux"))]
    {
        0
    }
}

fn report(label: &str, peak_allocated_bytes: i64, rss_before_kb: u64, rss_after_kb: u64) {
    println!(
        "[pack_input_alloc] {label}: peak_allocated_bytes={peak_allocated_bytes} \
         rss_delta_kb={}",
        i64::try_from(rss_after_kb).unwrap_or(i64::MAX)
            - i64::try_from(rss_before_kb).unwrap_or(i64::MAX),
    );
}

fn main() {
    let (file, bytes) = support::large_pack();
    let path = file.path().to_str().expect("utf-8 path").to_owned();
    println!("[pack_input_alloc] fixture pack is {} bytes", bytes.len());

    // Acquisition, Tier 1 (sealed-memfd mmap): one O(n) copy into the memfd, no
    // per-term heap growth.
    {
        let rss_before = rss_kb();
        let base = reset_peak();
        let input = ImmutableInput::from_disk_path(&path).expect("acquire from disk");
        black_box(input.as_bytes().len());
        let peak = peak_since(base);
        report("acquire_tier1_sealed_memfd", peak, rss_before, rss_kb());
    }

    // Acquisition, Tier 2 (owned buffer): the bytes live on the heap.
    {
        let rss_before = rss_kb();
        let base = reset_peak();
        let input = ImmutableInput::from_owned(bytes.clone());
        black_box(input.as_bytes().len());
        let peak = peak_since(base);
        report("acquire_tier2_owned", peak, rss_before, rss_kb());
    }

    let view = PackView::from_bytes(&bytes).expect("open pack view");

    // Reasoning over the zero-copy PackView (no rebuild): the closure is built once.
    {
        let rss_before = rss_kb();
        let base = reset_peak();
        let (closure, _report) =
            materialize(&view, Materialization::Rdfs).expect("materialize over view");
        black_box(closure.quads().count());
        let peak = peak_since(base);
        report("reason_over_pack_view", peak, rss_before, rss_kb());
    }

    // Reasoning over an owned RdfDataset rebuilt from the view (the path R2 avoids):
    // the rebuild's interned tables are allocated ON TOP of the closure.
    {
        let rss_before = rss_kb();
        let base = reset_peak();
        let rebuilt = dataset_from_view(&view).expect("rebuild owned dataset");
        let (closure, _report) =
            materialize(&*rebuilt, Materialization::Rdfs).expect("materialize over rebuilt");
        black_box(closure.quads().count());
        let peak = peak_since(base);
        report("reason_over_rebuilt_dataset", peak, rss_before, rss_kb());
    }
}
