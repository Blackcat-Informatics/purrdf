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
//! `rdfD2` is RDF entailment's single pattern. Both the bare-`RDF` lane and the `RDFS`
//! lane fire it, because RDFS entailment subsumes RDF entailment — `rules(Regime::Rdfs)`
//! has always said so, listing the §8.1.1 patterns ahead of the §9.2.1 ones.
//!
//! # The four patterns this family does NOT state, and the one reason
//!
//! `rdfD1`, `rdfD1a`, `rdfs14` and `rdfs14a` are absent, and all four are absent for a
//! single structural reason rather than four separate ones: **their conclusions are
//! existentially quantified.** Each concludes about a FRESH blank node — a surrogate the
//! specification writes `_:nnn` — either standing for a datatyped literal (`rdfD1`), for a
//! recognized datatype's inhabited value space (`rdfD1a`), for a triple term (`rdfs14`),
//! or for a proposition in any graph at all, including the empty one (`rdfs14a`).
//!
//! A DL clause CAN carry that shape: it is
//! [`HeadForm::Existential`](purrdf_datalog::clause::HeadForm::Existential), and the IR
//! represents it as a first-class value. What cannot is this crate's evaluator.
//! [`compile`](purrdf_datalog::seminaive::compile) refuses every non-atomic head form by
//! design, and [`crate::engine`] rests on the evaluator minting no terms —
//! `Terms::value` is total precisely because every term of every derived fact was either
//! seeded or is a program constant. A surrogate blank node is neither.
//!
//! That is a limit worth stating twice, because the obvious repair is wrong. Minting the
//! surrogates would not merely widen the closure, it would make this crate answer SPARQL's
//! RDFS entailment regime incorrectly: the W3C case `rdfs13` asks `?L rdf:type
//! rdfs:Literal` over a graph whose only literal is `"foo"`, and requires ZERO solutions —
//! while plain RDFS entailment, with `rdfD1` surrogates materialized, gives `_:nnn
//! rdf:type xsd:string` and hence `_:nnn rdf:type rdfs:Literal` through `rdfs1`, `rdfs13`
//! and `rdfs9`. The regime does not admit an answer that binds a variable to a surrogate,
//! so a chase that produced one would be wrong where this one is merely incomplete.
//!
//! Both halves of that incompleteness are REPORTED rather than assumed:
//! [`Construct::DatatypeValueSpace`](crate::Construct::DatatypeValueSpace) for the two RDF
//! patterns and [`Construct::TripleTerm`](crate::Construct::TripleTerm) for the two triple
//! term ones, and `rules(Regime::Rdfs)` minus [`crate::implemented`] names all four.

use purrdf_datalog::clause::DlClause;

use super::{atom, iri, quad, var};
use crate::vocab::{
    RDF_DIRLANGSTRING, RDF_LANGSTRING, RDF_PROPERTY, RDF_TYPE, RDFS_CLASS,
    RDFS_CONTAINERMEMBERSHIPPROPERTY, RDFS_DATATYPE, RDFS_DOMAIN, RDFS_LITERAL, RDFS_MEMBER,
    RDFS_RANGE, RDFS_RESOURCE, RDFS_SUBCLASSOF, RDFS_SUBPROPERTYOF, XSD_STRING,
};

/// `rdfD2`: `T(?s, ?p, ?o)` ⇒ `?p rdf:type rdf:Property`.
pub(super) fn predicate_property() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?p"), RDF_TYPE, iri(RDF_PROPERTY)),
        vec![quad(var("?s"), var("?p"), var("?o"))],
    )]
}

/// The datatype IRIs this chase recognizes — RDF 1.2 Semantics §8's mandatory `D`.
///
/// "The datatype IRIs `rdf:langString`, `rdf:dirLangString`, and `xsd:string` MUST be
/// recognized by all RDF interpretations", and "when `D` is
/// `{rdf:langString, rdf:dirLangString, xsd:string}` then we simply say S RDF entails E".
/// So this is not a default this crate invented to fill a hole where a caller's
/// configuration should be: it is the `D` the unqualified phrase "RDFS entails" is
/// defined against, and it is fixed by the specification. `rdf:XMLLiteral`, `rdf:HTML` and
/// `rdf:JSON` are exactly the ones an interpretation MAY decline to recognize, and this
/// chase declines; a wider `D` is what
/// [`Construct::DatatypeValueSpace`](crate::Construct::DatatypeValueSpace) reports.
const RECOGNIZED_DATATYPES: [&str; 3] = [RDF_LANGSTRING, RDF_DIRLANGSTRING, XSD_STRING];

