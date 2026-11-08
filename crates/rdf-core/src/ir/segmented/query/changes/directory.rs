// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Source-independent effective keys, retained without physical row addresses.
//! Chunked Patricia paths copy only changed leaves; every owner keeps its grant.

use super::{Multiset256, QueryStorage, SegmentedError, SegmentedOperation, Shared};
use crate::ir::source_directory::{
    DirectoryEntry, DirectoryRead, first_differing_bit as difference,
};
use crate::source::SourceDifferenceWork;
use purrdf_lex::walk::WorkList;

const LEAF_KEYS: usize = 32;

#[derive(Clone, Debug, Default)]
pub(super) struct EffectiveDirectory {
    root: Option<Shared<Node>>,
}

#[derive(Debug)]
enum Kind {
    Leaf(Box<[[u8; 32]]>),
    Branch {
        bit: u16,
        left: Shared<Node>,
        right: Shared<Node>,
    },
}

#[derive(Debug)]
struct Node {
    first: [u8; 32],
    summary: Multiset256,
    kind: Kind,
    // Keys and child references drop before their original allocation grant.
    _storage: QueryStorage,
}

#[cfg(test)]
std::thread_local! {
    static BUILDS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static COPIES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

fn allocate(
    first: [u8; 32],
    summary: Multiset256,
    kind: Kind,
    mut storage: QueryStorage,
) -> Result<Shared<Node>, SegmentedError> {
    storage.run(|memory| memory.add_bytes(Shared::<Node>::allocation_layout().size()))?;
    #[cfg(test)]
    BUILDS.set(BUILDS.get() + 1);
    Shared::try_new(Node {
        first,
        summary,
        kind,
        _storage: storage,
    })
    .map_err(|_| SegmentedError::AllocationFailed("effective source directory node"))
}

fn leaf(keys: &[[u8; 32]], operation: &SegmentedOperation) -> Result<Shared<Node>, SegmentedError> {
    debug_assert!(!keys.is_empty() && keys.len() <= LEAF_KEYS);
    let mut storage = QueryStorage::new(operation)?;
    let owned = storage.run(|memory| {
        let keys = memory.collect(keys.iter().copied())?;
        memory.boxed_slice(keys)
    })?;
    let mut summary = Multiset256::default();
    for key in keys {
        summary
            .insert(*key)
            .map_err(|_| SegmentedError::AddressExhausted)?;
    }
    #[cfg(test)]
    COPIES.set(COPIES.get() + keys.len());
    allocate(keys[0], summary, Kind::Leaf(owned), storage)
}

fn branch(
    bit: u16,
    left: Shared<Node>,
    right: Shared<Node>,
    operation: &SegmentedOperation,
) -> Result<Shared<Node>, SegmentedError> {
    let mut summary = left.summary;
    summary
        .combine(right.summary)
        .map_err(|_| SegmentedError::AddressExhausted)?;
    allocate(
        left.first,
        summary,
        Kind::Branch { bit, left, right },
        QueryStorage::new(operation)?,
    )
}

fn right(key: &[u8; 32], bit: u16) -> bool {
    key[usize::from(bit / 8)] & (0x80 >> (bit % 8)) != 0
}

enum Build {
    Range(std::ops::Range<usize>),
    Join(u16),
}

impl EffectiveDirectory {
    pub(super) fn summary(&self) -> Multiset256 {
        self.root
            .as_ref()
            .map_or_else(Multiset256::default, |root| root.summary)
    }

    /// Positive source composition needs only independently certified keys.
    /// Unchanged members neither resolve RDF values nor copy tree paths.
    pub(super) fn with_source_union<D: DirectoryRead>(
        &self,
        source: &D,
        operation: &SegmentedOperation,
    ) -> Result<Self, SegmentedError>
    where
        SegmentedError: From<D::Error>,
    {
        let mut next = self.clone();
        for index in 0..source.len()? {
            let key = source.record(index)?.digest();
            if next.find(key)?.is_none() {
                next = next.set(key, true, operation)?;
            }
        }
        Ok(next)
    }

    /// Initial capture alone copies the sorted keys. Subsequent versions share
    /// this independent storage, never a prior dataset, provider or local ID.
    pub(super) fn from_source<D: DirectoryRead>(
        source: &D,
        operation: &SegmentedOperation,
    ) -> Result<Self, SegmentedError>
    where
        SegmentedError: From<D::Error>,
    {
        let mut storage = QueryStorage::new(operation)?;
        let mut keys = Vec::new();
        let mut pending = WorkList::<_, 0>::new();
        let mut complete = WorkList::<_, 0>::new();
        storage.run(|memory| -> Result<(), SegmentedError> {
            for index in 0..source.len()? {
                memory.push(&mut keys, source.record(index)?.digest())?;
            }
            if !keys.is_empty() {
                pending.try_push_admitted(Build::Range(0..keys.len()), memory)?;
            }
            while let Some(task) = pending.pop() {
                match task {
                    Build::Range(range) if range.len() <= LEAF_KEYS => {
                        complete.try_push_admitted(leaf(&keys[range], operation)?, memory)?;
                    }
                    Build::Range(range) => {
                        let bit = difference(&keys[range.start], &keys[range.end - 1])
                            .ok_or(SegmentedError::Corrupt("duplicate effective source keys"))?;
                        let middle = range.start
                            + keys[range.clone()].partition_point(|key| !right(key, bit));
                        pending.try_push_admitted(Build::Join(bit), memory)?;
                        pending.try_push_admitted(Build::Range(middle..range.end), memory)?;
                        pending.try_push_admitted(Build::Range(range.start..middle), memory)?;
                    }
                    Build::Join(bit) => {
                        let right = complete.pop().expect("completed right subtree");
                        let left = complete.pop().expect("completed left subtree");
                        complete.try_push_admitted(branch(bit, left, right, operation)?, memory)?;
                    }
                }
            }
            Ok(())
        })?;
        let root = complete.pop();
        storage.run(|memory| {
            memory.release_vec(keys)?;
            pending.release_admitted(memory)?;
            complete.release_admitted(memory)
        })?;
        Ok(Self { root })
    }

    pub(super) fn set(
        &self,
        key: [u8; 32],
        present: bool,
        operation: &SegmentedOperation,
    ) -> Result<Self, SegmentedError> {
        let Some(root) = &self.root else {
            return Ok(Self {
                root: if present {
                    Some(leaf(&[key], operation)?)
                } else {
                    None
                },
            });
        };
        let mut storage = QueryStorage::new(operation)?;
        let mut path = WorkList::<_, 0>::new();
        let mut current = root.clone();
        let mut replacement;
        loop {
            if let Kind::Branch {
                bit,
                left,
                right: upper,
            } = &current.kind
            {
                if let Some(split) = difference(&key, &current.first)
                    && split < *bit
                {
                    if !present {
                        return Ok(self.clone());
                    }
                    let added = leaf(&[key], operation)?;
                    replacement = Some(if right(&key, split) {
                        branch(split, current, added, operation)?
                    } else {
                        branch(split, added, current, operation)?
                    });
                    break;
                }
                let go_right = right(&key, *bit);
                let child = if go_right {
                    upper.clone()
                } else {
                    left.clone()
                };
                storage.run(|memory| path.try_push_admitted((current, go_right), memory))?;
                current = child;
                continue;
            }
            let Kind::Leaf(keys) = &current.kind else {
                unreachable!("branch handled above");
            };
            let position = keys.binary_search(&key);
            if position.is_ok() == present {
                return Ok(self.clone());
            }
            let mut edited = [[0; 32]; LEAF_KEYS + 1];
            let count = match position {
                Ok(at) => {
                    edited[..at].copy_from_slice(&keys[..at]);
                    edited[at..keys.len() - 1].copy_from_slice(&keys[at + 1..]);
                    keys.len() - 1
                }
                Err(at) => {
                    edited[..at].copy_from_slice(&keys[..at]);
                    edited[at] = key;
                    edited[at + 1..=keys.len()].copy_from_slice(&keys[at..]);
                    keys.len() + 1
                }
            };
            replacement = if count == 0 {
                None
            } else if count <= LEAF_KEYS {
                Some(leaf(&edited[..count], operation)?)
            } else {
                let split =
                    difference(&edited[0], &edited[count - 1]).expect("distinct sorted keys");
                let middle = edited[..count].partition_point(|key| !right(key, split));
                Some(branch(
                    split,
                    leaf(&edited[..middle], operation)?,
                    leaf(&edited[middle..count], operation)?,
                    operation,
                )?)
            };
            break;
        }
        while let Some((parent, go_right)) = path.pop() {
            let Kind::Branch { bit, left, right } = &parent.kind else {
                unreachable!("path contains branches");
            };
            let sibling = if go_right { left } else { right };
            replacement = Some(match replacement {
                None => sibling.clone(),
                Some(child) if go_right => branch(*bit, sibling.clone(), child, operation)?,
                Some(child) => branch(*bit, child, sibling.clone(), operation)?,
            });
        }
        storage.run(|memory| path.release_admitted(memory))?;
        Ok(Self { root: replacement })
    }
}

impl DirectoryRead for EffectiveDirectory {
    type Error = SegmentedError;
    type Record = [u8; 32];

    fn len(&self) -> Result<usize, Self::Error> {
        usize::try_from(self.summary().population()).map_err(|_| SegmentedError::AddressExhausted)
    }

    fn record(&self, mut index: usize) -> Result<Self::Record, Self::Error> {
        let mut node = self
            .root
            .as_ref()
            .ok_or(SegmentedError::Corrupt("empty effective directory ordinal"))?;
        loop {
            match &node.kind {
                Kind::Leaf(keys) => {
                    return keys.get(index).copied().ok_or(SegmentedError::Corrupt(
                        "effective directory ordinal outside range",
                    ));
                }
                Kind::Branch { left, right, .. } => {
                    let size = usize::try_from(left.summary.population())
                        .map_err(|_| SegmentedError::AddressExhausted)?;
                    node = if index < size {
                        left
                    } else {
                        index -= size;
                        right
                    };
                }
            }
        }
    }

    fn prefix(
        &self,
        mut end: usize,
        work: &mut SourceDifferenceWork,
    ) -> Result<Multiset256, Self::Error> {
        if end > self.len()? {
            return Err(SegmentedError::Corrupt(
                "effective directory prefix outside range",
            ));
        }
        let mut sum = Multiset256::default();
        let mut node = self.root.as_ref();
        while end != 0 {
            let current = node.expect("nonempty prefix has a root");
            if end as u128 == current.summary.population() {
                sum.combine(current.summary)
                    .map_err(|_| SegmentedError::AddressExhausted)?;
                break;
            }
            match &current.kind {
                Kind::Leaf(keys) => {
                    for key in &keys[..end] {
                        sum.insert(*key)
                            .map_err(|_| SegmentedError::AddressExhausted)?;
                        work.members += 1;
                    }
                    break;
                }
                Kind::Branch { left, right, .. } => {
                    let size = usize::try_from(left.summary.population())
                        .map_err(|_| SegmentedError::AddressExhausted)?;
                    if end <= size {
                        node = Some(left);
                    } else {
                        sum.combine(left.summary)
                            .map_err(|_| SegmentedError::AddressExhausted)?;
                        end -= size;
                        node = Some(right);
                    }
                }
            }
        }
        Ok(sum)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn operation() -> SegmentedOperation {
        SegmentedOperation::new(crate::SegmentedReadLimits::new(
            256_000_000,
            8,
            100_000,
            256_000_000,
            32,
        ))
        .unwrap()
    }

    fn key(index: u64) -> [u8; 32] {
        *purrdf_hash::blake3::hash(&index.to_le_bytes()).as_bytes()
    }

    fn check(directory: &EffectiveDirectory, expected: &BTreeSet<[u8; 32]>) {
        assert_eq!(directory.len().unwrap(), expected.len());
        let mut sum = Multiset256::default();
        for (index, key) in expected.iter().enumerate() {
            assert_eq!(directory.record(index).unwrap(), *key);
            assert_eq!(
                directory
                    .prefix(index, &mut SourceDifferenceWork::default())
                    .unwrap(),
                sum
            );
            sum.insert(*key).unwrap();
        }
        assert_eq!(directory.summary(), sum);
        assert_eq!(
            directory
                .prefix(expected.len(), &mut SourceDifferenceWork::default())
                .unwrap(),
            sum
        );
    }

    #[test]
    fn persistent_effective_directory_matches_set_edits_and_old_snapshots() {
        let operation = operation();
        let mut directory = EffectiveDirectory::default();
        let mut expected = BTreeSet::new();
        let mut state = 683;
        for step in 0..4096 {
            let draw = purrdf_hash::mix::splitmix64_next(&mut state);
            let key = key(draw % 512);
            let present = draw & 512 == 0;
            let old = directory.clone();
            let before = expected.clone();
            directory = directory.set(key, present, &operation).unwrap();
            if present {
                expected.insert(key);
            } else {
                expected.remove(&key);
            }
            if step % 31 == 0 {
                check(&old, &before);
                check(&directory, &expected);
            }
        }
        check(&directory, &expected);
        for key in expected {
            directory = directory.set(key, false, &operation).unwrap();
        }
        assert!(directory.root.is_none());
    }

    #[test]
    fn effective_directory_fixed_edits_copy_bounded_keys_and_paths() {
        let operation = operation();
        let mut measurements = Vec::new();
        for count in [128, 4096, 65536] {
            let mut directory = EffectiveDirectory::default();
            for index in 0..count {
                directory = directory.set(key(index), true, &operation).unwrap();
            }
            BUILDS.set(0);
            COPIES.set(0);
            let old = directory.clone();
            assert_eq!((BUILDS.get(), COPIES.get()), (0, 0));
            let changed = directory
                .set(key(count / 2), false, &operation)
                .unwrap()
                .set(key(count + 1), true, &operation)
                .unwrap();
            let builds = BUILDS.get();
            let copies = COPIES.get();
            assert!(builds <= 64);
            assert!(copies <= 96);
            assert_eq!(old.len().unwrap(), count as usize);
            assert_eq!(changed.len().unwrap(), count as usize);
            assert!(old.find(key(count / 2)).unwrap().is_some());
            assert!(changed.find(key(count / 2)).unwrap().is_none());
            assert!(changed.find(key(count + 1)).unwrap().is_some());
            measurements.push((count, builds, copies));
        }
        eprintln!(
            "effective source directory edits (population, nodes, copied keys): {measurements:?}"
        );
    }
}
