// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use purrdf_hash::blake3::Hasher;
use purrdf_lex::cbor::head;

use super::{HEADER_CONTEXT, Header, read_digest, read_lineage, read_registry_profile};
use crate::v2::fields::Fields;
use crate::v2::{BLAKE3_256, GENERATION, Item, MalformedReason, RESYNC_KIND, WireError};

const MAX_BYTES: usize = 4096;
const MAX_MARKS: u64 = 64;
const MARKER: &[u8; 16] = &[
    0xfc, 0xf3, 0xeb, 0x32, 0xa2, 0xf0, 0x26, 0x7d, 0xd1, 0x6a, 0x53, 0xf0, 0x9b, 0xff, 0x87, 0x92,
];

impl<'a> Header<'a> {
    pub(in crate::v2) fn decode_resync(item: Item<'a>) -> Result<Self, WireError> {
        let bytes = item.bytes();
        let limit = WireError::new(MalformedReason::OutOfRange, MAX_BYTES);
        match read_resync(bytes) {
            Err(error) if bytes.len() > MAX_BYTES && error.offset >= MAX_BYTES => Err(limit),
            result => result,
        }
    }
}

fn read_resync(bytes: &[u8]) -> Result<Header<'_>, WireError> {
    let mut fields = Fields::new(bytes);
    fields.record(11)?;
    fields.fixed_uint(RESYNC_KIND, MalformedReason::WrongType)?;
    fields.fixed_uint(GENERATION, MalformedReason::OutOfRange)?;
    let marker_offset = fields.offset();
    if fields.bytes()? != MARKER {
        return Err(WireError::new(MalformedReason::OutOfRange, marker_offset));
    }
    let (registry_version, hash_algorithm, profile) = read_registry_profile(&mut fields)?;
    let holds_hash = hash_algorithm.get() == BLAKE3_256;
    let lineage = read_lineage(&mut fields, holds_hash)?;
    fields.uint()?;
    fields.option(|fields| read_digest(fields, holds_hash))?;
    read_marks(&mut fields)?;
    let identifier_offset = fields.offset();
    read_digest(&mut fields, holds_hash)?;
    if bytes.len() > MAX_BYTES {
        return Err(WireError::new(MalformedReason::OutOfRange, MAX_BYTES));
    }
    let identifier = holds_hash.then(|| {
        let mut hasher = Hasher::new_derive_key(HEADER_CONTEXT);
        hasher.update(&[0x8a]);
        hasher.update(&bytes[1..identifier_offset]);
        *hasher.finalize().as_bytes()
    });
    Ok(Header {
        registry_version,
        hash_algorithm,
        profile,
        canonical: None,
        lineage,
        identifier,
    })
}

fn read_marks(fields: &mut Fields<'_>) -> Result<(), WireError> {
    let count = fields.array()?;
    let mut previous: Option<&[u8]> = None;
    for index in 0..count {
        let offset = fields.offset();
        if offset >= MAX_BYTES {
            return Err(WireError::new(MalformedReason::OutOfRange, MAX_BYTES));
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
        fields.closed_value_until(MAX_BYTES)?;
        previous = Some(&remaining[..fields.offset() - offset]);
    }
    Ok(())
}
