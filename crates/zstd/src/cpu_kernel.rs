#![allow(
    clippy::cast_lossless,
    clippy::empty_line_after_doc_comments,
    clippy::fn_params_excessive_bools,
    clippy::inline_always,
    clippy::use_self,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! CPU kernel dispatch — single detect+match at the dispatch site,
//! propagated through the inner pipeline as a generic parameter so
//! leaf hot-path code monomorphises against the chosen kernel.
//!
//! Architecture rationale: per-subsystem
//! dispatch scatters the choice across HUF / FSE / SIMD-copy
//! independently and pays the cost N times per call. Lifting the
//! dispatch to the outermost feasible call site collapses it to one
//! detect there; the inner leaf-hot-path ops then route through
//! `K::method` calls on the chosen kernel zero-sized type.
//!
//! Current wiring: the only active dispatch site
//! is `decoding::literals_section_decoder::decompress_literals`,
//! which `match`es `detect_cpu_kernel()` and routes into per-K
//! `decompress_literals_*` `#[target_feature]` wrappers. The full
//! pipeline-wide propagation envisioned in the issue (FrameDecoder /
//! FrameCompressor entry, sequence executor, match copy) is
//! incremental; subsequent tiers extend the dispatch surface without
//! changing this trait or the kernel ZSTs.
//!
//! Structure code (block loop, FCS check, offset history, repeat
//! semantics) stays single-impl and only carries `K` as a phantom on
//! the outer function. Monomorphisation specialises ONLY the bodies
//! that actually differ per ISA — `mask_lower_bits`, `huf_burst`,
//! `copy_chunk`, etc.

use std::sync::OnceLock;

/// Trait covering the leaf hot-path operations whose bodies differ
/// per ISA. Implementations are ZSTs; the trait is `Copy` so it can
/// be `Default`-constructed at each call site without runtime cost.
///
/// New methods land here ONLY when their codegen genuinely differs
/// per kernel (BMI2 intrinsic vs scalar shift, AVX2 256-bit move vs
/// SSE2 128-bit move, etc.). Structure ops that have one canonical
/// implementation must NOT be on this trait — they stay on the
/// existing decoder / encoder types.
// Public (rather than `pub(crate)`) because `BitReaderReversed` is
// generic over `K: CpuKernel = ScalarKernel` and is re-exported via
// the `bench_internals`-gated `testing` module; under that feature
// the visibility of every type that appears in `BitReaderReversed`'s
// bounds (the trait + the default kernel) must match the type's own
// visibility, otherwise rustc rejects with `private_bounds` /
// `private_interfaces`. The trait surface stays narrow on stable
// crate users: nothing outside `bench_internals` constructs a
// non-Scalar kernel directly.
pub(crate) trait CpuKernel: Copy + 'static {
    /// Mask the low `n` bits of `value`, returning the remaining
    /// high bits zeroed. The FSE bitstream hot path fires this 3×
    /// per decoded sequence; on BMI2-capable hardware this maps to
    /// a single `_bzhi_u64` instruction, otherwise to a scalar
    /// `u64::MAX >> (64 - n)` shift + mask.
    ///
    /// Precondition: `n <= 64`. Behaviour for `n == 0` is "return 0";
    /// behaviour for `n > 64` is unspecified — callers MUST uphold
    /// the bound. The test-only `mask_lower_bits` helper in
    /// `bit_reader_reverse.rs` debug-asserts the bound for its
    /// unit tests, but production callers (FSE / HUF hot paths)
    /// derive `n` from `accuracy_log` / `max_num_bits` which the
    /// per-stream table builders pin to `n <= MAX_*_BITS` at
    /// construction time; no per-call wrapper assert runs.
    fn mask_lower_bits(value: u64, n: u8) -> u64;
}

/// Scalar fallback — portable, no SIMD or BMI2 intrinsics. Selected
/// when no x86 or aarch64 feature is detected at runtime.
#[derive(Copy, Clone, Default)]
pub(crate) struct ScalarKernel;

