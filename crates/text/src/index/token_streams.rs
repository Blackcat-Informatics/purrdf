// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Persistent ordered storage shared by document, dictionary and posting tables.
//! Leaves carry up to 32 values; an edit copies only its leaf and AVL path.

use super::{Shared, TextError, TokenStream};
use crate::query_workspace::{CapacityFailure, overflow};

const LEAF: usize = 32;
// An AVL tree over a usize-sized sequence has height below twice usize::BITS.
const PATH: usize = usize::BITS as usize * 2;
type Link<T> = Option<Shared<Node<T>>>;

#[derive(Clone, Debug)]
pub(crate) struct Table<T> {
    root: Link<T>,
}

#[derive(Debug)]
enum Node<T> {
    Leaf(Vec<T>),
    Branch {
        left: Shared<Self>,
        right: Shared<Self>,
        len: usize,
        height: u8,
    },
}

fn allocate<T>(node: Node<T>) -> Result<Shared<Node<T>>, TextError> {
    #[cfg(test)]
    NODE_BUILDS.set(NODE_BUILDS.get() + 1);
    Shared::try_new(node).map_err(|_| {
        TextError::Capacity(CapacityFailure::Allocator {
            construct: "text index table",
        })
    })
}

fn handles<T>(capacity: usize) -> Result<Vec<T>, TextError> {
    let mut result = Vec::new();
    result.try_reserve_exact(capacity).map_err(|_| {
        TextError::Capacity(CapacityFailure::Allocator {
            construct: "text index leaf",
        })
    })?;
    Ok(result)
}

fn copy_handles<'a, T: Clone + 'a>(output: &mut Vec<T>, input: impl IntoIterator<Item = &'a T>) {
    for item in input {
        #[cfg(test)]
        HANDLE_CLONES.set(HANDLE_CLONES.get() + 1);
        output.push(item.clone());
    }
}

impl<T> Node<T> {
    fn len(&self) -> usize {
        match self {
            Self::Leaf(items) => items.len(),
            Self::Branch { len, .. } => *len,
        }
    }
    fn height(&self) -> u8 {
        match self {
            Self::Leaf(_) => 1,
            Self::Branch { height, .. } => *height,
        }
    }
    fn children(&self) -> (&Shared<Self>, &Shared<Self>) {
        let Self::Branch { left, right, .. } = self else {
            unreachable!("a taller AVL node is a branch");
        };
        (left, right)
    }
}

fn branch<T>(left: Shared<Node<T>>, right: Shared<Node<T>>) -> Result<Shared<Node<T>>, TextError> {
    let len = left.len().checked_add(right.len()).ok_or_else(overflow)?;
    let height = left
        .height()
        .max(right.height())
        .checked_add(1)
        .ok_or_else(overflow)?;
    allocate(Node::Branch {
        left,
        right,
        len,
        height,
    })
}

fn balance<T>(left: Shared<Node<T>>, right: Shared<Node<T>>) -> Result<Shared<Node<T>>, TextError> {
    if left.height() > right.height() + 1 {
        let (a, b) = left.children();
        if a.height() >= b.height() {
            return branch(a.clone(), branch(b.clone(), right)?);
        }
        let (b0, b1) = b.children();
        return branch(branch(a.clone(), b0.clone())?, branch(b1.clone(), right)?);
    }
    if right.height() > left.height() + 1 {
        let (a, b) = right.children();
        if b.height() >= a.height() {
            return branch(branch(left, a.clone())?, b.clone());
        }
        let (a0, a1) = a.children();
        return branch(branch(left, a0.clone())?, branch(a1.clone(), b.clone())?);
    }
    branch(left, right)
}

