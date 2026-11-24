// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::num::NonZeroU64;
use std::io::{self, Write};

use purrdf_lex::cbor::DecodeErrorKind;

use crate::v2::fields::Fields;
use crate::v2::parity::SPAN_BYTES;
use crate::v2::writer::write_structure;
use crate::v2::{
    ContentError, ContentLabelling, FrameEnvelope, FrameField, FrameType, Header, Item, ItemKind,
    LinkState, PayloadBudget, PayloadCapacity, PayloadOutcome, PayloadState, PayloadUnchecked,
    REGISTRY_VERSION, RdfGraph, ReadFailure, RebuildState,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::v2) enum PackRegion {
    Value,
    Blob,
}

/// The structure level of a readable segment's canonical form check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CanonicalStructureCheck {
    /// The items agree with the canonical construction.
    Verified,
    /// The first structural difference.
    Failed {
        /// File-relative byte offset of the difference.
        offset: usize,
        /// Item position within the segment.
        position: usize,
        /// Rule that failed.
        difference: CanonicalStructureDifference,
    },
    /// The segment makes no known canonical claim.
    NotClaimed,
    /// Read capacity prevented the comparison.
    ExceedsCapacity,
}

/// The part of the canonical structure that differs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CanonicalStructureDifference {
    /// The opener's fields differ from the canonical header.
    Header,
    /// An item is missing, extra or out of order.
    FrameSequence,
    /// The dictionary content differs.
    Terms,
    /// The RDF row content differs.
    Rows,
    /// The value packs differ.
    ValueRegion,
    /// The blob packs or external declarations differ.
    BlobRegion,
    /// The binding records differ.
    Bindings,
    /// The suppression records differ.
    Suppression,
    /// The commit differs.
    Commit,
    /// The index differs.
    Index,
}

// Ordinary frame kinds occur in this order. Parity is tracked independently
// over the actual encoded stream.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    ValuePack,
    BlobPack,
    External,
    Terms,
    Quads,
    ReifierBindings,
    Annotations,
    EmptyGraphs,
    Bindings,
    Suppression,
    Commit,
    Index,
    Offsets,
    Locator,
    LooseValue,
    LooseBlob,
    Parity,
    Other,
}

impl Kind {
    const fn frame(kind: FrameType, pack_region: PackRegion) -> Self {
        match kind {
            FrameType::Pack => match pack_region {
                PackRegion::Value => Self::ValuePack,
                PackRegion::Blob => Self::BlobPack,
            },
            FrameType::External => Self::External,
            FrameType::Terms => Self::Terms,
            FrameType::Quads => Self::Quads,
            FrameType::ReifierBindings => Self::ReifierBindings,
            FrameType::Annotations => Self::Annotations,
            FrameType::EmptyGraphs => Self::EmptyGraphs,
            FrameType::Bindings => Self::Bindings,
            FrameType::Suppress => Self::Suppression,
            FrameType::Commit => Self::Commit,
            FrameType::Index => Self::Index,
            FrameType::Value => Self::LooseValue,
            FrameType::Chunk | FrameType::Blob | FrameType::Recipe => Self::LooseBlob,
            FrameType::Parity => Self::Parity,
        }
    }

    const fn loose(self) -> bool {
        matches!(self, Self::LooseValue | Self::LooseBlob)
    }

    const fn difference(self) -> CanonicalStructureDifference {
        use CanonicalStructureDifference as D;
        match self {
            Self::ValuePack | Self::LooseValue => D::ValueRegion,
            Self::BlobPack | Self::External | Self::LooseBlob => D::BlobRegion,
            Self::Terms => D::Terms,
            Self::Quads | Self::ReifierBindings | Self::Annotations | Self::EmptyGraphs => D::Rows,
            Self::Bindings => D::Bindings,
            Self::Suppression => D::Suppression,
            Self::Commit => D::Commit,
            Self::Index => D::Index,
            Self::Offsets | Self::Locator | Self::Parity | Self::Other => D::FrameSequence,
        }
    }
}

struct Located<'a> {
    kind: Kind,
    item: Option<Item<'a>>,
    offset: usize,
    position: usize,
}

