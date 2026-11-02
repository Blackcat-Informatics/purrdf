// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Dense physical value encodings; definition levels belong to the page layer.
//!
//! Wire authority: Apache Parquet, Data Pages / Encodings. Delta blocks use
//! wrapping arithmetic, LSB-first miniblocks and no bodies for unused miniblocks.

use core::{fmt, mem::size_of, ops::Range};

use purrdf_core::ir::pack::bits::{
    IntVector, IntVectorRef, bits_for, read_varint, write_varint, zigzag_decode, zigzag_encode,
};
use purrdf_lex::allocation::StorageError;

use crate::{ColumnarError, PhysicalType, ValueBodyReason};

pub(crate) const PLAIN: i32 = 0;
pub(crate) const DELTA_BINARY_PACKED: i32 = 5;
pub(crate) const DELTA_LENGTH_BYTE_ARRAY: i32 = 6;
pub(crate) const DELTA_BYTE_ARRAY: i32 = 7;
pub(crate) const RLE_DICTIONARY: i32 = 8;
const MAX_BYTES: usize = i32::MAX as usize;
const BLOCK: usize = 128;
const MINIBLOCKS: usize = 4;
const MINI: usize = BLOCK / MINIBLOCKS;

/// Original operands of a malformed physical value encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncodingReason {
    /// The shared packed-integer reader refused the adapted miniblock.
    Packed(purrdf_core::ir::pack::bits::PackBitsError),
    /// A delta block's dimensions violate the Parquet layout.
    DeltaLayout {
        /// Declared values per block.
        block: u64,
        /// Declared miniblocks per block.
        miniblocks: u64,
    },
    /// The encoded count differs from the page's present-value count.
    ValueCount {
        /// Original encoded population.
        declared: u64,
        /// Population established by the page.
        expected: usize,
    },
    /// A packed width exceeds its physical integer width.
    BitWidth {
        /// Original packed width.
        width: u8,
        /// Width of the physical integer.
        maximum: u8,
    },
    /// An index does not address the original dictionary.
    DictionaryIndex {
        /// Original decoded ordinal.
        index: u64,
        /// Original dictionary population.
        entries: usize,
    },
    /// A dictionary encoded page has no dictionary page.
    MissingDictionary,
    /// An encoder's page value is absent from its chunk dictionary.
    DictionaryValue {
        /// Present-value ordinal within the page.
        position: usize,
    },
    /// A delta length is negative or exceeds the signed PLAIN length field.
    Length {
        /// Original reconstructed length.
        value: i64,
    },
    /// A prefix extends beyond the preceding reconstructed value.
    Prefix {
        /// Original reconstructed prefix length.
        length: i64,
        /// Available bytes in the preceding value.
        previous: usize,
    },
    /// A signed delta header does not fit its physical integer width.
    IntegerRange {
        /// Original signed header.
        value: i64,
        /// Width of its physical integer.
        bits: u8,
    },
    /// An encoding does not apply to the supplied physical type.
    Physical {
        /// Requested encoding identifier.
        encoding: i32,
        /// Actual physical type.
        physical: PhysicalType,
    },
    /// A dictionary page and data page have different physical types.
    DictionaryPhysical {
        /// Physical type of the data page.
        page: PhysicalType,
        /// Physical type of the dictionary page.
        dictionary: PhysicalType,
    },
}

impl purrdf_hash::error::StructuredError for EncodingReason {
    purrdf_hash::error_visit! {
        Packed(cause) => "columnar-value-packed" { "cause" => Cause(cause) },
        DeltaLayout { block, miniblocks } => "columnar-delta-layout" {
            "block" => Unsigned(u128::from(*block)), "miniblocks" => Unsigned(u128::from(*miniblocks))
        },
        ValueCount { declared, expected } => "columnar-encoding-value-count" {
            "declared" => Unsigned(u128::from(*declared)), "expected" => Unsigned(*expected as u128)
        },
        BitWidth { width, maximum } => "columnar-encoding-bit-width" {
            "width" => Unsigned(u128::from(*width)), "maximum" => Unsigned(u128::from(*maximum))
        },
        DictionaryIndex { index, entries } => "columnar-dictionary-index" {
            "index" => Unsigned(u128::from(*index)), "entries" => Unsigned(*entries as u128)
        },
        MissingDictionary => "columnar-missing-dictionary" {},
        DictionaryValue { position } => "columnar-dictionary-value" { "position" => Unsigned(*position as u128) },
        Length { value } => "columnar-delta-length" { "value" => Signed(i128::from(*value)) },
        Prefix { length, previous } => "columnar-delta-prefix" {
            "length" => Signed(i128::from(*length)), "previous" => ByteCount(*previous as u128)
        },
        IntegerRange { value, bits } => "columnar-delta-integer-range" {
            "value" => Signed(i128::from(*value)), "bits" => Unsigned(u128::from(*bits))
        },
        Physical { encoding, physical } => "columnar-encoding-physical" {
            "encoding" => Signed(i128::from(*encoding)), "physical" => Record(physical)
        },
        DictionaryPhysical { page, dictionary } => "columnar-dictionary-physical" {
            "page" => Record(page), "dictionary" => Record(dictionary)
        },
    }
}

