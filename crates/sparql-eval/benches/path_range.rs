// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

#![allow(missing_docs)]

//! Large exact-range property-path regression benchmark.
//!
//! The start node fans into cycles of pairwise-coprime lengths. Their combined frontier
//! period is the product of those lengths, while the binary-power evaluator is bounded by
//! graph nodes and the 32 bits of the range exponent.

use std::sync::Arc;

use criterion::{Criterion, criterion_group, criterion_main};
use purrdf_core::{RdfDataset, RdfDatasetBuilder, SparqlEngine, SparqlRequest, SparqlResult};
use purrdf_sparql_eval::NativeSparqlEngine;

const EX: &str = "https://example.org/";
const CYCLE_LENGTHS: &[usize] = &[5, 7, 11, 13, 17];
const QUERY: &str = "SELECT ?o WHERE { \
    <https://example.org/a> <https://example.org/p>{4000000000} ?o \
}";

fn coprime_cycle_dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let start = builder.intern_iri(&format!("{EX}a"));
    let predicate = builder.intern_iri(&format!("{EX}p"));

    for (cycle, &length) in CYCLE_LENGTHS.iter().enumerate() {
        let nodes: Vec<_> = (0..length)
            .map(|index| builder.intern_iri(&format!("{EX}c{cycle}_{index}")))
            .collect();
        builder.push_quad(start, predicate, nodes[0], None);
        for index in 0..length {
            builder.push_quad(nodes[index], predicate, nodes[(index + 1) % length], None);
        }
    }

    builder.freeze().expect("freeze coprime-cycle dataset")
}

fn run(engine: &NativeSparqlEngine, dataset: &Arc<RdfDataset>) -> usize {
    let result = engine
        .query(
            dataset,
            SparqlRequest {
                query: QUERY,
                base_iri: None,
                substitutions: &[],
            },
        )
        .expect("large exact range evaluates");
    match result {
        SparqlResult::Solutions { rows, .. } => rows.len(),
        SparqlResult::Boolean(_) | SparqlResult::Graph(_) => 0,
    }
}

fn bench_path_range(c: &mut Criterion) {
    let dataset = coprime_cycle_dataset();
    let engine = NativeSparqlEngine::new();
    assert_eq!(run(&engine, &dataset), CYCLE_LENGTHS.len());

    c.bench_function("path_range/large_exact_coprime_cycles", |bencher| {
        bencher.iter(|| criterion::black_box(run(&engine, &dataset)));
    });
}

criterion_group!(benches, bench_path_range);
criterion_main!(benches);
