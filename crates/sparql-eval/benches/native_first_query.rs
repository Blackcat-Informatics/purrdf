// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! First-use source observation, warm execution and build plus first query.
//!
//! Reuses the evaluator's row-loop fixture: two default-graph literal rows per
//! subject, an integer modulo 100 and one shared three-element composite list.
//! `PURRDF_FIRST_QUERY_SUBJECTS` selects the subject count (100,000 by default).
//! This distribution differs from the core freeze benchmark's mixed objects.
//! Each cold sample owns a fresh dataset and engine. Query preparation is outside
//! timing; source identity, statistics and access preparation remain inside the
//! first prepared query. The combined lane includes fixture build and freeze.
//! Warm lanes reuse the same engine, dataset and prepared query after one call.

#[path = "../tests/support/mod.rs"]
mod support;

use purrdf_core::{RdfDataset, SparqlResult};
use purrdf_sparql_eval::{NativeSparqlEngine, PreparedQuery, QueryOptions};
use purrdf_testkit::bench::{BatchSize, Bench, BenchmarkId, bench_group, bench_main};
use std::time::Duration;
use support::governor_workloads::loop_dataset;

fn bench_first_query(bench: &mut Bench) {
    let subjects = std::env::var_os("PURRDF_FIRST_QUERY_SUBJECTS").map_or(100_000, |value| {
        value
            .into_string()
            .expect("subject count is UTF-8")
            .parse::<usize>()
            .expect("subject count is an integer")
    });
    assert!(subjects > 0, "the selected subject must exist");
    let prepare = |text: &str| {
        let engine = NativeSparqlEngine::new();
        let query = engine.prepare_query(text, None).expect("query prepares");
        (engine, query)
    };
    let execute = |engine: &NativeSparqlEngine, dataset: &RdfDataset, query: &PreparedQuery| {
        engine
            .query_prepared_view(dataset, query, &[], &QueryOptions::EMPTY)
            .expect("query completes")
    };
    let mut group = bench.benchmark_group("native_first_query");
    group
        .sample_size(10)
        .warm_up_time(Duration::from_millis(100))
        .measurement_time(Duration::from_secs(1));
    for (name, text) in [
        (
            "ask",
            "ASK WHERE { <https://example.org/s0> <https://example.org/v> 0 }",
        ),
        (
            "select",
            "SELECT ?v WHERE { <https://example.org/s0> <https://example.org/v> ?v }",
        ),
    ] {
        let dataset = loop_dataset(subjects);
        assert_eq!(
            dataset.quad_count(),
            subjects.checked_mul(2).expect("row count fits usize")
        );
        let (engine, query) = prepare(text);
        match execute(&engine, &dataset, &query) {
            SparqlResult::Boolean(answer) => assert!(answer),
            SparqlResult::Solutions { rows, .. } => assert_eq!(rows.len(), 1),
            SparqlResult::Graph(_) => panic!("the query does not construct a graph"),
        }
        group.bench_function(
            BenchmarkId::new(format!("warm_{name}"), subjects),
            |bencher| {
                bencher.iter(|| execute(&engine, &dataset, &query));
            },
        );
        group.bench_function(
            BenchmarkId::new(format!("cold_{name}"), subjects),
            |bencher| {
                bencher.iter_batched(
                    || {
                        let (engine, query) = prepare(text);
                        (engine, query, loop_dataset(subjects))
                    },
                    |(engine, query, dataset)| {
                        let result = execute(&engine, &dataset, &query);
                        (engine, query, dataset, result)
                    },
                    BatchSize::LargeInput,
                );
            },
        );
        group.bench_function(
            BenchmarkId::new(format!("build_freeze_first_{name}"), subjects),
            |bencher| {
                bencher.iter_batched(
                    || prepare(text),
                    |(engine, query)| {
                        let dataset = loop_dataset(subjects);
                        let result = execute(&engine, &dataset, &query);
                        (engine, query, dataset, result)
                    },
                    BatchSize::LargeInput,
                );
            },
        );
    }
    group.finish();
}

bench_group!(benches, bench_first_query);
bench_main!(benches);
