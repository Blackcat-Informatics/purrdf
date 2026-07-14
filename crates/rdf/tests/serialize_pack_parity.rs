// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Serializer parity across `DatasetView` backends: the native RDF serializer
//! ([`serialize_dataset_to_format`]) is generic over any [`DatasetView`], so the SAME
//! rich fixture serialized through the production [`RdfDataset`] and through a
//! [`PackView`] opened over that dataset's pack bytes must be BYTE-IDENTICAL for every
//! [`NativeRdfFormat`] — and report the same star-layer drop count.
//!
//! This is the egress twin of the query-side parity guard in
//! `crates/sparql-eval/tests/pack_query_e2e.rs`: it proves a `PackView` (or any
//! `DatasetView`) serializes straight to RDF text with zero materialization and no
//! behavioral divergence from the source dataset. Byte-identity is the strong claim —
//! it certifies that the pack codec preserves the quad/term/side-table iteration order
//! the serializer folds into its deterministic output.

use std::sync::Arc;

use purrdf_core::{
    BlankScope, DatasetView, PackBuilder, PackView, RdfDataset, RdfDatasetBuilder, RdfLiteral,
    RdfTextDirection,
};
use purrdf_rdf::{NativeRdfFormat, serialize_dataset_to_format};

/// Build the rich fixture `RdfDataset`, mirroring
/// `crates/sparql-eval/tests/pack_query_e2e.rs`: default graph (with a two-hop `knows`
/// join chain, a term used as both predicate and subject, every literal shape — simple,
/// typed, language-tagged, directional — a scoped blank node, and an RDF 1.2 triple
/// term asserted as an object) + 2 named graphs + a reifier with a default-scoped AND a
/// graph-scoped annotation. `example.org` IRIs ONLY.
fn build_fixture() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();

    let alice = b.intern_iri("http://example.org/alice");
    let bob = b.intern_iri("http://example.org/bob");
    let carol = b.intern_iri("http://example.org/carol");
    let knows = b.intern_iri("http://example.org/knows");
    let age = b.intern_iri("http://example.org/age");
    let name = b.intern_iri("http://example.org/name");
    let sees = b.intern_iri("http://example.org/sees");
    let likes = b.intern_iri("http://example.org/likes");
    let greeting = b.intern_iri("http://example.org/greeting");
    let confidence = b.intern_iri("http://example.org/confidence");
    let high = b.intern_iri("http://example.org/high");
    let source = b.intern_iri("http://example.org/source");
    let doc = b.intern_iri("http://example.org/doc");
    let meta = b.intern_iri("http://example.org/meta");
    let states_fact = b.intern_iri("http://example.org/statesFact");
    let reifier = b.intern_iri("http://example.org/r");
    let graph1 = b.intern_iri("http://example.org/graph1");
    let graph2 = b.intern_iri("http://example.org/graph2");

    let blank = b.intern_blank("b1", BlankScope::DEFAULT);

    let forty_two = b.intern_literal(RdfLiteral {
        lexical_form: "42".to_string(),
        datatype: Some("http://www.w3.org/2001/XMLSchema#integer".to_string()),
        language: None,
        direction: None,
    });
    let alice_name_en = b.intern_literal(RdfLiteral {
        lexical_form: "Alice".to_string(),
        datatype: None,
        language: Some("en".to_string()),
        direction: None,
    });
    let anon_name = b.intern_literal(RdfLiteral {
        lexical_form: "Anon".to_string(),
        datatype: None,
        language: None,
        direction: None,
    });
    let knows_label = b.intern_literal(RdfLiteral {
        lexical_form: "the knows predicate".to_string(),
        datatype: None,
        language: None,
        direction: None,
    });
    let hello_ltr = b.intern_literal(RdfLiteral {
        lexical_form: "Hello".to_string(),
        datatype: None,
        language: Some("en".to_string()),
        direction: Some(RdfTextDirection::Ltr),
    });
    let carol_age = b.intern_literal(RdfLiteral {
        lexical_form: "30".to_string(),
        datatype: Some("http://www.w3.org/2001/XMLSchema#integer".to_string()),
        language: None,
        direction: None,
    });

    // The RDF 1.2 triple term `<<( alice knows bob )>>`, asserted as an OBJECT below and
    // reified further down — occupies both roles.
    let alice_knows_bob = b.intern_triple(alice, knows, bob);

    // -- Default graph ----------------------------------------------------------
    b.push_quad(alice, knows, bob, None);
    b.push_quad(bob, knows, carol, None);
    b.push_quad(alice, age, forty_two, None);
    b.push_quad(alice, name, alice_name_en, None);
    // `knows` used as a SUBJECT here and as a PREDICATE above.
    b.push_quad(knows, name, knows_label, None);
    b.push_quad(blank, name, anon_name, None);
    b.push_quad(bob, sees, blank, None);
    b.push_quad(alice, greeting, hello_ltr, None);
    b.push_quad(meta, states_fact, alice_knows_bob, None);

    // -- Named graph 1 ------------------------------------------------------------
    b.push_quad(carol, age, carol_age, Some(graph1));
    b.push_quad(bob, knows, carol, Some(graph1));

    // -- Named graph 2 ------------------------------------------------------------
    b.push_quad(carol, likes, alice, Some(graph2));

    // -- Reifier + annotations -----------------------------------------------------
    b.push_reifier(reifier, alice_knows_bob);
    b.push_annotation(reifier, confidence, high);
    // A second, graph-scoped annotation on the SAME reifier.
    b.push_annotation_in_graph(reifier, source, doc, Some(graph1));

    b.freeze().expect("fixture dataset must validate")
}

