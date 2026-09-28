// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! AES table hashing on builds targeting x86-64 or little-endian AArch64 AES.
//!
//! An integer enters a 128-bit accumulator through one AES round; finalization
//! takes two more rounds before truncating to 64 bits. Short byte strings pack
//! into one block, and 17–32 bytes use sequential first/last block absorption.
//! Longer strings retain independent compression lanes before absorption.

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
/// For 17–32 bytes, the first and last sixteen bytes feed two lanes. Each
/// takes two AES rounds before their outputs are XOR-ed. For longer slices,
/// four 128-bit lanes each take one 16-byte block of every 64-byte step as
/// `lane = R(lane ⊕ block, key)`, where `R` is one AES round. Steps run from
/// the start while more than 64 bytes remain, and the final step is the last
/// 64 bytes. For 33–64 bytes, the first 32 and last 32 feed all four.
///
/// For slices over 32 bytes, each lane then takes one more round on its own,
/// `w = R(lane, f)`, before any lanes meet. The merge is
/// `R(w0 ⊕ w1, m0) ⊕ R(w2 ⊕ w3, m1)`.
///
/// The extra round removed the structured collisions found by the sparse-key
/// tests. The first design XOR-ed a one-round lane into a two-round lane;
/// sparse differences cancelled across those lanes, producing full 128-bit
/// collisions. Each individual round is a permutation, but merging lanes and
/// truncating the result are not. The quality tests provide empirical coverage,
/// not a cryptographic collision-resistance claim.
// Keep the long-slice body out of the short string Hash implementation.
// Inlining it makes the standard library string wrapper exceed its inline budget.
#[inline(never)]
pub(crate) fn compress_long(bytes: &[u8]) -> (u64, u64) {
    let len = bytes.len();
    debug_assert!(len > 16);
    if len <= 32 {
        // The common IRI case needs only the first and last 16 bytes. Two
        // rounds per lane diffuse every input byte before the lanes meet;
        // the caller's folded multiply mixes the two output words again.
        let (low, high) = AES_LANE_INIT[0];
        let first = Block::from_words(low, high)
            .xor(block(bytes, 0))
            .round(Block::from_words(AES_LANE_KEY[0].0, AES_LANE_KEY[0].1))
            .round(Block::from_words(AES_FINAL_KEY[0].0, AES_FINAL_KEY[0].1));
        let (low, high) = AES_LANE_INIT[1];
        let last = Block::from_words(low, high)
            .xor(block(bytes, len - 16))
            .round(Block::from_words(AES_LANE_KEY[1].0, AES_LANE_KEY[1].1))
            .round(Block::from_words(AES_FINAL_KEY[1].0, AES_FINAL_KEY[1].1));
        return first.xor(last).words();
    }
    let keys = AES_LANE_KEY.map(|(low, high)| Block::from_words(low, high));
    let mut lanes = AES_LANE_INIT.map(|(low, high)| Block::from_words(low, high));
    if len <= 64 {
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

/// Full-width AES state, truncated only after two final diffusion rounds.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Engine {
    state: Block,
}

impl Engine {
    pub(crate) const NAME: &str = "aes";

    #[inline]
    pub(crate) fn new() -> Self {
        Self {
            state: Block::from_words(AES_LANE_INIT[0].0, AES_LANE_INIT[0].1),
        }
    }

    #[inline]
    fn absorb_integer(&mut self, low: u64, high: u64) {
        self.state = self
            .state
            .xor(Block::from_words(low, high))
            .round(Block::from_words(AES_LANE_KEY[0].0, AES_LANE_KEY[0].1));
    }

    #[inline]
    pub(crate) fn word(&mut self, word: u64) {
        self.absorb_integer(word, 0);
    }

    #[inline]
    pub(crate) fn wide(&mut self, word: u128) {
        self.absorb_integer(word as u64, (word >> 64) as u64);
    }

    // Assembly inspection found this outlined at monomorphic string call
    // sites, forcing the otherwise register-only state through stack memory.
    #[allow(clippy::inline_always)] // Preserve register state across the public Hasher wrapper.
    #[inline(always)]
    pub(crate) fn bytes(&mut self, bytes: &[u8]) {
        if (17..=32).contains(&bytes.len()) {
            self.state = self
                .state
                .xor(Block::from_words(
                    (bytes.len() as u64).wrapping_mul(super::keys::LEN_M),
                    0,
                ))
                .xor(block(bytes, 0))
                .round(Block::from_words(AES_LANE_KEY[0].0, AES_LANE_KEY[0].1))
                .xor(block(bytes, bytes.len() - 16))
                .round(Block::from_words(AES_LANE_KEY[1].0, AES_LANE_KEY[1].1));
            return;
        }
        let (low, high) = if bytes.len() <= 16 {
            super::portable::pack_short(bytes)
        } else {
            compress_long(bytes)
        };
        let domain = (bytes.len() as u64).wrapping_mul(super::keys::LEN_M);
        self.absorb_integer(low ^ domain, high);
    }

    #[inline]
    pub(crate) fn finish(self) -> u64 {
        // Three AES rounds after the last integer enters: one on absorption,
        // then these two. Truncating after only two rounds left structured
        // collisions among high-bit integer keys, even with good avalanche.
        self.state
            .round(Block::from_words(AES_FINAL_KEY[0].0, AES_FINAL_KEY[0].1))
            .round(Block::from_words(AES_FINAL_KEY[1].0, AES_FINAL_KEY[1].1))
            .words()
            .0
    }

    #[inline]
    pub(crate) fn terminal(tag: u8, bytes: &[u8]) -> u64 {
        if (17..=32).contains(&bytes.len()) {
            let mut hasher = Self::new();
            hasher.state = hasher.state.xor(Block::from_words(
                u64::from(tag).wrapping_mul(super::keys::K_A),
                0,
            ));
            hasher.bytes(bytes);
            hasher.finish()
        } else {
            super::Engine::<super::Aes>::terminal(tag, bytes)
        }
    }
}
