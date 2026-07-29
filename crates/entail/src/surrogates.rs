// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! What `rdfD1` and `rdfs14` OBSERVE, walked once into internal relations the clause
//! language can join.
//!
//! Both rules quantify over a property of a TERM rather than over a triple pattern:
//! `rdfD1` fires on "a triple in which a datatyped literal `"sss"^^ddd` appears, for a
//! recognized `ddd`", and `rdfs14` on "a triple in which a triple term appears". A
//! [`ClauseAtom`](purrdf_datalog::clause::ClauseAtom) is four terms with no term-KIND test
//! — deliberately, because the store interns a term by its lexical surface and a rule that
//! could ask "is this a literal?" would be asking about the surface's spelling — so neither
//! premise is expressible as a clause and both are answered here, by one pass over the
//! dataset's default graph.
//!
//! That is the same discipline [`crate::lists`] applies to `LIST[…]` and
//! [`crate::datatypes`] to the XSD value spaces: a premise the clause language cannot
//! state is MATERIALIZED as a positive relation, never approximated and never dropped.
//!
//! # Which datatypes count
//!
//! [`RECOGNIZED_DATATYPES`] is RDF 1.2 Semantics §8's mandatory `D` — `rdf:langString`,
//! `rdf:dirLangString`, `xsd:string`. It is not a default this crate invented: it is the
//! `D` the unqualified phrase "RDF entails" is defined against, and `rdfs1` in
//! [`crate::calculus::rdfs`] quantifies over exactly the same three. A wider `D` is what
//! [`Construct::DatatypeValueSpace`](crate::Construct::DatatypeValueSpace) reports.
//!
//! # Determinism
//!
//! Observations are keyed by lexical surface in a `BTreeMap`/`BTreeSet`, and
//! [`SurrogateIndex::materialize`] emits them in that order. Two runs over one dataset
//! produce byte-identical facts.

use std::collections::{BTreeMap, BTreeSet};

use purrdf_core::TermValue;

use crate::lists::{DATATYPED_RELATION, INTERNAL_GRAPH, InternalFact, QUOTED_RELATION};
use crate::vocab::{RDF_DIRLANGSTRING, RDF_LANGSTRING, RDFS_PROPOSITION, XSD_STRING};

/// The datatype IRIs every RDF interpretation MUST recognize (RDF 1.2 Semantics §8).
pub(crate) const RECOGNIZED_DATATYPES: [&str; 3] = [RDF_LANGSTRING, RDF_DIRLANGSTRING, XSD_STRING];

/// The bracketed surface of a constant IRI, as the store interns it.
fn iri_surface(iri: &str) -> String {
    format!("<{iri}>")
}

/// The terms of one run that `rdfD1` and `rdfs14` fire on.
#[derive(Debug, Default)]
pub(crate) struct SurrogateIndex {
    /// Literal surface → its recognized datatype IRI, in surface order.
    datatyped: BTreeMap<String, String>,
    /// Triple-term surfaces, in surface order.
    quoted: BTreeSet<String>,
}

impl SurrogateIndex {
    /// Record `value`, held in the store under `surface`, if either rule observes it.
    ///
    /// A literal whose datatype is not one of [`RECOGNIZED_DATATYPES`] is NOT recorded:
    /// `rdfD1` is stated over a recognized `ddd`, and an interpretation that does not
    /// recognize a datatype says nothing about the literals carrying it. That is the same
    /// judgement `rdfs1` makes, and the two must agree or the closure would type a
    /// surrogate with a datatype the calculus never declared.
    pub(crate) fn observe(&mut self, surface: &str, value: &TermValue) {
        match value {
            TermValue::Literal { datatype, .. } => {
                if RECOGNIZED_DATATYPES.contains(&datatype.as_str()) {
                    self.datatyped
                        .entry(surface.to_owned())
                        .or_insert_with(|| datatype.clone());
                }
            }
            TermValue::Triple { .. } => {
                self.quoted.insert(surface.to_owned());
            }
            TermValue::Iri(_) | TermValue::Blank { .. } => {}
        }
    }

