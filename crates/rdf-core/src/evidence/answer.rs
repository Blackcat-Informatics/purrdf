// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Answer-tier accumulation over the original admitted evidence records.

use purrdf_lex::allocation::{Admission, Memory, StorageError};

use super::{
    AnswerTier, EvidenceRecord, Information, ReasonId, SearchCompleteness, StandpointKey,
    WitnessQuestion, WitnessVerdict,
};

/// The original evaluator partition, independent of a row's FOUR value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidencePartition {
    /// The actual query's membership constraints were established.
    Established,
    /// At least one genuine semantic constraint remains undecidable.
    Undecided,
}

/// Query specification for accumulation, without a vocabulary default.
#[derive(Clone, Copy, Debug)]
pub struct AnswerRequest {
    /// Requested rendering tier; lower tiers never construct reason counts.
    pub tier: AnswerTier,
    /// The exact witness question; ordinary row queries supply no question.
    pub question: Option<WitnessQuestion>,
    /// Selected context, including the explicit unscoped context.
    pub standpoint: StandpointKey,
    /// Actual source completeness, distinct from output enumeration progress.
    pub input_completeness: SearchCompleteness,
    /// Caller-declared closure, never inferred from an empty result.
    pub closure_declared: bool,
}

/// Counts of original rows carrying one content-identified reason.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReasonCount {
    /// Original canonical cause/concern description identity.
    pub reason: ReasonId,
    /// Established rows, including explicit accepted overrides if supplied.
    pub established: u64,
    /// Original undecided rows; counts do not turn them into false membership.
    pub undecided: u64,
}

/// A hard physical refusal or broken answer-publication invariant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnswerError {
    /// A previous mutation failed; no partial summary can be published.
    Aborted,
    /// Enumeration stopped before a tier requiring the complete partitions.
    EnumerationRequired,
    /// The answer has not reached an explicit finish boundary.
    NotFinished,
    /// Observation was attempted after the finish boundary.
    AlreadyFinished,
    /// An undecided partition row falsely claimed complete semantic evaluation.
    InvalidUndecided,
    /// Original native admission, allocator or physical count overflow.
    Storage(StorageError),
}

impl core::fmt::Display for AnswerError {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Aborted => out.write_str("answer evidence accumulation was aborted"),
            Self::EnumerationRequired => {
                out.write_str("this answer tier requires complete row enumeration")
            }
            Self::NotFinished => out.write_str("answer evidence has not finished"),
            Self::AlreadyFinished => out.write_str("answer evidence already finished"),
            Self::InvalidUndecided => out.write_str("undecided row lacks semantic incompleteness"),
            Self::Storage(error) => error.fmt(out),
        }
    }
}
impl std::error::Error for AnswerError {}

/// A borrowed final status; no rendered ledger or copied reason description.
#[derive(Clone, Copy, Debug)]
pub struct AnswerStatus<'a> {
    /// Independent polarities actually witnessed in the selected context.
    pub information: Information,
    /// Full input completeness, also accounting for an unenumerated suffix.
    pub completeness: SearchCompleteness,
    /// Original established rows seen by this query.
    pub established: u64,
    /// Original undecided rows seen by this query.
    pub undecided: u64,
    /// Canonical reason counts, empty below the summary tier.
    pub reasons: &'a [ReasonCount],
    /// Answer to the explicitly requested question, never a guessed full FOUR.
    pub verdict: Option<WitnessVerdict>,
}

/// One original native count-array owner, with an allocation-free verdict path.
///
/// The array is destroyed before the final admission owner. Actual row ids and
/// their dictionaries remain in the query's original owner at every tier.
#[derive(Debug)]
pub struct AnswerAccumulator<S> {
    counts: Vec<ReasonCount>,
    request: AnswerRequest,
    information: Information,
    completeness: SearchCompleteness,
    established: u64,
    undecided: u64,
    finished: Option<bool>,
    live: usize,
    aborted: bool,
    admission: S,
}

impl<S: Admission> AnswerAccumulator<S> {
    /// Start without publishing or allocating any metadata array.
    #[must_use]
    pub const fn with_admission(request: AnswerRequest, admission: S) -> Self {
        Self {
            counts: Vec::new(),
            information: Information::Neither,
            completeness: request.input_completeness,
            request,
            established: 0,
            undecided: 0,
            finished: None,
            live: 0,
            aborted: false,
            admission,
        }
    }

