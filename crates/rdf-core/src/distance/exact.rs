// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The one body of the [`Exact`](super::Exact) law.
//!
//! Every function here is `#[inline(always)]` and generic, and none is called directly
//! by a consumer: `dispatch` compiles each of them once per path (portable, and AVX2 on
//! `x86_64`) by inlining the same source into differently-featured wrappers. So there
//! is one source order, and every path is a compilation of it.
//!
//! The formulation is load-bearing. A fold written `acc[l] += a[16 * c + l] * b[..]`
//! hides the lanes' independence behind index arithmetic and compiles to scalar code;
//! the fold below walks whole sixteen-element arrays (`as_chunks`, the array form of
//! `chunks_exact`) and rebuilds the accumulator with `array::from_fn`, which LLVM packs.

use core::array;

use super::{Bound, Bounded, Measure, RowsRef, Scalar};

/// The number of independent accumulators.
pub(crate) const LANES: usize = 16;

/// How many whole chunks are folded between bound tests: 64 elements.
pub(crate) const CHECKPOINT_CHUNKS: usize = 4;

/// The fixed pairwise tree over the sixteen lanes: `(l, l+8)`, `(l, l+4)`,
/// `(l, l+2)`, then `(0, 1)`.
#[allow(
    clippy::inline_always,
    reason = "the law's single body must be inlined into every dispatch wrapper so each \
              wrapper compiles it under its own target features"
)]
#[inline(always)]
pub(crate) fn tree(lanes: &[f64; LANES]) -> f64 {
    let s8: [f64; 8] = array::from_fn(|l| lanes[l] + lanes[l + 8]);
    let s4: [f64; 4] = array::from_fn(|l| s8[l] + s8[l + 4]);
    let s2: [f64; 2] = array::from_fn(|l| s4[l] + s4[l + 2]);
    s2[0] + s2[1]
}

/// `sum(a[i] · b[i])` under the exact law, over the operands' common prefix.
#[allow(
    clippy::inline_always,
    reason = "the law's single body must be inlined into every dispatch wrapper so each \
              wrapper compiles it under its own target features"
)]
#[allow(
    clippy::suboptimal_flops,
    reason = "the multiply and the add are deliberately separate roundings: a fused \
              multiply-add would make the bits depend on the target's FMA support"
)]
#[inline(always)]
pub(crate) fn dot<A: Scalar, B: Scalar>(a: &[A], b: &[B]) -> f64 {
    let len = a.len().min(b.len());
    let (chunks_a, tail_a) = a[..len].as_chunks::<LANES>();
    let (chunks_b, tail_b) = b[..len].as_chunks::<LANES>();
    let mut lanes = [0.0_f64; LANES];
    for (x, y) in chunks_a.iter().zip(chunks_b) {
        lanes = array::from_fn(|l| {
            let product = x[l].widen() * y[l].widen();
            lanes[l] + product
        });
    }
    let mut sum = tree(&lanes);
    for (x, y) in tail_a.iter().zip(tail_b) {
        let product = x.widen() * y.widen();
        sum += product;
    }
    sum
}

/// `sum((a[i] - b[i])²)` under the exact law, abandoned once a checkpoint meets `bound`.
///
/// Every term is a square, so under round-to-nearest every lane is non-decreasing, and
/// the tree is monotone in each lane: a checkpoint's `tree(lanes)` is never above the
/// final value. So once one meets the bound the total does too, and stopping is
/// exact rather than approximate. Non-finiteness is tested **before** the bound at every
/// checkpoint, so an overflow is reported as an overflow even under a bound an infinity
/// would meet. A distance that is not abandoned is bit-identical to the full fold, because
/// the checkpoints only read the lanes.
#[allow(
    clippy::inline_always,
    reason = "the law's single body must be inlined into every dispatch wrapper so each \
              wrapper compiles it under its own target features"
)]
#[allow(
    clippy::suboptimal_flops,
    reason = "the multiply and the add are deliberately separate roundings: a fused \
              multiply-add would make the bits depend on the target's FMA support"
)]
#[inline(always)]
pub(crate) fn squared_euclidean_bounded<A: Scalar, B: Scalar>(
    a: &[A],
    b: &[B],
    bound: Bound,
) -> Bounded {
    let len = a.len().min(b.len());
    let (chunks_a, tail_a) = a[..len].as_chunks::<LANES>();
    let (chunks_b, tail_b) = b[..len].as_chunks::<LANES>();
    let mut lanes = [0.0_f64; LANES];
    for (block_a, block_b) in chunks_a
        .chunks(CHECKPOINT_CHUNKS)
        .zip(chunks_b.chunks(CHECKPOINT_CHUNKS))
    {
        for (x, y) in block_a.iter().zip(block_b) {
            lanes = array::from_fn(|l| {
                let delta = x[l].widen() - y[l].widen();
                let square = delta * delta;
                lanes[l] + square
            });
        }
        if block_a.len() == CHECKPOINT_CHUNKS {
            let partial = tree(&lanes);
            if !partial.is_finite() {
                return Bounded::NonFinite;
            }
            if bound.is_met_by(partial) {
                return Bounded::Beyond;
            }
        }
    }
    let mut sum = tree(&lanes);
    for (x, y) in tail_a.iter().zip(tail_b) {
        let delta = x.widen() - y.widen();
        let square = delta * delta;
        sum += square;
    }
    if !sum.is_finite() {
        return Bounded::NonFinite;
    }
    if bound.is_met_by(sum) {
        return Bounded::Beyond;
    }
    Bounded::Below(sum)
}

