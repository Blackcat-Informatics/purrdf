// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Dated native patterns through the public SPARQL execution doors.

#![cfg(not(target_arch = "wasm32"))]

mod support;

use std::sync::Arc;

use purrdf_core::term_fixture::empty_dataset;
use purrdf_core::xsd_regex::xpath::{Limits, Profile, Resource};
use purrdf_core::{RdfDiagnostic, SparqlEngine, SparqlRequest, SparqlResult, TermValue};
use purrdf_sparql_eval::{
    EvalError, ExtensionEnv, FallibleSparqlError, MemoryRelation, NativeSparqlEngine,
    PropertyFunctionRegistry, QueryGovernors, QueryOptions,
};
use support::{render_cell, solutions};

fn select(
    engine: &NativeSparqlEngine,
    expression: &str,
) -> Result<Vec<Option<TermValue>>, RdfDiagnostic> {
    let query = format!("SELECT ({expression} AS ?value) WHERE {{}}");
    let (_, mut rows) = solutions(engine.query(
        &empty_dataset(),
        SparqlRequest {
            query: &query,
            base_iri: None,
            substitutions: &[],
        },
    )?);
    assert_eq!(rows.len(), 1, "{query}");
    Ok(rows.remove(0))
}

#[test]
fn one_prepared_query_alternates_dated_laws_without_changing_compatibility() {
    let data = empty_dataset();
    let compatibility = NativeSparqlEngine::new();
    assert_eq!(compatibility.xpath_regex(), None);
    let query = r#"SELECT
        (REGEX("aa", "(a)\\1") AS ?backreference)
        (REGEX("a", "(?:a)") AS ?group)
        (REGEX("a.b", "a.b", "q") AS ?quoted)
        (REPLACE("aa", "(a)\\1", "$1X") AS ?replacement)
        WHERE {}"#;
    let prepared = compatibility.prepare_query(query, None).unwrap();
    for (profile, expected) in [
        (Profile::Xpath31, ["true", "true", "true", "aX"]),
        (Profile::Xpath20, ["true", "UNBOUND", "UNBOUND", "aX"]),
        (Profile::Xpath31, ["true", "true", "true", "aX"]),
    ] {
        let engine = NativeSparqlEngine::new().with_xpath_regex(profile, Limits::new());
        assert_eq!(engine.xpath_regex(), Some((profile, Limits::new())));
        let (_, rows) = solutions(
            engine
                .query_prepared(&data, &prepared, &[], QueryOptions::EMPTY)
                .unwrap(),
        );
        assert_eq!(rows.len(), 1);
        let actual: Vec<_> = rows[0]
            .iter()
            .map(|cell| render_cell(cell.as_ref()))
            .collect();
        assert_eq!(actual, expected, "{}", profile.name());
    }
    let (_, rows) = solutions(
        compatibility
            .query_prepared(&data, &prepared, &[], QueryOptions::EMPTY)
            .unwrap(),
    );
    assert!(rows[0][0].is_none());
    assert_eq!(render_cell(rows[0][2].as_ref()), "true");
}

