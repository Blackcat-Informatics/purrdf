// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Ordered native storage over the original admitted array home.

use super::{AdmittedVec, WorkspaceCapability};
use crate::EvalError;
use std::borrow::Borrow;

#[derive(Debug)]
struct Node<K, V> {
    key: K,
    value: V,
    parent: Option<usize>,
    left: Option<usize>,
    right: Option<usize>,
    height: usize,
}

#[derive(Debug)]
struct Slot<K, V> {
    node: Option<Node<K, V>>,
    next_free: Option<usize>,
}

/// An ordered map whose concrete node array retains its native admission.
///
/// AVL rotations move indexes, never payloads or their original owners. Vacant
/// slots are reused; growth goes through `AdmittedVec` before a new node is born.
/// Borrowed ordered walks follow parent links and allocate no iterator stack.
pub struct AdmittedOrderedMap<K, V> {
    slots: AdmittedVec<Slot<K, V>>,
    root: Option<usize>,
    free: Option<usize>,
    len: usize,
}

impl<K, V> AdmittedOrderedMap<K, V> {
    /// Empty ordered storage sharing the original execution capability.
    #[must_use]
    pub fn new(workspace: &WorkspaceCapability) -> Self {
        Self { slots: AdmittedVec::new(workspace), root: None, free: None, len: 0 }
    }

    /// Number of live entries, independently of reusable admitted slots.
    #[must_use]
    pub const fn len(&self) -> usize { self.len }

    /// Whether this map holds no live entry.
    #[must_use]
    pub const fn is_empty(&self) -> bool { self.len == 0 }

    fn node(&self, index: usize) -> &Node<K, V> {
        self.slots[index].node.as_ref().expect("linked ordered node is live")
    }

    fn node_mut(&mut self, index: usize) -> &mut Node<K, V> {
        self.slots.as_mut_slice()[index].node.as_mut().expect("linked ordered node is live")
    }

    fn height(&self, index: Option<usize>) -> usize {
        index.map_or(0, |index| self.node(index).height)
    }

    fn refresh_height(&mut self, index: usize) {
        let node = self.node(index);
        let height = 1 + self.height(node.left).max(self.height(node.right));
        self.node_mut(index).height = height;
    }

    fn replace_child(&mut self, parent: Option<usize>, old: usize, new: Option<usize>) {
        if let Some(parent) = parent {
            if self.node(parent).left == Some(old) {
                self.node_mut(parent).left = new;
            } else {
                assert_eq!(self.node(parent).right, Some(old), "ordered parent owns its child");
                self.node_mut(parent).right = new;
            }
        } else {
            assert_eq!(self.root, Some(old), "ordered root is current");
            self.root = new;
        }
        if let Some(new) = new { self.node_mut(new).parent = parent; }
    }

    fn rotate_left(&mut self, index: usize) -> usize {
        let right = self.node(index).right.expect("left rotation has a right child");
        let parent = self.node(index).parent;
        let middle = self.node(right).left;
        self.replace_child(parent, index, Some(right));
        self.node_mut(index).right = middle;
        if let Some(middle) = middle { self.node_mut(middle).parent = Some(index); }
        self.node_mut(right).left = Some(index);
        self.node_mut(index).parent = Some(right);
        self.refresh_height(index);
        self.refresh_height(right);
        right
    }

    fn rotate_right(&mut self, index: usize) -> usize {
        let left = self.node(index).left.expect("right rotation has a left child");
        let parent = self.node(index).parent;
        let middle = self.node(left).right;
        self.replace_child(parent, index, Some(left));
        self.node_mut(index).left = middle;
        if let Some(middle) = middle { self.node_mut(middle).parent = Some(index); }
        self.node_mut(left).right = Some(index);
        self.node_mut(index).parent = Some(left);
        self.refresh_height(index);
        self.refresh_height(left);
        left
    }

