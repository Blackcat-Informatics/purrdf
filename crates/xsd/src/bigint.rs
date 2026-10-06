// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact arbitrary-precision accumulation over the shared binary integer home.
//!
//! XSD individual integer literals remain bounded by `i128`; running totals,
//! exact decimal comparison and certified numerical evaluation use this unbounded
//! value. Its binary magnitude has three inline limbs and a vector beyond them.
//! This compatibility facade shares every arithmetic body with [`Int`].

use crate::{
    ieee::ratio::{self, Rounding},
    integer::{Int, LimbScratch, LimbScratchError},
};
use core::fmt;

/// Exact signed integer for XSD accumulation and certified arithmetic.
#[derive(Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BigInt(Int);

impl BigInt {
    /// Canonical zero.
    #[must_use]
    pub const fn zero() -> Self {
        Self(Int::zero())
    }

    /// Whether this value is exactly zero.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    /// Every signed machine integer, including `i128::MIN`, exactly.
    #[must_use]
    pub fn from_i128(value: i128) -> Self {
        Self(Int::from_i128(value))
    }

    /// Every unsigned machine magnitude exactly.
    #[must_use]
    pub fn from_u128(value: u128) -> Self {
        Self(Int::from_u128(value))
    }

    /// Borrow the shared binary integer without conversion or allocation.
    #[must_use]
    pub const fn as_integer(&self) -> &Int {
        &self.0
    }

    /// Move the shared binary integer without conversion or allocation.
    #[must_use]
    pub fn into_integer(self) -> Int {
        self.0
    }

    /// Exact in-place addition.
    pub fn add_assign(&mut self, other: &Self) {
        self.0 = self.0.add(&other.0);
    }

    /// Add another parsed machine integer into an unbounded total.
    pub fn add_i128(&mut self, value: i128) {
        self.add_assign(&Self::from_i128(value));
    }

    /// Narrow only when the exact value fits.
    #[must_use]
    pub fn to_i128(&self) -> Option<i128> {
        self.0.to_i128()
    }

    /// Correctly half-even rounded binary64, formed entirely from integer bits.
    /// Zero is positive; overflow is signed infinity.
    #[must_use]
    pub fn to_f64(&self) -> f64 {
        ratio::to_binary64(&self.0, &Int::one(), Rounding::NearestEven).expect("unit denominator")
    }

    /// Correctly half-even rounded binary32 without double rounding.
    #[must_use]
    pub fn to_f32(&self) -> f32 {
        ratio::to_binary32(&self.0, &Int::one(), Rounding::NearestEven).expect("unit denominator")
    }

    /// XSD canonical integer spelling: no leading zeros or signed zero.
    #[must_use]
    pub fn to_decimal_string(&self) -> String {
        self.0.to_string()
    }

    /// Canonical decimal spelling of the mantissa divided by `10^scale`.
    /// Integer-valued results omit the decimal point; trailing fraction zeros
    /// are suppressed, matching the established XSD decimal boundary.
    #[must_use]
    pub fn to_decimal_lexical(&self, scale: u32) -> String {
        let digits = self.0.abs().to_string();
        let scale = usize::try_from(scale).unwrap_or(usize::MAX);
        let (int_part, frac_part) = if scale == 0 {
            (digits, String::new())
        } else if digits.len() > scale {
            let split = digits.len() - scale;
            (digits[..split].to_string(), digits[split..].to_string())
        } else {
            let pad = "0".repeat(scale - digits.len());
            ("0".to_string(), format!("{pad}{digits}"))
        };
        let frac_trimmed = frac_part.trim_end_matches('0');
        let sign = if self.is_negative() { "-" } else { "" };
        if frac_trimmed.is_empty() {
            format!("{sign}{int_part}")
        } else {
            format!("{sign}{int_part}.{frac_trimmed}")
        }
    }

    /// Whether this value is strictly negative.
    #[must_use]
    pub fn is_negative(&self) -> bool {
        self.0.is_negative()
    }

    /// Multiply by an exact power of ten.
    #[must_use]
    pub fn mul_pow10(&self, exp: u32) -> Self {
        Self(self.0.mul(&Int::pow10(exp)))
    }

    /// Multiply by an exact power of two.
    #[must_use]
    pub fn mul_pow2(&self, exp: u32) -> Self {
        Self(self.0.shl(exp))
    }

    /// Truncated quotient and unsigned remainder of a positive machine divisor.
    /// Zero divisor returns `None`.
    #[must_use]
    pub fn div_rem_u64(&self, divisor: u64) -> Option<(Self, u64)> {
        let (quotient, remainder) = self.0.div_rem(&Int::from_u64(divisor))?;
        Some((Self(quotient), remainder.unsigned_abs_u128()? as u64))
    }

