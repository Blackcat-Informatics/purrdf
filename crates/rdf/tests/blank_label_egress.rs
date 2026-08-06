// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Egress totality for blank-node labels: every native text codec serializes
//! EVERY label, escaping the ones its target syntax cannot spell, and the
//! document it writes re-parses into an isomorphic dataset.
//!
//! Four layers are pinned here:
//!
//! 1. **Escape-and-reparse matrix** — a table of adversarial labels crossed
//!    with EVERY media type in the native codec registry
//!    ([`NativeRdfFormat::all`]): serialization always succeeds, the emitted
//!    label is legal under the format's own alphabet, and re-parsing the
//!    document restores the input's blank-node LABEL IDENTITY — ingress inverts
//!    the egress transform exactly, so the round trip is not merely isomorphic.
//!    Enumerating the registry means any codec added to it is auto-covered:
//!    the alphabet `match` below fails to compile until the new format is
//!    classified.
//! 2. **Injectivity across the document** — two distinct blank nodes never
//!    conflate on egress, including the adversarial case where one node's
//!    label equals the escape image of the other's.
//! 3. **Byte-stability golden** — a legal-label dataset serializes
//!    byte-identically to the pre-escape serializer for every line codec (zero
//!    golden churn for real single-scope data).
//! 4. **Scope-conflation regression** — `(label "a", scope 1)` and
//!    `(label "a.s1", scope 0)` serialize to DISTINCT labels and re-parse as
//!    two nodes: the scoped pair as its envelope, the literal label verbatim.

use std::collections::BTreeSet;
use std::sync::Arc;

use purrdf_rdf::blank_label::{LabelAlphabet, encode_blank_label, is_valid_label};
use purrdf_rdf::{
    BlankScope, NativeRdfFormat, RdfDataset, RdfDatasetBuilder, SerializeGraph, TermRef,
    datasets_isomorphic, parse_dataset, serialize_dataset,
};

/// One quad whose subject is a blank node with the given raw label at the
/// DEFAULT scope.
fn blank_subject_dataset(label: &str) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_blank(label, BlankScope::DEFAULT);
    let p = b.intern_iri("https://example.org/p");
    let o = b.intern_iri("https://example.org/o");
    b.push_quad(s, p, o, None);
    b.freeze().expect("dataset freezes")
}

/// The distinct blank `(label, scope)` pairs among a dataset's quad subjects
/// and objects.
fn blank_nodes(ds: &RdfDataset) -> BTreeSet<(String, u32)> {
    let mut blanks = BTreeSet::new();
    for quad in ds.quads() {
        for id in [quad.s, quad.o] {
            if let TermRef::Blank { label, scope } = ds.resolve(id) {
                blanks.insert((label.to_owned(), scope.ordinal()));
            }
        }
    }
    blanks
}

/// The hostile-label table. Every entry is a label some producer can hand the
/// serializer, and every entry must now SERIALIZE — the columns that used to
/// record accept/reject verdicts are gone, because there is no reject.
const HOSTILE_LABELS: &[&str] = &[
    "",                    // no identifier at all
    "bad\u{1f}label",      // C0 control: no syntax admits it, XML cannot spell it
    "a b",                 // whitespace splits a `_:` token
    "a\nb",                // a newline XML would normalize away
    "<urn:x>",             // IRI delimiters
    "0abc",                // digit start: legal BLANK_NODE_LABEL, illegal NCName
    "a.b",                 // interior dot
    "a..b",                // a doubled interior dot: a DIFFERENT label from `a.b`
    "a...b",               // …and a tripled one
    "trailing.",           // BLANK_NODE_LABEL forbids a final '.'; NCName does not
    "-lead",               // '-' is inner-only in both name alphabets
    "\u{d7}y",             // U+00D7 ×: the gap just past [#xC0-#xD6]
    "日本",                // PN_CHARS_BASE / NameStartChar
    "c14n0",               // canonicalization labels are legal everywhere
    "purrdfesc_a_000020b", // the envelope of "a b": must not conflate with it
    "purrdfesc_abc",       // an envelope-SHAPED label: must not conflate with `abc`
    "abc",                 // …and the label it would conflate with
];

