// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Sparse, authenticated block admission with exact bounded evidence and live pins.

use crate::dataset_view::lock_read_state;
use crate::hash::{
    hash_of, hash_table_allocation_bound, hash_table_retained_bound, reserve_table_with_memory,
};
use hashbrown::HashTable;
use std::sync::{Arc, Mutex, OnceLock};

use crate::dataset_view::{
    DatasetView, FallibleDatasetView, GraphMatch, TermGuard, ViewOperationStatus,
};
use crate::{GlobalTermId, QuadIds, RdfStoreCapabilities, TermRef, TermValue};
use purrdf_hash::blake3::Hasher;
use purrdf_lex::allocation::StorageError;
use purrdf_lex::allocation::shared::{Shared, SharedCloneError};

use super::build::fixed_records;
use super::format::{self, DecodedRecords, Header, Reader, Stream};
use super::{SegmentedError, SegmentedProvider, SegmentedReceipt, SegmentedSnapshot};

mod cache;
#[cfg(test)]
mod cache_control;
mod statistics;

/// Read an identifier ordinal without allocating a decoded reverse dictionary.
fn reverse_id(ids: &[u8], index: u64) -> Result<GlobalTermId, SegmentedError> {
    let offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(8))
        .ok_or(SegmentedError::AddressExhausted)?;
    Reader::at(ids, offset).id()
}

// Both owners reserve through the same admission home after their own readiness
// check; a session additionally verifies its provider's snapshot identity.
macro_rules! reserve_workspace_method {
    ($(#[$meta:meta])* $visibility:vis) => {
        $(#[$meta])*
        $visibility fn reserve_workspace(
            &self,
            bytes: u64,
        ) -> Result<SegmentedReservation, SegmentedError> {
            self.ready()?;
            reserve_workspace(&self.state, &self.budget, &self.base, bytes)
        }
    };
}

/// Caller-selected inclusive ceilings for one operational read session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentedReadLimits {
    live_bytes: u64,
    cache_blocks: u32,
    requests: u32,
    io_bytes: u64,
    pins: u32,
}
impl SegmentedReadLimits {
    /// Set live bytes, sparse cache slots, exact request capacity, cumulative I/O
    /// bytes and simultaneously outstanding term guards. Zero is a hard limit.
    #[must_use]
    pub const fn new(
        live_bytes: u64,
        cache_blocks: u32,
        requests: u32,
        io_bytes: u64,
        pins: u32,
    ) -> Self {
        Self {
            live_bytes,
            cache_blocks,
            requests,
            io_bytes,
            pins,
        }
    }
    /// Inclusive live-byte ceiling shared by cache, pins, metadata, scratch and evidence.
    #[must_use]
    pub const fn live_bytes(self) -> u64 {
        self.live_bytes
    }
    /// Capacity of the exact, never-truncated request ledger.
    #[must_use]
    pub const fn requests(self) -> u32 {
        self.requests
    }
}

/// One exact host read, retained even when the provider refuses it.
pub struct SegmentedRequest {
    snapshot: Shared<RequestIdentity>,
    position: u64,
    bytes: u32,
    completed: bool,
}
#[derive(Debug)]
struct RequestIdentity {
    snapshot: SegmentedSnapshot,
    _reservation: SegmentedReservation,
}
impl std::fmt::Debug for SegmentedRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SegmentedRequest")
            .field("snapshot", &self.snapshot.snapshot)
            .field("position", &self.position)
            .field("bytes", &self.bytes)
            .field("completed", &self.completed)
            .finish()
    }
}
impl PartialEq for SegmentedRequest {
    fn eq(&self, other: &Self) -> bool {
        self.snapshot.snapshot == other.snapshot.snapshot
            && self.position == other.position
            && self.bytes == other.bytes
            && self.completed == other.completed
    }
}
impl Eq for SegmentedRequest {}
impl SegmentedRequest {
    /// Retain an exact request without allocating or copying its shared identity.
    ///
    /// # Errors
    /// Refuses checked shared-owner count overflow before retaining the request.
    pub fn try_clone(&self) -> Result<Self, SharedCloneError> {
        Ok(Self {
            snapshot: self.snapshot.try_clone()?,
            position: self.position,
            bytes: self.bytes,
            completed: self.completed,
        })
    }
    /// Certified source identity qualifying this provider-local byte address.
    #[must_use]
    pub fn snapshot(&self) -> SegmentedSnapshot {
        self.snapshot.snapshot
    }
    /// Global encoded byte position, independent of pointer width.
    #[must_use]
    pub const fn position(&self) -> u64 {
        self.position
    }
    /// Requested local range length.
    #[must_use]
    pub const fn bytes(&self) -> u32 {
        self.bytes
    }
    /// Whether the provider filled that range successfully.
    #[must_use]
    pub const fn completed(&self) -> bool {
        self.completed
    }
}

#[derive(Debug, Default)]
struct Residency {
    live: u64,
    peak: u64,
    pins: u32,
}
#[derive(Debug)]
struct Budget {
    state: Mutex<Residency>,
    limit: u64,
    max_pins: u32,
}
impl Budget {
    fn reserve(self: &Arc<Self>, bytes: u64) -> Result<SegmentedReservation, SegmentedError> {
        let mut state = lock_read_state(&self.state);
        let requested = state
            .live
            .checked_add(bytes)
            .ok_or(SegmentedError::AddressExhausted)?;
        if requested > self.limit {
            return Err(SegmentedError::Residency {
                requested,
                limit: self.limit,
            });
        }
        state.live = requested;
        state.peak = state.peak.max(requested);
        drop(state);
        Ok(SegmentedReservation {
            budget: Arc::clone(self),
            bytes,
            control: 0,
            cache: None,
            base: None,
        })
    }
}

/// A pre-allocation live-byte reservation. The allocation it admits must not grow
/// past `bytes()`, and must be released before the reservation is dropped.
#[derive(Debug)]
pub struct SegmentedReservation {
    budget: Arc<Budget>,
    bytes: u64,
    // An owned factory admits its concrete box before allocating it. Resizing
    // payload to zero must still cover that live control allocation.
    control: u64,
    // Cache blocks themselves hold reservations. A weak account prevents their
    // reservations from keeping the cache alive through a reference cycle.
    cache: Option<std::sync::Weak<Mutex<State>>>,
    // Escaping workspace owns the original account allocation. The base itself
    // and cache-owned reservations leave this empty, preventing an owner cycle.
    base: Option<Arc<Self>>,
}
impl SegmentedReservation {
    #[cfg(test)]
    pub(super) fn base_owner(&self) -> &Arc<Self> {
        self.base
            .as_ref()
            .expect("workspace owns its original base")
    }
    #[cfg(test)]
    pub(super) fn live_bytes(&self) -> u64 {
        lock_read_state(&self.budget.state).live
    }
    /// The admitted retained-capacity charge.
    #[must_use]
    pub const fn bytes(&self) -> u64 {
        self.bytes
    }
    fn shrink(&mut self, actual: u64) {
        debug_assert!(actual <= self.bytes);
        lock_read_state(&self.budget.state).live -= self.bytes - actual;
        self.bytes = actual;
    }
}
impl crate::WorkspaceReservation for SegmentedReservation {
    type Error = SegmentedError;
    fn resize(&mut self, bytes: u64) -> Result<(), SegmentedError> {
        let bytes = bytes
            .checked_add(self.control)
            .ok_or(SegmentedError::AddressExhausted)?;
        if bytes <= self.bytes {
            self.shrink(bytes);
            return Ok(());
        }
        let growth = bytes - self.bytes;
        if let Some(cache) = self.cache.as_ref().and_then(std::sync::Weak::upgrade) {
            make_room(&mut lock_read_state(&cache), &self.budget, growth, None)?;
        }
        let mut state = lock_read_state(&self.budget.state);
        let requested = state
            .live
            .checked_add(growth)
            .ok_or(SegmentedError::AddressExhausted)?;
        if requested > self.budget.limit {
            return Err(SegmentedError::Residency {
                requested,
                limit: self.budget.limit,
            });
        }
        state.live = requested;
        state.peak = state.peak.max(requested);
        drop(state);
        self.bytes = bytes;
        Ok(())
    }
}
impl crate::WorkspaceReservation for [SegmentedReservation; 1] {
    type Error = SegmentedError;
    fn resize(&mut self, bytes: u64) -> Result<(), SegmentedError> {
        crate::WorkspaceReservation::resize(&mut self[0], bytes)
    }
}
impl Drop for SegmentedReservation {
    fn drop(&mut self) {
        lock_read_state(&self.budget.state).live -= self.bytes;
        drop(self.base.take());
    }
}

#[derive(Debug)]
struct DecodedPayload {
    records: DecodedRecords,
    reservation: SegmentedReservation,
}

impl DecodedPayload {
    fn new(records: DecodedRecords, mut reservation: SegmentedReservation) -> Self {
        let actual = records.arena.capacity() as u64
            + (records.entries.capacity() * size_of::<(GlobalTermId, core::ops::Range<usize>)>())
                as u64;
        reservation.shrink(actual);
        Self {
            records,
            reservation,
        }
    }
}

#[derive(Debug)]
struct CachedBlock {
    encoded: Vec<u8>,
    decoded: OnceLock<DecodedPayload>,
    dictionary_range: Option<(u64, u64)>,
    reservation: SegmentedReservation,
}

impl CachedBlock {
    fn records(&self) -> Option<&DecodedRecords> {
        self.decoded.get().map(|payload| &payload.records)
    }

