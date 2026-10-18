// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original compaction refusals; presentation never replaces native values.

use core::fmt;
use purrdf_hash::error::ErrorValue;
use purrdf_lex::cbor::Value;

use crate::{
    codec::CodecError, cose::Sign1Error, dict::DictError, mmr::Hex32Error, model::Diagnostic,
    writer::WriterError,
};

/// A malformed unpadded base64url spelling (RFC 4648 §5).
#[derive(Debug)]
#[non_exhaustive]
pub enum Base64UrlError {
    /// The byte length leaves one unpaired trailing character.
    Length {
        /// Original input byte count.
        actual: usize,
    },
    /// Padding is forbidden in this spelling.
    Padding,
    /// A byte lies outside the URL-safe alphabet.
    Alphabet {
        /// Original rejected byte.
        byte: u8,
    },
}

/// Why a rewrite timestamp is not an explicit-UTC XSD dateTime.
#[derive(Debug)]
#[non_exhaustive]
pub enum TimestampError {
    /// Original datatype parser failure.
    Parse(purrdf_xsd::XsdError),
    /// The parsed timestamp has no timezone or a nonzero timezone.
    Zone {
        /// Parsed signed timezone offset, absent when unspecified.
        minutes: Option<i64>,
    },
}

impl fmt::Display for TimestampError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("rewrite timestamp must be a valid XSD dateTime with an explicit UTC timezone")
    }
}
purrdf_lex::error_source!(TimestampError { Parse });
impl purrdf_hash::error::StructuredError for TimestampError {
    purrdf_hash::error_visit! {
        Parse(cause) => "gts-compact-timestamp-parse" { "cause" => Cause(cause) },
        Zone { minutes } => "gts-compact-timestamp-zone" {
            "minutes" => Value(minutes.map_or(ErrorValue::Absent, |minutes| ErrorValue::Signed(i128::from(minutes))))
        },
    }
}

impl fmt::Display for Base64UrlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Length { actual } => write!(
                f,
                "base64url input length {actual} leaves a single trailing character (invalid)"
            ),
            Self::Padding => f.write_str("unexpected padding character '=' in unpadded base64url"),
            Self::Alphabet { byte } => write!(
                f,
                "byte {byte:#04x} is outside the base64url (RFC 4648 §5) alphabet"
            ),
        }
    }
}
impl std::error::Error for Base64UrlError {}
impl purrdf_hash::error::StructuredError for Base64UrlError {
    purrdf_hash::error_visit! {
        Length { actual } => "gts-base64url-length" { "actual" => ByteCount(*actual as u128) },
        Padding => "gts-base64url-padding" { "byte" => Unsigned(u128::from(b'=')) },
        Alphabet { byte } => "gts-base64url-alphabet" { "byte" => Unsigned(u128::from(*byte)) },
    }
}

