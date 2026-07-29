// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `scm-*` — the semantics of the schema vocabulary, OWL 2 Profiles §4.3 Table 9.
//!
//! Eighteen of the table's twenty rules live here. `scm-sco` and `scm-spo` do not: they are
//! the OWL 2 RL names for `rdfs11` and `rdfs5`, and one clause with two names is stated
//! once, in [`super::rdfs`], where the RDFS numbering orders it;
//! [`super::ChaseRule::rule_id`] answers with the OWL name under the `OWL-RL` lane.
//!
//! # A conjunctive conclusion is stated as one clause per conjunct
//!
//! Five rules here — `scm-cls`, `scm-eqc1`, `scm-op`, `scm-dp` and `scm-eqp1` — conclude a
//! CONJUNCTION of triples. A conjunctive head is a representable DL clause
//! ([`HeadForm::Conjunctive`](purrdf_datalog::clause::HeadForm::Conjunctive)) and is not a
//! Datalog rule, so each conjunct is stated as its own clause over the shared body rather
//! than encoded in a head form the evaluator refuses. The two statements are equivalent —
//! nothing here mints a witness, so there is no shared existential to lose — and the split
//! is what makes a clause index NOT a rule index, which
//! [`super::program_with_attribution`] exists to keep straight.
//!
//! # The two rules that read a list
//!
//! `scm-int` and `scm-uni` quantify over an RDF collection with the `LIST[…]`
//! meta-notation. Neither needs the list's ORDER — each concludes one triple per member,
//! independently — so both join directly against [`crate::lists`]'s
//! `LIST(head, index, member)` relation, which the pre-pass materializes once per run. The
//! index is bound and unused in both, and that is the honest shape: these two rules are
//! about membership, and the rules that need the position (`prp-adp`, `cax-adc`) are the
//! ones that read it.

use purrdf_datalog::clause::DlClause;

use super::{atom, internal, iri, var};
use crate::lists::LIST_RELATION;
use crate::vocab::{
    OWL_ALLVALUESFROM, OWL_CLASS, OWL_DATATYPEPROPERTY, OWL_EQUIVALENTCLASS,
    OWL_EQUIVALENTPROPERTY, OWL_HASVALUE, OWL_INTERSECTIONOF, OWL_NOTHING, OWL_OBJECTPROPERTY,
    OWL_ONPROPERTY, OWL_SOMEVALUESFROM, OWL_THING, OWL_UNIONOF, RDF_TYPE, RDFS_DOMAIN, RDFS_RANGE,
    RDFS_SUBCLASSOF, RDFS_SUBPROPERTYOF,
};

/// `scm-cls`: `?c rdf:type owl:Class` ⇒ `?c rdfs:subClassOf ?c`,
/// `?c owl:equivalentClass ?c`, `?c rdfs:subClassOf owl:Thing`,
/// `owl:Nothing rdfs:subClassOf ?c`.
///
/// Four clauses over one body, in the order the specification writes the conjuncts.
pub(super) fn class_reflexive() -> Vec<DlClause> {
    let premise = || vec![atom(var("?c"), RDF_TYPE, iri(OWL_CLASS))];
    vec![
        DlClause::datalog(atom(var("?c"), RDFS_SUBCLASSOF, var("?c")), premise()),
        DlClause::datalog(atom(var("?c"), OWL_EQUIVALENTCLASS, var("?c")), premise()),
        DlClause::datalog(atom(var("?c"), RDFS_SUBCLASSOF, iri(OWL_THING)), premise()),
        DlClause::datalog(
            atom(iri(OWL_NOTHING), RDFS_SUBCLASSOF, var("?c")),
            premise(),
        ),
    ]
}

