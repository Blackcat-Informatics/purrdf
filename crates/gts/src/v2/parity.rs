// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::{fmt, num::NonZeroU64, ops::Range};
use std::collections::TryReserveError;

use purrdf_hash::blake3::Hasher;

use super::fields::Fields;
use super::{
    DIGEST_LENGTH, FrameCheck, FrameEnvelope, FrameField, LinkState, PayloadRefusal, PayloadState,
    REGISTRY_VERSION, WireError,
};
use crate::FastSet;

mod encode;
mod stream;

pub(super) use encode::{Recorder, RecordingWriter};
pub(super) use stream::ParityStream;
pub use stream::{ParityFinding, ParityFindingKind};

type Digest = [u8; DIGEST_LENGTH];
pub(super) const GEOMETRY_ONE: u64 = 1;
const SPAN_BYTES: usize = 4 * 1024 * 1024;
const MAX_SHARD_BYTES: usize = 4096;
const DATA_COLUMNS: usize = 64;
const PARITY_ROWS: usize = 4;
const MASK_WIDTH: usize = u64::BITS as usize;
const DATA_MASKS: usize = (SPAN_BYTES / MAX_SHARD_BYTES).div_ceil(MASK_WIDTH);
const SHARD_DIGEST_BYTES: usize = 16;
// Every digest in a table has the same one-byte deterministic CBOR head.
const DIGEST_ENTRY_BYTES: usize = 1 + SHARD_DIGEST_BYTES;
type DigestEntry = [u8; DIGEST_ENTRY_BYTES];
const ZERO_PADDING: [u8; MAX_SHARD_BYTES] = [0; MAX_SHARD_BYTES];

/// The result of checking a parity frame before judging its placement in the stream.
#[derive(Debug)]
#[non_exhaustive]
pub enum ParityCheck<'a> {
    /// No verified table can be used from the frame.
    Damaged,
    /// A payload capacity prevented checking an otherwise admissible frame.
    Unchecked(PayloadRefusal),
    /// The frame uses a parity geometry defined by a later registry.
    UnknownGeometry {
        /// The unknown geometry code.
        code: u64,
        /// Whether this frame produces an unknown-enumeration finding.
        report: bool,
    },
    /// A verified table and its associated parity bytes.
    Table {
        /// The table eligible for the stream placement and shard checks.
        table: ParityTable<'a>,
        /// The independent frame payload check failed.
        payload_damaged: bool,
    },
}

/// A borrowed parity table verified against its digest and segment opener.
#[derive(Debug)]
pub struct ParityTable<'a> {
    span: Range<u64>,
    geometry: Geometry,
    data_digests: &'a [DigestEntry],
    parity_digests: &'a [DigestEntry],
    parity: &'a [u8],
}

impl ParityTable<'_> {
    /// The offsets of the covered span in the logical segment stream.
    #[must_use]
    pub fn span(&self) -> Range<u64> {
        self.span.clone()
    }

    /// Compare the stored data and parity shards with their table entries.
    ///
    /// Supply the exact span from the logical segment stream, in stream
    /// order, after checking the parity frame's placement.
    ///
    /// # Errors
    /// Returns the expected and supplied span lengths when they differ.
    pub fn compare_shards(&self, span: &[u8]) -> Result<ParityMismatches, ParitySpanLength> {
        if span.len() != self.geometry.span_bytes {
            return Err(ParitySpanLength {
                expected: self.geometry.span_bytes,
                supplied: span.len(),
            });
        }

        Ok(self.compare_stream_shards(core::iter::once(span)))
    }

    fn compare_stream_shards<'a>(
        &self,
        mut parts: impl Iterator<Item = &'a [u8]>,
    ) -> ParityMismatches {
        let initial = shard_hasher();
        let mut part = &[][..];
        let mut remaining = self.geometry.span_bytes;
        let size = self.geometry.shard_bytes;
        let mut data = [0; DATA_MASKS];
        for (shard, digest) in self.data_digests.iter().enumerate() {
            let mut hasher = initial.clone();
            let mut needed = remaining.min(size);
            remaining -= needed;
            let padding = size - needed;
            while needed != 0 {
                if part.is_empty() {
                    // The caller supplies exactly the span's admitted bytes.
                    part = parts.next().expect("parity span has remaining bytes");
                }
                let length = needed.min(part.len());
                let (prefix, rest) = part.split_at(length);
                hasher.update(prefix);
                part = rest;
                needed -= length;
            }
            hasher.update(&ZERO_PADDING[..padding]);
            if !matches_shard(&hasher, digest) {
                data[shard / MASK_WIDTH] |= 1 << (shard % MASK_WIDTH);
            }
        }
        let mut parity = 0;
        for (shard, (bytes, digest)) in self
            .parity
            .chunks_exact(size)
            .zip(self.parity_digests)
            .enumerate()
        {
            let mut hasher = initial.clone();
            hasher.update(bytes);
            if !matches_shard(&hasher, digest) {
                parity |= 1 << shard;
            }
        }
        ParityMismatches { data, parity }
    }
}

