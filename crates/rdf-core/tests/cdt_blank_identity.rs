// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A `cdt:List` / `cdt:Map` literal is a literal. Canonicalization, the
//! isomorphism oracle built on it and canonical relabeling read its lexical
//! form as written: a blank label spelled inside it is text, not a blank node
//! of the graph, because the canonical line sequence carries the literal as one
//! literal term. Only the whole-dataset skolemization rewrite reaches inside a
//! composite literal.
//!
//! These tests exercise that from the public surface, in both directions: text
//! that differs is a different dataset, and a renaming of a blank node that
//! holds a position in a statement is still a renaming.

use std::sync::Arc;

use purrdf_core::{
    BlankScope, RdfDataset, RdfDatasetBuilder, RdfLiteral, TermRef, canonical_relabel,
    datasets_isomorphic, deskolemize, skolemize, try_canonicalize,
};

const LIST: &str = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/List";
const P: &str = "http://example.org/p";
const S: &str = "http://example.org/s";

/// `ex:s ex:p "<lexical>"^^cdt:List .`
fn list_object(lexical: &str) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri(S).unwrap();
    let p = b.intern_iri(P).unwrap();
    let o = b.intern_literal(RdfLiteral::typed(lexical, LIST)).unwrap();
    b.push_quad(s, p, o, None).unwrap();
    b.freeze().expect("a structurally valid one-quad dataset")
}

/// `_:{subject} ex:p "<lexical>"^^cdt:List .`
fn blank_subject_list(subject: &str, lexical: &str) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_blank(subject, BlankScope::DEFAULT).unwrap();
    let p = b.intern_iri(P).unwrap();
    let o = b.intern_literal(RdfLiteral::typed(lexical, LIST)).unwrap();
    b.push_quad(s, p, o, None).unwrap();
    b.freeze().expect("a structurally valid one-quad dataset")
}

// ── Isomorphism ─────────────────────────────────────────────────────────────

/// Two datasets whose composite literals spell a blank label differently hold
/// two different literals. The label is text, so a renaming inside the literal
/// is not a renaming of the graph.
#[test]
fn a_label_spelled_inside_a_composite_literal_is_text() {
    let a = list_object("[_:x, 42]");
    let b = list_object("[_:y, 42]");
    assert!(
        !datasets_isomorphic(
            &a,
            &b,
            &mut purrdf_core::CanonicalizationControl::new(purrdf_core::CANON_CORPUS_POLICY, None)
        )
        .unwrap()
    );
    assert!(
        !datasets_isomorphic(
            &b,
            &a,
            &mut purrdf_core::CanonicalizationControl::new(purrdf_core::CANON_CORPUS_POLICY, None)
        )
        .unwrap()
    );
    // The neighbouring valid case: the same text is the same dataset.
    assert!(
        datasets_isomorphic(
            &a,
            &list_object("[_:x, 42]"),
            &mut purrdf_core::CanonicalizationControl::new(purrdf_core::CANON_CORPUS_POLICY, None)
        )
        .unwrap()
    );
}

/// A blank node that holds a position in a statement is renamed freely, whatever
/// the literal beside it spells: only the position's blank node is a node.
#[test]
fn renaming_a_term_position_blank_is_a_renaming_beside_a_composite_literal() {
    let a = blank_subject_list("x", "[_:x, 42]");
    let b = blank_subject_list("y", "[_:x, 42]");
    assert!(
        datasets_isomorphic(
            &a,
            &b,
            &mut purrdf_core::CanonicalizationControl::new(purrdf_core::CANON_CORPUS_POLICY, None)
        )
        .unwrap()
    );
    // A literal that spells the other label is another literal.
    let c = blank_subject_list("y", "[_:y, 42]");
    assert!(
        !datasets_isomorphic(
            &a,
            &c,
            &mut purrdf_core::CanonicalizationControl::new(purrdf_core::CANON_CORPUS_POLICY, None)
        )
        .unwrap()
    );
}

/// A change to the literal's GROUND content is a difference.
#[test]
fn ground_content_still_counts() {
    assert!(
        !datasets_isomorphic(
            &list_object("[_:x, 42]"),
            &list_object("[_:x, 43]"),
            &mut purrdf_core::CanonicalizationControl::new(purrdf_core::CANON_CORPUS_POLICY, None)
        )
        .unwrap()
    );
}

// ── Canonicalization ────────────────────────────────────────────────────────

