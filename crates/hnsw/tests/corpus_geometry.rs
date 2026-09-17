// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The corpus generator's own geometry, measured rather than asserted.
//!
//! These check the properties the generator exists to provide -- a low effective dimension,
//! neighbours that are genuinely near, power-law cluster sizes -- from the vectors it
//! produces rather than from the parameters that produced them.

#[path = "support/corpus.rs"]
mod corpus;

use corpus::{
    CorpusShape, Stream, Structured, cluster_of, cluster_weights, embedding_like,
    embedding_like_structured, extreme_but_finite, normalize, separated_manifolds,
};
use purrdf_hnsw::VectorMatrix;

/// The inverse participation ratio of a corpus's per-coordinate variances.
///
/// Computed from the generated vectors rather than from the parameters that produced
/// them, so it measures the corpus rather than restating the generator.
///
/// **This is the DIAGONAL proxy, not the spectral quantity.** The module docs define
/// `d_eff` over the covariance *spectrum*; this folds the diagonal of the covariance,
/// and the two coincide only when the covariance is near-diagonal. For this generator
/// that holds well enough to be fair -- the structure is axis-aligned by construction,
/// since the spectrum is imposed by scaling ambient coordinate `j` -- but row
/// normalization does couple coordinates, so the agreement is an approximation and not
/// an identity. A reader should not take a bound on this number as a verified bound on
/// the spectral `d_eff` in general.
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
    let matrix = embedding_like(CorpusShape::embedding_like(512, 512), 0x5EED).expect("generates");
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
    let shaped = embedding_like(CorpusShape::embedding_like(512, dims), 0x5EED).expect("generates");

    let uniform_eff = effective_dimension(&uniform);
    let shaped_eff = effective_dimension(&shaped);

    // The absolute half of the claim, which a ratio alone does not make: i.i.d.
    // coordinates have equal variances, so their inverse participation ratio is the
    // width itself, up to sampling noise in the variance estimates: measured 255.66
    // against a nominal 256 here. The bound is deliberately loose against that, so it
    // cannot flake on sampling noise while still catching a real collapse. Without it a
    // regression that dropped uniform_eff from ~256 to 40 would still satisfy the ratio
    // below, and this test's name would be asserting something nothing checked.
    assert!(
        uniform_eff > dims as f64 * 0.5,
        "a uniform corpus must score near its nominal width {dims}; got {uniform_eff}"
    );
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
    let shaped = embedding_like(CorpusShape::embedding_like(256, 256), 0xBEEF).expect("generates");
    let cosine = mean_nearest_cosine(&shaped);
    assert!(
        cosine > 0.5,
        "a structured corpus must have genuinely near neighbours; mean = {cosine}"
    );
}

/// The mean cosine between each row and the centroid it was actually drawn around.
///
/// Both arguments are unit-norm ambient vectors -- the generator renormalises a row and a
/// centroid through the same map -- so the dot product IS the cosine, with no division to
/// introduce a second rounding.
fn mean_centroid_cosine(structured: &Structured) -> f64 {
    let matrix = &structured.matrix;
    let total: f64 = (0..matrix.rows())
        .map(|row| {
            let centroid = structured.centroids.row(structured.cluster[row]);
            matrix
                .row(row)
                .iter()
                .zip(centroid)
                .map(|(x, y)| x * y)
                .sum::<f64>()
        })
        .sum();
    total / matrix.rows() as f64
}

/// A measured cosine as an exact integer at six decimals.
///
/// Pinned rather than bounded, on this repository's rule that a difference is a real defect
/// and not a tolerance to widen. The generator is a pure function of its seed over add,
/// multiply, divide and `sqrt` only, so the sixth decimal is reproducible; the quantity being
/// pinned moves by tenths when the geometry changes, so the pin is nowhere near its own
/// rounding boundary.
fn pinned(cosine: f64) -> i64 {
    (cosine * 1e6).round() as i64
}

/// The shape the tightness tests vary, with everything but the cosine held fixed.
fn tightness_shape(dims: usize, cosine: f64) -> CorpusShape {
    CorpusShape {
        rows: 512,
        dims,
        intrinsic: 32,
        clusters: 64,
        within_cluster_cosine: cosine,
    }
}

