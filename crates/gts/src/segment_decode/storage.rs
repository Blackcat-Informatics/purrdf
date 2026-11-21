// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The shared resolver's resident and caller-owned state boundaries.

use core::convert::Infallible;
use core::ops::Deref;
use std::sync::Arc;

use crate::FastMap;
use crate::dictionary::{TermKey, TermLease};
use crate::model::{Term, Triple3};

/// An original term owner held while its dependencies are being resolved.
#[derive(Clone, Debug)]
pub enum ResolutionTerm {
    /// Ownership supplied through the original resident streaming callback.
    Resident(Arc<Term>),
    /// A payload inseparable from its original admitted storage account.
    Stored(TermLease),
}

impl Deref for ResolutionTerm {
    type Target = Term;

    fn deref(&self) -> &Term {
        match self {
            Self::Resident(term) => term,
            Self::Stored(term) => term,
        }
    }
}

/// Where a term stands in the resolver's search for the triple terms that reach
/// themselves (GTS-SPEC §7.3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SearchPhase {
    /// Not yet visited by a search.
    #[default]
    Unsettled,
    /// Visited and still on the search stack of the running search.
    Open,
    /// Its component is decided; [`ComponentState::degrades`] answers it.
    Settled,
}

/// The per-term state of the search that decides which triple terms are
/// reachable from their own components.
///
/// A resolver stores it inside [`PendingResolution::component`] and reads it
/// back unchanged; its contents are opaque. A storage that persists a pending
/// continuation outside memory persists this value with it, through
/// [`Self::to_words`] and [`Self::from_words`], and must return exactly what
/// it was given: a storage that returns a default value instead is detected
/// by the resolver and refused.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ComponentState {
    phase: SearchPhase,
    degraded: bool,
    loops: bool,
    cursor: u8,
    order: u64,
    low: u64,
    below: Option<usize>,
    up: Option<usize>,
}

impl ComponentState {
    /// The fixed encoding of one state, for a storage that persists it.
    #[must_use]
    pub fn to_words(self) -> [u64; 5] {
        let flags = u64::from(self.phase as u8)
            | u64::from(self.degraded) << 2
            | u64::from(self.loops) << 3
            | u64::from(self.cursor) << 8;
        let link = |id: Option<usize>| id.map_or(0, |id| id as u64 + 1);
        [flags, self.order, self.low, link(self.below), link(self.up)]
    }

    /// The state encoded by [`Self::to_words`], or `None` for words no state
    /// encodes.
    #[must_use]
    pub fn from_words(words: [u64; 5]) -> Option<Self> {
        let [flags, order, low, below, up] = words;
        let phase = match flags & 0b11 {
            0 => SearchPhase::Unsettled,
            1 => SearchPhase::Open,
            2 => SearchPhase::Settled,
            _ => return None,
        };
        if flags >> 16 != 0 || (flags >> 4) & 0b1111 != 0 {
            return None;
        }
        let link = |word: u64| match word {
            0 => Some(None),
            word => usize::try_from(word - 1).ok().map(Some),
        };
        Some(Self {
            phase,
            degraded: flags >> 2 & 1 == 1,
            loops: flags >> 3 & 1 == 1,
            cursor: u8::try_from(flags >> 8).ok()?,
            order,
            low,
            below: link(below)?,
            up: link(up)?,
        })
    }

    /// Whether the term is a triple term that reaches itself, once settled.
    #[must_use]
    pub fn degrades(self) -> bool {
        self.phase == SearchPhase::Settled && self.degraded
    }

    pub(super) fn is_unsettled(self) -> bool {
        self.phase == SearchPhase::Unsettled
    }

    pub(super) fn is_open(self) -> bool {
        self.phase == SearchPhase::Open
    }

    pub(super) fn is_settled(self) -> bool {
        self.phase == SearchPhase::Settled
    }

    /// Enter the running search at discovery number `order`, above `below` on
    /// the search stack and below `up` on the search path.
    pub(super) fn open(order: u64, below: Option<usize>, up: Option<usize>) -> Self {
        Self {
            phase: SearchPhase::Open,
            order,
            low: order,
            below,
            up,
            ..Self::default()
        }
    }

    pub(super) fn settle(self, degraded: bool) -> Self {
        Self {
            phase: SearchPhase::Settled,
            degraded,
            ..self
        }
    }

    pub(super) fn order(self) -> u64 {
        self.order
    }

    pub(super) fn low(self) -> u64 {
        self.low
    }

    pub(super) fn with_low(self, low: u64) -> Self {
        Self { low, ..self }
    }

    pub(super) fn below(self) -> Option<usize> {
        self.below
    }

    pub(super) fn up(self) -> Option<usize> {
        self.up
    }

