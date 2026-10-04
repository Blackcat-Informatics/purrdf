// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
//!   construct outside this crate's in-scope subset. It is a
//!   hard error, NOT a parse-it-anyway: the downstream evaluator must
//!   never be handed a partially-understood algebra.
//! * [`ParseError::Iri`] — an IRI/CURIE in term position failed RFC-3987
//!   validation (delegated to `purrdf-iri`).
//! * [`ParseError::CdtArity`] — a SEP-0009 composite-datatype function was
//!   called with a number of arguments its spec-fixed signature does not admit.
//!
//! How deeply a request nests is never a reason: the parser holds what encloses the
//! cursor on heap-allocated stacks, so nesting is bounded by memory alone.

use core::fmt;
use purrdf_lex::diagnostic::{DiagnosticParameter, DiagnosticPresentation, DiagnosticValue};

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

purrdf_lex::constructors! {
    impl ParseError {
        /// Construct a [`ParseError::Unsupported`] from any displayable feature name.
        pub fn unsupported(feature) -> Self::Unsupported;

        /// Construct a [`ParseError::Syntax`] at a byte offset.
        pub fn syntax(reason, at: usize) -> Self::Syntax { .. };

        /// Construct a [`ParseError::Lex`] at a byte offset.
        pub fn lex(reason, at: usize) -> Self::Lex { .. };
    }
}

