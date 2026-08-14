// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! End-to-end coverage of the charge schedule as the evaluator actually spends it.
//!
//! [`super`]'s own tests pin the accounting primitives in isolation. These drive real
//! queries over real datasets, which is the only way to check the two properties that
//! only exist once charging is wired to evaluation: that a governed **parallel** run
//! trips at exactly the point a governed sequential run does, and that the ceilings which
//! are not simple counters — the cell peak, the scratch arena, the recursion guard —
//! measure what they claim to.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use pretty_assertions::assert_eq;
use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, RdfLiteral, ResourceDimension, StopCause, TermId, TermValue,
    TrippedGovernor,
};
use purrdf_sparql_algebra::{
    AggregateExpression, AggregateFunction, Expression, Function, GraphPattern, Literal, NamedNode,
    NamedNodePattern, TermPattern, TriplePattern, Variable,
};

use crate::agg_fn::{AggregateAccumulator, AggregateRegistry, AlgebraicClass, CustomAggregate};
use crate::error::EvalError;
use crate::eval::{EvalCtx, MAX_UDF_DEPTH, eval_evaluated};
use crate::governor::lift::Evaluated;
use crate::governor::{ChargePoint, GovernorState, QueryGovernors, StopSignal};
use crate::parallel::{force_parallel_for_test, force_sequential_operation};
use crate::user_fn::{Arity, Volatility};

/// The namespace every fixture in this module uses.
const EX: &str = "http://example.org/";

/// What one governed evaluation observed: the rows it kept, and the governor evidence it
/// accumulated. Both halves matter — a governor that stops at the right cost while
/// keeping the wrong rows is as broken as one that keeps the right rows at the wrong
/// cost.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Run {
    /// The rows the run committed.
    rows: usize,
    /// Fuel charged.
    fuel: u64,
    /// The governor that stopped the run, if one did.
    tripped: Option<TrippedGovernor>,
}

/// Evaluate `pattern` over `dataset` under `governors`, reporting what it kept and spent.
fn run(pattern: &GraphPattern, dataset: &RdfDataset, governors: &QueryGovernors) -> Run {
    let state = Arc::new(GovernorState::new(governors));
    let mut ctx = EvalCtx::new(dataset).with_governors(Arc::clone(&state));
    let evaluated = eval_evaluated(pattern, &mut ctx).expect("evaluation must not fail");
    let evidence = state.evidence();
    Run {
        rows: evaluated.rows().len(),
        fuel: evidence.consumed_in(ResourceDimension::Fuel),
        tripped: evidence.tripped(),
    }
}

/// The exact fuel `pattern` costs when nothing refuses it.
///
/// This is what [`QueryGovernors::METERED`] is for, and it is spelled that way here so
/// there is exactly one spelling of "measure without bounding" in the crate — a test that
/// reached for a hand-written near-maximum ceiling would be quietly teaching the pattern
/// the named constant exists to replace.
fn full_cost(pattern: &GraphPattern, dataset: &RdfDataset) -> u64 {
    let measured = run(pattern, dataset, &QueryGovernors::METERED);
    assert_eq!(measured.tripped, None, "the measuring run must complete");
    measured.fuel
}

fn var(name: &str) -> TermPattern {
    TermPattern::Variable(Variable::new(name))
}

fn pred(local: &str) -> NamedNodePattern {
    NamedNodePattern::NamedNode(NamedNode::new_unchecked(format!("{EX}{local}")))
}

fn bgp(patterns: Vec<TriplePattern>) -> GraphPattern {
    GraphPattern::Bgp { patterns }
}

fn triple(subject: TermPattern, predicate: &str, object: TermPattern) -> TriplePattern {
    TriplePattern {
        subject,
        predicate: pred(predicate),
        object,
    }
}

/// A two-hop chain wide enough that the second pattern's row loop is genuinely above
/// [`crate::parallel::PARALLEL_MIN_ROWS`], so the parallel branch is really taken rather
/// than merely reachable.
fn chain_dataset(links: usize) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let p = builder.intern_iri(&format!("{EX}p"));
    let q = builder.intern_iri(&format!("{EX}q"));
    for index in 0..links {
        let s = builder.intern_iri(&format!("{EX}s{index}"));
        let m = builder.intern_iri(&format!("{EX}m{index}"));
        let z = builder.intern_iri(&format!("{EX}z{index}"));
        builder.push_quad(s, p, m, None);
        builder.push_quad(m, q, z, None);
    }
    builder
        .freeze()
        .expect("chain dataset is positionally valid")
}

/// `?s :p ?m . ?m :q ?z` — the plan whose second pattern expands one row per input row.
fn chain_pattern() -> GraphPattern {
    bgp(vec![
        triple(var("s"), "p", var("m")),
        triple(var("m"), "q", var("z")),
    ])
}

#[test]
fn ac3_effective_budget_is_invariant_under_worker_count() {
    // The sequential guard is deliberately NOT engaged: no `force_parallel_for_test`,
    // no `force_sequential_operation`, and an input of 1500 rows, which is above
    // `PARALLEL_MIN_ROWS`. The row loop therefore really runs on rayon, and each pool
    // below really splits it into a different number of differently-sized chunks —
    // `chunk_size_for` is `len / (threads * 4)`, so one thread gives chunks of 375 and
    // eight gives chunks of 46. A test that quietly ran the sequential branch would
    // prove nothing at all about the parallel fold.
    let dataset = chain_dataset(1500);
    let pattern = chain_pattern();

    let cost = full_cost(&pattern, &dataset);
    let budget = cost / 2;
    assert!(budget > 0, "the fixture must cost something to halve");

    let mut observed: Vec<(usize, Run)> = Vec::new();
    for threads in [1_usize, 2, 3, 5, 8] {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .expect("building a fixed-size pool");
        let outcome = pool.install(|| {
            assert_eq!(
                rayon::current_num_threads(),
                threads,
                "the pool must actually be the size this iteration asked for"
            );
            run(
                &pattern,
                &dataset,
                &QueryGovernors::UNBOUNDED.with_fuel(budget),
            )
        });
        observed.push((threads, outcome));
    }

    let (_, first) = &observed[0];
    assert!(
        first.tripped.is_some(),
        "the fixture must actually trip, or invariance is vacuous"
    );
    for (threads, outcome) in &observed {
        assert_eq!(
            outcome, first,
            "the trip point moved at {threads} worker(s): it must be a pure function of \
             the query, the data, and the budget, never of the machine"
        );
    }
}

#[test]
fn ac3_forced_parallel_equals_forced_sequential_under_governors() {
    let dataset = chain_dataset(1500);
    let pattern = chain_pattern();
    let budget = full_cost(&pattern, &dataset) / 2;
    let governors = QueryGovernors::UNBOUNDED.with_fuel(budget);

    let parallel = {
        let _guard = force_parallel_for_test(true);
        run(&pattern, &dataset, &governors)
    };
    let sequential = {
        let _guard = force_sequential_operation();
        run(&pattern, &dataset, &governors)
    };
    // And the plain gate too, which for this input size chooses parallel on its own.
    let ungated = run(&pattern, &dataset, &governors);

    assert!(parallel.tripped.is_some(), "the fixture must actually trip");
    assert_eq!(
        parallel, sequential,
        "the certified partial must not depend on which evaluation strategy ran"
    );
    assert_eq!(parallel, ungated);
}

