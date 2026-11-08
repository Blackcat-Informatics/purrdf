// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Fixed canonical operands and complete reference results for reciprocal counts.

use purrdf_xsd::exact::{Integer, Rational};
use std::str::FromStr;

pub(super) const CATEGORIES: [&str; 3] = ["positive_compact", "negative_minimum", "wide"];

pub(super) struct Fixture {
    inputs: Vec<Rational>,
    expected: Vec<Rational>,
}

impl Fixture {
    pub(super) fn new(category: &str, operations: usize) -> Self {
        let values: Vec<Rational> = match category {
            "positive_compact" => [
                "1/1",
                "2/1",
                "1/2",
                "3/2",
                "21/34",
                "355/113",
                "7/11",
                "99991/99989",
                "298257223563/1000000000",
                "17/31",
                "31/17",
                "1/127",
                "127/1",
                "9223372036854775807/3",
                "3/9223372036854775807",
                "18446744073709551617/19",
            ]
            .into_iter()
            .map(|text| Rational::from_str(text).expect("positive rational"))
            .collect(),
            "negative_minimum" => [
                format!("{}/1", i128::MIN),
                format!("{}/3", i128::MIN),
                format!("{}/{}", i128::MIN, i128::MAX),
                "-1/1".into(),
                "-1/2".into(),
                "-3/2".into(),
                "-21/34".into(),
                "-355/113".into(),
                "-7/11".into(),
                "-99991/99989".into(),
                "-298257223563/1000000000".into(),
                "-17/31".into(),
                "-31/17".into(),
                "-1/127".into(),
                "-127/1".into(),
                "-9223372036854775808/3".into(),
            ]
            .iter()
            .map(|text| Rational::from_str(text).expect("negative rational"))
            .collect(),
            "wide" => {
                let mut values = Vec::new();
                for bits in [256, 1024] {
                    let power = Integer::from(2_i128).pow(bits);
                    for (numerator, denominator) in [
                        (
                            &power + &Integer::ONE,
                            &Integer::from(2_i128).pow(bits - 1) + &Integer::ONE,
                        ),
                        (&power - &Integer::ONE, &power + &Integer::ONE),
                        (&power + &Integer::from(17_i128), Integer::ONE),
                        (Integer::ONE, &power + &Integer::from(37_i128)),
                    ] {
                        values.push(
                            Rational::new(numerator.clone(), denominator.clone())
                                .expect("positive denominator"),
                        );
                        values.push(
                            Rational::new(-numerator, denominator).expect("positive denominator"),
                        );
                    }
                }
                values
            }
            _ => panic!("unknown reciprocal category: {category}"),
        };
        assert_eq!(values.len(), 16);
        // The original checked constructor remains the canonical reference,
        // independent of the reciprocal body under measurement.
        let expected_values: Vec<_> = values
            .iter()
            .map(|value| {
                Rational::new(value.denominator().clone(), value.numerator().clone())
                    .expect("nonzero input")
            })
            .collect();
        let inputs = values.iter().cloned().cycle().take(operations).collect();
        let expected = expected_values
            .iter()
            .cloned()
            .cycle()
            .take(operations)
            .collect();
        Self { inputs, expected }
    }

    pub(super) fn run(&self, outputs: &mut Vec<Rational>) {
        assert_eq!(outputs.as_slice(), []);
        assert!(outputs.capacity() >= self.inputs.len());
        for value in &self.inputs {
            outputs.push(std::hint::black_box(value).recip().expect("nonzero input"));
        }
        std::hint::black_box(&*outputs);
    }

    pub(super) fn validate(&self, outputs: &[Rational]) {
        assert_eq!(outputs, self.expected);
    }
}
