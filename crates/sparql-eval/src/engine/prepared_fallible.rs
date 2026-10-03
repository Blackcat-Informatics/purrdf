// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Prepared execution composing caller governors and fallible view checkpoints.

use super::{
    AdmittedSubstitutions, Arc, FallibleDatasetView, FallibleSparqlError, FallibleSparqlResult,
    GovernedEvidence, GovernorState, NativeSparqlEngine, PreparedQuery, QueryGovernors,
    QueryOptions, TermValue, ViewOperationStatus, finish_governed_fallible_query,
};
use crate::remote::ServiceResolver;

impl NativeSparqlEngine {
    /// Execute a prepared query on a fallible view under per-call governors.
    ///
    /// No text parsing or plan-cache lookup occurs, including for algebra admitted
    /// through [`PreparedQuery::rewritten`]. Each call owns a fresh governor state,
    /// so reusing a plan or governor configuration never shares consumption.
    /// Reporting and evaluation workspace remain admitted through their respective
    /// publication scopes, exactly as for [`Self::query_governed_fallible_view`].
    ///
    /// `options` must carry the registries the plan was admitted against. Lazy
    /// operational reads retain deterministic sequential execution, and a result
    /// is published only after its final ready checkpoint.
    ///
    /// # Errors
    /// Operational failures outrank evaluator diagnostics and governor trips,
    /// discarding partial answers. A ready view returns a typed query diagnostic
    /// or budget exhaustion with certified partial answers and both receipts.
    #[allow(
        clippy::result_large_err,
        reason = "the error carries both operation receipts and certified partial answers"
    )]
    pub fn query_prepared_governed_fallible_view<'d, D: FallibleDatasetView + Sync>(
        &'d self,
        dataset: &'d D,
        prepared: &PreparedQuery,
        substitutions: &[(String, TermValue)],
        options: QueryOptions<'d>,
        governors: &QueryGovernors,
    ) -> FallibleSparqlResult<D::Error, GovernedEvidence<D::Evidence>> {
        let _reporting = super::reserve_governed_reporting(dataset, governors)?;
        let state = Arc::new(GovernorState::new(governors));
        preflight_governed_fallible_view(dataset, &state)?;
        self.query_prepared_governed_fallible_admitted(
            dataset,
            prepared,
            &AdmittedSubstitutions::prepared(substitutions),
            options,
            &state,
        )
    }

    /// Execute a prepared query on a fallible view under a shared operation governor.
    ///
    /// No text parsing or cache lookup occurs. The immutable plan and governor can
    /// be shared across worker-local engines. Each operation evaluates sequentially
    /// for deterministic lazy-read order and publishes only after a final ready
    /// checkpoint. The shared governor is neither reset nor multiplied.
    ///
    /// # Errors
    /// Operational failures outrank query errors and exhausted budgets, with all
    /// internal partial answers discarded. Query errors and budget exhaustion keep
    /// their typed outcomes and the combined view/governor evidence.
    #[allow(
        clippy::result_large_err,
        reason = "the error carries both operation receipts and certified partial answers"
    )]
    pub fn query_prepared_governed_fallible_in_operation<'d, D: FallibleDatasetView + Sync>(
        &'d self,
        dataset: &'d D,
        prepared: &PreparedQuery,
        substitutions: &[(String, TermValue)],
        options: QueryOptions<'d>,
        state: &Arc<GovernorState>,
    ) -> FallibleSparqlResult<D::Error, GovernedEvidence<D::Evidence>> {
        preflight_governed_fallible_view(dataset, state)?;
        let _reporting = super::reserve_fallible_reporting(dataset).map_err(|error| {
            error.map_evidence(|evidence| GovernedEvidence::new(evidence, state.evidence()))
        })?;
        self.query_prepared_governed_fallible_admitted(
            dataset,
            prepared,
            &AdmittedSubstitutions::prepared(substitutions),
            options,
            state,
        )
    }

    /// The admitted prepared boundary, with reporting owned by its public entry.
    #[allow(
        clippy::result_large_err,
        reason = "the shared boundary preserves the public entry's paired receipts and partial answers"
    )]
    pub(super) fn query_prepared_governed_fallible_admitted<'d, D: FallibleDatasetView + Sync>(
        &'d self,
        dataset: &'d D,
        prepared: &PreparedQuery,
        substitutions: &AdmittedSubstitutions<'_>,
        options: QueryOptions<'d>,
        state: &Arc<GovernorState>,
    ) -> FallibleSparqlResult<D::Error, GovernedEvidence<D::Evidence>> {
        if let Err(diagnostic) = super::bounded_workspace::check_inputs(
            dataset,
            !substitutions.values.is_empty(),
            options,
        ) {
            return finish_governed_fallible_query(dataset, state, Err(diagnostic));
        }
        let workspace = super::reserve_fallible_workspace(
            dataset,
            &prepared.query,
            !substitutions.values.is_empty(),
            options,
        )
        .map_err(|error| {
            error.map_evidence(|evidence| GovernedEvidence::new(evidence, state.evidence()))
        })?;
        let evaluation = self.query_governed_prepared_admitted(
            dataset,
            prepared,
            substitutions,
            options,
            state,
            super::Sequencing::for_view::<D>(),
            &workspace,
        );
        finish_governed_fallible_query(dataset, state, evaluation)
    }

    /// [`Self::query_prepared_governed_fallible_view`] with a federation source.
    ///
    /// The explicit `source` replaces [`QueryOptions::remote`]. Local reads and
    /// `SERVICE` calls use the same per-call governor and publication boundary.
    ///
    /// # Errors
    /// Returns the same typed errors and operational failure precedence as
    /// [`Self::query_prepared_governed_fallible_view`].
    #[allow(
        clippy::result_large_err,
        reason = "the error carries both operation receipts and certified partial answers"
    )]
    pub fn query_prepared_governed_fallible_with_source_view<'d, D: FallibleDatasetView + Sync>(
        &'d self,
        dataset: &'d D,
        prepared: &PreparedQuery,
        substitutions: &[(String, TermValue)],
        source: &'d (dyn ServiceResolver + Sync),
        options: QueryOptions<'d>,
        governors: &QueryGovernors,
    ) -> FallibleSparqlResult<D::Error, GovernedEvidence<D::Evidence>> {
        self.query_prepared_governed_fallible_view(
            dataset,
            prepared,
            substitutions,
            QueryOptions {
                remote: Some(source),
                ..options
            },
            governors,
        )
    }

    /// The prepared, shared-governor fallible entry with a federation source.
    ///
    /// `SERVICE` receives the same operation stop signal as local evaluation.
    ///
    /// Exactly [`Self::query_prepared_governed_fallible_in_operation`] with
    /// [`QueryOptions::remote`] set to `source`; the explicit `source` replaces whatever
    /// `options.remote` held.
    ///
    /// # Errors
    /// Returns the same typed errors and publication guarantees as
    /// [`Self::query_prepared_governed_fallible_in_operation`].
    #[allow(
        clippy::result_large_err,
        reason = "the error carries both operation receipts and certified partial answers"
    )]
    pub fn query_prepared_governed_fallible_with_source_in_operation<
        'd,
        D: FallibleDatasetView + Sync,
    >(
        &'d self,
        dataset: &'d D,
        prepared: &PreparedQuery,
        substitutions: &[(String, TermValue)],
        source: &'d (dyn ServiceResolver + Sync),
        options: QueryOptions<'d>,
        state: &Arc<GovernorState>,
    ) -> FallibleSparqlResult<D::Error, GovernedEvidence<D::Evidence>> {
        self.query_prepared_governed_fallible_in_operation(
            dataset,
            prepared,
            substitutions,
            QueryOptions {
                remote: Some(source),
                ..options
            },
            state,
        )
    }
}

/// Sample the ingress checkpoint once, pairing its root cause with governor use.
#[allow(
    clippy::result_large_err,
    reason = "the checkpoint preserves the public entry's typed paired receipts"
)]
pub(super) fn preflight_governed_fallible_view<D: FallibleDatasetView + Sync>(
    dataset: &D,
    state: &GovernorState,
) -> Result<(), super::GovernedReadFailure<D>> {
    match dataset.operation_status() {
        ViewOperationStatus::Ready { .. } => Ok(()),
        ViewOperationStatus::Failed { error, evidence } => Err(FallibleSparqlError::Operational {
            error,
            evidence: GovernedEvidence::new(evidence, state.evidence()),
        }),
    }
}
