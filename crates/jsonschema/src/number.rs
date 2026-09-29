// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact decimal arithmetic for JSON Schema numbers.
//! Parsed number lexemes are retained by `serde_json/arbitrary_precision`;
//! digits and exponents past a machine word compute on
//! [`purrdf_xsd::bigint::BigInt`].

use purrdf_xsd::bigint::BigInt;
use serde_json::Number;
use std::cmp::Ordering;

/// A normalized decimal coefficient and base-ten exponent.
///
/// The coefficient stays a digit string and the exponent is a machine word
/// until it is not: JSON permits arbitrarily many exponent digits, and JSON
/// Schema compares and divides numbers by their exact value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Decimal {
    negative: bool,
    coefficient: String,
    exponent: Exponent,
}

/// Signed base-ten exponent. JSON permits arbitrarily many exponent digits;
/// `Large` holds only values outside `i64`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Exponent {
    Small(i64),
    Large(BigInt),
}

impl Exponent {
    fn parse(source: &str) -> Self {
        if let Ok(value) = source.parse() {
            return Self::Small(value);
        }
        Self::from_big(BigInt::from_digits(source).unwrap_or_else(BigInt::zero))
    }

    fn from_big(value: BigInt) -> Self {
        value
            .to_i128()
            .and_then(|value| i64::try_from(value).ok())
            .map_or(Self::Large(value), Self::Small)
    }

    fn to_big(&self) -> BigInt {
        match self {
            Self::Small(value) => BigInt::from_i128(i128::from(*value)),
            Self::Large(value) => value.clone(),
        }
    }

    fn offset(self, amount: i64) -> Self {
        if let Self::Small(value) = self
            && let Some(sum) = value.checked_add(amount)
        {
            return Self::Small(sum);
        }
        let mut sum = self.to_big();
        sum.add_i128(i128::from(amount));
        Self::from_big(sum)
    }

    fn to_i64(&self) -> Option<i64> {
        match self {
            Self::Small(value) => Some(*value),
            Self::Large(_) => None,
        }
    }

    fn is_nonnegative(&self) -> bool {
        match self {
            Self::Small(value) => *value >= 0,
            Self::Large(value) => !value.is_negative(),
        }
    }

    fn difference(&self, other: &Self) -> Self {
        let mut difference = self.to_big();
        difference.add_assign(&other.to_big().negated());
        Self::from_big(difference)
    }
}

impl Ord for Exponent {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Small(left), Self::Small(right)) => left.cmp(right),
            _ => self.to_big().cmp(&other.to_big()),
        }
    }
}
impl PartialOrd for Exponent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Decimal {
    pub(crate) fn from_number(number: &Number) -> Self {
        let source = number.to_string();
        let body = source.strip_prefix('-').unwrap_or(&source);
        let negative = body.len() != source.len();
        let (mantissa, exponent) = body.split_once(['e', 'E']).unwrap_or((body, "0"));
        let explicit_exponent = Exponent::parse(exponent);
        let mut coefficient = String::with_capacity(mantissa.len());
        let mut fractional = 0_i64;
        let mut after_dot = false;
        for ch in mantissa.chars() {
            if ch == '.' {
                after_dot = true;
            } else {
                coefficient.push(ch);
                if after_dot {
                    fractional += 1;
                }
            }
        }
        let first = coefficient
            .find(|ch| ch != '0')
            .unwrap_or(coefficient.len());
        if first == coefficient.len() {
            return Self {
                negative: false,
                coefficient: "0".to_owned(),
                exponent: Exponent::Small(0),
            };
        }
        coefficient.drain(..first);
        let trailing = coefficient.trim_end_matches('0').len();
        let removed = coefficient.len() - trailing;
        coefficient.truncate(trailing);
        Self {
            negative,
            coefficient,
            exponent: explicit_exponent
                .offset(-fractional)
                .offset(i64::try_from(removed).expect("number length")),
        }
    }

    pub(crate) fn parts(&self) -> (bool, &str, &Exponent) {
        (self.negative, &self.coefficient, &self.exponent)
    }

    pub(crate) fn is_integer(&self) -> bool {
        self.exponent.is_nonnegative()
    }
    pub(crate) fn is_positive(&self) -> bool {
        !self.negative && self.coefficient != "0"
    }

    pub(crate) fn to_u64_saturating(&self) -> Option<u64> {
        if self.negative || !self.is_integer() {
            return None;
        }
        if self.exponent.to_i64().is_none() {
            return Some(u64::MAX);
        }
        let total = i64::try_from(self.coefficient.len())
            .ok()?
            .saturating_add(self.exponent.to_i64()?);
        if total > 20 {
            return Some(u64::MAX);
        }
        let mut result = self.coefficient.parse::<u64>().unwrap_or(u64::MAX);
        for _ in 0..self.exponent.to_i64()? {
            result = result.saturating_mul(10);
        }
        Some(result)
    }

    fn cmp_magnitude(&self, other: &Self) -> Ordering {
        match (self.coefficient == "0", other.coefficient == "0") {
            (true, true) => return Ordering::Equal,
            (true, false) => return Ordering::Less,
            (false, true) => return Ordering::Greater,
            (false, false) => {}
        }
        let left_order = self
            .exponent
            .clone()
            .offset(i64::try_from(self.coefficient.len()).unwrap_or(i64::MAX));
        let right_order = other
            .exponent
            .clone()
            .offset(i64::try_from(other.coefficient.len()).unwrap_or(i64::MAX));
        match left_order.cmp(&right_order) {
            Ordering::Equal => {
                let left = self.coefficient.as_bytes();
                let right = other.coefficient.as_bytes();
                for index in 0..left.len().max(right.len()) {
                    match left
                        .get(index)
                        .copied()
                        .unwrap_or(b'0')
                        .cmp(&right.get(index).copied().unwrap_or(b'0'))
                    {
                        Ordering::Equal => {}
                        order => return order,
                    }
                }
                Ordering::Equal
            }
            order => order,
        }
    }

