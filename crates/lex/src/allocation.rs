// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Fallible physical buffer growth for native lexical parsers and compilers.
//!
//! Concrete shared and boxed allocations live in two confined storage modules.
//! The front-end algebra and the RDF kernel both use this lower home, so grammar
//! callers can report allocator refusal without a reverse dependency or unsafe
//! code. Their native payload owners keep admission live through deallocation.

// Only these storage primitives manipulate their own freshly allocated blocks.
// Scanners, parsers, and the admission/accounting body remain safe Rust.
#[allow(unsafe_code)]
mod boxed;
#[allow(unsafe_code)]
pub mod shared;
pub use boxed::{try_boxed, try_boxed_one};
pub use shared::{Shared, SharedAllocError, SharedCloneError, SharedOwned, SharedText, SharedView};
mod text;
pub use text::OwnedText;
mod ordered;
pub use ordered::{OrderedIter, OrderedRange, OrderedTable};

use core::alloc::Layout;
use core::fmt;
use std::string::String;
use std::vec::Vec;

/// A physical failure, separate from any native lexical language.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StorageError {
    /// A logical address space refused the requested count before mutation.
    Capacity(CapacityError),
    /// A concrete allocation layout or live-byte sum cannot be represented.
    SizeOverflow,
    /// A fallible allocation refused its already admitted destination.
    AllocationFailed,
    /// The caller's admission callback refused; it retains its original typed error.
    AdmissionFailed,
    /// A native Display implementation failed without a physical storage refusal.
    FormattingFailed,
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Capacity(error) => error.fmt(f),
            Self::SizeOverflow => f.write_str("native allocation layout overflow"),
            Self::AllocationFailed => f.write_str("native destination allocation failed"),
            Self::AdmissionFailed => f.write_str("native workspace admission refused"),
            Self::FormattingFailed => f.write_str("native diagnostic formatting failed"),
        }
    }
}

impl std::error::Error for StorageError {}

/// Exhaustion of one named logical space, distinct from physical allocation.
/// Counts use u128 so even an attempted count beyond a u64 space is reported
/// exactly, without wrapping or replacing the first failure with a layout error.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CapacityError {
    /// The table or address space whose limit was reached.
    pub space: CapacitySpace,
    // Inline bytes preserve the full counts without imposing u128 alignment on
    // every enclosing operational error. No allocation is needed on refusal.
    attempted: [u8; 16],
    limit: [u8; 16],
}

impl CapacityError {
    /// Retain the exact requested count and limit of one logical address space.
    #[must_use]
    pub const fn new(space: CapacitySpace, attempted: u128, limit: u128) -> Self {
        Self {
            space,
            attempted: attempted.to_le_bytes(),
            limit: limit.to_le_bytes(),
        }
    }

    /// The requested total count, including the attempted addition.
    #[must_use]
    pub const fn attempted(self) -> u128 {
        u128::from_le_bytes(self.attempted)
    }

    /// The largest representable count in that space.
    #[must_use]
    pub const fn limit(self) -> u128 {
        u128::from_le_bytes(self.limit)
    }

    /// Check the original producer's next count before it mutates storage.
    ///
    /// # Errors
    /// Returns the named space, exact attempted count and actual address limit.
    pub fn check(space: CapacitySpace, attempted: u128, limit: u128) -> Result<(), Self> {
        if attempted > limit {
            Err(Self::new(space, attempted, limit))
        } else {
            Ok(())
        }
    }

    /// Mint one logical slot after checking its exclusive table count.
    /// The requested count is computed in u128, even at a u64 boundary.
    ///
    /// # Errors
    /// Returns the exact next count and the producer's actual representation limit.
    pub fn slot(space: CapacitySpace, index: u64, limit: u64) -> Result<u64, Self> {
        Self::check(space, u128::from(index) + 1, u128::from(limit))?;
        Ok(index)
    }
}

impl fmt::Debug for CapacityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CapacityError")
            .field("space", &self.space)
            .field("attempted", &self.attempted())
            .field("limit", &self.limit())
            .finish()
    }
}

impl core::hash::Hash for CapacityError {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.space.hash(state);
        self.attempted().hash(state);
        self.limit().hash(state);
    }
}

/// The native logical spaces whose producers report exact capacity failures.
/// The closed identity is retained through certificates and language boundaries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CapacitySpace {
    /// Resident RDF terms.
    LocalTerms,
    /// Resident RDF text arena.
    LocalTermBytes,
    /// Global RDF terms.
    GlobalTerms,
    /// Global RDF text arena.
    GlobalTermBytes,
    /// Resident statement rows.
    Quads,
    /// Resident RDF reifier bindings.
    Reifiers,
    /// Resident RDF annotation rows.
    Annotations,
    /// Lazy statement permutation addresses.
    Permutation,
    /// Mutable delta terms.
    DeltaTerms,
    /// Dataset merge blank scopes.
    BlankScopes,
    /// Streaming import blank scopes.
    ImportBlankScopes,
    /// Provenance units.
    ProvenanceUnits,
    /// Provenance artifacts.
    Artifacts,
    /// Provenance origin sets.
    OriginSets,
    /// Datalog atomic terms.
    DatalogTerms,
    /// Datalog materialized rows.
    DatalogRows,
    /// Datalog relation partitions.
    DatalogPartitions,
    /// Datalog rules.
    DatalogRules,
    /// Datalog proof terms.
    DatalogProofs,
    /// Datalog compound nodes.
    DatalogNodes,
    /// Datalog metavariables.
    DatalogMetas,
    /// Datalog atomic symbols.
    DatalogSymbols,
    /// Resident graph-free asserted triples.
    Triples,
}

