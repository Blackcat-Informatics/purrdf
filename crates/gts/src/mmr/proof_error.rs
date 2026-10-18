// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native proof parsing, verification and file admission failures.

use super::Hex32Error;
use core::fmt;
use purrdf_hash::{error::ErrorValue, hex::Lower};
use purrdf_lex::{
    cbor::Value,
    json::{self, Value as Json},
};

/// Why a detached MMR proof cannot be parsed, verified or derived.
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum ProofError {
    /// A fixed-size digest has the wrong byte length.
    DigestLength {
        /// Original digest role.
        field: &'static str,
        /// Original byte count.
        actual: usize,
    },
    /// A peak height cannot be represented as a platform-sized width.
    PeakHeight {
        /// Original height.
        height: usize,
    },
    /// A leaf index lies outside the covered population.
    LeafRange {
        /// Original leaf index.
        leaf_index: usize,
        /// Covered leaf count.
        count: usize,
    },
    /// Accumulating a peak range would overflow the platform index.
    PeakRangeOverflow {
        /// Original start.
        start: usize,
        /// Peak width being added.
        width: usize,
    },
    /// The peak ranges do not cover the declared leaf.
    Uncovered {
        /// Original leaf index.
        leaf_index: usize,
        /// Declared covered count.
        count: usize,
    },
    /// A selected peak index is outside the actual peak list.
    PeakIndex {
        /// Original peak index.
        actual: usize,
        /// Actual peak list length.
        available: usize,
    },
    /// Peak heights disagree with the declared covered count.
    PeakHeights {
        /// Original ordered heights.
        actual: Vec<usize>,
        /// Declared covered count.
        count: usize,
    },
    /// The leaf belongs to a different peak.
    LeafPeak {
        /// Original leaf index.
        leaf_index: usize,
        /// Computed peak index.
        expected: usize,
        /// Declared peak index.
        actual: usize,
    },
    /// A path step does not advance one height.
    ParentHeight {
        /// Original path parent height.
        parent: usize,
        /// Height before this step.
        previous: usize,
    },
    /// The reconstructed path height does not reach the peak.
    PathHeight {
        /// Reconstructed height.
        actual: usize,
        /// Required peak height.
        expected: usize,
    },
    /// The path does not reconstruct its selected peak hash.
    PeakMismatch {
        /// Reconstructed peak.
        actual: Vec<u8>,
        /// Declared peak.
        expected: Vec<u8>,
    },
    /// The peaks do not reconstruct the declared root.
    RootMismatch {
        /// Reconstructed root.
        actual: Vec<u8>,
        /// Declared root.
        expected: Vec<u8>,
    },
    /// The shared JSON reader rejected the document.
    Json(json::Error),
    /// An object was required.
    JsonObject {
        /// Original parsing context.
        context: String,
        /// Original rejected value.
        actual: Json,
    },
    /// An array was required.
    JsonArray {
        /// Original parsing context.
        context: String,
        /// Original rejected value.
        actual: Json,
    },
    /// A required member is absent.
    JsonMissing {
        /// Required member name.
        field: String,
    },
    /// A member is not a string.
    JsonString {
        /// Original member name.
        field: String,
        /// Original rejected value.
        actual: Json,
    },
    /// A member is not representable as an unsigned 64-bit integer.
    JsonUnsigned {
        /// Original member name.
        field: String,
        /// Original rejected value, including its number lexeme.
        actual: Json,
    },
    /// An unsigned member does not fit the platform index.
    JsonPlatform {
        /// Original member name.
        field: String,
        /// Original full-width integer.
        actual: u64,
    },
    /// A declaration names an unsupported value.
    Unsupported {
        /// Original declaration role.
        declaration: &'static str,
        /// Original declaration value.
        actual: String,
        /// Supported values in declaration order.
        allowed: &'static [&'static str],
    },
    /// An identifier could not be decoded.
    Hex(Hex32Error),
    /// A file ends in an incomplete CBOR item.
    Torn {
        /// Original byte offset.
        offset: usize,
    },
    /// A file carries no CBOR item.
    Empty,
    /// An item cannot be unwrapped as a header map.
    Header {
        /// Original item index.
        item: usize,
        /// Original header decoder error.
        cause: crate::wire::HeaderError,
    },
    /// A map does not declare a GTS v1 header.
    NotHeader {
        /// Original item index.
        item: usize,
        /// Original rejected header map.
        actual: Value,
    },
    /// A stored header or frame identity does not match its content.
    FileIdMismatch {
        /// Original item index.
        item: usize,
        /// Whether this is a header rather than a frame.
        header: bool,
        /// Stored identity.
        actual: Vec<u8>,
        /// Computed identity.
        expected: Vec<u8>,
    },
    /// A header or frame has no byte-string identity.
    FileMissingId {
        /// Original item index.
        item: usize,
        /// Whether this is a header rather than a frame.
        header: bool,
        /// Original identity field, absent if missing.
        actual: Option<Value>,
    },
    /// A frame is not a map.
    NotFrame {
        /// Original item index.
        item: usize,
        /// Original rejected item.
        actual: Value,
    },
    /// A frame does not link to its predecessor.
    PrevMismatch {
        /// Original item index.
        item: usize,
        /// Original predecessor field, absent if missing.
        actual: Option<Value>,
        /// Required predecessor identity.
        expected: Vec<u8>,
    },
    /// An index claims more frames than precede it.
    IndexCount {
        /// Original index item position.
        item: usize,
        /// Declared covered count.
        count: usize,
        /// Actual preceding population.
        limit: usize,
    },
    /// An index's head does not match its covered prefix.
    IndexHead {
        /// Original index item position.
        item: usize,
        /// Whether the covered prefix is empty.
        empty: bool,
        /// Declared head.
        actual: Vec<u8>,
        /// Required head.
        expected: Vec<u8>,
    },
    /// An index's MMR root does not match its covered prefix.
    IndexRoot {
        /// Original index item position.
        item: usize,
        /// Declared MMR root.
        actual: Vec<u8>,
        /// Computed root.
        expected: Vec<u8>,
    },
    /// No valid index covers the requested frame.
    NotCovered {
        /// Original requested frame identity.
        target: Vec<u8>,
    },
}

