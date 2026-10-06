// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use super::{CoordinateMath, DoubleDouble, FixedInterval, FloatInterval, MathError, MathLimits};
use crate::BigInt;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

fn context(bits: u32) -> CoordinateMath {
    CoordinateMath::new(MathLimits {
        precision_bits: bits,
        ..MathLimits::DEFAULT
    })
    .expect("admit math")
}

#[cfg(all(target_arch = "x86", not(target_feature = "sse2")))]
#[test]
fn x87_baseline_validates_mxcsr_before_runtime_vector_products() {
    use super::FloatProductBackend;
    use crate::ieee::{control, environment::FloatEnvironmentError};
    use purrdf_hash::Backend;
    if !FloatProductBackend::Sse2.is_available() {
        return;
    }
    let mut math = context(128);
    let saved = control::mxcsr();
    for changed in [
        saved | (1 << 15),
        saved | (1 << 13),
        (saved & !0x3f) & !(1 << 11),
    ] {
        {
            let _guard = control::Mxcsr::load(changed);
            math.set_binary64_backend(FloatProductBackend::Portable)
                .unwrap();
            drop(math.enter_chunk().unwrap());
            math.set_binary64_backend(FloatProductBackend::Sse2)
                .unwrap();
            let error = math.enter_chunk().unwrap_err();
            assert!(matches!(
                error,
                MathError::Environment(
                    FloatEnvironmentError::FlushToZero { .. }
                        | FloatEnvironmentError::RoundingMode { .. }
                        | FloatEnvironmentError::TrapsEnabled { .. }
                )
            ));
        }
        assert_eq!(control::mxcsr(), saved);
    }
    drop(math.enter_chunk().unwrap());
}
fn rational(numerator: i128, denominator: i128, ctx: &mut CoordinateMath) -> FixedInterval {
    FixedInterval::from_ratio(
        &BigInt::from_i128(numerator),
        &BigInt::from_i128(denominator),
        ctx,
    )
    .expect("rational")
}
fn encloses(value: &FixedInterval, numerator: &BigInt, denominator: &BigInt) {
    let scaled = numerator.mul_pow2(value.precision_bits());
    assert!(value.lower().mul(denominator) <= scaled);
    assert!(value.upper().mul(denominator) >= scaled);
}
fn encloses_decimal_interval(value: &FixedInterval, digits: &str, places: u32) {
    let floor = BigInt::from_digits(digits).expect("decimal witness");
    let denominator = BigInt::from_i128(1).mul_pow10(places);
    // A published decimal truncation witnesses a tiny interval, not an
    // infinitely precise transcendental truth. Require our enclosure to contain
    // that whole independent interval, at substantially finer witness precision.
    encloses(value, &floor, &denominator);
    encloses(value, &floor.add(&BigInt::from_i128(1)), &denominator);
}

#[test]
fn exact_zero_trigonometry_keeps_structural_zeros_and_admission() {
    let mut ctx = context(80);
    let zero = FixedInterval::from_i64(0, &mut ctx).unwrap();
    let one = FixedInterval::from_i64(1, &mut ctx).unwrap();
    assert_eq!(zero.sin_cos(&mut ctx).unwrap(), (zero.clone(), one.clone()));
    assert_eq!(zero.sin_cos_range(&mut ctx).unwrap(), (zero.clone(), one));
    let mut mismatched = context(112);
    assert!(matches!(
        zero.sin_cos_range(&mut mismatched),
        Err(MathError::PrecisionExhausted)
    ));
}

#[test]
fn tiny_argument_remainders_fit_independent_exact_alternating_bounds() {
    let mut ctx = context(192);
    let x = rational(1, 1_i128 << 20, &mut ctx);
    let one = rational(1, 1, &mut ctx);
    let sine_lower = x
        .sub(&rational(1, 6_i128 << 60, &mut ctx), &mut ctx)
        .unwrap();
    let sine_upper = sine_lower
        .add(&rational(1, 120_i128 << 100, &mut ctx), &mut ctx)
        .unwrap();
    let cosine_lower = one
        .sub(&rational(1, 2_i128 << 40, &mut ctx), &mut ctx)
        .unwrap();
    let cosine_upper = cosine_lower
        .add(&rational(1, 24_i128 << 80, &mut ctx), &mut ctx)
        .unwrap();
    let atan_lower = x
        .sub(&rational(1, 3_i128 << 60, &mut ctx), &mut ctx)
        .unwrap();
    let atan_upper = atan_lower
        .add(&rational(1, 5_i128 << 100, &mut ctx), &mut ctx)
        .unwrap();
    let (sine, cosine) = x.sin_cos(&mut ctx).unwrap();
    let atan = x.atan(&mut ctx).unwrap();
    for (value, lower, upper) in [
        (sine, sine_lower, sine_upper),
        (cosine, cosine_lower, cosine_upper),
        (atan, atan_lower, atan_upper),
    ] {
        assert!(value.lower() >= lower.upper());
        assert!(value.upper() <= upper.lower());
    }
}

#[test]
fn outward_signed_arithmetic_and_half_even_rounding() {
    let mut ctx = context(64);
    let third = rational(1, 3, &mut ctx);
    let negative = rational(-1, 7, &mut ctx);
    encloses(
        &third.add(&negative, &mut ctx).expect("sum"),
        &BigInt::from_i128(4),
        &BigInt::from_i128(21),
    );
    encloses(
        &third.sub(&negative, &mut ctx).expect("difference"),
        &BigInt::from_i128(10),
        &BigInt::from_i128(21),
    );
    encloses(
        &third.mul(&negative, &mut ctx).expect("product"),
        &BigInt::from_i128(-1),
        &BigInt::from_i128(21),
    );
    encloses(
        &third.div(&negative, &mut ctx).expect("quotient"),
        &BigInt::from_i128(-7),
        &BigInt::from_i128(3),
    );
    for (n, wanted) in [(1, 0), (3, 2), (5, 2), (-1, 0), (-3, -2), (-5, -2)] {
        let value = rational(n, 2, &mut ctx);
        let rounded = value.round_decimal(0, &mut ctx).expect("ties");
        assert_eq!(
            rounded,
            (BigInt::from_i128(wanted), BigInt::from_i128(wanted))
        );
    }
    let scale = BigInt::from_i128(1).mul_pow2(64);
    let crossing = FixedInterval::from_bounds(scale.negated(), scale, &mut ctx).expect("crossing");
    assert!(crossing.square(&mut ctx).expect("square").lower().is_zero());
    assert!(crossing.abs(&mut ctx).expect("absolute").lower().is_zero());
    assert_eq!(
        third.div(&crossing, &mut ctx),
        Err(MathError::PrecisionExhausted)
    );
}