    /// Observe one original row; operational failures belong to the caller's Err.
    ///
    /// # Errors
    /// Native refusal aborts the whole accumulator; no partial summary is valid.
    pub fn observe(
        &mut self,
        record: &EvidenceRecord,
        partition: EvidencePartition,
    ) -> Result<(), AnswerError> {
        if self.aborted {
            return Err(AnswerError::Aborted);
        }
        if self.finished.is_some() {
            return Err(AnswerError::AlreadyFinished);
        }
        if partition == EvidencePartition::Undecided
            && record.completeness() == SearchCompleteness::Complete
        {
            self.aborted = true;
            return Err(AnswerError::InvalidUndecided);
        }
        let result = self.observe_row(record, partition);
        if result.is_err() {
            self.aborted = true;
        }
        result.map_err(AnswerError::Storage)
    }

    fn observe_row(
        &mut self,
        record: &EvidenceRecord,
        partition: EvidencePartition,
    ) -> Result<(), StorageError> {
        let count = match partition {
            EvidencePartition::Established => &mut self.established,
            EvidencePartition::Undecided => &mut self.undecided,
        };
        *count = count.checked_add(1).ok_or(StorageError::SizeOverflow)?;
        if partition == EvidencePartition::Established {
            self.information = self
                .information
                .information_join(record.information().get(self.request.standpoint));
        }
        self.completeness = self.completeness.worse(record.completeness());
        self.observe_reasons(record, Some(partition))
    }

    /// Retain an original search attestation independently of row membership.
    /// Its reasons remain visible at summary tier even when no row was emitted;
    /// neither polarity nor either row count is invented for the search itself.
    /// Closure is declared separately: a completely searched open-world scope
    /// remains complete but cannot establish a closed negative conclusion.
    ///
    /// # Errors
    /// Native refusal aborts publication exactly as for a row observation.
    pub fn observe_search(
        &mut self,
        record: &EvidenceRecord,
        closure_declared: bool,
    ) -> Result<(), AnswerError> {
        if self.aborted {
            return Err(AnswerError::Aborted);
        }
        if self.finished.is_some() {
            return Err(AnswerError::AlreadyFinished);
        }
        self.completeness = self.completeness.worse(record.completeness());
        self.request.closure_declared &= closure_declared;
        let result = self.observe_reasons(record, None);
        if result.is_err() {
            self.aborted = true;
        }
        result.map_err(AnswerError::Storage)
    }

    fn observe_reasons(
        &mut self,
        record: &EvidenceRecord,
        partition: Option<EvidencePartition>,
    ) -> Result<(), StorageError> {
        if self.request.tier < AnswerTier::Summary {
            return Ok(());
        }
        let mut memory = Memory::resume(&mut self.admission, self.live);
        let result = (|| {
            for &reason in record.reasons() {
                let index = match self
                    .counts
                    .binary_search_by_key(&reason, |count| count.reason)
                {
                    Ok(index) => index,
                    Err(index) => {
                        let required = self
                            .counts
                            .len()
                            .checked_add(1)
                            .ok_or(StorageError::SizeOverflow)?;
                        memory.reserve(&mut self.counts, required)?;
                        self.counts.insert(
                            index,
                            ReasonCount {
                                reason,
                                established: 0,
                                undecided: 0,
                            },
                        );
                        index
                    }
                };
                let count = match partition {
                    Some(EvidencePartition::Established) => &mut self.counts[index].established,
                    Some(EvidencePartition::Undecided) => &mut self.counts[index].undecided,
                    None => continue,
                };
                *count = count.checked_add(1).ok_or(StorageError::SizeOverflow)?;
            }
            Ok(())
        })();
        self.live = memory.admitted_bytes();
        result
    }

    /// Earliest sound stop, only at the verdict tier and for its exact question.
    ///
    /// An incomplete closed-absence question is unknown without consuming a
    /// suffix. Positive support alone never settles contradiction or full FOUR.
    /// Higher tiers continue to enumerate their required original partitions.
    #[must_use]
    pub fn witness_stop(&self) -> Option<WitnessVerdict> {
        if self.aborted || self.request.tier != AnswerTier::Verdict {
            return None;
        }
        let question = self.request.question?;
        if matches!(question, WitnessQuestion::ClosedAbsence)
            && !self
                .completeness
                .admits_absence(self.request.closure_declared)
            && !self.information.supports()
        {
            return Some(WitnessVerdict::Unknown);
        }
        question.settle(
            self.information,
            self.completeness,
            false,
            self.request.closure_declared,
        )
    }

    /// Mark the real enumeration boundary before publishing the borrowed status.
    ///
    /// # Errors
    /// Refuses unfinished higher-tier enumeration or an unsound verdict stop.
    pub fn finish(&mut self, exhausted: bool) -> Result<(), AnswerError> {
        if self.aborted {
            return Err(AnswerError::Aborted);
        }
        if self.finished.is_some() {
            return Err(AnswerError::AlreadyFinished);
        }
        if !exhausted && self.witness_stop().is_none() {
            return Err(AnswerError::EnumerationRequired);
        }
        self.finished = Some(exhausted);
        Ok(())
    }

