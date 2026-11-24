// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The P3 wire records shared by bindings and suppression.

use core::num::NonZeroU64;
use std::io::{self, Write};

use purrdf_lex::cbor::head;
use purrdf_lex::walk::WorkList;

use super::fields::{Fields, OptionShape, push};
use super::terms::read_enum;
use super::{BLAKE3_256, ContentError, DIGEST_LENGTH, MalformedReason, WireError};

mod admission;
mod arrival;
mod fold;
mod resolution;
mod visibility;

pub(super) use visibility::Matches;

pub(super) use admission::{EntryAdmission, LocalTerms, ScopedLeaf, ScopedNode};
pub(super) use arrival::PendingReferences;
pub use arrival::ReferenceFindings;
pub use fold::{
    MissingReferent, ReferenceFold, ReferenceLocation, ResolvedReferent, StateLocation,
};
pub(super) use fold::{ReferenceClaims, StateCatalog};
pub(super) use resolution::FrameOwners;
pub use resolution::{FrameLocation, GroundLeaf, GroundNode, GroundTerm, ResolvedLeaf};
pub use visibility::{SuppressionClaim, SuppressionKind, SuppressionUnmatched, VisibleFold};

type Digest = [u8; DIGEST_LENGTH];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Leaf<'a> {
    encoded: &'a [u8],
}

impl<'a> Leaf<'a> {
    pub(super) fn nodes(self) -> impl Iterator<Item = LeafNode<'a>> {
        let mut fields = Fields::new(self.encoded);
        core::iter::from_fn(move || {
            if fields.remaining().is_empty() {
                return None;
            }
            Some(read_node(&mut fields, NonZeroU64::MIN).expect("leaf grammar was admitted"))
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LeafNode<'a, Id = u64> {
    Term(Id),
    Blank { frame: &'a Digest, label: &'a [u8] },
    Triple,
}

fn read_node<'a>(
    fields: &mut Fields<'a>,
    registry: NonZeroU64,
) -> Result<LeafNode<'a>, ContentError> {
    fields.record(2)?;
    Ok(match read_enum(fields, 3, registry)? {
        1 => LeafNode::Term(fields.uint()?),
        2 => {
            fields.record(2)?;
            LeafNode::Blank {
                frame: fields.fixed_bytes()?,
                label: fields.bytes()?,
            }
        }
        _ => {
            fields.record(3)?;
            LeafNode::Triple
        }
    })
}

/// Value named by a binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Referent {
    /// A blob's unit digest.
    Blob(Digest),
    /// The effective identifier of an ordinary frame.
    Frame(Digest),
    /// The effective identifier of a commit that can name a state.
    State(Digest),
}

pub(super) fn write_referent(
    output: &mut (impl Write + ?Sized),
    referent: &Referent,
) -> io::Result<()> {
    match referent {
        Referent::Blob(digest) => {
            output.write_all(&[0x82, 1])?;
            write_blob_name(output, digest)
        }
        Referent::Frame(digest) => {
            output.write_all(&[0x82, 2])?;
            head::write_bytes(output, digest)
        }
        Referent::State(digest) => {
            output.write_all(&[0x82, 3])?;
            head::write_bytes(output, digest)
        }
    }
}

pub(super) fn write_blob_name(
    output: &mut (impl Write + ?Sized),
    digest: &Digest,
) -> io::Result<()> {
    head::write_head(output, head::ARRAY, 2)?;
    head::write_head(output, head::UNSIGNED, BLAKE3_256)?;
    head::write_bytes(output, digest)
}

/// A term that stands for a transport referent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Binding<L> {
    /// The local or resolved term denoting the referent.
    pub term: L,
    /// The named blob, frame or state.
    pub referent: Referent,
}

/// One value in the suppression record.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Suppression<L> {
    /// Hide an occurrence of this term, including in a triple.
    Term(L),
    /// Hide the quad in the given graph or default graph.
    Quad([L; 3], Option<L>),
    /// Hide every annotation and reifier binding with this reifier.
    Reifier(L),
    /// Hide this named graph and its lines.
    Graph(L),
    /// Hide declarations of this blob.
    Blob(Digest),
    /// Hide the reifier binding in the named or default graph.
    ReifierBinding([L; 2], Option<L>),
    /// Hide the annotation in the named or default graph.
    Annotation([L; 3], Option<L>),
    /// Hide the binding of this term to this referent.
    Binding(Binding<L>),
}

pub(super) fn decode_bindings(
    payload: &[u8],
    expected: u64,
    registry: NonZeroU64,
) -> Result<Vec<Binding<Leaf<'_>>>, ContentError> {
    decode(payload, expected, registry, Decoder::binding)
}

