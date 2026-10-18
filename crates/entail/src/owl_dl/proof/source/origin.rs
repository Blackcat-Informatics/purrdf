// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Consumer-owned normalization anchors, not a full syntax or causal read trace.

use super::{Kb, ResolvedTerm, term};
use crate::owl_dl::axiom_source::AxiomSource;

/// What originally introduced a normalized inclusion in the consumer's schema.
///
/// An anchor identifies the source operation. Nested expression syntax and
/// actual current premises remain separate provenance obligations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedAxiomSource<'a> {
    /// No source provenance, as for a synthetic programmatically assembled schema.
    Unrecorded,
    /// An exact authored default-graph triple passed to the original reverse mapper.
    Triple {
        /// Original subject, including scoped-blank expression identity.
        subject: ResolvedTerm<'a>,
        /// Original axiom predicate.
        predicate: ResolvedTerm<'a>,
        /// Original object term; never replaced with a normalized concept ID.
        object: ResolvedTerm<'a>,
    },
    /// Node-authored n-ary axiom or negative property assertion.
    Node {
        /// Original source node.
        node: ResolvedTerm<'a>,
        /// Actual rdf:type trigger; absent for vocabulary-identified negative assertions.
        kind: Option<ResolvedTerm<'a>>,
    },
    /// Native defining equation rooted at this source expression cycle cut point.
    RecursiveDefinition(ResolvedTerm<'a>),
    /// Query-written defining equation, not an asserted source axiom.
    QueryDefinition(ResolvedTerm<'a>),
    /// Fixed empty extension of the standard bottom property; not authored RDF.
    EmptyRole(ResolvedTerm<'a>),
}

pub(super) fn resolve(kb: &Kb, source: AxiomSource) -> ResolvedAxiomSource<'_> {
    match source {
        AxiomSource::Unrecorded => ResolvedAxiomSource::Unrecorded,
        AxiomSource::Triple(s, p, o) => ResolvedAxiomSource::Triple {
            subject: term(kb, s),
            predicate: term(kb, p),
            object: term(kb, o),
        },
        AxiomSource::Node { node, kind } => ResolvedAxiomSource::Node {
            node: term(kb, node),
            kind: kind.map(|id| term(kb, id)),
        },
        AxiomSource::RecursiveDefinition(node) => {
            ResolvedAxiomSource::RecursiveDefinition(term(kb, node))
        }
        AxiomSource::QueryDefinition(node) => ResolvedAxiomSource::QueryDefinition(term(kb, node)),
        AxiomSource::EmptyRole(property) => ResolvedAxiomSource::EmptyRole(term(kb, property)),
    }
}
