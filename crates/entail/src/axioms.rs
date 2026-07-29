// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The FINITE part of the RDF and RDFS axiomatic triples, transcribed from RDF 1.2
//! Semantics.
//!
//! # Why this table exists
//!
//! `S RDFS entails E` is defined over the interpretations that satisfy `S` **and** the
//! axiomatic triples — the axioms are given, exactly like `S`'s own triples, and no rule
//! of §9.2.1 concludes one. So the faithful encoding of the definition is to hand them to
//! the evaluator as PREMISES, which is what [`crate::engine::close`] does, and not to
//! invent a rule id to credit them to.
//!
//! Their consequences are what a caller actually sees. `rdfs:subClassOf` has an axiomatic
//! `rdfs:domain` and `rdfs:range` of `rdfs:Class`, so a graph that merely says
//! `:a rdfs:subClassOf :b` RDFS-entails `:a rdf:type rdfs:Class` and
//! `:b rdf:type rdfs:Class` through `rdfs2` / `rdfs3` — and only then does `rdfs10`
//! license `:a rdfs:subClassOf :a`. Without the axioms that path does not exist and those
//! reflexive triples are simply missing; with them, they are drawn where the specification
//! licenses them and nowhere else.
//!
//! # Why the set below is FINITE, and what is left out
//!
//! Both axiom tables are written with a trailing `…`, and both times the ellipsis stands
//! for exactly one thing: the container-membership family `rdf:_1`, `rdf:_2`, … . RDF's
//! table ends `rdf:_1 rdf:type rdf:Property .` `rdf:_2 rdf:type rdf:Property .` `…`, and
//! RDFS's ends with the three-triple block `rdf:_n rdf:type
//! rdfs:ContainerMembershipProperty .` / `rdf:_n rdfs:domain rdfs:Resource .` /
//! `rdf:_n rdfs:range rdfs:Resource .` repeated for every `n`. Everything ABOVE those
//! ellipses is a fixed, finite list about the RDF and RDFS vocabulary itself, and that
//! list is what this module holds — [`RDF_AXIOMS`] transcribes the first table and
//! [`RDFS_AXIOMS`] the second, each in the specification's own order.
//!
//! No forward chase can materialize the `rdf:_n` family, so it is not asserted, and the
//! run reports [`Construct::AxiomaticTriples`](crate::Construct::AxiomaticTriples) saying
//! so. That is the whole of what is missing: the boundary names an unbounded family, not
//! an unwritten table.
//!
//! # `rdfs:Proposition`
//!
//! RDF 1.2 adds `rdf:reifies rdfs:range rdfs:Proposition .` to the RDFS table, which is
//! the only axiom that mentions the class `rdfs14` / `rdfs14a` are about. It is
//! transcribed like the rest; it needs no rule of its own.

use crate::vocab::{
    RDF_ALT, RDF_BAG, RDF_FIRST, RDF_LIST, RDF_NIL, RDF_OBJECT, RDF_PREDICATE, RDF_PROPERTY,
    RDF_REIFIES, RDF_REST, RDF_SEQ, RDF_STATEMENT, RDF_SUBJECT, RDF_TYPE, RDF_VALUE, RDFS_CLASS,
    RDFS_COMMENT, RDFS_CONTAINER, RDFS_CONTAINERMEMBERSHIPPROPERTY, RDFS_DATATYPE, RDFS_DOMAIN,
    RDFS_ISDEFINEDBY, RDFS_LABEL, RDFS_LITERAL, RDFS_MEMBER, RDFS_PROPOSITION, RDFS_RANGE,
    RDFS_RESOURCE, RDFS_SEEALSO, RDFS_SUBCLASSOF, RDFS_SUBPROPERTYOF,
};

