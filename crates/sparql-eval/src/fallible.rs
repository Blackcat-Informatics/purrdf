// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Public result boundary for operationally fallible SPARQL execution.
//!
//! Ordinary resident and validated-pack queries keep returning
//! `Result<SparqlResult, RdfDiagnostic>`. A lazy view whose reads can fail instead
//! uses [`FallibleSparqlResult`]: only a final ready checkpoint yields
//! [`CompleteSparqlResult`], while an ordinary query diagnostic, a typed operational root
//! cause, or an exhausted execution budget carries the evidence accumulated by that
//! execution.
//!
//! Internal partial rows never cross this boundary. A governed execution's partial answers
//! do — but only after being materialized into the ordinary egress model and labelled with
//! what they bound ([`PartialAnswers`]), which is a different thing entirely: the
//! interned, arena-backed rows the evaluator holds still stop here.

use purrdf_core::{RdfDiagnostic, SparqlResult, TrippedGovernor};

use crate::governed::PartialAnswers;

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

/// A query over a [`FallibleDatasetView`](purrdf_core::FallibleDatasetView) that did not
/// reach a complete result.
///
/// # Why this type carries no `PartialEq`/`Eq`
///
/// [`Self::BudgetExhausted`] carries a materialized [`SparqlResult`], which is
/// deliberately not comparable — it holds an `Arc<RdfDataset>`, whose equality is a
/// dataset isomorphism question rather than a derive. Comparing two of these values was
/// never the right test anyway: a test asserts the *discriminant* and the evidence, both
/// of which are still comparable on their own.
#[derive(Debug, Clone)]
#[non_exhaustive]
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
    /// A caller-set execution governor stopped the query before it finished, over a view
    /// that stayed operational throughout.
    ///
    /// **This is not a failure of the query and not a failure of the view.** It is the
    /// third outcome: the answer is incomplete, the cause is typed, and the rows the
    /// budget already paid for are carried rather than discarded. It reaches the error
    /// channel only because [`CompleteSparqlResult`] is a *completeness* certificate and
    /// must never be constructed from partial rows; a caller that wants the two
    /// non-failure outcomes side by side uses
    /// [`GovernedOutcome`](crate::GovernedOutcome) on the infallible lane.
    BudgetExhausted {
        /// The governor that stopped the execution.
        tripped: TrippedGovernor,
        /// What the rows the execution reached bound, materialized.
        partial: PartialAnswers,
        /// Deterministic evidence at the final ready checkpoint. On the governed lane
        /// this is a
        /// [`GovernedEvidence`](crate::GovernedEvidence), so the governor accounting
        /// that produced the trip travels with the view's own evidence.
        evidence: Evidence,
    },
}

impl<OperationalError, Evidence> FallibleSparqlError<OperationalError, Evidence> {
    /// Borrow the deterministic evidence carried by every non-complete outcome.
    #[must_use]
    pub const fn evidence(&self) -> &Evidence {
        match self {
            Self::Query { evidence, .. }
            | Self::Operational { evidence, .. }
            | Self::BudgetExhausted { evidence, .. } => evidence,
        }
    }

    /// Borrow the operational root cause, when the view failed.
    #[must_use]
    pub const fn operational_error(&self) -> Option<&OperationalError> {
        match self {
            Self::Query { .. } | Self::BudgetExhausted { .. } => None,
            Self::Operational { error, .. } => Some(error),
        }
    }

    /// Borrow the ordinary query diagnostic, when parsing/evaluation failed while
    /// the view remained ready.
    #[must_use]
    pub const fn diagnostic(&self) -> Option<&RdfDiagnostic> {
        match self {
            Self::Query { diagnostic, .. } => Some(diagnostic),
            Self::Operational { .. } | Self::BudgetExhausted { .. } => None,
        }
    }

    /// The governor that stopped the execution, when one did.
    #[must_use]
    pub const fn tripped(&self) -> Option<TrippedGovernor> {
        match self {
            Self::Query { .. } | Self::Operational { .. } => None,
            Self::BudgetExhausted { tripped, .. } => Some(*tripped),
        }
    }

    /// Borrow the certified partial answers, when a governor stopped the execution.
    #[must_use]
    pub const fn partial_answers(&self) -> Option<&PartialAnswers> {
        match self {
            Self::Query { .. } | Self::Operational { .. } => None,
            Self::BudgetExhausted { partial, .. } => Some(partial),
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
            Self::BudgetExhausted { tripped, .. } => {
                write!(f, "query budget exhausted: {tripped}")
            }
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
            // A tripped governor is a typed outcome, not an error with a cause: there is
            // no underlying failure to point at, and inventing one would report a
            // bounded query as a broken one.
            Self::BudgetExhausted { .. } => None,
        }
    }
}