/// The blank-node label alphabet a format's codec emits. The `match` is
/// EXHAUSTIVE on purpose: adding a codec to the registry fails this test's
/// compile until its blank-label alphabet is classified here. Restated
/// independently of the serializer's own table so the matrix cross-checks it.
fn emitted_alphabet(format: NativeRdfFormat) -> LabelAlphabet {
    match format {
        NativeRdfFormat::Turtle
        | NativeRdfFormat::TriG
        | NativeRdfFormat::NTriples
        | NativeRdfFormat::NQuads
        | NativeRdfFormat::HexTuples
        | NativeRdfFormat::JsonLd
        | NativeRdfFormat::YamlLd => LabelAlphabet::BlankNodeLabel,
        NativeRdfFormat::RdfXml => LabelAlphabet::NcName,
        NativeRdfFormat::TriX => LabelAlphabet::XmlText,
    }
}

#[test]
fn every_hostile_label_serializes_and_reparses_isomorphically() {
    for format in NativeRdfFormat::all() {
        let media_type = format.media_type();
        let alphabet = emitted_alphabet(format);
        for label in HOSTILE_LABELS {
            let ds = blank_subject_dataset(label);
            let bytes =
                serialize_dataset(&ds, media_type, SerializeGraph::Dataset).unwrap_or_else(|e| {
                    panic!("{media_type} must serialize blank label {label:?}: {e}")
                });

            // The document re-parses — the whole point of escaping instead of
            // refusing — and carries the same graph up to blank renaming.
            let reparsed = parse_dataset(&bytes, media_type, None).unwrap_or_else(|e| {
                panic!(
                    "{media_type} output for {label:?} must re-parse: {e}\n{}",
                    String::from_utf8_lossy(&bytes)
                )
            });
            assert!(
                datasets_isomorphic(&ds, &reparsed),
                "{media_type} round trip for {label:?} is not isomorphic:\n{}",
                String::from_utf8_lossy(&bytes)
            );

            // Exactly one blank node in, exactly one blank node out.
            assert_eq!(
                blank_nodes(&reparsed).len(),
                1,
                "{media_type} re-parses {label:?} to one blank node"
            );

            // The token the serializer wrote is legal under the format's own
            // alphabet, so an external conforming parser reads it too. Restated
            // from the egress contract (`encode_blank_label`) rather than read
            // back off the re-parsed dataset, because ingress now DECODES that
            // token — which is what the identity assertion below pins.
            let emitted = encode_blank_label(label, BlankScope::DEFAULT, alphabet);
            assert!(
                is_valid_label(&emitted, alphabet),
                "{media_type} emitted {emitted:?} for {label:?}, illegal under {alphabet:?}"
            );

            // Stronger than isomorphism: the round trip restores the blank node's
            // LABEL IDENTITY, because ingress inverts the egress transform exactly.
            assert_eq!(
                blank_nodes(&reparsed).into_iter().next(),
                Some(((*label).to_owned(), 0u32)),
                "{media_type} round trip must restore {label:?} verbatim:\n{}",
                String::from_utf8_lossy(&bytes)
            );
        }
    }
}

#[test]
fn a_legal_label_passes_through_unescaped_in_every_format() {
    // The escape must be inert on real data: a plain label is written verbatim
    // in every syntax, so no fixture or golden can churn.
    for format in NativeRdfFormat::all() {
        let media_type = format.media_type();
        let ds = blank_subject_dataset("alpha");
        let bytes = serialize_dataset(&ds, media_type, SerializeGraph::Dataset)
            .expect("a legal label always serializes");
        let text = String::from_utf8(bytes).expect("utf-8");
        assert!(
            text.contains("alpha"),
            "{media_type} must write the label verbatim: {text}"
        );
        assert!(
            !text.contains("purrdfesc_"),
            "{media_type} must not escape a legal label: {text}"
        );
    }
}

#[test]
fn distinct_blank_nodes_never_conflate_on_egress() {
    // The adversarial pair: one node whose label is ILLEGAL everywhere, and a
    // second whose label is exactly the first one's escape image. A
    // non-injective escape would fold them into one node.
    let illegal = "a b";
    let image = "purrdfesc_a_000020b";
    let mut b = RdfDatasetBuilder::new();
    let first = b.intern_blank(illegal, BlankScope::DEFAULT);
    let second = b.intern_blank(image, BlankScope::DEFAULT);
    let p = b.intern_iri("https://example.org/p");
    let o = b.intern_iri("https://example.org/o");
    b.push_quad(first, p, o, None);
    b.push_quad(second, p, o, None);
    let ds = b.freeze().expect("dataset freezes");
    assert_eq!(blank_nodes(&ds).len(), 2, "two distinct nodes in the IR");

    for format in NativeRdfFormat::all() {
        let media_type = format.media_type();
        let bytes = serialize_dataset(&ds, media_type, SerializeGraph::Dataset)
            .expect("both labels serialize");
        let reparsed = parse_dataset(&bytes, media_type, None).expect("output re-parses");
        assert_eq!(
            blank_nodes(&reparsed).len(),
            2,
            "{media_type} must keep the two nodes apart:\n{}",
            String::from_utf8_lossy(&bytes)
        );
        assert!(
            datasets_isomorphic(&ds, &reparsed),
            "{media_type} round trip is not isomorphic:\n{}",
            String::from_utf8_lossy(&bytes)
        );
    }
}

