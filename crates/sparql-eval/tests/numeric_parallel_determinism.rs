// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A governed query whose rows run on the arbitrary-precision tower stops at the same
//! point however its row loops are forked.
//!
//! The governor's contract for a forked row loop (`crate::row_checkpoint`) is that the
//! workers' charges are committed in source order after the join, so the trip, the
//! consumption and the certified answer are a function of the query and the data, not
//! of the thread count or the schedule. An exact-number operation is charged by its own
//! size, from inside the expression, so it is charged in the worker that evaluates the
//! row — and must reach the governor in the same order.
//!
//! Every forked loop that evaluates an expression is driven here over a column of
//! values past the machine words, at fuel and scratch ceilings that trip partway
//! through: `FILTER`, a projection (`BIND`), the per-group `SUM`/`AVG` fold, the
//! statistical aggregates, and an `OPTIONAL` filter. Each runs repeatedly under
//! rayon pools of 1, 2, 4, 8 and 32 threads, and every run must report the same trip,
//! the same consumption in every dimension, the same F&O error counts and the same
//! answer.

use std::sync::Arc;

use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, RdfLiteral, ResourceDimension, SparqlRequest, SparqlResult,
};
use purrdf_sparql_eval::{
    AggregateRegistry, ExtensionEnv, GovernedOutcome, NativeSparqlEngine, PartialAnswers,
    QueryGovernors, QueryOptions,
};

const EX: &str = "http://example.org/";
const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
const STAT: &str = "http://example.org/agg/";

/// Rows, and groups: both past the 1,024-item fork threshold, so every loop below
/// really splits into chunks.
const ROWS: usize = 1_500;

/// The stable callback sits in the OPTIONAL predicate itself. Its worker mask proves
/// real production predicate execution across workers rather than inferring a fork
/// from an answer or from the outer query running inside a pool.
#[test]
#[cfg(not(target_arch = "wasm32"))]
fn bookkeeping_optional_filters_fork_and_reachable_cell_bounds_stay_sequential() {
    use purrdf_sparql_eval::{Arity, UserFunctionRegistry, Volatility};
    use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

    let dataset = dataset();
    let mask = Arc::new(AtomicU64::new(0));
    let visits = Arc::new(AtomicUsize::new(0));
    let mut registry = UserFunctionRegistry::default();
    let seen = Arc::clone(&mask);
    let counted = Arc::clone(&visits);
    registry.register_native(
        format!("{EX}fork_tick"),
        Arity::Exact(1),
        Volatility::Stable,
        Arc::new(move |_: &[&purrdf_core::TermValue]| {
            let worker = rayon::current_thread_index().expect("query executes in its pool");
            seen.fetch_or(1 << worker, Ordering::Relaxed);
            counted.fetch_add(1, Ordering::Relaxed);
            std::thread::yield_now();
            Ok(Some(purrdf_core::TermValue::boolean(true)))
        }),
    );
    let engine = NativeSparqlEngine::new();
    let functions = engine
        .bind_functions(registry, ExtensionEnv::empty())
        .expect("stable predicate function");
    let options = QueryOptions::new().with_functions(&functions);
    let query = format!(
        "SELECT ?s ?v WHERE {{ ?s <{EX}v> ?v OPTIONAL {{ \
         {{ SELECT ?s ?w WHERE {{ ?s <{EX}w> ?w }} }} FILTER(<{EX}fork_tick>(?w)) }} }}"
    );
    let cells = (ROWS * 3) as u64;
    let mut expected: Option<Observation> = None;
    for threads in [1, 2, 4, 8, 32] {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .expect("predicate pool");
        for (label, governors, forks) in [
            ("plain", QueryGovernors::UNBOUNDED, true),
            ("metered", QueryGovernors::METERED, true),
            (
                "CLI fuel",
                QueryGovernors::METERED.with_fuel(1_000_000_000),
                true,
            ),
            (
                "inclusive finite cells",
                QueryGovernors::METERED.with_max_intermediate_cells(cells),
                false,
            ),
        ] {
            mask.store(0, Ordering::Relaxed);
            visits.store(0, Ordering::Relaxed);
            let observed = pool.install(|| observe(&dataset, &query, options, &governors));
            let workers = mask.load(Ordering::Relaxed).count_ones();
            assert_eq!(visits.load(Ordering::Relaxed), ROWS, "{label}/{threads}");
            assert!(
                observed.tripped.is_none(),
                "{label}/{threads}: {observed:?}"
            );
            if forks && threads > 1 {
                assert!(
                    workers > 1,
                    "{label}/{threads}: OPTIONAL predicate did not fork"
                );
            } else {
                assert_eq!(
                    workers, 1,
                    "{label}/{threads}: reachable cap must stay ordered"
                );
            }
            eprintln!(
                "OPTIONAL fork witness: {label}/{threads}, predicate workers={workers}, visits={ROWS}"
            );
            if label == "metered" && expected.is_none() {
                expected = Some(observed.clone());
            }
            if let Some(expected) = &expected {
                assert_eq!(observed.answer, expected.answer, "{label}/{threads}");
                if label != "plain" {
                    assert_eq!(
                        &observed, expected,
                        "all counters/errors: {label}/{threads}"
                    );
                    assert_eq!(
                        observed
                            .consumed
                            .iter()
                            .find(|(name, _)| name == "intermediate-cells")
                            .map(|(_, value)| *value),
                        Some(cells)
                    );
                }
            }
        }
        let refused = pool.install(|| {
            observe(
                &dataset,
                &query,
                options,
                &QueryGovernors::METERED.with_max_intermediate_cells(cells - 1),
            )
        });
        assert!(
            refused
                .tripped
                .as_ref()
                .is_some_and(|trip| trip.contains("IntermediateCells")),
            "finite neighbour must trip: {refused:?}"
        );
    }
}