    /// Borrow final status without changing either partition or its reasons.
    ///
    /// # Errors
    /// Refuses an aborted or unfinished answer.
    pub fn status(&self) -> Result<AnswerStatus<'_>, AnswerError> {
        if self.aborted {
            return Err(AnswerError::Aborted);
        }
        let exhausted = self.finished.ok_or(AnswerError::NotFinished)?;
        let verdict = if exhausted {
            self.request.question.and_then(|question| {
                question.settle(
                    self.information,
                    self.completeness,
                    true,
                    self.request.closure_declared,
                )
            })
        } else {
            self.witness_stop()
        };
        Ok(AnswerStatus {
            information: self.information,
            completeness: if exhausted {
                self.completeness
            } else {
                self.completeness.worse(SearchCompleteness::NotEvaluated)
            },
            established: self.established,
            undecided: self.undecided,
            reasons: &self.counts,
            verdict,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        TermBox, TermValue,
        evidence::{Frame, InformationMap, ReasonDescription},
    };
    use purrdf_lex::allocation::Resident;

    #[derive(Debug)]
    struct Refuse;
    impl Admission for Refuse {
        fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
            if bytes == 0 {
                Ok(())
            } else {
                Err(StorageError::AdmissionFailed)
            }
        }
    }

    fn request(tier: AnswerTier, question: WitnessQuestion) -> AnswerRequest {
        AnswerRequest {
            tier,
            question: Some(question),
            standpoint: StandpointKey::Unscoped,
            input_completeness: SearchCompleteness::Complete,
            closure_declared: true,
        }
    }

    fn record(
        information: Information,
        completeness: SearchCompleteness,
        reasons: &[ReasonId],
    ) -> EvidenceRecord {
        EvidenceRecord::from_parts(
            &InformationMap::uniform(information),
            completeness,
            &Frame::actual(),
            reasons,
            &[],
            &mut Memory::new(&mut Resident),
        )
        .unwrap()
    }

    fn reason() -> ReasonId {
        let cause = TermValue::iri("https://example.org/missing-time");
        let concern = TermValue::Triple {
            s: TermBox::new(TermValue::iri("https://example.org/record")),
            p: TermBox::new(TermValue::iri("https://example.org/time")),
            o: TermBox::new(TermValue::simple_literal("required")),
        };
        ReasonDescription {
            cause: &cause,
            concern: &concern,
            constraint: None,
            vault: None,
            watermark: None,
        }
        .identity(&mut Memory::new(&mut Resident))
        .unwrap()
    }

    #[test]
    fn exact_witness_questions_stop_without_any_metadata_capacity_or_false_classification() {
        let supported = record(Information::Supported, SearchCompleteness::Complete, &[]);
        let opposed = record(Information::Opposed, SearchCompleteness::Complete, &[]);
        for question in [
            WitnessQuestion::Support,
            WitnessQuestion::Contradiction,
            WitnessQuestion::Classify,
        ] {
            let mut answer =
                AnswerAccumulator::with_admission(request(AnswerTier::Verdict, question), Refuse);
            answer
                .observe(&supported, EvidencePartition::Established)
                .unwrap();
            assert_eq!(
                answer.witness_stop(),
                if question == WitnessQuestion::Support {
                    Some(WitnessVerdict::Yes)
                } else {
                    None
                }
            );
            if question != WitnessQuestion::Support {
                assert_eq!(answer.finish(false), Err(AnswerError::EnumerationRequired));
                answer
                    .observe(&opposed, EvidencePartition::Established)
                    .unwrap();
                assert_eq!(answer.witness_stop(), Some(WitnessVerdict::Yes));
            }
            answer.finish(false).unwrap();
            let status = answer.status().unwrap();
            assert_eq!(status.verdict, Some(WitnessVerdict::Yes));
            assert_eq!(
                status.information,
                if question == WitnessQuestion::Support {
                    Information::Supported
                } else {
                    Information::Both
                }
            );
            assert_eq!(status.completeness, SearchCompleteness::NotEvaluated);
            assert_eq!(status.reasons, []);
            assert_eq!(answer.live, 0);
        }
    }

