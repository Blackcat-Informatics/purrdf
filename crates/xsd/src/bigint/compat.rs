// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Numeric-tower adapters over the single binary integer representation.

use super::{BigInt, Mag};
use std::ops::{Add, Mul, Neg, Sub};

/// Balanced binary products switch to Karatsuba at this limb count.
pub const KARATSUBA_THRESHOLD: usize = 32;

impl BigInt {
    /// The number of binary magnitude limbs, zero for zero.
    #[must_use]
    pub fn limb_len(&self) -> usize {
        self.magnitude.len()
    }

    /// Most significant binary limb, zero for zero.
    pub(crate) fn leading_u64(&self) -> u64 {
        self.magnitude.last().copied().unwrap_or(0)
    }

    /// Add an exact integer into this value.
    pub fn add_assign(&mut self, other: &Self) {
        *self = self.add(other);
    }

    /// Add a machine integer without narrowing the result.
    pub fn add_i128(&mut self, value: i128) {
        self.add_assign(&Self::from_i128(value));
    }

    /// Narrow only when the exact value fits.
    #[must_use]
    pub fn to_i64(&self) -> Option<i64> {
        self.to_i128().and_then(|value| i64::try_from(value).ok())
    }

    /// Canonical signed decimal digits.
    #[must_use]
    pub fn to_decimal_string(&self) -> String {
        self.to_string()
    }

    /// Exact count of decimal digits in the magnitude, one for zero.
    #[must_use]
    pub fn decimal_digits(&self) -> u64 {
        self.to_string().trim_start_matches('-').len() as u64
    }

    /// Exact number of factors of ten, zero for zero.
    #[must_use]
    pub fn trailing_decimal_zeros(&self) -> u64 {
        let Some(limit) = self.trailing_zeros() else {
            return 0;
        };
        if let Ok(limit) = u32::try_from(limit) {
            let (_, removed) = self
                .strip_fives_using(limit, &super::Unbounded)
                .expect("unbounded integer storage");
            u64::from(removed)
        } else {
            self.strip_fives_wide_using(limit, &super::Unbounded)
                .expect("unbounded integer storage")
                .1
        }
    }

    /// Parse an optionally signed decimal integer.
    #[must_use]
    pub fn from_digits(text: &str) -> Option<Self> {
        let (negative, digits) = match text.as_bytes().first() {
            Some(b'-') => (true, &text[1..]),
            Some(b'+') => (false, &text[1..]),
            _ => (false, text),
        };
        let value = Self::from_decimal_digits(digits)?;
        Some(if negative { value.negated() } else { value })
    }

    /// Exact fixed-point decimal spelling, with insignificant zeros removed.
    #[must_use]
    pub fn to_decimal_lexical(&self, scale: u32) -> String {
        let digits = self.abs().to_string();
        let scale = scale as usize;
        let (integer, fraction) = if scale == 0 {
            (digits, String::new())
        } else if digits.len() > scale {
            let split = digits.len() - scale;
            (digits[..split].to_owned(), digits[split..].to_owned())
        } else {
            (
                "0".to_owned(),
                format!("{}{digits}", "0".repeat(scale - digits.len())),
            )
        };
        let fraction = fraction.trim_end_matches('0');
        let sign = if self.is_negative() { "-" } else { "" };
        if fraction.is_empty() {
            format!("{sign}{integer}")
        } else {
            format!("{sign}{integer}.{fraction}")
        }
    }

    /// Exact finite decimal representation of a dyadic value.
    #[must_use]
    pub fn from_binary(numerator: i128, exponent: i32) -> (Self, u32) {
        let value = Self::from_i128(numerator);
        if exponent >= 0 {
            (value.mul_pow2(exponent.unsigned_abs()), 0)
        } else {
            (
                value.mul_pow5(exponent.unsigned_abs()),
                exponent.unsigned_abs(),
            )
        }
    }

    /// Correctly rounded binary64 through the tower's one rounding home.
    #[must_use]
    pub fn to_f64(&self) -> f64 {
        if self.bit_len() >= 1025 {
            return if self.is_negative() {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            };
        }
        f64::from_bits(crate::exact::binary::round_ratio(
            self.is_negative(),
            &self.abs(),
            &Self::one(),
            &crate::exact::binary::BINARY64,
        ))
    }

    /// Correctly rounded binary32 without a binary64 intermediate.
    #[must_use]
    pub fn to_f32(&self) -> f32 {
        if self.bit_len() >= 129 {
            return if self.is_negative() {
                f32::NEG_INFINITY
            } else {
                f32::INFINITY
            };
        }
        f32::from_bits(crate::exact::binary::round_ratio(
            self.is_negative(),
            &self.abs(),
            &Self::one(),
            &crate::exact::binary::BINARY32,
        ) as u32)
    }

    /// Exact remainder with the dividend's sign.
    #[must_use]
    pub fn rem(&self, divisor: &Self) -> Option<Self> {
        self.div_rem(divisor).map(|(_, remainder)| remainder)
    }

