// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The reassociated arithmetic, held to its own contract on every dispatch path this
//! host can execute.
//!
//! Its bits are not pinned, so nothing here compares them with a golden. What the
//! contract does promise is checked instead: every result is within the summation
//! error bound of a double-double reference; one path gives one answer for one input
//! wherever that input sits in memory and whichever API asked; the bounded form never
//! contradicts the full one; an overflow is refused; and the exact arithmetic never
//! runs this code, shown on an input where the two genuinely differ.

use super::tests::{
    Executed, REQUIRE_PATHS_VAR, Store, Stream, at_offset, exact_compiled, paths_of, reference_dot,
    required_paths, widened,
};
use super::*;

/// Every reassociated path this host can execute, as resolved handles.
///
/// A handle is built directly only for a path the processor reports, which is the same
/// condition `reassociated::path` checks.
fn host_paths() -> Vec<Resolved<Reassociated>> {
    #[cfg(target_arch = "x86_64")]
    {
        let mut paths = vec![Resolved::on(Path::Sse2)];
        let avx2_fma =
            std::is_x86_feature_detected!("avx2") && std::is_x86_feature_detected!("fma");
        if avx2_fma {
            paths.push(Resolved::on(Path::Avx2Fma));
            if std::is_x86_feature_detected!("avx512f") {
                paths.push(Resolved::on(Path::Avx512f));
            }
        }
        paths
    }
    #[cfg(target_arch = "aarch64")]
    {
        vec![Resolved::on(Path::Neon)]
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        vec![Reassociated::resolve().expect("a reassociated path on this target")]
    }
}

/// The reassociated paths this build holds a compilation of, whether or not the host
/// runs them.
fn reassociated_compiled() -> &'static [Path] {
    #[cfg(target_arch = "x86_64")]
    {
        &[Path::Sse2, Path::Avx2Fma, Path::Avx512f]
    }
    #[cfg(target_arch = "aarch64")]
    {
        &[Path::Neon]
    }
    #[cfg(all(
        any(target_arch = "wasm32", target_arch = "wasm64"),
        target_feature = "simd128"
    ))]
    {
        &[Path::WasmSimd128]
    }
    #[cfg(all(
        any(target_arch = "wasm32", target_arch = "wasm64"),
        not(target_feature = "simd128")
    ))]
    {
        &[Path::WasmScalar]
    }
    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "wasm32",
        target_arch = "wasm64"
    )))]
    {
        &[Path::Portable]
    }
}

/// The paths among `paths` whose compilation contracts multiplies into adds.
fn fma_paths(paths: &[Resolved<Reassociated>]) -> Vec<Resolved<Reassociated>> {
    paths
        .iter()
        .copied()
        .filter(|path| matches!(path.path(), Path::Avx2Fma | Path::Avx512f | Path::Neon))
        .collect()
}

/// Assert `executed` ran every reassociated path the host runs, and every required one.
fn assert_ran(executed: &Executed, test: &str) {
    executed.assert_ran(test, reassociated_compiled(), &paths_of(&host_paths()));
}

fn names(paths: &[Resolved<Reassociated>]) -> String {
    paths
        .iter()
        .map(|path| path.path().name())
        .collect::<Vec<_>>()
        .join(", ")
}

// ---- the double-double reference ---------------------------------------------

/// `a + b` as an unevaluated pair `(sum, error)`, exactly.
fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let sum = a + b;
    let b_virtual = sum - a;
    let a_virtual = sum - b_virtual;
    (sum, (a - a_virtual) + (b - b_virtual))
}

/// `a · b` as an unevaluated pair `(product, error)`, exactly (barring underflow).
fn two_prod(a: f64, b: f64) -> (f64, f64) {
    let product = a * b;
    (product, a.mul_add(b, -product))
}

/// A double-double accumulator: about 106 significant bits.
#[derive(Clone, Copy, Default)]
struct DoubleDouble {
    hi: f64,
    lo: f64,
}

impl DoubleDouble {
    fn add(self, hi: f64, lo: f64) -> Self {
        let (sum, error) = two_sum(self.hi, hi);
        let error = error + self.lo + lo;
        let (hi, lo) = two_sum(sum, error);
        Self { hi, lo }
    }

    fn value(self) -> f64 {
        self.hi + self.lo
    }
}

/// The dot product to about 106 bits, and `Σ|a[i]·b[i]|`.
fn dd_dot(a: &[f64], b: &[f64]) -> (f64, f64) {
    let mut sum = DoubleDouble::default();
    let mut magnitude = 0.0_f64;
    for (&x, &y) in a.iter().zip(b) {
        let (product, error) = two_prod(x, y);
        sum = sum.add(product, error);
        magnitude += product.abs();
    }
    (sum.value(), magnitude)
}

