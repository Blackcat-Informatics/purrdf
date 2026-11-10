// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Incremental native answer status under the original evidence admission.

use crate::evidence_exec::{EvidenceAdmission, StatusFailure, answer_failure, observe_answer_rows};
use crate::solution::{RetainedRow, SolutionSeq};
use crate::{EvalError, RowEvidence, WorkspaceCapability};
use purrdf_core::ViewTermId;
use purrdf_core::evidence::{
    AnswerAccumulator, AnswerError, AnswerRequest, AnswerTier, EvidencePartition, WitnessVerdict,
};

pub(super) struct CursorAnswer {
    answer: AnswerAccumulator<EvidenceAdmission>,
    admission: EvidenceAdmission,
    workspace: WorkspaceCapability,
    verdict: bool,
    observed_stop: bool,
    settled: bool,
}

impl CursorAnswer {
    pub(super) fn new(
        request: AnswerRequest,
        workspace: &WorkspaceCapability,
    ) -> Result<Self, EvalError> {
        let admission = EvidenceAdmission::new(workspace)?;
        Ok(Self {
            answer: AnswerAccumulator::with_admission(request, admission.clone()),
            admission,
            workspace: workspace.clone(),
            verdict: request.tier == AnswerTier::Verdict,
            observed_stop: false,
            settled: false,
        })
    }

    fn failure(&self, error: StatusFailure) -> EvalError {
        match error {
            StatusFailure::Answer(AnswerError::Storage(error)) => self.admission.storage_error(
                error,
                "native cursor answer reason counts",
                &self.workspace,
            ),
            error => answer_failure(error, &self.workspace),
        }
    }

    pub(super) const fn is_verdict(&self) -> bool {
        self.verdict
    }

    pub(super) fn witness_stop(&self) -> Option<WitnessVerdict> {
        self.answer.witness_stop()
    }

    fn observe_search(&mut self, search: &RowEvidence) -> Result<(), EvalError> {
        search
            .with_record(|record| {
                self.answer
                    .observe_search(record, search.permits_closed_absence())
            })?
            .map_err(|error| self.failure(StatusFailure::Answer(error)))
    }

    pub(super) fn begin_block(&mut self, search: Option<&RowEvidence>) -> Result<(), EvalError> {
        if self.verdict
            && let Some(search) = search
        {
            self.observe_search(search)?;
        }
        Ok(())
    }

    pub(super) fn observe_row<I: ViewTermId>(
        &mut self,
        row: &RetainedRow<I>,
    ) -> Result<(), EvalError> {
        if self.observed_stop {
            return Ok(());
        }
        let partition = if row.evidence().is_undecided() {
            EvidencePartition::Undecided
        } else {
            EvidencePartition::Established
        };
        row.evidence()
            .with_record(|record| self.answer.observe(record, partition))?
            .map_err(|error| self.failure(StatusFailure::Answer(error)))?;
        self.observed_stop = self.answer.witness_stop().is_some();
        Ok(())
    }

    pub(super) fn observe_block<I: ViewTermId>(
        &mut self,
        rows: &SolutionSeq<I>,
        search: Option<&RowEvidence>,
    ) -> Result<(), EvalError> {
        self.begin_block(search)?;
        observe_answer_rows(&mut self.answer, rows).map_err(|error| self.failure(error))?;
        self.observed_stop = self.answer.witness_stop().is_some();
        Ok(())
    }

    pub(super) fn finish(
        &mut self,
        search: &RowEvidence,
        exhausted: bool,
    ) -> Result<(), EvalError> {
        self.observe_search(search)?;
        if exhausted || self.answer.witness_stop().is_some() {
            self.answer
                .finish(exhausted)
                .map_err(|error| self.failure(StatusFailure::Answer(error)))?;
            self.settled = true;
        }
        Ok(())
    }

    pub(super) fn status(&self) -> Result<super::CursorAnswerStatus<'_>, EvalError> {
        if !self.settled {
            return Ok(super::CursorAnswerStatus::UnreadSuffix);
        }
        self.answer
            .status()
            .map(super::CursorAnswerStatus::Settled)
            .map_err(|error| self.failure(StatusFailure::Answer(error)))
    }
}
