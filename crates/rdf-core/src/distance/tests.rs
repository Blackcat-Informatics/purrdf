// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The exact law, held to a scalar reference model on every dispatch path this host can
//! execute.
//!
//! The asm evidence gate can show that an exact kernel has no fused multiply-add; it
//! cannot show the order the kernel adds in. That is proven here, and only here: a plain
//! nested-loop emulation of the lanes, the tree and the tail is the reference, every
//! path is compared with it bit for bit, and a neighbour oracle shows the reference can
//! tell the lane-tree order apart from every other plausible one. The reference rounds
//! every operation with the integer binary64 of `binary64_tests::soft`, not with the
//! floating-point unit, so it is the IEEE-754 result on every target -- the x87, which
//! rounds twice without the kernels' guard, included.

use super::binary64::Precision;
use super::binary64_tests::soft;
use super::*;

/// Every path this host can execute, as resolved handles.
///
/// A handle is built directly only for a path the processor reports, which is the same
/// condition `dispatch::exact_path` checks.
pub(super) fn host_paths() -> Vec<Resolved<Exact>> {
    let portable = Resolved::on(Path::Portable);
    #[cfg(target_arch = "x86_64")]
    {
        let mut paths = vec![portable];
        if std::is_x86_feature_detected!("avx2") {
            paths.push(Resolved::on(Path::Avx2));
        }
        if reassociated::runs_avx512f() {
            paths.push(Resolved::on(Path::Avx512f));
        }
        paths
    }
    #[cfg(not(target_arch = "x86_64"))]
    vec![portable]
}

/// The exact paths this build holds a compilation of, whether or not the host runs them.
pub(super) fn exact_compiled() -> &'static [Path] {
    #[cfg(target_arch = "x86_64")]
    {
        &[Path::Portable, Path::Avx2, Path::Avx512f]
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        &[Path::Portable]
    }
}

// ---- the executed-path requirement -------------------------------------------------

/// The variable a CI job sets to name the dispatch paths its host must execute, as a
/// comma-separated list of [`Path::name`]s.
///
/// Read by the test harness only. Unset, every test still asserts it executed every path
/// the host runs; set, a listed path the host cannot run, or that a test did not
/// execute, fails the run -- which is how a job on an emulated or known processor
/// proves a path ran rather than that it was merely compiled.
pub(super) const REQUIRE_PATHS_VAR: &str = "PURRDF_REQUIRE_DISPATCH_PATHS";

/// Every dispatch path, by which a required name is resolved.
const ALL_PATHS: [Path; 8] = [
    Path::Portable,
    Path::Avx2,
    Path::Sse2,
    Path::Avx2Fma,
    Path::Avx512f,
    Path::Neon,
    Path::WasmSimd128,
    Path::WasmScalar,
];

/// The paths [`REQUIRE_PATHS_VAR`] names, in its order, or none when it is unset.
///
/// # Panics
///
/// When the variable is set but names no path, is not UTF-8, or names something that
/// is not a path: a misspelt requirement must fail rather than require nothing.
pub(super) fn required_paths() -> Vec<Path> {
    let Some(value) = std::env::var_os(REQUIRE_PATHS_VAR) else {
        return Vec::new();
    };
    let value = value
        .into_string()
        .unwrap_or_else(|raw| panic!("{REQUIRE_PATHS_VAR} is not UTF-8: {raw:?}"));
    let known = ALL_PATHS.map(Path::name).join(", ");
    let mut paths = Vec::new();
    for name in value
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        let path = ALL_PATHS
            .into_iter()
            .find(|path| path.name() == name)
            .unwrap_or_else(|| {
                panic!(
                    "{REQUIRE_PATHS_VAR} names `{name}`, which is not a dispatch path; the \
                     paths are: {known}"
                )
            });
        if !paths.contains(&path) {
            paths.push(path);
        }
    }
    assert!(
        !paths.is_empty(),
        "{REQUIRE_PATHS_VAR} is set but names no path ({value:?}); the paths are: {known}"
    );
    paths
}

/// The dispatch paths a test actually ran a kernel on, recorded as it runs them.
#[derive(Default)]
pub(super) struct Executed(Vec<Path>);

impl Executed {
    /// Record that a kernel ran along `path`.
    pub(super) fn ran(&mut self, path: Path) {
        if !self.0.contains(&path) {
            self.0.push(path);
        }
    }