/// `ex:s{i} ex:v n_i` with `n_i` a forty-digit integer, `ex:s{i} ex:d d_i` a decimal
/// with forty fractional digits, `ex:s{i} ex:w w_i` an integer whose length grows with
/// `i` (from 40 digits to 340), and `ex:s{i} ex:g g{i}` — one group per subject.
fn dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let v = builder.intern_iri(&format!("{EX}v"));
    let d = builder.intern_iri(&format!("{EX}d"));
    let g = builder.intern_iri(&format!("{EX}g"));
    let w = builder.intern_iri(&format!("{EX}w"));
    for index in 0..ROWS {
        let s = builder.intern_iri(&format!("{EX}s{index}"));
        let big = builder.intern_literal(RdfLiteral::typed(
            format!("1{index:039}"),
            format!("{XSD}integer"),
        ));
        let fine = builder.intern_literal(RdfLiteral::typed(
            format!("0.{index:039}1"),
            format!("{XSD}decimal"),
        ));
        let growing = builder.intern_literal(RdfLiteral::typed(
            format!("1{}", "7".repeat(39 + index / 5)),
            format!("{XSD}integer"),
        ));
        builder.push_quad(s, w, growing, None);
        let group = builder.intern_iri(&format!("{EX}g{index}"));
        builder.push_quad(s, v, big, None);
        builder.push_quad(s, d, fine, None);
        builder.push_quad(s, g, group, None);
    }
    builder.freeze().expect("a valid fixture")
}

/// What one governed run reported.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Observation {
    tripped: Option<String>,
    consumed: Vec<(String, u64)>,
    expression_errors: Vec<(String, u64)>,
    answer: String,
}

fn rows(result: &SparqlResult) -> String {
    match result {
        SparqlResult::Solutions { rows, .. } => format!("{} rows {rows:?}", rows.len()),
        other => format!("{other:?}"),
    }
}

