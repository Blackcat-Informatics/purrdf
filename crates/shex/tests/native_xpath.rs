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

#[test]
fn adversary_patterns_conform_at_the_production_defaults_like_the_compatibility_engine() {
    // Each shape was refused by the native matcher at a fraction of these sizes.
    let data = empty_dataset();
    let options = ValidationOptions::default();
    let text = purrdf_testkit::text::word_prose(1 << 20);
    let pairs = "ab".repeat(1 << 19);
    let forty = "a".repeat(40);
    for (pattern, value, expected) in [
        ("node.*graph.*zzz", text.clone(), false),
        ("node.*graph.*zzz", format!("{text} zzz"), true),
        ("alpha.*zzz", text.clone(), false),
        ("alpha.*zzz", format!("{text} zzz"), true),
        ("^([a-z]+ ?)+$", text.clone(), true),
        ("^([a-z]+ ?)+$", format!("{text}!"), false),
        (r"^(\w+\s)*\w+$", text.clone(), true),
        (r"^(\w+\s)*\w+$", format!("{text} "), false),
        ("^(a|b)*$", pairs.clone(), true),
        ("^(a|b)*$", format!("{pairs}c"), false),
        ("^(ab)*$", pairs.clone(), true),
        ("^(ab)*$", format!("{pairs}a"), false),
        ("^(?:ab)*$", pairs.clone(), true),
        ("^(?:ab)*$", format!("{pairs}a"), false),
        ("^(a|aa)*$|^(a*)*b$", format!("{forty}b"), true),
        ("^(a|aa)*$|^(a*)*b$", format!("{forty}c"), false),
    ] {
        let parsed = schema(&node(pattern, ""));
        let map = [association(&value)];
        assert_eq!(
            validate(&parsed, &data, &map).all_conformant(),
            expected,
            "compatibility {pattern}"
        );
        for profile in Profile::ALL {
            if profile == Profile::Xpath20 && pattern.contains("(?:") {
                continue;
            }
            let result =
                validate_with_xpath(&parsed, &data, &map, &options, profile, Limits::new())
                    .unwrap_or_else(|error| panic!("{profile:?} {pattern}: {error}"));
            assert_eq!(result.all_conformant(), expected, "{profile:?} {pattern}");
        }
    }
}

#[test]
fn a_backreference_blowup_pattern_still_refuses_beside_a_conformant_neighbour() {
    let data = empty_dataset();
    let options = ValidationOptions::default();
    let parsed = schema(&node(r"^(a|aa)*c\1$", ""));
    let forty = "a".repeat(40);
    for profile in Profile::ALL {
        let error = validate_with_xpath(
            &parsed,
            &data,
            &[association(&forty)],
            &options,
            profile,
            Limits::new(),
        )
        .unwrap_err();
        assert!(
            matches!(
                &error,
                Error::Resource(cause) if matches!(
                    cause.resource,
                    Resource::MatchSteps | Resource::MatchStates | Resource::MatchSlots
                )
            ),
            "{profile:?}: {error}"
        );
        assert!(
            validate_with_xpath(
                &parsed,
                &data,
                &[association(&format!("{forty}ca"))],
                &options,
                profile,
                Limits::new(),
            )
            .unwrap()
            .all_conformant(),
            "{profile:?}"
        );
    }
}

#[test]
fn counted_repetition_patterns_conform_at_the_production_defaults_like_the_compatibility_engine() {
    // Each shape was refused by the native matcher at these sizes: a counted
    // group repeated from every start kept one thread per distinct count.
    let data = empty_dataset();
    let options = ValidationOptions::default();
    let pairs = "ab".repeat(1 << 19);
    let mixed = "abbaab".repeat((4 << 20) / 6);
    let prose = purrdf_testkit::text::word_prose(8 << 20);
    let prose_4m = &prose[..prose[..4 << 20].rfind(' ').unwrap()];
    let quads = "abcd".repeat(1 << 18);
    // A count the compatibility engine refuses to build is compared through the
    // pattern with the same matches.
    for (pattern, compatible, value, suffix) in [
        ("(ab){1,1000}c", "(ab){1,1000}c", &pairs[..128 << 10], "c"),
        ("(ab){2,50}c", "(ab){2,50}c", pairs.as_str(), "c"),
        ("(ab){1,100}c", "(ab){1,100}c", pairs.as_str(), "c"),
        ("(ab|cd){1,20}e", "(ab|cd){1,20}e", quads.as_str(), "e"),
        (
            "((a|b){3}){5,9}c",
            "((a|b){3}){5,9}c",
            &mixed[..400 << 10],
            "c",
        ),
        (
            "((a|b){2}){2,5}c",
            "((a|b){2}){2,5}c",
            &mixed[..800 << 10],
            "c",
        ),
        ("(a|b){1,30}c", "(a|b){1,30}c", mixed.as_str(), "c"),
        ("(a|b){3,9}c", "(a|b){3,9}c", mixed.as_str(), "c"),
        (r"(\w+\s){3,5}zzz", r"(\w+\s){3,5}zzz", prose_4m, " zzz"),
        (
            "node.*graph.*zzz",
            "node.*graph.*zzz",
            prose.as_str(),
            " zzz",
        ),
        ("(ab){1,100000}c", "abc", &pairs[..128 << 10], "c"),
        ("^(a?){18446744073709551616}$", "^a*$", "b", ""),
    ] {
        let completed = format!("{value}{suffix}");
        for (text, expected) in [(value, false), (completed.as_str(), !suffix.is_empty())] {
            let map = [association(text)];
            assert_eq!(
                validate(&schema(&node(compatible, "")), &data, &map).all_conformant(),
                expected,
                "compatibility {compatible} over {}",
                text.len()
            );
            let parsed = schema(&node(pattern, ""));
            for profile in Profile::ALL {
                let result =
                    validate_with_xpath(&parsed, &data, &map, &options, profile, Limits::new())
                        .unwrap_or_else(|error| panic!("{profile:?} {pattern}: {error}"));
                assert_eq!(
                    result.all_conformant(),
                    expected,
                    "{profile:?} {pattern} over {}",
                    text.len()
                );
            }
        }
    }
    // A nullable body required more than u64 times conforms on the empty string.
    let parsed = schema(&node("^(a?){18446744073709551616}$", ""));
    for profile in Profile::ALL {
        assert!(
            validate_with_xpath(
                &parsed,
                &data,
                &[association("")],
                &options,
                profile,
                Limits::new()
            )
            .unwrap()
            .all_conformant(),
            "{profile:?}"
        );
    }
}
