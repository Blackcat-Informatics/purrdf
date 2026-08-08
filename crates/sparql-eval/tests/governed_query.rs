// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Public-surface tests for governed SPARQL execution.
//!
//! Every test here drives the **public** API only — `NativeSparqlEngine::query_governed`,
//! `query_prepared_governed_view`, `query_governed_fallible_view`, and the outcome types
//! they return. That restriction is the point: a governor whose typed outcome is only
//! reachable from inside the crate governs nothing a consumer can act on, so these tests
//! are written from exactly the vantage a consumer has.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use purrdf_core::{
    BlankScope, GovernorEvidence, InMemoryPageProvider, PagedDataset, PagedQueryLimits, RdfDataset,
    RdfDatasetBuilder, ResourceDimension, SparqlRequest, SparqlResult, StopCause, TrippedGovernor,
};
use purrdf_sparql_eval::{
    CHARGE_SCHEDULE, CancellationFlag, ChargePoint, FallibleSparqlError, GOVERNOR_PROFILE_DIGEST,
    GOVERNOR_PROFILE_ID, GOVERNOR_PROFILE_VERSION, GovernedOutcome, HttpRemoteQuerySource,
    HttpRequest, NativeSparqlEngine, PartialAnswers, QueryExplanation, QueryGovernors,
    QueryOptions, RemoteError, STOP_POLL_FUEL, StopSignal, WallDeadline, resolve_precedence,
};

/// The number of `ex:p` edges in the fixture dataset.
const EDGES: usize = 8;

/// `EDGES` subjects, each with one `ex:p` edge — a query over it returns exactly `EDGES`
/// rows, which is what makes the inclusive cap boundary observable to the row.
fn fixture() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let p = builder.intern_iri("http://example.org/p");
    for index in 0..EDGES {
        let s = builder.intern_iri(&format!("http://example.org/s{index}"));
        let o = builder.intern_iri(&format!("http://example.org/o{index}"));
        builder.push_quad(s, p, o, None);
    }
    builder.freeze().expect("freeze fixture")
}

fn blank_fixture() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let p = builder.intern_iri("http://example.org/p");
    for index in 0..EDGES {
        let s = builder.intern_blank(&format!("b{index}"), BlankScope::DEFAULT);
        let o = builder.intern_iri(&format!("http://example.org/o{index}"));
        builder.push_quad(s, p, o, None);
    }
    builder.freeze().expect("freeze blank fixture")
}

fn request(query: &str) -> SparqlRequest<'_> {
    SparqlRequest {
        query,
        base_iri: None,
        substitutions: &[],
    }
}

fn one_remote_binding(_: HttpRequest<'_>) -> Result<Vec<u8>, RemoteError> {
    Ok(br#"{
      "head": { "vars": ["remote"] },
      "results": { "bindings": [
        { "remote": { "type": "literal", "value": "ok" } }
      ] }
    }"#
    .to_vec())
}

/// The all-rows SELECT the ceiling tests bound.
const ALL_ROWS: &str = "SELECT ?s ?o WHERE { ?s <http://example.org/p> ?o }";

/// Run `query` over the fixture under `governors`, asserting only that it is not an
/// error — a trip is an outcome, never an `Err`.
fn governed(query: &str, governors: &QueryGovernors) -> GovernedOutcome {
    NativeSparqlEngine::new()
        .query_governed(&fixture(), request(query), QueryOptions::EMPTY, governors)
        .expect("a tripped governor is an outcome, not a query error")
}

/// The row count of a solutions result.
fn row_count(result: &SparqlResult) -> usize {
    match result {
        SparqlResult::Solutions { rows, .. } => rows.len(),
        other => panic!("expected SELECT solutions, got: {other:?}"),
    }
}

/// The first-column IRIs of a solutions result, sorted — the comparable identity of an
/// answer set.
fn subjects(result: &SparqlResult) -> Vec<String> {
    match result {
        SparqlResult::Solutions { rows, .. } => {
            let mut out: Vec<String> = rows.iter().map(|row| format!("{:?}", row[0])).collect();
            out.sort();
            out
        }
        other => panic!("expected SELECT solutions, got: {other:?}"),
    }
}

/// The frozen graph a graph-producing query returned.
fn graph_of(result: &SparqlResult) -> &Arc<RdfDataset> {
    match result {
        SparqlResult::Graph(graph) => graph,
        other => panic!("expected a CONSTRUCT/DESCRIBE graph, got: {other:?}"),
    }
}

/// The number of **statements** in a graph result, in the same denomination the answer cap
/// charges a graph-producing form: every ordinary quad, every RDF 1.2 reifier binding, and
/// every annotation.
///
/// Written out here rather than borrowed from the evaluator on purpose. This is the
/// caller-visible reading of "how big is the answer", so if the governor ever came to
/// denominate something else — solution rows, say — the boundary tests below would notice
/// from outside instead of agreeing with the implementation by construction.
fn statement_count(graph: &RdfDataset) -> usize {
    graph.quad_count() + graph.reifiers().count() + graph.annotations().count()
}

/// The exhaustion an outcome must be, with its certified rows.
fn exhausted(outcome: &GovernedOutcome) -> (TrippedGovernor, &PartialAnswers, &GovernorEvidence) {
    match outcome {
        GovernedOutcome::Complete { result, .. } => {
            panic!("expected an exhausted budget, got a complete {result:?}")
        }
        GovernedOutcome::BudgetExhausted(exhausted) => {
            (exhausted.tripped, &exhausted.partial, &exhausted.evidence)
        }
    }
}

#[derive(Debug)]
struct PollCountdown {
    quiet: usize,
    polls: AtomicUsize,
    latched: AtomicBool,
}

impl PollCountdown {
    fn new(quiet: usize) -> Arc<Self> {
        Arc::new(Self {
            quiet,
            polls: AtomicUsize::new(0),
            latched: AtomicBool::new(false),
        })
    }
}

impl StopSignal for PollCountdown {
    fn poll(&self) -> Option<StopCause> {
        if self.latched.load(Ordering::Relaxed) {
            return Some(StopCause::Cancelled);
        }
        if self.polls.fetch_add(1, Ordering::Relaxed) < self.quiet {
            return None;
        }
        self.latched.store(true, Ordering::Relaxed);
        Some(StopCause::Cancelled)
    }
}

fn large_fixture(edges: usize) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let p = builder.intern_iri("http://example.org/p");
    for index in 0..edges {
        let s = builder.intern_iri(&format!("http://example.org/n{index}"));
        let o = builder.intern_iri(&format!("http://example.org/n{}", index + 1));
        builder.push_quad(s, p, o, None);
    }
    builder.freeze().expect("freeze large fixture")
}

fn stopped_partial_rows(dataset: &Arc<RdfDataset>, query: &str) -> usize {
    let outcome = NativeSparqlEngine::new()
        .query_governed(
            dataset,
            request(query),
            QueryOptions::EMPTY,
            &QueryGovernors::UNBOUNDED.with_stop_signal(PollCountdown::new(16)),
        )
        .expect("a stop is an outcome");
    let (tripped, partial, _) = exhausted(&outcome);
    assert_eq!(
        tripped,
        TrippedGovernor::Stopped {
            cause: StopCause::Cancelled
        }
    );
    let partial = partial
        .result()
        .expect("these monotone plans keep a lower bound");
    row_count(partial.result())
}

#[test]
fn stop_only_governors_checkpoint_inside_large_bgp_path_and_expression_loops() {
    const SIZE: usize = 256;
    let dataset = large_fixture(SIZE);

    let bgp = stopped_partial_rows(
        &dataset,
        "SELECT ?s ?o WHERE { ?s <http://example.org/p> ?o }",
    );
    assert!((1..SIZE).contains(&bgp), "BGP stopped at {bgp} rows");

    let path = stopped_partial_rows(
        &dataset,
        "SELECT ?o WHERE { <http://example.org/n0> <http://example.org/p>+ ?o }",
    );
    assert!((1..SIZE).contains(&path), "path stopped at {path} rows");

    let values = (0..SIZE)
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    let expression_query =
        format!("SELECT ?x WHERE {{ VALUES ?x {{ {values} }} FILTER(?x >= 0) }}");
    let expression = stopped_partial_rows(&dataset, &expression_query);
    assert!(
        (1..SIZE).contains(&expression),
        "expression loop stopped at {expression} rows"
    );
}

