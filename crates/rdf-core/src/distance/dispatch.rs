// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The compilations of the [`Exact`](super::Exact) body, and the choice between them.
//!
//! Each path is an ordinary module of ordinary wrapper functions around the one
//! `#[inline(always)]` body in `exact`. `portable` compiles it for the target's
//! baseline features; on `x86_64`, `avx2` compiles the identical source under
//! `#[target_feature(enable = "avx2")]`. The functions here are only ever *called*,
//! never turned into pointers: a `#[target_feature]` function does not coerce to a plain
//! `fn` pointer, and a table of them would be a table of unsafe calls with no record of
//! which processor reported what.
//!
//! No path enables `fma`, and none could fuse the exact fold if it did: Rust never
//! marks a plain `*` or `+` as contractable, so LLVM has no licence to fuse them.
//!
//! The portable wrappers are `#[inline(never)]` so each measure has one out-of-line
//! copy per instantiation, which is also the symbol the asm evidence gate measures.

use super::{Bound, Bounded, Measure, Path, RowsRef, Scalar, exact};

/// The path [`Exact`](super::Exact) runs on this process.
///
/// `is_x86_feature_detected!` caches its answer, so this is a load after the first call.
pub(crate) fn exact_path() -> Path {
    #[cfg(target_arch = "x86_64")]
    {
        if std::is_x86_feature_detected!("avx2") {
            return Path::Avx2;
        }
    }
    Path::Portable
}

/// The generic body compiled for the target's baseline features.
pub(crate) mod portable {
    use super::{Bound, Bounded, Measure, RowsRef, Scalar, exact};

    /// See [`exact::distances`].
    #[inline(never)]
    pub(crate) fn distances<Q: Scalar, T: Scalar>(
        measure: Measure,
        query: &[Q],
        query_norm: f64,
        rows: RowsRef<'_, T>,
        out: &mut [Option<f64>],
    ) {
        exact::distances(measure, query, query_norm, rows, out);
    }

    /// See [`exact::distances_indexed`].
    #[inline(never)]
    pub(crate) fn distances_indexed<Q: Scalar, T: Scalar>(
        measure: Measure,
        query: &[Q],
        query_norm: f64,
        rows: RowsRef<'_, T>,
        ids: &[usize],
        out: &mut [Option<f64>],
    ) {
        exact::distances_indexed(measure, query, query_norm, rows, ids, out);
    }

    /// See [`exact::distance`].
    #[inline(never)]
    pub(crate) fn distance<Q: Scalar, T: Scalar>(
        measure: Measure,
        a: &[Q],
        a_norm: f64,
        b: &[T],
        b_norm: f64,
    ) -> Option<f64> {
        exact::distance(measure, a, a_norm, b, b_norm)
    }

    /// See [`exact::distance_bounded`].
    #[inline(never)]
    pub(crate) fn distance_bounded<Q: Scalar, T: Scalar>(
        measure: Measure,
        a: &[Q],
        a_norm: f64,
        b: &[T],
        b_norm: f64,
        bound: Bound,
    ) -> Bounded {
        exact::distance_bounded(measure, a, a_norm, b, b_norm, bound)
    }
}

/// The same generic body compiled with AVX2 enabled.
///
/// Safe functions carrying `#[target_feature]`: calling one is `unsafe` from any context
/// that does not itself enable AVX2, and the only such call sites are the wrappers
/// below, each behind a [`Path::Avx2`] that only [`exact_path`] produces.
#[cfg(target_arch = "x86_64")]
pub(crate) mod avx2 {
    use super::{Bound, Bounded, Measure, RowsRef, Scalar, exact};

    /// See [`exact::distances`].
    #[target_feature(enable = "avx2")]
    pub(crate) fn distances<Q: Scalar, T: Scalar>(
        measure: Measure,
        query: &[Q],
        query_norm: f64,
        rows: RowsRef<'_, T>,
        out: &mut [Option<f64>],
    ) {
        exact::distances(measure, query, query_norm, rows, out);
    }

