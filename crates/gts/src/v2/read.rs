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
use super::parity::ParityStream;
use super::{
    ContentError, DIGEST_LENGTH, DefinitionFindings, FoldError, FormedBudget, FrameCheck,
    FrameEnvelope, FrameField, FrameType, Generation, Header, IndexPrefixCheck,
    IndexStorageFinding, IndexStorageItem, Item, ItemKind, LinkState, MalformedReason,
    ParityFinding, PayloadBudget, PayloadCapacity, PayloadDamage, PayloadRefusal, PayloadState,
    RESYNC_MAX_BYTES, RdfFold, ResyncCandidate, ResyncScan, RowFindings, SegmentCheckError,
    SegmentDictionary, SegmentVerifier, StoredState, ValueName, ValueStore, WireError,
};
use crate::{FastMap, model::Graph};

mod blob;

pub use blob::ExternalBlob;

/// Generation-neutral bounds applied before an opener selects its reader.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DetectionCapacity {
    /// Maximum encoded bytes in the opening item.
    pub encoded_opener_bytes: usize,
    /// Maximum arrays, maps and tags open while delimiting the opener.
    pub nesting_depth: usize,
    /// Maximum structural work. A complete opener charges
    /// [`Self::OPENER_WORK_PER_BYTE`] units per encoded byte; damaged-prefix
    /// recovery charges [`Self::RECOVERY_WORK_PER_BYTE`] units per scanned byte.
    pub work: usize,
}

impl DetectionCapacity {
    /// Delimiting, tagged-child inspection and generation inspection.
    pub const OPENER_WORK_PER_BYTE: usize = 3;
    /// Pattern matching plus worst-case candidate parsing for one recovery byte.
    pub const RECOVERY_WORK_PER_BYTE: usize = RESYNC_MAX_BYTES * 2 + 3;

    /// Conservative work charge for scanning this many damaged-prefix bytes.
    pub const fn recovery_work(bytes: usize) -> usize {
        bytes.saturating_mul(Self::RECOVERY_WORK_PER_BYTE)
    }

    const LEGACY: Self = Self {
        encoded_opener_bytes: usize::MAX,
        nesting_depth: usize::MAX,
        work: usize::MAX,
    };
}

impl Default for DetectionCapacity {
    fn default() -> Self {
        Self {
            encoded_opener_bytes: 64 * 1024 * 1024,
            nesting_depth: 256,
            work: 192 * 1024 * 1024,
        }
    }
}

/// Limits for materializing a generation 2 RDF graph.
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

/// Complete limits for generation detection and the selected graph reader.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct ReadLimits {
    /// Bounds shared by every generation before dispatch.
    pub detection: DetectionCapacity,
    /// Complete generation 1 read contract when the opener delegates to the
    /// legacy graph reader.
    pub first_generation: crate::reader::GtsReadLimits,
    /// Generation 2 graph materialization limits.
    pub second_generation: ReadCapacity,
}

impl ReadLimits {
    const fn legacy(second_generation: ReadCapacity) -> Self {
        Self {
            detection: DetectionCapacity::LEGACY,
            first_generation: crate::reader::GtsReadLimits::UNBOUNDED,
            second_generation,
        }
    }
}

