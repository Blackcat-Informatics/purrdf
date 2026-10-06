// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Public arithmetic replay on native and wasm testkit runners. The elementary
//! mantissas below are independently known values rounded to twelve places;
//! endpoint agreement certifies rounding rather than accepting a tolerance.

use purrdf_hash::{
    Backend,
    fnv::{BASIS, fold},
};
use purrdf_testkit::harness::report_digest;
use purrdf_xsd::{
    BigInt,
    ieee::ratio::{Rounding, to_binary64_in},
    integer::{ExactArithmeticCost, Int, LimbScratch, LimbScratchError},
    math::{
        CertifiedInterval, CoordinateMath, FixedInterval, FloatEnclosure, FloatInterval,
        FloatProductBackend, IntervalContext, MathError, MathLimits, with_taylor_workspace,
        with_taylor_workspace_observed,
    },
    numeric::{
        CANONICAL_IEEE_MAX_BYTES, canonical_double, canonical_double_into, canonical_float_into,
    },
    rational::{Rat, RationalComparisonError},
};

#[global_allocator]
static GLOBAL: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;

const ELEMENTARY: [i128; 5] = [
    3_141_592_653_590, // pi
    644_217_687_238,   // sin(7/10)
    764_842_187_284,   // cos(7/10)
    1_414_213_562_373, // sqrt(2)
    785_398_163_397,   // atan2(1,1)
];
const ELEMENTARY_DIGEST: u64 = 0x8268_aff8_be76_30be;
// Interval products with known operand signs take the round-to-nearest
// product and step one ulp outward (an exact zero stays exact), which every
// backend computes identically; mixed-sign products keep the packed path.
const ENDPOINT_DIGEST: u64 = 0xf150_8dda_c398_273b;

fn elementary<I: CertifiedInterval>(context: &mut IntervalContext<'_>) -> Vec<i128> {
    let one = I::from_i64(1, context).unwrap();
    let angle = I::from_ratio(&BigInt::from_i128(7), &BigInt::from_i128(10), context).unwrap();
    let (sine, cosine) = angle.sin_cos(context).unwrap();
    let values = [
        I::pi(context).unwrap(),
        sine,
        cosine,
        I::from_i64(2, context).unwrap().sqrt(context).unwrap(),
        I::atan2(&one, &one, context).unwrap(),
    ];
    values
        .iter()
        .zip(ELEMENTARY)
        .map(|(value, expected)| {
            let (lower, upper) = value.round_decimal(12, context).unwrap();
            assert_eq!(lower, upper, "the whole enclosure must round identically");
            assert_eq!(lower.to_i128(), Some(expected));
            expected
        })
        .collect()
}

fn certified_elementary_replay() {
    let digest = report_digest("certified_elementary_replay", ELEMENTARY.len() + 2, || {
        let mut math = CoordinateMath::new(MathLimits::DEFAULT).unwrap();
        let reference = elementary::<FixedInterval>(&mut IntervalContext::fixed(&mut math));
        for backend in FloatProductBackend::all_available() {
            let mut worker = CoordinateMath::new(MathLimits::DEFAULT).unwrap();
            worker.set_binary64_backend(backend).unwrap();
            worker.prepare_binary64().unwrap();
            assert_eq!(
                elementary::<FloatEnclosure>(&mut IntervalContext::binary64(&mut worker).unwrap()),
                reference,
                "{}",
                backend.name(),
            );
        }
        let mut digest = BASIS;
        for value in reference {
            digest = fold(digest, &value.to_be_bytes());
        }
        for (value, expected) in [
            (
                FixedInterval::from_i64(2, &mut math)
                    .unwrap()
                    .log(&mut math)
                    .unwrap(),
                693_147_180_560,
            ),
            (
                FixedInterval::from_i64(1, &mut math)
                    .unwrap()
                    .exp(&mut math)
                    .unwrap(),
                2_718_281_828_459,
            ),
        ] {
            let (lower, upper) = value.round_decimal(12, &mut math).unwrap();
            assert_eq!(lower, upper);
            assert_eq!(lower.to_i128(), Some(expected));
            digest = fold(digest, &expected.to_be_bytes());
        }
        digest
    });
    assert_eq!(digest, ELEMENTARY_DIGEST);
}

