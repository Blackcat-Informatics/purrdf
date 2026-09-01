// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The two typed failure modes: a scanner failure ([`CdtError`], which always
//! carries a byte offset into the lexical form) and a comparison failure
//! ([`CdtTypeError`], which is a SPARQL type error and has no position).

use alloc::string::String;
use core::fmt;

/// A failure to map a lexical form into a composite value.
///
/// Every variant carries `offset`: the **byte offset into the lexical form** at
/// which the failure was detected, so a diagnostic can point at the exact
/// character. Offsets are byte offsets into the original `&str` and are always on a
/// UTF-8 character boundary or at `lexical.len()`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CdtError {
    /// The lexical form is longer than [`crate::MAX_LEXICAL_BYTES`]. Detected
    /// before scanning starts, so `offset` is [`crate::MAX_LEXICAL_BYTES`] itself.
    InputTooLarge {
        /// The offset at which the accepted prefix ends.
        offset: usize,
        /// The actual length of the offered lexical form, in bytes.
        length: usize,
    },
    /// Composite nesting deeper than [`crate::MAX_NESTING_DEPTH`].
    DepthExceeded {
        /// Byte offset of the opening delimiter that would have exceeded the bound.
        offset: usize,
        /// The bound that was exceeded.
        limit: usize,
    },
    /// More elements than [`crate::MAX_ELEMENTS`], counted across every level.
    TooManyElements {
        /// Byte offset of the element that would have exceeded the bound.
        offset: usize,
        /// The bound that was exceeded.
        limit: usize,
    },
    /// A byte was found where the grammar admits something else (a missing `,`, a
    /// missing `:`, a trailing comma, an unexpected closing delimiter, …).
    Unexpected {
        /// Byte offset of the offending byte.
        offset: usize,
        /// What the grammar admitted at this position.
        expected: &'static str,
    },
    /// The lexical form ended while a production was still open (an unterminated
    /// list, map, string, or IRIREF). `offset` is the end of input.
    UnexpectedEnd {
        /// Byte offset of the end of input.
        offset: usize,
        /// What the grammar admitted at this position.
        expected: &'static str,
    },
    /// An `\u`/`\U`/`ECHAR` escape sequence is malformed, or names a code point that
    /// is not a Unicode scalar value.
    BadEscape {
        /// Byte offset of the `\` that opens the escape.
        offset: usize,
        /// A short, stable explanation.
        reason: &'static str,
    },
    /// An `IRIREF` is syntactically an IRI reference but is not **absolute** (it has
    /// no scheme), or is not a valid IRI at all. CDT lexical forms carry no base, so
    /// a relative reference could never be resolved.
    NotAbsoluteIri {
        /// Byte offset of the opening `<`.
        offset: usize,
        /// The offending IRI text, after escape processing.
        iri: String,
        /// A short, stable explanation.
        reason: &'static str,
    },
    /// A `LANGTAG` is malformed, or a `--dir` suffix is neither `ltr` nor `rtl`.
    BadLanguageTag {
        /// Byte offset of the `@`.
        offset: usize,
        /// A short, stable explanation.
        reason: &'static str,
    },
    /// A `NumericLiteral` is not a valid `INTEGER`, `DECIMAL` or `DOUBLE`.
    BadNumericLiteral {
        /// Byte offset of the first byte of the numeric literal.
        offset: usize,
        /// A short, stable explanation.
        reason: &'static str,
    },
    /// A `BLANK_NODE_LABEL` is malformed (empty, or ending in `.`).
    BadBlankNodeLabel {
        /// Byte offset of the `_`.
        offset: usize,
        /// A short, stable explanation.
        reason: &'static str,
    },
    /// Two entries of one map have the same key. SEP-0009 requires map keys to be
    /// pairwise distinct; see [`crate::parse_map`] for the exact distinctness rule
    /// this crate enforces.
    ///
    /// Raised by [`crate::parse_map`] for a scanned map and by
    /// [`CdtValue::map`](crate::CdtValue::map) for a programmatically built one — the
    /// two differ only in what `offset` indexes into, which each documents.
    DuplicateMapKey {
        /// Byte offset of the first byte of the *second* occurrence of the key: into
        /// the scanned lexical form, or — for a map with no scanned form — into the
        /// canonical form the map would have.
        offset: usize,
        /// The canonical lexical form of the duplicated key.
        key: String,
    },
    /// The lexical form is a complete composite followed by more non-whitespace
    /// text.
    TrailingText {
        /// Byte offset of the first byte of the trailing text.
        offset: usize,
    },
}

impl CdtError {
    /// The byte offset into the lexical form at which the failure was detected.
    #[must_use]
    pub const fn offset(&self) -> usize {
        match self {
            Self::InputTooLarge { offset, .. }
            | Self::DepthExceeded { offset, .. }
            | Self::TooManyElements { offset, .. }
            | Self::Unexpected { offset, .. }
            | Self::UnexpectedEnd { offset, .. }
            | Self::BadEscape { offset, .. }
            | Self::NotAbsoluteIri { offset, .. }
            | Self::BadLanguageTag { offset, .. }
            | Self::BadNumericLiteral { offset, .. }
            | Self::BadBlankNodeLabel { offset, .. }
            | Self::DuplicateMapKey { offset, .. }
            | Self::TrailingText { offset } => *offset,
        }
    }
}

impl fmt::Display for CdtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputTooLarge { offset, length } => write!(
                f,
                "lexical form is {length} bytes, over the {offset}-byte bound"
            ),
            Self::DepthExceeded { offset, limit } => {
                write!(f, "at byte {offset}: nesting deeper than {limit}")
            }
            Self::TooManyElements { offset, limit } => {
                write!(f, "at byte {offset}: more than {limit} elements")
            }
            Self::Unexpected { offset, expected } => {
                write!(f, "at byte {offset}: expected {expected}")
            }
            Self::UnexpectedEnd { offset, expected } => {
                write!(f, "at byte {offset}: input ended, expected {expected}")
            }
            Self::BadEscape { offset, reason } => {
                write!(f, "at byte {offset}: bad escape sequence: {reason}")
            }
            Self::NotAbsoluteIri {
                offset,
                iri,
                reason,
            } => {
                write!(f, "at byte {offset}: <{iri}> is not usable here: {reason}")
            }
            Self::BadLanguageTag { offset, reason } => {
                write!(f, "at byte {offset}: bad language tag: {reason}")
            }
            Self::BadNumericLiteral { offset, reason } => {
                write!(f, "at byte {offset}: bad numeric literal: {reason}")
            }
            Self::BadBlankNodeLabel { offset, reason } => {
                write!(f, "at byte {offset}: bad blank node label: {reason}")
            }
            Self::DuplicateMapKey { offset, key } => {
                write!(f, "at byte {offset}: duplicate map key {key}")
            }
            Self::TrailingText { offset } => {
                write!(f, "at byte {offset}: trailing text after the composite")
            }
        }
    }
}

impl core::error::Error for CdtError {}

/// A SPARQL **type error** raised by a value-space comparison.
///
/// This is the third outcome of [`crate::list_equal`] / [`crate::list_less_than`]
/// and their map counterparts: distinct from `Ok(false)`. A consumer must never
/// read it as "not equal" or "not less" — SPARQL propagates it, and the whole
/// enclosing expression becomes an error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CdtTypeError {
    /// A short, stable explanation of which pair could not be compared and why.
    pub reason: &'static str,
}

impl fmt::Display for CdtTypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "type error: {}", self.reason)
    }
}

impl core::error::Error for CdtTypeError {}
