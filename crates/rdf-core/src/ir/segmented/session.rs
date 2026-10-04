// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Sparse, authenticated block admission with exact bounded evidence and live pins.

use crate::dataset_view::lock_read_state;
use std::sync::{Arc, Mutex};

use crate::dataset_view::{
    DatasetView, FallibleDatasetView, GraphMatch, TermGuard, ViewOperationStatus,
};
use crate::{GlobalTermId, QuadIds, RdfStoreCapabilities, TermRef, TermValue};
use purrdf_hash::blake3::Hasher;

use super::build::fixed_records;
use super::format::{self, DecodedRecords, Header, Reader, Stream};
use super::{SegmentedError, SegmentedProvider, SegmentedReceipt, SegmentedSnapshot};

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentedRequest {
    position: u64,
    bytes: u32,
    completed: bool,
}
impl SegmentedRequest {
    /// Global encoded byte position, independent of pointer width.
    #[must_use]
    pub const fn position(self) -> u64 {
        self.position
    }
    /// Requested local range length.
    #[must_use]
    pub const fn bytes(self) -> u32 {
        self.bytes
    }
    /// Whether the provider filled that range successfully.
    #[must_use]
    pub const fn completed(self) -> bool {
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
        })
    }
}

/// A pre-allocation live-byte reservation. The allocation it admits must not grow
/// past `bytes()`, and must be released before the reservation is dropped.
#[derive(Debug)]
pub struct SegmentedReservation {
    budget: Arc<Budget>,
    bytes: u64,
}
impl SegmentedReservation {
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
        if bytes <= self.bytes {
            self.shrink(bytes);
            return Ok(());
        }
        let growth = bytes - self.bytes;
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
impl Drop for SegmentedReservation {
    fn drop(&mut self) {
        lock_read_state(&self.budget.state).live -= self.bytes;
    }
}

#[derive(Debug)]
struct CachedBlock {
    encoded: Vec<u8>,
    decoded: Option<DecodedRecords>,
    _reservation: SegmentedReservation,
}
#[derive(Debug)]
struct CacheEntry {
    slot: u64,
    block: Arc<CachedBlock>,
}
struct State {
    cache: Vec<CacheEntry>,
    requests: Vec<SegmentedRequest>,
    error: Option<SegmentedError>,
    io_bytes: u64,
    evictions: u64,
    hash: Hasher,
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

/// Snapshot evidence shares the bounded ledger allocation. Its exact prefix and
/// digest are immutable even if the live session subsequently appends requests.
/// Cloning/checkpointing evidence does not clone or expand the request vector.
#[derive(Debug, Clone)]
pub struct SegmentedEvidence {
    snapshot: SegmentedSnapshot,
    state: Arc<Mutex<State>>,
    prefix: usize,
    hash: [u8; 32],
    io_bytes: u64,
    live_bytes: u64,
    peak_bytes: u64,
    evictions: u64,
    _base: Arc<SegmentedReservation>,
}
impl PartialEq for SegmentedEvidence {
    fn eq(&self, other: &Self) -> bool {
        if self.snapshot != other.snapshot
            || self.prefix != other.prefix
            || self.hash != other.hash
            || self.io_bytes != other.io_bytes
            || self.live_bytes != other.live_bytes
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
impl Eq for SegmentedEvidence {}
impl SegmentedEvidence {
    /// Immutable representation named by every request.
    #[must_use]
    pub const fn snapshot(&self) -> SegmentedSnapshot {
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

/// A certified immutable snapshot, read through one bounded operational session.
/// Metadata and translations are not expanded into whole-dataset arrays. Every
/// cached object and returned pin retains its live reservation through eviction.
#[derive(Debug)]
pub struct SegmentedSession {
    provider: Arc<dyn SegmentedProvider>,
    receipt: SegmentedReceipt,
    limits: SegmentedReadLimits,
    header: Header,
    state: Arc<Mutex<State>>,
    budget: Arc<Budget>,
    base: Arc<SegmentedReservation>,
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
        let cache_count =
            usize::try_from(limits.cache_blocks).map_err(|_| SegmentedError::AddressExhausted)?;
        let requests =
            usize::try_from(limits.requests).map_err(|_| SegmentedError::AddressExhausted)?;
        if cache_count == 0 {
            return Err(SegmentedError::PinnedBlocks);
        }
        let base_bytes =
            u64::try_from(size_of::<Self>() + size_of::<State>() + size_of::<Budget>() + 192)
                .map_err(|_| SegmentedError::AddressExhausted)?
                .checked_add(
                    u64::from(limits.cache_blocks)
                        * u64::try_from(size_of::<CacheEntry>())
                            .map_err(|_| SegmentedError::AddressExhausted)?,
                )
                .and_then(|v| {
                    v.checked_add(
                        u64::from(limits.requests)
                            * u64::try_from(size_of::<SegmentedRequest>()).ok()?,
                    )
                })
                .ok_or(SegmentedError::AddressExhausted)?;
        if base_bytes > limits.live_bytes {
            return Err(SegmentedError::Residency {
                requested: base_bytes,
                limit: limits.live_bytes,
            });
        }
        // Reserve before creating the session's heap owners or fixed capacities.
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
        });
        let state = Arc::new(Mutex::new(State {
            cache: Vec::with_capacity(cache_count),
            requests: Vec::with_capacity(requests),
            error: None,
            io_bytes: 0,
            evictions: 0,
            hash: {
                let mut hash = Hasher::new();
                hash.update(super::SEGMENTED_REQUESTS_DOMAIN.as_bytes());
                hash
            },
        }));
        let mut session = Self {
            provider,
            receipt: receipt.clone(),
            limits,
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
                flags: 0,
            },
            state,
            budget,
            base,
        };
        let header = session.block(0)?;
        let decoded = Header::decode(&header.encoded)?;
        decoded.validate_addresses(receipt.block_count)?;
        if decoded.block_bytes != receipt.block_bytes {
            return Err(SegmentedError::Corrupt(
                "receipt/header block size mismatch",
            ));
        }
        session.header = decoded;
        Ok(session)
    }

