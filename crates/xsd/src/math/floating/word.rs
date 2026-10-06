// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Outward double-word intervals: about 106 significant bits from correctly
//! rounded binary64 operations alone.
//!
//! A [`Word`] is the unevaluated sum `hi + lo` of two binary64 values, kept
//! normalized by an exact two-sum so that `|lo| <= ulp(hi)/2`. Sum, product,
//! quotient and square root follow the classical error-free transformations
//! (Dekker, *Numerische Mathematik* 18 (1971), doi:10.1007/BF01397083; Joldes,
//! Muller and Popescu, *ACM TOMS* 44 (2017), doi:10.1145/3121432). Without a
//! fused multiply-add, every operation here has relative error below
//! `16 u^2 = 2^-102`, `u = 2^-53`, whenever no operand or partial result leaves
//! `[2^-400, 2^400]`; the range is checked, and leaving it refuses rather than
//! guessing.
//!
//! Interval endpoints are then stepped outward by `2^-98` of their leading
//! component. That step exceeds every operation's error bound sixteenfold, so
//! each returned interval encloses the exact result of the operation applied to
//! every point of its operands. Exact zero stays exact: the relative bounds
//! force a computed zero to be the exact one.
//!
//! The elementary functions reduce their argument with the same interval
//! operations and evaluate fixed-degree Taylor or arctangent series whose
//! remainders are added as explicit intervals, mirroring the binary64 kernels.

use std::sync::OnceLock;

use super::{MathError, two_product, two_sum};
use crate::ieee::Binary64;
use crate::ieee::dyadic::Binary64Dyadic;
use crate::ieee::ratio::{Rounding, to_binary64};
use crate::integer::Int;
use core::cmp::Ordering;

/// Relative outward step of every endpoint, `2^-98`.
const STEP: f64 = f64::from_bits((1023 - 98) << 52);
/// The checked magnitude range of every nonzero leading component.
const SMALLEST: f64 = f64::from_bits((1023 - 400) << 52);
const LARGEST: f64 = f64::from_bits((1023 + 400) << 52);

/// A normalized double-word value `hi + lo`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Word {
    hi: f64,
    lo: f64,
}

impl Word {
    /// Exact zero.
    pub const ZERO: Self = Self { hi: 0.0, lo: 0.0 };

    /// One binary64 value, exactly.
    ///
    /// # Errors
    /// Refuses a nonfinite value or one outside the checked range.
    pub fn from_binary64(value: f64) -> Result<Self, MathError> {
        Self { hi: value, lo: 0.0 }.checked()
    }

    /// The leading component.
    #[must_use]
    pub const fn hi(self) -> f64 {
        self.hi
    }

    /// The trailing component.
    #[must_use]
    pub const fn lo(self) -> f64 {
        self.lo
    }

    /// Whether the value is exactly zero.
    #[must_use]
    pub fn is_zero(self) -> bool {
        self.hi == 0.0
    }

    /// Whether the value is strictly negative.
    #[must_use]
    pub fn is_negative(self) -> bool {
        self.hi < 0.0
    }

    fn abs(self) -> Self {
        if self.is_negative() { -self } else { self }
    }

    fn checked(self) -> Result<Self, MathError> {
        if !self.hi.is_finite() || !self.lo.is_finite() {
            return Err(MathError::Binary64Range);
        }
        if self.hi == 0.0 {
            return Ok(Self::ZERO);
        }
        let magnitude = self.hi.abs();
        if !(SMALLEST..=LARGEST).contains(&magnitude) {
            return Err(MathError::PrecisionExhausted);
        }
        Ok(self)
    }

    fn pair(hi: f64, lo: f64, ops: Binary64<'_>) -> Result<Self, MathError> {
        let (hi, lo) = two_sum(hi, lo, ops)?;
        Self { hi, lo }.checked()
    }

    fn add(self, rhs: Self, ops: Binary64<'_>) -> Result<Self, MathError> {
        // Accurate double-word sum (Joldes-Muller-Popescu, Algorithm 6),
        // with exact two-sums in place of the fast variant.
        let (sh, sl) = two_sum(self.hi, rhs.hi, ops)?;
        let (th, tl) = two_sum(self.lo, rhs.lo, ops)?;
        let carry = ops.add(sl, th);
        let (vh, vl) = two_sum(sh, carry, ops)?;
        let tail = ops.add(tl, vl);
        Self::pair(vh, tail, ops)
    }

