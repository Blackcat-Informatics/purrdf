// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! A blank node written inside a `cdt:List` / `cdt:Map` literal is a blank node
//! OF THE GRAPH, so it must participate in RDFC-1.0 canonicalization, in the
//! isomorphism oracle built on it, and in every whole-dataset term rewrite.
//!
//! These tests exercise that from the public surface, in both directions: a
//! consistent renaming of an embedded blank node must compare isomorphic, and a
//! change that conflates or splits blank-node identity must not.

use std::sync::Arc;

use purrdf_core::{
    BlankScope, RdfDataset, RdfDatasetBuilder, RdfLiteral, TermRef, canonical_relabel,
    canonicalize, datasets_isomorphic, deskolemize, skolemize,
};

const LIST: &str = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/List";
const P: &str = "http://example.org/p";
const S: &str = "http://example.org/s";

/// `ex:s ex:p "<lexical>"^^cdt:List .`
fn list_object(lexical: &str) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri(S);
    let p = b.intern_iri(P);
    let o = b.intern_literal(RdfLiteral::typed(lexical, LIST));
    b.push_quad(s, p, o, None);
    b.freeze().expect("a structurally valid one-quad dataset")
}

/// `_:{subject} ex:p "<lexical>"^^cdt:List .`
fn blank_subject_list(subject: &str, lexical: &str) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_blank(subject, BlankScope::DEFAULT);
    let p = b.intern_iri(P);
    let o = b.intern_literal(RdfLiteral::typed(lexical, LIST));
    b.push_quad(s, p, o, None);
    b.freeze().expect("a structurally valid one-quad dataset")
}

// ── Isomorphism: the positive direction ─────────────────────────────────────

/// Two datasets differing ONLY by a consistent renaming of a blank node that
/// occurs inside a composite literal are isomorphic. Treating the label as
/// literal text would call them different.
#[test]
fn a_consistent_renaming_inside_a_composite_literal_is_isomorphic() {
    let a = list_object("[_:x, 42]");
    let b = list_object("[_:y, 42]");
    assert!(datasets_isomorphic(&a, &b));
    assert!(datasets_isomorphic(&b, &a));
}

/// The renaming stays consistent across every occurrence, including a repeated
/// one and one reached through a nested composite (`bnodes-turtle-21`).
#[test]
fn a_consistent_renaming_holds_across_repeats_and_nesting() {
    let a = list_object("[_:x, 42, [_:x], _:x]");
    let b = list_object("[_:q, 42, [_:q], _:q]");
    assert!(datasets_isomorphic(&a, &b));
}

/// A label shared by a term-position blank and an embedded one renames as one
/// node (`bnodes-turtle-05`: the subject and the element are the same node).
#[test]
fn a_renaming_spanning_the_literal_boundary_is_isomorphic() {
    let a = blank_subject_list("x", "[_:x, 42]");
    let b = blank_subject_list("y", "[_:y, 42]");
    assert!(datasets_isomorphic(&a, &b));
}

// ── Isomorphism: the negative direction ─────────────────────────────────────

/// Splitting one embedded blank node into two is NOT a renaming
/// (`bnodes-turtle-01` says `[_:b, _:b]` holds one node; `bnodes-turtle-03` says
/// `[_:b1, _:b2]` holds two).
#[test]
fn splitting_an_embedded_blank_node_is_not_isomorphic() {
    let one = list_object("[_:x, 42, _:x]");
    let two = list_object("[_:x, 42, _:y]");
    assert!(!datasets_isomorphic(&one, &two));
    assert!(!datasets_isomorphic(&two, &one));
}

/// Conflating the subject with the embedded element is not a renaming either:
/// `bnodes-turtle-05` asserts `?s = ?e1` for the shared label and
/// `bnodes-turtle-07` asserts `?s != ?e1` for distinct ones, so the two datasets
/// are structurally different.
#[test]
fn conflating_across_the_literal_boundary_is_not_isomorphic() {
    let shared = blank_subject_list("x", "[_:x, 42]");
    let split = blank_subject_list("x", "[_:y, 42]");
    assert!(!datasets_isomorphic(&shared, &split));
    assert!(!datasets_isomorphic(&split, &shared));
}

/// A change to the literal's GROUND content is still a difference — the blank
/// handling must not make the rest of the lexical form invisible.
#[test]
fn ground_content_still_counts() {
    assert!(!datasets_isomorphic(
        &list_object("[_:x, 42]"),
        &list_object("[_:x, 43]")
    ));
}

/// Adding an embedded blank node changes the blank-node count, so the cheap
/// structural pre-check must see it too.
#[test]
fn an_embedded_blank_node_counts_as_a_blank_node() {
    assert!(!datasets_isomorphic(
        &list_object("[42]"),
        &list_object("[_:x]")
    ));
}

// ── Canonicalization ────────────────────────────────────────────────────────

/// Isomorphic datasets canonicalize to byte-equal N-Quads, and the canonical
/// form carries the ISSUED labels rather than the input ones.
#[test]
fn the_canonical_form_carries_issued_labels_inside_the_literal() {
    let a = canonicalize(&list_object("[_:x, 42]")).nquads;
    let b = canonicalize(&list_object("[_:y, 42]")).nquads;
    assert_eq!(a, b);
    assert!(a.contains("c14n0"), "canonical N-Quads: {a}");
    assert!(
        !a.contains("_:x") && !a.contains("_:y"),
        "leaked input: {a}"
    );
}

/// `canonical_relabel` rewrites the embedded occurrence too, so no label in the
/// output dataset dangles: every blank the relabeled literal names resolves to a
/// blank node the relabeled dataset actually holds.
#[test]
fn canonical_relabel_leaves_no_dangling_embedded_label() {
    let relabeled = canonical_relabel(&blank_subject_list("x", "[_:x, 42, _:x]"))
        .expect("an admissible dataset");
    let lexical = sole_literal(&relabeled);
    let names = purrdf_core::cdt_blank::cdt_embedded_blanks(&lexical, LIST);
    assert_eq!(names.len(), 2, "both occurrences survive: {lexical}");
    for (label, scope) in &names {
        assert!(
            relabeled.term_id_by_blank(label, *scope).is_some(),
            "embedded label {label:?} dangles in {lexical}"
        );
        assert!(
            label.starts_with("c14n"),
            "input label leaked into {lexical}"
        );
    }
    // And the embedded node is still the SAME node as the subject.
    let subject = relabeled.quads().next().map(|q| q.s).expect("one quad");
    let embedded = relabeled
        .term_id_by_blank(&names[0].0, names[0].1)
        .expect("resolved above");
    assert_eq!(subject, embedded);
}

/// Relabeling twice is the identity on the second pass, and never disturbs the
/// isomorphism class.
#[test]
fn canonical_relabel_is_idempotent_over_embedded_blanks() {
    let source = list_object("[_:x, 42, [_:x], _:y]");
    let once = canonical_relabel(&source).expect("admissible");
    let twice = canonical_relabel(&once).expect("admissible");
    assert_eq!(sole_literal(&once), sole_literal(&twice));
    assert!(datasets_isomorphic(&source, &once));
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
        datasets_isomorphic(&source, &back),
        "skolemize/deskolemize is not the identity over composite literals"
    );
}

/// The lexical form of the sole literal object in a one-quad dataset.
fn sole_literal(ds: &RdfDataset) -> String {
    let object = ds.quads().next().map(|q| q.o).expect("one quad");
    match ds.resolve(object) {
        TermRef::Literal { lexical, .. } => lexical.to_owned(),
        other => panic!("expected a literal object, got {other:?}"),
    }
}