/// Shard indices whose bytes disagree with a verified parity table.
#[derive(Debug, PartialEq, Eq)]
pub struct ParityMismatches {
    data: [u64; DATA_MASKS],
    parity: u64,
}

impl ParityMismatches {
    /// Failing data shard indices in table order.
    pub fn data_shards(&self) -> impl Iterator<Item = usize> + '_ {
        self.data.iter().enumerate().flat_map(|(word, mask)| {
            (0..MASK_WIDTH)
                .filter(move |bit| mask & (1 << bit) != 0)
                .map(move |bit| word * MASK_WIDTH + bit)
        })
    }

    /// Failing parity shard indices in table order.
    pub fn parity_shards(&self) -> impl Iterator<Item = usize> + '_ {
        (0..MASK_WIDTH).filter(|bit| self.parity & (1 << bit) != 0)
    }
}

/// The supplied bytes do not cover the table's declared span.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ParitySpanLength {
    /// Number of bytes indicated by the table.
    pub expected: usize,
    /// Number of bytes supplied for verification.
    pub supplied: usize,
}

impl fmt::Display for ParitySpanLength {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GTS parity span needs {} bytes, received {}",
            self.expected, self.supplied
        )
    }
}

impl std::error::Error for ParitySpanLength {}

impl purrdf_hash::error::StructuredError for ParitySpanLength {
    fn visit<'error>(&'error self, visitor: &mut dyn purrdf_hash::error::ErrorVisitor<'error>) {
        use purrdf_hash::error::ErrorValue;
        visitor.variant("gts-v2-parity-span-length");
        visitor.value(
            Some("expected"),
            ErrorValue::ByteCount(self.expected as u128),
        );
        visitor.value(
            Some("supplied"),
            ErrorValue::ByteCount(self.supplied as u128),
        );
    }
}

fn matches_shard(hasher: &Hasher, entry: &DigestEntry) -> bool {
    hasher.finalize().as_bytes()[..SHARD_DIGEST_BYTES] == entry[1..]
}

fn shard_hasher() -> Hasher {
    Hasher::new_derive_key("gts2 shard")
}

fn parity_table_digest(opener: &Digest, digest_lists: &[u8]) -> Digest {
    let mut hasher = Hasher::new_derive_key("gts2 parity table");
    hasher.update(&[0x83, 0x58, DIGEST_LENGTH as u8]);
    hasher.update(opener);
    hasher.update(digest_lists);
    *hasher.finalize().as_bytes()
}

#[derive(Debug)]
struct Geometry {
    span_bytes: usize,
    shard_bytes: usize,
    data_shards: usize,
    parity_shards: usize,
}

impl Geometry {
    fn for_span(first: u64, end: u64) -> Option<Self> {
        let span_bytes = usize::try_from(end.checked_sub(first)?).ok()?;
        if !first.is_multiple_of(SPAN_BYTES as u64) || !(1..=SPAN_BYTES).contains(&span_bytes) {
            return None;
        }
        let shard_bytes = span_bytes
            .div_ceil(DATA_COLUMNS)
            .clamp(64, MAX_SHARD_BYTES)
            .next_power_of_two();
        let data_shards = span_bytes.div_ceil(shard_bytes);
        let parity_shards = PARITY_ROWS * data_shards.div_ceil(DATA_COLUMNS);
        Some(Self {
            span_bytes,
            shard_bytes,
            data_shards,
            parity_shards,
        })
    }
}

pub(super) struct ParityClaim<'a> {
    geometry: u64,
    first: u64,
    end: u64,
    anchor: &'a Digest,
    table_digest: &'a Digest,
}

impl<'a> ParityClaim<'a> {
    pub(super) fn read(fields: &mut Fields<'a>) -> Result<Self, WireError> {
        fields.record(5)?;
        Ok(Self {
            geometry: fields.uint()?,
            first: fields.uint()?,
            end: fields.uint()?,
            anchor: fields.fixed_bytes()?,
            table_digest: fields.fixed_bytes()?,
        })
    }

