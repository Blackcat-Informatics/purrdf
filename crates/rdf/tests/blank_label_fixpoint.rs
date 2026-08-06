// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Text egress is a FIXED POINT: `serialize(parse(serialize(D)))` is
//! byte-identical to `serialize(D)`, for every dataset and every native text
//! format that has a parser.
//!
//! The egress encoding a serializer applies to a blank node
//! (`blank_label::encode_blank_label`) is injective and therefore NOT
//! idempotent on the reserved marker namespace: enveloping an envelope is not
//! the same as enveloping once. That is only sound because ingress inverts it
//! exactly (`blank_label::decode_blank_label`, image-checked). Without the
//! inverse, every serialize/parse cycle would re-encode an already-encoded
//! label and the markers would layer (`purrdfesc_purrdfesc_…`), so a document
//! that merely passes through a conversion pipeline would grow without bound
//! and its blank nodes would stop denoting the nodes they came from.
//!
//! Three layers are pinned here:
//!
//! 1. **Byte-fixpoint property** — arbitrary datasets over a hostile label
//!    generator (interior dots, envelope look-alikes, non-default scopes,
//!    out-of-alphabet scalars, marker collisions), crossed with every media
//!    type in the native codec registry.
//! 2. **Label identity** — a well-formed round trip restores the exact
//!    `(label, scope)` pair, not merely an isomorphic relabeling.
//! 3. **Named regressions** — `_:a.b` and `_:x` at scope 2 through N-Triples and
//!    Turtle, byte-stable across three full cycles.

use std::collections::BTreeSet;
use std::sync::Arc;

use proptest::prelude::*;
use purrdf_rdf::{
    BlankScope, NativeRdfFormat, RdfDataset, RdfDatasetBuilder, SerializeGraph, TermRef,
    parse_dataset, serialize_dataset,
};

/// Serialize a dataset to `format`'s text, or explain which format refused.
fn serialize(dataset: &RdfDataset, format: NativeRdfFormat) -> Vec<u8> {
    serialize_dataset(dataset, format.media_type(), SerializeGraph::Dataset)
        .unwrap_or_else(|e| panic!("{} must serialize: {e}", format.media_type()))
}

/// Parse `format`'s text back into a frozen dataset.
fn parse(bytes: &[u8], format: NativeRdfFormat) -> Arc<RdfDataset> {
    parse_dataset(bytes, format.media_type(), None).unwrap_or_else(|e| {
        panic!(
            "{} output must re-parse: {e}\n{}",
            format.media_type(),
            String::from_utf8_lossy(bytes)
        )
    })
}

/// The distinct blank `(label, scope)` pairs a dataset holds, straight off the
/// IR (never through the owned rendering, which would re-encode them).
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

/// A one-quad dataset whose subject is a blank node with the given raw
/// `(label, scope)`.
fn blank_subject_dataset(label: &str, scope: BlankScope) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_blank(label, scope);
    let p = b.intern_iri("https://example.org/p");
    let o = b.intern_iri("https://example.org/o");
    b.push_quad(s, p, o, None);
    b.freeze().expect("dataset freezes")
}

/// Raw blank labels a producer can hand the serializer, chosen to attack the
/// encoding: every dot position, strings that MIMIC an envelope, the reserved
/// marker namespace, and scalars outside each target alphabet.
const HOSTILE_LABELS: &[&str] = &[
    "b0",
    "a.b",
    "a.b.c",
    "a..b",
    "trailing.",
    ".lead",
    "a.s1",
    "a..s1",
    "x.s2",
    "a...b",
    "purrdfesc",
    "purrdfesc1_a",
    "purrdfesc_abc",
    "abc",
    "s0.b0",
    "c1.s5",
    "0abc",
    "-lead",
    "a b",
    "a\nb",
    "bad\u{1f}label",
    "<urn:x>",
    "\u{d7}y",
    "日本",
    "purrdfesc_a",
    "purrdfesc_a_000020b",
    "purrdfesc__12",
    "c14n0",
    "",
];

/// A hostile `(label, scope)` pair: any label from the table above at any of the
/// first few scopes, including the default one.
fn arb_blank() -> impl Strategy<Value = (String, u32)> {
    (
        proptest::sample::select(HOSTILE_LABELS).prop_map(str::to_owned),
        0u32..4,
    )
}

