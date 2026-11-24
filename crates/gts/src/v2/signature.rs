// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Signature inputs and verification under P6.

use core::num::NonZeroU64;

use purrdf_ed25519 as ed25519;
use purrdf_hash::blake3::Hasher;
use purrdf_lex::cbor::head;

use crate::cose::composite;
use crate::mldsa65;

use super::fields::Fields;
use super::{DIGEST_LENGTH, MalformedReason, REGISTRY_VERSION, WireError};

type Digest = [u8; DIGEST_LENGTH];
pub(super) const ED25519: u64 = 1;
pub(super) const COMPOSITE: u64 = 2;

/// The claimed identity, algorithm and public bytes of a caller-held key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeldKey<'a> {
    /// The key identifier that selects this entry.
    pub key_id: Digest,
    /// The registry code claimed for this public key.
    pub algorithm: u64,
    /// The encoded key, checked for consistency when selected.
    pub public_key: &'a [u8],
}

/// Caller-held keys and caller trust, in deterministic record order.
#[derive(Debug, Default)]
pub struct SignatureInputs<'a> {
    trusted: Vec<Digest>,
    held: Vec<HeldKey<'a>>,
}

impl<'a> SignatureInputs<'a> {
    /// Sort the two sets by their encodings and remove exact duplicates.
    #[must_use]
    pub fn new(mut trusted: Vec<Digest>, mut held: Vec<HeldKey<'a>>) -> Self {
        trusted.sort_unstable();
        trusted.dedup();
        // Deterministic CBOR orders uint heads by value and byte strings by
        // length first within this fixed-shape record.
        held.sort_unstable_by_key(|key| {
            (
                key.key_id,
                key.algorithm,
                key.public_key.len(),
                key.public_key,
            )
        });
        held.dedup();
        Self { trusted, held }
    }

    /// The caller's trusted key identifiers, in deterministic order.
    #[must_use]
    pub fn trusted_keys(&self) -> &[Digest] {
        &self.trusted
    }

    /// The caller's held key records, in deterministic order.
    #[must_use]
    pub fn held_keys(&self) -> &[HeldKey<'a>] {
        &self.held
    }

    pub(super) fn verify(
        &self,
        record: &SignatureRecord<'_>,
        header: &Digest,
        frame: &Digest,
    ) -> Result<SignatureStatus, mldsa65::Error> {
        Self::verify_held(&self.held, record, header, frame)
    }

    pub(super) fn verify_held(
        held: &[HeldKey<'_>],
        record: &SignatureRecord<'_>,
        header: &Digest,
        frame: &Digest,
    ) -> Result<SignatureStatus, mldsa65::Error> {
        use SignatureStatus as Status;

        if !matches!(record.algorithm, ED25519 | COMPOSITE) {
            return Ok(Status::Unverified(SignatureUnverified::UnknownAlgorithm));
        }
        let first = held.partition_point(|key| key.key_id < *record.key_id);
        let mut named = held[first..]
            .iter()
            .take_while(|key| key.key_id == *record.key_id)
            .peekable();
        if named.peek().is_none() {
            return Ok(Status::Unverified(SignatureUnverified::KeyNotHeld));
        }
        let Some(key) = named.find(|key| {
            key.algorithm == record.algorithm
                && key_identifier(key.algorithm, key.public_key) == key.key_id
        }) else {
            return Ok(Status::Invalid(SignatureInvalid::KeyMismatch));
        };
        let (key_length, value_length) = if record.algorithm == ED25519 {
            (ed25519::PUBLIC_KEY_LENGTH, ed25519::SIGNATURE_LENGTH)
        } else {
            (composite::PUBLIC_KEY_LENGTH, composite::SIGNATURE_LENGTH)
        };
        if key.public_key.len() != key_length || record.value.len() != value_length {
            return Ok(Status::Invalid(SignatureInvalid::ValueLength));
        }
        let message = signature_message(record.algorithm, record.key_id, header, frame);
        if record.algorithm == ED25519 {
            let (Ok(public), Ok(value)) = (key.public_key.try_into(), record.value.try_into())
            else {
                return Ok(Status::Invalid(SignatureInvalid::ValueLength));
            };
            return Ok(
                match ed25519::VerifyingKey::verify_encoded_strict(
                    public,
                    &message,
                    &ed25519::Signature::from_bytes(value),
                ) {
                    Ok(()) => Status::Valid,
                    Err(error) => Status::Invalid(SignatureInvalid::Ed25519(error)),
                },
            );
        }
        Ok(
            match composite::verify_components(key.public_key, &message, record.value) {
                Ok(()) => Status::Valid,
                Err(composite::ComponentVerificationError::Length) => {
                    Status::Invalid(SignatureInvalid::ValueLength)
                }
                Err(composite::ComponentVerificationError::Classical) => {
                    Status::Invalid(SignatureInvalid::CompositeClassical)
                }
                Err(composite::ComponentVerificationError::PostQuantum) => {
                    Status::Invalid(SignatureInvalid::CompositePostQuantum)
                }
                Err(composite::ComponentVerificationError::Operation(cause)) => return Err(cause),
            },
        )
    }
}

/// Why a signature could not be verified.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SignatureUnverified {
    /// The algorithm is outside the reader's registry.
    UnknownAlgorithm,
    /// There was no held key with the signature's identifier.
    KeyNotHeld,
}

