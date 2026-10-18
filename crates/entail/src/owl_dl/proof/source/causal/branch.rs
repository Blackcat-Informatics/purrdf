// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original exhaustive branch licenses, resolved through the checked consumer vocabulary.

use super::super::{
    ResolvedConcept, ResolvedNode, ResolvedReserved, ResolvedRole, ResolvedTerm, node, proof_role,
    term,
};
use crate::owl_dl::{
    Kb,
    proof::{BranchOutcome, DlProof, ProofAlternative, ProofGround},
};

/// An original branch point whose complete alternatives were checked in the enclosing refutation.
#[derive(Clone, Copy)]
pub struct CausalBranch<'a> {
    proof: &'a DlProof,
    kb: &'a Kb,
    ordinal: usize,
}

impl<'a> CausalBranch<'a> {
    pub(super) const fn checked(proof: &'a DlProof, kb: &'a Kb, ordinal: usize) -> Self {
        Self { proof, kb, ordinal }
    }

    /// Original branch ordinal in this checked proof, not a source identifier.
    #[must_use]
    pub const fn ordinal(&self) -> usize {
        self.ordinal
    }

    /// Original clause ordinal in this proof's checked calculus/phase.
    #[must_use]
    pub fn clause(&self) -> usize {
        self.proof.branches[self.ordinal].clause
    }

    /// Actual original binding, resolving named terms and keeping anonymous identities proof-local.
    pub fn frame(&self) -> impl ExactSizeIterator<Item = ResolvedNode<'a>> + 'a + use<'a> {
        let kb = self.kb;
        self.proof.branches[self.ordinal]
            .frame
            .iter()
            .map(move |original| node(kb, original))
    }

    /// Clause-generated alternatives in their original checked order.
    pub fn alternatives(
        &self,
    ) -> impl ExactSizeIterator<Item = CausalAlternative<'a>> + 'a + use<'a> {
        let kb = self.kb;
        self.proof.branches[self.ordinal]
            .alternatives
            .iter()
            .map(move |alternative| CausalAlternative { kb, alternative })
    }

    /// Original nominal-introduction alternatives, separate from clause-generated alternatives.
    /// Their trust classification remains the enclosing native replay's classification.
    pub fn introduced(
        &self,
    ) -> impl ExactSizeIterator<Item = CausalAlternative<'a>> + 'a + use<'a> {
        let kb = self.kb;
        self.proof.branches[self.ordinal]
            .introduced
            .iter()
            .map(move |alternative| CausalAlternative { kb, alternative })
    }

    /// Original closure references for every alternative, bound by the enclosing checked proof.
    #[must_use]
    pub fn outcomes(&self) -> &'a [BranchOutcome] {
        &self.proof.branches[self.ordinal].outcomes
    }
}

impl std::fmt::Debug for CausalBranch<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CausalBranch")
            .field("ordinal", &self.ordinal)
            .field("clause", &self.clause())
            .field("outcomes", &self.outcomes())
            .finish()
    }
}

/// One original native alternative, read from its checked proof rather than regenerated.
#[derive(Clone, Copy)]
pub struct CausalAlternative<'a> {
    kb: &'a Kb,
    alternative: &'a ProofAlternative,
}

impl<'a> CausalAlternative<'a> {
    /// Every actual head atom, in original order and consumer vocabulary.
    pub fn head(&self) -> impl ExactSizeIterator<Item = CausalGround<'a>> + 'a + use<'a> {
        let kb = self.kb;
        self.alternative
            .atoms
            .iter()
            .map(move |atom| ground(kb, atom))
    }
}

impl std::fmt::Debug for CausalAlternative<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.head()).finish()
    }
}

/// Source-aware native branch head, preserving exact normalized and generated identities.
#[derive(Debug)]
pub enum CausalGround<'a> {
    /// Original normalized class assignment.
    Concept(ResolvedNode<'a>, ResolvedConcept<'a>),
    /// Original local self-edge assignment.
    SelfLoop(ResolvedNode<'a>, ResolvedRole<'a>),
    /// Original qualified minimum, preserving its bound, role and normalized filler.
    AtLeast(ResolvedNode<'a>, u32, ResolvedRole<'a>, ResolvedConcept<'a>),
    /// Original equality alternative between proof-local/native denotations.
    Equal(ResolvedNode<'a>, ResolvedNode<'a>),
    /// Original equality with the exact source/query term.
    EqualIndividual(ResolvedNode<'a>, ResolvedTerm<'a>),
    /// Original reserved nominal and its recursive native origin.
    EqualReserved(ResolvedNode<'a>, ResolvedReserved<'a>),
}

fn ground<'a>(kb: &'a Kb, atom: &'a ProofGround) -> CausalGround<'a> {
    match atom {
        ProofGround::Concept {
            node: original,
            concept,
        } => CausalGround::Concept(node(kb, original), ResolvedConcept::of(kb, *concept)),
        ProofGround::SelfLoop {
            node: original,
            role,
        } => CausalGround::SelfLoop(node(kb, original), proof_role(kb, *role)),
        ProofGround::AtLeast {
            node: original,
            n,
            role,
            filler,
        } => CausalGround::AtLeast(
            node(kb, original),
            *n,
            proof_role(kb, *role),
            ResolvedConcept::of(kb, *filler),
        ),
        ProofGround::Equal { left, right } => CausalGround::Equal(node(kb, left), node(kb, right)),
        ProofGround::EqualIndividual {
            node: original,
            individual,
        } => CausalGround::EqualIndividual(node(kb, original), term(kb, *individual)),
        ProofGround::EqualReserved {
            node: original,
            root,
        } => CausalGround::EqualReserved(node(kb, original), ResolvedReserved { kb, root }),
    }
}
