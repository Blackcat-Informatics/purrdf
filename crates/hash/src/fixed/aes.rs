// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The AES-round compression of long slices, compiled only into builds whose
//! target enables `aes` on x86-64 or little-endian AArch64.
//!
//! Only slices longer than 16 bytes reach it: integers, `finish` and 0–16-byte
//! slices are the portable folds on every build, because the bench
//! (`benches/hasher.rs`) measured AES rounds no faster there, and slower
//! than folds as a latency chain.

use super::keys::{AES_FINAL_KEY, AES_LANE_INIT, AES_LANE_KEY, AES_MERGE_KEY};
use crate::arch::Block;

#[inline]
fn block(bytes: &[u8], at: usize) -> Block {
    let (chunk, _) = bytes[at..]
        .split_first_chunk::<16>()
        .expect("sixteen bytes at the offset");
    Block::load(chunk)
}

#[inline]
fn step(lanes: &mut [Block; 4], keys: &[Block; 4], data: [Block; 4]) {
    for ((lane, key), data) in lanes.iter_mut().zip(keys).zip(data) {
        *lane = lane.xor(data).round(*key);
    }
}

#[inline]
fn quad(bytes: &[u8], at: usize) -> [Block; 4] {
    [
        block(bytes, at),
        block(bytes, at + 16),
        block(bytes, at + 32),
        block(bytes, at + 48),
    ]
}

/// Compresses a slice of more than 16 bytes to two words.
///
/// Four 128-bit lanes each take one 16-byte block of every 64-byte step as
/// `lane = R(lane ⊕ block, key)`, where `R` is one AES round. Steps run from
/// the start while more than 64 bytes remain, and the final step is the last
/// 64 bytes. For 17–32 bytes, the first and last sixteen bytes feed two
/// lanes. For 33–64 bytes, the first 32 and last 32 feed all four.
///
/// Each lane then takes one more round on its own, `w = R(lane, f)`, before
/// any lanes meet. The merge is `R(w0 ⊕ w1, m0) ⊕ R(w2 ⊕ w3, m1)`.
///
/// The extra round is what makes the merge sound. The first design XOR-ed a
/// lane that had one round into a lane that had two. Sparse inputs then
/// produced equal differences on both sides, and the sparse-key test found
/// full 128-bit collisions. After two rounds, a difference confined to one
/// lane can reach far too many values to be matched by a difference in
/// another lane. A difference in a single lane cannot cancel at all, because
/// each round is a permutation.
#[inline]
pub(crate) fn compress_long(bytes: &[u8]) -> (u64, u64) {
    let len = bytes.len();
    debug_assert!(len > 16);
    let keys = AES_LANE_KEY.map(|(low, high)| Block::from_words(low, high));
    let mut lanes = AES_LANE_INIT.map(|(low, high)| Block::from_words(low, high));
    if len <= 32 {
        lanes[0] = lanes[0].xor(block(bytes, 0)).round(keys[0]);
        lanes[1] = lanes[1].xor(block(bytes, len - 16)).round(keys[1]);
    } else if len <= 64 {
        step(
            &mut lanes,
            &keys,
            [
                block(bytes, 0),
                block(bytes, 16),
                block(bytes, len - 32),
                block(bytes, len - 16),
            ],
        );
    } else {
        let steps = (len - 1) / 64;
        for index in 0..steps {
            step(&mut lanes, &keys, quad(bytes, 64 * index));
        }
        step(&mut lanes, &keys, quad(bytes, len - 64));
    }
    let finals = AES_FINAL_KEY.map(|(low, high)| Block::from_words(low, high));
    let [w0, w1, w2, w3] = [0, 1, 2, 3].map(|lane| lanes[lane].round(finals[lane]));
    let [m0, m1] = AES_MERGE_KEY.map(|(low, high)| Block::from_words(low, high));
    w0.xor(w1).round(m0).xor(w2.xor(w3).round(m1)).words()
}
