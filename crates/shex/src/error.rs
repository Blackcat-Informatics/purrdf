// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Typed ShEx parse failures.
//!
//! Per the repo `no-optionality / hard-fail` doctrine, every malformed schema
//! is a typed [`ShexError`] — never a degraded fallback, never a panic. The
//! parsers are fuzz-safe: arbitrary input yields `Err`, not `panic!`.

use core::fmt;

/// Why a ShExC or ShExJ document failed to parse into a
/// [`crate::ast::Schema`].
#[derive(Clone, PartialEq, Eq)]
pub enum ShexError {
    /// The ShExC byte stream could not be tokenized. Carries a human reason
    /// and the byte offset at which the lexer gave up.
    Lex {
        /// What the lexer objected to.
        reason: String,
        /// Byte offset of the offending input.
        at: usize,
    },
    /// The ShExC token stream violates the grammar. Carries a human reason
    /// and the byte offset of the offending token (best-effort).
    Syntax {
        /// What the parser objected to.
        reason: String,
        /// Byte offset of the offending token.
        at: usize,
    },
    /// An IRI could not be resolved against the schema base (delegated to
    /// `purrdf-iri`).
    Iri {
        /// The rejected lexical form.
        lexical: String,
        /// The underlying resolution failure.
        reason: String,
    },
    /// The ShExJ document is not valid JSON or does not match the ShExJ
    /// object model.
    Shexj(String),
    /// An `IMPORT`ed schema could not be resolved by the caller-supplied
    /// import resolver (no ambient I/O — resolution is injected).
    Import(String),
    /// Two schemas in the same import closure declare the same shape label
    /// with conflicting definitions.
    ImportConflict(String),
}

impl ShexError {
    /// Construct a [`ShexError::Lex`] at a byte offset.
    pub fn lex(reason: impl Into<String>, at: usize) -> Self {
        Self::Lex {
            reason: reason.into(),
            at,
        }
    }

    /// Construct a [`ShexError::Syntax`] at a byte offset.
    pub fn syntax(reason: impl Into<String>, at: usize) -> Self {
        Self::Syntax {
            reason: reason.into(),
            at,
        }
    }

    /// Construct a [`ShexError::Shexj`] from any displayable reason.
    pub fn shexj(reason: impl Into<String>) -> Self {
        Self::Shexj(reason.into())
    }

    /// Construct a [`ShexError::Import`] for an unresolvable import IRI.
    pub fn import(iri: impl Into<String>) -> Self {
        Self::Import(iri.into())
    }

    /// Construct a [`ShexError::ImportConflict`] for a conflicting shape label.
    pub fn import_conflict(label: impl Into<String>) -> Self {
        Self::ImportConflict(label.into())
    }
}

impl fmt::Display for ShexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lex { reason, at } => write!(f, "ShExC lex error at byte {at}: {reason}"),
            Self::Syntax { reason, at } => {
                write!(f, "ShExC syntax error at byte {at}: {reason}")
            }
            Self::Iri { lexical, reason } => {
                write!(f, "invalid IRI {lexical:?}: {reason}")
            }
            Self::Shexj(reason) => write!(f, "ShExJ error: {reason}"),
            Self::Import(iri) => write!(f, "unresolved IMPORT <{iri}>"),
            Self::ImportConflict(label) => {
                write!(f, "conflicting redefinition of shape {label}")
            }
        }
    }
}

// `Debug` mirrors `Display` so test failures print the human-readable reason
// rather than a struct dump (matches the `purrdf-iri`/`purrdf-xsd` convention).
impl fmt::Debug for ShexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl std::error::Error for ShexError {}

/// Convenience alias for fallible ShEx parse operations.
pub type Result<T> = core::result::Result<T, ShexError>;
