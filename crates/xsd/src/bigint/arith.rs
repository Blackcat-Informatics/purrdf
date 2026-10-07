// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The general arithmetic of [`BigInt`]: subtraction, truncating division with a
//! quotient, powers, the greatest common divisor, powers of ten, decimal digit
//! counts and the operator traits, so [`crate::exact`]'s arbitrary-precision
//! tower computes on the workspace's one big integer rather than a second one.
//!
//! # Cost
//!
//! Every operation here is a bounded function of its operands' limb counts
//! ([`BigInt::limb_len`]), which is what makes it governable:
//!
//! * `+`, `−`, negation, comparison and decimal shifts are linear;
//! * `×` is schoolbook below [`KARATSUBA_THRESHOLD`] limbs and Karatsuba above it,
//!   so a product of two `n`-limb operands costs at most `n²` and asymptotically
//!   `n^1.585` limb multiplications;
//! * [`BigInt::div_rem`] is schoolbook long division, `(m − n + 1) × (n + 1)` limb
//!   steps for an `m`-limb dividend and an `n`-limb divisor;
//! * [`BigInt::pow`] is square-and-multiply: its result has at most
//!   `limb_len × exp` limbs, and its work is dominated by the final squaring.
//!
//! [`crate::exact::Cost`] turns those bounds into the charge a governor takes
//! before the operation runs.

use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Mul, Neg, Sub};

use super::{
    BigInt, LIMB_BASE, magnitude_add, magnitude_cmp, magnitude_decimal_digits,
    magnitude_div_rem_u64, magnitude_limbs, magnitude_mul, magnitude_sub,
};

/// Below this many limbs in the shorter operand a product is schoolbook; at or
/// above it, Karatsuba. Chosen by the `exact` bench's instruction counts
/// (`xsd_exact_karatsuba`): splitting a 48-limb product costs 3.5% more than
/// schoolbook, while from 128 limbs (about 1,150 digits) the split wins — by
/// 21% there, 2.6× at 1,024 limbs and 3.5× at 2,048.
pub const KARATSUBA_THRESHOLD: usize = 64;

/// Decimal digits in one base-`1e9` limb.
const LIMB_DIGITS: u32 = 9;

impl BigInt {
    /// The value one.
    #[must_use]
    pub fn one() -> Self {
        Self::from_i128(1)
    }

    /// `10^exp`, exactly: a limb shift of one, so linear in `exp`.
    #[must_use]
    pub fn pow10(exp: u32) -> Self {
        Self::one().mul_pow10(exp)
    }

    /// The number of base-`1e9` limbs in the magnitude (zero for zero) — the size
    /// every cost bound in this module is stated in.
    #[must_use]
    pub fn limb_len(&self) -> usize {
        self.limbs.len()
    }

    /// The top two limbs of the magnitude as one number, `top × 1e9 + next`
    /// (just `top` for a one-limb value, zero for zero): the leading digits a
    /// size estimate reads, below `1e18`.
    #[must_use]
    pub(crate) fn leading_u64(&self) -> u64 {
        match self.limbs.as_slice() {
            [] => 0,
            [only] => u64::from(*only),
            [.., next, top] => u64::from(*top) * LIMB_BASE + u64::from(*next),
        }
    }

    /// The number of decimal digits in the magnitude (`1` for zero, as its
    /// canonical lexical form `"0"` has one digit).
    #[must_use]
    pub fn decimal_digits(&self) -> u64 {
        let Some(&top) = self.limbs.last() else {
            return 1;
        };
        let below = (self.limbs.len() as u64 - 1) * u64::from(LIMB_DIGITS);
        below + u64::from(top.ilog10()) + 1
    }

    /// How many times ten divides the value exactly (zero for zero).
    #[must_use]
    pub fn trailing_decimal_zeros(&self) -> u64 {
        let mut zeros = 0_u64;
        for &limb in &self.limbs {
            if limb == 0 {
                zeros += u64::from(LIMB_DIGITS);
                continue;
            }
            let mut limb = limb;
            while limb % 10 == 0 {
                limb /= 10;
                zeros += 1;
            }
            return zeros;
        }
        0
    }

    /// `-1`, `0` or `1` by the sign of the value.
    #[must_use]
    pub fn signum(&self) -> i32 {
        if self.is_zero() {
            0
        } else if self.negative {
            -1
        } else {
            1
        }
    }

    /// The absolute value.
    #[must_use]
    pub fn abs(&self) -> Self {
        Self {
            negative: false,
            limbs: self.limbs.clone(),
        }
    }

