// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::any::Any;
use std::fmt;
use std::io::{self, Read, Seek, SeekFrom};
use std::sync::Arc;

use crate::dictionary::{DictionaryError, DictionaryLimits};
use purrdf_hash::error::IoFailure;
use purrdf_lex::cbor::{
    EncodedItem, Limits, ReplayBuffer, StructuralCursor, StructuralError, StructuralRange, Value,
};

/// Retained identity of one caller-owned immutable source, decoding capability
/// snapshot or policy. Clones share identity; another owner is always distinct.
#[derive(Clone)]
pub struct AccessOwner {
    owner: Arc<dyn Any + Send + Sync>,
}
purrdf_hash::debug_non_exhaustive!(AccessOwner {});
purrdf_lex::constructors! {
    impl AccessOwner {
        /// Retain the caller's existing owner without allocating another owner.
        pub fn from_owner(owner: Arc<dyn Any + Send + Sync>) -> Self { .. };
    }
}
impl PartialEq for AccessOwner {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner)
    }
}
impl Eq for AccessOwner {}

type KeySnapshot = dyn Fn(&str) -> Option<[u8; 32]> + Send + Sync;

/// Concrete immutable key snapshot, retained under the same allocation that
/// identifies the actual resolver. A new resolver owner requires a new binding.
/// Interior key rotation must change the authoritative capability generation.
#[derive(Clone)]
pub struct DecodeCapabilities {
    owner: AccessOwner,
    resolver: Arc<KeySnapshot>,
}
purrdf_hash::debug_non_exhaustive!(DecodeCapabilities {});
impl DecodeCapabilities {
    /// Bind the actual resolver and its identity to the caller's same owner.
    /// An explicit no-key snapshot is a resolver that always returns `None`.
    pub fn new<F>(resolver: Arc<F>) -> Self
    where
        F: Fn(&str) -> Option<[u8; 32]> + Send + Sync + 'static,
    {
        Self {
            owner: AccessOwner::from_owner(resolver.clone()),
            resolver,
        }
    }
    /// Identity the source callback must report as its concrete capabilities.
    pub fn owner(&self) -> &AccessOwner {
        &self.owner
    }
    pub(super) fn resolver(&self) -> &crate::reader::ContentKeyResolver<'_> {
        &*self.resolver
    }
}

/// Actual immutable lookup policy. The owner identifies these concrete values,
/// including the reader's resource ceilings, rather than an unrelated label.
#[derive(Debug)]
pub struct LookupPolicy {
    /// Permit concatenated segments, matching the original reader option.
    pub allow_segments: bool,
    /// Requested global freshness anchor. Selective lookup refuses this request;
    /// the existing full reader enforces it during explicit verification.
    pub expected_head: Option<Vec<u8>>,
    /// Exact byte-window and metadata ceilings captured by the seek input.
    pub limits: DictionaryLimits,
}
impl LookupPolicy {
    /// Identity of this same immutable policy allocation for the host callback.
    pub fn owner(self: &Arc<Self>) -> AccessOwner {
        AccessOwner::from_owner(self.clone())
    }
}

/// Actual source and decoding context observed by the host identity callback.
///
/// The source owner must retain an immutable snapshot, or its generation must
/// change whenever bytes change. Length and modification time alone do not
/// establish identity. Capability ownership identifies the concrete key/codec
/// snapshot, and policy ownership identifies the concrete requested policy.
/// Rotations require a new owner or generation, even when lengths are unchanged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccessBinding {
    /// Owner of the source snapshot or authoritative version identity.
    pub source: AccessOwner,
    /// Actual source generation, checked before every lookup or source access.
    pub generation: u64,
    /// Actual pinned source length, in bytes.
    pub length: u64,
    /// Owner of the concrete decoding capabilities, including content keys.
    pub capabilities: AccessOwner,
    /// Actual capability generation, including key rotation.
    pub capability_generation: u64,
    /// Owner of the concrete verification and decoding policy.
    pub policy: AccessOwner,
}

