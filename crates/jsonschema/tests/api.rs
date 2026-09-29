// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The public surface: every refusal next to the valid neighbour it must not
//! swallow, and the three output formats.

use std::sync::OnceLock;

use purrdf_jsonschema::{
    Dialect, EvaluationCause, MAX_REF_CHAIN, Metaschemas, OutputFormat, Registry, Schema,
    SchemaError,
    ecma::{MatchLimits, PatternError},
};
use serde_json::{Value, json};

const DRAFT: &str = "https://json-schema.org/draft/2020-12/schema";
const DRAFT_2019_09: &str = "https://json-schema.org/draft/2019-09/schema";
const DRAFT_07: &str = "http://json-schema.org/draft-07/schema#";

/// The vendored meta-schemas of all three drafts, built once for every test.
fn metaschemas() -> &'static Metaschemas {
    static SET: OnceLock<Metaschemas> = OnceLock::new();
    SET.get_or_init(|| {
        Metaschemas::new(
            purrdf_testkit::jsonschema_metaschemas::all()
                .map(|(uri, text)| (uri, serde_json::from_str::<Value>(text).expect("JSON"))),
        )
        .expect("the vendored meta-schemas form a set")
    })
}

fn set_of(documents: &[(&'static str, &'static str)]) -> Result<Metaschemas, SchemaError> {
    Metaschemas::new(
        documents
            .iter()
            .map(|&(uri, text)| (uri, serde_json::from_str::<Value>(text).expect("JSON"))),
    )
}

fn registry() -> Registry {
    Registry::with_metaschemas(metaschemas())
}

fn compile(document: Value) -> Result<Schema, SchemaError> {
    Schema::from_document(metaschemas(), "https://example.org/schema.json", document)
}

#[test]
fn parsed_json_numbers_keep_every_decimal_digit() {
    let distinct: Value = serde_json::from_str("1.0000000000000001").expect("JSON number");
    let one: Value = serde_json::from_str("1.0").expect("JSON number");
    assert_ne!(distinct.to_string(), one.to_string());

    let constant = compile(json!({"const": one})).expect("const schema");
    assert!(!constant.is_valid(&distinct).expect("evaluation"));
    let enumeration = compile(json!({"enum": [1.0]})).expect("enum schema");
    assert!(!enumeration.is_valid(&distinct).expect("evaluation"));
    let unique = compile(json!({"uniqueItems": true})).expect("unique schema");
    assert!(
        unique
            .is_valid(&Value::Array(vec![json!(1.0), distinct.clone()]))
            .expect("evaluation")
    );

    let lower = compile(json!({"exclusiveMinimum": 1.0})).expect("bound schema");
    assert!(lower.is_valid(&distinct).expect("evaluation"));
    let integer = compile(json!({"type": "integer"})).expect("type schema");
    assert!(!integer.is_valid(&distinct).expect("evaluation"));
    let multiple = compile(json!({"multipleOf": 0.1})).expect("multiple schema");
    assert!(!multiple.is_valid(&distinct).expect("evaluation"));
}

#[test]
fn draft_06_and_older_are_refused_and_every_supported_spelling_is_accepted() {
    for dialect in [
        "http://json-schema.org/draft-06/schema#",
        "http://json-schema.org/draft-06/schema",
        "http://json-schema.org/draft-04/schema#",
        "http://json-schema.org/draft-03/schema#",
        "http://json-schema.org/schema#",
        "https://json-schema.org/draft/next/schema",
    ] {
        match compile(json!({"$schema": dialect, "type": "string"})) {
            Err(SchemaError::UnsupportedDialect { dialect: got, .. }) => assert_eq!(got, dialect),
            other => panic!("{dialect}: expected a dialect refusal, got {other:?}"),
        }
    }
    for dialect in [
        DRAFT,
        "https://json-schema.org/draft/2020-12/schema#",
        DRAFT_2019_09,
        "https://json-schema.org/draft/2019-09/schema#",
        DRAFT_07,
        "http://json-schema.org/draft-07/schema",
    ] {
        let accepted = compile(json!({"$schema": dialect, "type": "string"}))
            .unwrap_or_else(|error| panic!("{dialect}: {error}"));
        assert!(
            accepted.is_valid(&json!("text")).expect("evaluation"),
            "{dialect}"
        );
        assert!(
            !accepted.is_valid(&json!(1)).expect("evaluation"),
            "{dialect}"
        );
    }
    let unstated = compile(json!({"type": "string"})).expect("no $schema means 2020-12");
    assert!(!unstated.is_valid(&json!(1)).expect("evaluation"));
}

