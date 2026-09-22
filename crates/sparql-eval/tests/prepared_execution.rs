// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A prepared, parameterized execution: prepare once, bind and run many times.
//!
//! The per-execution allocation pin lives here, in the crate that owns the property,
//! rather than only downstream. Every other assertion about this path is in
//! `purrdf-shapes`' allocation suite, which means a change to the evaluator that
//! regressed it would redden a different crate's test — or, if those fixtures ever
//! drifted, nothing at all.
//!
//! [`PREPARED_EXECUTION_RUN_ALLOCATIONS`] is the pin: a memoized re-run's EXACT
//! marginal cost, not merely its stability. A test that only asserted stability
//! ("run N and run N+1 cost the same") would pass at any constant magnitude,
//! including one that regressed from zero to some larger number that then stayed
//! flat — which is exactly the shape of regression an exact pin catches and a
//! stability-only assertion cannot.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use purrdf_alloc_probe::{CountingAllocator, WholeProcessWindow};
use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlEngine, SparqlRequest, SparqlResult, TermValue,
};
use purrdf_sparql_eval::{
    AggregateAccumulator, AggregateRegistry, AlgebraicClass, Arity, CustomAggregate, EvalError,
    InternedOutcome, MemoryRelation, NativeSparqlEngine, PropertyFunctionRegistry, QueryOptions,
    Volatility,
};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

/// Serializes every measured region in this binary.
///
/// [`WholeProcessWindow`] reads one process-global ledger and `cargo test` runs test
/// functions concurrently, so two measurements in flight at once would each report
/// the union of both regions while appearing to report their own. Every test here
/// takes it as its FIRST statement — a lock taken after even one allocation has
/// already let that allocation land unguarded — and holds it for the whole body,
/// which is why it is bound rather than dropped immediately.
static MEASURE_LOCK: Mutex<()> = Mutex::new(());

/// Take [`MEASURE_LOCK`], absorbing poison.
fn measure_lock() -> MutexGuard<'static, ()> {
    MEASURE_LOCK.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Turn `PreparedExecution`'s memo differential oracle off for `operation`, then
/// restore it — even if `operation` panics.
///
/// # Why this test needs it
///
/// `purrdf_sparql_eval::set_memo_verification_enabled` gates a debug-only oracle
/// inside `PreparedExecution::substituted` that re-runs the FULL, un-memoized
/// rewrite on every memo hit and compares it against the memo's answer — see that
/// function's rustdoc in `crates/sparql-eval/src/execution.rs` for the full case.
/// Its own cost — a whole clone-and-walk of the admitted algebra — IS the
/// allocation the memo exists to remove, and `cargo test` always builds with
/// `debug_assertions` on, so without turning it off, [`PREPARED_EXECUTION_RUN_ALLOCATIONS`]
/// below would pin the oracle's cost rather than the memo's.
///
/// Unlike `crates/shapes/tests/sparql_path_alloc.rs`'s version of this helper,
/// every test in this file runs on the calling thread alone — nothing here fans
/// work out over `rayon` — so a plain `set_memo_verification_enabled` call
/// reaches every thread this file's measurements ever run on, and no
/// `rayon::broadcast` is needed to make the flag visible to a worker pool.
fn without_memo_verification<T>(operation: impl FnOnce() -> T) -> T {
    purrdf_sparql_eval::set_memo_verification_enabled(false);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(operation));
    purrdf_sparql_eval::set_memo_verification_enabled(true);
    match result {
        Ok(value) => value,
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

/// The exact marginal allocation cost of one run of an already-warm, already-memoized
/// [`purrdf_sparql_eval::PreparedExecution`] — a re-bind plus a memo-hit write of its
/// cells, with the memo's differential oracle held off by [`without_memo_verification`].
///
/// Not zero: re-binding still writes a probe list and the evaluator's own per-call
/// setup (a fresh solution buffer, the interned egress) is not free. What this pin
/// claims is narrower and achievable — that the cost is a FIXED constant, independent
/// of how many times the execution has already run — which is what
/// `re_running_a_prepared_execution_costs_27_allocations` asserts against it, exactly
/// at both `N` and `N+1`. If this ever moves, re-measure with `without_memo_verification`
/// bracketing the window exactly as the test does, and update this constant to match:
/// it is not a ceiling, it is the currently-measured marginal cost.
const PREPARED_EXECUTION_RUN_ALLOCATIONS: u64 = 27;

const QUERY: &str = "SELECT ?o WHERE { ?this <http://example.org/p> ?o }";

/// `:s{i} :p :o{i}` for `i` in `0..subjects`.
fn dataset(subjects: u32) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri("http://example.org/p");
    for i in 0..subjects {
        let s = b.intern_iri(&format!("http://example.org/s{i}"));
        let o = b.intern_iri(&format!("http://example.org/o{i}"));
        b.push_quad(s, p, o, None);
    }
    b.freeze().expect("freeze")
}

fn iri(i: u32) -> TermValue {
    TermValue::Iri(format!("http://example.org/s{i}"))
}

#[test]
fn a_prepared_execution_answers_each_binding_from_one_plan() {
    let _guard = measure_lock();
    let ds = dataset(8);
    let engine = NativeSparqlEngine::new();
    let mut execution = engine
        .prepare_execution(QUERY, None, &["this"], QueryOptions::EMPTY)
        .expect("prepare");

    // Each binding must answer for ITS OWN subject. A handle that reused the first
    // binding, or that failed to narrow at all, is distinguishable here because every
    // subject has a different object.
    for subject in 0..8u32 {
        execution.bind(0, iri(subject)).expect("bind");
        let answer = engine
            .execute(&mut execution, &*ds, QueryOptions::EMPTY, |outcome| {
                let InternedOutcome::Solutions(solutions) = outcome else {
                    panic!("expected solutions");
                };
                assert_eq!(solutions.len(), 1, "exactly one row per subject");
                let row = &solutions.rows()[0];
                format!("{:?}", solutions.cell(row, 0).expect("bound cell"))
            })
            .expect("execute");
        assert!(
            answer.contains(&format!("http://example.org/o{subject}")),
            "subject s{subject} must answer o{subject}, got {answer}"
        );
    }
}

#[test]
fn re_running_a_prepared_execution_costs_27_allocations() {
    let _guard = measure_lock();
    let ds = dataset(8);
    let engine = NativeSparqlEngine::new();
    let mut execution = engine
        .prepare_execution(QUERY, None, &["this"], QueryOptions::EMPTY)
        .expect("prepare");

    let mut run = |subject: u32| -> u64 {
        execution.bind(0, iri(subject)).expect("bind");
        let window = WholeProcessWindow::open();
        engine
            .execute(&mut execution, &*ds, QueryOptions::EMPTY, |outcome| {
                let InternedOutcome::Solutions(solutions) = outcome else {
                    panic!("expected solutions");
                };
                assert_eq!(solutions.len(), 1);
            })
            .expect("execute");
        window.close().allocations
    };

    // Warm every lazy on this thread with the exact call being measured.
    let _warm = (run(0), run(1));

    // The measured window: the memo's differential oracle comes OFF here and only
    // here — see [`without_memo_verification`] for why a debug build's own
    // correctness check would otherwise be exactly the allocation this pin is
    // trying to see past.
    let (first, second, third) = without_memo_verification(|| (run(2), run(3), run(4)));
    println!("prepared execution: {first}, {second}, {third} allocations per run");
    assert_eq!(
        (first, second, third),
        (
            PREPARED_EXECUTION_RUN_ALLOCATIONS,
            PREPARED_EXECUTION_RUN_ALLOCATIONS,
            PREPARED_EXECUTION_RUN_ALLOCATIONS
        ),
        "a prepared execution's per-run cost is pinned EXACTLY: a figure that moves — in \
         either direction — between runs, or away from the pinned constant, means per-run \
         state is accumulating or the evaluator's marginal cost has changed"
    );
}

