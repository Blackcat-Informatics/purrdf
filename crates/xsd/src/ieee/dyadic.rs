// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact, normalized decomposition of finite IEEE binary64 values.
//!
//! Decoding reads only integer fields. A nonzero value is
//! `(-1)^negative × significand × 2^exponent`, with an odd significand.
//! Both zero encodings have significand and exponent zero; their sign is retained.

/// The exact dyadic value of a finite binary64, including its zero sign.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Binary64Dyadic {
    negative: bool,
    significand: u64,
    exponent: i32,
}

impl Binary64Dyadic {
    /// Decode a finite value, returning `None` for either infinity or any NaN.
    /// No floating-point arithmetic or environment validation is needed.
    #[must_use]
    pub const fn decode(value: f64) -> Option<Self> {
        let bits = value.to_bits();
        let biased = ((bits >> 52) & 0x7ff) as i32;
        if biased == 0x7ff {
            return None;
        }
        let fraction = bits & ((1_u64 << 52) - 1);
        let (mut significand, mut exponent) = if biased == 0 {
            (fraction, -1074)
        } else {
            (fraction | (1_u64 << 52), biased - 1075)
        };
        if significand == 0 {
            exponent = 0;
        } else {
            let trailing = significand.trailing_zeros();
            significand >>= trailing;
            exponent += trailing as i32;
        }
        Some(Self {
            negative: bits >> 63 != 0,
            significand,
            exponent,
        })
    }

    /// The original sign bit, including for negative zero.
    #[must_use]
    pub const fn negative(self) -> bool {
        self.negative
    }

    /// The odd nonzero significand, or zero for either zero encoding.
    #[must_use]
    pub const fn significand(self) -> u64 {
        self.significand
    }

    /// The exponent of two, in `[-1074, 1023]`, or zero for a zero value.
    #[must_use]
    pub const fn exponent(self) -> i32 {
        self.exponent
    }
}

#[cfg(test)]
mod tests {
    use super::Binary64Dyadic;

    #[test]
    fn finite_fields_normalize_without_losing_zero_sign() {
        let cases = [
            (0, false, 0, 0),
            (1_u64 << 63, true, 0, 0),
            (1, false, 1, -1074),
            (2, false, 1, -1073),
            ((1_u64 << 52) - 1, false, (1_u64 << 52) - 1, -1074),
            (1_u64 << 52, false, 1, -1022),
            (0.5_f64.to_bits(), false, 1, -1),
            ((-1.5_f64).to_bits(), true, 3, -1),
            (f64::MAX.to_bits(), false, (1_u64 << 53) - 1, 971),
        ];
        for (bits, negative, significand, exponent) in cases {
            let got = Binary64Dyadic::decode(f64::from_bits(bits)).expect("finite");
            assert_eq!(got.negative(), negative);
            assert_eq!(got.significand(), significand);
            assert_eq!(got.exponent(), exponent);
        }
        for value in [
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
            f64::from_bits(u64::MAX),
        ] {
            assert_eq!(Binary64Dyadic::decode(value), None);
        }
    }

    #[test]
    fn every_normal_power_of_two_has_one_significand() {
        for biased in 1_u64..0x7ff {
            for sign in [0, 1_u64 << 63] {
                let got = Binary64Dyadic::decode(f64::from_bits(sign | (biased << 52)))
                    .expect("normal power of two");
                assert_eq!(got.significand(), 1);
                assert_eq!(
                    got.exponent(),
                    i32::try_from(biased).expect("eleven bits") - 1023
                );
                assert_eq!(got.negative(), sign != 0);
            }
        }
    }
}
