// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Prepared SELECT ingress with its original query and native execution permit.

use super::{
    EvaluationFailure, NativeSparqlEngine, PreparedQuery, QueryCoordinates, QueryOptions,
    RequestParameters, bounded_workspace, checked_query_read, coordinates, runtime_substitution,
};
use crate::eval::{CursorQuery, SelectCursor, SelectRow};
use crate::substitute::Prebindings;
use purrdf_core::{CursorDatasetView, DatasetView, RdfDiagnostic, TermValue, WorkspaceReservation};

#[expect(
    clippy::large_enum_variant,
    reason = "the cursor retains its admitted context inline rather than allocating another execution owner"
)]
enum PreparedCursorExecution<'q, 'd, D: DatasetView + Sync> {
    Active(SelectCursor<'q, 'd, D>),
    Refused {
        exhausted: crate::BudgetExhausted,
        answer_requested: bool,
        schema: crate::solution::SharedSchema,
    },
}

/// An ordinary prepared SELECT and the original permit that admits its reads.
/// A yielded row borrows this owner until its interned cells have been consumed.
pub struct PreparedSelectCursor<'q, 'd, D: DatasetView + Sync, R> {
    execution: PreparedCursorExecution<'q, 'd, D>,
    coordinates: QueryCoordinates,
    publication: crate::user_fn::RefusalPublication,
    identity: Option<super::AdmittedRelationIdentity>,
    workspace: crate::workspace::QueryWorkspace<D::ReadError>,
    _permit: Option<R>,
}

purrdf_hash::debug_non_exhaustive!(['q, 'd: 'q, D: DatasetView + Sync, R]
    PreparedSelectCursor<'q, 'd, D, R> {});

impl<'d, D: DatasetView<ReadError = std::convert::Infallible> + Sync, R>
    PreparedSelectCursor<'_, 'd, D, R>
{
    /// Read the next original row, retaining its scratch and evidence owner.
    ///
    /// # Errors
    /// Returns the original evaluation failure and permanently ends the cursor.
    pub fn next_row(&mut self) -> Result<Option<SelectRow<'_, 'd, D>>, crate::EvalError> {
        let next = self.prepare_next_row();
        let next = self.publication.finish(next, |_| true)?;
        Ok(next.map(|ordinal| self.prepared_row(ordinal)))
    }

    /// Stop this consumer and return its original completion and partition receipt.
    ///
    /// # Errors
    /// A failed cursor cannot publish a successful settlement.
    pub fn settle(&mut self) -> Result<crate::eval::SelectSettlement, crate::EvalError> {
        let settled = self.settle_original();
        self.publication.finish(settled, |_| true)
    }

    /// Borrow the requested native semantic status after successful settlement.
    /// Native admission refusal is already settled at open; a requested semantic
    /// status then reports an unread search rather than a complete empty answer.
    ///
    /// # Errors
    /// Refuses an unsettled or failed read and preserves native count failures.
    pub fn answer_status(&self) -> Result<crate::eval::CursorAnswerStatus<'_>, crate::EvalError> {
        self.original_answer_status()
    }

    /// Whether evaluation has settled as a complete SELECT.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.complete_read()
    }

    /// The coordinates captured by the prepared execution's original observer.
    #[must_use]
    pub const fn coordinates(&self) -> &QueryCoordinates {
        &self.coordinates
    }
}

