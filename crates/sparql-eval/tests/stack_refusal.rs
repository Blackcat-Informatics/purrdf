// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A prepared correlated query retains the native small-stack refusal and the
//! WASM worklist answer, then refuses below each physical margin. The
//! shared runner executes this test natively and on actual WebAssembly.

use purrdf_core::{RdfDatasetBuilder, SparqlResult, TermValue};
use purrdf_sparql_eval::{
    EvalError, GovernedOutcome, NativeSparqlEngine, QueryGovernors, QueryOptions,
};

fn prepared_evaluation_refuses_the_actual_smaller_stack_without_poisoning_the_caller() {
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri("http://example.org/p").unwrap();
    let probe = builder.intern_iri("http://example.org/q").unwrap();
    for index in 1..=4 {
        let subject = builder
            .intern_iri(&format!("http://example.org/s{index}"))
            .unwrap();
        let object = builder
            .intern_iri(&format!("http://example.org/o{index}"))
            .unwrap();
        builder.push_quad(subject, predicate, object, None).unwrap();
        if index == 1 {
            builder.push_quad(subject, probe, object, None).unwrap();
        }
    }
    let dataset = builder.freeze().expect("valid resident fixture");
    let engine = NativeSparqlEngine::new();
    // Below the unchanged host-depth ceiling. Native and WASM frames and margins
    // differ on the smaller stack. An even number of negations selects only s1.
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
                    &QueryOptions::EMPTY,
                    &QueryGovernors::METERED,
                )?;
                let GovernedOutcome::Complete { result, .. } = outcome else {
                    panic!("metering without ceilings must not stop this query");
                };
                Ok(result)
            } else {
                engine.query_prepared(&dataset, &prepared, &[], &QueryOptions::EMPTY)
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
        let smaller = purrdf_stack::on_stack_scoped(192 * 1024, || {
            let scoped_engine = NativeSparqlEngine::new();
            let result = evaluate(&scoped_engine);
            let canary_result =
                scoped_engine.query_prepared(&dataset, &canary, &[], &QueryOptions::EMPTY);
            // Native fixed frames may consume the remaining margin even for a
            // shallow query. Recovery must succeed on the caller's stack below.
            #[cfg(not(target_arch = "wasm32"))]
            match canary_result {
                Ok(answer) => assert_answer(answer),
                Err(error) => assert_eq!(error.code, EvalError::STACK_EXHAUSTED_CODE),
            }
            #[cfg(target_arch = "wasm32")]
            assert_answer(
                canary_result.expect("the WASM smaller stack still answers its shallow canary"),
            );
            result
        })
        .expect("the smaller stack scope is admitted");
        // The native margin is 128 KiB and the WASM shadow-stack margin 64 KiB.
        // Preserve each actual physical boundary without changing either guard.
        #[cfg(not(target_arch = "wasm32"))]
        assert_eq!(
            smaller
                .expect_err("the native smaller stack retains its typed refusal")
                .code,
            EvalError::STACK_EXHAUSTED_CODE,
        );
        #[cfg(target_arch = "wasm32")]
        assert_answer(smaller.expect("the worklist query answers on the WASM smaller stack"));
        let refused = purrdf_stack::on_stack_scoped(purrdf_stack::MARGIN_BYTES / 2, || {
            evaluate(&NativeSparqlEngine::new())
        })
        .expect("the below-margin stack scope is admitted")
        .expect_err("the actual stack guard refuses before a partial answer escapes");
        assert_eq!(refused.code, EvalError::STACK_EXHAUSTED_CODE);
        // The caller's stack context is restored after each typed refusal.
        assert_answer(
            engine
                .query_prepared(&dataset, &canary, &[], &QueryOptions::EMPTY)
                .expect("the caller's shallow canary answers after refusal"),
        );
        assert_answer(
            evaluate(&engine).expect("the same engine still answers on the caller's stack"),
        );
    }
}

purrdf_testkit::harness_main!(
    prepared_evaluation_refuses_the_actual_smaller_stack_without_poisoning_the_caller,
);