/// The squared Euclidean distance to about 106 bits, and `Σ(a[i] - b[i])²`.
fn dd_squared_euclidean(a: &[f64], b: &[f64]) -> (f64, f64) {
    let mut sum = DoubleDouble::default();
    let mut magnitude = 0.0_f64;
    for (&x, &y) in a.iter().zip(b) {
        // `x - y = delta + tail` exactly, so the square is
        // `delta² + 2·delta·tail + tail²`; the last term is below the reference's own
        // precision.
        let (delta, tail) = two_sum(x, -y);
        let (square, square_error) = two_prod(delta, delta);
        let cross = 2.0 * delta * tail;
        sum = sum.add(square, square_error + cross);
        magnitude += square;
    }
    (sum.value(), magnitude)
}

/// The contract's error bound for an `n`-term sum of magnitude `magnitude`:
/// `2·n·ε·Σ|tᵢ|`, plus the absolute error gradual underflow may add, one smallest
/// subnormal per operation.
fn error_bound(n: usize, magnitude: f64) -> f64 {
    let n = n.max(1) as f64;
    let summation = 2.0 * n * f64::EPSILON * magnitude;
    let underflow = 4.0 * n * f64::from_bits(1);
    summation + underflow
}

// ---- the tests -------------------------------------------------------------------

/// One stored-width pair over every host path, every API, against the reference.
fn check_bound<Q: Store, T: Store>(
    paths: &[Resolved<Reassociated>],
    executed: &mut Executed,
    a: &[f64],
    b: &[f64],
    offset: usize,
) -> usize {
    let a_buffer: Vec<Q> = at_offset(a, offset);
    let b_buffer: Vec<T> = at_offset(b, offset);
    let a_typed = &a_buffer[offset..];
    let b_typed = &b_buffer[offset..];
    let (a_wide, b_wide) = (widened(a_typed), widened(b_typed));
    let n = a.len();
    let (dot, dot_magnitude) = dd_dot(&a_wide, &b_wide);
    let (squares, squares_magnitude) = dd_squared_euclidean(&a_wide, &b_wide);
    let view = RowsRef::new(b_typed, 1, n, &[]).expect("one row");
    let mut compared = 0;
    for &path in paths {
        executed.ran(path.path());
        let fast_dot = -path
            .distance(Measure::NegativeDot, a_typed, 0.0, b_typed, 0.0)
            .expect("finite");
        assert!(
            (fast_dot - dot).abs() <= error_bound(n, dot_magnitude),
            "dot on {} at len {n}: {fast_dot} vs reference {dot}",
            path.path()
        );
        let fast_squares = path
            .distance(Measure::SquaredEuclidean, a_typed, 0.0, b_typed, 0.0)
            .expect("finite");
        assert!(
            (fast_squares - squares).abs() <= error_bound(n, squares_magnitude),
            "squared Euclidean on {} at len {n}: {fast_squares} vs reference {squares}",
            path.path()
        );
        // The batch form runs the same compilation, so it is the same number.
        let mut out = [None];
        path.distances(Measure::SquaredEuclidean, a_typed, 0.0, view, &mut out);
        assert_eq!(out[0].map(f64::to_bits), Some(fast_squares.to_bits()));
        compared += 3;
    }
    compared
}

#[test]
fn reassociated_within_error_bound() {
    let paths = host_paths();
    let mut stream = Stream(0x0FA5_7000_0000_0001);
    let lengths: Vec<usize> = (0..=70).chain([127, 128, 129, 1_024, 4_096]).collect();
    let mut compared = 0;
    let mut executed = Executed::default();
    for &len in &lengths {
        for offset in 0..4 {
            let a = stream.vector(len);
            let b = stream.vector(len);
            compared += check_bound::<f64, f64>(&paths, &mut executed, &a, &b, offset);
            compared += check_bound::<f64, f32>(&paths, &mut executed, &a, &b, offset);
            compared += check_bound::<f32, f64>(&paths, &mut executed, &a, &b, offset);
            compared += check_bound::<f32, f32>(&paths, &mut executed, &a, &b, offset);
        }
    }
    println!("{compared} comparisons against the double-double reference");
    assert_ran(&executed, "reassociated_within_error_bound");
}

/// Every API's answer for one pair on one path, as bits.
fn every_api<Q: Store, T: Store>(
    path: Resolved<Reassociated>,
    measure: Measure,
    a: &[f64],
    b: &[f64],
    offset: usize,
) -> [Option<u64>; 4] {
    let a_buffer: Vec<Q> = at_offset(a, offset);
    let b_buffer: Vec<T> = at_offset(b, offset);
    let a_typed = &a_buffer[offset..];
    let b_typed = &b_buffer[offset..];
    let exact = path.exact();
    let (a_norm, b_norm) = (exact.norm(a_typed), exact.norm(b_typed));
    let norms = [b_norm];
    let view = RowsRef::new(b_typed, 1, a.len(), &norms).expect("one row");
    let mut batch = [None];
    path.distances(measure, a_typed, a_norm, view, &mut batch);
    let mut indexed = [None];
    path.distances_indexed(measure, a_typed, a_norm, view, &[0], &mut indexed);
    let pair = path.distance(measure, a_typed, a_norm, b_typed, b_norm);
    let bounded = match path.distance_bounded(
        measure,
        a_typed,
        a_norm,
        b_typed,
        b_norm,
        Bound::Above(f64::INFINITY),
    ) {
        Bounded::Below(value) => Some(value),
        Bounded::NonFinite => None,
        Bounded::Beyond => panic!("an infinite bound is never met"),
    };
    [batch[0], indexed[0], pair, bounded].map(|value| value.map(f64::to_bits))
}

