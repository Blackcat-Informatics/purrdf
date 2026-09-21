// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One-shot allocation instrument for deterministic Pydantic emission.
//!
//! The measuring is the workspace's shared instrument: a whole-process window,
//! which is the scope the hand-rolled armed counters here kept, arms the ledger
//! around one emission and reports its four quantities separately.

use std::hint::black_box;

use purrdf_alloc_probe::{CountingAllocator, WholeProcessWindow};

#[path = "../benches/support/pydantic.rs"]
mod pydantic_support;
use pydantic_support::{Fixture, Mode, SIZES};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

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
    let window = WholeProcessWindow::open();
    let package = black_box(fixture.emit());
    let measured = window.close();
    let allocations = measured.allocations;
    let requested_bytes = measured.requested_bytes;
    let retained_bytes = measured.retained_bytes;
    let peak_live_bytes = measured.peak_working_bytes;
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