    /// Exact quotient and remainder of a decimal shift.
    #[must_use]
    pub fn div_rem_pow10(&self, exponent: u64) -> (Self, Self) {
        if exponent == 0 {
            return (self.clone(), Self::zero());
        }
        if exponent >= self.decimal_digits() {
            return (Self::zero(), self.clone());
        }
        self.div_rem(&Self::pow10(
            u32::try_from(exponent).expect("decimal exponent fits u32"),
        ))
        .expect("power of ten is nonzero")
    }

    /// Square-and-multiply exact power, including zero to power zero.
    #[must_use]
    pub fn pow(&self, mut exponent: u32) -> Self {
        let mut result = Self::one();
        let mut factor = self.clone();
        while exponent != 0 {
            if exponent & 1 != 0 {
                result = result.mul_fast(&factor);
            }
            exponent >>= 1;
            if exponent != 0 {
                factor = factor.mul_fast(&factor);
            }
        }
        // Recombination may retain geometric spare capacity. A completed power
        // keeps exactly its logical limbs, matching its result-size admission.
        // Moving through a boxed slice guarantees capacity equals length;
        // inline values remain allocation-free, and shared/pooled inputs stay
        // in their original storage when no multiplication produced a heap.
        if let Mag::Heap(words) = &mut result.magnitude {
            *words = std::mem::take(words).into_boxed_slice().into_vec();
        }
        result
    }

    /// Exact subtraction.
    #[must_use]
    pub fn minus(&self, other: &Self) -> Self {
        self.sub(other)
    }

    /// Exact addition.
    #[must_use]
    pub fn plus(&self, other: &Self) -> Self {
        self.add(other)
    }

    /// Schoolbook small products and recursive Karatsuba large products.
    #[must_use]
    pub fn mul_fast(&self, other: &Self) -> Self {
        if self.limb_len().min(other.limb_len()) < KARATSUBA_THRESHOLD {
            return self
                .mul_using(other, &super::Unbounded)
                .expect("unbounded integer storage");
        }
        let split = self.limb_len().max(other.limb_len()) / 2;
        let (long, short) = if self.limb_len() >= other.limb_len() {
            (self, other)
        } else {
            (other, self)
        };
        if short.limb_len() <= split {
            let mut output = Mag::zeroed(long.limb_len() + short.limb_len() + 1);
            for (piece_index, words) in long.magnitude.chunks(short.limb_len()).enumerate() {
                let piece = Self::from_parts(false, Mag::from_slice(words)).mul_fast(&short.abs());
                let offset = piece_index * short.limb_len();
                let mut carry = 0_u128;
                let mut at = offset;
                for &word in piece.magnitude.iter() {
                    let sum = u128::from(output[at]) + u128::from(word) + carry;
                    output[at] = sum as u64;
                    carry = sum >> 64;
                    at += 1;
                }
                while carry != 0 {
                    let sum = u128::from(output[at]) + carry;
                    output[at] = sum as u64;
                    carry = sum >> 64;
                    at += 1;
                }
            }
            super::mag_trim(&mut output);
            return Self::from_parts(self.is_negative() != other.is_negative(), output);
        }
        let parts = |value: &Self| {
            let middle = split.min(value.limb_len());
            (
                Self::from_parts(false, Mag::from_slice(&value.magnitude[..middle])),
                Self::from_parts(false, Mag::from_slice(&value.magnitude[middle..])),
            )
        };
        let (a0, a1) = parts(self);
        let (b0, b1) = parts(other);
        let z0 = a0.mul_fast(&b0);
        let z2 = a1.mul_fast(&b1);
        let z1 = a0.add(&a1).mul_fast(&b0.add(&b1)).sub(&z0).sub(&z2);
        let shift = u64::try_from(split)
            .expect("limb count fits u64")
            .checked_mul(64)
            .expect("product shift fits u64");
        let shifted_middle = Self::from_parts(
            false,
            super::mag_shl(&z1.magnitude, shift, &super::Unbounded)
                .expect("unbounded integer storage"),
        );
        let shifted_high = Self::from_parts(
            false,
            super::mag_shl(
                &z2.magnitude,
                shift.checked_mul(2).expect("product shift fits u64"),
                &super::Unbounded,
            )
            .expect("unbounded integer storage"),
        );
        let magnitude = z0.add(&shifted_middle).add(&shifted_high);
        if self.is_negative() != other.is_negative() {
            magnitude.negated()
        } else {
            magnitude
        }
    }
}

impl Add<&BigInt> for &BigInt {
    type Output = BigInt;
    fn add(self, other: &BigInt) -> BigInt {
        self.plus(other)
    }
}
impl Sub<&BigInt> for &BigInt {
    type Output = BigInt;
    fn sub(self, other: &BigInt) -> BigInt {
        self.minus(other)
    }
}
impl Mul<&BigInt> for &BigInt {
    type Output = BigInt;
    fn mul(self, other: &BigInt) -> BigInt {
        self.mul_fast(other)
    }
}
impl Neg for &BigInt {
    type Output = BigInt;
    fn neg(self) -> BigInt {
        self.negated()
    }
}
impl Neg for BigInt {
    type Output = Self;
    fn neg(self) -> Self {
        self.negated()
    }
}