    fn mul(self, rhs: Self, ops: Binary64<'_>) -> Result<Self, MathError> {
        if self.is_zero() || rhs.is_zero() {
            return Ok(Self::ZERO);
        }
        let (ch, cl) = two_product(self.hi, rhs.hi, ops)?.ok_or(MathError::PrecisionExhausted)?;
        let cross = ops.add(ops.mul(self.hi, rhs.lo), ops.mul(self.lo, rhs.hi));
        Self::pair(ch, ops.add(cl, cross), ops)
    }

    fn div(self, rhs: Self, ops: Binary64<'_>) -> Result<Self, MathError> {
        if rhs.is_zero() {
            return Err(MathError::Domain("division by zero"));
        }
        if self.is_zero() {
            return Ok(Self::ZERO);
        }
        let quotient = ops.div(self.hi, rhs.hi);
        let (product, error) =
            two_product(quotient, rhs.hi, ops)?.ok_or(MathError::PrecisionExhausted)?;
        let (r1, r2) = two_sum(self.hi, negate(product), ops)?;
        let remainder = ops.sub(
            ops.add(ops.add(ops.sub(r1, error), r2), self.lo),
            ops.mul(quotient, rhs.lo),
        );
        Self::pair(quotient, ops.div(remainder, rhs.hi), ops)
    }

    fn sqrt(self, ops: Binary64<'_>) -> Result<Self, MathError> {
        if self.is_zero() {
            return Ok(Self::ZERO);
        }
        if self.is_negative() {
            return Err(MathError::Domain("negative square root"));
        }
        let root = ops.sqrt(self.hi);
        let (square, error) = two_product(root, root, ops)?.ok_or(MathError::PrecisionExhausted)?;
        let remainder = ops.add(ops.sub(ops.sub(self.hi, square), error), self.lo);
        Self::pair(root, ops.div(remainder, ops.mul(root, 2.0)), ops)
    }

    fn step(self, outward: bool, ops: Binary64<'_>) -> Result<Self, MathError> {
        if self.is_zero() {
            return Ok(self);
        }
        let amount = ops.mul(self.hi.abs(), STEP);
        let lo = if outward {
            ops.add(self.lo, amount)
        } else {
            ops.sub(self.lo, amount)
        };
        Self::pair(self.hi, lo, ops)
    }
}

impl core::ops::Neg for Word {
    type Output = Self;
    /// Exact negation.
    fn neg(self) -> Self {
        Self {
            hi: negate(self.hi),
            lo: negate(self.lo),
        }
    }
}

impl Eq for Word {}
impl PartialOrd for Word {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Word {
    fn cmp(&self, other: &Self) -> Ordering {
        // Normalized words order by their leading component, then the tail.
        self.hi
            .partial_cmp(&other.hi)
            .expect("finite words")
            .then_with(|| self.lo.partial_cmp(&other.lo).expect("finite words"))
    }
}

fn negate(value: f64) -> f64 {
    f64::from_bits(value.to_bits() ^ (1 << 63))
}

/// Inclusive double-word endpoints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WordInterval {
    lower: Word,
    upper: Word,
}

impl core::ops::Neg for WordInterval {
    type Output = Self;
    /// Exact negation with exchanged endpoints.
    fn neg(self) -> Self {
        Self {
            lower: -self.upper,
            upper: -self.lower,
        }
    }
}

#[derive(Clone, Copy)]
enum Sign {
    Nonnegative,
    Nonpositive,
    Mixed,
}

impl WordInterval {
    /// Validated ordered endpoints.
    ///
    /// # Errors
    /// Refuses reversed or out-of-range endpoints.
    pub fn from_bounds(lower: Word, upper: Word) -> Result<Self, MathError> {
        let lower = lower.checked()?;
        let upper = upper.checked()?;
        if lower > upper {
            return Err(MathError::Domain("reversed double-word interval"));
        }
        Ok(Self { lower, upper })
    }

    /// One exact binary64 interval.
    ///
    /// # Errors
    /// Refuses nonfinite, reversed or out-of-range endpoints.
    pub fn from_binary64(lower: f64, upper: f64) -> Result<Self, MathError> {
        Self::from_bounds(Word::from_binary64(lower)?, Word::from_binary64(upper)?)
    }

