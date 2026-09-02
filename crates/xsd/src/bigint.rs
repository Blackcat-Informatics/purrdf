// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! [`BigInt`]: an arbitrary-precision signed integer used to accumulate an exact
//! running sum without an `i128`-bounded intermediate overflow poisoning the
//! answer.
//!
//! # Why this exists
//!
//! `XsdValue::Integer` (see [`crate::value::XsdValue`]) is deliberately
//! `i128`-bounded — an individual `xsd:integer` LITERAL beyond that magnitude
//! hard-fails to parse (see the crate's module docs). That is a reasonable bound
//! for one literal. It is NOT a reasonable bound for a running total accumulated
//! across many literals: `xsd:integer`'s value space is genuinely unbounded per
//! XSD, so `SUM`/`AVG` over a group whose individual values each fit comfortably
//! in `i128` can still have a true mathematical total that does not — most
//! simply, TWO values near `i128::MAX` add to a total near `2 × i128::MAX`, which
//! is an entirely ordinary (if large) integer, not an error.
//!
//! [`crate::numeric::Decimal`] cannot serve this role: its mantissa is `i128`
//! too (by the same deliberate, documented bound), so promoting a running
//! integer sum through `Decimal` hits the identical ceiling one type up. A
//! genuine "arbitrary precision" accumulator therefore needs a representation
//! with no fixed width at all — this module is the minimal one: constructed
//! from an `i128`, added to another `BigInt`, narrowed back to `i128` when it
//! fits, and rendered as the canonical `xsd:integer` decimal lexical form when
//! it does not. `SUM` needs only that. `AVG`'s finish (dividing the running sum
//! by the folded row count) needs two more narrow operations once the sum has
//! escaped `i128`: scaling by a power of ten and dividing by a single machine
//! integer (the count) — [`BigInt::mul_pow10`] and [`BigInt::div_rem_u64`],
//! both still exact and both still far short of a general-purpose bignum: no
//! `BigInt × BigInt` multiplication, no `BigInt ÷ BigInt` division, no parsing
//! from a lexical form, because a row count is always a single machine word and
//! nothing in this crate ever needs to multiply two running sums together.
//!
//! # Representation
//!
//! Sign-magnitude: a `negative` flag plus a little-endian `Vec<u32>` of base-`1e9`
//! limbs. Base `1e9` (rather than a binary base) makes [`BigInt::to_decimal_string`]
//! a direct concatenation with no base conversion, and keeps every intermediate
//! limb sum (at most `2 × (1e9 − 1) + 1 < 2^31`) comfortably inside a `u64`
//! carry lane with no overflow reasoning beyond "two `u32`s and a carry fit in a
//! `u64`". Zero is the canonical empty-limb-vector representation (`negative`
//! always `false` for zero), so limb-vector equality is exactly value equality
//! for anything this module constructs (every constructor trims trailing —
//! i.e. most-significant — zero limbs).

use std::cmp::Ordering;
use std::fmt::Write as _;

/// Each limb holds a base-`1e9` digit group.
const LIMB_BASE: u64 = 1_000_000_000;

/// `i128::MIN`'s magnitude (`2^127`) — the one negative magnitude with no
/// positive `i128` counterpart, handled specially by [`BigInt::to_i128`].
const I128_MIN_MAGNITUDE: u128 = 1u128 << 127;

/// An arbitrary-precision signed integer — see the module docs for why this
/// exists and what it deliberately does not support.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BigInt {
    /// `true` for a negative value. Always `false` when `limbs` is empty (zero).
    negative: bool,
    /// Base-`1_000_000_000` limbs, least-significant first, no trailing
    /// (most-significant) zero limb — the canonical form `PartialEq` relies on.
    limbs: Vec<u32>,
}

impl BigInt {
    /// The value zero.
    #[must_use]
    pub const fn zero() -> Self {
        Self {
            negative: false,
            limbs: Vec::new(),
        }
    }

