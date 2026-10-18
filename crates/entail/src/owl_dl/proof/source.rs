// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Borrowed source-aware views of checked native clash evidence.
//!
//! These views resolve the native normalized vocabulary, not the original RDF
//! expression syntax or a causal dependency graph. A checked witness can depend
//! on other nodes; its identity alone never licenses local contradiction.
//! Handles borrow the consumer context and expose no bare-id constructor.

use super::{ClashReplay, ClashStep, DlProof, DlProofContext, DlProofError, NodeRef, ProofFact};
use crate::owl_dl::Kb;
use crate::owl_dl::concept::{Concept, Role};
use purrdf_core::TermValue;
use purrdf_xsd::range::DataRange;

pub mod causal;
mod origin;
mod schema;
pub use origin::ResolvedAxiomSource;
pub use schema::{QualifierKind, ResolvedSchema, ResolvedSchemaStep, SchemaReason, SchemaUpper};

/// An exact RDF term and whether it existed before query preparation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedTerm<'a> {
    /// Consumer source vocabulary, retaining blank scope and literal metadata.
    /// Includes standard vocabulary interned by the native parser; this is not
    /// a claim that the term occurs in an asserted premise.
    Source(&'a TermValue),
    /// A query/refutation symbol; never an asserted source individual.
    QuerySymbol(&'a TermValue),
}

fn term(kb: &Kb, id: u32) -> ResolvedTerm<'_> {
    let value = kb.interner.value(id);
    if id < kb.source_terms {
        ResolvedTerm::Source(value)
    } else {
        ResolvedTerm::QuerySymbol(value)
    }
}

/// A native role resolved against its consumer term table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedRole<'a> {
    /// Property term, without an invented inverse-property IRI.
    pub property: ResolvedTerm<'a>,
    /// Whether the property is read in the inverse direction.
    pub inverse: bool,
}

fn role(kb: &Kb, role: Role) -> ResolvedRole<'_> {
    let (property, inverse) = match role {
        Role::Named(property) => (property, false),
        Role::Inv(property) => (property, true),
    };
    ResolvedRole {
        property: term(kb, property),
        inverse,
    }
}

/// A borrowed normalized native concept, including internal calculus atoms.
#[derive(Clone, Copy)]
pub struct ResolvedConcept<'a> {
    kb: &'a Kb,
    concept: &'a Concept,
}

