// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Matching oracle cases for the instruction and allocation windows.

use std::hint::black_box;

use purrdf_testkit::exact::{Direction, Rational};
use purrdf_xsd::exact::{Decimal, Integer};
use purrdf_xsd::{XsdValue, numeric_div};

pub(super) const CATEGORIES: [&str; 3] = ["terminating", "repeating", "wide"];

pub(super) struct Case {
    left: XsdValue,
    right: XsdValue,
    expected: String,
}

pub(super) fn fixture(category: &str, operations: u32) -> Vec<Case> {
    (0..operations)
        .map(|i| {
            let a = 6 * i128::from(i) + 1;
            let b = 2_i128.pow(i % 11) * 5_i128.pow(i % 6);
            let (a, b) = match category {
                "terminating" => (a, b),
                "repeating" => (a, 3 * b),
                "wide" => ((1_i128 << 100) + a, (3 * b) << 80),
                _ => panic!("unknown division fixture: {category}"),
            };
            let signed = |value: i128, bit| if i & bit == 0 { value } else { -value };
            let (a, b) = (signed(a, 1), signed(b, 2));
            let (left_scale, right_scale) = (i % 7, (i / 7) % 7);
            let exact = Rational::from_decimal(a, left_scale)
                .div(&Rational::from_decimal(b, right_scale))
                .unwrap();
            let expected = exact.to_canonical_decimal().unwrap_or_else(|| {
                exact
                    .round_to_scale(18, Direction::HalfEven)
                    .canonical_terminating()
            });
            let value = |coefficient, scale| {
                XsdValue::Decimal(Decimal::new(Integer::from_i128(coefficient), scale))
            };
            Case {
                left: value(a, left_scale),
                right: value(b, right_scale),
                expected,
            }
        })
        .collect()
}

pub(super) fn run(cases: &[Case], output: &mut Vec<XsdValue>) {
    for case in cases {
        output.push(numeric_div(black_box(&case.left), black_box(&case.right)).unwrap());
    }
}

pub(super) fn validate(cases: &[Case], output: &[XsdValue]) {
    assert_eq!(cases.len(), output.len());
    for (case, answer) in cases.iter().zip(output) {
        let XsdValue::Decimal(answer) = answer else {
            panic!("decimal quotient returned a different type");
        };
        assert_eq!(answer.to_string(), case.expected);
    }
}
