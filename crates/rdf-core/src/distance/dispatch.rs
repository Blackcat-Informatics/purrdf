// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The compilations of the [`Exact`](super::Exact) body, and the choice between them.
//!
//! Each path is an ordinary module of ordinary wrapper functions around the one
//! `#[inline(always)]` body in `exact`. `portable` compiles it for the target's
//! baseline features; on `x86_64`, `avx2` and `avx512f` compile the identical source
//! under `#[target_feature(enable = "avx2")]` and `#[target_feature(enable = "avx512f")]`.
//! The functions here are only ever *called*, never turned into pointers: a
//! `#[target_feature]` function does not coerce to a plain `fn` pointer, and a table of
//! them would be a table of unsafe calls with no record of which processor reported what.
//!
//! Only the AVX-512F path enables `fma` (the feature implies it), and no path can fuse
//! the exact fold: Rust never marks a plain `*` or `+` as contractable, so LLVM has no
//! licence to fuse them. The asm evidence gate requires zero FMA instructions in every
//! exact compilation, the AVX-512F one included.
//!
//! The portable wrappers are `#[inline(never)]` so each measure has one out-of-line
//! copy per instantiation, which is also the symbol the asm evidence gate measures.
//!
//! Each entry point below enters a [`Precision`] scope before choosing a path and hands
//! its [`Binary64`] operations down, so every operation of the law runs where they are
//! correctly rounded: on the x87 that sets the precision-control field for the duration,
//! and everywhere else it is nothing.

use super::binary64::{Binary64, Precision};
#[cfg(target_arch = "x86_64")]
use super::reassociated;
use super::{Bound, Bounded, Measure, Path, RowsRef, Scalar, exact};

/// The path [`Exact`](super::Exact) runs on this process: AVX-512F, then AVX2, then
/// portable, the widest the processor reports.
///
/// The AVX-512F path is detected by the one rule both arithmetics select it by,
/// `reassociated::runs_avx512f`: AVX-512F, AVX2 and FMA, the features its
/// `#[target_feature]` enables. `is_x86_feature_detected!` caches its answer, so this is
/// a few loads after the first call.
pub(crate) fn exact_path() -> Path {
    #[cfg(target_arch = "x86_64")]
    {
        if reassociated::runs_avx512f() {
            return Path::Avx512f;
        }
        if std::is_x86_feature_detected!("avx2") {
            return Path::Avx2;
        }
    }
    Path::Portable
}

/// The generic body compiled for the target's baseline features.
pub(crate) mod portable {
    use super::{Binary64, Bound, Bounded, Measure, RowsRef, Scalar, exact};

    /// See [`exact::distances`].
    #[inline(never)]
    pub(crate) fn distances<Q: Scalar, T: Scalar>(
        ops: Binary64<'_>,
        measure: Measure,
        query: &[Q],
        query_norm: f64,
        rows: RowsRef<'_, T>,
        out: &mut [Option<f64>],
    ) {
        exact::distances(ops, measure, query, query_norm, rows, out);
    }

    /// See [`exact::distances_indexed`].
    #[inline(never)]
    pub(crate) fn distances_indexed<Q: Scalar, T: Scalar>(
        ops: Binary64<'_>,
        measure: Measure,
        query: &[Q],
        query_norm: f64,
        rows: RowsRef<'_, T>,
        ids: &[usize],
        out: &mut [Option<f64>],
    ) {
        exact::distances_indexed(ops, measure, query, query_norm, rows, ids, out);
    }

    /// See [`exact::distance`].
    #[inline(never)]
    pub(crate) fn distance<Q: Scalar, T: Scalar>(
        ops: Binary64<'_>,
        measure: Measure,
        a: &[Q],
        a_norm: f64,
        b: &[T],
        b_norm: f64,
    ) -> Option<f64> {
        exact::distance(ops, measure, a, a_norm, b, b_norm)
    }

    /// See [`exact::distance_bounded`].
    #[inline(never)]
    pub(crate) fn distance_bounded<Q: Scalar, T: Scalar>(
        ops: Binary64<'_>,
        measure: Measure,
        a: &[Q],
        a_norm: f64,
        b: &[T],
        b_norm: f64,
        bound: Bound,
    ) -> Bounded {
        exact::distance_bounded(ops, measure, a, a_norm, b, b_norm, bound)
    }
}

