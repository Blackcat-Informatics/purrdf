// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Ordered persistent `SELECT *` variable snapshots.
//!
//! Appending a newly visible name path-copies at most one node per level of a
//! binary trie. Earlier sub-SELECT projections keep their exact ordered prefix
//! without retaining another growing variable array. Reads do not allocate.

use core::fmt;
use core::hash::{Hash, Hasher};

use purrdf_lex::allocation::{Admission, Memory, OrderedTable, Shared, StorageError};

use crate::Variable;
use crate::retained_size::HeapBytes;

#[cfg(test)]
thread_local! {
    static VARIABLE_WORK: core::cell::Cell<u64> = const { core::cell::Cell::new(0) };
}

/// Count a variable visit or copy in the parser's nested projection path.
#[cfg(test)]
pub(crate) fn record_variable_work() {
    VARIABLE_WORK.with(|work| work.set(work.get() + 1));
}

#[cfg(test)]
pub(crate) fn take_variable_work() -> u64 {
    VARIABLE_WORK.with(|work| work.replace(0))
}

/// Borrowed names of either an explicit projection or a persistent star snapshot.
#[derive(Clone, Copy, Debug)]
pub enum ProjectionNames<'a> {
    /// Explicit `SELECT ?name` variable slice.
    Explicit(&'a [Variable]),
    /// Shared `SELECT *` snapshot.
    Star(&'a ProjectionVars),
}

impl<'a> ProjectionNames<'a> {
    /// Visit names in the order they become visible.
    #[must_use]
    pub fn iter(self) -> ProjectionNamesIter<'a> {
        match self {
            Self::Explicit(names) => ProjectionNamesIter::Explicit(names.iter()),
            Self::Star(names) => ProjectionNamesIter::Star(names.iter()),
        }
    }

    /// Whether this projection retains a name.
    #[must_use]
    pub fn contains(self, name: &Variable) -> bool {
        self.iter().any(|candidate| candidate == name)
    }

    /// Number of names retained by this projection.
    #[must_use]
    pub fn len(self) -> usize {
        match self {
            Self::Explicit(names) => names.len(),
            Self::Star(names) => names.len(),
        }
    }

    /// Whether this projection retains no names.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }

    /// Borrow the name at a first-appearance position.
    #[must_use]
    pub fn get(self, index: usize) -> Option<&'a Variable> {
        match self {
            Self::Explicit(names) => names.get(index),
            Self::Star(names) => names.get(index),
        }
    }
}

/// Iterator over the two projection storage forms.
#[derive(Debug)]
pub enum ProjectionNamesIter<'a> {
    /// An explicit variable slice.
    Explicit(core::slice::Iter<'a, Variable>),
    /// A persistent star snapshot.
    Star(ProjectionIter<'a>),
}

impl<'a> Iterator for ProjectionNamesIter<'a> {
    type Item = &'a Variable;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Explicit(names) => names.next(),
            Self::Star(names) => names.next(),
        }
    }
}

enum Node {
    Leaf(Variable),
    Branch {
        left: Option<Shared<Self>>,
        right: Option<Shared<Self>>,
    },
}

/// A first-appearance-ordered, immutable star projection. `Clone` retains the
/// published root; `push_with_memory` copies only its binary search path.
#[derive(Clone, Default)]
pub struct ProjectionVars {
    root: Option<Shared<Node>>,
    len: usize,
    capacity: usize,
}

impl ProjectionVars {
    fn same_prefix(
        left: Option<&Shared<Node>>,
        right: Option<&Shared<Node>>,
        span: usize,
        keep: usize,
    ) -> bool {
        if keep == 0 {
            return true;
        }
        let (Some(left), Some(right)) = (left, right) else {
            return false;
        };
        if core::ptr::eq(&raw const **left, &raw const **right) {
            return true;
        }
        if span == 1 {
            return matches!((&**left, &**right), (Node::Leaf(a), Node::Leaf(b)) if a == b);
        }
        let (
            Node::Branch {
                left: left_left,
                right: left_right,
            },
            Node::Branch {
                left: right_left,
                right: right_right,
            },
        ) = (&**left, &**right)
        else {
            return false;
        };
        let half = span / 2;
        Self::same_prefix(
            left_left.as_ref(),
            right_left.as_ref(),
            half,
            keep.min(half),
        ) && Self::same_prefix(
            left_right.as_ref(),
            right_right.as_ref(),
            half,
            keep.saturating_sub(half),
        )
    }

