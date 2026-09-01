// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `rdf:reifies` is not a functional property: one reifier id may bind SEVERAL
//! triple terms, in the same graph or in different graphs. This suite pins the
//! GTS lane end to end for that shape — `to_gts` → `reader::read` → both
//! importers — asserting the binding COUNT, the binding CONTENT, and the graph
//! slot each binding was declared in.
//!
//! It also pins what makes that possible: a quoted-triple TERM states its own
//! `(s, p, o)` on the wire instead of borrowing some reifier's binding, so two
//! distinct triple terms can no longer collapse into one — and a quoted triple
//! that nobody reifies round-trips without a fabricated reifier at all.

use purrdf_rdf::gts_write::to_gts;
use purrdf_rdf::ir::{RdfDataset, RdfDatasetBuilder};
use purrdf_rdf::{RdfLookaside, RdfReifier, RdfTerm, TermRef, import_gts_events, import_gts_graph};
use std::sync::Arc;

/// One reifier id (`r1`) bound to two DIFFERENT triple terms, one declaration in
/// `<g1>` and one in `<g2>` — the `graphReifierScope` shape, across two graphs.
fn one_reifier_two_bindings_two_graphs() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let a = b.intern_iri("http://example.org/a");
    let related = b.intern_iri("http://example.org/related");
    let bb = b.intern_iri("http://example.org/b");
    let c = b.intern_iri("http://example.org/c");
    let r1 = b.intern_iri("http://example.org/r1");
    let g1 = b.intern_iri("http://example.org/g1");
    let g2 = b.intern_iri("http://example.org/g2");

    b.push_quad(a, related, bb, Some(g1));
    b.push_quad(a, related, c, Some(g2));

    let t1 = b.intern_triple(a, related, bb);
    let t2 = b.intern_triple(a, related, c);
    b.push_reifier_in_graph(r1, t1, Some(g1));
    b.push_reifier_in_graph(r1, t2, Some(g2));

    b.freeze().expect("valid dataset")
}

/// The bindings as `(reifier, subject, predicate, object, graph)` display rows,
/// sorted so the assertion compares content rather than emission order.
fn binding_rows(reifiers: &[RdfReifier]) -> Vec<String> {
    let mut rows: Vec<String> = reifiers
        .iter()
        .map(|r| {
            format!(
                "{:?} | {:?} {} {:?} | {:?}",
                r.reifier, r.statement.subject, r.statement.predicate, r.statement.object, r.graph
            )
        })
        .collect();
    rows.sort();
    rows
}

/// The exact rows the fixture asserts, spelled out so the test cannot pass by
/// comparing a truncated projection against itself.
fn expected_rows() -> Vec<String> {
    let mut rows = vec![
        format!(
            "{:?} | {:?} {} {:?} | {:?}",
            RdfTerm::iri("http://example.org/r1"),
            RdfTerm::iri("http://example.org/a"),
            "http://example.org/related",
            RdfTerm::iri("http://example.org/b"),
            Some(RdfTerm::iri("http://example.org/g1")),
        ),
        format!(
            "{:?} | {:?} {} {:?} | {:?}",
            RdfTerm::iri("http://example.org/r1"),
            RdfTerm::iri("http://example.org/a"),
            "http://example.org/related",
            RdfTerm::iri("http://example.org/c"),
            Some(RdfTerm::iri("http://example.org/g2")),
        ),
    ];
    rows.sort();
    rows
}

#[test]
fn multi_binding_reifier_survives_the_graph_import() {
    let ds = one_reifier_two_bindings_two_graphs();
    let before = binding_rows(&ds.owned_reifiers().collect::<Vec<_>>());
    assert_eq!(before, expected_rows(), "the fixture itself carries 2 rows");

    let bytes = to_gts(&ds, &RdfLookaside::default(), "purrdf-test").expect("to_gts");
    let graph = purrdf_gts::reader::read(&bytes, false, None);
    assert!(graph.diagnostics.is_empty(), "{:?}", graph.diagnostics);
    assert_eq!(
        graph.reifiers.len(),
        2,
        "the reader must keep BOTH bindings of the one reifier id: {:?}",
        graph.reifiers
    );

    let bundle = import_gts_graph(graph).expect("import the folded graph");
    let after = binding_rows(&bundle.dataset.owned_reifiers().collect::<Vec<_>>());
    assert_eq!(
        after,
        expected_rows(),
        "count, content and graph slot must round-trip"
    );
}

#[test]
fn multi_binding_reifier_survives_the_event_import() {
    let ds = one_reifier_two_bindings_two_graphs();

    let bytes = to_gts(&ds, &RdfLookaside::default(), "purrdf-test").expect("to_gts");
    let bundle = import_gts_events(&bytes).expect("import the event stream");
    let after = binding_rows(&bundle.dataset.owned_reifiers().collect::<Vec<_>>());
    assert_eq!(
        after,
        expected_rows(),
        "count, content and graph slot must round-trip"
    );
}