#[test]
fn ac1_query_governors_accepts_every_governor_and_reaches_evaluation() {
    // Every caller-settable governor at once, each generous enough that none can fire,
    // plus a live (unfired) stop signal. "Accepted" is not enough: the evidence must show
    // that the counters actually ran, which is what proves the configuration reached
    // evaluation rather than being parsed and dropped.
    let signal = CancellationFlag::new();
    let governors = QueryGovernors::UNBOUNDED
        .with_fuel(1_000_000)
        .with_max_answers(1_000)
        .with_max_intermediate_cells(1_000_000)
        .with_max_scratch_bytes(1_000_000)
        .with_max_remote_requests(4)
        .with_stop_signal(Arc::new(signal.clone()));

    let outcome = governed(ALL_ROWS, &governors);
    let GovernedOutcome::Complete {
        result, evidence, ..
    } = &outcome
    else {
        panic!("no governor could fire, so the query must complete: {outcome:?}");
    };
    assert_eq!(row_count(result), EDGES);
    assert!(evidence.is_complete(), "no governor tripped: {evidence:?}");
    assert_eq!(outcome.tripped(), None);

    // The ceilings are echoed back exactly as set, on every dimension.
    assert_eq!(evidence.limit_for(ResourceDimension::Fuel), 1_000_000);
    assert_eq!(evidence.limit_for(ResourceDimension::AnswerRows), 1_000);
    assert_eq!(
        evidence.limit_for(ResourceDimension::IntermediateCells),
        1_000_000
    );
    assert_eq!(
        evidence.limit_for(ResourceDimension::ScratchBytes),
        1_000_000
    );
    assert_eq!(evidence.limit_for(ResourceDimension::RemoteRequests), 4);

    // And the counters ran: fuel was spent, every answer row was charged, and a bag was
    // measured. A configuration that never reached evaluation would report zeroes.
    assert!(
        evidence.consumed_in(ResourceDimension::Fuel) > 0,
        "fuel must be charged during evaluation: {evidence:?}"
    );
    assert_eq!(
        evidence.consumed_in(ResourceDimension::AnswerRows),
        EDGES as u64
    );
    assert!(
        evidence.consumed_in(ResourceDimension::IntermediateCells) > 0,
        "the intermediate-cell peak must be observed: {evidence:?}"
    );
    assert!(
        !signal.is_cancelled(),
        "the stop signal is polled, never flipped, by the evaluator"
    );
}

#[test]
fn ac2_trip_is_neither_complete_nor_error() {
    // The complete answer, as the oracle every partial result is checked against.
    let complete = governed(ALL_ROWS, &QueryGovernors::UNBOUNDED);
    let GovernedOutcome::Complete { result: full, .. } = &complete else {
        panic!("the ungoverned-equivalent run must complete: {complete:?}");
    };
    let all = subjects(full);
    assert_eq!(all.len(), EDGES);

    let expired = WallDeadline::after(Duration::ZERO);
    assert!(
        expired.poll().is_some(),
        "a zero budget is expired on its first poll"
    );
    let cancelled = CancellationFlag::new();
    cancelled.cancel();

    // The `cuts_rows` column says whether this governor can stop the search mid-answer.
    //
    // The intermediate-cell ceiling is the one that does not merely stop the search but
    // *precedes* it: the cost planner's estimated peak for this plan already exceeds a
    // ceiling of zero, so the query is refused at admission and never evaluated. That is
    // the only mechanism that can act before a materialized bag exists — the meter, by
    // construction, can only observe a bag it already holds — so the trip it reports is
    // `Refused`, carrying the ESTIMATE that was refused rather than a consumption nothing
    // measured, and the certified answer it leaves is empty.
    let cases: [(&str, QueryGovernors, TrippedGovernor, bool); 5] = [
        (
            "fuel",
            QueryGovernors::UNBOUNDED.with_fuel(0),
            TrippedGovernor::Budget {
                dimension: ResourceDimension::Fuel,
                limit: 0,
                consumed: 1,
            },
            true,
        ),
        (
            "answer cap",
            QueryGovernors::UNBOUNDED.with_max_answers(3),
            TrippedGovernor::Budget {
                dimension: ResourceDimension::AnswerRows,
                limit: 3,
                consumed: 4,
            },
            true,
        ),
        (
            "intermediate cells",
            QueryGovernors::UNBOUNDED.with_max_intermediate_cells(0),
            TrippedGovernor::Refused {
                dimension: ResourceDimension::IntermediateCells,
                limit: 0,
                estimate: (EDGES * 2) as u64,
            },
            true,
        ),
        (
            "deadline",
            QueryGovernors::UNBOUNDED.with_stop_signal(Arc::new(expired)),
            TrippedGovernor::Stopped {
                cause: StopCause::Deadline,
            },
            true,
        ),
        (
            "cancellation",
            QueryGovernors::UNBOUNDED.with_stop_signal(Arc::new(cancelled)),
            TrippedGovernor::Stopped {
                cause: StopCause::Cancelled,
            },
            true,
        ),
    ];

    for (label, governors, expected, cuts_rows) in cases {
        let outcome = governed(ALL_ROWS, &governors);
        let (tripped, partial, evidence) = exhausted(&outcome);
        assert_eq!(tripped, expected, "{label}: the trip names its governor");
        assert_eq!(
            evidence.tripped,
            Some(expected),
            "{label}: the evidence and the outcome report one trip"
        );
        assert!(!outcome.is_complete(), "{label}: a trip is not completion");

        // Whatever crossed is a sound subset of the true answer: these plans are a bare
        // BGP under a projection, so every surviving row is a certified answer.
        let certified = partial
            .result()
            .unwrap_or_else(|| panic!("{label}: a prefix-monotone plan certifies its rows"));
        assert!(partial.is_certain(), "{label}: {partial:?}");
        assert!(
            certified.is_positional_prefix(),
            "{label}: a larger budget returns these rows first"
        );
        for subject in subjects(certified.result()) {
            assert!(
                all.contains(&subject),
                "{label}: a partial answer invented the row {subject}"
            );
        }
        let reached = row_count(certified.result());
        assert!(
            reached <= EDGES,
            "{label}: a partial answer cannot exceed the true one"
        );
        assert!(
            !cuts_rows || reached < EDGES,
            "{label}: a governor that stops the search mid-answer must leave fewer rows"
        );
    }
}

#[test]
fn ac2_ask_with_a_nonempty_certain_set_keeps_the_trip_and_carries_true() {
    // `ASK` asks whether ANY solution exists, so one row that is certainly an answer
    // settles its semantic value outright. The execution still stopped operationally,
    // however, so the true value travels as a certified partial under BudgetExhausted;
    // reporting Complete beside evidence that says a governor tripped is contradictory.
    let ask = "ASK { ?s <http://example.org/p> ?o }";
    let metered = governed(ask, &QueryGovernors::METERED);
    let cost = metered
        .evidence()
        .consumed_in(ResourceDimension::Fuel)
        .max(1);

    let mut settled_early = 0_usize;
    for fuel in 0..cost {
        // Strictly below the measured cost, the search cannot have finished.
        let outcome = governed(ask, &QueryGovernors::UNBOUNDED.with_fuel(fuel));
        assert!(
            !outcome.is_complete(),
            "a sub-boundary execution cannot be Complete: {outcome:?}"
        );
        let (_, partial, evidence) = exhausted(&outcome);
        assert_eq!(outcome.tripped(), evidence.tripped);
        if partial
            .result()
            .is_some_and(|partial| matches!(partial.result(), SparqlResult::Boolean(true)))
        {
            settled_early += 1;
        }
    }
    assert!(
        settled_early > 0,
        "a certain row must be able to settle ASK before the budget does"
    );
}

