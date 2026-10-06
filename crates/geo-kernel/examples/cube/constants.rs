// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Independently derive the cube-grid constants from bounded Machin series and
//! factorial equations. This host generator reads no external data or code.

use super::{Int, Rat};

fn atan_interval(inverse: i64, terms: u32) -> (Rat, Rat) {
    let inverse = Int::from_i64(inverse);
    let square = inverse.mul(&inverse);
    let mut power = inverse;
    let mut sum = Rat::zero();
    for term in 0..terms {
        let numerator = Int::from_i64(if term & 1 == 0 { 1 } else { -1 });
        let denominator = power.mul(&Int::from_i64(i64::from(2 * term + 1)));
        sum = sum.add(&Rat::new(numerator, denominator).expect("positive denominator"));
        power = power.mul(&square);
    }
    let numerator = Int::from_i64(if terms & 1 == 0 { 1 } else { -1 });
    let denominator = power.mul(&Int::from_i64(i64::from(2 * terms + 1)));
    let adjacent = sum.add(&Rat::new(numerator, denominator).expect("positive denominator"));
    if sum < adjacent {
        (sum, adjacent)
    } else {
        (adjacent, sum)
    }
}

fn round_scaled(value: &Rat, fractional_bits: u32) -> Int {
    let numerator = value.numerator().abs().shl(fractional_bits);
    let (quotient, remainder) = numerator
        .div_rem(value.denominator())
        .expect("positive rational denominator");
    let odd = !quotient
        .div_rem(&Int::from_i64(2))
        .expect("two is nonzero")
        .1
        .is_zero();
    let comparison = remainder.shl(1).cmp(value.denominator());
    let rounded = if comparison.is_gt() || (comparison.is_eq() && odd) {
        quotient.add(&Int::one())
    } else {
        quotient
    };
    if value.numerator().is_negative() {
        rounded.neg()
    } else {
        rounded
    }
}

fn limbs(mut integer: Int) -> [u64; 4] {
    let mut output = [0; 4];
    let radix = Int::one().shl(64);
    for limb in &mut output {
        let (quotient, remainder) = integer.div_rem(&radix).expect("nonzero radix");
        *limb = u64::try_from(remainder.to_i128().expect("one limb fits i128"))
            .expect("nonnegative limb fits u64");
        integer = quotient;
    }
    assert!(integer.is_zero(), "four limbs hold the generated constant");
    output
}

fn coefficients(sine: bool) -> [i128; 10] {
    let mut output = [0; 10];
    for (term, coefficient) in output.iter_mut().enumerate() {
        let degree = 2 * term + usize::from(sine);
        let factorial = (1..=degree).fold(Int::one(), |value, factor| {
            value.mul(&Int::from_i128(factor as i128))
        });
        let numerator = Int::from_i64(if term & 1 == 0 { 1 } else { -1 });
        let rational = Rat::new(numerator, factorial).expect("positive factorial");
        *coefficient = round_scaled(&rational, 96)
            .to_i128()
            .expect("a Q96 coefficient fits i128");
    }
    output
}

#[derive(Debug)]
pub(crate) struct GeneratedConstants {
    pub(crate) pi192: [u64; 4],
    pub(crate) pi96: u128,
    pub(crate) sine: [i128; 10],
    pub(crate) cosine: [i128; 10],
}

pub(crate) fn certified_constants() -> GeneratedConstants {
    let (lo5, hi5) = atan_interval(5, 44);
    let (lo239, hi239) = atan_interval(239, 13);
    let lo = lo5
        .mul(&Rat::from_i64(16))
        .sub(&hi239.mul(&Rat::from_i64(4)));
    let hi = hi5
        .mul(&Rat::from_i64(16))
        .sub(&lo239.mul(&Rat::from_i64(4)));
    let width_limit = Rat::new(Int::one(), Int::one().shl(200)).expect("power of two");
    assert!(hi.sub(&lo) < width_limit);
    let pi192 = round_scaled(&lo, 192);
    assert_eq!(
        pi192,
        round_scaled(&hi, 192),
        "the enclosure certifies rounding"
    );
    let pi96 = round_scaled(
        &Rat::new(pi192.clone(), Int::one().shl(192)).expect("power of two"),
        96,
    );
    GeneratedConstants {
        pi192: limbs(pi192),
        pi96: u128::try_from(pi96.to_i128().expect("Q96 pi fits i128")).expect("positive pi"),
        sine: coefficients(true),
        cosine: coefficients(false),
    }
}