    /// Inclusive lower endpoint.
    #[must_use]
    pub const fn lower(&self) -> &Word {
        &self.lower
    }

    /// Inclusive upper endpoint.
    #[must_use]
    pub const fn upper(&self) -> &Word {
        &self.upper
    }

    fn point(value: Word) -> Self {
        Self {
            lower: value,
            upper: value,
        }
    }

    fn outward(lower: Word, upper: Word, ops: Binary64<'_>) -> Result<Self, MathError> {
        Self::from_bounds(lower.step(false, ops)?, upper.step(true, ops)?)
    }

    fn sign(self) -> Sign {
        if !self.lower.is_negative() {
            Sign::Nonnegative
        } else if self.upper.is_negative() || self.upper.is_zero() {
            Sign::Nonpositive
        } else {
            Sign::Mixed
        }
    }

    /// Outward sum.
    ///
    /// # Errors
    /// Refuses range exhaustion.
    pub fn add(self, rhs: Self, ops: Binary64<'_>) -> Result<Self, MathError> {
        Self::outward(
            self.lower.add(rhs.lower, ops)?,
            self.upper.add(rhs.upper, ops)?,
            ops,
        )
    }

    /// Outward difference.
    ///
    /// # Errors
    /// Refuses range exhaustion.
    pub fn sub(self, rhs: Self, ops: Binary64<'_>) -> Result<Self, MathError> {
        self.add(-rhs, ops)
    }

    /// Outward absolute value.
    #[must_use]
    pub fn abs(self) -> Self {
        match self.sign() {
            Sign::Nonnegative => self,
            Sign::Nonpositive => -self,
            Sign::Mixed => Self {
                lower: Word::ZERO,
                upper: self.lower.abs().max(self.upper),
            },
        }
    }

    /// Outward product.
    ///
    /// # Errors
    /// Refuses range exhaustion.
    pub fn mul(self, rhs: Self, ops: Binary64<'_>) -> Result<Self, MathError> {
        let (a, b) = (self, rhs);
        // Monotone rounding selects two endpoint products when a sign is known.
        let pairs = match (a.sign(), b.sign()) {
            (Sign::Nonnegative, Sign::Nonnegative) => {
                Some(((a.lower, b.lower), (a.upper, b.upper)))
            }
            (Sign::Nonnegative, Sign::Nonpositive) => {
                Some(((a.upper, b.lower), (a.lower, b.upper)))
            }
            (Sign::Nonpositive, Sign::Nonnegative) => {
                Some(((a.lower, b.upper), (a.upper, b.lower)))
            }
            (Sign::Nonpositive, Sign::Nonpositive) => {
                Some(((a.upper, b.upper), (a.lower, b.lower)))
            }
            (Sign::Nonnegative, Sign::Mixed) => Some(((a.upper, b.lower), (a.upper, b.upper))),
            (Sign::Nonpositive, Sign::Mixed) => Some(((a.lower, b.upper), (a.lower, b.lower))),
            (Sign::Mixed, Sign::Nonnegative) => Some(((a.lower, b.upper), (a.upper, b.upper))),
            (Sign::Mixed, Sign::Nonpositive) => Some(((a.upper, b.lower), (a.lower, b.lower))),
            (Sign::Mixed, Sign::Mixed) => None,
        };
        if let Some(((p, q), (r, s))) = pairs {
            return Self::outward(p.mul(q, ops)?, r.mul(s, ops)?, ops);
        }
        let products = [
            a.lower.mul(b.lower, ops)?,
            a.lower.mul(b.upper, ops)?,
            a.upper.mul(b.lower, ops)?,
            a.upper.mul(b.upper, ops)?,
        ];
        let lower = products.into_iter().min().expect("four products");
        let upper = products.into_iter().max().expect("four products");
        Self::outward(lower, upper, ops)
    }

    /// Outward square.
    ///
    /// # Errors
    /// Refuses range exhaustion.
    pub fn square(self, ops: Binary64<'_>) -> Result<Self, MathError> {
        match self.sign() {
            Sign::Nonnegative => Self::outward(
                self.lower.mul(self.lower, ops)?,
                self.upper.mul(self.upper, ops)?,
                ops,
            ),
            Sign::Nonpositive => Self::outward(
                self.upper.mul(self.upper, ops)?,
                self.lower.mul(self.lower, ops)?,
                ops,
            ),
            Sign::Mixed => {
                let magnitude = self.lower.abs().max(self.upper);
                Self::outward(Word::ZERO, magnitude.mul(magnitude, ops)?, ops)
            }
        }
    }

