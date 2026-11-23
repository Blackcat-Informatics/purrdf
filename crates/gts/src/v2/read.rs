// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

#![expect(
    clippy::result_large_err,
    reason = "preserve native failure data without allocating on the refusal path"
)]

use core::fmt;
use core::num::NonZeroU64;
use core::ops::Range;
use std::borrow::Cow;
use std::collections::VecDeque;

use purrdf_lex::cbor::{DecodeErrorKind, StructuralError};

use super::frame::FrameWire;
use super::index::StorageIndex;
use super::parity::{ParityResumption, ParityStream};
use super::{
    CommitCheck, ContentError, DIGEST_LENGTH, DefinitionFindings, FoldError, FormedBudget,
    FrameCheck, FrameEnvelope, FrameField, FrameType, Generation, Header, IndexPrefixCheck,
    IndexStorageFinding, IndexStorageItem, Item, ItemKind, LinkState, MalformedReason, ParityCheck,
    ParityFinding, ParitySolveError, PayloadBudget, PayloadCapacity, PayloadDamage, PayloadRefusal,
    PayloadState, RESYNC_MAX_BYTES, RdfFold, ResyncCandidate, ResyncScan, RowFindings,
    SegmentCheckError, SegmentDictionary, SegmentVerifier, StoredState, ValueName, ValueStore,
    WireError,
};
use crate::{FastMap, model::Graph};

mod blob;

pub use blob::ExternalBlob;

/// Limits for materializing an RDF graph from a GTS v2 file.
#[derive(Clone, Copy, Debug)]
pub struct ReadCapacity {
    /// Maximum stored and decoded payload lengths.
    pub payload: PayloadCapacity,
    /// Maximum total bytes copied by front coding.
    pub formed_bytes: u64,
    /// Maximum total bytes admitted for recipe and value rebuilds.
    pub rebuilt_bytes: u64,
    /// Maximum number of CBOR items delimited.
    pub items: u64,
}

impl Default for ReadCapacity {
    fn default() -> Self {
        Self {
            payload: PayloadCapacity {
                stored_bytes_per_frame: 1 << 26,
                decoded_bytes_per_frame: 1 << 26,
                decoded_bytes_per_read: 1 << 36,
            },
            formed_bytes: 1 << 37,
            rebuilt_bytes: 1 << 37,
            items: 1 << 26,
        }
    }
}

/// A GTS file decoded into the graph model of its generation.
#[derive(Debug)]
#[non_exhaustive]
pub enum ReadGraph<'a> {
    /// The legacy reader's original result, including its diagnostics.
    First(Graph),
    /// The resolved GTS v2 RDF lines and blob declarations.
    Second(RdfGraph<'a>),
}

/// The RDF and blob declaration projection of a generation 2 file.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct RdfGraph<'a> {
    /// Readable RDF terms and resolved RDF lines.
    pub rdf: RdfFold,
    /// Verified blob declarations in file order.
    pub blobs: Vec<VerifiedBlob<'a>>,
    /// References to named blobs declared without inline bytes.
    pub external_blobs: Vec<ExternalBlob<'a>>,
    /// File offsets of row frames whose declared order differs from their rows.
    pub row_order_violations: Vec<usize>,
    /// File offsets whose stored check differs from the compressed bytes.
    pub stored_digest_mismatches: Vec<usize>,
    /// Damaged payloads and rows held by unreadable term definitions.
    pub frame_failures: Vec<ReadError>,
    /// Scanned resync openers and the framing damage before them, in file order.
    pub resynchronizations: Vec<Resynchronization>,
    /// Damaged regions ended by verified parity frames, in file order.
    pub parity_recoveries: Vec<ParityRecovery>,
    /// Commits that cannot verify the ordinary prefix across a parity recovery.
    pub interrupted_commits: Vec<(usize, CommitCheck)>,
    /// Parity diagnostics and repair outcomes, followed by each segment's tail.
    pub parity_findings: Vec<ParityFinding>,
    /// A repaired copy when any span passes verification.
    pub repaired: Option<Vec<u8>>,
    /// Closing or intermediate index checks in file order.
    pub index_checks: Vec<GraphIndexCheck>,
    /// False or misplaced offsets items and locators.
    pub index_storage_findings: Vec<IndexStorageFinding>,
}

/// The prefix check of an index encountered during a graph read.
#[derive(Debug)]
#[non_exhaustive]
pub struct GraphIndexCheck {
    /// Physical offset of the index frame.
    pub offset: usize,
    /// Prefix comparisons or the failure that prevented them.
    pub prefix: Result<IndexPrefixCheck, ReadFailure>,
}

/// A verified resync opener reached by scanning after framing failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Resynchronization {
    /// Byte offset of the accepted opener.
    pub offset: usize,
    /// Skipped bytes when the opener starts inside the failed item.
    pub damage: Option<FramingDamage>,
}

/// A verified parity frame reached after an undelimitable region.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ParityRecovery {
    /// Byte range skipped before the accepted parity frame.
    pub damage: FramingDamage,
    /// File offset of the parity frame that continued the segment.
    pub offset: usize,
    /// Whether the parity anchor differs from the preceding known head.
    pub detached: bool,
}

/// The extent and kind of bytes skipped when resynchronizing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FramingDamage {
    /// Bytes that end inside an otherwise structurally valid CBOR item.
    TornTail {
        /// Byte offset of the torn item.
        offset: usize,
        /// Number of skipped bytes.
        length: usize,
    },
    /// Bytes that contain a structural CBOR violation.
    Undelimitable {
        /// Byte offset of the undelimitable item.
        offset: usize,
        /// Number of skipped bytes.
        length: usize,
    },
}

