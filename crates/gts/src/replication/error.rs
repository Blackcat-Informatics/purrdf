// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original inventory findings used by replication admission.

use super::DiffStatus;
use crate::model::Diagnostic;
use purrdf_hash::error::ErrorValue;
use purrdf_hash::hex::Lower;

/// The first original defect in a replication inventory.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum InventoryProblem {
    /// A file-level reader refusal.
    Fatal(Diagnostic),
    /// An incomplete or malformed trailing item.
    Torn(crate::wire::TornAppendError),
    /// A reader finding in a particular segment.
    Segment {
        /// Original segment ordinal.
        segment: u64,
        /// Complete original reader finding.
        diagnostic: Diagnostic,
    },
}

impl core::fmt::Display for InventoryProblem {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Fatal(diagnostic) | Self::Segment { diagnostic, .. } => {
                write!(f, "{}: {}", diagnostic.code, diagnostic.detail)
            }
            Self::Torn(cause) => cause.fmt(f),
        }
    }
}

purrdf_lex::error_source!(InventoryProblem => {
    Self::Fatal(diagnostic) | Self::Segment { diagnostic, .. } => diagnostic,
    Self::Torn(cause) => cause,
});

impl purrdf_hash::error::StructuredError for InventoryProblem {
    purrdf_hash::error_visit! {
        Fatal(diagnostic) => "gts-replication-fatal" { "diagnostic" => Cause(diagnostic) },
        Torn(cause) => "gts-replication-torn" { "offset" => Unsigned(u128::from(cause.offset)), "cause" => Cause(&cause.cause) },
        Segment { segment, diagnostic } => "gts-replication-segment" { "segment" => Unsigned(u128::from(*segment)), "diagnostic" => Cause(diagnostic) },
    }
}

/// Original replication admission and continuity findings.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum ReplicationError {
    /// The input inventory is not clean.
    Inventory(Box<InventoryProblem>),
    /// Local or remote inventories contain reader findings.
    Inventories {
        /// First local finding, when present.
        local: Option<Box<InventoryProblem>>,
        /// First remote finding, when present.
        remote: Option<Box<InventoryProblem>>,
    },
    /// The requested peer head is unknown.
    UnknownHead {
        /// Original requested head.
        head: Vec<u8>,
    },
    /// A valid frame with this identity was not found.
    MissingFrame {
        /// Original frame identity.
        frame: Vec<u8>,
    },
    /// A declared offset does not fit the host address space.
    Address {
        /// Offset's role in the operation.
        field: &'static str,
        /// Original full-width offset.
        actual: u64,
    },
    /// A frame suffix is outside the supplied local buffer.
    BufferRange {
        /// First local byte.
        start: usize,
        /// Exclusive final local byte.
        end: usize,
        /// Supplied local buffer length.
        length: usize,
    },
    /// Matching segment heads occur at different byte ranges.
    Alignment {
        /// Segment position in both inventories.
        segment: usize,
        /// Local start and exclusive end.
        local: crate::model::ByteRange,
        /// Remote start and exclusive end.
        remote: crate::model::ByteRange,
    },
    /// Only the final local segment can be extended.
    ExtraSegments {
        /// Common segment prefix length.
        common: usize,
        /// Number of local segments past the permitted last one.
        extra: usize,
    },
    /// Overlapping frames have different identities.
    FrameMismatch {
        /// Segment position.
        segment: usize,
        /// Frame position within that segment.
        frame: usize,
        /// Original local identity.
        local: Vec<u8>,
        /// Original remote identity.
        remote: Vec<u8>,
    },
    /// The remote extension is invalid or does not chain to the local prefix.
    Extension {
        /// Segment position.
        segment: usize,
        /// Whether the first new remote frame passed validation.
        valid: bool,
        /// Original previous-link claim.
        previous: Option<Vec<u8>>,
        /// Expected last local frame or header identity.
        expected: Option<Vec<u8>>,
    },
    /// The caller supplied a non-spliceable diff result.
    Unspliceable {
        /// Original diff status.
        status: DiffStatus,
    },
    /// A fetching diff has no splice offset.
    MissingSpliceOffset,
    /// The splice offset exceeds local bytes.
    SpliceOffset {
        /// Original requested offset.
        offset: u64,
        /// Local byte length.
        length: usize,
    },
    /// A fetch range exceeds or reverses remote bytes.
    FetchRange {
        /// Original start.
        start: u64,
        /// Original exclusive end.
        end: u64,
        /// Remote byte length.
        length: usize,
    },
}