    /// Assert this test ran exactly the paths `runnable` names -- every path of its
    /// arithmetic the host runs -- and every required path of that arithmetic this build
    /// compiles (`compiled`); then print them.
    ///
    /// A required path this build holds no compilation of is not this test's to run; the
    /// dedicated requirement test refuses it by name.
    pub(super) fn assert_ran(&self, test: &str, compiled: &[Path], runnable: &[Path]) {
        for path in runnable {
            assert!(
                self.0.contains(path),
                "{test}: the host runs {path}, but the test did not execute it (executed: \
                 {})",
                self.names()
            );
        }
        for path in &self.0 {
            assert!(
                runnable.contains(path),
                "{test}: executed {path}, which the host does not report it runs"
            );
        }
        for path in required_paths() {
            if !compiled.contains(&path) {
                continue;
            }
            assert!(
                runnable.contains(&path),
                "{test}: {REQUIRE_PATHS_VAR} requires {path}, which this host cannot run \
                 (it runs: {})",
                runnable
                    .iter()
                    .map(|path| path.name())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            assert!(
                self.0.contains(&path),
                "{test}: {REQUIRE_PATHS_VAR} requires {path}, which the test did not execute"
            );
        }
        println!("{test} executed paths: {}", self.names());
    }

    fn names(&self) -> String {
        self.0
            .iter()
            .map(|path| path.name())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// The paths of `handles`.
pub(super) fn paths_of<A: Arithmetic>(handles: &[Resolved<A>]) -> Vec<Path> {
    handles.iter().map(|handle| handle.path()).collect()
}

// ---- the reference model ---------------------------------------------------

/// The exact law over already-rounded terms, as plain nested loops: sixteen lanes by
/// index, the pairwise tree written out step by step, then the tail in ascending index.
fn reference_fold(terms: &[f64]) -> f64 {
    let chunks = terms.len() / 16;
    let mut lanes = [0.0_f64; 16];
    for chunk in 0..chunks {
        for (lane, slot) in lanes.iter_mut().enumerate() {
            *slot = soft::add(*slot, terms[chunk * 16 + lane]);
        }
    }
    let mut s8 = [0.0_f64; 8];
    for l in 0..8 {
        s8[l] = soft::add(lanes[l], lanes[l + 8]);
    }
    let mut s4 = [0.0_f64; 4];
    for l in 0..4 {
        s4[l] = soft::add(s8[l], s8[l + 4]);
    }
    let s2 = [soft::add(s4[0], s4[2]), soft::add(s4[1], s4[3])];
    let mut sum = soft::add(s2[0], s2[1]);
    for &term in &terms[chunks * 16..] {
        sum = soft::add(sum, term);
    }
    sum
}

/// The reference dot product: each product rounded once, then the reference fold.
pub(super) fn reference_dot(a: &[f64], b: &[f64]) -> f64 {
    let terms: Vec<f64> = a.iter().zip(b).map(|(&x, &y)| soft::mul(x, y)).collect();
    reference_fold(&terms)
}

/// The reference squared Euclidean distance.
fn reference_squared_euclidean(a: &[f64], b: &[f64]) -> f64 {
    let terms: Vec<f64> = a
        .iter()
        .zip(b)
        .map(|(&x, &y)| {
            let delta = soft::sub(x, y);
            soft::mul(delta, delta)
        })
        .collect();
    reference_fold(&terms)
}

/// The reference finished distance, or `None` when it is not finite.
pub(super) fn reference_distance(
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
            let denominator = soft::mul(a_norm, b_norm);
            let quotient = soft::div(reference_dot(a, b), denominator);
            soft::sub(1.0, quotient)
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
        crate::test_rng::splitmix64_next(&mut self.0)
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
    executed: &mut Executed,
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
    let exact = Resolved::<Exact>::on(Path::Portable);
    let query_norm = exact.norm(query_typed);
    let norms: Vec<f64> = rows_wide.iter().map(|row| exact.norm(row)).collect();
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
            executed.ran(path.path());
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
    let mut executed = Executed::default();
    for &len in &lengths {
        for offset in 0..4 {
            let query = stream.vector(len);
            let rows: Vec<Vec<f64>> = (0..ROWS).map(|_| stream.vector(len)).collect();
            vectors += 1 + ROWS;
            compared += check_group::<f64, f64>(&paths, &mut executed, &query, &rows, offset);
            compared += check_group::<f64, f32>(&paths, &mut executed, &query, &rows, offset);
            compared += check_group::<f32, f32>(&paths, &mut executed, &query, &rows, offset);
            compared += check_group::<f32, f64>(&paths, &mut executed, &query, &rows, offset);
        }
    }
    assert!(vectors >= 10_000, "the fixture is {vectors} vectors");
    println!("{vectors} vectors, {compared} comparisons against the reference model");
    executed.assert_ran(
        "exact_paths_agree_bitwise",
        exact_compiled(),
        &paths_of(&paths),
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
    // ascending sequential, lanes combined in ascending order, adjacent-pair tree. Each
    // rounds through the reference's binary64, so the x87's wider registers cannot keep
    // the 1 a binary64 sum drops.
    let mut sequential = 0.0_f64;
    for (&x, &y) in chunk.iter().zip(&ones) {
        let product = soft::mul(x, y);
        sequential = soft::add(sequential, product);
    }
    let mut lane_order = 0.0_f64;
    for value in chunk {
        lane_order = soft::add(lane_order, value);
    }
    let adjacent = {
        let pairwise = |values: &[f64]| -> Vec<f64> {
            values
                .chunks(2)
                .map(|pair| soft::add(pair[0], pair[1]))
                .collect()
        };
        let s2 = pairwise(&pairwise(&pairwise(&chunk)));
        soft::add(s2[0], s2[1])
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
            lanes[index % 16] = soft::add(lanes[index % 16], *value);
        }
        reference_fold(&lanes)
    };
    assert_eq!(
        tail_into_lane, 1.0,
        "the alternative order must genuinely differ"
    );

    let mut executed = Executed::default();
    for path in &paths {
        executed.ran(path.path());
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
    executed.assert_ran(
        "order_is_the_lane_tree",
        exact_compiled(),
        &paths_of(&paths),
    );
}

#[test]
fn bounded_matches_full() {
    let paths = host_paths();
    let mut executed = Executed::default();
    let mut stream = Stream(0xB0B0_0000_0000_0001);
    for len in [1_usize, 63, 64, 65, 127, 128, 200, 4_096] {
        let a: Vec<f64> = (0..len).map(|_| stream.signed()).collect();
        let b: Vec<f64> = (0..len).map(|_| stream.signed()).collect();
        let full = reference_squared_euclidean(&a, &b);
        assert!(full > 0.0 && full.is_finite());
        for path in &paths {
            executed.ran(path.path());
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
    executed.assert_ran("bounded_matches_full", exact_compiled(), &paths_of(&paths));
}

#[test]
fn nonfinite_checked_before_bound() {
    let paths = host_paths();
    let mut executed = Executed::default();
    for len in [8_usize, 64, 4_096] {
        // One overflowing lane (or tail term), everything else zero.
        let mut huge = vec![0.0_f64; len];
        huge[3] = f64::MAX / 2.0;
        huge[len - 1] = -f64::MAX / 2.0;
        let zero = vec![0.0_f64; len];
        for path in &paths {
            executed.ran(path.path());
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
    executed.assert_ran(
        "nonfinite_checked_before_bound",
        exact_compiled(),
        &paths_of(&paths),
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
    for path in [Path::Portable, Path::Avx2, Path::Avx512f] {
        assert_eq!(
            Exact::image_code(path),
            Some(Exact::IMAGE_CODE),
            "every exact path records the one code"
        );
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
        let expected = if reassociated::runs_avx512f() {
            Path::Avx512f
        } else if std::is_x86_feature_detected!("avx2") {
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

/// The exact arithmetic's one image code resolves on every host, whichever exact path
/// the processor runs: hiding AVX-512F moves resolution to the AVX2 (or portable) path,
/// and the recorded code still resolves there, to the same bits the reference model
/// gives. Unhidden, the host's own report selects its widest exact path again.
#[cfg(target_arch = "x86_64")]
#[test]
fn the_exact_code_resolves_without_avx512f() {
    use super::reassociated::{Feature, hidden};

    let mut stream = Stream(0x5EED_0000_0000_0512);
    let a = stream.vector(4_099);
    let b = stream.vector(4_099);
    let want = Some((-reference_dot(&a, &b)).to_bits());
    let dot = |handle: Resolved<Exact>| {
        handle
            .distance(Measure::NegativeDot, &a, 0.0, &b, 0.0)
            .map(f64::to_bits)
    };
    let narrower = if std::is_x86_feature_detected!("avx2") {
        Path::Avx2
    } else {
        Path::Portable
    };
    {
        let _hidden = hidden::hide(Feature::Avx512f);
        let resolved = Exact::resolve().expect("the default environment is the IEEE one");
        assert_eq!(
            resolved.path(),
            narrower,
            "no AVX-512F, so the next exact path"
        );
        let recorded =
            Exact::resolve_recorded(Exact::IMAGE_CODE).expect("the exact code runs anywhere");
        assert_eq!(recorded, resolved);
        assert_eq!(dot(recorded), want, "{narrower} computes the law's bits");
    }
    let widest = Exact::resolve().expect("the default environment is the IEEE one");
    let expected = if reassociated::runs_avx512f() {
        Path::Avx512f
    } else {
        narrower
    };
    assert_eq!(
        widest.path(),
        expected,
        "unhidden, the host's report decides"
    );
    assert_eq!(
        Exact::resolve_recorded(Exact::IMAGE_CODE),
        Ok(widest),
        "the recorded code resolves the widest exact path"
    );
    assert_eq!(dot(widest), want, "{expected} computes the law's bits");
    println!(
        "the_exact_code_resolves_without_avx512f: hidden {narrower}, unhidden {}",
        widest.path()
    );
}

/// Which probe rows return bits other than the IEEE-754 ones on this thread, by index.
///
/// Every row is observed, not only the first failure, so a test can hold the whole
/// pattern a departure produces against the one the probe's table claims for it.
fn failing_rows() -> [bool; env::PROBES.len()] {
    let precision = Precision::enter();
    let ops = precision.binary64();
    let mut failing = [false; env::PROBES.len()];
    for (slot, probe) in failing.iter_mut().zip(&env::PROBES) {
        *slot = probe.observe(ops) != probe.expected;
    }
    failing
}

/// The indices set in `rows`, for a readable assertion.
fn indices(rows: [bool; env::PROBES.len()]) -> Vec<usize> {
    rows.iter()
        .enumerate()
        .filter_map(|(index, failing)| failing.then_some(index))
        .collect()
}

/// A probe refusal, as the probe builds it for row `row` observing `observed`.
fn probe_refusal(row: usize, observed: f64) -> FloatEnvironmentError {
    let probe = env::PROBES[row];
    let evidence = FloatEnvironmentEvidence::Probe {
        operation: probe.operation,
        expected: probe.expected,
        observed: observed.to_bits(),
    };
    match probe.departure {
        env::Departure::Flush => FloatEnvironmentError::FlushToZero { evidence },
        env::Departure::Rounding => FloatEnvironmentError::RoundingMode { evidence },
        env::Departure::DoubleRounding => FloatEnvironmentError::DoubleRounding { evidence },
    }
}

/// `1 + 2⁻⁵²`, the successor of one.
const ONE_PLUS_ULP: f64 = 1.0 + f64::EPSILON;

#[test]
fn the_default_environment_passes_every_probe_row() {
    // The valid neighbour of every refusal below, and the control row of every pattern
    // they compare: in the default environment no row differs, so a row that fails under
    // a departure fails because of it.
    assert_eq!(indices(failing_rows()), Vec::<usize>::new());
    assert_eq!(env::probe(), Ok(()));
    assert_eq!(env::check(), Ok(()));
    let precision = Precision::enter();
    for probe in &env::PROBES {
        assert_eq!(
            probe.observe(precision.binary64()),
            probe.expected,
            "{}",
            probe.operation
        );
    }
    // The table's constants, recomputed from their definitions on this thread.
    assert_eq!(env::PROBES[0].expected, (f64::MIN_POSITIVE / 2.0).to_bits());
    assert_eq!(env::PROBES[1].expected, (2.0 * f64::EPSILON).to_bits());
    assert_eq!(env::PROBES[2].expected, ONE_PLUS_ULP.to_bits());
    assert_eq!(env::PROBES[5].expected, (-1.0_f64).to_bits());
    assert_eq!(
        env::PROBES[7].expected,
        (ONE_PLUS_ULP + f64::EPSILON).to_bits()
    );
    // The double-rounding rows' constants, recomputed by the software reference, which
    // rounds in integers and so gives the IEEE-754 result on every unit.
    for probe in &env::PROBES[8..] {
        let (a, b) = probe.operands();
        let once = match probe.symbol() {
            '+' => soft::add(a, b),
            '*' => soft::mul(a, b),
            _ => soft::div(a, b),
        };
        assert_eq!(once.to_bits(), probe.expected, "{}", probe.operation);
    }
    assert_eq!(env::PROBES[8].expected, ONE_PLUS_ULP.to_bits());
    assert_eq!(
        env::PROBES[11].expected,
        (f64::MIN_POSITIVE / 2.0 + f64::from_bits(1)).to_bits()
    );
}

#[test]
fn every_probe_row_is_distinct_and_names_its_departure() {
    // Two rows with the same operands would test one thing twice and leave a departure
    // the table claims to cover untested.
    for (index, probe) in env::PROBES.iter().enumerate() {
        for other in &env::PROBES[..index] {
            assert_ne!(probe.operation, other.operation);
        }
    }
    let flush = env::PROBES
        .iter()
        .filter(|probe| probe.departure == env::Departure::Flush)
        .count();
    assert_eq!(
        flush, 2,
        "one row for flushed results, one for flushed operands"
    );
    let double = env::PROBES
        .iter()
        .filter(|probe| probe.departure == env::Departure::DoubleRounding)
        .count();
    assert_eq!(
        double, 5,
        "two sums and a product in the normal range, a product and a quotient in the \
         subnormal range"
    );
}

// ---- x86: MXCSR on both widths --------------------------------------------------

/// The current thread's MXCSR.
#[cfg(any(
    target_arch = "x86_64",
    all(target_arch = "x86", target_feature = "sse2")
))]
fn read_mxcsr() -> u32 {
    let mut value: u32 = 0;
    // SAFETY: `stmxcsr` stores the 32-bit MXCSR, and nothing else, to a live, aligned,
    // writable `u32` on this stack frame.
    unsafe {
        core::arch::asm!(
            "stmxcsr [{ptr}]",
            ptr = in(reg) &raw mut value,
            options(nostack, preserves_flags),
        );
    }
    value
}

/// Load `value` into the current thread's MXCSR.
#[cfg(any(
    target_arch = "x86_64",
    all(target_arch = "x86", target_feature = "sse2")
))]
fn write_mxcsr(value: u32) {
    // SAFETY: `ldmxcsr` loads MXCSR from a live, aligned `u32`. Every value loaded here
    // differs from the saved register only in the FTZ, DAZ and rounding-control fields,
    // `Mxcsr` restores the saved value on every exit including a panic, and the register
    // is per-thread, so no other test observes it.
    unsafe {
        core::arch::asm!(
            "ldmxcsr [{ptr}]",
            ptr = in(reg) &raw const value,
            options(nostack, preserves_flags, readonly),
        );
    }
}

/// MXCSR loaded with a value for as long as the guard lives; the saved value is restored
/// when it drops, including by unwinding.
#[cfg(any(
    target_arch = "x86_64",
    all(target_arch = "x86", target_feature = "sse2")
))]
struct Mxcsr(u32);

#[cfg(any(
    target_arch = "x86_64",
    all(target_arch = "x86", target_feature = "sse2")
))]
impl Mxcsr {
    fn load(value: u32) -> Self {
        let saved = read_mxcsr();
        write_mxcsr(value);
        Self(saved)
    }
}

