// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The RDF and RDFS entailment patterns — RDF 1.2 Semantics §8.1.1 and §9.2.1.
//!
//! This family holds the patterns the RDF and RDFS lanes are DEFINED by, in the numeric
//! order the specification writes them, and it holds them for the `OWL-RL` lane too. Nine
//! of them appear in the OWL 2 RL/RDF tables under a different name — `rdfs2` is `prp-dom`,
//! `rdfs3` is `prp-rng`, `rdfs5` is `scm-spo`, `rdfs7` is `prp-spo1`, `rdfs9` is `cax-sco`,
//! `rdfs11` is `scm-sco` — and a renamed rule stays HERE rather than moving to the OWL
//! family that renames it, because it is one clause with two names and the RDFS numbering is
//! the one that orders it. [`super::ChaseRule::rule_id`] answers with the name of whichever
//! calculus ran.
//!
//! Three more — `rdfs6`, `rdfs8` and `rdfs10` — have no OWL 2 RL name at all, because
//! OWL 2 RL/RDF omits them from its tables. The `OWL-RL` lane fires them anyway and reports
//! them under their RDFS name.
//!
//! `rdfD2` is RDF entailment's single pattern and the only rule the bare-`RDF` lane fires.
//!
//! The patterns this family does not yet state are the axiomatic and container ones —
//! `rdfD1`, `rdfD1a`, `rdfs1`, `rdfs4`, `rdfs12`, `rdfs13`, `rdfs14`, `rdfs14a` — each of
//! which needs either the datatype map or vocabulary this chase does not carry.

use purrdf_datalog::clause::DlClause;

use super::{atom, iri, quad, var};
use crate::vocab::{
    RDF_PROPERTY, RDF_TYPE, RDFS_CLASS, RDFS_DOMAIN, RDFS_RANGE, RDFS_RESOURCE, RDFS_SUBCLASSOF,
    RDFS_SUBPROPERTYOF,
};

/// `rdfD2`: `T(?s, ?p, ?o)` ⇒ `?p rdf:type rdf:Property`.
pub(super) fn predicate_property() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?p"), RDF_TYPE, iri(RDF_PROPERTY)),
        vec![quad(var("?s"), var("?p"), var("?o"))],
    )]
}

/// `rdfs2` / `prp-dom`: `?p rdfs:domain ?c`, `T(?x, ?p, ?y)` ⇒ `?x rdf:type ?c`.
pub(super) fn domain() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?x"), RDF_TYPE, var("?c")),
        vec![
            atom(var("?p"), RDFS_DOMAIN, var("?c")),
            quad(var("?x"), var("?p"), var("?y")),
        ],
    )]
}

/// `rdfs3` / `prp-rng`: `?p rdfs:range ?c`, `T(?x, ?p, ?y)` ⇒ `?y rdf:type ?c`.
pub(super) fn range() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?y"), RDF_TYPE, var("?c")),
        vec![
            atom(var("?p"), RDFS_RANGE, var("?c")),
            quad(var("?x"), var("?p"), var("?y")),
        ],
    )]
}

/// `rdfs5` / `scm-spo`: `rdfs:subPropertyOf` is transitive.
pub(super) fn sub_property_transitive() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?p1"), RDFS_SUBPROPERTYOF, var("?p3")),
        vec![
            atom(var("?p1"), RDFS_SUBPROPERTYOF, var("?p2")),
            atom(var("?p2"), RDFS_SUBPROPERTYOF, var("?p3")),
        ],
    )]
}

/// `rdfs6`: `?p rdf:type rdf:Property` ⇒ `?p rdfs:subPropertyOf ?p`.
pub(super) fn sub_property_reflexive() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?p"), RDFS_SUBPROPERTYOF, var("?p")),
        vec![atom(var("?p"), RDF_TYPE, iri(RDF_PROPERTY))],
    )]
}

/// `rdfs7` / `prp-spo1`: `?p1 rdfs:subPropertyOf ?p2`, `T(?x, ?p1, ?y)` ⇒ `T(?x, ?p2, ?y)`.
pub(super) fn sub_property_rewrite() -> Vec<DlClause> {
    vec![DlClause::datalog(
        quad(var("?x"), var("?p2"), var("?y")),
        vec![
            atom(var("?p1"), RDFS_SUBPROPERTYOF, var("?p2")),
            quad(var("?x"), var("?p1"), var("?y")),
        ],
    )]
}