impl<'q, 'd: 'q, D: DatasetView + Sync, R> PreparedSelectCursor<'q, 'd, D, R> {
    /// Original projected columns, readable before the first row and for empty answers.
    /// This header never advances a dataset probe.
    #[must_use]
    pub fn schema(&self) -> &crate::solution::VarSchema {
        match &self.execution {
            PreparedCursorExecution::Active(cursor) => cursor.schema(),
            PreparedCursorExecution::Refused { schema, .. } => schema,
        }
    }

    fn prepare_next_row(&mut self) -> Result<Option<usize>, crate::EvalError> {
        match &mut self.execution {
            PreparedCursorExecution::Active(cursor) => cursor.prepare_next_row(),
            PreparedCursorExecution::Refused { .. } => Ok(None),
        }
    }

    fn prepared_row(&self, ordinal: usize) -> SelectRow<'_, 'd, D> {
        let PreparedCursorExecution::Active(cursor) = &self.execution else {
            unreachable!("a refused cursor has no published row ordinal");
        };
        cursor.prepared_row(ordinal)
    }

    fn abort_read(&mut self) {
        if let PreparedCursorExecution::Active(cursor) = &mut self.execution {
            cursor.abort_read();
        }
    }

    fn complete_read(&self) -> bool {
        match &self.execution {
            PreparedCursorExecution::Active(cursor) => cursor.is_complete(),
            PreparedCursorExecution::Refused { .. } => false,
        }
    }

    fn settle_original(&mut self) -> Result<crate::eval::SelectSettlement, crate::EvalError> {
        match &mut self.execution {
            PreparedCursorExecution::Active(cursor) => {
                let original = cursor.is_settled();
                let settled = cursor.settle()?;
                if !original && let Some(identity) = &mut self.identity {
                    identity.value.witness = cursor.take_relation_witness();
                }
                Ok(settled)
            }
            PreparedCursorExecution::Refused { exhausted, .. } => {
                let prefix = |result: &crate::PartialSparqlResult| crate::eval::CursorPrefix {
                    rows: 0,
                    positional: result.is_positional_prefix(),
                };
                let answers = match &exhausted.partial {
                    crate::PartialAnswers::Certain(result) => {
                        crate::PartialAnswers::Certain(prefix(result))
                    }
                    crate::PartialAnswers::AtMost(result) => {
                        crate::PartialAnswers::AtMost(prefix(result))
                    }
                    crate::PartialAnswers::Unknown(barrier) => {
                        crate::PartialAnswers::Unknown(*barrier)
                    }
                };
                Ok(crate::eval::SelectSettlement {
                    completion: crate::eval::SelectCompletion::Truncated {
                        tripped: exhausted.tripped,
                        answers,
                    },
                    delivered: 0,
                    established: 0,
                    undecided: 0,
                    governors: Some(exhausted.evidence.clone()),
                })
            }
        }
    }

    fn original_answer_status(
        &self,
    ) -> Result<crate::eval::CursorAnswerStatus<'_>, crate::EvalError> {
        match &self.execution {
            PreparedCursorExecution::Active(cursor) => cursor.answer_status(),
            PreparedCursorExecution::Refused {
                answer_requested, ..
            } => Ok(if *answer_requested {
                crate::eval::CursorAnswerStatus::UnreadSuffix
            } else {
                crate::eval::CursorAnswerStatus::Unrequested
            }),
        }
    }

    /// Borrow the original registry identity and settled relation witness.
    /// Native admission refusal is already settled at construction and exposes
    /// its original admitted registry identity without invoking a producer.
    ///
    /// # Errors
    /// Refuses a running or failed read rather than publishing an unfinished witness.
    pub fn relation_identity(&self) -> Result<Option<&crate::RelationIdentity>, crate::EvalError> {
        match &self.execution {
            PreparedCursorExecution::Active(cursor) => {
                if !cursor.is_settled() {
                    return Err(crate::EvalError::config(
                        "relation identity requires successful cursor settlement",
                    ));
                }
                Ok(self.identity.as_ref().map(|identity| &identity.value))
            }
            PreparedCursorExecution::Refused { exhausted, .. } => Ok(Some(&exhausted.relations)),
        }
    }

    /// Actual retaining strategies selected so far; reading it never advances a source.
    #[must_use]
    pub fn buffering_report(&self) -> crate::eval::CursorBufferingReport {
        match &self.execution {
            PreparedCursorExecution::Active(cursor) => cursor.buffering_report(),
            PreparedCursorExecution::Refused { .. } => {
                crate::eval::CursorBufferingReport::default()
            }
        }
    }

    /// Committed governor counters, or the original admission-refusal snapshot.
    /// A live producer can retain pending charges until its unit is acknowledged;
    /// settlement includes performed work before publishing its receipt.
    #[must_use]
    pub fn governor_evidence(&self) -> Option<purrdf_core::GovernorEvidence> {
        match &self.execution {
            PreparedCursorExecution::Active(cursor) => cursor.governor_evidence(),
            PreparedCursorExecution::Refused { exhausted, .. } => Some(exhausted.evidence.clone()),
        }
    }
}