impl CapacitySpace {
    const NAMES: [(Self, &'static str); 23] = [
        (Self::LocalTerms, "rdf-local-terms"),
        (Self::LocalTermBytes, "rdf-local-text"),
        (Self::GlobalTerms, "rdf-global-terms"),
        (Self::GlobalTermBytes, "rdf-global-text"),
        (Self::Quads, "rdf-quads"),
        (Self::Reifiers, "rdf-reifiers"),
        (Self::Annotations, "rdf-annotations"),
        (Self::Permutation, "rdf-permutation"),
        (Self::DeltaTerms, "rdf-delta-terms"),
        (Self::BlankScopes, "rdf-merge-blank-scopes"),
        (Self::ImportBlankScopes, "rdf-import-blank-scopes"),
        (Self::ProvenanceUnits, "rdf-provenance-units"),
        (Self::Artifacts, "rdf-provenance-artifacts"),
        (Self::OriginSets, "rdf-origin-sets"),
        (Self::DatalogTerms, "datalog-terms"),
        (Self::DatalogRows, "datalog-rows"),
        (Self::DatalogPartitions, "datalog-partitions"),
        (Self::DatalogRules, "datalog-rules"),
        (Self::DatalogProofs, "datalog-proofs"),
        (Self::DatalogNodes, "datalog-nodes"),
        (Self::DatalogMetas, "datalog-metavariables"),
        (Self::DatalogSymbols, "datalog-symbols"),
        (Self::Triples, "rdf-triples"),
    ];

    /// Stable allocation-free identity for error publication.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        Self::NAMES[self as usize].1
    }

    /// Decode the same closed identity without allocating or leaking strings.
    #[must_use]
    pub fn of_name(name: &str) -> Option<Self> {
        Self::NAMES
            .iter()
            .find_map(|&(space, spelling)| (spelling == name).then_some(space))
    }
}

impl fmt::Display for CapacitySpace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for CapacityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} capacity exhausted: attempted {}, limit {}",
            self.space,
            self.attempted(),
            self.limit()
        )
    }
}

impl std::error::Error for CapacityError {}

crate::variant_from!(StorageError { Capacity(CapacityError) });

/// A borrowed caller's live-capacity admission for one native computation.
pub trait Admission {
    /// Admit the next exact total before allocation; shrink only after destruction.
    fn resize(&mut self, live_bytes: usize) -> Result<(), StorageError>;
}

/// A zero-size resident callback with no application capacity preset.
///
/// Accepts each exact native live layout; Memory still checks the layout and
/// performs the actual allocation fallibly. This supplies no borrowed external
/// grant, so resident callers own the returned payload themselves.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Resident;

impl Admission for Resident {
    fn resize(&mut self, _live_bytes: usize) -> Result<(), StorageError> {
        Ok(())
    }
}

impl<A: Admission + ?Sized> Admission for &mut A {
    fn resize(&mut self, live: usize) -> Result<(), StorageError> {
        (**self).resize(live)
    }
}

/// Copy text after the caller's original reservation policy admits its destination.
///
/// # Errors
/// Propagates the reservation policy's original failure before copying any bytes.
pub fn string_with_reserve<E>(
    input: &str,
    reserve: impl FnOnce(&mut String, usize) -> Result<(), E>,
) -> Result<String, E> {
    let mut text = String::new();
    reserve(&mut text, input.len())?;
    text.push_str(input);
    Ok(text)
}

/// Amortized array capacity shared by native vectors and UTF-8 buffers.
/// Empty buffers start with eight byte elements, four elements up to 1 KiB
/// each, or one larger element. Nonempty buffers double their existing capacity,
/// or use the required extent when it is larger. The returned concrete layout
/// must be admitted before allocating; this function itself allocates nothing.
///
/// # Errors
/// Returns capacity overflow before any destination allocation.
pub fn growth_capacity<T>(capacity: usize, required: usize) -> Result<usize, StorageError> {
    if required <= capacity {
        return Ok(capacity);
    }
    let first = if capacity != 0 {
        0
    } else if size_of::<T>() == 1 {
        8
    } else if size_of::<T>() <= 1024 {
        4
    } else {
        1
    };
    capacity
        .checked_mul(2)
        .map(|doubled| doubled.max(required).max(first))
        .ok_or(StorageError::SizeOverflow)
}

/// Exact replacement and previous array layouts for a native heap insertion.
/// The existing backing array remains live while the replacement is born.
///
/// # Errors
/// Returns count, capacity or layout overflow before allocation.
pub fn heap_insertion_layout<T: Ord>(
    heap: &std::collections::BinaryHeap<T>,
) -> Result<Option<(usize, usize)>, StorageError> {
    let required = heap
        .len()
        .checked_add(1)
        .ok_or(StorageError::SizeOverflow)?;
    if required <= heap.capacity() {
        return Ok(None);
    }
    let capacity = growth_capacity::<T>(heap.capacity(), required)?;
    let replacement = Layout::array::<T>(capacity)
        .map_err(|_| StorageError::SizeOverflow)?
        .size();
    let previous = Layout::array::<T>(heap.capacity())
        .map_err(|_| StorageError::SizeOverflow)?
        .size();
    Ok(Some((replacement, previous)))
}

