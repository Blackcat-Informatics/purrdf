// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One-shot allocation probes for deterministic PURREMB workloads.
//!
//! This executable is deliberately separate from the timed Criterion harness:
//! its global allocator accounts for every allocation, which would otherwise
//! contaminate latency and throughput measurements.
//!
//! Each phase is bracketed by a [`WholeProcessWindow`] from the workspace's
//! shared instrument, so a phase that fans out over worker threads is counted
//! where it allocates rather than reported as nearly free.

use std::hint::black_box;

use purrdf_alloc_probe::{CountingAllocator, Measurement, WholeProcessWindow};
use purrdf_core::{EmbeddingView, verify_embedding};

#[allow(
    dead_code,
    reason = "the shared fixture module also exposes accessors used only by the timed process"
)]
#[path = "support/purremb.rs"]
mod fixture;

use fixture::{build_catalog_fixture, build_f32_fixture, build_f64_fixture};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn report(label: &str, measured: Measurement) {
    println!(
        "[purremb_alloc] {label}: allocations={} requested_bytes={} retained_bytes={} peak_working_bytes={}",
        measured.allocations,
        measured.requested_bytes,
        measured.retained_bytes,
        measured.peak_working_bytes
    );
}

fn main() {
    let window = WholeProcessWindow::open();
    let f32_fixture = build_f32_fixture();
    report("f32_fixture_build", window.close());
    println!(
        "[purremb_alloc] f32_fixture artifact_bytes={}",
        f32_fixture.bytes.len()
    );

    let window = WholeProcessWindow::open();
    {
        let mut view = EmbeddingView::from_bytes(&f32_fixture.bytes).expect("f32 view");
        black_box(verify_embedding(&mut view).expect("verified f32 fixture"));
    }
    report("f32_full_verify", window.close());

    let window = WholeProcessWindow::open();
    let streamed = f32_fixture.stream_once();
    let measured = window.close();
    assert_eq!(streamed, f32_fixture.bytes, "streaming output is canonical");
    report("f32_streaming_write", measured);
    drop(streamed);

    let window = WholeProcessWindow::open();
    let f64_fixture = build_f64_fixture();
    report("f64_fixture_build", window.close());
    println!(
        "[purremb_alloc] f64_fixture artifact_bytes={}",
        f64_fixture.bytes.len()
    );

    let window = WholeProcessWindow::open();
    let catalog = build_catalog_fixture();
    report("chunk_catalog_build", window.close());
    println!(
        "[purremb_alloc] chunk_catalog chunks={} artifact_bytes={}",
        catalog.chunk_count,
        catalog.bytes.len()
    );

    let window = WholeProcessWindow::open();
    {
        let mut view = EmbeddingView::from_bytes(&catalog.bytes).expect("catalog view");
        black_box(verify_embedding(&mut view).expect("verified catalog"));
    }
    report("chunk_catalog_full_verify", window.close());
}
