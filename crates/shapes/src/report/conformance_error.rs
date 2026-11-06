// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Typed conformance configuration refusals.

/// Why a conformance-disallow set cannot represent the offered request.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConformanceError {
    /// An empty set cannot be stated by a validation report.
    EmptySet,
    /// The offered spelling fails the native IRI grammar.
    InvalidIri {
        /// Original caller spelling.
        input: String,
        /// Original native parser cause.
        cause: purrdf_iri::IriError,
    },
    /// A syntactically valid reference has no scheme.
    RelativeIri {
        /// Original caller spelling.
        input: String,
    },
    /// A report's conformance-disallow value is not an IRI term.
    NotIri {
        /// Original RDF value.
        value: crate::term::Term,
    },
}

impl purrdf_hash::error::StructuredError for ConformanceError {
    purrdf_hash::error_visit! {
        EmptySet => "shacl-conformance-empty-set" {},
        InvalidIri { input, cause } => "shacl-conformance-invalid-iri" {
            "input" => Text(input), "cause" => Cause(cause),
        },
        RelativeIri { input } => "shacl-conformance-relative-iri" { "input" => Text(input) },
        NotIri { value } => "shacl-conformance-not-iri" { "value" => Term(value) },
    }
}

impl std::fmt::Display for ConformanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptySet => f.write_str(
                "the conformance-disallow set is empty; a validation report with no \
                 sh:conformanceDisallows triple means the DEFAULT set (sh:Violation, sh:Warning, \
                 sh:Info), so an empty set has no report that states it — name at least one \
                 severity",
            ),
            Self::InvalidIri { input, .. } | Self::RelativeIri { input } => write!(
                f,
                "sh:conformanceDisallows value {input:?} is not an absolute IRI; all values of \
                 sh:conformanceDisallows must be IRIs"
            ),
            Self::NotIri { value } => write!(
                f,
                "sh:conformanceDisallows value {value} is not an IRI; all values of \
                 sh:conformanceDisallows must be IRIs"
            ),
        }
    }
}

impl std::error::Error for ConformanceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidIri { cause, .. } => Some(cause),
            _ => None,
        }
    }
}
