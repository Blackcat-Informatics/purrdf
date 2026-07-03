// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Unit tests for result-shape-map JSON output (`ResultShapeMap::to_result_json`):
//! the array shape, `node`/`shape`/`status`/`reason` fields, term syntax for
//! IRIs and literals, and deterministic output.

use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, TermValue};
use purrdf_shex::{parse_shexc, validate, ShapeSelector};

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
