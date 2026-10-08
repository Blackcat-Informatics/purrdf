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
fn native_storage_refusals_preserve_the_resource_code_and_typed_cause() {
    use purrdf_core::xsd_regex::xpath::Error as NativeError;

    for resource in [
        Resource::PatternBytes,
        Resource::CompileSteps,
        Resource::ProgramNodes,
        Resource::CompileSlots,
        Resource::MatchSteps,
        Resource::MatchStates,
        Resource::MatchSlots,
        Resource::OutputBytes,
    ] {
        // Exercise the public conversion with a typed host refusal. This does
        // not depend on forcing the host allocator to exhaust its memory.
        let cause = NativeError::Allocation { resource, units: 7 };
        let error = EvalError::XPathRegex(cause.clone());
        assert_eq!(
            error.diagnostic_code(),
            Some(resource.code()),
            "{resource:?}"
        );
        assert_eq!(error.code(), Some(resource.code()), "{resource:?}");
        assert_eq!(
            std::error::Error::source(&error)
                .unwrap()
                .downcast_ref::<NativeError>(),
            Some(&cause),
        );
    }
}

#[test]
fn contextual_reassignment_reuses_current_dated_laws_and_refuses_resource_exhaustion() {
    let data = empty_dataset();
    let engine = NativeSparqlEngine::new().with_xpath_regex(Profile::Xpath20, Limits::new());
    let query = r#"SELECT ?carrier ?value WHERE {
        VALUES ?carrier { 1 }
        BIND(0 AS ?value)
        BIND(REGEX("a", "(?:a)") AS ?value)
    }"#;
    let prepared = engine
        .prepare_rdflib_query(query, None, &[], QueryOptions::EMPTY)
        .unwrap();
    for (profile, expected) in [
        (Profile::Xpath31, "true"),
        (Profile::Xpath20, "0"),
        (Profile::Xpath31, "true"),
    ] {
        let options = QueryOptions::new().with_xpath_regex(profile, Limits::new());
        let (_, rows) = solutions(
            engine
                .query_rdflib_prepared_view(&data, &prepared, options)
                .unwrap(),
        );
        assert_eq!(rows.len(), 1);
        assert_eq!(render_cell(rows[0][0].as_ref()), "1");
        // A syntax error retains the previous actual assignment. A successful
        // rebind replaces it, independently of the engine's default dated law.
        assert_eq!(render_cell(rows[0][1].as_ref()), expected);
    }
    let exhausted = QueryOptions::new().with_xpath_regex(
        Profile::Xpath31,
        Limits::new().with(Resource::PatternBytes, 0),
    );
    assert_eq!(
        engine
            .query_rdflib_prepared_view(&data, &prepared, exhausted)
            .unwrap_err()
            .code,
        Resource::PatternBytes.code(),
        "resource exhaustion must not restore the earlier assignment"
    );
    let (_, rows) = solutions(
        engine
            .query_rdflib_prepared_view(
                &data,
                &prepared,
                QueryOptions::new().with_xpath_regex(Profile::Xpath31, Limits::new()),
            )
            .unwrap(),
    );
    assert_eq!(render_cell(rows[0][1].as_ref()), "true");
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

#[test]
fn explain_evaluates_under_the_requests_dated_law() {
    let data = Arc::new(empty_dataset());
    let engine = NativeSparqlEngine::new();
    let query = r#"SELECT (REGEX("aa", "^(a)\\1$") AS ?m) WHERE {}"#;
    let withheld = QueryOptions::new().with_xpath_regex(
        Profile::Xpath31,
        Limits::new().with(Resource::MatchSteps, 0),
    );
    // The request's law runs the pattern, so its exhausted bound is the
    // explanation's operational failure rather than a compatibility verdict.
    let error = engine
        .explain_query_with_options(&data, query, None, withheld)
        .unwrap_err();
    assert_eq!(error.code, Resource::MatchSteps.code());
    for options in [
        QueryOptions::new().with_xpath_regex(Profile::Xpath20, Limits::new()),
        QueryOptions::new().with_xpath_regex(Profile::Xpath31, Limits::new()),
        QueryOptions::EMPTY,
    ] {
        engine
            .explain_query_with_options(&data, query, None, options)
            .unwrap();
    }
}

