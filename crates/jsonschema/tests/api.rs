// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The public surface: every refusal next to the valid neighbour it must not
//! swallow, and the three output formats.

use purrdf_jsonschema::{
    DRAFT_07, DRAFT_2019_09, OutputFormat, Registry, Schema, SchemaError, ecma::PatternError,
};
use serde_json::{Value, json};

const DRAFT: &str = "https://json-schema.org/draft/2020-12/schema";

fn compile(document: Value) -> Result<Schema, SchemaError> {
    Schema::from_document("https://example.org/schema.json", document)
}

/// A tuple schema in each implemented dialect's spelling: one string, then
/// nothing.
fn tuple_of_one_string(dialect: &str) -> Value {
    if dialect.starts_with(DRAFT) {
        json!({"$schema": dialect, "prefixItems": [{"type": "string"}], "items": false})
    } else {
        json!({"$schema": dialect, "items": [{"type": "string"}], "additionalItems": false})
    }
}

#[test]
fn draft_06_is_refused_and_draft_07_2019_09_and_2020_12_are_accepted() {
    for dialect in [
        "http://json-schema.org/draft-06/schema#",
        "http://json-schema.org/draft-04/schema#",
        "https://json-schema.org/draft/next/schema",
    ] {
        match compile(json!({"$schema": dialect, "type": "string"})) {
            Err(SchemaError::UnsupportedDialect { dialect: got, .. }) => assert_eq!(got, dialect),
            other => panic!("{dialect}: expected a dialect refusal, got {other:?}"),
        }
    }
    // Each accepted neighbour is read in its own dialect: the same tuple is
    // spelled `items` + `additionalItems` in draft-07 and 2019-09 and
    // `prefixItems` + `items` in 2020-12, and each one constrains position.
    for dialect in [
        "http://json-schema.org/draft-07/schema#",
        DRAFT_07,
        DRAFT_2019_09,
        DRAFT,
        "https://json-schema.org/draft/2020-12/schema#",
    ] {
        let schema = compile(tuple_of_one_string(dialect))
            .unwrap_or_else(|error| panic!("{dialect}: {error}"));
        assert!(schema.is_valid(&json!(["a"])), "{dialect}");
        assert!(!schema.is_valid(&json!([1])), "{dialect}");
        assert!(!schema.is_valid(&json!(["a", "b"])), "{dialect}");
    }
    // Array-form `items` is not 2020-12: `items` holds one schema there.
    assert!(matches!(
        compile(json!({"$schema": DRAFT, "items": [{"type": "string"}]})),
        Err(SchemaError::InvalidKeyword { .. })
    ));
    // In 2019-09 `prefixItems` is an unknown keyword, constraining nothing.
    let ignored = compile(json!({"$schema": DRAFT_2019_09, "prefixItems": [{"type": "string"}]}))
        .expect("an unknown keyword is an annotation");
    assert!(ignored.is_valid(&json!([1])));
    let unstated = compile(json!({"type": "string"})).expect("no $schema means 2020-12");
    assert!(!unstated.is_valid(&json!(1)));
}

#[test]
fn a_reference_into_another_dialect_is_refused_and_one_into_an_implemented_dialect_is_evaluated_in_it()
 {
    let mut registry = Registry::new();
    registry
        .add_resource(
            "https://example.org/old.json",
            json!({"$schema": "http://json-schema.org/draft-06/schema#", "type": "string"}),
        )
        .expect("registering a foreign document is not using it");
    registry
        .add_resource(
            "https://example.org/tuple-2019.json",
            tuple_of_one_string(DRAFT_2019_09),
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
            "https://example.org/uses-2019.json",
            json!({"$schema": DRAFT, "$ref": "tuple-2019.json"}),
        )
        .expect("register");
    assert!(matches!(
        registry.compile("https://example.org/uses-old.json"),
        Err(SchemaError::UnsupportedDialect { .. })
    ));
    let schema = registry
        .compile("https://example.org/uses-2019.json")
        .expect("a 2020-12 schema may reference a 2019-09 one");
    assert!(schema.is_valid(&json!(["a"])));
    assert!(
        !schema.is_valid(&json!(["a", "b"])),
        "additionalItems applies"
    );
    assert!(!schema.is_valid(&json!([1])), "array-form items applies");
}

