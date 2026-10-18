// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native fold and chain refusals, before human diagnostic rendering.

use core::fmt;
use purrdf_hash::error::ErrorValue;

#[derive(Debug)]
pub(super) enum FoldError {
    ConflictingReifier {
        reifier: usize,
    },
    UnknownFrameType {
        frame_type: String,
    },
    Language {
        term: i128,
        tag: String,
        refusal: &'static str,
    },
    Direction {
        term: i128,
        direction: String,
    },
    RefusedLanguageDirection {
        term: i128,
        direction: String,
    },
    ForwardReference {
        term: i128,
        datatype: Option<i128>,
        reifier: Option<i128>,
        triple: Option<[Option<i128>; 3]>,
    },
    ReifiesArray {
        snapshot: bool,
    },
    RecursiveReifier {
        reifier: usize,
        triple: crate::model::Triple3,
    },
    FrameMap,
    SelfHash {
        stored: Option<Vec<u8>>,
        computed: Vec<u8>,
    },
    Previous {
        actual: Option<Vec<u8>>,
        expected: Vec<u8>,
    },
    EmptyFile,
    FirstHeader,
    SegmentBoundary {
        item: u64,
    },
    ExpectedHead {
        actual: Vec<u8>,
        expected: Vec<u8>,
    },
    Torn {
        offset: u64,
    },
    HeaderHash {
        stored: Option<Vec<u8>>,
        computed: Vec<u8>,
    },
    HeaderVersion {
        magic: Option<purrdf_lex::cbor::Value>,
        version: Option<purrdf_lex::cbor::Value>,
    },
    ByteOffset {
        position: u64,
        read: usize,
    },
    ItemOrdinal {
        item: u64,
    },
}

impl FoldError {
    pub(super) fn diagnostic_code(&self) -> &'static str {
        match self {
            Self::ConflictingReifier { .. } => "ConflictingReifier",
            Self::UnknownFrameType { .. } => "UnknownFrameType",
            Self::ForwardReference { .. } => "ForwardReference",
            Self::Previous { .. } => "BrokenChain",
            Self::EmptyFile => "EmptyFile",
            Self::SegmentBoundary { .. } => "SegmentBoundary",
            Self::ExpectedHead { .. } => "TruncatedLog",
            Self::Torn { .. } => "TornAppendError",
            Self::ByteOffset { .. } | Self::ItemOrdinal { .. } => "ResourceLimit",
            Self::Language { .. }
            | Self::Direction { .. }
            | Self::RefusedLanguageDirection { .. }
            | Self::ReifiesArray { .. }
            | Self::RecursiveReifier { .. }
            | Self::FrameMap
            | Self::SelfHash { .. }
            | Self::FirstHeader
            | Self::HeaderHash { .. }
            | Self::HeaderVersion { .. } => "DamagedFrame",
        }
    }
}

impl fmt::Display for FoldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConflictingReifier { reifier } => write!(
                f,
                "reifier {reifier} binds several triples while a quoted-triple term still resolves through it"
            ),
            Self::UnknownFrameType { frame_type } => {
                write!(f, "unsupported frame type {frame_type:?}")
            }
            Self::Language { term, tag, refusal } => write!(
                f,
                "term {term} carries a language tag the RDF concrete-syntax grammar refuses ({refusal}): {tag:?}"
            ),
            Self::Direction { term, direction } => write!(
                f,
                "term {term} carries a base direction outside the RDF 1.2 value space (ltr, rtl): {direction:?}"
            ),
            Self::RefusedLanguageDirection { term, .. } => write!(
                f,
                "term {term} loses its base direction as well, because a base direction is only meaningful alongside a language tag and this term's tag was refused"
            ),
            Self::ForwardReference { term, .. } => write!(f, "term {term} has an out-of-range ref"),
            Self::ReifiesArray { snapshot } => write!(
                f,
                "{}reifies payload must be a row array",
                if *snapshot { "snapshot " } else { "" }
            ),
            Self::RecursiveReifier { reifier, .. } => write!(
                f,
                "reifier {reifier} creates a recursive quoted-triple binding"
            ),
            Self::FrameMap => f.write_str("frame is not a map"),
            Self::SelfHash { .. } => f.write_str("frame self-hash mismatch"),
            Self::Previous { .. } => f.write_str("prev does not match"),
            Self::EmptyFile => f.write_str("no CBOR items"),
            Self::FirstHeader => f.write_str("first item is not a header"),
            Self::SegmentBoundary { item } => write!(
                f,
                "segment boundary at item {item} but reader is in pre-segment mode; remainder of file NOT folded (folding it with file-global term-ids would silently misfold — §16)"
            ),
            Self::ExpectedHead { .. } => f.write_str("observed head does not match expected head"),
            Self::Torn { offset } => write!(f, "torn at offset {offset}"),
            Self::HeaderHash { .. } => f.write_str("header self-hash mismatch"),
            Self::HeaderVersion { magic, version } => {
                write!(f, "unsupported header magic/version {magic:?}/{version:?}")
            }
            Self::ByteOffset { .. } => f.write_str("GTS source byte offset exceeds u64::MAX"),
            Self::ItemOrdinal { .. } => f.write_str("GTS item ordinal exceeds u64::MAX"),
        }
    }
}

