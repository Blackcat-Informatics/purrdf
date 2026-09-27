// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Armv8 kernels: SHA-1 on the SHA1 instructions, CRC-32 on the CRC32
//! instructions, alone or behind `pmull` folding.

use core::arch::aarch64::{
    __crc32b, __crc32d, uint32x4_t, uint64x2_t, vaddq_u32, vdupq_n_u32, vdupq_n_u64, veorq_u64,
    vgetq_lane_u32, vgetq_lane_u64, vld1q_u8, vmull_p64, vreinterpretq_u32_u8,
    vreinterpretq_u64_p128, vreinterpretq_u64_u8, vrev32q_u8, vsetq_lane_u32, vsetq_lane_u64,
    vsha1cq_u32, vsha1h_u32, vsha1mq_u32, vsha1pq_u32, vsha1su0q_u32, vsha1su1q_u32,
};
use std::arch::is_aarch64_feature_detected;

use super::{Crc32Update, Sha1Blocks};
use crate::crc32::FOLD;
use crate::sha1::K;

/// Sixteen bytes as a vector, byte `i` in lane `i`.
#[inline]
#[target_feature(enable = "neon")]
fn load(bytes: &[u8; 16]) -> core::arch::aarch64::uint8x16_t {
    // SAFETY: `bytes` is sixteen readable bytes; `vld1q_u8` needs only byte
    // alignment.
    unsafe { vld1q_u8(bytes.as_ptr()) }
}

// --- SHA-1 ----------------------------------------------------------------

/// The Armv8 SHA1 block function, if this processor has it (Rust's `sha2`
/// target feature covers FEAT_SHA1).
pub(crate) fn sha1_aarch64() -> Option<Sha1Blocks> {
    is_aarch64_feature_detected!("sha2").then_some(sha1_blocks as Sha1Blocks)
}

fn sha1_blocks(state: &mut [u32; 5], blocks: &[u8]) {
    // SAFETY: this function escapes the module only through `sha1_aarch64`,
    // which returns it after detecting `sha2`; NEON is part of the AArch64
    // baseline.
    unsafe { sha1_kernel(state, blocks) }
}

/// FIPS 180-4 §6.1.2 on the Armv8 SHA1 instructions.
///
/// Register layout (Arm's): lane 0 of ABCD is A, lane `i` of a message quad
/// is `W[4g + i]`. `sha1c`/`sha1p`/`sha1m` run four rounds of Ch, Parity or
/// Maj from ABCD, a scalar E and a quad with K already added; the E of the
/// next four rounds is `sha1h(A)`, A rotated by 30. The schedule for a quad is
/// `sha1su1(sha1su0(Q[g−4], Q[g−3], Q[g−2]), Q[g−1])`.
#[target_feature(enable = "neon,sha2")]
fn sha1_kernel(state: &mut [u32; 5], blocks: &[u8]) {
    let mut abcd = vsetq_lane_u32::<3>(
        state[3],
        vsetq_lane_u32::<2>(
            state[2],
            vsetq_lane_u32::<1>(state[1], vdupq_n_u32(state[0])),
        ),
    );
    let mut e = state[4];
    let k: [uint32x4_t; 4] = [
        vdupq_n_u32(K[0]),
        vdupq_n_u32(K[1]),
        vdupq_n_u32(K[2]),
        vdupq_n_u32(K[3]),
    ];
    let (blocks, _) = blocks.as_chunks::<64>();
    for block in blocks {
        let (quads, _) = block.as_chunks::<16>();
        let (abcd_start, e_start) = (abcd, e);
        // Byte-reverse each 32-bit lane: big-endian message words.
        let mut w = [
            vreinterpretq_u32_u8(vrev32q_u8(load(&quads[0]))),
            vreinterpretq_u32_u8(vrev32q_u8(load(&quads[1]))),
            vreinterpretq_u32_u8(vrev32q_u8(load(&quads[2]))),
            vreinterpretq_u32_u8(vrev32q_u8(load(&quads[3]))),
        ];
        macro_rules! quads {
            ($($g:literal => $rounds:ident),* $(,)?) => {$(
                if $g >= 4 {
                    w[$g % 4] = vsha1su1q_u32(
                        vsha1su0q_u32(w[$g % 4], w[($g + 1) % 4], w[($g + 2) % 4]),
                        w[($g + 3) % 4],
                    );
                }
                let with_k = vaddq_u32(w[$g % 4], k[$g / 5]);
                let a = vgetq_lane_u32::<0>(abcd);
                abcd = $rounds(abcd, e, with_k);
                e = vsha1h_u32(a);
            )*};
        }
        quads!(
            0 => vsha1cq_u32, 1 => vsha1cq_u32, 2 => vsha1cq_u32, 3 => vsha1cq_u32,
            4 => vsha1cq_u32,
            5 => vsha1pq_u32, 6 => vsha1pq_u32, 7 => vsha1pq_u32, 8 => vsha1pq_u32,
            9 => vsha1pq_u32,
            10 => vsha1mq_u32, 11 => vsha1mq_u32, 12 => vsha1mq_u32, 13 => vsha1mq_u32,
            14 => vsha1mq_u32,
            15 => vsha1pq_u32, 16 => vsha1pq_u32, 17 => vsha1pq_u32, 18 => vsha1pq_u32,
            19 => vsha1pq_u32,
        );
        abcd = vaddq_u32(abcd, abcd_start);
        e = e.wrapping_add(e_start);
    }
    state[0] = vgetq_lane_u32::<0>(abcd);
    state[1] = vgetq_lane_u32::<1>(abcd);
    state[2] = vgetq_lane_u32::<2>(abcd);
    state[3] = vgetq_lane_u32::<3>(abcd);
    state[4] = e;
}