/// Precisely which admitted source or capability facts changed.
#[derive(Clone, Debug)]
pub struct BindingChange {
    /// Whether source ownership still agrees.
    pub same_source: bool,
    /// Whether capability ownership still agrees.
    pub same_capabilities: bool,
    /// Whether policy ownership still agrees.
    pub same_policy: bool,
    /// Previously admitted source generation.
    pub expected_generation: u64,
    /// Currently observed source generation.
    pub actual_generation: u64,
    /// Previously admitted source length.
    pub expected_length: u64,
    /// Currently observed source length.
    pub actual_length: u64,
    /// Previously admitted capability generation.
    pub expected_capability_generation: u64,
    /// Currently observed capability generation.
    pub actual_capability_generation: u64,
}
impl purrdf_hash::error::StructuredError for BindingChange {
    purrdf_hash::error_visit! {
        @record self { same_source, same_capabilities, same_policy, expected_generation,
            actual_generation, expected_length, actual_length,
            expected_capability_generation, actual_capability_generation } => "gts-indexed-binding-change" {
            "same_source" => Boolean(self.same_source),
            "same_capabilities" => Boolean(self.same_capabilities),
            "same_policy" => Boolean(self.same_policy),
            "expected_generation" => Unsigned(u128::from(self.expected_generation)),
            "actual_generation" => Unsigned(u128::from(self.actual_generation)),
            "expected_length" => ByteCount(u128::from(self.expected_length)),
            "actual_length" => ByteCount(u128::from(self.actual_length)),
            "expected_capability_generation" => Unsigned(u128::from(self.expected_capability_generation)),
            "actual_capability_generation" => Unsigned(u128::from(self.actual_capability_generation)),
        }
    }
}

/// Original storage/structure refusal or a violated indexed-access invariant.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum IndexedError {
    /// Indirect lookup needs explicitly prepared authenticated reference access.
    ReferencePreparationRequired(u64),
    /// The actual policy or byte ceilings differ from the captured policy.
    PolicyMismatch(&'static str),
    /// The requested global condition requires explicit full-file verification.
    FullVerificationRequired(&'static str),
    /// The supplied concrete resolver differs from the input's captured owner.
    CapabilityOwner,
    /// Original frame, header, capability or decoding refusal, shared intact.
    Native(purrdf_lex::allocation::NativeCause),
    /// An observed catalog contradicts a locator or has no usable index.
    Catalog(super::CatalogError),
    /// Original source or identity-provider failure.
    Io(IoFailure),
    /// Original structural-navigation refusal with its absolute source offset.
    Structure(StructuralError),
    /// Original term storage, admission or cancellation refusal.
    Dictionary(DictionaryError),
    /// A source, capability or policy changed before the requested access.
    BindingChanged(BindingChange),
    /// A bounded local quantity cannot represent the requested operation.
    Overflow(&'static str),
}
purrdf_lex::variant_from!(IndexedError {
    Io(IoFailure),
    Structure(StructuralError),
    Dictionary(DictionaryError),
});
impl fmt::Display for IndexedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReferencePreparationRequired(segment) => write!(
                f,
                "indirect term lookup in segment {segment} requires explicit prepare_reference_access"
            ),
            Self::PolicyMismatch(part) => write!(f, "indexed policy mismatch: {part}"),
            Self::FullVerificationRequired(policy) => write!(
                f,
                "selective lookup cannot enforce {policy}; use explicit full-file verification"
            ),
            Self::CapabilityOwner => {
                f.write_str("indexed resolver does not own the captured capabilities")
            }
            Self::Native(cause) => cause.fmt(f),
            Self::Catalog(cause) => cause.fmt(f),
            Self::Io(cause) => cause.fmt(f),
            Self::Structure(cause) => cause.fmt(f),
            Self::Dictionary(cause) => cause.fmt(f),
            Self::BindingChanged(change) => write!(
                f,
                "indexed binding changed: owner matches source={}, capabilities={}, policy={}; generation {} to {}, length {} to {}",
                change.same_source,
                change.same_capabilities,
                change.same_policy,
                change.expected_generation,
                change.actual_generation,
                change.expected_length,
                change.actual_length
            ),
            Self::Overflow(resource) => write!(f, "indexed {resource} exceeds its representation"),
        }
    }
}
impl std::error::Error for IndexedError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Native(cause) => Some(&***cause),
            Self::Catalog(cause) => Some(cause),
            Self::Io(cause) => Some(cause),
            Self::Structure(cause) => Some(cause),
            Self::Dictionary(cause) => Some(cause),
            Self::BindingChanged(_)
            | Self::Overflow(_)
            | Self::CapabilityOwner
            | Self::PolicyMismatch(_)
            | Self::FullVerificationRequired(_)
            | Self::ReferencePreparationRequired(_) => None,
        }
    }
}
impl IndexedError {
    pub(super) fn native(cause: impl purrdf_hash::error::NativeError + 'static) -> Self {
        match crate::model::retain_native_cause(cause) {
            Ok(cause) => Self::Native(cause),
            Err(cause) => Self::Dictionary(DictionaryError::Storage(cause)),
        }
    }
}
impl purrdf_hash::error::StructuredError for IndexedError {
    purrdf_hash::error_visit! {
        ReferencePreparationRequired(segment) => "gts-indexed-reference-preparation-required" { "segment" => Unsigned(u128::from(*segment)) },
        PolicyMismatch(part) => "gts-indexed-policy-mismatch" { "part" => Code(part) },
        FullVerificationRequired(policy) => "gts-indexed-full-verification-required" { "policy" => Code(policy) },
        CapabilityOwner => "gts-indexed-capability-owner" {},
        Native(cause) => "gts-indexed-native" { "cause" => Cause(&***cause) },
        Catalog(cause) => "gts-indexed-catalog" { "cause" => Cause(cause) },
        Io(cause) => "gts-indexed-io" { "cause" => Cause(cause) },
        Structure(cause) => "gts-indexed-structure" { "cause" => Cause(cause) },
        Dictionary(cause) => "gts-indexed-dictionary" { "cause" => Cause(cause) },
        BindingChanged(change) => "gts-indexed-source-changed" { "change" => Record(change) },
        Overflow(resource) => "gts-indexed-overflow" { "resource" => Text(resource) },
    }
}