/// `rdfs1`: any IRI `aaa` in `D` ⇒ `aaa rdf:type rdfs:Datatype`.
///
/// One premise-free clause per recognized datatype, in [`RECOGNIZED_DATATYPES`] order.
/// The rule quantifies over `D` rather than over the graph, so it holds for every input
/// including the empty one, and a clause with no body is exactly that statement.
pub(super) fn datatype_typed() -> Vec<DlClause> {
    RECOGNIZED_DATATYPES
        .into_iter()
        .map(|datatype| {
            DlClause::datalog(
                atom(iri(datatype), RDF_TYPE, iri(RDFS_DATATYPE)),
                Vec::new(),
            )
        })
        .collect()
}

/// `rdfs4`: any triple `ttt` in which `xxx` appears ⇒ `xxx rdf:type rdfs:Resource`.
///
/// Three clauses, one per position, because RDF 1.2 MERGED RDF 1.0's subject-only
/// `rdfs4a` and object-only `rdfs4b` into a single rule over EVERY position of a triple —
/// so the predicate clause is not an extra this crate added, it is the half of the merged
/// rule the superseded pair never had.
///
/// The object clause concludes into subject position, so a literal or triple-term object
/// yields a generalized-RDF conclusion the RDF 1.2 IR cannot hold; it is derived, dropped
/// at the materialization boundary and reported as
/// [`Construct::GeneralizedRdf`](crate::Construct::GeneralizedRdf), exactly like
/// `rdfs3`'s.
pub(super) fn resource_typed() -> Vec<DlClause> {
    ["?s", "?p", "?o"]
        .into_iter()
        .map(|position| {
            DlClause::datalog(
                atom(var(position), RDF_TYPE, iri(RDFS_RESOURCE)),
                vec![quad(var("?s"), var("?p"), var("?o"))],
            )
        })
        .collect()
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

/// `rdfs12`: `?p rdf:type rdfs:ContainerMembershipProperty` ⇒ `?p rdfs:subPropertyOf
/// rdfs:member`.
///
/// The premise is a typing the graph asserts, so the rule fires on a caller's container
/// property. It does NOT fire on `rdf:_1`, `rdf:_2`, …: the axiom that types those is a
/// member of the unbounded family [`crate::axioms`] cannot assert, which is what
/// [`Construct::AxiomaticTriples`](crate::Construct::AxiomaticTriples) reports.
pub(super) fn container_membership() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?p"), RDFS_SUBPROPERTYOF, iri(RDFS_MEMBER)),
        vec![atom(
            var("?p"),
            RDF_TYPE,
            iri(RDFS_CONTAINERMEMBERSHIPPROPERTY),
        )],
    )]
}

/// `rdfs13`: `?d rdf:type rdfs:Datatype` ⇒ `?d rdfs:subClassOf rdfs:Literal`.
pub(super) fn datatype_literal() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?d"), RDFS_SUBCLASSOF, iri(RDFS_LITERAL)),
        vec![atom(var("?d"), RDF_TYPE, iri(RDFS_DATATYPE))],
    )]
}

/// The RDF and RDFS patterns this chase states, in RDF 1.2 Semantics order.
///
/// Handed to [`super::collect_families`], which asks each family in turn; see that macro
/// for the protocol and [`super::declare_chase_rules`] for what an entry means.
macro_rules! rdfs_rules {
    ($continue:ident, $rest:tt, $($rules:tt)*) => {
        $continue! { $rest $($rules)*
            /// `rdfD2` — every predicate is an `rdf:Property`. RDFS entailment subsumes
            /// RDF entailment, so both lanes fire it.
            PredicateProperty {
                id: RdfD2,
                lanes: [Rdf, Rdfs],
                clauses: rdfs::predicate_property,
            },
            /// `rdfs1` — every recognized datatype IRI is an `rdfs:Datatype`. Premise-free.
            DatatypeTyped {
                id: Rdfs1,
                lanes: [Rdfs],
                clauses: rdfs::datatype_typed,
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
            /// `rdfs4` — every term of every triple, in every position, is an
            /// `rdfs:Resource`.
            ResourceTyped {
                id: Rdfs4,
                lanes: [Rdfs],
                clauses: rdfs::resource_typed,
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
            /// `rdfs12` — a container-membership property is a sub-property of
            /// `rdfs:member`.
            ContainerMembership {
                id: Rdfs12,
                lanes: [Rdfs],
                clauses: rdfs::container_membership,
            },
            /// `rdfs13` — a datatype is a sub-class of `rdfs:Literal`.
            DatatypeLiteral {
                id: Rdfs13,
                lanes: [Rdfs],
                clauses: rdfs::datatype_literal,
            },
        }
    };
}

pub(crate) use rdfs_rules;