impl fmt::Display for EncodingReason {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Packed(cause) => fmt::Display::fmt(cause, out),
            Self::DeltaLayout { block, miniblocks } => {
                write!(out, "invalid delta layout {block}/{miniblocks}")
            }
            Self::ValueCount { declared, expected } => {
                write!(out, "value count {declared}, expected {expected}")
            }
            Self::BitWidth { width, maximum } => write!(out, "bit width {width} exceeds {maximum}"),
            Self::DictionaryIndex { index, entries } => {
                write!(out, "dictionary index {index} outside {entries} entries")
            }
            Self::MissingDictionary => out.write_str("dictionary page is missing"),
            Self::DictionaryValue { position } => {
                write!(out, "page value {position} is absent from dictionary")
            }
            Self::Length { value } => write!(out, "invalid delta byte length {value}"),
            Self::Prefix { length, previous } => write!(
                out,
                "prefix length {length} exceeds previous value length {previous}"
            ),
            Self::IntegerRange { value, bits } => write!(
                out,
                "delta header {value} exceeds signed {bits}-bit representation"
            ),
            Self::Physical { encoding, physical } => {
                write!(out, "encoding {encoding} does not apply to {physical:?}")
            }
            Self::DictionaryPhysical { page, dictionary } => {
                write!(
                    out,
                    "dictionary type {dictionary:?} differs from page type {page:?}"
                )
            }
        }
    }
}
purrdf_hash::error_sources!(EncodingReason, Packed);

fn refusal(reason: EncodingReason) -> ColumnarError {
    ColumnarError::value_body("physical value encoding", ValueBodyReason::Encoding(reason))
}

fn add(a: usize, b: usize) -> Result<usize, ColumnarError> {
    a.checked_add(b).ok_or_else(|| {
        ColumnarError::value_body(
            "physical value encoding",
            ValueBodyReason::OffsetOverflow {
                offset: a,
                length: b,
            },
        )
    })
}

fn reserve<T>(out: &mut Vec<T>, additional: usize) -> Result<(), ColumnarError> {
    reserve_charged(out, additional, size_of::<T>().max(1))
}

fn reserve_charged<T>(
    out: &mut Vec<T>,
    additional: usize,
    charge: usize,
) -> Result<(), ColumnarError> {
    let needed = add(out.len(), additional)?;
    let maximum = MAX_BYTES / charge;
    if needed > maximum {
        return Err(ColumnarError::limit(
            "value encoding workspace",
            needed,
            maximum,
        ));
    }
    if needed <= out.capacity() {
        return Ok(());
    }
    let target = out.capacity().saturating_mul(2).max(needed).min(maximum);
    out.try_reserve_exact(target - out.len())
        .map_err(|_| ColumnarError::Storage(StorageError::AllocationFailed))
}

fn append(out: &mut Vec<u8>, bytes: &[u8]) -> Result<(), ColumnarError> {
    let end = add(out.len(), bytes.len())?;
    if end > MAX_BYTES {
        return Err(ColumnarError::limit(
            "decoded or encoded value bytes",
            end,
            MAX_BYTES,
        ));
    }
    reserve(out, bytes.len())?;
    out.extend_from_slice(bytes);
    Ok(())
}

fn append_varint(out: &mut Vec<u8>, value: u64) -> Result<(), ColumnarError> {
    let mut bytes = purrdf_core::SmallVec::<[u8; 10]>::default();
    write_varint(&mut bytes, value);
    append(out, &bytes)
}

fn take<'a>(bytes: &'a [u8], pos: &mut usize, len: usize) -> Result<&'a [u8], ColumnarError> {
    let end = add(*pos, len)?;
    let value = bytes
        .get(*pos..end)
        .ok_or_else(|| ColumnarError::truncated("physical value encoding", end, bytes.len()))?;
    *pos = end;
    Ok(value)
}

