// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench entry points and probe helpers are private measurement tooling, not API.
#![allow(missing_docs)]

//! Report-only allocation demand of snapshot emission through `SnapshotBuilder`.
//!
//! `cargo bench -p purrdf-rdf --bench gts_emit_alloc`. The fixture is parsed
//! before any window opens. Each case opens one whole-process window, ingests the
//! datasets into a builder (the sample taken there is what the builder holds),
//! then meters the emission call as a leg, so the window's peak covers the
//! builder, the payload and the compressed output together. Emission's peak above
//! what the builder held is the window peak less that sample. Counts and bytes
//! only; no time is measured.
//!
//! Instruction counts per call come from the host's `perf stat -e
//! instructions:u` over this binary, never from a clock here. Set
//! `PURRDF_GTS_EMIT_CALLS=<n>` to run `n` calls and nothing else, with
//! `PURRDF_GTS_EMIT_MODE=ingest` (builder construction alone) or `emit`
//! (construction and emission) and `PURRDF_GTS_EMIT_QUADS=<n>`; the per-call
//! figure is the difference between an `n`-call run and a zero-call run, over `n`.

use purrdf_alloc_probe::{CountingAllocator, WholeProcessWindow};
use purrdf_gts::wire::canonical;

#[path = "../tests/support/gts_emit.rs"]
mod fixture;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

const ROUTINE_QUADS: [usize; 2] = [4_096, 16_384];

fn chains() -> [(&'static str, Option<Vec<String>>); 2] {
    [("zstd", None), ("empty", Some(Vec::new()))]
}

fn env_usize(name: &str) -> Option<usize> {
    std::env::var(name).ok().map(|value| {
        value
            .parse()
            .unwrap_or_else(|_| panic!("{name} must be a decimal count, got {value:?}"))
    })
}

fn calls(n: usize, quads: usize, emit: bool) {
    let datasets = fixture::datasets(quads);
    for _ in 0..n {
        let builder = fixture::builder(&datasets);
        if emit {
            std::hint::black_box(fixture::emit(builder, None, false, false));
        } else {
            std::hint::black_box(builder);
        }
    }
}

fn main() {
    if let Some(n) = env_usize("PURRDF_GTS_EMIT_CALLS") {
        let quads = env_usize("PURRDF_GTS_EMIT_QUADS").unwrap_or(ROUTINE_QUADS[0]);
        let emit = match std::env::var("PURRDF_GTS_EMIT_MODE").as_deref() {
            Ok("emit") => true,
            Ok("ingest") => false,
            other => panic!("PURRDF_GTS_EMIT_MODE must be ingest or emit, got {other:?}"),
        };
        calls(n, quads, emit);
        return;
    }
    for quads in ROUTINE_QUADS {
        let datasets = fixture::datasets(quads);
        let payload_bytes = canonical(&fixture::builder(&datasets).snapshot_payload()).len();
        for (name, chain) in chains() {
            let window = WholeProcessWindow::open();
            let builder = fixture::builder(&datasets);
            let held = window.sample();
            let (out, leg) = window.metered(|| fixture::emit(builder, chain.clone(), false, false));
            let whole = window.close();
            let out_bytes = out.len();
            drop(out);
            let above = whole.peak_working_bytes - held.retained_bytes;
            println!(
                "[gts_emit_alloc] chain={name} quads={quads} payload_bytes={payload_bytes} out_bytes={out_bytes} builder_held_bytes={} ingest_allocations={} emit_allocations={} emit_requested_bytes={} emit_retained_bytes={} window_peak_bytes={} emit_peak_above_builder={above} emit_allocations_per_quad={:.3}",
                held.retained_bytes,
                held.allocations,
                leg.allocations,
                leg.requested_bytes,
                leg.retained_bytes,
                whole.peak_working_bytes,
                leg.allocations as f64 / quads as f64,
            );
        }
    }
}
