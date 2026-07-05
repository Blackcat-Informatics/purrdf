// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Unit tests for result-shape-map JSON output (`ResultShapeMap::to_result_json`):
//! the array shape, `node`/`shape`/`status`/`reason` fields, term syntax for
//! IRIs and literals, and deterministic output.

use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, TermValue};
use purrdf_shex::{
    ConformanceStatus, NodeSelector, ResultEntry, ResultShapeMap, ShapeSelector, parse_shape_map,
    parse_shexc, validate,
};

const S: &str = "http://a.example/S";

/// s1 <p1> o1 (IRI object) ; s2 <p1> "x" (literal object)
fn data() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let s1 = b.intern_iri("http://a.example/s1");
    let s2 = b.intern_iri("http://a.example/s2");
    let p1 = b.intern_iri("http://a.example/p1");
    let o1 = b.intern_iri("http://a.example/o1");
    let x = b.intern_literal(purrdf_core::RdfLiteral::simple("x"));
    b.push_quad(s1, p1, o1, None);
    b.push_quad(s2, p1, x, None);
    b.freeze().expect("freeze")
}

#[test]
fn array_fields_and_status() {
    let schema = parse_shexc("<http://a.example/S> { <http://a.example/p1> IRI }", None)
        .expect("schema parses");
    let data = data();
    let map = [
        (
            TermValue::iri("http://a.example/s1"),
            ShapeSelector::Label(S.to_owned()),
        ),
        (
            TermValue::iri("http://a.example/s2"),
            ShapeSelector::Label(S.to_owned()),
        ),
    ];
    let result = validate(&schema, &data, &map);
    let json: serde_json::Value =
        serde_json::from_str(&result.to_result_json()).expect("valid JSON");
    let rows = json.as_array().expect("array");
    assert_eq!(rows.len(), 2);

    // s1 conforms (IRI object); no reason field.
    assert_eq!(rows[0]["node"], "<http://a.example/s1>");
    assert_eq!(rows[0]["shape"], format!("<{S}>"));
    assert_eq!(rows[0]["status"], "conformant");
    assert!(rows[0].get("reason").is_none());

    // s2 fails (literal object where IRI required); reason present.
    assert_eq!(rows[1]["node"], "<http://a.example/s2>");
    assert_eq!(rows[1]["status"], "nonconformant");
    assert!(rows[1]["reason"].is_string());
}

#[test]
fn output_is_deterministic() {
    let schema = parse_shexc("<http://a.example/S> {}", None).expect("schema parses");
    let data = data();
    let map = [(
        TermValue::iri("http://a.example/s1"),
        ShapeSelector::Label(S.to_owned()),
    )];
    let result = validate(&schema, &data, &map);
    assert_eq!(result.to_result_json(), result.to_result_json());
}

#[test]
fn literal_and_start_term_syntax() {
    // Empty open shape conforms for any (even detached) focus.
    let schema = parse_shexc(
        "start = @<http://a.example/S>\n<http://a.example/S> {}",
        None,
    )
    .expect("schema parses");
    let data = data();
    let map = [
        (TermValue::lang_literal("hi", "EN"), ShapeSelector::Start),
        (
            TermValue::typed_literal("5", "http://www.w3.org/2001/XMLSchema#integer"),
            ShapeSelector::Start,
        ),
        (TermValue::simple_literal("a\"b"), ShapeSelector::Start),
    ];
    let result = validate(&schema, &data, &map);
    let json: serde_json::Value =
        serde_json::from_str(&result.to_result_json()).expect("valid JSON");
    let rows = json.as_array().expect("array");
    assert_eq!(rows[0]["node"], "\"hi\"@en");
    assert_eq!(rows[0]["shape"], "START");
    assert_eq!(
        rows[1]["node"],
        "\"5\"^^<http://www.w3.org/2001/XMLSchema#integer>"
    );
    // The escaped quote survives the Turtle+JSON round trip.
    assert_eq!(rows[2]["node"], "\"a\\\"b\"");
}

// ── round-trip: to_result_json() -> parse_shape_map() recovers the node ────

/// The node-term string an entry with `node` renders to, extracted from
/// `to_result_json` (the emitter under test).
fn emitted_node_term(node: &TermValue) -> String {
    let result = ResultShapeMap {
        entries: vec![ResultEntry {
            node: node.clone(),
            shape: ShapeSelector::Start,
            status: ConformanceStatus::Conformant,
            reason: None,
        }],
    };
    let json: serde_json::Value =
        serde_json::from_str(&result.to_result_json()).expect("valid JSON");
    json[0]["node"]
        .as_str()
        .expect("node is a string")
        .to_owned()
}

