// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native per-invocation hard-constraint accounting and original row evidence.

use std::sync::Mutex;

use purrdf_core::{
    TermValue,
    query_scope::{ConstraintCounts, ConstraintDecision, ScopeCounters, ScopeError},
};
use purrdf_hash::hex::Digest32;

use crate::{
    AdmittedVec, CandidateMembership, CandidateScope, EvalError, NativeDiagnostic,
    NativeDiagnosticKind, QueryEvidence, WorkspaceCapability, evidence_exec::EvidenceAdmission,
};

/// One caller-declared constraint over the original candidate key.
///
/// Its identity names the complete original constraint value, independently of
/// rankings and candidates. Missing data is an undecided decision with a reason
/// declared in the supplied query owner; a failed read is an operational error.
pub trait CandidateConstraint: core::fmt::Debug + Sync {
    /// Canonical content identity of the complete original constraint.
    fn identity(&self) -> Digest32;

    /// Decide this constraint before the producer applies its depth.
    ///
    /// # Errors
    /// Returns the original data-read, cancellation or physical failure.
    fn decide(
        &self,
        candidate: &TermValue,
        evidence: &QueryEvidence,
        workspace: &WorkspaceCapability,
    ) -> Result<ConstraintDecision, EvalError>;
}

/// A native original owner for one stratum's complete constraint accounting.
///
/// The producer borrows it while generating candidates. Returned metadata owns
/// the original query dictionary and can outlive this borrowed evaluator. The
/// counter payload is destroyed before its original admission grant.
#[derive(Debug)]
pub struct NativeCandidateScope<'a> {
    counts: Mutex<ScopeCounters<EvidenceAdmission>>,
    constraints: AdmittedVec<&'a dyn CandidateConstraint>,
    admission: EvidenceAdmission,
    evidence: QueryEvidence,
    workspace: WorkspaceCapability,
}

fn scope_error<E>(
    error: ScopeError<E>,
    admission: &EvidenceAdmission,
    workspace: &WorkspaceCapability,
    callback: impl FnOnce(E) -> EvalError,
) -> EvalError {
    match error {
        ScopeError::Callback(original) => callback(original),
        ScopeError::Storage(storage) => {
            admission.storage_error(storage, "native candidate constraint accounting", workspace)
        }
        ScopeError::Aborted => NativeDiagnostic::error_cause(
            NativeDiagnosticKind::Internal,
            crate::error::EvaluatorInvariant::CandidateAccountingAborted,
            workspace,
        ),
        ScopeError::NonCanonicalConstraints => NativeDiagnostic::error(
            NativeDiagnosticKind::Config,
            "candidate constraints require sorted unique content identities",
            workspace,
        ),
    }
}

impl<'a> NativeCandidateScope<'a> {
    /// Admit canonical constraints before the producer generates a candidate.
    ///
    /// # Errors
    /// Refuses duplicate/out-of-order identities or original native admission.
    pub fn new(
        constraints: &[&'a dyn CandidateConstraint],
        workspace: &WorkspaceCapability,
    ) -> Result<Self, EvalError> {
        let admission = EvidenceAdmission::new(workspace)?;
        let mut retained = AdmittedVec::with_capacity(constraints.len(), workspace)?;
        let mut identities = AdmittedVec::with_capacity(constraints.len(), workspace)?;
        for &constraint in constraints {
            retained.push(constraint)?;
            identities.push(constraint.identity())?;
        }
        let counts = ScopeCounters::with_admission(&identities, admission.clone())
            .map_err(|error| scope_error(error, &admission, workspace, |never| match never {}))?;
        Ok(Self {
            counts: Mutex::new(counts),
            constraints: retained,
            admission,
            evidence: QueryEvidence::new(workspace)?,
            workspace: workspace.clone(),
        })
    }

    /// Borrow complete counts without cloning or detaching the native payload.
    ///
    /// # Errors
    /// Refuses an aborted invocation or a poisoned original counter owner.
    pub fn with_counts<R>(
        &self,
        read: impl FnOnce(&[ConstraintCounts]) -> R,
    ) -> Result<R, EvalError> {
        let counts = self.counts.lock().map_err(|_| {
            NativeDiagnostic::error_cause(
                NativeDiagnosticKind::Internal,
                crate::error::EvaluatorInvariant::CandidateAccountingPoisoned,
                &self.workspace,
            )
        })?;
        let complete = counts.counts().map_err(|error| {
            scope_error(
                error,
                &self.admission,
                &self.workspace,
                |never| match never {},
            )
        })?;
        let result = read(complete);
        drop(counts);
        Ok(result)
    }
}

impl CandidateScope for NativeCandidateScope<'_> {
    fn evidence_owner(&self) -> Option<&QueryEvidence> {
        Some(&self.evidence)
    }

