#![allow(
    clippy::inline_always,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Encoder fastpath: hot encode functions duplicated per CPU feature set so the
//! whole hot loop stays inside one `#[target_feature]` umbrella and SIMD/BMI2
//! intrinsics inline natively (no ABI barrier).
//!
//! All kernel functions are `unsafe fn`; the explicit inner `unsafe { }` blocks
//! around intrinsic calls are kept for safety documentation (this matches the
//! Rust 2024 recommended style enforced by `unsafe_op_in_unsafe_fn`). The
//! `unused_unsafe` lint sees them as redundant inside an `unsafe fn` body, so
//! we silence it at the module level rather than removing the documentation.
#![allow(unused_unsafe)]
//!
//! # Background
//!
//! In Rust, `#[target_feature(enable = "...")]` creates an ABI boundary: a
//! caller without the same feature set must call the function non-inline. In
//! C, the equivalent intrinsics inline via macros without restriction. That ABI
//! barrier is the dominant structural reason our encoder cannot match the
//! C zstd upstream zstd on per-block latency — every hot-path SIMD call becomes a
//! function call (~100 cycles overhead per BT walk iter, ~32-512 iters per
//! position, thousands of positions per block).
//!
//! # Strategy
//!
//! Each architecture-specific submodule (`neon`, `avx2_bmi2`, `sse42`,
//! `scalar`) holds a duplicate of the hot encode path, with every function in
//! the chain marked with the same `#[target_feature]`. Inside the module
//! everything inlines freely. The single ABI boundary is the dispatcher entry
//! point in this `mod.rs`, called once per encoder invocation.
//!
//! # Variant matrix
//!
//! - `scalar`: portable baseline, no SIMD assumptions. Used on unsupported
//!   targets and as fallback.
//! - `neon` (aarch64 only): NEON is part of the AArch64 baseline ISA but Rust
//!   still flags intrinsics like `vld1q_u8` with `#[target_feature(enable =
//!   "neon")]`, so we still need the umbrella attribute to let them inline.
//! - `sse42` (x86_64): SSE4.2 baseline for modern x86 CPUs (post-2008). Enables
//!   `_mm_crc32_*` hash mixing.
//! - `avx2_bmi2` (x86_64): adds AVX2 (32-byte vectors) and BMI2 (`pext`,
//!   `pdep`, `bzhi`) — common on Haswell+ (2013+).
//!
//! # Dispatcher
//!
//! [`select_kernel`] picks the best supported variant once per process via a
//! `OnceLock`. Encoder entry points call through the cached function pointer.
//! The single indirect call is amortized over the entire compression call,
//! and once inside the variant module the call graph is straight-line inlined.
//!
//! # Roadmap inside this module
//!
//! Week 1 (this commit): module scaffold + dispatcher skeleton.
//! Week 2a: match-length / common-prefix-len + `count_match_from_indices`.
//! Week 3a: BT walk (`bt_insert_step_no_rebase`,
//!   `bt_insert_and_collect_matches`) + HC chain walk.
//! Week 3b: optimal parser DP (`build_optimal_plan_impl` + price helpers).
//! Week 4: entropy encoders (FSE `encode_interleaved`, Huff0 `encode_stream`).
//! Week 5-6: bench vs `perf/pre-intrinsics-refactor-baseline` tag, profile,
//!   finalize.
//!
//! Refactor history and working rules for the multi-week PR #110 effort are
//! captured in the corresponding pull-request description.

// Scaffold-stage: the dispatcher and variant tags are wired up before any
// caller adopts them, so the dead-code lint would fire on every commit until
// Week 2a lands. Allow at module level and drop the allow as consumers come
// online.
#![allow(dead_code)]

pub(crate) mod scalar;

#[cfg(all(target_arch = "aarch64", target_endian = "little"))]
pub(crate) mod neon;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub(crate) mod sse42;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub(crate) mod avx2_bmi2;

#[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
pub(crate) mod simd128;

/// Runtime-selected variant tag. Picked once per process by [`select_kernel`].
///
/// Each variant corresponds to one of the submodules above and dictates which
/// implementation of the hot encoder path the dispatcher will call into.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum FastpathKernel {
    Scalar,
    #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
    Neon,
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    Sse42,
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    Avx2Bmi2,
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    Simd128,
}

