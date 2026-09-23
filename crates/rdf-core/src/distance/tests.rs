// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The exact law, held to a scalar reference model on every dispatch path this host can
//! execute.
//!
//! The asm evidence gate can show that an exact kernel has no fused multiply-add; it
//! cannot show the order the kernel adds in. That is proven here, and only here: a plain
//! nested-loop emulation of the lanes, the tree and the tail is the reference, every
//! path is compared with it bit for bit, and a neighbour oracle shows the reference can
//! tell the lane-tree order apart from every other plausible one.

use super::*;

/// Every path this host can execute, as resolved handles.
///
/// A handle is built directly only for a path the processor reports, which is the same
/// condition `dispatch::exact_path` checks.
fn host_paths() -> Vec<Resolved<Exact>> {
    let portable = Resolved::on(Path::Portable);
    #[cfg(target_arch = "x86_64")]
    {
        if std::is_x86_feature_detected!("avx2") {
            return vec![portable, Resolved::on(Path::Avx2)];
        }
    }
    vec![portable]
}

/// The names of `paths`, for the executed-paths line each test prints.
fn names(paths: &[Resolved<Exact>]) -> String {
    paths
        .iter()
        .map(|path| path.path().name())
        .collect::<Vec<_>>()
        .join(", ")
}

// ---- the reference model ---------------------------------------------------

/// The exact law over already-rounded terms, as plain nested loops: sixteen lanes by
/// index, the pairwise tree written out step by step, then the tail in ascending index.
fn reference_fold(terms: &[f64]) -> f64 {
    let chunks = terms.len() / 16;
    let mut lanes = [0.0_f64; 16];
    for chunk in 0..chunks {
        for (lane, slot) in lanes.iter_mut().enumerate() {
            *slot += terms[chunk * 16 + lane];
        }
    }
    let mut s8 = [0.0_f64; 8];
    for l in 0..8 {
        s8[l] = lanes[l] + lanes[l + 8];
    }
    let mut s4 = [0.0_f64; 4];
    for l in 0..4 {
        s4[l] = s8[l] + s8[l + 4];
    }
    let s2 = [s4[0] + s4[2], s4[1] + s4[3]];
    let mut sum = s2[0] + s2[1];
    for &term in &terms[chunks * 16..] {
        sum += term;
    }
    sum
}

/// The reference dot product: each product rounded once, then the reference fold.
pub(super) fn reference_dot(a: &[f64], b: &[f64]) -> f64 {
    let terms: Vec<f64> = a.iter().zip(b).map(|(x, y)| x * y).collect();
    reference_fold(&terms)
}

/// The reference squared Euclidean distance.
fn reference_squared_euclidean(a: &[f64], b: &[f64]) -> f64 {
    let terms: Vec<f64> = a
        .iter()
        .zip(b)
        .map(|(x, y)| {
            let delta = x - y;
            delta * delta
        })
        .collect();
    reference_fold(&terms)
}

/// The reference finished distance, or `None` when it is not finite.
fn reference_distance(
    measure: Measure,
    a: &[f64],
    a_norm: f64,
    b: &[f64],
    b_norm: f64,
) -> Option<f64> {
    let value = match measure {
        Measure::SquaredEuclidean => reference_squared_euclidean(a, b),
        Measure::NegativeDot => -reference_dot(a, b),
        Measure::Cosine => {
            let denominator = a_norm * b_norm;
            let quotient = reference_dot(a, b) / denominator;
            1.0 - quotient
        }
    };
    value.is_finite().then_some(value)
}

/// An optional distance as comparable bits, so `-0.0` and `+0.0` are told apart.
fn bits(value: Option<f64>) -> Option<u64> {
    value.map(f64::to_bits)
}

// ---- the fixture -----------------------------------------------------------

/// A seeded splitmix64 stream.
pub(super) struct Stream(pub(super) u64);

