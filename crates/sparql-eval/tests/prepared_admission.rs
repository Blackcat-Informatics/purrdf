// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Compiler algebra, public-field compatibility and allocation lifetime contracts.

use purrdf_core::{RdfDatasetBuilder, ResourceDimension, SparqlResult};
use purrdf_sparql_algebra::{
    Expression, GraphPattern, GroundTerm, GroundTriple, Literal, NamedNode, ParserOptions,
    PropertyFunctionCall, PropertyPathExpression, Query, QueryDataset, TermPattern, Variable,
};
use purrdf_sparql_eval::governor::GovernorState;
use purrdf_sparql_eval::{
    CacheLimits, MemoryRelation, NativeSparqlEngine, PlanCache, PreparedQuery,
    PropertyFunctionRegistry, QueryGovernors, QueryOptions,
};
use std::sync::Arc;

fn ask(pattern: GraphPattern) -> Query {
    Query::Ask {
        pattern,
        dataset: QueryDataset::default(),
        base_iri: None,
        version: None,
    }
}

fn named() -> GroundTerm {
    GroundTerm::NamedNode(NamedNode::new("http://example.org/value").unwrap())
}

fn values(variables: Vec<Variable>, row: Vec<Option<GroundTerm>>) -> GraphPattern {
    GraphPattern::Values {
        variables,
        bindings: vec![row],
    }
}

#[test]
fn malformed_compiler_rows_are_refused_before_evaluation() {
    let engine = NativeSparqlEngine::new();
    for pattern in [
        values(vec![], vec![Some(named())]),
        values(vec![Variable::new("x")], vec![]),
        values(
            vec![Variable::new("x"), Variable::new("x")],
            vec![Some(named()), Some(named())],
        ),
    ] {
        let query = ask(pattern);
        assert!(
            engine
                .prepare_algebra(query.clone(), QueryOptions::EMPTY)
                .is_err()
        );
        assert!(PreparedQuery::rewritten(query, QueryOptions::EMPTY).is_err());
    }
    assert_eq!(engine.plan_cache_stats().misses, 0);
    let empty = engine
        .prepare_algebra(ask(values(vec![], vec![])), QueryOptions::EMPTY)
        .unwrap();
    let data = RdfDatasetBuilder::new().freeze().unwrap();
    assert!(matches!(
        engine
            .query_prepared(&data, &empty, &[], QueryOptions::EMPTY)
            .unwrap(),
        SparqlResult::Boolean(true)
    ));
}

#[test]
fn term_validation_preserves_rdf_values_and_rejects_invalid_structure() {
    let engine = NativeSparqlEngine::new();
    let invalid = [
        GroundTerm::NamedNode(NamedNode::new_unchecked("relative")),
        GroundTerm::NamedNode(NamedNode::new_unchecked("http://example.org/\u{0}")),
        GroundTerm::Literal(Literal::new_lang("text", "en--rtl", None)),
        GroundTerm::Literal(Literal::new_typed(
            "text",
            NamedNode::new_unchecked("relative"),
        )),
        GroundTerm::Triple(Box::new(GroundTriple {
            subject: GroundTerm::Literal(Literal::new_simple("subject")),
            predicate: NamedNode::new("http://example.org/p").unwrap(),
            object: named(),
        })),
    ];
    for term in invalid {
        assert!(
            engine
                .prepare_algebra(
                    ask(values(vec![Variable::new("x")], vec![Some(term)])),
                    QueryOptions::EMPTY
                )
                .is_err()
        );
    }
    for term in [
        GroundTerm::Literal(Literal::new_typed(
            "not-an-integer",
            NamedNode::new("http://www.w3.org/2001/XMLSchema#integer").unwrap(),
        )),
        GroundTerm::Literal(Literal::new_lang(
            "مرحبا",
            "ar",
            Some(purrdf_sparql_algebra::BaseDirection::Rtl),
        )),
        GroundTerm::BlankNode(purrdf_sparql_algebra::BlankNode::new(
            "opaque label.with spaces",
        )),
    ] {
        engine
            .prepare_algebra(
                ask(values(vec![Variable::new("名")], vec![Some(term)])),
                QueryOptions::EMPTY,
            )
            .unwrap();
    }
}