/// A stop signal that stays quiet until its `fires_on`-th poll, then latches.
///
/// This is how a test arranges for a stop signal and a fuel ceiling to come due at the
/// **same** charge point. The evaluator polls a signal at exactly two kinds of moment:
/// at each operator boundary, and — inside the charge path — at the instant a ceiling is
/// crossed, so that a condition already true there is resolved by precedence rather than
/// by whichever was tested first. Counting polls therefore aims the signal at a chosen
/// one of those moments without the signal needing to see the fuel counter at all.
#[derive(Debug)]
struct FiresOnNthPoll {
    /// The poll ordinal from which this signal reports a cause.
    fires_on: u64,
    /// Polls seen so far. Also read by the test, which asserts the signal really did
    /// fire where it was aimed rather than somewhere earlier.
    polls: AtomicU64,
    /// The latch. Once set, the signal reports the same cause forever, honouring the
    /// contract every [`StopSignal`] implementation is bound by.
    fired: AtomicBool,
}

impl FiresOnNthPoll {
    /// A signal that first reports a cause on its `fires_on`-th poll.
    const fn new(fires_on: u64) -> Self {
        Self {
            fires_on,
            polls: AtomicU64::new(0),
            fired: AtomicBool::new(false),
        }
    }
}

impl StopSignal for FiresOnNthPoll {
    fn poll(&self) -> Option<StopCause> {
        if self.fired.load(Ordering::Relaxed) {
            return Some(StopCause::Cancelled);
        }
        let seen = self.polls.fetch_add(1, Ordering::Relaxed) + 1;
        if seen < self.fires_on {
            return None;
        }
        self.fired.store(true, Ordering::Relaxed);
        Some(StopCause::Cancelled)
    }
}

#[test]
fn fuel_and_stop_signal_due_at_the_same_charge_point_resolve_by_precedence() {
    let dataset = chain_dataset(64);
    let pattern = chain_pattern();
    let budget = full_cost(&pattern, &dataset) / 2;

    // Fuel alone: the fuel ceiling is what stops the query.
    let fuel_only = run(
        &pattern,
        &dataset,
        &QueryGovernors::UNBOUNDED.with_fuel(budget),
    );
    let TrippedGovernor::Budget {
        dimension,
        consumed,
        ..
    } = fuel_only.tripped.expect("the fuel ceiling must be reached")
    else {
        panic!("a fuel ceiling must report a budget governor");
    };
    assert_eq!(dimension, ResourceDimension::Fuel);
    assert!(consumed > budget, "the ceiling was genuinely crossed");

    // The same budget, plus a signal aimed at the first bounded work checkpoint inside
    // the node. Poll one is the operator entry and poll two is the first candidate scan.
    // A stop-only configuration must not wait for the later ordered fuel fold merely
    // because that fold is where the numeric ceiling would have crossed.
    assert!(
        budget < crate::governor::STOP_POLL_FUEL,
        "the fixture must not reach a periodic poll, or the two would not be simultaneous"
    );
    let signal = Arc::new(FiresOnNthPoll::new(2));
    let both = run(
        &pattern,
        &dataset,
        &QueryGovernors::UNBOUNDED
            .with_fuel(budget)
            .with_stop_signal(Arc::clone(&signal) as Arc<dyn StopSignal>),
    );
    assert_eq!(
        signal.polls.load(Ordering::Relaxed),
        2,
        "the signal must fire at the first in-node checkpoint"
    );
    assert_eq!(
        both.tripped,
        Some(TrippedGovernor::Stopped {
            cause: StopCause::Cancelled
        }),
        "an explicit cancellation stops before the deferred fuel fold"
    );
    // Candidate work is metered in an order-stable fold after the scan. Cancellation at
    // the scan checkpoint prevents that fold from claiming work beyond the stop.
    assert!(
        both.fuel <= budget,
        "fuel spent {} must not cross the ceiling {budget} after cancellation",
        both.fuel
    );

    // The mirror image, and the reason the rule is written over conditions rather than
    // over checks: aim the signal past every poll this query makes and the fuel ceiling
    // is what stops it, with the identical budget and the identical signal type.
    let late = Arc::new(FiresOnNthPoll::new(u64::MAX));
    let fuel_wins = run(
        &pattern,
        &dataset,
        &QueryGovernors::UNBOUNDED
            .with_fuel(budget)
            .with_stop_signal(Arc::clone(&late) as Arc<dyn StopSignal>),
    );
    assert_eq!(fuel_wins.tripped, fuel_only.tripped);
}

#[test]
fn intermediate_cells_use_the_maximum_operator_instance_not_a_sum() {
    // Ten two-column rows through a filter, then projected to one column. Three operator
    // instances commit 20, 20 and 10 cells: the largest single instance is 20 and the
    // sum is 50, so a ceiling of exactly 20 separates the two readings.
    let mut builder = RdfDatasetBuilder::new();
    let p = builder.intern_iri(&format!("{EX}p"));
    for index in 0..10 {
        let s = builder.intern_iri(&format!("{EX}s{index}"));
        let o = builder.intern_iri(&format!("{EX}o{index}"));
        builder.push_quad(s, p, o, None);
    }
    let dataset = builder.freeze().expect("fixture is positionally valid");

    let pattern = GraphPattern::Project {
        inner: Box::new(GraphPattern::Filter {
            expr: Expression::Bound(Variable::new("s")),
            inner: Box::new(bgp(vec![triple(var("s"), "p", var("o"))])),
        }),
        variables: vec![Variable::new("s")],
    };

    let at_the_peak = run(
        &pattern,
        &dataset,
        &QueryGovernors::UNBOUNDED.with_max_intermediate_cells(20),
    );
    assert_eq!(
        at_the_peak.tripped, None,
        "a ceiling equal to the largest single operator instance is inclusive, and a \
         running sum would have tripped here"
    );
    assert_eq!(at_the_peak.rows, 10);

    let below_the_peak = run(
        &pattern,
        &dataset,
        &QueryGovernors::UNBOUNDED.with_max_intermediate_cells(19),
    );
    assert_eq!(
        below_the_peak.tripped,
        Some(TrippedGovernor::Budget {
            dimension: ResourceDimension::IntermediateCells,
            limit: 19,
            consumed: 20,
        }),
        "one cell below the peak must trip, and must report the peak, not a total"
    );

    // Cell-denominated, not row-denominated: the same ten rows over one column are half
    // the allocation and are admitted by a ceiling the two-column bag exceeds.
    let narrow = GraphPattern::Project {
        inner: Box::new(bgp(vec![triple(var("s"), "p", var("o"))])),
        variables: vec![Variable::new("s")],
    };
    assert_eq!(
        run(
            &narrow,
            &dataset,
            &QueryGovernors::UNBOUNDED.with_max_intermediate_cells(19),
        )
        .tripped,
        Some(TrippedGovernor::Budget {
            dimension: ResourceDimension::IntermediateCells,
            limit: 19,
            consumed: 20,
        }),
        "the basic graph pattern below the projection is still two columns wide"
    );
}

