// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Prepared operation execution preserves binding, completeness and shared budgets.

use std::sync::{Arc, Barrier};

use purrdf_core::{
    InMemoryPageProvider, PageGeneration, PagedDataset, PagedQueryLimits, RdfDataset,
    RdfDatasetBuilder, ResourceDimension, SparqlRequest, SparqlResult, StopCause, TermValue,
    TrippedGovernor,
};
use purrdf_sparql_eval::governor::GovernorState;
use purrdf_sparql_eval::{
    CancellationFlag, FallibleSparqlError, GovernedOutcome, HttpRemoteQuerySource, HttpRequest,
    NativeSparqlEngine, QueryGovernors, QueryOptions, RemoteError,
};

const SELECT: &str =
    "SELECT ?this ?value WHERE { ?this <http://example.org/p> ?value } ORDER BY ?this";

fn data() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri("http://example.org/p");
    let value = builder.intern_iri("http://example.org/value");
    for name in ["a", "b", "c"] {
        let subject = builder.intern_iri(&format!("http://example.org/{name}"));
        builder.push_quad(subject, predicate, value, None);
    }
    builder.freeze().unwrap()
}

fn pages() -> PagedDataset {
    PagedDataset::from_provider(Arc::new(InMemoryPageProvider::with_byte_lengths(
        vec![(data(), 101)],
        PageGeneration(7),
    )))
    .unwrap()
}

fn metered() -> Arc<GovernorState> {
    Arc::new(GovernorState::new(&QueryGovernors::METERED))
}

fn equal_results(left: SparqlResult, right: SparqlResult) {
    match (left, right) {
        (SparqlResult::Boolean(a), SparqlResult::Boolean(b)) => assert_eq!(a, b),
        (
            SparqlResult::Solutions {
                variables: av,
                rows: ar,
                ..
            },
            SparqlResult::Solutions {
                variables: bv,
                rows: br,
                ..
            },
        ) => {
            assert_eq!(av, bv);
            assert_eq!(ar, br);
        }
        (SparqlResult::Graph(a), SparqlResult::Graph(b)) => {
            assert_eq!(a.quads().collect::<Vec<_>>(), b.quads().collect::<Vec<_>>());
            assert_eq!(a.term_count(), b.term_count());
        }
        (a, b) => panic!("query form changed: {a:?} versus {b:?}"),
    }
}

#[test]
fn prepared_operations_match_text_with_shacl_prebinding_for_each_result_form() {
    let engine = NativeSparqlEngine::new();
    let dataset = data();
    let options = QueryOptions {
        prebinding: purrdf_sparql_eval::ShaclPrebinding::Applied,
        ..QueryOptions::EMPTY
    };
    for query in [
        SELECT,
        "ASK { ?this <http://example.org/p> ?value }",
        "CONSTRUCT { ?this <http://example.org/result> ?value } WHERE { ?this <http://example.org/p> ?value }",
    ] {
        let prepared = engine.prepare_query(query, None).unwrap();
        for name in ["a", "missing"] {
            let substitutions = [(
                "this".into(),
                TermValue::Iri(format!("http://example.org/{name}")),
            )];
            let text_state = metered();
            let prepared_state = metered();
            let text = engine
                .query_governed_in_operation(
                    &*dataset,
                    SparqlRequest {
                        query,
                        base_iri: None,
                        substitutions: &substitutions,
                    },
                    options,
                    &text_state,
                )
                .unwrap();
            let plan = engine
                .query_prepared_governed_in_operation(
                    &*dataset,
                    &prepared,
                    &substitutions,
                    options,
                    &prepared_state,
                )
                .unwrap();
            let (
                GovernedOutcome::Complete {
                    result: a,
                    evidence: ae,
                    ..
                },
                GovernedOutcome::Complete {
                    result: b,
                    evidence: be,
                    ..
                },
            ) = (text, plan)
            else {
                panic!("metered operation must complete");
            };
            equal_results(a, b);
            assert_eq!(ae, be, "preparation cannot change execution charges");
        }
    }
}

#[test]
fn shared_prepared_fallible_operations_keep_both_meters_and_certified_prefixes() {
    let paged = pages();
    let engine = NativeSparqlEngine::new();
    let prepared = engine.prepare_query(SELECT, None).unwrap();
    let state = metered();
    for name in ["a", "b"] {
        let view = paged.query_view(PagedQueryLimits::new(1, 101));
        let substitutions = [(
            "this".into(),
            TermValue::Iri(format!("http://example.org/{name}")),
        )];
        let result = engine
            .query_prepared_governed_fallible_in_operation(
                &view,
                &prepared,
                &substitutions,
                QueryOptions::EMPTY,
                &state,
            )
            .unwrap();
        assert!(matches!(result.result, SparqlResult::Solutions { rows, .. } if rows.len() == 1));
        assert_eq!(result.evidence.view.consumed_bytes, 101);
        assert_eq!(result.evidence.view.consumed_pages, 1);
        assert!(result.evidence.governors.is_complete());
        assert_eq!(result.evidence.governors, state.evidence());
    }
    let view = paged.query_view(PagedQueryLimits::UNBOUNDED);
    let limited = Arc::new(GovernorState::new(
        &QueryGovernors::METERED.with_max_answers(2),
    ));
    let error = engine
        .query_prepared_governed_fallible_in_operation(
            &view,
            &prepared,
            &[],
            QueryOptions::EMPTY,
            &limited,
        )
        .unwrap_err();
    let FallibleSparqlError::BudgetExhausted {
        tripped,
        partial,
        evidence,
    } = error
    else {
        panic!("expected typed exhaustion");
    };
    assert_eq!(
        tripped,
        TrippedGovernor::Budget {
            dimension: ResourceDimension::AnswerRows,
            limit: 2,
            consumed: 3
        }
    );
    assert!(
        matches!(partial.result().unwrap().result(), SparqlResult::Solutions { rows, .. } if rows.len() == 2)
    );
    assert_eq!(evidence.view.consumed_pages, 1);
    assert_eq!(evidence.governors, limited.evidence());
}