#[test]
fn an_earlier_fuel_trip_cannot_bypass_the_answer_cap_on_any_result_sequence() {
    let queries = [
        ALL_ROWS,
        "CONSTRUCT { ?s <http://example.org/q> ?o } WHERE { ?s <http://example.org/p> ?o }",
        "DESCRIBE ?s WHERE { ?s <http://example.org/p> ?o }",
    ];

    for query in queries {
        let metered = governed(query, &QueryGovernors::METERED)
            .evidence()
            .consumed_in(ResourceDimension::Fuel);
        let mut witnessed = false;
        for fuel in 0..metered {
            let outcome = governed(
                query,
                &QueryGovernors::UNBOUNDED
                    .with_fuel(fuel)
                    .with_max_answers(1),
            );
            let Some(exhausted) = outcome.exhausted() else {
                continue;
            };
            if !matches!(
                exhausted.tripped,
                TrippedGovernor::Budget {
                    dimension: ResourceDimension::Fuel,
                    ..
                }
            ) || exhausted
                .evidence
                .consumed_in(ResourceDimension::AnswerRows)
                != 2
            {
                continue;
            }

            let payload_size =
                exhausted
                    .partial
                    .result()
                    .map_or(0, |partial| match partial.result() {
                        SparqlResult::Solutions { rows, .. } => rows.len(),
                        SparqlResult::Graph(graph) => statement_count(graph),
                        SparqlResult::Boolean(_) => 1,
                    });
            assert!(
                payload_size <= 1,
                "the first trip stayed fuel but the cap leaked {payload_size} answers for {query}"
            );
            witnessed = true;
            break;
        }
        assert!(
            witnessed,
            "the fixture must exercise a fuel-first then answer-cap cut for {query}"
        );
    }
}

#[test]
fn cutting_an_upper_bound_at_the_answer_cap_withholds_the_bound() {
    // A trip in the right arm of MINUS leaves an upper bound: subtracting only the
    // right rows reached so far can leave false-positive rows, but it cannot omit a true
    // answer. Removing an arbitrary row from that upper bound at the answer cap would
    // break precisely that guarantee, because the removed row could be a true answer.
    let query = "SELECT ?s WHERE { \
        ?s <http://example.org/p> ?o \
        MINUS { ?s <http://example.org/p> ?z } \
    }";
    let measured = governed(query, &QueryGovernors::METERED)
        .evidence()
        .consumed_in(ResourceDimension::Fuel);

    let mut witnessed = false;
    for fuel in 0..measured {
        let outcome = governed(
            query,
            &QueryGovernors::UNBOUNDED
                .with_fuel(fuel)
                .with_max_answers(1),
        );
        let Some(exhausted) = outcome.exhausted() else {
            continue;
        };
        if !matches!(
            exhausted.tripped,
            TrippedGovernor::Budget {
                dimension: ResourceDimension::Fuel,
                ..
            }
        ) || exhausted
            .evidence
            .consumed_in(ResourceDimension::AnswerRows)
            != 2
        {
            continue;
        }

        let PartialAnswers::Unknown(barrier) = &exhausted.partial else {
            panic!(
                "answer-cap removal must not leave a forged upper bound: {:?}",
                exhausted.partial
            );
        };
        assert_eq!(barrier.operator(), "answer-cap");
        assert!(
            exhausted.partial.result().is_none(),
            "no row may cross after the upper-bound certificate collapses"
        );
        witnessed = true;
        break;
    }

    assert!(
        witnessed,
        "the fixture must reach a fuel-truncated MINUS upper bound that the cap cuts"
    );
}

#[test]
fn public_blank_node_filter_cannot_forge_an_upper_bound() {
    let dataset = blank_fixture();
    let query = "SELECT ?s WHERE { \
        ?s <http://example.org/p> ?o \
        MINUS { ?s <http://example.org/p> ?z } \
    }";
    let engine = NativeSparqlEngine::new();
    let measured = engine
        .query_governed(
            &dataset,
            request(query),
            QueryOptions::EMPTY,
            &QueryGovernors::METERED,
        )
        .expect("metered query")
        .evidence()
        .consumed_in(ResourceDimension::Fuel);

    let mut witnessed = false;
    for fuel in 0..measured {
        let outcome = engine
            .query_governed(
                &dataset,
                request(query),
                QueryOptions::EMPTY,
                &QueryGovernors::UNBOUNDED.with_fuel(fuel),
            )
            .expect("a fuel trip is an outcome");
        let Some(exhausted) = outcome.exhausted() else {
            continue;
        };
        let PartialAnswers::AtMost(partial) = &exhausted.partial else {
            continue;
        };
        if row_count(partial.result()) == 0 {
            continue;
        }

        let untouched = exhausted.partial.clone().withholding_blank_nodes(|_| false);
        assert!(
            matches!(untouched, PartialAnswers::AtMost(_)),
            "a no-op filter keeps the original upper bound"
        );

        let filtered = exhausted.partial.clone().withholding_blank_nodes(|_| true);
        let PartialAnswers::Unknown(barrier) = filtered else {
            panic!("removing a possible answer must discard the upper bound");
        };
        assert_eq!(barrier.operator(), "blank-node-filter");
        witnessed = true;
        break;
    }

    assert!(
        witnessed,
        "the fixture must expose a non-empty upper bound with blank-node rows"
    );
}

#[test]
fn every_complete_outcome_has_complete_evidence() {
    for query in [
        ALL_ROWS,
        "ASK { ?s <http://example.org/p> ?o }",
        "CONSTRUCT { ?s <http://example.org/q> ?o } WHERE { ?s <http://example.org/p> ?o }",
        "DESCRIBE ?s WHERE { ?s <http://example.org/p> ?o }",
    ] {
        let outcome = governed(query, &QueryGovernors::METERED);
        let GovernedOutcome::Complete { evidence, .. } = outcome else {
            panic!("an unreachable ceiling must complete for {query}")
        };
        assert!(evidence.is_complete(), "complete outcome had {evidence:?}");
        assert_eq!(evidence.tripped, None);
    }
}

#[test]
fn budget_trip_can_never_masquerade_as_an_empty_complete_result() {
    // The failure this whole surface exists to prevent: a query that stopped early
    // reported as a query that finished and found nothing. Zero fuel yields zero rows,
    // and the outcome must still be exhaustion.
    let outcome = governed(ALL_ROWS, &QueryGovernors::UNBOUNDED.with_fuel(0));
    let (tripped, partial, _) = exhausted(&outcome);
    assert!(matches!(
        tripped,
        TrippedGovernor::Budget {
            dimension: ResourceDimension::Fuel,
            ..
        }
    ));
    let certified = partial
        .result()
        .expect("an empty lower bound is still a bound");
    assert_eq!(
        row_count(certified.result()),
        0,
        "nothing was reached before the first charge"
    );
    assert!(
        !outcome.is_complete(),
        "an empty partial answer is not an empty complete answer"
    );
    assert!(
        outcome.into_complete().is_err(),
        "reducing the outcome to a Result keeps the trip on the error side"
    );
}

#[test]
fn answer_cap_boundary_is_inclusive() {
    // Ceilings are inclusive everywhere in this engine: consumption EQUAL to the cap is
    // admitted, and only consumption that exceeds it trips.
    let at_the_cap = governed(
        ALL_ROWS,
        &QueryGovernors::UNBOUNDED.with_max_answers(EDGES as u64),
    );
    let GovernedOutcome::Complete {
        result, evidence, ..
    } = &at_the_cap
    else {
        panic!("cap == result size is complete: {at_the_cap:?}");
    };
    assert_eq!(row_count(result), EDGES);
    assert!(evidence.is_complete());

    let one_below = governed(
        ALL_ROWS,
        &QueryGovernors::UNBOUNDED.with_max_answers(EDGES as u64 - 1),
    );
    let (tripped, partial, _) = exhausted(&one_below);
    assert_eq!(
        tripped,
        TrippedGovernor::Budget {
            dimension: ResourceDimension::AnswerRows,
            limit: EDGES as u64 - 1,
            consumed: EDGES as u64,
        },
        "cap == size - 1 trips on the row that would have exceeded it"
    );
    let certified = partial.result().expect("the admitted prefix is certified");
    assert_eq!(
        row_count(certified.result()),
        EDGES - 1,
        "the cap admits exactly its own count of rows"
    );
    assert!(partial.is_certain() && certified.is_positional_prefix());
}