    /// Whether this value is exactly zero.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.limbs.is_empty()
    }

    /// Construct from an `i128`, exactly (every `i128`, including `i128::MIN`, has
    /// an exact `BigInt` representation).
    #[must_use]
    pub fn from_i128(value: i128) -> Self {
        if value == 0 {
            return Self::zero();
        }
        Self {
            negative: value < 0,
            limbs: magnitude_limbs(value.unsigned_abs()),
        }
    }

    /// Construct from a `u128` **magnitude**, exactly — always non-negative.
    ///
    /// [`Self::from_i128`] cannot serve every magnitude this crate needs to compare:
    /// `i128::MIN`'s magnitude is `2^127`, which has no `i128` counterpart at all, and
    /// it is a perfectly ordinary `xsd:decimal` mantissa. The exact numeric order
    /// ([`crate::numeric::numeric_total_cmp`]) compares magnitudes with the signs
    /// already decided, so it needs the unsigned constructor rather than a signed one
    /// it would immediately have to take the absolute value of.
    #[must_use]
    pub fn from_u128(magnitude: u128) -> Self {
        if magnitude == 0 {
            return Self::zero();
        }
        Self {
            negative: false,
            limbs: magnitude_limbs(magnitude),
        }
    }

    /// Add `other` into `self` in place, exactly — this operation alone never
    /// fails or truncates, whatever magnitude the two operands or their sum
    /// reach: that is the entire point of an arbitrary-precision accumulator.
    pub fn add_assign(&mut self, other: &Self) {
        if other.is_zero() {
            return;
        }
        if self.is_zero() {
            self.clone_from(other);
            return;
        }
        if self.negative == other.negative {
            self.limbs = magnitude_add(&self.limbs, &other.limbs);
        } else {
            match magnitude_cmp(&self.limbs, &other.limbs) {
                Ordering::Equal => *self = Self::zero(),
                Ordering::Greater => {
                    self.limbs = magnitude_sub(&self.limbs, &other.limbs);
                }
                Ordering::Less => {
                    self.limbs = magnitude_sub(&other.limbs, &self.limbs);
                    self.negative = other.negative;
                }
            }
        }
    }

    /// Add the `i128` `value` into `self` in place, exactly. A thin convenience
    /// over [`Self::add_assign`] for the common case of folding one more parsed
    /// `xsd:integer` literal (already `i128`-bounded, per this crate's parse
    /// layer) into a running `BigInt` total.
    pub fn add_i128(&mut self, value: i128) {
        self.add_assign(&Self::from_i128(value));
    }

    /// Narrow to `i128` if — and only if — the exact value fits. `None` means
    /// genuinely out of `i128` range, not a truncated/wrapped approximation.
    #[must_use]
    pub fn to_i128(&self) -> Option<i128> {
        let mut magnitude: u128 = 0;
        for &limb in self.limbs.iter().rev() {
            magnitude = magnitude
                .checked_mul(u128::from(LIMB_BASE))?
                .checked_add(u128::from(limb))?;
        }
        if self.negative {
            if magnitude == I128_MIN_MAGNITUDE {
                return Some(i128::MIN);
            }
            i128::try_from(magnitude).ok().map(|m| -m)
        } else {
            i128::try_from(magnitude).ok()
        }
    }

    /// Lossy `f64` approximation, for promoting an out-of-`i128`-range running
    /// integer sum into the IEEE `xsd:float`/`xsd:double` tower once a
    /// float/double value joins the fold. Precision loss here is expected and
    /// correct: `xsd:float`/`xsd:double` are IEEE types, never exact, exactly as
    /// today's `i128 → f64` promotion (`purrdf_xsd::numeric`'s `num_f64`) is
    /// already lossy for a large in-range `i128`.
    #[must_use]
    pub fn to_f64(&self) -> f64 {
        let mut acc = 0.0_f64;
        for &limb in self.limbs.iter().rev() {
            acc = acc.mul_add(LIMB_BASE as f64, f64::from(limb));
        }
        if self.negative { -acc } else { acc }
    }

    /// The XSD 1.1 canonical `xsd:integer` lexical form (§3.3.13
    /// `integerCanonicalMap`): optional `-` sign, then decimal digits with no
    /// leading zero (`"0"` for zero itself).
    #[must_use]
    pub fn to_decimal_string(&self) -> String {
        let digits = magnitude_decimal_digits(&self.limbs);
        if self.negative {
            format!("-{digits}")
        } else {
            digits
        }
    }

    /// The XSD 1.1 canonical `xsd:decimal` lexical form (§3.3.3.2
    /// `decimalCanonicalMap`) of `self` interpreted as a fixed-point mantissa at
    /// `scale` fractional digits — i.e. as if `self` were a
    /// [`crate::numeric::Decimal`]'s mantissa, but with no `i128` bound on the
    /// magnitude. Mirrors `Decimal::canonical_lexical`'s digit-split/trim
    /// algorithm exactly (an integer-valued result has no decimal point; a
    /// fractional one keeps its fractional part with trailing zeros trimmed).
    ///
    /// Used only by [`crate::numeric::bigint_avg_decimal_lexical`] — `AVG`'s
    /// finish once the scale-18 quotient mantissa has ALSO escaped `i128` (not
    /// just the running sum that produced it) — the identical TEXT-rendering
    /// bypass this module's own [`Self::to_decimal_string`] already gives
    /// `SUM`'s finish for a pure-integer running total that exceeds `i128`.
    #[must_use]
    pub fn to_decimal_lexical(&self, scale: u8) -> String {
        let digits = magnitude_decimal_digits(&self.limbs);
        let scale = usize::from(scale);
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
        let sign = if self.negative { "-" } else { "" };
        if frac_trimmed.is_empty() {
            format!("{sign}{int_part}")
        } else {
            format!("{sign}{int_part}.{frac_trimmed}")
        }
    }

    /// Whether this value is strictly negative (`false` for zero).
    #[must_use]
    pub fn is_negative(&self) -> bool {
        self.negative
    }

    /// `self × 10^exp`, exactly — used by `AVG`'s finish to scale a running sum
    /// up to `purrdf_xsd::numeric`'s `MAX_DECIMAL_SCALE` fractional digits BEFORE
    /// dividing by the folded row count (see [`Self::div_rem_u64`]), mirroring
    /// `decimal_div_raw`'s own scale-then-divide shape but over an
    /// arbitrary-precision dividend.
    #[must_use]
    pub fn mul_pow10(&self, exp: u32) -> Self {
        if self.is_zero() || exp == 0 {
            return self.clone();
        }
        let whole_limb_shift = usize::try_from(exp / 9).unwrap_or(usize::MAX);
        let leftover = exp % 9;
        let mut limbs = vec![0u32; whole_limb_shift];
        limbs.extend_from_slice(&self.limbs);
        if leftover > 0 {
            // `leftover < 9`, so `10^leftover <= 1e8`, comfortably inside `u64`.
            let factor = 10u64.pow(leftover);
            limbs = mul_by_small(&limbs, factor);
        }
        Self {
            negative: self.negative,
            limbs,
        }
    }

    /// `self × 2^exp`, exactly — the binary counterpart of [`Self::mul_pow10`].
    ///
    /// The decimal branch of the numeric tower is `mantissa / 10^scale`; the IEEE
    /// branch (`xsd:float`, `xsd:double`) is a **dyadic** rational,
    /// `significand × 2^exponent`, because every finite IEEE value is one exactly.
    /// Putting the two branches on a common footing to compare them without
    /// rounding therefore means scaling one side by a power of TWO, which a
    /// base-`1e9` representation cannot express as a limb shift the way
    /// [`Self::mul_pow10`] can. See [`crate::numeric::numeric_total_cmp`] for the
    /// order this exists to make exact.
    ///
    /// `2^29 < LIMB_BASE`, so the scaling runs in 29-bit chunks through the same
    /// single-limb multiply [`Self::mul_pow10`] uses, and every intermediate stays
    /// inside the `u64` carry lane that multiply already argues for.
    #[must_use]
    pub fn mul_pow2(&self, exp: u32) -> Self {
        if self.is_zero() || exp == 0 {
            return self.clone();
        }
        // 29 bits at a time: `2^29 = 536_870_912 < LIMB_BASE`, so each chunk is a
        // legal `mul_by_small` factor.
        const CHUNK: u32 = 29;
        let mut limbs = self.limbs.clone();
        let mut remaining = exp;
        while remaining > 0 {
            let step = remaining.min(CHUNK);
            limbs = mul_by_small(&limbs, 1u64 << step);
            remaining -= step;
        }
        Self {
            negative: self.negative,
            limbs,
        }
    }

    /// Divide by the positive machine integer `divisor`, truncating toward zero
    /// exactly as integer division does, returning `(quotient, |remainder|)`.
    /// `None` only for `divisor == 0` — `AVG`'s one caller always passes a
    /// folded row COUNT, which is never zero for a non-empty group.
    #[must_use]
    pub fn div_rem_u64(&self, divisor: u64) -> Option<(Self, u64)> {
        if divisor == 0 {
            return None;
        }
        let (quotient_limbs, remainder) = magnitude_div_rem_u64(&self.limbs, divisor);
        let quotient = Self {
            negative: self.negative && !quotient_limbs.is_empty(),
            limbs: quotient_limbs,
        };
        Some((quotient, remainder))
    }
}

