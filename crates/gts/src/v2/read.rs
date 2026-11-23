// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

#![expect(
    clippy::result_large_err,
    reason = "preserve native failure data without allocating on the refusal path"
)]

use core::fmt;
use core::num::NonZeroU64;
use std::borrow::Cow;

use purrdf_lex::cbor::{DecodeErrorKind, StructuralError};

use super::frame::FrameWire;
use super::{
    ContentError, DIGEST_LENGTH, DefinitionFindings, FoldError, FormedBudget, FrameCheck,
    FrameEnvelope, FrameField, FrameType, Generation, Header, Item, ItemKind, LinkState,
    MalformedReason, PayloadBudget, PayloadCapacity, PayloadDamage, PayloadRefusal, PayloadState,
    RESYNC_MAX_BYTES, RdfFold, ResyncCandidate, ResyncScan, RowFindings, SegmentCheckError,
    SegmentDictionary, SegmentVerifier, StoredState, ValueName, ValueStore, WireError,
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
    /// Scanned resync openers and the framing damage before them, in file order.
    pub resynchronizations: Vec<Resynchronization>,
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

#[expect(
    clippy::large_enum_variant,
    reason = "keep the prepared frame on the stack without allocating for each item"
)]
enum ItemCheck<'a> {
    Frame(FrameEnvelope<'a>, FrameWire<'a>),
    Opener(Header<'a>),
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
            self.count_held_items(end, self.next_item(end))?;
            self.scan_resync(0, end)?
        } else {
            self.scan_undelimited(&initial, 0)?
        };
        let Some(candidate) = candidate else {
            return Err(initial);
        };
        let start = candidate.offset;
        let end = start + candidate.bytes.len();
        let segment = self.resume_segment(initial, &candidate, None)?;
        self.read_segment(segment, start, end)
    }

    fn scan_undelimited(
        &mut self,
        failure: &ReadError,
        scan_start: usize,
    ) -> Result<Option<ResyncCandidate<'a>>, ReadError> {
        let ReadFailure::Framing(error) = &failure.failure else {
            return Ok(None);
        };
        match error.kind() {
            DecodeErrorKind::Malformed(_) => self.count_item(failure.offset)?,
            DecodeErrorKind::Truncated => {}
            _ => return Ok(None),
        }
        self.scan_resync(scan_start, self.bytes.len())
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

    fn resume_segment(
        &mut self,
        original: ReadError,
        candidate: &ResyncCandidate<'a>,
        previous: Option<&Segment<'a>>,
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
        let segment =
            segment_from_header(candidate.header, previous).map_err(|failure| ReadError {
                offset: candidate.offset,
                failure,
            })?;
        self.count_item(candidate.offset)?;
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
        first: Result<Option<Item<'a>>, ReadError>,
    ) -> Result<(), ReadError> {
        let limit = failed_end
            .saturating_add(RESYNC_MAX_BYTES)
            .min(self.bytes.len());
        let mut offset = failed_end;
        let mut first = Some(first);
        while offset < limit && self.items < self.item_limit {
            // The immediate next item's extent can move the decision point
            // past the byte lookahead. Later items count only within it.
            match first
                .take()
                .unwrap_or_else(|| delimit(&self.bytes[..limit], offset))
            {
                Ok(Some(item)) => {
                    self.count_item(offset)?;
                    offset += item.bytes().len();
                }
                Ok(None) => break,
                Err(error) => {
                    let ReadFailure::Framing(cause) = &error.failure else {
                        return Err(error);
                    };
                    match cause.kind() {
                        // A held undelimitable region extends to the end of
                        // the input for the purpose of the item count.
                        DecodeErrorKind::Malformed(_) => return self.count_item(offset),
                        DecodeErrorKind::Truncated => {}
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
        let segment = segment_from_header(opener, None).map_err(|failure| ReadError {
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
        let mut next = self.next_item(offset);
        let mut held: Option<GraphFrame<'a>> = None;
        loop {
            let item = match next {
                Ok(Some(item)) => item,
                Ok(None) => return Ok(self.graph),
                Err(error) => {
                    let Some(candidate) = self.scan_undelimited(&error, previous_start + 1)? else {
                        return Err(error);
                    };
                    previous_start = candidate.offset;
                    offset = candidate.offset + candidate.bytes.len();
                    segment = self.resume_segment(error, &candidate, Some(&segment))?;
                    next = self.next_item(offset);
                    continue;
                }
            };
            self.count_item(offset)?;
            let end = offset + item.bytes().len();
            next = self.next_item(end);
            let head = held
                .as_ref()
                .and_then(|frame| frame.checked.link.effective_identifier.as_ref())
                .unwrap_or_else(|| segment.verifier.head());
            // The current frame already links under a held predecessor's
            // computed ID. Its successor cannot heal it in that case.
            let witness = if held.is_none() {
                next.as_ref().ok().and_then(|item| *item)
            } else {
                None
            };
            let checked = Self::inspect_item(item, witness, head)
                .map_err(|failure| ReadError { offset, failure })?;
            if let Some(failure) = self.observe_item(checked, offset, &mut held, &mut segment)? {
                let scan_start = if held.is_some() {
                    offset
                } else {
                    previous_start + 1
                };
                let candidate = match self.count_held_items(end, next) {
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
                let original = ReadError { offset, failure };
                let Some(candidate) = candidate? else {
                    return Err(original);
                };
                if let Some(frame) = held {
                    // The accepted opener replaces this frame's only witness.
                    let failure = frame
                        .checked
                        .malformed
                        .map_or(ReadFailure::BrokenLink, ReadFailure::Wire);
                    return Err(ReadError {
                        offset: frame.offset,
                        failure,
                    });
                }
                previous_start = candidate.offset;
                offset = candidate.offset + candidate.bytes.len();
                segment = self.resume_segment(original, &candidate, Some(&segment))?;
                next = self.next_item(offset);
                continue;
            }
            previous_start = offset;
            offset = end;
        }
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
        offset: usize,
        held: &mut Option<GraphFrame<'a>>,
        segment: &mut Segment<'a>,
    ) -> Result<Option<ReadFailure>, ReadError> {
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
                if !framed {
                    // Independent payload checks occur before an attempted scan.
                    return Ok(Some(
                        admit_checked(checked)
                            .err()
                            .unwrap_or(ReadFailure::Frame(FrameType::Parity)),
                    ));
                }
                let frame = GraphFrame {
                    offset,
                    envelope,
                    checked,
                };
                if frame.checked.link.state == LinkState::Healed {
                    // Only the next item's framing can make this link final.
                    *held = Some(frame);
                } else {
                    self.deliver_frame(frame, segment)?;
                }
            }
            ItemCheck::Opener(header) => {
                self.deliver_held(held.take(), segment)?;
                *segment = segment_from_header(header, Some(segment))
                    .map_err(|failure| ReadError { offset, failure })?;
            }
            ItemCheck::Unsupported(kind) => {
                self.deliver_held(held.take(), segment)?;
                return Err(ReadError {
                    offset,
                    failure: ReadFailure::Item(kind),
                });
            }
            ItemCheck::FramingFailure(failure) => return Ok(Some(failure)),
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
        let checked = admit_checked(checked)?;
        if checked.payload.stored == Some(StoredState::Mismatch) {
            super::fields::push(&mut self.graph.stored_digest_mismatches, offset)?;
        }
        let mut checked = segment
            .verifier
            .accept_checked(&frame, checked, &self.payload)
            .map_err(ReadFailure::Segment)?;
        let kind = checked
            .frame
            .frame_type
            .and_then(FrameType::from_code)
            .ok_or(ReadFailure::MissingContent)?;
        match kind {
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
    })
}