/// A query that MINTS a term per row, so the retained scratch interner has
/// something in it to leak.
///
/// `CONCAT` produces a value the dataset does not contain, which is exactly the
/// case `SolutionTerm::Computed` exists for: it is written into the run's scratch
/// table and read back out of it by index. A query whose every answer is already a
/// dataset term would leave that table empty, and every assertion below about
/// clearing it would hold vacuously.
const MINTING_QUERY: &str =
    "SELECT ?v WHERE { ?this <http://example.org/p> ?o . BIND(CONCAT(\"tag-\", STR(?o)) AS ?v) }";

/// One run of `execution` bound to subject `i`, returning the `ScratchBytes` the
/// governor charged it.
///
/// A FRESH [`purrdf_sparql_eval::GovernorState`] per run, deliberately: the charge
/// site compares the interner's running minted total against what THIS state has
/// already consumed, so a fresh state makes each run report its own minting rather
/// than a difference against an earlier run's. That is what turns a carried-over
/// `minted_bytes` into a visible number instead of a silent one.
fn scratch_bytes_of_run(
    engine: &NativeSparqlEngine,
    execution: &mut purrdf_sparql_eval::PreparedExecution,
    ds: &RdfDataset,
    subject: u32,
) -> u64 {
    use purrdf_sparql_eval::{GovernorState, QueryGovernors, ResourceDimension};
    execution.bind(0, iri(subject)).expect("bind");
    let state = Arc::new(GovernorState::new(&QueryGovernors::METERED));
    let rows = engine
        .execute_governed_in_operation(execution, ds, QueryOptions::EMPTY, &state, |outcome| {
            let InternedOutcome::Solutions(solutions) = outcome else {
                panic!("expected solutions");
            };
            solutions.len()
        })
        .expect("the metered run completes");
    let purrdf_sparql_eval::InternedGoverned::Complete { value, .. } = rows else {
        panic!("METERED engages every counter at a ceiling nothing reaches, so nothing trips");
    };
    assert_eq!(value, 1, "each subject answers exactly one row");
    state
        .evidence()
        .consumed
        .get(ResourceDimension::ScratchBytes)
}

/// **A handle's later run is observationally a FRESH handle's first run: the
/// retained scratch interner is emptied between runs, values and counter alike.**
///
/// `PreparedExecution` keeps its scratch interner across runs and clears it rather
/// than dropping it, which is the whole of `ExecutionWorkspace`. The failure mode
/// of that design is not a crash: a table retained and not fully cleared carries
/// one run's state into the next, and `ScratchInterner::clear` is written as a
/// destructuring `let` precisely so a field cannot be forgotten. The compiler
/// enforces that every field is NAMED. This enforces that clearing them is
/// OBSERVABLE, which the compiler cannot.
///
/// Three halves, because the fields fail differently and a test of one proves
/// nothing about the others. Each was confirmed to OBSERVE by breaking the clear it
/// guards and watching this test redden — which is how the third came to exist: the
/// first two both stayed green with the value tables left uncleared.
///
/// 1. **The answers.** Bindings run in sequence on ONE handle must answer exactly
///    what FRESH handles answer, binding for binding. The control is the fresh
///    handle — its interner has never held anything — and every treatment row is a
///    different subject with a different minted value, so a run answering from a
///    stale table answers the wrong string rather than an equal one.
/// 2. **The counter.** `minted_bytes` looks like bookkeeping and is what
///    `EvalCtx::charge_scratch_growth` charges the `ScratchBytes` ceiling from, so
///    carrying it would make the Nth run report N runs' minting and trip a ceiling
///    early. A reused handle's second run must charge exactly what a fresh handle's
///    first run charges for the same binding — and that figure must be non-zero,
///    or this half is measuring a query that mints nothing.
/// 3. **The tables.** Leaving the value table uncleared is the one failure the
///    first two halves CANNOT see, and it is worth saying why rather than leaving
///    the gap: the interner de-duplicates by value, so a stale entry is found
///    rather than misread and every answer stays right. What it breaks is the
///    saving itself — the retained workspace grows by one run's values on every
///    run, forever, which is the opposite of keeping a bounded capacity. That is
///    observable only through the size, so the size is what this half reads.
#[test]
fn a_reused_handle_answers_and_charges_exactly_as_a_fresh_one_does() {
    let _guard = measure_lock();
    let ds = dataset(16);
    let engine = NativeSparqlEngine::new();

    let answer_of = |execution: &mut purrdf_sparql_eval::PreparedExecution, subject: u32| {
        execution.bind(0, iri(subject)).expect("bind");
        engine
            .execute(execution, &*ds, QueryOptions::EMPTY, |outcome| {
                let InternedOutcome::Solutions(solutions) = outcome else {
                    panic!("expected solutions");
                };
                assert_eq!(solutions.len(), 1, "exactly one row per subject");
                let row = &solutions.rows()[0];
                format!("{:?}", solutions.cell(row, 0).expect("bound cell"))
            })
            .expect("execute")
    };

    // 1. The answers.
    let mut reused = engine
        .prepare_execution(MINTING_QUERY, None, &["this"], QueryOptions::EMPTY)
        .expect("prepare");
    let from_reused: Vec<String> = (0..8u32).map(|s| answer_of(&mut reused, s)).collect();
    let from_fresh: Vec<String> = (0..8u32)
        .map(|s| {
            let mut once = engine
                .prepare_execution(MINTING_QUERY, None, &["this"], QueryOptions::EMPTY)
                .expect("prepare");
            answer_of(&mut once, s)
        })
        .collect();
    assert_eq!(
        from_reused, from_fresh,
        "a handle's Nth run must answer what a fresh handle's first run answers; a \
         difference here is one focus node's computed value surfacing in another's row"
    );
    // Non-vacuity: the eight answers must actually differ from one another, or a
    // stale table could serve any of them and the comparison above would hold.
    let mut distinct = from_reused.clone();
    distinct.sort();
    distinct.dedup();
    assert_eq!(
        distinct.len(),
        8,
        "the eight bindings must mint eight DIFFERENT values, or a run answering from \
         a stale table would be indistinguishable from one answering correctly: \
         {from_reused:?}"
    );

    // 2. The counter.
    let mut reused = engine
        .prepare_execution(MINTING_QUERY, None, &["this"], QueryOptions::EMPTY)
        .expect("prepare");
    let first = scratch_bytes_of_run(&engine, &mut reused, &ds, 0);
    let second = scratch_bytes_of_run(&engine, &mut reused, &ds, 1);
    let mut virgin = engine
        .prepare_execution(MINTING_QUERY, None, &["this"], QueryOptions::EMPTY)
        .expect("prepare");
    let control = scratch_bytes_of_run(&engine, &mut virgin, &ds, 1);
    assert!(
        first > 0,
        "the fixture query must actually mint something, or this half is vacuous"
    );
    assert_eq!(
        second, control,
        "a reused handle's second run must charge the `ScratchBytes` governor exactly \
         what a fresh handle's first run charges for the same binding; a larger figure \
         is `minted_bytes` carried across the clear, which would trip a ceiling after \
         N runs instead of on the run that earned it"
    );
    assert_eq!(
        second, first,
        "`o0` and `o1` are the same length, so the two bindings mint the same number of \
         bytes; a difference means this half is reading something other than one run's \
         minting"
    );

    // 3. The tables.
    let mut reused = engine
        .prepare_execution(MINTING_QUERY, None, &["this"], QueryOptions::EMPTY)
        .expect("prepare");
    for subject in 0..8u32 {
        drop(answer_of(&mut reused, subject));
    }
    let after_eight = reused.retained_workspace_bytes();
    // Eight MORE runs, on eight subjects none of the first eight used, so every one
    // of them mints a value the interner has never seen. A run repeating an earlier
    // binding would be de-duplicated even by an uncleared table, and this half would
    // hold for the wrong reason.
    for subject in 8..16u32 {
        drop(answer_of(&mut reused, subject));
    }
    let after_sixteen = reused.retained_workspace_bytes();
    assert!(
        after_eight > 0,
        "the retained workspace must actually be holding tables, or this half is \
         asserting that nothing does not grow"
    );
    assert_eq!(
        after_eight, after_sixteen,
        "the retained workspace must not grow with the NUMBER of runs: sixteen runs \
         hold what eight hold. A larger figure is one run's values surviving into the \
         next run's table — which still answers correctly, and grows without bound"
    );
}

