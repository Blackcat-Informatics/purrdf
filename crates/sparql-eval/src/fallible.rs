// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Public result boundary for operationally fallible SPARQL execution.
//!
//! Ordinary resident and validated-pack queries keep returning
//! `Result<SparqlResult, RdfDiagnostic>`. A lazy view whose reads can fail instead
//! uses [`FallibleSparqlResult`]: only a final ready checkpoint yields
//! [`CompleteSparqlResult`], while either an ordinary query diagnostic or a typed
//! operational root cause carries the evidence accumulated by that execution.
//! Internal partial rows never cross this boundary.

use purrdf_core::{RdfDiagnostic, SparqlResult};

/// The public return type for a query over an operationally fallible view.
pub type FallibleSparqlResult<OperationalError, Evidence> =
    Result<CompleteSparqlResult<Evidence>, FallibleSparqlError<OperationalError, Evidence>>;

/// A fully materialized SPARQL result whose backing view reached a final ready
/// checkpoint.
///
/// The wrapper is the completeness certificate: the evaluator never constructs it
/// from internal partial rows. `evidence` records the deterministic resources and
/// lazy requests consumed by this exact execution.
#[derive(Debug, Clone)]
pub struct CompleteSparqlResult<Evidence> {
    /// The complete dataset-independent SPARQL result.
    pub result: SparqlResult,
    /// Deterministic operational evidence captured after result materialization.
    pub evidence: Evidence,
}

impl<Evidence> CompleteSparqlResult<Evidence> {
    /// Decompose the completeness certificate into result and evidence.
    #[must_use]
    pub fn into_parts(self) -> (SparqlResult, Evidence) {
        (self.result, self.evidence)
    }
}

/// Failure of a query over a [`FallibleDatasetView`](purrdf_core::FallibleDatasetView).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FallibleSparqlError<OperationalError, Evidence> {
    /// Parsing or evaluation failed while the view itself remained operational.
    Query {
        /// The ordinary parse/evaluation diagnostic.
        diagnostic: RdfDiagnostic,
        /// Deterministic evidence captured at the final ready checkpoint.
        evidence: Evidence,
    },
    /// The view failed operationally. This variant takes precedence over any
    /// evaluator error derived after data became unavailable.
    Operational {
        /// The sticky operational root cause.
        error: OperationalError,
        /// Deterministic evidence at the failure boundary.
        evidence: Evidence,
    },
}

impl<OperationalError, Evidence> FallibleSparqlError<OperationalError, Evidence> {
    /// Borrow the deterministic evidence carried by either failure variant.
    #[must_use]
    pub const fn evidence(&self) -> &Evidence {
        match self {
            Self::Query { evidence, .. } | Self::Operational { evidence, .. } => evidence,
        }
    }

    /// Borrow the operational root cause, when the view failed.
    #[must_use]
    pub const fn operational_error(&self) -> Option<&OperationalError> {
        match self {
            Self::Query { .. } => None,
            Self::Operational { error, .. } => Some(error),
        }
    }

    /// Borrow the ordinary query diagnostic, when parsing/evaluation failed while
    /// the view remained ready.
    #[must_use]
    pub const fn diagnostic(&self) -> Option<&RdfDiagnostic> {
        match self {
            Self::Query { diagnostic, .. } => Some(diagnostic),
            Self::Operational { .. } => None,
        }
    }
}

impl<OperationalError: std::fmt::Display, Evidence> std::fmt::Display
    for FallibleSparqlError<OperationalError, Evidence>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Query { diagnostic, .. } => diagnostic.fmt(f),
            Self::Operational { error, .. } => write!(f, "operational query failure: {error}"),
        }
    }
}

impl<OperationalError, Evidence> std::error::Error
    for FallibleSparqlError<OperationalError, Evidence>
where
    OperationalError: std::error::Error + 'static,
    Evidence: std::fmt::Debug,
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Query { diagnostic, .. } => Some(diagnostic),
            Self::Operational { error, .. } => Some(error),
        }
    }
}
