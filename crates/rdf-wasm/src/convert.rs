// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Conversions between the JS-facing [`Quad`]/[`Term`] objects and the engine's
//! dataset-independent value space ([`QuadValues`]/[`TermValue`]) that the COW
//! [`MutableDataset`](purrdf::ir::MutableDataset) mutates and queries by.
//!
//! `TermValue` is the engine's value→id lookup key: every component is by
//! value, with literals canonicalized exactly as the interner canonicalizes them, so a
//! JS-built quad and an engine-stored quad resolve to the same term ids.

use std::convert::Infallible;

use purrdf::ir::QuadValues;
use purrdf::{BlankScope, RdfLiteral, RdfTerm, RdfTriple, TermBox, TermValue};

use crate::term::{Quad, Term, TermInner, canonicalize_literal};

/// Lower an owned [`RdfTerm`] to its dataset-independent [`TermValue`]. Owned terms
/// have no blank-node scope, so blanks take the default scope (matching the engine's
/// `intern_owned_term`).
///
/// A triple term is lowered bottom-up over [`RdfTerm::try_fold`]'s work list.
pub(crate) fn rdf_term_to_term_value(term: &RdfTerm) -> TermValue {
    let lowered = term.try_fold(
        |leaf| {
            Ok::<_, Infallible>(match leaf {
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
                RdfTerm::Triple(_) => unreachable!("a triple term is folded from its parts"),
            })
        },
        |predicate| Ok(TermValue::Iri(predicate.to_owned())),
        |s, p, o| {
            Ok(TermValue::Triple {
                s: TermBox::new(s),
                p: TermBox::new(p),
                o: TermBox::new(o),
            })
        },
    );
    match lowered {
        Ok(value) => value,
    }
}

/// The owned [`RdfTerm`] of a term value that is not a triple term.
fn leaf_rdf_term(value: TermValue) -> RdfTerm {
    match value {
        TermValue::Iri(iri) => RdfTerm::Iri(iri),
        TermValue::Blank { label, .. } => RdfTerm::BlankNode(label),
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => RdfTerm::Literal(RdfLiteral {
            lexical_form,
            datatype: Some(datatype),
            language,
            direction,
        }),
        TermValue::Triple { .. } => unreachable!("a triple term is folded from its parts"),
    }
}

/// Assemble an owned triple term, refusing a predicate that is not an IRI.
fn triple_rdf_term(
    subject: RdfTerm,
    predicate: RdfTerm,
    object: RdfTerm,
) -> Result<RdfTerm, String> {
    let RdfTerm::Iri(predicate) = predicate else {
        return Err("a triple-term predicate must be an IRI".to_owned());
    };
    Ok(RdfTerm::Triple(Box::new(RdfTriple::new(
        subject, predicate, object,
    ))))
}

/// Lift a dataset-independent [`TermValue`] back to an owned [`RdfTerm`].
///
/// A triple term is lifted bottom-up over [`TermValue::try_fold`]'s work list.
pub(crate) fn term_value_to_rdf_term(value: &TermValue) -> Result<RdfTerm, String> {
    value.try_fold(|leaf| Ok(leaf_rdf_term(leaf.clone())), triple_rdf_term)
}

/// Lift a dataset-independent [`TermValue`] into an owned [`RdfTerm`] by moving
/// every string and nested value. Query-result egress uses this path so one cell
/// is not cloned while crossing the wasm boundary.
///
/// A triple term is lifted bottom-up over [`TermValue::try_fold_owned`]'s work list.
///
/// Out of line, so the per-cell marshalling is one compiled function rather than a
/// fragment of the SELECT row loop.
#[inline(never)]
pub(crate) fn term_value_into_rdf_term(value: TermValue) -> Result<RdfTerm, String> {
    value.try_fold_owned(|leaf| Ok(leaf_rdf_term(leaf)), triple_rdf_term)
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
            term_value_to_rdf_term(&value).expect("borrowed conversion")
        );
    }
}

