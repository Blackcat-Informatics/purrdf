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
use purrdf::{BlankScope, RdfTerm, TermBox, TermValue};

use crate::term::{Quad, Term, TermInner};

/// How a blank node's scope crosses to JS: a declared option of the conversion, set on
/// the engine as `QueryEngine#blankScope` (`"keep"` or `"merge"`).
///
/// The engine holds a blank node as a `(label, scope)` pair, so two nodes that share a
/// label in different scopes are two nodes. JS has one string for a blank node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum BlankScopeMode {
    /// The default. A scoped blank crosses as its deterministic scope envelope, so two
    /// blank nodes that share a label in different scopes stay two nodes in JS, and an
    /// unscoped blank keeps its bare label.
    #[default]
    Keep,
    /// The scope is dropped: a blank crosses as its bare label, so two blank nodes that
    /// share a label in different scopes are ONE node in JS. Lossy by declaration, for a
    /// caller that wants the labels the data was written with.
    Merge,
}

impl BlankScopeMode {
    /// The option's spelling for `"keep"`.
    pub(crate) const KEEP: &'static str = "keep";
    /// The option's spelling for `"merge"`.
    pub(crate) const MERGE: &'static str = "merge";

    /// The mode a JS option value names, or the refusal that lists the accepted values.
    pub(crate) fn parse(name: &str) -> Result<Self, String> {
        match name {
            Self::KEEP => Ok(Self::Keep),
            Self::MERGE => Ok(Self::Merge),
            other => Err(format!(
                "unknown blankScope {other:?} (expected \"{}\" or \"{}\")",
                Self::KEEP,
                Self::MERGE
            )),
        }
    }

    /// The option's spelling of this mode.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Keep => Self::KEEP,
            Self::Merge => Self::MERGE,
        }
    }
}

/// Lift a dataset-independent [`TermValue`] into an owned [`RdfTerm`] by moving
/// every string and nested value: [`TermValue::into_rdf_term`]. Query-result egress
/// uses this path so one cell is not cloned while crossing the wasm boundary. Under
/// [`BlankScopeMode::Keep`] a scoped blank node crosses as its deterministic scope
/// envelope, so two blank nodes that share a label in different scopes stay two nodes
/// in JS; under [`BlankScopeMode::Merge`] every blank crosses as its bare label.
///
/// Out of line, so the per-cell marshalling is one compiled function rather than a
/// fragment of the SELECT row loop.
#[inline(never)]
pub(crate) fn term_value_into_rdf_term(
    value: TermValue,
    mode: BlankScopeMode,
) -> Result<RdfTerm, String> {
    let value = match mode {
        BlankScopeMode::Keep => value,
        BlankScopeMode::Merge => merge_blank_scopes(value),
    };
    value.into_rdf_term().map_err(|error| error.to_string())
}

/// `value` with every blank node, at any depth of a triple term, moved to the default
/// scope. Folded over the term's own work list, so a deep triple term costs no stack.
fn merge_blank_scopes(value: TermValue) -> TermValue {
    match value.try_fold_owned::<TermValue, std::convert::Infallible>(
        |leaf| {
            Ok(match leaf {
                TermValue::Blank { label, .. } => TermValue::Blank {
                    label,
                    scope: BlankScope::DEFAULT,
                },
                other => other,
            })
        },
        |s, p, o| {
            Ok(TermValue::Triple {
                s: TermBox::new(s),
                p: TermBox::new(p),
                o: TermBox::new(o),
            })
        },
    ) {
        Ok(merged) => merged,
    }
}

/// Lower a JS [`Quad`] to the engine's [`QuadValues`] insert/query key.
pub(crate) fn quad_to_quad_values(quad: &Quad) -> Result<QuadValues, String> {
    let s = quad.subject.to_value()?;
    let p = match &quad.predicate.inner {
        TermInner::Named(iri) => TermValue::Iri(iri.clone()),
        _ => return Err("a quad predicate must be a NamedNode".to_owned()),
    };
    let o = quad.object.to_value()?;
    let g = match &quad.graph.inner {
        TermInner::DefaultGraph => None,
        TermInner::Named(_) | TermInner::Blank(_) => Some(quad.graph.to_value()?),
        _ => return Err("a quad graph must be a NamedNode, BlankNode, or DefaultGraph".to_owned()),
    };
    Ok(QuadValues { s, p, o, g })
}

