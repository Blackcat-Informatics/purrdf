// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! W3C-conformance golden corpus for the native SPARQL Results serializer
//! (purrdf S9, EPIC #906 / #915).
//!
//! A single, realistic "books" running-example dataset (the shape from the W3C
//! SPARQL Results spec) is serialized to ALL FOUR formats (JSON/XML/CSV/TSV) and
//! snapshotted with `insta`, exercising every RDF term kind in one coherent
//! dataset: bound IRIs, plain `xsd:string` literals, an unbound cell, a
//! language-tagged literal, a typed (`xsd:integer`) literal, and a blank node.
//! Beyond the SELECT corpus this also pins the ASK boolean paths, the maximal
//! RDF-1.2-star CONSTRUCT graph path (a quad plus a reifier and an annotation),
//! and the populated-`purrdf`-provenance behaviour across every format — including
//! the CSV/TSV no-silent-cap exit-gate trim, which is *signalled* via
//! `provenance_dropped`, never hidden.
//!
//! These goldens are the cross-format authority: the per-`src` unit tests pin
//! exact substrings, while these snapshots pin the WHOLE document for each
//! format against the W3C format specs.

use purrdf_core::{
    BlankScope, RdfAnnotation, RdfDatasetBuilder, RdfLiteral, RdfQuad, RdfReifier, RdfTerm,
    RdfTextDirection, RdfTriple, TermValue,
};
use purrdf_sparql_results::{
    serialize, to_csv, to_tsv, ResultProvenance, SolutionProvenance, SparqlResult,
    SparqlResultsFormat,
};

const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
const RDF_LANGSTRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";
const RDF_DIRLANGSTRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString";

/// A plain (untyped, non-language) literal — i.e. an `xsd:string`.
fn plain(lex: &str) -> TermValue {
    TermValue::Literal {
        lexical_form: lex.to_string(),
        datatype: XSD_STRING.to_string(),
        language: None,
        direction: None,
    }
}

/// A typed `xsd:integer` literal.
fn integer(lex: &str) -> TermValue {
    TermValue::Literal {
        lexical_form: lex.to_string(),
        datatype: XSD_INTEGER.to_string(),
        language: None,
        direction: None,
    }
}

/// A language-tagged literal.
fn lang(lex: &str, tag: &str) -> TermValue {
    TermValue::Literal {
        lexical_form: lex.to_string(),
        datatype: RDF_LANGSTRING.to_string(),
        language: Some(tag.to_string()),
        direction: None,
    }
}

/// A directional (base-direction-carrying) language-tagged literal — RDF-1.2
/// `rdf:dirLangString`.
fn dir_lang(lex: &str, tag: &str, direction: RdfTextDirection) -> TermValue {
    TermValue::Literal {
        lexical_form: lex.to_string(),
        datatype: RDF_DIRLANGSTRING.to_string(),
        language: Some(tag.to_string()),
        direction: Some(direction),
    }
}

fn iri(s: &str) -> TermValue {
    TermValue::Iri(s.to_string())
}

/// The shared W3C "running example" books dataset.
///
/// Variables `?book ?title`, with rows that — taken together — exercise every
/// RDF term kind across both columns:
///   * book6 → an IRI + a plain `xsd:string` title (the textbook row).
///   * book7 → an IRI + a language-tagged title.
///   * an unbound `book` (None) + a plain title (the unbound-cell row).
///   * a blank-node book + a typed `xsd:integer` "title" (edition number) — the
///     blank-node and typed-literal carriers, in one row.
fn books() -> SparqlResult {
    SparqlResult::Solutions {
        variables: vec!["book".to_string(), "title".to_string()],
        rows: vec![
            vec![
                Some(iri("http://example.org/book/book6")),
                Some(plain("Harry Potter and the Half-Blood Prince")),
            ],
            vec![
                Some(iri("http://example.org/book/book7")),
                Some(lang("Harry Potter et l'Ordre du Phénix", "fr")),
            ],
            vec![None, Some(plain("Anonymous Anthology"))],
            vec![
                Some(TermValue::Blank {
                    label: "draft".to_string(),
                    scope: BlankScope(0),
                }),
                Some(integer("5")),
            ],
        ],
        aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
    }
}

