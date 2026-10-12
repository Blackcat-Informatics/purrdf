// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One read owner and lazy value identities across source-local dictionaries.

use std::hash::Hasher as _;
use std::sync::{Arc, Mutex};

use hashbrown::HashTable;
use purrdf_lex::allocation::{Admission, Memory, StorageError};

use crate::dataset_view::lock_read_state;
use crate::hash::{hash_of, reserve_table_with_memory};
use crate::{
    DatasetView, GlobalTermId, RdfDataset, TermId, TermLookupError, TermValue, WorkspaceReservation,
};

use super::session::SegmentedOperation;
use super::{
    SegmentedError, SegmentedReadLimits, SegmentedReservation, SegmentedSession,
    SegmentedStackSnapshot,
};

mod view;
pub use view::SegmentedStackTermGuard;

/// A dictionary in the pinned chronological publication. Layer indexes are oldest
/// first. A missing metadata carrier is not an empty substitute for another source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SegmentedStackSource {
    /// A sealed layer's physical dictionary.
    Layer(usize),
    /// A sealed layer's normalized removals.
    Removals(usize),
    /// A sealed layer's explicit graph declarations.
    Declarations(usize),
    /// The frozen current batch.
    Head,
    /// The frozen current batch's removals.
    HeadRemovals,
    /// The frozen current batch's explicit declarations.
    HeadDeclarations,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct SourceTerm {
    source: SegmentedStackSource,
    index: u64,
}

#[derive(Clone, Copy, Debug)]
struct Representative {
    term: SourceTerm,
}

#[derive(Debug)]
struct Identities {
    representatives: Vec<Representative>,
    values: HashTable<(GlobalTermId, u64)>,
    aliases: HashTable<(SourceTerm, GlobalTermId)>,
    // Native buffers above die before their admission owner.
    storage: QueryStorage,
}

/// A materialized touched value whose original read admission outlives its payload.
/// This owner contains no other dictionary entries and can outlive the query view.
#[derive(Debug)]
pub struct SegmentedStackTerm {
    value: TermValue,
    _storage: QueryStorage,
}

impl SegmentedStackTerm {
    /// Borrow the exact scoped RDF value copied through the original term home.
    #[must_use]
    pub const fn value(&self) -> &TermValue {
        &self.value
    }
}

/// One operational read of a pinned publication. Only touched values receive
/// query IDs; no historical dictionary or translation array is rebuilt on open.
#[derive(Debug)]
pub struct SegmentedStackQuery<'a> {
    snapshot: &'a SegmentedStackSnapshot,
    sessions: Vec<SegmentedSession>,
    identities: Mutex<Identities>,
    _sessions_storage: QueryStorage,
    operation: SegmentedOperation,
}

impl SegmentedStackSnapshot {
    /// Open every certified header under one cumulative read owner.
    ///
    /// # Errors
    /// Preserves the first provider, identity, admission or allocation failure.
    pub fn query(
        &self,
        limits: SegmentedReadLimits,
    ) -> Result<SegmentedStackQuery<'_>, SegmentedError> {
        let operation = SegmentedOperation::new(limits)?;
        let mut storage = QueryStorage::new(&operation)?;
        let mut sessions = Vec::new();
        storage.run(|memory| memory.reserve(&mut sessions, self.layers.len()))?;
        for layer in self.layers.iter() {
            sessions.push(operation.open(Arc::clone(&layer.provider), &layer.receipt)?);
        }
        let identities = Identities {
            representatives: Vec::new(),
            values: HashTable::new(),
            aliases: HashTable::new(),
            storage: QueryStorage::new(&operation)?,
        };
        let query = SegmentedStackQuery {
            snapshot: self,
            sessions,
            identities: Mutex::new(identities),
            _sessions_storage: storage,
            operation,
        };
        query.check_ready()?;
        Ok(query)
    }
}

