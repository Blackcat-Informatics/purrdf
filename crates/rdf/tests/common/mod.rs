// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Shared test fixture for the pack/serializer parity guards
//! (`pack_reconstruct_roundtrip.rs` and `serialize_pack_parity.rs`).
//!
//! Both guards need the SAME rich RDF-1.2 dataset — default + named graphs, every
//! literal shape, a scoped blank node, a quoted-triple term, a reifier declared in the
//! default graph AND in a named graph with an annotation in each, `example.org` IRIs
//! only — so a single copy lives here instead of drifting across two files.

use std::sync::Arc;

use purrdf_core::{BlankScope, RdfDataset, RdfDatasetBuilder, RdfLiteral, RdfTextDirection};

/// Build the rich fixture `RdfDataset`, mirroring
/// `crates/sparql-eval/tests/pack_query_e2e.rs`: default graph (with a two-hop `knows`
/// join chain, a term used as both predicate and subject, every literal shape — simple,
/// typed, language-tagged, directional — a scoped blank node, and an RDF 1.2 triple
/// term asserted as an object) + 2 named graphs + a reifier declared in the default
/// graph and again in `graph1`, carrying a default-scoped AND a graph-scoped
/// annotation. `example.org` IRIs ONLY.
pub(crate) fn build_fixture() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();

    let alice = b.intern_iri("http://example.org/alice").unwrap();
    let bob = b.intern_iri("http://example.org/bob").unwrap();
    let carol = b.intern_iri("http://example.org/carol").unwrap();
    let knows = b.intern_iri("http://example.org/knows").unwrap();
    let age = b.intern_iri("http://example.org/age").unwrap();
    let name = b.intern_iri("http://example.org/name").unwrap();
    let sees = b.intern_iri("http://example.org/sees").unwrap();
    let likes = b.intern_iri("http://example.org/likes").unwrap();
    let greeting = b.intern_iri("http://example.org/greeting").unwrap();
    let confidence = b.intern_iri("http://example.org/confidence").unwrap();
    let high = b.intern_iri("http://example.org/high").unwrap();
    let source = b.intern_iri("http://example.org/source").unwrap();
    let doc = b.intern_iri("http://example.org/doc").unwrap();
    let meta = b.intern_iri("http://example.org/meta").unwrap();
    let states_fact = b.intern_iri("http://example.org/statesFact").unwrap();
    let reifier = b.intern_iri("http://example.org/r").unwrap();
    let graph1 = b.intern_iri("http://example.org/graph1").unwrap();
    let graph2 = b.intern_iri("http://example.org/graph2").unwrap();

    let blank = b.intern_blank("b1", BlankScope::DEFAULT).unwrap();

    let forty_two = b
        .intern_literal(RdfLiteral {
            lexical_form: "42".to_string(),
            datatype: Some("http://www.w3.org/2001/XMLSchema#integer".to_string()),
            language: None,
            direction: None,
        })
        .unwrap();
    let alice_name_en = b
        .intern_literal(RdfLiteral {
            lexical_form: "Alice".to_string(),
            datatype: None,
            language: Some("en".to_string()),
            direction: None,
        })
        .unwrap();
    let anon_name = b
        .intern_literal(RdfLiteral {
            lexical_form: "Anon".to_string(),
            datatype: None,
            language: None,
            direction: None,
        })
        .unwrap();
    let knows_label = b
        .intern_literal(RdfLiteral {
            lexical_form: "the knows predicate".to_string(),
            datatype: None,
            language: None,
            direction: None,
        })
        .unwrap();
    let hello_ltr = b
        .intern_literal(RdfLiteral {
            lexical_form: "Hello".to_string(),
            datatype: None,
            language: Some("en".to_string()),
            direction: Some(RdfTextDirection::Ltr),
        })
        .unwrap();
    let carol_age = b
        .intern_literal(RdfLiteral {
            lexical_form: "30".to_string(),
            datatype: Some("http://www.w3.org/2001/XMLSchema#integer".to_string()),
            language: None,
            direction: None,
        })
        .unwrap();

    // The RDF 1.2 triple term `<<( alice knows bob )>>`, asserted as an OBJECT below and
    // reified further down — occupies both roles.
    let alice_knows_bob = b.intern_triple(alice, knows, bob).unwrap();

    // -- Default graph ----------------------------------------------------------
    b.push_quad(alice, knows, bob, None).unwrap();
    b.push_quad(bob, knows, carol, None).unwrap();
    b.push_quad(alice, age, forty_two, None).unwrap();
    b.push_quad(alice, name, alice_name_en, None).unwrap();
    // `knows` used as a SUBJECT here and as a PREDICATE above.
    b.push_quad(knows, name, knows_label, None).unwrap();
    b.push_quad(blank, name, anon_name, None).unwrap();
    b.push_quad(bob, sees, blank, None).unwrap();
    b.push_quad(alice, greeting, hello_ltr, None).unwrap();
    b.push_quad(meta, states_fact, alice_knows_bob, None)
        .unwrap();

    // -- Named graph 1 ------------------------------------------------------------
    b.push_quad(carol, age, carol_age, Some(graph1)).unwrap();
    b.push_quad(bob, knows, carol, Some(graph1)).unwrap();

    // -- Named graph 2 ------------------------------------------------------------
    b.push_quad(carol, likes, alice, Some(graph2)).unwrap();

    // -- Reifier + annotations -----------------------------------------------------
    b.push_reifier(reifier, alice_knows_bob).unwrap();
    b.push_annotation(reifier, confidence, high).unwrap();
    // The SAME reifier declared a second time inside a named graph, with its own
    // annotation there. The statement layer is per-graph, so an annotation belongs
    // to the reifier declared in ITS graph: a `graph1` annotation needs a `graph1`
    // declaration to hang off, and without one the same bytes would read back as an
    // ordinary `graph1` quad about the reifier resource. Declaring it in both graphs
    // is what makes this fixture's graph-scoped annotation expressible in every
    // syntax, which is what the serializer round-trip guards require.
    b.push_reifier_in_graph(reifier, alice_knows_bob, Some(graph1))
        .unwrap();
    b.push_annotation_in_graph(reifier, source, doc, Some(graph1))
        .unwrap();

    b.freeze().expect("fixture dataset must validate")
}
