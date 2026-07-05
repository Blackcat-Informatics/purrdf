// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Conversions between the JS-facing [`Quad`]/[`Term`] objects and the engine's
//! dataset-independent value space ([`QuadValues`]/[`TermValue`]) that the COW
//! [`MutableDataset`](purrdf::ir::MutableDataset) mutates and queries by.
//!
//! `TermValue` is the engine's value→id lookup key (purrdf P4): every component is by
//! value, with literals canonicalized exactly as the interner canonicalizes them, so a
//! JS-built quad and an engine-stored quad resolve to the same term ids.

use purrdf::ir::QuadValues;
use purrdf::{BlankScope, RdfLiteral, RdfTerm, RdfTriple, TermValue};

use crate::term::{Quad, Term, TermInner, canonicalize_literal};

/// Lower an owned [`RdfTerm`] to its dataset-independent [`TermValue`]. Owned terms
/// have no blank-node scope, so blanks take the default scope (matching the engine's
/// `intern_owned_term`).
pub(crate) fn rdf_term_to_term_value(term: &RdfTerm) -> TermValue {
    match term {
        RdfTerm::Iri(iri) => TermValue::Iri(iri.clone()),
        RdfTerm::BlankNode(label) => TermValue::Blank {
            label: label.clone(),
            scope: BlankScope::DEFAULT,
        },
        RdfTerm::Literal(lit) => {
            let canonical = canonicalize_literal(lit.clone());
            TermValue::Literal {
                lexical_form: canonical.lexical_form,
                // canonicalize_literal always sets a datatype.
                datatype: canonical.datatype.unwrap_or_default(),
                language: canonical.language,
                direction: canonical.direction,
            }
        }
        RdfTerm::Triple(triple) => TermValue::Triple {
            s: Box::new(rdf_term_to_term_value(&triple.subject)),
            p: Box::new(TermValue::Iri(triple.predicate.clone())),
            o: Box::new(rdf_term_to_term_value(&triple.object)),
        },
    }
}

/// Lift a dataset-independent [`TermValue`] back to an owned [`RdfTerm`].
pub(crate) fn term_value_to_rdf_term(value: &TermValue) -> Result<RdfTerm, String> {
    Ok(match value {
        TermValue::Iri(iri) => RdfTerm::Iri(iri.clone()),
        TermValue::Blank { label, .. } => RdfTerm::BlankNode(label.clone()),
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => RdfTerm::Literal(RdfLiteral {
            lexical_form: lexical_form.clone(),
            datatype: Some(datatype.clone()),
            language: language.clone(),
            direction: *direction,
        }),
        TermValue::Triple { s, p, o } => {
            let predicate = match p.as_ref() {
                TermValue::Iri(iri) => iri.clone(),
                _ => return Err("a triple-term predicate must be an IRI".to_owned()),
            };
            RdfTerm::Triple(Box::new(RdfTriple::new(
                term_value_to_rdf_term(s)?,
                predicate,
                term_value_to_rdf_term(o)?,
            )))
        }
    })
}

/// Lower a JS [`Quad`] to the engine's [`QuadValues`] insert/query key.
pub(crate) fn quad_to_quad_values(quad: &Quad) -> Result<QuadValues, String> {
    let s = rdf_term_to_term_value(&quad.subject.to_rdf_term()?);
    let p = match &quad.predicate.inner {
        TermInner::Named(iri) => TermValue::Iri(iri.clone()),
        _ => return Err("a quad predicate must be a NamedNode".to_owned()),
    };
    let o = rdf_term_to_term_value(&quad.object.to_rdf_term()?);
    let g = match &quad.graph.inner {
        TermInner::DefaultGraph => None,
        TermInner::Named(_) | TermInner::Blank(_) => {
            Some(rdf_term_to_term_value(&quad.graph.to_rdf_term()?))
        }
        _ => return Err("a quad graph must be a NamedNode, BlankNode, or DefaultGraph".to_owned()),
    };
    Ok(QuadValues { s, p, o, g })
}

/// Lift an engine [`QuadValues`] back to a JS [`Quad`].
pub(crate) fn quad_values_to_quad(values: &QuadValues) -> Result<Quad, String> {
    let subject = Term::from_rdf_term(&term_value_to_rdf_term(&values.s)?);
    let predicate = match &values.p {
        TermValue::Iri(iri) => Term::from_inner(TermInner::Named(iri.clone())),
        _ => return Err("a quad predicate must be an IRI".to_owned()),
    };
    let object = Term::from_rdf_term(&term_value_to_rdf_term(&values.o)?);
    let graph = match &values.g {
        None => Term::from_inner(TermInner::DefaultGraph),
        Some(g) => Term::from_rdf_term(&term_value_to_rdf_term(g)?),
    };
    Ok(Quad::from_parts(subject, predicate, object, graph))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(iri: &str) -> Term {
        Term::from_inner(TermInner::Named(iri.to_owned()))
    }

    #[test]
    fn quad_round_trips_through_quad_values() {
        let q = Quad::from_parts(
            named("https://e/s"),
            named("https://e/p"),
            Term::literal(RdfLiteral::language_tagged("Hi", "EN")),
            Term::from_inner(TermInner::DefaultGraph),
        );
        let qv = quad_to_quad_values(&q).unwrap();
        // The language tag is lowercased and rdf:langString applied (engine C0.1).
        match &qv.o {
            TermValue::Literal {
                datatype, language, ..
            } => {
                assert_eq!(
                    datatype,
                    "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString"
                );
                assert_eq!(language.as_deref(), Some("en"));
            }
            other => panic!("expected a literal, got {other:?}"),
        }
        let back = quad_values_to_quad(&qv).unwrap();
        assert!(q.equals(&back));
    }

    #[test]
    fn plain_literal_canonicalizes_to_xsd_string() {
        let qv = quad_to_quad_values(&Quad::from_parts(
            named("https://e/s"),
            named("https://e/p"),
            Term::literal(RdfLiteral::simple("plain")),
            Term::from_inner(TermInner::DefaultGraph),
        ))
        .unwrap();
        match &qv.o {
            TermValue::Literal { datatype, .. } => {
                assert_eq!(datatype, "http://www.w3.org/2001/XMLSchema#string");
            }
            other => panic!("expected a literal, got {other:?}"),
        }
    }

    #[test]
    fn non_named_predicate_is_rejected() {
        let q = Quad::from_parts(
            named("https://e/s"),
            Term::from_inner(TermInner::Blank("p".to_owned())),
            named("https://e/o"),
            Term::from_inner(TermInner::DefaultGraph),
        );
        assert!(quad_to_quad_values(&q).is_err());
    }
}
