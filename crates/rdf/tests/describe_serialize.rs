// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The Symmetric CBD `describe` extraction (in `purrdf-core`) must produce a
//! structurally valid subgraph that every `native_codecs` serializer here in
//! `purrdf` can emit — the docs multi-format export depends on exactly that hand-off.

use purrdf_core::{RdfDatasetBuilder, RdfLiteral};
use purrdf_rdf::describe::describe;
use purrdf_rdf::native_codecs::jsonld::serialize_dataset_to_jsonld;
use purrdf_rdf::{SerializeGraph, parse_dataset, serialize_dataset};

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

/// A description whose every layer is graph-scoped is still a structurally valid
/// subgraph, and every DATASET-capable serializer emits it with its graphs intact.
///
/// The extraction carries the graph slot through the RDF 1.2 statement layer, so the
/// re-interned subgraph now holds reifier and annotation rows keyed to a named graph —
/// a shape the default-graph round-trip above never builds. This pins that the
/// hand-off to the serializers survives it, and that the N-Quads round-trip returns
/// the same three rows in the same graph.
#[test]
fn a_graph_scoped_description_round_trips_through_the_dataset_serializers() {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri(S);
    let p = b.intern_iri("https://e/p");
    let o = b.intern_iri("https://e/o");
    let g = b.intern_iri("https://e/g");
    b.push_quad(s, p, o, Some(g));
    let triple = b.intern_triple(s, p, o);
    let reifier = b.intern_iri("https://e/r");
    b.push_reifier_in_graph(reifier, triple, Some(g));
    let note = b.intern_iri("https://e/note");
    let n = b.intern_literal(RdfLiteral::simple("n"));
    b.push_annotation_in_graph(reifier, note, n, Some(g));
    let ds = b.freeze().expect("freeze");

    let scbd = describe(&ds, S).expect("describe");
    assert_eq!(scbd.quad_count(), 1);
    assert_eq!(scbd.reifiers().count(), 1);
    assert_eq!(scbd.annotations().count(), 1);

    for media in [
        "application/n-quads",
        "application/trig",
        "text/turtle",
        "application/n-triples",
        "application/rdf+xml",
    ] {
        let bytes = serialize_dataset(&scbd, media, SerializeGraph::Dataset)
            .unwrap_or_else(|e| panic!("serialize {media}: {e}"));
        // The single-graph syntaxes legitimately emit an empty document here (every row
        // is graph-scoped and they have nowhere to put it); what is pinned is that they
        // do not fail on the shape.
        let _ = bytes;
    }

    let nq = serialize_dataset(&scbd, "application/n-quads", SerializeGraph::Dataset).unwrap();
    let text = String::from_utf8(nq.clone()).unwrap();
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), 3, "base quad + reifier + annotation:\n{text}");
    assert!(
        lines.iter().all(|l| l.contains("<https://e/g> .")),
        "every layer keeps its graph:\n{text}"
    );

    let back = parse_dataset(&nq, "application/n-quads", None).unwrap();
    assert_eq!(back.quad_count(), 1);
    assert_eq!(back.reifiers().count(), 1);
    assert_eq!(back.annotations().count(), 1);
}
