// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The wire model of a multi-binding reifier, at the GTS layer.
//!
//! `rdf:reifies` is not a functional property, so one reifier id may bind
//! several distinct triples. The `reifies` frame therefore carries all of them,
//! and a quoted-triple TERM states its own components (wire `"tt"`) instead of
//! borrowing one reifier's binding.
//!
//! `ConflictingReifier` survives for exactly one shape: a term written in the
//! older indirect spelling (`rf`, no `tt`) whose reifier id turns out to bind
//! more than one triple, so the file asks for a single term with two meanings.

use purrdf_gts::model::{Graph, Term, TermKind};
use purrdf_gts::reader::read;
use purrdf_gts::writer::Writer;

fn iri(value: &str) -> Term {
    Term {
        kind: TermKind::Iri,
        value: Some(value.to_string()),
        datatype: None,
        lang: None,
        direction: None,
        reifier: None,
        triple: None,
    }
}

fn triple_term(s: usize, p: usize, o: usize) -> Term {
    Term {
        kind: TermKind::Triple,
        value: None,
        datatype: None,
        lang: None,
        direction: None,
        reifier: None,
        triple: Some((s, p, o)),
    }
}

/// Terms `0..=4`: `a`, `related`, `b`, `c`, `r1`.
fn base_terms() -> Vec<Term> {
    vec![
        iri("http://example.org/a"),
        iri("http://example.org/related"),
        iri("http://example.org/b"),
        iri("http://example.org/c"),
        iri("http://example.org/r1"),
    ]
}

/// One reifier id bound to two different triples, in two different graphs, is
/// legitimate RDF 1.2 — both rows survive the fold, with their graph slots, and
/// no diagnostic is raised.
#[test]
fn a_reifier_id_may_bind_several_triples() {
    let mut terms = base_terms();
    terms.push(iri("http://example.org/g1")); // 5
    terms.push(iri("http://example.org/g2")); // 6

    let mut writer = Writer::new("purrdf-test");
    writer.add_terms(&terms);
    writer.add_reifies(&[(4, (0, 1, 2), Some(5)), (4, (0, 1, 3), Some(6))]);
    let bytes = writer.into_bytes();

    let graph = read(&bytes, false, None);
    assert!(graph.diagnostics.is_empty(), "{:?}", graph.diagnostics);
    assert_eq!(
        graph.reifiers,
        vec![(4, (0, 1, 2), Some(5)), (4, (0, 1, 3), Some(6))],
        "both bindings survive, each keeping the graph it was declared in"
    );
    assert_eq!(graph.reifier_binding_count(4), 2);
    assert_eq!(
        graph.reifier_bindings(4).collect::<Vec<_>>(),
        vec![((0, 1, 2), Some(5)), ((0, 1, 3), Some(6))]
    );
}

/// A self-describing quoted triple resolves from its own `"tt"`, so the reifier
/// id's several bindings never touch it.
#[test]
fn a_self_describing_triple_term_ignores_the_reifier_bindings() {
    let mut terms = base_terms();
    terms.push(triple_term(0, 1, 2)); // 5
    terms.push(triple_term(0, 1, 3)); // 6

    let mut writer = Writer::new("purrdf-test");
    writer.add_terms(&terms);
    writer.add_reifies(&[(4, (0, 1, 2), None), (4, (0, 1, 3), None)]);
    let bytes = writer.into_bytes();

    let graph = read(&bytes, false, None);
    assert!(graph.diagnostics.is_empty(), "{:?}", graph.diagnostics);

    // Locate the two triple terms in the (unsorted) fold order and check each
    // resolves to ITS OWN components rather than the reifier's first binding.
    let resolved: Vec<Option<(usize, usize, usize)>> = (0..graph.terms.len())
        .filter(|&tid| graph.terms[tid].kind == TermKind::Triple)
        .map(|tid| graph.triple_of(tid))
        .collect();
    assert_eq!(resolved.len(), 2);
    assert_ne!(
        resolved[0], resolved[1],
        "two distinct triple terms must stay distinct: {resolved:?}"
    );
}

