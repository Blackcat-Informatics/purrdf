// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The folded multiply: the low and high halves of a 128-bit product,
//! XOR-ed together.
//!
//! Two formulations compute the same function. [`fold_wide`] multiplies in
//! `u128`, which a 64-bit target does with one widening multiply.
//! [`fold_halves`] builds the product from four 32×32→64 partial products,
//! which a 32-bit target (i686, wasm32) does natively instead of calling a
//! 128-bit multiply routine. [`fold`] picks one per target; both are
//! compiled everywhere so a test can hold them equal on every target.

/// `lo64(a·b) ⊕ hi64(a·b)` through `u128`.
#[cfg_attr(
    not(target_pointer_width = "64"),
    allow(
        dead_code,
        reason = "compiled on every target so the tests can compare the two"
    )
)]
#[inline]
pub(crate) const fn fold_wide(a: u64, b: u64) -> u64 {
    let product = (a as u128) * (b as u128);
    (product as u64) ^ ((product >> 64) as u64)
}

/// `lo64(a·b) ⊕ hi64(a·b)` from 32-bit halves.
///
/// With `a = a1·2^32 + a0` and `b = b1·2^32 + b0`, the product is
/// `p11·2^64 + (p01 + p10)·2^32 + p00`. The low word takes `p00`'s low half
/// and, in its high half, the low 32 bits of `middle = hi(p00) + lo(p01) +
/// lo(p10)` (at most `3·(2^32 − 1)`, so it cannot overflow). The high word is
/// `p11 + hi(p01) + hi(p10) + hi(middle)`, which is exact because the whole
/// product is below `2^128`.
#[cfg_attr(
    target_pointer_width = "64",
    allow(
        dead_code,
        reason = "compiled on every target so the tests can compare the two"
    )
)]
#[inline]
pub(crate) const fn fold_halves(a: u64, b: u64) -> u64 {
    const LOW: u64 = 0xFFFF_FFFF;
    let (a0, a1) = (a & LOW, a >> 32);
    let (b0, b1) = (b & LOW, b >> 32);
    let p00 = a0 * b0;
    let p01 = a0 * b1;
    let p10 = a1 * b0;
    let p11 = a1 * b1;
    let middle = (p00 >> 32) + (p01 & LOW) + (p10 & LOW);
    let low = (p00 & LOW) | (middle << 32);
    let high = p11 + (p01 >> 32) + (p10 >> 32) + (middle >> 32);
    low ^ high
}

/// The folded multiply this target computes fastest.
#[cfg(target_pointer_width = "64")]
#[inline]
pub(crate) const fn fold(a: u64, b: u64) -> u64 {
    fold_wide(a, b)
}

/// The folded multiply this target computes fastest.
#[cfg(not(target_pointer_width = "64"))]
#[inline]
pub(crate) const fn fold(a: u64, b: u64) -> u64 {
    fold_halves(a, b)
}

#[cfg(test)]
mod tests {
    use super::{fold_halves, fold_wide};
    use crate::mix::splitmix64_next;

    #[test]
    fn both_formulations_agree() {
        let edges = [
            0,
            1,
            2,
            0xFFFF_FFFF,
            0x1_0000_0000,
            0xFFFF_FFFF_0000_0000,
            0x8000_0000_0000_0000,
            u64::MAX - 1,
            u64::MAX,
        ];
        for a in edges {
            for b in edges {
                assert_eq!(fold_wide(a, b), fold_halves(a, b), "{a:#x} · {b:#x}");
            }
        }
        let mut state = 0x666f_6c64;
        for _ in 0..1 << 20 {
            let (a, b) = (splitmix64_next(&mut state), splitmix64_next(&mut state));
            assert_eq!(fold_wide(a, b), fold_halves(a, b), "{a:#x} · {b:#x}");
        }
    }

    #[test]
    fn folds_the_full_product() {
        // (2^64 − 1)² = 2^128 − 2^65 + 1: low word 1, high word 2^64 − 2.
        assert_eq!(fold_wide(u64::MAX, u64::MAX), 1 ^ (u64::MAX - 1));
        // 2^32 · 2^32 = 2^64: low word 0, high word 1.
        assert_eq!(fold_halves(1 << 32, 1 << 32), 1);
    }
}