fn directed_endpoint_product_replay() {
    let digest = report_digest("directed_endpoint_product_replay", 8, || {
        purrdf_hash::dispatch::assert_required_available::<FloatProductBackend>(
            "numeric",
            FloatProductBackend::is_available,
        );
        let paths: Vec<_> = FloatProductBackend::all_available().collect();
        #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
        assert!(paths.contains(&FloatProductBackend::Simd128));
        #[cfg(all(target_arch = "wasm32", not(target_feature = "simd128")))]
        assert!(!paths.contains(&FloatProductBackend::Simd128));
        let minimum = f64::from_bits(1);
        let huge = f64::from_bits((1023 + 449) << 52);
        let tiny = f64::from_bits((1023 - 449) << 52);
        let rows = [
            ((0.7, f64::from_bits(0.7_f64.to_bits() + 1)), (-1.2, -1.2)),
            ((-0.0, 0.0), (-1.0, 1.0)),
            ((minimum, minimum), (0.5, 0.5)),
            ((huge, huge), (tiny, tiny)),
            ((-tiny, tiny), (-huge, huge)),
            ((1.0, 1.0), (1.0, 1.0)),
            ((-2.0, 3.0), (-5.0, 7.0)),
            ((0.1, 0.1), (0.2, 0.2)),
        ];
        let mut digest = BASIS;
        for (left, right) in rows {
            let left = FloatInterval::from_bounds(left.0, left.1).unwrap();
            let right = FloatInterval::from_bounds(right.0, right.1).unwrap();
            let mut math = CoordinateMath::new(MathLimits::DEFAULT).unwrap();
            math.set_binary64_backend(FloatProductBackend::Portable)
                .unwrap();
            let expected = left.mul(right, &math.enter_chunk().unwrap()).unwrap();
            for backend in paths.iter().copied() {
                math.set_binary64_backend(backend).unwrap();
                let actual = left.mul(right, &math.enter_chunk().unwrap()).unwrap();
                assert_eq!(actual.lower().to_bits(), expected.lower().to_bits());
                assert_eq!(actual.upper().to_bits(), expected.upper().to_bits());
            }
            digest = fold(digest, &expected.lower().to_bits().to_be_bytes());
            digest = fold(digest, &expected.upper().to_bits().to_be_bytes());
        }
        digest
    });
    assert_eq!(digest, ENDPOINT_DIGEST);
}

fn numerical_refusals_are_complete() {
    // Reconstruct a multi-limb product quotient and bracket its exact integer
    // square root using the admitted original arithmetic on every target.
    let scratch = LimbScratch::new(16, 16).unwrap();
    let first = Int::one().shl(260).add(&Int::from_i64(3));
    let second = Int::one().shl(140).add(&Int::from_i64(7));
    let divisor = Int::one().shl(130).add(&Int::one());
    let (quotient, remainder) = first
        .mul_div_rem_in(&second, &divisor, &scratch)
        .unwrap()
        .unwrap();
    let product = first.mul(&second);
    assert_eq!(quotient.mul(&divisor).add(&remainder), product);
    assert!(!remainder.is_negative() && remainder < divisor);
    let root = first
        .sqrt_product_floor_in(&second, &scratch)
        .unwrap()
        .unwrap();
    assert!(root.mul(&root) <= product);
    let next = root.add(&Int::one());
    assert!(next.mul(&next) > product);
    drop((quotient, remainder, root));
    let held: Vec<_> = (0..16).map(|_| first.copy_in(&scratch).unwrap()).collect();
    assert_eq!(
        first.shl_in(1, &scratch),
        Err(LimbScratchError::Exhausted { destinations: 16 })
    );
    drop(held);
    assert_eq!(scratch.available(), 16);
    let mut math = CoordinateMath::new(MathLimits::DEFAULT).unwrap();
    assert!(matches!(
        FixedInterval::from_ratio(&BigInt::from_i128(1), &BigInt::zero(), &mut math),
        Err(MathError::Domain(_)),
    ));
    let mut bounded = CoordinateMath::new(MathLimits {
        max_work: 1,
        ..MathLimits::DEFAULT
    })
    .unwrap();
    assert_eq!(
        FixedInterval::pi(&mut bounded),
        Err(MathError::WorkExhausted)
    );
    assert!(
        CoordinateMath::new(MathLimits {
            max_workspace_bytes: 0,
            ..MathLimits::DEFAULT
        })
        .is_err()
    );
}