impl std::error::Error for FoldError {}

impl purrdf_hash::error::StructuredError for FoldError {
    purrdf_hash::error_visit! {
        ConflictingReifier { reifier } => "gts-conflicting-reifier" { "reifier" => Unsigned(*reifier as u128) },
        UnknownFrameType { frame_type } => "gts-unknown-frame-type" { "frame_type" => Text(frame_type) },
        Language { term, tag, refusal } => "gts-refused-language" { "term" => Signed(*term), "tag" => Text(tag), "refusal" => Code(refusal) },
        Direction { term, direction } => "gts-refused-direction" { "term" => Signed(*term), "direction" => Text(direction) },
        RefusedLanguageDirection { term, direction } => "gts-refused-language-direction" { "term" => Signed(*term), "direction" => Text(direction) },
        ForwardReference { term, datatype, reifier, triple } => "gts-forward-reference" {
            "term" => Signed(*term),
            "datatype" => Value(datatype.map_or(ErrorValue::Absent, ErrorValue::Signed)),
            "reifier" => Value(reifier.map_or(ErrorValue::Absent, ErrorValue::Signed)),
            "triple" => List(triple.as_ref().map_or(&[][..], |ids| ids.as_slice()).iter().map(|id| id.map_or(ErrorValue::Absent, ErrorValue::Signed)))
        },
        ReifiesArray { snapshot } => "gts-reifies-not-array" { "snapshot" => Boolean(*snapshot) },
        RecursiveReifier { reifier, triple } => "gts-recursive-reifier" { "reifier" => Unsigned(*reifier as u128), "subject" => Unsigned(triple.0 as u128), "predicate" => Unsigned(triple.1 as u128), "object" => Unsigned(triple.2 as u128) },
        FrameMap => "gts-frame-not-map" {},
        SelfHash { stored, computed } => "gts-frame-self-hash" { "stored" => Value(stored.as_deref().map_or(ErrorValue::Absent, ErrorValue::Bytes)), "computed" => Bytes(computed) },
        Previous { actual, expected } => "gts-frame-previous" { "actual" => Value(actual.as_deref().map_or(ErrorValue::Absent, ErrorValue::Bytes)), "expected" => Bytes(expected) },
        EmptyFile => "gts-empty-file" {},
        FirstHeader => "gts-first-item-not-header" {},
        SegmentBoundary { item } => "gts-segment-boundary" { "item" => Unsigned(u128::from(*item)) },
        ExpectedHead { actual, expected } => "gts-expected-head" { "actual" => Bytes(actual), "expected" => Bytes(expected) },
        Torn { offset } => "gts-torn-append" { "offset" => Unsigned(u128::from(*offset)) },
        HeaderHash { stored, computed } => "gts-header-self-hash" { "stored" => Value(stored.as_deref().map_or(ErrorValue::Absent, ErrorValue::Bytes)), "computed" => Bytes(computed) },
        HeaderVersion { magic, version } => "gts-header-version" { "magic" => Value(magic.as_ref().map_or(ErrorValue::Absent, |value| ErrorValue::Record(value))), "version" => Value(version.as_ref().map_or(ErrorValue::Absent, |value| ErrorValue::Record(value))) },
        ByteOffset { position, read } => "gts-byte-offset-overflow" { "position" => Unsigned(u128::from(*position)), "read" => Unsigned(*read as u128) },
        ItemOrdinal { item } => "gts-item-ordinal-overflow" { "item" => Unsigned(u128::from(*item)) },
    }
}
