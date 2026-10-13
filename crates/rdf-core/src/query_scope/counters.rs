// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Per-constraint accounting at the candidate-generation boundary.

use crate::evidence::ReasonId;
use purrdf_hash::hex::Digest32;
use purrdf_lex::allocation::{Admission, Memory, StorageError};

/// One declared constraint's decision about an original candidate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConstraintDecision {
    /// The data establishes membership.
    Satisfied,
    /// The data establishes non-membership.
    Rejected,
    /// The data cannot decide, with the original content-identified reason.
    Undecided(ReasonId),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TermBox, TermValue, evidence::ReasonDescription};
    use core::convert::Infallible;
    use purrdf_lex::allocation::Resident;

    #[test]
    fn unknown_time_is_counted_separately_even_when_place_rejects() {
        let cause = TermValue::iri("https://example.org/missing-time");
        let concern = TermValue::Triple {
            s: TermBox::new(TermValue::iri("https://example.org/constraint")),
            p: TermBox::new(TermValue::iri("https://example.org/time")),
            o: TermBox::new(TermValue::simple_literal("required")),
        };
        let reason = ReasonDescription {
            cause: &cause,
            concern: &concern,
            constraint: None,
            vault: None,
            watermark: None,
        }
        .identity(&mut Memory::new(&mut Resident))
        .unwrap();
        let time = Digest32::new([1; 32]);
        let place = Digest32::new([2; 32]);
        let mut scope = ScopeCounters::with_admission(&[time, place], Resident).unwrap();
        let retained = scope.live;
        for _ in 0..2279 {
            scope
                .evaluate(
                    |constraint| {
                        Ok::<_, Infallible>(if constraint == time {
                            ConstraintDecision::Undecided(reason)
                        } else {
                            ConstraintDecision::Rejected
                        })
                    },
                    |decisions| {
                        assert_eq!(
                            decisions,
                            &[
                                (0, ConstraintDecision::Undecided(reason)),
                                (1, ConstraintDecision::Rejected)
                            ]
                        );
                        Ok::<_, Infallible>(())
                    },
                )
                .unwrap();
            assert_eq!(scope.live, retained);
        }
        let counts = scope.counts().unwrap();
        assert_eq!(
            (counts[0].satisfied, counts[0].rejected, counts[0].undecided),
            (0, 0, 2279)
        );
        assert_eq!(
            (counts[1].satisfied, counts[1].rejected, counts[1].undecided),
            (0, 2279, 0)
        );
    }

    #[test]
    fn operational_failure_cannot_publish_partial_counts_or_an_unknown_row() {
        let constraints = [Digest32::new([1; 32]), Digest32::new([2; 32])];
        let mut scope = ScopeCounters::with_admission(&constraints, Resident).unwrap();
        let mut emitted = false;
        let error = scope
            .evaluate(
                |constraint| {
                    if constraint == constraints[0] {
                        Ok(ConstraintDecision::Satisfied)
                    } else {
                        Err("original scan failed")
                    }
                },
                |_| {
                    emitted = true;
                    Ok(())
                },
            )
            .unwrap_err();
        assert!(matches!(
            error,
            ScopeError::Callback("original scan failed")
        ));
        assert!(!emitted);
        assert!(matches!(scope.counts(), Err(ScopeError::Aborted)));
    }

    #[test]
    fn satisfied_candidates_need_no_temporary_decision_capacity() {
        #[derive(Debug)]
        struct Capacity {
            live: usize,
            limit: usize,
        }
        impl Admission for Capacity {
            fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
                if bytes > self.limit {
                    return Err(StorageError::AdmissionFailed);
                }
                self.live = bytes;
                Ok(())
            }
        }
        let mut scope = ScopeCounters::with_admission(
            &[Digest32::new([1; 32])],
            Capacity {
                live: 0,
                limit: usize::MAX,
            },
        )
        .unwrap();
        scope.admission.limit = scope.live;
        scope
            .evaluate(
                |_| Ok::<_, Infallible>(ConstraintDecision::Satisfied),
                |decisions| {
                    assert_eq!(decisions, []);
                    Ok::<_, Infallible>(())
                },
            )
            .unwrap();
        assert_eq!(scope.counts().unwrap()[0].satisfied, 1);
        assert_eq!(scope.admission.live, scope.live);
        assert!(matches!(
            scope.evaluate(
                |_| Ok::<_, Infallible>(ConstraintDecision::Rejected),
                |_| Ok::<_, Infallible>(())
            ),
            Err(ScopeError::Storage(StorageError::AdmissionFailed))
        ));
        assert!(matches!(scope.counts(), Err(ScopeError::Aborted)));
    }
}

/// Separate exact counts for one caller-declared constraint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConstraintCounts {
    /// Canonical identity of the original constraint description.
    pub constraint: Digest32,
    /// Candidates the data establishes as satisfying this constraint.
    pub satisfied: u64,
    /// Candidates the data establishes as failing this constraint.
    pub rejected: u64,
    /// Candidates this constraint could not decide; never counted as rejected.
    pub undecided: u64,
}