    /// Outward quotient, refusing a divisor that touches zero.
    ///
    /// # Errors
    /// Refuses a zero divisor, an unresolved zero crossing, or range exhaustion.
    pub fn div(self, rhs: Self, ops: Binary64<'_>) -> Result<Self, MathError> {
        let touches_zero =
            (rhs.lower.is_negative() || rhs.lower.is_zero()) && !rhs.upper.is_negative();
        if touches_zero {
            return Err(if rhs.lower.is_zero() && rhs.upper.is_zero() {
                MathError::Domain("division by zero")
            } else {
                MathError::PrecisionExhausted
            });
        }
        let (a, b) = (self, rhs);
        let ((p, q), (r, s)) = if b.lower.is_negative() {
            match a.sign() {
                Sign::Nonnegative => ((a.upper, b.upper), (a.lower, b.lower)),
                Sign::Nonpositive => ((a.upper, b.lower), (a.lower, b.upper)),
                Sign::Mixed => ((a.upper, b.upper), (a.lower, b.upper)),
            }
        } else {
            match a.sign() {
                Sign::Nonnegative => ((a.lower, b.upper), (a.upper, b.lower)),
                Sign::Nonpositive => ((a.lower, b.lower), (a.upper, b.upper)),
                Sign::Mixed => ((a.lower, b.lower), (a.upper, b.lower)),
            }
        };
        Self::outward(p.div(q, ops)?, r.div(s, ops)?, ops)
    }

    /// Outward nonnegative square root.
    ///
    /// # Errors
    /// Refuses a negative argument or an unresolved crossing of zero.
    pub fn sqrt(self, ops: Binary64<'_>) -> Result<Self, MathError> {
        if self.lower.is_negative() {
            return Err(if self.upper.is_negative() {
                MathError::Domain("negative square root")
            } else {
                MathError::PrecisionExhausted
            });
        }
        let lower = self.lower.sqrt(ops)?.step(false, ops)?.max(Word::ZERO);
        let upper = self.upper.sqrt(ops)?.step(true, ops)?;
        Self::from_bounds(lower, upper)
    }

    /// The enclosure of `[lower - spread, upper + spread]` for a nonnegative
    /// binary64 spread.
    fn widened(self, spread: f64, ops: Binary64<'_>) -> Result<Self, MathError> {
        if spread == 0.0 {
            return Ok(self);
        }
        // A spread below the checked range widens by the range floor instead.
        let spread = Word::from_binary64(spread.max(SMALLEST))?;
        Self::outward(
            self.lower.add(-spread, ops)?,
            self.upper.add(spread, ops)?,
            ops,
        )
    }

    fn middle(self, ops: Binary64<'_>) -> Result<(Word, f64), MathError> {
        if self.lower == self.upper {
            return Ok((self.lower, 0.0));
        }
        let half = Word::from_binary64(0.5)?;
        let centre = self.lower.add(self.upper, ops)?.mul(half, ops)?;
        let centre = centre.max(self.lower).min(self.upper);
        let below = centre.add(-self.lower, ops)?;
        let above = self.upper.add(-centre, ops)?;
        // A binary64 upper bound of the larger distance to an endpoint.
        let spread = ops
            .add(below.hi.abs(), below.lo.abs())
            .max(ops.add(above.hi.abs(), above.lo.abs()));
        Ok((centre, ops.mul(spread, 1.000_000_1).next_up()))
    }

    fn clamp_unit(self) -> Result<Self, MathError> {
        let one = Word::from_binary64(1.0)?;
        Self::from_bounds(self.lower.max(-one), self.upper.min(one))
    }
}

/// Exact-recurrence coefficient enclosures shared by every double-word kernel.
struct Tables {
    pi: WordInterval,
    sine: [WordInterval; 16],
    cosine: [WordInterval; 16],
    arctangent: [WordInterval; 18],
}