#[test]
fn directed_decimal_interval_rounding_preserves_signs_exact_boundaries_and_ties() {
    use crate::ieee::ratio::Rounding::{Down, NearestEven, Up};
    for bounded in [false, true] {
        let mut math = context(128);
        if bounded {
            math.set_limb_scratch(crate::integer::LimbScratch::new(64, 16).unwrap())
                .unwrap();
        }
        for (numerator, down, nearest, up) in [
            (1, 12, 12, 13),
            (3, 37, 38, 38),
            (-1, -13, -12, -12),
            (-3, -38, -38, -37),
            (2, 25, 25, 25),
            (-2, -25, -25, -25),
            (0, 0, 0, 0),
        ] {
            let value = rational(numerator, 8, &mut math);
            for (rounding, expected) in [(Down, down), (NearestEven, nearest), (Up, up)] {
                let expected = BigInt::from_i128(expected);
                assert_eq!(
                    value.round_decimal_with(2, rounding, &mut math).unwrap(),
                    (expected.clone(), expected),
                );
            }
            assert_eq!(
                value.round_decimal(2, &mut math).unwrap(),
                value.round_decimal_with(2, NearestEven, &mut math).unwrap(),
            );
        }
        let lower = rational(-3, 8, &mut math);
        let upper = rational(5, 8, &mut math);
        let interval =
            FixedInterval::from_bounds(lower.lower().clone(), upper.upper().clone(), &mut math)
                .unwrap();
        for (rounding, lower, upper) in [(Down, -38, 62), (NearestEven, -38, 62), (Up, -37, 63)] {
            assert_eq!(
                interval.round_decimal_with(2, rounding, &mut math).unwrap(),
                (BigInt::from_i128(lower), BigInt::from_i128(upper)),
            );
        }
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let refused = interval.round_decimal_with(u32::MAX, Up, &mut math);
        let measured = window.close();
        assert_eq!(refused, Err(MathError::WorkspaceExhausted));
        assert_eq!(measured.allocations, 0);
    }
}

#[test]
fn exact_integer_quotients_and_square_roots_use_the_one_arithmetic_home() {
    let mut state = 0x8264_17a3_9823_673b;
    for index in 0..256 {
        let mut a = BigInt::zero();
        let mut b = BigInt::from_i128(1);
        for _ in 0..8 {
            state = purrdf_hash::mix::splitmix64_step(state);
            a = a.mul_pow2(64).add(&BigInt::from_i128(i128::from(state)));
            state = purrdf_hash::mix::splitmix64_step(state);
            b = b.mul_pow2(64).add(&BigInt::from_i128(i128::from(state)));
        }
        let dividend = a.mul(&b).add(&a);
        let divisor = b.add(&BigInt::from_i128(1));
        let (q, r) = dividend.div_rem(&divisor).expect("nonzero divisor");
        assert_eq!(q.mul(&divisor).add(&r), dividend);
        assert!(r >= BigInt::zero() && r < divisor);
        let root = dividend.sqrt_floor().expect("nonnegative root");
        assert!(root.mul(&root) <= dividend);
        let successor = root.add(&BigInt::from_i128(1));
        assert!(successor.mul(&successor) > dividend, "row {index}");
        for (a, b) in [
            (&dividend, divisor.negated()),
            (&dividend.negated(), divisor.clone()),
            (&dividend.negated(), divisor.negated()),
        ] {
            let (q, r) = a.div_rem(&b).expect("signed division");
            assert_eq!(q.mul(&b).add(&r), *a);
            assert!(r.abs() < b.abs());
            assert!(r.is_zero() || r.is_negative() == a.is_negative());
        }
    }
    assert_eq!(BigInt::from_i128(-1).sqrt_floor(), None);
    assert_eq!(
        BigInt::from_i128(4).sqrt_floor(),
        Some(BigInt::from_i128(2))
    );
}

#[test]
fn generated_constants_and_transcendentals_enclose_independent_decimal_witnesses() {
    let mut ctx = context(128);
    let pi = FixedInterval::pi(&mut ctx).expect("pi");
    encloses_decimal_interval(
        &pi,
        "3141592653589793238462643383279502884197169399375105820974944592307816406286208998628034825342117067",
        99,
    );
    let one = rational(1, 1, &mut ctx);
    let two = rational(2, 1, &mut ctx);
    let exp = one.exp(&mut ctx).expect("exp1");
    encloses_decimal_interval(
        &exp,
        "2718281828459045235360287471352662497757247093699959574966967627724076630353547594571382178525166427",
        99,
    );
    let logarithm = two.log(&mut ctx).expect("log2");
    encloses_decimal_interval(
        &logarithm,
        "693147180559945309417232121458176568075500134360255254120680009493393621969694715605863326996418687",
        99,
    );
    let (sin, cos) = one.sin_cos(&mut ctx).expect("trig1");
    encloses_decimal_interval(
        &sin,
        "841470984807896506652502321630298999622563060798371065672751709991910404391239668948639743543052695",
        99,
    );
    encloses_decimal_interval(
        &cos,
        "540302305868139717400936607442976603732310420617922227670097255381100394774471764517951856087183089",
        99,
    );
    let arctangent = one.atan(&mut ctx).expect("atan1");
    let quarter = pi.div(&rational(4, 1, &mut ctx), &mut ctx).expect("pi4");
    assert!(arctangent.lower() <= quarter.upper() && arctangent.upper() >= quarter.lower());
    encloses(
        &one.log(&mut ctx).expect("log1"),
        &BigInt::zero(),
        &BigInt::from_i128(1),
    );
    encloses(
        &two.sqrt(&mut ctx)
            .expect("sqrt2")
            .square(&mut ctx)
            .expect("square root enclosure"),
        &BigInt::from_i128(2),
        &BigInt::from_i128(1),
    );
}

