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

use super::index::{IndexContent, StorageIndex};
use super::parity::{ParityResumption, ParityStream};
use super::payload::CandidateRefusal;
use super::rdf::RowScope;
use super::signature::{AuthenticationProgress, SignatureRecord};
use super::{
    CommitCheck, ContentError, DIGEST_LENGTH, DefinitionFindings, FoldError, FormedBudget,
    FrameCheck, FrameEnvelope, FrameField, FrameType, Generation, Header, IndexPrefixCheck,
    IndexStorageFinding, IndexStorageItem, Item, ItemKind, LinkState, MalformedReason, ParityCheck,
    ParityFinding, ParitySolveError, PayloadBudget, PayloadCapacity, PayloadDamage, PayloadRefusal,
    PayloadState, RESYNC_MAX_BYTES, RdfFold, ResyncCandidate, ResyncRefusal, ResyncScan, Row,
    RowFindings, SegmentAuthentication, SegmentCheckError, SegmentDictionary, SegmentFrame,
    SegmentVerifier, SignatureCheck, SignatureInputs, SignatureStatus, StoredState,
    UnknownFrameType, ValueName, ValueStore, WireError,
};
use crate::{FastMap, model::Graph};

mod blob;
mod error_inspection;
mod frame_wire;
mod resync;

use blob::{BlobClaim, RebuildBudget};
pub use blob::{BlobEntry, ExternalBlob, RebuildResult, RebuildState};
pub use frame_wire::{FrameWireResult, PayloadOutcome, PayloadUnchecked};
use resync::PastChecks;
pub use resync::{ResyncOpener, ResyncPast, ResyncReach};

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
    /// Maximum arrays, maps and tags open while delimiting a generation 2 item.
    pub second_generation_nesting_depth: usize,
}

impl Default for ReadLimits {
    fn default() -> Self {
        Self {
            detection: DetectionCapacity::default(),
            first_generation: crate::reader::GtsReadLimits::DEFAULT,
            second_generation: ReadCapacity::default(),
            second_generation_nesting_depth: 256,
        }
    }
}

/// A GTS file decoded into the graph model of its generation.
#[derive(Debug)]
#[non_exhaustive]
#[allow(
    clippy::large_enum_variant,
    reason = "the public variants return their graph by value without an extra allocation"
)]
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
    /// Readable RDF terms and resolved RDF lines. Empty after an item refusal.
    pub rdf: RdfFold,
    /// Where the supplied bytes end relative to framing and commits.
    pub end: FileEnd,
    /// Independent wire checks of the retained frames, in file order.
    pub frame_wire: Vec<FrameWireResult>,
    /// Number of opaque frames, omitted items and framing regions in the checked prefix.
    pub left_out: u64,
    /// Verified blob declarations in file order. Empty after an item refusal.
    pub blobs: Vec<VerifiedBlob<'a>>,
    /// References to named blobs declared without inline bytes. Empty after an item refusal.
    pub external_blobs: Vec<ExternalBlob<'a>>,
    /// Folded blob declarations grouped by name, including blobs without verified bytes.
    pub blob_set: Vec<BlobEntry<'a>>,
    /// Frame offsets of declarations that disagree with their blob entry's minimum length.
    pub blob_length_conflicts: Vec<usize>,
    /// Recipe and value rebuild outcomes in declaration order.
    pub rebuilds: Vec<RebuildResult>,
    /// File offsets of row frames whose declared order differs from their rows.
    pub row_order_violations: Vec<usize>,
    /// File offsets whose stored check differs from the compressed bytes.
    pub stored_digest_mismatches: Vec<usize>,
    /// Findings for opaque frames, unavailable rebuilds and held rows.
    pub frame_failures: Vec<ReadError>,
    /// Complete items that could not be classified in their segment.
    pub opaque_items: Vec<ReadError>,
    /// Opener offsets and declared versions of segments with newer registries.
    pub unsupported_registries: Vec<(usize, NonZeroU64)>,
    /// Segments whose opener prevents link and content checks.
    pub skipped_segments: Vec<SkippedSegment>,
    /// Lineage findings at segment openers in file order.
    pub lineage_findings: Vec<(usize, LineageFinding)>,
    /// Frame offsets where an earlier unsupported critical value stopped the fold.
    pub stopped_frames: Vec<usize>,
    /// Scanned resync openers and the framing damage before them, in file order.
    pub resynchronizations: Vec<Resynchronization>,
    /// Decoded resync openers and their independent past checks, in file order.
    pub resync_openers: Vec<ResyncOpener>,
    /// Damaged regions ended by verified parity frames, in file order.
    pub parity_recoveries: Vec<ParityRecovery>,
    /// The first recovery candidate refused by the total decoded-byte budget.
    pub recovery_refusal: Option<RecoveryRefusal>,
    /// Trailing bytes of the input with no accepted recovery point in the final segment.
    pub framing_tail: Option<FramingDamage>,
    /// The first position left unread when the item limit is reached.
    pub item_refusal: Option<ItemRefusal>,
    /// Commits that cannot verify the ordinary prefix across a parity recovery.
    pub interrupted_commits: Vec<(usize, CommitCheck)>,
    /// Commit checks that failed on a fully walked ordinary prefix.
    pub invalid_commits: Vec<(usize, CommitCheck)>,
    /// Parity diagnostics and repair outcomes, followed by each segment's tail.
    pub parity_findings: Vec<ParityFinding>,
    /// A repaired copy when any span passes verification.
    pub repaired: Option<Vec<u8>>,
    /// Closing or intermediate index checks in file order.
    pub index_checks: Vec<GraphIndexCheck>,
    /// False or misplaced offsets items and locators.
    pub index_storage_findings: Vec<IndexStorageFinding>,
    /// Signature checks on retained frames, in file order.
    pub signatures: Vec<FrameSignatures>,
    /// Authentication of each readable segment, in file order.
    pub segment_authentication: Vec<SegmentAuthentication>,
    /// Commit coverage and uncommitted tails of readable segments, in file order.
    pub segment_durability: Vec<SegmentDurability>,
    first_content_omission: Option<usize>,
    framing_tail_cause: Option<StructuralError>,
    annotation_width_offset: Option<usize>,
}

impl RdfGraph<'_> {
    /// Take the resolved RDF fold when the read omitted no content.
    ///
    /// Use this at dataset-only boundaries that cannot return the read
    /// evidence. A false index or commit claim over readable content and a
    /// complete uncommitted suffix do not prevent projection.
    ///
    /// # Errors
    /// Retains the cause of the omission, width refusal, item refusal, recovery or trailing damage.
    pub fn into_complete_rdf(self) -> Result<RdfFold, ReadError> {
        if let Some(refused) = self.item_refusal {
            return Err(ReadError {
                offset: refused.offset,
                failure: ReadFailure::ItemCapacity(refused.limit),
            });
        }
        if let Some(offset) = self.annotation_width_offset {
            return Err(ReadError {
                offset,
                failure: ReadFailure::AnnotationWidth,
            });
        }
        if let Some(offset) = self.first_content_omission {
            let cause = self
                .frame_failures
                .into_iter()
                .chain(self.opaque_items)
                .chain(
                    self.skipped_segments
                        .into_iter()
                        .map(|segment| segment.opener),
                )
                .find(|error| error.offset == offset);
            return Err(cause.unwrap_or(ReadError {
                offset,
                failure: ReadFailure::OmittedContent,
            }));
        }
        if let Some(damage) = self.framing_tail {
            return Err(ReadError {
                offset: damage.offset(),
                failure: ReadFailure::FramingTail {
                    damage,
                    cause: self.framing_tail_cause,
                },
            });
        }
        if let Some(recovery) = self.resynchronizations.first() {
            return Err(ReadError {
                offset: recovery.offset,
                failure: ReadFailure::RecoveredResync(*recovery),
            });
        }
        if let Some(recovery) = self.parity_recoveries.first() {
            return Err(ReadError {
                offset: recovery.damage.offset(),
                failure: ReadFailure::RecoveredParity(*recovery),
            });
        }
        if let Some(refusal) = self.recovery_refusal {
            return Err(ReadError {
                offset: refusal.offset,
                failure: ReadFailure::RecoveryRefused(refusal),
            });
        }
        if self.rdf.has_held_rows() || self.rdf.annotation_width_exceeded() {
            return Err(ReadError {
                offset: 0,
                failure: ReadFailure::OmittedContent,
            });
        }
        Ok(self.rdf)
    }

    fn note_content_omission(&mut self, offset: usize) {
        self.first_content_omission = Some(
            self.first_content_omission
                .map_or(offset, |earlier| earlier.min(offset)),
        );
    }

    fn retain_content_failure(
        &mut self,
        offset: usize,
        failure: ReadFailure,
    ) -> Result<(), std::collections::TryReserveError> {
        self.note_content_omission(offset);
        super::fields::push(&mut self.frame_failures, ReadError { offset, failure })
    }
}