fn observe(
    dataset: &Arc<RdfDataset>,
    query: &str,
    options: QueryOptions<'_>,
    governors: &QueryGovernors,
) -> Observation {
    let outcome = NativeSparqlEngine::new()
        .query_governed(
            dataset,
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &[],
            },
            options,
            governors,
        )
        .unwrap_or_else(|e| panic!("{query}: {e}"));
    let answer = match &outcome {
        GovernedOutcome::Complete { result, .. } => format!("complete {}", rows(result)),
        GovernedOutcome::BudgetExhausted(exhausted) => match &exhausted.partial {
            PartialAnswers::Certain(partial) => format!("certain {}", rows(partial.result())),
            PartialAnswers::AtMost(partial) => format!("at-most {}", rows(partial.result())),
            PartialAnswers::Unknown(barrier) => format!("withheld at {barrier}"),
        },
    };
    let evidence = outcome.evidence();
    Observation {
        tripped: outcome.tripped().map(|governor| format!("{governor:?}")),
        consumed: ResourceDimension::ALL
            .into_iter()
            .map(|dimension| {
                (
                    dimension.label().to_owned(),
                    evidence.consumed_in(dimension),
                )
            })
            .collect(),
        expression_errors: evidence
            .expression_errors()
            .iter()
            .map(|(code, count)| (code.qname().to_owned(), *count))
            .collect(),
        answer,
    }
}

/// Run `query` metered, then at half its metered fuel and at half its metered scratch,
/// on pools of every size, `REPEATS` times each: every run of one ceiling must report
/// the same observation, and the halved fuel ceiling must really trip.
fn assert_deterministic(query: &str, options: QueryOptions<'_>) {
    const REPEATS: usize = 6;
    let dataset = dataset();
    let metered = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .expect("a pool")
        .install(|| observe(&dataset, query, options, &QueryGovernors::METERED));
    assert!(metered.tripped.is_none(), "{query}: {metered:?}");
    let spent = |label: &str| {
        metered
            .consumed
            .iter()
            .find(|(name, _)| name == label)
            .map_or(0, |(_, value)| *value)
    };
    let ceilings = [
        ("metered", QueryGovernors::METERED),
        (
            "fuel",
            QueryGovernors::UNBOUNDED.with_fuel(spent("fuel") / 2),
        ),
        (
            "scratch-bytes",
            QueryGovernors::UNBOUNDED.with_max_scratch_bytes(spent("scratch-bytes") / 2),
        ),
        // The command line's `--fuel`: a fuel ceiling over a metered base, so scratch is
        // counted, and reported at the trip, as well.
        (
            "metered fuel",
            QueryGovernors::METERED.with_fuel(spent("fuel") * 5 / 7),
        ),
    ];
    for (label, governors) in &ceilings {
        let mut seen: Option<(usize, Observation)> = None;
        for threads in [1_usize, 2, 4, 8, 32] {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .expect("a pool");
            for _ in 0..REPEATS {
                let observed = pool.install(|| observe(&dataset, query, options, governors));
                assert!(
                    observed.tripped.is_some() || !label.ends_with("fuel"),
                    "{query}: half the metered fuel must trip: {observed:?}"
                );
                match &seen {
                    None => seen = Some((threads, observed)),
                    Some((first, expected)) => assert_eq!(
                        &observed, expected,
                        "{query}: at half the metered {label}, {threads} threads disagree with {first}"
                    ),
                }
            }
        }
    }
}

#[test]
fn a_filter_over_exact_values_trips_at_one_row() {
    assert_deterministic(
        &format!("SELECT ?s WHERE {{ ?s <{EX}v> ?v FILTER(?v * ?v * ?v > 0) }}"),
        QueryOptions::EMPTY,
    );
}

#[test]
fn a_projection_over_exact_values_trips_at_one_row() {
    assert_deterministic(
        &format!(
            "SELECT ?s (STR(?v * ?v * ?v) AS ?cube) (?d * ?d AS ?fine) \
             WHERE {{ ?s <{EX}v> ?v ; <{EX}d> ?d }}"
        ),
        QueryOptions::EMPTY,
    );
}

#[test]
fn per_group_sums_over_exact_values_trip_at_one_group() {
    assert_deterministic(
        &format!(
            "SELECT ?g (SUM(?v * ?v) AS ?sum) (AVG(?d) AS ?mean) \
             WHERE {{ ?s <{EX}v> ?v ; <{EX}d> ?d ; <{EX}g> ?g }} GROUP BY ?g"
        ),
        QueryOptions::EMPTY,
    );
}

