// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original per-row evidence ownership for rule derivation and coalescing.

use purrdf_core::evidence::{
    AggregateBound, EvidenceColumn, EvidenceColumnError, EvidenceCombination, EvidenceId,
    EvidenceRecord, EvidenceRecordError, EvidenceWorkspace, EvidenceWorkspaceError, ReasonId,
    SearchCompleteness,
};
use purrdf_hash::hex::Digest32;
use purrdf_lex::allocation::{Admission, Memory};

use crate::id::RowId;

/// The row column and table owned by one derivation computation.
///
/// Facts retain their dense original row identities. The column is destroyed
/// before the native workspace and its grant, including when publication fails.
/// Uniform default facts have no id array and no allocated evidence records.
/// Provenance winner selection cannot discard alternative evidence witnesses.
#[derive(Debug)]
pub struct DerivationEvidence<S: Admission> {
    rows: EvidenceColumn,
    workspace: EvidenceWorkspace<S>,
}

impl<S: Admission> DerivationEvidence<S> {
    /// Own the caller's original grant alongside an existing seed row count.
    #[must_use]
    pub fn with_admission(seed_rows: usize, admission: S) -> Self {
        Self {
            rows: EvidenceColumn::uniform_default(seed_rows),
            workspace: EvidenceWorkspace::with_admission(admission),
        }
    }

    /// Number of facts aligned with the original evidence column.
    #[must_use]
    pub const fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Borrow the row-id column for an immutable parallel round snapshot.
    #[must_use]
    pub const fn rows(&self) -> &EvidenceColumn {
        &self.rows
    }

    /// Borrow a retained row's full immutable evidence.
    ///
    /// # Errors
    /// Refuses invalid row ids, invalid evidence ids or an aborted original owner.
    pub fn record(&self, row: RowId) -> Result<&EvidenceRecord, EvidenceWorkspaceError> {
        let id = self
            .rows
            .get(row.address()?)
            .map_err(EvidenceWorkspaceError::Column)?;
        self.workspace.get(id)
    }