#[test]
fn fallible_prepared_checkpoints_discard_answers_and_preserve_operational_precedence() {
    let paged = pages();
    let view = paged.query_view(PagedQueryLimits::new(0, 0));
    let engine = NativeSparqlEngine::new();
    let query = "SELECT * WHERE { { ?s ?p ?o } UNION { SERVICE <http://example.org/service> { ?a ?b ?c } } }";
    let prepared = engine.prepare_query(query, None).unwrap();
    let error = engine
        .query_prepared_governed_fallible_in_operation(
            &view,
            &prepared,
            &[],
            QueryOptions::EMPTY,
            &metered(),
        )
        .unwrap_err();
    assert!(
        matches!(error, FallibleSparqlError::Operational { .. }),
        "page failure must outrank the independent SERVICE error: {error:?}"
    );
    assert!(error.partial_answers().is_none());
    assert!(error.diagnostic().is_none());

    // The failed view is now latched. Its preflight must outrank both a cancelled
    // governor and caller-mutated malformed algebra without charging execution.
    let mut malformed = purrdf_sparql_eval::PreparedQuery::rewritten(
        purrdf_sparql_algebra::SparqlParser::new()
            .parse_query("ASK {}")
            .unwrap(),
        QueryOptions::EMPTY,
    )
    .unwrap();
    if let purrdf_sparql_algebra::Query::Ask { pattern, .. } = &mut malformed.query {
        *pattern = purrdf_sparql_algebra::GraphPattern::Values {
            variables: vec![],
            bindings: vec![vec![None]],
        };
    }
    let stop = Arc::new(CancellationFlag::new());
    stop.cancel();
    let cancelled = Arc::new(GovernorState::new(
        &QueryGovernors::METERED.with_stop_signal(stop),
    ));
    let error = engine
        .query_prepared_governed_fallible_in_operation(
            &view,
            &malformed,
            &[],
            QueryOptions::EMPTY,
            &cancelled,
        )
        .unwrap_err();
    assert!(matches!(error, FallibleSparqlError::Operational { .. }));
    assert_eq!(cancelled.evidence().consumed_in(ResourceDimension::Fuel), 0);
}

#[test]
fn remote_cancellation_prevents_prepared_fallible_publication() {
    let paged = pages();
    let view = paged.query_view(PagedQueryLimits::UNBOUNDED);
    let engine = NativeSparqlEngine::new();
    let query =
        "SELECT ?remote WHERE { SERVICE <http://example.org/service> { BIND(\"ok\" AS ?remote) } }";
    let prepared = engine.prepare_query(query, None).unwrap();
    let stop = Arc::new(CancellationFlag::new());
    let state = Arc::new(GovernorState::new(
        &QueryGovernors::METERED.with_stop_signal(stop.clone()),
    ));
    let source = HttpRemoteQuerySource::new(
        move |_: HttpRequest<'_>| -> Result<Vec<u8>, RemoteError> {
            stop.cancel();
            Ok(br#"{"head":{"vars":["remote"]},"results":{"bindings":[{"remote":{"type":"literal","value":"ok"}}]}}"#.to_vec())
        },
    );
    let error = engine
        .query_prepared_governed_fallible_with_source_in_operation(
            &view,
            &prepared,
            &[],
            &source,
            QueryOptions::EMPTY,
            &state,
        )
        .unwrap_err();
    assert_eq!(
        error.tripped(),
        Some(TrippedGovernor::Stopped {
            cause: StopCause::Cancelled
        })
    );
    assert!(matches!(error, FallibleSparqlError::BudgetExhausted { .. }));
    assert!(state.evidence().tripped.is_some());
}

#[test]
fn simultaneously_started_worker_local_engines_share_one_plan_and_fuel_ceiling() {
    let dataset = data();
    let engine = NativeSparqlEngine::new();
    let prepared = engine.prepare_query(SELECT, None).unwrap();
    let measured = metered();
    engine
        .query_prepared_governed_in_operation(
            &*dataset,
            &prepared,
            &[],
            QueryOptions::EMPTY,
            &measured,
        )
        .unwrap();
    let cost = measured.evidence().consumed_in(ResourceDimension::Fuel);
    assert!(cost > 0);
    let shared = Arc::new(GovernorState::new(&QueryGovernors::METERED.with_fuel(cost)));
    let barrier = Barrier::new(4);
    let results = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|_| {
                scope.spawn(|| {
                    let worker = NativeSparqlEngine::new();
                    barrier.wait();
                    let result = worker
                        .query_prepared_governed_in_operation(
                            &*dataset,
                            &prepared,
                            &[],
                            QueryOptions::EMPTY,
                            &shared,
                        )
                        .unwrap();
                    assert_eq!(worker.plan_cache_stats().misses, 0);
                    result
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert!(
        results
            .iter()
            .filter(|result| matches!(result, GovernedOutcome::Complete { .. }))
            .count()
            <= 1
    );
    assert!(shared.evidence().tripped.is_some());
    assert!(shared.evidence().consumed_in(ResourceDimension::Fuel) >= cost);
    assert_eq!(engine.plan_memory_observer().stats().live_plans, 1);
}
