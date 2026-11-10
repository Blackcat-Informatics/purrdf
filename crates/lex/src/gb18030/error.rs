// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original native and caller-owned transcoding refusals.

use super::{Direction, Extent, Profile};

/// A strict source or resource refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// A byte cannot continue the current GB 18030 or UTF-8 scalar.
    Malformed,
    /// Final input ended inside a scalar.
    Incomplete,
    /// Syntactically four-byte GB code lies outside assigned mapping ranges.
    Unassigned,
    /// Original input ceiling would be exceeded.
    InputLimit {
        /// Inclusive caller ceiling.
        limit: u64,
    },
    /// Output ceiling would be exceeded before emission.
    OutputLimit {
        /// Inclusive caller ceiling.
        limit: u64,
    },
    /// A source or output counter cannot represent the next byte.
    OffsetOverflow,
    /// An already finished or failed stream was driven again.
    Closed,
}

/// Exact original input, edition and native decoder cause.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Error {
    /// Selected mapping edition.
    pub profile: Profile,
    /// Selected source and output encodings.
    pub direction: Direction,
    /// Source sequence extent; empty for a refusal before source consumption.
    pub source: Extent,
    /// Original sequence bytes; only `length` bytes participate.
    pub bytes: [u8; 4],
    /// Number of original sequence bytes.
    pub length: u8,
    /// Typed refusal.
    pub kind: ErrorKind,
    /// Original UTF-8 validation error on the encoding path, when applicable.
    pub utf8: Option<std::str::Utf8Error>,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{} {:?} at source bytes {}..{}: {:?}",
            self.profile.name(),
            self.direction,
            self.source.start,
            self.source.end,
            self.kind
        )
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.utf8.as_ref().map(|cause| cause as _)
    }
}
impl purrdf_hash::error::StructuredError for ErrorKind {
    purrdf_hash::error_visit! {
        Malformed => "gb18030-malformed" {},
        Incomplete => "gb18030-incomplete" {},
        Unassigned => "gb18030-unassigned" {},
        InputLimit { limit } => "gb18030-input-limit" { "limit" => ByteCount(u128::from(*limit)) },
        OutputLimit { limit } => "gb18030-output-limit" { "limit" => ByteCount(u128::from(*limit)) },
        OffsetOverflow => "gb18030-offset-overflow" {},
        Closed => "gb18030-closed" {},
    }
}
impl purrdf_hash::error::StructuredError for Error {
    purrdf_hash::error_visit! {
        @record self { profile, direction, source, bytes, length, kind, utf8 }
        => "gb18030-transcode" {
            "profile" => Code(self.profile.name()),
            "direction" => Code(match self.direction { Direction::Decode => "decode", Direction::Encode => "encode" }),
            "start" => ByteCount(u128::from(self.source.start)),
            "end" => ByteCount(u128::from(self.source.end)),
            "bytes" => Bytes(&self.bytes[..usize::from(self.length)]),
            "kind" => Record(&self.kind),
            "utf8" => Value(self.utf8.as_ref().map_or(purrdf_hash::error::ErrorValue::Absent, |cause| purrdf_hash::error::ErrorValue::Cause(cause))),
        }
    }
}

/// Native refusal, or the exact caller work/cancellation/output failure.
#[derive(Debug)]
pub enum StreamError<C, O> {
    /// Native failure with original profile and source.
    Native(Error),
    /// Unchanged work or cancellation error.
    Control(C),
    /// Unchanged sink error, including caller allocation/admission failures.
    Output(O),
}
impl<C: core::fmt::Display, O: core::fmt::Display> core::fmt::Display for StreamError<C, O> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Native(cause) => cause.fmt(f),
            Self::Control(cause) => cause.fmt(f),
            Self::Output(cause) => cause.fmt(f),
        }
    }
}
impl<C: std::error::Error + 'static, O: std::error::Error + 'static> std::error::Error
    for StreamError<C, O>
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(match self {
            Self::Native(cause) => cause,
            Self::Control(cause) => cause,
            Self::Output(cause) => cause,
        })
    }
}

impl<C: purrdf_hash::error::StructuredError, O: purrdf_hash::error::StructuredError>
    purrdf_hash::error::StructuredError for StreamError<C, O>
{
    purrdf_hash::error_visit! {
        Native(cause) => "gb18030-native" { "cause" => Record(cause) },
        Control(cause) => "gb18030-control" { "cause" => Record(cause) },
        Output(cause) => "gb18030-output" { "cause" => Record(cause) },
    }
}
