// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::{fmt, ops::Range};

use purrdf_hash::blake3::Hasher;
use purrdf_lex::cbor::{DecodeErrorKind, StructuralError, head};

use super::{HEADER_CONTEXT, Header, read_digest, read_lineage, read_profile, read_registry_hash};
use crate::v2::fields::Fields;
use crate::v2::{
    BLAKE3_256, GENERATION, Item, MalformedReason, PayloadBudget, RESYNC_KIND, RESYNC_MAX_BYTES,
    WireError,
};

const MAX_MARKS: u64 = 64;
const PATTERN: &[u8; 20] = &[
    0x8b, 0x05, 0x02, 0x50, 0xfc, 0xf3, 0xeb, 0x32, 0xa2, 0xf0, 0x26, 0x7d, 0xd1, 0x6a, 0x53, 0xf0,
    0x9b, 0xff, 0x87, 0x92,
];

/// A resync opener whose place, fields and identifier passed the candidate test.
#[derive(Debug)]
#[non_exhaustive]
pub struct ResyncCandidate<'a> {
    /// Start of the opener in the original file.
    pub offset: usize,
    /// Exact serialized opener, ending at the next item boundary.
    pub bytes: &'a [u8],
    /// Parsed opener, including the verified segment identifier.
    pub header: Header<'a>,
}

/// The first resync candidate refused by the total decoded-byte budget.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ResyncRefusal {
    /// Start of the occurrence whose charge was refused.
    pub offset: usize,
    /// Total decoded bytes including the attempted charge, saturated at the wire limit.
    pub requested_total: u64,
}

impl fmt::Display for ResyncRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GTS resync at byte {} would charge {} total decoded bytes",
            self.offset, self.requested_total
        )
    }
}

impl std::error::Error for ResyncRefusal {}

impl purrdf_hash::error::StructuredError for ResyncRefusal {
    fn visit<'error>(&'error self, visitor: &mut dyn purrdf_hash::error::ErrorVisitor<'error>) {
        use purrdf_hash::error::ErrorValue;
        visitor.variant("gts-v2-resync-total-decoded");
        visitor.value(Some("offset"), ErrorValue::ByteCount(self.offset as u128));
        visitor.value(
            Some("count"),
            ErrorValue::ByteCount(u128::from(self.requested_total)),
        );
    }
}

/// Test resync candidates in the runs selected by a GTS v2 framing walk.
///
/// Supply one scanner for a read. Runs arrive in file order and may overlap.
/// The framing walk decides when a run is eligible, including item capacity
/// and the exclusion of the preceding item when it was healed.
pub struct ResyncScan<'a> {
    bytes: &'a [u8],
    next: usize,
    max_depth: usize,
    depth_refusal: Option<(usize, usize, StructuralError)>,
}

impl fmt::Debug for ResyncScan<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResyncScan")
            .field("file_len", &self.bytes.len())
            .field("next", &self.next)
            .field("max_depth", &self.max_depth)
            .finish()
    }
}

impl<'a> ResyncScan<'a> {
    /// Start scanning runs in the original file.
    #[must_use]
    pub const fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            next: 0,
            max_depth: bytes.len(),
            depth_refusal: None,
        }
    }

    /// Start scanning runs under a caller-owned structural depth ceiling.
    pub(in crate::v2) const fn with_depth(bytes: &'a [u8], max_depth: usize) -> Self {
        Self {
            bytes,
            next: 0,
            max_depth,
            depth_refusal: None,
        }
    }

    /// First prefix byte not yet examined by this scanner.
    pub(in crate::v2) const fn next_offset(&self) -> usize {
        self.next
    }

    /// Take the first candidate framing refusal caused by the depth ceiling.
    pub(in crate::v2) fn take_depth_refusal(&mut self) -> Option<(usize, usize, StructuralError)> {
        self.depth_refusal.take()
    }

    /// Find the first acceptable occurrence starting in the supplied run.
    ///
    /// A candidate may extend past the run's end. An overlapping later run
    /// skips offsets already examined. All candidates use the read's shared
    /// payload budget, including those rejected after their place is checked.
    ///
    /// # Errors
    /// Returns the offset and attempted total on the first decoded-byte
    /// refusal. An exhausted total prevents further candidate tests.
    pub fn scan(
        &mut self,
        run: Range<usize>,
        budget: &mut PayloadBudget,
    ) -> Result<Option<ResyncCandidate<'a>>, ResyncRefusal> {
        let start = run.start.max(self.next);
        let end = run.end.min(self.bytes.len());
        if start >= end || budget.total_refused() {
            return Ok(None);
        }
        let lookahead = (end.saturating_add(PATTERN.len() - 1)).min(self.bytes.len());
        for (relative, window) in self.bytes[start..lookahead]
            .windows(PATTERN.len())
            .enumerate()
        {
            if window != PATTERN {
                continue;
            }
            let offset = start + relative;
            self.next = offset + 1;
            let bytes = &self.bytes[offset
                ..offset
                    .saturating_add(RESYNC_MAX_BYTES)
                    .min(self.bytes.len())];
            let item = match Item::first_with_depth(bytes, self.max_depth) {
                Ok(Some(item)) => item,
                Err(error) if matches!(error.kind(), DecodeErrorKind::Depth { .. }) => {
                    if self.depth_refusal.is_none() {
                        self.depth_refusal = Some((offset, self.max_depth, error));
                    }
                    continue;
                }
                Ok(None) | Err(_) => continue,
            };
            let bytes = item.bytes();
            let Ok(prefix) = ResyncPrefix::read(bytes, true) else {
                continue;
            };
            if prefix.place != offset as u64 {
                continue;
            }
            budget
                .charge_resync(RESYNC_MAX_BYTES as u64)
                .map_err(|requested_total| ResyncRefusal {
                    offset,
                    requested_total,
                })?;
            if let Ok((header, stored, len)) = prefix.read_tail(bytes)
                && header.identifier.as_ref().is_some_and(|id| stored == id)
            {
                return Ok(Some(ResyncCandidate {
                    offset,
                    bytes: &bytes[..len],
                    header,
                }));
            }
        }
        self.next = end;
        Ok(None)
    }
}

