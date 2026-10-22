// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Canonical partition traversal with removable original identities.

use super::{
    Arity, DictionaryError, PartitionId, StoreKey, StoreShape, TermDictionary, TermIdentity,
};
use purrdf_core::FastMap;
use purrdf_lex::allocation::StorageError;
use std::{collections::BTreeMap, ops::Bound, sync::Arc};

pub(super) type OwnedKey<K> = Arc<[Option<K>; 2]>;
type Ordered<K> = BTreeMap<OwnedKey<K>, usize>;

#[derive(Debug)]
pub(super) struct Located<K> {
    slot: usize,
    key: OwnedKey<K>,
}

impl<K> Clone for Located<K> {
    fn clone(&self) -> Self {
        Self {
            slot: self.slot,
            key: Arc::clone(&self.key),
        }
    }
}

#[derive(Debug)]
pub(super) enum Directory<I: TermIdentity, K: Ord> {
    Dense {
        order: Vec<usize>,
        predicates: FastMap<I, Vec<usize>>,
    },
    Sparse {
        order: Ordered<K>,
        predicates: FastMap<I, Ordered<K>>,
        locations: FastMap<PartitionId, Located<K>>,
    },
}

impl<I: TermIdentity, K: Ord> Default for Directory<I, K> {
    fn default() -> Self {
        Self::Dense {
            order: Vec::new(),
            predicates: FastMap::default(),
        }
    }
}

impl<I: TermIdentity, K: Ord> Clone for Directory<I, K> {
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
            } => Self::Sparse {
                order: order.clone(),
                predicates: predicates.clone(),
                locations: locations.clone(),
            },
        }
    }
}

#[derive(Debug)]
pub(super) enum Position<K> {
    Dense(usize),
    Sparse(OwnedKey<K>),
}

impl<I: TermIdentity, K: Ord> Directory<I, K> {
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
            let key = &locations
                .get(&previous)
                .expect("the immutable scan retains its preceding key")
                .key;
            tree.range::<OwnedKey<K>, _>((Bound::Excluded(key), Bound::Unbounded))
                .next()
        } else {
            tree.first_key_value()
        };
        next.map(|(_, &slot)| slot)
    }

    pub(super) fn admit(
        &mut self,
        predicate: Option<I>,
    ) -> Result<Option<Vec<usize>>, StorageError> {
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

    pub(super) fn publish(
        &mut self,
        slot: usize,
        id: PartitionId,
        predicate: I,
        position: Position<K>,
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
                    order,
                    predicates,
                    locations,
                },
                Position::Sparse(key),
            ) => {
                assert!(
                    order.insert(Arc::clone(&key), slot).is_none(),
                    "canonical partition keys are identity-consistent"
                );
                assert!(
                    predicates
                        .entry(predicate)
                        .or_default()
                        .insert(Arc::clone(&key), slot)
                        .is_none()
                );
                assert!(
                    locations.insert(id, Located { slot, key }).is_none(),
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
        } = self
        else {
            panic!("only maintained directories retire keys");
        };
        let Located { slot, key } = locations
            .remove(&id)
            .expect("retiring an original live key");
        assert_eq!(order.remove(&key), Some(slot));
        let tree = predicates
            .get_mut(&predicate)
            .expect("live predicate directory");
        assert_eq!(tree.remove(&key), Some(slot));
        if tree.is_empty() {
            predicates.remove(&predicate);
        }
        slot
    }

    pub(super) fn from_live<D: TermDictionary<Id = I, OrderKey = K>, const N: usize>(
        dictionary: &D,
        entries: impl IntoIterator<Item = (usize, PartitionId, StoreKey<I, N>)>,
    ) -> Result<Self, DictionaryError<D::Error>>
    where
        Arity<N>: StoreShape<N>,
    {
        let mut directory = Self::Sparse {
            order: BTreeMap::new(),
            predicates: FastMap::default(),
            locations: FastMap::default(),
        };
        for (slot, id, key) in entries {
            let order = retained_key::<D, N>(dictionary, key).map_err(DictionaryError::Source)?;
            let predicate = Arity::<N>::key_term(key, 0);
            directory.admit(Some(predicate))?;
            directory.publish(slot, id, predicate, Position::Sparse(order), None, |_| {
                unreachable!("sparse publication reads its retained key")
            });
        }
        dictionary.check_read().map_err(DictionaryError::Source)?;
        Ok(directory)
    }
}

pub(super) fn retained_key<D: TermDictionary, const N: usize>(
    dictionary: &D,
    key: StoreKey<D::Id, N>,
) -> Result<OwnedKey<D::OrderKey>, D::Error>
where
    Arity<N>: StoreShape<N>,
{
    dictionary.check_read()?;
    let mut values = core::array::from_fn(|_| None);
    for (coordinate, value) in values
        .iter_mut()
        .enumerate()
        .take(Arity::<N>::KEY_POSITIONS.len())
    {
        *value = Some(dictionary.order_key(Arity::<N>::key_term(key, coordinate))?);
    }
    dictionary.check_read()?;
    Ok(Arc::new(values))
}