fn bounded_rational_ownership_and_rounding_replay() {
    let scratch = LimbScratch::new(32, 80).unwrap();
    let denominator = Int::one().shl(2000);
    let halfway = denominator.add(&Int::one().shl(1947));
    for (numerator, expected) in [
        (halfway.sub(&Int::one()), 1.0_f64.to_bits()),
        (halfway.clone(), 1.0_f64.to_bits()),
        (halfway.add(&Int::one()), 1.0_f64.to_bits() + 1),
    ] {
        let value = to_binary64_in(&numerator, &denominator, Rounding::NearestEven, &scratch)
            .unwrap()
            .unwrap();
        assert_eq!(value.to_bits(), expected);
    }
    let decimal = Int::from_i64(3).mul(&Int::pow10(400));
    let normalized = Rat::from_decimal_in(&decimal, 400, &scratch).unwrap();
    assert_eq!(normalized, Rat::from_i64(3));
    let source = Rat::new(Int::one().shl(448).add(&Int::one()), Int::one()).unwrap();
    let copied = Rat::new(
        source.numerator().copy_in(&scratch).unwrap(),
        source.denominator().clone(),
    )
    .unwrap()
    .into_shared();
    assert_eq!(scratch.available(), scratch.destination_capacity());
    assert_eq!(
        source.compare_in(&copied, &scratch).unwrap(),
        std::cmp::Ordering::Equal
    );
    drop(scratch);
    assert_eq!(source, copied);
}

fn reusable_taylor_pool_admission_and_allocation_replay() {
    let scratch = LimbScratch::new(64, 32).unwrap();
    let mut math = CoordinateMath::new(MathLimits::DEFAULT).unwrap();
    math.set_limb_scratch(scratch).unwrap();
    let run = |math: &mut CoordinateMath| {
        with_taylor_workspace(8, 4, math, |workspace, math| {
            let one = FixedInterval::from_i64(1, math)?;
            let state = workspace.constant(one.clone(), math)?;
            let mark = workspace.checkpoint();
            for _ in 0..8 {
                let next = workspace.integral(state, one.clone(), math)?;
                workspace.assign(state, next, math)?;
                workspace.rewind(mark)?;
            }
            let sixth = workspace.coefficient(state, 6)?;
            let expected =
                FixedInterval::from_ratio(&BigInt::from_i128(1), &BigInt::from_i128(720), math)?;
            assert!(sixth.lower() <= expected.lower() && sixth.upper() >= expected.upper());
            Ok(())
        })
    };
    run(&mut math).unwrap();
    let retained = math.workspace_reserved();
    let allocations = purrdf_alloc_probe::CurrentThreadWindow::open();
    for _ in 0..16 {
        run(&mut math).unwrap();
    }
    let receipt = allocations.close();
    assert_eq!(receipt.allocations, 0);
    assert_eq!(receipt.requested_bytes, 0);
    assert_eq!(math.workspace_reserved(), retained);
    let mut refused = CoordinateMath::new(MathLimits::DEFAULT).unwrap();
    let allocations = purrdf_alloc_probe::CurrentThreadWindow::open();
    let result = with_taylor_workspace_observed(
        8,
        4,
        &mut refused,
        &mut (),
        |_, ()| Err(MathError::Cancelled),
        |_, _, ()| Ok(()),
    );
    let receipt = allocations.close();
    assert_eq!(result, Err(MathError::Cancelled));
    assert_eq!(receipt.allocations, 0);
    assert!(refused.taylor_scratch().is_none());
    assert_eq!(refused.workspace_reserved(), 0);
}

