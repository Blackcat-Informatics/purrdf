// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Query-local immutable record interning without insertion-dependent identities.

use purrdf_hash::hex::Digest32;
use purrdf_lex::allocation::{Admission, Memory, StorageError};

use super::{EvidenceCombination, EvidenceId, EvidenceRecord};

/// A broken identity/reference invariant or native storage refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceDictionaryError {
    /// The id is not owned by this query's record table.
    InvalidId,
    /// Different canonical records produced the same content digest.
    IdentityCollision,
    /// Native physical refusal; it never becomes semantic missing knowledge.
    Storage(StorageError),
}

impl From<StorageError> for EvidenceDictionaryError {
    fn from(error: StorageError) -> Self {
        Self::Storage(error)
    }
}

impl core::fmt::Display for EvidenceDictionaryError {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidId => out.write_str("evidence id outside its query-local dictionary"),
            Self::IdentityCollision => {
                out.write_str("different evidence records have the same canonical identity")
            }
            Self::Storage(error) => error.fmt(out),
        }
    }
}

impl std::error::Error for EvidenceDictionaryError {}

/// One query's immutable record owners and sorted content lookup.
///
/// Id zero is virtual: a default-only query allocates neither an index nor a
/// record array. Other ids remain stable as the table grows. The eventual result
/// owner retains this entire table and its original native admission; an id must
/// never be detached from that owner or interpreted in another query's table.
#[derive(Debug, Default)]
pub struct EvidenceDictionary {
    default: EvidenceRecord,
    records: Vec<EvidenceRecord>,
    index: Vec<(Digest32, EvidenceId)>,
    pairs: Vec<((EvidenceCombination, EvidenceId, EvidenceId), EvidenceId)>,
}

impl EvidenceDictionary {
    /// Number of materialized nondefault records.
    #[must_use]
    pub fn materialized_len(&self) -> usize {
        self.records.len()
    }

    /// Borrow the original immutable record behind a query-local id.
    ///
    /// # Errors
    /// Refuses ids outside this dictionary instead of substituting default.
    pub fn get(&self, id: EvidenceId) -> Result<&EvidenceRecord, EvidenceDictionaryError> {
        if id.is_default() {
            return Ok(&self.default);
        }
        self.records
            .get(id.0 - 1)
            .ok_or(EvidenceDictionaryError::InvalidId)
    }

    /// Borrow canonical content order for a final ledger's stable remapping.
    #[must_use]
    pub fn canonical_index(&self) -> &[(Digest32, EvidenceId)] {
        &self.index
    }

    /// Combine original records once per commutative pair and operator law.
    ///
    /// A default/default operation allocates nothing. Default is the truth-meet
    /// identity, but never the information-join identity: default plus opposition
    /// must retain both witnesses. Memo keys are query-local execution details
    /// and do not enter any published content identity.
    ///
    /// # Errors
    /// Refuses invalid ids, identity collision or physical storage failure.
    pub fn combine<S: Admission + ?Sized>(
        &mut self,
        left: EvidenceId,
        right: EvidenceId,
        law: EvidenceCombination,
        memory: &mut Memory<'_, S>,
    ) -> Result<EvidenceId, EvidenceDictionaryError> {
        self.get(left)?;
        self.get(right)?;
        if left == right {
            return Ok(left);
        }
        if law == EvidenceCombination::Conjunction {
            if left.is_default() {
                return Ok(right);
            }
            if right.is_default() {
                return Ok(left);
            }
        }
        let key = (law, left.min(right), left.max(right));
        let position = match self.pairs.binary_search_by_key(&key, |entry| entry.0) {
            Ok(position) => return Ok(self.pairs[position].1),
            Err(position) => position,
        };
        // Reserve persistent memo storage before constructing a scoped child.
        // No failed child scope may refund this still-live destination capacity.
        let next = self
            .pairs
            .len()
            .checked_add(1)
            .ok_or(StorageError::SizeOverflow)?;
        memory.reserve(&mut self.pairs, next)?;
        let record = self.get(left)?.combine(self.get(right)?, law, memory)?;
        let id = self.intern(record, memory)?;
        self.pairs.insert(position, (key, id));
        Ok(id)
    }

