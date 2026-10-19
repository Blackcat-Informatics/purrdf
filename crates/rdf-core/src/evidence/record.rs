// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Canonical immutable row evidence, independent of query-local dictionary ids.

use purrdf_hash::{Domain, blake3::RecordHasher, frame::frame_le_into, hex::Digest32};
use purrdf_lex::allocation::{Admission, Memory, StorageError};

use super::{
    AggregateBound, EvidenceCombination, Frame, Information, InformationMap, ReasonId,
    SearchCompleteness, StandpointError, StandpointKey, StandpointOrder,
};

const RECORD_DOMAIN: Domain = Domain::new(b"purrdf-core/evidence-record/v1");

/// Invalid aggregate evidence or refusal of its original native construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceRecordError {
    /// A scoped witness is outside the caller's declared standpoint order.
    Standpoint(StandpointError<Digest32>),
    /// One output column was assigned incompatible bounds in one declaration.
    ConflictingAggregateBound(Digest32),
    /// Physical layout, admission or allocation refusal.
    Storage(StorageError),
}

impl purrdf_hash::error::StructuredError for EvidenceRecordError {
    purrdf_hash::error_visit! {
        Standpoint(cause) => "evidence-record-standpoint" { "cause" => Cause(cause) },
        ConflictingAggregateBound(column) => "evidence-record-conflicting-bound" { "column" => Record(column) },
        Storage(cause) => "evidence-record-storage" { "cause" => Cause(cause) },
    }
}

purrdf_lex::variant_from!(EvidenceRecordError { Storage(StorageError) });

impl core::fmt::Display for EvidenceRecordError {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Standpoint(error) => error.fmt(out),
            Self::ConflictingAggregateBound(column) => write!(
                out,
                "conflicting evidence bounds for aggregate column {column}"
            ),
            Self::Storage(error) => error.fmt(out),
        }
    }
}

impl std::error::Error for EvidenceRecordError {}

/// Immutable canonical evidence belonging to its original query admission.
///
/// Reason ids identify descriptions rather than rows. Aggregate bounds are
/// keyed by output-column content identity: COUNT and MIN in the same result
/// may carry different bounds. Query-local row/dictionary numbers, insertion
/// order, rank and score never enter this record's published identity.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct EvidenceRecord {
    information: InformationMap,
    completeness: SearchCompleteness,
    frame: Frame,
    reasons: Vec<ReasonId>,
    aggregate_bounds: Vec<(Digest32, AggregateBound)>,
}