    fn retained_charge(&self) -> u64 {
        self.reservation.bytes()
            + self
                .decoded
                .get()
                .map_or(0, |payload| payload.reservation.bytes())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct CacheKey {
    snapshot: SegmentedSnapshot,
    slot: u64,
}
#[derive(Clone, Copy, Debug)]
struct DictionaryPosition {
    key: CacheKey,
    bucket: u64,
    first: u64,
    end: u64,
}
#[derive(Debug)]
struct CacheEntry {
    key: CacheKey,
    block: Arc<CachedBlock>,
    older: Option<CacheKey>,
    newer: Option<CacheKey>,
}
#[derive(Clone, Copy)]
enum ReaderBuffer {
    Cache {
        block_charge: u64,
        dictionary_entries: usize,
    },
    Requests,
}
struct State {
    cache: HashTable<CacheEntry>,
    oldest: Option<CacheKey>,
    newest: Option<CacheKey>,
    // Learn routing bounds only from authenticated dictionary reads. They
    // survive payload eviction without owning a pin or lexical bytes, and grow
    // under the operation's original grant, never from a declared image size.
    dictionary_positions: HashTable<DictionaryPosition>,
    requests: Vec<SegmentedRequest>,
    error: Option<SegmentedError>,
    io_bytes: u64,
    evictions: u64,
    hash: Hasher,
    // Buffers die before their original live-byte grant is released. This
    // reservation has no cache backlink: resizing while holding State must not
    // lock State again.
    storage: super::query::QueryStorage,
}

impl State {
    fn reserve_buffer(&mut self, buffer: ReaderBuffer) -> Result<(), SegmentedError> {
        loop {
            // Memory checks the old/new replacement overlap. Also plan the
            // final cache-plus-block layout before allocating: reclaiming one
            // slot can avoid growth altogether, especially beside pinned blocks.
            let refusal = if let ReaderBuffer::Cache {
                block_charge,
                dictionary_entries,
            } = buffer
            {
                let required = self
                    .cache
                    .len()
                    .checked_add(1)
                    .ok_or(StorageError::SizeOverflow)?;
                let next = hash_table_allocation_bound::<CacheEntry>(required)
                    .ok_or(StorageError::SizeOverflow)?;
                let old =
                    hash_table_retained_bound(&self.cache).ok_or(StorageError::SizeOverflow)?;
                let mut increase = if required > self.cache.capacity() {
                    next.saturating_sub(old)
                } else {
                    0
                };
                if dictionary_entries != 0 {
                    let required = self
                        .dictionary_positions
                        .len()
                        .checked_add(dictionary_entries)
                        .ok_or(StorageError::SizeOverflow)?;
                    if required > self.dictionary_positions.capacity() {
                        let next = hash_table_allocation_bound::<DictionaryPosition>(required)
                            .ok_or(StorageError::SizeOverflow)?;
                        let old = hash_table_retained_bound(&self.dictionary_positions)
                            .ok_or(StorageError::SizeOverflow)?;
                        increase = increase
                            .checked_add(next.saturating_sub(old))
                            .ok_or(StorageError::SizeOverflow)?;
                    }
                }
                let increase = u64::try_from(increase).map_err(|_| StorageError::SizeOverflow)?;
                let budget = &self.storage.reservation.budget;
                let live = lock_read_state(&budget.state).live;
                let requested = live
                    .checked_add(increase)
                    .and_then(|live| live.checked_add(block_charge))
                    .ok_or(SegmentedError::AddressExhausted)?;
                (requested > budget.limit).then_some(SegmentedError::Residency {
                    requested,
                    limit: budget.limit,
                })
            } else {
                None
            };
            let result = if let Some(error) = refusal {
                Err(error)
            } else {
                match buffer {
                    ReaderBuffer::Cache {
                        dictionary_entries, ..
                    } => {
                        let required = self
                            .cache
                            .len()
                            .checked_add(1)
                            .ok_or(StorageError::SizeOverflow)?;
                        self.storage.run(|memory| {
                            reserve_table_with_memory(
                                &mut self.cache,
                                required,
                                |entry| hash_of(&entry.key),
                                memory,
                            )?;
                            if dictionary_entries != 0 {
                                let required = self
                                    .dictionary_positions
                                    .len()
                                    .checked_add(dictionary_entries)
                                    .ok_or(StorageError::SizeOverflow)?;
                                reserve_table_with_memory(
                                    &mut self.dictionary_positions,
                                    required,
                                    |entry| hash_of(&(entry.key.snapshot, entry.bucket)),
                                    memory,
                                )?;
                            }
                            Ok::<(), StorageError>(())
                        })
                    }
                    ReaderBuffer::Requests => self
                        .storage
                        .run(|memory| memory.reserve_for_push(&mut self.requests)),
                }
            };
            // State is already locked. Evict here rather than installing a
            // reservation backlink that would recursively acquire that lock.
            // Optional routing must fit spare capacity without evicting payloads.
            let can_evict = matches!(
                buffer,
                ReaderBuffer::Requests
                    | ReaderBuffer::Cache {
                        dictionary_entries: 0,
                        ..
                    }
            );
            if can_evict
                && matches!(&result, Err(SegmentedError::Residency { .. }))
                && self.evict_one()?
            {
                continue;
            }
            return result;
        }
    }
}

// Workspace growth and block loading evict through the same account. Lexical
// pins prevent eviction; iterator-owned rows can leave the cache while their
// original reservation stays live. Checked addition is a bound, not a heuristic.
fn make_room(
    state: &mut State,
    budget: &Budget,
    charge: u64,
    cache_slots: Option<u32>,
) -> Result<(), SegmentedError> {
    loop {
        let requested = lock_read_state(&budget.state)
            .live
            .checked_add(charge)
            .ok_or(SegmentedError::AddressExhausted)?;
        let slots_available = match cache_slots {
            None => true,
            Some(slots) => {
                state.cache.len()
                    < usize::try_from(slots).map_err(|_| SegmentedError::AddressExhausted)?
            }
        };
        if requested <= budget.limit && slots_available {
            return Ok(());
        }
        if !state.evict_one()? {
            let error = if state.cache.is_empty() {
                SegmentedError::Residency {
                    requested,
                    limit: budget.limit,
                }
            } else {
                SegmentedError::PinnedBlocks
            };
            return Err(error);
        }
    }
}
impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SegmentedReadState")
            .field("cache_blocks", &self.cache.len())
            .field("requests", &self.requests.len())
            .field("error", &self.error)
            .field("io_bytes", &self.io_bytes)
            .field("evictions", &self.evictions)
            .finish_non_exhaustive()
    }
}

fn reserve_workspace(
    state: &Arc<Mutex<State>>,
    budget: &Arc<Budget>,
    base: &Arc<SegmentedReservation>,
    bytes: u64,
) -> Result<SegmentedReservation, SegmentedError> {
    make_room(&mut lock_read_state(state), budget, bytes, None)?;
    budget.reserve(bytes).map(|mut reservation| {
        reservation.cache = Some(Arc::downgrade(state));
        reservation.base = Some(Arc::clone(base));
        reservation
    })
}

/// Snapshot evidence shares the bounded ledger allocation. Its exact prefix and
/// digest are immutable even if the live session subsequently appends requests.
/// Cloning/checkpointing evidence does not clone or expand the request vector.
#[derive(Debug, Clone)]
pub struct SegmentedEvidence<Identity = SegmentedSnapshot> {
    snapshot: Identity,
    state: Arc<Mutex<State>>,
    prefix: usize,
    hash: [u8; 32],
    io_bytes: u64,
    live_bytes: u64,
    unpinned_cache_bytes: u64,
    bookkeeping_bytes: u64,
    peak_bytes: u64,
    evictions: u64,
    _base: Arc<SegmentedReservation>,
}
impl<Identity: PartialEq> PartialEq for SegmentedEvidence<Identity> {
    fn eq(&self, other: &Self) -> bool {
        if self.snapshot != other.snapshot
            || self.prefix != other.prefix
            || self.hash != other.hash
            || self.io_bytes != other.io_bytes
            || self.live_bytes != other.live_bytes
            || self.unpinned_cache_bytes != other.unpinned_cache_bytes
            || self.bookkeeping_bytes != other.bookkeeping_bytes
            || self.peak_bytes != other.peak_bytes
            || self.evictions != other.evictions
        {
            return false;
        }
        if Arc::ptr_eq(&self.state, &other.state) {
            return true;
        }
        let (first, second) = if Arc::as_ptr(&self.state) < Arc::as_ptr(&other.state) {
            (self, other)
        } else {
            (other, self)
        };
        let a = first
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let b = second
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        a.requests[..first.prefix] == b.requests[..second.prefix]
    }
}
impl<Identity: Eq> Eq for SegmentedEvidence<Identity> {}
impl<Identity> SegmentedEvidence<Identity> {
    /// Original immutable carrier identity retained by this checkpoint.
    #[must_use]
    pub const fn identity(&self) -> &Identity {
        &self.snapshot
    }
    /// Descriptor anchoring this checkpoint. Each request carries its own source
    /// identity when multiple descriptors share this operation.
    #[must_use]
    pub const fn snapshot(&self) -> Identity
    where
        Identity: Copy,
    {
        self.snapshot
    }
    /// Number of exact retained requests at this checkpoint.
    #[must_use]
    pub const fn request_count(&self) -> usize {
        self.prefix
    }
    /// Fixed-order digest of the exact request prefix, including host refusals.
    #[must_use]
    pub const fn request_digest(&self) -> &[u8; 32] {
        &self.hash
    }
    /// Total bytes requested, including authenticated-index proof ranges.
    #[must_use]
    pub const fn io_bytes(&self) -> u64 {
        self.io_bytes
    }
    /// Live charged capacity at the checkpoint, including outstanding pins.
    #[must_use]
    pub const fn live_bytes(&self) -> u64 {
        self.live_bytes
    }
    /// Charged capacity retained exclusively by the evictable provider cache.
    ///
    /// Subtracting this from [`Self::live_bytes`] isolates the original account's
    /// other live owners. Pinned blocks remain in that balance: a leaked pin or
    /// query grant cannot be hidden by a legitimate cache eviction or reload.
    /// Reader array capacity is reported separately by [`Self::bookkeeping_bytes`].
    #[must_use]
    pub const fn unpinned_cache_bytes(&self) -> u64 {
        self.unpinned_cache_bytes
    }
    /// Charged request-ledger and cache-slot capacity retained by the reader.
    ///
    /// These buffers grow on demand and survive individual query results.
    /// Subtract this and [`Self::unpinned_cache_bytes`] from live residency to
    /// compare the original control, pinned blocks and external workspace owners.
    #[must_use]
    pub const fn bookkeeping_bytes(&self) -> u64 {
        self.bookkeeping_bytes
    }
    /// Highest pre-admitted live capacity so far; never exceeds the caller's ceiling.
    #[must_use]
    pub const fn peak_bytes(&self) -> u64 {
        self.peak_bytes
    }
    /// Completed sparse-cache evictions. Reloads remain distinct exact requests.
    #[must_use]
    pub const fn evictions(&self) -> u64 {
        self.evictions
    }
    /// Inspect the exact immutable ledger prefix without an unbudgeted clone.
    pub fn with_requests<T>(&self, inspect: impl FnOnce(&[SegmentedRequest]) -> T) -> T {
        let state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        inspect(&state.requests[..self.prefix])
    }
}

fn checkpoint<I>(
    snapshot: I,
    owner: &Arc<Mutex<State>>,
    budget: &Arc<Budget>,
    base: &Arc<SegmentedReservation>,
) -> SegmentedEvidence<I> {
    let state = lock_read_state(owner);
    let residency = lock_read_state(&budget.state);
    SegmentedEvidence {
        snapshot,
        state: Arc::clone(owner),
        prefix: state.requests.len(),
        hash: *state.hash.finalize().as_bytes(),
        io_bytes: state.io_bytes,
        live_bytes: residency.live,
        bookkeeping_bytes: state.storage.reservation.bytes(),
        // Disjoint charged reservations cannot sum past checked live residency.
        unpinned_cache_bytes: state
            .cache
            .iter()
            .filter(|entry| Arc::strong_count(&entry.block) == 1)
            .map(|entry| entry.block.retained_charge())
            .sum(),
        peak_bytes: residency.peak,
        evictions: state.evictions,
        _base: Arc::clone(base),
    }
}

fn owned_workspace(
    bytes: u64,
    reserve: impl FnOnce(u64) -> Result<SegmentedReservation, SegmentedError>,
) -> Result<crate::OwnedWorkspaceReservation<SegmentedError>, SegmentedError> {
    let control = u64::try_from(size_of::<SegmentedReservation>())
        .map_err(|_| SegmentedError::AddressExhausted)?;
    let total = bytes
        .checked_add(control)
        .ok_or(SegmentedError::AddressExhausted)?;
    let mut reservation = reserve(total)?;
    reservation.control = control;
    // The one-element allocation has the concrete box layout and is admitted
    // before birth. Converting it to the erased capability needs no growth.
    let boxed = crate::small::try_boxed_one(reservation)
        .map_err(|_| SegmentedError::AllocationFailed("owned workspace reservation"))?;
    Ok(boxed as crate::OwnedWorkspaceReservation<SegmentedError>)
}

/// The original live-byte/cache/request/first-fault owner shared by every
/// descriptor read in one stack query. Opening an ordinary session uses the
/// same owner with one descriptor.
#[derive(Debug, Clone)]
pub(super) struct SegmentedOperation {
    limits: SegmentedReadLimits,
    state: Arc<Mutex<State>>,
    budget: Arc<Budget>,
    base: Arc<SegmentedReservation>,
}

impl SegmentedOperation {
    pub(super) fn new(limits: SegmentedReadLimits) -> Result<Self, SegmentedError> {
        let cache_count =
            usize::try_from(limits.cache_blocks).map_err(|_| SegmentedError::AddressExhausted)?;
        if cache_count == 0 {
            return Err(SegmentedError::PinnedBlocks);
        }
        let base_bytes =
            u64::try_from(size_of::<Self>() + size_of::<State>() + size_of::<Budget>() + 192)
                .map_err(|_| SegmentedError::AddressExhausted)?;
        if base_bytes > limits.live_bytes {
            return Err(SegmentedError::Residency {
                requested: base_bytes,
                limit: limits.live_bytes,
            });
        }
        // Reserve actual account/control layouts, not permitted future reads.
        let budget = Arc::new(Budget {
            state: Mutex::new(Residency {
                live: base_bytes,
                peak: base_bytes,
                pins: 0,
            }),
            limit: limits.live_bytes,
            max_pins: limits.pins,
        });
        let base = Arc::new(SegmentedReservation {
            budget: Arc::clone(&budget),
            bytes: base_bytes,
            control: 0,
            cache: None,
            base: None,
        });
        let state = Arc::new(Mutex::new(State {
            cache: HashTable::new(),
            oldest: None,
            newest: None,
            dictionary_positions: HashTable::new(),
            requests: Vec::new(),
            error: None,
            io_bytes: 0,
            evictions: 0,
            hash: {
                let mut hash = Hasher::new();
                hash.update(super::SEGMENTED_REQUESTS_DOMAIN.as_bytes());
                hash
            },
            storage: super::query::QueryStorage {
                reservation: budget.reserve(0)?,
                cause: None,
            },
        }));
        Ok(Self {
            limits,
            state,
            budget,
            base,
        })
    }

    pub(super) fn ready(&self) -> Result<(), SegmentedError> {
        let error = lock_read_state(&self.state).error.clone();
        error.map_or(Ok(()), Err)
    }

    pub(super) fn evidence<I>(&self, identity: I) -> SegmentedEvidence<I> {
        checkpoint(identity, &self.state, &self.budget, &self.base)
    }

    pub(super) fn live_limit(&self) -> u64 {
        self.limits.live_bytes
    }

