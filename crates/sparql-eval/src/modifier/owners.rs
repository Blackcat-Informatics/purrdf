// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Allocation owners for duplicate tracking and stable row permutations.

use crate::solution::RetainedRow;
use crate::workspace::AdmittedVec;
use crate::{DetHasher, EvalError, WorkspaceAllocation, WorkspaceCapability};
use purrdf_core::ViewTermId;
use purrdf_core::hash::table::HashTable;
use std::cmp::Ordering;
use std::hash::BuildHasher as _;

/// Hash/equality are exact SolutionTerm cell identity. RetainedRow clones share
/// the original cell allocation and lease, including wide spilled rows.
pub(super) struct DedupState<K: DedupKey, const ADJACENT: bool> {
    table: HashTable<K>,
    previous: Option<K>,
    allocation: Option<WorkspaceAllocation>,
    workspace: WorkspaceCapability,
}

/// Only tracked duplicate witnesses need the survivor's output ordinal. Keeping
/// it out of ordinary keys preserves their row-only physical representation.
pub(super) trait DedupKey {
    type Id: ViewTermId;
    const COALESCE: bool;

    fn row(&self) -> &RetainedRow<Self::Id>;
    fn ordinal(&self) -> usize;
    fn from_row(row: RetainedRow<Self::Id>, ordinal: usize) -> Self;
}

impl<I: ViewTermId> DedupKey for RetainedRow<I> {
    type Id = I;
    const COALESCE: bool = false;

    fn row(&self) -> &Self {
        self
    }

    fn ordinal(&self) -> usize {
        0
    }

    fn from_row(row: Self, _ordinal: usize) -> Self {
        row
    }
}

impl<I: ViewTermId> DedupKey for (RetainedRow<I>, usize) {
    type Id = I;
    const COALESCE: bool = true;

    fn row(&self) -> &RetainedRow<I> {
        &self.0
    }

    fn ordinal(&self) -> usize {
        self.1
    }

    fn from_row(row: RetainedRow<I>, ordinal: usize) -> Self {
        (row, ordinal)
    }
}

impl<K: DedupKey, const ADJACENT: bool> DedupState<K, ADJACENT> {
    pub(super) fn new(workspace: &WorkspaceCapability) -> Self {
        Self {
            table: HashTable::new(),
            previous: None,
            allocation: None,
            workspace: workspace.clone(),
        }
    }

    /// Return the first survivor's ordinal; equality remains binding-only.
    /// Evidence is merged in the original output owner, never in hash keys.
    pub(super) fn survivor(
        &mut self,
        row: &RetainedRow<K::Id>,
        ordinal: usize,
    ) -> Result<Option<usize>, EvalError> {
        if ADJACENT {
            if let Some(previous) = &self.previous
                && previous.row() == row
            {
                return Ok(Some(previous.ordinal()));
            }
            self.previous = Some(K::from_row(row.clone_admitted(&self.workspace)?, ordinal));
            return Ok(None);
        }
        let hash = row_hash(row);
        if let Some(seen) = self.table.find(hash, |seen| seen.row() == row) {
            return Ok(Some(seen.ordinal()));
        }
        let needed = self
            .table
            .len()
            .checked_add(1)
            .ok_or(EvalError::WorkspaceBoundOverflow)?;
        if needed > self.table.capacity() {
            self.grow(needed)?;
        }
        self.table.insert_unique(
            hash,
            K::from_row(row.clone_admitted(&self.workspace)?, ordinal),
            |key| row_hash(key.row()),
        );
        Ok(None)
    }

    /// Old bucket/control bytes and all cell leases remain covered until the
    /// replacement is allocated and populated. Failure preserves the old table.
    fn grow(&mut self, needed: usize) -> Result<(), EvalError> {
        let capacity = self
            .table
            .capacity()
            .checked_mul(2)
            .ok_or(EvalError::WorkspaceBoundOverflow)?
            .max(needed);
        let bytes = purrdf_core::hash::hash_table_allocation_bound::<K>(capacity)
            .ok_or(EvalError::WorkspaceBoundOverflow)?;
        let allocation = self
            .workspace
            .charge(u64::try_from(bytes).map_err(|_| EvalError::WorkspaceBoundOverflow)?)?;
        let mut next = HashTable::new();
        next.try_reserve(capacity, |key: &K| row_hash(key.row()))
            .map_err(|_| EvalError::AllocationFailed {
                construct: "distinct row keys",
            })?;
        let old = std::mem::take(&mut self.table);
        for entry in old {
            next.insert_unique(row_hash(entry.row()), entry, |key| row_hash(key.row()));
        }
        self.table = next;
        // The old IntoIter freed its allocation before this releases its grant.
        self.allocation = Some(allocation);
        Ok(())
    }
}

fn row_hash<I: ViewTermId>(row: &RetainedRow<I>) -> u64 {
    DetHasher::default().hash_one(row)
}

/// Stable bottom-up merge of input ordinals. The two real usize arrays are
/// admitted before allocation; no merge or comparator dispatch grows either.
/// Left wins ties, and the existing ceil(log2 n) moving-side cost law applies.
pub(crate) fn order_permutation(
    length: usize,
    workspace: &WorkspaceCapability,
    mut compare: impl FnMut(usize, usize) -> Result<Ordering, EvalError>,
) -> Result<AdmittedVec<usize>, EvalError> {
    let mut order = AdmittedVec::with_capacity(length, workspace)?;
    for ordinal in 0..length {
        order.push(ordinal)?;
    }
    if length < 2 {
        return Ok(order);
    }
    let mut scratch = AdmittedVec::with_capacity(length, workspace)?;
    for _ in 0..length {
        scratch.push(0)?;
    }
    let mut run = 1usize;
    while run < length {
        let step = run
            .checked_mul(2)
            .ok_or(EvalError::WorkspaceBoundOverflow)?;
        let mut begin = 0usize;
        while begin < length {
            let middle = begin
                .checked_add(run)
                .ok_or(EvalError::WorkspaceBoundOverflow)?
                .min(length);
            let end = begin
                .checked_add(step)
                .ok_or(EvalError::WorkspaceBoundOverflow)?
                .min(length);
            let (mut left, mut right, mut output) = (begin, middle, begin);
            while left < middle && right < end {
                if compare(order[left], order[right])? != Ordering::Greater {
                    scratch.as_mut_slice()[output] = order[left];
                    left += 1;
                } else {
                    scratch.as_mut_slice()[output] = order[right];
                    right += 1;
                }
                output += 1;
            }
            while left < middle {
                scratch.as_mut_slice()[output] = order[left];
                left += 1;
                output += 1;
            }
            while right < end {
                scratch.as_mut_slice()[output] = order[right];
                right += 1;
                output += 1;
            }
            begin = end;
        }
        std::mem::swap(&mut order, &mut scratch);
        run = step;
    }
    Ok(order)
}