#[test]
fn draft_07_ref_overrides_its_siblings_and_2019_09_ref_does_not() {
    let draft_07 = compile(json!({
        "$schema": DRAFT_07,
        "definitions": {"s": {"type": "string"}},
        "properties": {"a": {"$ref": "#/definitions/s", "maxLength": 1}}
    }))
    .expect("compiles");
    assert!(
        draft_07.is_valid(&json!({"a": "long"})),
        "maxLength is ignored"
    );
    assert!(!draft_07.is_valid(&json!({"a": 1})));
    let draft_2019 = compile(json!({
        "$schema": DRAFT_2019_09,
        "$defs": {"s": {"type": "string"}},
        "properties": {"a": {"$ref": "#/$defs/s", "maxLength": 1}}
    }))
    .expect("compiles");
    assert!(
        !draft_2019.is_valid(&json!({"a": "long"})),
        "maxLength applies"
    );
    assert!(draft_2019.is_valid(&json!({"a": "l"})));
}

#[test]
fn a_draft_07_plain_name_id_is_an_anchor() {
    let schema = compile(json!({
        "$schema": DRAFT_07,
        "definitions": {"s": {"$id": "#short", "maxLength": 1}},
        "items": {"$ref": "#short"}
    }))
    .expect("compiles");
    assert!(schema.is_valid(&json!(["a"])));
    assert!(!schema.is_valid(&json!(["ab"])));
    assert!(matches!(
        compile(json!({"$schema": DRAFT_07, "definitions": {"s": {"$id": "#1bad"}}})),
        Err(SchemaError::InvalidIdentifier { .. })
    ));
}

#[test]
fn recursive_ref_other_than_the_root_is_refused_and_the_root_follows_the_dynamic_scope() {
    assert!(matches!(
        compile(
            json!({"$schema": DRAFT_2019_09, "$defs": {"a": true}, "$recursiveRef": "#/$defs/a"})
        ),
        Err(SchemaError::InvalidKeyword { .. })
    ));
    // The extensible tree: `tree.json` recurses through `$recursiveRef`, and
    // `strict.json` extends it; evaluated from `strict.json`, the recursion
    // lands on `strict.json`, so nested nodes are strict too.
    let mut registry = Registry::new();
    registry
        .add_resource(
            "https://example.org/tree.json",
            json!({
                "$schema": DRAFT_2019_09,
                "$recursiveAnchor": true,
                "type": "object",
                "properties": {"children": {"type": "array", "items": {"$recursiveRef": "#"}}}
            }),
        )
        .expect("tree");
    registry
        .add_resource(
            "https://example.org/strict.json",
            json!({
                "$schema": DRAFT_2019_09,
                "$recursiveAnchor": true,
                "$ref": "tree.json",
                "unevaluatedProperties": false
            }),
        )
        .expect("strict");
    let tree = registry
        .compile("https://example.org/tree.json")
        .expect("tree");
    let strict = registry
        .compile("https://example.org/strict.json")
        .expect("strict");
    let nested_extra = json!({"children": [{"extra": 1}]});
    assert!(tree.is_valid(&nested_extra));
    assert!(!strict.is_valid(&nested_extra), "the recursion is strict");
    assert!(strict.is_valid(&json!({"children": [{"children": []}]})));
}

#[test]
fn only_the_2020_12_contains_evaluates_items_for_unevaluated_items() {
    let schema = |dialect: &str, extra: Value| {
        let mut document = json!({
            "$schema": dialect,
            "contains": {"type": "string"},
            "unevaluatedItems": false
        });
        if let (Some(document), Some(extra)) = (document.as_object_mut(), extra.as_object()) {
            document.extend(extra.clone());
        }
        compile(document).unwrap_or_else(|error| panic!("{dialect}: {error}"))
    };
    assert!(schema(DRAFT, json!({})).is_valid(&json!(["a"])));
    assert!(
        !schema(DRAFT, json!({})).is_valid(&json!(["a", 1])),
        "1 is unevaluated"
    );
    assert!(
        !schema(DRAFT_2019_09, json!({})).is_valid(&json!(["a"])),
        "`contains` evaluated nothing"
    );
    assert!(
        schema(DRAFT_2019_09, json!({"items": {"type": "string"}})).is_valid(&json!(["a"])),
        "`items` evaluated it"
    );
}

