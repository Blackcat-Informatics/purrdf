// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Shared ordered lookup storage with an exact, fallible native array allocation.
//!
//! Keys order lookup only; consumers keep declaration/output order separately.
//! AVL links are array indices, so widening a query never allocates tree nodes
//! through an opaque collection. Vacant slots are reused after removal.

use super::{Admission, Memory, StorageError};
use core::{cmp::Ordering, mem};

#[derive(Clone)]
struct Entry<K, V> {
    key: K,
    value: V,
    parent: Option<usize>,
    left: Option<usize>,
    right: Option<usize>,
    height: usize,
}

#[derive(Clone)]
enum Slot<K, V> {
    Live(Entry<K, V>),
    Free(Option<usize>),
}

/// One balanced lookup tree in a caller-admitted reusable slot arena.
///
/// Both empty construction and every lookup are allocation-free. The memory
/// supplied to growth/release must be the original memory of this arena.
/// Lookup lends an immutable entry; tree edits exclusively borrow a mutable entry.
pub struct OrderedTable<K, V> {
    slots: Vec<Slot<K, V>>,
    root: Option<usize>,
    free: Option<usize>,
    len: usize,
}

impl<K, V> Default for OrderedTable<K, V> {
    fn default() -> Self {
        Self {
            slots: Vec::new(),
            root: None,
            free: None,
            len: 0,
        }
    }
}

