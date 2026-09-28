// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The per-row checkpoint ([`crate::row_checkpoint`]) against the claims it makes.
//!
//! 1. A forked `FILTER` or `BIND` row loop and the same loop evaluated sequentially keep the
//!    same rows, spend the same fuel and report the same trip under every fuel budget —
//!    so the forked trip lands at the sequential row. The sequential twin of each query is
//!    the same query with a `RAND()` conjunct that is always true: `RAND()` charges
//!    nothing and is not fork-safe, so it holds the loop on the evaluation's own context
//!    and changes nothing else. `UNFOLD` and aggregate accumulation never fork; their
//!    loops are held to the same claim across forced-parallel and forced-sequential
//!    evaluation of everything around them.
//! 2. An already-latched trip is observed before the next row, on the forked path as on
//!    the sequential one.
//! 3. Each unit of work is reported to the stop signal once, and the fuel a loop spends is
//!    its admitted rows' — plus the one refused admission when a ceiling trips.
//! 4. `UNFOLD` admits input row `i` and then ingests what it expands to before admitting
//!    row `i + 1`, which the per-node ledger shows when the ingest trips.
//!
//! In-crate rather than under `tests/` because [`crate::parallel::force_parallel_for_test`]
//! and the checkpoint itself are crate-private.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use pretty_assertions::assert_eq;
use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, RdfLiteral, ResourceDimension, StopCause, TermValue,
    TrippedGovernor,
};
use purrdf_sparql_algebra::{GraphPattern, Query, SparqlParser};

use crate::eval::{EvalCtx, eval_evaluated};
use crate::governor::{ChargePoint, GovernorState, QueryGovernors, STOP_POLL_FUEL, StopSignal};
use crate::parallel::{PARALLEL_MIN_ROWS, force_parallel_for_test};
use crate::row_checkpoint::RowCheckpoint;

/// The fixture namespace.
const EX: &str = "http://example.org/";

/// The SEP-0009 composite-list datatype, as the spec spells it.
const CDT_LIST: &str = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/List";

/// Rows each fixture drives through the loop under test: above the parallel threshold, so
/// a forced-parallel run really splits the loop into several chunks.
const ROWS: usize = 1_500;

/// What one evaluation kept and spent.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Run {
    /// The rows the root held, by value and in order.
    rows: Vec<Vec<Option<TermValue>>>,
    /// Fuel charged.
    fuel: u64,
    /// The governor that stopped the run, if one did.
    tripped: Option<TrippedGovernor>,
}

/// `ex:s{i} ex:v i` and `ex:s{i} ex:l "[i,i+1]"^^cdt:List` for every `i < rows`.
fn dataset(rows: usize) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let v = builder.intern_iri(&format!("{EX}v"));
    let l = builder.intern_iri(&format!("{EX}l"));
    for index in 0..rows {
        let s = builder.intern_iri(&format!("{EX}s{index}"));
        let value = builder.intern_literal(RdfLiteral::typed(
            index.to_string(),
            "http://www.w3.org/2001/XMLSchema#integer",
        ));
        let list = builder.intern_literal(RdfLiteral::typed(
            format!("[{index},{}]", index + 1),
            CDT_LIST,
        ));
        builder.push_quad(s, v, value, None);
        builder.push_quad(s, l, list, None);
    }
    builder.freeze().expect("the fixture is positionally valid")
}

/// The algebra of a `SELECT` query written against the fixture namespace.
fn select(body: &str) -> GraphPattern {
    let text = format!(
        "PREFIX ex: <{EX}> PREFIX cdt: <http://w3id.org/awslabs/neptune/SPARQL-CDTs/> {body}"
    );
    match SparqlParser::new()
        .parse_query(&text)
        .unwrap_or_else(|error| panic!("`{text}` parses: {error:?}"))
    {
        Query::Select { pattern, .. } => pattern,
        other => panic!("`{text}` is a SELECT, got {other:?}"),
    }
}

