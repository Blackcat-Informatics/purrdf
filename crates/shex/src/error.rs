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
    /// import resolver (no ambient I/O — resolution is injected). Carries the
    /// import IRI and the concrete cause the resolver reported, so a read or
    /// parse failure behind an import surfaces its real reason rather than a
    /// vague "unresolved import".
    Import {
        /// The import IRI that failed to resolve.
        iri: String,
        /// The concrete failure the resolver surfaced for `iri`.
        cause: Box<Self>,
    },
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

    /// Construct a [`ShexError::Import`] for an unresolvable import IRI,
    /// preserving the concrete `cause` the resolver reported.
    pub fn import(iri: impl Into<String>, cause: Self) -> Self {
        Self::Import {
            iri: iri.into(),
            cause: Box::new(cause),
        }
    }

    /// Construct a [`ShexError::ImportConflict`] for a conflicting shape label.
    pub fn import_conflict(label: impl Into<String>) -> Self {
        Self::ImportConflict(label.into())
    }

    /// The byte offset the failure was reported at, for the position-bearing
    /// variants ([`Lex`](Self::Lex)/[`Syntax`](Self::Syntax)). `None` for the
    /// others — in particular [`Import`](Self::Import), whose offset (if any)
    /// belongs to a *different* source document than the one being parsed.
    #[must_use]
    pub fn byte_offset(&self) -> Option<usize> {
        match self {
            Self::Lex { at, .. } | Self::Syntax { at, .. } => Some(*at),
            Self::Iri { .. } | Self::Shexj(_) | Self::Import { .. } | Self::ImportConflict(_) => {
                None
            }
        }
    }

    /// Resolve this error's byte offset to a 1-based source [`Position`] against
    /// the original ShExC text.
    ///
    /// The lexer keeps a cheap byte offset on the happy path; the line/column
    /// table is built here, on the error path only. `None` for variants without
    /// a byte offset in this document.
    ///
    /// [`Position`]: purrdf_iri::Position
    #[must_use]
    pub fn locate(&self, src: &str) -> Option<purrdf_iri::Position> {
        self.byte_offset()
            .map(|at| purrdf_iri::LineIndex::new(src).locate(src, at))
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
            Self::Import { iri, cause } => write!(f, "unresolved IMPORT <{iri}>: {cause}"),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_offset_only_for_positional_variants() {
        assert_eq!(ShexError::lex("x", 5).byte_offset(), Some(5));
        assert_eq!(ShexError::syntax("y", 9).byte_offset(), Some(9));
        assert_eq!(ShexError::shexj("bad").byte_offset(), None);
        assert_eq!(
            ShexError::import("http://example.org/s", ShexError::syntax("z", 1)).byte_offset(),
            None
        );
    }

    #[test]
    fn locate_resolves_line_and_column() {
        let src = "PREFIX ex: <http://example.org/>\n<S> {\n  ex:p .\n}";
        let at = src.find("ex:p").unwrap();
        let pos = ShexError::syntax("unexpected", at).locate(src).unwrap();
        assert_eq!((pos.line, pos.column), (3, 3));
        assert!(ShexError::shexj("bad").locate(src).is_none());
    }
}
