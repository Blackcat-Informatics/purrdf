// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The crate error type.
//!
//! Serialization of a well-formed [`crate::SparqlResult`] to the W3C result
//! formats is infallible, but the graph (CONSTRUCT) path and future formats can
//! surface structural problems. The type is intentionally kept so the public
//! `serialize` entry points (Tasks 2–3) can return `Result<_, Error>` rather
//! than panicking — library code in this crate never `unwrap`/`expect`/`panic!`s
//! on caller input.

use std::fmt;

/// Errors produced while serializing a SPARQL result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// A result value violated an invariant the serializer relies on (for
    /// example a triple-term predicate that is not an IRI). Carries a
    /// human-readable description of what was malformed.
    MalformedTerm(String),
    /// A format-specific egress constraint was violated in a way the caller must
    /// be told about (reserved for the JSON/XML/CSV/TSV writers in later tasks).
    Format(String),
    /// An internal invariant failed. Used sparingly; prefer a specific variant.
    Internal(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::MalformedTerm(msg) => write!(f, "malformed result term: {msg}"),
            Error::Format(msg) => write!(f, "result format error: {msg}"),
            Error::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl std::error::Error for Error {}
