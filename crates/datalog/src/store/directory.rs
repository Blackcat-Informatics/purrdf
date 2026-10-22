// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Canonical partition traversal with removable original identities.

use super::{
    Arity, DictionaryError, DictionaryTerm, PartitionId, StoreKey, StoreShape, TermDictionary,
    TermIdentity,
};
use crate::allocation::{flush_retired_bytes, reserve_index_in, reserve_ordered_in};
use purrdf_core::FastMap;
use purrdf_lex::allocation::{
    Admission, Memory, OrderedEntry, OrderedSlot, OrderedTable, Resident, StorageError,
};
use std::cmp::Ordering;

type Ordered<K> = OrderedTable<K, (PartitionId, usize)>;

#[derive(Debug, Default)]
pub(super) struct SupplementalBytes {
    predicates: usize,
    locations: usize,
    retired: usize,
}

#[derive(Debug)]
pub(super) struct Preparation<I: TermIdentity, K: Copy> {
    pub(super) directory: Directory<I, K>,
    pub(super) next: usize,
}

impl<I: TermIdentity, K: Copy> Default for Preparation<I, K> {
    fn default() -> Self {
        Self {
            directory: Directory::sparse(true),
            next: 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Located {
    slot: usize,
    order: OrderedSlot,
    predicate: OrderedSlot,
}

#[derive(Debug)]
pub(super) enum Directory<I: TermIdentity, K: Copy> {
    Dense {
        order: Vec<usize>,
        predicates: FastMap<I, Vec<usize>>,
    },
    Sparse {
        order: Ordered<K>,
        predicates: FastMap<I, Ordered<K>>,
        locations: FastMap<PartitionId, Located>,
        spare: Ordered<K>,
        bytes: usize,
        admitted: Option<SupplementalBytes>,
    },
}

impl<I: TermIdentity, K: Copy> Default for Directory<I, K> {
    fn default() -> Self {
        Self::Dense {
            order: Vec::new(),
            predicates: FastMap::default(),
        }
    }
}

impl<I: TermIdentity, K: Copy> Clone for Directory<I, K> {
    fn clone(&self) -> Self {
        match self {
            Self::Dense { order, predicates } => Self::Dense {
                order: order.clone(),
                predicates: predicates.clone(),
            },
            Self::Sparse {
                order,
                predicates,
                locations,
                spare,
                ..
            } => {
                let mut resident = Resident;
                let mut memory = Memory::new(&mut resident);
                let order = order
                    .try_clone(&mut memory)
                    .expect("cloning an ordered directory");
                let spare = spare
                    .try_clone(&mut memory)
                    .expect("cloning a spare directory");
                let predicates = predicates
                    .iter()
                    .map(|(&id, tree)| {
                        (
                            id,
                            tree.try_clone(&mut memory)
                                .expect("cloning a predicate directory"),
                        )
                    })
                    .collect();
                Self::Sparse {
                    order,
                    predicates,
                    locations: locations.clone(),
                    spare,
                    bytes: memory.admitted_bytes(),
                    admitted: None,
                }
            }
        }
    }
}

#[derive(Debug)]
pub(super) enum Position {
    Dense(usize),
    Sparse {
        order: OrderedSlot,
        predicate: OrderedSlot,
        new_predicate: bool,
    },
}

pub(super) enum Prepared<'a, K> {
    Dense(usize),
    Sparse {
        order: OrderedEntry<'a, K, (PartitionId, usize)>,
        predicate: OrderedEntry<'a, K, (PartitionId, usize)>,
        new_predicate: bool,
    },
}

impl<K> Prepared<'_, K> {
    pub(super) fn publish(self, id: PartitionId, slot: usize) -> Position {
        match self {
            Self::Dense(position) => Position::Dense(position),
            Self::Sparse {
                order,
                predicate,
                new_predicate,
            } => {
                let (order, old_order) = order.publish((id, slot));
                let (predicate, old_predicate) = predicate.publish((id, slot));
                assert!(old_order.is_none() && old_predicate.is_none());
                Position::Sparse {
                    order,
                    predicate,
                    new_predicate,
                }
            }
        }
    }
}

impl<I: TermIdentity, K: Copy> Directory<I, K> {
    pub(super) const fn is_sparse(&self) -> bool {
        matches!(self, Self::Sparse { .. })
    }

    pub(super) fn dense_order(&self) -> &[usize] {
        let Self::Dense { order, .. } = self else {
            panic!("dense traversal requires a dense directory");
        };
        order
    }