pub(super) fn decode_suppressions(
    payload: &[u8],
    expected: u64,
    registry: NonZeroU64,
) -> Result<Vec<Suppression<Leaf<'_>>>, ContentError> {
    decode(payload, expected, registry, Decoder::suppression)
}

fn decode<'a, T>(
    bytes: &'a [u8],
    expected: u64,
    registry: NonZeroU64,
    mut entry: impl FnMut(&mut Decoder<'a>) -> Result<T, ContentError>,
) -> Result<Vec<T>, ContentError> {
    let mut fields = Fields::new(bytes);
    let count = fields.array()?;
    let mut decoder = Decoder {
        fields,
        registry,
        forbidden: None,
        parents: WorkList::new(),
    };
    let mut entries = Vec::new();
    for _ in 0..count {
        push(&mut entries, entry(&mut decoder)?)?;
    }
    decoder.fields.finish()?;
    if count != expected {
        return Err(WireError::new(MalformedReason::InconsistentCounts, 0).into());
    }
    if let Some(offset) = decoder.forbidden {
        return Err(WireError::new(MalformedReason::ForbiddenCombination, offset).into());
    }
    Ok(entries)
}

struct Triple {
    offset: usize,
    remaining: u8,
    has_pair: bool,
}

struct Decoder<'a> {
    fields: Fields<'a>,
    registry: NonZeroU64,
    forbidden: Option<usize>,
    parents: WorkList<Triple, 8>,
}

