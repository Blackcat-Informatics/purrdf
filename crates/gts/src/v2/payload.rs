// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::fmt;
use core::num::NonZeroU64;
use std::borrow::Cow;
use std::collections::TryReserveError;

use purrdf_hash::blake3::{Hasher, derive_key};
use purrdf_lex::cbor::head;
use purrdf_zstd::decoding::{FrameDecoder, errors::FrameDecoderError};

use super::fields::Fields;
use super::{
    DIGEST_LENGTH, FrameEnvelope, FrameField, MalformedReason, REGISTRY_VERSION, WireError,
};

const STORED_CHECK_LENGTH: usize = 16;
pub(super) const ZSTD: u64 = 1;

/// Three byte limits used by the P1 payload check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PayloadCapacity {
    /// Limit on each stored payload, encoded content and encoded signature list.
    pub stored_bytes_per_frame: u64,
    /// Limit on the declared decoded length of a single frame.
    pub decoded_bytes_per_frame: u64,
    /// Limit on the aggregate declared decoded length.
    pub decoded_bytes_per_read: u64,
}

/// Shared decoded-length accounting across all frames of a read.
#[derive(Clone, Debug)]
pub struct PayloadBudget {
    capacity: PayloadCapacity,
    charged: u64,
    refused: bool,
}

#[derive(Clone, Copy)]
pub(super) struct CandidateRefusal {
    pub(super) offset: usize,
    pub(super) length: usize,
    pub(super) requested_total: u64,
}

impl PayloadBudget {
    /// Begin a read with the supplied byte limits.
    #[must_use]
    pub const fn new(capacity: PayloadCapacity) -> Self {
        Self {
            capacity,
            charged: 0,
            refused: false,
        }
    }

    /// The bytes charged to the read before its first total refusal.
    #[must_use]
    pub const fn charged(&self) -> u64 {
        self.charged
    }

    pub(super) const fn admits_content(&self, length: u64) -> bool {
        length <= self.capacity.stored_bytes_per_frame
    }

    fn admit(&mut self, lengths: [u64; 3], decoded: u64) -> Result<u64, PayloadRefusal> {
        for count in lengths {
            if count > self.capacity.stored_bytes_per_frame {
                return Err(PayloadRefusal::StoredSize(count));
            }
        }
        if decoded > self.capacity.decoded_bytes_per_frame {
            return Err(PayloadRefusal::DecodedSize(decoded));
        }
        self.admit_total(decoded)
            .map_err(PayloadRefusal::TotalDecoded)
    }

    pub(super) const fn total_refused(&self) -> bool {
        self.refused
    }

    pub(super) fn charge_resync(&mut self, decoded: u64) -> Result<(), u64> {
        self.charged = self.admit_total(decoded)?;
        Ok(())
    }

    pub(super) fn charge_parity_candidate(&mut self, length: u64) -> Result<(), PayloadRefusal> {
        self.charged = self.admit([length, 0, 0], length)?;
        Ok(())
    }

    fn admit_total(&mut self, decoded: u64) -> Result<u64, u64> {
        let total = self.charged.checked_add(decoded);
        if !self.refused
            && let Some(total) = total
            && total <= self.capacity.decoded_bytes_per_read
        {
            return Ok(total);
        }
        self.refused = true;
        Err(total.unwrap_or(u64::MAX))
    }
}

/// Result of the check over the original stored bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoredState {
    /// The computed stored check equals the declared check.
    Matches,
    /// The computed stored check differs from the declared check.
    Mismatch,
}

/// A payload that was left unchecked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PayloadRefusal {
    /// First stored payload, content or signature length above the limit.
    StoredSize(u64),
    /// Declared decoded length above the per-frame limit.
    DecodedSize(u64),
    /// Declared aggregate decoded length, saturated to the wire count range.
    TotalDecoded(u64),
    /// An unimplemented codec in a newer registry.
    UnknownCodec(u64),
    /// An unimplemented storage alternative in a newer registry.
    UnknownStorage(u64),
}

