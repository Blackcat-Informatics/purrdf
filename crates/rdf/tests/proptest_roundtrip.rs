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
    BlankScope, NativeRdfFormat, RdfDataset, RdfDatasetBuilder, RdfLiteral, RdfLookaside, RdfQuad,
    RdfTerm, RdfTriple, SerializeGraph, canonical_flat_nquads, flat_rdf_quads_from_dataset,
    parse_dataset, serialize_dataset,
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
    #[test]
    fn legal_blank_labels_pass_through_unescaped(label in arb_bnode_label()) {
        let dataset = dataset_from_quads(vec![RdfQuad::new(
            RdfTerm::blank_node(label.clone()),
            "https://example.org/p",
            RdfTerm::iri("https://example.org/o"),
        )]);
        let bytes = serialize(dataset.as_ref(), NativeRdfFormat::NTriples);
        let text = String::from_utf8(bytes).expect("utf-8");
        let qualified = BlankScope::DEFAULT.qualify_label(&label);
        prop_assert!(text.starts_with(&format!("_:{qualified} ")), "{}", text);
    }

    /// The same inertness for the RDF/XML `NCName` alphabet: an in-alphabet
    /// label lands in `rdf:nodeID` verbatim.
    #[test]
    fn legal_ncname_labels_pass_through_unescaped(label in arb_ncname_label()) {
        let dataset = dataset_from_quads(vec![RdfQuad::new(
            RdfTerm::blank_node(label.clone()),
            "https://example.org/p",
            RdfTerm::iri("https://example.org/o"),
        )]);
        let bytes = serialize(dataset.as_ref(), NativeRdfFormat::RdfXml);
        let text = String::from_utf8(bytes).expect("utf-8");
        let qualified = BlankScope::DEFAULT.qualify_label(&label);
        prop_assert!(text.contains(&format!("rdf:nodeID=\"{qualified}\"")), "{}", text);
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

/// A raw interior dot is doubled on the N-Quads wire (`_:a.b` → `_:a..b`), the
/// doubled token lexes back as ONE blank node, and the round trip is canonical.
#[test]
fn nquads_dotted_label_doubles_on_wire_and_stays_one_node() {
    let dataset = dotted_label_dataset();
    let bytes = serialize(dataset.as_ref(), NativeRdfFormat::NQuads);
    let text = std::str::from_utf8(&bytes).expect("N-Quads output is UTF-8");
    assert!(
        text.contains("_:a..b "),
        "the raw interior dot must be doubled on the wire: {text}"
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
