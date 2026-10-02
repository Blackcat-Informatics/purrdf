// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A prepared correlated query answers on the caller's stack and refuses on a
//! smaller physical stack, with the same typed boundary under metering. The
//! shared runner executes this test natively and on actual WebAssembly.

use purrdf_core::{RdfDatasetBuilder, SparqlResult, TermValue};
use purrdf_sparql_eval::{
    EvalError, GovernedOutcome, NativeSparqlEngine, QueryGovernors, QueryOptions,
};

fn prepared_evaluation_refuses_the_actual_smaller_stack_without_poisoning_the_caller() {
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri("http://example.org/p");
    let probe = builder.intern_iri("http://example.org/q");
    for index in 1..=4 {
        let subject = builder.intern_iri(&format!("http://example.org/s{index}"));
        let object = builder.intern_iri(&format!("http://example.org/o{index}"));
        builder.push_quad(subject, predicate, object, None);
        if index == 1 {
            builder.push_quad(subject, probe, object, None);
        }
    }
    let dataset = builder.freeze().expect("valid resident fixture");
    let engine = NativeSparqlEngine::new();
    // Below the unchanged host-depth ceiling, but tall enough to consume the
    // smaller stack's headroom. An even number of negations selects only s1.
    let depth = 80;
    let query = format!(
        "SELECT ?s WHERE {{ {}?s <http://example.org/q> ?z{} }}",
        "?s <http://example.org/p> ?o FILTER NOT EXISTS { ".repeat(depth),
        " }".repeat(depth),
    );
    let prepared = engine
        .prepare_query(&query, None)
        .expect("preparation on the caller's stack");
    let canary = engine
        .prepare_query("SELECT ?s WHERE { ?s <http://example.org/q> ?z }", None)
        .expect("the shallow canary prepares");

    for governed in [false, true] {
        let evaluate = |engine: &NativeSparqlEngine| {
            if governed {
                let outcome = engine.query_prepared_governed_view(
                    dataset.as_ref(),
                    &prepared,
                    &[],
                    QueryOptions::EMPTY,
                    &QueryGovernors::METERED,
                )?;
                let GovernedOutcome::Complete { result, .. } = outcome else {
                    panic!("metering without ceilings must not stop this query");
                };
                Ok(result)
            } else {
                engine.query_prepared(&dataset, &prepared, &[], QueryOptions::EMPTY)
            }
        };
        let assert_answer = |answer: SparqlResult| {
            let SparqlResult::Solutions {
                variables, rows, ..
            } = answer
            else {
                panic!("SELECT must return solutions");
            };
            assert_eq!(variables, ["s"]);
            assert_eq!(
                rows,
                [vec![Some(TermValue::Iri("http://example.org/s1".into()))]],
            );
        };
        assert_answer(evaluate(&engine).expect("the query answers on the caller's stack"));
        // Engines contain caller-selected resolvers and are not Send. Carry the
        // prepared plan into a fresh engine on the actual evaluating stack.
        let refused = purrdf_stack::on_stack_scoped(192 * 1024, || {
            let scoped_engine = NativeSparqlEngine::new();
            let result = evaluate(&scoped_engine);
            assert_answer(
                scoped_engine
                    .query_prepared(&dataset, &canary, &[], QueryOptions::EMPTY)
                    .expect("the refused engine still answers its shallow canary"),
            );
            result
        })
        .expect("the smaller stack scope is admitted")
        .expect_err("the actual stack guard refuses before a partial answer escapes");
        assert_eq!(refused.code, EvalError::STACK_EXHAUSTED_CODE);
        // The caller's stack context is restored after each typed refusal.
        assert_answer(
            evaluate(&engine).expect("the same engine still answers on the caller's stack"),
        );
    }
}

purrdf_testkit::harness_main!(
    prepared_evaluation_refuses_the_actual_smaller_stack_without_poisoning_the_caller,
);
