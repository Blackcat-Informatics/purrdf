// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::borrow::Cow;
use std::io;

use purrdf_lex::cbor::head;

use crate::v2::writer::ReferenceRecords;
use crate::v2::{
    ContentError, FrameType, Line, PayloadOutcome, PayloadRefusal, PayloadUnchecked, RdfGraph,
    ReadFailure, RebuildState, SegmentWriteError, Term, TermId,
};

use super::ContentLabelling;

/// The labelling level of a readable segment's P7 canonical form check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CanonicalLabellingCheck {
    /// The stored blank labels and any eligible index digest agree with the fold.
    Verified,
    /// The first difference at this level.
    Failed {
        /// File-relative byte offset of the affected frame.
        offset: usize,
        /// Item position within the segment.
        position: usize,
        /// Rule that failed at this position.
        difference: CanonicalLabellingDifference,
    },
    /// The first reason the comparison could not run.
    Unchecked(CanonicalLabellingUnchecked),
}

/// A difference found by the labelling level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CanonicalLabellingDifference {
    /// The stored labels name no labelling of the fold.
    BlankLabels,
    /// The index digest differs from the digest of the fold.
    ContentDigest,
}

/// Why the labelling level of an own-fold report is unchecked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CanonicalLabellingUnchecked {
    /// The segment makes no known canonical claim.
    NotClaimed,
    /// A capacity refusal prevented a comparison.
    ExceedsCapacity,
    /// The own fold could not be labelled.
    LabellingRefused,
}

pub(super) fn check(
    fold: &RdfGraph<'_>,
    labelling: Option<&ContentLabelling>,
) -> io::Result<CanonicalLabellingCheck> {
    use CanonicalLabellingCheck::Unchecked;
    use CanonicalLabellingDifference::{BlankLabels, ContentDigest};

    if !fold.canonical_claim {
        return Ok(Unchecked(CanonicalLabellingUnchecked::NotClaimed));
    }
    let index = index_digest(fold);
    if matches!(index, Some((_, None))) || content_exceeds_capacity(fold) {
        return Ok(Unchecked(CanonicalLabellingUnchecked::ExceedsCapacity));
    }
    let Some(labelled) = labelling else {
        return Ok(Unchecked(CanonicalLabellingUnchecked::LabellingRefused));
    };
    let labels_difference = invalid_labels(fold, labelled)?.map(|offset| (offset, BlankLabels));
    let digest_difference = index.and_then(|(offset, digest)| {
        (digest != labelled.content_digest).then_some((offset, ContentDigest))
    });
    let Some((offset, difference)) = [labels_difference, digest_difference]
        .into_iter()
        .flatten()
        .min_by_key(|(offset, _)| *offset)
    else {
        return Ok(CanonicalLabellingCheck::Verified);
    };
    let index = fold
        .frame_wire
        .binary_search_by_key(&offset, |frame| frame.offset)
        .map_err(|_| io::Error::other("the canonical difference has no retained frame"))?;
    Ok(CanonicalLabellingCheck::Failed {
        offset,
        position: fold.frame_wire[index].position,
        difference,
    })
}

// None in the digest slot identifies an index whose payload could not be
// checked under capacity. An admitted index with an absent claim is skipped.
fn index_digest(fold: &RdfGraph<'_>) -> Option<(usize, Option<[u8; 32]>)> {
    fold.index_checks.iter().rev().find_map(|index| {
        if fold.stopped_frames.binary_search(&index.offset).is_ok() {
            return None;
        }
        if matches!(
            index.prefix,
            Err(ReadFailure::PayloadRefusal(
                PayloadRefusal::StoredSize(_)
                    | PayloadRefusal::DecodedSize(_)
                    | PayloadRefusal::TotalDecoded(_)
            ))
        ) {
            return Some((index.offset, None));
        }
        let digest = index.declared_digest?;
        Some((index.offset, Some(digest)))
    })
}

fn content_exceeds_capacity(fold: &RdfGraph<'_>) -> bool {
    fold.rebuilds
        .iter()
        .any(|r| r.is_value && matches!(r.state, RebuildState::Refused { .. }))
        || fold.frame_failures.iter().any(|item| {
            matches!(
                item.failure,
                ReadFailure::Content(ContentError::FormedCapacity(_))
            )
        })
        || fold.frame_wire.iter().any(|frame| {
            let kind = frame.frame_type.and_then(FrameType::from_code);
            !matches!(
                kind,
                None | Some(FrameType::Commit | FrameType::Index | FrameType::Parity)
            ) && frame.payload == PayloadOutcome::Unchecked(PayloadUnchecked::ExceedsCapacity)
                && fold.stopped_frames.binary_search(&frame.offset).is_err()
                && !has_prior_failure(fold, frame.offset)
        })
}

