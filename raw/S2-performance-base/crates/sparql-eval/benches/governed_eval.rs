// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API, so the workspace `missing_docs` lint is
// not asked of their items.
#![allow(missing_docs)]

//! Report-only execution-governor cost envelope.
//!
//! The comparisons keep distinct costs distinct:
//!
//! - ordinary ungoverned evaluation, whose latency is the regression ceiling;
//! - the typed governed carrier under `UNBOUNDED`, which should take the same recursive
//!   fast path while constructing an outcome and empty evidence;
//! - a full `METERED` receipt through the production ordered parallel fold and through
//!   the measurement-only forced-sequential branch;
//! - exact property-path ranges at increasing graph sizes, with a fixed four-billion
//!   exponent, so growth follows the reachable relation rather than the numeric range;
//! - linear paths connecting typed endpoints versus explicit triple expansion, with
//!   preparation outside timing and a cold join-order cache in every sample;
//! - equivalent UNION positions, opposing connector/attribute selectivity, and
//!   preparation growth for compact sequences of alternatives;
//! - the per-row loops of `FILTER`, `BIND` and `UNFOLD` over 16 384 rows, each run
//!   ungoverned, under a stop signal alone, and under a fuel ceiling with a stop
//!   signal, on the forced-sequential engine and on the default one that forks a
//!   parallel-safe row loop. The stop signal is a never-cancelled
//!   [`CancellationFlag`], so every row pays the poll and none is refused; the fuel
//!   ceiling is one below the largest representable, so it is engaged and charged but
//!   never reached. `UNFOLD`'s row loop is never forked, so its two engine lanes
//!   differ only below it.
//!
//! Timing is deliberately not a gate. Correctness and exact receipts are asserted before
//! the harness samples; this target only reports the price of those guarantees.

#[path = "../tests/support/mod.rs"]
mod support;

use support::result_size;

use std::{fmt::Write, sync::Arc};

use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, RdfLiteral, ResourceDimension, SparqlResult, TermValue,
};
use purrdf_iri::vocab::rdf::TYPE;
use purrdf_sparql_eval::{
    CancellationFlag, EvalOptions, GovernedOutcome, NativeSparqlEngine, PreparedQuery,
    QueryGovernors, QueryOptions,
};
use purrdf_testkit::bench::{Bench, BenchmarkId, bench_group, bench_main};

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

fn run_plain(
    engine: &NativeSparqlEngine,
    dataset: &Arc<RdfDataset>,
    prepared: &PreparedQuery,
) -> usize {
    let result = engine
        .query_prepared(dataset, prepared, &[], QueryOptions::EMPTY)
        .expect("benchmark query evaluates");
    result_size(&result)
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
    (result_size(&result), fuel)
}

fn bench_governed_query(c: &mut Bench) {
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
        bencher.iter(|| std::hint::black_box(run_plain(&parallel, &dataset, &prepared)));
    });
    group.bench_function("unbounded_carrier", |bencher| {
        bencher.iter(|| {
            std::hint::black_box(run_governed(
                &parallel,
                &dataset,
                &prepared,
                &QueryGovernors::UNBOUNDED,
            ));
        });
    });
    group.bench_function("metered_receipt_parallel", |bencher| {
        bencher.iter(|| {
            std::hint::black_box(run_governed(
                &parallel,
                &dataset,
                &prepared,
                &QueryGovernors::METERED,
            ));
        });
    });
    group.bench_function("metered_receipt_sequential", |bencher| {
        bencher.iter(|| {
            std::hint::black_box(run_governed(
                &sequential,
                &dataset,
                &prepared,
                &QueryGovernors::METERED,
            ));
        });
    });
    group.finish();
}