#[test]
fn quadrants_branch_cuts_and_tighter_precision_are_explicit() {
    let mut ctx = context(96);
    let one = rational(1, 1, &mut ctx);
    let negative = rational(-1, 1, &mut ctx);
    let zero = rational(0, 1, &mut ctx);
    let pi = FixedInterval::pi(&mut ctx).expect("pi");
    assert_eq!(
        FixedInterval::atan2(&zero, &negative, &mut ctx).expect("negative axis"),
        pi
    );
    let reaching_cut = FixedInterval::from_bounds(BigInt::from_i128(-1), BigInt::zero(), &mut ctx)
        .expect("cut endpoint");
    assert_eq!(
        FixedInterval::atan2(&reaching_cut, &negative, &mut ctx),
        Err(MathError::PrecisionExhausted)
    );
    assert!(matches!(
        FixedInterval::atan2(&zero, &zero, &mut ctx),
        Err(MathError::Domain(_))
    ));
    for (y, x, sign) in [
        (&one, &one, 1),
        (&one, &negative, 1),
        (&negative, &negative, -1),
        (&negative, &one, -1),
    ] {
        let angle = FixedInterval::atan2(y, x, &mut ctx).expect("quadrant");
        assert_eq!(angle.lower().is_negative(), sign < 0);
        let (sine, cosine) = angle.sin_cos(&mut ctx).expect("phase");
        assert_eq!(sine.lower().is_negative(), y.lower().is_negative());
        assert_eq!(cosine.lower().is_negative(), x.lower().is_negative());
    }
    let mut higher = context(256);
    let higher_pi = FixedInterval::pi(&mut higher).expect("refined pi");
    assert!(pi.lower().mul_pow2(160) <= *higher_pi.lower());
    assert!(pi.upper().mul_pow2(160) >= *higher_pi.upper());
}

#[test]
fn admission_and_cancellation_never_return_partial_values() {
    assert!(matches!(
        CoordinateMath::new(MathLimits {
            precision_bits: 15,
            ..MathLimits::DEFAULT
        }),
        Err(MathError::PrecisionExhausted)
    ));
    let mut ctx = context(64);
    let reserved = ctx.limits().max_workspace_bytes - ctx.workspace_peak();
    assert_eq!(ctx.reserve_workspace(reserved), Ok(()));
    assert_eq!(
        FixedInterval::pi(&mut ctx),
        Err(MathError::WorkspaceExhausted)
    );
    ctx.release_workspace(reserved).expect("release");
    ctx.set_cancellation(Arc::new(AtomicBool::new(true)));
    assert_eq!(FixedInterval::pi(&mut ctx), Err(MathError::Cancelled));
    let mut tiny = CoordinateMath::new(MathLimits {
        max_work: 1,
        ..MathLimits::DEFAULT
    })
    .expect("tiny context");
    assert_eq!(FixedInterval::pi(&mut tiny), Err(MathError::WorkExhausted));
    let mut other = context(32);
    let value = rational(1, 1, &mut other);
    assert_eq!(
        value.add(&value, &mut context(64)),
        Err(MathError::PrecisionExhausted)
    );
}

#[test]
fn controlled_float_intervals_and_expansion_residuals_cover_exact_rationals() {
    let mut ctx = context(128);
    let chunk = ctx.enter_chunk().expect("validated chunk");
    let third = FloatInterval::point(1.0)
        .expect("one")
        .div(FloatInterval::point(3.0).expect("three"), &chunk)
        .expect("third");
    let exact_third = rational(1, 3, &mut ctx);
    let lower_third = FixedInterval::from_binary64(third.lower(), &mut ctx).expect("lower dyadic");
    let upper_third = FixedInterval::from_binary64(third.upper(), &mut ctx).expect("upper dyadic");
    assert!(lower_third.lower() <= exact_third.lower());
    assert!(upper_third.upper() >= exact_third.upper());
    let big = DoubleDouble::from_binary64(1.0e16).expect("big");
    let small = DoubleDouble::from_binary64(1.0).expect("small");
    let sum = big.add(small, &chunk).expect("sum");
    assert_eq!(sum.hi(), 1.0e16);
    assert_eq!(sum.lo(), 1.0);
    let product = sum.mul(sum, &chunk).expect("product");
    let exact =
        BigInt::from_i128(10_000_000_000_000_001).mul(&BigInt::from_i128(10_000_000_000_000_001));
    let bounds = product.enclosure(&chunk).expect("enclosure");
    let lower = FixedInterval::from_binary64(bounds.lower(), &mut ctx).expect("lower");
    let upper = FixedInterval::from_binary64(bounds.upper(), &mut ctx).expect("upper");
    assert!(lower.lower() <= &exact.mul_pow2(128));
    assert!(upper.upper() >= &exact.mul_pow2(128));
}

#[test]
fn outward_binary64_conversion_keeps_extremes_and_subnormal_bits_exact() {
    let mut ctx = context(1200);
    let mut cases = vec![
        0,
        1,
        2,
        0x000f_ffff_ffff_ffff,
        0x0010_0000_0000_0000,
        0x3ff0_0000_0000_0000,
        0x3ff0_0000_0000_0001,
        0x7fef_ffff_ffff_ffff,
    ];
    let mut state = 0x1536_ac69_5374_ab76;
    for _ in 0..64 {
        state = purrdf_hash::mix::splitmix64_step(state);
        let bits = state & 0x7fff_ffff_ffff_ffff;
        if f64::from_bits(bits).is_finite() {
            cases.push(bits);
        }
    }
    for bits in cases {
        for sign in [0, 1_u64 << 63] {
            let value = f64::from_bits(bits | sign);
            let exact = FixedInterval::from_binary64(value, &mut ctx).expect("exact dyadic");
            assert_eq!(exact.lower(), exact.upper());
            let enclosure = exact.to_binary64(&mut ctx).expect("outward bits");
            if bits == 0 {
                assert_eq!(enclosure.lower(), 0.0);
                assert_eq!(enclosure.upper(), 0.0);
            } else {
                assert_eq!(enclosure.lower().to_bits(), value.to_bits());
                assert_eq!(enclosure.upper().to_bits(), value.to_bits());
            }
        }
    }
    let huge = FixedInterval::from_ratio(
        &BigInt::from_i128(1).mul_pow2(1024),
        &BigInt::from_i128(1),
        &mut ctx,
    )
    .expect("large integer");
    assert_eq!(huge.to_binary64(&mut ctx), Err(MathError::Binary64Range));
}

#[global_allocator]
static ALLOCATOR: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;

fn fast_contains_fixed(fast: FloatInterval, exact: &FixedInterval, math: &mut CoordinateMath) {
    let lower = FixedInterval::from_binary64(fast.lower(), math).expect("fast lower bits");
    let upper = FixedInterval::from_binary64(fast.upper(), math).expect("fast upper bits");
    assert!(
        lower.lower() <= exact.lower(),
        "fast lower excludes exact enclosure"
    );
    assert!(
        upper.upper() >= exact.upper(),
        "fast upper excludes exact enclosure"
    );
}

