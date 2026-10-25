// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original operands from native page-body consistency checks.

use core::fmt;

/// A native Data Page V2 body refusal after header and metadata validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PageBodyReason {
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
            Self::DefinitionPresence { .. } => {
                output.write_str("presence disagrees with schema nullability")
            }
            Self::DefinitionSize { .. } => {
                output.write_str("smaller than definition-level section")
            }
            Self::PlainSize { actual, expected } => {
                write!(output, "has {actual} bytes, expected {expected}")
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
