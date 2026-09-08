// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Typed CONSTRUCT publication versus an intermediate frozen result dataset.
#![allow(missing_docs)]

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use purrdf_core::ir::import::DatasetImporter;
use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlResult};
use purrdf_sparql_eval::{NativeSparqlEngine, PreparedQuery, QueryOptions};
use std::sync::Arc;

fn data(rows: usize) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri("https://example.org/p");
    for row in 0..rows {
        let s = b.intern_iri(&format!("https://example.org/s{row}"));
        let o = b.intern_literal(RdfLiteral::simple(row.to_string()));
        b.push_quad(s, p, o, None);
    }
    b.freeze().expect("data")
}

fn verify_publication(engine: &NativeSparqlEngine, plan: &PreparedQuery, data: &Arc<RdfDataset>) {
    let SparqlResult::Graph(expected) = engine
        .query_prepared(data, plan, &[], QueryOptions::EMPTY)
        .expect("ordinary")
    else {
        panic!("graph")
    };
    let mut target = RdfDatasetBuilder::new();
    let stats = engine
        .construct_prepared_into_view(data.as_ref(), plan, &[], QueryOptions::EMPTY, &mut target)
        .expect("typed");
    let actual = target.freeze().expect("publication");
    assert_eq!(
        expected.owned_quads().collect::<Vec<_>>(),
        actual.owned_quads().collect::<Vec<_>>()
    );
    assert_eq!(
        expected.owned_reifiers().collect::<Vec<_>>(),
        actual.owned_reifiers().collect::<Vec<_>>()
    );
    assert_eq!(
        expected.owned_annotations().collect::<Vec<_>>(),
        actual.owned_annotations().collect::<Vec<_>>()
    );
    assert_eq!(
        expected.owned_named_graphs().collect::<Vec<_>>(),
        actual.owned_named_graphs().collect::<Vec<_>>()
    );
    assert_eq!(stats.intermediate_freezes, 0);
    println!(
        "construct publication rows={} stats={stats:?}",
        data.quad_count()
    );
}

fn bench(c: &mut Criterion) {
    let engine = NativeSparqlEngine::new();
    let plan = engine.prepare_query("CONSTRUCT { ?s <https://example.org/output> ?o } WHERE { ?s <https://example.org/p> ?o }",None).expect("plan");
    let mut group = c.benchmark_group("construct_publication");
    for rows in [100, 10_000] {
        let data = data(rows);
        verify_publication(&engine, &plan, &data);
        group.bench_with_input(
            BenchmarkId::new("frozen_intermediate", rows),
            &data,
            |b, data| {
                b.iter(|| {
                    let SparqlResult::Graph(graph) = engine
                        .query_prepared(data, &plan, &[], QueryOptions::EMPTY)
                        .expect("query")
                    else {
                        panic!("graph")
                    };
                    let mut target = RdfDatasetBuilder::new();
                    DatasetImporter::new(&mut target, graph.as_ref()).append();
                    std::hint::black_box(target.freeze().expect("publication"));
                });
            },
        );
        group.bench_with_input(BenchmarkId::new("typed_staging", rows), &data, |b, data| {
            b.iter(|| {
                let mut target = RdfDatasetBuilder::new();
                let stats = engine
                    .construct_prepared_into_view(
                        data.as_ref(),
                        &plan,
                        &[],
                        QueryOptions::EMPTY,
                        &mut target,
                    )
                    .expect("construct");
                std::hint::black_box((target.freeze().expect("publication"), stats));
            });
        });
    }
    group.finish();
}
criterion_group!(benches, bench);
criterion_main!(benches);