fn bounded_ieee_canonical_allocation_replay() {
    let mut output = String::with_capacity(CANONICAL_IEEE_MAX_BYTES);
    let allocations = purrdf_alloc_probe::CurrentThreadWindow::open();
    for value in [
        0.0,
        -0.0,
        1.0,
        0.1,
        f64::from_bits(1),
        -f64::MIN_POSITIVE,
        f64::MAX,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
    ] {
        output.clear();
        canonical_double_into(value, &mut output).unwrap();
    }
    for value in [0.1, f32::from_bits(1), -f32::MIN_POSITIVE, f32::MAX] {
        output.clear();
        canonical_float_into(value, &mut output).unwrap();
    }
    let receipt = allocations.close();
    assert_eq!(receipt.allocations, 0);
    assert_eq!(receipt.requested_bytes, 0);
    let allocations = purrdf_alloc_probe::CurrentThreadWindow::open();
    let owned = canonical_double(-f64::MIN_POSITIVE);
    let receipt = allocations.close();
    assert_eq!(owned, "-2.2250738585072014E-308");
    assert_eq!(receipt.allocations, 1);
    assert_eq!(receipt.requested_bytes, CANONICAL_IEEE_MAX_BYTES as u64);
    assert_eq!(receipt.retained_bytes, CANONICAL_IEEE_MAX_BYTES as i64);
    assert_eq!(receipt.peak_working_bytes, CANONICAL_IEEE_MAX_BYTES as i64);
}

fn finite_decimal_scale_admission_and_original_limb_replay() {
    let no_destinations = LimbScratch::new(0, 0).unwrap();
    for integer in [Rat::zero(), Rat::from_i64(-7)] {
        let cost =
            ExactArithmeticCost::finite_decimal_scale(integer.denominator().bit_len()).unwrap();
        let mut math = CoordinateMath::new(MathLimits {
            max_work: 16,
            ..MathLimits::DEFAULT
        })
        .unwrap();
        math.admit_exact_cost(cost).unwrap();
        assert_eq!(
            integer.finite_decimal_scale_in(&no_destinations).unwrap(),
            Some(0)
        );
    }
    let source = Rat::new(Int::one(), Int::one().mul_pow5(4096).shl(2048)).unwrap();
    let cost = ExactArithmeticCost::finite_decimal_scale(source.denominator().bit_len()).unwrap();
    let scratch = LimbScratch::new(8, 192).unwrap();
    let mut math = CoordinateMath::new(MathLimits {
        max_work: cost.work_items,
        ..MathLimits::DEFAULT
    })
    .unwrap();
    let allocations = purrdf_alloc_probe::CurrentThreadWindow::open();
    math.admit_exact_cost(cost).unwrap();
    assert_eq!(
        source.finite_decimal_scale_in(&scratch).unwrap(),
        Some(4096)
    );
    let receipt = allocations.close();
    assert_eq!(receipt.allocations, 0);
    assert_eq!(receipt.requested_bytes, 0);
    assert_eq!(scratch.available(), scratch.destination_capacity());

    let long = Rat::new(Int::one(), Int::one().shl(32_768).add(&Int::one())).unwrap();
    let cost = ExactArithmeticCost::finite_decimal_scale(long.denominator().bit_len()).unwrap();
    for (limits, expected) in [
        (
            MathLimits {
                max_work: 64,
                ..MathLimits::DEFAULT
            },
            MathError::WorkExhausted,
        ),
        (
            MathLimits {
                max_work: cost.work_items,
                max_workspace_bytes: 16_384,
                ..MathLimits::DEFAULT
            },
            MathError::WorkspaceExhausted,
        ),
    ] {
        let mut refused = CoordinateMath::new(limits).unwrap();
        let allocations = purrdf_alloc_probe::CurrentThreadWindow::open();
        let result = (|| {
            refused.admit_exact_cost(cost)?;
            let scratch = LimbScratch::new(8, 1024).map_err(MathError::LimbScratch)?;
            long.finite_decimal_scale_in(&scratch)
                .map_err(MathError::LimbScratch)
        })();
        let receipt = allocations.close();
        assert_eq!(result, Err(expected));
        assert_eq!(receipt.allocations, 0);
        assert_eq!(receipt.requested_bytes, 0);
        assert_eq!(refused.work_used(), 0);
    }
    assert!(ExactArithmeticCost::finite_decimal_scale(u64::MAX).is_none());
}

