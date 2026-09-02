// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Keeps the root `README.md` Rust quickstart honest: this test is the same
//! code, verbatim. If it stops compiling or passing, fix the README too.

use purrdf::{
    NativeRdfFormat, RdfDatasetBuilder, RdfLiteral, SerializeGraph, parse_dataset,
    serialize_dataset, serialize_dataset_to_format,
};

#[test]
fn readme_quickstart_round_trips() {
    // Build a dataset in interned TermId space.
    let mut b = RdfDatasetBuilder::new();
    let alice = b.intern_iri("https://example.org/alice");
    let knows = b.intern_iri("http://xmlns.com/foaf/0.1/knows");
    let bob = b.intern_iri("https://example.org/bob");
    let name = b.intern_iri("http://xmlns.com/foaf/0.1/name");
    let hi = b.intern_literal(RdfLiteral::simple("Alice"));
    b.push_quad(alice, knows, bob, None);
    b.push_quad(alice, name, hi, None);
    let ds = b.freeze().expect("freeze");

    // Serialize to any native codec and parse back, losslessly.
    let ttl = serialize_dataset(&ds, "text/turtle", SerializeGraph::Dataset).unwrap();
    let back = parse_dataset(&ttl, "text/turtle", None).unwrap();
    assert_eq!(back.quad_count(), 2);
}

#[test]
fn readme_document_base_round_trips() {
    let base = "https://example.org/base/";

    // Ingress: a relative subject resolves against the base.
    let doc = "<rel> <https://example.org/p> <https://example.org/o> .\n";
    let ds = parse_dataset(doc.as_bytes(), "text/turtle", Some(base)).unwrap();
    assert_eq!(ds.quad_count(), 1);

    // Egress: Turtle can express a base, so it is written and relativized against.
    let out = serialize_dataset_to_format(&ds, NativeRdfFormat::Turtle, Some(base)).unwrap();
    let turtle = String::from_utf8(out.bytes).unwrap();
    assert!(turtle.contains("@base <https://example.org/base/> ."));

    // With no base in scope a relative reference hard-fails — none is ever fabricated.
    let err = parse_dataset(doc.as_bytes(), "text/turtle", None).unwrap_err();
    assert_eq!(err.code, "iri-relative-no-base");
}
