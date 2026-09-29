// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Branch-free selection masks and secret wiping.
//!
//! A mask is `0` or `u64::MAX`. Masks are produced arithmetically and passed
//! through [`black_box`] so the optimizer cannot see a boolean behind them and
//! turn a masked select back into a branch on secret data.

use core::hint::black_box;
use core::sync::atomic::{Ordering, compiler_fence};

/// `u64::MAX` when `a == b`, else `0`, with no data-dependent branch.
pub(crate) fn eq_mask(a: u64, b: u64) -> u64 {
    let x = a ^ b;
    // (x | -x) has its top bit set exactly when x != 0.
    let nonzero = (x | x.wrapping_neg()) >> 63;
    black_box(nonzero.wrapping_sub(1))
}

/// `u64::MAX` when `bit` is 1, `0` when it is 0.
pub(crate) fn bit_mask(bit: u64) -> u64 {
    black_box(0u64.wrapping_sub(bit & 1))
}

/// Byte-string equality that reads every byte before deciding.
pub(crate) fn bytes_eq(a: &[u8; 32], b: &[u8; 32]) -> bool {
    let diff = a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y));
    black_box(diff) == 0
}

/// Overwrite `slots` with their default value in a way the optimizer must
/// keep: the cleared slice is handed to [`black_box`] (so the stores are
/// observed) and a compiler fence stops them being sunk past the caller's
/// deallocation. This is the wipe every secret in the crate goes through on
/// drop; it needs no `unsafe` volatile store.
pub(crate) fn wipe<T: Copy + Default>(slots: &mut [T]) {
    slots.fill(T::default());
    black_box(&mut *slots);
    compiler_fence(Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::{bit_mask, bytes_eq, eq_mask, wipe};

    #[test]
    fn masks_are_all_or_nothing() {
        assert_eq!(eq_mask(5, 5), u64::MAX);
        assert_eq!(eq_mask(5, 6), 0);
        assert_eq!(eq_mask(0, u64::MAX), 0);
        assert_eq!(eq_mask(u64::MAX, u64::MAX), u64::MAX);
        assert_eq!(bit_mask(1), u64::MAX);
        assert_eq!(bit_mask(0), 0);
    }

    #[test]
    fn byte_equality_sees_every_position() {
        let a = [7u8; 32];
        for at in 0..32 {
            let mut b = a;
            b[at] ^= 0x80;
            assert!(!bytes_eq(&a, &b));
        }
        assert!(bytes_eq(&a, &a.clone()));
    }

    #[test]
    fn wipe_clears_every_slot() {
        let mut secret = [0xa5u8; 32];
        wipe(&mut secret);
        assert_eq!(secret, [0u8; 32]);
        let mut digits = [-3i8; 64];
        wipe(&mut digits);
        assert_eq!(digits, [0i8; 64]);
    }
}