/// Exact live-buffer accounting shared by native scanners, compilers and walks.
/// The borrowed callback itself is stack storage, never a retained payload owner.
/// After a native result leaves, its caller retains the original admission grant
/// with that result; use `admitted_bytes` only after construction scratch has died.
pub struct Memory<'a, S: Admission + ?Sized> {
    storage: &'a mut S,
    live: usize,
}

impl<'a, S: Admission + ?Sized> Memory<'a, S> {
    /// Lend the original account to a native carrier that owns a resumed Memory
    /// view. The returned total is that view's exact admitted layout counter,
    /// never a measurement of an already constructed payload. The carrier
    /// completes or destroys its scratch before returning the same counter.
    pub fn with_account<T>(&mut self, body: impl FnOnce(&mut S, usize) -> (T, usize)) -> T {
        let (value, admitted) = body(self.storage, self.live);
        self.live = admitted;
        value
    }
    /// Start one native computation with no owned heap capacity.
    /// All nonempty buffers passed to this memory must have been allocated through
    /// it, or explicitly included with `add_bytes` under the same original grant.
    #[must_use]
    pub fn new(storage: &'a mut S) -> Self {
        Self { storage, live: 0 }
    }

    /// Resume a native computation under the same original admission grant.
    /// `live_bytes` must be the requested layout total already owned by `storage`;
    /// the caller keeps those payloads alive until this memory adopts or releases
    /// them. This constructor allocates nothing and does not resize the grant.
    #[must_use]
    pub fn resume(storage: &'a mut S, live_bytes: usize) -> Self {
        Self {
            storage,
            live: live_bytes,
        }
    }

    /// The currently admitted requested layouts of payload and working buffers.
    /// Read the surviving total only after construction scratch has been released.
    #[must_use]
    pub const fn admitted_bytes(&self) -> usize {
        self.live
    }

    /// Run a native child body under this same original admission. On physical
    /// failure, the body's owned locals die before their original grant delta is
    /// released. Successful returned payloads keep their original grant unchanged.
    /// This never observes or adopts a resident factory's returned payload.
    ///
    /// # Errors
    /// Returns the body's first physical failure. Refund failure after that
    /// failure cannot replace it; successful child payload keeps its original grant.
    pub fn scope<T>(
        &mut self,
        body: impl FnOnce(&mut Self) -> Result<T, StorageError>,
    ) -> Result<T, StorageError> {
        self.try_scope(body)
    }