/// Actual underlying transport observations, including replay prefetch. Identity
/// callback invocations are counted separately; their host work is caller-owned.
#[derive(Clone, Copy, Debug, Default)]
pub struct IndexedWork {
    /// Actual selected-frame content-hash computations, including mismatches.
    pub verified_frames: u128,
    /// Calls to the logical payload/capability decoder, including plain
    /// pass-through and transformed predecessors needed for row counts.
    /// Encoded CBOR admission and frame hashing do not increment this counter.
    pub decoded_payloads: u128,
    /// Numeric term-reference projections visited while explicitly preparing
    /// dictionary contributors. Reusing retained projections adds no visits.
    pub reference_projections: u128,
    /// Bytes actually returned by the underlying source, including prefetch.
    pub read_bytes: u128,
    /// Calls to the underlying Read implementation, including EOF and failures.
    pub read_calls: u128,
    /// Calls to the underlying Seek implementation, including position queries.
    pub seek_calls: u128,
    /// Calls to the host's current source/capability/policy identity provider.
    pub identity_checks: u128,
}

fn observe(counter: &mut u128, amount: u128) -> io::Result<()> {
    *counter = counter.checked_add(amount).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "indexed transport observation overflow",
        )
    })?;
    Ok(())
}
struct Observed<R> {
    source: R,
    work: IndexedWork,
}
impl<R: Read> Read for Observed<R> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        observe(&mut self.work.read_calls, 1)?;
        let result = self.source.read(output);
        if let Ok(count) = result
            && count <= output.len()
        {
            observe(&mut self.work.read_bytes, count as u128)?;
        }
        result
    }
}
impl<R: Seek> Seek for Observed<R> {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        observe(&mut self.work.seek_calls, 1)?;
        self.source.seek(position)
    }
}

/// A structural extent retained with its exact source and decoding binding.
/// It remains structural-only; materialization goes through its bound input.
#[derive(Clone, Debug)]
pub struct BoundRange {
    range: StructuralRange,
    binding: AccessBinding,
}
impl BoundRange {
    /// Captured absolute first encoded byte.
    pub const fn start(&self) -> u64 {
        self.range.start()
    }
    /// Captured absolute byte immediately after this item.
    pub const fn end(&self) -> u64 {
        self.range.end()
    }
    /// Captured encoded length, without native-size narrowing.
    pub const fn encoded_len(&self) -> u64 {
        self.range.encoded_len()
    }
}