#[test]
fn each_dialect_reads_its_own_keywords() {
    // Array-form `items` with `additionalItems`: draft-07 and 2019-09 only.
    for dialect in [DRAFT_07, DRAFT_2019_09] {
        let schema = compile(json!({
            "$schema": dialect,
            "items": [{"type": "integer"}],
            "additionalItems": false
        }))
        .expect("compiles");
        assert!(schema.is_valid(&json!([1])).expect("evaluation"));
        assert!(!schema.is_valid(&json!(["a"])).expect("evaluation"));
        assert!(!schema.is_valid(&json!([1, 2])).expect("evaluation"));
    }
    // Draft-07 ignores every sibling of `$ref`; 2019-09 applies them.
    let siblings = |dialect: &str| {
        compile(json!({
            "$schema": dialect,
            "definitions": {"any": true},
            "properties": {"a": {"$ref": "#/definitions/any", "type": "string"}}
        }))
        .expect("compiles")
    };
    assert!(
        siblings(DRAFT_07)
            .is_valid(&json!({"a": 1}))
            .expect("evaluation")
    );
    assert!(
        !siblings(DRAFT_2019_09)
            .is_valid(&json!({"a": 1}))
            .expect("evaluation")
    );
    // Draft-07 content assertions; 2019-09 content annotations.
    let content = |dialect: &str| {
        compile(json!({"$schema": dialect, "contentMediaType": "application/json"}))
            .expect("compiles")
    };
    assert!(
        !content(DRAFT_07)
            .is_valid(&json!("[1,"))
            .expect("evaluation")
    );
    assert!(
        content(DRAFT_07)
            .is_valid(&json!("{}"))
            .expect("evaluation")
    );
    assert!(
        content(DRAFT_2019_09)
            .is_valid(&json!("[1,"))
            .expect("evaluation")
    );
    // `unevaluatedItems` does not see `contains` in 2019-09, and does in 2020-12.
    let contains = |dialect: &str| {
        compile(json!({
            "$schema": dialect,
            "contains": {"type": "string"},
            "unevaluatedItems": false
        }))
        .expect("compiles")
    };
    assert!(
        !contains(DRAFT_2019_09)
            .is_valid(&json!(["a"]))
            .expect("evaluation")
    );
    assert!(contains(DRAFT).is_valid(&json!(["a"])).expect("evaluation"));
}

#[test]
fn a_recursive_ref_other_than_the_root_is_refused_and_the_root_is_followed() {
    match compile(json!({
        "$schema": DRAFT_2019_09,
        "$defs": {"a": true},
        "$recursiveRef": "#/$defs/a"
    })) {
        Err(SchemaError::InvalidKeyword { location, .. }) => {
            assert!(location.ends_with("/$recursiveRef"), "{location}");
        }
        other => panic!("expected a $recursiveRef refusal, got {other:?}"),
    }
    let tree = compile(json!({
        "$schema": DRAFT_2019_09,
        "type": "array",
        "items": {"$recursiveRef": "#"}
    }))
    .expect("the root compiles");
    assert!(tree.is_valid(&json!([[], [[]]])).expect("evaluation"));
    assert!(!tree.is_valid(&json!([[1]])).expect("evaluation"));
}