/// `ex:{subject} ex:v "{text}"` for each named text.
fn texts(values: &[(&str, &str)]) -> Arc<purrdf_core::RdfDataset> {
    let mut builder = purrdf_core::RdfDatasetBuilder::new();
    let predicate = builder.intern_iri("http://example.org/v");
    for &(subject, text) in values {
        let subject = builder.intern_iri(&format!("http://example.org/{subject}"));
        let value = builder.intern_literal(purrdf_core::RdfLiteral::simple(text));
        builder.push_quad(subject, predicate, value, None);
    }
    builder.freeze().unwrap()
}

/// The single cell of `SELECT ?r WHERE { ex:{subject} ex:v ?t BIND({call} AS ?r) }`.
fn bound(
    engine: &NativeSparqlEngine,
    data: &Arc<purrdf_core::RdfDataset>,
    subject: &str,
    call: &str,
) -> Result<String, RdfDiagnostic> {
    let query = format!(
        "SELECT ?r WHERE {{ <http://example.org/{subject}> <http://example.org/v> ?t \
         BIND({call} AS ?r) }}"
    );
    let (_, rows) = solutions(engine.query(
        data,
        SparqlRequest {
            query: &query,
            base_iri: None,
            substitutions: &[],
        },
    )?);
    assert_eq!(rows.len(), 1, "{query}");
    Ok(render_cell(rows[0][0].as_ref()))
}