    pub(super) fn dense_predicate(&self, predicate: I) -> &[usize] {
        let Self::Dense { predicates, .. } = self else {
            panic!("dense traversal requires a dense directory");
        };
        predicates.get(&predicate).map_or(&[], Vec::as_slice)
    }

    pub(super) fn slot(&self, id: PartitionId) -> usize {
        match self {
            Self::Dense { .. } => id.resident_index(),
            Self::Sparse { locations, .. } => {
                locations
                    .get(&id)
                    .expect("a retained partition cursor owns its original live identity")
                    .slot
            }
        }
    }

    pub(super) fn after(
        &self,
        predicate: Option<I>,
        previous: Option<PartitionId>,
    ) -> Option<usize> {
        let Self::Sparse {
            order,
            predicates,
            locations,
            ..
        } = self
        else {
            panic!("key traversal requires a sparse directory");
        };
        let tree = if let Some(predicate) = predicate {
            predicates.get(&predicate)?
        } else {
            order
        };
        let next = if let Some(previous) = previous {
            let located = locations
                .get(&previous)
                .expect("the immutable scan retains its preceding key");
            let at = if predicate.is_some() {
                located.predicate
            } else {
                located.order
            };
            assert_eq!(
                tree.at_slot(at).expect("a live directory locator").1.0,
                previous
            );
            tree.next_slot(at)
        } else {
            tree.first_slot()
        };
        next.map(|at| tree.at_slot(at).expect("a live directory successor").1.1)
    }

    pub(super) fn admit(
        &mut self,
        predicate: Option<I>,
    ) -> Result<Option<Vec<usize>>, StorageError> {
        assert!(
            !self.is_supplemental(),
            "supplemental directory growth lends its original memory"
        );
        let mut bucket = None;
        match self {
            Self::Dense { order, predicates } => {
                order
                    .try_reserve(1)
                    .map_err(|_| StorageError::AllocationFailed)?;
                if let Some(slots) = predicate.and_then(|id| predicates.get_mut(&id)) {
                    slots
                        .try_reserve(1)
                        .map_err(|_| StorageError::AllocationFailed)?;
                } else {
                    predicates
                        .try_reserve(1)
                        .map_err(|_| StorageError::AllocationFailed)?;
                    let mut slots = Vec::new();
                    slots
                        .try_reserve(1)
                        .map_err(|_| StorageError::AllocationFailed)?;
                    bucket = Some(slots);
                }
            }
            Self::Sparse {
                predicates,
                locations,
                ..
            } => {
                predicates
                    .try_reserve(1)
                    .map_err(|_| StorageError::AllocationFailed)?;
                locations
                    .try_reserve(1)
                    .map_err(|_| StorageError::AllocationFailed)?;
            }
        }
        Ok(bucket)
    }

    pub(super) const fn is_supplemental(&self) -> bool {
        matches!(
            self,
            Self::Sparse {
                admitted: Some(_),
                ..
            }
        )
    }

    pub(super) fn flush_retired_in<A: Admission + ?Sized>(
        &mut self,
        memory: &mut Memory<'_, A>,
    ) -> Result<(), StorageError> {
        if let Self::Sparse {
            admitted: Some(admitted),
            ..
        } = self
        {
            flush_retired_bytes(&mut admitted.retired, memory)?;
        }
        Ok(())
    }

    pub(super) fn admit_in<A: Admission + ?Sized>(
        &mut self,
        predicate: Option<I>,
        memory: &mut Memory<'_, A>,
    ) -> Result<Option<Vec<usize>>, StorageError> {
        let Self::Sparse {
            order,
            predicates,
            locations,
            spare,
            admitted: Some(admitted),
            ..
        } = self
        else {
            return self.admit(predicate);
        };
        flush_retired_bytes(&mut admitted.retired, memory)?;
        reserve_index_in(
            predicates,
            &mut admitted.predicates,
            1,
            memory,
            &mut admitted.retired,
        )?;
        reserve_index_in(
            locations,
            &mut admitted.locations,
            1,
            memory,
            &mut admitted.retired,
        )?;
        reserve_ordered_in(order, memory, &mut admitted.retired)?;
        let tree = predicate
            .and_then(|id| predicates.get_mut(&id))
            .unwrap_or(spare);
        reserve_ordered_in(tree, memory, &mut admitted.retired)?;
        Ok(None)
    }