#[test]
fn one_engine_reuses_a_preparation_with_request_laws_and_current_bounds() {
    let data = empty_dataset();
    let engine = NativeSparqlEngine::new().with_xpath_regex(Profile::Xpath20, Limits::new());
    let query = r#"SELECT (REGEX("a", "(?:a)") AS ?value) WHERE {}"#;
    let prepared = engine.prepare_query(query, None).unwrap();
    assert_eq!(QueryOptions::EMPTY.xpath_regex(), None);
    for (profile, expected) in [
        (Profile::Xpath31, "true"),
        (Profile::Xpath20, "UNBOUND"),
        (Profile::Xpath31, "true"),
    ] {
        let options = QueryOptions::new().with_xpath_regex(profile, Limits::new());
        assert_eq!(options.xpath_regex(), Some((profile, Limits::new())));
        let (_, rows) = solutions(
            engine
                .query_prepared(&data, &prepared, &[], options)
                .unwrap(),
        );
        assert_eq!(render_cell(rows[0][0].as_ref()), expected);
    }
    for resource in [
        Resource::PatternBytes,
        Resource::CompileSteps,
        Resource::ProgramNodes,
        Resource::CompileSlots,
        Resource::MatchSteps,
    ] {
        let low =
            QueryOptions::new().with_xpath_regex(Profile::Xpath31, Limits::new().with(resource, 0));
        assert_eq!(
            engine
                .query_prepared(&data, &prepared, &[], low)
                .unwrap_err()
                .code,
            resource.code()
        );
    }
    let high = QueryOptions::new().with_xpath_regex(Profile::Xpath31, Limits::new());
    let (_, rows) = solutions(engine.query_prepared(&data, &prepared, &[], high).unwrap());
    assert_eq!(render_cell(rows[0][0].as_ref()), "true");
    let (_, rows) = solutions(
        engine
            .query_prepared(&data, &prepared, &[], QueryOptions::EMPTY)
            .unwrap(),
    );
    assert!(
        rows[0][0].is_none(),
        "the engine's original law is retained"
    );
}

#[test]
fn prepared_algebra_has_fresh_regex_construction_and_execution_bounds() {
    let data = empty_dataset();
    let engine = NativeSparqlEngine::new().with_xpath_regex(Profile::Xpath31, Limits::new());
    let prepared = engine
        .prepare_query(
            r#"SELECT (REGEX("a", "^(a|b)$") AS ?matched)
                (REPLACE("a", "^(a|b)$", "x") AS ?replaced) WHERE {}"#,
            None,
        )
        .unwrap();
    let no_compile = QueryOptions::new().with_xpath_regex(
        Profile::Xpath31,
        Limits::new().with(Resource::CompileSteps, 0),
    );
    // Query preparation retains algebra. Regex linking happens in each fresh
    // execution context, so no compiled artifact exists on this first request.
    assert_eq!(
        engine
            .query_prepared(&data, &prepared, &[], no_compile)
            .unwrap_err()
            .code,
        Resource::CompileSteps.code()
    );
    let high = QueryOptions::new().with_xpath_regex(Profile::Xpath31, Limits::new());
    let (_, rows) = solutions(engine.query_prepared(&data, &prepared, &[], high).unwrap());
    assert_eq!(render_cell(rows[0][0].as_ref()), "true");
    assert_eq!(render_cell(rows[0][1].as_ref()), "x");
    for resource in [
        Resource::CompileSteps,
        Resource::MatchSteps,
        Resource::OutputBytes,
    ] {
        let low =
            QueryOptions::new().with_xpath_regex(Profile::Xpath31, Limits::new().with(resource, 0));
        assert_eq!(
            engine
                .query_prepared(&data, &prepared, &[], low)
                .unwrap_err()
                .code,
            resource.code()
        );
    }
    let (_, rows) = solutions(engine.query_prepared(&data, &prepared, &[], high).unwrap());
    assert_eq!(render_cell(rows[0][0].as_ref()), "true");
    assert_eq!(render_cell(rows[0][1].as_ref()), "x");
}