/// The exact per-call allocation cost of evaluating an already-admitted
/// [`purrdf_sparql_eval::PreparedQuery`] through `query_prepared`.
///
/// This is the OTHER prepared door, and the one with a per-call re-admission on it:
/// a bare `&PreparedQuery` may be handed to calls naming different registries, so
/// `check_plan_matches_relations` runs on every call — a `Query::validate` walk and
/// a feasibility replanning walk — where a `PreparedExecution` runs them once and
/// carries the result ([`PREPARED_EXECUTION_RUN_ALLOCATIONS`] is that path).
///
/// It is pinned because it is where two reductions actually land, and neither one
/// is visible on any other pin in this workspace:
///
/// * the nesting guard was DUPLICATED between that per-call check and the
///   evaluation it precedes, so the per-call copy went (`engine::check_plan_soundness`);
///   the surviving copy runs over the tree actually evaluated, refuses the same
///   trees with the same diagnostic, and no longer allocates and grows a second
///   traversal stack per call;
/// * `Query::validate`'s IRI admission stopped building an owned `purrdf_iri::Iri`
///   per IRI in the query and now asks `purrdf_iri::is_absolute`, which runs the
///   identical grammar over a borrow — one heap `String` per IRI per call, gone.
///
/// Measured on this revision at **59**, against **65** before those two changes, over
/// [`PREPARED_PLAN_QUERY`]'s three distinct IRIs — and the six decompose exactly,
/// each half isolated by reverting one change at a time and re-measuring:
///
/// * 65 → 62 when the IRI admission stopped owning: **three**, one `String` per IRI
///   occurrence in the algebra, which is why this fixture's query carries three
///   distinct IRIs rather than one;
/// * 62 → 59 when the duplicate nesting walk went: **three**, the traversal stack
///   that second walk allocated and grew on every call.
const PREPARED_PLAN_CALL_ALLOCATIONS: u64 = 59;

/// The query [`PREPARED_PLAN_CALL_ALLOCATIONS`] is measured over.
///
/// Three DISTINCT IRIs, spelled out rather than prefixed, because the per-IRI term
/// this pin exists to hold down is per IRI OCCURRENCE in the algebra: a one-IRI
/// query would make that term and the constant beside it indistinguishable.
const PREPARED_PLAN_QUERY: &str = "SELECT ?o WHERE {\n  ?s <http://example.org/p> ?o .\n  \
     ?s <http://example.org/q> ?r .\n  FILTER(?r != <http://example.org/absent>)\n}";

/// **Evaluating an admitted plan through `query_prepared` costs exactly
/// [`PREPARED_PLAN_CALL_ALLOCATIONS`] allocations, on every call.**
///
/// An exact pin rather than a stability one, for the reason this file's module
/// documentation gives: "run N and run N+1 cost the same" is satisfied at any
/// magnitude, including one that regressed and then stayed flat.
///
/// The answer is checked inside the measured window on every call, so a plan that
/// became cheaper by answering less would fail here rather than read as a win.
#[test]
fn evaluating_an_admitted_plan_costs_a_pinned_constant_per_call() {
    let _guard = measure_lock();
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri("http://example.org/p");
    let q = b.intern_iri("http://example.org/q");
    for i in 0..8u32 {
        let s = b.intern_iri(&format!("http://example.org/s{i}"));
        let o = b.intern_iri(&format!("http://example.org/o{i}"));
        let r = b.intern_iri(&format!("http://example.org/r{i}"));
        b.push_quad(s, p, o, None);
        b.push_quad(s, q, r, None);
    }
    let ds = b.freeze().expect("freeze");

    let engine = NativeSparqlEngine::new();
    let prepared = engine
        .prepare_query_with_options(PREPARED_PLAN_QUERY, None, QueryOptions::EMPTY)
        .expect("the fixture query parses and is admitted");

    let run = || -> u64 {
        let window = WholeProcessWindow::open();
        let answer = engine
            .query_prepared(&ds, &prepared, &[], QueryOptions::EMPTY)
            .expect("the admitted plan evaluates");
        let allocations = window.close().allocations;
        let SparqlResult::Solutions { rows, .. } = answer else {
            panic!("a SELECT answers with solutions");
        };
        assert_eq!(
            rows.len(),
            8,
            "every subject must still answer, or a cheaper figure is a smaller answer"
        );
        allocations
    };

    // Warm every lazy on this thread with the exact call being measured.
    let _warm = (run(), run());
    let (first, second, third) = (run(), run(), run());
    println!("query_prepared: {first}, {second}, {third} allocations per call");
    assert_eq!(
        (first, second, third),
        (
            PREPARED_PLAN_CALL_ALLOCATIONS,
            PREPARED_PLAN_CALL_ALLOCATIONS,
            PREPARED_PLAN_CALL_ALLOCATIONS
        ),
        "the per-call cost of evaluating an admitted plan is pinned EXACTLY: a figure that \
         moves — in either direction — means the per-call re-admission or the evaluator's \
         marginal cost has changed"
    );
}

#[test]
fn running_with_a_parameter_unbound_is_refused() {
    let _guard = measure_lock();
    let ds = dataset(4);
    let engine = NativeSparqlEngine::new();
    let mut execution = engine
        .prepare_execution(QUERY, None, &["this"], QueryOptions::EMPTY)
        .expect("prepare");

    // The refusal. Treating an unbound parameter as unrestricted would answer over
    // every subject — a silently WIDER answer, which is the failure this refuses.
    let refused = engine.execute(&mut execution, &*ds, QueryOptions::EMPTY, |_| ());
    let error = refused.expect_err("an unbound parameter must be refused");
    assert!(
        error.to_string().contains("unbound"),
        "the diagnostic must name the problem: {error}"
    );

    // Its NEIGHBOUR: the same execution, same engine, same dataset, once bound, must
    // succeed. Without this the refusal above is satisfied equally well by an
    // execution that can never run at all.
    execution.bind(0, iri(1)).expect("bind");
    let rows = engine
        .execute(&mut execution, &*ds, QueryOptions::EMPTY, |outcome| {
            let InternedOutcome::Solutions(solutions) = outcome else {
                panic!("expected solutions");
            };
            solutions.len()
        })
        .expect("a bound execution must run");
    assert_eq!(rows, 1, "the bound run must answer for its subject");
}

