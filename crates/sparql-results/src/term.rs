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
use purrdf_core::{RdfTerm, TermValue, emit_term};

/// Bridge an egress [`TermValue`] into the owned [`RdfTerm`] model, through
/// [`TermValue::to_rdf_term`]: a scoped blank node's `(label, scope)` pair is encoded
/// into the owned model's single string slot, which the kernel emitter re-targets into
/// `BLANK_NODE_LABEL` when it writes the `_:` token, so the CSV/TSV cell carries
/// exactly what the JSON/XML writers encode directly.
///
/// # Errors
///
/// RDF 1.2 requires a triple-term predicate to be an IRI. Returns
/// [`Error::MalformedTerm`] when a triple term at any nesting depth carries a
/// predicate that is not an IRI — that is malformed RDF, so it is rejected rather
/// than laundered into a fabricated IRI string.
pub(crate) fn term_value_to_rdf_term(value: &TermValue) -> Result<RdfTerm, Error> {
    value
        .to_rdf_term()
        .map_err(|_| Error::MalformedTerm("triple-term predicate is not an IRI".to_string()))
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
                datatype: "http://www.w3.org/2001/XMLSchema#string".to_string(),
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
            let value = purrdf_core::term_fixture::term_value(
                &mut state,
                purrdf_testkit::rng::splitmix64_next,
                &mut budget,
                purrdf_core::term_fixture::TermShape::Any,
            );
            let found = term_value_to_rdf_term(&value);
            assert_eq!(found, reference(&value), "seed {seed}");
            refused += usize::from(found.is_err());
        }
        assert!(refused > 0, "some generated term has a non-IRI predicate");
    }
}