fn tables() -> Result<&'static Tables, MathError> {
    static TABLES: OnceLock<Option<Tables>> = OnceLock::new();
    TABLES
        .get_or_init(|| {
            let pi = pi_enclosure()?;
            let mut factorial = Int::one();
            let mut sine = [WordInterval::point(Word::ZERO); 16];
            let mut cosine = [WordInterval::point(Word::ZERO); 16];
            for degree in 0..32_u32 {
                if degree != 0 {
                    factorial = factorial.mul(&Int::from_u64(u64::from(degree)));
                }
                let index = (degree / 2) as usize;
                let value = WordInterval::from_ratio(&Int::one(), &factorial).ok()?;
                let value = if index.is_multiple_of(2) {
                    value
                } else {
                    -value
                };
                if degree % 2 == 0 {
                    cosine[index] = value;
                } else {
                    sine[index] = value;
                }
            }
            let mut arctangent = [WordInterval::point(Word::ZERO); 18];
            for (index, coefficient) in arctangent.iter_mut().enumerate() {
                let value =
                    WordInterval::from_ratio(&Int::one(), &Int::from_u64(2 * index as u64 + 1))
                        .ok()?;
                *coefficient = if index.is_multiple_of(2) {
                    value
                } else {
                    -value
                };
            }
            Some(Tables {
                pi,
                sine,
                cosine,
                arctangent,
            })
        })
        .as_ref()
        .ok_or(MathError::PrecisionExhausted)
}

/// Machin's formula with alternating partial-sum enclosures of each
/// arctangent, in exact rationals: `pi = 16 atan(1/5) - 4 atan(1/239)`.
fn pi_enclosure() -> Option<WordInterval> {
    // atan(1/q) lies between consecutive partial sums of its alternating series.
    let atan_bounds = |q: i64, terms: u32| -> (Int, Int, Int) {
        // Returns (lower numerator, upper numerator, common denominator).
        let q2 = Int::from_i64(q * q);
        let mut denominator = Int::one();
        let mut power = Int::from_i64(q);
        let mut sum_numerator = Int::zero();
        let mut previous = Int::zero();
        let mut last_denominator = Int::one();
        for k in 0..terms {
            let term_denominator = power.mul(&Int::from_i64(2 * i64::from(k) + 1));
            // sum += (-1)^k / term_denominator over a growing common denominator.
            previous = sum_numerator.clone();
            let scaled = sum_numerator.mul(&term_denominator);
            sum_numerator = if k % 2 == 0 {
                scaled.add(&denominator)
            } else {
                scaled.sub(&denominator)
            };
            denominator = denominator.mul(&term_denominator);
            last_denominator = term_denominator;
            power = power.mul(&q2);
        }
        // The previous partial sum, rewritten over the final denominator.
        let previous = previous.mul(&last_denominator);
        if terms.is_multiple_of(2) {
            (sum_numerator, previous, denominator)
        } else {
            (previous, sum_numerator, denominator)
        }
    };
    let (a_low, a_high, a_den) = atan_bounds(5, 40);
    let (b_low, b_high, b_den) = atan_bounds(239, 12);
    // pi in [16 a_low/a_den - 4 b_high/b_den, 16 a_high/a_den - 4 b_low/b_den].
    let common = a_den.mul(&b_den);
    let low = Int::from_i64(16)
        .mul(&a_low)
        .mul(&b_den)
        .sub(&Int::from_i64(4).mul(&b_high).mul(&a_den));
    let high = Int::from_i64(16)
        .mul(&a_high)
        .mul(&b_den)
        .sub(&Int::from_i64(4).mul(&b_low).mul(&a_den));
    let lower = WordInterval::from_ratio(&low, &common).ok()?;
    let upper = WordInterval::from_ratio(&high, &common).ok()?;
    WordInterval::from_bounds(lower.lower, upper.upper).ok()
}

