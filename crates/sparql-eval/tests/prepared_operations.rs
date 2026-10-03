// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Prepared operation execution preserves binding, completeness and shared budgets.

mod support;

use support::three_subjects_one_value;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};

use purrdf_core::{
    InMemoryPageProvider, PageGeneration, PagedDataset, PagedQueryEvidence, PagedQueryLimits,
    ResourceDimension, SparqlRequest, SparqlResult, StopCause, TermValue, TrippedGovernor,
};
use purrdf_sparql_algebra::{
    Child, GraphPattern, NamedNode, NamedNodePattern, Query, QueryDataset, TermPattern,
    TriplePattern, Variable,
};
use purrdf_sparql_eval::governor::GovernorState;
use purrdf_sparql_eval::{
    CancellationFlag, FallibleSparqlError, GovernedEvidence, GovernedOutcome,
    HttpRemoteQuerySource, HttpRequest, NativeSparqlEngine, PreparedQuery, QueryGovernors,
    QueryOptions, RemoteError, ShaclPrebinding,
};

const SELECT: &str =
    "SELECT ?this ?value WHERE { ?this <http://example.org/p> ?value } ORDER BY ?this";

fn pages() -> PagedDataset {
    PagedDataset::from_provider(Arc::new(InMemoryPageProvider::with_byte_lengths(
        vec![(three_subjects_one_value(), 101)],
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
    let dataset = three_subjects_one_value();
    let options = QueryOptions::new().with_prebinding(ShaclPrebinding::Applied);
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
fn prepared_fallible_calls_match_text_and_shared_receipts_for_every_query_form() {
    let paged = pages();
    let engine = NativeSparqlEngine::new();
    for query in [
        SELECT,
        "ASK { ?this <http://example.org/p> ?value }",
        "CONSTRUCT { ?this <http://example.org/result> ?value } WHERE { ?this <http://example.org/p> ?value }",
        "DESCRIBE ?this WHERE { ?this <http://example.org/p> ?value }",
    ] {
        let prepared = engine.prepare_query(query, None).unwrap();
        for prebinding in [ShaclPrebinding::None, ShaclPrebinding::Applied] {
            let options = QueryOptions::new().with_prebinding(prebinding);
            for name in [None, Some("a"), Some("missing")] {
                let substitutions: Vec<_> = name
                    .map(|name| ("this".into(), support::iri(name)))
                    .into_iter()
                    .collect();
                let text = engine
                    .query_governed_fallible_view(
                        &paged.query_view(PagedQueryLimits::UNBOUNDED),
                        SparqlRequest {
                            query,
                            base_iri: None,
                            substitutions: &substitutions,
                        },
                        options,
                        &QueryGovernors::METERED,
                    )
                    .unwrap();
                let plan = engine
                    .query_prepared_governed_fallible_view(
                        &paged.query_view(PagedQueryLimits::UNBOUNDED),
                        &prepared,
                        &substitutions,
                        options,
                        &QueryGovernors::METERED,
                    )
                    .unwrap();
                let state = metered();
                let shared = engine
                    .query_prepared_governed_fallible_in_operation(
                        &paged.query_view(PagedQueryLimits::UNBOUNDED),
                        &prepared,
                        &substitutions,
                        options,
                        &state,
                    )
                    .unwrap();
                assert_eq!(
                    text.evidence.view, plan.evidence.view,
                    "{query}: {prebinding:?}, {name:?}"
                );
                assert_eq!(text.evidence.governors, plan.evidence.governors);
                assert_eq!(plan.evidence.view, shared.evidence.view);
                assert_eq!(plan.evidence.governors, shared.evidence.governors);
                assert_eq!(shared.evidence.governors, state.evidence());
                equal_results(text.result, plan.result.clone());
                equal_results(plan.result, shared.result);
            }
        }
    }
}

#[test]
fn compiler_constructed_plans_execute_without_text_or_cache_activity() {
    let query = Query::Select {
        pattern: GraphPattern::Project {
            inner: Child::new(GraphPattern::Bgp {
                patterns: vec![TriplePattern {
                    subject: TermPattern::Variable(Variable::new("this")),
                    predicate: NamedNodePattern::NamedNode(
                        NamedNode::new("http://example.org/p").unwrap(),
                    ),
                    object: TermPattern::Variable(Variable::new("value")),
                }],
            }),
            variables: vec![Variable::new("this"), Variable::new("value")],
        },
        dataset: QueryDataset::default(),
        base_iri: None,
        version: None,
    };
    let prepared = PreparedQuery::rewritten(query, QueryOptions::EMPTY).unwrap();
    let paged = pages();
    let worker = NativeSparqlEngine::new();
    let answer = worker
        .query_prepared_governed_fallible_view(
            &paged.query_view(PagedQueryLimits::UNBOUNDED),
            &prepared,
            &[],
            QueryOptions::EMPTY,
            &QueryGovernors::METERED,
        )
        .unwrap();
    let text = NativeSparqlEngine::new()
        .query_governed_fallible_view(
            &paged.query_view(PagedQueryLimits::UNBOUNDED),
            SparqlRequest {
                query: "SELECT ?this ?value WHERE { ?this <http://example.org/p> ?value }",
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions::EMPTY,
            &QueryGovernors::METERED,
        )
        .unwrap();
    assert_eq!(worker.plan_cache_stats().misses, 0);
    assert_eq!(worker.plan_cache_stats().hits, 0);
    assert_eq!(worker.cached_plan_count(), 0);
    assert_eq!(support::row_count(&answer.result), 3);
    assert_eq!(answer.evidence.view, text.evidence.view);
    assert_eq!(answer.evidence.governors, text.evidence.governors);
    equal_results(answer.result, text.result);
}

#[test]
fn repeated_prepared_calls_own_independent_budgets_and_certified_answers() {
    let paged = pages();
    let engine = NativeSparqlEngine::new();
    let prepared = engine.prepare_query(SELECT, None).unwrap();
    let governors = QueryGovernors::METERED.with_max_answers(2);
    let mut prior: Option<(SparqlResult, GovernedEvidence<PagedQueryEvidence>)> = None;
    for _ in 0..2 {
        let error = engine
            .query_prepared_governed_fallible_view(
                &paged.query_view(PagedQueryLimits::UNBOUNDED),
                &prepared,
                &[],
                QueryOptions::EMPTY,
                &governors,
            )
            .unwrap_err();
        let FallibleSparqlError::BudgetExhausted {
            tripped,
            partial,
            evidence,
        } = error
        else {
            panic!("a healthy view must report its exhausted governor");
        };
        assert_eq!(
            tripped,
            TrippedGovernor::Budget {
                dimension: ResourceDimension::AnswerRows,
                limit: 2,
                consumed: 3,
            }
        );
        let certified = partial.result().expect("the admitted rows are certified");
        assert!(partial.is_certain());
        assert!(certified.is_positional_prefix());
        assert_eq!(support::row_count(certified.result()), 2);
        assert_eq!(evidence.view.consumed_pages, 1);
        assert_eq!(evidence.view.consumed_bytes, 101);
        assert_eq!(evidence.governors.tripped, Some(tripped));
        if let Some((old_result, old_evidence)) = prior.take() {
            assert_eq!(evidence.view, old_evidence.view);
            assert_eq!(
                evidence.governors, old_evidence.governors,
                "per-call charges cannot accumulate"
            );
            equal_results(certified.result().clone(), old_result);
        }
        prior = Some((certified.result().clone(), evidence));
    }
}

#[test]
fn shared_prepared_fallible_operations_keep_both_meters_and_certified_prefixes() {
    let paged = pages();
    let engine = NativeSparqlEngine::new();
    let prepared = engine.prepare_query(SELECT, None).unwrap();
    let state = metered();
    let mut prior = state.evidence();
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
        let fuel = result
            .evidence
            .governors
            .consumed_in(ResourceDimension::Fuel);
        assert!(
            fuel > prior.consumed_in(ResourceDimension::Fuel),
            "each quiescent operation adds to the shared meter"
        );
        for dimension in ResourceDimension::ALL {
            assert!(
                result.evidence.governors.consumed_in(dimension) >= prior.consumed_in(dimension),
                "shared operation consumption cannot regress in {dimension:?}"
            );
        }
        prior = result.evidence.governors;
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
    // governor and an admission failure without charging execution.
    //
    // The admission failure used to be forged: an "ASK {}" plan whose public
    // `query` field was overwritten by hand with a malformed zero-arity `VALUES`
    // row, bypassing admission entirely. `PreparedQuery::query` is now a private
    // field with no setter (see `crates/sparql-eval/src/engine.rs`), so that
    // construction no longer compiles — and, as
    // `crates/sparql-eval/tests/prepared_admission.rs`'s
    // `malformed_compiler_rows_are_refused_before_evaluation` already shows, this
    // exact malformed row is refused by every legitimate constructor, so no
    // `PreparedQuery` carrying it could ever have reached this call. A REAL
    // admission failure reached legitimately stands in its place: a plan admitted
    // under a property-function registry, then handed to this call under
    // `QueryOptions::EMPTY` — the same registry-mismatch shape
    // `check_plan_matches_relations` refuses on every non-`PreparedExecution`
    // entry (see `crates/sparql-eval/tests/prepared_execution.rs`'s
    // `executing_a_prepared_plan_under_a_mismatched_property_function_registry_is_refused_…`).
    // What this test needs from it is unchanged: an admission failure to outrank
    // with the latched view's operational one.
    let mut registry = purrdf_sparql_eval::PropertyFunctionRegistry::new();
    registry.register(
        "http://example.org/relation",
        Arc::new(purrdf_sparql_eval::MemoryRelation::new(1, 1, vec![]).unwrap()),
    );
    let relation_env = purrdf_sparql_eval::ExtensionEnv::over_relations(registry)
        .expect("the fixture relation declares without panicking");
    let relation_options = QueryOptions::new().with_env(&relation_env);
    let admission_mismatched = engine
        .prepare_query_with_options(
            "ASK { ?s <http://example.org/relation> ?o }",
            None,
            relation_options,
        )
        .expect("admits under the relation registry");
    let stop = Arc::new(CancellationFlag::new());
    stop.cancel();
    let governors = QueryGovernors::METERED.with_stop_signal(stop);
    let cancelled = Arc::new(GovernorState::new(&governors));
    let error = engine
        .query_prepared_governed_fallible_in_operation(
            &view,
            &admission_mismatched,
            &[],
            QueryOptions::EMPTY,
            &cancelled,
        )
        .unwrap_err();
    assert!(matches!(error, FallibleSparqlError::Operational { .. }));
    assert_eq!(cancelled.evidence().consumed_in(ResourceDimension::Fuel), 0);
    let per_call = engine
        .query_prepared_governed_fallible_view(
            &view,
            &admission_mismatched,
            &[],
            QueryOptions::EMPTY,
            &governors,
        )
        .unwrap_err();
    assert_eq!(per_call.operational_error(), error.operational_error());
    assert!(per_call.partial_answers().is_none());
    assert!(per_call.diagnostic().is_none());
    let FallibleSparqlError::Operational { evidence, .. } = per_call else {
        panic!("latched failure outranks cancelled and mismatched execution");
    };
    assert_eq!(evidence.governors.consumed_in(ResourceDimension::Fuel), 0);
}

#[test]
fn healthy_prepared_views_keep_query_errors_and_registry_admission_typed() {
    let paged = pages();
    let engine = NativeSparqlEngine::new();
    let query =
        "SELECT ?remote WHERE { SERVICE <http://example.org/service> { BIND(\"ok\" AS ?remote) } }";
    let prepared = engine.prepare_query(query, None).unwrap();
    let view = paged.query_view(PagedQueryLimits::UNBOUNDED);
    let state = metered();
    for result in [
        engine.query_governed_fallible_view(
            &view,
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions::EMPTY,
            &QueryGovernors::METERED,
        ),
        engine.query_prepared_governed_fallible_view(
            &view,
            &prepared,
            &[],
            QueryOptions::EMPTY,
            &QueryGovernors::METERED,
        ),
        engine.query_prepared_governed_fallible_in_operation(
            &view,
            &prepared,
            &[],
            QueryOptions::EMPTY,
            &state,
        ),
    ] {
        let error = result.unwrap_err();
        assert!(error.operational_error().is_none());
        assert!(error.partial_answers().is_none());
        let FallibleSparqlError::Query {
            diagnostic,
            evidence,
        } = error
        else {
            panic!("a healthy SERVICE view retains its evaluator diagnostic");
        };
        assert_eq!(
            diagnostic.code,
            purrdf_sparql_eval::EvalError::SERVICE_UNCONFIGURED_CODE
        );
        assert!(evidence.governors.is_complete());
    }

    let mut registry = purrdf_sparql_eval::PropertyFunctionRegistry::new();
    registry.register(
        "http://example.org/relation",
        Arc::new(
            purrdf_sparql_eval::MemoryRelation::new(
                1,
                1,
                vec![vec![support::iri("a"), support::iri("value")]],
            )
            .unwrap(),
        ),
    );
    let env = purrdf_sparql_eval::ExtensionEnv::over_relations(registry).unwrap();
    let options = QueryOptions::new().with_env(&env);
    let relation = engine
        .prepare_query_with_options(
            "SELECT ?this ?value WHERE { ?this <http://example.org/relation> ?value }",
            None,
            options,
        )
        .unwrap();
    for shared in [false, true] {
        let state = metered();
        let view = paged.query_view(PagedQueryLimits::UNBOUNDED);
        let result = if shared {
            engine.query_prepared_governed_fallible_in_operation(
                &view,
                &relation,
                &[],
                QueryOptions::EMPTY,
                &state,
            )
        } else {
            engine.query_prepared_governed_fallible_view(
                &view,
                &relation,
                &[],
                QueryOptions::EMPTY,
                &QueryGovernors::METERED,
            )
        };
        let FallibleSparqlError::Query {
            diagnostic,
            evidence,
        } = result.unwrap_err()
        else {
            panic!("registry mismatch must stay a query admission error");
        };
        assert_eq!(diagnostic.code, "native-sparql-property-function");
        assert_eq!(evidence.governors.consumed_in(ResourceDimension::Fuel), 0);
        assert!(evidence.governors.is_complete());
        let answer = if shared {
            engine.query_prepared_governed_fallible_in_operation(
                &view,
                &relation,
                &[],
                options,
                &state,
            )
        } else {
            engine.query_prepared_governed_fallible_view(
                &view,
                &relation,
                &[],
                options,
                &QueryGovernors::METERED,
            )
        }
        .unwrap();
        let (variables, rows) = support::solutions(answer.result);
        assert_eq!(variables, ["this", "value"]);
        assert_eq!(
            rows,
            [vec![Some(support::iri("a")), Some(support::iri("value"))]]
        );
        if shared {
            assert_eq!(answer.evidence.governors, state.evidence());
        }
    }
}

#[test]
fn explicit_per_call_source_replaces_options_remote_and_preserves_receipts() {
    let paged = pages();
    let engine = NativeSparqlEngine::new();
    let query =
        "SELECT ?remote WHERE { SERVICE <http://example.org/service> { BIND(\"ok\" AS ?remote) } }";
    let prepared = engine.prepare_query(query, None).unwrap();
    let replaced_calls = Arc::new(AtomicUsize::new(0));
    let calls = replaced_calls.clone();
    let replaced =
        HttpRemoteQuerySource::new(move |_: HttpRequest<'_>| -> Result<Vec<u8>, RemoteError> {
            calls.fetch_add(1, Ordering::Relaxed);
            Err(RemoteError::Disabled)
        });
    let source = HttpRemoteQuerySource::new(|_: HttpRequest<'_>| -> Result<Vec<u8>, RemoteError> {
        Ok(br#"{"head":{"vars":["remote"]},"results":{"bindings":[{"remote":{"type":"literal","value":"ok"}}]}}"#.to_vec())
    });
    let answer = engine
        .query_prepared_governed_fallible_with_source_view(
            &paged.query_view(PagedQueryLimits::UNBOUNDED),
            &prepared,
            &[],
            &source,
            QueryOptions::EMPTY.with_remote(Some(&replaced)),
            &QueryGovernors::METERED,
        )
        .unwrap();
    let direct = engine
        .query_prepared_governed_fallible_view(
            &paged.query_view(PagedQueryLimits::UNBOUNDED),
            &prepared,
            &[],
            QueryOptions::EMPTY.with_remote(Some(&source)),
            &QueryGovernors::METERED,
        )
        .unwrap();
    assert_eq!(replaced_calls.load(Ordering::Relaxed), 0);
    assert_eq!(support::row_count(&answer.result), 1);
    let (variables, rows) = support::solutions(answer.result.clone());
    assert_eq!(variables, ["remote"]);
    assert_eq!(rows, [vec![Some(TermValue::simple_literal("ok"))]]);
    assert_eq!(answer.evidence.view, direct.evidence.view);
    assert_eq!(answer.evidence.governors, direct.evidence.governors);
    equal_results(answer.result, direct.result);
}

#[test]
fn remote_cancellation_prevents_prepared_fallible_publication() {
    let paged = pages();
    let engine = NativeSparqlEngine::new();
    let query =
        "SELECT ?remote WHERE { SERVICE <http://example.org/service> { BIND(\"ok\" AS ?remote) } }";
    let prepared = engine.prepare_query(query, None).unwrap();
    for shared in [false, true] {
        let view = paged.query_view(PagedQueryLimits::UNBOUNDED);
        let stop = Arc::new(CancellationFlag::new());
        let governors = QueryGovernors::METERED.with_stop_signal(stop.clone());
        let state = Arc::new(GovernorState::new(&governors));
        let source = HttpRemoteQuerySource::new(
            move |_: HttpRequest<'_>| -> Result<Vec<u8>, RemoteError> {
                stop.cancel();
                Ok(br#"{"head":{"vars":["remote"]},"results":{"bindings":[{"remote":{"type":"literal","value":"ok"}}]}}"#.to_vec())
            },
        );
        let result = if shared {
            engine.query_prepared_governed_fallible_with_source_in_operation(
                &view,
                &prepared,
                &[],
                &source,
                QueryOptions::EMPTY,
                &state,
            )
        } else {
            engine.query_prepared_governed_fallible_with_source_view(
                &view,
                &prepared,
                &[],
                &source,
                QueryOptions::EMPTY,
                &governors,
            )
        };
        let error = result.unwrap_err();
        let trip = TrippedGovernor::Stopped {
            cause: StopCause::Cancelled,
        };
        assert_eq!(error.tripped(), Some(trip));
        let FallibleSparqlError::BudgetExhausted { evidence, .. } = error else {
            panic!("a remote cancellation cannot publish completion");
        };
        assert_eq!(evidence.governors.tripped, Some(trip));
        if shared {
            assert_eq!(evidence.governors, state.evidence());
        }
    }
}

#[test]
fn simultaneously_started_worker_local_engines_share_one_plan_and_fuel_ceiling() {
    let dataset = three_subjects_one_value();
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