    pub(super) fn reserve_owned_workspace(
        &self,
        bytes: u64,
    ) -> Result<crate::OwnedWorkspaceReservation<SegmentedError>, SegmentedError> {
        owned_workspace(bytes, |bytes| self.reserve_workspace(bytes))
    }

    pub(super) fn fail<T>(&self, error: SegmentedError) -> Result<T, SegmentedError> {
        latch_error(&self.state, error)
    }

    reserve_workspace_method!(pub(super));

    pub(super) fn open(
        &self,
        provider: Arc<dyn SegmentedProvider>,
        receipt: &SegmentedReceipt,
    ) -> Result<SegmentedSession, SegmentedError> {
        let error = lock_read_state(&self.state).error.clone();
        if let Some(error) = error {
            return Err(error);
        }
        if provider.snapshot() != receipt.snapshot || provider.byte_len() != receipt.byte_len {
            return latch_error(&self.state, SegmentedError::SnapshotMismatch);
        }
        let descriptor = (|| {
            let bytes = u64::try_from(size_of::<SegmentedSession>())
                .map_err(|_| SegmentedError::AddressExhausted)?;
            make_room(&mut lock_read_state(&self.state), &self.budget, bytes, None)?;
            self.budget.reserve(bytes)
        })()
        .or_else(|error| latch_error(&self.state, error))?;
        let identity = (|| {
            let bytes = u64::try_from(Shared::<RequestIdentity>::allocation_layout().size())
                .map_err(|_| SegmentedError::AddressExhausted)?;
            let reservation = self.reserve_workspace(bytes)?;
            Shared::try_new(RequestIdentity {
                snapshot: receipt.snapshot,
                _reservation: reservation,
            })
            .map_err(|_| SegmentedError::AllocationFailed("request identity"))
        })()
        .or_else(|error| latch_error(&self.state, error))?;
        let mut session = SegmentedSession {
            provider,
            receipt: receipt.clone(),
            identity,
            limits: self.limits,
            header: Header {
                block_bytes: receipt.block_bytes,
                records_per_block: 1,
                first_term_index: 0,
                max_owned_term_bytes: 0,
                terms: 0,
                dictionary: Stream::default(),
                reverse: Stream::default(),
                streams: [Stream::default(); 3],
                graphs: Stream::default(),
                summaries: Stream::default(),
                value_ranges: Stream::default(),
                statistics: [Stream::default(); 4],
                effective_directory: Stream::default(),
                source_checkpoints: Stream::default(),
                overflow: Stream::default(),
                flags: 0,
                source_union: purrdf_hash::multiset::Multiset256::default(),
            },
            state: Arc::clone(&self.state),
            budget: Arc::clone(&self.budget),
            base: Arc::clone(&self.base),
            _descriptor: descriptor,
        };
        let header = session.block(0)?;
        let decoded = Header::decode(&header.encoded).or_else(|error| session.fail(error))?;
        decoded
            .validate_addresses(receipt.block_count)
            .or_else(|error| session.fail(error))?;
        if decoded.block_bytes != receipt.block_bytes {
            return session.fail(SegmentedError::Corrupt(
                "receipt/header block size mismatch",
            ));
        }
        session.header = decoded;
        Ok(session)
    }
}

fn latch_error<T>(state: &Mutex<State>, error: SegmentedError) -> Result<T, SegmentedError> {
    let mut state = lock_read_state(state);
    let error = state.error.get_or_insert(error).clone();
    drop(state);
    Err(error)
}

/// A certified immutable snapshot, read through one bounded operational session.
/// Metadata and translations are not expanded into whole-dataset arrays. Every
/// cached object and returned pin retains its live reservation through eviction.
#[derive(Debug)]
pub struct SegmentedSession {
    provider: Arc<dyn SegmentedProvider>,
    receipt: SegmentedReceipt,
    identity: Shared<RequestIdentity>,
    limits: SegmentedReadLimits,
    header: Header,
    state: Arc<Mutex<State>>,
    budget: Arc<Budget>,
    base: Arc<SegmentedReservation>,
    _descriptor: SegmentedReservation,
}

impl SegmentedSession {
    /// The named graph record at `index`, ascending; `None` once a read fault has
    /// latched (the fault is recorded, as every read records one).
    fn named_graph_at(&self, index: u64) -> Option<GlobalTermId> {
        let per_block = (u64::from(self.header.block_bytes) - 9) / 8;
        let block = self
            .block(self.header.graphs.first + index / per_block)
            .ok()?;
        let records = match fixed_records(&block.encoded, format::GRAPHS, 8) {
            Ok(data) => data,
            Err(error) => {
                let _ = self.fail::<()>(error);
                return None;
            }
        };
        let offset = usize::try_from(index % per_block).ok()?.checked_mul(8)?;
        match records
            .get(offset..offset + 8)
            .ok_or(SegmentedError::Corrupt("missing named graph ordinal"))
            .and_then(|bytes| Reader::at(bytes, 0).id())
        {
            Ok(id) => Some(id),
            Err(error) => {
                let _ = self.fail::<()>(error);
                None
            }
        }
    }

    /// Qualify a compact term ID for external storage or a later session attachment.
    ///
    /// # Errors
    /// Refuses IDs outside this certified dictionary interval.
    pub fn handle(&self, id: GlobalTermId) -> Result<super::SegmentedHandle, SegmentedError> {
        let index = id
            .index()
            .checked_sub(self.header.first_term_index)
            .ok_or(SegmentedError::Corrupt("foreign term attachment"))?;
        if index >= self.header.terms {
            return Err(SegmentedError::Corrupt("foreign term attachment"));
        }
        Ok(super::SegmentedHandle {
            snapshot: self.receipt.snapshot,
            id,
        })
    }

    /// Validate an external generation-qualified handle before retaining its compact
    /// ID in joins. An attachment to another snapshot is never silently reinterpreted.
    ///
    /// # Errors
    /// Refuses another generation or an ID outside the certified interval.
    pub fn attach(&self, handle: super::SegmentedHandle) -> Result<GlobalTermId, SegmentedError> {
        if handle.snapshot != self.receipt.snapshot {
            return Err(SegmentedError::SnapshotMismatch);
        }
        self.handle(handle.id).map(|checked| checked.id)
    }
    /// Selectively reopen a fully certified snapshot by authenticating its header.
    /// No term, reverse-index, graph, or quad block is decoded at open.
    ///
    /// # Errors
    /// Refuses mismatched authority/provider identity, unauthenticated bytes,
    /// insufficient pre-allocation budget or an exhausted exact request ledger.
    pub fn open(
        provider: Arc<dyn SegmentedProvider>,
        receipt: &SegmentedReceipt,
        limits: SegmentedReadLimits,
    ) -> Result<Self, SegmentedError> {
        if provider.snapshot() != receipt.snapshot || provider.byte_len() != receipt.byte_len {
            return Err(SegmentedError::SnapshotMismatch);
        }
        SegmentedOperation::new(limits)?.open(provider, receipt)
    }

    fn fail<T>(&self, error: SegmentedError) -> Result<T, SegmentedError> {
        latch_error(&self.state, error)
    }
    pub(super) fn source_union(
        &self,
    ) -> Result<purrdf_hash::multiset::Multiset256, SegmentedError> {
        self.ready()?;
        Ok(self.header.source_union)
    }

    fn ready(&self) -> Result<(), SegmentedError> {
        let error = lock_read_state(&self.state).error.clone();
        if let Some(error) = error {
            return Err(error);
        }
        if self.provider.snapshot() != self.receipt.snapshot
            || self.provider.byte_len() != self.receipt.byte_len
        {
            return self.fail(SegmentedError::SnapshotMismatch);
        }
        Ok(())
    }

    reserve_workspace_method! {
        /// Reserve query scratch, output staging, or a host batch BEFORE allocating it.
        /// Storage cache admission observes the same ceiling while this guard is alive.
        ///
        /// # Errors
        /// Refuses a sticky read failure or capacity blocked by live cache/pins.
        pub
    }

    fn make_room(&self, state: &mut State, charge: u64, slot: bool) -> Result<(), SegmentedError> {
        make_room(
            state,
            &self.budget,
            charge,
            slot.then_some(self.limits.cache_blocks),
        )
        .inspect_err(|error| {
            // A refused workspace resize leaves the readable snapshot valid.
            // Block-load failures remain sticky so omitted facts cannot succeed.
            if slot {
                state.error.get_or_insert_with(|| error.clone());
            }
        })
    }

    fn request(
        &self,
        state: &mut State,
        position: u64,
        output: &mut [u8],
    ) -> Result<(), SegmentedError> {
        if u64::try_from(state.requests.len()).map_err(|_| SegmentedError::AddressExhausted)?
            >= u64::from(self.limits.requests)
        {
            return Err(SegmentedError::EvidenceExhausted);
        }
        let bytes = u32::try_from(output.len()).map_err(|_| SegmentedError::AddressExhausted)?;
        let io = state
            .io_bytes
            .checked_add(u64::from(bytes))
            .ok_or(SegmentedError::AddressExhausted)?;
        if io > self.limits.io_bytes {
            return Err(SegmentedError::IoBytes {
                requested: io,
                limit: self.limits.io_bytes,
            });
        }
        if position
            .checked_add(u64::from(bytes))
            .is_none_or(|end| end > self.receipt.byte_len)
        {
            return Err(SegmentedError::Corrupt("range escapes certified provider"));
        }
        state.reserve_buffer(ReaderBuffer::Requests)?;
        state.requests.push(SegmentedRequest {
            snapshot: self
                .identity
                .try_clone()
                .map_err(SegmentedError::SharedOwnership)?,
            position,
            bytes,
            completed: false,
        });
        state.io_bytes = io;
        let result = self.provider.read_at(position, output);
        if result.is_ok() {
            state
                .requests
                .last_mut()
                .expect("the admitted request was appended")
                .completed = true;
        }
        state.hash.update(self.receipt.snapshot.as_bytes());
        state.hash.update(&position.to_le_bytes());
        state.hash.update(&bytes.to_le_bytes());
        state.hash.update(&[u8::from(result.is_ok())]);
        result
    }

    fn read_authenticated(
        &self,
        state: &mut State,
        slot: u64,
        output: &mut [u8],
    ) -> Result<(), SegmentedError> {
        if slot >= self.receipt.block_count
            || output.len() as u64 != u64::from(self.receipt.block_bytes)
        {
            return Err(SegmentedError::Corrupt(
                "invalid authenticated page request",
            ));
        }
        let position = slot
            .checked_mul(u64::from(self.receipt.block_bytes))
            .ok_or(SegmentedError::AddressExhausted)?;
        self.request(state, position, output)?;
        let mut digest = format::leaf_digest(slot, output);
        let mut node = self
            .receipt
            .leaf_count
            .checked_add(slot)
            .ok_or(SegmentedError::AddressExhausted)?;
        let data_bytes = self
            .receipt
            .block_count
            .checked_mul(u64::from(self.receipt.block_bytes))
            .ok_or(SegmentedError::AddressExhausted)?;
        while node > 1 {
            let position = (node ^ 1)
                .checked_mul(32)
                .and_then(|n| n.checked_add(data_bytes))
                .ok_or(SegmentedError::AddressExhausted)?;
            let mut sibling = [0; 32];
            self.request(state, position, &mut sibling)?;
            digest = if node & 1 == 0 {
                format::node_digest(&digest, &sibling)
            } else {
                format::node_digest(&sibling, &digest)
            };
            node /= 2;
        }
        if digest != self.receipt.root {
            return Err(SegmentedError::Corrupt("block/index authentication failed"));
        }
        Ok(())
    }