#[test]
fn unbind_all_returns_every_slot_to_unbound_and_a_later_bind_answers_the_new_value() {
    let _guard = measure_lock();
    let ds = dataset(4);
    let engine = NativeSparqlEngine::new();
    let mut execution = engine
        .prepare_execution(QUERY, None, &["this"], QueryOptions::EMPTY)
        .expect("prepare");

    // Bind and run: s1 answers o1. o0 and o1 differ (see `dataset`/`iri`), so this
    // execution's rows are the oracle for which subject it actually ran under.
    execution.bind(0, iri(1)).expect("bind");
    let rows = engine
        .execute(&mut execution, &*ds, QueryOptions::EMPTY, |outcome| {
            let InternedOutcome::Solutions(solutions) = outcome else {
                panic!("expected solutions");
            };
            let row = &solutions.rows()[0];
            format!("{:?}", solutions.cell(row, 0).expect("bound cell"))
        })
        .expect("a bound execution must run");
    assert!(rows.contains("http://example.org/o1"), "got {rows}");

    // THE INVALID CASE: after clearing, running with nothing re-bound is refused —
    // exactly as it would be on an execution nothing had ever bound. If `unbind_all`
    // left the slot's old value in place, this run would silently re-answer for s1
    // instead of failing here.
    execution.unbind_all();
    let refused = engine.execute(&mut execution, &*ds, QueryOptions::EMPTY, |_| ());
    let error = refused.expect_err("a cleared parameter must be refused, not re-answered stale");
    assert!(
        error.to_string().contains("unbound"),
        "the diagnostic must name the problem: {error}"
    );

    // THE NEIGHBOURING VALID CASE: binding again after the clear makes it answer,
    // and it must answer for the NEW value (s2 -> o2), never the stale s1 -> o1 the
    // clear was meant to erase. A test that only checked `is_err()` above could not
    // tell "honoured the clear" from "silently dropped the whole execution".
    execution.bind(0, iri(2)).expect("bind after clearing");
    let rows = engine
        .execute(&mut execution, &*ds, QueryOptions::EMPTY, |outcome| {
            let InternedOutcome::Solutions(solutions) = outcome else {
                panic!("expected solutions");
            };
            let row = &solutions.rows()[0];
            format!("{:?}", solutions.cell(row, 0).expect("bound cell"))
        })
        .expect("a re-bound execution must run");
    assert!(
        rows.contains("http://example.org/o2"),
        "must answer the NEW binding (o2), not the cleared one (o1): got {rows}"
    );
    assert!(!rows.contains("http://example.org/o1"), "got {rows}");
}

#[test]
fn binding_a_name_that_was_not_declared_is_refused() {
    let _guard = measure_lock();
    let engine = NativeSparqlEngine::new();
    let mut execution = engine
        .prepare_execution(QUERY, None, &["this"], QueryOptions::EMPTY)
        .expect("prepare");

    let error = execution
        .bind_named("absent", iri(0))
        .expect_err("an undeclared parameter name must be refused");
    assert!(
        error.to_string().contains("absent"),
        "the diagnostic must name the parameter: {error}"
    );
    // The neighbour: a name that WAS declared still binds.
    execution
        .bind_named("this", iri(0))
        .expect("a declared parameter must bind by name");
    assert_eq!(execution.slot("this"), Some(0));

    // And an out-of-range slot is refused on the positional door too.
    let error = execution
        .bind(7, iri(0))
        .expect_err("an out-of-range slot must be refused");
    assert!(error.to_string().contains('7'), "got {error}");
}

#[test]
fn declaring_one_parameter_twice_is_refused() {
    let _guard = measure_lock();
    let engine = NativeSparqlEngine::new();
    let error = engine
        .prepare_execution(QUERY, None, &["this", "this"], QueryOptions::EMPTY)
        .expect_err("a repeated parameter has no single slot to bind");
    assert!(error.to_string().contains("more than once"), "got {error}");

    // The neighbour: two DISTINCT parameters prepare fine, including one the query
    // does not mention — which is not an error, only an unused binding.
    let execution = engine
        .prepare_execution(QUERY, None, &["this", "other"], QueryOptions::EMPTY)
        .expect("distinct parameters must prepare");
    assert_eq!(execution.parameters().len(), 2);
}

// ---------------------------------------------------------------------------
// The ID DOOR's refusal parity.
//
// `PreparedExecution::bind_id` / `bind_named_id` bind a parameter to the dataset's
// own term id instead of to an owned value, to skip the round trip in which an id
// becomes a term, the term becomes algebra, and the algebra is hashed back to the
// id. It is an ADDITIONAL door, not a replacement, and the thing an additional door
// most easily gets wrong is not its answers but its REFUSALS: a second entry that
// accepts what the first one refuses is a silent widening, and every refusal below
// exists because accepting it would answer a question nobody asked.
//
// So each of the four refusals the value door has is executed through BOTH doors
// and the two diagnostics compared, and each is paired with its neighbouring
// accepted case — which must not merely succeed but ANSWER, over a fixture whose
// subjects carry distinct objects, so "accepted" and "accepted and then silently
// answered for the wrong subject" cannot be confused.
// ---------------------------------------------------------------------------

/// This fixture's id for `:s{i}`.
///
/// Through `term_id_by_value`, which is the only value→id door a `DatasetView`
/// has, so the id handed to `bind_id` below is genuinely the dataset's and not a
/// number the test invented.
fn subject_id(ds: &RdfDataset, i: u32) -> purrdf_core::TermId {
    ds.term_id_by_value(&iri(i))
        .expect("the fixture interns every subject")
}

/// The `?o` values one prepared run answers, as debug strings.
fn run_answers(
    engine: &NativeSparqlEngine,
    execution: &mut purrdf_sparql_eval::PreparedExecution,
    ds: &RdfDataset,
) -> Vec<String> {
    engine
        .execute(execution, ds, QueryOptions::EMPTY, |outcome| {
            let InternedOutcome::Solutions(solutions) = outcome else {
                panic!("expected solutions");
            };
            solutions
                .rows()
                .iter()
                .map(|row| format!("{:?}", solutions.cell(row, 0)))
                .collect::<Vec<_>>()
        })
        .expect("a bound execution must run")
}

