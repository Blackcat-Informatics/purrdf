// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Deterministic migration before a resident dictionary exceeds its allowance.

use super::{DictionaryError, DictionaryLimits, Term, TermKey, TermLease, TermStorage, Triple3};

/// Explicit ownership policy selected before a dictionary drive starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpillPolicy {
    /// Use caller-owned external storage from the first accepted entry.
    External,
    /// Commit migration before the next operation would exceed these live bytes.
    AtBytes(usize),
}

/// Current residency, including a permanently failed migration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DictionaryResidence {
    /// Entries still belong to the admitted resident store.
    Resident,
    /// All entries belong to the caller's external store.
    External,
    /// An original failure prevents every subsequent successful operation.
    Failed,
}

/// Resident storage exposing its exact next allocation and ordered migration.
///
/// Required-byte observations include actual buffer capacities, owned terms,
/// reverse/lookup metadata and retained pins, not merely lexical string lengths.
/// They must be calculated before the corresponding allocation or mutation.
pub trait ResidentTermStorage: TermStorage {
    /// Exact admitted live bytes required by the proposed term insertion.
    ///
    /// # Errors
    /// Returns checked layout or original admission failure before mutation.
    fn required_term_bytes(&self, key: TermKey, term: &Term) -> Result<usize, DictionaryError>;
    /// Exact admitted live bytes required by a first reifier binding.
    ///
    /// # Errors
    /// Returns checked layout or original admission failure before mutation.
    fn required_binding_bytes(
        &self,
        key: TermKey,
        triple: Triple3,
    ) -> Result<usize, DictionaryError>;
    /// Current admitted bytes, including owners that escaped through leases.
    fn retained_bytes(&self) -> usize;
    /// Next stored source key in segment/append order, without collecting keys.
    fn next_key(&self, after: Option<TermKey>) -> Option<TermKey>;
    /// Next first binding in segment/append order, without collecting bindings.
    fn next_binding(&self, after: Option<TermKey>) -> Option<(TermKey, Triple3)>;
    /// Destroy resident entries and buffers before refunding their original grants.
    /// Escaped leases retain their own original allocation accounts.
    ///
    /// # Errors
    /// Returns an original refund/storage failure; never claims migration success.
    fn release_entries(&mut self) -> Result<(), DictionaryError>;
}

/// One resident-to-external transition preserving every original source key.
///
/// The external store is supplied by the host. Migration is incremental and
/// provisional until its commit succeeds. Resident owners remain live until
/// then. Any failure poisons the coordinator with its original native cause;
/// it cannot continue by silently selecting either old or partial storage.
pub struct SpillingDictionary<R, E> {
    resident: R,
    external: E,
    policy: SpillPolicy,
    residence: DictionaryResidence,
    failure: Option<DictionaryError>,
    transitions: u64,
}

impl<R, E> core::fmt::Debug for SpillingDictionary<R, E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SpillingDictionary")
            .field("policy", &self.policy)
            .field("residence", &self.residence)
            .field("transitions", &self.transitions)
            .finish_non_exhaustive()
    }
}

impl<R: ResidentTermStorage, E: TermStorage> SpillingDictionary<R, E> {
    /// Select an explicit resident allowance or direct external operation.
    /// Direct external operation requires an empty resident input; an existing
    /// resident entry makes the coordinator fail rather than silently ignore it.
    pub fn new(resident: R, external: E, policy: SpillPolicy) -> Self {
        let failure = (policy == SpillPolicy::External
            && (resident.next_key(None).is_some() || resident.next_binding(None).is_some()))
        .then_some(DictionaryError::Corrupt(
            "direct external operation has resident entries",
        ));
        let residence = match (policy, failure.is_some()) {
            (_, true) => DictionaryResidence::Failed,
            (SpillPolicy::External, false) => DictionaryResidence::External,
            (SpillPolicy::AtBytes(_), false) => DictionaryResidence::Resident,
        };
        Self {
            resident,
            external,
            policy,
            residence,
            failure,
            transitions: 0,
        }
    }

    /// Observe the current residence without inspecting or loading payloads.
    #[must_use]
    pub const fn residence(&self) -> DictionaryResidence {
        self.residence
    }

    /// Successful committed migrations, distinct from failed attempts.
    #[must_use]
    pub const fn transitions(&self) -> u64 {
        self.transitions
    }

    /// Resident bytes still retained, including externally held lease owners.
    #[must_use]
    pub fn retained_resident_bytes(&self) -> usize {
        self.resident.retained_bytes()
    }

    /// Recover both caller-owned stores, including any failed provisional writes.
    pub fn into_parts(self) -> (R, E) {
        (self.resident, self.external)
    }

    fn healthy(&self) -> Result<(), DictionaryError> {
        match &self.failure {
            Some(error) => Err(error.clone()),
            None => Ok(()),
        }
    }

    fn checked<T>(&mut self, result: Result<T, DictionaryError>) -> Result<T, DictionaryError> {
        if let Err(error) = &result {
            self.failure = Some(error.clone());
            self.residence = DictionaryResidence::Failed;
        }
        result
    }