#[test]
fn per_group_statistical_aggregates_over_exact_values_trip_at_one_group() {
    let mut registry = AggregateRegistry::default();
    registry.register_statistical_aggregates(STAT);
    let env = ExtensionEnv::over_aggregates(registry).expect("the statistical set reads cleanly");
    assert_deterministic(
        &format!(
            "SELECT ?g (AGG(<{STAT}VARIANCE>, ?v * ?v) AS ?variance) \
             (AGG(<{STAT}MEDIAN>, ?d) AS ?median) \
             WHERE {{ ?s <{EX}v> ?v ; <{EX}d> ?d ; <{EX}g> ?g }} GROUP BY ?g"
        ),
        QueryOptions::new().with_env(&env),
    );
}

#[test]
fn an_optional_filter_over_exact_values_trips_at_one_row() {
    assert_deterministic(
        &format!(
            "SELECT ?s ?d WHERE {{ ?s <{EX}v> ?v \
             OPTIONAL {{ {{ SELECT ?s ?d WHERE {{ ?s <{EX}d> ?d }} }} FILTER(?v * ?v * ?d > 0) }} }}"
        ),
        QueryOptions::EMPTY,
    );
}

/// A chain of forked loops — a sub-`SELECT`'s `FILTER` and projection, then a `BIND`
/// and a `FILTER` over it — trips inside the `BIND`'s product under the command line's
/// `--fuel` (a fuel ceiling over a metered base), and the scratch reported at the trip is
/// the same however the loops forked. On one worker the commit refuses the product; on
/// more, a worker stops first and the loop finishes in order, where the refused product
/// leaves the previous row's minted result unclaimed. A row committed whole therefore
/// claims the arena only as far as its own last growth charge, as the in-order loop does.
#[test]
fn a_chain_of_forked_loops_reports_one_scratch_figure_at_a_fuel_trip() {
    let mut builder = RdfDatasetBuilder::new();
    let v = builder.intern_iri(&format!("{EX}v"));
    for index in 0..6_000 {
        let s = builder.intern_iri(&format!("{EX}s{index}"));
        let big = builder.intern_literal(RdfLiteral::typed(
            format!("1{index:039}"),
            format!("{XSD}integer"),
        ));
        builder.push_quad(s, v, big, None);
    }
    let dataset = builder.freeze().expect("a valid fixture");
    let query = format!(
        "SELECT ?s ?r WHERE {{ {{ SELECT ?s (?v * ?v AS ?q) WHERE {{ ?s <{EX}v> ?v \
         FILTER(?v * ?v > 0) }} }} BIND(?q * ?q AS ?r) FILTER(?r > 0) }}"
    );
    let pool = |threads| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .expect("a pool")
    };
    let metered = pool(1).install(|| {
        observe(
            &dataset,
            &query,
            QueryOptions::EMPTY,
            &QueryGovernors::METERED,
        )
    });
    let fuel = metered
        .consumed
        .iter()
        .find(|(name, _)| name == "fuel")
        .map_or(0, |(_, value)| *value);
    for parts in [2_u64, 5] {
        let governors = QueryGovernors::METERED.with_fuel(fuel * parts / 7);
        let reference =
            pool(1).install(|| observe(&dataset, &query, QueryOptions::EMPTY, &governors));
        assert!(reference.tripped.is_some(), "{parts}/7 of the fuel trips");
        for threads in [2_usize, 4, 8, 32] {
            for _ in 0..3 {
                let observed = pool(threads)
                    .install(|| observe(&dataset, &query, QueryOptions::EMPTY, &governors));
                assert_eq!(
                    observed, reference,
                    "{parts}/7 of the fuel on {threads} threads"
                );
            }
        }
    }
}

/// The per-group loop's own row charges, with no arbitrary-precision work at all, trip
/// at one group too: a governed `GROUP BY` folds its groups in order.
#[test]
fn per_group_counts_trip_at_one_group() {
    assert_deterministic(
        &format!(
            "SELECT ?g (SUM(1) AS ?sum) (COUNT(?s) AS ?n) \
             WHERE {{ ?s <{EX}v> ?v ; <{EX}g> ?g }} GROUP BY ?g"
        ),
        QueryOptions::EMPTY,
    );
}