fn has_prior_failure(fold: &RdfGraph<'_>, offset: usize) -> bool {
    let first = fold
        .frame_failures
        .partition_point(|item| item.offset < offset);
    fold.frame_failures[first..]
        .iter()
        .take_while(|item| item.offset == offset)
        .any(|item| {
            matches!(
                item.failure,
                ReadFailure::Wire(_) | ReadFailure::BrokenLink | ReadFailure::UnknownFrame(_)
            )
        })
}

fn invalid_labels(graph: &RdfGraph<'_>, labelled: &ContentLabelling) -> io::Result<Option<usize>> {
    let positions = labelled.blank_positions()?;
    if positions.is_empty() {
        return Ok(None);
    }
    let rdf = &graph.rdf;
    let first = graph.canonical_labels.iter().find_map(|&(id, offset)| {
        let Term::Blank { label, .. } = rdf.term(id)? else {
            return None;
        };
        let &position = positions.get(&(0, label.as_ref()))?;
        head::with_encoded_head(head::UNSIGNED, position, |bytes| {
            (bytes != label.as_ref()).then_some(offset)
        })
    });
    let Some(offset) = first else {
        return Ok(None);
    };
    let mut images = Vec::new();
    images
        .try_reserve_exact(rdf.terms().len())
        .map_err(io::Error::other)?;
    for (index, term) in rdf.terms().enumerate() {
        let id = TermId::from_index(index);
        let image = match term {
            Term::Blank { segment, label } => {
                if let Some(&position) = positions.get(&(0, label.as_ref())) {
                    head::with_encoded_head(head::UNSIGNED, position, |bytes| {
                        if !positions.contains_key(&(0, bytes)) {
                            return None;
                        }
                        rdf.find_term(&Term::Blank {
                            segment: *segment,
                            label: Cow::Borrowed(bytes),
                        })
                    })
                } else {
                    Some(id)
                }
            }
            Term::Triple(terms) => match terms.map(|id| images[id.index()]) {
                [Some(s), Some(p), Some(o)] => rdf.find_term(&Term::Triple([s, p, o])),
                _ => None,
            },
            _ => Some(id),
        };
        images.push(image);
    }
    // The distinct L1 positions give an injective map on the used blanks.
    // Membership of all renamed lines and records therefore proves equality
    // of the finite fold, including the counts of annotations.
    let map = |id: TermId| images[id.index()];
    if !rdf
        .lines()
        .all(|line| renamed_line(line, map).is_some_and(|image| rdf.contains_line(image)))
    {
        return Ok(Some(offset));
    }
    let before = ReferenceRecords::new(&graph.references, |id| Ok(id.index() as u64))
        .map_err(io::Error::other)?;
    match ReferenceRecords::new(&graph.references, |id| {
        map(id)
            .map(|id| id.index() as u64)
            .ok_or(SegmentWriteError::ReferenceTerm(
                crate::v2::FoldError::TermHandle(id),
            ))
    }) {
        Ok(after) => Ok((before != after).then_some(offset)),
        Err(SegmentWriteError::ReferenceTerm(_)) => Ok(Some(offset)),
        Err(error) => Err(io::Error::other(error)),
    }
}

fn renamed_line(line: Line, map: impl Fn(TermId) -> Option<TermId>) -> Option<Line> {
    let graph = |name| match name {
        Some(id) => map(id).map(Some),
        None => Some(None),
    };
    Some(match line {
        Line::Quad([s, p, o], g) => Line::Quad([map(s)?, map(p)?, map(o)?], graph(g)?),
        Line::ReifierBinding([r, t], g) => Line::ReifierBinding([map(r)?, map(t)?], graph(g)?),
        Line::Annotation([r, p, o], count, g) => {
            Line::Annotation([map(r)?, map(p)?, map(o)?], count, graph(g)?)
        }
        Line::EmptyGraph(g) => Line::EmptyGraph(map(g)?),
    })
}
