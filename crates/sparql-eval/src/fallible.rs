// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
//!
//! The two halves of that sentence are refusals for two unrelated reasons, and neither
//! weakens the other. Rows stop here as a matter of **representation**: an evaluator row
//! carries [`SolutionTerm`](crate::SolutionTerm)s interned in a per-query scratch table
//! that dies with the evaluation context, so handing one out would be handing out a
//! dangling reference in all but name. [`FallibleSparqlError::Operational`] discards its
//! rows for a second, independent reason — **soundness**: a page that could not be read
//! bounds nothing, so the rows computed without it are neither a subset nor a superset of
//! the answer and there is no certificate to attach. A governed truncation is the case
//! where both objections lift at once: the rows are materialized out of the arena first,
//! and the engine chose to stop work it could have done, so what they bound is derivable.

use crate::{RetainedEvidence, RetainedSparqlResult};
use purrdf_core::{RdfDiagnostic, TrippedGovernor};

use crate::governed::PartialAnswers;

/// The public return type for a query over an operationally fallible view.
pub type FallibleSparqlResult<OperationalError, Evidence> =
    Result<CompleteSparqlResult<Evidence>, FallibleSparqlError<OperationalError, Evidence>>;

/// A caller-owned value and its exact final operational receipt, used by scoped
/// visitors and measured EXPLAIN. The value may be published only after this
/// complete read boundary returns `Ok`.
pub type FallibleScopedResult<R, OperationalError, Evidence> =
    Result<(R, Evidence), FallibleSparqlError<OperationalError, Evidence>>;

/// A fully materialized SPARQL result whose backing view reached a final ready
/// checkpoint.
///
/// The wrapper is the completeness certificate: the evaluator never constructs it
/// from internal partial rows. `evidence` records the deterministic resources and
/// lazy requests consumed by this exact execution.
#[derive(Debug, Clone)]
pub struct CompleteSparqlResult<Evidence> {
    /// The complete dataset-independent SPARQL result.
    pub result: RetainedSparqlResult,
    /// Deterministic operational evidence captured after result materialization.
    pub evidence: RetainedEvidence<Evidence>,
}

impl<Evidence> CompleteSparqlResult<Evidence> {
    /// Decompose the completeness certificate into result and evidence.
    #[must_use]
    pub fn into_parts(self) -> (RetainedSparqlResult, RetainedEvidence<Evidence>) {
        (self.result, self.evidence)
    }
}

/// An allocation-free engine control refusal, including before the first
/// workspace account exists. This preserves the exact static cause without
/// constructing diagnostic text under a refused allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum QueryControlFailure {
    /// The required allocation owner was not supplied.
    Unpriced(&'static str),
    /// A concrete live layout cannot be represented.
    LayoutOverflow,
    /// The native diagnostic formatter violated its contract.
    DiagnosticFormatting,
    /// The shared account stopped after retaining its original source cause.
    WorkspaceStopped,
}

impl QueryControlFailure {
    pub(crate) fn from_eval_error(error: &crate::EvalError) -> Option<Self> {
        match error {
            crate::EvalError::WorkspaceUnpriced(value) => Some(Self::Unpriced(value)),
            crate::EvalError::WorkspaceBoundOverflow => Some(Self::LayoutOverflow),
            crate::EvalError::UnstableNativeDiagnostic => Some(Self::DiagnosticFormatting),
            crate::EvalError::WorkspaceStopped => Some(Self::WorkspaceStopped),
            _ => None,
        }
    }

    fn as_eval_error(self) -> crate::EvalError {
        match self {
            Self::Unpriced(value) => crate::EvalError::WorkspaceUnpriced(value),
            Self::LayoutOverflow => crate::EvalError::WorkspaceBoundOverflow,
            Self::DiagnosticFormatting => crate::EvalError::UnstableNativeDiagnostic,
            Self::WorkspaceStopped => crate::EvalError::WorkspaceStopped,
        }
    }

    /// Stable machine-readable identity without a rendered allocation.
    #[must_use]
    pub fn diagnostic_code(self) -> &'static str {
        self.as_eval_error()
            .diagnostic_code()
            .expect("every static control failure has a diagnostic code")
    }
}

impl std::fmt::Display for QueryControlFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.as_eval_error(), f)
    }
}

impl std::error::Error for QueryControlFailure {}