/// The structure of a normalized concept; not an asserted RDF expression node.
#[derive(Debug)]
pub enum ConceptKind<'a> {
    /// Universal object-domain class.
    Top,
    /// Empty class.
    Bottom,
    /// Class term in the consumer source or prepared query.
    Named(ResolvedTerm<'a>),
    /// Classical complement.
    Not(ResolvedConcept<'a>),
    /// Canonically ordered conjunction members.
    And(ConceptMembers<'a>),
    /// Canonically ordered disjunction members.
    Or(ConceptMembers<'a>),
    /// Existential role restriction.
    Some(ResolvedRole<'a>, ResolvedConcept<'a>),
    /// Universal role restriction.
    All(ResolvedRole<'a>, ResolvedConcept<'a>),
    /// Qualified lower cardinality bound.
    Min(u32, ResolvedRole<'a>, ResolvedConcept<'a>),
    /// Qualified upper cardinality bound.
    Max(u32, ResolvedRole<'a>, ResolvedConcept<'a>),
    /// Canonically ordered exact individual terms, without a unique-name assumption.
    Nominal(NominalMembers<'a>),
    /// Original native datatype-range AST, owned by the XSD range implementation.
    Data(&'a DataRange),
    /// Local reflexivity restriction.
    SelfRestriction(ResolvedRole<'a>),
    /// Native automaton obligation; its machine/state are context-local, not RDF terms.
    RoleState {
        /// Machine ordinal within this consumer's compiled role program.
        machine: usize,
        /// Local automaton state.
        state: usize,
        /// Original universal filler.
        filler: ResolvedConcept<'a>,
        /// Whether this atom is the classical complement.
        negated: bool,
    },
}

impl<'a> ResolvedConcept<'a> {
    fn of(kb: &'a Kb, id: u32) -> Self {
        Self {
            kb,
            concept: kb.table.concept(id),
        }
    }

    fn child(self, concept: &'a Concept) -> Self {
        Self { concept, ..self }
    }

    /// Inspect one constructor without cloning or recursively allocating its tree.
    #[must_use]
    pub fn kind(self) -> ConceptKind<'a> {
        match self.concept {
            Concept::Top => ConceptKind::Top,
            Concept::Bottom => ConceptKind::Bottom,
            Concept::Named(id) => ConceptKind::Named(term(self.kb, *id)),
            Concept::Not(child) => ConceptKind::Not(self.child(child)),
            Concept::And(children) => ConceptKind::And(ConceptMembers {
                kb: self.kb,
                children: children.iter(),
            }),
            Concept::Or(children) => ConceptKind::Or(ConceptMembers {
                kb: self.kb,
                children: children.iter(),
            }),
            Concept::Some(r, child) => ConceptKind::Some(role(self.kb, *r), self.child(child)),
            Concept::All(r, child) => ConceptKind::All(role(self.kb, *r), self.child(child)),
            Concept::Min(n, r, child) => ConceptKind::Min(*n, role(self.kb, *r), self.child(child)),
            Concept::Max(n, r, child) => ConceptKind::Max(*n, role(self.kb, *r), self.child(child)),
            Concept::Nominal(ids) => ConceptKind::Nominal(NominalMembers {
                kb: self.kb,
                ids: ids.iter(),
            }),
            Concept::Data(id) => ConceptKind::Data(self.kb.data_ranges.range(*id)),
            Concept::SelfRestriction(r) => ConceptKind::SelfRestriction(role(self.kb, *r)),
            Concept::RoleState {
                machine,
                state,
                filler,
                negated,
            } => ConceptKind::RoleState {
                machine: *machine,
                state: *state,
                filler: Self::of(self.kb, *filler),
                negated: *negated,
            },
        }
    }
}

/// Allocation-free traversal of normalized conjunction/disjunction members.
#[derive(Clone)]
pub struct ConceptMembers<'a> {
    kb: &'a Kb,
    children: std::slice::Iter<'a, Concept>,
}

impl<'a> Iterator for ConceptMembers<'a> {
    type Item = ResolvedConcept<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.children.next().map(|concept| ResolvedConcept {
            kb: self.kb,
            concept,
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.children.size_hint()
    }
}

impl ExactSizeIterator for ConceptMembers<'_> {}
impl std::iter::FusedIterator for ConceptMembers<'_> {}

/// Allocation-free traversal of an exact native nominal set.
#[derive(Clone)]
pub struct NominalMembers<'a> {
    kb: &'a Kb,
    ids: std::slice::Iter<'a, u32>,
}

impl<'a> Iterator for NominalMembers<'a> {
    type Item = ResolvedTerm<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.ids.next().map(|id| term(self.kb, *id))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.ids.size_hint()
    }
}

impl ExactSizeIterator for NominalMembers<'_> {}
impl std::iter::FusedIterator for NominalMembers<'_> {}

/// A node of this checked proof, resolved without inventing a source name.
#[derive(Debug)]
pub enum ResolvedNode<'a> {
    /// Original or explicitly distinguished query term.
    Individual(ResolvedTerm<'a>),
    /// Proof-local existential identity. The ordinal has no cross-proof meaning.
    Anonymous(u32),
    /// Reserved nominal introduced by native at-most reasoning.
    Reserved(ResolvedReserved<'a>),
}

/// A borrowed reserved-root descriptor retaining its original recursive origin.
pub struct ResolvedReserved<'a> {
    kb: &'a Kb,
    root: &'a super::ReservedRef,
}

impl<'a> ResolvedReserved<'a> {
    /// Root whose at-most restriction introduced this reserved nominal.
    #[must_use]
    pub fn origin(&self) -> ResolvedNode<'a> {
        node(self.kb, &self.root.origin)
    }

    /// Counted role, preserving inverse direction.
    #[must_use]
    pub fn role(&self) -> ResolvedRole<'a> {
        proof_role(self.kb, self.root.role)
    }

    /// Normalized filler of the at-most restriction.
    #[must_use]
    pub fn filler(&self) -> ResolvedConcept<'a> {
        ResolvedConcept::of(self.kb, self.root.filler)
    }

    /// Native ordinal within this root's reserved set, not a source term.
    #[must_use]
    pub const fn index(&self) -> u32 {
        self.root.index
    }
}

fn node<'a>(kb: &'a Kb, node: &'a NodeRef) -> ResolvedNode<'a> {
    match node {
        NodeRef::Individual(id) => ResolvedNode::Individual(term(kb, *id)),
        NodeRef::Anonymous(id) => ResolvedNode::Anonymous(*id),
        NodeRef::Reserved(root) => ResolvedNode::Reserved(ResolvedReserved { kb, root }),
    }
}

fn proof_role(kb: &Kb, r: super::ProofRole) -> ResolvedRole<'_> {
    ResolvedRole {
        property: term(kb, r.property),
        inverse: r.inverse,
    }
}

/// One checked terminal witness atom, with its consumer vocabulary resolved.
#[derive(Debug)]
pub enum ResolvedFact<'a> {
    /// Normalized class membership.
    Concept(ResolvedNode<'a>, ResolvedConcept<'a>),
    /// Role reachability; this view alone does not describe its path licenses.
    Edge(ResolvedNode<'a>, ResolvedRole<'a>, ResolvedNode<'a>),
    /// A node denotes an exact individual term.
    Denotes(ResolvedNode<'a>, ResolvedTerm<'a>),
}

