// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Actual allocator traffic distinguishes suppressed local expansion from merely
//! suppressed callbacks after a nested negative guard refuses its next credit.

use std::sync::Mutex;

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_datalog::clause::{ClauseAtom, ClauseTerm, DlClause};
use purrdf_datalog::guard::{Guard, GuardCall, GuardEvaluator, GuardReads, Negation};
use purrdf_datalog::seminaive::{
    BudgetResource, EvalError, EvalOptions, compile, evaluate_guarded,
};
use purrdf_datalog::store::RelationStore;

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

struct Answers {
    /// Constructed before opening the allocation window, then moved to the caller.
    rows: Mutex<Option<Vec<Vec<String>>>>,
}

impl GuardEvaluator for Answers {
    fn evaluate(&self, call: &GuardCall<'_>) -> Result<Vec<Vec<String>>, String> {
        match call.guard.name() {
            "expand" => self
                .rows
                .lock()
                .expect("unpoisoned fixture")
                .take()
                .ok_or_else(|| "the one positive source invoked expand twice".to_owned()),
            "reject" => Ok(Vec::new()),
            other => Err(format!("unexpected guard {other}")),
        }
    }
}

#[test]
fn nested_refusal_stops_owned_local_expansion_and_preserves_exact_boundaries() {
    const ROWS: u64 = 128;
    const SURFACE_BYTES: usize = 65536;
    let rule = DlClause::datalog(
        ClauseAtom::positive(
            ClauseTerm::var("?s"),
            "https://example.org/result",
            ClauseTerm::var("?o"),
        ),
        vec![ClauseAtom::positive(
            ClauseTerm::var("?s"),
            "https://example.org/source",
            ClauseTerm::var("?o"),
        )],
    )
    .with_negations(vec![Negation::new(
        Vec::new(),
        vec![
            Guard::new(
                "expand",
                vec!["?s".to_owned()],
                vec!["?local".to_owned()],
                GuardReads::Bindings,
            ),
            Guard::filter("reject", vec!["?local".to_owned()]),
        ],
    )]);
    let program = compile(vec![rule]).expect("stratified fixture");
    let mut input = RelationStore::new();
    input.insert(
        "<https://example.org/s>",
        "<https://example.org/source>",
        "<https://example.org/o>",
        RelationStore::DEFAULT_GRAPH,
    );
    let surface = format!("<https://example.org/{}>", "x".repeat(SURFACE_BYTES));
    // One positive row, a negative frame, the first callback, and all its rows.
    // The nested callback's first reservation is then the refusal sentinel.
    let nested_boundary = ROWS + 3;
    let exact_boundary = 2 * ROWS + 3;
    for ceiling in [0, nested_boundary, exact_boundary - 1, exact_boundary] {
        let answers = Answers {
            rows: Mutex::new(Some((0..ROWS).map(|_| vec![surface.clone()]).collect())),
        };
        let seeded = input.clone();
        let options = EvalOptions::default().with_max_join_steps(ceiling);
        let window = CurrentThreadWindow::open();
        let result = evaluate_guarded(&program, seeded, &answers, &options, None);
        let measurement = window.close();
        if ceiling == exact_boundary {
            let model = result.expect("exact boundary admits every rejecting local row");
            assert_eq!(model.budget().join_steps(), exact_boundary);
            assert_eq!(model.facts().row_count(), 2);
        } else {
            let error = result.expect_err("total refusal before any derived fact commits");
            let EvalError::BudgetExhausted { resource, report } = error else {
                panic!("{error:?}");
            };
            assert_eq!(resource, BudgetResource::JoinSteps);
            assert_eq!(report.join_steps(), ceiling + 1);
            assert_eq!(report.stored_facts(), 1);
        }
        if ceiling == nested_boundary {
            // The reached first local must really have been cloned. The other 127
            // would request over 8 MiB on the defective post-refusal path. This
            // threshold includes modest evaluator bookkeeping, not caller output.
            assert!(
                measurement.requested_bytes >= SURFACE_BYTES as u64,
                "{measurement:?}"
            );
            assert!(
                measurement.requested_bytes < 4 * SURFACE_BYTES as u64,
                "post-refusal local expansion: {measurement:?}"
            );
        }
    }
}