#[test]
fn the_id_door_refuses_an_unbound_parameter_exactly_as_the_value_door_does() {
    let _guard = measure_lock();
    let ds = dataset(4);
    let engine = NativeSparqlEngine::new();
    // Two parameters, one of which the query never mentions, so "bind one and run"
    // is a state the engine has to refuse rather than a query that cannot parse.
    let mut execution = engine
        .prepare_execution(QUERY, None, &["this", "other"], QueryOptions::EMPTY)
        .expect("prepare");

    // THE REFUSAL, reached through the id door: slot 1 is still `None`. Treating it
    // as unrestricted would answer over every subject.
    execution
        .bind_id(0, &*ds, subject_id(&ds, 1))
        .expect("the id door binds");
    let through_id = engine
        .execute(&mut execution, &*ds, QueryOptions::EMPTY, |_| ())
        .expect_err("an unbound parameter must be refused after an id-door bind");

    // The same state reached through the value door refuses with the same words.
    execution.unbind_all();
    execution.bind(0, iri(1)).expect("the value door binds");
    let through_value = engine
        .execute(&mut execution, &*ds, QueryOptions::EMPTY, |_| ())
        .expect_err("an unbound parameter must be refused after a value-door bind");
    assert_eq!(
        through_id.to_string(),
        through_value.to_string(),
        "the two doors must refuse an unbound parameter identically"
    );

    // THE NEIGHBOUR: bind the second slot through the id door too, and the run must
    // ANSWER — for s1, whose object is o1 and nobody else's.
    execution.unbind_all();
    execution
        .bind_id(0, &*ds, subject_id(&ds, 1))
        .expect("the id door binds");
    execution
        .bind_id(1, &*ds, subject_id(&ds, 2))
        .expect("the id door binds");
    let answers = run_answers(&engine, &mut execution, &ds);
    assert_eq!(answers.len(), 1, "one subject, one object: {answers:?}");
    assert!(
        answers[0].contains("http://example.org/o1"),
        "the id door must answer for the subject it bound, not a neighbour: {answers:?}"
    );
}

#[test]
fn the_id_door_refuses_an_undeclared_name_exactly_as_the_value_door_does() {
    let _guard = measure_lock();
    let ds = dataset(4);
    let engine = NativeSparqlEngine::new();
    let mut execution = engine
        .prepare_execution(QUERY, None, &["this"], QueryOptions::EMPTY)
        .expect("prepare");

    let through_id = execution
        .bind_named_id("absent", &*ds, subject_id(&ds, 0))
        .expect_err("an undeclared parameter name must be refused by the id door");
    let through_value = execution
        .bind_named("absent", iri(0))
        .expect_err("an undeclared parameter name must be refused by the value door");
    assert_eq!(
        through_id.to_string(),
        through_value.to_string(),
        "the two doors must refuse an undeclared name identically"
    );
    assert!(
        through_id.to_string().contains("absent"),
        "the diagnostic must name the parameter: {through_id}"
    );

    // THE NEIGHBOUR: the declared name binds through the id door and answers.
    execution
        .bind_named_id("this", &*ds, subject_id(&ds, 3))
        .expect("a declared parameter must bind by name through the id door");
    let answers = run_answers(&engine, &mut execution, &ds);
    assert_eq!(answers.len(), 1, "one subject, one object: {answers:?}");
    assert!(
        answers[0].contains("http://example.org/o3"),
        "the named id door must answer for s3: {answers:?}"
    );
}

#[test]
fn the_id_door_refuses_an_out_of_range_slot_exactly_as_the_value_door_does() {
    let _guard = measure_lock();
    let ds = dataset(4);
    let engine = NativeSparqlEngine::new();
    let mut execution = engine
        .prepare_execution(QUERY, None, &["this"], QueryOptions::EMPTY)
        .expect("prepare");

    let through_id = execution
        .bind_id(7, &*ds, subject_id(&ds, 0))
        .expect_err("an out-of-range slot must be refused by the id door");
    let through_value = execution
        .bind(7, iri(0))
        .expect_err("an out-of-range slot must be refused by the value door");
    assert_eq!(
        through_id.to_string(),
        through_value.to_string(),
        "the two doors must refuse an out-of-range slot identically"
    );
    assert!(through_id.to_string().contains('7'), "got {through_id}");

    // THE NEIGHBOUR: the in-range slot binds through the id door and answers.
    execution
        .bind_id(0, &*ds, subject_id(&ds, 2))
        .expect("the in-range slot must bind");
    let answers = run_answers(&engine, &mut execution, &ds);
    assert_eq!(answers.len(), 1, "one subject, one object: {answers:?}");
    assert!(
        answers[0].contains("http://example.org/o2"),
        "the id door must answer for s2: {answers:?}"
    );
}

#[test]
fn a_duplicate_declaration_is_refused_before_either_door_exists() {
    let _guard = measure_lock();
    let ds = dataset(4);
    let engine = NativeSparqlEngine::new();

    // The fourth refusal is the one the two doors cannot differ on, and saying why
    // is the point: a repeated parameter name is refused at PREPARATION, so there is
    // no handle for either door to be called on. Parity here is structural rather
    // than compared — there is exactly one refusal and it happens before any binding
    // door exists.
    let error = engine
        .prepare_execution(QUERY, None, &["this", "this"], QueryOptions::EMPTY)
        .expect_err("a repeated parameter has no single slot to bind");
    assert!(error.to_string().contains("more than once"), "got {error}");

    // THE NEIGHBOUR: distinct names prepare, and the id door then answers through
    // the handle that refusal would have denied.
    let mut execution = engine
        .prepare_execution(QUERY, None, &["this", "other"], QueryOptions::EMPTY)
        .expect("distinct parameters must prepare");
    execution
        .bind_id(0, &*ds, subject_id(&ds, 0))
        .expect("the id door binds");
    execution
        .bind_id(1, &*ds, subject_id(&ds, 1))
        .expect("the id door binds");
    let answers = run_answers(&engine, &mut execution, &ds);
    assert_eq!(answers.len(), 1, "one subject, one object: {answers:?}");
    assert!(
        answers[0].contains("http://example.org/o0"),
        "the id door must answer for s0: {answers:?}"
    );
}

