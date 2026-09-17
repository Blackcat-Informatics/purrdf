// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// This bench is a plain `main`, but the workspace `missing_docs` lint applies to its
// items just the same; the reporting helpers below are internal probes, not API.
#![allow(missing_docs)]

//! Allocation and peak-memory evidence for the SHACL prepared-shapes product,
//! phase by phase.
//!
//! **Report-only. No figure here is asserted, compared against a baseline, or
//! gated on.** A prepared product is a structural change, so it needs no invented
//! speedup threshold; what it does need is an honest statement of what each phase
//! costs in memory, because the interesting risk in a "cache the parse" feature is
//! not that restoring is slow, it is that producing the cache has a peak the
//! producer never budgeted for.
//!
//! It runs in a separate process from the timed `shacl_product_reuse` target and
//! reads the same fixture module (`benches/support/product.rs`), because the
//! global allocator installed below performs atomic accounting on every
//! allocation and would contaminate any latency measurement sharing the process.
//!
//! # What each line reports
//!
//! * **`allocations`** — how many allocation calls the phase made.
//! * **`requested_bytes`** — total bytes requested through the global allocator
//!   during the phase, including memory freed again before it ended. This is the
//!   allocator *traffic*, not the footprint.
//! * **`retained_bytes`** — the change in live bytes across the phase: what the
//!   phase's result is still holding when it returns.
//! * **`peak_working_bytes`** — the high-water mark of live bytes during the
//!   phase, relative to where it started. Traffic and peak are reported
//!   separately on purpose: a phase that allocates and frees a large buffer
//!   repeatedly has high traffic and a modest peak, while a phase that assembles
//!   one large structure has the reverse, and conflating them would hide exactly
//!   the distinction between `encode/to_product` (which re-packs and
//!   canonicalizes the shapes dataset) and the restore paths.
//!
//! # The intermediate bytes
//!
//! The encoded product's length is printed here as `artifact_bytes`, and that is
//! the "intermediate bytes" the product interposes between preparation and
//! restore. It is NOT only a bench-log fact: the same quantity is pinned for the
//! determinism fixture as the asserted constant `GOLDEN_LEN` in
//! `crates/shapes/tests/product_determinism.rs`, so a codec change that doubled
//! the artifact fails the build rather than quietly changing a number in a log
//! nobody re-reads. The figure printed here is for this target's own, larger
//! fixture and is the illustration; the test's is the enforced fact.
//!
//! Run with `cargo bench -p purrdf-shapes --bench shacl_product_alloc` (the
//! `make bench` lane) — excluded from `make check`.

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

use purrdf_shapes::engine::{PreparedShapes, parse_shapes};
use purrdf_shapes::product::{HostBindings, ShapesProduct, ShapesProfile};

#[path = "support/product.rs"]
mod fixture;

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

/// One reading of the four counters.
#[derive(Clone, Copy)]
struct AllocationSnapshot {
    count: u64,
    requested: u64,
    live: i64,
    peak: i64,
}

/// Set the peak high-water mark to the current live bytes and read the counters.
///
/// Returns the baseline a later [`snapshot`] is differenced against, so each phase
/// reports its own numbers rather than the process total so far.
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

/// Read the counters without disturbing them.
fn snapshot() -> AllocationSnapshot {
    AllocationSnapshot {
        count: ALLOCATION_COUNT.load(Ordering::Relaxed),
        requested: ALLOCATION_BYTES.load(Ordering::Relaxed),
        live: LIVE_BYTES.load(Ordering::Relaxed),
        peak: PEAK_BYTES.load(Ordering::Relaxed),
    }
}

/// Print one phase's four figures.
fn report(label: &str, before: AllocationSnapshot, after: AllocationSnapshot) {
    println!(
        "[shacl_product_alloc] {label}: allocations={} requested_bytes={} retained_bytes={} \
         peak_working_bytes={}",
        after.count.saturating_sub(before.count),
        after.requested.saturating_sub(before.requested),
        after.live.saturating_sub(before.live),
        after.peak.saturating_sub(before.peak)
    );
}

fn main() {
    // Untimed, unmeasured: the fixture's own source text and data graph are the
    // input to every phase below, not one of the phases.
    let source = fixture::shapes_source();
    let data = fixture::data();
    let host = HostBindings::empty();
    println!(
        "[shacl_product_alloc] fixture shapes source is {} bytes",
        source.len()
    );

    // The cold path a product replaces: Turtle text to a reusable preparation.
    let before = reset_peak();
    let prepared = fixture::prepared(&source);
    report("cold/parse_and_prepare", before, snapshot());

    // The reusable preparation alone, over an already-parsed shapes graph: the
    // class-catalog derivation both restore paths repeat rather than carry.
    let parsed = Arc::new(parse_shapes(&source, None).expect("the bench shapes graph parses"));
    let before = reset_peak();
    let reusable = PreparedShapes::new(Arc::clone(&parsed));
    report("prepare/reusable", before, snapshot());
    drop(reusable);

    // The producer's cost, and the pipeline's peak-memory event: the shapes
    // dataset is re-packed and canonicalized so the product can state an identity
    // it has actually established.
    let before = reset_peak();
    let product = fixture::encode(&prepared);
    report("encode/to_product", before, snapshot());
    println!(
        "[shacl_product_alloc] artifact_bytes={} (the intermediate bytes; the determinism \
         fixture's equivalent is an asserted constant, not only a log line)",
        product.len()
    );

    // The structural tier alone: envelope framing and digests, admitting nothing.
    {
        let before = reset_peak();
        let view = ShapesProduct::open(&product).expect("the product opens");
        let after = snapshot();
        black_box(view.section_kinds());
        report("restore/open", before, after);
    }

    // The memo path. The open is outside the measured region so this line reports
    // admission rather than admission plus the structural tier above.
    {
        let view = ShapesProduct::open(&product).expect("the product opens");
        let before = reset_peak();
        let admitted = view
            .admit(&ShapesProfile::CORE, &host)
            .expect("the product admits");
        let after = snapshot();
        report("restore/admit", before, after);
        drop(admitted);
    }

    // The memo-free path: ignore the AST section, re-derive from the carried
    // dataset. Its retained and peak figures against `restore/admit` are what say
    // whether the model section is paying for itself.
    {
        let view = ShapesProduct::open(&product).expect("the product opens");
        let before = reset_peak();
        let rebuilt = view
            .rebuild(&ShapesProfile::CORE, &host)
            .expect("the product rebuilds");
        let after = snapshot();
        report("restore/rebuild", before, after);
        drop(rebuilt);
    }

    // Per-dataset work, reported apart from every line above: paid once per
    // snapshot no matter how the preparation was obtained.
    let before = reset_peak();
    let validator = prepared
        .bind_shared_dataset(Arc::clone(&data))
        .expect("the bench data graph binds");
    report("bind/dataset", before, snapshot());

    let before = reset_peak();
    let validation = validator.validate().expect("validation runs");
    let after = snapshot();
    report("eval/validate", before, after);
    // Outside the measured region: a phase that reached no constraint would
    // otherwise report a small, tidy, meaningless number.
    fixture::assert_non_vacuous(&validation);
    println!(
        "[shacl_product_alloc] eval/validate produced {} result(s), conforms={}",
        validation.results.len(),
        validation.conforms
    );
}
