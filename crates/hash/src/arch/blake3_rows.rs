// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Four chunks, each occupying four lanes of a 512-bit register. A single
//! vector G evaluates all four independent quarter rounds in all four chunks.

use crate::blake3::{IV, SCHEDULE};
use core::arch::x86_64::{
    __m512i, _mm_loadu_si128, _mm512_add_epi32, _mm512_castps_si512, _mm512_castsi128_si512,
    _mm512_castsi512_ps, _mm512_inserti32x4, _mm512_loadu_si512, _mm512_mask_permutexvar_epi32,
    _mm512_mask_set1_epi32, _mm512_permutex2var_epi32, _mm512_ror_epi32, _mm512_set1_epi32,
    _mm512_setr_epi32, _mm512_shuffle_epi32, _mm512_shuffle_ps, _mm512_storeu_si512,
    _mm512_xor_si512,
};

// The register groups are [m0,m2,m4,m6], [m1,m3,m5,m7],
// [m14,m8,m10,m12], [m15,m9,m11,m13]. Applying the fixed BLAKE3
// permutation in this basis requires seven two-source/selected permutations.
#[inline]
#[target_feature(enable = "avx512f")]
fn indices<const A: usize, const B: usize, const C: usize, const D: usize>() -> __m512i {
    let pick = [A, B, C, D];
    let words: [u32; 16] =
        core::array::from_fn(|i| (i / 4 * 4 + pick[i % 4] % 4 + (pick[i % 4] / 4) * 16) as u32);
    // SAFETY: the array contains sixteen readable lanes.
    unsafe { _mm512_loadu_si512(words.as_ptr().cast()) }
}
/// The BLAKE3 message-word permutation applied to the four row registers between rounds:
/// a fixed shuffle of message words, not a seeded shuffle of a slice.
#[inline]
#[target_feature(enable = "avx512f")]
fn permute(m: &mut [__m512i; 4]) {
    let a = _mm512_permutex2var_epi32(m[0], indices::<1, 5, 7, 2>(), m[1]);
    let b = _mm512_permutex2var_epi32(m[0], indices::<3, 6, 0, 0>(), m[2]);
    let b = _mm512_mask_permutexvar_epi32(b, 0x8888, indices::<0, 0, 0, 3>(), m[3]);
    let c = _mm512_permutex2var_epi32(m[3], indices::<0, 4, 0, 1>(), m[1]);
    let c = _mm512_mask_permutexvar_epi32(c, 0x4444, indices::<0, 0, 3, 0>(), m[2]);
    let d = _mm512_permutex2var_epi32(m[2], indices::<1, 6, 0, 0>(), m[3]);
    let d = _mm512_mask_permutexvar_epi32(d, 0x4444, indices::<0, 0, 2, 0>(), m[1]);
    *m = [a, b, c, d];
}
// Guard the explicitly grouped permutation against the crate's schedule.
const _: () = {
    let expected = [2, 6, 3, 10, 7, 0, 4, 13, 1, 11, 12, 5, 9, 14, 15, 8];
    let mut i = 0;
    while i < 16 {
        assert!(SCHEDULE[1][i] == expected[i]);
        i += 1;
    }
};

#[inline]
#[target_feature(enable = "avx512f")]
fn initial(first: usize) -> __m512i {
    let [a, b, c, d] = [
        IV[first] as i32,
        IV[first + 1] as i32,
        IV[first + 2] as i32,
        IV[first + 3] as i32,
    ];
    _mm512_setr_epi32(a, b, c, d, a, b, c, d, a, b, c, d, a, b, c, d)
}

