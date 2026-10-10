// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Allocation owners for duplicate tracking and stable row permutations.

use crate::solution::RetainedRow;
use crate::workspace::AdmittedVec;
use crate::{DetHasher, EvalError, WorkspaceAllocation, WorkspaceCapability};
use hashbrown::HashTable;
use purrdf_core::ViewTermId;
use std::cmp::Ordering;
use std::hash::BuildHasher as _;

/// Hash/equality are exact SolutionTerm cell identity. RetainedRow clones share
/// the original cell allocation and lease, including wide spilled rows.
pub(super) struct DedupState<I: ViewTermId, const ADJACENT: bool> {
    table: HashTable<RetainedRow<I>>,
    previous: Option<RetainedRow<I>>,
    allocation: Option<WorkspaceAllocation>,
    workspace: WorkspaceCapability,
}

impl<I: ViewTermId, const ADJACENT: bool> DedupState<I, ADJACENT> {
    pub(super) fn new(workspace: &WorkspaceCapability) -> Self {
        Self {
            table: HashTable::new(),
            previous: None,
            allocation: None,
            workspace: workspace.clone(),
        }
    }

    pub(super) fn keep(&mut self, row: &RetainedRow<I>) -> Result<bool, EvalError> {
        if ADJACENT {
            let keep = self.previous.as_ref() != Some(row);
            self.previous = Some(row.clone_admitted(&self.workspace)?);
            return Ok(keep);
        }
        let hash = row_hash(row);
        if self.table.find(hash, |seen| seen == row).is_some() {
            return Ok(false);
        }
        let needed = self
            .table
            .len()
            .checked_add(1)
            .ok_or(EvalError::WorkspaceBoundOverflow)?;
        if needed > self.table.capacity() {
            self.grow(needed)?;
        }
        self.table
            .insert_unique(hash, row.clone_admitted(&self.workspace)?, row_hash::<I>);
        Ok(true)
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
        let bytes = purrdf_core::hash::hash_table_allocation_bound::<RetainedRow<I>>(capacity)
            .ok_or(EvalError::WorkspaceBoundOverflow)?;
        let allocation = self
            .workspace
            .charge(u64::try_from(bytes).map_err(|_| EvalError::WorkspaceBoundOverflow)?)?;
        let mut next = HashTable::new();
        next.try_reserve(capacity, row_hash::<I>)
            .map_err(|_| EvalError::AllocationFailed {
                construct: "distinct row keys",
            })?;
        let old = std::mem::take(&mut self.table);
        for row in old {
            next.insert_unique(row_hash(&row), row, row_hash::<I>);
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