/// `rdfs8`: `?c rdf:type rdfs:Class` ⇒ `?c rdfs:subClassOf rdfs:Resource`.
pub(super) fn class_resource() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?c"), RDFS_SUBCLASSOF, iri(RDFS_RESOURCE)),
        vec![atom(var("?c"), RDF_TYPE, iri(RDFS_CLASS))],
    )]
}

/// `rdfs9` / `cax-sco`: `?c1 rdfs:subClassOf ?c2`, `?x rdf:type ?c1` ⇒ `?x rdf:type ?c2`.
pub(super) fn sub_class_instance() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?x"), RDF_TYPE, var("?c2")),
        vec![
            atom(var("?c1"), RDFS_SUBCLASSOF, var("?c2")),
            atom(var("?x"), RDF_TYPE, var("?c1")),
        ],
    )]
}

/// `rdfs10`: `?c rdf:type rdfs:Class` ⇒ `?c rdfs:subClassOf ?c`.
pub(super) fn sub_class_reflexive() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?c"), RDFS_SUBCLASSOF, var("?c")),
        vec![atom(var("?c"), RDF_TYPE, iri(RDFS_CLASS))],
    )]
}

/// `rdfs11` / `scm-sco`: `rdfs:subClassOf` is transitive.
pub(super) fn sub_class_transitive() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?c1"), RDFS_SUBCLASSOF, var("?c3")),
        vec![
            atom(var("?c1"), RDFS_SUBCLASSOF, var("?c2")),
            atom(var("?c2"), RDFS_SUBCLASSOF, var("?c3")),
        ],
    )]
}

/// The RDF and RDFS patterns this chase states, in RDF 1.2 Semantics order.
///
/// Handed to [`super::collect_families`], which asks each family in turn; see that macro
/// for the protocol and [`super::declare_chase_rules`] for what an entry means.
macro_rules! rdfs_rules {
    ($continue:ident, $rest:tt, $($rules:tt)*) => {
        $continue! { $rest $($rules)*
            /// `rdfD2` — every predicate is an `rdf:Property`. The bare-`RDF` lane only.
            PredicateProperty {
                id: RdfD2,
                lanes: [Rdf],
                clauses: rdfs::predicate_property,
            },
            /// `rdfs2` / `prp-dom` — a domain declaration types the subject.
            Domain {
                id: Rdfs2,
                owl: PrpDom,
                lanes: [Rdfs, OwlRl],
                clauses: rdfs::domain,
            },
            /// `rdfs3` / `prp-rng` — a range declaration types the object.
            Range {
                id: Rdfs3,
                owl: PrpRng,
                lanes: [Rdfs, OwlRl],
                clauses: rdfs::range,
            },
            /// `rdfs5` / `scm-spo` — `rdfs:subPropertyOf` is transitive.
            SubPropertyTransitive {
                id: Rdfs5,
                owl: ScmSpo,
                lanes: [Rdfs, OwlRl],
                clauses: rdfs::sub_property_transitive,
            },
            /// `rdfs6` — a property is a sub-property of itself.
            SubPropertyReflexive {
                id: Rdfs6,
                lanes: [Rdfs, OwlRl],
                clauses: rdfs::sub_property_reflexive,
            },
            /// `rdfs7` / `prp-spo1` — a sub-property assertion re-predicates a triple.
            SubPropertyRewrite {
                id: Rdfs7,
                owl: PrpSpo1,
                lanes: [Rdfs, OwlRl],
                clauses: rdfs::sub_property_rewrite,
            },
            /// `rdfs8` — a class is a sub-class of `rdfs:Resource`.
            ClassResource {
                id: Rdfs8,
                lanes: [Rdfs, OwlRl],
                clauses: rdfs::class_resource,
            },
            /// `rdfs9` / `cax-sco` — a sub-class assertion re-types an instance.
            SubClassInstance {
                id: Rdfs9,
                owl: CaxSco,
                lanes: [Rdfs, OwlRl],
                clauses: rdfs::sub_class_instance,
            },
            /// `rdfs10` — a class is a sub-class of itself.
            SubClassReflexive {
                id: Rdfs10,
                lanes: [Rdfs, OwlRl],
                clauses: rdfs::sub_class_reflexive,
            },
            /// `rdfs11` / `scm-sco` — `rdfs:subClassOf` is transitive.
            SubClassTransitive {
                id: Rdfs11,
                owl: ScmSco,
                lanes: [Rdfs, OwlRl],
                clauses: rdfs::sub_class_transitive,
            },
        }
    };
}

pub(crate) use rdfs_rules;
