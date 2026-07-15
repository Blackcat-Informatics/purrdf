// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Shared test fixture for the pack/serializer parity guards
//! (`pack_reconstruct_roundtrip.rs` and `serialize_pack_parity.rs`).
//!
//! Both guards need the SAME rich RDF-1.2 dataset — default + named graphs, every
//! literal shape, a scoped blank node, a quoted-triple term, a reifier with a
//! default-scoped AND a graph-scoped annotation, `example.org` IRIs only — so a single
//! copy lives here instead of drifting across two files.

use std::sync::Arc;

use purrdf_core::{BlankScope, RdfDataset, RdfDatasetBuilder, RdfLiteral, RdfTextDirection};

/// Build the rich fixture `RdfDataset`, mirroring
/// `crates/sparql-eval/tests/pack_query_e2e.rs`: default graph (with a two-hop `knows`
/// join chain, a term used as both predicate and subject, every literal shape — simple,
/// typed, language-tagged, directional — a scoped blank node, and an RDF 1.2 triple
/// term asserted as an object) + 2 named graphs + a reifier with a default-scoped AND a
/// graph-scoped annotation. `example.org` IRIs ONLY.
pub(crate) fn build_fixture() -> Arc<RdfDataset> {
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
