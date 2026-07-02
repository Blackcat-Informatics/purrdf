// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Native RDF codec hot-path benchmark.
//!
//! Report-only, `cargo bench -p purrdf --bench native_codecs`. The fixture is
//! deterministic N-Quads with default and named graph rows so both parser and
//! serializer exercise dataset-capable paths.

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use purrdf_rdf::{parse_dataset, serialize_dataset, SerializeGraph};

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

fn bench_serialize_nquads(c: &mut Criterion) {
    let text = nquads_fixture(ROWS);
    let dataset = parse_dataset(text.as_bytes(), "application/n-quads", None).expect("parse");
    let mut group = c.benchmark_group("native_codecs_serialize");
    group.throughput(Throughput::Elements(dataset.quad_count() as u64));
    group.bench_function("nquads_2k", |bencher| {
        bencher.iter(|| {
            let bytes = serialize_dataset(
                black_box(&dataset),
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
    bench_serialize_nquads
);
criterion_main!(benches);