impl core::fmt::Display for ReplicationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Inventory(problem) => problem.fmt(f),
            Self::Inventories { local, remote } => {
                if let Some(local) = local {
                    write!(f, "local: {local}")?;
                }
                if let Some(remote) = remote {
                    if local.is_some() {
                        f.write_str("; ")?;
                    }
                    write!(f, "remote: {remote}")?;
                }
                Ok(())
            }
            Self::UnknownHead { .. } => f.write_str("unknown peer head; scan required"),
            Self::MissingFrame { frame } => write!(f, "frame {} not found", Lower(frame)),
            Self::Address { field, .. } => write!(f, "{field} exceeds local address space"),
            Self::BufferRange { .. } => f.write_str("frame range is outside buffered source"),
            Self::Alignment {
                segment,
                local,
                remote,
            } => write!(
                f,
                "segment {segment} heads match but byte ranges diverge (local {}..{}, remote {}..{}); likely a re-encoded mirror",
                local.start, local.end, remote.start, remote.end
            ),
            Self::ExtraSegments { common, extra } => write!(
                f,
                "local has {extra} segment(s) beyond the common prefix at segment {common}; only local's own last segment may be a partial extension"
            ),
            Self::FrameMismatch { segment, frame, .. } => write!(
                f,
                "segment {segment} frame {frame} id diverges from remote's frame at the same index"
            ),
            Self::Extension { segment, .. } => write!(
                f,
                "segment {segment}'s first new remote frame does not chain onto local's head; possible fork"
            ),
            Self::Unspliceable { .. } => f.write_str("diff result cannot be spliced"),
            Self::MissingSpliceOffset => f.write_str("diff result has no splice offset"),
            Self::SpliceOffset { offset, length } => {
                write!(f, "splice offset {offset} exceeds local length {length}")
            }
            Self::FetchRange { start, end, length } => write!(
                f,
                "fetch range {start}..{end} exceeds remote length {length}"
            ),
        }
    }
}

purrdf_lex::error_source!(ReplicationError => {
    Self::Inventory(problem) => &**problem,
});

impl purrdf_hash::error::StructuredError for ReplicationError {
    purrdf_hash::error_visit! {
        Inventory(problem) => "gts-replication-inventory" { "cause" => Cause(&**problem) },
        Inventories { local, remote } => "gts-replication-inventories" {
            "local" => Value(local.as_deref().map_or(ErrorValue::Absent, |value| ErrorValue::Cause(value))),
            "remote" => Value(remote.as_deref().map_or(ErrorValue::Absent, |value| ErrorValue::Cause(value)))
        },
        UnknownHead { head } => "gts-replication-unknown-head" { "head" => Bytes(head) },
        MissingFrame { frame } => "gts-replication-missing-frame" { "frame" => Bytes(frame) },
        Address { field, actual } => "gts-replication-address" { "field" => Code(field), "actual" => Unsigned(u128::from(*actual)), "limit" => Unsigned(usize::MAX as u128) },
        BufferRange { start, end, length } => "gts-replication-buffer-range" { "start" => Unsigned(*start as u128), "end" => Unsigned(*end as u128), "length" => Unsigned(*length as u128) },
        Alignment { segment, local, remote } => "gts-replication-alignment" { "segment" => Unsigned(*segment as u128), "local_start" => Unsigned(u128::from(local.start)), "local_end" => Unsigned(u128::from(local.end)), "remote_start" => Unsigned(u128::from(remote.start)), "remote_end" => Unsigned(u128::from(remote.end)) },
        ExtraSegments { common, extra } => "gts-replication-extra-segments" { "common" => Unsigned(*common as u128), "extra" => Unsigned(*extra as u128) },
        FrameMismatch { segment, frame, local, remote } => "gts-replication-frame-mismatch" { "segment" => Unsigned(*segment as u128), "frame" => Unsigned(*frame as u128), "local" => Bytes(local), "remote" => Bytes(remote) },
        Extension { segment, valid, previous, expected } => "gts-replication-extension" { "segment" => Unsigned(*segment as u128), "valid" => Boolean(*valid), "previous" => Value(previous.as_deref().map_or(ErrorValue::Absent, ErrorValue::Bytes)), "expected" => Value(expected.as_deref().map_or(ErrorValue::Absent, ErrorValue::Bytes)) },
        Unspliceable { status } => "gts-replication-unspliceable" { "status" => Record(status) },
        MissingSpliceOffset => "gts-replication-missing-splice-offset" {},
        SpliceOffset { offset, length } => "gts-replication-splice-offset" { "offset" => Unsigned(u128::from(*offset)), "length" => Unsigned(*length as u128) },
        FetchRange { start, end, length } => "gts-replication-fetch-range" { "start" => Unsigned(u128::from(*start)), "end" => Unsigned(u128::from(*end)), "length" => Unsigned(*length as u128) },
    }
}