#[test]
fn scratch_growth_is_charged_so_a_satisfied_row_count_cannot_hide_an_oom() {
    // Five rows. Five. A row cap, an answer cap, and a cell ceiling are all trivially
    // satisfied — and the query still mints five kilobytes of owned string into the
    // scratch arena, which is the shape that takes a process out on wasm where an
    // allocation trap kills the module instance outright.
    let mut builder = RdfDatasetBuilder::new();
    let p = builder.intern_iri(&format!("{EX}p"));
    for index in 0..5 {
        let s = builder.intern_iri(&format!("{EX}s{index}"));
        let o = builder.intern_literal(RdfLiteral {
            lexical_form: format!("value-{index}"),
            datatype: None,
            language: None,
            direction: None,
        });
        builder.push_quad(s, p, o, None);
    }
    let dataset = builder.freeze().expect("fixture is positionally valid");

    let padding = "x".repeat(1024);
    let pattern = GraphPattern::Extend {
        inner: Box::new(bgp(vec![triple(var("s"), "p", var("o"))])),
        variable: Variable::new("big"),
        expression: Expression::FunctionCall(
            Function::Concat,
            vec![
                Expression::Variable(Variable::new("o")),
                Expression::Literal(Literal::new_simple(&padding)),
            ],
        ),
    };

    // Measure the arena growth with every other ceiling generously satisfied.
    let state = Arc::new(GovernorState::new(
        &QueryGovernors::UNBOUNDED
            .with_max_scratch_bytes(u64::MAX - 1)
            .with_max_answers(5)
            .with_max_intermediate_cells(1_000),
    ));
    let mut ctx = EvalCtx::new(&dataset).with_governors(Arc::clone(&state));
    let evaluated = eval_evaluated(&pattern, &mut ctx).expect("evaluation must not fail");
    assert!(
        matches!(evaluated, Evaluated::Complete(_)),
        "every counted ceiling is satisfied, so this query completes"
    );
    let minted = state
        .evidence()
        .consumed_in(ResourceDimension::ScratchBytes);
    assert!(
        minted > 5 * 1024,
        "five rows minted {minted} bytes; the fixture must actually grow the arena"
    );

    // The identical query, with the identical row count, cell count and answer count —
    // and one byte less arena than it needs.
    let starved = run(
        &pattern,
        &dataset,
        &QueryGovernors::UNBOUNDED
            .with_max_scratch_bytes(minted - 1)
            .with_max_answers(5)
            .with_max_intermediate_cells(1_000),
    );
    let tripped = starved.tripped.expect("the arena ceiling must be reached");
    assert_eq!(
        tripped.label(),
        "scratch-exhausted",
        "a satisfied row count must not be able to hide unbounded arena growth"
    );

    // And the boundary is inclusive, like every other ceiling.
    assert_eq!(
        run(
            &pattern,
            &dataset,
            &QueryGovernors::UNBOUNDED
                .with_max_scratch_bytes(minted)
                .with_max_answers(5)
                .with_max_intermediate_cells(1_000),
        )
        .tripped,
        None
    );
}

#[test]
fn udf_depth_is_reported_through_evidence_without_being_relaxable() {
    let dataset = RdfDatasetBuilder::new()
        .freeze()
        .expect("freeze empty dataset");

    // The recursion guard is in force on every execution, governed or not, and every
    // builder preserves it — there is no setter that writes this slot, which is what
    // "not relaxable" means concretely.
    for governors in [
        QueryGovernors::UNBOUNDED,
        QueryGovernors::UNBOUNDED.with_fuel(1),
        QueryGovernors::UNBOUNDED.with_max_answers(1),
        QueryGovernors::UNBOUNDED.with_max_intermediate_cells(1),
        QueryGovernors::UNBOUNDED.with_max_scratch_bytes(1),
        QueryGovernors::UNBOUNDED.with_max_remote_requests(1),
    ] {
        assert_eq!(
            governors.limits().get(ResourceDimension::UdfDepth),
            u64::from(MAX_UDF_DEPTH),
            "no builder may move the recursion guard"
        );
    }

    // Reported: descending three function frames reports depth three, not three
    // invocations. Depth is a maximum, not a total — a query that calls a function a
    // thousand times at depth one has reached depth one.
    let state = Arc::new(GovernorState::new(&QueryGovernors::UNBOUNDED));
    let root = EvalCtx::new(&*dataset).with_governors(Arc::clone(&state));
    let first = root
        .child_for_user_fn()
        .expect("depth 1 is not an error")
        .expect("depth 1 is admitted");
    let second = first
        .child_for_user_fn()
        .expect("depth 2 is not an error")
        .expect("depth 2 is admitted");
    let _third = second
        .child_for_user_fn()
        .expect("depth 3 is not an error")
        .expect("depth 3 is admitted");
    assert_eq!(
        state.evidence().consumed_in(ResourceDimension::UdfDepth),
        3,
        "the depth reached is reported through the same evidence as every other dimension"
    );
    let sibling = first
        .child_for_user_fn()
        .expect("depth 2 again is not an error")
        .expect("depth 2 again is admitted");
    drop(sibling);
    assert_eq!(
        state.evidence().consumed_in(ResourceDimension::UdfDepth),
        3,
        "a second frame at depth two must not add to a depth that was already reached"
    );

    // Enforced, and enforced identically whether or not a governor is attached: the
    // ceiling the governor compares against IS the build constant.
    let mut governed = EvalCtx::new(&*dataset)
        .with_governors(Arc::new(GovernorState::new(&QueryGovernors::UNBOUNDED)));
    let mut ungoverned = EvalCtx::new(&*dataset);
    for _ in 0..MAX_UDF_DEPTH {
        governed = governed
            .child_for_user_fn()
            .expect("inside the governed depth bound is not an error")
            .expect("inside the governed depth bound is admitted");
        ungoverned = ungoverned
            .child_for_user_fn()
            .expect("inside the ungoverned depth bound is not an error")
            .expect("inside the ungoverned depth bound is admitted");
    }
    assert!(
        governed
            .child_for_user_fn()
            .expect("a governed ceiling is an outcome, not an error")
            .is_none(),
        "a caller must not be able to buy their way past the recursion guard, and the \
         governed trip must remain typed"
    );
    assert!(ungoverned.child_for_user_fn().is_err());
}

#[test]
fn ungoverned_evaluation_charges_nothing() {
    let dataset = chain_dataset(1500);
    let pattern = chain_pattern();

    // The ungoverned entry point: no governor state at all, so every charge helper
    // short-circuits on one null test.
    let mut plain = EvalCtx::new(&dataset);
    let ungoverned = eval_evaluated(&pattern, &mut plain).expect("evaluation must not fail");
    let Evaluated::Complete(ungoverned) = ungoverned else {
        panic!("an ungoverned execution has nothing that can trip");
    };

    // `UNBOUNDED` is engaged on no caller-settable dimension, so it must not touch a
    // counter either. Zero consumption everywhere is the observable proof that the
    // per-dimension short-circuit ran ahead of the accounting.
    let state = Arc::new(GovernorState::new(&QueryGovernors::UNBOUNDED));
    let mut governed_ctx = EvalCtx::new(&dataset).with_governors(Arc::clone(&state));
    let governed = eval_evaluated(&pattern, &mut governed_ctx).expect("evaluation must not fail");
    let Evaluated::Complete(governed) = governed else {
        panic!("an unbounded execution has nothing that can trip");
    };

    let evidence = state.evidence();
    assert!(evidence.is_complete());
    for dimension in ResourceDimension::ALL {
        assert_eq!(
            evidence.consumed_in(dimension),
            0,
            "{} was charged by a query that engaged no ceiling",
            dimension.label()
        );
    }
    assert!(
        !state.should_abandon(),
        "nothing tripped, so nothing may ask a worker to abandon its chunk"
    );

    // A governor never changes an answer, only an outcome.
    assert_eq!(governed.rows.len(), ungoverned.rows.len());
    assert_eq!(governed.rows, ungoverned.rows);
    assert_eq!(governed.schema.vars(), ungoverned.schema.vars());

    // A caller who set only a stop signal pays no fuel-charge overhead either: the fuel
    // dimension stays disengaged, so the counter is never reached.
    let stop_only = Arc::new(GovernorState::new(
        &QueryGovernors::UNBOUNDED
            .with_stop_signal(Arc::new(crate::governor::CancellationFlag::new())),
    ));
    let mut stop_ctx = EvalCtx::new(&dataset).with_governors(Arc::clone(&stop_only));
    eval_evaluated(&pattern, &mut stop_ctx).expect("evaluation must not fail");
    assert_eq!(
        stop_only.evidence().consumed_in(ResourceDimension::Fuel),
        0,
        "engaging a stop signal must not engage the fuel counter"
    );
}

