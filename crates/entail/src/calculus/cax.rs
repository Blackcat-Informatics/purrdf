// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `cax-*` — the semantics of class axioms, OWL 2 Profiles §4.3 Table 7.
//!
//! Four of the table's five rules live here. `cax-sco` does not: it is the OWL 2 RL name
//! for `rdfs9`, and one clause with two names is stated once, in [`super::rdfs`], where the
//! RDFS numbering orders it. [`super::ChaseRule::rule_id`] answers with `cax-sco` under the
//! `OWL-RL` lane, so an `OWL-RL` report does name the rule — this module simply is not
//! where it is written.
//!
//! # `cax-eqc1` and `cax-eqc2` are not redundant, they are one hop shorter
//!
//! Both conclusions also follow from `scm-eqc1` (which reads `owl:equivalentClass` as
//! mutual `rdfs:subClassOf`) and then `cax-sco`. Stating them anyway is what the
//! specification's table says, and it is observable rather than cosmetic: `scm-eqc1` has to
//! fire first, so the two-hop path reaches the conclusion one round LATER, and a report
//! credits the rule that was first to add the triple. Omitting them would move every such
//! conclusion's attribution onto `cax-sco` and make an `OWL-RL` report say a rule fired
//! that the caller's ontology never used.
//!
//! # The two rules stated here that conclude `false`
//!
//! `cax-dw` and `cax-adc` conclude `false`: a body match is an INCONSISTENCY WITNESS, not a
//! triple. Both are stated with the specification's own bodies and declared with
//! `concludes: Inconsistency,`, and [`super::constraint_clause`] lowers each into the
//! clause the evaluator runs — see the [calculus docs](super) for why the lowering
//! fabricates nothing.

use purrdf_datalog::clause::DlClause;

use super::{atom, internal, internal_graph, iri, var};
use crate::lists::{INDEX_DISTINCT_RELATION, LIST_RELATION};
use crate::vocab::{
    OWL_ALLDISJOINTCLASSES, OWL_DISJOINTWITH, OWL_EQUIVALENTCLASS, OWL_MEMBERS, RDF_TYPE,
};

/// `cax-eqc1`: `?c1 owl:equivalentClass ?c2`, `?x rdf:type ?c1` ⇒ `?x rdf:type ?c2`.
pub(super) fn equivalent_class_instance_left() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?x"), RDF_TYPE, var("?c2")),
        vec![
            atom(var("?c1"), OWL_EQUIVALENTCLASS, var("?c2")),
            atom(var("?x"), RDF_TYPE, var("?c1")),
        ],
    )]
}

/// `cax-eqc2`: `?c1 owl:equivalentClass ?c2`, `?x rdf:type ?c2` ⇒ `?x rdf:type ?c1`.
pub(super) fn equivalent_class_instance_right() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?x"), RDF_TYPE, var("?c1")),
        vec![
            atom(var("?c1"), OWL_EQUIVALENTCLASS, var("?c2")),
            atom(var("?x"), RDF_TYPE, var("?c2")),
        ],
    )]
}

/// `cax-dw`: `?c1 owl:disjointWith ?c2`, `?x rdf:type ?c1`, `?x rdf:type ?c2` ⇒ `false`.
pub(super) fn disjoint_with() -> Vec<DlClause> {
    vec![DlClause::inconsistency(vec![
        atom(var("?c1"), OWL_DISJOINTWITH, var("?c2")),
        atom(var("?x"), RDF_TYPE, var("?c1")),
        atom(var("?x"), RDF_TYPE, var("?c2")),
    ])]
}

/// `cax-adc`: `?x rdf:type owl:AllDisjointClasses`, `?x owl:members ?y` over the list
/// `?c1 … ?cn`, `?z rdf:type ?ci`, `?z rdf:type ?cj` with `i ≠ j` ⇒ `false`.
///
/// The two members come from [`crate::lists`]'s `LIST(head, index, member)` and `i ≠ j` is
/// `INDEX_DISTINCT(?i, ?j)` over the index pairs the pre-pass materializes.
/// Dropping that condition would let `i = j` match and make a single class assertion an
/// inconsistency, which is unsound — so it is expressed even though the `false` head means
/// no evaluator runs the clause today.
pub(super) fn all_disjoint_classes() -> Vec<DlClause> {
    vec![DlClause::inconsistency(vec![
        atom(var("?x"), RDF_TYPE, iri(OWL_ALLDISJOINTCLASSES)),
        atom(var("?x"), OWL_MEMBERS, var("?y")),
        internal(LIST_RELATION, var("?y"), var("?ci"), var("?i")),
        internal(LIST_RELATION, var("?y"), var("?cj"), var("?j")),
        internal(
            INDEX_DISTINCT_RELATION,
            var("?i"),
            var("?j"),
            internal_graph(),
        ),
        atom(var("?z"), RDF_TYPE, var("?ci")),
        atom(var("?z"), RDF_TYPE, var("?cj")),
    ])]
}

/// The `cax-*` rules this chase states here, in OWL 2 Profiles Table 7 order.
///
/// Handed to [`super::collect_families`], which asks each family in turn; see that macro
/// for the protocol and [`super::declare_chase_rules`] for what an entry means.
macro_rules! cax_rules {
    ($continue:ident, $rest:tt, $($rules:tt)*) => {
        $continue! { $rest $($rules)*
            /// `cax-eqc1` — an `owl:equivalentClass` assertion re-types an instance, left
            /// to right. `OWL-RL` only.
            EquivalentClassInstanceLeft {
                id: CaxEqc1,
                lanes: [OwlRl],
                clauses: cax::equivalent_class_instance_left,
            },
            /// `cax-eqc2` — the same, right to left. `OWL-RL` only.
            EquivalentClassInstanceRight {
                id: CaxEqc2,
                lanes: [OwlRl],
                clauses: cax::equivalent_class_instance_right,
            },
            /// `cax-dw` — two disjoint classes with a shared instance is an
            /// inconsistency. DECLARED, not evaluated: the head is `false`.
            DisjointWith {
                id: CaxDw,
                lanes: [OwlRl],
                clauses: cax::disjoint_with,
                concludes: Inconsistency,
            },
            /// `cax-adc` — the same, over an `owl:AllDisjointClasses` list. DECLARED, not
            /// evaluated: the head is `false`.
            AllDisjointClasses {
                id: CaxAdc,
                lanes: [OwlRl],
                clauses: cax::all_disjoint_classes,
                concludes: Inconsistency,
            },
        }
    };
}

pub(crate) use cax_rules;
