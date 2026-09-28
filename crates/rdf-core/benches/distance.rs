// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! Report-only latency harness for the `purrdf_core::distance` kernels, under both
//! arithmetics.
//!
//! Every public kernel entry point a ranked-retrieval surface calls is driven here, once
//! under [`Exact`] and once under [`Reassociated`], over the same deterministic inputs:
//!
//! * `distance_batch` -- [`Resolved::distances`], every row of the matrix against one
//!   query (the kNN relation's exhaustive scan);
//! * `distance_indexed` -- [`Resolved::distances_indexed`], a fixed id list against one
//!   query (the HNSW beam's per-node neighbour batch);
//! * `distance_pair` -- [`Resolved::distance`], one pair (HNSW's per-pair calls);
//! * `distance_bounded` -- [`Resolved::distance_bounded`], one pair under a bound that is
//!   never met (`inf`, the full fold) and one that is met halfway (`half`, the early
//!   abandon HNSW's neighbour selection takes).
//!
//! The batch and indexed groups run the dot fold (`NegativeDot`) and the squared-Euclidean
//! fold (`SquaredEuclidean`); the pair group the dot fold; the bounded group the
//! squared-Euclidean fold, the only measure whose partial sums are monotone.
//!
//! Benchmark ids are `<group>/<arithmetic>/<measure>/<query>x<row>/d<dims>`, so the SIMD
//! audit (`docs/design/purrdf-simd.md` §4.2) can name the `distance.exact.*` and
//! `distance.reassociated.*` sites each id exercises. The width pairs are the three the
//! workspace calls: `f64xf64` (the kNN relation over a binary64 space), `f64xf32` (an
//! arbitrary query against binary32 rows) and `f32xf32` (a stored binary32 row as the
//! query). The dimensions are common production embedding widths.
//!
//! All inputs come from a fixed splitmix64 stream. No timing or speedup is asserted --
//! criterion's stdout summary is the report, and the two arithmetics are reported side by
//! side rather than divided into each other.

use std::hint::black_box;
use std::time::Duration;

use criterion::measurement::WallTime;
use criterion::{BenchmarkGroup, Criterion, Throughput, criterion_group, criterion_main};
use purrdf_core::distance::{
    Arithmetic, Bound, Exact, Measure, Reassociated, Resolved, RowsRef, Scalar,
};
use purrdf_testkit::rng::splitmix64_step;

/// The rows in every matrix.
const ROWS: usize = 4_096;

/// The rows the indexed batch scores per call: a beam-sized neighbour list.
const INDEXED: usize = 512;

/// The production embedding widths measured.
const DIMS: [usize; 3] = [384, 768, 1_536];

/// The seed of the fixture stream.
const SEED: u64 = 0x4449_5354_414e_4345;

/// `len` values in `[-1, 1)` from the fixed stream at `seed`, none exactly zero, each
/// exactly representable in binary32 so the `f32` copy holds the same values.
fn stream(len: usize, seed: u64) -> Vec<f64> {
    let mut state = seed;
    (0..len)
        .map(|_| {
            state = splitmix64_step(state);
            let unit = (state >> 11) as f64 / (1_u64 << 53) as f64;
            let value = f64::from(unit.mul_add(2.0, -1.0) as f32);
            if value == 0.0 { 0.25 } else { value }
        })
        .collect()
}

/// The narrow copy of `values`. Exact: [`stream`] only yields binary32 values.
fn narrow(values: &[f64]) -> Vec<f32> {
    values.iter().map(|&value| value as f32).collect()
}

/// One dimension's inputs, in both stored widths.
struct Fixture {
    dims: usize,
    query_f64: Vec<f64>,
    query_f32: Vec<f32>,
    rows_f64: Vec<f64>,
    rows_f32: Vec<f32>,
    /// A deterministic, duplicate-free id list: an odd multiplier permutes a power-of-two
    /// row count.
    ids: Vec<usize>,
}

impl Fixture {
    fn new(dims: usize) -> Self {
        let query_f64 = stream(dims, SEED ^ dims as u64);
        let rows_f64 = stream(ROWS * dims, SEED.rotate_left(17) ^ dims as u64);
        let ids = (0..INDEXED)
            .map(|i| i.wrapping_mul(2_654_435_761) % ROWS)
            .collect();
        Self {
            dims,
            query_f32: narrow(&query_f64),
            rows_f32: narrow(&rows_f64),
            query_f64,
            rows_f64,
            ids,
        }
    }
}

