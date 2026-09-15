// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Report-only latency measurements; allocation probes run in a separate process.

// Criterion emits a public entry point in a non-library benchmark target.
#![allow(missing_docs)]

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use purrdf_json::{SourceDocument, analyze, decode_document, encode, project};
use std::hint::black_box;

#[path = "support/fixture.rs"]
mod fixture;

fn benchmarks(criterion: &mut Criterion) {
    let profile = fixture::profile();
    let mut group = criterion.benchmark_group("ordered_json");
    for rows in fixture::SIZES {
        let text = fixture::document(rows);
        let source = SourceDocument {
            id: fixture::SOURCE,
            bytes: text.as_bytes(),
        };
        let model = analyze(source, &profile).unwrap();
        let dataset = project(&model, &profile).unwrap();
        group.throughput(Throughput::Bytes(text.len() as u64));
        group.bench_function(BenchmarkId::new("analyze", rows), |bench| {
            bench.iter(|| analyze(black_box(source), &profile).unwrap());
        });
        group.bench_function(BenchmarkId::new("project", rows), |bench| {
            bench.iter(|| project(black_box(&model), &profile).unwrap());
        });
        group.bench_function(BenchmarkId::new("encode", rows), |bench| {
            bench.iter(|| encode(black_box(source), &profile).unwrap());
        });
        group.bench_function(BenchmarkId::new("decode", rows), |bench| {
            bench.iter(|| decode_document(black_box(&dataset), model.id(), &profile).unwrap());
        });
    }
    group.finish();
}

criterion_group!(benches, benchmarks);
criterion_main!(benches);
