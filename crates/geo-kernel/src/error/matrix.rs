// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native diagnostic intersection-matrix grammar refusals.

/// Why a diagnostic nine-cell intersection matrix cannot be read.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MatrixInputError {
    /// A character has no intersection-dimension interpretation.
    Entry {
        /// Original character position, not a byte offset.
        index: usize,
        /// Original rejected scalar.
        character: char,
    },
    /// The bounded grammar walk did not finish at exactly nine cells.
    Length {
        /// Original input spelling.
        input: String,
        /// Cells observed before the existing walk stopped (at most ten).
        observed: usize,
    },
}

impl purrdf_hash::error::StructuredError for MatrixInputError {
    purrdf_hash::error_visit! {
        Entry { index, character } => "geo-matrix-entry" {
            "index" => Unsigned(*index as u128), "character" => Character(*character)
        },
        Length { input, observed } => "geo-matrix-length" {
            "input" => Text(input), "observed" => Unsigned(*observed as u128), "expected" => Unsigned(9)
        },
    }
}

impl core::fmt::Display for MatrixInputError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Entry { character, .. } => write!(
                f,
                "{character:?} is not an intersection-matrix entry; a matrix is written with F, 0, 1 and 2 only ({character:?} looks like a PATTERN character, and a pattern is matched against a matrix rather than being one)"
            ),
            Self::Length { input, observed } => write!(
                f,
                "an intersection matrix has exactly nine entries; got {observed} in {input:?}"
            ),
        }
    }
}
impl std::error::Error for MatrixInputError {}
