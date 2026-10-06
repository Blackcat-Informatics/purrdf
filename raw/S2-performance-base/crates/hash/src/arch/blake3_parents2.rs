// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Two BLAKE3 parents packed as two four-word groups in 256-bit vectors.

use crate::blake3::{IV, SCHEDULE};
use core::arch::x86_64::{
    __m256i, _mm_loadu_si128, _mm256_add_epi32, _mm256_castsi128_si256, _mm256_inserti128_si256,
    _mm256_loadu_si256, _mm256_mask_blend_epi32, _mm256_permutex2var_epi32, _mm256_ror_epi32,
    _mm256_set1_epi32, _mm256_setr_epi32, _mm256_shuffle_epi32, _mm256_storeu_si256,
    _mm256_xor_si256,
};

#[inline]
#[target_feature(enable = "avx2,avx512f,avx512vl")]
fn message<const A: usize, const B: usize, const C: usize, const D: usize>(
    rows: &[__m256i; 4],
) -> __m256i {
    let pick = [A, B, C, D];
    let indices: [u32; 8] = core::array::from_fn(|lane| {
        (lane / 4 * 4 + pick[lane % 4] % 4 + ((pick[lane % 4] / 4) % 2) * 8) as u32
    });
    // SAFETY: the local array contains all eight lanes.
    let index = unsafe { _mm256_loadu_si256(indices.as_ptr().cast()) };
    if A < 8 && B < 8 && C < 8 && D < 8 {
        return _mm256_permutex2var_epi32(rows[0], index, rows[1]);
    }
    if A >= 8 && B >= 8 && C >= 8 && D >= 8 {
        return _mm256_permutex2var_epi32(rows[2], index, rows[3]);
    }
    let low = _mm256_permutex2var_epi32(rows[0], index, rows[1]);
    let high = _mm256_permutex2var_epi32(rows[2], index, rows[3]);
    let nibble = u8::from(A >= 8)
        | (u8::from(B >= 8) << 1)
        | (u8::from(C >= 8) << 2)
        | (u8::from(D >= 8) << 3);
    _mm256_mask_blend_epi32(nibble * 0x11, low, high)
}

#[inline]
#[target_feature(enable = "avx2,avx512f,avx512vl")]
fn initial(first: usize) -> __m256i {
    let [a, b, c, d] = [
        IV[first] as i32,
        IV[first + 1] as i32,
        IV[first + 2] as i32,
        IV[first + 3] as i32,
    ];
    _mm256_setr_epi32(a, b, c, d, a, b, c, d)
}

/// Hashes the two independent parent blocks made from four child CVs.
#[inline]
#[target_feature(enable = "avx2,avx512f,avx512vl")]
pub(crate) fn parents(children: &[[u32; 8]; 4]) -> [[u32; 8]; 2] {
    let mut rows = [_mm256_set1_epi32(0); 4];
    for (row, output) in rows.iter_mut().enumerate() {
        // SAFETY: the four child arrays occupy exactly 128 readable bytes.
        // Each load covers sixteen bytes inside one of the two parent blocks.
        unsafe {
            let ptr = children.as_ptr().cast::<u8>().add(row * 16);
            let first = _mm256_castsi128_si256(_mm_loadu_si128(ptr.cast()));
            *output = _mm256_inserti128_si256::<1>(first, _mm_loadu_si128(ptr.add(64).cast()));
        }
    }
    let mut a = initial(0);
    let mut b = initial(4);
    let mut c = initial(0);
    let mut d = _mm256_setr_epi32(0, 0, 64, 4, 0, 0, 64, 4);
    macro_rules! g {
        ($x:expr, $y:expr) => {{
            a = _mm256_add_epi32(_mm256_add_epi32(a, b), $x);
            d = _mm256_ror_epi32::<16>(_mm256_xor_si256(d, a));
            c = _mm256_add_epi32(c, d);
            b = _mm256_ror_epi32::<12>(_mm256_xor_si256(b, c));
            a = _mm256_add_epi32(_mm256_add_epi32(a, b), $y);
            d = _mm256_ror_epi32::<8>(_mm256_xor_si256(d, a));
            c = _mm256_add_epi32(c, d);
            b = _mm256_ror_epi32::<7>(_mm256_xor_si256(b, c));
        }};
    }
    macro_rules! round {
        ($r:expr) => {{
            g!(
                message::<
                    { SCHEDULE[$r][0] },
                    { SCHEDULE[$r][2] },
                    { SCHEDULE[$r][4] },
                    { SCHEDULE[$r][6] },
                >(&rows),
                message::<
                    { SCHEDULE[$r][1] },
                    { SCHEDULE[$r][3] },
                    { SCHEDULE[$r][5] },
                    { SCHEDULE[$r][7] },
                >(&rows)
            );
            a = _mm256_shuffle_epi32::<0x93>(a);
            c = _mm256_shuffle_epi32::<0x39>(c);
            d = _mm256_shuffle_epi32::<0x4e>(d);
            g!(
                message::<
                    { SCHEDULE[$r][14] },
                    { SCHEDULE[$r][8] },
                    { SCHEDULE[$r][10] },
                    { SCHEDULE[$r][12] },
                >(&rows),
                message::<
                    { SCHEDULE[$r][15] },
                    { SCHEDULE[$r][9] },
                    { SCHEDULE[$r][11] },
                    { SCHEDULE[$r][13] },
                >(&rows)
            );
            a = _mm256_shuffle_epi32::<0x39>(a);
            c = _mm256_shuffle_epi32::<0x93>(c);
            d = _mm256_shuffle_epi32::<0x4e>(d);
        }};
    }
    round!(0);
    round!(1);
    round!(2);
    round!(3);
    round!(4);
    round!(5);
    round!(6);
    let mut low = [0u32; 8];
    let mut high = [0u32; 8];
    // SAFETY: both arrays hold all eight output lanes.
    unsafe {
        _mm256_storeu_si256(low.as_mut_ptr().cast(), _mm256_xor_si256(a, c));
        _mm256_storeu_si256(high.as_mut_ptr().cast(), _mm256_xor_si256(b, d));
    }
    core::array::from_fn(|parent| {
        core::array::from_fn(|word| {
            if word < 4 {
                low[parent * 4 + word]
            } else {
                high[parent * 4 + word - 4]
            }
        })
    })
}
