// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Property-based round-trip tests (#787, T6 of #781): `parse ∘ serialize = id`,
//! modulo canonical form, for the native RDF serialization codecs the kernel exposes.
//!
//! # Equivalence is canonical, never byte-exact
//!
//! A faithful round-trip is allowed to rename blank nodes and to collapse the
//! `"x"` ≡ `"x"^^xsd:string` distinction. Byte equality would therefore produce
//! spurious failures (cf. the GTS codec-skew doctrine, PR #595: the drift gate is
//! semantic). Every property here compares **RDFC-1.0 canonical quad sets** via the
//! native [`purrdf_rdf::canonical_flat_nquads`] (#910), the same comparator the
//! production native canonicalizer wraps.
//!
//! # Single generator, three codecs (EPIC #906 — native only)
//!
//! One generator authors a frozen [`RdfDataset`] fixture; the native text codecs
//! ([`purrdf_rdf::serialize_dataset`] / [`purrdf_rdf::parse_dataset`]) serialize and
//! re-parse it for N-Quads and TriG, and the GTS fold/unfold path covers the third
//! codec. EPIC #906 removed oxigraph, so this gate now exercises the native codecs
//! against the native RDFC-1.0 comparator directly (it is no longer a cross-check
//! against an independent oxigraph implementation — the native engine is the sole
//! authority). The native text codec's own isomorphism round-trips additionally live
//! in `crates/rdf/src/native_codecs/mod.rs`.
//!
//! # Generators dodge codec-lossy inputs deliberately
//!
//! GTS drops language *direction*, so the generators emit no direction and only
//! already-canonical literals (`i32` integers, `true`/`false`, plain/typed strings,
//! standard language tags) so the preserve-path (GTS) and the text codecs agree.
//!
//! # Coverage and deferrals
//!
//! * **JSON-LD** is no longer exercised here: the native text codecs cover Turtle /
//!   TriG / N-Triples / N-Quads / RDF-XML (no JSON-LD), and the prior JSON-LD
//!   property tested oxigraph's JSON-LD serializer — removed with oxigraph (EPIC #906).
//! * **CLIF / CGIF / XCL** round-trips: depend on the open Common Logic epic
//!   (#718/#719) and do not exist yet.

use proptest::prelude::*;
use purrdf_rdf::{
    canonical_flat_nquads, flat_rdf_quads_from_dataset, parse_dataset, serialize_dataset,
    NativeRdfFormat, RdfDataset, RdfDatasetBuilder, RdfLiteral, RdfLookaside, RdfQuad, RdfTerm,
    RdfTriple, SerializeGraph,
};

const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
const XSD_BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";
const XSD_DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";
const XSD_NON_NEGATIVE_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#nonNegativeInteger";

// ── Canonical comparator (native RDFC-1.0, #910) ─────────────────────────────────

/// The native flat RDFC-1.0 canonical N-Quads of a dataset — the comparator for every
/// round-trip property (blank-node labels canonicalized, lines sorted/deduped).
fn canonical(dataset: &RdfDataset) -> String {
    canonical_flat_nquads(dataset).expect("native RDFC-1.0 canonicalization")
}

/// Serialize a dataset to RDF text of `format` (full RDF 1.2 statement layer).
fn serialize(dataset: &RdfDataset, format: NativeRdfFormat) -> Vec<u8> {
    serialize_dataset(dataset, format.media_type(), SerializeGraph::Dataset)
        .expect("native serialize")
}

/// Parse RDF text of `format` back into a frozen dataset.
fn parse(bytes: &[u8], format: NativeRdfFormat) -> std::sync::Arc<RdfDataset> {
    parse_dataset(bytes, format.media_type(), None).expect("native parse")
}

/// Re-freeze a dataset's flat quad stream WITHOUT the RDF 1.2 statement overlay, so the
/// comparator sees the same flat triple set on both sides of a round-trip (the GTS path
/// and the text path re-materialize reifiers/annotations as plain `rdf:reifies` rows).
fn flat(dataset: &RdfDataset) -> std::sync::Arc<RdfDataset> {
    let quads = flat_rdf_quads_from_dataset(dataset);
    let mut b = RdfDatasetBuilder::new();
    for quad in &quads {
        b.push_owned_quad(quad);
    }
    b.freeze().expect("flat dataset must freeze")
}

