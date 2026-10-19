// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native evidence ids beside solution cells, without a default-path row array.

use purrdf_lex::allocation::{Admission, Memory, StorageError};

use super::EvidenceId;

/// A physical refusal or a broken row/evidence alignment invariant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceColumnError {
    /// Native admission, layout or allocator refusal.
    Storage(StorageError),
    /// A row index is outside the original column.
    RowOutOfBounds,
}

impl purrdf_hash::error::StructuredError for EvidenceColumnError {
    purrdf_hash::error_visit! {
        Storage(cause) => "evidence-column-storage" { "cause" => Cause(cause) },
        RowOutOfBounds => "evidence-column-row-out-of-bounds" {},
    }
}

impl core::fmt::Display for EvidenceColumnError {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Storage(error) => error.fmt(out),
            Self::RowOutOfBounds => out.write_str("evidence row index outside its solution column"),
        }
    }
}

impl std::error::Error for EvidenceColumnError {}

purrdf_lex::variant_from!(EvidenceColumnError { Storage(StorageError) });

/// One id per row, with allocation-free uniform default representation.
///
/// The result owner retains the original admission through destruction of this
/// array. It must not clone a resident array and charge its capacity afterwards.
#[derive(Debug, Default)]
pub struct EvidenceColumn {
    len: usize,
    ids: Vec<EvidenceId>,
}

impl EvidenceColumn {
    /// A default column aligned with an existing default-evidence row bag.
    #[must_use]
    pub const fn uniform_default(len: usize) -> Self {
        Self {
            len,
            ids: Vec::new(),
        }
    }

    /// Number of aligned rows, including uniform rows without an array.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Whether there are no aligned rows.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Whether all rows use the common allocation-free representation.
    #[must_use]
    pub fn is_uniform_default(&self) -> bool {
        self.ids.is_empty()
    }

    /// Borrow one original row's id; invalid alignment is never default evidence.
    ///
    /// # Errors
    /// Refuses an index outside the solution bag.
    pub fn get(&self, index: usize) -> Result<EvidenceId, EvidenceColumnError> {
        if index >= self.len {
            return Err(EvidenceColumnError::RowOutOfBounds);
        }
        Ok(if self.ids.is_empty() {
            EvidenceId::DEFAULT
        } else {
            self.ids[index]
        })
    }

    /// Append beside a newly produced row under its original native admission.
    ///
    /// # Errors
    /// Reports physical cardinality overflow or admission/allocation refusal.
    pub fn push<S: Admission + ?Sized>(
        &mut self,
        id: EvidenceId,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        let next = self.len.checked_add(1).ok_or(StorageError::SizeOverflow)?;
        if self.ids.is_empty() {
            if !id.is_default() {
                let materialized = memory.scope(|memory| {
                    let mut ids = Vec::new();
                    memory.reserve(&mut ids, next)?;
                    ids.resize(self.len, EvidenceId::DEFAULT);
                    ids.push(id);
                    Ok(ids)
                })?;
                self.ids = materialized;
            }
        } else {
            memory.push(&mut self.ids, id)?;
        }
        self.len = next;
        Ok(())
    }