/// The declared media type and verified bytes of an embedded blob.
#[derive(Debug)]
#[non_exhaustive]
pub struct VerifiedBlob<'a> {
    /// Blob identity and length.
    pub name: ValueName,
    /// Media type from the declaration.
    pub media_type: Option<Cow<'a, str>>,
    /// Verified payload, borrowed for plain storage.
    pub bytes: Cow<'a, [u8]>,
}

/// A graph read could not verify or interpret the item at this byte offset.
#[derive(Debug)]
pub struct ReadError {
    /// File-relative start of the affected item.
    pub offset: usize,
    /// The framing, verification or interpretation failure.
    pub failure: ReadFailure,
}

/// The reason an item could not be used to materialize a graph.
#[derive(Debug)]
#[non_exhaustive]
pub enum ReadFailure {
    /// No opener was found.
    Empty,
    /// The first item's generation cannot be read.
    Generation(Generation),
    /// The CBOR item could not be delimited.
    Framing(StructuralError),
    /// The item is outside the item count capacity.
    ItemCapacity(u64),
    /// The item failed the structural classification rules.
    Classification(MalformedReason),
    /// A header or envelope field failed validation.
    Wire(WireError),
    /// The registry code is newer than this graph reader.
    Registry(u64),
    /// Lineage does not match the previous segment.
    Lineage,
    /// An item kind requires additional interpretation.
    Item(ItemKind),
    /// A frame type requires additional interpretation.
    Frame(FrameType),
    /// The link to the preceding frame is broken.
    BrokenLink,
    /// The decoded payload failed verification.
    PayloadDamage(PayloadDamage),
    /// The payload could not be checked.
    PayloadRefusal(PayloadRefusal),
    /// The segment verifier could not complete a check.
    Segment(SegmentCheckError),
    /// Parity reconstruction could not run.
    ParityRepair(ParitySolveError),
    /// The commit disagrees with the computed prefix.
    Commit,
    /// The content of a verified frame is malformed or unavailable.
    Content(ContentError),
    /// The verification result has no content for this frame.
    MissingContent,
    /// Some definitions could not be read.
    Definitions(DefinitionFindings),
    /// Some rows could not be resolved.
    Rows(RowFindings),
    /// The annotation multiplicity is wider than the wire format.
    AnnotationWidth,
    /// The declared unit or blob identity disagrees with its decoded bytes.
    BlobDigest,
    /// No verified unit was available to a recipe entry.
    MissingUnit(ValueName),
    /// The recipe entry's length disagrees with the unit it names.
    UnitLength(ValueName),
    /// The recipe or value exceeds the remaining rebuild capacity.
    RebuildCapacity(ValueName),
    /// The graph could not retain the verified content.
    Fold(FoldError),
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GTS graph read at byte {}: ", self.offset)?;
        match &self.failure {
            ReadFailure::Framing(error) => error.fmt(f),
            ReadFailure::Wire(error) => error.fmt(f),
            ReadFailure::Segment(error) => error.fmt(f),
            ReadFailure::ParityRepair(error) => error.fmt(f),
            ReadFailure::Content(error) => error.fmt(f),
            ReadFailure::Fold(error) => error.fmt(f),
            failure => write!(f, "{failure:?}"),
        }
    }
}

impl std::error::Error for ReadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.failure {
            ReadFailure::Framing(error) => Some(error),
            ReadFailure::Wire(error) => Some(error),
            ReadFailure::Segment(error) => Some(error),
            ReadFailure::ParityRepair(error) => Some(error),
            ReadFailure::Content(error) => Some(error),
            ReadFailure::Fold(error) => Some(error),
            ReadFailure::PayloadDamage(PayloadDamage::DecodeFailed(error)) => Some(error),
            _ => None,
        }
    }
}

/// Decode a GTS file into the graph model of its opening generation.
///
/// The first generation uses the existing reader over the original bytes.
/// The generation 2 graph projection checks framing, links, payloads, commits
/// and RDF positions. Frames with damaged payloads and rows held by unreadable
/// definitions stay outside the fold and carry findings. An item that requires
/// unsupported content interpretation prevents a complete graph projection.
/// The reader can recover at a verified resync opener after failure of the
/// framing test or opening generation recognition. A verified parity frame
/// can resume a readable segment after undelimitable bytes. An ordinary frame
/// whose healing witness is discarded prevents a complete graph projection.
/// Parity findings describe the supplied bytes before any repair.
/// Indexes report comparisons against the observed prefix and never supply
/// graph content.
/// The graph carries no assertion about file authenticity or canonical form.
///
/// # Errors
/// Returns the byte position and reason when a graph cannot be produced.
pub fn read(bytes: &[u8], capacity: ReadCapacity) -> Result<ReadGraph<'_>, ReadError> {
    if capacity.items == 0 && !bytes.is_empty() {
        return Err(ReadError {
            offset: 0,
            failure: ReadFailure::ItemCapacity(0),
        });
    }
    let opener = match delimit(bytes, 0) {
        Ok(Some(item)) => item,
        Ok(None) => {
            return Err(ReadError {
                offset: 0,
                failure: ReadFailure::Empty,
            });
        }
        Err(error) => {
            return GraphReader::new(bytes, capacity)
                .recover_initial(error, None)
                .map(ReadGraph::Second);
        }
    };
    let generation = opener.generation().map_err(|error| ReadError {
        offset: 0,
        failure: ReadFailure::Framing(error),
    })?;
    match generation {
        Generation::First => Ok(ReadGraph::First(crate::reader::read(bytes, true, None))),
        Generation::Second | Generation::Unrecognized => GraphReader::new(bytes, capacity)
            .read_first(opener, generation)
            .map(ReadGraph::Second),
        generation => Err(ReadError {
            offset: 0,
            failure: ReadFailure::Generation(generation),
        }),
    }
}

