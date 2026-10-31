// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original operands from native page-body consistency checks.

use core::fmt;

/// A native Data Page V2 body refusal after header and metadata validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PageBodyReason {
    /// No compressed frame is present.
    EmptyFrameStream,
    /// Known frame declarations do not sum to the Parquet value-body size.
    FrameContentTotal {
        /// Total bytes claimed by the native frame headers, without decoding.
        declared: u64,
        /// Original Parquet plain value-body size.
        expected: u64,
    },
    /// A frame's declared output exceeds the remaining Parquet value body.
    FrameContentSize {
        /// Original full-width frame content size.
        declared: u64,
        /// Remaining bytes allowed by the original Parquet declaration.
        remaining: u64,
    },
    /// Definition-level presence disagrees with schema nullability.
    DefinitionPresence {
        /// Original definition-level byte count.
        length: usize,
        /// Original column nullability.
        repetition: crate::Repetition,
    },
    /// The declared body is smaller than its definition-level section.
    DefinitionSize {
        /// Original uncompressed page byte count.
        page: usize,
        /// Original definition-level byte count.
        definitions: usize,
    },
    /// An uncompressed value body has a different length from its declaration.
    PlainSize {
        /// Original encoded value-body byte count.
        actual: usize,
        /// Declared plain value-body byte count.
        expected: usize,
    },
    /// Declared PLAIN bytes cannot encode the page's present-value count.
    Cardinality {
        /// Original declared uncompressed value-body bytes.
        declared: usize,
        /// Bytes required by the physical value count.
        required: usize,
        /// Original physical value type.
        physical_type: crate::PhysicalType,
        /// Original number of present values.
        present: usize,
    },
    /// Successful decompression produced a different length from its declaration.
    DecodedSize {
        /// Original successful decoder output byte count.
        actual: usize,
        /// Declared plain value-body byte count.
        expected: usize,
    },
    /// Definition levels encode a different null count from the page header.
    NullCount {
        /// Original decoded null count.
        actual: usize,
        /// Original signed header count.
        declared: i32,
    },
}

impl purrdf_hash::error::StructuredError for PageBodyReason {
    purrdf_hash::error_visit! {
        EmptyFrameStream => "columnar-zstd-empty-frame-stream" {},
        FrameContentTotal { declared, expected } => "columnar-frame-content-total" {
            "declared" => ByteCount(u128::from(*declared)), "expected" => ByteCount(u128::from(*expected)),
        },
        FrameContentSize { declared, remaining } => "columnar-frame-content-size" {
            "declared" => ByteCount(u128::from(*declared)), "remaining" => ByteCount(u128::from(*remaining)),
        },
        DefinitionPresence { length, repetition } => "columnar-definition-presence" {
            "length" => ByteCount(*length as u128),
            "repetition" => Code(match repetition {
                crate::Repetition::Required => "required",
                crate::Repetition::Optional => "optional",
            }),
        },
        DefinitionSize { page, definitions } => "columnar-definition-size" {
            "page" => ByteCount(*page as u128), "definitions" => ByteCount(*definitions as u128),
        },
        PlainSize { actual, expected } => "columnar-plain-size" {
            "actual" => ByteCount(*actual as u128), "expected" => ByteCount(*expected as u128),
        },
        Cardinality { declared, required, physical_type, present } => "columnar-plain-cardinality" {
            "declared" => ByteCount(*declared as u128), "required" => ByteCount(*required as u128), "physical_type" => Code(match physical_type { crate::PhysicalType::Int64 => "int64", crate::PhysicalType::ByteArray => "byte-array" }), "present" => Unsigned(*present as u128),
        },
        DecodedSize { actual, expected } => "columnar-decoded-size" {
            "actual" => ByteCount(*actual as u128), "expected" => ByteCount(*expected as u128),
        },
        NullCount { actual, declared } => "columnar-null-count" {
            "actual" => Unsigned(*actual as u128), "declared" => Signed(i128::from(*declared)),
        },
    }
}

impl fmt::Display for PageBodyReason {
    /// Render the established body diagnostic from its original scalar operands.
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyFrameStream => output.write_str("empty frame stream"),
            Self::FrameContentTotal { declared, expected } => {
                write!(
                    output,
                    "frame headers declare {declared} bytes, Parquet declares {expected}"
                )
            }
            Self::FrameContentSize {
                declared,
                remaining,
            } => {
                write!(
                    output,
                    "declares {declared} decoded bytes, only {remaining} remain in the Parquet value body"
                )
            }
            Self::DefinitionPresence { .. } => {
                output.write_str("presence disagrees with schema nullability")
            }
            Self::DefinitionSize { .. } => {
                output.write_str("smaller than definition-level section")
            }
            Self::PlainSize { actual, expected } => {
                write!(output, "has {actual} bytes, expected {expected}")
            }
            Self::Cardinality {
                declared,
                required,
                physical_type,
                present,
            } => {
                let relation = if *physical_type == crate::PhysicalType::Int64 || *present == 0 {
                    "exactly"
                } else {
                    "at least"
                };
                write!(
                    output,
                    "declares {declared} value bytes, requires {relation} {required}"
                )
            }
            Self::DecodedSize { actual, expected } => {
                write!(output, "decoded {actual} bytes, expected {expected}")
            }
            Self::NullCount { actual, declared } => {
                write!(output, "encode {actual} nulls, header declares {declared}")
            }
        }
    }
}

impl std::error::Error for PageBodyReason {}
