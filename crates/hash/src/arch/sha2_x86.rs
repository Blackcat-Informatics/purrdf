// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! SHA-256 with SHA-NI and AVX2 input loads, and SHA-512 with a paired
//! AVX-512VL message schedule and scalar rounds. FIPS 180-4 §6.2.2 and
//! §6.4.2, and Intel's SHA256RNDS2, SHA256MSG1/2, VPRORQ and VPTERNLOGQ.

use core::arch::x86_64::{
    _mm_add_epi32, _mm_add_epi64, _mm_alignr_epi8, _mm_castps_si128, _mm_castsi128_ps,
    _mm_cvtsi128_si64, _mm_extract_epi64, _mm_loadu_si128, _mm_ror_epi64, _mm_setr_epi8,
    _mm_setzero_si128, _mm_sha256msg1_epu32, _mm_sha256msg2_epu32, _mm_sha256rnds2_epu32,
    _mm_shuffle_epi8, _mm_shuffle_epi32, _mm_shuffle_ps, _mm_srli_epi64, _mm_storeu_si128,
    _mm_ternarylogic_epi64, _mm256_add_epi64, _mm256_alignr_epi8, _mm256_broadcastsi128_si256,
    _mm256_castsi256_si128, _mm256_extracti128_si256, _mm256_loadu_si256,
    _mm256_permute2x128_si256, _mm256_permute4x64_epi64, _mm256_setzero_si256,
    _mm256_sha512msg1_epi64, _mm256_sha512msg2_epi64, _mm256_sha512rnds2_epi64,
    _mm256_shuffle_epi8, _mm256_storeu_si256,
};
use std::is_x86_feature_detected;

use crate::sha2::{K32, K64, Sha256Blocks, Sha512Blocks};

pub(crate) fn sha256_blocks() -> Option<Sha256Blocks> {
    (is_x86_feature_detected!("sha")
        && is_x86_feature_detected!("ssse3")
        && is_x86_feature_detected!("avx2"))
    .then_some(sha256_entry)
}

/// The checked SHA-NI entry is a bridge to the x86-specific instruction set.
fn sha256_entry(state: &mut [u32; 8], whole: &[u8], tail: &[u8]) {
    // SAFETY: the entry is returned only after checking every kernel feature.
    unsafe { sha256_kernel(state, whole, tail) }
}

pub(crate) fn sha512_blocks() -> Option<Sha512Blocks> {
    (is_x86_feature_detected!("avx512f")
        && is_x86_feature_detected!("avx512vl")
        && is_x86_feature_detected!("bmi2")
        && is_x86_feature_detected!("ssse3")
        && is_x86_feature_detected!("sse4.1"))
    .then_some(sha512_entry)
}

pub(crate) fn sha512_instruction_blocks() -> Option<Sha512Blocks> {
    (is_x86_feature_detected!("sha512") && is_x86_feature_detected!("avx2"))
        .then_some(sha512_instruction_entry)
}

fn sha512_instruction_entry(state: &mut [u64; 8], whole: &[u8], tail: &[u8]) {
    // SAFETY: the entry is returned only with both instruction sets available.
    unsafe { sha512_instruction_kernel(state, whole, tail) }
}

/// The checked AVX-512 entry is a bridge to the x86-specific schedule.
fn sha512_entry(state: &mut [u64; 8], whole: &[u8], tail: &[u8]) {
    // SAFETY: the entry is returned only after checking every kernel feature.
    unsafe { sha512_kernel(state, whole, tail) }
}