/// Two exact binary64 parts below and above an exact rational whose
/// numerator and denominator fit in 64 bits, without integer allocation.
fn ratio_word_small(n: &Int, d: &Int) -> Option<(Word, Word)> {
    use crate::ieee::ratio::binary64_u128;
    let negative = n.is_negative() != d.is_negative();
    let numerator = u64::try_from(n.unsigned_abs_u128()?).ok()?;
    let denominator = u64::try_from(d.unsigned_abs_u128()?).ok()?;
    if numerator == 0 || denominator == 0 {
        return None;
    }
    let hi = binary64_u128(
        false,
        u128::from(numerator),
        u128::from(denominator),
        Rounding::NearestEven,
    )?;
    let dyadic = Binary64Dyadic::decode(hi)?;
    let exponent = dyadic.exponent();
    // rest = n/d - s 2^e = (n 2^-e - s d) / (d 2^-e) for e < 0, both in 128 bits.
    let scale = u32::try_from(-i64::from(exponent)).ok()?;
    if scale > 63 {
        return None;
    }
    let shifted = i128::from(numerator) << scale;
    let product = i128::from(dyadic.significand()).checked_mul(i128::from(denominator))?;
    let rest = shifted.checked_sub(product)?;
    let rest_negative = rest < 0;
    let scope = crate::ieee::Binary64Scope::enter();
    let ops = scope.ops();
    // rest / (d 2^scale): round rest/d, then scale by the exact power 2^-scale.
    // Directed rounding commutes with an exact power-of-two scaling.
    let power = f64::from_bits((1023 - u64::from(scale)) << 52);
    let (low, high) = if rest == 0 {
        (0.0, 0.0)
    } else {
        let low = binary64_u128(
            rest_negative,
            rest.unsigned_abs(),
            u128::from(denominator),
            Rounding::Down,
        )?;
        let high = binary64_u128(
            rest_negative,
            rest.unsigned_abs(),
            u128::from(denominator),
            Rounding::Up,
        )?;
        let (low, high) = (ops.mul(low, power), ops.mul(high, power));
        // Leaving the normal range would make that scaling inexact.
        if low != 0.0 && low.abs() < f64::MIN_POSITIVE
            || high != 0.0 && high.abs() < f64::MIN_POSITIVE
        {
            return None;
        }
        (low, high)
    };
    let lower = Word::pair(hi, low, ops).ok()?;
    let upper = Word::pair(hi, high, ops).ok()?;
    Some(if negative {
        (-upper, -lower)
    } else {
        (lower, upper)
    })
}

/// Two exact binary64 parts below and above an exact rational.
fn ratio_word(n: &Int, d: &Int) -> Option<(Word, Word)> {
    if let Some(words) = ratio_word_small(n, d) {
        return Some(words);
    }
    ratio_word_wide(n, d)
}

fn ratio_word_wide(n: &Int, d: &Int) -> Option<(Word, Word)> {
    let hi = to_binary64(n, d, Rounding::NearestEven)?;
    if !hi.is_finite() {
        return None;
    }
    let dyadic = Binary64Dyadic::decode(hi)?;
    let mut significand = Int::from_u64(dyadic.significand());
    if dyadic.negative() {
        significand = significand.neg();
    }
    let exponent = dyadic.exponent();
    let (rest, rest_denominator) = if exponent >= 0 {
        (
            n.sub(&significand.shl(exponent.unsigned_abs()).mul(d)),
            d.clone(),
        )
    } else {
        let scale = exponent.unsigned_abs();
        (n.shl(scale).sub(&significand.mul(d)), d.shl(scale))
    };
    let low = to_binary64(&rest, &rest_denominator, Rounding::Down)?;
    let high = to_binary64(&rest, &rest_denominator, Rounding::Up)?;
    // Both sums are exact: hi + low <= n/d <= hi + high.
    let scope = crate::ieee::Binary64Scope::enter();
    let ops = scope.ops();
    let lower = Word::pair(hi, low, ops).ok()?;
    let upper = Word::pair(hi, high, ops).ok()?;
    Some((lower, upper))
}

impl WordInterval {
    /// The enclosure of an exact signed ratio.
    ///
    /// # Errors
    /// Refuses a zero denominator or a value outside the checked range.
    pub fn from_ratio(numerator: &Int, denominator: &Int) -> Result<Self, MathError> {
        if denominator.is_zero() {
            return Err(MathError::Domain("division by zero"));
        }
        if numerator.is_zero() {
            return Ok(Self::point(Word::ZERO));
        }
        let (lower, upper) =
            ratio_word(numerator, denominator).ok_or(MathError::PrecisionExhausted)?;
        Self::from_bounds(lower, upper)
    }

    /// The enclosure of pi.
    ///
    /// # Errors
    /// Refuses only if the generated constant table could not be built.
    pub fn pi() -> Result<Self, MathError> {
        Ok(tables()?.pi)
    }

