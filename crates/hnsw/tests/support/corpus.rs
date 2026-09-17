// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Deterministic corpora with the geometry real embeddings have.
//!
//! This is a HARNESS, not part of the crate's published surface. It generates the corpora the
//! benches and the invariant suites measure over, and it is shared by `#[path]` include so
//! that every one of them draws from the SAME generator -- on a crate whose central finding
//! is that the distribution decides the recall, two harnesses generating two distributions
//! would be measuring two different questions and reporting one number.
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
//! 4. **Cluster tightness you can ask for, and get.** Members are placed at an *intended
//!    cosine* `ρ` to their centroid: `v = ρ·c + sqrt(1 - ρ²)·u`. The alternative — isotropic
//!    noise at a fixed amplitude `σ` — fails in two different ways depending on where the
//!    noise is added, and BOTH are measured in `tests/corpus_geometry.rs` rather than argued
//!    here:
//!
//!    * Added in the **ambient** space, the achieved cosine tracks the width. Its expectation
//!      is `1 / sqrt(1 + d·σ²·var)`, where `var` is the variance of the noise draw — `1/3`
//!      for [`Stream::unit`], which is uniform on `[-1, 1)` rather than the unit-variance
//!      draw the textbook form assumes. One amplitude measures 0.89 at `d = 64` and 0.24 at
//!      `d = 4096`: members of the same nominal tightness end up nearly orthogonal to their
//!      centroid as the width grows.
//!    * Added in the **latent** space, tightness does NOT track the width — the latent
//!      dimension is fixed, so there is no `d` to grow. What is lost instead is control: the
//!      achieved cosine is an emergent 0.94 that no caller asked for, and it moves with
//!      `intrinsic` and `σ` rather than with the one number the caller cares about.
//!
//!    So width-invariance alone is not the argument for this parameterisation, and it would
//!    be dishonest to present it as one. The argument is that `ρ` is the achieved cosine, at
//!    every width, without retuning.
//!
//! The unit tests measure `d_eff` from the generated vectors, but by the **diagonal** of
//! the covariance rather than its spectrum -- see `effective_dimension` in the test module.
//! For an axis-aligned generator like this one the two agree closely; the distinction is
//! recorded because the tests would otherwise read as verifying the spectral claim for any
//! corpus, which they do not.
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

// Included by `#[path]` into several test and bench binaries, each of which uses a different
// subset. Matches the sibling `support/purremb.rs`.
#![allow(dead_code, unreachable_pub)]

use purrdf_hnsw::level::splitmix64;
use purrdf_hnsw::{Result, VectorMatrix};

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

/// The declared scale ladder BOTH bench harnesses run, as `(rows, dims)`.
///
/// ONE table, because the two harnesses read ONE environment variable. They did not share
/// it: `benches/build.rs` declared 5,000 upward and `benches/recall.rs` declared 2,048
/// upward, so `PURRDF_HNSW_BENCH_SCALES=5000` narrowed one harness and hard-panicked the
/// other, and the README's own narrowing example was therefore a command that fails. A
/// variable with two vocabularies is a variable whose documented value is correct for
/// whichever harness the writer happened to run.
pub const LADDER: [(usize, usize); 4] = [
    (5_000, 4_096),
    (50_000, 4_096),
    (200_000, 4_096),
    (1_000_000, 4_096),
];

