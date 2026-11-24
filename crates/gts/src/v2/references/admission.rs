// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use crate::v2::fields::push;
use crate::v2::rdf::RowScope;
use crate::v2::rdf::dictionary::ByReferenceTerms;
use crate::v2::{FoldError, RdfFold, SegmentDictionary, Term, TermId};

use super::{Binding, Digest, Leaf, LeafNode, Suppression};

/// The term decisions available at the position of a reference frame.
pub(in crate::v2) struct LocalTerms<'a> {
    dictionary: &'a SegmentDictionary,
    scope: RowScope,
    fold: &'a RdfFold,
}

#[derive(Debug)]
pub(in crate::v2) enum EntryAdmission<T> {
    Accepted(T),
    HeldUntilCommit(T),
    ForwardReference,
    OpaqueTerm,
}

/// Local term IDs are replaced before the segment dictionary goes out of scope.
#[derive(Debug)]
pub(in crate::v2) struct ScopedLeaf(pub(in crate::v2) Vec<ScopedNode>);

#[derive(Debug)]
pub(in crate::v2) enum ScopedNode {
    Term(TermId),
    Blank { frame: Digest, label: Vec<u8> },
    Triple,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Position {
    Subject,
    Predicate,
    Object,
}

impl Position {
    const fn next(self) -> Self {
        match self {
            Self::Subject => Self::Predicate,
            Self::Predicate | Self::Object => Self::Object,
        }
    }

    const fn admits(self, term: &Term<'_>) -> bool {
        match self {
            Self::Subject => matches!(term, Term::Iri(_) | Term::Blank { .. }),
            Self::Predicate => matches!(term, Term::Iri(_)),
            Self::Object => true,
        }
    }
}

impl<'a> LocalTerms<'a> {
    pub(in crate::v2) const fn new(
        dictionary: &'a SegmentDictionary,
        scope: RowScope,
        fold: &'a RdfFold,
    ) -> Self {
        Self {
            dictionary,
            scope,
            fold,
        }
    }

    pub(in crate::v2) fn binding(
        &self,
        binding: Binding<Leaf<'_>>,
    ) -> Result<EntryAdmission<Binding<ScopedLeaf>>, FoldError> {
        self.admit([Some(binding.term), None, None, None], None, |_| {
            binding.try_map(|leaf| self.materialize(leaf))
        })
    }

    pub(in crate::v2) fn suppression(
        &self,
        suppression: Suppression<Leaf<'_>>,
    ) -> Result<EntryAdmission<Suppression<ScopedLeaf>>, FoldError> {
        self.admit(Self::suppression_leaves(suppression), None, |_| {
            suppression.try_map(|leaf| self.materialize(leaf))
        })
    }

    pub(in crate::v2) fn suppression_with_references(
        &self,
        suppression: Suppression<Leaf<'_>>,
        references: &ByReferenceTerms,
    ) -> Result<EntryAdmission<Option<Suppression<ScopedLeaf>>>, FoldError> {
        self.admit(
            Self::suppression_leaves(suppression),
            Some(references),
            |by_reference| {
                if by_reference {
                    Ok(None)
                } else {
                    suppression.try_map(|leaf| self.materialize(leaf)).map(Some)
                }
            },
        )
    }

    fn suppression_leaves(suppression: Suppression<Leaf<'_>>) -> [Option<Leaf<'_>>; 4] {
        match suppression {
            Suppression::Term(term) | Suppression::Reifier(term) | Suppression::Graph(term) => {
                [Some(term), None, None, None]
            }
            Suppression::Quad([s, p, o], graph) | Suppression::Annotation([s, p, o], graph) => {
                [Some(s), Some(p), Some(o), graph]
            }
            Suppression::Blob(_) => [None; 4],
            Suppression::ReifierBinding([r, t], graph) => [Some(r), Some(t), graph, None],
            Suppression::Binding(binding) => [Some(binding.term), None, None, None],
        }
    }