#[cfg(test)]
mod term_walk_tests {
    //! The owned-model conversions against their recursive references, and the lowering
    //! at a hundred thousand levels on a 128 KiB thread.

    use purrdf::{BlankScope, RdfLiteral, RdfTerm, RdfTriple, TermBox, TermValue};

    use super::{rdf_term_to_term_value, term_value_into_rdf_term, term_value_to_rdf_term};
    use crate::term::canonicalize_literal;

    fn reference_lower(term: &RdfTerm) -> TermValue {
        match term {
            RdfTerm::Triple(triple) => TermValue::Triple {
                s: TermBox::new(reference_lower(&triple.subject)),
                p: TermBox::new(TermValue::Iri(triple.predicate.clone())),
                o: TermBox::new(reference_lower(&triple.object)),
            },
            RdfTerm::BlankNode(label) => TermValue::Blank {
                label: label.clone(),
                scope: BlankScope::DEFAULT,
            },
            RdfTerm::Literal(lit) => {
                let canonical = canonicalize_literal(lit.clone());
                TermValue::Literal {
                    lexical_form: canonical.lexical_form,
                    datatype: canonical.datatype.unwrap_or_default(),
                    language: canonical.language,
                    direction: canonical.direction,
                }
            }
            RdfTerm::Iri(iri) => TermValue::Iri(iri.clone()),
        }
    }

    fn reference_lift(value: &TermValue) -> Result<RdfTerm, String> {
        Ok(match value {
            TermValue::Triple { s, p, o } => {
                let TermValue::Iri(predicate) = p.as_ref() else {
                    return Err("a triple-term predicate must be an IRI".to_owned());
                };
                RdfTerm::Triple(Box::new(RdfTriple::new(
                    reference_lift(s)?,
                    predicate.clone(),
                    reference_lift(o)?,
                )))
            }
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
        })
    }

    /// Every generated term lowers and lifts — by reference and by value — exactly as the
    /// recursive references do, a non-IRI predicate's refusal included.
    #[test]
    fn the_conversions_agree_with_their_recursive_references_on_generated_terms() {
        let mut refused = 0;
        for seed in 0..400_u64 {
            let mut state = seed;
            let mut budget = 8;
            let value = purrdf_core::test_rng::term_value(
                &mut state,
                &mut budget,
                purrdf_core::test_rng::TermShape::Any,
            );
            let lifted = term_value_to_rdf_term(&value);
            assert_eq!(lifted, reference_lift(&value), "seed {seed}");
            assert_eq!(term_value_into_rdf_term(value), lifted, "seed {seed}");
            match lifted {
                Ok(term) => assert_eq!(
                    rdf_term_to_term_value(&term),
                    reference_lower(&term),
                    "seed {seed}"
                ),
                Err(_) => refused += 1,
            }
        }
        assert!(refused > 0, "some generated term has a non-IRI predicate");
    }

    /// An owned triple term a hundred thousand levels deep is lowered on a thread whose
    /// whole stack is 128 KiB.
    #[test]
    fn a_hundred_thousand_level_term_is_lowered_on_a_128_kib_thread() {
        const LEVELS: usize = 100_000;
        std::thread::Builder::new()
            .stack_size(128 * 1024)
            .spawn(|| {
                let mut term = RdfTerm::iri("http://example.org/o");
                for _ in 0..LEVELS {
                    term = RdfTerm::triple(RdfTriple::new(
                        RdfTerm::iri("http://example.org/s"),
                        "http://example.org/p",
                        term,
                    ));
                }
                assert_eq!(
                    rdf_term_to_term_value(&term),
                    purrdf_core::test_rng::triple_chain(LEVELS)
                );
                // The owned model's derived drop descends once per level, so the chain
                // is taken apart one level at a time.
                while let RdfTerm::Triple(triple) = term {
                    term = triple.object;
                }
            })
            .expect("the thread starts")
            .join()
            .expect("the lowering did not overflow the thread's stack");
    }
}
