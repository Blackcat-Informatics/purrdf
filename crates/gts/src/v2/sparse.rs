// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::collections::TryReserveError;
use std::io;

use purrdf_lex::{allocation::try_boxed, cbor::StructuralError};

use crate::FastMap;
use crate::v2::fields::{Fields, push};
use crate::v2::read::blob::UnitDeclaration;
use crate::v2::{
    ContentError, DIGEST_LENGTH, FoldError, FrameField, FrameType, Header, Item, ItemKind,
    PayloadResourceError, ReadCapacity, ReadFailure, WireError,
};

mod claims;
mod frame;
mod graph;
mod input;
mod opening;
mod unit;

use claims::Claims;
use frame::{FrameHead, LocalFrame, Strings, predecessor, verify};
pub use graph::{RangeGraph, RangeRow, RangeTerm};
use input::{Input, ItemInput};
use opening::{IndexArea, Layout, index_area, locate};
pub use unit::RangeUnit;
use unit::{loose_unit, pack_units};

/// The reason a range read could not answer its query.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RangeReason {
    /// The requested end does not have a recognized locator.
    Locator,
    /// The named index or the items following it cannot be walked.
    IndexItem,
    /// The encoded offsets are invalid or disagree with the walked suffix.
    Offsets,
    /// The advertised segment opener is unavailable or unreadable.
    Header,
    /// The predecessor or the local frame link could not verify.
    Identifier,
    /// The frame envelope, payload or content failed its local rules.
    Payload,
    /// The index's claims fail their intrinsic order or range rules.
    IndexClaims,
    /// The index covers sealed content.
    Sealed,
    /// A candidate field identifies a frame outside the reader's registry.
    FrameType,
    /// A verified frame contradicts the index claim that selected it.
    Contradiction,
    /// Reading or decoding the next range would exceed capacity.
    Capacity,
    /// A unit query has the wrong digest length.
    Query,
    /// Suppression requires unavailable bytes of a referenced value.
    Value,
    /// Covered items could not be placed at their claimed frame positions.
    Placement,
    /// The holder could not supply the selected byte range.
    Read,
}

#[derive(Debug)]
enum Stop {
    Unknown(RangeReason),
    AddressSpace,
    Structure(StructuralError),
    DecodeResource(PayloadResourceError),
    Reserve(TryReserveError),
}

impl From<WireError> for Stop {
    fn from(_: WireError) -> Self {
        Self::Unknown(RangeReason::Payload)
    }
}

impl From<TryReserveError> for Stop {
    fn from(cause: TryReserveError) -> Self {
        Self::Reserve(cause)
    }
}

impl Stop {
    fn content(error: ReadFailure) -> Self {
        match error {
            ReadFailure::Content(ContentError::FormedCapacity(_)) => {
                Self::Unknown(RangeReason::Capacity)
            }
            ReadFailure::Content(ContentError::Resource(error))
            | ReadFailure::Fold(FoldError::Resource(error)) => Self::Reserve(error),
            _ => Self::Unknown(RangeReason::Payload),
        }
    }

    fn content_error(error: ContentError) -> Self {
        Self::content(ReadFailure::Content(error))
    }

    fn fold_error(error: FoldError) -> Self {
        Self::content(ReadFailure::Fold(error))
    }
}

/// The requested graph or unit within the selected segment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RangeTarget<'a> {
    /// Rows of the default graph or the graph named by this IRI text.
    Graph(Option<&'a str>),
    /// Verified units whose digest equals these bytes.
    Unit(&'a [u8]),
}

/// Select an indexed segment and one graph or unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RangeQuery<'a> {
    /// Offset just after the selected segment's closing locator.
    pub segment_end: u64,
    /// Content requested from the index's covered frames.
    pub target: RangeTarget<'a>,
    /// Apply local suppressions to graph rows.
    pub suppression: bool,
}

/// The segment whose opener the range procedure reached.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct RangeScope {
    /// Absolute offset of the segment's opener.
    pub header_offset: u64,
    /// Absolute offset of the index named by the locator.
    pub index_offset: u64,
    /// The frame count advertised by a locally verified index.
    pub frame_count: Option<u64>,
}