    /// Scope native construction while preserving its original typed domain error.
    /// Failed local payloads die before their original grant delta is released.
    ///
    /// # Errors
    /// Returns the first domain failure; a refund refusal never replaces it.
    /// Any returned error payload retains its own original owner independently
    /// of the released child buffers, just as a successful retained payload does.
    pub fn try_scope<T, E>(
        &mut self,
        body: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<T, E> {
        let baseline = self.live;
        match body(self) {
            Ok(value) => Ok(value),
            Err(original) => {
                // The body's locals have died. Attempt their original refund even
                // when a sticky caller now refuses shrink; the original operation
                // error remains authoritative. A failed refund keeps the existing
                // grant intact until its owner dies, never freeing it too early.
                if let Some(released) = self.live.checked_sub(baseline) {
                    let _ = self.release_bytes(released);
                }
                Err(original)
            }
        }
    }

    /// Borrow the original admission for a native payload factory above this leaf.
    pub fn admission_mut(&mut self) -> &mut S {
        self.storage
    }

    /// Admit a native payload's concrete layout before calling its fallible factory.
    ///
    /// # Errors
    /// Returns checked live-byte overflow or the caller's original refusal marker.
    pub fn add_bytes(&mut self, bytes: usize) -> Result<(), StorageError> {
        self.add(bytes)
    }

    /// Move an originally admitted array into another admitted array.
    /// Empty destinations reuse the original buffer; otherwise both buffers stay
    /// covered through growth and the source capacity is released after destruction.
    ///
    /// # Errors
    /// Returns checked layout, allocator or original admission refusal.
    pub fn append<T>(
        &mut self,
        destination: &mut Vec<T>,
        source: Vec<T>,
    ) -> Result<(), StorageError> {
        if destination.is_empty() {
            let previous = std::mem::replace(destination, source);
            return self.release_vec(previous);
        }
        let capacity = source.capacity();
        self.extend(destination, source)?;
        self.release_bytes(
            Layout::array::<T>(capacity)
                .map_err(|_| StorageError::SizeOverflow)?
                .size(),
        )
    }

    /// Release a concrete layout only after destroying its original allocation.
    ///
    /// # Errors
    /// Returns an unbalanced live-byte invariant or the caller's refusal marker.
    pub fn release_bytes(&mut self, bytes: usize) -> Result<(), StorageError> {
        self.remove(bytes)
    }

    fn add(&mut self, bytes: usize) -> Result<(), StorageError> {
        let next = self
            .live
            .checked_add(bytes)
            .ok_or(StorageError::SizeOverflow)?;
        if next != self.live {
            self.storage.resize(next)?;
        }
        self.live = next;
        Ok(())
    }

    fn remove(&mut self, bytes: usize) -> Result<(), StorageError> {
        let next = self
            .live
            .checked_sub(bytes)
            .ok_or(StorageError::SizeOverflow)?;
        if next != self.live {
            self.storage.resize(next)?;
        }
        self.live = next;
        Ok(())
    }

    /// Replace a vector buffer while both old and new layouts remain admitted.
    /// The old contents survive admission or allocator refusal unchanged. The buffer
    /// must already belong to this memory's live total if it is nonempty.
    ///
    /// # Errors
    /// Returns checked layout overflow, admission refusal or allocator refusal.
    pub fn reserve<T>(&mut self, values: &mut Vec<T>, required: usize) -> Result<(), StorageError> {
        if required <= values.capacity() {
            return Ok(());
        }
        let old = Layout::array::<T>(values.capacity())
            .map_err(|_| StorageError::SizeOverflow)?
            .size();
        let next = Layout::array::<T>(required)
            .map_err(|_| StorageError::SizeOverflow)?
            .size();
        self.add(next)?;
        let mut replacement = Vec::new();
        if replacement.try_reserve_exact(required).is_err() {
            drop(replacement);
            // Keep the allocator's original failure if a caller refuses refund.
            let _ = self.release_bytes(next);
            return Err(StorageError::AllocationFailed);
        }
        // Global Vec's exact reservation requests precisely the certified Layout.
        // Refuse a violated allocator/collection certificate before using it.
        if size_of::<T>() != 0 && replacement.capacity() != required {
            drop(replacement);
            let _ = self.release_bytes(next);
            return Err(StorageError::SizeOverflow);
        }
        replacement.append(values);
        drop(core::mem::replace(values, replacement));
        self.remove(old)
    }

    /// Admit a single boxed value before fallible physical construction.
    /// Its existing payload already belongs to this memory or an independent owner.
    ///
    /// # Errors
    /// Returns original admission, layout or allocator refusal.
    pub fn boxed<T>(&mut self, value: T) -> Result<Box<T>, StorageError> {
        let bytes = Layout::new::<T>().size();
        self.add_bytes(bytes)?;
        match try_boxed(value) {
            Ok(value) => Ok(value),
            Err(_) => {
                // The allocator refused first. A subsequent refund refusal
                // must not replace that original physical failure.
                let _ = self.release_bytes(bytes);
                Err(StorageError::AllocationFailed)
            }
        }
    }

    /// Freeze an originally admitted array without an infallible shrinking
    /// reallocation. If the array has spare capacity, its exact replacement is
    /// admitted while the original remains alive.
    ///
    /// # Errors
    /// Returns checked layout overflow, admission or allocator refusal.
    pub fn boxed_slice<T>(&mut self, mut values: Vec<T>) -> Result<Box<[T]>, StorageError> {
        if size_of::<T>() != 0 && values.capacity() != values.len() {
            let old = Layout::array::<T>(values.capacity())
                .map_err(|_| StorageError::SizeOverflow)?
                .size();
            let next = Layout::array::<T>(values.len())
                .map_err(|_| StorageError::SizeOverflow)?
                .size();
            self.add(next)?;
            let mut replacement = Vec::new();
            replacement
                .try_reserve_exact(values.len())
                .map_err(|_| StorageError::AllocationFailed)?;
            if replacement.capacity() != values.len() {
                return Err(StorageError::SizeOverflow);
            }
            replacement.append(&mut values);
            drop(values);
            self.remove(old)?;
            values = replacement;
        }
        // Exact capacity equals length, so this conversion reallocates nothing.
        Ok(values.into_boxed_slice())
    }

    /// Append one value after admitting and fallibly allocating any buffer growth.
    ///
    /// # Errors
    /// Returns checked layout overflow, admission refusal or allocator refusal.
    pub fn push<T>(&mut self, values: &mut Vec<T>, value: T) -> Result<(), StorageError> {
        self.reserve_for_push(values)?;
        values.push(value);
        Ok(())
    }

    /// Grow a native binary heap through the same admitted vector home.
    /// Admission and allocation refusal leave the original entries untouched.
    /// Successful replacement destroys the old buffer before refunding it.
    ///
    /// # Errors
    /// Returns checked layout, admission, allocator or refund refusal.
    pub fn push_heap<T: Ord>(
        &mut self,
        heap: &mut std::collections::BinaryHeap<T>,
        value: T,
    ) -> Result<(), StorageError> {
        if let Some((_, previous)) = heap_insertion_layout(heap)? {
            let required = heap
                .len()
                .checked_add(1)
                .ok_or(StorageError::SizeOverflow)?;
            let capacity = growth_capacity::<T>(heap.capacity(), required)?;
            let mut replacement = Vec::new();
            self.reserve(&mut replacement, capacity)?;
            let mut original = core::mem::take(heap).into_vec();
            replacement.append(&mut original);
            drop(original);
            self.release_bytes(previous)?;
            *heap = std::collections::BinaryHeap::from(replacement);
        }
        heap.push(value);
        Ok(())
    }

    /// Destroy the heap entries and backing buffer before refunding its layout.
    /// Element payloads keep their independent original ownership.
    ///
    /// # Errors
    /// Returns checked layout, unbalanced accounting or refund refusal.
    pub fn release_heap<T: Ord>(
        &mut self,
        heap: std::collections::BinaryHeap<T>,
    ) -> Result<(), StorageError> {
        self.release_vec(heap.into_vec())
    }

    /// Admit amortized vector growth before constructing the value to append.
    ///
    /// # Errors
    /// Returns checked layout overflow, admission refusal or allocator refusal.
    pub fn reserve_for_push<T>(&mut self, values: &mut Vec<T>) -> Result<(), StorageError> {
        let required = values
            .len()
            .checked_add(1)
            .ok_or(StorageError::SizeOverflow)?;
        if required > values.capacity() {
            let capacity = growth_capacity::<T>(values.capacity(), required)?;
            self.reserve(values, capacity)?;
        }
        Ok(())
    }

    /// Stably sort admitted payload slots through native admitted index buffers.
    /// Original indices break comparison ties; the unstable index sorter itself
    /// allocates nothing. Payloads stay in their original array throughout.
    ///
    /// # Errors
    /// Returns checked index layout, allocator or original admission refusal.
    pub fn sort_by<T>(
        &mut self,
        values: &mut [T],
        mut compare: impl FnMut(&T, &T) -> core::cmp::Ordering,
    ) -> Result<(), StorageError> {
        if values.len() < 2 {
            return Ok(());
        }
        let mut order = Vec::new();
        self.reserve(&mut order, values.len())?;
        order.extend(0..values.len());
        order.sort_unstable_by(|a, b| compare(&values[*a], &values[*b]).then(a.cmp(b)));
        let mut destinations = Vec::new();
        self.reserve(&mut destinations, values.len())?;
        destinations.resize(values.len(), 0);
        for (destination, original) in order.iter().copied().enumerate() {
            destinations[original] = destination;
        }
        self.release_vec(order)?;
        for first in 0..values.len() {
            while destinations[first] != first {
                let next = destinations[first];
                values.swap(first, next);
                destinations.swap(first, next);
            }
        }
        self.release_vec(destinations)
    }

    /// Collect native values through the same fallible vector growth home.
    /// Payloads supplied by the iterator retain their original admission.
    ///
    /// # Errors
    /// Returns checked layout, admission or physical allocation refusal.
    pub fn collect<T>(
        &mut self,
        values: impl IntoIterator<Item = T>,
    ) -> Result<Vec<T>, StorageError> {
        self.try_map(values, |value, _| Ok(value))
    }

    /// Map a source through one fallible admitted array and element factory.
    /// Its known extent is reserved before the first payload birth; uncertain
    /// extents use the same native amortized growth. Partial values die before
    /// failure cleanup. Retained element and error payloads keep their own owners.
    ///
    /// # Errors
    /// Returns the first element error, layout, admission or allocator refusal.
    pub fn try_map<T, U, E: From<StorageError>>(
        &mut self,
        values: impl IntoIterator<Item = T>,
        mut make: impl FnMut(T, &mut Self) -> Result<U, E>,
    ) -> Result<Vec<U>, E> {
        self.try_scope(|memory| {
            let values = values.into_iter();
            let mut output = Vec::new();
            memory.reserve(&mut output, values.size_hint().0)?;
            for value in values {
                memory.reserve_for_push(&mut output)?;
                output.push(make(value, memory)?);
            }
            Ok(output)
        })
    }

    /// Append native values without an infallible capacity-growth path.
    ///
    /// # Errors
    /// Returns checked layout, admission or physical allocation refusal.
    pub fn extend<T>(
        &mut self,
        output: &mut Vec<T>,
        values: impl IntoIterator<Item = T>,
    ) -> Result<(), StorageError> {
        for value in values {
            self.push(output, value)?;
        }
        Ok(())
    }

    /// Destroy a vector before releasing its original buffer's admission.
    /// Independently priced owned element payloads require their matching release;
    /// this method releases only the vector's concrete backing array layout.
    ///
    /// # Errors
    /// Returns layout overflow, an unbalanced live-byte invariant or refusal.
    pub fn release_vec<T>(&mut self, values: Vec<T>) -> Result<(), StorageError> {
        let bytes = Layout::array::<T>(values.capacity())
            .map_err(|_| StorageError::SizeOverflow)?
            .size();
        drop(values);
        self.remove(bytes)
    }

    /// Destroy an original vector and preserve the operation's first physical error.
    /// A refund refusal propagates when the operation succeeded. On an existing
    /// failure the original cause wins, while the original grant remains alive if
    /// its caller refuses shrink.
    ///
    /// # Errors
    /// Returns the operation's original error, otherwise its actual refund failure.
    pub fn release_vec_after<T, R>(
        &mut self,
        values: Vec<T>,
        result: Result<R, StorageError>,
    ) -> Result<R, StorageError> {
        let refund = self.release_vec(values);
        result.and_then(|value| refund.map(|()| value))
    }

    /// Destroy owned UTF-8 storage before releasing its original buffer admission.
    ///
    /// # Errors
    /// Returns an unbalanced live-byte invariant or the caller's refusal marker.
    pub fn release_string(&mut self, text: String) -> Result<(), StorageError> {
        let bytes = text.capacity();
        drop(text);
        self.remove(bytes)
    }

    /// Replace UTF-8 storage while old and new allocations both remain admitted.
    /// The old text survives admission or allocator refusal unchanged.
    ///
    /// # Errors
    /// Returns checked layout overflow, admission refusal or allocator refusal.
    pub fn reserve_string(
        &mut self,
        text: &mut String,
        required: usize,
    ) -> Result<(), StorageError> {
        if required <= text.capacity() {
            return Ok(());
        }
        let next = Layout::array::<u8>(required)
            .map_err(|_| StorageError::SizeOverflow)?
            .size();
        let old = text.capacity();
        self.add(next)?;
        let mut replacement = String::new();
        replacement
            .try_reserve_exact(required)
            .map_err(|_| StorageError::AllocationFailed)?;
        if replacement.capacity() != required {
            return Err(StorageError::SizeOverflow);
        }
        replacement.push_str(text);
        drop(core::mem::replace(text, replacement));
        self.remove(old)
    }

    /// Append borrowed UTF-8 after admitting any amortized buffer growth.
    /// The original text survives admission or allocator refusal unchanged.
    /// Its nonempty buffer must already belong to this memory's live total.
    ///
    /// # Errors
    /// Returns checked layout overflow, admission refusal or allocator refusal.
    pub fn push_str(&mut self, text: &mut String, suffix: &str) -> Result<(), StorageError> {
        let required = text
            .len()
            .checked_add(suffix.len())
            .ok_or(StorageError::SizeOverflow)?;
        if required > text.capacity() {
            let capacity = growth_capacity::<u8>(text.capacity(), required)?;
            self.reserve_string(text, capacity)?;
        }
        text.push_str(suffix);
        Ok(())
    }

    /// Append one scalar after admitting any physical UTF-8 buffer growth.
    ///
    /// # Errors
    /// Returns checked layout overflow, admission refusal or allocator refusal.
    pub fn push_char(&mut self, text: &mut String, ch: char) -> Result<(), StorageError> {
        let mut utf8 = [0; 4];
        self.push_str(text, ch.encode_utf8(&mut utf8))
    }

    /// Append through the shared lexical emitters, admitting physical growth
    /// before allocation and retaining the first failed fragment's cause.
    ///
    /// # Errors
    /// Returns checked layout overflow, admission refusal or allocator refusal.
    pub fn append_text(
        &mut self,
        text: &mut String,
        emit: impl FnOnce(&mut dyn crate::text_out::TextOut),
    ) -> Result<(), StorageError> {
        let mut sink = FormatSink {
            memory: self,
            text,
            failure: None,
        };
        emit(&mut sink);
        sink.failure.map_or(Ok(()), Err)
    }

    /// Render borrowed native fields once, admitting each actual UTF-8 growth
    /// before allocation. The Display implementation must itself allocate no
    /// payload storage; this sink owns the rendered destination.
    ///
    /// # Errors
    /// Preserves physical failures and distinguishes a formatter's own failure.
    pub fn format(&mut self, value: &(impl fmt::Display + ?Sized)) -> Result<String, StorageError> {
        use core::fmt::Write;
        let mut text = String::new();
        let mut sink = FormatSink {
            memory: self,
            text: &mut text,
            failure: None,
        };
        match write!(&mut sink, "{value}") {
            Ok(()) => Ok(text),
            Err(_) => Err(sink.failure.unwrap_or(StorageError::FormattingFailed)),
        }
    }

    /// Copy borrowed text into an admitted, fallibly allocated UTF-8 destination.
    ///
    /// # Errors
    /// Returns checked layout overflow, admission refusal or allocator refusal.
    pub fn string(&mut self, input: &str) -> Result<String, StorageError> {
        string_with_reserve(input, |text, required| self.reserve_string(text, required))
    }
}

impl<S: Admission + ?Sized> fmt::Debug for Memory<'_, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Memory")
            .field("live_bytes", &self.live)
            .finish_non_exhaustive()
    }
}

