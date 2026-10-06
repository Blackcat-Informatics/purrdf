// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Dated native pattern selection through SHACL's actual preparation and binding doors.

#![cfg(not(target_arch = "wasm32"))]

#[path = "support/terms.rs"]
mod terms;
#[path = "support/turtle.rs"]
mod turtle;

use std::error::Error as _;
use std::fmt::Write as _;
use std::sync::Arc;

use purrdf_core::xsd_regex::xpath::{Error, Limits, Profile, Resource};
use purrdf_rdf::ir::{MutableDataset, ViewLimits};
use purrdf_shapes::data::ShaclData;
use purrdf_shapes::data_view::ShaclDatasetView;
use purrdf_shapes::engine::{
    GovernedValidation, PreparedShapes, ValidationOptions, project_dataset,
};
use purrdf_shapes::product::{HostBindings, ShapesProduct, ShapesProfile};
use purrdf_shapes::xpath::{
    XPathValidationError, validate_dataset, validate_dataset_with_governors,
};
use purrdf_sparql_eval::QueryGovernors;
use terms::example_org as ex;

const PREFIXES: &str = "
@prefix ex: <http://example.org/> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
";

#[test]
fn native_backreferences_reach_every_focus_door_without_changing_legacy() {
    let data = turtle::data(PREFIXES, r#"ex:n ex:p "aa" ."#);
    let preparation = PreparedShapes::new(Arc::new(turtle::loads(
        PREFIXES,
        r#"ex:S a sh:NodeShape; sh:targetNode ex:n;
            sh:property [ sh:path ex:p; sh:pattern "(a)\\1" ] ."#,
    )));
    let legacy = preparation.bind_shared_dataset(Arc::clone(&data)).unwrap();
    let original = legacy.validate().unwrap().to_ntriples();
    assert!(!legacy.validate().unwrap().conforms);
    for profile in [Profile::Xpath20, Profile::Xpath31, Profile::Xpath20] {
        let selected = preparation.clone().with_xpath_regex(profile, Limits::new());
        let bound = selected.bind_shared_dataset(Arc::clone(&data)).unwrap();
        assert_eq!(bound.selection(), (profile, Limits::new()));
        let id = bound.term_id(&ex("n")).unwrap();
        for report in [
            bound.validate().unwrap(),
            bound.validate_focus_nodes(&[ex("n")]).unwrap(),
            bound.validate_focus_node_ids(&[id]).unwrap(),
            selected.bind_dataset(&data).unwrap().validate().unwrap(),
            validate_dataset(
                &data,
                Arc::clone(preparation.shapes()),
                profile,
                Limits::new(),
            )
            .unwrap(),
        ] {
            assert!(report.conforms && report.results.is_empty());
        }
        assert_eq!(legacy.validate().unwrap().to_ntriples(), original);
    }
}

#[test]
fn every_binding_route_retains_the_selected_pattern_law() {
    let data = turtle::data(PREFIXES, r#"ex:n ex:p "aa" ."#);
    let projected = project_dataset(&data).unwrap();
    let snapshot = Arc::new(
        MutableDataset::new(Arc::clone(&data))
            .snapshot_view()
            .unwrap(),
    );
    let preparation = PreparedShapes::new(Arc::new(turtle::loads(
        PREFIXES,
        r#"ex:S a sh:NodeShape; sh:targetNode ex:n;
            sh:property [ sh:path ex:p; sh:pattern "(a)\\1" ] ."#,
    )));
    for profile in [Profile::Xpath20, Profile::Xpath31] {
        let selected = preparation.clone().with_xpath_regex(profile, Limits::new());
        let bindings = [
            selected
                .bind(ShaclData::new(Arc::clone(&data), Arc::clone(&data), None))
                .unwrap(),
            selected.bind_dataset(&data).unwrap(),
            selected.bind_shared_dataset(Arc::clone(&data)).unwrap(),
            selected
                .bind_view(Arc::new(ShaclDatasetView::project(Arc::clone(&data))))
                .unwrap(),
            selected
                .bind_shared_dataset_with_shapes_graph(
                    Arc::clone(&data),
                    Some("http://example.org/shapes"),
                    ViewLimits::default(),
                )
                .unwrap(),
            selected
                .bind_delta_with_shapes_graph(
                    Arc::clone(&snapshot),
                    Some("http://example.org/shapes"),
                    ViewLimits::default(),
                )
                .unwrap(),
            selected
                .bind_projected_dataset(Arc::clone(&projected))
                .unwrap(),
            selected
                .bind_projected_dataset_with_shapes_graph(
                    Arc::clone(&projected),
                    Some("http://example.org/shapes"),
                )
                .unwrap(),
        ];
        for bound in bindings {
            assert_eq!(bound.selection(), (profile, Limits::new()));
            let id = bound.term_id(&ex("n")).unwrap();
            for report in [
                bound.validate().unwrap(),
                bound.validate_focus_nodes(&[ex("n")]).unwrap(),
                bound.validate_focus_node_ids(&[id]).unwrap(),
            ] {
                assert!(report.conforms && report.results.is_empty());
            }
        }
    }
}

#[test]
fn governed_native_validation_keeps_budget_and_pattern_refusals_distinct() {
    let data = turtle::data(PREFIXES, r#"ex:n ex:p "aa" ."#);
    let selected = PreparedShapes::new(Arc::new(turtle::loads(
        PREFIXES,
        r#"ex:S a sh:NodeShape; sh:targetNode ex:n;
            sh:property [ sh:path ex:p; sh:pattern "(a)\\1" ] ."#,
    )))
    .with_xpath_regex(Profile::Xpath31, Limits::new());
    let zero = QueryGovernors::UNBOUNDED.with_fuel(0);
    for outcome in [
        selected
            .validate_with_governors(
                ShaclData::new(Arc::clone(&data), Arc::clone(&data), None),
                &zero,
            )
            .unwrap(),
        selected
            .validate_dataset_with_governors(&data, &zero)
            .unwrap(),
        selected
            .bind_shared_dataset(Arc::clone(&data))
            .unwrap()
            .validate_with_governors(&zero)
            .unwrap(),
        validate_dataset_with_governors(
            &data,
            Arc::clone(selected.preparation().shapes()),
            Profile::Xpath31,
            Limits::new(),
            &zero,
        )
        .unwrap(),
    ] {
        let GovernedValidation::Complete { report, .. } = outcome else {
            panic!("native Core pattern work spends no SPARQL fuel")
        };
        assert!(report.conforms && report.results.is_empty());
    }
    let low = selected.clone().with_xpath_regex(
        Profile::Xpath31,
        Limits::new().with(Resource::MatchSteps, 0),
    );
    assert!(matches!(
        low.validate_dataset_with_governors(&data, &zero),
        Err(XPathValidationError::Pattern(Error::Resource(refusal)))
            if refusal.resource == Resource::MatchSteps
    ));
    assert!(matches!(
        selected
            .validate_dataset_with_governors(&data, &zero)
            .unwrap(),
        GovernedValidation::Complete { .. }
    ));
}

#[test]
fn governed_target_and_constraint_queries_share_one_budget_without_partial_findings() {
    let data = turtle::data(PREFIXES, r#"ex:n ex:p "a" ."#);
    let selected = PreparedShapes::new(Arc::new(turtle::loads(
        PREFIXES,
        r#"ex:S a sh:NodeShape; sh:class ex:Missing;
            sh:target [ a sh:SPARQLTarget; sh:select '''
                SELECT ?this WHERE { ?this <http://example.org/p> ?value . FILTER(REGEX(?value, "(?:a)")) }
            ''' ];
            sh:sparql [ sh:select '''
                SELECT $this WHERE { FILTER(REGEX("aa", "(a)\\\\1")) }
            ''' ] ."#,
    )))
    .with_xpath_regex(Profile::Xpath31, Limits::new());
    let ordinary = selected.bind_shared_dataset(Arc::clone(&data)).unwrap();
    let expected = ordinary.validate().unwrap();
    assert_eq!(expected.results.len(), 2);
    let GovernedValidation::Complete { report, .. } = selected
        .validate_dataset_with_governors(&data, &QueryGovernors::UNBOUNDED)
        .unwrap()
    else {
        panic!("the unbounded valid query neighbor completes")
    };
    assert_eq!(report.to_ntriples(), expected.to_ntriples());
    let zero = QueryGovernors::UNBOUNDED.with_fuel(0);
    for outcome in [
        selected
            .validate_dataset_with_governors(&data, &zero)
            .unwrap(),
        ordinary.validate_with_governors(&zero).unwrap(),
    ] {
        assert!(matches!(
            outcome,
            GovernedValidation::BudgetExhausted { .. }
        ));
        assert!(outcome.tripped().is_some());
    }
    let low = selected.clone().with_xpath_regex(
        Profile::Xpath31,
        Limits::new().with(Resource::MatchSteps, 0),
    );
    assert!(matches!(
        low.validate_dataset_with_governors(&data, &QueryGovernors::UNBOUNDED),
        Err(XPathValidationError::Query(diagnostic))
            if diagnostic.code == Resource::MatchSteps.code()
    ));
    assert!(matches!(
        selected
            .validate_dataset_with_governors(&data, &QueryGovernors::UNBOUNDED)
            .unwrap(),
        GovernedValidation::Complete { .. }
    ));
}

#[test]
fn alternating_laws_and_failed_admission_preserve_the_actual_warm_program() {
    let data = turtle::data(PREFIXES, r#"ex:n ex:p "a" ."#);
    let selected = PreparedShapes::new(Arc::new(turtle::loads(
        PREFIXES,
        r#"ex:S a sh:NodeShape; sh:targetNode ex:n;
            sh:property [ sh:path ex:p; sh:pattern "(?:a|b)" ] ."#,
    )))
    .with_xpath_regex(Profile::Xpath31, Limits::new());
    selected.admit_declared_patterns().unwrap();
    assert!(
        selected
            .bind_shared_dataset(Arc::clone(&data))
            .unwrap()
            .validate()
            .unwrap()
            .conforms
    );
    let other = selected
        .clone()
        .with_xpath_regex(Profile::Xpath20, Limits::new());
    assert!(matches!(
        other.admit_declared_patterns(),
        Err(XPathValidationError::Pattern(Error::Syntax { .. }))
    ));
    let findings = other
        .bind_shared_dataset(Arc::clone(&data))
        .unwrap()
        .validate()
        .unwrap();
    assert!(!findings.conforms && findings.results.len() == 1);
    let warm = selected.with_xpath_regex(
        Profile::Xpath31,
        Limits::new().with(Resource::CompileSteps, 0),
    );
    warm.admit_declared_patterns().unwrap();
    assert!(
        warm.bind_shared_dataset(Arc::clone(&data))
            .unwrap()
            .validate()
            .unwrap()
            .conforms
    );
    for resource in [
        Resource::PatternBytes,
        Resource::ProgramNodes,
        Resource::CompileSlots,
        Resource::MatchSteps,
        Resource::MatchStates,
        Resource::MatchSlots,
    ] {
        let low = warm.clone().with_xpath_regex(
            Profile::Xpath31,
            Limits::new()
                .with(Resource::CompileSteps, 0)
                .with(resource, 0),
        );
        let error = low
            .bind_shared_dataset(Arc::clone(&data))
            .unwrap()
            .validate()
            .unwrap_err();
        let XPathValidationError::Pattern(Error::Resource(ref refusal)) = error else {
            panic!("expected the actual {resource:?} refusal: {error}")
        };
        assert_eq!(refusal.resource, resource);
        assert!(error.source().unwrap().downcast_ref::<Error>().is_some());
        assert!(
            warm.bind_shared_dataset(Arc::clone(&data))
                .unwrap()
                .validate()
                .unwrap()
                .conforms
        );
    }
}

#[test]
fn copy_on_write_validation_options_preserve_the_original_cache_owner() {
    let data = turtle::data(PREFIXES, r#"ex:n ex:p "aa" ."#);
    let selected = PreparedShapes::new(Arc::new(turtle::loads(
        PREFIXES,
        r#"ex:S a sh:NodeShape; sh:targetNode ex:n;
            sh:property [ sh:path ex:p; sh:pattern "(a)\\1" ] ."#,
    )))
    .with_xpath_regex(Profile::Xpath31, Limits::new());
    selected.admit_declared_patterns().unwrap();
    let warm = selected.clone().with_xpath_regex(
        Profile::Xpath31,
        Limits::new().with(Resource::CompileSteps, 0),
    );
    assert!(
        warm.bind_shared_dataset(Arc::clone(&data))
            .unwrap()
            .validate()
            .unwrap()
            .conforms
    );
    let copied = selected
        .preparation()
        .clone()
        .with_validation_options(
            ValidationOptions::default().with_subclass_of_in_shapes_graph(true),
        )
        .with_xpath_regex(
            Profile::Xpath31,
            Limits::new().with(Resource::CompileSteps, 0),
        );
    assert!(matches!(
        copied.admit_declared_patterns(),
        Err(XPathValidationError::Pattern(Error::Resource(refusal)))
            if refusal.resource == Resource::CompileSteps
    ));
    assert!(
        warm.bind_shared_dataset(Arc::clone(&data))
            .unwrap()
            .validate()
            .unwrap()
            .conforms
    );
    assert!(
        copied
            .with_xpath_regex(Profile::Xpath31, Limits::new())
            .bind_shared_dataset(data)
            .unwrap()
            .validate()
            .unwrap()
            .conforms
    );
}

#[test]
fn strict_source_preflight_checks_untargeted_declarations() {
    let data = turtle::data(PREFIXES, "");
    let selected = PreparedShapes::new(Arc::new(turtle::loads(
        PREFIXES,
        r#"ex:S a sh:NodeShape; sh:pattern "[" ."#,
    )))
    .with_xpath_regex(Profile::Xpath31, Limits::new());
    assert!(
        selected
            .bind_shared_dataset(data)
            .unwrap()
            .validate()
            .unwrap()
            .conforms
    );
    assert!(matches!(
        selected.admit_declared_patterns(),
        Err(XPathValidationError::Pattern(Error::Syntax { .. }))
    ));
}

#[test]
fn native_operational_failure_aborts_nested_boolean_constraints() {
    let data = turtle::data(PREFIXES, r#"ex:n ex:p "a" ."#);
    for nested in [
        r#"sh:node [ sh:pattern "a" ]"#,
        r#"sh:not [ sh:pattern "a" ]"#,
        r#"sh:and ( [ sh:pattern "a" ] )"#,
        r#"sh:or ( [ sh:pattern "a" ] )"#,
        r#"sh:xone ( [ sh:pattern "a" ] )"#,
    ] {
        let shapes = turtle::loads(
            PREFIXES,
            &format!(
                "ex:S a sh:NodeShape; sh:targetNode ex:n; sh:class ex:Missing; sh:property [ sh:path ex:p; {nested} ] ."
            ),
        );
        let preparation = PreparedShapes::new(Arc::new(shapes));
        let low = preparation.clone().with_xpath_regex(
            Profile::Xpath31,
            Limits::new().with(Resource::MatchSteps, 0),
        );
        assert!(
            matches!(low.bind_shared_dataset(Arc::clone(&data)).unwrap().validate(), Err(XPathValidationError::Pattern(Error::Resource(refusal))) if refusal.resource == Resource::MatchSteps),
            "{nested}"
        );
        let normal = preparation
            .with_xpath_regex(Profile::Xpath31, Limits::new())
            .bind_shared_dataset(Arc::clone(&data))
            .unwrap()
            .validate()
            .unwrap();
        assert!(
            !normal.conforms,
            "the independent class finding remains at admitted limits"
        );
    }
}

fn some_value_preparation(query: &str) -> PreparedShapes {
    PreparedShapes::new(Arc::new(turtle::loads(
        PREFIXES,
        &format!(
            "ex:S a sh:NodeShape; sh:targetNode ex:n;
                sh:property [ sh:path ex:p; sh:someValue [
                    sh:or ( [ sh:class ex:Duck ] [ sh:sparql [
                        sh:select '''{query}'''
                    ] ] )
                ] ] ."
        ),
    )))
}

#[test]
fn some_value_query_failure_is_discarded_when_a_later_value_conforms() {
    let data = turtle::data(
        PREFIXES,
        "ex:n ex:p ex:aBad, ex:zGood . ex:zGood a ex:Duck .",
    );
    let preparation = some_value_preparation(
        "SELECT $this WHERE { FILTER(<http://example.org/missingFunction>($this)) }",
    );
    assert!(
        preparation
            .bind_shared_dataset(Arc::clone(&data))
            .unwrap()
            .validate()
            .unwrap()
            .conforms
    );
    for profile in [Profile::Xpath20, Profile::Xpath31] {
        let selected = preparation.clone().with_xpath_regex(profile, Limits::new());
        let bound = selected.bind_shared_dataset(Arc::clone(&data)).unwrap();
        assert!(
            bound.term_id(&ex("aBad")).unwrap().term_id()
                < bound.term_id(&ex("zGood")).unwrap().term_id()
        );
        let report = bound.validate().unwrap();
        assert!(report.conforms && report.results.is_empty(), "{profile:?}");
        let error = selected
            .bind_dataset(&turtle::data(PREFIXES, "ex:n ex:p ex:aBad ."))
            .unwrap()
            .validate()
            .unwrap_err();
        assert!(
            error.to_string().contains("native-sparql-custom-function"),
            "{error}"
        );
    }
}

#[test]
fn some_value_query_resource_refusal_survives_a_later_conforming_value() {
    let data = turtle::data(
        PREFIXES,
        "ex:n ex:p ex:aBad, ex:zGood . ex:zGood a ex:Duck .",
    );
    let preparation =
        some_value_preparation("SELECT $this WHERE { FILTER(REGEX(STR($this), \"bad\", \"i\")) }");
    for profile in [Profile::Xpath20, Profile::Xpath31] {
        let selected = preparation.clone().with_xpath_regex(profile, Limits::new());
        let bound = selected.bind_shared_dataset(Arc::clone(&data)).unwrap();
        assert!(
            bound.term_id(&ex("aBad")).unwrap().term_id()
                < bound.term_id(&ex("zGood")).unwrap().term_id()
        );
        assert!(bound.validate().unwrap().conforms);
        let low = selected.with_xpath_regex(profile, Limits::new().with(Resource::MatchSteps, 0));
        assert!(
            matches!(low.bind_dataset(&data).unwrap().validate(), Err(XPathValidationError::Query(diagnostic))
                if diagnostic.code == Resource::MatchSteps.code()),
            "{profile:?}"
        );
    }
}

#[test]
fn target_binding_uses_the_selected_law_before_acquiring_focus_nodes() {
    let data = turtle::data(PREFIXES, r#"ex:n ex:p "a" ."#);
    let preparation = PreparedShapes::new(Arc::new(turtle::loads(
        PREFIXES,
        r#"
        ex:S a sh:NodeShape; sh:class ex:Missing;
            sh:target [ a sh:SPARQLTarget; sh:select '''
                SELECT ?this WHERE { ?this <http://example.org/p> ?value . FILTER(REGEX(?value, "(?:a)")) }
            ''' ] .
    "#,
    )));
    for (profile, count) in [
        (Profile::Xpath31, 1),
        (Profile::Xpath20, 0),
        (Profile::Xpath31, 1),
    ] {
        let bound = preparation
            .clone()
            .with_xpath_regex(profile, Limits::new())
            .bind_shared_dataset(Arc::clone(&data))
            .unwrap();
        assert_eq!(
            bound.validate().unwrap().results.len(),
            count,
            "{}",
            profile.name()
        );
    }
    let error = preparation
        .clone()
        .with_xpath_regex(
            Profile::Xpath31,
            Limits::new().with(Resource::MatchSteps, 0),
        )
        .bind_shared_dataset(Arc::clone(&data))
        .unwrap_err();
    let XPathValidationError::Query(diagnostic) = error else {
        panic!("target failure must retain its diagnostic: {error}")
    };
    assert_eq!(diagnostic.code, Resource::MatchSteps.code());
    assert_eq!(
        preparation
            .with_xpath_regex(Profile::Xpath31, Limits::new())
            .bind_shared_dataset(data)
            .unwrap()
            .validate()
            .unwrap()
            .results
            .len(),
        1
    );
}

#[test]
fn constant_dynamic_regex_and_replace_share_native_query_selection() {
    let data = turtle::data(PREFIXES, r#"ex:n ex:p "aa" ."#);
    for select in [
        r#"SELECT $this WHERE { FILTER(REGEX("aa", "(a)\\1")) }"#,
        r#"SELECT $this WHERE { $this <http://example.org/p> ?value . BIND("(a)\\1" AS ?pattern) FILTER(REGEX(?value, ?pattern)) }"#,
        r#"SELECT $this WHERE { FILTER(REPLACE("aa", "(a)\\1", "$1X") = "aX") }"#,
    ] {
        let query_literal = purrdf_lex::literal_escape::escape(
            select,
            purrdf_lex::literal_escape::Carrier::Canonical,
        );
        let preparation = PreparedShapes::new(Arc::new(turtle::loads(
            PREFIXES,
            &format!(
                "ex:S a sh:NodeShape; sh:targetNode ex:n; sh:sparql [ sh:select \"{query_literal}\" ] ."
            ),
        )));
        let selected = preparation
            .clone()
            .with_xpath_regex(Profile::Xpath31, Limits::new());
        assert_eq!(
            selected
                .bind_shared_dataset(Arc::clone(&data))
                .unwrap()
                .validate()
                .unwrap()
                .results
                .len(),
            1,
            "{select}"
        );
        let low = selected.with_xpath_regex(
            Profile::Xpath31,
            Limits::new().with(Resource::MatchSteps, 0),
        );
        assert!(
            matches!(low.bind_shared_dataset(Arc::clone(&data)).unwrap().validate(), Err(XPathValidationError::Query(diagnostic)) if diagnostic.code == Resource::MatchSteps.code()),
            "{select}"
        );
        assert!(
            preparation
                .bind_shared_dataset(Arc::clone(&data))
                .unwrap()
                .validate()
                .unwrap()
                .conforms,
            "legacy law remains distinct"
        );
    }
}

#[test]
fn quoted_flag_obeys_the_selected_dated_law() {
    let data = turtle::data(PREFIXES, r#"ex:n ex:p "a.b", "axb" ."#);
    let preparation = PreparedShapes::new(Arc::new(turtle::loads(
        PREFIXES,
        r#"ex:S a sh:NodeShape; sh:targetNode ex:n;
            sh:property [ sh:path ex:p; sh:pattern "a.b"; sh:flags "q" ] ."#,
    )));
    for (profile, findings) in [
        (Profile::Xpath31, 1),
        (Profile::Xpath20, 2),
        (Profile::Xpath31, 1),
    ] {
        let bound = preparation
            .clone()
            .with_xpath_regex(profile, Limits::new())
            .bind_shared_dataset(Arc::clone(&data))
            .unwrap();
        assert_eq!(bound.validate().unwrap().results.len(), findings);
    }
}

#[test]
fn sparql_function_children_inherit_native_patterns_and_refusals() {
    let data = turtle::data(PREFIXES, "");
    let preparation = PreparedShapes::new(Arc::new(turtle::loads(
        PREFIXES,
        r#"
        ex:F a sh:SPARQLFunction; sh:parameter [ sh:path ex:arg ];
            sh:returnType xsd:boolean; sh:select '''
                SELECT (REGEX($arg, "(a)\\\\1") AS ?result) WHERE {}
            ''' .
        ex:S a sh:NodeShape; sh:targetNode ex:n; sh:sparql [ sh:select '''
            SELECT $this WHERE { FILTER(<http://example.org/F>("aa")) }
        ''' ] .
    "#,
    )));
    assert_eq!(
        preparation
            .clone()
            .with_xpath_regex(Profile::Xpath31, Limits::new())
            .bind_shared_dataset(Arc::clone(&data))
            .unwrap()
            .validate()
            .unwrap()
            .results
            .len(),
        1
    );
    let low = preparation.clone().with_xpath_regex(
        Profile::Xpath31,
        Limits::new().with(Resource::MatchSteps, 0),
    );
    assert!(
        matches!(low.bind_shared_dataset(Arc::clone(&data)).unwrap().validate(), Err(XPathValidationError::Query(diagnostic)) if diagnostic.code == Resource::MatchSteps.code())
    );
    assert!(
        preparation
            .bind_shared_dataset(data)
            .unwrap()
            .validate()
            .unwrap()
            .conforms
    );
}

#[test]
fn rules_entailment_constructs_under_the_selected_law_and_aborts_on_refusal() {
    let data = turtle::data(PREFIXES, r#"ex:n ex:p "a" ."#);
    let preparation = PreparedShapes::new(Arc::new(turtle::loads(
        PREFIXES,
        r#"ex:shapes sh:entailment sh:RulesEntailment .
        ex:S a sh:NodeShape; sh:targetNode ex:n; sh:rule ex:R;
            sh:property [ sh:path ex:inferred; sh:hasValue ex:hit ] .
        ex:R a sh:SPARQLRule; sh:construct '''
            CONSTRUCT { $this <http://example.org/inferred> <http://example.org/hit> }
            WHERE { $this <http://example.org/p> ?value . FILTER(REGEX(?value, "(?:a)")) }
        ''' ."#,
    )));
    for (profile, conforms) in [
        (Profile::Xpath31, true),
        (Profile::Xpath20, false),
        (Profile::Xpath31, true),
    ] {
        let bound = preparation
            .clone()
            .with_xpath_regex(profile, Limits::new())
            .bind_shared_dataset(Arc::clone(&data))
            .unwrap();
        assert_eq!(
            bound.validate().unwrap().conforms,
            conforms,
            "{}",
            profile.name()
        );
    }
    let low = preparation.clone().with_xpath_regex(
        Profile::Xpath31,
        Limits::new().with(Resource::MatchSteps, 0),
    );
    assert!(matches!(
        low.bind_shared_dataset(Arc::clone(&data)),
        Err(XPathValidationError::Query(diagnostic))
            if diagnostic.code == Resource::MatchSteps.code()
    ));
    let selected = preparation.with_xpath_regex(Profile::Xpath31, Limits::new());
    assert!(
        selected
            .bind_shared_dataset(Arc::clone(&data))
            .unwrap()
            .validate()
            .unwrap()
            .conforms
    );
    let zero = QueryGovernors::UNBOUNDED.with_fuel(0);
    assert!(matches!(
        selected
            .validate_dataset_with_governors(&data, &zero)
            .unwrap(),
        GovernedValidation::BudgetExhausted { .. }
    ));
}

#[test]
fn global_rules_preserve_native_laws_and_reached_operational_refusals() {
    let preparation = PreparedShapes::new(Arc::new(turtle::loads(
        PREFIXES,
        r#"ex:shapes sh:entailment sh:RulesEntailment .
        ex:S a sh:NodeShape; sh:targetNode ex:n;
            sh:property [ sh:path ex:inferred; sh:hasValue ex:hit ] .
        ex:R a sh:SPARQLRule; sh:construct '''
            CONSTRUCT { ?subject <http://example.org/inferred> <http://example.org/hit> }
            WHERE { ?subject <http://example.org/p> ?value . FILTER(REGEX("a", "(?:a)")) }
        ''' ."#,
    )));
    assert_eq!(preparation.shapes().rules.global_rules.len(), 1);
    for (data_source, conforms) in [("", false), (r#"ex:n ex:p "a" ."#, true)] {
        let data = turtle::data(PREFIXES, data_source);
        let selected = preparation
            .clone()
            .with_xpath_regex(Profile::Xpath31, Limits::new());
        assert_eq!(
            selected
                .bind_shared_dataset(Arc::clone(&data))
                .unwrap()
                .validate()
                .unwrap()
                .conforms,
            conforms
        );
        assert!(
            !preparation
                .clone()
                .with_xpath_regex(Profile::Xpath20, Limits::new())
                .bind_shared_dataset(Arc::clone(&data))
                .unwrap()
                .validate()
                .unwrap()
                .conforms
        );
        for resource in [
            Resource::ProgramNodes,
            Resource::CompileSlots,
            Resource::MatchSteps,
        ] {
            let low = selected
                .clone()
                .with_xpath_regex(Profile::Xpath31, Limits::new().with(resource, 0));
            let bound = low.bind_shared_dataset(Arc::clone(&data));
            if data_source.is_empty() {
                // An empty join never invokes the expression. Selecting a native
                // law does not eagerly execute a dormant pattern.
                assert!(!bound.unwrap().validate().unwrap().conforms);
            } else {
                assert!(matches!(
                    bound,
                    Err(XPathValidationError::Query(diagnostic))
                        if diagnostic.code == resource.code()
                ));
            }
        }
        if !data_source.is_empty() {
            assert!(matches!(
                selected
                    .validate_dataset_with_governors(&data, &QueryGovernors::UNBOUNDED.with_fuel(0))
                    .unwrap(),
                GovernedValidation::BudgetExhausted { .. }
            ));
        }
        assert_eq!(
            selected
                .bind_shared_dataset(Arc::clone(&data))
                .unwrap()
                .validate()
                .unwrap()
                .conforms,
            conforms
        );
    }
}

#[test]
fn selected_execution_preserves_prepared_product_bytes_and_restore_provenance() {
    let data = turtle::data(PREFIXES, r#"ex:n ex:p "aa" ."#);
    let preparation = PreparedShapes::new(Arc::new(turtle::loads(
        PREFIXES,
        r#"
        ex:S a sh:NodeShape; sh:targetNode ex:n;
            sh:property [ sh:path ex:p; sh:pattern "(a)\\1" ] .
    "#,
    )));
    let bytes = preparation.to_product(&ShapesProfile::CORE).unwrap();
    let admitted = ShapesProduct::open(&bytes)
        .unwrap()
        .admit(&ShapesProfile::CORE, &HostBindings::empty())
        .unwrap();
    let rebuilt = ShapesProduct::open(&bytes)
        .unwrap()
        .rebuild(&ShapesProfile::CORE, &HostBindings::empty())
        .unwrap();
    for prepared in [preparation, admitted, rebuilt] {
        let original = prepared.to_product(&ShapesProfile::CORE).unwrap();
        let provenance = prepared.provenance().clone();
        let selected = prepared.with_xpath_regex(Profile::Xpath31, Limits::new());
        selected.admit_declared_patterns().unwrap();
        let bound = selected.bind_shared_dataset(Arc::clone(&data)).unwrap();
        assert_eq!(bound.provenance(), &provenance);
        assert!(bound.validate().unwrap().conforms);
        assert_eq!(
            selected
                .preparation()
                .to_product(&ShapesProfile::CORE)
                .unwrap(),
            original
        );
    }
}

/// A copy-on-write change inserting `ex:n ex:p value` over a base holding only
/// `ex:other ex:q "x"`, and `shapes` prepared under no selection.
fn change_fixture(
    shapes: &str,
    value: &str,
) -> (PreparedShapes, Arc<purrdf_rdf::ir::DeltaDatasetView>) {
    use purrdf_rdf::{DatasetMut, QuadValues, TermValue};
    let base = turtle::data(PREFIXES, r#"ex:other ex:q "x" ."#);
    let mut mutation = MutableDataset::new(base);
    assert!(
        mutation
            .insert(QuadValues {
                s: TermValue::iri("http://example.org/n"),
                p: TermValue::iri("http://example.org/p"),
                o: TermValue::simple_literal(value),
                g: None,
            })
            .unwrap()
    );
    let preparation = PreparedShapes::new(Arc::new(turtle::loads(PREFIXES, shapes)));
    (preparation, Arc::new(mutation.snapshot_view().unwrap()))
}

/// Run the selected change door, ungoverned and governed with zero fuel, and
/// require the two to agree on scope and report.
fn selected_change(
    preparation: &PreparedShapes,
    delta: &Arc<purrdf_rdf::ir::DeltaDatasetView>,
    profile: Profile,
    limits: Limits,
) -> Result<(purrdf_shapes::engine::ChangeScope, bool), XPathValidationError> {
    let bound = preparation
        .clone()
        .with_xpath_regex(profile, limits)
        .bind_delta_with_shapes_graph(Arc::clone(delta), None, ViewLimits::default())?;
    assert_eq!(bound.selection(), (profile, limits));
    let change = bound.validate_change(delta)?;
    Ok((change.scope, change.report.conforms))
}

const CORE_PATTERN_SHAPES: &str = r#"ex:S a sh:NodeShape; sh:targetSubjectsOf ex:p;
    sh:property [ sh:path ex:p; sh:pattern "PATTERN" ] ."#;

#[test]
fn change_path_validates_core_patterns_under_the_selected_law() {
    use purrdf_shapes::engine::ChangeScope;
    let bounded = ChangeScope::Bounded { focus_nodes: 1 };
    for (pattern, value, xpath20, xpath31) in [
        // Non-capturing groups are XPath 3.1 only: under 2.0 the malformed
        // pattern is the ordinary sh:pattern finding.
        ("(?:a)b", "ab", false, true),
        // Both dated laws define backreferences.
        ("^(a)\\\\1$", "aa", true, true),
    ] {
        let shapes = CORE_PATTERN_SHAPES.replace("PATTERN", pattern);
        let (preparation, delta) = change_fixture(&shapes, value);
        for (profile, conforms) in [(Profile::Xpath20, xpath20), (Profile::Xpath31, xpath31)] {
            assert_eq!(
                selected_change(&preparation, &delta, profile, Limits::new()).unwrap(),
                (bounded, conforms),
                "{pattern} under {}",
                profile.name()
            );
        }
    }
    // The unselected change path keeps the compatibility law, which refuses a
    // backreference both dated laws match.
    let (preparation, delta) =
        change_fixture(&CORE_PATTERN_SHAPES.replace("PATTERN", "^(a)\\\\1$"), "aa");
    let legacy = preparation
        .bind_delta_with_shapes_graph(Arc::clone(&delta), None, ViewLimits::default())
        .unwrap();
    let change = purrdf_shapes::engine::validate_change(&legacy, &delta).unwrap();
    assert_eq!(change.scope, bounded);
    assert!(!change.report.conforms);
}

#[test]
fn change_path_fallback_runs_sparql_regex_under_the_selected_law() {
    // A SPARQL constraint has no bounded footprint, so the change loop falls back
    // to a full validation; its REGEX still runs under the binding's law.
    let shapes = r#"ex:S a sh:NodeShape; sh:targetSubjectsOf ex:p;
        sh:sparql [ sh:select """SELECT $this WHERE {
            $this <http://example.org/p> ?o FILTER(REGEX(?o, "(?:a)b")) }""" ] ."#;
    let (preparation, delta) = change_fixture(shapes, "ab");
    for (profile, conforms) in [(Profile::Xpath20, true), (Profile::Xpath31, false)] {
        let (scope, verdict) =
            selected_change(&preparation, &delta, profile, Limits::new()).unwrap();
        assert!(
            !scope.is_bounded(),
            "a SPARQL constraint has no bounded footprint"
        );
        assert_eq!(verdict, conforms, "{}", profile.name());
    }
}

#[test]
fn change_path_resource_refusal_raises_and_the_neighbour_is_admitted() {
    let (preparation, delta) =
        change_fixture(&CORE_PATTERN_SHAPES.replace("PATTERN", "^(a)\\\\1$"), "aa");
    let zero = QueryGovernors::UNBOUNDED.with_fuel(0);
    for profile in [Profile::Xpath20, Profile::Xpath31] {
        let starved = Limits::new().with(Resource::MatchSteps, 0);
        assert!(matches!(
            selected_change(&preparation, &delta, profile, starved),
            Err(XPathValidationError::Pattern(Error::Resource(refusal)))
                if refusal.resource == Resource::MatchSteps
        ));
        let bound = preparation
            .clone()
            .with_xpath_regex(profile, starved)
            .bind_delta_with_shapes_graph(Arc::clone(&delta), None, ViewLimits::default())
            .unwrap();
        assert!(matches!(
            bound.validate_change_with_governors(&delta, &zero),
            Err(XPathValidationError::Pattern(Error::Resource(refusal)))
                if refusal.resource == Resource::MatchSteps
        ));

        // The valid neighbour: the same change under the production bounds is
        // admitted, ungoverned and governed alike, and the bounded expansion spends
        // no SPARQL fuel.
        assert!(
            selected_change(&preparation, &delta, profile, Limits::new())
                .unwrap()
                .1
        );
        let bound = preparation
            .clone()
            .with_xpath_regex(profile, Limits::new())
            .bind_delta_with_shapes_graph(Arc::clone(&delta), None, ViewLimits::default())
            .unwrap();
        let governed = bound.validate_change_with_governors(&delta, &zero).unwrap();
        assert!(governed.scope.is_bounded());
        let GovernedValidation::Complete { report, .. } = governed.outcome else {
            panic!("native Core pattern work spends no SPARQL fuel")
        };
        assert!(report.conforms && report.results.is_empty());
    }
}

// ── Rules and node expressions under a selected law ──────────────────────────────

/// Non-capturing groups are XPath 3.1 only; under 2.0 the pattern is ill-formed.
const NONCAPTURING: &str = "(?:a)b";
/// Both dated laws define backreferences; the compatibility law refuses them.
const BACKREFERENCE: &str = r"^(a)\1$";

/// `text` as a quoted Turtle (or SPARQL) string literal.
fn quoted(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

/// The pattern one byte over the production source bound, and the one exactly at it.
fn bound_patterns() -> [String; 2] {
    let bound = usize::try_from(Limits::new().limit(Resource::PatternBytes)).unwrap();
    ["a".repeat(bound + 1), "a".repeat(bound)]
}

/// The objects a rules run inferred, as lexical forms of `ex:hit` values.
fn hits(inference: &purrdf_shapes::Inference) -> Vec<String> {
    let mut values: Vec<String> = inference
        .inferred()
        .iter()
        .map(|[_, _, object]| object.to_string())
        .collect();
    values.sort();
    values
}

/// A shapes graph whose rule infers `?this ex:hit ?v` for each `ex:p` value matching
/// `pattern`, through a SPARQL rule's `REGEX` (`sparql`) or a triple rule's condition
/// shape's `sh:pattern`.
fn rule_shapes(pattern: &str, sparql: bool) -> purrdf_shapes::shapes::Shapes {
    let rule = if sparql {
        let construct = format!(
            "CONSTRUCT {{ $this <http://example.org/hit> ?v }} \
             WHERE {{ $this <http://example.org/p> ?v FILTER(REGEX(?v, {})) }}",
            quoted(pattern)
        );
        format!("[ a sh:SPARQLRule ; sh:construct {} ]", quoted(&construct))
    } else {
        format!(
            "[ a sh:TripleRule ; sh:subject sh:this ; sh:predicate ex:hit ; \
               sh:object [ sh:path ex:p ] ; \
               sh:condition [ sh:property [ sh:path ex:p ; sh:pattern {} ] ] ]",
            quoted(pattern)
        )
    };
    turtle::loads(
        PREFIXES,
        &format!("ex:S a sh:NodeShape ; sh:targetSubjectsOf ex:p ; sh:rule {rule} ."),
    )
}

/// A SPARQL 1.2 RL rule set inferring `?s :hit ?v` for each `:p` value matching
/// `pattern`.
fn rule_set(pattern: &str) -> purrdf_shapes::srl::RuleSetDocument {
    purrdf_shapes::srl::parse_and_check(
        &format!(
            "PREFIX : <http://example.org/>\n\
             RULE {{ ?s :hit ?v }} WHERE {{ ?s :p ?v FILTER(REGEX(?v, {})) }}",
            quoted(pattern)
        ),
        None,
    )
    .expect("the rule set checks")
}

fn selected_rules(
    source: purrdf_shapes::RuleSource<'_>,
    data: &purrdf_rdf::RdfDataset,
    profile: Profile,
) -> Result<purrdf_shapes::Inference, XPathValidationError> {
    purrdf_shapes::xpath::run_rules(
        source,
        data,
        &purrdf_shapes::RuleLimits::default(),
        purrdf_shapes::LimitKnobs::default(),
        profile,
        Limits::new(),
    )
}

fn compatibility_rules(
    source: purrdf_shapes::RuleSource<'_>,
    data: &purrdf_rdf::RdfDataset,
) -> purrdf_shapes::Inference {
    purrdf_shapes::run_rules(
        source,
        data,
        &purrdf_shapes::RuleLimits::default(),
        purrdf_shapes::LimitKnobs::default(),
    )
    .expect("the compatibility run succeeds")
}

#[test]
fn every_rule_language_infers_under_each_selected_law() {
    let data = turtle::data(PREFIXES, r#"ex:n ex:p "ab" . ex:m ex:p "aa" ."#);
    let ab = vec![quoted("ab")];
    let aa = vec![quoted("aa")];
    for (pattern, xpath20, xpath31) in [
        (NONCAPTURING, Vec::new(), ab),
        (BACKREFERENCE, aa.clone(), aa),
    ] {
        let sparql = rule_shapes(pattern, true);
        let triple = rule_shapes(pattern, false);
        let srl = rule_set(pattern);
        for (profile, expected) in [(Profile::Xpath20, &xpath20), (Profile::Xpath31, &xpath31)] {
            for (route, source) in [
                ("SPARQL rule", purrdf_shapes::RuleSource::Shapes(&sparql)),
                ("triple rule", purrdf_shapes::RuleSource::Shapes(&triple)),
                ("SPARQL 1.2 RL", purrdf_shapes::RuleSource::Srl(&srl)),
            ] {
                let inference = selected_rules(source, &data, profile)
                    .unwrap_or_else(|error| panic!("{route} {pattern}: {error}"));
                assert_eq!(
                    &hits(&inference),
                    expected,
                    "{route} {pattern} under {}",
                    profile.name()
                );
            }
        }
    }
    // Unselected, every route keeps the compatibility law, which refuses the
    // backreference both dated laws match.
    let sparql = rule_shapes(BACKREFERENCE, true);
    let triple = rule_shapes(BACKREFERENCE, false);
    let srl = rule_set(BACKREFERENCE);
    for source in [
        purrdf_shapes::RuleSource::Shapes(&sparql),
        purrdf_shapes::RuleSource::Shapes(&triple),
        purrdf_shapes::RuleSource::Srl(&srl),
    ] {
        assert_eq!(
            hits(&compatibility_rules(source, &data)),
            Vec::<String>::new()
        );
    }
}

#[test]
fn a_rules_resource_refusal_aborts_and_the_neighbour_runs() {
    let data = turtle::data(PREFIXES, r#"ex:n ex:p "ab" ."#);
    let [over, at] = bound_patterns();
    for profile in Profile::ALL {
        // A SPARQL rule's and a rule set's REGEX are refused by the query engine.
        let sparql = rule_shapes(&over, true);
        let srl = rule_set(&over);
        for source in [
            purrdf_shapes::RuleSource::Shapes(&sparql),
            purrdf_shapes::RuleSource::Srl(&srl),
        ] {
            assert!(matches!(
                selected_rules(source, &data, profile),
                Err(XPathValidationError::Query(diagnostic))
                    if diagnostic.code == Resource::PatternBytes.code()
            ));
        }
        // A condition shape's sh:pattern is refused by the native compiler.
        let triple = rule_shapes(&over, false);
        assert!(matches!(
            selected_rules(purrdf_shapes::RuleSource::Shapes(&triple), &data, profile),
            Err(XPathValidationError::Pattern(Error::Resource(refusal)))
                if refusal.resource == Resource::PatternBytes
        ));
        // The pattern exactly at the bound is admitted, and matches nothing.
        let sparql = rule_shapes(&at, true);
        let triple = rule_shapes(&at, false);
        let srl = rule_set(&at);
        for source in [
            purrdf_shapes::RuleSource::Shapes(&sparql),
            purrdf_shapes::RuleSource::Shapes(&triple),
            purrdf_shapes::RuleSource::Srl(&srl),
        ] {
            assert_eq!(
                hits(&selected_rules(source, &data, profile).unwrap()),
                Vec::<String>::new()
            );
        }
    }
}

/// The node expression `_:e` of `body`, evaluated at the focus node `focus` under the
/// selected law, or the compatibility law for `None`.
fn node_expression(
    body: &str,
    focus: &str,
    profile: Option<Profile>,
) -> Result<Vec<String>, XPathValidationError> {
    let document = purrdf_shapes::text_ingest::parse_turtle_document(
        &format!(
            "{PREFIXES}@prefix shnex: <http://www.w3.org/ns/shacl-node-expr#> .\n\
             @prefix sparql: <http://www.w3.org/ns/sparql#> .\n{body}"
        ),
        None,
    )
    .expect("the shapes document parses");
    let data = turtle::data(PREFIXES, "");
    let focus = purrdf_shapes::free_expression::parse_term(focus).unwrap();
    let request = purrdf_shapes::free_expression::FreeExpression {
        shapes: &document.dataset,
        prefixes: &document.prefixes,
        root: &purrdf_shapes::term::Term::blank("e"),
        data: data.as_ref(),
        focus: &focus,
        scope: &[],
        imports: &purrdf_shapes::ShapesImports::new(),
    };
    let evaluated = match profile {
        None => {
            purrdf_shapes::free_expression::evaluate(&request).map_err(XPathValidationError::from)
        }
        Some(profile) => {
            purrdf_shapes::xpath::evaluate_free_expression(&request, profile, Limits::new())
        }
    }?;
    Ok(evaluated.outputs.iter().map(ToString::to_string).collect())
}

/// A filter shape's `sh:pattern` over the focus node.
fn filter_expression(pattern: &str) -> String {
    format!(
        "_:e shnex:filterShape [ sh:pattern {} ] ; shnex:nodes [ shnex:var \"focusNode\" ] .",
        quoted(pattern)
    )
}

/// The `sparql:regex` function over the focus node.
fn regex_expression(pattern: &str) -> String {
    format!(
        "_:e sparql:regex ( [ shnex:var \"focusNode\" ] {} ) .",
        quoted(pattern)
    )
}

#[test]
fn node_expressions_evaluate_under_each_selected_law() {
    let ab = quoted("ab");
    let aa = quoted("aa");
    let truth = "\"true\"^^<http://www.w3.org/2001/XMLSchema#boolean>".to_owned();
    for (pattern, focus, xpath20, xpath31) in [
        (NONCAPTURING, &ab, false, true),
        (BACKREFERENCE, &aa, true, true),
    ] {
        for (profile, matches) in [(Profile::Xpath20, xpath20), (Profile::Xpath31, xpath31)] {
            let filtered =
                node_expression(&filter_expression(pattern), focus, Some(profile)).unwrap();
            assert_eq!(
                filtered,
                if matches {
                    vec![focus.clone()]
                } else {
                    Vec::new()
                },
                "filter shape {pattern} under {}",
                profile.name()
            );
            let called = node_expression(&regex_expression(pattern), focus, Some(profile)).unwrap();
            assert_eq!(
                called == vec![truth.clone()],
                matches,
                "sparql:regex {pattern} under {}: {called:?}",
                profile.name()
            );
        }
    }
    // Unselected, the compatibility law refuses the backreference both laws match.
    assert_eq!(
        node_expression(&filter_expression(BACKREFERENCE), &aa, None).unwrap(),
        Vec::<String>::new()
    );
    assert_ne!(
        node_expression(&regex_expression(BACKREFERENCE), &aa, None).unwrap(),
        vec![truth]
    );
}

#[test]
fn a_node_expression_resource_refusal_aborts_and_the_neighbour_evaluates() {
    let [over, at] = bound_patterns();
    let focus = quoted("ab");
    for profile in Profile::ALL {
        assert!(matches!(
            node_expression(&filter_expression(&over), &focus, Some(profile)),
            Err(XPathValidationError::Pattern(Error::Resource(refusal)))
                if refusal.resource == Resource::PatternBytes
        ));
        assert!(matches!(
            node_expression(&regex_expression(&over), &focus, Some(profile)),
            Err(XPathValidationError::Query(diagnostic))
                if diagnostic.code == Resource::PatternBytes.code()
        ));
        assert_eq!(
            node_expression(&filter_expression(&at), &focus, Some(profile)).unwrap(),
            Vec::<String>::new()
        );
        assert_eq!(
            node_expression(&regex_expression(&at), &focus, Some(profile)).unwrap(),
            vec!["\"false\"^^<http://www.w3.org/2001/XMLSchema#boolean>".to_owned()]
        );
    }
}

#[test]
fn text_graphs_validate_under_each_selected_law() {
    let shapes = |pattern: &str| {
        format!(
            "{PREFIXES}ex:S a sh:NodeShape ; sh:targetSubjectsOf ex:p ; \
             sh:property [ sh:path ex:p ; sh:pattern {} ] .",
            quoted(pattern)
        )
    };
    let validate = |pattern: &str, value: &str, profile: Profile| {
        purrdf_shapes::xpath::validate_graphs_with_shapes_graph(
            &format!(
                "<http://example.org/n> <http://example.org/p> {} .\n",
                quoted(value)
            ),
            &shapes(pattern),
            None,
            None,
            &ValidationOptions::default(),
            &purrdf_shapes::ShapesImports::new(),
            profile,
            Limits::new(),
        )
    };
    for (pattern, value, xpath20, xpath31) in [
        (NONCAPTURING, "ab", false, true),
        (BACKREFERENCE, "aa", true, true),
    ] {
        for (profile, conforms) in [(Profile::Xpath20, xpath20), (Profile::Xpath31, xpath31)] {
            assert_eq!(
                validate(pattern, value, profile).unwrap().conforms,
                conforms,
                "{pattern} under {}",
                profile.name()
            );
        }
    }
    let [over, at] = bound_patterns();
    for profile in Profile::ALL {
        assert!(matches!(
            validate(&over, "a", profile),
            Err(XPathValidationError::Pattern(Error::Resource(refusal)))
                if refusal.resource == Resource::PatternBytes
        ));
        assert!(!validate(&at, "a", profile).unwrap().conforms);
    }
    // A parse refusal is the compatibility entry's own error.
    let refused = purrdf_shapes::xpath::validate_graphs_with_shapes_graph(
        "not n-triples",
        &shapes("a"),
        None,
        None,
        &ValidationOptions::default(),
        &purrdf_shapes::ShapesImports::new(),
        Profile::Xpath31,
        Limits::new(),
    );
    assert!(matches!(refused, Err(XPathValidationError::Shapes(_))));
}

/// The property shapes that fail, in order: one `sh:pattern` shape `ex:P{i}` per case.
fn failing_pattern_shapes(report: &purrdf_shapes::report::ValidationReport) -> Vec<String> {
    let mut failing: Vec<String> = report
        .results
        .iter()
        .map(|result| format!("{:?}", result.source_shape))
        .collect();
    failing.sort();
    failing.dedup();
    failing
}

#[test]
fn adversary_pattern_shapes_validate_at_the_production_defaults_like_the_compatibility_engine() {
    // Each shape was refused by the native matcher at a fraction of these sizes.
    let text = purrdf_testkit::text::word_prose(1 << 20);
    let pairs = "ab".repeat(1 << 19);
    let forty = "a".repeat(40);
    let values = [
        ("prose", text.clone()),
        ("zzz", format!("{text} zzz")),
        ("bang", format!("{text}!")),
        ("spaced", format!("{text} ")),
        ("pairs", pairs.clone()),
        ("pairs_c", format!("{pairs}c")),
        ("pairs_a", format!("{pairs}a")),
        ("forty_b", format!("{forty}b")),
        ("forty_c", format!("{forty}c")),
    ];
    let mut body = String::new();
    for (subject, value) in &values {
        writeln!(body, "ex:{subject} ex:p \"{value}\" .").unwrap();
    }
    let data = turtle::data(PREFIXES, &body);
    let cases = [
        ("node.*graph.*zzz", "prose", false),
        ("node.*graph.*zzz", "zzz", true),
        ("alpha.*zzz", "prose", false),
        ("alpha.*zzz", "zzz", true),
        ("^([a-z]+ ?)+$", "prose", true),
        ("^([a-z]+ ?)+$", "bang", false),
        (r"^(\w+\s)*\w+$", "prose", true),
        (r"^(\w+\s)*\w+$", "spaced", false),
        ("^(a|b)*$", "pairs", true),
        ("^(a|b)*$", "pairs_c", false),
        ("^(ab)*$", "pairs", true),
        ("^(ab)*$", "pairs_a", false),
        ("^(?:ab)*$", "pairs", true),
        ("^(?:ab)*$", "pairs_a", false),
        ("^(a|aa)*$|^(a*)*b$", "forty_b", true),
        ("^(a|aa)*$|^(a*)*b$", "forty_c", false),
    ];
    for profile in [Profile::Xpath20, Profile::Xpath31] {
        // XPath 2.0 has no non-capturing group, so its graph omits those shapes.
        let selected: Vec<_> = cases
            .iter()
            .enumerate()
            .filter(|(_, (pattern, ..))| profile == Profile::Xpath31 || !pattern.contains("(?:"))
            .collect();
        let mut shapes = String::new();
        for (index, (pattern, subject, _)) in &selected {
            writeln!(
                shapes,
                "ex:S{index} a sh:NodeShape; sh:targetNode ex:{subject}; sh:property ex:P{index} .
                    ex:P{index} sh:path ex:p; sh:pattern \"{}\" .",
                pattern.replace('\\', "\\\\")
            )
            .unwrap();
        }
        let shapes = Arc::new(turtle::loads(PREFIXES, &shapes));
        let expected: Vec<String> = {
            let mut failing: Vec<String> = selected
                .iter()
                .filter(|(_, (_, _, conforms))| !conforms)
                .map(|(index, _)| format!("{:?}", ex(&format!("P{index}"))))
                .collect();
            failing.sort();
            failing
        };
        let compatibility = PreparedShapes::new(Arc::clone(&shapes))
            .bind_shared_dataset(Arc::clone(&data))
            .unwrap()
            .validate()
            .unwrap();
        assert_eq!(
            failing_pattern_shapes(&compatibility),
            expected,
            "compatibility"
        );
        let native = validate_dataset(&data, Arc::clone(&shapes), profile, Limits::new())
            .unwrap_or_else(|error| panic!("{profile:?}: {error}"));
        assert_eq!(failing_pattern_shapes(&native), expected, "{profile:?}");
    }
}

#[test]
fn a_backreference_blowup_pattern_shape_still_refuses_beside_a_conforming_neighbour() {
    let forty = "a".repeat(40);
    let data = turtle::data(
        PREFIXES,
        &format!("ex:refused ex:p \"{forty}\" .\nex:neighbour ex:p \"{forty}ca\" .\n"),
    );
    let shape = |subject: &str| {
        Arc::new(turtle::loads(
            PREFIXES,
            &format!(
                r#"ex:S a sh:NodeShape; sh:targetNode ex:{subject};
                    sh:property [ sh:path ex:p; sh:pattern "^(a|aa)*c\\1$" ] ."#
            ),
        ))
    };
    for profile in Profile::ALL {
        assert!(matches!(
            validate_dataset(&data, shape("refused"), profile, Limits::new()),
            Err(XPathValidationError::Pattern(Error::Resource(refusal)))
                if matches!(
                    refusal.resource,
                    Resource::MatchSteps | Resource::MatchStates | Resource::MatchSlots
                )
        ));
        let report = validate_dataset(&data, shape("neighbour"), profile, Limits::new()).unwrap();
        assert!(report.conforms && report.results.is_empty(), "{profile:?}");
    }
}

#[test]
fn counted_pattern_shapes_validate_at_the_production_defaults_like_the_compatibility_engine() {
    // Each shape was refused by the native matcher at these sizes: a counted
    // group repeated from every start kept one thread per distinct count.
    let pairs = "ab".repeat(1 << 19);
    let mixed = "abbaab".repeat((4 << 20) / 6);
    let prose = purrdf_testkit::text::word_prose(8 << 20);
    let values = [
        ("pairs_128k", &pairs[..128 << 10], "c"),
        ("pairs", pairs.as_str(), "c"),
        ("quads", &"abcd".repeat(1 << 18), "e"),
        ("mixed_400k", &mixed[..400 << 10], "c"),
        ("mixed_800k", &mixed[..800 << 10], "c"),
        ("mixed", mixed.as_str(), "c"),
        (
            "prose_4m",
            &prose[..prose[..4 << 20].rfind(' ').unwrap()],
            " zzz",
        ),
        ("prose", prose.as_str(), " zzz"),
    ];
    let mut body = String::from("ex:empty ex:p \"\" .\n");
    for (subject, value, suffix) in &values {
        writeln!(body, "ex:{subject} ex:p \"{value}\" .").unwrap();
        writeln!(body, "ex:{subject}_completed ex:p \"{value}{suffix}\" .").unwrap();
    }
    let data = turtle::data(PREFIXES, &body);
    // The native pattern, the compatibility pattern with the same matches, and
    // the plain subject whose completed neighbour conforms.
    let cases = [
        ("(ab){1,1000}c", "(ab){1,1000}c", "pairs_128k"),
        ("(ab){2,50}c", "(ab){2,50}c", "pairs"),
        ("(ab){1,100}c", "(ab){1,100}c", "pairs"),
        ("(ab|cd){1,20}e", "(ab|cd){1,20}e", "quads"),
        ("((a|b){3}){5,9}c", "((a|b){3}){5,9}c", "mixed_400k"),
        ("((a|b){2}){2,5}c", "((a|b){2}){2,5}c", "mixed_800k"),
        ("(a|b){1,30}c", "(a|b){1,30}c", "mixed"),
        ("(a|b){3,9}c", "(a|b){3,9}c", "mixed"),
        (r"(\w+\s){3,5}zzz", r"(\w+\s){3,5}zzz", "prose_4m"),
        ("node.*graph.*zzz", "node.*graph.*zzz", "prose"),
        ("(ab){1,100000}c", "abc", "pairs_128k"),
    ];
    let graph = |native: bool| {
        let mut shapes = String::new();
        for (index, (pattern, compatible, subject)) in cases.iter().enumerate() {
            let pattern = if native { pattern } else { compatible };
            for (suffix, focus) in [("", *subject), ("c", &format!("{subject}_completed"))] {
                writeln!(
                    shapes,
                    "ex:S{index}{suffix} a sh:NodeShape; sh:targetNode ex:{focus}; \
                     sh:property ex:P{index}{suffix} .
                    ex:P{index}{suffix} sh:path ex:p; sh:pattern \"{}\" .",
                    pattern.replace('\\', "\\\\")
                )
                .unwrap();
            }
        }
        if native {
            writeln!(
                shapes,
                "ex:Empty a sh:NodeShape; sh:targetNode ex:empty, ex:pairs; \
                 sh:property ex:PEmpty .
                ex:PEmpty sh:path ex:p; sh:pattern \"^(a?){{18446744073709551616}}$\" ."
            )
            .unwrap();
        }
        Arc::new(turtle::loads(PREFIXES, &shapes))
    };
    // Every plain subject fails its pattern, every completed one conforms, and
    // the nullable whole fails only the non-empty subject.
    let mut expected: Vec<String> = (0..cases.len())
        .map(|index| format!("{:?}", ex(&format!("P{index}"))))
        .collect();
    expected.sort();
    let compatibility = PreparedShapes::new(graph(false))
        .bind_shared_dataset(Arc::clone(&data))
        .unwrap()
        .validate()
        .unwrap();
    assert_eq!(
        failing_pattern_shapes(&compatibility),
        expected,
        "compatibility"
    );
    expected.push(format!("{:?}", ex("PEmpty")));
    expected.sort();
    for profile in Profile::ALL {
        let native = validate_dataset(&data, graph(true), profile, Limits::new())
            .unwrap_or_else(|error| panic!("{profile:?}: {error}"));
        assert_eq!(failing_pattern_shapes(&native), expected, "{profile:?}");
        let empty = native
            .results
            .iter()
            .filter(|result| format!("{:?}", result.source_shape) == format!("{:?}", ex("PEmpty")))
            .map(|result| format!("{:?}", result.focus_node))
            .collect::<Vec<_>>();
        assert_eq!(empty, [format!("{:?}", ex("pairs"))], "{profile:?}");
    }
}