#[test]
fn malformed_ranges_targets_and_nested_expressions_are_refused() {
    let engine = NativeSparqlEngine::new();
    let empty = || Box::new(GraphPattern::Bgp { patterns: vec![] });
    let invalid = [
        GraphPattern::Path {
            subject: TermPattern::Variable(Variable::new("s")),
            path: PropertyPathExpression::Range {
                inner: Box::new(PropertyPathExpression::NamedNode(
                    NamedNode::new("http://example.org/p").unwrap(),
                )),
                min: 3,
                max: Some(2),
            },
            object: TermPattern::Variable(Variable::new("o")),
        },
        GraphPattern::Unfold {
            inner: empty(),
            expression: Expression::Variable(Variable::new("list")),
            element: Variable::new("x"),
            companion: Some(Variable::new("x")),
        },
        values(vec![Variable::new("invalid name")], vec![Some(named())]),
    ];
    for pattern in invalid {
        assert!(
            engine
                .prepare_algebra(ask(pattern), QueryOptions::EMPTY)
                .is_err()
        );
    }
    let mut expression = Expression::Literal(Literal::new_simple("leaf"));
    for _ in 0..(purrdf_sparql_algebra::MAX_GRAPH_PATTERN_NODES
        + 8 * purrdf_sparql_algebra::MAX_GRAPH_PATTERN_DEPTH)
    {
        expression = Expression::Not(Box::new(expression));
    }
    assert!(
        engine
            .prepare_algebra(
                ask(GraphPattern::Filter {
                    expr: expression,
                    inner: empty()
                }),
                QueryOptions::EMPTY
            )
            .is_err()
    );
}

#[test]
fn rewritten_calls_share_registry_and_arity_admission() {
    let engine = NativeSparqlEngine::new();
    let call = |subject_args, object_args| {
        ask(GraphPattern::PropertyFunction(PropertyFunctionCall {
            iri: "http://example.org/relation".into(),
            subject_args,
            object_args,
        }))
    };
    assert!(PreparedQuery::rewritten(call(vec![], vec![]), QueryOptions::EMPTY).is_err());
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(
        "http://example.org/relation",
        Arc::new(MemoryRelation::new(1, 1, vec![]).unwrap()),
    );
    let options = QueryOptions {
        property_functions: &registry,
        ..QueryOptions::EMPTY
    };
    assert!(PreparedQuery::rewritten(call(vec![], vec![]), options).is_err());
    let query = call(
        vec![TermPattern::Variable(Variable::new("s"))],
        vec![TermPattern::Variable(Variable::new("o"))],
    );
    let typed = engine.prepare_algebra(query.clone(), options).unwrap();
    let rewritten = PreparedQuery::rewritten(query, options).unwrap();
    let data = RdfDatasetBuilder::new().freeze().unwrap();
    for prepared in [&*typed, &rewritten] {
        assert!(matches!(
            engine
                .query_prepared(&data, prepared, &[], options)
                .unwrap(),
            SparqlResult::Boolean(false)
        ));
    }
}

#[test]
fn public_query_mutation_cannot_bypass_admission_or_spend_governor_fuel() {
    let engine = NativeSparqlEngine::new();
    let mut prepared = PreparedQuery::rewritten(
        ask(GraphPattern::Bgp { patterns: vec![] }),
        QueryOptions::EMPTY,
    )
    .unwrap();
    prepared.query = ask(values(vec![], vec![Some(named())]));
    let data = RdfDatasetBuilder::new().freeze().unwrap();
    let state = Arc::new(GovernorState::new(&QueryGovernors::METERED));
    assert!(
        engine
            .query_prepared(&data, &prepared, &[], QueryOptions::EMPTY)
            .is_err()
    );
    assert!(
        engine
            .query_prepared_governed_in_operation(
                &*data,
                &prepared,
                &[],
                QueryOptions::EMPTY,
                &state
            )
            .is_err()
    );
    assert_eq!(state.evidence().consumed_in(ResourceDimension::Fuel), 0);
}