/// A dataset of one to four quads whose subjects and objects are drawn from the
/// hostile blank pool (plus an IRI object, so a quad can be blank-free).
fn arb_dataset() -> impl Strategy<Value = Arc<RdfDataset>> {
    proptest::collection::vec((arb_blank(), arb_blank(), 0usize..3), 1..5).prop_map(|rows| {
        let mut b = RdfDatasetBuilder::new();
        let p = b.intern_iri("https://example.org/p");
        for ((subject_label, subject_scope), (object_label, object_scope), object_kind) in rows {
            let s = b.intern_blank(&subject_label, BlankScope(subject_scope));
            let o = match object_kind {
                0 => b.intern_blank(&object_label, BlankScope(object_scope)),
                1 => b.intern_iri("https://example.org/o"),
                // A plain literal object, so a quad can end in something that is
                // not a node at all. The lexical form is deliberately benign:
                // this gate is about blank LABELS, and a control character in a
                // literal is a separate (XML-representability) concern.
                _ => b.intern_literal(purrdf_rdf::RdfLiteral {
                    lexical_form: "value".to_owned(),
                    datatype: None,
                    language: None,
                    direction: None,
                }),
            };
            b.push_quad(s, p, o, None);
        }
        b.freeze().expect("dataset freezes")
    })
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, ..ProptestConfig::default() })]

    /// The load-bearing property: one serialize/parse cycle changes NO bytes, in
    /// every format the registry can both write and read.
    #[test]
    fn text_egress_is_a_byte_fixpoint(dataset in arb_dataset()) {
        for format in NativeRdfFormat::all() {
            let first = serialize(dataset.as_ref(), format);
            let reparsed = parse(&first, format);
            let second = serialize(reparsed.as_ref(), format);
            prop_assert_eq!(
                String::from_utf8_lossy(&first).into_owned(),
                String::from_utf8_lossy(&second).into_owned(),
                "{} is not a fixed point", format.media_type()
            );
        }
    }

    /// Stronger than isomorphism: the round trip returns the dataset's exact
    /// `(label, scope)` pairs, so a blank node still denotes the node it named.
    #[test]
    fn a_round_trip_restores_blank_label_identity(
        (label, scope) in arb_blank()
    ) {
        let scope = BlankScope(scope);
        let dataset = blank_subject_dataset(&label, scope);
        for format in NativeRdfFormat::all() {
            let bytes = serialize(dataset.as_ref(), format);
            let reparsed = parse(&bytes, format);
            prop_assert_eq!(
                blank_nodes(reparsed.as_ref()),
                BTreeSet::from([(label.clone(), scope.ordinal())]),
                "{} lost the label identity of {:?} @ {:?}\n{}",
                format.media_type(), label, scope, String::from_utf8_lossy(&bytes)
            );
        }
    }
}

/// Serialize `dataset`, then run `cycles` further parse→serialize passes,
/// asserting every pass produces the SAME bytes and the same `(label, scope)`
/// set. Returns the (stable) document text.
fn assert_stable_across_cycles(
    dataset: &RdfDataset,
    format: NativeRdfFormat,
    cycles: usize,
) -> String {
    let expected_blanks = blank_nodes(dataset);
    let first = serialize(dataset, format);
    let mut current = first.clone();
    for cycle in 1..=cycles {
        let reparsed = parse(&current, format);
        assert_eq!(
            blank_nodes(reparsed.as_ref()),
            expected_blanks,
            "{} cycle {cycle} changed the blank `(label, scope)` set",
            format.media_type()
        );
        let next = serialize(reparsed.as_ref(), format);
        assert_eq!(
            String::from_utf8_lossy(&first),
            String::from_utf8_lossy(&next),
            "{} cycle {cycle} changed the document bytes",
            format.media_type()
        );
        current = next;
    }
    String::from_utf8(first).expect("native text output is UTF-8")
}