    /// Whether this snapshot extends `prefix` by exactly these visible names.
    /// Parser-built snapshots share unchanged trie subtrees, so thousands of
    /// enclosing stars compare only the copied search paths.
    pub(crate) fn is_appended_to<'a>(
        &self,
        prefix: &Self,
        appended: impl IntoIterator<Item = &'a Variable>,
    ) -> bool {
        let mut index = prefix.len;
        for variable in appended {
            if self.get(index) != Some(variable) {
                return false;
            }
            index += 1;
        }
        if index != self.len {
            return false;
        }
        self.has_prefix(prefix)
    }

    /// Whether the saved leading names still match a shared earlier snapshot.
    pub(crate) fn has_prefix(&self, prefix: &Self) -> bool {
        if prefix.len > self.len {
            return false;
        }
        if prefix.len == 0 {
            return true;
        }
        let Some(root) = self.root.as_ref() else {
            return false;
        };
        let Some(prefix_root) = prefix.root.as_ref() else {
            return false;
        };
        let mut root = root;
        let mut span = self.capacity;
        while span > prefix.capacity {
            let Node::Branch { left, .. } = &**root else {
                return false;
            };
            let Some(left) = left.as_ref() else {
                return false;
            };
            root = left;
            span /= 2;
        }
        span == prefix.capacity
            && Self::same_prefix(Some(root), Some(prefix_root), span, prefix.len)
    }

    /// Number of visible variables.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Whether this projection is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The variable at its first-appearance position.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&Variable> {
        if index >= self.len {
            return None;
        }
        let mut node = self.root.as_ref()?;
        let mut span = self.capacity;
        let mut offset = index;
        while span > 1 {
            let Node::Branch { left, right } = &**node else {
                unreachable!("a projection trie branch spans more than one slot")
            };
            span /= 2;
            node = if offset < span {
                left.as_ref()
            } else {
                offset -= span;
                right.as_ref()
            }?;
        }
        match &**node {
            Node::Leaf(variable) => Some(variable),
            Node::Branch { .. } => unreachable!("a projection trie leaf spans one slot"),
        }
    }

    /// Iterate in first-appearance order without an intermediate array.
    #[must_use]
    pub fn iter(&self) -> ProjectionIter<'_> {
        ProjectionIter {
            projection: self,
            front: 0,
            back: self.len,
        }
    }

    fn allocate<S: Admission + ?Sized>(
        node: Node,
        memory: &mut Memory<'_, S>,
    ) -> Result<Shared<Node>, StorageError> {
        memory.add_bytes(Shared::<Node>::allocation_layout().size())?;
        let shared = Shared::try_new(node).map_err(|_| StorageError::AllocationFailed)?;
        Ok(shared)
    }

    fn insert<S: Admission + ?Sized>(
        root: Option<&Shared<Node>>,
        span: usize,
        index: usize,
        variable: &Variable,
        memory: &mut Memory<'_, S>,
    ) -> Result<Shared<Node>, StorageError> {
        if span == 1 {
            return Self::allocate(Node::Leaf(variable.clone()), memory);
        }
        let half = span / 2;
        let (left, right) = match root {
            Some(root) => match &**root {
                Node::Branch { left, right } => (left.clone(), right.clone()),
                Node::Leaf(_) => unreachable!("a projection trie branch spans more than one slot"),
            },
            None => (None, None),
        };
        let (left, right) = if index < half {
            (
                Some(Self::insert(left.as_ref(), half, index, variable, memory)?),
                right,
            )
        } else {
            (
                left,
                Some(Self::insert(
                    right.as_ref(),
                    half,
                    index - half,
                    variable,
                    memory,
                )?),
            )
        };
        Self::allocate(Node::Branch { left, right }, memory)
    }

    /// Append a newly visible name after admitting every copied trie node.
    ///
    /// # Errors
    /// Returns typed size or allocation refusal before publishing a new root.
    pub(crate) fn push_with_memory<S: Admission + ?Sized>(
        &mut self,
        variable: &Variable,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        let next_len = self.len.checked_add(1).ok_or(StorageError::SizeOverflow)?;
        let before = memory.admitted_bytes();
        let assembled = (|| {
            if self.len == 0 {
                Ok((Self::insert(None, 1, 0, variable, memory)?, 1))
            } else if self.len == self.capacity {
                let capacity = self
                    .capacity
                    .checked_mul(2)
                    .ok_or(StorageError::SizeOverflow)?;
                let right = Self::insert(None, self.capacity, 0, variable, memory)?;
                let root = Self::allocate(
                    Node::Branch {
                        left: self.root.clone(),
                        right: Some(right),
                    },
                    memory,
                )?;
                Ok((root, capacity))
            } else {
                Ok((
                    Self::insert(
                        self.root.as_ref(),
                        self.capacity,
                        self.len,
                        variable,
                        memory,
                    )?,
                    self.capacity,
                ))
            }
        })();
        let (root, capacity) = match assembled {
            Ok(value) => value,
            Err(error) => {
                let unpublished = memory
                    .admitted_bytes()
                    .checked_sub(before)
                    .ok_or(StorageError::SizeOverflow)?;
                memory.release_bytes(unpublished)?;
                return Err(error);
            }
        };
        self.root = Some(root);
        self.capacity = capacity;
        self.len = next_len;
        Ok(())
    }

    /// Charge each reachable shared node once across every projection root in
    /// one algebra. A root retained alone still owns all of its ancestors;
    /// path-copy nodes dropped before publication are no longer charged.
    pub(crate) fn reachable_bytes<S: Admission + ?Sized>(
        &self,
        seen: &mut OrderedTable<usize, ()>,
        memory: &mut Memory<'_, S>,
        raw: bool,
    ) -> Result<usize, StorageError> {
        let mut work = Vec::new();
        if let Some(root) = &self.root {
            memory.push(&mut work, root)?;
        }
        let mut bytes = 0usize;
        while let Some(node) = work.pop() {
            let address = (&raw const **node) as usize;
            if seen.insert(address, (), memory)?.is_some() {
                continue;
            }
            bytes = bytes
                .checked_add(Shared::<Node>::allocation_layout().size())
                .ok_or(StorageError::SizeOverflow)?;
            match &**node {
                Node::Leaf(variable) => {
                    if !raw {
                        bytes = bytes
                            .checked_add(variable.heap_bytes(false))
                            .ok_or(StorageError::SizeOverflow)?;
                    }
                }
                Node::Branch { left, right } => {
                    if let Some(left) = left {
                        memory.push(&mut work, left)?;
                    }
                    if let Some(right) = right {
                        memory.push(&mut work, right)?;
                    }
                }
            }
        }
        memory.release_vec(work)?;
        Ok(bytes)
    }
}