    #[test]
    fn empty_search_retains_reason_without_fabricating_a_row_or_witness() {
        let reason = reason();
        let search = record(
            Information::Supported,
            SearchCompleteness::Undetermined,
            &[reason],
        );
        let mut answer = AnswerAccumulator::with_admission(
            request(AnswerTier::Summary, WitnessQuestion::Support),
            Resident,
        );
        answer.observe_search(&search, true).unwrap();
        answer.finish(true).unwrap();
        let status = answer.status().unwrap();
        assert_eq!(status.information, Information::Neither);
        assert_eq!(status.completeness, SearchCompleteness::Undetermined);
        assert_eq!((status.established, status.undecided), (0, 0));
        assert_eq!(status.verdict, Some(WitnessVerdict::Unknown));
        assert_eq!(
            status.reasons,
            &[ReasonCount {
                reason,
                established: 0,
                undecided: 0
            }]
        );
        let mut verdict = AnswerAccumulator::with_admission(
            request(AnswerTier::Verdict, WitnessQuestion::Support),
            Refuse,
        );
        verdict.observe_search(&search, true).unwrap();
        verdict.finish(true).unwrap();
        assert_eq!(verdict.status().unwrap().reasons, []);
    }

    #[test]
    fn undecided_filter_rows_never_settle_positive_support_and_keep_original_reasons() {
        let reason = reason();
        let undecided = record(
            Information::Supported,
            SearchCompleteness::Undetermined,
            &[reason],
        );
        let mut answer = AnswerAccumulator::with_admission(
            request(AnswerTier::Verdict, WitnessQuestion::Support),
            Refuse,
        );
        answer
            .observe(&undecided, EvidencePartition::Undecided)
            .unwrap();
        assert_eq!(answer.witness_stop(), None);
        answer.finish(true).unwrap();
        let status = answer.status().unwrap();
        assert_eq!(status.verdict, Some(WitnessVerdict::Unknown));
        assert_eq!(status.information, Information::Neither);
        assert_eq!((status.established, status.undecided), (0, 1));
        assert_eq!(undecided.reasons(), [reason]);
        let mut closed_no = request(AnswerTier::Verdict, WitnessQuestion::ClosedAbsence);
        closed_no.input_completeness = SearchCompleteness::Undetermined;
        let mut answer = AnswerAccumulator::with_admission(closed_no, Refuse);
        assert_eq!(answer.witness_stop(), Some(WitnessVerdict::Unknown));
        answer.finish(false).unwrap();
        assert_eq!(
            answer.status().unwrap().verdict,
            Some(WitnessVerdict::Unknown)
        );
    }

    #[test]
    fn higher_tiers_count_shared_reasons_and_require_complete_partition_enumeration() {
        let reason = reason();
        let undecided = record(
            Information::Supported,
            SearchCompleteness::Undetermined,
            &[reason],
        );
        let established = record(Information::Both, SearchCompleteness::Complete, &[]);
        for tier in [AnswerTier::Summary, AnswerTier::Rows, AnswerTier::Ledger] {
            let mut answer = AnswerAccumulator::with_admission(
                request(tier, WitnessQuestion::Contradiction),
                Resident,
            );
            answer
                .observe(&established, EvidencePartition::Established)
                .unwrap();
            assert_eq!(
                answer.witness_stop(),
                None,
                "higher tiers still need their actual rows"
            );
            assert_eq!(answer.finish(false), Err(AnswerError::EnumerationRequired));
            for _ in 0..2_279 {
                answer
                    .observe(&undecided, EvidencePartition::Undecided)
                    .unwrap();
            }
            answer.finish(true).unwrap();
            let status = answer.status().unwrap();
            assert_eq!((status.established, status.undecided), (1, 2_279));
            assert_eq!(status.information, Information::Both);
            assert_eq!(status.completeness, SearchCompleteness::Undetermined);
            assert_eq!(
                status.reasons,
                [ReasonCount {
                    reason,
                    established: 0,
                    undecided: 2_279
                }]
            );
            assert_eq!(status.verdict, Some(WitnessVerdict::Yes));
        }
    }

    #[test]
    fn reason_count_admission_failure_aborts_instead_of_publishing_an_unknown_summary() {
        let reason = reason();
        let undecided = record(
            Information::Supported,
            SearchCompleteness::Undetermined,
            &[reason],
        );
        let mut answer = AnswerAccumulator::with_admission(
            request(AnswerTier::Summary, WitnessQuestion::Support),
            Refuse,
        );
        assert_eq!(
            answer.observe(&undecided, EvidencePartition::Undecided),
            Err(AnswerError::Storage(StorageError::AdmissionFailed))
        );
        assert!(matches!(answer.status(), Err(AnswerError::Aborted)));
        assert_eq!(answer.finish(true), Err(AnswerError::Aborted));
        assert_eq!(answer.counts, []);
        assert_eq!(answer.live, 0);
        let mut inconsistent = AnswerAccumulator::with_admission(
            request(AnswerTier::Verdict, WitnessQuestion::Support),
            Refuse,
        );
        assert_eq!(
            inconsistent.observe(&EvidenceRecord::default(), EvidencePartition::Undecided),
            Err(AnswerError::InvalidUndecided)
        );
        assert!(matches!(inconsistent.status(), Err(AnswerError::Aborted)));
    }
}
