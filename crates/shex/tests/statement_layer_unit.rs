// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! ShEx over the **RDF 1.2 statement layer**: reifiers and statement
//! annotations.
//!
//! RDF 1.2 reifier bindings and statement annotations live in side-tables that
//! are NOT part of the frozen dataset's quad table. These tests pin that the
//! validator and the shape-map resolver both see that layer:
//!
//! * a shape-map node selector over an annotation predicate (or over
//!   `rdf:reifies`) selects the reifier, in both directions;
//! * a focus node that IS a reifier has a real neighbourhood — its
//!   `rdf:reifies` arc plus one arc per statement annotation — including when
//!   the same node is also an ordinary subject;
//! * inverse arcs reach it too (`^rdf:reifies` from a triple term, and
//!   `^<annotationPredicate>` from an annotation object);
//! * `CLOSED` over a reifier is really checked rather than passing vacuously;
//! * ordinary (non-reifier) neighbourhoods are unchanged.
//!
//! These are the RDF 1.2 *statement* annotations, not the unrelated ShEx
//! *schema* annotations (`// predicate object`).

use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral, TermValue};
use purrdf_shex::{
    ConformanceStatus, ShapeSelector, ValidationOptions, parse_shape_map, parse_shexc,
    resolve_shape_map, validate,
};

const ALICE: &str = "http://example.org/alice";
const BOB: &str = "http://example.org/bob";
const DOC: &str = "http://example.org/doc";
const KNOWS: &str = "http://example.org/knows";
const NAME: &str = "http://example.org/name";
const LABEL: &str = "http://example.org/label";
const CERTAINTY: &str = "http://example.org/certainty";
const SOURCE: &str = "http://example.org/source";
const R1: &str = "http://example.org/r1";
const R2: &str = "http://example.org/r2";
const R3: &str = "http://example.org/r3";
const REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

/// The fixture graph.
///
/// Base quads (the only rows the `quads` table holds):
///
/// ```text
/// ex:alice ex:knows ex:bob .
/// ex:alice ex:name  "Alice" .
/// ex:r3    ex:label "R3" .
/// ```
///
/// Statement layer (side-tables), over `T1 = <<ex:alice ex:knows ex:bob>>` and
/// `T2 = <<ex:alice ex:name "Alice">>`:
///
/// ```text
/// ex:r1 rdf:reifies T1 ;  ex:certainty "0.9" ; ex:source ex:doc .
/// ex:r2 rdf:reifies T1 ;  ex:certainty "0.4" .
/// ex:r3 rdf:reifies T2 .
/// ```
///
/// `ex:r3` is deliberately BOTH an ordinary subject and a reifier, so the
/// merged neighbourhood is exercised.
fn data() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let alice = b.intern_iri(ALICE);
    let bob = b.intern_iri(BOB);
    let doc = b.intern_iri(DOC);
    let knows = b.intern_iri(KNOWS);
    let name = b.intern_iri(NAME);
    let label = b.intern_iri(LABEL);
    let certainty = b.intern_iri(CERTAINTY);
    let source = b.intern_iri(SOURCE);
    let r1 = b.intern_iri(R1);
    let r2 = b.intern_iri(R2);
    let r3 = b.intern_iri(R3);
    let alice_name = b.intern_literal(RdfLiteral::simple("Alice"));
    let r3_label = b.intern_literal(RdfLiteral::simple("R3"));
    let high = b.intern_literal(RdfLiteral::simple("0.9"));
    let low = b.intern_literal(RdfLiteral::simple("0.4"));

    b.push_quad(alice, knows, bob, None);
    b.push_quad(alice, name, alice_name, None);
    b.push_quad(r3, label, r3_label, None);

    let t1 = b.intern_triple(alice, knows, bob);
    let t2 = b.intern_triple(alice, name, alice_name);
    b.push_reifier(r1, t1);
    b.push_reifier(r2, t1);
    b.push_reifier(r3, t2);
    b.push_annotation(r1, certainty, high);
    b.push_annotation(r1, source, doc);
    b.push_annotation(r2, certainty, low);

    b.freeze().expect("fixture dataset must validate")
}