#[test]
fn multi_binding_reifier_writes_deterministically() {
    let ds = one_reifier_two_bindings_two_graphs();
    let first = to_gts(&ds, &RdfLookaside::default(), "purrdf-test").expect("first write");
    let second = to_gts(&ds, &RdfLookaside::default(), "purrdf-test").expect("second write");
    assert_eq!(first, second, "the GTS writer must be byte-deterministic");

    // The bindings' emission order is content-derived, not insertion order: a
    // graph whose rows were pushed in the opposite order writes the SAME bytes.
    let mut b = RdfDatasetBuilder::new();
    let a = b.intern_iri("http://example.org/a");
    let related = b.intern_iri("http://example.org/related");
    let bb = b.intern_iri("http://example.org/b");
    let c = b.intern_iri("http://example.org/c");
    let r1 = b.intern_iri("http://example.org/r1");
    let g1 = b.intern_iri("http://example.org/g1");
    let g2 = b.intern_iri("http://example.org/g2");
    b.push_quad(a, related, c, Some(g2));
    b.push_quad(a, related, bb, Some(g1));
    let t2 = b.intern_triple(a, related, c);
    let t1 = b.intern_triple(a, related, bb);
    b.push_reifier_in_graph(r1, t2, Some(g2));
    b.push_reifier_in_graph(r1, t1, Some(g1));
    let reversed = b.freeze().expect("valid dataset");
    let reversed = to_gts(&reversed, &RdfLookaside::default(), "purrdf-test").expect("write");
    assert_eq!(
        first, reversed,
        "binding order on the wire must be content-derived, not insertion order"
    );
}

/// Two DISTINCT quoted-triple terms sharing one reifier id used to collapse into
/// one, because a triple term's components were looked up through that id.
#[test]
fn two_triple_terms_sharing_one_reifier_stay_distinct() {
    let mut b = RdfDatasetBuilder::new();
    let a = b.intern_iri("http://example.org/a");
    let related = b.intern_iri("http://example.org/related");
    let bb = b.intern_iri("http://example.org/b");
    let c = b.intern_iri("http://example.org/c");
    let r1 = b.intern_iri("http://example.org/r1");
    let holds = b.intern_iri("http://example.org/holds");

    let t1 = b.intern_triple(a, related, bb);
    let t2 = b.intern_triple(a, related, c);
    // Both triple terms occur as ordinary object terms AND are reified by the
    // same reifier id.
    b.push_quad(a, holds, t1, None);
    b.push_quad(a, holds, t2, None);
    b.push_reifier(r1, t1);
    b.push_reifier(r1, t2);
    let ds = b.freeze().expect("valid dataset");

    let bytes = to_gts(&ds, &RdfLookaside::default(), "purrdf-test").expect("to_gts");
    let graph = purrdf_gts::reader::read(&bytes, false, None);
    assert!(graph.diagnostics.is_empty(), "{:?}", graph.diagnostics);
    let bundle = import_gts_graph(graph).expect("import");

    let objects: Vec<String> = bundle
        .dataset
        .quad_refs()
        .map(|q| format!("{:?}", q.o))
        .collect();
    assert_eq!(objects.len(), 2, "both quads survive: {objects:?}");
    assert_ne!(
        objects[0], objects[1],
        "the two quoted triples must stay distinct terms: {objects:?}"
    );
}

/// A quoted triple that NOBODY reifies round-trips: it names its own components,
/// so no reifier — real or fabricated — has to exist for it.
#[test]
fn an_unreified_quoted_triple_round_trips() {
    let mut b = RdfDatasetBuilder::new();
    let a = b.intern_iri("http://example.org/a");
    let p = b.intern_iri("http://example.org/p");
    let o = b.intern_iri("http://example.org/o");
    let t = b.intern_triple(a, p, o);
    b.push_quad(a, p, t, None);
    let ds = b.freeze().expect("valid dataset");

    let bytes = to_gts(&ds, &RdfLookaside::default(), "purrdf-test").expect("to_gts");
    let graph = purrdf_gts::reader::read(&bytes, false, None);
    assert!(graph.diagnostics.is_empty(), "{:?}", graph.diagnostics);
    assert!(
        graph.reifiers.is_empty(),
        "no reifier row may be invented for an unreified quoted triple: {:?}",
        graph.reifiers
    );
    assert!(
        graph
            .terms
            .iter()
            .all(|term| term.kind != purrdf_gts::model::TermKind::Bnode),
        "no blank node may be invented either: {:?}",
        graph.terms
    );

    let bundle = import_gts_graph(graph).expect("import");
    assert_eq!(bundle.dataset.quad_count(), 1);
    let quad = bundle.dataset.quad_refs().next().expect("one quad");
    assert!(
        matches!(quad.o, TermRef::Triple { .. }),
        "the object is still a quoted triple"
    );

    // And the event path agrees.
    let events = import_gts_events(&bytes).expect("import the event stream");
    assert_eq!(events.dataset.quad_count(), 1);
}

/// The legacy ambiguous shape stays loud on the STREAMING path too: a fold
/// diagnostic is a hard error there, so such a file fails closed rather than
/// importing one of two possible meanings.
#[test]
fn the_streaming_import_fails_closed_on_an_ambiguous_legacy_triple_term() {
    use purrdf_gts::model::{Term, TermKind};

    let iri = |value: &str| Term {
        kind: TermKind::Iri,
        value: Some(value.to_string()),
        datatype: None,
        lang: None,
        direction: None,
        reifier: None,
        triple: None,
    };
    let terms = vec![
        iri("http://example.org/a"),
        iri("http://example.org/related"),
        iri("http://example.org/b"),
        iri("http://example.org/c"),
        iri("http://example.org/r1"),
        Term {
            kind: TermKind::Triple,
            value: None,
            datatype: None,
            lang: None,
            direction: None,
            reifier: Some(4),
            triple: None,
        },
    ];
    let mut writer = purrdf_gts::writer::Writer::new("purrdf-test");
    writer.add_terms(&terms);
    writer.add_reifies(&[(4, (0, 1, 2), None), (4, (0, 1, 3), None)]);
    let bytes = writer.into_bytes();

    let err = import_gts_events(&bytes).expect_err("an ambiguous triple term must hard-fail");
    assert!(
        err.message.contains("ConflictingReifier"),
        "unexpected diagnostic: {err:?}"
    );
}
