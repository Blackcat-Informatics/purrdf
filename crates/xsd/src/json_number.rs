// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact values of JSON number lexemes: order, equality, integrality and
//! divisibility.
//!
//! RFC 8259 §6 puts no bound on a number's digits or on its exponent, and JSON
//! Schema compares and divides numbers by their mathematical value (`1.0`
//! equals `1`, `1e999999999999999999999` is a number like any other, and
//! `0.0075` is a multiple of `0.0001`). Converting either side to a machine
//! float rounds, and converting to a machine integer overflows, so nothing here
//! converts: a [`JsonNumber`] is the lexeme reduced to its significant digits
//! and a base-ten exponent, the exponent a machine word until it is not and a
//! [`BigInt`] after that.
//!
//! This is the one exact reading of a JSON number in the workspace: the JSON
//! Schema validator's `const`, `enum`, `uniqueItems`, range and `multipleOf`
//! keywords all compute on it, and [`cmp`] is its order over two lexemes.

use core::cmp::Ordering;
use core::hash::{Hash, Hasher};

use crate::bigint::BigInt;

/// A signed base-ten exponent. `Large` holds only values outside `i64`, so the
/// representation is canonical and the derived equality and hash are the
/// value's.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Exponent {
    Small(i64),
    Large(BigInt),
}