/// The input or caller configuration is not safely compactable.
#[derive(Debug)]
#[non_exhaustive]
pub enum CompactRefusedError {
    /// Dictionary names must be nonempty.
    EmptyDictionaryName,
    /// A pinned name occurs more than once.
    DuplicateDictionary {
        /// Repeated name.
        name: String,
    },
    /// A pinned dictionary cannot be parsed.
    PinnedDictionary {
        /// Caller-supplied name.
        name: String,
        /// Original dictionary decoder error.
        cause: DictError,
    },
    /// The selection names an unpinned dictionary.
    UnknownDictionary {
        /// Selected name.
        name: String,
    },
    /// The selected dictionary has no transform that can consume it.
    DictionaryTransform {
        /// Selected name.
        name: String,
        /// Original ordered transform chain.
        transform: Vec<String>,
    },
    /// The pinned dictionaries have no transform that can consume them.
    UnusedDictionaries {
        /// Number of pinned dictionaries.
        count: usize,
        /// Original ordered transform chain.
        transform: Vec<String>,
    },
    /// A zstd level has no zstd-family transform.
    LevelTransform {
        /// Requested level.
        level: i32,
        /// Original ordered transform chain.
        transform: Vec<String>,
    },
    /// The segment reader rejected the file.
    InputDiagnostic(Diagnostic),
    /// The file has an incomplete trailing item.
    Torn {
        /// Original byte offset.
        offset: u64,
    },
    /// A segment did not verify cleanly.
    SegmentDiagnostic {
        /// Original zero-based segment index.
        segment: usize,
        /// Original reader diagnostic and native cause.
        cause: Diagnostic,
    },
    /// Segments declare incompatible profiles.
    MixedProfiles {
        /// Sorted unique profile names.
        profiles: Vec<String>,
    },
    /// Evidence artifacts require explicit sealing of their original bytes.
    EvidenceProfile,
    /// Rewriting would invalidate a frame-addressed suppression.
    FrameSuppression {
        /// Original suppression target.
        target: Value,
    },
    /// A content blob cannot be decoded.
    BlobDecode {
        /// Original blob digest label.
        digest: String,
        /// Original codec error.
        cause: CodecError,
    },
    /// A provenance predicate has a nonliteral object.
    Literal {
        /// Original subject ID.
        node: usize,
        /// Required predicate IRI.
        predicate: String,
        /// Original object ID.
        object: usize,
    },
    /// Multiple current compaction records match the complete source-head list.
    AmbiguousProvenance {
        /// Sorted source heads used for matching.
        source_heads: Vec<Vec<u8>>,
    },
    /// No current compaction record matches the complete source-head list.
    MissingProvenance {
        /// Sorted source heads used for matching.
        source_heads: Vec<Vec<u8>>,
    },
    /// A compaction node has repeated detached roots.
    RepeatedRoots {
        /// Original subject ID.
        node: usize,
        /// Number of roots found.
        count: usize,
    },
    /// A detached root is not a 32-byte hex digest.
    DetachedRoot {
        /// Original subject ID.
        node: usize,
        /// Original identifier decoder error.
        cause: Hex32Error,
    },
    /// A compaction node declares no source heads.
    MissingSourceHeads {
        /// Original subject ID.
        node: usize,
    },
    /// A source head lacks its required digest prefix.
    SourceHeadPrefix {
        /// Original subject ID.
        node: usize,
        /// Original source-head spelling.
        actual: String,
    },
    /// A source head has a malformed digest.
    SourceHead {
        /// Original subject ID.
        node: usize,
        /// Original identifier decoder error.
        cause: Hex32Error,
    },
    /// A detached signature's literal field is malformed.
    DetachedLiteral {
        /// Original detached signature subject ID.
        node: usize,
        /// Original provenance field refusal.
        cause: Box<Self>,
    },
    /// A detached signature does not have exactly one required literal.
    DetachedLiteralCount {
        /// Original subject ID.
        node: usize,
        /// Required predicate IRI.
        predicate: String,
        /// Actual number of literals.
        count: usize,
    },
    /// A detached signature's source-frame identifier is malformed.
    DetachedSourceFrame {
        /// Original subject ID.
        node: usize,
        /// Original identifier decoder error.
        cause: Hex32Error,
    },
    /// A detached signature's COSE encoding is malformed.
    DetachedEncoding {
        /// Original subject ID.
        node: usize,
        /// Original base64url decoder error.
        cause: Base64UrlError,
    },
    /// A frame signature has no COSE byte string.
    FrameMissingCose {
        /// Original frame identifier.
        frame_id: Vec<u8>,
    },
    /// A frame's COSE envelope is malformed.
    FrameCose(Sign1Error),
    /// A detached source-frame ID has the wrong byte length.
    DetachedFrameLength {
        /// Original byte length.
        actual: usize,
    },
    /// A detached COSE envelope is malformed.
    DetachedCose(Sign1Error),
    /// Derived dictionaries have no content corpus.
    EmptyCorpus {
        /// Number of pinned dictionary definitions.
        dictionaries: usize,
    },
    /// A derived dictionary cannot be built.
    BuildDictionary {
        /// Original dictionary name.
        name: String,
        /// Original dictionary construction error.
        cause: DictError,
    },
    /// The rewrite timestamp is not a valid explicit-UTC XSD dateTime.
    Timestamp {
        /// Original timestamp spelling.
        actual: String,
        /// Original parser failure or parsed timezone refusal.
        cause: TimestampError,
    },
    /// Pack writer construction failed.
    ConfigureWriter(WriterError),
    /// Pack frame authoring failed.
    AuthorFrame {
        /// Frame type being authored.
        frame_type: String,
        /// Original writer refusal.
        cause: WriterError,
    },
    /// A content blob could not be authored.
    AuthorBlob(WriterError),
    /// The packaging signer refused the final index.
    SignPackaging(WriterError),
}