#[test]
fn the_two_doors_are_interchangeable_within_one_execution() {
    let _guard = measure_lock();
    let ds = dataset(4);
    let engine = NativeSparqlEngine::new();
    let mut execution = engine
        .prepare_execution(QUERY, None, &["this"], QueryOptions::EMPTY)
        .expect("prepare");

    // The id door is ADDITIONAL, so one handle must take either door on any run and
    // answer for whatever the LAST bind said — including when the doors alternate.
    // A slot that remembered which door wrote it, or that kept an earlier door's
    // value beside a later one's, would answer a stale subject here rather than
    // fail, which is why each step reads the object rather than the row count.
    for (step, expected) in [(0u32, "o0"), (1, "o1"), (2, "o2"), (3, "o3")] {
        if step % 2 == 0 {
            execution
                .bind_id(0, &*ds, subject_id(&ds, step))
                .expect("the id door binds");
        } else {
            execution.bind(0, iri(step)).expect("the value door binds");
        }
        let answers = run_answers(&engine, &mut execution, &ds);
        assert_eq!(answers.len(), 1, "one subject, one object: {answers:?}");
        assert!(
            answers[0].contains(&format!("http://example.org/{expected}")),
            "step {step} must answer {expected}: {answers:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// `execute` must refuse a plan/registry disagreement, exactly the way every
// other governed and ungoverned entry that can observe one already does
// (`query_prepared_view`, `query_prepared_fallible_view`,
// `query_governed_prepared_in_state`, …). `execute` takes `execution` (which
// carries the plan `prepare_execution` admitted under ITS `options`) and a
// SEPARATE `options` for the run — two different calls, so nothing else stops
// them from naming different registries. Before this fix that disagreement
// evaluated silently; these tests are what makes it unreachable on the
// prepared-execution handle too.
// ---------------------------------------------------------------------------

/// The namespace a host configures for the relation predicate below. Registering a
/// relation is what makes `NativeSparqlEngine::prepare_for` recognise the predicate
/// as a call at parse time; with no registry in scope it is ordinary data.
const REL_NS: &str = "https://example.org/rel/";

/// The data namespace of the relation/dataset fixture terms.
const REL_EX: &str = "https://example.org/d/";

/// The query every property-function test below prepares and runs.
const RELATION_QUERY: &str = "PREFIX rel: <https://example.org/rel/>\n\
                              SELECT ?person ?team WHERE { ?person rel:memberOf ?team }\n";

/// The host relation: three (person, team) pairs held in host memory, reachable
/// from no graph.
fn relation_registry() -> PropertyFunctionRegistry {
    let iri = |local: &str| TermValue::iri(format!("{REL_EX}{local}"));
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(
        format!("{REL_NS}memberOf"),
        Arc::new(
            MemoryRelation::new(
                1,
                1,
                vec![
                    vec![iri("ada"), iri("alpha")],
                    vec![iri("brian"), iri("alpha")],
                    vec![iri("chen"), iri("beta")],
                ],
            )
            .expect("every row is two values wide"),
        ),
    );
    registry
}

/// A dataset holding ONE ordinary triple under the very same predicate IRI the
/// relation above registers, binding a DIFFERENT (person, team) pair than any row
/// the relation emits. This is the observing oracle: a run that honours the
/// registry answers the relation's three rows and never touches this triple; a run
/// that silently fell back to a plain graph scan (the shape this fix closes) would
/// answer this ONE triple instead. The two answers cannot be confused for each
/// other by row count OR by content, so the test can assert on the rows rather
/// than merely on `is_err()`/`len()`.
fn relation_dataset() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri(&format!("{REL_EX}graph_only"));
    let p = b.intern_iri(&format!("{REL_NS}memberOf"));
    let o = b.intern_iri(&format!("{REL_EX}graph_only_team"));
    b.push_quad(s, p, o, None);
    b.freeze().expect("freeze fixture")
}

fn with_relations(registry: &PropertyFunctionRegistry) -> QueryOptions<'_> {
    QueryOptions {
        property_functions: registry,
        ..QueryOptions::EMPTY
    }
}

/// Read `RELATION_QUERY`'s `(?person, ?team)` rows out of an interned outcome as
/// owned `(String, String)` pairs, in solution order.
fn person_team_rows<D: purrdf_core::DatasetView + Sync>(
    outcome: InternedOutcome<'_, '_, D>,
) -> Vec<(String, String)> {
    let InternedOutcome::Solutions(solutions) = outcome else {
        panic!("expected solutions");
    };
    let person = solutions.column("person").expect("?person is projected");
    let team = solutions.column("team").expect("?team is projected");
    solutions
        .rows()
        .iter()
        .map(|row| {
            let cell = |column: usize| {
                solutions
                    .cell(row, column)
                    .and_then(|value| value.as_iri().map(str::to_owned))
                    .expect("every cell here is a bound IRI")
            };
            (cell(person), cell(team))
        })
        .collect()
}

/// (a) THE INVALID CASE and (b) its neighbouring VALID case, per the repo rule: a
/// plan prepared under `QueryOptions::EMPTY` (no registry in scope, so
/// `rel:memberOf` stayed an ordinary triple pattern) must be refused when
/// `execute` is handed `options` that DOES carry the registry — and the identical
/// plan/registry pairing, matched, must still answer.
#[test]
fn executing_a_prepared_plan_under_a_mismatched_property_function_registry_is_refused_but_a_matched_registry_still_answers()
 {
    let _guard = measure_lock();
    let engine = NativeSparqlEngine::new();
    let registry = relation_registry();
    let ds = relation_dataset();

    // (a) Prepared with NO registry: the predicate parses as ordinary data, so the
    // admitted plan is a plain BGP triple pattern over `rel:memberOf`.
    let mut stale = engine
        .prepare_execution(RELATION_QUERY, None, &[], QueryOptions::EMPTY)
        .expect("prepare with no registry parses as ordinary data");

    let refused = engine.execute(&mut stale, &*ds, with_relations(&registry), |_| ());
    let error = refused.expect_err(
        "a plan prepared with no registry must be refused when executed under one, not \
         silently evaluated as a graph scan over the ordinary triple pattern it was admitted \
         as",
    );
    assert_eq!(error.code, "native-sparql-property-function");

    // (b) The neighbour: prepared AND executed under the SAME registry. The rows
    // must be exactly the relation's three rows — never `graph_only`/
    // `graph_only_team`, which is what a silently-dropped registry would have
    // answered instead (see `relation_dataset`'s doc comment for the oracle).
    let mut matched = engine
        .prepare_execution(RELATION_QUERY, None, &[], with_relations(&registry))
        .expect("prepare with the registry lowers the predicate to a call");
    let rows = engine
        .execute(
            &mut matched,
            &*ds,
            with_relations(&registry),
            person_team_rows,
        )
        .expect("a plan and options that agree on the registry must execute");
    assert_eq!(
        rows,
        vec![
            (format!("{REL_EX}ada"), format!("{REL_EX}alpha")),
            (format!("{REL_EX}brian"), format!("{REL_EX}alpha")),
            (format!("{REL_EX}chen"), format!("{REL_EX}beta")),
        ],
        "the answer must be the relation's rows, honoured from the registry — not the \
         graph's differing graph_only/graph_only_team triple"
    );
}

// ---------------------------------------------------------------------------
// The custom-aggregate twin. An unregistered `Custom` aggregate IRI is refused at
// PREPARE time (see `purrdf-sparql-eval`'s `aggregate_function_e2e.rs`), so there
// is no registry-free "stale plan" shape to reuse here the way there is for a
// property function: the reproduction instead needs TWO non-empty registries that
// resolve the SAME IRI to DIFFERENT aggregates with IDENTICAL declared metadata
// (arity, volatility, algebraic class, state bound) — declared metadata alone
// cannot prove the two registries answer a call the same way.
// ---------------------------------------------------------------------------

const SUM_IRI: &str = "https://example.org/agg#sum";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

/// A running integer sum over its single argument's lexical form.
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

    fn combine(&mut self, other: Box<dyn AggregateAccumulator>) -> Result<(), EvalError> {
        if let Some(TermValue::Literal { lexical_form, .. }) = other.finish()?
            && let Ok(n) = lexical_form.parse::<i64>()
        {
            self.total += n;
        }
        Ok(())
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
        self
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
    fn init(&self, _scalarvals: &[(String, TermValue)]) -> Box<dyn AggregateAccumulator> {
        Box::new(SumAccumulator { total: 0 })
    }
}

/// Declares IDENTICALLY to [`SumAggregate`] (arity, volatility, algebraic class,
/// state bound) but computes a PRODUCT — the oracle: a SUM over the fixture below
/// is 15, a PRODUCT is 40, so the two cannot be confused by accident.
struct ProductAccumulator {
    total: i64,
}

impl AggregateAccumulator for ProductAccumulator {
    fn step(&mut self, args: &[TermValue]) -> Result<(), EvalError> {
        if let Some(TermValue::Literal { lexical_form, .. }) = args.first()
            && let Ok(n) = lexical_form.parse::<i64>()
        {
            self.total *= n;
        }
        Ok(())
    }

    fn combine(&mut self, other: Box<dyn AggregateAccumulator>) -> Result<(), EvalError> {
        if let Some(TermValue::Literal { lexical_form, .. }) = other.finish()?
            && let Ok(n) = lexical_form.parse::<i64>()
        {
            self.total *= n;
        }
        Ok(())
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
        self
    }

    fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
        Ok(Some(TermValue::typed_literal(
            self.total.to_string(),
            XSD_INTEGER,
        )))
    }
}

