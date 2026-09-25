// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The [`Reassociated`](super::Reassociated) arithmetic: its one body, its compilations,
//! and the choice between them.
//!
//! # The body
//!
//! A distance is folded in 64-element blocks. Inside a block the terms are summed with
//! `algebraic_add`, each product formed with `algebraic_mul` (and each difference with
//! `algebraic_sub`), which licenses LLVM to reassociate the block's sum across as many
//! vector accumulators as the target has and to contract each multiply into the add
//! that consumes it. The block sums are then combined with plain `+`, in ascending
//! block order. So every checkpoint the bounded squared-Euclidean fold tests is a true
//! prefix of the full value: its blocks' sums are non-negative, plain round-to-nearest
//! addition of a non-negative value never decreases the total, and a checkpoint that
//! meets the bound proves the total does too.
//!
//! # One compilation per path
//!
//! A reassociated result is a function of the inputs only for a fixed compilation:
//! two copies of the body, inlined into two callers, may be reassociated or contracted
//! differently and disagree in the last bit. So every path compiles the body exactly
//! once, out of line and non-generic, for each of the four pairs of stored widths
//! (`<path>::<query>_<row>::dot` and `::squared_euclidean_bounded`), and every caller on
//! that path calls that copy. `Scalar` is sealed to PURREMB's two widths precisely so
//! that a generic operand can be routed to its concrete copy; after monomorphization
//! the routing is a constant.
//!
//! The paths: on `x86_64` the baseline (SSE2) compilation, one with AVX2 and FMA
//! enabled, and one with AVX-512F enabled. A new result is computed on the widest the
//! processor reports, chosen at run time in the order AVX-512F, AVX2+FMA, SSE2; a result
//! an image recorded on a narrower one is recomputed on that one, since the processor
//! runs every compilation up to its widest. On `aarch64` the baseline is NEON, and on
//! wasm it is `simd128` or scalar as the build was made; both are fixed at compile time.
//! On every other target the baseline is the portable path: the same body compiled once
//! for whatever the target's baseline features are, fixed at compile time, so the
//! arithmetic runs on every target a build can be made for. Relaxed wasm SIMD is never
//! enabled: its results are left to the engine, which no contract here can name.
//!
//! The `#[target_feature]` compilations are only ever *called*, from the ordinary
//! wrapper impls below, never turned into function pointers.
//!
//! # Binary64 on every target
//!
//! Every operation, licensed or plain, is a [`Binary64`] method, and each entry point
//! enters a [`Precision`] scope first, as the exact law's do. On every target but the
//! x87 the licensed methods are the `algebraic_*` operations and the plain ones the
//! language operators, unchanged. On the x87 both are the correctly rounded binary64
//! sequences `binary64` describes, in written order: the licence permits reassociation
//! and contraction and requires neither, so that is one of the results it admits, and
//! it keeps the evidence's claim of binary64 arithmetic true there too -- every value the
//! fold holds is a binary64 value, rounded once, rather than an 80-bit register's.

use super::binary64::{Algebraic, Binary64, Precision};
use super::exact::{cosine, finite};
use super::sealed::{Stored, Width};
use super::{Bound, Bounded, Measure, Path, PathUnavailable, RowsRef, Scalar};

/// The elements summed under one reassociation licence before the block's sum is added,
/// in order, to the total; also the bounded fold's checkpoint interval.
pub(crate) const BLOCK: usize = 64;

/// The evidence text for `$name`, one literal per path.
macro_rules! evidence_along {
    ($name:literal) => {
        concat!(
            "reassociated binary64 arithmetic: distance sums are reassociated and may be \
             contracted to fused multiply-add along the ",
            $name,
            " dispatch path of this build, so results may differ in the last bits from \
             the exact arithmetic and between dispatch paths or builds, the sign of a zero \
             result is unspecified, and near-ties may order differently"
        )
    };
}

/// The divergence a reassociated result carries along `path`.
pub(crate) const fn evidence(path: Path) -> &'static str {
    match path {
        Path::Portable => evidence_along!("portable"),
        Path::Avx2 => evidence_along!("avx2"),
        Path::Sse2 => evidence_along!("sse2"),
        Path::Avx2Fma => evidence_along!("avx2+fma"),
        Path::Avx512f => evidence_along!("avx512f"),
        Path::Neon => evidence_along!("neon"),
        Path::WasmSimd128 => evidence_along!("wasm-simd128"),
        Path::WasmScalar => evidence_along!("wasm-scalar"),
    }
}

