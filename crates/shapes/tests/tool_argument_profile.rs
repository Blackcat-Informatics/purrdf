// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Public compiler consumer for the worked FnO tool input and output contracts.

use purrdf_shapes::json_schema::{
    CompiledSchema, Namespaces, ToolArgumentProfile, ToolFieldBinding, ToolSchemaError,
    ToolSchemaLimits, compile, compile_tool_arguments,
};

const PREFIXES: &str = r"
@prefix ex: <https://example.org/tools/> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
";

const LIMITS: ToolSchemaLimits = ToolSchemaLimits {
    work: 1_000_000,
    bytes: 64_000_000,
    nodes: 1_000_000,
};

fn namespaces() -> Namespaces {
    Namespaces::new("ex", &[("ex".into(), "https://example.org/tools/".into())]).unwrap()
}

fn profile(root: &str, class: &str, fields: &[&str]) -> ToolArgumentProfile {
    ToolArgumentProfile {
        root_shape_iri: format!("https://example.org/tools/{root}"),
        root_class_iri: format!("https://example.org/tools/{class}"),
        fields: fields
            .iter()
            .map(|field| ToolFieldBinding {
                parent_shape_iri: format!("https://example.org/tools/{root}"),
                predicate_iri: format!("https://example.org/tools/{field}"),
                implementation_property: (*field).to_owned(),
            })
            .collect(),
    }
}

fn metaschemas() -> purrdf_jsonschema::Metaschemas {
    purrdf_jsonschema::Metaschemas::new(
        purrdf_testkit::jsonschema_metaschemas::DRAFT_2020_12
            .iter()
            .map(|&(uri, text)| (uri, purrdf_lex::json::read(text).unwrap())),
    )
    .unwrap()
}

#[expect(
    clippy::result_large_err,
    reason = "exercise the original public compiler error without boxing its native cause"
)]
fn compile_plain(body: &str, fields: &[&str]) -> Result<CompiledSchema, ToolSchemaError> {
    let shapes = purrdf_shapes::engine::parse_shapes(&format!("{PREFIXES}{body}"), None)
        .expect("source shapes");
    compile_tool_arguments(
        &shapes,
        &namespaces(),
        &profile("Args", "Call", fields),
        &LIMITS,
    )
}

const SHAPES: &str = r#"
ex:QueryInput a sh:NodeShape; sh:targetClass ex:QueryArguments; sh:closed true;
 sh:property [sh:path ex:customer_id; sh:datatype xsd:string; sh:pattern "^customer-[0-9]+$"; sh:minCount 1; sh:maxCount 1];
 sh:property [sh:path ex:limit; sh:datatype xsd:integer; sh:minInclusive 1; sh:maxInclusive 100; sh:minCount 1; sh:maxCount 1].
ex:EditInput a sh:NodeShape; sh:targetClass ex:EditArguments; sh:closed true;
 sh:property [sh:path ex:path; sh:datatype xsd:string; sh:minCount 1; sh:maxCount 1];
 sh:property [sh:path ex:text; sh:datatype xsd:string; sh:minCount 1; sh:maxCount 1].
ex:QueryOutput a sh:NodeShape; sh:targetClass ex:QueryResult; sh:closed true;
 sh:property [sh:path ex:orders; sh:datatype xsd:string; sh:minCount 1; sh:maxCount 1].
ex:EditOutput a sh:NodeShape; sh:targetClass ex:EditResult; sh:closed true;
 sh:property [sh:path ex:version; sh:datatype xsd:string; sh:minCount 1; sh:maxCount 1].
"#;

