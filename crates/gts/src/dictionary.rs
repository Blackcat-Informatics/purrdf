// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Caller-owned storage and pinned ownership for segment-local dictionaries.
//!
//! Storage keys preserve the original segment and append-order term id. The
//! portable engine opens no files: a host supplies storage and keeps its native
//! storage and admission failures in the original error chain.

use core::fmt;
use core::ops::Deref;
use purrdf_hash::error::IoFailure;
use purrdf_lex::allocation::{SharedOwned, StorageError};

use crate::model::{Term, Triple3};

mod spill;
pub use spill::{DictionaryResidence, ResidentTermStorage, SpillPolicy, SpillingDictionary};

/// The original identity of one dictionary entry, independent of residency.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TermKey {
    /// Zero-based segment index; also the blank-node identity boundary.
    pub segment: u64,
    /// Append-order identifier within that segment, including snapshot deltas.
    pub term: usize,
}

/// Explicit source-window and retained-metadata admission for a stored reader.
///
/// Metadata entries are checked before a decoded frame is accepted as retained
/// reader state. The current frame is a separate bounded temporary window.
/// Each retained value originates in an encoded/decoded frame of these bounds;
/// this is a cardinality contract, not an asserted exact allocator-byte count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DictionaryLimits {
    /// Maximum encoded source bytes in one header or frame.
    pub encoded_frame_bytes: usize,
    /// Maximum output of each frame/blob transform stage.
    pub decoded_frame_bytes: usize,
    /// Maximum retained reader-side metadata entries across accepted frames.
    pub metadata_entries: usize,
}

/// Settled reader-side table observations, separate from caller storage/output.
#[derive(Clone, Copy, Debug, Default)]
pub struct ReaderMetadata {
    /// Total logical retained entries under the metadata policy.
    pub entries: usize,
    /// Retained per-frame identities in the active segment.
    pub frame_ids: usize,
    /// Allocated outer frame-identity vector slots.
    pub frame_capacity: usize,
    /// Retained provenance subject states.
    pub provenance_subjects: usize,
    /// Available provenance hash-table entry capacity.
    pub provenance_capacity: usize,
}

/// An original storage failure or a violated dictionary invariant.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum DictionaryError {
    /// Original native allocation or caller-admission failure.
    Storage(StorageError),
    /// Original host I/O failure, including its typed native cause.
    Io(IoFailure),
    /// Original integrity or decode failure in previously admitted replay bytes.
    Replay(purrdf_lex::cbor::DecodeError),
    /// An admitted dictionary entry is absent from caller storage.
    Missing(TermKey),
    /// Explicitly bounded live workspace could not admit the next operation.
    Limit {
        /// The bounded resource that refused the operation.
        resource: &'static str,
        /// Required quantity that could not be admitted.
        required: usize,
        /// Caller-selected maximum, inclusive.
        limit: usize,
    },
    /// The selected sink does not implement admitted term ownership.
    UnsupportedSink,
    /// Stored or indexed bytes no longer describe their admitted entry.
    Corrupt(&'static str),
}

impl fmt::Display for DictionaryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Storage(cause) => cause.fmt(f),
            Self::Io(cause) => cause.fmt(f),
            Self::Replay(cause) => cause.fmt(f),
            Self::Missing(key) => write!(
                f,
                "dictionary entry {} in segment {} is missing",
                key.term, key.segment
            ),
            Self::Limit {
                resource,
                required,
                limit,
            } => write!(f, "dictionary {resource} needs {required}, limit {limit}"),
            Self::UnsupportedSink => {
                f.write_str("the selected GTS sink does not accept admitted term leases")
            }
            Self::Corrupt(reason) => write!(f, "invalid dictionary storage: {reason}"),
        }
    }
}

impl std::error::Error for DictionaryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Storage(cause) => Some(cause),
            Self::Io(cause) => Some(cause),
            Self::Replay(cause) => Some(cause),
            _ => None,
        }
    }
}

