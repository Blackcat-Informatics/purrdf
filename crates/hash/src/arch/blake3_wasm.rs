// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Four independent BLAKE3 chunks on wasm SIMD128.
use core::arch::wasm32::{
    i32x4_shuffle, u32x4, u32x4_add, u32x4_extract_lane, u32x4_shl, u32x4_shr, u32x4_splat, v128,
    v128_bitselect, v128_load, v128_or, v128_xor,
};
#[derive(Clone, Copy)]
struct V(v128);
impl V {
    #[inline]
    fn splat(x: u32) -> Self {
        Self(u32x4_splat(x))
    }
    #[inline]
    fn load(x: [u32; 4]) -> Self {
        Self(u32x4(x[0], x[1], x[2], x[3]))
    }
    #[inline]
    fn array(self) -> [u32; 4] {
        [
            u32x4_extract_lane::<0>(self.0),
            u32x4_extract_lane::<1>(self.0),
            u32x4_extract_lane::<2>(self.0),
            u32x4_extract_lane::<3>(self.0),
        ]
    }
    #[inline]
    fn add(self, b: Self) -> Self {
        Self(u32x4_add(self.0, b.0))
    }
    #[inline]
    fn xor(self, b: Self) -> Self {
        Self(v128_xor(self.0, b.0))
    }
    #[inline]
    fn rotr<const R: i32, const L: i32>(self) -> Self {
        if R == 16 {
            return Self(i8x16_shuffle::<
                2,
                3,
                0,
                1,
                6,
                7,
                4,
                5,
                10,
                11,
                8,
                9,
                14,
                15,
                12,
                13,
            >(self.0, self.0));
        }
        if R == 8 {
            return Self(i8x16_shuffle::<
                1,
                2,
                3,
                0,
                5,
                6,
                7,
                4,
                9,
                10,
                11,
                8,
                13,
                14,
                15,
                12,
            >(self.0, self.0));
        }
        Self(v128_or(
            u32x4_shr(self.0, R as u32),
            u32x4_shl(self.0, L as u32),
        ))
    }
    #[inline]
    fn select(self, mask: Self, yes: Self) -> Self {
        Self(v128_bitselect(yes.0, self.0, mask.0))
    }
}
#[inline]
fn load_block(bytes: &[u8; 4096], block: usize) -> [V; 16] {
    assert!(block < 16);
    let mut out = [V::splat(0); 16];
    for col in 0..4 {
        // SAFETY: all four unaligned vector loads are inside their full block.
        unsafe {
            let a = v128_load(bytes.as_ptr().add(block * 64 + col * 16).cast());
            let b = v128_load(bytes.as_ptr().add(1024 + block * 64 + col * 16).cast());
            let c = v128_load(bytes.as_ptr().add(2048 + block * 64 + col * 16).cast());
            let d = v128_load(bytes.as_ptr().add(3072 + block * 64 + col * 16).cast());
            let ab0 = i32x4_shuffle::<0, 4, 1, 5>(a, b);
            let ab1 = i32x4_shuffle::<2, 6, 3, 7>(a, b);
            let cd0 = i32x4_shuffle::<0, 4, 1, 5>(c, d);
            let cd1 = i32x4_shuffle::<2, 6, 3, 7>(c, d);
            out[col * 4] = V(i32x4_shuffle::<0, 1, 4, 5>(ab0, cd0));
            out[col * 4 + 1] = V(i32x4_shuffle::<2, 3, 6, 7>(ab0, cd0));
            out[col * 4 + 2] = V(i32x4_shuffle::<0, 1, 4, 5>(ab1, cd1));
            out[col * 4 + 3] = V(i32x4_shuffle::<2, 3, 6, 7>(ab1, cd1));
        }
    }
    out
}
super::blake3_lanes::blake3_lanes!("simd128", 4);

impl V {
    #[inline]
    fn cycle1(self) -> Self {
        Self(i32x4_shuffle::<1, 2, 3, 0>(self.0, self.0))
    }
    #[inline]
    fn cycle2(self) -> Self {
        Self(i32x4_shuffle::<2, 3, 0, 1>(self.0, self.0))
    }
    #[inline]
    fn cycle3(self) -> Self {
        Self(i32x4_shuffle::<3, 0, 1, 2>(self.0, self.0))
    }
}
super::blake3_lanes::blake3_single4!("simd128");

use core::arch::wasm32::i8x16_shuffle;