/// A compile-time restatement of the fork rule: a worker's context shares the ONE live
/// accounting state through an `Arc` clone rather than copying a counter, so a governed
/// query's effective budget is not multiplied by the thread count. A single-threaded test
/// cannot catch that, so it is checked by pointer identity here.
#[test]
fn a_forked_worker_shares_one_accounting_state() {
    let dataset = RdfDatasetBuilder::new()
        .freeze()
        .expect("freeze empty dataset");
    let state = Arc::new(GovernorState::new(&QueryGovernors::UNBOUNDED.with_fuel(4)));
    let parent = EvalCtx::new(&*dataset).with_governors(Arc::clone(&state));

    let worker = parent.fork_for_worker();
    let callee = parent
        .child_for_user_fn()
        .expect("depth 1 is not an error")
        .expect("depth 1 is admitted");
    for context in [&parent, &worker, &callee] {
        assert!(
            std::ptr::eq(
                Arc::as_ptr(context.governor_state().expect("governed")),
                Arc::as_ptr(&state)
            ),
            "a per-worker copy would multiply the budget by the thread count"
        );
    }

    // And spending through a worker is spending the caller's budget.
    for _ in 0..4 {
        worker
            .charge(ChargePoint::AlgebraNodeEntry)
            .expect("inside the ceiling");
    }
    assert_eq!(
        parent.charge(ChargePoint::AlgebraNodeEntry),
        Err(TrippedGovernor::Budget {
            dimension: ResourceDimension::Fuel,
            limit: 4,
            consumed: 5,
        })
    );
}

#[test]
fn metered_reports_consumption_that_unbounded_does_not() {
    let dataset = chain_dataset(64);
    let pattern = chain_pattern();

    // `UNBOUNDED` engages no caller-settable dimension, so every charge site
    // short-circuits ahead of its counter and the evidence is empty. That is the price of
    // an ungoverned query costing exactly nothing, and it is why a second named state
    // exists rather than a compromise on the first.
    let unbounded = run(&pattern, &dataset, &QueryGovernors::UNBOUNDED);
    assert_eq!(unbounded.tripped, None);
    assert_eq!(unbounded.fuel, 0);

    // `METERED` counts everything and bounds nothing: the completed result carries the
    // numbers a consumer needs in order to choose real ceilings, which is the entire
    // reason evidence is returned on the complete path.
    let metered = run(&pattern, &dataset, &QueryGovernors::METERED);
    assert_eq!(
        metered.tripped, None,
        "measuring must never turn into refusing"
    );
    assert!(
        metered.fuel > 0,
        "a metered run must report what the query actually cost"
    );

    // A governor never changes an answer, only an outcome: measuring the query does not
    // change which rows it produced.
    assert_eq!(metered.rows, unbounded.rows);

    // Every caller-settable dimension is engaged, so none of them is a hole in the
    // evidence a consumer would then size a budget against.
    let state = GovernorState::new(&QueryGovernors::METERED);
    for dimension in [
        ResourceDimension::Fuel,
        ResourceDimension::AnswerRows,
        ResourceDimension::IntermediateCells,
        ResourceDimension::ScratchBytes,
        ResourceDimension::RemoteRequests,
    ] {
        assert!(
            state.is_engaged_in(dimension),
            "{} must be counted by a metered run",
            dimension.label()
        );
        assert!(
            !QueryGovernors::UNBOUNDED.is_engaged_in(dimension),
            "{} must stay free for an unbounded run",
            dimension.label()
        );
    }

    // And the measurement is directly usable as a budget: the exact cost completes, and
    // one below it does not. This is the loop the mode exists to close.
    assert_eq!(
        run(
            &pattern,
            &dataset,
            &QueryGovernors::UNBOUNDED.with_fuel(metered.fuel),
        )
        .tripped,
        None,
        "a ceiling set from a metered measurement must admit the query it measured"
    );
    assert!(
        run(
            &pattern,
            &dataset,
            &QueryGovernors::UNBOUNDED.with_fuel(metered.fuel - 1),
        )
        .tripped
        .is_some(),
        "and one unit less must not"
    );
}

#[test]
fn metered_does_not_relax_the_udf_depth_ceiling() {
    let dataset = RdfDatasetBuilder::new()
        .freeze()
        .expect("freeze empty dataset");

    // Metering is a door into the accounting, not a door around the fixed ceilings.
    assert_eq!(
        QueryGovernors::METERED
            .limits()
            .get(ResourceDimension::UdfDepth),
        u64::from(MAX_UDF_DEPTH),
        "the recursion guard is a build constant, and no mode may move it"
    );
    assert_eq!(
        QueryGovernors::METERED
            .limits()
            .get(ResourceDimension::UdfDepth),
        QueryGovernors::UNBOUNDED
            .limits()
            .get(ResourceDimension::UdfDepth),
    );

    // Enforced in practice, not just configured: a metered run fails closed at exactly
    // the same depth an unbounded one does.
    let mut metered = EvalCtx::new(&*dataset)
        .with_governors(Arc::new(GovernorState::new(&QueryGovernors::METERED)));
    for _ in 0..MAX_UDF_DEPTH {
        metered = metered
            .child_for_user_fn()
            .expect("inside the depth bound is not an evaluation error")
            .expect("inside the depth bound does not trip");
    }
    assert!(
        metered
            .child_for_user_fn()
            .expect("a governed ceiling is an outcome, not an evaluation error")
            .is_none(),
        "asking to be measured must not buy a deeper stack, and the trip stays typed"
    );
    assert!(matches!(
        metered.expression_barrier.observed(),
        Some(TrippedGovernor::Budget {
            dimension: ResourceDimension::UdfDepth,
            limit,
            consumed,
        }) if limit == u64::from(MAX_UDF_DEPTH)
            && consumed == u64::from(MAX_UDF_DEPTH) + 1
    ));
}

// ---------------------------------------------------------------------------
// The property-function points
// ---------------------------------------------------------------------------

/// The predicate IRI every property-function fixture in this module is registered under.
const PF: &str = "http://example.org/pf/emit";

