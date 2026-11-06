// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original nonfatal query-artifact refusals, independent of their source bytes.

/// Why an ownership analyzer skipped a query artifact.
///
/// These causes retain their original fields after the catalog is dropped.
/// A refusal remains a diagnostic; it does not fail the ownership analysis.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum QueryArtifactError {
    /// The artifact was not UTF-8, before any SPARQL parsing was attempted.
    Utf8(core::str::Utf8Error),
    /// The native SPARQL parser refused the decoded artifact.
    Sparql(purrdf_sparql_algebra::ParseError),
}

impl core::fmt::Display for QueryArtifactError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Utf8(cause) => write!(formatter, "query is not valid UTF-8: {cause}"),
            Self::Sparql(cause) => cause.fmt(formatter),
        }
    }
}

purrdf_hash::error_sources!(QueryArtifactError, Utf8, Sparql);

impl purrdf_hash::error::StructuredError for QueryArtifactError {
    purrdf_hash::error_visit! {
        Utf8(cause) => "slice-query-utf8" { "cause" => Cause(cause) },
        Sparql(cause) => "slice-query-sparql" { "cause" => Cause(cause) },
    }
}
