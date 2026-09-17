// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Deterministic corpora with the geometry real embeddings have.
//!
//! # Why uniform random vectors cannot measure this index
//!
//! Draw every coordinate independently and distances **concentrate**: as the width grows,
//! the nearest and farthest points of a corpus converge to the same distance, and "nearest
//! neighbour" stops naming anything. An index measured against such a corpus reports a
//! number about the generator, not about itself — and the number is bad however good the
//! index is, because there is no gradient for a graph to descend.
//!
//! The quantity that governs it is not the nominal width `d` but the **effective
//! dimension** of the covariance spectrum `λ₁ … λ_d`:
//!
//! ```text
//! d_eff = (Σ λⱼ)² / Σ λⱼ²
//! ```
//!
//! Relative spread in pairwise distance scales as `1 / sqrt(d_eff)`. For i.i.d. coordinates
//! every `λ` is equal, `d_eff = d`, and at `d = 4096` the spread is about 1/64: the first
//! and hundredth neighbour differ by around one percent of a typical distance. Real
//! embeddings are nothing like that. Their spectra decay steeply — Matryoshka training
//! makes it an explicit objective, by requiring every leading prefix to be a usable
//! embedding on its own — so `d_eff` lands in the tens whatever the nominal width, and
//! neighbours separate.
//!
//! # What this generator does
//!
//! Four properties, each present because leaving it out collapses the geometry:
//!
//! 1. **Low intrinsic dimension.** Vectors are drawn in a latent space of a few dozen
//!    dimensions and carried into the full width by a fixed projection. Structure that
//!    lives on a low-dimensional manifold survives being embedded in a wide one.
//! 2. **A decaying spectrum.** Ambient coordinate `j` is scaled by `1 / sqrt(j + 1)`, so
//!    variance falls as `1 / (j + 1)` and leading coordinates dominate — the Matryoshka
//!    property, and a fair model of ordinary embeddings too. With that spectrum
//!    `d_eff ≈ (ln d)² / ζ(2)`, which is about 42 at `d = 4096`.
//! 3. **Power-law cluster sizes.** Corpora are not made of equal piles. Membership follows
//!    a harmonic law, so a few clusters hold most rows and a long tail holds the rest.
//! 4. **Dimension-invariant cluster tightness.** Members are placed at an *intended cosine*
//!    `ρ` to their centroid: `v = ρ·c + sqrt(1 - ρ²)·u`. This is the property that must be
//!    parameterised, and getting it wrong is subtle. Adding isotropic noise of a fixed
//!    amplitude `σ` instead gives an expected cosine of `1 / sqrt(1 + d·σ²)`, which is a
//!    function of the width: an amplitude that makes tight clusters at `d = 64` drives the
//!    cosine to about 0.077 at `d = 4096`, leaving members essentially orthogonal to the
//!    centroid they were supposedly drawn around. A generator like that produces a corpus
//!    that *looks* clustered in its source and is indistinguishable from uniform in its
//!    output.
//!
//! # Determinism
//!
//! Everything derives from `splitmix64` over a caller-supplied seed: no RNG crate, no
//! clock, no entropy. The arithmetic is confined to add, multiply, divide and `sqrt`,
//! **deliberately**. IEEE-754 requires `sqrt` to be correctly rounded, so it is bit-identical
//! on every target; `powf`, `ln` and `exp` carry no such requirement and legitimately differ
//! in the last place between libm implementations, which would put a cross-target digest at
//! the mercy of the host's math library. The decay exponent is fixed at 1 for exactly that
//! reason — `1 / sqrt(j + 1)` needs no transcendental. Cluster assignment is integer
//! arithmetic on the hash stream, so no float takes part in a branch either.

use crate::error::Result;
use crate::graph::VectorMatrix;
use crate::level::splitmix64;

/// The shape and geometry of a generated corpus.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CorpusShape {
    /// How many vectors.
    pub rows: usize,
    /// The ambient width.
    pub dims: usize,
    /// The latent dimension the structure actually lives in.
    pub intrinsic: usize,
    /// How many clusters the latent structure carries.
    pub clusters: usize,
    /// The intended cosine between a member and its centroid, in `[0, 1)`.
    ///
    /// Dimension-invariant by construction. A value near 1 makes tight clusters, a value
    /// near 0 makes the corpus formless; realistic embedding corpora sit well above 0.5.
    pub within_cluster_cosine: f64,
}

impl CorpusShape {
    /// A corpus with the geometry of a Matryoshka embedding set at this width.
    #[must_use]
    pub const fn embedding_like(rows: usize, dims: usize) -> Self {
        Self {
            rows,
            dims,
            intrinsic: 32,
            clusters: 64,
            within_cluster_cosine: 0.75,
        }
    }
}