#[test]
fn anchor_names_follow_their_dialect() {
    let with_anchor = |dialect: &str, anchor: &str| {
        compile(json!({
            "$schema": dialect,
            "$defs": {"s": {"$anchor": anchor, "type": "string"}},
            "$ref": format!("#{anchor}")
        }))
    };
    assert!(with_anchor(DRAFT_2019_09, "a:b").is_ok());
    assert!(matches!(
        with_anchor(DRAFT, "a:b"),
        Err(SchemaError::InvalidIdentifier { .. })
    ));
    assert!(with_anchor(DRAFT, "_a").is_ok());
    assert!(matches!(
        with_anchor(DRAFT_2019_09, "_a"),
        Err(SchemaError::InvalidIdentifier { .. })
    ));
}

#[test]
fn the_default_dialect_is_settable_to_an_implemented_dialect_only() {
    let mut registry = Registry::new();
    assert!(matches!(
        registry.set_default_dialect("http://json-schema.org/draft-06/schema#"),
        Err(SchemaError::UnsupportedDialect { .. })
    ));
    registry
        .add_resource(
            "https://example.org/2020.json",
            json!({"items": [{"type": "string"}]}),
        )
        .expect("register");
    assert!(
        matches!(
            registry.compile("https://example.org/2020.json"),
            Err(SchemaError::InvalidKeyword { .. })
        ),
        "the refused default left 2020-12 in place"
    );
    registry
        .set_default_dialect("http://json-schema.org/draft-07/schema#")
        .expect("draft-07 is implemented");
    registry
        .add_resource(
            "https://example.org/07.json",
            json!({"items": [{"type": "string"}], "additionalItems": false}),
        )
        .expect("register");
    let schema = registry
        .compile("https://example.org/07.json")
        .expect("draft-07");
    assert!(schema.is_valid(&json!(["a"])));
    assert!(!schema.is_valid(&json!(["a", 1])));
}

#[test]
fn a_document_registered_before_its_metaschema_waits_for_it() {
    let mut registry = Registry::new();
    registry
        .add_resource(
            "https://example.org/schema.json",
            json!({"$schema": "https://example.org/meta.json", "$anchor": "a:b", "minimum": 2}),
        )
        .expect("a document may arrive before its meta-schema");
    assert!(matches!(
        registry.compile("https://example.org/schema.json"),
        Err(SchemaError::UnsupportedDialect { dialect, .. })
            if dialect == "https://example.org/meta.json"
    ));
    registry
        .add_resource(
            "https://example.org/meta.json",
            json!({"$schema": DRAFT_2019_09, "$recursiveAnchor": true, "$ref": DRAFT_2019_09}),
        )
        .expect("meta");
    let schema = registry
        .compile("https://example.org/schema.json#a:b")
        .expect("scanned as 2019-09 once its meta-schema arrived");
    assert!(!schema.is_valid(&json!(1)));
    assert!(schema.is_valid(&json!(2)));
}