impl SegmentedStackQuery<'_> {
    /// Final readiness includes all descriptors, even sources never scanned or
    /// whose authenticated header has already left the sparse cache.
    ///
    /// # Errors
    /// Returns the operation's first cause, never an absent-fact substitute.
    pub fn check_ready(&self) -> Result<(), SegmentedError> {
        self.operation.ready()?;
        for layer in self.snapshot.layers.iter() {
            layer
                .check_descriptor()
                .or_else(|error| self.operation.fail(error))?;
        }
        Ok(())
    }

    /// Number of distinct RDF values actually touched by this operation.
    #[must_use]
    pub fn touched_terms(&self) -> usize {
        lock_read_state(&self.identities).representatives.len()
    }

    fn find_touched(&self, value: &TermValue) -> Result<Option<GlobalTermId>, SegmentedError> {
        let mut scratch = QueryStorage::new(&self.operation)?;
        let hash = scratch.hash(value)?;
        let ids = lock_read_state(&self.identities);
        for (id, _) in ids.values.iter_hash(hash) {
            let index =
                usize::try_from(id.index()).map_err(|_| SegmentedError::AddressExhausted)?;
            let candidate = self.source_value(ids.representatives[index].term)?;
            if scratch
                .run(|memory| candidate.value.try_cmp_with_memory(value, memory))?
                .is_eq()
            {
                let found = *id;
                drop(ids);
                return Ok(Some(found));
            }
        }
        drop(ids);
        Ok(None)
    }

    /// Import a source-local zero-based term index into this operation's value
    /// identity. Equal values share one ID; equal local indexes alone never do.
    ///
    /// # Errors
    /// Refuses a foreign source/index or actual read/storage failure. Hash bucket
    /// collisions compare exact values while their original scratch remains owned.
    pub fn intern_source_term(
        &self,
        source: SegmentedStackSource,
        index: u64,
    ) -> Result<GlobalTermId, SegmentedError> {
        self.operation.ready()?;
        self.intern(SourceTerm { source, index })
            .or_else(|error| self.operation.fail(error))
    }

    fn intern(&self, term: SourceTerm) -> Result<GlobalTermId, SegmentedError> {
        let mut ids = lock_read_state(&self.identities);
        let alias_hash = hash_of(&term);
        if let Some((_, id)) = ids.aliases.find(alias_hash, |(key, _)| *key == term) {
            return Ok(*id);
        }
        let value = self.source_value(term)?;
        let mut scratch = QueryStorage::new(&self.operation)?;
        let hash = scratch.hash(&value.value)?;
        let mut existing = None;
        for (id, _) in ids.values.iter_hash(hash) {
            let representative = ids.representatives
                [usize::try_from(id.index()).map_err(|_| SegmentedError::AddressExhausted)?]
            .term;
            let candidate = self.source_value(representative)?;
            if scratch
                .run(|memory| candidate.value.try_cmp_with_memory(&value.value, memory))?
                .is_eq()
            {
                existing = Some(*id);
                break;
            }
        }
        let Identities {
            representatives,
            values,
            aliases,
            storage,
        } = &mut *ids;
        let id = if let Some(id) = existing {
            id
        } else {
            let index = u64::try_from(representatives.len())
                .map_err(|_| SegmentedError::AddressExhausted)?;
            GlobalTermId::from_index(index).map_err(StorageError::from)?
        };
        storage.run(|memory| {
            let next_alias = aliases
                .len()
                .checked_add(1)
                .ok_or(StorageError::SizeOverflow)?;
            reserve_table_with_memory(aliases, next_alias, |(key, _)| hash_of(key), memory)?;
            if existing.is_none() {
                let next = representatives
                    .len()
                    .checked_add(1)
                    .ok_or(StorageError::SizeOverflow)?;
                memory.reserve(representatives, next)?;
                reserve_table_with_memory(values, next, |id| id.1, memory)?;
            }
            Ok::<_, StorageError>(())
        })?;
        if existing.is_none() {
            representatives.push(Representative { term });
            values.insert_unique(hash, (id, hash), |id| id.1);
        }
        aliases.insert_unique(alias_hash, (term, id), |(key, _)| hash_of(key));
        drop(ids);
        Ok(id)
    }

    /// Materialize only this touched query value, retaining its original admission
    /// until the returned owner is dropped, including after the query itself.
    ///
    /// # Errors
    /// Refuses an ID not minted here or any underlying operational failure.
    pub fn term_value(&self, id: GlobalTermId) -> Result<SegmentedStackTerm, SegmentedError> {
        self.operation.ready()?;
        let term = {
            let ids = lock_read_state(&self.identities);
            usize::try_from(id.index())
                .ok()
                .and_then(|index| ids.representatives.get(index))
                .map(|entry| entry.term)
        }
        .ok_or(SegmentedError::Corrupt("foreign query term ID"));
        term.and_then(|term| self.source_value(term))
            .or_else(|error| self.operation.fail(error))
    }

    fn resident(&self, source: SegmentedStackSource) -> Option<&RdfDataset> {
        match source {
            SegmentedStackSource::Layer(_) => None,
            SegmentedStackSource::Removals(index) => self
                .snapshot
                .layers
                .get(index)?
                .metadata
                .removals()
                .map(Arc::as_ref),
            SegmentedStackSource::Declarations(index) => self
                .snapshot
                .layers
                .get(index)?
                .metadata
                .declarations()
                .map(Arc::as_ref),
            SegmentedStackSource::Head => self
                .snapshot
                .head
                .as_ref()
                .map(|head| head.dataset.as_ref()),
            SegmentedStackSource::HeadRemovals => self
                .snapshot
                .head
                .as_ref()?
                .metadata
                .removals()
                .map(Arc::as_ref),
            SegmentedStackSource::HeadDeclarations => self
                .snapshot
                .head
                .as_ref()?
                .metadata
                .declarations()
                .map(Arc::as_ref),
        }
    }

    fn source_value(&self, term: SourceTerm) -> Result<SegmentedStackTerm, SegmentedError> {
        let mut storage = QueryStorage::new(&self.operation)?;
        let value = storage.run(|memory| {
            if let SegmentedStackSource::Layer(index) = term.source {
                let source = self
                    .sessions
                    .get(index)
                    .ok_or(SegmentedError::Corrupt("foreign layer index"))?;
                let id = GlobalTermId::checked_from_index(term.index)
                    .ok_or(SegmentedError::AddressExhausted)?;
                source.term_value_with_memory(
                    id,
                    memory,
                    |error| match error {
                        TermLookupError::ForeignId => {
                            SegmentedError::Corrupt("foreign source term ID")
                        }
                        TermLookupError::Read(error) => error,
                    },
                    |error, _| error.into(),
                )
            } else {
                let source = self
                    .resident(term.source)
                    .ok_or(SegmentedError::Corrupt("missing resident source"))?;
                let index =
                    u32::try_from(term.index).map_err(|_| SegmentedError::AddressExhausted)?;
                let id = TermId::from_index(index).map_err(StorageError::from)?;
                source.term_value_with_memory(
                    id,
                    memory,
                    |error| match error {
                        TermLookupError::ForeignId => {
                            SegmentedError::Corrupt("foreign resident term ID")
                        }
                        TermLookupError::Read(error) => match error {},
                    },
                    |error, _| error.into(),
                )
            }
        })?;
        Ok(SegmentedStackTerm {
            value,
            _storage: storage,
        })
    }
}