impl<K, V> OrderedTable<K, V> {
    /// Exclusive live payloads in arena order, without traversal scratch.
    /// Use the ordered immutable iterator when key order is part of a result.
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.slots.iter_mut().filter_map(|slot| match slot {
            Slot::Live(entry) => Some(&mut entry.value),
            Slot::Free(_) => None,
        })
    }

    /// Number of live associations, excluding reusable vacant slots.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Whether no live association remains.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn entry(&self, index: usize) -> &Entry<K, V> {
        match &self.slots[index] {
            Slot::Live(entry) => entry,
            Slot::Free(_) => unreachable!("scope tree link points at a vacant slot"),
        }
    }

    fn entry_mut(&mut self, index: usize) -> &mut Entry<K, V> {
        match &mut self.slots[index] {
            Slot::Live(entry) => entry,
            Slot::Free(_) => unreachable!("scope tree link points at a vacant slot"),
        }
    }

    fn find(&self, mut compare: impl FnMut(&K) -> Ordering) -> Option<usize> {
        let mut at = self.root;
        while let Some(index) = at {
            let entry = self.entry(index);
            at = match compare(&entry.key) {
                Ordering::Less => entry.left,
                Ordering::Greater => entry.right,
                Ordering::Equal => return Some(index),
            };
        }
        None
    }

    fn extreme(&self, mut index: usize, left: bool) -> usize {
        while let Some(child) = if left {
            self.entry(index).left
        } else {
            self.entry(index).right
        } {
            index = child;
        }
        index
    }

    fn neighbor(&self, mut index: usize, forward: bool) -> Option<usize> {
        let child = if forward {
            self.entry(index).right
        } else {
            self.entry(index).left
        };
        if let Some(child) = child {
            return Some(self.extreme(child, forward));
        }
        while let Some(parent) = self.entry(index).parent {
            let from = if forward {
                self.entry(parent).left
            } else {
                self.entry(parent).right
            };
            if from == Some(index) {
                return Some(parent);
            }
            index = parent;
        }
        None
    }

    /// Walk key order using parent links, without a heap traversal stack.
    pub fn ordered(&self) -> OrderedIter<'_, K, V> {
        OrderedIter {
            table: self,
            front: self.root.map(|root| self.extreme(root, true)),
            back: self.root.map(|root| self.extreme(root, false)),
            remaining: self.len,
        }
    }

    /// Borrow the first stored key and payload without copying their owners.
    pub fn first_key_value(&self) -> Option<(&K, &V)> {
        let entry = self.entry(self.extreme(self.root?, true));
        Some((&entry.key, &entry.value))
    }

    /// Borrow the last stored key and payload without copying their owners.
    pub fn last_key_value(&self) -> Option<(&K, &V)> {
        let entry = self.entry(self.extreme(self.root?, false));
        Some((&entry.key, &entry.value))
    }

    /// Borrow a key and its value through the caller's original ordering.
    pub fn get_key_value_by(&self, compare: impl FnMut(&K) -> Ordering) -> Option<(&K, &V)> {
        self.find(compare).map(|index| {
            let entry = self.entry(index);
            (&entry.key, &entry.value)
        })
    }

    /// Mutate only a found payload; lookup acquires no storage.
    pub fn get_mut_by(&mut self, compare: impl FnMut(&K) -> Ordering) -> Option<&mut V> {
        let index = self.find(compare)?;
        Some(&mut self.entry_mut(index).value)
    }

    /// Lookup a borrowed spelling without manufacturing an owned key.
    /// The comparator returns query-versus-stored-key ordering.
    pub fn get_by(&self, compare: impl FnMut(&K) -> Ordering) -> Option<&V> {
        self.find(compare).map(|index| &self.entry(index).value)
    }

    /// Iterate live slots without a traversal allocation. This is arena order,
    /// not key order; output/declaration order remains the consumer's own law.
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> + Clone {
        self.slots.iter().filter_map(|slot| match slot {
            Slot::Live(entry) => Some((&entry.key, &entry.value)),
            Slot::Free(_) => None,
        })
    }

    /// Drop all keys/values, retaining the already-admitted backing capacity.
    pub fn clear(&mut self) {
        self.slots.clear();
        self.root = None;
        self.free = None;
        self.len = 0;
    }

    /// Exact original arena capacity, for a containing owner's destruction.
    pub fn storage_bytes(&self) -> Result<usize, StorageError> {
        core::alloc::Layout::array::<Slot<K, V>>(self.slots.capacity())
            .map(|layout| layout.size())
            .map_err(|_| StorageError::SizeOverflow)
    }

    /// Destroy this arena before releasing its original physical array layout.
    pub fn release<S: Admission + ?Sized>(
        self,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        memory.release_vec(self.slots)
    }

    /// Copy only slot metadata and shallow immutable native key/value owners.
    /// K/V clones used by the parser must themselves allocate no payload.
    pub fn try_clone<S: Admission + ?Sized>(
        &self,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, StorageError>
    where
        K: Clone,
        V: Clone,
    {
        let mut slots = Vec::new();
        memory.reserve(&mut slots, self.slots.len())?;
        slots.extend(self.slots.iter().cloned());
        Ok(Self {
            slots,
            root: self.root,
            free: self.free,
            len: self.len,
        })
    }

    fn height(&self, index: Option<usize>) -> usize {
        index.map_or(0, |index| self.entry(index).height)
    }

    fn update_height(&mut self, index: usize) {
        let entry = self.entry(index);
        let height = self.height(entry.left).max(self.height(entry.right)) + 1;
        self.entry_mut(index).height = height;
    }

    /// Replace one parent link (or the root), then install the same parent
    /// on the replacement. Every structural operation goes through this home.
    fn link(&mut self, parent: Option<usize>, old: usize, next: Option<usize>) {
        if let Some(parent) = parent {
            let entry = self.entry_mut(parent);
            if entry.left == Some(old) {
                entry.left = next;
            } else {
                assert_eq!(entry.right, Some(old), "scope tree parent link drifted");
                entry.right = next;
            }
        } else {
            assert_eq!(self.root, Some(old), "scope tree root link drifted");
            self.root = next;
        }
        if let Some(next) = next {
            self.entry_mut(next).parent = parent;
        }
    }

    fn rotate_left(&mut self, index: usize) -> usize {
        let pivot = self
            .entry(index)
            .right
            .expect("a left rotation has a right child");
        let middle = self.entry(pivot).left;
        let parent = self.entry(index).parent;
        self.link(parent, index, Some(pivot));
        self.entry_mut(index).right = middle;
        if let Some(middle) = middle {
            self.entry_mut(middle).parent = Some(index);
        }
        self.entry_mut(pivot).left = Some(index);
        self.entry_mut(index).parent = Some(pivot);
        self.update_height(index);
        self.update_height(pivot);
        pivot
    }

    fn rotate_right(&mut self, index: usize) -> usize {
        let pivot = self
            .entry(index)
            .left
            .expect("a right rotation has a left child");
        let middle = self.entry(pivot).right;
        let parent = self.entry(index).parent;
        self.link(parent, index, Some(pivot));
        self.entry_mut(index).left = middle;
        if let Some(middle) = middle {
            self.entry_mut(middle).parent = Some(index);
        }
        self.entry_mut(pivot).right = Some(index);
        self.entry_mut(index).parent = Some(pivot);
        self.update_height(index);
        self.update_height(pivot);
        pivot
    }

    fn rebalance(&mut self, mut at: Option<usize>) {
        while let Some(index) = at {
            self.update_height(index);
            let entry = self.entry(index);
            let left = self.height(entry.left);
            let right = self.height(entry.right);
            let root = if left > right && left - right > 1 {
                let child = entry.left.expect("the taller side is nonempty");
                let child_entry = self.entry(child);
                if self.height(child_entry.right) > self.height(child_entry.left) {
                    self.rotate_left(child);
                }
                self.rotate_right(index)
            } else if right > left && right - left > 1 {
                let child = entry.right.expect("the taller side is nonempty");
                let child_entry = self.entry(child);
                if self.height(child_entry.left) > self.height(child_entry.right) {
                    self.rotate_right(child);
                }
                self.rotate_left(index)
            } else {
                index
            };
            at = self.entry(root).parent;
        }
    }

    fn swap_payloads(&mut self, first: usize, second: usize) {
        let (first, second) = if first < second {
            let (before, after) = self.slots.split_at_mut(second);
            (&mut before[first], &mut after[0])
        } else {
            let (before, after) = self.slots.split_at_mut(first);
            (&mut after[0], &mut before[second])
        };
        let (Slot::Live(first), Slot::Live(second)) = (first, second) else {
            unreachable!("scope tree payload swap names a vacant slot");
        };
        mem::swap(&mut first.key, &mut second.key);
        mem::swap(&mut first.value, &mut second.value);
    }
}

