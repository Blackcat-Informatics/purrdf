// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Scoped presentation references over authoritative BLAKE3 identities.
//!
//! The caller supplies a namespace identifying one append-only scope for its
//! entire lifetime. It must not reuse that namespace for an independent or
//! forked assignment history. Initial input order is explicit: the first
//! distinct digest receives ordinal one, then two, and so on. Duplicates reuse
//! their ordinal. Growth appends without changing an existing assignment.
//!
//! Revision version one is BLAKE3 of these concatenated bytes: the registered
//! `purrdf-core/content-alias-revision/v1` domain; an eight-byte little-endian
//! length of 32 and the exact scope digest; an eight-byte little-endian count;
//! then each complete 32-byte content digest in assignment order. Scope framing
//! and the count use `purrdf_hash::frame`; fixed-size digest entries need no
//! additional delimiter. The export's `PURRAL01` version uses that exact law.
//!
//! A reference is always accompanied by its scope. Neither its ordinal nor its
//! spelling is a content address, a signature, or an integrity check. Only the
//! original [`Digest32`] identifies content. This module never changes content,
//! RDF terms, provenance, or canonical digest serialization.
//!
//! The host retains a [`Presentation`] with the exact mapping revision when
//! sending a request. The model returns a structured JSON record containing
//! only `{"aliases":[1,2]}`; the host supplies the retained presentation to
//! [`AliasScope::resolve_response`]. Changed mappings refuse stale responses.
//! Native membership validation remains mandatory with constrained decoding;
//! selecting the wrong valid member remains a separate semantic failure.
//!
//! Every allocation and lookup uses the original caller admission. Complete
//! output leases borrow that owner until their storage dies. Failed growth and
//! restore never publish assignments. A cleanup shrink can be refused after a
//! successful commit: [`Commit::cleanup_refusal`] reports that outcome, and the
//! original grant remains conservative. Lease destruction records such a refusal
//! for [`AliasScope::take_cleanup_refusal`]; explicit [`Output::release`] also
//! returns it. No failure is reported as a rolled-back mutation after commit.

use core::{fmt, mem, num::NonZeroU64, ops::Deref};
use purrdf_hash::{Domain, blake3, frame, hex::Digest32};
use purrdf_lex::{
    allocation::{Admission, Memory, OrderedTable, OwnedAdmission, StorageError},
    json::Value,
};

const REVISION_DOMAIN: Domain = Domain::new(b"purrdf-core/content-alias-revision/v1");
const MAGIC: &[u8; 8] = b"PURRAL01";
const HEADER: usize = 8 + Digest32::LEN * 2 + 8;

macro_rules! digest_kind {
    ($name:ident, $code:literal, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(Digest32);
        purrdf_lex::constructors! {
            impl $name {
                /// Wrap the exact caller-provided identity.
                pub fn new(digest: Digest32) -> Self;
            }
        }
        impl $name {
            /// The complete original digest.
            pub const fn digest(self) -> Digest32 {
                self.0
            }
        }
        impl purrdf_hash::error::StructuredError for $name {
            purrdf_hash::error_visit! {
                @tuple self (digest) => $code { "digest" => Record(digest) }
            }
        }
    };
}
digest_kind!(
    ScopeId,
    "content-alias-scope",
    "Caller namespace for one non-reused append-only assignment history."
);
digest_kind!(
    MappingRevision,
    "content-alias-revision",
    "BLAKE3 identity of the exact scoped assignment sequence, not of content."
);

/// A positive ordinal, meaningful only with its original scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Alias(NonZeroU64);
impl Alias {
    /// Read a positive native ordinal.
    /// # Errors
    /// Zero is never a reference.
    pub fn new(ordinal: u64) -> Result<Self, Error> {
        NonZeroU64::new(ordinal)
            .map(Self)
            .ok_or(Error::InvalidAlias { ordinal })
    }
    /// The short ordinal carried by a structured model response.
    pub const fn ordinal(self) -> u64 {
        self.0.get()
    }
}

/// A reference bound to its exact caller namespace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScopedAlias {
    /// Original assignment scope.
    pub scope: ScopeId,
    /// Positive ordinal within that scope.
    pub alias: Alias,
}
impl purrdf_hash::error::StructuredError for ScopedAlias {
    purrdf_hash::error_visit! {
        @record self { scope, alias } => "scoped-content-alias" {
            "scope" => Record(&self.scope), "alias" => Unsigned(u128::from(self.alias.ordinal())),
        }
    }
}

