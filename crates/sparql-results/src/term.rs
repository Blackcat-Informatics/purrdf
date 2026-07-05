// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Shared term-lexicalization helpers, built on the `purrdf-core` kernel
//! authority — **not** a reimplementation of RDF term syntax.
//!
//! The bridge maps the egress [`TermValue`] (the per-cell value of a
//! `SparqlResult::Solutions` row, or a CONSTRUCT graph term) into the owned
//! [`RdfTerm`] model, then defers to the kernel's `emit_term` for N-Triples /
//! TSV lexicalization. The four W3C result-document writers (JSON/XML/CSV/TSV)
//! all consume these helpers so term syntax has exactly one source of truth.

use purrdf_core::{RdfLiteral, RdfTerm, RdfTriple, TermValue, emit_term};

/// The IRI of `xsd:string`, the implicit datatype of a plain (untyped,
/// non-language) literal. The egress model always populates `datatype`, so a
/// plain literal arrives as a literal carrying this IRI; the owned model and
/// Turtle/N-Triples abbreviate it to a bare `"lex"` form.
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

/// Bridge an egress [`TermValue`] into the owned [`RdfTerm`] model.
///
/// This is infallible by construction: every well-formed result cell maps to a
/// term. The one structural soft spot is a triple-term predicate that is not an
/// IRI (malformed RDF). Since the signature is infallible, we extract the IRI
/// when the predicate is an `Iri`, and otherwise fall back to the kernel
/// lexicalization of the bridged predicate with its `<>` delimiters stripped —
/// pragmatic, because predicates are IRIs in all real data.
// Consumed by the JSON/XML/CSV/TSV document writers landing in Tasks 2–3; the
// lib-only build can't see those call sites yet (the test module exercises it).
pub(crate) fn term_value_to_rdf_term(value: &TermValue) -> RdfTerm {
    match value {
        TermValue::Iri(s) => RdfTerm::iri(s.clone()),
        TermValue::Blank { label, scope } => {
            RdfTerm::blank_node(scope.qualify_label(label).into_owned())
        }
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => {
            if language.is_some() {
                RdfTerm::literal(RdfLiteral {
                    lexical_form: lexical_form.clone(),
                    datatype: None,
                    language: language.clone(),
                    direction: *direction,
                })
            } else if datatype == XSD_STRING {
                RdfTerm::literal(RdfLiteral {
                    lexical_form: lexical_form.clone(),
                    datatype: None,
                    language: None,
                    direction: None,
                })
            } else {
                RdfTerm::literal(RdfLiteral {
                    lexical_form: lexical_form.clone(),
                    datatype: Some(datatype.clone()),
                    language: None,
                    direction: *direction,
                })
            }
        }
        TermValue::Triple { s, p, o } => {
            let subject = term_value_to_rdf_term(s);
            let object = term_value_to_rdf_term(o);
            let predicate = predicate_iri(p);
            RdfTerm::triple(RdfTriple {
                subject,
                predicate,
                object,
                location: None,
            })
        }
    }
}

/// Extract the predicate IRI of a triple term. Predicates are IRIs in all valid
/// RDF; for a (malformed) non-IRI predicate we fall back to the kernel
/// lexicalization of the bridged term with the `<>` delimiters trimmed, so the
/// infallible bridge still produces a string rather than panicking.
fn predicate_iri(p: &TermValue) -> String {
    match p {
        TermValue::Iri(iri) => iri.clone(),
        other => {
            let token = emit_term(&term_value_to_rdf_term(other));
            token
                .strip_prefix('<')
                .and_then(|t| t.strip_suffix('>'))
                .map(str::to_string)
                .unwrap_or(token)
        }
    }
}

/// The N-Triples / TSV token for a result cell: the kernel `emit_term` over the
/// bridged owned term (`<iri>`, `_:label`, `"lex"` / `"lex"@lang` /
/// `"lex"^^<dt>`, or `<< s p o >>`).
pub(crate) fn ntriples_token(value: &TermValue) -> String {
    emit_term(&term_value_to_rdf_term(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use purrdf_core::BlankScope;

    #[test]
    fn iri_token() {
        let v = TermValue::Iri("http://example.org/s".to_string());
        assert_eq!(ntriples_token(&v), "<http://example.org/s>");
    }

    #[test]
    fn blank_default_scope_token() {
        let v = TermValue::Blank {
            label: "b0".to_string(),
            scope: BlankScope(0),
        };
        let token = ntriples_token(&v);
        assert!(token.starts_with("_:"), "expected blank node, got {token}");
    }

    #[test]
    fn blank_non_default_scope_distinct() {
        let a = TermValue::Blank {
            label: "b0".to_string(),
            scope: BlankScope(0),
        };
        let b = TermValue::Blank {
            label: "b0".to_string(),
            scope: BlankScope(7),
        };
        // Different scopes qualify the same label distinctly.
        assert_ne!(ntriples_token(&a), ntriples_token(&b));
    }

    #[test]
    fn simple_literal_is_bare() {
        let v = TermValue::Literal {
            lexical_form: "x".to_string(),
            datatype: "http://www.w3.org/2001/XMLSchema#string".to_string(),
            language: None,
            direction: None,
        };
        assert_eq!(ntriples_token(&v), "\"x\"");
    }

    #[test]
    fn typed_literal_carries_datatype() {
        let v = TermValue::Literal {
            lexical_form: "5".to_string(),
            datatype: "http://www.w3.org/2001/XMLSchema#integer".to_string(),
            language: None,
            direction: None,
        };
        assert_eq!(
            ntriples_token(&v),
            "\"5\"^^<http://www.w3.org/2001/XMLSchema#integer>"
        );
    }

    #[test]
    fn language_literal_carries_tag() {
        let v = TermValue::Literal {
            lexical_form: "x".to_string(),
            datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString".to_string(),
            language: Some("en".to_string()),
            direction: None,
        };
        assert_eq!(ntriples_token(&v), "\"x\"@en");
    }

    #[test]
    fn triple_term_token() {
        let v = TermValue::Triple {
            s: Box::new(TermValue::Iri("http://example.org/s".to_string())),
            p: Box::new(TermValue::Iri("http://example.org/p".to_string())),
            o: Box::new(TermValue::Iri("http://example.org/o".to_string())),
        };
        assert_eq!(
            ntriples_token(&v),
            "<< <http://example.org/s> <http://example.org/p> <http://example.org/o> >>"
        );
    }

    #[test]
    fn directional_literal_ltr_carries_direction_suffix() {
        use purrdf_core::RdfTextDirection;
        let v = TermValue::Literal {
            lexical_form: "hello".to_string(),
            datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString".to_string(),
            language: Some("en".to_string()),
            direction: Some(RdfTextDirection::Ltr),
        };
        let token = ntriples_token(&v);
        assert!(
            token.contains("--ltr"),
            "expected --ltr direction suffix in token, got: {token}"
        );
        assert_eq!(token, "\"hello\"@en--ltr");
    }
}
