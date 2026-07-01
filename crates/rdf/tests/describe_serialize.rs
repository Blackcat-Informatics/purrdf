// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The Symmetric CBD `describe` extraction (in `purrdf-core`) must produce a
//! structurally valid subgraph that every `native_codecs` serializer here in
//! `purrdf` can emit — the docs multi-format export depends on exactly that hand-off.

use purrdf_core::{RdfDatasetBuilder, RdfLiteral};
use purrdf_rdf::describe::describe;
use purrdf_rdf::native_codecs::jsonld::serialize_dataset_to_jsonld;
use purrdf_rdf::{parse_dataset, serialize_dataset, SerializeGraph};

const S: &str = "https://e/s";

#[test]
fn describe_round_trips_through_every_serializer() {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri(S);
    let p = b.intern_iri("https://e/p");
    let o = b.intern_iri("https://e/o");
    let label = b.intern_iri("https://e/label");
    let hi = b.intern_literal(RdfLiteral::simple("hi"));
    b.push_quad(s, p, o, None);
    b.push_quad(s, label, hi, None);
    let ds = b.freeze().expect("freeze");

    let scbd = describe(&ds, S).expect("describe");

    // Every native RDF format serializes non-empty bytes.
    for media in [
        "text/turtle",
        "application/n-triples",
        "application/n-quads",
        "application/trig",
        "application/rdf+xml",
    ] {
        let bytes = serialize_dataset(&scbd, media, SerializeGraph::Dataset)
            .unwrap_or_else(|e| panic!("serialize {media}: {e}"));
        assert!(!bytes.is_empty(), "{media} produced empty output");
    }
    // JSON-LD rides the separate native_codecs path (not a NativeRdfFormat).
    let jsonld = serialize_dataset_to_jsonld(&scbd).expect("jsonld");
    assert!(jsonld.trim_start().starts_with('{') || jsonld.contains("@graph"));

    // A Turtle round-trip preserves the two triples.
    let ttl = serialize_dataset(&scbd, "text/turtle", SerializeGraph::Dataset).unwrap();
    let back = parse_dataset(&ttl, "text/turtle", None).unwrap();
    assert_eq!(back.quad_count(), 2);
}
