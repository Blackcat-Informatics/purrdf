// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact, dependency-free decimal arithmetic for JSON Schema numbers.
//! Parsed number lexemes are retained by `serde_json/arbitrary_precision`.

use serde_json::Number;
use std::cmp::Ordering;

/// A normalized decimal coefficient and base-ten exponent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Decimal {
    negative: bool,
    coefficient: String,
    exponent: Exponent,
}

/// Signed base-ten exponent. JSON permits arbitrarily many exponent digits.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Exponent {
    Small(i64),
    Large { negative: bool, digits: String },
}

impl Exponent {
    fn parse(source: &str) -> Self {
        if let Ok(value) = source.parse() {
            return Self::Small(value);
        }
        let negative = source.starts_with('-');
        let digits = source
            .trim_start_matches(['+', '-'])
            .trim_start_matches('0');
        if digits.is_empty() {
            return Self::Small(0);
        }
        let normalized = if negative {
            format!("-{digits}")
        } else {
            digits.to_owned()
        };
        if let Ok(value) = normalized.parse() {
            return Self::Small(value);
        }
        Self::Large {
            negative,
            digits: digits.to_owned(),
        }
    }

    fn offset(self, amount: i64) -> Self {
        if let Self::Small(value) = self {
            if let Some(sum) = value.checked_add(amount) {
                return Self::Small(sum);
            }
            return Self::from_parts(
                value.is_negative(),
                &BigDec::from_digits(&value.unsigned_abs().to_string()),
            )
            .offset_big(amount);
        }
        self.offset_big(amount)
    }

    fn offset_big(self, amount: i64) -> Self {
        let (negative, mut magnitude) = self.parts();
        let other_negative = amount.is_negative();
        let other = BigDec::from_digits(&amount.unsigned_abs().to_string());
        let (negative, magnitude) = if negative == other_negative {
            magnitude.add(&other);
            (negative, magnitude)
        } else {
            match magnitude.cmp(&other) {
                Ordering::Greater | Ordering::Equal => {
                    magnitude.subtract(&other);
                    (negative, magnitude)
                }
                Ordering::Less => {
                    let mut other = other;
                    other.subtract(&magnitude);
                    (other_negative, other)
                }
            }
        };
        Self::from_parts(negative, &magnitude)
    }

    fn from_parts(negative: bool, magnitude: &BigDec) -> Self {
        let digits = magnitude.to_digits();
        let signed = if negative {
            format!("-{digits}")
        } else {
            digits.clone()
        };
        signed
            .parse()
            .map_or(Self::Large { negative, digits }, Self::Small)
    }

    fn parts(&self) -> (bool, BigDec) {
        match self {
            Self::Small(value) => (
                value.is_negative(),
                BigDec::from_digits(&value.unsigned_abs().to_string()),
            ),
            Self::Large { negative, digits } => (*negative, BigDec::from_digits(digits)),
        }
    }

    fn to_i64(&self) -> Option<i64> {
        match self {
            Self::Small(value) => Some(*value),
            Self::Large { .. } => None,
        }
    }

    fn is_nonnegative(&self) -> bool {
        match self {
            Self::Small(value) => *value >= 0,
            Self::Large { negative, .. } => !negative,
        }
    }

    fn difference(&self, other: &Self) -> Self {
        let (left_negative, mut left) = self.parts();
        let (right_negative, right) = other.parts();
        let (negative, magnitude) = if left_negative != right_negative {
            left.add(&right);
            (left_negative, left)
        } else {
            match left.cmp(&right) {
                Ordering::Greater | Ordering::Equal => {
                    left.subtract(&right);
                    (left_negative, left)
                }
                Ordering::Less => {
                    let mut right = right;
                    right.subtract(&left);
                    (!left_negative, right)
                }
            }
        };
        Self::from_parts(negative, &magnitude)
    }
}

impl Ord for Exponent {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Small(left), Self::Small(right)) => left.cmp(right),
            _ => {
                let (left_negative, left) = self.parts();
                let (right_negative, right) = other.parts();
                match (left_negative, right_negative) {
                    (true, false) => Ordering::Less,
                    (false, true) => Ordering::Greater,
                    (true, true) => right.cmp(&left),
                    (false, false) => left.cmp(&right),
                }
            }
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
        let modulus = BigDec::from_digits(denominator);
        let mut remainder = BigDec::zero();
        for digit in numerator.bytes() {
            remainder.mul_small(10);
            remainder.add_small(digit - b'0');
            remainder.reduce(&modulus);
        }
        // Repeated squaring makes large exponents bounded by log(exponent).
        let mut power = BigDec::from_digits("10");
        power.reduce(&modulus);
        let (_, mut exponent) = zeros.parts();
        while !exponent.is_zero() {
            if exponent.is_odd() {
                remainder = remainder.mul(&power).modulo(&modulus);
            }
            power = power.mul(&power).modulo(&modulus);
            exponent.div_two();
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
    let (_, mut exponent) = exponent.parts();
    while !exponent.is_zero() {
        if exponent.is_odd() {
            result = mul_mod(result, square, modulus);
        }
        square = mul_mod(square, square, modulus);
        exponent.div_two();
    }
    result
}