impl Stream {
    pub(super) fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A value in `[-1, 1)`.
    pub(super) fn signed(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64 / (1_u64 << 53) as f64).mul_add(2.0, -1.0)
    }

    /// One component of the adversarial class `class`.
    pub(super) fn component(&mut self, class: u64) -> f64 {
        match class {
            // Ordinary magnitudes.
            0 => self.signed(),
            // Exponents spread across 120 binades, so lanes and tail differ in scale.
            1 => {
                let exponent = i32::try_from(self.next_u64() % 121).expect("small") - 60;
                self.signed() * 2.0_f64.powi(exponent)
            }
            // Subnormals (and the odd zero), with either sign.
            2 => {
                let magnitude = f64::from_bits(self.next_u64() % (1_u64 << 52));
                if self.next_u64() & 1 == 1 {
                    -magnitude
                } else {
                    magnitude
                }
            }
            // Signed zeros among tiny normals.
            3 => match self.next_u64() % 4 {
                0 => 0.0,
                1 => -0.0,
                _ => self.signed() * 1e-300,
            },
            // Catastrophic cancellation: `1e16 + 1 - 1e16` depends on the order.
            _ => match self.next_u64() % 3 {
                0 => 1e16,
                1 => -1e16,
                _ => 1.0,
            },
        }
    }

    pub(super) fn vector(&mut self, len: usize) -> Vec<f64> {
        let class = self.next_u64() % 5;
        (0..len).map(|_| self.component(class)).collect()
    }
}

/// A storage width a fixture can be written at.
pub(super) trait Store: Scalar {
    fn store(value: f64) -> Self;
}

impl Store for f64 {
    fn store(value: f64) -> Self {
        value
    }
}

impl Store for f32 {
    fn store(value: f64) -> Self {
        value as Self
    }
}

/// `values` at width `T`, starting `offset` elements into a padded buffer so the
/// kernel sees every alignment the allocator does not promise.
pub(super) fn at_offset<T: Store>(values: &[f64], offset: usize) -> Vec<T> {
    let mut buffer: Vec<T> = (0..offset).map(|_| T::store(7.0)).collect();
    buffer.extend(values.iter().map(|&value| T::store(value)));
    buffer
}

pub(super) fn widened<T: Scalar>(values: &[T]) -> Vec<f64> {
    values.iter().map(|value| value.widen()).collect()
}

const MEASURES: [Measure; 3] = [
    Measure::SquaredEuclidean,
    Measure::NegativeDot,
    Measure::Cosine,
];

/// Every API of every path, over one matrix at one storage width pair, against the
/// reference model. Returns how many comparisons were made.
fn check_group<Q: Store, T: Store>(
    paths: &[Resolved<Exact>],
    query: &[f64],
    rows: &[Vec<f64>],
    offset: usize,
) -> usize {
    let dims = query.len();
    let query_buffer: Vec<Q> = at_offset(query, offset);
    let query_typed = &query_buffer[offset..];
    let flat: Vec<f64> = rows.iter().flatten().copied().collect();
    let row_buffer: Vec<T> = at_offset(&flat, offset);
    let row_typed = &row_buffer[offset..];

    // The reference reads exactly the values the kernel reads: the stored ones, widened.
    let query_wide = widened(query_typed);
    let rows_wide: Vec<Vec<f64>> = (0..rows.len())
        .map(|row| widened(&row_typed[row * dims..(row + 1) * dims]))
        .collect();
    let query_norm = norm(query_typed);
    let norms: Vec<f64> = rows_wide.iter().map(|row| norm(row)).collect();
    let view = RowsRef::new(row_typed, rows.len(), dims, &norms).expect("a well-shaped matrix");
    let ids: Vec<usize> = (0..rows.len()).rev().chain([0, rows.len() / 2]).collect();

    let mut compared = 0;
    for measure in MEASURES {
        let expected: Vec<Option<u64>> = rows_wide
            .iter()
            .zip(&norms)
            .map(|(row, &row_norm)| {
                bits(reference_distance(
                    measure,
                    &query_wide,
                    query_norm,
                    row,
                    row_norm,
                ))
            })
            .collect();
        for &path in paths {
            let mut out = vec![Some(f64::NAN); rows.len()];
            path.distances(measure, query_typed, query_norm, view, &mut out);
            let got: Vec<Option<u64>> = out.iter().copied().map(bits).collect();
            assert_eq!(
                got,
                expected,
                "{measure:?} batch on {} at len {dims}, offset {offset}",
                path.path()
            );

            let mut indexed = vec![Some(f64::NAN); ids.len()];
            path.distances_indexed(measure, query_typed, query_norm, view, &ids, &mut indexed);
            for (&id, value) in ids.iter().zip(&indexed) {
                assert_eq!(
                    bits(*value),
                    expected[id],
                    "{measure:?} indexed row {id} on {} at len {dims}, offset {offset}",
                    path.path()
                );
            }

            for (row, want) in expected.iter().enumerate() {
                let pair =
                    path.distance(measure, query_typed, query_norm, view.row(row), norms[row]);
                assert_eq!(
                    bits(pair),
                    *want,
                    "{measure:?} pair row {row} on {} at len {dims}, offset {offset}",
                    path.path()
                );
                let bounded = path.distance_bounded(
                    measure,
                    query_typed,
                    query_norm,
                    view.row(row),
                    norms[row],
                    Bound::Above(f64::INFINITY),
                );
                let bounded = match bounded {
                    Bounded::Below(value) => Some(value.to_bits()),
                    Bounded::NonFinite => None,
                    Bounded::Beyond => panic!("an infinite bound is never met"),
                };
                assert_eq!(
                    bounded,
                    *want,
                    "{measure:?} bounded row {row} on {} at len {dims}, offset {offset}",
                    path.path()
                );
                compared += 4;
            }
        }
        // The per-pair entry point without a handle is the portable compilation.
        for (row, want) in expected.iter().enumerate() {
            let pair = Exact::distance(measure, query_typed, query_norm, view.row(row), norms[row]);
            assert_eq!(bits(pair), *want, "{measure:?} Exact::distance row {row}");
        }
    }
    compared
}

