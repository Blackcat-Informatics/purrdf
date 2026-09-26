// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The public surface: every refusal next to the valid neighbour it must not
//! swallow, and the three output formats.

use purrdf_jsonschema::{OutputFormat, Registry, Schema, SchemaError, ecma::PatternError};
use serde_json::{Value, json};

const DRAFT: &str = "https://json-schema.org/draft/2020-12/schema";

fn compile(document: Value) -> Result<Schema, SchemaError> {
    Schema::from_document("https://example.org/schema.json", document)
}

#[test]
fn draft_07_is_refused_and_2020_12_is_accepted() {
    for dialect in [
        "http://json-schema.org/draft-07/schema#",
        "http://json-schema.org/draft-04/schema#",
        "https://json-schema.org/draft/2019-09/schema",
    ] {
        match compile(json!({"$schema": dialect, "type": "string"})) {
            Err(SchemaError::UnsupportedDialect { dialect: got, .. }) => assert_eq!(got, dialect),
            other => panic!("{dialect}: expected a dialect refusal, got {other:?}"),
        }
    }
    let accepted = compile(json!({"$schema": DRAFT, "type": "string"})).expect("2020-12 compiles");
    assert!(accepted.is_valid(&json!("text")));
    assert!(!accepted.is_valid(&json!(1)));
    let with_hash = compile(json!({"$schema": format!("{DRAFT}#"), "type": "string"}))
        .expect("the empty fragment spelling is the same dialect");
    assert!(with_hash.is_valid(&json!("text")));
    let unstated = compile(json!({"type": "string"})).expect("no $schema means 2020-12");
    assert!(!unstated.is_valid(&json!(1)));
}

#[test]
fn a_reference_into_another_dialect_is_refused_and_a_2020_12_reference_is_followed() {
    let mut registry = Registry::new();
    registry
        .add_resource(
            "https://example.org/old.json",
            json!({"$schema": "http://json-schema.org/draft-07/schema#", "type": "string"}),
        )
        .expect("registering a foreign document is not using it");
    registry
        .add_resource(
            "https://example.org/new.json",
            json!({"$schema": DRAFT, "type": "string"}),
        )
        .expect("register");
    registry
        .add_resource(
            "https://example.org/uses-old.json",
            json!({"$ref": "old.json"}),
        )
        .expect("register");
    registry
        .add_resource(
            "https://example.org/uses-new.json",
            json!({"$ref": "new.json"}),
        )
        .expect("register");
    assert!(matches!(
        registry.compile("https://example.org/uses-old.json"),
        Err(SchemaError::UnsupportedDialect { .. })
    ));
    let schema = registry
        .compile("https://example.org/uses-new.json")
        .expect("compiles");
    assert!(schema.is_valid(&json!("a")));
    assert!(!schema.is_valid(&json!(1)));
}

#[test]
fn a_required_unknown_vocabulary_is_refused_and_an_optional_one_is_ignored() {
    let meta = |required: bool| {
        json!({
            "$schema": DRAFT,
            "$vocabulary": {
                "https://json-schema.org/draft/2020-12/vocab/core": true,
                "https://json-schema.org/draft/2020-12/vocab/validation": true,
                "https://example.org/vocab/custom": required
            },
            "$ref": DRAFT
        })
    };
    for required in [true, false] {
        let mut registry = Registry::new();
        registry
            .add_resource("https://example.org/meta.json", meta(required))
            .expect("meta");
        registry
            .add_resource(
                "https://example.org/schema.json",
                json!({"$schema": "https://example.org/meta.json", "minimum": 2, "properties": {"a": false}}),
            )
            .expect("schema");
        let compiled = registry.compile("https://example.org/schema.json");
        if required {
            assert!(matches!(
                compiled,
                Err(SchemaError::UnsupportedVocabulary { .. })
            ));
        } else {
            let schema = compiled.expect("an optional unknown vocabulary is ignored");
            assert!(!schema.is_valid(&json!(1)), "validation is in force");
            assert!(
                schema.is_valid(&json!({"a": 1})),
                "the applicator vocabulary is not declared"
            );
        }
    }
}

