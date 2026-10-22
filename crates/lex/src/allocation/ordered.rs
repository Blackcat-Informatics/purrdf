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

/// A physical locator in an ordered arena, valid while its association is live.
/// It is not a persistent identity: removal permits its slot to be reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OrderedSlot(usize);

#[derive(Debug, Clone, Copy)]
enum Placement {
    Occupied(usize),
    Vacant {
        index: usize,
        parent: Option<usize>,
        left: bool,
        len: usize,
    },
}

/// An exclusively borrowed, admitted placement awaiting its original payload.
/// It prevents another edit from invalidating its path and clones no grant.
#[derive(Debug)]
pub struct OrderedEntry<'a, K, V> {
    table: &'a mut OrderedTable<K, V>,
    key: K,
    placement: Placement,
}

impl<K, V> OrderedEntry<'_, K, V> {
    /// Physical locator which becomes valid when this entry is published.
    pub fn slot(&self) -> OrderedSlot {
        OrderedSlot(match self.placement {
            Placement::Occupied(index) | Placement::Vacant { index, .. } => index,
        })
    }

    /// Whether publication replaces an already live value.
    pub fn is_occupied(&self) -> bool {
        matches!(self.placement, Placement::Occupied(_))
    }

    /// Publish without comparison or allocation through the admitted placement.
    pub fn publish(self, value: V) -> (OrderedSlot, Option<V>) {
        let Self {
            table,
            key,
            placement,
        } = self;
        match placement {
            Placement::Occupied(index) => (
                OrderedSlot(index),
                Some(mem::replace(&mut table.entry_mut(index).value, value)),
            ),
            Placement::Vacant {
                index,
                parent,
                left,
                len,
            } => {
                let entry = Entry {
                    key,
                    value,
                    parent,
                    left: None,
                    right: None,
                    height: 1,
                };
                if let Some(free) = table.free {
                    assert_eq!(
                        free, index,
                        "the exclusive insertion retains its original free slot"
                    );
                    let Slot::Free(next) = &table.slots[index] else {
                        unreachable!("scope free list points at a live slot");
                    };
                    table.free = *next;
                    table.slots[index] = Slot::Live(entry);
                } else {
                    assert_eq!(index, table.slots.len());
                    table.slots.push(Slot::Live(entry));
                }
                if let Some(parent) = parent {
                    if left {
                        table.entry_mut(parent).left = Some(index);
                    } else {
                        table.entry_mut(parent).right = Some(index);
                    }
                } else {
                    table.root = Some(index);
                }
                table.len = len;
                table.rebalance(parent);
                (OrderedSlot(index), None)
            }
        }
    }
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
}

impl<K, V> OrderedTable<K, V> {
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