#[test]
fn fast_series_remainder_bounds_follow_integer_inequalities() {
    let mut factorial = BigInt::from_i128(1);
    for n in 1..=23 {
        factorial = factorial.mul_small(n);
        if n == 20 {
            assert!(factorial >= BigInt::from_i128(1).mul_pow2(61));
        }
        if n == 23 {
            assert!(factorial >= BigInt::from_i128(3).mul_pow2(72));
        }
    }
    let mut power = BigInt::from_i128(1);
    for _ in 0..49 {
        power = power.mul_small(3);
    }
    assert!(power.mul_small(4) >= BigInt::from_i128(9).mul_pow2(76));
}

#[test]
fn fast_transcendental_enclosures_contain_the_independent_exact_engine() {
    let mut math = CoordinateMath::new(MathLimits {
        precision_bits: 192,
        max_work: u64::MAX,
        ..MathLimits::DEFAULT
    })
    .expect("admit math");
    math.prepare_binary64().expect("generate coefficients");
    let chunk = math.enter_chunk().expect("validated chunk");
    let mut state = 0x7367_83ab_4613_623a;
    for _ in 0..128 {
        state = purrdf_hash::mix::splitmix64_step(state);
        let magnitude = f64::from_bits((1027_u64 << 52) | (state & ((1_u64 << 52) - 1)));
        let angle = if state >> 63 == 0 {
            magnitude
        } else {
            -magnitude
        };
        let source = FloatInterval::point(angle).expect("finite angle");
        let exact = FixedInterval::from_binary64(angle, &mut math).expect("exact angle bits");
        let (fast_sine, fast_cosine) = source.sin_cos(&mut math, &chunk).expect("fast trig");
        let (sine, cosine) = exact.sin_cos(&mut math).expect("fixed trig");
        fast_contains_fixed(fast_sine, &sine, &mut math);
        fast_contains_fixed(fast_cosine, &cosine, &mut math);
        fast_contains_fixed(
            source.atan(&mut math, &chunk).expect("fast atan"),
            &exact.atan(&mut math).expect("fixed atan"),
            &mut math,
        );
        let positive = FloatInterval::point(magnitude).expect("positive");
        let exact_positive =
            FixedInterval::from_binary64(magnitude, &mut math).expect("exact positive");
        fast_contains_fixed(
            positive.log(&mut math, &chunk).expect("fast log"),
            &exact_positive.log(&mut math).expect("fixed log"),
            &mut math,
        );
        fast_contains_fixed(
            source.exp(&mut math, &chunk).expect("fast exp"),
            &exact.exp(&mut math).expect("fixed exp"),
            &mut math,
        );
    }
    assert_eq!(math.execution_stats().fixed_fallbacks, 0);
}

#[test]
fn fast_axes_subnormals_branch_cuts_and_exact_decimal_rounding() {
    let mut math = CoordinateMath::new(MathLimits {
        precision_bits: 1200,
        max_work: u64::MAX,
        ..MathLimits::DEFAULT
    })
    .expect("admit math");
    let chunk = math.enter_chunk().expect("validated chunk");
    let one = FloatInterval::point(1.0).expect("one");
    let tiny = FloatInterval::point(f64::from_bits(1)).expect("smallest positive");
    let angle =
        FloatInterval::atan2(one, tiny, &mut math, &chunk).expect("finite singularity neighbor");
    let exact_one = FixedInterval::from_i64(1, &mut math).expect("fixed one");
    let exact_tiny =
        FixedInterval::from_binary64(f64::from_bits(1), &mut math).expect("exact subnormal");
    let exact_angle =
        FixedInterval::atan2(&exact_one, &exact_tiny, &mut math).expect("exact atan2");
    fast_contains_fixed(angle, &exact_angle, &mut math);
    let cut = FloatInterval::from_bounds(-f64::from_bits(1), 0.0).expect("cut endpoint");
    assert_eq!(
        FloatInterval::atan2(cut, one.neg(), &mut math, &chunk),
        Err(MathError::PrecisionExhausted)
    );
    for value in [
        f64::from_bits(1),
        f64::MIN_POSITIVE,
        0.5,
        1.5,
        2.5,
        -0.5,
        -1.5,
        -2.5,
        f64::MAX,
    ] {
        let interval = FloatInterval::point(value).expect("finite");
        let fixed = FixedInterval::from_binary64(value, &mut math).expect("exact bits");
        assert_eq!(
            interval
                .round_decimal(0, &mut math, &chunk)
                .expect("exact float ties"),
            fixed.round_decimal(0, &mut math).expect("exact grid ties")
        );
    }
    for value in [f64::from_bits(1), f64::MIN_POSITIVE, 1.0, 2.0, f64::MAX] {
        let source = FloatInterval::point(value).expect("finite positive");
        let exact = FixedInterval::from_binary64(value, &mut math).expect("exact bits");
        fast_contains_fixed(
            source.log(&mut math, &chunk).expect("fast log extremes"),
            &exact.log(&mut math).expect("fixed log extremes"),
            &mut math,
        );
    }
    for value in [-740.0, -100.0, -1.0, 0.0, 1.0, 100.0, 709.0] {
        let source = FloatInterval::point(value).expect("finite");
        let exact = FixedInterval::from_binary64(value, &mut math).expect("exact bits");
        fast_contains_fixed(
            source.exp(&mut math, &chunk).expect("fast exp extremes"),
            &exact.exp(&mut math).expect("fixed exp extremes"),
            &mut math,
        );
    }
}

#[test]
fn prepared_fast_inner_kernels_allocate_nothing_and_tables_share_across_workers() {
    let mut math = CoordinateMath::new(MathLimits {
        max_work: u64::MAX,
        ..MathLimits::DEFAULT
    })
    .expect("admit math");
    let prepared = math
        .prepare_binary64()
        .expect("prepare shared coefficients");
    let mut worker = CoordinateMath::new(MathLimits {
        max_work: u64::MAX,
        ..MathLimits::DEFAULT
    })
    .expect("worker math");
    worker
        .install_binary64(prepared)
        .expect("install immutable coefficients");
    let chunk = worker.enter_chunk().expect("validated chunk");
    let source = FloatInterval::point(0.7).expect("finite source");
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    for _ in 0..4096 {
        std::hint::black_box(source.sin_cos(&mut worker, &chunk).expect("fast trig"));
        std::hint::black_box(source.atan(&mut worker, &chunk).expect("fast atan"));
        std::hint::black_box(
            FloatInterval::atan2(source, source, &mut worker, &chunk).expect("fast atan2"),
        );
        std::hint::black_box(source.log(&mut worker, &chunk).expect("fast log"));
        std::hint::black_box(source.exp(&mut worker, &chunk).expect("fast exp"));
    }
    let measured = window.close();
    assert_eq!(measured.allocations, 0);
    assert_eq!(worker.execution_stats().fixed_fallbacks, 0);
    assert_eq!(worker.execution_stats().binary64_preparations, 0);
    assert_eq!(worker.execution_stats().binary64_requests, 4096 * 5);
}