/// A deterministic stream of values in `[-1, 1)`, never exactly zero.
struct Stream(u64);

impl Stream {
    const fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_bits(&mut self) -> u64 {
        self.0 = splitmix64(self.0);
        self.0
    }

    /// A value in `[-1, 1)`, never exactly zero so a norm cannot collapse.
    fn unit(&mut self) -> f64 {
        let bits = self.next_bits();
        #[expect(
            clippy::cast_precision_loss,
            reason = "a 53-bit mantissa is ample for a coordinate draw"
        )]
        let value = ((bits >> 11) as f64 / (1_u64 << 53) as f64).mul_add(2.0, -1.0);
        if value == 0.0 { 0.125 } else { value }
    }

    /// A unit-norm vector of `len` components.
    fn direction(&mut self, len: usize) -> Vec<f64> {
        let mut values: Vec<f64> = (0..len).map(|_| self.unit()).collect();
        normalize(&mut values);
        values
    }
}

/// Scale `values` to unit L2 norm. A zero vector is left alone; `Stream::unit` cannot
/// produce one, and a caller-supplied degenerate vector is the caller's to refuse.
fn normalize(values: &mut [f64]) {
    let sum: f64 = values.iter().map(|value| value * value).sum();
    if sum <= 0.0 {
        return;
    }
    let norm = sum.sqrt();
    for value in values.iter_mut() {
        *value /= norm;
    }
}

/// Which cluster a draw falls in, given the weights [`cluster_weights`] computed once.
///
/// Takes the table rather than rebuilding it: this is called once per generated row, and
/// rebuilding a `clusters`-length vector per row allocated once per row for a table that
/// never changes.
fn cluster_of(bits: u64, weights: &[u64], total: u64) -> usize {
    let mut pick = bits % total.max(1);
    for (c, weight) in weights.iter().enumerate() {
        if pick < *weight {
            return c;
        }
        pick -= *weight;
    }
    weights.len().saturating_sub(1)
}

/// The harmonic size law's per-cluster weights, computed once per corpus.
///
/// Cluster `c` receives weight `ceil(clusters / (c + 1))`, so the first cluster is the
/// largest and the tail is long. Integer arithmetic throughout, so the assignment cannot
/// drift with a math library.
fn cluster_weights(clusters: usize) -> (Vec<u64>, u64) {
    let weights: Vec<u64> = (0..clusters)
        .map(|c| (clusters as u64).div_ceil(c as u64 + 1))
        .collect();
    let total = weights.iter().sum();
    (weights, total)
}

/// Generate a corpus with the geometry described by `shape`.
///
/// # Errors
///
/// Whatever [`VectorMatrix::new`] refuses: a zero axis, an element count that overflows, or
/// a non-finite component (which this generator cannot produce).
///
/// # Panics
///
/// Panics if `within_cluster_cosine` is not in `[0, 1)`, which is a caller error rather than
/// a data condition.
pub fn embedding_like(shape: CorpusShape, seed: u64) -> Result<VectorMatrix> {
    assert!(
        (0.0..1.0).contains(&shape.within_cluster_cosine),
        "the intended within-cluster cosine must lie in [0, 1)"
    );
    let CorpusShape {
        rows,
        dims,
        intrinsic,
        clusters,
        within_cluster_cosine: rho,
    } = shape;
    let latent = intrinsic.max(1).min(dims);
    let clusters = clusters.max(1);
    let spread = f64::mul_add(rho, -rho, 1.0).sqrt();

    let mut stream = Stream::new(seed);

    // The projection carrying latent structure into the ambient width, and the spectrum
    // that makes leading coordinates dominate. Both are fixed for the whole corpus: they
    // are the space, not the sample.
    let projection: Vec<Vec<f64>> = (0..latent).map(|_| stream.direction(dims)).collect();
    #[expect(
        clippy::cast_precision_loss,
        reason = "a coordinate index is far inside a 53-bit mantissa"
    )]
    let scale: Vec<f64> = (0..dims).map(|j| 1.0 / ((j as f64) + 1.0).sqrt()).collect();

    let centroids: Vec<Vec<f64>> = (0..clusters).map(|_| stream.direction(latent)).collect();
    let (weights, weight_total) = cluster_weights(clusters);

    let mut data = Vec::with_capacity(rows * dims);
    for _ in 0..rows {
        let cluster = cluster_of(stream.next_bits(), &weights, weight_total);
        let offset = stream.direction(latent);

        // Intended cosine, not absolute noise: the mix is the same at every width.
        let mut point = Vec::with_capacity(latent);
        for axis in 0..latent {
            point.push(rho.mul_add(centroids[cluster][axis], spread * offset[axis]));
        }
        normalize(&mut point);

        // Project, apply the spectrum, and renormalise so every row is comparable.
        let mut ambient = vec![0.0_f64; dims];
        for (axis, weight) in point.iter().enumerate() {
            for (column, target) in ambient.iter_mut().enumerate() {
                *target = weight.mul_add(projection[axis][column], *target);
            }
        }
        for (column, value) in ambient.iter_mut().enumerate() {
            *value *= scale[column];
        }
        normalize(&mut ambient);
        data.extend_from_slice(&ambient);
    }

    VectorMatrix::new(rows, dims, data)
}

