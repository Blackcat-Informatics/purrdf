// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Reconstruction round-trip: the public, `DatasetView`-generic reconstructor
//! [`dataset_from_view`] materializes a concrete [`Arc<RdfDataset>`] from a mmap'd
//! [`PackView`], and that reconstruction is IDENTICAL to the source dataset the pack
//! was built from — isomorphic by RDFC-1.0 digest AND byte-identical when serialized
//! to a star-capable format.
//!
//! This is the ingress twin of the serializer parity guard in
//! `serialize_pack_parity.rs` and the query parity guard in
//! `crates/sparql-eval/tests/pack_query_e2e.rs`: it proves a caller holding only a
//! read-only pack projection can recover a full `RdfDataset` (to feed a transform
//! that needs one, e.g. the reasoner) with zero loss — every base quad, every RDF-1.2
//! reifier binding, and every default- and graph-scoped annotation survive the round
//! trip. The same rich fixture (`example.org` only) exercises every seam the pack
//! codec claims to unify.

use std::sync::Arc;

use purrdf_core::{
    BlankScope, PackBuilder, PackView, RdfDataset, RdfDatasetBuilder, RdfLiteral, RdfTextDirection,
    dataset_from_view, datasets_isomorphic,
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

/// The headline: reconstruct the rich fixture straight off its `PackView` and prove the
/// result is the same dataset — isomorphic by RDFC-1.0 digest AND byte-identical in a
/// star-capable serialization.
#[test]
fn dataset_from_view_roundtrips_the_full_fixture() {
    let ds = build_fixture();

    // Build the pack, open a read-only view over it, and reconstruct a concrete dataset.
    let bytes = PackBuilder::build_bytes(&ds).expect("pack build");
    let view = PackView::from_bytes(&bytes).expect("pack opens");
    let rebuilt = dataset_from_view(&view).expect("reconstruct from view");

    // 1) Direct dataset isomorphism (RDFC-1.0 blank-node canonicalization under the hood).
    assert!(
        datasets_isomorphic(&rebuilt, &ds), // deref-coerces Arc<RdfDataset> -> &RdfDataset
        "reconstruction must be isomorphic to the source dataset"
    );

    // 2) Isomorphism by RDFC-1.0 content digest: build a pack from the reconstruction and
    //    compare its stored RDFC-1.0 digest to the source pack's. Equal digests certify
    //    the two datasets canonicalize identically (base quads + reifier + annotations).
    let rebuilt_bytes = PackBuilder::build_bytes(&rebuilt).expect("pack build from reconstruction");
    let rebuilt_view = PackView::from_bytes(&rebuilt_bytes).expect("rebuilt pack opens");
    assert_eq!(
        view.rdfc_digest(),
        rebuilt_view.rdfc_digest(),
        "reconstruction's RDFC-1.0 digest must equal the source pack's"
    );

    // 3) Byte-identical N-Quads (a star-capable format: the triple-term-as-object base
    //    quad and the reifier/annotation side-tables all serialize losslessly).
    let rebuilt_nq = serialize_dataset_to_format(&*rebuilt, NativeRdfFormat::NQuads, None)
        .expect("serialize reconstruction to NQuads")
        .bytes;
    let source_nq = serialize_dataset_to_format(&*ds, NativeRdfFormat::NQuads, None)
        .expect("serialize source to NQuads")
        .bytes;
    assert_eq!(
        rebuilt_nq, source_nq,
        "reconstruction must serialize byte-identically to the source in NQuads"
    );
    assert!(
        !source_nq.is_empty(),
        "non-vacuous: the fixture is non-empty"
    );
}