fn delimit(bytes: &[u8], offset: usize) -> Result<Option<Item<'_>>, ReadError> {
    Item::first(&bytes[offset..]).map_err(|error| ReadError {
        offset,
        failure: ReadFailure::Framing(error),
    })
}

struct Segment<'a> {
    verifier: SegmentVerifier,
    dictionary: SegmentDictionary,
    ordinal: Option<u64>,
    file_ordinal: u64,
    units: FastMap<[u8; 32], Unit<'a>>,
    parity: ParityStream<'a>,
    index_storage: StorageIndex,
}

enum Unit<'a> {
    Blob(usize),
    Chunk(Cow<'a, [u8]>),
}

struct GraphFrame<'a> {
    offset: usize,
    envelope: FrameEnvelope<'a>,
    checked: FrameCheck<'a>,
}

struct FailedItem<'a> {
    failure: ReadFailure,
    accelerator: Option<GraphFrame<'a>>,
}

enum Recovery<'a> {
    Resync(ResyncCandidate<'a>),
    Parity {
        offset: usize,
        region_counted: bool,
        candidate: ParityResumption<'a>,
    },
}

#[derive(Default)]
struct HeldItems<'a> {
    complete: VecDeque<Item<'a>>,
    complete_end: usize,
    next: Option<Result<Option<Item<'a>>, ReadError>>,
    region_counted: bool,
}

impl<'a> HeldItems<'a> {
    fn peek(&mut self, reader: &GraphReader<'a>, offset: usize) -> Option<Item<'a>> {
        if let Some(item) = self.complete.front() {
            return Some(*item);
        }
        self.next
            .get_or_insert_with(|| reader.next_item(offset))
            .as_ref()
            .ok()
            .and_then(|item| *item)
    }

    fn take(
        &mut self,
        reader: &mut GraphReader<'a>,
        offset: usize,
    ) -> Result<Option<Item<'a>>, ReadError> {
        if let Some(item) = self.complete.pop_front() {
            return Ok(Some(item));
        }
        let next = self
            .next
            .take()
            .unwrap_or_else(|| reader.next_item(offset))?;
        if next.is_some() {
            reader.count_item(offset)?;
        }
        Ok(next)
    }
}

#[expect(
    clippy::large_enum_variant,
    reason = "keep the prepared frame on the stack without allocating for each item"
)]
enum ItemCheck<'a> {
    Frame(FrameEnvelope<'a>, FrameWire<'a>),
    Opener(Header<'a>),
    StorageClaim(IndexStorageItem, bool),
    Unsupported(ItemKind),
    FramingFailure(ReadFailure),
}

struct GraphReader<'a> {
    bytes: &'a [u8],
    scan: ResyncScan<'a>,
    graph: RdfGraph<'a>,
    values: ValueStore,
    payload: PayloadBudget,
    formed: FormedBudget,
    items: u64,
    item_limit: u64,
    rebuilt_remaining: u64,
}

impl<'a> GraphReader<'a> {
    fn new(bytes: &'a [u8], capacity: ReadCapacity) -> Self {
        Self {
            bytes,
            scan: ResyncScan::new(bytes),
            graph: RdfGraph::default(),
            values: ValueStore::default(),
            payload: PayloadBudget::new(capacity.payload),
            formed: FormedBudget::new(capacity.formed_bytes),
            items: 0,
            item_limit: capacity.items,
            rebuilt_remaining: capacity.rebuilt_bytes,
        }
    }