    fn block(&self, slot: u64) -> Result<Arc<CachedBlock>, SegmentedError> {
        self.ready()?;
        if slot >= self.receipt.block_count {
            return self.fail(SegmentedError::Corrupt("block outside certified directory"));
        }
        let mut state = lock_read_state(&self.state);
        // Another operation can fail after ready() releases State. Inspect the
        // original fault again under this lock, before cache reuse or preflight.
        if let Some(error) = state.error.clone() {
            drop(state);
            return Err(error);
        }
        let key = CacheKey {
            snapshot: self.receipt.snapshot,
            slot,
        };
        if let Some(block) = state.cache_hit(key) {
            return Ok(block);
        }
        let proof_reads = self.receipt.leaf_count.trailing_zeros();
        let needed = usize::try_from(proof_reads)
            .map_err(|_| SegmentedError::AddressExhausted)?
            .checked_add(1)
            .ok_or(SegmentedError::AddressExhausted)?;
        if state
            .requests
            .len()
            .checked_add(needed)
            .is_none_or(|count| {
                u64::try_from(count).map_or(true, |count| count > u64::from(self.limits.requests))
            })
        {
            state.error = Some(SegmentedError::EvidenceExhausted);
            return Err(SegmentedError::EvidenceExhausted);
        }
        let io = state
            .io_bytes
            .checked_add(u64::from(self.receipt.block_bytes))
            .and_then(|v| v.checked_add(u64::from(proof_reads) * 32))
            .ok_or(SegmentedError::AddressExhausted)?;
        if io > self.limits.io_bytes {
            let error = SegmentedError::IoBytes {
                requested: io,
                limit: self.limits.io_bytes,
            };
            state.error = Some(error.clone());
            return Err(error);
        }
        let encoded = u64::from(self.receipt.block_bytes);
        let in_dictionary = slot >= self.header.dictionary.first
            && slot < self.header.dictionary.first + self.header.dictionary.blocks;
        let records = if slot != 0 && in_dictionary {
            u64::from(self.header.records_per_block)
        } else {
            0
        };
        let decoded = encoded
            .checked_mul(records)
            .ok_or(SegmentedError::AddressExhausted)?;
        let overhead = u64::try_from(size_of::<CachedBlock>() + 64)
            .map_err(|_| SegmentedError::AddressExhausted)?;
        let charge = encoded
            .checked_add(overhead)
            .ok_or(SegmentedError::AddressExhausted)?;
        self.make_room(&mut state, charge, true)?;
        if let Err(error) = state.reserve_buffer(ReaderBuffer::Cache {
            block_charge: charge,
            dictionary_entries: 0,
        }) {
            state.error.get_or_insert_with(|| error.clone());
            return Err(error);
        }
        // Admitted cache capacity can consume the room previously left for
        // this block. Reclaim against the actual combined live layouts.
        self.make_room(&mut state, charge, true)?;
        let reservation = self.budget.reserve(charge)?;
        let mut bytes = vec![
            0_u8;
            usize::try_from(self.receipt.block_bytes)
                .map_err(|_| SegmentedError::AddressExhausted)?
        ];
        let result = (|| {
            self.read_authenticated(&mut state, slot, &mut bytes)?;
            let (decode_bytes, scratch_bytes) = if in_dictionary && bytes[0] == format::DICTIONARY {
                (
                    format::decode_memory_bytes(&bytes, self.header.records_per_block, decoded)?,
                    format::decode_scratch_bytes(&bytes, self.header.records_per_block, decoded)?,
                )
            } else {
                (0, 0)
            };
            let additional = decode_bytes
                .checked_add(scratch_bytes)
                .ok_or(SegmentedError::AddressExhausted)?;
            self.make_room(&mut state, additional, false)?;
            let payload_reservation = self.budget.reserve(decode_bytes)?;
            let scratch = self.budget.reserve(scratch_bytes)?;
            let decoded_records = match bytes[0] {
                format::DICTIONARY if slot != 0 => Some(DecodedRecords::decode(
                    &bytes,
                    self.header.records_per_block,
                    decoded,
                    self.header.overflow,
                    |slot, page| self.read_authenticated(&mut state, slot, page),
                )?),
                _ => None,
            };
            drop(scratch);
            let dictionary_range = if in_dictionary && bytes[0] == format::SPANNED_DICTIONARY {
                let (id, span) = format::overflow_descriptor(&bytes, self.header.overflow)?;
                if u64::from(span.bytes) > self.header.max_owned_term_bytes {
                    return Err(SegmentedError::Corrupt("overflow term exceeds owned bound"));
                }
                Some((
                    id.index(),
                    id.index()
                        .checked_add(1)
                        .ok_or(SegmentedError::AddressExhausted)?,
                ))
            } else if in_dictionary {
                let records = decoded_records.as_ref().ok_or(SegmentedError::Corrupt(
                    "forward dictionary block kind changed",
                ))?;
                let first = records.entries[0].0.index();
                let end = first
                    .checked_add(
                        u64::try_from(records.entries.len())
                            .map_err(|_| SegmentedError::AddressExhausted)?,
                    )
                    .ok_or(SegmentedError::AddressExhausted)?;
                Some((first, end))
            } else {
                None
            };
            let decoded = OnceLock::new();
            if let Some(records) = decoded_records {
                decoded
                    .set(DecodedPayload::new(records, payload_reservation))
                    .map_err(|_| SegmentedError::Corrupt("dictionary payload initialized twice"))?;
            }
            Ok(Arc::new(CachedBlock {
                encoded: bytes,
                decoded,
                dictionary_range,
                reservation,
            }))
        })();
        match result {
            Ok(block) => {
                // Only authenticated decoded bounds establish whether a page
                // spans one or two buckets. Admit that exact index growth while
                // the pending payload is already charged, before publishing it.
                let dictionary_entries = block.dictionary_range.map_or(0, |(first, end)| {
                    let width = u64::from(format::MAX_RECORDS_PER_BLOCK);
                    (first / width..=(end - 1) / width)
                        .filter(|&bucket| {
                            state
                                .dictionary_positions
                                .find(hash_of(&(key.snapshot, bucket)), |position| {
                                    position.key == key && position.bucket == bucket
                                })
                                .is_none()
                        })
                        .count()
                });
                let retain_route = if dictionary_entries == 0 {
                    true
                } else {
                    match state.reserve_buffer(ReaderBuffer::Cache {
                        block_charge: 0,
                        dictionary_entries,
                    }) {
                        Ok(()) => true,
                        // Routing hints are optional cache metadata. The payload
                        // and exact request ledger have already been admitted;
                        // refusing only this index growth cannot invalidate them.
                        Err(SegmentedError::Residency { .. }) => false,
                        Err(error) => {
                            state.error.get_or_insert_with(|| error.clone());
                            return Err(error);
                        }
                    }
                };
                state.cache_insert(key, Arc::clone(&block), retain_route);
                Ok(block)
            }
            Err(error) => {
                state.error.get_or_insert_with(|| error.clone());
                drop(state);
                Err(error)
            }
        }
    }

    fn materialize_overflow(&self, block: &CachedBlock) -> Result<(), SegmentedError> {
        self.ready()?;
        let mut state = lock_read_state(&self.state);
        if let Some(error) = state.error.clone() {
            return Err(error);
        }
        if block.records().is_some() {
            return Ok(());
        }
        let result: Result<(), SegmentedError> = (|| {
            let bytes =
                format::decode_memory_bytes(&block.encoded, 1, self.header.max_owned_term_bytes)?;
            let scratch_bytes = u64::from(self.header.block_bytes);
            self.make_room(
                &mut state,
                bytes
                    .checked_add(scratch_bytes)
                    .ok_or(SegmentedError::AddressExhausted)?,
                false,
            )?;
            let reservation = self.budget.reserve(bytes)?;
            let scratch = self.budget.reserve(scratch_bytes)?;
            let records = DecodedRecords::decode(
                &block.encoded,
                1,
                self.header.max_owned_term_bytes,
                self.header.overflow,
                |slot, page| self.read_authenticated(&mut state, slot, page),
            )?;
            format::decode_term(records.key(0))?;
            drop(scratch);
            block
                .decoded
                .set(DecodedPayload::new(records, reservation))
                .map_err(|_| SegmentedError::Corrupt("dictionary payload initialized twice"))?;
            Ok(())
        })();
        if let Err(error) = &result {
            state.error.get_or_insert_with(|| error.clone());
        }
        result
    }

    fn compare_key(
        &self,
        id: GlobalTermId,
        key: &[u8],
    ) -> Result<core::cmp::Ordering, SegmentedError> {
        let (block, index) = self.term_page(id)?;
        if let Some(records) = block.records() {
            return Ok(records.key(index).cmp(key));
        }
        let (_, span) = format::overflow_descriptor(&block.encoded, self.header.overflow)?;
        let mut state = lock_read_state(&self.state);
        if let Some(error) = state.error.clone() {
            return Err(error);
        }
        let result: Result<core::cmp::Ordering, SegmentedError> = (|| {
            self.make_room(&mut state, u64::from(self.header.block_bytes), false)?;
            let _scratch = self.budget.reserve(u64::from(self.header.block_bytes))?;
            let mut consumed = 0;
            let mut ordering = None;
            let completed = format::visit_overflow(
                span,
                self.header.overflow,
                self.header.block_bytes as usize,
                |slot, page| self.read_authenticated(&mut state, slot, page),
                |chunk| {
                    let count = chunk.len().min(key.len() - consumed);
                    let compared = chunk[..count].cmp(&key[consumed..consumed + count]);
                    consumed += count;
                    if compared != core::cmp::Ordering::Equal {
                        ordering = Some(compared);
                    } else if consumed == key.len() && consumed < span.bytes as usize {
                        ordering = Some(core::cmp::Ordering::Greater);
                    }
                    if ordering.is_some() {
                        core::ops::ControlFlow::Break(())
                    } else {
                        core::ops::ControlFlow::Continue(())
                    }
                },
            )?;
            match (completed, ordering) {
                (core::ops::ControlFlow::Break(()), Some(ordering)) => Ok(ordering),
                (core::ops::ControlFlow::Continue(()), None) => {
                    Ok((span.bytes as usize).cmp(&key.len()))
                }
                _ => Err(SegmentedError::Corrupt("inconsistent overflow comparison")),
            }
        })();
        if let Err(error) = &result {
            state.error.get_or_insert_with(|| error.clone());
        }
        result
    }

    fn term_block(&self, id: GlobalTermId) -> Result<(Arc<CachedBlock>, usize), SegmentedError> {
        let (block, index) = self.term_page(id)?;
        if block.records().is_none() {
            self.materialize_overflow(&block)?;
        }
        Ok((block, index))
    }