    /// Narrow to `i64` if — and only if — the exact value fits.
    #[must_use]
    pub fn to_i64(&self) -> Option<i64> {
        self.to_i128().and_then(|value| i64::try_from(value).ok())
    }

    /// Truncating division, as Rust's `/` and `%` are on machine integers:
    /// `(quotient, remainder)` with the quotient rounded toward zero and the
    /// remainder carrying the sign of `self`, so `self = quotient × divisor +
    /// remainder` and `|remainder| < |divisor|`. That is XPath F&O 3.1
    /// `op:numeric-integer-divide` and `op:numeric-mod`. `None` only for a zero
    /// divisor (`err:FOAR0001`).
    #[must_use]
    pub fn div_rem(&self, divisor: &Self) -> Option<(Self, Self)> {
        if divisor.is_zero() {
            return None;
        }
        let (quotient, remainder) = magnitude_div_rem(&self.limbs, &divisor.limbs);
        Some((
            Self {
                negative: self.negative != divisor.negative && !quotient.is_empty(),
                limbs: quotient,
            },
            Self {
                negative: self.negative && !remainder.is_empty(),
                limbs: remainder,
            },
        ))
    }

    /// Truncating division by `10^exp`: `(quotient, remainder)` as
    /// [`Self::div_rem`] would return for the divisor `10^exp`, but linear — the
    /// whole limbs below the cut are the remainder, with one machine division for
    /// the limb the cut falls inside.
    #[must_use]
    pub fn div_rem_pow10(&self, exp: u64) -> (Self, Self) {
        if exp == 0 || self.is_zero() {
            return (self.clone(), Self::zero());
        }
        let whole = usize::try_from(exp / u64::from(LIMB_DIGITS)).unwrap_or(usize::MAX);
        if whole >= self.limbs.len() {
            return (Self::zero(), self.clone());
        }
        let partial = u32::try_from(exp % u64::from(LIMB_DIGITS)).expect("below nine");
        let mut low = self.limbs[..whole].to_vec();
        let high = &self.limbs[whole..];
        let (mut quotient, cut) = if partial == 0 {
            (high.to_vec(), 0_u64)
        } else {
            magnitude_div_rem_u64(high, 10_u64.pow(partial))
        };
        if partial != 0 {
            // `cut < 10^partial < 1e9`, so it is the top remainder limb itself.
            low.push(u32::try_from(cut).expect("a digit group below 1e9"));
        }
        trim(&mut low);
        trim(&mut quotient);
        (
            Self {
                negative: self.negative && !quotient.is_empty(),
                limbs: quotient,
            },
            Self {
                negative: self.negative && !low.is_empty(),
                limbs: low,
            },
        )
    }

    /// `self^exp`, exactly, by square-and-multiply (`0^0` is `1`, as XPath F&O
    /// 3.1 `math:pow` defines it). The result has at most `limb_len × exp`
    /// limbs; charge [`crate::exact::Integer::pow_cost`] before calling it on a
    /// caller-controlled exponent.
    #[must_use]
    pub fn pow(&self, exp: u32) -> Self {
        let mut result = Self::one();
        if exp == 0 {
            return result;
        }
        if self.is_zero() {
            return Self::zero();
        }
        let mut base = self.abs();
        let mut remaining = exp;
        loop {
            if remaining & 1 == 1 {
                result = Self {
                    negative: false,
                    limbs: mul_magnitudes(&result.limbs, &base.limbs),
                };
            }
            remaining >>= 1;
            if remaining == 0 {
                break;
            }
            base = Self {
                negative: false,
                limbs: mul_magnitudes(&base.limbs, &base.limbs),
            };
        }
        result.negative = self.negative && exp & 1 == 1;
        result
    }

    /// The greatest common divisor of the two magnitudes (always non-negative;
    /// `gcd(0, 0) = 0`), by Euclid's algorithm over [`Self::div_rem`].
    #[must_use]
    pub fn gcd(&self, other: &Self) -> Self {
        let mut a = self.limbs.clone();
        let mut b = other.limbs.clone();
        while !b.is_empty() {
            if let ([x], [y]) = (a.as_slice(), b.as_slice()) {
                return Self::from_u128(crate::wide::gcd(u128::from(*x), u128::from(*y)));
            }
            let (_, remainder) = magnitude_div_rem(&a, &b);
            a = b;
            b = remainder;
        }
        Self {
            negative: false,
            limbs: a,
        }
    }

    /// The product through the threshold dispatch ([`KARATSUBA_THRESHOLD`]):
    /// the same value as [`Self::mul`], sub-quadratic for large operands.
    #[must_use]
    pub fn mul_fast(&self, other: &Self) -> Self {
        if self.is_zero() || other.is_zero() {
            return Self::zero();
        }
        Self {
            negative: self.negative != other.negative,
            limbs: mul_magnitudes(&self.limbs, &other.limbs),
        }
    }