impl<K: Ord, V> OrderedTable<K, V> {
    /// Borrow the payload associated with an exactly ordered key.
    pub fn get(&self, key: &K) -> Option<&V> {
        self.get_by(|candidate| key.cmp(candidate))
    }

    /// Mutate an existing payload without allocating or changing its key.
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        let index = self.find(|candidate| key.cmp(candidate))?;
        Some(&mut self.entry_mut(index).value)
    }

    /// Reserve the actual next array layout without modifying any key/value.
    /// Consumers may reserve their parallel output destination before insertion.
    pub fn reserve_insert<S: Admission + ?Sized>(
        &mut self,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        if self.free.is_none() {
            let required = self
                .slots
                .len()
                .checked_add(1)
                .ok_or(StorageError::SizeOverflow)?;
            if required > self.slots.capacity() {
                let capacity = self
                    .slots
                    .capacity()
                    .checked_mul(2)
                    .ok_or(StorageError::SizeOverflow)?
                    .max(required);
                memory.reserve(&mut self.slots, capacity)?;
            }
        }
        Ok(())
    }

    /// Replace the value in place, or admit storage before adding a new key.
    /// Refusal leaves every existing association unchanged.
    pub fn insert<S: Admission + ?Sized>(
        &mut self,
        key: K,
        value: V,
        memory: &mut Memory<'_, S>,
    ) -> Result<Option<V>, StorageError> {
        let mut at = self.root;
        let mut parent = None;
        let mut left = false;
        while let Some(index) = at {
            let entry = self.entry(index);
            parent = Some(index);
            at = match key.cmp(&entry.key) {
                Ordering::Less => {
                    left = true;
                    entry.left
                }
                Ordering::Greater => {
                    left = false;
                    entry.right
                }
                Ordering::Equal => {
                    return Ok(Some(mem::replace(&mut self.entry_mut(index).value, value)));
                }
            };
        }
        let len = self.len.checked_add(1).ok_or(StorageError::SizeOverflow)?;
        self.reserve_insert(memory)?;
        let entry = Entry {
            key,
            value,
            parent,
            left: None,
            right: None,
            height: 1,
        };
        let index = if let Some(index) = self.free {
            let Slot::Free(next) = &self.slots[index] else {
                unreachable!("scope free list points at a live slot");
            };
            self.free = *next;
            self.slots[index] = Slot::Live(entry);
            index
        } else {
            let index = self.slots.len();
            self.slots.push(Slot::Live(entry));
            index
        };
        if let Some(parent) = parent {
            if left {
                self.entry_mut(parent).left = Some(index);
            } else {
                self.entry_mut(parent).right = Some(index);
            }
        } else {
            self.root = Some(index);
        }
        self.len = len;
        self.rebalance(parent);
        Ok(None)
    }

    /// Remove an association without allocation; reuse its original slot later.
    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.remove_entry_by(|candidate| key.cmp(candidate))
            .map(|(_, value)| value)
    }

    /// Remove the original key and payload through a borrowed comparison.
    pub fn remove_entry_by(&mut self, compare: impl FnMut(&K) -> Ordering) -> Option<(K, V)> {
        let index = self.find(compare)?;
        Some(self.remove_index(index))
    }

    fn remove_index(&mut self, mut index: usize) -> (K, V) {
        let entry = self.entry(index);
        if let (Some(_), Some(mut successor)) = (entry.left, entry.right) {
            while let Some(left) = self.entry(successor).left {
                successor = left;
            }
            self.swap_payloads(index, successor);
            index = successor;
        }
        let entry = self.entry(index);
        let parent = entry.parent;
        let child = entry.left.or(entry.right);
        self.link(parent, index, child);
        let Slot::Live(entry) = mem::replace(&mut self.slots[index], Slot::Free(self.free)) else {
            unreachable!("scope removal names a vacant slot");
        };
        self.free = Some(index);
        self.len -= 1;
        self.rebalance(parent);
        (entry.key, entry.value)
    }

    /// Move the first stored key and payload, retaining reusable arena storage.
    pub fn pop_first(&mut self) -> Option<(K, V)> {
        self.root
            .map(|root| self.extreme(root, true))
            .map(|index| self.remove_index(index))
    }

    /// Move the last stored key and payload, retaining reusable arena storage.
    pub fn pop_last(&mut self) -> Option<(K, V)> {
        self.root
            .map(|root| self.extreme(root, false))
            .map(|index| self.remove_index(index))
    }

    fn seek<Q: ?Sized + Ord>(&self, bound: &Q, forward: bool, inclusive: bool) -> Option<usize>
    where
        K: std::borrow::Borrow<Q>,
    {
        let mut next = self.root;
        let mut found = None;
        while let Some(index) = next {
            let entry = self.entry(index);
            let order = entry.key.borrow().cmp(bound);
            let qualifies = (inclusive && order.is_eq())
                || if forward {
                    order.is_gt()
                } else {
                    order.is_lt()
                };
            if qualifies {
                found = Some(index);
                next = if forward { entry.left } else { entry.right };
            } else {
                next = if forward { entry.right } else { entry.left };
            }
        }
        found
    }

    /// Seek both bounds in logarithmic time, then visit only matching entries.
    pub fn range<Q: ?Sized + Ord, R: std::ops::RangeBounds<Q>>(
        &self,
        bounds: R,
    ) -> OrderedRange<'_, K, V>
    where
        K: std::borrow::Borrow<Q>,
    {
        use std::ops::Bound;
        let front = match bounds.start_bound() {
            Bound::Unbounded => self.root.map(|root| self.extreme(root, true)),
            Bound::Included(key) => self.seek(key, true, true),
            Bound::Excluded(key) => self.seek(key, true, false),
        };
        let back = match bounds.end_bound() {
            Bound::Unbounded => self.root.map(|root| self.extreme(root, false)),
            Bound::Included(key) => self.seek(key, false, true),
            Bound::Excluded(key) => self.seek(key, false, false),
        };
        let live = front
            .zip(back)
            .is_some_and(|(front, back)| self.entry(front).key <= self.entry(back).key);
        OrderedRange {
            table: self,
            front: front.filter(|_| live),
            back: back.filter(|_| live),
        }
    }
}