fn join<T: Clone>(left: Link<T>, right: Link<T>) -> Result<Link<T>, TextError> {
    let (Some(left), Some(right)) = (left.as_ref(), right.as_ref()) else {
        return Ok(left.or(right));
    };
    if left.height() > right.height() + 1 {
        let (a, b) = left.children();
        let inner = join(Some(b.clone()), Some(right.clone()))?
            .expect("joining nonempty trees stays nonempty");
        return balance(a.clone(), inner).map(Some);
    }
    if right.height() > left.height() + 1 {
        let (a, b) = right.children();
        let inner = join(Some(left.clone()), Some(a.clone()))?
            .expect("joining nonempty trees stays nonempty");
        return balance(inner, b.clone()).map(Some);
    }
    if let (Node::Leaf(a), Node::Leaf(b)) = (&**left, &**right)
        && a.len() + b.len() <= LEAF
    {
        let mut items = handles(a.len() + b.len())?;
        copy_handles(&mut items, a.iter().chain(b));
        return allocate(Node::Leaf(items)).map(Some);
    }
    branch(left.clone(), right.clone()).map(Some)
}

impl<T> Default for Table<T> {
    fn default() -> Self {
        Self { root: None }
    }
}

pub(super) type TokenTable = Table<TokenStream>;

impl<T: Clone> Table<T> {
    #[cfg(test)]
    pub(crate) fn height(&self) -> u8 {
        self.root.as_ref().map_or(0, |node| node.height())
    }
    pub(crate) fn from_streams(streams: Vec<T>) -> Result<Self, TextError> {
        let count = streams.len();
        let mut input = streams.into_iter();
        fn build<T: Clone>(
            count: usize,
            input: &mut impl Iterator<Item = T>,
        ) -> Result<Link<T>, TextError> {
            if count == 0 {
                return Ok(None);
            }
            if count <= LEAF {
                let mut items = handles(count)?;
                items.extend(input.take(count));
                return allocate(Node::Leaf(items)).map(Some);
            }
            let left = build(count / 2, input)?;
            let right = build(count - count / 2, input)?;
            join(left, right)
        }
        Ok(Self {
            root: build(count, &mut input)?,
        })
    }

    pub(crate) fn len(&self) -> usize {
        self.root.as_ref().map_or(0, |node| node.len())
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub(crate) fn binary_search_by(
        &self,
        compare: impl FnMut(&T) -> core::cmp::Ordering,
    ) -> Result<usize, usize> {
        self.search_range(0, self.len(), compare)
    }

    fn search_range(
        &self,
        mut low: usize,
        mut high: usize,
        mut compare: impl FnMut(&T) -> core::cmp::Ordering,
    ) -> Result<usize, usize> {
        while low < high {
            let middle = low + (high - low) / 2;
            match compare(&self[middle]) {
                core::cmp::Ordering::Less => low = middle + 1,
                core::cmp::Ordering::Equal => return Ok(middle),
                core::cmp::Ordering::Greater => high = middle,
            }
        }
        Err(low)
    }

    pub(crate) fn partition_point(&self, mut before: impl FnMut(&T) -> bool) -> usize {
        let mut low = 0;
        let mut high = self.len();
        while low < high {
            let middle = low + (high - low) / 2;
            if before(&self[middle]) {
                low = middle + 1;
            } else {
                high = middle;
            }
        }
        low
    }

    pub(crate) fn try_binary_search_by<E>(
        &self,
        mut compare: impl FnMut(&T) -> Result<core::cmp::Ordering, E>,
    ) -> Result<Result<usize, usize>, E> {
        let mut failure = None;
        let found = self.binary_search_by(|item| {
            if failure.is_some() {
                return core::cmp::Ordering::Equal;
            }
            match compare(item) {
                Ok(order) => order,
                Err(error) => {
                    failure = Some(error);
                    core::cmp::Ordering::Equal
                }
            }
        });
        failure.map_or(Ok(found), Err)
    }

    pub(crate) fn try_partition_point<E>(
        &self,
        mut before: impl FnMut(&T) -> Result<bool, E>,
    ) -> Result<usize, E> {
        let mut failure = None;
        let found = self.partition_point(|item| {
            if failure.is_some() {
                return false;
            }
            match before(item) {
                Ok(before) => before,
                Err(error) => {
                    failure = Some(error);
                    false
                }
            }
        });
        failure.map_or(Ok(found), Err)
    }

    pub(crate) fn replace(&self, index: usize, value: T) -> Result<Self, TextError> {
        fn replace<T: Clone>(
            node: &Shared<Node<T>>,
            index: usize,
            value: T,
        ) -> Result<Shared<Node<T>>, TextError> {
            match &**node {
                Node::Leaf(items) => {
                    let mut next = handles(items.len())?;
                    copy_handles(&mut next, &items[..index]);
                    next.push(value);
                    copy_handles(&mut next, &items[index + 1..]);
                    allocate(Node::Leaf(next))
                }
                Node::Branch { left, right, .. } => {
                    if index < left.len() {
                        branch(replace(left, index, value)?, right.clone())
                    } else {
                        branch(left.clone(), replace(right, index - left.len(), value)?)
                    }
                }
            }
        }
        if index >= self.len() {
            return Err(TextError::data(
                "index table replacement is outside its row space",
            ));
        }
        Ok(Self {
            root: Some(replace(
                self.root.as_ref().expect("a nonempty table has its root"),
                index,
                value,
            )?),
        })
    }

    pub(crate) fn slice(&self, start: usize, end: usize) -> Slice<'_, T> {
        assert!(start <= end && end <= self.len(), "index-owned table range");
        Slice {
            table: Some(self),
            start,
            end,
        }
    }