    fn decide(
        &self,
        candidate: &TermValue,
        workspace: &WorkspaceCapability,
    ) -> Result<CandidateMembership, EvalError> {
        let mut counts = self.counts.lock().map_err(|_| {
            NativeDiagnostic::error_cause(
                NativeDiagnosticKind::Internal,
                crate::error::EvaluatorInvariant::CandidateAccountingPoisoned,
                &self.workspace,
            )
        })?;
        let mut ordinal = 0;
        counts
            .evaluate(
                |_| {
                    if workspace.has_failed() {
                        return Err(EvalError::WorkspaceStopped);
                    }
                    let constraint = self.constraints[ordinal];
                    ordinal += 1;
                    constraint.decide(candidate, &self.evidence, workspace)
                },
                |decisions| {
                    if workspace.has_failed() {
                        return Err(EvalError::WorkspaceStopped);
                    }
                    if decisions
                        .iter()
                        .any(|(_, decision)| *decision == ConstraintDecision::Rejected)
                    {
                        return Ok(CandidateMembership::Rejected);
                    }
                    if decisions.is_empty() {
                        return Ok(CandidateMembership::Established(
                            crate::RowEvidence::default(),
                        ));
                    }
                    self.evidence
                        .undecided_reasons(decisions.iter().filter_map(|(_, decision)| {
                            match decision {
                                ConstraintDecision::Undecided(reason) => Some(*reason),
                                ConstraintDecision::Satisfied | ConstraintDecision::Rejected => {
                                    None
                                }
                            }
                        }))
                        .map(CandidateMembership::Undecided)
                },
            )
            .map_err(|error| {
                scope_error(
                    error,
                    &self.admission,
                    &self.workspace,
                    core::convert::identity,
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_core::{TermBox, evidence::ReasonDescription};

    #[derive(Debug)]
    struct Missing {
        cause: TermValue,
        concern: TermValue,
    }

    impl CandidateConstraint for Missing {
        fn identity(&self) -> Digest32 {
            Digest32::new([1; 32])
        }

        fn decide(
            &self,
            _: &TermValue,
            evidence: &QueryEvidence,
            _: &WorkspaceCapability,
        ) -> Result<ConstraintDecision, EvalError> {
            evidence
                .declare_reason(ReasonDescription {
                    cause: &self.cause,
                    concern: &self.concern,
                    constraint: None,
                    vault: None,
                    watermark: None,
                })
                .map(ConstraintDecision::Undecided)
        }
    }

    #[derive(Debug)]
    struct Other {
        fails: bool,
    }

    impl CandidateConstraint for Other {
        fn identity(&self) -> Digest32 {
            Digest32::new([2; 32])
        }

        fn decide(
            &self,
            _: &TermValue,
            _: &QueryEvidence,
            _: &WorkspaceCapability,
        ) -> Result<ConstraintDecision, EvalError> {
            if self.fails {
                Err(EvalError::AllocationFailed {
                    construct: "original constraint read",
                })
            } else {
                Ok(ConstraintDecision::Rejected)
            }
        }
    }

    #[test]
    fn original_unknown_counts_survive_another_constraint_rejection() {
        let candidate = TermValue::iri("https://example.org/record");
        let missing = Missing {
            cause: TermValue::iri("https://example.org/no-time"),
            concern: TermValue::Triple {
                s: TermBox::new(candidate.clone()),
                p: TermBox::new(TermValue::iri("https://example.org/time")),
                o: TermBox::new(TermValue::simple_literal("missing")),
            },
        };
        let other = Other { fails: false };
        let workspace = WorkspaceCapability::default();
        let scope = NativeCandidateScope::new(&[&missing, &other], &workspace).unwrap();
        for _ in 0..2279 {
            assert!(matches!(
                scope.decide(&candidate, &workspace).unwrap(),
                CandidateMembership::Rejected
            ));
        }
        scope
            .with_counts(|counts| {
                assert_eq!(counts[0].undecided, 2279);
                assert_eq!(counts[0].rejected, 0);
                assert_eq!(counts[1].rejected, 2279);
                assert_eq!(counts[1].undecided, 0);
            })
            .unwrap();
        let unknown = NativeCandidateScope::new(&[&missing], &workspace).unwrap();
        let CandidateMembership::Undecided(row) = unknown.decide(&candidate, &workspace).unwrap()
        else {
            panic!("missing data must remain undecided");
        };
        let source_claim = unknown
            .evidence_owner()
            .unwrap()
            .with_completeness(
                &crate::RowEvidence::default(),
                purrdf_core::evidence::SearchCompleteness::Undetermined,
                None,
            )
            .unwrap();
        let row = row
            .combine(
                &source_claim,
                purrdf_core::evidence::EvidenceCombination::Conjunction,
            )
            .expect("candidate and dataset declarations share the original owner");
        drop(unknown);
        assert!(row.is_undecided());
        row.with_record(|record| {
            assert_eq!(
                record.completeness(),
                purrdf_core::evidence::SearchCompleteness::Undetermined
            );
            assert_eq!(record.reasons().len(), 1);
        })
        .unwrap();
        let failure = Other { fails: true };
        let failed = NativeCandidateScope::new(&[&missing, &failure], &workspace).unwrap();
        assert!(matches!(
            failed.decide(&candidate, &workspace),
            Err(EvalError::AllocationFailed {
                construct: "original constraint read"
            })
        ));
        assert!(failed.with_counts(|_| ()).is_err());
    }
}
