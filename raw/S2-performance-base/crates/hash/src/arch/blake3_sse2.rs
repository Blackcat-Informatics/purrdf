// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Four independent BLAKE3 chunks on x86 SSE2.
#[cfg(target_arch = "x86")]
use core::arch::x86 as intrinsics;
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64 as intrinsics;
use intrinsics::{
    __m128i, _mm_add_epi32, _mm_and_si128, _mm_andnot_si128, _mm_loadu_si128, _mm_or_si128,
    _mm_set1_epi32, _mm_slli_epi32, _mm_srli_epi32, _mm_storeu_si128, _mm_unpackhi_epi32,
    _mm_unpackhi_epi64, _mm_unpacklo_epi32, _mm_unpacklo_epi64, _mm_xor_si128,
};
// One set of loads and arithmetic wrappers serves SSE2 and AVX-512VL.
// Only the rotation instruction and the checked target-feature contract differ.
macro_rules! four_words {
    ($feature:literal, $rotate:ident) => {
        #[derive(Clone, Copy)]
        struct V(__m128i);
        impl V {
            #[inline]
            #[target_feature(enable = $feature)]
            fn splat(x: u32) -> Self {
                Self(_mm_set1_epi32(x as i32))
            }
            #[inline]
            #[target_feature(enable = $feature)]
            fn load(x: [u32; 4]) -> Self {
                unsafe { Self(_mm_loadu_si128(x.as_ptr().cast())) }
            }
            #[inline]
            #[target_feature(enable = $feature)]
            fn array(self) -> [u32; 4] {
                let mut a = [0; 4];
                unsafe {
                    _mm_storeu_si128(a.as_mut_ptr().cast(), self.0);
                }
                a
            }
            #[inline]
            #[target_feature(enable = $feature)]
            fn add(self, b: Self) -> Self {
                Self(_mm_add_epi32(self.0, b.0))
            }
            #[inline]
            #[target_feature(enable = $feature)]
            fn xor(self, b: Self) -> Self {
                Self(_mm_xor_si128(self.0, b.0))
            }
            #[inline]
            #[target_feature(enable = $feature)]
            fn rotr<const R: i32, const L: i32>(self) -> Self {
                $rotate!(self.0, R, L)
            }
            #[inline]
            #[target_feature(enable = $feature)]
            fn select(self, mask: Self, yes: Self) -> Self {
                Self(_mm_or_si128(
                    _mm_and_si128(mask.0, yes.0),
                    _mm_andnot_si128(mask.0, self.0),
                ))
            }
        }
        // SAFETY for the primitive wrappers: SSE2 is baseline on x86-64. Loads and
        // stores above use full local arrays and permit unaligned addresses.
        #[inline]
        #[target_feature(enable = $feature)]
        fn load_block(bytes: &[u8; 4096], block: usize) -> [V; 16] {
            assert!(block < 16);
            let mut out = [V::splat(0); 16];
            for col in 0..4 {
                // SAFETY: four full blocks at fixed chunk offsets, block < 16;
                // each load reads one 16-byte quarter of its 64-byte block.
                unsafe {
                    let a = _mm_loadu_si128(bytes.as_ptr().add(block * 64 + col * 16).cast());
                    let b =
                        _mm_loadu_si128(bytes.as_ptr().add(1024 + block * 64 + col * 16).cast());
                    let c =
                        _mm_loadu_si128(bytes.as_ptr().add(2048 + block * 64 + col * 16).cast());
                    let d =
                        _mm_loadu_si128(bytes.as_ptr().add(3072 + block * 64 + col * 16).cast());
                    let ab0 = _mm_unpacklo_epi32(a, b);
                    let ab1 = _mm_unpackhi_epi32(a, b);
                    let cd0 = _mm_unpacklo_epi32(c, d);
                    let cd1 = _mm_unpackhi_epi32(c, d);
                    out[col * 4] = V(_mm_unpacklo_epi64(ab0, cd0));
                    out[col * 4 + 1] = V(_mm_unpackhi_epi64(ab0, cd0));
                    out[col * 4 + 2] = V(_mm_unpacklo_epi64(ab1, cd1));
                    out[col * 4 + 3] = V(_mm_unpackhi_epi64(ab1, cd1));
                }
            }
            out
        }
    };
}
macro_rules! rotate_sse2 {
    ($value:expr, $right:ident, $left:ident) => {{
        if $right == 16 {
            Self(_mm_shufflehi_epi16::<0xb1>(_mm_shufflelo_epi16::<0xb1>(
                $value,
            )))
        } else {
            Self(_mm_or_si128(
                _mm_srli_epi32::<$right>($value),
                _mm_slli_epi32::<$left>($value),
            ))
        }
    }};
}
four_words!("sse2", rotate_sse2);
super::blake3_lanes::blake3_lanes!("sse2", 4);

use intrinsics::_mm_shuffle_epi32;
impl V {
    #[inline]
    fn cycle1(self) -> Self {
        unsafe { Self(_mm_shuffle_epi32::<0x39>(self.0)) }
    }
    #[inline]
    fn cycle2(self) -> Self {
        unsafe { Self(_mm_shuffle_epi32::<0x4e>(self.0)) }
    }
    #[inline]
    fn cycle3(self) -> Self {
        unsafe { Self(_mm_shuffle_epi32::<0x93>(self.0)) }
    }
}
super::blake3_lanes::blake3_single4!("sse2");

use intrinsics::{_mm_shufflehi_epi16, _mm_shufflelo_epi16};

#[cfg(target_arch = "x86_64")]
pub(super) mod avx512 {
    use super::intrinsics::{
        __m128i, _mm_add_epi32, _mm_and_si128, _mm_andnot_si128, _mm_loadu_si128, _mm_or_si128,
        _mm_ror_epi32, _mm_set1_epi32, _mm_storeu_si128, _mm_unpackhi_epi32, _mm_unpackhi_epi64,
        _mm_unpacklo_epi32, _mm_unpacklo_epi64, _mm_xor_si128,
    };
    macro_rules! rotate_native {
        ($value:expr, $right:ident, $left:ident) => {
            Self(_mm_ror_epi32::<$right>($value))
        };
    }
    four_words!("avx512f,avx512vl", rotate_native);
    super::super::blake3_lanes::blake3_lanes!("avx512f,avx512vl", 4);
}
