// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Literal value-space *fidelity* tests for the native RDF codecs.
//!
//! # Why these assertions are RAW, never canonical
//!
//! The sibling `proptest_roundtrip.rs` compares both sides of a round-trip via the
//! RDFC-1.0 [`purrdf_rdf::canonical_flat_nquads`] comparator. RDFC-1.0 is allowed to
//! relabel blank nodes AND to rewrite literal lexical forms into canonical form, so a
//! comparator-mediated round-trip would happily *mask* a codec that normalizes the
//! value space (`"0.90"` → `"0.9"`) or narrows a datatype
//! (`xsd:nonNegativeInteger` → `xsd:integer`). These tests therefore assert on the
//! **raw serialized bytes** (exact `^^<datatype-IRI>` substrings) and on a **by-value
//! term lookup** ([`RdfDataset::term_id_by_value`]) after a re-parse — never through the
//! canonical comparator. The acceptance criterion is byte-faithful lexical form + exact
//! datatype IRI on both legs of `parse ∘ serialize`.
//!
//! Fidelity literals exercised (each must survive verbatim, no narrowing):
//! `"0.90"^^xsd:decimal`, `"007"^^xsd:nonNegativeInteger`, `"1.0E0"^^xsd:double`,
//! `"+1.5"^^xsd:decimal`, `"0.50"^^xsd:decimal`.

use purrdf_rdf::{
    NativeRdfFormat, RdfDatasetBuilder, RdfLiteral, RdfQuad, RdfTerm, SerializeGraph, TermValue,
    parse_dataset, serialize_dataset,
};

const XSD_DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";
const XSD_NON_NEGATIVE_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#nonNegativeInteger";
const XSD_DOUBLE: &str = "http://www.w3.org/2001/XMLSchema#double";

const SUBJECT: &str = "https://e/s";
const PREDICATE: &str = "https://e/p";

/// One fidelity literal: its exact lexical form and exact datatype IRI. The codecs must
/// reproduce BOTH verbatim — no value-space normalization, no datatype narrowing.
struct Fidelity {
    lexical: &'static str,
    datatype: &'static str,
}

const FIDELITY_LITERALS: &[Fidelity] = &[
    // Trailing zero must survive — NOT "0.9".
    Fidelity {
        lexical: "0.90",
        datatype: XSD_DECIMAL,
    },
    // Leading zeros AND the narrower datatype must survive — NOT "7", NOT xsd:integer.
    Fidelity {
        lexical: "007",
        datatype: XSD_NON_NEGATIVE_INTEGER,
    },
    Fidelity {
        lexical: "1.0E0",
        datatype: XSD_DOUBLE,
    },
    // Leading plus sign must survive — NOT "1.5".
    Fidelity {
        lexical: "+1.5",
        datatype: XSD_DECIMAL,
    },
    Fidelity {
        lexical: "0.50",
        datatype: XSD_DECIMAL,
    },
];

/// Build the single-quad dataset `<https://e/s> <https://e/p> "lexical"^^<datatype>`.
fn dataset_with_literal(lexical: &str, datatype: &str) -> std::sync::Arc<purrdf_rdf::RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let quad = RdfQuad::new(
        RdfTerm::iri(SUBJECT.to_owned()),
        PREDICATE.to_owned(),
        RdfTerm::literal(RdfLiteral::typed(lexical.to_owned(), datatype.to_owned())),
    );
    b.push_owned_quad(&quad);
    b.freeze().expect("fidelity dataset must freeze")
}

/// The exact N-Triples/N-Quads/Turtle/TriG token a fidelity literal must serialize to:
/// the verbatim lexical form quoted, then `^^` and the FULL `<datatype-IRI>` (the native
/// text serializers never abbreviate the datatype against a declared prefix).
fn expected_text_token(f: &Fidelity) -> String {
    format!("\"{}\"^^<{}>", f.lexical, f.datatype)
}

/// Round-trip a single fidelity literal through one text codec and assert RAW fidelity on
/// both legs: the serialized bytes contain the exact `^^<…>` token, and the re-parsed
/// dataset contains a literal with the exact lexical form + datatype (by-value lookup).
fn assert_text_fidelity(format: NativeRdfFormat, f: &Fidelity) {
    let ds = dataset_with_literal(f.lexical, f.datatype);

    let bytes = serialize_dataset(&ds, format.media_type(), SerializeGraph::Dataset)
        .expect("native serialize");
    let text = std::str::from_utf8(&bytes).expect("serialized RDF must be UTF-8");

    let token = expected_text_token(f);
    assert!(
        text.contains(&token),
        "{format:?}: raw output must carry the verbatim lexical form + full datatype IRI {token:?}, \
         got:\n{text}",
    );

    let after = parse_dataset(&bytes, format.media_type(), None).expect("native parse");
    assert!(
        after
            .term_id_by_value(&TermValue::Literal {
                lexical_form: f.lexical.to_owned(),
                datatype: f.datatype.to_owned(),
                language: None,
                direction: None,
            })
            .is_some(),
        "{:?}: re-parsed literal must keep EXACT lexical form {:?} + datatype {:?} \
         (no canonicalization, no narrowing)",
        format,
        f.lexical,
        f.datatype,
    );
}