fn check_deterministic<Q: Store, T: Store>(path: Resolved<Reassociated>, a: &[f64], b: &[f64]) {
    for measure in [
        Measure::SquaredEuclidean,
        Measure::NegativeDot,
        Measure::Cosine,
    ] {
        let first = every_api::<Q, T>(path, measure, a, b, 0);
        assert!(
            first.iter().all(|bits| *bits == first[0]),
            "{measure:?} on {} at len {}: every API runs the one compilation, got {first:?}",
            path.path(),
            a.len()
        );
        for offset in 1..4 {
            assert_eq!(
                every_api::<Q, T>(path, measure, a, b, offset),
                first,
                "{measure:?} on {} at len {}, offset {offset}",
                path.path(),
                a.len()
            );
        }
    }
}

#[test]
fn reassociated_deterministic_per_path() {
    let paths = host_paths();
    let mut stream = Stream(0xDE7E_0000_0000_0002);
    let mut executed = Executed::default();
    for len in (0..=70).chain([127, 128, 129, 1_024, 4_096]) {
        let a = stream.vector(len);
        let b = stream.vector(len);
        for &path in &paths {
            executed.ran(path.path());
            check_deterministic::<f64, f64>(path, &a, &b);
            check_deterministic::<f64, f32>(path, &a, &b);
            check_deterministic::<f32, f64>(path, &a, &b);
            check_deterministic::<f32, f32>(path, &a, &b);
        }
    }
    assert_ran(&executed, "reassociated_deterministic_per_path");
}

#[test]
fn reassociated_bounded_agrees_with_full() {
    let paths = host_paths();
    let mut stream = Stream(0xB0B0_0000_0000_0003);
    let mut executed = Executed::default();
    for len in [1_usize, 63, 64, 65, 127, 128, 200, 4_096] {
        let a: Vec<f64> = (0..len).map(|_| stream.signed()).collect();
        let b: Vec<f64> = (0..len).map(|_| stream.signed()).collect();
        for &path in &paths {
            executed.ran(path.path());
            let full = path
                .distance(Measure::SquaredEuclidean, &a, 0.0, &b, 0.0)
                .expect("finite");
            assert!(full > 0.0);
            let bounded =
                |bound| path.distance_bounded(Measure::SquaredEuclidean, &a, 0.0, &b, 0.0, bound);
            // Swept across bounds below, at and above the distance, in both
            // strictnesses: a value is the full value bit for bit, and an abandonment
            // is one the full value justifies.
            for scale in [0.0_f64, 0.25, 0.5, 0.99, 1.0, 1.01, 2.0] {
                let limit = full * scale;
                for bound in [Bound::AtOrAbove(limit), Bound::Above(limit)] {
                    match bounded(bound) {
                        Bounded::Below(value) => {
                            assert_eq!(value.to_bits(), full.to_bits(), "len {len} {bound:?}");
                            assert!(!bound.is_met_by(full), "len {len} {bound:?}");
                        }
                        Bounded::Beyond => {
                            assert!(bound.is_met_by(full), "len {len}: abandoned at {bound:?}");
                        }
                        Bounded::NonFinite => panic!("the fixture is finite"),
                    }
                }
            }
            // The neighbour: a bound one ulp above the distance is not met, so the
            // distance comes back in full.
            assert_eq!(
                bounded(Bound::AtOrAbove(full.next_up())),
                Bounded::Below(full),
                "{} len {len}: one ulp above",
                path.path()
            );
            assert_eq!(bounded(Bound::Above(full)), Bounded::Below(full));
            assert_eq!(bounded(Bound::AtOrAbove(full)), Bounded::Beyond);
            assert_eq!(bounded(Bound::Above(full.next_down())), Bounded::Beyond);
        }
    }

    // A checkpoint really abandons: the first block already meets the bound, and the
    // non-finite term after it is never read. The neighbour below every checkpoint
    // reaches it.
    let mut early = vec![1.0_f64; 128];
    early[100] = f64::NAN;
    let zero = vec![0.0_f64; 128];
    for &path in &paths {
        let bounded = |bound| {
            path.distance_bounded(Measure::SquaredEuclidean, &early, 0.0, &zero, 0.0, bound)
        };
        assert_eq!(bounded(Bound::AtOrAbove(10.0)), Bounded::Beyond);
        assert_eq!(bounded(Bound::AtOrAbove(1e9)), Bounded::NonFinite);
    }
    assert_ran(&executed, "reassociated_bounded_agrees_with_full");
}

