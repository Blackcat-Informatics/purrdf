// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Allocation-free archive refusal context.

use super::Span;
use purrdf_hash::error::IoFailure;
use purrdf_lex::allocation::StorageError;
use std::{fmt, num::ParseIntError, str::Utf8Error};

/// The explicitly admitted archive resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Limit {
    /// Supplied archive bytes.
    Source,
    /// Physical member and extension headers.
    Entries,
    /// Cumulative extension body bytes.
    Metadata,
    /// Current admitted layout total, including unreleased construction scratch.
    Storage,
    /// Scans, comparisons and admitted storage work.
    Work,
}
impl Limit {
    pub(super) const fn name(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Entries => "entries",
            Self::Metadata => "metadata",
            Self::Storage => "storage",
            Self::Work => "work",
        }
    }
}

/// A malformed or unsupported archive construct.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Problem {
    /// Header magic or version selects an unsupported layout.
    Format,
    /// A numeric header field is not strict POSIX octal.
    Octal,
    /// A PAX integer is not an unsigned decimal spelling.
    Decimal,
    /// A PAX timestamp is not a signed decimal timestamp.
    Timestamp,
    /// A PAX record has no length delimiter.
    PaxLength,
    /// A PAX record length lies outside its body.
    PaxOverrun,
    /// A PAX record lacks its newline.
    PaxNewline,
    /// A PAX record lacks a nonempty key and equals delimiter.
    PaxEquals,
    /// A nonzero body pad changes the framing contract.
    Padding,
    /// Nonzero data follows the end markers.
    Trailing,
    /// An extension has no following real member.
    PendingMetadata,
    /// A selected metadata keyword changes unsupported sparse framing.
    Sparse,
    /// The stream has no real member.
    Empty,
}
impl Problem {
    pub(super) const fn name(self) -> &'static str {
        match self {
            Self::Format => "format",
            Self::Octal => "octal",
            Self::Decimal => "decimal",
            Self::Timestamp => "timestamp",
            Self::PaxLength => "pax-length",
            Self::PaxOverrun => "pax-overrun",
            Self::PaxNewline => "pax-newline",
            Self::PaxEquals => "pax-equals",
            Self::Padding => "padding",
            Self::Trailing => "trailing",
            Self::PendingMetadata => "pending-metadata",
            Self::Sparse => "sparse",
            Self::Empty => "empty",
        }
    }
}

/// The actual explicit source operation that failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Operation {
    /// Move the explicit seekable source to an original location.
    Seek,
    /// Read an original physical header.
    Header,
    /// Read an original extension body.
    Metadata,
    /// Read a selected original metadata field.
    Field,
    /// Read the padding after an effective body.
    Padding,
    /// Read archive end markers or trailing padding.
    Terminator,
}
impl Operation {
    pub(super) const fn name(self) -> &'static str {
        match self {
            Self::Seek => "seek tar member boundary",
            Self::Header => "read tar header",
            Self::Metadata => "read tar member body",
            Self::Field => "read tar metadata field",
            Self::Padding => "read tar body padding",
            Self::Terminator => "read tar end markers",
        }
    }
}