    /// See [`exact::distances_indexed`].
    #[target_feature(enable = "avx2")]
    pub(crate) fn distances_indexed<Q: Scalar, T: Scalar>(
        measure: Measure,
        query: &[Q],
        query_norm: f64,
        rows: RowsRef<'_, T>,
        ids: &[usize],
        out: &mut [Option<f64>],
    ) {
        exact::distances_indexed(measure, query, query_norm, rows, ids, out);
    }

    /// See [`exact::distance`].
    #[target_feature(enable = "avx2")]
    pub(crate) fn distance<Q: Scalar, T: Scalar>(
        measure: Measure,
        a: &[Q],
        a_norm: f64,
        b: &[T],
        b_norm: f64,
    ) -> Option<f64> {
        exact::distance(measure, a, a_norm, b, b_norm)
    }

    /// See [`exact::distance_bounded`].
    #[target_feature(enable = "avx2")]
    pub(crate) fn distance_bounded<Q: Scalar, T: Scalar>(
        measure: Measure,
        a: &[Q],
        a_norm: f64,
        b: &[T],
        b_norm: f64,
        bound: Bound,
    ) -> Bounded {
        exact::distance_bounded(measure, a, a_norm, b, b_norm, bound)
    }
}

/// The message for a path this target cannot compile, which no [`Resolved`] carries.
///
/// [`Resolved`]: super::Resolved
#[cfg(not(target_arch = "x86_64"))]
const NO_AVX2: &str = "an AVX2 path exists only on x86_64, and only `exact_path` produces one";

/// [`exact::distances`] along `path`.
pub(crate) fn distances<Q: Scalar, T: Scalar>(
    path: Path,
    measure: Measure,
    query: &[Q],
    query_norm: f64,
    rows: RowsRef<'_, T>,
    out: &mut [Option<f64>],
) {
    match path {
        Path::Portable => portable::distances(measure, query, query_norm, rows, out),
        #[cfg(target_arch = "x86_64")]
        // SAFETY: a `Path::Avx2` reaches here only inside a `Resolved`, whose sole
        // constructor for this path is `exact_path`, which returns it only after
        // `is_x86_feature_detected!("avx2")` reported the feature on this processor.
        Path::Avx2 => unsafe { avx2::distances(measure, query, query_norm, rows, out) },
        #[cfg(not(target_arch = "x86_64"))]
        Path::Avx2 => unreachable!("{NO_AVX2}"),
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
    match path {
        Path::Portable => {
            portable::distances_indexed(measure, query, query_norm, rows, ids, out);
        }
        #[cfg(target_arch = "x86_64")]
        // SAFETY: as in `distances`; a `Path::Avx2` exists only after AVX2 was detected.
        Path::Avx2 => unsafe {
            avx2::distances_indexed(measure, query, query_norm, rows, ids, out);
        },
        #[cfg(not(target_arch = "x86_64"))]
        Path::Avx2 => unreachable!("{NO_AVX2}"),
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
    match path {
        Path::Portable => portable::distance(measure, a, a_norm, b, b_norm),
        #[cfg(target_arch = "x86_64")]
        // SAFETY: as in `distances`; a `Path::Avx2` exists only after AVX2 was detected.
        Path::Avx2 => unsafe { avx2::distance(measure, a, a_norm, b, b_norm) },
        #[cfg(not(target_arch = "x86_64"))]
        Path::Avx2 => unreachable!("{NO_AVX2}"),
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
    match path {
        Path::Portable => portable::distance_bounded(measure, a, a_norm, b, b_norm, bound),
        #[cfg(target_arch = "x86_64")]
        // SAFETY: as in `distances`; a `Path::Avx2` exists only after AVX2 was detected.
        Path::Avx2 => unsafe { avx2::distance_bounded(measure, a, a_norm, b, b_norm, bound) },
        #[cfg(not(target_arch = "x86_64"))]
        Path::Avx2 => unreachable!("{NO_AVX2}"),
    }
}