#[test]
fn asserted_formats_this_crate_cannot_check_are_refused_and_checkable_ones_assert() {
    let mut registry = Registry::new();
    registry
        .add_resource(
            "https://example.org/assert.json",
            json!({
                "$schema": DRAFT,
                "$vocabulary": {
                    "https://json-schema.org/draft/2020-12/vocab/core": true,
                    "https://json-schema.org/draft/2020-12/vocab/format-assertion": true
                },
                "$dynamicAnchor": "meta"
            }),
        )
        .expect("meta");
    for (index, format) in ["idn-hostname", "idn-email", "hostname", "no-such-format"]
        .iter()
        .enumerate()
    {
        let uri = format!("https://example.org/refused/{index}");
        registry
            .add_resource(
                &uri,
                json!({"$schema": "https://example.org/assert.json", "format": format}),
            )
            .expect("schema");
        match registry.compile(&uri) {
            Err(SchemaError::UnsupportedFormat { format: got, .. }) => assert_eq!(&got, format),
            other => panic!("{format}: expected a format refusal, got {other:?}"),
        }
    }
    registry
        .add_resource(
            "https://example.org/ipv4.json",
            json!({"$schema": "https://example.org/assert.json", "format": "ipv4"}),
        )
        .expect("schema");
    let asserting = registry
        .compile("https://example.org/ipv4.json")
        .expect("ipv4 asserts");
    assert!(asserting.is_valid(&json!("127.0.0.1")));
    assert!(!asserting.is_valid(&json!("not-an-ipv4")));
    let annotating =
        compile(json!({"format": "idn-hostname"})).expect("an annotation needs no check");
    assert!(annotating.is_valid(&json!("anything at all")));
}

#[test]
fn an_unrunnable_pattern_is_refused_and_its_runnable_neighbour_compiles() {
    match compile(json!({"pattern": "^(?=a)b"})) {
        Err(SchemaError::Pattern {
            error: PatternError::Unsupported { construct, .. },
            ..
        }) => assert_eq!(construct, "lookahead"),
        other => panic!("expected a lookahead refusal, got {other:?}"),
    }
    match compile(json!({"patternProperties": {"(a)\\1": true}})) {
        Err(SchemaError::Pattern {
            error: PatternError::Unsupported { construct, .. },
            ..
        }) => assert_eq!(construct, "backreference"),
        other => panic!("expected a backreference refusal, got {other:?}"),
    }
    assert!(matches!(
        compile(json!({"pattern": "\\a"})),
        Err(SchemaError::Pattern {
            error: PatternError::Syntax { .. },
            ..
        })
    ));
    let neighbour = compile(json!({"pattern": "^(?:a)b"})).expect("a plain group compiles");
    assert!(neighbour.is_valid(&json!("ab")));
    assert!(!neighbour.is_valid(&json!("b")));
}

#[test]
fn a_schema_invalid_against_the_metaschema_is_refused_and_its_valid_neighbour_is_not() {
    match compile(json!({"required": ["a", "a"]})) {
        Err(SchemaError::InvalidSchema { errors, .. }) => {
            assert!(
                errors
                    .iter()
                    .any(|(_, instance, _)| instance == "/required"),
                "{errors:?}"
            );
        }
        other => {
            panic!("expected the meta-schema to refuse duplicate required names, got {other:?}")
        }
    }
    let neighbour = compile(json!({"required": ["a", "b"]})).expect("distinct names are valid");
    assert!(!neighbour.is_valid(&json!({"a": 1})));
    assert!(neighbour.is_valid(&json!({"a": 1, "b": 2})));
    assert!(matches!(
        compile(json!({"minLength": -1})),
        Err(SchemaError::InvalidKeyword { .. })
    ));
    assert!(matches!(
        compile(json!(5)),
        Err(SchemaError::InvalidKeyword { .. })
    ));
}