    pub(super) fn cursor(self) -> u8 {
        self.cursor
    }

    pub(super) fn advance(self) -> Self {
        Self {
            cursor: self.cursor + 1,
            ..self
        }
    }

    pub(super) fn loops(self) -> bool {
        self.loops
    }

    pub(super) fn with_loop(self) -> Self {
        Self {
            loops: true,
            ..self
        }
    }
}

/// One iterative dependency-resolution continuation, persisted with its term.
///
/// Construct one with [`Self::new`] and set the public fields: the struct is
/// `#[non_exhaustive]`, so an external struct literal or an exhaustive
/// destructuring does not compile and an implementer cannot silently drop a
/// field added later.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct PendingResolution {
    /// Original term, or absence after an incomplete consumer callback.
    pub term: Option<ResolutionTerm>,
    /// The parent to resume after resolving this entry's dependencies.
    pub parent: Option<usize>,
    /// The next dependency slot to inspect.
    pub next: u8,
    /// Whether this entry lies on the active dependency path.
    pub active: bool,
    /// The search state that decides whether the term reaches itself.
    pub component: ComponentState,
}

impl PendingResolution {
    /// A continuation that has inspected nothing: no parent, the first
    /// dependency slot, off the active path, in no component.
    #[must_use]
    pub fn new(term: Option<ResolutionTerm>) -> Self {
        Self {
            term,
            parent: None,
            next: 0,
            active: false,
            component: ComponentState::default(),
        }
    }
}

/// Current logical entries in one resolver's active state.
#[derive(Clone, Copy, Debug, Default)]
pub struct ResolutionCounts {
    /// Unfinished terms, including incomplete callbacks.
    pub pending: usize,
    /// Completed mappings to original consumer identifiers.
    pub resolved: usize,
    /// First reifier bindings retained for indirect quoted terms.
    pub reifiers: usize,
}

/// Caller-owned raw, pending and completed dictionary-resolution state.
///
/// Hosts own serialization of their concrete consumer identifiers. No new
/// conversion, serialization or ordering bound is imposed on `Id`. Keys and
/// first-binding semantics must survive eviction and reopening. A stored term
/// returned from `pending` retains its original admission owner while borrowed.
pub trait ResolutionStorage<Id: Copy> {
    /// Original storage, corruption, admission or cancellation failure.
    type Error;

    /// Read a pending term and its iterative continuation.
    ///
    /// # Errors
    /// Returns the original backend refusal; missing keys remain distinct.
    fn pending(&mut self, key: TermKey) -> Result<Option<PendingResolution>, Self::Error>;
    /// Persist one pending continuation before advancing the computation.
    ///
    /// # Errors
    /// Returns the original backend refusal without claiming publication.
    fn set_pending(&mut self, key: TermKey, value: PendingResolution) -> Result<(), Self::Error>;
    /// Release completed continuation state after its mapping is durable.
    ///
    /// # Errors
    /// Returns the original backend refusal.
    fn remove_pending(&mut self, key: TermKey) -> Result<(), Self::Error>;
    /// Find the next introduced identifier in increasing append order.
    ///
    /// Implementations must not collect the complete external dictionary to
    /// answer this operation. Identifiers already resolved may still be visited.
    ///
    /// # Errors
    /// Returns the original backend refusal.
    fn next_term(
        &mut self,
        segment: u64,
        after: Option<usize>,
    ) -> Result<Option<usize>, Self::Error>;
    /// Read an already completed target identifier.
    ///
    /// # Errors
    /// Returns the original backend refusal.
    fn resolved(&mut self, key: TermKey) -> Result<Option<Id>, Self::Error>;
    /// Persist the identifier returned by the actual consumer callback.
    ///
    /// # Errors
    /// Returns the original backend refusal.
    fn set_resolved(&mut self, key: TermKey, id: Id) -> Result<(), Self::Error>;
    /// Read the first accepted binding of an indirect quoted term.
    ///
    /// # Errors
    /// Returns the original backend refusal.
    fn first_binding(&mut self, key: TermKey) -> Result<Option<Triple3>, Self::Error>;
    /// Record a binding only when the key has no prior accepted binding.
    ///
    /// # Errors
    /// Returns the original backend refusal.
    fn bind_first(&mut self, key: TermKey, triple: Triple3) -> Result<(), Self::Error>;
    /// Release active state once all callbacks for this segment have succeeded.
    /// Persistent hosts may retain their durable records for reopening.
    ///
    /// # Errors
    /// Returns the original backend refusal before reporting completion.
    fn complete_segment(&mut self, segment: u64) -> Result<(), Self::Error>;
    /// Observe active logical entries without loading external payloads.
    fn counts(&self) -> ResolutionCounts;
}