#[target_feature(enable = "sha512,avx2")]
unsafe fn sha512_instruction_kernel(state: &mut [u64; 8], whole: &[u8], tail: &[u8]) {
    let reverse = _mm256_broadcastsi128_si256(_mm_setr_epi8(
        7, 6, 5, 4, 3, 2, 1, 0, 15, 14, 13, 12, 11, 10, 9, 8,
    ));
    // SAFETY: each unaligned load reads four of the eight owned state words.
    let (abcd, efgh) = unsafe {
        let ptr = state.as_ptr();
        (
            _mm256_loadu_si256(ptr.cast()),
            _mm256_loadu_si256(ptr.add(4).cast()),
        )
    };
    // In increasing lane order RNDS2 expects [F,E,B,A] and [H,G,D,C].
    let mut leading =
        _mm256_permute4x64_epi64::<0x1b>(_mm256_permute2x128_si256::<0x20>(abcd, efgh));
    let mut trailing =
        _mm256_permute4x64_epi64::<0x1b>(_mm256_permute2x128_si256::<0x31>(abcd, efgh));
    for input in [whole, tail] {
        for block in input.as_chunks::<128>().0 {
            let (prior_leading, prior_trailing) = (leading, trailing);
            let mut schedule = [_mm256_setzero_si256(); 4];
            for (words, bytes) in schedule.iter_mut().zip(block.as_chunks::<32>().0) {
                // SAFETY: each chunk is a complete vector of initialized bytes.
                *words = _mm256_shuffle_epi8(
                    unsafe { _mm256_loadu_si256(bytes.as_ptr().cast()) },
                    reverse,
                );
            }
            for group in 0..20 {
                let i = group & 3;
                if group >= 4 {
                    let next = schedule[(i + 1) & 3];
                    let previous = schedule[(i + 3) & 3];
                    let two_back = schedule[(i + 2) & 3];
                    let across = _mm256_permute2x128_si256::<0x21>(two_back, previous);
                    // The qwords at t-7 through t-4 straddle the last two groups.
                    let seven_back = _mm256_alignr_epi8::<8>(across, two_back);
                    let low = _mm256_sha512msg1_epi64(schedule[i], _mm256_castsi256_si128(next));
                    schedule[i] =
                        _mm256_sha512msg2_epi64(_mm256_add_epi64(low, seven_back), previous);
                }
                // SAFETY: each group selects four constants, within K64[0..80].
                let constants = unsafe { _mm256_loadu_si256(K64.as_ptr().add(4 * group).cast()) };
                let sum = _mm256_add_epi64(schedule[i], constants);
                // After two rounds the old leading lanes are the new trailing lanes.
                trailing = _mm256_sha512rnds2_epi64(trailing, leading, _mm256_castsi256_si128(sum));
                leading =
                    _mm256_sha512rnds2_epi64(leading, trailing, _mm256_extracti128_si256::<1>(sum));
            }
            leading = _mm256_add_epi64(leading, prior_leading);
            trailing = _mm256_add_epi64(trailing, prior_trailing);
        }
    }
    let abef = _mm256_permute4x64_epi64::<0x1b>(leading);
    let cdgh = _mm256_permute4x64_epi64::<0x1b>(trailing);
    // SAFETY: both stores fill disjoint halves of the state.
    unsafe {
        let ptr = state.as_mut_ptr();
        _mm256_storeu_si256(ptr.cast(), _mm256_permute2x128_si256::<0x20>(abef, cdgh));
        _mm256_storeu_si256(
            ptr.add(4).cast(),
            _mm256_permute2x128_si256::<0x31>(abef, cdgh),
        );
    }
}

