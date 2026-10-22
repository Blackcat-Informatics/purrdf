// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Retained pre-publication storage for the original size-tiered batch merge.

use super::{Batch, MergeFrame, ObjectIndex, Relation, RowId, TermIdentity, index};
use crate::allocation::reserve_array_in;
use purrdf_lex::allocation::{Admission, Memory, StorageError};
use purrdf_lex::walk::HeapSortFrame;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Shape {
    rows: usize,
    merged: usize,
    intermediate: usize,
    merges: usize,
    additional: usize,
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
            additional,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Collect,
    Sort,
    Fill,
    Merge,
    Objects,
    SortObjects,
    Publication,
    Ready,
}

#[derive(Debug)]
pub(super) struct Preparation<I: TermIdentity> {
    shape: Shape,
    rows: Vec<(I, I, RowId)>,
    outputs: [Batch<(), I>; 2],
    permutation: Vec<usize>,
    admitted: bool,
    objects: bool,
    phase: Phase,
    cursor: usize,
    current: usize,
    merging: usize,
    merge: Option<MergeFrame>,
    sort: Option<HeapSortFrame>,
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
            phase: Phase::Collect,
            cursor: 0,
            current: shape.merges % 2,
            merging: 0,
            merge: None,
            sort: None,
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
        assert_eq!(self.shape, shape, "preparation pins the original relation");
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

    /// Prepare only existing identities. A refused dictionary transaction can
    /// roll back its new identities without invalidating retained sorting work.
    pub(super) fn resume(
        &mut self,
        tail: &super::Tail<I>,
        batches: &[Batch<(), I>],
        mut allow: impl FnMut(usize) -> bool,
    ) -> Result<bool, StorageError> {
        loop {
            match self.phase {
                Phase::Collect => {
                    while self.cursor < tail.len() {
                        if !allow(1) {
                            return Ok(false);
                        }
                        if let Some(([subject, object], Some(row))) = tail.data.row(self.cursor) {
                            self.rows.push((subject, object, row));
                        }
                        self.cursor += 1;
                    }
                    assert_eq!(self.rows.len() + self.shape.additional, self.shape.rows);
                    self.sort = Some(HeapSortFrame::new(self.rows.len()));
                    self.phase = Phase::Sort;
                }
                Phase::Sort => {
                    let complete = self.sort.as_mut().expect("tail sorter").resume(
                        &mut self.rows,
                        |a, b| Ok::<_, core::convert::Infallible>((a.0, a.1).cmp(&(b.0, b.1))),
                        || allow(1),
                    );
                    if !complete.unwrap_or_else(|never| match never {}) {
                        return Ok(false);
                    }
                    self.sort = None;
                    self.cursor = 0;
                    self.phase = Phase::Fill;
                }
                Phase::Fill => {
                    while let Some(&(subject, object, row)) = self.rows.get(self.cursor) {
                        if !allow(1) {
                            return Ok(false);
                        }
                        self.outputs[self.current].push_reserved(subject, object, row, ());
                        self.cursor += 1;
                    }
                    self.phase = Phase::Merge;
                }
                Phase::Merge => {
                    if self.merging == self.shape.merges {
                        assert_eq!(self.current, 0);
                        self.cursor = 0;
                        self.phase = if self.objects {
                            Phase::Objects
                        } else {
                            Phase::Publication
                        };
                        continue;
                    }
                    let left = &batches[batches.len() - 1 - self.merging];
                    let (first, second) = self.outputs.split_at_mut(1);
                    let frame = self.merge.get_or_insert_with(MergeFrame::default);
                    let complete = if self.current == 0 {
                        frame.resume(left, &first[0], &mut second[0], &mut allow)
                    } else {
                        frame.resume(left, &second[0], &mut first[0], &mut allow)
                    };
                    if !complete.unwrap_or_else(|never| match never {}) || !allow(1) {
                        return Ok(false);
                    }
                    self.merge = None;
                    self.merging += 1;
                    self.current = 1 - self.current;
                }
                Phase::Objects => {
                    while self.cursor < self.outputs[0].len() {
                        if !allow(1) {
                            return Ok(false);
                        }
                        self.permutation.push(self.cursor);
                        self.cursor += 1;
                    }
                    self.sort = Some(HeapSortFrame::new(self.permutation.len()));
                    self.phase = Phase::SortObjects;
                }
                Phase::SortObjects => {
                    let batch = &self.outputs[0];
                    let columns = [batch.data.column(0), batch.data.column(1)];
                    let complete = self.sort.as_mut().expect("object sorter").resume(
                        &mut self.permutation,
                        |&a, &b| {
                            Ok::<_, core::convert::Infallible>(index::compare_row_positions(
                                a,
                                b,
                                &columns,
                                &[1, 0],
                            ))
                        },
                        || allow(1),
                    );
                    if !complete.unwrap_or_else(|never| match never {}) {
                        return Ok(false);
                    }
                    self.sort = None;
                    self.phase = Phase::Publication;
                }
                Phase::Publication => {
                    // Four column/weight shifts, permutation adjustment/insertion,
                    // four bounded binary searches and the original header moves.
                    // Reserve this indivisible suffix before minting identities.
                    let cost = if self.shape.additional == 0 {
                        0
                    } else {
                        self.shape
                            .merged
                            .checked_mul(6)
                            .and_then(|n| {
                                n.checked_add(4 * self.shape.merged.bit_width() as usize + 4)
                            })
                            .ok_or(StorageError::SizeOverflow)?
                    }
                    .checked_add(self.shape.merges)
                    .ok_or(StorageError::SizeOverflow)?;
                    if !allow(cost) {
                        return Ok(false);
                    }
                    self.phase = Phase::Ready;
                }
                Phase::Ready => return Ok(true),
            }
        }
    }