/// Little-endian decimal digits. Only divisibility needs arithmetic beyond
/// comparison, and this avoids a runtime big-integer dependency.
#[derive(Clone)]
struct BigDec(Vec<u8>);
impl BigDec {
    fn zero() -> Self {
        Self(vec![0])
    }
    fn from_digits(s: &str) -> Self {
        let mut value = Self(s.bytes().rev().map(|b| b - b'0').collect());
        value.trim();
        value
    }
    fn to_digits(&self) -> String {
        self.0
            .iter()
            .rev()
            .map(|digit| char::from(b'0' + digit))
            .collect()
    }
    fn is_zero(&self) -> bool {
        self.0.len() == 1 && self.0[0] == 0
    }
    fn trim(&mut self) {
        while self.0.len() > 1 && self.0.last() == Some(&0) {
            self.0.pop();
        }
    }
    fn cmp(&self, other: &Self) -> Ordering {
        self.0
            .len()
            .cmp(&other.0.len())
            .then_with(|| self.0.iter().rev().cmp(other.0.iter().rev()))
    }
    fn add_small(&mut self, n: u8) {
        let mut carry = n;
        for digit in &mut self.0 {
            let sum = *digit + carry;
            *digit = sum % 10;
            carry = sum / 10;
            if carry == 0 {
                return;
            }
        }
        if carry != 0 {
            self.0.push(carry);
        }
    }
    fn add(&mut self, other: &Self) {
        let mut carry = 0_u8;
        self.0.resize(self.0.len().max(other.0.len()), 0);
        for (index, digit) in self.0.iter_mut().enumerate() {
            let sum = *digit + other.0.get(index).copied().unwrap_or(0) + carry;
            *digit = sum % 10;
            carry = sum / 10;
        }
        if carry != 0 {
            self.0.push(carry);
        }
    }
    fn is_odd(&self) -> bool {
        self.0[0] & 1 != 0
    }
    fn div_two(&mut self) {
        let mut carry = 0_u8;
        for digit in self.0.iter_mut().rev() {
            let value = carry * 10 + *digit;
            *digit = value / 2;
            carry = value % 2;
        }
        self.trim();
    }
    fn mul_small(&mut self, n: u8) {
        let mut carry = 0;
        for digit in &mut self.0 {
            let product = *digit * n + carry;
            *digit = product % 10;
            carry = product / 10;
        }
        while carry != 0 {
            self.0.push(carry % 10);
            carry /= 10;
        }
        self.trim();
    }
    fn subtract(&mut self, other: &Self) {
        let mut borrow = 0_i8;
        for (index, digit) in self.0.iter_mut().enumerate() {
            let diff = *digit as i8 - other.0.get(index).copied().unwrap_or(0) as i8 - borrow;
            if diff < 0 {
                *digit = (diff + 10) as u8;
                borrow = 1;
            } else {
                *digit = diff as u8;
                borrow = 0;
            }
        }
        self.trim();
    }
    fn reduce(&mut self, modulus: &Self) {
        while self.cmp(modulus) != Ordering::Less {
            self.subtract(modulus);
        }
    }
    fn modulo(mut self, modulus: &Self) -> Self {
        let mut result = Self::zero();
        for digit in self.0.drain(..).rev() {
            result.mul_small(10);
            result.add_small(digit);
            result.reduce(modulus);
        }
        result
    }
    fn mul(&self, other: &Self) -> Self {
        let mut result = vec![0_u8; self.0.len() + other.0.len()];
        for (i, &left) in self.0.iter().enumerate() {
            let mut carry = 0_u8;
            for (j, &right) in other.0.iter().enumerate() {
                let value = result[i + j] + left * right + carry;
                result[i + j] = value % 10;
                carry = value / 10;
            }
            result[i + other.0.len()] += carry;
        }
        let mut out = Self(result);
        out.trim();
        out
    }
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