#[test]
fn reassociated_overflow_is_nonfinite() {
    let paths = host_paths();
    let mut executed = Executed::default();
    for len in [1_usize, 8, 64, 65, 200] {
        let huge = vec![1e300_f64; len];
        let negative = vec![-1e300_f64; len];
        for &path in &paths {
            executed.ran(path.path());
            assert_eq!(
                path.distance(Measure::NegativeDot, &huge, 0.0, &huge, 0.0),
                None,
                "{} len {len}: a dot of 1e300 components overflows",
                path.path()
            );
            assert_eq!(
                path.distance(Measure::SquaredEuclidean, &huge, 0.0, &negative, 0.0),
                None,
                "{} len {len}: so does the distance between ±1e300",
                path.path()
            );
            for bound in [
                Bound::Above(f64::INFINITY),
                Bound::AtOrAbove(f64::INFINITY),
                Bound::AtOrAbove(1.0),
            ] {
                assert_eq!(
                    path.distance_bounded(
                        Measure::SquaredEuclidean,
                        &huge,
                        0.0,
                        &negative,
                        0.0,
                        bound
                    ),
                    Bounded::NonFinite,
                    "{} len {len} under {bound:?}: the overflow is checked before the bound",
                    path.path()
                );
            }
            let view = RowsRef::new(&negative, 1, len, &[]).expect("one row");
            let mut out = [Some(0.0)];
            path.distances(Measure::SquaredEuclidean, &huge, 0.0, view, &mut out);
            assert_eq!(out, [None], "{} len {len}: the batch form", path.path());

            // The valid neighbour: large but finite ranks.
            let large = vec![1e150_f64; len];
            let value = path
                .distance(Measure::SquaredEuclidean, &large, 0.0, &vec![0.0; len], 0.0)
                .expect("finite squares of 1e150 are ranked, not refused");
            let square = 1e150_f64 * 1e150_f64;
            assert!(
                value.is_finite() && value >= square,
                "{} len {len}",
                path.path()
            );
        }
    }
    assert_ran(&executed, "reassociated_overflow_is_nonfinite");
}

#[test]
fn no_substitution_exact_never_fast() {
    // One 64-element block. Every product is `±(1 + 2⁻³⁰)² = ±(1 + 2⁻²⁹ + 2⁻⁶⁰)`, which
    // rounds to `±(1 + 2⁻²⁹)`: the first 32 positive, the last 32 negative. Rounded
    // products cancel exactly in every order, so every unfused arithmetic, the exact
    // law included, returns zero. A fused multiply-add keeps the 2⁻⁶⁰ the rounding
    // drops: an accumulator that has summed positive products and then fuses a
    // negative one is left holding a residue of it, so any contraction moves the sum
    // off zero.
    let factor = 1.0 + 2.0_f64.powi(-30);
    let a: Vec<f64> = (0..64)
        .map(|index| if index < 32 { factor } else { -factor })
        .collect();
    let b = vec![factor; 64];

    let reference = reference_dot(&a, &b);
    assert_eq!(reference, 0.0, "the exact law's reference model");
    let exact = Exact::resolve().expect("default environment");
    let exact_dot = -exact
        .distance(Measure::NegativeDot, &a, 0.0, &b, 0.0)
        .expect("finite");
    assert_eq!(
        exact_dot.to_bits(),
        reference.to_bits(),
        "the exact arithmetic equals its reference on {}",
        exact.path()
    );

    let paths = host_paths();
    let fused = fma_paths(&paths);
    #[cfg(target_arch = "x86_64")]
    if std::is_x86_feature_detected!("avx2") && std::is_x86_feature_detected!("fma") {
        assert!(!fused.is_empty(), "an FMA-capable host runs an FMA path");
    }
    let mut executed = Executed::default();
    for path in &fused {
        executed.ran(path.path());
        let fast = -path
            .distance(Measure::NegativeDot, &a, 0.0, &b, 0.0)
            .expect("finite");
        // The precondition: on this input the two arithmetics genuinely differ, so the
        // comparisons below can tell which one ran.
        assert_ne!(
            fast.to_bits(),
            exact_dot.to_bits(),
            "{}: contraction must move this sum, or the test observes nothing",
            path.path()
        );
        assert_ne!(
            fast,
            reference,
            "{}: the reassociated arithmetic did not fall back to the exact one",
            path.path()
        );
    }
    println!(
        "no_substitution_exact_never_fast: exact on {}",
        exact.path()
    );
    let fma_compiled: Vec<Path> = reassociated_compiled()
        .iter()
        .copied()
        .filter(|path| matches!(path, Path::Avx2Fma | Path::Avx512f | Path::Neon))
        .collect();
    executed.assert_ran(
        "no_substitution_exact_never_fast",
        &fma_compiled,
        &paths_of(&fused),
    );
}