    fn fail<T>(&self, error: SegmentedError) -> Result<T, SegmentedError> {
        let mut state = lock_read_state(&self.state);
        let error = state.error.get_or_insert(error).clone();
        drop(state);
        Err(error)
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

    /// Reserve query scratch, output staging, or a host batch BEFORE allocating it.
    /// Storage cache admission observes the same ceiling while this guard is alive.
    ///
    /// # Errors
    /// Refuses a sticky read failure or capacity blocked by live cache/pins.
    pub fn reserve_workspace(&self, bytes: u64) -> Result<SegmentedReservation, SegmentedError> {
        self.ready()?;
        let mut state = lock_read_state(&self.state);
        self.make_room(&mut state, bytes, false)?;
        self.budget.reserve(bytes).inspect_err(|error| {
            state.error = Some(error.clone());
        })
    }

    fn make_room(&self, state: &mut State, charge: u64, slot: bool) -> Result<(), SegmentedError> {
        loop {
            let enough = lock_read_state(&self.budget.state)
                .live
                .checked_add(charge)
                .is_some_and(|live| live <= self.limits.live_bytes);
            if enough
                && (!slot
                    || state.cache.len()
                        < usize::try_from(self.limits.cache_blocks)
                            .map_err(|_| SegmentedError::AddressExhausted)?)
            {
                return Ok(());
            }
            if let Some(index) = state
                .cache
                .iter()
                .position(|entry| Arc::strong_count(&entry.block) == 1)
            {
                state.cache.remove(index);
                state.evictions = state
                    .evictions
                    .checked_add(1)
                    .ok_or(SegmentedError::AddressExhausted)?;
            } else {
                let error = if state.cache.is_empty() {
                    SegmentedError::Residency {
                        requested: lock_read_state(&self.budget.state)
                            .live
                            .saturating_add(charge),
                        limit: self.limits.live_bytes,
                    }
                } else {
                    SegmentedError::PinnedBlocks
                };
                state.error.get_or_insert_with(|| error.clone());
                return Err(error);
            }
        }
    }

    fn request(
        &self,
        state: &mut State,
        position: u64,
        output: &mut [u8],
    ) -> Result<(), SegmentedError> {
        if state.requests.len() == state.requests.capacity() {
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
        state.requests.push(SegmentedRequest {
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
        state.hash.update(&position.to_le_bytes());
        state.hash.update(&bytes.to_le_bytes());
        state.hash.update(&[u8::from(result.is_ok())]);
        result
    }

    fn block(&self, slot: u64) -> Result<Arc<CachedBlock>, SegmentedError> {
        self.ready()?;
        if slot >= self.receipt.block_count {
            return self.fail(SegmentedError::Corrupt("block outside certified directory"));
        }
        let mut state = lock_read_state(&self.state);
        if let Some(index) = state.cache.iter().position(|entry| entry.slot == slot) {
            let entry = state.cache.remove(index);
            let block = Arc::clone(&entry.block);
            state.cache.push(entry);
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
            .is_none_or(|count| count > state.requests.capacity())
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
        let in_reverse = slot >= self.header.reverse.first
            && slot < self.header.reverse.first + self.header.reverse.blocks;
        let records = if slot != 0 && (in_dictionary || in_reverse) {
            u64::from(self.header.records_per_block)
        } else {
            0
        };
        let decoded = encoded
            .checked_mul(records)
            .ok_or(SegmentedError::AddressExhausted)?;
        let entries = records
            .checked_mul(
                u64::try_from(size_of::<(GlobalTermId, core::ops::Range<usize>)>())
                    .map_err(|_| SegmentedError::AddressExhausted)?,
            )
            .ok_or(SegmentedError::AddressExhausted)?;
        let overhead = u64::try_from(size_of::<CachedBlock>() + 64)
            .map_err(|_| SegmentedError::AddressExhausted)?;
        let charge = encoded
            .checked_add(decoded)
            .and_then(|v| v.checked_add(entries))
            .and_then(|v| v.checked_add(overhead))
            .ok_or(SegmentedError::AddressExhausted)?;
        self.make_room(&mut state, charge, true)?;
        let mut reservation = self.budget.reserve(charge)?;
        let mut bytes = vec![
            0_u8;
            usize::try_from(self.receipt.block_bytes)
                .map_err(|_| SegmentedError::AddressExhausted)?
        ];
        let result = (|| {
            let position = slot
                .checked_mul(encoded)
                .ok_or(SegmentedError::AddressExhausted)?;
            self.request(&mut state, position, &mut bytes)?;
            let mut digest = format::leaf_digest(slot, &bytes);
            let mut node = self
                .receipt
                .leaf_count
                .checked_add(slot)
                .ok_or(SegmentedError::AddressExhausted)?;
            let data_bytes = self
                .receipt
                .block_count
                .checked_mul(encoded)
                .ok_or(SegmentedError::AddressExhausted)?;
            while node > 1 {
                let sibling_position = (node ^ 1)
                    .checked_mul(32)
                    .and_then(|v| v.checked_add(data_bytes))
                    .ok_or(SegmentedError::AddressExhausted)?;
                let mut sibling = [0_u8; 32];
                self.request(&mut state, sibling_position, &mut sibling)?;
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
            let decoded_records = match bytes[0] {
                format::DICTIONARY | format::REVERSE if slot != 0 => Some(DecodedRecords::decode(
                    &bytes,
                    bytes[0],
                    self.header.records_per_block,
                    decoded,
                )?),
                _ => None,
            };
            let actual = encoded
                + overhead
                + decoded_records.as_ref().map_or(0, |records| {
                    u64::try_from(records.arena.capacity()).expect("local arena capacity fits u64")
                        + u64::try_from(
                            records.entries.capacity()
                                * size_of::<(GlobalTermId, core::ops::Range<usize>)>(),
                        )
                        .expect("local entries capacity fits u64")
                });
            reservation.shrink(actual);
            Ok(Arc::new(CachedBlock {
                encoded: bytes,
                decoded: decoded_records,
                _reservation: reservation,
            }))
        })();
        match result {
            Ok(block) => {
                state.cache.push(CacheEntry {
                    slot,
                    block: Arc::clone(&block),
                });
                Ok(block)
            }
            Err(error) => {
                state.error.get_or_insert_with(|| error.clone());
                drop(state);
                Err(error)
            }
        }
    }

    fn term_block(&self, id: GlobalTermId) -> Result<(Arc<CachedBlock>, usize), SegmentedError> {
        let local = id
            .index()
            .checked_sub(self.header.first_term_index)
            .ok_or(SegmentedError::Corrupt("foreign term ID"))?;
        if local >= self.header.terms {
            return self.fail(SegmentedError::Corrupt("foreign term ID"));
        }
        let slot = self.header.dictionary.first + local / u64::from(self.header.records_per_block);
        let index = usize::try_from(local % u64::from(self.header.records_per_block))
            .map_err(|_| SegmentedError::AddressExhausted)?;
        let block = self.block(slot)?;
        let records = block.decoded.as_ref().ok_or(SegmentedError::Corrupt(
            "forward dictionary block kind changed",
        ))?;
        if records
            .entries
            .get(index)
            .is_none_or(|(stored, _)| *stored != id)
        {
            return self.fail(SegmentedError::Corrupt(
                "forward dictionary ordinal changed",
            ));
        }
        format::decode_term(records.key(index))?;
        Ok((block, index))
    }

    fn lookup_key(&self, key: &[u8]) -> Result<Option<GlobalTermId>, SegmentedError> {
        let mut lo = 0_u64;
        let mut hi = self.header.reverse.blocks;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let block = self.block(self.header.reverse.first + mid)?;
            let records = block
                .decoded
                .as_ref()
                .ok_or(SegmentedError::Corrupt("reverse index block kind changed"))?;
            if records.key(0) <= key {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        if lo == 0 {
            return Ok(None);
        }
        let block = self.block(self.header.reverse.first + lo - 1)?;
        let records = block
            .decoded
            .as_ref()
            .ok_or(SegmentedError::Corrupt("reverse index block kind changed"))?;
        let index = records
            .entries
            .binary_search_by(|(_, range)| records.arena[range.clone()].cmp(key));
        Ok(index.ok().map(|index| records.entries[index].0))
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
        let mut lo = start;
        let mut hi = end;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let summary = self.summary(mid)?;
            let max =
                crate::bytes::read_u64_le(&summary, 32).expect("fixed summary subject maximum");
            if max < subject.index() + 1 {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        Ok(lo..end)
    }
    fn stream(&self, table: usize, pattern: Pattern) -> QuadIter<'_> {
        QuadIter {
            session: self,
            table,
            summaries: None,
            rows: None,
            row: 0,
            pattern,
        }
    }
    /// Current exact checkpoint evidence. Its request-prefix access performs no clone.
    #[must_use]
    pub fn evidence(&self) -> SegmentedEvidence {
        let state = lock_read_state(&self.state);
        let residency = lock_read_state(&self.budget.state);
        SegmentedEvidence {
            snapshot: self.receipt.snapshot,
            state: Arc::clone(&self.state),
            prefix: state.requests.len(),
            hash: *state.hash.finalize().as_bytes(),
            io_bytes: state.io_bytes,
            live_bytes: residency.live,
            peak_bytes: residency.peak,
            evictions: state.evictions,
            _base: Arc::clone(&self.base),
        }
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
            .decoded
            .as_ref()
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
    rows: Option<Arc<CachedBlock>>,
    row: usize,
    pattern: Pattern,
}
impl Iterator for QuadIter<'_> {
    type Item = QuadIds<GlobalTermId>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.session.ready().is_err() {
            return None;
        }
        loop {
            if let Some(block) = &self.rows {
                let data = match fixed_records(&block.encoded, format::QUADS, 32) {
                    Ok(data) => data,
                    Err(error) => {
                        let _ = self.session.fail::<()>(error);
                        return None;
                    }
                };
                while self.row < data.len() / 32 {
                    let index = self.row;
                    self.row += 1;
                    let quad = match format::decode_quad(&data[index * 32..index * 32 + 32]) {
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
                self.rows = None;
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
            self.rows = match self.session.block(page) {
                Ok(block) => Some(block),
                Err(_) => return None,
            };
            self.row = 0;
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