    /// Commit the admitted suffix without allocation or another source callback.
    pub(super) fn publish(
        mut self: Box<Self>,
        relation: &mut Relation<I>,
        incoming: Option<(I, I, RowId)>,
    ) {
        assert_eq!(self.phase, Phase::Ready);
        assert_eq!(usize::from(incoming.is_some()), self.shape.additional);
        if let Some((subject, object, row)) = incoming {
            let batch = &mut self.outputs[0];
            let position = batch
                .data
                .column(0)
                .partition_point(|&value| value < subject);
            let end = batch
                .data
                .column(0)
                .partition_point(|&value| value <= subject);
            let position = position
                + batch.data.column(1)[position..end].partition_point(|&value| value < object);
            let object_position = if self.objects {
                self.permutation.partition_point(|&p| {
                    (batch.data.column(1)[p], batch.data.column(0)[p]) < (object, subject)
                })
            } else {
                0
            };
            batch
                .data
                .insert_reserved(position, [subject, object], Some(row));
            assert!(batch.weight.capacity() > batch.weight.len());
            batch.weight.insert(position, ());
            batch.live += 1;
            if self.objects {
                for p in &mut self.permutation {
                    if *p >= position {
                        *p += 1;
                    }
                }
                self.permutation.insert(object_position, position);
            }
        }
        relation.tail.data.clear();
        let mut retired = 0_usize;
        for _ in 0..self.shape.merges {
            let left = relation
                .batches
                .pop()
                .expect("prepared original merge span");
            retired = retired
                .checked_add(left.retire())
                .expect("retired original batch layouts");
        }
        let batch = core::mem::replace(&mut self.outputs[0], Batch::empty(self.admitted));
        assert_eq!(
            batch.len(),
            self.shape.merged,
            "live set keys are disjoint before consolidation"
        );
        if self.objects {
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

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_lex::allocation::Resident;

    #[test]
    fn every_seal_credit_cut_preserves_existing_rows_and_exact_work() {
        let original = |rows| {
            let mut relation = Relation::<u64> {
                maintain_objects: true,
                ..Relation::default()
            };
            for index in 0..rows {
                relation.insert(index % 7, index, RowId::from_index(index).unwrap());
            }
            for index in [4, 18] {
                relation.remove(index % 7, index).unwrap();
            }
            relation
                .reserve_row_in(&mut Memory::new(&mut Resident), &mut 0)
                .unwrap();
            relation
        };
        let rows_of = |relation: &Relation<u64>| {
            let mut rows = relation.tail.iter().collect::<Vec<_>>();
            for batch in &relation.batches {
                rows.extend((0..batch.len()).filter_map(|p| batch.row_at(p)));
            }
            rows.sort_unstable();
            rows
        };
        for rows in [63, 127, 191, 319] {
            let mut reference = original(rows);
            let mut total = 0;
            assert!(
                reference
                    .prepare_row_work(|cost| {
                        total += cost;
                        true
                    })
                    .unwrap()
            );
            let incoming = (3, 1000, RowId::from_index(rows).unwrap());
            let mut expected = rows_of(&reference);
            expected.push(incoming);
            expected.sort_unstable_by_key(|&(subject, object, _)| (subject, object));
            for limit in 0..=total {
                let mut relation = original(rows);
                let before = rows_of(&relation);
                let headers = relation.batches.len();
                let mut used = 0;
                let complete = relation
                    .prepare_row_work(|cost| {
                        if used + cost > limit {
                            false
                        } else {
                            used += cost;
                            true
                        }
                    })
                    .unwrap();
                assert_eq!(rows_of(&relation), before);
                assert_eq!(relation.row_count(), before.len());
                assert_eq!(relation.batches.len(), headers);
                if !complete {
                    assert!(
                        relation
                            .prepare_row_work(|cost| {
                                used += cost;
                                true
                            })
                            .unwrap()
                    );
                }
                assert_eq!(used, total, "cut {limit} never repeats admitted work");
                relation.insert_reserved(incoming.0, incoming.1, incoming.2);
                assert!(relation.tail.is_empty());
                let batch = &relation.batches[0];
                assert_eq!(
                    (0..batch.len())
                        .map(|p| batch.row_at(p).unwrap())
                        .collect::<Vec<_>>(),
                    expected
                );
                let order = &batch.object_index.get().unwrap().perm;
                assert!(order.windows(2).all(|pair| {
                    let a = batch.key_at(pair[0]);
                    let b = batch.key_at(pair[1]);
                    (a.1, a.0) < (b.1, b.0)
                }));
            }
        }
    }
}