    /// Parse `[+-]?DIGIT+`; leading zeros are allowed and signed zero is zero.
    #[must_use]
    pub fn from_digits(text: &str) -> Option<Self> {
        let (negative, digits) = match text.as_bytes().first() {
            Some(b'-') => (true, &text[1..]),
            Some(b'+') => (false, &text[1..]),
            _ => (false, text),
        };
        let magnitude = Int::from_decimal_digits(digits)?;
        Some(Self(if negative { magnitude.neg() } else { magnitude }))
    }

    /// Exact dyadic as decimal mantissa and scale: `m * 2^e = n / 10^scale`.
    #[must_use]
    pub fn from_binary(numerator: i128, exponent: i32) -> (Self, u32) {
        let value = Self::from_i128(numerator);
        if exponent >= 0 {
            (value.mul_pow2(exponent.unsigned_abs()), 0)
        } else {
            let places = exponent.unsigned_abs();
            (value.mul_pow5(places), places)
        }
    }

    /// Exact multiplication by a machine factor.
    #[must_use]
    pub fn mul_small(&self, factor: u32) -> Self {
        Self(self.0.mul(&Int::from_u64(u64::from(factor))))
    }

    /// Multiply by an exact power of five.
    #[must_use]
    pub fn mul_pow5(&self, exp: u32) -> Self {
        Self(self.0.mul_pow5(exp))
    }

    /// Exact multiplication.
    #[must_use]
    pub fn mul(&self, other: &Self) -> Self {
        Self(self.0.mul(&other.0))
    }

    /// Truncated remainder, or `None` for zero divisor.
    #[must_use]
    pub fn rem(&self, divisor: &Self) -> Option<Self> {
        self.0
            .div_rem(&divisor.0)
            .map(|(_, remainder)| Self(remainder))
    }

    /// Truncated exact quotient and remainder, or `None` for zero divisor.
    #[must_use]
    pub fn div_rem(&self, divisor: &Self) -> Option<(Self, Self)> {
        self.0
            .div_rem(&divisor.0)
            .map(|(quotient, remainder)| (Self(quotient), Self(remainder)))
    }

    /// Exact sum.
    #[must_use]
    pub fn add(&self, other: &Self) -> Self {
        Self(self.0.add(&other.0))
    }

    /// Exact difference.
    #[must_use]
    pub fn sub(&self, other: &Self) -> Self {
        Self(self.0.sub(&other.0))
    }

    /// Absolute value.
    #[must_use]
    pub fn abs(&self) -> Self {
        Self(self.0.abs())
    }

    /// Exact bit length, also a conservative numerical workspace bound.
    #[must_use]
    pub fn bits_upper_bound(&self) -> usize {
        usize::try_from(self.0.bit_len()).unwrap_or(usize::MAX)
    }

    /// Current owned heap bytes; inline storage is excluded.
    #[must_use]
    pub fn allocated_bytes(&self) -> usize {
        self.0.allocated_bytes()
    }

    /// Detach immutable limbs from a reusable arena for admitted preparation.
    #[must_use]
    pub fn detached(&self) -> Self {
        Self(self.0.detached())
    }

    /// Actual canonical limb bytes retained by that detached copy.
    #[must_use]
    pub fn detached_heap_bytes(&self) -> usize {
        self.0.detached_heap_bytes()
    }

    /// Integer floor square root, or `None` when negative.
    #[must_use]
    pub fn sqrt_floor(&self) -> Option<Self> {
        self.0.sqrt_floor().map(Self)
    }

    /// Whether the value is odd.
    #[must_use]
    pub fn is_odd(&self) -> bool {
        self.0.is_odd()
    }

    /// Additive inverse, preserving canonical zero.
    #[must_use]
    pub fn negated(&self) -> Self {
        Self(self.0.neg())
    }

    pub(crate) fn has_unbounded_limb_storage(&self) -> bool {
        self.0.has_unbounded_limb_storage()
    }