#[test]
fn exact_paths_agree_bitwise() {
    let paths = host_paths();
    let lengths: Vec<usize> = (0..=70).chain([1_024, 4_096]).collect();
    const ROWS: usize = 34;
    let mut stream = Stream(0x5EED_D157_A11C_E000);
    let mut vectors = 0_usize;
    let mut compared = 0_usize;
    for &len in &lengths {
        for offset in 0..4 {
            let query = stream.vector(len);
            let rows: Vec<Vec<f64>> = (0..ROWS).map(|_| stream.vector(len)).collect();
            vectors += 1 + ROWS;
            compared += check_group::<f64, f64>(&paths, &query, &rows, offset);
            compared += check_group::<f64, f32>(&paths, &query, &rows, offset);
            compared += check_group::<f32, f32>(&paths, &query, &rows, offset);
            compared += check_group::<f32, f64>(&paths, &query, &rows, offset);
        }
    }
    assert!(vectors >= 10_000, "the fixture is {vectors} vectors");
    println!(
        "exact_paths_agree_bitwise executed paths: {} ({vectors} vectors, {compared} \
         comparisons against the reference model)",
        names(&paths)
    );
}

#[test]
fn order_is_the_lane_tree() {
    let paths = host_paths();
    let ones = [1.0_f64; 17];

    // One whole chunk: lane 0 holds 1e16, lane 8 holds -1e16, lane 1 holds 1.
    let mut chunk = [0.0_f64; 16];
    chunk[0] = 1e16;
    chunk[8] = -1e16;
    chunk[1] = 1.0;
    // The tree pairs lane 0 with lane 8 first, so the big terms cancel exactly and the
    // 1 survives.
    assert_eq!(reference_dot(&chunk, &ones[..16]), 1.0);
    // Three other orders a kernel could plausibly have, each computed here, each 0:
    // ascending sequential, lanes combined in ascending order, adjacent-pair tree.
    let mut sequential = 0.0_f64;
    for (x, y) in chunk.iter().zip(&ones) {
        let product = x * y;
        sequential += product;
    }
    let mut lane_order = 0.0_f64;
    for value in chunk {
        lane_order += value;
    }
    let adjacent = {
        let s8: Vec<f64> = chunk.chunks(2).map(|pair| pair[0] + pair[1]).collect();
        let s4: Vec<f64> = s8.chunks(2).map(|pair| pair[0] + pair[1]).collect();
        let s2: Vec<f64> = s4.chunks(2).map(|pair| pair[0] + pair[1]).collect();
        s2[0] + s2[1]
    };
    assert_eq!(sequential, 0.0);
    assert_eq!(lane_order, 0.0);
    assert_eq!(adjacent, 0.0);

    // The tail comes after the tree: 1e16 in lane 0, 1 in lane 1, -1e16 as the tail.
    // The tree rounds 1e16 + 1 back to 1e16 and the tail cancels it: 0. Folding the tail
    // into its lane before the tree would give 1.
    let mut tailed = [0.0_f64; 17];
    tailed[0] = 1e16;
    tailed[1] = 1.0;
    tailed[16] = -1e16;
    assert_eq!(reference_dot(&tailed, &ones), 0.0);
    let tail_into_lane = {
        let mut lanes = [0.0_f64; 16];
        for (index, value) in tailed.iter().enumerate() {
            lanes[index % 16] += value;
        }
        reference_fold(&lanes)
    };
    assert_eq!(
        tail_into_lane, 1.0,
        "the alternative order must genuinely differ"
    );

    for path in &paths {
        let dot = |a: &[f64], b: &[f64]| {
            -path
                .distance(Measure::NegativeDot, a, 0.0, b, 0.0)
                .expect("finite")
        };
        assert_eq!(
            dot(&chunk, &ones[..16]),
            1.0,
            "{}: the lane tree",
            path.path()
        );
        assert_ne!(dot(&chunk, &ones[..16]), sequential);
        assert_eq!(
            dot(&tailed, &ones),
            0.0,
            "{}: the tail after the tree",
            path.path()
        );
        assert_ne!(dot(&tailed, &ones), tail_into_lane);
    }
    println!("order_is_the_lane_tree executed paths: {}", names(&paths));
}