impl NativeSparqlEngine {
    /// Open an ordinary prepared SELECT over a transferable native view.
    ///
    /// This uses the same registry checks, prebinding rewrite, coordinate capture,
    /// query options and original workspace account as `query_prepared_view`.
    ///
    /// # Errors
    /// Refuses a different query form, registry disagreement, admission failure,
    /// invalid substitution or source preparation failure.
    pub fn open_select_cursor<'q, 'd: 'q, D>(
        &'d self,
        dataset: &'d D,
        prepared: &'q PreparedQuery,
        substitutions: &[(String, TermValue)],
        options: &QueryOptions<'d, 'd>,
    ) -> Result<
        PreparedSelectCursor<
            'q,
            'd,
            D,
            impl WorkspaceReservation<Error = D::ReadError> + Send + 'd,
        >,
        RdfDiagnostic,
    >
    where
        D: CursorDatasetView<ReadError = std::convert::Infallible>,
    {
        self.open_select_cursor_input(dataset, prepared, substitutions, options, None, None)
    }

    /// Open the same prepared SELECT with original per-row evidence and a native
    /// semantic answer request. Read `answer_status` after successful settlement.
    ///
    /// # Errors
    /// Preserves the original admission, declaration and query failures.
    pub fn open_select_cursor_evidence<'q, 'd: 'q, D>(
        &'d self,
        dataset: &'d D,
        prepared: &'q PreparedQuery,
        substitutions: &[(String, TermValue)],
        options: &QueryOptions<'d, 'd>,
        source: &'d dyn crate::EvidenceSource<D::Id>,
        answer: purrdf_core::evidence::AnswerRequest,
    ) -> Result<
        PreparedSelectCursor<
            'q,
            'd,
            D,
            impl WorkspaceReservation<Error = D::ReadError> + Send + 'd,
        >,
        RdfDiagnostic,
    >
    where
        D: CursorDatasetView<ReadError = std::convert::Infallible>,
    {
        self.open_select_cursor_input(
            dataset,
            prepared,
            substitutions,
            options,
            Some(source),
            Some(answer),
        )
    }

    fn open_select_cursor_input<'q, 'd: 'q, D>(
        &'d self,
        dataset: &'d D,
        prepared: &'q PreparedQuery,
        substitutions: &[(String, TermValue)],
        options: &QueryOptions<'d, 'd>,
        source: Option<&'d dyn crate::EvidenceSource<D::Id>>,
        answer: Option<purrdf_core::evidence::AnswerRequest>,
    ) -> Result<
        PreparedSelectCursor<
            'q,
            'd,
            D,
            impl WorkspaceReservation<Error = D::ReadError> + Send + 'd,
        >,
        RdfDiagnostic,
    >
    where
        D: CursorDatasetView<ReadError = std::convert::Infallible>,
    {
        let publication = crate::user_fn::RefusalPublication::default();
        let (cursor, coordinates, workspace, permit) =
            checked_query_read(dataset, &publication, || {
                let workspace = bounded_workspace::reserve_cursor(dataset)
                    .map_err(bounded_workspace::AdmissionError::into_diagnostic)?;
                RequestParameters::none().check_admitted(
                    prepared,
                    options,
                    &workspace.capability(),
                )?;
                let (cursor, coordinates) = self.select_cursor_admitted(
                    dataset,
                    prepared,
                    substitutions,
                    options,
                    &workspace,
                    &publication,
                    source,
                    answer,
                    None,
                )?;
                let (workspace, permit) = workspace.into_parts();
                Ok::<_, EvaluationFailure>((cursor, coordinates, workspace, permit))
            })?;
        Ok(PreparedSelectCursor {
            execution: PreparedCursorExecution::Active(cursor),
            coordinates,
            publication,
            identity: None,
            workspace,
            _permit: permit,
        })
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "one shared cursor ingress retains the original publication and workspace authorities"
    )]
    fn select_cursor_admitted<'q, 'd: 'q, D: CursorDatasetView>(
        &'d self,
        dataset: &'d D,
        prepared: &'q PreparedQuery,
        substitutions: &[(String, TermValue)],
        options: &QueryOptions<'d, 'd>,
        workspace: &crate::workspace::QueryWorkspace<D::ReadError>,
        publication: &crate::user_fn::RefusalPublication,
        source: Option<&'d dyn crate::EvidenceSource<D::Id>>,
        answer: Option<purrdf_core::evidence::AnswerRequest>,
        state: Option<crate::workspace::SharedWorkspace<crate::governor::GovernorState>>,
    ) -> Result<(SelectCursor<'q, 'd, D>, QueryCoordinates), EvaluationFailure> {
        let mut context = match state {
            Some(state) => self.governed_ctx(dataset, state, options, workspace)?,
            None => self.query_ctx(dataset, options, workspace)?,
        };
        context.options.force_sequential |=
            super::Sequencing::for_view::<D>() == super::Sequencing::Sequential;
        if let Some(owner) = options.user_function_admission {
            context = context.with_user_function_admission(Some(publication.admission(owner)));
        }
        let coordinates = coordinates::capture_query(
            &context,
            prepared,
            Prebindings::Owned(substitutions),
            options,
            coordinates::Consumer::Retained,
        )?;
        if let Some(source) = source {
            context = context.with_evidence_source(source)?;
        }
        context.evidence_verdict = answer;
        let (query, kept) = if substitutions.is_empty() {
            (
                CursorQuery::Borrowed(prepared.query()),
                Some(&prepared.plan),
            )
        } else {
            let query = runtime_substitution(
                prepared.query(),
                Prebindings::Owned(substitutions),
                &workspace.capability(),
            )?;
            (
                CursorQuery::Owned(crate::workspace::NativeSharedWorkspace::new_admitted(
                    query,
                    &workspace.capability(),
                )?),
                None,
            )
        };
        let mut cursor = SelectCursor::from_admitted_query(query, kept, context)?;
        cursor.set_prepared_layout(prepared)?;
        Ok((cursor, coordinates))
    }
}

