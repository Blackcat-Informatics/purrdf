#![allow(
    clippy::inline_always,
    clippy::ptr_as_ptr,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Scalar fastpath variant — portable baseline used on targets without SIMD
//! intrinsics or when feature detection picks the fallback. Also provides the
//! shared scalar tail used by the SIMD variants once the vector loop has
//! consumed all whole chunks.
//!
//! No `#[target_feature]` attributes here: every function uses portable Rust
//! and must compile on any supported target.

/// Position of the first mismatching byte inside an 8-byte XOR diff. On
/// little-endian targets the low byte corresponds to the lowest address, so
/// `trailing_zeros / 8` is the index of the first non-equal byte.
#[inline(always)]
#[cfg(target_endian = "little")]
pub(crate) const fn mismatch_byte_index(diff: usize) -> usize {
    diff.trailing_zeros() as usize / 8
}

#[inline(always)]
#[cfg(target_endian = "big")]
pub(crate) const fn mismatch_byte_index(diff: usize) -> usize {
    diff.leading_zeros() as usize / 8
}

/// The first `XXH64` prime, used as Zstandard's match-hash multiplier on every path.
pub(crate) use purrdf_hash::xxh64::PRIME_1 as HASH_MIX_PRIME;

/// Pure scalar `hash_mix_u64` — single multiplication. Used by
/// `FastpathKernel::Scalar` and as the fallback when no CRC instruction is
/// available.
#[inline(always)]
pub(crate) fn hash_mix_u64(value: u64) -> u64 {
    value.wrapping_mul(HASH_MIX_PRIME)
}

/// Scalar prefix-length scan starting from `off` until `max`, using
/// word-sized XOR chunks then a byte tail. Callable from any target_feature
/// context — no SIMD intrinsics involved.
///
/// # Safety
/// `lhs` and `rhs` must point to at least `max` initialized bytes each.
#[inline(always)]
pub(crate) unsafe fn common_prefix_len_scalar_ptr(
    lhs: *const u8,
    rhs: *const u8,
    mut off: usize,
    max: usize,
) -> usize {
    let chunk = size_of::<usize>();
    while off + chunk <= max {
        let lhs_word = unsafe { core::ptr::read_unaligned(lhs.add(off) as *const usize) };
        let rhs_word = unsafe { core::ptr::read_unaligned(rhs.add(off) as *const usize) };
        let diff = lhs_word ^ rhs_word;
        if diff != 0 {
            return off + mismatch_byte_index(diff);
        }
        off += chunk;
    }
    while off < max {
        if unsafe { *lhs.add(off) != *rhs.add(off) } {
            break;
        }
        off += 1;
    }
    off
}

/// Scalar-only common-prefix-length probe used by `FastpathKernel::Scalar`.
///
/// # Safety
/// `lhs` and `rhs` must point to at least `max` initialized bytes each.
#[inline(always)]
pub(crate) unsafe fn common_prefix_len_ptr(lhs: *const u8, rhs: *const u8, max: usize) -> usize {
    unsafe { common_prefix_len_scalar_ptr(lhs, rhs, 0, max) }
}

/// Count a match from the scalar binary-tree walker. Little-endian `aarch64`
/// uses the NEON walker instead.
///
/// # Safety
/// Caller-side BT-walk invariants ensure
/// `candidate_idx + tail_limit ≤ concat.len()` and
/// `current_idx + tail_limit ≤ concat.len()`.
#[cfg(not(all(target_arch = "aarch64", target_endian = "little")))]
#[inline(always)]
pub(crate) unsafe fn count_match_from_indices(
    concat: &[u8],
    current_idx: usize,
    candidate_idx: usize,
    tail_limit: usize,
    seed_len: usize,
) -> usize {
    // SAFETY: this entry's index bounds cover the selected scalar prefix probe.
    unsafe {
        count_match_with(
            concat,
            current_idx,
            candidate_idx,
            tail_limit,
            seed_len,
            |lhs, rhs, remaining| common_prefix_len_ptr(lhs, rhs, remaining),
        )
    }
}

/// Compose the leading word probe, selected vector kernel and portable tail.
///
/// # Safety
/// Both pointers are readable for `max` bytes. The selected vector callback
/// upholds its ISA contract and returns the equal prefix it consumed.
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    all(target_arch = "aarch64", target_endian = "little"),
    all(target_arch = "wasm32", target_feature = "simd128")
))]
#[inline(always)]
pub(crate) unsafe fn common_prefix_len_with(
    lhs: *const u8,
    rhs: *const u8,
    max: usize,
    vector_prefix: impl FnOnce(*const u8, *const u8, usize) -> usize,
) -> usize {
    let chunk = size_of::<usize>();
    if chunk <= max {
        // SAFETY: the caller provides at least max initialized bytes.
        let lhs_word = unsafe { core::ptr::read_unaligned(lhs.cast::<usize>()) };
        let rhs_word = unsafe { core::ptr::read_unaligned(rhs.cast::<usize>()) };
        let diff = lhs_word ^ rhs_word;
        if diff != 0 {
            return mismatch_byte_index(diff);
        }
    }
    let off = vector_prefix(lhs, rhs, max);
    // SAFETY: the callback consumed an equal prefix within the original range.
    unsafe { common_prefix_len_scalar_ptr(lhs, rhs, off, max) }
}

/// Apply the common seed clamp and index geometry before a selected prefix probe.
///
/// # Safety
/// Both indices plus `tail_limit` are within `concat`. The callback upholds
/// its ISA contract and probes no more than the supplied remaining range.
#[inline(always)]
pub(crate) unsafe fn count_match_with(
    concat: &[u8],
    current_idx: usize,
    candidate_idx: usize,
    tail_limit: usize,
    seed_len: usize,
    prefix: impl FnOnce(*const u8, *const u8, usize) -> usize,
) -> usize {
    let seed = seed_len.min(tail_limit);
    if seed == tail_limit {
        return seed;
    }
    let remaining = tail_limit - seed;
    let base = concat.as_ptr();
    // SAFETY: the seed is at most tail_limit, covered by the caller's bounds.
    let lhs = unsafe { base.add(candidate_idx + seed) };
    let rhs = unsafe { base.add(current_idx + seed) };
    seed + prefix(lhs, rhs, remaining)
}
