// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact arithmetic over JSON numbers.
//!
//! JSON Schema compares numbers by their mathematical value: `1` and `1.0` are
//! the same number and both are integers, `0.0075` is a multiple of `0.0001`,
//! and `1e308` is a multiple of `0.5`. Binary floating point answers none of
//! those reliably, so every number is read into a [`Decimal`] — a sign, an
//! integer coefficient and a power of ten — and compared and divided exactly.
//!
//! A number that `serde_json` holds as `u64`/`i64` is exact already. One it
//! holds as `f64` is read through Rust's shortest round-trip rendering, which
//! is the shortest decimal that parses back to the same binary64; with the
//! workspace's `float_roundtrip` parse that is the decimal the document spelled
//! whenever the document spelled at most seventeen significant digits.

use std::cmp::Ordering;

use serde_json::Number;

/// `(-1)^negative × coefficient × 10^exponent`, normalized so the coefficient
/// has no trailing zero digit and zero is `+0 × 10^0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Decimal {
    negative: bool,
    coefficient: u128,
    exponent: i32,
}

impl Decimal {
    const ZERO: Self = Self {
        negative: false,
        coefficient: 0,
        exponent: 0,
    };

    fn new(negative: bool, mut coefficient: u128, mut exponent: i32) -> Self {
        if coefficient == 0 {
            return Self::ZERO;
        }
        while coefficient.is_multiple_of(10) {
            coefficient /= 10;
            exponent += 1;
        }
        Self {
            negative,
            coefficient,
            exponent,
        }
    }

    /// The exact value of a parsed JSON number.
    pub(crate) fn from_number(number: &Number) -> Self {
        if let Some(value) = number.as_u64() {
            return Self::new(false, u128::from(value), 0);
        }
        if let Some(value) = number.as_i64() {
            return Self::new(value < 0, u128::from(value.unsigned_abs()), 0);
        }
        // A JSON number is finite, so `serde_json` never holds a NaN or an
        // infinity here; `as_f64` is `Some` for every remaining number.
        let value = number.as_f64().unwrap_or(0.0);
        Self::from_f64(value)
    }

    fn from_f64(value: f64) -> Self {
        // `{:e}` is the shortest round-trip rendering in scientific form:
        // `-1.25e-7`, `1e308`, `0e0`.
        let text = format!("{value:e}");
        let (mantissa, exponent) = text.split_once('e').unwrap_or((text.as_str(), "0"));
        let negative = mantissa.starts_with('-');
        let mantissa = mantissa.trim_start_matches('-');
        let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
        let mut coefficient: u128 = 0;
        for digit in whole.bytes().chain(fraction.bytes()) {
            coefficient = coefficient * 10 + u128::from(digit - b'0');
        }
        let exponent: i32 = exponent.parse().unwrap_or(0);
        let fraction_digits = i32::try_from(fraction.len()).unwrap_or(0);
        Self::new(negative, coefficient, exponent - fraction_digits)
    }

    /// The normalized sign, coefficient and exponent.
    pub(crate) const fn parts(self) -> (bool, u128, i32) {
        (self.negative, self.coefficient, self.exponent)
    }

    /// Whether the value has no fractional part.
    pub(crate) const fn is_integer(self) -> bool {
        self.exponent >= 0
    }

    /// Whether the value is strictly positive.
    pub(crate) const fn is_positive(self) -> bool {
        !self.negative && self.coefficient != 0
    }

    /// The value as a `u64` when it is a non-negative integer that fits;
    /// `u64::MAX` for a larger non-negative integer (every bound this crate
    /// reads is a count, and no count exceeds a `u64`).
    pub(crate) fn to_u64_saturating(self) -> Option<u64> {
        if self.negative || !self.is_integer() {
            return None;
        }
        let exponent = u32::try_from(self.exponent).ok()?;
        let scaled = 10_u128
            .checked_pow(exponent)
            .and_then(|power| self.coefficient.checked_mul(power));
        Some(scaled.map_or(u64::MAX, |value| u64::try_from(value).unwrap_or(u64::MAX)))
    }

    fn digit_count(value: u128) -> i32 {
        let mut count = 1;
        let mut rest = value / 10;
        while rest > 0 {
            count += 1;
            rest /= 10;
        }
        count
    }

    fn cmp_magnitude(self, other: Self) -> Ordering {
        match (self.coefficient == 0, other.coefficient == 0) {
            (true, true) => return Ordering::Equal,
            (true, false) => return Ordering::Less,
            (false, true) => return Ordering::Greater,
            (false, false) => {}
        }
        let self_order = Self::digit_count(self.coefficient) + self.exponent;
        let other_order = Self::digit_count(other.coefficient) + other.exponent;
        if self_order != other_order {
            return self_order.cmp(&other_order);
        }
        // Equal orders of magnitude: shifting the coefficient with the larger
        // exponent left by the difference keeps it within the other one's
        // digit count, which is at most 39 digits and fits a u128.
        match self.exponent.cmp(&other.exponent) {
            Ordering::Equal => self.coefficient.cmp(&other.coefficient),
            Ordering::Greater => {
                let shift = (self.exponent - other.exponent).unsigned_abs();
                (self.coefficient * 10_u128.pow(shift)).cmp(&other.coefficient)
            }
            Ordering::Less => {
                let shift = (other.exponent - self.exponent).unsigned_abs();
                self.coefficient
                    .cmp(&(other.coefficient * 10_u128.pow(shift)))
            }
        }
    }