#[test]
fn a_draft_07_id_with_a_pointer_fragment_is_refused_and_a_plain_name_is_an_anchor() {
    match compile(json!({
        "$schema": DRAFT_07,
        "definitions": {"a": {"$id": "#/definitions/a"}}
    })) {
        Err(SchemaError::InvalidIdentifier { .. }) => {}
        other => panic!("expected an identifier refusal, got {other:?}"),
    }
    let schema = compile(json!({
        "$schema": DRAFT_07,
        "allOf": [{"$ref": "#name"}],
        "definitions": {"a": {"$id": "#name", "type": "integer"}}
    }))
    .expect("a plain-name $id compiles");
    assert!(schema.is_valid(&json!(1)).expect("evaluation"));
    assert!(!schema.is_valid(&json!("a")).expect("evaluation"));
}

#[test]
fn a_reference_into_another_dialect_is_read_in_it_and_one_into_an_unsupported_dialect_is_refused() {
    let mut registry = registry();
    registry
        .add_resource(
            "https://example.org/old.json",
            json!({"$schema": "http://json-schema.org/draft-04/schema#", "type": "string"}),
        )
        .expect("registering a foreign document is not using it");
    registry
        .add_resource(
            "https://example.org/seven.json",
            json!({"$schema": DRAFT_07, "items": [{"type": "string"}], "additionalItems": false}),
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
            "https://example.org/uses-seven.json",
            json!({"$ref": "seven.json"}),
        )
        .expect("register");
    assert!(matches!(
        registry.compile("https://example.org/uses-old.json"),
        Err(SchemaError::UnsupportedDialect { .. })
    ));
    let schema = registry
        .compile("https://example.org/uses-seven.json")
        .expect("compiles");
    assert!(schema.is_valid(&json!(["a"])).expect("evaluation"));
    assert!(
        !schema.is_valid(&json!(["a", "b"])).expect("evaluation"),
        "read as draft-07 array-form items"
    );
}

#[test]
fn a_missing_metaschema_is_named_and_its_registered_neighbour_compiles() {
    // No meta-schema registered: compiling needs the 2020-12 one to check the
    // document against.
    let mut bare = Registry::new();
    bare.add_resource("https://example.org/s.json", json!({"type": "string"}))
        .expect("registering needs no meta-schema");
    match bare.compile("https://example.org/s.json") {
        Err(SchemaError::MissingMetaschema {
            metaschema,
            resource,
        }) => {
            assert_eq!(metaschema, DRAFT);
            assert_eq!(resource, "https://example.org/s.json");
        }
        other => panic!("expected a missing meta-schema, got {other:?}"),
    }
    let mut registered = registry();
    registered
        .add_resource("https://example.org/s.json", json!({"type": "string"}))
        .expect("register");
    assert!(registered.compile("https://example.org/s.json").is_ok());

    // A `$ref` to a published meta-schema that is not registered.
    let seven_only = set_of(purrdf_testkit::jsonschema_metaschemas::DRAFT_07)
        .expect("draft-07 alone is a complete set");
    let reference = json!({"$schema": DRAFT_07, "$ref": DRAFT_2019_09});
    match Schema::from_document(&seven_only, "https://example.org/r.json", reference.clone()) {
        Err(SchemaError::MissingMetaschema { metaschema, .. }) => {
            assert_eq!(metaschema, DRAFT_2019_09);
        }
        other => panic!("expected a missing meta-schema, got {other:?}"),
    }
    let schema = Schema::from_document(metaschemas(), "https://example.org/r.json", reference)
        .expect("registered, the reference resolves");
    assert!(
        schema
            .is_valid(&json!({"type": "string"}))
            .expect("evaluation")
    );
    assert!(!schema.is_valid(&json!({"type": 1})).expect("evaluation"));

    // A custom meta-schema is registered before the documents declaring it.
    let uses_meta = json!({"$schema": "https://example.org/meta", "type": "string"});
    let mut registry = registry();
    match registry.add_resource("https://example.org/uses-meta.json", uses_meta.clone()) {
        Err(SchemaError::MissingMetaschema { metaschema, .. }) => {
            assert_eq!(metaschema, "https://example.org/meta");
        }
        other => panic!("expected a missing meta-schema, got {other:?}"),
    }
    registry
        .add_resource(
            "https://example.org/meta",
            json!({"$schema": DRAFT, "$dynamicAnchor": "meta"}),
        )
        .expect("meta");
    registry
        .add_resource("https://example.org/uses-meta.json", uses_meta)
        .expect("registered after its meta-schema");
    let schema = registry
        .compile("https://example.org/uses-meta.json")
        .expect("compiles");
    assert!(!schema.is_valid(&json!(1)).expect("evaluation"));
}