    fn range(&self, start: usize, end: usize) -> Iter<'_, T> {
        let mut result = Iter {
            pending: [None; PATH],
            depth: 0,
            leaf: [].iter(),
            remaining: end - start,
        };
        if start == end {
            return result;
        }
        let mut node = self.root.as_deref().expect("a nonempty table has its root");
        let mut index = start;
        loop {
            match node {
                Node::Leaf(items) => {
                    result.leaf = items[index..].iter();
                    return result;
                }
                Node::Branch { left, right, .. } => {
                    if index < left.len() {
                        result.pending[result.depth] = Some(right);
                        result.depth += 1;
                        node = left;
                    } else {
                        index -= left.len();
                        node = right;
                    }
                }
            }
        }
    }

    pub(crate) fn get(&self, mut index: usize) -> Option<&T> {
        let mut node = self.root.as_deref()?;
        if index >= node.len() {
            return None;
        }
        loop {
            match node {
                Node::Leaf(items) => return items.get(index),
                Node::Branch { left, right, .. } => {
                    if index < left.len() {
                        node = left;
                    } else {
                        index -= left.len();
                        node = right;
                    }
                }
            }
        }
    }

    pub(crate) fn insert(&self, index: usize, value: T) -> Result<Self, TextError> {
        if index > self.len() {
            return Err(TextError::data(
                "token table insertion is outside the document space",
            ));
        }
        self.len().checked_add(1).ok_or_else(overflow)?;
        fn insert<T: Clone>(
            node: &Shared<Node<T>>,
            index: usize,
            value: T,
        ) -> Result<Link<T>, TextError> {
            match &**node {
                Node::Leaf(items) => {
                    let mut next = handles(items.len() + 1)?;
                    copy_handles(&mut next, &items[..index]);
                    next.push(value);
                    copy_handles(&mut next, &items[index..]);
                    if next.len() <= LEAF {
                        return allocate(Node::Leaf(next)).map(Some);
                    }
                    let mut right = handles(next.len() / 2)?;
                    let middle = next.len() - next.len() / 2;
                    right.extend(next.drain(middle..));
                    join(
                        Some(allocate(Node::Leaf(next))?),
                        Some(allocate(Node::Leaf(right))?),
                    )
                }
                Node::Branch { left, right, .. } => {
                    if index <= left.len() {
                        join(insert(left, index, value)?, Some(right.clone()))
                    } else {
                        join(
                            Some(left.clone()),
                            insert(right, index - left.len(), value)?,
                        )
                    }
                }
            }
        }
        let root = match &self.root {
            Some(root) => insert(root, index, value)?,
            None => {
                let mut items = handles(1)?;
                items.push(value);
                Some(allocate(Node::Leaf(items))?)
            }
        };
        Ok(Self { root })
    }

    pub(crate) fn remove(&self, index: usize) -> Result<Self, TextError> {
        if index >= self.len() {
            return Err(TextError::data(
                "token table removal is outside the document space",
            ));
        }
        fn remove<T: Clone>(node: &Shared<Node<T>>, index: usize) -> Result<Link<T>, TextError> {
            match &**node {
                Node::Leaf(items) => {
                    if items.len() == 1 {
                        return Ok(None);
                    }
                    let mut next = handles(items.len() - 1)?;
                    copy_handles(&mut next, &items[..index]);
                    copy_handles(&mut next, &items[index + 1..]);
                    allocate(Node::Leaf(next)).map(Some)
                }
                Node::Branch { left, right, .. } => {
                    if index < left.len() {
                        join(remove(left, index)?, Some(right.clone()))
                    } else {
                        join(Some(left.clone()), remove(right, index - left.len())?)
                    }
                }
            }
        }
        Ok(Self {
            root: remove(
                self.root.as_ref().expect("a nonempty table has its root"),
                index,
            )?,
        })
    }

    pub(crate) fn iter(&self) -> Iter<'_, T> {
        self.range(0, self.len())
    }
}

