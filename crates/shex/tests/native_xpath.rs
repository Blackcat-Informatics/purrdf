// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Selected XPath laws and operational refusal through the public ShEx engine.
#![cfg(not(target_arch = "wasm32"))]

use purrdf_core::term_fixture::empty_dataset;
use purrdf_core::xsd_regex::xpath::{Error, Limits, Profile, Resource};
use purrdf_core::{RdfDatasetBuilder, RdfLiteral, TermValue};
use purrdf_shex::{
    Schema, ShapeSelector, ValidationOptions, XPathValidationError, XPathValidator, parse_shexj,
    validate, validate_shape_map_with_xpath, validate_with_xpath,
};

fn node(pattern: &str, flags: &str) -> String {
    use purrdf_lex::json::{Value, write_compact};
    format!(
        r#"{{"type":"NodeConstraint","pattern":{},"flags":{}}}"#,
        write_compact(&Value::from(pattern)),
        write_compact(&Value::from(flags))
    )
}

fn schema(expression: &str) -> Schema {
    parse_shexj(
        &format!(r#"{{"type":"Schema","start":{expression}}}"#),
        None,
    )
    .unwrap()
}

fn labeled(id: &str, expression: &str) -> String {
    use purrdf_lex::json::{Value, read, write_compact};
    let mut value = read(expression).unwrap();
    let Value::Object(object) = &mut value else {
        panic!("the fixture expression is an object")
    };
    object.insert("id", Value::from(id));
    write_compact(&value)
}

fn association(text: &str) -> (TermValue, ShapeSelector) {
    (TermValue::simple_literal(text), ShapeSelector::Start)
}

fn refused(error: &Error, resource: Resource) {
    assert!(error.is_operational());
    assert!(matches!(error, Error::Resource(cause) if cause.resource == resource));
}

#[test]
fn reused_schema_selects_the_actual_dated_law_and_preserves_compatibility() {
    let data = empty_dataset();
    let options = ValidationOptions::default();
    for (pattern, flags, expected20, expected31, legacy) in [
        (r"^(a)\1$", "", true, true, false),
        ("^(?:a)$", "", false, true, true),
        ("a.b", "q", false, true, true),
    ] {
        let parsed = schema(&node(pattern, flags));
        let map = [association(if flags == "q" {
            "a.b"
        } else if pattern.contains('1') {
            "aa"
        } else {
            "a"
        })];
        assert_eq!(validate(&parsed, &data, &map).all_conformant(), legacy);
        let mut validator = XPathValidator::new(&parsed);
        for (profile, expected) in [
            (Profile::Xpath31, expected31),
            (Profile::Xpath20, expected20),
            (Profile::Xpath31, expected31),
        ] {
            let result = validator
                .validate(&data, &map, &options, profile, Limits::new())
                .unwrap();
            assert_eq!(
                result.all_conformant(),
                expected,
                "{pattern} / {flags} / {profile:?}"
            );
        }
    }
}

#[test]
fn compilation_and_matching_refusals_remain_typed_with_valid_neighbors() {
    let data = empty_dataset();
    let options = ValidationOptions::default();
    let parsed = schema(&node("^(a|b)+$", ""));
    let map = [association("aab")];
    for resource in [
        Resource::PatternBytes,
        Resource::CompileSteps,
        Resource::ProgramNodes,
        Resource::CompileSlots,
        Resource::MatchSteps,
        Resource::MatchStates,
        Resource::MatchSlots,
    ] {
        let error = validate_with_xpath(
            &parsed,
            &data,
            &map,
            &options,
            Profile::Xpath31,
            Limits::new().with(resource, 0),
        )
        .unwrap_err();
        refused(&error, resource);
        assert!(
            validate_with_xpath(
                &parsed,
                &data,
                &map,
                &options,
                Profile::Xpath31,
                Limits::new()
            )
            .unwrap()
            .all_conformant()
        );
    }
    let invalid = schema(&node("é(", ""));
    let error = validate_with_xpath(
        &invalid,
        &data,
        &map,
        &options,
        Profile::Xpath31,
        Limits::new().with(Resource::PatternBytes, 2),
    )
    .unwrap_err();
    assert!(
        matches!(error, Error::Resource(cause) if cause.resource == Resource::PatternBytes && cause.required == 3 && cause.limit == 2)
    );
}

#[test]
fn current_bounds_are_admitted_before_a_reused_program_and_fuel_is_fresh() {
    let data = empty_dataset();
    let options = ValidationOptions::default();
    let parsed = schema(&node("^(?:a|b)$", ""));
    let map = [association("a")];
    let mut validator = XPathValidator::new(&parsed);
    assert!(
        validator
            .validate(&data, &map, &options, Profile::Xpath31, Limits::new())
            .unwrap()
            .all_conformant()
    );
    for resource in [
        Resource::PatternBytes,
        Resource::ProgramNodes,
        Resource::CompileSlots,
        Resource::MatchSteps,
        Resource::MatchStates,
        Resource::MatchSlots,
    ] {
        let error = validator
            .validate(
                &data,
                &map,
                &options,
                Profile::Xpath31,
                Limits::new().with(resource, 0),
            )
            .unwrap_err();
        refused(&error, resource);
        assert!(
            validator
                .validate(&data, &map, &options, Profile::Xpath31, Limits::new())
                .unwrap()
                .all_conformant()
        );
    }
    assert!(
        !validator
            .validate(&data, &map, &options, Profile::Xpath20, Limits::new())
            .unwrap()
            .all_conformant()
    );
    // A failed attempt under another grammar cannot poison the retained program.
    assert!(
        validator
            .validate(
                &data,
                &map,
                &options,
                Profile::Xpath31,
                Limits::new().with(Resource::CompileSteps, 0)
            )
            .unwrap()
            .all_conformant()
    );
}

#[test]
fn syntax_findings_are_not_reused_as_operational_verdicts() {
    let data = empty_dataset();
    let options = ValidationOptions::default();
    let parsed = schema(&node("[", ""));
    let map = [association("a")];
    let mut validator = XPathValidator::new(&parsed);
    assert!(
        !validator
            .validate(&data, &map, &options, Profile::Xpath31, Limits::new())
            .unwrap()
            .all_conformant()
    );
    let error = validator
        .validate(
            &data,
            &map,
            &options,
            Profile::Xpath31,
            Limits::new().with(Resource::CompileSteps, 0),
        )
        .unwrap_err();
    refused(&error, Resource::CompileSteps);
    assert!(
        !validator
            .validate(&data, &map, &options, Profile::Xpath31, Limits::new())
            .unwrap()
            .all_conformant()
    );
}

#[test]
fn negation_alternatives_references_and_partial_maps_do_not_erase_refusal() {
    let data = empty_dataset();
    let options = ValidationOptions::default();
    let pattern = node("a", "");
    let pass = r#"{"type":"NodeConstraint"}"#;
    for expression in [
        format!(r#"{{"type":"ShapeNot","shapeExpr":{pattern}}}"#),
        format!(r#"{{"type":"ShapeOr","shapeExprs":[{pattern},{pass}]}}"#),
        format!(r#"{{"type":"ShapeAnd","shapeExprs":[{pattern},{pass}]}}"#),
    ] {
        let parsed = schema(&expression);
        let error = validate_with_xpath(
            &parsed,
            &data,
            &[association("a")],
            &options,
            Profile::Xpath31,
            Limits::new().with(Resource::MatchSteps, 0),
        )
        .unwrap_err();
        refused(&error, Resource::MatchSteps);
    }
    let pass = labeled("urn:example:pass", pass);
    let pattern = labeled("urn:example:pattern", &pattern);
    let parsed = parse_shexj(
        &format!(
            r#"{{"type":"Schema","shapes":[{pass},{pattern}],"start":"urn:example:pattern"}}"#
        ),
        None,
    )
    .unwrap();
    let map = [
        (
            association("a").0,
            ShapeSelector::Label("urn:example:pass".to_owned()),
        ),
        association("a"),
    ];
    let error = validate_with_xpath(
        &parsed,
        &data,
        &map,
        &options,
        Profile::Xpath31,
        Limits::new().with(Resource::MatchSteps, 0),
    )
    .unwrap_err();
    refused(&error, Resource::MatchSteps);
}

#[test]
fn triple_assignment_cannot_hide_an_operational_value_failure() {
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_iri("urn:example:subject");
    let predicate = builder.intern_iri("urn:example:predicate");
    let value = builder.intern_literal(RdfLiteral::simple("aa"));
    builder.push_quad(subject, predicate, value, None);
    let data = builder.freeze().unwrap();
    let pattern = node(r"^(a)\1$", "");
    let parsed = schema(&format!(
        r#"{{"type":"Shape","expression":{{"type":"OneOf","expressions":[{{"type":"TripleConstraint","predicate":"urn:example:predicate","valueExpr":{pattern}}},{{"type":"TripleConstraint","predicate":"urn:example:predicate"}}]}}}}"#
    ));
    let map = [(TermValue::iri("urn:example:subject"), ShapeSelector::Start)];
    let options = ValidationOptions::default();
    let error = validate_with_xpath(
        &parsed,
        &data,
        &map,
        &options,
        Profile::Xpath31,
        Limits::new().with(Resource::MatchSteps, 0),
    )
    .unwrap_err();
    refused(&error, Resource::MatchSteps);
    assert!(
        validate_with_xpath(
            &parsed,
            &data,
            &map,
            &options,
            Profile::Xpath31,
            Limits::new()
        )
        .unwrap()
        .all_conformant()
    );
}

#[test]
fn textual_shape_map_door_preserves_both_parse_and_operational_causes() {
    let data = empty_dataset();
    let options = ValidationOptions::default();
    let parsed = schema(&node(r"^(a)\1$", ""));
    let error = validate_shape_map_with_xpath(
        &parsed,
        &data,
        r#""aa" @START"#,
        None,
        &options,
        Profile::Xpath31,
        Limits::new().with(Resource::PatternBytes, 0),
    )
    .unwrap_err();
    match error {
        XPathValidationError::Regex(error) => refused(&error, Resource::PatternBytes),
        other => panic!("wrong channel: {other}"),
    }
    assert!(matches!(
        validate_shape_map_with_xpath(
            &parsed,
            &data,
            "<urn:example:n>@<urn:example:unknown>",
            None,
            &options,
            Profile::Xpath31,
            Limits::new()
        ),
        Err(XPathValidationError::ShapeMap(
            purrdf_shex::ShexError::UnknownShape(_)
        ))
    ));
}

#[test]
fn interned_labeled_verdicts_do_not_survive_into_a_request_with_less_fuel() {
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_iri("urn:example:subject");
    let predicate = builder.intern_iri("urn:example:predicate");
    let value = builder.intern_literal(RdfLiteral::simple("aa"));
    builder.push_quad(subject, predicate, value, None);
    let data = builder.freeze().unwrap();
    let expr = labeled("urn:example:pattern", &node(r"^(a)\1$", ""));
    let parsed = parse_shexj(
        &format!(r#"{{"type":"Schema","shapes":[{expr}],"start":"urn:example:pattern"}}"#),
        None,
    )
    .unwrap();
    let map = [association("aa"), association("aa")];
    let options = ValidationOptions::default();
    let mut validator = XPathValidator::new(&parsed);
    assert!(
        validator
            .validate(&data, &map, &options, Profile::Xpath31, Limits::new())
            .unwrap()
            .all_conformant()
    );
    let error = validator
        .validate(
            &data,
            &map,
            &options,
            Profile::Xpath31,
            Limits::new().with(Resource::MatchSteps, 0),
        )
        .unwrap_err();
    refused(&error, Resource::MatchSteps);
    assert!(
        validator
            .validate(&data, &map, &options, Profile::Xpath20, Limits::new())
            .unwrap()
            .all_conformant()
    );
}