/// One axiomatic triple, as three IRIs. Every axiom of both tables is IRI-only, so no
/// term sum type is needed and none is offered.
pub(crate) type Axiom = (&'static str, &'static str, &'static str);

/// RDF 1.2 Semantics §8, table "RDF axioms" — every triple above the `…`.
///
/// The ellipsis the table ends with stands for the container-membership family
/// `rdf:_1 rdf:type rdf:Property .`, `rdf:_2 …`, which is unbounded and therefore not
/// here; see the [module docs](self).
pub(crate) const RDF_AXIOMS: [Axiom; 9] = [
    (RDF_TYPE, RDF_TYPE, RDF_PROPERTY),
    (RDF_SUBJECT, RDF_TYPE, RDF_PROPERTY),
    (RDF_PREDICATE, RDF_TYPE, RDF_PROPERTY),
    (RDF_OBJECT, RDF_TYPE, RDF_PROPERTY),
    (RDF_REIFIES, RDF_TYPE, RDF_PROPERTY),
    (RDF_FIRST, RDF_TYPE, RDF_PROPERTY),
    (RDF_REST, RDF_TYPE, RDF_PROPERTY),
    (RDF_VALUE, RDF_TYPE, RDF_PROPERTY),
    (RDF_NIL, RDF_TYPE, RDF_LIST),
];

/// RDF 1.2 Semantics §9, table "RDFS axiomatic triples" — every triple above the `…`.
///
/// Transcribed in the specification's own order: the seventeen `rdfs:domain` axioms, the
/// seventeen `rdfs:range` axioms, then the six sub-class / sub-property axioms. The
/// ellipsis stands for the three-triple `rdf:_n` block repeated for every `n`, which is
/// unbounded and therefore not here; see the [module docs](self).
pub(crate) const RDFS_AXIOMS: [Axiom; 40] = [
    // --- domain ---
    (RDF_TYPE, RDFS_DOMAIN, RDFS_RESOURCE),
    (RDF_REIFIES, RDFS_DOMAIN, RDFS_RESOURCE),
    (RDFS_DOMAIN, RDFS_DOMAIN, RDF_PROPERTY),
    (RDFS_RANGE, RDFS_DOMAIN, RDF_PROPERTY),
    (RDFS_SUBPROPERTYOF, RDFS_DOMAIN, RDF_PROPERTY),
    (RDFS_SUBCLASSOF, RDFS_DOMAIN, RDFS_CLASS),
    (RDF_SUBJECT, RDFS_DOMAIN, RDF_STATEMENT),
    (RDF_PREDICATE, RDFS_DOMAIN, RDF_STATEMENT),
    (RDF_OBJECT, RDFS_DOMAIN, RDF_STATEMENT),
    (RDFS_MEMBER, RDFS_DOMAIN, RDFS_RESOURCE),
    (RDF_FIRST, RDFS_DOMAIN, RDF_LIST),
    (RDF_REST, RDFS_DOMAIN, RDF_LIST),
    (RDFS_SEEALSO, RDFS_DOMAIN, RDFS_RESOURCE),
    (RDFS_ISDEFINEDBY, RDFS_DOMAIN, RDFS_RESOURCE),
    (RDFS_COMMENT, RDFS_DOMAIN, RDFS_RESOURCE),
    (RDFS_LABEL, RDFS_DOMAIN, RDFS_RESOURCE),
    (RDF_VALUE, RDFS_DOMAIN, RDFS_RESOURCE),
    // --- range ---
    (RDF_TYPE, RDFS_RANGE, RDFS_CLASS),
    (RDF_REIFIES, RDFS_RANGE, RDFS_PROPOSITION),
    (RDFS_DOMAIN, RDFS_RANGE, RDFS_CLASS),
    (RDFS_RANGE, RDFS_RANGE, RDFS_CLASS),
    (RDFS_SUBPROPERTYOF, RDFS_RANGE, RDF_PROPERTY),
    (RDFS_SUBCLASSOF, RDFS_RANGE, RDFS_CLASS),
    (RDF_SUBJECT, RDFS_RANGE, RDFS_RESOURCE),
    (RDF_PREDICATE, RDFS_RANGE, RDFS_RESOURCE),
    (RDF_OBJECT, RDFS_RANGE, RDFS_RESOURCE),
    (RDFS_MEMBER, RDFS_RANGE, RDFS_RESOURCE),
    (RDF_FIRST, RDFS_RANGE, RDFS_RESOURCE),
    (RDF_REST, RDFS_RANGE, RDF_LIST),
    (RDFS_SEEALSO, RDFS_RANGE, RDFS_RESOURCE),
    (RDFS_ISDEFINEDBY, RDFS_RANGE, RDFS_RESOURCE),
    (RDFS_COMMENT, RDFS_RANGE, RDFS_LITERAL),
    (RDFS_LABEL, RDFS_RANGE, RDFS_LITERAL),
    (RDF_VALUE, RDFS_RANGE, RDFS_RESOURCE),
    // --- sub-class and sub-property ---
    (RDF_ALT, RDFS_SUBCLASSOF, RDFS_CONTAINER),
    (RDF_BAG, RDFS_SUBCLASSOF, RDFS_CONTAINER),
    (RDF_SEQ, RDFS_SUBCLASSOF, RDFS_CONTAINER),
    (
        RDFS_CONTAINERMEMBERSHIPPROPERTY,
        RDFS_SUBCLASSOF,
        RDF_PROPERTY,
    ),
    (RDFS_ISDEFINEDBY, RDFS_SUBPROPERTYOF, RDFS_SEEALSO),
    (RDFS_DATATYPE, RDFS_SUBCLASSOF, RDFS_CLASS),
];

/// The axiomatic triples `regime`'s lane asserts as premises, in specification order.
///
/// * `RDFS` asserts both tables. RDFS entailment subsumes RDF entailment, so the RDF
///   axioms hold too; the specification notes that all but one of them (`rdf:nil rdf:type
///   rdf:List .`) is redundant given the RDFS table, and asserting a redundant premise
///   costs one store insert and removes a case to reason about.
/// * `RDF` asserts NONE, and that is a statement rather than an omission: every RDF axiom
///   is a `rdf:type` triple, so the only conclusion the lane's single rule `rdfD2` can
///   draw from the whole table is `rdf:type rdf:type rdf:Property .` — an axiom of the
///   table itself. The axioms license nothing this lane does not already conclude, and
///   `the_rdf_axioms_license_nothing_the_rdf_lane_does_not_already_draw` checks that.
/// * `OWL-RL` asserts none either: OWL 2 Profiles §4.3 states that OWL 2 RL/RDF omits the
///   RDF and RDFS axiomatic triples, so asserting them would be a different calculus from
///   the one the lane's rule table names.
pub(crate) fn axioms_for(regime: crate::Regime) -> &'static [Axiom] {
    /// Both tables, spliced RDF-first, so the seeded order is the specification's.
    static RDF_THEN_RDFS: [Axiom; RDF_AXIOMS.len() + RDFS_AXIOMS.len()] = splice();
    /// No axioms — the lanes that assert none.
    static NONE: [Axiom; 0] = [];
    match regime {
        crate::Regime::Rdfs => &RDF_THEN_RDFS,
        crate::Regime::Simple
        | crate::Regime::Rdf
        | crate::Regime::OwlRl
        | crate::Regime::OwlDirect
        | crate::Regime::Rif
        | crate::Regime::D => &NONE,
    }
}

