// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Byte packing and portable compression of long slices.

use super::fold::fold;
use super::keys::{LANE_INIT, LANE_KEY};

/// The little-endian `u64` at `at`.
#[inline]
pub(crate) fn read64(bytes: &[u8], at: usize) -> u64 {
    let (word, _) = bytes[at..]
        .split_first_chunk::<8>()
        .expect("eight bytes at the offset");
    u64::from_le_bytes(*word)
}

/// The little-endian `u32` at `at`.
#[inline]
fn read32(bytes: &[u8], at: usize) -> u64 {
    let (word, _) = bytes[at..]
        .split_first_chunk::<4>()
        .expect("four bytes at the offset");
    u64::from(u32::from_le_bytes(*word))
}

/// Packs 0–16 bytes into two words, injectively for each fixed length.
///
/// 1–3 bytes take the first, middle and last byte; 4–8 the first and last
/// four (overlapping when fewer than eight); 9–16 the first and last eight
/// (overlapping when fewer than sixteen). For a fixed length every byte
/// lands in at least one position, so equal words mean equal bytes.
#[inline]
pub(crate) fn pack_short(bytes: &[u8]) -> (u64, u64) {
    let len = bytes.len();
    debug_assert!(len <= 16);
    if len >= 9 {
        (read64(bytes, 0), read64(bytes, len - 8))
    } else if len >= 4 {
        (read32(bytes, 0) | (read32(bytes, len - 4) << 32), 0)
    } else if len > 0 {
        let first = u64::from(bytes[0]);
        let middle = u64::from(bytes[len / 2]);
        let last = u64::from(bytes[len - 1]);
        (first | (middle << 8) | (last << 16), 0)
    } else {
        (0, 0)
    }
}

#[inline]
fn step(lanes: &mut [u64; 4], words: [u64; 4]) {
    for ((lane, word), key) in lanes.iter_mut().zip(words).zip(LANE_KEY) {
        *lane = fold(*lane ^ word, key);
    }
}

#[inline]
fn block_words(block: &[u8; 32]) -> [u64; 4] {
    [
        read64(block, 0),
        read64(block, 8),
        read64(block, 16),
        read64(block, 24),
    ]
}

/// Compresses a slice of more than 16 bytes to two words.
///
/// For 17–32 bytes, two folds mix the first sixteen bytes and two rotated
/// words bring in the last sixteen. Longer slices use four lanes, each
/// folding one word of every 32-byte block by its own key. Blocks run from
/// the start while more than 32 bytes remain; the final block is the last
/// 32 bytes, overlapping its predecessor when the length is not a multiple
/// of 32. The caller mixes in the length, which overlap leaves ambiguous.
#[inline]
pub(crate) fn compress_long(bytes: &[u8]) -> (u64, u64) {
    let len = bytes.len();
    debug_assert!(len > 16);
    if len <= 32 {
        // The common IRI length class has four possibly overlapping words.
        // Two independent folded products mix the first sixteen bytes; the
        // last sixteen enter separate words before the caller's final pair
        // of folds. Distinct rotations keep aligned overlap from cancelling.
        let first = fold(LANE_INIT[0] ^ read64(bytes, 0), LANE_KEY[0])
            ^ (LANE_INIT[2] ^ read64(bytes, len - 16)).rotate_left(17);
        let second = fold(LANE_INIT[1] ^ read64(bytes, 8), LANE_KEY[1])
            ^ (LANE_INIT[3] ^ read64(bytes, len - 8)).rotate_left(43);
        return (first, second);
    }
    let mut lanes = LANE_INIT;
    let (blocks, _) = bytes[..len - 1].as_chunks::<32>();
    for block in blocks {
        step(&mut lanes, block_words(block));
    }
    let last = bytes.last_chunk::<32>().expect("more than 32 bytes");
    step(&mut lanes, block_words(last));
    (lanes[0] ^ lanes[2], lanes[1] ^ lanes[3])
}
