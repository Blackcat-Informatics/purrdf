// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Independent unbounded arithmetic for the published per-operation law.
//!
//! Testkit's schoolbook natural numbers share no production Integer/Fixed/wide
//! kernel. Parameters are raw input values, never computed production scores.

use purrdf_testkit::exact::Natural;
use purrdf_text::FieldInput;

const SCALE: u128 = 1_000_000_000_000;
const INTERNAL: u128 = 1_000_000_000_000_000_000;

fn natural(value: u128) -> Natural {
    Natural::from_u128(value)
}

fn quotient_product(left: &Natural, right: &Natural, divisor: &Natural) -> Natural {
    left.mul(right).div_rem(divisor).0
}

fn logarithm(raw: &Natural) -> Natural {
    let mut exponent = 0;
    while *raw >= natural(2 * SCALE).shl(exponent) {
        exponent += 1;
    }
    let mantissa = quotient_product(raw, &natural(INTERNAL / SCALE), &natural(1).shl(exponent));
    let z = quotient_product(
        &mantissa.sub(&natural(INTERNAL)),
        &natural(INTERNAL),
        &mantissa.add(&natural(INTERNAL)),
    );
    let square = quotient_product(&z, &z, &natural(INTERNAL));
    let mut power = z;
    let mut series = Natural::default();
    for denominator in (1..40).step_by(2) {
        series = series.add(&power.div_rem(&natural(denominator)).0);
        power = quotient_product(&power, &square, &natural(INTERNAL));
    }
    natural(u128::from(exponent) * 693_147_180_559_945_309)
        .add(&series.mul_small(2))
        .div_rem(&natural(INTERNAL / SCALE))
        .0
}

/// One term's exact raw contribution, with all intermediate integers unbounded.
pub(crate) fn contribution(
    documents: u64,
    frequency: u64,
    populations: &[u64],
    totals: &[u128],
    parameters: &[(i128, i128)],
    inputs: &[FieldInput],
) -> i128 {
    assert_eq!(populations.len(), totals.len());
    assert_eq!(parameters.len(), totals.len());
    assert_eq!(inputs.len(), totals.len());
    let scale = natural(SCALE);
    let idf = logarithm(&scale.add(&quotient_product(
        &natural(u128::from(documents - frequency) * 2 + 1),
        &scale,
        &natural(u128::from(frequency) * 2 + 1),
    )));
    let mut pseudo = Natural::default();
    for (((input, &(weight, b)), &population), &total) in
        inputs.iter().zip(parameters).zip(populations).zip(totals)
    {
        if input.term_frequency != 0 {
            let relative = quotient_product(
                &natural(u128::from(input.length) * u128::from(population)),
                &scale,
                &natural(total),
            );
            let coefficient = natural(u128::try_from(b).expect("nonnegative coefficient"));
            let normalization =
                scale
                    .sub(&coefficient)
                    .add(&quotient_product(&coefficient, &relative, &scale));
            let normalized = quotient_product(
                &natural(u128::from(input.term_frequency) * SCALE),
                &scale,
                &normalization,
            );
            pseudo = pseudo.add(&quotient_product(
                &normalized,
                &natural(u128::try_from(weight).expect("nonnegative weight")),
                &scale,
            ));
        }
    }
    let k1 = natural(12 * SCALE / 10);
    let numerator = quotient_product(&pseudo, &k1.add(&scale), &scale);
    let saturation = quotient_product(&numerator, &scale, &pseudo.add(&k1));
    i128::try_from(
        quotient_product(&idf, &saturation, &scale)
            .to_u128()
            .expect("the output score fits its certified public representation"),
    )
    .expect("nonnegative score fits i128")
}