/// Allocation-free bidirectional traversal in canonical key order.
pub struct OrderedIter<'a, K, V> {
    table: &'a OrderedTable<K, V>,
    front: Option<usize>,
    back: Option<usize>,
    remaining: usize,
}
impl<'a, K, V> Iterator for OrderedIter<'a, K, V> {
    type Item = (&'a K, &'a V);
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        let index = self.front.expect("nonempty ordered walk has a front");
        self.front = self.table.neighbor(index, true);
        self.remaining -= 1;
        let entry = self.table.entry(index);
        Some((&entry.key, &entry.value))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}
impl<K, V> DoubleEndedIterator for OrderedIter<'_, K, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        let index = self.back.expect("nonempty ordered walk has a back");
        self.back = self.table.neighbor(index, false);
        self.remaining -= 1;
        let entry = self.table.entry(index);
        Some((&entry.key, &entry.value))
    }
}
impl<K, V> ExactSizeIterator for OrderedIter<'_, K, V> {}
impl<K, V> std::iter::FusedIterator for OrderedIter<'_, K, V> {}

/// Allocation-free traversal between borrowed ordered bounds.
pub struct OrderedRange<'a, K, V> {
    table: &'a OrderedTable<K, V>,
    front: Option<usize>,
    back: Option<usize>,
}
impl<'a, K, V> Iterator for OrderedRange<'a, K, V> {
    type Item = (&'a K, &'a V);
    fn next(&mut self) -> Option<Self::Item> {
        let index = self.front?;
        if self.back == Some(index) {
            self.front = None;
            self.back = None;
        } else {
            self.front = self.table.neighbor(index, true);
        }
        let entry = self.table.entry(index);
        Some((&entry.key, &entry.value))
    }
}
impl<K, V> DoubleEndedIterator for OrderedRange<'_, K, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let index = self.back?;
        if self.front == Some(index) {
            self.front = None;
            self.back = None;
        } else {
            self.back = self.table.neighbor(index, false);
        }
        let entry = self.table.entry(index);
        Some((&entry.key, &entry.value))
    }
}
impl<K, V> std::iter::FusedIterator for OrderedRange<'_, K, V> {}