/// A relation that emits a fixed table on every invocation and COUNTS what it was asked
/// for.
///
/// The count is the observation no governor evidence carries: evidence says what the
/// engine spent, and only the relation can say whether it was entered at all. That is
/// what separates "the ceiling prevented the call" from "the call was made and its rows
/// discarded" — exactly the distinction the corpus's exchange counts draw for `SERVICE`.
#[derive(Debug)]
struct CountingRelation {
    /// The rows every invocation emits, in this order.
    rows: Vec<crate::property_fn::PfRow>,
    /// The bound this relation DECLARES, which admission control reads and which is not
    /// required to equal `rows.len()` — a relation that over-declares is the case a cell
    /// ceiling exists to refuse before it runs.
    declared: u64,
    /// Invocations opened.
    opens: Arc<AtomicU64>,
    /// Rows pulled across every invocation.
    pulls: Arc<AtomicU64>,
    /// The single declared mode: the all-free pattern of this arity, which subsumes every
    /// invocation of it.
    modes: Vec<purrdf_core::binding_pattern::BindingPattern>,
}

impl CountingRelation {
    /// A one-column relation emitting `count` IRIs per invocation, declaring the truth.
    fn emitting(count: usize) -> Self {
        Self {
            rows: (0..count)
                .map(|index| vec![TermValue::iri(format!("{EX}r{index}"))])
                .collect(),
            declared: count as u64,
            opens: Arc::new(AtomicU64::new(0)),
            pulls: Arc::new(AtomicU64::new(0)),
            modes: vec![crate::property_fn::PfArity::new(0, 1).all_free_mode()],
        }
    }

    fn opens(&self) -> u64 {
        self.opens.load(Ordering::Relaxed)
    }
}

impl crate::property_fn::PropertyFunction for CountingRelation {
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }

    fn arity(&self) -> crate::property_fn::PfArity {
        crate::property_fn::PfArity::new(0, 1)
    }

    fn modes(&self) -> &[purrdf_core::binding_pattern::BindingPattern] {
        &self.modes
    }

    fn rows_per_invocation(&self, _mode: purrdf_core::binding_pattern::BindingPattern) -> u64 {
        self.declared
    }

    fn open(
        &self,
        _args: &crate::property_fn::PfArgs<'_>,
        _ceiling: Option<u64>,
    ) -> Result<Box<dyn crate::property_fn::PfCursor>, EvalError> {
        self.opens.fetch_add(1, Ordering::Relaxed);
        Ok(Box::new(CountingCursor {
            rows: self.rows.clone(),
            next: 0,
            pulls: Arc::clone(&self.pulls),
        }))
    }
}

/// [`CountingRelation`]'s cursor: the table, in order, once.
#[derive(Debug)]
struct CountingCursor {
    rows: Vec<crate::property_fn::PfRow>,
    next: usize,
    pulls: Arc<AtomicU64>,
}

impl crate::property_fn::PfCursor for CountingCursor {
    fn next(&mut self) -> Result<Option<crate::property_fn::PfRow>, EvalError> {
        self.pulls.fetch_add(1, Ordering::Relaxed);
        let row = self.rows.get(self.next).cloned();
        self.next += 1;
        Ok(row)
    }
}

/// A registry holding `relation` under [`PF`].
fn pf_registry(
    relation: Arc<dyn crate::property_fn::PropertyFunction>,
) -> crate::property_fn::PropertyFunctionRegistry {
    let mut registry = crate::property_fn::PropertyFunctionRegistry::new();
    registry.register(PF, relation);
    registry
}

/// `{ ?s :p ?m . () <PF> ?x }` — one invocation per driving row, exactly the shape the
/// feasibility-ordering pass rebuilds a chain into.
fn pf_pattern() -> GraphPattern {
    GraphPattern::Lateral {
        left: Box::new(bgp(vec![triple(var("s"), "p", var("m"))])),
        right: Box::new(GraphPattern::PropertyFunction(
            purrdf_sparql_algebra::PropertyFunctionCall {
                iri: PF.to_owned(),
                subject_args: vec![],
                object_args: vec![var("x")],
            },
        )),
    }
}

/// A dataset of `links` driving rows for [`pf_pattern`].
fn pf_dataset(links: usize) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let p = builder.intern_iri(&format!("{EX}p"));
    for index in 0..links {
        let s = builder.intern_iri(&format!("{EX}s{index}"));
        let m = builder.intern_iri(&format!("{EX}m{index}"));
        builder.push_quad(s, p, m, None);
    }
    builder.freeze().expect("dataset is positionally valid")
}

/// Evaluate `pattern` with `relations` wired, reporting what it kept and spent together
/// with the per-node, per-charge-point ledger.
///
/// The ledger is what makes an "exactly this much invocation fuel" claim checkable at all:
/// the evidence carries one fuel total, and a total cannot say which point spent it.
fn run_with_relations(
    pattern: &GraphPattern,
    dataset: &RdfDataset,
    relations: &crate::property_fn::PropertyFunctionRegistry,
    governors: &QueryGovernors,
) -> (Run, Vec<crate::governor::NodeCharges>) {
    let state = Arc::new(GovernorState::new(governors));
    let ledger = Arc::new(crate::governor::ledger::ChargeLedger::for_plan(
        pattern,
        &crate::DetHashMap::default(),
    ));
    let mut ctx = EvalCtx::new(dataset)
        .with_governors(Arc::clone(&state))
        .with_property_functions(relations)
        .with_charge_ledger(Arc::clone(&ledger));
    let evaluated = eval_evaluated(pattern, &mut ctx).expect("evaluation must not fail");
    let evidence = state.evidence();
    (
        Run {
            rows: evaluated.rows().len(),
            fuel: evidence.consumed_in(ResourceDimension::Fuel),
            tripped: evidence.tripped(),
        },
        ledger.snapshot(),
    )
}

/// The fuel `point` accounts for across the whole ledger.
fn ledger_fuel(ledger: &[crate::governor::NodeCharges], point: ChargePoint) -> u64 {
    ledger.iter().map(|node| node.fuel_at(point)).sum()
}

#[test]
fn a_governed_property_function_query_spends_exactly_its_invocation_and_row_fuel() {
    // Three driving rows, two emitted rows each: three invocations and six accepted rows,
    // counted rather than approximated, because the whole point of a dedicated point is
    // that a relation's work stops being invisible inside the generic per-node accounting.
    let dataset = pf_dataset(3);
    let relation = Arc::new(CountingRelation::emitting(2));
    let registry =
        pf_registry(Arc::clone(&relation) as Arc<dyn crate::property_fn::PropertyFunction>);
    let pattern = pf_pattern();

    let (measured, ledger) =
        run_with_relations(&pattern, &dataset, &registry, &QueryGovernors::METERED);
    assert_eq!(measured.tripped, None, "the measuring run must complete");
    assert_eq!(
        measured.rows, 6,
        "three driving rows times two emitted rows"
    );
    assert_eq!(relation.opens(), 3, "one invocation per driving row");

    assert_eq!(
        ledger_fuel(&ledger, ChargePoint::PropertyFunctionInvocation),
        3,
        "the invocation point counts calls into host code, one per driving row"
    );
    assert_eq!(
        ledger_fuel(&ledger, ChargePoint::PropertyFunctionRow),
        6,
        "the row point counts rows the relation emitted and this engine accepted"
    );

    // And the decomposition adds up to the single number the evidence reports: a ledger
    // that did not would be a decomposition of some other quantity.
    let total: u64 = ledger
        .iter()
        .map(crate::governor::NodeCharges::fuel_total)
        .sum();
    assert_eq!(total, measured.fuel);
}