/// The canonical document carries the literal's text as written and labels the
/// blank node of the statement by its position.
#[test]
fn the_canonical_form_keeps_the_literal_text_and_labels_the_subject() {
    let owner = try_canonicalize(
        &blank_subject_list("x", "[_:x, 42]"),
        &mut purrdf_core::CanonicalizationControl::new(purrdf_core::CANON_CORPUS_POLICY, None),
    )
    .unwrap();
    let text = owner.nquads();
    assert!(text.contains("\"[_:x, 42]\""), "literal text kept: {text}");
    assert!(
        text.starts_with("_:0 "),
        "subject labelled by position: {text}"
    );
    // A renamed subject over the same literal canonicalizes to the same bytes.
    let other = try_canonicalize(
        &blank_subject_list("q", "[_:x, 42]"),
        &mut purrdf_core::CanonicalizationControl::new(purrdf_core::CANON_CORPUS_POLICY, None),
    )
    .unwrap();
    assert_eq!(text, other.nquads());
}

/// `canonical_relabel` copies the composite literal as written and relabels
/// only the blank node that holds a position.
#[test]
fn canonical_relabel_keeps_the_composite_literal_text() {
    let relabeled = canonical_relabel(
        &blank_subject_list("x", "[_:x, 42, _:x]"),
        &mut purrdf_core::CanonicalizationControl::new(purrdf_core::CANON_CORPUS_POLICY, None),
    )
    .expect("an admissible dataset");
    assert_eq!(sole_literal(&relabeled), "[_:x, 42, _:x]");
    let subject = relabeled.quads().next().map(|q| q.s).expect("one quad");
    assert_eq!(
        relabeled.resolve(subject),
        TermRef::Blank {
            label: "0",
            scope: BlankScope::DEFAULT
        }
    );
}

/// Relabeling twice is the identity on the second pass, and never disturbs the
/// isomorphism class.
#[test]
fn canonical_relabel_is_idempotent_over_composite_literals() {
    let source = blank_subject_list("x", "[_:x, 42, [_:x], _:y]");
    let once = canonical_relabel(
        &source,
        &mut purrdf_core::CanonicalizationControl::new(purrdf_core::CANON_CORPUS_POLICY, None),
    )
    .expect("admissible");
    let twice = canonical_relabel(
        &once,
        &mut purrdf_core::CanonicalizationControl::new(purrdf_core::CANON_CORPUS_POLICY, None),
    )
    .expect("admissible");
    assert_eq!(sole_literal(&once), sole_literal(&twice));
    assert!(
        datasets_isomorphic(
            &source,
            &once,
            &mut purrdf_core::CanonicalizationControl::new(purrdf_core::CANON_CORPUS_POLICY, None)
        )
        .unwrap()
    );
}

// ── Skolemization round trip ────────────────────────────────────────────────

/// Skolemizing turns an embedded blank into a genid IRI and de-skolemizing turns
/// it back, so the pair stays invertible over composite literals.
#[test]
fn skolemization_round_trips_over_an_embedded_blank() {
    let source = blank_subject_list("x", "[_:x, 42]");
    let skolemized = skolemize(&source, "http://example.org").expect("no reserved genid");
    let lexical = sole_literal(&skolemized);
    assert!(
        lexical.contains("/.well-known/genid/"),
        "the embedded blank was not skolemized: {lexical}"
    );
    assert!(
        purrdf_core::cdt_blank::cdt_embedded_blanks(&lexical, LIST).is_empty(),
        "a blank survived skolemization: {lexical}"
    );

    let back = deskolemize(&skolemized, "http://example.org").expect("invertible");
    assert!(
        datasets_isomorphic(
            &source,
            &back,
            &mut purrdf_core::CanonicalizationControl::new(purrdf_core::CANON_CORPUS_POLICY, None)
        )
        .unwrap(),
        "skolemize/deskolemize is not the identity over composite literals"
    );
}

/// A rewrite writes every IRI element it touches in the composite canonical
/// `IRIREF` form, so an apostrophe (legal raw in an `IRIREF`) stays raw and a
/// plain IRI is byte-identical.
#[test]
fn a_rewritten_composite_keeps_iri_elements_in_their_canonical_form() {
    let source = blank_subject_list(
        "x",
        "[_:x, <http://example.org/it's>, <http://example.org/a>]",
    );
    let skolemized = skolemize(&source, "http://example.org").expect("no reserved genid");
    let lexical = sole_literal(&skolemized);
    assert!(
        lexical.contains("<http://example.org/it's>"),
        "an apostrophe was escaped: {lexical}"
    );
    assert!(lexical.contains("<http://example.org/a>"), "{lexical}");
    assert!(!lexical.contains("\\u0027"), "{lexical}");
}

/// The lexical form of the sole literal object in a one-quad dataset.
fn sole_literal(ds: &RdfDataset) -> String {
    let object = ds.quads().next().map(|q| q.o).expect("one quad");
    match ds.resolve(object) {
        TermRef::Literal { lexical, .. } => lexical.to_owned(),
        other => panic!("expected a literal object, got {other:?}"),
    }
}
