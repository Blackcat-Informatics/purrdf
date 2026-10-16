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

/// The original reason an IRI cannot occupy a SPARQL term position.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum IriTermReason {
    /// An original IRI validator refusal, without a rendered copy.
    Validation {
        /// The exact native failure and its original owned payload.
        cause: purrdf_iri::IriError,
        /// Whether the existing human spelling leads with the native code.
        include_code: bool,
    },
    /// The reference is valid but has no scheme in an absolute term position.
    Relative,
}

impl IriTermReason {
    /// The original validator failure, absent for a valid relative reference.
    #[must_use]
    pub const fn validation_error(&self) -> Option<&purrdf_iri::IriError> {
        match self {
            Self::Validation {
                cause,
                include_code: _,
            } => Some(cause),
            Self::Relative => None,
        }
    }
}

impl purrdf_hash::error::StructuredError for IriTermReason {
    purrdf_hash::error_visit! {
        Validation { cause, include_code } => "sparql-iri-validation" {
            "cause" => Cause(cause), "include_code" => Boolean(*include_code),
        },
        Relative => "sparql-iri-relative-term" {},
    }
}

impl fmt::Display for IriTermReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation {
                cause,
                include_code,
            } => {
                if *include_code {
                    write!(f, "{}: ", cause.diagnostic_code())?;
                }
                fmt::Display::fmt(cause, f)
            }
            Self::Relative => f.write_str("relative IRI reference in term position (no scheme)"),
        }
    }
}

impl std::error::Error for IriTermReason {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.validation_error().map(|cause| cause as _)
    }
}

/// Why a SPARQL query string failed to parse into the algebra.
#[derive(Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseError {
    /// Physical storage refusal, distinct from the query language.
    Storage(purrdf_lex::allocation::StorageError),
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
        reason: IriTermReason,
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

impl purrdf_hash::error::StructuredError for ParseError {
    purrdf_hash::error_visit! {
        Storage(error) => "sparql-parse-storage" { "cause" => Cause(error) },
        Lex { reason, at } => "sparql-parse-lex" {
            "reason" => Text(reason), "at" => ByteCount(*at as u128),
        },
        Syntax { reason, at } => "sparql-parse-syntax" {
            "reason" => Text(reason), "at" => ByteCount(*at as u128),
        },
        Unsupported(feature) => "sparql-parse-unsupported" { "feature" => Text(feature) },
        Iri { lexical, reason } => "sparql-parse-iri" {
            "lexical" => Iri(lexical), "reason" => Cause(reason),
        },
        CdtArity { iri, expected, found, at } => "sparql-parse-cdt-arity" {
            "iri" => Iri(iri), "expected" => Text(expected),
            "found" => Unsigned(*found as u128), "at" => ByteCount(*at as u128),
        },
    }
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
    ///
    /// An [`Iri`](Self::Iri) failure that `purrdf-iri` refused carries that refusal's
    /// own typed presentation as [`detail`](DiagnosticPresentation::detail) (for
    /// example `iri-bad-percent-encoding` with its unsigned byte `offset`, or
    /// `iri-relative-no-base`), so a host reads the IRI condition without parsing
    /// `reason`. Detail uses the original validator failure directly, without
    /// reparsing the lexical form or matching human messages. A valid relative
    /// reference has its own typed reason and no validator detail.
    #[must_use]
    pub fn presentation(&self) -> DiagnosticPresentation {
        let mut resident = purrdf_lex::allocation::Resident;
        self.presentation_with_memory(&mut purrdf_lex::allocation::Memory::new(&mut resident))
            .expect("resident parse presentation allocation")
    }

    /// Preserve exact typed parse and IRI conditions using original native storage.
    ///
    /// # Errors
    /// Returns physical refusal; a storage parse failure remains operational.
    pub fn try_presentation_with_memory<S: purrdf_lex::allocation::Admission + ?Sized>(
        &self,
        memory: &mut purrdf_lex::allocation::Memory<'_, S>,
    ) -> core::result::Result<DiagnosticPresentation, purrdf_lex::allocation::StorageError> {
        if let Self::Storage(error) = self {
            return Err(*error);
        }
        self.presentation_with_memory(memory)
    }

