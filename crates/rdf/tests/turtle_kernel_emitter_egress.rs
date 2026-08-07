// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `purrdf_core::turtle::{emit_reifier, emit_resource}` egress totality.
//!
//! Both functions build a Turtle statement out of a caller-supplied
//! `(predicate, object)` property list. Prior to structuring `object` as an
//! [`RdfTerm`], a caller could hand either function an already-rendered
//! object string; a hostile pre-rendered blank-node token (e.g. `_:bad
//! label`) would then reach the document verbatim, producing output no
//! conforming parser — including PurRDF's own — could read back. Now that
//! `object` is a structured [`RdfTerm`] routed through the module's own
//! [`emit_term`](purrdf_rdf::emit_term), a hostile blank-node label is
//! escaped like every other emitting position, so the document this crate's
//! native Turtle parser reads back is always the one the caller meant.

use purrdf_rdf::{RdfReifier, RdfTerm, RdfTriple, emit_reifier, emit_resource, parse_dataset};

/// Adversarial blank-node labels a caller might pass as an annotation /
/// property object: interior whitespace and a multiplication-sign byte just
/// past the `PN_CHARS_BASE` gap, neither legal under `BLANK_NODE_LABEL`.
const HOSTILE_LABELS: &[&str] = &["bad label", "a\u{d7}b"];

#[test]
fn emit_reifier_annotation_object_round_trips_through_the_native_parser() {
    let triple = RdfTriple::new(
        RdfTerm::iri("http://example.org/s"),
        "http://example.org/p",
        RdfTerm::iri("http://example.org/o"),
    );
    let reifier = RdfReifier::new(RdfTerm::iri("http://example.org/r"), triple);

    for label in HOSTILE_LABELS {
        let doc = emit_reifier(
            &reifier,
            &[(
                "http://example.org/annotates".to_owned(),
                RdfTerm::blank_node(*label),
            )],
        );
        assert!(
            !doc.contains(&format!("_:{label}")),
            "the raw hostile label must never reach the document: {doc}"
        );

        let dataset = parse_dataset(doc.as_bytes(), "text/turtle", None).unwrap_or_else(|err| {
            panic!("emitted reifier with hostile annotation object must re-parse: {err}\n{doc}")
        });
        assert_eq!(
            dataset.reifiers().count(),
            1,
            "the rdf:reifies binding round-trips: {doc}"
        );
        assert_eq!(
            dataset.annotations().count(),
            1,
            "the annotation triple round-trips: {doc}"
        );
    }
}

#[test]
fn emit_resource_property_object_round_trips_through_the_native_parser() {
    for label in HOSTILE_LABELS {
        let doc = emit_resource(
            "http://example.org/subject",
            &[(
                "http://example.org/annotates".to_owned(),
                RdfTerm::blank_node(*label),
            )],
        );
        assert!(
            !doc.contains(&format!("_:{label}")),
            "the raw hostile label must never reach the document: {doc}"
        );

        let dataset = parse_dataset(doc.as_bytes(), "text/turtle", None).unwrap_or_else(|err| {
            panic!("emitted resource with hostile property object must re-parse: {err}\n{doc}")
        });
        assert_eq!(
            dataset.quad_count(),
            1,
            "the property triple round-trips: {doc}"
        );
    }
}