impl Exponent {
    /// `[ "+" / "-" ] 1*DIGIT`, already matched by the caller's grammar check.
    fn from_digits(sign_and_digits: &str) -> Option<Self> {
        let (negative, digits) = match sign_and_digits.as_bytes().first() {
            Some(b'-') => (true, &sign_and_digits[1..]),
            Some(b'+') => (false, &sign_and_digits[1..]),
            _ => (false, sign_and_digits),
        };
        let small = digits.bytes().try_fold(0_i64, |value, digit| {
            value.checked_mul(10)?.checked_add(i64::from(digit - b'0'))
        });
        match small {
            Some(value) if negative => Some(Self::Small(-value)),
            Some(value) => Some(Self::Small(value)),
            None => BigInt::from_digits(sign_and_digits).map(Self::from_big),
        }
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

    /// `self + amount`, exactly.
    fn offset(&self, amount: i128) -> Self {
        if let Self::Small(value) = self
            && let Some(sum) = i128::from(*value).checked_add(amount)
            && let Ok(sum) = i64::try_from(sum)
        {
            return Self::Small(sum);
        }
        let mut sum = self.to_big();
        sum.add_i128(amount);
        Self::from_big(sum)
    }

    const fn to_i64(&self) -> Option<i64> {
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

    /// `self − other`, exactly.
    fn difference(&self, other: &Self) -> BigInt {
        let mut difference = self.to_big();
        difference.add_assign(&other.to_big().negated());
        difference
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

/// The exact value of a JSON number: `±coefficient × 10^exponent`, with the
/// coefficient's leading and trailing zeros removed, or zero.
///
/// The form is canonical, so two lexemes that denote one number (`1`, `1.0`,
/// `10e-1`, `-0` and `0`) give equal values that hash alike, and `Ord` is the
/// numeric order.
///
/// ```rust
/// use purrdf_xsd::json_number::JsonNumber;
///
/// let number = |text| JsonNumber::parse(text).unwrap();
/// assert_eq!(number("1.0"), number("10e-1"));
/// assert!(number("0.0075").is_multiple_of(&number("0.0001")));
/// assert!(number("1e400").is_integer());
/// assert!(JsonNumber::parse("01").is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonNumber {
    negative: bool,
    /// The significant ASCII digits; `"0"` for zero.
    coefficient: String,
    /// `0` for zero.
    exponent: Exponent,
}

impl JsonNumber {
    /// The value of `lexeme`, or `None` when it is not an RFC 8259 §6
    /// `number = [ minus ] int [ frac ] [ exp ]`: a leading `+`, a leading zero
    /// before more digits, a bare `.`, an empty exponent, and anything else
    /// outside the grammar are refused.
    #[must_use]
    pub fn parse(lexeme: &str) -> Option<Self> {
        let bytes = lexeme.as_bytes();
        let (negative, body) = match bytes.first() {
            Some(b'-') => (true, &lexeme[1..]),
            _ => (false, lexeme),
        };
        let digits_from = |text: &str| text.bytes().take_while(u8::is_ascii_digit).count();
        let int_len = digits_from(body);
        // `int = zero / ( digit1-9 *DIGIT )`.
        if int_len == 0 || (int_len > 1 && body.as_bytes()[0] == b'0') {
            return None;
        }
        let (int, rest) = body.split_at(int_len);
        let (frac, rest) = match rest.strip_prefix('.') {
            Some(after) => {
                let frac_len = digits_from(after);
                if frac_len == 0 {
                    return None;
                }
                after.split_at(frac_len)
            }
            None => ("", rest),
        };
        let exponent = match rest.as_bytes().first() {
            None => Exponent::Small(0),
            Some(b'e' | b'E') => {
                // `exp = e [ minus / plus ] 1*DIGIT`.
                let after = &rest[1..];
                let signed = after.strip_prefix(['+', '-']).unwrap_or(after);
                if signed.is_empty() || digits_from(signed) != signed.len() {
                    return None;
                }
                Exponent::from_digits(after)?
            }
            Some(_) => return None,
        };
        let mut coefficient = String::with_capacity(int.len() + frac.len());
        coefficient.push_str(int);
        coefficient.push_str(frac);
        let leading = coefficient
            .bytes()
            .take_while(|&digit| digit == b'0')
            .count();
        if leading == coefficient.len() {
            return Some(Self::zero());
        }
        coefficient.drain(..leading);
        let significant = coefficient.trim_end_matches('0').len();
        let trailing = coefficient.len() - significant;
        coefficient.truncate(significant);
        // The value is `int frac × 10^(exponent − |frac|)`; each term of the
        // offset is a slice length, far inside `i128`.
        let shift = i128::try_from(trailing).unwrap_or(i128::MAX)
            - i128::try_from(frac.len()).unwrap_or(i128::MAX);
        Some(Self {
            negative,
            coefficient,
            exponent: exponent.offset(shift),
        })
    }

    fn zero() -> Self {
        Self {
            negative: false,
            coefficient: "0".to_owned(),
            exponent: Exponent::Small(0),
        }
    }

    /// Whether the value is zero.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.coefficient == "0"
    }

    /// Whether the value is an integer (`1.0` and `1e2` are).
    #[must_use]
    pub fn is_integer(&self) -> bool {
        self.exponent.is_nonnegative()
    }

    /// Whether the value is greater than zero.
    #[must_use]
    pub fn is_positive(&self) -> bool {
        !self.negative && !self.is_zero()
    }

    /// The value as a count: `None` for a negative number or a non-integer,
    /// and `u64::MAX` for an integer past it.
    #[must_use]
    pub fn to_u64_saturating(&self) -> Option<u64> {
        if self.negative || !self.is_integer() {
            return None;
        }
        let Some(exponent) = self.exponent.to_i64() else {
            return Some(u64::MAX);
        };
        let total = i64::try_from(self.coefficient.len())
            .ok()?
            .saturating_add(exponent);
        if total > 20 {
            return Some(u64::MAX);
        }
        let mut result = self
            .coefficient
            .bytes()
            .try_fold(0_u64, |value, digit| {
                value.checked_mul(10)?.checked_add(u64::from(digit - b'0'))
            })
            .unwrap_or(u64::MAX);
        for _ in 0..exponent {
            result = result.saturating_mul(10);
        }
        Some(result)
    }

    fn cmp_magnitude(&self, other: &Self) -> Ordering {
        match (self.is_zero(), other.is_zero()) {
            (true, true) => return Ordering::Equal,
            (true, false) => return Ordering::Less,
            (false, true) => return Ordering::Greater,
            (false, false) => {}
        }
        // The order of magnitude is `exponent + |coefficient|`; at one order,
        // digit strings without leading or trailing zeros compare as their
        // bytes do.
        let order = |number: &Self| {
            number
                .exponent
                .offset(i128::try_from(number.coefficient.len()).unwrap_or(i128::MAX))
        };
        order(self).cmp(&order(other)).then_with(|| {
            self.coefficient
                .as_bytes()
                .cmp(other.coefficient.as_bytes())
        })
    }

    /// Whether `self` is an integer multiple of `divisor`, exactly (JSON
    /// Schema `multipleOf`): `0.0075` is a multiple of `0.0001`, `1e308` of
    /// `0.5`. Zero is a multiple of everything, and nothing but zero is a
    /// multiple of zero.
    #[must_use]
    pub fn is_multiple_of(&self, divisor: &Self) -> bool {
        if self.is_zero() {
            return true;
        }
        if divisor.is_zero() || self.cmp_magnitude(divisor) == Ordering::Less {
            return false;
        }
        if self.exponent < divisor.exponent {
            // The divisor has at least one more factor of ten than we do, and
            // its coefficient has no trailing zero, so it cannot divide ours.
            return false;
        }
        // `self / divisor = (a / b) × 10^zeros`: an integer exactly when `b`
        // divides `a × 10^zeros`. The remainder has at most the divisor's
        // digits, however long the operands.
        let zeros = self.exponent.difference(&divisor.exponent);
        let (numerator, denominator) = (&self.coefficient, &divisor.coefficient);
        if let (Some(value), Some(modulus)) = (digits_u128(numerator), digits_u128(denominator)) {
            return mul_mod(value % modulus, pow_mod(10, zeros, modulus), modulus) == 0;
        }
        let modulus = BigInt::from_digits(denominator).expect("a coefficient is digits");
        let reduce = |value: BigInt| value.rem(&modulus).expect("a nonzero coefficient");
        let mut remainder =
            reduce(BigInt::from_digits(numerator).expect("a coefficient is digits"));
        // Repeated squaring: the work grows with log(zeros).
        let mut power = reduce(BigInt::from_i128(10));
        let mut exponent = zeros;
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

/// The value of an ASCII digit string, when it fits `u128`.
fn digits_u128(digits: &str) -> Option<u128> {
    digits.bytes().try_fold(0_u128, |value, digit| {
        value.checked_mul(10)?.checked_add(u128::from(digit - b'0'))
    })
}

const fn add_mod(left: u128, right: u128, modulus: u128) -> u128 {
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

fn pow_mod(base: u128, mut exponent: BigInt, modulus: u128) -> u128 {
    let mut result = 1 % modulus;
    let mut square = base % modulus;
    while !exponent.is_zero() {
        if exponent.is_odd() {
            result = mul_mod(result, square, modulus);
        }
        square = mul_mod(square, square, modulus);
        exponent = exponent.div_rem_u64(2).expect("a nonzero divisor").0;
    }
    result
}

impl Hash for JsonNumber {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // The form is canonical, so equal values have equal fields.
        self.negative.hash(state);
        self.coefficient.hash(state);
        self.exponent.hash(state);
    }
}

impl Ord for JsonNumber {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.negative, other.negative) {
            (false, true) => Ordering::Greater,
            (true, false) => Ordering::Less,
            (false, false) => self.cmp_magnitude(other),
            (true, true) => other.cmp_magnitude(self),
        }
    }
}

impl PartialOrd for JsonNumber {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// The order of two JSON number lexemes by the numbers they denote, exactly:
/// `1.0` and `10e-1` equal `1`, `-0` equals `0`, and no length of digits or
/// exponent loses precision. `None` when either side is not an RFC 8259 §6
/// `number` ([`JsonNumber::parse`]).
#[must_use]
pub fn cmp(left: &str, right: &str) -> Option<Ordering> {
    Some(JsonNumber::parse(left)?.cmp(&JsonNumber::parse(right)?))
}

#[cfg(test)]
mod tests {
    use super::{JsonNumber, cmp};
    use core::cmp::Ordering::{Equal, Greater, Less};

    fn number(text: &str) -> JsonNumber {
        JsonNumber::parse(text).expect("a JSON number")
    }

    #[test]
    fn equal_values_with_different_spellings_compare_equal() {
        assert_eq!(cmp("1", "1.0"), Some(Equal));
        assert_eq!(cmp("1", "10e-1"), Some(Equal));
        assert_eq!(cmp("100", "1E+2"), Some(Equal));
        assert_eq!(cmp("-0", "0.000e7"), Some(Equal));
        assert_eq!(
            cmp("1e999999999999999999999", "10e999999999999999999998"),
            Some(Equal)
        );
        assert_eq!(number("1e0000000000000000000000001"), number("10"));
    }

    #[test]
    fn close_values_are_ordered_exactly() {
        assert_eq!(cmp("1.0000000000000001", "1"), Some(Greater));
        assert_eq!(cmp("-1.0000000000000001", "-1"), Some(Less));
        assert_eq!(
            cmp("18446744073709551615", "18446744073709551616"),
            Some(Less)
        );
        assert_eq!(
            cmp("-18446744073709551600", "-18446744073709551615"),
            Some(Greater)
        );
        assert_eq!(cmp("1e-999999999999999999999", "0"), Some(Greater));
        assert_eq!(cmp("-1e-999999999999999999999", "0"), Some(Less));
        assert_eq!(
            cmp("1e999999999999999999999", "1e999999999999999999998"),
            Some(Greater)
        );
        assert_eq!(
            cmp("1e-999999999999999999999", "1e-999999999999999999998"),
            Some(Less)
        );
        assert_eq!(cmp("0.0001", "0.001"), Some(Less));
        assert_eq!(cmp("-1e308", "1e-308"), Some(Less));
        assert_eq!(cmp("0.12", "0.123"), Some(Less));
        assert_eq!(cmp("0.13", "0.123"), Some(Greater));
    }

    #[test]
    fn integrality_is_decided_by_value_not_spelling() {
        assert!(number("12345678910111213141516171819202122232425262728293031").is_integer());
        assert!(number("1.0").is_integer());
        assert!(number("1e400").is_integer());
        assert!(!number("1.5").is_integer());
        assert!(!number("1e-400").is_integer());
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
            ("0", "0", true),
            ("1", "0", false),
            (
                "123456789012345678901234567890123456789012345678901234567890",
                "3",
                true,
            ),
            (
                "1234567890123456789012345678901234567890e5",
                "12345678901234567890123456789012345678901",
                false,
            ),
        ] {
            assert_eq!(
                number(value).is_multiple_of(&number(divisor)),
                expected,
                "{value}/{divisor}"
            );
        }
    }

    #[test]
    fn counts_saturate() {
        assert_eq!(number("3.0").to_u64_saturating(), Some(3));
        assert_eq!(number("30e-1").to_u64_saturating(), Some(3));
        assert_eq!(number("1e30").to_u64_saturating(), Some(u64::MAX));
        assert_eq!(
            number("1e999999999999999999999").to_u64_saturating(),
            Some(u64::MAX)
        );
        assert_eq!(number("3.5").to_u64_saturating(), None);
        assert_eq!(number("-3").to_u64_saturating(), None);
    }

    #[test]
    fn equal_values_hash_alike() {
        use core::hash::{BuildHasher, BuildHasherDefault};
        let hash = |text: &str| {
            BuildHasherDefault::<purrdf_hash::fixed::FixedHasher>::default().hash_one(number(text))
        };
        assert_eq!(hash("1"), hash("1.0"));
        assert_eq!(hash("-0"), hash("0e5"));
        assert_ne!(hash("1"), hash("-1"));
    }

    #[test]
    fn a_leading_plus_is_refused_and_the_unsigned_spelling_accepted() {
        assert_eq!(cmp("+1", "1"), None);
        assert_eq!(cmp("1", "1"), Some(Equal));
    }

    #[test]
    fn a_leading_zero_before_digits_is_refused_and_a_lone_zero_accepted() {
        assert_eq!(cmp("01", "1"), None);
        assert_eq!(cmp("0", "0.5"), Some(Less));
    }

    #[test]
    fn a_bare_point_is_refused_and_a_digit_after_it_accepted() {
        assert_eq!(cmp("1.", "1"), None);
        assert_eq!(cmp(".5", "1"), None);
        assert_eq!(cmp("1.5", "1"), Some(Greater));
    }

    #[test]
    fn an_empty_exponent_is_refused_and_a_signed_one_accepted() {
        assert_eq!(cmp("1e", "1"), None);
        assert_eq!(cmp("1e+", "1"), None);
        assert_eq!(cmp("1e+-1", "1"), None);
        assert_eq!(cmp("1e-0", "1"), Some(Equal));
    }

    #[test]
    fn text_outside_the_grammar_is_refused_and_a_number_accepted() {
        for text in [
            "", "-", "NaN", "Infinity", "1 ", " 1", "0x10", "1_000", "1e1.5", "1-2+e",
        ] {
            assert_eq!(cmp(text, "1"), None, "{text:?}");
        }
        assert_eq!(cmp("-12.5e3", "-12500"), Some(Equal));
    }
}