impl purrdf_hash::error::StructuredError for DictionaryError {
    purrdf_hash::error_visit! {
        Storage(cause) => "gts-dictionary-storage" { "cause" => Cause(cause) },
        Io(cause) => "gts-dictionary-io" { "cause" => Cause(cause) },
        Replay(cause) => "gts-dictionary-replay" { "cause" => Cause(cause) },
        Missing(key) => "gts-dictionary-missing" {
            "segment" => Unsigned(u128::from(key.segment)),
            "term" => Unsigned(key.term as u128)
        },
        Limit { resource, required, limit } => "gts-dictionary-limit" {
            "resource" => Code(resource), "required" => Unsigned(*required as u128),
            "limit" => Unsigned(*limit as u128)
        },
        UnsupportedSink => "gts-dictionary-sink" {},
        Corrupt(reason) => "gts-dictionary-corrupt" { "reason" => Code(reason) },
    }
}

purrdf_lex::variant_from!(DictionaryError {
    Storage(StorageError),
    Io(IoFailure),
});

impl From<std::io::Error> for DictionaryError {
    fn from(cause: std::io::Error) -> Self {
        Self::Io(cause.into())
    }
}

/// A decoded term and the original account admitting its complete allocation.
///
/// Evicting an entry from a cache cannot release its admission while a consumer
/// still holds this lease or a clone. No independently escapable payload owner
/// is exposed. Borrowed fields remain tied to the lease.
#[derive(Clone, Debug)]
pub struct TermLease {
    value: SharedOwned<Term>,
    bytes: usize,
}

impl TermLease {
    /// Complete concrete storage of a term and its retained admission owner.
    ///
    /// # Errors
    /// Refuses an unrepresentable live-byte sum before allocation.
    pub fn allocation_bytes<O>(term: &Term) -> Result<usize, StorageError> {
        [
            term.value.as_ref(),
            term.lang.as_ref(),
            term.direction.as_ref(),
        ]
        .into_iter()
        .flatten()
        .try_fold(
            SharedOwned::<Term>::allocation_layout::<O>().size(),
            |sum, text| {
                sum.checked_add(text.capacity())
                    .ok_or(StorageError::SizeOverflow)
            },
        )
    }

    /// Publish a term after the host has admitted this exact allocation.
    ///
    /// `owner` must retain admission for [`Self::allocation_bytes`] through its
    /// destruction. The payload is destroyed before that owner is released.
    ///
    /// # Errors
    /// Returns the original layout failure or physical allocation refusal.
    pub fn try_from_admitted<O: Send + Sync + 'static>(
        term: Term,
        owner: O,
    ) -> Result<Self, StorageError> {
        let bytes = Self::allocation_bytes::<O>(&term)?;
        let value = SharedOwned::try_from_admitted(term, owner)
            .map_err(|_| StorageError::AllocationFailed)?;
        Ok(Self { value, bytes })
    }

    /// Bytes retained by this allocation, including its original owner.
    #[must_use]
    pub const fn retained_bytes(&self) -> usize {
        self.bytes
    }
}

impl Deref for TermLease {
    type Target = Term;

    fn deref(&self) -> &Term {
        &self.value
    }
}

/// Caller-owned persistent term storage, independent of a reader's cache.
///
/// Every operation is fallible. A write failure must never select resident
/// fallback. Implementations admit their buffers before allocation, retain
/// grants in returned leases, and preserve keys across eviction and reopening.
/// Multiple handles may share the same store so validation and resolution use
/// one persisted dictionary without retaining its complete decoded contents.
pub trait TermStorage {
    /// Observe settled reader tables after each provisional frame folds.
    /// These cardinalities/capacities are not allocator-byte or RSS estimates.
    /// # Errors
    /// Preserves an original host observation or cancellation refusal.
    fn reader_metadata(&mut self, _usage: ReaderMetadata) -> Result<(), DictionaryError> {
        Ok(())
    }
    /// Select explicit source-window and reader-metadata admission.
    ///
    /// # Errors
    /// Returns the original host policy or cancellation refusal.
    fn limits(&mut self) -> Result<DictionaryLimits, DictionaryError>;
    /// Persist the original accepted term under its immutable source key.
    ///
    /// # Errors
    /// Returns the original storage, admission or cancellation refusal.
    fn put(&mut self, key: TermKey, term: &Term) -> Result<(), DictionaryError>;

    /// Read and pin one original accepted term, admitting its decoded storage.
    ///
    /// `None` means the key is absent; a storage failure must return `Err`.
    ///
    /// # Errors
    /// Returns the original storage, corruption, admission or cancellation cause.
    fn get(&mut self, key: TermKey) -> Result<Option<TermLease>, DictionaryError>;

