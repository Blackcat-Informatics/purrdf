// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Retained pre-publication storage for the original size-tiered batch merge.

use super::{Batch, ObjectIndex, Relation, RowId, TermIdentity, index, merge_batches_into};
use crate::allocation::reserve_array_in;
use purrdf_lex::allocation::{Admission, Memory, StorageError};

#[derive(Debug, Clone, Copy)]
pub(super) struct Shape {
    rows: usize,
    merged: usize,
    intermediate: usize,
    merges: usize,
}

impl Shape {
    pub(super) fn of<I: TermIdentity>(
        relation: &Relation<I>,
        additional: usize,
    ) -> Result<Self, StorageError> {
        let rows = relation
            .tail
            .iter()
            .count()
            .checked_add(additional)
            .ok_or(StorageError::SizeOverflow)?;
        let mut shape = Self {
            rows,
            merged: rows,
            intermediate: 0,
            merges: 0,
        };
        for batch in relation.batches.iter().rev() {
            if shape.merged.saturating_mul(2) < batch.len() {
                break;
            }
            shape.intermediate = shape.merged;
            shape.merged = shape
                .merged
                .checked_add(batch.live)
                .ok_or(StorageError::SizeOverflow)?;
            shape.merges += 1;
        }
        Ok(shape)
    }

    pub(super) fn batch_count(self, previous: usize) -> usize {
        previous - self.merges + 1
    }
}

#[derive(Debug)]
pub(super) struct Preparation<I: TermIdentity> {
    shape: Shape,
    rows: Vec<(I, I, RowId)>,
    outputs: [Batch<(), I>; 2],
    permutation: Vec<usize>,
    admitted: bool,
    objects: bool,
}

impl<I: TermIdentity> Preparation<I> {
    pub(super) fn new(shape: Shape, admitted: bool) -> Self {
        Self {
            shape,
            rows: Vec::new(),
            outputs: core::array::from_fn(|_| Batch::empty(admitted)),
            permutation: Vec::new(),
            admitted,
            objects: false,
        }
    }

    pub(super) fn storage_bytes(&self) -> Result<usize, StorageError> {
        let rows = self
            .rows
            .capacity()
            .checked_mul(size_of::<(I, I, RowId)>())
            .ok_or(StorageError::SizeOverflow)?;
        let permutation = self
            .permutation
            .capacity()
            .checked_mul(size_of::<usize>())
            .ok_or(StorageError::SizeOverflow)?;
        self.outputs.iter().try_fold(
            rows.checked_add(permutation)
                .and_then(|bytes| bytes.checked_add(size_of::<Self>()))
                .ok_or(StorageError::SizeOverflow)?,
            |bytes, output| {
                bytes
                    .checked_add(output.storage_bytes()?)
                    .ok_or(StorageError::SizeOverflow)
            },
        )
    }

    pub(super) fn reserve<A: Admission + ?Sized>(
        &mut self,
        shape: Shape,
        objects: bool,
        memory: &mut Memory<'_, A>,
        retired: &mut usize,
    ) -> Result<(), StorageError> {
        self.shape = shape;
        self.objects = objects;
        reserve_array_in(&mut self.rows, memory, retired, |values, memory| {
            memory.reserve(values, shape.rows)
        })?;
        self.outputs[0].reserve_in(shape.merged, memory, retired)?;
        self.outputs[1].reserve_in(shape.intermediate, memory, retired)?;
        if objects {
            reserve_array_in(&mut self.permutation, memory, retired, |values, memory| {
                memory.reserve(values, shape.merged)
            })?;
        }
        Ok(())
    }

    /// Every array and the final header slot was admitted before the new row.
    /// The original two-way merge performs no allocation while publishing.
    pub(super) fn publish(mut self: Box<Self>, relation: &mut Relation<I>) {
        self.rows.clear();
        assert!(self.rows.capacity() >= self.shape.rows);
        self.rows.extend(relation.tail.iter());
        assert_eq!(
            self.rows.len(),
            self.shape.rows,
            "the original prospective tail stays unchanged"
        );
        relation.tail.data.clear();
        self.rows
            .sort_unstable_by_key(|&(subject, object, _)| (subject, object));
        let mut current = self.shape.merges % 2;
        self.outputs[current].fill_sorted(&self.rows);
        let mut retired = 0_usize;
        for _ in 0..self.shape.merges {
            let left = relation
                .batches
                .pop()
                .expect("the admitted original merge span");
            let (first, second) = self.outputs.split_at_mut(1);
            let result = if current == 0 {
                merge_batches_into(&left, &first[0], &mut second[0])
            } else {
                merge_batches_into(&left, &second[0], &mut first[0])
            };
            match result {
                Ok(()) => {}
                Err(never) => match never {},
            }
            retired = retired
                .checked_add(left.retire())
                .expect("retired original batch layouts");
            current = 1 - current;
        }
        assert_eq!(current, 0);
        let batch = core::mem::replace(&mut self.outputs[0], Batch::empty(self.admitted));
        assert_eq!(
            batch.len(),
            self.shape.merged,
            "live set keys are disjoint before consolidation"
        );
        if self.objects {
            self.permutation.clear();
            assert!(self.permutation.capacity() >= batch.len());
            self.permutation.extend(0..batch.len());
            index::sort_row_positions(
                &mut self.permutation,
                &[batch.data.column(0), batch.data.column(1)],
                &[1, 0],
            );
            batch
                .object_index
                .set(ObjectIndex {
                    perm: core::mem::take(&mut self.permutation),
                    admitted: self.admitted,
                })
                .expect("the original prepared batch has no published permutation");
        }
        assert!(
            relation.batches.capacity() > relation.batches.len(),
            "the final batch header slot is admitted"
        );
        relation.batches.push(batch);
        relation.retired_bytes = relation
            .retired_bytes
            .checked_add(retired)
            .and_then(|bytes| bytes.checked_add(self.retire()))
            .expect("retired sealing storage fits its original grant");
    }

    pub(super) fn retire(self: Box<Self>) -> usize {
        let bytes = if self.admitted {
            self.storage_bytes()
                .expect("original seal preparation layouts")
        } else {
            0
        };
        drop(self);
        bytes
    }
}
