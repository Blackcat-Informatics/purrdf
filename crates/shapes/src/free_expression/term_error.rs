// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original term-boundary operands and native parser failures.

/// Why an offered RDF term could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TermParseError {
    /// A bare IRI spelling fails the native IRI grammar.
    Iri {
        /// Original caller input, before trimming.
        input: String,
        /// Original native cause.
        cause: purrdf_iri::IriError,
    },
    /// A bare reference has no scheme.
    RelativeIri {
        /// Original caller input, before trimming.
        input: String,
    },
    /// The shared N-Triples parser refused the term's carrier statement.
    Text {
        /// Original caller input, before trimming.
        input: String,
        /// Exact carrier statement parsed by native ingress.
        carrier: String,
        /// Byte offset at which the term spelling begins in the carrier.
        term_offset: usize,
        /// Ordered original parser diagnostics, located in the carrier.
        diagnostics: crate::text_ingest::TextErrors,
    },
    /// The carrier did not produce exactly one statement.
    NotOneTerm {
        /// Original caller input, before trimming.
        input: String,
        /// Actual number of resulting statements.
        statements: usize,
    },
}

impl purrdf_hash::error::StructuredError for TermParseError {
    purrdf_hash::error_visit! {
        Iri { input, cause } => "shacl-term-iri-refused" { "input" => Text(input), "cause" => Cause(cause) },
        RelativeIri { input } => "shacl-term-relative-iri" { "input" => Text(input) },
        Text { input, carrier, term_offset, diagnostics } => "shacl-term-text-refused" {
            "input" => Text(input), "carrier" => Text(carrier),
            "term_offset" => Unsigned(*term_offset as u128), "cause" => Cause(diagnostics),
        },
        NotOneTerm { input, statements } => "shacl-term-not-one" {
            "input" => Text(input), "statements" => Unsigned(*statements as u128),
        },
    }
}

impl std::fmt::Display for TermParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Iri { input, cause } => write!(
                f,
                "`{input}` is neither an N-Triples term nor an absolute IRI: {cause}"
            ),
            Self::RelativeIri { input } => write!(
                f,
                "`{input}` is a relative IRI reference; write the absolute IRI, or an N-Triples term"
            ),
            Self::Text {
                input, diagnostics, ..
            } => write!(f, "`{input}` is not one N-Triples term: {diagnostics}"),
            Self::NotOneTerm { input, .. } => write!(f, "`{input}` is not one N-Triples term"),
        }
    }
}

impl std::error::Error for TermParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Iri { cause, .. } => Some(cause),
            Self::Text { diagnostics, .. } => Some(diagnostics),
            _ => None,
        }
    }
}