#[test]
fn adversary_shapes_answer_at_the_production_defaults_like_the_compatibility_engine() {
    // Each shape was refused by the native matcher at a fraction of these sizes:
    // multiple unbounded runs at 8 KB, group repetition from 44 KB, a word
    // repetition at 159 KB, and the nested nullable repetition at 41 bytes.
    let text = purrdf_testkit::text::word_prose(1 << 20);
    let pairs = "ab".repeat(1 << 19);
    let forty = "a".repeat(40);
    let [zzz, bang, spaced, pairs_c, pairs_a, forty_b, forty_c] = [
        format!("{text} zzz"),
        format!("{text}!"),
        format!("{text} "),
        format!("{pairs}c"),
        format!("{pairs}a"),
        format!("{forty}b"),
        format!("{forty}c"),
    ];
    let data = texts(&[
        ("prose", &text),
        ("zzz", &zzz),
        ("bang", &bang),
        ("spaced", &spaced),
        ("pairs", &pairs),
        ("pairs_c", &pairs_c),
        ("pairs_a", &pairs_a),
        ("forty_b", &forty_b),
        ("forty_c", &forty_c),
    ]);
    let compatibility = NativeSparqlEngine::new();
    let laws = [Profile::Xpath20, Profile::Xpath31]
        .map(|profile| NativeSparqlEngine::new().with_xpath_regex(profile, Limits::new()));
    for (pattern, subject, expected) in [
        ("node.*graph.*zzz", "prose", "false"),
        ("node.*graph.*zzz", "zzz", "true"),
        ("alpha.*zzz", "prose", "false"),
        ("alpha.*zzz", "zzz", "true"),
        ("^([a-z]+ ?)+$", "prose", "true"),
        ("^([a-z]+ ?)+$", "bang", "false"),
        (r"^(\w+\s)*\w+$", "prose", "true"),
        (r"^(\w+\s)*\w+$", "spaced", "false"),
        ("^(a|b)*$", "pairs", "true"),
        ("^(a|b)*$", "pairs_c", "false"),
        ("^(ab)*$", "pairs", "true"),
        ("^(ab)*$", "pairs_a", "false"),
        ("^(?:ab)*$", "pairs", "true"),
        ("^(?:ab)*$", "pairs_a", "false"),
        ("^(a|aa)*$|^(a*)*b$", "forty_b", "true"),
        ("^(a|aa)*$|^(a*)*b$", "forty_c", "false"),
    ] {
        let call = format!(
            "REGEX(?t, {})",
            purrdf_testkit::text::sparql_string(pattern)
        );
        assert_eq!(
            bound(&compatibility, &data, subject, &call).unwrap(),
            expected,
            "compatibility {pattern} on {subject}"
        );
        for (engine, profile) in laws.iter().zip([Profile::Xpath20, Profile::Xpath31]) {
            if profile == Profile::Xpath20 && pattern.contains("(?:") {
                continue;
            }
            assert_eq!(
                bound(engine, &data, subject, &call)
                    .unwrap_or_else(|error| panic!("{profile:?} {pattern}: {}", error.code)),
                expected,
                "{profile:?} {pattern} on {subject}"
            );
        }
    }
    // A replacement keeps the last iteration's capture over the whole input.
    for engine in std::iter::once(&compatibility).chain(&laws) {
        assert_eq!(
            bound(engine, &data, "pairs", r#"REPLACE(?t, "^(a|b)+$", "[$1]")"#).unwrap(),
            "[b]"
        );
    }
}

#[test]
fn backreference_blowups_still_refuse_beside_an_answered_neighbour() {
    let forty = "a".repeat(40);
    let neighbour = format!("{forty}ca");
    let data = texts(&[("refused", &forty), ("neighbour", &neighbour)]);
    let call = format!(
        "REGEX(?t, {})",
        purrdf_testkit::text::sparql_string(r"^(a|aa)*c\1$")
    );
    for profile in [Profile::Xpath20, Profile::Xpath31] {
        let engine = NativeSparqlEngine::new().with_xpath_regex(profile, Limits::new());
        let refusal = bound(&engine, &data, "refused", &call).unwrap_err();
        assert!(
            [
                Resource::MatchSteps,
                Resource::MatchStates,
                Resource::MatchSlots
            ]
            .iter()
            .any(|resource| refusal.code == resource.code()),
            "{profile:?}: {}",
            refusal.code
        );
        assert_eq!(
            bound(&engine, &data, "neighbour", &call).unwrap(),
            "true",
            "{profile:?}"
        );
    }
}

#[test]
fn counted_repetition_shapes_answer_at_the_production_defaults_like_the_compatibility_engine() {
    // Each shape was refused by the native matcher at these sizes: a counted
    // group repeated from every start kept one thread per distinct count.
    let pairs = "ab".repeat(1 << 19);
    let mixed = "abbaab".repeat((4 << 20) / 6);
    let prose = purrdf_testkit::text::word_prose(8 << 20);
    let values = [
        ("pairs_128k", pairs[..128 << 10].to_owned()),
        ("pairs", pairs.clone()),
        ("quads", "abcd".repeat(1 << 18)),
        ("mixed_400k", mixed[..400 << 10].to_owned()),
        ("mixed_800k", mixed[..800 << 10].to_owned()),
        ("mixed", mixed.clone()),
        (
            "prose_4m",
            prose[..prose[..4 << 20].rfind(' ').unwrap()].to_owned(),
        ),
        ("prose", prose.clone()),
    ];
    let mut named: Vec<(String, String)> = Vec::new();
    for (subject, value) in &values {
        named.push(((*subject).to_owned(), value.clone()));
        for suffix in ["c", "e", " zzz"] {
            named.push((
                format!("{subject}_{}", suffix.trim()),
                format!("{value}{suffix}"),
            ));
        }
    }
    named.push(("empty".to_owned(), String::new()));
    // A long run that an ambiguous counted body matches only at its end, and a
    // mebibyte of short runs that one replacement rewrites thousands of times.
    let run = "a".repeat(1 << 20);
    named.push(("run".to_owned(), run.clone()));
    named.push(("run_b".to_owned(), format!("{run}b")));
    let mut runs = "a".repeat(100_000);
    let mut index = 0_usize;
    while runs.len() < 1 << 20 {
        runs.push_str(&"a".repeat(1 + (index * 7 + index / 5) % 13));
        runs.push(if index.is_multiple_of(3) { 'c' } else { 'b' });
        index += 1;
    }
    named.push(("runs".to_owned(), runs));
    let data = texts(
        &named
            .iter()
            .map(|(subject, value)| (subject.as_str(), value.as_str()))
            .collect::<Vec<_>>(),
    );
    let compatibility = NativeSparqlEngine::new();
    let laws = [Profile::Xpath20, Profile::Xpath31]
        .map(|profile| NativeSparqlEngine::new().with_xpath_regex(profile, Limits::new()));
    for (pattern, subject, expected) in [
        ("(ab){1,1000}c", "pairs_128k", "false"),
        ("(ab){1,1000}c", "pairs_128k_c", "true"),
        ("(ab){2,50}c", "pairs", "false"),
        ("(ab){2,50}c", "pairs_c", "true"),
        ("(ab){1,100}c", "pairs", "false"),
        ("(ab){1,100}c", "pairs_c", "true"),
        ("(ab|cd){1,20}e", "quads", "false"),
        ("(ab|cd){1,20}e", "quads_e", "true"),
        ("((a|b){3}){5,9}c", "mixed_400k", "false"),
        ("((a|b){3}){5,9}c", "mixed_400k_c", "true"),
        ("((a|b){2}){2,5}c", "mixed_800k", "false"),
        ("((a|b){2}){2,5}c", "mixed_800k_c", "true"),
        ("(a|b){1,30}c", "mixed", "false"),
        ("(a|b){1,30}c", "mixed_c", "true"),
        ("(a|b){3,9}c", "mixed", "false"),
        ("(a|b){3,9}c", "mixed_c", "true"),
        (r"(\w+\s){3,5}zzz", "prose_4m", "false"),
        (r"(\w+\s){3,5}zzz", "prose_4m_zzz", "true"),
        ("node.*graph.*zzz", "prose", "false"),
        ("node.*graph.*zzz", "prose_zzz", "true"),
        ("(a|aa){1,1000}b", "run", "false"),
        ("(a|aa){1,1000}b", "run_b", "true"),
        // The compatibility engine refuses to build these repetition counts.
        ("(ab){1,100000}c", "pairs_128k", "false"),
        ("(ab){1,100000}c", "pairs_128k_c", "true"),
        ("(a?){18446744073709551616}", "empty", "true"),
        ("^(a?){18446744073709551616}$", "pairs_128k", "false"),
    ] {
        let call = format!(
            "REGEX(?t, {})",
            purrdf_testkit::text::sparql_string(pattern)
        );
        if !pattern.contains("{1,100000}") && !pattern.contains("18446744073709551616") {
            assert_eq!(
                bound(&compatibility, &data, subject, &call).unwrap(),
                expected,
                "compatibility {pattern} on {subject}"
            );
        }
        for (engine, profile) in laws.iter().zip(Profile::ALL) {
            assert_eq!(
                bound(engine, &data, subject, &call)
                    .unwrap_or_else(|error| panic!("{profile:?} {pattern}: {}", error.code)),
                expected,
                "{profile:?} {pattern} on {subject}"
            );
        }
    }
    // Replacement reads the last iteration's capture of the leftmost match.
    for (pattern, subject) in [
        ("(ab){1,1000}c", "pairs_128k_c"),
        ("(ab|cd){1,20}e", "quads_e"),
        ("((a|b){3}){5,9}c", "mixed_400k_c"),
        ("(a|b){1,30}c", "mixed_c"),
        (r"(\w+\s){3,5}zzz", "prose_4m_zzz"),
        ("(a|aa){1,1000}b", "run_b"),
        ("(a|aa){1,1000}b", "runs"),
        ("(a|aa){2,5}b", "runs"),
        ("((a|aa){1,4})(b|c)", "runs"),
    ] {
        let call = format!(
            "STRLEN(REPLACE(?t, {}, \"[$1]\"))",
            purrdf_testkit::text::sparql_string(pattern)
        );
        let expected = bound(&compatibility, &data, subject, &call).unwrap();
        for (engine, profile) in laws.iter().zip(Profile::ALL) {
            assert_eq!(
                bound(engine, &data, subject, &call)
                    .unwrap_or_else(|error| panic!("{profile:?} {pattern}: {}", error.code)),
                expected,
                "{profile:?} {pattern} on {subject}"
            );
        }
        let call = format!(
            "REPLACE(?t, {}, \"[$1]\")",
            purrdf_testkit::text::sparql_string(pattern)
        );
        let expected = bound(&compatibility, &data, subject, &call).unwrap();
        for engine in &laws {
            assert_eq!(bound(engine, &data, subject, &call).unwrap(), expected);
        }
    }
}