#[test]
fn a_withheld_bound_names_the_barrier_that_withheld_it() {
    // An aggregate over a truncated input is a different number, not a subset of the true
    // one, so no row may cross a `GROUP BY` below a trip. The caller is handed the
    // actionable half instead: the operator that withheld them.
    let query = "SELECT (COUNT(?o) AS ?n) WHERE { ?s <http://example.org/p> ?o }";
    let mut withheld = 0_usize;
    // The exact fuel at which the trip lands inside the grouped subtree is a property of
    // the charge schedule, not of this test, so the whole low range is swept: every
    // withheld outcome must name `Group`, and at least one must occur.
    for fuel in 0..24_u64 {
        let outcome = governed(query, &QueryGovernors::UNBOUNDED.with_fuel(fuel));
        let GovernedOutcome::BudgetExhausted(exhausted) = &outcome else {
            continue;
        };
        if let PartialAnswers::Unknown(barrier) = &exhausted.partial {
            withheld += 1;
            assert_eq!(
                barrier.operator(),
                "Group",
                "the barrier names the operator that withheld the rows"
            );
            assert!(
                barrier.to_string().contains("Group"),
                "the barrier renders the operator it names: {barrier}"
            );
            assert!(exhausted.partial.result().is_none(), "no row may cross");
            assert!(!exhausted.partial.is_certain());
        }
    }
    assert!(
        withheld > 0,
        "a trip below GROUP BY must be reachable through the public surface"
    );
}

#[test]
fn metered_governors_measure_an_execution_without_bounding_it() {
    // The intended way to size a budget: run under METERED, read the cost, then set the
    // real ceilings from it. Nothing can trip, so the outcome is complete.
    let outcome = governed(ALL_ROWS, &QueryGovernors::METERED);
    let GovernedOutcome::Complete {
        result, evidence, ..
    } = &outcome
    else {
        panic!("METERED bounds nothing and cannot trip: {outcome:?}");
    };
    assert_eq!(row_count(result), EDGES);
    let fuel = evidence.consumed_in(ResourceDimension::Fuel);
    assert!(
        fuel > 0,
        "a metered run reports what it spent: {evidence:?}"
    );

    // And the measurement is a budget: the same query under exactly that much fuel
    // completes, one unit less does not.
    let at_cost = governed(ALL_ROWS, &QueryGovernors::UNBOUNDED.with_fuel(fuel));
    assert!(
        at_cost.is_complete(),
        "the measured cost is a sufficient budget: {at_cost:?}"
    );
    let starved = governed(ALL_ROWS, &QueryGovernors::UNBOUNDED.with_fuel(fuel - 1));
    assert!(
        !starved.is_complete(),
        "one unit below the measured cost cannot finish: {starved:?}"
    );
}

#[test]
fn d6_precedence_order() {
    // The order, as the kernel vocabulary ranks it:
    //   EvalError > stop signal (cancelled > deadline) > intermediate cells > fuel >
    //   answer cap.
    // The ranking itself is one function, exercised here on every adjacent pair through
    // the public `resolve_precedence`.
    let cancelled = TrippedGovernor::Stopped {
        cause: StopCause::Cancelled,
    };
    let deadline = TrippedGovernor::Stopped {
        cause: StopCause::Deadline,
    };
    let budget = |dimension| TrippedGovernor::Budget {
        dimension,
        limit: 1,
        consumed: 2,
    };
    let cells = budget(ResourceDimension::IntermediateCells);
    let fuel = budget(ResourceDimension::Fuel);
    let cap = budget(ResourceDimension::AnswerRows);
    let refused = TrippedGovernor::Refused {
        dimension: ResourceDimension::IntermediateCells,
        limit: 1,
        estimate: 2,
    };
    for (winner, loser) in [
        (cancelled, deadline),
        (deadline, refused),
        (refused, cells),
        (cells, fuel),
        (fuel, cap),
    ] {
        assert_eq!(resolve_precedence([winner, loser]), Some(winner));
        assert_eq!(
            resolve_precedence([loser, winner]),
            Some(winner),
            "precedence is a rank, not an argument order"
        );
    }
    assert_eq!(resolve_precedence(std::iter::empty()), None);

    // End to end: a stop signal that is already firing outranks every ceiling that is
    // crossed at the same charge point. Every ceiling below is set to zero, so all of
    // them are true the instant evaluation starts.
    let flag = CancellationFlag::new();
    flag.cancel();
    let outcome = governed(
        ALL_ROWS,
        &QueryGovernors::UNBOUNDED
            .with_fuel(0)
            .with_max_intermediate_cells(0)
            .with_max_answers(0)
            .with_stop_signal(Arc::new(flag)),
    );
    let (tripped, _, _) = exhausted(&outcome);
    assert_eq!(
        tripped,
        TrippedGovernor::Stopped {
            cause: StopCause::Cancelled
        },
        "a firing stop signal outranks every ceiling crossed with it"
    );

    // Admission precedes EVERY charge point, so a plan the cost model already predicts
    // will breach the cell ceiling reports that refusal rather than whichever ceiling the
    // first charge would have crossed. This is the one ordering rule the charge-point
    // precedence cannot express, because the refusal happens before there is a charge
    // point to rank against.
    let outcome = governed(
        ALL_ROWS,
        &QueryGovernors::UNBOUNDED
            .with_fuel(0)
            .with_max_intermediate_cells(0)
            .with_max_answers(0),
    );
    let (tripped, _, _) = exhausted(&outcome);
    assert_eq!(
        tripped,
        TrippedGovernor::Refused {
            dimension: ResourceDimension::IntermediateCells,
            limit: 0,
            estimate: (EDGES * 2) as u64,
        }
    );

    // Ceilings that are crossed at DIFFERENT charge points report the one crossed first —
    // precedence is evaluated over the conditions already true at a charge point, never
    // over conditions that might become true later. Fuel is charged on entering the first
    // algebra node, before any bag exists to measure or any answer row exists to admit,
    // so it is the reported trip even though it ranks below the cell ceiling. With the
    // cell ceiling raised past the estimate, admission passes and this is again what the
    // evaluator reports.
    let outcome = governed(
        ALL_ROWS,
        &QueryGovernors::UNBOUNDED
            .with_fuel(0)
            .with_max_intermediate_cells(u64::MAX - 1)
            .with_max_answers(0),
    );
    let (tripped, _, _) = exhausted(&outcome);
    assert_eq!(
        tripped,
        TrippedGovernor::Budget {
            dimension: ResourceDimension::Fuel,
            limit: 0,
            consumed: 1,
        }
    );

    // And a genuine failure outranks a governor outright: a query with no answer must
    // never be reported as a query that ran out of budget, whatever the budget was doing.
    // A non-silent SERVICE with no configured source is that failure.
    let engine = NativeSparqlEngine::new();
    let diagnostic = engine
        .query_governed(
            &fixture(),
            request("SELECT * WHERE { SERVICE <http://example.org/remote> { ?a ?b ?c } }"),
            QueryOptions::EMPTY,
            &QueryGovernors::METERED,
        )
        .expect_err("a federation failure is an error, not an exhausted budget");
    assert_eq!(diagnostic.code, "native-sparql-query-eval");

    // The same holds before evaluation begins: a query that does not parse is a parse
    // diagnostic even under a governor that is already firing.
    let flag = CancellationFlag::new();
    flag.cancel();
    let diagnostic = engine
        .query_governed(
            &fixture(),
            request("SELECT WHERE {"),
            QueryOptions::EMPTY,
            &QueryGovernors::UNBOUNDED.with_stop_signal(Arc::new(flag)),
        )
        .expect_err("an unparseable query is a parse error, not an exhausted budget");
    assert_eq!(diagnostic.code, "native-sparql-query-parse");
}