impl Default for ReadLimits {
    fn default() -> Self {
        Self {
            detection: DetectionCapacity::default(),
            first_generation: crate::reader::GtsReadLimits::DEFAULT,
            second_generation: ReadCapacity::default(),
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
    /// Scanned resync openers and the framing damage before them, in file order.
    pub resynchronizations: Vec<Resynchronization>,
    /// Parity diagnostics in segment and item order, followed by each segment's tail.
    pub parity_findings: Vec<ParityFinding>,
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
    /// Generation detection needs more opener bytes than admitted.
    DetectionBytes {
        /// First encoded byte beyond the capacity.
        required: usize,
        /// Configured opener-byte ceiling.
        limit: usize,
        /// Native framing failure that led bounded recovery to this ceiling.
        cause: Option<StructuralError>,
    },
    /// Generation detection needs more nesting than admitted.
    DetectionDepth {
        /// First nesting depth beyond the capacity.
        required: usize,
        /// Configured nesting ceiling.
        limit: usize,
        /// Native structural refusal at the first excess opening item.
        cause: StructuralError,
    },
    /// Generation detection needs more structural work than admitted.
    DetectionWork {
        /// Conservative work charge required by the opener.
        required: usize,
        /// Configured structural-work ceiling.
        limit: usize,
        /// Native framing failure that led bounded recovery to this ceiling.
        cause: Option<StructuralError>,
    },
    /// A generation 1 resource exceeded its explicit read capacity.
    FirstGenerationCapacity(crate::reader::GtsReadLimitError),
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
            ReadFailure::Content(error) => Some(error),
            ReadFailure::Fold(error) => Some(error),
            ReadFailure::DetectionDepth { cause, .. } => Some(cause),
            ReadFailure::DetectionBytes {
                cause: Some(cause), ..
            }
            | ReadFailure::DetectionWork {
                cause: Some(cause), ..
            } => Some(cause),
            ReadFailure::FirstGenerationCapacity(error) => Some(error),
            ReadFailure::PayloadDamage(PayloadDamage::DecodeFailed(error)) => Some(error),
            _ => None,
        }
    }
}

/// Decode a GTS file into the graph model of its opening generation.
///
/// The first generation uses the existing reader over the original bytes.
/// The generation 2 graph projection checks framing, links, payloads, commits
/// and RDF positions. It returns an error when a row or term is unreadable or
/// when an item requires content interpretation outside the graph projection.
/// The reader can recover at a verified resync opener after failure of the
/// framing test or opening generation recognition. An ordinary frame whose
/// healing witness is discarded prevents a complete graph projection.
/// Parity findings describe the supplied bytes before any repair.
/// Indexes report comparisons against the observed prefix and never supply
/// graph content.
/// The graph carries no assertion about file authenticity or canonical form.
///
/// # Errors
/// Returns the byte position and reason when a graph cannot be produced.
pub fn read(bytes: &[u8], capacity: ReadCapacity) -> Result<ReadGraph<'_>, ReadError> {
    read_inner(bytes, ReadLimits::legacy(capacity))
}

/// Decode a GTS file under one finite generation-neutral read contract.
///
/// Use this entry point for untrusted input. [`ReadLimits::default`] bounds
/// generation detection and both generation readers; callers handling larger
/// admitted datasets can raise the applicable finite fields.
///
/// # Errors
/// Returns the byte position and reason when a graph cannot be produced.
pub fn read_with_limits(bytes: &[u8], limits: ReadLimits) -> Result<ReadGraph<'_>, ReadError> {
    read_inner(bytes, limits)
}

fn read_inner(bytes: &[u8], limits: ReadLimits) -> Result<ReadGraph<'_>, ReadError> {
    let capacity = limits.second_generation;
    if capacity.items == 0 && !bytes.is_empty() {
        return Err(ReadError {
            offset: 0,
            failure: ReadFailure::ItemCapacity(0),
        });
    }
    let mut detection_work = 0;
    let (opener, generation) = match detect_opener(bytes, limits.detection, &mut detection_work) {
        Ok(Some(detected)) => detected,
        Ok(None) => {
            return Err(ReadError {
                offset: 0,
                failure: ReadFailure::Empty,
            });
        }
        Err(error) => {
            if !matches!(error.failure, ReadFailure::Framing(_)) {
                return Err(error);
            }
            return GraphReader::new(bytes, capacity, limits.detection, Some(detection_work))
                .recover_initial(error, None)
                .map(ReadGraph::Second);
        }
    };
    match generation {
        Generation::First => crate::reader::read_with_limits(
            bytes,
            crate::reader::ReadOptions::new(true, None),
            limits.first_generation,
        )
        .map(ReadGraph::First)
        .map_err(|error| ReadError {
            offset: 0,
            failure: ReadFailure::FirstGenerationCapacity(error),
        }),
        Generation::Second => GraphReader::new(bytes, capacity, limits.detection, None)
            .read_first(opener, generation)
            .map(ReadGraph::Second),
        Generation::Unrecognized => {
            GraphReader::new(bytes, capacity, limits.detection, Some(detection_work))
                .read_first(opener, generation)
                .map(ReadGraph::Second)
        }
        generation => Err(ReadError {
            offset: 0,
            failure: ReadFailure::Generation(generation),
        }),
    }
}