#[test]
fn the_2019_09_format_vocabulary_asserts_only_when_required() {
    let meta = |required: bool| {
        json!({
            "$schema": DRAFT_2019_09,
            "$vocabulary": {
                "https://json-schema.org/draft/2019-09/vocab/core": true,
                "https://json-schema.org/draft/2019-09/vocab/format": required
            },
            "$recursiveAnchor": true
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
                json!({"$schema": "https://example.org/meta.json", "format": "ipv4"}),
            )
            .expect("schema");
        let schema = registry
            .compile("https://example.org/schema.json")
            .expect("compiles");
        assert_eq!(
            schema.is_valid(&json!("not-an-ipv4")),
            !required,
            "{required}"
        );
        assert!(schema.is_valid(&json!("127.0.0.1")));
    }
}

#[test]
fn draft_07_content_asserts_what_it_can_decode_and_annotates_the_rest() {
    let json_in_base64 = compile(json!({
        "$schema": DRAFT_07,
        "contentEncoding": "base64",
        "contentMediaType": "application/json"
    }))
    .expect("compiles");
    assert!(json_in_base64.is_valid(&json!("eyJmb28iOiAiYmFyIn0K")));
    assert!(
        !json_in_base64.is_valid(&json!("ezp9Cg==")),
        "decodes to an object with no key"
    );
    assert!(!json_in_base64.is_valid(&json!("{}")), "not base64");
    let png = compile(json!({"$schema": DRAFT_07, "contentMediaType": "image/png"}))
        .expect("an unparsed media type is an annotation");
    assert!(png.is_valid(&json!("anything")));
    let in_2019 =
        compile(json!({"$schema": DRAFT_2019_09, "contentMediaType": "application/json"}))
            .expect("compiles");
    assert!(
        in_2019.is_valid(&json!("{\"a\"}")),
        "2019-09 content is an annotation"
    );
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

#[test]
fn a_custom_metaschema_is_seen_by_its_registry_only() {
    const META: &str = "https://example.org/meta.json";
    const SCHEMA: &str = "https://example.org/schema.json";
    let asserting_meta = json!({
        "$schema": DRAFT,
        "$vocabulary": {
            "https://json-schema.org/draft/2020-12/vocab/core": true,
            "https://json-schema.org/draft/2020-12/vocab/format-assertion": true
        },
        "$dynamicAnchor": "meta"
    });
    let annotating_meta = json!({
        "$schema": DRAFT,
        "$vocabulary": {
            "https://json-schema.org/draft/2020-12/vocab/core": true,
            "https://json-schema.org/draft/2020-12/vocab/format-annotation": true
        },
        "$dynamicAnchor": "meta"
    });
    let schema = json!({"$schema": META, "format": "ipv6"});

    let mut first = Registry::new();
    first
        .add_resource(META, asserting_meta)
        .expect("the first registry's meta-schema");
    first
        .add_resource(SCHEMA, schema.clone())
        .expect("the first registry's schema");

    // A second registry shares the vendored meta-schemas, not the first
    // registry's own: its schema waits for a meta-schema it has not seen.
    let mut second = Registry::new();
    second
        .add_resource(SCHEMA, schema)
        .expect("the second registry's schema");
    assert!(matches!(
        second.compile(SCHEMA),
        Err(SchemaError::UnsupportedDialect { dialect, .. }) if dialect == META
    ));
    // The URI the first registry claimed is free in the second.
    second
        .add_resource(META, annotating_meta)
        .expect("the second registry's own meta-schema at the same URI");

    let asserting = first.compile(SCHEMA).expect("the first compiles");
    let annotating = second.compile(SCHEMA).expect("the second compiles");
    assert!(asserting.is_valid(&json!("2001:db8::1")));
    assert!(!asserting.is_valid(&json!("2001:db8::1::2")));
    assert!(annotating.is_valid(&json!("2001:db8::1")));
    assert!(
        annotating.is_valid(&json!("2001:db8::1::2")),
        "the second registry's meta-schema only annotates"
    );

    // A clone carries what was added to the original; what is added to the
    // clone stays in the clone.
    let mut clone = first.clone();
    clone
        .add_resource(
            "https://example.org/only-in-clone.json",
            json!({"type": "null"}),
        )
        .expect("the clone's own document");
    assert!(
        clone
            .compile("https://example.org/only-in-clone.json")
            .is_ok()
    );
    assert!(matches!(
        first.compile("https://example.org/only-in-clone.json"),
        Err(SchemaError::UnresolvedReference { .. })
    ));
    assert!(
        !clone
            .compile(SCHEMA)
            .expect("inherited")
            .is_valid(&json!("x"))
    );

    // Every registry sees the vendored meta-schemas, and none may re-register
    // one.
    for registry in [&mut first, &mut second] {
        let meta = registry.compile(DRAFT).expect("the vendored meta-schema");
        assert!(meta.is_valid(&json!({"type": "string"})));
        assert!(!meta.is_valid(&json!({"type": 5})));
        assert!(matches!(
            registry.add_resource(DRAFT, json!({})),
            Err(SchemaError::DuplicateResource { .. })
        ));
        registry
            .add_resource("https://example.org/fresh.json", json!({}))
            .expect("a URI no registry has claimed");
    }
}