    /// Construct a seed row's evidence under its original native admission.
    ///
    /// # Errors
    /// Refuses invalid rows before construction, then original record/storage errors.
    pub fn seed(
        &mut self,
        row: RowId,
        factory: impl FnOnce(&mut Memory<'_, S>) -> Result<EvidenceRecord, EvidenceRecordError>,
    ) -> Result<EvidenceId, EvidenceWorkspaceError> {
        self.rows
            .get(row.address()?)
            .map_err(EvidenceWorkspaceError::Column)?;
        let id = self.workspace.intern(factory)?;
        self.workspace.set_row(&mut self.rows, row.address()?, id)?;
        Ok(id)
    }

    /// Conjoin every matched source through the one query-local pair memo.
    ///
    /// # Errors
    /// Returns invalid source rows or original combination/refusal errors.
    pub fn conjunction(
        &mut self,
        sources: impl IntoIterator<Item = RowId>,
    ) -> Result<EvidenceId, EvidenceWorkspaceError> {
        let mut combined = EvidenceId::DEFAULT;
        for source in sources {
            let id = self
                .rows
                .get(source.address()?)
                .map_err(EvidenceWorkspaceError::Column)?;
            combined = self
                .workspace
                .combine(combined, id, EvidenceCombination::Conjunction)?;
        }
        Ok(combined)
    }

    /// Publish a new fact beside its already identified joined evidence.
    ///
    /// # Errors
    /// Refuses out-of-order row publication, unknown ids or original storage failure.
    pub fn push_derived(
        &mut self,
        row: RowId,
        id: EvidenceId,
    ) -> Result<(), EvidenceWorkspaceError> {
        if row.address()? != self.rows.len() {
            return Err(EvidenceWorkspaceError::Column(
                EvidenceColumnError::RowOutOfBounds,
            ));
        }
        self.workspace.push_row(&mut self.rows, id)
    }

    /// Coalesce another derivation without replacing its independent witness.
    ///
    /// Returns whether the stored evidence changed. This is a fixpoint delta even
    /// when the fact already existed; callers must schedule dependent rules again.
    /// The preferred public provenance proof is a separate concern.
    ///
    /// # Errors
    /// Returns alignment/combination errors or original physical refusal.
    pub fn coalesce(
        &mut self,
        row: RowId,
        witness: EvidenceId,
    ) -> Result<bool, EvidenceWorkspaceError> {
        let previous = self
            .rows
            .get(row.address()?)
            .map_err(EvidenceWorkspaceError::Column)?;
        let combined =
            self.workspace
                .combine(previous, witness, EvidenceCombination::Alternatives)?;
        if previous == combined {
            return Ok(false);
        }
        self.workspace
            .set_row(&mut self.rows, row.address()?, combined)?;
        Ok(true)
    }

    /// Borrow the original table for lazy result annotations and canonical ids.
    #[must_use]
    pub const fn workspace(&self) -> &EvidenceWorkspace<S> {
        &self.workspace
    }

    pub(crate) fn workspace_mut(&mut self) -> &mut EvidenceWorkspace<S> {
        &mut self.workspace
    }

    /// Negate an existential result only in its complete, declared closed scope.
    ///
    /// Otherwise the original row remains undecided and carries the supplied
    /// canonical reason. Missing scope is semantic data here; actual read or
    /// construction failures return their typed operational error instead.
    ///
    /// # Errors
    /// Returns unknown ids or original record/table/admission failure.
    pub fn absence(
        &mut self,
        existential: EvidenceId,
        closure_declared: bool,
        missing_scope: ReasonId,
    ) -> Result<EvidenceId, EvidenceWorkspaceError> {
        self.workspace.transform(existential, |record, memory| {
            if record.completeness().admits_absence(closure_declared) {
                record.negated(memory)
            } else {
                record.with_completeness(
                    record
                        .completeness()
                        .worse(SearchCompleteness::Undetermined),
                    Some(missing_scope),
                    memory,
                )
            }
        })
    }

    /// A found path is a complete witness despite missing alternative data.
    ///
    /// # Errors
    /// Returns unknown ids or original native transformation failure.
    pub fn found_path(
        &mut self,
        witness: EvidenceId,
    ) -> Result<EvidenceId, EvidenceWorkspaceError> {
        self.workspace.transform(witness, |record, memory| {
            record.with_completeness(SearchCompleteness::Complete, None, memory)
        })
    }

    /// Certify the aggregate output's independent columns under the same owner.
    ///
    /// # Errors
    /// Returns unknown ids, conflicting certificates or original physical refusal.
    pub fn aggregate(
        &mut self,
        input: EvidenceId,
        columns: &[(Digest32, AggregateBound)],
    ) -> Result<EvidenceId, EvidenceWorkspaceError> {
        self.workspace.transform(input, |record, memory| {
            record.with_aggregate_bounds(columns, memory)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_core::evidence::{Frame, Information, InformationMap, SearchCompleteness};
    use purrdf_lex::allocation::{Resident, StorageError};

    #[test]
    fn contradictory_derivations_change_existing_fact_without_losing_witnesses() {
        let mut evidence = DerivationEvidence::with_admission(2, Resident);
        let opposed = evidence
            .seed(RowId::from_index(1).unwrap(), |memory| {
                EvidenceRecord::from_parts(
                    &InformationMap::uniform(Information::Opposed),
                    SearchCompleteness::Undetermined,
                    &Frame::actual(),
                    &[],
                    &[],
                    memory,
                )
            })
            .unwrap();
        let joined = evidence
            .conjunction([RowId::from_index(0).unwrap(), RowId::from_index(1).unwrap()])
            .unwrap();
        assert_eq!(joined, opposed);
        evidence
            .push_derived(RowId::from_index(2).unwrap(), joined)
            .unwrap();
        assert_eq!(
            evidence
                .record(RowId::from_index(2).unwrap())
                .unwrap()
                .information()
                .fallback(),
            Information::Opposed
        );
        assert_eq!(
            evidence
                .record(RowId::from_index(2).unwrap())
                .unwrap()
                .completeness(),
            SearchCompleteness::Undetermined
        );
        assert!(
            evidence
                .coalesce(RowId::from_index(2).unwrap(), EvidenceId::DEFAULT)
                .unwrap()
        );
        let both = evidence.record(RowId::from_index(2).unwrap()).unwrap();
        assert_eq!(both.information().fallback(), Information::Both);
        assert_eq!(both.completeness(), SearchCompleteness::Complete);
        assert!(
            !evidence
                .coalesce(RowId::from_index(2).unwrap(), opposed)
                .unwrap()
        );
        assert_eq!(evidence.row_count(), 3);
    }

    #[test]
    fn ordinary_facts_need_no_evidence_capacity_and_foreign_rows_are_refused() {
        #[derive(Debug)]
        struct NoCapacity;
        impl Admission for NoCapacity {
            fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
                if bytes == 0 {
                    Ok(())
                } else {
                    Err(StorageError::AdmissionFailed)
                }
            }
        }
        let mut evidence = DerivationEvidence::with_admission(10_000, NoCapacity);
        let default = evidence
            .conjunction((0..10_000).map(|index| RowId::from_index(index).unwrap()))
            .unwrap();
        assert_eq!(default, EvidenceId::DEFAULT);
        evidence
            .push_derived(RowId::from_index(10_000).unwrap(), default)
            .unwrap();
        assert!(
            !evidence
                .coalesce(RowId::from_index(3).unwrap(), default)
                .unwrap()
        );
        assert!(evidence.rows().is_uniform_default());
        assert!(matches!(
            evidence.push_derived(RowId::from_index(10_002).unwrap(), default),
            Err(EvidenceWorkspaceError::Column(
                EvidenceColumnError::RowOutOfBounds
            ))
        ));
        assert_eq!(evidence.row_count(), 10_001);
        assert!(matches!(
            evidence.record(RowId::from_index(10_001).unwrap()),
            Err(EvidenceWorkspaceError::Column(
                EvidenceColumnError::RowOutOfBounds
            ))
        ));
    }
}