/// `T1 = <<ex:alice ex:knows ex:bob>>` as a value.
fn t1_value() -> TermValue {
    TermValue::Triple {
        s: Box::new(TermValue::iri(ALICE)),
        p: Box::new(TermValue::iri(KNOWS)),
        o: Box::new(TermValue::iri(BOB)),
    }
}

/// Validate `node` against the single shape declared by `schema_src`.
fn check(schema_src: &str, label: &str, node: TermValue) -> Result<(), String> {
    let schema = parse_shexc(schema_src, None).expect("schema parses");
    let data = data();
    let map = vec![(node, ShapeSelector::Label(label.to_owned()))];
    let result = validate(&schema, &data, &map);
    assert_eq!(result.entries.len(), 1);
    match result.entries[0].status {
        ConformanceStatus::Conformant => Ok(()),
        ConformanceStatus::Nonconformant => {
            Err(result.entries[0].reason.clone().unwrap_or_default())
        }
    }
}

/// The nodes a shape-map selector picks out, in the resolver's deterministic
/// order.
fn selected(map_src: &str) -> Vec<TermValue> {
    let data = data();
    let map = parse_shape_map(map_src, None).expect("shape map parses");
    resolve_shape_map(&map, &data)
        .into_iter()
        .map(|(node, _)| node)
        .collect()
}

// ── shape-map selectors (the originally reported symptom) ────────────────────

#[test]
fn selector_over_annotation_predicate_selects_reifiers() {
    // Pre-fix this selected NOTHING: annotations are invisible to `quads`.
    assert_eq!(
        selected(&format!("{{FOCUS <{CERTAINTY}> _}}@START")),
        vec![TermValue::iri(R1), TermValue::iri(R2)],
    );
}

#[test]
fn selector_over_annotation_predicate_object_direction() {
    assert_eq!(
        selected(&format!("{{_ <{SOURCE}> FOCUS}}@START")),
        vec![TermValue::iri(DOC)],
    );
    // Anchored on the reifier subject (the indexed run), not a full scan.
    assert_eq!(
        selected(&format!("{{<{R1}> <{SOURCE}> FOCUS}}@START")),
        vec![TermValue::iri(DOC)],
    );
    assert_eq!(
        selected(&format!("{{<{R2}> <{SOURCE}> FOCUS}}@START")),
        Vec::<TermValue>::new(),
    );
}

#[test]
fn selector_over_annotation_predicate_with_object_anchor() {
    assert_eq!(
        selected(&format!("{{FOCUS <{CERTAINTY}> \"0.4\"}}@START")),
        vec![TermValue::iri(R2)],
    );
}

#[test]
fn selector_over_rdf_reifies_both_directions() {
    // Every reifier, sorted by term string.
    assert_eq!(
        selected(&format!("{{FOCUS <{REIFIES}> _}}@START")),
        vec![TermValue::iri(R1), TermValue::iri(R2), TermValue::iri(R3),],
    );
    // The object direction yields the two reified triple terms.
    assert_eq!(
        selected(&format!("{{_ <{REIFIES}> FOCUS}}@START")),
        vec![
            t1_value(),
            TermValue::Triple {
                s: Box::new(TermValue::iri(ALICE)),
                p: Box::new(TermValue::iri(NAME)),
                o: Box::new(TermValue::simple_literal("Alice")),
            },
        ],
    );
    // Anchoring the object on a concrete triple term picks its reifiers.
    assert_eq!(
        selected(&format!(
            "{{FOCUS <{REIFIES}> << <{ALICE}> <{KNOWS}> <{BOB}> >>}}@START"
        )),
        vec![TermValue::iri(R1), TermValue::iri(R2)],
    );
}

#[test]
fn selector_over_ordinary_predicate_is_unchanged() {
    assert_eq!(
        selected(&format!("{{FOCUS <{KNOWS}> _}}@START")),
        vec![TermValue::iri(ALICE)],
    );
    assert_eq!(
        selected(&format!("{{_ <{KNOWS}> FOCUS}}@START")),
        vec![TermValue::iri(BOB)],
    );
}

// ── a reifier as the focus node ──────────────────────────────────────────────