impl fmt::Display for ProofError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DigestLength { field, .. } => write!(f, "{field} must be 32 bytes"),
            Self::PeakHeight { height } => write!(f, "peak height {height} is too large"),
            Self::LeafRange { leaf_index, count } => write!(
                f,
                "leaf_index {leaf_index} is outside covered count {count}"
            ),
            Self::PeakRangeOverflow { .. } => f.write_str("peak ranges overflow usize"),
            Self::Uncovered { leaf_index, count } => write!(
                f,
                "peak ranges do not cover leaf_index {leaf_index} for count {count}"
            ),
            Self::PeakIndex { actual, .. } => write!(f, "peak_index {actual} is out of range"),
            Self::PeakHeights { actual, count } => {
                write!(f, "peak heights {actual:?} do not match count {count}")
            }
            Self::LeafPeak {
                leaf_index,
                expected,
                actual,
            } => write!(
                f,
                "leaf_index {leaf_index} belongs to peak {expected}, not {actual}"
            ),
            Self::ParentHeight { parent, previous } => write!(
                f,
                "path parent height {parent} does not follow height {previous}"
            ),
            Self::PathHeight { actual, expected } => write!(
                f,
                "path height {actual} does not reach peak height {expected}"
            ),
            Self::PeakMismatch { .. } => {
                f.write_str("proof path does not reconstruct the selected peak")
            }
            Self::RootMismatch { .. } => {
                f.write_str("proof peaks do not reconstruct the declared root")
            }
            Self::Json(cause) => write!(f, "invalid proof JSON: {cause}"),
            Self::JsonObject { context, .. } => write!(f, "{context} must be a JSON object"),
            Self::JsonArray { context, .. } => write!(f, "{context} must be a JSON array"),
            Self::JsonMissing { field } => write!(f, "proof JSON missing {field:?}"),
            Self::JsonString { field, .. } => write!(f, "{field:?} must be a string"),
            Self::JsonUnsigned { field, .. } => write!(f, "{field:?} must be an unsigned integer"),
            Self::JsonPlatform { field, .. } => {
                write!(f, "{field:?} is too large for this platform")
            }
            Self::Unsupported {
                declaration,
                actual,
                ..
            } => write!(f, "unsupported {declaration} {actual:?}"),
            Self::Hex(cause) => cause.fmt(f),
            Self::Torn { offset } => {
                write!(f, "input has a torn trailing CBOR item at byte {offset}")
            }
            Self::Empty => f.write_str("input is empty"),
            Self::Header { item, cause } => {
                write!(f, "item {item} is not a segment header: {cause}")
            }
            Self::NotHeader { item, .. } => write!(f, "item {item} is not a GTS v1 header"),
            Self::FileIdMismatch { item, header, .. } => write!(
                f,
                "{} {item} id mismatch",
                if *header { "header" } else { "item" }
            ),
            Self::FileMissingId { item, header, .. } => write!(
                f,
                "{} {item} is missing id",
                if *header { "header" } else { "item" }
            ),
            Self::NotFrame { item, .. } => write!(f, "item {item} frame is not a map"),
            Self::PrevMismatch { item, .. } => write!(f, "item {item} prev mismatch"),
            Self::IndexCount { item, count, limit } => write!(
                f,
                "item {item} index covers {count} frame(s), but only {limit} precede the index"
            ),
            Self::IndexHead { item, empty, .. } => write!(
                f,
                "item {item} {}index head mismatch",
                if *empty { "empty " } else { "" }
            ),
            Self::IndexRoot { item, .. } => write!(f, "item {item} index mmr mismatch"),
            Self::NotCovered { target } => {
                write!(f, "no valid index mmr covers frame {}", Lower(target))
            }
        }
    }
}