    /// Publish an already constructed record from this same native admission.
    ///
    /// The caller constructs the payload with the original `Memory`, never a
    /// resident clone followed by capacity charging. Existing persistent arrays
    /// are reserved outside any rollback scope: if a later reserve fails, their
    /// retained capacities remain covered while the rejected record is destroyed.
    ///
    /// # Errors
    /// Reports identity collision, physical overflow, or native storage refusal.
    pub fn intern<S: Admission + ?Sized>(
        &mut self,
        record: EvidenceRecord,
        memory: &mut Memory<'_, S>,
    ) -> Result<EvidenceId, EvidenceDictionaryError> {
        if record.is_default() {
            record.release(memory)?;
            return Ok(EvidenceId::DEFAULT);
        }
        let identity = record.identity();
        let insertion = match self.index.binary_search_by_key(&identity, |entry| entry.0) {
            Ok(position) => {
                let id = self.index[position].1;
                let same = self.get(id)? == &record;
                record.release(memory)?;
                return if same {
                    Ok(id)
                } else {
                    Err(EvidenceDictionaryError::IdentityCollision)
                };
            }
            Err(position) => position,
        };
        let prepared = (|| {
            let count = self
                .records
                .len()
                .checked_add(1)
                .ok_or(StorageError::SizeOverflow)?;
            memory.reserve(&mut self.records, count)?;
            memory.reserve(&mut self.index, count)?;
            Ok::<_, StorageError>(EvidenceId(count))
        })();
        let id = match prepared {
            Ok(id) => id,
            Err(original) => {
                // Destruction precedes refund; cleanup refusal cannot disguise
                // the original failed publication as successful interning.
                let _ = record.release(memory);
                return Err(original.into());
            }
        };
        self.records.push(record);
        self.index.insert(insertion, (identity, id));
        Ok(id)
    }

    /// Destroy record payloads, then their original table/index capacities.
    ///
    /// # Errors
    /// Returns the original admission callback's refund refusal.
    pub fn release<S: Admission + ?Sized>(
        self,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        let Self {
            default,
            mut records,
            index,
            pairs,
        } = self;
        default.release(memory)?;
        while let Some(record) = records.pop() {
            record.release(memory)?;
        }
        memory.release_vec(records)?;
        memory.release_vec(index)?;
        memory.release_vec(pairs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::{Frame, InformationMap, SearchCompleteness};
    use purrdf_lex::allocation::Resident;

    #[test]
    fn repeated_content_has_one_owner_and_default_has_no_table() {
        let mut admission = Resident;
        let mut memory = Memory::new(&mut admission);
        let mut dictionary = EvidenceDictionary::default();
        let zero = dictionary
            .intern(EvidenceRecord::default(), &mut memory)
            .unwrap();
        assert!(zero.is_default());
        assert_eq!(memory.admitted_bytes(), 0);
        let mut original = None;
        for _ in 0..1000 {
            let record = EvidenceRecord::from_parts(
                &InformationMap::default(),
                SearchCompleteness::Undetermined,
                &Frame::actual(),
                &[],
                &[],
                &mut memory,
            )
            .unwrap();
            let id = dictionary.intern(record, &mut memory).unwrap();
            assert_eq!(*original.get_or_insert(id), id);
        }
        assert_eq!(dictionary.materialized_len(), 1);
        assert_eq!(dictionary.canonical_index().len(), 1);
        assert!(dictionary.get(zero).unwrap().is_default());
        assert_eq!(
            dictionary.get(EvidenceId(2)),
            Err(EvidenceDictionaryError::InvalidId)
        );
        dictionary.release(&mut memory).unwrap();
        assert_eq!(memory.admitted_bytes(), 0);
    }
}