/// The path the baseline compilation is, on this target.
#[cfg(target_arch = "x86_64")]
const BASELINE: Path = Path::Sse2;
/// The path the baseline compilation is, on this target.
#[cfg(target_arch = "aarch64")]
const BASELINE: Path = Path::Neon;
/// The path the baseline compilation is, on this target.
#[cfg(all(
    any(target_arch = "wasm32", target_arch = "wasm64"),
    target_feature = "simd128"
))]
const BASELINE: Path = Path::WasmSimd128;
/// The path the baseline compilation is, on this target.
#[cfg(all(
    any(target_arch = "wasm32", target_arch = "wasm64"),
    not(target_feature = "simd128")
))]
const BASELINE: Path = Path::WasmScalar;
/// The path the baseline compilation is on a target with no named reassociated path:
/// the portable one, the body compiled for the target's baseline features.
#[cfg(not(any(
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "wasm32",
    target_arch = "wasm64"
)))]
const BASELINE: Path = Path::Portable;

/// A processor feature one of the `x86_64` compilations is built with.
#[cfg(target_arch = "x86_64")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Feature {
    /// AVX2.
    Avx2,
    /// Fused multiply-add.
    Fma,
    /// AVX-512 Foundation.
    Avx512f,
}

#[cfg(target_arch = "x86_64")]
impl Feature {
    /// The feature's name, as `is_x86_feature_detected!` and `#[target_feature]` spell it.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Avx2 => "avx2",
            Self::Fma => "fma",
            Self::Avx512f => "avx512f",
        }
    }

    /// Whether the processor reports this feature.
    ///
    /// `is_x86_feature_detected!` caches its answer, so this is a load after the first
    /// call. Under `cfg(test)` a unit test may hide a feature from the calling thread
    /// (`hidden`), standing in for a processor that lacks it; a test can only ever take
    /// a feature away, never report one the processor lacks, so every `true` here is the
    /// processor's own answer.
    fn detected(self) -> bool {
        #[cfg(test)]
        if hidden::hides(self) {
            return false;
        }
        match self {
            Self::Avx2 => std::is_x86_feature_detected!("avx2"),
            Self::Fma => std::is_x86_feature_detected!("fma"),
            Self::Avx512f => std::is_x86_feature_detected!("avx512f"),
        }
    }
}

/// The test hook that hides processor features from the calling thread.
#[cfg(all(test, target_arch = "x86_64"))]
pub(crate) mod hidden {
    use std::cell::Cell;

    use super::Feature;

    thread_local! {
        /// The features the calling thread treats as absent.
        static HIDDEN: Cell<[bool; 3]> = const { Cell::new([false; 3]) };
    }

    const fn slot(feature: Feature) -> usize {
        match feature {
            Feature::Avx2 => 0,
            Feature::Fma => 1,
            Feature::Avx512f => 2,
        }
    }

    /// While alive, the calling thread treats `feature` as one the processor lacks.
    pub(crate) struct Hidden {
        previous: [bool; 3],
    }

    impl Drop for Hidden {
        fn drop(&mut self) {
            HIDDEN.with(|hidden| hidden.set(self.previous));
        }
    }

    /// Hide `feature` from this thread until the guard drops.
    pub(crate) fn hide(feature: Feature) -> Hidden {
        let previous = HIDDEN.with(Cell::get);
        let mut now = previous;
        now[slot(feature)] = true;
        HIDDEN.with(|hidden| hidden.set(now));
        Hidden { previous }
    }

    /// Whether a test hid `feature` on this thread.
    pub(crate) fn hides(feature: Feature) -> bool {
        HIDDEN.with(Cell::get)[slot(feature)]
    }
}

/// The first feature among `features` the processor does not report.
#[cfg(target_arch = "x86_64")]
fn first_missing(features: &[Feature]) -> Option<Feature> {
    features.iter().copied().find(|feature| !feature.detected())
}

/// The features [`Path::Avx2Fma`]'s compilation needs.
#[cfg(target_arch = "x86_64")]
const AVX2_FMA: [Feature; 2] = [Feature::Avx2, Feature::Fma];