#[cfg(any(
    target_arch = "x86_64",
    all(target_arch = "x86", target_feature = "sse2")
))]
impl Drop for Mxcsr {
    fn drop(&mut self) {
        write_mxcsr(self.0);
    }
}

/// MXCSR flush-to-zero.
#[cfg(any(
    target_arch = "x86_64",
    all(target_arch = "x86", target_feature = "sse2")
))]
const MXCSR_FTZ: u32 = 1 << 15;
/// MXCSR denormals-are-zero.
#[cfg(any(
    target_arch = "x86_64",
    all(target_arch = "x86", target_feature = "sse2")
))]
const MXCSR_DAZ: u32 = 1 << 6;
/// MXCSR rounding control: down, up and toward zero.
#[cfg(any(
    target_arch = "x86_64",
    all(target_arch = "x86", target_feature = "sse2")
))]
const MXCSR_DOWN: u32 = 1 << 13;
#[cfg(any(
    target_arch = "x86_64",
    all(target_arch = "x86", target_feature = "sse2")
))]
const MXCSR_UP: u32 = 2 << 13;
#[cfg(any(
    target_arch = "x86_64",
    all(target_arch = "x86", target_feature = "sse2")
))]
const MXCSR_TOWARD_ZERO: u32 = 3 << 13;