#[test]
fn a_property_function_call_trips_at_the_same_point_parallel_and_sequential() {
    // Above `PARALLEL_MIN_ROWS`, so the driving row loop the call rides is genuinely a
    // candidate for the rayon branch rather than merely reachable from it.
    let dataset = pf_dataset(1500);
    let relation = Arc::new(CountingRelation::emitting(2));
    let registry =
        pf_registry(Arc::clone(&relation) as Arc<dyn crate::property_fn::PropertyFunction>);
    let pattern = pf_pattern();

    let (measured, _) = run_with_relations(&pattern, &dataset, &registry, &QueryGovernors::METERED);
    let budget = measured.fuel / 2;
    assert!(budget > 0, "the fixture must cost something to halve");
    let governors = QueryGovernors::UNBOUNDED.with_fuel(budget);

    let parallel = {
        let _guard = force_parallel_for_test(true);
        run_with_relations(&pattern, &dataset, &registry, &governors).0
    };
    let sequential = {
        let _guard = force_sequential_operation();
        run_with_relations(&pattern, &dataset, &registry, &governors).0
    };

    assert!(parallel.tripped.is_some(), "the fixture must actually trip");
    assert_eq!(
        parallel, sequential,
        "a relation's charges are folded in source-item order, so which strategy ran cannot \
         move the trip point"
    );
}

#[test]
fn a_budget_below_the_first_emitted_row_truncates_rather_than_failing() {
    let dataset = pf_dataset(3);
    let relation = Arc::new(CountingRelation::emitting(2));
    let registry =
        pf_registry(Arc::clone(&relation) as Arc<dyn crate::property_fn::PropertyFunction>);
    let pattern = pf_pattern();

    // The cost of everything up to and including the first invocation charge, so the next
    // charge the execution attempts is the first row's. It is refused, and the refusal
    // travels on the governed channel: an empty certified bag, never an evaluation error.
    let (metered, ledger) =
        run_with_relations(&pattern, &dataset, &registry, &QueryGovernors::METERED);
    assert_eq!(metered.tripped, None);
    let rows_charged = ledger_fuel(&ledger, ChargePoint::PropertyFunctionRow);
    assert!(
        rows_charged > 0,
        "the fixture must charge rows to cut below"
    );

    let mut budget = metered.fuel;
    let mut cut = None;
    // Walk the budget down until the run first trips, so the boundary is MEASURED rather
    // than derived from an assumption about which points precede the first row.
    while budget > 0 {
        budget -= 1;
        let (run, ledger) = run_with_relations(
            &pattern,
            &dataset,
            &registry,
            &QueryGovernors::UNBOUNDED.with_fuel(budget),
        );
        if run.tripped.is_some() && ledger_fuel(&ledger, ChargePoint::PropertyFunctionRow) == 0 {
            cut = Some(run);
            break;
        }
    }
    let cut = cut.expect("some budget refuses the first emitted row");
    assert_eq!(
        cut.rows, 0,
        "no row was ever charged, so the call committed none"
    );
    assert!(matches!(
        cut.tripped,
        Some(TrippedGovernor::Budget {
            dimension: ResourceDimension::Fuel,
            ..
        })
    ));

    // Deterministic: the same ceiling cuts in the same place, every time.
    let again = run_with_relations(
        &pattern,
        &dataset,
        &registry,
        &QueryGovernors::UNBOUNDED.with_fuel(budget),
    )
    .0;
    assert_eq!(again, cut);
}

#[test]
fn a_zero_fuel_ceiling_never_enters_the_relation_at_all() {
    let dataset = pf_dataset(3);
    let relation = Arc::new(CountingRelation::emitting(2));
    let registry =
        pf_registry(Arc::clone(&relation) as Arc<dyn crate::property_fn::PropertyFunction>);

    let (run, ledger) = run_with_relations(
        &pf_pattern(),
        &dataset,
        &registry,
        &QueryGovernors::UNBOUNDED.with_fuel(0),
    );
    assert!(
        matches!(
            run.tripped,
            Some(TrippedGovernor::Budget {
                dimension: ResourceDimension::Fuel,
                limit: 0,
                ..
            })
        ),
        "a zero ceiling is valid and admits no charged work: {run:?}"
    );
    assert_eq!(run.rows, 0);
    assert_eq!(
        relation.opens(),
        0,
        "a ceiling that lets host code run once has already been exceeded by the time it \
         is consulted"
    );
    for point in [
        ChargePoint::PropertyFunctionInvocation,
        ChargePoint::PropertyFunctionRow,
    ] {
        assert_eq!(ledger_fuel(&ledger, point), 0, "{point} was charged");
    }
}

// ---------------------------------------------------------------------------
// Aggregate charge points: `aggregate-invocation` and `aggregate-accumulation`
// ---------------------------------------------------------------------------

/// An integer literal, typed exactly as `RdfDatasetBuilder` mints one from a bare Turtle
/// integer — the shape [`agg_group_dataset`]'s fixture and `SUM`'s numeric fold both expect.
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

/// `groups × rows_per_group` rows, `ex:catN` grouping `ex:s{i}` into `groups` equal-sized
/// buckets via `ex:cat`, each row also carrying its own value on `ex:val` — the same shape
/// `crate::modifier`'s own `group_aggregate_forced_parallel_and_sequential_agree` fixture
/// uses, so a group's row count and a query's total row count are both exact, known
/// quantities a test can assert against rather than approximate.
fn agg_group_dataset(groups: i64, rows_per_group: i64) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let cat_pred = b.intern_iri(&format!("{EX}cat"));
    let val_pred = b.intern_iri(&format!("{EX}val"));
    for g in 0..groups {
        for r in 0..rows_per_group {
            let i = g * rows_per_group + r;
            let subject = b.intern_iri(&format!("{EX}s{i}"));
            let cat = b.intern_literal(RdfLiteral {
                lexical_form: format!("cat{g}"),
                datatype: Some(XSD_INTEGER.to_owned()),
                language: None,
                direction: None,
            });
            let val = b.intern_literal(RdfLiteral {
                lexical_form: i.to_string(),
                datatype: Some(XSD_INTEGER.to_owned()),
                language: None,
                direction: None,
            });
            b.push_quad(subject, cat_pred, cat, None);
            b.push_quad(subject, val_pred, val, None);
        }
    }
    b.freeze().expect("aggregate dataset is positionally valid")
}

/// `?s ex:cat ?cat . ?s ex:val ?val` — [`agg_group_dataset`]'s driving join.
fn agg_join_inner() -> GraphPattern {
    GraphPattern::Join {
        left: Box::new(bgp(vec![triple(var("s"), "cat", var("cat"))])),
        right: Box::new(bgp(vec![triple(var("s"), "val", var("val"))])),
    }
}

/// `SELECT ?cat (SUM(?val) AS ?total) WHERE { ... } GROUP BY ?cat` over
/// [`agg_join_inner`] — one built-in aggregate expression per group.
fn sum_group_pattern() -> GraphPattern {
    GraphPattern::Group {
        inner: Box::new(agg_join_inner()),
        variables: vec![Variable::new("cat")],
        aggregates: vec![(
            Variable::new("total"),
            AggregateExpression {
                function: AggregateFunction::Sum,
                args: vec![Expression::Variable(Variable::new("val"))],
                scalarvals: Vec::new(),
                distinct: false,
            },
        )],
    }
}