#[test]
fn bounded_matches_full() {
    let paths = host_paths();
    let mut stream = Stream(0xB0B0_0000_0000_0001);
    for len in [1_usize, 63, 64, 65, 127, 128, 200, 4_096] {
        let a: Vec<f64> = (0..len).map(|_| stream.signed()).collect();
        let b: Vec<f64> = (0..len).map(|_| stream.signed()).collect();
        let full = reference_squared_euclidean(&a, &b);
        assert!(full > 0.0 && full.is_finite());
        for path in &paths {
            let bounded =
                |bound| path.distance_bounded(Measure::SquaredEuclidean, &a, 0.0, &b, 0.0, bound);
            let below = Bounded::Below(full);
            assert_eq!(bounded(Bound::Above(f64::INFINITY)), below, "len {len}");
            // `Above` abandons only on a strictly greater sum.
            assert_eq!(
                bounded(Bound::Above(full)),
                below,
                "len {len}: equal is not above"
            );
            assert_eq!(
                bounded(Bound::Above(full.next_down())),
                Bounded::Beyond,
                "len {len}: one ulp under the distance"
            );
            // `AtOrAbove` abandons on equality.
            assert_eq!(
                bounded(Bound::AtOrAbove(full)),
                Bounded::Beyond,
                "len {len}: equal is at"
            );
            assert_eq!(
                bounded(Bound::AtOrAbove(full.next_up())),
                below,
                "len {len}: one ulp over the distance"
            );
        }
    }

    // A checkpoint really abandons: the first 64 terms already meet the bound, and a
    // non-finite term after them is never read. Without the checkpoint the fold would
    // reach it and report NonFinite; the neighbour below the checkpoint reaches it.
    let mut early = vec![1.0_f64; 128];
    early[100] = f64::NAN;
    let zero = vec![0.0_f64; 128];
    for path in &paths {
        assert_eq!(
            path.distance_bounded(
                Measure::SquaredEuclidean,
                &early,
                0.0,
                &zero,
                0.0,
                Bound::AtOrAbove(10.0)
            ),
            Bounded::Beyond,
            "{}: the 64-term checkpoint abandons",
            path.path()
        );
        assert_eq!(
            path.distance_bounded(
                Measure::SquaredEuclidean,
                &early,
                0.0,
                &zero,
                0.0,
                Bound::AtOrAbove(1e9)
            ),
            Bounded::NonFinite,
            "{}: a bound no checkpoint meets reaches the non-finite term",
            path.path()
        );
    }
    println!("bounded_matches_full executed paths: {}", names(&paths));
}

