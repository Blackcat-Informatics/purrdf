// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Actual allocator evidence over independent production factors. Input creation
//! and compilation are outside each window; complete evaluation remains inside.

use purrdf_alloc_probe::{CountingAllocator, WholeProcessWindow};
use purrdf_datalog::clause::{ClauseAtom, ClauseTerm, DlClause};
use purrdf_datalog::seminaive::compile;
use purrdf_datalog::store::RelationStore;

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[test]
fn independent_existential_factor_work_and_allocation_grow_additively() {
    let atom = |variable: &str, predicate: &str| {
        ClauseAtom::positive(
            ClauseTerm::var(variable),
            predicate,
            ClauseTerm::iri("https://example.org/value"),
        )
    };
    let program = compile(vec![DlClause::datalog(
        atom("?x", "https://example.org/result"),
        vec![
            atom("?y", "https://example.org/b"),
            atom("?x", "https://example.org/a"),
        ],
    )])
    .expect("factor fixture compiles");
    let mut previous = None;
    for n in [100_usize, 1_000, 10_000] {
        let mut input = RelationStore::new();
        for i in 0..n {
            for predicate in ["a", "b"] {
                input
                    .insert(
                        &format!("<https://example.org/s{i:05}>"),
                        &format!("<https://example.org/{predicate}>"),
                        "<https://example.org/value>",
                        RelationStore::DEFAULT_GRAPH,
                    )
                    .unwrap();
            }
        }
        let model_input = input.clone();
        let window = WholeProcessWindow::open();
        let evaluation = purrdf_datalog::seminaive::evaluate_guarded(
            &program,
            input,
            &purrdf_datalog::guard::NoGuards,
            &purrdf_datalog::seminaive::EvalOptions::default()
                .with_derivation_mode(purrdf_datalog::seminaive::DerivationMode::Record),
            None,
        )
        .expect("default additive work succeeds");
        let allocation = window.close();
        let window = WholeProcessWindow::open();
        let model = purrdf_datalog::seminaive::evaluate(&program, model_input).unwrap();
        let model_allocation = window.close();
        assert!(model.derivations().is_none());
        assert_eq!(model.work(), evaluation.work());
        assert_eq!(
            model.facts().facts_sorted(),
            evaluation.facts().facts_sorted()
        );
        assert!(model_allocation.requested_bytes < allocation.requested_bytes);
        assert!(model_allocation.retained_bytes < allocation.retained_bytes);
        assert_eq!(
            evaluation
                .derivations()
                .expect("derivation recording was requested")
                .len(),
            n
        );
        assert_eq!(evaluation.budget().join_steps(), 3 * n as u64);
        assert!(allocation.requested_bytes > 0);
        assert!(allocation.peak_working_bytes > 0);
        if let Some((traffic, peak)) = previous {
            // Tenfold independent-input growth permits allocator/table thresholds;
            // quadratic products grow one hundredfold and violate both bounds.
            assert!(allocation.requested_bytes < 20 * traffic);
            assert!(allocation.peak_working_bytes < 20 * peak);
        }
        eprintln!(
            "independent_factors n={n} join_steps={} requested_bytes={} retained_bytes={} peak_working_bytes={}",
            evaluation.budget().join_steps(),
            allocation.requested_bytes,
            allocation.retained_bytes,
            allocation.peak_working_bytes
        );
        previous = Some((allocation.requested_bytes, allocation.peak_working_bytes));
    }
}