fn varint(bytes: &[u8], pos: &mut usize) -> Result<u64, ColumnarError> {
    let offset = *pos;
    read_varint(bytes, pos).map_err(|cause| {
        ColumnarError::value_body(
            "physical value encoding",
            ValueBodyReason::Varint { offset, cause },
        )
    })
}

fn finish(pos: usize, bytes: &[u8]) -> Result<(), ColumnarError> {
    if pos != bytes.len() {
        return Err(ColumnarError::value_body(
            "physical value encoding",
            ValueBodyReason::TrailingValues {
                offset: pos,
                available: bytes.len(),
            },
        ));
    }
    Ok(())
}

pub(super) struct Plain<'a> {
    bytes: &'a [u8],
    physical: PhysicalType,
    pos: usize,
}

impl<'a> Plain<'a> {
    pub(super) fn new(bytes: &'a [u8], physical: PhysicalType) -> Self {
        // A row group may contain multiple physical pages. Its borrowed input
        // has no single-page ceiling; each value and each emitted page does.
        Self {
            bytes,
            physical,
            pos: 0,
        }
    }

    pub(super) fn next(&mut self) -> Result<Option<&'a [u8]>, ColumnarError> {
        if self.pos == self.bytes.len() {
            return Ok(None);
        }
        let start = self.pos;
        match self.physical {
            PhysicalType::Int64 => {
                take(self.bytes, &mut self.pos, 8)?;
            }
            PhysicalType::ByteArray => {
                let length = purrdf_core::bytes::read_u32_le(self.bytes, self.pos)
                    .ok_or_else(|| {
                        ColumnarError::truncated(
                            "PLAIN BYTE_ARRAY length",
                            self.pos.saturating_add(4),
                            self.bytes.len(),
                        )
                    })?
                    .cast_signed();
                self.pos += 4;
                if length < 0 {
                    return Err(ColumnarError::value_body(
                        "PLAIN BYTE_ARRAY",
                        ValueBodyReason::NegativeLength { length },
                    ));
                }
                take(self.bytes, &mut self.pos, length as usize)?;
            }
        }
        Ok(Some(&self.bytes[start..self.pos]))
    }
}

fn plain_count(bytes: &[u8], physical: PhysicalType) -> Result<usize, ColumnarError> {
    let mut reader = Plain::new(bytes, physical);
    let mut count = 0;
    while reader.next()?.is_some() {
        count += 1;
    }
    Ok(count)
}

/// One owned copy of dictionary PLAIN bytes, plus bounded range/index metadata.
/// Entries use first-occurrence order. Lookup sorts only ordinal metadata.
pub(crate) struct Dictionary {
    physical: PhysicalType,
    plain: Vec<u8>,
    entries: Vec<Range<u32>>,
    order: Vec<u32>,
}

// A canonical allowance for the largest supported host representation. Fixed
// ordinals/ranges and this charge make budget decisions identical on WebAssembly
// and native hosts; smaller controls do not admit a different dictionary.
const DICTIONARY_CONTROL_BYTES: usize = 80;
const _: () = assert!(size_of::<Dictionary>() <= DICTIONARY_CONTROL_BYTES);

fn dictionary_value<'a>(plain: &'a [u8], range: &Range<u32>) -> &'a [u8] {
    &plain[range.start as usize..range.end as usize]
}

impl Dictionary {
    pub(crate) fn build(
        plain: &[u8],
        physical: PhysicalType,
        max_entries: usize,
        max_bytes: usize,
    ) -> Result<Option<Self>, ColumnarError> {
        plain_count(plain, physical)?;
        let max_bytes = max_bytes.min(MAX_BYTES);
        let mut dictionary = Self {
            physical,
            plain: Vec::new(),
            entries: Vec::new(),
            order: Vec::new(),
        };
        let mut reader = Plain::new(plain, physical);
        while let Some(value) = reader.next()? {
            let Err(position) = dictionary.search(value) else {
                continue;
            };
            if dictionary.entries.len() >= max_entries.min(i32::MAX as usize) {
                return Ok(None);
            }
            let entries = add(dictionary.entries.len(), 1)?;
            let bytes = add(dictionary.plain.len(), value.len())?;
            let other = dictionary.plain.capacity()
                + dictionary.order.capacity() * size_of::<u32>()
                + DICTIONARY_CONTROL_BYTES;
            if !grow_bounded(&mut dictionary.entries, entries, other, max_bytes)? {
                return Ok(None);
            }
            let other = dictionary.plain.capacity()
                + dictionary.entries.capacity() * size_of::<Range<u32>>()
                + DICTIONARY_CONTROL_BYTES;
            if !grow_bounded(&mut dictionary.order, entries, other, max_bytes)? {
                return Ok(None);
            }
            let other = dictionary.entries.capacity() * size_of::<Range<u32>>()
                + dictionary.order.capacity() * size_of::<u32>()
                + DICTIONARY_CONTROL_BYTES;
            if !grow_bounded(&mut dictionary.plain, bytes, other, max_bytes)? {
                return Ok(None);
            }
            let start = dictionary.plain.len();
            dictionary.plain.extend_from_slice(value);
            // The entry count and dictionary bytes were bounded by signed
            // 32-bit wire fields before any growth or mutation.
            dictionary
                .order
                .insert(position, dictionary.entries.len() as u32);
            dictionary.entries.push(start as u32..bytes as u32);
        }
        Ok((!dictionary.entries.is_empty()).then_some(dictionary))
    }