    fn term_page(&self, id: GlobalTermId) -> Result<(Arc<CachedBlock>, usize), SegmentedError> {
        let local = id
            .index()
            .checked_sub(self.header.first_term_index)
            .ok_or(SegmentedError::Corrupt("foreign term ID"))?;
        if local >= self.header.terms {
            return self.fail(SegmentedError::Corrupt("foreign term ID"));
        }
        // The count cap gives a lower bound, never an exact block address. Probe
        // it first so existing full-population images still need one block read.
        // Variable populations then use authenticated ID fences in a bounded
        // binary search, learning bounds lazily and releasing every probe pin.
        let mut lo = local / u64::from(self.header.records_per_block);
        let mut hi = self.header.dictionary.blocks;
        let bucket = id.index() / u64::from(format::MAX_RECORDS_PER_BLOCK);
        let mut first_probe = None;
        {
            let state = lock_read_state(&self.state);
            'buckets: for nearby in bucket.saturating_sub(1)..=bucket.saturating_add(1) {
                for position in state
                    .dictionary_positions
                    .iter_hash(hash_of(&(self.receipt.snapshot, nearby)))
                {
                    if position.key.snapshot != self.receipt.snapshot || position.bucket != nearby {
                        continue;
                    }
                    let mid = position.key.slot - self.header.dictionary.first;
                    // Dense IDs and nonempty pages bound the distance to an
                    // unknown page. Use nearby authenticated ranges without
                    // scanning the cache or eagerly reading a directory.
                    if id.index() < position.first {
                        let distance = position.first - id.index();
                        lo = lo.max(mid.saturating_sub(distance));
                        let pages = distance.div_ceil(u64::from(self.header.records_per_block));
                        hi = hi.min(mid.saturating_sub(pages).saturating_add(1));
                    } else if id.index() >= position.end {
                        let distance = id.index() - position.end;
                        lo = lo.max(
                            mid.checked_add(1)
                                .and_then(|value| {
                                    value.checked_add(
                                        distance / u64::from(self.header.records_per_block),
                                    )
                                })
                                .ok_or(SegmentedError::AddressExhausted)?,
                        );
                        hi = hi.min(mid.saturating_add(distance).saturating_add(2));
                    } else {
                        first_probe = Some(mid);
                        break 'buckets;
                    }
                }
            }
        }
        let mut first_probe = Some(first_probe.unwrap_or(lo));
        while lo < hi {
            let mid = first_probe.take().unwrap_or_else(|| lo + (hi - lo) / 2);
            let block = self.block(self.header.dictionary.first + mid)?;
            let (first, end) = block.dictionary_range.ok_or(SegmentedError::Corrupt(
                "forward dictionary block kind changed",
            ))?;
            if id.index() < first {
                hi = mid;
            } else {
                let offset = id.index() - first;
                if id.index() < end {
                    // Narrow only an in-block offset, including on wasm32.
                    let index =
                        usize::try_from(offset).map_err(|_| SegmentedError::AddressExhausted)?;
                    if let Some(records) = block.records() {
                        let (stored, _) = records.entries[index];
                        if stored != id {
                            return self.fail(SegmentedError::Corrupt(
                                "forward dictionary ordinal changed",
                            ));
                        }
                        format::decode_term(records.key(index))?;
                    }
                    return Ok((block, index));
                }
                lo = mid + 1;
            }
        }
        self.fail(SegmentedError::Corrupt(
            "forward dictionary ordinal missing",
        ))
    }

    fn lookup_key(&self, key: &[u8]) -> Result<Option<GlobalTermId>, SegmentedError> {
        let mut lo = 0_u64;
        let mut hi = self.header.reverse.blocks;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let block = self.block(self.header.reverse.first + mid)?;
            let ids = fixed_records(&block.encoded, format::REVERSE, 8)?;
            let id = reverse_id(ids, 0)?;
            if self.compare_key(id, key)? != core::cmp::Ordering::Greater {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        if lo == 0 {
            return Ok(None);
        }
        let block = self.block(self.header.reverse.first + lo - 1)?;
        let ids = fixed_records(&block.encoded, format::REVERSE, 8)?;
        let count = u64::try_from(ids.len() / 8).map_err(|_| SegmentedError::AddressExhausted)?;
        let index = format::partition_point(count, |index| {
            Ok::<_, SegmentedError>(
                self.compare_key(reverse_id(ids, index)?, key)? == core::cmp::Ordering::Less,
            )
        })?;
        if index == count {
            return Ok(None);
        }
        let id = reverse_id(ids, index)?;
        Ok((self.compare_key(id, key)? == core::cmp::Ordering::Equal).then_some(id))
    }

    fn lookup_value(
        &self,
        value: &TermValue,
        depth: u32,
    ) -> Result<Option<GlobalTermId>, SegmentedError> {
        self.ready()?;
        if depth > 16 {
            return Ok(None);
        }
        let term = match value {
            TermValue::Iri(iri) => TermRef::Iri(iri),
            TermValue::Blank { label, scope } => TermRef::Blank {
                label,
                scope: *scope,
            },
            TermValue::Literal {
                lexical_form,
                datatype,
                language,
                direction,
            } => {
                // No owned IRI clone at this boundary: encode the datatype leaf
                // directly and release that scratch before encoding the literal.
                let effective_datatype = if language.is_some() {
                    crate::RdfLiteral::language_datatype_iri(*direction)
                } else {
                    datatype
                };
                let Some(datatype) = self.lookup_leaf_iri(effective_datatype)? else {
                    return Ok(None);
                };
                let needs_fold = language
                    .as_deref()
                    .filter(|tag| !purrdf_iri::langtag::is_identity_folded(tag));
                let _fold = needs_fold
                    .map(|tag| {
                        self.reserve_workspace(
                            u64::try_from(tag.len()).expect("local tag bytes fit u64"),
                        )
                    })
                    .transpose()?;
                let lowered = needs_fold.map(purrdf_iri::langtag::identity_fold);
                return self.lookup_term_ref(TermRef::Literal {
                    lexical: lexical_form,
                    datatype,
                    language: lowered.as_deref().or(language.as_deref()),
                    direction: *direction,
                });
            }
            TermValue::Triple { s, p, o } => {
                let Some(s) = self.lookup_value(s, depth + 1)? else {
                    return Ok(None);
                };
                let Some(p) = self.lookup_value(p, depth + 1)? else {
                    return Ok(None);
                };
                let Some(o) = self.lookup_value(o, depth + 1)? else {
                    return Ok(None);
                };
                TermRef::Triple { s, p, o }
            }
        };
        self.lookup_term_ref(term)
    }
    fn lookup_leaf_iri(&self, iri: &str) -> Result<Option<GlobalTermId>, SegmentedError> {
        self.lookup_term_ref(TermRef::Iri(iri))
    }
    fn lookup_term_ref(
        &self,
        term: TermRef<'_, GlobalTermId>,
    ) -> Result<Option<GlobalTermId>, SegmentedError> {
        let size = match term {
            TermRef::Iri(iri) => iri.len().checked_add(5),
            TermRef::Blank { label, .. } => label.len().checked_add(9),
            TermRef::Literal {
                lexical, language, ..
            } => lexical
                .len()
                .checked_add(language.map_or(0, str::len))
                .and_then(|v| v.checked_add(if language.is_some() { 19 } else { 15 })),
            TermRef::Triple { .. } => Some(25),
        }
        .ok_or(SegmentedError::AddressExhausted)?;
        let _scratch = self.reserve_workspace(
            u64::try_from(size).map_err(|_| SegmentedError::AddressExhausted)?,
        )?;
        let mut key = Vec::with_capacity(size);
        format::encode_term(term, &mut key)?;
        debug_assert_eq!(key.len(), size);
        self.lookup_key(&key)
    }

    fn summary(&self, ordinal: u64) -> Result<[u8; format::SUMMARY_BYTES], SegmentedError> {
        if ordinal >= self.header.summaries.rows {
            return self.fail(SegmentedError::Corrupt(
                "summary outside certified ordinal range",
            ));
        }
        let per_block = (u64::from(self.header.block_bytes) - 9)
            / u64::try_from(format::SUMMARY_BYTES).expect("fixed summary width fits u64");
        let slot = self.header.summaries.first + ordinal / per_block;
        let index =
            usize::try_from(ordinal % per_block).map_err(|_| SegmentedError::AddressExhausted)?;
        let metadata = self.block(slot)?;
        let data = fixed_records(&metadata.encoded, format::SUMMARIES, format::SUMMARY_BYTES)?;
        data.get(index * format::SUMMARY_BYTES..(index + 1) * format::SUMMARY_BYTES)
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or(SegmentedError::Corrupt("missing summary ordinal"))
    }
    fn summary_range(
        &self,
        table: usize,
        subject: Option<GlobalTermId>,
    ) -> Result<core::ops::Range<u64>, SegmentedError> {
        let start = self.header.streams[..table]
            .iter()
            .map(|stream| stream.blocks)
            .sum::<u64>();
        let end = start + self.header.streams[table].blocks;
        let Some(subject) = subject else {
            return Ok(start..end);
        };
        let offset = format::partition_point(end - start, |mid| {
            let summary = self.summary(start + mid)?;
            let max =
                crate::bytes::read_u64_le(&summary, 32).expect("fixed summary subject maximum");
            Ok::<_, SegmentedError>(max < subject.index() + 1)
        })?;
        Ok(start + offset..end)
    }
    pub(super) const fn has_stream_rows(&self, table: usize) -> bool {
        self.header.streams[table].rows != 0
    }

    pub(super) fn stream(
        &self,
        table: usize,
        pattern: Pattern,
    ) -> impl Iterator<Item = QuadIds<GlobalTermId>> + '_ {
        self.stream_range(table, pattern, None)
    }

    pub(super) fn stream_range<'a>(
        &'a self,
        table: usize,
        pattern: Pattern,
        values: Option<&'a crate::AnnotationValueRange<'a>>,
    ) -> impl Iterator<Item = QuadIds<GlobalTermId>> + 'a {
        QuadIter {
            session: self,
            table,
            summaries: None,
            rows: None,
            row: 0..0,
            pattern,
            values,
        }
    }

    fn summary_values_may_match(
        &self,
        summary: &[u8],
        predicate: Option<GlobalTermId>,
        values: crate::AnnotationValueRange<'_>,
    ) -> Result<bool, SegmentedError> {
        let first = crate::bytes::read_u64_le(summary, 88)
            .ok_or(SegmentedError::Corrupt("value range start is missing"))?;
        let count = crate::bytes::read_u64_le(summary, 96)
            .ok_or(SegmentedError::Corrupt("value range count is missing"))?;
        let end = first
            .checked_add(count)
            .ok_or(SegmentedError::AddressExhausted)?;
        if end > self.header.value_ranges.rows {
            return Err(SegmentedError::Corrupt(
                "value range escapes certified directory",
            ));
        }
        let per = (u64::from(self.header.block_bytes) - 9) / format::VALUE_RANGE_BYTES as u64;
        for ordinal in first..end {
            // Copy only one fixed metadata record, then unpin the metadata before
            // any dictionary endpoint read. One-slot caches remain usable.
            let encoded: [u8; format::VALUE_RANGE_BYTES] = {
                let block = self.block(self.header.value_ranges.first + ordinal / per)?;
                let rows = fixed_records(
                    &block.encoded,
                    format::VALUE_RANGES,
                    format::VALUE_RANGE_BYTES,
                )?;
                let offset = (ordinal % per) as usize * format::VALUE_RANGE_BYTES;
                rows.get(offset..offset + format::VALUE_RANGE_BYTES)
                    .and_then(|bytes| bytes.try_into().ok())
                    .ok_or(SegmentedError::Corrupt("missing value interval"))?
            };
            let p = Reader::at(&encoded, 0).id()?;
            if predicate.is_some_and(|predicate| p != predicate) {
                continue;
            }
            let low = crate::bytes::read_u64_le(&encoded, 8).expect("fixed interval width");
            let high = crate::bytes::read_u64_le(&encoded, 16).expect("fixed interval width");
            let endpoints = match (low, high) {
                (0, 0) => None,
                (0, _) | (_, 0) => return Err(SegmentedError::Corrupt("half-open value summary")),
                _ => Some((
                    GlobalTermId::from_index(low - 1).map_err(StorageError::from)?,
                    GlobalTermId::from_index(high - 1).map_err(StorageError::from)?,
                )),
            };
            let mut storage = super::query::QueryStorage {
                reservation: self.reserve_workspace(0)?,
                cause: None,
            };
            let matches = storage.run(|memory| {
                crate::annotation_range::interval_may_match(self, endpoints, values, memory)
                    .map_err(|error| match error {
                        crate::annotation_range::RangeReadError::Read(error) => error,
                        crate::annotation_range::RangeReadError::Storage(error) => error.into(),
                    })
            });
            let matches = match matches {
                Ok(matches) => matches,
                Err(_)
                    if values
                        .workspace
                        .is_some_and(crate::AnnotationRangeWorkspace::stopped) =>
                {
                    return Ok(false);
                }
                Err(error) => return Err(error),
            };
            if matches {
                return Ok(true);
            }
        }
        Ok(false)
    }
    /// Current exact checkpoint evidence. Its request-prefix access performs no clone.
    #[must_use]
    pub fn evidence(&self) -> SegmentedEvidence {
        checkpoint(self.receipt.snapshot, &self.state, &self.budget, &self.base)
    }

    /// Stream RDF 1.2 TriG lines through the shared term emitter. Graph declaration
    /// lines preserve explicit empty graphs; all three RDF streams retain their own
    /// graph slots. The caller publishes its drain only after this returns `Ok`.
    ///
    /// # Errors
    /// Reserves the fixed sink window and worst overlapping lexical/traversal scratch
    /// before allocation. A read or drain failure refuses completion; partial bytes
    /// already handed to the caller's unpublished drain must be discarded.
    pub fn export_trig_lines(
        &self,
        drain: &mut dyn crate::sink::ByteDrain,
    ) -> Result<SegmentedEvidence, super::SegmentedExportError> {
        let scratch = self
            .header
            .max_owned_term_bytes
            .checked_mul(8)
            .and_then(|n| {
                n.checked_add(u64::try_from(crate::sink::DRAIN_BUFFER_BYTES + 4096).ok()?)
            })
            .ok_or(super::SegmentedExportError::Source(
                SegmentedError::AddressExhausted,
            ))?;
        let _reservation = self
            .reserve_workspace(scratch)
            .map_err(super::SegmentedExportError::Source)?;
        let mut output = crate::sink::TextSink::to_drain(drain);
        let write = |id, output: &mut crate::sink::TextSink<'_>| {
            crate::turtle::try_write_view_term(self, id, output).map_err(|error| match error {
                crate::TermLookupError::Read(error) => super::SegmentedExportError::Source(error),
                crate::TermLookupError::ForeignId => super::SegmentedExportError::Source(
                    SegmentedError::Corrupt("export term reference is invalid"),
                ),
            })
        };
        self.ready().map_err(super::SegmentedExportError::Source)?;
        for graph in self.named_graphs() {
            write(graph, &mut output)?;
            output.push_str(" {}\n");
            if output.failed() {
                break;
            }
        }
        for table in 0..3 {
            for quad in self.stream(table, (None, None, None, GraphMatch::Any)) {
                if let Some(graph) = quad.g {
                    write(graph, &mut output)?;
                    output.push_str(" { ");
                }
                for (index, id) in [quad.s, quad.p, quad.o].into_iter().enumerate() {
                    if index != 0 {
                        output.push(' ');
                    }
                    write(id, &mut output)?;
                }
                output.push_str(if quad.g.is_some() { " . }\n" } else { " .\n" });
                if output.failed() {
                    break;
                }
            }
            if output.failed() {
                break;
            }
        }
        self.ready().map_err(super::SegmentedExportError::Source)?;
        output
            .finish()
            .map_err(super::SegmentedExportError::Output)?;
        Ok(self.evidence())
    }
}