/// The same generic body compiled with AVX2 enabled.
///
/// Safe functions carrying `#[target_feature]`: calling one is `unsafe` from any context
/// that does not itself enable AVX2, and the only such call sites are the wrappers
/// below, each behind a [`Path::Avx2`] that only [`exact_path`] produces.
#[cfg(target_arch = "x86_64")]
pub(crate) mod avx2 {
    use super::{Binary64, Bound, Bounded, Measure, RowsRef, Scalar, exact};

    /// See [`exact::distances`].
    #[target_feature(enable = "avx2")]
    pub(crate) fn distances<Q: Scalar, T: Scalar>(
        ops: Binary64<'_>,
        measure: Measure,
        query: &[Q],
        query_norm: f64,
        rows: RowsRef<'_, T>,
        out: &mut [Option<f64>],
    ) {
        exact::distances(ops, measure, query, query_norm, rows, out);
    }

    /// See [`exact::distances_indexed`].
    #[target_feature(enable = "avx2")]
    pub(crate) fn distances_indexed<Q: Scalar, T: Scalar>(
        ops: Binary64<'_>,
        measure: Measure,
        query: &[Q],
        query_norm: f64,
        rows: RowsRef<'_, T>,
        ids: &[usize],
        out: &mut [Option<f64>],
    ) {
        exact::distances_indexed(ops, measure, query, query_norm, rows, ids, out);
    }

    /// See [`exact::distance`].
    #[target_feature(enable = "avx2")]
    pub(crate) fn distance<Q: Scalar, T: Scalar>(
        ops: Binary64<'_>,
        measure: Measure,
        a: &[Q],
        a_norm: f64,
        b: &[T],
        b_norm: f64,
    ) -> Option<f64> {
        exact::distance(ops, measure, a, a_norm, b, b_norm)
    }

    /// See [`exact::distance_bounded`].
    #[target_feature(enable = "avx2")]
    pub(crate) fn distance_bounded<Q: Scalar, T: Scalar>(
        ops: Binary64<'_>,
        measure: Measure,
        a: &[Q],
        a_norm: f64,
        b: &[T],
        b_norm: f64,
        bound: Bound,
    ) -> Bounded {
        exact::distance_bounded(ops, measure, a, a_norm, b, b_norm, bound)
    }
}

/// The same generic body compiled with AVX-512F enabled.
///
/// Safe functions carrying `#[target_feature]`: calling one is `unsafe` from any context
/// that does not itself enable AVX-512F, and the only such call sites are the wrappers
/// below, each behind a [`Path::Avx512f`] that only [`exact_path`] produces.
#[cfg(target_arch = "x86_64")]
pub(crate) mod avx512f {
    use super::{Binary64, Bound, Bounded, Measure, RowsRef, Scalar, exact};

    /// See [`exact::distances`].
    #[target_feature(enable = "avx512f")]
    pub(crate) fn distances<Q: Scalar, T: Scalar>(
        ops: Binary64<'_>,
        measure: Measure,
        query: &[Q],
        query_norm: f64,
        rows: RowsRef<'_, T>,
        out: &mut [Option<f64>],
    ) {
        exact::distances(ops, measure, query, query_norm, rows, out);
    }

    /// See [`exact::distances_indexed`].
    #[target_feature(enable = "avx512f")]
    pub(crate) fn distances_indexed<Q: Scalar, T: Scalar>(
        ops: Binary64<'_>,
        measure: Measure,
        query: &[Q],
        query_norm: f64,
        rows: RowsRef<'_, T>,
        ids: &[usize],
        out: &mut [Option<f64>],
    ) {
        exact::distances_indexed(ops, measure, query, query_norm, rows, ids, out);
    }

    /// See [`exact::distance`].
    #[target_feature(enable = "avx512f")]
    pub(crate) fn distance<Q: Scalar, T: Scalar>(
        ops: Binary64<'_>,
        measure: Measure,
        a: &[Q],
        a_norm: f64,
        b: &[T],
        b_norm: f64,
    ) -> Option<f64> {
        exact::distance(ops, measure, a, a_norm, b, b_norm)
    }