    /// Outward sine and cosine.
    ///
    /// # Errors
    /// Refuses arguments beyond `|x| <= 2^20` or range exhaustion.
    pub fn sin_cos(self, ops: Binary64<'_>) -> Result<(Self, Self), MathError> {
        let tables = tables()?;
        let (centre, spread) = self.middle(ops)?;
        if centre.hi.abs() > 1_048_576.0 {
            return Err(MathError::PrecisionExhausted);
        }
        let quarter = tables.pi.mul(Self::from_binary64(0.5, 0.5)?, ops)?;
        let estimate = ops.mul(centre.hi, ops.div(2.0, tables.pi.lower.hi));
        let turns = estimate.round_ties_even();
        let reduced =
            Self::point(centre).sub(quarter.mul(Self::from_binary64(turns, turns)?, ops)?, ops)?;
        if reduced.lower.hi < -1.0 || reduced.upper.hi > 1.0 {
            return Err(MathError::PrecisionExhausted);
        }
        let square = reduced.square(ops)?;
        let sine = horner(&tables.sine, square, ops)?.mul(reduced, ops)?;
        let cosine = horner(&tables.cosine, square, ops)?;
        // On |x| <= 1 the omitted Taylor tails are below 1/32! and 1/31!.
        let tail = f64::from_bits((1023 - 112) << 52);
        let sine = sine.widened(tail, ops)?;
        let cosine = cosine.widened(tail, ops)?;
        // |turns| <= 2^21 here, so the conversion and residue are exact.
        #[allow(clippy::cast_possible_truncation)]
        let quadrant = (turns as i64).rem_euclid(4);
        let (sine, cosine) = match quadrant {
            0 => (sine, cosine),
            1 => (cosine, -sine),
            2 => (-sine, -cosine),
            _ => (-cosine, sine),
        };
        // |sin'|, |cos'| <= 1 spread the centre values over the argument.
        Ok((
            sine.widened(spread, ops)?.clamp_unit()?,
            cosine.widened(spread, ops)?.clamp_unit()?,
        ))
    }

    /// Outward arctangent.
    ///
    /// # Errors
    /// Refuses range exhaustion.
    pub fn atan(self, ops: Binary64<'_>) -> Result<Self, MathError> {
        if self.lower == self.upper {
            return atan_point(self.lower, ops);
        }
        if self.lower.is_negative() && !self.upper.is_negative() && !self.upper.is_zero() {
            let lower = atan_point(self.lower, ops)?;
            let upper = atan_point(self.upper, ops)?;
            return Self::from_bounds(lower.lower, upper.upper);
        }
        // Increasing with derivative at most 1/(1+min|x|^2) on a sign-definite
        // interval: the mean-value form about one interior point.
        let (centre, spread) = self.middle(ops)?;
        // A binary64 lower bound of min|x|: each leading component is within
        // one unit of its word, so two steps toward zero suffice.
        let near = self
            .lower
            .hi
            .abs()
            .min(self.upper.hi.abs())
            .next_down()
            .next_down()
            .max(0.0);
        let slope = ops
            .div(1.0, ops.add(1.0, ops.mul(near, near)))
            .next_up()
            .min(1.0);
        let spread = ops.mul(spread, slope).next_up();
        atan_point(centre, ops)?.widened(spread, ops)
    }