#[test]
fn evidence_text_pinned() {
    assert_eq!(
        Reassociated::evidence(Path::Avx2Fma),
        Some(
            "reassociated binary64 arithmetic: distance sums are reassociated and may be \
             contracted to fused multiply-add along the avx2+fma dispatch path of this \
             build, so results may differ in the last bits from the exact arithmetic and \
             between dispatch paths or builds, the sign of a zero result is unspecified, \
             and near-ties may order differently"
        )
    );
    for path in [
        Path::Portable,
        Path::Avx2,
        Path::Sse2,
        Path::Avx2Fma,
        Path::Avx512f,
        Path::Neon,
        Path::WasmSimd128,
        Path::WasmScalar,
    ] {
        let expected = format!(
            "reassociated binary64 arithmetic: distance sums are reassociated and may be \
             contracted to fused multiply-add along the {path} dispatch path of this build, \
             so results may differ in the last bits from the exact arithmetic and between \
             dispatch paths or builds, the sign of a zero result is unspecified, and \
             near-ties may order differently"
        );
        assert_eq!(Reassociated::evidence(path), Some(expected.as_str()));
        assert_eq!(
            Exact::evidence(path),
            None,
            "the exact law discloses nothing"
        );
    }
    let resolved = Reassociated::resolve().expect("default environment");
    assert_eq!(resolved.evidence(), Reassociated::evidence(resolved.path()));
}

#[test]
fn the_reassociated_identity_is_pinned() {
    assert_eq!(Reassociated::ID, "binary64-reassociated-v1");
    assert_ne!(Reassociated::ID, Exact::ID);
    assert_eq!(Reassociated::IMAGE_CODES, [2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(Exact::IMAGE_CODES, [1]);
    // `Portable` is a path of both arithmetics, and each records it under its own code.
    let table = [
        (Path::Portable, Some(1), Some(8)),
        (Path::Avx2, Some(1), None),
        (Path::Sse2, None, Some(2)),
        (Path::Avx2Fma, None, Some(3)),
        (Path::Avx512f, Some(1), Some(4)),
        (Path::Neon, None, Some(5)),
        (Path::WasmSimd128, None, Some(6)),
        (Path::WasmScalar, None, Some(7)),
    ];
    for (path, exact, reassociated) in table {
        assert_eq!(Exact::image_code(path), exact, "{path}");
        assert_eq!(Reassociated::image_code(path), reassociated, "{path}");
    }
    let resolved = Reassociated::resolve().expect("default environment");
    assert!(Reassociated::IMAGE_CODES.contains(&resolved.image_code()));
    assert_eq!(
        Exact::resolve().expect("default environment").image_code(),
        1
    );
}

#[test]
fn reassociated_resolves_the_path_the_processor_predicts() {
    let resolved = Reassociated::resolve().expect("the default environment is the IEEE one");
    #[cfg(target_arch = "x86_64")]
    {
        let avx2_fma =
            std::is_x86_feature_detected!("avx2") && std::is_x86_feature_detected!("fma");
        let expected = if avx2_fma && std::is_x86_feature_detected!("avx512f") {
            Path::Avx512f
        } else if avx2_fma {
            Path::Avx2Fma
        } else {
            Path::Sse2
        };
        assert_eq!(resolved.path(), expected);
    }
    #[cfg(target_arch = "aarch64")]
    assert_eq!(resolved.path(), Path::Neon);
    #[cfg(all(
        any(target_arch = "wasm32", target_arch = "wasm64"),
        target_feature = "simd128"
    ))]
    assert_eq!(resolved.path(), Path::WasmSimd128);
    #[cfg(all(
        any(target_arch = "wasm32", target_arch = "wasm64"),
        not(target_feature = "simd128")
    ))]
    assert_eq!(resolved.path(), Path::WasmScalar);
    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "wasm32",
        target_arch = "wasm64"
    )))]
    {
        assert_eq!(resolved.path(), Path::Portable);
        assert_eq!(resolved.image_code(), 8);
    }
    assert!(host_paths().contains(&resolved));
    println!(
        "reassociated_resolves_the_path_the_processor_predicts resolved path: {}",
        resolved.path()
    );
}

/// On a target with named reassociated paths the portable one is not among them: a
/// handle on it reaches no compilation, so it can never stand in for the path the
/// processor runs. The valid neighbour, a handle on the resolved path, computes.
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[test]
#[should_panic(expected = "portable is not a reassociated path of this build")]
fn the_portable_path_is_not_dispatched_where_a_named_path_exists() {
    let a = [1.0_f64, 2.0, 3.0];
    let resolved = Reassociated::resolve().expect("the default environment is the IEEE one");
    assert_ne!(resolved.path(), Path::Portable);
    assert_eq!(
        resolved.distance(Measure::NegativeDot, &a, 0.0, &a, 0.0),
        Some(-14.0),
        "the resolved path computes"
    );
    let _ = Resolved::<Reassociated>::on(Path::Portable).distance(
        Measure::NegativeDot,
        &a,
        0.0,
        &a,
        0.0,
    );
}

