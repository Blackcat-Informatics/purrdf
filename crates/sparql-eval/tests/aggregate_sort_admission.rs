// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Aggregate sort keys participate in admission, effects and execution contracts.

use std::sync::Arc;

use purrdf_core::{RdfDatasetBuilder, ResourceDimension, SparqlEngine, SparqlRequest};
use purrdf_sparql_algebra::{
    AggregateExpression, AggregateFunction, Expression, GraphPattern, GroundTerm, Literal,
    OrderExpression, PropertyFunctionCall, Query, QueryDataset, TermPattern, Variable,
};
use purrdf_sparql_eval::eval::{EvalCtx, eval, evaluate_query};
use purrdf_sparql_eval::governor::GovernorState;
use purrdf_sparql_eval::{
    EvalError, InProcessServiceResolver, MemoryRelation, NativeSparqlEngine, PreparedQuery,
    PropertyFunctionRegistry, QueryGovernors, QueryOptions,
};

fn ask(pattern: GraphPattern) -> Query {
    Query::Ask {
        pattern,
        dataset: QueryDataset::default(),
        base_iri: None,
        version: None,
    }
}

fn fold(key: Expression) -> GraphPattern {
    GraphPattern::Group {
        inner: Box::new(GraphPattern::Values {
            variables: vec![Variable::new("v")],
            bindings: vec![vec![Some(GroundTerm::Literal(Literal::new_simple(
                "value",
            )))]],
        }),
        variables: vec![],
        aggregates: vec![(
            Variable::new("list"),
            AggregateExpression::new(
                AggregateFunction::Fold,
                vec![Expression::Variable(Variable::new("v"))],
                vec![],
                vec![OrderExpression::Asc(key)],
                false,
            )
            .unwrap(),
        )],
    }
}

fn request(query: &str) -> SparqlRequest<'_> {
    SparqlRequest {
        query,
        base_iri: None,
        substitutions: &[],
    }
}

#[test]
fn calls_reachable_only_through_aggregate_sort_keys_require_admission() {
    let call = GraphPattern::PropertyFunction(PropertyFunctionCall {
        iri: "http://example.org/unregistered".into(),
        subject_args: vec![TermPattern::Variable(Variable::new("s"))],
        object_args: vec![TermPattern::Variable(Variable::new("o"))],
    });
    let custom = GraphPattern::Group {
        inner: Box::new(GraphPattern::Bgp { patterns: vec![] }),
        variables: vec![],
        aggregates: vec![(
            Variable::new("custom"),
            AggregateExpression::new(
                AggregateFunction::Custom(
                    purrdf_sparql_algebra::NamedNode::new("http://example.org/unregistered")
                        .unwrap(),
                ),
                vec![Expression::Literal(Literal::new_simple("value"))],
                vec![],
                vec![],
                false,
            )
            .unwrap(),
        )],
    };
    let engine = NativeSparqlEngine::new();
    let data = RdfDatasetBuilder::new().freeze().unwrap();
    for (inner, code) in [
        (call, "native-sparql-property-function"),
        (custom, "native-sparql-aggregate-function"),
    ] {
        let query = ask(fold(Expression::Exists(Box::new(inner))));
        assert_eq!(
            engine
                .prepare_algebra(query.clone(), QueryOptions::EMPTY)
                .unwrap_err()
                .code,
            code
        );
        assert!(PreparedQuery::rewritten(query.clone(), QueryOptions::EMPTY).is_err());
        let mut prepared = PreparedQuery::rewritten(
            ask(GraphPattern::Bgp { patterns: vec![] }),
            QueryOptions::EMPTY,
        )
        .unwrap();
        prepared.query = query;
        let state = Arc::new(GovernorState::new(&QueryGovernors::METERED));
        assert_eq!(
            engine
                .query_prepared_governed_in_operation(
                    &*data,
                    &prepared,
                    &[],
                    QueryOptions::EMPTY,
                    &state,
                )
                .unwrap_err()
                .code,
            code
        );
        assert_eq!(state.evidence().consumed_in(ResourceDimension::Fuel), 0);
    }
}

#[test]
fn aggregate_output_collisions_keep_their_error_when_the_child_would_truncate() {
    let data = RdfDatasetBuilder::new().freeze().unwrap();
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
        let pattern = GraphPattern::Group {
            inner: Box::new(GraphPattern::Bgp { patterns: vec![] }),
            variables,
            aggregates,
        };
        for fuel in [1, u64::MAX] {
            let state = Arc::new(GovernorState::new(&QueryGovernors::METERED.with_fuel(fuel)));
            let mut ctx = EvalCtx::new(&*data).with_governors(state);
            let error = eval(&pattern, &mut ctx).unwrap_err();
            assert_eq!(
                error,
                EvalError::Config("aggregate output collides with another group output".into())
            );
            let state = Arc::new(GovernorState::new(&QueryGovernors::METERED.with_fuel(fuel)));
            let mut ctx = EvalCtx::new(&*data).with_governors(state);
            assert_eq!(
                evaluate_query(&ask(pattern.clone()), &mut ctx).unwrap_err(),
                error
            );
        }
    }
}