impl CpuKernel for ScalarKernel {
    #[inline(always)]
    fn mask_lower_bits(value: u64, n: u8) -> u64 {
        // `checked_shr` returns `None` for shift counts >= 64, which
        // happens exactly when `n == 0` (`64 - 0 = 64`). Mapping
        // both that case and the invalid `n > 64` underflow to 0
        // gives the mathematically-correct empty mask for n=0 and
        // a safe-ish fallback for the invalid range.
        let mask = u64::MAX
            .checked_shr(64u32.wrapping_sub(n as u32))
            .unwrap_or(0);
        value & mask
    }
}

// The SSE2 tier exists in `CpuKernelTag` (it carries the 128-bit copy-chunk
// choice for the unified copy dispatch) but needs no `CpuKernel` ZST yet: the
// only trait method, `mask_lower_bits`, has no SSE2-specific form (SSE2 has no
// bit-extract), so the Sse2 tag routes through the scalar bodies for the
// FSE/HUF paths. A dedicated `Sse2Kernel` lands when `copy_chunk` moves onto
// the trait.

/// x86_64 BMI2-only kernel: `_bzhi_u64` for mask_lower_bits. Selected
/// when the CPU has BMI2 but not the AVX2 SIMD width to upgrade to
/// the Avx2 kernel. Treated as a stepping stone between Sse2 and
/// Avx2 on hardware that has BMI2 but not AVX2 (rare in practice but
/// matches upstream zstd's gating).
#[cfg(target_arch = "x86_64")]
#[derive(Copy, Clone, Default)]
pub(crate) struct Bmi2Kernel;

#[cfg(target_arch = "x86_64")]
impl CpuKernel for Bmi2Kernel {
    #[inline(always)]
    fn mask_lower_bits(value: u64, n: u8) -> u64 {
        // SAFETY: this kernel ZST is only reachable via the
        // `match detect_cpu_kernel() { CpuKernelTag::Bmi2 => ... }`
        // dispatch arms at decoder entry sites, all of which fire only
        // after `detect_cpu_kernel` confirmed BMI2 is available on the
        // running CPU.
        unsafe { mask_lower_bits_bmi2_impl(value, n) }
    }
}

/// x86_64 AVX2 + BMI2 kernel (x86-64-v3 baseline). The common modern
/// x86 case — most CPUs released since 2013 (Haswell) have AVX2+BMI2.
/// Uses `_bzhi_u64` for mask ops; future trait methods will use AVX2
/// 256-bit moves for `copy_chunk` and pext for HUF burst.
#[cfg(target_arch = "x86_64")]
#[derive(Copy, Clone, Default)]
pub(crate) struct Avx2Kernel;

#[cfg(target_arch = "x86_64")]
impl CpuKernel for Avx2Kernel {
    #[inline(always)]
    fn mask_lower_bits(value: u64, n: u8) -> u64 {
        // SAFETY: Avx2Kernel is selected only after runtime detect
        // confirmed both AVX2 and BMI2 — `_bzhi_u64` is callable.
        unsafe { mask_lower_bits_bmi2_impl(value, n) }
    }
}

/// aarch64 NEON baseline kernel. Used on all aarch64 hardware that
/// exposes NEON (effectively universal on the supported targets).
///
/// `#[allow(dead_code)]`: scaffolding for the future aarch64 dispatch
/// arm in `decompress_literals` / `decode_and_execute_sequences`.
/// The struct + trait impl land first so the dispatch wiring can be
/// added incrementally without churning the CpuKernel surface; until
/// the dispatch arm uses it the type is reachable only as a phantom.
#[cfg(target_arch = "aarch64")]
#[allow(dead_code)]
#[derive(Copy, Clone, Default)]
pub(crate) struct NeonKernel;