/// A non-empty provenance carrier for the populated-path snapshots.
fn populated_provenance() -> ResultProvenance {
    ResultProvenance {
        query_hash: Some("sha256:cafebabe".to_string()),
        engine: Some("purrdf-sparql-eval".to_string()),
        solutions: vec![SolutionProvenance {
            sources: vec!["http://example.org/graph/library".to_string()],
        }],
    }
}

fn text(result: &SparqlResult, format: SparqlResultsFormat, prov: &ResultProvenance) -> String {
    let outcome = serialize(result, format, prov).expect("serialization succeeds");
    String::from_utf8(outcome.bytes).expect("UTF-8 output")
}

// ---------------------------------------------------------------------------
// 1. SELECT — the books dataset, all term kinds, in all four formats.
// ---------------------------------------------------------------------------

#[test]
fn select_books_json() {
    insta::assert_snapshot!(text(
        &books(),
        SparqlResultsFormat::Json,
        &ResultProvenance::default()
    ));
}

#[test]
fn select_books_xml() {
    insta::assert_snapshot!(text(
        &books(),
        SparqlResultsFormat::Xml,
        &ResultProvenance::default()
    ));
}

#[test]
fn select_books_csv() {
    // `insta` normalizes CRLF→LF in text snapshots, so the snapshot pins the
    // content shape (bare header, bare values, `_:draft`, empty unbound field,
    // RFC-4180 form) while the byte-level CRLF requirement is asserted here on
    // the raw bytes.
    let outcome = serialize(
        &books(),
        SparqlResultsFormat::Csv,
        &ResultProvenance::default(),
    )
    .expect("csv serializes");
    let raw = String::from_utf8(outcome.bytes).expect("UTF-8");
    assert!(
        raw.contains("\r\n") && !raw.contains("\n\n") && !raw.replace("\r\n", "").contains('\n'),
        "CSV records must be CRLF-terminated (RFC 4180): {raw:?}"
    );
    insta::assert_snapshot!(raw);
}

#[test]
fn select_books_tsv() {
    // TSV uses bare LF line ends (no CR); pinned on the raw bytes here, with the
    // content shape (`?`-prefixed header, Turtle-syntax terms) in the snapshot.
    let outcome = serialize(
        &books(),
        SparqlResultsFormat::Tsv,
        &ResultProvenance::default(),
    )
    .expect("tsv serializes");
    let raw = String::from_utf8(outcome.bytes).expect("UTF-8");
    assert!(
        !raw.contains('\r'),
        "TSV must use bare LF line ends, no CR: {raw:?}"
    );
    assert!(raw.starts_with("?book\t?title\n"), "TSV header: {raw:?}");
    insta::assert_snapshot!(raw);
}

// ---------------------------------------------------------------------------
// 2. ASK + CONSTRUCT — JSON + XML snapshot; CSV/TSV must Err (no W3C
//    tabular representation exists for either an ASK boolean or a CONSTRUCT
//    graph).
// ---------------------------------------------------------------------------

#[test]
fn ask_true_json() {
    insta::assert_snapshot!(text(
        &SparqlResult::Boolean(true),
        SparqlResultsFormat::Json,
        &ResultProvenance::default()
    ));
}

#[test]
fn ask_true_xml() {
    insta::assert_snapshot!(text(
        &SparqlResult::Boolean(true),
        SparqlResultsFormat::Xml,
        &ResultProvenance::default()
    ));
}