struct FormatSink<'m, 'a, S: Admission + ?Sized> {
    memory: &'m mut Memory<'a, S>,
    text: &'m mut String,
    failure: Option<StorageError>,
}

impl<S: Admission + ?Sized> fmt::Write for FormatSink<'_, '_, S> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        if self.failure.is_some() {
            return Err(fmt::Error);
        }
        let result = self.memory.push_str(self.text, text);
        result.map_err(|failure| {
            self.failure = Some(failure);
            fmt::Error
        })
    }
}

impl<S: Admission + ?Sized> crate::text_out::TextOut for FormatSink<'_, '_, S> {
    fn push_str(&mut self, text: &str) {
        // TextOut records failures in its sink; append_text returns that cause.
        let _ = fmt::Write::write_str(self, text);
    }

    fn push(&mut self, ch: char) {
        let mut utf8 = [0; 4];
        crate::text_out::TextOut::push_str(self, ch.encode_utf8(&mut utf8));
    }

    fn failed(&self) -> bool {
        self.failure.is_some()
    }
}

#[cfg(test)]
mod resumed_memory_regressions {
    use super::{Admission, Memory, StorageError};

    #[test]
    fn capacity_counts_preserve_full_width_numeric_identity_without_wide_alignment() {
        use super::{CapacityError, CapacitySpace};
        use core::hash::{Hash, Hasher};
        use purrdf_hash::fixed::FixedHasher;

        const MAXIMUM: CapacityError =
            CapacityError::new(CapacitySpace::GlobalTerms, u128::MAX, u128::MAX);
        const COUNT: u128 = MAXIMUM.attempted();
        const LIMIT: u128 = MAXIMUM.limit();
        assert_eq!((COUNT, LIMIT), (u128::MAX, u128::MAX));
        assert!(align_of::<CapacityError>() <= align_of::<u64>());
        for attempted in [0, u128::from(u64::MAX) + 1, u128::MAX] {
            for limit in [0, u128::from(u64::MAX), u128::MAX] {
                let error = CapacityError::new(CapacitySpace::GlobalTerms, attempted, limit);
                assert_eq!(error.attempted(), attempted);
                assert_eq!(error.limit(), limit);
                assert_eq!(
                    format!("{error:?}"),
                    format!(
                        "CapacityError {{ space: GlobalTerms, attempted: {attempted}, limit: {limit} }}"
                    )
                );
                assert_eq!(
                    error.to_string(),
                    format!(
                        "{} capacity exhausted: attempted {attempted}, limit {limit}",
                        error.space
                    )
                );
                let mut observed = FixedHasher::default();
                error.hash(&mut observed);
                let mut original = FixedHasher::default();
                error.space.hash(&mut original);
                attempted.hash(&mut original);
                limit.hash(&mut original);
                assert_eq!(observed.finish(), original.finish());
                assert_eq!(
                    CapacityError::check(error.space, attempted, limit),
                    if attempted > limit {
                        Err(error)
                    } else {
                        Ok(())
                    }
                );
            }
        }
    }

