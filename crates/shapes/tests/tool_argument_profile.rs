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

/// Decimal boundaries and same-domain logical predicates must not round through IEEE.
#[test]
fn decimal_boundaries_and_logical_membership_remain_exact() {
    let compiled = compile_plain(
        r#"
ex:Args a sh:NodeShape; sh:targetClass ex:Call; sh:closed true;
 sh:property [sh:path ex:amount; sh:datatype xsd:decimal; sh:nodeKind sh:Literal; sh:minCount 1; sh:maxCount 1;
  sh:minInclusive "9007199254740993.0000000000000001"^^xsd:decimal;
  sh:maxExclusive "9007199254740993.0000000000000003"^^xsd:decimal;
  sh:not [sh:datatype xsd:decimal; sh:in ("9007199254740993.0000000000000002"^^xsd:decimal)];
  sh:someValue [sh:datatype xsd:decimal; sh:minInclusive 1]].
"#,
        &["amount"],
    )
    .expect("exact decimal profile");
    assert!(compiled.losses.is_empty());
    let schema = purrdf_jsonschema::Schema::from_document(
        &metaschemas(),
        "mem:///decimal.json",
        purrdf_lex::json::read(&compiled.schema_json).unwrap(),
    )
    .unwrap();
    for (value, expected) in [
        ("9007199254740993.0000000000000001", true),
        ("9007199254740993.0000000000000002", false),
        ("9007199254740993.0000000000000003", false),
        ("9007199254740993", false),
    ] {
        assert_eq!(
            schema
                .is_valid(&purrdf_lex::json::read(&format!("{{\"amount\":{value}}}")).unwrap())
                .unwrap(),
            expected
        );
    }
}

/// Finite IEEE enums prove their own domain; unrestricted numbers retain a located check.
#[test]
fn finite_ieee_constants_are_exact_and_unrestricted_domains_are_ledgered() {
    for (datatype, exact) in [
        ("float", "0.100000001490116119384765625"),
        (
            "double",
            "0.1000000000000000055511151231257827021181583404541015625",
        ),
    ] {
        let body = format!(
            r#"
ex:Args a sh:NodeShape; sh:targetClass ex:Call;
 sh:property [sh:path ex:amount; sh:datatype xsd:{datatype}; sh:nodeKind sh:Literal; sh:minCount 1; sh:maxCount 1;
  sh:in ("1.0E-1"^^xsd:{datatype} "5.0E-1"^^xsd:{datatype}); sh:minInclusive 0.1].
"#
        );
        let compiled = compile_plain(&body, &["amount"]).expect("finite IEEE profile");
        assert!(
            compiled.losses.is_empty(),
            "{datatype}: {:?}",
            compiled.losses
        );
        let schema = purrdf_jsonschema::Schema::from_document(
            &metaschemas(),
            "mem:///ieee.json",
            purrdf_lex::json::read(&compiled.schema_json).unwrap(),
        )
        .unwrap();
        for (value, expected) in [(exact, true), ("0.5", true), ("0.1", false)] {
            assert_eq!(
                schema
                    .is_valid(&purrdf_lex::json::read(&format!("{{\"amount\":{value}}}")).unwrap())
                    .unwrap(),
                expected,
                "{datatype}: {value}"
            );
        }
        let body = format!(
            "ex:Args a sh:NodeShape; sh:targetClass ex:Call; sh:property [sh:path ex:amount; sh:datatype xsd:{datatype}; sh:maxCount 1; sh:minInclusive 0.1]."
        );
        let compiled =
            compile_plain(&body, &["amount"]).expect("located original-SHACL obligation");
        let [loss] = compiled.losses.entries() else {
            panic!("{:?}", compiled.losses);
        };
        assert_eq!(loss.code, "plain:ieee-representability");
        assert_eq!(
            loss.location.as_ref().unwrap().subject.as_deref(),
            Some("<https://example.org/tools/Args>")
        );
        assert!(compiled.schema_json.contains("original SHACL validation"));
    }
}

/// A refusal must distinguish invalid/nonfinite/sign/identity failures and retain source.
#[test]
fn numeric_refusals_have_canonical_valid_neighbors_and_original_predicates() {
    use purrdf_shapes::json_schema::ToolRepresentationError;
    for (datatype, lexical, reason, valid) in [
        (
            "decimal",
            "1e0",
            ToolRepresentationError::NumericConstant,
            "1",
        ),
        (
            "double",
            "INF",
            ToolRepresentationError::NumericConstant,
            "1.0E0",
        ),
        (
            "float",
            "NaN",
            ToolRepresentationError::NumericConstant,
            "1.0E0",
        ),
        (
            "float",
            "-0.0E0",
            ToolRepresentationError::SignedZero,
            "0.0E0",
        ),
        (
            "decimal",
            "1.0",
            ToolRepresentationError::NumericIdentity,
            "1",
        ),
    ] {
        let body = |text| {
            format!(
                "ex:Args a sh:NodeShape; sh:targetClass ex:Call; sh:property [sh:path ex:amount; sh:datatype xsd:{datatype}; sh:maxCount 1; sh:in (\"{text}\"^^xsd:{datatype})]."
            )
        };
        let error = compile_plain(&body(lexical), &["amount"]).expect_err("source-bearing refusal");
        assert!(
            matches!(error, ToolSchemaError::UnsupportedConstruct { location, predicate, reason: actual }
            if location.contains("Args") && predicate.as_deref() == Some("https://example.org/tools/amount") && actual == reason)
        );
        let compiled = compile_plain(&body(valid), &["amount"]).expect("canonical finite neighbor");
        assert!(compiled.losses.is_empty());
    }
}

