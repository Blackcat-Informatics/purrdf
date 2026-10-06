// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Admission cost in isolation and in minimal repeated execution.

use purrdf_core::{RdfDatasetBuilder, SparqlEngine, SparqlRequest};
use purrdf_sparql_eval::{NativeSparqlEngine, QueryOptions};
use purrdf_testkit::bench::{Bench, bench_main};
use std::hint::black_box;

fn bench(c: &mut Bench) {
    let engine = NativeSparqlEngine::new();
    let data = RdfDatasetBuilder::new().freeze().unwrap();
    let query = "ASK { VALUES ?x { <http://example.org/value> } }";
    let prepared = engine.prepare_query(query, None).unwrap();
    let mut group = c.benchmark_group("prepared_admission");
    group.bench_function("validate_algebra", |b| {
        b.iter(|| black_box(prepared.query()).validate().unwrap());
    });
    group.bench_function("execute_prepared_minimal", |b| {
        b.iter(|| {
            black_box(
                engine
                    .query_prepared(black_box(&data), &prepared, &[], QueryOptions::EMPTY)
                    .unwrap(),
            )
        });
    });
    group.bench_function("execute_cached_text_minimal", |b| {
        b.iter(|| {
            black_box(
                engine
                    .query(
                        black_box(&data),
                        SparqlRequest {
                            query,
                            base_iri: None,
                            substitutions: &[],
                        },
                    )
                    .unwrap(),
            )
        });
    });
    group.finish();
}
/// Run admission benchmarks with the harness's command-line configuration.
pub fn benches() {
    let mut criterion = Bench::default().configure_from_args();
    bench(&mut criterion);
}
bench_main!(benches);