/// `scm-eqc1`: `?c1 owl:equivalentClass ?c2` ⇒ `?c1 rdfs:subClassOf ?c2`,
/// `?c2 rdfs:subClassOf ?c1`.
pub(super) fn equivalent_class() -> Vec<DlClause> {
    vec![
        DlClause::datalog(
            atom(var("?c1"), RDFS_SUBCLASSOF, var("?c2")),
            vec![atom(var("?c1"), OWL_EQUIVALENTCLASS, var("?c2"))],
        ),
        DlClause::datalog(
            atom(var("?c2"), RDFS_SUBCLASSOF, var("?c1")),
            vec![atom(var("?c1"), OWL_EQUIVALENTCLASS, var("?c2"))],
        ),
    ]
}

/// `scm-eqc2`: `?c1 rdfs:subClassOf ?c2`, `?c2 rdfs:subClassOf ?c1` ⇒
/// `?c1 owl:equivalentClass ?c2`.
pub(super) fn equivalent_class_from_mutual_subclass() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?c1"), OWL_EQUIVALENTCLASS, var("?c2")),
        vec![
            atom(var("?c1"), RDFS_SUBCLASSOF, var("?c2")),
            atom(var("?c2"), RDFS_SUBCLASSOF, var("?c1")),
        ],
    )]
}

/// `scm-op`: `?p rdf:type owl:ObjectProperty` ⇒ `?p rdfs:subPropertyOf ?p`,
/// `?p owl:equivalentProperty ?p`.
pub(super) fn object_property_reflexive() -> Vec<DlClause> {
    property_reflexive(OWL_OBJECTPROPERTY)
}

/// `scm-dp`: `?p rdf:type owl:DatatypeProperty` ⇒ `?p rdfs:subPropertyOf ?p`,
/// `?p owl:equivalentProperty ?p`.
pub(super) fn datatype_property_reflexive() -> Vec<DlClause> {
    property_reflexive(OWL_DATATYPEPROPERTY)
}

/// The shared shape of `scm-op` and `scm-dp`: the two rules differ in ONE constant, the
/// class the premise types the property with, and stating that once is what keeps them
/// from drifting apart.
fn property_reflexive(class: &str) -> Vec<DlClause> {
    let premise = || vec![atom(var("?p"), RDF_TYPE, iri(class))];
    vec![
        DlClause::datalog(atom(var("?p"), RDFS_SUBPROPERTYOF, var("?p")), premise()),
        DlClause::datalog(
            atom(var("?p"), OWL_EQUIVALENTPROPERTY, var("?p")),
            premise(),
        ),
    ]
}

/// `scm-eqp1`: `?p1 owl:equivalentProperty ?p2` ⇒ `?p1 rdfs:subPropertyOf ?p2`,
/// `?p2 rdfs:subPropertyOf ?p1`.
pub(super) fn equivalent_property() -> Vec<DlClause> {
    vec![
        DlClause::datalog(
            atom(var("?p1"), RDFS_SUBPROPERTYOF, var("?p2")),
            vec![atom(var("?p1"), OWL_EQUIVALENTPROPERTY, var("?p2"))],
        ),
        DlClause::datalog(
            atom(var("?p2"), RDFS_SUBPROPERTYOF, var("?p1")),
            vec![atom(var("?p1"), OWL_EQUIVALENTPROPERTY, var("?p2"))],
        ),
    ]
}

/// `scm-eqp2`: `?p1 rdfs:subPropertyOf ?p2`, `?p2 rdfs:subPropertyOf ?p1` ⇒
/// `?p1 owl:equivalentProperty ?p2`.
pub(super) fn equivalent_property_from_mutual_subproperty() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?p1"), OWL_EQUIVALENTPROPERTY, var("?p2")),
        vec![
            atom(var("?p1"), RDFS_SUBPROPERTYOF, var("?p2")),
            atom(var("?p2"), RDFS_SUBPROPERTYOF, var("?p1")),
        ],
    )]
}

/// `scm-dom1`: `?p rdfs:domain ?c1`, `?c1 rdfs:subClassOf ?c2` ⇒ `?p rdfs:domain ?c2`.
pub(super) fn domain_widened() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?p"), RDFS_DOMAIN, var("?c2")),
        vec![
            atom(var("?p"), RDFS_DOMAIN, var("?c1")),
            atom(var("?c1"), RDFS_SUBCLASSOF, var("?c2")),
        ],
    )]
}