    /// The difference `self − other`, exactly.
    #[must_use]
    pub fn minus(&self, other: &Self) -> Self {
        let mut out = self.clone();
        out.add_assign(&other.negated());
        out
    }

    /// The sum `self + other`, exactly.
    #[must_use]
    pub fn plus(&self, other: &Self) -> Self {
        let mut out = self.clone();
        out.add_assign(other);
        out
    }
}

impl fmt::Display for BigInt {
    /// The canonical `xsd:integer` lexical form ([`BigInt::to_decimal_string`]).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.negative {
            f.write_str("-")?;
        }
        f.write_str(&magnitude_decimal_digits(&self.limbs))
    }
}

impl Add<&BigInt> for &BigInt {
    type Output = BigInt;
    fn add(self, rhs: &BigInt) -> BigInt {
        self.plus(rhs)
    }
}

impl Sub<&BigInt> for &BigInt {
    type Output = BigInt;
    fn sub(self, rhs: &BigInt) -> BigInt {
        self.minus(rhs)
    }
}

impl Mul<&BigInt> for &BigInt {
    type Output = BigInt;
    fn mul(self, rhs: &BigInt) -> BigInt {
        self.mul_fast(rhs)
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

/// Drop most-significant zero limbs, restoring the canonical form.
fn trim(limbs: &mut Vec<u32>) {
    let len = trimmed(limbs).len();
    limbs.truncate(len);
}

/// A canonical view of `limbs`: the slice without its most-significant zeros.
fn trimmed(limbs: &[u32]) -> &[u32] {
    let end = limbs
        .iter()
        .rposition(|&limb| limb != 0)
        .map_or(0, |at| at + 1);
    &limbs[..end]
}

/// `a × b` over canonical magnitudes, schoolbook or Karatsuba by size.
pub(super) fn mul_magnitudes(a: &[u32], b: &[u32]) -> Vec<u32> {
    let (a, b) = (trimmed(a), trimmed(b));
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    if a.len().min(b.len()) < KARATSUBA_THRESHOLD {
        return magnitude_mul(a, b);
    }
    karatsuba(a, b)
}

/// `a × b` by one Karatsuba split at half the longer operand: with
/// `a = a₁·Bᵐ + a₀` and `b = b₁·Bᵐ + b₀`, the product is
/// `z₂·B²ᵐ + z₁·Bᵐ + z₀` where `z₀ = a₀b₀`, `z₂ = a₁b₁` and
/// `z₁ = (a₀ + a₁)(b₀ + b₁) − z₀ − z₂` — three half-size products for four.
/// When one operand is no longer than the split, the other is cut into
/// split-sized pieces instead, so lopsided products stay sub-quadratic.
fn karatsuba(a: &[u32], b: &[u32]) -> Vec<u32> {
    let (long, short) = if a.len() >= b.len() { (a, b) } else { (b, a) };
    let split = long.len() / 2;
    if short.len() <= split {
        // Lopsided: accumulate `short × piece` for each `short.len()`-sized piece
        // of the long operand, in place, so the whole product costs
        // `long / short` balanced products and one pass of carries — never a copy
        // of the running total per piece, which made it quadratic in `long`.
        let step = short.len();
        let mut out = vec![0_u32; long.len() + short.len() + 1];
        let mut offset = 0;
        while offset < long.len() {
            let end = (offset + step).min(long.len());
            let piece = mul_magnitudes(&long[offset..end], short);
            add_into(&mut out[offset..], &piece);
            offset = end;
        }
        trim(&mut out);
        return out;
    }
    let (a0, a1) = (trimmed(&long[..split]), &long[split..]);
    let (b0, b1) = (trimmed(&short[..split]), trimmed(&short[split..]));
    let z0 = mul_magnitudes(a0, b0);
    let z2 = mul_magnitudes(a1, b1);
    let a_sum = magnitude_add(a0, a1);
    let b_sum = magnitude_add(b0, b1);
    let middle = mul_magnitudes(&a_sum, &b_sum);
    let z1 = magnitude_sub(&magnitude_sub(&middle, &z0), &z2);
    let mut out = magnitude_add(&z0, &shifted(&z1, split));
    trim(&mut out);
    out = magnitude_add(&out, &shifted(&z2, 2 * split));
    trim(&mut out);
    out
}

/// `dst += src` in place over base-`1e9` limbs, `dst` long enough that the carry
/// out of its last limb is zero (the caller sized it for the whole product).
fn add_into(dst: &mut [u32], src: &[u32]) {
    let base = u32::try_from(LIMB_BASE).expect("1e9 fits u32");
    let mut carry = 0_u32;
    let mut at = 0;
    for &limb in src {
        // Each limb is below 1e9, so `dst + src + carry < 2e9 + 1 < 2^32`.
        let sum = dst[at] + limb + carry;
        (dst[at], carry) = if sum >= base {
            (sum - base, 1)
        } else {
            (sum, 0)
        };
        at += 1;
    }
    while carry != 0 {
        let sum = dst[at] + carry;
        (dst[at], carry) = if sum >= base {
            (sum - base, 1)
        } else {
            (sum, 0)
        };
        at += 1;
    }
}

/// `limbs × B^places`: `places` zero limbs prepended (zero stays zero).
fn shifted(limbs: &[u32], places: usize) -> Vec<u32> {
    let limbs = trimmed(limbs);
    if limbs.is_empty() {
        return Vec::new();
    }
    let mut out = vec![0_u32; places];
    out.extend_from_slice(limbs);
    out
}

/// `(a ÷ b, a mod b)` over canonical magnitudes, `b` non-empty.
///
/// A one-limb divisor is the machine-word division [`magnitude_div_rem_u64`].
/// Otherwise Knuth's Algorithm D (TAOCP vol. 2, §4.3.1) in base `1e9`, in place:
/// both operands are scaled by `f = ⌊B / (top + 1)⌋` so the divisor's top limb
/// is at least `B / 2`; each quotient digit is estimated from the top two limbs
/// of the running remainder over the divisor's top limb and refined against its
/// second limb, which leaves it at most one too large; the digit's multiple of
/// the divisor is subtracted from the remainder window in one fused pass, and the
/// rare negative result is repaired by adding the divisor back once. The
/// remainder is unscaled by `f` at the end. Every product is below `B² < 2^60`,
/// inside the `u64` lane with its carry.
pub(super) fn magnitude_div_rem(a: &[u32], b: &[u32]) -> (Vec<u32>, Vec<u32>) {
    if magnitude_cmp(a, b) == Ordering::Less {
        return (Vec::new(), a.to_vec());
    }
    if let [single] = b {
        let (quotient, remainder) = magnitude_div_rem_u64(a, u64::from(*single));
        return (quotient, magnitude_limbs(u128::from(remainder)));
    }
    let base = LIMB_BASE;
    let n = b.len();
    let m = a.len() - n;
    let factor = base / (u64::from(b[n - 1]) + 1);
    // u = a·f with one extra top limb; v = b·f, still n limbs (its top < B).
    let mut u = scale_limbs(a, factor, a.len() + 1);
    let v = scale_limbs(b, factor, n);
    let (v_top, v_next) = (u64::from(v[n - 1]), u64::from(v[n - 2]));
    let mut quotient = vec![0_u32; m + 1];
    for j in (0..=m).rev() {
        let numerator = u64::from(u[j + n]) * base + u64::from(u[j + n - 1]);
        let mut digit = numerator / v_top;
        let mut rest = numerator % v_top;
        while digit >= base || digit * v_next > rest * base + u64::from(u[j + n - 2]) {
            digit -= 1;
            rest += v_top;
            if rest >= base {
                break;
            }
        }
        // u[j ..= j + n] −= digit · v.
        let mut carry = 0_u64;
        let mut borrow = 0_u64;
        for (offset, &limb) in v.iter().enumerate() {
            let product = digit * u64::from(limb) + carry;
            carry = product / base;
            let take = product % base + borrow;
            let have = u64::from(u[j + offset]);
            if have >= take {
                u[j + offset] = narrow(have - take);
                borrow = 0;
            } else {
                u[j + offset] = narrow(have + base - take);
                borrow = 1;
            }
        }
        let take = carry + borrow;
        let have = u64::from(u[j + n]);
        if have >= take {
            u[j + n] = narrow(have - take);
        } else {
            // The estimate was one too large: add the divisor back once; the
            // carry out of the top limb cancels the borrow.
            u[j + n] = narrow(have + base - take);
            digit -= 1;
            let mut carry = 0_u64;
            for (offset, &limb) in v.iter().enumerate() {
                let sum = u64::from(u[j + offset]) + u64::from(limb) + carry;
                u[j + offset] = narrow(sum % base);
                carry = sum / base;
            }
            u[j + n] = narrow((u64::from(u[j + n]) + carry) % base);
        }
        quotient[j] = narrow(digit);
    }
    trim(&mut quotient);
    u.truncate(n);
    trim(&mut u);
    let (remainder, leftover) = magnitude_div_rem_u64(&u, factor);
    debug_assert_eq!(
        leftover, 0,
        "the scaled remainder is a multiple of the factor"
    );
    (quotient, remainder)
}

/// `limbs × factor` (`factor < B`) as exactly `len` base-`1e9` limbs, the top
/// ones zero where the product is shorter.
fn scale_limbs(limbs: &[u32], factor: u64, len: usize) -> Vec<u32> {
    let mut out = Vec::with_capacity(len);
    let mut carry = 0_u64;
    for &limb in limbs {
        let product = u64::from(limb) * factor + carry;
        out.push(narrow(product % LIMB_BASE));
        carry = product / LIMB_BASE;
    }
    if carry > 0 {
        out.push(narrow(carry));
    }
    debug_assert!(
        out.len() <= len,
        "the scaled value fits the requested limbs"
    );
    out.resize(len, 0);
    out
}

/// A value already reduced below `1e9`, as a limb.
fn narrow(value: u64) -> u32 {
    debug_assert!(value < LIMB_BASE, "a base-1e9 digit");
    // Below 1e9 < 2^32 by construction at every call site.
    value as u32
}

#[cfg(test)]
mod tests {
    use super::super::BigInt;
    use super::KARATSUBA_THRESHOLD;
    use purrdf_testkit::rng::splitmix64_next;
    use std::fmt::Write as _;

    fn random(state: &mut u64, limbs: usize, negative: bool) -> BigInt {
        let mut text = String::new();
        for _ in 0..limbs {
            write!(text, "{:09}", splitmix64_next(state) % 1_000_000_000).expect("a String");
        }
        if text.is_empty() {
            text.push('0');
        }
        let value = BigInt::from_digits(&text).expect("digits");
        if negative { value.negated() } else { value }
    }

    #[test]
    fn karatsuba_agrees_with_schoolbook_across_the_threshold() {
        let mut state = 0x4B41_5241_u64;
        for (la, lb) in [
            (KARATSUBA_THRESHOLD, KARATSUBA_THRESHOLD),
            (KARATSUBA_THRESHOLD + 1, KARATSUBA_THRESHOLD * 3),
            (200, 200),
            (511, 64),
            (64, 1000),
            (97, 98),
        ] {
            let a = random(&mut state, la, false);
            let b = random(&mut state, lb, true);
            assert_eq!(a.mul_fast(&b), a.mul(&b), "{la} × {lb} limbs");
        }
    }

    #[test]
    fn div_rem_reconstructs_the_dividend() {
        let mut state = 0xD1F0_u64;
        for la in [1_usize, 2, 3, 7, 30, 120] {
            for lb in [1_usize, 2, 3, 9, 40] {
                let a = random(&mut state, la, la % 2 == 0);
                let b = random(&mut state, lb, lb % 3 == 0);
                if b.is_zero() {
                    continue;
                }
                let (q, r) = a.div_rem(&b).expect("nonzero divisor");
                assert_eq!(&(&q * &b) + &r, a, "{la}/{lb}");
                assert!(r.abs() < b.abs(), "|r| < |b|");
                assert!(r.is_zero() || r.is_negative() == a.is_negative());
            }
        }
        assert!(BigInt::one().div_rem(&BigInt::zero()).is_none());
    }

    #[test]
    fn pow_gcd_and_decimal_shapes() {
        assert_eq!(BigInt::from_i128(-3).pow(3), BigInt::from_i128(-27));
        assert_eq!(BigInt::zero().pow(0), BigInt::one());
        assert_eq!(
            BigInt::from_i128(2).pow(127).to_string(),
            (1_u128 << 127).to_string()
        );
        assert_eq!(BigInt::pow10(40).decimal_digits(), 41);
        assert_eq!(BigInt::pow10(40).trailing_decimal_zeros(), 40);
        assert_eq!(BigInt::zero().decimal_digits(), 1);
        let a = BigInt::from_i128(2)
            .pow(200)
            .mul_fast(&BigInt::from_i128(3).pow(50));
        let b = BigInt::from_i128(2)
            .pow(90)
            .mul_fast(&BigInt::from_i128(3).pow(120));
        let g = BigInt::from_i128(2)
            .pow(90)
            .mul_fast(&BigInt::from_i128(3).pow(50));
        assert_eq!(a.gcd(&b.negated()), g);
        let (q, r) = BigInt::from_i128(-123_456_789_012).div_rem_pow10(4);
        assert_eq!(
            (q, r),
            (BigInt::from_i128(-12_345_678), BigInt::from_i128(-9_012))
        );
    }
}