#[test]
fn nonfinite_checked_before_bound() {
    let paths = host_paths();
    for len in [8_usize, 64, 4_096] {
        // One overflowing lane (or tail term), everything else zero.
        let mut huge = vec![0.0_f64; len];
        huge[3] = f64::MAX / 2.0;
        huge[len - 1] = -f64::MAX / 2.0;
        let zero = vec![0.0_f64; len];
        for path in &paths {
            for bound in [
                Bound::Above(f64::INFINITY),
                // An infinity meets this bound, so a bound test that ran first would
                // report the overflow as merely far away.
                Bound::AtOrAbove(f64::INFINITY),
                Bound::AtOrAbove(1.0),
            ] {
                assert_eq!(
                    path.distance_bounded(Measure::SquaredEuclidean, &huge, 0.0, &zero, 0.0, bound),
                    Bounded::NonFinite,
                    "{} at len {len} under {bound:?}",
                    path.path()
                );
            }
            assert_eq!(
                path.distance(Measure::SquaredEuclidean, &huge, 0.0, &zero, 0.0),
                None,
                "{}: the full fold refuses it too",
                path.path()
            );

            // The valid neighbour: large but finite still ranks.
            let mut large = vec![0.0_f64; len];
            large[3] = 1e150;
            large[len - 1] = 1e150;
            let square = 1e150_f64 * 1e150_f64;
            let expected = reference_squared_euclidean(&large, &zero);
            assert!(expected.is_finite() && expected >= square);
            assert_eq!(
                path.distance_bounded(
                    Measure::SquaredEuclidean,
                    &large,
                    0.0,
                    &zero,
                    0.0,
                    Bound::AtOrAbove(f64::INFINITY)
                ),
                Bounded::Below(expected),
                "{} at len {len}: finite squares are ranked",
                path.path()
            );
        }
    }
    println!(
        "nonfinite_checked_before_bound executed paths: {}",
        names(&paths)
    );
}

// ---- identity ---------------------------------------------------------------

#[test]
fn the_exact_identity_is_pinned() {
    assert_eq!(Exact::ID, "binary64-lane16-tree-v1");
    assert_eq!(Exact::IMAGE_CODE, 1);
    assert_ne!(
        Exact::IMAGE_CODE,
        0,
        "zero was the reserved value of older images"
    );
    assert_eq!(EXACT_LANES, 16);
    for path in [Path::Portable, Path::Avx2] {
        assert_eq!(
            Exact::evidence(path),
            None,
            "the exact law discloses no divergence"
        );
    }
}

#[test]
fn a_misshapen_matrix_is_refused_and_a_well_shaped_one_is_not() {
    let data = [1.0_f64; 12];
    assert!(RowsRef::new(&data, 3, 4, &[]).is_some());
    assert!(RowsRef::new(&data, 3, 4, &[1.0, 2.0, 3.0]).is_some());
    assert!(RowsRef::new(&data, 4, 4, &[]).is_none(), "too few values");
    assert!(RowsRef::new(&data, 2, 4, &[]).is_none(), "too many values");
    assert!(
        RowsRef::new(&data, 3, 4, &[1.0]).is_none(),
        "one norm for three rows"
    );
    assert!(
        RowsRef::new(&data, usize::MAX, 2, &[]).is_none(),
        "an overflowing shape"
    );
    let empty: [f64; 0] = [];
    assert!(
        RowsRef::new(&empty, 5, 0, &[]).is_some(),
        "zero-dimensional rows exist"
    );
}