    /// Outward principal angle with the binary64 kernel's branch rule: an exact
    /// negative-axis zero chooses pi and reaching the cut from below refuses.
    ///
    /// # Errors
    /// Refuses the origin, unresolved branch cuts or range exhaustion.
    pub fn atan2(y: Self, x: Self, ops: Binary64<'_>) -> Result<Self, MathError> {
        let pi = tables()?.pi;
        let half = pi.mul(Self::from_binary64(0.5, 0.5)?, ops)?;
        let x_magnitude = x.lower.abs().max(x.upper.abs());
        if !y.lower.is_negative() && !y.lower.is_zero() && y.lower >= x_magnitude {
            return half.sub(x.div(y, ops)?.atan(ops)?, ops);
        }
        if y.upper.is_negative() && -y.upper >= x_magnitude {
            return (-half).sub(x.div(y, ops)?.atan(ops)?, ops);
        }
        if !x.lower.is_negative() && !x.lower.is_zero() {
            return y.div(x, ops)?.atan(ops);
        }
        if x.upper.is_negative() {
            if y.lower.is_zero() && y.upper.is_zero() {
                return Ok(pi);
            }
            if !y.lower.is_negative() {
                return y.div(x, ops)?.atan(ops)?.add(pi, ops);
            }
            if y.upper.is_negative() {
                return y.div(x, ops)?.atan(ops)?.sub(pi, ops);
            }
            return Err(MathError::PrecisionExhausted);
        }
        if !y.lower.is_negative() && !y.lower.is_zero() {
            return half.sub(x.div(y, ops)?.atan(ops)?, ops);
        }
        if y.upper.is_negative() {
            return (-half).sub(x.div(y, ops)?.atan(ops)?, ops);
        }
        Err(
            if x.lower.is_zero() && x.upper.is_zero() && y.lower.is_zero() && y.upper.is_zero() {
                MathError::Domain("atan2 origin")
            } else {
                MathError::PrecisionExhausted
            },
        )
    }
}

fn horner(
    coefficients: &[WordInterval],
    x: WordInterval,
    ops: Binary64<'_>,
) -> Result<WordInterval, MathError> {
    let Some((&last, preceding)) = coefficients.split_last() else {
        return Err(MathError::Domain("empty polynomial"));
    };
    let mut result = last;
    for &coefficient in preceding.iter().rev() {
        result = result.mul(x, ops)?.add(coefficient, ops)?;
    }
    Ok(result)
}

/// Arctangent of one exact value: reciprocal reduction above one, three
/// half-angle reductions to `|t| <= tan(pi/32) < 0.0985`, then the
/// alternating series to degree 35, whose remainder is below `2^-117`.
fn atan_point(value: Word, ops: Binary64<'_>) -> Result<WordInterval, MathError> {
    if value.is_zero() {
        return Ok(WordInterval::point(Word::ZERO));
    }
    let tables = tables()?;
    let one = WordInterval::from_binary64(1.0, 1.0)?;
    let mut reduced = WordInterval::point(value.abs());
    let reciprocal = value.abs().hi > 1.0;
    if reciprocal {
        reduced = one.div(reduced, ops)?;
    }
    for _ in 0..3 {
        reduced = reduced.div(
            one.add(one.add(reduced.square(ops)?, ops)?.sqrt(ops)?, ops)?,
            ops,
        )?;
    }
    if reduced.upper.hi > 0.1 {
        return Err(MathError::PrecisionExhausted);
    }
    let mut result = horner(&tables.arctangent, reduced.square(ops)?, ops)?
        .mul(reduced, ops)?
        .widened(f64::from_bits((1023 - 117) << 52), ops)?
        .mul(WordInterval::from_binary64(8.0, 8.0)?, ops)?;
    if reciprocal {
        result = tables
            .pi
            .mul(WordInterval::from_binary64(0.5, 0.5)?, ops)?
            .sub(result, ops)?;
    }
    Ok(if value.is_negative() { -result } else { result })
}

#[cfg(test)]
mod tests {
    use super::{Int, ratio_word_small, ratio_word_wide};

    #[test]
    fn the_narrow_ratio_split_equals_the_arbitrary_width_split() {
        let mut state = 0x510e_527f_ade6_82d1_u64;
        let mut checked = 0;
        for case in 0..20_000_u32 {
            let numerator = purrdf_hash::mix::splitmix64_next(&mut state) >> (case % 64);
            let denominator = (purrdf_hash::mix::splitmix64_next(&mut state) >> (case % 61)).max(1);
            for sign in [1_i64, -1] {
                let n = Int::from_u64(numerator).mul(&Int::from_i64(sign));
                let d = Int::from_u64(denominator);
                if let Some(small) = ratio_word_small(&n, &d) {
                    assert_eq!(
                        Some(small),
                        ratio_word_wide(&n, &d),
                        "{numerator}/{denominator}"
                    );
                    checked += 1;
                }
            }
        }
        // Exact decimals with twelve places take the narrow path.
        assert!(
            ratio_word_small(
                &Int::from_i64(36_530_042_355_041),
                &Int::from_i64(1_000_000_000_000)
            )
            .is_some()
        );
        assert!(checked > 20_000, "{checked}");
    }
}