struct ProductAggregate;

impl CustomAggregate for ProductAggregate {
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
    fn init(&self, _scalarvals: &[(String, TermValue)]) -> Box<dyn AggregateAccumulator> {
        Box::new(ProductAccumulator { total: 1 })
    }
}

fn sum_registry() -> AggregateRegistry {
    let mut registry = AggregateRegistry::new();
    registry.register(SUM_IRI, Arc::new(SumAggregate));
    registry
}

fn product_registry() -> AggregateRegistry {
    let mut registry = AggregateRegistry::new();
    registry.register(SUM_IRI, Arc::new(ProductAggregate));
    registry
}

fn with_aggregates(registry: &AggregateRegistry) -> QueryOptions<'_> {
    QueryOptions {
        aggregates: registry,
        ..QueryOptions::EMPTY
    }
}

/// `ex:val` = {1, 2, 2, 10}: SUM = 15, PRODUCT = 40 — a PRODUCT answer here could
/// only come from silently running under the mismatched registry.
fn aggregate_dataset() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let val = b.intern_iri(&format!("{REL_EX}val"));
    for (index, n) in [1i64, 2, 2, 10].into_iter().enumerate() {
        let s = b.intern_iri(&format!("{REL_EX}s{index}"));
        let v = b.intern_literal(RdfLiteral::typed(n.to_string(), XSD_INTEGER.to_owned()));
        b.push_quad(s, val, v, None);
    }
    b.freeze().expect("freeze fixture")
}

fn agg_query() -> String {
    format!("SELECT (AGG(<{SUM_IRI}>, ?v) AS ?total) WHERE {{ ?s <{REL_EX}val> ?v }}")
}

fn total_cell<D: purrdf_core::DatasetView + Sync>(outcome: InternedOutcome<'_, '_, D>) -> i64 {
    let InternedOutcome::Solutions(solutions) = outcome else {
        panic!("expected solutions");
    };
    assert_eq!(solutions.len(), 1, "one row, no GROUP BY");
    let row = &solutions.rows()[0];
    let total = solutions.column("total").expect("?total is projected");
    match solutions.cell(row, total).expect("bound") {
        TermValue::Literal { lexical_form, .. } => lexical_form.parse().expect("integer literal"),
        other => panic!("expected an integer literal, got {other:?}"),
    }
}

/// (a) THE INVALID CASE and (b) its neighbouring VALID case for a custom
/// aggregate: registry A (SUM) admits and arity-checks the call at prepare time;
/// executing under registry B (PRODUCT, declared identically) must be refused —
/// never silently computed under B's different accumulator. The SAME registry
/// instance at both prepare and execute must still work.
#[test]
fn executing_a_prepared_plan_under_a_mismatched_aggregate_registry_is_refused_but_a_matched_registry_still_answers()
 {
    let _guard = measure_lock();
    let engine = NativeSparqlEngine::new();
    let registry_a = sum_registry();
    let registry_b = product_registry();

    // The reproduction only means what it claims if the two registries' DECLARED
    // metadata is byte-identical for this IRI — confirm that first.
    assert_eq!(
        registry_a.describe().expect("no panic"),
        registry_b.describe().expect("no panic"),
        "the two registries must declare identically for this to be a meaningful \
         reproduction of the declaration-only fingerprint gap"
    );

    let ds = aggregate_dataset();
    let query = agg_query();

    // (a) Prepared under registry A (SUM), executed under registry B (PRODUCT).
    let mut prepared = engine
        .prepare_execution(&query, None, &[], with_aggregates(&registry_a))
        .expect("registry A admits and arity-checks the call");
    let refused = engine.execute(&mut prepared, &*ds, with_aggregates(&registry_b), |_| ());
    let error = refused.expect_err(
        "a plan prepared under registry A must be refused when executed under registry B, \
         never silently computed under B's different accumulator",
    );
    assert_eq!(error.code, "native-sparql-aggregate-function");

    // (b) The neighbour: the SAME registry instance at both prepare and execute.
    let mut matched = engine
        .prepare_execution(&query, None, &[], with_aggregates(&registry_a))
        .expect("registry A admits and arity-checks the call");
    let total = engine
        .execute(&mut matched, &*ds, with_aggregates(&registry_a), total_cell)
        .expect("the SAME registry instance must be accepted at execution");
    assert_eq!(total, 15, "1 + 2 + 2 + 10, never the PRODUCT's 40");
}

// ---------------------------------------------------------------------------
// The prebind memo: `PreparedExecution::substituted` retains a rewritten tree
// after the SECOND consecutive sighting of one lane and value-shape list, and
// every run after that writes into it instead of rewriting a fresh one — see
// `purrdf_sparql_eval`'s `prebind_memo` module. The tests below observe the memo
// itself, not merely the answers it produces: "the third run answers correctly"
// is satisfied equally well by a handle that silently rebuilds the tree every
// time and happens to get it right, which is exactly the failure this path's own
// design note warns against.
// ---------------------------------------------------------------------------

/// **The memo's reuse is observable by its COST, not just its answer.**
///
/// A handle that dispatches to the memo on its third-and-later runs pays for one
/// rewrite (the several extra ones `PrebindMemo::build` performs, on the second
/// sighting) and then writes cells — refcount bumps — on every run after. A
/// handle that silently kept taking the ordinary rewrite on every run, whatever
/// `substituted` claims, clones the whole admitted algebra and reallocates every
/// visited node each time. So a memoized run and a run that can never build one
/// (a brand-new handle every time, so it is always a first sighting) must differ
/// in allocation count — and if the dispatch were bypassed, they would not.
#[test]
fn the_third_run_costs_measurably_less_because_the_memo_is_reused() {
    let _guard = measure_lock();
    let ds = dataset(8);
    let engine = NativeSparqlEngine::new();

    // A fresh handle every call: this execution never reaches a second sighting of
    // anything, so `substituted` always takes the ordinary rewrite. Preparing is
    // outside the window; only bind + execute is measured.
    let run_never_memoized = |subject: u32| -> u64 {
        let mut execution = engine
            .prepare_execution(QUERY, None, &["this"], QueryOptions::EMPTY)
            .expect("prepare");
        execution.bind(0, iri(subject)).expect("bind");
        let window = WholeProcessWindow::open();
        engine
            .execute(&mut execution, &*ds, QueryOptions::EMPTY, |outcome| {
                let InternedOutcome::Solutions(solutions) = outcome else {
                    panic!("expected solutions");
                };
                assert_eq!(solutions.len(), 1);
            })
            .expect("execute");
        window.close().allocations
    };

    let mut memoized = engine
        .prepare_execution(QUERY, None, &["this"], QueryOptions::EMPTY)
        .expect("prepare");
    let mut run_memoized = |subject: u32| -> u64 {
        memoized.bind(0, iri(subject)).expect("bind");
        let window = WholeProcessWindow::open();
        engine
            .execute(&mut memoized, &*ds, QueryOptions::EMPTY, |outcome| {
                let InternedOutcome::Solutions(solutions) = outcome else {
                    panic!("expected solutions");
                };
                assert_eq!(solutions.len(), 1);
            })
            .expect("execute");
        window.close().allocations
    };

    // Warm-up, outside every window: the plan cache, the pre-binding-name
    // interner and the allocator's arenas are first-touch lazies. This is also
    // `memoized`'s first sighting (ordinary rewrite).
    let _ = run_never_memoized(0);
    let _ = run_memoized(0);
    // `memoized`'s second consecutive sighting: the run that BUILDS the memo,
    // which costs several extra rewrites and is never the figure to compare
    // against.
    let _ = run_memoized(1);

    // Measured. `never_memoized` is a first sighting every time, by construction;
    // `memoized` is now on its third-and-later sightings, which hit the retained
    // tree.
    let never_a = run_never_memoized(2);
    let never_b = run_never_memoized(3);
    let memo_a = run_memoized(2);
    let memo_b = run_memoized(3);

    println!("never memoized: {never_a}, {never_b}; memoized (reused): {memo_a}, {memo_b}");
    assert_eq!(
        memo_a, memo_b,
        "a memo hit's cost must be stable across runs, exactly as the ordinary path is"
    );
    assert!(
        memo_a < never_a && memo_a < never_b,
        "a memoized run must cost strictly less than a run that can never build a memo, \
         or the dispatch is silently taking the ordinary rewrite on every run and reuse is \
         not actually happening: memoized = {memo_a}, never memoized = {never_a}/{never_b}"
    );
}