/// A segment whose complete items were framed without link or content checks.
#[derive(Debug)]
#[non_exhaustive]
pub struct SkippedSegment {
    /// File-relative segment ordinal.
    pub ordinal: u64,
    /// The opener's position and the reason the segment was skipped.
    pub opener: ReadError,
    /// Item count including the opener and each undelimitable region.
    pub items: usize,
}

/// A lineage finding at a segment opener.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LineageFinding {
    /// A later segment begins a lineage without a predecessor.
    Begins,
    /// The declared previous head differs from the preceding segment's head.
    HeadMismatch,
    /// The declared ordinal has no preceding segment or is out of sequence.
    OrdinalMismatch,
}

/// The end of a readable GTS v2 file under P7 §1.2.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum FileEnd {
    /// The item limit prevented reading the rest of the file.
    NotRead,
    /// The input ends in an undelimitable region.
    Undelimitable,
    /// The input ends inside an item.
    Torn,
    /// The last segment holds no valid commit.
    #[default]
    NoCommit,
    /// Uncommitted ordinary material follows the last valid commit.
    AfterCommit,
    /// The last valid commit carries an authenticating signature.
    AtAuthenticatedCommit,
    /// The end is at a valid commit.
    AtValidCommit,
}

/// An unread suffix caused by the read-wide item limit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ItemRefusal {
    /// The first unread byte.
    pub offset: usize,
    /// Length of the unread suffix.
    pub length: usize,
    /// The caller's item limit.
    pub limit: u64,
}

impl FileEnd {
    const fn observe_uncommitted(&mut self) {
        if matches!(self, Self::AtAuthenticatedCommit | Self::AtValidCommit) {
            *self = Self::AfterCommit;
        }
    }
}

/// The checked signatures of an ordinary frame.
#[derive(Debug)]
#[non_exhaustive]
pub struct FrameSignatures {
    /// File-relative segment ordinal of the frame.
    pub segment: u64,
    /// Item position within the segment, counting the opener as zero.
    pub position: usize,
    /// File-relative byte offset of the frame.
    pub offset: usize,
    /// Signature results in their original record order.
    pub checks: Vec<SignatureCheck>,
}

/// Commit coverage of one readable segment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct SegmentDurability {
    /// File-relative segment ordinal.
    pub segment: u64,
    /// Ordinary frames through the last valid commit, including the commit.
    pub committed_frames: u64,
    /// Counted ordinary frames after the last valid commit, or from the opener
    /// when no valid commit was found.
    pub uncommitted_tail: Option<UncommittedTail>,
}

/// Ordinary frames outside a segment's committed prefix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct UncommittedTail {
    /// Item position of the first counted frame, with the opener at zero.
    pub position: usize,
    /// Ordinary frames with an effective identifier, excluding indexes.
    pub count: u64,
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

/// A recovery scan reached the total decoded-byte limit in a readable segment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct RecoveryRefusal {
    /// File-relative ordinal of the segment whose recovery scan was refused.
    pub segment: u64,
    /// Item position where the candidate begins, counting the opener as zero.
    pub position: usize,
    /// Byte offset of the refused candidate.
    pub offset: usize,
    /// Encoded size of a parity candidate or the fixed resync charge.
    pub length: usize,
    /// Total decoded bytes including the refused charge, saturated at the wire limit.
    pub requested_total: u64,
}

/// The extent and kind of a framing break in the input.
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

impl FramingDamage {
    const fn offset(self) -> usize {
        match self {
            Self::TornTail { offset, .. } | Self::Undelimitable { offset, .. } => offset,
        }
    }
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
    /// A generation 2 item needs more nesting than admitted.
    ItemDepth {
        /// First nesting depth beyond the capacity.
        required: usize,
        /// Configured generation 2 item nesting ceiling.
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
    /// The link hash of a newer registry is unavailable.
    UnavailableHash(NonZeroU64),
    /// Lineage does not match the previous segment.
    Lineage,
    /// An item kind requires additional interpretation.
    Item(ItemKind),
    /// A frame type requires additional interpretation.
    Frame(FrameType),
    /// An unknown frame type in a newer registry.
    UnknownFrame(UnknownFrameType),
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
    /// An ML-DSA verification operation could not complete.
    Signature(crate::mldsa65::Error),
    /// A strict RDF projection cannot use a recovered resync prefix.
    RecoveredResync(Resynchronization),
    /// A strict RDF projection cannot use a prefix with a parity resumption.
    RecoveredParity(ParityRecovery),
    /// A strict RDF projection cannot use a prefix with a refused recovery.
    RecoveryRefused(RecoveryRefusal),
    /// A strict RDF projection requires the file to end at an item boundary.
    FramingTail {
        /// The unchecked range of the supplied input.
        damage: FramingDamage,
        /// The native CBOR framing failure, when the reader observed it.
        cause: Option<StructuralError>,
    },
    /// The graph records an omission without a more specific retained cause.
    OmittedContent,
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
            ReadFailure::Signature(error) => error.fmt(f),
            ReadFailure::UnavailableHash(algorithm) => {
                write!(
                    f,
                    "the segment uses unavailable link hash algorithm {algorithm}"
                )
            }
            ReadFailure::RecoveredResync(_) => {
                f.write_str("the RDF fold omits material before a resync opener")
            }
            ReadFailure::RecoveredParity(_) => {
                f.write_str("the RDF fold omits material before a parity resumption")
            }
            ReadFailure::RecoveryRefused(_) => {
                f.write_str("the decoded-byte limit prevented recovery")
            }
            ReadFailure::FramingTail {
                damage: FramingDamage::TornTail { .. },
                ..
            } => f.write_str("the file ends inside an item"),
            ReadFailure::FramingTail {
                damage: FramingDamage::Undelimitable { .. },
                ..
            } => f.write_str("the file ends with undelimitable bytes"),
            ReadFailure::OmittedContent => f.write_str("the RDF fold omits content"),
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
            ReadFailure::Signature(error) => Some(error),
            ReadFailure::DetectionDepth { cause, .. } | ReadFailure::ItemDepth { cause, .. } => {
                Some(cause)
            }
            ReadFailure::DetectionBytes {
                cause: Some(cause), ..
            }
            | ReadFailure::DetectionWork {
                cause: Some(cause), ..
            }
            | ReadFailure::FramingTail {
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
/// and RDF positions. Frames with damaged payloads and rows held by unreadable
/// definitions stay outside the fold and carry findings. Complete opaque items
/// put subsequent term resolution in doubt. An item that requires unsupported
/// content interpretation prevents a complete graph projection.
/// An unreadable opener or unavailable link hash leaves its segment's items
/// delimited and counted until the next opener.
/// Reaching the item limit after opening a segment preserves the checks on
/// the prefix and leaves the file fold empty.
/// The reader can recover at a verified resync opener after failure of the
/// framing test or opening generation recognition. A verified parity frame
/// can resume a readable segment after undelimitable bytes. Broken frames
/// retain their original link findings even when the reader recovers.
/// If the final segment ends inside or outside the item grammar, the reader
/// returns the checked prefix and the extent of the trailing bytes.
/// A recovery candidate refused by the total decoded-byte budget is reported
/// with its input location. Later complete frames retain their wire checks.
/// Parity findings describe the supplied bytes before any repair.
/// Indexes report comparisons against the observed prefix and never supply
/// graph content.
/// Signatures are checked with empty held-key and trust sets.
///
/// # Errors
/// Returns the byte position and reason when a graph cannot be produced.
pub fn read(bytes: &[u8], capacity: ReadCapacity) -> Result<ReadGraph<'_>, ReadError> {
    read_with_signature_inputs(bytes, capacity, &SignatureInputs::default())
}

/// Read a GTS file using caller-held keys and caller trust for generation 2.
///
/// Signature results use the final effective identifier of each retained
/// frame. The generation 1 result is produced by its original reader.
/// Use [`read_with_limits_and_signature_inputs`] for finite detection and
/// item nesting limits.
/// # Errors
/// Returns the byte position and reason when a graph cannot be produced.
pub fn read_with_signature_inputs<'a>(
    bytes: &'a [u8],
    capacity: ReadCapacity,
    signatures: &SignatureInputs<'_>,
) -> Result<ReadGraph<'a>, ReadError> {
    require_second_generation_item(bytes, capacity)?;
    let opener = match delimit(bytes, 0) {
        Ok(Some(item)) => item,
        Ok(None) => {
            return Err(ReadError {
                offset: 0,
                failure: ReadFailure::Empty,
            });
        }
        Err(error) => {
            return GraphReader::new(bytes, capacity, None, None, signatures)
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
        Generation::Second | Generation::Unrecognized => {
            GraphReader::new(bytes, capacity, None, None, signatures)
                .read_first(opener, generation)
                .map(ReadGraph::Second)
        }
        generation => Err(ReadError {
            offset: 0,
            failure: ReadFailure::Generation(generation),
        }),
    }
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
    read_with_limits_and_signature_inputs(bytes, limits, &SignatureInputs::default())
}

/// Decode a GTS file with finite read limits and caller-supplied signature inputs.
///
/// # Errors
/// Returns the byte position and reason when a graph cannot be produced.
pub fn read_with_limits_and_signature_inputs<'a>(
    bytes: &'a [u8],
    limits: ReadLimits,
    signatures: &SignatureInputs<'_>,
) -> Result<ReadGraph<'a>, ReadError> {
    let capacity = limits.second_generation;
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
            require_second_generation_item(bytes, capacity)?;
            return GraphReader::new(
                bytes,
                capacity,
                Some((limits.detection, detection_work)),
                Some(limits.second_generation_nesting_depth),
                signatures,
            )
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
        Generation::Second | Generation::Unrecognized => {
            require_second_generation_item(bytes, capacity)?;
            GraphReader::new(
                bytes,
                capacity,
                Some((limits.detection, detection_work)),
                Some(limits.second_generation_nesting_depth),
                signatures,
            )
            .read_first(opener, generation)
            .map(ReadGraph::Second)
        }
        generation => Err(ReadError {
            offset: 0,
            failure: ReadFailure::Generation(generation),
        }),
    }
}

