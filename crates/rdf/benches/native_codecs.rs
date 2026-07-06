// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! Native RDF codec hot-path benchmark.
//!
//! Report-only, `cargo bench -p purrdf --bench native_codecs`. The fixture is
//! deterministic N-Quads with default and named graph rows so both parser and
//! serializer exercise dataset-capable paths.

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use purrdf_rdf::{
    ParseOptions, SerializeGraph, parse_dataset, parse_dataset_with, serialize_dataset,
};

const ROWS: usize = 2_000;

/// Rows for the parallel-vs-sequential parse comparison: ~50k rows is ~4.6 MiB,
/// comfortably above the 1 MiB chunk-parallel threshold in
/// `native_codecs::text_parse` while keeping the in-memory fixture small.
const LARGE_ROWS: usize = 50_000;

fn nquads_fixture(rows: usize) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(rows * 140);
    for idx in 0..rows {
        let _ = writeln!(
            out,
            "<https://example.org/s{idx}> <https://example.org/p> \"{idx}\" <https://example.org/g{}> .",
            idx % 8
        );
    }
    out
}

fn bench_parse_nquads(c: &mut Criterion) {
    let text = nquads_fixture(ROWS);
    let mut group = c.benchmark_group("native_codecs_parse");
    group.throughput(Throughput::Bytes(text.len() as u64));
    group.bench_function("nquads_2k", |bencher| {
        bencher.iter(|| {
            let dataset = parse_dataset(black_box(text.as_bytes()), "application/n-quads", None)
                .expect("parse");
            black_box(dataset);
        });
    });
    group.finish();
}

/// Chunk-parallel vs forced-sequential N-Quads parse over the SAME large fixture.
/// `parse_dataset` auto-selects the parallel path above the 1 MiB threshold;
/// `parse_dataset_forced_sequential` (bench-only, `#[doc(hidden)]`) pins the
/// single-threaded pipeline as the baseline. Outputs are byte-identical (the
/// determinism tests in `native_codecs::text_parse` are the gate); only wall time
/// differs.
fn bench_parse_nquads_parallel_vs_sequential(c: &mut Criterion) {
    let text = nquads_fixture(LARGE_ROWS);
    let mut group = c.benchmark_group("native_codecs_parse_50k");
    group.sample_size(10);
    group.throughput(Throughput::Bytes(text.len() as u64));
    group.bench_function("nquads_50k_sequential", |bencher| {
        bencher.iter(|| {
            let dataset = purrdf_rdf::native_codecs::parse_dataset_forced_sequential(
                black_box(text.as_bytes()),
                "application/n-quads",
                None,
            )
            .expect("parse");
            black_box(dataset);
        });
    });
    group.bench_function("nquads_50k_parallel", |bencher| {
        bencher.iter(|| {
            let dataset = parse_dataset(black_box(text.as_bytes()), "application/n-quads", None)
                .expect("parse");
            black_box(dataset);
        });
    });
    group.finish();
}

/// N-Quads fixture whose literals carry quote / backslash / tab / newline / C0-control
/// escapes. Parsing decodes them to raw chars, so serialization drives the escape
/// fast-path's BOUNDARY branch (not just the clean-run copy the numeric fixture
/// exercises). Every escape is a valid N-Quads ECHAR/UCHAR so the fixture round-trips.
fn nquads_fixture_escape_heavy(rows: usize) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(rows * 180);
    for idx in 0..rows {
        let _ = writeln!(
            out,
            "<https://example.org/s{idx}> <https://example.org/p> \"q\\\"{idx}\\\" back\\\\ tab\\t nl\\n ctl\\u0001\" <https://example.org/g{}> .",
            idx % 8
        );
    }
    out
}

/// Span-tracking OFF vs ON over the SAME N-Quads fixture, REPORT-ONLY. The `off`
/// arm is `parse_dataset_with(track_source_spans=false)`, which threads the
/// zero-sized `NoSpans` collector — the same disabled-recording path
/// `parse_dataset` compiles to; the `on` arm sets `track_source_spans=true`,
/// forcing the sequential line pipeline and populating a `SpanTable`. This exists
/// so the `off` path can be OBSERVED in the criterion report to be unchanged; it
/// asserts NOTHING about relative timing (the machine is not quiet and numbers are
/// indicative only) — the byte-identical dataset guarantee is proven by the
/// `parse::tests` (`tracking_off_returns_no_table`, `dataset_is_identical_with_tracking`).
fn bench_parse_nquads_span_tracking(c: &mut Criterion) {
    let text = nquads_fixture(ROWS);
    let mut group = c.benchmark_group("native_codecs_parse_span_tracking");
    group.throughput(Throughput::Bytes(text.len() as u64));
    group.bench_function("nquads_2k_spans_off", |bencher| {
        bencher.iter(|| {
            let options = ParseOptions {
                track_source_spans: false,
            };
            let (dataset, table) = parse_dataset_with(
                black_box(text.as_bytes()),
                "application/n-quads",
                None,
                black_box(&options),
            )
            .expect("parse");
            black_box((dataset, table));
        });
    });
    group.bench_function("nquads_2k_spans_on", |bencher| {
        bencher.iter(|| {
            let options = ParseOptions {
                track_source_spans: true,
            };
            let (dataset, table) = parse_dataset_with(
                black_box(text.as_bytes()),
                "application/n-quads",
                None,
                black_box(&options),
            )
            .expect("parse");
            black_box((dataset, table));
        });
    });
    group.finish();
}

fn bench_serialize_nquads(c: &mut Criterion) {
    let clean = nquads_fixture(ROWS);
    let clean_ds = parse_dataset(clean.as_bytes(), "application/n-quads", None).expect("parse");
    let dirty = nquads_fixture_escape_heavy(ROWS);
    let dirty_ds = parse_dataset(dirty.as_bytes(), "application/n-quads", None).expect("parse");

    let mut group = c.benchmark_group("native_codecs_serialize");
    group.throughput(Throughput::Elements(clean_ds.quad_count() as u64));
    // Clean literals: the escape scan finds no boundary, one wholesale copy per literal.
    group.bench_function("nquads_2k", |bencher| {
        bencher.iter(|| {
            let bytes = serialize_dataset(
                black_box(&clean_ds),
                "application/n-quads",
                SerializeGraph::Dataset,
            )
            .expect("serialize");
            black_box(bytes);
        });
    });
    // Escape-heavy literals: every literal has multiple boundary chars, exercising the
    // per-char fallback interleaved with clean-run copies.
    group.bench_function("nquads_2k_escape_heavy", |bencher| {
        bencher.iter(|| {
            let bytes = serialize_dataset(
                black_box(&dirty_ds),
                "application/n-quads",
                SerializeGraph::Dataset,
            )
            .expect("serialize");
            black_box(bytes);
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_parse_nquads,
    bench_parse_nquads_parallel_vs_sequential,
    bench_parse_nquads_span_tracking,
    bench_serialize_nquads
);
criterion_main!(benches);
