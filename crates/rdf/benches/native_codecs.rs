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

fn nquads_fixture(rows: usize) -> String {
    let mut out = String::with_capacity(rows * 140);
    for idx in 0..rows {
        out.push_str(&format!(
            "<https://example.org/s{idx}> <https://example.org/p> \"{idx}\" <https://example.org/g{}> .\n",
            idx % 8
        ));
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

criterion_group!(benches, bench_parse_nquads, bench_serialize_nquads);
criterion_main!(benches);