/// `scm-dom2`: `?p2 rdfs:domain ?c`, `?p1 rdfs:subPropertyOf ?p2` ⇒ `?p1 rdfs:domain ?c`.
pub(super) fn domain_inherited() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?p1"), RDFS_DOMAIN, var("?c")),
        vec![
            atom(var("?p2"), RDFS_DOMAIN, var("?c")),
            atom(var("?p1"), RDFS_SUBPROPERTYOF, var("?p2")),
        ],
    )]
}

/// `scm-rng1`: `?p rdfs:range ?c1`, `?c1 rdfs:subClassOf ?c2` ⇒ `?p rdfs:range ?c2`.
pub(super) fn range_widened() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?p"), RDFS_RANGE, var("?c2")),
        vec![
            atom(var("?p"), RDFS_RANGE, var("?c1")),
            atom(var("?c1"), RDFS_SUBCLASSOF, var("?c2")),
        ],
    )]
}

/// `scm-rng2`: `?p2 rdfs:range ?c`, `?p1 rdfs:subPropertyOf ?p2` ⇒ `?p1 rdfs:range ?c`.
pub(super) fn range_inherited() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?p1"), RDFS_RANGE, var("?c")),
        vec![
            atom(var("?p2"), RDFS_RANGE, var("?c")),
            atom(var("?p1"), RDFS_SUBPROPERTYOF, var("?p2")),
        ],
    )]
}

/// `scm-hv`: `?c1 owl:hasValue ?i`, `?c1 owl:onProperty ?p1`, `?c2 owl:hasValue ?i`,
/// `?c2 owl:onProperty ?p2`, `?p1 rdfs:subPropertyOf ?p2` ⇒ `?c1 rdfs:subClassOf ?c2`.
pub(super) fn has_value_restriction() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?c1"), RDFS_SUBCLASSOF, var("?c2")),
        vec![
            atom(var("?c1"), OWL_HASVALUE, var("?i")),
            atom(var("?c1"), OWL_ONPROPERTY, var("?p1")),
            atom(var("?c2"), OWL_HASVALUE, var("?i")),
            atom(var("?c2"), OWL_ONPROPERTY, var("?p2")),
            atom(var("?p1"), RDFS_SUBPROPERTYOF, var("?p2")),
        ],
    )]
}

/// `scm-svf1`: two `owl:someValuesFrom` restrictions on the SAME property whose fillers are
/// related by `rdfs:subClassOf` ⇒ `?c1 rdfs:subClassOf ?c2`.
pub(super) fn some_values_filler() -> Vec<DlClause> {
    vec![restriction_by_filler(OWL_SOMEVALUESFROM)]
}

/// `scm-svf2`: two `owl:someValuesFrom` restrictions on the SAME filler whose properties are
/// related by `rdfs:subPropertyOf` ⇒ `?c1 rdfs:subClassOf ?c2`.
pub(super) fn some_values_property() -> Vec<DlClause> {
    vec![restriction_by_property(OWL_SOMEVALUESFROM, false)]
}

/// `scm-avf1`: two `owl:allValuesFrom` restrictions on the SAME property whose fillers are
/// related by `rdfs:subClassOf` ⇒ `?c1 rdfs:subClassOf ?c2`.
pub(super) fn all_values_filler() -> Vec<DlClause> {
    vec![restriction_by_filler(OWL_ALLVALUESFROM)]
}

/// `scm-avf2`: two `owl:allValuesFrom` restrictions on the SAME filler whose properties are
/// related by `rdfs:subPropertyOf` ⇒ `?c2 rdfs:subClassOf ?c1`.
///
/// The conclusion is CONTRAVARIANT, unlike `scm-svf2`'s, and that is the specification's
/// own asymmetry rather than a transcription slip: a universal restriction over a WIDER
/// property is the stronger class, so the sub-class relation runs the other way.
pub(super) fn all_values_property() -> Vec<DlClause> {
    vec![restriction_by_property(OWL_ALLVALUESFROM, true)]
}

