// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Physical bounds and the standard flat-column page indexes.

use core::cmp::Ordering;
use core::hash::Hasher;

use super::encoding::Plain;
use crate::compact::CompactWriter;
use crate::{ColumnarError, PhysicalType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Location {
    pub(super) offset: i64,
    pub(super) bytes: i32,
}

impl purrdf_hash::error::StructuredError for Location {
    purrdf_hash::error_visit! {
        @record self { offset, bytes } => "columnar-auxiliary-location" {
            "offset" => Signed(i128::from(self.offset)),
            "bytes" => Signed(i128::from(self.bytes)),
        }
    }
}

/// `BYTE_ARRAY` bounds exclude the PLAIN length prefix. `INT64` bounds keep their
/// eight little-endian bytes, but compare as signed values. Exact full bounds
/// also preserve valid UTF-8 for columns carrying that logical annotation.
#[derive(Debug, Default)]
pub(super) struct Statistics {
    pub(super) min: Option<Vec<u8>>,
    pub(super) max: Option<Vec<u8>>,
    pub(super) nulls: i64,
    min_exact: bool,
    max_exact: bool,
}

fn compare(left: &[u8], right: &[u8], physical: PhysicalType) -> Ordering {
    match physical {
        PhysicalType::ByteArray => left.cmp(right),
        PhysicalType::Int64 => {
            let integer = |bytes: &[u8]| {
                i64::from_le_bytes(*bytes.first_chunk().expect("validated integer value"))
            };
            integer(left).cmp(&integer(right))
        }
    }
}

impl Statistics {
    pub(super) fn collect(
        plain: &[u8],
        physical: PhysicalType,
        nulls: usize,
        utf8: bool,
    ) -> Result<Self, ColumnarError> {
        let mut min = None;
        let mut max = None;
        let mut values = Plain::new(plain, physical);
        while let Some(value) = values.next()? {
            let value = match physical {
                PhysicalType::Int64 => value,
                PhysicalType::ByteArray => &value[4..],
            };
            if min.is_none_or(|min| compare(value, min, physical).is_lt()) {
                min = Some(value);
            }
            if max.is_none_or(|max| compare(value, max, physical).is_gt()) {
                max = Some(value);
            }
        }
        let bounded = physical == PhysicalType::ByteArray && !utf8;
        let mut stats = Self {
            nulls: nulls as i64,
            min_exact: true,
            max_exact: true,
            ..Self::default()
        };
        stats.min = min.map(|value| {
            if bounded && value.len() > 64 {
                stats.min_exact = false;
                value[..64].to_vec()
            } else {
                value.to_vec()
            }
        });
        stats.max = max.map(|value| {
            if bounded && value.len() > 64 {
                // A prefix alone is a lower bound. Its rightmost byte with a
                // successor gives a strict upper bound after dropping its tail.
                if let Some(last) = value[..64].iter().rposition(|&byte| byte != u8::MAX) {
                    let mut upper = value[..=last].to_vec();
                    upper[last] += 1;
                    stats.max_exact = false;
                    return upper;
                }
            }
            // No finite bounded successor exists for an all-maximum prefix.
            // Preserve the exact maximum instead of publishing a false bound.
            value.to_vec()
        });
        Ok(stats)
    }

    pub(super) fn write(&self, writer: &mut CompactWriter) {
        writer.i64_field(3, self.nulls);
        if let (Some(min), Some(max)) = (&self.min, &self.max) {
            // Deprecated min/max fields use the old signed BYTE_ARRAY ordering
            // and are deliberately absent. These fields use TypeDefinedOrder.
            writer.binary_field(5, max);
            writer.binary_field(6, min);
            writer.bool_field(7, self.max_exact);
            writer.bool_field(8, self.min_exact);
        }
    }
}

#[derive(Debug)]
pub(super) struct Page {
    pub(super) offset: i64,
    pub(super) bytes: i32,
    pub(super) first_row: i64,
    pub(super) stats: Statistics,
}

pub(super) fn column_index(pages: &[Page]) -> Vec<u8> {
    CompactWriter::root(|writer| {
        writer.list_bool_field(1, pages.iter().map(|page| page.stats.min.is_none()));
        writer.list_binary_field(
            2,
            pages
                .iter()
                .map(|page| page.stats.min.as_deref().unwrap_or_default()),
        );
        writer.list_binary_field(
            3,
            pages
                .iter()
                .map(|page| page.stats.max.as_deref().unwrap_or_default()),
        );
        // UNORDERED is valid for all populations, including sorted/all-null.
        writer.i32_field(4, 0);
        writer.list_i64_field(5, pages.iter().map(|page| page.stats.nulls));
    })
}

pub(super) fn offset_index(pages: &[Page]) -> Vec<u8> {
    CompactWriter::root(|writer| {
        writer.list_struct_field(1, pages, |writer, page| {
            writer.i64_field(1, page.offset);
            writer.i32_field(2, page.bytes);
            writer.i64_field(3, page.first_row);
        });
    })
}

/// Parquet's uncompressed split-block algorithm with the specified seed-zero
/// hash. The bitset cap is validated before source preparation/output.
pub(super) fn bloom(
    plain: &[u8],
    physical: PhysicalType,
    present: usize,
    options: crate::BloomOptions,
) -> Result<Vec<u8>, ColumnarError> {
    let blocks = ((present as u64)
        .saturating_mul(u64::from(options.bits_per_value))
        .div_ceil(256))
    .max(1)
    .min(u64::from(options.max_bytes / 32)) as usize;
    let mut bits = vec![0u8; blocks * 32];
    let mut values = Plain::new(plain, physical);
    while let Some(value) = values.next()? {
        let value = match physical {
            PhysicalType::Int64 => value,
            PhysicalType::ByteArray => &value[4..],
        };
        let mut hash = twox_hash::XxHash64::with_seed(0);
        hash.write(value);
        let hash = hash.finish();
        let block = (((hash >> 32) * blocks as u64) >> 32) as usize;
        const SALT: [u32; 8] = [
            0x47b6_137b,
            0x4497_4d91,
            0x8824_ad5b,
            0xa2b7_289d,
            0x7054_95c7,
            0x2df1_424b,
            0x9efc_4947,
            0x5c6b_fb31,
        ];
        for (lane, salt) in SALT.into_iter().enumerate() {
            let bit = 1u32 << ((hash as u32).wrapping_mul(salt) >> 27);
            let start = block * 32 + lane * 4;
            let word = u32::from_le_bytes(
                *bits[start..start + 4]
                    .first_chunk()
                    .expect("four-byte Bloom lane"),
            );
            bits[start..start + 4].copy_from_slice(&(word | bit).to_le_bytes());
        }
    }
    let mut out = CompactWriter::root(|writer| {
        writer.i32_field(1, bits.len() as i32);
        writer.struct_field(2, |writer| writer.struct_field(1, |_| {}));
        writer.struct_field(3, |writer| writer.struct_field(1, |_| {}));
        writer.struct_field(4, |writer| writer.struct_field(1, |_| {}));
    });
    out.extend_from_slice(&bits);
    Ok(out)
}
