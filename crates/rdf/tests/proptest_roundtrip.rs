// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Property-based round-trip tests (T6 of): `parse ∘ serialize = id`,
//! modulo canonical form, for the native RDF serialization codecs the kernel exposes.
//!
//! # Equivalence is canonical, never byte-exact
//!
//! A faithful round-trip is allowed to rename blank nodes and to collapse the
//! `"x"` ≡ `"x"^^xsd:string` distinction. Byte equality would therefore produce
//! spurious failures (cf. the GTS codec-skew doctrine, : the drift gate is
//! semantic). Every property here compares **RDFC-1.0 canonical quad sets** via the
//! native [`purrdf_rdf::canonical_flat_nquads`], the same comparator the
//! production native canonicalizer wraps.
//!
//! # One generator family, four codec paths ( — native only)
//!
//! One generator family authors frozen [`RdfDataset`] fixtures; the native text codecs
//! ([`purrdf_rdf::serialize_dataset`] / [`purrdf_rdf::parse_dataset`]) serialize and
//! re-parse them for N-Quads and TriG, the GTS fold/unfold path covers the third
//! codec, and an NCName-restricted generator drives the RDF/XML round-trip. With
//! oxigraph removed, this gate exercises the native codecs against the native
//! RDFC-1.0 comparator directly (it is no longer a cross-check against an
//! independent oxigraph implementation — the native engine is the sole
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
//!   property tested oxigraph's JSON-LD serializer — removed with oxigraph.
//! * **CLIF / CGIF / XCL** round-trips: depend on the open Common Logic epic
//!   and do not exist yet.

use proptest::prelude::*;
use purrdf_rdf::{
    BlankScope, CanonHash, NativeRdfFormat, RdfDataset, RdfDatasetBuilder, RdfLiteral,
    RdfLookaside, RdfQuad, RdfTerm, RdfTriple, SerializeGraph, canonical_flat_nquads,
    canonicalize_with, dataset_from_quad_sources, flat_dataset_from_quad_sources,
    flat_dataset_from_quads, flat_rdf_quads_from_dataset, parse_dataset, serialize_dataset,
    try_canonicalize_flat_view,
};

const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
const XSD_BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";
const XSD_DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";
const XSD_NON_NEGATIVE_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#nonNegativeInteger";

// ── Canonical comparator (native RDFC-1.0) ─────────────────────────────────

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

/// A handful of non-ASCII `PN_CHARS_BASE` starters, exercising the exact
/// Unicode ranges of the W3C `BLANK_NODE_LABEL` production (Latin-1 letters,
/// combining-accent precomposed letters, CJK).
fn arb_pn_chars_base_nonascii() -> impl Strategy<Value = &'static str> {
    prop::sample::select(vec!["ü", "é", "日", "本", "ñ"])
}

/// The FULL legal `BLANK_NODE_LABEL` surface: leading lowercase letter, leading
/// digit, leading underscore, interior dots (never trailing), interior hyphens,
/// and non-ASCII `PN_CHARS_BASE` starters. Every generated label satisfies
/// [`purrdf_rdf::blank_label::is_valid_blank_node_label`] (proved by
/// `bnode_label_generator_is_in_contract` below).
fn arb_bnode_label() -> impl Strategy<Value = String> {
    prop_oneof![
        // Plain lowercase ASCII.
        "[a-z][a-z0-9]{0,6}".prop_map(String::from),
        // Leading digit (legal for BLANK_NODE_LABEL, illegal for NCName).
        "[0-9][a-z0-9]{0,6}".prop_map(String::from),
        // Leading underscore (PN_CHARS_U).
        "_[a-z0-9]{0,6}".prop_map(String::from),
        // Interior dot — legal mid-label, never trailing (the tail class
        // excludes '.', so the final character is always PN_CHARS).
        "[a-z][a-z0-9]{0,2}\\.[a-z0-9]{1,3}".prop_map(String::from),
        // Interior hyphen (PN_CHARS includes '-').
        "[a-z][a-z0-9]{0,2}-[a-z0-9]{1,3}".prop_map(String::from),
        // Non-ASCII PN_CHARS_BASE starter with an ASCII tail.
        (arb_pn_chars_base_nonascii(), "[a-z0-9]{0,4}")
            .prop_map(|(head, tail)| format!("{head}{tail}")),
    ]
}