#[test]
fn an_incomplete_metaschema_set_is_refused_and_a_complete_one_is_built() {
    let core_only: Vec<_> = purrdf_testkit::jsonschema_metaschemas::DRAFT_2020_12
        .iter()
        .copied()
        .filter(|(uri, _)| uri.ends_with("/meta/core"))
        .collect();
    assert!(matches!(
        set_of(&core_only),
        Err(SchemaError::MissingMetaschema { metaschema, .. }) if metaschema == DRAFT
    ));
    let whole = set_of(purrdf_testkit::jsonschema_metaschemas::DRAFT_2020_12)
        .expect("the whole 2020-12 set");
    assert!(whole.contains(DRAFT));
    assert!(!whole.contains(DRAFT_2019_09));
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
        let mut registry = registry();
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
            assert!(
                !schema.is_valid(&json!(1)).expect("evaluation"),
                "validation is in force"
            );
            assert!(
                schema.is_valid(&json!({"a": 1})).expect("evaluation"),
                "the applicator vocabulary is not declared"
            );
        }
    }
}

#[test]
fn an_unknown_format_is_refused_under_format_assertion_and_every_defined_one_asserts() {
    let mut registry = registry();
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
    registry
        .add_resource(
            "https://example.org/refused.json",
            json!({"$schema": "https://example.org/assert.json", "format": "no-such-format"}),
        )
        .expect("schema");
    match registry.compile("https://example.org/refused.json") {
        Err(SchemaError::UnsupportedFormat { format, .. }) => assert_eq!(format, "no-such-format"),
        other => panic!("expected a format refusal, got {other:?}"),
    }
    for (index, (format, valid, invalid)) in [
        ("ipv4", "127.0.0.1", "not-an-ipv4"),
        ("hostname", "xn--bcher-kva.example", "-a.example"),
        ("idn-hostname", "b\u{fc}cher.example", "a\u{b7}l.example"),
        ("idn-email", "j\u{f6}e@b\u{fc}cher.example", "j\u{f6}e"),
    ]
    .into_iter()
    .enumerate()
    {
        let uri = format!("https://example.org/asserting/{index}");
        registry
            .add_resource(
                &uri,
                json!({"$schema": "https://example.org/assert.json", "format": format}),
            )
            .expect("schema");
        let asserting = registry.compile(&uri).expect("a defined format asserts");
        assert!(
            asserting.is_valid(&json!(valid)).expect("evaluation"),
            "{format} {valid}"
        );
        assert!(
            !asserting.is_valid(&json!(invalid)).expect("evaluation"),
            "{format} {invalid}"
        );
    }
    let annotating =
        compile(json!({"format": "idn-hostname"})).expect("an annotation needs no check");
    assert!(
        annotating
            .is_valid(&json!("anything at all"))
            .expect("evaluation")
    );
}

#[test]
fn format_assertion_is_opt_in_and_an_unknown_format_stays_an_annotation() {
    for dialect in [
        Dialect::Draft07,
        Dialect::Draft2019_09,
        Dialect::Draft2020_12,
    ] {
        for assert in [false, true] {
            let mut registry = registry();
            registry.set_default_dialect(dialect);
            registry.set_format_assertion(assert);
            registry
                .add_resource(
                    "https://example.org/f.json",
                    json!({"properties": {
                        "a": {"format": "ipv4"},
                        "b": {"format": "no-such-format"}
                    }}),
                )
                .expect("schema");
            let schema = registry
                .compile("https://example.org/f.json")
                .expect("compiles");
            assert_eq!(
                schema
                    .is_valid(&json!({"a": "not-an-ipv4"}))
                    .expect("evaluation"),
                !assert,
                "{dialect:?} assert={assert}"
            );
            assert!(
                schema
                    .is_valid(&json!({"a": "10.0.0.1", "b": "anything"}))
                    .expect("evaluation")
            );
        }
    }
}

