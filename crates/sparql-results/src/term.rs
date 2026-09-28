// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Shared term-lexicalization helpers, built on the `purrdf-core` kernel
//! authority — **not** a reimplementation of RDF term syntax.
//!
//! The bridge maps the egress [`TermValue`] (the per-cell value of a
//! `SparqlResult::Solutions` row, or a CONSTRUCT graph term) into the owned
//! [`RdfTerm`] model, then defers to the kernel's `emit_term` for N-Triples /
//! TSV lexicalization. The four W3C result-document writers (JSON/XML/CSV/TSV)
//! all consume these helpers so term syntax has exactly one source of truth.

use crate::error::Error;
use purrdf_core::{RdfLiteral, RdfTerm, RdfTriple, TermValue, emit_term};

/// The IRI of `xsd:string`, the implicit datatype of a plain (untyped,
/// non-language) literal. The egress model always populates `datatype`, so a
/// plain literal arrives as a literal carrying this IRI; the owned model and
/// Turtle/N-Triples abbreviate it to a bare `"lex"` form.
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

/// Bridge an egress [`TermValue`] into the owned [`RdfTerm`] model.
///
/// # Errors
///
/// RDF 1.2 requires a triple-term predicate to be an IRI. Returns
/// [`Error::MalformedTerm`] when a triple term at any nesting depth carries a
/// predicate that is not an IRI — that is malformed RDF, so it is rejected rather
/// than laundered into a fabricated IRI string.
///
/// A triple term is bridged bottom-up over [`TermValue::try_fold`]'s work list.
// The shared TermValue → owned-model bridge every result-document writer
// lexicalizes through, so term syntax has exactly one source of truth.
pub(crate) fn term_value_to_rdf_term(value: &TermValue) -> Result<RdfTerm, Error> {
    value.try_fold(
        |leaf| {
            Ok(match leaf {
                TermValue::Iri(s) => RdfTerm::iri(s.clone()),
                // The owned model has ONE string slot for a blank node, so the
                // `(label, scope)` pair is encoded into it under the unconstrained
                // owned alphabet. The kernel emitter RE-TARGETS that spelling into
                // `BLANK_NODE_LABEL` when it writes the `_:` token, so the CSV/TSV
                // cell carries exactly what the JSON/XML writers encode directly.
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
                TermValue::Triple { .. } => unreachable!("a triple term is folded from its parts"),
            })
        },
        |subject, predicate, object| {
            // RDF predicates must be IRIs; a non-IRI predicate has no valid
            // lexicalization → hard-fail rather than fabricate one.
            let RdfTerm::Iri(predicate) = predicate else {
                return Err(Error::MalformedTerm(
                    "triple-term predicate is not an IRI".to_string(),
                ));
            };
            Ok(RdfTerm::triple(RdfTriple {
                subject,
                predicate,
                object,
                location: None,
            }))
        },
    )
}

