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

/// One iterative dependency-resolution continuation, persisted with its term.
#[derive(Clone, Debug)]
pub struct PendingResolution {
    /// Original term, or absence after an incomplete consumer callback.
    pub term: Option<ResolutionTerm>,
    /// The parent to resume after resolving this entry's dependencies.
    pub parent: Option<usize>,
    /// The next dependency slot to inspect.
    pub next: u8,
    /// Whether this entry lies on the active dependency path.
    pub active: bool,
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