/// Why a signature check failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SignatureInvalid {
    /// No held key with the identifier matched its claimed identity.
    KeyMismatch,
    /// The key or signature had the wrong encoded length.
    ValueLength,
    /// The first failed Ed25519 check.
    Ed25519(ed25519::EncodedVerificationError),
    /// The classical component of the composite did not verify.
    CompositeClassical,
    /// The post-quantum component of the composite did not verify.
    CompositePostQuantum,
}

/// The verification outcome independently of trust and frame linkage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SignatureStatus {
    /// All checks of the algorithm passed.
    Valid,
    /// The first failed check.
    Invalid(SignatureInvalid),
    /// A required algorithm or key was unavailable.
    Unverified(SignatureUnverified),
}

impl purrdf_hash::error::StructuredError for SignatureStatus {
    purrdf_hash::error_visit! {
        Valid => "gts-v2-signature-valid" {},
        Invalid(reason) => "gts-v2-signature-invalid" { "reason" => Record(reason) },
        Unverified(reason) => "gts-v2-signature-unverified" { "reason" => Record(reason) },
    }
}

impl purrdf_hash::error::StructuredError for SignatureInvalid {
    purrdf_hash::error_visit! {
        KeyMismatch => "gts-v2-signature-key-mismatch" {},
        ValueLength => "gts-v2-signature-value-length" {},
        Ed25519(cause) => "gts-v2-signature-ed25519" { "cause" => Cause(cause) },
        CompositeClassical => "gts-v2-signature-composite-classical" {},
        CompositePostQuantum => "gts-v2-signature-composite-post-quantum" {},
    }
}

impl purrdf_hash::error::StructuredError for SignatureUnverified {
    purrdf_hash::error_visit! {
        UnknownAlgorithm => "gts-v2-signature-unknown-algorithm" {},
        KeyNotHeld => "gts-v2-signature-key-not-held" {},
    }
}

/// The verification, coverage and authentication of one frame signature.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct SignatureCheck {
    /// The algorithm code from the record.
    pub algorithm: u64,
    /// The key identifier from the record.
    pub key_id: Digest,
    /// The independent status of the signature.
    pub status: SignatureStatus,
    /// Beginning of coverage in ordinary frame positions.
    pub coverage_first: u64,
    /// Exclusive end of coverage in ordinary frame positions.
    pub coverage_end: u64,
    /// Whether the signature authenticates under the caller's trust.
    pub authenticated: bool,
}

/// Authenticity of the counted ordinary frames in a segment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Authenticity {
    /// At least one frame was counted, and all counted frames authenticate.
    Authenticated,
    /// Some counted frames authenticate.
    PartlyAuthenticated,
    /// No counted frame authenticates.
    Unauthenticated,
}

/// The signature-based authentication summary of one readable segment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct SegmentAuthentication {
    /// The file-relative segment ordinal.
    pub segment: u64,
    /// Authenticated ordinary frames, excluding indexes.
    pub authenticated_frames: u64,
    /// Authenticity across the counted frames.
    pub authenticity: Authenticity,
}

#[derive(Default)]
struct CoverageCount {
    linked: u64,
    authenticated: u64,
    run_linked: u64,
    run_authenticated: u64,
}