/// A prepared SELECT whose native reads retain their typed operational status.
pub struct FallibleSelectCursor<'q, 'd, D: purrdf_core::FallibleDatasetView + Sync, R> {
    inner: PreparedSelectCursor<'q, 'd, D, R>,
    dataset: &'d D,
    failure: Option<crate::FallibleSparqlError<D::Error, D::Evidence>>,
    settlement: Option<crate::RetainedEvidence<D::Evidence>>,
}

purrdf_hash::debug_non_exhaustive!(['q, 'd: 'q, D: purrdf_core::FallibleDatasetView + Sync, R]
    FallibleSelectCursor<'q, 'd, D, R> {});

impl<'d, D: purrdf_core::FallibleDatasetView + Sync, R> FallibleSelectCursor<'_, 'd, D, R> {
    /// Original projected columns, including an empty answer's header.
    /// This accessor performs no source read or operational checkpoint.
    #[must_use]
    pub fn schema(&self) -> &crate::solution::VarSchema {
        self.inner.schema()
    }

    /// Read a row only after the source certifies the actual read boundary.
    ///
    /// # Errors
    /// Preserves the first typed source failure and its original evidence. A
    /// healthy source preserves the original evaluation diagnostic instead.
    pub fn next_row(&mut self) -> super::FallibleControlResult<D, Option<SelectRow<'_, 'd, D>>> {
        if let Some(failure) = &self.failure {
            return Err(failure.clone());
        }
        if self.settlement.is_some() {
            return Ok(None);
        }
        let next = self
            .inner
            .prepare_next_row()
            .map_err(EvaluationFailure::from);
        let checked = self.inner.publication.finish(
            super::finish_fallible_read(self.dataset, next, Some(&self.inner.workspace)),
            |error| matches!(error, crate::FallibleSparqlError::Query { .. }),
        );
        match checked {
            Ok((ordinal, _evidence)) => Ok(ordinal.map(|ordinal| self.inner.prepared_row(ordinal))),
            Err(error) => {
                self.inner.abort_read();
                self.failure = Some(error.clone());
                Err(error)
            }
        }
    }

    /// Settle the original query prefix and certify its final operational status.
    /// Repeated settlement reuses that original checkpoint.
    ///
    /// # Errors
    /// Returns the first original source or query failure instead of a successful receipt.
    pub fn settle(
        &mut self,
    ) -> super::FallibleControlResult<D, FallibleSelectSettlement<D::Evidence>> {
        if let Some(failure) = &self.failure {
            return Err(failure.clone());
        }
        if let Some(evidence) = &self.settlement {
            return Ok(FallibleSelectSettlement {
                query: self
                    .inner
                    .settle_original()
                    .expect("an originally settled cursor remains settled"),
                evidence: evidence.clone(),
            });
        }
        let query = self
            .inner
            .settle_original()
            .map_err(EvaluationFailure::from);
        let checked = self.inner.publication.finish(
            super::finish_fallible_read(self.dataset, query, Some(&self.inner.workspace)),
            |error| matches!(error, crate::FallibleSparqlError::Query { .. }),
        );
        match checked {
            Ok((query, evidence)) => {
                let evidence = crate::RetainedEvidence::backend(evidence);
                self.settlement = Some(evidence.clone());
                Ok(FallibleSelectSettlement { query, evidence })
            }
            Err(error) => {
                self.inner.abort_read();
                self.failure = Some(error.clone());
                Err(error)
            }
        }
    }

    /// Borrow native semantic status after the original final source checkpoint.
    ///
    /// # Errors
    /// Refuses a failed or unsettled read and preserves native count failures.
    pub fn answer_status(&self) -> Result<crate::eval::CursorAnswerStatus<'_>, crate::EvalError> {
        if self.failure.is_some() || self.settlement.is_none() {
            return Err(crate::EvalError::config(
                "native answer status requires successful source settlement",
            ));
        }
        self.inner.original_answer_status()
    }

    /// Whether evaluation has settled completely and the source remains healthy.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.failure.is_none() && self.inner.complete_read()
    }

    /// Borrow the registry identity and original witness after source settlement.
    ///
    /// # Errors
    /// Refuses a failed or unfinished read instead of publishing partial evidence.
    pub fn relation_identity(&self) -> Result<Option<&crate::RelationIdentity>, crate::EvalError> {
        if self.failure.is_some() || self.settlement.is_none() {
            return Err(crate::EvalError::config(
                "relation identity requires successful source settlement",
            ));
        }
        self.inner.relation_identity()
    }

    /// Actual selected retaining strategies, without another source checkpoint.
    #[must_use]
    pub fn buffering_report(&self) -> crate::eval::CursorBufferingReport {
        self.inner.buffering_report()
    }

    /// Committed governor counters at the current read boundary.
    /// Pending producer charges are included when the read is settled.
    #[must_use]
    pub fn governor_evidence(&self) -> Option<purrdf_core::GovernorEvidence> {
        self.inner.governor_evidence()
    }

    /// The coordinates captured by the original prepared execution.
    #[must_use]
    pub const fn coordinates(&self) -> &QueryCoordinates {
        &self.inner.coordinates
    }
}