#[test]
fn identifiers_are_checked_when_registered() {
    let mut registry = Registry::new();
    assert!(matches!(
        registry.add_resource("relative.json", json!({})),
        Err(SchemaError::InvalidUri { .. })
    ));
    assert!(matches!(
        registry.add_resource("https://example.org/a.json#frag", json!({})),
        Err(SchemaError::InvalidUri { .. })
    ));
    assert!(matches!(
        registry.add_resource("https://example.org/a.json", json!({"$anchor": "1bad"})),
        Err(SchemaError::InvalidIdentifier { .. })
    ));
    assert!(matches!(
        registry.add_resource(
            "https://example.org/b.json",
            json!({"$defs": {"x": {"$id": "c.json#frag"}}})
        ),
        Err(SchemaError::InvalidIdentifier { .. })
    ));
    registry
        .add_resource("https://example.org/a.json#", json!({"$anchor": "good"}))
        .expect("an empty fragment and a well-formed anchor register");
    assert!(matches!(
        registry.add_resource("https://example.org/a.json", json!({})),
        Err(SchemaError::DuplicateResource { .. })
    ));
    assert!(matches!(
        registry.add_resource(DRAFT, json!({})),
        Err(SchemaError::DuplicateResource { .. })
    ));
    assert!(registry.compile("https://example.org/a.json#good").is_ok());
    assert!(matches!(
        registry.compile("https://example.org/a.json#missing"),
        Err(SchemaError::UnresolvedReference { .. })
    ));
}

#[test]
fn an_unresolved_reference_is_refused_and_a_resolved_one_is_followed() {
    assert!(matches!(
        compile(json!({"$ref": "https://example.org/elsewhere.json"})),
        Err(SchemaError::UnresolvedReference { .. })
    ));
    let local =
        compile(json!({"$ref": "#/$defs/s", "$defs": {"s": {"type": "string"}}})).expect("local");
    assert!(local.is_valid(&json!("s")));
}

#[test]
fn a_reference_cycle_that_consumes_nothing_fails_instead_of_recursing() {
    let schema = compile(json!({"$ref": "#/$defs/a", "$defs": {"a": {"$ref": "#/$defs/b"}, "b": {"$ref": "#/$defs/a"}}}))
        .expect("a cycle compiles");
    assert!(!schema.is_valid(&json!(1)));
    let output = schema.evaluate(&json!(1));
    assert!(output.errors().any(|unit| {
        unit.error
            .as_deref()
            .is_some_and(|error| error.contains("reference cycle"))
    }));
    let consuming = compile(json!({"$defs": {"tree": {"type": "array", "items": {"$ref": "#/$defs/tree"}}}, "$ref": "#/$defs/tree"}))
        .expect("a consuming cycle compiles");
    assert!(consuming.is_valid(&json!([[[]], []])));
    assert!(!consuming.is_valid(&json!([[1]])));
}

