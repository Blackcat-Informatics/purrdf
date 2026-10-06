// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Four independent BLAKE3 chunks on little-endian AArch64 NEON.
use core::arch::aarch64::{
    uint32x4_t, vaddq_u32, vbslq_u32, vdupq_n_u32, veorq_u32, vld1q_u32, vreinterpretq_u32_u64,
    vreinterpretq_u64_u32, vshlq_n_u32, vst1q_u32, vzip1q_u32, vzip1q_u64, vzip2q_u32, vzip2q_u64,
};
#[derive(Clone, Copy)]
struct V(uint32x4_t);
impl V {
    #[inline]
    fn splat(x: u32) -> Self {
        unsafe { Self(vdupq_n_u32(x)) }
    }
    #[inline]
    fn load(x: [u32; 4]) -> Self {
        unsafe { Self(vld1q_u32(x.as_ptr())) }
    }
    #[inline]
    fn array(self) -> [u32; 4] {
        let mut a = [0; 4];
        unsafe {
            vst1q_u32(a.as_mut_ptr(), self.0);
        }
        a
    }
    #[inline]
    fn add(self, b: Self) -> Self {
        unsafe { Self(vaddq_u32(self.0, b.0)) }
    }
    #[inline]
    fn xor(self, b: Self) -> Self {
        unsafe { Self(veorq_u32(self.0, b.0)) }
    }
    #[inline]
    fn rotr<const R: i32, const L: i32>(self) -> Self {
        // SAFETY: the module requires NEON. These rotate amounts are the
        // four fixed BLAKE3 constants, within the immediate operand ranges.
        unsafe {
            if R == 16 {
                return Self(vreinterpretq_u32_u16(vrev32q_u16(vreinterpretq_u16_u32(
                    self.0,
                ))));
            }
            Self(vsriq_n_u32::<R>(vshlq_n_u32::<L>(self.0), self.0))
        }
    }
    #[inline]
    fn select(self, mask: Self, yes: Self) -> Self {
        unsafe { Self(vbslq_u32(mask.0, yes.0, self.0)) }
    }
}
// SAFETY for these primitive wrappers: this module is compiled only with
// NEON enabled; local array loads/stores access exactly four u32 words.
#[inline]
fn load_block(bytes: &[u8; 4096], block: usize) -> [V; 16] {
    assert!(block < 16);
    let mut out = [V::splat(0); 16];
    for col in 0..4 {
        // SAFETY: the vectors lie within four full blocks. NEON permits
        // unaligned accesses; interpreting words is valid on little-endian Arm.
        unsafe {
            let a = vld1q_u32(bytes.as_ptr().add(block * 64 + col * 16).cast());
            let b = vld1q_u32(bytes.as_ptr().add(1024 + block * 64 + col * 16).cast());
            let c = vld1q_u32(bytes.as_ptr().add(2048 + block * 64 + col * 16).cast());
            let d = vld1q_u32(bytes.as_ptr().add(3072 + block * 64 + col * 16).cast());
            let ab0 = vreinterpretq_u64_u32(vzip1q_u32(a, b));
            let ab1 = vreinterpretq_u64_u32(vzip2q_u32(a, b));
            let cd0 = vreinterpretq_u64_u32(vzip1q_u32(c, d));
            let cd1 = vreinterpretq_u64_u32(vzip2q_u32(c, d));
            out[col * 4] = V(vreinterpretq_u32_u64(vzip1q_u64(ab0, cd0)));
            out[col * 4 + 1] = V(vreinterpretq_u32_u64(vzip2q_u64(ab0, cd0)));
            out[col * 4 + 2] = V(vreinterpretq_u32_u64(vzip1q_u64(ab1, cd1)));
            out[col * 4 + 3] = V(vreinterpretq_u32_u64(vzip2q_u64(ab1, cd1)));
        }
    }
    out
}
super::blake3_lanes::blake3_lanes!("neon", 4);

use core::arch::aarch64::vextq_u32;
impl V {
    #[inline]
    fn cycle1(self) -> Self {
        unsafe { Self(vextq_u32::<1>(self.0, self.0)) }
    }
    #[inline]
    fn cycle2(self) -> Self {
        unsafe { Self(vextq_u32::<2>(self.0, self.0)) }
    }
    #[inline]
    fn cycle3(self) -> Self {
        unsafe { Self(vextq_u32::<3>(self.0, self.0)) }
    }
}
super::blake3_lanes::blake3_single4!("neon");

use core::arch::aarch64::{vreinterpretq_u16_u32, vreinterpretq_u32_u16, vrev32q_u16, vsriq_n_u32};
