// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `scm-*` — the semantics of the schema vocabulary, OWL 2 Profiles §4.3 Table 9.
//!
//! This family holds the schema-vocabulary rules the `OWL-RL` lane fires and the RDFS
//! tables never had: `owl:equivalentClass` and `owl:equivalentProperty` read as mutual
//! `rdfs:subClassOf` / `rdfs:subPropertyOf`.
//!
//! `scm-sco` and `scm-spo` are NOT here. They are the OWL 2 RL names for `rdfs11` and
//! `rdfs5`, and one clause with two names is stated once, in [`super::rdfs`], where the
//! RDFS numbering orders it; [`super::ChaseRule::rule_id`] answers with the OWL name under
//! the `OWL-RL` lane.
//!
//! Both rules here take TWO clauses. The specification conclusion is a conjunction of two
//! triples, and a conjunctive head is not a Datalog clause, so each direction is stated
//! separately rather than encoded in a head form the evaluator refuses — which is what makes
//! a clause index not a rule index.
//!
//! The rules this family does not yet state are `scm-cls`, `scm-eqc2`, `scm-op`, `scm-dp`,
//! `scm-eqp2`, `scm-dom1`, `scm-dom2`, `scm-rng1`, `scm-rng2`, `scm-hv`, `scm-svf1`,
//! `scm-svf2`, `scm-avf1`, `scm-avf2` and `scm-int`.

use purrdf_datalog::clause::DlClause;

use super::{atom, var};
use crate::vocab::{
    OWL_EQUIVALENTCLASS, OWL_EQUIVALENTPROPERTY, RDFS_SUBCLASSOF, RDFS_SUBPROPERTYOF,
};

/// `scm-eqc1`: `owl:equivalentClass` ⇒ `rdfs:subClassOf`, both directions.
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

/// `scm-eqp1`: `owl:equivalentProperty` ⇒ `rdfs:subPropertyOf`, both directions.
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

/// The `scm-*` rules this chase states, in OWL 2 Profiles Table 9 order.
///
/// Handed to [`super::collect_families`], which asks each family in turn; see that macro
/// for the protocol and [`super::declare_chase_rules`] for what an entry means.
macro_rules! scm_rules {
    ($continue:ident, $rest:tt, $($rules:tt)*) => {
        $continue! { $rest $($rules)*
            /// `scm-eqc1` — `owl:equivalentClass` is mutual `rdfs:subClassOf`. `OWL-RL`
            /// only.
            EquivalentClass {
                id: ScmEqc1,
                lanes: [OwlRl],
                clauses: scm::equivalent_class,
            },
            /// `scm-eqp1` — `owl:equivalentProperty` is mutual `rdfs:subPropertyOf`.
            /// `OWL-RL` only.
            EquivalentProperty {
                id: ScmEqp1,
                lanes: [OwlRl],
                clauses: scm::equivalent_property,
            },
        }
    };
}

pub(crate) use scm_rules;