/// [`LADDER`], narrowed to the row counts `PURRDF_HNSW_BENCH_SCALES` names.
///
/// Every declared rung runs by default: a point that is not measured is a point these
/// harnesses do not claim, and a default that skips the one scale the offer is about is how
/// a missing measurement gets reported as a completed run.
///
/// The knob can only take rungs AWAY. That is ENFORCED here rather than promised in prose:
/// the parsed list is filtered against [`LADDER`], and a row count naming no rung is a hard
/// failure. `build.rs` previously documented the guarantee in three places while returning
/// the parsed list verbatim, so `PURRDF_HNSW_BENCH_SCALES=64` printed a `rows 64` row that
/// the declared ladder never contained. A knob that can widen a run is a knob that can hide
/// a default that was never wide enough, and a knob whose one-way property lives only in a
/// doc comment is not one-way.
pub fn declared_scales() -> Vec<(usize, usize)> {
    let Some(spec) = std::env::var_os("PURRDF_HNSW_BENCH_SCALES") else {
        return LADDER.to_vec();
    };
    let spec = spec.to_string_lossy();
    let wanted: Vec<usize> = spec
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| {
            part.parse::<usize>()
                .unwrap_or_else(|_| panic!("PURRDF_HNSW_BENCH_SCALES: {part:?} is not a row count"))
        })
        .collect();
    let declared: Vec<usize> = LADDER.iter().map(|(rows, _)| *rows).collect();
    assert!(
        wanted.iter().all(|rows| declared.contains(rows)),
        "PURRDF_HNSW_BENCH_SCALES named {wanted:?}, which is not a subset of the declared \
         ladder {declared:?}; the knob narrows that ladder and cannot add a rung to it"
    );
    let chosen: Vec<(usize, usize)> = LADDER
        .into_iter()
        .filter(|(rows, _)| wanted.contains(rows))
        .collect();
    assert!(
        !chosen.is_empty(),
        "PURRDF_HNSW_BENCH_SCALES named no rung of the declared ladder {LADDER:?}"
    );
    chosen
}

/// A deterministic stream of values in `[-1, 1)`, never exactly zero.
pub(crate) struct Stream(u64);

impl Stream {
    pub(crate) const fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub(crate) fn next_bits(&mut self) -> u64 {
        self.0 = splitmix64(self.0);
        self.0
    }