#[test]
fn ntriples_preserves_literal_value_space() {
    for f in FIDELITY_LITERALS {
        assert_text_fidelity(NativeRdfFormat::NTriples, f);
    }
}

#[test]
fn nquads_preserves_literal_value_space() {
    for f in FIDELITY_LITERALS {
        assert_text_fidelity(NativeRdfFormat::NQuads, f);
    }
}

#[test]
fn turtle_preserves_literal_value_space() {
    for f in FIDELITY_LITERALS {
        assert_text_fidelity(NativeRdfFormat::Turtle, f);
    }
}

#[test]
fn trig_preserves_literal_value_space() {
    for f in FIDELITY_LITERALS {
        assert_text_fidelity(NativeRdfFormat::TriG, f);
    }
}

/// JSON-LD fidelity: the native JSON-LD codec must emit the verbatim lexical form as a
/// raw `@value` string and the exact datatype, and re-parse to the same lexical form +
/// datatype IRI. Asserted on the RAW JSON text + a by-value term lookup, NEVER via the
/// canonical comparator.
#[test]
fn jsonld_preserves_literal_value_space() {
    use purrdf_rdf::native_codecs::jsonld::{parse_jsonld, serialize_dataset_to_jsonld};

    for f in FIDELITY_LITERALS {
        let ds = dataset_with_literal(f.lexical, f.datatype);

        let json = serialize_dataset_to_jsonld(&ds).expect("native JSON-LD serialize");
        // The lexical form rides through verbatim as a quoted @value string ("0.90", not
        // "0.9"; "007", not "7"; "+1.5", not "1.5").
        let expected_value = format!("\"{}\"", f.lexical);
        assert!(
            json.contains(&expected_value),
            "JSON-LD: @value must carry the verbatim lexical form {expected_value}, got:\n{json}"
        );

        let after = parse_jsonld(json.as_bytes()).expect("native JSON-LD parse");
        assert!(
            after
                .term_id_by_value(&TermValue::Literal {
                    lexical_form: f.lexical.to_owned(),
                    datatype: f.datatype.to_owned(),
                    language: None,
                    direction: None,
                })
                .is_some(),
            "JSON-LD: re-parsed literal must keep EXACT lexical form {:?} + datatype {:?}",
            f.lexical,
            f.datatype,
        );
    }
}

/// The JSON-LD-star statement-metadata downcast mints IRIs in the CALLER's
/// vocabulary only: star input without a configured vocab must hard-fail
/// (PurRDF is not an ontology and fabricates no default namespace), while
/// star-free input downcasts fine unconfigured.
#[test]
fn jsonld_statement_metadata_downcast_requires_caller_vocab() {
    use purrdf_rdf::native_codecs::jsonld::{
        StatementMetadataVocab, jsonld_to_statement_metadata_nquads,
    };

    // Star-bearing JSON-LD: an `@annotation` reifier on the base triple.
    let star_json = r#"{
      "@graph": [
        {
          "@id": "https://e/s",
          "https://e/p": {
            "@id": "https://e/o",
            "@annotation": {
              "@id": "https://e/r",
              "https://e/source": { "@id": "https://e/doc" }
            }
          }
        }
      ]
    }"#;

    // Unconfigured + star features = hard error, never a fabricated namespace.
    let err = jsonld_to_statement_metadata_nquads(star_json.as_bytes(), None)
        .expect_err("star downcast without a vocab must fail closed");
    assert!(
        err.to_string()
            .contains("requires a statement-metadata vocabulary"),
        "unexpected error: {err}"
    );

    // A caller-supplied vocabulary lowers into the caller's namespace.
    let vocab = StatementMetadataVocab {
        statement_metadata: "https://example.org/meta/StatementMetadata",
        q_subject: "https://example.org/meta/qSubject",
        q_predicate: "https://example.org/meta/qPredicate",
        q_object: "https://example.org/meta/qObject",
        q_object_literal: "https://example.org/meta/qObjectLiteral",
    };
    let nq = jsonld_to_statement_metadata_nquads(star_json.as_bytes(), Some(&vocab))
        .expect("star downcast with an explicit vocab");
    assert!(nq.contains("https://example.org/meta/StatementMetadata"));
    assert!(nq.contains("https://example.org/meta/qSubject"));
    assert!(
        !nq.contains("<<("),
        "downcast output must be star-free N-Quads:\n{nq}"
    );

    // Star-free input needs no vocabulary at all.
    let plain_json = r#"{
      "@graph": [
        { "@id": "https://e/s", "https://e/p": { "@id": "https://e/o" } }
      ]
    }"#;
    let nq = jsonld_to_statement_metadata_nquads(plain_json.as_bytes(), None)
        .expect("star-free downcast must work unconfigured");
    assert!(nq.contains("<https://e/s> <https://e/p> <https://e/o> ."));
}