#[test]
fn prepared_plans_do_not_share_a_budget_across_runs() {
    // Governors are per call. The same prepared plan run twice under the same ceilings
    // must behave identically both times: an engine-held state would drain the first
    // run's consumption into the second and make the second trip for no reason.
    let dataset = fixture();
    let engine = NativeSparqlEngine::new();
    let prepared = engine.prepare_query(ALL_ROWS, None).expect("prepare");
    let governors = QueryGovernors::UNBOUNDED
        .with_fuel(1_000)
        .with_max_answers(EDGES as u64);

    let mut fuel_spent = Vec::new();
    for run in 0..3 {
        let outcome = engine
            .query_prepared_governed_view(
                &*dataset,
                &prepared,
                &[],
                QueryOptions::EMPTY,
                &governors,
            )
            .expect("prepared governed run");
        let GovernedOutcome::Complete {
            result, evidence, ..
        } = &outcome
        else {
            panic!("run {run} must complete under a budget sized for it: {outcome:?}");
        };
        assert_eq!(row_count(result), EDGES);
        fuel_spent.push(evidence.consumed_in(ResourceDimension::Fuel));
    }
    assert_eq!(
        fuel_spent[0], fuel_spent[1],
        "consumption is per execution, never cumulative"
    );
    assert_eq!(fuel_spent[1], fuel_spent[2]);
}

#[test]
fn a_fallible_view_reports_a_trip_as_budget_exhausted_with_both_meters() {
    // The governed fallible lane carries two independent measurements of one execution:
    // the view's pages and bytes, and the evaluator's fuel, rows, and cells.
    let paged = PagedDataset::from_provider(Arc::new(InMemoryPageProvider::new(vec![fixture()])))
        .expect("seal page");
    let engine = NativeSparqlEngine::new();

    let view = paged.query_view(PagedQueryLimits::UNBOUNDED);
    let complete = engine
        .query_governed_fallible_view(
            &view,
            request(ALL_ROWS),
            QueryOptions::EMPTY,
            &QueryGovernors::METERED,
        )
        .expect("a metered run over a ready view completes");
    assert_eq!(row_count(&complete.result), EDGES);
    assert!(complete.evidence.governors.is_complete());
    assert!(
        complete
            .evidence
            .governors
            .consumed_in(ResourceDimension::Fuel)
            > 0
    );

    let view = paged.query_view(PagedQueryLimits::UNBOUNDED);
    let error = engine
        .query_governed_fallible_view(
            &view,
            request(ALL_ROWS),
            QueryOptions::EMPTY,
            &QueryGovernors::UNBOUNDED.with_max_answers(2),
        )
        .expect_err("a trip is not a complete result");
    match &error {
        FallibleSparqlError::BudgetExhausted {
            tripped,
            partial,
            evidence,
        } => {
            assert_eq!(
                *tripped,
                TrippedGovernor::Budget {
                    dimension: ResourceDimension::AnswerRows,
                    limit: 2,
                    consumed: 3,
                }
            );
            let certified = partial.result().expect("the admitted prefix is certified");
            assert_eq!(row_count(certified.result()), 2);
            assert_eq!(evidence.governors.tripped, Some(*tripped));
        }
        other => panic!("expected an exhausted budget, got: {other:?}"),
    }
    assert_eq!(
        error.tripped(),
        Some(TrippedGovernor::Budget {
            dimension: ResourceDimension::AnswerRows,
            limit: 2,
            consumed: 3,
        })
    );
    assert!(error.partial_answers().is_some());
    assert!(
        error.diagnostic().is_none() && error.operational_error().is_none(),
        "an exhausted budget is neither a query failure nor a view failure"
    );
}

#[test]
fn a_fallible_federated_view_composes_page_and_remote_evidence() {
    let paged = PagedDataset::from_provider(Arc::new(InMemoryPageProvider::new(vec![fixture()])))
        .expect("seal page");
    let source = HttpRemoteQuerySource::new(one_remote_binding);
    let query = "SELECT ?s ?remote WHERE {
        ?s <http://example.org/p> ?o
        SERVICE <https://query.example/sparql> { BIND(\"ok\" AS ?remote) }
    }";
    let engine = NativeSparqlEngine::new();

    let view = paged.query_view(PagedQueryLimits::UNBOUNDED);
    let complete = engine
        .query_governed_fallible_with_source_view(
            &view,
            request(query),
            &source,
            QueryOptions::EMPTY,
            &QueryGovernors::METERED,
        )
        .expect("the local page and remote response are both ready");
    assert_eq!(row_count(&complete.result), EDGES);
    assert_eq!(complete.evidence.view.consumed_pages, 1);
    assert!(complete.evidence.view.consumed_bytes > 0);
    assert_eq!(
        complete
            .evidence
            .governors
            .consumed_in(ResourceDimension::RemoteRequests),
        1
    );
    let complete_fuel = complete
        .evidence
        .governors
        .consumed_in(ResourceDimension::Fuel);

    let view = paged.query_view(PagedQueryLimits::UNBOUNDED);
    let exhausted = engine
        .query_governed_fallible_with_source_view(
            &view,
            request(query),
            &source,
            QueryOptions::EMPTY,
            &QueryGovernors::UNBOUNDED.with_fuel(complete_fuel - 1),
        )
        .expect_err("the final fuel unit prevents a completeness certificate");
    let FallibleSparqlError::BudgetExhausted { evidence, .. } = exhausted else {
        panic!("expected a typed governor trip")
    };
    assert_eq!(evidence.view.consumed_pages, 1);
    assert!(evidence.governors.tripped().is_some());
}

// ---------------------------------------------------------------------------
// Pushdown: the cap that PREVENTS work rather than reporting it
// ---------------------------------------------------------------------------

/// A store wide enough that scanning all of it costs visibly more than scanning a handful
/// of rows, so "the scan stopped" is separable from "the answer was cut".
const WIDE_EDGES: usize = 2_000;

/// `edges` subjects on `ex:p`, with zero-padded local names so the first *k* rows of a
/// scan are the same *k* IRIs whatever `edges` is.
fn wide_fixture(edges: usize) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let p = builder.intern_iri("http://example.org/p");
    for index in 0..edges {
        let s = builder.intern_iri(&format!("http://example.org/s{index:06}"));
        let o = builder.intern_iri(&format!("http://example.org/o{index:06}"));
        builder.push_quad(s, p, o, None);
    }
    builder.freeze().expect("freeze wide fixture")
}

/// Every `bgp-candidate-quad` unit the ledger attributes to any node of `explanation`.
fn candidate_quads(explanation: &QueryExplanation) -> u64 {
    explanation
        .ledger()
        .iter()
        .map(|node| node.fuel_at(ChargePoint::BgpCandidateQuad))
        .sum()
}