/// A failed payload check under P1 §5.4.
#[derive(Debug)]
#[non_exhaustive]
pub enum PayloadDamage {
    /// A payload field or the codec header violates wire grammar.
    Malformed,
    /// The native decoder failed, retaining its original reason.
    DecodeFailed(FrameDecoderError),
    /// The compressed input continued after the frame.
    TrailingCompressedBytes,
    /// The plain payload has a different length from the declaration.
    DecodedLength,
    /// The decoded bytes have a different digest from the declaration.
    PayloadDigest,
}

/// Payload check state, retaining decoded bytes only on verification.
#[derive(Debug)]
pub enum PayloadState<'a> {
    /// Capacity, length and digest checks succeeded.
    Verified(Cow<'a, [u8]>),
    /// A payload check failed.
    Damaged(PayloadDamage),
    /// Capacity or the codec prevented the payload check.
    Unchecked(PayloadRefusal),
}

/// Independent stored and decoded checks for an envelope.
#[derive(Debug)]
#[non_exhaustive]
pub struct PayloadCheck<'a> {
    /// Stored-byte check when the storage form provides one.
    pub stored: Option<StoredState>,
    /// First violation from a field needed for payload admission.
    pub malformed: Option<WireError>,
    /// The reader's payload result before interpreting content.
    pub state: PayloadState<'a>,
}

/// A host resource refusal preparing the bounded decoder output.
#[derive(Debug)]
#[non_exhaustive]
pub enum PayloadResourceError {
    /// The declared length exceeds the host address space.
    AddressSpace(u64),
    /// The host allocator refused the bounded output buffer.
    Reserve(TryReserveError),
}

impl fmt::Display for PayloadResourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AddressSpace(length) => write!(
                f,
                "GTS v2 decoded length {length} exceeds the host address space"
            ),
            Self::Reserve(cause) => write!(f, "could not reserve GTS v2 decoder output ({cause})"),
        }
    }
}

impl std::error::Error for PayloadResourceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::AddressSpace(_) => None,
            Self::Reserve(cause) => Some(cause),
        }
    }
}

impl purrdf_hash::error::StructuredError for PayloadResourceError {
    purrdf_hash::error_visit! {
        AddressSpace(length) => "gts-v2-payload-address-space" { "length" => ByteCount(u128::from(*length)) },
        Reserve(cause) => "gts-v2-payload-reservation" { "cause" => Cause(cause) },
    }
}

enum Storage<'a> {
    Plain,
    Compressed {
        codec: NonZeroU64,
        check: &'a [u8; STORED_CHECK_LENGTH],
    },
    Unknown(u64),
}

pub(super) struct PayloadWire<'a> {
    fields: Result<DecodedFields<'a>, WireError>,
    stored: Option<(NonZeroU64, &'a [u8; STORED_CHECK_LENGTH], &'a [u8])>,
    forbidden: Option<WireError>,
}

impl PayloadWire<'_> {
    pub(super) fn stored_check(&self) -> Option<StoredState> {
        self.stored
            .map(|(codec, check, payload)| check_stored(codec, check, payload))
    }
}

struct DecodedFields<'a> {
    length: u64,
    digest: &'a [u8; DIGEST_LENGTH],
    storage: Storage<'a>,
    payload: &'a [u8],
}

impl PayloadWire<'_> {
    pub(super) fn grammar_error(&self) -> Option<WireError> {
        self.forbidden
            .or_else(|| self.fields.as_ref().err().copied())
    }
}