/// [`sum_group_pattern`] with the aggregate expression naming a registered
/// [`CustomAggregate`] instead of the built-in `SUM` — same group shape, same argument
/// variable, so the two patterns' fuel is comparable value for value.
fn custom_sum_group_pattern(iri: &str) -> GraphPattern {
    GraphPattern::Group {
        inner: Box::new(agg_join_inner()),
        variables: vec![Variable::new("cat")],
        aggregates: vec![(
            Variable::new("total"),
            AggregateExpression {
                function: AggregateFunction::Custom(NamedNode::new_unchecked(iri)),
                args: vec![Expression::Variable(Variable::new("val"))],
                scalarvals: Vec::new(),
                distinct: false,
            },
        )],
    }
}

/// `SELECT (COUNT(DISTINCT ?val) AS ?n) WHERE { ... }` — no `GROUP BY`, so the whole input
/// is one implicit group, and every row shares the SAME `?val` (see
/// [`duplicate_value_dataset`]) so `DISTINCT` discards all but the first.
fn count_distinct_group_pattern() -> GraphPattern {
    GraphPattern::Group {
        inner: Box::new(bgp(vec![triple(var("s"), "val", var("val"))])),
        variables: Vec::new(),
        aggregates: vec![(
            Variable::new("n"),
            AggregateExpression {
                function: AggregateFunction::Count,
                args: vec![Expression::Variable(Variable::new("val"))],
                scalarvals: Vec::new(),
                distinct: true,
            },
        )],
    }
}

/// `rows` triples `ex:sN ex:val "same"`, every one binding `?val` to the SAME literal —
/// the fixture [`count_distinct_group_pattern`] dedups down to a single kept value while
/// still touching every row.
fn duplicate_value_dataset(rows: i64) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let val_pred = b.intern_iri(&format!("{EX}val"));
    let same = b.intern_literal(RdfLiteral {
        lexical_form: "same".to_owned(),
        datatype: None,
        language: None,
        direction: None,
    });
    for i in 0..rows {
        let subject = b.intern_iri(&format!("{EX}s{i}"));
        b.push_quad(subject, val_pred, same, None);
    }
    b.freeze()
        .expect("duplicate-value dataset is positionally valid")
}

/// [`run_with_relations`]'s twin for the aggregate seam: evaluate `pattern` under
/// `governors`, with `registry` wired if the pattern names a custom aggregate, reporting
/// what it kept and spent together with the per-node, per-charge-point ledger.
fn run_with_aggregates(
    pattern: &GraphPattern,
    dataset: &RdfDataset,
    registry: Option<&AggregateRegistry>,
    governors: &QueryGovernors,
) -> (Run, Vec<crate::governor::NodeCharges>) {
    let state = Arc::new(GovernorState::new(governors));
    let ledger = Arc::new(crate::governor::ledger::ChargeLedger::for_plan(
        pattern,
        &crate::DetHashMap::default(),
    ));
    let mut ctx = EvalCtx::new(dataset)
        .with_governors(Arc::clone(&state))
        .with_charge_ledger(Arc::clone(&ledger));
    if let Some(registry) = registry {
        ctx = ctx.with_aggregates(registry);
    }
    let evaluated = eval_evaluated(pattern, &mut ctx).expect("evaluation must not fail");
    let evidence = state.evidence();
    (
        Run {
            rows: evaluated.rows().len(),
            fuel: evidence.consumed_in(ResourceDimension::Fuel),
            tripped: evidence.tripped(),
        },
        ledger.snapshot(),
    )
}

/// A `Commutative` custom `SUM`-alike over a single numeric-lexical argument — the same
/// shape `crate::agg_fn`'s own test fixture uses, kept local so this module's aggregate
/// coverage does not reach into another module's `#[cfg(test)]` items.
struct SumAccumulator {
    total: i64,
}

impl AggregateAccumulator for SumAccumulator {
    fn step(&mut self, args: &[TermValue]) -> Result<(), EvalError> {
        if let Some(TermValue::Literal { lexical_form, .. }) = args.first()
            && let Ok(n) = lexical_form.parse::<i64>()
        {
            self.total += n;
        }
        Ok(())
    }

    fn combine(&mut self, other: Box<dyn AggregateAccumulator>) {
        if let Ok(Some(TermValue::Literal { lexical_form, .. })) = other.finish()
            && let Ok(n) = lexical_form.parse::<i64>()
        {
            self.total += n;
        }
    }

    fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
        Ok(Some(TermValue::typed_literal(
            self.total.to_string(),
            XSD_INTEGER,
        )))
    }
}

struct SumAggregate;

impl CustomAggregate for SumAggregate {
    fn arity(&self) -> Arity {
        Arity::Exact(1)
    }
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }
    fn algebraic_class(&self) -> AlgebraicClass {
        AlgebraicClass::Commutative
    }
    fn state_bound(&self) -> u64 {
        0
    }
    fn init(&self) -> Box<dyn AggregateAccumulator> {
        Box::new(SumAccumulator { total: 0 })
    }
}

const CUSTOM_SUM_IRI: &str = "http://example.org/agg/customSum";

fn custom_sum_registry() -> AggregateRegistry {
    let mut registry = AggregateRegistry::new();
    registry.register(CUSTOM_SUM_IRI, Arc::new(SumAggregate));
    registry
}

#[test]
fn a_governed_group_by_query_spends_exactly_its_aggregate_invocation_and_accumulation_fuel() {
    // Three groups of four rows each: one aggregate expression per group, so
    // `aggregate-invocation` must total exactly three (one per group), and every one of
    // the twelve rows contributes exactly one value to `SUM`, so `aggregate-accumulation`
    // must total exactly twelve — counted rather than approximated, because the whole
    // point of a dedicated pair of points is that a fold's per-group and per-value work
    // stop being invisible inside the generic per-node accounting.
    let dataset = agg_group_dataset(3, 4);
    let pattern = sum_group_pattern();

    let (measured, ledger) =
        run_with_aggregates(&pattern, &dataset, None, &QueryGovernors::METERED);
    assert_eq!(measured.tripped, None, "the measuring run must complete");
    assert_eq!(measured.rows, 3, "one output row per group");

    assert_eq!(
        ledger_fuel(&ledger, ChargePoint::AggregateInvocation),
        3,
        "the invocation point counts one fold per group, independent of group size"
    );
    assert_eq!(
        ledger_fuel(&ledger, ChargePoint::AggregateAccumulation),
        12,
        "the accumulation point counts one charge per value folded, across every group"
    );

    let total: u64 = ledger
        .iter()
        .map(crate::governor::NodeCharges::fuel_total)
        .sum();
    assert_eq!(total, measured.fuel);
}

#[test]
fn a_custom_aggregate_spends_the_same_invocation_and_accumulation_fuel_as_a_built_in_over_the_same_group_shape()
 {
    // Same group shape (three groups of four rows, one aggregate expression per group)
    // folded through a registered `CustomAggregate` instead of the built-in `SUM` fold —
    // the two charge points are dispatched from the ONE call site in
    // `crate::modifier::eval_aggregate` that decides which kind of fold an expression
    // names, so a caller's budget must not need to know which one a given aggregate
    // function happens to be.
    let dataset = agg_group_dataset(3, 4);
    let registry = custom_sum_registry();

    let (builtin, builtin_ledger) = run_with_aggregates(
        &sum_group_pattern(),
        &dataset,
        None,
        &QueryGovernors::METERED,
    );
    let (custom, custom_ledger) = run_with_aggregates(
        &custom_sum_group_pattern(CUSTOM_SUM_IRI),
        &dataset,
        Some(&registry),
        &QueryGovernors::METERED,
    );
    assert_eq!(builtin.tripped, None);
    assert_eq!(custom.tripped, None);

    for point in [
        ChargePoint::AggregateInvocation,
        ChargePoint::AggregateAccumulation,
    ] {
        assert_eq!(
            ledger_fuel(&builtin_ledger, point),
            ledger_fuel(&custom_ledger, point),
            "{point}: a built-in and a custom aggregate over the same group shape must \
             cost the same fuel"
        );
    }
}