fn admitted_comparison_branches_and_original_limb_replay() {
    let denominator = Int::one().shl(448);
    let left = Rat::new(Int::from_i64(7), denominator.clone()).unwrap();
    let right = Rat::new(Int::from_i64(9), denominator.clone()).unwrap();
    let unequal = Rat::new(Int::from_i64(9), denominator.add(&Int::one())).unwrap();
    let cases = [
        (&left, &right, core::cmp::Ordering::Less, 2),
        (&right, &left, core::cmp::Ordering::Greater, 2),
        (&left, &unequal, core::cmp::Ordering::Less, 2),
        (&left, &left, core::cmp::Ordering::Equal, 2),
    ];
    let scratch = LimbScratch::new(8, 32).unwrap();
    let negative = right.neg();
    let mut math = CoordinateMath::new(MathLimits::DEFAULT).unwrap();
    let allocations = purrdf_alloc_probe::CurrentThreadWindow::open();
    for (a, b, expected, stages) in cases {
        let mut calls = 0;
        let result = a.compare_admitted_in(b, &scratch, |cost, evaluate| {
            calls += 1;
            math.admit_exact_cost(cost)?;
            evaluate().map_err(MathError::LimbScratch)
        });
        assert_eq!(result, Ok(expected));
        assert_eq!(calls, stages);
    }
    let mut sign_calls = 0;
    assert_eq!(
        left.compare_admitted_in(&negative, &scratch, |cost, evaluate| {
            sign_calls += 1;
            math.admit_exact_cost(cost)?;
            evaluate().map_err(MathError::LimbScratch)
        }),
        Ok(core::cmp::Ordering::Greater)
    );
    assert_eq!(sign_calls, 1);
    let receipt = allocations.close();
    assert_eq!(receipt.allocations, 0);
    assert_eq!(receipt.requested_bytes, 0);
    assert_eq!(scratch.available(), scratch.destination_capacity());

    // A long source is owned before the measured window. Its first admitted
    // scan refuses before any original source scan or allocating product body.
    let long = Rat::new(Int::one(), Int::one().shl(32_768).add(&Int::one())).unwrap();
    let mut refused = CoordinateMath::new(MathLimits {
        max_work: 64,
        ..MathLimits::DEFAULT
    })
    .unwrap();
    let mut evaluated = false;
    let allocations = purrdf_alloc_probe::CurrentThreadWindow::open();
    let result = long.compare_admitted(&left, |cost, evaluate| {
        refused.admit_exact_cost(cost)?;
        evaluated = true;
        evaluate().map_err(MathError::LimbScratch)
    });
    let receipt = allocations.close();
    assert_eq!(
        result,
        Err(RationalComparisonError::Admission(MathError::WorkExhausted))
    );
    assert!(!evaluated);
    assert_eq!(refused.work_used(), 0);
    assert_eq!(receipt.allocations, 0);
    assert_eq!(receipt.requested_bytes, 0);

    // The cross-product phase remains separately admitted after the source
    // scan; a refusal there cannot run the heap-allocating original product.
    let mut evaluated = 0;
    let allocations = purrdf_alloc_probe::CurrentThreadWindow::open();
    let result = left.compare_admitted(&unequal, |_, evaluate| {
        if evaluated == 1 {
            return Err(MathError::WorkspaceExhausted);
        }
        evaluated += 1;
        evaluate().map_err(MathError::LimbScratch)
    });
    let receipt = allocations.close();
    assert_eq!(
        result,
        Err(RationalComparisonError::Admission(
            MathError::WorkspaceExhausted
        ))
    );
    assert_eq!(evaluated, 1);
    assert_eq!(receipt.allocations, 0);
    assert_eq!(receipt.requested_bytes, 0);
}