#[test]
fn directed_basic_residual_signs_enclose_exact_arithmetic_across_exponents() {
    let mut math = CoordinateMath::new(MathLimits {
        precision_bits: 2300,
        max_work: u64::MAX,
        ..MathLimits::DEFAULT
    })
    .expect("wide exact oracle");
    let chunk = math.enter_chunk().expect("controlled chunk");
    let mut state = 0x5ae8_b376_8921_6c04;
    for _ in 0..512 {
        let mut draw = || {
            state = purrdf_hash::mix::splitmix64_step(state);
            let exponent = 1023 + (state % 901) as i32 - 450;
            state = purrdf_hash::mix::splitmix64_step(state);
            f64::from_bits(
                (state & ((1 << 52) - 1)) | ((exponent as u64) << 52) | (state & (1 << 63)),
            )
        };
        let a = draw();
        let b = draw();
        let fixed_a = FixedInterval::from_binary64(a, &mut math).expect("exact a");
        let fixed_b = FixedInterval::from_binary64(b, &mut math).expect("exact b");
        let fa = FloatInterval::point(a).expect("finite a");
        let fb = FloatInterval::point(b).expect("finite b");
        let exact_product = fixed_a.mul(&fixed_b, &mut math).expect("exact product");
        for path in <super::FloatProductBackend as purrdf_hash::Backend>::all_available() {
            math.set_binary64_backend(path).unwrap();
            let product_chunk = math.enter_chunk().unwrap();
            fast_contains_fixed(
                fa.mul(fb, &product_chunk).unwrap(),
                &exact_product,
                &mut math,
            );
        }
        for (fast, exact) in [
            (
                fa.add(fb, &chunk).expect("sum"),
                fixed_a.add(&fixed_b, &mut math).expect("exact sum"),
            ),
            (
                fa.sub(fb, &chunk).expect("difference"),
                fixed_a.sub(&fixed_b, &mut math).expect("exact difference"),
            ),
            (
                fa.mul(fb, &chunk).expect("product"),
                fixed_a.mul(&fixed_b, &mut math).expect("exact product"),
            ),
            (
                fa.div(fb, &chunk).expect("quotient"),
                fixed_a.div(&fixed_b, &mut math).expect("exact quotient"),
            ),
            (
                fa.abs().sqrt(&chunk).expect("sqrt"),
                fixed_a
                    .abs(&mut math)
                    .expect("absolute")
                    .sqrt(&mut math)
                    .expect("exact sqrt"),
            ),
        ] {
            fast_contains_fixed(fast, &exact, &mut math);
        }
    }
}

#[test]
fn floor_bounds_keep_negative_noninteger_and_exact_boundary_order() {
    let mut math = context(64);
    for (n, d, floor) in [(3, 2, 1), (-3, 2, -2), (2, 1, 2), (-2, 1, -2), (0, 3, 0)] {
        let value = rational(n, d, &mut math);
        let bounds = value.floor_bounds(&mut math).unwrap();
        assert_eq!(bounds, (BigInt::from_i128(floor), BigInt::from_i128(floor)));
    }
    let low = rational(-3, 2, &mut math);
    let high = rational(5, 2, &mut math);
    let interval = low.hull(&high, &mut math).unwrap();
    assert_eq!(
        interval.floor_bounds(&mut math).unwrap(),
        (BigInt::from_i128(-2), BigInt::from_i128(2))
    );
}

#[test]
fn exact_cost_refuses_before_charging_or_retaining_failed_scratch() {
    use crate::integer::ExactArithmeticCost;
    let mut math = context(64);
    let work = math.work_used();
    let reserved = math.workspace_reserved();
    let peak = math.workspace_peak();
    let denied = ExactArithmeticCost {
        work_items: 1,
        workspace_bytes: u64::MAX,
        output_bits: 1,
    };
    assert_eq!(
        math.admit_exact_cost(denied),
        Err(MathError::WorkspaceExhausted)
    );
    assert_eq!(math.work_used(), work);
    assert_eq!(math.workspace_reserved(), reserved);
    assert_eq!(math.workspace_peak(), peak);
    let denied = ExactArithmeticCost {
        work_items: u64::MAX,
        workspace_bytes: 0,
        output_bits: 1,
    };
    assert_eq!(math.admit_exact_cost(denied), Err(MathError::WorkExhausted));
    assert_eq!(math.work_used(), work);
    assert_eq!(math.workspace_reserved(), reserved);
    let admitted =
        ExactArithmeticCost::for_operation(crate::integer::ExactOperation::RationalAdd, 8, 1)
            .unwrap();
    math.admit_exact_cost(admitted)
        .expect("admitted full reduction body");
    assert_eq!(math.work_used(), work + admitted.work_items);
    assert_eq!(math.workspace_reserved(), reserved);
}

#[test]
fn interval_extrema_preserve_signed_boxes_and_admitted_detached_inputs() {
    let mut math = context(192);
    for a in -3..=3 {
        for b in a..=3 {
            for c in -3..=3 {
                for d in c..=3 {
                    let left = FixedInterval::from_bounds(
                        BigInt::from_i128(a),
                        BigInt::from_i128(b),
                        &mut math,
                    )
                    .unwrap();
                    let right = FixedInterval::from_bounds(
                        BigInt::from_i128(c),
                        BigInt::from_i128(d),
                        &mut math,
                    )
                    .unwrap();
                    let minimum = left.minimum(&right, &mut math).unwrap();
                    let maximum = left.maximum(&right, &mut math).unwrap();
                    assert_eq!(minimum.lower(), &BigInt::from_i128(a.min(c)));
                    assert_eq!(minimum.upper(), &BigInt::from_i128(b.min(d)));
                    assert_eq!(maximum.lower(), &BigInt::from_i128(a.max(c)));
                    assert_eq!(maximum.upper(), &BigInt::from_i128(b.max(d)));
                    assert_eq!(
                        minimum.add(&maximum, &mut math).unwrap(),
                        left.add(&right, &mut math).unwrap(),
                    );
                }
            }
        }
    }
    let mut other = context(224);
    let mismatch = FixedInterval::from_i64(1, &mut other).unwrap();
    let left = FixedInterval::from_i64(-9, &mut math).unwrap().detached();
    let right = FixedInterval::from_i64(7, &mut math).unwrap().detached();
    assert_eq!(
        left.minimum(&mismatch, &mut math),
        Err(MathError::PrecisionExhausted)
    );
    math.set_limb_scratch(crate::integer::LimbScratch::new(16, 32).unwrap())
        .unwrap();
    let control = purrdf_alloc_probe::CurrentThreadWindow::open();
    std::hint::black_box(Vec::<u8>::with_capacity(512));
    assert_eq!(control.close().allocations, 1);
    let measured = purrdf_alloc_probe::CurrentThreadWindow::open();
    let minimum = left.minimum(&right, &mut math).unwrap();
    let maximum = left.maximum(&right, &mut math).unwrap();
    assert_eq!(measured.close().allocations, 0);
    assert_eq!(minimum.lower(), left.lower());
    assert_eq!(maximum.upper(), right.upper());
}

