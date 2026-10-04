// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! An exact rational oracle for numeric conversions: the value a conversion should
//! produce, computed from the source's exact value with integer arithmetic only.
//!
//! A test of a float conversion against a float is a test of one rounding against
//! another, so this module never forms a float from anything but the exact rounding it
//! computes. A [`Rational`] holds `±numerator / denominator` over [`Natural`], an
//! unbounded unsigned integer. It is built from an integer, a decimal mantissa and
//! scale, a decimal numeral, or the exact binary value of an `f64`/`f32`, and rounded:
//!
//! * [`Rational::to_f64`] / [`Rational::to_f32`]: to nearest, ties to even, at the
//!   format's precision, subnormals included, overflowing to an infinity and keeping
//!   the sign of a zero (IEEE 754 `roundTiesToEven`, XSD 1.1 `floatingPointRound`);
//! * [`Rational::to_scale_ties_toward_zero`]: to the nearest multiple of `10^-scale`,
//!   ties toward zero (XPath F&O 3.1 §19.1.2.3's float-to-decimal cast).
//!
//! The arithmetic is schoolbook and slow by design: it has to be obviously right, not
//! fast, and it shares no code with any implementation it checks.

use std::cmp::Ordering;

/// An unbounded unsigned integer: little-endian base-2^32 limbs, no trailing zero
/// limb (zero is the empty vector).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Natural {
    limbs: Vec<u32>,
}

impl Natural {
    /// The natural number `value`.
    #[must_use]
    pub fn from_u128(mut value: u128) -> Self {
        let mut limbs = Vec::new();
        while value != 0 {
            limbs.push((value & 0xFFFF_FFFF) as u32);
            value >>= 32;
        }
        Self { limbs }
    }