    fn read_first(
        self,
        opener: Item<'a>,
        generation: Generation,
    ) -> Result<RdfGraph<'a>, ReadError> {
        let end = opener.bytes().len();
        let failure = if generation == Generation::Second {
            match decode_opener(opener) {
                Ok(header) => return self.read_opened(header, 0, end),
                Err(failure) => failure,
            }
        } else {
            ReadFailure::Generation(generation)
        };
        self.recover_initial(ReadError { offset: 0, failure }, Some(end))
    }

    fn recover_initial(
        mut self,
        initial: ReadError,
        framed_end: Option<usize>,
    ) -> Result<RdfGraph<'a>, ReadError> {
        let candidate = if let Some(end) = framed_end {
            self.count_item(0)?;
            self.count_held_items(end, &mut HeldItems::default())?;
            self.scan_resync(0, end)?
        } else if self.count_undelimited(&initial, false)?.is_some() {
            self.scan_resync(0, self.bytes.len())?
        } else {
            None
        };
        let Some(candidate) = candidate else {
            return Err(initial);
        };
        let start = candidate.offset;
        let end = start + candidate.bytes.len();
        let segment = self.resume_segment(initial, &candidate, None)?;
        self.read_segment(segment, start, end)
    }

    fn count_undelimited(
        &mut self,
        failure: &ReadError,
        region_counted: bool,
    ) -> Result<Option<bool>, ReadError> {
        let ReadFailure::Framing(error) = &failure.failure else {
            return Ok(None);
        };
        let counted = match error.kind() {
            DecodeErrorKind::Malformed(_) => {
                if !region_counted {
                    self.count_item(failure.offset)?;
                }
                true
            }
            DecodeErrorKind::Truncated => region_counted,
            _ => return Ok(None),
        };
        Ok(Some(counted))
    }

    fn scan_resync(
        &mut self,
        start: usize,
        end: usize,
    ) -> Result<Option<ResyncCandidate<'a>>, ReadError> {
        if self.items == self.item_limit {
            return Ok(None);
        }
        self.scan
            .scan(start..end, &mut self.payload)
            .map_err(|refused| ReadError {
                offset: refused.offset,
                failure: ReadFailure::PayloadRefusal(PayloadRefusal::TotalDecoded(
                    refused.requested_total,
                )),
            })
    }

    fn scan_recovery(
        &mut self,
        failure: &ReadError,
        scan_start: usize,
        region_counted: bool,
        segment: &Segment<'a>,
    ) -> Result<Option<Recovery<'a>>, ReadError> {
        let Some(region_counted) = self.count_undelimited(failure, region_counted)? else {
            return Ok(None);
        };
        if self.items == self.item_limit || self.payload.total_refused() {
            return Ok(None);
        }
        if let Some(candidate) = self.scan_resync(scan_start, failure.offset)? {
            return Ok(Some(Recovery::Resync(candidate)));
        }
        let parity_slots = if region_counted { 1 } else { 2 };
        let parity_allowed = self.item_limit - self.items >= parity_slots;
        let history = segment.verifier.parity_history();
        for offset in failure.offset..self.bytes.len() {
            // Both candidate types start with a deterministic 11-field array.
            let bytes = &self.bytes[offset..];
            if bytes.first() != Some(&0x8b) {
                continue;
            }
            if let Some(candidate) = self.scan_resync(offset, offset + 1)? {
                return Ok(Some(Recovery::Resync(candidate)));
            }
            if !parity_allowed {
                continue;
            }
            if let Some(candidate) =
                history
                    .scan_at(bytes, &mut self.payload)
                    .map_err(|count| ReadError {
                        offset,
                        failure: ReadFailure::PayloadRefusal(PayloadRefusal::TotalDecoded(count)),
                    })?
            {
                return Ok(Some(Recovery::Parity {
                    offset,
                    region_counted,
                    candidate,
                }));
            }
        }
        Ok(None)
    }

    fn resume_parity(
        &mut self,
        region: Range<usize>,
        region_counted: bool,
        candidate: ParityResumption<'a>,
        segment: &mut Segment<'a>,
    ) -> Result<(), ReadError> {
        let offset = region.end;
        if !region_counted {
            self.count_item(region.start)?;
        }
        self.count_item(offset)?;
        let damage = FramingDamage::Undelimitable {
            offset: region.start,
            length: region.len(),
        };
        let accept = || {
            segment.parity.damaged_region(region)?;
            segment.dictionary.mark_doubt();
            let detached = segment
                .verifier
                .resume_parity(candidate.anchor, &candidate.table)?;
            segment
                .parity
                .other_item(offset..offset + candidate.length)?;
            segment.parity.observe(
                offset,
                offset + candidate.payload_end,
                ParityCheck::Table {
                    table: candidate.table,
                    payload_damaged: false,
                },
            )?;
            super::fields::push(
                &mut self.graph.parity_recoveries,
                ParityRecovery {
                    damage,
                    offset,
                    detached,
                },
            )
        };
        accept().map_err(|error| ReadError {
            offset,
            failure: error.into(),
        })
    }

    fn resume_segment(
        &mut self,
        original: ReadError,
        candidate: &ResyncCandidate<'a>,
        previous: Option<&mut Segment<'a>>,
    ) -> Result<Segment<'a>, ReadError> {
        let offset = original.offset;
        let damage = if candidate.offset > offset {
            let length = candidate.offset - offset;
            let Err(prefix) = delimit(&self.bytes[..candidate.offset], offset) else {
                return Err(original);
            };
            let ReadFailure::Framing(error) = &prefix.failure else {
                return Err(prefix);
            };
            Some(match error.kind() {
                DecodeErrorKind::Truncated => FramingDamage::TornTail { offset, length },
                DecodeErrorKind::Malformed(_) => FramingDamage::Undelimitable { offset, length },
                _ => return Err(prefix),
            })
        } else {
            None
        };
        let start = candidate.offset;
        let end = start + candidate.bytes.len();
        let segment = segment_from_header(candidate.header, start..end, previous.as_deref())
            .map_err(|failure| ReadError {
                offset: candidate.offset,
                failure,
            })?;
        self.count_item(candidate.offset)?;
        if let Some(previous) = previous {
            if let Some(FramingDamage::Undelimitable { offset, length }) = damage {
                previous
                    .parity
                    .damaged_region(offset..offset + length)
                    .map_err(|error| ReadError {
                        offset,
                        failure: error.into(),
                    })?;
            }
            self.finish_segment(previous, start)?;
        }
        super::fields::push(
            &mut self.graph.resynchronizations,
            Resynchronization {
                offset: candidate.offset,
                damage,
            },
        )
        .map_err(|error| ReadError {
            offset: candidate.offset,
            failure: error.into(),
        })?;
        Ok(segment)
    }

    fn count_held_items(
        &mut self,
        failed_end: usize,
        items: &mut HeldItems<'a>,
    ) -> Result<(), ReadError> {
        if items.region_counted {
            return Ok(());
        }
        let limit = failed_end
            .saturating_add(RESYNC_MAX_BYTES)
            .min(self.bytes.len());
        let mut offset = if items.complete.is_empty() {
            failed_end
        } else {
            items.complete_end
        };
        while offset < limit && self.items < self.item_limit {
            // The immediate next item's extent can move the decision point
            // past the byte lookahead. Later items count only within it.
            let full_input = offset == failed_end || limit == self.bytes.len();
            let next = if full_input {
                items.next.take().unwrap_or_else(|| self.next_item(offset))
            } else {
                delimit(&self.bytes[..limit], offset)
            };
            match next {
                Ok(Some(item)) => {
                    items.complete.try_reserve(1).map_err(|error| ReadError {
                        offset,
                        failure: error.into(),
                    })?;
                    self.count_item(offset)?;
                    offset += item.bytes().len();
                    items.complete.push_back(item);
                    items.complete_end = offset;
                }
                Ok(None) => break,
                Err(error) => {
                    let ReadFailure::Framing(cause) = &error.failure else {
                        return Err(error);
                    };
                    match cause.kind() {
                        // A held undelimitable region extends to the end of
                        // the input for the purpose of the item count.
                        DecodeErrorKind::Malformed(_) => {
                            self.count_item(offset)?;
                            items.region_counted = true;
                            items.next = Some(Err(error));
                            return Ok(());
                        }
                        DecodeErrorKind::Truncated => {
                            if full_input {
                                items.next = Some(Err(error));
                            }
                        }
                        _ => return Err(error),
                    }
                    break;
                }
            }
        }
        self.check_item_limit(offset)
    }

    fn read_opened(
        mut self,
        opener: Header<'a>,
        start: usize,
        end: usize,
    ) -> Result<RdfGraph<'a>, ReadError> {
        self.count_item(start)?;
        let segment =
            segment_from_header(opener, start..end, None).map_err(|failure| ReadError {
                offset: start,
                failure,
            })?;
        self.read_segment(segment, start, end)
    }

    fn read_segment(
        mut self,
        mut segment: Segment<'a>,
        mut previous_start: usize,
        mut offset: usize,
    ) -> Result<RdfGraph<'a>, ReadError> {
        let mut items = HeldItems::default();
        let mut held: Option<GraphFrame<'a>> = None;
        loop {
            let item = match items.take(&mut self, offset) {
                Ok(Some(item)) => item,
                Ok(None) => {
                    self.finish_segment(&segment, offset)?;
                    return Ok(self.graph);
                }
                Err(error) => {
                    match self.scan_recovery(
                        &error,
                        previous_start + 1,
                        items.region_counted,
                        &segment,
                    )? {
                        Some(Recovery::Resync(candidate)) => {
                            previous_start = candidate.offset;
                            offset = candidate.offset + candidate.bytes.len();
                            segment = self.resume_segment(error, &candidate, Some(&mut segment))?;
                        }
                        Some(Recovery::Parity {
                            offset: start,
                            region_counted,
                            candidate,
                        }) => {
                            previous_start = start;
                            offset = start + candidate.length;
                            self.resume_parity(
                                error.offset..start,
                                region_counted,
                                candidate,
                                &mut segment,
                            )?;
                        }
                        None => return Err(error),
                    }
                    items = HeldItems::default();
                    continue;
                }
            };
            let end = offset + item.bytes().len();
            let head = held
                .as_ref()
                .and_then(|frame| frame.checked.link.effective_identifier.as_ref())
                .unwrap_or_else(|| segment.verifier.predecessor());
            // The current frame already links under a held predecessor's
            // computed ID. Its successor cannot heal it in that case.
            let witness = if held.is_none() {
                items.peek(&self, end)
            } else {
                None
            };
            let checked = Self::inspect_item(item, witness, head, offset, &segment)
                .map_err(|failure| ReadError { offset, failure })?;
            if let Some(failed) =
                self.observe_item(checked, offset..end, &mut held, &mut segment)?
            {
                let scan_start = if held.is_some() {
                    offset
                } else {
                    previous_start + 1
                };
                let candidate = match self.count_held_items(end, &mut items) {
                    Err(error) if matches!(&error.failure, ReadFailure::ItemCapacity(_)) => {
                        Err(error)
                    }
                    counted => {
                        counted?;
                        self.scan_resync(scan_start, end)
                    }
                };
                if !matches!(candidate, Ok(Some(_))) {
                    self.deliver_held(held.take(), &mut segment)?;
                }
                if let Some(accelerator) = failed.accelerator
                    && (matches!(candidate, Ok(None))
                        || matches!(&candidate, Err(error) if matches!(error.failure, ReadFailure::ItemCapacity(_))))
                {
                    self.deliver_frame(accelerator, &mut segment)?;
                    previous_start = offset;
                    offset = end;
                    continue;
                }
                let original = ReadError {
                    offset,
                    failure: failed.failure,
                };
                let Some(candidate) = candidate? else {
                    return Err(original);
                };
                if let Some(mut frame) = held.take() {
                    // The accepted opener replaces this frame's only witness.
                    if frame.checked.frame_type.and_then(FrameType::from_code)
                        != Some(FrameType::Index)
                    {
                        let failure = frame
                            .checked
                            .malformed
                            .map_or(ReadFailure::BrokenLink, ReadFailure::Wire);
                        return Err(ReadError {
                            offset: frame.offset,
                            failure,
                        });
                    }
                    let link = &mut frame.checked.link;
                    link.state = LinkState::Broken;
                    link.effective_identifier = link.stored_identifier;
                    self.deliver_frame(frame, &mut segment)?;
                }
                previous_start = candidate.offset;
                offset = candidate.offset + candidate.bytes.len();
                segment = self.resume_segment(original, &candidate, Some(&mut segment))?;
                items = HeldItems::default();
                continue;
            }
            previous_start = offset;
            offset = end;
        }
    }

    fn finish_segment(&mut self, segment: &Segment<'a>, offset: usize) -> Result<(), ReadError> {
        let read_error = |error: std::collections::TryReserveError| ReadError {
            offset,
            failure: error.into(),
        };
        segment
            .parity
            .finish(
                self.bytes,
                &mut self.graph.parity_findings,
                &mut self.graph.repaired,
                &mut self.payload,
            )
            .map_err(|failure| ReadError { offset, failure })?;
        segment
            .index_storage
            .finish(
                segment.parity.item_count(),
                &mut self.graph.index_storage_findings,
            )
            .map_err(read_error)
    }

    fn count_item(&mut self, offset: usize) -> Result<(), ReadError> {
        self.check_item_limit(offset)?;
        self.items += 1;
        Ok(())
    }

    fn check_item_limit(&self, offset: usize) -> Result<(), ReadError> {
        if self.items == self.item_limit && offset < self.bytes.len() {
            return Err(ReadError {
                offset,
                failure: ReadFailure::ItemCapacity(self.item_limit),
            });
        }
        Ok(())
    }

    fn next_item(&self, offset: usize) -> Result<Option<Item<'a>>, ReadError> {
        self.check_item_limit(offset)?;
        delimit(self.bytes, offset)
    }

    fn inspect_item(
        item: Item<'a>,
        next: Option<Item<'a>>,
        predecessor: &[u8; DIGEST_LENGTH],
        offset: usize,
        segment: &Segment<'a>,
    ) -> Result<ItemCheck<'a>, ReadFailure> {
        // The graph reader admits only registry version 1 segment openers.
        let registry = NonZeroU64::MIN;
        let kind = match item.classify(registry) {
            Ok(kind) => kind,
            Err(error) => {
                return Ok(ItemCheck::FramingFailure(ReadFailure::Classification(
                    error,
                )));
            }
        };
        match kind {
            ItemKind::Header | ItemKind::Resync => Ok(match decode_opener(item) {
                Ok(header) => ItemCheck::Opener(header),
                Err(error) => ItemCheck::FramingFailure(error),
            }),
            ItemKind::Frame => {
                let envelope = item
                    .frame()
                    .map_err(ReadFailure::Framing)?
                    .ok_or(ReadFailure::MissingContent)?;
                let witness = next
                    .map(Item::frame)
                    .transpose()
                    .map_err(ReadFailure::Framing)?
                    .flatten();
                let wire = envelope.check_wire(predecessor, witness.as_ref(), registry);
                Ok(ItemCheck::Frame(envelope, wire))
            }
            kind @ (ItemKind::Offsets | ItemKind::Locator) => {
                let (storage_item, claim) = if kind == ItemKind::Offsets {
                    (
                        IndexStorageItem::Offsets,
                        item.offsets_match(segment.parity.item_lengths()),
                    )
                } else {
                    (
                        IndexStorageItem::Locator,
                        item.locator_distance().map(|distance| {
                            segment.index_storage.locator_matches(offset, distance)
                        }),
                    )
                };
                Ok(match claim {
                    Ok(matches) => ItemCheck::StorageClaim(storage_item, matches),
                    Err(error) => ItemCheck::FramingFailure(ReadFailure::Wire(error)),
                })
            }
            _ => {
                let framed = item
                    .passes_framing_test(Some((registry, predecessor)), None)
                    .map_err(ReadFailure::Framing)?;
                Ok(if framed {
                    ItemCheck::Unsupported(kind)
                } else {
                    ItemCheck::FramingFailure(ReadFailure::Item(kind))
                })
            }
        }
    }

    fn observe_item(
        &mut self,
        item: ItemCheck<'a>,
        extent: Range<usize>,
        held: &mut Option<GraphFrame<'a>>,
        segment: &mut Segment<'a>,
    ) -> Result<Option<FailedItem<'a>>, ReadError> {
        let offset = extent.start;
        match item {
            ItemCheck::Frame(envelope, wire) => {
                let framed = wire.passes_framing();
                if framed {
                    self.deliver_held(held.take(), segment)?;
                }
                let checked =
                    envelope
                        .check_prepared(wire, &mut self.payload)
                        .map_err(|error| ReadError {
                            offset,
                            failure: ReadFailure::Segment(SegmentCheckError::Payload(error)),
                        })?;
                let frame = GraphFrame {
                    offset,
                    envelope,
                    checked,
                };
                if !framed {
                    // Independent payload checks occur before an attempted scan.
                    let kind = frame.checked.frame_type.and_then(FrameType::from_code);
                    let failed = if let Some(kind @ (FrameType::Index | FrameType::Parity)) = kind {
                        FailedItem {
                            failure: ReadFailure::Frame(kind),
                            accelerator: Some(frame),
                        }
                    } else {
                        FailedItem {
                            failure: admit_checked(frame.checked)
                                .err()
                                .unwrap_or(ReadFailure::MissingContent),
                            accelerator: None,
                        }
                    };
                    return Ok(Some(failed));
                }
                if frame.checked.link.state == LinkState::Healed {
                    // Only the next item's framing can make this link final.
                    *held = Some(frame);
                } else {
                    self.deliver_frame(frame, segment)?;
                }
            }
            ItemCheck::Opener(header) => {
                self.deliver_held(held.take(), segment)?;
                let opened = segment_from_header(header, extent, Some(segment))
                    .map_err(|failure| ReadError { offset, failure })?;
                self.finish_segment(segment, offset)?;
                *segment = opened;
            }
            ItemCheck::StorageClaim(item, matches) => {
                self.deliver_held(held.take(), segment)?;
                let record = || {
                    segment.index_storage.observe(
                        offset,
                        segment.parity.item_count(),
                        item,
                        matches,
                    )?;
                    segment.parity.other_item(extent)
                };
                record().map_err(|error| ReadError {
                    offset,
                    failure: error.into(),
                })?;
            }
            ItemCheck::Unsupported(kind) => {
                self.deliver_held(held.take(), segment)?;
                return Err(ReadError {
                    offset,
                    failure: ReadFailure::Item(kind),
                });
            }
            ItemCheck::FramingFailure(failure) => {
                return Ok(Some(FailedItem {
                    failure,
                    accelerator: None,
                }));
            }
        }
        Ok(None)
    }

    fn deliver_held(
        &mut self,
        held: Option<GraphFrame<'a>>,
        segment: &mut Segment<'a>,
    ) -> Result<(), ReadError> {
        if let Some(held) = held {
            self.deliver_frame(held, segment)?;
        }
        Ok(())
    }

    fn deliver_frame(
        &mut self,
        frame: GraphFrame<'a>,
        segment: &mut Segment<'a>,
    ) -> Result<(), ReadError> {
        let GraphFrame {
            offset,
            envelope,
            checked,
        } = frame;
        self.observe_frame(envelope, checked, offset, segment)
            .map_err(|failure| ReadError { offset, failure })
    }

    fn observe_frame(
        &mut self,
        frame: FrameEnvelope<'a>,
        checked: FrameCheck<'a>,
        offset: usize,
        segment: &mut Segment<'a>,
    ) -> Result<(), ReadFailure> {
        let kind = checked.frame_type.and_then(FrameType::from_code);
        let accelerator = matches!(kind, Some(FrameType::Index | FrameType::Parity));
        let damaged_payload = !accelerator
            && checked
                .malformed
                .is_none_or(|error| error.reason == MalformedReason::CodecConstraint)
            && checked.link.state != LinkState::Broken
            && matches!(checked.payload.state, PayloadState::Damaged(_));
        let checked = if accelerator || damaged_payload {
            checked
        } else {
            admit_checked(checked)?
        };
        if kind != Some(FrameType::Parity) && checked.payload.stored == Some(StoredState::Mismatch)
        {
            super::fields::push(&mut self.graph.stored_digest_mismatches, offset)?;
        }
        let mut checked = segment
            .verifier
            .accept_checked(&frame, checked, &self.payload)
            .map_err(ReadFailure::Segment)?;
        if !frame.transparent() {
            let wire = &checked.frame;
            let passes = wire.malformed.is_none()
                && wire.link.state == LinkState::Linked
                && !checked.index.as_ref().is_some_and(Result::is_err)
                && wire.payload.stored.map_or(
                    matches!(wire.payload.state, PayloadState::Verified(_)),
                    |stored| stored == StoredState::Matches,
                );
            segment.parity.ordinary(
                offset..offset + frame.bytes().len(),
                wire.link.state,
                passes,
                wire.link.effective_identifier,
            )?;
        } else {
            segment
                .parity
                .other_item(offset..offset + frame.bytes().len())?;
        }
        if damaged_payload {
            if let Some(reservation) = &checked.terms {
                segment
                    .dictionary
                    .observe_terms(&mut self.graph.rdf, reservation, None, &self.values)
                    .map_err(ReadFailure::Fold)?;
            }
            super::fields::push(
                &mut self.graph.frame_failures,
                ReadError {
                    offset,
                    failure: checked.frame.malformed.map_or_else(
                        || payload_failure(checked.frame.payload.state),
                        ReadFailure::Wire,
                    ),
                },
            )?;
            return Ok(());
        }
        let kind = kind.ok_or(ReadFailure::MissingContent)?;
        match kind {
            FrameType::Parity => {
                let check = checked.parity.take().ok_or(ReadFailure::MissingContent)?;
                segment.parity.observe(
                    offset,
                    offset + frame.start(FrameField::Identifier),
                    check,
                )?;
            }
            FrameType::Terms => {
                let reservation = checked.terms.as_mut().ok_or(ReadFailure::MissingContent)?;
                reservation.content = reservation
                    .content
                    .take()
                    .transpose()
                    .map_err(ReadFailure::Content)?
                    .map(Ok);
                let decoded = segment
                    .verifier
                    .decode_terms(&checked, &mut self.formed)
                    .ok_or(ReadFailure::MissingContent)?
                    .map_err(ReadFailure::Content)?;
                let reservation = checked.terms.as_ref().ok_or(ReadFailure::MissingContent)?;
                let findings = segment
                    .dictionary
                    .observe_terms(
                        &mut self.graph.rdf,
                        reservation,
                        Some(decoded),
                        &self.values,
                    )
                    .map_err(ReadFailure::Fold)?;
                if findings.ill_formed_term
                    || findings.forward_reference
                    || findings.position_constraint
                    || !findings.missing_values.is_empty()
                {
                    return Err(ReadFailure::Definitions(findings));
                }
            }
            FrameType::Quads
            | FrameType::ReifierBindings
            | FrameType::Annotations
            | FrameType::EmptyGraphs => {
                self.observe_rows(checked, offset, segment)?;
            }
            FrameType::Blob
            | FrameType::Chunk
            | FrameType::Recipe
            | FrameType::Value
            | FrameType::External
            | FrameType::Pack => {
                let bytes = verified_payload(checked.frame.payload.state)?;
                self.observe_blob_frame(
                    kind,
                    frame.read_field(FrameField::Content),
                    bytes,
                    segment,
                )?;
            }
            FrameType::Commit if checked.commit.is_some_and(|commit| commit.valid) => {}
            FrameType::Commit
                if segment.verifier.incomplete_prefix() && checked.commit.is_some() =>
            {
                let commit = checked.commit.ok_or(ReadFailure::MissingContent)?;
                super::fields::push(&mut self.graph.interrupted_commits, (offset, commit))?;
            }
            FrameType::Commit => return Err(ReadFailure::Commit),
            FrameType::Index => {
                segment.index_storage.closing_index(offset);
                let prefix = if let Some(prefix) = checked.index.take() {
                    prefix.map_err(ReadFailure::Content)
                } else {
                    match admit_checked(checked.frame) {
                        Err(error) => Err(error),
                        Ok(_) => return Err(ReadFailure::MissingContent),
                    }
                };
                super::fields::push(
                    &mut self.graph.index_checks,
                    GraphIndexCheck { offset, prefix },
                )?;
            }
            kind => return Err(ReadFailure::Frame(kind)),
        }
        Ok(())
    }

    fn observe_rows(
        &mut self,
        checked: super::SegmentFrame<'_>,
        offset: usize,
        segment: &mut Segment<'_>,
    ) -> Result<(), ReadFailure> {
        if let Some(Err(error)) = checked.rows {
            return Err(ReadFailure::Content(error));
        }
        let decoded = segment
            .verifier
            .decode_rows(&checked)
            .ok_or(ReadFailure::MissingContent)?
            .map_err(ReadFailure::Content)?;
        if decoded.declared_order_violated {
            super::fields::push(&mut self.graph.row_order_violations, offset)?;
        }
        let findings = segment
            .dictionary
            .fold_rows(&mut self.graph.rdf, &decoded)
            .map_err(|error| match error {
                FoldError::RepeatedAnnotation => ReadFailure::Content(
                    WireError::new(MalformedReason::UnorderedOrRepeated, 0).into(),
                ),
                error => ReadFailure::Fold(error),
            })?;
        if findings.forward_reference || findings.position_constraint {
            return Err(ReadFailure::Rows(findings));
        }
        if findings.opaque_term {
            super::fields::push(
                &mut self.graph.frame_failures,
                ReadError {
                    offset,
                    failure: ReadFailure::Rows(findings),
                },
            )?;
        }
        if self.graph.rdf.annotation_width_exceeded() {
            return Err(ReadFailure::AnnotationWidth);
        }
        Ok(())
    }
}

