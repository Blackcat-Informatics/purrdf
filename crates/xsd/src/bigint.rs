// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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

    /// The correctly rounded `f64` value (IEEE 754 round-to-nearest,
    /// ties-to-even) — the conversion XPath `xs:double` casting and SPARQL's
    /// `integer ⊂ double` promotion require when an out-of-`i128`-range running
    /// integer sum joins the `xsd:double` tower. A magnitude that rounds past
    /// `f64::MAX` (at or above `2^1024 − 2^970`) is `±∞`, as IEEE overflow
    /// under round-to-nearest gives; zero is `+0.0`.
    ///
    /// The algorithm rounds ONCE: the magnitude is converted exactly to binary,
    /// its top 64 significant bits are taken with a sticky bit OR-ed into the
    /// lowest of them for any nonzero bit below (that bit sits far under the
    /// 53-bit rounding position, so it only breaks ties, never moves one), the
    /// resulting `u64` is rounded to 53 bits by the correctly rounded integer
    /// cast, and the power-of-two scale is applied by an exact multiplication.
    /// An earlier revision folded the base-`1e9` limbs through a Horner loop,
    /// rounding at every limb, which could land one ulp away from the correctly
    /// rounded value.
    #[must_use]
    pub fn to_f64(&self) -> f64 {
        let magnitude = match binary_top64(&self.limbs) {
            BinaryTop::Small(value) => value as f64,
            BinaryTop::Scaled { top, shift } => {
                // `top`'s bit 63 is set, so `top as f64` lies in `[2^63, 2^64]`
                // and `× 2^shift` is exact unless it overflows — and an overflow
                // there is exactly IEEE's round-to-nearest overflow to `∞`. The
                // product goes through `ieee::f64_mul` so that both the rounded
                // `top` and the product are binary64 values on every target: on the
                // x87 the bare expression keeps `top` unrounded and the product at
                // the register's exponent range, so a tie that overflows to `∞`
                // stays finite.
                if shift > 1024 - 64 {
                    f64::INFINITY
                } else {
                    crate::ieee::f64_mul(top as f64, pow2_f64(shift))
                }
            }
        };
        if self.negative { -magnitude } else { magnitude }
    }

    /// The correctly rounded `f32` value (round-to-nearest, ties-to-even) — the
    /// `integer ⊂ float` twin of [`Self::to_f64`], by the same single-rounding
    /// algorithm at 24 bits. Converting through [`Self::to_f64`] and narrowing
    /// would round twice, which can differ from the correctly rounded value when
    /// the first rounding lands exactly on an `f32` halfway point.
    #[must_use]
    pub fn to_f32(&self) -> f32 {
        let magnitude = match binary_top64(&self.limbs) {
            BinaryTop::Small(value) => value as f32,
            BinaryTop::Scaled { top, shift } => {
                if shift > 128 - 64 {
                    f32::INFINITY
                } else {
                    crate::ieee::f32_mul(top as f32, pow2_f32(shift))
                }
            }
        };
        if self.negative { -magnitude } else { magnitude }
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

/// A magnitude's binary form as far as a correctly rounded float conversion
/// needs it: either the whole value (it fits `u128`, whose casts to `f64`/`f32`
/// are themselves correctly rounded), or its top 64 significant bits with a
/// sticky bit for everything below, plus the power of two they are scaled by.
enum BinaryTop {
    /// The exact magnitude.
    Small(u128),
    /// `value ≈ top × 2^shift`: `top` has bit 63 set, and its bit 0 is OR-ed
    /// with "some discarded lower bit was nonzero" so that a round-to-nearest of
    /// `top` to at most 53 bits rounds exactly as the full value would.
    Scaled { top: u64, shift: u32 },
}

/// Convert base-`1e9` limbs (least significant first) into [`BinaryTop`], exactly.
fn binary_top64(limbs: &[u32]) -> BinaryTop {
    // Exact base conversion into little-endian base-2^64 words:
    // `words = words × 1e9 + limb`, most significant limb first. Each word's
    // product plus carry is `< 2^64 × 1e9 + 2^64 < 2^128`, so the `u128` lane
    // never overflows.
    let mut words: Vec<u64> = Vec::with_capacity(limbs.len() / 2 + 1);
    for &limb in limbs.iter().rev() {
        let mut carry = u128::from(limb);
        for word in &mut words {
            let product = u128::from(*word) * u128::from(LIMB_BASE) + carry;
            // Low 64 bits of the product; the high half carries on.
            *word = product as u64;
            carry = product >> 64;
        }
        if carry > 0 {
            // `carry < 2^64` by the bound above.
            words.push(carry as u64);
        }
    }
    match words.as_slice() {
        [] => BinaryTop::Small(0),
        [low] => BinaryTop::Small(u128::from(*low)),
        [low, high] => BinaryTop::Small((u128::from(*high) << 64) | u128::from(*low)),
        [lower @ .., next, top_word] => {
            // `top_word != 0` (the conversion never pushes a zero word).
            let lead = top_word.leading_zeros();
            let top = if lead == 0 {
                *top_word
            } else {
                (top_word << lead) | (next >> (64 - lead))
            };
            let dropped_from_next = if lead == 0 { *next } else { next << lead };
            let sticky = dropped_from_next != 0 || lower.iter().any(|&word| word != 0);
            // Bit length is `64 × (len − 1) + (64 − lead)`; the top 64 bits sit
            // `bit_length − 64` places up. `words.len() ≤ u32::MAX / 64` for any
            // magnitude this crate can build, so the product cannot overflow.
            let word_count = u32::try_from(lower.len() + 2).unwrap_or(u32::MAX);
            let shift = 64 * (word_count - 1) - lead;
            BinaryTop::Scaled {
                top: top | u64::from(sticky),
                shift,
            }
        }
    }
}

/// `2^exp` as an `f64`, exactly, for `exp ≤ 1023`.
fn pow2_f64(exp: u32) -> f64 {
    debug_assert!(exp <= 1023, "2^{exp} is not a finite f64");
    f64::from_bits((u64::from(exp) + 1023) << 52)
}

/// `2^exp` as an `f32`, exactly, for `exp ≤ 127`.
fn pow2_f32(exp: u32) -> f32 {
    debug_assert!(exp <= 127, "2^{exp} is not a finite f32");
    f32::from_bits((exp + 127) << 23)
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

    // A deterministic SplitMix64 counter stream for the float-conversion
    // tests — this zero-dependency crate ships no RNG; the workspace's
    // shared test-only stream comes in through `purrdf-testkit`, a
    // dev-dependency.
    use purrdf_testkit::rng::splitmix64_next as splitmix64;

    /// The exact `BigInt` `Σ words[i] × 2^(64 i)`, negated when `negative`.
    fn from_words(words: &[u64], negative: bool) -> BigInt {
        let mut acc = BigInt::zero();
        for &word in words.iter().rev() {
            acc = acc.mul_pow2(64);
            acc.add_assign(&BigInt::from_u128(u128::from(word)));
        }
        if negative && !acc.is_zero() {
            acc.negative = true;
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
        for &limb in value.limbs.iter().rev() {
            acc = acc.mul_add(super::LIMB_BASE as f64, f64::from(limb));
        }
        if value.negative { -acc } else { acc }
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
        negative.negative = true;
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
}
