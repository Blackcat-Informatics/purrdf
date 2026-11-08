// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact rational inputs and independent rounded-bit guards for conversion counts.

use purrdf_testkit::exact::Rational as Oracle;
use purrdf_xsd::exact::{Integer, Rational};
use std::str::FromStr;

pub(super) const CATEGORIES: [&str; 3] = ["compact", "wide_normal", "boundary"];

pub(super) struct Fixture {
    inputs: Vec<Rational>,
    expected: Vec<u64>,
}

impl Fixture {
    pub(super) fn new(category: &str, operations: usize) -> Self {
        let power = |bits| Integer::from(2_i128).pow(bits);
        let fraction = |n, d| Rational::new(n, d).expect("positive denominator");
        let signed = |values: Vec<Rational>| {
            values
                .into_iter()
                .flat_map(|value| {
                    let negative = -&value;
                    [value, negative]
                })
                .collect::<Vec<_>>()
        };
        let values = match category {
            "compact" => [
                "0/1".into(),
                "1/1".into(),
                "-1/1".into(),
                "1/3".into(),
                "-1/3".into(),
                "3/2".into(),
                "-3/2".into(),
                "9007199254740993/1".into(),
                "-9007199254740993/1".into(),
                "16777217/1".into(),
                "-16777217/1".into(),
                format!("{}/1", i128::MIN),
                format!("{}/3", i128::MIN),
                format!("{}/7", i128::MAX),
                format!("-{}/7", i128::MAX),
                "127/19".into(),
            ]
            .iter()
            .map(|text| Rational::from_str(text).expect("compact rational"))
            .collect(),
            "wide_normal" => {
                let mut values = Vec::new();
                for shift in [256, 1024] {
                    for offset in 0..4_i128 {
                        values.push(fraction(
                            &power(shift) + &Integer::from(17 + offset * 2),
                            &power(shift - 1) + &Integer::from(31 + offset * 2),
                        ));
                    }
                }
                signed(values)
            }
            "boundary" => signed(vec![
                fraction(Integer::ONE, power(1074)),
                fraction(Integer::ONE, power(1075)),
                fraction(Integer::from(3_i128), power(1075)),
                fraction(&power(52) - &Integer::ONE, power(1074)),
                fraction(Integer::ONE, power(1022)),
                fraction(&power(53) + &Integer::ONE, power(53)),
                Rational::from_integer(&(&power(53) - &Integer::ONE) * &power(971)),
                Rational::from_integer(&(&power(54) - &Integer::ONE) * &power(970)),
            ]),
            _ => panic!("unknown float rounding category: {category}"),
        };
        assert_eq!(values.len(), 16);
        let expected_values: Vec<_> = values
            .iter()
            .map(|value| {
                Oracle::parse(&value.numerator().to_string())
                    .expect("integer numerator")
                    .div(
                        &Oracle::parse(&value.denominator().to_string())
                            .expect("integer denominator"),
                    )
                    .expect("positive denominator")
                    .to_f64()
                    .to_bits()
            })
            .collect();
        Self {
            inputs: values.iter().cloned().cycle().take(operations).collect(),
            expected: expected_values
                .iter()
                .copied()
                .cycle()
                .take(operations)
                .collect(),
        }
    }

    pub(super) fn run(&self, outputs: &mut Vec<u64>) {
        assert_eq!(outputs.as_slice(), []);
        assert!(outputs.capacity() >= self.inputs.len());
        for value in &self.inputs {
            outputs.push(std::hint::black_box(value).to_f64().to_bits());
        }
        std::hint::black_box(&*outputs);
    }

    pub(super) fn validate(&self, outputs: &[u64]) {
        assert_eq!(outputs, self.expected);
    }
}