/// The one shape `ConflictingReifier` still guards: a `tt`-less triple term
/// whose reifier id binds two different triples. Every binding is still kept —
/// the diagnostic reports an ambiguous TERM, it does not drop data.
#[test]
fn a_legacy_indirect_triple_term_over_a_rebound_reifier_is_flagged() {
    let mut terms = base_terms();
    terms.push(Term {
        kind: TermKind::Triple,
        value: None,
        datatype: None,
        lang: None,
        direction: None,
        reifier: Some(4),
        triple: None,
    }); // 5 — the legacy indirect spelling

    let mut writer = Writer::new("purrdf-test");
    writer.add_terms(&terms);
    writer.add_reifies(&[(4, (0, 1, 2), None), (4, (0, 1, 3), None)]);
    let bytes = writer.into_bytes();

    let graph = read(&bytes, false, None);
    assert_eq!(
        graph
            .diagnostics
            .iter()
            .map(|d| d.code.as_str())
            .collect::<Vec<_>>(),
        vec!["ConflictingReifier"],
        "{:?}",
        graph.diagnostics
    );
    assert_eq!(
        graph.reifiers.len(),
        2,
        "the ambiguity is REPORTED, never resolved by dropping a binding"
    );
    assert_eq!(
        graph.triple_of(5),
        Some((0, 1, 2)),
        "an indirect term can only mean the first binding"
    );
}

/// A `tt` naming a not-yet-introduced term is a forward reference, exactly like
/// a forward `dt`/`rf`: it is reported and the field is dropped, never trusted.
#[test]
fn a_forward_referencing_tt_is_reported() {
    let mut terms = base_terms();
    terms.push(triple_term(0, 1, 99)); // 5 — 99 was never introduced

    let mut writer = Writer::new("purrdf-test");
    writer.add_terms(&terms);
    let bytes = writer.into_bytes();

    let graph = read(&bytes, false, None);
    assert!(
        graph
            .diagnostics
            .iter()
            .any(|d| d.code == "ForwardReference"),
        "{:?}",
        graph.diagnostics
    );
    assert_eq!(graph.terms[5].triple, None);
}

/// A nested quoted triple keeps its components introduced before it after the
/// writer's canonical id remap, whichever way the content happens to sort.
#[test]
fn nested_triple_terms_stay_topologically_ordered() {
    let mut graph = Graph::default();
    graph.terms.push(iri("http://example.org/a")); // 0
    graph.terms.push(iri("http://example.org/p")); // 1
    graph.terms.push(iri("http://example.org/z")); // 2
    graph.terms.push(iri("http://example.org/b")); // 3
    graph.terms.push(triple_term(2, 1, 3)); // 4 — <<( z p b )>>
    graph.terms.push(triple_term(0, 1, 4)); // 5 — <<( a p <<( z p b )>> )>>
    graph.quads.push((0, 1, 5, None));

    let bytes = Writer::deterministic(&graph, "purrdf-test")
        .expect("write")
        .into_bytes();
    let folded = read(&bytes, false, None);
    assert!(folded.diagnostics.is_empty(), "{:?}", folded.diagnostics);

    for (tid, term) in folded.terms.iter().enumerate() {
        if let Some((s, p, o)) = term.triple {
            assert!(
                s < tid && p < tid && o < tid,
                "term {tid} forward-references its own components: {term:?}"
            );
        }
    }
    let quad = folded.quads.first().copied().expect("one quad");
    let inner = folded.triple_of(quad.2).expect("outer triple resolves");
    assert_eq!(
        folded
            .triple_of(inner.2)
            .map(|(s, _, _)| folded.terms[s].value.clone().unwrap_or_default()),
        Some("http://example.org/z".to_string()),
        "the nested triple survives intact"
    );
}