#[test]
fn answer_cap_stops_the_scan_rather_than_materializing_everything() {
    // The claim under test is *prevention*, not truncation. A cap applied only at the root
    // produces exactly the same answer as a cap pushed into the scan — so an assertion on
    // the rows cannot tell the two apart, and every such assertion would still pass on a
    // build with no pushdown at all. What separates them is the work: a root-only cap
    // scans the whole store and discards, and a pushed one stops.
    //
    // Two observables, from two directions.

    // (1) The whole execution's cost, over two stores that differ only in size. Under the
    //     same cap the reported fuel must be IDENTICAL: a scan that stops at the ceiling
    //     cannot know how many quads it did not look at, while a scan that materialises
    //     everything charges once per quad and would report four times as much on the
    //     larger store.
    const CAP: u64 = 5;
    let engine = NativeSparqlEngine::new();
    // METERED engages every counter at a ceiling nothing can reach, then the cap replaces
    // the answer-row ceiling alone — so fuel is measured while the cap is the only thing
    // that can fire.
    let capped = QueryGovernors::METERED.with_max_answers(CAP);

    let mut spent = Vec::new();
    let mut first_rows = Vec::new();
    for edges in [WIDE_EDGES, WIDE_EDGES * 4] {
        let dataset = wide_fixture(edges);
        let outcome = engine
            .query_governed(&dataset, request(ALL_ROWS), QueryOptions::EMPTY, &capped)
            .expect("a capped run is an outcome, not an error");
        let exhausted = outcome
            .exhausted()
            .unwrap_or_else(|| panic!("a cap of {CAP} below {edges} answers must trip"));
        assert_eq!(
            exhausted.tripped,
            TrippedGovernor::Budget {
                dimension: ResourceDimension::AnswerRows,
                limit: CAP,
                consumed: CAP + 1,
            },
            "the pushed ceiling is the cap PLUS ONE, precisely so the cap itself can still \
             tell an exactly-full answer from an overflowing one"
        );
        let certified = exhausted
            .partial
            .result()
            .expect("a bare BGP under a projection certifies its rows");
        assert_eq!(row_count(certified.result()), CAP as usize);
        assert!(certified.is_positional_prefix());
        first_rows.push(subjects(certified.result()));
        spent.push(exhausted.evidence.consumed_in(ResourceDimension::Fuel));

        // The scale the capped cost is being separated from: the same query over the same
        // store with nothing capped.
        let whole = engine
            .query_governed(
                &dataset,
                request(ALL_ROWS),
                QueryOptions::EMPTY,
                &QueryGovernors::METERED,
            )
            .expect("a metered run completes");
        assert!(
            spent[spent.len() - 1] * 20 < whole.evidence().consumed_in(ResourceDimension::Fuel),
            "a capped run over {edges} edges spent {} fuel against an uncapped {}; the cap \
             is not reaching the scan",
            spent[spent.len() - 1],
            whole.evidence().consumed_in(ResourceDimension::Fuel)
        );
    }
    assert_eq!(
        spent[0], spent[1],
        "quadrupling the store changed what a capped query cost, so the scan is still \
         being materialised and cut at the root"
    );
    assert_eq!(
        first_rows[0], first_rows[1],
        "the pushdown must not change WHICH rows are the answer's first rows"
    );

    // (2) The ledger's own charge counts, per node. `LIMIT` is the same pushdown reached
    //     through query semantics rather than through an operational cap, and EXPLAIN
    //     reports what each node actually charged — so the `bgp-candidate-quad` column is
    //     a direct count of the candidate quads the scan examined.
    let dataset = wide_fixture(WIDE_EDGES);
    let limited = engine
        .explain_query(
            &dataset,
            "SELECT ?s ?o WHERE { ?s <http://example.org/p> ?o } LIMIT 5",
            None,
        )
        .expect("explain the limited query");
    let unlimited = engine
        .explain_query(&dataset, ALL_ROWS, None)
        .expect("explain the unlimited query");
    assert_eq!(
        candidate_quads(&unlimited),
        WIDE_EDGES as u64,
        "an unlimited scan examines every quad exactly once"
    );
    assert_eq!(
        candidate_quads(&limited),
        5,
        "`LIMIT 5` needs five candidate quads and must charge for five — not for the \
         {WIDE_EDGES} the store holds"
    );

    // And the answers agree, which is the guarantee the pushdown is not allowed to buy
    // performance with: the limited plan's rows are the unlimited plan's first rows.
    let unlimited_rows = engine
        .query_governed(
            &dataset,
            request(ALL_ROWS),
            QueryOptions::EMPTY,
            &QueryGovernors::UNBOUNDED,
        )
        .expect("the unlimited query");
    let GovernedOutcome::Complete { result, .. } = &unlimited_rows else {
        panic!("nothing is bounded, so it completes: {unlimited_rows:?}");
    };
    let limited_rows = engine
        .query_governed(
            &dataset,
            request("SELECT ?s ?o WHERE { ?s <http://example.org/p> ?o } LIMIT 5"),
            QueryOptions::EMPTY,
            &QueryGovernors::UNBOUNDED,
        )
        .expect("the limited query");
    let GovernedOutcome::Complete {
        result: limited_result,
        ..
    } = &limited_rows
    else {
        panic!("nothing is bounded, so it completes: {limited_rows:?}");
    };
    let (SparqlResult::Solutions { rows: all, .. }, SparqlResult::Solutions { rows: five, .. }) =
        (result, limited_result)
    else {
        panic!("both are SELECTs");
    };
    assert_eq!(five.len(), 5);
    assert_eq!(
        &all[..5],
        &five[..],
        "a stopped scan must return the same rows a full scan's first five were"
    );
}

// ---------------------------------------------------------------------------
// Admission: the refusal that happens before the first charge
// ---------------------------------------------------------------------------

