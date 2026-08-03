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
use std::time::Duration;

use purrdf_core::{
    GovernorEvidence, InMemoryPageProvider, PagedDataset, PagedQueryLimits, RdfDataset,
    RdfDatasetBuilder, ResourceDimension, SparqlRequest, SparqlResult, StopCause, TrippedGovernor,
};
use purrdf_sparql_eval::{
    CancellationFlag, FallibleSparqlError, GovernedOutcome, NativeSparqlEngine, PartialAnswers,
    QueryGovernors, StopSignal, WallDeadline, resolve_precedence,
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

fn request(query: &str) -> SparqlRequest<'_> {
    SparqlRequest {
        query,
        base_iri: None,
        substitutions: &[],
    }
}

/// The all-rows SELECT the ceiling tests bound.
const ALL_ROWS: &str = "SELECT ?s ?o WHERE { ?s <http://example.org/p> ?o }";

/// Run `query` over the fixture under `governors`, asserting only that it is not an
/// error — a trip is an outcome, never an `Err`.
fn governed(query: &str, governors: &QueryGovernors) -> GovernedOutcome {
    NativeSparqlEngine::new()
        .query_governed(&fixture(), request(query), governors)
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
    let GovernedOutcome::Complete { result, evidence } = &outcome else {
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
    // The intermediate-cell ceiling cannot: it bounds how large one operator's bag may
    // GET, so it is a peak observation made once the bag exists, and the rows it leaves in
    // hand are the ones already computed. Reporting the whole answer as a certified lower
    // bound is sound — a lower bound is allowed to be tight — and it is still not
    // completion, because the query was stopped rather than finished.
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
            TrippedGovernor::Budget {
                dimension: ResourceDimension::IntermediateCells,
                limit: 0,
                consumed: (EDGES * 2) as u64,
            },
            false,
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
fn ac2_ask_with_a_nonempty_certain_set_is_complete_true_despite_a_trip() {
    // `ASK` asks whether ANY solution exists, so one row that is certainly an answer
    // settles it outright — whichever governor stopped the search. Reporting "budget
    // exhausted" for a question the evaluator has already answered would throw away a
    // complete result, so the governed surface must not degrade it.
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
        if let GovernedOutcome::Complete { result, .. } = &outcome {
            assert!(
                matches!(result, SparqlResult::Boolean(true)),
                "a search that could not finish may only answer true, never false: {result:?}"
            );
            settled_early += 1;
        } else {
            // The other outcome is honest exhaustion, never a fabricated `false`
            // presented as complete.
            let (_, partial, _) = exhausted(&outcome);
            assert!(partial.result().is_some() || partial.barrier().is_some());
        }
    }
    assert!(
        settled_early > 0,
        "a certain row must be able to settle ASK before the budget does"
    );
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
    let GovernedOutcome::Complete { result, evidence } = &at_the_cap else {
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
    let GovernedOutcome::Complete { result, evidence } = &outcome else {
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
    for (winner, loser) in [
        (cancelled, deadline),
        (deadline, cells),
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

    // Ceilings that are crossed at DIFFERENT charge points report the one crossed first —
    // precedence is evaluated over the conditions already true at a charge point, never
    // over conditions that might become true later. Fuel is charged on entering the first
    // algebra node, before any bag exists to measure or any answer row exists to admit,
    // so it is the reported trip even though it ranks below the cell ceiling.
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
            .query_prepared_governed_view(&*dataset, &prepared, &[], &governors)
            .expect("prepared governed run");
        let GovernedOutcome::Complete { result, evidence } = &outcome else {
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
        .query_governed_fallible_view(&view, request(ALL_ROWS), &QueryGovernors::METERED)
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