fn require_second_generation_item(bytes: &[u8], capacity: ReadCapacity) -> Result<(), ReadError> {
    if capacity.items == 0 && !bytes.is_empty() {
        return Err(ReadError {
            offset: 0,
            failure: ReadFailure::ItemCapacity(0),
        });
    }
    Ok(())
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

fn framing_damage(error: &ReadError, end: usize) -> Option<FramingDamage> {
    let ReadFailure::Framing(cause) = &error.failure else {
        return None;
    };
    let offset = error.offset;
    let length = end - offset;
    match cause.kind() {
        DecodeErrorKind::Truncated => Some(FramingDamage::TornTail { offset, length }),
        DecodeErrorKind::Malformed(_) => Some(FramingDamage::Undelimitable { offset, length }),
        _ => None,
    }
}

struct Segment<'a> {
    registry: NonZeroU64,
    verifier: SegmentVerifier,
    dictionary: SegmentDictionary,
    ordinal: Option<u64>,
    file_ordinal: u64,
    header_identifier: [u8; DIGEST_LENGTH],
    authentication: AuthenticationProgress,
    durability: SegmentDurability,
    end: FileEnd,
    units: FastMap<[u8; 32], Unit<'a>>,
    parity: ParityStream<'a>,
    index_storage: StorageIndex,
    index_content: IndexContent,
    pending_rows: Vec<PendingRows>,
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
    retained_frame: Option<GraphFrame<'a>>,
    storage_item: Option<IndexStorageItem>,
    opener: bool,
}

impl From<ResyncRefusal> for CandidateRefusal {
    fn from(refused: ResyncRefusal) -> Self {
        Self {
            offset: refused.offset,
            length: RESYNC_MAX_BYTES,
            requested_total: refused.requested_total,
        }
    }
}

struct PendingRows {
    offset: usize,
    scope: RowScope,
    rows: Vec<Row>,
}

enum Recovery<'a> {
    Resync(ResyncCandidate<'a>),
    Parity {
        offset: usize,
        region_counted: bool,
        candidate: ParityResumption<'a>,
    },
}

pub(super) enum RecoveryFailure {
    Capacity(CandidateRefusal),
    Depth(RecoveryDepth),
}

pub(super) struct RecoveryDepth {
    pub(super) offset: usize,
    pub(super) limit: usize,
    pub(super) cause: StructuralError,
    pub(super) detection: bool,
}

impl From<RecoveryDepth> for ReadError {
    fn from(depth: RecoveryDepth) -> Self {
        let RecoveryDepth {
            offset,
            limit,
            cause,
            detection,
        } = depth;
        let required = limit.saturating_add(1);
        let failure = if detection {
            ReadFailure::DetectionDepth {
                required,
                limit,
                cause,
            }
        } else {
            ReadFailure::ItemDepth {
                required,
                limit,
                cause,
            }
        };
        Self { offset, failure }
    }
}

#[derive(Default)]
struct HeldItems<'a> {
    complete: VecDeque<Item<'a>>,
    complete_end: usize,
    next: Option<Result<Option<Item<'a>>, ReadError>>,
    region_counted: bool,
}

impl<'a> HeldItems<'a> {
    fn peek(&mut self, reader: &GraphReader<'a, '_>, offset: usize) -> Option<Item<'a>> {
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
        reader: &mut GraphReader<'a, '_>,
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

struct GraphReader<'a, 'keys> {
    bytes: &'a [u8],
    scan: ResyncScan<'a>,
    initial_scan: Option<ResyncScan<'a>>,
    initial_end: Option<usize>,
    initial_refusal: Option<ReadError>,
    initial_exhausted: bool,
    initial_nesting_depth: Option<usize>,
    item_nesting_depth: Option<usize>,
    graph: RdfGraph<'a>,
    values: ValueStore,
    payload: PayloadBudget,
    past: PastChecks<'a>,
    formed: FormedBudget,
    items: u64,
    item_limit: u64,
    rebuilt: RebuildBudget,
    blob_claims: Vec<BlobClaim<'a>>,
    signature_inputs: &'keys SignatureInputs<'keys>,
}

impl<'a, 'keys> GraphReader<'a, 'keys> {
    fn new(
        bytes: &'a [u8],
        capacity: ReadCapacity,
        detection: Option<(DetectionCapacity, usize)>,
        item_nesting_depth: Option<usize>,
        signature_inputs: &'keys SignatureInputs<'keys>,
    ) -> Self {
        let (initial_scan, initial_end, initial_refusal) =
            detection.map_or((None, None, None), |(detection, used)| {
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
                (
                    Some(ResyncScan::with_depth(
                        &bytes[..end],
                        detection.nesting_depth,
                    )),
                    Some(end),
                    refusal,
                )
            });
        let initial_nesting_depth = detection.map(|(capacity, _)| capacity.nesting_depth);
        let scan = item_nesting_depth.map_or_else(
            || ResyncScan::new(bytes),
            |depth| ResyncScan::with_depth(bytes, depth),
        );
        Self {
            bytes,
            scan,
            initial_scan,
            initial_end,
            initial_refusal,
            initial_exhausted: false,
            initial_nesting_depth,
            item_nesting_depth,
            graph: RdfGraph::default(),
            values: ValueStore::default(),
            payload: PayloadBudget::new(capacity.payload),
            past: PastChecks::new(capacity.payload.decoded_bytes_per_read),
            formed: FormedBudget::new(capacity.formed_bytes),
            items: 0,
            item_limit: capacity.items,
            rebuilt: RebuildBudget::new(capacity.rebuilt_bytes),
            blob_claims: Vec::new(),
            signature_inputs,
        }
    }

    fn scan_end(&self) -> usize {
        self.initial_end.unwrap_or(self.bytes.len())
    }

