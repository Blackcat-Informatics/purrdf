// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use super::KARATSUBA_THRESHOLD;

#[cfg(test)]
mod tests {
    use super::super::{BigInt, LimbScratch, LimbScratchError};
    use super::KARATSUBA_THRESHOLD;
    use purrdf_testkit::rng::splitmix64_next;

    fn random(state: &mut u64, limbs: usize, negative: bool) -> BigInt {
        let mut value = BigInt::zero();
        for index in 0..limbs {
            let word = splitmix64_next(state) | u64::from(index == 0);
            value = value.shl(64).add(&BigInt::from_u64(word));
        }
        if negative { value.negated() } else { value }
    }

    #[test]
    fn karatsuba_agrees_with_schoolbook_across_the_threshold() {
        let mut state = 0x4B41_5241_u64;
        for (la, lb) in [
            (KARATSUBA_THRESHOLD, KARATSUBA_THRESHOLD),
            (KARATSUBA_THRESHOLD + 1, KARATSUBA_THRESHOLD * 3),
            (200, 200),
            (511, 64),
            (64, 1000),
            (97, 98),
        ] {
            let a = random(&mut state, la, false);
            let b = random(&mut state, lb, true);
            assert_eq!(a.mul_fast(&b), a.mul(&b), "{la} × {lb} limbs");
        }
    }

    #[test]
    fn div_rem_reconstructs_the_dividend() {
        let mut state = 0xD1F0_u64;
        for la in [1_usize, 2, 3, 7, 30, 120] {
            for lb in [1_usize, 2, 3, 9, 40] {
                let a = random(&mut state, la, la % 2 == 0);
                let b = random(&mut state, lb, lb % 3 == 0);
                if b.is_zero() {
                    continue;
                }
                let (q, r) = a.div_rem(&b).expect("nonzero divisor");
                assert_eq!(&(&q * &b) + &r, a, "{la}/{lb}");
                assert!(r.abs() < b.abs(), "|r| < |b|");
                assert!(r.is_zero() || r.is_negative() == a.is_negative());
            }
        }
        assert!(BigInt::one().div_rem(&BigInt::zero()).is_none());
    }

    #[test]
    fn recursive_division_admits_transform_products_for_dense_quotients() {
        let bits = 65_536 * 64;
        let power = BigInt::one().shl(bits);
        let divisor = power.sub(&BigInt::one());
        // (B²-B-1)/(B-1) = B-1, remainder B-2. No product constructs
        // either the input or its independent expected quotient/remainder.
        let dividend = BigInt::one().shl(bits * 2).sub(&power).sub(&BigInt::one());
        let expected_remainder = divisor.sub(&BigInt::one());
        let layout = crate::exact::cost::integer_division_layout(
            dividend.limb_len() as u64,
            divisor.limb_len() as u64,
        )
        .unwrap();
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let (quotient, remainder) = dividend.div_rem(&divisor).unwrap();
        let measurement = window.close();
        assert_eq!(quotient, divisor);
        assert_eq!(remainder, expected_remainder);
        assert!(
            u64::try_from(measurement.peak_working_bytes).unwrap()
                <= layout.required_bytes() as u64,
            "{measurement:?}, {layout:?}"
        );
    }

    #[test]
    fn block_division_reconstructs_threshold_product_and_remainder() {
        let mut state = 0x0042_4C4F_434B_u64;
        for divisor_limbs in [2_047, 2_048, 2_049] {
            let divisor = random(&mut state, divisor_limbs, false);
            for quotient_limbs in [1, 2_047, 2_048, 4_097] {
                let expected_quotient = random(&mut state, quotient_limbs, false);
                let dividend = divisor.mul_fast(&expected_quotient).add(&BigInt::one());
                let layout = crate::exact::cost::integer_division_layout(
                    dividend.limb_len() as u64,
                    divisor.limb_len() as u64,
                )
                .expect("addressable division layout");
                let cost =
                    crate::exact::cost::div(dividend.limb_len() as u64, divisor.limb_len() as u64);
                for (negative_dividend, negative_divisor) in
                    [(false, false), (true, false), (false, true), (true, true)]
                {
                    let signed_dividend = if negative_dividend {
                        dividend.negated()
                    } else {
                        dividend.clone()
                    };
                    let signed_divisor = if negative_divisor {
                        divisor.negated()
                    } else {
                        divisor.clone()
                    };
                    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
                    let (quotient, remainder) = signed_dividend
                        .div_rem(&signed_divisor)
                        .expect("nonzero divisor");
                    let measurement = window.close();
                    assert!(measurement.allocations > 0, "live allocation counter");
                    assert!(
                        usize::try_from(measurement.peak_working_bytes).unwrap()
                            <= layout.required_bytes(),
                        "{divisor_limbs}/{quotient_limbs}: {measurement:?} exceeds {layout:?}"
                    );
                    assert!(
                        u64::try_from(measurement.peak_working_bytes).unwrap() <= cost.bytes(),
                        "division governor peak: {measurement:?} exceeds {cost:?}"
                    );
                    let expected = if negative_dividend != negative_divisor {
                        expected_quotient.negated()
                    } else {
                        expected_quotient.clone()
                    };
                    assert_eq!(quotient, expected);
                    assert_eq!(
                        remainder,
                        BigInt::from_i64(if negative_dividend { -1 } else { 1 })
                    );
                    assert_eq!(&(&quotient * &signed_divisor) + &remainder, signed_dividend);
                }
            }
        }
    }