/// The items of an answered range query.
#[derive(Debug)]
#[non_exhaustive]
pub enum RangeContent {
    /// Verified rows and their segment-local dictionary.
    Graph(Box<RangeGraph>),
    /// Verified units in item and member order.
    Units(Vec<RangeUnit>),
}

/// The result of selecting one graph or unit through a segment's index.
#[derive(Debug)]
#[non_exhaustive]
pub struct RangeAnswer<'a> {
    /// Registry version used by the query.
    pub registry_version: u64,
    /// The capacities applied to this read.
    pub capacity: ReadCapacity,
    /// The original query.
    pub query: RangeQuery<'a>,
    /// Segment and index evidence available before any refusal.
    pub scope: Option<RangeScope>,
    /// Locally verified items or the first condition that prevented an answer.
    pub outcome: Result<RangeContent, RangeReason>,
}

/// A host failure that prevents a sparse reader from returning an answer.
#[derive(Debug)]
#[non_exhaustive]
pub enum RangeReadError {
    /// A selected range cannot be represented in this address space.
    AddressSpace,
    /// Locating CBOR item structure exhausted host resources.
    Structure(StructuralError),
    /// The host could not allocate a decoded payload.
    DecodeResource(PayloadResourceError),
    /// The host could not retain query state.
    Resource(TryReserveError),
}

impl core::fmt::Display for RangeReadError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::AddressSpace => f.write_str("the GTS range exceeds the host address space"),
            Self::Structure(error) => error.fmt(f),
            Self::DecodeResource(error) => error.fmt(f),
            Self::Resource(error) => error.fmt(f),
        }
    }
}

purrdf_lex::error_source!(RangeReadError {
    Structure,
    DecodeResource,
    Resource
});

/// Use the closing index and short range reads to answer a GTS v2 query.
///
/// The holder must return exactly the requested bytes from one immutable file
/// of the supplied length. The result establishes local frame verification.
/// # Errors
/// Returns a host resource error when the read cannot produce an answer.
pub fn read_range(
    length: u64,
    query: RangeQuery<'_>,
    capacity: ReadCapacity,
    reader: impl FnMut(u64, &mut [u8]) -> io::Result<()>,
) -> Result<RangeAnswer<'_>, RangeReadError> {
    let mut read = QueryRead {
        input: Input::new(reader, capacity.payload.decoded_bytes_per_read),
        length,
        capacity,
        query,
        scope: None,
    };
    let outcome = match read.answer() {
        Ok(content) => Ok(content),
        Err(Stop::Unknown(reason)) => Err(reason),
        Err(Stop::AddressSpace) => return Err(RangeReadError::AddressSpace),
        Err(Stop::Structure(cause)) => return Err(RangeReadError::Structure(cause)),
        Err(Stop::DecodeResource(cause)) => return Err(RangeReadError::DecodeResource(cause)),
        Err(Stop::Reserve(cause)) => return Err(RangeReadError::Resource(cause)),
    };
    Ok(RangeAnswer {
        registry_version: crate::v2::REGISTRY_VERSION,
        capacity,
        query,
        scope: read.scope,
        outcome,
    })
}

struct QueryRead<'a, R> {
    input: Input<R>,
    length: u64,
    capacity: ReadCapacity,
    query: RangeQuery<'a>,
    scope: Option<RangeScope>,
}

struct OpenSegment {
    index: IndexArea,
    opener: [u8; DIGEST_LENGTH],
}

#[derive(Clone, Copy)]
struct Covered {
    item: usize,
    kind: FrameType,
}

struct PlacedIndex {
    layout: Layout,
    claims: Claims,
    covered: Vec<Covered>,
    items: FastMap<usize, ItemInput>,
    opener: [u8; DIGEST_LENGTH],
}