    pub(super) fn publish(
        &mut self,
        slot: usize,
        id: PartitionId,
        predicate: I,
        position: Position,
        bucket: Option<Vec<usize>>,
        predicate_at: impl Fn(usize) -> I,
    ) {
        match (self, position) {
            (Self::Dense { order, predicates }, Position::Dense(position)) => {
                let previous = position
                    .checked_sub(1)
                    .and_then(|i| order.get(i))
                    .copied()
                    .filter(|&slot| predicate_at(slot) == predicate);
                order.insert(position, slot);
                let slots = predicates
                    .entry(predicate)
                    .or_insert_with(|| bucket.expect("new predicate bucket was admitted"));
                let position = previous.map_or(0, |previous| {
                    if slots.last() == Some(&previous) {
                        slots.len()
                    } else {
                        slots
                            .iter()
                            .position(|&slot| slot == previous)
                            .expect("preceding predicate partition")
                            + 1
                    }
                });
                slots.insert(position, slot);
            }
            (
                Self::Sparse {
                    predicates,
                    locations,
                    spare,
                    ..
                },
                Position::Sparse {
                    order,
                    predicate: predicate_slot,
                    new_predicate,
                },
            ) => {
                if new_predicate {
                    assert!(
                        predicates
                            .insert(predicate, core::mem::take(spare))
                            .is_none()
                    );
                }
                assert!(
                    locations
                        .insert(
                            id,
                            Located {
                                slot,
                                order,
                                predicate: predicate_slot
                            }
                        )
                        .is_none(),
                    "partition identities never alias a retired key"
                );
            }
            _ => unreachable!("publication retains its original directory representation"),
        }
    }

    pub(super) fn remove(&mut self, id: PartitionId, predicate: I) -> usize {
        let Self::Sparse {
            order,
            predicates,
            locations,
            spare,
            bytes,
            admitted,
        } = self
        else {
            panic!("only maintained directories retire keys");
        };
        let Located {
            slot,
            order: order_slot,
            predicate: predicate_slot,
        } = locations
            .remove(&id)
            .expect("retiring an original live key");
        assert_eq!(
            order.remove_slot(order_slot).expect("live global key").1,
            (id, slot)
        );
        let tree = predicates
            .get_mut(&predicate)
            .expect("live predicate directory");
        assert_eq!(
            tree.remove_slot(predicate_slot)
                .expect("live predicate key")
                .1,
            (id, slot)
        );
        if tree.is_empty() {
            let tree = predicates
                .remove(&predicate)
                .expect("empty predicate arena");
            // Keep one reusable arena owned by the directory. Retiring another
            // empty arena destroys its payload before returning its exact grant.
            let retired = core::mem::replace(spare, tree);
            if let Some(admitted) = admitted {
                let released = retired
                    .storage_bytes()
                    .expect("the original directory arena layout");
                drop(retired);
                admitted.retired = admitted
                    .retired
                    .checked_add(released)
                    .expect("retired directory storage fits its original grant");
            } else {
                let mut resident = Resident;
                let mut memory = Memory::resume(&mut resident, *bytes);
                retired
                    .release(&mut memory)
                    .expect("releasing original directory storage");
                *bytes = memory.admitted_bytes();
            }
        }
        slot
    }

    fn sparse(supplemental: bool) -> Self {
        Self::Sparse {
            order: OrderedTable::default(),
            predicates: FastMap::default(),
            locations: FastMap::default(),
            spare: OrderedTable::default(),
            bytes: 0,
            admitted: supplemental.then(SupplementalBytes::default),
        }
    }

    /// Resident test stores exercise the public insertion boundary after setup.
    #[cfg(test)]
    pub(super) fn resident_from_live<D: TermDictionary<Id = I>, const N: usize>(
        dictionary: &D,
        entries: impl IntoIterator<Item = (usize, PartitionId, StoreKey<I, N>)>,
    ) -> Result<Self, DictionaryError<D::Error>>
    where
        Arity<N>: StoreShape<N, Key<I> = K>,
    {
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let mut directory = Self::sparse(false);
        for entry in entries {
            directory.insert_live_in::<D, N, _>(dictionary, entry, &mut memory)?;
        }
        dictionary.check_read().map_err(DictionaryError::Source)?;
        Ok(directory)
    }

    pub(super) fn insert_live_in<D: TermDictionary<Id = I>, const N: usize, A: Admission + ?Sized>(
        &mut self,
        dictionary: &D,
        entry: (usize, PartitionId, StoreKey<I, N>),
        memory: &mut Memory<'_, A>,
    ) -> Result<(), DictionaryError<D::Error>>
    where
        Arity<N>: StoreShape<N, Key<I> = K>,
    {
        let (slot, id, key) = entry;
        let predicate = Arity::<N>::key_term(key, 0);
        self.admit_in(Some(predicate), memory)?;
        let prepared = self.prepare_sparse_in::<D, N, A>(dictionary, key, memory)?;
        let position = prepared.publish(id, slot);
        self.publish(slot, id, predicate, position, None, |_| {
            unreachable!("sparse publication reads its retained key")
        });
        Ok(())
    }