/// Select the best supported variant for the running CPU. Cached after first
/// call; intended to be invoked once at the entry point of each encoder call
/// so the rest of the call graph can keep working with the resolved kernel
/// value as a const-foldable input.
#[inline]
pub(crate) fn select_kernel() -> FastpathKernel {
    {
        use std::sync::OnceLock;
        static CACHE: OnceLock<FastpathKernel> = OnceLock::new();
        *CACHE.get_or_init(detect_kernel_uncached)
    }
}

#[inline]
// On wasm32+simd128 the tier is resolved unconditionally to `Simd128` (no
// runtime CPUID), so the trailing `Scalar` fallback is statically unreachable
// there; it stays the reachable fallback on every other target.
#[cfg_attr(
    all(target_arch = "wasm32", target_feature = "simd128"),
    allow(unreachable_code)
)]
fn detect_kernel_uncached() -> FastpathKernel {
    // Each kernel's `hash_mix_u64` uses a hardware CRC instruction
    // (`_mm_crc32_u64` on x86, `__crc32d` on AArch64) for the upstream zstd-style
    // mix. The CRC ISA extension is NOT implied by the SIMD umbrella that
    // names the kernel:
    //   * `_mm_crc32_u64` is SSE4.2, NOT AVX2 — older Intel CPUs can ship
    //     AVX2+BMI2 without SSE4.2 in software (though all real shipping
    //     parts have both, compile-time `target_feature` enforcement
    //     doesn't propagate the implication).
    //   * `__crc32d` is the optional `crc` extension on AArch64, separate
    //     from the NEON baseline.
    //
    // Both kernels must therefore gate on the CRC support explicitly at
    // runtime (std path) and at compile time (no_std path). Without the
    // CRC ISA available the hash mix would trap with an illegal
    // instruction, so we fall back to a SIMD-less kernel that uses the
    // scalar multiply-only mix.
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        if std::is_x86_feature_detected!("avx2")
            && std::is_x86_feature_detected!("bmi2")
            && std::is_x86_feature_detected!("sse4.2")
        {
            return FastpathKernel::Avx2Bmi2;
        }
        if std::is_x86_feature_detected!("sse4.2") {
            return FastpathKernel::Sse42;
        }
    }
    #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
    {
        // NEON is part of the AArch64 baseline, but the `crc` extension is
        // optional. Both must be present before selecting the NEON kernel
        // because its `hash_mix_u64` calls `__crc32d` directly.
        if std::arch::is_aarch64_feature_detected!("neon")
            && std::arch::is_aarch64_feature_detected!("crc")
        {
            return FastpathKernel::Neon;
        }
    }

    // wasm SIMD is a compile-time feature (no runtime detection), so the
    // `+simd128` payload selects the SIMD kernel and the scalar payload never
    // compiles the variant. `hash_mix_u64` routes through the scalar mixer
    // (wasm has no CRC), so there's no extra feature to gate on here.
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    {
        return FastpathKernel::Simd128;
    }

    FastpathKernel::Scalar
}

/// Public entry point for match-length probes — used during migration as the
/// shim that callers in `match_generator` adopt without yet being themselves
/// inside the `#[target_feature]` umbrella. Once the BT walk methods are
/// lifted into the umbrella (Week 3a) they will call the per-kernel symbol
/// directly so the entire inner loop inlines.
#[inline]
pub(crate) fn dispatch_count_match_from_indices(
    concat: &[u8],
    current_idx: usize,
    candidate_idx: usize,
    tail_limit: usize,
    seed_len: usize,
) -> usize {
    match select_kernel() {
        FastpathKernel::Scalar => unsafe {
            scalar::count_match_from_indices(
                concat,
                current_idx,
                candidate_idx,
                tail_limit,
                seed_len,
            )
        },
        #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
        FastpathKernel::Neon => unsafe {
            neon::count_match_from_indices(concat, current_idx, candidate_idx, tail_limit, seed_len)
        },
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        FastpathKernel::Sse42 => unsafe {
            sse42::count_match_from_indices(
                concat,
                current_idx,
                candidate_idx,
                tail_limit,
                seed_len,
            )
        },
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        FastpathKernel::Avx2Bmi2 => unsafe {
            avx2_bmi2::count_match_from_indices(
                concat,
                current_idx,
                candidate_idx,
                tail_limit,
                seed_len,
            )
        },
        #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
        FastpathKernel::Simd128 => unsafe {
            simd128::count_match_from_indices(
                concat,
                current_idx,
                candidate_idx,
                tail_limit,
                seed_len,
            )
        },
    }
}