impl Located<'_> {
    const fn fail(&self, difference: CanonicalStructureDifference) -> CanonicalStructureCheck {
        CanonicalStructureCheck::Failed {
            offset: self.offset,
            position: self.position,
            difference,
        }
    }

    const fn wrong_kind(&self) -> CanonicalStructureCheck {
        self.fail(if self.kind.loose() {
            self.kind.difference()
        } else {
            CanonicalStructureDifference::FrameSequence
        })
    }
}

struct Source<'a, 'fold> {
    tail: &'a [u8],
    registry: NonZeroU64,
    offset: usize,
    position: usize,
    over_span: usize,
    fold: &'fold RdfGraph<'a>,
}

impl<'a> Source<'a, '_> {
    fn take(&mut self) -> io::Result<Located<'a>> {
        let offset = self.offset;
        let position = self.position;
        let item = match Item::first(self.tail) {
            Ok(item) => item,
            Err(error)
                if matches!(
                    error.kind(),
                    DecodeErrorKind::Truncated | DecodeErrorKind::Malformed(_)
                ) =>
            {
                None
            }
            Err(error) => return Err(io::Error::other(error)),
        };
        let kind = match item {
            Some(item) => self.classify(item, offset)?,
            None => Kind::Other,
        };
        if let Some(item) = item {
            self.position += 1;
            self.offset += item.bytes().len();
            self.tail = &self.tail[item.bytes().len()..];
        } else {
            self.tail = &[];
        }
        Ok(Located {
            kind,
            item,
            offset,
            position,
        })
    }

    fn classify(&self, item: Item<'_>, offset: usize) -> io::Result<Kind> {
        Ok(match item.classify(self.registry) {
            Ok(ItemKind::Offsets) => Kind::Offsets,
            Ok(ItemKind::Locator) => Kind::Locator,
            Ok(ItemKind::Frame) => {
                let Some(kind) = frame_type(&envelope(item)?) else {
                    return Ok(Kind::Other);
                };
                let region = if kind == FrameType::Pack {
                    let index = self
                        .fold
                        .pack_regions
                        .partition_point(|(at, _)| *at < offset);
                    self.fold
                        .pack_regions
                        .get(index)
                        .map_or(PackRegion::Blob, |&(_, region)| region)
                } else {
                    PackRegion::Blob
                };
                Kind::frame(kind, region)
            }
            _ => Kind::Other,
        })
    }

    fn parity_after(
        &mut self,
        length: usize,
        final_frame: bool,
    ) -> io::Result<Option<Located<'a>>> {
        // Decompose ordinary item lengths before adding them to the remainder.
        let tail = self.over_span + length % SPAN_BYTES;
        let full = length / SPAN_BYTES + tail / SPAN_BYTES;
        self.over_span = tail % SPAN_BYTES;
        let short = usize::from(final_frame && self.over_span != 0);
        for _ in 0..full + short {
            let found = self.take()?;
            if found.kind != Kind::Parity {
                return Ok(Some(found));
            }
        }
        Ok(None)
    }

    fn finish(mut self) -> io::Result<CanonicalStructureCheck> {
        for kind in [Kind::Offsets, Kind::Locator] {
            let found = self.take()?;
            if found.kind != kind {
                return Ok(found.wrong_kind());
            }
        }
        Ok(if self.tail.is_empty() {
            CanonicalStructureCheck::Verified
        } else {
            self.take()?.wrong_kind()
        })
    }
}

fn frame_type(frame: &FrameEnvelope<'_>) -> Option<FrameType> {
    FrameType::from_code(frame.read_field(FrameField::FrameType).uint().ok()?)
}

