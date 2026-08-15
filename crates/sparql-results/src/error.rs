// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The crate error type.
//!
//! Serialization of a [`crate::SparqlResult`] to the W3C result formats can
//! surface structural problems (a malformed term, a format that cannot carry
//! the result kind), so every public `serialize` entry point returns
//! `Result<_, Error>` rather than panicking — library code in this crate never
//! `unwrap`/`expect`/`panic!`s on caller input. Blank-node label syntax is NOT
//! among those problems: the writers escape an out-of-alphabet label into the
//! W3C `BLANK_NODE_LABEL` alphabet instead of failing.

use std::fmt;

/// Errors produced while serializing a SPARQL result.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// A result value violated an invariant the serializer relies on (for
    /// example a triple-term predicate that is not an IRI). Carries a
    /// human-readable description of what was malformed.
    MalformedTerm(String),
    /// A format-specific egress constraint was violated in a way the caller must
    /// be told about (an unsupported result kind for the format, or an
    /// XML-unrepresentable character in a literal or IRI).
    Format(String),
    /// A caller-supplied [`crate::model::ProvenanceNamespace`] failed
    /// construction-time validation: the `prefix` is not a valid XML
    /// Namespaces `NCName`, the `prefix` collides with a reserved name
    /// (`xml`/`xmlns`, checked case-insensitively), or the `iri` is not a
    /// syntactically valid absolute IRI. See
    /// [`crate::model::ProvenanceNamespace::new`] for the exact rules.
    InvalidNamespace(String),
    /// An internal invariant failed. Used sparingly; prefer a specific variant.
    Internal(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MalformedTerm(msg) => write!(f, "malformed result term: {msg}"),
            Self::Format(msg) => write!(f, "result format error: {msg}"),
            Self::InvalidNamespace(msg) => write!(f, "invalid provenance namespace: {msg}"),
            Self::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl std::error::Error for Error {}