    fn presentation_with_memory<S: purrdf_lex::allocation::Admission + ?Sized>(
        &self,
        memory: &mut purrdf_lex::allocation::Memory<'_, S>,
    ) -> core::result::Result<DiagnosticPresentation, purrdf_lex::allocation::StorageError> {
        let presentation = self.own_presentation_with_memory(memory)?;
        let Self::Iri { lexical: _, reason } = self else {
            return Ok(presentation);
        };
        match reason.validation_error() {
            Some(cause) => presentation
                .try_with_detail_with_memory(cause.try_presentation_with_memory(memory)?, memory)
                .map_err(|error| error.fixed_template_storage()),
            None => Ok(presentation),
        }
    }

    #[expect(
        clippy::literal_string_with_formatting_args,
        reason = "these templates are validated and rendered by DiagnosticPresentation"
    )]
    fn own_presentation_with_memory<S: purrdf_lex::allocation::Admission + ?Sized>(
        &self,
        memory: &mut purrdf_lex::allocation::Memory<'_, S>,
    ) -> core::result::Result<DiagnosticPresentation, purrdf_lex::allocation::StorageError> {
        use DiagnosticValue::{Text, Unsigned};

        let (identity, template, parameters) = match self {
            Self::Storage(error) => ("sparql-parse-storage", "SPARQL storage error: {reason}", {
                let native_values = [DiagnosticParameter::try_new_with_memory(
                    "reason",
                    Text(memory.format(error)?),
                    memory,
                )?];
                memory.collect(native_values)
            }?),
            Self::Lex { reason, at } => (
                "sparql-parse-lex",
                "SPARQL lex error at byte {at}: {reason}",
                {
                    let native_values = [
                        DiagnosticParameter::try_new_with_memory(
                            "at",
                            Unsigned(*at as u64),
                            memory,
                        )?,
                        DiagnosticParameter::try_new_with_memory(
                            "reason",
                            Text(memory.string(reason)?),
                            memory,
                        )?,
                    ];
                    memory.collect(native_values)
                }?,
            ),
            Self::Syntax { reason, at } => (
                "sparql-parse-syntax",
                "SPARQL syntax error at byte {at}: {reason}",
                {
                    let native_values = [
                        DiagnosticParameter::try_new_with_memory(
                            "at",
                            Unsigned(*at as u64),
                            memory,
                        )?,
                        DiagnosticParameter::try_new_with_memory(
                            "reason",
                            Text(memory.string(reason)?),
                            memory,
                        )?,
                    ];
                    memory.collect(native_values)
                }?,
            ),
            Self::Unsupported(feature) => (
                "sparql-parse-unsupported",
                "unsupported SPARQL construct: {feature} is outside the SPARQL 1.2 \
                 query language this processor implements",
                {
                    let native_values = [DiagnosticParameter::try_new_with_memory(
                        "feature",
                        Text(memory.string(feature)?),
                        memory,
                    )?];
                    memory.collect(native_values)
                }?,
            ),
            Self::Iri { lexical, reason } => (
                "sparql-parse-iri",
                "invalid IRI {lexical:?} in term position: {reason}",
                {
                    let native_values = [
                        DiagnosticParameter::try_new_with_memory(
                            "lexical",
                            Text(memory.string(lexical)?),
                            memory,
                        )?,
                        DiagnosticParameter::try_new_with_memory(
                            "reason",
                            Text(memory.format(reason)?),
                            memory,
                        )?,
                    ];
                    memory.collect(native_values)
                }?,
            ),
            Self::CdtArity {
                iri,
                expected,
                found,
                at,
            } => (
                "sparql-parse-cdt-arity",
                "SPARQL syntax error at byte {at}: <{iri}> takes {expected}, not {found}",
                {
                    let native_values = [
                        DiagnosticParameter::try_new_with_memory(
                            "at",
                            Unsigned(*at as u64),
                            memory,
                        )?,
                        DiagnosticParameter::try_new_with_memory(
                            "iri",
                            Text(memory.string(iri)?),
                            memory,
                        )?,
                        DiagnosticParameter::try_new_with_memory(
                            "expected",
                            Text(memory.string(expected)?),
                            memory,
                        )?,
                        DiagnosticParameter::try_new_with_memory(
                            "found",
                            Unsigned(*found as u64),
                            memory,
                        )?,
                    ];
                    memory.collect(native_values)
                }?,
            ),
        };
        // Unreachable refusal: `DiagnosticPresentation::new` judges only the identity,
        // the template and the parameter names, and every one of those is a literal
        // fixed per arm above. Argument values are spliced into the rendering and never
        // re-read as template text, so no field value can make an arm fail.
        // `presentation_preserves_variant_identity_fields_and_english` constructs every
        // arm, and `presentation_is_independent_of_field_values` feeds each one
        // template-like text, so a drifted arm fails the test suite, not a caller.
        DiagnosticPresentation::try_new_with_memory(identity, template, parameters, memory)
            .map_err(|error| error.fixed_template_storage())
    }

    /// Capacity of the original raw error text; intrinsic leaf owners are separate.
    /// Used only after parser scratch has been destroyed to release its original grant.
    #[must_use]
    pub fn owned_text_bytes(&self) -> Option<usize> {
        match self {
            Self::Storage(_) => Some(0),
            Self::Lex { reason, .. } | Self::Syntax { reason, .. } | Self::Unsupported(reason) => {
                Some(reason.capacity())
            }
            Self::Iri { lexical, reason } => lexical.capacity().checked_add(
                reason
                    .validation_error()
                    .map_or(0, purrdf_iri::IriError::owned_text_bytes),
            ),
            Self::CdtArity { iri, expected, .. } => iri.capacity().checked_add(expected.capacity()),
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
            Self::Unsupported(_) | Self::Iri { .. } | Self::Storage(_) => None,
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

/// Borrowed term-position IRI diagnostic fields, without lexical or reason copies.
/// The caller retains any native reason's original owner while this view renders.
#[derive(Clone, Copy)]
pub struct IriTermDisplay<'a> {
    lexical: &'a str,
    reason: &'a dyn fmt::Display,
}

impl<'a> IriTermDisplay<'a> {
    /// Borrow the original lexical spelling and its precise refusal reason.
    #[must_use]
    pub fn new(lexical: &'a str, reason: &'a dyn fmt::Display) -> Self {
        Self { lexical, reason }
    }
}

impl fmt::Debug for IriTermDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IriTermDisplay")
            .field("lexical", &self.lexical)
            .finish_non_exhaustive()
    }
}

