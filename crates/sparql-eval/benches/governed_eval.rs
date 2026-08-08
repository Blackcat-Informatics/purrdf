// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! Report-only execution-governor cost envelope.
//!
//! Four comparisons keep distinct costs distinct:
//!
//! - ordinary ungoverned evaluation, whose latency is the regression ceiling;
//! - the typed governed carrier under `UNBOUNDED`, which should take the same recursive
//!   fast path while constructing an outcome and empty evidence;
//! - a full `METERED` receipt through the production ordered parallel fold and through
//!   the measurement-only forced-sequential branch;
//! - exact property-path ranges at increasing graph sizes, with a fixed four-billion
//!   exponent, so growth follows the reachable relation rather than the numeric range.
//!
//! Timing is deliberately not a gate. Correctness and exact receipts are asserted before
//! Criterion samples; this target only reports the price of those guarantees.

use std::sync::Arc;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use purrdf_core::{RdfDataset, RdfDatasetBuilder, ResourceDimension, SparqlResult, TermValue};
use purrdf_sparql_eval::{
    EvalOptions, GovernedOutcome, NativeSparqlEngine, PreparedQuery, QueryGovernors, QueryOptions,
};

const EX: &str = "https://example.org/";
const JOIN_QUERY: &str = "SELECT ?s ?o ?z WHERE { \
    ?s <https://example.org/p> ?o . \
    ?s <https://example.org/q> ?z \
}";
const PATH_QUERY: &str = "SELECT ?o WHERE { \
    <https://example.org/n0> <https://example.org/p>{4000000000} ?o \
}";

fn join_dataset(rows: usize) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let p = builder.intern_iri(&format!("{EX}p"));
    let q = builder.intern_iri(&format!("{EX}q"));
    for index in 0..rows {
        let subject = builder.intern_iri(&format!("{EX}s{index}"));
        let object = builder.intern_iri(&format!("{EX}o{index}"));
        let joined = builder.intern_iri(&format!("{EX}z{index}"));
        builder.push_quad(subject, p, object, None);
        builder.push_quad(subject, q, joined, None);
    }
    builder.freeze().expect("freeze governed benchmark dataset")
}

fn ring_dataset(nodes: usize) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri(&format!("{EX}p"));
    let ids: Vec<_> = (0..nodes)
        .map(|index| builder.intern_iri(&format!("{EX}n{index}")))
        .collect();
    for index in 0..nodes {
        builder.push_quad(ids[index], predicate, ids[(index + 1) % nodes], None);
    }
    builder.freeze().expect("freeze path-scaling dataset")
}

fn result_rows(result: &SparqlResult) -> usize {
    match result {
        SparqlResult::Solutions { rows, .. } => rows.len(),
        SparqlResult::Graph(graph) => graph.quad_count(),
        SparqlResult::Boolean(value) => usize::from(*value),
    }
}

fn run_plain(
    engine: &NativeSparqlEngine,
    dataset: &Arc<RdfDataset>,
    prepared: &PreparedQuery,
) -> usize {
    let result = engine
        .query_prepared(dataset, prepared, &[], QueryOptions::EMPTY)
        .expect("benchmark query evaluates");
    result_rows(&result)
}

fn run_governed(
    engine: &NativeSparqlEngine,
    dataset: &Arc<RdfDataset>,
    prepared: &PreparedQuery,
    governors: &QueryGovernors,
) -> (usize, u64) {
    let outcome = engine
        .query_prepared_governed_view(
            &**dataset,
            prepared,
            &[] as &[(String, TermValue)],
            QueryOptions::EMPTY,
            governors,
        )
        .expect("governed benchmark query evaluates");
    let fuel = outcome.evidence().consumed_in(ResourceDimension::Fuel);
    let GovernedOutcome::Complete { result, .. } = outcome else {
        panic!("benchmark ceilings are unreachable");
    };
    (result_rows(&result), fuel)
}

fn bench_governed_query(c: &mut Criterion) {
    const ROWS: usize = 4096;

    let dataset = join_dataset(ROWS);
    let parallel = NativeSparqlEngine::new();
    let sequential = NativeSparqlEngine::new().with_eval_options(EvalOptions {
        force_sequential: true,
        ..EvalOptions::default()
    });
    let prepared = parallel
        .prepare_query(JOIN_QUERY, None)
        .expect("parse governed benchmark query");

    // Warm plan caches and prove that every measured lane runs the same real workload.
    assert_eq!(run_plain(&parallel, &dataset, &prepared), ROWS);
    assert_eq!(
        run_governed(&parallel, &dataset, &prepared, &QueryGovernors::UNBOUNDED,).0,
        ROWS
    );
    let parallel_receipt = run_governed(&parallel, &dataset, &prepared, &QueryGovernors::METERED);
    let sequential_receipt =
        run_governed(&sequential, &dataset, &prepared, &QueryGovernors::METERED);
    assert_eq!(parallel_receipt, sequential_receipt);
    assert!(parallel_receipt.1 > 0, "METERED must produce a receipt");

    let mut group = c.benchmark_group("governed_eval/query_4096");
    group.bench_function("ungoverned_baseline", |bencher| {
        bencher.iter(|| criterion::black_box(run_plain(&parallel, &dataset, &prepared)));
    });
    group.bench_function("unbounded_carrier", |bencher| {
        bencher.iter(|| {
            criterion::black_box(run_governed(
                &parallel,
                &dataset,
                &prepared,
                &QueryGovernors::UNBOUNDED,
            ));
        });
    });
    group.bench_function("metered_receipt_parallel", |bencher| {
        bencher.iter(|| {
            criterion::black_box(run_governed(
                &parallel,
                &dataset,
                &prepared,
                &QueryGovernors::METERED,
            ));
        });
    });
    group.bench_function("metered_receipt_sequential", |bencher| {
        bencher.iter(|| {
            criterion::black_box(run_governed(
                &sequential,
                &dataset,
                &prepared,
                &QueryGovernors::METERED,
            ));
        });
    });
    group.finish();
}

fn bench_path_scaling(c: &mut Criterion) {
    let engine = NativeSparqlEngine::new();
    let prepared = engine
        .prepare_query(PATH_QUERY, None)
        .expect("parse exact-range path benchmark query");
    let mut group = c.benchmark_group("governed_eval/path_exact_4e9");

    for nodes in [32_usize, 64, 128] {
        let dataset = ring_dataset(nodes);
        assert_eq!(run_plain(&engine, &dataset, &prepared), 1);
        group.bench_with_input(BenchmarkId::from_parameter(nodes), &nodes, |bencher, _| {
            bencher.iter(|| criterion::black_box(run_plain(&engine, &dataset, &prepared)));
        });
    }
    group.finish();
}

criterion_group!(benches, bench_governed_query, bench_path_scaling);
criterion_main!(benches);