    /// Whether this is zero.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.limbs.is_empty()
    }

    /// The number of significant bits (zero has none).
    #[must_use]
    pub fn bit_len(&self) -> u64 {
        self.limbs.last().map_or(0, |top| {
            (self.limbs.len() as u64 - 1) * 32 + u64::from(32 - top.leading_zeros())
        })
    }

    /// Whether the lowest bit is set.
    #[must_use]
    pub fn is_odd(&self) -> bool {
        self.limbs.first().is_some_and(|low| low & 1 == 1)
    }

    fn trim(mut self) -> Self {
        while self.limbs.last() == Some(&0) {
            self.limbs.pop();
        }
        self
    }

    /// `self × factor`.
    #[must_use]
    pub fn mul_small(&self, factor: u32) -> Self {
        let mut carry = 0_u64;
        let mut limbs = Vec::with_capacity(self.limbs.len() + 1);
        for &limb in &self.limbs {
            let product = u64::from(limb) * u64::from(factor) + carry;
            limbs.push((product & 0xFFFF_FFFF) as u32);
            carry = product >> 32;
        }
        limbs.push(carry as u32);
        Self { limbs }.trim()
    }

    /// `self × 2^shift`.
    #[must_use]
    pub fn shl(&self, shift: u64) -> Self {
        if self.is_zero() {
            return Self::default();
        }
        let whole = usize::try_from(shift / 32).expect("a shift that fits memory");
        let bits = (shift % 32) as u32;
        let mut limbs = vec![0_u32; whole];
        let mut carry = 0_u32;
        for &limb in &self.limbs {
            if bits == 0 {
                limbs.push(limb);
            } else {
                limbs.push((limb << bits) | carry);
                carry = limb >> (32 - bits);
            }
        }
        limbs.push(carry);
        Self { limbs }.trim()
    }

    /// `self × 10^exponent`.
    #[must_use]
    pub fn mul_pow10(&self, exponent: u32) -> Self {
        (0..exponent).fold(self.clone(), |value, _| value.mul_small(10))
    }

    /// `self + other`.
    #[must_use]
    pub fn add(&self, other: &Self) -> Self {
        let mut limbs = Vec::with_capacity(self.limbs.len().max(other.limbs.len()) + 1);
        let mut carry = 0_u64;
        for index in 0..self.limbs.len().max(other.limbs.len()) {
            let sum = u64::from(self.limbs.get(index).copied().unwrap_or(0))
                + u64::from(other.limbs.get(index).copied().unwrap_or(0))
                + carry;
            limbs.push((sum & 0xFFFF_FFFF) as u32);
            carry = sum >> 32;
        }
        limbs.push(carry as u32);
        Self { limbs }.trim()
    }

    /// `self - other`, which must not be negative.
    ///
    /// # Panics
    ///
    /// When `other > self`.
    #[must_use]
    pub fn sub(&self, other: &Self) -> Self {
        assert!(*self >= *other, "a natural difference is not negative");
        let mut limbs = Vec::with_capacity(self.limbs.len());
        let mut borrow = 0_i64;
        for (index, &limb) in self.limbs.iter().enumerate() {
            let mut difference =
                i64::from(limb) - i64::from(other.limbs.get(index).copied().unwrap_or(0)) - borrow;
            borrow = i64::from(difference < 0);
            if difference < 0 {
                difference += 1 << 32;
            }
            limbs.push(u32::try_from(difference).expect("a limb"));
        }
        Self { limbs }.trim()
    }

    /// `self × other`.
    #[must_use]
    pub fn mul(&self, other: &Self) -> Self {
        let mut limbs = vec![0_u32; self.limbs.len() + other.limbs.len() + 1];
        for (i, &a) in self.limbs.iter().enumerate() {
            let mut carry = 0_u64;
            for (j, &b) in other.limbs.iter().enumerate() {
                let cell = u64::from(limbs[i + j]) + u64::from(a) * u64::from(b) + carry;
                limbs[i + j] = (cell & 0xFFFF_FFFF) as u32;
                carry = cell >> 32;
            }
            let mut at = i + other.limbs.len();
            while carry != 0 {
                let cell = u64::from(limbs[at]) + carry;
                limbs[at] = (cell & 0xFFFF_FFFF) as u32;
                carry = cell >> 32;
                at += 1;
            }
        }
        Self { limbs }.trim()
    }

    /// `(⌊self / divisor⌋, self mod divisor)` by binary long division.
    ///
    /// # Panics
    ///
    /// When `divisor` is zero.
    #[must_use]
    pub fn div_rem(&self, divisor: &Self) -> (Self, Self) {
        assert!(!divisor.is_zero(), "division by zero");
        let mut quotient = Self::default();
        let mut remainder = Self::default();
        for bit in (0..self.bit_len()).rev() {
            remainder = remainder.shl(1);
            if self.bit(bit) {
                remainder = remainder.add(&Self::from_u128(1));
            }
            quotient = quotient.shl(1);
            if remainder >= *divisor {
                remainder = remainder.sub(divisor);
                quotient = quotient.add(&Self::from_u128(1));
            }
        }
        (quotient, remainder)
    }

    fn bit(&self, index: u64) -> bool {
        usize::try_from(index / 32)
            .ok()
            .and_then(|limb| self.limbs.get(limb))
            .is_some_and(|limb| (limb >> (index % 32)) & 1 == 1)
    }

    /// The value, when it fits a `u128`.
    #[must_use]
    pub fn to_u128(&self) -> Option<u128> {
        if self.limbs.len() > 4 {
            return None;
        }
        Some(
            self.limbs
                .iter()
                .rev()
                .fold(0_u128, |value, &limb| (value << 32) | u128::from(limb)),
        )
    }
}

impl Ord for Natural {
    fn cmp(&self, other: &Self) -> Ordering {
        self.limbs
            .len()
            .cmp(&other.limbs.len())
            .then_with(|| self.limbs.iter().rev().cmp(other.limbs.iter().rev()))
    }
}

impl PartialOrd for Natural {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// An exact rational `±numerator / denominator` (denominator never zero). Zero keeps
/// the sign it was built with, so a negative zero rounds to `-0.0`.
#[derive(Clone, Debug)]
pub struct Rational {
    negative: bool,
    numerator: Natural,
    denominator: Natural,
}

/// The parameters of an IEEE 754 binary format that rounding needs.
struct Binary {
    /// Significand precision in bits, the hidden bit included (53, 24).
    precision: u32,
    /// The smallest normal exponent (-1022, -126).
    min_exponent: i64,
    /// The largest finite exponent (1023, 127).
    max_exponent: i64,
}

const BINARY64: Binary = Binary {
    precision: 53,
    min_exponent: -1022,
    max_exponent: 1023,
};

const BINARY32: Binary = Binary {
    precision: 24,
    min_exponent: -126,
    max_exponent: 127,
};

/// A rounded binary value: `±significand × 2^exponent`, or an infinity.
enum Rounded {
    Finite {
        negative: bool,
        significand: u64,
        exponent: i64,
    },
    Infinite {
        negative: bool,
    },
}

impl Rational {
    /// `±numerator / denominator`.
    ///
    /// # Panics
    ///
    /// When `denominator` is zero.
    #[must_use]
    pub fn new(negative: bool, numerator: Natural, denominator: Natural) -> Self {
        assert!(
            !denominator.is_zero(),
            "a rational has a nonzero denominator"
        );
        Self {
            negative,
            numerator,
            denominator,
        }
    }

