// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Egress enforcement for blank-node labels: no native text codec may silently
//! emit a label that is illegal in ITS target syntax.
//!
//! Three layers are pinned here:
//!
//! 1. **Hostile-label matrix** — a table of adversarial labels crossed with
//!    EVERY media type in the native codec registry
//!    ([`NativeRdfFormat::all`]), asserting the per-alphabet accept/reject
//!    verdicts (line codecs → `BLANK_NODE_LABEL`, RDF/XML → `NCName`,
//!    structured codecs → unconstrained non-empty). Enumerating the registry
//!    means any codec added to it is auto-covered: the verdict `match` below
//!    fails to compile until the new format is classified.
//! 2. **Byte-stability golden** — a legal dot-free-label dataset serializes
//!    byte-identically to the pre-enforcement serializer for every line codec
//!    (zero golden churn for real single-scope data).
//! 3. **Scope-conflation regression** — `(label "a", scope 1)` and
//!    `(label "a.s1", scope 0)` serialize to DISTINCT labels and re-parse as
//!    two nodes (the dot-doubling injectivity fix).

use std::collections::BTreeSet;
use std::sync::Arc;

use purrdf_rdf::{
    BlankScope, NativeRdfFormat, RdfDataset, RdfDatasetBuilder, SerializeGraph, TermRef,
    parse_dataset, serialize_dataset,
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

/// The hostile-label table: `(label, legal as BLANK_NODE_LABEL, legal as
/// NCName)`. The verdicts are restated here independently of
/// `purrdf_core::blank_label` so this matrix cross-checks the serializer stack
/// rather than echoing the validator. Every label is non-empty, so the
/// unconstrained (structured-codec) verdict is always ACCEPT.
const HOSTILE_LABELS: &[(&str, bool, bool)] = &[
    ("bad\u{1f}label", false, false), // C0 control: no alphabet admits it
    ("a b", false, false),            // whitespace splits the token
    ("<urn:x>", false, false),        // IRI delimiters
    ("0abc", true, false),            // digit start: legal PN_CHARS_U|[0-9], illegal NCName start
    ("a.b", true, true),              // interior dot: legal in both
    ("trailing.", false, true),       // BLANK_NODE_LABEL forbids a final '.'; NCName does not
    ("-lead", false, false),          // '-' is inner-only in both alphabets
    ("\u{d7}y", false, false),        // U+00D7 ×: the gap just past [#xC0-#xD6]
    ("日本", true, true),             // PN_CHARS_BASE / NameStartChar
    ("c14n0", true, true),            // canonicalization labels are legal everywhere
];

/// Which hostile-table verdict column applies to a format. The `match` is
/// EXHAUSTIVE on purpose: adding a codec to the registry fails this test's
/// compile until its blank-label alphabet is classified here.
fn expected_accept(format: NativeRdfFormat, bnl_ok: bool, ncname_ok: bool) -> bool {
    match format {
        NativeRdfFormat::Turtle
        | NativeRdfFormat::TriG
        | NativeRdfFormat::NTriples
        | NativeRdfFormat::NQuads => bnl_ok,
        NativeRdfFormat::RdfXml => ncname_ok,
        // Structured codecs escape the label as opaque text: any non-empty
        // label serializes without error.
        NativeRdfFormat::TriX
        | NativeRdfFormat::HexTuples
        | NativeRdfFormat::JsonLd
        | NativeRdfFormat::YamlLd => true,
    }
}

#[test]
fn hostile_labels_are_enforced_per_alphabet_across_the_whole_registry() {
    for format in NativeRdfFormat::all() {
        let media_type = format.media_type();
        for &(label, bnl_ok, ncname_ok) in HOSTILE_LABELS {
            let ds = blank_subject_dataset(label);
            let outcome = serialize_dataset(&ds, media_type, SerializeGraph::Dataset);
            if expected_accept(format, bnl_ok, ncname_ok) {
                assert!(
                    outcome.is_ok(),
                    "{media_type} must accept blank label {label:?}: {:?}",
                    outcome.err()
                );
            } else {
                let err = outcome.expect_err(&format!(
                    "{media_type} must reject blank label {label:?} loudly"
                ));
                assert!(
                    err.message.contains("invalid blank-node label"),
                    "{media_type} error names the failure for {label:?}: {err:?}"
                );
                assert!(
                    err.message.contains("alphabet"),
                    "{media_type} error names the alphabet for {label:?}: {err:?}"
                );
            }
        }
    }
}

#[test]
fn okf_writer_accepts_every_hostile_label_without_error() {
    // OKF is a structured (frontmatter/JSON-shaped) egress whose blank ids are
    // unconstrained strings: no hostile label may turn into a write error
    // (unrepresentable structure is declared loss, never a label-syntax
    // failure).
    let config = purrdf_rdf::OkfConfig::new(
        "https://example.org/okf#",
        "https://example.org/doc/",
        ["type", "title"],
    )
    .expect("valid caller profile");
    for &(label, _, _) in HOSTILE_LABELS {
        let ds = blank_subject_dataset(label);
        purrdf_rdf::write_okf_bundle(&ds, &config).unwrap_or_else(|e| {
            panic!("OKF writer must accept blank label {label:?} without error: {e}")
        });
    }
}

#[test]
fn empty_label_is_rejected_by_every_codec() {
    // Even the unconstrained structured codecs refuse an EMPTY label: there is
    // no such thing as a blank node with no identifier in any target syntax.
    for format in NativeRdfFormat::all() {
        let ds = blank_subject_dataset("");
        serialize_dataset(&ds, format.media_type(), SerializeGraph::Dataset).expect_err(&format!(
            "{} must reject the empty blank label",
            format.media_type()
        ));
    }
}

#[test]
fn interior_dot_label_round_trips_to_one_node_via_dot_doubling() {
    // `a.b` is legal BLANK_NODE_LABEL syntax; the injective scope encoding
    // doubles the raw dot on egress (`_:a..b`), which must re-parse as a
    // SINGLE blank node.
    let ds = blank_subject_dataset("a.b");
    for media_type in ["text/turtle", "application/n-triples"] {
        let bytes = serialize_dataset(&ds, media_type, SerializeGraph::Dataset)
            .expect("interior-dot label serializes");
        let text = String::from_utf8(bytes).expect("utf-8");
        assert!(
            text.contains("_:a..b"),
            "{media_type} doubles the raw dot: {text}"
        );
        let reparsed =
            parse_dataset(text.as_bytes(), media_type, None).expect("emitted document re-parses");
        assert_eq!(
            blank_nodes(&reparsed).len(),
            1,
            "{media_type} re-parses to exactly one blank node: {text}"
        );
    }
}

// ── R6 byte-stability golden ─────────────────────────────────────────────────

/// A legal dot-free-label dataset: `_:alpha` (default scope) linked to
/// `_:beta` (scope 2), plus one IRI object.
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
fn dot_free_labels_serialize_byte_identically_to_the_pre_enforcement_serializer() {
    // Golden literals captured from the pre-change serializer output (this
    // exact fixture, serialized by the serializer WITHOUT egress enforcement):
    // legal dot-free labels must not churn a single byte.
    let goldens: &[(&str, &str)] = &[
        (
            "text/turtle",
            "@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n\
             @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n\
             \n\
             _:alpha <https://example.org/p> _:beta.s2 .\n\
             _:beta.s2 <https://example.org/p> <https://example.org/o> .\n",
        ),
        (
            "application/trig",
            "@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n\
             \n\
             _:alpha <https://example.org/p> _:beta.s2 .\n\
             _:beta.s2 <https://example.org/p> <https://example.org/o> .\n",
        ),
        (
            "application/n-triples",
            "_:alpha <https://example.org/p> _:beta.s2 .\n\
             _:beta.s2 <https://example.org/p> <https://example.org/o> .\n",
        ),
        (
            "application/n-quads",
            "_:alpha <https://example.org/p> _:beta.s2 .\n\
             _:beta.s2 <https://example.org/p> <https://example.org/o> .\n",
        ),
    ];
    let ds = dot_free_blank_dataset();
    for (media_type, golden) in goldens {
        let bytes = serialize_dataset(&ds, media_type, SerializeGraph::Dataset)
            .expect("legal labels serialize");
        assert_eq!(
            String::from_utf8(bytes).expect("utf-8"),
            *golden,
            "{media_type} bytes must match the pre-enforcement serializer exactly"
        );
    }
}

// ── scope-conflation regression ──────────────────────────────────────────────

#[test]
fn scoped_label_and_naive_qualified_twin_never_conflate() {
    // Before dot-doubling, raw "a" at scope 1 and raw "a.s1" at scope 0 BOTH
    // qualified to "a.s1" — two distinct nodes conflated into one on egress.
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
    assert!(text.contains("_:a.s1 "), "scoped node keeps `.s1`: {text}");
    assert!(
        text.contains("_:a..s1 "),
        "raw-dot twin doubles its dot: {text}"
    );

    let reparsed = parse_dataset(text.as_bytes(), "application/n-triples", None)
        .expect("emitted document re-parses");
    assert_eq!(
        blank_nodes(&reparsed).len(),
        2,
        "the two nodes survive the round trip distinctly: {text}"
    );
}