/// Evaluate `pattern` under `governors`, parallel paths forced on or off.
fn run(
    pattern: &GraphPattern,
    dataset: &RdfDataset,
    governors: &QueryGovernors,
    parallel: bool,
) -> Run {
    let _guard = force_parallel_for_test(parallel);
    let state = Arc::new(GovernorState::new(governors));
    let mut ctx = EvalCtx::new(dataset).with_governors(Arc::clone(&state));
    let evaluated = eval_evaluated(pattern, &mut ctx).expect("evaluation must not fail");
    let rows = evaluated
        .rows()
        .rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|cell| cell.map(|term| ctx.scratch.value_of(ctx.dataset, term)))
                .collect()
        })
        .collect();
    let evidence = state.evidence();
    Run {
        rows,
        fuel: evidence.consumed_in(ResourceDimension::Fuel),
        tripped: evidence.tripped(),
    }
}

/// The fuel `pattern` costs when nothing refuses it.
fn full_cost(pattern: &GraphPattern, dataset: &RdfDataset) -> u64 {
    let measured = run(pattern, dataset, &QueryGovernors::METERED, false);
    assert_eq!(measured.tripped, None, "the measuring run must complete");
    measured.fuel
}

/// Every budget the sweep tries: dense at both ends, a fixed stride in between, so the
/// ceilings that fall inside the row loop are exercised at many rows.
fn budgets(full: u64) -> Vec<u64> {
    let mut budgets: Vec<u64> = (0..=full).step_by(11).collect();
    budgets.extend(0..=full.min(40));
    budgets.extend(full.saturating_sub(40)..=full + 1);
    budgets.sort_unstable();
    budgets.dedup();
    budgets
}

/// Sweep every budget over `a` and `b`, which must evaluate to the same thing, asserting
/// that they do; `a` is additionally run under both forced-parallel and forced-sequential
/// evaluation. Returns how many budgets truncated inside the row loop — a strict,
/// non-empty prefix of the full answer — so a caller can prove the sweep was not vacuous.
fn assert_same_under_every_budget(
    a: &GraphPattern,
    b: &GraphPattern,
    dataset: &RdfDataset,
) -> usize {
    let full = full_cost(a, dataset);
    assert_eq!(
        full,
        full_cost(b, dataset),
        "the two forms must cost the same fuel when nothing refuses them"
    );
    let complete = run(a, dataset, &QueryGovernors::METERED, true);
    assert_eq!(
        complete,
        run(b, dataset, &QueryGovernors::METERED, true),
        "the two forms must answer the same when nothing refuses them"
    );
    let mut inside = 0;
    for budget in budgets(full) {
        let governors = QueryGovernors::UNBOUNDED.with_fuel(budget);
        let forked = run(a, dataset, &governors, true);
        let single_chunk = run(a, dataset, &governors, false);
        let sequential = run(b, dataset, &governors, true);
        assert_eq!(
            forked, sequential,
            "budget {budget}: the forked loop kept, spent or tripped differently"
        );
        assert_eq!(
            forked, single_chunk,
            "budget {budget}: the trip point moved with the chunk count"
        );
        if budget >= full {
            assert_eq!(
                forked.tripped, None,
                "budget {budget} admits the whole query"
            );
            assert_eq!(
                forked.rows, complete.rows,
                "budget {budget}: the full answer"
            );
        }
        if forked.tripped.is_some()
            && !forked.rows.is_empty()
            && forked.rows.len() < complete.rows.len()
        {
            inside += 1;
        }
    }
    inside
}

// The fixture really crosses the parallel threshold.
const _: () = assert!(ROWS > PARALLEL_MIN_ROWS);

#[test]
fn a_forked_filter_trips_at_the_sequential_row_under_every_fuel_budget() {
    let dataset = dataset(ROWS);
    let forked = select("SELECT ?s ?v WHERE { ?s ex:v ?v FILTER(?v > 10) }");
    let sequential = select("SELECT ?s ?v WHERE { ?s ex:v ?v FILTER(?v > 10 && RAND() < 2) }");
    let inside = assert_same_under_every_budget(&forked, &sequential, &dataset);
    assert!(inside > 0, "no budget truncated the FILTER loop part-way");
}