#[cfg(target_arch = "aarch64")]
impl CpuKernel for NeonKernel {
    #[inline(always)]
    fn mask_lower_bits(value: u64, n: u8) -> u64 {
        // aarch64 has no BMI2 equivalent that improves on the scalar
        // shift-and-mask sequence for this op; the codegen is
        // identical to the Scalar kernel here. Other trait methods
        // (huf_burst, copy_chunk) will diverge once they land.
        ScalarKernel::mask_lower_bits(value, n)
    }
}

/// aarch64 SVE kernel. Variable-vector-length SVE extends NEON for
/// HUF burst / SIMD copy on Graviton3 / Apple M-series with SVE
/// support. Mask op identical to NEON / Scalar.
///
/// `#[allow(dead_code)]`: same scaffolding rationale as `NeonKernel`.
#[cfg(target_arch = "aarch64")]
#[allow(dead_code)]
#[derive(Copy, Clone, Default)]
pub(crate) struct SveKernel;

#[cfg(target_arch = "aarch64")]
impl CpuKernel for SveKernel {
    #[inline(always)]
    fn mask_lower_bits(value: u64, n: u8) -> u64 {
        ScalarKernel::mask_lower_bits(value, n)
    }
}

/// Single `#[target_feature(enable = "bmi2")]` wrapper around the
/// `_bzhi_u64` intrinsic. Lifted to a free function so each kernel
/// impl that needs the BMI2 path (Bmi2 / Avx2 / Vbmi2) calls the
/// same shared body. With `#[inline]` LLVM inlines the call into
/// any caller that itself has BMI2 in scope; outside that scope the
/// target_feature boundary is preserved.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "bmi2")]
#[inline]
unsafe fn mask_lower_bits_bmi2_impl(value: u64, n: u8) -> u64 {
    // The intrinsic call is permitted directly inside a function
    // already annotated `#[target_feature(enable = "bmi2")]` — no
    // `unsafe { ... }` block needed (the function-level `unsafe`
    // already covers it). SAFETY: caller selected a kernel whose
    // CpuKernelTag was resolved after `is_x86_feature_detected!("bmi2")`
    // returned true, so the BMI2 instruction set is available.
    core::arch::x86_64::_bzhi_u64(value, n as u32)
}

/// Pure boolean-input variant of the x86 kernel-tag selection. Both the
/// `std` runtime-detect path and the `no_std` compile-time-cfg path
/// route through this helper so the precedence rules stay in one place
/// (and are unit-testable without runtime CPUID).
///
/// The VBMI2 tier requires every AVX-512 sub-feature it touches AND the
/// AVX2 baseline — VBMI2 kernels mix VBMI2-only intrinsics with AVX2
/// 256-bit moves, so the dispatch must be conditioned on `has_avx2` too.
/// Likewise the Avx2 tier requires both AVX2 and BMI2.
#[cfg(target_arch = "x86_64")]
#[inline(always)]
// Params go unused when the matching `kernel_*` feature is disabled (the
// rung that consumes them is `#[cfg]`-ed out); they are still passed by the
// detect callers. Silence the conditional unused-variable warning rather
// than thread per-feature `_`-prefixes through the signature.
#[allow(unused_variables)]
const fn select_x86_kernel(
    has_avx512vbmi2: bool,
    has_avx512f: bool,
    has_avx512vl: bool,
    has_avx512bw: bool,
    has_bmi2: bool,
    has_avx2: bool,
    has_sse2: bool,
) -> CpuKernelTag {
    if has_avx2 && has_bmi2 {
        return CpuKernelTag::Avx2;
    }

    if has_bmi2 {
        return CpuKernelTag::Bmi2;
    }

    if has_sse2 {
        return CpuKernelTag::Sse2;
    }
    CpuKernelTag::Scalar
}