impl<K: core::fmt::Debug, V: core::fmt::Debug> core::fmt::Debug for OrderedTable<K, V> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_map().entries(self.ordered()).finish()
    }
}
impl<K, V> core::fmt::Debug for OrderedIter<'_, K, V> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("OrderedIter")
            .field("remaining", &self.remaining)
            .finish_non_exhaustive()
    }
}
impl<K, V> core::fmt::Debug for OrderedRange<'_, K, V> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("OrderedRange").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::OrderedTable;
    use crate::allocation::{Admission, Memory, Resident, StorageError};
    use core::{cell::Cell, cmp::Ordering};
    use std::collections::BTreeMap;

    #[test]
    fn ordered_ranges_and_crossing_walks_match_independent_ordered_oracle() {
        use std::ops::Bound::{Excluded, Included, Unbounded};
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let mut table = OrderedTable::default();
        let mut oracle = BTreeMap::new();
        for step in 0..1200_u64 {
            let key = (step.wrapping_mul(73) ^ (step / 7)) % 97;
            if step % 3 == 0 {
                assert_eq!(table.remove(&key), oracle.remove(&key));
            } else {
                assert_eq!(
                    table.insert(key, step, &mut memory).unwrap(),
                    oracle.insert(key, step)
                );
            }
            let original_live = memory.admitted_bytes();
            let mut actual = table.ordered();
            let mut expected = oracle.iter();
            for position in 0..oracle.len() {
                let (actual, expected) = if position % 2 == 0 {
                    (actual.next(), expected.next())
                } else {
                    (actual.next_back(), expected.next_back())
                };
                assert_eq!(actual, expected);
            }
            assert_eq!(actual.next(), None);
            assert_eq!(actual.next_back(), None);
            assert_eq!(memory.admitted_bytes(), original_live);
        }
        for low in 0..100 {
            for high in low..100 {
                for bounds in [
                    (Included(low), Included(high)),
                    (Included(low), Excluded(high)),
                    (Excluded(low), Included(high)),
                    (Unbounded, Included(high)),
                    (Included(low), Unbounded),
                ] {
                    let mut actual = table.range(bounds);
                    let mut expected = oracle.range(bounds);
                    let mut front = true;
                    loop {
                        let got = if front {
                            actual.next()
                        } else {
                            actual.next_back()
                        };
                        let want = if front {
                            expected.next()
                        } else {
                            expected.next_back()
                        };
                        assert_eq!(got, want, "range {bounds:?}");
                        if want.is_none() {
                            break;
                        }
                        front = !front;
                    }
                    assert_eq!(actual.next(), None);
                    assert_eq!(actual.next_back(), None);
                }
            }
        }
        while let Some(expected) = oracle.pop_first() {
            assert_eq!(table.pop_first(), Some(expected));
            if let Some(expected) = oracle.pop_last() {
                assert_eq!(table.pop_last(), Some(expected));
            }
        }
        assert!(table.is_empty());
        table.release(&mut memory).unwrap();
        assert_eq!(memory.admitted_bytes(), 0);
    }

    struct Limit {
        live: usize,
        maximum: usize,
        refusals: usize,
    }

    impl Admission for Limit {
        fn resize(&mut self, live: usize) -> Result<(), StorageError> {
            if live > self.maximum {
                self.refusals += 1;
                return Err(StorageError::AdmissionFailed);
            }
            self.live = live;
            Ok(())
        }
    }

    #[test]
    fn associations_survive_rotations_deletions_and_vacant_slot_reuse() {
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let mut native = OrderedTable::default();
        let mut oracle = BTreeMap::new();
        // Ascending, descending, and interior deletions exercise both directions,
        // two-child removal, root replacement, and recycled slots.
        for key in 0..512 {
            assert_eq!(
                native.insert(key, key * 7, &mut memory).unwrap(),
                oracle.insert(key, key * 7)
            );
        }
        for key in (0..512).step_by(3).rev() {
            assert_eq!(native.remove(&key), oracle.remove(&key));
        }
        for key in (0..768).rev() {
            assert_eq!(
                native.insert(key, key * 11, &mut memory).unwrap(),
                oracle.insert(key, key * 11)
            );
        }
        for key in 0..900 {
            assert_eq!(native.get(&key), oracle.get(&key));
        }
        for key in (0..768).step_by(2) {
            assert_eq!(native.remove(&key), oracle.remove(&key));
        }
        for key in (0..768).rev() {
            assert_eq!(native.remove(&key), oracle.remove(&key));
        }
        assert!(native.is_empty());
        assert_eq!(native.get(&0), None);
        native.release(&mut memory).unwrap();
        assert_eq!(memory.admitted_bytes(), 0);
    }

    #[test]
    fn refused_growth_changes_no_live_association_and_release_destroys_first() {
        let mut limit = Limit {
            live: 0,
            maximum: usize::MAX,
            refusals: 0,
        };
        let mut memory = Memory::new(&mut limit);
        let mut native = OrderedTable::default();
        native.insert(10, 20, &mut memory).unwrap();
        let admitted = memory.admitted_bytes();
        memory.admission_mut().maximum = admitted;
        assert_eq!(
            native.insert(11, 21, &mut memory),
            Err(StorageError::AdmissionFailed)
        );
        assert_eq!(native.get(&10), Some(&20));
        assert_eq!(native.get(&11), None);
        assert_eq!(memory.admitted_bytes(), admitted);
        assert_eq!(memory.admission_mut().refusals, 1);
        // Replacement, deletion, and reuse fit the original arena; a zero-growth
        // operation must stay healthy even after the table's separate refusal.
        assert_eq!(native.insert(10, 22, &mut memory).unwrap(), Some(20));
        assert_eq!(native.remove(&10), Some(22));
        assert_eq!(native.insert(11, 23, &mut memory).unwrap(), None);
        native.release(&mut memory).unwrap();
        assert_eq!(memory.admission_mut().live, 0);
    }

    struct Counted<'a> {
        key: usize,
        comparisons: &'a Cell<usize>,
    }
    impl PartialEq for Counted<'_> {
        fn eq(&self, other: &Self) -> bool {
            self.key == other.key
        }
    }
    impl Eq for Counted<'_> {}
    impl PartialOrd for Counted<'_> {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }
    impl Ord for Counted<'_> {
        fn cmp(&self, other: &Self) -> Ordering {
            self.comparisons.set(self.comparisons.get() + 1);
            self.key.cmp(&other.key)
        }
    }

    fn wide_comparisons(width: usize) -> usize {
        let comparisons = Cell::new(0);
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let mut native = OrderedTable::default();
        // Sorted lexical identities are the degenerate input for an unbalanced
        // tree, independently of the parser's whole-pattern consultation count.
        for key in 0..width {
            native
                .insert(
                    Counted {
                        key,
                        comparisons: &comparisons,
                    },
                    key,
                    &mut memory,
                )
                .unwrap();
        }
        for key in 0..width {
            assert_eq!(
                native.get(&Counted {
                    key,
                    comparisons: &comparisons
                }),
                Some(&key)
            );
        }
        native.release(&mut memory).unwrap();
        comparisons.get()
    }

    #[test]
    fn wide_sorted_lookup_does_not_degrade_to_quadratic_work() {
        let narrow = wide_comparisons(1024);
        let wide = wide_comparisons(8192);
        // Eight times the width: quadratic lookup would require ~64x work.
        // Logarithmic balanced lookup has room for its increased path height.
        assert!(wide < narrow * 20, "comparison work: {narrow} -> {wide}");
    }

    #[test]
    fn borrowed_lookup_and_clear_need_no_new_storage() {
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let mut native = OrderedTable::default();
        native.insert(String::from("ex"), 7, &mut memory).unwrap();
        let live = memory.admitted_bytes();
        assert_eq!(native.get_by(|stored| "ex".cmp(stored.as_str())), Some(&7));
        assert_eq!(native.get_by(|stored| "absent".cmp(stored.as_str())), None);
        assert_eq!(memory.admitted_bytes(), live);
        native.clear();
        assert!(native.is_empty());
        assert_eq!(
            memory.admitted_bytes(),
            live,
            "clear retains covered arena capacity"
        );
        native.release(&mut memory).unwrap();
        assert_eq!(memory.admitted_bytes(), 0);
    }
}