fn integer_difference_interval_admission_and_original_limb_replay() {
    let left = Rat::new(
        Int::one().shl(448).add(&Int::from_i64(7)),
        Int::one().shl(449).add(&Int::from_i64(3)),
    )
    .unwrap();
    let right = Rat::new(
        Int::one().shl(440).neg(),
        Int::one().shl(447).add(&Int::one()),
    )
    .unwrap();
    let shape = |value: &Rat| (value.numerator().bit_len(), value.denominator().bit_len());
    let cost =
        ExactArithmeticCost::rational_difference_integer_interval(shape(&left), shape(&right), 8)
            .unwrap();
    let scratch = LimbScratch::new(8, 32).unwrap();
    let mut math = CoordinateMath::new(MathLimits {
        max_work: cost.work_items,
        ..MathLimits::DEFAULT
    })
    .unwrap();
    let allocations = purrdf_alloc_probe::CurrentThreadWindow::open();
    math.admit_exact_cost(cost).unwrap();
    assert!(
        left.difference_in_integer_interval_in(&right, -180, 180, &scratch)
            .unwrap()
    );
    let receipt = allocations.close();
    assert_eq!(receipt.allocations, 0);
    assert_eq!(receipt.requested_bytes, 0);
    assert_eq!(scratch.available(), scratch.destination_capacity());

    let long = Rat::new(Int::one(), Int::one().shl(32_768).add(&Int::one())).unwrap();
    let zero = Rat::zero();
    let cost =
        ExactArithmeticCost::rational_difference_integer_interval(shape(&long), shape(&zero), 8)
            .unwrap();
    for (limits, expected) in [
        (
            MathLimits {
                max_work: cost.work_items - 1,
                ..MathLimits::DEFAULT
            },
            MathError::WorkExhausted,
        ),
        (
            MathLimits {
                max_work: cost.work_items,
                max_workspace_bytes: 16_384,
                ..MathLimits::DEFAULT
            },
            MathError::WorkspaceExhausted,
        ),
    ] {
        let mut refused = CoordinateMath::new(limits).unwrap();
        let allocations = purrdf_alloc_probe::CurrentThreadWindow::open();
        let result = (|| {
            refused.admit_exact_cost(cost)?;
            let scratch = LimbScratch::new(8, 1024).map_err(MathError::LimbScratch)?;
            long.difference_in_integer_interval_in(&zero, -180, 180, &scratch)
                .map_err(MathError::LimbScratch)
        })();
        let receipt = allocations.close();
        assert_eq!(result, Err(expected));
        assert_eq!(receipt.allocations, 0);
        assert_eq!(receipt.requested_bytes, 0);
        assert_eq!(refused.work_used(), 0);
    }
    assert!(
        ExactArithmeticCost::rational_difference_integer_interval((u64::MAX, 1), (1, 1), 8)
            .is_none()
    );
}

purrdf_testkit::harness_main!(
    certified_elementary_replay,
    directed_endpoint_product_replay,
    numerical_refusals_are_complete,
    bounded_rational_ownership_and_rounding_replay,
    reusable_taylor_pool_admission_and_allocation_replay,
    bounded_ieee_canonical_allocation_replay,
    finite_decimal_scale_admission_and_original_limb_replay,
    integer_difference_interval_admission_and_original_limb_replay,
    admitted_comparison_branches_and_original_limb_replay,
);
