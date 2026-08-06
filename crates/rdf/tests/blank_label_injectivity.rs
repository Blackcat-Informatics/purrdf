// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Parsing distinct blank-node labels is INJECTIVE: a document whose `_:` tokens
//! are pairwise distinct always yields that many distinct blank nodes.
//!
//! This is the whole-stack statement of the blank-label codec's contract, driven
//! through the same surfaces a user reaches (`parse_dataset`, `serialize_dataset`,
//! RDFC-1.0 canonicalization) rather than through the codec functions directly — a conflation that the unit tests missed would still have
//! to survive here.
//!
//! The regression class it exists for: an egress transform that maps the legal
//! label alphabet onto a PROPER SUBSET of itself cannot be injective, so no
//! ingress decode can undo it. The old encoding doubled raw dots (`a.b` → `a..b`,
//! making the token `a..b` ambiguous) and decoded the reserved marker without an
//! image check (`purrdfesc_abc` → `abc`), which merged FIVE distinct legal labels
//! into three nodes with no diagnostic — silently changing what the data means,
//! including what it canonicalizes to. The SPARQL-visible half of the same probe
//! (`COUNT(DISTINCT ?s)` = 5) lives in `crates/purrdf/tests/blank_label_injectivity.rs`,
//! where the evaluator is in scope.

use std::collections::BTreeSet;
use std::sync::Arc;

use proptest::prelude::*;
use purrdf_rdf::{
    BlankScope, RdfDataset, RdfDatasetBuilder, SerializeGraph, TermRef, canonicalize,
    parse_dataset, serialize_dataset,
};

const NTRIPLES: &str = "application/n-triples";

/// The adversary's probe, verbatim: five DISTINCT legal `BLANK_NODE_LABEL`s, one
/// per predicate so a merge cannot hide behind quad deduplication.
const PROBE: &str = "_:a.b <https://example.org/p1> \"1\" .\n\
                     _:a..b <https://example.org/p2> \"2\" .\n\
                     _:a...b <https://example.org/p3> \"3\" .\n\
                     _:purrdfesc_abc <https://example.org/p4> \"4\" .\n\
                     _:abc <https://example.org/p5> \"5\" .\n";

/// The same five triples with only THREE distinct subjects — what the defective
/// encoding turned the probe into. Kept as an explicit non-isomorphic control.
const MERGED: &str = "_:a.b <https://example.org/p1> \"1\" .\n\
                      _:a.b <https://example.org/p2> \"2\" .\n\
                      _:a...b <https://example.org/p3> \"3\" .\n\
                      _:abc <https://example.org/p4> \"4\" .\n\
                      _:abc <https://example.org/p5> \"5\" .\n";

fn parse(text: &str) -> Arc<RdfDataset> {
    parse_dataset(text.as_bytes(), NTRIPLES, None)
        .unwrap_or_else(|e| panic!("N-Triples must parse: {e}\n{text}"))
}

fn serialize(dataset: &RdfDataset) -> String {
    let bytes = serialize_dataset(dataset, NTRIPLES, SerializeGraph::Dataset)
        .expect("every dataset serializes");
    String::from_utf8(bytes).expect("native text output is UTF-8")
}

/// The distinct blank `(label, scope)` pairs a dataset holds, read straight off
/// the IR.
fn blank_nodes(dataset: &RdfDataset) -> BTreeSet<(String, u32)> {
    let mut blanks = BTreeSet::new();
    for quad in dataset.quads() {
        for id in [quad.s, quad.o] {
            if let TermRef::Blank { label, scope } = dataset.resolve(id) {
                blanks.insert((label.to_owned(), scope.ordinal()));
            }
        }
    }
    blanks
}

#[test]
fn the_five_label_probe_parses_to_five_distinct_nodes() {
    let ds = parse(PROBE);
    assert_eq!(ds.quad_count(), 5, "five quads");
    assert_eq!(
        blank_nodes(&ds),
        BTreeSet::from([
            ("a.b".to_owned(), 0),
            ("a..b".to_owned(), 0),
            ("a...b".to_owned(), 0),
            ("purrdfesc_abc".to_owned(), 0),
            ("abc".to_owned(), 0),
        ]),
        "each token must intern VERBATIM at the default scope"
    );
}

#[test]
fn the_five_label_probe_is_a_byte_fixpoint_from_the_first_write() {
    let gen1 = serialize(&parse(PROBE));
    let gen2 = serialize(&parse(&gen1));
    assert_eq!(gen1, gen2, "convert is a byte fixpoint from gen1");
    let gen3 = serialize(&parse(&gen2));
    assert_eq!(gen2, gen3, "…and stays one");

    // Four of the five labels are outside the reserved namespace, so their bytes
    // never moved at all; only the marker-prefixed one is enveloped, once.
    for token in ["_:a.b ", "_:a..b ", "_:a...b ", "_:abc "] {
        assert!(
            gen1.contains(token),
            "{token:?} must survive verbatim: {gen1}"
        );
    }
    assert!(
        gen1.contains("_:purrdfesc_purrdfesc_00005Fabc "),
        "the marker-prefixed label is enveloped exactly once: {gen1}"
    );
    assert_eq!(
        blank_nodes(&parse(&gen1)).len(),
        5,
        "the fixpoint document still holds five nodes"
    );
}

#[test]
fn the_five_label_probe_canonicalizes_apart_from_its_merged_variant() {
    // RDFC-1.0 mints the dataset's content-addressed identity, so a conflation
    // on the parse path would make a five-node graph canonicalize identically to
    // a genuinely three-node one — silently changing what the data IS.
    let probe = canonicalize(&parse(PROBE));
    let merged = canonicalize(&parse(MERGED));
    assert_eq!(probe.labels.len(), 5, "five blank nodes to canonicalize");
    assert_eq!(merged.labels.len(), 3, "the control really has three");
    assert_ne!(
        probe.nquads, merged.nquads,
        "two non-isomorphic datasets must not canonicalize to identical bytes"
    );
    // Canonicalization is itself stable across the round trip.
    assert_eq!(
        probe.nquads,
        canonicalize(&parse(&serialize(&parse(PROBE)))).nquads
    );
}

