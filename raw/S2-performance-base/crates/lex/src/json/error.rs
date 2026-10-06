// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The one refusal every JSON reading entry point returns.

use core::fmt;

use crate::json_escape::JsonEscapeErrorKind;

/// What is wrong with a JSON document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    /// The grammar required the named element here (RFC 8259 §§2–7).
    Expected(&'static str),
    /// A string holds a raw control character (U+0000–U+001F), which RFC 8259
    /// §7 requires to be escaped.
    RawControl,
    /// A malformed escape, or a `\u` escape that names an unpaired surrogate.
    Escape(JsonEscapeErrorKind),
    /// The input is not UTF-8 (RFC 8259 §8.1).
    InvalidUtf8,
    /// Something other than whitespace follows the top-level value.
    Trailing,
    /// More containers are open at once than [`super::Limits::max_depth`]
    /// allows.
    Depth {
        /// The cap that was exceeded.
        limit: usize,
    },
    /// More value occurrences than [`super::Limits::max_values`] allows.
    Values {
        /// The cap that was exceeded.
        limit: u64,
    },
    /// A decoded string or member name is longer than
    /// [`super::Limits::max_string_bytes`].
    StringBytes {
        /// The cap that was exceeded.
        limit: usize,
    },
    /// An object repeats a member name while
    /// [`super::Limits::unique_members`] is set (RFC 7493 §2.3).
    DuplicateMember,
}

/// A refused JSON document: what is wrong, and the byte offset where.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Error {
    kind: ErrorKind,
    offset: usize,
}

impl Error {
    pub(crate) const fn new(kind: ErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    /// What is wrong.
    pub const fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// The byte offset, in the document, of the defect.
    pub const fn offset(&self) -> usize {
        self.offset
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JSON byte {}: ", self.offset)?;
        match self.kind {
            ErrorKind::Expected(what) => write!(f, "expected {what}"),
            ErrorKind::RawControl => f.write_str("a control character in a string must be escaped"),
            ErrorKind::Escape(kind) => f.write_str(match kind {
                JsonEscapeErrorKind::Truncated => "the input ends inside an escape",
                JsonEscapeErrorKind::BadEscape => {
                    "a backslash must be followed by one of `\"`, `\\`, `/`, `b`, `f`, `n`, `r`, \
                     `t` or `u`"
                }
                JsonEscapeErrorKind::BadHex => "`\\u` must be followed by four hexadecimal digits",
                JsonEscapeErrorKind::UnpairedHigh => {
                    "a high surrogate must be followed by a `\\u` low surrogate"
                }
                JsonEscapeErrorKind::UnpairedLow => {
                    "a low surrogate has no high surrogate before it"
                }
            }),
            ErrorKind::InvalidUtf8 => f.write_str("the document is not UTF-8"),
            ErrorKind::Trailing => f.write_str("trailing data after the top-level value"),
            ErrorKind::Depth { limit } => {
                write!(f, "more than {limit} nested arrays and objects")
            }
            ErrorKind::Values { limit } => write!(f, "more than {limit} values"),
            ErrorKind::StringBytes { limit } => {
                write!(f, "a string longer than {limit} bytes")
            }
            ErrorKind::DuplicateMember => f.write_str("an object repeats a member name"),
        }
    }
}

impl std::error::Error for Error {}