    #[test]
    fn block_division_handles_normalization_shifts_and_largest_remainders() {
        let mut state = 0x0042_4C4F_434B_5348_u64;
        for leading_zeros in [0_u32, 1, 31, 63] {
            let divisor = BigInt::one()
                .shl(64 * 2_047 + 63 - leading_zeros)
                .add(&random(&mut state, 2_047, false));
            let expected_quotient = random(&mut state, 2_048, false);
            let expected_remainder = divisor.sub(&BigInt::one());
            let dividend = divisor
                .mul_fast(&expected_quotient)
                .add(&expected_remainder);
            let (quotient, remainder) = dividend.div_rem(&divisor).expect("nonzero divisor");
            assert_eq!(quotient, expected_quotient, "shift {leading_zeros}");
            assert_eq!(remainder, expected_remainder, "shift {leading_zeros}");

            let capacity = dividend.limb_len() + 256;
            let scratch = LimbScratch::new(128, capacity).expect("admitted recursive destinations");
            let window = purrdf_alloc_probe::CurrentThreadWindow::open();
            let actual = dividend
                .div_rem_in(&divisor, &scratch)
                .expect("recursive division fits the admitted arena")
                .expect("nonzero divisor");
            assert_eq!(window.close().allocations, 0, "no fallback allocations");
            assert_eq!(actual, (expected_quotient, expected_remainder));
            assert_eq!(scratch.available(), 126, "only the two results remain held");
            let retained = actual.0.clone();
            drop(actual);
            assert_eq!(
                scratch.available(),
                127,
                "the clone retains its destination"
            );
            drop(retained);
            assert_eq!(
                scratch.available(),
                128,
                "last-owner release returns the slot"
            );

            let insufficient = LimbScratch::new(3, capacity).expect("ordinary division arena");
            let window = purrdf_alloc_probe::CurrentThreadWindow::open();
            let refusal = dividend.div_rem_in(&divisor, &insufficient);
            assert_eq!(
                window.close().allocations,
                0,
                "refusal has no heap fallback"
            );
            assert!(matches!(
                refusal,
                Err(LimbScratchError::Exhausted { destinations: 3 })
            ));
            assert_eq!(insufficient.available(), 3, "refusal releases partial work");
            assert_eq!(
                BigInt::from_u64(65).div_rem_in(&BigInt::from_u64(7), &insufficient),
                Ok(Some((BigInt::from_u64(9), BigInt::from_u64(2))))
            );
        }
        for limbs in [34_943, 34_944, 34_945] {
            let width = 64 * limbs;
            let power = BigInt::one().shl(width);
            let divisor = power.sub(&BigInt::one());
            let dividend = BigInt::one().shl(2 * width).sub(&BigInt::from_u64(2));
            let layout = crate::exact::cost::integer_division_layout(
                dividend.limb_len() as u64,
                divisor.limb_len() as u64,
            )
            .expect("addressable deep normalization layout");
            let cost =
                crate::exact::cost::div(dividend.limb_len() as u64, divisor.limb_len() as u64);
            let window = purrdf_alloc_probe::CurrentThreadWindow::open();
            let actual = dividend.div_rem(&divisor).expect("nonzero divisor");
            let measurement = window.close();
            assert!(
                measurement.allocations > 0,
                "live deep normalization counter"
            );
            assert!(
                usize::try_from(measurement.peak_working_bytes).unwrap() <= layout.required_bytes()
            );
            assert!(u64::try_from(measurement.peak_working_bytes).unwrap() <= cost.bytes());
            assert_eq!(actual, (power, divisor.sub(&BigInt::one())));
        }
    }

    #[test]
    fn decimal_render_reuses_one_quotient_destination() {
        let digits = "12345678901234567890".repeat(20);
        let value = BigInt::from_digits(&digits).expect("valid decimal digits");
        let layout = value.decimal_render_layout(0).expect("addressable layout");
        let capacity = value.limb_len().max(layout.group_capacity());
        let scratch = LimbScratch::new(2, capacity).expect("two admitted destinations");
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let prepared = value
            .prepare_decimal_digits_in(&scratch)
            .expect("one quotient and one group destination");
        assert_eq!(window.close().allocations, 0);
        assert_eq!(prepared.to_string(), digits);
        assert_eq!(
            scratch.available(),
            1,
            "only the group destination remains held"
        );
        drop(prepared);
        assert_eq!(
            scratch.available(),
            2,
            "the original group owner is returned"
        );

        let one = LimbScratch::new(1, capacity).expect("one admitted destination");
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        assert_eq!(
            value.prepare_decimal_digits_in(&one).unwrap_err(),
            LimbScratchError::Exhausted { destinations: 1 }
        );
        assert_eq!(window.close().allocations, 0);
        assert_eq!(one.available(), 1, "refusal returns the group destination");

        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let prepared = value.prepare_decimal_digits().expect("native rendering");
        assert_eq!(window.close().allocations, 2, "one group and one quotient");
        assert_eq!(prepared.to_string(), digits);

        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let lexical = value.to_string();
        let rendered = window.close();
        assert_eq!(lexical, digits);
        assert!(
            rendered.allocations <= 9,
            "canonical output allocations: {}",
            rendered.allocations
        );
    }

