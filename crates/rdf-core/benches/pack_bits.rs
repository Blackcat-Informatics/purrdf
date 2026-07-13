// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! Report-only latency harness for the succinct `pack` primitives
//! (`crate::ir::pack::bits`): [`IntVector::get`], and [`RankSelect`]'s
//! `rank1`/`select1`, each at a few sizes. No timing/speedup assertion —
//! criterion's stdout summary is the report.

use criterion::{Criterion, criterion_group, criterion_main};
use purrdf_core::ir::pack::bits::{BitVec, IntVector, RankSelect};

/// The `IntVector`/bitmap sizes exercised, small to large enough to cross several
/// rank/select superblock (512-bit) boundaries.
const SIZES: [usize; 3] = [64, 4_096, 262_144];

/// Build an `IntVector` of `n` values in a fixed, deterministic (no RNG) pattern
/// wide enough to need 17 bits (crosses word boundaries repeatedly since 17 does
/// not divide 64).
fn build_int_vector(n: usize) -> IntVector {
    let width = 17;
    let mut v = IntVector::with_width(width);
    for i in 0..n {
        v.push((i as u64 * 2_654_435_761) % (1u64 << width));
    }
    v
}

/// Build a `RankSelect` bitmap of `n` bits with a deterministic ~1-in-3 density
/// (no RNG), so `rank1`/`select1` see a realistic mix of superblocks/words.
fn build_rank_select(n: usize) -> RankSelect {
    let mut b = BitVec::new();
    for i in 0..n {
        b.push(i % 3 == 0);
    }
    b.freeze()
}

fn bench_int_vector_get(c: &mut Criterion) {
    let mut group = c.benchmark_group("pack_bits_int_vector_get");
    for &n in &SIZES {
        let v = build_int_vector(n);
        group.bench_function(format!("n={n}"), |b| {
            b.iter(|| {
                let mut acc = 0u64;
                for i in 0..v.len() {
                    acc = acc.wrapping_add(std::hint::black_box(v.get(i)));
                }
                std::hint::black_box(acc)
            });
        });
    }
    group.finish();
}

fn bench_rank1(c: &mut Criterion) {
    let mut group = c.benchmark_group("pack_bits_rank1");
    for &n in &SIZES {
        let rs = build_rank_select(n);
        group.bench_function(format!("n={n}"), |b| {
            b.iter(|| {
                let mut acc = 0usize;
                // Sample rank1 at every position, the O(1) hot path.
                for i in 0..=rs.len() {
                    acc = acc.wrapping_add(std::hint::black_box(rs.rank1(i)));
                }
                std::hint::black_box(acc)
            });
        });
    }
    group.finish();
}

fn bench_select1(c: &mut Criterion) {
    let mut group = c.benchmark_group("pack_bits_select1");
    for &n in &SIZES {
        let rs = build_rank_select(n);
        let total_ones = rs.total_ones();
        group.bench_function(format!("n={n}"), |b| {
            b.iter(|| {
                let mut acc = 0usize;
                for k in 0..total_ones {
                    if let Some(pos) = std::hint::black_box(rs.select1(k)) {
                        acc = acc.wrapping_add(pos);
                    }
                }
                std::hint::black_box(acc)
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_int_vector_get, bench_rank1, bench_select1);
criterion_main!(benches);