#[test]
fn a_forked_bind_trips_at_the_sequential_row_under_every_fuel_budget() {
    let dataset = dataset(ROWS);
    let forked = select("SELECT ?s ?w WHERE { ?s ex:v ?v BIND(?v + 1 AS ?w) }");
    let sequential =
        select("SELECT ?s ?w WHERE { ?s ex:v ?v BIND(IF(RAND() < 2, ?v + 1, 0) AS ?w) }");
    let inside = assert_same_under_every_budget(&forked, &sequential, &dataset);
    assert!(inside > 0, "no budget truncated the BIND loop part-way");
}

#[test]
fn an_unfold_trips_at_the_same_row_under_every_fuel_budget() {
    let dataset = dataset(ROWS);
    let pattern = select("SELECT ?s ?e WHERE { ?s ex:l ?l UNFOLD(?l AS ?e) }");
    let inside = assert_same_under_every_budget(&pattern, &pattern, &dataset);
    assert!(inside > 0, "no budget truncated the UNFOLD loop part-way");
}

#[test]
fn aggregate_accumulation_trips_at_the_same_row_under_every_fuel_budget() {
    let dataset = dataset(ROWS);
    let pattern = select("SELECT (COUNT(?v) AS ?n) (SUM(?v) AS ?t) WHERE { ?s ex:v ?v }");
    let full = full_cost(&pattern, &dataset);
    let complete = run(&pattern, &dataset, &QueryGovernors::METERED, true);
    assert_eq!(complete.rows.len(), 1, "one group");
    // Accumulation charges one unit per value per aggregate, and far fewer than half a
    // group's rows are charged after it, so this budget runs out inside the loop.
    let inside = full - u64::try_from(ROWS / 2).expect("fits");
    let mut sweep = budgets(full);
    sweep.push(inside);
    for budget in sweep {
        let governors = QueryGovernors::UNBOUNDED.with_fuel(budget);
        let parallel = run(&pattern, &dataset, &governors, true);
        let sequential = run(&pattern, &dataset, &governors, false);
        assert_eq!(parallel, sequential, "budget {budget}");
        if budget >= full {
            assert_eq!(parallel, complete, "budget {budget}: the full answer");
        }
        if budget == inside {
            assert!(
                parallel.tripped.is_some(),
                "budget {budget} runs out inside the accumulation loop"
            );
        }
    }
}

/// A signal that records every poll and the work each one reported, and fires — then
/// stays fired — once `fire` is set.
#[derive(Debug, Default)]
struct WorkLog {
    polls: AtomicU64,
    work: AtomicU64,
    fire: AtomicBool,
}

impl StopSignal for WorkLog {
    fn poll(&self) -> Option<StopCause> {
        self.poll_after_work(1)
    }

    fn poll_after_work(&self, work: u64) -> Option<StopCause> {
        assert!(work >= 1, "every checkpoint stands for at least one unit");
        self.polls.fetch_add(1, Ordering::Relaxed);
        self.work.fetch_add(work, Ordering::Relaxed);
        self.fire
            .load(Ordering::Relaxed)
            .then_some(StopCause::Cancelled)
    }
}

/// A governor state with a fuel ceiling of `fuel` and `log` as its signal.
fn state_with(fuel: u64, log: &Arc<WorkLog>) -> Arc<GovernorState> {
    Arc::new(GovernorState::new(
        &QueryGovernors::UNBOUNDED
            .with_fuel(fuel)
            .with_stop_signal(Arc::clone(log) as Arc<dyn StopSignal>),
    ))
}

/// Pass `checkpoint` once per row of `rows`, keeping every admitted row, the way one
/// forked worker does; returns the rows it kept.
fn worker(
    checkpoint: &mut RowCheckpoint,
    ctx: &EvalCtx<'_, RdfDataset>,
    rows: std::ops::Range<usize>,
) -> Vec<usize> {
    let mut kept = Vec::new();
    for row in rows {
        if checkpoint.pass(ctx).is_err() {
            continue;
        }
        kept.push(row);
        checkpoint.keep();
    }
    kept
}