impl fmt::Display for IriTermDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid IRI {lexical:?} in term position: {reason}",
            lexical = self.lexical,
            reason = self.reason
        )
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Storage(error) => write!(f, "SPARQL storage error: {error}"),
            Self::Lex { reason, at } => write!(f, "SPARQL lex error at byte {at}: {reason}"),
            Self::Syntax { reason, at } => write!(f, "SPARQL syntax error at byte {at}: {reason}"),
            Self::Unsupported(feature) => write!(
                f,
                "unsupported SPARQL construct: {feature} is outside the SPARQL 1.2 query language this processor implements"
            ),
            Self::Iri { lexical, reason } => {
                fmt::Display::fmt(&IriTermDisplay::new(lexical, reason), f)
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

purrdf_lex::variant_from!(ParseError { Storage(purrdf_lex::allocation::StorageError) });

// `Debug` mirrors `Display` so test failures print the human-readable reason
// rather than a struct dump (matches the `purrdf-iri`/`purrdf-xsd` convention).
impl fmt::Debug for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Storage(error) => Some(error),
            Self::Iri { reason, lexical: _ } => Some(reason),
            Self::Lex { reason: _, at: _ }
            | Self::Syntax { reason: _, at: _ }
            | Self::Unsupported(_)
            | Self::CdtArity {
                iri: _,
                expected: _,
                found: _,
                at: _,
            } => None,
        }
    }
}

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
                    reason: IriTermReason::Relative,
                },
                "sparql-parse-iri",
                "invalid IRI \"quoted\\\"\\\\漢\" in term position: relative IRI reference in term position (no scheme)",
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
                reason: IriTermReason::Validation {
                    cause: purrdf_iri::IriError::BadScheme(hostile.into()),
                    include_code: false,
                },
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
            format!(
                "invalid IRI {hostile:?} in term position: {}",
                purrdf_iri::IriError::BadScheme(hostile.into())
            ),
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

    /// An IRI refusal carries the `purrdf-iri` condition as typed detail, from every
    /// path that raises one (term resolution, a `BASE` directive, a caller base, an
    /// AST constructor), while the English stays exactly the parse error's own.
    #[test]
    fn iri_failures_carry_the_typed_iri_cause_as_detail() {
        use crate::{NamedNode, SparqlParser};
        let parser = SparqlParser::new();
        let cases = [
            (
                parser
                    .parse_query("SELECT * WHERE { <http://example.org/%zz> ?p ?o }")
                    .unwrap_err(),
                "iri-bad-percent-encoding",
                Some(("offset", DiagnosticValue::Unsigned(19))),
            ),
            (
                parser
                    .parse_query("SELECT * WHERE { <relative> ?p ?o }")
                    .unwrap_err(),
                "iri-relative-no-base",
                Some(("reference", DiagnosticValue::Text("relative".into()))),
            ),
            (
                parser
                    .parse_query("BASE <http://example.org/%g0> SELECT * WHERE { ?s ?p ?o }")
                    .unwrap_err(),
                "iri-bad-percent-encoding",
                Some(("offset", DiagnosticValue::Unsigned(19))),
            ),
            (
                SparqlParser::new()
                    .with_base_iri("relative/")
                    .parse_query("SELECT * WHERE { ?s ?p ?o }")
                    .unwrap_err(),
                "iri-non-absolute-base",
                None,
            ),
            (
                NamedNode::new("http://example.org/a b").unwrap_err(),
                "iri-disallowed-char",
                Some(("offset", DiagnosticValue::Unsigned(20))),
            ),
        ];
        for (error, identity, parameter) in cases {
            let ParseError::Iri { reason, .. } = &error else {
                panic!("an IRI refusal, got {error:?}");
            };
            let presentation = error.presentation();
            assert_eq!(presentation.message_id(), "sparql-parse-iri");
            assert_eq!(presentation.english(), error.to_string());
            let detail = presentation.detail().expect("typed IRI cause");
            assert_eq!(detail.message_id(), identity, "{error}");
            assert!(reason.to_string().contains(detail.english()), "{reason}");
            if let Some((name, value)) = parameter {
                let found = detail
                    .parameters()
                    .iter()
                    .find(|candidate| candidate.name() == name)
                    .expect("named parameter");
                assert_eq!(found.value(), &value, "{error}");
            }
            let json = presentation.to_json();
            assert_eq!(
                json.get("detail")
                    .and_then(|detail| detail.get("messageId"))
                    .and_then(purrdf_lex::json::Value::as_str),
                Some(identity)
            );
        }
        // A valid relative reference is refused by the term-position rule,
        // rather than by the IRI validator.
        let relative = NamedNode::new("relative").unwrap_err();
        assert!(relative.presentation().detail().is_none());
        assert_eq!(
            purrdf_testkit::error_capture::Capture::read(&relative)
                .child("reason")
                .code,
            "sparql-iri-relative-term"
        );
    }

    #[test]
    fn byte_offset_only_for_positional_variants() {
        assert_eq!(ParseError::lex("x", 7).byte_offset(), Some(7));
        assert_eq!(ParseError::syntax("y", 3).byte_offset(), Some(3));
        assert_eq!(ParseError::unsupported("SERVICE").byte_offset(), None);
        assert_eq!(
            ParseError::Iri {
                lexical: "::".into(),
                reason: IriTermReason::Relative,
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
