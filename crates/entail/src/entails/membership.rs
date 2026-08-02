// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The SIDE CONDITION every OWL schema conclusion carries, and how it is established.
//!
//! # Why this is a module and not a line in each mechanism
//!
//! An OWL schema statement is almost never the one thing it looks like. `p rdf:type
//! owl:TransitiveProperty` is `p ∈ IOOP` **and** the transitivity implication.
//! `c rdfs:subClassOf d` is `c,d ∈ IC` **and** the inclusion. The RDF-Based comprehension
//! condition that licenses an anonymous `owl:unionOf` class licenses it only for operands
//! that are already in `IC`. In every case the second conjunct is the one a reader forgets is
//! there, and a mechanism that established only the interesting half would be claiming
//! conclusions the semantics does not license — W3C's own corpus publishes non-entailments of
//! exactly that shape.
//!
//! So the membership half lives in one place, is spelled out once per semantic set, and is
//! ESTABLISHED rather than assumed: [`Membership::establish`] looks for a typing in the
//! premise's own closure, which is the [`homomorphism`](super::homomorphism) mechanism's test
//! applied to a ground triple. It is an entailment check, not a syntactic look at the
//! premise's bytes: `x rdf:type owl:Class` reaches it whether the premise asserted it or
//! `scm-cls` derived it.
//!
//! # Which typings establish which set, and why those
//!
//! Each list below is exactly the typings whose class extension IS the set, or is contained in
//! it by a semantic condition of OWL 2's RDF-Based Semantics. A typing that merely tends to
//! accompany the set is not here: over-accepting would license a conclusion the semantics does
//! not, which is the one error direction that produces a wrong `Entailed`.
//!
//! The lists are ordered, and the order is the tie-break: [`Membership::establish`] returns
//! the FIRST typing it finds, so a term the closure types two ways yields one reproducible
//! citation rather than whichever the iteration happened to reach.

use purrdf_core::TermValue;

use crate::entails::graph::Triple;
use crate::entails::homomorphism::Closure;
use crate::vocab::{
    OWL_CLASS, OWL_DATATYPEPROPERTY, OWL_OBJECTPROPERTY, RDF_PROPERTY, RDF_TYPE, RDFS_CLASS,
};

/// Which semantic set a named term of a schema statement must belong to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Membership {
    /// `∈ IC`, the classes. `ICEXT(rdfs:Class) = IC` is RDF Semantics' own definition, and
    /// OWL 2's RDF-Based Semantics states `ICEXT(owl:Class) = IC`, so either typing
    /// establishes it.
    Class,
    /// `∈ IP`, the properties. `ICEXT(rdf:Property) = IP`, and OWL 2's RDF-Based Semantics
    /// places both `IOOP` (object properties) and `IODP` (data properties) inside `IP`.
    Property,
    /// `∈ IOOP`, the object properties. Only an `owl:ObjectProperty` typing establishes this
    /// one: `IOOP` is a PROPER part of `IP` in general, so a bare `rdf:Property` typing does
    /// not reach it.
    ObjectProperty,
}

impl Membership {
    /// The typings that establish this membership, in the order they are tried.
    pub(crate) const fn typings(self) -> &'static [&'static str] {
        match self {
            Self::Class => &[OWL_CLASS, RDFS_CLASS],
            Self::Property => &[RDF_PROPERTY, OWL_OBJECTPROPERTY, OWL_DATATYPEPROPERTY],
            Self::ObjectProperty => &[OWL_OBJECTPROPERTY],
        }
    }

    /// The closure triple that establishes `term ∈ self`, or `None` if none does.
    ///
    /// The triple is returned rather than a boolean because it is EVIDENCE: a warrant cites
    /// it, and [`verify`](super::verify) re-looks it up without running a reasoner.
    pub(crate) fn establish(self, closure: &Closure, term: &TermValue) -> Option<Triple> {
        self.typings().iter().find_map(|class| {
            let triple = [
                term.clone(),
                TermValue::iri(RDF_TYPE),
                TermValue::iri(*class),
            ];
            closure.contains(&triple).then_some(triple)
        })
    }
}

#[cfg(test)]
mod tests {
    use purrdf_core::TermValue;

    use super::Membership;
    use crate::entails::homomorphism::Closure;
    use crate::vocab::{OWL_CLASS, OWL_OBJECTPROPERTY, RDF_PROPERTY, RDF_TYPE, RDFS_CLASS};

    const A: &str = "http://example.org/A";

    fn closure_of(typings: &[&str]) -> Closure {
        Closure::of(
            typings
                .iter()
                .map(|class| {
                    [
                        TermValue::iri(A),
                        TermValue::iri(RDF_TYPE),
                        TermValue::iri(*class),
                    ]
                })
                .collect(),
        )
    }

    /// Every accepted typing establishes its set, and the cited triple is the one found.
    #[test]
    fn each_accepted_typing_establishes_its_set() {
        for membership in [
            Membership::Class,
            Membership::Property,
            Membership::ObjectProperty,
        ] {
            for typing in membership.typings() {
                let closure = closure_of(&[typing]);
                let found = membership
                    .establish(&closure, &TermValue::iri(A))
                    .unwrap_or_else(|| panic!("{typing} establishes {membership:?}"));
                assert_eq!(found[2], TermValue::iri(*typing));
            }
        }
    }

    /// `IOOP` IS NOT `IP`. A bare `rdf:Property` typing does not establish an object
    /// property, which is the distinction the three-way split exists for.
    #[test]
    fn a_bare_property_typing_does_not_establish_an_object_property() {
        let closure = closure_of(&[RDF_PROPERTY]);
        assert!(
            Membership::ObjectProperty
                .establish(&closure, &TermValue::iri(A))
                .is_none()
        );
        assert!(
            Membership::Property
                .establish(&closure, &TermValue::iri(A))
                .is_some()
        );
    }

    /// A term nothing types establishes nothing, and a class typing is not a property one.
    #[test]
    fn an_untyped_or_wrongly_typed_term_establishes_nothing() {
        let empty = Closure::of(Vec::new());
        assert!(
            Membership::Class
                .establish(&empty, &TermValue::iri(A))
                .is_none()
        );
        let classes = closure_of(&[OWL_CLASS, RDFS_CLASS]);
        assert!(
            Membership::Property
                .establish(&classes, &TermValue::iri(A))
                .is_none()
        );
    }

    /// The citation is REPRODUCIBLE: a term typed two ways yields the first accepted typing,
    /// not whichever the iteration reached.
    #[test]
    fn a_doubly_typed_term_cites_the_first_accepted_typing() {
        let closure = closure_of(&[OWL_OBJECTPROPERTY, RDF_PROPERTY]);
        let found = Membership::Property
            .establish(&closure, &TermValue::iri(A))
            .expect("both typings are in the closure");
        assert_eq!(found[2], TermValue::iri(RDF_PROPERTY));
    }
}