#[test]
fn a_latched_trip_is_observed_before_the_next_row_on_the_forked_path() {
    let dataset = dataset(0);
    let log = Arc::new(WorkLog::default());
    let state = state_with(u64::MAX / 2, &log);
    let ctx = EvalCtx::new(dataset.as_ref()).with_governors(Arc::clone(&state));
    let point = ChargePoint::RowExpressionEvaluation;
    let template = RowCheckpoint::for_rows(&ctx, point, true, 10);
    assert_eq!(template.forked_len(10), 10);

    let mut first = template.clone();
    let mut kept = worker(&mut first, &ctx, 0..3);
    // Ten rows are far from a stride poll ([`STOP_POLL_FUEL`] rows under this fuel
    // ceiling), so only the latched-trip check can stop the next row.
    let latched = state.record_trip(TrippedGovernor::Stopped {
        cause: StopCause::Cancelled,
    });
    assert_eq!(
        first.pass(&ctx),
        Err(latched),
        "the next row observes the trip"
    );
    let mut second = template.clone();
    kept.extend(worker(&mut second, &ctx, 4..10));
    assert_eq!(kept, vec![0, 1, 2]);

    template.commit(&ctx, &mut kept, [first, second]);
    assert_eq!(
        kept,
        vec![0, 1, 2],
        "the prefix admitted before the trip survives"
    );
    assert_eq!(
        state.consumed_in(ResourceDimension::Fuel),
        3 * point.cost(),
        "the rows admitted before the trip are paid for, and nothing after it"
    );
}

#[test]
fn a_latched_trip_is_observed_before_the_next_row_on_the_sequential_path() {
    let dataset = dataset(0);
    let log = Arc::new(WorkLog::default());
    let state = state_with(u64::MAX / 2, &log);
    let ctx = EvalCtx::new(dataset.as_ref()).with_governors(Arc::clone(&state));
    let point = ChargePoint::RowExpressionEvaluation;
    let mut checkpoint = RowCheckpoint::for_rows(&ctx, point, false, 10);
    for _ in 0..3 {
        assert_eq!(checkpoint.pass(&ctx), Ok(()));
    }
    let latched = state.record_trip(TrippedGovernor::Stopped {
        cause: StopCause::Cancelled,
    });
    assert_eq!(checkpoint.pass(&ctx), Err(latched));
    assert_eq!(state.consumed_in(ResourceDimension::Fuel), 3 * point.cost());
}

#[test]
fn an_unlatched_forked_worker_admits_every_row() {
    // The neighbour of the latched case: nothing latched, nothing fired, every row runs.
    let dataset = dataset(0);
    let log = Arc::new(WorkLog::default());
    let state = state_with(u64::MAX / 2, &log);
    let ctx = EvalCtx::new(dataset.as_ref()).with_governors(Arc::clone(&state));
    let template = RowCheckpoint::for_rows(&ctx, ChargePoint::RowExpressionEvaluation, true, 10);
    let mut only = template.clone();
    let mut kept = worker(&mut only, &ctx, 0..10);
    template.commit(&ctx, &mut kept, [only]);
    assert_eq!(kept, (0..10).collect::<Vec<_>>());
    assert_eq!(state.tripped(), None);
}

/// Rows chosen so neither loop's work is a whole number of poll intervals: every unit is
/// then either reported by a poll inside the loop or still pending after it.
fn rows_for_work_test() -> usize {
    let stride = usize::try_from(STOP_POLL_FUEL / ChargePoint::RowExpressionEvaluation.cost())
        .expect("fits");
    2 * stride + 5
}