#[test]
fn admitted_plan_lifetimes_remain_observable_after_eviction_and_cache_drop() {
    let mut cache = PlanCache::with_limits(CacheLimits {
        entries: 1,
        bytes: 1_000_000,
    });
    let observer = cache.memory_observer();
    let first = cache.prepare("ASK {}", None).unwrap();
    let first_bytes = first.retained_size_bytes();
    assert_eq!(observer.stats().retained_bytes, first_bytes);
    let another_handle = first.clone();
    assert_eq!(observer.stats().live_plans, 1);
    let second = cache.prepare("ASK { ?s ?p ?o }", None).unwrap();
    assert_eq!(
        (
            observer.stats().retained_plans,
            observer.stats().detached_plans
        ),
        (1, 1)
    );
    assert_eq!(observer.stats().detached_bytes, first_bytes);
    drop(first);
    assert_eq!(observer.stats().live_plans, 2);
    drop(another_handle);
    assert_eq!(observer.stats().detached_plans, 0);
    drop(cache);
    assert_eq!(
        (
            observer.stats().retained_plans,
            observer.stats().detached_plans
        ),
        (0, 1)
    );
    drop(second);
    assert_eq!(
        observer.stats(),
        purrdf_sparql_eval::PlanMemoryStats::default()
    );
}

#[test]
fn disabled_and_compiler_plans_are_counted_without_retention() {
    let engine = NativeSparqlEngine::new().with_plan_cache_limits(CacheLimits {
        entries: 0,
        bytes: 0,
    });
    let observer = engine.plan_memory_observer();
    let text = engine.prepare_query("ASK {}", None).unwrap();
    let mut typed = engine
        .prepare_algebra(
            ask(GraphPattern::Bgp { patterns: vec![] }),
            QueryOptions::EMPTY,
        )
        .unwrap();
    assert_eq!(
        (observer.stats().live_plans, observer.stats().detached_plans),
        (2, 2)
    );
    let admitted_bytes = observer.stats().live_bytes;
    Arc::get_mut(&mut typed).unwrap().query = ask(values(
        vec![Variable::new("x")],
        vec![Some(GroundTerm::Literal(Literal::new_simple(
            "x".repeat(4096),
        )))],
    ));
    assert_eq!(
        observer.stats().live_bytes,
        admitted_bytes,
        "admission accounting does not claim to intercept caller mutation"
    );
    assert!(typed.retained_size_bytes() > admitted_bytes);
    drop((text, typed));
    assert_eq!(
        observer.stats(),
        purrdf_sparql_eval::PlanMemoryStats::default()
    );
}

#[test]
fn cache_keys_keep_base_unicode_and_field_boundaries_distinct() {
    let mut cache = PlanCache::new();
    let query = "SELECT ?名 WHERE { ?名 <predicate> ?value }";
    let a = cache.prepare(query, Some("http://example.org/a/")).unwrap();
    let b = cache.prepare(query, Some("http://example.org/b/")).unwrap();
    assert_ne!(a.query, b.query);
    assert!(Arc::ptr_eq(
        &a,
        &cache.prepare(query, Some("http://example.org/a/")).unwrap()
    ));
    let options = |names| ParserOptions {
        extension_fn_namespaces: names,
        ..ParserOptions::default()
    };
    let one = cache
        .prepare_with(
            "ASK {}",
            None,
            &options(vec!["http://example.org/α\u{1}http://example.org/β".into()]),
        )
        .unwrap();
    let two = cache
        .prepare_with(
            "ASK {}",
            None,
            &options(vec![
                "http://example.org/α".into(),
                "http://example.org/β".into(),
            ]),
        )
        .unwrap();
    assert!(!Arc::ptr_eq(&one, &two));
    assert_eq!(cache.stats().entries, 4);
}