/// A declaration, original producer callback or physical failure.
#[derive(Debug)]
pub enum ScopeError<E> {
    /// Constraint identities are not sorted and unique.
    NonCanonicalConstraints,
    /// An earlier operation aborted this owner; partial counts cannot be read.
    Aborted,
    /// The original decision or emission callback failed operationally.
    Callback(E),
    /// Native storage/layout/count overflow or refusal.
    Storage(StorageError),
}

impl<E: core::fmt::Display> core::fmt::Display for ScopeError<E> {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NonCanonicalConstraints => {
                out.write_str("scope constraints must be sorted and unique")
            }
            Self::Aborted => {
                out.write_str("scoped candidate generation was aborted by its original failure")
            }
            Self::Callback(error) => error.fmt(out),
            Self::Storage(error) => error.fmt(out),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for ScopeError<E> {}

/// Original admission owner for a stratum's full constraint accounting.
///
/// Producers invoke this during candidate generation, before their own depth or
/// any outer LIMIT. Every constraint is decided even if another already rejected
/// the candidate, so missing-time counts are independent of place/entity results.
/// A failed callback aborts the operation rather than manufacturing undecided
/// data. Counts are destroyed before their original grant owner.
#[derive(Debug)]
pub struct ScopeCounters<S: Admission> {
    counts: Vec<ConstraintCounts>,
    live: usize,
    aborted: bool,
    admission: S,
}

impl<S: Admission> ScopeCounters<S> {
    /// Admit the original counters before any candidate is generated.
    ///
    /// # Errors
    /// Refuses noncanonical constraint identities or native storage failure.
    pub fn with_admission(
        constraints: &[Digest32],
        mut admission: S,
    ) -> Result<Self, ScopeError<core::convert::Infallible>> {
        if constraints.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(ScopeError::NonCanonicalConstraints);
        }
        let (counts, live) = {
            let mut memory = Memory::new(&mut admission);
            let counts = memory
                .scope(|memory| {
                    memory.collect(constraints.iter().map(|&constraint| ConstraintCounts {
                        constraint,
                        satisfied: 0,
                        rejected: 0,
                        undecided: 0,
                    }))
                })
                .map_err(ScopeError::Storage)?;
            (counts, memory.admitted_bytes())
        };
        Ok(Self {
            counts,
            live,
            aborted: false,
            admission,
        })
    }

    /// Read complete accounting; an aborted prefix is never a valid summary.
    ///
    /// # Errors
    /// Refuses an owner aborted by its original callback or physical failure.
    pub fn counts(&self) -> Result<&[ConstraintCounts], ScopeError<core::convert::Infallible>> {
        if self.aborted {
            Err(ScopeError::Aborted)
        } else {
            Ok(&self.counts)
        }
    }

    /// Decide a candidate, then lend its non-satisfied decisions for publication.
    ///
    /// An empty slice means every declared constraint was satisfied. Sparse
    /// entries are `(constraint ordinal, decision)` in canonical constraint order;
    /// undecided reasons remain distinct from actual rejections. The emitter
    /// creates retained evidence under its original native result owner; it
    /// cannot detach this temporary decision array from the counters' admission.
    /// Successful all-satisfied candidates allocate no decision array.
    ///
    /// # Errors
    /// Returns the original callback failure, physical refusal or aborted owner.
    pub fn evaluate<E, R>(
        &mut self,
        mut decide: impl FnMut(Digest32) -> Result<ConstraintDecision, E>,
        emit: impl FnOnce(&[(usize, ConstraintDecision)]) -> Result<R, E>,
    ) -> Result<R, ScopeError<E>> {
        if self.aborted {
            return Err(ScopeError::Aborted);
        }
        let mut memory = Memory::resume(&mut self.admission, self.live);
        let result = memory.try_scope(|memory| {
            let mut decisions = Vec::new();
            for (ordinal, counts) in self.counts.iter_mut().enumerate() {
                let decision = decide(counts.constraint).map_err(ScopeError::Callback)?;
                let count = match decision {
                    ConstraintDecision::Satisfied => &mut counts.satisfied,
                    ConstraintDecision::Rejected => &mut counts.rejected,
                    ConstraintDecision::Undecided(_) => &mut counts.undecided,
                };
                *count = count
                    .checked_add(1)
                    .ok_or(ScopeError::Storage(StorageError::SizeOverflow))?;
                if decision != ConstraintDecision::Satisfied {
                    memory
                        .push(&mut decisions, (ordinal, decision))
                        .map_err(ScopeError::Storage)?;
                }
            }
            let emitted = emit(&decisions).map_err(ScopeError::Callback);
            let refunded = memory.release_vec(decisions);
            match emitted {
                Ok(value) => {
                    refunded.map_err(ScopeError::Storage)?;
                    Ok(value)
                }
                Err(original) => Err(original),
            }
        });
        self.live = memory.admitted_bytes();
        self.aborted = result.is_err();
        result
    }
}