#[test]
fn reifier_focus_matches_reifies_and_annotation_arcs() {
    // Pre-fix the neighbourhood was EMPTY, so the required arcs were missing.
    let schema = format!(
        "<http://example.org/Stmt> {{ <{REIFIES}> . ; <{CERTAINTY}> LITERAL ; <{SOURCE}> IRI ? }}"
    );
    check(&schema, "http://example.org/Stmt", TermValue::iri(R1))
        .expect("r1 has rdf:reifies, ex:certainty and ex:source arcs");
    check(&schema, "http://example.org/Stmt", TermValue::iri(R2))
        .expect("r2 has rdf:reifies and ex:certainty; ex:source is optional");
}

#[test]
fn reifier_focus_value_expression_sees_the_triple_term() {
    // The `rdf:reifies` object is the triple term, so a NONLITERAL value
    // expression holds while a LITERAL one must fail.
    let ok = format!("<http://example.org/Stmt> {{ <{REIFIES}> NONLITERAL }}");
    check(&ok, "http://example.org/Stmt", TermValue::iri(R2)).expect("a triple term is nonliteral");

    let bad = format!("<http://example.org/Stmt> {{ <{REIFIES}> LITERAL }}");
    let err = check(&bad, "http://example.org/Stmt", TermValue::iri(R2))
        .expect_err("a triple term is not a literal");
    assert!(err.contains(REIFIES), "reason: {err}");
}

#[test]
fn reifier_that_is_also_an_ordinary_subject_merges_both_layers() {
    // `ex:r3` carries an ordinary `ex:label` quad AND a reifier binding; the
    // neighbourhood is the union of the two.
    let schema = format!("<http://example.org/Both> {{ <{REIFIES}> . ; <{LABEL}> LITERAL }}");
    check(&schema, "http://example.org/Both", TermValue::iri(R3))
        .expect("both the quad-table arc and the statement-layer arc are present");
}

#[test]
fn non_reifier_focus_gains_no_statement_arcs() {
    // Both statement-layer views key on the REIFIER as subject, so an ordinary
    // subject's neighbourhood is untouched — `ex:alice` is the subject of the
    // reified triple, never a reifier.
    let schema = format!("<http://example.org/User> CLOSED {{ <{KNOWS}> IRI ; <{NAME}> LITERAL }}");
    check(&schema, "http://example.org/User", TermValue::iri(ALICE))
        .expect("alice's neighbourhood is exactly its two quad-table arcs");
}

// ── inverse arcs ─────────────────────────────────────────────────────────────

#[test]
fn inverse_rdf_reifies_from_a_triple_term_finds_its_reifiers() {
    let schema = format!("<http://example.org/Reified> {{ ^<{REIFIES}> IRI {{2}} }}");
    check(&schema, "http://example.org/Reified", t1_value())
        .expect("T1 is reified by exactly ex:r1 and ex:r2");

    let wrong = format!("<http://example.org/Reified> {{ ^<{REIFIES}> IRI {{3}} }}");
    check(&wrong, "http://example.org/Reified", t1_value()).expect_err("T1 has only two reifiers");
}

#[test]
fn inverse_annotation_predicate_from_an_annotation_object_finds_the_reifier() {
    let schema = format!("<http://example.org/Doc> {{ ^<{SOURCE}> IRI }}");
    check(&schema, "http://example.org/Doc", TermValue::iri(DOC))
        .expect("ex:doc is the ex:source annotation object of exactly ex:r1");
}

#[test]
fn inverse_arcs_over_an_ordinary_predicate_are_unchanged() {
    let schema = format!("<http://example.org/Known> {{ ^<{KNOWS}> IRI }}");
    check(&schema, "http://example.org/Known", TermValue::iri(BOB))
        .expect("bob is known by exactly alice");
}

// ── CLOSED over a reifier (the behaviour change) ─────────────────────────────

#[test]
fn closed_over_a_reifier_no_longer_passes_vacuously() {
    // BEHAVIOUR CHANGE: pre-fix a reifier's neighbourhood was empty, so this
    // CLOSED shape passed vacuously. Post-fix the `rdf:reifies` arc is real and
    // the shape does not mention it, so CLOSED rejects — correct semantics.
    let schema = format!("<http://example.org/Closed> CLOSED {{ <{CERTAINTY}> LITERAL ? }}");
    let err = check(&schema, "http://example.org/Closed", TermValue::iri(R2))
        .expect_err("a CLOSED shape over a reifier must mention rdf:reifies");
    assert!(err.contains("CLOSED"), "reason: {err}");
    assert!(err.contains(REIFIES), "reason: {err}");
}