/// Trusted host envelope retained with one presentation, outside model output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Presentation {
    /// Exact scope of the presented references.
    pub scope: ScopeId,
    /// Exact assignment sequence when the request was prepared.
    pub revision: MappingRevision,
}

/// Successful atomic publication and any later refused cleanup shrink.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Commit {
    /// The fully committed mapping revision.
    pub revision: MappingRevision,
    /// Original cleanup refusal after the old payload died. The new mapping is
    /// committed and its conservative original grant remains retained.
    pub cleanup_refusal: Option<StorageError>,
}

/// A native, structured refusal with no partially published result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// A lookup or restore names another scope.
    Scope {
        /// This map's scope.
        expected: ScopeId,
        /// Supplied scope.
        actual: ScopeId,
    },
    /// A response or export names a different assignment sequence.
    Revision {
        /// Required revision.
        expected: MappingRevision,
        /// Supplied revision.
        actual: MappingRevision,
    },
    /// Zero is not a valid short ordinal.
    InvalidAlias {
        /// Supplied value.
        ordinal: u64,
    },
    /// The digest has no assigned reference.
    UnknownDigest {
        /// Unchanged authoritative identity.
        digest: Digest32,
    },
    /// The reference is not a member of its scope.
    UnknownReference {
        /// Original submitted reference.
        reference: ScopedAlias,
    },
    /// A restored sequence repeats a digest instead of carrying its canonical assignment.
    Duplicate {
        /// Repeated identity.
        digest: Digest32,
        /// Original ordinal.
        first: u64,
        /// Conflicting ordinal.
        repeated: u64,
    },
    /// Restore would replace an existing assignment.
    Prefix {
        /// Ordinal that differs or would disappear.
        ordinal: u64,
    },
    /// Malformed versioned export bytes.
    Wire {
        /// Original byte offset.
        offset: usize,
        /// Required shape.
        expected: &'static str,
    },
    /// A structured response has a different shape or an invalid ordinal.
    Response {
        /// Array position, when applicable.
        index: Option<usize>,
        /// Required shape.
        expected: &'static str,
    },
    /// Original work/storage admission, layout or physical allocation refusal.
    Storage(StorageError),
}
purrdf_hash::error_sources!(Error, Storage);
purrdf_lex::variant_from!(Error { Storage(StorageError) });
impl purrdf_hash::error::StructuredError for Error {
    purrdf_hash::error_visit! {
        Scope { expected, actual } => "content-alias-wrong-scope" { "expected" => Record(expected), "actual" => Record(actual) },
        Revision { expected, actual } => "content-alias-wrong-revision" { "expected" => Record(expected), "actual" => Record(actual) },
        InvalidAlias { ordinal } => "content-alias-invalid-ordinal" { "ordinal" => Unsigned(u128::from(*ordinal)) },
        UnknownDigest { digest } => "content-alias-unknown-digest" { "digest" => Record(digest) },
        UnknownReference { reference } => "content-alias-unknown-reference" { "reference" => Record(reference) },
        Duplicate { digest, first, repeated } => "content-alias-duplicate-assignment" { "digest" => Record(digest), "first" => Unsigned(u128::from(*first)), "repeated" => Unsigned(u128::from(*repeated)) },
        Prefix { ordinal } => "content-alias-incompatible-prefix" { "ordinal" => Unsigned(u128::from(*ordinal)) },
        Wire { offset, expected } => "content-alias-wire" { "offset" => ByteCount(*offset as u128), "expected" => Text(expected) },
        Response { index, expected } => "content-alias-response" { "index" => Value(index.map_or(purrdf_hash::error::ErrorValue::Absent, |index| purrdf_hash::error::ErrorValue::Unsigned(index as u128))), "expected" => Text(expected) },
        Storage(cause) => "content-alias-storage" { "cause" => Cause(cause) },
    }
}
impl fmt::Display for Error {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Scope { expected, actual } => write!(
                out,
                "content reference scope {actual:?} differs from {expected:?}"
            ),
            Self::Revision { expected, actual } => write!(
                out,
                "content reference revision {actual:?} differs from {expected:?}"
            ),
            Self::InvalidAlias { ordinal } => {
                write!(out, "content alias {ordinal} is not positive")
            }
            Self::UnknownDigest { digest } => {
                write!(out, "content identity {digest} has no scoped alias")
            }
            Self::UnknownReference { reference } => {
                write!(out, "unknown content reference {reference:?}")
            }
            Self::Duplicate {
                digest,
                first,
                repeated,
            } => write!(out, "identity {digest} repeats alias {first} at {repeated}"),
            Self::Prefix { ordinal } => {
                write!(out, "restore changes existing content alias {ordinal}")
            }
            Self::Wire { offset, expected } => write!(
                out,
                "content alias export byte {offset}: expected {expected}"
            ),
            Self::Response { index, expected } => write!(
                out,
                "content alias response at {index:?}: expected {expected}"
            ),
            Self::Storage(cause) => cause.fmt(out),
        }
    }
}