/// Resolve the reassociated arithmetic under MXCSR `value`, restoring the register.
#[cfg(target_arch = "x86_64")]
#[test]
fn reassociated_refuses_flush_to_zero() {
    fn set_mxcsr(value: u32) {
        // SAFETY: `ldmxcsr` loads MXCSR from a live, aligned `u32`. The value differs
        // from the saved register only in the FTZ bit, the saved value is restored
        // before any float arithmetic, and the register is per-thread.
        unsafe {
            core::arch::asm!(
                "ldmxcsr [{ptr}]",
                ptr = in(reg) &raw const value,
                options(nostack, preserves_flags, readonly),
            );
        }
    }
    let saved = env::mxcsr();
    set_mxcsr(saved | (1 << 15));
    let refused = Reassociated::resolve();
    let refused_recorded = Reassociated::resolve_recorded(2);
    set_mxcsr(saved);
    let flush = FloatEnvironmentError::FlushToZero {
        evidence: FloatEnvironmentEvidence::Register {
            name: "MXCSR",
            bits: u64::from(saved | (1 << 15)),
        },
    };
    assert_eq!(refused, Err(flush));
    assert_eq!(
        refused_recorded,
        Err(RecordedPathError::FloatEnvironment(flush)),
        "a recorded path is resolved under the same environment check"
    );
    assert!(
        Reassociated::resolve().is_ok(),
        "the valid neighbour resolves"
    );
    assert!(
        Reassociated::resolve_recorded(2).is_ok(),
        "and so does its recorded path"
    );
}

/// The path each reassociated image code names.
fn path_named(code: u32) -> Path {
    match code {
        2 => Path::Sse2,
        3 => Path::Avx2Fma,
        4 => Path::Avx512f,
        5 => Path::Neon,
        6 => Path::WasmSimd128,
        7 => Path::WasmScalar,
        8 => Path::Portable,
        other => panic!("{other} is not a reassociated image code"),
    }
}

/// Every code resolves to the path it names exactly when this host can run that path --
/// the set [`host_paths`] builds from the processor's own report -- and every other code
/// is refused naming the path and why. The widest path is one of the accepted, never the
/// only one where the processor runs several.
#[test]
fn a_recorded_path_resolves_whenever_the_host_runs_it() {
    let runnable = host_paths();
    let mut accepted = Vec::new();
    for &code in Reassociated::IMAGE_CODES {
        let path = path_named(code);
        match Reassociated::resolve_recorded(code) {
            Ok(resolved) => {
                assert_eq!(resolved.path(), path, "code {code} resolves its own path");
                assert_eq!(resolved.image_code(), code);
                assert!(
                    runnable.contains(&resolved),
                    "{path} is not one the host runs"
                );
                accepted.push(resolved);
            }
            Err(RecordedPathError::Unavailable {
                path: refused,
                reason,
            }) => {
                assert_eq!(refused, path);
                assert!(
                    !runnable.iter().any(|handle| handle.path() == path),
                    "{path} runs on this host, yet code {code} was refused: {reason}"
                );
                #[cfg(target_arch = "x86_64")]
                if matches!(path, Path::Avx2Fma | Path::Avx512f) {
                    assert!(
                        matches!(reason, PathUnavailable::MissingFeature(_)),
                        "{path} is compiled into every x86_64 build: {reason}"
                    );
                    continue;
                }
                assert_eq!(reason, PathUnavailable::NotCompiled, "{path}");
            }
            Err(other) => panic!("code {code}: {other}"),
        }
    }
    assert_eq!(
        accepted, runnable,
        "the codes accepted are exactly the paths the host runs"
    );
    let widest = Reassociated::resolve().expect("the default environment is the IEEE one");
    assert_eq!(accepted.last(), Some(&widest), "the widest is among them");
    #[cfg(target_arch = "x86_64")]
    {
        let sse2 = Reassociated::resolve_recorded(2).expect("SSE2 is every x86_64's baseline");
        assert_eq!(sse2.path(), Path::Sse2);
        if std::is_x86_feature_detected!("avx2") && std::is_x86_feature_detected!("fma") {
            assert_ne!(
                widest.path(),
                Path::Sse2,
                "an AVX2+FMA host resolves a wider path for new results"
            );
            assert_eq!(
                Reassociated::resolve_recorded(3)
                    .expect("an AVX2+FMA host runs the AVX2+FMA path")
                    .path(),
                Path::Avx2Fma
            );
        }
        // The x86_64 build holds no NEON, wasm or portable compilation.
        for code in [5, 6, 7, 8] {
            assert_eq!(
                Reassociated::resolve_recorded(code),
                Err(RecordedPathError::Unavailable {
                    path: path_named(code),
                    reason: PathUnavailable::NotCompiled,
                })
            );
        }
    }
    println!(
        "a_recorded_path_resolves_whenever_the_host_runs_it accepted paths: {}",
        names(&accepted)
    );
}

/// A code of no arithmetic, or of the other one, is refused by name; each arithmetic
/// accepts its own. The exact arithmetic has one code, which resolves to the path a new
/// result would run on, since every exact path returns the same bits.
#[test]
fn a_recorded_code_of_another_arithmetic_is_unknown() {
    for code in [0, 1, 9, u32::MAX] {
        assert_eq!(
            Reassociated::resolve_recorded(code),
            Err(RecordedPathError::UnknownCode {
                arithmetic: Reassociated::ID,
                code,
            })
        );
    }
    for code in [0, 2, 3, 4, 8] {
        assert_eq!(
            Exact::resolve_recorded(code),
            Err(RecordedPathError::UnknownCode {
                arithmetic: Exact::ID,
                code,
            })
        );
    }
    assert_eq!(
        Exact::resolve_recorded(Exact::IMAGE_CODE),
        Ok(Exact::resolve().expect("the default environment is the IEEE one"))
    );
    let refusal = Reassociated::resolve_recorded(1).expect_err("the exact code");
    assert!(refusal.to_string().contains(Reassociated::ID), "{refusal}");
}