#[test]
fn a_scoped_pair_round_trips_through_its_envelope() {
    // `("x", scope 2)` has no verbatim spelling — a scope has to be carried
    // somewhere — so it is written as its envelope and re-parses to that same
    // pair, while the literal labels `x` and `x.s2` stay untouched beside it.
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri("https://example.org/p");
    let scoped = b.intern_blank("x", BlankScope(2));
    let plain = b.intern_blank("x", BlankScope::DEFAULT);
    let suffixed = b.intern_blank("x.s2", BlankScope::DEFAULT);
    for (i, s) in [scoped, plain, suffixed].into_iter().enumerate() {
        let o = b.intern_iri(&format!("https://example.org/o{i}"));
        b.push_quad(s, p, o, None);
    }
    let ds = b.freeze().expect("dataset freezes");

    let text = serialize(&ds);
    assert!(
        text.contains("_:purrdfesc2_x "),
        "the scoped pair is written as its envelope: {text}"
    );
    assert!(
        text.contains("_:x "),
        "the literal label is verbatim: {text}"
    );
    assert!(
        text.contains("_:x.s2 "),
        "the suffix-shaped literal label is verbatim: {text}"
    );

    let reparsed = parse(&text);
    assert_eq!(
        blank_nodes(&reparsed),
        BTreeSet::from([
            ("x".to_owned(), 2),
            ("x".to_owned(), 0),
            ("x.s2".to_owned(), 0),
        ]),
        "three distinct nodes, each restored to its exact (label, scope): {text}"
    );
    assert_eq!(serialize(&reparsed), text, "and the bytes never move");
}

#[test]
fn standardize_apart_keeps_per_document_blanks_distinct() {
    // Two documents that both spell `_:b0` merge into TWO nodes (C0.2), and the
    // merged dataset serializes to a document that re-parses to two nodes.
    let one = parse("_:b0 <https://example.org/p> \"1\" .\n");
    let two = parse("_:b0 <https://example.org/p> \"2\" .\n");
    let mut merged = RdfDatasetBuilder::new();
    merged.push_dataset(&one);
    merged.push_dataset(&two);
    let merged = merged.freeze().expect("merge freezes");
    assert_eq!(
        blank_nodes(&merged).len(),
        2,
        "standardize-apart keeps the two `_:b0`s distinct"
    );

    let text = serialize(&merged);
    let reparsed = parse(&text);
    assert_eq!(
        blank_nodes(&reparsed),
        blank_nodes(&merged),
        "the scope envelopes restore both pairs exactly: {text}"
    );
    assert_eq!(serialize(&reparsed), text);
}

/// A generator over labels that are legal `BLANK_NODE_LABEL`s and cover every
/// class the encoding reasons about: plain, dotted (in every run length),
/// scope-suffix-shaped, and marker-prefixed.
fn arb_legal_label() -> impl Strategy<Value = String> {
    proptest::sample::select(vec![
        "abc".to_owned(),
        "b0".to_owned(),
        "c14n0".to_owned(),
        "a.b".to_owned(),
        "a..b".to_owned(),
        "a...b".to_owned(),
        "a.b.c".to_owned(),
        "x.s1".to_owned(),
        "x.s01".to_owned(),
        "s0.b0".to_owned(),
        "purrdfesc".to_owned(),
        "purrdfesc_abc".to_owned(),
        "purrdfesc1_a".to_owned(),
        "purrdfesc_a_000020b".to_owned(),
        "0abc".to_owned(),
        "_x".to_owned(),
        "日本".to_owned(),
    ])
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, ..ProptestConfig::default() })]

    /// PARSE IS INJECTIVE: a document listing any SET of distinct legal labels —
    /// each on its own predicate, so nothing can merge by deduplication — yields
    /// exactly that many distinct blank nodes, every token outside the reserved
    /// marker namespace interns as the label it spells, and serialize ∘ parse ∘
    /// serialize is a byte fixpoint from gen1.
    #[test]
    fn a_document_of_distinct_labels_parses_to_that_many_nodes(
        labels in proptest::collection::btree_set(arb_legal_label(), 1..8)
    ) {
        let mut document = String::new();
        for (i, label) in labels.iter().enumerate() {
            use std::fmt::Write as _;
            let _ = writeln!(document, "_:{label} <https://example.org/p{i}> \"{i}\" .");
        }
        let ds = parse(&document);
        let nodes = blank_nodes(&ds);
        prop_assert_eq!(
            nodes.len(),
            labels.len(),
            "parse must be injective over {}\ngot {:?}", document, nodes
        );

        // Every token outside the reserved marker namespace — which is every
        // token any foreign document carries — denotes ITSELF at the default
        // scope, byte for byte.
        for label in &labels {
            if label.starts_with("purrdfesc") {
                continue;
            }
            prop_assert!(
                nodes.contains(&(label.clone(), 0u32)),
                "{} must intern verbatim: {:?}", label, nodes
            );
        }

        // …and the document settles after one write.
        let gen1 = serialize(&ds);
        let gen2 = serialize(&parse(&gen1));
        prop_assert_eq!(&gen1, &gen2, "not a byte fixpoint for {}", document);
        prop_assert_eq!(
            blank_nodes(&parse(&gen2)).len(),
            labels.len(),
            "the fixpoint document lost a node: {}", gen2
        );
    }
}