    /// Every constant IRI these observations put in the fact store, so
    /// [`crate::engine`]'s surface dictionary can read one back.
    ///
    /// The datatypes come from the DATA, and `rdfD1`'s clause carries them in a variable,
    /// so they are not program constants the clause walk would already have recorded — and
    /// `rdf:type ddd` puts one in an OBJECT position of the closure. `rdfs:Proposition` is
    /// a program constant of `rdfs14a` but not of the bare-`RDF` lane, so it is named here
    /// too rather than assumed.
    pub(crate) fn iris(&self) -> impl Iterator<Item = &str> {
        self.datatyped
            .values()
            .map(String::as_str)
            .chain(std::iter::once(RDFS_PROPOSITION))
    }

    /// The observations, as internal facts.
    pub(crate) fn materialize(&self) -> Vec<InternalFact> {
        let mut facts = Vec::new();
        for (surface, datatype) in &self.datatyped {
            facts.push(InternalFact {
                subject: surface.clone(),
                predicate: DATATYPED_RELATION,
                object: iri_surface(datatype),
                graph: INTERNAL_GRAPH.to_owned(),
            });
        }
        for surface in &self.quoted {
            facts.push(InternalFact {
                subject: surface.clone(),
                predicate: QUOTED_RELATION,
                object: iri_surface(RDFS_PROPOSITION),
                graph: INTERNAL_GRAPH.to_owned(),
            });
        }
        facts
    }
}

#[cfg(test)]
mod tests {
    use super::{RECOGNIZED_DATATYPES, SurrogateIndex, iri_surface};
    use crate::lists::{DATATYPED_RELATION, QUOTED_RELATION};
    use crate::vocab::{RDFS_PROPOSITION, XSD_STRING};
    use purrdf_core::TermValue;

    /// A fixture IRI. PurRDF mints no vocabulary, so every fixture term is `example.org`.
    const EX_S: &str = "http://example.org/s";
    /// A fixture predicate IRI.
    const EX_P: &str = "http://example.org/p";
    /// A fixture object IRI.
    const EX_O: &str = "http://example.org/o";

    /// A plain literal carries `xsd:string` (RDF 1.1 C0.1), which IS recognized, so it is
    /// observed; a literal of an unrecognized datatype is not judged either way.
    #[test]
    fn only_a_recognized_datatype_is_observed() {
        let mut index = SurrogateIndex::default();
        index.observe("\"cat\"", &TermValue::simple_literal("cat"));
        index.observe(
            "\"42\"^^<http://example.org/dt>",
            &TermValue::typed_literal("42", "http://example.org/dt"),
        );
        index.observe("<s>", &TermValue::iri(EX_S));
        index.observe("_:0.b", &TermValue::blank("b"));
        let facts = index.materialize();
        assert_eq!(facts.len(), 1, "{facts:?}");
        assert_eq!(facts[0].predicate, DATATYPED_RELATION);
        assert_eq!(facts[0].object, iri_surface(XSD_STRING));
        assert!(RECOGNIZED_DATATYPES.contains(&XSD_STRING));
    }

    /// A triple term is observed once, and paired with the class `rdfs14` types its
    /// surrogate with.
    #[test]
    fn a_triple_term_is_observed_and_paired_with_rdfs_proposition() {
        let mut index = SurrogateIndex::default();
        let quoted = TermValue::Triple {
            s: Box::new(TermValue::iri(EX_S)),
            p: Box::new(TermValue::iri(EX_P)),
            o: Box::new(TermValue::iri(EX_O)),
        };
        index.observe("<<( <s> <p> <o> )>>", &quoted);
        index.observe("<<( <s> <p> <o> )>>", &quoted);
        let facts = index.materialize();
        assert_eq!(facts.len(), 1, "one term, one observation: {facts:?}");
        assert_eq!(facts[0].predicate, QUOTED_RELATION);
        assert_eq!(facts[0].object, iri_surface(RDFS_PROPOSITION));
    }

    /// Every IRI the pre-pass puts in the store is named for the surface dictionary.
    #[test]
    fn the_named_iris_cover_every_object_the_pre_pass_emits() {
        let mut index = SurrogateIndex::default();
        index.observe("\"cat\"", &TermValue::simple_literal("cat"));
        index.observe("\"chat\"@fr", &TermValue::lang_literal("chat", "fr"));
        let named: Vec<String> = index.iris().map(iri_surface).collect();
        for fact in index.materialize() {
            assert!(named.contains(&fact.object), "{:?} unnamed", fact.object);
        }
    }
}