/// Freeze generated quads into the IR. The bnode-label rewrite from scope
/// qualification is irrelevant here: the comparator canonicalizes blank nodes
/// under RDFC-1.0.
fn dataset_from_quads(quads: Vec<RdfQuad>) -> std::sync::Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    for quad in quads {
        b.push_owned_quad(&quad);
    }
    b.freeze()
        .expect("generated quads must freeze into a valid dataset")
}

// ── Generators (valid, codec-safe inputs) ───────────────────────────────────────

fn arb_iri() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9]{0,6}".prop_map(|s| format!("https://example.org/{s}"))
}

fn arb_bnode_label() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9]{0,6}".prop_map(String::from)
}

fn arb_text() -> impl Strategy<Value = String> {
    // Printable ASCII without quote/backslash/control chars so GTS and the text codecs
    // escaping cannot diverge; widening this is a follow-up, not a v1 concern.
    "[A-Za-z0-9._-]{0,12}".prop_map(String::from)
}

fn arb_lang() -> impl Strategy<Value = String> {
    prop::sample::select(vec!["en", "fr", "de", "es"]).prop_map(String::from)
}

fn arb_literal() -> impl Strategy<Value = RdfLiteral> {
    prop_oneof![
        arb_text().prop_map(RdfLiteral::simple),
        arb_text().prop_map(|t| RdfLiteral::typed(t, XSD_STRING)),
        // i32::to_string is already a canonical xsd:integer lexical form (no
        // leading zeros, no "-0").
        any::<i32>().prop_map(|n| RdfLiteral::typed(n.to_string(), XSD_INTEGER)),
        prop::sample::select(vec!["true", "false"]).prop_map(|b| RdfLiteral::typed(b, XSD_BOOLEAN)),
        (arb_text(), arb_lang()).prop_map(|(t, l)| RdfLiteral::language_tagged(t, l)),
    ]
}

/// Non-canonical xsd:decimal / xsd:nonNegativeInteger lexical forms (trailing zeros,
/// leading zeros, leading `+`). The text codecs must round-trip these structurally; the
/// CANONICAL comparator in the text properties only proves the structural round-trip —
/// the RAW lexical-form + datatype fidelity (no value-space normalization, no datatype
/// narrowing) is proven separately in `literal_fidelity.rs`. These inputs are NOT fed to
/// the GTS property (its preserve-path expects already-canonical literals — see the
/// module doc), only to the text-only `nquads_roundtrip` / `trig_roundtrip` properties.
fn arb_noncanonical_literal() -> impl Strategy<Value = RdfLiteral> {
    prop_oneof![
        prop::sample::select(vec!["0.90", "0.50", "+1.5", "-0.0", "1.0E0"])
            .prop_map(|t| RdfLiteral::typed(t, XSD_DECIMAL)),
        prop::sample::select(vec!["007", "0042", "00"])
            .prop_map(|t| RdfLiteral::typed(t, XSD_NON_NEGATIVE_INTEGER)),
    ]
}

/// Leaf object terms (no quoted triple) — used inside quoted triples to keep the
/// nesting bounded and free of inner blank nodes.
fn arb_simple_object() -> impl Strategy<Value = RdfTerm> {
    prop_oneof![
        arb_iri().prop_map(RdfTerm::iri),
        arb_literal().prop_map(RdfTerm::literal),
    ]
}

/// One level of RDF-1.2 quoted triple: `<< iri iri (iri|literal) >>`.
fn arb_quoted_triple() -> impl Strategy<Value = RdfTriple> {
    (arb_iri(), arb_iri(), arb_simple_object())
        .prop_map(|(s, p, o)| RdfTriple::new(RdfTerm::iri(s), p, o))
}

/// Object terms without a quoted triple — the surface GTS represents faithfully
/// (GTS lowers bare triple-term objects to blank nodes, since its quoted-triple
/// support goes through the reifier idiom, not bare triple terms).
fn arb_object_basic() -> impl Strategy<Value = RdfTerm> {
    prop_oneof![
        arb_iri().prop_map(RdfTerm::iri),
        arb_bnode_label().prop_map(RdfTerm::blank_node),
        arb_literal().prop_map(RdfTerm::literal),
    ]
}