/// A handle on a narrower recorded path runs that path's compilation, not the widest's.
///
/// On an input where contraction moves the sum, a baseline compiled without FMA returns
/// the unfused zero while an AVX2+FMA host's widest path does not, so the two are told
/// apart by their bits. A build whose baseline itself enables FMA (a `target-cpu` that
/// has it) contracts on both, so there the handle's path is what is observed.
#[cfg(target_arch = "x86_64")]
#[test]
fn a_recorded_narrower_path_computes_its_own_bits() {
    let factor = 1.0 + 2.0_f64.powi(-30);
    let a: Vec<f64> = (0..64)
        .map(|index| if index < 32 { factor } else { -factor })
        .collect();
    let b = vec![factor; 64];
    let recorded = Reassociated::resolve_recorded(2).expect("SSE2 is every x86_64's baseline");
    assert_eq!(recorded.path(), Path::Sse2);
    let dot = |handle: Resolved<Reassociated>| {
        -handle
            .distance(Measure::NegativeDot, &a, 0.0, &b, 0.0)
            .expect("finite")
    };
    let on_recorded = dot(recorded);
    assert_eq!(
        on_recorded.to_bits(),
        dot(Resolved::on(Path::Sse2)).to_bits(),
        "the recorded handle is the SSE2 compilation"
    );
    let widest = Reassociated::resolve().expect("the default environment is the IEEE one");
    let exercised = if cfg!(target_feature = "fma") {
        "baseline built with fma: path observed"
    } else {
        assert_eq!(
            on_recorded, 0.0,
            "an unfused baseline never contracts, so the products cancel"
        );
        if widest.path() == Path::Sse2 {
            "sse2-only host"
        } else {
            assert_ne!(
                dot(widest).to_bits(),
                on_recorded.to_bits(),
                "the widest path contracts on this input, so the two are told apart"
            );
            "recorded sse2 told apart from the wider path by its bits"
        }
    };
    println!("a_recorded_narrower_path_computes_its_own_bits exercised: {exercised}");
}

/// A processor without AVX-512F cannot run an image recorded on it, and is refused
/// naming the feature; its neighbour, the AVX2+FMA path, still resolves, and becomes
/// the widest. Hiding FMA refuses both, naming FMA. Hiding only ever removes a feature,
/// so each refusal is observed against the host's real report.
#[cfg(target_arch = "x86_64")]
#[test]
fn a_recorded_path_whose_feature_is_hidden_is_refused_naming_it() {
    use super::reassociated::{Feature, hidden};

    let avx2_fma = std::is_x86_feature_detected!("avx2") && std::is_x86_feature_detected!("fma");
    let avx512f = avx2_fma && std::is_x86_feature_detected!("avx512f");
    {
        let _hidden = hidden::hide(Feature::Avx512f);
        assert_eq!(
            Reassociated::resolve_recorded(4),
            Err(RecordedPathError::Unavailable {
                path: Path::Avx512f,
                reason: PathUnavailable::MissingFeature("avx512f"),
            })
        );
        let widest = Reassociated::resolve().expect("the default environment is the IEEE one");
        if avx2_fma {
            assert_eq!(
                Reassociated::resolve_recorded(3).map(Resolved::path),
                Ok(Path::Avx2Fma),
                "the neighbour path still runs"
            );
            assert_eq!(widest.path(), Path::Avx2Fma);
        } else {
            assert_eq!(widest.path(), Path::Sse2);
        }
    }
    {
        let _hidden = hidden::hide(Feature::Fma);
        let missing = if avx512f { "fma" } else { "avx512f" };
        assert_eq!(
            Reassociated::resolve_recorded(4),
            Err(RecordedPathError::Unavailable {
                path: Path::Avx512f,
                reason: PathUnavailable::MissingFeature(missing),
            })
        );
        let missing = if std::is_x86_feature_detected!("avx2") {
            "fma"
        } else {
            "avx2"
        };
        assert_eq!(
            Reassociated::resolve_recorded(3),
            Err(RecordedPathError::Unavailable {
                path: Path::Avx2Fma,
                reason: PathUnavailable::MissingFeature(missing),
            })
        );
        assert_eq!(
            Reassociated::resolve_recorded(2).map(Resolved::path),
            Ok(Path::Sse2),
            "the baseline needs no feature"
        );
    }
    // Unhidden, the host answers with its own report again.
    assert_eq!(Reassociated::resolve_recorded(4).is_ok(), avx512f);
    assert_eq!(Reassociated::resolve_recorded(3).is_ok(), avx2_fma);
    println!(
        "a_recorded_path_whose_feature_is_hidden_is_refused_naming_it host: avx2+fma {avx2_fma}, \
         avx512f {avx512f}"
    );
}