    pub(crate) fn from_page(
        body: &[u8],
        count: usize,
        physical: PhysicalType,
    ) -> Result<Self, ColumnarError> {
        let actual = plain_count(body, physical)?;
        if actual != count {
            return Err(refusal(EncodingReason::ValueCount {
                declared: actual as u64,
                expected: count,
            }));
        }
        let owned = count
            .checked_mul(size_of::<Range<u32>>() + size_of::<u32>())
            .and_then(|metadata| metadata.checked_add(body.len()))
            .and_then(|bytes| bytes.checked_add(DICTIONARY_CONTROL_BYTES));
        if owned.is_none_or(|bytes| bytes > MAX_BYTES) {
            return Err(ColumnarError::limit(
                "dictionary page ownership",
                owned.unwrap_or(usize::MAX),
                MAX_BYTES,
            ));
        }
        let mut dictionary = Self {
            physical,
            plain: Vec::new(),
            entries: Vec::new(),
            order: Vec::new(),
        };
        reserve(&mut dictionary.plain, body.len())?;
        reserve(&mut dictionary.entries, count)?;
        reserve(&mut dictionary.order, count)?;
        dictionary.plain.extend_from_slice(body);
        let mut reader = Plain::new(body, physical);
        while let Some(value) = reader.next()? {
            dictionary
                .entries
                .push((reader.pos - value.len()) as u32..reader.pos as u32);
        }
        // The owner bound above is at most i32::MAX bytes and charges twelve
        // metadata bytes per entry, so every ordinal fits u32.
        dictionary.order.extend(0..count as u32);
        dictionary.order.sort_unstable_by(|&a, &b| {
            dictionary_value(&dictionary.plain, &dictionary.entries[a as usize]).cmp(
                dictionary_value(&dictionary.plain, &dictionary.entries[b as usize]),
            )
        });
        Ok(dictionary)
    }

    pub(crate) fn plain(&self) -> &[u8] {
        &self.plain
    }
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    fn search(&self, value: &[u8]) -> Result<usize, usize> {
        self.order
            .binary_search_by(|&index| {
                dictionary_value(&self.plain, &self.entries[index as usize]).cmp(value)
            })
            .map(|position| self.order[position] as usize)
    }

    fn value(&self, index: u64) -> Result<&[u8], ColumnarError> {
        let index = usize::try_from(index).map_err(|_| {
            refusal(EncodingReason::DictionaryIndex {
                index,
                entries: self.len(),
            })
        })?;
        let range = self.entries.get(index).ok_or_else(|| {
            refusal(EncodingReason::DictionaryIndex {
                index: index as u64,
                entries: self.len(),
            })
        })?;
        Ok(dictionary_value(&self.plain, range))
    }