    pub(crate) fn is_multiple_of(&self, divisor: &Self) -> bool {
        if self.coefficient == "0" {
            return true;
        }
        if divisor.coefficient == "0" {
            return false;
        }
        if self.cmp_magnitude(divisor) == Ordering::Less {
            return false;
        }
        // Divide the scaled coefficients in base ten. The remainder has at
        // most the divisor's digits, even for arbitrarily long input numbers.
        let (numerator, denominator, zeros) = if self.exponent >= divisor.exponent {
            (
                &self.coefficient,
                &divisor.coefficient,
                self.exponent.difference(&divisor.exponent),
            )
        } else {
            // The divisor has at least one extra factor of ten. Its normalized
            // coefficient has no trailing zero, so it cannot divide ours.
            return false;
        };
        if let (Ok(value), Ok(modulus)) = (numerator.parse::<u128>(), denominator.parse::<u128>()) {
            return mul_mod(value % modulus, pow_mod(10, &zeros, modulus), modulus) == 0;
        }
        let modulus = BigInt::from_digits(denominator).expect("a coefficient is digits");
        let reduce = |value: BigInt| value.rem(&modulus).expect("a nonzero coefficient");
        let mut remainder =
            reduce(BigInt::from_digits(numerator).expect("a coefficient is digits"));
        // Repeated squaring makes large exponents bounded by log(exponent).
        let mut power = reduce(BigInt::from_i128(10));
        let mut exponent = zeros.to_big();
        while !exponent.is_zero() {
            if exponent.is_odd() {
                remainder = reduce(remainder.mul(&power));
            }
            power = reduce(power.mul(&power));
            exponent = exponent.div_rem_u64(2).expect("a nonzero divisor").0;
        }
        remainder.is_zero()
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

fn mul_mod(left: u128, right: u128, modulus: u128) -> u128 {
    if let Some(product) = left.checked_mul(right) {
        return product % modulus;
    }
    let mut result = 0;
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

fn pow_mod(base: u128, exponent: &Exponent, modulus: u128) -> u128 {
    let mut result = 1 % modulus;
    let mut square = base % modulus;
    let mut exponent = exponent.to_big();
    while !exponent.is_zero() {
        if exponent.is_odd() {
            result = mul_mod(result, square, modulus);
        }
        square = mul_mod(square, square, modulus);
        exponent = exponent.div_rem_u64(2).expect("a nonzero divisor").0;
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
            (false, false) => self.cmp_magnitude(other),
            (true, true) => other.cmp_magnitude(self),
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
    fn exact_equality_and_order() {
        assert_eq!(dec("1"), dec("1.0"));
        assert_ne!(dec("1.0"), dec("1.0000000000000001"));
        assert!(dec("18446744073709551600") < dec("18446744073709551615"));
        assert!(dec("-18446744073709551600") > dec("-18446744073709551615"));
        assert!(dec("0.0001") < dec("0.001"));
        assert!(dec("-1e308") < dec("1e-308"));
        assert!(dec("12345678910111213141516171819202122232425262728293031").is_integer());
        assert_eq!(
            dec("1e999999999999999999999"),
            dec("10e999999999999999999998")
        );
        assert_eq!(dec("1e0000000000000000000000001"), dec("10"));
        assert!(dec("1e999999999999999999999") > dec("1e999999999999999999998"));
        assert!(dec("1e-999999999999999999999") < dec("1e-999999999999999999998"));
    }
    #[test]
    fn exact_divisibility() {
        for (value, divisor, expected) in [
            ("0.0075", "0.0001", true),
            ("0.00751", "0.0001", false),
            ("1e308", "0.5", true),
            ("4.5", "1.5", true),
            ("35", "1.5", false),
            ("19.99", "0.01", true),
            ("7", "2", false),
            ("18446744073709551615", "5", true),
            ("1.0000000000000001", "0.1", false),
            ("1e999999999999999999999", "2", true),
            ("1e-999999999999999999999", "2", false),
        ] {
            assert_eq!(
                dec(value).is_multiple_of(&dec(divisor)),
                expected,
                "{value}/{divisor}"
            );
        }
    }
    #[test]
    fn counts_saturate() {
        assert_eq!(dec("3.0").to_u64_saturating(), Some(3));
        assert_eq!(dec("1e30").to_u64_saturating(), Some(u64::MAX));
        assert_eq!(dec("3.5").to_u64_saturating(), None);
    }

    #[test]
    fn frozen_python_fraction_oracle() {
        for line in include_str!("../tests/numeric_oracle_vectors.txt").lines() {
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            let mut fields = line.split('|');
            let op = fields.next().expect("operation");
            let left = fields.next().expect("left");
            let right = fields.next().expect("right");
            let expected = fields.next().expect("answer");
            assert_eq!(fields.next(), None, "{line}");
            let answer = match op {
                "cmp" => match dec(left).cmp(&dec(right)) {
                    Ordering::Less => "<",
                    Ordering::Equal => "=",
                    Ordering::Greater => ">",
                },
                "mul" => {
                    if dec(left).is_multiple_of(&dec(right)) {
                        "true"
                    } else {
                        "false"
                    }
                }
                _ => panic!("unknown oracle operation: {op}"),
            };
            assert_eq!(answer, expected, "{line}");
        }
    }
}
