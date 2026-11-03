// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Shared fixed-work inputs and full answer guards for the public float factory.

use purrdf_xsd::exact::{Decimal, Rational};

pub(super) const OPERATIONS: usize = 8192;
pub(super) const CATEGORIES: [&str; 3] = ["fractional", "integral", "extreme"];

pub(super) struct Fixture {
    inputs: Vec<f64>,
    expected: Vec<Rational>,
}

impl Fixture {
    pub(super) fn new(category: &str) -> Self {
        let values: &[f64] = match category {
            "fractional" => &[
                0.1,
                -0.1,
                0.5,
                -0.5,
                1.5,
                -1.5,
                0.25,
                -0.25,
                1.0 / 3.0,
                -1.0 / 3.0,
                0.125,
                -0.125,
                1.1,
                -1.1,
                2.75,
                -2.75,
            ],
            "integral" => &[
                0.0,
                -0.0,
                1.0,
                -1.0,
                2.0,
                -2.0,
                42.0,
                -42.0,
                2_f64.powi(53),
                -2_f64.powi(53),
                2_f64.powi(127),
                -2_f64.powi(127),
                2_f64.powi(200),
                -2_f64.powi(200),
                2_f64.powi(500),
                -2_f64.powi(500),
            ],
            "extreme" => &[
                f64::from_bits(1),
                -f64::from_bits(1),
                f64::from_bits(2),
                -f64::from_bits(2),
                f64::from_bits((1_u64 << 52) - 1),
                -f64::from_bits((1_u64 << 52) - 1),
                f64::MIN_POSITIVE,
                -f64::MIN_POSITIVE,
                f64::from_bits((1_u64 << 52) + 1),
                -f64::from_bits((1_u64 << 52) + 1),
                f64::MAX,
                f64::MIN,
                2_f64.powi(1023),
                -2_f64.powi(1023),
                f64::from_bits(f64::MAX.to_bits() - 1),
                -f64::from_bits(f64::MAX.to_bits() - 1),
            ],
            _ => panic!("unknown float factory category: {category}"),
        };
        let inputs: Vec<_> = values.iter().copied().cycle().take(OPERATIONS).collect();
        let expected_values: Vec<_> = values
            .iter()
            .map(|&value| {
                Rational::from_decimal(&Decimal::from_f64(value).expect("finite fixture"))
            })
            .collect();
        let expected = expected_values
            .iter()
            .cloned()
            .cycle()
            .take(OPERATIONS)
            .collect();
        Self { inputs, expected }
    }

    /// The output vector is reserved before any measurement window.
    pub(super) fn run(&self, outputs: &mut Vec<Rational>) {
        assert_eq!(outputs.as_slice(), []);
        assert!(outputs.capacity() >= self.inputs.len());
        for &value in &self.inputs {
            outputs.push(Rational::from_f64(std::hint::black_box(value)).expect("finite fixture"));
        }
        std::hint::black_box(&*outputs);
    }

    pub(super) fn validate(&self, outputs: &[Rational]) {
        assert_eq!(outputs, self.expected);
    }
}