#[test]
fn co_reference_survives_escaping() {
    // One illegally-labelled blank node used TWICE must stay one node after
    // the escape: `_:x p _:x` may not become two nodes.
    let mut b = RdfDatasetBuilder::new();
    let node = b.intern_blank("a b", BlankScope::DEFAULT);
    let p = b.intern_iri("https://example.org/p");
    b.push_quad(node, p, node, None);
    let ds = b.freeze().expect("dataset freezes");

    for format in NativeRdfFormat::all() {
        let media_type = format.media_type();
        let bytes = serialize_dataset(&ds, media_type, SerializeGraph::Dataset)
            .expect("the label serializes");
        let reparsed = parse_dataset(&bytes, media_type, None).expect("output re-parses");
        assert_eq!(
            blank_nodes(&reparsed).len(),
            1,
            "{media_type} must keep the self-reference one node:\n{}",
            String::from_utf8_lossy(&bytes)
        );
        assert!(datasets_isomorphic(&ds, &reparsed), "{media_type}");
    }
}

#[test]
fn serialization_is_byte_deterministic_for_escaped_labels() {
    // The repo's determinism rule holds through the escape: same dataset, same
    // bytes, every run and every format.
    for format in NativeRdfFormat::all() {
        let media_type = format.media_type();
        for label in HOSTILE_LABELS {
            let ds = blank_subject_dataset(label);
            let first = serialize_dataset(&ds, media_type, SerializeGraph::Dataset)
                .expect("serializes once");
            let second = serialize_dataset(&ds, media_type, SerializeGraph::Dataset)
                .expect("serializes twice");
            assert_eq!(first, second, "{media_type} for {label:?}");
        }
    }
}

#[test]
fn okf_writer_accepts_every_hostile_label_without_error() {
    // OKF is a structured (frontmatter/JSON-shaped) egress whose blank ids are
    // internal keys, never RDF document syntax: no hostile label may turn into
    // a write error (unrepresentable structure is declared loss, never a
    // label-syntax failure).
    let config = purrdf_rdf::OkfConfig::new(
        "https://example.org/okf#",
        "https://example.org/doc/",
        ["type", "title"],
    )
    .expect("valid caller profile");
    for label in HOSTILE_LABELS {
        let ds = blank_subject_dataset(label);
        purrdf_rdf::write_okf_bundle(&ds, &config).unwrap_or_else(|e| {
            panic!("OKF writer must accept blank label {label:?} without error: {e}")
        });
    }
}

#[test]
fn the_dotted_family_stays_distinct_and_verbatim_on_the_wire() {
    // `a.b`, `a..b` and `a...b` are three DISTINCT legal `BLANK_NODE_LABEL`s.
    // Each reaches the wire byte for byte, and a document holding all three
    // re-parses to three blank nodes — the class the old dot-doubling egress
    // silently merged into one.
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri("https://example.org/p");
    let o = b.intern_iri("https://example.org/o");
    for label in ["a.b", "a..b", "a...b"] {
        let s = b.intern_blank(label, BlankScope::DEFAULT);
        b.push_quad(s, p, o, None);
    }
    let ds = b.freeze().expect("dataset freezes");
    assert_eq!(blank_nodes(&ds).len(), 3, "three distinct nodes in the IR");

    for media_type in ["text/turtle", "application/n-triples"] {
        let bytes = serialize_dataset(&ds, media_type, SerializeGraph::Dataset)
            .expect("interior-dot labels serialize");
        let text = String::from_utf8(bytes).expect("utf-8");
        for label in ["_:a.b ", "_:a..b ", "_:a...b "] {
            assert!(
                text.contains(label),
                "{media_type} must write {label:?} verbatim: {text}"
            );
        }
        let reparsed =
            parse_dataset(text.as_bytes(), media_type, None).expect("emitted document re-parses");
        assert_eq!(
            blank_nodes(&reparsed).len(),
            3,
            "{media_type} re-parses to exactly three blank nodes: {text}"
        );
        assert!(
            datasets_isomorphic(&ds, &reparsed),
            "{media_type} round trip is not isomorphic: {text}"
        );
    }
}