pub(crate) struct Slice<'a, T> {
    table: Option<&'a Table<T>>,
    start: usize,
    end: usize,
}
impl<T> Copy for Slice<'_, T> {}
impl<T> Clone for Slice<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, T: Clone> Slice<'a, T> {
    pub(crate) fn binary_search_by(
        &self,
        compare: impl FnMut(&T) -> core::cmp::Ordering,
    ) -> Result<usize, usize> {
        self.table.map_or(Err(0), |table| {
            table
                .search_range(self.start, self.end, compare)
                .map(|at| at - self.start)
                .map_err(|at| at - self.start)
        })
    }
    pub(crate) const fn empty() -> Self {
        Self {
            table: None,
            start: 0,
            end: 0,
        }
    }
    pub(crate) const fn len(&self) -> usize {
        self.end - self.start
    }
    pub(crate) fn get(&self, at: usize) -> Option<&'a T> {
        if at >= self.len() {
            return None;
        }
        self.table?.get(self.start + at)
    }
    pub(crate) fn iter(&self) -> Iter<'a, T> {
        self.table.map_or_else(
            || Iter {
                pending: [None; PATH],
                depth: 0,
                leaf: [].iter(),
                remaining: 0,
            },
            |table| table.range(self.start, self.end),
        )
    }
}

impl<T: Clone> core::ops::Index<usize> for Table<T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).expect("index-owned token document")
    }
}