#[test]
fn family_contract_digest_unchanged_by_arithmetic() {
    use crate::{
        AppliedStage, ArtifactIdentity, ArtifactIdentityKind, ContentDigest, DimensionalityPolicy,
        DistanceMetric, EmbeddingFamilyContract, PrefixPostprocessing, StageImplementation,
        VectorDtype,
    };

    fn identity(name: &str) -> ArtifactIdentity {
        ArtifactIdentity::new(
            format!("https://example.org/{name}"),
            "application/octet-stream",
            ContentDigest::of(name.as_bytes()),
            None,
            ArtifactIdentityKind::Single,
        )
        .expect("artifact identity")
    }
    fn stage(name: &str) -> AppliedStage {
        AppliedStage::Applied(
            StageImplementation::new(
                format!("https://example.org/{name}"),
                ContentDigest::of(name.as_bytes()),
                "application/octet-stream",
                vec![1],
            )
            .expect("stage"),
        )
    }
    let contract = EmbeddingFamilyContract {
        model: identity("model"),
        engine: identity("engine"),
        tokenizer: identity("tokenizer"),
        execution: stage("execution"),
        subject_projection: stage("projection"),
        preprocessing: AppliedStage::NotApplied,
        chunking: AppliedStage::NotApplied,
        pooling: stage("pooling"),
        normalization: AppliedStage::NotApplied,
        truncation: AppliedStage::NotApplied,
        dtype: VectorDtype::F32,
        metric: DistanceMetric::SquaredEuclidean,
        dimensionality: DimensionalityPolicy::fixed(64, PrefixPostprocessing::None)
            .expect("fixed dimensions"),
        extensions: Vec::new(),
    };
    let family = contract.derive().expect("family");

    // The metric names WHAT is measured; the arithmetic names HOW the number is rounded.
    // PURREMB lets a kernel optimize evaluation while preserving the metric, so the family
    // contract carries no arithmetic and every arithmetic reads the same digest. Pinned,
    // so a change that folded an arithmetic into the metric would move it.
    const PINNED: &str = "fe2d8e7942175db980a8c0ac308cf2d69077f74012c03d7ecf1cf3b282fa7333";
    let arithmetic_ids = [Exact::ID, Reassociated::ID];
    for id in arithmetic_ids {
        assert!(
            !family
                .canonical_contract
                .windows(id.len())
                .any(|window| window == id.as_bytes()),
            "the family contract must not name the arithmetic {id}"
        );
        assert_eq!(
            family.contract_digest.to_hex(),
            PINNED,
            "the family-contract digest under the {id} arithmetic"
        );
    }
}

// ---- the float environment ---------------------------------------------------

#[test]
fn default_float_environment_resolves() {
    let resolved = Exact::resolve().expect("the default environment is the IEEE one");
    assert!(
        host_paths().contains(&resolved),
        "the resolved path {} is one this host runs",
        resolved.path()
    );
    #[cfg(target_arch = "x86_64")]
    {
        let expected = if std::is_x86_feature_detected!("avx2") {
            Path::Avx2
        } else {
            Path::Portable
        };
        assert_eq!(
            resolved.path(),
            expected,
            "resolution picks the widest exact path"
        );
    }
    #[cfg(not(target_arch = "x86_64"))]
    assert_eq!(resolved.path(), Path::Portable);
    println!(
        "default_float_environment_resolves resolved path: {}",
        resolved.path()
    );
}

/// Load `value` into the current thread's MXCSR.
#[cfg(target_arch = "x86_64")]
fn set_mxcsr(value: u32) {
    // SAFETY: `ldmxcsr` loads MXCSR from a live, aligned `u32`. The values loaded here
    // differ from the saved register only in the FTZ, DAZ and rounding-control fields,
    // every caller restores the saved value before doing any float arithmetic, and the
    // register is per-thread, so no other test observes it.
    unsafe {
        core::arch::asm!(
            "ldmxcsr [{ptr}]",
            ptr = in(reg) &raw const value,
            options(nostack, preserves_flags, readonly),
        );
    }
}

/// Resolve under MXCSR `value`, restoring the saved register before returning.
#[cfg(target_arch = "x86_64")]
fn resolve_under_mxcsr(value: u32) -> Result<Resolved<Exact>, FloatEnvironmentError> {
    let saved = env::mxcsr();
    set_mxcsr(value);
    let outcome = Exact::resolve();
    set_mxcsr(saved);
    assert_eq!(env::mxcsr(), saved, "the register was restored");
    outcome
}