#[test]
fn nonregular_patterns_execute_and_malformed_syntax_is_refused() {
    let lookahead = compile(json!({"pattern": "^(?=a)b"})).expect("lookahead compiles");
    assert!(!lookahead.is_valid(&json!("ab")).expect("evaluation"));
    let backreference =
        compile(json!({"patternProperties": {"^(a)\\1$": false}})).expect("backreference compiles");
    assert!(
        !backreference
            .is_valid(&json!({"aa": 1}))
            .expect("evaluation")
    );
    assert!(
        backreference
            .is_valid(&json!({"ab": 1}))
            .expect("evaluation")
    );
    assert!(matches!(
        compile(json!({"pattern": "\\a"})),
        Err(SchemaError::Pattern {
            error: PatternError::Syntax { .. },
            ..
        })
    ));
    let neighbour = compile(json!({"pattern": "^(?:a)b"})).expect("a plain group compiles");
    assert!(neighbour.is_valid(&json!("ab")).expect("evaluation"));
    assert!(!neighbour.is_valid(&json!("b")).expect("evaluation"));
}

#[test]
fn matcher_exhaustion_is_an_error_not_an_invalid_verdict() {
    let schema = compile(json!({"pattern": "(?=a)(a+)+b"})).expect("schema");
    let input = json!("aaaaaaaaaaaaaaaaaaaaaaaaaa");
    let limits = MatchLimits {
        steps: 500,
        states: 500,
    };
    let error = schema
        .is_valid_with_limits(&input, limits)
        .expect_err("budget exceeded");
    assert!(error.keyword_location.ends_with("/pattern"));
    assert!(matches!(
        error.cause,
        EvaluationCause::Pattern(PatternError::Resource { .. })
    ));
    assert!(schema.evaluate_with_limits(&input, limits).is_err());
}

#[test]
fn overdeep_regex_format_is_a_resource_error() {
    let mut registry = registry();
    registry.set_format_assertion(true);
    registry
        .add_resource("https://example.org/regex", json!({"format": "regex"}))
        .expect("resource");
    let schema = registry
        .compile("https://example.org/regex")
        .expect("schema");
    let pattern = format!("{}x{}", "(".repeat(251), ")".repeat(251));
    let error = schema.is_valid(&json!(pattern)).expect_err("depth limit");
    assert!(matches!(
        error.cause,
        EvaluationCause::Pattern(PatternError::Resource { .. })
    ));
}

/// A schema whose root starts a chain of `links` references, each to the
/// next definition, ending at a definition that requires a string.
fn reference_chain(links: usize) -> Value {
    let mut definitions = serde_json::Map::new();
    for link in 1..links {
        definitions.insert(
            format!("d{link}"),
            json!({"$ref": format!("#/$defs/d{}", link + 1)}),
        );
    }
    definitions.insert(format!("d{links}"), json!({"type": "string"}));
    json!({"$ref": "#/$defs/d1", "$defs": definitions})
}

#[test]
fn a_reference_chain_past_its_bound_stops_and_one_at_the_bound_validates() {
    let at_bound = compile(reference_chain(MAX_REF_CHAIN)).expect("schema");
    assert!(at_bound.is_valid(&json!("text")).expect("evaluation"));
    assert!(!at_bound.is_valid(&json!(1)).expect("evaluation"));
    assert!(
        at_bound
            .evaluate(&json!("text"))
            .expect("evaluation")
            .is_valid()
    );

    let past = compile(reference_chain(MAX_REF_CHAIN + 1)).expect("schema");
    let error = past.is_valid(&json!("text")).expect_err("chain bound");
    assert_eq!(error.cause, EvaluationCause::ReferenceChain);
    assert!(error.keyword_location.ends_with("/$ref"), "{error}");
    assert_eq!(error.instance_location, "");
    assert_eq!(
        past.evaluate(&json!("text"))
            .expect_err("chain bound")
            .cause,
        EvaluationCause::ReferenceChain
    );
}