/// Every MXCSR departure, the probe rows it must fail, and the refusal the probe returns.
#[cfg(any(
    target_arch = "x86_64",
    all(target_arch = "x86", target_feature = "sse2")
))]
fn mxcsr_departures() -> [(&'static str, u32, Vec<usize>, FloatEnvironmentError); 5] {
    [
        ("FTZ", MXCSR_FTZ, vec![0, 11, 12], probe_refusal(0, 0.0)),
        ("DAZ", MXCSR_DAZ, vec![1], probe_refusal(1, 0.0)),
        (
            "down",
            MXCSR_DOWN,
            vec![2, 5, 7, 8, 12],
            probe_refusal(2, 1.0),
        ),
        (
            "up",
            MXCSR_UP,
            vec![3, 4, 6, 9, 10, 11],
            probe_refusal(3, ONE_PLUS_ULP),
        ),
        (
            "toward zero",
            MXCSR_TOWARD_ZERO,
            vec![2, 4, 7, 8, 12],
            probe_refusal(2, 1.0),
        ),
    ]
}

#[cfg(any(
    target_arch = "x86_64",
    all(target_arch = "x86", target_feature = "sse2")
))]
#[test]
fn the_probe_alone_refuses_every_mxcsr_departure() {
    // The probe, not the register read: this is the refusal every target without a
    // register reader relies on, exercised where the environment can be set.
    let saved = read_mxcsr();
    for (name, bits, rows, refusal) in mxcsr_departures() {
        let (failing, outcome) = {
            let _loaded = Mxcsr::load(saved | bits);
            (failing_rows(), env::probe())
        };
        assert_eq!(read_mxcsr(), saved, "the register was restored");
        assert_eq!(indices(failing), rows, "{name}: the rows that must fail");
        assert_eq!(outcome, Err(refusal), "{name}: the probe's refusal");
    }
    // The valid neighbour: the saved register loaded through the same guard passes.
    let (failing, outcome) = {
        let _loaded = Mxcsr::load(saved);
        (failing_rows(), env::probe())
    };
    assert_eq!(indices(failing), Vec::<usize>::new());
    assert_eq!(outcome, Ok(()));
}