/// A group with the harness's modest budget: every id runs briefly, and the whole target
/// stays in the low minutes.
fn group<'c>(c: &'c mut Criterion, name: &str) -> BenchmarkGroup<'c, WallTime> {
    let mut group = c.benchmark_group(name);
    group.sample_size(10);
    group.warm_up_time(Duration::from_millis(300));
    group.measurement_time(Duration::from_secs(1));
    group
}

const fn measure_name(measure: Measure) -> &'static str {
    match measure {
        Measure::Cosine => "cosine",
        Measure::NegativeDot => "dot",
        Measure::SquaredEuclidean => "sqeuclid",
    }
}

/// The two batch measures: the dot fold and the squared-Euclidean fold.
const BATCH_MEASURES: [Measure; 2] = [Measure::NegativeDot, Measure::SquaredEuclidean];

/// One arithmetic's handle with the label its ids carry.
#[derive(Clone, Copy)]
struct Law<A: Arithmetic> {
    label: &'static str,
    resolved: Resolved<A>,
}

fn batch<A: Arithmetic, Q: Scalar, T: Scalar>(
    group: &mut BenchmarkGroup<'_, WallTime>,
    law: Law<A>,
    pair: &str,
    dims: usize,
    query: &[Q],
    rows: &[T],
) {
    let view = RowsRef::new(rows, ROWS, dims, &[]).expect("the matrix is rectangular");
    let mut out = vec![None; ROWS];
    group.throughput(Throughput::Elements((ROWS * dims) as u64));
    for measure in BATCH_MEASURES {
        let id = format!("{}/{}/{pair}/d{dims}", law.label, measure_name(measure));
        group.bench_function(id, |b| {
            b.iter(|| {
                law.resolved
                    .distances(measure, black_box(query), 0.0, view, &mut out);
                black_box(&out);
            });
        });
    }
}

fn indexed<A: Arithmetic, Q: Scalar, T: Scalar>(
    group: &mut BenchmarkGroup<'_, WallTime>,
    law: Law<A>,
    pair: &str,
    fixture: &Fixture,
    query: &[Q],
    rows: &[T],
) {
    let dims = fixture.dims;
    let view = RowsRef::new(rows, ROWS, dims, &[]).expect("the matrix is rectangular");
    let mut out = vec![None; fixture.ids.len()];
    group.throughput(Throughput::Elements((fixture.ids.len() * dims) as u64));
    for measure in BATCH_MEASURES {
        let id = format!("{}/{}/{pair}/d{dims}", law.label, measure_name(measure));
        group.bench_function(id, |b| {
            b.iter(|| {
                law.resolved.distances_indexed(
                    measure,
                    black_box(query),
                    0.0,
                    view,
                    black_box(&fixture.ids),
                    &mut out,
                );
                black_box(&out);
            });
        });
    }
}

fn pair_dot<A: Arithmetic, Q: Scalar, T: Scalar>(
    group: &mut BenchmarkGroup<'_, WallTime>,
    law: Law<A>,
    pair: &str,
    dims: usize,
    query: &[Q],
    row: &[T],
) {
    group.throughput(Throughput::Elements(dims as u64));
    let id = format!("{}/dot/{pair}/d{dims}", law.label);
    group.bench_function(id, |b| {
        b.iter(|| {
            black_box(law.resolved.distance(
                Measure::NegativeDot,
                black_box(query),
                0.0,
                black_box(row),
                0.0,
            ))
        });
    });
}

fn bounded<A: Arithmetic, Q: Scalar, T: Scalar>(
    group: &mut BenchmarkGroup<'_, WallTime>,
    law: Law<A>,
    pair: &str,
    dims: usize,
    query: &[Q],
    row: &[T],
) {
    // The half bound is taken from the exact distance so both arithmetics abandon at the
    // same threshold; it is met near the midpoint of the fold under either one.
    let full = exact()
        .resolved
        .distance(Measure::SquaredEuclidean, query, 0.0, row, 0.0)
        .expect("the fixture is finite");
    group.throughput(Throughput::Elements(dims as u64));
    for (bound_label, bound) in [
        ("inf", Bound::Above(f64::INFINITY)),
        ("half", Bound::Above(full / 2.0)),
    ] {
        let id = format!("{}/sqeuclid-{bound_label}/{pair}/d{dims}", law.label);
        group.bench_function(id, |b| {
            b.iter(|| {
                black_box(law.resolved.distance_bounded(
                    Measure::SquaredEuclidean,
                    black_box(query),
                    0.0,
                    black_box(row),
                    0.0,
                    bound,
                ))
            });
        });
    }
}