/// Cached runtime-detected kernel tag. The actual `CpuKernel` impl
/// (`ScalarKernel` / `Bmi2Kernel` / `Avx2Kernel` / `Vbmi2Kernel` /
/// `NeonKernel` / `SveKernel`) is constructed at the dispatch site —
/// currently only `decoding::literals_section_decoder::decompress_literals`
/// — via a `match` on this tag that branches into the per-K
/// `target_feature`-wrapped specialisation. Pipeline-wide dispatch
/// (FrameDecoder / FrameCompressor entry, sequence executor, match
/// copy) lands incrementally in follow-up tiers.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum CpuKernelTag {
    Scalar,
    #[cfg(target_arch = "x86_64")]
    Sse2,
    #[cfg(target_arch = "x86_64")]
    Bmi2,
    #[cfg(target_arch = "x86_64")]
    Avx2,

    #[cfg(target_arch = "aarch64")]
    Neon,
    // Both constructors of `Sve` need a reachable feature: runtime
    // detection via `std::arch::is_aarch64_feature_detected!` (so
    // `all()`) or compile-time `target_feature = "sve"` in
    // RUSTFLAGS. Without either, the variant is unreachable and a
    // `match` arm referencing it warns as dead.
    #[cfg(target_arch = "aarch64")]
    Sve,
}

/// Detect once and cache the best available CPU kernel for the
/// current process. Subsequent calls return the cached tag without
/// re-running CPU-feature detection. Std-only — no-std targets use
/// the compile-time variant below that resolves at build time.

pub(crate) fn detect_cpu_kernel() -> CpuKernelTag {
    static CACHED: OnceLock<CpuKernelTag> = OnceLock::new();
    *CACHED.get_or_init(detect_cpu_kernel_uncached)
}

fn detect_cpu_kernel_uncached() -> CpuKernelTag {
    #[cfg(target_arch = "x86_64")]
    {
        use std::arch::is_x86_feature_detected;
        // Gate each probe on its tier feature: `cfg!(...)` const-folds, so the
        // `&&` short-circuits away the runtime `is_x86_feature_detected!` call
        // (and its CPUID/cache traffic) for tiers the build disabled — the
        // matching `select_x86_kernel` rung is `#[cfg]`-ed out anyway.
        return select_x86_kernel(
            false,
            false,
            false,
            false,
            is_x86_feature_detected!("bmi2"),
            is_x86_feature_detected!("avx2"),
            is_x86_feature_detected!("sse2"),
        );
    }
    #[cfg(target_arch = "aarch64")]
    {
        use std::arch::is_aarch64_feature_detected;

        if is_aarch64_feature_detected!("sve") {
            return CpuKernelTag::Sve;
        }

        if is_aarch64_feature_detected!("neon") {
            return CpuKernelTag::Neon;
        }
        return CpuKernelTag::Scalar;
    }
    #[allow(unreachable_code)]
    CpuKernelTag::Scalar
}

impl CpuKernelTag {
    /// Stable lowercase diagnostic name for this tier (used by
    /// [`active_cpu_kernel_name`] and the bench/dashboard reporting). Pure
    /// mapping over the tag, so every arm is exercisable in tests regardless
    /// of which tier the running CPU actually resolves to.
    pub(crate) fn name(self) -> &'static str {
        match self {
            CpuKernelTag::Scalar => "scalar",
            #[cfg(target_arch = "x86_64")]
            CpuKernelTag::Sse2 => "sse2",
            #[cfg(target_arch = "x86_64")]
            CpuKernelTag::Bmi2 => "bmi2",
            #[cfg(target_arch = "x86_64")]
            CpuKernelTag::Avx2 => "avx2",

            #[cfg(target_arch = "aarch64")]
            CpuKernelTag::Neon => "neon",
            #[cfg(target_arch = "aarch64")]
            CpuKernelTag::Sve => "sve",
        }
    }
}

/// Name of the CPU kernel tier this process selected for the entropy /
/// sequence hot paths: decode (literals + FSE sequence decode) and encode
/// (entropy) share this dispatch. Returned as a stable lowercase
/// string for diagnostics and benchmark/dashboard reporting; the value is
/// what the runtime CPU-feature detection (or compile-time `target_feature`
/// on `no_std`) actually resolves to on this machine, so a dashboard can
/// attribute a measurement to the kernel that produced it.
pub fn active_cpu_kernel_name() -> &'static str {
    detect_cpu_kernel().name()
}
