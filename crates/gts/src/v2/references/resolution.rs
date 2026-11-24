// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::borrow::Cow;
use std::collections::TryReserveError;

use purrdf_lex::walk::WorkList;

use crate::FastMap;
use crate::v2::fields::push;
use crate::v2::{FoldError, LinkState, RdfFold, Term, TermId};

use super::{Digest, LeafNode, ScopedLeaf, ScopedNode};

/// The position of a frame found by its effective identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameLocation {
    /// File-relative segment ordinal.
    pub segment: u64,
    /// Item position in that segment.
    pub position: usize,
    /// Byte offset of the frame.
    pub offset: usize,
}

struct Owners {
    first: FrameLocation,
    // Excluding the referencing segment can skip only one distinct holder.
    holders: [Option<u64>; 2],
}

#[derive(Default)]
pub(in crate::v2) struct FrameOwners(FastMap<Digest, Owners>);

impl FrameOwners {
    /// Observe an ordinary frame with a usable effective identifier.
    pub(in crate::v2) fn observe(
        &mut self,
        identifier: Digest,
        frame: FrameLocation,
        link: LinkState,
    ) -> Result<(), TryReserveError> {
        if !self.0.contains_key(&identifier) {
            self.0.try_reserve(1)?;
        }
        let owners = self.0.entry(identifier).or_insert(Owners {
            first: frame,
            holders: [None; 2],
        });
        if link != LinkState::Broken
            && !owners.holders.contains(&Some(frame.segment))
            && let Some(slot) = owners.holders.iter_mut().find(|slot| slot.is_none())
        {
            *slot = Some(frame.segment);
        }
        Ok(())
    }

    /// Resolve a frame referent, including the effective identifier of a broken frame.
    pub(in crate::v2) fn first(&self, identifier: &Digest) -> Option<FrameLocation> {
        self.0.get(identifier).map(|owners| owners.first)
    }

    fn blank(&self, source: u64, frame: &Digest, label: &[u8], fold: &RdfFold) -> Option<TermId> {
        let segment = self
            .0
            .get(frame)?
            .holders
            .into_iter()
            .flatten()
            .find(|&segment| segment != source)?;
        fold.find_term(&Term::Blank {
            segment,
            label: Cow::Borrowed(label),
        })
    }
}

/// A component of a ground leaf, in terms of readable terms and its own nodes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GroundTerm {
    /// Handle of a readable term in the RDF fold.
    Named(TermId),
    /// Child node in the owning ground leaf.
    Ground(usize),
}

/// An unresolved pair or a triple with an unresolved component.
#[derive(Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GroundNode {
    /// A wire blank-node pair with no named node in the file.
    Pair {
        /// Effective frame identifier named by the pair.
        frame: Digest,
        /// Original label bytes.
        label: Vec<u8>,
    },
    /// A triple with at least one ground component.
    Triple([GroundTerm; 3]),
}

/// An unnamed leaf. Child ground handles precede their parent. The last node is its root.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct GroundLeaf(pub(in crate::v2) Vec<GroundNode>);

impl GroundLeaf {
    /// Ground nodes, with each child before its parent and the root last.
    #[must_use]
    pub fn nodes(&self) -> &[GroundNode] {
        &self.0
    }
}

/// Result of resolving a binding or suppression leaf in the file.
#[derive(Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ResolvedLeaf {
    /// A readable term in the file's fold.
    Named(TermId),
    /// A leaf containing a pair that names no node in the file.
    Ground(GroundLeaf),
}