fn detect_opener<'a>(
    bytes: &'a [u8],
    capacity: DetectionCapacity,
    work_used: &mut usize,
) -> Result<Option<(Item<'a>, Generation)>, ReadError> {
    let encoded_limit = capacity.encoded_opener_bytes.min(capacity.work);
    let available = bytes.len().min(encoded_limit);
    let refusal = |required_bytes: usize| {
        let failure = if capacity.encoded_opener_bytes <= capacity.work {
            ReadFailure::DetectionBytes {
                required: required_bytes,
                limit: capacity.encoded_opener_bytes,
                cause: None,
            }
        } else {
            ReadFailure::DetectionWork {
                required: required_bytes,
                limit: capacity.work,
                cause: None,
            }
        };
        ReadError { offset: 0, failure }
    };
    if available == 0 && !bytes.is_empty() {
        return Err(refusal(1));
    }
    match Item::first_with_depth(&bytes[..available], capacity.nesting_depth) {
        Ok(Some(item)) => {
            let required = item
                .bytes()
                .len()
                .saturating_mul(DetectionCapacity::OPENER_WORK_PER_BYTE);
            if required > capacity.work {
                return Err(ReadError {
                    offset: 0,
                    failure: ReadFailure::DetectionWork {
                        required,
                        limit: capacity.work,
                        cause: None,
                    },
                });
            }
            *work_used = required;
            let generation = item.generation().map_err(|error| ReadError {
                offset: 0,
                failure: ReadFailure::Framing(error),
            })?;
            Ok(Some((item, generation)))
        }
        Ok(None) => Ok(None),
        Err(error)
            if matches!(error.kind(), DecodeErrorKind::Truncated) && bytes.len() > available =>
        {
            Err(refusal(available.saturating_add(1)))
        }
        Err(error) => {
            *work_used = usize::try_from(error.offset())
                .unwrap_or(usize::MAX)
                .saturating_add(1)
                .min(available);
            match error.kind() {
                DecodeErrorKind::Depth { limit } => Err(ReadError {
                    offset: usize::try_from(error.offset()).unwrap_or(usize::MAX),
                    failure: ReadFailure::DetectionDepth {
                        required: limit.saturating_add(1),
                        limit,
                        cause: error,
                    },
                }),
                _ => Err(ReadError {
                    offset: 0,
                    failure: ReadFailure::Framing(error),
                }),
            }
        }
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
    initial_scan: Option<ResyncScan<'a>>,
    initial_end: Option<usize>,
    initial_refusal: Option<ReadError>,
    initial_exhausted: bool,
    graph: RdfGraph<'a>,
    values: ValueStore,
    payload: PayloadBudget,
    formed: FormedBudget,
    items: u64,
    item_limit: u64,
    rebuilt_remaining: u64,
}

impl<'a> GraphReader<'a> {
    fn new(
        bytes: &'a [u8],
        capacity: ReadCapacity,
        detection: DetectionCapacity,
        detection_work: Option<usize>,
    ) -> Self {
        let (initial_scan, initial_end, initial_refusal) =
            detection_work.map_or((None, None, None), |used| {
                let remaining = detection.work.saturating_sub(used);
                let work_end = remaining / DetectionCapacity::RECOVERY_WORK_PER_BYTE;
                let byte_end = detection.encoded_opener_bytes;
                let end = bytes.len().min(work_end).min(byte_end);
                let refusal = (end < bytes.len()).then(|| {
                    let failure = if byte_end <= work_end {
                        ReadFailure::DetectionBytes {
                            required: end.saturating_add(1),
                            limit: byte_end,
                            cause: None,
                        }
                    } else {
                        ReadFailure::DetectionWork {
                            required: used.saturating_add(
                                end.saturating_add(1)
                                    .saturating_mul(DetectionCapacity::RECOVERY_WORK_PER_BYTE),
                            ),
                            limit: detection.work,
                            cause: None,
                        }
                    };
                    ReadError {
                        offset: end,
                        failure,
                    }
                });
                (Some(ResyncScan::new(&bytes[..end])), Some(end), refusal)
            });
        Self {
            bytes,
            scan: ResyncScan::new(bytes),
            initial_scan,
            initial_end,
            initial_refusal,
            initial_exhausted: false,
            graph: RdfGraph::default(),
            values: ValueStore::default(),
            payload: PayloadBudget::new(capacity.payload),
            formed: FormedBudget::new(capacity.formed_bytes),
            items: 0,
            item_limit: capacity.items,
            rebuilt_remaining: capacity.rebuilt_bytes,
        }
    }

    fn scan_end(&self) -> usize {
        self.initial_end.unwrap_or(self.bytes.len())
    }

    fn finish_detection(&mut self) {
        self.initial_scan = None;
        self.initial_end = None;
        self.initial_refusal = None;
    }

    fn detection_exhausted(&self) -> bool {
        self.initial_exhausted
    }

    fn detection_refusal(&mut self, initial: &ReadError) -> Option<ReadError> {
        if !self.detection_exhausted() {
            return None;
        }
        let mut refusal = self.initial_refusal.take()?;
        let ReadFailure::Framing(cause) = &initial.failure else {
            return Some(refusal);
        };
        match &mut refusal.failure {
            ReadFailure::DetectionBytes { cause: slot, .. }
            | ReadFailure::DetectionWork { cause: slot, .. } => {
                *slot = Some(cause.clone());
            }
            _ => {}
        }
        Some(refusal)
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
        } else {
            self.scan_undelimited(&initial, 0, false)?
        };
        let Some(candidate) = candidate else {
            return Err(self.detection_refusal(&initial).unwrap_or(initial));
        };
        let start = candidate.offset;
        let end = start + candidate.bytes.len();
        self.finish_detection();
        let segment = self.resume_segment(initial, &candidate, None)?;
        self.read_segment(segment, start, end)
    }

    fn scan_undelimited(
        &mut self,
        failure: &ReadError,
        scan_start: usize,
        region_counted: bool,
    ) -> Result<Option<ResyncCandidate<'a>>, ReadError> {
        let ReadFailure::Framing(error) = &failure.failure else {
            return Ok(None);
        };
        match error.kind() {
            DecodeErrorKind::Malformed(_) => {
                if !region_counted {
                    self.count_item(failure.offset)?;
                }
            }
            DecodeErrorKind::Truncated => {}
            _ => return Ok(None),
        }
        self.scan_resync(scan_start, self.scan_end())
    }

    fn scan_resync(
        &mut self,
        start: usize,
        end: usize,
    ) -> Result<Option<ResyncCandidate<'a>>, ReadError> {
        if self.items == self.item_limit {
            return Ok(None);
        }
        let scanned = if let Some(scan) = &mut self.initial_scan {
            let scanned = scan.scan(start..end, &mut self.payload);
            self.initial_exhausted =
                scanned.as_ref().is_ok_and(Option::is_none) && scan.next_offset() >= end;
            scanned
        } else {
            self.scan.scan(start..end, &mut self.payload)
        };
        scanned.map_err(|refused| ReadError {
            offset: refused.offset,
            failure: ReadFailure::PayloadRefusal(PayloadRefusal::TotalDecoded(
                refused.requested_total,
            )),
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
                    .index_storage
                    .record_length(length)
                    .map_err(|error| ReadError {
                        offset,
                        failure: error.into(),
                    })?;
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
        let scan_end = self.scan_end();
        let limit = failed_end.saturating_add(RESYNC_MAX_BYTES).min(scan_end);
        let mut offset = if items.complete.is_empty() {
            failed_end
        } else {
            items.complete_end
        };
        while offset < limit && self.items < self.item_limit {
            // The immediate next item's extent can move the decision point
            // past the byte lookahead. Later items count only within it.
            let full_input = offset == failed_end || limit == scan_end;
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
                    let Some(candidate) =
                        self.scan_undelimited(&error, previous_start + 1, items.region_counted)?
                    else {
                        return Err(error);
                    };
                    previous_start = candidate.offset;
                    offset = candidate.offset + candidate.bytes.len();
                    segment = self.resume_segment(error, &candidate, Some(&mut segment))?;
                    items = HeldItems::default();
                    continue;
                }
            };
            let end = offset + item.bytes().len();
            let head = held
                .as_ref()
                .and_then(|frame| frame.checked.link.effective_identifier.as_ref())
                .unwrap_or_else(|| segment.verifier.head());
            // The current frame already links under a held predecessor's
            // computed ID. Its successor cannot heal it in that case.
            let witness = if held.is_none() {
                items.peek(&self, end)
            } else {
                None
            };
            let checked = Self::inspect_item(item, witness, head, offset, &segment.index_storage)
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
            .finish(self.bytes, &mut self.graph.parity_findings)
            .map_err(read_error)?;
        segment
            .index_storage
            .finish(&mut self.graph.index_storage_findings)
            .map_err(read_error)
    }

    fn count_item(&mut self, offset: usize) -> Result<(), ReadError> {
        self.check_item_limit(offset)?;
        self.items += 1;
        Ok(())
    }

    fn check_item_limit(&self, offset: usize) -> Result<(), ReadError> {
        if self.items == self.item_limit && offset < self.scan_end() {
            return Err(ReadError {
                offset,
                failure: ReadFailure::ItemCapacity(self.item_limit),
            });
        }
        Ok(())
    }

    fn next_item(&self, offset: usize) -> Result<Option<Item<'a>>, ReadError> {
        self.check_item_limit(offset)?;
        let end = self.scan_end();
        if offset >= end {
            return Ok(None);
        }
        delimit(&self.bytes[..end], offset)
    }

    fn inspect_item(
        item: Item<'a>,
        next: Option<Item<'a>>,
        predecessor: &[u8; DIGEST_LENGTH],
        offset: usize,
        storage_index: &StorageIndex,
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
                        item.offsets_match(storage_index.lengths()),
                    )
                } else {
                    (
                        IndexStorageItem::Locator,
                        item.locator_distance()
                            .map(|distance| storage_index.locator_matches(offset, distance)),
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
                segment
                    .index_storage
                    .observe(offset, extent.len(), item, matches)
                    .map_err(|error| ReadError {
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
        let checked = if matches!(kind, Some(FrameType::Index | FrameType::Parity)) {
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
            )?;
        }
        let kind = kind.ok_or(ReadFailure::MissingContent)?;
        segment.index_storage.record_length(frame.bytes().len())?;
        match kind {
            FrameType::Parity => {
                let check = checked.parity.take().ok_or(ReadFailure::MissingContent)?;
                segment.parity.observe(offset, check)?;
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
        if findings.forward_reference || findings.opaque_term || findings.position_constraint {
            return Err(ReadFailure::Rows(findings));
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
        index_storage: StorageIndex::new(extent.len())?,
        parity: ParityStream::new(extent),
    })
}
