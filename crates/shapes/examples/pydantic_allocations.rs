// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! One-shot allocation instrument for deterministic Pydantic emission.

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

#[path = "../benches/support/pydantic.rs"]
mod pydantic_support;
use pydantic_support::{Fixture, Mode, SIZES};

struct CountingAllocator;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static REQUESTED_BYTES: AtomicUsize = AtomicUsize::new(0);
static LIVE_BYTES: AtomicUsize = AtomicUsize::new(0);
static PEAK_LIVE_BYTES: AtomicUsize = AtomicUsize::new(0);
static ACTIVE: AtomicBool = AtomicBool::new(false);

fn record_allocation(bytes: usize) {
    if !ACTIVE.load(Ordering::Relaxed) {
        return;
    }
    ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
    REQUESTED_BYTES.fetch_add(bytes, Ordering::Relaxed);
    let live = LIVE_BYTES.fetch_add(bytes, Ordering::Relaxed) + bytes;
    PEAK_LIVE_BYTES.fetch_max(live, Ordering::Relaxed);
}

fn record_deallocation(bytes: usize) {
    if ACTIVE.load(Ordering::Relaxed) {
        LIVE_BYTES.fetch_sub(bytes, Ordering::Relaxed);
    }
}

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            record_allocation(layout.size());
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            record_allocation(layout.size());
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        record_deallocation(layout.size());
        unsafe { System.dealloc(pointer, layout) };
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let replacement = unsafe { System.realloc(pointer, layout, new_size) };
        if !replacement.is_null() {
            record_deallocation(layout.size());
            record_allocation(new_size);
        }
        replacement
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn reset() {
    ACTIVE.store(false, Ordering::Relaxed);
    ALLOCATIONS.store(0, Ordering::Relaxed);
    REQUESTED_BYTES.store(0, Ordering::Relaxed);
    LIVE_BYTES.store(0, Ordering::Relaxed);
    PEAK_LIVE_BYTES.store(0, Ordering::Relaxed);
    ACTIVE.store(true, Ordering::Relaxed);
}

fn main() {
    println!(
        "mode,definitions,allocations,requested_bytes,retained_bytes,peak_live_bytes,artifact_bytes,files"
    );
    for definitions in SIZES {
        for mode in Mode::ALL {
            measure(&Fixture::new(definitions, mode));
        }
    }
    measure(&Fixture::maximum_high_fanout());
}

fn measure(fixture: &Fixture) {
    black_box(fixture.emit());
    reset();
    let package = black_box(fixture.emit());
    let allocations = ALLOCATIONS.load(Ordering::Relaxed);
    let requested_bytes = REQUESTED_BYTES.load(Ordering::Relaxed);
    let retained_bytes = LIVE_BYTES.load(Ordering::Relaxed);
    let peak_live_bytes = PEAK_LIVE_BYTES.load(Ordering::Relaxed);
    ACTIVE.store(false, Ordering::Relaxed);
    let artifact_bytes = package.artifacts.values().map(Vec::len).sum::<usize>();
    let files = package.artifacts.len();
    println!(
        "{},{},{allocations},{requested_bytes},{retained_bytes},{peak_live_bytes},{artifact_bytes},{files}",
        fixture.mode.label(),
        fixture.definitions
    );
    black_box(&package);
    drop(package);
}
