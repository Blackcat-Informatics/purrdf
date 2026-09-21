// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A prepared, parameterized execution: prepare once, bind and run many times.
//!
//! The per-execution allocation pin lives here, in the crate that owns the property,
//! rather than only downstream. Every other assertion about this path is in
//! `purrdf-shapes`' allocation suite, which means a change to the evaluator that
//! regressed it would redden a different crate's test — or, if those fixtures ever
//! drifted, nothing at all.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use purrdf_alloc_probe::{CountingAllocator, WholeProcessWindow};
use purrdf_core::{RdfDataset, RdfDatasetBuilder, TermValue};
use purrdf_sparql_eval::{InternedOutcome, NativeSparqlEngine, QueryOptions};

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
fn re_running_a_prepared_execution_costs_no_setup() {
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

    let first = run(2);
    let second = run(3);
    let third = run(4);
    println!("prepared execution: {first}, {second}, {third} allocations per run");
    assert_eq!(
        (first, second),
        (second, third),
        "a prepared execution's cost must not depend on how many times it has run; \
         a figure that moves between runs means per-run state is accumulating"
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