/// A dictionary block pin. Borrowed text cannot outlive this guard, and its block
/// stays charged even if a cache eviction removes the cache's own reference.
#[derive(Debug)]
pub struct SegmentedTermGuard {
    block: Arc<CachedBlock>,
    index: usize,
    budget: Arc<Budget>,
    _base: Arc<SegmentedReservation>,
}
impl TermGuard<GlobalTermId> for SegmentedTermGuard {
    fn term(&self) -> TermRef<'_, GlobalTermId> {
        let records = self
            .block
            .records()
            .expect("a term guard is minted only from a decoded dictionary block");
        format::decode_term(records.key(self.index))
            .expect("term bytes were checked before the immutable guard was minted")
    }
}
impl Drop for SegmentedTermGuard {
    fn drop(&mut self) {
        lock_read_state(&self.budget.state).pins -= 1;
    }
}

type Pattern = (
    Option<GlobalTermId>,
    Option<GlobalTermId>,
    Option<GlobalTermId>,
    GraphMatch<GlobalTermId>,
);
struct QuadIter<'a> {
    session: &'a SegmentedSession,
    table: usize,
    summaries: Option<core::ops::Range<u64>>,
    // Keep the authenticated allocation and the validated byte range together,
    // without a self-reference or repeated header/cache work between yields.
    rows: Option<Arc<CachedBlock>>,
    row: core::ops::Range<usize>,
    pattern: Pattern,
    values: Option<&'a crate::AnnotationValueRange<'a>>,
}
impl Iterator for QuadIter<'_> {
    type Item = QuadIds<GlobalTermId>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.values.is_some_and(|range| {
                range
                    .workspace
                    .is_some_and(crate::AnnotationRangeWorkspace::stopped)
            }) {
                return None;
            }
            if let Some(block) = &self.rows {
                while self.row.start < self.row.end {
                    let index = self.row.start;
                    self.row.start += 32;
                    let quad = match format::decode_quad(&block.encoded[index..index + 32]) {
                        Ok(quad) => quad,
                        Err(error) => {
                            let _ = self.session.fail::<()>(error);
                            return None;
                        }
                    };
                    let (s, p, o, g) = self.pattern;
                    if s.is_none_or(|id| id == quad.s)
                        && p.is_none_or(|id| id == quad.p)
                        && o.is_none_or(|id| id == quad.o)
                        && g.matches(quad.g)
                    {
                        return Some(quad);
                    }
                }
                // Release before summary lookup or the next payload admission:
                // a one-slot cache must not require two live row blocks.
                self.rows = None;
            }
            if self.session.ready().is_err() {
                return None;
            }
            if self.summaries.is_none() {
                self.summaries = match self.session.summary_range(self.table, self.pattern.0) {
                    Ok(range) => Some(range),
                    Err(error) => {
                        let _ = self.session.fail::<()>(error);
                        return None;
                    }
                };
            }
            let ordinal = self.summaries.as_mut()?.next()?;
            let summary = match self.session.summary(ordinal) {
                Ok(summary) => summary,
                Err(error) => {
                    let _ = self.session.fail::<()>(error);
                    return None;
                }
            };
            if self.pattern.0.is_some_and(|subject| {
                crate::bytes::read_u64_le(&summary, 24).is_some_and(|min| min > subject.index() + 1)
            }) {
                return None;
            }
            if summary[8] != u8::try_from(self.table).expect("three RDF streams")
                || !summary_matches(&summary, self.pattern)
            {
                continue;
            }
            let page =
                crate::bytes::read_u64_le(&summary, 0).expect("certified summary block position");
            if let Some(values) = self.values {
                match self
                    .session
                    .summary_values_may_match(&summary, self.pattern.1, *values)
                {
                    Ok(true) => {}
                    Ok(false) => continue,
                    Err(error) => {
                        let _ = self.session.fail::<()>(error);
                        return None;
                    }
                }
            }
            let Ok(block) = self.session.block(page) else {
                return None;
            };
            let data = match fixed_records(&block.encoded, format::QUADS, 32) {
                Ok(data) => data,
                Err(error) => {
                    let _ = self.session.fail::<()>(error);
                    return None;
                }
            };
            let start = data.as_ptr().addr() - block.encoded.as_ptr().addr();
            self.row = start..start + data.len();
            self.rows = Some(block);
        }
    }
}
fn summary_matches(summary: &[u8], (s, p, o, graph): Pattern) -> bool {
    let graph = match graph {
        GraphMatch::Any => None,
        GraphMatch::Default => Some(0),
        GraphMatch::Named(id) => Some(id.index() + 1),
    };
    for (axis, value) in [
        s.map(|id| id.index() + 1),
        p.map(|id| id.index() + 1),
        o.map(|id| id.index() + 1),
        graph,
    ]
    .into_iter()
    .enumerate()
    {
        if let Some(value) = value {
            let min = crate::bytes::read_u64_le(summary, 24 + axis * 16)
                .expect("certified fixed summary width");
            let max = crate::bytes::read_u64_le(summary, 32 + axis * 16)
                .expect("certified fixed summary width");
            if value < min || value > max {
                return false;
            }
        }
    }
    true
}

impl DatasetView for SegmentedSession {
    type Id = GlobalTermId;
    type ReadError = SegmentedError;
    type TermGuard<'a> = SegmentedTermGuard;
    type ProbePlan = ();
    fn snapshot_identity(&self) -> Result<Option<purrdf_hash::hex::Digest32>, Self::ReadError> {
        self.ready()?;
        Ok(Some(purrdf_hash::hex::Digest32::new(
            *self.receipt.snapshot.as_bytes(),
        )))
    }

    fn source_statement_count(&self) -> Result<Option<u128>, Self::ReadError> {
        self.ready()?;
        Ok(Some(
            self.header
                .streams
                .iter()
                .map(|stream| u128::from(stream.rows))
                .sum(),
        ))
    }
    fn statistics(&self) -> Option<&dyn crate::SnapshotStatistics<Self::Id>> {
        Some(self)
    }
    fn source_summary(
        &self,
        predicate: Self::Id,
        graph: GraphMatch<Self::Id>,
        stream: crate::StatisticsStream,
    ) -> Result<Option<crate::source::SourceSummary>, Self::ReadError> {
        Self::source_summary(self, predicate, graph, stream)
    }
    fn read_error(&self) -> Option<Self::ReadError> {
        self.ready().err()
    }
    fn reserve_workspace(
        &self,
        bytes: u64,
    ) -> Result<impl crate::WorkspaceReservation<Error = Self::ReadError> + '_, Self::ReadError>
    {
        Self::reserve_workspace(self, bytes)
    }
    fn max_owned_term_bytes(&self) -> Option<u64> {
        Some(self.header.max_owned_term_bytes)
    }
    fn reserve_owned_workspace(
        &self,
        bytes: u64,
    ) -> Option<Result<crate::OwnedWorkspaceReservation<Self::ReadError>, Self::ReadError>> {
        Some(owned_workspace(bytes, |bytes| {
            Self::reserve_workspace(self, bytes)
        }))
    }
    fn storage_live_budget(&self) -> Option<u64> {
        Some(self.limits.live_bytes)
    }
    fn resolve(&self, id: Self::Id) -> Result<Self::TermGuard<'_>, Self::ReadError> {
        // Reserve the guard permit before any block I/O or decode allocation.
        let mut state = lock_read_state(&self.budget.state);
        if state.pins >= self.budget.max_pins {
            drop(state);
            return self.fail(SegmentedError::PinnedBlocks);
        }
        state.pins += 1;
        drop(state);
        let (block, index) = match self.term_block(id) {
            Ok(term) => term,
            Err(error) => {
                lock_read_state(&self.budget.state).pins -= 1;
                return self.fail(error);
            }
        };
        Ok(SegmentedTermGuard {
            block,
            index,
            budget: Arc::clone(&self.budget),
            // An owned pin may outlive the session. Retaining this reservation
            // keeps all shared budget ownership conservatively charged too.
            _base: Arc::clone(&self.base),
        })
    }
    fn term_id_by_value(&self, value: &TermValue) -> Result<Option<Self::Id>, Self::ReadError> {
        self.lookup_value(value, 0)
            .or_else(|error| self.fail(error))
    }
    fn quads(&self) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.stream(0, (None, None, None, GraphMatch::Any))
    }
    fn quads_for_pattern(
        &self,
        s: Option<Self::Id>,
        p: Option<Self::Id>,
        o: Option<Self::Id>,
        g: GraphMatch<Self::Id>,
    ) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.stream(0, (s, p, o, g))
    }
    fn probe_plan(&self, _: bool, _: bool, _: bool, _: GraphMatch<Self::Id>) -> Self::ProbePlan {}
    fn quads_for_pattern_with_plan(
        &self,
        (): &Self::ProbePlan,
        s: Option<Self::Id>,
        p: Option<Self::Id>,
        o: Option<Self::Id>,
        g: GraphMatch<Self::Id>,
    ) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.quads_for_pattern(s, p, o, g)
    }
    fn cardinality_estimate(
        &self,
        s: Option<Self::Id>,
        p: Option<Self::Id>,
        o: Option<Self::Id>,
        g: GraphMatch<Self::Id>,
    ) -> u64 {
        let estimate = (|| {
            let mut total = 0_u64;
            // A bound subject admits a short certified range. Count that range
            // with the same bounded iterator the evaluator will consume, before
            // it reserves operator rows. This keeps dense physical pages from
            // inflating a selective join into an unaffordable Cartesian bound.
            // Unbound subjects retain the certified page-cardinality upper bound.
            if s.is_some() {
                for table in 0..3 {
                    for _ in self.stream(table, (s, p, o, g)) {
                        total = total
                            .checked_add(1)
                            .ok_or(SegmentedError::AddressExhausted)?;
                    }
                }
                if let Some(error) = self.read_error() {
                    return Err(error);
                }
                return Ok(total);
            }
            for table in 0..3 {
                for ordinal in self.summary_range(table, s)? {
                    let summary = self.summary(ordinal)?;
                    if s.is_some_and(|subject| {
                        crate::bytes::read_u64_le(&summary, 24)
                            .is_some_and(|min| min > subject.index() + 1)
                    }) {
                        break;
                    }
                    if summary_matches(&summary, (s, p, o, g)) {
                        total = total
                            .checked_add(
                                crate::bytes::read_u64_le(&summary, 16)
                                    .ok_or(SegmentedError::Corrupt("summary row count missing"))?,
                            )
                            .ok_or(SegmentedError::AddressExhausted)?;
                    }
                }
            }
            Ok::<_, SegmentedError>(total)
        })();
        match estimate {
            Ok(total) => total,
            Err(error) => {
                let _ = self.fail::<()>(error);
                0
            }
        }
    }
    fn term_count(&self) -> u64 {
        self.header.terms
    }
    fn workspace_candidate_bound(
        &self,
        s: Option<Self::Id>,
        p: Option<Self::Id>,
        o: Option<Self::Id>,
        g: GraphMatch<Self::Id>,
    ) -> Result<Option<u64>, Self::ReadError> {
        // This implementation's checked summary/range body already includes
        // all three physical tables. Count it once, preserving its sticky error.
        self.checked_read(|view| Some(view.cardinality_estimate(s, p, o, g)))
    }
    fn len_hint(&self) -> Option<u64> {
        Some(self.header.streams[0].rows)
    }
    fn triple_term_nesting_bound(&self) -> Option<usize> {
        Some(16)
    }
    fn capabilities(&self) -> RdfStoreCapabilities {
        RdfStoreCapabilities {
            named_graphs: self.header.graphs.rows != 0,
            quoted_triples: self.header.flags & 1 != 0,
            reifiers: self.header.flags & 2 != 0,
            annotations: self.header.flags & 4 != 0,
            ..RdfStoreCapabilities::plain_rdf()
        }
    }
    fn reifier_quads(&self) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.stream(1, (None, None, None, GraphMatch::Any))
    }
    fn reifier_quads_of(&self, reifier: Self::Id) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.stream(1, (Some(reifier), None, None, GraphMatch::Any))
    }
    fn annotations_of_with_graph(
        &self,
        reifier: Self::Id,
    ) -> impl Iterator<Item = (Self::Id, Self::Id, Option<Self::Id>)> + '_ {
        self.stream(2, (Some(reifier), None, None, GraphMatch::Any))
            .map(|row| (row.p, row.o, row.g))
    }
    fn annotation_quads(&self) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.stream(2, (None, None, None, GraphMatch::Any))
    }
    fn reifier_quads_in_graph(
        &self,
        graph: GraphMatch<Self::Id>,
    ) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.stream(1, (None, None, None, graph))
    }
    fn annotation_quads_in_graph(
        &self,
        graph: GraphMatch<Self::Id>,
    ) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.stream(2, (None, None, None, graph))
    }

    fn annotation_quads_for_range<'a>(
        &'a self,
        predicate: Option<Self::Id>,
        object: Option<Self::Id>,
        graph: GraphMatch<Self::Id>,
        range: &'a crate::AnnotationValueRange<'a>,
    ) -> impl Iterator<Item = QuadIds<Self::Id>> + 'a {
        self.stream_range(2, (None, predicate, object, graph), Some(range))
    }
    fn named_graphs(&self) -> impl Iterator<Item = Self::Id> + '_ {
        (0..self.header.graphs.rows).map_while(move |index| self.named_graph_at(index))
    }

    /// Membership in [`named_graphs`](DatasetView::named_graphs): a binary search of
    /// the graph records, which the image stores strictly ascending (checked when it
    /// is opened), so a probe reads `O(log n)` records rather than every block. A
    /// read fault latches as it does for enumeration and answers `false`.
    fn has_named_graph(&self, graph: Self::Id) -> bool {
        let (mut low, mut high) = (0, self.header.graphs.rows);
        while low < high {
            let middle = low + (high - low) / 2;
            let Some(id) = self.named_graph_at(middle) else {
                return false;
            };
            match id.cmp(&graph) {
                std::cmp::Ordering::Less => low = middle + 1,
                std::cmp::Ordering::Greater => high = middle,
                std::cmp::Ordering::Equal => return true,
            }
        }
        false
    }
}
impl FallibleDatasetView for SegmentedSession {
    type Error = SegmentedError;
    type Evidence = SegmentedEvidence;
    fn operation_status(&self) -> ViewOperationStatus<Self::Error, Self::Evidence> {
        let error = self.read_error();
        let evidence = self.evidence();
        match error {
            None => ViewOperationStatus::Ready { evidence },
            Some(error) => ViewOperationStatus::Failed { error, evidence },
        }
    }
}