#[test]
fn a_forked_loop_reports_each_unit_of_work_once_and_spends_its_admitted_rows() {
    let dataset = dataset(0);
    let log = Arc::new(WorkLog::default());
    let state = state_with(u64::MAX / 2, &log);
    let ctx = EvalCtx::new(dataset.as_ref()).with_governors(Arc::clone(&state));
    let point = ChargePoint::RowExpressionEvaluation;
    let rows = rows_for_work_test();
    let template = RowCheckpoint::for_rows(&ctx, point, true, rows);

    // Two workers splitting the rows unevenly, as chunks do.
    let split = rows / 3;
    let mut first = template.clone();
    let mut second = template.clone();
    let mut kept = worker(&mut first, &ctx, 0..split);
    kept.extend(worker(&mut second, &ctx, split..rows));
    template.commit(&ctx, &mut kept, [first, second]);

    let admitted = u64::try_from(rows).expect("fits") * point.cost();
    assert_eq!(kept.len(), rows);
    assert_eq!(state.consumed_in(ResourceDimension::Fuel), admitted);
    // The rows passed after each worker's last poll are pending; one more poll drains them.
    let polls_before = log.polls.load(Ordering::Relaxed);
    assert_eq!(state.poll_stop(), None);
    assert_eq!(log.polls.load(Ordering::Relaxed), polls_before + 1);
    assert_eq!(
        log.work.load(Ordering::Relaxed),
        admitted,
        "every admitted row's work reached the signal exactly once"
    );
}

#[test]
fn a_sequential_loop_reports_each_unit_of_work_once_and_spends_its_admitted_rows() {
    let dataset = dataset(0);
    let log = Arc::new(WorkLog::default());
    let state = state_with(u64::MAX / 2, &log);
    let ctx = EvalCtx::new(dataset.as_ref()).with_governors(Arc::clone(&state));
    let point = ChargePoint::RowExpressionEvaluation;
    let rows = rows_for_work_test();
    let mut checkpoint = RowCheckpoint::for_rows(&ctx, point, false, rows);
    for _ in 0..rows {
        assert_eq!(checkpoint.pass(&ctx), Ok(()));
    }
    let admitted = u64::try_from(rows).expect("fits") * point.cost();
    assert_eq!(state.consumed_in(ResourceDimension::Fuel), admitted);
    assert_eq!(state.poll_stop(), None);
    assert_eq!(log.work.load(Ordering::Relaxed), admitted);
}

#[test]
fn forked_and_sequential_loops_spend_the_same_fuel_and_keep_the_same_rows_at_a_trip() {
    let dataset = dataset(0);
    let point = ChargePoint::RowExpressionEvaluation;
    let rows = 40;
    for ceiling in [0_u64, 1, 9, 10, 39, 40, 41] {
        // Sequential: charge each row until one is refused.
        let log = Arc::new(WorkLog::default());
        let state = state_with(ceiling, &log);
        let ctx = EvalCtx::new(dataset.as_ref()).with_governors(Arc::clone(&state));
        let mut checkpoint = RowCheckpoint::for_rows(&ctx, point, false, rows);
        let mut sequential_kept = Vec::new();
        for row in 0..rows {
            if checkpoint.pass(&ctx).is_err() {
                break;
            }
            sequential_kept.push(row);
        }
        let sequential = (
            sequential_kept,
            state.consumed_in(ResourceDimension::Fuel),
            state.tripped(),
        );

        // Forked: fork only what the ceiling admits, commit in order after the join.
        let log = Arc::new(WorkLog::default());
        let state = state_with(ceiling, &log);
        let ctx = EvalCtx::new(dataset.as_ref()).with_governors(Arc::clone(&state));
        let template = RowCheckpoint::for_rows(&ctx, point, true, rows);
        let forked_len = template.forked_len(rows);
        let mut first = template.clone();
        let mut second = template.clone();
        let mut kept = worker(&mut first, &ctx, 0..forked_len / 2);
        kept.extend(worker(&mut second, &ctx, forked_len / 2..forked_len));
        template.commit(&ctx, &mut kept, [first, second]);
        let forked = (
            kept,
            state.consumed_in(ResourceDimension::Fuel),
            state.tripped(),
        );

        assert_eq!(forked, sequential, "ceiling {ceiling}");
        let expected_rows = usize::try_from(ceiling).expect("fits").min(rows);
        assert_eq!(
            forked.0.len(),
            expected_rows,
            "ceiling {ceiling}: fuel spent equals rows admitted"
        );
        let refused = u64::from(expected_rows < rows);
        assert_eq!(
            forked.1,
            (u64::try_from(expected_rows).expect("fits") + refused) * point.cost(),
            "ceiling {ceiling}: the admitted rows plus the one refused admission"
        );
    }
}