#[test]
fn service_silent_cannot_hide_forwarding_hazards_in_aggregate_sort_keys() {
    let data = RdfDatasetBuilder::new().freeze().unwrap();
    let engine = NativeSparqlEngine::new();
    let source = InProcessServiceResolver::new();
    let mut relations = PropertyFunctionRegistry::new();
    relations.register(
        "http://example.org/relation",
        Arc::new(MemoryRelation::new(1, 1, vec![]).unwrap()),
    );
    let options = QueryOptions {
        property_functions: &relations,
        ..QueryOptions::EMPTY
    };
    for (key, expected) in [
        (
            "EXISTS { ?s <http://example.org/relation> ?o }",
            "property-function call inside a SERVICE body",
        ),
        (
            "EXISTS { SELECT (AGG(<http://example.org/custom>, ?x) AS ?n) WHERE { VALUES ?x { 1 } } }",
            "custom-aggregate call inside a SERVICE body",
        ),
        (
            "<http://example.org/custom>(?v)",
            "custom scalar-function call inside a SERVICE SILENT body",
        ),
        (
            "EXISTS { VALUES ?x { 1 } LATERAL { VALUES ?y { 2 } } }",
            "LATERAL clause inside a SERVICE SILENT body",
        ),
    ] {
        let query = format!(
            "SELECT * WHERE {{ SERVICE SILENT <http://example.org/endpoint> {{ \
             SELECT (FOLD(?v ORDER BY ASC({key})) AS ?list) WHERE {{ VALUES ?v {{ 1 }} }} \
             }} }}"
        );
        let error = engine
            .query_with_source_view(&*data, request(&query), &source, options)
            .unwrap_err();
        assert!(error.message.contains(expected), "{error:?}");
    }
}

#[test]
fn basic_profile_checks_aggregate_sort_keys_before_governor_work() {
    let data = RdfDatasetBuilder::new().freeze().unwrap();
    let engine = NativeSparqlEngine::new();
    let body = "SELECT (FOLD(?v ORDER BY TRIPLE(<http://example.org/s>, \
                <http://example.org/p>, <http://example.org/o>)) AS ?list) \
                WHERE { VALUES ?v { 1 } }";
    engine
        .query(&data, request(&format!("VERSION \"1.2\" {body}")))
        .unwrap();
    let query = format!("VERSION \"1.2-basic\" {body}");
    let prepared = engine.prepare_query(&query, None).unwrap();
    let error = engine.query(&data, request(&query)).unwrap_err();
    assert!(error.message.contains("1.2-basic"), "{error:?}");
    let state = Arc::new(GovernorState::new(&QueryGovernors::METERED));
    let error = engine
        .query_prepared_governed_in_operation(&*data, &prepared, &[], QueryOptions::EMPTY, &state)
        .unwrap_err();
    assert!(error.message.contains("1.2-basic"), "{error:?}");
    assert_eq!(state.evidence().consumed_in(ResourceDimension::Fuel), 0);
}

#[test]
fn exists_limit_zero_preserves_a_hard_error_in_an_aggregate_sort_key() {
    let data = RdfDatasetBuilder::new().freeze().unwrap();
    let engine = NativeSparqlEngine::new();
    let body = "SELECT (FOLD(?v ORDER BY <http://example.org/undefined>(?v)) AS ?list) \
                WHERE { VALUES ?v { 1 } } LIMIT 0";
    let outside = engine.query(&data, request(body)).unwrap_err();
    assert_eq!(outside.code, "native-sparql-custom-function");
    let query = format!("ASK {{ FILTER EXISTS {{ {body} }} }}");
    let inside = engine.query(&data, request(&query)).unwrap_err();
    assert_eq!(inside.code, outside.code);
}

#[test]
fn explanation_accounts_for_the_pattern_reached_from_an_aggregate_sort_key() {
    let data = RdfDatasetBuilder::new().freeze().unwrap();
    let query = "SELECT (FOLD(?v ORDER BY ASC(EXISTS { VALUES ?sort_only { 1 } })) \
                 AS ?list) WHERE { VALUES ?v { 1 } }";
    let explanation = NativeSparqlEngine::new()
        .explain_query(&data, query, None)
        .unwrap();
    let values: Vec<_> = explanation
        .ledger()
        .iter()
        .filter(|node| node.label == "Values")
        .collect();
    assert_eq!(
        values.len(),
        2,
        "both VALUES sites must appear in the ledger"
    );
    assert!(values.iter().all(|node| node.fuel_total() > 0));
    assert_eq!(
        explanation
            .ledger()
            .iter()
            .map(purrdf_sparql_eval::NodeCharges::fuel_total)
            .sum::<u64>(),
        explanation.evidence().consumed_in(ResourceDimension::Fuel)
    );
}