    pub(crate) fn encode_indexes(&self, plain: &[u8]) -> Result<Vec<u8>, ColumnarError> {
        if plain.len() > MAX_BYTES {
            return Err(ColumnarError::limit(
                "dictionary index page",
                plain.len(),
                MAX_BYTES,
            ));
        }
        let count = plain_count(plain, self.physical)?;
        let mut indexes = Vec::new();
        reserve(&mut indexes, count)?;
        let mut reader = Plain::new(plain, self.physical);
        while let Some(value) = reader.next()? {
            let index = self.search(value).map_err(|_| {
                refusal(EncodingReason::DictionaryValue {
                    position: indexes.len(),
                })
            })?;
            indexes.push(index as u64);
        }
        let width = bits_for(self.len().saturating_sub(1) as u64) as u8;
        let mut out = Vec::new();
        append(&mut out, &[width])?;
        let mut pos = 0;
        while pos < indexes.len() {
            let run = indexes[pos..]
                .iter()
                .take_while(|&&value| value == indexes[pos])
                .count();
            if run >= 8 || pos + run == indexes.len() {
                append_varint(&mut out, (run as u64) << 1)?;
                append(
                    &mut out,
                    &indexes[pos].to_le_bytes()[..usize::from(width).div_ceil(8)],
                )?;
                pos += run;
            } else {
                let start = pos;
                pos = (pos + 8).min(indexes.len());
                while pos < indexes.len() {
                    let run = indexes[pos..]
                        .iter()
                        .take_while(|&&value| value == indexes[pos])
                        .count();
                    if run >= 8 || pos + run == indexes.len() {
                        break;
                    }
                    pos = (pos + 8).min(indexes.len());
                }
                let groups = (pos - start).div_ceil(8);
                append_varint(&mut out, ((groups as u64) << 1) | 1)?;
                let mut packed = IntVector::with_width(u32::from(width));
                packed.try_reserve(groups * 8)?;
                for &index in &indexes[start..pos] {
                    packed.push(index);
                }
                for _ in pos - start..groups * 8 {
                    packed.push(0);
                }
                append_packed(&mut out, &packed)?;
            }
        }
        Ok(out)
    }
}

// Count retained capacities and the full replacement allocation before growth;
// the old allocation can remain live while reserve copies it. A missed budget
// returns PLAIN fallback, while actual allocator refusal stays an error.
fn grow_bounded<T>(
    values: &mut Vec<T>,
    needed: usize,
    other: usize,
    budget: usize,
) -> Result<bool, ColumnarError> {
    if values.capacity() >= needed {
        return Ok(true);
    }
    for target in [values.capacity().saturating_mul(2).max(needed), needed] {
        let peak = values
            .capacity()
            .checked_add(target)
            .and_then(|slots| slots.checked_mul(size_of::<T>()))
            .and_then(|bytes| bytes.checked_add(other));
        if peak.is_some_and(|bytes| bytes <= budget) {
            values
                .try_reserve_exact(target - values.len())
                .map_err(|_| ColumnarError::Storage(StorageError::AllocationFailed))?;
            return Ok(true);
        }
    }
    Ok(false)
}

fn append_packed(out: &mut Vec<u8>, packed: &IntVector) -> Result<(), ColumnarError> {
    if packed.width() == 0 {
        return Ok(());
    }
    let bytes = packed_bytes_len(packed.len(), packed.width() as u8)?;
    reserve(out, bytes)?;
    out.extend(packed.packed_bytes().take(bytes));
    Ok(())
}

fn packed_bytes_len(values: usize, width: u8) -> Result<usize, ColumnarError> {
    let bytes = (values as u64)
        .checked_mul(u64::from(width))
        .map(|bits| bits.div_ceil(8));
    let bytes = bytes
        .and_then(|bytes| usize::try_from(bytes).ok())
        .filter(|&bytes| bytes <= MAX_BYTES);
    bytes.ok_or_else(|| ColumnarError::limit("packed values", usize::MAX, MAX_BYTES))
}

fn unpack(
    bytes: &[u8],
    width: u8,
    stored: usize,
    wanted: usize,
    mut value: impl FnMut(u64) -> Result<(), ColumnarError>,
) -> Result<(), ColumnarError> {
    if width == 0 {
        for _ in 0..wanted {
            value(0)?;
        }
        return Ok(());
    }
    // Adapt the shared bit reader's framing; Parquet itself has no such header.
    let padded = add(bytes.len(), 7)? / 8 * 8;
    let mut framed = Vec::new();
    reserve(&mut framed, add(12, padded)?)?;
    framed.extend_from_slice(&(stored as u64).to_le_bytes());
    framed.extend_from_slice(&u32::from(width).to_le_bytes());
    framed.extend_from_slice(bytes);
    framed.resize(12 + padded, 0);
    let packed = IntVectorRef::from_bytes(&framed)
        .map_err(|cause| refusal(EncodingReason::Packed(cause)))?;
    for index in 0..wanted {
        value(packed.get(index))?;
    }
    Ok(())
}