impl<R> QueryRead<'_, R>
where
    R: FnMut(u64, &mut [u8]) -> io::Result<()>,
{
    fn answer(&mut self) -> Result<RangeContent, Stop> {
        let open = self.open_segment()?;
        let mut placed = self.verify_index(open)?;
        match self.query.target {
            RangeTarget::Graph(target) => {
                let graph = self.graph(&mut placed, target)?;
                Ok(RangeContent::Graph(try_boxed(graph)?))
            }
            RangeTarget::Unit(digest) => {
                let digest = digest
                    .try_into()
                    .map_err(|_| Stop::Unknown(RangeReason::Query))?;
                self.units(&mut placed, digest).map(RangeContent::Units)
            }
        }
    }

    fn open_segment(&mut self) -> Result<OpenSegment, Stop> {
        let (index_offset, locator) = locate(&mut self.input, self.length, self.query.segment_end)?;
        let index = index_area(
            &mut self.input,
            index_offset,
            locator,
            self.capacity.items,
            self.capacity.payload,
        )?;
        let extent = index.layout.items[0].clone();
        let header_offset = extent.start;
        let length = extent.end - extent.start;
        let invalid = RangeReason::Header;
        let bytes =
            ItemInput::new(extent).bytes(&mut self.input, header_offset, length, invalid)?;
        let item = Item::first(&bytes)
            .map_err(|_| Stop::Unknown(invalid))?
            .filter(|item| item.bytes().len() == bytes.len())
            .ok_or(Stop::Unknown(invalid))?;
        let registry = core::num::NonZeroU64::MIN;
        let header = match item.classify(registry) {
            Ok(ItemKind::Header) => Header::decode(item),
            Ok(ItemKind::Resync) => Header::decode_resync(item),
            _ => return Err(Stop::Unknown(invalid)),
        }
        .map_err(|_| Stop::Unknown(invalid))?;
        let opener = header
            .identifier()
            .filter(|_| header.registry_version().get() <= crate::v2::REGISTRY_VERSION)
            .ok_or(Stop::Unknown(invalid))?;
        self.scope = Some(RangeScope {
            header_offset,
            index_offset,
            frame_count: None,
        });
        if let RangeTarget::Unit(digest) = self.query.target
            && digest.len() != DIGEST_LENGTH
        {
            return Err(Stop::Unknown(RangeReason::Query));
        }
        Ok(OpenSegment { index, opener })
    }

    fn verify_index(&mut self, open: OpenSegment) -> Result<PlacedIndex, Stop> {
        let OpenSegment { index, opener } = open;
        let IndexArea {
            layout,
            mut index,
            head,
        } = index;
        let mut items = FastMap::default();
        let previous = self.local_predecessor(&layout, &mut items, layout.index, &opener)?;
        let registry = core::num::NonZeroU64::MIN;
        head.check_full(&mut self.input, &mut index, registry)?;
        let frame = verify(
            &mut self.input,
            &mut index,
            &head,
            &previous,
            self.capacity.payload,
            registry,
        )?;
        let (count, _) = frame.content().count_and_digest()?;
        let claims = Claims::read(frame.payload(), registry)?;
        self.scope
            .as_mut()
            .expect("a verified index follows an admitted segment opener")
            .frame_count = Some(count);
        claims.verify(count)?;
        let slots = layout.covered(count)?;
        let kinds = claims.runs.iter().flat_map(|run| {
            // Each positive run fits the checked covered frame count.
            std::iter::repeat_n(run.kind, run.count as usize)
        });
        let mut covered = Vec::new();
        covered.try_reserve_exact(slots.len())?;
        covered.extend(
            slots
                .into_iter()
                .zip(kinds)
                .map(|(item, kind)| Covered { item, kind }),
        );
        Ok(PlacedIndex {
            layout,
            claims,
            covered,
            items,
            opener,
        })
    }

    fn local_predecessor(
        &mut self,
        layout: &Layout,
        items: &mut FastMap<usize, ItemInput>,
        item: usize,
        opener: &[u8; DIGEST_LENGTH],
    ) -> Result<[u8; DIGEST_LENGTH], Stop> {
        if item == 1 {
            return Ok(*opener);
        }
        let previous = item - 1;
        predecessor(&mut self.input, Self::item(layout, items, previous)?)
    }

    fn item<'a>(
        layout: &Layout,
        items: &'a mut FastMap<usize, ItemInput>,
        item: usize,
    ) -> Result<&'a mut ItemInput, Stop> {
        if !items.contains_key(&item) {
            items.try_reserve(1)?;
        }
        Ok(items
            .entry(item)
            .or_insert_with(|| ItemInput::new(layout.items[item].clone())))
    }

    fn fetch(&mut self, placed: &mut PlacedIndex, covered: Covered) -> Result<LocalFrame, Stop> {
        let PlacedIndex {
            layout,
            items,
            opener,
            ..
        } = placed;
        let previous = self.local_predecessor(layout, items, covered.item, opener)?;
        let item = Self::item(layout, items, covered.item)?;
        let capacity = self.capacity.payload;
        let head = FrameHead::read(
            &mut self.input,
            item,
            Some(covered.kind),
            Strings::Full {
                stored_limit: capacity.stored_bytes_per_frame,
            },
            RangeReason::Placement,
        )?;
        let registry = core::num::NonZeroU64::MIN;
        head.check_full(&mut self.input, item, registry)?;
        head.admit_payload(&mut self.input, item, capacity, RangeReason::Payload)?;
        head.acquire(&mut self.input, item, capacity)?;
        verify(&mut self.input, item, &head, &previous, capacity, registry)
    }

    fn units(
        &mut self,
        placed: &mut PlacedIndex,
        digest: &[u8; DIGEST_LENGTH],
    ) -> Result<Vec<RangeUnit>, Stop> {
        let members = &placed.claims.members;
        let start = members.partition_point(|member| member.digest < *digest);
        let end = members.partition_point(|member| member.digest <= *digest);
        for member in &members[start..end] {
            let kind = usize::try_from(member.position)
                .ok()
                .and_then(|index| placed.covered.get(index))
                .map(|entry| entry.kind);
            if kind != Some(FrameType::Pack) {
                return Err(Stop::Unknown(RangeReason::IndexClaims));
            }
        }
        let mut chosen = Vec::new();
        let mut next_member = start;
        for (ordinal, &covered) in placed.covered.iter().enumerate() {
            let first_member = next_member;
            while next_member < end && members[next_member].position == ordinal as u64 {
                next_member += 1;
            }
            let matching = match covered.kind {
                FrameType::Chunk | FrameType::Blob => {
                    self.unit_head_digest(&placed.layout, &mut placed.items, covered)? == *digest
                }
                FrameType::Pack => first_member != next_member,
                _ => false,
            };
            if matching {
                push(&mut chosen, (covered, first_member..next_member))?;
            }
        }
        let mut units = Vec::new();
        for (covered, members) in chosen {
            let frame = self.fetch(placed, covered)?;
            match covered.kind {
                FrameType::Chunk | FrameType::Blob => {
                    push(
                        &mut units,
                        loose_unit(&frame, covered.kind, digest, covered.item)?,
                    )?;
                }
                FrameType::Pack => {
                    let claimed = placed.claims.members[members]
                        .iter()
                        .map(|member| member.member);
                    pack_units(
                        &frame,
                        core::num::NonZeroU64::MIN,
                        digest,
                        covered.item,
                        claimed,
                        &mut units,
                    )?;
                }
                _ => unreachable!("the unit target plan only admits chunks, blobs, and packs"),
            }
        }
        Ok(units)
    }

    fn unit_head_digest(
        &mut self,
        layout: &Layout,
        items: &mut FastMap<usize, ItemInput>,
        covered: Covered,
    ) -> Result<[u8; DIGEST_LENGTH], Stop> {
        let item = Self::item(layout, items, covered.item)?;
        let head = FrameHead::read(
            &mut self.input,
            item,
            Some(covered.kind),
            Strings::Content,
            RangeReason::Placement,
        )?;
        head.check_unit_head(&mut self.input, item, covered.kind)?;
        let content = head
            .field(FrameField::Content)
            .expect("a frame head locates its content field");
        let bytes = item.bytes(
            &mut self.input,
            content.start,
            content.end - content.start,
            RangeReason::Payload,
        )?;
        let mut fields = Fields::new(&bytes);
        let declaration = UnitDeclaration::read(&mut fields, covered.kind == FrameType::Blob)?;
        Ok(declaration.digest)
    }
}
