// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`, which
// would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! Timed benchmark of the real CLI pack path, broken into the THREE phases of a pack
//! operation so each cost is legible on its own rather than lumped:
//!
//! * **acquisition** — building an [`ImmutableInput`] from disk (the Tier-1 sealed
//!   `memfd` snapshot the common case takes) versus from an owned buffer (the Tier-2
//!   path stdin and non-Linux take);
//! * **verification** — the unconditional canonical `verify_pack` every read/reason
//!   path runs on open;
//! * **reasoning** — materializing an RDFS closure over the zero-copy `PackView`
//!   directly versus over an owned `RdfDataset` rebuilt from the view
//!   (`dataset_from_view`) — the rebuild the DatasetView boundary now avoids.
//!
//! Report-only (not a gate). Peak-allocation evidence for the same paths lives in the
//! separate `pack_input_alloc` bench, whose tracking allocator would otherwise skew
//! these latencies.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use purrdf_cli::immutable::ImmutableInput;
use purrdf_core::{PackView, dataset_from_view, verify_pack};
use purrdf_entail::{Materialization, materialize};

#[path = "support/pack.rs"]
mod support;

/// Phase 1: build an [`ImmutableInput`] from disk (Tier-1 sealed `memfd`) vs from an
/// owned buffer (Tier 2).
fn acquisition(c: &mut Criterion) {
    // `file` is held for the whole benchmark so `path` stays valid; the acquisition
    // paths read the bytes themselves (Tier 1 maps the fd, Tier 2 reads the file).
    let (file, _bytes) = support::large_pack();
    let path = file.path().to_str().expect("utf-8 path").to_owned();

    let mut group = c.benchmark_group("pack_acquisition");
    group.bench_function("tier1_sealed_memfd", |b| {
        b.iter(|| {
            let input = ImmutableInput::from_disk_path(&path).expect("acquire from disk");
            black_box(input.as_bytes().len())
        });
    });
    // The owned path reads the whole file into a heap buffer each time (what stdin and
    // non-Linux take), a fair counterpart to the Tier-1 disk acquisition above.
    group.bench_function("tier2_owned_read", |b| {
        b.iter(|| {
            let read = std::fs::read(&path).expect("read pack bytes");
            let input = ImmutableInput::from_owned(read);
            black_box(input.as_bytes().len())
        });
    });
    group.finish();
}

/// Phase 2: the unconditional canonical `verify_pack` run on every pack open.
fn verification(c: &mut Criterion) {
    let (_file, bytes) = support::large_pack();
    let mut group = c.benchmark_group("pack_verification");
    group.bench_function("verify_pack", |b| {
        b.iter(|| black_box(verify_pack(black_box(&bytes)).expect("verify")));
    });
    group.finish();
}

/// Phase 3: materialize an RDFS closure over the zero-copy `PackView` vs over an owned
/// `RdfDataset` rebuilt from it (the rebuild the DatasetView boundary now avoids).
fn reasoning(c: &mut Criterion) {
    let (_file, bytes) = support::large_pack();
    let view = PackView::from_bytes(&bytes).expect("open pack view");

    let mut group = c.benchmark_group("pack_reasoning_rdfs");
    // The path this change preserves: the reasoner is seeded from the view directly.
    group.bench_function("over_pack_view", |b| {
        b.iter(|| black_box(materialize(&view, Materialization::Rdfs).expect("materialize")));
    });
    // The path it avoids: rebuild an owned dataset from the view, then reason over it.
    group.bench_function("over_rebuilt_dataset", |b| {
        b.iter(|| {
            let rebuilt = dataset_from_view(&view).expect("rebuild owned dataset");
            black_box(materialize(&*rebuilt, Materialization::Rdfs).expect("materialize"))
        });
    });
    group.finish();
}

criterion_group!(benches, acquisition, verification, reasoning);
criterion_main!(benches);
