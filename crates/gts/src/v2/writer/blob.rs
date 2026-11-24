// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::io::{self, Write};
use std::sync::OnceLock;

use purrdf_hash::blake3::Hasher;
use purrdf_lex::cbor::head;

use super::{
    Buffer, FrameType, RangeFull, SegmentWriteError, SegmentWriter, ValueName, write_option,
};

const WINDOW: usize = 64;
const MIN_CHUNK: usize = 16_384;
const AVERAGE_CHUNK: usize = 65_536;
const MAX_CHUNK: usize = 262_144;
const STRICT_MASK_LIMIT: u64 = 1 << 46;
const LOOSE_MASK_LIMIT: u64 = 1 << 50;

static GEAR_TABLE: OnceLock<[u64; 256]> = OnceLock::new();

#[derive(Clone, Copy)]
pub(super) enum RecipeKind<'a> {
    Blob(Option<&'a str>),
    Value,
}

impl<W: Write> SegmentWriter<W> {
    /// Append a blob chunked by the generation 2 cut rule.
    ///
    /// A blob with at most one unit is stored whole. Otherwise the writer
    /// emits the units before their recipe.
    ///
    /// # Errors
    /// Returns an allocation, range or sink error.
    pub fn add_chunked_blob(
        &mut self,
        bytes: &[u8],
        media_type: Option<&str>,
    ) -> Result<ValueName, SegmentWriteError> {
        let first = cut(bytes);
        if first == bytes.len() {
            let digest = self.add_blob(bytes, media_type)?;
            return Ok(ValueName {
                digest,
                length: bytes.len() as u64,
            });
        }
        self.write_recipe(bytes, first, FrameType::Recipe, media_type)
    }

    /// Append a nonempty value and return the name a term can reference.
    ///
    /// The value's units precede its declaration in the segment.
    ///
    /// # Errors
    /// Returns an empty-value, allocation, range or sink error.
    pub fn add_value(&mut self, bytes: &[u8]) -> Result<ValueName, SegmentWriteError> {
        if bytes.is_empty() {
            return Err(SegmentWriteError::EmptyValue);
        }
        self.write_recipe(bytes, cut(bytes), FrameType::Value, None)
    }

    fn write_recipe(
        &mut self,
        bytes: &[u8],
        first: usize,
        kind: FrameType,
        media_type: Option<&str>,
    ) -> Result<ValueName, SegmentWriteError> {
        if self.head.is_none() {
            return Err(SegmentWriteError::IncompleteFrame);
        }
        let mut parts = Vec::new();
        let mut remaining = bytes;
        let mut length = first;
        while !remaining.is_empty() {
            let (unit, rest) = remaining.split_at(length);
            parts.try_reserve(1).map_err(io::Error::other)?;
            parts.push((ValueName::of(unit), unit));
            remaining = rest;
            length = cut(remaining);
        }
        let count = parts.len() as u64;
        if u128::from(self.range.count().get()) + u128::from(count) + 1 > u128::from(u64::MAX) {
            return Err(SegmentWriteError::RangeFull(RangeFull));
        }
        let payload = recipe_payload(parts.iter().map(|&(part, _)| part))?;
        for (part, unit) in parts {
            self.append_chunk(unit, &part.digest)?;
        }
        let name = ValueName::of(bytes);
        let recipe = if kind == FrameType::Recipe {
            RecipeKind::Blob(media_type)
        } else {
            RecipeKind::Value
        };
        self.append(kind, &payload.0, |output| {
            recipe_content(output, name, recipe, count)
        })?;
        Ok(name)
    }
}

pub(super) fn recipe_payload(
    parts: impl ExactSizeIterator<Item = ValueName>,
) -> io::Result<Buffer> {
    let count = parts.len();
    let mut payload = Buffer::default();
    // A CBOR head needs at most nine bytes. Each entry needs one byte for
    // the array, 34 for the digest and up to nine for the length.
    payload
        .0
        .try_reserve_exact(count.saturating_mul(44).saturating_add(9))
        .map_err(io::Error::other)?;
    head::write_head(&mut payload, head::ARRAY, count as u64)?;
    for part in parts {
        head::write_head(&mut payload, head::ARRAY, 2)?;
        head::write_bytes(&mut payload, &part.digest)?;
        head::write_head(&mut payload, head::UNSIGNED, part.length)?;
    }
    Ok(payload)
}

pub(super) fn recipe_content(
    output: &mut impl Write,
    name: ValueName,
    kind: RecipeKind<'_>,
    count: u64,
) -> io::Result<()> {
    head::write_head(
        output,
        head::ARRAY,
        if matches!(kind, RecipeKind::Blob(_)) {
            4
        } else {
            3
        },
    )?;
    head::write_bytes(output, &name.digest)?;
    head::write_head(output, head::UNSIGNED, name.length)?;
    if let RecipeKind::Blob(media) = kind {
        write_option(output, media, head::write_text)?;
    }
    head::write_head(output, head::UNSIGNED, count)
}

pub(super) fn cut(bytes: &[u8]) -> usize {
    if bytes.len() <= MIN_CHUNK {
        return bytes.len();
    }
    let max = bytes.len().min(MAX_CHUNK);
    let gear = GEAR_TABLE.get_or_init(|| {
        let mut xof = Hasher::new_derive_key("gts2 gear table").finalize_xof();
        core::array::from_fn(|_| {
            let mut word = [0; 8];
            xof.fill(&mut word);
            u64::from_be_bytes(word)
        })
    });
    let mut fingerprint = 0_u64;
    let start = MIN_CHUNK - WINDOW;
    // The final candidate is the input end or the limit. Both cut without
    // testing the mask, so only the bytes before it can make an earlier cut.
    for (index, &byte) in bytes[start..max - 1].iter().enumerate() {
        fingerprint = (fingerprint << 1).wrapping_add(gear[usize::from(byte)]);
        let length = start + index + 1;
        let mask_limit = if length < AVERAGE_CHUNK {
            STRICT_MASK_LIMIT
        } else {
            LOOSE_MASK_LIMIT
        };
        if length >= MIN_CHUNK && fingerprint < mask_limit {
            return length;
        }
    }
    max
}