    /// The same exact operation with bounded reusable limb storage.
    /// # Errors
    /// Refuses scratch limb/destination exhaustion without allocating.
    pub fn copy_in(&self, scratch: &LimbScratch) -> Result<Self, LimbScratchError> {
        self.0.copy_in(scratch).map(Self)
    }
    /// The same exact operation with bounded reusable limb storage.
    /// # Errors
    /// Refuses scratch limb/destination exhaustion without allocating.
    pub fn add_in(&self, other: &Self, scratch: &LimbScratch) -> Result<Self, LimbScratchError> {
        self.0.add_in(&other.0, scratch).map(Self)
    }
    /// The same exact operation with bounded reusable limb storage.
    /// # Errors
    /// Refuses scratch limb/destination exhaustion without allocating.
    pub fn sub_in(&self, other: &Self, scratch: &LimbScratch) -> Result<Self, LimbScratchError> {
        self.0.sub_in(&other.0, scratch).map(Self)
    }
    /// The same exact operation with bounded reusable limb storage.
    /// # Errors
    /// Refuses scratch limb/destination exhaustion without allocating.
    pub fn mul_in(&self, other: &Self, scratch: &LimbScratch) -> Result<Self, LimbScratchError> {
        self.0.mul_in(&other.0, scratch).map(Self)
    }
    /// Truncated exact quotient/remainder with bounded reusable storage.
    /// # Errors
    /// Refuses scratch exhaustion; a zero divisor returns `Ok(None)`.
    pub fn div_rem_in(
        &self,
        other: &Self,
        scratch: &LimbScratch,
    ) -> Result<Option<(Self, Self)>, LimbScratchError> {
        Ok(self
            .0
            .div_rem_in(&other.0, scratch)?
            .map(|(q, r)| (Self(q), Self(r))))
    }
    /// Exact power-of-two product with bounded reusable storage.
    /// # Errors
    /// Refuses scratch limb/destination exhaustion without allocating.
    pub fn mul_pow2_in(&self, exp: u32, scratch: &LimbScratch) -> Result<Self, LimbScratchError> {
        self.0.shl_in(exp, scratch).map(Self)
    }
    /// Exact power-of-ten product with bounded reusable storage.
    /// # Errors
    /// Refuses scratch limb/destination exhaustion without allocating.
    pub fn mul_pow10_in(&self, exp: u32, scratch: &LimbScratch) -> Result<Self, LimbScratchError> {
        self.0
            .mul_in(&Int::pow10_in(exp, scratch)?, scratch)
            .map(Self)
    }
    /// Integer floor root with bounded reusable storage.
    /// # Errors
    /// Refuses scratch exhaustion; a negative input returns `Ok(None)`.
    pub fn sqrt_floor_in(&self, scratch: &LimbScratch) -> Result<Option<Self>, LimbScratchError> {
        Ok(self.0.sqrt_floor_in(scratch)?.map(Self))
    }

    /// Exact fused product/quotient with ordinary integer storage.
    #[must_use]
    pub fn mul_div_rem(&self, factor: &Self, divisor: &Self) -> Option<(Self, Self)> {
        self.0
            .mul_div_rem(&factor.0, &divisor.0)
            .map(|(q, r)| (Self(q), Self(r)))
    }
    /// Identical fused product/quotient with bounded reusable limb storage.
    /// # Errors
    /// Refuses scratch exhaustion; zero divisor returns `Ok(None)`.
    pub fn mul_div_rem_in(
        &self,
        factor: &Self,
        divisor: &Self,
        scratch: &LimbScratch,
    ) -> Result<Option<(Self, Self)>, LimbScratchError> {
        Ok(self
            .0
            .mul_div_rem_in(&factor.0, &divisor.0, scratch)?
            .map(|(q, r)| (Self(q), Self(r))))
    }
    /// Exact fused product floor root with ordinary integer storage.
    #[must_use]
    pub fn sqrt_product_floor(&self, factor: &Self) -> Option<Self> {
        self.0.sqrt_product_floor(&factor.0).map(Self)
    }
    /// Identical fused product floor root with bounded reusable limb storage.
    /// # Errors
    /// Refuses scratch exhaustion; negative product returns `Ok(None)`.
    pub fn sqrt_product_floor_in(
        &self,
        factor: &Self,
        scratch: &LimbScratch,
    ) -> Result<Option<Self>, LimbScratchError> {
        Ok(self.0.sqrt_product_floor_in(&factor.0, scratch)?.map(Self))
    }

    // The legacy Debug shape and the independent Horner regression witness use
    // decimal groups. This is a formatting projection, never an arithmetic backend.
    fn decimal_limbs(&self) -> Vec<u32> {
        let mut magnitude = self.abs();
        let mut limbs = Vec::new();
        while !magnitude.is_zero() {
            let (quotient, remainder) = magnitude.div_rem_u64(1_000_000_000).expect("nonzero base");
            limbs.push(remainder as u32);
            magnitude = quotient;
        }
        limbs
    }
}