#[test]
fn parsed_and_compiler_preparation_preserve_flat_operator_boundary_acceptance() {
    // One group element per OPTIONAL, with no nested group contents. This
    // reaches the parser's combinator budget while using only two brace levels.
    let query = format!(
        "ASK {{ {} }}",
        "OPTIONAL {} ".repeat(purrdf_sparql_algebra::MAX_GRAPH_PATTERN_NODES)
    );
    let engine = NativeSparqlEngine::new();
    let parsed = purrdf_sparql_algebra::SparqlParser::new()
        .parse_query(&query)
        .unwrap();
    parsed.validate().unwrap();
    let text = engine.prepare_query(&query, None).unwrap();
    let typed = engine.prepare_algebra(parsed, QueryOptions::EMPTY).unwrap();
    assert_eq!(text.query, typed.query);
    let data = RdfDatasetBuilder::new().freeze().unwrap();
    // Preparation accepts the parser's envelope. Execution retains its existing
    // narrower recursive-evaluator guard and must return a diagnostic safely.
    for prepared in [&text, &typed] {
        assert!(
            engine
                .query_prepared(&data, prepared, &[], QueryOptions::EMPTY)
                .is_err()
        );
        assert!(
            engine
                .query_prepared_governed_in_operation(
                    &*data,
                    prepared,
                    &[],
                    QueryOptions::EMPTY,
                    &Arc::new(GovernorState::new(&QueryGovernors::METERED))
                )
                .is_err()
        );
    }
    let too_many = format!(
        "ASK {{ {} }}",
        "OPTIONAL {} ".repeat(purrdf_sparql_algebra::MAX_GRAPH_PATTERN_NODES + 1)
    );
    assert!(engine.prepare_query(&too_many, None).is_err());
}

struct SubjectBoundRelation {
    relation: MemoryRelation,
    modes: [purrdf_core::binding_pattern::BindingPattern; 1],
}

impl purrdf_sparql_eval::PropertyFunction for SubjectBoundRelation {
    fn volatility(&self) -> purrdf_sparql_eval::user_fn::Volatility {
        purrdf_sparql_eval::user_fn::Volatility::Stable
    }
    fn arity(&self) -> purrdf_sparql_eval::PfArity {
        purrdf_sparql_eval::PfArity::new(1, 1)
    }
    fn modes(&self) -> &[purrdf_core::binding_pattern::BindingPattern] {
        &self.modes
    }
    fn rows_per_invocation(&self, _: purrdf_core::binding_pattern::BindingPattern) -> u64 {
        1
    }
    fn open(
        &self,
        args: &purrdf_sparql_eval::PfArgs<'_>,
        ceiling: Option<u64>,
    ) -> Result<Box<dyn purrdf_sparql_eval::PfCursor>, purrdf_sparql_eval::EvalError> {
        self.relation.open(args, ceiling)
    }
}