#[test]
fn request_laws_reach_governed_queries_and_both_update_lanes() {
    let engine = NativeSparqlEngine::new().with_xpath_regex(
        Profile::Xpath20,
        Limits::new().with(Resource::MatchSteps, 0),
    );
    let data = empty_dataset();
    let high = QueryOptions::new().with_xpath_regex(Profile::Xpath31, Limits::new());
    let low = high.with_xpath_regex(
        Profile::Xpath31,
        Limits::new().with(Resource::MatchSteps, 0),
    );
    let ask = SparqlRequest {
        query: r#"ASK { FILTER(REGEX("a", "(?:a)")) }"#,
        base_iri: None,
        substitutions: &[],
    };
    assert!(matches!(
        engine
            .query_governed(&data, ask, high, &QueryGovernors::METERED)
            .unwrap(),
        purrdf_sparql_eval::GovernedOutcome::Complete {
            result: SparqlResult::Boolean(true),
            ..
        }
    ));
    assert_eq!(
        engine
            .query_governed(&data, ask, low, &QueryGovernors::METERED)
            .unwrap_err()
            .code,
        Resource::MatchSteps.code()
    );
    let update = SparqlRequest {
        query: r#"INSERT DATA { <http://example.org/a> <http://example.org/p> <http://example.org/b> };
            INSERT { <http://example.org/c> <http://example.org/p> <http://example.org/d> }
            WHERE { FILTER(REGEX("a", "(?:a)")) }"#,
        base_iri: None,
        substitutions: &[],
    };
    for governed in [false, true] {
        let original = empty_dataset();
        let mut changed = Arc::clone(&original);
        let error = if governed {
            engine
                .update_governed(&mut changed, update, low, &QueryGovernors::METERED)
                .unwrap_err()
        } else {
            engine
                .update_with_options(&mut changed, update, low)
                .unwrap_err()
        };
        assert_eq!(error.code, Resource::MatchSteps.code());
        assert!(Arc::ptr_eq(&changed, &original));
        if governed {
            engine
                .update_governed(&mut changed, update, high, &QueryGovernors::METERED)
                .unwrap();
        } else {
            engine
                .update_with_options(&mut changed, update, high)
                .unwrap();
        }
        assert_eq!(changed.quad_count(), 2);
    }
}

#[test]
fn constant_and_dynamic_patterns_refuse_every_resource_in_its_own_channel() {
    for expression in [
        r#"REGEX("ab", "(a|ab)")"#,
        r#"REGEX("ab", CONCAT("(a|", "ab)"))"#,
    ] {
        for resource in [
            Resource::PatternBytes,
            Resource::CompileSteps,
            Resource::ProgramNodes,
            Resource::CompileSlots,
            Resource::MatchSteps,
            Resource::MatchStates,
            Resource::MatchSlots,
        ] {
            let engine = NativeSparqlEngine::new()
                .with_xpath_regex(Profile::Xpath31, Limits::new().with(resource, 0));
            let error = select(&engine, expression).unwrap_err();
            assert_eq!(error.code, resource.code(), "{expression}: {resource:?}");
        }
        let engine = NativeSparqlEngine::new().with_xpath_regex(Profile::Xpath31, Limits::new());
        assert_eq!(
            render_cell(select(&engine, expression).unwrap()[0].as_ref()),
            "true"
        );
    }
}

#[test]
fn replacement_language_errors_are_unbound_but_output_refusal_aborts() {
    let engine = NativeSparqlEngine::new().with_xpath_regex(Profile::Xpath31, Limits::new());
    for expression in [
        r#"REPLACE("a", "a?", "x")"#,
        r#"REPLACE("a", "a", "$")"#,
        r#"REGEX("a", "[", "")"#,
        r#"REGEX("a", "a", "z")"#,
    ] {
        assert_eq!(
            select(&engine, expression).unwrap(),
            vec![None],
            "{expression}"
        );
    }
    assert_eq!(
        render_cell(select(&engine, r#"REPLACE("a", "a", "$", "q")"#).unwrap()[0].as_ref()),
        "$"
    );
    for expression in [
        r#"REPLACE("aa", "a", "é")"#,
        r#"REPLACE("aa", CONCAT("", "a"), "é")"#,
    ] {
        let low = NativeSparqlEngine::new().with_xpath_regex(
            Profile::Xpath31,
            Limits::new().with(Resource::OutputBytes, 3),
        );
        assert_eq!(
            select(&low, expression).unwrap_err().code,
            Resource::OutputBytes.code()
        );
        let admitted = NativeSparqlEngine::new().with_xpath_regex(
            Profile::Xpath31,
            Limits::new().with(Resource::OutputBytes, 4),
        );
        assert_eq!(
            render_cell(select(&admitted, expression).unwrap()[0].as_ref()),
            "éé"
        );
    }
}

#[test]
fn operational_failures_survive_negation_coalesce_logic_and_exists() {
    let engine = NativeSparqlEngine::new().with_xpath_regex(
        Profile::Xpath31,
        Limits::new().with(Resource::MatchSteps, 0),
    );
    for expression in [
        r#"!REGEX("a", "a")"#,
        r#"COALESCE(REGEX("a", "a"), true)"#,
        r#"IF(REGEX("a", "a"), true, false)"#,
        r#"REGEX("a", "a") || true"#,
        r#"REGEX("a", "a") && false"#,
        r#"!EXISTS { FILTER(REGEX("a", "a")) }"#,
    ] {
        assert_eq!(
            select(&engine, expression).unwrap_err().code,
            Resource::MatchSteps.code()
        );
    }
}

#[test]
fn a_linked_refusal_in_an_unselected_lazy_branch_does_not_execute() {
    let engine = NativeSparqlEngine::new().with_xpath_regex(
        Profile::Xpath31,
        Limits::new().with(Resource::CompileSteps, 0),
    );
    for expression in [
        r#"IF(true, "selected", REGEX("a", "a"))"#,
        r#"COALESCE("selected", REGEX("a", "a"))"#,
    ] {
        assert_eq!(
            render_cell(select(&engine, expression).unwrap()[0].as_ref()),
            "selected"
        );
    }
}

#[test]
fn prepared_execution_does_not_retain_an_earlier_admission_or_refusal() {
    let data = empty_dataset();
    for query in [
        r#"ASK { FILTER(REGEX("ab", "(a|ab)")) }"#,
        r#"ASK { FILTER(REGEX("ab", CONCAT("(a|", "ab)"))) }"#,
    ] {
        let prepared = NativeSparqlEngine::new()
            .prepare_query(query, None)
            .unwrap();
        for resource in [
            Resource::PatternBytes,
            Resource::ProgramNodes,
            Resource::CompileSlots,
            Resource::MatchSteps,
        ] {
            for limit in [None, Some(0), None] {
                let limits =
                    limit.map_or_else(Limits::new, |limit| Limits::new().with(resource, limit));
                let engine = NativeSparqlEngine::new().with_xpath_regex(Profile::Xpath31, limits);
                let result = engine.query_prepared(&data, &prepared, &[], QueryOptions::EMPTY);
                if limit.is_some() {
                    assert_eq!(result.unwrap_err().code, resource.code());
                } else {
                    assert!(matches!(result.unwrap(), SparqlResult::Boolean(true)));
                }
            }
        }
    }
}

#[test]
fn governed_and_fallible_execution_refuse_instead_of_certifying_partial_rows() {
    let data = empty_dataset();
    let engine = NativeSparqlEngine::new().with_xpath_regex(
        Profile::Xpath31,
        Limits::new().with(Resource::MatchSteps, 0),
    );
    let query = r#"SELECT ?value WHERE { VALUES ?value { "a" "b" } FILTER(REGEX(?value, "a")) }"#;
    let request = SparqlRequest {
        query,
        base_iri: None,
        substitutions: &[],
    };
    let error = engine
        .query_governed(
            &data,
            request,
            QueryOptions::EMPTY,
            &QueryGovernors::METERED,
        )
        .unwrap_err();
    assert_eq!(error.code, Resource::MatchSteps.code());
    let error = engine
        .query_fallible_view(&*data, request, QueryOptions::EMPTY)
        .unwrap_err();
    let FallibleSparqlError::Query { diagnostic, .. } = error else {
        panic!("native XPath refusal must not become a view failure or a partial answer");
    };
    assert_eq!(diagnostic.code, Resource::MatchSteps.code());
}

#[test]
fn update_where_refusal_rolls_back_the_entire_request() {
    let mut data = empty_dataset();
    let original = Arc::clone(&data);
    let query = r#"INSERT DATA { <http://example.org/earlier> <http://example.org/p> <http://example.org/o> };
        INSERT { <http://example.org/later> <http://example.org/p> <http://example.org/o> }
        WHERE { FILTER(REGEX("a", "a")) }"#;
    let request = SparqlRequest {
        query,
        base_iri: None,
        substitutions: &[],
    };
    let low = NativeSparqlEngine::new().with_xpath_regex(
        Profile::Xpath31,
        Limits::new().with(Resource::MatchSteps, 0),
    );
    assert_eq!(
        low.update_with_options(&mut data, request, QueryOptions::EMPTY)
            .unwrap_err()
            .code,
        Resource::MatchSteps.code()
    );
    assert!(Arc::ptr_eq(&data, &original));
    let high = NativeSparqlEngine::new().with_xpath_regex(Profile::Xpath31, Limits::new());
    high.update_with_options(&mut data, request, QueryOptions::EMPTY)
        .unwrap();
    assert_eq!(data.quad_count(), 2);
}

#[test]
fn on_demand_row_filters_use_the_same_law_and_typed_refusal() {
    let data = empty_dataset();
    let iri = "http://example.org/native-pattern-rows";
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(
        iri,
        Arc::new(
            MemoryRelation::new(
                1,
                1,
                vec![vec![
                    TermValue::iri("http://example.org/subject"),
                    TermValue::Literal {
                        lexical_form: "aa".to_owned(),
                        datatype: purrdf_xsd::datatype::XSD_STRING.to_owned(),
                        language: None,
                        direction: None,
                    },
                ]],
            )
            .unwrap(),
        ),
    );
    let env = ExtensionEnv::over_relations(registry).unwrap();
    let options = QueryOptions::new().with_env(&env);
    let query = format!(
        r#"SELECT ?subject ?text WHERE {{ ?subject <{iri}> ?text . FILTER(REGEX(?text, "(a)\\1")) }}"#
    );
    let high = NativeSparqlEngine::new().with_xpath_regex(Profile::Xpath31, Limits::new());
    let prepared = high
        .prepare_query_with_options(&query, None, options)
        .unwrap();
    let mut cursor = high.open_call_cursor(&prepared, options).unwrap();
    assert!(cursor.next_row(&*data).unwrap().is_some());
    assert!(cursor.next_row(&*data).unwrap().is_none());
    let low = NativeSparqlEngine::new().with_xpath_regex(
        Profile::Xpath31,
        Limits::new().with(Resource::MatchSteps, 0),
    );
    let mut cursor = low.open_call_cursor(&prepared, options).unwrap();
    let EvalError::XPathRegex(purrdf_core::xsd_regex::xpath::Error::Resource(refusal)) =
        cursor.next_row(&*data).unwrap_err()
    else {
        panic!("on-demand native matcher must preserve its typed resource cause");
    };
    assert_eq!(refusal.resource, Resource::MatchSteps);
    for (engine, limits, permitted) in [
        (&low, Limits::new(), true),
        (&high, Limits::new().with(Resource::MatchSteps, 0), false),
    ] {
        let options = options.with_xpath_regex(Profile::Xpath31, limits);
        let mut cursor = engine.open_call_cursor(&prepared, options).unwrap();
        let row = cursor.next_row(&*data);
        if permitted {
            assert!(row.unwrap().is_some());
            assert!(cursor.next_row(&*data).unwrap().is_none());
        } else {
            assert!(matches!(
                row,
                Err(EvalError::XPathRegex(
                    purrdf_core::xsd_regex::xpath::Error::Resource(refusal)
                )) if refusal.resource == Resource::MatchSteps
            ));
        }
    }
}