#[test]
fn the_intended_cluster_cosine_is_the_cosine_the_corpus_achieves() {
    // THE central parameter, and until this test existed nothing graded it. The generator's
    // whole reason to exist is that cluster tightness is specified as an intended COSINE
    // rather than as an absolute noise amplitude; a corpus whose achieved tightness ignored
    // that parameter would be formless, every recall number taken over it would be a
    // statement about the fixture, and the lane would have stayed green. Driving the
    // parameter from 0.75 to 0.001 moved the nearest-neighbour statistic this file already
    // measured by 0.13 and crossed none of its assertions.
    //
    // Measured against the centroid each row was DRAWN AROUND, which is the relationship the
    // parameter actually names, and pinned exactly at each rung so the whole curve is held.
    let intended = [0.05, 0.35, 0.75, 0.95];
    let achieved: Vec<i64> = intended
        .into_iter()
        .map(|cosine| {
            let structured = embedding_like_structured(tightness_shape(256, cosine), 0xC051_5EED)
                .expect("generates");
            pinned(mean_centroid_cosine(&structured))
        })
        .collect();

    assert_eq!(
        achieved,
        vec![61_455, 351_538, 751_847, 951_277],
        "the achieved within-cluster cosine moved; it is a pure function of the generator, \
         so this is a real change in the corpus every recall figure is measured over"
    );

    // The claim itself, stated as a claim rather than as a pin: the parameter is an INTENDED
    // cosine, so the corpus must actually achieve it. Every rung lands within 0.012 of what
    // it asked for; the bound is what separates "honoured" from "correlated with".
    for (asked, got) in intended.into_iter().zip(&achieved) {
        let got = *got as f64 / 1e6;
        assert!(
            (got - asked).abs() < 0.02,
            "a corpus asked for an intended cosine of {asked} achieved {got}"
        );
    }
    assert!(
        achieved.windows(2).all(|pair| pair[0] < pair[1]),
        "and a higher intended cosine must give a tighter corpus: {achieved:?}"
    );

    // The ladder above grades the MECHANISM, over shapes this test builds itself. That is not
    // sufficient, and the difference is the whole finding: the corpus every recall figure in
    // this crate is measured over comes from `CorpusShape::embedding_like`, whose tightness is
    // a default this test would never touch. A default quietly moved toward formless would
    // leave the ladder green and turn every recall number into a statement about the fixture.
    // So the shipped default is pinned too, by the same measurement.
    let default = embedding_like_structured(CorpusShape::embedding_like(512, 256), 0xC051_5EED)
        .expect("generates");
    assert_eq!(
        pinned(mean_centroid_cosine(&default)),
        751_847,
        "the DEFAULT corpus shape's achieved tightness moved; every recall and build figure \
         this crate reports is measured over this shape"
    );
}

#[test]
fn cluster_tightness_is_the_same_at_every_width() {
    // The half of the claim that is about WIDTH, and the reason the parameter is a cosine at
    // all. The module documents the alternative it rejects: isotropic noise of a fixed
    // amplitude gives an expected cosine of 1/sqrt(1 + d*sigma^2), so an amplitude tuned to
    // make tight clusters at d = 64 drives the cosine to about 0.077 at d = 4096 -- members
    // essentially orthogonal to the centroid they were supposedly drawn around, a corpus that
    // looks clustered in its source and is indistinguishable from uniform in its output.
    //
    // That failure is invisible to a test taken at ONE width, which is what every geometry
    // test here was. Three widths spanning a factor of sixteen, one intended cosine.
    let achieved: Vec<i64> = [256, 1024, 4096]
        .into_iter()
        .map(|dims| {
            let structured = embedding_like_structured(tightness_shape(dims, 0.75), 0xC051_5EED)
                .expect("generates");
            pinned(mean_centroid_cosine(&structured))
        })
        .collect();

    assert_eq!(
        achieved,
        vec![751_847, 742_942, 743_679],
        "the achieved within-cluster cosine moved at one or more widths"
    );

    // Measured span: 0.0089 across widths 256 to 4,096, against an intended 0.75. The
    // rejected parameterisation would have fallen to about 0.077 at the top width -- a span
    // of roughly 0.67 -- so this bound sits two orders of magnitude inside the failure it
    // exists to catch, and is not a tolerance widened to fit a measurement.
    let (low, high) = (
        *achieved.iter().min().expect("three widths"),
        *achieved.iter().max().expect("three widths"),
    );
    assert!(
        high - low < 20_000,
        "the achieved cosine must not track the width: it spans {low}..{high} at six \
         decimals across a 16x range of widths, which is the width-dependence this \
         parameterisation exists to avoid"
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