    fn read_table(
        &self,
        payload: &'a [u8],
        geometry: Geometry,
        opener: &Digest,
    ) -> Option<ParityTable<'a>> {
        let mut fields = Fields::new(payload);
        fields.record(3).ok()?;
        let start = fields.offset();
        let data_digests = read_digests(payload, &mut fields, geometry.data_shards).ok()?;
        let parity_digests = read_digests(payload, &mut fields, geometry.parity_shards).ok()?;
        let end = fields.offset();
        let parity = fields.bytes().ok()?;
        fields.finish().ok()?;
        if parity.len() != geometry.shard_bytes * geometry.parity_shards {
            return None;
        }
        if &parity_table_digest(opener, &payload[start..end]) != self.table_digest {
            return None;
        }

        Some(ParityTable {
            span: self.first..self.end,
            geometry,
            data_digests,
            parity_digests,
            parity,
        })
    }
}

fn read_digests<'a>(
    bytes: &'a [u8],
    fields: &mut Fields<'a>,
    count: usize,
) -> Result<&'a [DigestEntry], WireError> {
    fields.record(count as u64)?;
    let start = fields.offset();
    for _ in 0..count {
        fields.fixed_bytes::<SHARD_DIGEST_BYTES>()?;
    }
    // Each parsed digest consists of its one-byte head and 16 data bytes.
    Ok(bytes[start..fields.offset()].as_chunks().0)
}

#[derive(Debug)]
pub(super) struct ParityHistory {
    opener: Digest,
    starts: FastSet<u64>,
    seen_short: bool,
}

pub(super) struct ParityObservation<'a> {
    check: ParityCheck<'a>,
    first_to_record: Option<u64>,
    short: bool,
}

impl<'a> ParityObservation<'a> {
    const fn without_span(check: ParityCheck<'a>) -> Self {
        Self {
            check,
            first_to_record: None,
            short: false,
        }
    }
}

impl ParityHistory {
    pub(super) fn new(opener: Digest) -> Self {
        Self {
            opener,
            starts: FastSet::default(),
            seen_short: false,
        }
    }

    pub(super) fn inspect<'a>(
        &mut self,
        frame: &FrameEnvelope<'a>,
        checked: &FrameCheck<'a>,
        predecessor: &Digest,
        registry: NonZeroU64,
        content_stopped: bool,
    ) -> Result<ParityObservation<'a>, TryReserveError> {
        if checked.link.state == LinkState::Broken {
            return Ok(ParityObservation::without_span(ParityCheck::Damaged));
        }
        let Ok(claim) = ParityClaim::read(&mut frame.read_field(FrameField::Content)) else {
            return Ok(ParityObservation::without_span(ParityCheck::Damaged));
        };
        let envelope_damaged = checked.parity_envelope_damaged || checked.malformed.is_some();
        if !envelope_damaged && registry.get() > REGISTRY_VERSION && claim.geometry > GEOMETRY_ONE {
            return Ok(ParityObservation::without_span(
                ParityCheck::UnknownGeometry {
                    code: claim.geometry,
                    report: !content_stopped
                        && matches!(checked.payload.state, PayloadState::Verified(_)),
                },
            ));
        }

        let blocked = self.seen_short || self.starts.contains(&claim.first);
        let first_to_record = if blocked {
            None
        } else {
            self.starts.try_reserve(1)?;
            Some(claim.first)
        };
        let short = claim
            .end
            .checked_sub(claim.first)
            .is_some_and(|length| (1..SPAN_BYTES as u64).contains(&length));
        let check = if blocked
            || envelope_damaged
            || claim.geometry != GEOMETRY_ONE
            || claim.anchor != predecessor
        {
            ParityCheck::Damaged
        } else {
            self.check_table(frame, checked, &claim)
        };
        Ok(ParityObservation {
            check,
            first_to_record,
            short,
        })
    }

    fn check_table<'a>(
        &self,
        frame: &FrameEnvelope<'a>,
        checked: &FrameCheck<'a>,
        claim: &ParityClaim<'a>,
    ) -> ParityCheck<'a> {
        let Some(geometry) = Geometry::for_span(claim.first, claim.end) else {
            return ParityCheck::Damaged;
        };
        if let PayloadState::Unchecked(reason) = checked.payload.state {
            return ParityCheck::Unchecked(reason);
        }
        let Ok(payload) = frame.read_field(FrameField::Payload).bytes() else {
            return ParityCheck::Damaged;
        };
        claim
            .read_table(payload, geometry, &self.opener)
            .map_or(ParityCheck::Damaged, |table| ParityCheck::Table {
                table,
                payload_damaged: matches!(checked.payload.state, PayloadState::Damaged(_)),
            })
    }

    pub(super) fn record<'a>(&mut self, observation: ParityObservation<'a>) -> ParityCheck<'a> {
        if let Some(first) = observation.first_to_record {
            self.starts.insert(first);
        }
        self.seen_short |= observation.short;
        observation.check
    }
}