    /// Whether `self / divisor` is an integer; `divisor` must be positive.
    pub(crate) fn is_multiple_of(self, divisor: Self) -> bool {
        if self.coefficient == 0 {
            return true;
        }
        if divisor.coefficient == 0 {
            return false;
        }
        match self.exponent.cmp(&divisor.exponent) {
            Ordering::Less => {
                // self.c / (divisor.c × 10^k) with k > 0.
                let shift = (divisor.exponent - self.exponent).unsigned_abs();
                10_u128
                    .checked_pow(shift)
                    .and_then(|power| divisor.coefficient.checked_mul(power))
                    .is_some_and(|denominator| self.coefficient.is_multiple_of(denominator))
            }
            _ => {
                // (self.c × 10^k) mod divisor.c, with k ≥ 0, by modular
                // exponentiation. Both coefficients come from a u64 or from at
                // most seventeen f64 digits, so each is below 2^64 and every
                // product below stays inside a u128.
                let shift = (self.exponent - divisor.exponent).unsigned_abs();
                let modulus = divisor.coefficient;
                let power = pow_mod(10, shift, modulus);
                mul_mod(self.coefficient % modulus, power, modulus) == 0
            }
        }
    }
}

fn mul_mod(left: u128, right: u128, modulus: u128) -> u128 {
    match left.checked_mul(right) {
        Some(product) => product % modulus,
        None => {
            // Only reachable for a modulus above 2^64, which no JSON number
            // this crate reads produces; double-and-add keeps it exact anyway.
            let mut result = 0_u128;
            let mut addend = left % modulus;
            let mut factor = right;
            while factor > 0 {
                if factor & 1 == 1 {
                    result = add_mod(result, addend, modulus);
                }
                addend = add_mod(addend, addend, modulus);
                factor >>= 1;
            }
            result
        }
    }
}

fn add_mod(left: u128, right: u128, modulus: u128) -> u128 {
    let (sum, overflowed) = left.overflowing_add(right);
    if overflowed || sum >= modulus {
        sum.wrapping_sub(modulus)
    } else {
        sum
    }
}

fn pow_mod(base: u128, mut exponent: u32, modulus: u128) -> u128 {
    let mut result = 1 % modulus;
    let mut square = base % modulus;
    while exponent > 0 {
        if exponent & 1 == 1 {
            result = mul_mod(result, square, modulus);
        }
        square = mul_mod(square, square, modulus);
        exponent >>= 1;
    }
    result
}

impl PartialOrd for Decimal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Decimal {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.negative, other.negative) {
            (false, true) => Ordering::Greater,
            (true, false) => Ordering::Less,
            (false, false) => self.cmp_magnitude(*other),
            (true, true) => other.cmp_magnitude(*self),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dec(text: &str) -> Decimal {
        let value: serde_json::Value = serde_json::from_str(text).expect("number");
        Decimal::from_number(value.as_number().expect("number"))
    }

    #[test]
    fn integers_and_their_float_spellings_are_one_value() {
        assert_eq!(dec("1"), dec("1.0"));
        assert!(dec("1.0").is_integer());
        assert!(!dec("1.5").is_integer());
        assert!(dec("12345678910111213141516171819202122232425262728293031").is_integer());
        assert_eq!(dec("-0.0"), dec("0"));
    }

    #[test]
    fn ordering_is_exact_across_representations() {
        assert!(dec("18446744073709551600") < dec("18446744073709551615"));
        // Below i64::MIN both parse to the same binary64, so they are equal.
        assert!(dec("-18446744073709551600") >= dec("-18446744073709551615"));
        assert!(dec("-9223372036854775807") > dec("-9223372036854775808"));
        assert!(dec("1.5") > dec("1"));
        assert!(dec("0.0001") < dec("0.001"));
        assert!(dec("-1e308") < dec("1e-308"));
        assert_eq!(
            dec("9.727837981879871e26").cmp(&dec("9.727837981879871e26")),
            Ordering::Equal
        );
    }

    #[test]
    fn multiple_of_is_decimal_not_binary() {
        assert!(dec("0.0075").is_multiple_of(dec("0.0001")));
        assert!(!dec("0.00751").is_multiple_of(dec("0.0001")));
        assert!(dec("1e308").is_multiple_of(dec("0.5")));
        assert!(dec("4.5").is_multiple_of(dec("1.5")));
        assert!(!dec("35").is_multiple_of(dec("1.5")));
        assert!(dec("19.99").is_multiple_of(dec("0.01")));
        assert!(!dec("7").is_multiple_of(dec("2")));
        assert!(dec("0").is_multiple_of(dec("0.3")));
        assert!(dec("18446744073709551615").is_multiple_of(dec("5")));
    }

    #[test]
    fn counts_saturate_and_refuse_fractions() {
        assert_eq!(dec("3").to_u64_saturating(), Some(3));
        assert_eq!(dec("3.0").to_u64_saturating(), Some(3));
        assert_eq!(dec("1e30").to_u64_saturating(), Some(u64::MAX));
        assert_eq!(dec("3.5").to_u64_saturating(), None);
        assert_eq!(dec("-1").to_u64_saturating(), None);
    }
}
