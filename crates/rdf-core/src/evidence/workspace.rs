// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original admission ownership for the lifetime of a query's evidence table.

use purrdf_hash::hex::Digest32;
use purrdf_lex::allocation::{Admission, Memory, StorageError};

use super::{
    EvidenceColumn, EvidenceColumnError, EvidenceCombination, EvidenceDictionary,
    EvidenceDictionaryError, EvidenceId, EvidenceRecord, EvidenceRecordError,
};

/// A failed native table mutation or an attempt to reuse its aborted owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceWorkspaceError {
    /// A prior physical or invariant failure aborted this query's evidence owner.
    Aborted,
    /// The original immutable record could not be constructed.
    Record(EvidenceRecordError),
    /// The original table operation failed.
    Dictionary(EvidenceDictionaryError),
    /// An original aligned row column could not be produced or changed.
    Column(EvidenceColumnError),
}

impl core::fmt::Display for EvidenceWorkspaceError {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Aborted => {
                out.write_str("query evidence owner was aborted by its original failure")
            }
            Self::Record(error) => error.fmt(out),
            Self::Dictionary(error) => error.fmt(out),
            Self::Column(error) => error.fmt(out),
        }
    }
}

impl std::error::Error for EvidenceWorkspaceError {}

impl From<StorageError> for EvidenceWorkspaceError {
    fn from(error: StorageError) -> Self {
        Self::Dictionary(EvidenceDictionaryError::Storage(error))
    }
}

/// A query-local table and its one original admission owner.
///
/// `S` owns the caller's grant and releases it on destruction. The table field
/// precedes that owner deliberately: Rust destroys all immutable payloads and
/// memo/index arrays before destroying their grant. Construction and mutation
/// resume the same live account, never adopting or pricing a resident copy.
/// Results retain this owner through their existing native shared-owner home.
/// There is no `Clone` that would duplicate the table or detach its admission.
#[derive(Debug)]
pub struct EvidenceWorkspace<S: Admission> {
    dictionary: EvidenceDictionary,
    live: usize,
    aborted: bool,
    admission: S,
}

impl<S: Admission> EvidenceWorkspace<S> {
    /// Own the original caller admission before any evidence payload is born.
    #[must_use]
    pub fn with_admission(admission: S) -> Self {
        Self {
            dictionary: EvidenceDictionary::default(),
            live: 0,
            aborted: false,
            admission,
        }
    }

    /// Borrow original evidence through its still-live table owner.
    ///
    /// # Errors
    /// Refuses an aborted query or an id outside this table.
    pub fn get(&self, id: EvidenceId) -> Result<&EvidenceRecord, EvidenceWorkspaceError> {
        if self.aborted {
            return Err(EvidenceWorkspaceError::Aborted);
        }
        self.dictionary
            .get(id)
            .map_err(EvidenceWorkspaceError::Dictionary)
    }

    /// Borrow canonical content order for a final result's ledger projection.
    ///
    /// # Errors
    /// An aborted owner never publishes a partially constructed result.
    pub fn canonical_index(&self) -> Result<&[(Digest32, EvidenceId)], EvidenceWorkspaceError> {
        if self.aborted {
            return Err(EvidenceWorkspaceError::Aborted);
        }
        Ok(self.dictionary.canonical_index())
    }

    /// Append a row's id under this table's original native admission.
    ///
    /// The column must be destroyed before this owner. Its ids refer to this
    /// dictionary; invalid ids are refused before an aligned row is published.
    ///
    /// # Errors
    /// Returns invalid evidence ids, original column refusal or an aborted owner.
    pub fn push_row(
        &mut self,
        column: &mut EvidenceColumn,
        id: EvidenceId,
    ) -> Result<(), EvidenceWorkspaceError> {
        self.get(id)?;
        self.column_operation(|memory| {
            column
                .push(id, memory)
                .map_err(EvidenceColumnError::Storage)
        })
    }

    /// Revise a stored row after independent derivations change its evidence.
    ///
    /// # Errors
    /// Returns invalid ids/alignment, native column refusal or an aborted owner.
    pub fn set_row(
        &mut self,
        column: &mut EvidenceColumn,
        row: usize,
        id: EvidenceId,
    ) -> Result<(), EvidenceWorkspaceError> {
        self.get(id)?;
        self.column_operation(|memory| column.set(row, id, memory))
    }