impl CoverageCount {
    const fn begin_run(&mut self) {
        self.run_linked = 0;
        self.run_authenticated = 0;
    }

    const fn count_linked(&mut self) {
        self.linked += 1;
        self.run_linked += 1;
    }

    const fn authenticate(&mut self, commit: bool) {
        if commit {
            // A valid commit covers every linked frame in the prefix.
            self.authenticated = self.linked;
        } else {
            self.authenticated += self.run_linked - self.run_authenticated;
        }
        self.run_authenticated = self.run_linked;
    }
}

#[derive(Default)]
pub(super) struct AuthenticationProgress {
    pub(super) run_start: u64,
    counted: u64,
    ordinary: CoverageCount,
    suppression: CoverageCount,
}

impl AuthenticationProgress {
    pub(super) const fn begin_run(&mut self, first: u64) {
        self.run_start = first;
        self.ordinary.begin_run();
        self.suppression.begin_run();
    }

    pub(super) const fn count_frame(&mut self, link_holds: bool) {
        // Every counted frame already fits the range's ordinary count.
        self.counted += 1;
        if link_holds {
            self.ordinary.count_linked();
        }
    }

    pub(super) const fn count_folded_suppression(&mut self, authenticates: bool) {
        self.suppression.count_linked();
        if authenticates {
            self.suppression.authenticate(false);
        }
    }

    pub(super) const fn authenticate(&mut self, commit: bool) {
        self.ordinary.authenticate(commit);
        self.suppression.authenticate(commit);
    }

    pub(super) const fn folded_suppressions(&self) -> u64 {
        self.suppression.linked
    }

    pub(super) const fn unauthenticated_suppressions(&self) -> u64 {
        self.suppression.linked - self.suppression.authenticated
    }

    pub(super) const fn summary(&self, segment: u64) -> SegmentAuthentication {
        let authenticated_frames = self.ordinary.authenticated;
        let authenticity = if authenticated_frames == 0 {
            Authenticity::Unauthenticated
        } else if authenticated_frames == self.counted {
            Authenticity::Authenticated
        } else {
            Authenticity::PartlyAuthenticated
        };
        SegmentAuthentication {
            segment,
            authenticated_frames,
            authenticity,
        }
    }
}

pub(super) struct SignatureRecord<'a> {
    pub(super) algorithm: u64,
    pub(super) key_id: &'a Digest,
    pub(super) value: &'a [u8],
}

impl<'a> SignatureRecord<'a> {
    pub(super) fn read(fields: &mut Fields<'a>, registry: NonZeroU64) -> Result<Self, WireError> {
        fields.record(3)?;
        let offset = fields.offset();
        let algorithm = fields.uint()?;
        if algorithm == 0 || (algorithm > COMPOSITE && registry.get() <= REGISTRY_VERSION) {
            return Err(WireError::new(MalformedReason::UnregisteredValue, offset));
        }
        Ok(Self {
            algorithm,
            key_id: fields.fixed_bytes()?,
            value: fields.bytes()?,
        })
    }
}

pub(super) fn key_identifier(algorithm: u64, public_key: &[u8]) -> Digest {
    let mut hash = Hasher::new_derive_key("gts2 key id");
    hash.update(&[0x82]);
    hash_head(&mut hash, head::UNSIGNED, algorithm);
    hash_bytes(&mut hash, public_key);
    *hash.finalize().as_bytes()
}

pub(super) fn signature_message(
    algorithm: u64,
    key_id: &Digest,
    header: &Digest,
    frame: &Digest,
) -> Digest {
    let mut hash = Hasher::new_derive_key("gts2 signature subject");
    hash.update(&[0x84]);
    hash_head(&mut hash, head::UNSIGNED, algorithm);
    for digest in [key_id, header, frame] {
        hash_bytes(&mut hash, digest);
    }
    *hash.finalize().as_bytes()
}

fn hash_bytes(hash: &mut Hasher, bytes: &[u8]) {
    head::with_encoded_bytes(bytes, |head, body| {
        hash.update(head);
        hash.update(body);
    });
}

fn hash_head(hash: &mut Hasher, major: u8, value: u64) {
    head::with_encoded_head(major, value, |encoded| hash.update(encoded));
}