impl<'a> FrameEnvelope<'a> {
    pub(super) fn payload_wire(&self, registry: NonZeroU64) -> PayloadWire<'a> {
        let length = self.read_field(FrameField::DecodedLength).uint();
        let digest = self
            .read_field(FrameField::PayloadDigest)
            .fixed_bytes::<DIGEST_LENGTH>();
        let storage = read_storage(&mut self.read_field(FrameField::Storage), registry);
        let payload = self.read_field(FrameField::Payload).bytes();
        let forbidden = (length == Ok(0)
            && digest.is_ok()
            && matches!(storage.as_ref(), Ok(Storage::Compressed { .. })))
        .then(|| {
            WireError::new(
                MalformedReason::ForbiddenCombination,
                self.start(FrameField::Storage),
            )
        });
        let stored =
            storage
                .as_ref()
                .ok()
                .zip(payload.as_ref().ok())
                .and_then(|(storage, payload)| {
                    if let Storage::Compressed { codec, check } = *storage {
                        Some((codec, check, *payload))
                    } else {
                        None
                    }
                });
        let fields = match (length, digest, storage, payload) {
            (Ok(length), Ok(digest), Ok(storage), Ok(payload)) => Ok(DecodedFields {
                length,
                digest,
                storage,
                payload,
            }),
            (Err(error), ..)
            | (_, Err(error), ..)
            | (_, _, Err(error), _)
            | (_, _, _, Err(error)) => Err(error),
        };
        PayloadWire {
            fields,
            stored,
            forbidden,
        }
    }

    pub(super) fn check_payload(
        &self,
        wire: PayloadWire<'a>,
        budget: &mut PayloadBudget,
    ) -> Result<PayloadCheck<'a>, PayloadResourceError> {
        let stored = wire.stored_check();
        self.check_decoded_payload(wire, stored, budget)
    }

    pub(super) fn check_decoded_payload(
        &self,
        wire: PayloadWire<'a>,
        stored: Option<StoredState>,
        budget: &mut PayloadBudget,
    ) -> Result<PayloadCheck<'a>, PayloadResourceError> {
        let mut malformed = wire.grammar_error();
        let Ok(DecodedFields {
            length: decoded,
            digest,
            storage,
            payload,
        }) = wire.fields
        else {
            return Ok(PayloadCheck {
                stored,
                malformed,
                state: PayloadState::Damaged(PayloadDamage::Malformed),
            });
        };
        let lengths = [
            payload.len() as u64,
            self.field(FrameField::Content).len() as u64,
            self.field(FrameField::Signatures).len() as u64,
        ];
        let state = match budget.admit(lengths, decoded) {
            Err(refusal) => PayloadState::Unchecked(refusal),
            Ok(charge) => {
                let state = match storage {
                    Storage::Plain => check_plain(decoded, digest, payload),
                    Storage::Unknown(tag) => {
                        PayloadState::Unchecked(PayloadRefusal::UnknownStorage(tag))
                    }
                    Storage::Compressed { codec, .. } if codec.get() != ZSTD => {
                        PayloadState::Unchecked(PayloadRefusal::UnknownCodec(codec.get()))
                    }
                    Storage::Compressed { .. } => {
                        if matches_zstd_header(decoded, payload) {
                            decode_zstd(payload, decoded, digest)?
                        } else {
                            malformed.get_or_insert_with(|| {
                                WireError::new(
                                    MalformedReason::CodecConstraint,
                                    self.start(FrameField::Payload),
                                )
                            });
                            PayloadState::Damaged(PayloadDamage::Malformed)
                        }
                    }
                };
                budget.charged = charge;
                state
            }
        };
        Ok(PayloadCheck {
            stored,
            malformed,
            state,
        })
    }
}

fn read_storage<'a>(
    fields: &mut Fields<'a>,
    registry: NonZeroU64,
) -> Result<Storage<'a>, WireError> {
    let offset = fields.offset();
    let count = fields.array()?;
    let wrong_length = WireError::new(MalformedReason::WrongLength, offset);
    if count == 0 {
        return Err(wrong_length);
    }
    let tag_offset = fields.offset();
    match fields.uint()? {
        1 if count == 1 => Ok(Storage::Plain),
        2 if count == 3 => Ok(Storage::Compressed {
            codec: fields.registry_value(registry, ZSTD)?,
            check: fields.fixed_bytes()?,
        }),
        1 | 2 => Err(wrong_length),
        tag if tag != 0 && registry.get() > REGISTRY_VERSION => {
            for _ in 1..count {
                fields.closed_value()?;
            }
            Ok(Storage::Unknown(tag))
        }
        _ => Err(WireError::new(
            MalformedReason::UnregisteredValue,
            tag_offset,
        )),
    }
}

