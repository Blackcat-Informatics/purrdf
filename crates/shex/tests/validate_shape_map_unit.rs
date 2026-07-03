// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Unit tests for [`validate_shape_map`], the one-call
//! `parse_shape_map` → `resolve_shape_map` → `validate_with` → `ResultShapeMap`
//! entry point.

use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, TermValue};
use purrdf_shex::{
    parse_shexc, validate_shape_map, ConformanceStatus, ShapeSelector, ValidationOptions,
};

const P1: &str = "http://a.example/p1";
const S: &str = "http://a.example/S";

/// s1 <p1> o1 ; s1 <p1> o3 (two arcs — over the schema's cardinality of 1) ;
/// s2 <p1> o2 (exactly one arc — conforms).
fn data() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let mut arc = |s: &str, p: &str, o: &str| {
        let s = b.intern_iri(s);
        let p = b.intern_iri(p);
        let o = b.intern_iri(o);
        b.push_quad(s, p, o, None);
    };
    arc("http://a.example/s1", P1, "http://a.example/o1");
    arc("http://a.example/s1", P1, "http://a.example/o3");
    arc("http://a.example/s2", P1, "http://a.example/o2");
    b.freeze().expect("freeze")
}

/// `<S> { <p1> IRI }` — exactly one `p1` arc with an IRI object.
fn schema() -> purrdf_shex::Schema {
    parse_shexc(&format!("<{S}> {{ <{P1}> IRI }}"), None).expect("schema parses")
}

#[test]
fn query_selector_yields_mixed_conformance() {
    let data = data();
    let schema = schema();
    let map_src = format!("{{FOCUS <{P1}> _}}@<{S}>");

    let result = validate_shape_map(
        &schema,
        &data,
        &map_src,
        None,
        &ValidationOptions::default(),
    )
    .expect("validates");

    assert_eq!(result.entries.len(), 2);
    // Deterministic order: resolve_shape_map sorts by term string (s1 < s2).
    assert_eq!(
        result.entries[0].node,
        TermValue::iri("http://a.example/s1")
    );
    assert_eq!(
        result.entries[0].status,
        ConformanceStatus::Nonconformant,
        "s1 has two p1 arcs, over the schema's cardinality of 1"
    );
    assert!(result.entries[0].reason.is_some());

    assert_eq!(
        result.entries[1].node,
        TermValue::iri("http://a.example/s2")
    );
    assert_eq!(
        result.entries[1].status,
        ConformanceStatus::Conformant,
        "s2 has exactly one p1 arc"
    );
    assert!(result.entries[1].reason.is_none());
    assert!(!result.all_conformant());

    // to_result_json is stable and valid JSON.
    let json = result.to_result_json();
    assert_eq!(json, result.to_result_json());
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    let rows = parsed.as_array().expect("array");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["status"], "nonconformant");
    assert_eq!(rows[1]["status"], "conformant");
}

#[test]
fn explicit_node_association_conforms() {
    let data = data();
    let schema = schema();
    let map_src = format!("<http://a.example/s2>@<{S}>");

    let result = validate_shape_map(
        &schema,
        &data,
        &map_src,
        None,
        &ValidationOptions::default(),
    )
    .expect("validates");

    assert_eq!(result.entries.len(), 1);
    assert_eq!(
        result.entries[0].node,
        TermValue::iri("http://a.example/s2")
    );
    assert_eq!(result.entries[0].shape, ShapeSelector::Label(S.to_owned()));
    assert_eq!(result.entries[0].status, ConformanceStatus::Conformant);
}

#[test]
fn parse_error_is_a_hard_err() {
    let data = data();
    let schema = schema();
    let result = validate_shape_map(
        &schema,
        &data,
        "not a valid shape map @@",
        None,
        &ValidationOptions::default(),
    );
    assert!(result.is_err());
}
