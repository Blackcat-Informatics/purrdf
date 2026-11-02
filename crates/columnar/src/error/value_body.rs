// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original operands and causes from definition levels and PLAIN value bodies.

use core::{fmt, num::TryFromIntError};
use purrdf_core::ir::pack::bits::PackBitsError;

/// A native definition-level or PLAIN value-body refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ValueBodyReason {
    /// Original native dictionary or delta value-codec refusal.
    Encoding(super::EncodingReason),
    /// The shared varint reader refused an encoded run header.
    Varint {
        /// Byte position at which decoding began.
        offset: usize,
        /// Original byte-codec refusal.
        cause: PackBitsError,
    },
    /// The original unsigned header selects unsupported bit-packed encoding.
    UnsupportedBitPackedRun {
        /// Complete original run header, including its encoding bit.
        header: u64,
    },
    /// A decoded run length does not fit the host address space.
    RunLength {
        /// Original unsigned run length.
        length: u64,
        /// Original integer-conversion refusal.
        cause: TryFromIntError,
    },
    /// A run is empty or extends beyond the remaining rows.
    InvalidRun {
        /// Original decoded run length.
        length: usize,
        /// Rows remaining before this run.
        remaining: usize,
    },
    /// A definition level is outside the fixed Boolean representation.
    DefinitionLevel {
        /// Original encoded level byte.
        level: u8,
    },
    /// Bytes remain after the final definition-level run.
    TrailingLevels {
        /// First byte after the final run.
        offset: usize,
        /// Original enclosing byte length.
        available: usize,
    },
    /// A value's byte range cannot be represented in the host address space.
    OffsetOverflow {
        /// Original range start.
        offset: usize,
        /// Original requested byte length.
        length: usize,
    },
    /// Bytes remain after the last declared value.
    TrailingValues {
        /// First byte after the last declared value.
        offset: usize,
        /// Original enclosing byte length.
        available: usize,
    },
    /// A PLAIN byte-array prefix declares a negative signed length.
    NegativeLength {
        /// Original signed prefix value.
        length: i32,
    },
}

impl purrdf_hash::error::StructuredError for ValueBodyReason {
    purrdf_hash::error_visit! {
        Encoding(cause) => "columnar-value-encoding" { "cause" => Cause(cause) },
        Varint { offset, cause } => "columnar-value-varint" {
            "offset" => ByteCount(*offset as u128), "cause" => Cause(cause),
        },
        UnsupportedBitPackedRun { header } => "columnar-bit-packed-run" {
            "header" => Unsigned(u128::from(*header)),
        },
        RunLength { length, cause } => "columnar-run-length" {
            "length" => Unsigned(u128::from(*length)), "cause" => Cause(cause),
        },
        InvalidRun { length, remaining } => "columnar-invalid-run" {
            "length" => Unsigned(*length as u128), "remaining" => Unsigned(*remaining as u128),
        },
        DefinitionLevel { level } => "columnar-definition-level" {
            "level" => Unsigned(u128::from(*level)),
        },
        TrailingLevels { offset, available } => "columnar-trailing-levels" {
            "offset" => ByteCount(*offset as u128), "available" => ByteCount(*available as u128),
        },
        OffsetOverflow { offset, length } => "columnar-value-offset" {
            "offset" => ByteCount(*offset as u128), "length" => ByteCount(*length as u128),
        },
        TrailingValues { offset, available } => "columnar-trailing-values" {
            "offset" => ByteCount(*offset as u128), "available" => ByteCount(*available as u128),
        },
        NegativeLength { length } => "columnar-negative-length" {
            "length" => Signed(i128::from(*length)),
        },
    }
}

impl fmt::Display for ValueBodyReason {
    /// Render the established detail from original operands without storing text.
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Encoding(cause) => cause.fmt(output),
            Self::Varint { cause, .. } => cause.fmt(output),
            Self::UnsupportedBitPackedRun { header } => header.cast_signed().fmt(output),
            Self::RunLength { .. } => output.write_str("run length exceeds usize"),
            Self::InvalidRun { .. } => output.write_str("zero or overlong run"),
            Self::DefinitionLevel { level } => {
                write!(output, "level is {level}, expected 0 or 1")
            }
            Self::TrailingLevels { .. } => output.write_str("trailing bytes after final run"),
            Self::OffsetOverflow { .. } => output.write_str("offset overflows usize"),
            Self::TrailingValues { .. } => output.write_str("trailing bytes after values"),
            Self::NegativeLength { .. } => output.write_str("negative length"),
        }
    }
}

purrdf_lex::error_source!(ValueBodyReason => {
    Self::Encoding(cause) => cause,
    Self::Varint { cause, .. } => cause,
    Self::RunLength { cause, .. } => cause,
});