    /// Copy selected aligned ids under the same original admission and table.
    ///
    /// The returned column must be destroyed before this owner.
    ///
    /// # Errors
    /// Returns alignment/storage refusal or an aborted owner.
    pub fn select_rows(
        &mut self,
        column: &EvidenceColumn,
        rows: &[usize],
    ) -> Result<EvidenceColumn, EvidenceWorkspaceError> {
        self.column_operation(|memory| column.select(rows, memory))
    }

    fn column_operation<R>(
        &mut self,
        operation: impl FnOnce(&mut Memory<'_, S>) -> Result<R, EvidenceColumnError>,
    ) -> Result<R, EvidenceWorkspaceError> {
        if self.aborted {
            return Err(EvidenceWorkspaceError::Aborted);
        }
        let mut memory = Memory::resume(&mut self.admission, self.live);
        let result = operation(&mut memory).map_err(EvidenceWorkspaceError::Column);
        self.live = memory.admitted_bytes();
        self.aborted = result.is_err();
        result
    }

    /// Construct and intern under the same admission, before publication.
    ///
    /// The factory's failed temporaries die inside the original rollback scope.
    /// Persistent table growth occurs outside that scope and remains covered
    /// even if a later mutation refuses. Any failed mutation aborts this owner;
    /// its first typed failure is returned directly, never rendered as a gap.
    ///
    /// # Errors
    /// Returns the original record/table failure or refusal of an aborted owner.
    pub fn intern(
        &mut self,
        factory: impl FnOnce(&mut Memory<'_, S>) -> Result<EvidenceRecord, EvidenceRecordError>,
    ) -> Result<EvidenceId, EvidenceWorkspaceError> {
        self.construct(|_, memory| factory(memory).map_err(EvidenceWorkspaceError::Record))
    }

    /// Transform one immutable row record under the same original admission.
    ///
    /// The factory borrows the actual table entry and builds its output natively;
    /// no resident record clone is adopted or priced afterwards.
    ///
    /// # Errors
    /// Returns unknown ids, original record/table refusal or an aborted owner.
    pub fn transform(
        &mut self,
        id: EvidenceId,
        factory: impl FnOnce(
            &EvidenceRecord,
            &mut Memory<'_, S>,
        ) -> Result<EvidenceRecord, EvidenceRecordError>,
    ) -> Result<EvidenceId, EvidenceWorkspaceError> {
        self.construct(|dictionary, memory| {
            let record = dictionary
                .get(id)
                .map_err(EvidenceWorkspaceError::Dictionary)?;
            factory(record, memory).map_err(EvidenceWorkspaceError::Record)
        })
    }

    fn construct(
        &mut self,
        factory: impl FnOnce(
            &EvidenceDictionary,
            &mut Memory<'_, S>,
        ) -> Result<EvidenceRecord, EvidenceWorkspaceError>,
    ) -> Result<EvidenceId, EvidenceWorkspaceError> {
        if self.aborted {
            return Err(EvidenceWorkspaceError::Aborted);
        }
        let mut memory = Memory::resume(&mut self.admission, self.live);
        let result = memory
            .try_scope(|memory| factory(&self.dictionary, memory))
            .and_then(|record| {
                self.dictionary
                    .intern(record, &mut memory)
                    .map_err(EvidenceWorkspaceError::Dictionary)
            });
        self.live = memory.admitted_bytes();
        self.aborted = result.is_err();
        result
    }

    /// Run an operator's original scratch or destination allocation on this account.
    ///
    /// Persistent destinations must not be mutated inside a rollback scope. A
    /// caller constructing temporary values may use `Memory::try_scope` and must
    /// destroy its values before refunding their native layouts.
    ///
    /// # Errors
    /// Returns the original typed operation error and aborts further publication.
    pub fn native_operation<R>(
        &mut self,
        operation: impl FnOnce(
            &EvidenceDictionary,
            &mut Memory<'_, S>,
        ) -> Result<R, EvidenceWorkspaceError>,
    ) -> Result<R, EvidenceWorkspaceError> {
        if self.aborted {
            return Err(EvidenceWorkspaceError::Aborted);
        }
        let mut memory = Memory::resume(&mut self.admission, self.live);
        let result = operation(&self.dictionary, &mut memory);
        self.live = memory.admitted_bytes();
        self.aborted = result.is_err();
        result
    }

    /// Combine the complete Cartesian proof family without expanding its product.
    ///
    /// Every id belongs to this table. Scratch contains borrowed original records,
    /// never cloned payloads. An empty factor has no derivation; zero factors
    /// produce the ordinary conjunction identity.
    ///
    /// # Errors
    /// Returns original reference, record or native storage failures.
    pub fn factorized(
        &mut self,
        factors: &[&[EvidenceId]],
    ) -> Result<Option<EvidenceId>, EvidenceWorkspaceError> {
        for factor in factors {
            for &id in *factor {
                self.get(id)?;
            }
        }
        if factors.iter().any(|factor| factor.is_empty()) {
            return Ok(None);
        }
        if factors
            .iter()
            .all(|factor| factor.iter().all(|id| id.is_default()))
        {
            return Ok(Some(EvidenceId::DEFAULT));
        }
        self.construct(|dictionary, memory| {
            let storage =
                |error| EvidenceWorkspaceError::Dictionary(EvidenceDictionaryError::Storage(error));
            let mut groups = Vec::new();
            for factor in factors {
                let mut records = Vec::new();
                for &id in *factor {
                    let record = dictionary
                        .get(id)
                        .map_err(EvidenceWorkspaceError::Dictionary)?;
                    memory.reserve_for_push(&mut records).map_err(storage)?;
                    records.push(record);
                }
                memory.reserve_for_push(&mut groups).map_err(storage)?;
                groups.push(records);
            }
            let references = memory
                .collect(groups.iter().map(Vec::as_slice))
                .map_err(storage)?;
            let record = EvidenceRecord::factorized(&references, memory)
                .map_err(storage)?
                .expect("all factors were checked nonempty before construction");
            memory.release_vec(references).map_err(storage)?;
            for records in &mut groups {
                memory
                    .release_vec(core::mem::take(records))
                    .map_err(storage)?;
            }
            memory.release_vec(groups).map_err(storage)?;
            Ok(record)
        })
        .map(Some)
    }

    /// Combine two rows through this original query's pair memo and admission.
    ///
    /// # Errors
    /// Returns the original table/storage failure or refusal of an aborted owner.
    pub fn combine(
        &mut self,
        left: EvidenceId,
        right: EvidenceId,
        law: EvidenceCombination,
    ) -> Result<EvidenceId, EvidenceWorkspaceError> {
        if self.aborted {
            return Err(EvidenceWorkspaceError::Aborted);
        }
        let mut memory = Memory::resume(&mut self.admission, self.live);
        let result = self
            .dictionary
            .combine(left, right, law, &mut memory)
            .map_err(EvidenceWorkspaceError::Dictionary);
        self.live = memory.admitted_bytes();
        self.aborted = result.is_err();
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::{Frame, Information, InformationMap, SearchCompleteness};
    use purrdf_lex::allocation::{Resident, StorageError};

    #[test]
    fn default_join_identity_is_not_duplicate_information_identity() {
        let mut owner = EvidenceWorkspace::with_admission(Resident);
        let opposed = owner
            .intern(|memory| {
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
        assert_eq!(
            owner
                .combine(
                    EvidenceId::DEFAULT,
                    opposed,
                    EvidenceCombination::Conjunction
                )
                .unwrap(),
            opposed
        );
        let both = owner
            .combine(
                EvidenceId::DEFAULT,
                opposed,
                EvidenceCombination::Alternatives,
            )
            .unwrap();
        assert_eq!(
            owner.get(both).unwrap().information().fallback(),
            Information::Both
        );
        assert_eq!(
            owner.get(both).unwrap().completeness(),
            SearchCompleteness::Complete
        );
        let live = owner.live;
        for _ in 0..1000 {
            assert_eq!(
                owner
                    .combine(
                        opposed,
                        EvidenceId::DEFAULT,
                        EvidenceCombination::Alternatives
                    )
                    .unwrap(),
                both
            );
        }
        assert_eq!(owner.live, live);
    }

    #[test]
    fn failed_native_factory_aborts_without_publishing_a_semantic_gap() {
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
        let mut owner = EvidenceWorkspace::with_admission(Refuse);
        let error = owner
            .intern(|memory| {
                let information = InformationMap::from_entries(
                    Information::Neither,
                    &[(
                        crate::evidence::StandpointKey::Unscoped,
                        Information::Supported,
                    )],
                    memory,
                )?;
                EvidenceRecord::from_parts(
                    &information,
                    SearchCompleteness::Complete,
                    &Frame::actual(),
                    &[],
                    &[],
                    memory,
                )
            })
            .unwrap_err();
        assert_eq!(
            error,
            EvidenceWorkspaceError::Record(EvidenceRecordError::Storage(
                StorageError::AdmissionFailed
            ))
        );
        assert_eq!(
            owner.get(EvidenceId::DEFAULT),
            Err(EvidenceWorkspaceError::Aborted)
        );
        assert_eq!(owner.live, 0);
    }
}
