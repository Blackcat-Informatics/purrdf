// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Runtime DE-9IM pattern grammar refusals and original input data.

/// Why a runtime DE-9IM pattern is malformed.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PatternInputError {
    /// A scalar is absent from the pattern alphabet.
    Character {
        /// Original refused scalar.
        character: char,
    },
    /// The input does not contain nine pattern positions.
    Length {
        /// Original input spelling.
        input: String,
        /// Full original character count.
        count: usize,
    },
}

impl purrdf_hash::error::StructuredError for PatternInputError {
    purrdf_hash::error_visit! {
        Character { character } => "geo-pattern-character" { "character" => Character(*character) },
        Length { input, count } => "geo-pattern-length" {
            "input" => Text(input), "count" => Unsigned(*count as u128), "expected" => Unsigned(9)
        },
    }
}

impl PatternInputError {
    pub(crate) fn native_text_bytes(&self) -> usize {
        match self {
            Self::Character { character: _ } => 0,
            Self::Length { input, count: _ } => input.capacity(),
        }
    }
}

impl core::fmt::Display for PatternInputError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Character { character } => write!(
                f,
                "{character:?} is not a DE-9IM pattern character; a pattern is written with F, T, 0, 1, 2 and * only"
            ),
            Self::Length { input, count } => write!(
                f,
                "a DE-9IM pattern has exactly nine positions; {input:?} has {count}"
            ),
        }
    }
}
impl std::error::Error for PatternInputError {}
