// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Typed parse/resolution failures.
//!
//! Per the repo `no-optionality / hard-fail` doctrine, every malformed input is a
//! typed [`IriError`] — never a degraded fallback, never a silent default. The
//! variants are deliberately specific so callers (and conformance vectors) can
//! assert *why* a string was rejected, not merely that it was.

use core::fmt;

/// Why an IRI/URI string (or a reference-resolution / CURIE operation) failed.
#[derive(Clone, PartialEq, Eq)]
pub enum IriError {
    /// The string is empty where a non-empty IRI/URI was required.
    Empty,
    /// A scheme was required (e.g. resolving against a base that has no scheme,
    /// or validating an absolute URI) but none was present.
    MissingScheme,
    /// The scheme is present but malformed: it must match `ALPHA *( ALPHA / DIGIT
    /// / "+" / "-" / "." )` (RFC-3986 §3.1). Carries the offending scheme text.
    BadScheme(String),
    /// A percent-encoding triplet (`%` `HEXDIG` `HEXDIG`) is truncated or contains
    /// a non-hex digit. Carries the byte offset of the offending `%`.
    BadPercentEncoding(usize),
    /// A character outside the permitted grammar appeared in a component. Carries
    /// the offending `char` and its byte offset.
    DisallowedChar(char, usize),
    /// The authority/host component is malformed (e.g. an unterminated IPv6
    /// literal `[...]`). Carries a short reason.
    BadAuthority(String),
    /// Reference resolution was asked to produce an absolute IRI from a base that
    /// is itself not absolute (has no scheme) — RFC-3986 §5.1 requires an absolute
    /// base. Carries the base text.
    NonAbsoluteBase(String),
}

impl fmt::Display for IriError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("empty IRI/URI string"),
            Self::MissingScheme => f.write_str("missing scheme"),
            Self::BadScheme(s) => write!(f, "malformed scheme: {s:?}"),
            Self::BadPercentEncoding(at) => {
                write!(f, "malformed percent-encoding at byte {at}")
            }
            Self::DisallowedChar(c, at) => {
                write!(f, "disallowed character {c:?} at byte {at}")
            }
            Self::BadAuthority(why) => write!(f, "malformed authority: {why}"),
            Self::NonAbsoluteBase(b) => {
                write!(f, "base IRI is not absolute (no scheme): {b:?}")
            }
        }
    }
}

// `Debug` mirrors `Display` so test failures print the human-readable reason
// rather than a struct dump (matches the `purrdf-xsd` `XsdError` convention).
impl fmt::Debug for IriError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl std::error::Error for IriError {}

/// Convenience alias for fallible IRI operations.
pub type Result<T> = core::result::Result<T, IriError>;
