// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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

mod namespace;
pub use namespace::NamespaceRefusal;
mod malformed;
pub use malformed::MalformedResult;
mod read;
pub use crate::json_read::JsonRelease;
pub use read::{ReadReason, ReadRefusal, ReadShape, ReadToken};

/// Errors produced while serializing a SPARQL result.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The chosen result format cannot represent this result form.
    UnsupportedResult {
        /// Original requested output format.
        format: crate::SparqlResultsFormat,
        /// True for ASK, false for a CONSTRUCT graph.
        ask: bool,
    },
    /// A TSV header contains a character the format cannot escape.
    TsvHeader {
        /// Original variable name.
        variable: String,
    },
    /// Original XML grammar refusal and its source coordinates.
    XmlSyntax {
        /// Original XML cause.
        cause: purrdf_lex::xml::XmlError,
        /// Original source line.
        line: usize,
        /// Original source column.
        column: usize,
    },
    /// An XML results document is not UTF-8.
    Utf8 {
        /// Original input byte length.
        length: usize,
        /// Original UTF-8 cause.
        cause: core::str::Utf8Error,
    },
    /// The original JSON grammar refusal and byte offset.
    Json(purrdf_lex::json::Error),
    /// An original scalar that XML cannot represent.
    XmlCharacter(purrdf_core::xml_escape::InvalidXmlChar),
    /// The original results-document language tag and grammar refusal.
    LanguageTag {
        /// The results format being decoded.
        format: &'static str,
        /// Original rejected tag.
        language: String,
        /// Original grammar classification.
        cause: purrdf_iri::langtag::LanguageTagError,
    },
    /// A result value violated an invariant the serializer relies on (for
    /// example a triple-term predicate that is not an IRI). Retains the
    /// native invariant and its original operands.
    MalformedTerm(MalformedResult),
    /// An input document violates the results grammar. Retains the structural
    /// requirement or original token/term independently of its presentation.
    Format(ReadRefusal),
    /// A caller-supplied [`crate::model::ProvenanceNamespace`] failed
    /// construction-time validation: the `prefix` is not a valid XML
    /// Namespaces `NCName`, the `prefix` collides with a reserved name
    /// (`xml`/`xmlns`, checked case-insensitively), or the `iri` is not a
    /// syntactically valid absolute IRI. See
    /// [`crate::model::ProvenanceNamespace::new`] for the exact rules.
    InvalidNamespace(NamespaceRefusal),
    /// Original native producer admission or allocator refusal.
    Storage(purrdf_lex::allocation::StorageError),
    /// Original term-release refusal. A failed census retains the original term
    /// and its grant; `None` means destruction preceded the refund refusal.
    TermRelease {
        /// Original payload when destruction has not occurred.
        term: Option<purrdf_core::TermValue>,
        /// Original census or refund failure.
        cause: purrdf_lex::allocation::StorageError,
    },
    /// A JSON cleanup refusal retaining all original residual containers.
    JsonRelease {
        /// Remaining payloads and their original backing allocations.
        owner: JsonRelease,
        /// Original cleanup refusal, retained across retries.
        cause: purrdf_lex::allocation::StorageError,
    },
    /// The destination a streaming serialization was writing into failed.
    ///
    /// Retains the original native drain cause, including shared I/O ownership.
    ///
    /// `kind` is kept alongside the cause because the description alone is not
    /// actionable: the one distinction a caller streaming to a pipe actually has to
    /// make is a downstream reader that closed early — the ubiquitous `… | head`
    /// idiom, which is not a failure — against every other write error, which is.
    /// Flattening that into a string forced any caller wanting the distinction to
    /// carry it out of band or to match on prose.
    ///
    /// The type is the SINK's classification rather than `std::io::ErrorKind`,
    /// because that is the classification actually made here and it is the one that
    /// stays true off `std::io`: a [`purrdf_core::sink::ByteDrain`] may be a JS
    /// callback, a C function pointer or a Python file object, none of which has an
    /// `io::ErrorKind` to report. It is `Copy + Eq`, so it costs this enum none of
    /// its derives.
    Write {
        /// What kind of write failure this was.
        kind: purrdf_core::sink::DrainErrorKind,
        /// The original destination refusal, without a rendered replacement.
        cause: purrdf_core::sink::DrainError,
    },
}

