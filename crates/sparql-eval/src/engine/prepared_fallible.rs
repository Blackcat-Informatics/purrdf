// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Prepared execution composing caller governors and fallible view checkpoints.

use super::{
    Arc, FallibleDatasetView, FallibleSparqlError, FallibleSparqlResult, GovernedEvidence,
    GovernorState, NativeSparqlEngine, PreparedQuery, QueryOptions, TermValue, ViewOperationStatus,
    finish_governed_fallible_query,
};
use crate::remote::ServiceResolver;

impl NativeSparqlEngine {
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
        self.prepared_fallible_in_state(dataset, prepared, substitutions, options, None, state)
    }

    /// The prepared, shared-governor fallible entry with a federation source.
    ///
    /// `SERVICE` receives the same operation stop signal as local evaluation.
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
        self.prepared_fallible_in_state(
            dataset,
            prepared,
            substitutions,
            options,
            Some(source),
            state,
        )
    }

    #[allow(
        clippy::result_large_err,
        reason = "the error carries both operation receipts and certified partial answers"
    )]
    fn prepared_fallible_in_state<'d, D: FallibleDatasetView + Sync>(
        &'d self,
        dataset: &'d D,
        prepared: &PreparedQuery,
        substitutions: &[(String, TermValue)],
        options: QueryOptions<'d>,
        source: Option<&'d (dyn ServiceResolver + Sync)>,
        state: &Arc<GovernorState>,
    ) -> FallibleSparqlResult<D::Error, GovernedEvidence<D::Evidence>> {
        if let ViewOperationStatus::Failed { error, evidence } = dataset.operation_status() {
            return Err(FallibleSparqlError::Operational {
                error,
                evidence: GovernedEvidence::new(evidence, state.evidence()),
            });
        }
        let evaluation = {
            let _sequential = crate::parallel::force_sequential_operation();
            self.query_governed_prepared_in_state(
                dataset,
                prepared,
                substitutions,
                options,
                source,
                state,
            )
        };
        finish_governed_fallible_query(dataset, state, evaluation)
    }
}
