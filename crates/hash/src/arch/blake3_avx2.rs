// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Eight independent BLAKE3 chunks on AVX2, without AVX-512 requirements.
use core::arch::x86_64::{
    __m256i, _mm256_add_epi32, _mm256_and_si256, _mm256_andnot_si256, _mm256_loadu_si256,
    _mm256_or_si256, _mm256_permute2x128_si256, _mm256_set1_epi32, _mm256_slli_epi32,
    _mm256_srli_epi32, _mm256_storeu_si256, _mm256_unpackhi_epi32, _mm256_unpackhi_epi64,
    _mm256_unpacklo_epi32, _mm256_unpacklo_epi64, _mm256_xor_si256,
};
#[derive(Clone, Copy)]
struct V(__m256i);
impl V {
    #[inline]
    #[target_feature(enable = "avx2")]
    fn splat(x: u32) -> Self {
        Self(_mm256_set1_epi32(x as i32))
    }
    #[inline]
    #[target_feature(enable = "avx2")]
    fn load(x: [u32; 8]) -> Self {
        // SAFETY: the array contains all eight readable words.
        unsafe { Self(_mm256_loadu_si256(x.as_ptr().cast())) }
    }
    #[inline]
    #[target_feature(enable = "avx2")]
    fn array(self) -> [u32; 8] {
        let mut a = [0; 8];
        // SAFETY: the array contains all eight writable words.
        unsafe {
            _mm256_storeu_si256(a.as_mut_ptr().cast(), self.0);
        }
        a
    }
    #[inline]
    #[target_feature(enable = "avx2")]
    fn add(self, b: Self) -> Self {
        Self(_mm256_add_epi32(self.0, b.0))
    }
    #[inline]
    #[target_feature(enable = "avx2")]
    fn xor(self, b: Self) -> Self {
        Self(_mm256_xor_si256(self.0, b.0))
    }
    #[inline]
    #[target_feature(enable = "avx2")]
    fn rotr<const R: i32, const L: i32>(self) -> Self {
        if R == 16 {
            return Self(_mm256_shuffle_epi8(
                self.0,
                _mm256_setr_epi8(
                    2, 3, 0, 1, 6, 7, 4, 5, 10, 11, 8, 9, 14, 15, 12, 13, 2, 3, 0, 1, 6, 7, 4, 5,
                    10, 11, 8, 9, 14, 15, 12, 13,
                ),
            ));
        }
        if R == 8 {
            return Self(_mm256_shuffle_epi8(
                self.0,
                _mm256_setr_epi8(
                    1, 2, 3, 0, 5, 6, 7, 4, 9, 10, 11, 8, 13, 14, 15, 12, 1, 2, 3, 0, 5, 6, 7, 4,
                    9, 10, 11, 8, 13, 14, 15, 12,
                ),
            ));
        }
        Self(_mm256_or_si256(
            _mm256_srli_epi32::<R>(self.0),
            _mm256_slli_epi32::<L>(self.0),
        ))
    }
    #[inline]
    #[target_feature(enable = "avx2")]
    fn select(self, mask: Self, yes: Self) -> Self {
        Self(_mm256_or_si256(
            _mm256_and_si256(mask.0, yes.0),
            _mm256_andnot_si256(mask.0, self.0),
        ))
    }
}
#[inline]
#[target_feature(enable = "avx2")]
fn load_block(bytes: &[u8; 8192], block: usize) -> [V; 16] {
    assert!(block < 16);
    let mut out = [V::splat(0); 16];
    for half in 0..2 {
        let mut rows = [_mm256_set1_epi32(0); 8];
        for (i, row) in rows.iter_mut().enumerate() {
            // SAFETY: the load is a 32-byte half of a full block within chunk i.
            *row = unsafe {
                _mm256_loadu_si256(bytes.as_ptr().add(i * 1024 + block * 64 + half * 32).cast())
            };
        }
        let mut pairs = [_mm256_set1_epi32(0); 8];
        for i in (0..8).step_by(2) {
            pairs[i] = _mm256_unpacklo_epi32(rows[i], rows[i + 1]);
            pairs[i + 1] = _mm256_unpackhi_epi32(rows[i], rows[i + 1]);
        }
        let mut groups = [_mm256_set1_epi32(0); 8];
        for i in (0..8).step_by(4) {
            groups[i] = _mm256_unpacklo_epi64(pairs[i], pairs[i + 2]);
            groups[i + 1] = _mm256_unpackhi_epi64(pairs[i], pairs[i + 2]);
            groups[i + 2] = _mm256_unpacklo_epi64(pairs[i + 1], pairs[i + 3]);
            groups[i + 3] = _mm256_unpackhi_epi64(pairs[i + 1], pairs[i + 3]);
        }
        for col in 0..4 {
            out[half * 8 + col] = V(_mm256_permute2x128_si256::<0x20>(
                groups[col],
                groups[col + 4],
            ));
            out[half * 8 + col + 4] = V(_mm256_permute2x128_si256::<0x31>(
                groups[col],
                groups[col + 4],
            ));
        }
    }
    out
}
super::blake3_lanes::blake3_lanes!("avx2", 8);

use core::arch::x86_64::{_mm256_setr_epi8, _mm256_shuffle_epi8};