#[test]
fn signed_product_corners_equal_independent_exact_extrema() {
    let mut math = context(112);
    for a in -4..=4 {
        for b in a..=4 {
            for c in -4..=4 {
                for d in c..=4 {
                    let left = FixedInterval::from_bounds(
                        BigInt::from_i128(a),
                        BigInt::from_i128(b),
                        &mut math,
                    )
                    .unwrap();
                    let right = FixedInterval::from_bounds(
                        BigInt::from_i128(c),
                        BigInt::from_i128(d),
                        &mut math,
                    )
                    .unwrap();
                    let result = left.mul(&right, &mut math).unwrap();
                    let products = [a * c, a * d, b * c, b * d];
                    let lower = *products.iter().min().unwrap();
                    let upper = *products.iter().max().unwrap();
                    // Endpoint integers denote units of 2^-112: products have
                    // units 2^-224. These small integers yield floor=-1/0 and
                    // ceiling=0/1, so this independently checks every sign case.
                    assert_eq!(
                        result.lower(),
                        &BigInt::from_i128(if lower < 0 { -1 } else { 0 })
                    );
                    assert_eq!(result.upper(), &BigInt::from_i128(i128::from(upper > 0)));
                }
            }
        }
    }
}

#[test]
fn integer_endpoint_floors_and_ceilings_include_signed_exact_boundaries() {
    let mut math = context(96);
    for numerator in -17_i128..=17 {
        for denominator in 1_i128..=8 {
            let value = rational(numerator, denominator, &mut math);
            let (floor_lower, floor_upper) = value.floor_bounds(&mut math).unwrap();
            let (ceil_lower, ceil_upper) = value.ceil_bounds(&mut math).unwrap();
            let floor = numerator.div_euclid(denominator);
            let ceil = floor + i128::from(numerator.rem_euclid(denominator) != 0);
            assert!(floor_lower <= BigInt::from_i128(floor));
            assert!(floor_upper >= BigInt::from_i128(floor));
            assert!(ceil_lower <= BigInt::from_i128(ceil));
            assert!(ceil_upper >= BigInt::from_i128(ceil));
            if denominator & (denominator - 1) == 0 || numerator % denominator == 0 {
                assert_eq!(floor_lower, BigInt::from_i128(floor));
                assert_eq!(floor_upper, BigInt::from_i128(floor));
                assert_eq!(ceil_lower, BigInt::from_i128(ceil));
                assert_eq!(ceil_upper, BigInt::from_i128(ceil));
            }
        }
    }
}

#[test]
fn high_precision_scoped_arithmetic_reuses_its_admitted_scale_storage() {
    let mut math = CoordinateMath::new(MathLimits {
        precision_bits: 448,
        max_work: u64::MAX,
        ..MathLimits::DEFAULT
    })
    .unwrap();
    let scratch = crate::integer::LimbScratch::new(64, 64).unwrap();
    math.set_limb_scratch(scratch).unwrap();
    let argument = rational(1, 3, &mut math);
    argument.sin_cos(&mut math).unwrap();
    let instrument = purrdf_alloc_probe::CurrentThreadWindow::open();
    std::hint::black_box(Vec::<u8>::with_capacity(512));
    assert_eq!(instrument.close().allocations, 1);
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    for _ in 0..8 {
        std::hint::black_box(argument.sin_cos(&mut math).unwrap());
    }
    assert_eq!(window.close().allocations, 0);
}

#[test]
fn insufficient_scale_destination_restores_the_owned_scratch_reservation() {
    for owned in [false, true] {
        let mut math = context(448);
        let scale = math.scale.clone();
        let reserved = math.workspace_reserved();
        let scratch = crate::integer::LimbScratch::new(8, 3).unwrap();
        let refused = if owned {
            math.set_limb_scratch(scratch)
        } else {
            math.set_borrowed_limb_scratch(scratch)
        };
        assert_eq!(
            refused,
            Err(MathError::LimbScratch(
                crate::integer::LimbScratchError::Capacity {
                    required_limbs: 8,
                    admitted_limbs: 3,
                }
            ))
        );
        assert_eq!(math.scale, scale);
        assert_eq!(math.workspace_reserved(), reserved);
        assert!(math.limb_scratch().is_none());
    }
}

#[test]
fn dyadic_powers_use_exact_grid_endpoints_and_refuse_growth_before_allocation() {
    let mut math = context(448);
    math.set_limb_scratch(crate::integer::LimbScratch::new(64, 64).unwrap())
        .unwrap();
    for exponent in [-512, -448, -96, -1, 0, 1, 64, 300] {
        let value = FixedInterval::power_of_two(exponent, &mut math).unwrap();
        if exponent < -448 {
            assert!(value.lower().is_zero());
            assert_eq!(value.upper(), &BigInt::from_i128(1));
        } else {
            let exact = BigInt::from_i128(1).mul_pow2((448 + exponent).unsigned_abs());
            assert_eq!(value.lower(), &exact);
            assert_eq!(value.upper(), &exact);
        }
    }
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let refused = FixedInterval::power_of_two(i32::MIN, &mut math);
    let measured = window.close();
    assert_eq!(refused, Err(MathError::WorkspaceExhausted));
    assert_eq!(measured.allocations, 0);
}

#[test]
fn borrowed_arena_context_construction_allocates_no_integer_temporary() {
    let scratch = crate::integer::LimbScratch::new(64, 64).unwrap();
    let limits = MathLimits {
        precision_bits: 448,
        ..MathLimits::DEFAULT
    };
    let expected = BigInt::from_i128(1).mul_pow2(448);
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    for _ in 0..16 {
        let math = CoordinateMath::new_with_borrowed_limb_scratch(limits, scratch.clone()).unwrap();
        assert_eq!(math.scale, expected);
        std::hint::black_box(math);
    }
    assert_eq!(window.close().allocations, 0);
    assert_eq!(scratch.available(), scratch.destination_capacity());
}