#[target_feature(enable = "sha,ssse3,avx2")]
unsafe fn sha256_kernel(state: &mut [u32; 8], whole: &[u8], tail: &[u8]) {
    let reverse = _mm_setr_epi8(3, 2, 1, 0, 7, 6, 5, 4, 11, 10, 9, 8, 15, 14, 13, 12);
    let reverse_pair = _mm256_broadcastsi128_si256(reverse);
    // SHA256RNDS2 uses [F,E,B,A] and [H,G,D,C] in increasing lane order.
    // SAFETY: the state owns eight consecutive u32 values. Each load
    // reads exactly four words, and neither one requires alignment.
    let (abcd, efgh) = unsafe {
        let p = state.as_ptr();
        (_mm_loadu_si128(p.cast()), _mm_loadu_si128(p.add(4).cast()))
    };
    let left = _mm_castsi128_ps(efgh);
    let right = _mm_castsi128_ps(abcd);
    let mut abef = _mm_castps_si128(_mm_shuffle_ps::<0x11>(left, right));
    let mut cdgh = _mm_castps_si128(_mm_shuffle_ps::<0xbb>(left, right));
    for input in [whole, tail] {
        for block in input.as_chunks::<64>().0 {
            let (previous_abef, previous_cdgh) = (abef, cdgh);
            // SAFETY: the block is exactly 64 initialized bytes. AVX2's
            // shuffle reverses each word within its own 128-bit lane.
            let (lo, hi) = unsafe {
                let p = block.as_ptr();
                (
                    _mm256_loadu_si256(p.cast()),
                    _mm256_loadu_si256(p.add(32).cast()),
                )
            };
            let lo = _mm256_shuffle_epi8(lo, reverse_pair);
            let hi = _mm256_shuffle_epi8(hi, reverse_pair);
            let mut w0 = _mm256_castsi256_si128(lo);
            let mut w1 = _mm256_extracti128_si256::<1>(lo);
            let mut w2 = _mm256_castsi256_si128(hi);
            let mut w3 = _mm256_extracti128_si256::<1>(hi);
            macro_rules! rounds {
                ($j:literal, $w:ident) => {{
                    // SAFETY: K has 64 words and each invocation starts at the
                    // beginning of a four-word group (0 through 60).
                    let constants = unsafe { _mm_loadu_si128(K32.as_ptr().add($j).cast()) };
                    let next = _mm_add_epi32($w, constants);
                    cdgh = _mm_sha256rnds2_epu32(cdgh, abef, next);
                    abef = _mm_sha256rnds2_epu32(abef, cdgh, _mm_shuffle_epi32::<0x0e>(next));
                }};
            }
            macro_rules! extend {
                ($j:literal, $target:ident, $second:ident, $third:ident, $fourth:ident) => {{
                    // Four output words share the σ0 contribution from adjacent
                    // lanes, and W[t-7] spans the last two old groups. MSG2 adds
                    // the σ1 contributions, including its internal dependencies.
                    let low = _mm_sha256msg1_epu32($target, $second);
                    let seven_back = _mm_alignr_epi8::<4>($fourth, $third);
                    $target = _mm_sha256msg2_epu32(_mm_add_epi32(low, seven_back), $fourth);
                    rounds!($j, $target);
                }};
            }
            rounds!(0, w0);
            rounds!(4, w1);
            rounds!(8, w2);
            rounds!(12, w3);
            extend!(16, w0, w1, w2, w3);
            extend!(20, w1, w2, w3, w0);
            extend!(24, w2, w3, w0, w1);
            extend!(28, w3, w0, w1, w2);
            extend!(32, w0, w1, w2, w3);
            extend!(36, w1, w2, w3, w0);
            extend!(40, w2, w3, w0, w1);
            extend!(44, w3, w0, w1, w2);
            extend!(48, w0, w1, w2, w3);
            extend!(52, w1, w2, w3, w0);
            extend!(56, w2, w3, w0, w1);
            extend!(60, w3, w0, w1, w2);
            abef = _mm_add_epi32(abef, previous_abef);
            cdgh = _mm_add_epi32(cdgh, previous_cdgh);
        }
    }
    let left = _mm_castsi128_ps(abef);
    let right = _mm_castsi128_ps(cdgh);
    let abcd = _mm_castps_si128(_mm_shuffle_ps::<0xbb>(left, right));
    let efgh = _mm_castps_si128(_mm_shuffle_ps::<0x11>(left, right));
    // SAFETY: each store writes four of the state's eight words.
    unsafe {
        let p = state.as_mut_ptr();
        _mm_storeu_si128(p.cast(), abcd);
        _mm_storeu_si128(p.add(4).cast(), efgh);
    }
}