purrdf_lex::constructors! {
    impl CompactRefusedError {
        /// Retain the original blob digest and native decoder failure.
        pub(super) fn blob_decode(digest, cause: CodecError) -> Self::BlobDecode { .. };
    }
}

impl fmt::Display for CompactRefusedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyDictionaryName => f.write_str("an in-band dictionary name must be non-empty"),
            Self::DuplicateDictionary { name } => write!(f, "duplicate in-band dictionary name {name:?}"),
            Self::PinnedDictionary { name, cause } => write!(f, "the pinned dictionary {name:?} is not a parseable finalized zstd dictionary: {cause}"),
            Self::UnknownDictionary { name } => write!(f, "the plan selects dictionary {name:?}, which it does not pin"),
            Self::DictionaryTransform { name, transform } => write!(f, "the plan selects dictionary {name:?} but its transform chain {transform:?} carries no zstd-family codec to prime"),
            Self::UnusedDictionaries { count, transform } => write!(f, "the plan pins {count} dictionar(y/ies) but its transform chain {transform:?} carries no zstd-family codec, so nothing could ever use them"),
            Self::LevelTransform { transform, .. } => write!(f, "the plan declares a zstd level but its transform chain {transform:?} carries no zstd-family codec to encode at it"),
            Self::InputDiagnostic(cause) => write!(f, "input is not a clean GTS file: {}: {}", cause.code, cause.detail),
            Self::Torn { offset } => write!(f, "input has a torn append at byte {offset}"),
            Self::SegmentDiagnostic { segment, cause } => write!(f, "segment {segment} does not verify cleanly: {}: {}", cause.code, cause.detail),
            Self::MixedProfiles { profiles } => {
                f.write_str("mixed segment profiles [")?;
                for (index, profile) in profiles.iter().enumerate() {
                    if index != 0 { f.write_str(", ")?; }
                    write!(f, "'{profile}'")?;
                }
                f.write_str("] are not compactable (v1)")
            },
            Self::EvidenceProfile => f.write_str("an 'evidence' artifact's signed chain IS the artifact; refusing to re-order it without --seal-original (§10.1)"),
            Self::FrameSuppression { .. } => f.write_str("input carries a frame-addressed suppression; the rewrite assigns new frame ids, so the target would silently dangle (§10.1)"),
            Self::BlobDecode { digest, cause } => write!(f, "cannot decode blob {digest}: {cause:?}"),
            Self::Literal { node, predicate, .. } => write!(f, "provenance node {node} needs a literal {predicate}"),
            Self::AmbiguousProvenance { .. } => f.write_str("ambiguous compaction provenance for source heads"),
            Self::MissingProvenance { .. } => f.write_str("missing compaction provenance for source heads"),
            Self::RepeatedRoots { node, .. } => write!(f, "compaction node {node} has repeated detached signature roots"),
            Self::DetachedRoot { node, cause } => write!(f, "compaction node {node} detached root: {cause}"),
            Self::MissingSourceHeads { node } => write!(f, "compaction node {node} needs source heads"),
            Self::SourceHeadPrefix { node, .. } => write!(f, "compaction node {node} invalid source head"),
            Self::SourceHead { node, cause } => write!(f, "compaction node {node} source head: {cause}"),
            Self::DetachedLiteral { node, cause } => write!(f, "detached signature node {node}: {cause}"),
            Self::DetachedLiteralCount { node, predicate, .. } => write!(f, "detached signature node {node} needs exactly one literal {predicate}"),
            Self::DetachedSourceFrame { node, cause } => write!(f, "detached signature node {node} source frame: {cause}"),
            Self::DetachedEncoding { node, cause } => write!(f, "detached signature node {node} COSE encoding: {cause}"),
            Self::FrameMissingCose { .. } => f.write_str("frame signature must carry a COSE byte string"),
            Self::FrameCose(cause) => write!(f, "frame signature COSE: {cause}"),
            Self::DetachedFrameLength { .. } => f.write_str("detached signature source frame must be 32 bytes"),
            Self::DetachedCose(cause) => write!(f, "detached signature COSE: {cause}"),
            Self::EmptyCorpus { .. } => f.write_str("the plan pins in-band dictionaries but the input has no content blobs to build them from (§8.5 dct)"),
            Self::BuildDictionary { name, cause } => write!(f, "cannot build the pack dictionary {name:?}: {cause}"),
            Self::Timestamp { cause, .. } => cause.fmt(f),
            Self::ConfigureWriter(cause) => write!(f, "cannot configure the pack writer: {cause}"),
            Self::AuthorFrame { frame_type, cause } => write!(f, "cannot author the {frame_type} frame: {cause}"),
            Self::AuthorBlob(cause) => write!(f, "cannot author the content blob frame: {cause}"),
            Self::SignPackaging(cause) => write!(f, "cannot sign the packaging index: {cause}"),
        }
    }
}

