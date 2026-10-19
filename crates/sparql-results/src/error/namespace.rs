// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native provenance namespace validation refusals.

/// The original rejected prefix or namespace IRI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamespaceRefusal {
    /// The prefix is not an XML NCName.
    Prefix(String),
    /// XML reserves this prefix, case-insensitively.
    Reserved(String),
    /// The namespace IRI is syntactically malformed.
    Iri {
        /// Original namespace spelling.
        iri: String,
        /// Original grammar refusal.
        cause: purrdf_iri::IriError,
    },
    /// The namespace IRI has no scheme.
    Relative(String),
}

impl purrdf_hash::error::StructuredError for NamespaceRefusal {
    purrdf_hash::error_visit! {
        Prefix(prefix) => "results-namespace-prefix" { "prefix" => Text(prefix) },
        Reserved(prefix) => "results-namespace-reserved" { "prefix" => Text(prefix) },
        Iri { iri, cause } => "results-namespace-iri" { "iri" => Iri(iri), "cause" => Cause(cause) },
        Relative(iri) => "results-namespace-relative" { "iri" => Iri(iri) },
    }
}

impl core::fmt::Display for NamespaceRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Prefix(prefix) => {
                write!(f, "namespace prefix {prefix:?} is not a valid XML NCName")
            }
            Self::Reserved(prefix) => write!(
                f,
                "namespace prefix {prefix:?} is reserved (xml/xmlns, case-insensitive)"
            ),
            Self::Iri { iri, cause } => {
                write!(f, "namespace IRI {iri:?} is not a valid IRI: {cause}")
            }
            Self::Relative(iri) => write!(
                f,
                "namespace IRI {iri:?} is a relative IRI reference (no scheme); a namespace IRI must be absolute"
            ),
        }
    }
}

impl std::error::Error for NamespaceRefusal {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let Self::Iri { cause, .. } = self {
            Some(cause)
        } else {
            None
        }
    }
}