/// Resolve under MXCSR `value`, restoring the saved register before returning.
#[cfg(any(
    target_arch = "x86_64",
    all(target_arch = "x86", target_feature = "sse2")
))]
fn resolve_under_mxcsr(value: u32) -> Result<Resolved<Exact>, FloatEnvironmentError> {
    let saved = read_mxcsr();
    let outcome = {
        let _loaded = Mxcsr::load(value);
        Exact::resolve()
    };
    assert_eq!(read_mxcsr(), saved, "the register was restored");
    outcome
}

#[cfg(any(
    target_arch = "x86_64",
    all(target_arch = "x86", target_feature = "sse2")
))]
#[test]
fn resolve_refuses_every_mxcsr_departure_and_answers_the_default() {
    let saved = read_mxcsr();
    for (name, bits, _, probe) in mxcsr_departures() {
        let refused = resolve_under_mxcsr(saved | bits);
        // On x86_64 the register is read first, so the refusal names it; on 32-bit x86
        // no register is read and the probe's refusal is the answer.
        #[cfg(target_arch = "x86_64")]
        let expected = {
            let evidence = FloatEnvironmentEvidence::Register {
                name: "MXCSR",
                bits: u64::from(saved | bits),
            };
            match probe {
                FloatEnvironmentError::FlushToZero { .. } => {
                    FloatEnvironmentError::FlushToZero { evidence }
                }
                _ => FloatEnvironmentError::RoundingMode { evidence },
            }
        };
        #[cfg(target_arch = "x86")]
        let expected = probe;
        assert_eq!(refused, Err(expected), "{name} must be refused by name");
    }
    // The valid neighbour: the same register with those fields clear resolves, so the
    // refusals above are about the bits and not about having touched the register.
    assert!(resolve_under_mxcsr(saved).is_ok());
    assert!(
        Exact::resolve().is_ok(),
        "and the thread is back in the default environment"
    );
}