/// Every required dispatch path executes on this host, against its oracle.
///
/// The paths are every one either arithmetic runs on this host, and every one
/// [`REQUIRE_PATHS_VAR`] names. Each is reached through the processor's own
/// report: a reassociated path through [`Arithmetic::resolve_recorded`] on its image
/// code, then held to the double-double error bound and to the exact arithmetic's
/// answer within twice that bound; an exact path held to the reference model bit for
/// bit. A required path this build holds no compilation of, or whose features the
/// processor does not report, fails by name -- so a job that names a path proves the
/// path ran, not merely that it was compiled.
#[test]
fn every_required_dispatch_path_executes() {
    let exact_runnable = paths_of(&tests::host_paths());
    let mut runnable = exact_runnable.clone();
    for path in paths_of(&host_paths()) {
        if !runnable.contains(&path) {
            runnable.push(path);
        }
    }
    let mut all_compiled = exact_compiled().to_vec();
    for &path in reassociated_compiled() {
        if !all_compiled.contains(&path) {
            all_compiled.push(path);
        }
    }
    let mut targets = runnable.clone();
    for path in required_paths() {
        if !targets.contains(&path) {
            targets.push(path);
        }
    }
    let mut stream = Stream(0x5EED_0000_0000_0004);
    let fixture: Vec<(Vec<f64>, Vec<f64>)> = [0_usize, 1, 15, 16, 17, 63, 64, 65, 200, 4_096]
        .into_iter()
        .flat_map(|len| [len; 5])
        .map(|len| (stream.vector(len), stream.vector(len)))
        .collect();
    let exact = Exact::resolve().expect("the default environment is the IEEE one");
    let mut executed = Executed::default();
    for path in targets {
        let mut compiled = false;
        if reassociated_compiled().contains(&path) {
            compiled = true;
            let code = Reassociated::image_code(path).expect("a reassociated path has a code");
            let handle = Reassociated::resolve_recorded(code).unwrap_or_else(|refusal| {
                panic!("{path} must execute on this host, but its image code {code} is refused: {refusal}")
            });
            assert_eq!(handle.path(), path, "code {code} resolves its own path");
            for (a, b) in &fixture {
                let n = a.len();
                let (dot, dot_magnitude) = dd_dot(a, b);
                let dot_bound = error_bound(n, dot_magnitude);
                let fast_dot = -handle
                    .distance(Measure::NegativeDot, a, 0.0, b, 0.0)
                    .expect("finite");
                let exact_dot = -exact
                    .distance(Measure::NegativeDot, a, 0.0, b, 0.0)
                    .expect("finite");
                assert!(
                    (fast_dot - dot).abs() <= dot_bound,
                    "dot on {path} at len {n}: {fast_dot} vs reference {dot}"
                );
                assert!(
                    (fast_dot - exact_dot).abs() <= 2.0 * dot_bound,
                    "dot on {path} at len {n}: {fast_dot} vs the exact arithmetic's {exact_dot}"
                );
                let (squares, squares_magnitude) = dd_squared_euclidean(a, b);
                let squares_bound = error_bound(n, squares_magnitude);
                let fast_squares = handle
                    .distance(Measure::SquaredEuclidean, a, 0.0, b, 0.0)
                    .expect("finite");
                let exact_squares = exact
                    .distance(Measure::SquaredEuclidean, a, 0.0, b, 0.0)
                    .expect("finite");
                assert!(
                    (fast_squares - squares).abs() <= squares_bound,
                    "squared Euclidean on {path} at len {n}: {fast_squares} vs reference \
                     {squares}"
                );
                assert!(
                    (fast_squares - exact_squares).abs() <= 2.0 * squares_bound,
                    "squared Euclidean on {path} at len {n}: {fast_squares} vs the exact \
                     arithmetic's {exact_squares}"
                );
            }
            executed.ran(path);
        }
        if exact_compiled().contains(&path) {
            compiled = true;
            assert!(
                exact_runnable.contains(&path),
                "{REQUIRE_PATHS_VAR} requires the exact {path} path, which this host cannot \
                 run (it runs: {})",
                exact_runnable
                    .iter()
                    .map(|path| path.name())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            let handle = Resolved::<Exact>::on(path);
            for (a, b) in &fixture {
                assert_eq!(
                    handle
                        .distance(Measure::NegativeDot, a, 0.0, b, 0.0)
                        .map(f64::to_bits),
                    Some((-reference_dot(a, b)).to_bits()),
                    "exact dot on {path} at len {}",
                    a.len()
                );
            }
            executed.ran(path);
        }
        assert!(
            compiled,
            "{REQUIRE_PATHS_VAR} requires {path}, which this {} build holds no compilation \
             of (its paths: {})",
            std::env::consts::ARCH,
            all_compiled
                .iter()
                .map(|path| path.name())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    executed.assert_ran(
        "every_required_dispatch_path_executes",
        &all_compiled,
        &runnable,
    );
}