/// The shared shape of `scm-svf1` and `scm-avf1`: two restrictions on one property, ordered
/// by their fillers.
fn restriction_by_filler(restriction: &str) -> DlClause {
    DlClause::datalog(
        atom(var("?c1"), RDFS_SUBCLASSOF, var("?c2")),
        vec![
            atom(var("?c1"), restriction, var("?y1")),
            atom(var("?c1"), OWL_ONPROPERTY, var("?p")),
            atom(var("?c2"), restriction, var("?y2")),
            atom(var("?c2"), OWL_ONPROPERTY, var("?p")),
            atom(var("?y1"), RDFS_SUBCLASSOF, var("?y2")),
        ],
    )
}

/// The shared shape of `scm-svf2` and `scm-avf2`: two restrictions on one filler, ordered by
/// their properties. `contravariant` selects which way the conclusion runs, which is the
/// ONE thing that differs between the two rules.
fn restriction_by_property(restriction: &str, contravariant: bool) -> DlClause {
    let body = vec![
        atom(var("?c1"), restriction, var("?y")),
        atom(var("?c1"), OWL_ONPROPERTY, var("?p1")),
        atom(var("?c2"), restriction, var("?y")),
        atom(var("?c2"), OWL_ONPROPERTY, var("?p2")),
        atom(var("?p1"), RDFS_SUBPROPERTYOF, var("?p2")),
    ];
    let head = if contravariant {
        atom(var("?c2"), RDFS_SUBCLASSOF, var("?c1"))
    } else {
        atom(var("?c1"), RDFS_SUBCLASSOF, var("?c2"))
    };
    DlClause::datalog(head, body)
}

/// `scm-int`: `?c owl:intersectionOf ?x` over the list `?c1 … ?cn` ⇒ `?c rdfs:subClassOf ?ci`
/// for every `?ci`.
pub(super) fn intersection() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?c"), RDFS_SUBCLASSOF, var("?ci")),
        vec![
            atom(var("?c"), OWL_INTERSECTIONOF, var("?x")),
            internal(LIST_RELATION, var("?x"), var("?ci"), var("?i")),
        ],
    )]
}

/// `scm-uni`: `?c owl:unionOf ?x` over the list `?c1 … ?cn` ⇒ `?ci rdfs:subClassOf ?c` for
/// every `?ci`.
pub(super) fn union() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?ci"), RDFS_SUBCLASSOF, var("?c")),
        vec![
            atom(var("?c"), OWL_UNIONOF, var("?x")),
            internal(LIST_RELATION, var("?x"), var("?ci"), var("?i")),
        ],
    )]
}