fn check_stored(
    codec: NonZeroU64,
    expected: &[u8; STORED_CHECK_LENGTH],
    payload: &[u8],
) -> StoredState {
    if &stored_digest(codec, payload) == expected {
        StoredState::Matches
    } else {
        StoredState::Mismatch
    }
}

pub(super) fn stored_digest(codec: NonZeroU64, payload: &[u8]) -> [u8; STORED_CHECK_LENGTH] {
    let mut hasher = Hasher::new_derive_key("gts2 stored");
    hasher.update(&[0x81]);
    head::with_encoded_head(head::UNSIGNED, codec.get(), |head| {
        hasher.update(head);
    });
    hasher.update(payload);
    let digest = hasher.finalize();
    core::array::from_fn(|index| digest.as_bytes()[index])
}

fn check_plain<'a>(
    length: u64,
    expected: &[u8; DIGEST_LENGTH],
    payload: &'a [u8],
) -> PayloadState<'a> {
    if payload.len() as u64 == length {
        check_digest(Cow::Borrowed(payload), expected)
    } else {
        PayloadState::Damaged(PayloadDamage::DecodedLength)
    }
}

fn check_digest<'a>(decoded: Cow<'a, [u8]>, expected: &[u8; DIGEST_LENGTH]) -> PayloadState<'a> {
    if payload_digest(&decoded) == *expected {
        PayloadState::Verified(decoded)
    } else {
        PayloadState::Damaged(PayloadDamage::PayloadDigest)
    }
}

pub(super) fn payload_digest(bytes: &[u8]) -> [u8; DIGEST_LENGTH] {
    derive_key("gts2 payload", bytes)
}

pub(super) fn matches_zstd_header(length: u64, payload: &[u8]) -> bool {
    let Some(body) = payload.strip_prefix(&[0x28, 0xb5, 0x2f, 0xfd]) else {
        return false;
    };
    let (prefix, size, width): (&[u8], _, _) = match length {
        0 => return false,
        1..=255 => (&[0x20], length, 1),
        256..=65_791 => (&[0x60], length - 256, 2),
        65_792..=67_108_864 => (&[0xa0], length, 4),
        67_108_865..=4_294_967_294 => (&[0x80, 0x80], length, 4),
        _ => (&[0xc0, 0x80], length, 8),
    };
    body.strip_prefix(prefix)
        .is_some_and(|body| body.starts_with(&size.to_le_bytes()[..width]))
}

fn decode_zstd<'a>(
    mut input: &[u8],
    length: u64,
    expected: &[u8; DIGEST_LENGTH],
) -> Result<PayloadState<'a>, PayloadResourceError> {
    let length = usize::try_from(length).map_err(|_| PayloadResourceError::AddressSpace(length))?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(length)
        .map_err(PayloadResourceError::Reserve)?;
    output.resize(length, 0);
    match FrameDecoder::new().decode_frame(&mut input, &mut output) {
        Ok(_) if !input.is_empty() => Ok(PayloadState::Damaged(
            PayloadDamage::TrailingCompressedBytes,
        )),
        Ok(produced) if produced == length => Ok(check_digest(Cow::Owned(output), expected)),
        Ok(produced) => Ok(PayloadState::Damaged(PayloadDamage::DecodeFailed(
            FrameDecoderError::FrameContentSizeMismatch {
                declared: length as u64,
                produced: produced as u64,
            },
        ))),
        Err(cause) => Ok(PayloadState::Damaged(PayloadDamage::DecodeFailed(cause))),
    }
}