/// **A blank-node focus node and an IRI focus node, through the SAME handle,
/// alternated enough to build a memo for one shape and then cross the other.**
///
/// `term_pattern_from_ground` (`crates/sparql-eval/src/substitute.rs`) refuses a
/// blank-node value, so a blank focus node is bound by the `VALUES` seed alone
/// while an IRI one is ALSO pushed into the leaf pattern — two different trees.
/// This drives one execution IRI, IRI (builds a memo for the IRI shape), blank (a
/// different shape: the ordinary rewrite, the IRI memo left alone), IRI again
/// (the memo is still there and still correct), blank again — and checks every
/// answer. Two IRI subjects and two blank subjects with DIFFERING objects are the
/// observing oracle: a handle that corrupted across the shape switch would answer
/// with a neighbour's object rather than failing outright.
#[test]
fn a_blank_and_an_iri_focus_node_answer_correctly_through_the_same_handle() {
    let _guard = measure_lock();
    const NS: &str = "http://example.org/execution-blank#";
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri(&format!("{NS}p"));
    let iri_s0 = b.intern_iri(&format!("{NS}iri0"));
    let iri_s1 = b.intern_iri(&format!("{NS}iri1"));
    let blank_s0 = b.intern_blank("blank0", purrdf_core::BlankScope::DEFAULT);
    let blank_s1 = b.intern_blank("blank1", purrdf_core::BlankScope::DEFAULT);
    let o_iri0 = b.intern_iri(&format!("{NS}o-iri0"));
    let o_iri1 = b.intern_iri(&format!("{NS}o-iri1"));
    let o_blank0 = b.intern_iri(&format!("{NS}o-blank0"));
    let o_blank1 = b.intern_iri(&format!("{NS}o-blank1"));
    b.push_quad(iri_s0, p, o_iri0, None);
    b.push_quad(iri_s1, p, o_iri1, None);
    b.push_quad(blank_s0, p, o_blank0, None);
    b.push_quad(blank_s1, p, o_blank1, None);
    let ds = b.freeze().expect("freeze");

    let engine = NativeSparqlEngine::new();
    let query = format!("SELECT ?o WHERE {{ ?this <{NS}p> ?o }}");
    let mut execution = engine
        .prepare_execution(&query, None, &["this"], QueryOptions::EMPTY)
        .expect("prepare");

    let run = |execution: &mut purrdf_sparql_eval::PreparedExecution, this: TermValue| -> String {
        execution.bind(0, this).expect("bind");
        engine
            .execute(execution, &*ds, QueryOptions::EMPTY, |outcome| {
                let InternedOutcome::Solutions(solutions) = outcome else {
                    panic!("expected solutions");
                };
                assert_eq!(solutions.len(), 1, "exactly one row per focus node");
                let row = &solutions.rows()[0];
                format!("{:?}", solutions.cell(row, 0).expect("bound cell"))
            })
            .expect("execute")
    };

    let this_iri = |n: u32| TermValue::iri(format!("{NS}iri{n}"));
    let this_blank = |n: u32| TermValue::blank(format!("blank{n}"));

    // IRI, IRI: builds the memo for the IRI shape.
    let answer = run(&mut execution, this_iri(0));
    assert!(answer.contains("o-iri0"), "got {answer}");
    let answer = run(&mut execution, this_iri(1));
    assert!(answer.contains("o-iri1"), "got {answer}");

    // A BLANK focus node: a different shape, so the ordinary rewrite runs and the
    // IRI memo is left untouched.
    let answer = run(&mut execution, this_blank(0));
    assert!(answer.contains("o-blank0"), "got {answer}");

    // Back to IRI: the memo built two runs ago is still there and still correct.
    let answer = run(&mut execution, this_iri(0));
    assert!(answer.contains("o-iri0"), "got {answer}");

    // The second blank focus node.
    let answer = run(&mut execution, this_blank(1));
    assert!(answer.contains("o-blank1"), "got {answer}");
}

/// **A repeated pre-bound name is not an error: it is the documented empty
/// solution.**
///
/// Two seeds binding the SAME variable to two DIFFERENT terms are incompatible —
/// see `apply_probes`' doc comment in `crates/sparql-eval/src/substitute.rs` — so
/// the query answers zero rows rather than being refused or silently answering as
/// if only one binding had been supplied. This is not reachable through
/// `PreparedExecution` (`prepare_execution` refuses a repeated PARAMETER
/// declaration outright — see `declaring_one_parameter_twice_is_refused` above),
/// so it is exercised on the ordinary `SparqlEngine::query` door, which builds its
/// pre-binding list straight from the caller's substitutions with no such check.
#[test]
fn a_repeated_pre_bound_name_answers_the_empty_solution() {
    let _guard = measure_lock();
    let ds = dataset(4);
    let engine = NativeSparqlEngine::new();

    let repeated = [("this".to_owned(), iri(0)), ("this".to_owned(), iri(1))];
    let result = engine
        .query(
            &ds,
            SparqlRequest {
                query: QUERY,
                base_iri: None,
                substitutions: &repeated,
            },
        )
        .expect("a repeated pre-bound name is a defined answer, not a refusal");
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("expected a solution sequence");
    };
    assert!(
        rows.is_empty(),
        "two incompatible bindings for the same variable must answer the empty solution, \
         got {rows:?}"
    );

    // The neighbour: a SINGLE pre-binding (no repeat) over the same query and
    // dataset still answers normally, so the emptiness above is about the REPEAT
    // and not about the subject being individually unanswerable.
    let single = [("this".to_owned(), iri(0))];
    let result = engine
        .query(
            &ds,
            SparqlRequest {
                query: QUERY,
                base_iri: None,
                substitutions: &single,
            },
        )
        .expect("a single pre-binding must still answer");
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("expected a solution sequence");
    };
    assert_eq!(rows.len(), 1, "one subject bound once must answer one row");
}
