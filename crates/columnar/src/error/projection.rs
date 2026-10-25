// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original native RDF projection grammar refusals.

use core::fmt;
use purrdf_core::TermValue;

/// Why an RDF source cannot be projected into the native five-table schema.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProjectionReason {
    /// A triple-term dependency returns to an active term.
    CyclicTriple,
    /// A literal's resolved datatype is not an IRI.
    DatatypeNotIri {
        /// Original resolved value, transferred without copying its term tree.
        actual: TermValue,
    },
    /// A source term has no entry in the completed resolution closure.
    UnresolvedSource,
    /// A resolved value has no entry in the canonical dictionary.
    MissingTerm,
    /// A reifier binding's object is not a triple term.
    ReifierObject,
}

impl purrdf_hash::error::StructuredError for ProjectionReason {
    purrdf_hash::error_visit! {
        CyclicTriple => "columnar-projection-cyclic-triple" {},
        DatatypeNotIri { actual } => "columnar-projection-datatype-not-iri" {
            "actual" => Term(actual),
        },
        UnresolvedSource => "columnar-projection-unresolved-source" {},
        MissingTerm => "columnar-projection-missing-term" {},
        ReifierObject => "columnar-projection-reifier-object" {},
    }
}

impl fmt::Display for ProjectionReason {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CyclicTriple => output.write_str("cyclic triple-term reference"),
            Self::DatatypeNotIri { .. } => {
                output.write_str("datatype id does not resolve to an IRI")
            }
            Self::UnresolvedSource => output.write_str("source id was not resolved"),
            Self::MissingTerm => output.write_str("value is absent from dictionary"),
            Self::ReifierObject => output.write_str("object is not a triple term"),
        }
    }
}

impl core::error::Error for ProjectionReason {}
