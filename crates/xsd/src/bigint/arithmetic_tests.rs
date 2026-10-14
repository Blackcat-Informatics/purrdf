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
