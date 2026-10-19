// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The original JSON refusal beneath a geometry carrier.

/// Native JSON grammar failure and the original source character at its offset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JsonSyntaxError {
    /// Original JSON reader/escape failure, including all original limit fields.
    pub cause: purrdf_lex::json::Error,
    /// Original source character, or no character at the end of the input.
    pub found: Option<char>,
}

impl core::hash::Hash for JsonSyntaxError {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        // Equality includes the complete native cause. Its original kind and
        // offset already satisfy the hash contract without rendering it or
        // imposing Hash on the JSON error API.
        core::hash::Hash::hash(&core::mem::discriminant(&self.cause.kind()), state);
        core::hash::Hash::hash(&self.cause.offset(), state);
        core::hash::Hash::hash(&self.found, state);
    }
}

impl purrdf_hash::error::StructuredError for JsonSyntaxError {
    purrdf_hash::error_visit! {
        @record self { cause, found } => "geo-json-syntax" {
            "cause" => Cause(&self.cause), "found" => Value(match self.found {
                Some(character) => purrdf_hash::error::ErrorValue::Character(character),
                None => purrdf_hash::error::ErrorValue::Absent,
            })
        }
    }
}

impl core::fmt::Display for JsonSyntaxError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{}, found {}",
            self.cause,
            crate::json::Found(self.found)
        )
    }
}
purrdf_lex::error_source!(JsonSyntaxError, cause);