/// A complete inventory could not be established. Spans always name original bytes.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// A caller-selected resource would be exceeded before the operation.
    Limit {
        /// Resource whose allowance was refused.
        kind: Limit,
        /// Exact attempted count.
        attempted: u128,
        /// Original allowance.
        maximum: u128,
        /// Original archive location of the refused operation.
        offset: u64,
    },
    /// Native physical storage or caller admission refused.
    Storage {
        /// Original operation location.
        offset: u64,
        /// Native original refusal; caller admission retains its own cause.
        cause: StorageError,
    },
    /// A required byte range is incomplete.
    Truncated {
        /// Required original byte range.
        required: Span,
        /// Actual original archive length.
        available: u64,
    },
    /// A header checksum differs from its stored value.
    Checksum {
        /// Original header location.
        offset: u64,
        /// Computed unsigned checksum.
        actual: u64,
        /// Original stated checksum.
        expected: u64,
    },
    /// A physical record kind is unsupported.
    Kind {
        /// Original header location.
        offset: u64,
        /// Original type flag.
        actual: u8,
    },
    /// An original field is malformed or selects unsupported semantics.
    Invalid {
        /// Exact original bytes of the malformed construct.
        span: Span,
        /// Typed violated grammar or framing rule.
        problem: Problem,
    },
    /// Text conversion refused the original bytes rather than replacing them.
    Utf8 {
        /// Exact original rejected field.
        span: Span,
        /// Original native UTF-8 cause.
        cause: Utf8Error,
    },
    /// A numeric value cannot be represented.
    Integer {
        /// Original numeric spelling.
        span: Span,
        /// Original native integer cause.
        cause: ParseIntError,
    },
    /// A PAX record component is not UTF-8.
    PaxUtf8 {
        /// Original first length digit of this record.
        record: u64,
        /// Exact rejected component.
        span: Span,
        /// Original complete extension body.
        body: Span,
        /// Whether this is a length rather than record text.
        length: bool,
        /// Original native decoder cause.
        cause: Utf8Error,
    },
    /// A PAX record length cannot be represented.
    PaxInteger {
        /// Original first length digit of this record.
        record: u64,
        /// Exact original length spelling.
        span: Span,
        /// Original extension body.
        body: Span,
        /// Original native integer cause.
        cause: ParseIntError,
    },
    /// A PAX record violates its physical record grammar.
    Pax {
        /// Original first length digit of this record.
        record: u64,
        /// Original rejected record or remaining body.
        span: Span,
        /// Original complete extension body.
        body: Span,
        /// Original stated record byte length, when available.
        length: usize,
        /// Exact violated record rule.
        problem: Problem,
    },
    /// Explicit seekable source access failed.
    Io {
        /// Actual original source operation.
        operation: Operation,
        /// Original operation location.
        offset: u64,
        /// Original native I/O cause.
        cause: IoFailure,
    },
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Limit {
                kind,
                attempted,
                maximum,
                offset,
            } => write!(
                f,
                "tar {} allowance {maximum} refuses {attempted} at byte {offset}",
                kind.name()
            ),
            Self::Storage { offset, cause } => write!(f, "tar storage at byte {offset}: {cause}"),
            Self::Truncated {
                required,
                available,
            } => write!(
                f,
                "tar bytes {}..{} exceed archive length {available}",
                required.start, required.end
            ),
            Self::Checksum {
                offset,
                actual,
                expected,
            } => write!(
                f,
                "tar checksum at byte {offset}: {actual:o}, expected {expected:o}"
            ),
            Self::Kind { offset, actual } => {
                write!(f, "unsupported tar kind {actual} at byte {offset}")
            }
            Self::Invalid { span, problem } => write!(
                f,
                "invalid tar {} at bytes {}..{}",
                problem.name(),
                span.start,
                span.end
            ),
            Self::Utf8 { span, cause } => write!(f, "tar text at byte {}: {cause}", span.start),
            Self::Integer { span, cause } => {
                write!(f, "tar number at byte {}: {cause}", span.start)
            }
            Self::PaxUtf8 { span, cause, .. } => {
                write!(f, "tar PAX text at byte {}: {cause}", span.start)
            }
            Self::PaxInteger { span, cause, .. } => {
                write!(f, "tar PAX length at byte {}: {cause}", span.start)
            }
            Self::Pax { span, problem, .. } => {
                write!(f, "invalid tar {} at byte {}", problem.name(), span.start)
            }
            Self::Io {
                operation,
                offset,
                cause,
            } => write!(f, "{} at byte {offset}: {cause}", operation.name()),
        }
    }
}
purrdf_lex::error_source!(Error => {
    Self::Storage { cause, .. } => cause,
    Self::Utf8 { cause, .. } => cause,
    Self::Integer { cause, .. } => cause,
    Self::Io { cause, .. } => cause,
    Self::PaxUtf8 { cause, .. } => cause,
    Self::PaxInteger { cause, .. } => cause,
});
impl purrdf_hash::error::StructuredError for Error {
    purrdf_hash::error_visit! {
        Limit { kind, attempted, maximum, offset } => "gts-tar-inventory-limit" { "kind" => Code(kind.name()), "attempted" => Unsigned(*attempted), "maximum" => Unsigned(*maximum), "offset" => ByteCount(u128::from(*offset)) },
        Storage { offset, cause } => "gts-tar-inventory-storage" { "offset" => ByteCount(u128::from(*offset)), "cause" => Cause(cause) },
        Truncated { required, available } => "gts-tar-inventory-truncated" { "required" => Cause(required), "available" => ByteCount(u128::from(*available)) },
        Checksum { offset, actual, expected } => "gts-tar-inventory-checksum" { "offset" => ByteCount(u128::from(*offset)), "actual" => Unsigned(u128::from(*actual)), "expected" => Unsigned(u128::from(*expected)) },
        Kind { offset, actual } => "gts-tar-inventory-kind" { "offset" => ByteCount(u128::from(*offset)), "actual" => Unsigned(u128::from(*actual)) },
        Invalid { span, problem } => "gts-tar-inventory-invalid" { "span" => Cause(span), "problem" => Code(problem.name()) },
        Utf8 { span, cause } => "gts-tar-inventory-utf8" { "span" => Cause(span), "cause" => Cause(cause) },
        Integer { span, cause } => "gts-tar-inventory-integer" { "span" => Cause(span), "cause" => Cause(cause) },
        PaxUtf8 { record, span, body, length, cause } => "gts-tar-inventory-pax-utf8" { "record" => ByteCount(u128::from(*record)), "span" => Cause(span), "body" => Cause(body), "length" => Boolean(*length), "cause" => Cause(cause) },
        PaxInteger { record, span, body, cause } => "gts-tar-inventory-pax-integer" { "record" => ByteCount(u128::from(*record)), "span" => Cause(span), "body" => Cause(body), "cause" => Cause(cause) },
        Pax { record, span, body, length, problem } => "gts-tar-inventory-pax" { "record" => ByteCount(u128::from(*record)), "span" => Cause(span), "body" => Cause(body), "length" => ByteCount(*length as u128), "problem" => Code(problem.name()) },
        Io { operation, offset, cause } => "gts-tar-inventory-io" { "operation" => Code(operation.name()), "offset" => ByteCount(u128::from(*offset)), "cause" => Cause(cause) },
    }
}