    /// Copy selected ids in the exact caller-supplied row order.
    ///
    /// Selection permits repeated indices, preserving bags and join multiplicity.
    /// A sort, slice or filter supplies its original indices rather than sorting
    /// ids independently from solution cells.
    ///
    /// # Errors
    /// Refuses invalid row indices before allocation, or physical storage refusal.
    pub fn select<S: Admission + ?Sized>(
        &self,
        indices: &[usize],
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, EvidenceColumnError> {
        self.select_indices(indices.iter().copied(), memory)
    }

    /// Copy the original column under the destination's native admission.
    ///
    /// Uniform default evidence remains allocation-free. This copies only ids;
    /// the destination must retain the same original immutable evidence table.
    ///
    /// # Errors
    /// Returns the original physical refusal before publishing a copied column.
    pub fn copy<S: Admission + ?Sized>(
        &self,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, EvidenceColumnError> {
        self.select_indices(0..self.len, memory)
    }

    fn select_indices<S: Admission + ?Sized>(
        &self,
        indices: impl ExactSizeIterator<Item = usize> + Clone,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, EvidenceColumnError> {
        if indices.clone().any(|index| index >= self.len) {
            return Err(EvidenceColumnError::RowOutOfBounds);
        }
        if self.ids.is_empty() || indices.clone().all(|index| self.ids[index].is_default()) {
            return Ok(Self::uniform_default(indices.len()));
        }
        let len = indices.len();
        let ids = memory.scope(|memory| memory.collect(indices.map(|index| self.ids[index])))?;
        Ok(Self { len, ids })
    }

    /// Update a stored fact after another derivation adds independent evidence.
    ///
    /// Row identities and their order remain fixed. The first nondefault update
    /// admits the complete original id column before materialization. Returning
    /// to uniform default destroys its array before releasing native capacity.
    ///
    /// # Errors
    /// Refuses an invalid row or original physical admission/allocation failure.
    pub fn set<S: Admission + ?Sized>(
        &mut self,
        index: usize,
        id: EvidenceId,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), EvidenceColumnError> {
        if index >= self.len {
            return Err(EvidenceColumnError::RowOutOfBounds);
        }
        if self.ids.is_empty() {
            if !id.is_default() {
                let ids = memory.scope(|memory| {
                    memory.collect((0..self.len).map(|row| {
                        if row == index {
                            id
                        } else {
                            EvidenceId::DEFAULT
                        }
                    }))
                })?;
                self.ids = ids;
            }
        } else {
            self.ids[index] = id;
            if id.is_default() && self.ids.iter().all(|stored| stored.is_default()) {
                memory.release_vec(core::mem::take(&mut self.ids))?;
            }
        }
        Ok(())
    }

    /// Destroy the retained array before refunding its original native capacity.
    ///
    /// # Errors
    /// Propagates the original admission callback's refund failure.
    pub fn release<S: Admission + ?Sized>(
        self,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        memory.release_vec(self.ids)
    }
}

#[cfg(test)]
mod tests {
    use super::{EvidenceColumn, EvidenceColumnError};
    use crate::evidence::EvidenceId;
    use purrdf_lex::allocation::{Admission, Memory, StorageError};

    #[derive(Default)]
    struct Capacity {
        limit: usize,
        live: usize,
        peak: usize,
    }

    impl Admission for Capacity {
        fn resize(&mut self, live_bytes: usize) -> Result<(), StorageError> {
            if live_bytes > self.limit {
                return Err(StorageError::AdmissionFailed);
            }
            self.live = live_bytes;
            self.peak = self.peak.max(live_bytes);
            Ok(())
        }
    }

    #[test]
    fn default_rows_and_reordered_bags_need_no_capacity() {
        let mut capacity = Capacity::default();
        let mut memory = Memory::new(&mut capacity);
        let mut column = EvidenceColumn::uniform_default(10_000);
        column.push(EvidenceId::DEFAULT, &mut memory).unwrap();
        column.set(9_999, EvidenceId::DEFAULT, &mut memory).unwrap();
        let copied = column.copy(&mut memory).unwrap();
        assert_eq!(copied.len(), column.len());
        assert!(copied.is_uniform_default());
        copied.release(&mut memory).unwrap();
        let selected = column.select(&[10_000, 2, 2, 0], &mut memory).unwrap();
        assert_eq!(selected.len(), 4);
        assert!(selected.is_uniform_default());
        selected.release(&mut memory).unwrap();
        column.release(&mut memory).unwrap();
        assert_eq!(memory.admitted_bytes(), 0);
        assert_eq!(capacity.peak, 0);
    }

    #[test]
    fn nondefault_publication_is_refused_before_mutating_rows() {
        let mut capacity = Capacity::default();
        let mut memory = Memory::new(&mut capacity);
        let mut column = EvidenceColumn::uniform_default(3);
        assert_eq!(
            column.push(EvidenceId(1), &mut memory),
            Err(StorageError::AdmissionFailed)
        );
        assert_eq!(column.len(), 3);
        assert!(column.is_uniform_default());
        assert_eq!(column.get(2), Ok(EvidenceId::DEFAULT));
        assert_eq!(column.get(3), Err(EvidenceColumnError::RowOutOfBounds));
        assert_eq!(
            column.set(1, EvidenceId(2), &mut memory),
            Err(EvidenceColumnError::Storage(StorageError::AdmissionFailed))
        );
        assert!(column.is_uniform_default());
        assert_eq!(column.get(1), Ok(EvidenceId::DEFAULT));
        assert_eq!(memory.admitted_bytes(), 0);
    }

    #[test]
    fn original_ids_follow_duplicate_selection_and_release() {
        let mut capacity = Capacity {
            limit: usize::MAX,
            ..Capacity::default()
        };
        let mut memory = Memory::new(&mut capacity);
        let mut column = EvidenceColumn::uniform_default(2);
        column.push(EvidenceId(7), &mut memory).unwrap();
        column.push(EvidenceId(3), &mut memory).unwrap();
        let copied = column.copy(&mut memory).unwrap();
        column.set(2, EvidenceId(5), &mut memory).unwrap();
        assert_eq!(copied.get(2), Ok(EvidenceId(7)));
        assert_eq!(column.get(2), Ok(EvidenceId(5)));
        column.set(2, EvidenceId(7), &mut memory).unwrap();
        copied.release(&mut memory).unwrap();
        let selected = column.select(&[3, 2, 3, 0], &mut memory).unwrap();
        assert_eq!(selected.get(0), Ok(EvidenceId(3)));
        assert_eq!(selected.get(1), Ok(EvidenceId(7)));
        assert_eq!(selected.get(2), Ok(EvidenceId(3)));
        assert_eq!(selected.get(3), Ok(EvidenceId::DEFAULT));
        let defaults = column.select(&[1, 0, 1], &mut memory).unwrap();
        assert!(defaults.is_uniform_default());
        assert!(matches!(
            column.select(&[4], &mut memory),
            Err(EvidenceColumnError::RowOutOfBounds)
        ));
        defaults.release(&mut memory).unwrap();
        selected.release(&mut memory).unwrap();
        column.release(&mut memory).unwrap();
        assert_eq!(memory.admitted_bytes(), 0);
        assert_eq!(capacity.live, 0);
    }
}
