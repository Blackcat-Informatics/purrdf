// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! Report-only benchmark for the npm wasm SPARQL wrapper.
//!
//! The native evaluator already has broad coverage in `purrdf-sparql-eval`.
//! This harness measures the binding-level overhead that TypeScript users hit:
//! repeated SELECT calls through one reused `QueryEngine` instance, which keeps
//! the native plan cache alive, versus constructing a fresh wrapper per call.

use std::fmt::Write as _;

use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};

use purrdf_wasm::{Dataset, QueryEngine};

const SELECT_BY_OBJECT: &str = "\
PREFIX ex: <https://example.org/>
SELECT ?s WHERE { ?s ex:p ex:o7 }";

fn fixture_dataset() -> Dataset {
    let mut input = String::from("@prefix ex: <https://example.org/> .\n");
    for i in 0..512 {
        writeln!(&mut input, "ex:s{i} ex:p ex:o{} .", i % 16).expect("write fixture quad");
        writeln!(&mut input, "ex:s{i} ex:score {i} .").expect("write fixture score");
    }
    Dataset::parse(&input, "turtle", None).expect("parse bench fixture")
}

fn run_select(engine: &QueryEngine, dataset: &Dataset) -> usize {
    engine
        .select(dataset, SELECT_BY_OBJECT, None)
        .expect("SELECT succeeds")
        .row_count()
}

fn bench_query_engine_reuse(c: &mut Criterion) {
    let dataset = fixture_dataset();
    let reused = QueryEngine::new();
    assert_eq!(run_select(&reused, &dataset), 32);

    let mut group = c.benchmark_group("wasm_query_engine_reuse");
    group.bench_function("reused_engine_select", |bencher| {
        bencher.iter(|| black_box(run_select(black_box(&reused), black_box(&dataset))));
    });
    group.bench_function("fresh_engine_select", |bencher| {
        bencher.iter_batched(
            QueryEngine::new,
            |engine| black_box(run_select(&engine, black_box(&dataset))),
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

criterion_group!(benches, bench_query_engine_reuse);
criterion_main!(benches);