    #[derive(Default)]
    struct OriginalGrant {
        live: usize,
        limit: usize,
        attempted: usize,
    }
    impl Admission for OriginalGrant {
        fn resize(&mut self, live: usize) -> Result<(), StorageError> {
            self.attempted = live;
            if live > self.limit {
                return Err(StorageError::AdmissionFailed);
            }
            self.live = live;
            Ok(())
        }
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn failed_array_allocation_refunds_only_the_replacement_layout() {
        let mut grant = OriginalGrant {
            limit: usize::MAX,
            ..OriginalGrant::default()
        };
        let mut original = Vec::<u8>::new();
        let mut memory = Memory::new(&mut grant);
        memory.reserve(&mut original, 13).unwrap();
        original.extend_from_slice(b"original");
        // A valid Layout whose native allocation cannot fit the address space.
        // Vec reports allocator refusal without an infallible allocation or OOM.
        let impossible = usize::try_from(isize::MAX).unwrap();
        assert_eq!(
            memory.reserve(&mut original, impossible),
            Err(StorageError::AllocationFailed)
        );
        assert_eq!(original, b"original");
        assert_eq!(original.capacity(), 13);
        assert_eq!(memory.admitted_bytes(), 13);
        assert_eq!(memory.admission_mut().live, 13);
        memory.reserve(&mut original, 32).unwrap();
        assert_eq!(original, b"original");
        assert_eq!(memory.admitted_bytes(), 32);
        memory.release_vec(original).unwrap();
        assert_eq!(memory.admitted_bytes(), 0);
    }