impl ParseError {
    /// The original parse condition, with stable variant identity and exact typed
    /// fields. Hosts can distinguish a syntax rejection from lexical, IRI,
    /// unsupported-construct and arity failures without interpreting English.
    #[must_use]
    #[expect(
        clippy::literal_string_with_formatting_args,
        reason = "these templates are validated and rendered by DiagnosticPresentation"
    )]
    pub fn presentation(&self) -> DiagnosticPresentation {
        use DiagnosticValue::{Text, Unsigned};
        let parameter = |name, value| DiagnosticParameter::new(name, value);
        let (identity, template, parameters) = match self {
            Self::Lex { reason, at } => (
                "sparql-parse-lex",
                "SPARQL lex error at byte {at}: {reason}",
                vec![
                    parameter("at", Unsigned(*at as u64)),
                    parameter("reason", Text(reason.clone())),
                ],
            ),
            Self::Syntax { reason, at } => (
                "sparql-parse-syntax",
                "SPARQL syntax error at byte {at}: {reason}",
                vec![
                    parameter("at", Unsigned(*at as u64)),
                    parameter("reason", Text(reason.clone())),
                ],
            ),
            Self::Unsupported(feature) => (
                "sparql-parse-unsupported",
                "unsupported SPARQL construct: {feature} is outside the SPARQL 1.2 \
                 query language this processor implements",
                vec![parameter("feature", Text(feature.clone()))],
            ),
            Self::Iri { lexical, reason } => (
                "sparql-parse-iri",
                "invalid IRI {lexical:?} in term position: {reason}",
                vec![
                    parameter("lexical", Text(lexical.clone())),
                    parameter("reason", Text(reason.clone())),
                ],
            ),
            Self::CdtArity {
                iri,
                expected,
                found,
                at,
            } => (
                "sparql-parse-cdt-arity",
                "SPARQL syntax error at byte {at}: <{iri}> takes {expected}, not {found}",
                vec![
                    parameter("at", Unsigned(*at as u64)),
                    parameter("iri", Text(iri.clone())),
                    parameter("expected", Text(expected.clone())),
                    parameter("found", Unsigned(*found as u64)),
                ],
            ),
        };
        // Unreachable refusal: `DiagnosticPresentation::new` judges only the identity,
        // the template and the parameter names, and every one of those is a literal
        // fixed per arm above. Argument values are spliced into the rendering and never
        // re-read as template text, so no field value can make an arm fail.
        // `presentation_preserves_variant_identity_fields_and_english` constructs every
        // arm, and `presentation_is_independent_of_field_values` feeds each one
        // template-like text, so a drifted arm fails the test suite, not a caller.
        DiagnosticPresentation::new(identity, template, parameters)
            .expect("parse error templates and their typed argument sets agree")
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
        f.write_str(self.presentation().english())
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
    fn presentation_preserves_variant_identity_fields_and_english() {
        let errors = [
            (
                ParseError::lex("unexpected character", 7),
                "sparql-parse-lex",
                "SPARQL lex error at byte 7: unexpected character",
            ),
            (
                ParseError::syntax("unexpected token", 7),
                "sparql-parse-syntax",
                "SPARQL syntax error at byte 7: unexpected token",
            ),
            (
                ParseError::unsupported("a feature"),
                "sparql-parse-unsupported",
                "unsupported SPARQL construct: a feature is outside the SPARQL 1.2 query language this processor implements",
            ),
            (
                ParseError::Iri {
                    lexical: "quoted\"\\漢".into(),
                    reason: "invalid".into(),
                },
                "sparql-parse-iri",
                "invalid IRI \"quoted\\\"\\\\漢\" in term position: invalid",
            ),
            (
                ParseError::CdtArity {
                    iri: "http://example.org/function".into(),
                    expected: "2 arguments".into(),
                    found: 1,
                    at: 7,
                },
                "sparql-parse-cdt-arity",
                "SPARQL syntax error at byte 7: <http://example.org/function> takes 2 arguments, not 1",
            ),
        ];
        for (error, identity, english) in errors {
            let presentation = error.presentation();
            assert_eq!(presentation.message_id(), identity);
            assert_eq!(presentation.english(), english);
            assert_eq!(error.to_string(), english);
            let json = presentation.to_json();
            assert_eq!(
                json.get("messageId")
                    .and_then(purrdf_lex::json::Value::as_str),
                Some(identity)
            );
        }
        let syntax = ParseError::syntax("scope violation", 7).presentation();
        assert_eq!(
            syntax.parameters()[0].value(),
            &DiagnosticValue::Unsigned(7)
        );
        assert_eq!(
            syntax.parameters()[1].value(),
            &DiagnosticValue::Text("scope violation".into())
        );
        if let Ok(at) = usize::try_from(9_007_199_254_740_993_u64) {
            let record = ParseError::syntax("scope violation", at)
                .presentation()
                .to_json();
            assert_eq!(
                record
                    .get("parameters")
                    .unwrap()
                    .get("at")
                    .unwrap()
                    .get("value")
                    .unwrap()
                    .as_str(),
                Some("9007199254740993")
            );
        }
    }

    /// Field values never reach template validation: brace-, colon- and
    /// quote-bearing text in every textual field renders verbatim (or `Debug`-quoted
    /// where the template asks), and extreme offsets render exactly, in every arm.
    #[test]
    fn presentation_is_independent_of_field_values() {
        let hostile = "{at} }{ {{x}} {reason:?} \"\\ \u{0}";
        let errors = [
            ParseError::lex(hostile, usize::MAX),
            ParseError::syntax(hostile, 0),
            ParseError::unsupported(hostile),
            ParseError::Iri {
                lexical: hostile.into(),
                reason: hostile.into(),
            },
            ParseError::CdtArity {
                iri: hostile.into(),
                expected: hostile.into(),
                found: usize::MAX,
                at: usize::MAX,
            },
        ];
        let expected = [
            format!("SPARQL lex error at byte {}: {hostile}", usize::MAX),
            format!("SPARQL syntax error at byte 0: {hostile}"),
            format!(
                "unsupported SPARQL construct: {hostile} is outside the SPARQL 1.2 \
                 query language this processor implements"
            ),
            format!("invalid IRI {hostile:?} in term position: {hostile}"),
            format!(
                "SPARQL syntax error at byte {max}: <{hostile}> takes {hostile}, not {max}",
                max = usize::MAX
            ),
        ];
        for (error, english) in errors.iter().zip(expected) {
            assert_eq!(error.presentation().english(), english);
            assert_eq!(error.to_string(), english);
        }
    }

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

    /// The text reaches users (a SHACL `sh:select` body surfaces it), so it names the
    /// construct and the language, never an internal work label.
    #[test]
    fn unsupported_names_the_construct_in_the_languages_terms() {
        let text = ParseError::unsupported("solution modifiers on ASK").to_string();
        assert_eq!(
            text,
            "unsupported SPARQL construct: solution modifiers on ASK is outside the SPARQL \
             1.2 query language this processor implements"
        );
        assert!(!text.contains("purrdf"), "{text}");
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