purrdf_lex::error_source!(CompactRefusedError => {
    Self::PinnedDictionary { cause, .. } => cause,
    Self::InputDiagnostic(cause) => cause,
    Self::SegmentDiagnostic { cause, .. } => cause,
    Self::BlobDecode { cause, .. } => cause,
    Self::DetachedRoot { cause, .. } => cause,
    Self::SourceHead { cause, .. } => cause,
    Self::DetachedLiteral { cause, .. } => cause.as_ref(),
    Self::DetachedSourceFrame { cause, .. } => cause,
    Self::DetachedEncoding { cause, .. } => cause,
    Self::FrameCose(cause) => cause,
    Self::DetachedCose(cause) => cause,
    Self::BuildDictionary { cause, .. } => cause,
    Self::Timestamp { cause, .. } => cause,
    Self::ConfigureWriter(cause) => cause,
    Self::AuthorFrame { cause, .. } => cause,
    Self::AuthorBlob(cause) => cause,
    Self::SignPackaging(cause) => cause,
});

impl purrdf_hash::error::StructuredError for CompactRefusedError {
    purrdf_hash::error_visit! {
        EmptyDictionaryName => "gts-compact-empty-dictionary-name" {},
        DuplicateDictionary { name } => "gts-compact-duplicate-dictionary" { "name" => Text(name) },
        PinnedDictionary { name, cause } => "gts-compact-pinned-dictionary" { "name" => Text(name), "cause" => Cause(cause) },
        UnknownDictionary { name } => "gts-compact-unknown-dictionary" { "name" => Text(name) },
        DictionaryTransform { name, transform } => "gts-compact-dictionary-transform" { "name" => Text(name), "transform" => List(transform.iter().map(|name| ErrorValue::Text(name))) },
        UnusedDictionaries { count, transform } => "gts-compact-unused-dictionaries" { "count" => Unsigned(*count as u128), "transform" => List(transform.iter().map(|name| ErrorValue::Text(name))) },
        LevelTransform { level, transform } => "gts-compact-level-transform" { "level" => Signed(i128::from(*level)), "transform" => List(transform.iter().map(|name| ErrorValue::Text(name))) },
        InputDiagnostic(cause) => "gts-compact-input" { "cause" => Cause(cause) },
        Torn { offset } => "gts-compact-torn" { "offset" => ByteCount(u128::from(*offset)) },
        SegmentDiagnostic { segment, cause } => "gts-compact-segment" { "segment" => Unsigned(*segment as u128), "cause" => Cause(cause) },
        MixedProfiles { profiles } => "gts-compact-mixed-profiles" { "profiles" => List(profiles.iter().map(|profile| ErrorValue::Text(profile))) },
        EvidenceProfile => "gts-compact-evidence-profile" {},
        FrameSuppression { target } => "gts-compact-frame-suppression" { "target" => Record(target) },
        BlobDecode { digest, cause } => "gts-compact-blob-decode" { "digest" => Text(digest), "cause" => Cause(cause) },
        Literal { node, predicate, object } => "gts-compact-literal" { "node" => Unsigned(*node as u128), "predicate" => Iri(predicate), "object" => Unsigned(*object as u128) },
        AmbiguousProvenance { source_heads } => "gts-compact-ambiguous-provenance" { "source_heads" => List(source_heads.iter().map(|head| ErrorValue::Bytes(head))) },
        MissingProvenance { source_heads } => "gts-compact-missing-provenance" { "source_heads" => List(source_heads.iter().map(|head| ErrorValue::Bytes(head))) },
        RepeatedRoots { node, count } => "gts-compact-repeated-roots" { "node" => Unsigned(*node as u128), "count" => Unsigned(*count as u128) },
        DetachedRoot { node, cause } => "gts-compact-detached-root" { "node" => Unsigned(*node as u128), "cause" => Cause(cause) },
        MissingSourceHeads { node } => "gts-compact-missing-source-heads" { "node" => Unsigned(*node as u128) },
        SourceHeadPrefix { node, actual } => "gts-compact-source-head-prefix" { "node" => Unsigned(*node as u128), "actual" => Text(actual) },
        SourceHead { node, cause } => "gts-compact-source-head" { "node" => Unsigned(*node as u128), "cause" => Cause(cause) },
        DetachedLiteral { node, cause } => "gts-compact-detached-literal" { "node" => Unsigned(*node as u128), "cause" => Cause(cause.as_ref()) },
        DetachedLiteralCount { node, predicate, count } => "gts-compact-detached-literal-count" { "node" => Unsigned(*node as u128), "predicate" => Iri(predicate), "count" => Unsigned(*count as u128) },
        DetachedSourceFrame { node, cause } => "gts-compact-detached-source-frame" { "node" => Unsigned(*node as u128), "cause" => Cause(cause) },
        DetachedEncoding { node, cause } => "gts-compact-detached-encoding" { "node" => Unsigned(*node as u128), "cause" => Cause(cause) },
        FrameMissingCose { frame_id } => "gts-compact-frame-missing-cose" { "frame_id" => Bytes(frame_id) },
        FrameCose(cause) => "gts-compact-frame-cose" { "cause" => Cause(cause) },
        DetachedFrameLength { actual } => "gts-compact-detached-frame-length" { "actual" => ByteCount(*actual as u128), "expected" => ByteCount(32) },
        DetachedCose(cause) => "gts-compact-detached-cose" { "cause" => Cause(cause) },
        EmptyCorpus { dictionaries } => "gts-compact-empty-corpus" { "dictionaries" => Unsigned(*dictionaries as u128) },
        BuildDictionary { name, cause } => "gts-compact-build-dictionary" { "name" => Text(name), "cause" => Cause(cause) },
        Timestamp { actual, cause } => "gts-compact-timestamp" { "actual" => Text(actual), "cause" => Cause(cause) },
        ConfigureWriter(cause) => "gts-compact-configure-writer" { "cause" => Cause(cause) },
        AuthorFrame { frame_type, cause } => "gts-compact-author-frame" { "frame_type" => Code(frame_type), "cause" => Cause(cause) },
        AuthorBlob(cause) => "gts-compact-author-blob" { "cause" => Cause(cause) },
        SignPackaging(cause) => "gts-compact-sign-packaging" { "cause" => Cause(cause) },
    }
}