#[test]
fn admission_refuses_an_estimated_ceiling_breach_before_evaluating() {
    let engine = NativeSparqlEngine::new();
    let dataset = fixture();

    // A ceiling the cost model already predicts this plan will breach. Nothing is
    // evaluated: the refusal is decided from the plan and the dataset's statistics alone.
    let refused = engine
        .query_governed(
            &dataset,
            request(ALL_ROWS),
            QueryOptions::EMPTY,
            &QueryGovernors::METERED.with_max_intermediate_cells(1),
        )
        .expect("a refusal is an outcome, not an error");
    let exhausted = refused
        .exhausted()
        .expect("the estimate exceeds the ceiling, so the plan is refused");

    // The trip NAMES the governor, and names it as a refusal: the payload is an
    // `estimate`, never a `consumed`. That distinction is the whole type.
    let TrippedGovernor::Refused {
        dimension,
        limit,
        estimate,
    } = exhausted.tripped
    else {
        panic!(
            "an admission decision must be reported as a refusal, not as a measured \
             ceiling crossing: {:?}",
            exhausted.tripped
        );
    };
    assert_eq!(dimension, ResourceDimension::IntermediateCells);
    assert_eq!(limit, 1);
    assert!(estimate > limit, "a refusal only fires above the ceiling");
    assert_eq!(
        exhausted.tripped.label(),
        "cardinality-admission-refused",
        "the label distinguishes a refusal from the same dimension's exhausted budget"
    );
    assert!(
        exhausted.tripped.to_string().contains("estimated"),
        "the prose must say ESTIMATED: {}",
        exhausted.tripped
    );

    // BEFORE evaluating. Every counter is engaged (METERED), so a single charge anywhere
    // would show up — and none did.
    for dimension in ResourceDimension::ALL {
        assert_eq!(
            exhausted.evidence.consumed_in(dimension),
            0,
            "a refused query consumed {dimension:?}, so something ran after all"
        );
    }
    // It never claims completeness, and what it hands back is the honest statement about a
    // query that did not run: a certified lower bound over nothing.
    assert!(!refused.is_complete());
    let certified = exhausted
        .partial
        .result()
        .expect("the empty lower bound is still a bound");
    assert_eq!(row_count(certified.result()), 0);
    assert!(certified.is_positional_prefix());

    // Deterministic: same query, same data, same ceiling — same decision, every time.
    for attempt in 1..8 {
        let again = engine
            .query_governed(
                &dataset,
                request(ALL_ROWS),
                QueryOptions::EMPTY,
                &QueryGovernors::METERED.with_max_intermediate_cells(1),
            )
            .expect("a refusal is an outcome");
        assert_eq!(
            again.tripped(),
            Some(exhausted.tripped),
            "attempt {attempt} reached a different admission decision"
        );
    }
    // …and monotone in the ceiling: raised above the estimate, the same plan is admitted
    // and completes. A refusal costs a caller an answer they could have had; it can never
    // hand them a wrong one.
    let admitted = engine
        .query_governed(
            &dataset,
            request(ALL_ROWS),
            QueryOptions::EMPTY,
            &QueryGovernors::METERED.with_max_intermediate_cells(estimate),
        )
        .expect("an admitted run");
    assert!(
        admitted.is_complete(),
        "the ceiling is inclusive at admission too: estimate == limit is admitted"
    );

    // Admission governs every query form. Graph-producing refusals retain the graph shape;
    // ASK withholds its empty materialization because `false` would be a settled negative
    // answer rather than a useful representation of an empty witness lower bound.
    for query in [
        "ASK { ?s <http://example.org/p> ?o }",
        "CONSTRUCT { ?s <http://example.org/q> ?o } \
         WHERE { ?s <http://example.org/p> ?o }",
        "DESCRIBE ?s WHERE { ?s <http://example.org/p> ?o }",
    ] {
        let outcome = engine
            .query_governed(
                &dataset,
                request(query),
                QueryOptions::EMPTY,
                &QueryGovernors::METERED.with_max_intermediate_cells(1),
            )
            .expect("a refusal is an outcome");
        let refused = outcome
            .exhausted()
            .unwrap_or_else(|| panic!("{query}: the same plan must be refused"));
        assert!(
            matches!(refused.tripped, TrippedGovernor::Refused { .. }),
            "{query}: {:?}",
            refused.tripped
        );
        match (query.starts_with("ASK"), &refused.partial) {
            (true, PartialAnswers::Unknown(barrier)) => {
                assert_eq!(barrier.operator(), "ask-unsettled");
            }
            (false, PartialAnswers::Certain(empty)) => {
                let SparqlResult::Graph(graph) = empty.result() else {
                    panic!("{query}: a refusal must retain its graph result arm");
                };
                assert_eq!(statement_count(graph), 0, "{query}: nothing ran");
            }
            (_, other) => panic!("{query}: unexpected refusal certificate: {other:?}"),
        }
    }

    // A REFUSAL and an OBSERVED TRIP in the same dimension must be distinguishable, or the
    // refusal would be free to masquerade as a measurement. A property path is the case
    // that separates them: it is not a basic graph pattern, so the cost model produces no
    // estimate for it — the plan is admitted, and the live cell meter then observes the
    // bag that really materialised and reports a `Budget` crossing carrying a `consumed`.
    let observed = engine
        .query_governed(
            &dataset,
            request("SELECT ?s ?o WHERE { ?s <http://example.org/p>+ ?o }"),
            QueryOptions::EMPTY,
            &QueryGovernors::METERED.with_max_intermediate_cells(1),
        )
        .expect("an observed trip is an outcome");
    let observed = observed
        .exhausted()
        .expect("the path's real bag exceeds a ceiling of one");
    let TrippedGovernor::Budget {
        dimension: observed_dimension,
        consumed,
        ..
    } = observed.tripped
    else {
        panic!(
            "a ceiling crossed by a MEASUREMENT must be reported as a budget, not as a \
             refusal: {:?}",
            observed.tripped
        );
    };
    assert_eq!(observed_dimension, ResourceDimension::IntermediateCells);
    assert!(consumed > 1);
    assert_eq!(observed.tripped.label(), "cardinality-exhausted");
    assert_ne!(
        observed.tripped.label(),
        exhausted.tripped.label(),
        "one dimension, two verdicts, two labels: a caller must be able to tell a plan \
         that was predicted to be too big from one that was measured to be"
    );
    assert!(
        observed.evidence.consumed_in(ResourceDimension::Fuel) > 0,
        "an observed trip ran; that is what makes it an observation"
    );
}

// ---------------------------------------------------------------------------
// EXPLAIN: the per-node ledger
// ---------------------------------------------------------------------------

#[test]
fn explain_reports_a_deterministic_per_node_ledger() {
    let engine = NativeSparqlEngine::new();
    let dataset = fixture();
    // A plan with several distinct node kinds, one of which is a two-pattern BGP so the
    // cost planner produces both a join order and a cardinality estimate.
    let query = "SELECT DISTINCT ?s ?n WHERE { \
                 ?s <http://example.org/p> ?o . ?s <http://example.org/n> ?n \
                 FILTER(?n != <http://example.org/absent>) } ORDER BY ?s";

    let first = engine
        .explain_query(&dataset, query, None)
        .expect("explain the query");
    // Byte-identical across runs. A frozen corpus pins this text, so any clock read, any
    // address, and any hash-map iteration in the rendering would surface here.
    let rendered = first.render();
    for attempt in 1..8 {
        let again = engine
            .explain_query(&dataset, query, None)
            .expect("explain the query again");
        assert_eq!(
            again.render(),
            rendered,
            "attempt {attempt} rendered a different explanation for the same query, data \
             and build"
        );
    }
    // A second engine, with a cold plan cache, renders the same bytes: the explanation is
    // a function of the query and the data, never of the engine's history.
    assert_eq!(
        NativeSparqlEngine::new()
            .explain_query(&dataset, query, None)
            .expect("explain from a cold engine")
            .render(),
        rendered,
        "a cold engine explained the same query differently"
    );

    // The profile the cost was priced under travels with it, because a cost without one is
    // not comparable across builds.
    let profile = first.profile();
    assert_eq!(profile.id, GOVERNOR_PROFILE_ID);
    assert_eq!(profile.version, GOVERNOR_PROFILE_VERSION);
    assert_eq!(profile.digest, *GOVERNOR_PROFILE_DIGEST);
    assert_eq!(profile.stop_poll_fuel, STOP_POLL_FUEL);
    assert!(rendered.contains(GOVERNOR_PROFILE_ID));
    assert!(rendered.contains(&format!("v{GOVERNOR_PROFILE_VERSION}")));
    assert!(rendered.contains(&*GOVERNOR_PROFILE_DIGEST));
    assert!(rendered.contains(&STOP_POLL_FUEL.to_string()));
    // The schedule itself is rendered, so a reader can price the ledger's fuel column
    // without knowing this build.
    for (label, _) in CHARGE_SCHEDULE {
        assert!(
            rendered.contains(label),
            "the rendered schedule omits the charge point {label}"
        );
    }

    // The ledger is per node, in the plan's pre-order, with no gaps and no repeats.
    let ledger = first.ledger();
    assert!(ledger.len() >= 4, "expected a multi-node plan: {ledger:?}");
    for (position, node) in ledger.iter().enumerate() {
        assert_eq!(node.ordinal, position, "ordinals are positional");
        assert!(!node.label.is_empty());
    }
    assert_eq!(ledger[0].depth, 0, "the first node is the plan root");

    // The ledger DECOMPOSES the evidence: its fuel column sums to exactly the total the
    // evidence reports. A decomposition that did not add up would be a decomposition of
    // some other number.
    let ledger_fuel: u64 = ledger
        .iter()
        .map(purrdf_sparql_eval::NodeCharges::fuel_total)
        .sum();
    assert_eq!(
        ledger_fuel,
        first.evidence().consumed_in(ResourceDimension::Fuel),
        "the per-node fuel column must sum to the execution's fuel total"
    );
    assert!(ledger_fuel > 0, "the explained run must actually have run");

    // Estimated versus actual, wherever the cost planner produced a number. That pairing is
    // the only thing that makes the estimator's error observable at all.
    let estimated: Vec<_> = ledger
        .iter()
        .filter_map(|node| node.estimate.as_ref().map(|estimate| (node, estimate)))
        .collect();
    assert!(
        !estimated.is_empty(),
        "a two-pattern BGP must carry the planner's prediction: {ledger:?}"
    );
    for (node, estimate) in &estimated {
        assert!(estimate.columns > 0, "a BGP with variables has columns");
        assert!(
            estimate.peak_rows >= estimate.rows,
            "the peak of a running estimate is at least its last value"
        );
        assert_eq!(
            estimate.peak_cells(),
            estimate.peak_rows.saturating_mul(estimate.columns)
        );
        assert!(
            rendered.contains(&format!(
                "estimated-rows={} actual-rows={}",
                estimate.rows, node.rows
            )),
            "the rendering must print the prediction beside what materialised"
        );
    }

    // The join orders the API returned before the ledger existed are still there, unmoved.
    assert_eq!(
        first.join_orders().len(),
        2,
        "a two-pattern BGP contributes two ordered pattern strings: {:?}",
        first.join_orders()
    );

    // Explaining does not bound the query: `METERED` reaches no ceiling, so the run the
    // explanation describes is the run a caller would get.
    assert!(first.evidence().is_complete(), "{:?}", first.evidence());
}