    fn prepare(&mut self) -> Result<(), DictionaryError> {
        self.healthy()?;
        if self.residence == DictionaryResidence::Resident {
            self.migrate_if_needed(self.resident.retained_bytes())?;
        }
        Ok(())
    }

    fn migrate_if_needed(&mut self, required: usize) -> Result<(), DictionaryError> {
        let SpillPolicy::AtBytes(limit) = self.policy else {
            return Ok(());
        };
        if self.residence != DictionaryResidence::Resident || required <= limit {
            return Ok(());
        }
        let result = (|| {
            let mut previous = None;
            while let Some(key) = self.resident.next_key(previous) {
                if previous.is_some_and(|previous| key <= previous) {
                    return Err(DictionaryError::Corrupt(
                        "unordered resident term migration",
                    ));
                }
                self.external.checkpoint(1)?;
                let term = self
                    .resident
                    .get(key)?
                    .ok_or(DictionaryError::Missing(key))?;
                self.external.put(key, &term)?;
                previous = Some(key);
            }
            let mut previous = None;
            while let Some((key, triple)) = self.resident.next_binding(previous) {
                if previous.is_some_and(|previous| key <= previous) {
                    return Err(DictionaryError::Corrupt(
                        "unordered resident binding migration",
                    ));
                }
                self.external.checkpoint(1)?;
                self.external.bind_first(key, triple)?;
                previous = Some(key);
            }
            self.external.commit()?;
            self.resident.release_entries()?;
            self.transitions = self
                .transitions
                .checked_add(1)
                .ok_or(purrdf_lex::allocation::StorageError::SizeOverflow)?;
            self.residence = DictionaryResidence::External;
            Ok(())
        })();
        self.checked(result)
    }
}

impl<R: ResidentTermStorage, E: TermStorage> TermStorage for SpillingDictionary<R, E> {
    fn reader_metadata(&mut self, usage: super::ReaderMetadata) -> Result<(), DictionaryError> {
        self.prepare()?;
        let result = match self.residence {
            DictionaryResidence::Resident => self.resident.reader_metadata(usage),
            _ => self.external.reader_metadata(usage),
        };
        self.checked(result)
    }
    fn limits(&mut self) -> Result<DictionaryLimits, DictionaryError> {
        self.prepare()?;
        if self.policy == SpillPolicy::External {
            let result = self.external.limits();
            return self.checked(result);
        }
        let result = (|| {
            let resident = self.resident.limits()?;
            let external = self.external.limits()?;
            Ok(DictionaryLimits {
                encoded_frame_bytes: resident
                    .encoded_frame_bytes
                    .min(external.encoded_frame_bytes),
                decoded_frame_bytes: resident
                    .decoded_frame_bytes
                    .min(external.decoded_frame_bytes),
                metadata_entries: resident.metadata_entries.min(external.metadata_entries),
            })
        })();
        self.checked(result)
    }
    fn put(&mut self, key: TermKey, term: &Term) -> Result<(), DictionaryError> {
        self.prepare()?;
        if self.residence == DictionaryResidence::Resident {
            let required = self.resident.required_term_bytes(key, term);
            let required = self.checked(required)?;
            self.migrate_if_needed(required)?;
        }
        let result = match self.residence {
            DictionaryResidence::Resident => self.resident.put(key, term),
            _ => self.external.put(key, term),
        };
        self.checked(result)
    }
    fn get(&mut self, key: TermKey) -> Result<Option<TermLease>, DictionaryError> {
        self.prepare()?;
        let result = match self.residence {
            DictionaryResidence::Resident => self.resident.get(key),
            _ => self.external.get(key),
        };
        self.checked(result)
    }
    fn first_binding(&mut self, key: TermKey) -> Result<Option<Triple3>, DictionaryError> {
        self.prepare()?;
        let result = match self.residence {
            DictionaryResidence::Resident => self.resident.first_binding(key),
            _ => self.external.first_binding(key),
        };
        self.checked(result)
    }
    fn bind_first(&mut self, key: TermKey, triple: Triple3) -> Result<(), DictionaryError> {
        self.prepare()?;
        if self.residence == DictionaryResidence::Resident {
            let required = self.resident.required_binding_bytes(key, triple);
            let required = self.checked(required)?;
            self.migrate_if_needed(required)?;
        }
        let result = match self.residence {
            DictionaryResidence::Resident => self.resident.bind_first(key, triple),
            _ => self.external.bind_first(key, triple),
        };
        self.checked(result)
    }
    fn checkpoint(&mut self, units: u64) -> Result<(), DictionaryError> {
        self.prepare()?;
        let result = match self.residence {
            DictionaryResidence::Resident => self.resident.checkpoint(units),
            _ => self.external.checkpoint(units),
        };
        self.checked(result)
    }
    fn commit(&mut self) -> Result<(), DictionaryError> {
        self.prepare()?;
        let result = match self.residence {
            DictionaryResidence::Resident => self.resident.commit(),
            _ => self.external.commit(),
        };
        self.checked(result)
    }
}