/// A structural child cursor whose binding is checked before every advancement.
#[derive(Debug)]
pub struct BoundCursor {
    cursor: StructuralCursor,
    binding: AccessBinding,
}

/// Caller-supplied seek input bound to a live source and decoding context.
///
/// The identity callback must report authoritative current ownership, generation,
/// actual length, key/codec capabilities and policy without moving the source
/// cursor. It runs before and after source operations and before higher layers
/// may use any catalog, raw-term or resolved-term cache entry. No full validation
/// is performed by opening this input.
pub struct IndexedInput<R: Read + Seek, C> {
    reader: ReplayBuffer<Observed<R>>,
    current: C,
    binding: AccessBinding,
    limits: DictionaryLimits,
    identity_checks: u128,
    failure: Option<IndexedError>,
}
purrdf_hash::debug_non_exhaustive!([R: Read + Seek, C] IndexedInput<R, C> {});
impl<R: Read + Seek, C: FnMut(&R) -> io::Result<AccessBinding>> IndexedInput<R, C> {
    /// Capture a concrete source/capability/policy binding without scanning data.
    ///
    /// # Errors
    /// Returns the original identity or initial source-position failure.
    pub fn new(source: R, mut current: C, limits: DictionaryLimits) -> Result<Self, IndexedError> {
        let binding = current(&source).map_err(|cause| IndexedError::Io(cause.into()))?;
        let reader = ReplayBuffer::new(Observed {
            source,
            work: IndexedWork::default(),
        })
        .map_err(|cause| IndexedError::Io(cause.into()))?;
        Ok(Self {
            reader,
            current,
            binding,
            limits,
            identity_checks: 1,
            failure: None,
        })
    }
    /// Captured binding, not a replacement for a fresh identity check.
    pub fn binding(&self) -> &AccessBinding {
        &self.binding
    }
    /// Captured per-frame and retained-metadata admission limits.
    pub const fn limits(&self) -> DictionaryLimits {
        self.limits
    }
    /// Actual transport and callback observations accumulated by this input.
    pub fn work(&self) -> IndexedWork {
        IndexedWork {
            identity_checks: self.identity_checks,
            ..self.reader.get_ref().work
        }
    }
    /// Verify the current binding before consulting any indexed state.
    ///
    /// # Errors
    /// Returns the original host refusal or the precise binding difference.
    pub fn check_current(&mut self) -> Result<(), IndexedError> {
        self.operation(|input| {
            input.identity_checks = input
                .identity_checks
                .checked_add(1)
                .ok_or(IndexedError::Overflow("identity checks"))?;
            let actual = (input.current)(&input.reader.get_ref().source)
                .map_err(|cause| IndexedError::Io(cause.into()))?;
            input.require_binding(&actual)
        })
    }

    fn operation<T>(
        &mut self,
        task: impl FnOnce(&mut Self) -> Result<T, IndexedError>,
    ) -> Result<T, IndexedError> {
        if let Some(cause) = &self.failure {
            return Err(cause.clone());
        }
        let result = task(self);
        if let Err(cause) = &result
            && self.failure.is_none()
        {
            self.failure = Some(cause.clone());
        }
        result
    }

    fn require_binding(&self, actual: &AccessBinding) -> Result<(), IndexedError> {
        if actual != &self.binding {
            return Err(IndexedError::BindingChanged(BindingChange {
                same_source: actual.source == self.binding.source,
                same_capabilities: actual.capabilities == self.binding.capabilities,
                same_policy: actual.policy == self.binding.policy,
                expected_generation: self.binding.generation,
                actual_generation: actual.generation,
                expected_length: self.binding.length,
                actual_length: actual.length,
                expected_capability_generation: self.binding.capability_generation,
                actual_capability_generation: actual.capability_generation,
            }));
        }
        Ok(())
    }

    fn check_bound(&mut self, binding: &AccessBinding) -> Result<(), IndexedError> {
        self.check_current()?;
        self.require_binding(binding)
    }

    fn bound(&self, range: StructuralRange) -> BoundRange {
        BoundRange {
            range,
            binding: self.binding.clone(),
        }
    }