/// Hash-mix dispatch that takes the resolved [`FastpathKernel`] by value, so
/// the caller can cache it once per matcher / encoder lifetime instead of
/// hitting the `OnceLock` atomic on every call.
///
/// Critical for the default-level Dfast hot path: `hash_index` runs once per
/// input byte. The previous per-call `dispatch_hash_mix_u64` shape was a
/// measurable regression versus storing the kernel on the matcher (the old
/// pre-refactor pattern).
#[inline(always)]
pub(crate) fn hash_mix_u64_with_kernel(kernel: FastpathKernel, value: u64) -> u64 {
    match kernel {
        FastpathKernel::Scalar => scalar::hash_mix_u64(value),
        #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
        FastpathKernel::Neon => unsafe { neon::hash_mix_u64(value) },
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        FastpathKernel::Sse42 => unsafe { sse42::hash_mix_u64(value) },
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        FastpathKernel::Avx2Bmi2 => unsafe { avx2_bmi2::hash_mix_u64(value) },
        #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
        FastpathKernel::Simd128 => simd128::hash_mix_u64(value),
    }
}

/// Hash-mix dispatch that resolves the kernel via [`select_kernel`] on every
/// call. Suitable for cold paths or callers that only mix a handful of values
/// per encoder lifetime. Hot loops should call [`hash_mix_u64_with_kernel`]
/// with a cached kernel instead.
#[inline]
pub(crate) fn dispatch_hash_mix_u64(value: u64) -> u64 {
    hash_mix_u64_with_kernel(select_kernel(), value)
}

/// Public entry point for raw-pointer prefix-length scans (BT byte compare,
/// repcode extend, etc.). Same migration shim semantics as
/// [`dispatch_count_match_from_indices`].
///
/// # Safety
/// `lhs` / `rhs` must each point to at least `max` initialized bytes.
#[inline]
pub(crate) unsafe fn dispatch_common_prefix_len_ptr(
    lhs: *const u8,
    rhs: *const u8,
    max: usize,
) -> usize {
    // Cold-path shim: resolves the kernel via `select_kernel()` on every call.
    // Hot match-finder loops resolve the kernel once per block and call
    // [`dispatch_common_prefix_len_ptr_with_kernel`] directly.
    unsafe { dispatch_common_prefix_len_ptr_with_kernel(select_kernel(), lhs, rhs, max) }
}

/// Prefix-length scan against an already-resolved [`FastpathKernel`], so a hot
/// loop pays the kernel-select once per block (caller-cached) instead of the
/// `OnceLock` atomic + branch on every byte-compare.
///
/// # Safety
/// `lhs` / `rhs` must each point to at least `max` initialized bytes.
#[inline(always)]
pub(crate) unsafe fn dispatch_common_prefix_len_ptr_with_kernel(
    kernel: FastpathKernel,
    lhs: *const u8,
    rhs: *const u8,
    max: usize,
) -> usize {
    match kernel {
        FastpathKernel::Scalar => unsafe { scalar::common_prefix_len_ptr(lhs, rhs, max) },
        #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
        FastpathKernel::Neon => unsafe { neon::common_prefix_len_ptr(lhs, rhs, max) },
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        FastpathKernel::Sse42 => unsafe { sse42::common_prefix_len_ptr(lhs, rhs, max) },
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        FastpathKernel::Avx2Bmi2 => unsafe { avx2_bmi2::common_prefix_len_ptr(lhs, rhs, max) },
        #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
        FastpathKernel::Simd128 => unsafe { simd128::common_prefix_len_ptr(lhs, rhs, max) },
    }
}
