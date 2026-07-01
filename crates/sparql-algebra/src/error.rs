// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Typed SPARQL parse failures.
//!
//! Per the repo `no-optionality / hard-fail` doctrine, every malformed or
//! out-of-scope query is a typed [`ParseError`] — never a degraded fallback,
//! never a silent default. The variants are deliberately specific so callers
//! (and conformance vectors) can assert *why* a query was rejected:
//!
//! * [`ParseError::Lex`] — the byte stream could not be tokenized.
//! * [`ParseError::Syntax`] — the token stream violates the SPARQL grammar.
//! * [`ParseError::Unsupported`] — the query is well-formed SPARQL but uses a
//!   construct outside this crate's in-scope subset (purrdf S5 scope). It is a
//!   hard error, NOT a parse-it-anyway: the downstream evaluator (S6 #912) must
//!   never be handed a partially-understood algebra.
//! * [`ParseError::Iri`] — an IRI/CURIE in term position failed RFC-3987
//!   validation (delegated to `purrdf-iri`).

use core::fmt;

/// Why a SPARQL query string failed to parse into the algebra.
#[derive(Clone, PartialEq, Eq)]
pub enum ParseError {
    /// Tokenization failed. Carries a human reason and the byte offset at which
    /// the lexer gave up.
    Lex { reason: String, at: usize },
    /// The token stream violated the grammar. Carries a human reason and the
    /// byte offset of the offending token (best-effort).
    Syntax { reason: String, at: usize },
    /// Well-formed SPARQL that uses a construct outside this crate's in-scope
    /// subset. Carries the name of the unsupported feature.
    Unsupported(String),
    /// An IRI/CURIE in term position is not a valid RFC-3987 IRI. Carries the
    /// rejected lexical form and the underlying reason.
    Iri { lexical: String, reason: String },
}

impl ParseError {
    /// Construct a [`ParseError::Unsupported`] from any displayable feature name.
    pub fn unsupported(feature: impl Into<String>) -> Self {
        ParseError::Unsupported(feature.into())
    }

    /// Construct a [`ParseError::Syntax`] at a byte offset.
    pub fn syntax(reason: impl Into<String>, at: usize) -> Self {
        ParseError::Syntax {
            reason: reason.into(),
            at,
        }
    }

    /// Construct a [`ParseError::Lex`] at a byte offset.
    pub fn lex(reason: impl Into<String>, at: usize) -> Self {
        ParseError::Lex {
            reason: reason.into(),
            at,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Lex { reason, at } => write!(f, "SPARQL lex error at byte {at}: {reason}"),
            ParseError::Syntax { reason, at } => {
                write!(f, "SPARQL syntax error at byte {at}: {reason}")
            }
            ParseError::Unsupported(feature) => {
                write!(
                    f,
                    "unsupported SPARQL construct (purrdf S5 scope): {feature}"
                )
            }
            ParseError::Iri { lexical, reason } => {
                write!(f, "invalid IRI {lexical:?} in term position: {reason}")
            }
        }
    }
}

// `Debug` mirrors `Display` so test failures print the human-readable reason
// rather than a struct dump (matches the `purrdf-iri`/`purrdf-xsd` convention).
impl fmt::Debug for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl std::error::Error for ParseError {}

/// Convenience alias for fallible SPARQL parse operations.
pub type Result<T> = core::result::Result<T, ParseError>;