/// Allocation-free forward or reverse traversal of a projection snapshot.
#[derive(Debug)]
pub struct ProjectionIter<'a> {
    projection: &'a ProjectionVars,
    front: usize,
    back: usize,
}

impl<'a> Iterator for ProjectionIter<'a> {
    type Item = &'a Variable;

    fn next(&mut self) -> Option<Self::Item> {
        if self.front == self.back {
            return None;
        }
        #[cfg(test)]
        record_variable_work();
        let index = self.front;
        self.front += 1;
        self.projection.get(index)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.back - self.front;
        (remaining, Some(remaining))
    }
}

impl DoubleEndedIterator for ProjectionIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.front == self.back {
            return None;
        }
        #[cfg(test)]
        record_variable_work();
        self.back -= 1;
        self.projection.get(self.back)
    }
}

impl ExactSizeIterator for ProjectionIter<'_> {}

impl<'a> IntoIterator for &'a ProjectionVars {
    type Item = &'a Variable;
    type IntoIter = ProjectionIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl PartialEq for ProjectionVars {
    fn eq(&self, other: &Self) -> bool {
        self.len == other.len && self.iter().eq(other.iter())
    }
}
impl Eq for ProjectionVars {}

impl Hash for ProjectionVars {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.len.hash(state);
        for variable in self {
            variable.hash(state);
        }
    }
}

impl fmt::Debug for ProjectionVars {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_lex::allocation::Resident;

    #[test]
    fn outer_only_snapshot_charges_every_live_node_and_releases_last_owner() {
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let mut projection = ProjectionVars::default();
        for index in 0..4 {
            projection
                .push_with_memory(&Variable::new(format!("v{index}")), &mut memory)
                .expect("admitted append");
        }
        let root = projection.root.as_ref().expect("nonempty root");
        assert_eq!(root.strong_count(), 1);
        let clone = projection.clone();
        assert_eq!(root.strong_count(), 2);
        drop(clone);
        assert_eq!(root.strong_count(), 1);

        let mut seen = OrderedTable::default();
        let bytes = projection
            .reachable_bytes(&mut seen, &mut memory, true)
            .expect("reachable census");
        assert_eq!(bytes, 7 * Shared::<Node>::allocation_layout().size());
        assert_eq!(
            projection
                .reachable_bytes(&mut seen, &mut memory, true)
                .expect("same root is counted once"),
            0
        );
        seen.release(&mut memory).expect("release scratch census");
        drop(projection);
    }

    #[test]
    fn mid_path_refusal_keeps_the_old_root_and_refunds_unpublished_nodes() {
        struct Limit {
            maximum: usize,
        }
        impl Admission for Limit {
            fn resize(&mut self, live: usize) -> Result<(), StorageError> {
                if live > self.maximum {
                    Err(StorageError::AdmissionFailed)
                } else {
                    Ok(())
                }
            }
        }
        let mut limit = Limit {
            maximum: usize::MAX,
        };
        let mut memory = Memory::new(&mut limit);
        let mut projection = ProjectionVars::default();
        projection
            .push_with_memory(&Variable::new("first"), &mut memory)
            .unwrap();
        let before = memory.admitted_bytes();
        memory.admission_mut().maximum = before + Shared::<Node>::allocation_layout().size();
        assert_eq!(
            projection.push_with_memory(&Variable::new("second"), &mut memory),
            Err(StorageError::AdmissionFailed)
        );
        assert_eq!(memory.admitted_bytes(), before);
        assert_eq!(projection.len(), 1);
        assert_eq!(projection.get(0), Some(&Variable::new("first")));
        assert_eq!(projection.root.as_ref().unwrap().strong_count(), 1);
    }
}