    /// The integer `value`.
    #[must_use]
    pub fn from_i128(value: i128) -> Self {
        Self::new(
            value < 0,
            Natural::from_u128(value.unsigned_abs()),
            Natural::from_u128(1),
        )
    }

    /// The decimal `mantissa × 10^-scale`.
    #[must_use]
    pub fn from_decimal(mantissa: i128, scale: u32) -> Self {
        Self::new(
            mantissa < 0,
            Natural::from_u128(mantissa.unsigned_abs()),
            Natural::from_u128(1).mul_pow10(scale),
        )
    }

    /// The exact value of a decimal numeral: an optional sign, digits with at most one
    /// point, and an optional `e`/`E` exponent. `None` for anything else (including
    /// the special values, which are not rationals).
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let (negative, body) = match text.as_bytes().first()? {
            b'-' => (true, &text[1..]),
            b'+' => (false, &text[1..]),
            _ => (false, text),
        };
        let (number, exponent) = match body.find(['e', 'E']) {
            Some(at) => (&body[..at], body[at + 1..].parse::<i64>().ok()?),
            None => (body, 0),
        };
        let (whole, fraction) = number.split_once('.').unwrap_or((number, ""));
        if whole.is_empty() && fraction.is_empty() {
            return None;
        }
        let mut digits = Natural::default();
        for byte in whole.bytes().chain(fraction.bytes()) {
            if !byte.is_ascii_digit() {
                return None;
            }
            digits = digits
                .mul_small(10)
                .add(&Natural::from_u128(u128::from(byte - b'0')));
        }
        let shift = exponent - i64::try_from(fraction.len()).ok()?;
        let power = u32::try_from(shift.unsigned_abs()).ok()?;
        Some(if shift >= 0 {
            Self::new(negative, digits.mul_pow10(power), Natural::from_u128(1))
        } else {
            Self::new(negative, digits, Natural::from_u128(1).mul_pow10(power))
        })
    }

    /// The exact value of a finite `f64`.
    ///
    /// # Panics
    ///
    /// When `value` is `NaN` or infinite.
    #[must_use]
    pub fn from_f64(value: f64) -> Self {
        assert!(value.is_finite(), "a finite double");
        let bits = value.to_bits();
        let biased = i64::try_from((bits >> 52) & 0x7FF).expect("an exponent");
        let fraction = bits & ((1 << 52) - 1);
        let (significand, exponent) = if biased == 0 {
            (fraction, -1074)
        } else {
            (fraction | (1 << 52), biased - 1075)
        };
        Self::binary(bits >> 63 == 1, significand, exponent)
    }

    /// The exact value of a finite `f32`.
    ///
    /// # Panics
    ///
    /// When `value` is `NaN` or infinite.
    #[must_use]
    pub fn from_f32(value: f32) -> Self {
        assert!(value.is_finite(), "a finite float");
        let bits = value.to_bits();
        let biased = i64::from((bits >> 23) & 0xFF);
        let fraction = u64::from(bits & ((1 << 23) - 1));
        let (significand, exponent) = if biased == 0 {
            (fraction, -149)
        } else {
            (fraction | (1 << 23), biased - 150)
        };
        Self::binary(bits >> 31 == 1, significand, exponent)
    }

    fn binary(negative: bool, significand: u64, exponent: i64) -> Self {
        let significand = Natural::from_u128(u128::from(significand));
        if exponent >= 0 {
            Self::new(
                negative,
                significand.shl(exponent.unsigned_abs()),
                Natural::from_u128(1),
            )
        } else {
            Self::new(
                negative,
                significand,
                Natural::from_u128(1).shl(exponent.unsigned_abs()),
            )
        }
    }

    /// Whether this is negative (a negative zero included).
    #[must_use]
    pub const fn is_negative(&self) -> bool {
        self.negative
    }

    /// Whether the two values are equal (the signs of zeros aside).
    #[must_use]
    pub fn value_eq(&self, other: &Self) -> bool {
        if self.numerator.is_zero() && other.numerator.is_zero() {
            return true;
        }
        // a/b = c/d exactly when a·d = c·b.
        let left = self.numerator.mul(&other.denominator);
        let right = other.numerator.mul(&self.denominator);
        self.negative == other.negative && left == right
    }