/// Lift an engine [`QuadValues`] back to a JS [`Quad`].
pub(crate) fn quad_values_to_quad(values: &QuadValues) -> Result<Quad, String> {
    let subject = Term::from_value(values.s.clone(), BlankScopeMode::Keep)?;
    let predicate = match &values.p {
        TermValue::Iri(iri) => Term::from_inner(TermInner::Named(iri.clone())),
        _ => return Err("a quad predicate must be an IRI".to_owned()),
    };
    let object = Term::from_value(values.o.clone(), BlankScopeMode::Keep)?;
    let graph = match &values.g {
        None => Term::from_inner(TermInner::DefaultGraph),
        Some(g) => Term::from_value(g.clone(), BlankScopeMode::Keep)?,
    };
    Ok(Quad::from_parts(subject, predicate, object, graph))
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf::RdfLiteral;

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
            term_value_into_rdf_term(value.clone(), BlankScopeMode::Keep)
                .expect("owned conversion"),
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

    #[test]
    fn the_blank_scope_option_names_exactly_keep_and_merge() {
        assert_eq!(BlankScopeMode::default(), BlankScopeMode::Keep);
        assert_eq!(BlankScopeMode::parse("keep"), Ok(BlankScopeMode::Keep));
        assert_eq!(BlankScopeMode::parse("merge"), Ok(BlankScopeMode::Merge));
        for mode in [BlankScopeMode::Keep, BlankScopeMode::Merge] {
            assert_eq!(BlankScopeMode::parse(mode.name()), Ok(mode));
        }
        let refused = BlankScopeMode::parse("Merge").expect_err("the names are case-sensitive");
        assert!(refused.contains("\"keep\" or \"merge\""), "{refused}");
    }

    #[test]
    fn merge_drops_scope_where_keep_holds_two_nodes_on_the_rust_side() {
        let first = scoped_blank("b", BlankScope(1));
        let second = scoped_blank("b", BlankScope(2));
        // The Rust side: two nodes, and under `keep` two distinct JS labels.
        assert_ne!(first, second);
        let keep = |v: &TermValue| {
            term_value_into_rdf_term(v.clone(), BlankScopeMode::Keep).expect("converts")
        };
        assert_ne!(keep(&first), keep(&second));
        // The declared lossy side: both cross as the bare label, and are one JS node.
        let merge = |v: &TermValue| {
            term_value_into_rdf_term(v.clone(), BlankScopeMode::Merge).expect("converts")
        };
        assert_eq!(merge(&first), RdfTerm::BlankNode("b".to_owned()));
        assert_eq!(merge(&first), merge(&second));
        // An unscoped blank is unchanged by either mode.
        let plain = scoped_blank("b", BlankScope::DEFAULT);
        assert_eq!(keep(&plain), merge(&plain));
        // A neighbour that is not a blank is unchanged too.
        let iri = TermValue::iri("https://e/s");
        assert_eq!(keep(&iri), merge(&iri));
    }

    #[test]
    fn merge_reaches_a_blank_inside_a_triple_term() {
        let triple = |object: TermValue| TermValue::Triple {
            s: TermBox::new(TermValue::iri("https://e/s")),
            p: TermBox::new(TermValue::iri("https://e/p")),
            o: TermBox::new(object),
        };
        let one = triple(scoped_blank("b", BlankScope(1)));
        let two = triple(scoped_blank("b", BlankScope(2)));
        let convert = |v: &TermValue, mode| term_value_into_rdf_term(v.clone(), mode).expect("ok");
        assert_ne!(
            convert(&one, BlankScopeMode::Keep),
            convert(&two, BlankScopeMode::Keep)
        );
        assert_eq!(
            convert(&one, BlankScopeMode::Merge),
            convert(&two, BlankScopeMode::Merge)
        );
    }
}