    /// Locate one structural item under the original pinned source extent.
    ///
    /// # Errors
    /// Returns source/context changes or original grammar/storage refusals.
    pub fn scan(&mut self, start: u64) -> Result<Option<BoundRange>, IndexedError> {
        self.operation(|input| {
            input.check_current()?;
            let range = StructuralRange::scan(
                &mut input.reader,
                start,
                input.binding.length,
                Limits::default(),
            )?;
            input.check_current()?;
            Ok(range.map(|range| input.bound(range)))
        })
    }

    /// Validate and materialize a selected bounded metadata item.
    ///
    /// # Errors
    /// Returns original storage/structure refusal, a changed or foreign binding,
    /// or the encoded-frame limit before allocating the selected value.
    pub fn decode(&mut self, range: &BoundRange) -> Result<Value, IndexedError> {
        self.operation(|input| {
            input.check_bound(&range.binding)?;
            input.check_window(range)?;
            let value = range.range.decode(&mut input.reader)?;
            input.check_current()?;
            Ok(value)
        })
    }
    /// Validate and retain a bounded selected encoded item without constructing
    /// its complete value tree. It keeps the same original source binding.
    /// # Errors
    /// Returns original storage/grammar failure, changed binding or byte limit.
    pub fn encoded(&mut self, range: &BoundRange) -> Result<EncodedItem, IndexedError> {
        self.operation(|input| {
            input.check_bound(&range.binding)?;
            input.check_window(range)?;
            let encoded = range.range.encoded(&mut input.reader)?;
            input.check_current()?;
            Ok(encoded)
        })
    }
    fn check_window(&self, range: &BoundRange) -> Result<(), IndexedError> {
        let required = usize::try_from(range.encoded_len())
            .map_err(|_| IndexedError::Overflow("selected encoded extent"))?;
        if required > self.limits.encoded_frame_bytes {
            return Err(DictionaryError::Limit {
                resource: "encoded-frame-bytes",
                required,
                limit: self.limits.encoded_frame_bytes,
            }
            .into());
        }
        Ok(())
    }

    /// Inspect the selected item's root major type under its original binding.
    /// # Errors
    /// Returns the original source/context or structural refusal.
    pub fn major(&mut self, range: &BoundRange) -> Result<u8, IndexedError> {
        self.operation(|input| {
            input.check_bound(&range.binding)?;
            let major = range.range.major(&mut input.reader)?;
            input.check_current()?;
            Ok(major)
        })
    }

    /// Inspect a tag and bind its structural child to this same context.
    /// # Errors
    /// Returns the original source/context or structural refusal.
    pub fn tag(&mut self, range: &BoundRange) -> Result<Option<(u64, BoundRange)>, IndexedError> {
        self.operation(|input| {
            input.check_bound(&range.binding)?;
            let tagged = range.range.tag(&mut input.reader)?;
            input.check_current()?;
            Ok(tagged.map(|(tag, child)| (tag, input.bound(child))))
        })
    }

    fn children(
        &mut self,
        range: &BoundRange,
        map: bool,
    ) -> Result<Option<BoundCursor>, IndexedError> {
        self.operation(|input| {
            input.check_bound(&range.binding)?;
            let cursor = if map {
                range.range.map(&mut input.reader)?
            } else {
                range.range.array(&mut input.reader)?
            };
            input.check_current()?;
            Ok(cursor.map(|cursor| BoundCursor {
                cursor,
                binding: input.binding.clone(),
            }))
        })
    }

    /// Iterate structural array children under the captured source binding.
    /// # Errors
    /// Returns the original source/context or structural refusal.
    pub fn array(&mut self, range: &BoundRange) -> Result<Option<BoundCursor>, IndexedError> {
        self.children(range, false)
    }
    /// Iterate alternating original map keys and values under that binding.
    /// # Errors
    /// Returns the original source/context or structural refusal.
    pub fn map(&mut self, range: &BoundRange) -> Result<Option<BoundCursor>, IndexedError> {
        self.children(range, true)
    }
    /// Advance a structural cursor only after verifying its original binding.
    /// # Errors
    /// Returns the original source/context or structural refusal.
    pub fn next(&mut self, cursor: &mut BoundCursor) -> Result<Option<BoundRange>, IndexedError> {
        self.operation(|input| {
            input.check_bound(&cursor.binding)?;
            let child = cursor.cursor.next(&mut input.reader)?;
            input.check_current()?;
            Ok(child.map(|range| input.bound(range)))
        })
    }
}

#[cfg(test)]
mod tests;