/// A query over a [`FallibleDatasetView`](purrdf_core::FallibleDatasetView) that did not
/// reach a complete result.
///
/// # Why this type carries no `PartialEq`/`Eq`
///
/// [`Self::BudgetExhausted`] carries a materialized [`RetainedSparqlResult`],
/// which is deliberately not comparable: its graph variant retains a dataset owner,
/// whose equality is a dataset isomorphism question rather than a derive. Comparing
/// two of these values was never the right test anyway: a test asserts the
/// *discriminant* and the evidence, both
/// of which are still comparable on their own.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum FallibleSparqlError<OperationalError, Evidence> {
    /// Parsing or evaluation failed while the view itself remained operational.
    Query {
        /// The ordinary parse/evaluation diagnostic.
        diagnostic: crate::RetainedDiagnostic,
        /// Deterministic evidence captured at the final ready checkpoint.
        evidence: Evidence,
    },
    /// The view refused admission or failed operationally. This variant takes
    /// precedence over any evaluator error derived after data became unavailable.
    Operational {
        /// The typed refusal or read error; an existing sticky root takes precedence.
        error: OperationalError,
        /// Deterministic evidence at the failure boundary.
        evidence: Evidence,
    },
    /// A real engine allocator refused construction. This static outcome needs
    /// no fresh diagnostic allocation, including at the first account boundary.
    AllocationFailed {
        /// The engine allocation that could not be constructed.
        construct: &'static str,
        /// Backend-owned or previously admitted evidence at the checkpoint.
        evidence: Evidence,
    },
    /// An engine control invariant refused the operation without allocating text.
    ControlFailure {
        /// Exact typed, allocation-free engine cause.
        failure: QueryControlFailure,
        /// Backend-owned or previously admitted evidence.
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
        partial: PartialAnswers<crate::RetainedPartialSparqlResult>,
        /// Deterministic evidence at the final ready checkpoint. On the governed lane
        /// this is a
        /// [`GovernedEvidence`](crate::GovernedEvidence), so the governor accounting
        /// that produced the trip travels with the view's own evidence.
        evidence: Evidence,
    },
}

impl<OperationalError, Evidence> FallibleSparqlError<OperationalError, Evidence> {
    /// Attach an outer operation receipt without changing the outcome or root cause.
    pub(crate) fn map_evidence<V>(
        self,
        map: impl FnOnce(Evidence) -> V,
    ) -> FallibleSparqlError<OperationalError, V> {
        match self {
            Self::Query {
                diagnostic,
                evidence,
            } => FallibleSparqlError::Query {
                diagnostic,
                evidence: map(evidence),
            },
            Self::Operational { error, evidence } => FallibleSparqlError::Operational {
                error,
                evidence: map(evidence),
            },
            Self::AllocationFailed {
                construct,
                evidence,
            } => FallibleSparqlError::AllocationFailed {
                construct,
                evidence: map(evidence),
            },
            Self::ControlFailure { failure, evidence } => FallibleSparqlError::ControlFailure {
                failure,
                evidence: map(evidence),
            },
            Self::BudgetExhausted {
                tripped,
                partial,
                evidence,
            } => FallibleSparqlError::BudgetExhausted {
                tripped,
                partial,
                evidence: map(evidence),
            },
        }
    }

    /// Borrow the deterministic evidence carried by every non-complete outcome.
    #[must_use]
    pub const fn evidence(&self) -> &Evidence {
        match self {
            Self::Query { evidence, .. }
            | Self::Operational { evidence, .. }
            | Self::AllocationFailed { evidence, .. }
            | Self::ControlFailure { evidence, .. }
            | Self::BudgetExhausted { evidence, .. } => evidence,
        }
    }

    /// Borrow the operational root cause, when the view failed.
    #[must_use]
    pub const fn operational_error(&self) -> Option<&OperationalError> {
        match self {
            Self::Query { .. }
            | Self::AllocationFailed { .. }
            | Self::ControlFailure { .. }
            | Self::BudgetExhausted { .. } => None,
            Self::Operational { error, .. } => Some(error),
        }
    }

    /// Borrow the ordinary query diagnostic, when parsing/evaluation failed while
    /// the view remained ready.
    #[must_use]
    pub fn diagnostic(&self) -> Option<&RdfDiagnostic> {
        match self {
            Self::Query { diagnostic, .. } => Some(diagnostic.diagnostic()),
            Self::Operational { .. }
            | Self::AllocationFailed { .. }
            | Self::ControlFailure { .. }
            | Self::BudgetExhausted { .. } => None,
        }
    }

    /// The governor that stopped the execution, when one did.
    #[must_use]
    pub const fn tripped(&self) -> Option<TrippedGovernor> {
        match self {
            Self::Query { .. }
            | Self::Operational { .. }
            | Self::AllocationFailed { .. }
            | Self::ControlFailure { .. } => None,
            Self::BudgetExhausted { tripped, .. } => Some(*tripped),
        }
    }

    /// Borrow the certified partial answers, when a governor stopped the execution.
    #[must_use]
    pub const fn partial_answers(
        &self,
    ) -> Option<&PartialAnswers<crate::RetainedPartialSparqlResult>> {
        match self {
            Self::Query { .. }
            | Self::Operational { .. }
            | Self::AllocationFailed { .. }
            | Self::ControlFailure { .. } => None,
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
            Self::AllocationFailed { construct, .. } => {
                write!(f, "engine allocation failed: {construct}")
            }
            Self::ControlFailure { failure, .. } => failure.fmt(f),
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
            Self::ControlFailure { failure, .. } => Some(failure),
            // A tripped governor is a typed outcome, not an error with a cause: there is
            // no underlying failure to point at, and inventing one would report a
            // bounded query as a broken one.
            Self::BudgetExhausted { .. } | Self::AllocationFailed { .. } => None,
        }
    }
}
