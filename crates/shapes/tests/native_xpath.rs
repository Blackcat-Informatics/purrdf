// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Dated native pattern selection through SHACL's actual preparation and binding doors.

#![cfg(not(target_arch = "wasm32"))]

#[path = "support/terms.rs"]
mod terms;
#[path = "support/turtle.rs"]
mod turtle;

use std::error::Error as _;
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
