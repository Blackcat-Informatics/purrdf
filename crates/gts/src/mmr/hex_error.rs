// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native MMR identifier decoding refusals.

use core::fmt;

/// A malformed 32-byte hexadecimal identifier.
#[derive(Debug)]
#[non_exhaustive]
pub enum Hex32Error {
    /// The trimmed, unprefixed spelling is not 64 bytes long.
    Length {
        /// Original unprefixed byte count.
        actual: usize,
    },
    /// The shared hexadecimal decoder rejected a digit.
    Digits(purrdf_hash::hex::HexError),
}

impl fmt::Display for Hex32Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Length { .. } => f.write_str("expected a 32-byte hex value"),
            Self::Digits(_) => f.write_str("hex value contains a non-hex character"),
        }
    }
}

purrdf_lex::error_source!(Hex32Error { Digits });

impl purrdf_hash::error::StructuredError for Hex32Error {
    purrdf_hash::error_visit! {
        Length { actual } => "gts-hex32-length" { "actual" => ByteCount(*actual as u128), "expected" => ByteCount(64) },
        Digits(cause) => "gts-hex32-digits" { "cause" => Cause(cause) },
    }
}