/// Basic objects plus RDF-1.2 quoted triples AND non-canonical decimal /
/// nonNegativeInteger literals — round-tripped by the lossless N-Quads/TriG codecs (NOT
/// GTS, which uses [`arb_object_basic`] with already-canonical literals only).
fn arb_object_star() -> impl Strategy<Value = RdfTerm> {
    prop_oneof![
        4 => arb_object_basic(),
        1 => arb_quoted_triple().prop_map(RdfTerm::triple),
        2 => arb_noncanonical_literal().prop_map(RdfTerm::literal),
    ]
}

fn arb_subject() -> impl Strategy<Value = RdfTerm> {
    prop_oneof![
        arb_iri().prop_map(RdfTerm::iri),
        arb_bnode_label().prop_map(RdfTerm::blank_node),
    ]
}

fn mk_quad(
    (subject, predicate, object, graph): (RdfTerm, String, RdfTerm, Option<String>),
) -> RdfQuad {
    let quad = RdfQuad::new(subject, predicate, object);
    match graph {
        Some(g) => quad.in_graph(RdfTerm::iri(g)),
        None => quad,
    }
}

/// Dataset over the GTS-faithful surface (no bare quoted-triple objects).
fn arb_dataset() -> impl Strategy<Value = std::sync::Arc<RdfDataset>> {
    let quad = (
        arb_subject(),
        arb_iri(),
        arb_object_basic(),
        prop::option::of(arb_iri()),
    )
        .prop_map(mk_quad);
    prop::collection::vec(quad, 0..16).prop_map(dataset_from_quads)
}

/// Dataset including RDF-1.2 quoted triples (for the lossless N-Quads/TriG codecs).
fn arb_dataset_star() -> impl Strategy<Value = std::sync::Arc<RdfDataset>> {
    let quad = (
        arb_subject(),
        arb_iri(),
        arb_object_star(),
        prop::option::of(arb_iri()),
    )
        .prop_map(mk_quad);
    prop::collection::vec(quad, 0..16).prop_map(dataset_from_quads)
}

// ── Config ──────────────────────────────────────────────────────────────────────

fn config() -> ProptestConfig {
    // Bounded case count keeps each property well under the nextest 60s
    // slow-timeout; raise locally with PROPTEST_CASES to deepen the search.
    let cases = std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(64);
    ProptestConfig {
        cases,
        // No on-disk regression files in a clean checkout / CI tree.
        failure_persistence: None,
        ..ProptestConfig::default()
    }
}

// ── Properties ──────────────────────────────────────────────────────────────────

proptest! {
    #![proptest_config(config())]

    /// N-Quads: serialize → parse round-trips to the same canonical quad set,
    /// including RDF-1.2 quoted triples.
    #[test]
    fn nquads_roundtrip(dataset in arb_dataset_star()) {
        let bytes = serialize(dataset.as_ref(), NativeRdfFormat::NQuads);
        let after = parse(&bytes, NativeRdfFormat::NQuads);
        prop_assert_eq!(canonical(&flat(dataset.as_ref())), canonical(after.as_ref()));
    }

    /// TriG: same property, exercising named graphs and quoted triples.
    #[test]
    fn trig_roundtrip(dataset in arb_dataset_star()) {
        let bytes = serialize(dataset.as_ref(), NativeRdfFormat::TriG);
        let after = parse(&bytes, NativeRdfFormat::TriG);
        prop_assert_eq!(canonical(&flat(dataset.as_ref())), canonical(after.as_ref()));
    }

    /// GTS fold/unfold: RdfDataset -> `to_gts` -> fold -> purrdf import preserves the
    /// same canonical quad set.
    #[test]
    fn gts_roundtrip(dataset in arb_dataset()) {
        let bytes = purrdf_rdf::gts_write::to_gts(dataset.as_ref(), &RdfLookaside::default(), "purrdf-proptest")
            .expect("to_gts should succeed");
        let graph = purrdf_gts::reader::read(&bytes, false, None);
        prop_assert!(graph.diagnostics.is_empty(), "GTS fold diagnostics: {:?}", graph.diagnostics);
        let after = purrdf_rdf::import_gts_graph(graph).expect("import folded GTS graph");
        prop_assert_eq!(
            canonical(&flat(dataset.as_ref())),
            canonical(&flat(after.dataset.as_ref()))
        );
    }
}