impl NativeSparqlEngine {
    /// Open an ordinary prepared SELECT over an operationally fallible source.
    /// Both original reporting and execution permits remain held through the
    /// cursor's final status checkpoint and early-drop cleanup.
    ///
    /// # Errors
    /// Returns the original admission, source or query failure with backend evidence.
    pub fn open_select_cursor_fallible_view<'q, 'd: 'q, D>(
        &'d self,
        dataset: &'d D,
        prepared: &'q PreparedQuery,
        substitutions: &[(String, TermValue)],
        options: &QueryOptions<'d, 'd>,
    ) -> Result<
        FallibleSelectCursor<'q, 'd, D, impl Send + 'd>,
        crate::FallibleSparqlError<D::Error, D::Evidence>,
    >
    where
        D: CursorDatasetView + purrdf_core::FallibleDatasetView,
    {
        self.open_select_cursor_fallible_input(
            dataset,
            prepared,
            substitutions,
            options,
            None,
            None,
            None,
        )
    }

    /// Open a fallible prepared SELECT with original per-row and answer evidence.
    /// Its semantic status is available only after a successful source settlement.
    ///
    /// # Errors
    /// Returns the first original source, declaration, admission or query failure.
    pub fn open_select_cursor_evidence_fallible_view<'q, 'd: 'q, D>(
        &'d self,
        dataset: &'d D,
        prepared: &'q PreparedQuery,
        substitutions: &[(String, TermValue)],
        options: &QueryOptions<'d, 'd>,
        source: &'d dyn crate::EvidenceSource<D::Id>,
        answer: purrdf_core::evidence::AnswerRequest,
    ) -> Result<
        FallibleSelectCursor<'q, 'd, D, impl Send + 'd>,
        crate::FallibleSparqlError<D::Error, D::Evidence>,
    >
    where
        D: CursorDatasetView + purrdf_core::FallibleDatasetView,
    {
        self.open_select_cursor_fallible_input(
            dataset,
            prepared,
            substitutions,
            options,
            Some(source),
            Some(answer),
            None,
        )
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "one fallible ingress preserves the original permits for both governor and evidence selections"
    )]
    fn open_select_cursor_fallible_input<'q, 'd: 'q, D>(
        &'d self,
        dataset: &'d D,
        prepared: &'q PreparedQuery,
        substitutions: &[(String, TermValue)],
        options: &QueryOptions<'d, 'd>,
        source: Option<&'d dyn crate::EvidenceSource<D::Id>>,
        answer: Option<purrdf_core::evidence::AnswerRequest>,
        governors: Option<&crate::QueryGovernors>,
    ) -> Result<
        FallibleSelectCursor<'q, 'd, D, impl Send + 'd>,
        crate::FallibleSparqlError<D::Error, D::Evidence>,
    >
    where
        D: CursorDatasetView + purrdf_core::FallibleDatasetView,
    {
        super::preflight_fallible_view(dataset)?;
        let reporting = bounded_workspace::reserve_cursor_reporting(dataset)
            .map_err(|failure| super::fallible_admission_failure(dataset, failure))?;
        let owner = reporting
            .execution()
            .capture_direct_errors()
            .map_err(|error| {
                super::fallible_admission_failure(
                    dataset,
                    bounded_workspace::AdmissionError::from(error),
                )
            })?;
        let execution = bounded_workspace::reserve_cursor_execution(dataset, owner)
            .map_err(|failure| super::fallible_admission_failure(dataset, failure))?;
        let publication = crate::user_fn::RefusalPublication::default();
        let opened = (|| match governors {
            Some(governors) => {
                let state = crate::workspace::SharedWorkspace::new_admitted(
                    crate::governor::GovernorState::with_workspace(
                        governors,
                        execution.capability(),
                    ),
                    &execution.capability(),
                )?;
                self.select_cursor_governed_admitted(
                    dataset,
                    prepared,
                    substitutions,
                    options,
                    state,
                    &execution,
                    &publication,
                    source,
                    answer,
                )
                .map(|(execution, coordinates, identity)| (execution, coordinates, Some(identity)))
            }
            None => {
                RequestParameters::none().check_admitted(
                    prepared,
                    options,
                    &execution.capability(),
                )?;
                self.select_cursor_admitted(
                    dataset,
                    prepared,
                    substitutions,
                    options,
                    &execution,
                    &publication,
                    source,
                    answer,
                    None,
                )
                .map(|(cursor, coordinates)| {
                    (PreparedCursorExecution::Active(cursor), coordinates, None)
                })
            }
        })();
        let ((cursor_execution, coordinates, identity), _evidence) = publication.finish(
            super::finish_fallible_read(dataset, opened, Some(&execution)),
            |error| matches!(error, crate::FallibleSparqlError::Query { .. }),
        )?;
        let (workspace, execution_permit) = execution.into_parts();
        Ok(FallibleSelectCursor {
            inner: PreparedSelectCursor {
                execution: cursor_execution,
                coordinates,
                publication,
                identity,
                workspace,
                _permit: Some((reporting, execution_permit)),
            },
            dataset,
            failure: None,
            settlement: None,
        })
    }
}