/// Native causes without a recursive JSON residual owner. Payloads move between
/// this finite representation and the public error without rendering or copying.
macro_rules! native_primary {
    (
        records { $( $record:ident { $( $field:ident: $field_type:ty ),+ $(,)? } ),+ $(,)? }
        tuples { $( $tuple:ident ( $value:ident: $value_type:ty ) ),+ $(,)? }
    ) => {
        #[derive(Debug, PartialEq, Eq)]
        pub(crate) enum NativePrimary {
            $( $record { $( $field: $field_type ),+ }, )+
            $( $tuple($value_type), )+
        }

        impl NativePrimary {
            /// Move a native cause, returning an existing residual owner separately.
            pub(crate) fn from_error(
                error: Error,
            ) -> Result<Self, (JsonRelease, purrdf_lex::allocation::StorageError)> {
                match error {
                    $( Error::$record { $( $field ),+ } => Ok(Self::$record { $( $field ),+ }), )+
                    $( Error::$tuple($value) => Ok(Self::$tuple($value)), )+
                    Error::JsonRelease { owner, cause } => Err((owner, cause)),
                }
            }

            /// Restore the exact original public variant and owned payload.
            pub(crate) fn into_error(self) -> Error {
                match self {
                    $( Self::$record { $( $field ),+ } => Error::$record { $( $field ),+ }, )+
                    $( Self::$tuple($value) => Error::$tuple($value), )+
                }
            }
        }
    };
}

