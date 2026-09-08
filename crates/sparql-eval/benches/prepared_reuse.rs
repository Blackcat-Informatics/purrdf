// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Isolate cold preparation, cache hits and prepared execution. Uses existing
//! public APIs so the identical benchmark can measure the base revision.

use criterion::{Criterion, criterion_main};
use purrdf_core::ir::pack::dataset_from_view;
use purrdf_core::{
    BlankScope, RdfDatasetBuilder, RdfLiteral, SparqlEngine, SparqlRequest, canonical_relabel,
};
use purrdf_sparql_eval::{NativeSparqlEngine, PlanCache, QueryOptions};
use std::hint::black_box;

const QUERY: &str = "SELECT ?s ?value WHERE { ?s <http://example.org/p> ?value } ORDER BY ?s";

fn bench(c: &mut Criterion) {
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri("http://example.org/p");
    let value = builder.intern_iri("http://example.org/value");
    for index in 0..64 {
        let subject = builder.intern_iri(&format!("http://example.org/s{index}"));
        builder.push_quad(subject, predicate, value, None);
    }
    let data = builder.freeze().expect("valid benchmark dataset");
    let engine = NativeSparqlEngine::new();
    let prepared = engine.prepare_query(QUERY, None).expect("prepare");
    let mut cache = PlanCache::new();
    cache.prepare(QUERY, None).expect("warm cache");
    let mut group = c.benchmark_group("prepared_reuse");
    group.bench_function("prepare_cold", |b| {
        b.iter(|| {
            black_box(
                PlanCache::new()
                    .prepare(black_box(QUERY), None)
                    .expect("prepare"),
            )
        });
    });
    group.bench_function("prepare_warm", |b| {
        b.iter(|| black_box(cache.prepare(black_box(QUERY), None).expect("prepare")));
    });
    group.bench_function("execute_prepared", |b| {
        b.iter(|| {
            black_box(
                engine
                    .query_prepared(black_box(&data), &prepared, &[], QueryOptions::EMPTY)
                    .expect("execute"),
            )
        });
    });
    group.bench_function("execute_warm_text", |b| {
        b.iter(|| {
            black_box(
                engine
                    .query(
                        black_box(&data),
                        SparqlRequest {
                            query: QUERY,
                            base_iri: None,
                            substitutions: &[],
                        },
                    )
                    .expect("execute"),
            )
        });
    });
    group.bench_function("execute_cold_text", |b| {
        b.iter(|| {
            black_box(
                NativeSparqlEngine::new()
                    .query(
                        black_box(&data),
                        SparqlRequest {
                            query: QUERY,
                            base_iri: None,
                            substitutions: &[],
                        },
                    )
                    .expect("execute"),
            )
        });
    });
    group.finish();
}

fn canonical_reuse(c: &mut Criterion) {
    let mut group = c.benchmark_group("canonical_relabel");
    for (name, blanks) in [
        ("ground_literal_payload", false),
        ("blank_literal_payload", true),
    ] {
        let mut builder = RdfDatasetBuilder::new();
        let predicate = builder.intern_iri("http://example.org/payload");
        for index in 0..256 {
            let subject = if blanks {
                builder.intern_blank(&format!("b{index}"), BlankScope::DEFAULT)
            } else {
                builder.intern_iri(&format!("http://example.org/s{index}"))
            };
            let object = builder.intern_literal(RdfLiteral::simple(format!(
                "{index}:{}",
                "directional correspondence provenance ".repeat(64)
            )));
            builder.push_quad(subject, predicate, object, None);
        }
        let data = builder.freeze().expect("valid canonical benchmark dataset");
        group.bench_function(name, |b| {
            b.iter(|| black_box(canonical_relabel(black_box(&data)).expect("canonical relabel")));
        });
    }
    group.finish();
}

fn typed_materialization(c: &mut Criterion) {
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri("http://example.org/p");
    let literal = builder.intern_literal(RdfLiteral::simple("payload".repeat(512)));
    let shared = builder.intern_iri("http://example.org/shared");
    let triple = builder.intern_triple(shared, predicate, literal);
    for index in 0..4096 {
        let subject = builder.intern_iri(&format!("http://example.org/s{index}"));
        builder.push_quad(subject, predicate, triple, None);
    }
    let data = builder.freeze().expect("valid materialization dataset");
    c.bench_function("dataset_from_view/repeated_quoted_payload", |b| {
        b.iter(|| black_box(dataset_from_view(black_box(&data)).expect("materialize")));
    });
}

/// Run the prepared-query comparison group with Criterion CLI configuration.
pub fn benches() {
    let mut criterion = Criterion::default().configure_from_args();
    bench(&mut criterion);
    canonical_reuse(&mut criterion);
    typed_materialization(&mut criterion);
}
criterion_main!(benches);