fn encode_integers(values: impl ExactSizeIterator<Item = i64>) -> Result<Vec<u8>, ColumnarError> {
    let count = values.len();
    let mut values = values;
    let mut out = Vec::new();
    append_varint(&mut out, BLOCK as u64)?;
    append_varint(&mut out, MINIBLOCKS as u64)?;
    append_varint(&mut out, count as u64)?;
    let mut previous = values.next().unwrap_or(0);
    append_varint(&mut out, zigzag_encode(previous))?;
    loop {
        let mut deltas = [0_i64; BLOCK];
        let mut length = 0;
        for delta in &mut deltas {
            let Some(value) = values.next() else {
                break;
            };
            *delta = value.wrapping_sub(previous);
            previous = value;
            length += 1;
        }
        if length == 0 {
            break;
        }
        let minimum = deltas[..length].iter().copied().min().unwrap_or(0);
        append_varint(&mut out, zigzag_encode(minimum))?;
        let widths = out.len();
        append(&mut out, &[0; MINIBLOCKS])?;
        for (mini, chunk) in deltas[..length].chunks(MINI).enumerate() {
            let width = bits_for(
                chunk
                    .iter()
                    .map(|delta| delta.wrapping_sub(minimum).cast_unsigned())
                    .max()
                    .unwrap_or(0),
            );
            out[widths + mini] = width as u8;
            if width == 0 {
                continue;
            }
            let mut packed = IntVector::with_width(width);
            packed.try_reserve(MINI)?;
            for delta in chunk {
                packed.push(delta.wrapping_sub(minimum).cast_unsigned());
            }
            for _ in chunk.len()..MINI {
                packed.push(0);
            }
            append_packed(&mut out, &packed)?;
        }
    }
    Ok(out)
}

fn decode_integers(
    bytes: &[u8],
    count: usize,
    bits: u8,
) -> Result<(Vec<i64>, usize), ColumnarError> {
    let mut pos = 0;
    let block = varint(bytes, &mut pos)?;
    let miniblocks = varint(bytes, &mut pos)?;
    if block == 0
        || block % 128 != 0
        || miniblocks == 0
        || block % miniblocks != 0
        || (block / miniblocks) % 32 != 0
    {
        return Err(refusal(EncodingReason::DeltaLayout { block, miniblocks }));
    }
    let declared = varint(bytes, &mut pos)?;
    if declared != count as u64 {
        return Err(refusal(EncodingReason::ValueCount {
            declared,
            expected: count,
        }));
    }
    if count > MAX_BYTES / size_of::<i64>() {
        return Err(ColumnarError::limit(
            "delta integer workspace",
            count,
            MAX_BYTES / size_of::<i64>(),
        ));
    }
    let first = zigzag_decode(varint(bytes, &mut pos)?);
    let checked_header = |value| {
        if bits == 32 && i32::try_from(value).is_err() {
            Err(refusal(EncodingReason::IntegerRange { value, bits }))
        } else {
            Ok(value)
        }
    };
    checked_header(first)?;
    let block = usize::try_from(block).map_err(|_| ColumnarError::LimitExceeded {
        context: "delta block size",
        value: block,
        maximum: usize::MAX as u64,
    })?;
    let miniblocks = usize::try_from(miniblocks).map_err(|_| ColumnarError::LimitExceeded {
        context: "delta miniblocks",
        value: miniblocks,
        maximum: usize::MAX as u64,
    })?;
    let mini = block / miniblocks;
    let mut values = Vec::new();
    reserve(&mut values, count)?;
    if count != 0 {
        values.push(first);
    }
    while values.len() < count {
        let minimum = checked_header(zigzag_decode(varint(bytes, &mut pos)?))?;
        let widths = take(bytes, &mut pos, miniblocks)?;
        for &width in widths {
            if values.len() == count {
                break;
            } // Unused widths have no bodies.
            if width > bits {
                return Err(refusal(EncodingReason::BitWidth {
                    width,
                    maximum: bits,
                }));
            }
            let length = packed_bytes_len(mini, width)?;
            let packed = take(bytes, &mut pos, length)?;
            let wanted = mini.min(count - values.len());
            unpack(packed, width, mini, wanted, |adjustment| {
                let previous = values.last().copied().unwrap_or(first);
                let value = if bits == 32 {
                    i64::from(
                        (previous as i32)
                            .wrapping_add((minimum as i32).wrapping_add(adjustment as u32 as i32)),
                    )
                } else {
                    previous
                        .wrapping_add(minimum)
                        .wrapping_add(adjustment.cast_signed())
                };
                values.push(value);
                Ok(())
            })?;
        }
    }
    Ok((values, pos))
}