/// Fixed-length paths connecting otherwise independent typed endpoints. Queries are
/// prepared outside timing; a fresh engine keeps every sample's join-order cache cold.
fn bench_linear_paths(c: &mut Bench) {
    let queries = [
        (
            "path",
            "?x <https://example.org/p>/<https://example.org/q> ?y",
        ),
        (
            "expanded",
            "?x <https://example.org/p> ?mid . ?mid <https://example.org/q> ?y",
        ),
    ]
    .map(|(label, path)| {
        let query = format!(
            "SELECT ?x ?y ?tx ?ty WHERE {{ {path} . ?x a ?tx . ?y a ?ty }} ORDER BY ?x ?y ?tx ?ty"
        );
        (
            label,
            NativeSparqlEngine::new()
                .prepare_query(&query, None)
                .expect("prepare linear-path benchmark"),
        )
    });
    let mut group = c.benchmark_group("governed_eval/linear_paths");
    for chains in [16_usize, 64, 256] {
        let mut builder = RdfDatasetBuilder::new();
        let p = builder.intern_iri(&format!("{EX}p"));
        let q = builder.intern_iri(&format!("{EX}q"));
        let rdf_type = builder.intern_iri(TYPE);
        let kind = builder.intern_iri(&format!("{EX}Kind"));
        for index in 0..chains {
            let subject = builder.intern_iri(&format!("{EX}s{index}"));
            let middle = builder.intern_iri(&format!("{EX}m{index}"));
            let object = builder.intern_iri(&format!("{EX}o{index}"));
            builder.push_quad(subject, p, middle, None);
            builder.push_quad(middle, q, object, None);
            builder.push_quad(subject, rdf_type, kind, None);
            builder.push_quad(object, rdf_type, kind, None);
        }
        let dataset = builder.freeze().expect("freeze linear-path dataset");
        let results = queries.each_ref().map(|(_, prepared)| {
            NativeSparqlEngine::new()
                .query_prepared(&dataset, prepared, &[], QueryOptions::EMPTY)
                .expect("linear-path answer")
        });
        let answers = results.each_ref().map(|result| match result {
            SparqlResult::Solutions {
                variables, rows, ..
            } => (variables, rows),
            other => panic!("expected solutions, got {other:?}"),
        });
        assert_eq!(answers[0], answers[1], "path and expansion agree");
        assert_eq!(result_size(&results[0]), chains);
        for (label, prepared) in &queries {
            group.bench_with_input(BenchmarkId::new(*label, chains), &chains, |bencher, _| {
                bencher.iter(|| {
                    std::hint::black_box(run_plain(&NativeSparqlEngine::new(), &dataset, prepared))
                });
            });
        }
    }
    group.finish();
}

fn bench_path_scaling(c: &mut Bench) {
    let engine = NativeSparqlEngine::new();
    let prepared = engine
        .prepare_query(PATH_QUERY, None)
        .expect("parse exact-range path benchmark query");
    let mut group = c.benchmark_group("governed_eval/path_exact_4e9");

    for nodes in [32_usize, 64, 128] {
        let dataset = ring_dataset(nodes);
        assert_eq!(run_plain(&engine, &dataset, &prepared), 1);
        group.bench_with_input(BenchmarkId::from_parameter(nodes), &nodes, |bencher, _| {
            bencher.iter(|| std::hint::black_box(run_plain(&engine, &dataset, &prepared)));
        });
    }
    group.finish();
}

/// Rows in the row-loop dataset: past the evaluator's parallel threshold, so the
/// default engine forks a parallel-safe `FILTER` or `BIND` loop.
const LOOP_ROWS: usize = 16_384;

const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
const CDT_LIST: &str = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/List";

/// Per subject `i`: `ex:v i % 100` (an `xsd:integer`) and `ex:list "[1,2,3]"`, a
/// three-element composite list.
fn loop_dataset(rows: usize) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let v = builder.intern_iri(&format!("{EX}v"));
    let list = builder.intern_iri(&format!("{EX}list"));
    let three = builder.intern_literal(RdfLiteral::typed("[1,2,3]", CDT_LIST));
    for index in 0..rows {
        let subject = builder.intern_iri(&format!("{EX}s{index}"));
        let value =
            builder.intern_literal(RdfLiteral::typed((index % 100).to_string(), XSD_INTEGER));
        builder.push_quad(subject, v, value, None);
        builder.push_quad(subject, list, three, None);
    }
    builder.freeze().expect("freeze the row-loop dataset")
}

/// `(bench id, query text, exact expected rows)` for the three row loops.
const LOOP_QUERIES: &[(&str, &str, usize)] = &[
    (
        "filter",
        "SELECT ?s WHERE { ?s <https://example.org/v> ?v FILTER(?v >= 10) }",
        // `?v < 10` on ten subjects of every hundred, and on ten of the last 84.
        LOOP_ROWS - 10 * LOOP_ROWS.div_ceil(100),
    ),
    (
        "bind",
        "SELECT ?s ?x WHERE { ?s <https://example.org/v> ?v BIND(?v * 2 + 1 AS ?x) }",
        LOOP_ROWS,
    ),
    (
        "unfold",
        "SELECT ?s ?e WHERE { ?s <https://example.org/list> ?l UNFOLD(?l AS ?e) }",
        3 * LOOP_ROWS,
    ),
];