    #[test]
    fn resume_preserves_original_payload_during_refused_buffer_growth() {
        let mut grant = OriginalGrant {
            limit: 44,
            ..OriginalGrant::default()
        };
        let mut original = Vec::<u8>::new();
        let live = {
            let mut memory = Memory::new(&mut grant);
            memory
                .reserve(&mut original, 13)
                .expect("original admitted array");
            memory.admitted_bytes()
        };
        assert_eq!(live, 13);
        let mut next = Vec::<u8>::new();
        {
            let mut memory = Memory::resume(&mut grant, live);
            assert_eq!(
                memory.reserve(&mut next, 32),
                Err(StorageError::AdmissionFailed)
            );
            assert_eq!(memory.admitted_bytes(), live);
            assert_eq!(next.capacity(), 0, "refused before allocating replacement");
        }
        assert_eq!(
            grant.attempted, 45,
            "original and new requested layouts overlap"
        );
        assert_eq!(grant.live, 13, "original payload stayed covered");
        let mut memory = Memory::resume(&mut grant, live);
        memory
            .release_vec(original)
            .expect("payload dies before its original admission");
        assert_eq!(memory.admitted_bytes(), 0);
    }
}

#[cfg(test)]
mod original_scope_regressions {
    use super::{Admission, Memory, StorageError};
    use std::cell::Cell;