pub(super) fn check_structure<'a>(
    fold: &RdfGraph<'a>,
    labelled: Option<&ContentLabelling>,
    (offset, input): (usize, &'a [u8]),
) -> io::Result<CanonicalStructureCheck> {
    use CanonicalStructureCheck as Check;
    if !fold.canonical_claim {
        return Ok(Check::NotClaimed);
    }
    if fold
        .frame_wire
        .iter()
        .any(|frame| frame.payload == PayloadOutcome::Unchecked(PayloadUnchecked::ExceedsCapacity))
        || fold
            .rebuilds
            .iter()
            .any(|result| matches!(result.state, RebuildState::Refused { .. }))
        || fold.frame_failures.iter().any(|item| {
            matches!(
                item.failure,
                ReadFailure::Content(ContentError::FormedCapacity(_))
            )
        })
    {
        return Ok(Check::ExceedsCapacity);
    }
    let opener = Item::first(input)
        .map_err(io::Error::other)?
        .ok_or_else(|| io::Error::other("a readable canonical segment has an opener"))?;
    let header = Header::decode(opener).map_err(io::Error::other)?;
    if header.registry_version().get() != REGISTRY_VERSION || header.lineage().is_some() {
        return Ok(Check::Failed {
            offset,
            position: 0,
            difference: CanonicalStructureDifference::Header,
        });
    }
    let extent = opener.bytes().len();
    let source = Source {
        tail: &input[extent..],
        registry: header.registry_version(),
        offset: offset + extent,
        position: 1,
        over_span: extent % SPAN_BYTES,
        fold,
    };
    if let Some(at) = first_counted(fold) {
        return first_before_failure(source, at);
    }
    let opener = header
        .identifier()
        .ok_or_else(|| io::Error::other("the canonical opener hash is unavailable"))?;
    let mut comparison = Compare {
        source,
        labelled: labelled.is_some(),
        region: PackRegion::Blob,
        expected: Vec::new(),
        difference: None,
    };
    if let Err(error) = write_structure(fold, labelled, opener, &mut comparison, |sink, region| {
        sink.region = region;
    }) {
        return comparison.difference.ok_or_else(|| io::Error::other(error));
    }
    comparison.source.finish()
}

fn first_counted(fold: &RdfGraph<'_>) -> Option<usize> {
    fold.frame_failures
        .iter()
        .map(|error| error.offset)
        .chain(
            fold.frame_wire
                .iter()
                .filter_map(|frame| (frame.link == LinkState::Healed).then_some(frame.offset)),
        )
        .chain(fold.term_order_violations.iter().copied())
        .chain(fold.row_order_violations.iter().copied())
        .chain(fold.blob_length_conflicts.iter().copied())
        .chain(fold.unreferenced_units.iter().map(|unit| unit.offset))
        .chain(fold.invalid_commits.iter().map(|&(offset, _)| offset))
        .chain(fold.interrupted_commits.iter().map(|&(offset, _)| offset))
        .chain(fold.index_checks.iter().filter_map(|index| {
            let failed = index.prefix.as_ref().map_or(true, |prefix| {
                !prefix.head_matches
                    || !prefix.root_matches
                    || !prefix.type_runs_match
                    || prefix.graph_directory_matches == Some(false)
                    || prefix.pack_members_match == Some(false)
            });
            failed.then_some(index.offset)
        }))
        .filter(|&offset| {
            fold.frame_wire
                .binary_search_by_key(&offset, |frame| frame.offset)
                .ok()
                .is_none_or(|frame| {
                    fold.frame_wire[frame].frame_type != Some(FrameType::Parity as u64)
                })
        })
        .min()
}

fn first_before_failure(
    mut source: Source<'_, '_>,
    at: usize,
) -> io::Result<CanonicalStructureCheck> {
    let closing_index = source
        .fold
        .frame_wire
        .iter()
        .rfind(|frame| frame.frame_type == Some(FrameType::Index as u64))
        .map(|frame| frame.offset);
    let mut previous = None;
    loop {
        let found = source.take()?;
        if found.offset >= at {
            return Ok(found.fail(found.kind.difference()));
        }
        if found.kind >= Kind::LooseValue
            || previous.is_some_and(|kind| {
                kind > found.kind || (kind == found.kind && kind >= Kind::Commit)
            })
        {
            return Ok(found.wrong_kind());
        }
        if found.kind <= Kind::Index {
            let length = found
                .item
                .expect("an ordinary frame has a complete item")
                .bytes()
                .len();
            if let Some(found) = source.parity_after(length, closing_index == Some(found.offset))? {
                return Ok(if found.offset >= at {
                    found.fail(found.kind.difference())
                } else {
                    found.wrong_kind()
                });
            }
        }
        previous = Some(found.kind);
    }
}