impl<'a> Header<'a> {
    pub(in crate::v2) fn decode_resync(item: Item<'a>) -> Result<Self, WireError> {
        let bytes = item.bytes();
        let limit = WireError::new(MalformedReason::OutOfRange, RESYNC_MAX_BYTES);
        let decoded = ResyncPrefix::read(bytes, false).and_then(|prefix| prefix.read_tail(bytes));
        match decoded {
            Err(error) if bytes.len() > RESYNC_MAX_BYTES && error.offset >= RESYNC_MAX_BYTES => {
                Err(limit)
            }
            result => result.map(|(header, _, _)| header),
        }
    }
}

struct ResyncPrefix<'a> {
    header: Header<'a>,
    place: u64,
    fields: Fields<'a>,
}

impl<'a> ResyncPrefix<'a> {
    fn read(bytes: &'a [u8], require_held_hash: bool) -> Result<Self, WireError> {
        let mut fields = Fields::new(bytes);
        fields.record(11)?;
        fields.fixed_uint(RESYNC_KIND, MalformedReason::WrongType)?;
        fields.fixed_uint(GENERATION, MalformedReason::OutOfRange)?;
        let marker_offset = fields.offset();
        if fields.bytes()? != &PATTERN[4..] {
            return Err(WireError::new(MalformedReason::OutOfRange, marker_offset));
        }
        let (registry_version, hash_algorithm) =
            read_registry_hash(&mut fields, require_held_hash)?;
        let profile = read_profile(&mut fields)?;
        let lineage = read_lineage(&mut fields, hash_algorithm.get() == BLAKE3_256)?;
        let place = fields.uint()?;
        let header = Header {
            registry_version,
            hash_algorithm,
            profile,
            canonical: None,
            lineage,
            identifier: None,
        };
        Ok(Self {
            header,
            place,
            fields,
        })
    }

    fn read_tail(mut self, bytes: &[u8]) -> Result<(Header<'a>, &'a [u8], usize), WireError> {
        let holds_hash = self.header.hash_algorithm.get() == BLAKE3_256;
        self.fields
            .option(|fields| read_digest(fields, holds_hash))?;
        read_marks(&mut self.fields)?;
        let identifier_offset = self.fields.offset();
        let stored = read_digest(&mut self.fields, holds_hash)?;
        let len = self.fields.offset();
        if len > RESYNC_MAX_BYTES {
            return Err(WireError::new(
                MalformedReason::OutOfRange,
                RESYNC_MAX_BYTES,
            ));
        }
        self.header.identifier = holds_hash.then(|| {
            let mut hasher = Hasher::new_derive_key(HEADER_CONTEXT);
            hasher.update(&[0x8a]);
            hasher.update(&bytes[1..identifier_offset]);
            *hasher.finalize().as_bytes()
        });
        Ok((self.header, stored, len))
    }
}

fn read_marks(fields: &mut Fields<'_>) -> Result<(), WireError> {
    let count = fields.array()?;
    let mut previous: Option<&[u8]> = None;
    for index in 0..count {
        let offset = fields.offset();
        if offset >= RESYNC_MAX_BYTES {
            return Err(WireError::new(
                MalformedReason::OutOfRange,
                RESYNC_MAX_BYTES,
            ));
        }
        let mut shape = *fields;
        let length = shape.array()?;
        let mut rest = shape.remaining();
        if length == 0 || head::read_argument(&mut rest, head::UNSIGNED).is_none() {
            return Err(WireError::new(MalformedReason::WrongType, offset));
        }
        if index == MAX_MARKS {
            return Err(WireError::new(MalformedReason::OutOfRange, offset));
        }
        let remaining = fields.remaining();
        if let Some(previous) = previous {
            // A complete CBOR item cannot be a proper prefix of another.
            // Its length bounds the comparison even if this mark crosses
            // the cap. The ordering violation belongs to the mark's start.
            let len = previous.len().min(remaining.len());
            let prefix = &remaining[..len];
            if prefix < &previous[..len] || prefix == previous {
                return Err(WireError::new(MalformedReason::UnorderedOrRepeated, offset));
            }
        }
        fields.closed_value_until(RESYNC_MAX_BYTES)?;
        previous = Some(&remaining[..fields.offset() - offset]);
    }
    Ok(())
}