purrdf_lex::error_source!(ProofError => {
    Self::Json(cause) => cause,
    Self::Hex(cause) => cause,
    Self::Header { cause, .. } => cause,
});
purrdf_lex::variant_from! { ProofError { Hex(Hex32Error) } }

impl purrdf_hash::error::StructuredError for ProofError {
    purrdf_hash::error_visit! {
        DigestLength { field, actual } => "gts-proof-digest-length" { "field" => Code(field), "actual" => ByteCount(*actual as u128), "expected" => ByteCount(32) },
        PeakHeight { height } => "gts-proof-peak-height" { "height" => Unsigned(*height as u128) },
        LeafRange { leaf_index, count } => "gts-proof-leaf-range" { "leaf_index" => Unsigned(*leaf_index as u128), "count" => Unsigned(*count as u128) },
        PeakRangeOverflow { start, width } => "gts-proof-peak-range-overflow" { "start" => Unsigned(*start as u128), "width" => Unsigned(*width as u128) },
        Uncovered { leaf_index, count } => "gts-proof-uncovered" { "leaf_index" => Unsigned(*leaf_index as u128), "count" => Unsigned(*count as u128) },
        PeakIndex { actual, available } => "gts-proof-peak-index" { "actual" => Unsigned(*actual as u128), "available" => Unsigned(*available as u128) },
        PeakHeights { actual, count } => "gts-proof-peak-heights" { "actual" => List(actual.iter().map(|height| ErrorValue::Unsigned(*height as u128))), "count" => Unsigned(*count as u128) },
        LeafPeak { leaf_index, expected, actual } => "gts-proof-leaf-peak" { "leaf_index" => Unsigned(*leaf_index as u128), "expected" => Unsigned(*expected as u128), "actual" => Unsigned(*actual as u128) },
        ParentHeight { parent, previous } => "gts-proof-parent-height" { "parent" => Unsigned(*parent as u128), "previous" => Unsigned(*previous as u128) },
        PathHeight { actual, expected } => "gts-proof-path-height" { "actual" => Unsigned(*actual as u128), "expected" => Unsigned(*expected as u128) },
        PeakMismatch { actual, expected } => "gts-proof-peak-mismatch" { "actual" => Bytes(actual), "expected" => Bytes(expected) },
        RootMismatch { actual, expected } => "gts-proof-root-mismatch" { "actual" => Bytes(actual), "expected" => Bytes(expected) },
        Json(cause) => "gts-proof-json" { "cause" => Cause(cause) },
        JsonObject { context, actual } => "gts-proof-json-object" { "context" => Code(context), "actual" => Record(actual) },
        JsonArray { context, actual } => "gts-proof-json-array" { "context" => Code(context), "actual" => Record(actual) },
        JsonMissing { field } => "gts-proof-json-missing" { "field" => Code(field) },
        JsonString { field, actual } => "gts-proof-json-string" { "field" => Code(field), "actual" => Record(actual) },
        JsonUnsigned { field, actual } => "gts-proof-json-unsigned" { "field" => Code(field), "actual" => Record(actual) },
        JsonPlatform { field, actual } => "gts-proof-json-platform" { "field" => Code(field), "actual" => Unsigned(u128::from(*actual)), "maximum" => Unsigned(usize::MAX as u128) },
        Unsupported { declaration, actual, allowed } => "gts-proof-unsupported" { "declaration" => Code(declaration), "actual" => Text(actual), "allowed" => List(allowed.iter().map(|value| ErrorValue::Code(value))) },
        Hex(cause) => "gts-proof-hex" { "cause" => Cause(cause) },
        Torn { offset } => "gts-proof-torn" { "offset" => ByteCount(*offset as u128) },
        Empty => "gts-proof-empty" {},
        Header { item, cause } => "gts-proof-header" { "item" => Unsigned(*item as u128), "cause" => Cause(cause) },
        NotHeader { item, actual } => "gts-proof-not-header" { "item" => Unsigned(*item as u128), "actual" => Record(actual) },
        FileIdMismatch { item, header, actual, expected } => "gts-proof-file-id-mismatch" { "item" => Unsigned(*item as u128), "header" => Boolean(*header), "actual" => Bytes(actual), "expected" => Bytes(expected) },
        FileMissingId { item, header, actual } => "gts-proof-file-missing-id" { "item" => Unsigned(*item as u128), "header" => Boolean(*header), "actual" => Value(actual.as_ref().map_or(ErrorValue::Absent, |value| ErrorValue::Record(value))) },
        NotFrame { item, actual } => "gts-proof-not-frame" { "item" => Unsigned(*item as u128), "actual" => Record(actual) },
        PrevMismatch { item, actual, expected } => "gts-proof-prev-mismatch" { "item" => Unsigned(*item as u128), "actual" => Value(actual.as_ref().map_or(ErrorValue::Absent, |value| ErrorValue::Record(value))), "expected" => Bytes(expected) },
        IndexCount { item, count, limit } => "gts-proof-index-count" { "item" => Unsigned(*item as u128), "count" => Unsigned(*count as u128), "limit" => Unsigned(*limit as u128) },
        IndexHead { item, empty, actual, expected } => "gts-proof-index-head" { "item" => Unsigned(*item as u128), "empty" => Boolean(*empty), "actual" => Bytes(actual), "expected" => Bytes(expected) },
        IndexRoot { item, actual, expected } => "gts-proof-index-root" { "item" => Unsigned(*item as u128), "actual" => Bytes(actual), "expected" => Bytes(expected) },
        NotCovered { target } => "gts-proof-not-covered" { "target" => Bytes(target) },
    }
}