native_primary! {
    records {
        UnsupportedResult { format: crate::SparqlResultsFormat, ask: bool },
        TsvHeader { variable: String },
        XmlSyntax { cause: purrdf_lex::xml::XmlError, line: usize, column: usize },
        Utf8 { length: usize, cause: core::str::Utf8Error },
        LanguageTag { format: &'static str, language: String, cause: purrdf_iri::langtag::LanguageTagError },
        TermRelease { term: Option<purrdf_core::TermValue>, cause: purrdf_lex::allocation::StorageError },
        Write { kind: purrdf_core::sink::DrainErrorKind, cause: purrdf_core::sink::DrainError },
    }
    tuples {
        Json(cause: purrdf_lex::json::Error),
        XmlCharacter(cause: purrdf_core::xml_escape::InvalidXmlChar),
        MalformedTerm(cause: MalformedResult),
        Format(cause: ReadRefusal),
        InvalidNamespace(cause: NamespaceRefusal),
        Storage(cause: purrdf_lex::allocation::StorageError),
    }
}

impl NativePrimary {
    /// Dispose only payloads created by the closed JSON reader under this account.
    /// A pre-destruction census refusal returns the original semantic variant;
    /// a post-destruction refund refusal has no surviving payload to return.
    pub(crate) fn release_with_memory<S: purrdf_lex::allocation::Admission + ?Sized>(
        self,
        memory: &mut purrdf_lex::allocation::Memory<'_, S>,
    ) -> Result<(), (Option<Self>, purrdf_lex::allocation::StorageError)> {
        match self {
            Self::Format(ReadRefusal { format, reason }) => match reason {
                ReadReason::Shape(_) => Ok(()),
                ReadReason::Token { token, .. } => {
                    memory.release_string(token).map_err(|cause| (None, cause))
                }
                ReadReason::Predicate(term) => {
                    term.release_with_memory(memory).map_err(|(term, cause)| {
                        (
                            term.map(|term| {
                                Self::Format(ReadRefusal {
                                    format,
                                    reason: ReadReason::Predicate(term),
                                })
                            }),
                            cause,
                        )
                    })
                }
            },
            Self::LanguageTag { language, .. } => memory
                .release_string(language)
                .map_err(|cause| (None, cause)),
            Self::TermRelease {
                term: Some(term),
                cause: original,
            } => term.release_with_memory(memory).map_err(|(term, cause)| {
                (
                    term.map(|term| Self::TermRelease {
                        term: Some(term),
                        cause: original,
                    }),
                    cause,
                )
            }),
            // Other native variants carry no payload admitted by the JSON reader.
            Self::UnsupportedResult { .. }
            | Self::TsvHeader { .. }
            | Self::XmlSyntax { .. }
            | Self::Utf8 { .. }
            | Self::Json(_)
            | Self::XmlCharacter(_)
            | Self::MalformedTerm(_)
            | Self::InvalidNamespace(_)
            | Self::Storage(_)
            | Self::TermRelease { term: None, .. }
            | Self::Write { .. } => Ok(()),
        }
    }
}

// Keep native error presentation in one definition. The public container case
// alone adds residual ownership; all native codes, fields and messages are shared.
macro_rules! native_error_presentation {
    (
        $error:ident;
        structured { $($structured:tt)* }
        display($formatter:ident) { $($display:tt)* }
        source { $($sources:tt)* }
    ) => {
        impl purrdf_hash::error::StructuredError for $error {
            purrdf_hash::error_visit! {
                UnsupportedResult { format, ask } => "sparql-results-unsupported-form" { "format" => Record(format), "ask" => Boolean(*ask), "form" => Code(if *ask { "ask" } else { "construct" }) },
                TsvHeader { variable } => "sparql-results-tsv-header" { "variable" => Text(variable) },
                XmlSyntax { cause, line, column } => "sparql-results-xml-syntax" { "cause" => Cause(cause), "line" => Unsigned(*line as u128), "column" => Unsigned(*column as u128) },
                Utf8 { length, cause } => "sparql-results-utf8" { "length" => ByteCount(*length as u128), "cause" => Cause(cause) },
                Json(cause) => "sparql-results-json" { "cause" => Cause(cause) },
                XmlCharacter(cause) => "sparql-results-xml-character" { "cause" => Cause(cause) },
                LanguageTag { format, language, cause } => "sparql-results-language-tag" {
                    "format" => Code(format), "language" => Text(language), "cause" => Cause(cause)
                },
                MalformedTerm(cause) => "sparql-results-malformed-term" { "cause" => Cause(cause) },
                Format(cause) => "sparql-results-format" { "cause" => Cause(cause) },
                InvalidNamespace(cause) => "sparql-results-invalid-namespace" { "cause" => Cause(cause) },
                Storage(cause) => "sparql-results-storage" { "cause" => Cause(cause) },
                TermRelease { term, cause } => "sparql-results-term-release" {
                    "retained" => Boolean(term.is_some()), "cause" => Cause(cause)
                },
                Write { kind, cause } => "sparql-results-write" { "kind" => Record(kind), "cause" => Cause(cause) },
                $($structured)*
            }
        }

        impl fmt::Display for $error {
            fn fmt(&self, $formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self {
                    Self::UnsupportedResult {
                        format: crate::SparqlResultsFormat::Xml,
                        ask: false,
                    } => write!(
                        $formatter,
                        "result format error: {}",
                        crate::xml::GRAPH_IS_UNDEFINED_IN_SRX
                    ),
                    Self::UnsupportedResult { format, ask } => write!(
                        $formatter,
                        "result format error: SPARQL Results {} is defined only for SELECT variable bindings, not {}",
                        format.token().to_ascii_uppercase(),
                        if *ask { "ASK" } else { "CONSTRUCT graphs" }
                    ),
                    Self::TsvHeader { variable } => write!(
                        $formatter,
                        "result format error: variable name {variable:?} contains a tab/CR/LF, which SPARQL Results TSV has no way to escape in the header"
                    ),
                    Self::XmlSyntax {
                        cause,
                        line,
                        column,
                    } => write!(
                        $formatter,
                        "result format error: SPARQL-XML: {cause} (line {line}, column {column})"
                    ),
                    Self::Utf8 { .. } => $formatter.write_str("result format error: SPARQL-XML: non-UTF-8 document"),
                    Self::Json(cause) => write!($formatter, "result format error: SPARQL-JSON: {cause}"),
                    Self::XmlCharacter(cause) => write!($formatter, "result format error: {cause}"),
                    Self::LanguageTag {
                        format,
                        language,
                        cause,
                    } => write!(
                        $formatter,
                        "result format error: {format}: invalid language tag `{language}` ({}: {})",
                        cause.diagnostic_code(),
                        cause.message()
                    ),
                    Self::MalformedTerm(msg) => write!($formatter, "malformed result term: {msg}"),
                    Self::Format(msg) => write!($formatter, "result format error: {msg}"),
                    Self::InvalidNamespace(msg) => write!($formatter, "invalid provenance namespace: {msg}"),
                    Self::Storage(error) | Self::TermRelease { cause: error, .. } => write!($formatter, "{error}"),
                    Self::Write { cause, .. } => write!($formatter, "result write error: {cause}"),
                    $($display)*
                }
            }
        }

        purrdf_lex::error_source!($error => {
            Self::XmlSyntax { cause, .. } => cause,
            Self::Utf8 { cause, .. } => cause,
            Self::InvalidNamespace(cause) => cause,
            Self::MalformedTerm(cause) => cause,
            Self::Format(cause) => cause,
            Self::Json(cause) => cause,
            Self::XmlCharacter(cause) => cause,
            Self::LanguageTag { cause, .. } => cause,
            Self::Storage(cause) => cause,
            Self::TermRelease { cause, .. } => cause,
            Self::Write { cause, .. } => cause,
            $($sources)*
        });

    };
}

native_error_presentation! {
    Error;
    structured {
        JsonRelease { owner, cause } => "sparql-results-json-release" {
            "container" => Code(owner.kind()), "cause" => Cause(owner.primary(cause)), "cleanup" => Cause(cause)
        },
    }
    display(f) {
        Self::JsonRelease { owner, cause } => fmt::Display::fmt(owner.primary(cause), f),
    }
    source {
        Self::JsonRelease { owner, cause } => owner.primary(cause),
    }
}

native_error_presentation! {
    NativePrimary;
    structured {}
    display(f) {}
    source {}
}

/// The language-tag grammar a tag read out of a results DOCUMENT is held to.
///
/// The same [`purrdf_iri::langtag::Profile`] the RDF codec readers, the SPARQL
/// parser and `RdfLiteral::validate_components` all name, because a results
/// document is parsed input like any other and the grammar has one owner.
const LANGTAG_PROFILE: purrdf_iri::langtag::Profile =
    purrdf_iri::langtag::Profile::ConcreteSyntaxLangtagBounded;

/// Judge an `xml:lang` read out of a results document, retaining the original
/// grammar refusal and returning [`None`] when it accepts.
///
/// # Why the readers need this at all
///
/// [`crate::from_json`] and [`crate::from_xml`] are public API, and they are how
/// a federated `SERVICE` response enters the engine. A tag that arrives from a
/// hostile or merely sloppy endpoint is otherwise copied verbatim into a
/// [`purrdf_core::TermValue`] and written straight back out — `"x"@en us` in
/// TSV, `"xml:lang":"en us"` in JSON — which no reader, including this crate's
/// own, can parse back. Validating on ingress is what the RDF codec readers
/// already do for exactly the same input class.
///
/// The caller moves its original tag into [`Error::LanguageTag`]. The human
/// presentation keeps the grammar's diagnostic code and message; structural
/// inspection receives that original cause rather than parsing the prose.
pub(crate) fn language_tag_refusal(lang: &str) -> Option<purrdf_iri::langtag::LanguageTagError> {
    purrdf_iri::langtag::parse_with(lang, LANGTAG_PROFILE).err()
}