/// One final query receipt beside its original backend operational evidence.
#[derive(Debug, Clone)]
pub struct FallibleSelectSettlement<E> {
    /// The delivered partitions and original completion certificate.
    pub query: crate::eval::SelectSettlement,
    /// The source's final ready checkpoint, retained without re-reading it.
    pub evidence: crate::RetainedEvidence<E>,
}

impl NativeSparqlEngine {
    /// Open a prepared SELECT under this execution's original caller governors.
    /// Admission refusal opens no producer and retains its typed certificate;
    /// active reads settle with the original execution counters and bound.
    ///
    /// # Errors
    /// Returns the original registry, admission, source or evaluation failure.
    pub fn open_select_cursor_governed_view<'q, 'd: 'q, D>(
        &'d self,
        dataset: &'d D,
        prepared: &'q PreparedQuery,
        substitutions: &[(String, TermValue)],
        options: &QueryOptions<'d, 'd>,
        governors: &crate::QueryGovernors,
    ) -> Result<PreparedSelectCursor<'q, 'd, D, impl Send + 'd>, RdfDiagnostic>
    where
        D: CursorDatasetView<ReadError = std::convert::Infallible>,
    {
        self.open_select_cursor_governed_input(
            dataset,
            prepared,
            substitutions,
            options,
            governors,
            None,
            None,
        )
    }

    /// Open the governed SELECT with original row evidence and answer status.
    /// Unread or withheld work remains explicit in the final receipt.
    ///
    /// # Errors
    /// Preserves original governor admission, declaration and evaluation failures.
    #[expect(
        clippy::too_many_arguments,
        reason = "the caller selects both execution governors and the original evidence question"
    )]
    pub fn open_select_cursor_governed_evidence_view<'q, 'd: 'q, D>(
        &'d self,
        dataset: &'d D,
        prepared: &'q PreparedQuery,
        substitutions: &[(String, TermValue)],
        options: &QueryOptions<'d, 'd>,
        governors: &crate::QueryGovernors,
        source: &'d dyn crate::EvidenceSource<D::Id>,
        answer: purrdf_core::evidence::AnswerRequest,
    ) -> Result<PreparedSelectCursor<'q, 'd, D, impl Send + 'd>, RdfDiagnostic>
    where
        D: CursorDatasetView<ReadError = std::convert::Infallible>,
    {
        self.open_select_cursor_governed_input(
            dataset,
            prepared,
            substitutions,
            options,
            governors,
            Some(source),
            Some(answer),
        )
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "one governed ingress retains the independent caller budgets and native evidence request"
    )]
    fn open_select_cursor_governed_input<'q, 'd: 'q, D>(
        &'d self,
        dataset: &'d D,
        prepared: &'q PreparedQuery,
        substitutions: &[(String, TermValue)],
        options: &QueryOptions<'d, 'd>,
        governors: &crate::QueryGovernors,
        source: Option<&'d dyn crate::EvidenceSource<D::Id>>,
        answer: Option<purrdf_core::evidence::AnswerRequest>,
    ) -> Result<PreparedSelectCursor<'q, 'd, D, impl Send + 'd>, RdfDiagnostic>
    where
        D: CursorDatasetView<ReadError = std::convert::Infallible>,
    {
        let publication = crate::user_fn::RefusalPublication::default();
        let (execution, coordinates, identity, workspace, reporting) =
            checked_query_read(dataset, &publication, || {
                let reporting = bounded_workspace::reserve_cursor_reporting(dataset)
                    .map_err(bounded_workspace::AdmissionError::into_diagnostic)?;
                let workspace = reporting.execution();
                let state = crate::workspace::SharedWorkspace::new_admitted(
                    crate::governor::GovernorState::with_workspace(
                        governors,
                        workspace.capability(),
                    ),
                    &workspace.capability(),
                )?;
                let (execution, coordinates, identity) = self.select_cursor_governed_admitted(
                    dataset,
                    prepared,
                    substitutions,
                    options,
                    state,
                    &workspace,
                    &publication,
                    source,
                    answer,
                )?;
                Ok::<_, EvaluationFailure>((execution, coordinates, identity, workspace, reporting))
            })?;
        Ok(PreparedSelectCursor {
            execution,
            coordinates,
            publication,
            identity: Some(identity),
            workspace,
            _permit: Some(reporting),
        })
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "the shared admitted body retains original state, publication and evidence authorities"
    )]
    fn select_cursor_governed_admitted<'q, 'd: 'q, D: CursorDatasetView>(
        &'d self,
        dataset: &'d D,
        prepared: &'q PreparedQuery,
        substitutions: &[(String, TermValue)],
        options: &QueryOptions<'d, 'd>,
        state: crate::workspace::SharedWorkspace<crate::governor::GovernorState>,
        workspace: &crate::workspace::QueryWorkspace<D::ReadError>,
        publication: &crate::user_fn::RefusalPublication,
        source: Option<&'d dyn crate::EvidenceSource<D::Id>>,
        answer: Option<purrdf_core::evidence::AnswerRequest>,
    ) -> Result<
        (
            PreparedCursorExecution<'q, 'd, D>,
            QueryCoordinates,
            super::AdmittedRelationIdentity,
        ),
        EvaluationFailure,
    > {
        RequestParameters::none().check_admitted(prepared, options, &workspace.capability())?;
        if !matches!(
            prepared.query(),
            purrdf_sparql_algebra::Query::Select { .. }
        ) {
            return Err(
                crate::EvalError::config("an ordinary row cursor requires a SELECT query").into(),
            );
        }
        let mut identity = super::relation_identity(
            prepared,
            options.property_functions(),
            &workspace.capability(),
        )?;
        if let Some(refused) = self.admit_refusal(
            dataset,
            prepared.query(),
            options.property_functions(),
            &state,
            &mut identity.value,
            workspace,
        ) {
            let exhausted = refused?;
            let schema = crate::eval::syntactic_schema_admitted(
                super::query_pattern(prepared.query()),
                &workspace.capability(),
            )?;
            let schema = crate::blank_scope::without_joined_blanks_admitted(
                prepared.restore_solution_layout(
                    crate::solution::SolutionSeq::<D::Id>::empty(schema),
                    &workspace.capability(),
                )?,
                &workspace.capability(),
            )?
            .schema;
            return Ok((
                PreparedCursorExecution::Refused {
                    exhausted,
                    answer_requested: answer.is_some(),
                    schema,
                },
                QueryCoordinates::NotStarted,
                identity,
            ));
        }
        let (cursor, coordinates) = self.select_cursor_admitted(
            dataset,
            prepared,
            substitutions,
            options,
            workspace,
            publication,
            source,
            answer,
            Some(state),
        )?;
        Ok((
            PreparedCursorExecution::Active(cursor),
            coordinates,
            identity,
        ))
    }
}

