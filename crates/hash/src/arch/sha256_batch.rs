// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! SHA-256 across independent AVX-512 lanes. The arithmetic follows FIPS
//! 180-4 §6.2.2, with Intel's VPGATHERDD, VPSHUFB, VPRORD and VPTERNLOGD.

use core::arch::x86_64::{
    __m512i, _mm_setr_epi8, _mm512_add_epi32, _mm512_broadcast_i32x4, _mm512_cmpgt_epi32_mask,
    _mm512_i32gather_epi32, _mm512_loadu_si512, _mm512_mask_add_epi32, _mm512_ror_epi32,
    _mm512_set1_epi32, _mm512_setr_epi32, _mm512_setzero_si512, _mm512_shuffle_epi8,
    _mm512_srli_epi32, _mm512_ternarylogic_epi32,
};

use crate::sha2::{
    IV256 as IV, K32 as K, Sha256Batch,
    batch::{BYTES, LANES, STRIDE},
};

pub(crate) fn sha256_batch_available() -> bool {
    std::is_x86_feature_detected!("avx512f") && std::is_x86_feature_detected!("avx512bw")
}

/// Hash the batch owner's initialized lane matrix on AVX-512.
pub(crate) fn sha256_batch(batch: &Sha256Batch) -> Option<[[u8; 32]; LANES]> {
    if !sha256_batch_available() {
        return None;
    }
    let (data, blocks, count) = batch.padded_lanes();
    // SAFETY: feature detection establishes both required extensions. Only
    // Sha256Batch can mint the initialized matrix and its validated lengths.
    Some(unsafe { compress(data, blocks, count) })
}