    #[test]
    fn decimal_render_tree_preserves_zero_padding_peak_and_scratch_release() {
        for groups in [255_u32, 256, 257, 511, 512, 513, 2106, 8191, 8192, 8193] {
            let digits = groups * 19;
            let power = BigInt::pow10(digits);
            for (value, expected) in [
                (power.sub(&BigInt::one()), "9".repeat(digits as usize)),
                (
                    power.add(&BigInt::one()).negated(),
                    format!("-1{}1", "0".repeat(digits as usize - 1)),
                ),
            ] {
                let layout = value.decimal_render_layout(0).unwrap();
                let window = purrdf_alloc_probe::CurrentThreadWindow::open();
                let prepared = value.prepare_decimal_digits().unwrap();
                let measured = window.close();
                assert_eq!(prepared.to_string(), expected, "{groups} groups");
                assert!(
                    usize::try_from(measured.peak_working_bytes).unwrap()
                        <= layout.temporary_bytes(),
                    "{groups}: {measured:?}, {layout:?}"
                );
                assert_eq!(
                    measured.retained_bytes,
                    i64::try_from(prepared.heap_bytes()).unwrap()
                );
                let cost = crate::exact::cost::render(value.limb_len() as u64, 0);
                assert!(u64::try_from(measured.peak_working_bytes).unwrap() <= cost.bytes());
            }
        }

        let value = BigInt::pow10(160_000).add(&BigInt::one());
        let layout = value.decimal_render_layout(0).unwrap();
        let capacity = value.limb_len() * 2 + 128;
        let scratch = LimbScratch::new(128, capacity).unwrap();
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let prepared = value.prepare_decimal_digits_in(&scratch).unwrap();
        assert_eq!(scratch.available(), 127);
        drop(prepared);
        assert_eq!(scratch.available(), 128);
        assert_eq!(window.close().allocations, 0);

        let one = LimbScratch::new(1, capacity).unwrap();
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        assert!(matches!(
            value.prepare_decimal_digits_in(&one),
            Err(LimbScratchError::Exhausted { .. })
        ));
        assert_eq!(one.available(), 1);
        let neighbor = BigInt::one().shl_in(400, &one).unwrap();
        drop(neighbor);
        assert_eq!(one.available(), 1);
        assert_eq!(window.close().allocations, 0);

        let narrow = LimbScratch::new(128, layout.group_capacity() - 1).unwrap();
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        assert!(matches!(
            value.prepare_decimal_digits_in(&narrow),
            Err(LimbScratchError::Capacity { .. })
        ));
        assert_eq!(narrow.available(), 128);
        assert_eq!(window.close().allocations, 0);
    }

    #[test]
    fn pow_gcd_and_decimal_shapes() {
        assert_eq!(BigInt::from_i128(-3).pow(3), BigInt::from_i128(-27));
        assert_eq!(BigInt::zero().pow(0), BigInt::one());
        assert_eq!(
            BigInt::from_i128(2).pow(127).to_string(),
            (1_u128 << 127).to_string()
        );
        assert_eq!(BigInt::pow10(40).decimal_digits(), 41);
        assert_eq!(BigInt::pow10(40).trailing_decimal_zeros(), 40);
        assert_eq!(BigInt::zero().decimal_digits(), 1);
        let a = BigInt::from_i128(2)
            .pow(200)
            .mul_fast(&BigInt::from_i128(3).pow(50));
        let b = BigInt::from_i128(2)
            .pow(90)
            .mul_fast(&BigInt::from_i128(3).pow(120));
        let g = BigInt::from_i128(2)
            .pow(90)
            .mul_fast(&BigInt::from_i128(3).pow(50));
        assert_eq!(a.gcd(&b.negated()), g);
        let (q, r) = BigInt::from_i128(-123_456_789_012).div_rem_pow10(4);
        assert_eq!(
            (q, r),
            (BigInt::from_i128(-12_345_678), BigInt::from_i128(-9_012))
        );
    }

    #[test]
    fn gcd_reduces_a_wide_initial_gap_without_changing_the_exact_factor() {
        let mut state = 0x4743_4457_u64;
        let common = random(&mut state, 17, false);
        let wide = common.shl(64 * 500);
        let narrow = common.mul_fast(&BigInt::from_u64(3));
        assert_eq!(wide.gcd(&narrow), common);
        assert_eq!(narrow.negated().gcd(&wide), common);
        let scratch = LimbScratch::new(8, wide.limb_len() + 1).unwrap();
        assert_eq!(wide.gcd_in(&narrow, &scratch).unwrap(), common);
    }
}