struct Compare<'a, 'fold> {
    source: Source<'a, 'fold>,
    labelled: bool,
    region: PackRegion,
    expected: Vec<u8>,
    difference: Option<CanonicalStructureCheck>,
}

impl Write for Compare<'_, '_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.expected
            .try_reserve(bytes.len())
            .map_err(io::Error::other)?;
        self.expected.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        let expected = Item::first(&self.expected)
            .map_err(io::Error::other)?
            .ok_or_else(|| io::Error::other("canonical formation flushed an empty frame"))?;
        let expected = envelope(expected)?;
        let kind = frame_type(&expected).ok_or_else(|| {
            io::Error::other("canonical formation produced an unknown frame type")
        })?;
        let actual = self.source.take()?;
        if actual.kind != Kind::frame(kind, self.region) {
            self.difference = Some(actual.wrong_kind());
        } else {
            let frame = envelope(actual.item.expect("a named frame has a complete item"))?;
            if !frame_matches(
                &expected,
                &frame,
                kind == FrameType::Index,
                self.labelled,
                self.source.registry,
            )? {
                self.difference = Some(actual.fail(actual.kind.difference()));
            } else if let Some(found) = self
                .source
                .parity_after(frame.bytes().len(), kind == FrameType::Index)?
            {
                self.difference = Some(found.wrong_kind());
            }
        }
        if self.difference.is_some() {
            // Stop forming expected frames once the first difference is known.
            Err(io::Error::other("the canonical structure differs"))
        } else {
            self.expected.clear();
            Ok(())
        }
    }
}

fn envelope(item: Item<'_>) -> io::Result<FrameEnvelope<'_>> {
    item.frame()
        .map_err(io::Error::other)?
        .ok_or_else(|| io::Error::other("a named canonical frame has no envelope"))
}

fn frame_matches(
    expected: &FrameEnvelope<'_>,
    actual: &FrameEnvelope<'_>,
    index: bool,
    labelled: bool,
    registry: NonZeroU64,
) -> io::Result<bool> {
    if [
        FrameField::Critical,
        FrameField::Transparent,
        FrameField::Content,
    ]
    .into_iter()
    .any(|field| expected.field(field) != actual.field(field))
    {
        return Ok(false);
    }
    let expected = expected
        .read_field(FrameField::Payload)
        .bytes()
        .map_err(io::Error::other)?;
    // The fold already admitted this payload. This is a bounded comparison
    // of one frame, with no new charge against the original read.
    let mut budget = PayloadBudget::new(PayloadCapacity {
        stored_bytes_per_frame: actual.bytes().len() as u64,
        decoded_bytes_per_frame: actual
            .read_field(FrameField::DecodedLength)
            .uint()
            .map_err(io::Error::other)?,
        decoded_bytes_per_read: u64::MAX,
    });
    let actual = actual
        .check(&[0; 32], None, registry, &mut budget)
        .map_err(io::Error::other)?;
    let PayloadState::Verified(payload) = actual.payload.state else {
        return Ok(false);
    };
    if index {
        return index_matches(expected, &payload, labelled);
    }
    Ok(expected == payload.as_ref())
}

fn index_matches(expected: &[u8], actual: &[u8], labelled: bool) -> io::Result<bool> {
    fn fields(bytes: &[u8]) -> Result<[&[u8]; 6], crate::v2::WireError> {
        let mut fields = Fields::new(bytes);
        fields.record(6)?;
        let mut result = [&[][..]; 6];
        for field in &mut result {
            let start = fields.offset();
            fields.closed_value()?;
            *field = &bytes[start..fields.offset()];
        }
        Ok(result)
    }
    let expected = fields(expected).map_err(io::Error::other)?;
    let actual = fields(actual).map_err(io::Error::other)?;
    for index in [0, 1, 3, 5] {
        if expected[index] != actual[index] {
            return Ok(false);
        }
    }
    // Index content digest bytes belong to the labelling level.
    Ok(!labelled || (expected[2] == actual[2] && actual[4].starts_with(&[0x82, 2])))
}