    /// Compare and admit an association before constructing its payload.
    /// All comparisons must agree with the tree's original total key order.
    /// Dropping the prepared entry leaves existing associations unchanged;
    /// successfully reserved arena capacity remains under the original memory.
    ///
    /// # Errors
    /// Preserves the original comparison or physical-storage refusal.
    pub fn prepare_insert<S: Admission + ?Sized, E: From<StorageError>>(
        &mut self,
        key: K,
        memory: &mut Memory<'_, S>,
        mut compare: impl FnMut(&K, &K) -> Result<Ordering, E>,
    ) -> Result<OrderedEntry<'_, K, V>, E> {
        let mut at = self.root;
        let mut parent = None;
        let mut left = false;
        while let Some(index) = at {
            let entry = self.entry(index);
            parent = Some(index);
            at = match compare(&key, &entry.key)? {
                Ordering::Less => {
                    left = true;
                    entry.left
                }
                Ordering::Greater => {
                    left = false;
                    entry.right
                }
                Ordering::Equal => {
                    return Ok(OrderedEntry {
                        table: self,
                        key,
                        placement: Placement::Occupied(index),
                    });
                }
            };
        }
        let len = self
            .len
            .checked_add(1)
            .ok_or(StorageError::SizeOverflow)
            .map_err(E::from)?;
        self.reserve_insert(memory).map_err(E::from)?;
        let index = self.free.unwrap_or(self.slots.len());
        Ok(OrderedEntry {
            table: self,
            key,
            placement: Placement::Vacant {
                index,
                parent,
                left,
                len,
            },
        })
    }

    /// Admit a placement, construct its payload, then publish without allocation.
    /// The callback receives its future locator and whether it replaces a value.
    /// Failed construction preserves existing associations and admitted capacity.
    ///
    /// # Errors
    /// Preserves the original comparison, payload or physical-storage refusal.
    pub fn try_insert_with<S: Admission + ?Sized, E: From<StorageError>>(
        &mut self,
        key: K,
        memory: &mut Memory<'_, S>,
        compare: impl FnMut(&K, &K) -> Result<Ordering, E>,
        make: impl FnOnce(&mut Memory<'_, S>, OrderedSlot, bool) -> Result<V, E>,
    ) -> Result<(OrderedSlot, Option<V>), E> {
        let entry = self.prepare_insert(key, memory, compare)?;
        let value = make(memory, entry.slot(), entry.is_occupied())?;
        Ok(entry.publish(value))
    }

    /// First live physical locator, without comparing or projecting keys.
    pub fn first_slot(&self) -> Option<OrderedSlot> {
        self.root.map(|root| OrderedSlot(self.extreme(root, true)))
    }

    /// Borrow a live association at its original physical locator.
    pub fn at_slot(&self, slot: OrderedSlot) -> Option<(&K, &V)> {
        match self.slots.get(slot.0)? {
            Slot::Live(entry) => Some((&entry.key, &entry.value)),
            Slot::Free(_) => None,
        }
    }

    /// Next association in the established key order, with no key comparison.
    /// The supplied locator must still name its original live association.
    pub fn next_slot(&self, slot: OrderedSlot) -> Option<OrderedSlot> {
        self.at_slot(slot)?;
        self.neighbor(slot.0, true).map(OrderedSlot)
    }

    /// Remove a live association while preserving every surviving locator.
    /// The removed locator is invalidated and may be reused by a later insertion.
    pub fn remove_slot(&mut self, slot: OrderedSlot) -> Option<(K, V)> {
        self.at_slot(slot)?;
        Some(self.remove_index(slot.0))
    }

    fn remove_index(&mut self, index: usize) -> (K, V) {
        let entry = self.entry(index);
        let parent = entry.parent;
        let rebalance = if let (Some(left), Some(right)) = (entry.left, entry.right) {
            let successor = self.extreme(right, true);
            let predecessor = self.entry(successor).parent;
            if predecessor != Some(index) {
                let child = self.entry(successor).right;
                self.link(predecessor, successor, child);
                self.entry_mut(successor).right = Some(right);
                self.entry_mut(right).parent = Some(successor);
            }
            self.link(parent, index, Some(successor));
            self.entry_mut(successor).left = Some(left);
            self.entry_mut(left).parent = Some(successor);
            if predecessor == Some(index) {
                Some(successor)
            } else {
                predecessor
            }
        } else {
            let child = entry.left.or(entry.right);
            self.link(parent, index, child);
            parent
        };
        let Slot::Live(entry) = mem::replace(&mut self.slots[index], Slot::Free(self.free)) else {
            unreachable!("scope removal names a vacant slot");
        };
        self.free = Some(index);
        self.len -= 1;
        self.rebalance(rebalance);
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

    /// Replace the value in place, or admit storage before adding a new key.
    /// Refusal leaves every existing association unchanged.
    pub fn insert<S: Admission + ?Sized>(
        &mut self,
        key: K,
        value: V,
        memory: &mut Memory<'_, S>,
    ) -> Result<Option<V>, StorageError> {
        self.try_insert_with(
            key,
            memory,
            |a, b| Ok::<_, StorageError>(a.cmp(b)),
            |_, _, _| Ok(value),
        )
        .map(|(_, previous)| previous)
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

    #[test]
    fn external_order_preserves_every_surviving_physical_locator() {
        struct Key(i32);
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let mut table = OrderedTable::default();
        let mut slots = vec![None; 97];
        let mut oracle = BTreeMap::new();
        for step in 0..97 {
            let key = (step * 37) % 97;
            let (slot, old) = table
                .try_insert_with(
                    Key(key),
                    &mut memory,
                    |a, b| Ok::<_, StorageError>(b.0.cmp(&a.0)),
                    |_, _, replacing| {
                        assert!(!replacing);
                        Ok(key * 11)
                    },
                )
                .unwrap();
            assert_eq!(old, None);
            slots[usize::try_from(key).unwrap()] = Some(slot);
            oracle.insert(core::cmp::Reverse(key), key * 11);
        }
        for step in 0..97 {
            let key = (step * 53) % 97;
            let slot = slots[usize::try_from(key).unwrap()].take().unwrap();
            let (removed, value) = table.remove_slot(slot).unwrap();
            assert_eq!((removed.0, value), (key, key * 11));
            assert!(table.at_slot(slot).is_none());
            oracle.remove(&core::cmp::Reverse(key));
            for (key, &slot) in slots.iter().enumerate() {
                if let Some(slot) = slot {
                    let (stored, value) = table.at_slot(slot).unwrap();
                    assert_eq!(usize::try_from(stored.0).unwrap(), key);
                    assert_eq!(*value, stored.0 * 11);
                }
            }
            let mut walked = Vec::new();
            let mut at = table.first_slot();
            while let Some(slot) = at {
                let (key, value) = table.at_slot(slot).unwrap();
                walked.push((core::cmp::Reverse(key.0), *value));
                at = table.next_slot(slot);
            }
            assert_eq!(
                walked,
                oracle.iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>()
            );
        }
        assert!(table.is_empty());
        assert_eq!(table.storage_bytes().unwrap(), memory.admitted_bytes());
        table.release(&mut memory).unwrap();
        assert_eq!(memory.admitted_bytes(), 0);
    }

    struct Limit {
        live: usize,
        maximum: usize,
        refusals: usize,
    }

    #[test]
    fn abandoned_preparation_keeps_capacity_and_publishes_nothing() {
        let mut limit = Limit {
            live: 0,
            maximum: usize::MAX,
            refusals: 0,
        };
        let mut memory = Memory::new(&mut limit);
        let mut table = OrderedTable::default();
        let original = table
            .prepare_insert(1, &mut memory, |a, b| Ok::<_, StorageError>(a.cmp(b)))
            .unwrap()
            .publish(10)
            .0;
        {
            let prepared = table
                .prepare_insert(2, &mut memory, |a, b| Ok::<_, StorageError>(a.cmp(b)))
                .unwrap();
            assert!(!prepared.is_occupied());
        }
        assert_eq!(table.len(), 1);
        assert_eq!(table.at_slot(original), Some((&1, &10)));
        assert_eq!(table.storage_bytes().unwrap(), memory.admitted_bytes());
        let admitted = memory.admitted_bytes();
        let prepared = table
            .prepare_insert(2, &mut memory, |a, b| Ok::<_, StorageError>(a.cmp(b)))
            .unwrap();
        let future = prepared.slot();
        // The external transaction commits only after all fallible preparation.
        memory.admission_mut().maximum = 0;
        let (published, previous) = prepared.publish(20);
        assert_eq!(published, future);
        assert_eq!(previous, None);
        assert_eq!(table.at_slot(published), Some((&2, &20)));
        assert_eq!(memory.admitted_bytes(), admitted);
        table.release(&mut memory).unwrap();
        assert_eq!(memory.admission_mut().live, 0);
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
    fn external_comparison_storage_and_payload_refusals_publish_no_association() {
        #[derive(Debug, PartialEq, Eq)]
        enum Refusal {
            Storage(StorageError),
            Read(u16),
            Payload,
        }
        crate::variant_from!(Refusal { Storage(StorageError) });
        let mut limit = Limit {
            live: 0,
            maximum: usize::MAX,
            refusals: 0,
        };
        let mut memory = Memory::new(&mut limit);
        let mut table = OrderedTable::default();
        let (original, _) = table
            .try_insert_with(
                1,
                &mut memory,
                |a, b| Ok::<_, Refusal>(a.cmp(b)),
                |_, _, _| Ok(10),
            )
            .unwrap();
        let calls = Cell::new(0);
        let original_bytes = memory.admitted_bytes();
        assert_eq!(
            table.try_insert_with(
                2,
                &mut memory,
                |_, _| Err(Refusal::Read(7)),
                |_, _, _| {
                    calls.set(calls.get() + 1);
                    Ok(20)
                }
            ),
            Err(Refusal::Read(7))
        );
        assert_eq!(calls.get(), 0);
        assert_eq!(memory.admitted_bytes(), original_bytes);
        memory.admission_mut().maximum = original_bytes;
        assert_eq!(
            table.try_insert_with(
                2,
                &mut memory,
                |a, b| Ok::<_, Refusal>(a.cmp(b)),
                |_, _, _| {
                    calls.set(calls.get() + 1);
                    Ok(20)
                }
            ),
            Err(Refusal::Storage(StorageError::AdmissionFailed))
        );
        assert_eq!(
            calls.get(),
            0,
            "physical admission precedes payload construction"
        );
        memory.admission_mut().maximum = usize::MAX;
        assert_eq!(
            table.try_insert_with(
                2,
                &mut memory,
                |a, b| Ok::<_, Refusal>(a.cmp(b)),
                |_, _, _| {
                    calls.set(calls.get() + 1);
                    Err(Refusal::Payload)
                }
            ),
            Err(Refusal::Payload)
        );
        assert_eq!(table.len(), 1);
        assert_eq!(table.at_slot(original), Some((&1, &10)));
        assert_eq!(table.storage_bytes().unwrap(), memory.admitted_bytes());
        let (second, _) = table
            .try_insert_with(
                2,
                &mut memory,
                |a, b| Ok::<_, Refusal>(a.cmp(b)),
                |_, _, _| Ok(20),
            )
            .unwrap();
        assert_ne!(original, second);
        assert_eq!(
            table.try_insert_with(
                1,
                &mut memory,
                |a, b| Ok::<_, Refusal>(a.cmp(b)),
                |_, slot, replacing| {
                    assert_eq!(slot, original);
                    assert!(replacing);
                    Err(Refusal::Payload)
                }
            ),
            Err(Refusal::Payload)
        );
        assert_eq!(table.at_slot(original), Some((&1, &10)));
        table.release(&mut memory).unwrap();
        assert_eq!(memory.admitted_bytes(), 0);
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