    /// The magnitude compared with `2^exponent`.
    fn cmp_pow2(&self, exponent: i64) -> Ordering {
        if exponent >= 0 {
            self.numerator
                .cmp(&self.denominator.shl(exponent.unsigned_abs()))
        } else {
            self.numerator
                .shl(exponent.unsigned_abs())
                .cmp(&self.denominator)
        }
    }

    /// The magnitude compared with `limit`.
    #[must_use]
    pub fn cmp_magnitude(&self, limit: &Self) -> Ordering {
        self.numerator
            .mul(&limit.denominator)
            .cmp(&limit.numerator.mul(&self.denominator))
    }

    /// Round to `format` to nearest, ties to even.
    fn round_binary(&self, format: &Binary) -> Rounded {
        let negative = self.negative;
        if self.numerator.is_zero() {
            return Rounded::Finite {
                negative,
                significand: 0,
                exponent: 0,
            };
        }
        // The binade: 2^e ≤ |x| < 2^(e+1), found from the bit lengths and fixed up.
        let mut binade = i64::try_from(self.numerator.bit_len()).expect("bits")
            - i64::try_from(self.denominator.bit_len()).expect("bits");
        while self.cmp_pow2(binade) == Ordering::Less {
            binade -= 1;
        }
        while self.cmp_pow2(binade + 1) != Ordering::Less {
            binade += 1;
        }
        // The quantum 2^q of the target grid at this binade (fixed below the normal
        // range: the subnormal grid).
        let quantum = binade.max(format.min_exponent) - i64::from(format.precision) + 1;
        // significand = |x| / 2^q = A / B, rounded to an integer.
        let (scaled, divisor) = if quantum >= 0 {
            (
                self.numerator.clone(),
                self.denominator.shl(quantum.unsigned_abs()),
            )
        } else {
            (
                self.numerator.shl(quantum.unsigned_abs()),
                self.denominator.clone(),
            )
        };
        let (floor, remainder) = scaled.div_rem(&divisor);
        let twice = remainder.shl(1);
        let up = match twice.cmp(&divisor) {
            Ordering::Greater => true,
            Ordering::Equal => floor.is_odd(),
            Ordering::Less => false,
        };
        let mut significand = floor
            .add(&Natural::from_u128(u128::from(up)))
            .to_u128()
            .and_then(|value| u64::try_from(value).ok())
            .expect("a significand of at most precision + 1 bits");
        let mut exponent = quantum;
        if significand == 1 << format.precision {
            significand >>= 1;
            exponent += 1;
        }
        if exponent + i64::from(format.precision) - 1 > format.max_exponent {
            return Rounded::Infinite { negative };
        }
        Rounded::Finite {
            negative,
            significand,
            exponent,
        }
    }

    /// The `f64` nearest this value, ties to even.
    #[must_use]
    pub fn to_f64(&self) -> f64 {
        match self.round_binary(&BINARY64) {
            Rounded::Infinite { negative } => {
                if negative {
                    f64::NEG_INFINITY
                } else {
                    f64::INFINITY
                }
            }
            Rounded::Finite {
                negative,
                significand,
                exponent,
            } => {
                let sign = u64::from(negative) << 63;
                if significand == 0 {
                    return f64::from_bits(sign);
                }
                let bits = if significand < 1 << 52 {
                    // Subnormal: exponent is the subnormal quantum, -1074.
                    significand
                } else {
                    let biased = u64::try_from(exponent + 52 + 1023).expect("a normal exponent");
                    (biased << 52) | (significand & ((1 << 52) - 1))
                };
                f64::from_bits(sign | bits)
            }
        }
    }

    /// The `f32` nearest this value, ties to even — rounded once, never via `f64`.
    #[must_use]
    pub fn to_f32(&self) -> f32 {
        match self.round_binary(&BINARY32) {
            Rounded::Infinite { negative } => {
                if negative {
                    f32::NEG_INFINITY
                } else {
                    f32::INFINITY
                }
            }
            Rounded::Finite {
                negative,
                significand,
                exponent,
            } => {
                let sign = u32::from(negative) << 31;
                let significand = u32::try_from(significand).expect("24 bits");
                if significand == 0 {
                    return f32::from_bits(sign);
                }
                let bits = if significand < 1 << 23 {
                    significand
                } else {
                    let biased = u32::try_from(exponent + 23 + 127).expect("a normal exponent");
                    (biased << 23) | (significand & ((1 << 23) - 1))
                };
                f32::from_bits(sign | bits)
            }
        }
    }