#[derive(Clone)]
pub(crate) struct Iter<'a, T> {
    pending: [Option<&'a Node<T>>; PATH],
    depth: usize,
    leaf: core::slice::Iter<'a, T>,
    remaining: usize,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        loop {
            if let Some(item) = self.leaf.next() {
                self.remaining -= 1;
                return Some(item);
            }
            self.depth = self.depth.checked_sub(1)?;
            let mut node = self.pending[self.depth]
                .take()
                .expect("an active token path has its node");
            loop {
                match node {
                    Node::Leaf(items) => {
                        self.leaf = items.iter();
                        break;
                    }
                    Node::Branch { left, right, .. } => {
                        self.pending[self.depth] = Some(right);
                        self.depth += 1;
                        node = left;
                    }
                }
            }
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}
impl<T> ExactSizeIterator for Iter<'_, T> {}
impl<T> core::iter::FusedIterator for Iter<'_, T> {}

#[cfg(test)]
std::thread_local! {
    pub(crate) static NODE_BUILDS: core::cell::Cell<usize> = const { core::cell::Cell::new(0) };
    pub(crate) static HANDLE_CLONES: core::cell::Cell<usize> = const { core::cell::Cell::new(0) };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::{StoredToken, share_buffer, share_index};
    use purrdf_testkit::rng::splitmix64_next;

    fn value(ordinal: usize) -> TokenStream {
        share_buffer(vec![StoredToken {
            term: share_index(String::from("word")).unwrap(),
            position: 0,
            predicate: 0,
            ordinal,
        }])
        .unwrap()
    }

    fn values(table: &TokenTable) -> Vec<usize> {
        table
            .iter()
            .map(|stream| stream.as_slice()[0].ordinal)
            .collect()
    }

    fn invariants(node: &Node<TokenStream>) -> (usize, u8) {
        match node {
            Node::Leaf(items) => {
                assert!((1..=LEAF).contains(&items.len()));
                (items.len(), 1)
            }
            Node::Branch {
                left,
                right,
                len,
                height,
            } => {
                let (a, ha) = invariants(left);
                let (b, hb) = invariants(right);
                assert!(ha.abs_diff(hb) <= 1, "unbalanced children: {ha}, {hb}");
                assert_eq!(*len, a + b);
                assert_eq!(*height, ha.max(hb) + 1);
                (*len, *height)
            }
        }
    }

    #[test]
    fn positional_edits_match_a_vector_and_preserve_prior_snapshots() {
        let mut state = 0x4192_be71;
        let mut table = TokenTable::default();
        let mut model = Vec::new();
        for ordinal in 0..4096 {
            let old = table.clone();
            let prior = model.clone();
            let choice = splitmix64_next(&mut state) % 5;
            if model.is_empty() || choice < 3 {
                let at = (splitmix64_next(&mut state) % (model.len() as u64 + 1)) as usize;
                table = table.insert(at, value(ordinal)).unwrap();
                model.insert(at, ordinal);
            } else if choice == 3 {
                let at = (splitmix64_next(&mut state) % model.len() as u64) as usize;
                table = table.replace(at, value(ordinal)).unwrap();
                model[at] = ordinal;
            } else {
                let at = (splitmix64_next(&mut state) % model.len() as u64) as usize;
                table = table.remove(at).unwrap();
                model.remove(at);
            }
            assert_eq!(values(&table), model);
            assert_eq!(values(&old), prior);
            assert_eq!(table.iter().len(), model.len());
            let start = (splitmix64_next(&mut state) % (model.len() as u64 + 1)) as usize;
            let end = start
                + (splitmix64_next(&mut state) % (model.len() as u64 - start as u64 + 1)) as usize;
            assert_eq!(
                table
                    .slice(start, end)
                    .iter()
                    .map(|stream| stream.as_slice()[0].ordinal)
                    .collect::<Vec<_>>(),
                model[start..end]
            );
            assert!(table.get(model.len()).is_none());
            for (at, &expected) in model.iter().enumerate() {
                assert_eq!(table.get(at).unwrap().as_slice()[0].ordinal, expected);
            }
            if let Some(root) = &table.root {
                assert_eq!(invariants(root).0, model.len());
            }
        }
        while !model.is_empty() {
            let at = (splitmix64_next(&mut state) % model.len() as u64) as usize;
            table = table.remove(at).unwrap();
            model.remove(at);
            assert_eq!(values(&table), model);
            if let Some(root) = &table.root {
                invariants(root);
            }
        }
        assert!(table.root.is_none());
        assert!(table.remove(0).is_err());
        assert!(table.replace(0, value(0)).is_err());
        assert!(table.insert(1, value(0)).is_err());
        assert!(table.iter().next().is_none());
        assert!(table.iter().next().is_none());
    }

    #[test]
    fn cloning_is_constant_and_fixed_edits_copy_only_chunks_and_paths() {
        let mut measurements = Vec::new();
        for count in [128, 4096, 131_072] {
            let table = TokenTable::from_streams(vec![TokenStream::default(); count]).unwrap();
            let height = table.root.as_ref().unwrap().height();
            NODE_BUILDS.set(0);
            HANDLE_CLONES.set(0);
            let prior = table.clone();
            assert_eq!(NODE_BUILDS.get(), 0);
            assert_eq!(HANDLE_CLONES.get(), 0);
            let next = table
                .insert(count / 2, value(count))
                .unwrap()
                .remove(count / 2 + 1)
                .unwrap();
            let nodes = NODE_BUILDS.get();
            let handles = HANDLE_CLONES.get();
            assert_eq!(next.len(), count);
            assert_eq!(next.get(count / 2).unwrap().as_slice()[0].ordinal, count);
            assert_eq!(prior.get(count / 2).unwrap().as_slice().len(), 0);
            assert!(nodes <= 12 * (usize::from(height) + 1));
            assert!(handles <= LEAF * 12 * (usize::from(height) + 1));
            invariants(next.root.as_ref().unwrap());
            measurements.push((count, height, nodes, handles));
        }
        eprintln!(
            "persistent token table (documents, height, new nodes, copied handles): {measurements:?}"
        );
    }
}