impl<Id: Copy, S: ResolutionStorage<Id> + ?Sized> ResolutionStorage<Id> for &mut S {
    type Error = S::Error;

    fn pending(&mut self, key: TermKey) -> Result<Option<PendingResolution>, Self::Error> {
        (**self).pending(key)
    }
    fn set_pending(&mut self, key: TermKey, value: PendingResolution) -> Result<(), Self::Error> {
        (**self).set_pending(key, value)
    }
    fn remove_pending(&mut self, key: TermKey) -> Result<(), Self::Error> {
        (**self).remove_pending(key)
    }
    fn next_term(
        &mut self,
        segment: u64,
        after: Option<usize>,
    ) -> Result<Option<usize>, Self::Error> {
        (**self).next_term(segment, after)
    }
    fn resolved(&mut self, key: TermKey) -> Result<Option<Id>, Self::Error> {
        (**self).resolved(key)
    }
    fn set_resolved(&mut self, key: TermKey, id: Id) -> Result<(), Self::Error> {
        (**self).set_resolved(key, id)
    }
    fn first_binding(&mut self, key: TermKey) -> Result<Option<Triple3>, Self::Error> {
        (**self).first_binding(key)
    }
    fn bind_first(&mut self, key: TermKey, triple: Triple3) -> Result<(), Self::Error> {
        (**self).bind_first(key, triple)
    }
    fn complete_segment(&mut self, segment: u64) -> Result<(), Self::Error> {
        (**self).complete_segment(segment)
    }
    fn counts(&self) -> ResolutionCounts {
        (**self).counts()
    }
}

/// The original resolver's resident convenience storage.
#[doc(hidden)]
#[derive(Debug)]
pub struct ResidentResolutionStorage<Id> {
    pending: FastMap<TermKey, PendingResolution>,
    resolved: FastMap<TermKey, Id>,
    reifiers: FastMap<TermKey, Triple3>,
    keys: Vec<TermKey>,
    unordered: bool,
}

impl<Id> Default for ResidentResolutionStorage<Id> {
    fn default() -> Self {
        Self {
            pending: FastMap::default(),
            resolved: FastMap::default(),
            reifiers: FastMap::default(),
            keys: Vec::new(),
            unordered: false,
        }
    }
}

impl<Id: Copy> ResolutionStorage<Id> for ResidentResolutionStorage<Id> {
    type Error = Infallible;

    fn pending(&mut self, key: TermKey) -> Result<Option<PendingResolution>, Infallible> {
        Ok(self.pending.get(&key).cloned())
    }

    fn set_pending(&mut self, key: TermKey, value: PendingResolution) -> Result<(), Infallible> {
        if self.pending.insert(key, value).is_none() {
            self.unordered |= self.keys.last().is_some_and(|last| *last > key);
            self.keys.push(key);
        }
        Ok(())
    }

    fn remove_pending(&mut self, key: TermKey) -> Result<(), Infallible> {
        self.pending.remove(&key);
        Ok(())
    }

    fn next_term(
        &mut self,
        segment: u64,
        after: Option<usize>,
    ) -> Result<Option<usize>, Infallible> {
        if self.unordered {
            self.keys.sort_unstable();
            self.unordered = false;
        }
        let start = self.keys.partition_point(|key| {
            key.segment < segment
                || (key.segment == segment && after.is_some_and(|id| key.term <= id))
        });
        Ok(self
            .keys
            .get(start)
            .filter(|key| key.segment == segment)
            .map(|key| key.term))
    }

    fn resolved(&mut self, key: TermKey) -> Result<Option<Id>, Infallible> {
        Ok(self.resolved.get(&key).copied())
    }

    fn set_resolved(&mut self, key: TermKey, id: Id) -> Result<(), Infallible> {
        self.resolved.insert(key, id);
        Ok(())
    }

    fn first_binding(&mut self, key: TermKey) -> Result<Option<Triple3>, Infallible> {
        Ok(self.reifiers.get(&key).copied())
    }

    fn bind_first(&mut self, key: TermKey, triple: Triple3) -> Result<(), Infallible> {
        self.reifiers.entry(key).or_insert(triple);
        Ok(())
    }

    fn complete_segment(&mut self, _segment: u64) -> Result<(), Infallible> {
        self.pending.clear();
        self.resolved.clear();
        self.reifiers.clear();
        self.keys.clear();
        self.unordered = false;
        Ok(())
    }

    fn counts(&self) -> ResolutionCounts {
        ResolutionCounts {
            pending: self.pending.len(),
            resolved: self.resolved.len(),
            reifiers: self.reifiers.len(),
        }
    }
}