#[test]
fn sign_selected_interval_products_enclose_every_endpoint_product() {
    let mut math = context(64);
    let chunk = math.enter_chunk().unwrap();
    let ops = chunk.ops();
    let mut state = 0x2545_f491_4f6c_dd1d_u64;
    let mut draw = |zero: bool| {
        let bits = purrdf_hash::mix::splitmix64_next(&mut state);
        if zero && bits.is_multiple_of(7) {
            return 0.0;
        }
        // Signed values spanning many binades, including exact small integers.
        let magnitude =
            f64::from_bits(((bits >> 12) & ((1 << 52) - 1)) | ((1015 + (bits % 20)) << 52));
        if bits & (1 << 63) == 0 {
            magnitude
        } else {
            -magnitude
        }
    };
    for _ in 0..20_000 {
        let pair = |a: f64, b: f64| FloatInterval::from_bounds(a.min(b), a.max(b)).unwrap();
        let x = pair(draw(true), draw(true));
        let y = pair(draw(true), draw(true));
        let product = x.mul(y, &chunk).unwrap();
        let products = [
            super::floating::reference_product(x.lower(), y.lower(), ops),
            super::floating::reference_product(x.lower(), y.upper(), ops),
            super::floating::reference_product(x.upper(), y.lower(), ops),
            super::floating::reference_product(x.upper(), y.upper(), ops),
        ];
        // The exact-residual hull of the four endpoint products is the tightest
        // outward enclosure; the selected path contains it, within one ulp.
        let lower = products.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
        let upper = products
            .iter()
            .map(|p| p.1)
            .fold(f64::NEG_INFINITY, f64::max);
        assert!(
            product.lower() <= lower && upper <= product.upper(),
            "{x:?} {y:?}"
        );
        assert!(product.lower() >= lower.next_down() && product.upper() <= upper.next_up());
        if x.lower() == 0.0 && x.upper() == 0.0 {
            assert_eq!((product.lower(), product.upper()), (0.0, 0.0));
        }
    }
}

#[test]
#[allow(
    clippy::drop_non_drop,
    reason = "the chunk restores the x87 control word on drop and must end before the \
              fixed-point oracle"
)]
fn mean_value_arctangent_encloses_every_sampled_point_of_its_interval() {
    let mut state = 0x6a09_e667_f3bc_c908_u64;
    for case in 0..400_u32 {
        let mut math = context(160);
        let bits = purrdf_hash::mix::splitmix64_next(&mut state);
        // Signed centres from 1e-6 to 1e6 with relative widths up to 2^-20.
        let exponent = 1003 + (bits % 40);
        let centre = f64::from_bits(((bits >> 12) & ((1 << 52) - 1)) | (exponent << 52));
        let centre = if case % 2 == 0 { centre } else { -centre };
        let width = f64::from_bits(((1023 - 20 - (bits % 30)) << 52) | (bits >> 40));
        let delta = purrdf_xsd_test_mul(centre.abs(), width);
        let (lower, upper) = (centre - delta, centre + delta);
        let interval = FloatInterval::from_bounds(lower, upper).unwrap();
        let chunk = math.enter_chunk().unwrap();
        let enclosure = interval.atan(&mut math, &chunk).unwrap();
        drop(chunk);
        for sample in [lower, centre, upper] {
            let exact = FixedInterval::from_binary64(sample, &mut math)
                .unwrap()
                .atan(&mut math)
                .unwrap()
                .to_binary64(&mut math)
                .unwrap();
            assert!(
                enclosure.lower() <= exact.lower() && exact.upper() <= enclosure.upper(),
                "{interval:?} {sample} {enclosure:?} {exact:?}"
            );
        }
    }
}

fn purrdf_xsd_test_mul(a: f64, b: f64) -> f64 {
    a * b
}

#[test]
#[allow(
    clippy::drop_non_drop,
    clippy::suboptimal_flops,
    reason = "the chunk restores the x87 control word on drop and must end before the \
              fixed-point oracle; oracle arithmetic deliberately avoids fused multiply-add"
)]
fn binary64_series_enclose_independent_fixed_point_values_over_narrow_intervals() {
    let mut state = 0xbb67_ae85_84ca_a73b_u64;
    for case in 0..300_u32 {
        let mut math = context(160);
        let bits = purrdf_hash::mix::splitmix64_next(&mut state);
        // Centres in [-6, 6) for sine/cosine and [0.1, 6) for logarithm and
        // exponential, with relative widths from 2^-52 to 2^-30.
        let unit = f64::from_bits((bits >> 12) | (1023 << 52)) - 1.0;
        let centre = unit * 12.0 - 6.0;
        let width = f64::from_bits(((1023 - 30 - (bits % 22)) << 52) | (bits >> 40));
        let spread = centre.abs().max(1.0) * width;
        let interval = FloatInterval::from_bounds(centre - spread, centre + spread).unwrap();
        let positive = FloatInterval::from_bounds(
            unit * 5.9 + 0.1 - spread * 0.01,
            unit * 5.9 + 0.1 + spread * 0.01,
        )
        .unwrap();
        let chunk = math.enter_chunk().unwrap();
        let (sine, cosine) = interval.sin_cos(&mut math, &chunk).unwrap();
        let exponential = positive.exp(&mut math, &chunk).unwrap();
        let logarithm = positive.log(&mut math, &chunk).unwrap();
        drop(chunk);
        for sample in [interval.lower(), centre, interval.upper()] {
            let (s, c) = FixedInterval::from_binary64(sample, &mut math)
                .unwrap()
                .sin_cos(&mut math)
                .unwrap();
            let (s, c) = (
                s.to_binary64(&mut math).unwrap(),
                c.to_binary64(&mut math).unwrap(),
            );
            assert!(
                sine.lower() <= s.lower() && s.upper() <= sine.upper(),
                "{case} sin {sample}"
            );
            assert!(
                cosine.lower() <= c.lower() && c.upper() <= cosine.upper(),
                "{case} cos {sample}"
            );
        }
        for sample in [positive.lower(), positive.upper()] {
            let x = FixedInterval::from_binary64(sample, &mut math).unwrap();
            let e = x.exp(&mut math).unwrap().to_binary64(&mut math).unwrap();
            let l = x.log(&mut math).unwrap().to_binary64(&mut math).unwrap();
            assert!(
                exponential.lower() <= e.lower() && e.upper() <= exponential.upper(),
                "{case} exp"
            );
            assert!(
                logarithm.lower() <= l.lower() && l.upper() <= logarithm.upper(),
                "{case} log"
            );
        }
        // The enclosures stay narrow: a few units in the last place beyond the
        // argument spread itself.
        assert!(
            sine.upper() - sine.lower() <= 4.0 * spread + 1e-14,
            "{case} sine width"
        );
    }
}