#[test]
fn the_three_output_formats_project_one_evaluation() {
    let schema = compile(json!({
        "$id": "https://example.org/person.json",
        "title": "Person",
        "type": "object",
        "properties": {
            "name": {"type": "string", "readOnly": true},
            "age": {"$ref": "#/$defs/age"}
        },
        "$defs": {"age": {"type": "integer", "minimum": 0}}
    }))
    .expect("compiles");

    let invalid = schema.evaluate(&json!({"name": 1, "age": -1}));
    assert!(!invalid.is_valid());
    assert_eq!(invalid.to_json(OutputFormat::Flag), json!({"valid": false}));
    let basic = invalid.to_json(OutputFormat::Basic);
    let errors = basic["errors"].as_array().expect("errors");
    assert!(basic.get("annotations").is_none());
    let find = |location: &str| {
        errors
            .iter()
            .find(|unit| unit["keywordLocation"] == location)
            .unwrap_or_else(|| panic!("no error at {location}: {basic}"))
    };
    let name = find("/properties/name/type");
    assert_eq!(name["instanceLocation"], "/name");
    assert_eq!(
        name["absoluteKeywordLocation"],
        "https://example.org/person.json#/properties/name/type"
    );
    let age = find("/properties/age/$ref/minimum");
    assert_eq!(age["instanceLocation"], "/age");
    assert_eq!(
        age["absoluteKeywordLocation"],
        "https://example.org/person.json#/$defs/age/minimum"
    );
    assert!(
        errors
            .iter()
            .all(|unit| unit.get("annotation").is_none() && unit["valid"] == false)
    );

    let detailed = invalid.to_json(OutputFormat::Detailed);
    assert_eq!(detailed["valid"], false);
    assert_eq!(detailed["keywordLocation"], "/properties");
    let nested = detailed["errors"].as_array().expect("detailed errors");
    assert_eq!(nested.len(), 2, "{detailed}");

    let valid = schema.evaluate(&json!({"name": "Ada", "age": 36}));
    assert!(valid.is_valid());
    assert_eq!(valid.to_json(OutputFormat::Flag), json!({"valid": true}));
    let basic = valid.to_json(OutputFormat::Basic);
    assert!(basic.get("errors").is_none());
    let annotations = basic["annotations"].as_array().expect("annotations");
    let annotation = |location: &str| {
        annotations
            .iter()
            .find(|unit| unit["keywordLocation"] == location)
            .map(|unit| unit["annotation"].clone())
    };
    assert_eq!(annotation("/title"), Some(json!("Person")));
    assert_eq!(annotation("/properties/name/readOnly"), Some(json!(true)));
    assert_eq!(annotation("/properties"), Some(json!(["age", "name"])));
    let detailed = valid.to_json(OutputFormat::Detailed);
    assert_eq!(detailed["valid"], true);
}

#[test]
fn annotations_of_failed_subschemas_are_dropped() {
    let schema =
        compile(json!({"anyOf": [{"title": "wrong", "type": "string"}, {"title": "right"}]}))
            .expect("compiles");
    let output = schema.evaluate(&json!(1));
    assert!(output.is_valid());
    let titles: Vec<&Value> = output
        .annotations()
        .filter(|unit| unit.keyword_location.ends_with("/title"))
        .filter_map(|unit| unit.annotation.as_ref())
        .collect();
    assert_eq!(titles, [&json!("right")]);
}

#[test]
fn unevaluated_properties_see_through_every_in_place_applicator() {
    let schema = compile(json!({
        "allOf": [{"properties": {"a": true}}],
        "anyOf": [{"properties": {"b": true}}, {"properties": {"c": true}, "required": ["c"]}],
        "if": {"properties": {"d": {"const": 1}}, "required": ["d"]},
        "then": {"properties": {"e": true}},
        "dependentSchemas": {"f": {"properties": {"f": true, "g": true}}},
        "$ref": "#/$defs/h",
        "$defs": {"h": {"properties": {"h": true}}},
        "not": {"properties": {"z": true}, "required": ["never"]},
        "unevaluatedProperties": false
    }))
    .expect("compiles");
    assert!(
        schema.is_valid(&json!({"a": 1, "b": 1, "c": 1, "d": 1, "e": 1, "f": 1, "g": 1, "h": 1}))
    );
    assert!(
        !schema.is_valid(&json!({"z": 1})),
        "`not` contributes no annotations"
    );
    assert!(
        !schema.is_valid(&json!({"d": 2, "e": 1})),
        "`then` did not apply"
    );
    assert!(
        !schema.is_valid(&json!({"g": 1})),
        "`dependentSchemas` did not apply"
    );
}
