// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API, so the workspace `missing_docs` lint is
// not asked of their items.
#![allow(missing_docs)]

//! Report-only latency harness for [`purrdf_core::SmallVec`] on the workspace's
//! row shapes: `IdVec` push / from_slice / collect / clone / into_iter at an
//! inline length (3, fits the 4 inline slots) and a spilled length (32), and
//! dense `Option<TermId>` rows built by `smallvec![None; w]`. No timing or
//! speedup assertion — the harness's stdout summary is the report.

use std::hint::black_box;

use purrdf_core::{IdVec, SmallVec, TermId};
use purrdf_testkit::bench::{Bench, bench_group, bench_main};

/// Row lengths measured: one that stays inline and one that spills.
const LENGTHS: [(&str, usize); 2] = [("inline", 3), ("spilled", 32)];

/// A deterministic id sequence of length `n`.
fn ids(n: usize) -> Vec<TermId> {
    (0..n as u32)
        .map(|i| TermId::from_index(i * 7 + 1))
        .collect()
}

fn bench_push(c: &mut Bench) {
    let mut group = c.benchmark_group("small_push");
    for (label, n) in LENGTHS {
        let source = ids(n);
        group.bench_function(label, |b| {
            b.iter(|| {
                let mut row = IdVec::new();
                for &id in black_box(&source) {
                    row.push(id);
                }
                black_box(row)
            });
        });
    }
    group.finish();
}

fn bench_from_slice(c: &mut Bench) {
    let mut group = c.benchmark_group("small_from_slice");
    for (label, n) in LENGTHS {
        let source = ids(n);
        group.bench_function(label, |b| {
            b.iter(|| black_box(IdVec::from_slice(black_box(&source))));
        });
    }
    group.finish();
}

fn bench_collect(c: &mut Bench) {
    let mut group = c.benchmark_group("small_collect");
    for (label, n) in LENGTHS {
        let source = ids(n);
        group.bench_function(label, |b| {
            b.iter(|| black_box(black_box(&source).iter().copied().collect::<IdVec>()));
        });
    }
    group.finish();
}

fn bench_clone(c: &mut Bench) {
    let mut group = c.benchmark_group("small_clone");
    for (label, n) in LENGTHS {
        let row = IdVec::from_slice(&ids(n));
        group.bench_function(label, |b| {
            b.iter(|| black_box(black_box(&row).clone()));
        });
    }
    group.finish();
}

fn bench_into_iter(c: &mut Bench) {
    let mut group = c.benchmark_group("small_into_iter");
    for (label, n) in LENGTHS {
        let row = IdVec::from_slice(&ids(n));
        group.bench_function(label, |b| {
            b.iter_batched(
                || row.clone(),
                |row| {
                    let mut acc = 0usize;
                    for id in row {
                        acc = acc.wrapping_add(id.index());
                    }
                    black_box(acc)
                },
                purrdf_testkit::bench::BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_none_rows(c: &mut Bench) {
    let mut group = c.benchmark_group("small_none_row");
    for (label, width) in LENGTHS {
        group.bench_function(label, |b| {
            b.iter(|| {
                let row: SmallVec<[Option<TermId>; 4]> =
                    purrdf_core::smallvec![None; black_box(width)];
                black_box(row)
            });
        });
    }
    group.finish();
}

bench_group!(
    benches,
    bench_push,
    bench_from_slice,
    bench_collect,
    bench_clone,
    bench_into_iter,
    bench_none_rows
);
bench_main!(benches);