fn byte_values(plain: &[u8]) -> Result<Vec<&[u8]>, ColumnarError> {
    let count = plain_count(plain, PhysicalType::ByteArray)?;
    let mut values = Vec::new();
    reserve_charged(&mut values, count, VALUE_REFERENCE_BYTES)?;
    let mut reader = Plain::new(plain, PhysicalType::ByteArray);
    while let Some(value) = reader.next()? {
        values.push(&value[4..]);
    }
    Ok(values)
}

// One canonical charge for a borrowed byte slice on every supported host.
// A smaller pointer representation must not admit a different value codec.
const VALUE_REFERENCE_BYTES: usize = 16;
const _: () = assert!(size_of::<&[u8]>() <= VALUE_REFERENCE_BYTES);

fn encode_lengths(values: &[&[u8]]) -> Result<Vec<u8>, ColumnarError> {
    let mut out = encode_integers(values.iter().map(|value| value.len() as i64))?;
    for value in values {
        append(&mut out, value)?;
    }
    Ok(out)
}

pub(crate) fn encode_delta(
    encoding: i32,
    plain: &[u8],
    physical: PhysicalType,
) -> Result<Vec<u8>, ColumnarError> {
    if plain.len() > MAX_BYTES {
        return Err(ColumnarError::limit(
            "delta value page",
            plain.len(),
            MAX_BYTES,
        ));
    }
    match (encoding, physical) {
        (DELTA_BINARY_PACKED, PhysicalType::Int64) => {
            plain_count(plain, physical)?;
            encode_integers(
                plain
                    .as_chunks::<8>()
                    .0
                    .iter()
                    .map(|value| i64::from_le_bytes(*value)),
            )
        }
        (DELTA_LENGTH_BYTE_ARRAY, PhysicalType::ByteArray) => encode_lengths(&byte_values(plain)?),
        (DELTA_BYTE_ARRAY, PhysicalType::ByteArray) => {
            let values = byte_values(plain)?;
            let mut prefixes = Vec::new();
            let mut suffixes = Vec::new();
            reserve(&mut prefixes, values.len())?;
            reserve_charged(&mut suffixes, values.len(), VALUE_REFERENCE_BYTES)?;
            let mut previous: &[u8] = &[];
            for value in values {
                let prefix = purrdf_deflate::common_prefix_len(previous, value);
                prefixes.push(prefix as i64);
                suffixes.push(&value[prefix..]);
                previous = value;
            }
            let mut out = encode_integers(prefixes.into_iter())?;
            append(&mut out, &encode_lengths(&suffixes)?)?;
            Ok(out)
        }
        _ => Err(refusal(EncodingReason::Physical { encoding, physical })),
    }
}

fn length(value: i64) -> Result<usize, ColumnarError> {
    if !(0..=i64::from(i32::MAX)).contains(&value) {
        return Err(refusal(EncodingReason::Length { value }));
    }
    Ok(value as usize)
}

fn decode_bytes(
    bytes: &[u8],
    count: usize,
    prefixes: Option<&[i64]>,
) -> Result<Vec<u8>, ColumnarError> {
    let (lengths, mut pos) = decode_integers(bytes, count, 32)?;
    let mut out = Vec::new();
    let mut previous = 0..0;
    for (index, suffix) in lengths.into_iter().enumerate() {
        let prefix = prefixes.map_or(0, |values| values[index]);
        if prefix < 0 || prefix as u64 > previous.len() as u64 {
            return Err(refusal(EncodingReason::Prefix {
                length: prefix,
                previous: previous.len(),
            }));
        }
        let suffix = take(bytes, &mut pos, length(suffix)?)?;
        let prefix = prefix as usize;
        let total = add(prefix, suffix.len())?;
        if total > MAX_BYTES {
            return Err(ColumnarError::limit("BYTE_ARRAY length", total, MAX_BYTES));
        }
        let end = add(add(out.len(), 4)?, total)?;
        if end > MAX_BYTES {
            return Err(ColumnarError::limit(
                "decoded PLAIN byte arrays",
                end,
                MAX_BYTES,
            ));
        }
        let additional = end - out.len();
        reserve(&mut out, additional)?;
        out.extend_from_slice(&(total as i32).to_le_bytes());
        let start = out.len();
        out.extend_from_within(previous.start..previous.start + prefix);
        out.extend_from_slice(suffix);
        previous = start..out.len();
    }
    finish(pos, bytes)?;
    Ok(out)
}