impl From<Int> for BigInt {
    fn from(value: Int) -> Self {
        Self(value)
    }
}
impl From<BigInt> for Int {
    fn from(value: BigInt) -> Self {
        value.0
    }
}
impl fmt::Debug for BigInt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BigInt")
            .field("negative", &self.is_negative())
            .field("limbs", &self.decimal_limbs())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::BigInt;

    #[test]
    fn roundtrips_i128_extremes() {
        for v in [
            0_i128,
            1,
            -1,
            i128::MAX,
            i128::MIN,
            i128::MAX - 1,
            i128::MIN + 1,
        ] {
            assert_eq!(BigInt::from_i128(v).to_i128(), Some(v), "roundtrip of {v}");
        }
    }

    #[test]
    fn adds_within_i128_exactly() {
        let mut a = BigInt::from_i128(40);
        a.add_i128(2);
        assert_eq!(a.to_i128(), Some(42));
        assert_eq!(a.to_decimal_string(), "42");
    }

    #[test]
    fn cancels_back_into_i128_range() {
        // i128::MAX + 1 + (-i128::MAX) == 1, even though the running total visits
        // i128::MAX (fits) then i128::MAX + 1 (does NOT fit i128) along the way.
        let mut sum = BigInt::from_i128(i128::MAX);
        sum.add_i128(1);
        sum.add_i128(-i128::MAX);
        assert_eq!(sum.to_i128(), Some(1));
        assert_eq!(sum.to_decimal_string(), "1");
    }

    #[test]
    fn exceeds_i128_and_still_renders_exactly() {
        let mut sum = BigInt::from_i128(i128::MAX);
        sum.add_i128(i128::MAX);
        assert_eq!(sum.to_i128(), None, "2 * i128::MAX must not fit i128");
        assert_eq!(
            sum.to_decimal_string(),
            "340282366920938463463374607431768211454"
        );
    }

    #[test]
    fn negative_exceeds_i128_and_still_renders_exactly() {
        let mut sum = BigInt::from_i128(i128::MIN);
        sum.add_i128(i128::MIN);
        assert_eq!(sum.to_i128(), None);
        assert_eq!(
            sum.to_decimal_string(),
            "-340282366920938463463374607431768211456"
        );
    }

    #[test]
    fn to_decimal_lexical_matches_decimal_canonical_lexical_shape() {
        // Integer-valued: no decimal point, matching XSD 1.1 §E.1 `decimalCanonicalMap`.
        assert_eq!(BigInt::from_i128(0).to_decimal_lexical(0), "0");
        assert_eq!(BigInt::from_i128(42).to_decimal_lexical(0), "42");
        // scale > 0 but the magnitude's digits are all consumed by trailing zeros:
        // 4200 at scale 2 is "42.00" -> trimmed to "42".
        assert_eq!(BigInt::from_i128(4200).to_decimal_lexical(2), "42");
        // Fractional, trailing zeros trimmed but not the whole fraction: 425 at
        // scale 2 is "4.25".
        assert_eq!(BigInt::from_i128(425).to_decimal_lexical(2), "4.25");
        // Magnitude shorter than scale: leading zero padding in the fraction.
        assert_eq!(BigInt::from_i128(5).to_decimal_lexical(3), "0.005");
        // Negative sign preserved.
        assert_eq!(BigInt::from_i128(-425).to_decimal_lexical(2), "-4.25");
    }

    #[test]
    fn to_decimal_lexical_exceeds_i128_and_still_renders_exactly() {
        // The whole point of this method: a magnitude with no i128 mantissa
        // representation at all still renders as exact canonical decimal text —
        // 2 * i128::MAX at scale 18 (AVG's fixed target scale).
        let mut dividend = BigInt::from_i128(i128::MAX);
        dividend.add_i128(i128::MAX);
        let scaled = dividend.mul_pow10(18);
        assert_eq!(
            scaled.to_decimal_lexical(18),
            "340282366920938463463374607431768211454"
        );
    }

    #[test]
    fn addition_is_commutative_and_order_independent() {
        let values: [i128; 5] = [i128::MAX, 1, -i128::MAX, i128::MIN / 2, -(i128::MIN / 2)];
        let forward = {
            let mut acc = BigInt::zero();
            for v in values {
                acc.add_i128(v);
            }
            acc
        };
        let backward = {
            let mut acc = BigInt::zero();
            for v in values.iter().rev() {
                acc.add_i128(*v);
            }
            acc
        };
        assert_eq!(forward, backward);
        assert_eq!(forward.to_decimal_string(), "1");
    }

    #[test]
    fn zero_is_canonical() {
        let mut a = BigInt::from_i128(5);
        a.add_i128(-5);
        assert!(a.is_zero());
        assert_eq!(a.to_decimal_string(), "0");
        assert_eq!(a, BigInt::zero());
    }

    // A deterministic SplitMix64 counter stream for the float-conversion
    // tests: the workspace's shared test stream, through `purrdf-testkit` (a
    // dev-dependency).
    use purrdf_testkit::rng::splitmix64_next as splitmix64;

    /// The exact `BigInt` `Σ words[i] × 2^(64 i)`, negated when `negative`.
    fn from_words(words: &[u64], negative: bool) -> BigInt {
        let mut acc = BigInt::zero();
        for &word in words.iter().rev() {
            acc = acc.mul_pow2(64);
            acc.add_assign(&BigInt::from_u128(u128::from(word)));
        }
        if negative && !acc.is_zero() {
            acc = acc.negated();
        }
        acc
    }

    /// `2^exp`, exactly.
    fn pow2(exp: u32) -> BigInt {
        BigInt::from_i128(1).mul_pow2(exp)
    }

    /// `a × 2^exp + offset`, exactly.
    fn scaled(a: u64, exp: u32, offset: i128) -> BigInt {
        let mut value = BigInt::from_u128(u128::from(a)).mul_pow2(exp);
        value.add_i128(offset);
        value
    }

    /// A random magnitude of exactly `bits` significant bits (zero for `bits == 0`).
    fn random_of_bit_length(state: &mut u64, bits: u32) -> BigInt {
        if bits == 0 {
            return BigInt::zero();
        }
        let word_count = bits.div_ceil(64) as usize;
        let mut words: Vec<u64> = (0..word_count).map(|_| splitmix64(state)).collect();
        let top_bits = bits - 64 * (word_count as u32 - 1);
        let top = words.last_mut().expect("at least one word");
        if top_bits < 64 {
            *top &= (1u64 << top_bits) - 1;
        }
        *top |= 1u64 << (top_bits - 1);
        // Sometimes clear every word below the top one, so exact ties and
        // near-powers of two are drawn too, not only dense bit patterns.
        if splitmix64(state).is_multiple_of(8) {
            let len = words.len();
            for word in &mut words[..len - 1] {
                *word = 0;
            }
        }
        from_words(&words, splitmix64(state).is_multiple_of(2))
    }

    /// The independent oracle: Rust's decimal-to-float parser is correctly
    /// rounded (round-half-even, `±∞` on overflow), so parsing the exact decimal
    /// form gives the correctly rounded conversion by a route that shares no code
    /// with [`BigInt::to_f64`]/[`BigInt::to_f32`].
    fn oracle_f64(value: &BigInt) -> f64 {
        value
            .to_decimal_string()
            .parse::<f64>()
            .expect("decimal integer parses")
    }

    fn oracle_f32(value: &BigInt) -> f32 {
        value
            .to_decimal_string()
            .parse::<f32>()
            .expect("decimal integer parses")
    }

    fn assert_matches_oracle(value: &BigInt) {
        assert_eq!(
            value.to_f64().to_bits(),
            oracle_f64(value).to_bits(),
            "to_f64 of {}",
            value.to_decimal_string()
        );
        assert_eq!(
            value.to_f32().to_bits(),
            oracle_f32(value).to_bits(),
            "to_f32 of {}",
            value.to_decimal_string()
        );
    }

    /// The conversion this module shipped before: a Horner fold over the
    /// base-`1e9` limbs, rounding at every limb. Kept only as the refused side of
    /// the witness below.
    fn horner_to_f64(value: &BigInt) -> f64 {
        let mut acc = 0.0_f64;
        for &limb in value.decimal_limbs().iter().rev() {
            acc = acc.mul_add(1_000_000_000.0, f64::from(limb));
        }
        if value.is_negative() { -acc } else { acc }
    }

    #[test]
    fn float_conversions_are_correctly_rounded_across_bit_lengths() {
        let mut state = 0x5EED_0F64_u64;
        for bits in 0..=1100_u32 {
            for _ in 0..6 {
                assert_matches_oracle(&random_of_bit_length(&mut state, bits));
            }
        }
    }

    #[test]
    fn float_conversions_are_correctly_rounded_at_powers_of_two_and_ties() {
        assert_matches_oracle(&BigInt::zero());
        assert_eq!(BigInt::zero().to_f64().to_bits(), 0, "zero is +0.0");
        for exp in 0..=1100_u32 {
            let power = pow2(exp);
            assert_matches_oracle(&power);
            for offset in [-2_i128, -1, 1, 2] {
                let mut near = power.clone();
                near.add_i128(offset);
                assert_matches_oracle(&near);
            }
        }
        // 2^54 ± k: the ulp there is 4, so ±2 are exact ties and ±1/±3 sit just
        // below/above halfway.
        for offset in -8_i128..=8 {
            assert_matches_oracle(&scaled(1, 54, offset));
        }
        // Exact 53-bit ties at every scale (even and odd kept significands), and
        // the values one unit either side of each.
        for exp in 1..=1000_u32 {
            for significand in [(1u64 << 53) + 1, (1u64 << 53) + 3, (1u64 << 54) - 1] {
                for offset in [-1_i128, 0, 1] {
                    assert_matches_oracle(&scaled(significand, exp, offset));
                }
            }
            // The same ties at 24 bits, for `to_f32`.
            for significand in [(1u64 << 24) + 1, (1u64 << 24) + 3] {
                for offset in [-1_i128, 0, 1] {
                    assert_matches_oracle(&scaled(significand, exp, offset));
                }
            }
        }
    }

    #[test]
    fn float_conversions_overflow_exactly_at_the_rounding_boundary() {
        // f64::MAX = (2^53 − 1) × 2^971; the halfway point to 2^1024 is
        // (2^54 − 1) × 2^970, a tie whose even neighbour is 2^1024 → ∞.
        let max = scaled((1u64 << 53) - 1, 971, 0);
        assert_eq!(max.to_f64(), f64::MAX);
        let halfway = scaled((1u64 << 54) - 1, 970, 0);
        let mut below = halfway.clone();
        below.add_i128(-1);
        assert_eq!(below.to_f64(), f64::MAX, "just below halfway rounds down");
        assert_eq!(halfway.to_f64(), f64::INFINITY, "the tie rounds to even: ∞");
        assert_eq!(pow2(1024).to_f64(), f64::INFINITY);
        assert_eq!(pow2(1100).to_f64(), f64::INFINITY);
        let mut negative = halfway.clone();
        negative = negative.negated();
        assert_eq!(negative.to_f64(), f64::NEG_INFINITY);
        for value in [&max, &below, &halfway, &negative] {
            assert_matches_oracle(value);
        }
        // f32::MAX = (2^24 − 1) × 2^104; halfway to 2^128 is (2^25 − 1) × 2^103.
        let f32_halfway = scaled((1u64 << 25) - 1, 103, 0);
        let mut f32_below = f32_halfway.clone();
        f32_below.add_i128(-1);
        assert_eq!(f32_below.to_f32(), f32::MAX);
        assert_eq!(f32_halfway.to_f32(), f32::INFINITY);
        assert_matches_oracle(&f32_halfway);
        assert_matches_oracle(&f32_below);
    }

    /// The refused-old / neighbour-new pair: search the random stream for a value
    /// the old per-limb Horner fold rounded away from the correctly rounded
    /// result, assert such a value exists (so this test observes the defect, not
    /// a vacuous pass), and assert the new conversion matches the oracle there
    /// and on every neighbour the search inspected.
    #[test]
    fn the_old_horner_conversion_is_refused_and_the_new_one_matches_the_oracle() {
        let mut state = 0x0DD_BA11_u64;
        let mut witness: Option<BigInt> = None;
        'search: for bits in 54..=400_u32 {
            for _ in 0..64 {
                let value = random_of_bit_length(&mut state, bits);
                let oracle = oracle_f64(&value);
                assert_eq!(value.to_f64().to_bits(), oracle.to_bits());
                if horner_to_f64(&value).to_bits() != oracle.to_bits() {
                    witness = Some(value);
                    break 'search;
                }
            }
        }
        let witness = witness.expect("the old Horner fold misrounds some value in the stream");
        assert_ne!(
            horner_to_f64(&witness).to_bits(),
            oracle_f64(&witness).to_bits()
        );
        assert_eq!(witness.to_f64().to_bits(), oracle_f64(&witness).to_bits());
    }

    /// `(2^24 + 1) × 2^103 + 1` sits just above an `f32` halfway point, so it
    /// rounds up to `(2^24 + 2) × 2^103`; going through `f64` first drops the
    /// `+ 1` and leaves an exact tie that rounds to even, `2^127` — the double
    /// rounding `to_f32` exists to avoid.
    #[test]
    fn to_f32_rounds_once_where_narrowing_to_f64_would_round_twice() {
        let value = scaled((1u64 << 24) + 1, 103, 1);
        let correct = f32::from_bits((254 << 23) | 1);
        assert_eq!(value.to_f32().to_bits(), correct.to_bits());
        assert_eq!(oracle_f32(&value).to_bits(), correct.to_bits());
        assert_ne!((value.to_f64() as f32).to_bits(), correct.to_bits());
    }

    /// A value the old Horner fold rounds one ulp high (`9.174069745714053e24`
    /// against the correctly rounded `9.174069745714052e24`).
    const PINNED_WITNESS: u128 = 9_174_069_745_714_051_707_214_353;

    /// A fixed witness found by the search above, pinned so the defect stays
    /// visible even if the random stream's draw order ever changes.
    #[test]
    fn pinned_horner_witness_rounds_correctly() {
        let witness = BigInt::from_u128(PINNED_WITNESS);
        assert_ne!(
            horner_to_f64(&witness).to_bits(),
            oracle_f64(&witness).to_bits()
        );
        assert_eq!(witness.to_f64().to_bits(), oracle_f64(&witness).to_bits());
    }

    #[test]
    fn mul_pow10_is_exact_across_a_limb_boundary() {
        let a = BigInt::from_i128(123);
        assert_eq!(a.mul_pow10(0).to_decimal_string(), "123");
        assert_eq!(a.mul_pow10(2).to_decimal_string(), "12300");
        // 9 (a whole limb) plus a leftover of 2 more digits.
        assert_eq!(
            a.mul_pow10(11).to_decimal_string(),
            format!("123{}", "0".repeat(11))
        );
        let neg = BigInt::from_i128(-7);
        assert_eq!(neg.mul_pow10(3).to_decimal_string(), "-7000");
        assert_eq!(BigInt::zero().mul_pow10(5), BigInt::zero());
    }

    #[test]
    fn div_rem_u64_matches_i128_division_within_i128_range() {
        for (dividend, divisor) in [(100_i128, 3_u64), (-100, 3), (7, 2), (-7, 2), (0, 5)] {
            let (quotient, remainder) = BigInt::from_i128(dividend).div_rem_u64(divisor).unwrap();
            let expected_q = dividend / i128::from(divisor);
            let expected_r = (dividend % i128::from(divisor)).unsigned_abs();
            assert_eq!(
                quotient.to_i128(),
                Some(expected_q),
                "{dividend} / {divisor}"
            );
            assert_eq!(u128::from(remainder), expected_r, "{dividend} % {divisor}");
        }
    }

    #[test]
    fn div_rem_u64_rejects_zero_divisor() {
        assert!(BigInt::from_i128(5).div_rem_u64(0).is_none());
    }

    /// `i128::MIN`'s magnitude is `2^127`, which `from_i128` can only reach as a
    /// NEGATIVE value — the exact numeric order needs it as a magnitude.
    #[test]
    fn from_u128_reaches_the_magnitude_no_i128_can_hold() {
        let magnitude = i128::MIN.unsigned_abs();
        assert!(i128::try_from(magnitude).is_err());
        let big = BigInt::from_u128(magnitude);
        assert!(!big.is_negative());
        assert_eq!(
            big.to_decimal_string(),
            "170141183460469231731687303715884105728"
        );
        assert_eq!(BigInt::from_u128(0), BigInt::zero());
        assert_eq!(BigInt::from_u128(42), BigInt::from_i128(42));
    }

    #[test]
    fn mul_pow2_is_exact_across_chunk_boundaries() {
        let one = BigInt::from_i128(1);
        assert_eq!(one.mul_pow2(0), one);
        assert_eq!(one.mul_pow2(10).to_i128(), Some(1024));
        // Straddles the 29-bit chunking twice over.
        assert_eq!(one.mul_pow2(64).to_i128(), Some(1i128 << 64));
        assert_eq!(one.mul_pow2(126).to_i128(), Some(1i128 << 126));
        // And beyond i128 entirely, where the whole point is that nothing wraps.
        assert_eq!(
            one.mul_pow2(128).to_decimal_string(),
            "340282366920938463463374607431768211456"
        );
        assert_eq!(BigInt::from_i128(-3).mul_pow2(4).to_i128(), Some(-48));
        assert_eq!(BigInt::zero().mul_pow2(9), BigInt::zero());
    }

    #[test]
    fn ord_agrees_with_i128_and_survives_the_i128_ceiling() {
        let values: [i128; 7] = [i128::MIN, -5, -1, 0, 1, 5, i128::MAX];
        for a in values {
            for b in values {
                assert_eq!(
                    BigInt::from_i128(a).cmp(&BigInt::from_i128(b)),
                    a.cmp(&b),
                    "{a} vs {b}"
                );
            }
        }
        // Two totals that both escaped i128 still order exactly.
        let mut huge = BigInt::from_i128(i128::MAX);
        huge.add_i128(i128::MAX);
        let mut huger = huge.clone();
        huger.add_i128(1);
        assert!(huge < huger);
        assert!(BigInt::from_i128(i128::MAX) < huge);
    }

    #[test]
    fn mul_pow10_then_div_rem_u64_reproduces_exact_decimal_division() {
        // (i128::MAX + i128::MAX) / 2 == i128::MAX exactly — an average whose
        // SUM does not fit i128 but whose quotient does.
        let mut sum = BigInt::from_i128(i128::MAX);
        sum.add_i128(i128::MAX);
        let scaled = sum.mul_pow10(18);
        let (quotient, _remainder) = scaled.div_rem_u64(2).unwrap();
        assert_eq!(
            quotient.to_decimal_string(),
            format!("{}{}", i128::MAX, "0".repeat(18))
        );
    }

    #[test]
    fn from_digits_reads_signed_digit_runs_and_refuses_anything_else() {
        for text in ["", "+", "-", "1.0", "1e3", " 1", "0x1", "1_0"] {
            assert_eq!(BigInt::from_digits(text), None, "{text:?}");
        }
        assert_eq!(BigInt::from_digits("-0"), Some(BigInt::zero()));
        assert_eq!(BigInt::from_digits("+007"), Some(BigInt::from_i128(7)));
        let long = "123456789012345678901234567890123456789";
        assert_eq!(
            BigInt::from_digits(long)
                .map(|value| value.to_decimal_string())
                .as_deref(),
            Some(long)
        );
        assert_eq!(
            BigInt::from_digits(&i128::MIN.to_string()),
            Some(BigInt::from_i128(i128::MIN))
        );
    }

    /// `mul` and `rem` agree with `i128` wherever it holds the product, and
    /// `rem` keeps the dividend's sign as Rust's `%` does.
    #[test]
    fn mul_and_rem_match_i128_arithmetic() {
        let values: [i128; 12] = [
            0,
            1,
            -1,
            7,
            -13,
            999_999_999,
            1_000_000_000,
            -1_000_000_007,
            123_456_789_012_345_678,
            -(1 << 62),
            (1 << 63) - 25,
            4_611_686_018_427_387_903,
        ];
        for &a in &values {
            for &b in &values {
                let product = BigInt::from_i128(a).mul(&BigInt::from_i128(b));
                assert_eq!(product.to_i128(), Some(a * b), "{a} × {b}");
                if b == 0 {
                    assert_eq!(BigInt::from_i128(a).rem(&BigInt::zero()), None);
                } else {
                    let rem = BigInt::from_i128(a).rem(&BigInt::from_i128(b));
                    assert_eq!(rem.and_then(|r| r.to_i128()), Some(a % b), "{a} % {b}");
                }
            }
        }
    }

    /// Multi-limb divisors: `(q·d + r) mod d == r` for large `q` and `r < d`.
    #[test]
    fn rem_by_a_multi_limb_divisor_recovers_the_remainder() {
        let divisors = [
            "1000000000",
            "1000000001",
            "999999999999999999",
            "123456789123456789123456789",
            "1000000000000000000000000000000000001",
        ];
        let quotients = ["1", "999999999", "98765432109876543210987654321", "5"];
        for divisor in divisors {
            let d = BigInt::from_digits(divisor).expect("digits");
            for quotient in quotients {
                let q = BigInt::from_digits(quotient).expect("digits");
                for remainder in ["0", "1", "999999999"] {
                    let r = BigInt::from_digits(remainder).expect("digits");
                    let mut n = q.mul(&d);
                    n.add_assign(&r);
                    assert_eq!(
                        n.rem(&d),
                        Some(r.clone()),
                        "{quotient}·{divisor}+{remainder}"
                    );
                    assert_eq!(n.negated().rem(&d), Some(r.negated()));
                }
            }
        }
    }

    #[test]
    fn mul_pow5_and_mul_pow2_compose_to_mul_pow10() {
        let value = BigInt::from_i128(-123_456_789_987);
        for exp in [0, 1, 12, 13, 14, 29, 100, 400] {
            assert_eq!(
                value.mul_pow5(exp).mul_pow2(exp),
                value.mul_pow10(exp),
                "{exp}"
            );
        }
        assert_eq!(BigInt::from_i128(3).mul_small(0), BigInt::zero());
        assert_eq!(
            BigInt::from_i128(-3).mul_small(u32::MAX).to_i128(),
            Some(-3 * i128::from(u32::MAX))
        );
    }

    #[test]
    fn from_binary_writes_dyadic_values_out_in_full() {
        let show = |numerator: i128, exponent: i32| {
            let (mantissa, scale) = BigInt::from_binary(numerator, exponent);
            mantissa.to_decimal_lexical(scale)
        };
        assert_eq!(show(3, 0), "3");
        assert_eq!(show(3, 4), "48");
        assert_eq!(show(3, -1), "1.5");
        assert_eq!(show(-1, -3), "-0.125");
        assert_eq!(show(0, -1074), "0");
    }

    #[test]
    fn parity_and_negation() {
        assert!(BigInt::from_i128(1_000_000_001).is_odd());
        assert!(!BigInt::from_i128(-1_000_000_000).is_odd());
        assert!(!BigInt::zero().is_odd());
        assert_eq!(BigInt::zero().negated(), BigInt::zero());
        assert_eq!(BigInt::from_i128(5).negated(), BigInt::from_i128(-5));
    }
}
