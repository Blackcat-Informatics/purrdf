// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! GTS generation 2 framing, segment verification and RDF graph transport.

mod fields;
mod frame;
mod header;
mod index;
mod item;
mod link;
mod parity;
mod payload;
mod range;
mod rdf;
mod read;
mod rows;
mod segment;
mod signature;
mod terms;
mod writer;

pub use fields::WireError;
pub use frame::{FrameCheck, FrameEnvelope, FrameField, FrameType, UnknownFrameType};
pub use header::{
    CanonicalForm, Header, Lineage, Profile, ResyncCandidate, ResyncRefusal, ResyncScan,
    write_header,
};
pub use index::{IndexPrefixCheck, IndexStorageFinding, IndexStorageItem, IndexStorageProblem};
pub use item::{Generation, Item, ItemKind, names_second_generation};
pub use link::{LinkCheck, LinkState};
pub use parity::{
    ParityCheck, ParityFinding, ParityFindingKind, ParityMismatches, ParityPatch,
    ParityRepairFailure, ParitySolution, ParitySolveError, ParitySpanLength, ParityTable,
};
pub use payload::{
    PayloadBudget, PayloadCapacity, PayloadCheck, PayloadDamage, PayloadRefusal,
    PayloadResourceError, PayloadState, StoredState,
};
pub use range::{CommitCheck, CommitClaim, Range, RangeFull};
pub use rdf::{
    DefinitionFindings, FoldError, Line, LiteralDirection, LiteralValue, RdfFold, RowDisposition,
    RowFindings, SegmentDictionary, Term, TermId, ValueName, ValueStore,
};
pub use read::{
    BlobEntry, DetectionCapacity, ExternalBlob, FileEnd, FrameSignatures, FrameWireResult,
    FramingDamage, GraphIndexCheck, ItemRefusal, LineageFinding, ParityRecovery, PayloadOutcome,
    PayloadUnchecked, RdfGraph, ReadCapacity, ReadError, ReadFailure, ReadGraph, ReadLimits,
    RebuildResult, RebuildState, RecoveryRefusal, ResyncOpener, ResyncPast, ResyncReach,
    Resynchronization, SegmentDurability, SkippedSegment, UncommittedTail, VerifiedBlob, read,
    read_with_limits, read_with_limits_and_signature_inputs, read_with_signature_inputs,
};
pub use rows::{DecodedRows, Row, RowHeader, RowOrder};
pub use segment::{SegmentCheckError, SegmentFrame, SegmentVerifier};
pub use signature::{
    Authenticity, HeldKey, SegmentAuthentication, SignatureCheck, SignatureInputs,
    SignatureInvalid, SignatureStatus, SignatureUnverified,
};
pub use terms::{
    ContentError, DecodedTerms, FormedBudget, LiteralForm, TermDefinition, TermOrder,
    TermReservation, TermText, TermsHeader, TypedValue, ValueReference,
};
pub use writer::{
    CoSignError, SegmentWriteError, SegmentWriter, SignatureWriteError, append_cosigned_commit,
    rdf_segment_bytes, write_rdf_segment,
};

/// The reason a wire value violates the GTS v2 grammar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum MalformedReason {
    /// A head is long or indefinite.
    NotDeterministic,
    /// An item has a different CBOR type or its text is invalid UTF-8.
    WrongType,
    /// A record or byte string has an inadmissible length.
    WrongLength,
    /// An integer lies outside the field's range.
    OutOfRange,
    /// An enumeration value is unregistered.
    UnregisteredValue,
    /// The individually typed fields are incompatible.
    ForbiddenCombination,
    /// The compressed payload does not begin with its required codec header.
    CodecConstraint,
    /// Adjacent elements are out of order or equal.
    UnorderedOrRepeated,
    /// A declared count or extent differs from the values it describes.
    InconsistentCounts,
}

/// The typed generation field of a GTS v2 opener.
pub const GENERATION: u64 = 2;
const HEADER_KIND: u64 = 1;
const FRAME_KIND: u64 = 2;
const FRAME_FIELDS: usize = 11;
const RESYNC_KIND: u64 = 5;
const REGISTRY_VERSION: u64 = 1;
const BLAKE3_256: u64 = 1;
const DIGEST_LENGTH: usize = 32;
const RESYNC_MAX_BYTES: usize = 4096;