#[test]
fn distinct_still_charges_accumulation_for_every_value_it_goes_on_to_discard() {
    // Twelve rows, every one binding `?val` to the SAME literal: `COUNT(DISTINCT ?val)`
    // keeps exactly one of them, but the work of producing and inspecting each of the
    // twelve against the running `DISTINCT` set already happened, so the accumulation
    // point must still count twelve — the documented reading of
    // `ChargePoint::AggregateAccumulation`, checked rather than merely asserted in prose.
    let dataset = duplicate_value_dataset(12);
    let pattern = count_distinct_group_pattern();

    let (measured, ledger) =
        run_with_aggregates(&pattern, &dataset, None, &QueryGovernors::METERED);
    assert_eq!(measured.tripped, None);
    assert_eq!(measured.rows, 1, "one implicit group, no GROUP BY");

    assert_eq!(
        ledger_fuel(&ledger, ChargePoint::AggregateInvocation),
        1,
        "one aggregate expression over one implicit group"
    );
    assert_eq!(
        ledger_fuel(&ledger, ChargePoint::AggregateAccumulation),
        12,
        "every row's value was produced and inspected, even the eleven DISTINCT discarded"
    );
}

#[test]
fn an_aggregate_heavy_query_under_a_tight_budget_stops_with_the_governed_outcome() {
    // Before `aggregate-accumulation` existed, this query's per-value fold work was
    // invisible to the schedule: its whole cost rode the generic per-node accounting, so
    // no fuel ceiling could bind while that fold was still running — the query either
    // completed or was stopped by some other dimension entirely, never by the cost of the
    // fold itself. This walks the budget down from the full measured cost, exactly as
    // `a_budget_below_the_first_emitted_row_truncates_rather_than_failing` does for the
    // property-function seam, until it finds the highest ceiling at which
    // `aggregate-accumulation`'s ledgered total is strictly less than the complete run's —
    // i.e. the ceiling that stops the query WHILE it is mid-fold, having already folded
    // some values into a group's running state but not all of them.
    let dataset = agg_group_dataset(5, 8); // 40 rows across 5 groups
    let pattern = sum_group_pattern();

    let (full, full_ledger) =
        run_with_aggregates(&pattern, &dataset, None, &QueryGovernors::METERED);
    assert_eq!(full.tripped, None, "the measuring run must complete");
    assert_eq!(full.rows, 5);
    let full_accumulation = ledger_fuel(&full_ledger, ChargePoint::AggregateAccumulation);
    assert_eq!(
        full_accumulation, 40,
        "one accumulation charge per row, every group"
    );

    let mut budget = full.fuel;
    let mut mid_fold = None;
    while budget > 0 {
        budget -= 1;
        let (run, ledger) = run_with_aggregates(
            &pattern,
            &dataset,
            None,
            &QueryGovernors::UNBOUNDED.with_fuel(budget),
        );
        let accumulated = ledger_fuel(&ledger, ChargePoint::AggregateAccumulation);
        if accumulated < full_accumulation {
            mid_fold = Some((run, accumulated));
            break;
        }
    }
    let (tight, accumulated) =
        mid_fold.expect("some budget below the full cost must cut into the accumulation fold");
    assert!(
        accumulated > 0,
        "the ceiling that first cuts the fold must still admit at least one folded value, or \
         the walk overshot past `aggregate-invocation` into the fold's own admission"
    );
    assert!(
        matches!(
            tight.tripped,
            Some(TrippedGovernor::Budget {
                dimension: ResourceDimension::Fuel,
                ..
            })
        ),
        "the governed outcome must name the fuel ceiling: {:?}",
        tight.tripped
    );
    // `Group` is an opaque edge under any truncation of its own fold (see
    // `crate::modifier::eval_group`'s docs): an aggregate over a partial fold is a
    // different number, never a subset of the true one, so the whole grouped output is
    // withheld rather than reporting some groups' answers and not others'.
    assert_eq!(
        tight.rows, 0,
        "a trip inside the fold must withhold the whole grouped output, not a subset of it"
    );

    // Deterministic: the same ceiling cuts in the same place, every time.
    let again = run_with_aggregates(
        &pattern,
        &dataset,
        None,
        &QueryGovernors::UNBOUNDED.with_fuel(budget),
    )
    .0;
    assert_eq!(again, tight);
}

#[test]
fn within_group_chunked_fold_charges_identically_forced_parallel_vs_sequential() {
    // One huge group — `agg_group_dataset(1, rows)` puts every row under the SAME
    // `?cat` key — large enough to cross `PARALLEL_MIN_ROWS` and actually chunk the
    // fold (see `crate::parallel::par_chunk_reduce_init`, wired in by
    // `crate::modifier::eval_aggregate`'s phase 2). Every charge this aggregate makes
    // happens in phase 1 — the row-evaluation loop, UNCHANGED by whether phase 2 goes
    // on to chunk the fold — so forced-parallel and forced-sequential must spend
    // EXACTLY the same fuel at both aggregate charge points, automatically, with no
    // charge site needing to special-case chunking at all.
    const ROWS: i64 = 3000;
    let dataset = agg_group_dataset(1, ROWS);
    let pattern = sum_group_pattern();

    let (parallel_run, parallel_ledger) = {
        let _guard = force_parallel_for_test(true);
        run_with_aggregates(&pattern, &dataset, None, &QueryGovernors::METERED)
    };
    let (sequential_run, sequential_ledger) = {
        let _guard = force_sequential_operation();
        run_with_aggregates(&pattern, &dataset, None, &QueryGovernors::METERED)
    };

    assert_eq!(parallel_run.tripped, None);
    assert_eq!(sequential_run.tripped, None);
    assert_eq!(parallel_run.rows, 1, "one huge group");
    assert_eq!(sequential_run.rows, 1);
    assert_eq!(
        parallel_run.fuel, sequential_run.fuel,
        "total fuel spend must not depend on whether the within-group fold chunked"
    );
    for point in [
        ChargePoint::AggregateInvocation,
        ChargePoint::AggregateAccumulation,
    ] {
        assert_eq!(
            ledger_fuel(&parallel_ledger, point),
            ledger_fuel(&sequential_ledger, point),
            "{point}: within-group chunking must not change per-point spend"
        );
    }
    assert_eq!(
        ledger_fuel(&parallel_ledger, ChargePoint::AggregateAccumulation),
        u64::try_from(ROWS).unwrap(),
        "one accumulation charge per row in the single huge group"
    );
}

/// The type parameter is spelled out in a couple of places above; this keeps the
/// production id type named so a change to it is a compile error here rather than a
/// silent change of what these tests cover.
const _: fn() = || {
    fn assert_id<I: purrdf_core::ViewTermId>() {}
    assert_id::<TermId>();
};
