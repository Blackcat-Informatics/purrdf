// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! "Reject malformed, never panic" property gate (T7) for the purrdf-shapes
//! shapes frontend.
//!
//! `engine::parse_shapes` parses untrusted SHACL Turtle; given arbitrary input it
//! must return `Ok`/`Err`, never panic. Inputs are bounded so a superlinear
//! parse cannot become a spurious timeout. See `crates/rdf/tests/never_panic.rs`
//! for the contract rationale.

use proptest::prelude::*;
use purrdf_shapes::engine::parse_shapes;
use purrdf_shapes::json_schema::Namespaces;
use purrdf_shapes::{
    LinkmlDocument, SchemaDatatypeMap, SchemaImportConfig, import_json_schema, import_linkml,
    parse_linkml,
};
use serde_json::{Map, Number, Value};

fn arbitrary_bytes() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(any::<u8>(), 0..4096)
}

const NODE_EXPR_PREFIXES: &str = "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
     @prefix ex: <http://example.org/ns#> .\n";

/// A cyclic SHACL-AF node expression must be a hard `Err`, whatever node kind
/// spells the cycle.
///
/// Rust stack exhaustion ABORTS the process and is not catchable, so an
/// unguarded node-expression cycle is not a "panic" this suite could observe —
/// the test binary reaching its own assertions is the proof that the parse
/// terminated. Both cases below are reachable from ordinary Turtle: a named
/// node referring to itself through `sh:union`, and a labelled blank node doing
/// the same.
#[test]
fn cyclic_node_expression_graph_is_hard_error() {
    let named = parse_shapes(&format!(
        "{NODE_EXPR_PREFIXES}\
         ex:S a sh:NodeShape ; sh:targetClass ex:C ; sh:expression ex:E .\n\
         ex:E sh:union ( ex:E ) .\n"
    ))
    .expect_err("a named-node expression cycle must be rejected");
    assert!(
        named.contains("cyclic"),
        "error must name the cycle, got: {named}"
    );

    let blank = parse_shapes(&format!(
        "{NODE_EXPR_PREFIXES}\
         ex:S a sh:NodeShape ; sh:targetClass ex:C ; sh:expression _:e .\n\
         _:e sh:union ( _:e ) .\n"
    ))
    .expect_err("a blank-node expression cycle must be rejected");
    assert!(
        blank.contains("cyclic"),
        "error must name the cycle, got: {blank}"
    );
}

/// The cycle guard is a STACK of in-flight parses, not a visited set: a node
/// expression DAG that reaches the same named sub-expression through two
/// disjoint branches is not a cycle and must still parse.
///
/// This is the anti-regression guard for extending the guard beyond blank
/// nodes — a visited-set implementation would wrongly reject this document.
#[test]
fn shared_named_sub_expression_is_not_a_cycle() {
    let shapes = parse_shapes(&format!(
        "{NODE_EXPR_PREFIXES}\
         ex:S a sh:NodeShape ; sh:targetClass ex:C ; sh:expression ex:Top .\n\
         ex:Top sh:union ( ex:Left ex:Right ) .\n\
         ex:Left sh:union ( ex:Shared ) .\n\
         ex:Right sh:union ( ex:Shared ) .\n\
         ex:Shared sh:union ( ex:a ) .\n"
    ))
    .expect("a shared sub-expression reached twice is a DAG, not a cycle");
    assert_eq!(shapes.node_shapes.len(), 1);
}

/// Structure-aware SHACL Turtle: real `sh:` shape fragments interleaved with
/// noise, to reach the shape-graph interpreter, not just the Turtle lexer.
fn structured_shapes() -> impl Strategy<Value = String> {
    let fragments: Vec<&'static str> = vec![
        "@prefix sh: <http://www.w3.org/ns/shacl#> .\n",
        "@prefix ex: <https://example.org/> .\n",
        "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n",
        "ex:S a sh:NodeShape ; sh:targetClass ex:C .\n",
        "ex:S sh:property [ sh:path ex:p ; sh:minCount 1 ] .\n",
        "ex:S sh:property [ sh:path ex:p ; sh:datatype xsd:string ] .\n",
        "ex:S sh:property [ sh:path ex:p ; sh:pattern \"^a+$\" ] .\n",
        "ex:S sh:property [ sh:path ex:p ; sh:minCount \"notanint\" ] .\n",
        "ex:S sh:node ex:S .\n",
        "ex:S sh:property [ sh:path [ sh:inversePath ex:p ] ] .\n",
        "\u{0}\u{1}",
        "ex:S a sh:NodeShape ; sh:property",
        "@prefix sh:",
    ];
    prop::collection::vec(prop::sample::select(fragments), 0..24).prop_map(|parts| parts.concat())
}

fn arbitrary_json() -> impl Strategy<Value = Value> {
    let leaf = prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::Bool),
        any::<i64>().prop_map(|value| Value::Number(Number::from(value))),
        ".{0,64}".prop_map(Value::String),
    ];
    leaf.prop_recursive(5, 128, 8, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..8).prop_map(Value::Array),
            prop::collection::btree_map(".{0,24}", inner, 0..8).prop_map(|entries| {
                Value::Object(entries.into_iter().collect::<Map<String, Value>>())
            }),
        ]
    })
}

fn schema_import_config() -> SchemaImportConfig {
    const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
    let namespaces = Namespaces::new(
        "ex",
        &[("ex".to_owned(), "https://example.org/".to_owned())],
    )
    .expect("valid test namespace");
    let datatypes = SchemaDatatypeMap::new(
        format!("{XSD}string"),
        format!("{XSD}boolean"),
        format!("{XSD}integer"),
        format!("{XSD}decimal"),
        format!("{XSD}dateTime"),
        format!("{XSD}date"),
        format!("{XSD}time"),
        format!("{XSD}anyURI"),
    )
    .expect("valid test datatypes");
    SchemaImportConfig::new(namespaces, datatypes)
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, ..ProptestConfig::default() })]

    #[test]
    fn parse_shapes_never_panics_raw(data in arbitrary_bytes()) {
        if let Ok(text) = std::str::from_utf8(&data) {
            let _ = parse_shapes(text);
        }
    }

    #[test]
    fn parse_shapes_never_panics_structured(text in structured_shapes()) {
        let _ = parse_shapes(&text);
    }

    #[test]
    fn import_json_schema_never_panics(value in arbitrary_json()) {
        let input = serde_json::to_string(&value).expect("JSON value serializes");
        let _ = import_json_schema(&input, &schema_import_config());
    }

    #[test]
    fn parse_linkml_never_panics(data in arbitrary_bytes()) {
        if let Ok(text) = std::str::from_utf8(&data) {
            let _ = parse_linkml(text);
        }
    }

    #[test]
    fn import_linkml_never_panics(section in 0_usize..4, value in arbitrary_json()) {
        let mut document = serde_json::json!({
            "id": "https://example.org/schema/linkml",
            "name": "Generated-Schema",
            "metamodel_version": "1.11.0",
            "prefixes": {
                "ex": "https://example.org/",
                "linkml": "https://w3id.org/linkml/"
            },
            "default_prefix": "ex",
            "classes": {},
            "enums": {},
            "slots": {},
            "types": {}
        });
        let section_name = ["classes", "enums", "slots", "types"][section];
        document[section_name]["Generated"] = value;
        if let Ok(document) = LinkmlDocument::from_value(document) {
            let _ = import_linkml(&document, &schema_import_config());
        }
    }
}
