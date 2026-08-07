// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! End-to-end round-trip of SPARQL-rule entailment through text serialization.
//!
//! Drives the pipeline exactly as a text-level consumer would: parse a shapes
//! Turtle document carrying a `sh:SPARQLRule` whose CONSTRUCT template mints
//! anonymous property-shape blanks (`sh:property [ sh:path … ; sh:minCount 1 ]`),
//! parse a data Turtle document, entail, serialize the entailed dataset to
//! Turtle AND N-Triples, re-parse each document, and assert canonical N-Quads
//! equality with the entailed dataset — proving the minted per-focus blank
//! labels survive both egress syntaxes losslessly.

use purrdf::{SerializeGraph, canonicalize, parse_dataset, serialize_dataset};
use purrdf_shapes::rules::entail_dataset;
use purrdf_shapes::shapes::from_dataset_with_prefixes;
use purrdf_shapes::text_ingest::{extract_prefixes, parse_turtle_to_dataset};

/// A shapes document whose rule CONSTRUCTs a real SHACL property shape onto
/// each focus node, through an anonymous (bracketed) template blank.
const SHAPES_TTL: &str = r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/ns#> .

ex:ThingShape a sh:NodeShape ;
  sh:targetClass ex:Thing ;
  sh:rule [ a sh:SPARQLRule ; sh:construct
    "CONSTRUCT { $this sh:property [ sh:path ex:name ; sh:minCount 1 ] } WHERE { $this a ex:Thing }" ] .
"#;

/// Focus nodes whose renderings are hostile to naive blank-label minting: a
/// fragment IRI and a colon-riddled URN.
const DATA_TTL: &str = r"
@prefix ex: <http://example.org/ns#> .

<http://example.org/p#frag> a ex:Thing .
<urn:maplib:69e3353f-2246-4469-bb83-a068cdaa9f1c> a ex:Thing .
";

#[test]
fn sparql_rule_entailment_roundtrips_through_text_serialization() {
    let data = parse_turtle_to_dataset(DATA_TTL).expect("data Turtle must parse");
    let shapes_dataset = parse_turtle_to_dataset(SHAPES_TTL).expect("shapes Turtle must parse");
    let prefixes = extract_prefixes(SHAPES_TTL);
    let shapes = from_dataset_with_prefixes(&shapes_dataset, &prefixes).expect("shapes must load");

    let entailed = entail_dataset(data.as_ref(), &shapes).expect("entailment must succeed");
    assert!(
        entailed.quad_refs().count() > data.quad_refs().count(),
        "the rule must derive property-shape triples for both foci"
    );

    let canonical_entailed = canonicalize(entailed.as_ref()).nquads;
    for media in ["text/turtle", "application/n-triples"] {
        let bytes = serialize_dataset(entailed.as_ref(), media, SerializeGraph::Dataset)
            .unwrap_or_else(|e| panic!("{media} serialization must succeed: {e}"));
        let reparsed = parse_dataset(&bytes, media, None)
            .unwrap_or_else(|e| panic!("{media} re-parse must succeed: {e}"));
        assert_eq!(
            canonical_entailed,
            canonicalize(reparsed.as_ref()).nquads,
            "{media} round-trip must preserve the entailed dataset"
        );
    }
}
