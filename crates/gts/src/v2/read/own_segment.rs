// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::fmt;
use core::ops::Range;
use std::collections::{TryReserveError, VecDeque};

use purrdf_lex::cbor::StructuralError;

use crate::v2::fields::push;
use crate::v2::index::IndexFoldClaims;
use crate::v2::{FrameType, Item, SignatureInputs};

use super::state::ReadPoint;
use super::{
    GraphReader, HeldItems, RdfGraph, ReadError, ReadFailure, ResyncReach, Segment, decode_opener,
};

enum CachedNext<'a> {
    Unchecked,
    Delimited(Option<Item<'a>>),
    Framing(usize, StructuralError),
    Depth(usize, usize, usize, StructuralError),
    ItemCapacity(usize, u64),
}

struct Lookahead<'a> {
    complete: VecDeque<Item<'a>>,
    complete_end: usize,
    next: CachedNext<'a>,
    region_counted: bool,
}

impl<'a> Lookahead<'a> {
    fn capture(items: &HeldItems<'a>, ordinal: u64, opener: usize) -> Result<Self, ReadError> {
        let next = match &items.next {
            None => CachedNext::Unchecked,
            Some(Ok(item)) => CachedNext::Delimited(*item),
            Some(Err(error)) => match &error.failure {
                ReadFailure::Framing(cause) => CachedNext::Framing(error.offset, cause.clone()),
                ReadFailure::ItemDepth {
                    required,
                    limit,
                    cause,
                } => CachedNext::Depth(error.offset, *required, *limit, cause.clone()),
                ReadFailure::ItemCapacity(limit) => CachedNext::ItemCapacity(error.offset, *limit),
                _ => {
                    return Err(ReadError {
                        offset: error.offset,
                        failure: ReadFailure::SegmentOwnFold(ordinal),
                    });
                }
            },
        };
        Ok(Self {
            complete: copy_items(&items.complete).map_err(|cause| ReadError {
                offset: opener,
                failure: cause.into(),
            })?,
            complete_end: items.complete_end,
            next,
            region_counted: items.region_counted,
        })
    }

    fn restore(&self) -> Result<HeldItems<'a>, TryReserveError> {
        let next = match &self.next {
            CachedNext::Unchecked => None,
            CachedNext::Delimited(item) => Some(Ok(*item)),
            CachedNext::Framing(offset, cause) => Some(Err(ReadError {
                offset: *offset,
                failure: ReadFailure::Framing(cause.clone()),
            })),
            CachedNext::Depth(offset, required, limit, cause) => Some(Err(ReadError {
                offset: *offset,
                failure: ReadFailure::ItemDepth {
                    required: *required,
                    limit: *limit,
                    cause: cause.clone(),
                },
            })),
            CachedNext::ItemCapacity(offset, limit) => Some(Err(ReadError {
                offset: *offset,
                failure: ReadFailure::ItemCapacity(*limit),
            })),
        };
        Ok(HeldItems {
            complete: copy_items(&self.complete)?,
            complete_end: self.complete_end,
            next,
            region_counted: self.region_counted,
        })
    }
}

fn copy_items<'a>(source: &VecDeque<Item<'a>>) -> Result<VecDeque<Item<'a>>, TryReserveError> {
    let mut complete = VecDeque::new();
    complete.try_reserve_exact(source.len())?;
    complete.extend(source.iter().copied());
    Ok(complete)
}

pub(in crate::v2) struct OwnSegment<'a> {
    pub(in crate::v2) ordinal: u64,
    opener: Range<usize>,
    reach: ResyncReach,
    point: ReadPoint,
    lookahead: Lookahead<'a>,
    wire: Range<usize>,
    size: u128,
}