fn verified_payload(state: PayloadState<'_>) -> Result<Cow<'_, [u8]>, ReadFailure> {
    match state {
        PayloadState::Verified(bytes) => Ok(bytes),
        state => Err(payload_failure(state)),
    }
}

fn payload_failure(state: PayloadState<'_>) -> ReadFailure {
    match state {
        PayloadState::Damaged(damage) => ReadFailure::PayloadDamage(damage),
        PayloadState::Unchecked(reason) => ReadFailure::PayloadRefusal(reason),
        PayloadState::Verified(_) => ReadFailure::MissingContent,
    }
}

impl From<WireError> for ReadFailure {
    fn from(error: WireError) -> Self {
        Self::Wire(error)
    }
}

impl From<std::collections::TryReserveError> for ReadFailure {
    fn from(error: std::collections::TryReserveError) -> Self {
        Self::Fold(error.into())
    }
}

fn admit_checked(checked: FrameCheck<'_>) -> Result<FrameCheck<'_>, ReadFailure> {
    if let Some(error) = checked.malformed {
        return Err(ReadFailure::Wire(error));
    }
    if checked.link.state == LinkState::Broken {
        return Err(ReadFailure::BrokenLink);
    }
    if !matches!(checked.payload.state, PayloadState::Verified(_)) {
        return Err(payload_failure(checked.payload.state));
    }
    Ok(checked)
}

fn decode_opener(opener: Item<'_>) -> Result<Header<'_>, ReadFailure> {
    let kind = opener
        .classify(NonZeroU64::MIN)
        .map_err(ReadFailure::Classification)?;
    match kind {
        ItemKind::Header => Header::decode(opener).map_err(ReadFailure::Wire),
        ItemKind::Resync => Header::decode_resync(opener).map_err(ReadFailure::Wire),
        _ => Err(ReadFailure::Item(kind)),
    }
}

fn segment_from_header<'a>(
    header: Header<'a>,
    extent: Range<usize>,
    previous: Option<&Segment<'a>>,
) -> Result<Segment<'a>, ReadFailure> {
    let registry = header.registry_version();
    if registry.get() != super::REGISTRY_VERSION {
        return Err(ReadFailure::Registry(registry.get()));
    }
    let lineage = header.lineage();
    if let Some(lineage) = lineage
        && previous.is_none_or(|previous| {
            lineage.previous_head != previous.verifier.head()
                || previous
                    .ordinal
                    .is_some_and(|ordinal| ordinal.checked_add(1) != Some(lineage.ordinal.get()))
        })
    {
        return Err(ReadFailure::Lineage);
    }
    let file_ordinal = previous.map_or(0, |previous| previous.file_ordinal + 1);
    let identifier = header.identifier().ok_or(ReadFailure::MissingContent)?;
    Ok(Segment {
        ordinal: lineage.map(|lineage| lineage.ordinal.get()),
        verifier: SegmentVerifier::new(identifier, registry),
        dictionary: SegmentDictionary::new(file_ordinal),
        file_ordinal,
        units: FastMap::default(),
        index_storage: StorageIndex::default(),
        parity: ParityStream::new(extent, identifier, registry)?,
    })
}