#[test]
fn csv_and_tsv_reject_non_tabular_results() {
    // CSV and TSV are defined ONLY for SELECT variable bindings; ASK booleans
    // and CONSTRUCT graphs have no W3C tabular representation → both must `Err`.
    // (`is_err()` rather than `matches!(.., Err(_))` to satisfy clippy's
    // `redundant_pattern_matching`; semantically the same assertion.)
    let prov = ResultProvenance::default();
    assert!(to_csv(&SparqlResult::Boolean(true), &prov).is_err());
    assert!(to_tsv(&SparqlResult::Boolean(true), &prov).is_err());
    assert!(to_csv(&starred_graph(), &prov).is_err());
    assert!(to_tsv(&starred_graph(), &prov).is_err());
}

// ---------------------------------------------------------------------------
// 3. CONSTRUCT — maximal RDF-1.2-star graph: a quad + a reifier + an annotation.
// ---------------------------------------------------------------------------

/// Build a CONSTRUCT-result dataset that carries one base quad, one reifier
/// (`rdf:reifies` over a triple term), and one statement annotation on that
/// reifier — so the embedded N-Triples demonstrates the maximal star path.
fn starred_graph() -> SparqlResult {
    let mut builder = RdfDatasetBuilder::new();

    // Base quad: book6 dc:title "Harry Potter and the Half-Blood Prince".
    let title_lit = RdfTerm::literal(RdfLiteral {
        lexical_form: "Harry Potter and the Half-Blood Prince".to_string(),
        datatype: None,
        language: None,
        direction: None,
    });
    builder.push_owned_quad(&RdfQuad {
        subject: RdfTerm::iri("http://example.org/book/book6"),
        predicate: "http://purl.org/dc/elements/1.1/title".to_string(),
        object: title_lit.clone(),
        graph_name: None,
        location: None,
    });

    // Reifier: <stmt1> rdf:reifies << book6 dc:title "…" >>.
    let statement = RdfTriple {
        subject: RdfTerm::iri("http://example.org/book/book6"),
        predicate: "http://purl.org/dc/elements/1.1/title".to_string(),
        object: title_lit,
        location: None,
    };
    let reifier_term = RdfTerm::iri("http://example.org/stmt/stmt1");
    let reifier = RdfReifier::new(reifier_term.clone(), statement.clone());
    let reifier_id = builder.intern_owned_term(&reifier.reifier);
    let triple_id = builder.intern_owned_term(&RdfTerm::triple(statement));
    builder.push_reifier(reifier_id, triple_id);

    // Annotation on the reifier: <stmt1> ex:source <wikipedia>.
    let annotation = RdfAnnotation::new(
        reifier_term,
        "http://example.org/vocab#source",
        RdfTerm::iri("https://en.wikipedia.org/wiki/Half-Blood_Prince"),
    );
    let ann_reifier_id = builder.intern_owned_term(&annotation.reifier);
    let ann_pred_id = builder.intern_iri(annotation.predicate.clone());
    let ann_obj_id = builder.intern_owned_term(&annotation.object);
    builder.push_annotation(ann_reifier_id, ann_pred_id, ann_obj_id);

    let dataset = builder.freeze().expect("starred dataset freezes");
    SparqlResult::Graph(dataset)
}

#[test]
fn construct_starred_graph_json() {
    insta::assert_snapshot!(text(
        &starred_graph(),
        SparqlResultsFormat::Json,
        &ResultProvenance::default()
    ));
}

// ---------------------------------------------------------------------------
// 4. POPULATED PROVENANCE — JSON/XML carry the purrdf extension; CSV/TSV trim it
//    and SIGNAL the drop (no silent cap).
// ---------------------------------------------------------------------------

#[test]
fn select_books_json_with_provenance() {
    insta::assert_snapshot!(text(
        &books(),
        SparqlResultsFormat::Json,
        &populated_provenance()
    ));
}

#[test]
fn select_books_xml_with_provenance() {
    insta::assert_snapshot!(text(
        &books(),
        SparqlResultsFormat::Xml,
        &populated_provenance()
    ));
}