/// Every native RDF format the serializer targets.
const ALL_FORMATS: &[NativeRdfFormat] = &[
    NativeRdfFormat::Turtle,
    NativeRdfFormat::TriG,
    NativeRdfFormat::NTriples,
    NativeRdfFormat::NQuads,
    NativeRdfFormat::RdfXml,
    NativeRdfFormat::TriX,
    NativeRdfFormat::HexTuples,
    NativeRdfFormat::JsonLd,
    NativeRdfFormat::YamlLd,
];

/// Whether `fmt` is a classic star-incapable quad syntax with NO RDF-1.2 triple-term
/// surface at all: it cannot serialize the fixture's triple-term-as-object base quad
/// (`meta statesFact <<( alice knows bob )>>`) and, by deliberate design, HARD-errors
/// on it rather than dropping it silently (see `native_codecs::trix` /
/// `native_codecs::hextuples`). Every other format either carries the star layer or,
/// like RDF/XML, has a triple-term surface for the base quad while dropping only the
/// reifier/annotation statement layer.
fn is_classic_no_triple_term_surface(fmt: NativeRdfFormat) -> bool {
    matches!(fmt, NativeRdfFormat::TriX | NativeRdfFormat::HexTuples)
}

#[test]
fn pack_view_serializes_identically_to_source_dataset() {
    let ds = build_fixture();
    let bytes = PackBuilder::build_bytes(&ds).expect("pack build");
    let view = PackView::from_bytes(&bytes).expect("pack opens");

    // Sanity: the fixture actually carries a star layer, so the parity check genuinely
    // exercises the reifier/annotation side tables (a check over star-free data would
    // be vacuous for the star-drop accounting).
    assert!(view.reifier_quads().count() >= 1, "fixture has a reifier");
    assert!(
        view.annotation_quads().count() >= 2,
        "fixture has two annotations"
    );

    // At least one format must succeed with real bytes, so the whole matrix is never a
    // vacuous "every backend errored identically" pass.
    let mut succeeded = 0usize;

    for &fmt in ALL_FORMATS {
        let from_source = serialize_dataset_to_format(&*ds, fmt, None);
        let from_pack = serialize_dataset_to_format(&view, fmt, None);

        match (from_source, from_pack) {
            (Ok(source), Ok(pack)) => {
                // The `PackView` (id space `PackId`) and the source `RdfDataset` (id
                // space `TermId`) intern terms in different orders, so byte-identity
                // here certifies the serializer's canonical, value-based ordering is
                // fully backend-independent.
                assert!(
                    !source.bytes.is_empty(),
                    "{fmt:?}: source serialization must be non-vacuous"
                );
                assert!(
                    !pack.bytes.is_empty(),
                    "{fmt:?}: pack serialization must be non-vacuous"
                );
                assert_eq!(
                    source.bytes, pack.bytes,
                    "{fmt:?}: PackView serialization must be BYTE-IDENTICAL to the source RdfDataset"
                );
                assert_eq!(
                    source.statement_rows_dropped, pack.statement_rows_dropped,
                    "{fmt:?}: star-layer drop count must match across backends"
                );
                assert!(
                    !is_classic_no_triple_term_surface(fmt),
                    "{fmt:?}: a classic no-triple-term format must NOT serialize the \
                     triple-term-as-object fixture — it is expected to hard-error"
                );
                succeeded += 1;
            }
            (Err(source), Err(pack)) => {
                // Parity holds on the error path too: a format that cannot represent the
                // fixture must fail IDENTICALLY on both backends (same code + message),
                // never diverge (one erroring while the other emits partial output).
                assert!(
                    is_classic_no_triple_term_surface(fmt),
                    "{fmt:?}: unexpected serialize error on both backends: {}",
                    source.message
                );
                assert_eq!(
                    source.code, pack.code,
                    "{fmt:?}: error code must match across backends"
                );
                assert_eq!(
                    source.message, pack.message,
                    "{fmt:?}: error message must match across backends"
                );
            }
            (source, pack) => panic!(
                "{fmt:?}: PackView and source diverge (one Ok, one Err): source={source:?} pack={pack:?}"
            ),
        }
    }

    assert!(
        succeeded >= 7,
        "expected the star-capable + RDF/XML formats to serialize; only {succeeded} did"
    );
}