    fn rebalance(&mut self, mut next: Option<usize>) {
        while let Some(index) = next {
            self.refresh_height(index);
            let left = self.node(index).left;
            let right = self.node(index).right;
            let top = if self.height(left) > self.height(right) + 1 {
                let left = left.expect("left-heavy node has a child");
                if self.height(self.node(left).right) > self.height(self.node(left).left) {
                    self.rotate_left(left);
                }
                self.rotate_right(index)
            } else if self.height(right) > self.height(left) + 1 {
                let right = right.expect("right-heavy node has a child");
                if self.height(self.node(right).left) > self.height(self.node(right).right) {
                    self.rotate_right(right);
                }
                self.rotate_left(index)
            } else { index };
            next = self.node(top).parent;
        }
    }

    fn extreme(&self, mut index: usize, left: bool) -> usize {
        while let Some(child) = if left { self.node(index).left } else { self.node(index).right } {
            index = child;
        }
        index
    }

    fn neighbor(&self, mut index: usize, forward: bool) -> Option<usize> {
        let child = if forward { self.node(index).right } else { self.node(index).left };
        if let Some(child) = child { return Some(self.extreme(child, forward)); }
        while let Some(parent) = self.node(index).parent {
            let from = if forward { self.node(parent).left } else { self.node(parent).right };
            if from == Some(index) { return Some(parent); }
            index = parent;
        }
        None
    }

    /// Borrow entries in key order with no native working allocation.
    #[must_use]
    pub fn iter(&self) -> OrderedIter<'_, K, V> {
        OrderedIter {
            map: self,
            front: self.root.map(|root| self.extreme(root, true)),
            back: self.root.map(|root| self.extreme(root, false)),
            remaining: self.len,
        }
    }

    /// Borrow values in the same canonical key order.
    pub fn values(&self) -> impl DoubleEndedIterator<Item = &V> + ExactSizeIterator {
        self.iter().map(|(_, value)| value)
    }

    /// Borrow keys in canonical order.
    pub fn keys(&self) -> impl DoubleEndedIterator<Item = &K> + ExactSizeIterator {
        self.iter().map(|(key, _)| key)
    }

    /// Destroy every payload while retaining the original array capacity grant.
    pub fn clear(&mut self) {
        self.slots.clear(); self.root = None; self.free = None; self.len = 0;
    }
}

impl<K: Ord, V> AdmittedOrderedMap<K, V> {
    fn find<Q: ?Sized + Ord>(&self, key: &Q) -> Option<usize> where K: Borrow<Q> {
        let mut next = self.root;
        while let Some(index) = next {
            let node = self.node(index);
            match key.cmp(node.key.borrow()) {
                std::cmp::Ordering::Less => next = node.left,
                std::cmp::Ordering::Greater => next = node.right,
                std::cmp::Ordering::Equal => return Some(index),
            }
        }
        None
    }

    /// Borrow a value without cloning a key or acquiring fresh storage.
    #[must_use]
    pub fn get<Q: ?Sized + Ord>(&self, key: &Q) -> Option<&V> where K: Borrow<Q> {
        self.find(key).map(|index| &self.node(index).value)
    }

    /// Borrow the stored key together with its value.
    #[must_use]
    pub fn get_key_value<Q: ?Sized + Ord>(&self, key: &Q) -> Option<(&K, &V)> where K: Borrow<Q> {
        self.find(key).map(|index| { let node = self.node(index); (&node.key, &node.value) })
    }

    /// Mutate an existing value without growing or detaching its map storage.
    #[must_use]
    pub fn get_mut<Q: ?Sized + Ord>(&mut self, key: &Q) -> Option<&mut V> where K: Borrow<Q> {
        self.find(key).map(|index| &mut self.node_mut(index).value)
    }

    /// Test membership under the original borrowed key ordering.
    #[must_use]
    pub fn contains_key<Q: ?Sized + Ord>(&self, key: &Q) -> bool where K: Borrow<Q> {
        self.find(key).is_some()
    }