/// Residual datatype checks cannot become an exact negand through a cached reference.
#[test]
fn ieee_residual_references_refuse_exact_logic_and_decimal_neighbors_work() {
    use purrdf_shapes::json_schema::ToolRepresentationError;
    let body = r"
ex:Args a sh:NodeShape; sh:targetClass ex:Call;
 sh:property [sh:path ex:amount; sh:datatype xsd:double; sh:maxCount 1; sh:node ex:Numeric; sh:not ex:Numeric].
ex:Numeric a sh:NodeShape; sh:datatype xsd:double; sh:minInclusive 1.
";
    let error = compile_plain(body, &["amount"])
        .expect_err("an approximate cached definition cannot be negated");
    assert!(
        matches!(error, ToolSchemaError::UnsupportedConstruct { location, predicate, reason: ToolRepresentationError::NumericLogic }
        if location.contains("Numeric") && predicate.as_deref() == Some("https://example.org/tools/amount"))
    );
    let compiled = compile_plain(&body.replace("xsd:double", "xsd:decimal"), &["amount"])
        .expect("exact decimal negation remains supported");
    assert!(compiled.losses.is_empty());
    // The same target under unknown and integer scopes must not share a
    // datatype-incompatible definition, whichever field compiles first.
    for (plain, integer) in [("a", "b"), ("b", "a")] {
        let body = format!(
            r"
ex:Args a sh:NodeShape; sh:targetClass ex:Call;
 sh:property [sh:path ex:{plain}; sh:maxCount 1; sh:nodeKind sh:Literal; sh:node ex:Numeric];
 sh:property [sh:path ex:{integer}; sh:maxCount 1; sh:datatype xsd:integer; sh:node ex:Numeric].
ex:Numeric a sh:NodeShape; sh:datatype xsd:decimal.
"
        );
        let compiled =
            compile_plain(&body, &["a", "b"]).expect("scope-specific numeric references");
        assert!(compiled.losses.is_empty());
        let schema = purrdf_jsonschema::Schema::from_document(
            &metaschemas(),
            "mem:///numeric-reference.json",
            purrdf_lex::json::read(&compiled.schema_json).unwrap(),
        )
        .unwrap();
        for (instance, expected) in [
            (format!("{{\"{plain}\":1.5}}"), true),
            (format!("{{\"{plain}\":\"1.5\"}}"), false),
            (format!("{{\"{integer}\":1}}"), false),
        ] {
            assert_eq!(
                schema
                    .is_valid(&purrdf_lex::json::read(&instance).unwrap())
                    .unwrap(),
                expected,
                "{instance}"
            );
        }
    }
    let finite = compile_plain(
        r#"
ex:Args a sh:NodeShape; sh:targetClass ex:Call;
 sh:property [sh:path ex:amount; sh:maxCount 1; sh:in ("1.5"^^xsd:decimal)].
"#,
        &["amount"],
    )
    .expect("homogeneous finite decimal domain selects its carrier");
    assert!(finite.losses.is_empty());
    let ambiguous = compile_plain(
        r#"
ex:Args a sh:NodeShape; sh:targetClass ex:Call;
 sh:property [sh:path ex:amount; sh:maxCount 1; sh:in (1 "1"^^xsd:decimal)].
"#,
        &["amount"],
    )
    .expect_err("different numeric RDF tags must not collapse");
    assert!(matches!(
        ambiguous,
        ToolSchemaError::UnsupportedConstruct {
            reason: ToolRepresentationError::NumericDomain,
            ..
        }
    ));
}

/// The first subnormal's promotion threshold is a half-subnormal, not representable as f64.
#[test]
fn decimal_ieee_midpoint_and_native_ceilings_preserve_exact_boundaries() {
    use purrdf_testkit::exact::{Natural, Rational};
    let body = r#"
ex:Args a sh:NodeShape; sh:targetClass ex:Call;
 sh:property [sh:path ex:amount; sh:datatype xsd:decimal; sh:maxCount 1;
  sh:minInclusive "4.9406564584124654E-324"^^xsd:double].
"#;
    let shapes = purrdf_shapes::engine::parse_shapes(&format!("{PREFIXES}{body}"), None).unwrap();
    let profile = profile("Args", "Call", &["amount"]);
    let compiled = compile_tool_arguments(&shapes, &namespaces(), &profile, &LIMITS)
        .expect("native admitted midpoint");
    let value = purrdf_lex::json::read(&compiled.schema_json).unwrap();
    let midpoint = value["properties"]["amount"]["exclusiveMinimum"]
        .as_number()
        .expect("odd first subnormal needs a strict midpoint");
    let oracle = Rational::new(
        false,
        Natural::from_u128(1),
        Natural::from_u128(1).shl(1075),
    );
    assert!(
        Rational::parse(midpoint.lexeme())
            .unwrap()
            .value_eq(&oracle)
    );
    for resource in ["work", "bytes", "nodes"] {
        let mut tiny = LIMITS;
        match resource {
            "work" => tiny.work = 0,
            "bytes" => tiny.bytes = 0,
            _ => tiny.nodes = 0,
        }
        assert!(
            matches!(compile_tool_arguments(&shapes, &namespaces(), &profile, &tiny),
            Err(ToolSchemaError::Exhausted { resource: actual, .. }) if actual == resource)
        );
    }
    let repeated = compile_tool_arguments(&shapes, &namespaces(), &profile, &LIMITS)
        .expect("valid ceiling neighbor");
    assert_eq!(repeated.schema_json, compiled.schema_json);
}