#[cfg(target_arch = "x86_64")]
#[test]
fn flush_to_zero_is_refused() {
    let saved = env::mxcsr();
    for (name, bit) in [("FTZ", 1_u32 << 15), ("DAZ", 1 << 6)] {
        let refused = resolve_under_mxcsr(saved | bit);
        assert_eq!(
            refused,
            Err(FloatEnvironmentError::FlushToZero {
                register: "MXCSR",
                bits: u64::from(saved | bit),
            }),
            "{name} set must be refused by name"
        );
    }
    for (name, rounding) in [
        ("down", 1_u32 << 13),
        ("up", 2 << 13),
        ("toward zero", 3 << 13),
    ] {
        let refused = resolve_under_mxcsr(saved | rounding);
        assert!(
            matches!(
                refused,
                Err(FloatEnvironmentError::RoundingMode {
                    register: "MXCSR",
                    ..
                })
            ),
            "rounding {name} must be refused, got {refused:?}"
        );
    }
    // The valid neighbour: the same register with those fields clear resolves, so the
    // refusals above are about the bits and not about having touched the register.
    assert!(resolve_under_mxcsr(saved).is_ok());
    assert!(
        Exact::resolve().is_ok(),
        "and the thread is back in the default environment"
    );
}

/// Load `value` into the current thread's FPCR.
#[cfg(target_arch = "aarch64")]
fn set_fpcr(value: u64) {
    // SAFETY: FPCR is writable at EL0. The values written here differ from the saved
    // register only in the FZ and rounding-mode fields, every caller restores the saved
    // value before doing any float arithmetic, and the register is per-thread.
    unsafe {
        core::arch::asm!(
            "msr fpcr, {value}",
            value = in(reg) value,
            options(nomem, nostack, preserves_flags),
        );
    }
}

/// Resolve under FPCR `value`, restoring the saved register before returning.
#[cfg(target_arch = "aarch64")]
fn resolve_under_fpcr(value: u64) -> Result<Resolved<Exact>, FloatEnvironmentError> {
    let saved = env::fpcr();
    set_fpcr(value);
    let outcome = Exact::resolve();
    set_fpcr(saved);
    assert_eq!(env::fpcr(), saved, "the register was restored");
    outcome
}

#[cfg(target_arch = "aarch64")]
#[test]
fn flush_to_zero_is_refused_on_aarch64() {
    let saved = env::fpcr();
    let refused = resolve_under_fpcr(saved | (1 << 24));
    assert_eq!(
        refused,
        Err(FloatEnvironmentError::FlushToZero {
            register: "FPCR",
            bits: saved | (1 << 24),
        }),
        "FZ set must be refused by name"
    );
    for rounding in [1_u64 << 22, 2 << 22, 3 << 22] {
        let refused = resolve_under_fpcr(saved | rounding);
        assert!(
            matches!(
                refused,
                Err(FloatEnvironmentError::RoundingMode {
                    register: "FPCR",
                    ..
                })
            ),
            "a directed rounding mode must be refused, got {refused:?}"
        );
    }
    assert!(
        resolve_under_fpcr(saved).is_ok(),
        "the valid neighbour resolves"
    );
}

#[test]
fn every_refusal_renders_a_distinct_sentence() {
    let cases = [
        FloatEnvironmentError::FlushToZero {
            register: "MXCSR",
            bits: 0x9fc0,
        },
        FloatEnvironmentError::RoundingMode {
            register: "FPCR",
            bits: 0x40_0000,
        },
        FloatEnvironmentError::Uninspectable {
            target_arch: "example",
        },
    ];
    let messages: Vec<String> = cases.iter().map(ToString::to_string).collect();
    for (index, message) in messages.iter().enumerate() {
        assert!(!message.contains("FloatEnvironmentError"), "{message}");
        assert!(!messages[..index].contains(message), "duplicate: {message}");
    }
    assert!(messages[0].contains("MXCSR") && messages[0].contains("0x9fc0"));
}