    /// A value in `[-1, 1)`, never exactly zero so a norm cannot collapse.
    pub(crate) fn unit(&mut self) -> f64 {
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
pub(crate) fn normalize(values: &mut [f64]) {
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
pub(crate) fn cluster_of(bits: u64, weights: &[u64], total: u64) -> usize {
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
pub(crate) fn cluster_weights(clusters: usize) -> (Vec<u64>, u64) {
    let weights: Vec<u64> = (0..clusters)
        .map(|c| (clusters as u64).div_ceil(c as u64 + 1))
        .collect();
    let total = weights.iter().sum();
    (weights, total)
}

/// Carry one latent point into the ambient space: project, apply the spectrum, renormalise.
///
/// Extracted so a ROW and a CENTROID travel the identical path. A cosine is only a statement
/// about the generated geometry if both of its arguments were mapped the same way; measuring
/// a row in the ambient space against a centroid in the latent space would compare two
/// different spaces and report a number about the mismatch.
fn to_ambient(point: &[f64], projection: &[Vec<f64>], scale: &[f64], dims: usize) -> Vec<f64> {
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
    ambient
}

/// A generated corpus together with the latent structure that produced it.
///
/// The generator's geometric claims are claims about the RELATIONSHIP between a row and the
/// centroid it was drawn around, and a caller handed only the matrix cannot check any of them:
/// the cluster assignment and the centroids are consumed inside the generator and thrown away.
/// So the lane that grades this generator was reduced to measuring a nearest-neighbour cosine,
/// which the intended-cosine parameter barely moves -- driving it from 0.75 to 0.001, from
/// shaped to formless, changed that statistic by 0.13 and crossed no assertion.
///
/// This type is what makes the central claim gradeable: the same rows, plus which cluster each
/// was drawn around, plus every centroid carried into the ambient space through the SAME
/// projection and spectrum the rows went through.
pub struct Structured {
    /// The corpus, identical to what [`embedding_like`] returns for the same arguments.
    pub matrix: VectorMatrix,
    /// The cluster each row was drawn around, by row index.
    pub cluster: Vec<usize>,
    /// Each cluster's centroid, in the ambient space, one per row.
    pub centroids: VectorMatrix,
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
    Ok(embedding_like_structured(shape, seed)?.matrix)
}

/// [`embedding_like`], keeping the latent structure instead of discarding it.
///
/// Byte-identical corpora: the rows are produced by the same draws in the same order, and the
/// centroids are mapped AFTER the row loop from values the generator already held, so no
/// additional value is taken from the stream. Every pinned digest and recall golden in this
/// crate is computed over the output of this function.
///
/// # Errors
///
/// As [`embedding_like`].
///
/// # Panics
///
/// As [`embedding_like`].
pub fn embedding_like_structured(shape: CorpusShape, seed: u64) -> Result<Structured> {
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
    let mut membership = Vec::with_capacity(rows);
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
        data.extend_from_slice(&to_ambient(&point, &projection, &scale, dims));
        membership.push(cluster);
    }

    // The centroids through the identical map. No draw is taken here -- these are values the
    // generator already holds -- so the corpus is unchanged by the fact that we keep them.
    let mut centroid_data = Vec::with_capacity(clusters * dims);
    for centroid in &centroids {
        centroid_data.extend_from_slice(&to_ambient(centroid, &projection, &scale, dims));
    }

    Ok(Structured {
        matrix: VectorMatrix::new(rows, dims, data)?,
        cluster: membership,
        centroids: VectorMatrix::new(clusters, dims, centroid_data)?,
    })
}

/// The rejected parameterisation applied where this generator ACTUALLY mixes: the latent space.
///
/// The near counterfactual, and the one that is easy to get wrong. [`embedding_like_structured`]
/// forms its member in a latent space of `shape.intrinsic` dimensions and only then projects;
/// so the literal one-line alternative to `v = ρ·c + sqrt(1-ρ²)·u` is `v = c + σ·n` **in that
/// same latent space**, with everything else — the projection, the spectrum, the renormalisation,
/// the cluster law, the draw order — held identical. Every other line here is copied from the
/// shipped path deliberately: a control that differs in a second place cannot attribute what it
/// measures to the first.
///
/// `shape.within_cluster_cosine` is ignored; `sigma` replaces it.
///
/// # Errors
///
/// As [`embedding_like_structured`].
pub fn latent_fixed_amplitude(shape: CorpusShape, sigma: f64, seed: u64) -> Result<Structured> {
    let CorpusShape {
        rows,
        dims,
        intrinsic,
        clusters,
        ..
    } = shape;
    let latent = intrinsic.max(1).min(dims);
    let clusters = clusters.max(1);

    let mut stream = Stream::new(seed);
    let projection: Vec<Vec<f64>> = (0..latent).map(|_| stream.direction(dims)).collect();
    #[expect(
        clippy::cast_precision_loss,
        reason = "a coordinate index is far inside a 53-bit mantissa"
    )]
    let scale: Vec<f64> = (0..dims).map(|j| 1.0 / ((j as f64) + 1.0).sqrt()).collect();
    let centroids: Vec<Vec<f64>> = (0..clusters).map(|_| stream.direction(latent)).collect();
    let (weights, weight_total) = cluster_weights(clusters);

    let mut data = Vec::with_capacity(rows * dims);
    let mut membership = Vec::with_capacity(rows);
    for _ in 0..rows {
        let cluster = cluster_of(stream.next_bits(), &weights, weight_total);

        // The ONE line that differs from the shipped generator: a fixed amplitude rather than
        // an intended cosine. The same number of draws is taken either way, so the two paths
        // stay aligned on the stream and a difference in the output is a difference in the
        // mix rather than in the sampling.
        let mut point: Vec<f64> = (0..latent)
            .map(|axis| stream.unit().mul_add(sigma, centroids[cluster][axis]))
            .collect();
        normalize(&mut point);

        data.extend_from_slice(&to_ambient(&point, &projection, &scale, dims));
        membership.push(cluster);
    }

    let mut centroid_data = Vec::with_capacity(clusters * dims);
    for centroid in &centroids {
        centroid_data.extend_from_slice(&to_ambient(centroid, &projection, &scale, dims));
    }

    Ok(Structured {
        matrix: VectorMatrix::new(rows, dims, data)?,
        cluster: membership,
        centroids: VectorMatrix::new(clusters, dims, centroid_data)?,
    })
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