#[target_feature(enable = "avx512f,avx512vl,bmi2,ssse3,sse4.1")]
unsafe fn sha512_kernel(state: &mut [u64; 8], whole: &[u8], tail: &[u8]) {
    let reverse = _mm_setr_epi8(7, 6, 5, 4, 3, 2, 1, 0, 15, 14, 13, 12, 11, 10, 9, 8);
    for input in [whole, tail] {
        for block in input.as_chunks::<128>().0 {
            let mut w = [_mm_setzero_si128(); 8];
            for (out, bytes) in w.iter_mut().zip(block.as_chunks::<16>().0) {
                // SAFETY: bytes contains a full sixteen-byte vector.
                *out = _mm_shuffle_epi8(unsafe { _mm_loadu_si128(bytes.as_ptr().cast()) }, reverse);
            }
            let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *state;
            macro_rules! round {
                ($kw:expr) => {{
                    let high = e.rotate_right(14) ^ e.rotate_right(18) ^ e.rotate_right(41);
                    let low = a.rotate_right(28) ^ a.rotate_right(34) ^ a.rotate_right(39);
                    let first = h
                        .wrapping_add(high)
                        .wrapping_add(g ^ (e & (f ^ g)))
                        .wrapping_add($kw);
                    let second = low.wrapping_add(b ^ ((a ^ b) & (b ^ c)));
                    [a, b, c, d, e, f, g, h] = [
                        first.wrapping_add(second),
                        a,
                        b,
                        c,
                        d.wrapping_add(first),
                        e,
                        f,
                        g,
                    ];
                }};
            }
            macro_rules! pair {
                ($t:expr, $slot:literal) => {{
                    // SAFETY: a pair is two complete consecutive constants, ending
                    // at or before round 79.
                    let k = unsafe { _mm_loadu_si128(K64.as_ptr().add($t).cast()) };
                    let next = _mm_add_epi64(w[$slot], k);
                    round!(_mm_cvtsi128_si64(next) as u64);
                    round!(_mm_extract_epi64::<1>(next) as u64);
                }};
            }
            macro_rules! extend {
                ($base:expr, $i:literal) => {{
                    let x = _mm_alignr_epi8::<8>(w[($i + 1) & 7], w[$i]);
                    let y = w[($i + 7) & 7];
                    let seven_back = _mm_alignr_epi8::<8>(w[($i + 5) & 7], w[($i + 4) & 7]);
                    let lo = _mm_ternarylogic_epi64::<0x96>(
                        _mm_ror_epi64::<1>(x),
                        _mm_ror_epi64::<8>(x),
                        _mm_srli_epi64::<7>(x),
                    );
                    let hi = _mm_ternarylogic_epi64::<0x96>(
                        _mm_ror_epi64::<19>(y),
                        _mm_ror_epi64::<61>(y),
                        _mm_srli_epi64::<6>(y),
                    );
                    w[$i] = _mm_add_epi64(_mm_add_epi64(w[$i], lo), _mm_add_epi64(seven_back, hi));
                    pair!($base + 2 * $i, $i);
                }};
            }
            pair!(0, 0);
            pair!(2, 1);
            pair!(4, 2);
            pair!(6, 3);
            pair!(8, 4);
            pair!(10, 5);
            pair!(12, 6);
            pair!(14, 7);
            for base in [16, 32, 48, 64] {
                extend!(base, 0);
                extend!(base, 1);
                extend!(base, 2);
                extend!(base, 3);
                extend!(base, 4);
                extend!(base, 5);
                extend!(base, 6);
                extend!(base, 7);
            }
            for (value, add) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
                *value = value.wrapping_add(add);
            }
        }
    }
}