#[test]
fn rewrites_reorder_feasible_calls_and_public_mutations_must_be_reprepared() {
    use purrdf_core::{TermValue, binding_pattern::BindingPattern};
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(
        "http://example.org/relation",
        Arc::new(SubjectBoundRelation {
            relation: MemoryRelation::new(
                1,
                1,
                vec![vec![
                    TermValue::Iri("http://example.org/value".into()),
                    TermValue::Iri("http://example.org/result".into()),
                ]],
            )
            .unwrap(),
            modes: [BindingPattern::from_code("bf")],
        }),
    );
    let options = QueryOptions {
        property_functions: &registry,
        ..QueryOptions::EMPTY
    };
    let call = GraphPattern::PropertyFunction(PropertyFunctionCall {
        iri: "http://example.org/relation".into(),
        subject_args: vec![TermPattern::Variable(Variable::new("s"))],
        object_args: vec![TermPattern::Variable(Variable::new("o"))],
    });
    assert!(PreparedQuery::rewritten(ask(call.clone()), options).is_err());
    let raw = ask(GraphPattern::Join {
        left: Box::new(GraphPattern::Lateral {
            left: Box::new(GraphPattern::Bgp { patterns: vec![] }),
            right: Box::new(call),
        }),
        right: Box::new(GraphPattern::Bgp {
            patterns: vec![purrdf_sparql_algebra::TriplePattern {
                subject: TermPattern::Variable(Variable::new("s")),
                predicate: purrdf_sparql_algebra::NamedNodePattern::NamedNode(
                    NamedNode::new("http://example.org/binding").unwrap(),
                ),
                object: TermPattern::Variable(Variable::new("bound")),
            }],
        }),
    });
    let mut rewritten = PreparedQuery::rewritten(raw.clone(), options).unwrap();
    assert_ne!(
        rewritten.query, raw,
        "admission must put the binding before the call"
    );
    let engine = NativeSparqlEngine::new();
    let typed = engine.prepare_algebra(raw.clone(), options).unwrap();
    assert_eq!(typed.query, rewritten.query);
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_iri("http://example.org/value");
    let predicate = builder.intern_iri("http://example.org/binding");
    builder.push_quad(subject, predicate, subject, None);
    let data = builder.freeze().unwrap();
    assert!(matches!(
        engine
            .query_prepared(&data, &rewritten, &[], options)
            .unwrap(),
        SparqlResult::Boolean(true)
    ));
    rewritten.query = raw;
    let state = Arc::new(GovernorState::new(&QueryGovernors::METERED));
    let diagnostic = engine
        .query_prepared_governed_in_operation(&*data, &rewritten, &[], options, &state)
        .unwrap_err();
    assert_eq!(diagnostic.code, "native-sparql-algebra");
    assert_eq!(state.evidence().consumed_in(ResourceDimension::Fuel), 0);
    let readmitted = PreparedQuery::rewritten(rewritten.query, options).unwrap();
    assert!(matches!(
        engine
            .query_prepared(&data, &readmitted, &[], options)
            .unwrap(),
        SparqlResult::Boolean(true)
    ));
}

#[test]
fn parser_and_prepared_execution_agree_near_the_evaluator_depth_boundary() {
    let query = format!(
        "ASK {{ {} }}",
        "OPTIONAL {} ".repeat(purrdf_sparql_algebra::MAX_GRAPH_PATTERN_DEPTH - 2)
    );
    let engine = NativeSparqlEngine::new();
    let prepared = engine.prepare_query(&query, None).unwrap();
    let data = RdfDatasetBuilder::new().freeze().unwrap();
    assert!(matches!(
        engine
            .query_prepared(&data, &prepared, &[], QueryOptions::EMPTY)
            .unwrap(),
        SparqlResult::Boolean(true)
    ));
    let state = Arc::new(GovernorState::new(&QueryGovernors::METERED));
    let result = engine
        .query_prepared_governed_in_operation(&*data, &prepared, &[], QueryOptions::EMPTY, &state)
        .unwrap();
    assert!(matches!(
        result,
        purrdf_sparql_eval::GovernedOutcome::Complete {
            result: SparqlResult::Boolean(true),
            ..
        }
    ));
}

#[test]
fn compiler_aggregate_calls_are_admitted_with_their_registry() {
    use purrdf_sparql_algebra::{AggregateExpression, AggregateFunction};
    let aggregate = AggregateExpression::new(
        AggregateFunction::Custom(NamedNode::new("http://example.org/aggregate").unwrap()),
        vec![Expression::Variable(Variable::new("x"))],
        vec![],
        vec![],
        false,
    )
    .unwrap();
    let query = ask(GraphPattern::Group {
        inner: Box::new(values(vec![Variable::new("x")], vec![Some(named())])),
        variables: vec![],
        aggregates: vec![(Variable::new("result"), aggregate)],
    });
    let error = PreparedQuery::rewritten(query.clone(), QueryOptions::EMPTY).unwrap_err();
    assert_eq!(error.code, "native-sparql-aggregate-function");
    let engine = NativeSparqlEngine::new();
    assert_eq!(
        engine
            .prepare_algebra(query, QueryOptions::EMPTY)
            .unwrap_err()
            .code,
        error.code
    );
}

