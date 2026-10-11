// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Retained execution authority around the one shared ordered arena.

use super::{LexicalFrame, WorkspaceCapability};
use crate::EvalError;
use purrdf_lex::allocation::{Memory, OrderedIter, OrderedRange, OrderedTable};
use std::borrow::Borrow;

/// Ordered membership over the same admitted arena as ordered associations.
#[derive(Debug)]
pub struct AdmittedOrderedSet<K> {
    entries: AdmittedOrderedMap<K, ()>,
}

impl<K> AdmittedOrderedSet<K> {
    /// Borrow the original execution authority retained by the membership arena.
    pub fn workspace(&self) -> &WorkspaceCapability {
        self.entries.workspace()
    }

    /// Empty membership retaining the original execution authority.
    #[must_use]
    pub fn new(workspace: &WorkspaceCapability) -> Self {
        Self {
            entries: AdmittedOrderedMap::new(workspace),
        }
    }
    /// Number of members.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    /// Whether membership is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    /// Borrow each member in canonical order without traversal allocation.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &K> + ExactSizeIterator {
        self.entries.keys()
    }
}
impl<K: Ord> AdmittedOrderedSet<K> {
    /// Whether two canonically ordered memberships share a key, without scratch.
    #[must_use]
    pub fn intersects(&self, other: &Self) -> bool {
        let mut left = self.iter().peekable();
        let mut right = other.iter().peekable();
        while let (Some(a), Some(b)) = (left.peek(), right.peek()) {
            match a.cmp(b) {
                std::cmp::Ordering::Less => {
                    left.next();
                }
                std::cmp::Ordering::Greater => {
                    right.next();
                }
                std::cmp::Ordering::Equal => return true,
            }
        }
        false
    }
    /// Build membership through the original admitted arena.
    /// # Errors
    /// Returns the original layout, admission or allocator refusal.
    pub fn from_iter_admitted(
        values: impl IntoIterator<Item = K>,
        workspace: &WorkspaceCapability,
    ) -> Result<Self, EvalError> {
        let mut set = Self::new(workspace);
        for value in values {
            set.insert(value)?;
        }
        Ok(set)
    }
    /// Test borrowed membership without manufacturing a key.
    pub fn contains<Q: ?Sized + Ord>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
    {
        self.entries.contains_key(key)
    }
    /// Admit arena replacement before installing membership.
    /// # Errors
    /// Returns the original layout, admission or allocator refusal.
    pub fn insert(&mut self, key: K) -> Result<bool, EvalError> {
        self.entries.insert(key, ()).map(|old| old.is_none())
    }
}

impl<K: PartialEq> PartialEq for AdmittedOrderedSet<K> {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len() && self.iter().eq(other.iter())
    }
}
impl<K: Eq> Eq for AdmittedOrderedSet<K> {}
impl<K: std::hash::Hash> std::hash::Hash for AdmittedOrderedSet<K> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&self.len(), state);
        for key in self.iter() {
            key.hash(state);
        }
    }
}
impl<'a, K> IntoIterator for &'a AdmittedOrderedSet<K> {
    type Item = &'a K;
    type IntoIter = std::iter::Map<OrderedIter<'a, K, ()>, fn((&'a K, &'a ())) -> &'a K>;
    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter().map(|(key, ())| key)
    }
}

/// Ordered storage retaining its original native admission through destruction.
/// The table dies before its frame; keys and values keep their intrinsic owners.
pub struct AdmittedOrderedMap<K, V> {
    table: OrderedTable<K, V>,
    frame: LexicalFrame,
}

impl<K, V> AdmittedOrderedMap<K, V> {
    /// Exclusive live payloads in arena order, retaining the original backing grant.
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.table.values_mut()
    }
    /// Empty ordered storage sharing the original execution capability.
    #[must_use]
    pub fn new(workspace: &WorkspaceCapability) -> Self {
        Self {
            table: OrderedTable::default(),
            frame: LexicalFrame::new(workspace),
        }
    }

    /// Borrow the original execution authority for retained payload factories.
    pub fn workspace(&self) -> &WorkspaceCapability {
        self.frame.workspace()
    }

    /// Number of live entries, excluding reusable vacant slots.
    #[must_use]
    pub fn len(&self) -> usize {
        self.table.len()
    }

    /// Whether this map holds no live association.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.table.is_empty()
    }

    /// Walk key order without cloning keys or allocating traversal scratch.
    pub fn iter(&self) -> OrderedIter<'_, K, V> {
        self.table.ordered()
    }

    /// Borrow payloads in canonical key order.
    pub fn values(&self) -> impl DoubleEndedIterator<Item = &V> + ExactSizeIterator {
        self.iter().map(|(_, value)| value)
    }

    /// Borrow keys in canonical order.
    pub fn keys(&self) -> impl DoubleEndedIterator<Item = &K> + ExactSizeIterator {
        self.iter().map(|(key, _)| key)
    }

    /// Destroy keys and payloads while retaining the original arena capacity.
    pub fn clear(&mut self) {
        self.table.clear();
    }

    /// Borrow the first key and payload without acquiring storage.
    pub fn first_key_value(&self) -> Option<(&K, &V)> {
        self.table.first_key_value()
    }

    /// Borrow the last key and payload without acquiring storage.
    pub fn last_key_value(&self) -> Option<(&K, &V)> {
        self.table.last_key_value()
    }
}