/// Concatenate the two tables, RDF first, at compile time.
const fn splice() -> [Axiom; RDF_AXIOMS.len() + RDFS_AXIOMS.len()] {
    let mut out = [RDF_AXIOMS[0]; RDF_AXIOMS.len() + RDFS_AXIOMS.len()];
    let mut written = 0;
    let mut i = 0;
    while i < RDF_AXIOMS.len() {
        out[written] = RDF_AXIOMS[i];
        written += 1;
        i += 1;
    }
    i = 0;
    while i < RDFS_AXIOMS.len() {
        out[written] = RDFS_AXIOMS[i];
        written += 1;
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{Axiom, RDF_AXIOMS, RDFS_AXIOMS, axioms_for};
    use crate::Regime;
    use crate::calculus::ALL_REGIMES;
    use std::collections::BTreeSet;

    /// The RDF prefix, for the transcription checks below.
    const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
    /// The RDFS prefix.
    const RDFS: &str = "http://www.w3.org/2000/01/rdf-schema#";

    /// Both tables are duplicate-free, and the two do not overlap.
    #[test]
    fn the_axiom_tables_are_duplicate_free_and_disjoint() {
        let rdf: BTreeSet<Axiom> = RDF_AXIOMS.into_iter().collect();
        let rdfs: BTreeSet<Axiom> = RDFS_AXIOMS.into_iter().collect();
        assert_eq!(
            rdf.len(),
            RDF_AXIOMS.len(),
            "the RDF table repeats a triple"
        );
        assert_eq!(
            rdfs.len(),
            RDFS_AXIOMS.len(),
            "the RDFS table repeats a triple"
        );
        assert!(
            rdf.is_disjoint(&rdfs),
            "a triple is transcribed into both tables"
        );
    }

    /// EVERY axiom names the RDF or RDFS vocabulary and nothing else.
    ///
    /// This is what makes "PurRDF mints no vocabulary" checkable for this table: an axiom
    /// whose subject, predicate or object left those two namespaces would be an invention,
    /// and the specification's tables contain no such triple.
    #[test]
    fn every_axiom_term_is_rdf_or_rdfs_vocabulary() {
        for axiom in RDF_AXIOMS.iter().chain(RDFS_AXIOMS.iter()) {
            for term in <[&str; 3]>::from(*axiom) {
                assert!(
                    term.starts_with(RDF) || term.starts_with(RDFS),
                    "{term} is not RDF or RDFS vocabulary"
                );
            }
        }
    }

    /// The finite tables stop exactly where the specification's ellipses begin: no
    /// container-membership property may appear in either of them.
    #[test]
    fn no_container_membership_property_is_transcribed() {
        for axiom in RDF_AXIOMS.iter().chain(RDFS_AXIOMS.iter()) {
            for term in <[&str; 3]>::from(*axiom) {
                assert!(
                    !term.starts_with(&format!("{RDF}_")),
                    "{term} is from the unbounded rdf:_n family, which no finite table may \
                     hold"
                );
            }
        }
    }

    /// The specification's own shape, pinned: the RDFS table is seventeen domain axioms,
    /// seventeen range axioms and six sub-class / sub-property axioms.
    #[test]
    fn the_rdfs_table_has_the_specified_shape() {
        let count = |predicate: &str| {
            RDFS_AXIOMS
                .iter()
                .filter(|&&(_, p, _)| p == predicate)
                .count()
        };
        assert_eq!(count(&format!("{RDFS}domain")), 17);
        assert_eq!(count(&format!("{RDFS}range")), 17);
        assert_eq!(count(&format!("{RDFS}subClassOf")), 5);
        assert_eq!(count(&format!("{RDFS}subPropertyOf")), 1);
        assert_eq!(RDFS_AXIOMS.len(), 40);
        // The RDF table is eight `rdf:Property` typings plus `rdf:nil rdf:type rdf:List`.
        assert_eq!(RDF_AXIOMS.len(), 9);
        assert_eq!(
            RDF_AXIOMS
                .iter()
                .filter(|&&(_, _, o)| o == format!("{RDF}Property"))
                .count(),
            8
        );
    }

    /// Only the `RDFS` lane asserts axioms, and it asserts both tables spliced RDF-first.
    #[test]
    fn only_the_rdfs_lane_asserts_axioms() {
        for regime in ALL_REGIMES {
            let asserted = axioms_for(regime);
            if matches!(regime, Regime::Rdfs) {
                assert_eq!(asserted.len(), RDF_AXIOMS.len() + RDFS_AXIOMS.len());
                assert_eq!(asserted[..RDF_AXIOMS.len()], RDF_AXIOMS);
                assert_eq!(asserted[RDF_AXIOMS.len()..], RDFS_AXIOMS);
            } else {
                assert!(asserted.is_empty(), "{regime:?} asserts axioms");
            }
        }
    }

    /// The claim the `RDF` lane's empty axiom list rests on: `rdfD2` — the lane's only
    /// rule — draws nothing from the RDF axiom table that the table does not already
    /// contain.
    ///
    /// `rdfD2` reads a triple's PREDICATE and types it `rdf:Property`. Every RDF axiom is
    /// a `rdf:type` triple, so the whole table yields the single conclusion
    /// `rdf:type rdf:type rdf:Property .`, which is the table's own first row. Seeding the
    /// table into that lane could therefore only move a triple from "derived, and emitted"
    /// to "premise, and not emitted", which is a smaller closure for no new entailment.
    #[test]
    fn the_rdf_axioms_license_nothing_the_rdf_lane_does_not_already_draw() {
        let concluded: BTreeSet<Axiom> = RDF_AXIOMS
            .iter()
            .map(|&(_, p, _)| (p, crate::vocab::RDF_TYPE, crate::vocab::RDF_PROPERTY))
            .collect();
        let table: BTreeSet<Axiom> = RDF_AXIOMS.into_iter().collect();
        assert!(
            concluded.is_subset(&table),
            "rdfD2 concludes {concluded:?} from the RDF axioms, which the table does not \
             already contain"
        );
    }
}