/// The full arithmetic order on the value — sign first, then magnitude.
///
/// This is a genuine total order over the values this type represents, and it agrees
/// with the derived `PartialEq` because the representation is canonical (zero is the
/// empty limb vector with `negative == false`, and every constructor trims
/// most-significant zero limbs), so two `BigInt`s compare `Equal` exactly when they
/// are structurally equal. Providing it is safe in the way an `Ord` on
/// [`crate::XsdValue`] would NOT be: an integer has one value space and one order,
/// with no lexical form to conflate it with and no cross-datatype promotion to hide.
impl Ord for BigInt {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.negative, other.negative) {
            (false, true) => Ordering::Greater,
            (true, false) => Ordering::Less,
            (false, false) => magnitude_cmp(&self.limbs, &other.limbs),
            // Both negative: the larger magnitude is the smaller value.
            (true, true) => magnitude_cmp(&other.limbs, &self.limbs),
        }
    }
}

impl PartialOrd for BigInt {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// The canonical base-`1e9` limbs of a non-zero `u128` magnitude, least-significant
/// first. Shared by [`BigInt::from_i128`] and [`BigInt::from_u128`], which differ only
/// in where the sign comes from.
fn magnitude_limbs(mut magnitude: u128) -> Vec<u32> {
    let mut limbs = Vec::new();
    while magnitude > 0 {
        // `magnitude % LIMB_BASE < 1e9`, which fits `u32` with headroom to spare.
        limbs.push((magnitude % u128::from(LIMB_BASE)) as u32);
        magnitude /= u128::from(LIMB_BASE);
    }
    limbs
}

/// Render a canonical (no trailing zero limb) magnitude as unsigned decimal digits,
/// with no leading zero (`"0"` for the empty/zero magnitude). Shared by
/// [`BigInt::to_decimal_string`] and [`BigInt::to_decimal_lexical`], which differ
/// only in the sign placement and in whether a decimal point is spliced in.
fn magnitude_decimal_digits(limbs: &[u32]) -> String {
    let Some((most_significant, rest)) = limbs.split_last() else {
        return "0".to_string();
    };
    let mut out = String::with_capacity(limbs.len() * 9);
    write!(out, "{most_significant}").expect("writing to a String cannot fail");
    for limb in rest.iter().rev() {
        write!(out, "{limb:09}").expect("writing to a String cannot fail");
    }
    out
}

/// Compare two canonical (no trailing zero limb) magnitudes.
fn magnitude_cmp(a: &[u32], b: &[u32]) -> Ordering {
    if a.len() != b.len() {
        return a.len().cmp(&b.len());
    }
    for (x, y) in a.iter().rev().zip(b.iter().rev()) {
        if x != y {
            return x.cmp(y);
        }
    }
    Ordering::Equal
}

/// `a + b` over base-`1e9` magnitudes (both little-endian, canonical).
fn magnitude_add(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut out = Vec::with_capacity(a.len().max(b.len()) + 1);
    let mut carry: u64 = 0;
    for i in 0..a.len().max(b.len()) {
        let x = u64::from(a.get(i).copied().unwrap_or(0));
        let y = u64::from(b.get(i).copied().unwrap_or(0));
        let sum = x + y + carry;
        // `sum % LIMB_BASE < 1e9`, which fits `u32` with headroom to spare.
        out.push((sum % LIMB_BASE) as u32);
        carry = sum / LIMB_BASE;
    }
    if carry > 0 {
        // A carry out of a two-limb-plus-carry sum is `< LIMB_BASE`, which fits `u32`.
        out.push(carry as u32);
    }
    out
}

/// `a - b` over base-`1e9` magnitudes, requiring `a >= b` (per
/// [`magnitude_cmp`]) — the caller always checks before calling. Returns the
/// canonical (trailing-zero-limb-trimmed) result.
fn magnitude_sub(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut out = Vec::with_capacity(a.len());
    let mut borrow: i64 = 0;
    for (i, &a_limb) in a.iter().enumerate() {
        let x = i64::from(a_limb);
        let y = i64::from(b.get(i).copied().unwrap_or(0));
        let mut diff = x - y - borrow;
        if diff < 0 {
            diff += i64::try_from(LIMB_BASE).expect("LIMB_BASE fits i64 with vast headroom");
            borrow = 1;
        } else {
            borrow = 0;
        }
        // `diff` is in `[0, LIMB_BASE)` here by construction, so it fits `u32` exactly.
        out.push(diff as u32);
    }
    while out.last() == Some(&0) {
        out.pop();
    }
    out
}

/// `a × factor` over a base-`1e9` magnitude, where `factor < LIMB_BASE` (the only
/// case [`BigInt::mul_pow10`] needs — a single leftover decimal digit's worth of
/// scaling after the whole-limb shift). Each limb's product is
/// `< LIMB_BASE × LIMB_BASE < 2^60`, comfortably inside `u64` alongside the carry.
fn mul_by_small(a: &[u32], factor: u64) -> Vec<u32> {
    let mut out = Vec::with_capacity(a.len() + 1);
    let mut carry: u64 = 0;
    for &limb in a {
        let product = u64::from(limb) * factor + carry;
        out.push((product % LIMB_BASE) as u32);
        carry = product / LIMB_BASE;
    }
    while carry > 0 {
        out.push((carry % LIMB_BASE) as u32);
        carry /= LIMB_BASE;
    }
    while out.last() == Some(&0) {
        out.pop();
    }
    out
}

/// `a ÷ divisor` over a base-`1e9` magnitude, `divisor > 0` (the caller checks).
/// Standard schoolbook single-limb-divisor long division, most significant limb
/// first: at each step the running remainder satisfies `remainder < divisor`
/// (the loop invariant), so `cur = remainder × LIMB_BASE + limb < divisor ×
/// LIMB_BASE`, which makes the quotient digit `cur / divisor < LIMB_BASE` —
/// always exactly one base-`1e9` digit, however large `divisor` is. Returns the
/// canonical (trailing-zero-limb-trimmed) quotient and the final remainder.
fn magnitude_div_rem_u64(a: &[u32], divisor: u64) -> (Vec<u32>, u64) {
    let mut quotient = vec![0u32; a.len()];
    let mut remainder: u128 = 0;
    let divisor = u128::from(divisor);
    for i in (0..a.len()).rev() {
        let cur = remainder * u128::from(LIMB_BASE) + u128::from(a[i]);
        quotient[i] = (cur / divisor) as u32;
        remainder = cur % divisor;
    }
    while quotient.last() == Some(&0) {
        quotient.pop();
    }
    (
        quotient,
        u64::try_from(remainder).expect("remainder < divisor <= u64::MAX"),
    )
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
        // Integer-valued: no decimal point, matching XSD 1.1 §3.3.3.2.
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

    #[test]
    fn to_f64_is_a_lossy_but_reasonable_approximation() {
        let mut sum = BigInt::from_i128(i128::MAX);
        sum.add_i128(i128::MAX);
        let approx = sum.to_f64();
        let exact = 2.0_f64 * (i128::MAX as f64);
        assert!((approx - exact).abs() / exact < 1e-9);
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
}
