// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `prp-*` — the semantics of axioms about properties, OWL 2 Profiles §4.3 Table 5.
//!
//! This family holds the property-axiom rules the `OWL-RL` lane fires and the RDFS tables
//! never had: the property characteristics (`owl:SymmetricProperty`,
//! `owl:TransitiveProperty`) and `owl:inverseOf`.
//!
//! `prp-dom`, `prp-rng` and `prp-spo1` are NOT here. They are the OWL 2 RL names for
//! `rdfs2`, `rdfs3` and `rdfs7`, and one clause with two names is stated once, in
//! [`super::rdfs`], where the RDFS numbering orders it; [`super::ChaseRule::rule_id`]
//! answers with the OWL name under the `OWL-RL` lane.
//!
//! The rules this family does not yet state are `prp-ap`, `prp-fp`, `prp-ifp`, `prp-irp`,
//! `prp-asyp`, `prp-spo2`, `prp-eqp1`, `prp-eqp2`, `prp-pdw`, `prp-adp`, `prp-key`,
//! `prp-npa1` and `prp-npa2`.

use purrdf_datalog::clause::DlClause;

use super::{atom, iri, quad, var};
use crate::vocab::{OWL_INVERSEOF, OWL_SYMMETRICPROPERTY, OWL_TRANSITIVEPROPERTY, RDF_TYPE};

/// `prp-symp`: `?p rdf:type owl:SymmetricProperty`, `T(?x, ?p, ?y)` ⇒ `T(?y, ?p, ?x)`.
pub(super) fn symmetric() -> Vec<DlClause> {
    vec![DlClause::datalog(
        quad(var("?y"), var("?p"), var("?x")),
        vec![
            atom(var("?p"), RDF_TYPE, iri(OWL_SYMMETRICPROPERTY)),
            quad(var("?x"), var("?p"), var("?y")),
        ],
    )]
}

/// `prp-trp`: `?p rdf:type owl:TransitiveProperty`, `T(?x, ?p, ?y)`, `T(?y, ?p, ?z)` ⇒
/// `T(?x, ?p, ?z)`.
pub(super) fn transitive() -> Vec<DlClause> {
    vec![DlClause::datalog(
        quad(var("?x"), var("?p"), var("?z")),
        vec![
            atom(var("?p"), RDF_TYPE, iri(OWL_TRANSITIVEPROPERTY)),
            quad(var("?x"), var("?p"), var("?y")),
            quad(var("?y"), var("?p"), var("?z")),
        ],
    )]
}

/// `prp-inv1`: `?p1 owl:inverseOf ?p2`, `T(?x, ?p1, ?y)` ⇒ `T(?y, ?p2, ?x)`.
pub(super) fn inverse1() -> Vec<DlClause> {
    vec![DlClause::datalog(
        quad(var("?y"), var("?p2"), var("?x")),
        vec![
            atom(var("?p1"), OWL_INVERSEOF, var("?p2")),
            quad(var("?x"), var("?p1"), var("?y")),
        ],
    )]
}

/// `prp-inv2`: `?p1 owl:inverseOf ?p2`, `T(?x, ?p2, ?y)` ⇒ `T(?y, ?p1, ?x)`.
pub(super) fn inverse2() -> Vec<DlClause> {
    vec![DlClause::datalog(
        quad(var("?y"), var("?p1"), var("?x")),
        vec![
            atom(var("?p1"), OWL_INVERSEOF, var("?p2")),
            quad(var("?x"), var("?p2"), var("?y")),
        ],
    )]
}

/// The `prp-*` rules this chase states, in OWL 2 Profiles Table 5 order.
///
/// Handed to [`super::collect_families`], which asks each family in turn; see that macro
/// for the protocol and [`super::declare_chase_rules`] for what an entry means.
macro_rules! prp_rules {
    ($continue:ident, $rest:tt, $($rules:tt)*) => {
        $continue! { $rest $($rules)*
            /// `prp-symp` — a symmetric property mirrors its triples. `OWL-RL` only.
            Symmetric {
                id: PrpSymp,
                lanes: [OwlRl],
                clauses: prp::symmetric,
            },
            /// `prp-trp` — a transitive property composes its triples. `OWL-RL` only.
            Transitive {
                id: PrpTrp,
                lanes: [OwlRl],
                clauses: prp::transitive,
            },
            /// `prp-inv1` — an `owl:inverseOf` assertion, read left to right. `OWL-RL`
            /// only.
            Inverse1 {
                id: PrpInv1,
                lanes: [OwlRl],
                clauses: prp::inverse1,
            },
            /// `prp-inv2` — an `owl:inverseOf` assertion, read right to left. `OWL-RL`
            /// only.
            Inverse2 {
                id: PrpInv2,
                lanes: [OwlRl],
                clauses: prp::inverse2,
            },
        }
    };
}

pub(crate) use prp_rules;