#[cfg(test)]
mod operation_tests {
    use super::*;
    use crate::{SegmentedBuildLimits, SegmentedBuilder, SegmentedImage};

    #[test]
    fn escaped_request_retains_its_identity_and_original_charge() {
        let (image, _) = image("urn:escaped-request");
        let session = SegmentedSession::open(
            Arc::new(image.provider()),
            image.receipt(),
            SegmentedReadLimits::new(2_000_000, 1, 50_000, 1_000_000, 2),
        )
        .unwrap();
        assert!(size_of::<SegmentedRequest>() <= 24);
        let budget = Arc::downgrade(&session.budget);
        let retained = session.base.bytes
            + u64::try_from(Shared::<RequestIdentity>::allocation_layout().size()).unwrap();
        let evidence = session.evidence();
        {
            let state = lock_read_state(&session.state);
            let buffers = state.requests.capacity() * size_of::<SegmentedRequest>()
                + hash_table_retained_bound(&state.cache).unwrap()
                + hash_table_retained_bound(&state.dictionary_positions).unwrap();
            drop(state);
            assert_eq!(
                evidence.bookkeeping_bytes(),
                u64::try_from(buffers).unwrap()
            );
        }
        let request = evidence.with_requests(|requests| requests[0].try_clone().unwrap());
        let duplicate = request.try_clone().unwrap();
        assert_eq!(request, duplicate);
        assert!(std::ptr::eq(
            &raw const *request.snapshot,
            &raw const *duplicate.snapshot,
        ));
        drop(duplicate);
        assert_eq!(request.snapshot(), image.receipt().snapshot());
        assert!(request.completed());
        let debug = format!("{request:?}");
        assert!(debug.starts_with("SegmentedRequest { snapshot: SegmentedSnapshot("));
        assert!(!debug.contains("reservation"));
        drop(session);
        drop(evidence);
        let original = budget.upgrade().unwrap();
        assert_eq!(lock_read_state(&original.state).live, retained);
        drop(request);
        assert_eq!(lock_read_state(&original.state).live, 0);
        drop(original);
        assert!(budget.upgrade().is_none());
    }

    #[test]
    fn workspace_outlives_session_with_original_budget_control_charge() {
        use crate::WorkspaceReservation as _;
        let (image, _) = image("urn:workspace");
        let session = SegmentedSession::open(
            Arc::new(image.provider()),
            image.receipt(),
            SegmentedReadLimits::new(2_000_000, 1, 100, 1_000_000, 2),
        )
        .unwrap();
        let base = Arc::downgrade(&session.base);
        let base_bytes = session.base.bytes();
        let mut workspace = session.reserve_workspace(4096).unwrap();
        let mut owned = DatasetView::reserve_owned_workspace(&session, 8192)
            .unwrap()
            .unwrap();
        drop(session);
        let control = u64::try_from(size_of::<SegmentedReservation>()).unwrap();
        assert_eq!(workspace.live_bytes(), base_bytes + 4096 + 8192 + control);
        workspace.resize(0).unwrap();
        owned.resize(0).unwrap();
        assert_eq!(workspace.live_bytes(), base_bytes + control);
        drop(workspace);
        assert!(base.upgrade().is_some());
        drop(owned);
        assert!(base.upgrade().is_none());
    }

    #[test]
    fn canonical_reserved_refusal_retains_its_original_bounded_admission() {
        use crate::{CanonError, CanonHash, TermPosition, try_canonicalize_view};
        let (image, _) = image("urn:purrdf:rdfc:escaped");
        let session = image
            .open_session(SegmentedReadLimits::new(2_000_000, 1, 100, 1_000_000, 2))
            .unwrap();
        let budget = Arc::downgrade(&session.budget);
        let CanonError::ReservedVocabulary(refusal) =
            try_canonicalize_view(&session, CanonHash::Sha256).unwrap_err()
        else {
            panic!("the reserved IRI must be refused");
        };
        let duplicate = refusal.clone();
        assert!(std::ptr::eq(
            refusal.iri().as_ptr(),
            duplicate.iri().as_ptr()
        ));
        drop(session);
        let original = budget.upgrade().unwrap();
        let retained = lock_read_state(&original.state).live;
        assert!(retained > 0);
        assert_eq!(duplicate.iri(), "urn:purrdf:rdfc:escaped");
        assert_eq!(duplicate.position(), TermPosition::Subject);
        let captured = purrdf_testkit::error_capture::Capture::read(&duplicate);
        assert_eq!(captured.code, "canonical-reserved-vocabulary");
        assert!(matches!(
            captured.field("iri"),
            purrdf_hash::error::ErrorValue::Iri("urn:purrdf:rdfc:escaped")
        ));
        assert_eq!(
            captured.child("position").code,
            "canonical-position-subject"
        );
        drop(refusal);
        assert_eq!(lock_read_state(&original.state).live, retained);
        drop(duplicate);
        assert_eq!(lock_read_state(&original.state).live, 0);
        drop(original);
        assert!(budget.upgrade().is_none());
    }

    #[test]
    fn overflow_payload_stays_charged_until_the_last_escaped_guard() {
        let label = format!("urn:overflow:{}", "x".repeat(50_000));
        let (image, id) = image(&label);
        let session = image
            .open_session(SegmentedReadLimits::new(
                2_000_000, 1, 20_000, 10_000_000, 8,
            ))
            .unwrap();
        assert_eq!(session.header.block_bytes, 1024);
        let guard = session.resolve(id).unwrap();
        let before = session.evidence().request_count();
        let duplicate = session.resolve(id).unwrap();
        assert_eq!(session.evidence().request_count(), before);
        drop(duplicate);
        let budget = Arc::clone(&guard.budget);
        let retained = guard.block.retained_charge();
        assert!(retained >= label.len() as u64);
        drop(session);
        assert_eq!(guard.term(), TermRef::Iri(&label));
        assert!(lock_read_state(&budget.state).live >= retained);
        drop(guard);
        assert_eq!(lock_read_state(&budget.state).live, 0);
    }

    #[test]
    fn overflow_provider_failure_keeps_the_original_failed_request() {
        const FIRST: SegmentedError = SegmentedError::Provider {
            operation: "overflow fixture page",
            host_code: Some(73),
        };
        #[derive(Debug)]
        struct Failing {
            inner: super::super::SegmentedBytes,
            position: u64,
        }
        impl SegmentedProvider for Failing {
            fn snapshot(&self) -> SegmentedSnapshot {
                self.inner.snapshot()
            }
            fn byte_len(&self) -> u64 {
                self.inner.byte_len()
            }
            fn read_at(&self, position: u64, output: &mut [u8]) -> Result<(), SegmentedError> {
                if position == self.position {
                    return Err(FIRST);
                }
                self.inner.read_at(position, output)
            }
        }
        let (image, id) = image(&format!("urn:failure:{}", "x".repeat(20_000)));
        let header = Header::decode(image.bytes()).unwrap();
        let position = (header.overflow.first + 1) * u64::from(header.block_bytes);
        let session = SegmentedSession::open(
            Arc::new(Failing {
                inner: image.provider(),
                position,
            }),
            image.receipt(),
            SegmentedReadLimits::new(2_000_000, 2, 20_000, 10_000_000, 8),
        )
        .unwrap();
        assert!(matches!(session.resolve(id), Err(ref error) if *error == FIRST));
        assert_eq!(session.read_error(), Some(FIRST));
        session.evidence().with_requests(|requests| {
            let failed = requests.last().unwrap();
            assert_eq!(failed.position(), position);
            assert!(!failed.completed());
        });
        assert_eq!(lock_read_state(&session.budget.state).pins, 0);
        assert!(matches!(session.resolve(id), Err(ref error) if *error == FIRST));
    }

    fn image(label: &str) -> (SegmentedImage, GlobalTermId) {
        let limits = SegmentedBuildLimits::new(4096, 200_000, 4096, 1024, 4).unwrap();
        let mut builder = SegmentedBuilder::new(limits).unwrap();
        let mut id = None;
        builder
            .intern_batch(&[TermValue::iri(label)], |_, term| id = Some(term))
            .unwrap();
        let id = id.unwrap();
        builder
            .push_quad(QuadIds {
                s: id,
                p: id,
                o: id,
                g: None,
            })
            .unwrap();
        (builder.seal().unwrap(), id)
    }

    #[test]
    fn fault_between_ready_and_block_lock_preserves_the_first_cause() {
        const FIRST: SegmentedError = SegmentedError::Provider {
            operation: "interleaved first failure",
            host_code: Some(17),
        };
        #[derive(Debug)]
        struct InterleavedProvider {
            inner: super::super::SegmentedBytes,
            pending: Mutex<Option<std::sync::Weak<Mutex<State>>>>,
        }
        impl SegmentedProvider for InterleavedProvider {
            fn snapshot(&self) -> SegmentedSnapshot {
                let pending = lock_read_state(&self.pending).take();
                if let Some(state) = pending.and_then(|owner| owner.upgrade()) {
                    lock_read_state(&state).error = Some(FIRST);
                }
                self.inner.snapshot()
            }
            fn byte_len(&self) -> u64 {
                self.inner.byte_len()
            }
            fn read_at(&self, position: u64, output: &mut [u8]) -> Result<(), SegmentedError> {
                self.inner.read_at(position, output)
            }
        }
        let (image, _) = image("urn:interleaved-first-fault");
        for (cached, requests_tight) in [(false, true), (false, false), (true, true)] {
            let provider = Arc::new(InterleavedProvider {
                inner: image.provider(),
                pending: Mutex::new(None),
            });
            let mut session = SegmentedSession::open(
                provider.clone(),
                image.receipt(),
                SegmentedReadLimits::new(2_000_000, 2, 2000, 10_000_000, 4),
            )
            .unwrap();
            let before = session.evidence();
            if requests_tight {
                session.limits.requests = u32::try_from(before.request_count()).unwrap();
            } else {
                session.limits.io_bytes = before.io_bytes();
            }
            *lock_read_state(&provider.pending) = Some(Arc::downgrade(&session.state));
            // The provider callback deterministically latches another operation's
            // fault after ready inspected State and before block takes its lock.
            let slot = if cached {
                0
            } else {
                session.header.dictionary.first
            };
            assert!(matches!(session.block(slot), Err(error) if error == FIRST));
            assert_eq!(session.read_error(), Some(FIRST));
            assert_eq!(session.evidence().request_count(), before.request_count());
        }
    }

