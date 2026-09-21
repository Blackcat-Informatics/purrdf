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
    /// The destination a streaming serialization was writing into failed.
    ///
    /// Carries the failure's description rather than the `io::Error` itself,
    /// because this enum is `Clone + PartialEq + Eq` and an `io::Error` is none of
    /// those. That trade costs the `source()` chain, which terminates here.
    ///
    /// `kind` is kept ALONGSIDE the message because the description alone is not
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
        /// The failure's description.
        message: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MalformedTerm(msg) => write!(f, "malformed result term: {msg}"),
            Self::Format(msg) => write!(f, "result format error: {msg}"),
            Self::InvalidNamespace(msg) => write!(f, "invalid provenance namespace: {msg}"),
            Self::Internal(msg) => write!(f, "internal error: {msg}"),
            Self::Write { message, .. } => write!(f, "result write error: {message}"),
        }
    }
}

impl std::error::Error for Error {}

/// The language-tag grammar a tag read out of a results DOCUMENT is held to.
///
/// The same [`purrdf_iri::langtag::Profile`] the RDF codec readers, the SPARQL
/// parser and `RdfLiteral::validate_components` all name, because a results
/// document is parsed input like any other and the grammar has one owner.
const LANGTAG_PROFILE: purrdf_iri::langtag::Profile =
    purrdf_iri::langtag::Profile::ConcreteSyntaxLangtagBounded;

/// Judge an `xml:lang` read out of a results document, returning the refusal
/// prose when the grammar rejects it and [`None`] when it accepts.
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
/// # What the message carries
///
/// The grammar's own [`diagnostic_code`](purrdf_iri::langtag::LanguageTagError::diagnostic_code)
/// and [`message`](purrdf_iri::langtag::LanguageTagError::message), in the
/// `(code: prose)` shape the CSVW and codec refusals already use, plus the
/// offending tag verbatim. There is no position to preserve: this crate's
/// [`Error`] is a `String` payload with no location field, and neither reader's
/// intermediate tree records source offsets, so the quoted tag IS the locating
/// information available. The caller prefixes its own format name.
pub(crate) fn language_tag_refusal(lang: &str) -> Option<String> {
    purrdf_iri::langtag::parse_with(lang, LANGTAG_PROFILE)
        .err()
        .map(|error| {
            format!(
                "invalid language tag `{lang}` ({}: {})",
                error.diagnostic_code(),
                error.message()
            )
        })
}