/// The features [`Path::Avx512f`]'s compilation needs, as [`path`] selects it.
#[cfg(target_arch = "x86_64")]
const AVX512F: [Feature; 3] = [Feature::Avx512f, Feature::Avx2, Feature::Fma];

/// Whether the processor reports every feature a [`Path::Avx512f`] compilation needs.
///
/// One rule for both arithmetics' AVX-512F paths: [`path`] and [`recorded`] select the
/// reassociated one by it, and `dispatch::exact_path` the exact one, so the path name
/// means the same detection whichever law runs along it.
#[cfg(target_arch = "x86_64")]
pub(crate) fn runs_avx512f() -> bool {
    first_missing(&AVX512F).is_none()
}

/// The path [`Reassociated`](super::Reassociated) runs on this process: the widest the
/// processor reports, or the build's compile-time path. Every target has one.
///
/// This is the path a new result is computed on. A result recorded on another path this
/// process can also run is resolved by [`recorded`] instead.
pub(crate) fn path() -> Path {
    #[cfg(target_arch = "x86_64")]
    {
        if runs_avx512f() {
            return Path::Avx512f;
        }
        if first_missing(&AVX2_FMA).is_none() {
            return Path::Avx2Fma;
        }
    }
    BASELINE
}

/// The reassociated path an image records as `code`, or `None` for a code that is not
/// one of the arithmetic's.
pub(crate) const fn path_of(code: u32) -> Option<Path> {
    match code {
        2 => Some(Path::Sse2),
        3 => Some(Path::Avx2Fma),
        4 => Some(Path::Avx512f),
        5 => Some(Path::Neon),
        6 => Some(Path::WasmSimd128),
        7 => Some(Path::WasmScalar),
        8 => Some(Path::Portable),
        _ => None,
    }
}

/// `path`, when this process can run it: its compilation is in this build and the
/// processor reports every feature that compilation was built with.
///
/// The second producer of a [`Path::Avx2Fma`] or [`Path::Avx512f`], beside [`path`]; it
/// returns one only after detecting exactly the features [`path`] requires for it.
///
/// # Errors
///
/// [`PathUnavailable::NotCompiled`] for a path of another target (NEON or wasm on
/// `x86_64`, the `x86_64` paths anywhere else, wasm `simd128` in a build without it) or a
/// reassociated baseline other than this target's, and
/// [`PathUnavailable::MissingFeature`] naming the first feature the processor does not
/// report.
pub(crate) fn recorded(path: Path) -> Result<Path, PathUnavailable> {
    match path {
        #[cfg(target_arch = "x86_64")]
        Path::Avx2Fma => first_missing(&AVX2_FMA).map_or(Ok(path), |missing| {
            Err(PathUnavailable::MissingFeature(missing.name()))
        }),
        #[cfg(target_arch = "x86_64")]
        Path::Avx512f => first_missing(&AVX512F).map_or(Ok(path), |missing| {
            Err(PathUnavailable::MissingFeature(missing.name()))
        }),
        path if path == BASELINE => Ok(path),
        _ => Err(PathUnavailable::NotCompiled),
    }
}

/// The one body every compilation inlines.
mod body {
    use super::{Algebraic, BLOCK, Binary64, Bound, Bounded, Scalar};