#[test]
fn closed_over_a_reifier_conforms_once_it_mentions_the_layer() {
    let schema =
        format!("<http://example.org/Closed> CLOSED {{ <{REIFIES}> . ; <{CERTAINTY}> LITERAL ? }}");
    check(&schema, "http://example.org/Closed", TermValue::iri(R2))
        .expect("r2's arcs are exactly rdf:reifies and ex:certainty");

    // ex:r1 also carries an ex:source annotation, which this CLOSED shape does
    // not mention.
    let err = check(&schema, "http://example.org/Closed", TermValue::iri(R1))
        .expect_err("ex:source is an unmentioned arc of ex:r1");
    assert!(err.contains(SOURCE), "reason: {err}");
}

// ── determinism ──────────────────────────────────────────────────────────────

#[test]
fn merged_arcs_are_order_stable_across_repeated_validations() {
    // The merged neighbourhood is sorted and de-duplicated, so repeated runs
    // produce byte-identical result shape maps.
    let schema = parse_shexc(
        &format!("<http://example.org/Stmt> {{ <{REIFIES}> . ; <{CERTAINTY}> LITERAL ? ; <{SOURCE}> IRI ? }}"),
        None,
    )
    .expect("schema parses");
    let map: Vec<(TermValue, ShapeSelector)> = [R1, R2, R3]
        .iter()
        .map(|iri| {
            (
                TermValue::iri(*iri),
                ShapeSelector::Label("http://example.org/Stmt".to_owned()),
            )
        })
        .collect();
    let first = validate(&schema, &data(), &map).to_result_json();
    for _ in 0..4 {
        assert_eq!(validate(&schema, &data(), &map).to_result_json(), first);
    }
    assert!(
        !first.contains("nonconformant"),
        "every reifier conforms: {first}"
    );
}

// ── a dataset with no statement layer is untouched ───────────────────────────

#[test]
fn plain_dataset_validates_unchanged() {
    let mut b = RdfDatasetBuilder::new();
    let alice = b.intern_iri(ALICE);
    let name = b.intern_iri(NAME);
    let alice_name = b.intern_literal(RdfLiteral::simple("Alice"));
    b.push_quad(alice, name, alice_name, None);
    let plain = b.freeze().expect("fixture dataset must validate");

    let schema = parse_shexc(
        &format!("<http://example.org/User> CLOSED {{ <{NAME}> LITERAL }}"),
        None,
    )
    .expect("schema parses");
    let map = vec![(
        TermValue::iri(ALICE),
        ShapeSelector::Label("http://example.org/User".to_owned()),
    )];
    let result = validate(&schema, &plain, &map);
    assert_eq!(result.entries[0].status, ConformanceStatus::Conformant);

    let sel = parse_shape_map(&format!("{{FOCUS <{NAME}> _}}@START"), None).expect("parses");
    assert_eq!(
        resolve_shape_map(&sel, &plain)
            .into_iter()
            .map(|(node, _)| node)
            .collect::<Vec<_>>(),
        vec![TermValue::iri(ALICE)],
    );
}

// ── options plumbing (validate_with parity) ──────────────────────────────────

#[test]
fn validate_with_default_options_matches_validate() {
    let schema = parse_shexc(
        &format!("<http://example.org/Stmt> {{ <{REIFIES}> . ; <{CERTAINTY}> LITERAL }}"),
        None,
    )
    .expect("schema parses");
    let data = data();
    let map = vec![(
        TermValue::iri(R1),
        ShapeSelector::Label("http://example.org/Stmt".to_owned()),
    )];
    let plain = validate(&schema, &data, &map);
    let with_options =
        purrdf_shex::validate_with(&schema, &data, &map, &ValidationOptions::default());
    assert_eq!(plain, with_options);
    assert!(plain.all_conformant());
}