// --- CRC-32 ---------------------------------------------------------------

/// The CRC32-instruction register update, if this processor has `crc`.
pub(crate) fn crc32_aarch64_crc32() -> Option<Crc32Update> {
    is_aarch64_feature_detected!("crc").then_some(crc32_update_crc as Crc32Update)
}

/// The `pmull`-folding register update, if this processor has `aes` (which
/// carries PMULL) and `crc`.
pub(crate) fn crc32_aarch64_pmull() -> Option<Crc32Update> {
    (is_aarch64_feature_detected!("aes") && is_aarch64_feature_detected!("crc"))
        .then_some(crc32_update_pmull as Crc32Update)
}

fn crc32_update_crc(register: u32, data: &[u8]) -> u32 {
    // SAFETY: this function escapes the module only through
    // `crc32_aarch64_crc32`, which returns it after detecting `crc`.
    unsafe { crc32_crc_kernel(register, data) }
}

fn crc32_update_pmull(register: u32, data: &[u8]) -> u32 {
    // SAFETY: this function escapes the module only through
    // `crc32_aarch64_pmull`, which returns it after detecting `aes` and `crc`;
    // NEON is part of the AArch64 baseline.
    unsafe {
        if data.len() < 64 {
            crc32_crc_kernel(register, data)
        } else {
            crc32_pmull_kernel(register, data)
        }
    }
}

/// The CRC32 instructions implement exactly this polynomial, reflected, on a
/// raw register: eight bytes (little-endian) per `crc32x`.
#[target_feature(enable = "crc")]
fn crc32_crc_kernel(mut register: u32, data: &[u8]) -> u32 {
    let (words, tail) = data.as_chunks::<8>();
    for word in words {
        register = __crc32d(register, u64::from_le_bytes(*word));
    }
    for &byte in tail {
        register = __crc32b(register, byte);
    }
    register
}

/// `acc` folded forward by the distance `k` encodes (see [`FOLD`]).
#[inline]
#[target_feature(enable = "neon,aes")]
fn fold(acc: uint64x2_t, k: [u64; 2]) -> uint64x2_t {
    let low = vmull_p64(vgetq_lane_u64::<0>(acc), k[0]);
    let high = vmull_p64(vgetq_lane_u64::<1>(acc), k[1]);
    veorq_u64(vreinterpretq_u64_p128(low), vreinterpretq_u64_p128(high))
}

/// The same folding as the x86-64 kernel, over `data` of at least 64 bytes.
/// The 128-bit accumulator F is finished by two `crc32x` from a zero
/// register, which compute F·x^32 mod P directly.
#[target_feature(enable = "neon,aes,crc")]
fn crc32_pmull_kernel(register: u32, data: &[u8]) -> u32 {
    let (chunks, tail) = data.as_chunks::<16>();
    debug_assert!(chunks.len() >= 4);
    let chunk = |bytes: &[u8; 16]| vreinterpretq_u64_u8(load(bytes));
    let mut acc = [
        veorq_u64(
            chunk(&chunks[0]),
            vsetq_lane_u64::<0>(u64::from(register), vdupq_n_u64(0)),
        ),
        chunk(&chunks[1]),
        chunk(&chunks[2]),
        chunk(&chunks[3]),
    ];
    let mut rest = &chunks[4..];
    while let Some((group, after)) = rest.split_first_chunk::<4>() {
        for (lane, bytes) in acc.iter_mut().zip(group) {
            *lane = veorq_u64(fold(*lane, FOLD.by_512), chunk(bytes));
        }
        rest = after;
    }
    let mut single = acc[0];
    for lane in &acc[1..] {
        single = veorq_u64(fold(single, FOLD.by_128), *lane);
    }
    for bytes in rest {
        single = veorq_u64(fold(single, FOLD.by_128), chunk(bytes));
    }
    let reduced = __crc32d(
        __crc32d(0, vgetq_lane_u64::<0>(single)),
        vgetq_lane_u64::<1>(single),
    );
    crc32_crc_kernel(reduced, tail)
}