    /// See [`exact::distance_bounded`].
    #[target_feature(enable = "avx512f")]
    pub(crate) fn distance_bounded<Q: Scalar, T: Scalar>(
        ops: Binary64<'_>,
        measure: Measure,
        a: &[Q],
        a_norm: f64,
        b: &[T],
        b_norm: f64,
        bound: Bound,
    ) -> Bounded {
        exact::distance_bounded(ops, measure, a, a_norm, b, b_norm, bound)
    }
}

/// Refuse a path that is not one of this build's exact paths, which no
/// `Resolved<Exact>` carries: only [`exact_path`] produces one.
#[cold]
fn not_exact(path: Path) -> ! {
    unreachable!("{path} is not an exact path of this build; only `exact_path` produces one")
}

/// [`exact::distances`] along `path`.
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
    match path {
        Path::Portable => portable::distances(ops, measure, query, query_norm, rows, out),
        #[cfg(target_arch = "x86_64")]
        // SAFETY: a `Path::Avx2` reaches here only inside a `Resolved`, whose sole
        // constructor for this path is `exact_path`, which returns it only after
        // `is_x86_feature_detected!("avx2")` reported the feature on this processor.
        Path::Avx2 => unsafe { avx2::distances(ops, measure, query, query_norm, rows, out) },
        #[cfg(target_arch = "x86_64")]
        // SAFETY: a `Path::Avx512f` reaches here only inside a `Resolved`, whose sole
        // constructor for this path is `exact_path`, which returns it only after
        // `reassociated::runs_avx512f` detected AVX-512F, AVX2 and FMA on this processor.
        Path::Avx512f => unsafe {
            avx512f::distances(ops, measure, query, query_norm, rows, out);
        },
        other => not_exact(other),
    }
}

/// [`exact::distances_indexed`] along `path`.
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
    match path {
        Path::Portable => {
            portable::distances_indexed(ops, measure, query, query_norm, rows, ids, out);
        }
        #[cfg(target_arch = "x86_64")]
        // SAFETY: as in `distances`; a `Path::Avx2` exists only after AVX2 was detected.
        Path::Avx2 => unsafe {
            avx2::distances_indexed(ops, measure, query, query_norm, rows, ids, out);
        },
        #[cfg(target_arch = "x86_64")]
        // SAFETY: as in `distances`; a `Path::Avx512f` exists only after AVX-512F was
        // detected.
        Path::Avx512f => unsafe {
            avx512f::distances_indexed(ops, measure, query, query_norm, rows, ids, out);
        },
        other => not_exact(other),
    }
}

/// [`exact::distance`] along `path`.
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
    match path {
        Path::Portable => portable::distance(ops, measure, a, a_norm, b, b_norm),
        #[cfg(target_arch = "x86_64")]
        // SAFETY: as in `distances`; a `Path::Avx2` exists only after AVX2 was detected.
        Path::Avx2 => unsafe { avx2::distance(ops, measure, a, a_norm, b, b_norm) },
        #[cfg(target_arch = "x86_64")]
        // SAFETY: as in `distances`; a `Path::Avx512f` exists only after AVX-512F was
        // detected.
        Path::Avx512f => unsafe { avx512f::distance(ops, measure, a, a_norm, b, b_norm) },
        other => not_exact(other),
    }
}

/// [`exact::distance_bounded`] along `path`.
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
    match path {
        Path::Portable => portable::distance_bounded(ops, measure, a, a_norm, b, b_norm, bound),
        #[cfg(target_arch = "x86_64")]
        // SAFETY: as in `distances`; a `Path::Avx2` exists only after AVX2 was detected.
        Path::Avx2 => unsafe { avx2::distance_bounded(ops, measure, a, a_norm, b, b_norm, bound) },
        #[cfg(target_arch = "x86_64")]
        // SAFETY: as in `distances`; a `Path::Avx512f` exists only after AVX-512F was
        // detected.
        Path::Avx512f => unsafe {
            avx512f::distance_bounded(ops, measure, a, a_norm, b, b_norm, bound)
        },
        other => not_exact(other),
    }
}