#[target_feature(enable = "avx512f,avx512bw")]
unsafe fn compress(data: &[u8; BYTES], blocks: &[i32; LANES], count: usize) -> [[u8; 32]; LANES] {
    let word_stride = (STRIDE / 4) as i32;
    let offsets = _mm512_setr_epi32(
        0,
        word_stride,
        2 * word_stride,
        3 * word_stride,
        4 * word_stride,
        5 * word_stride,
        6 * word_stride,
        7 * word_stride,
        8 * word_stride,
        9 * word_stride,
        10 * word_stride,
        11 * word_stride,
        12 * word_stride,
        13 * word_stride,
        14 * word_stride,
        15 * word_stride,
    );
    // SAFETY: blocks is exactly one fully initialized vector.
    let length = unsafe { _mm512_loadu_si512(blocks.as_ptr().cast()) };
    let reverse = _mm512_broadcast_i32x4(_mm_setr_epi8(
        3, 2, 1, 0, 7, 6, 5, 4, 11, 10, 9, 8, 15, 14, 13, 12,
    ));
    let mut state = IV.map(|word| _mm512_set1_epi32(word as i32));
    let groups = blocks[..count].iter().copied().max().unwrap_or(0) as usize;
    for group in 0..groups {
        let still_active = _mm512_cmpgt_epi32_mask(length, _mm512_set1_epi32(group as i32));
        let mut schedule = [_mm512_setzero_si512(); 16];
        for (i, column) in schedule.iter_mut().enumerate() {
            // SAFETY: offsets are the starts of the fixed owned lanes,
            // in words. group and i select a four-byte column within
            // the first lane's 512 bytes, and so within all other lanes.
            let raw = unsafe {
                let p = data.as_ptr().cast::<u8>().add(group * 64 + i * 4);
                _mm512_i32gather_epi32::<4>(offsets, p.cast())
            };
            *column = _mm512_shuffle_epi8(raw, reverse);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = state;
        macro_rules! round {
            ($index:expr, $slot:literal) => {{
                let choose = _mm512_ternarylogic_epi32::<0xca>(e, f, g);
                let majority = _mm512_ternarylogic_epi32::<0xe8>(a, b, c);
                let hi = _mm512_ternarylogic_epi32::<0x96>(
                    _mm512_ror_epi32::<6>(e),
                    _mm512_ror_epi32::<11>(e),
                    _mm512_ror_epi32::<25>(e),
                );
                let lo = _mm512_ternarylogic_epi32::<0x96>(
                    _mm512_ror_epi32::<2>(a),
                    _mm512_ror_epi32::<13>(a),
                    _mm512_ror_epi32::<22>(a),
                );
                let first = _mm512_add_epi32(
                    _mm512_add_epi32(_mm512_add_epi32(h, hi), choose),
                    _mm512_add_epi32(schedule[$slot], _mm512_set1_epi32(K[$index] as i32)),
                );
                let second = _mm512_add_epi32(lo, majority);
                [a, b, c, d, e, f, g, h] = [
                    _mm512_add_epi32(first, second),
                    a,
                    b,
                    c,
                    _mm512_add_epi32(d, first),
                    e,
                    f,
                    g,
                ];
            }};
        }
        macro_rules! extend {
            ($base:literal, $slot:literal) => {{
                let x = schedule[($slot + 1) & 15];
                let y = schedule[($slot + 14) & 15];
                let lo = _mm512_ternarylogic_epi32::<0x96>(
                    _mm512_ror_epi32::<7>(x),
                    _mm512_ror_epi32::<18>(x),
                    _mm512_srli_epi32::<3>(x),
                );
                let hi = _mm512_ternarylogic_epi32::<0x96>(
                    _mm512_ror_epi32::<17>(y),
                    _mm512_ror_epi32::<19>(y),
                    _mm512_srli_epi32::<10>(y),
                );
                schedule[$slot] = _mm512_add_epi32(
                    _mm512_add_epi32(schedule[$slot], schedule[($slot + 9) & 15]),
                    _mm512_add_epi32(lo, hi),
                );
                round!($base + $slot, $slot);
            }};
        }
        round!(0, 0);
        round!(1, 1);
        round!(2, 2);
        round!(3, 3);
        round!(4, 4);
        round!(5, 5);
        round!(6, 6);
        round!(7, 7);
        round!(8, 8);
        round!(9, 9);
        round!(10, 10);
        round!(11, 11);
        round!(12, 12);
        round!(13, 13);
        round!(14, 14);
        round!(15, 15);
        extend!(16, 0);
        extend!(16, 1);
        extend!(16, 2);
        extend!(16, 3);
        extend!(16, 4);
        extend!(16, 5);
        extend!(16, 6);
        extend!(16, 7);
        extend!(16, 8);
        extend!(16, 9);
        extend!(16, 10);
        extend!(16, 11);
        extend!(16, 12);
        extend!(16, 13);
        extend!(16, 14);
        extend!(16, 15);
        extend!(32, 0);
        extend!(32, 1);
        extend!(32, 2);
        extend!(32, 3);
        extend!(32, 4);
        extend!(32, 5);
        extend!(32, 6);
        extend!(32, 7);
        extend!(32, 8);
        extend!(32, 9);
        extend!(32, 10);
        extend!(32, 11);
        extend!(32, 12);
        extend!(32, 13);
        extend!(32, 14);
        extend!(32, 15);
        extend!(48, 0);
        extend!(48, 1);
        extend!(48, 2);
        extend!(48, 3);
        extend!(48, 4);
        extend!(48, 5);
        extend!(48, 6);
        extend!(48, 7);
        extend!(48, 8);
        extend!(48, 9);
        extend!(48, 10);
        extend!(48, 11);
        extend!(48, 12);
        extend!(48, 13);
        extend!(48, 14);
        extend!(48, 15);
        for (word, add) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *word = _mm512_mask_add_epi32(*word, still_active, *word, add);
        }
    }
    let mut out = [[0; 32]; LANES];
    for (i, word) in state.into_iter().enumerate() {
        let word = _mm512_shuffle_epi8(word, reverse);
        // SAFETY: the vector and the array are each exactly 64 bytes.
        let column = unsafe { core::mem::transmute::<__m512i, [[u8; 4]; LANES]>(word) };
        for (dst, bytes) in out[..count].iter_mut().zip(column) {
            dst[i * 4..i * 4 + 4].copy_from_slice(&bytes);
        }
    }
    out
}