/// A scratch ceiling trips partway through a forked `FILTER` at one row: the working
/// set of each row's product grows with the row, the ceiling admits the products of
/// the early rows and not the late ones, and every pool trips at the same charge. The
/// trip is inside a predicate, so the `FILTER`'s output is withheld, as the loop run in
/// order on one context withholds it.
#[test]
fn a_scratch_ceiling_trips_at_one_row() {
    let dataset = dataset();
    let query = format!("SELECT ?s WHERE {{ ?s <{EX}w> ?w FILTER(?w * ?w > 0) }}");
    let run = |ceiling: u64| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .expect("a pool")
            .install(|| {
                observe(
                    &dataset,
                    &query,
                    QueryOptions::EMPTY,
                    &QueryGovernors::UNBOUNDED.with_max_scratch_bytes(ceiling),
                )
            })
    };
    // The smallest ceiling the whole loop fits under, by bisection.
    let (mut low, mut high) = (0_u64, 1_u64 << 30);
    while low + 1 < high {
        let middle = low + (high - low) / 2;
        if run(middle).tripped.is_none() {
            high = middle;
        } else {
            low = middle;
        }
    }
    let ceiling = high * 3 / 5;
    let reference = run(ceiling);
    let spent = reference
        .consumed
        .iter()
        .find(|(name, _)| name == "scratch-bytes")
        .map_or(0, |(_, value)| *value);
    assert!(
        reference.tripped.is_some()
            && reference.answer.starts_with("withheld at ")
            && spent > high / 4,
        "the ceiling trips partway, inside a predicate: {reference:.200?}"
    );
    for threads in [1_usize, 2, 4, 8, 32] {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .expect("a pool");
        for _ in 0..6 {
            let observed = pool.install(|| {
                observe(
                    &dataset,
                    &query,
                    QueryOptions::EMPTY,
                    &QueryGovernors::UNBOUNDED.with_max_scratch_bytes(ceiling),
                )
            });
            assert_eq!(observed, reference, "{threads} threads");
        }
    }
    for boundary in [high - 1, high, high + 1] {
        let expected = run(boundary);
        assert_eq!(
            expected.tripped.is_some(),
            boundary < high,
            "inclusive scratch ceiling"
        );
        for threads in [1, 4, 32] {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .expect("pool");
            let observed = pool.install(|| {
                observe(
                    &dataset,
                    &query,
                    QueryOptions::EMPTY,
                    &QueryGovernors::UNBOUNDED.with_max_scratch_bytes(boundary),
                )
            });
            assert_eq!(
                observed, expected,
                "scratch boundary {boundary} at {threads} workers"
            );
        }
    }
}

#[test]
fn inclusive_fuel_boundary_keeps_full_values_and_all_dimensions() {
    let dataset = dataset();
    let query = format!("SELECT ?s (?v * ?v AS ?square) WHERE {{ ?s <{EX}v> ?v }}");
    let metered = observe(
        &dataset,
        &query,
        QueryOptions::EMPTY,
        &QueryGovernors::METERED,
    );
    let fuel = metered
        .consumed
        .iter()
        .find(|(name, _)| name == "fuel")
        .expect("fuel")
        .1;
    for boundary in [fuel - 1, fuel, fuel + 1] {
        let governors = QueryGovernors::METERED.with_fuel(boundary);
        let reference = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .expect("pool")
            .install(|| observe(&dataset, &query, QueryOptions::EMPTY, &governors));
        assert_eq!(
            reference.tripped.is_some(),
            boundary < fuel,
            "inclusive fuel ceiling"
        );
        if boundary >= fuel {
            assert_eq!(reference, metered);
        }
        for threads in [4, 32] {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .expect("pool");
            let observed =
                pool.install(|| observe(&dataset, &query, QueryOptions::EMPTY, &governors));
            assert_eq!(
                observed, reference,
                "fuel boundary {boundary} at {threads} workers"
            );
        }
    }
}