/// Row `r`'s components out of a row-major matrix.
fn row<T>(rows: &[T], dims: usize, r: usize) -> &[T] {
    &rows[r * dims..(r + 1) * dims]
}

fn exact() -> Law<Exact> {
    Law {
        label: "exact",
        resolved: Exact::resolve().expect("the default float environment is the IEEE one"),
    }
}

fn reassociated() -> Law<Reassociated> {
    Law {
        label: "reassociated",
        resolved: Reassociated::resolve().expect("the default float environment is the IEEE one"),
    }
}

/// Every width pair of one kernel, under both arithmetics: `$call!(law, pair, query, rows)`
/// is expanded for each `(arithmetic, width pair)` so each is its own monomorphization.
macro_rules! each_law_and_pair {
    ($f:expr, $rows64:expr, $rows32:expr, |$law:ident, $pair:ident, $query:ident, $rows:ident| $body:expr) => {{
        let f = $f;
        {
            let $law = exact();
            {
                let ($pair, $query, $rows) = ("f64xf64", f.query_f64.as_slice(), $rows64);
                $body;
            }
            {
                let ($pair, $query, $rows) = ("f64xf32", f.query_f64.as_slice(), $rows32);
                $body;
            }
            {
                let ($pair, $query, $rows) = ("f32xf32", f.query_f32.as_slice(), $rows32);
                $body;
            }
        }
        {
            let $law = reassociated();
            {
                let ($pair, $query, $rows) = ("f64xf64", f.query_f64.as_slice(), $rows64);
                $body;
            }
            {
                let ($pair, $query, $rows) = ("f64xf32", f.query_f64.as_slice(), $rows32);
                $body;
            }
            {
                let ($pair, $query, $rows) = ("f32xf32", f.query_f32.as_slice(), $rows32);
                $body;
            }
        }
    }};
}

fn bench_batch(c: &mut Criterion, fixtures: &[Fixture]) {
    let mut group = group(c, "distance_batch");
    for f in fixtures {
        each_law_and_pair!(
            f,
            f.rows_f64.as_slice(),
            f.rows_f32.as_slice(),
            |law, pair, query, rows| { batch(&mut group, law, pair, f.dims, query, rows) }
        );
    }
    group.finish();
}

fn bench_indexed(c: &mut Criterion, fixtures: &[Fixture]) {
    let mut group = group(c, "distance_indexed");
    for f in fixtures {
        each_law_and_pair!(
            f,
            f.rows_f64.as_slice(),
            f.rows_f32.as_slice(),
            |law, pair, query, rows| { indexed(&mut group, law, pair, f, query, rows) }
        );
    }
    group.finish();
}

fn bench_pair(c: &mut Criterion, fixtures: &[Fixture]) {
    let mut group = group(c, "distance_pair");
    for f in fixtures {
        each_law_and_pair!(
            f,
            row(&f.rows_f64, f.dims, 1),
            row(&f.rows_f32, f.dims, 1),
            |law, pair, query, rows| { pair_dot(&mut group, law, pair, f.dims, query, rows) }
        );
    }
    group.finish();
}

fn bench_bounded(c: &mut Criterion, fixtures: &[Fixture]) {
    let mut group = group(c, "distance_bounded");
    for f in fixtures {
        each_law_and_pair!(
            f,
            row(&f.rows_f64, f.dims, 1),
            row(&f.rows_f32, f.dims, 1),
            |law, pair, query, rows| { bounded(&mut group, law, pair, f.dims, query, rows) }
        );
    }
    group.finish();
}

fn bench_distance(c: &mut Criterion) {
    let fixtures: Vec<Fixture> = DIMS.into_iter().map(Fixture::new).collect();
    bench_batch(c, &fixtures);
    bench_indexed(c, &fixtures);
    bench_pair(c, &fixtures);
    bench_bounded(c, &fixtures);
}

criterion_group!(benches, bench_distance);
criterion_main!(benches);