fn arb_text() -> impl Strategy<Value = String> {
    // Printable ASCII without quote/backslash/control chars so GTS and the text codecs
    // escaping cannot diverge.
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

/// NCName-LEGAL blank labels for the RDF/XML round-trip: letter or underscore
/// start (never a digit — `rdf:nodeID` values are XML `NCName`s), interior
/// hyphens, interior dots, and the non-ASCII `NCNameStartChar` range. The
/// RDF/XML codec emits and re-reads the full `NCName` alphabet, so the
/// generator covers it rather than an ASCII subset; the pass-through property
/// below asserts these labels reach the document unescaped.
fn arb_ncname_label() -> impl Strategy<Value = String> {
    prop_oneof![
        "[a-z][a-z0-9]{0,6}".prop_map(String::from),
        "_[a-z0-9]{0,6}".prop_map(String::from),
        "[a-z][a-z0-9]{0,2}-[a-z0-9]{1,3}".prop_map(String::from),
        "[a-z][a-z0-9]{0,2}\\.[a-z0-9]{1,3}".prop_map(String::from),
        (arb_pn_chars_base_nonascii(), "[a-z0-9]{0,4}")
            .prop_map(|(head, tail)| format!("{head}{tail}")),
    ]
}

/// Dataset over the RDF/XML-faithful surface: default graph only (RDF/XML is a
/// single-graph syntax), no quoted triples (star-incapable), and blank labels
/// drawn from the NCName-legal generator.
fn arb_dataset_rdfxml() -> impl Strategy<Value = std::sync::Arc<RdfDataset>> {
    let subject = prop_oneof![
        arb_iri().prop_map(RdfTerm::iri),
        arb_ncname_label().prop_map(RdfTerm::blank_node),
    ];
    let object = prop_oneof![
        arb_iri().prop_map(RdfTerm::iri),
        arb_ncname_label().prop_map(RdfTerm::blank_node),
        arb_literal().prop_map(RdfTerm::literal),
    ];
    let quad = (subject, arb_iri(), object).prop_map(|(s, p, o)| RdfQuad::new(s, p, o));
    prop::collection::vec(quad, 0..16).prop_map(dataset_from_quads)
}

// ── Wrapper-agreement differential generators: scoped blanks, nested triples,
// blank graph names, and both reifier spellings ─────────────────────────────

const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

/// The overlay's OWN reserved sentinel for a reifier binding
/// (`purrdf_rdf::RESERVED_NAMESPACE` joined with `"reifies"`). Not exported
/// (`purrdf-rdf` mints no vocabulary); spelled locally exactly as [`RDF_REIFIES`]
/// above already is.
const SENTINEL_REIFIES: &str = "urn:purrdf:rdfc:reifies";

/// A graph-position term: an IRI or a BLANK NODE — a named graph named by a
/// blank node, which `arb_dataset`/`arb_dataset_star` above never generate
/// (their graph slot is `prop::option::of(arb_iri())`, IRI-only).
fn arb_graph_term() -> impl Strategy<Value = RdfTerm> {
    prop_oneof![
        arb_iri().prop_map(RdfTerm::iri),
        arb_bnode_label().prop_map(RdfTerm::blank_node),
    ]
}

/// A quoted triple nested up to TWO levels deep: [`arb_quoted_triple`]'s
/// one-level triple, sometimes wrapped as the object of an outer triple.
fn arb_nested_triple() -> impl Strategy<Value = RdfTriple> {
    arb_quoted_triple().prop_flat_map(|inner| {
        let wrap = inner.clone();
        prop_oneof![
            Just(inner),
            (arb_iri(), arb_iri()).prop_map(move |(s, p)| {
                RdfTriple::new(RdfTerm::iri(s), p, RdfTerm::triple(wrap.clone()))
            }),
        ]
    })
}

/// Object terms for the differential generator: the basic surface, plus a
/// (possibly two-level-nested) quoted triple.
fn arb_diff_object() -> impl Strategy<Value = RdfTerm> {
    prop_oneof![
        3 => arb_object_basic(),
        1 => arb_nested_triple().prop_map(RdfTerm::triple),
    ]
}

/// Builds the actual [`RdfQuad`] from a generated `(subject, predicate, object,
/// graph)` tuple — the mapping [`arb_diff_quad`] and [`arb_diff_quad_no_sentinel`]
/// share, so the two differ ONLY in which predicates they can produce.
fn diff_quad_from_parts((s, p, o, g): (RdfTerm, String, RdfTerm, Option<RdfTerm>)) -> RdfQuad {
    let quad = RdfQuad::new(s, p, o);
    match g {
        Some(g) => quad.in_graph(g),
        None => quad,
    }
}

/// A quad for the differential generator: any subject, a predicate that is USUALLY
/// an ordinary IRI but OCCASIONALLY the literal `rdf:reifies` (so the
/// INGESTION-time fold — `fold_statement_layer`, the production parsing route —
/// has something to fold), and OCCASIONALLY the overlay's OWN reserved sentinel
/// `urn:purrdf:rdfc:reifies` (so the CANONICALIZATION-time fold in `canon.rs` has
/// something to fold too — the two are different seams). A (possibly nested) object,
/// and an optional graph that may itself be a blank node, complete the quad.
///
/// See [`arb_diff_quad_no_sentinel`] for the variant the pre-delegation ORACLE
/// property below needs: that route shares the sentinel fold's defect (module
/// documentation of `flat_canon_of_a_sentinel_spelled_reifier_matches_its_real_predicate_spelling`),
/// so a generator that can produce the sentinel is not safe input for an
/// oracle-agreement property.
fn arb_diff_quad() -> impl Strategy<Value = RdfQuad> {
    let predicate = prop_oneof![
        4 => arb_iri(),
        1 => Just(RDF_REIFIES.to_owned()),
        1 => Just(SENTINEL_REIFIES.to_owned()),
    ];
    (
        arb_subject(),
        predicate,
        arb_diff_object(),
        prop::option::of(arb_graph_term()),
    )
        .prop_map(diff_quad_from_parts)
}

/// [`arb_diff_quad`], constrained to NEVER produce the overlay's reserved sentinel —
/// the generator the pre-delegation oracle property uses, because that oracle route
/// shares the sentinel fold's defect and an oracle comparison over sentinel-spelled
/// input would only prove the two agree on being wrong.
fn arb_diff_quad_no_sentinel() -> impl Strategy<Value = RdfQuad> {
    let predicate = prop_oneof![
        4 => arb_iri(),
        1 => Just(RDF_REIFIES.to_owned()),
    ];
    (
        arb_subject(),
        predicate,
        arb_diff_object(),
        prop::option::of(arb_graph_term()),
    )
        .prop_map(diff_quad_from_parts)
}

/// Two independently-generated quad sources. [`dataset_from_quad_sources`] and
/// [`flat_dataset_from_quad_sources`] standardize each source apart under its
/// own fresh [`BlankScope`], so a blank label repeated across the two sources
/// still names two DISTINCT nodes — the "scoped blanks" surface the
/// wrapper-agreement differential requires.
fn arb_diff_sources() -> impl Strategy<Value = (Vec<RdfQuad>, Vec<RdfQuad>)> {
    (
        prop::collection::vec(arb_diff_quad(), 0..6),
        prop::collection::vec(arb_diff_quad(), 0..4),
    )
}

/// [`arb_diff_sources`], built from [`arb_diff_quad_no_sentinel`] — the oracle
/// property's own generator (see that function's documentation for why).
fn arb_diff_sources_no_sentinel() -> impl Strategy<Value = (Vec<RdfQuad>, Vec<RdfQuad>)> {
    (
        prop::collection::vec(arb_diff_quad_no_sentinel(), 0..6),
        prop::collection::vec(arb_diff_quad_no_sentinel(), 0..4),
    )
}

// ── Config ──────────────────────────────────────────────────────────────────────

fn config() -> ProptestConfig {
    // Bounded case count keeps each property fast under `cargo test` (and the
    // CI job timeout); raise locally with PROPTEST_CASES to deepen the search.
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

    /// The blank-label generator itself is in-contract: every label it emits is a
    /// legal `BLANK_NODE_LABEL` under the exact egress alphabet.
    #[test]
    fn bnode_label_generator_is_in_contract(label in arb_bnode_label()) {
        prop_assert!(
            purrdf_rdf::blank_label::is_valid_blank_node_label(&label),
            "generated label is outside the BLANK_NODE_LABEL contract: {label:?}"
        );
    }

    /// The NCName-label generator is in-contract for BOTH alphabets it must
    /// satisfy: the XML `NCName` grammar (RDF/XML `rdf:nodeID`) and the
    /// `BLANK_NODE_LABEL` grammar (so the same fixtures stay text-codec-legal).
    #[test]
    fn ncname_label_generator_is_in_contract(label in arb_ncname_label()) {
        prop_assert!(
            purrdf_rdf::blank_label::is_valid_ncname(&label),
            "generated label is outside the NCName contract: {label:?}"
        );
        prop_assert!(
            purrdf_rdf::blank_label::is_valid_blank_node_label(&label),
            "generated label is outside the BLANK_NODE_LABEL contract: {label:?}"
        );
    }

    /// RDF/XML: serialize → parse round-trips the same canonical quad set for
    /// NCName-legal blank labels (letters/underscore start).
    #[test]
    fn rdfxml_roundtrip(dataset in arb_dataset_rdfxml()) {
        let bytes = serialize(dataset.as_ref(), NativeRdfFormat::RdfXml);
        let after = parse(&bytes, NativeRdfFormat::RdfXml);
        prop_assert_eq!(canonical(&flat(dataset.as_ref())), canonical(after.as_ref()));
    }

    /// The egress escape is INERT on an in-alphabet label: a legal
    /// `BLANK_NODE_LABEL` reaches the N-Triples document byte-identically (up
    /// to the scope qualification every writer applies first), so no fixture,
    /// vector or golden can churn.
    ///
    /// `dataset_from_quads` builds the fixture through `push_owned_quad`, the
    /// OWNED ingress boundary, which decodes each label per
    /// [`BlankScope::unqualify_label`](purrdf_rdf::BlankScope::unqualify_label).
    /// A generated label is outside the reserved marker namespace, so that
    /// decode is the identity and the wire form is the label itself; the
    /// expectation is nonetheless written as the decode-then-requalify FIXPOINT,
    /// which mirrors the boundary exactly rather than assuming it.
    #[test]
    fn legal_blank_labels_pass_through_unescaped(label in arb_bnode_label()) {
        let dataset = dataset_from_quads(vec![RdfQuad::new(
            RdfTerm::blank_node(label.clone()),
            "https://example.org/p",
            RdfTerm::iri("https://example.org/o"),
        )]);
        let bytes = serialize(dataset.as_ref(), NativeRdfFormat::NTriples);
        let text = String::from_utf8(bytes).expect("utf-8");
        let (decoded, scope) = BlankScope::unqualify_label(&label);
        let qualified = scope.qualify_label(&decoded);
        prop_assert!(text.starts_with(&format!("_:{qualified} ")), "{}", text);
    }

    /// The same inertness for the RDF/XML `NCName` alphabet: an in-alphabet
    /// label lands in `rdf:nodeID` verbatim.
    ///
    /// See `legal_blank_labels_pass_through_unescaped` above: `dataset_from_quads`
    /// goes through the owned-ingress boundary, so the expected wire token is
    /// the decode-then-requalify fixpoint of the generated string.
    #[test]
    fn legal_ncname_labels_pass_through_unescaped(label in arb_ncname_label()) {
        let dataset = dataset_from_quads(vec![RdfQuad::new(
            RdfTerm::blank_node(label.clone()),
            "https://example.org/p",
            RdfTerm::iri("https://example.org/o"),
        )]);
        let bytes = serialize(dataset.as_ref(), NativeRdfFormat::RdfXml);
        let text = String::from_utf8(bytes).expect("utf-8");
        let (decoded, scope) = BlankScope::unqualify_label(&label);
        let qualified = scope.qualify_label(&decoded);
        prop_assert!(text.contains(&format!("rdf:nodeID=\"{qualified}\"")), "{}", text);
    }

    /// Wrapper-agreement differential: `try_canonicalize_flat_view` agrees with the
    /// PRE-DELEGATION flatten-and-canonicalize route —
    /// `flat_dataset_from_quads(&flat_rdf_quads_from_dataset(&d))` then
    /// `canonicalize_with` — over generated datasets carrying scoped blanks (two
    /// independently-scoped sources), nested triple terms, blank graph names,
    /// and the `rdf:reifies` reifier "spelling": [`dataset_from_quad_sources`]
    /// (folds an `rdf:reifies` triple-term quad into a reifier binding, the same
    /// route production ingestion uses) and [`flat_dataset_from_quad_sources`]
    /// (leaves it a plain quad), chosen per generated case by `fold`.
    ///
    /// Deliberately generated WITHOUT the overlay's reserved sentinel
    /// (`arb_diff_sources_no_sentinel`, not `arb_diff_sources`): the reference route
    /// this property compares against shares the sentinel fold's defect (it never
    /// folds a base quad's sentinel spelling back into a statement-layer row
    /// either), so a sentinel-spelled input would make this an agreement between two
    /// wrong answers rather than a check. See
    /// `flat_canon_of_a_sentinel_spelled_reifier_matches_its_real_predicate_spelling`
    /// below for the property that covers the sentinel, as an IDENTITY law instead
    /// of an oracle comparison.
    #[test]
    fn flat_view_canon_matches_the_pre_delegation_flatten_route(
        (source0, source1) in arb_diff_sources_no_sentinel(),
        fold in any::<bool>(),
        sha384 in any::<bool>(),
    ) {
        let sources: [&[RdfQuad]; 2] = [&source0, &source1];
        let built = if fold {
            dataset_from_quad_sources(&sources)
        } else {
            flat_dataset_from_quad_sources(&sources)
        };
        let Ok(dataset) = built else {
            // A generated stream that fails to freeze (e.g. an `rdf:reifies` quad
            // whose object is not itself a triple term, under the folding route)
            // is not this property's concern — it is about agreement on datasets
            // that DO exist.
            return Ok(());
        };
        let hash = if sha384 { CanonHash::Sha384 } else { CanonHash::Sha256 };
        let reference = {
            let flat = flat_dataset_from_quads(&flat_rdf_quads_from_dataset(&dataset))
                .expect("an already-valid dataset's own flat quad stream must re-freeze");
            canonicalize_with(&flat, hash).nquads
        };
        match try_canonicalize_flat_view(&*dataset, hash) {
            Ok(canonicalized) => prop_assert_eq!(canonicalized.nquads, reference),
            Err(err) => prop_assert!(
                false,
                "generated non-reserved-vocabulary input was refused: {err}"
            ),
        }
    }

    /// The spelled-row identity law, generated: a base quad spelling a reifier row through the
    /// overlay's OWN reserved sentinel (`urn:purrdf:rdfc:reifies`, generated by
    /// [`arb_diff_quad`] via [`arb_diff_sources`]) must flat-canonicalize to the
    /// SAME bytes as the identical dataset with every such quad's predicate swapped
    /// for the row's real predicate (`rdf:reifies`) instead — the two spell the same
    /// content, so they must not split identity.
    ///
    /// An IDENTITY assertion, deliberately NOT an oracle comparison: the
    /// pre-delegation route `flat_view_canon_matches_the_pre_delegation_flatten_route`
    /// compares against shares this exact defect (it never folds a base quad's
    /// sentinel spelling back into a statement-layer row either), so comparing
    /// against it here would only prove the two agree on being wrong — this property
    /// instead compares the fixed implementation against ITSELF on the two
    /// content-equivalent spellings.
    ///
    /// A sentinel-spelled quad whose object is NOT a triple term is not a fold
    /// candidate at all (still reserved vocabulary, refused under both spellings'
    /// generation — see `the_fold_is_shape_exact_and_its_near_misses_still_refuse`
    /// in `purrdf-core`), so it is not this property's concern: the swapped dataset
    /// is trivially always admissible (the swap removes every reserved-namespace
    /// IRI), but the ORIGINAL is admissible only when every sentinel occurrence is
    /// in the fold's exact shape — exactly the condition this property gates on.
    #[test]
    fn flat_canon_of_a_sentinel_spelled_reifier_matches_its_real_predicate_spelling(
        (source0, source1) in arb_diff_sources(),
        fold in any::<bool>(),
        sha384 in any::<bool>(),
    ) {
        let sources: [&[RdfQuad]; 2] = [&source0, &source1];
        let built = if fold {
            dataset_from_quad_sources(&sources)
        } else {
            flat_dataset_from_quad_sources(&sources)
        };
        let Ok(dataset) = built else {
            return Ok(());
        };

        let swap = |quads: &[RdfQuad]| -> Vec<RdfQuad> {
            quads
                .iter()
                .cloned()
                .map(|mut q| {
                    if q.predicate == SENTINEL_REIFIES {
                        q.predicate = RDF_REIFIES.to_owned();
                    }
                    q
                })
                .collect()
        };
        let swapped0 = swap(&source0);
        let swapped1 = swap(&source1);
        let swapped_sources: [&[RdfQuad]; 2] = [&swapped0, &swapped1];
        let swapped_built = if fold {
            dataset_from_quad_sources(&swapped_sources)
        } else {
            flat_dataset_from_quad_sources(&swapped_sources)
        };
        // The swap can turn a sentinel-spelled quad into an ordinary `rdf:reifies`
        // quad the INGESTION-time fold (`fold == true`) then tries to fold too; if
        // ITS object is not a triple term either, freezing fails for the same
        // unrelated reason `built` above may have — not this property's concern.
        let Ok(swapped_dataset) = swapped_built else {
            return Ok(());
        };

        let hash = if sha384 { CanonHash::Sha384 } else { CanonHash::Sha256 };
        // The swap removes every reserved-namespace IRI, so the only way this can
        // still refuse is an unrelated poison-budget exhaustion — skip rather than
        // fail on that, since it is not what this property is about either.
        let Ok(swapped_canonicalized) = try_canonicalize_flat_view(&*swapped_dataset, hash) else {
            return Ok(());
        };

        match try_canonicalize_flat_view(&*dataset, hash) {
            Ok(canonicalized) => prop_assert_eq!(
                canonicalized.nquads,
                swapped_canonicalized.nquads,
                "a sentinel-spelled reifier row must flat-canonicalize identically \
                 to its real-predicate spelling"
            ),
            Err(_) => {
                // A sentinel occurrence that is NOT the fold's exact shape (a
                // non-triple-term object) is a genuine, unrelated refusal — the
                // original and the swapped dataset are not equivalent content in
                // that case, so there is nothing to compare.
                return Ok(());
            }
        }
    }
}

// ── Named regressions: dotted labels at the lexer hazard positions ──────────────

/// The distinct blank-node labels of a dataset (owned rendering, i.e. the
/// scope-qualified label the serializer writes).
fn distinct_blank_labels(dataset: &RdfDataset) -> std::collections::BTreeSet<String> {
    let mut labels = std::collections::BTreeSet::new();
    for quad in dataset.owned_quads() {
        for term in [&quad.subject, &quad.object] {
            if let RdfTerm::BlankNode(label) = term {
                labels.insert(label.clone());
            }
        }
    }
    labels
}

/// One default-graph quad whose subject blank carries the raw label `a.b`.
fn dotted_label_dataset() -> std::sync::Arc<RdfDataset> {
    dataset_from_quads(vec![RdfQuad::new(
        RdfTerm::blank_node("a.b"),
        "https://example.org/p",
        RdfTerm::iri("https://example.org/o"),
    )])
}

/// A raw interior dot reaches the N-Quads wire VERBATIM (`_:a.b`), the token
/// lexes back as ONE blank node, and the round trip is canonical.
#[test]
fn nquads_dotted_label_is_verbatim_on_wire_and_stays_one_node() {
    let dataset = dotted_label_dataset();
    let bytes = serialize(dataset.as_ref(), NativeRdfFormat::NQuads);
    let text = std::str::from_utf8(&bytes).expect("N-Quads output is UTF-8");
    assert!(
        text.contains("_:a.b "),
        "the raw interior dot must reach the wire verbatim: {text}"
    );
    let after = parse(&bytes, NativeRdfFormat::NQuads);
    assert_eq!(
        distinct_blank_labels(after.as_ref()).len(),
        1,
        "the dotted label must lex as exactly one blank node"
    );
    assert_eq!(
        canonical(&flat(dataset.as_ref())),
        canonical(after.as_ref())
    );
}

/// Round-trip `text` through `format` (parse → serialize → parse) and assert
/// the canonical quad set is stable and the blank-node count is `blanks`.
fn assert_text_roundtrip(text: &str, format: NativeRdfFormat, blanks: usize) {
    let first = parse(text.as_bytes(), format);
    assert_eq!(
        distinct_blank_labels(first.as_ref()).len(),
        blanks,
        "distinct blank nodes after the first parse of {text:?}"
    );
    let bytes = serialize(first.as_ref(), format);
    let second = parse(&bytes, format);
    assert_eq!(
        canonical(first.as_ref()),
        canonical(second.as_ref()),
        "re-serialization must round-trip {text:?}"
    );
    assert_eq!(
        distinct_blank_labels(second.as_ref()).len(),
        blanks,
        "distinct blank nodes after the round trip of {text:?}"
    );
}

/// Turtle predicate-list position: a dotted blank label immediately followed by
/// `;` must end at the `b` (never consume the `;`), yielding one node.
#[test]
fn turtle_dotted_label_followed_by_semicolon() {
    assert_text_roundtrip(
        "<https://example.org/s> <https://example.org/p> _:a..b;<https://example.org/q> \
         <https://example.org/o> .\n",
        NativeRdfFormat::Turtle,
        1,
    );
}

/// Turtle object-list position: a dotted blank label immediately followed by `,`.
#[test]
fn turtle_dotted_label_followed_by_comma() {
    assert_text_roundtrip(
        "<https://example.org/s> <https://example.org/p> _:a..b,<https://example.org/o> .\n",
        NativeRdfFormat::Turtle,
        1,
    );
}

/// Turtle bracketed property-list position: a dotted blank label immediately
/// followed by `]` (one dotted node plus the anonymous bracket node).
#[test]
fn turtle_dotted_label_followed_by_close_bracket() {
    assert_text_roundtrip(
        "<https://example.org/s> <https://example.org/p> [ <https://example.org/q> _:a..b] .\n",
        NativeRdfFormat::Turtle,
        2,
    );
}

/// TriG graph-block position: a dotted blank label immediately followed by `}`
/// (the block's final triple may omit its `.`).
#[test]
fn trig_dotted_label_followed_by_close_brace() {
    assert_text_roundtrip(
        "<https://example.org/g> { <https://example.org/s> <https://example.org/p> _:a..b}\n",
        NativeRdfFormat::TriG,
        1,
    );
}

/// The class that used to flake `legal_blank_labels_pass_through_unescaped` /
/// `legal_ncname_labels_pass_through_unescaped`: a raw label shaped like a
/// scope encoding (`a.s1`). It is now simply a LITERAL label — the owned
/// boundary decodes it to itself at the default scope, and every serializer
/// writes it back byte for byte — so the wire token is `a.s1` in both formats.
/// Pinned as a standalone, non-random regression so this exact class can never
/// flake silently again.
#[test]
fn a_scope_suffix_shaped_label_is_a_literal_label() {
    let (decoded, scope) = BlankScope::unqualify_label("a.s1");
    assert_eq!((decoded.as_ref(), scope.ordinal()), ("a.s1", 0));
    assert_eq!(scope.qualify_label(&decoded), "a.s1");

    let dataset = dataset_from_quads(vec![RdfQuad::new(
        RdfTerm::blank_node("a.s1"),
        "https://example.org/p",
        RdfTerm::iri("https://example.org/o"),
    )]);

    let nt_bytes = serialize(dataset.as_ref(), NativeRdfFormat::NTriples);
    let nt_text = String::from_utf8(nt_bytes).expect("utf-8");
    assert!(
        nt_text.starts_with("_:a.s1 "),
        "N-Triples must emit the literal label verbatim: {nt_text}"
    );

    let xml_bytes = serialize(dataset.as_ref(), NativeRdfFormat::RdfXml);
    let xml_text = String::from_utf8(xml_bytes).expect("utf-8");
    assert!(
        xml_text.contains("rdf:nodeID=\"a.s1\""),
        "RDF/XML must emit the literal label verbatim: {xml_text}"
    );
}

/// The RDF/XML serializer ESCAPES an NCName-illegal blank label (`rdf:nodeID`
/// cannot carry a digit-led label) instead of refusing, and the escaped
/// document round-trips to an isomorphic dataset.
#[test]
fn rdfxml_escapes_an_ncname_illegal_blank_label() {
    let dataset = dataset_from_quads(vec![RdfQuad::new(
        RdfTerm::blank_node("0abc"),
        "https://example.org/p",
        RdfTerm::iri("https://example.org/o"),
    )]);
    let bytes = serialize_dataset(
        dataset.as_ref(),
        NativeRdfFormat::RdfXml.media_type(),
        SerializeGraph::Dataset,
    )
    .expect("an NCName-illegal blank label is escaped, never refused");
    let text = String::from_utf8(bytes.clone()).expect("utf-8");
    assert!(
        text.contains("rdf:nodeID=\"purrdfesc_0abc\""),
        "the digit-led label must be escaped into an NCName: {text}"
    );
    let after = parse(&bytes, NativeRdfFormat::RdfXml);
    assert_eq!(
        canonical(&flat(dataset.as_ref())),
        canonical(after.as_ref()),
        "the escaped document is isomorphic to the input: {text}"
    );
}