impl ResolvedLeaf {
    /// Visit triples before their components, with named terms kept as handles.
    pub(in crate::v2) fn visit<E>(
        &self,
        mut node: impl FnMut(LeafNode<'_, TermId>) -> Result<(), E>,
        mut allocation_error: impl FnMut(TryReserveError) -> E,
    ) -> Result<(), E> {
        let ground = match self {
            Self::Named(term) => return node(LeafNode::Term(*term)),
            Self::Ground(ground) => ground.nodes(),
        };
        // A ground leaf's root is the final entry in its postorder storage.
        let mut work = WorkList::<GroundTerm, 8>::with(GroundTerm::Ground(ground.len() - 1));
        while let Some(term) = work.pop() {
            match term {
                GroundTerm::Named(term) => node(LeafNode::Term(term))?,
                GroundTerm::Ground(index) => match &ground[index] {
                    GroundNode::Pair { frame, label } => node(LeafNode::Blank { frame, label })?,
                    GroundNode::Triple(children) => {
                        node(LeafNode::Triple)?;
                        for &child in children.iter().rev() {
                            work.try_push(child).map_err(&mut allocation_error)?;
                        }
                    }
                },
            }
        }
        Ok(())
    }
}

impl ScopedLeaf {
    pub(in crate::v2) fn resolve(
        self,
        source: u64,
        frames: &FrameOwners,
        fold: &mut RdfFold,
    ) -> Result<ResolvedLeaf, FoldError> {
        let mut terms = WorkList::<GroundTerm, 8>::new();
        let mut ground = Vec::new();
        for node in self.0.into_iter().rev() {
            let term = match node {
                ScopedNode::Term(id) => GroundTerm::Named(id),
                ScopedNode::Blank { frame, label } => {
                    if let Some(id) = frames.blank(source, &frame, &label, fold) {
                        GroundTerm::Named(id)
                    } else {
                        let id = ground.len();
                        push(&mut ground, GroundNode::Pair { frame, label })?;
                        GroundTerm::Ground(id)
                    }
                }
                ScopedNode::Triple => {
                    let components = core::array::from_fn(|_| {
                        terms
                            .pop()
                            .expect("an admitted triple has three components")
                    });
                    match components {
                        [
                            GroundTerm::Named(s),
                            GroundTerm::Named(p),
                            GroundTerm::Named(o),
                        ] => GroundTerm::Named(fold.intern_term(Term::Triple([s, p, o]))?),
                        _ => {
                            let id = ground.len();
                            push(&mut ground, GroundNode::Triple(components))?;
                            GroundTerm::Ground(id)
                        }
                    }
                }
            };
            terms.try_push(term)?;
        }
        Ok(match terms.pop().expect("an admitted leaf has a root") {
            GroundTerm::Named(id) => ResolvedLeaf::Named(id),
            GroundTerm::Ground(_) => ResolvedLeaf::Ground(GroundLeaf(ground)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Cow, FrameLocation, FrameOwners, GroundLeaf, GroundNode, GroundTerm, LinkState, RdfFold,
        ResolvedLeaf, ScopedLeaf, ScopedNode, Term,
    };

    #[test]
    fn a_blank_pair_uses_the_first_other_holder_without_falling_through_its_dictionary() {
        let frame = [7; 32];
        let label = b"local";
        let mut fold = RdfFold::default();
        let mut frames = FrameOwners::default();
        for (segment, link) in [
            (0, LinkState::Broken),
            (1, LinkState::Linked),
            (2, LinkState::Healed),
        ] {
            frames
                .observe(
                    frame,
                    FrameLocation {
                        segment,
                        position: 1,
                        offset: segment as usize,
                    },
                    link,
                )
                .unwrap();
        }
        assert_eq!(frames.first(&frame).unwrap().segment, 0);
        let blank = |segment| Term::Blank {
            segment,
            label: Cow::Borrowed(label),
        };
        fold.intern_term(blank(0)).unwrap();
        let resolved_blank = fold.intern_term(blank(2)).unwrap();
        let predicate = fold
            .intern_term(Term::Iri(Cow::Borrowed("http://example.org/p")))
            .unwrap();
        let object = fold
            .intern_term(Term::Iri(Cow::Borrowed("http://example.org/o")))
            .unwrap();
        let leaf = || {
            ScopedLeaf(vec![
                ScopedNode::Triple,
                ScopedNode::Blank {
                    frame,
                    label: label.to_vec(),
                },
                ScopedNode::Term(predicate),
                ScopedNode::Term(object),
            ])
        };

        let ResolvedLeaf::Ground(GroundLeaf(nodes)) =
            leaf().resolve(3, &frames, &mut fold).unwrap()
        else {
            panic!("the first linkable holder has no definition for the label");
        };
        assert!(matches!(nodes.as_slice(), [
            GroundNode::Pair { frame: held, label: written },
            GroundNode::Triple([GroundTerm::Ground(0), GroundTerm::Named(p), GroundTerm::Named(o)])
        ] if held == &frame && written == label && *p == predicate && *o == object));
        let ResolvedLeaf::Named(triple) = leaf().resolve(1, &frames, &mut fold).unwrap() else {
            panic!("the referencing segment is excluded from the holder search");
        };
        assert_eq!(
            fold.term(triple),
            Some(&Term::Triple([resolved_blank, predicate, object]))
        );
    }
}
