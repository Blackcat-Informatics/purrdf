#![allow(
    clippy::cast_lossless,
    clippy::checked_conversions,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
use alloc::vec::Vec;

/// Returns the minimum number of bytes needed to represent this value, as
/// either 1, 2, 4, or 8 bytes. A value of 0 will still return one byte.
///
/// Used for variable length fields like `Dictionary_ID`.
pub(crate) fn find_min_size(val: u64) -> usize {
    if val == 0 {
        return 1;
    }
    if val >> 8 == 0 {
        return 1;
    }
    if val >> 16 == 0 {
        return 2;
    }
    if val >> 32 == 0 {
        return 4;
    }
    8
}

/// Appends the value represented using the smallest number of bytes needed
/// (1, 2, 4, or 8; zero takes 1 byte) directly onto `output`.
///
/// Operates in **little-endian**.
pub(crate) fn write_minified_val(val: u64, output: &mut Vec<u8>) {
    let new_size = find_min_size(val);
    output.extend_from_slice(&val.to_le_bytes()[0..new_size]);
}

/// Returns the minimum FCS field size for the given content size.
///
/// FCS has different sizing rules than `Dictionary_ID` due to the +256 offset
/// for 2-byte fields (spec range 256–65791). When `single_segment` is true,
/// values 0–255 can use a 1-byte field (FCS flag = 0 combined with the
/// single-segment flag). Otherwise, only the 1-byte encoding is unavailable:
/// values 256–65791 still use a 2-byte field, while smaller values fall back
/// to the 4-byte encoding.
///
/// <https://github.com/facebook/zstd/blob/dev/doc/zstd_compression_format.md#frame_content_size>
pub(crate) fn find_fcs_field_size(val: u64, single_segment: bool) -> usize {
    if single_segment && val <= 255 {
        return 1;
    }
    if (256..=65791).contains(&val) {
        return 2;
    }
    if val <= u32::MAX as u64 {
        return 4;
    }
    8
}