#[test]
fn descending_into_the_instance_starts_a_new_reference_chain() {
    // Every level of the instance follows one reference, so a chain never
    // grows past one link however deep the instance is.
    let schema = compile(json!({
        "type": "array",
        "items": {"$ref": "#"}
    }))
    .expect("schema");
    let mut instance = json!([]);
    for _ in 0..2 * MAX_REF_CHAIN {
        instance = Value::Array(vec![instance]);
    }
    assert!(schema.is_valid(&instance).expect("evaluation"));
}

/// `depth` single-property objects nested inside one another.
fn tree(depth: usize) -> Value {
    let mut instance = json!({});
    for _ in 0..depth {
        let mut node = serde_json::Map::new();
        node.insert("child".to_owned(), instance);
        instance = Value::Object(node);
    }
    instance
}

#[test]
fn a_recursive_schema_validates_a_thousand_deep_instance_on_a_small_stack() {
    let schema = compile(json!({
        "type": "object",
        "properties": {"child": {"$ref": "#"}},
        "unevaluatedProperties": false
    }))
    .expect("schema");
    // 256 KiB of stack: the evaluator keeps its subschemas in progress on the
    // heap, so an instance's depth must not demand machine stack.
    let evaluation = std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(move || {
            let deep = tree(1_000);
            assert!(schema.is_valid(&deep).expect("evaluation"));
            let output = schema.evaluate(&deep).expect("evaluation");
            assert!(output.is_valid());
            drop(output);

            // The verdict is real, not a default: a wrong leaf a thousand levels
            // down fails the whole instance, and says where.
            let mut bad = json!({"child": 1});
            for _ in 0..999 {
                let mut node = serde_json::Map::new();
                node.insert("child".to_owned(), bad);
                bad = Value::Object(node);
            }
            assert!(!schema.is_valid(&bad).expect("evaluation"));
            let output = schema.evaluate(&bad).expect("evaluation");
            let deepest = output
                .errors()
                .map(|unit| unit.instance_location.len())
                .max()
                .expect("errors");
            assert_eq!(deepest, "/child".len() * 1_000);
        });
    evaluation
        .expect("thread")
        .join()
        .expect("the evaluation completes on a small stack");
}

/// `depth` arrays nested inside one another around `1`.
fn nested(depth: usize) -> Value {
    let mut value = json!(1);
    for _ in 0..depth {
        value = Value::Array(vec![value]);
    }
    value
}

/// A one-member object holding `value` under `keyword`.
fn keyword(keyword: &str, value: Value) -> Value {
    let mut schema = serde_json::Map::new();
    schema.insert(keyword.to_owned(), value);
    Value::Object(schema)
}

#[test]
fn const_enum_and_unique_items_decide_ten_thousand_deep_values() {
    // Built level by level and never passed through `json!`, `clone` or
    // serialization, which recurse per level on their own.
    const DEPTH: usize = 10_000;
    let schemas = [
        keyword("const", nested(DEPTH)),
        keyword("enum", Value::Array(vec![json!(0), nested(DEPTH)])),
    ];
    for document in schemas {
        let schema = compile(document).expect("schema");
        assert!(schema.is_valid(&nested(DEPTH)).expect("evaluation"));
        // A leaf a level shallower, or a different leaf at the same depth,
        // is a different value.
        assert!(!schema.is_valid(&nested(DEPTH - 1)).expect("evaluation"));
        let mut other = json!(2);
        for _ in 0..DEPTH {
            other = Value::Array(vec![other]);
        }
        assert!(!schema.is_valid(&other).expect("evaluation"));
    }

    let unique = compile(json!({"uniqueItems": true})).expect("schema");
    assert!(
        !unique
            .is_valid(&Value::Array(vec![nested(DEPTH), nested(DEPTH)]))
            .expect("evaluation")
    );
    assert!(
        unique
            .is_valid(&Value::Array(vec![nested(DEPTH), nested(DEPTH - 1)]))
            .expect("evaluation")
    );
    // Past sixteen items the check buckets by hash before comparing.
    let mut items: Vec<Value> = (0..20).map(|n| json!(n)).collect();
    items.push(nested(DEPTH));
    assert!(
        unique
            .is_valid(&Value::Array(items.clone()))
            .expect("evaluation")
    );
    items.push(nested(DEPTH));
    assert!(!unique.is_valid(&Value::Array(items)).expect("evaluation"));
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
    assert!(!neighbour.is_valid(&json!({"a": 1})).expect("evaluation"));
    assert!(
        neighbour
            .is_valid(&json!({"a": 1, "b": 2}))
            .expect("evaluation")
    );
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
    let mut registry = registry();
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
    assert!(local.is_valid(&json!("s")).expect("evaluation"));
}