/// The three governor configurations, freshly built so each carries its own stop flag.
fn loop_governors() -> [(&'static str, Option<QueryGovernors>); 3] {
    let stop = || Arc::new(CancellationFlag::new()) as Arc<dyn purrdf_sparql_eval::StopSignal>;
    [
        ("ungoverned", None),
        (
            "stop_signal",
            Some(QueryGovernors::UNBOUNDED.with_stop_signal(stop())),
        ),
        (
            "fuel_and_stop_signal",
            Some(
                QueryGovernors::UNBOUNDED
                    .with_fuel(u64::MAX - 1)
                    .with_stop_signal(stop()),
            ),
        ),
    ]
}

fn run_loop(
    engine: &NativeSparqlEngine,
    dataset: &Arc<RdfDataset>,
    prepared: &PreparedQuery,
    governors: Option<&QueryGovernors>,
) -> usize {
    governors.map_or_else(
        || run_plain(engine, dataset, prepared),
        |governors| run_governed(engine, dataset, prepared, governors).0,
    )
}

fn bench_governed_row_loops(c: &mut Bench) {
    let dataset = loop_dataset(LOOP_ROWS);
    let parallel = NativeSparqlEngine::new();
    let sequential = NativeSparqlEngine::new().with_eval_options(EvalOptions {
        force_sequential: true,
        ..EvalOptions::default()
    });
    let engines = [("sequential", &sequential), ("parallel", &parallel)];
    let governors = loop_governors();

    let mut group = c.benchmark_group("governed_eval/row_loops_16384");
    group.sample_size(10);
    for &(shape, query, expected) in LOOP_QUERIES {
        let prepared = parallel
            .prepare_query(query, None)
            .expect("parse row-loop benchmark query");
        for (lane, engine) in engines {
            for (governed, governor) in &governors {
                // Every lane answers the same rows before any is timed: a governor that
                // refused rows would otherwise read as a speedup.
                assert_eq!(
                    run_loop(engine, &dataset, &prepared, governor.as_ref()),
                    expected,
                    "{shape} on the {lane} engine, {governed}"
                );
                group.bench_function(format!("{shape}/{lane}/{governed}"), |bencher| {
                    bencher.iter(|| {
                        std::hint::black_box(run_loop(
                            engine,
                            &dataset,
                            &prepared,
                            governor.as_ref(),
                        ))
                    });
                });
            }
        }
    }
    group.finish();
}

/// The same connected bag with its terminal predicate on either side of
/// a UNION. Preparation is outside timing and both positions must answer equally.
fn bench_union_positions(c: &mut Bench) {
    let engine = NativeSparqlEngine::new();
    let mut group = c.benchmark_group("governed_eval/union_positions");
    for count in [64_usize, 256] {
        let mut builder = RdfDatasetBuilder::new();
        let node = builder.intern_iri(&format!("{EX}node"));
        let controls_a = builder.intern_iri(&format!("{EX}controlsA"));
        let controls_b = builder.intern_iri(&format!("{EX}controlsB"));
        let target = builder.intern_iri(&format!("{EX}target"));
        let connected = builder.intern_iri(&format!("{EX}connected"));
        let yes = builder.intern_iri(&format!("{EX}yes"));
        for index in 0..count {
            let terminal = builder.intern_iri(&format!("{EX}terminal{index}"));
            let control = builder.intern_iri(&format!("{EX}control{index}"));
            let subject = builder.intern_iri(&format!("{EX}s{index}"));
            let object = builder.intern_iri(&format!("{EX}target{index}"));
            builder.push_quad(terminal, node, subject, None);
            builder.push_quad(
                control,
                if index % 2 == 0 {
                    controls_a
                } else {
                    controls_b
                },
                terminal,
                None,
            );
            builder.push_quad(control, target, object, None);
            builder.push_quad(terminal, connected, yes, None);
        }
        let dataset = builder.freeze().expect("UNION position benchmark dataset");
        for (label, before, after) in [
            ("before", "?terminal ex:connected ex:yes .", ""),
            ("after", "", "?terminal ex:connected ex:yes ."),
        ] {
            let text = format!(
                "PREFIX ex: <{EX}> SELECT ?s ?object WHERE {{ ?terminal ex:node ?s . {before} \
                 {{ ?control ex:controlsA ?terminal }} UNION {{ ?control ex:controlsB ?terminal }} \
                 ?control ex:target ?object . {after} }}"
            );
            let prepared = engine
                .prepare_query(&text, None)
                .expect("UNION position benchmark query");
            assert_eq!(run_plain(&engine, &dataset, &prepared), count);
            assert_eq!(
                run_governed(&engine, &dataset, &prepared, &QueryGovernors::METERED).0,
                count
            );
            group.bench_function(BenchmarkId::new(label, count), |bencher| {
                bencher.iter(|| {
                    std::hint::black_box(run_governed(
                        &engine,
                        &dataset,
                        &prepared,
                        &QueryGovernors::METERED,
                    ))
                });
            });
        }
    }
    group.finish();
}

/// Opposing selectivity: the tiny connector and tiny attribute sides each drive
/// the same two answers under the same live allocation ceiling.
fn bench_union_selectivity(c: &mut Bench) {
    let mut group = c.benchmark_group("governed_eval/union_selectivity");
    for (label, tiny_attributes) in [("connector", false), ("attributes", true)] {
        let mut b = RdfDatasetBuilder::new();
        let left_p = b.intern_iri(&format!("{EX}left"));
        let right_p = b.intern_iri(&format!("{EX}right"));
        let a = b.intern_iri(&format!("{EX}a"));
        let bb = b.intern_iri(&format!("{EX}b"));
        for i in 0..4096 {
            let left = b.intern_iri(&format!("{EX}left{i}"));
            let right = b.intern_iri(&format!("{EX}right{i}"));
            let value = b.intern_iri(&format!("{EX}value{i}"));
            if !tiny_attributes || i < 2 {
                b.push_quad(left, left_p, value, None);
                b.push_quad(right, right_p, value, None);
            }
            if tiny_attributes || i < 2 {
                b.push_quad(left, if i % 2 == 0 { a } else { bb }, right, None);
            }
        }
        let dataset = b.freeze().expect("selectivity fixture");
        let engine = NativeSparqlEngine::new();
        let governors = QueryGovernors::METERED.with_max_intermediate_cells(4096);
        for (form, body) in [
            (
                "shared",
                "?left ex:left ?lv . { ?left ex:a ?right } UNION { ?left ex:b ?right } ?right ex:right ?rv",
            ),
            (
                "distributed",
                "{ ?left ex:a ?right . ?left ex:left ?lv . ?right ex:right ?rv } UNION { ?left ex:b ?right . ?left ex:left ?lv . ?right ex:right ?rv }",
            ),
        ] {
            let text = format!("PREFIX ex: <{EX}> SELECT ?left ?right ?lv ?rv WHERE {{ {body} }}");
            let prepared = engine.prepare_query(&text, None).expect("selective query");
            assert_eq!(run_governed(&engine, &dataset, &prepared, &governors).0, 2);
            group.bench_function(format!("{label}/{form}"), |bencher| {
                bencher.iter(|| {
                    std::hint::black_box(run_governed(&engine, &dataset, &prepared, &governors))
                });
            });
        }
    }
    group.finish();
}

/// Preparation growth over a compact sequence of two-arm connectors. Each
/// original triple remains exactly once after normalization.
fn bench_union_preparation(c: &mut Bench) {
    let mut group = c.benchmark_group("governed_eval/union_preparation");
    for count in [16_usize, 64, 256] {
        let mut body = String::new();
        for i in 0..count {
            write!(
                body,
                "?v{i} ex:attr ?a{i} . {{ ?v{i} ex:a ?v{} }} UNION {{ ?v{i} ex:b ?v{} }} ",
                i + 1,
                i + 1,
            )
            .expect("String writes are infallible");
        }
        let text = format!("PREFIX ex: <{EX}> SELECT ?v0 ?v{count} WHERE {{ {body} }}");
        let source = purrdf_sparql_algebra::SparqlParser::new()
            .parse_query(&text)
            .expect("preparation source");
        let engine = NativeSparqlEngine::new();
        let prepared = engine
            .prepare_algebra(source.clone(), QueryOptions::EMPTY)
            .expect("preparation plan");
        eprintln!(
            "union preparation count={count} retained_bytes={}",
            prepared.retained_size_bytes()
        );
        group.bench_function(BenchmarkId::from_parameter(count), |bencher| {
            bencher.iter(|| {
                std::hint::black_box(
                    engine
                        .prepare_algebra(source.clone(), QueryOptions::EMPTY)
                        .expect("prepared"),
                )
            });
        });
    }
    group.finish();
}

bench_group!(
    benches,
    bench_governed_query,
    bench_path_scaling,
    bench_linear_paths,
    bench_governed_row_loops,
    bench_union_positions,
    bench_union_selectivity,
    bench_union_preparation
);
bench_main!(benches);