    pub(super) fn prepare_sparse<D: TermDictionary<Id = I>, const N: usize>(
        &mut self,
        dictionary: &D,
        key: K,
    ) -> Result<Prepared<'_, K>, DictionaryError<D::Error>>
    where
        Arity<N>: StoreShape<N, Key<I> = K>,
    {
        assert!(
            !self.is_supplemental(),
            "supplemental preparation lends its original memory"
        );
        let Self::Sparse { bytes, .. } = self else {
            panic!("original sparse directory")
        };
        let bytes = *bytes;
        self.prepare_sparse_with::<D, N, _>(
            dictionary,
            key,
            &mut Memory::resume(&mut Resident, bytes),
        )
    }

    pub(super) fn prepare_sparse_in<
        D: TermDictionary<Id = I>,
        const N: usize,
        A: Admission + ?Sized,
    >(
        &mut self,
        dictionary: &D,
        key: K,
        memory: &mut Memory<'_, A>,
    ) -> Result<Prepared<'_, K>, DictionaryError<D::Error>>
    where
        Arity<N>: StoreShape<N, Key<I> = K>,
    {
        if self.is_supplemental() {
            self.prepare_sparse_with::<D, N, A>(dictionary, key, memory)
        } else {
            self.prepare_sparse::<D, N>(dictionary, key)
        }
    }

    fn prepare_sparse_with<D: TermDictionary<Id = I>, const N: usize, A: Admission + ?Sized>(
        &mut self,
        dictionary: &D,
        key: K,
        memory: &mut Memory<'_, A>,
    ) -> Result<Prepared<'_, K>, DictionaryError<D::Error>>
    where
        Arity<N>: StoreShape<N, Key<I> = K>,
    {
        let Self::Sparse {
            order,
            predicates,
            spare,
            bytes,
            admitted,
            ..
        } = self
        else {
            panic!("sparse preparation requires its original directory");
        };
        // An empty arena performs no search comparison. Validate every original
        // key coordinate even for its first entry, through the dictionary's
        // own ordering protocol (which also supports lexical-free native IDs).
        for coordinate in 0..Arity::<N>::KEY_POSITIONS.len() {
            let term = DictionaryTerm::Id(Arity::<N>::key_term(key, coordinate));
            let ordering = dictionary
                .compare_terms(term, term)
                .map_err(DictionaryError::Source)?;
            assert!(
                ordering.is_eq(),
                "a dictionary key compares equal to itself"
            );
        }
        let predicate_id = Arity::<N>::key_term(key, 0);
        let new_predicate = !predicates.contains_key(&predicate_id);
        let tree = predicates.get_mut(&predicate_id).unwrap_or(spare);
        let mut compare = |a: &K, b: &K| -> Result<Ordering, DictionaryError<D::Error>> {
            for coordinate in 0..Arity::<N>::KEY_POSITIONS.len() {
                let ordering = dictionary
                    .compare_terms(
                        DictionaryTerm::Id(Arity::<N>::key_term(*a, coordinate)),
                        DictionaryTerm::Id(Arity::<N>::key_term(*b, coordinate)),
                    )
                    .map_err(DictionaryError::Source)?;
                if !ordering.is_eq() {
                    return Ok(ordering);
                }
            }
            Ok(Ordering::Equal)
        };
        let before = memory.admitted_bytes();
        let order = order.prepare_insert(key, memory, &mut compare);
        if admitted.is_none() {
            *bytes = memory.admitted_bytes();
        } else {
            assert_eq!(
                memory.admitted_bytes(),
                before,
                "directory preparation was pre-admitted"
            );
        }
        let order = order?;
        let predicate = tree.prepare_insert(key, memory, &mut compare);
        if admitted.is_none() {
            *bytes = memory.admitted_bytes();
        } else {
            assert_eq!(
                memory.admitted_bytes(),
                before,
                "predicate preparation was pre-admitted"
            );
        }
        let predicate = predicate?;
        assert!(
            !order.is_occupied() && !predicate.is_occupied(),
            "new canonical partition"
        );
        dictionary.check_read().map_err(DictionaryError::Source)?;
        Ok(Prepared::Sparse {
            order,
            predicate,
            new_predicate,
        })
    }
}