// ---- aarch64: FPCR ---------------------------------------------------------------

/// Load `value` into the current thread's FPCR.
#[cfg(target_arch = "aarch64")]
fn write_fpcr(value: u64) {
    // SAFETY: FPCR is writable at EL0. The values written here differ from the saved
    // register only in the FZ and rounding-mode fields, `Fpcr` restores the saved value
    // on every exit including a panic, and the register is per-thread.
    unsafe {
        core::arch::asm!(
            "msr fpcr, {value}",
            value = in(reg) value,
            options(nostack, preserves_flags),
        );
    }
}

/// FPCR loaded with a value for as long as the guard lives.
#[cfg(target_arch = "aarch64")]
struct Fpcr(u64);

#[cfg(target_arch = "aarch64")]
impl Fpcr {
    fn load(value: u64) -> Self {
        let saved = env::fpcr();
        write_fpcr(value);
        Self(saved)
    }
}

#[cfg(target_arch = "aarch64")]
impl Drop for Fpcr {
    fn drop(&mut self) {
        write_fpcr(self.0);
    }
}

/// Every FPCR departure, the probe rows it must fail, and the refusal the probe returns.
/// FZ flushes subnormal operands as well as results, so both flush rows fail under it.
#[cfg(target_arch = "aarch64")]
fn fpcr_departures() -> [(&'static str, u64, Vec<usize>, FloatEnvironmentError); 4] {
    [
        ("FZ", 1 << 24, vec![0, 1, 11, 12], probe_refusal(0, 0.0)),
        (
            "RP",
            1 << 22,
            vec![3, 4, 6, 9, 10, 11],
            probe_refusal(3, ONE_PLUS_ULP),
        ),
        ("RM", 2 << 22, vec![2, 5, 7, 8, 12], probe_refusal(2, 1.0)),
        ("RZ", 3 << 22, vec![2, 4, 7, 8, 12], probe_refusal(2, 1.0)),
    ]
}

#[cfg(target_arch = "aarch64")]
#[test]
fn the_probe_alone_refuses_every_fpcr_departure() {
    let saved = env::fpcr();
    for (name, bits, rows, refusal) in fpcr_departures() {
        let (failing, outcome) = {
            let _loaded = Fpcr::load(saved | bits);
            (failing_rows(), env::probe())
        };
        assert_eq!(env::fpcr(), saved, "the register was restored");
        assert_eq!(indices(failing), rows, "{name}: the rows that must fail");
        assert_eq!(outcome, Err(refusal), "{name}: the probe's refusal");
    }
    let (failing, outcome) = {
        let _loaded = Fpcr::load(saved);
        (failing_rows(), env::probe())
    };
    assert_eq!(indices(failing), Vec::<usize>::new());
    assert_eq!(outcome, Ok(()));
}

#[cfg(target_arch = "aarch64")]
#[test]
fn resolve_refuses_every_fpcr_departure_by_register() {
    let saved = env::fpcr();
    for (name, bits, _, probe) in fpcr_departures() {
        let refused = {
            let _loaded = Fpcr::load(saved | bits);
            Exact::resolve()
        };
        let evidence = FloatEnvironmentEvidence::Register {
            name: "FPCR",
            bits: saved | bits,
        };
        let expected = match probe {
            FloatEnvironmentError::FlushToZero { .. } => {
                FloatEnvironmentError::FlushToZero { evidence }
            }
            _ => FloatEnvironmentError::RoundingMode { evidence },
        };
        assert_eq!(refused, Err(expected), "{name} must be refused by name");
    }
    let resolved = {
        let _loaded = Fpcr::load(saved);
        Exact::resolve()
    };
    assert!(resolved.is_ok(), "the valid neighbour resolves");
}

// ---- riscv64: the `frm` rounding field, a target with no register reader -----------

/// The current thread's dynamic rounding mode.
#[cfg(target_arch = "riscv64")]
fn read_frm() -> u64 {
    let value: u64;
    // SAFETY: `frm` is a user-level CSR of the F extension; reading it copies the field
    // into a general-purpose register and changes no state.
    unsafe {
        core::arch::asm!("csrr {value}, frm", value = out(reg) value, options(nostack));
    }
    value
}

/// Set the current thread's dynamic rounding mode.
#[cfg(target_arch = "riscv64")]
fn write_frm(value: u64) {
    // SAFETY: `frm` is a user-level CSR of the F extension. Every value written here is a
    // defined rounding mode, `Frm` restores the saved one on every exit including a
    // panic, and the field is per-thread.
    unsafe {
        core::arch::asm!("csrw frm, {value}", value = in(reg) value, options(nostack));
    }
}

/// `frm` set for as long as the guard lives.
#[cfg(target_arch = "riscv64")]
struct Frm(u64);

#[cfg(target_arch = "riscv64")]
impl Frm {
    fn load(value: u64) -> Self {
        let saved = read_frm();
        write_frm(value);
        Self(saved)
    }
}

#[cfg(target_arch = "riscv64")]
impl Drop for Frm {
    fn drop(&mut self) {
        write_frm(self.0);
    }
}

#[cfg(target_arch = "riscv64")]
#[test]
fn the_probe_refuses_every_riscv_rounding_mode_through_resolve() {
    // RISC-V has no flush-to-zero, but it has four rounding modes besides the default,
    // one of them round-to-nearest with ties AWAY from zero, which only the tie rows can
    // tell from ties-to-even. No register is read on this target, so `Exact::resolve`
    // refuses on the probe's evidence alone.
    let saved = read_frm();
    assert_eq!(saved, 0, "the default dynamic rounding mode is RNE");
    let departures = [
        ("RTZ", 1, vec![2, 4, 7, 8, 12], probe_refusal(2, 1.0)),
        ("RDN", 2, vec![2, 5, 7, 8, 12], probe_refusal(2, 1.0)),
        (
            "RUP",
            3,
            vec![3, 4, 6, 9, 10, 11],
            probe_refusal(3, ONE_PLUS_ULP),
        ),
        ("RMM", 4, vec![6], probe_refusal(6, ONE_PLUS_ULP)),
    ];
    for (name, mode, rows, refusal) in departures {
        let (failing, probed, resolved) = {
            let _loaded = Frm::load(mode);
            (failing_rows(), env::probe(), Exact::resolve())
        };
        assert_eq!(read_frm(), saved, "the rounding mode was restored");
        assert_eq!(indices(failing), rows, "{name}: the rows that must fail");
        assert_eq!(probed, Err(refusal), "{name}: the probe's refusal");
        assert_eq!(
            resolved,
            Err(refusal),
            "{name}: resolve refuses by the probe"
        );
    }
    let resolved = {
        let _loaded = Frm::load(saved);
        Exact::resolve()
    };
    assert!(resolved.is_ok(), "the valid neighbour resolves");
}

#[test]
fn every_refusal_renders_a_distinct_sentence() {
    let register = FloatEnvironmentEvidence::Register {
        name: "MXCSR",
        bits: 0x9fc0,
    };
    let cases = [
        FloatEnvironmentError::FlushToZero { evidence: register },
        FloatEnvironmentError::RoundingMode { evidence: register },
        probe_refusal(0, 0.0),
        probe_refusal(2, 1.0),
        probe_refusal(8, 1.0),
    ];
    let messages: Vec<String> = cases.iter().map(ToString::to_string).collect();
    for (index, message) in messages.iter().enumerate() {
        assert!(!message.contains("FloatEnvironmentError"), "{message}");
        assert!(!messages[..index].contains(message), "duplicate: {message}");
    }
    assert!(messages[0].contains("MXCSR") && messages[0].contains("0x9fc0"));
    assert!(
        messages[2].contains("f64::MIN_POSITIVE * 0.5")
            && messages[2].contains("0x0008000000000000")
            && messages[2].contains("0x0000000000000000"),
        "a probe refusal names its operation and both bit patterns: {}",
        messages[2]
    );
    assert!(messages[3].contains("1 + 0.75 ulp"), "{}", messages[3]);
    assert!(
        messages[4].contains("rounds binary64 results twice")
            && messages[4].contains("1 + (2^-53 + 2^-78)")
            && messages[4].contains("0x3ff0000000000001")
            && messages[4].contains("0x3ff0000000000000"),
        "{}",
        messages[4]
    );
}