impl<K: Ord, V> AdmittedOrderedMap<K, V> {
    /// Build associations through original admission, preserving producer failures.
    /// # Errors
    /// Returns the first producer, layout, admission or allocator refusal.
    pub fn try_from_iter_admitted(
        values: impl IntoIterator<Item = Result<(K, V), EvalError>>,
        workspace: &WorkspaceCapability,
    ) -> Result<Self, EvalError> {
        let mut map = Self::new(workspace);
        for value in values {
            let (key, value) = value?;
            map.insert(key, value)?;
        }
        Ok(map)
    }
    /// Borrow a payload through its original key ordering.
    pub fn get<Q: ?Sized + Ord>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
    {
        self.table.get_by(|candidate| key.cmp(candidate.borrow()))
    }

    /// Borrow the stored key and its payload without detaching either owner.
    pub fn get_key_value<Q: ?Sized + Ord>(&self, key: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q>,
    {
        self.table
            .get_key_value_by(|candidate| key.cmp(candidate.borrow()))
    }

    /// Mutate an existing payload without growing the map.
    pub fn get_mut<Q: ?Sized + Ord>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
    {
        self.table
            .get_mut_by(|candidate| key.cmp(candidate.borrow()))
    }

    /// Test membership without manufacturing an owned key.
    pub fn contains_key<Q: ?Sized + Ord>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
    {
        self.get(key).is_some()
    }

    /// Admit actual arena replacement before inserting a new association.
    /// Failure retains any still-live replacement and its original grant.
    ///
    /// # Errors
    /// Returns the original checked-layout, admission or allocator refusal.
    pub fn insert(&mut self, key: K, value: V) -> Result<Option<V>, EvalError> {
        let live = self.frame.admitted_bytes();
        let result = self
            .table
            .insert(key, value, &mut Memory::resume(&mut self.frame, live));
        result.map_err(|error| self.frame.storage_error(error, "ordered execution arena"))
    }

    /// Remove a payload without releasing reusable arena capacity.
    pub fn remove<Q: ?Sized + Ord>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
    {
        self.remove_entry(key).map(|(_, value)| value)
    }

    /// Move the original key and payload without cloning either owner.
    pub fn remove_entry<Q: ?Sized + Ord>(&mut self, key: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
    {
        self.table
            .remove_entry_by(|candidate| key.cmp(candidate.borrow()))
    }

    /// Move the first key and payload, retaining reusable admitted metadata.
    pub fn pop_first(&mut self) -> Option<(K, V)> {
        self.table.pop_first()
    }

    /// Move the last key and payload, retaining reusable admitted metadata.
    pub fn pop_last(&mut self) -> Option<(K, V)> {
        self.table.pop_last()
    }

    /// Seek both borrowed bounds, then visit only the matching key interval.
    pub fn range<Q: ?Sized + Ord, R: std::ops::RangeBounds<Q>>(
        &self,
        bounds: R,
    ) -> OrderedRange<'_, K, V>
    where
        K: Borrow<Q>,
    {
        self.table.range(bounds)
    }
}

/// Moving traversal retains the exact arena admission until its final drop.
pub struct OrderedIntoIter<K, V> {
    map: AdmittedOrderedMap<K, V>,
}

impl<K: Ord, V> Iterator for OrderedIntoIter<K, V> {
    type Item = (K, V);
    fn next(&mut self) -> Option<Self::Item> {
        self.map.pop_first()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.map.len(), Some(self.map.len()))
    }
}
impl<K: Ord, V> DoubleEndedIterator for OrderedIntoIter<K, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.map.pop_last()
    }
}
impl<K: Ord, V> ExactSizeIterator for OrderedIntoIter<K, V> {}
impl<K: Ord, V> std::iter::FusedIterator for OrderedIntoIter<K, V> {}
impl<K: Ord, V> IntoIterator for AdmittedOrderedMap<K, V> {
    type Item = (K, V);
    type IntoIter = OrderedIntoIter<K, V>;
    fn into_iter(self) -> Self::IntoIter {
        OrderedIntoIter { map: self }
    }
}
impl<'a, K, V> IntoIterator for &'a AdmittedOrderedMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = OrderedIter<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl<K: std::fmt::Debug, V: std::fmt::Debug> std::fmt::Debug for AdmittedOrderedMap<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.table.fmt(f)
    }
}
impl<K: PartialEq, V: PartialEq> PartialEq for AdmittedOrderedMap<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len() && self.iter().eq(other.iter())
    }
}
impl<K: Eq, V: Eq> Eq for AdmittedOrderedMap<K, V> {}
impl<K: Ord + Borrow<Q>, V, Q: ?Sized + Ord> std::ops::Index<&Q> for AdmittedOrderedMap<K, V> {
    type Output = V;
    fn index(&self, key: &Q) -> &V {
        self.get(key).expect("ordered association key is present")
    }
}
impl<K: std::fmt::Debug, V: std::fmt::Debug> std::fmt::Debug for OrderedIntoIter<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.map.fmt(f)
    }
}