#[test]
fn duplicate_projection_and_group_keys_preserve_parsed_and_compiler_results() {
    use purrdf_core::TermValue;
    let engine = NativeSparqlEngine::new();
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_iri("http://example.org/s");
    let predicate = builder.intern_iri("http://example.org/p");
    for object in ["http://example.org/a", "http://example.org/b"] {
        let object = builder.intern_iri(object);
        builder.push_quad(subject, predicate, object, None);
    }
    let data = builder.freeze().unwrap();
    for (text, columns, expected_rows) in [
        ("SELECT ?s ?s WHERE { ?s ?p ?o }", vec!["s"], 2),
        (
            "SELECT ?s (COUNT(*) AS ?count) WHERE { ?s ?p ?o } GROUP BY ?s ?s",
            vec!["s", "count"],
            1,
        ),
    ] {
        let parsed = purrdf_sparql_algebra::SparqlParser::new()
            .parse_query(text)
            .unwrap();
        parsed.validate().unwrap();
        let from_text = engine.prepare_query(text, None).unwrap();
        let from_algebra = engine.prepare_algebra(parsed, QueryOptions::EMPTY).unwrap();
        for plan in [&from_text, &from_algebra] {
            let SparqlResult::Solutions {
                variables, rows, ..
            } = engine
                .query_prepared(&data, plan, &[], QueryOptions::EMPTY)
                .unwrap()
            else {
                panic!("expected solutions");
            };
            assert_eq!(variables, columns);
            assert_eq!(rows.len(), expected_rows);
            assert!(rows.iter().all(|row| row.len() == columns.len()));
            assert_eq!(
                rows[0][0],
                Some(TermValue::Iri("http://example.org/s".into()))
            );
            if columns.len() == 2 {
                assert!(
                    matches!(&rows[0][1], Some(TermValue::Literal { lexical_form, .. }) if lexical_form == "2")
                );
            }
        }
    }
}

#[test]
fn aggregate_output_collisions_are_refused_without_rejecting_redundant_keys() {
    use purrdf_sparql_algebra::{AggregateExpression, AggregateFunction};
    let count =
        AggregateExpression::new(AggregateFunction::Count, vec![], vec![], vec![], false).unwrap();
    for (variables, aggregates) in [
        (
            vec![Variable::new("x")],
            vec![(Variable::new("x"), count.clone())],
        ),
        (
            vec![],
            vec![
                (Variable::new("x"), count.clone()),
                (Variable::new("x"), count),
            ],
        ),
    ] {
        let query = ask(GraphPattern::Group {
            inner: Box::new(GraphPattern::Bgp { patterns: vec![] }),
            variables,
            aggregates,
        });
        assert!(PreparedQuery::rewritten(query, QueryOptions::EMPTY).is_err());
    }
}

#[test]
fn deeply_nested_prepared_expressions_execute_without_recursive_admission_visitors() {
    let mut expression = Expression::Literal(Literal::new_simple("true"));
    for _ in 0..512 {
        expression = Expression::Not(Box::new(expression));
    }
    let engine = NativeSparqlEngine::new();
    let query = ask(GraphPattern::Filter {
        expr: expression,
        inner: Box::new(GraphPattern::Bgp { patterns: vec![] }),
    });
    let prepared = engine.prepare_algebra(query, QueryOptions::EMPTY).unwrap();
    let data = RdfDatasetBuilder::new().freeze().unwrap();
    assert!(matches!(
        engine
            .query_prepared(&data, &prepared, &[], QueryOptions::EMPTY)
            .unwrap(),
        SparqlResult::Boolean(true)
    ));
    let state = Arc::new(GovernorState::new(&QueryGovernors::METERED));
    assert!(matches!(
        engine
            .query_prepared_governed_in_operation(
                &*data,
                &prepared,
                &[],
                QueryOptions::EMPTY,
                &state
            )
            .unwrap(),
        purrdf_sparql_eval::GovernedOutcome::Complete {
            result: SparqlResult::Boolean(true),
            ..
        }
    ));
}