#[derive(Default)]
struct Payload {
    digests: Vec<Digest32>,
    index: OrderedTable<Digest32, Alias>,
}
impl Payload {
    fn owned_bytes(&self) -> Result<usize, StorageError> {
        let index = self.index.storage_bytes()?;
        self.digests
            .capacity()
            .checked_mul(size_of::<Digest32>())
            .and_then(|bytes| bytes.checked_add(index))
            .ok_or(StorageError::SizeOverflow)
    }
    fn find<S: Admission + ?Sized>(
        &self,
        digest: Digest32,
        memory: &mut Memory<'_, S>,
    ) -> Result<Option<Alias>, Error> {
        self.index
            .try_get_by(|key| {
                memory.admission_mut().checkpoint(1)?;
                Ok::<_, StorageError>(digest.cmp(key))
            })
            .map(Option::<&Alias>::copied)
            .map_err(Error::from)
    }
    fn append<S: Admission + ?Sized>(
        &mut self,
        digest: Digest32,
        memory: &mut Memory<'_, S>,
    ) -> Result<Alias, Error> {
        let ordinal = u64::try_from(self.digests.len())
            .ok()
            .and_then(|len| len.checked_add(1))
            .ok_or(StorageError::SizeOverflow)?;
        let alias = Alias::new(ordinal)?;
        let entry =
            self.index
                .prepare_insert_with_memory(digest, memory, |left, right, memory| {
                    memory.admission_mut().checkpoint(1)?;
                    Ok::<_, StorageError>(left.cmp(right))
                })?;
        memory.push(&mut self.digests, digest)?;
        if let Some(first) = entry.publish(alias).1 {
            return Err(Error::Duplicate {
                digest,
                first: first.ordinal(),
                repeated: ordinal,
            });
        }
        Ok(alias)
    }
}

fn check_scope(expected: ScopeId, actual: ScopeId) -> Result<(), Error> {
    if expected == actual {
        Ok(())
    } else {
        Err(Error::Scope { expected, actual })
    }
}
fn revision<S: Admission + ?Sized>(
    scope: ScopeId,
    digests: &[Digest32],
    memory: &mut Memory<'_, S>,
) -> Result<MappingRevision, Error> {
    memory.admission_mut().checkpoint(1)?;
    let mut hash = blake3::Hasher::new();
    hash.update(REVISION_DOMAIN.as_bytes());
    frame::frame_le_into(&mut hash, scope.0.as_bytes());
    frame::frame_length_into(&mut hash, digests.len());
    for digest in digests {
        memory.admission_mut().checkpoint(1)?;
        hash.update(digest.as_bytes());
    }
    Ok(MappingRevision(Digest32::new(*hash.finalize().as_bytes())))
}

/// One admitted, append-only mapping. The original caller account is retained
/// through every mapping/output allocation and is destroyed last.
pub struct AliasScope<S: Admission> {
    scope: ScopeId,
    revision: MappingRevision,
    payload: Payload,
    live: usize,
    cleanup_refusal: Option<StorageError>,
    admission: OwnedAdmission<S>,
}
purrdf_hash::debug_non_exhaustive!([S: Admission] AliasScope<S> {});
impl<S: Admission> AliasScope<S> {
    /// Start an empty mapping in a caller namespace that is never reused for
    /// another independent or forked history.
    /// # Errors
    /// Returns the original work admission refusal.
    pub fn new(scope: ScopeId, admission: S) -> Result<Self, Error> {
        let mut admission = OwnedAdmission::new(admission);
        let revision = revision(scope, &[], &mut Memory::new(&mut admission))?;
        Ok(Self {
            scope,
            revision,
            payload: Payload::default(),
            live: 0,
            cleanup_refusal: None,
            admission,
        })
    }
    /// This map's caller-supplied namespace; this getter performs no lookup.
    pub const fn scope(&self) -> ScopeId {
        self.scope
    }
    /// Take a refused destructor/commit cleanup shrink. Its grant remains
    /// conservative until the original authority accepts its release.
    pub fn take_cleanup_refusal(&mut self) -> Option<StorageError> {
        self.cleanup_refusal.take()
    }

