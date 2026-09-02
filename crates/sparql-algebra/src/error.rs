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
//!   hard error, NOT a parse-it-anyway: the downstream evaluator (S6) must
//!   never be handed a partially-understood algebra.
//! * [`ParseError::Iri`] — an IRI/CURIE in term position failed RFC-3987
//!   validation (delegated to `purrdf-iri`).
//! * [`ParseError::CdtArity`] — a SEP-0009 composite-datatype function was
//!   called with a number of arguments its spec-fixed signature does not admit.

use core::fmt;

/// Why a SPARQL query string failed to parse into the algebra.
#[derive(Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseError {
    /// Tokenization failed. Carries a human reason and the byte offset at which
    /// the lexer gave up.
    Lex {
        /// Human-readable description of why tokenization failed.
        reason: String,
        /// Byte offset into the query string at which the lexer gave up.
        at: usize,
    },
    /// The token stream violated the grammar. Carries a human reason and the
    /// byte offset of the offending token (best-effort).
    Syntax {
        /// Human-readable description of the grammar violation.
        reason: String,
        /// Byte offset of the offending token (best-effort).
        at: usize,
    },
    /// Well-formed SPARQL that uses a construct outside this crate's in-scope
    /// subset. Carries the name of the unsupported feature.
    Unsupported(String),
    /// An IRI/CURIE in term position is not a valid RFC-3987 IRI. Carries the
    /// rejected lexical form and the underlying reason.
    Iri {
        /// The rejected lexical form.
        lexical: String,
        /// The underlying validation failure reported by `purrdf-iri`.
        reason: String,
    },
    /// A SEP-0009 composite-datatype function (`cdt:List`, `cdt:subseq`, …) was
    /// called with an argument count its signature does not admit.
    ///
    /// Its own variant rather than a [`Self::Syntax`] because it is a **static**
    /// error with a decidable cause: SPARQL has no overloading on argument count,
    /// so a wrong-arity call can never evaluate to anything — not even to an
    /// expression error — and a consumer (or a conformance vector) that wants to
    /// assert *this* rejection should not have to match on prose. `cdt:Map` is the
    /// case that most needs saying out loud: its arguments are key/value **pairs**,
    /// so an odd count is refused here rather than silently dropping the trailing
    /// key.
    CdtArity {
        /// The function IRI the call named.
        iri: String,
        /// The signature the function admits, spelled for a human
        /// (`"an even number of arguments"`, `"2 or 3 arguments"`, …).
        expected: String,
        /// The number of arguments the call actually supplied.
        found: usize,
        /// Byte offset of the offending call (best-effort).
        at: usize,
    },
}

impl ParseError {
    /// Construct a [`ParseError::Unsupported`] from any displayable feature name.
    pub fn unsupported(feature: impl Into<String>) -> Self {
        Self::Unsupported(feature.into())
    }

    /// Construct a [`ParseError::Syntax`] at a byte offset.
    pub fn syntax(reason: impl Into<String>, at: usize) -> Self {
        Self::Syntax {
            reason: reason.into(),
            at,
        }
    }

    /// Construct a [`ParseError::Lex`] at a byte offset.
    pub fn lex(reason: impl Into<String>, at: usize) -> Self {
        Self::Lex {
            reason: reason.into(),
            at,
        }
    }

    /// The byte offset the failure was reported at, for the position-bearing
    /// variants ([`Lex`](Self::Lex)/[`Syntax`](Self::Syntax)/[`CdtArity`](Self::CdtArity)).
    /// `None` for [`Unsupported`](Self::Unsupported)/[`Iri`](Self::Iri), which are
    /// not tied to a single source position.
    #[must_use]
    pub fn byte_offset(&self) -> Option<usize> {
        match self {
            Self::Lex { at, .. } | Self::Syntax { at, .. } | Self::CdtArity { at, .. } => Some(*at),
            Self::Unsupported(_) | Self::Iri { .. } => None,
        }
    }

    /// Resolve this error's byte offset to a 1-based source [`Position`] against
    /// the original query text.
    ///
    /// This is the source-tracing seam: the lexer keeps a cheap byte offset on
    /// the happy path, and the line/column table is built here, on the error
    /// path only. `None` for variants without a byte offset.
    ///
    /// [`Position`]: purrdf_iri::Position
    #[must_use]
    pub fn locate(&self, src: &str) -> Option<purrdf_iri::Position> {
        self.byte_offset()
            .map(|at| purrdf_iri::LineIndex::new(src).locate(src, at))
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lex { reason, at } => write!(f, "SPARQL lex error at byte {at}: {reason}"),
            Self::Syntax { reason, at } => {
                write!(f, "SPARQL syntax error at byte {at}: {reason}")
            }
            Self::Unsupported(feature) => {
                write!(
                    f,
                    "unsupported SPARQL construct (purrdf S5 scope): {feature}"
                )
            }
            Self::Iri { lexical, reason } => {
                write!(f, "invalid IRI {lexical:?} in term position: {reason}")
            }
            Self::CdtArity {
                iri,
                expected,
                found,
                at,
            } => write!(
                f,
                "SPARQL syntax error at byte {at}: <{iri}> takes {expected}, not {found}"
            ),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_offset_only_for_positional_variants() {
        assert_eq!(ParseError::lex("x", 7).byte_offset(), Some(7));
        assert_eq!(ParseError::syntax("y", 3).byte_offset(), Some(3));
        assert_eq!(ParseError::unsupported("SERVICE").byte_offset(), None);
        assert_eq!(
            ParseError::Iri {
                lexical: "::".into(),
                reason: "bad".into()
            }
            .byte_offset(),
            None
        );
    }

    #[test]
    fn locate_resolves_line_and_column() {
        // Offset points at the 'x' on the third line.
        let src = "SELECT *\nWHERE {\n  x }";
        let at = src.find('x').unwrap();
        let pos = ParseError::syntax("unexpected token", at)
            .locate(src)
            .unwrap();
        assert_eq!((pos.line, pos.column), (3, 3));
        assert_eq!(pos.byte_offset, at);
        assert!(ParseError::unsupported("SERVICE").locate(src).is_none());
    }
}