    /// `sum(a[i] · b[i])` over one block, under the reassociation licence.
    #[allow(
        clippy::inline_always,
        reason = "the body must be inlined into every compilation so each compiles it \
                  under its own target features"
    )]
    #[inline(always)]
    fn block_dot<A: Scalar, B: Scalar>(ops: Binary64<'_>, a: &[A], b: &[B]) -> f64 {
        let mut sum = 0.0_f64;
        for (x, y) in a.iter().zip(b) {
            sum = ops.algebraic_add(sum, ops.algebraic_mul(x.widen(), y.widen()));
        }
        sum
    }

    /// `sum((a[i] - b[i])²)` over one block, under the reassociation licence.
    #[allow(
        clippy::inline_always,
        reason = "the body must be inlined into every compilation so each compiles it \
                  under its own target features"
    )]
    #[inline(always)]
    fn block_squares<A: Scalar, B: Scalar>(ops: Binary64<'_>, a: &[A], b: &[B]) -> f64 {
        let mut sum = 0.0_f64;
        for (x, y) in a.iter().zip(b) {
            let delta = ops.algebraic_sub(x.widen(), y.widen());
            sum = ops.algebraic_add(sum, ops.algebraic_mul(delta, delta));
        }
        sum
    }

    /// The reassociated dot product over the operands' common prefix.
    #[allow(
        clippy::inline_always,
        reason = "the body must be inlined into every compilation so each compiles it \
                  under its own target features"
    )]
    #[inline(always)]
    pub(crate) fn dot<A: Scalar, B: Scalar>(ops: Binary64<'_>, a: &[A], b: &[B]) -> f64 {
        let len = a.len().min(b.len());
        let (blocks_a, tail_a) = a[..len].as_chunks::<BLOCK>();
        let (blocks_b, tail_b) = b[..len].as_chunks::<BLOCK>();
        let mut total = 0.0_f64;
        for (x, y) in blocks_a.iter().zip(blocks_b) {
            let block = block_dot(ops, x, y);
            total = ops.add(total, block);
        }
        if !tail_a.is_empty() {
            let block = block_dot(ops, tail_a, tail_b);
            total = ops.add(total, block);
        }
        total
    }

    /// The reassociated squared Euclidean distance, abandoned once a block checkpoint
    /// meets `bound`. Non-finiteness is tested before the bound at every checkpoint.
    #[allow(
        clippy::inline_always,
        reason = "the body must be inlined into every compilation so each compiles it \
                  under its own target features"
    )]
    #[inline(always)]
    pub(crate) fn squared_euclidean_bounded<A: Scalar, B: Scalar>(
        ops: Binary64<'_>,
        a: &[A],
        b: &[B],
        bound: Bound,
    ) -> Bounded {
        let len = a.len().min(b.len());
        let (blocks_a, tail_a) = a[..len].as_chunks::<BLOCK>();
        let (blocks_b, tail_b) = b[..len].as_chunks::<BLOCK>();
        let mut total = 0.0_f64;
        for (x, y) in blocks_a.iter().zip(blocks_b) {
            let block = block_squares(ops, x, y);
            total = ops.add(total, block);
            if !total.is_finite() {
                return Bounded::NonFinite;
            }
            if bound.is_met_by(total) {
                return Bounded::Beyond;
            }
        }
        if !tail_a.is_empty() {
            let block = block_squares(ops, tail_a, tail_b);
            total = ops.add(total, block);
        }
        if !total.is_finite() {
            return Bounded::NonFinite;
        }
        if bound.is_met_by(total) {
            return Bounded::Beyond;
        }
        Bounded::Below(total)
    }
}