    /// Insert a retained payload after admitting any actual array growth.
    ///
    /// # Errors
    /// Returns the original checked-layout, admission or allocator refusal.
    pub fn insert(&mut self, key: K, value: V) -> Result<Option<V>, EvalError> {
        let mut parent = None;
        let mut next = self.root;
        let mut left = false;
        while let Some(index) = next {
            let node = self.node(index);
            match key.cmp(&node.key) {
                std::cmp::Ordering::Equal => return Ok(Some(std::mem::replace(&mut self.node_mut(index).value, value))),
                std::cmp::Ordering::Less => { parent = Some(index); left = true; next = node.left; }
                std::cmp::Ordering::Greater => { parent = Some(index); left = false; next = node.right; }
            }
        }
        let node = Node { key, value, parent, left: None, right: None, height: 1 };
        let index = if let Some(index) = self.free {
            self.free = self.slots[index].next_free;
            self.slots.as_mut_slice()[index] = Slot { node: Some(node), next_free: None };
            index
        } else {
            let index = self.slots.len();
            self.slots.push(Slot { node: Some(node), next_free: None })?;
            index
        };
        if let Some(parent) = parent {
            if left { self.node_mut(parent).left = Some(index); }
            else { self.node_mut(parent).right = Some(index); }
        } else { self.root = Some(index); }
        self.len += 1;
        self.rebalance(parent);
        Ok(None)
    }

    fn remove_index(&mut self, mut index: usize) -> (K, V) {
        if self.node(index).left.is_some() && self.node(index).right.is_some() {
            let successor = self.extreme(self.node(index).right.expect("right child checked"), true);
            let (low, high) = if index < successor { (index, successor) } else { (successor, index) };
            let (before, after) = self.slots.as_mut_slice().split_at_mut(high);
            let first = before[low].node.as_mut().expect("ordered swap first is live");
            let second = after[0].node.as_mut().expect("ordered swap second is live");
            std::mem::swap(&mut first.key, &mut second.key);
            std::mem::swap(&mut first.value, &mut second.value);
            index = successor;
        }
        let parent = self.node(index).parent;
        let child = self.node(index).left.or(self.node(index).right);
        self.replace_child(parent, index, child);
        let node = self.slots.as_mut_slice()[index].node.take().expect("removed ordered node is live");
        self.slots.as_mut_slice()[index].next_free = self.free;
        self.free = Some(index);
        self.len -= 1;
        self.rebalance(parent);
        (node.key, node.value)
    }

    /// Remove an entry without allocating or releasing reusable node storage.
    pub fn remove<Q: ?Sized + Ord>(&mut self, key: &Q) -> Option<V> where K: Borrow<Q> {
        self.find(key).map(|index| self.remove_index(index).1)
    }

    /// Move the first ordered payload, retaining reusable admitted metadata.
    pub fn pop_first(&mut self) -> Option<(K, V)> {
        self.root.map(|root| self.extreme(root, true)).map(|index| self.remove_index(index))
    }

    /// Move the last ordered payload, retaining reusable admitted metadata.
    pub fn pop_last(&mut self) -> Option<(K, V)> {
        self.root.map(|root| self.extreme(root, false)).map(|index| self.remove_index(index))
    }
}

/// An allocation-free, bidirectional borrowed walk of admitted ordered storage.
pub struct OrderedIter<'a, K, V> {
    map: &'a AdmittedOrderedMap<K, V>,
    front: Option<usize>,
    back: Option<usize>,
    remaining: usize,
}

impl<'a, K, V> Iterator for OrderedIter<'a, K, V> {
    type Item = (&'a K, &'a V);
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 { return None; }
        let index = self.front.expect("nonempty ordered walk has a front");
        self.front = self.map.neighbor(index, true);
        self.remaining -= 1;
        let node = self.map.node(index);
        Some((&node.key, &node.value))
    }
    fn size_hint(&self) -> (usize, Option<usize>) { (self.remaining, Some(self.remaining)) }
}

impl<K, V> DoubleEndedIterator for OrderedIter<'_, K, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 { return None; }
        let index = self.back.expect("nonempty ordered walk has a back");
        self.back = self.map.neighbor(index, false);
        self.remaining -= 1;
        let node = self.map.node(index);
        Some((&node.key, &node.value))
    }
}

impl<K, V> ExactSizeIterator for OrderedIter<'_, K, V> {}
impl<K, V> std::iter::FusedIterator for OrderedIter<'_, K, V> {}

impl<K: std::fmt::Debug, V: std::fmt::Debug> std::fmt::Debug for AdmittedOrderedMap<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}
