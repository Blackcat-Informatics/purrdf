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
    let mut order = ordinal_array(length, workspace)?;
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

fn ordinal_array(
    length: usize,
    workspace: &WorkspaceCapability,
) -> Result<AdmittedVec<usize>, EvalError> {
    let mut order = AdmittedVec::with_capacity(length, workspace)?;
    for ordinal in 0..length {
        order.push(ordinal)?;
    }
    Ok(order)
}

pub(crate) enum WindowOrder {
    Prefix(AdmittedVec<usize>),
    Permutation(AdmittedVec<usize>),
}

pub(crate) fn window_order(
    length: usize,
    keep: usize,
    workspace: &WorkspaceCapability,
    compare: impl FnMut(usize, usize) -> Result<Ordering, EvalError>,
) -> Result<WindowOrder, EvalError> {
    if keep.checked_mul(keep).is_some_and(|work| work <= length) {
        window_permutation(length, keep, workspace, compare).map(WindowOrder::Prefix)
    } else {
        window_partition(length, keep, workspace, compare).map(WindowOrder::Permutation)
    }
}

/// Partition an admitted ordinal owner, then use the original stable merge
/// only for its retained prefix. The suffix is a permutation, not ordered.
/// An introspective depth bound releases the partition owner before invoking
/// the original full sort, keeping hostile pivot sequences bounded.
pub(crate) fn window_partition(
    length: usize,
    keep: usize,
    workspace: &WorkspaceCapability,
    mut compare: impl FnMut(usize, usize) -> Result<Ordering, EvalError>,
) -> Result<AdmittedVec<usize>, EvalError> {
    let mut compare = |left: usize, right: usize| {
        compare(left, right).map(|order| order.then_with(|| left.cmp(&right)))
    };
    let mut order = ordinal_array(length, workspace)?;
    if keep == 0 {
        return Ok(order);
    }
    let (mut start, mut end) = (0, length);
    let mut depth = 2 * purrdf_xsd::exact::cost::sort_rounds(length);
    while end - start > 1 && keep < length {
        if depth == 0 {
            drop(order);
            return order_permutation(length, workspace, compare);
        }
        depth -= 1;
        // Median-of-three avoids repeatedly choosing an extreme class on
        // already ordered or periodic inputs. The depth guard still handles
        // hostile arrangements independently of this pivot heuristic.
        let mut first = start;
        let mut middle = start + (end - start) / 2;
        let mut last = end - 1;
        if compare(order[first], order[middle])?.is_gt() {
            std::mem::swap(&mut first, &mut middle);
        }
        if compare(order[middle], order[last])?.is_gt() {
            std::mem::swap(&mut middle, &mut last);
        }
        if compare(order[first], order[middle])?.is_gt() {
            std::mem::swap(&mut first, &mut middle);
        }
        let pivot_index = middle;
        let pivot = order[pivot_index];
        order.as_mut_slice().swap(pivot_index, end - 1);
        let mut split = start;
        for index in start..end - 1 {
            if compare(order[index], pivot)?.is_lt() {
                order.as_mut_slice().swap(index, split);
                split += 1;
            }
        }
        order.as_mut_slice().swap(split, end - 1);
        match split.cmp(&(keep - 1)) {
            Ordering::Equal => break,
            Ordering::Less => start = split + 1,
            Ordering::Greater => end = split,
        }
    }
    let keep = keep.min(length);
    let prefix = order_permutation(keep, workspace, |left, right| {
        compare(order[left], order[right])
    })?;
    let mut sorted = AdmittedVec::with_capacity(keep, workspace)?;
    for &index in &prefix {
        sorted.push(order[index])?;
    }
    order.as_mut_slice()[..keep].copy_from_slice(&sorted);
    Ok(order)
}

/// Keep only the best input ordinals in a max heap. Original ordinals decide
/// otherwise equal keys, retaining the full stable sort's tie order. The same
/// admitted array becomes the sorted result; heap sorting needs no scratch.
pub(crate) fn window_permutation(
    length: usize,
    keep: usize,
    workspace: &WorkspaceCapability,
    mut compare: impl FnMut(usize, usize) -> Result<Ordering, EvalError>,
) -> Result<AdmittedVec<usize>, EvalError> {
    let mut compare = |left: usize, right: usize| {
        compare(left, right).map(|order| order.then_with(|| left.cmp(&right)))
    };
    let mut heap = AdmittedVec::with_capacity(keep.min(length), workspace)?;
    for ordinal in 0..length {
        if heap.len() < keep {
            heap.push(ordinal)?;
            let mut child = heap.len() - 1;
            while child > 0 {
                let parent = (child - 1) / 2;
                if !compare(heap[parent], heap[child])?.is_lt() {
                    break;
                }
                heap.as_mut_slice().swap(parent, child);
                child = parent;
            }
        } else if keep != 0 && compare(ordinal, heap[0])?.is_lt() {
            heap.as_mut_slice()[0] = ordinal;
            window_sift_down(heap.as_mut_slice(), &mut compare)?;
        }
    }
    for end in (1..heap.len()).rev() {
        heap.as_mut_slice().swap(0, end);
        window_sift_down(&mut heap.as_mut_slice()[..end], &mut compare)?;
    }
    Ok(heap)
}

fn window_sift_down(
    heap: &mut [usize],
    compare: &mut impl FnMut(usize, usize) -> Result<Ordering, EvalError>,
) -> Result<(), EvalError> {
    let mut parent = 0usize;
    loop {
        let left = parent
            .checked_mul(2)
            .and_then(|index| index.checked_add(1))
            .ok_or(EvalError::WorkspaceBoundOverflow)?;
        if left >= heap.len() {
            return Ok(());
        }
        let right = left
            .checked_add(1)
            .ok_or(EvalError::WorkspaceBoundOverflow)?;
        let child = if right < heap.len() && compare(heap[left], heap[right])?.is_lt() {
            right
        } else {
            left
        };
        if !compare(heap[parent], heap[child])?.is_lt() {
            return Ok(());
        }
        heap.swap(parent, child);
        parent = child;
    }
}