/// Round-trips `node` through `to_result_json` then `parse_shape_map`,
/// asserting the recovered term equals the original — the docstring's
/// "round-trippable with `parse_shape_map`" claim, exercised end to end.
fn round_trip(node: &TermValue) -> TermValue {
    let term_str = emitted_node_term(node);
    // A space before '@START' disambiguates a bare literal's closing quote
    // from a `"lit"@lang`-style language tag: `parse_shape_map` tolerates
    // whitespace between the node selector and the shape's leading '@'.
    let map = parse_shape_map(&format!("{term_str} @START"), None)
        .unwrap_or_else(|e| panic!("re-parsing emitted term {term_str:?} failed: {e}"));
    match &map.0[0].node {
        NodeSelector::Node(value) => value.clone(),
        other => panic!("expected a concrete node, got {other:?}"),
    }
}

#[test]
fn round_trips_iri_and_blank() {
    let iri = TermValue::iri("http://a.example/s1");
    assert_eq!(round_trip(&iri), iri);
    let blank = TermValue::blank("b1");
    assert_eq!(round_trip(&blank), blank);
}

#[test]
fn round_trips_literal_forms() {
    let plain = TermValue::simple_literal("hello");
    assert_eq!(round_trip(&plain), plain);
    let typed = TermValue::typed_literal("5", "http://www.w3.org/2001/XMLSchema#integer");
    assert_eq!(round_trip(&typed), typed);
    let tagged = TermValue::lang_literal("hi", "en");
    assert_eq!(round_trip(&tagged), tagged);
}

#[test]
fn round_trips_literal_escapes_including_backspace_and_form_feed() {
    // Quote, backslash, newline, carriage return, tab, backspace (U+0008),
    // form feed (U+000C) — every escape `turtle_escape` must emit and
    // `parse_shape_map` must parse back.
    let lexical = "a\"b\\c\nd\re\tf\u{8}g\u{c}h";
    let literal = TermValue::simple_literal(lexical);
    assert_eq!(round_trip(&literal), literal);
}

#[test]
fn round_trips_quoted_triple_term() {
    let triple = TermValue::Triple {
        s: Box::new(TermValue::iri("http://a.example/s")),
        p: Box::new(TermValue::iri("http://a.example/p")),
        o: Box::new(TermValue::iri("http://a.example/o")),
    };
    assert_eq!(round_trip(&triple), triple);
}

#[test]
fn round_trips_nested_quoted_triple_term() {
    let inner = TermValue::Triple {
        s: Box::new(TermValue::iri("http://a.example/s")),
        p: Box::new(TermValue::iri("http://a.example/p")),
        o: Box::new(TermValue::iri("http://a.example/o")),
    };
    let outer = TermValue::Triple {
        s: Box::new(inner),
        p: Box::new(TermValue::iri("http://a.example/p2")),
        o: Box::new(TermValue::iri("http://a.example/o2")),
    };
    assert_eq!(round_trip(&outer), outer);
}

#[test]
fn round_trips_quoted_triple_with_escaped_literal_object() {
    let triple = TermValue::Triple {
        s: Box::new(TermValue::iri("http://a.example/s")),
        p: Box::new(TermValue::iri("http://a.example/p")),
        o: Box::new(TermValue::simple_literal(
            "line1\nline2\u{8}\u{c}\"quoted\"",
        )),
    };
    assert_eq!(round_trip(&triple), triple);
}

#[test]
fn reason_strings_with_quotes_and_newlines_stay_valid_json() {
    let schema = parse_shexc("<http://a.example/S> { <http://a.example/p1> IRI }", None)
        .expect("schema parses");
    let data = data();
    // s2's object is a literal, not an IRI, so this is nonconformant and
    // carries a reason string.
    let map = [(
        TermValue::iri("http://a.example/s2"),
        ShapeSelector::Label(S.to_owned()),
    )];
    let result = validate(&schema, &data, &map);
    let rendered = result.to_result_json();
    let json: serde_json::Value = serde_json::from_str(&rendered).expect("valid JSON");
    let reason = json[0]["reason"].as_str().expect("reason present");
    assert!(!reason.is_empty());

    // Force a reason containing a quote and a newline directly, bypassing
    // the matcher, to prove the JSON layer escapes arbitrary reason text.
    let result = ResultShapeMap {
        entries: vec![ResultEntry {
            node: TermValue::iri("http://a.example/s1"),
            shape: ShapeSelector::Start,
            status: ConformanceStatus::Nonconformant,
            reason: Some("bad \"value\"\nline two".to_owned()),
        }],
    };
    let rendered = result.to_result_json();
    let json: serde_json::Value = serde_json::from_str(&rendered).expect("valid JSON");
    assert_eq!(json[0]["reason"], "bad \"value\"\nline two");
}