    /// Read the first accepted statement-layer binding of an indirect term.
    ///
    /// # Errors
    /// Returns the original storage, admission or cancellation refusal.
    fn first_binding(&mut self, key: TermKey) -> Result<Option<Triple3>, DictionaryError>;

    /// Persist the first accepted binding without replacing it on later rows.
    ///
    /// # Errors
    /// Returns the original storage, admission or cancellation refusal.
    fn bind_first(&mut self, key: TermKey, triple: Triple3) -> Result<(), DictionaryError>;

    /// Admit deterministic work and observe caller cancellation before mutation.
    ///
    /// # Errors
    /// Returns the original caller refusal without replacing its cause.
    fn checkpoint(&mut self, units: u64) -> Result<(), DictionaryError>;

    /// Commit provisional writes before reporting successful dictionary use.
    ///
    /// # Errors
    /// Returns the original exhausted-storage or interrupted-write failure.
    fn commit(&mut self) -> Result<(), DictionaryError>;
}

impl<S: TermStorage + ?Sized> TermStorage for &mut S {
    fn reader_metadata(&mut self, usage: ReaderMetadata) -> Result<(), DictionaryError> {
        (**self).reader_metadata(usage)
    }
    fn limits(&mut self) -> Result<DictionaryLimits, DictionaryError> {
        (**self).limits()
    }
    fn put(&mut self, key: TermKey, term: &Term) -> Result<(), DictionaryError> {
        (**self).put(key, term)
    }
    fn get(&mut self, key: TermKey) -> Result<Option<TermLease>, DictionaryError> {
        (**self).get(key)
    }
    fn first_binding(&mut self, key: TermKey) -> Result<Option<Triple3>, DictionaryError> {
        (**self).first_binding(key)
    }
    fn bind_first(&mut self, key: TermKey, triple: Triple3) -> Result<(), DictionaryError> {
        (**self).bind_first(key, triple)
    }
    fn checkpoint(&mut self, units: u64) -> Result<(), DictionaryError> {
        (**self).checkpoint(units)
    }
    fn commit(&mut self) -> Result<(), DictionaryError> {
        (**self).commit()
    }
}

impl<S: TermStorage> TermStorage for &std::cell::RefCell<S> {
    fn reader_metadata(&mut self, usage: ReaderMetadata) -> Result<(), DictionaryError> {
        self.try_borrow_mut()
            .map_err(|_| DictionaryError::Corrupt("dictionary callback reentered"))?
            .reader_metadata(usage)
    }
    fn limits(&mut self) -> Result<DictionaryLimits, DictionaryError> {
        self.try_borrow_mut()
            .map_err(|_| DictionaryError::Corrupt("dictionary callback reentered"))?
            .limits()
    }
    fn put(&mut self, key: TermKey, term: &Term) -> Result<(), DictionaryError> {
        self.try_borrow_mut()
            .map_err(|_| DictionaryError::Corrupt("dictionary callback reentered"))?
            .put(key, term)
    }
    fn get(&mut self, key: TermKey) -> Result<Option<TermLease>, DictionaryError> {
        self.try_borrow_mut()
            .map_err(|_| DictionaryError::Corrupt("dictionary callback reentered"))?
            .get(key)
    }
    fn first_binding(&mut self, key: TermKey) -> Result<Option<Triple3>, DictionaryError> {
        self.try_borrow_mut()
            .map_err(|_| DictionaryError::Corrupt("dictionary callback reentered"))?
            .first_binding(key)
    }
    fn bind_first(&mut self, key: TermKey, triple: Triple3) -> Result<(), DictionaryError> {
        self.try_borrow_mut()
            .map_err(|_| DictionaryError::Corrupt("dictionary callback reentered"))?
            .bind_first(key, triple)
    }
    fn checkpoint(&mut self, units: u64) -> Result<(), DictionaryError> {
        self.try_borrow_mut()
            .map_err(|_| DictionaryError::Corrupt("dictionary callback reentered"))?
            .checkpoint(units)
    }
    fn commit(&mut self) -> Result<(), DictionaryError> {
        self.try_borrow_mut()
            .map_err(|_| DictionaryError::Corrupt("dictionary callback reentered"))?
            .commit()
    }
}