    /// The value with its fractional part discarded (rounded toward zero), as an
    /// `i128`, or `None` when that integer does not fit one.
    #[must_use]
    pub fn truncate_toward_zero(&self) -> Option<i128> {
        let (whole, _) = self.numerator.div_rem(&self.denominator);
        let magnitude = whole.to_u128()?;
        if self.negative {
            0_i128.checked_sub_unsigned(magnitude)
        } else {
            i128::try_from(magnitude).ok()
        }
    }

    /// The nearest multiple of `10^-scale`, ties toward zero, keeping this value's
    /// sign.
    #[must_use]
    pub fn round_to_scale_ties_toward_zero(&self, scale: u32) -> Self {
        let unit = Natural::from_u128(1).mul_pow10(scale);
        let (floor, remainder) = self.numerator.mul_pow10(scale).div_rem(&self.denominator);
        let up = remainder.shl(1) > self.denominator;
        let count = floor.add(&Natural::from_u128(u128::from(up)));
        Self::new(self.negative, count, unit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounding_matches_hand_computed_values() {
        // 1 + 2^-24 is a binary32 tie: even is 1.
        let tie = Rational::from_f64(1.0 + f64::from(2_f32.powi(-24)));
        assert_eq!(tie.to_f32().to_bits(), 1.0_f32.to_bits());
        // 1 + 3 × 2^-25 is above it.
        let above = Rational::parse("1.0000000894069671630859375").expect("a numeral");
        assert_eq!(above.to_f32(), 1.000_000_1);
        assert_eq!(Rational::parse("0.1").expect("0.1").to_f64(), 0.1);
        assert_eq!(Rational::parse("0.1").expect("0.1").to_f32(), 0.1_f32);
        // Halfway between the largest float and 2^128: ties to even overflow.
        let max_tie = Rational::from_f64(3.402_823_567_797_336_6e38);
        assert_eq!(max_tie.to_f32(), f32::INFINITY);
        // Half the smallest subnormal ties to zero; anything above it does not.
        assert_eq!(Rational::from_f64(2_f64.powi(-150)).to_f32().to_bits(), 0);
        assert_eq!(
            Rational::from_f64(f64::from_bits(2_f64.powi(-150).to_bits() + 1))
                .to_f32()
                .to_bits(),
            1
        );
        assert_eq!(
            Rational::from_f64(-0.0).to_f32().to_bits(),
            (-0.0_f32).to_bits()
        );
        assert_eq!(
            Rational::from_i128(i128::MAX).to_f64(),
            2_f64.powi(127),
            "i128::MAX rounds up to 2^127"
        );
        let tie = Rational::parse("0.0000057220458984375").expect("a numeral");
        assert!(
            tie.round_to_scale_ties_toward_zero(18)
                .value_eq(&Rational::from_decimal(5_722_045_898_437, 18))
        );
        let negative = Rational::parse("-0.0000057220458984375").expect("a numeral");
        assert!(
            negative
                .round_to_scale_ties_toward_zero(18)
                .value_eq(&Rational::from_decimal(-5_722_045_898_437, 18))
        );
        let above = Rational::parse("0.00000572204589843751").expect("a numeral");
        assert!(
            above
                .round_to_scale_ties_toward_zero(18)
                .value_eq(&Rational::from_decimal(5_722_045_898_438, 18))
        );
    }

    #[test]
    fn truncation_discards_the_fraction_toward_zero() {
        let cases = [
            ("2.9", Some(2)),
            ("-2.9", Some(-2)),
            ("-0.5", Some(0)),
            ("0.999", Some(0)),
            ("-170141183460469231731687303715884105728", Some(i128::MIN)),
            (
                "-170141183460469231731687303715884105728.9",
                Some(i128::MIN),
            ),
            ("-170141183460469231731687303715884105729", None),
            ("170141183460469231731687303715884105727.5", Some(i128::MAX)),
            ("170141183460469231731687303715884105728", None),
        ];
        for (numeral, expected) in cases {
            let value = Rational::parse(numeral).expect("a numeral");
            assert_eq!(value.truncate_toward_zero(), expected, "{numeral}");
        }
    }

    #[test]
    fn the_naturals_divide_exactly() {
        let a = Natural::from_u128(u128::MAX).mul(&Natural::from_u128(12_345));
        let (q, r) = a.div_rem(&Natural::from_u128(12_345));
        assert_eq!(q.to_u128(), Some(u128::MAX));
        assert!(r.is_zero());
        let (q, r) = Natural::from_u128(1_000_003).div_rem(&Natural::from_u128(1_000));
        assert_eq!((q.to_u128(), r.to_u128()), (Some(1_000), Some(3)));
    }
}