// ---------------------------------------------------------------------------
// The cap over a graph-producing form
// ---------------------------------------------------------------------------

/// The graph a query returns under `governors`, and whether it completed.
fn graph_outcome(query: &str, governors: &QueryGovernors) -> GovernedOutcome {
    NativeSparqlEngine::new()
        .query_governed(&fixture(), request(query), QueryOptions::EMPTY, governors)
        .expect("a tripped governor is an outcome, not a query error")
}

/// Assert the inclusive cap boundary over a graph-producing `query`: a cap equal to the
/// answer's statement count completes, and one below it trips with the graph truncated to
/// exactly the cap.
///
/// The denomination under test is **output statements**, which is what the cap counts for a
/// form whose answer *is* a graph — see `construct::commit_answer_triples`. Solution rows
/// would be the wrong meter: one `CONSTRUCT` row can instantiate a whole template, and a
/// `DESCRIBE` of one bound subject can pull in that subject's entire description.
fn assert_graph_cap_boundary(query: &str) -> usize {
    let complete = graph_outcome(query, &QueryGovernors::METERED);
    let GovernedOutcome::Complete { result, .. } = &complete else {
        panic!("METERED bounds nothing: {complete:?}");
    };
    let size = statement_count(graph_of(result));
    assert!(size > 1, "{query}: the fixture must produce a real graph");

    // cap == size: complete. Ceilings are inclusive everywhere in this engine.
    let at_the_cap = graph_outcome(
        query,
        &QueryGovernors::UNBOUNDED.with_max_answers(size as u64),
    );
    let GovernedOutcome::Complete {
        result, evidence, ..
    } = &at_the_cap
    else {
        panic!("{query}: cap == size must be complete: {at_the_cap:?}");
    };
    assert_eq!(statement_count(graph_of(result)), size);
    assert!(evidence.is_complete());
    assert_eq!(
        evidence.consumed_in(ResourceDimension::AnswerRows),
        size as u64,
        "{query}: the cap is charged once per output statement"
    );

    // cap == size - 1: exhausted, at the same boundary a SELECT trips at.
    let one_below = graph_outcome(
        query,
        &QueryGovernors::UNBOUNDED.with_max_answers(size as u64 - 1),
    );
    let (tripped, partial, _) = exhausted(&one_below);
    assert_eq!(
        tripped,
        TrippedGovernor::Budget {
            dimension: ResourceDimension::AnswerRows,
            limit: size as u64 - 1,
            consumed: size as u64,
        },
        "{query}: cap == size - 1 trips on the statement that would have exceeded it"
    );
    // `into_result` twice: the caller who wants the truncated graph itself, taking it out
    // of the certificate rather than borrowing through it.
    let certified = partial
        .clone()
        .into_result()
        .unwrap_or_else(|| panic!("{query}: the admitted prefix is certified"));
    assert!(partial.is_certain());
    assert!(
        certified.is_positional_prefix(),
        "{query}: the truncated graph is the complete graph's first statements, in the \
         frozen canonical order, so raising the cap returns these same statements first"
    );
    let truncated = certified.into_result();
    assert_eq!(
        statement_count(graph_of(&truncated)),
        size - 1,
        "{query}: the cap admits exactly its own count of statements"
    );
    size
}

#[test]
fn the_answer_cap_governs_construct_output_triples_at_the_same_inclusive_boundary() {
    // A one-row-to-one-triple template, so the boundary is legible: `EDGES` solutions
    // become `EDGES` statements.
    let flat = assert_graph_cap_boundary(
        "CONSTRUCT { ?s <http://example.org/q> ?o } WHERE { ?s <http://example.org/p> ?o }",
    );
    assert_eq!(flat, EDGES);

    // The reason rows are the wrong denomination, made concrete: the same `EDGES`
    // solutions through a three-triple template are three times the answer. A cap that
    // counted rows would let this query return 3× what the caller asked for.
    let fanned = assert_graph_cap_boundary(
        "CONSTRUCT { ?s <http://example.org/q> ?o . \
                     ?o <http://example.org/r> ?s . \
                     ?s <http://example.org/t> ?s } \
         WHERE { ?s <http://example.org/p> ?o }",
    );
    assert_eq!(
        fanned,
        EDGES * 3,
        "the cap must see the template's fan-out, not the WHERE's row count"
    );

    // RDF 1.2: a reifier binding is a statement the caller receives, and it lives in a
    // side table rather than in `quads` — so a cap that only counted quads would be a
    // governor an RDF 1.2 CONSTRUCT could be written straight around. Here the output has
    // NO quads at all and the cap still governs it.
    let reified = assert_graph_cap_boundary(
        "CONSTRUCT { ?r <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> \
                     <<( ?s <http://example.org/p> ?o )>> } \
         WHERE { ?s <http://example.org/p> ?o . BIND(?s AS ?r) }",
    );
    assert_eq!(reified, EDGES);
    let graph = graph_outcome(
        "CONSTRUCT { ?r <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> \
                     <<( ?s <http://example.org/p> ?o )>> } \
         WHERE { ?s <http://example.org/p> ?o . BIND(?s AS ?r) }",
        &QueryGovernors::METERED,
    );
    let GovernedOutcome::Complete { result, .. } = &graph else {
        panic!("METERED bounds nothing: {graph:?}");
    };
    assert_eq!(
        graph_of(result).quad_count(),
        0,
        "this template emits only reifier bindings"
    );
    assert_eq!(graph_of(result).reifiers().count(), EDGES);
}

#[test]
fn the_answer_cap_governs_describe_output_triples_at_the_same_inclusive_boundary() {
    // `DESCRIBE ?s` over every subject: the cap denominates the description's statements,
    // exactly as it does a `CONSTRUCT`'s. It matters more here — a `DESCRIBE` whose WHERE
    // bound a single row can still return that subject's entire concise bounded
    // description, so a row-denominated cap would bound nothing at all.
    let described = assert_graph_cap_boundary("DESCRIBE ?s WHERE { ?s <http://example.org/p> ?o }");
    assert_eq!(described, EDGES);

    // And with no pattern to evaluate at all: `DESCRIBE <iri>` charges the cap against the
    // description it built, which is the only work such a query does.
    let concrete = graph_outcome(
        "DESCRIBE <http://example.org/s0> <http://example.org/s1> <http://example.org/s2>",
        &QueryGovernors::METERED,
    );
    let GovernedOutcome::Complete { result, .. } = &concrete else {
        panic!("METERED bounds nothing: {concrete:?}");
    };
    let size = statement_count(graph_of(result));
    assert_eq!(size, 3, "three subjects, one statement each");
    assert!(
        graph_outcome(
            "DESCRIBE <http://example.org/s0> <http://example.org/s1> <http://example.org/s2>",
            &QueryGovernors::UNBOUNDED.with_max_answers(3),
        )
        .is_complete()
    );
    let one_below = graph_outcome(
        "DESCRIBE <http://example.org/s0> <http://example.org/s1> <http://example.org/s2>",
        &QueryGovernors::UNBOUNDED.with_max_answers(2),
    );
    let (tripped, partial, _) = exhausted(&one_below);
    assert_eq!(
        tripped,
        TrippedGovernor::Budget {
            dimension: ResourceDimension::AnswerRows,
            limit: 2,
            consumed: 3,
        }
    );
    let certified = partial.result().expect("the admitted prefix is certified");
    assert_eq!(statement_count(graph_of(certified.result())), 2);
}