// ── byte-stability golden ────────────────────────────────────────────────────

/// A legal-label dataset: `_:alpha` (default scope) linked to `_:beta` (scope
/// 2), plus one IRI object.
fn dot_free_blank_dataset() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let alpha = b.intern_blank("alpha", BlankScope::DEFAULT);
    let beta = b.intern_blank("beta", BlankScope(2));
    let p = b.intern_iri("https://example.org/p");
    let o = b.intern_iri("https://example.org/o");
    b.push_quad(alpha, p, beta, None);
    b.push_quad(beta, p, o, None);
    b.freeze().expect("dataset freezes")
}

#[test]
fn unscoped_legal_labels_serialize_byte_identically_to_the_pre_escape_serializer() {
    // Golden literals captured from the pre-change serializer output (this
    // exact fixture, serialized by the serializer WITHOUT the egress encoding):
    // an unscoped legal label must not churn a single byte. The scoped node is
    // the one spelling that moves, because a scope has to be carried somewhere:
    // it is the `purrdfesc{n}_{body}` envelope.
    let goldens: &[(&str, &str)] = &[
        (
            "text/turtle",
            "@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n\
             @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n\
             \n\
             _:alpha <https://example.org/p> _:purrdfesc2_beta .\n\
             _:purrdfesc2_beta <https://example.org/p> <https://example.org/o> .\n",
        ),
        (
            "application/trig",
            "@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n\
             \n\
             _:alpha <https://example.org/p> _:purrdfesc2_beta .\n\
             _:purrdfesc2_beta <https://example.org/p> <https://example.org/o> .\n",
        ),
        (
            "application/n-triples",
            "_:alpha <https://example.org/p> _:purrdfesc2_beta .\n\
             _:purrdfesc2_beta <https://example.org/p> <https://example.org/o> .\n",
        ),
        (
            "application/n-quads",
            "_:alpha <https://example.org/p> _:purrdfesc2_beta .\n\
             _:purrdfesc2_beta <https://example.org/p> <https://example.org/o> .\n",
        ),
    ];
    let ds = dot_free_blank_dataset();
    for (media_type, golden) in goldens {
        let bytes = serialize_dataset(&ds, media_type, SerializeGraph::Dataset)
            .expect("legal labels serialize");
        assert_eq!(
            String::from_utf8(bytes).expect("utf-8"),
            *golden,
            "{media_type} bytes must match the pre-escape serializer exactly"
        );
    }
}

// ── scope-conflation regression ──────────────────────────────────────────────

#[test]
fn scoped_label_and_naive_qualified_twin_never_conflate() {
    // Historically, raw "a" at scope 1 and raw "a.s1" at scope 0 BOTH qualified
    // to "a.s1" — two distinct nodes conflated into one on egress. Now the
    // scoped pair is an envelope and the literal label passes through, so the
    // two spellings cannot meet.
    let mut b = RdfDatasetBuilder::new();
    let scoped = b.intern_blank("a", BlankScope(1));
    let naive_twin = b.intern_blank("a.s1", BlankScope::DEFAULT);
    let p = b.intern_iri("https://example.org/p");
    let o = b.intern_iri("https://example.org/o");
    b.push_quad(scoped, p, o, None);
    b.push_quad(naive_twin, p, o, None);
    let ds = b.freeze().expect("dataset freezes");
    assert_eq!(blank_nodes(&ds).len(), 2, "two distinct nodes in the IR");

    let bytes = serialize_dataset(&ds, "application/n-triples", SerializeGraph::Dataset)
        .expect("both labels are legal");
    let text = String::from_utf8(bytes).expect("utf-8");
    assert!(
        text.contains("_:purrdfesc1_a "),
        "the scoped pair is written as its envelope: {text}"
    );
    assert!(
        text.contains("_:a.s1 "),
        "the literal label is written verbatim: {text}"
    );

    let reparsed = parse_dataset(text.as_bytes(), "application/n-triples", None)
        .expect("emitted document re-parses");
    assert_eq!(
        blank_nodes(&reparsed).len(),
        2,
        "the two nodes survive the round trip distinctly: {text}"
    );
}