#[test]
fn unfold_admits_each_input_row_then_ingests_it() {
    // Ten input rows, each expanding to two: with an intermediate-cell ceiling of 21 over
    // UNFOLD's three columns, its bag holds seven rows, so the ingest trips at the eighth —
    // the second element of input row 3. Admission(i) then ingest(i) has admitted input
    // rows 0 to 3 by then; admitting every row ahead of the ingest would have admitted
    // all ten.
    let dataset = dataset(10);
    let pattern = select("SELECT ?s ?e WHERE { ?s ex:l ?l UNFOLD(?l AS ?e) }");
    let admissions = |governors: &QueryGovernors| {
        let state = Arc::new(GovernorState::new(governors));
        let ledger = Arc::new(crate::governor::ledger::ChargeLedger::for_plan(
            &pattern,
            &crate::DetHashMap::default(),
        ));
        let mut ctx = EvalCtx::new(dataset.as_ref())
            .with_governors(Arc::clone(&state))
            .with_charge_ledger(Arc::clone(&ledger));
        let evaluated = eval_evaluated(&pattern, &mut ctx).expect("evaluation must not fail");
        let fuel: u64 = ledger
            .snapshot()
            .iter()
            .map(|node| node.fuel_at(ChargePoint::RowExpressionEvaluation))
            .sum();
        (evaluated.rows().rows.len(), fuel, state.tripped())
    };

    let (rows, fuel, tripped) =
        admissions(&QueryGovernors::METERED.with_max_intermediate_cells(21));
    assert!(
        matches!(
            tripped,
            Some(TrippedGovernor::Budget {
                dimension: ResourceDimension::IntermediateCells,
                ..
            })
        ),
        "the ingest trips the cell ceiling: {tripped:?}"
    );
    assert_eq!(rows, 7, "the bag the ceiling admits");
    assert_eq!(
        fuel,
        4 * ChargePoint::RowExpressionEvaluation.cost(),
        "input rows 0 to 3 were admitted, and no row after the one whose ingest tripped"
    );

    // The neighbour: without the cell ceiling every input row is admitted and expanded.
    let (rows, fuel, tripped) = admissions(&QueryGovernors::METERED);
    assert_eq!(tripped, None);
    assert_eq!(rows, 20);
    assert_eq!(fuel, 10 * ChargePoint::RowExpressionEvaluation.cost());
}

#[test]
fn a_forked_loop_with_nothing_to_charge_keeps_every_row() {
    // The neighbour of every refusal above: with no governor state at all, and with one
    // that carries only a cell ceiling (nothing to charge, no signal to poll), a forked
    // `FILTER` and `BIND` keep every row they would keep under a metered run.
    let dataset = dataset(ROWS);
    let filter = select("SELECT ?s ?v WHERE { ?s ex:v ?v FILTER(?v > 10) }");
    let bind = select("SELECT ?s ?w WHERE { ?s ex:v ?v BIND(?v + 1 AS ?w) }");
    for (pattern, expected) in [(&filter, ROWS - 11), (&bind, ROWS)] {
        let metered = run(pattern, &dataset, &QueryGovernors::METERED, true);
        assert_eq!(metered.rows.len(), expected);
        for parallel in [true, false] {
            let ungoverned = {
                let _guard = force_parallel_for_test(parallel);
                let mut ctx = EvalCtx::new(dataset.as_ref());
                eval_evaluated(pattern, &mut ctx)
                    .expect("evaluation must not fail")
                    .rows()
                    .rows
                    .len()
            };
            assert_eq!(ungoverned, expected, "ungoverned, parallel {parallel}");
            let cell_bounded = run(
                pattern,
                &dataset,
                &QueryGovernors::UNBOUNDED.with_max_intermediate_cells(u64::MAX / 2),
                parallel,
            );
            assert_eq!(cell_bounded.tripped, None);
            assert_eq!(
                cell_bounded.rows, metered.rows,
                "cell ceiling only, parallel {parallel}"
            );
        }
    }
}