/// A corpus of disjoint low-dimensional manifolds with no overlap between them.
///
/// The adversary for a navigable graph: structure that is locally dense and globally
/// disconnected, where a greedy descent that enters the wrong manifold has no downhill path
/// to the right one. A graph built by nearest-`M` truncation fails here in a way no uniform
/// corpus reveals.
///
/// # Errors
///
/// As [`embedding_like`].
pub fn separated_manifolds(
    rows: usize,
    dims: usize,
    manifolds: usize,
    seed: u64,
) -> Result<VectorMatrix> {
    let manifolds = manifolds.max(1).min(dims);
    let mut stream = Stream::new(seed);
    // Each manifold owns a disjoint block of coordinates, so two manifolds share no axis.
    let block = (dims / manifolds).max(1);
    let mut data = Vec::with_capacity(rows * dims);
    for row in 0..rows {
        let manifold = row % manifolds;
        let start = manifold * block;
        let mut ambient = vec![0.0_f64; dims];
        for slot in ambient
            .iter_mut()
            .take((start + block).min(dims))
            .skip(start)
        {
            *slot = stream.unit();
        }
        normalize(&mut ambient);
        data.extend_from_slice(&ambient);
    }
    VectorMatrix::new(rows, dims, data)
}

/// A corpus of extreme but finite magnitudes, including signed zero components.
///
/// Every component is finite and every row has a positive norm, so this is a corpus the
/// index must *handle*, not one it may refuse. It exists because a kernel that accumulates
/// without care, or a comparator that folds `-0.0` and `+0.0` differently, fails here and
/// nowhere else.
///
/// # Errors
///
/// As [`embedding_like`].
pub fn extreme_but_finite(rows: usize, dims: usize, seed: u64) -> Result<VectorMatrix> {
    // Large enough to stress accumulation, small enough that a squared-euclidean kernel
    // over `dims` components stays finite with room to spare.
    let large = f64::MAX.sqrt() / (dims.max(1) as f64) / 16.0;
    let mut stream = Stream::new(seed);
    let mut data = Vec::with_capacity(rows * dims);
    for row in 0..rows {
        for _ in 0..dims {
            let bits = stream.next_bits();
            let value = match bits % 4 {
                0 => 0.0,
                1 => -0.0,
                2 => large * stream.unit(),
                _ => stream.unit(),
            };
            data.push(value);
        }
        // A zero row would be refused under a norm-dividing kernel, which is a different
        // test; guarantee one non-zero component so this corpus is about magnitudes.
        let start = row * dims;
        if data[start..start + dims].iter().all(|v| *v == 0.0) {
            data[start] = large;
        }
    }
    VectorMatrix::new(rows, dims, data)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The inverse participation ratio of a corpus's per-coordinate variance.
    ///
    /// This is the quantity the module docs claim to control, computed from the generated
    /// vectors rather than from the parameters that produced them.
    fn effective_dimension(matrix: &VectorMatrix) -> f64 {
        let (rows, dims) = (matrix.rows(), matrix.dims());
        let mut variance = vec![0.0_f64; dims];
        for (column, slot) in variance.iter_mut().enumerate() {
            let mean: f64 = (0..rows).map(|row| matrix.row(row)[column]).sum::<f64>() / rows as f64;
            *slot = (0..rows)
                .map(|row| {
                    let d = matrix.row(row)[column] - mean;
                    d * d
                })
                .sum::<f64>()
                / rows as f64;
        }
        let sum: f64 = variance.iter().sum();
        let sum_sq: f64 = variance.iter().map(|v| v * v).sum();
        if sum_sq <= 0.0 {
            return 0.0;
        }
        sum * sum / sum_sq
    }

    /// The mean cosine between each row and the nearest other row.
    fn mean_nearest_cosine(matrix: &VectorMatrix) -> f64 {
        let rows = matrix.rows();
        let mut total = 0.0;
        for a in 0..rows {
            let mut best = -1.0_f64;
            for b in 0..rows {
                if a == b {
                    continue;
                }
                let dot: f64 = matrix
                    .row(a)
                    .iter()
                    .zip(matrix.row(b))
                    .map(|(x, y)| x * y)
                    .sum();
                best = best.max(dot);
            }
            total += best;
        }
        total / rows as f64
    }

    #[test]
    fn the_effective_dimension_is_far_below_the_nominal_width() {
        // The claim the module is built on, measured rather than asserted. A uniform corpus
        // at this width would score near the width itself.
        let matrix =
            embedding_like(CorpusShape::embedding_like(512, 512), 0x5EED).expect("generates");
        let d_eff = effective_dimension(&matrix);
        assert!(
            d_eff < 64.0,
            "a Matryoshka-shaped spectrum must concentrate variance; d_eff = {d_eff}"
        );
        assert!(
            d_eff > 1.0,
            "and it must not collapse to a single direction; d_eff = {d_eff}"
        );
    }

    #[test]
    fn a_uniform_corpus_scores_the_width_and_this_one_does_not() {
        // The control. Without it, the bound above could be satisfied by any generator at
        // all and would say nothing about the shaping.
        let dims = 256;
        let mut stream = Stream::new(0x00C0_FFEE);
        let mut data = Vec::with_capacity(512 * dims);
        for _ in 0..512 {
            let mut row: Vec<f64> = (0..dims).map(|_| stream.unit()).collect();
            normalize(&mut row);
            data.extend_from_slice(&row);
        }
        let uniform = VectorMatrix::new(512, dims, data).expect("valid");
        let shaped =
            embedding_like(CorpusShape::embedding_like(512, dims), 0x5EED).expect("generates");

        let uniform_eff = effective_dimension(&uniform);
        let shaped_eff = effective_dimension(&shaped);
        assert!(
            uniform_eff > shaped_eff * 4.0,
            "uniform d_eff {uniform_eff} should dwarf the shaped corpus's {shaped_eff}"
        );
    }

    #[test]
    fn neighbours_are_separated_rather_than_concentrated() {
        // The consequence that matters for an index: a nearest neighbour must actually be
        // near. Under concentration every pair looks alike and this number collapses toward
        // the corpus mean.
        let shaped =
            embedding_like(CorpusShape::embedding_like(256, 256), 0xBEEF).expect("generates");
        let cosine = mean_nearest_cosine(&shaped);
        assert!(
            cosine > 0.5,
            "a structured corpus must have genuinely near neighbours; mean = {cosine}"
        );
    }

    #[test]
    fn the_corpus_is_a_pure_function_of_its_seed() {
        let shape = CorpusShape::embedding_like(64, 48);
        let first = embedding_like(shape, 0xA11CE).expect("generates");
        let second = embedding_like(shape, 0xA11CE).expect("generates");
        assert_eq!(first, second);
        let other = embedding_like(shape, 0xA11CF).expect("generates");
        assert_ne!(
            first, other,
            "a different seed must give a different corpus"
        );
    }

    #[test]
    fn cluster_sizes_follow_a_power_law() {
        let clusters = 16;
        let mut counts = vec![0_usize; clusters];
        let mut stream = Stream::new(0xD15C0);
        let (weights, total) = cluster_weights(clusters);
        for _ in 0..10_000 {
            counts[cluster_of(stream.next_bits(), &weights, total)] += 1;
        }
        assert!(
            counts[0] > counts[clusters - 1] * 4,
            "the head must dwarf the tail: {counts:?}"
        );
        assert!(
            counts.iter().all(|c| *c > 0),
            "and the tail must still be populated: {counts:?}"
        );
    }

    #[test]
    fn separated_manifolds_do_not_share_an_axis() {
        let matrix = separated_manifolds(64, 32, 4, 0x11).expect("generates");
        // Rows from different manifolds are orthogonal: their supports are disjoint.
        let dot: f64 = matrix
            .row(0)
            .iter()
            .zip(matrix.row(1))
            .map(|(x, y)| x * y)
            .sum();
        assert_eq!(dot, 0.0, "adjacent rows sit on different manifolds");
    }

    #[test]
    fn extreme_magnitudes_stay_finite_and_rankable() {
        let matrix = extreme_but_finite(32, 16, 0x22).expect("generates");
        assert!(
            matrix.as_slice().iter().all(|v| v.is_finite()),
            "every component must be finite"
        );
        for row in 0..matrix.rows() {
            let norm: f64 = matrix.row(row).iter().map(|v| v * v).sum();
            assert!(norm > 0.0, "row {row} must have a positive norm");
        }
    }
}
