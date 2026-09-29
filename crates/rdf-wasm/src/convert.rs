// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Conversions between the JS-facing [`Quad`]/[`Term`] objects and the engine's
//! dataset-independent value space ([`QuadValues`]/[`TermValue`]) that the COW
//! [`MutableDataset`](purrdf::ir::MutableDataset) mutates and queries by.
//!
//! `TermValue` is the engine's value→id lookup key: every component is by
//! value, with literals canonicalized exactly as the interner canonicalizes them, so a
//! JS-built quad and an engine-stored quad resolve to the same term ids.

use purrdf::ir::QuadValues;
use purrdf::{RdfTerm, TermValue};

use crate::term::{Quad, Term, TermInner};

/// Lift a dataset-independent [`TermValue`] into an owned [`RdfTerm`] by moving
/// every string and nested value: [`TermValue::into_rdf_term`]. Query-result egress
/// uses this path so one cell is not cloned while crossing the wasm boundary, and a
/// scoped blank node crosses as its deterministic scope envelope, so two blank nodes
/// that share a label in different scopes stay two nodes in JS.
///
/// Out of line, so the per-cell marshalling is one compiled function rather than a
/// fragment of the SELECT row loop.
#[inline(never)]
pub(crate) fn term_value_into_rdf_term(value: TermValue) -> Result<RdfTerm, String> {
    value.into_rdf_term().map_err(|error| error.to_string())
}

/// Lower a JS [`Quad`] to the engine's [`QuadValues`] insert/query key.
pub(crate) fn quad_to_quad_values(quad: &Quad) -> Result<QuadValues, String> {
    let s = TermValue::from_rdf_term(&quad.subject.to_rdf_term()?);
    let p = match &quad.predicate.inner {
        TermInner::Named(iri) => TermValue::Iri(iri.clone()),
        _ => return Err("a quad predicate must be a NamedNode".to_owned()),
    };
    let o = TermValue::from_rdf_term(&quad.object.to_rdf_term()?);
    let g = match &quad.graph.inner {
        TermInner::DefaultGraph => None,
        TermInner::Named(_) | TermInner::Blank(_) => {
            Some(TermValue::from_rdf_term(&quad.graph.to_rdf_term()?))
        }
        _ => return Err("a quad graph must be a NamedNode, BlankNode, or DefaultGraph".to_owned()),
    };
    Ok(QuadValues { s, p, o, g })
}

/// Lift an engine [`QuadValues`] back to a JS [`Quad`].
pub(crate) fn quad_values_to_quad(values: &QuadValues) -> Result<Quad, String> {
    let subject = Term::from_rdf_term(&values.s.to_rdf_term().map_err(|e| e.to_string())?);
    let predicate = match &values.p {
        TermValue::Iri(iri) => Term::from_inner(TermInner::Named(iri.clone())),
        _ => return Err("a quad predicate must be an IRI".to_owned()),
    };
    let object = Term::from_rdf_term(&values.o.to_rdf_term().map_err(|e| e.to_string())?);
    let graph = match &values.g {
        None => Term::from_inner(TermInner::DefaultGraph),
        Some(g) => Term::from_rdf_term(&g.to_rdf_term().map_err(|e| e.to_string())?),
    };
    Ok(Quad::from_parts(subject, predicate, object, graph))
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf::{BlankScope, RdfLiteral, TermBox};

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

    #[test]
    fn owned_term_value_conversion_matches_borrowed_conversion() {
        let value = TermValue::Triple {
            s: TermBox::new(TermValue::Iri("https://e/s".to_owned())),
            p: TermBox::new(TermValue::Iri("https://e/p".to_owned())),
            o: TermBox::new(TermValue::Literal {
                lexical_form: "value".to_owned(),
                datatype: "https://e/datatype".to_owned(),
                language: None,
                direction: None,
            }),
        };
        assert_eq!(
            term_value_into_rdf_term(value.clone()).expect("owned conversion"),
            value.to_rdf_term().expect("borrowed conversion")
        );
    }

    fn scoped_blank(label: &str, scope: BlankScope) -> TermValue {
        TermValue::Blank {
            label: label.to_owned(),
            scope,
        }
    }

    /// A quad whose subject and object are two blank nodes: the values as the engine
    /// holds them, and the JS quad they cross the boundary as.
    fn blank_pair_quad(subject: TermValue, object: TermValue) -> (QuadValues, Quad) {
        let values = QuadValues {
            s: subject,
            p: TermValue::iri("https://e/p"),
            o: object,
            g: None,
        };
        let quad = quad_values_to_quad(&values).expect("blank nodes cross the boundary");
        (values, quad)
    }

    #[test]
    fn two_scoped_blanks_sharing_a_label_stay_distinct_in_js() {
        let (values, quad) = blank_pair_quad(
            scoped_blank("b", BlankScope(1)),
            scoped_blank("b", BlankScope(2)),
        );
        assert_eq!(quad.subject().term_type(), "BlankNode");
        assert_eq!(quad.object().term_type(), "BlankNode");
        assert_ne!(
            quad.subject().value(),
            quad.object().value(),
            "two scopes collapsed into one JS blank node"
        );
        assert!(!quad.subject().equals(&quad.object()));
        // The label is deterministic: the same pair crosses as the same label.
        let (_, again) = blank_pair_quad(
            scoped_blank("b", BlankScope(1)),
            scoped_blank("b", BlankScope(2)),
        );
        assert_eq!(quad.subject().value(), again.subject().value());
        assert_eq!(quad.object().value(), again.object().value());
        // And the round trip restores both `(label, scope)` pairs.
        assert_eq!(quad_to_quad_values(&quad).expect("the quad lowers"), values);
    }

    #[test]
    fn a_scoped_blank_is_distinct_from_the_unscoped_blank_of_its_label() {
        let (values, quad) = blank_pair_quad(
            scoped_blank("b", BlankScope::DEFAULT),
            scoped_blank("b", BlankScope(1)),
        );
        assert_ne!(quad.subject().value(), quad.object().value());
        assert_eq!(quad_to_quad_values(&quad).expect("the quad lowers"), values);
    }

    #[test]
    fn an_unscoped_blank_keeps_its_label_in_js() {
        let (values, quad) = blank_pair_quad(
            scoped_blank("b0", BlankScope::DEFAULT),
            scoped_blank("a.b", BlankScope::DEFAULT),
        );
        assert_eq!(quad.subject().value(), "b0");
        assert_eq!(quad.object().value(), "a.b");
        assert_eq!(quad_to_quad_values(&quad).expect("the quad lowers"), values);
    }
}