/// `sum((a[i] - b[i])²)` under the exact law.
///
/// Literally the bounded fold with a bound no finite sum can meet, so there is one
/// squared-Euclidean body and the bounded form cannot drift from the full one. An
/// overflow comes back as an infinity, which the caller reports as a non-finite result.
#[allow(
    clippy::inline_always,
    reason = "the law's single body must be inlined into every dispatch wrapper so each \
              wrapper compiles it under its own target features"
)]
#[inline(always)]
pub(crate) fn squared_euclidean<A: Scalar, B: Scalar>(a: &[A], b: &[B]) -> f64 {
    match squared_euclidean_bounded(a, b, Bound::Above(f64::INFINITY)) {
        Bounded::Below(sum) => sum,
        Bounded::NonFinite | Bounded::Beyond => f64::INFINITY,
    }
}

/// The finished distance under `measure`, or `None` when it left the finite range.
#[allow(
    clippy::inline_always,
    reason = "the law's single body must be inlined into every dispatch wrapper so each \
              wrapper compiles it under its own target features"
)]
#[inline(always)]
pub(crate) fn distance<A: Scalar, B: Scalar>(
    measure: Measure,
    a: &[A],
    a_norm: f64,
    b: &[B],
    b_norm: f64,
) -> Option<f64> {
    let value = match measure {
        Measure::SquaredEuclidean => squared_euclidean(a, b),
        Measure::NegativeDot => -dot(a, b),
        Measure::Cosine => cosine(dot(a, b), a_norm, b_norm),
    };
    finite(value)
}

/// PURREMB's cosine distance from a finished dot product: one product, one quotient,
/// one subtraction, each rounded on its own.
#[allow(
    clippy::inline_always,
    reason = "the law's single body must be inlined into every dispatch wrapper so each \
              wrapper compiles it under its own target features"
)]
#[inline(always)]
fn cosine(dot: f64, a_norm: f64, b_norm: f64) -> f64 {
    let denominator = a_norm * b_norm;
    let quotient = dot / denominator;
    1.0 - quotient
}

/// [`distance`], permitted to stop once the answer cannot clear `bound`.
#[allow(
    clippy::inline_always,
    reason = "the law's single body must be inlined into every dispatch wrapper so each \
              wrapper compiles it under its own target features"
)]
#[inline(always)]
pub(crate) fn distance_bounded<A: Scalar, B: Scalar>(
    measure: Measure,
    a: &[A],
    a_norm: f64,
    b: &[B],
    b_norm: f64,
    bound: Bound,
) -> Bounded {
    if measure == Measure::SquaredEuclidean {
        return squared_euclidean_bounded(a, b, bound);
    }
    match distance(measure, a, a_norm, b, b_norm) {
        None => Bounded::NonFinite,
        Some(value) if bound.is_met_by(value) => Bounded::Beyond,
        Some(value) => Bounded::Below(value),
    }
}

/// A finished value, or `None` when it is not finite.
#[allow(
    clippy::inline_always,
    reason = "the law's single body must be inlined into every dispatch wrapper so each \
              wrapper compiles it under its own target features"
)]
#[inline(always)]
fn finite(value: f64) -> Option<f64> {
    value.is_finite().then_some(value)
}

/// Every row of `rows` against `query`, in row order.
///
/// The measure is matched once, outside the row loop, so the loop over rows is one
/// monomorphic kernel per measure with no dispatch inside it.
#[allow(
    clippy::inline_always,
    reason = "the law's single body must be inlined into every dispatch wrapper so each \
              wrapper compiles it under its own target features"
)]
#[inline(always)]
pub(crate) fn distances<Q: Scalar, T: Scalar>(
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
                *slot = finite(squared_euclidean(query, rows.row(row)));
            }
        }
        Measure::NegativeDot => {
            for (row, slot) in out.iter_mut().enumerate() {
                *slot = finite(-dot(query, rows.row(row)));
            }
        }
        Measure::Cosine => {
            for (row, slot) in out.iter_mut().enumerate() {
                let value = cosine(dot(query, rows.row(row)), query_norm, rows.norm(row));
                *slot = finite(value);
            }
        }
    }
}

/// The rows named by `ids` against `query`, in `ids` order.
#[allow(
    clippy::inline_always,
    reason = "the law's single body must be inlined into every dispatch wrapper so each \
              wrapper compiles it under its own target features"
)]
#[inline(always)]
pub(crate) fn distances_indexed<Q: Scalar, T: Scalar>(
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
                *slot = finite(squared_euclidean(query, rows.row(row)));
            }
        }
        Measure::NegativeDot => {
            for (&row, slot) in ids.iter().zip(out.iter_mut()) {
                *slot = finite(-dot(query, rows.row(row)));
            }
        }
        Measure::Cosine => {
            for (&row, slot) in ids.iter().zip(out.iter_mut()) {
                let value = cosine(dot(query, rows.row(row)), query_norm, rows.norm(row));
                *slot = finite(value);
            }
        }
    }
}
