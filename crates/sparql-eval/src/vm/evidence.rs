// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Sparse records at the original VM's value-stack positions.

use crate::error::EvaluatorInvariant;
use purrdf_core::{DatasetView, evidence::EvidenceCombination};

use super::compile::Op;
use crate::eval::EvalCtx;
use crate::evidence_exec::RowEvidence;
use crate::{AdmittedMap, AdmittedVec, EvalError, WorkspaceCapability};

pub(super) struct Records {
    values: AdmittedMap<usize, RowEvidence>,
    branches: AdmittedVec<(usize, usize, RowEvidence)>,
    suspended: Option<RowEvidence>,
    pub(super) result: Option<RowEvidence>,
}

impl Records {
    pub(super) fn admitted(workspace: &WorkspaceCapability) -> Self {
        Self {
            values: AdmittedMap::default(),
            branches: AdmittedVec::new(workspace),
            suspended: None,
            result: None,
        }
    }

    pub(super) fn clear(&mut self) {
        self.values.clear();
        self.branches.clear();
        self.suspended = None;
        self.result = None;
    }

    pub(super) fn begin<D: DatasetView + Sync>(
        &mut self,
        op: Op,
        depth: usize,
        ctx: &EvalCtx<'_, D>,
    ) -> Result<Option<RowEvidence>, EvalError> {
        let consumed = match op {
            Op::Const(_)
            | Op::Var(_)
            | Op::Bound(_)
            | Op::StrConst(_)
            | Op::StrNone
            | Op::PushUnbound
            | Op::Exists(_)
            | Op::Jmp(_) => 0,
            Op::EbvOf
            | Op::Not
            | Op::NotExists
            | Op::Branch { .. }
            | Op::CoalesceNext(_)
            | Op::InNeedle(_)
            | Op::InEnd
            | Op::UnaryPlus
            | Op::UnaryMinus
            | Op::ToStrArg
            | Op::ToStrLexical
            | Op::ToLangLexical => 1,
            Op::Equal
            | Op::SameTerm
            | Op::Cmp(_)
            | Op::InItem(_)
            | Op::Arith(_)
            | Op::StrPred(_)
            | Op::LangMatches => 2,
            Op::Triple { .. } => 3,
            Op::Regex { flags, .. } => {
                if flags {
                    3
                } else {
                    2
                }
            }
            Op::Kleene { n, .. } => n as usize,
            Op::Call { argc, .. } | Op::CallCustom { argc, .. } => argc as usize,
        };
        let start = depth.checked_sub(consumed).ok_or_else(|| {
            EvalError::Invariant(EvaluatorInvariant::VmEvidenceOperandsOutsideStack)
        })?;
        let law = if matches!(op, Op::Kleene { or: true, .. }) {
            EvidenceCombination::Alternatives
        } else {
            EvidenceCombination::Conjunction
        };
        let mut record: Option<RowEvidence> = None;
        for position in start..depth {
            if let Some(operand) = self.values.remove(&position) {
                record = Some(match record {
                    None => operand,
                    Some(previous) => previous.combine(&operand, law)?,
                });
            }
        }
        if matches!(op, Op::NotExists)
            && let Some(original) = record.as_ref()
        {
            return ctx.negative_evidence(original).map(Some);
        }
        Ok(record)
    }

    pub(super) fn finish(
        &mut self,
        op: Op,
        before: usize,
        after: usize,
        pc: usize,
        record: Option<RowEvidence>,
        workspace: &WorkspaceCapability,
    ) -> Result<(), EvalError> {
        if let Some(record) = record {
            match op {
                Op::Branch { end, .. } => {
                    self.branches.push((end as usize, before - 1, record))?;
                }
                Op::CoalesceNext(_) if after < before => {}
                _ => {
                    let position = after.checked_sub(1).ok_or_else(|| {
                        EvalError::Invariant(EvaluatorInvariant::VmEvidenceResultSlotMissing)
                    })?;
                    self.attach(position, record, workspace)?;
                }
            }
        }
        self.settle(pc, workspace)
    }

    fn settle(&mut self, pc: usize, workspace: &WorkspaceCapability) -> Result<(), EvalError> {
        while self.branches.last().is_some_and(|branch| pc >= branch.0) {
            let (_, position, condition) = self.branches.pop().ok_or_else(|| {
                EvalError::Invariant(EvaluatorInvariant::VmConditionalEvidenceMissing)
            })?;
            self.attach(position, condition, workspace)?;
        }
        Ok(())
    }

    pub(super) fn attach(
        &mut self,
        position: usize,
        record: RowEvidence,
        workspace: &WorkspaceCapability,
    ) -> Result<(), EvalError> {
        let record = match self.values.remove(&position) {
            None => record,
            Some(previous) => previous.combine(&record, EvidenceCombination::Conjunction)?,
        };
        self.values.insert_admitted(position, record, workspace)?;
        Ok(())
    }

    pub(super) fn suspend(&mut self, record: Option<RowEvidence>) {
        self.suspended = record;
    }

    pub(super) fn resume(
        &mut self,
        position: usize,
        record: Option<RowEvidence>,
        pc: usize,
        workspace: &WorkspaceCapability,
    ) -> Result<(), EvalError> {
        if let Some(record) = self.suspended.take() {
            self.attach(position, record, workspace)?;
        }
        if let Some(record) = record {
            self.attach(position, record, workspace)?;
        }
        self.settle(pc, workspace)
    }

    pub(super) fn complete(&mut self, position: usize) {
        self.result = self.values.remove(&position);
    }
}