/// One compilation of [`body`] per pair of stored widths, each non-generic and out of
/// line, so it exists exactly once in the build.
macro_rules! compilation {
    ($(#[$feature:meta])?) => {
        compilation!(@pair f64_f64, f64, f64 $(, #[$feature])?);
        compilation!(@pair f64_f32, f64, f32 $(, #[$feature])?);
        compilation!(@pair f32_f64, f32, f64 $(, #[$feature])?);
        compilation!(@pair f32_f32, f32, f32 $(, #[$feature])?);
    };
    (@pair $name:ident, $query:ty, $row:ty $(, #[$feature:meta])?) => {
        /// The compilation for one pair of stored widths: query first, then row.
        pub(crate) mod $name {
            use super::super::{Binary64, Bound, Bounded, body};

            /// The reassociated dot product.
            $(#[$feature])?
            #[inline(never)]
            pub(crate) fn dot(ops: Binary64<'_>, a: &[$query], b: &[$row]) -> f64 {
                body::dot(ops, a, b)
            }

            /// The reassociated squared Euclidean distance, bounded.
            $(#[$feature])?
            #[inline(never)]
            pub(crate) fn squared_euclidean_bounded(
                ops: Binary64<'_>,
                a: &[$query],
                b: &[$row],
                bound: Bound,
            ) -> Bounded {
                body::squared_euclidean_bounded(ops, a, b, bound)
            }
        }
    };
}

/// The body compiled for the target's baseline features: SSE2 on `x86_64`, NEON on
/// `aarch64`, on wasm `simd128` or scalar as the build was made, and on every other
/// target the portable path.
pub(crate) mod baseline {
    compilation!();
}

/// The body compiled with AVX2 and FMA enabled.
///
/// Safe functions carrying `#[target_feature]`: calling one is `unsafe` from any
/// context that does not itself enable the features, and the only such call sites are
/// the [`Avx2Fma`] impl below, reached only along a [`Path::Avx2Fma`] that only [`path`]
/// and [`recorded`] produce, each after detecting AVX2 and FMA.
#[cfg(target_arch = "x86_64")]
pub(crate) mod avx2fma {
    compilation!(#[target_feature(enable = "avx2,fma")]);
}

/// The body compiled with AVX-512F enabled (which implies AVX2 and FMA).
///
/// As [`avx2fma`], reached only along a [`Path::Avx512f`] that only [`path`] and
/// [`recorded`] produce, each after detecting AVX-512F, AVX2 and FMA.
#[cfg(target_arch = "x86_64")]
pub(crate) mod avx512f {
    compilation!(#[target_feature(enable = "avx512f")]);
}

/// Route a generic operand pair to its concrete compilation in `$module`.
macro_rules! by_width {
    ($module:ident :: $op:ident ($ops:expr, $a:expr, $b:expr $(, $extra:expr)*)) => {
        match (Stored::width($a), Stored::width($b)) {
            (Width::F64(a), Width::F64(b)) => $module::f64_f64::$op($ops, a, b $(, $extra)*),
            (Width::F64(a), Width::F32(b)) => $module::f64_f32::$op($ops, a, b $(, $extra)*),
            (Width::F32(a), Width::F64(b)) => $module::f32_f64::$op($ops, a, b $(, $extra)*),
            (Width::F32(a), Width::F32(b)) => $module::f32_f32::$op($ops, a, b $(, $extra)*),
        }
    };
}

/// One compilation of the body, as the batch and pair kernels call it.
trait Compiled {
    /// The reassociated dot product.
    fn dot<Q: Scalar, T: Scalar>(ops: Binary64<'_>, a: &[Q], b: &[T]) -> f64;

    /// The reassociated squared Euclidean distance, bounded.
    fn squared_euclidean_bounded<Q: Scalar, T: Scalar>(
        ops: Binary64<'_>,
        a: &[Q],
        b: &[T],
        bound: Bound,
    ) -> Bounded;

    /// The reassociated squared Euclidean distance; an overflow is an infinity.
    #[inline]
    fn squared_euclidean<Q: Scalar, T: Scalar>(ops: Binary64<'_>, a: &[Q], b: &[T]) -> f64 {
        match Self::squared_euclidean_bounded(ops, a, b, Bound::Above(f64::INFINITY)) {
            Bounded::Below(sum) => sum,
            Bounded::NonFinite | Bounded::Beyond => f64::INFINITY,
        }
    }
}

/// The [`baseline`] compilation.
struct Baseline;

impl Compiled for Baseline {
    #[inline]
    fn dot<Q: Scalar, T: Scalar>(ops: Binary64<'_>, a: &[Q], b: &[T]) -> f64 {
        by_width!(baseline::dot(ops, a, b))
    }

    #[inline]
    fn squared_euclidean_bounded<Q: Scalar, T: Scalar>(
        ops: Binary64<'_>,
        a: &[Q],
        b: &[T],
        bound: Bound,
    ) -> Bounded {
        by_width!(baseline::squared_euclidean_bounded(ops, a, b, bound))
    }
}

/// The [`avx2fma`] compilation.
#[cfg(target_arch = "x86_64")]
struct Avx2Fma;

#[cfg(target_arch = "x86_64")]
impl Compiled for Avx2Fma {
    #[inline]
    fn dot<Q: Scalar, T: Scalar>(ops: Binary64<'_>, a: &[Q], b: &[T]) -> f64 {
        // SAFETY: `Avx2Fma` is named only by `on_path!` for a `Path::Avx2Fma`, which
        // reaches it only inside a `Resolved<Reassociated>`; that handle's only producers
        // for this path are `path` (through `Reassociated::resolve`) and `recorded`
        // (through `Reassociated::resolve_recorded`), each of which returns it only after
        // `is_x86_feature_detected!` reported both AVX2 and FMA on this processor.
        unsafe { by_width!(avx2fma::dot(ops, a, b)) }
    }

    #[inline]
    fn squared_euclidean_bounded<Q: Scalar, T: Scalar>(
        ops: Binary64<'_>,
        a: &[Q],
        b: &[T],
        bound: Bound,
    ) -> Bounded {
        // SAFETY: as in `dot`; a `Path::Avx2Fma` exists only after AVX2 and FMA were
        // detected.
        unsafe { by_width!(avx2fma::squared_euclidean_bounded(ops, a, b, bound)) }
    }
}

/// The [`avx512f`] compilation.
#[cfg(target_arch = "x86_64")]
struct Avx512f;

#[cfg(target_arch = "x86_64")]
impl Compiled for Avx512f {
    #[inline]
    fn dot<Q: Scalar, T: Scalar>(ops: Binary64<'_>, a: &[Q], b: &[T]) -> f64 {
        // SAFETY: `Avx512f` is named only by `on_path!` for a `Path::Avx512f`, which
        // reaches it only inside a `Resolved<Reassociated>`; that handle's only producers
        // for this path are `path` (through `Reassociated::resolve`) and `recorded`
        // (through `Reassociated::resolve_recorded`), each of which returns it only after
        // `is_x86_feature_detected!` reported AVX-512F, AVX2 and FMA on this processor.
        unsafe { by_width!(avx512f::dot(ops, a, b)) }
    }

    #[inline]
    fn squared_euclidean_bounded<Q: Scalar, T: Scalar>(
        ops: Binary64<'_>,
        a: &[Q],
        b: &[T],
        bound: Bound,
    ) -> Bounded {
        // SAFETY: as in `dot`; a `Path::Avx512f` exists only after AVX-512F was detected.
        unsafe { by_width!(avx512f::squared_euclidean_bounded(ops, a, b, bound)) }
    }
}

/// Refuse a path that is not one of this build's reassociated paths, which no
/// `Resolved<Reassociated>` carries: only [`path`] and [`recorded`] produce one, and
/// neither returns a path this build has no compilation of.
#[cold]
fn not_reassociated(path: Path) -> ! {
    unreachable!(
        "{path} is not a reassociated path of this build; only `path` and `recorded` \
         produce one"
    )
}

/// Run `$call` with `$k` naming the compilation `$path` selects. Matched once per
/// batch or pair, never inside a fold.
macro_rules! on_path {
    ($path:expr, $k:ident => $call:expr) => {
        match $path {
            #[cfg(target_arch = "x86_64")]
            Path::Avx512f => {
                type $k = Avx512f;
                $call
            }
            #[cfg(target_arch = "x86_64")]
            Path::Avx2Fma => {
                type $k = Avx2Fma;
                $call
            }
            path if path == BASELINE => {
                type $k = Baseline;
                $call
            }
            path => not_reassociated(path),
        }
    };
}

/// Every row of `rows` against `query` under compilation `K`, in row order.
fn batch<K: Compiled, Q: Scalar, T: Scalar>(
    ops: Binary64<'_>,
    measure: Measure,
    query: &[Q],
    query_norm: f64,
    rows: RowsRef<'_, T>,
    out: &mut [Option<f64>],
) {
    assert_eq!(
        query.len(),
        rows.dims(),
        "the query has {} components but the rows have {}",
        query.len(),
        rows.dims()
    );
    assert_eq!(
        out.len(),
        rows.rows(),
        "{} output slots for {} rows",
        out.len(),
        rows.rows()
    );
    match measure {
        Measure::SquaredEuclidean => {
            for (row, slot) in out.iter_mut().enumerate() {
                *slot = finite(K::squared_euclidean(ops, query, rows.row(row)));
            }
        }
        Measure::NegativeDot => {
            for (row, slot) in out.iter_mut().enumerate() {
                *slot = finite(-K::dot(ops, query, rows.row(row)));
            }
        }
        Measure::Cosine => {
            for (row, slot) in out.iter_mut().enumerate() {
                let value = cosine(
                    ops,
                    K::dot(ops, query, rows.row(row)),
                    query_norm,
                    rows.norm(row),
                );
                *slot = finite(value);
            }
        }
    }
}

/// The rows named by `ids` against `query` under compilation `K`, in `ids` order.
fn batch_indexed<K: Compiled, Q: Scalar, T: Scalar>(
    ops: Binary64<'_>,
    measure: Measure,
    query: &[Q],
    query_norm: f64,
    rows: RowsRef<'_, T>,
    ids: &[usize],
    out: &mut [Option<f64>],
) {
    assert_eq!(
        query.len(),
        rows.dims(),
        "the query has {} components but the rows have {}",
        query.len(),
        rows.dims()
    );
    assert_eq!(
        out.len(),
        ids.len(),
        "{} output slots for {} ids",
        out.len(),
        ids.len()
    );
    match measure {
        Measure::SquaredEuclidean => {
            for (&row, slot) in ids.iter().zip(out.iter_mut()) {
                *slot = finite(K::squared_euclidean(ops, query, rows.row(row)));
            }
        }
        Measure::NegativeDot => {
            for (&row, slot) in ids.iter().zip(out.iter_mut()) {
                *slot = finite(-K::dot(ops, query, rows.row(row)));
            }
        }
        Measure::Cosine => {
            for (&row, slot) in ids.iter().zip(out.iter_mut()) {
                let value = cosine(
                    ops,
                    K::dot(ops, query, rows.row(row)),
                    query_norm,
                    rows.norm(row),
                );
                *slot = finite(value);
            }
        }
    }
}

/// One pair under compilation `K`.
fn pair<K: Compiled, Q: Scalar, T: Scalar>(
    ops: Binary64<'_>,
    measure: Measure,
    a: &[Q],
    a_norm: f64,
    b: &[T],
    b_norm: f64,
) -> Option<f64> {
    let value = match measure {
        Measure::SquaredEuclidean => K::squared_euclidean(ops, a, b),
        Measure::NegativeDot => -K::dot(ops, a, b),
        Measure::Cosine => cosine(ops, K::dot(ops, a, b), a_norm, b_norm),
    };
    finite(value)
}

/// One pair under compilation `K`, permitted to stop once it cannot clear `bound`.
fn pair_bounded<K: Compiled, Q: Scalar, T: Scalar>(
    ops: Binary64<'_>,
    measure: Measure,
    a: &[Q],
    a_norm: f64,
    b: &[T],
    b_norm: f64,
    bound: Bound,
) -> Bounded {
    if measure == Measure::SquaredEuclidean {
        return K::squared_euclidean_bounded(ops, a, b, bound);
    }
    match pair::<K, Q, T>(ops, measure, a, a_norm, b, b_norm) {
        None => Bounded::NonFinite,
        Some(value) if bound.is_met_by(value) => Bounded::Beyond,
        Some(value) => Bounded::Below(value),
    }
}

/// Every row of `rows` against `query` along `path`.
pub(crate) fn distances<Q: Scalar, T: Scalar>(
    path: Path,
    measure: Measure,
    query: &[Q],
    query_norm: f64,
    rows: RowsRef<'_, T>,
    out: &mut [Option<f64>],
) {
    let precision = Precision::enter();
    let ops = precision.binary64();
    on_path!(path, K => batch::<K, Q, T>(ops, measure, query, query_norm, rows, out));
}

/// The rows named by `ids` against `query` along `path`.
pub(crate) fn distances_indexed<Q: Scalar, T: Scalar>(
    path: Path,
    measure: Measure,
    query: &[Q],
    query_norm: f64,
    rows: RowsRef<'_, T>,
    ids: &[usize],
    out: &mut [Option<f64>],
) {
    let precision = Precision::enter();
    let ops = precision.binary64();
    on_path!(path, K => batch_indexed::<K, Q, T>(ops, measure, query, query_norm, rows, ids, out));
}

/// One pair along `path`.
pub(crate) fn distance<Q: Scalar, T: Scalar>(
    path: Path,
    measure: Measure,
    a: &[Q],
    a_norm: f64,
    b: &[T],
    b_norm: f64,
) -> Option<f64> {
    let precision = Precision::enter();
    let ops = precision.binary64();
    on_path!(path, K => pair::<K, Q, T>(ops, measure, a, a_norm, b, b_norm))
}

/// One pair along `path`, permitted to stop once it cannot clear `bound`.
pub(crate) fn distance_bounded<Q: Scalar, T: Scalar>(
    path: Path,
    measure: Measure,
    a: &[Q],
    a_norm: f64,
    b: &[T],
    b_norm: f64,
    bound: Bound,
) -> Bounded {
    let precision = Precision::enter();
    let ops = precision.binary64();
    on_path!(path, K => pair_bounded::<K, Q, T>(ops, measure, a, a_norm, b, b_norm, bound))
}