/// Named regression: a raw interior dot (`_:a.b`) reaches the wire VERBATIM,
/// re-parses as the same label, and the bytes never move.
#[test]
fn dotted_label_is_stable_across_three_cycles() {
    let dataset = blank_subject_dataset("a.b", BlankScope::DEFAULT);
    for format in [NativeRdfFormat::NTriples, NativeRdfFormat::Turtle] {
        let text = assert_stable_across_cycles(dataset.as_ref(), format, 3);
        assert!(
            text.contains("_:a.b "),
            "{} must write the raw dot verbatim: {text}",
            format.media_type()
        );
        assert!(
            !text.contains("_:a..b"),
            "{} must not rewrite a legal dotted label: {text}",
            format.media_type()
        );
    }
}

/// Named regression: a NON-DEFAULT scope surfaces as the `purrdfesc{n}_` scope
/// envelope, decodes back to the same `(label, scope)` pair, and the bytes never
/// move again.
#[test]
fn scoped_label_is_stable_across_three_cycles() {
    let dataset = blank_subject_dataset("x", BlankScope(2));
    for format in [NativeRdfFormat::NTriples, NativeRdfFormat::Turtle] {
        let text = assert_stable_across_cycles(dataset.as_ref(), format, 3);
        assert!(
            text.contains("_:purrdfesc2_x"),
            "{} must write the scope envelope: {text}",
            format.media_type()
        );
        assert!(
            !text.contains("purrdfesc_purrdfesc"),
            "{} must not envelope an already-enveloped label: {text}",
            format.media_type()
        );
    }
}

/// The reserved namespace's one-step settling, pinned end to end: a label that
/// LOOKS like an envelope (`purrdfesc_abc`) is not one the encoder would have
/// written, so it is enveloped on the first write, re-parses to itself, and is a
/// byte fixpoint from then on — never merging with the label `abc`.
#[test]
fn an_envelope_shaped_label_settles_after_one_write() {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri("https://example.org/p");
    let o = b.intern_iri("https://example.org/o");
    for label in ["purrdfesc_abc", "abc"] {
        let s = b.intern_blank(label, BlankScope::DEFAULT);
        b.push_quad(s, p, o, None);
    }
    let dataset = b.freeze().expect("dataset freezes");
    for format in [NativeRdfFormat::NTriples, NativeRdfFormat::Turtle] {
        let first = serialize(dataset.as_ref(), format);
        let once = parse(&first, format);
        assert_eq!(
            blank_nodes(once.as_ref()),
            BTreeSet::from([("abc".to_owned(), 0), ("purrdfesc_abc".to_owned(), 0)]),
            "{} must keep the two labels apart",
            format.media_type()
        );
        assert_stable_across_cycles(once.as_ref(), format, 3);
    }
}

/// Named regression: an out-of-alphabet label is escaped ONCE and the marker
/// never layers, so the document is a fixed point from the first write.
#[test]
fn escaped_label_is_stable_across_three_cycles() {
    let dataset = blank_subject_dataset("a\u{d7}b", BlankScope::DEFAULT);
    for format in [NativeRdfFormat::NTriples, NativeRdfFormat::Turtle] {
        let text = assert_stable_across_cycles(dataset.as_ref(), format, 3);
        assert!(
            text.contains("_:purrdfesc_a_0000D7b"),
            "{} must escape the out-of-alphabet scalar: {text}",
            format.media_type()
        );
        assert!(
            !text.contains("purrdfesc_purrdfesc_"),
            "{} must not layer escape markers: {text}",
            format.media_type()
        );
    }
}

/// A marker-prefixed label whose body is NOT a well-formed envelope is not in
/// the encoder's image, so its bytes may move on the FIRST serialization — and
/// are a fixed point from then on, which is the whole contract.
#[test]
fn a_malformed_marker_label_is_a_fixed_point_from_the_first_cycle() {
    let dataset = blank_subject_dataset("purrdfesc__12", BlankScope::DEFAULT);
    for format in [NativeRdfFormat::NTriples, NativeRdfFormat::Turtle] {
        let first = serialize(dataset.as_ref(), format);
        let once = parse(&first, format);
        // The encoding rewrote it away from the reserved namespace, so the
        // label it re-parses to is the original one, and the document is
        // stable.
        assert_eq!(
            blank_nodes(once.as_ref()),
            BTreeSet::from([("purrdfesc__12".to_owned(), 0)]),
            "{} must decode the escaped marker-collision label",
            format.media_type()
        );
        assert_stable_across_cycles(once.as_ref(), format, 3);
    }
}