impl<'a> OwnSegment<'a> {
    pub(in crate::v2) fn fold(&self, original: &RdfGraph<'a>) -> Result<RdfGraph<'a>, ReadError> {
        let offset = self.opener.start;
        let missing = || ReadError {
            offset,
            failure: ReadFailure::MissingContent,
        };
        let input = original.state_source.input().ok_or_else(missing)?;
        let signatures = SignatureInputs::default();
        let mut reader = self.point.reader(input, &signatures, self.ordinal);
        reader.stop_after_segment = true;
        let opener = reader
            .delimit_item(self.opener.end, offset)?
            .ok_or_else(missing)?;
        let header = decode_opener(opener).map_err(|failure| ReadError { offset, failure })?;
        let segment =
            reader.open_segment(header, self.opener.clone(), None, self.reach, reader.items)?;
        let items = self.lookahead.restore().map_err(|error| ReadError {
            offset,
            failure: error.into(),
        })?;
        let folded = reader.read_segment(segment, offset, self.opener.end, items)?;
        if folded.labelling_size() != Some(self.size)
            || original.frame_wire.get(self.wire.clone()) != Some(folded.frame_wire.as_slice())
            || original.ordinary_folds.get(self.wire.clone())
                != Some(folded.ordinary_folds.as_slice())
        {
            return Err(ReadError {
                offset,
                failure: ReadFailure::SegmentOwnFold(self.ordinal),
            });
        }
        Ok(folded)
    }

    pub(in crate::v2) fn closing_index(
        &self,
        original: &RdfGraph<'a>,
    ) -> Option<(usize, IndexFoldClaims)> {
        let frames = original.frame_wire.get(self.wire.clone())?;
        let dispositions = original.ordinary_folds.get(self.wire.clone())?;
        let mut ordinary = frames
            .iter()
            .zip(dispositions)
            .rev()
            .filter_map(|(frame, admission)| admission.map(|admission| (frame, admission)));
        let (last, _) = ordinary.next()?;
        if last.frame_type != Some(FrameType::Index as u64)
            || ordinary.any(|(_, admitted)| !admitted)
        {
            return None;
        }
        let at = original
            .index_checks
            .binary_search_by_key(&last.offset, |check| check.offset)
            .ok()?;
        let index = &original.index_checks[at];
        index.prefix.as_ref().ok()?;
        Some((last.offset, index.fold_claims?))
    }
}

#[derive(Default)]
pub(super) struct OwnSegments<'a>(Vec<OwnSegment<'a>>);

impl fmt::Debug for OwnSegments<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OwnSegments")
            .field("segments", &self.0.len())
            .finish()
    }
}

impl<'a> OwnSegments<'a> {
    pub(super) fn finish(&mut self, ordinal: u64, size: u128, wire_end: usize) {
        if let Some(segment) = self
            .0
            .last_mut()
            .filter(|segment| segment.ordinal == ordinal)
        {
            segment.size = size;
            segment.wire.end = wire_end;
        }
    }

    pub(in crate::v2) fn iter(&self) -> impl ExactSizeIterator<Item = &OwnSegment<'a>> {
        self.0.iter()
    }
}

impl<'a> GraphReader<'a, '_> {
    pub(super) fn remember_own_segment(
        &mut self,
        segment: Option<&Segment<'a>>,
        opener: Range<usize>,
        reach: ResyncReach,
        items: &HeldItems<'a>,
    ) -> Result<(), ReadError> {
        let Some(segment) = segment.filter(|_| self.graph.state_source.input().is_some()) else {
            return Ok(());
        };
        let error = |cause: TryReserveError| ReadError {
            offset: opener.start,
            failure: cause.into(),
        };
        let wire = self.graph.frame_wire.len();
        let segment = OwnSegment {
            ordinal: segment.file_ordinal,
            opener: opener.clone(),
            reach,
            point: ReadPoint::capture(self, self.items),
            lookahead: Lookahead::capture(items, segment.file_ordinal, opener.start)?,
            wire: wire..wire,
            size: 0,
        };
        push(&mut self.graph.own_segments.0, segment).map_err(error)
    }
}
