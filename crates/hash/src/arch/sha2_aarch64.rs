// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! SHA-256 and SHA-512 on the Arm crypto extensions. Their state and
//! schedule groups follow FIPS 180-4 §6.2.2 and §6.4.2 and the Arm
//! SHA256H/H2, SHA256SU0/SU1, SHA512H/H2 and SHA512SU0/SU1 definitions.

use core::arch::aarch64::{
    vaddq_u32, vld1q_u8, vld1q_u32, vreinterpretq_u32_u8, vrev32q_u8, vsha256h2q_u32,
    vsha256hq_u32, vsha256su0q_u32, vsha256su1q_u32, vst1q_u32,
};
use std::arch::is_aarch64_feature_detected;

use crate::sha2::{K32, K64, Sha256Blocks, Sha512Blocks};
use core::arch::aarch64::{
    vaddq_u64, vextq_u64, vld1q_u64, vreinterpretq_u64_u8, vrev64q_u8, vsha512h2q_u64,
    vsha512hq_u64, vsha512su0q_u64, vsha512su1q_u64, vst1q_u64,
};

pub(crate) fn sha256_blocks() -> Option<Sha256Blocks> {
    (is_aarch64_feature_detected!("sha2") && is_aarch64_feature_detected!("neon"))
        .then_some(sha256_entry)
}

pub(crate) fn sha512_blocks() -> Option<Sha512Blocks> {
    // Rust's sha3 feature represents both FEAT_SHA512 and FEAT_SHA3.
    (is_aarch64_feature_detected!("sha3") && is_aarch64_feature_detected!("neon"))
        .then_some(sha512_entry)
}

fn sha512_entry(state: &mut [u64; 8], whole: &[u8], tail: &[u8]) {
    // SAFETY: this entry only escapes after detecting all the required features.
    unsafe { sha512_kernel(state, whole, tail) }
}

#[target_feature(enable = "sha3,neon")]
#[inline(never)]
unsafe fn sha512_kernel(state: &mut [u64; 8], whole: &[u8], tail: &[u8]) {
    let mut registers = core::array::from_fn::<_, 4, _>(|i| {
        // SAFETY: each pair comprises two of the eight consecutive state words.
        unsafe { vld1q_u64(state.as_ptr().add(i * 2)) }
    });
    for input in [whole, tail] {
        for block in input.as_chunks::<128>().0 {
            let mut schedule = core::array::from_fn::<_, 8, _>(|i| {
                // SAFETY: each pair loads sixteen of the block's initialized bytes.
                let bytes = unsafe { vld1q_u8(block.as_ptr().add(i * 16)) };
                vreinterpretq_u64_u8(vrev64q_u8(bytes))
            });
            let [mut ab, mut cd, mut ef, mut gh] = registers;
            for (group, constants) in K64.as_chunks::<2>().0.iter().enumerate() {
                let slot = group & 7;
                if group >= 8 {
                    let seven_back =
                        vextq_u64::<1>(schedule[(slot + 4) & 7], schedule[(slot + 5) & 7]);
                    let low = vsha512su0q_u64(schedule[slot], schedule[(slot + 1) & 7]);
                    schedule[slot] = vsha512su1q_u64(low, schedule[(slot + 7) & 7], seven_back);
                }
                // SAFETY: constants is exactly one pair of words.
                let words = vaddq_u64(schedule[slot], unsafe { vld1q_u64(constants.as_ptr()) });
                // The first round consumes the upper lanes of the instruction
                // operands. Reversing just W + K pairs its first word with H.
                let wh = vaddq_u64(gh, vextq_u64::<1>(words, words));
                let t1 = vsha512hq_u64(wh, vextq_u64::<1>(ef, gh), vextq_u64::<1>(cd, ef));
                (ab, cd, ef, gh) = (vsha512h2q_u64(t1, cd, ab), ab, vaddq_u64(cd, t1), ef);
            }
            for (state, update) in registers.iter_mut().zip([ab, cd, ef, gh]) {
                *state = vaddq_u64(*state, update);
            }
        }
    }
    for (i, register) in registers.into_iter().enumerate() {
        // SAFETY: both stored words fall within the eight-word destination.
        unsafe { vst1q_u64(state.as_mut_ptr().add(i * 2), register) };
    }
}

fn sha256_entry(state: &mut [u32; 8], whole: &[u8], tail: &[u8]) {
    // SAFETY: this entry only escapes after detecting all the required features.
    unsafe { sha256_kernel(state, whole, tail) }
}

#[target_feature(enable = "sha2,neon")]
#[inline(never)]
unsafe fn sha256_kernel(state: &mut [u32; 8], whole: &[u8], tail: &[u8]) {
    // SAFETY: both groups contain four of the eight consecutive state words.
    let (mut abcd, mut efgh) = unsafe {
        let p = state.as_ptr();
        (vld1q_u32(p), vld1q_u32(p.add(4)))
    };
    for input in [whole, tail] {
        for block in input.as_chunks::<64>().0 {
            let before = (abcd, efgh);
            let [a, b, c, d] = block.as_chunks::<16>().0 else {
                unreachable!("a 64-byte block has four 16-byte groups");
            };
            let [mut w0, mut w1, mut w2, mut w3] = [a, b, c, d].map(|group| {
                // SAFETY: every borrowed group owns sixteen initialized bytes.
                vreinterpretq_u32_u8(vrev32q_u8(unsafe { vld1q_u8(group.as_ptr()) }))
            });
            macro_rules! rounds {
                ($i:literal, $w:ident) => {{
                    // SAFETY: these are groups of four consecutive constants
                    // at offsets 0 through 60.
                    let constants = unsafe { vld1q_u32(K32.as_ptr().add($i)) };
                    let words = vaddq_u32($w, constants);
                    let previous_abcd = abcd;
                    abcd = vsha256hq_u32(previous_abcd, efgh, words);
                    efgh = vsha256h2q_u32(efgh, previous_abcd, words);
                }};
            }
            macro_rules! expand {
                ($i:literal, $target:ident, $next:ident, $third:ident, $fourth:ident) => {{
                    let first = vsha256su0q_u32($target, $next);
                    $target = vsha256su1q_u32(first, $third, $fourth);
                    rounds!($i, $target);
                }};
            }
            rounds!(0, w0);
            rounds!(4, w1);
            rounds!(8, w2);
            rounds!(12, w3);
            expand!(16, w0, w1, w2, w3);
            expand!(20, w1, w2, w3, w0);
            expand!(24, w2, w3, w0, w1);
            expand!(28, w3, w0, w1, w2);
            expand!(32, w0, w1, w2, w3);
            expand!(36, w1, w2, w3, w0);
            expand!(40, w2, w3, w0, w1);
            expand!(44, w3, w0, w1, w2);
            expand!(48, w0, w1, w2, w3);
            expand!(52, w1, w2, w3, w0);
            expand!(56, w2, w3, w0, w1);
            expand!(60, w3, w0, w1, w2);
            abcd = vaddq_u32(abcd, before.0);
            efgh = vaddq_u32(efgh, before.1);
        }
    }
    // SAFETY: both stores fit the eight writable state words.
    unsafe {
        let p = state.as_mut_ptr();
        vst1q_u32(p, abcd);
        vst1q_u32(p.add(4), efgh);
    }
}