#[test]
fn select_books_csv_drops_provenance_and_stays_pure() {
    let outcome = serialize(&books(), SparqlResultsFormat::Csv, &populated_provenance())
        .expect("csv serializes");
    assert!(
        outcome.provenance_dropped,
        "CSV must signal the provenance drop"
    );
    let body = String::from_utf8(outcome.bytes).expect("UTF-8");
    assert!(
        !body.contains("purrdf"),
        "CSV must stay pure W3C (no purrdf leak): {body}"
    );
}

#[test]
fn select_books_tsv_drops_provenance_and_stays_pure() {
    let outcome = serialize(&books(), SparqlResultsFormat::Tsv, &populated_provenance())
        .expect("tsv serializes");
    assert!(
        outcome.provenance_dropped,
        "TSV must signal the provenance drop"
    );
    let body = String::from_utf8(outcome.bytes).expect("UTF-8");
    assert!(
        !body.contains("purrdf"),
        "TSV must stay pure W3C (no purrdf leak): {body}"
    );
}

// ---------------------------------------------------------------------------
// 5. EDGE CASES — directional (rtl/ltr) language literals + escaping triggers,
//    across all four formats.  Verifies Gap 3 (direction suffix) flows through
//    TSV/CSV/JSON/XML, and that escaping-trigger characters render correctly per
//    each format's spec.
// ---------------------------------------------------------------------------

/// Edge-case dataset pinning the two cross-format dimensions the books corpus
/// omits: an RDF-1.2 directional (rtl) language literal, and a cell whose value
/// triggers escaping in every format (comma/quote for CSV RFC-4180, `&`/`<` for
/// XML/JSON).
fn edge_cases() -> SparqlResult {
    SparqlResult::Solutions {
        variables: vec!["term".to_string(), "note".to_string()],
        rows: vec![
            // Directional rtl literal (Arabic) + a plain note.
            vec![
                Some(dir_lang("مرحبا", "ar", RdfTextDirection::Rtl)),
                Some(plain("right-to-left greeting")),
            ],
            // Escaping-trigger cell + an ltr directional literal.
            vec![
                Some(plain("a, \"b\" & <c>")),
                Some(dir_lang("hello", "en", RdfTextDirection::Ltr)),
            ],
        ],
        aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
    }
}

#[test]
fn edge_cases_json() {
    insta::assert_snapshot!(text(
        &edge_cases(),
        SparqlResultsFormat::Json,
        &ResultProvenance::default()
    ));
}

#[test]
fn edge_cases_xml() {
    insta::assert_snapshot!(text(
        &edge_cases(),
        SparqlResultsFormat::Xml,
        &ResultProvenance::default()
    ));
}

#[test]
fn edge_cases_csv() {
    // `insta` normalizes CRLF→LF in text snapshots, so the snapshot pins the
    // content shape while the byte-level CRLF requirement is asserted on raw bytes.
    let outcome = serialize(
        &edge_cases(),
        SparqlResultsFormat::Csv,
        &ResultProvenance::default(),
    )
    .expect("csv serializes");
    let raw = String::from_utf8(outcome.bytes).expect("UTF-8");
    assert!(
        raw.contains("\r\n") && !raw.replace("\r\n", "").contains('\n'),
        "edge-case CSV must use exclusive CRLF line endings (RFC 4180): {raw:?}"
    );
    insta::assert_snapshot!(raw);
}

#[test]
fn edge_cases_tsv() {
    // TSV uses bare LF line ends (no CR); the direction suffix (--rtl/--ltr)
    // must appear in the output — this is the proof Gap 3 flows through TSV.
    let outcome = serialize(
        &edge_cases(),
        SparqlResultsFormat::Tsv,
        &ResultProvenance::default(),
    )
    .expect("tsv serializes");
    let raw = String::from_utf8(outcome.bytes).expect("UTF-8");
    assert!(
        !raw.contains('\r'),
        "TSV must use bare LF line ends, no CR: {raw:?}"
    );
    assert!(
        raw.contains("--rtl") || raw.contains("--ltr"),
        "TSV must carry direction suffix from Gap 3 kernel fix: {raw:?}"
    );
    insta::assert_snapshot!(raw);
}