#[test]
fn worked_query_and_file_tools_have_exact_plain_input_and_output_schemas() {
    let shapes = purrdf_shapes::engine::parse_shapes(&format!("{PREFIXES}{SHAPES}"), None)
        .expect("worked source shapes");
    let namespaces = namespaces();
    let original = compile(&shapes, &namespaces).expect("original JSON-LD source");
    let metaschemas = metaschemas();
    for (root, class, fields, expected, valid, invalid) in [
        (
            "QueryInput",
            "QueryArguments",
            &["customer_id", "limit"][..],
            r#"{"type":"object","properties":{"customer_id":{"type":"string","pattern":"^customer\\u{2d}(?:[0-9])+$"},"limit":{"type":"integer","minimum":1,"maximum":100}},"required":["customer_id","limit"],"additionalProperties":false}"#,
            r#"{"customer_id":"customer-17","limit":10}"#,
            r#"{"customer_id":"customer-17","limit":101}"#,
        ),
        (
            "EditInput",
            "EditArguments",
            &["path", "text"][..],
            r#"{"type":"object","properties":{"path":{"type":"string"},"text":{"type":"string"}},"required":["path","text"],"additionalProperties":false}"#,
            r#"{"path":"src/main.rs","text":"replacement"}"#,
            r#"{"path":"src/main.rs"}"#,
        ),
        (
            "QueryOutput",
            "QueryResult",
            &["orders"][..],
            r#"{"type":"object","properties":{"orders":{"type":"string"}},"required":["orders"],"additionalProperties":false}"#,
            r#"{"orders":"order-1"}"#,
            r#"{"orders":["order-1"]}"#,
        ),
        (
            "EditOutput",
            "EditResult",
            &["version"][..],
            r#"{"type":"object","properties":{"version":{"type":"string"}},"required":["version"],"additionalProperties":false}"#,
            r#"{"version":"v2"}"#,
            r#"{"version":"v2","extra":true}"#,
        ),
    ] {
        let compiled =
            compile_tool_arguments(&shapes, &namespaces, &profile(root, class, fields), &LIMITS)
                .expect("worked profile");
        let mut expected = purrdf_lex::json::read(expected).unwrap();
        expected["$schema"] = "https://json-schema.org/draft/2020-12/schema".into();
        expected["$defs"] = purrdf_lex::json::Object::new().into();
        expected.sort_keys();
        assert_eq!(
            compiled.schema_json,
            format!("{}\n", purrdf_lex::json::write_pretty(&expected)),
            "{root}"
        );
        assert_eq!(compiled.losses.entries(), []);
        let schema =
            purrdf_jsonschema::Schema::from_document(&metaschemas, "mem:///tool.json", expected)
                .unwrap();
        assert!(
            schema
                .is_valid(&purrdf_lex::json::read(valid).unwrap())
                .unwrap()
        );
        assert!(
            !schema
                .is_valid(&purrdf_lex::json::read(invalid).unwrap())
                .unwrap()
        );
        if root == "QueryInput" {
            assert!(
                !schema
                    .is_valid(
                        &purrdf_lex::json::read(r#"{"customer_id":"customer-x","limit":10}"#,)
                            .unwrap()
                    )
                    .unwrap()
            );
        }
    }
    let retained = compile(&shapes, &namespaces).expect("original source retained");
    assert_eq!(retained.schema_json, original.schema_json);
    assert_eq!(retained.openapi_json, original.openapi_json);
}

#[test]
fn plain_integer_logical_constraints_keep_exact_membership_and_presence() {
    let compiled = compile_plain(
        r"
ex:Args a sh:NodeShape; sh:targetClass ex:Call; sh:closed true;
 sh:property [sh:path ex:limit; sh:datatype xsd:integer; sh:maxCount 1;
  sh:xone (
   [sh:datatype xsd:integer; sh:minInclusive 1; sh:maxInclusive 3]
   [sh:datatype xsd:integer; sh:minInclusive 3; sh:maxInclusive 5]
  );
  sh:not [sh:datatype xsd:integer; sh:in (2)];
  sh:someValue [sh:datatype xsd:integer; sh:minInclusive 1]].
",
        &["limit"],
    )
    .expect("exact plain integer constraints");
    assert_eq!(compiled.losses.entries(), []);
    let schema = purrdf_jsonschema::Schema::from_document(
        &metaschemas(),
        "mem:///logical.json",
        purrdf_lex::json::read(&compiled.schema_json).unwrap(),
    )
    .unwrap();
    for (instance, expected) in [
        (r#"{"limit":1}"#, true),
        (r#"{"limit":4}"#, true),
        (r"{}", false),
        (r#"{"limit":2}"#, false),
        (r#"{"limit":3}"#, false),
        (r#"{"limit":6}"#, false),
    ] {
        assert_eq!(
            schema
                .is_valid(&purrdf_lex::json::read(instance).unwrap())
                .unwrap(),
            expected,
            "{instance}"
        );
    }
}

#[test]
fn cached_lossy_reference_is_not_treated_as_exact_negation() {
    let compiled = compile_plain(
        r"
ex:Args a sh:NodeShape; sh:targetClass ex:Call; sh:closed true;
 sh:property [sh:path ex:customer_id; sh:node ex:Lossy; sh:maxCount 1];
 sh:property [sh:path ex:limit; sh:datatype xsd:integer; sh:not ex:Lossy; sh:maxCount 1].
ex:Lossy a sh:NodeShape; sh:datatype xsd:integer; sh:equals ex:other.
",
        &["customer_id", "limit"],
    )
    .expect("losses remain available to the original SHACL validator");
    for (code, subject) in [
        ("sh:equals", "<https://example.org/tools/Lossy>"),
        ("sh:not", "<https://example.org/tools/Args>"),
    ] {
        assert!(
            compiled.losses.entries().iter().any(|entry| {
                entry.code == code
                    && entry
                        .location
                        .as_deref()
                        .and_then(|location| location.subject.as_deref())
                        == Some(subject)
            }),
            "missing {code} loss at {subject}"
        );
    }
    let schema = purrdf_jsonschema::Schema::from_document(
        &metaschemas(),
        "mem:///cached-loss.json",
        purrdf_lex::json::read(&compiled.schema_json).unwrap(),
    )
    .unwrap();
    assert!(
        schema
            .is_valid(&purrdf_lex::json::read(r#"{"customer_id":1,"limit":1}"#).unwrap())
            .unwrap()
    );
}

#[test]
fn plain_arguments_validate_scalar_nested_array_cardinality_and_closedness() {
    let body = r"
ex:Args a sh:NodeShape; sh:targetClass ex:Call; sh:closed true;
 sh:property [sh:path ex:customer; sh:datatype xsd:string; sh:maxCount 1; sh:minLength 2; sh:maxLength 18446744073709551615];
 sh:property [sh:path ex:customer; sh:minCount 1];
 sh:property [sh:path ex:limit; sh:datatype xsd:integer; sh:maxCount 1; sh:minInclusive 0; sh:maxInclusive 10];
 sh:property [sh:path ex:children; sh:class ex:ChildClass; sh:minCount 1; sh:maxCount 2].
ex:Inactive a sh:NodeShape; sh:targetClass ex:ChildClass; sh:deactivated true.
ex:Child a sh:NodeShape; sh:targetClass ex:ChildClass; sh:closed true;
 sh:property [sh:path ex:name; sh:datatype xsd:string; sh:minCount 1; sh:maxCount 1].
";
    let shapes = purrdf_shapes::engine::parse_shapes(&format!("{PREFIXES}{body}"), None).unwrap();
    let mut profile = profile("Args", "Call", &["customer", "limit", "children"]);
    profile.fields[0].implementation_property = "customer_id".into();
    profile.fields.push(ToolFieldBinding {
        parent_shape_iri: "https://example.org/tools/Child".into(),
        predicate_iri: "https://example.org/tools/name".into(),
        implementation_property: "name".into(),
    });
    let compiled = compile_tool_arguments(&shapes, &namespaces(), &profile, &LIMITS).unwrap();
    let mut expected = purrdf_lex::json::read(r##"{
        "$schema":"https://json-schema.org/draft/2020-12/schema",
        "$defs":{"ShapeRef0":{"type":"object","properties":{"name":{"type":"string"}},"required":["name"],"additionalProperties":false}},
        "type":"object","properties":{
            "customer_id":{"type":"string","minLength":2,"maxLength":18446744073709551615},
            "limit":{"type":"integer","minimum":0,"maximum":10},
            "children":{"type":"array","items":{"$ref":"#/$defs/ShapeRef0","type":["string","integer","boolean","object"]},"minItems":1,"maxItems":2}
        },"required":["children","customer_id"],"additionalProperties":false
    }"##).unwrap();
    expected.sort_keys();
    assert_eq!(
        compiled.schema_json,
        format!("{}\n", purrdf_lex::json::write_pretty(&expected))
    );
    assert_eq!(compiled.losses.entries(), []);
    let schema =
        purrdf_jsonschema::Schema::from_document(&metaschemas(), "mem:///nested.json", expected)
            .unwrap();
    for valid in [
        r#"{"customer_id":"customer-17","children":[{"name":"Ada"}]}"#,
        r#"{"customer_id":"ab","limit":10,"children":[{"name":"Ada"},{"name":"Max"}]}"#,
    ] {
        assert!(
            schema
                .is_valid(&purrdf_lex::json::read(valid).unwrap())
                .unwrap(),
            "{valid}"
        );
    }
    for invalid in [
        r#"{"children":[{"name":"Ada"}]}"#,
        r#"{"customer_id":"a","children":[{"name":"Ada"}]}"#,
        r#"{"customer_id":"ab","limit":11,"children":[{"name":"Ada"}]}"#,
        r#"{"customer_id":"ab","children":{"name":"Ada"}}"#,
        r#"{"customer_id":"ab","children":[]}"#,
        r#"{"customer_id":"ab","children":[{}]}"#,
        r#"{"customer_id":"ab","children":[{"name":"Ada","extra":1}]}"#,
        r#"{"customer_id":"ab","children":[{"name":"Ada"}],"@id":"x"}"#,
    ] {
        assert!(
            !schema
                .is_valid(&purrdf_lex::json::read(invalid).unwrap())
                .unwrap(),
            "{invalid}"
        );
    }
}

#[test]
fn boolean_pattern_matcher_refusal_retains_native_cause_and_source() {
    #[expect(
        clippy::result_large_err,
        reason = "assert the original public compiler error and native cause"
    )]
    let compile_renamed = |body: &str| {
        let shapes = purrdf_shapes::engine::parse_shapes(&format!("{PREFIXES}{body}"), None)
            .expect("source shapes");
        let mut profile = profile("Args", "Call", &["flag"]);
        profile.fields[0].implementation_property = "enabled".to_owned();
        compile_tool_arguments(&shapes, &namespaces(), &profile, &LIMITS)
    };
    let error = compile_renamed(
        r#"
ex:Args a sh:NodeShape; sh:targetClass ex:Call;
 sh:property [sh:path ex:flag; sh:datatype xsd:boolean; sh:maxCount 1; sh:pattern "a{1000000}"].
"#,
    )
    .expect_err("native pattern refusal cannot silently produce a schema");
    assert!(matches!(
        error,
        ToolSchemaError::Pattern {
            shape,
            property,
            predicate,
            cause: purrdf_core::xsd_regex::Ecma262Error::Source(_),
        } if shape.contains("https://example.org/tools/Args") && property == "enabled"
            && predicate.as_deref() == Some("https://example.org/tools/flag")
    ));

    let oversized = "a".repeat(purrdf_core::xsd_regex::MAX_SOURCE_BYTES + 1);
    let error = compile_renamed(&format!(
        r#"
ex:Args a sh:NodeShape; sh:targetClass ex:Call;
 sh:property [sh:path ex:flag; sh:datatype xsd:boolean; sh:maxCount 1; sh:not ex:Pattern].
ex:Pattern a sh:NodeShape; sh:datatype xsd:boolean; sh:pattern "{oversized}".
"#
    ))
    .expect_err("a proof refusal retains the referenced source shape");
    assert!(matches!(
        error,
        ToolSchemaError::Pattern {
            shape,
            property,
            predicate,
            cause: purrdf_core::xsd_regex::Ecma262Error::Source(_),
        } if shape.contains("https://example.org/tools/Pattern") && property == "enabled"
            && predicate.as_deref() == Some("https://example.org/tools/flag")
    ));

    let error = compile_renamed(
        "ex:Args a sh:NodeShape; sh:targetClass ex:Call; sh:property [sh:path ex:flag; sh:datatype xsd:anyURI; sh:maxCount 1].",
    ).expect_err("unsupported RDF carriers retain their original predicate");
    assert!(
        matches!(error, ToolSchemaError::UnsupportedConstruct { location, predicate, .. }
        if location.contains("enabled") && predicate.as_deref() == Some("https://example.org/tools/flag"))
    );
    let error = compile_renamed(
        "ex:Args a sh:NodeShape; sh:targetClass ex:Call; sh:property [sh:path ex:flag; sh:datatype xsd:boolean; sh:maxCount 1; sh:defaultValue true].",
    ).expect_err("property metadata refusal retains the direct RDF predicate");
    assert!(
        matches!(error, ToolSchemaError::UnsupportedConstruct { predicate, .. }
        if predicate.as_deref() == Some("https://example.org/tools/flag"))
    );
    for constraint in [
        "sh:in (\"not-an-integer\"^^xsd:integer)",
        "sh:hasValue \"not-an-integer\"^^xsd:integer",
    ] {
        let error = compile_renamed(&format!(
            "ex:Args a sh:NodeShape; sh:targetClass ex:Call; sh:property [sh:path ex:flag; sh:maxCount 1; sh:node ex:Scalar]. ex:Scalar a sh:NodeShape; sh:datatype xsd:integer; {constraint}."
        )).expect_err("nested constants are refused before enum emission");
        assert!(
            matches!(error, ToolSchemaError::UnsupportedConstruct { location, predicate, .. }
            if location.contains("https://example.org/tools/Scalar") && predicate.as_deref() == Some("https://example.org/tools/flag"))
        );
    }
}