    struct DropWitness<'a>(&'a Cell<bool>);
    impl Drop for DropWitness<'_> {
        fn drop(&mut self) {
            self.0.set(true);
        }
    }
    struct Grant<'a> {
        live: usize,
        scope_floor: usize,
        destroyed: &'a Cell<bool>,
    }
    impl Admission for Grant<'_> {
        fn resize(&mut self, next: usize) -> Result<(), StorageError> {
            if next < self.live && next == self.scope_floor {
                assert!(
                    self.destroyed.get(),
                    "the original child allocation must die before its grant is released"
                );
            }
            self.live = next;
            Ok(())
        }
    }

    #[test]
    fn native_failed_child_dies_before_refund_without_releasing_parent() {
        let destroyed = Cell::new(false);
        let mut grant = Grant {
            live: 0,
            scope_floor: 0,
            destroyed: &destroyed,
        };
        let mut memory = Memory::new(&mut grant);
        let parent = memory.string("original parent payload").unwrap();
        let parent_bytes = memory.admitted_bytes();
        memory.admission_mut().scope_floor = parent_bytes;
        let result: Result<(), StorageError> = memory.scope(|memory| {
            let _child = memory.boxed(DropWitness(&destroyed))?;
            let _text = memory.string("temporary original child string")?;
            Err(StorageError::AllocationFailed)
        });
        assert_eq!(result, Err(StorageError::AllocationFailed));
        assert!(destroyed.get());
        assert_eq!(memory.admitted_bytes(), parent_bytes);
        assert_eq!(parent, "original parent payload");
        memory.admission_mut().scope_floor = 0;
        memory.release_string(parent).unwrap();
        assert_eq!(memory.admitted_bytes(), 0);
    }

    #[test]
    fn native_success_keeps_original_output_grant_after_scope_returns() {
        let destroyed = Cell::new(true);
        let mut grant = Grant {
            live: 0,
            scope_floor: 0,
            destroyed: &destroyed,
        };
        let mut memory = Memory::new(&mut grant);
        let output = memory
            .scope(|memory| memory.string("successful original payload"))
            .unwrap();
        assert_eq!(memory.admitted_bytes(), output.capacity());
        memory.release_string(output).unwrap();
        assert_eq!(memory.admitted_bytes(), 0);
    }
}

#[cfg(test)]
mod original_failure_cleanup_regressions {
    use super::{Admission, Memory, StorageError};
    use core::cell::Cell;

    struct Payload<'a> {
        destroyed: &'a Cell<bool>,
    }
    impl Drop for Payload<'_> {
        fn drop(&mut self) {
            self.destroyed.set(true);
        }
    }
    struct RefuseShrink<'a> {
        destroyed: &'a Cell<bool>,
        live: usize,
        attempted: Option<usize>,
    }
    impl Admission for RefuseShrink<'_> {
        fn resize(&mut self, next: usize) -> Result<(), StorageError> {
            if next < self.live {
                assert!(
                    self.destroyed.get(),
                    "original payload dies before the refund callback"
                );
                self.attempted = Some(next);
                return Err(StorageError::AdmissionFailed);
            }
            self.live = next;
            Ok(())
        }
    }

    #[test]
    fn native_scope_preserves_checked_layout_failure_when_shrink_refuses() {
        let destroyed = Cell::new(false);
        let mut storage = RefuseShrink {
            destroyed: &destroyed,
            live: 0,
            attempted: None,
        };
        let (baseline, surviving, result) = {
            let mut memory = Memory::new(&mut storage);
            let parent = memory.string("parent").unwrap();
            let baseline = memory.admitted_bytes();
            let result: Result<(), StorageError> = memory.scope(|memory| {
                let mut children = Vec::new();
                memory.push(
                    &mut children,
                    Payload {
                        destroyed: &destroyed,
                    },
                )?;
                let mut impossible = Vec::<u64>::new();
                memory.reserve(&mut impossible, usize::MAX)?;
                Ok(())
            });
            assert_eq!(parent, "parent", "original outside payload remains intact");
            let surviving = memory.admitted_bytes();
            drop(parent);
            (baseline, surviving, result)
        };
        assert_eq!(
            result,
            Err(StorageError::SizeOverflow),
            "first concrete native layout error remains authoritative"
        );
        assert!(destroyed.get());
        assert_eq!(storage.attempted, Some(baseline));
        assert_eq!(
            storage.live, surviving,
            "failed shrink retains the original grant rather than releasing it early"
        );
    }

    #[test]
    fn array_refund_preserves_original_error_after_destroying_actual_elements() {
        let destroyed = Cell::new(false);
        let mut storage = RefuseShrink {
            destroyed: &destroyed,
            live: 0,
            attempted: None,
        };
        let result = {
            let mut memory = Memory::new(&mut storage);
            let mut values = Vec::new();
            memory
                .push(
                    &mut values,
                    Payload {
                        destroyed: &destroyed,
                    },
                )
                .unwrap();
            let mut impossible = Vec::<u64>::new();
            let original = memory.reserve(&mut impossible, usize::MAX);
            memory.release_vec_after(values, original)
        };
        assert_eq!(result, Err(StorageError::SizeOverflow));
        assert!(destroyed.get());
        assert_eq!(storage.attempted, Some(0));
    }

    #[test]
    fn array_refund_failure_is_hard_when_original_operation_succeeded() {
        let destroyed = Cell::new(false);
        let mut storage = RefuseShrink {
            destroyed: &destroyed,
            live: 0,
            attempted: None,
        };
        let result = {
            let mut memory = Memory::new(&mut storage);
            let mut values = Vec::new();
            memory
                .push(
                    &mut values,
                    Payload {
                        destroyed: &destroyed,
                    },
                )
                .unwrap();
            memory.release_vec_after(values, Ok(37))
        };
        assert_eq!(result, Err(StorageError::AdmissionFailed));
        assert!(destroyed.get());
        assert_eq!(storage.attempted, Some(0));
    }
}