#[test]
#[allow(
    clippy::drop_non_drop,
    clippy::suboptimal_flops,
    reason = "the chunk restores the x87 control word on drop and must end before the \
              fixed-point oracle; oracle arithmetic deliberately avoids fused multiply-add"
)]
fn double_word_intervals_enclose_fixed_references_to_about_one_hundred_bits() {
    use super::{Word, WordInterval};
    use crate::math::FloatEnclosure;
    // An exact rational of a double-word endpoint, on the fixed grid.
    fn fixed_word(word: Word, math: &mut CoordinateMath) -> FixedInterval {
        let hi = FixedInterval::from_binary64(word.hi(), math).unwrap();
        let lo = FixedInterval::from_binary64(word.lo(), math).unwrap();
        hi.add(&lo, math).unwrap()
    }
    fn contains(
        enclosure: WordInterval,
        reference: &FixedInterval,
        math: &mut CoordinateMath,
    ) -> bool {
        let lower = fixed_word(*enclosure.lower(), math);
        let upper = fixed_word(*enclosure.upper(), math);
        lower.lower() <= reference.lower() && reference.upper() <= upper.upper()
    }
    fn relative_width(enclosure: WordInterval) -> f64 {
        let lower = enclosure.lower().hi() + enclosure.lower().lo();
        let upper = enclosure.upper().hi() + enclosure.upper().lo();
        let width = (upper.hi_part() - lower.hi_part()).abs()
            + (enclosure.upper().lo() - enclosure.lower().lo()).abs();
        width / lower.abs().max(upper.abs()).max(1e-300)
    }
    trait Hi {
        fn hi_part(self) -> f64;
    }
    impl Hi for f64 {
        fn hi_part(self) -> f64 {
            self
        }
    }
    let _ = FloatEnclosure::from_binary64_bounds(0.0, 0.0);
    let mut state = 0x3c6e_f372_fe94_f82b_u64;
    let mut draw = || {
        let bits = purrdf_hash::mix::splitmix64_next(&mut state);
        let unit = f64::from_bits((bits >> 12) | (1023 << 52)) - 1.0;
        let tail = f64::from_bits(((bits >> 12) | (1023 << 52)) ^ 0x5555) - 1.0;
        (unit, tail)
    };
    for case in 0..400_u32 {
        let mut math = context(256);
        let chunk = math.enter_chunk().unwrap();
        let ops = chunk.ops();
        let (a, a_tail) = draw();
        let (b, b_tail) = draw();
        let a = a * 6.0 - 3.0;
        let b = b * 6.0 - 3.0;
        // Exact double-word points a + 2^-60 tail and b + 2^-60 tail.
        let word = |hi: f64, tail: f64| {
            let lo = tail * f64::from_bits((1023 - 60) << 52) * hi.abs().max(1e-3);
            WordInterval::from_binary64(hi, hi)
                .unwrap()
                .add(WordInterval::from_binary64(lo, lo).unwrap(), ops)
                .unwrap()
        };
        let x = word(a, a_tail);
        let y = word(b, b_tail);
        drop(chunk);
        let fx = {
            let l = fixed_word(*x.lower(), &mut math);
            let u = fixed_word(*x.upper(), &mut math);
            FixedInterval::from_bounds(l.lower().clone(), u.upper().clone(), &mut math).unwrap()
        };
        let fy = {
            let l = fixed_word(*y.lower(), &mut math);
            let u = fixed_word(*y.upper(), &mut math);
            FixedInterval::from_bounds(l.lower().clone(), u.upper().clone(), &mut math).unwrap()
        };
        let chunk = math.enter_chunk().unwrap();
        let ops = chunk.ops();
        let sum = x.add(y, ops).unwrap();
        let product = x.mul(y, ops).unwrap();
        let quotient = (b.abs() > 1e-6).then(|| x.div(y, ops).unwrap());
        let root = (a > 0.0).then(|| x.sqrt(ops).unwrap());
        let (sine, cosine) = x.sin_cos(ops).unwrap();
        let angle = WordInterval::atan2(y, x, ops);
        drop(chunk);
        assert!(
            contains(sum, &fx.add(&fy, &mut math).unwrap(), &mut math),
            "{case} sum"
        );
        assert!(
            contains(product, &fx.mul(&fy, &mut math).unwrap(), &mut math),
            "{case} product"
        );
        if let Some(quotient) = quotient {
            assert!(
                contains(quotient, &fx.div(&fy, &mut math).unwrap(), &mut math),
                "{case} quotient"
            );
            assert!(relative_width(quotient) < 1e-27, "{case} quotient width");
        }
        if let Some(root) = root {
            assert!(
                contains(root, &fx.sqrt(&mut math).unwrap(), &mut math),
                "{case} root"
            );
        }
        let (fs, fc) = fx.sin_cos(&mut math).unwrap();
        assert!(contains(sine, &fs, &mut math), "{case} sine");
        assert!(contains(cosine, &fc, &mut math), "{case} cosine");
        if let Ok(angle) = angle {
            let reference = FixedInterval::atan2(&fy, &fx, &mut math).unwrap();
            assert!(contains(angle, &reference, &mut math), "{case} atan2");
            assert!(
                relative_width(angle) < 1e-26,
                "{case} atan2 width {}",
                relative_width(angle)
            );
        }
        assert!(relative_width(product) < 1e-27, "{case} product width");
    }
    // Pi is enclosed tightly by the generated Machin bounds.
    let mut math = context(256);
    let pi = WordInterval::pi().unwrap();
    assert!(contains(
        pi,
        &FixedInterval::pi(&mut math).unwrap(),
        &mut math
    ));
    assert!(relative_width(pi) < 1e-29);
}

#[test]
fn fixed_proposals_approximate_the_exact_midpoint_without_allocation() {
    let mut math = context(224);
    for (numerator, denominator) in [
        (1_i64, 3_i64),
        (-7, 9),
        (355, 113),
        (1, 1 << 40),
        (-123_456_789, 1000),
    ] {
        let value = FixedInterval::from_ratio(
            &BigInt::from_i128(i128::from(numerator)),
            &BigInt::from_i128(i128::from(denominator)),
            &mut math,
        )
        .unwrap();
        let exact = numerator as f64 / denominator as f64;
        let approximate = value.approximate_binary64().unwrap();
        assert!(
            (approximate - exact).abs() <= exact.abs() * 1e-15,
            "{numerator}/{denominator}"
        );
    }
    let zero = FixedInterval::from_i64(0, &mut math).unwrap();
    assert_eq!(zero.approximate_binary64(), Some(0.0));
}