    fn finish_detection(&mut self) {
        self.initial_scan = None;
        self.initial_end = None;
        self.initial_refusal = None;
        self.initial_exhausted = false;
        self.initial_nesting_depth = None;
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
        mut self,
        opener: Item<'a>,
        generation: Generation,
    ) -> Result<RdfGraph<'a>, ReadError> {
        let end = opener.bytes().len();
        let failure = if generation == Generation::Second {
            self.admit_body_nesting(opener.bytes(), 0)?;
            match decode_opener(opener) {
                Ok(header) => {
                    self.finish_detection();
                    return self.read_opened(header, 0, end);
                }
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
        let mut items = HeldItems::default();
        let unreadable_opener =
            framed_end.filter(|_| matches!(initial.failure, ReadFailure::Wire(_)));
        let end = if let Some(end) = framed_end {
            self.count_item(0)?;
            if let Err(error) = self.count_held_items(end, &mut items) {
                if let Some(opener_end) = unreadable_opener
                    && matches!(error.failure, ReadFailure::ItemCapacity(_))
                {
                    return self.read_skipped_initial(initial, opener_end, items, None);
                }
                return Err(error);
            }
            end
        } else if self.count_undelimited(&initial, false)?.is_some() {
            self.scan_end()
        } else {
            return Err(initial);
        };
        let refusal = match self.scan_resync(0, end) {
            Ok(Some(candidate)) => {
                let start = candidate.offset;
                let end = start + candidate.bytes.len();
                self.admit_body_nesting(candidate.bytes, start)?;
                self.finish_detection();
                let segment = self.resume_segment(initial, &candidate, None)?;
                return self.read_segment(segment, start, end, HeldItems::default());
            }
            Ok(None) => None,
            Err(RecoveryFailure::Capacity(refused)) => Some(refused),
            Err(RecoveryFailure::Depth(depth)) => return Err(depth.into()),
        };
        if let Some(refusal) = self.detection_refusal(&initial) {
            return Err(refusal);
        }
        if let Some(opener_end) = unreadable_opener {
            return self.read_skipped_initial(initial, opener_end, items, refusal);
        }
        Err(refusal.map_or(initial, |refused| ReadError {
            offset: refused.offset,
            failure: ReadFailure::PayloadRefusal(PayloadRefusal::TotalDecoded(
                refused.requested_total,
            )),
        }))
    }

    fn read_skipped_initial(
        mut self,
        initial: ReadError,
        opener_end: usize,
        mut items: HeldItems<'a>,
        refusal: Option<CandidateRefusal>,
    ) -> Result<RdfGraph<'a>, ReadError> {
        self.finish_detection();
        let mut offset = opener_end;
        for item in &items.complete {
            self.admit_body_nesting(item.bytes(), offset)?;
            offset += item.bytes().len();
        }
        // The lookahead already charged an undelimitable region. Delimit it
        // under the body bounds without attempting to acquire another slot.
        items.next = items
            .region_counted
            .then(|| self.delimit_item(self.bytes.len(), offset));
        self.skip_segment(initial)?;
        if let Some(refused) = refusal {
            self.observe_recovery_refusal(refused, opener_end, None);
        }
        self.read_segment(None, 0, opener_end, items)
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
    ) -> Result<Option<ResyncCandidate<'a>>, RecoveryFailure> {
        if self.items == self.item_limit {
            return Ok(None);
        }
        let initial_end = self.initial_end;
        let initial = self.initial_scan.is_some();
        let (scanned, depth_refusal) = if let Some(scan) = &mut self.initial_scan {
            let scanned = scan.scan(start..end, &mut self.payload);
            self.initial_exhausted = scanned.as_ref().is_ok_and(Option::is_none)
                && initial_end.is_some_and(|admitted| scan.next_offset() >= admitted);
            let depth_refusal = scan.take_depth_refusal();
            (scanned, depth_refusal)
        } else {
            let scanned = self.scan.scan(start..end, &mut self.payload);
            let depth_refusal = self.scan.take_depth_refusal();
            (scanned, depth_refusal)
        };
        match (scanned, depth_refusal) {
            (Ok(None), Some((candidate_offset, limit, error))) => {
                let error_offset = usize::try_from(error.offset()).unwrap_or(usize::MAX);
                Err(RecoveryFailure::Depth(RecoveryDepth {
                    offset: candidate_offset.saturating_add(error_offset),
                    limit,
                    cause: error,
                    detection: initial,
                }))
            }
            (Ok(candidate), _) => Ok(candidate),
            (Err(refused), _) => Err(RecoveryFailure::Capacity(refused.into())),
        }
    }