impl<'a> Decoder<'a> {
    fn binding(&mut self) -> Result<Binding<Leaf<'a>>, ContentError> {
        self.fields.record(2)?;
        Ok(Binding {
            term: self.leaf()?,
            referent: self.referent()?,
        })
    }

    fn referent(&mut self) -> Result<Referent, ContentError> {
        self.fields.record(2)?;
        Ok(match read_enum(&mut self.fields, 3, self.registry)? {
            1 => Referent::Blob(self.blob_name()?),
            2 => Referent::Frame(*self.fields.fixed_bytes()?),
            _ => Referent::State(*self.fields.fixed_bytes()?),
        })
    }

    fn blob_name(&mut self) -> Result<Digest, ContentError> {
        self.fields.record(2)?;
        read_enum(&mut self.fields, BLAKE3_256, self.registry)?;
        Ok(*self.fields.fixed_bytes()?)
    }

    fn suppression(&mut self) -> Result<Suppression<Leaf<'a>>, ContentError> {
        self.fields.record(2)?;
        Ok(match read_enum(&mut self.fields, 8, self.registry)? {
            1 => Suppression::Term(self.leaf()?),
            2 => {
                let (terms, graph) = self.row()?;
                Suppression::Quad(terms, graph)
            }
            3 => Suppression::Reifier(self.leaf()?),
            4 => Suppression::Graph(self.leaf()?),
            5 => Suppression::Blob(self.blob_name()?),
            6 => {
                self.fields.record(3)?;
                Suppression::ReifierBinding([self.leaf()?, self.leaf()?], self.graph()?)
            }
            7 => {
                let (terms, graph) = self.row()?;
                Suppression::Annotation(terms, graph)
            }
            _ => Suppression::Binding(self.binding()?),
        })
    }

    fn row(&mut self) -> Result<([Leaf<'a>; 3], Option<Leaf<'a>>), ContentError> {
        self.fields.record(4)?;
        Ok(([self.leaf()?, self.leaf()?, self.leaf()?], self.graph()?))
    }

    fn graph(&mut self) -> Result<Option<Leaf<'a>>, ContentError> {
        match self.fields.option_shape()? {
            OptionShape::Absent => Ok(None),
            OptionShape::Present => self.leaf().map(Some),
            OptionShape::Unknown { tag_offset, .. } => {
                Err(WireError::new(MalformedReason::UnregisteredValue, tag_offset).into())
            }
        }
    }

    fn leaf(&mut self) -> Result<Leaf<'a>, ContentError> {
        let begin = self.fields.remaining();
        loop {
            let offset = self.fields.offset();
            let mut has_pair = match read_node(&mut self.fields, self.registry)? {
                LeafNode::Term(_) => false,
                LeafNode::Blank { .. } => true,
                LeafNode::Triple => {
                    self.parents.try_push(Triple {
                        offset,
                        remaining: 3,
                        has_pair: false,
                    })?;
                    continue;
                }
            };
            loop {
                let Some(parent) = self.parents.top_mut() else {
                    let consumed = begin.len() - self.fields.remaining().len();
                    return Ok(Leaf {
                        encoded: &begin[..consumed],
                    });
                };
                parent.remaining -= 1;
                parent.has_pair |= has_pair;
                if parent.remaining != 0 {
                    break;
                }
                let parent = self.parents.pop().expect("completed triple is on top");
                if !parent.has_pair {
                    self.forbidden = Some(
                        self.forbidden
                            .map_or(parent.offset, |first| first.min(parent.offset)),
                    );
                }
                has_pair = parent.has_pair;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::{
        ContentError, LeafNode, MalformedReason, Suppression, decode_bindings, decode_suppressions,
    };
    use crate::v2::{
        Binding, FrameField, FrameType, Item, MissingReferent, RdfFold, ReadCapacity, ReadFailure,
        ReadGraph, ReferenceFold, Referent, ResolvedLeaf, ResolvedReferent, SegmentWriter, Term,
        Visibility, read,
    };
    use core::num::NonZeroU64;

    fn reference_payload(mut file: &[u8], kind: FrameType) -> (u64, &[u8]) {
        while let Some(item) = Item::first(file).unwrap() {
            file = &file[item.bytes().len()..];
            if let Some(frame) = item.frame().unwrap()
                && frame.read_field(FrameField::FrameType).uint().unwrap() == kind as u64
            {
                let mut content = frame.read_field(FrameField::Content);
                content.record(1).unwrap();
                let count = content.uint().unwrap();
                return (
                    count,
                    frame.read_field(FrameField::Payload).bytes().unwrap(),
                );
            }
        }
        panic!("fixture has no {kind:?} frame")
    }

    #[test]
    fn malformed_bindings_report_type_then_count_then_leaf_shape() {
        for (file, expected) in [
            (
                include_bytes!("../../tests/fixtures/v2/P3/bindings-order-type-before-count.bin")
                    .as_slice(),
                MalformedReason::UnregisteredValue,
            ),
            (
                include_bytes!("../../tests/fixtures/v2/P3/bindings-count-mismatch.bin").as_slice(),
                MalformedReason::InconsistentCounts,
            ),
            (
                include_bytes!("../../tests/fixtures/v2/P3/bindings-triple-leaf-no-pair.bin")
                    .as_slice(),
                MalformedReason::ForbiddenCombination,
            ),
        ] {
            let (count, payload) = reference_payload(file, FrameType::Bindings);
            assert!(matches!(decode_bindings(payload, count, NonZeroU64::MIN),
                Err(ContentError::Malformed(error)) if error.reason == expected));
        }
    }

    #[test]
    fn a_deep_binding_keeps_every_triple_and_its_cross_segment_leaf() {
        let file = include_bytes!("../../tests/fixtures/v2/P3/bindings-leaf-deep.bin");
        let (count, payload) = reference_payload(file, FrameType::Bindings);
        let bindings = decode_bindings(payload, count, NonZeroU64::MIN).unwrap();
        let [binding] = bindings.as_slice() else {
            panic!("fixture holds one binding");
        };
        let (triples, pairs) = binding.term.nodes().fold((0, 0), |(triples, pairs), node| {
            (
                triples + usize::from(node == LeafNode::Triple),
                pairs + usize::from(matches!(node, LeafNode::Blank { .. })),
            )
        });
        assert_eq!((triples, pairs), (100_000, 1));
    }

    #[test]
    fn quad_suppression_reads_three_term_leaves_and_the_default_graph() {
        let file = include_bytes!("../../tests/fixtures/v2/P3/suppress-order-free.after.bin");
        let (count, payload) = reference_payload(file, FrameType::Suppress);
        let entries = decode_suppressions(payload, count, NonZeroU64::MIN).unwrap();
        let [Suppression::Quad(terms, None)] = entries.as_slice() else {
            panic!("fixture holds one default-graph quad suppression");
        };
        for (leaf, id) in terms.iter().zip(0..3) {
            assert!(leaf.nodes().eq([LeafNode::Term(id)]));
        }

        let ReadGraph::Second(graph) = read(file, ReadCapacity::default()).unwrap() else {
            panic!("fixture uses generation 2");
        };
        assert_eq!(
            graph.visibility,
            Visibility::Complete {
                unauthenticated_suppress: 1
            },
        );
        assert_eq!(graph.references.suppressions.len(), 1);
        assert_eq!(graph.rdf.lines().count(), 2);
        assert_eq!(graph.into_complete_rdf().unwrap().lines().count(), 1);
    }

    #[test]
    fn strict_projection_keeps_the_dropped_binding_reason() {
        let file = include_bytes!("../../tests/fixtures/v2/P3/bindings-forward-reference.bin");
        let ReadGraph::Second(graph) = read(file, ReadCapacity::default()).unwrap() else {
            panic!("fixture uses generation 2");
        };
        let [finding] = graph.reference_findings.as_slice() else {
            panic!("the forward reference has one frame finding");
        };
        assert!(finding.forward_reference);
        assert!(!finding.opaque_term);
        let finding = *finding;
        let error = graph.into_complete_rdf().unwrap_err();
        assert_eq!(error.offset, finding.at.offset);
        assert!(matches!(error.failure, ReadFailure::References(actual) if actual == finding));
    }

    #[test]
    fn an_unfolded_suppression_reports_incomplete_visibility() {
        let file = include_bytes!("../../tests/fixtures/v2/P3/suppress-unknown-codec.bin");
        let ReadGraph::Second(graph) = read(file, ReadCapacity::default()).unwrap() else {
            panic!("fixture uses generation 2");
        };
        assert_eq!(
            graph.visibility,
            Visibility::Incomplete {
                unfolded_suppress: 1,
                unopened_sealed: 0,
                unauthenticated_suppress: 0,
            },
        );
    }

    #[test]
    fn the_item_limit_keeps_visibility_of_the_checked_prefix() {
        let file = include_bytes!("../../tests/fixtures/v2/P3/suppress-order-free.after.bin");
        let capacity = ReadCapacity {
            items: 4,
            ..ReadCapacity::default()
        };
        let ReadGraph::Second(graph) = read(file, capacity).unwrap() else {
            panic!("fixture uses generation 2");
        };
        assert!(graph.item_refusal.is_some());
        assert_eq!(
            graph.visibility,
            Visibility::Complete {
                unauthenticated_suppress: 1
            },
        );
        assert_eq!(graph.rdf.lines().count(), 0);
    }

    #[test]
    fn a_value_from_an_earlier_run_does_not_name_a_state_after_a_refused_rebuild() {
        let input = include_bytes!("../../tests/fixtures/v2/P3/state-rebuild-refused-mixed.bin");
        let mut remaining = input.as_slice();
        let referent = loop {
            let item = Item::first(remaining)
                .unwrap()
                .expect("fixture holds a commit");
            remaining = &remaining[item.bytes().len()..];
            if let Some(frame) = item.frame().unwrap()
                && frame.read_field(FrameField::FrameType).uint().unwrap()
                    == FrameType::Commit as u64
            {
                break Referent::State(
                    *frame
                        .read_field(FrameField::Identifier)
                        .fixed_bytes()
                        .unwrap(),
                );
            }
        };
        let mut prefix = SegmentWriter::new(Vec::new()).unwrap();
        let value = [[b'a'; 32], [b'b'; 32]].concat();
        prefix.add_value(&value).unwrap();
        prefix.add_commit().unwrap();
        let mut binding_fold = RdfFold::default();
        let term = binding_fold
            .intern_term(Term::Iri(Cow::Borrowed("http://example.org/state")))
            .unwrap();
        let mut references = ReferenceFold::default();
        references.bindings.push(Binding {
            term: ResolvedLeaf::Named(term),
            referent,
        });
        let mut suffix = SegmentWriter::new(Vec::new()).unwrap();
        suffix
            .add_rdf_with_references(&binding_fold, &references)
            .unwrap();
        let bytes = [
            prefix.into_inner().as_slice(),
            input,
            suffix.into_inner().as_slice(),
        ]
        .concat();

        let read_at = |rebuilt_bytes| {
            let capacity = ReadCapacity {
                rebuilt_bytes,
                ..ReadCapacity::default()
            };
            let ReadGraph::Second(graph) = read(&bytes, capacity).unwrap() else {
                panic!("fixture uses generation 2");
            };
            graph
        };
        let limited = read_at(128);
        assert_eq!(
            limited.references.referents[&referent],
            ResolvedReferent::Missing(MissingReferent::StateUnavailable),
        );
        let Referent::State(identifier) = referent else {
            unreachable!("the test constructs a state referent");
        };
        assert!(limited.read_state(&identifier).unwrap().is_none());
        assert_eq!(limited.rdf.lines().count(), 1);
        let admitted = read_at(192);
        let ResolvedReferent::State(state) = admitted.references.referents[&referent] else {
            panic!("the in-run rebuild makes the state available");
        };
        assert_eq!(state.first_segment, 1);
        let state = admitted.read_state(&identifier).unwrap().unwrap();
        assert_eq!(state.rdf.lines().count(), 1);
        assert_eq!(state.references.bindings, []);
    }
}