#[derive(Debug)]
struct QueryStorage {
    reservation: SegmentedReservation,
    cause: Option<SegmentedError>,
}

impl QueryStorage {
    fn hash(&mut self, value: &TermValue) -> Result<u64, SegmentedError> {
        self.run(|memory| {
            let mut hash = purrdf_hash::fixed::FixedHasher::default();
            value.try_hash_with_memory(&mut hash, memory)?;
            Ok::<_, StorageError>(hash.finish())
        })
    }

    fn new(operation: &SegmentedOperation) -> Result<Self, SegmentedError> {
        Ok(Self {
            reservation: operation.reserve_workspace(0)?,
            cause: None,
        })
    }

    fn run<T, E: Into<SegmentedError>>(
        &mut self,
        body: impl FnOnce(&mut Memory<'_, Self>) -> Result<T, E>,
    ) -> Result<T, SegmentedError> {
        let live = usize::try_from(self.reservation.bytes())
            .map_err(|_| SegmentedError::AddressExhausted)?;
        let result = body(&mut Memory::resume(self, live));
        result.map_err(|error| self.cause.take().unwrap_or_else(|| error.into()))
    }
}

impl Admission for QueryStorage {
    fn resize(&mut self, live: usize) -> Result<(), StorageError> {
        let live = u64::try_from(live).map_err(|_| StorageError::SizeOverflow)?;
        self.reservation.resize(live).map_err(|error| {
            self.cause.get_or_insert(error);
            StorageError::AdmissionFailed
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TermFactory;
    use crate::{RdfDatasetBuilder, SegmentedLayerMetadata, SegmentedResidentHead, SegmentedStack};

    #[test]
    fn returned_term_retains_original_base_and_payload_charge_after_query_drop() {
        let mut head = RdfDatasetBuilder::new();
        let id = head
            .intern_value(&TermValue::iri("urn:retained-value"))
            .unwrap();
        let mut stack = SegmentedStack::default();
        stack.set_head(Some(
            SegmentedResidentHead::new(head.freeze().unwrap(), SegmentedLayerMetadata::default())
                .unwrap(),
        ));
        let snapshot = stack.snapshot().unwrap();
        let query = snapshot
            .query(SegmentedReadLimits::new(100_000, 1, 32, 100_000, 2))
            .unwrap();
        let id = query
            .intern_source_term(
                SegmentedStackSource::Head,
                u64::try_from(id.index()).unwrap(),
            )
            .unwrap();
        let term = query.term_value(id).unwrap();
        let base = Arc::downgrade(term._storage.reservation.base_owner());
        let payload_bytes = term._storage.reservation.bytes();
        let base_bytes = term._storage.reservation.base_owner().bytes();
        assert!(payload_bytes > 0);
        drop(query);
        assert_eq!(
            term._storage.reservation.live_bytes(),
            base_bytes + payload_bytes
        );
        assert_eq!(term.value().as_iri(), Some("urn:retained-value"));
        drop(term);
        assert!(base.upgrade().is_none());
    }
}
