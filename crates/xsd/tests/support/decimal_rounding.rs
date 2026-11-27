// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Operands with an independently known answer on each side of a rounding boundary.

use purrdf_xsd::exact::{Decimal, Rounding};
use std::hint::black_box;

pub(crate) const CATEGORIES: [&str; 3] = ["compact", "digits_4000", "digits_40000"];
pub(crate) const CASES: usize = 8;

pub(crate) struct Fixture([(Decimal, Decimal); CASES]);

impl Fixture {
    pub(crate) fn new(category: &str) -> Self {
        let digits = match category {
            "compact" => 8,
            "digits_4000" => 4_000,
            "digits_40000" => 40_000,
            _ => panic!("unknown decimal rounding category: {category}"),
        };
        let above = format!("5{}1", "0".repeat(digits - 2));
        let below = format!("4{}", "9".repeat(digits - 1));
        Self(std::array::from_fn(|index| {
            let whole = 20 + index;
            let negative = index & 1 != 0;
            let rounds_up = index & 2 == 0;
            let sign = if negative { "-" } else { "" };
            let fraction = if rounds_up { &above } else { &below };
            let rounded = whole + usize::from(rounds_up);
            (
                format!("{sign}{whole}.{fraction}").parse().unwrap(),
                format!("{sign}{rounded}").parse().unwrap(),
            )
        }))
    }

    pub(crate) fn run(&self, output: &mut Vec<Decimal>) {
        for (value, _) in &self.0 {
            output.push(
                black_box(value)
                    .try_round_admitted(0, Rounding::HalfCeiling, &mut |_| Ok(()))
                    .unwrap(),
            );
        }
    }

    pub(crate) fn validate(&self, output: &[Decimal]) {
        assert_eq!(output.len(), self.0.len());
        for (actual, (_, expected)) in output.iter().zip(&self.0) {
            assert_eq!(actual, expected);
        }
    }
}