impl EvidenceRecord {
    /// Retain every independent facet while inheriting declared scoped claims.
    ///
    /// # Errors
    /// Returns undeclared standpoint, native admission or record layout errors.
    pub fn inherited<S: Admission + ?Sized>(
        &self,
        order: &StandpointOrder<Digest32>,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, EvidenceRecordError> {
        memory.try_scope(|memory| {
            let information =
                self.information
                    .inherited(order, memory)
                    .map_err(|error| match error {
                        StandpointError::Storage(error) => EvidenceRecordError::Storage(error),
                        error => EvidenceRecordError::Standpoint(error),
                    })?;
            let result = Self::construct(
                &information,
                false,
                self.completeness,
                &self.frame,
                self.reasons.iter().copied(),
                &self.aggregate_bounds,
                memory,
            )?;
            information.release(memory)?;
            Ok(result)
        })
    }

    /// Borrow the pointwise polarity witnesses.
    #[must_use]
    pub const fn information(&self) -> &InformationMap {
        &self.information
    }

    /// Search completeness, independent of polarity and frame.
    #[must_use]
    pub const fn completeness(&self) -> SearchCompleteness {
        self.completeness
    }

    /// Borrow canonical scenario and standpoint restrictions.
    #[must_use]
    pub const fn frame(&self) -> &Frame {
        &self.frame
    }

    /// Borrow sorted unique content identities of the original reasons.
    #[must_use]
    pub fn reasons(&self) -> &[ReasonId] {
        &self.reasons
    }

    /// Borrow sorted unique output-column bounds.
    #[must_use]
    pub fn aggregate_bounds(&self) -> &[(Digest32, AggregateBound)] {
        &self.aggregate_bounds
    }

    /// Whether this is the allocation-free common supported/complete/actual row.
    #[must_use]
    pub fn is_default(&self) -> bool {
        self.information.fallback() == Information::Supported
            && self.information.entries().is_empty()
            && self.completeness == SearchCompleteness::Complete
            && self.frame.is_unrestricted_actual()
            && self.reasons.is_empty()
            && self.aggregate_bounds.is_empty()
    }

    /// Combine immutable witnesses using the actual operator's evidence law.
    ///
    /// Reasons remain a canonical union. Bounds of different output columns
    /// remain independent. For conjunctive dependencies an inexact bound cannot
    /// be erased by an exact premise; an exact alternative establishes the same
    /// coalesced binding exactly. Opposite inexact directions establish no single
    /// direction for the combined dependency and are retained as undetermined.
    ///
    /// # Errors
    /// Returns native storage refusal or physical cardinality overflow.
    pub fn combine<S: Admission + ?Sized>(
        &self,
        other: &Self,
        law: EvidenceCombination,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, StorageError> {
        memory.scope(|memory| {
            let (information, completeness, frame) = match law {
                EvidenceCombination::Conjunction => (
                    self.information.truth_meet(&other.information, memory)?,
                    self.completeness.worse(other.completeness),
                    self.frame.meet(&other.frame, memory)?,
                ),
                EvidenceCombination::Alternatives => (
                    self.information
                        .information_join(&other.information, memory)?,
                    self.completeness.better(other.completeness),
                    self.frame.union(&other.frame, memory)?,
                ),
            };
            let mut reasons = memory.collect(self.reasons.iter().chain(&other.reasons).copied())?;
            reasons.sort_unstable();
            reasons.dedup();
            let mut aggregate_bounds = memory.collect(
                self.aggregate_bounds
                    .iter()
                    .chain(&other.aggregate_bounds)
                    .copied(),
            )?;
            aggregate_bounds.sort_unstable_by_key(|entry| entry.0);
            let mut written = 0;
            for read in 0..aggregate_bounds.len() {
                let entry = aggregate_bounds[read];
                if written != 0 && aggregate_bounds[written - 1].0 == entry.0 {
                    let previous = &mut aggregate_bounds[written - 1].1;
                    *previous = combine_bound(*previous, entry.1, law);
                } else {
                    aggregate_bounds[written] = entry;
                    written += 1;
                }
            }
            aggregate_bounds.truncate(written);
            Ok(Self {
                information,
                completeness,
                frame,
                reasons,
                aggregate_bounds,
            })
        })
    }

    /// Combine every independent witness in a factorized derivation.
    ///
    /// Each factor contains alternatives; the factors themselves are conjunctive.
    /// Aggregate bounds do not distribute over alternatives, so each column
    /// carries its finite set of reachable states until the final alternative
    /// fold. An absent column is a separate identity state. This computes the
    /// expanded proof product without enumerating that product.
    ///
    /// # Errors
    /// Returns original native allocation or admission refusal.
    pub fn factorized<S: Admission + ?Sized>(
        factors: &[&[&Self]],
        memory: &mut Memory<'_, S>,
    ) -> Result<Option<Self>, StorageError> {
        if factors.iter().any(|factor| factor.is_empty()) {
            return Ok(None);
        }
        memory.scope(|memory| {
            let mut result = Self::default();
            for factor in factors {
                let mut alternatives = factor[0].combine(
                    &Self::default(),
                    EvidenceCombination::Conjunction,
                    memory,
                )?;
                for witness in &factor[1..] {
                    let next =
                        alternatives.combine(witness, EvidenceCombination::Alternatives, memory)?;
                    alternatives.release(memory)?;
                    alternatives = next;
                }
                let next =
                    result.combine(&alternatives, EvidenceCombination::Conjunction, memory)?;
                result.release(memory)?;
                alternatives.release(memory)?;
                result = next;
            }
            let mut columns = memory.collect(factors.iter().flat_map(|factor| {
                factor
                    .iter()
                    .flat_map(|record| record.aggregate_bounds.iter().map(|entry| entry.0))
            }))?;
            columns.sort_unstable();
            columns.dedup();
            let mut bounds = Vec::new();
            for column in &columns {
                let mut possible = 1;
                for factor in factors {
                    let mut alternatives = 0;
                    for witness in *factor {
                        let state = witness
                            .aggregate_bounds
                            .binary_search_by_key(column, |entry| entry.0)
                            .ok()
                            .map(|index| witness.aggregate_bounds[index].1);
                        alternatives |= bound_state_bit(state);
                    }
                    possible = conjoin_bound_states(possible, alternatives);
                }
                let mut folded = None;
                for (index, state) in BOUND_STATES.iter().enumerate().skip(1) {
                    if possible & (1 << index) != 0 {
                        let bound = state.expect("present bound state");
                        folded = Some(folded.map_or(bound, |previous| {
                            combine_bound(previous, bound, EvidenceCombination::Alternatives)
                        }));
                    }
                }
                if let Some(bound) = folded {
                    memory.reserve_for_push(&mut bounds)?;
                    bounds.push((*column, bound));
                }
            }
            memory.release_vec(columns)?;
            memory.release_vec(core::mem::replace(&mut result.aggregate_bounds, bounds))?;
            Ok(Some(result))
        })
    }

    /// Copy canonical components before publication under their native owner.
    ///
    /// Duplicate reason descriptions collapse. Duplicate column bounds must
    /// agree; contradictory declarations are hard errors, never an implicit
    /// replacement of one output column's evidence by another's.
    ///
    /// # Errors
    /// Refuses inconsistent column bounds or physical storage failure.
    pub fn from_parts<S: Admission + ?Sized>(
        information: &InformationMap,
        completeness: SearchCompleteness,
        frame: &Frame,
        reasons: &[ReasonId],
        aggregate_bounds: &[(Digest32, AggregateBound)],
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, EvidenceRecordError> {
        Self::construct(
            information,
            false,
            completeness,
            frame,
            reasons.iter().copied(),
            aggregate_bounds,
            memory,
        )
    }

    /// Relabel only search completeness and append an original semantic reason.
    ///
    /// Polarity, frame and individual aggregate columns remain independent. This
    /// is the native operation used for undecided FILTER/OPTIONAL rows; an
    /// operational failure is never supplied here as a semantic reason.
    ///
    /// # Errors
    /// Returns original native storage or malformed aggregate declarations.
    pub fn with_completeness<S: Admission + ?Sized>(
        &self,
        completeness: SearchCompleteness,
        reason: Option<ReasonId>,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, EvidenceRecordError> {
        Self::construct(
            &self.information,
            false,
            completeness,
            &self.frame,
            self.reasons.iter().copied().chain(reason),
            &self.aggregate_bounds,
            memory,
        )
    }

    /// Negate polarity after the caller established complete declared closure.
    ///
    /// Completeness, original reasons and frame remain intact. This method cannot
    /// grant closure itself: the operator must check its actual scope first.
    ///
    /// # Errors
    /// Returns original native storage or malformed aggregate declarations.
    pub fn negated<S: Admission + ?Sized>(
        &self,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, EvidenceRecordError> {
        Self::construct(
            &self.information,
            true,
            self.completeness,
            &self.frame,
            self.reasons.iter().copied(),
            &self.aggregate_bounds,
            memory,
        )
    }

    /// Attach the new aggregate output's separate per-column certificates.
    ///
    /// This output retains its input information, completeness, reasons and
    /// frame. Bounds belong to the new aggregate columns, rather than being a
    /// single answer-wide flag or copied certificates of old input columns.
    ///
    /// # Errors
    /// Refuses contradictory column declarations or original native refusal.
    pub fn with_aggregate_bounds<S: Admission + ?Sized>(
        &self,
        bounds: &[(Digest32, AggregateBound)],
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, EvidenceRecordError> {
        Self::construct(
            &self.information,
            false,
            self.completeness,
            &self.frame,
            self.reasons.iter().copied(),
            bounds,
            memory,
        )
    }

    /// Add a visible alias for an existing aggregate certificate, retaining the
    /// original column until a projection decides which columns survive.
    ///
    /// # Errors
    /// Returns inconsistent declarations or original native storage refusal.
    pub fn with_column_alias<S: Admission + ?Sized>(
        &self,
        source: Digest32,
        target: Digest32,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, EvidenceRecordError> {
        let bound = self
            .aggregate_bounds
            .iter()
            .find(|(column, _)| *column == source)
            .map(|(_, bound)| (target, *bound));
        self.with_bound_values(self.aggregate_bounds.iter().copied().chain(bound), memory)
    }

    /// Keep certificates only for columns selected by the original projection.
    /// Information, completeness, reasons and frame are independent of columns.
    ///
    /// # Errors
    /// Returns original native storage refusal.
    pub fn with_selected_bounds<S: Admission + ?Sized>(
        &self,
        mut keep: impl FnMut(Digest32) -> bool,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, EvidenceRecordError> {
        self.with_bound_values(
            self.aggregate_bounds
                .iter()
                .copied()
                .filter(|(column, _)| keep(*column)),
            memory,
        )
    }

    fn with_bound_values<S: Admission + ?Sized>(
        &self,
        values: impl Iterator<Item = (Digest32, AggregateBound)>,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, EvidenceRecordError> {
        memory.try_scope(|memory| {
            let bounds = memory.collect(values)?;
            let record = self.with_aggregate_bounds(&bounds, memory)?;
            memory.release_vec(bounds)?;
            Ok(record)
        })
    }

    fn construct<S: Admission + ?Sized>(
        information: &InformationMap,
        negate: bool,
        completeness: SearchCompleteness,
        frame: &Frame,
        reasons: impl Iterator<Item = ReasonId>,
        aggregate_bounds: &[(Digest32, AggregateBound)],
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, EvidenceRecordError> {
        memory.try_scope(|memory| {
            let information = if negate {
                information.negate(memory)?
            } else {
                information.truth_meet(&InformationMap::default(), memory)?
            };
            let frame = Frame::actual().meet(frame, memory)?;
            let mut reasons = memory.collect(reasons)?;
            reasons.sort_unstable();
            reasons.dedup();
            let mut aggregate_bounds = memory.collect(aggregate_bounds.iter().copied())?;
            aggregate_bounds.sort_unstable_by_key(|entry| entry.0);
            for pair in aggregate_bounds.windows(2) {
                if pair[0].0 == pair[1].0 && pair[0].1 != pair[1].1 {
                    return Err(EvidenceRecordError::ConflictingAggregateBound(pair[0].0));
                }
            }
            aggregate_bounds.dedup();
            Ok(Self {
                information,
                completeness,
                frame,
                reasons,
                aggregate_bounds,
            })
        })
    }

    /// Canonical framed identity of content, including every independent axis.
    #[must_use]
    pub fn identity(&self) -> Digest32 {
        let mut hash = RecordHasher::new();
        frame_le_into(&mut hash, RECORD_DOMAIN.as_bytes());
        frame_le_into(&mut hash, &[self.information.fallback() as u8]);
        frame_le_into(
            &mut hash,
            &(self.information.entries().len() as u64).to_le_bytes(),
        );
        for &(key, information) in self.information.entries() {
            match key {
                StandpointKey::Unscoped => frame_le_into(&mut hash, &[0]),
                StandpointKey::Declared(id) => {
                    frame_le_into(&mut hash, &[1]);
                    frame_le_into(&mut hash, id.as_bytes());
                }
            }
            frame_le_into(&mut hash, &[information as u8]);
        }
        frame_le_into(&mut hash, &[self.completeness as u8]);
        frame_le_into(&mut hash, self.frame.identity().as_bytes());
        frame_le_into(&mut hash, &(self.reasons.len() as u64).to_le_bytes());
        for reason in &self.reasons {
            frame_le_into(&mut hash, reason.digest().as_bytes());
        }
        frame_le_into(
            &mut hash,
            &(self.aggregate_bounds.len() as u64).to_le_bytes(),
        );
        for &(column, bound) in &self.aggregate_bounds {
            frame_le_into(&mut hash, column.as_bytes());
            frame_le_into(
                &mut hash,
                &[match bound {
                    AggregateBound::Exact => 0,
                    AggregateBound::Lower => 1,
                    AggregateBound::Upper => 2,
                    AggregateBound::Undetermined => 3,
                }],
            );
        }
        Digest32::new(*hash.finalize().as_bytes())
    }

    /// Destroy original component payloads before refunding each native layout.
    ///
    /// # Errors
    /// Returns the caller's typed refund refusal marker.
    pub fn release<S: Admission + ?Sized>(
        self,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        self.information.release(memory)?;
        self.frame.release(memory)?;
        memory.release_vec(self.reasons)?;
        memory.release_vec(self.aggregate_bounds)
    }
}

const BOUND_STATES: [Option<AggregateBound>; 5] = [
    None,
    Some(AggregateBound::Exact),
    Some(AggregateBound::Lower),
    Some(AggregateBound::Upper),
    Some(AggregateBound::Undetermined),
];

fn bound_state_bit(state: Option<AggregateBound>) -> u8 {
    1 << BOUND_STATES
        .iter()
        .position(|candidate| *candidate == state)
        .expect("closed aggregate bound state")
}

fn conjoin_bound_states(left: u8, right: u8) -> u8 {
    let mut result = 0;
    for (left_index, left_state) in BOUND_STATES.iter().enumerate() {
        if left & (1 << left_index) == 0 {
            continue;
        }
        for (right_index, right_state) in BOUND_STATES.iter().enumerate() {
            if right & (1 << right_index) == 0 {
                continue;
            }
            let state = match (*left_state, *right_state) {
                (Some(left), Some(right)) => {
                    Some(combine_bound(left, right, EvidenceCombination::Conjunction))
                }
                (left, right) => left.or(right),
            };
            result |= bound_state_bit(state);
        }
    }
    result
}

fn combine_bound(
    left: AggregateBound,
    right: AggregateBound,
    law: EvidenceCombination,
) -> AggregateBound {
    if left == right {
        return left;
    }
    match law {
        EvidenceCombination::Conjunction => match (left, right) {
            (AggregateBound::Exact, bound) | (bound, AggregateBound::Exact) => bound,
            _ => AggregateBound::Undetermined,
        },
        EvidenceCombination::Alternatives => match (left, right) {
            (AggregateBound::Exact, _) | (_, AggregateBound::Exact) => AggregateBound::Exact,
            _ => AggregateBound::Undetermined,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_lex::allocation::Resident;

    #[test]
    fn factorized_bounds_match_expanded_proofs_including_absent_columns() {
        let mut admission = Resident;
        let mut memory = Memory::new(&mut admission);
        let column = Digest32::new([7; 32]);
        for left in BOUND_STATES {
            for first in BOUND_STATES {
                for second in BOUND_STATES {
                    let mut records = Vec::new();
                    for state in [left, first, second] {
                        let bounds = state.map(|bound| (column, bound));
                        records.push(
                            EvidenceRecord::from_parts(
                                &InformationMap::default(),
                                SearchCompleteness::Complete,
                                &Frame::actual(),
                                &[],
                                bounds.as_slice(),
                                &mut memory,
                            )
                            .unwrap(),
                        );
                    }
                    let first_proof = records[0]
                        .combine(&records[1], EvidenceCombination::Conjunction, &mut memory)
                        .unwrap();
                    let second_proof = records[0]
                        .combine(&records[2], EvidenceCombination::Conjunction, &mut memory)
                        .unwrap();
                    let expanded = first_proof
                        .combine(
                            &second_proof,
                            EvidenceCombination::Alternatives,
                            &mut memory,
                        )
                        .unwrap();
                    let product = EvidenceRecord::factorized(
                        &[&[&records[0]], &[&records[1], &records[2]]],
                        &mut memory,
                    )
                    .unwrap()
                    .unwrap();
                    assert_eq!(product, expanded, "{left:?} × ({first:?} + {second:?})");
                    if left == Some(AggregateBound::Lower)
                        && first == Some(AggregateBound::Exact)
                        && second == Some(AggregateBound::Undetermined)
                    {
                        assert_eq!(
                            product.aggregate_bounds(),
                            &[(column, AggregateBound::Undetermined)]
                        );
                    }
                    product.release(&mut memory).unwrap();
                    expanded.release(&mut memory).unwrap();
                    first_proof.release(&mut memory).unwrap();
                    second_proof.release(&mut memory).unwrap();
                    for record in records {
                        record.release(&mut memory).unwrap();
                    }
                    assert_eq!(memory.admitted_bytes(), 0);
                }
            }
        }
        assert_eq!(
            EvidenceRecord::factorized(&[&[]], &mut memory).unwrap(),
            None
        );
        assert_eq!(
            EvidenceRecord::factorized(&[], &mut memory).unwrap(),
            Some(EvidenceRecord::default())
        );
        assert_eq!(memory.admitted_bytes(), 0);
    }

    #[test]
    fn per_column_bounds_are_canonical_and_do_not_collapse() {
        let mut admission = Resident;
        let mut memory = Memory::new(&mut admission);
        let count = Digest32::new([1; 32]);
        let min = Digest32::new([2; 32]);
        let first = EvidenceRecord::from_parts(
            &InformationMap::default(),
            SearchCompleteness::Undetermined,
            &Frame::actual(),
            &[],
            &[(count, AggregateBound::Lower), (min, AggregateBound::Upper)],
            &mut memory,
        )
        .unwrap();
        let second = EvidenceRecord::from_parts(
            &InformationMap::default(),
            SearchCompleteness::Undetermined,
            &Frame::actual(),
            &[],
            &[
                (min, AggregateBound::Upper),
                (count, AggregateBound::Lower),
                (count, AggregateBound::Lower),
            ],
            &mut memory,
        )
        .unwrap();
        assert_eq!(first, second);
        assert_eq!(first.identity(), second.identity());
        assert_eq!(first.aggregate_bounds().len(), 2);
        assert_ne!(first.identity(), EvidenceRecord::default().identity());
        let baseline = memory.admitted_bytes();
        assert_eq!(
            EvidenceRecord::from_parts(
                &InformationMap::default(),
                SearchCompleteness::Complete,
                &Frame::actual(),
                &[],
                &[
                    (count, AggregateBound::Lower),
                    (count, AggregateBound::Upper)
                ],
                &mut memory
            ),
            Err(EvidenceRecordError::ConflictingAggregateBound(count))
        );
        assert_eq!(memory.admitted_bytes(), baseline);
        first.release(&mut memory).unwrap();
        second.release(&mut memory).unwrap();
        assert_eq!(memory.admitted_bytes(), 0);
    }
}