pub(crate) fn decode_values(
    encoding: i32,
    encoded: &[u8],
    present: usize,
    physical: PhysicalType,
    dictionary: Option<&Dictionary>,
) -> Result<Vec<u8>, ColumnarError> {
    if encoded.len() > MAX_BYTES {
        return Err(ColumnarError::limit(
            "encoded value body",
            encoded.len(),
            MAX_BYTES,
        ));
    }
    let minimum = match physical {
        PhysicalType::Int64 => 8,
        PhysicalType::ByteArray => 4,
    };
    if present > MAX_BYTES / minimum {
        return Err(ColumnarError::limit(
            "dense PLAIN values",
            present,
            MAX_BYTES / minimum,
        ));
    }
    match (encoding, physical) {
        (PLAIN, _) => {
            let count = plain_count(encoded, physical)?;
            if count != present {
                return Err(refusal(EncodingReason::ValueCount {
                    declared: count as u64,
                    expected: present,
                }));
            }
            let mut out = Vec::new();
            append(&mut out, encoded)?;
            Ok(out)
        }
        (DELTA_BINARY_PACKED, PhysicalType::Int64) => {
            let (values, pos) = decode_integers(encoded, present, 64)?;
            finish(pos, encoded)?;
            let mut out = Vec::new();
            reserve(&mut out, values.len() * 8)?;
            for value in values {
                out.extend_from_slice(&value.to_le_bytes());
            }
            Ok(out)
        }
        (DELTA_LENGTH_BYTE_ARRAY, PhysicalType::ByteArray) => decode_bytes(encoded, present, None),
        (DELTA_BYTE_ARRAY, PhysicalType::ByteArray) => {
            let (prefixes, pos) = decode_integers(encoded, present, 32)?;
            decode_bytes(&encoded[pos..], present, Some(&prefixes))
        }
        (RLE_DICTIONARY, _) => {
            let dictionary =
                dictionary.ok_or_else(|| refusal(EncodingReason::MissingDictionary))?;
            if dictionary.physical != physical {
                return Err(refusal(EncodingReason::DictionaryPhysical {
                    page: physical,
                    dictionary: dictionary.physical,
                }));
            }
            let mut pos = 0;
            let width = take(encoded, &mut pos, 1)?[0];
            if width > 32 {
                return Err(refusal(EncodingReason::BitWidth { width, maximum: 32 }));
            }
            let mut out = Vec::new();
            let mut remaining = present;
            while remaining != 0 {
                let header = varint(encoded, &mut pos)?;
                let units = usize::try_from(header >> 1).map_err(|cause| {
                    ColumnarError::value_body(
                        "dictionary run",
                        ValueBodyReason::RunLength {
                            length: header >> 1,
                            cause,
                        },
                    )
                })?;
                if units == 0 {
                    return Err(ColumnarError::value_body(
                        "dictionary run",
                        ValueBodyReason::InvalidRun {
                            length: 0,
                            remaining,
                        },
                    ));
                }
                if header & 1 == 0 {
                    if units > remaining {
                        return Err(ColumnarError::value_body(
                            "dictionary run",
                            ValueBodyReason::InvalidRun {
                                length: units,
                                remaining,
                            },
                        ));
                    }
                    let raw = take(encoded, &mut pos, usize::from(width).div_ceil(8))?;
                    let mut index = [0; 8];
                    index[..raw.len()].copy_from_slice(raw);
                    let index = u64::from_le_bytes(index);
                    if bits_for(index) > u32::from(width) {
                        return Err(refusal(EncodingReason::BitWidth {
                            width: bits_for(index) as u8,
                            maximum: width,
                        }));
                    }
                    let value = dictionary.value(index)?;
                    for _ in 0..units {
                        append(&mut out, value)?;
                    }
                    remaining -= units;
                } else {
                    let stored = units.checked_mul(8).ok_or_else(|| {
                        ColumnarError::limit("dictionary packed run", usize::MAX, MAX_BYTES)
                    })?;
                    if stored > remaining && stored - remaining > 7 {
                        return Err(ColumnarError::value_body(
                            "dictionary packed run",
                            ValueBodyReason::InvalidRun {
                                length: stored,
                                remaining,
                            },
                        ));
                    }
                    let length = units.checked_mul(usize::from(width)).ok_or_else(|| {
                        ColumnarError::limit("dictionary packed bytes", usize::MAX, MAX_BYTES)
                    })?;
                    let raw = take(encoded, &mut pos, length)?;
                    let wanted = stored.min(remaining);
                    unpack(raw, width, stored, wanted, |index| {
                        append(&mut out, dictionary.value(index)?)
                    })?;
                    remaining -= wanted;
                }
            }
            finish(pos, encoded)?;
            Ok(out)
        }
        _ => Err(refusal(EncodingReason::Physical { encoding, physical })),
    }
}
