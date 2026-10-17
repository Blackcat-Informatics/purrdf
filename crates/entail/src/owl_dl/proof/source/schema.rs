// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Borrowed views of native bound derivations already checked by clash replay.

use super::{Kb, ResolvedConcept, ResolvedRole, role};
use crate::owl_dl::bounds::{
    QualifierProof, SchemaBoundEvidence, SchemaRule, SchemaStep, SchemaUpperBound,
};

/// A checked normalized implication step. It does not identify original RDF axioms.
#[derive(Debug)]
pub struct ResolvedSchemaStep<'a> {
    /// The normalized concept established by this step.
    pub concept: ResolvedConcept<'a>,
    /// The rule checked against the consumer's own normalized schema.
    pub reason: SchemaReason<'a>,
}

/// A native implication license, with table identifiers resolved in one context.
#[derive(Debug)]
pub enum SchemaReason<'a> {
    /// Selected class or qualifier assumption, not a statement of its existence.
    Assume,
    /// Object domain's universal class.
    Top,
    /// Projection from this already established conjunction.
    Conjunct(ResolvedConcept<'a>),
    /// Formation of a conjunction from its previously established members.
    Conjunction,
    /// One native normalized inclusion. Source RDF provenance is a separate layer.
    Inclusion {
        /// Antecedent established by earlier steps.
        sub: ResolvedConcept<'a>,
        /// Consequent of the normalized inclusion.
        sup: ResolvedConcept<'a>,
    },
}

fn step<'a>(kb: &'a Kb, step: &SchemaStep) -> ResolvedSchemaStep<'a> {
    let reason = match step.rule {
        SchemaRule::Assume => SchemaReason::Assume,
        SchemaRule::Top => SchemaReason::Top,
        SchemaRule::Conjunct(parent) => SchemaReason::Conjunct(ResolvedConcept::of(kb, parent)),
        SchemaRule::Conjunction => SchemaReason::Conjunction,
        SchemaRule::Inclusion(index) => {
            let (sub, sup) = kb.tbox[index];
            SchemaReason::Inclusion {
                sub: ResolvedConcept::of(kb, sub),
                sup: ResolvedConcept::of(kb, sup),
            }
        }
    };
    ResolvedSchemaStep {
        concept: ResolvedConcept::of(kb, step.concept),
        reason,
    }
}

/// The upper side of a checked schema contradiction.
#[derive(Debug)]
pub enum SchemaUpper<'a> {
    /// An actual normalized maximum restriction.
    Restriction(ResolvedConcept<'a>),
    /// Exact concrete value-space extent, not a fabricated restriction.
    DataExtent(u64),
}

/// Why the lower qualifier is included in the upper qualifier.
#[derive(Debug)]
pub enum QualifierKind {
    /// Identical normalized qualifiers.
    Identity,
    /// The upper restriction counts every value of the role.
    Universal,
    /// Checked native class implication, available through qualifier_derivation.
    Object,
    /// Independently recomputed native XSD value-space containment.
    Data,
}

/// A schema clash's independently checked native bound derivation.
///
/// All class/role views borrow the same prepared consumer source. This does not
/// turn a normalized inclusion into an asserted RDF premise or explain current
/// membership; those are separate causal provenance obligations.
pub struct ResolvedSchema<'a> {
    pub(super) kb: &'a Kb,
    pub(super) bounds: &'a SchemaBoundEvidence,
}

impl<'a> ResolvedSchema<'a> {
    /// Class established empty; its current inhabitant belongs to the clash witness.
    #[must_use]
    pub fn class(&self) -> ResolvedConcept<'a> {
        ResolvedConcept::of(self.kb, self.bounds.class)
    }

    /// Actual normalized minimum restriction (an existential has minimum one).
    #[must_use]
    pub fn lower(&self) -> ResolvedConcept<'a> {
        ResolvedConcept::of(self.kb, self.bounds.lower)
    }

    /// Actual normalized maximum restriction or exact concrete extent.
    #[must_use]
    pub fn upper(&self) -> SchemaUpper<'a> {
        match self.bounds.upper {
            SchemaUpperBound::Restriction(id) => {
                SchemaUpper::Restriction(ResolvedConcept::of(self.kb, id))
            }
            SchemaUpperBound::DataExtent(extent) => SchemaUpper::DataExtent(extent),
        }
    }

    /// Source-context implication steps establishing the two restrictions.
    pub fn class_derivation(&self) -> impl ExactSizeIterator<Item = ResolvedSchemaStep<'a>> + '_ {
        self.bounds.class_steps.iter().map(|s| step(self.kb, s))
    }

    /// The exact qualifier license selected and checked by native bound replay.
    #[must_use]
    pub const fn qualifier(&self) -> QualifierKind {
        match self.bounds.qualifier {
            QualifierProof::Identity => QualifierKind::Identity,
            QualifierProof::Universal => QualifierKind::Universal,
            QualifierProof::Object(_) => QualifierKind::Object,
            QualifierProof::Data => QualifierKind::Data,
        }
    }

    /// Object qualifier steps; empty for the other explicitly named qualifier kinds.
    pub fn qualifier_derivation(
        &self,
    ) -> impl ExactSizeIterator<Item = ResolvedSchemaStep<'a>> + '_ {
        self.bounds
            .qualifier_derivation()
            .iter()
            .map(|s| step(self.kb, s))
    }

    /// Checked normalized role inclusion/inverse steps, not current ABox edges.
    pub fn roles(
        &self,
    ) -> impl ExactSizeIterator<Item = (ResolvedRole<'a>, ResolvedRole<'a>)> + '_ {
        self.bounds
            .roles
            .iter()
            .map(|s| (role(self.kb, s.from), role(self.kb, s.to)))
    }
}

impl std::fmt::Debug for ResolvedSchema<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolvedSchema")
            .field("class", &self.class())
            .field("lower", &self.lower())
            .field("upper", &self.upper())
            .field("qualifier", &self.qualifier())
            .finish_non_exhaustive()
    }
}