    fn scan_recovery(
        &mut self,
        failure: &ReadError,
        scan_start: usize,
        region_counted: bool,
        segment: Option<&Segment<'a>>,
    ) -> Result<Option<Recovery<'a>>, ReadError> {
        let Some(region_counted) = self.count_undelimited(failure, region_counted)? else {
            return Ok(None);
        };
        if self.items == self.item_limit || self.payload.total_refused() {
            return Ok(None);
        }
        let damaged_start = failure.offset;
        match self.search_recovery(scan_start, damaged_start, region_counted, segment) {
            Ok(candidate) => Ok(candidate),
            Err(RecoveryFailure::Capacity(refused)) => {
                self.observe_recovery_refusal(refused, damaged_start, segment);
                Ok(None)
            }
            Err(RecoveryFailure::Depth(depth)) => Err(depth.into()),
        }
    }

    fn search_recovery(
        &mut self,
        scan_start: usize,
        damaged_start: usize,
        region_counted: bool,
        segment: Option<&Segment<'a>>,
    ) -> Result<Option<Recovery<'a>>, RecoveryFailure> {
        if let Some(candidate) = self.scan_resync(scan_start, damaged_start)? {
            return Ok(Some(Recovery::Resync(candidate)));
        }
        let parity_slots = if region_counted { 1 } else { 2 };
        let history = segment
            .filter(|_| self.item_limit - self.items >= parity_slots)
            .map(|segment| segment.verifier.parity_history());
        for offset in damaged_start..self.bytes.len() {
            // Both candidate types start with a deterministic 11-field array.
            let bytes = &self.bytes[offset..];
            if bytes.first() != Some(&0x8b) {
                continue;
            }
            if let Some(candidate) = self.scan_resync(offset, offset + 1)? {
                return Ok(Some(Recovery::Resync(candidate)));
            }
            if let Some(history) = &history
                && let Some(candidate) =
                    history.scan_at(bytes, offset, &mut self.payload, self.item_nesting_depth)?
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

    fn observe_recovery_refusal(
        &mut self,
        refused: CandidateRefusal,
        damaged_start: usize,
        segment: Option<&Segment<'a>>,
    ) {
        let (ordinal, items) = if let Some(segment) = segment {
            (segment.file_ordinal, segment.parity.item_count())
        } else {
            let segment = self
                .graph
                .skipped_segments
                .last()
                .expect("an unlinked walk begins by recording its skipped opener");
            (segment.ordinal, segment.items)
        };
        let position = items - usize::from(refused.offset < damaged_start);
        self.graph.recovery_refusal = Some(RecoveryRefusal {
            segment: ordinal,
            position,
            offset: refused.offset,
            length: refused.length,
            requested_total: refused.requested_total,
        });
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
        self.graph.left_out += 1;
        let accept = || {
            segment.parity.opaque_extent(region)?;
            segment.dictionary.mark_doubt();
            segment.end.observe_uncommitted();
            let detached = segment
                .verifier
                .resume_parity(candidate.anchor, &candidate.table)?;
            super::fields::push(
                &mut self.graph.frame_wire,
                FrameWireResult {
                    segment: segment.file_ordinal,
                    position: segment.parity.item_count(),
                    offset,
                    length: candidate.length,
                    frame_type: Some(FrameType::Parity as u64),
                    link: LinkState::Linked,
                    payload: PayloadOutcome::Verified,
                    stored: None,
                },
            )?;
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
    ) -> Result<Option<Segment<'a>>, ReadError> {
        let offset = original.offset;
        let damage = if candidate.offset > offset {
            let Err(prefix) = self.delimit_item(candidate.offset, offset) else {
                return Err(original);
            };
            Some(framing_damage(&prefix, candidate.offset).ok_or(prefix)?)
        } else {
            None
        };
        let start = candidate.offset;
        let end = start + candidate.bytes.len();
        self.count_item(candidate.offset)?;
        self.graph.left_out += u64::from(damage.is_some());
        let mut previous = previous;
        if let Some(previous) = previous.as_deref_mut() {
            if let Some(FramingDamage::Undelimitable { offset, length }) = damage {
                previous
                    .parity
                    .opaque_extent(offset..offset + length)
                    .map_err(|error| ReadError {
                        offset,
                        failure: error.into(),
                    })?;
            }
            self.finish_segment(previous, start)?;
        } else if !self.graph.skipped_segments.is_empty()
            && matches!(damage, Some(FramingDamage::Undelimitable { .. }))
        {
            self.count_skipped_item();
        }
        let segment = self.open_segment(
            candidate.header,
            start..end,
            previous.as_deref(),
            ResyncReach::Scanned,
        )?;
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
                self.delimit_item(limit, offset)
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
        let segment = self.open_segment(opener, start..end, None, ResyncReach::Walked)?;
        self.read_segment(segment, start, end, HeldItems::default())
    }

    fn open_segment(
        &mut self,
        header: Header<'a>,
        extent: Range<usize>,
        previous: Option<&Segment<'a>>,
        reach: ResyncReach,
    ) -> Result<Option<Segment<'a>>, ReadError> {
        let offset = extent.start;
        let registry = header.registry_version();
        let file_ordinal = self.next_segment_ordinal();
        if let Some(declaration) = header.resync() {
            let past = self
                .past
                .admit(
                    self.graph.resync_openers.len(),
                    offset,
                    declaration,
                    header.identifier().is_some(),
                )
                .map_err(|error| ReadError {
                    offset,
                    failure: error.into(),
                })?;
            super::fields::push(
                &mut self.graph.resync_openers,
                ResyncOpener {
                    segment: file_ordinal,
                    offset,
                    declared_offset: declaration.offset,
                    reach,
                    past,
                },
            )
            .map_err(|error| ReadError {
                offset,
                failure: error.into(),
            })?;
        }
        self.observe_lineage(header, offset, file_ordinal, previous)?;
        if registry.get() > super::REGISTRY_VERSION {
            super::fields::push(&mut self.graph.unsupported_registries, (offset, registry))
                .map_err(|error| ReadError {
                    offset,
                    failure: error.into(),
                })?;
        }
        let Some(identifier) = header.identifier() else {
            self.skip_segment(ReadError {
                offset,
                failure: ReadFailure::UnavailableHash(header.hash_algorithm()),
            })?;
            return Ok(None);
        };
        segment_from_header(header, identifier, extent, file_ordinal)
            .map(Some)
            .map_err(|failure| ReadError { offset, failure })
    }

    fn observe_lineage(
        &mut self,
        header: Header<'a>,
        offset: usize,
        file_ordinal: u64,
        previous: Option<&Segment<'a>>,
    ) -> Result<(), ReadError> {
        let lineage = header.lineage();
        let (head_mismatch, ordinal_mismatch) = match (lineage, header.identifier(), previous) {
            (Some(lineage), Some(_), Some(previous)) => (
                lineage.previous_head != previous.verifier.head(),
                previous
                    .ordinal
                    .is_some_and(|ordinal| ordinal.checked_add(1) != Some(lineage.ordinal.get())),
            ),
            (Some(_), Some(_), None) => (false, file_ordinal == 0),
            _ => (false, false),
        };
        for (finding, applies) in [
            (
                LineageFinding::Begins,
                file_ordinal > 0 && lineage.is_none(),
            ),
            (LineageFinding::HeadMismatch, head_mismatch),
            (LineageFinding::OrdinalMismatch, ordinal_mismatch),
        ] {
            if applies {
                super::fields::push(&mut self.graph.lineage_findings, (offset, finding)).map_err(
                    |error| ReadError {
                        offset,
                        failure: error.into(),
                    },
                )?;
            }
        }
        Ok(())
    }

    fn next_segment_ordinal(&self) -> u64 {
        (self.graph.segment_authentication.len() + self.graph.skipped_segments.len()) as u64
    }

    fn skip_segment(&mut self, opener: ReadError) -> Result<(), ReadError> {
        let offset = opener.offset;
        let ordinal = self.next_segment_ordinal();
        self.graph.note_content_omission(offset);
        super::fields::push(
            &mut self.graph.skipped_segments,
            SkippedSegment {
                ordinal,
                opener,
                items: 1,
            },
        )
        .map_err(|error| ReadError {
            offset,
            failure: error.into(),
        })
    }

    fn count_skipped_item(&mut self) {
        let segment = self
            .graph
            .skipped_segments
            .last_mut()
            .expect("an unlinked walk begins by recording its skipped opener");
        segment.items += 1;
    }

    fn read_segment(
        mut self,
        mut segment: Option<Segment<'a>>,
        mut previous_start: usize,
        mut offset: usize,
        mut items: HeldItems<'a>,
    ) -> Result<RdfGraph<'a>, ReadError> {
        let mut held: Option<GraphFrame<'a>> = None;
        loop {
            let item = match items.take(&mut self, offset) {
                Ok(Some(item)) => item,
                Ok(None) => break,
                Err(ReadError {
                    offset,
                    failure: ReadFailure::ItemCapacity(limit),
                }) => {
                    self.graph.item_refusal = Some(ItemRefusal {
                        offset,
                        length: self.bytes.len() - offset,
                        limit,
                    });
                    break;
                }
                Err(error) => {
                    match self.scan_recovery(
                        &error,
                        previous_start + 1,
                        items.region_counted,
                        segment.as_ref(),
                    )? {
                        Some(Recovery::Resync(candidate)) => {
                            previous_start = candidate.offset;
                            offset = candidate.offset + candidate.bytes.len();
                            segment = self.resume_segment(error, &candidate, segment.as_mut())?;
                        }
                        Some(Recovery::Parity {
                            offset: start,
                            region_counted,
                            candidate,
                        }) => {
                            previous_start = start;
                            offset = start + candidate.length;
                            let current = segment.as_mut().ok_or(ReadError {
                                offset: start,
                                failure: ReadFailure::MissingContent,
                            })?;
                            self.resume_parity(
                                error.offset..start,
                                region_counted,
                                candidate,
                                current,
                            )?;
                        }
                        None => {
                            let Some(damage) = framing_damage(&error, self.bytes.len()) else {
                                return Err(error);
                            };
                            if segment.is_none()
                                && matches!(damage, FramingDamage::Undelimitable { .. })
                            {
                                self.count_skipped_item();
                            }
                            self.graph.framing_tail = Some(damage);
                            if let ReadFailure::Framing(cause) = error.failure {
                                self.graph.framing_tail_cause = Some(cause);
                            }
                            self.graph.left_out += 1;
                            break;
                        }
                    }
                    items = HeldItems::default();
                    continue;
                }
            };
            let end = offset + item.bytes().len();
            if let Some(failed) =
                self.observe_item(item, offset..end, &mut held, &mut items, &mut segment)?
            {
                let scan_start = if held.is_some() {
                    offset
                } else {
                    previous_start + 1
                };
                let candidate = match self.count_held_items(end, &mut items) {
                    Ok(()) => match self.scan_resync(scan_start, end) {
                        Ok(candidate) => candidate,
                        Err(RecoveryFailure::Capacity(refused)) => {
                            if let Some(current) = segment.as_mut() {
                                self.deliver_held(held.take(), current)?;
                            }
                            self.observe_recovery_refusal(refused, offset, segment.as_ref());
                            None
                        }
                        Err(RecoveryFailure::Depth(depth)) => return Err(depth.into()),
                    },
                    Err(error) if matches!(error.failure, ReadFailure::ItemCapacity(_)) => None,
                    Err(error) => return Err(error),
                };
                if candidate.is_none()
                    && let Some(current) = segment.as_mut()
                {
                    self.deliver_held(held.take(), current)?;
                }
                if let Some(frame) = failed.retained_frame
                    && candidate.is_none()
                {
                    let current = segment.as_mut().ok_or(ReadError {
                        offset,
                        failure: ReadFailure::MissingContent,
                    })?;
                    self.deliver_frame(frame, current)?;
                    previous_start = offset;
                    offset = end;
                    continue;
                }
                let original = ReadError {
                    offset,
                    failure: failed.failure,
                };
                let Some(candidate) = candidate else {
                    if failed.opener {
                        if let Some(mut previous) = segment.take() {
                            self.finish_segment(&mut previous, offset)?;
                        }
                        self.skip_segment(original)?;
                    } else if let Some(current) = segment.as_mut() {
                        match (failed.storage_item, &original.failure) {
                            (Some(item), ReadFailure::Wire(error)) => {
                                Self::observe_storage_claim(
                                    offset..end,
                                    item,
                                    Err(*error),
                                    current,
                                )?;
                            }
                            (_, ReadFailure::Classification(_)) => {
                                self.retain_opaque_item(original, end, current)?;
                                self.graph.left_out += 1;
                                if !item.names_closing_hint() {
                                    self.graph.note_content_omission(offset);
                                    current.end.observe_uncommitted();
                                }
                            }
                            _ => return Err(original),
                        }
                    } else {
                        self.count_skipped_item();
                        self.graph.left_out += 1;
                    }
                    previous_start = offset;
                    offset = end;
                    continue;
                };
                if let (Some(mut frame), Some(current)) = (held.take(), segment.as_mut()) {
                    // The accepted opener replaces this frame's only witness.
                    let link = &mut frame.checked.link;
                    link.state = LinkState::Broken;
                    link.effective_identifier = link.stored_identifier.or(link.computed_identifier);
                    self.deliver_frame(frame, current)?;
                }
                previous_start = candidate.offset;
                offset = candidate.offset + candidate.bytes.len();
                segment = self.resume_segment(original, &candidate, segment.as_mut())?;
                items = HeldItems::default();
                continue;
            }
            previous_start = offset;
            offset = end;
        }
        if let Some(current) = segment.as_mut() {
            self.finish_segment(current, offset)?;
        }
        self.graph.end = match (self.graph.item_refusal, self.graph.framing_tail) {
            (Some(_), _) => {
                self.graph.rdf = RdfFold::default();
                self.graph.annotation_width_offset = None;
                self.graph.blobs.clear();
                self.graph.external_blobs.clear();
                self.graph.frame_failures.retain(|error| {
                    !matches!(
                        error.failure,
                        ReadFailure::Definitions(_) | ReadFailure::Rows(_)
                    )
                });
                FileEnd::NotRead
            }
            (_, Some(FramingDamage::Undelimitable { .. })) => FileEnd::Undelimitable,
            (_, Some(FramingDamage::TornTail { .. })) => FileEnd::Torn,
            (_, None) => segment.map_or(FileEnd::NoCommit, |segment| segment.end),
        };
        if self.graph.item_refusal.is_none() {
            self.fold_blob_claims().map_err(|error| ReadError {
                offset,
                failure: error.into(),
            })?;
        }
        self.graph.frame_failures.sort_unstable_by_key(|error| {
            let detail = match error.failure {
                ReadFailure::UnknownFrame(_) => 2,
                ReadFailure::PayloadDamage(_) | ReadFailure::PayloadRefusal(_) => 1,
                _ => 0,
            };
            (error.offset, detail)
        });
        self.past.finish(self.bytes, &mut self.graph.resync_openers);
        Ok(self.graph)
    }

    fn finish_segment(
        &mut self,
        segment: &mut Segment<'a>,
        offset: usize,
    ) -> Result<(), ReadError> {
        self.flush_rows(segment, false)?;
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
                self.item_nesting_depth,
            )
            .map_err(|failure| ReadError { offset, failure })?;
        segment
            .index_storage
            .finish(
                segment.parity.item_count(),
                &mut self.graph.index_storage_findings,
            )
            .map_err(read_error)?;
        super::fields::push(
            &mut self.graph.segment_authentication,
            segment.authentication.summary(segment.file_ordinal),
        )
        .map_err(read_error)?;
        super::fields::push(&mut self.graph.segment_durability, segment.durability)
            .map_err(read_error)
    }

    fn retain_opaque_item(
        &mut self,
        error: ReadError,
        end: usize,
        segment: &mut Segment<'a>,
    ) -> Result<(), ReadError> {
        let offset = error.offset;
        segment.dictionary.mark_doubt();
        segment
            .parity
            .opaque_extent(offset..end)
            .and_then(|()| super::fields::push(&mut self.graph.opaque_items, error))
            .map_err(|cause| ReadError {
                offset,
                failure: cause.into(),
            })
    }

    fn observe_storage_claim(
        extent: Range<usize>,
        item: IndexStorageItem,
        claim: Result<bool, WireError>,
        segment: &mut Segment<'a>,
    ) -> Result<(), ReadError> {
        let offset = extent.start;
        segment
            .index_storage
            .observe(offset, segment.parity.item_count(), item, claim)
            .and_then(|()| segment.parity.other_item(extent))
            .map_err(|error| ReadError {
                offset,
                failure: error.into(),
            })
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
        self.delimit_item(end, offset)
    }

    fn delimit_item(&self, end: usize, offset: usize) -> Result<Option<Item<'a>>, ReadError> {
        let Some(limit) = self.initial_nesting_depth.or(self.item_nesting_depth) else {
            return delimit(&self.bytes[..end], offset);
        };
        Item::first_with_depth(&self.bytes[offset..end], limit).map_err(|error| {
            let error_offset = usize::try_from(error.offset()).unwrap_or(usize::MAX);
            let (reported_offset, failure) = match error.kind() {
                DecodeErrorKind::Depth { limit } if self.initial_nesting_depth.is_some() => (
                    offset.saturating_add(error_offset),
                    ReadFailure::DetectionDepth {
                        required: limit.saturating_add(1),
                        limit,
                        cause: error,
                    },
                ),
                DecodeErrorKind::Depth { limit } => (
                    offset.saturating_add(error_offset),
                    ReadFailure::ItemDepth {
                        required: limit.saturating_add(1),
                        limit,
                        cause: error,
                    },
                ),
                _ => (offset, ReadFailure::Framing(error)),
            };
            ReadError {
                offset: reported_offset,
                failure,
            }
        })
    }

    fn admit_body_nesting(&self, bytes: &'a [u8], offset: usize) -> Result<(), ReadError> {
        let Some(limit) = self.item_nesting_depth else {
            return Ok(());
        };
        match Item::first_with_depth(bytes, limit) {
            Ok(Some(_)) => Ok(()),
            Err(error) if matches!(error.kind(), DecodeErrorKind::Depth { .. }) => {
                let error_offset = usize::try_from(error.offset()).unwrap_or(usize::MAX);
                Err(ReadError {
                    offset: offset.saturating_add(error_offset),
                    failure: ReadFailure::ItemDepth {
                        required: limit.saturating_add(1),
                        limit,
                        cause: error,
                    },
                })
            }
            Err(error) => Err(ReadError {
                offset,
                failure: ReadFailure::Framing(error),
            }),
            Ok(_) => Err(ReadError {
                offset,
                failure: ReadFailure::Empty,
            }),
        }
    }

    fn observe_item(
        &mut self,
        item: Item<'a>,
        extent: Range<usize>,
        held: &mut Option<GraphFrame<'a>>,
        items: &mut HeldItems<'a>,
        active: &mut Option<Segment<'a>>,
    ) -> Result<Option<FailedItem<'a>>, ReadError> {
        let offset = extent.start;
        let registry = active
            .as_ref()
            .map_or(NonZeroU64::MAX, |segment| segment.registry);
        let kind = match item.classify(registry) {
            Ok(kind) => kind,
            Err(error) => {
                return Ok(Some(FailedItem {
                    failure: ReadFailure::Classification(error),
                    retained_frame: None,
                    storage_item: None,
                    opener: false,
                }));
            }
        };
        if matches!(kind, ItemKind::Header | ItemKind::Resync) {
            let header = match decode_opener(item) {
                Ok(header) => header,
                Err(failure) => {
                    return Ok(Some(FailedItem {
                        failure,
                        retained_frame: None,
                        storage_item: None,
                        opener: true,
                    }));
                }
            };
            if let Some(previous) = active.as_mut() {
                self.deliver_held(held.take(), previous)?;
                self.finish_segment(previous, offset)?;
            }
            *active = self.open_segment(header, extent, active.as_ref(), ResyncReach::Walked)?;
            return Ok(None);
        }
        let Some(segment) = active.as_mut() else {
            self.count_skipped_item();
            self.graph.left_out += 1;
            return Ok(None);
        };
        match kind {
            ItemKind::Frame => {
                let framing_error = |error| ReadError {
                    offset,
                    failure: ReadFailure::Framing(error),
                };
                let envelope = item.frame().map_err(framing_error)?.ok_or(ReadError {
                    offset,
                    failure: ReadFailure::MissingContent,
                })?;
                let predecessor = held
                    .as_ref()
                    .and_then(|frame| frame.checked.link.effective_identifier.as_ref())
                    .unwrap_or_else(|| segment.verifier.predecessor());
                // A frame under a held predecessor cannot heal from its successor.
                let witness = if held.is_none() {
                    items
                        .peek(self, extent.end)
                        .map(Item::frame)
                        .transpose()
                        .map_err(framing_error)?
                        .flatten()
                } else {
                    None
                };
                let wire = envelope.check_wire(predecessor, witness.as_ref(), registry);
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
                    let failure = match kind {
                        Some(kind @ (FrameType::Index | FrameType::Parity)) => {
                            ReadFailure::Frame(kind)
                        }
                        _ => frame
                            .checked
                            .malformed
                            .map_or(ReadFailure::BrokenLink, ReadFailure::Wire),
                    };
                    return Ok(Some(FailedItem {
                        failure,
                        retained_frame: Some(frame),
                        storage_item: None,
                        opener: false,
                    }));
                }
                if frame.checked.link.state == LinkState::Healed {
                    // Only the next item's framing can make this link final.
                    *held = Some(frame);
                } else {
                    self.deliver_frame(frame, segment)?;
                }
            }
            ItemKind::Offsets | ItemKind::Locator => {
                let (item, claim) = if kind == ItemKind::Offsets {
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
                if let Err(error) = claim {
                    return Ok(Some(FailedItem {
                        failure: ReadFailure::Wire(error),
                        retained_frame: None,
                        storage_item: Some(item),
                        opener: false,
                    }));
                }
                self.deliver_held(held.take(), segment)?;
                Self::observe_storage_claim(extent, item, claim, segment)?;
            }
            ItemKind::Unknown(_) => {
                self.deliver_held(held.take(), segment)?;
                self.retain_opaque_item(
                    ReadError {
                        offset,
                        failure: ReadFailure::Item(kind),
                    },
                    extent.end,
                    segment,
                )?;
                self.graph.left_out += 1;
                self.graph.note_content_omission(offset);
                segment.end.observe_uncommitted();
            }
            ItemKind::Header | ItemKind::Resync => {
                unreachable!("the segment walk opens a new segment before checking content");
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
        super::fields::push(
            &mut self.graph.frame_wire,
            FrameWireResult {
                segment: segment.file_ordinal,
                position: segment.parity.item_count(),
                offset,
                length: envelope.bytes().len(),
                frame_type: checked.frame_type,
                link: checked.link.state,
                payload: PayloadOutcome::from(&checked.payload.state),
                stored: checked.payload.stored,
            },
        )
        .map_err(|error| ReadError {
            offset,
            failure: error.into(),
        })?;
        let kind = checked.frame_type.and_then(FrameType::from_code);
        let covered = checked.link.effective_identifier.is_some();
        match self.observe_frame(envelope, checked, offset, segment) {
            Ok(true) => self.flush_rows(segment, true)?,
            Ok(false) => {}
            Err(failure @ ReadFailure::Content(ContentError::UnknownEnumeration { .. })) => {
                if kind.is_some_and(FrameType::critical) {
                    segment.verifier.stop_content();
                }
                if covered {
                    segment.index_content.opaque_frame(kind);
                }
                self.graph.left_out += 1;
                self.graph
                    .retain_content_failure(offset, failure)
                    .map_err(|error| ReadError {
                        offset,
                        failure: error.into(),
                    })?;
            }
            Err(failure) => return Err(ReadError { offset, failure }),
        }
        Ok(())
    }

    fn observe_frame(
        &mut self,
        frame: FrameEnvelope<'a>,
        checked: FrameCheck<'a>,
        offset: usize,
        segment: &mut Segment<'a>,
    ) -> Result<bool, ReadFailure> {
        let kind = checked.frame_type.and_then(FrameType::from_code);
        let accelerator = matches!(kind, Some(FrameType::Index | FrameType::Parity));
        let broken = checked.link.state == LinkState::Broken;
        let failed_wire = broken || checked.malformed.is_some();
        let opaque = !accelerator
            && (failed_wire || !matches!(checked.payload.state, PayloadState::Verified(_)));
        if kind == Some(FrameType::Index)
            && failed_wire
            && let PayloadState::Unchecked(refusal @ PayloadRefusal::TotalDecoded(_)) =
                checked.payload.state
        {
            super::fields::push(
                &mut self.graph.frame_failures,
                ReadError {
                    offset,
                    failure: ReadFailure::PayloadRefusal(refusal),
                },
            )?;
        }
        if kind != Some(FrameType::Parity) && checked.payload.stored == Some(StoredState::Mismatch)
        {
            super::fields::push(&mut self.graph.stored_digest_mismatches, offset)?;
        }
        let position = segment.verifier.ordinary_count();
        // The frame consumes the current anchor. Consecutive recoveries can
        // change it before another ordinary frame is retained.
        if checked.link.effective_identifier.is_some()
            && segment.verifier.predecessor() != segment.verifier.head()
        {
            segment.authentication.begin_run(position);
        }
        let stopped = segment.verifier.content_stopped();
        let mut checked = segment
            .verifier
            .accept_checked(&frame, checked, &self.payload, Some(&segment.index_content))
            .map_err(ReadFailure::Segment)?;
        let omitted = if kind == Some(FrameType::Parity) {
            matches!(checked.parity, Some(ParityCheck::UnknownGeometry { .. }))
        } else {
            stopped
                || (broken && !frame.transparent())
                || checked.frame.malformed.is_some()
                || checked.frame.unknown_type.is_some()
                || !matches!(checked.frame.payload.state, PayloadState::Verified(_))
                || checked.index.as_ref().is_some_and(Result::is_err)
        };
        self.graph.left_out += u64::from(omitted);
        let committed = checked.commit.is_some_and(|commit| commit.valid);
        let authenticates = self.observe_signatures(frame, &checked, offset, position, segment)?;
        let covered = checked.frame.link.effective_identifier.is_some();
        let counted = covered && kind != Some(FrameType::Index);
        if counted {
            segment.authentication.count_frame(!broken);
        }
        if authenticates {
            segment.authentication.authenticate(committed);
        }
        if committed {
            segment.durability.committed_frames = segment.verifier.ordinary_count();
            segment.durability.uncommitted_tail = None;
            segment.end = if authenticates {
                FileEnd::AtAuthenticatedCommit
            } else {
                FileEnd::AtValidCommit
            };
        } else if !frame.transparent() && kind != Some(FrameType::Index) {
            segment.end.observe_uncommitted();
            if counted {
                let tail =
                    segment
                        .durability
                        .uncommitted_tail
                        .get_or_insert_with(|| UncommittedTail {
                            position: segment.parity.item_count(),
                            count: 0,
                        });
                tail.count += 1;
            }
        }
        if let Some(commit) = checked.commit.filter(|commit| !commit.valid) {
            let findings = if segment.verifier.incomplete_prefix() {
                &mut self.graph.interrupted_commits
            } else {
                &mut self.graph.invalid_commits
            };
            super::fields::push(findings, (offset, commit))?;
        }
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
        if broken && !frame.transparent() {
            segment.dictionary.mark_broken_link();
        }
        let unknown = checked.frame.unknown_type;
        if stopped
            && (kind != Some(FrameType::Parity)
                || matches!(checked.parity, Some(ParityCheck::UnknownGeometry { .. })))
        {
            super::fields::push(&mut self.graph.stopped_frames, offset)?;
        }
        if let Some(unknown) = unknown {
            self.graph.note_content_omission(offset);
            super::fields::push(
                &mut self.graph.frame_failures,
                ReadError {
                    offset,
                    failure: ReadFailure::UnknownFrame(unknown),
                },
            )?;
        }
        if opaque {
            if covered {
                segment.index_content.opaque_frame(kind);
            }
            if kind != Some(FrameType::Commit) {
                self.graph.note_content_omission(offset);
            }
            if !stopped && let Some(reservation) = &checked.terms {
                segment
                    .dictionary
                    .observe_terms(&mut self.graph.rdf, reservation, None, &self.values)
                    .map_err(ReadFailure::Fold)?;
            }
            let primary = checked
                .frame
                .malformed
                .map(ReadFailure::Wire)
                .or_else(|| broken.then_some(ReadFailure::BrokenLink));
            let payload = match checked.frame.payload.state {
                PayloadState::Verified(_) => None,
                PayloadState::Damaged(PayloadDamage::Malformed) if primary.is_some() => None,
                state => Some(payload_failure(state)),
            };
            for failure in [primary, payload].into_iter().flatten() {
                super::fields::push(
                    &mut self.graph.frame_failures,
                    ReadError { offset, failure },
                )?;
            }
            return Ok(committed);
        }
        if unknown.is_some() {
            return Ok(committed);
        }
        let kind = kind.ok_or(ReadFailure::MissingContent)?;
        if stopped
            && !matches!(
                kind,
                FrameType::Commit | FrameType::Parity | FrameType::Index
            )
        {
            if covered {
                segment.index_content.opaque_frame(Some(kind));
            }
            return Ok(committed);
        }
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
                self.observe_terms_frame(&mut checked, offset, segment)?;
            }
            FrameType::Quads
            | FrameType::ReifierBindings
            | FrameType::Annotations
            | FrameType::EmptyGraphs => {
                self.observe_rows(checked, offset, position, segment)?;
            }
            FrameType::Blob
            | FrameType::Chunk
            | FrameType::Recipe
            | FrameType::Value
            | FrameType::External
            | FrameType::Pack => {
                let bytes = verified_payload(checked.frame.payload.state)?;
                let result = self.observe_blob_frame(
                    kind,
                    frame.read_field(FrameField::Content),
                    bytes,
                    (offset, position),
                    segment,
                );
                match result {
                    Err(failure @ ReadFailure::Wire(_)) => {
                        segment.index_content.opaque_frame(Some(kind));
                        self.graph.left_out += 1;
                        self.graph.retain_content_failure(offset, failure)?;
                    }
                    result => result?,
                }
            }
            FrameType::Commit if checked.commit.is_some() => {
                return Ok(committed);
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
        Ok(false)
    }

    fn observe_terms_frame(
        &mut self,
        checked: &mut SegmentFrame<'_>,
        offset: usize,
        segment: &mut Segment<'_>,
    ) -> Result<(), ReadFailure> {
        let reservation = checked.terms.as_mut().ok_or(ReadFailure::MissingContent)?;
        let decoded = match reservation.content.take() {
            Some(Err(error)) => Err(error),
            content => {
                reservation.content = content;
                segment
                    .verifier
                    .decode_terms(checked, &mut self.formed)
                    .ok_or(ReadFailure::MissingContent)?
            }
        };
        let (terms, failure) = match decoded {
            Ok(terms) => (Some(terms), None),
            Err(error @ (ContentError::Malformed(_) | ContentError::FormedCapacity(_))) => {
                (None, Some(ReadFailure::Content(error)))
            }
            Err(error) => return Err(ReadFailure::Content(error)),
        };
        let reservation = checked.terms.as_ref().ok_or(ReadFailure::MissingContent)?;
        let findings = segment
            .dictionary
            .observe_terms(&mut self.graph.rdf, reservation, terms, &self.values)
            .map_err(ReadFailure::Fold)?;
        if let Some(failure) = failure {
            self.graph.left_out += 1;
            self.graph.retain_content_failure(offset, failure)?;
        }
        if findings.ill_formed_term
            || findings.forward_reference
            || findings.position_constraint
            || !findings.missing_values.is_empty()
        {
            self.graph
                .retain_content_failure(offset, ReadFailure::Definitions(findings))?;
        }
        Ok(())
    }

    fn observe_rows(
        &mut self,
        mut checked: SegmentFrame<'_>,
        offset: usize,
        position: u64,
        segment: &mut Segment<'_>,
    ) -> Result<(), ReadFailure> {
        let decoded = match checked.rows.take() {
            Some(Err(error)) => Err(ReadFailure::Content(error)),
            rows => {
                checked.rows = rows;
                segment
                    .verifier
                    .decode_rows(&checked)
                    .ok_or(ReadFailure::MissingContent)?
                    .map_err(ReadFailure::Content)
            }
        };
        let decoded = match decoded.and_then(|decoded| {
            segment
                .dictionary
                .check_annotations(&decoded.rows)
                .map_err(|error| match error {
                    FoldError::RepeatedAnnotation => ReadFailure::Content(
                        WireError::new(MalformedReason::UnorderedOrRepeated, 0).into(),
                    ),
                    error => ReadFailure::Fold(error),
                })?;
            Ok(decoded)
        }) {
            Ok(decoded) => decoded,
            Err(failure @ ReadFailure::Content(ContentError::Malformed(_))) => {
                segment.verifier.omit_row_order(&checked);
                segment
                    .index_content
                    .opaque_frame(checked.frame.frame_type.and_then(FrameType::from_code));
                self.graph.left_out += 1;
                self.graph.retain_content_failure(offset, failure)?;
                return Ok(());
            }
            Err(failure) => return Err(failure),
        };
        segment.index_content.record_rows(&decoded.rows, position)?;
        if decoded.declared_order_violated {
            super::fields::push(&mut self.graph.row_order_violations, offset)?;
        }
        let scope = segment.dictionary.row_scope();
        if scope.waits_for_commit() || !segment.pending_rows.is_empty() {
            return super::fields::push(
                &mut segment.pending_rows,
                PendingRows {
                    offset,
                    scope,
                    rows: decoded.rows,
                },
            )
            .map_err(ReadFailure::from);
        }
        self.fold_rows(&segment.dictionary, &decoded.rows, scope, offset)
    }

    fn observe_signatures(
        &mut self,
        frame: FrameEnvelope<'a>,
        checked: &SegmentFrame<'a>,
        offset: usize,
        ordinary: u64,
        segment: &mut Segment<'a>,
    ) -> Result<bool, ReadFailure> {
        if frame.transparent() {
            return Ok(false);
        }
        let Some(identifier) = checked.frame.link.effective_identifier else {
            return Ok(false);
        };
        let link_holds = checked.frame.link.state != LinkState::Broken;
        if !link_holds {
            segment
                .authentication
                .begin_run(segment.verifier.ordinary_count());
        }
        if !checked.frame.signatures_valid
            || !self
                .payload
                .admits_content(frame.field(FrameField::Signatures).len() as u64)
        {
            return Ok(false);
        }
        let mut fields = frame.read_field(FrameField::Signatures);
        let count = fields.array().map_err(ReadFailure::Wire)?;
        if count == 0 {
            return Ok(false);
        }
        let mut checks = Vec::new();
        let mut authenticates = false;
        let eligible = checked.frame.frame_type != Some(FrameType::Commit as u64)
            || checked.commit.is_some_and(|commit| commit.valid);
        for _ in 0..count {
            let signature =
                SignatureRecord::read(&mut fields, segment.registry).map_err(ReadFailure::Wire)?;
            let status = self
                .signature_inputs
                .verify(&signature, &segment.header_identifier, &identifier)
                .map_err(ReadFailure::Signature)?;
            let covered = status == SignatureStatus::Valid && link_holds;
            let authenticated = covered
                && eligible
                && self
                    .signature_inputs
                    .trusted_keys()
                    .binary_search(signature.key_id)
                    .is_ok();
            authenticates |= authenticated;
            super::fields::push(
                &mut checks,
                SignatureCheck {
                    algorithm: signature.algorithm,
                    key_id: *signature.key_id,
                    status,
                    coverage_first: if covered {
                        segment.authentication.run_start
                    } else {
                        ordinary
                    },
                    coverage_end: ordinary + u64::from(covered),
                    authenticated,
                },
            )?;
        }
        super::fields::push(
            &mut self.graph.signatures,
            FrameSignatures {
                segment: segment.file_ordinal,
                position: segment.parity.item_count(),
                offset,
                checks,
            },
        )?;
        Ok(authenticates)
    }

    fn flush_rows(&mut self, segment: &mut Segment<'_>, committed: bool) -> Result<(), ReadError> {
        for PendingRows {
            offset,
            scope,
            rows,
        } in segment.pending_rows.drain(..)
        {
            let scope = if committed {
                scope.below_commit()
            } else {
                scope
            };
            self.fold_rows(&segment.dictionary, &rows, scope, offset)
                .map_err(|failure| ReadError { offset, failure })?;
        }
        Ok(())
    }

    fn fold_rows(
        &mut self,
        dictionary: &SegmentDictionary,
        rows: &[Row],
        scope: RowScope,
        offset: usize,
    ) -> Result<(), ReadFailure> {
        let findings = dictionary
            .fold_scoped_rows(&mut self.graph.rdf, rows, scope)
            .map_err(ReadFailure::Fold)?;
        if findings.forward_reference || findings.position_constraint || findings.opaque_term {
            self.graph
                .retain_content_failure(offset, ReadFailure::Rows(findings))?;
        }
        if self.graph.rdf.annotation_width_exceeded() {
            self.graph.annotation_width_offset.get_or_insert(offset);
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

fn segment_from_header(
    header: Header<'_>,
    identifier: [u8; DIGEST_LENGTH],
    extent: Range<usize>,
    file_ordinal: u64,
) -> Result<Segment<'_>, ReadFailure> {
    let registry = header.registry_version();
    Ok(Segment {
        registry,
        ordinal: header.lineage().map(|lineage| lineage.ordinal.get()),
        verifier: SegmentVerifier::new(identifier, registry),
        dictionary: SegmentDictionary::new(file_ordinal),
        file_ordinal,
        header_identifier: identifier,
        authentication: AuthenticationProgress::default(),
        durability: SegmentDurability {
            segment: file_ordinal,
            committed_frames: 0,
            uncommitted_tail: None,
        },
        end: FileEnd::NoCommit,
        units: FastMap::default(),
        index_storage: StorageIndex::default(),
        index_content: IndexContent::default(),
        pending_rows: Vec::new(),
        parity: ParityStream::new(extent, identifier, registry)?,
    })
}