    #[test]
    fn reader_buffer_growth_reclaims_unpinned_blocks_before_refusal() {
        const CEILING: u64 = 2_000_000;
        let (image, _) = image("urn:reader-buffer-eviction");
        for grow_cache in [false, true] {
            let session = SegmentedSession::open(
                Arc::new(image.provider()),
                image.receipt(),
                SegmentedReadLimits::new(CEILING, 8, 2000, 10_000_000, 4),
            )
            .unwrap();
            let mut preload = vec![session.header.summaries.first];
            if grow_cache {
                preload.push(session.header.dictionary.first);
            }
            for slot in preload {
                drop(session.block(slot).unwrap());
            }
            {
                let mut state = lock_read_state(&session.state);
                // Fill the real ledger boundary independently of hash-table
                // bucket counts; both buffer-growth arms remain exercised.
                while state.requests.len() < state.requests.capacity() {
                    session.request(&mut state, 0, &mut [0]).unwrap();
                }
                let requests_full = state.requests.len() == state.requests.capacity();
                let cache_full = state.cache.len() == state.cache.capacity();
                drop(state);
                assert!(requests_full);
                assert_eq!(cache_full, grow_cache);
            }
            let before = session.evidence();
            // The ordinary row block fits exactly. A growing reader buffer also
            // needs room, and the original unpinned header can provide it.
            let charge = u64::from(session.receipt.block_bytes)
                + u64::try_from(size_of::<CachedBlock>() + 64).unwrap();
            let held = session
                .reserve_workspace(CEILING - before.live_bytes() - charge)
                .unwrap();
            let target = session.header.streams[0].first;
            drop(session.block(target).unwrap());
            let after = session.evidence();
            assert!(after.evictions() > before.evictions());
            assert_eq!(
                after.request_count() - before.request_count(),
                usize::try_from(session.receipt.leaf_count.trailing_zeros()).unwrap() + 1
            );
            assert!(after.peak_bytes() <= CEILING);
            assert_eq!(session.read_error(), None);
            drop(held);
        }
    }

    #[test]
    fn cache_slot_growth_counts_the_pending_block_before_allocating() {
        const CEILING: u64 = 2_000_000;
        let mut builder =
            SegmentedBuilder::new(SegmentedBuildLimits::new(4096, 200_000, 4096, 1024, 4).unwrap())
                .unwrap();
        let values: Vec<_> = (0..128)
            .map(|index| TermValue::iri(format!("urn:pending-cache:{index:03}")))
            .collect();
        let mut ids = Vec::new();
        builder.intern_batch(&values, |_, id| ids.push(id)).unwrap();
        for &id in &ids {
            builder
                .push_quad(QuadIds {
                    s: ids[0],
                    p: ids[0],
                    o: id,
                    g: None,
                })
                .unwrap();
        }
        let image = builder.seal().unwrap();
        for pin_header in [false, true] {
            let session = SegmentedSession::open(
                Arc::new(image.provider()),
                image.receipt(),
                SegmentedReadLimits::new(CEILING, 64, 2000, 10_000_000, 64),
            )
            .unwrap();
            let mut pinned = Vec::new();
            let mut offset = 0;
            loop {
                let state = lock_read_state(&session.state);
                let full = state.cache.len() == state.cache.capacity() && state.cache.len() >= 16;
                drop(state);
                if full {
                    break;
                }
                pinned.push(
                    session
                        .block(session.header.dictionary.first + offset)
                        .unwrap(),
                );
                offset += 1;
            }
            // Replaying the sole unpinned header gives the request ledger spare
            // capacity while restoring the full cache array and its pinned neighbors.
            assert!(lock_read_state(&session.state).evict_one().unwrap());
            drop(session.block(0).unwrap());
            let header_pin = pin_header.then(|| session.block(0).unwrap());
            let (old_layout, next_layout) = {
                let mut state = lock_read_state(&session.state);
                let shape = (state.cache.len(), state.cache.capacity());
                let required_requests = state.requests.len() + 32;
                let State {
                    storage, requests, ..
                } = &mut *state;
                storage
                    .run(|memory| memory.reserve(requests, required_requests))
                    .unwrap();
                let request_room = state.requests.capacity() - state.requests.len();
                assert_eq!(shape.0, shape.1);
                assert!(request_room > 8);
                let positions = hash_table_retained_bound(&state.dictionary_positions).unwrap();
                // These aligned four-ID fixture pages occupy one bucket each.
                let required = state.dictionary_positions.len() + 1;
                let next_positions = if required > state.dictionary_positions.capacity() {
                    hash_table_allocation_bound::<DictionaryPosition>(required).unwrap()
                } else {
                    positions
                };
                (
                    hash_table_retained_bound(&state.cache).unwrap() + positions,
                    hash_table_allocation_bound::<CacheEntry>(state.cache.len() + 1).unwrap()
                        + next_positions,
                )
            };
            let records = u64::from(session.header.records_per_block);
            let charge = u64::from(session.receipt.block_bytes) * (records + 1)
                + records
                    * u64::try_from(size_of::<(GlobalTermId, core::ops::Range<usize>)>()).unwrap()
                + u64::try_from(size_of::<CachedBlock>() + 64).unwrap();
            let before = session.evidence();
            let headroom = charge
                + if pin_header {
                    u64::try_from(next_layout - old_layout).unwrap()
                } else {
                    0
                };
            let held = session
                .reserve_workspace(CEILING - before.live_bytes() - headroom)
                .unwrap();
            drop(
                session
                    .block(session.header.dictionary.first + offset)
                    .unwrap(),
            );
            if pin_header {
                assert_eq!(session.evidence().evictions(), before.evictions());
            } else {
                assert!(session.evidence().evictions() > before.evictions());
            }
            assert!(session.evidence().peak_bytes() <= CEILING);
            assert_eq!(session.read_error(), None);
            assert_eq!(
                {
                    let state = lock_read_state(&session.state);
                    hash_table_retained_bound(&state.cache).unwrap()
                        + hash_table_retained_bound(&state.dictionary_positions).unwrap()
                },
                if pin_header { next_layout } else { old_layout }
            );
            drop(held);
            drop(header_pin);
            drop(pinned);
        }
    }

    #[test]
    fn composed_sources_share_cache_admission_and_qualified_requests() {
        let (first, first_id) = image("urn:first");
        let (second, second_id) = image("urn:second");
        assert_eq!(
            first_id, second_id,
            "segment-local IDs deliberately overlap"
        );
        let operation =
            SegmentedOperation::new(SegmentedReadLimits::new(2_000_000, 1, 2000, 10_000_000, 4))
                .unwrap();
        let a = operation
            .open(Arc::new(first.provider()), first.receipt())
            .unwrap();
        let opened = a.evidence();
        let b = operation
            .open(Arc::new(second.provider()), second.receipt())
            .unwrap();
        assert_eq!(a.term_value(first_id).unwrap(), TermValue::iri("urn:first"));
        assert_eq!(
            b.term_value(second_id).unwrap(),
            TermValue::iri("urn:second")
        );
        assert_eq!(a.term_value(first_id).unwrap(), TermValue::iri("urn:first"));
        let evidence = b.evidence();
        assert!(evidence.evictions() >= 3);
        assert!(evidence.peak_bytes() <= 2_000_000);
        assert!(evidence.request_count() > opened.request_count());
        evidence.with_requests(|requests| {
            assert!(
                requests
                    .iter()
                    .any(|r| r.snapshot() == first.receipt().snapshot())
            );
            assert!(
                requests
                    .iter()
                    .any(|r| r.snapshot() == second.receipt().snapshot())
            );
        });
        opened.with_requests(|requests| {
            assert!(
                requests
                    .iter()
                    .all(|r| r.snapshot() == first.receipt().snapshot())
            );
        });
        assert!(Arc::ptr_eq(&a.state, &b.state));
        assert!(Arc::ptr_eq(&a.budget, &b.budget));
    }

    #[test]
    fn second_source_exhaustion_is_first_fault_for_every_session() {
        let (first, first_id) = image("urn:first");
        let (second, _) = image("urn:second");
        let probe = SegmentedSession::open(
            Arc::new(first.provider()),
            first.receipt(),
            SegmentedReadLimits::new(2_000_000, 1, 2000, 10_000_000, 4),
        )
        .unwrap();
        let count = u32::try_from(probe.evidence().request_count()).unwrap();
        let operation =
            SegmentedOperation::new(SegmentedReadLimits::new(2_000_000, 1, count, 10_000_000, 4))
                .unwrap();
        let a = operation
            .open(Arc::new(first.provider()), first.receipt())
            .unwrap();
        assert!(matches!(
            operation.open(Arc::new(second.provider()), second.receipt()),
            Err(SegmentedError::EvidenceExhausted)
        ));
        assert_eq!(
            a.term_value(first_id),
            Err(crate::TermLookupError::Read(
                SegmentedError::EvidenceExhausted
            ))
        );
        assert!(matches!(
            a.operation_status(),
            ViewOperationStatus::Failed {
                error: SegmentedError::EvidenceExhausted,
                ..
            }
        ));
        // A later descriptor mismatch cannot replace the original work refusal.
        assert!(matches!(
            operation.open(Arc::new(second.provider()), first.receipt()),
            Err(SegmentedError::EvidenceExhausted)
        ));
        assert_eq!(a.evidence().request_count(), count as usize);
    }

    #[test]
    fn dictionary_positions_are_bound_to_their_certified_source() {
        let mut images = Vec::new();
        // Fix the two page populations independently of text compression: the
        // selected ID starts a five-record page but lies inside an eight-record
        // page that crosses the routing bucket boundary.
        for (width, records_per_block) in [(180, 5), (80, 8)] {
            let limits = SegmentedBuildLimits::new(20, 20_000, 0, 512, records_per_block)
                .unwrap()
                .with_first_term_index((1_u64 << 54) + 251)
                .unwrap();
            let mut builder = SegmentedBuilder::new(limits).unwrap();
            let values: Vec<_> = (0..10)
                .map(|index| {
                    let prefix = if index % 2 == 0 { "a" } else { "z" };
                    TermValue::iri(format!(
                        "https://example.org/{}/{index:02}",
                        prefix.repeat(width)
                    ))
                })
                .collect();
            let mut id = None;
            builder
                .intern_batch(&values, |offered, value| {
                    if offered == &values[5] {
                        id = Some(value);
                    }
                })
                .unwrap();
            images.push((builder.seal().unwrap(), id.unwrap(), values[5].clone()));
        }
        let operation =
            SegmentedOperation::new(SegmentedReadLimits::new(2_000_000, 2, 2000, 10_000_000, 4))
                .unwrap();
        let sessions: Vec<_> = images
            .iter()
            .map(|(image, _, _)| {
                operation
                    .open(Arc::new(image.provider()), image.receipt())
                    .unwrap()
            })
            .collect();
        // Load a separate page before pinning both cache slots below. Its
        // unpinned payload must be evicted while its routing bound survives.
        let other = GlobalTermId::from_index(sessions[0].header.first_term_index).unwrap();
        drop(sessions[0].resolve(other).unwrap());
        let pins: Vec<_> = sessions
            .iter()
            .zip(&images)
            .map(|(session, (_, id, _))| session.resolve(*id).unwrap())
            .collect();
        {
            let state = lock_read_state(&operation.state);
            let key = CacheKey {
                snapshot: images[0].0.receipt().snapshot(),
                slot: sessions[0].header.dictionary.first,
            };
            assert!(!state.cache.iter().any(|entry| entry.key == key));
            drop(state);
        }
        let before = operation.evidence(()).request_count();
        for index in [0, 1, 0, 1] {
            let (_, id, value) = &images[index];
            assert_eq!(sessions[index].term_value(*id).unwrap(), *value);
        }
        assert_eq!(operation.evidence(()).request_count(), before);
        let state = lock_read_state(&operation.state);
        let positions: Vec<_> = state
            .dictionary_positions
            .iter()
            .filter(|position| {
                position.first <= images[0].1.index() && images[0].1.index() < position.end
            })
            .collect();
        // One retained page crosses a 256-ID bucket boundary; both entries
        // must name the same physical owner without confusing the other image.
        assert_eq!(positions.len(), 3);
        assert_eq!(
            positions
                .iter()
                .filter(|position| position.key.snapshot == images[0].0.receipt().snapshot())
                .count(),
            1
        );
        assert_eq!(
            positions
                .iter()
                .filter(|position| position.key.snapshot == images[1].0.receipt().snapshot())
                .count(),
            2
        );
        assert!(
            state.dictionary_positions.len() > positions.len(),
            "routing bounds survive eviction of their payload pages"
        );
        drop(state);
        drop(pins);
    }
}