/// The N-Triples / TSV token for a result cell: the kernel `emit_term` over the
/// bridged owned term (`<iri>`, `_:label`, `"lex"` / `"lex"@lang` /
/// `"lex"^^<dt>`, or the non-asserting triple term `<<( s p o )>>`).
///
/// Total: these tokens must re-lex as Turtle terms, and the kernel guarantees
/// that by escaping a blank-node label outside the Turtle `BLANK_NODE_LABEL`
/// alphabet into it — deterministically and injectively, so distinct blank
/// nodes stay distinct across the whole result document.
///
/// # Errors
///
/// Returns [`Error::MalformedTerm`] under the same condition as
/// [`term_value_to_rdf_term`]: a triple term (at any nesting depth) whose
/// predicate is not an IRI.
pub(crate) fn ntriples_token(value: &TermValue) -> Result<String, Error> {
    term_value_to_rdf_term(value).map(|term| emit_term(&term))
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_core::BlankScope;
    use purrdf_core::TermBox;

    /// Test-only helper: unwrap the `Result` for the common well-formed case
    /// so existing assertions stay terse.
    fn token(v: &TermValue) -> String {
        ntriples_token(v).expect("well-formed term")
    }

    #[test]
    fn iri_token() {
        let v = TermValue::Iri("http://example.org/s".to_string());
        assert_eq!(token(&v), "<http://example.org/s>");
    }

    #[test]
    fn blank_default_scope_token() {
        let v = TermValue::Blank {
            label: "b0".to_string(),
            scope: BlankScope(0),
        };
        let t = token(&v);
        assert!(t.starts_with("_:"), "expected blank node, got {t}");
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
        assert_ne!(token(&a), token(&b));
    }

    #[test]
    fn simple_literal_is_bare() {
        let v = TermValue::Literal {
            lexical_form: "x".to_string(),
            datatype: "http://www.w3.org/2001/XMLSchema#string".to_string(),
            language: None,
            direction: None,
        };
        assert_eq!(token(&v), "\"x\"");
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
            token(&v),
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
        assert_eq!(token(&v), "\"x\"@en");
    }

    #[test]
    fn triple_term_token() {
        let v = TermValue::Triple {
            s: TermBox::new(TermValue::Iri("http://example.org/s".to_string())),
            p: TermBox::new(TermValue::Iri("http://example.org/p".to_string())),
            o: TermBox::new(TermValue::Iri("http://example.org/o".to_string())),
        };
        assert_eq!(
            token(&v),
            "<<( <http://example.org/s> <http://example.org/p> <http://example.org/o> )>>"
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
        let t = token(&v);
        assert!(
            t.contains("--ltr"),
            "expected --ltr direction suffix in token, got: {t}"
        );
        assert_eq!(t, "\"hello\"@en--ltr");
    }

    #[test]
    fn literal_predicate_is_malformed_term_error() {
        let v = TermValue::Triple {
            s: TermBox::new(TermValue::Iri("http://example.org/s".to_string())),
            p: TermBox::new(TermValue::Literal {
                lexical_form: "not-a-predicate".to_string(),
                datatype: XSD_STRING.to_string(),
                language: None,
                direction: None,
            }),
            o: TermBox::new(TermValue::Iri("http://example.org/o".to_string())),
        };
        let err = ntriples_token(&v).expect_err("literal predicate must be rejected");
        assert!(
            matches!(err, Error::MalformedTerm(_)),
            "expected MalformedTerm: {err:?}"
        );
    }

    #[test]
    fn blank_predicate_is_malformed_term_error() {
        let v = TermValue::Triple {
            s: TermBox::new(TermValue::Iri("http://example.org/s".to_string())),
            p: TermBox::new(TermValue::Blank {
                label: "b0".to_string(),
                scope: BlankScope(0),
            }),
            o: TermBox::new(TermValue::Iri("http://example.org/o".to_string())),
        };
        let err = ntriples_token(&v).expect_err("blank-node predicate must be rejected");
        assert!(
            matches!(err, Error::MalformedTerm(_)),
            "expected MalformedTerm: {err:?}"
        );
    }

    #[test]
    fn nested_triple_term_non_iri_predicate_is_malformed_term_error() {
        // The inner triple term (used as the outer subject) carries a
        // non-IRI predicate; the outer triple term's own predicate is fine.
        // The recursive bridge must still reject this.
        let inner = TermValue::Triple {
            s: TermBox::new(TermValue::Iri("http://example.org/s".to_string())),
            p: TermBox::new(TermValue::Blank {
                label: "b0".to_string(),
                scope: BlankScope(0),
            }),
            o: TermBox::new(TermValue::Iri("http://example.org/o".to_string())),
        };
        let outer = TermValue::Triple {
            s: TermBox::new(inner),
            p: TermBox::new(TermValue::Iri("http://example.org/concludes".to_string())),
            o: TermBox::new(TermValue::Iri("http://example.org/o2".to_string())),
        };
        let err = ntriples_token(&outer).expect_err("nested malformed predicate must be rejected");
        assert!(
            matches!(err, Error::MalformedTerm(_)),
            "expected MalformedTerm: {err:?}"
        );
    }
}

#[cfg(test)]
mod term_walk_tests {
    //! The owned-model bridge against its recursive reference.

    use purrdf_core::{RdfTerm, RdfTriple, TermValue};

    use super::term_value_to_rdf_term;
    use crate::error::Error;

    fn reference(value: &TermValue) -> Result<RdfTerm, Error> {
        let TermValue::Triple { s, p, o } = value else {
            return term_value_to_rdf_term(value);
        };
        let TermValue::Iri(predicate) = p.as_ref() else {
            return Err(Error::MalformedTerm(
                "triple-term predicate is not an IRI".to_string(),
            ));
        };
        let subject = reference(s)?;
        let object = reference(o)?;
        Ok(RdfTerm::triple(RdfTriple {
            subject,
            predicate: predicate.clone(),
            object,
            location: None,
        }))
    }

    /// Every generated term bridges to exactly the owned term, or the refusal, the
    /// recursive reference produces.
    #[test]
    fn the_bridge_agrees_with_its_recursive_reference_on_generated_terms() {
        let mut refused = 0;
        for seed in 0..400_u64 {
            let mut state = seed;
            let mut budget = 8;
            let value = crate::test_terms::term_value(
                &mut state,
                &mut budget,
                crate::test_terms::TermShape::Any,
            );
            let found = term_value_to_rdf_term(&value);
            assert_eq!(found, reference(&value), "seed {seed}");
            refused += usize::from(found.is_err());
        }
        assert!(refused > 0, "some generated term has a non-IRI predicate");
    }
}