/// The `scm-*` rules this chase states, in OWL 2 Profiles Table 9 order.
///
/// Handed to [`super::collect_families`], which asks each family in turn; see that macro
/// for the protocol and [`super::declare_chase_rules`] for what an entry means.
macro_rules! scm_rules {
    ($continue:ident, $rest:tt, $($rules:tt)*) => {
        $continue! { $rest $($rules)*
            /// `scm-cls` — an `owl:Class` is a sub-class of itself and of `owl:Thing`,
            /// equivalent to itself, and a super-class of `owl:Nothing`. Four clauses.
            /// `OWL-RL` only.
            ClassReflexive {
                id: ScmCls,
                lanes: [OwlRl],
                clauses: scm::class_reflexive,
            },
            /// `scm-eqc1` — `owl:equivalentClass` is mutual `rdfs:subClassOf`. `OWL-RL`
            /// only.
            EquivalentClass {
                id: ScmEqc1,
                lanes: [OwlRl],
                clauses: scm::equivalent_class,
            },
            /// `scm-eqc2` — mutual `rdfs:subClassOf` is `owl:equivalentClass`, the
            /// converse of `scm-eqc1`. `OWL-RL` only.
            EquivalentClassFromMutualSubclass {
                id: ScmEqc2,
                lanes: [OwlRl],
                clauses: scm::equivalent_class_from_mutual_subclass,
            },
            /// `scm-op` — an `owl:ObjectProperty` is a sub-property of, and equivalent
            /// to, itself. `OWL-RL` only.
            ObjectPropertyReflexive {
                id: ScmOp,
                lanes: [OwlRl],
                clauses: scm::object_property_reflexive,
            },
            /// `scm-dp` — the same for an `owl:DatatypeProperty`. `OWL-RL` only.
            DatatypePropertyReflexive {
                id: ScmDp,
                lanes: [OwlRl],
                clauses: scm::datatype_property_reflexive,
            },
            /// `scm-eqp1` — `owl:equivalentProperty` is mutual `rdfs:subPropertyOf`.
            /// `OWL-RL` only.
            EquivalentProperty {
                id: ScmEqp1,
                lanes: [OwlRl],
                clauses: scm::equivalent_property,
            },
            /// `scm-eqp2` — mutual `rdfs:subPropertyOf` is `owl:equivalentProperty`, the
            /// converse of `scm-eqp1`. `OWL-RL` only.
            EquivalentPropertyFromMutualSubproperty {
                id: ScmEqp2,
                lanes: [OwlRl],
                clauses: scm::equivalent_property_from_mutual_subproperty,
            },
            /// `scm-dom1` — a domain widens along `rdfs:subClassOf`. `OWL-RL` only.
            DomainWidened {
                id: ScmDom1,
                lanes: [OwlRl],
                clauses: scm::domain_widened,
            },
            /// `scm-dom2` — a domain is inherited along `rdfs:subPropertyOf`. `OWL-RL`
            /// only.
            DomainInherited {
                id: ScmDom2,
                lanes: [OwlRl],
                clauses: scm::domain_inherited,
            },
            /// `scm-rng1` — a range widens along `rdfs:subClassOf`. `OWL-RL` only.
            RangeWidened {
                id: ScmRng1,
                lanes: [OwlRl],
                clauses: scm::range_widened,
            },
            /// `scm-rng2` — a range is inherited along `rdfs:subPropertyOf`. `OWL-RL`
            /// only.
            RangeInherited {
                id: ScmRng2,
                lanes: [OwlRl],
                clauses: scm::range_inherited,
            },
            /// `scm-hv` — two `owl:hasValue` restrictions on one value, ordered by their
            /// properties. `OWL-RL` only.
            HasValueRestriction {
                id: ScmHv,
                lanes: [OwlRl],
                clauses: scm::has_value_restriction,
            },
            /// `scm-svf1` — two `owl:someValuesFrom` restrictions on one property,
            /// ordered by their fillers. `OWL-RL` only.
            SomeValuesFiller {
                id: ScmSvf1,
                lanes: [OwlRl],
                clauses: scm::some_values_filler,
            },
            /// `scm-svf2` — two `owl:someValuesFrom` restrictions on one filler, ordered
            /// by their properties. `OWL-RL` only.
            SomeValuesProperty {
                id: ScmSvf2,
                lanes: [OwlRl],
                clauses: scm::some_values_property,
            },
            /// `scm-avf1` — two `owl:allValuesFrom` restrictions on one property, ordered
            /// by their fillers. `OWL-RL` only.
            AllValuesFiller {
                id: ScmAvf1,
                lanes: [OwlRl],
                clauses: scm::all_values_filler,
            },
            /// `scm-avf2` — two `owl:allValuesFrom` restrictions on one filler, ordered
            /// CONTRAVARIANTLY by their properties. `OWL-RL` only.
            AllValuesProperty {
                id: ScmAvf2,
                lanes: [OwlRl],
                clauses: scm::all_values_property,
            },
            /// `scm-int` — an intersection is a sub-class of each of its members.
            /// `OWL-RL` only.
            Intersection {
                id: ScmInt,
                lanes: [OwlRl],
                clauses: scm::intersection,
            },
            /// `scm-uni` — a union is a super-class of each of its members. `OWL-RL`
            /// only.
            Union {
                id: ScmUni,
                lanes: [OwlRl],
                clauses: scm::union,
            },
        }
    };
}

pub(crate) use scm_rules;