#[test]
fn reserved_language_datatype_shapes_are_refused_by_text_and_compiler_admission() {
    use purrdf_core::TermValue;
    let engine = NativeSparqlEngine::new();
    let data = RdfDatasetBuilder::new().freeze().unwrap();
    for datatype in [
        "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString",
        "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString",
    ] {
        let text = format!("ASK {{ VALUES ?x {{ \"text\"^^<{datatype}> }} }}");
        let parsed = purrdf_sparql_algebra::SparqlParser::new()
            .parse_query(&text)
            .unwrap();
        assert!(engine.prepare_query(&text, None).is_err());
        assert!(
            engine
                .prepare_algebra(parsed.clone(), QueryOptions::EMPTY)
                .is_err()
        );
        let mut changed = PreparedQuery::rewritten(
            ask(GraphPattern::Bgp { patterns: vec![] }),
            QueryOptions::EMPTY,
        )
        .unwrap();
        changed.query = parsed;
        let state = Arc::new(GovernorState::new(&QueryGovernors::METERED));
        assert!(
            engine
                .query_prepared_governed_in_operation(
                    &*data,
                    &changed,
                    &[],
                    QueryOptions::EMPTY,
                    &state
                )
                .is_err()
        );
        assert_eq!(state.evidence().consumed_in(ResourceDimension::Fuel), 0);
    }
    for literal in [
        "\"text\"@en",
        "\"text\"@en--ltr",
        "\"text\"@en--rtl",
        "\"invalid-integer\"^^<http://www.w3.org/2001/XMLSchema#integer>",
    ] {
        let text = format!("SELECT ?x WHERE {{ VALUES ?x {{ {literal} }} }}");
        let from_text = engine.prepare_query(&text, None).unwrap();
        let parsed = purrdf_sparql_algebra::SparqlParser::new()
            .parse_query(&text)
            .unwrap();
        let from_algebra = engine.prepare_algebra(parsed, QueryOptions::EMPTY).unwrap();
        for plan in [&from_text, &from_algebra] {
            let SparqlResult::Solutions { rows, .. } = engine
                .query_prepared(&data, plan, &[], QueryOptions::EMPTY)
                .unwrap()
            else {
                panic!("expected solutions");
            };
            assert!(matches!(rows[0][0], Some(TermValue::Literal { .. })));
        }
    }
}

#[test]
fn oversized_value_trees_cannot_bypass_execution_admission_after_public_mutation() {
    let mut expression = Expression::Literal(Literal::new_simple("true"));
    for _ in 0..3068 {
        expression = Expression::Not(Box::new(expression));
    }
    let query = ask(GraphPattern::Filter {
        expr: expression,
        inner: Box::new(GraphPattern::Bgp { patterns: vec![] }),
    });
    let engine = NativeSparqlEngine::new();
    assert!(query.validate().is_err());
    let mut changed = PreparedQuery::rewritten(
        ask(GraphPattern::Bgp { patterns: vec![] }),
        QueryOptions::EMPTY,
    )
    .unwrap();
    changed.query = query;
    let data = RdfDatasetBuilder::new().freeze().unwrap();
    let state = Arc::new(GovernorState::new(&QueryGovernors::METERED));
    assert!(
        engine
            .query_prepared_governed_in_operation(
                &*data,
                &changed,
                &[],
                QueryOptions::EMPTY,
                &state
            )
            .is_err()
    );
    assert_eq!(state.evidence().consumed_in(ResourceDimension::Fuel), 0);
}