/// Successfully replayed terminal witness borrowing its proof and consumer source.
///
/// This is not a causal support graph or a local contradiction license.
pub struct ResolvedClash<'a> {
    replay: ClashReplay,
    step: &'a ClashStep,
    kb: &'a Kb,
}

impl ResolvedClash<'_> {
    /// The existing native replay report, including its explicit trust boundary.
    #[must_use]
    pub const fn replay(&self) -> &ClashReplay {
        &self.replay
    }

    /// Checked native bound derivation, when this is a prepared schema clash.
    /// Its normalized inclusions are not original asserted RDF syntax.
    #[must_use]
    pub fn schema(&self) -> Option<ResolvedSchema<'_>> {
        self.step.schema.as_ref().map(|evidence| ResolvedSchema {
            kb: self.kb,
            bounds: &evidence.bounds,
        })
    }

    /// Resolve terminal witness atoms lazily without copying the prepared source.
    pub fn witness(&self) -> impl ExactSizeIterator<Item = ResolvedFact<'_>> + '_ {
        self.step.witness.iter().map(|fact| match fact {
            ProofFact::Concept { node: n, concept } => {
                ResolvedFact::Concept(node(self.kb, n), ResolvedConcept::of(self.kb, *concept))
            }
            ProofFact::Edge { from, to, role } => ResolvedFact::Edge(
                node(self.kb, from),
                proof_role(self.kb, *role),
                node(self.kb, to),
            ),
            ProofFact::Denotes {
                node: n,
                individual,
            } => ResolvedFact::Denotes(node(self.kb, n), term(self.kb, *individual)),
        })
    }
}

impl DlProof {
    /// Replay current support against the consumer source, then resolve its witness.
    ///
    /// Missing legacy current support is refused, even if structural clause replay
    /// succeeds. This does not infer source premises from a terminal node identity.
    pub fn resolve_clash<'a>(
        &'a self,
        index: usize,
        ctx: &'a DlProofContext<'_>,
    ) -> Result<ResolvedClash<'a>, DlProofError> {
        let replay = self.replay_clash(index, ctx)?;
        if !replay.current_supported() {
            return Err(super::malformed(
                "source resolution requires checked current support",
            ));
        }
        Ok(ResolvedClash {
            replay,
            step: &self.clashes[index],
            kb: &ctx.kb,
        })
    }
}

impl DlProofContext<'_> {
    /// Inspect an absorbed clause's original TBox source in this prepared context.
    ///
    /// This is normalization metadata, not proof or current causal support. Structural
    /// and RBox clauses have no TBox origin. Prepared schema guards can use several
    /// inclusions; their checked derivations are available through ResolvedClash::schema.
    pub fn clause_tbox_origin(
        &self,
        index: usize,
    ) -> Result<Option<ResolvedAxiomSource<'_>>, DlProofError> {
        if index >= self.clauses.count() {
            return Err(DlProofError::UnknownClause {
                clause: index,
                clauses: self.clauses.count(),
            });
        }
        Ok(self
            .clauses
            .tbox_source(index)
            .map(|source| origin::resolve(&self.kb, self.kb.tbox_sources[source])))
    }

    /// Inspect internalized schema formulas and their original source operations.
    ///
    /// These quantify over the object domain; they are not asserted memberships or a
    /// selected clash's causal premises. Output borrows the prepared source directly.
    pub fn internalized_axioms(
        &self,
    ) -> impl ExactSizeIterator<Item = (ResolvedConcept<'_>, ResolvedAxiomSource<'_>)> + '_ {
        self.kb.meta.iter().enumerate().map(|(ordinal, &concept)| {
            (
                ResolvedConcept::of(&self.kb, concept),
                origin::resolve(
                    &self.kb,
                    self.kb.tbox_sources[self.kb.meta_sources[ordinal]],
                ),
            )
        })
    }
}

impl std::fmt::Debug for ResolvedConcept<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.kind().fmt(f)
    }
}

impl std::fmt::Debug for ConceptMembers<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

impl std::fmt::Debug for NominalMembers<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

impl std::fmt::Debug for ResolvedReserved<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolvedReserved")
            .field("origin", &self.origin())
            .field("role", &self.role())
            .field("filler", &self.filler())
            .field("index", &self.index())
            .finish()
    }
}

impl std::fmt::Debug for ResolvedClash<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolvedClash")
            .field("replay", &self.replay)
            .field("witness", &self.witness().len())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod normalization_tests;
#[cfg(test)]
mod tests;