impl NativeSparqlEngine {
    /// Open the same governed SELECT while retaining original fallible read status.
    /// Source failures retain precedence over admission refusal and query truncation.
    ///
    /// # Errors
    /// Returns original typed operational or query failures with backend evidence.
    pub fn open_select_cursor_governed_fallible_view<'q, 'd: 'q, D>(
        &'d self,
        dataset: &'d D,
        prepared: &'q PreparedQuery,
        substitutions: &[(String, TermValue)],
        options: &QueryOptions<'d, 'd>,
        governors: &crate::QueryGovernors,
    ) -> Result<
        FallibleSelectCursor<'q, 'd, D, impl Send + 'd>,
        crate::FallibleSparqlError<D::Error, D::Evidence>,
    >
    where
        D: CursorDatasetView + purrdf_core::FallibleDatasetView,
    {
        self.open_select_cursor_fallible_input(
            dataset,
            prepared,
            substitutions,
            options,
            None,
            None,
            Some(governors),
        )
    }

    /// Open the governed fallible SELECT with its original semantic answer request.
    ///
    /// # Errors
    /// Preserves operational precedence and original admission/declaration failures.
    #[expect(
        clippy::too_many_arguments,
        reason = "the caller supplies both governors and the original native evidence request"
    )]
    pub fn open_select_cursor_governed_evidence_fallible_view<'q, 'd: 'q, D>(
        &'d self,
        dataset: &'d D,
        prepared: &'q PreparedQuery,
        substitutions: &[(String, TermValue)],
        options: &QueryOptions<'d, 'd>,
        governors: &crate::QueryGovernors,
        source: &'d dyn crate::EvidenceSource<D::Id>,
        answer: purrdf_core::evidence::AnswerRequest,
    ) -> Result<
        FallibleSelectCursor<'q, 'd, D, impl Send + 'd>,
        crate::FallibleSparqlError<D::Error, D::Evidence>,
    >
    where
        D: CursorDatasetView + purrdf_core::FallibleDatasetView,
    {
        self.open_select_cursor_fallible_input(
            dataset,
            prepared,
            substitutions,
            options,
            Some(source),
            Some(answer),
            Some(governors),
        )
    }
}