    fn compute<T>(
        &mut self,
        body: impl FnOnce(&Payload, &mut Memory<'_, OwnedAdmission<S>>) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let mut memory = Memory::resume(&mut self.admission, self.live);
        let result = memory.try_scope(|memory| {
            memory.admission_mut().checkpoint(1)?;
            body(&self.payload, memory)
        });
        self.live = memory.admitted_bytes();
        result
    }
    fn release_vec<T>(&mut self, values: Vec<T>) -> Result<(), StorageError> {
        if values.capacity() == 0 {
            return Ok(());
        }
        let mut memory = Memory::resume(&mut self.admission, self.live);
        let result = memory.release_vec(values);
        self.live = memory.admitted_bytes();
        if let Err(cause) = result {
            self.cleanup_refusal = Some(cause);
        }
        result
    }
    fn commit(&mut self, payload: Payload, next: MappingRevision, old_bytes: usize) -> Commit {
        drop(mem::replace(&mut self.payload, payload));
        self.revision = next;
        let mut memory = Memory::resume(&mut self.admission, self.live);
        let cleanup_refusal = memory.release_bytes(old_bytes).err();
        self.live = memory.admitted_bytes();
        if cleanup_refusal.is_some() {
            self.cleanup_refusal = cleanup_refusal;
        }
        Commit {
            revision: next,
            cleanup_refusal,
        }
    }

    /// Atomically append previously unseen identities in the supplied order.
    /// Duplicate digests reuse their existing aliases, including within a batch.
    /// # Errors
    /// Wrong scope, ordinal overflow, work/storage refusal or allocation failure
    /// leaves every assignment and the revision unchanged.
    pub fn extend(&mut self, scope: ScopeId, ordered: &[Digest32]) -> Result<Commit, Error> {
        let own_scope = self.scope;
        let changes = self.compute(|old, memory| {
            check_scope(own_scope, scope)?;
            for &digest in ordered {
                memory.admission_mut().checkpoint(1)?;
                if old.find(digest, memory)?.is_none() {
                    return Ok(true);
                }
            }
            Ok(false)
        })?;
        if !changes {
            return Ok(Commit {
                revision: self.revision,
                cleanup_refusal: None,
            });
        }
        let old_bytes = self.payload.owned_bytes()?;
        let (payload, next) = self.compute(|old, memory| {
            check_scope(own_scope, scope)?;
            let mut payload = Payload::default();
            for &digest in old.digests.iter().chain(ordered) {
                memory.admission_mut().checkpoint(1)?;
                if payload.find(digest, memory)?.is_none() {
                    payload.append(digest, memory)?;
                }
            }
            let next = revision(scope, &payload.digests, memory)?;
            Ok((payload, next))
        })?;
        Ok(self.commit(payload, next, old_bytes))
    }
    /// Find the canonical alias for a full identity in this exact scope.
    /// # Errors
    /// Wrong scope, unknown identity or original work refusal.
    pub fn reference(&mut self, scope: ScopeId, digest: Digest32) -> Result<ScopedAlias, Error> {
        let own_scope = self.scope;
        self.compute(|payload, memory| {
            check_scope(own_scope, scope)?;
            let alias = payload
                .find(digest, memory)?
                .ok_or(Error::UnknownDigest { digest })?;
            Ok(ScopedAlias { scope, alias })
        })
    }
    /// Resolve a fully scoped reference to its exact original digest.
    /// # Errors
    /// Wrong scope, unknown ordinal or original work refusal.
    pub fn resolve(&mut self, reference: ScopedAlias) -> Result<Digest32, Error> {
        let own_scope = self.scope;
        self.compute(|payload, memory| {
            check_scope(own_scope, reference.scope)?;
            memory.admission_mut().checkpoint(1)?;
            usize::try_from(reference.alias.ordinal() - 1)
                .ok()
                .and_then(|index| payload.digests.get(index))
                .copied()
                .ok_or(Error::UnknownReference { reference })
        })
    }
    /// Retain the exact current scope/revision with a host request.
    /// # Errors
    /// Wrong scope or original work refusal.
    pub fn presentation(&mut self, scope: ScopeId) -> Result<Presentation, Error> {
        let own_scope = self.scope;
        let current = self.revision;
        self.compute(|_, _| {
            check_scope(own_scope, scope)?;
            Ok(Presentation {
                scope,
                revision: current,
            })
        })
    }
    /// Enumerate every valid reference in assignment order, under one complete
    /// admitted output lease. A provider's constrained output never replaces
    /// native response membership checks.
    /// # Errors
    /// Wrong scope or original work/storage refusal. No partial list escapes.
    pub fn references(&mut self, scope: ScopeId) -> Result<Output<'_, ScopedAlias, S>, Error> {
        let own_scope = self.scope;
        let values = self.compute(|payload, memory| {
            check_scope(own_scope, scope)?;
            let mut values = Vec::new();
            for index in 0..payload.digests.len() {
                memory.admission_mut().checkpoint(1)?;
                let ordinal = u64::try_from(index)
                    .ok()
                    .and_then(|index| index.checked_add(1))
                    .ok_or(StorageError::SizeOverflow)?;
                memory.push(
                    &mut values,
                    ScopedAlias {
                        scope,
                        alias: Alias::new(ordinal)?,
                    },
                )?;
            }
            Ok(values)
        })?;
        Ok(Output {
            values,
            scope: self,
        })
    }
    /// Resolve the strict native JSON record `{"aliases":[positive integers]}`
    /// using the host's retained envelope. Returned order and duplicates are
    /// preserved. Original content/provenance stays with its caller.
    /// # Errors
    /// Wrong scope/revision, another record shape, unknown membership, or
    /// original work/storage refusal. No partial result escapes.
    pub fn resolve_response(
        &mut self,
        presentation: Presentation,
        response: &Value,
    ) -> Result<Output<'_, Digest32, S>, Error> {
        let own_scope = self.scope;
        let current = self.revision;
        let values = self.compute(|payload, memory| {
            check_scope(own_scope, presentation.scope)?;
            if presentation.revision != current {
                return Err(Error::Revision {
                    expected: current,
                    actual: presentation.revision,
                });
            }
            let fields = response.as_object().ok_or(Error::Response {
                index: None,
                expected: "an object containing only aliases",
            })?;
            if fields.len() != 1 || fields.members()[0].0 != "aliases" {
                return Err(Error::Response {
                    index: None,
                    expected: "exactly one aliases field",
                });
            }
            let aliases = fields.members()[0].1.as_array().ok_or(Error::Response {
                index: None,
                expected: "an aliases array",
            })?;
            let mut values = Vec::new();
            for (index, value) in aliases.iter().enumerate() {
                memory.admission_mut().checkpoint(1)?;
                let number = value.as_number().ok_or(Error::Response {
                    index: Some(index),
                    expected: "a positive u64 integer alias",
                })?;
                memory.admission_mut().checkpoint(
                    u64::try_from(number.lexeme().len()).map_err(|_| StorageError::SizeOverflow)?,
                )?;
                let ordinal = number
                    .is_integer()
                    .then(|| number.as_u64())
                    .flatten()
                    .ok_or(Error::Response {
                        index: Some(index),
                        expected: "a positive u64 integer alias",
                    })?;
                let reference = ScopedAlias {
                    scope: own_scope,
                    alias: Alias::new(ordinal)?,
                };
                let digest = usize::try_from(ordinal - 1)
                    .ok()
                    .and_then(|index| payload.digests.get(index))
                    .copied()
                    .ok_or(Error::UnknownReference { reference })?;
                memory.push(&mut values, digest)?;
            }
            Ok(values)
        })?;
        Ok(Output {
            values,
            scope: self,
        })
    }

    /// Export a complete versioned binary mapping without changing any digest.
    /// Layout: eight magic/version bytes, scope, revision, little-endian u64
    /// count, then the ordered 32-byte digests. Ordinals are index plus one.
    /// # Errors
    /// Wrong scope or original work/storage refusal. No partial bytes escape.
    pub fn export(&mut self, scope: ScopeId) -> Result<Output<'_, u8, S>, Error> {
        let own_scope = self.scope;
        let current = self.revision;
        let values = self.compute(|payload, memory| {
            check_scope(own_scope, scope)?;
            let count =
                u64::try_from(payload.digests.len()).map_err(|_| StorageError::SizeOverflow)?;
            let mut values = Vec::new();
            for part in [
                MAGIC.as_slice(),
                scope.0.as_bytes(),
                current.0.as_bytes(),
                &count.to_le_bytes(),
            ] {
                memory.extend(&mut values, part.iter().copied())?;
            }
            for digest in &payload.digests {
                memory.admission_mut().checkpoint(1)?;
                memory.extend(&mut values, digest.as_bytes().iter().copied())?;
            }
            Ok(values)
        })?;
        Ok(Output {
            values,
            scope: self,
        })
    }
    /// Atomically restore a complete export into this exact scope. Existing
    /// assignments must be an unchanged prefix; restoration never renumbers.
    /// # Errors
    /// Wrong scope, malformed bytes/revision, duplicate identities, incompatible
    /// prefix or original work/storage refusal leaves the map unchanged.
    pub fn restore(&mut self, scope: ScopeId, bytes: &[u8]) -> Result<Commit, Error> {
        let own_scope = self.scope;
        let old_bytes = self.payload.owned_bytes()?;
        let (payload, next) = self.compute(|old, memory| {
            check_scope(own_scope, scope)?;
            if bytes.len() < HEADER || bytes.get(..8) != Some(MAGIC.as_slice()) {
                return Err(Error::Wire {
                    offset: 0,
                    expected: "a complete PURRAL01 header",
                });
            }
            let digest_at = |offset: usize| -> Result<Digest32, Error> {
                let raw = bytes
                    .get(offset..offset + Digest32::LEN)
                    .and_then(|bytes| bytes.try_into().ok())
                    .ok_or(Error::Wire {
                        offset,
                        expected: "32 digest bytes",
                    })?;
                Ok(Digest32::new(raw))
            };
            let restored_scope = ScopeId(digest_at(8)?);
            check_scope(own_scope, restored_scope)?;
            let stated = MappingRevision(digest_at(8 + Digest32::LEN)?);
            let count = crate::bytes::read_u64_le(bytes, HEADER - 8)
                .and_then(|count| usize::try_from(count).ok())
                .ok_or(StorageError::SizeOverflow)?;
            let length = count
                .checked_mul(Digest32::LEN)
                .and_then(|size| HEADER.checked_add(size))
                .ok_or(StorageError::SizeOverflow)?;
            if bytes.len() != length {
                return Err(Error::Wire {
                    offset: HEADER - 8,
                    expected: "the exact declared assignment bytes",
                });
            }
            if count < old.digests.len() {
                return Err(Error::Prefix {
                    ordinal: u64::try_from(count).map_err(|_| StorageError::SizeOverflow)? + 1,
                });
            }
            let mut payload = Payload::default();
            for index in 0..count {
                memory.admission_mut().checkpoint(1)?;
                let digest = digest_at(HEADER + index * Digest32::LEN)?;
                if old
                    .digests
                    .get(index)
                    .is_some_and(|existing| *existing != digest)
                {
                    return Err(Error::Prefix {
                        ordinal: u64::try_from(index).map_err(|_| StorageError::SizeOverflow)? + 1,
                    });
                }
                if let Some(first) = payload.find(digest, memory)? {
                    return Err(Error::Duplicate {
                        digest,
                        first: first.ordinal(),
                        repeated: u64::try_from(index).map_err(|_| StorageError::SizeOverflow)? + 1,
                    });
                }
                payload.append(digest, memory)?;
            }
            let actual = revision(scope, &payload.digests, memory)?;
            if actual != stated {
                return Err(Error::Revision {
                    expected: actual,
                    actual: stated,
                });
            }
            Ok((payload, actual))
        })?;
        Ok(self.commit(payload, next, old_bytes))
    }
}

/// A complete admitted native output borrowing its original scope owner.
/// It cannot detach the allocation or outlive that authority.
pub struct Output<'scope, T: Copy, S: Admission> {
    values: Vec<T>,
    scope: &'scope mut AliasScope<S>,
}
purrdf_hash::debug_non_exhaustive!(['scope, T: Copy, S: Admission] Output<'scope, T, S> {});
impl<T: Copy, S: Admission> Output<'_, T, S> {
    /// Destroy the output before refunding its original backing-array grant.
    /// # Errors
    /// A refused shrink keeps the original grant and is also recorded on scope.
    pub fn release(mut self) -> Result<(), StorageError> {
        let values = mem::take(&mut self.values);
        self.scope.release_vec(values)
    }
}
impl<T: Copy, S: Admission> Deref for Output<'_, T, S> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        &self.values
    }
}
impl<T: Copy, S: Admission> Drop for Output<'_, T, S> {
    fn drop(&mut self) {
        let values = mem::take(&mut self.values);
        let _ = self.scope.release_vec(values);
    }
}
