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

/// The widths the two tightness tests span, and the bound they hold the span to.
///
/// A factor of sixteen, which is wide enough that a width-dependent parameterisation has
/// nowhere to hide: see `the_rejected_parameterisation_really_is_width_dependent`, which
/// measures what this range does to the alternative.
const WIDTHS: [usize; 3] = [256, 1024, 4096];

/// The largest span, at six decimals, the achieved cosine may show across [`WIDTHS`].
const WIDTH_SPAN_BOUND: i64 = 20_000;

/// The rejected parameterisation: isotropic noise at a fixed per-component amplitude.
///
/// Implemented HERE, in the test file, rather than described in a comment. The module doc
/// rejects this form and gives a closed form for why -- an expected cosine of
/// `1/sqrt(1 + d*sigma^2)`, which falls as the width grows. A comment quoting that formula is
/// not evidence: the formula is about noise added in a bare ambient space, while this
/// generator projects, applies a spectrum and renormalises, all of which partially preserve
/// the centroid direction and make the closed form a substantial underestimate of the
/// achieved cosine. The only way to know what this corpus family would do under the rejected
/// form is to build it and measure.
///
/// `sigma` is the per-component noise amplitude, in the same ambient space the centroid lives
/// in, so the `d` of the closed form really is `dims`.
fn fixed_amplitude(dims: usize, sigma: f64, seed: u64) -> Structured {
    const ROWS: usize = 512;
    const CLUSTERS: usize = 64;
    let mut stream = Stream::new(seed);

    let mut centroid_data = Vec::with_capacity(CLUSTERS * dims);
    let mut centroids = Vec::with_capacity(CLUSTERS);
    for _ in 0..CLUSTERS {
        let mut direction: Vec<f64> = (0..dims).map(|_| stream.unit()).collect();
        normalize(&mut direction);
        centroid_data.extend_from_slice(&direction);
        centroids.push(direction);
    }

    let mut data = Vec::with_capacity(ROWS * dims);
    let mut cluster = Vec::with_capacity(ROWS);
    for row in 0..ROWS {
        let which = row % CLUSTERS;
        let mut point: Vec<f64> = centroids[which]
            .iter()
            .map(|value| stream.unit().mul_add(sigma, *value))
            .collect();
        normalize(&mut point);
        data.extend_from_slice(&point);
        cluster.push(which);
    }

    Structured {
        matrix: VectorMatrix::new(ROWS, dims, data).expect("valid"),
        cluster,
        centroids: VectorMatrix::new(CLUSTERS, dims, centroid_data).expect("valid"),
    }
}

#[test]
fn the_rejected_parameterisation_really_is_width_dependent() {
    // The CONTROL for the invariance test below, and the reason its bound can be called tight
    // rather than merely satisfied. Without this, "the achieved cosine does not track the
    // width" is a sentence with nothing to contrast against: a bound is only meaningful
    // beside the failure it excludes, and that failure has to be measured, not quoted from a
    // formula derived for a different construction.
    //
    // The amplitude is tuned so this form produces tight clusters at a NARROW width -- the
    // tuning the module doc names as the trap, because it is what an author would naturally
    // do while developing against small fixtures.
    const SIGMA: f64 = 0.110_24;
    let achieved: Vec<i64> = WIDTHS
        .into_iter()
        .map(|dims| {
            pinned(mean_centroid_cosine(&fixed_amplitude(
                dims, SIGMA, 0xBAD_C051,
            )))
        })
        .collect();

    // Measured: 0.700 at 256, 0.443 at 1,024, 0.239 at 4,096 -- the achieved tightness falls
    // by two thirds across the range, which is the whole defect. Note how far these sit from
    // the closed form's 0.493 / 0.273 / 0.140 at the same amplitude: the projection and the
    // renormalisation preserve much more of the centroid direction than the bare-ambient
    // formula predicts, so quoting the formula would have UNDERSTATED the achieved cosine by
    // about 1.7x at every width. The formula names the shape of the failure; only the
    // measurement gives its size.
    assert_eq!(
        achieved,
        vec![700_236, 442_590, 239_452],
        "the rejected parameterisation's measured width-dependence moved"
    );

    let span =
        achieved.iter().max().expect("three widths") - achieved.iter().min().expect("three widths");
    assert!(
        span > WIDTH_SPAN_BOUND,
        "the rejected form must FAIL the invariance bound this file holds the real \
         generator to, or that bound proves nothing: span {span} against a bound of \
         {WIDTH_SPAN_BOUND}"
    );
}

#[test]
fn cluster_tightness_is_the_same_at_every_width() {
    // The half of the claim that is about WIDTH, and the reason the parameter is a cosine at
    // all. The alternative the module rejects -- isotropic noise at a fixed amplitude -- makes
    // members progressively more orthogonal to the centroid they were supposedly drawn around
    // as the width grows, producing a corpus that looks clustered in its source and is
    // indistinguishable from uniform in its output. That failure is measured directly by
    // `the_rejected_parameterisation_really_is_width_dependent` rather than asserted here.
    //
    // The failure is invisible to a test taken at ONE width, which is what every geometry
    // test in this file was. Three widths spanning a factor of sixteen, one intended cosine.
    let achieved: Vec<i64> = WIDTHS
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

    // The bound is not a tolerance widened to fit a measurement, and that is now a measured
    // statement rather than a claim: the control above runs the rejected parameterisation over
    // the same widths and the same statistic, and its span must EXCEED this bound for that
    // test to pass. So the two tests bracket the bound from both sides -- the real generator
    // must come in under it, the rejected one must not -- and neither side is a number anybody
    // typed into a comment.
    let (low, high) = (
        *achieved.iter().min().expect("three widths"),
        *achieved.iter().max().expect("three widths"),
    );
    assert!(
        high - low < WIDTH_SPAN_BOUND,
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