    fn admit<T>(
        &self,
        leaves: [Option<Leaf<'_>>; 4],
        references: Option<&ByReferenceTerms>,
        materialize: impl FnOnce(bool) -> Result<T, FoldError>,
    ) -> Result<EntryAdmission<T>, FoldError> {
        let mut held = false;
        let mut needs_commit = false;
        let mut by_reference = false;
        let below_commit = self.scope.below_commit();
        for leaf in leaves.into_iter().flatten() {
            let mut position = Position::Object;
            for node in leaf.nodes() {
                match node {
                    LeafNode::Term(id) => {
                        if !self.scope.defines(id) {
                            return Ok(EntryAdmission::ForwardReference);
                        }
                        if let Some(term) = self.dictionary.readable_term(id)
                            && below_commit.admits(id)
                        {
                            let term = self.fold.term(term).ok_or(FoldError::TermHandle(term))?;
                            held |= !position.admits(term);
                            needs_commit |= !self.scope.admits(id);
                        } else if references
                            .is_some_and(|terms| terms.get(id, below_commit).is_some())
                        {
                            held |= position != Position::Object;
                            needs_commit |= !self.scope.admits(id);
                            by_reference = true;
                        } else {
                            held = true;
                        }
                    }
                    LeafNode::Blank { .. } => held |= position == Position::Predicate,
                    LeafNode::Triple => {
                        held |= position != Position::Object;
                        // In a readable leaf, nesting occurs only in the object.
                        // Once a position fails we continue looking for undefined IDs.
                        position = Position::Subject;
                        continue;
                    }
                }
                position = position.next();
            }
        }
        if held {
            Ok(EntryAdmission::OpaqueTerm)
        } else if needs_commit {
            materialize(by_reference).map(EntryAdmission::HeldUntilCommit)
        } else {
            materialize(by_reference).map(EntryAdmission::Accepted)
        }
    }

    fn materialize(&self, leaf: Leaf<'_>) -> Result<ScopedLeaf, FoldError> {
        let mut nodes = Vec::new();
        for node in leaf.nodes() {
            let node = match node {
                LeafNode::Term(id) => ScopedNode::Term(
                    self.dictionary
                        .readable_term(id)
                        .expect("local term admission succeeded"),
                ),
                LeafNode::Blank { frame, label } => {
                    let mut owned = Vec::new();
                    owned.try_reserve_exact(label.len())?;
                    owned.extend_from_slice(label);
                    ScopedNode::Blank {
                        frame: *frame,
                        label: owned,
                    }
                }
                LeafNode::Triple => ScopedNode::Triple,
            };
            push(&mut nodes, node)?;
        }
        Ok(ScopedLeaf(nodes))
    }
}

impl<L> Binding<L> {
    pub(in crate::v2) fn try_map<M, E>(
        self,
        leaf: impl FnOnce(L) -> Result<M, E>,
    ) -> Result<Binding<M>, E> {
        Ok(Binding {
            term: leaf(self.term)?,
            referent: self.referent,
        })
    }
}

impl<L> Suppression<L> {
    pub(in crate::v2) fn try_map<M, E>(
        self,
        mut leaf: impl FnMut(L) -> Result<M, E>,
    ) -> Result<Suppression<M>, E> {
        Ok(match self {
            Self::Term(term) => Suppression::Term(leaf(term)?),
            Self::Quad([s, p, o], g) => {
                Suppression::Quad([leaf(s)?, leaf(p)?, leaf(o)?], g.map(leaf).transpose()?)
            }
            Self::Reifier(term) => Suppression::Reifier(leaf(term)?),
            Self::Graph(term) => Suppression::Graph(leaf(term)?),
            Self::Blob(digest) => Suppression::Blob(digest),
            Self::ReifierBinding([r, t], g) => {
                Suppression::ReifierBinding([leaf(r)?, leaf(t)?], g.map(leaf).transpose()?)
            }
            Self::Annotation([r, p, o], g) => {
                Suppression::Annotation([leaf(r)?, leaf(p)?, leaf(o)?], g.map(leaf).transpose()?)
            }
            Self::Binding(binding) => Suppression::Binding(binding.try_map(leaf)?),
        })
    }
}