/// All loads are inside four complete chunks; the caller checks AVX-512F.
#[inline]
#[target_feature(enable = "avx512f")]
pub(crate) fn chunks(bytes: &[u8; 4096], counter: u64) -> [[u32; 8]; 4] {
    let mut cv_a = initial(0);
    let mut cv_b = initial(4);
    let counters: [u32; 16] = core::array::from_fn(|lane| match lane % 4 {
        0 => (counter + (lane / 4) as u64) as u32,
        1 => ((counter + (lane / 4) as u64) >> 32) as u32,
        2 => 64,
        _ => 0,
    });
    // SAFETY: the local array has sixteen readable lanes. Counters and
    // block length stay invariant across all sixteen compression blocks.
    let counters = unsafe { _mm512_loadu_si512(counters.as_ptr().cast()) };
    for block in 0..16 {
        let mut rows = [_mm512_set1_epi32(0); 4];
        for (row, output) in rows.iter_mut().enumerate() {
            // SAFETY: block < 16, row < 4, and each chunk has 1024 bytes.
            // Each unaligned read covers one full 16-byte row of its block.
            unsafe {
                let ptr = bytes.as_ptr().add(block * 64 + row * 16);
                let first = _mm512_castsi128_si512(_mm_loadu_si128(ptr.cast()));
                let second = _mm512_inserti32x4::<1>(first, _mm_loadu_si128(ptr.add(1024).cast()));
                let third = _mm512_inserti32x4::<2>(second, _mm_loadu_si128(ptr.add(2048).cast()));
                *output = _mm512_inserti32x4::<3>(third, _mm_loadu_si128(ptr.add(3072).cast()));
            }
        }
        // Split each pair of four-word rows into even and odd messages.
        let even_first = _mm512_castps_si512(_mm512_shuffle_ps::<0x88>(
            _mm512_castsi512_ps(rows[0]),
            _mm512_castsi512_ps(rows[1]),
        ));
        let odd_first = _mm512_castps_si512(_mm512_shuffle_ps::<0xdd>(
            _mm512_castsi512_ps(rows[0]),
            _mm512_castsi512_ps(rows[1]),
        ));
        let even_last = _mm512_castps_si512(_mm512_shuffle_ps::<0x88>(
            _mm512_castsi512_ps(rows[2]),
            _mm512_castsi512_ps(rows[3]),
        ));
        let odd_last = _mm512_castps_si512(_mm512_shuffle_ps::<0xdd>(
            _mm512_castsi512_ps(rows[2]),
            _mm512_castsi512_ps(rows[3]),
        ));
        let mut rows = [
            even_first,
            odd_first,
            _mm512_shuffle_epi32::<0x93>(even_last),
            _mm512_shuffle_epi32::<0x93>(odd_last),
        ];
        let flags = u32::from(block == 0) | (u32::from(block == 15) << 1);
        let mut a = cv_a;
        let mut b = cv_b;
        let mut c = initial(0);
        let mut d = _mm512_mask_set1_epi32(counters, 0x8888, flags as i32);
        macro_rules! g {
            ($x:expr, $y:expr) => {{
                a = _mm512_add_epi32(_mm512_add_epi32(a, b), $x);
                d = _mm512_ror_epi32::<16>(_mm512_xor_si512(d, a));
                c = _mm512_add_epi32(c, d);
                b = _mm512_ror_epi32::<12>(_mm512_xor_si512(b, c));
                a = _mm512_add_epi32(_mm512_add_epi32(a, b), $y);
                d = _mm512_ror_epi32::<8>(_mm512_xor_si512(d, a));
                c = _mm512_add_epi32(c, d);
                b = _mm512_ror_epi32::<7>(_mm512_xor_si512(b, c));
            }};
        }
        macro_rules! round {
            () => {{
                g!(rows[0], rows[1]);
                // b completes last in G. Keep it stationary and rotate the
                // other rows, so their shuffles overlap b's final operations.
                a = _mm512_shuffle_epi32::<0x93>(a);
                c = _mm512_shuffle_epi32::<0x39>(c);
                d = _mm512_shuffle_epi32::<0x4e>(d);
                g!(rows[2], rows[3]);
                a = _mm512_shuffle_epi32::<0x39>(a);
                c = _mm512_shuffle_epi32::<0x93>(c);
                d = _mm512_shuffle_epi32::<0x4e>(d);
            }};
        }
        round!();
        permute(&mut rows);
        round!();
        permute(&mut rows);
        round!();
        permute(&mut rows);
        round!();
        permute(&mut rows);
        round!();
        permute(&mut rows);
        round!();
        permute(&mut rows);
        round!();
        cv_a = _mm512_xor_si512(a, c);
        cv_b = _mm512_xor_si512(b, d);
    }
    let mut low = [0u32; 16];
    let mut high = [0u32; 16];
    // SAFETY: both arrays hold all sixteen output lanes.
    unsafe {
        _mm512_storeu_si512(low.as_mut_ptr().cast(), cv_a);
        _mm512_storeu_si512(high.as_mut_ptr().cast(), cv_b);
    }
    core::array::from_fn(|chunk| {
        core::array::from_fn(|word| {
            if word < 4 {
                low[chunk * 4 + word]
            } else {
                high[chunk * 4 + word - 4]
            }
        })
    })
}
