#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! WebAssembly `simd128` fastpath variant. Mirrors [`super::neon`] (the other
//! 128-bit-SIMD tier) but on `core::arch::wasm32` `v128` intrinsics. Every
//! hot-path function carries `#[target_feature(enable = "simd128")]` so the
//! intrinsics inline into the caller's loop instead of crossing the
//! target_feature ABI barrier as a non-inlinable call.
//!
//! wasm has no CRC instruction, so `hash_mix_u64` falls back to the portable
//! scalar mixer (a single 64-bit mix has no SIMD speedup anyway); the win here
//! is the vectorized `common_prefix_len` match-length scan.

#![cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
#![allow(dead_code)]

use core::arch::wasm32::{i8x16_bitmask, u8x16_eq, v128, v128_load};

use super::scalar;

pub(crate) const KERNEL_TAG: &str = "simd128";

/// wasm has no CRC unit; route the single-lane mix through the portable scalar
/// mixer so it stays bit-identical with every other tier's `hash_mix_u64`.
#[inline]
pub(crate) fn hash_mix_u64(value: u64) -> u64 {
    scalar::hash_mix_u64(value)
}

/// 16-byte `v128` prefix-length probe. Returns the number of leading equal
/// bytes in whole 16-byte chunks; the caller handles the scalar tail.
///
/// # Safety
/// `lhs` / `rhs` must point to at least `max` initialized bytes. `simd128` must
/// be available — enforced by the `target_feature` attribute (and, on wasm, by
/// the compile-time `+simd128` payload the dispatcher selects).
#[target_feature(enable = "simd128")]
#[inline]
pub(crate) unsafe fn prefix_len_simd(lhs: *const u8, rhs: *const u8, max: usize) -> usize {
    let mut off = 0usize;
    while off + 16 <= max {
        // SAFETY: `off + 16 <= max` and the caller guarantees `max` initialized
        // bytes behind each pointer, so both 16-byte loads stay in bounds.
        let a: v128 = unsafe { v128_load(lhs.add(off) as *const v128) };
        let b: v128 = unsafe { v128_load(rhs.add(off) as *const v128) };
        // `u8x16_eq` sets each lane to 0xFF on equality, 0x00 otherwise;
        // `i8x16_bitmask` then folds the per-lane high bit into a u16, so bit
        // `i` is set iff byte `i` matched. All 16 equal => 0xFFFF.
        let mask = i8x16_bitmask(u8x16_eq(a, b));
        if mask != 0xFFFF {
            // First mismatching byte = first cleared bit = trailing zeros of
            // the inverted mask.
            return off + (!mask).trailing_zeros() as usize;
        }
        off += 16;
    }
    off
}

/// `simd128` variant of `common_prefix_len_ptr`: vector loop then the shared
/// scalar tail. `target_feature(enable = "simd128")` so same-umbrella callers
/// inline it without an ABI barrier.
///
/// # Safety
/// `lhs` / `rhs` must point to at least `max` initialized bytes.
#[target_feature(enable = "simd128")]
#[inline]
pub(crate) unsafe fn common_prefix_len_ptr(lhs: *const u8, rhs: *const u8, max: usize) -> usize {
    // Leading scalar word probe (mirrors upstream C `ZSTD_count`'s first
    // `MEM_readST` check): a prefix that diverges within the first 8 bytes — the
    // common case on BT-tree node compares, where each node extends the seed by
    // only a short run — returns on one 8-byte read + count-trailing-zeros,
    // skipping the vector load. Longer matches fall through to the vector loop.
    let chunk = size_of::<usize>();
    if chunk <= max {
        let lhs_word = unsafe { core::ptr::read_unaligned(lhs.cast::<usize>()) };
        let rhs_word = unsafe { core::ptr::read_unaligned(rhs.cast::<usize>()) };
        let diff = lhs_word ^ rhs_word;
        if diff != 0 {
            return scalar::mismatch_byte_index(diff);
        }
    }
    let off = unsafe { prefix_len_simd(lhs, rhs, max) };
    unsafe { scalar::common_prefix_len_scalar_ptr(lhs, rhs, off, max) }
}

/// `simd128` variant of `count_match_from_indices` — the BT-walk match-length
/// probe entry point. Same invariants as the scalar variant, under the
/// `simd128` umbrella.
///
/// # Safety
/// BT walk invariants: `candidate_idx + tail_limit <= concat.len()` and
/// `current_idx + tail_limit <= concat.len()`.
#[target_feature(enable = "simd128")]
#[inline]
pub(crate) unsafe fn count_match_from_indices(
    concat: &[u8],
    current_idx: usize,
    candidate_idx: usize,
    tail_limit: usize,
    seed_len: usize,
) -> usize {
    let seed = seed_len.min(tail_limit);
    if seed == tail_limit {
        return seed;
    }
    let remaining = tail_limit - seed;
    let base = concat.as_ptr();
    let lhs = unsafe { base.add(candidate_idx + seed) };
    let rhs = unsafe { base.add(current_idx + seed) };
    let extra = unsafe { common_prefix_len_ptr(lhs, rhs, remaining) };
    seed + extra
}