#[test]
fn a_reference_cycle_that_consumes_nothing_fails_instead_of_recursing() {
    let schema = compile(json!({"$ref": "#/$defs/a", "$defs": {"a": {"$ref": "#/$defs/b"}, "b": {"$ref": "#/$defs/a"}}}))
        .expect("a cycle compiles");
    assert!(!schema.is_valid(&json!(1)).expect("evaluation"));
    let output = schema.evaluate(&json!(1)).expect("evaluation");
    assert!(output.errors().any(|unit| {
        unit.error
            .as_deref()
            .is_some_and(|error| error.contains("reference cycle"))
    }));
    let consuming = compile(json!({"$defs": {"tree": {"type": "array", "items": {"$ref": "#/$defs/tree"}}}, "$ref": "#/$defs/tree"}))
        .expect("a consuming cycle compiles");
    assert!(consuming.is_valid(&json!([[[]], []])).expect("evaluation"));
    assert!(!consuming.is_valid(&json!([[1]])).expect("evaluation"));
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

    let invalid = schema
        .evaluate(&json!({"name": 1, "age": -1}))
        .expect("evaluation");
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

    let valid = schema
        .evaluate(&json!({"name": "Ada", "age": 36}))
        .expect("evaluation");
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
fn each_draft_emits_independently_expected_output_locations() {
    for dialect in [DRAFT, DRAFT_2019_09, DRAFT_07] {
        let schema = compile(json!({"$schema": dialect, "type": "string"})).expect("schema");
        let output = schema.evaluate(&json!(2)).expect("evaluation");
        assert_eq!(output.to_json(OutputFormat::Flag), json!({"valid": false}));
        let expected_leaf = json!({
            "valid": false,
            "keywordLocation": "/type",
            "absoluteKeywordLocation": "https://example.org/schema.json#/type",
            "instanceLocation": "",
            "error": "expected string, found integer"
        });
        let basic = output.to_json(OutputFormat::Basic);
        assert_eq!(basic["valid"], false, "{dialect}");
        assert_eq!(basic["keywordLocation"], "", "{dialect}");
        assert_eq!(basic["instanceLocation"], "", "{dialect}");
        assert_eq!(basic["errors"], json!([expected_leaf]), "{dialect}");
        assert_eq!(
            output.to_json(OutputFormat::Detailed),
            expected_leaf,
            "{dialect}"
        );
    }
}

#[test]
fn annotations_of_failed_subschemas_are_dropped() {
    let schema =
        compile(json!({"anyOf": [{"title": "wrong", "type": "string"}, {"title": "right"}]}))
            .expect("compiles");
    let output = schema.evaluate(&json!(1)).expect("evaluation");
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
        schema
            .is_valid(&json!({"a": 1, "b": 1, "c": 1, "d": 1, "e": 1, "f": 1, "g": 1, "h": 1}))
            .expect("evaluation")
    );
    assert!(
        !schema.is_valid(&json!({"z": 1})).expect("evaluation"),
        "`not` contributes no annotations"
    );
    assert!(
        !schema
            .is_valid(&json!({"d": 2, "e": 1}))
            .expect("evaluation"),
        "`then` did not apply"
    );
    assert!(
        !schema.is_valid(&json!({"g": 1})).expect("evaluation"),
        "`dependentSchemas` did not apply"
    );
}
