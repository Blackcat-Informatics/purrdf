// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API, so the workspace `missing_docs` lint is
// not asked of their items.
#![allow(missing_docs)]

//! Report-only benchmark for the npm wasm SPARQL wrapper.
//!
//! The native evaluator already has broad coverage in `purrdf-sparql-eval`.
//! This harness measures the binding-level overhead that TypeScript users hit:
//! repeated SELECT calls through one reused `QueryEngine` instance, which keeps
//! the native plan cache alive, versus constructing a fresh wrapper per call.

use std::fmt::Write as _;

use purrdf_testkit::bench::{BatchSize, Bench, bench_group, bench_main, black_box};

use purrdf::{RdfTerm, RdfTriple};
use purrdf_wasm::{DataFactory, Dataset, QueryEngine, Term};

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
        .select(dataset, SELECT_BY_OBJECT, None, None)
        .expect("SELECT succeeds")
        .row_count()
}

fn bench_query_engine_reuse(c: &mut Bench) {
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

/// The owned-model accessor reproduces suffix cloning for a bounded reference chain.
fn walk_owned(term: &RdfTerm) -> usize {
    let mut at = term.clone();
    let mut depth = 0;
    while let RdfTerm::Triple(triple) = &at {
        at = triple.object.clone();
        depth += 1;
    }
    black_box(at);
    depth
}

fn walk_view(term: &Term) -> usize {
    let mut at = term.clone();
    let mut depth = 0;
    while let Some(next) = at.object() {
        at = next;
        depth += 1;
    }
    black_box(at);
    depth
}

fn bench_quoted_term_access(c: &mut Bench) {
    let factory = DataFactory::new();
    let iri = factory.named_node("https://example.org/p".to_owned());
    let mut group = c.benchmark_group("wasm_quoted_term_access");
    for depth in [64, 256, 1024] {
        let mut view = iri.clone();
        let mut owned = RdfTerm::iri("https://example.org/p");
        for _ in 0..depth {
            view = factory
                .quoted_triple(&iri, &iri, &view)
                .expect("IRI predicate");
            owned = RdfTerm::triple(RdfTriple::new(
                RdfTerm::iri("https://example.org/p"),
                "https://example.org/p",
                owned,
            ));
        }
        assert_eq!(walk_view(&view), depth);
        assert_eq!(walk_owned(&owned), depth);
        group.bench_function(format!("shared_view/{depth}"), |bencher| {
            bencher.iter(|| black_box(walk_view(black_box(&view))));
        });
        group.bench_function(format!("owned_suffix_reference/{depth}"), |bencher| {
            bencher.iter(|| black_box(walk_owned(black_box(&owned))));
        });
    }
    group.finish();
}

bench_group!(benches, bench_query_engine_reuse, bench_quoted_term_access);
bench_main!(benches);
