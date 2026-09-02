// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Exact, float-free integer and rational arithmetic.
//!
//! [`Int`] is an arbitrary-precision signed integer and [`Rat`] is an exact
//! rational built on top of it. Together they are the number tower every
//! geometric predicate in this crate decides on: a WKT coordinate parses into a
//! [`Rat`] and stays a [`Rat`] through every orientation test, intersection and
//! DE-9IM decision.
//!
//! # Zero floating-point arithmetic
//!
//! **No function in this module performs floating-point arithmetic.** There is
//! no `+`, `-`, `*` or `/` applied to an `f32` or an `f64` anywhere below. The
//! single place an `f64` value is produced at all is [`Rat::to_f64`], and it
//! does not compute that value with float operations: it derives the sign,
//! biased exponent and 52-bit trailing significand with integer arithmetic on
//! [`Int`] magnitudes, packs them into a `u64` bit pattern, and hands that
//! pattern to [`f64::from_bits`].
//!
//! This is the crate's central determinism guarantee, and it is a guarantee
//! about *portability*, not about accuracy. Floating-point results can differ
//! between hosts for reasons that are entirely legal: an x86-64 build may
//! contract a multiply and an add into a single fused `mulsd`/`fma` with one
//! rounding instead of two, an LLVM pass may reassociate a sum, a
//! `wasm32-unknown-unknown` build may lower the same expression differently
//! again, and the rounding mode is ambient global state that the host, not this
//! library, controls. Any of those turns a `>= 0` orientation test into a `< 0`
//! one for a nearly-degenerate triangle, and a geometry engine that decides
//! topology from such a test will report different topology on different hosts
//! for the same input bytes. PurRDF is a data carrier: one engine, one
//! behaviour, carried verbatim into Rust, Python, WebAssembly and C. So every
//! geometric *decision* is made in integer arithmetic, where the result is
//! defined by the values alone and is bit-for-bit identical on every target by
//! construction. [`Rat::to_f64`] exists only for the boundary where a caller
//! demands an `xsd:double`, and it is correctly rounded (round-half-to-even) so
//! that even the boundary is a pure function of the exact value.
//!
//! # Representation
//!
//! Both types keep exactly one representation of every value, so `PartialEq`,
//! `Eq` and `Hash` can be derived and two values that compare equal are
//! interchangeable everywhere:
//!
//! * [`Int`] stores a little-endian base-2<sup>64</sup> magnitude with no
//!   trailing zero limbs, and its sign flag is never set for zero — there is no
//!   negative zero.
//! * [`Rat`] stores a strictly positive denominator and keeps numerator and
//!   denominator coprime; zero is exactly `0/1`.

use core::cmp::Ordering;
use core::fmt;

use smallvec::{SmallVec, smallvec};

/// Little-endian base-2<sup>64</sup> magnitude, least significant limb first.
///
/// Three inline limbs cover every value below 2<sup>192</sup>, which is where
/// the products of realistic coordinate arithmetic live, so the common case
/// never reaches the allocator.
type Mag = SmallVec<[u64; 3]>;

/// Number of decimal digits processed per limb-sized chunk.
///
/// 10<sup>19</sup> is the largest power of ten that fits in a `u64`.
const DECIMAL_CHUNK: usize = 19;

/// Powers of ten that fit in a `u64`, indexed by exponent.
const POW10_U64: [u64; DECIMAL_CHUNK + 1] = [
    1,
    10,
    100,
    1_000,
    10_000,
    100_000,
    1_000_000,
    10_000_000,
    100_000_000,
    1_000_000_000,
    10_000_000_000,
    100_000_000_000,
    1_000_000_000_000,
    10_000_000_000_000,
    100_000_000_000_000,
    1_000_000_000_000_000,
    10_000_000_000_000_000,
    100_000_000_000_000_000,
    1_000_000_000_000_000_000,
    10_000_000_000_000_000_000,
];

/// The base each decimal chunk multiplies the accumulator by (10<sup>19</sup>).
const CHUNK_BASE: u64 = POW10_U64[DECIMAL_CHUNK];

/// Largest absolute decimal exponent [`Rat::parse_decimal`] will accept.
///
/// See [`Rat::parse_decimal`] for why this cap exists and why it is the only
/// refusal that parser makes on an otherwise well-formed lexical form.
const MAX_DECIMAL_EXPONENT: i64 = 100_000;

/// `f64` bit pattern of a positive infinity, sign bit clear.
const F64_INFINITY_BITS: u64 = 0x7ff0_0000_0000_0000;

// ---------------------------------------------------------------------------
// Magnitude primitives
//
// Every helper below takes and returns a canonical magnitude: little-endian,
// no trailing zero limbs, empty means zero.
// ---------------------------------------------------------------------------

/// Drops trailing zero limbs so the magnitude is canonical.
fn mag_trim(m: &mut Mag) {
    while m.last() == Some(&0) {
        m.pop();
    }
}

/// Builds a magnitude from a `u64`.
fn mag_from_u64(v: u64) -> Mag {
    if v == 0 { Mag::new() } else { smallvec![v] }
}

/// Builds a magnitude from a `u128`.
fn mag_from_u128(v: u128) -> Mag {
    let lo = v as u64;
    let hi = (v >> 64) as u64;
    if hi == 0 {
        mag_from_u64(lo)
    } else {
        smallvec![lo, hi]
    }
}

/// Returns the magnitude as a `u128`, or `None` if it needs more than 128 bits.
fn mag_to_u128(a: &[u64]) -> Option<u128> {
    match a.len() {
        0 => Some(0),
        1 => Some(u128::from(a[0])),
        2 => Some(u128::from(a[0]) | (u128::from(a[1]) << 64)),
        _ => None,
    }
}

/// Returns `true` when the magnitude is exactly one.
fn mag_is_one(a: &[u64]) -> bool {
    a.len() == 1 && a[0] == 1
}

/// Compares two magnitudes numerically.
fn mag_cmp(a: &[u64], b: &[u64]) -> Ordering {
    a.len()
        .cmp(&b.len())
        .then_with(|| a.iter().rev().cmp(b.iter().rev()))
}

/// Number of bits in the magnitude; zero has zero bits.
fn mag_bit_len(a: &[u64]) -> u64 {
    match a.last() {
        None => 0,
        Some(&top) => (a.len() as u64 - 1) * 64 + u64::from(64 - top.leading_zeros()),
    }
}

/// Returns bit `index` of the magnitude, counting from the least significant.
fn mag_bit(a: &[u64], index: u64) -> bool {
    let limb = (index / 64) as usize;
    limb < a.len() && (a[limb] >> (index % 64)) & 1 == 1
}

/// Number of trailing zero bits; zero (which has none) reports zero.
fn mag_trailing_zeros(a: &[u64]) -> u64 {
    match a.iter().position(|&limb| limb != 0) {
        None => 0,
        Some(i) => (i as u64) * 64 + u64::from(a[i].trailing_zeros()),
    }
}

/// Schoolbook addition of two magnitudes.
fn mag_add(a: &[u64], b: &[u64]) -> Mag {
    let (long, short) = if a.len() >= b.len() { (a, b) } else { (b, a) };
    let mut out: Mag = SmallVec::with_capacity(long.len() + 1);
    let mut carry = 0u64;
    for (i, &la) in long.iter().enumerate() {
        let lb = short.get(i).copied().unwrap_or(0);
        let sum = u128::from(la) + u128::from(lb) + u128::from(carry);
        out.push(sum as u64);
        carry = (sum >> 64) as u64;
    }
    if carry != 0 {
        out.push(carry);
    }
    out
}

/// Schoolbook subtraction of two magnitudes; `a` must be at least `b`.
fn mag_sub(a: &[u64], b: &[u64]) -> Mag {
    debug_assert!(mag_cmp(a, b) != Ordering::Less, "mag_sub would go negative");
    let mut out: Mag = SmallVec::with_capacity(a.len());
    let mut borrow = 0u64;
    for (i, &la) in a.iter().enumerate() {
        let lb = b.get(i).copied().unwrap_or(0);
        // `la - lb` underflowing leaves a value of at least 1, so subtracting a
        // borrow of at most 1 from it cannot underflow a second time.
        let (partial, borrowed_a) = la.overflowing_sub(lb);
        let (limb, borrowed_b) = partial.overflowing_sub(borrow);
        out.push(limb);
        borrow = u64::from(borrowed_a || borrowed_b);
    }
    debug_assert_eq!(borrow, 0, "mag_sub called with a < b");
    mag_trim(&mut out);
    out
}

/// Schoolbook multiplication of two magnitudes.
fn mag_mul(a: &[u64], b: &[u64]) -> Mag {
    if a.is_empty() || b.is_empty() {
        return Mag::new();
    }
    if let (Some(x), Some(y)) = (mag_to_u64(a), mag_to_u64(b)) {
        // Fast path: the whole product fits in a `u128`. This is the hot path
        // for coordinate arithmetic, where operands are small integers.
        return mag_from_u128(u128::from(x) * u128::from(y));
    }
    let mut out: Mag = smallvec![0u64; a.len() + b.len()];
    for (i, &ai) in a.iter().enumerate() {
        if ai == 0 {
            continue;
        }
        let mut carry = 0u64;
        for (j, &bj) in b.iter().enumerate() {
            let idx = i + j;
            let cur = u128::from(ai) * u128::from(bj) + u128::from(out[idx]) + u128::from(carry);
            out[idx] = cur as u64;
            carry = (cur >> 64) as u64;
        }
        // Row `i` is the first writer of limb `i + b.len()`: row `i - 1` stopped
        // at `i + b.len() - 1`. So the carry can be stored, not accumulated.
        out[i + b.len()] = carry;
    }
    mag_trim(&mut out);
    out
}

/// Returns the magnitude as a `u64`, or `None` if it needs more than 64 bits.
fn mag_to_u64(a: &[u64]) -> Option<u64> {
    match a.len() {
        0 => Some(0),
        1 => Some(a[0]),
        _ => None,
    }
}

/// Computes `a * multiplier + addend` for a single-limb multiplier and addend.
fn mag_mul_small_add(a: &[u64], multiplier: u64, addend: u64) -> Mag {
    let mut out: Mag = SmallVec::with_capacity(a.len() + 1);
    let mut carry = u128::from(addend);
    for &limb in a {
        let cur = u128::from(limb) * u128::from(multiplier) + carry;
        out.push(cur as u64);
        carry = cur >> 64;
    }
    while carry != 0 {
        out.push(carry as u64);
        carry >>= 64;
    }
    mag_trim(&mut out);
    out
}

/// Divides a magnitude by a single non-zero limb, returning quotient and remainder.
fn mag_divmod_small(a: &[u64], divisor: u64) -> (Mag, u64) {
    debug_assert_ne!(divisor, 0, "division by zero");
    let d = u128::from(divisor);
    let mut out: Mag = smallvec![0u64; a.len()];
    let mut rem: u128 = 0;
    for (slot, &limb) in out.iter_mut().zip(a.iter()).rev() {
        let cur = (rem << 64) | u128::from(limb);
        *slot = (cur / d) as u64;
        rem = cur % d;
    }
    mag_trim(&mut out);
    (out, rem as u64)
}

/// Shifts a magnitude left by `bits`.
fn mag_shl(a: &[u64], bits: u64) -> Mag {
    if a.is_empty() {
        return Mag::new();
    }
    let limb_shift = (bits / 64) as usize;
    let bit_shift = (bits % 64) as u32;
    let mut out: Mag = SmallVec::with_capacity(a.len() + limb_shift + 1);
    out.resize(limb_shift, 0);
    if bit_shift == 0 {
        out.extend_from_slice(a);
    } else {
        let mut carry = 0u64;
        for &limb in a {
            out.push((limb << bit_shift) | carry);
            carry = limb >> (64 - bit_shift);
        }
        if carry != 0 {
            out.push(carry);
        }
    }
    out
}

/// Shifts a magnitude right by `bits`, discarding the bits shifted out.
fn mag_shr(a: &[u64], bits: u64) -> Mag {
    let limb_shift = (bits / 64) as usize;
    if limb_shift >= a.len() {
        return Mag::new();
    }
    let rest = &a[limb_shift..];
    let bit_shift = (bits % 64) as u32;
    if bit_shift == 0 {
        return SmallVec::from_slice(rest);
    }
    let mut out: Mag = SmallVec::with_capacity(rest.len());
    let mut carry = 0u64;
    for &limb in rest.iter().rev() {
        out.push((limb >> bit_shift) | carry);
        carry = limb << (64 - bit_shift);
    }
    out.reverse();
    mag_trim(&mut out);
    out
}

/// Shifts a magnitude left by exactly one bit, in place.
fn mag_shl1_in_place(m: &mut Mag) {
    let mut carry = 0u64;
    for limb in &mut *m {
        let next = *limb >> 63;
        *limb = (*limb << 1) | carry;
        carry = next;
    }
    if carry != 0 {
        m.push(carry);
    }
}

/// Divides magnitudes, returning `(quotient, remainder)`; `b` must be non-zero.
///
/// The general case is binary shift-and-subtract long division: one iteration
/// per bit of the dividend, each iteration a compare and at most one subtract
/// against the divisor. It is `O(bits(a) * limbs(b))`, which is slower than
/// Knuth's algorithm D but short enough to read and check by eye — and the
/// single-limb and `u128` fast paths above it cover every operand size real
/// coordinate arithmetic produces.
fn mag_div_rem(a: &[u64], b: &[u64]) -> (Mag, Mag) {
    debug_assert!(!b.is_empty(), "division by zero");
    if mag_cmp(a, b) == Ordering::Less {
        return (Mag::new(), SmallVec::from_slice(a));
    }
    if b.len() == 1 {
        let (q, r) = mag_divmod_small(a, b[0]);
        return (q, mag_from_u64(r));
    }
    if let (Some(x), Some(y)) = (mag_to_u128(a), mag_to_u128(b)) {
        return (mag_from_u128(x / y), mag_from_u128(x % y));
    }
    let mut quotient: Mag = smallvec![0u64; a.len()];
    let mut rem: Mag = Mag::new();
    let mut i = mag_bit_len(a);
    while i > 0 {
        i -= 1;
        mag_shl1_in_place(&mut rem);
        if mag_bit(a, i) {
            if rem.is_empty() {
                rem.push(1);
            } else {
                rem[0] |= 1;
            }
        }
        if mag_cmp(&rem, b) != Ordering::Less {
            rem = mag_sub(&rem, b);
            quotient[(i / 64) as usize] |= 1u64 << (i % 64);
        }
    }
    mag_trim(&mut quotient);
    mag_trim(&mut rem);
    (quotient, rem)
}

/// Euclidean greatest common divisor on `u128`.
fn gcd_u128(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

/// Stein's binary greatest common divisor on magnitudes.
fn mag_gcd(a: &[u64], b: &[u64]) -> Mag {
    if a.is_empty() {
        return SmallVec::from_slice(b);
    }
    if b.is_empty() {
        return SmallVec::from_slice(a);
    }
    if mag_is_one(a) || mag_is_one(b) {
        return smallvec![1];
    }
    if let (Some(x), Some(y)) = (mag_to_u128(a), mag_to_u128(b)) {
        return mag_from_u128(gcd_u128(x, y));
    }
    let common = mag_trailing_zeros(a).min(mag_trailing_zeros(b));
    let mut u = mag_shr(a, mag_trailing_zeros(a));
    let mut v: Mag = SmallVec::from_slice(b);
    loop {
        v = mag_shr(&v, mag_trailing_zeros(&v));
        if mag_cmp(&u, &v) == Ordering::Greater {
            core::mem::swap(&mut u, &mut v);
        }
        v = mag_sub(&v, &u);
        if v.is_empty() {
            break;
        }
    }
    mag_shl(&u, common)
}

/// Integer floor square root of a `u128`.
fn isqrt_u128(n: u128) -> u128 {
    if n == 0 {
        return 0;
    }
    let bits = 128 - n.leading_zeros();
    // `x0 = 2^ceil(bits/2) >= sqrt(n)`, which is the precondition integer
    // Newton needs; it is also at most `2 * sqrt(n)`, so the loop converges in
    // a handful of steps.
    let mut x = 1u128 << bits.div_ceil(2);
    loop {
        let y = u128::midpoint(x, n / x);
        if y >= x {
            return x;
        }
        x = y;
    }
}

/// Accumulates a run of ASCII digit bytes into a magnitude.
///
/// The caller must have already established that every byte is an ASCII digit.
fn mag_from_digit_bytes(digits: impl Iterator<Item = u8>) -> Mag {
    let mut acc = Mag::new();
    let mut chunk_value = 0u64;
    let mut chunk_len = 0usize;
    for byte in digits {
        debug_assert!(byte.is_ascii_digit(), "non-digit reached the accumulator");
        chunk_value = chunk_value * 10 + u64::from(byte - b'0');
        chunk_len += 1;
        if chunk_len == DECIMAL_CHUNK {
            acc = mag_mul_small_add(&acc, CHUNK_BASE, chunk_value);
            chunk_value = 0;
            chunk_len = 0;
        }
    }
    if chunk_len > 0 {
        acc = mag_mul_small_add(&acc, POW10_U64[chunk_len], chunk_value);
    }
    acc
}

// ---------------------------------------------------------------------------
// Int
// ---------------------------------------------------------------------------

/// An arbitrary-precision signed integer.
///
/// The magnitude is little-endian base-2<sup>64</sup> with no trailing zero
/// limbs, an empty magnitude is zero, and the sign flag is never set for zero.
/// That gives exactly one representation per value, so `PartialEq`, `Eq` and
/// `Hash` are derived and agree with the numeric relations.
///
/// All arithmetic is exact and allocation-bounded; nothing here rounds, wraps,
/// saturates or touches a float.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Int {
    /// Sign flag; always `false` when the magnitude is empty.
    negative: bool,
    /// Little-endian base-2^64 magnitude with no trailing zero limbs.
    magnitude: Mag,
}

impl Int {
    /// The value zero.
    #[must_use]
    pub fn zero() -> Self {
        Self {
            negative: false,
            magnitude: Mag::new(),
        }
    }

    /// The value one.
    #[must_use]
    pub fn one() -> Self {
        Self {
            negative: false,
            magnitude: smallvec![1],
        }
    }

    /// Builds an integer from an `i64`.
    #[must_use]
    pub fn from_i64(value: i64) -> Self {
        Self::from_parts(value < 0, mag_from_u64(value.unsigned_abs()))
    }

    /// Builds an integer from an `i128`.
    #[must_use]
    pub fn from_i128(value: i128) -> Self {
        Self::from_parts(value < 0, mag_from_u128(value.unsigned_abs()))
    }

    /// Builds a non-negative integer from a `u64`.
    #[must_use]
    pub fn from_u64(value: u64) -> Self {
        Self::from_parts(false, mag_from_u64(value))
    }

    /// Returns `true` when this is zero.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.magnitude.is_empty()
    }

    /// Returns `true` when this is strictly less than zero.
    #[must_use]
    pub fn is_negative(&self) -> bool {
        self.negative
    }

    /// Returns `-1`, `0` or `1` according to the sign.
    #[must_use]
    pub fn signum(&self) -> i32 {
        if self.magnitude.is_empty() {
            0
        } else if self.negative {
            -1
        } else {
            1
        }
    }

    /// Returns the absolute value.
    #[must_use]
    pub fn abs(&self) -> Self {
        Self {
            negative: false,
            magnitude: self.magnitude.clone(),
        }
    }

    /// Returns the additive inverse.
    ///
    /// Negating zero yields zero: there is no negative zero in this type.
    #[must_use]
    pub fn neg(&self) -> Self {
        Self::from_parts(!self.negative, self.magnitude.clone())
    }

    /// Returns `self + other`.
    #[must_use]
    pub fn add(&self, other: &Self) -> Self {
        if self.negative == other.negative {
            Self::from_parts(self.negative, mag_add(&self.magnitude, &other.magnitude))
        } else {
            match mag_cmp(&self.magnitude, &other.magnitude) {
                Ordering::Equal => Self::zero(),
                Ordering::Greater => {
                    Self::from_parts(self.negative, mag_sub(&self.magnitude, &other.magnitude))
                }
                Ordering::Less => {
                    Self::from_parts(other.negative, mag_sub(&other.magnitude, &self.magnitude))
                }
            }
        }
    }

    /// Returns `self - other`.
    #[must_use]
    pub fn sub(&self, other: &Self) -> Self {
        self.add(&other.neg())
    }

    /// Returns `self * other`.
    #[must_use]
    pub fn mul(&self, other: &Self) -> Self {
        Self::from_parts(
            self.negative != other.negative,
            mag_mul(&self.magnitude, &other.magnitude),
        )
    }

    /// Returns `(quotient, remainder)`, or `None` when `other` is zero.
    ///
    /// Division **truncates toward zero** and the remainder takes the sign of
    /// the dividend, exactly matching Rust's `/` and `%` on primitive integers:
    ///
    /// | `self` | `other` | quotient | remainder |
    /// |---|---|---|---|
    /// | `7` | `3` | `2` | `1` |
    /// | `-7` | `3` | `-2` | `-1` |
    /// | `7` | `-3` | `-2` | `1` |
    /// | `-7` | `-3` | `2` | `-1` |
    ///
    /// The identity `quotient * other + remainder == self` holds in every case,
    /// and `|remainder| < |other|`.
    pub fn div_rem(&self, other: &Self) -> Option<(Self, Self)> {
        if other.is_zero() {
            return None;
        }
        let (q, r) = mag_div_rem(&self.magnitude, &other.magnitude);
        Some((
            Self::from_parts(self.negative != other.negative, q),
            Self::from_parts(self.negative, r),
        ))
    }

    /// Returns the greatest common divisor of `self` and `other`.
    ///
    /// The result is always non-negative; signs of the operands are ignored.
    /// `gcd(0, n) == |n|`, `gcd(n, 0) == |n|` and `gcd(0, 0) == 0`.
    #[must_use]
    pub fn gcd(&self, other: &Self) -> Self {
        Self::from_parts(false, mag_gcd(&self.magnitude, &other.magnitude))
    }

    /// Returns 10<sup>`exp`</sup>.
    #[must_use]
    pub fn pow10(exp: u32) -> Self {
        let mut mag: Mag = smallvec![1];
        let mut remaining = exp as usize;
        while remaining >= DECIMAL_CHUNK {
            mag = mag_mul_small_add(&mag, CHUNK_BASE, 0);
            remaining -= DECIMAL_CHUNK;
        }
        if remaining > 0 {
            mag = mag_mul_small_add(&mag, POW10_U64[remaining], 0);
        }
        Self::from_parts(false, mag)
    }

    /// Returns the value as an `i128`, or `None` when it does not fit.
    pub fn to_i128(&self) -> Option<i128> {
        let magnitude = mag_to_u128(&self.magnitude)?;
        if self.negative {
            // `i128::MIN` has magnitude `2^127`, which is representable as a
            // negative value but not as a positive one.
            if magnitude > 1u128 << 127 {
                None
            } else {
                Some(magnitude.wrapping_neg() as i128)
            }
        } else if magnitude > i128::MAX as u128 {
            None
        } else {
            Some(magnitude as i128)
        }
    }

    /// Number of bits in the magnitude; zero for zero.
    #[must_use]
    pub fn bit_len(&self) -> u64 {
        mag_bit_len(&self.magnitude)
    }

    /// Shifts the magnitude left by `bits`, preserving the sign.
    ///
    /// This multiplies by 2<sup>`bits`</sup> for every value, negative included.
    #[must_use]
    pub fn shl(&self, bits: u32) -> Self {
        Self::from_parts(self.negative, mag_shl(&self.magnitude, u64::from(bits)))
    }

    /// Shifts the magnitude right by `bits`, preserving the sign.
    ///
    /// The shift is applied to the **magnitude**, so it truncates **toward
    /// zero** rather than toward negative infinity: `(-5).shr(1)` is `-2`, not
    /// `-3`. This is division by 2<sup>`bits`</sup> under the same rounding rule
    /// [`Int::div_rem`] uses, not Rust's arithmetic `>>` on a signed primitive.
    #[must_use]
    pub fn shr(&self, bits: u32) -> Self {
        Self::from_parts(self.negative, mag_shr(&self.magnitude, u64::from(bits)))
    }

    /// Returns the exact integer floor square root, or `None` when negative.
    ///
    /// The result `r` satisfies `r * r <= self < (r + 1) * (r + 1)`.
    pub fn sqrt_floor(&self) -> Option<Self> {
        if self.negative {
            return None;
        }
        if self.magnitude.is_empty() {
            return Some(Self::zero());
        }
        if let Some(v) = mag_to_u128(&self.magnitude) {
            return Some(Self::from_parts(false, mag_from_u128(isqrt_u128(v))));
        }
        let bits = mag_bit_len(&self.magnitude);
        // `x0 = 2^ceil(bits/2)` is at least `sqrt(self)`, which is what makes
        // the monotone-descent termination test below correct.
        let mut x = mag_shl(&[1u64], bits.div_ceil(2));
        loop {
            let (d, _) = mag_div_rem(&self.magnitude, &x);
            let y = mag_shr(&mag_add(&x, &d), 1);
            if mag_cmp(&y, &x) != Ordering::Less {
                break;
            }
            x = y;
        }
        Some(Self::from_parts(false, x))
    }

    /// Parses a non-empty run of ASCII digits, with no sign and no separators.
    ///
    /// Returns `None` for an empty string or for any byte that is not `0`–`9`.
    /// Leading zeros are accepted and carry no meaning.
    pub fn from_decimal_digits(digits: &str) -> Option<Self> {
        let bytes = digits.as_bytes();
        if bytes.is_empty() || !bytes.iter().all(u8::is_ascii_digit) {
            return None;
        }
        Some(Self::from_parts(
            false,
            mag_from_digit_bytes(bytes.iter().copied()),
        ))
    }

    /// Builds a value from a sign flag and a magnitude, restoring the invariant.
    fn from_parts(negative: bool, mut magnitude: Mag) -> Self {
        mag_trim(&mut magnitude);
        let value = Self {
            negative: negative && !magnitude.is_empty(),
            magnitude,
        };
        value.assert_canonical();
        value
    }

    /// Checks the representation invariant. The checks compile out of release
    /// builds; the call itself is unconditional so the function is never dead.
    fn assert_canonical(&self) {
        debug_assert!(
            self.magnitude.last() != Some(&0),
            "magnitude has a trailing zero limb"
        );
        debug_assert!(
            !(self.negative && self.magnitude.is_empty()),
            "zero must not carry a sign"
        );
    }

    /// Returns `true` when this is exactly one.
    fn is_one(&self) -> bool {
        !self.negative && mag_is_one(&self.magnitude)
    }

    /// Returns `true` when the magnitude is odd.
    fn is_odd(&self) -> bool {
        self.magnitude.first().copied().unwrap_or(0) & 1 == 1
    }
}

impl Ord for Int {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.negative, other.negative) {
            // Zero is never negative, so a sign mismatch already decides it.
            (false, true) => Ordering::Greater,
            (true, false) => Ordering::Less,
            (false, false) => mag_cmp(&self.magnitude, &other.magnitude),
            (true, true) => mag_cmp(&other.magnitude, &self.magnitude),
        }
    }
}

impl PartialOrd for Int {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Int {
    /// Writes the canonical decimal form: a `-` only when negative, no leading
    /// zeros, and `0` for zero.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.magnitude.is_empty() {
            return f.write_str("0");
        }
        let mut groups: SmallVec<[u64; 4]> = SmallVec::new();
        let mut cur = self.magnitude.clone();
        while !cur.is_empty() {
            let (q, r) = mag_divmod_small(&cur, CHUNK_BASE);
            groups.push(r);
            cur = q;
        }
        if self.negative {
            f.write_str("-")?;
        }
        let mut rest = groups.iter().rev();
        let leading = rest.next().expect("a non-zero magnitude has a group");
        write!(f, "{leading}")?;
        for group in rest {
            write!(f, "{group:019}")?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Rat
// ---------------------------------------------------------------------------

/// An exact rational number.
///
/// The denominator is always strictly positive, numerator and denominator are
/// always coprime, and zero is exactly `0/1`. That gives exactly one
/// representation per value, so `PartialEq`, `Eq` and `Hash` are derived.
///
/// Every arithmetic operation is exact. The only lossy operation on this type
/// is [`Rat::to_f64`], which is the crate's single floating-point boundary.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Rat {
    /// Numerator; carries the sign of the value.
    num: Int,
    /// Denominator; always strictly positive and coprime with `num`.
    den: Int,
}

impl Rat {
    /// The value zero, as `0/1`.
    #[must_use]
    pub fn zero() -> Self {
        Self {
            num: Int::zero(),
            den: Int::one(),
        }
    }

    /// The value one, as `1/1`.
    #[must_use]
    pub fn one() -> Self {
        Self {
            num: Int::one(),
            den: Int::one(),
        }
    }

    /// Builds the rational `value / 1`.
    #[must_use]
    pub fn from_int(value: Int) -> Self {
        Self {
            num: value,
            den: Int::one(),
        }
    }

    /// Builds the rational `value / 1` from an `i64`.
    #[must_use]
    pub fn from_i64(value: i64) -> Self {
        Self::from_int(Int::from_i64(value))
    }

    /// Builds `num / den`, normalizing the sign and reducing to lowest terms.
    ///
    /// Returns `None` if and only if `den` is zero.
    pub fn new(num: Int, den: Int) -> Option<Self> {
        if den.is_zero() {
            return None;
        }
        let (num, den) = if den.is_negative() {
            (num.neg(), den.neg())
        } else {
            (num, den)
        };
        if num.is_zero() {
            return Some(Self::zero());
        }
        if den.is_one() {
            return Some(Self::from_reduced(num, den));
        }
        let g = num.gcd(&den);
        if g.is_one() {
            return Some(Self::from_reduced(num, den));
        }
        let reduced_num = num
            .div_rem(&g)
            .expect("a gcd of a non-zero pair is non-zero")
            .0;
        let reduced_den = den
            .div_rem(&g)
            .expect("a gcd of a non-zero pair is non-zero")
            .0;
        Some(Self::from_reduced(reduced_num, reduced_den))
    }

    /// The numerator, carrying the sign of the value.
    #[must_use]
    pub fn numerator(&self) -> &Int {
        &self.num
    }

    /// The denominator, always strictly positive.
    #[must_use]
    pub fn denominator(&self) -> &Int {
        &self.den
    }

    /// Returns `self + other`.
    #[must_use]
    pub fn add(&self, other: &Self) -> Self {
        // Reduce through the gcd of the denominators rather than multiplying
        // them out and reducing afterwards: the intermediate stays small and
        // the expensive gcd runs on `gcd(d1, d2)` instead of on the full
        // cross-product.
        let g = self.den.gcd(&other.den);
        if g.is_one() {
            let num = self.num.mul(&other.den).add(&other.num.mul(&self.den));
            return Self::from_reduced(num, self.den.mul(&other.den));
        }
        let d1g = self
            .den
            .div_rem(&g)
            .expect("a gcd of positives is positive")
            .0;
        let d2g = other
            .den
            .div_rem(&g)
            .expect("a gcd of positives is positive")
            .0;
        let t = self.num.mul(&d2g).add(&other.num.mul(&d1g));
        // `gcd(t, lcm(d1, d2)) == gcd(t, g)`, because `t` is coprime with both
        // `d1/g` and `d2/g`.
        let g2 = t.gcd(&g);
        if g2.is_one() {
            return Self::from_reduced(t, d1g.mul(&other.den));
        }
        let num = t
            .div_rem(&g2)
            .expect("a gcd of a non-zero pair is non-zero")
            .0;
        let den = d1g.mul(
            &other
                .den
                .div_rem(&g2)
                .expect("a gcd of a non-zero pair is non-zero")
                .0,
        );
        Self::from_reduced(num, den)
    }

    /// Returns `self - other`.
    #[must_use]
    pub fn sub(&self, other: &Self) -> Self {
        self.add(&other.neg())
    }

    /// Returns `self * other`.
    #[must_use]
    pub fn mul(&self, other: &Self) -> Self {
        // Cancel crosswise before multiplying, for the same reason as `add`.
        let g1 = self.num.gcd(&other.den);
        let g2 = other.num.gcd(&self.den);
        let (n1, d2) = if g1.is_one() {
            (self.num.clone(), other.den.clone())
        } else {
            let expect = "a gcd with a positive denominator is non-zero";
            (
                self.num.div_rem(&g1).expect(expect).0,
                other.den.div_rem(&g1).expect(expect).0,
            )
        };
        let (n2, d1) = if g2.is_one() {
            (other.num.clone(), self.den.clone())
        } else {
            let expect = "a gcd with a positive denominator is non-zero";
            (
                other.num.div_rem(&g2).expect(expect).0,
                self.den.div_rem(&g2).expect(expect).0,
            )
        };
        Self::from_reduced(n1.mul(&n2), d1.mul(&d2))
    }

    /// Returns `self / other`, or `None` when `other` is zero.
    pub fn div(&self, other: &Self) -> Option<Self> {
        Some(self.mul(&other.recip()?))
    }

    /// Returns the additive inverse.
    #[must_use]
    pub fn neg(&self) -> Self {
        Self {
            num: self.num.neg(),
            den: self.den.clone(),
        }
    }

    /// Returns the absolute value.
    #[must_use]
    pub fn abs(&self) -> Self {
        Self {
            num: self.num.abs(),
            den: self.den.clone(),
        }
    }

    /// Returns `-1`, `0` or `1` according to the sign.
    #[must_use]
    pub fn signum(&self) -> i32 {
        self.num.signum()
    }

    /// Returns `true` when this is zero.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.num.is_zero()
    }

    /// Returns `true` when the denominator is one.
    #[must_use]
    pub fn is_integer(&self) -> bool {
        self.den.is_one()
    }

    /// Returns the multiplicative inverse, or `None` when this is zero.
    pub fn recip(&self) -> Option<Self> {
        if self.num.is_zero() {
            return None;
        }
        Some(if self.num.is_negative() {
            Self {
                num: self.den.neg(),
                den: self.num.abs(),
            }
        } else {
            Self {
                num: self.den.clone(),
                den: self.num.clone(),
            }
        })
    }

    /// Builds the exact value `mantissa / 10^scale`.
    #[must_use]
    pub fn from_decimal(mantissa: Int, scale: u32) -> Self {
        if mantissa.is_zero() {
            return Self::zero();
        }
        Self::new(mantissa, Int::pow10(scale)).expect("a power of ten is non-zero")
    }

    /// Parses an XSD-style decimal or double lexical form **exactly**.
    ///
    /// The accepted grammar is an optional sign, then either digits with an
    /// optional `.` and optional fraction digits or a bare `.` with fraction
    /// digits, then an optional exponent (`e` or `E`, an optional sign, and at
    /// least one digit). At least one mantissa digit is required. Nothing else
    /// is accepted: no whitespace on either side, no digit separators, no
    /// radix prefix, and none of the special values `NaN`, `INF` or `-INF`.
    ///
    /// No floating-point value is constructed anywhere on this path. The
    /// mantissa digits become an [`Int`] and the exponent becomes an exact
    /// power of ten, so `parse_decimal("0.1")` is precisely `1/10` and not the
    /// binary approximation the `f64` literal `0.1` would give.
    ///
    /// # The one refusal this parser makes
    ///
    /// The lexical form allows an unbounded exponent, but `1e999999999` would
    /// ask for a power of ten with a billion digits and exhaust memory before
    /// producing anything. So the **absolute** value of the exponent is capped
    /// at 100000 and anything above that returns `None`. The cap is on the
    /// exponent as written, so it applies even when the mantissa is zero
    /// (`0e200000` is refused); that keeps the refusal a property of the
    /// lexical form alone, and therefore identical on every host. `1e100000` is
    /// at the cap and parses; only `1e100001` and beyond are refused.
    pub fn parse_decimal(text: &str) -> Option<Self> {
        let lexeme = scan_decimal(text)?;
        if lexeme.exponent > MAX_DECIMAL_EXPONENT || lexeme.exponent < -MAX_DECIMAL_EXPONENT {
            return None;
        }
        let magnitude = mag_from_digit_bytes(
            lexeme
                .int_digits
                .iter()
                .chain(lexeme.frac_digits.iter())
                .copied(),
        );
        let mantissa = Int::from_parts(lexeme.negative, magnitude);
        if mantissa.is_zero() {
            return Some(Self::zero());
        }
        // The value is `mantissa * 10^(-scale)`.
        let scale = lexeme.frac_digits.len() as i64 - lexeme.exponent;
        if scale > 0 {
            // A `u32` overflow needs more than four billion fraction digits,
            // which no `&str` this process can hold could supply.
            let scale = u32::try_from(scale).ok()?;
            Self::new(mantissa, Int::pow10(scale))
        } else {
            let scale = u32::try_from(-scale).ok()?;
            Some(Self::from_int(mantissa.mul(&Int::pow10(scale))))
        }
    }

    /// Returns the `f64` nearest to the exact value, rounding half to even.
    ///
    /// **This is the crate's single floating-point boundary.** It exists only
    /// so a caller who asked for an `xsd:double` gets one; no geometric
    /// decision anywhere in this crate is made on the result.
    ///
    /// The value is not computed with floating-point operations. The sign,
    /// biased exponent and 52-bit trailing significand are derived by exact
    /// integer division of the numerator by the denominator — the quotient is
    /// the significand and the remainder decides the rounding, by comparing
    /// twice the remainder against the divisor — and the resulting `u64` bit
    /// pattern is handed to [`f64::from_bits`]. The result is therefore the
    /// correctly rounded double, bit-for-bit identical on every target, in every
    /// case **except the sign of a zero**, which is recorded below.
    ///
    /// Boundary behaviour:
    ///
    /// * Zero returns positive zero.
    /// * Values too small to represent round to **positive** zero even when
    ///   negative. IEEE-754 round-to-nearest would give `-0.0` here, so this is
    ///   the one place the result is not the correctly rounded double: `-1e-400`
    ///   returns `+0.0` where `str::parse::<f64>` returns `-0.0`.
    ///
    ///   This is a consequence of the number model rather than an oversight.
    ///   [`Rat`] has no signed zero — there is exactly one canonical
    ///   representation per value, which is what makes `Eq` and `Hash`
    ///   derivable — so by the time a value has underflowed there is no sign
    ///   left to carry. `-0.0` is likewise unrepresentable on the way IN:
    ///   [`Rat::parse_decimal`] reads `-0.0` as zero, so a `POINT(-0.0 5)`
    ///   ordinate has already lost its sign before this function sees it.
    ///   Consequently **this function never produces `-0.0` at all**, and a
    ///   consumer must not read the sign of a zero as information.
    /// * Subnormals are produced exactly, including the smallest one.
    /// * Values too large to represent return [`f64::INFINITY`] or
    ///   [`f64::NEG_INFINITY`].
    #[must_use]
    pub fn to_f64(&self) -> f64 {
        if self.num.is_zero() {
            return f64::from_bits(0);
        }
        let magnitude_bits = f64_bits_of_ratio(&self.num.magnitude, &self.den.magnitude);
        let sign = if self.num.is_negative() && magnitude_bits != 0 {
            1u64 << 63
        } else {
            0
        };
        f64::from_bits(sign | magnitude_bits)
    }

    /// Renders the value as a decimal literal with at most `max_scale` fraction
    /// digits, rounding half to even.
    ///
    /// Trailing zeros in the fraction are suppressed and a value that needs no
    /// fraction gets no trailing `.`, so an exact short value such as `1/2`
    /// renders as `0.5` whatever `max_scale` is. A value that rounds to zero
    /// renders as `0` with no sign — this function never emits `-0`.
    #[must_use]
    pub fn to_decimal_string(&self, max_scale: u32) -> String {
        let mantissa = self.round_to_scale(max_scale);
        if mantissa.is_zero() {
            return "0".to_owned();
        }
        let digits = mantissa.abs().to_string();
        let scale = max_scale as usize;
        let mut out = String::with_capacity(digits.len() + 3);
        if mantissa.is_negative() {
            out.push('-');
        }
        if scale == 0 {
            out.push_str(&digits);
            return out;
        }
        let padded = if digits.len() <= scale {
            format!("{digits:0>width$}", width = scale + 1)
        } else {
            digits
        };
        let split = padded.len() - scale;
        out.push_str(&padded[..split]);
        let fraction = padded[split..].trim_end_matches('0');
        if !fraction.is_empty() {
            out.push('.');
            out.push_str(fraction);
        }
        out
    }

    /// Returns `round(self * 10^scale)`, rounding half to even.
    ///
    /// This is the mantissa of the value rounded to `scale` decimal places;
    /// `Rat::from_decimal(value.round_to_scale(s), s)` is the rounded value.
    #[must_use]
    pub fn round_to_scale(&self, scale: u32) -> Int {
        let scaled = self.num.mul(&Int::pow10(scale));
        if self.den.is_one() {
            return scaled;
        }
        // Round the magnitude and reapply the sign: half-to-even is symmetric
        // about zero, so this agrees with rounding the signed value directly.
        let (quotient, remainder) = scaled
            .abs()
            .div_rem(&self.den)
            .expect("a canonical denominator is non-zero");
        let rounded = match remainder.shl(1).cmp(&self.den) {
            Ordering::Greater => quotient.add(&Int::one()),
            Ordering::Equal if quotient.is_odd() => quotient.add(&Int::one()),
            Ordering::Equal | Ordering::Less => quotient,
        };
        if self.num.is_negative() {
            rounded.neg()
        } else {
            rounded
        }
    }

    /// Wraps an already reduced numerator and denominator.
    fn from_reduced(num: Int, den: Int) -> Self {
        let value = Self { num, den };
        value.assert_canonical();
        value
    }

    /// Checks the representation invariant. The checks compile out of release
    /// builds; the call itself is unconditional so the function is never dead.
    fn assert_canonical(&self) {
        debug_assert!(
            !self.den.is_zero() && !self.den.is_negative(),
            "denominator must be strictly positive"
        );
        debug_assert!(
            self.num.abs().gcd(&self.den).is_one(),
            "numerator and denominator must be coprime"
        );
        debug_assert!(
            !self.num.is_zero() || self.den.is_one(),
            "zero must be exactly 0/1"
        );
    }
}

impl Default for Rat {
    fn default() -> Self {
        Self::zero()
    }
}

impl Ord for Rat {
    fn cmp(&self, other: &Self) -> Ordering {
        let (sa, sb) = (self.signum(), other.signum());
        if sa != sb {
            return sa.cmp(&sb);
        }
        if self.den == other.den {
            return self.num.cmp(&other.num);
        }
        // Both denominators are strictly positive, so cross-multiplying keeps
        // the order.
        self.num.mul(&other.den).cmp(&other.num.mul(&self.den))
    }
}

impl PartialOrd for Rat {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// ---------------------------------------------------------------------------
// Decimal lexing and the float boundary
// ---------------------------------------------------------------------------

/// The pieces of a decimal lexical form, before any arithmetic happens.
#[derive(Clone, Debug)]
struct DecimalLexeme<'a> {
    /// `true` when the mantissa carried a `-` sign.
    negative: bool,
    /// Digits before the `.`, possibly empty.
    int_digits: &'a [u8],
    /// Digits after the `.`, possibly empty.
    frac_digits: &'a [u8],
    /// Signed exponent; zero when no exponent was written.
    exponent: i64,
}

/// Splits a decimal lexical form into its parts, or returns `None` if the whole
/// string is not one.
fn scan_decimal(text: &str) -> Option<DecimalLexeme<'_>> {
    let bytes = text.as_bytes();
    let mut i = 0usize;
    let negative = match bytes.first() {
        None => return None,
        Some(&c) if c == b'+' || c == b'-' => {
            i = 1;
            c == b'-'
        }
        Some(_) => false,
    };
    let int_start = i;
    i = skip_digits(bytes, i);
    let int_digits = &bytes[int_start..i];
    let frac_digits: &[u8] = if i < bytes.len() && bytes[i] == b'.' {
        let frac_start = i + 1;
        i = skip_digits(bytes, frac_start);
        &bytes[frac_start..i]
    } else {
        &[]
    };
    if int_digits.is_empty() && frac_digits.is_empty() {
        return None;
    }
    let exponent = if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        let (value, after) = scan_exponent(bytes, i + 1)?;
        i = after;
        value
    } else {
        0
    };
    if i != bytes.len() {
        return None;
    }
    Some(DecimalLexeme {
        negative,
        int_digits,
        frac_digits,
        exponent,
    })
}

/// Returns the index just past the run of ASCII digits starting at `from`.
fn skip_digits(bytes: &[u8], from: usize) -> usize {
    let mut i = from;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    i
}

/// Scans an exponent body (an optional sign and at least one digit) starting at
/// `from`, returning its signed value and the index just past it.
///
/// Returns `None` when no digit follows, which is the `1e` and `1e+` rejection.
fn scan_exponent(bytes: &[u8], from: usize) -> Option<(i64, usize)> {
    let signed = from < bytes.len() && (bytes[from] == b'+' || bytes[from] == b'-');
    let negative = from < bytes.len() && bytes[from] == b'-';
    let start = if signed { from + 1 } else { from };
    let end = skip_digits(bytes, start);
    if end == start {
        return None;
    }
    let mut acc = 0i64;
    for &byte in &bytes[start..end] {
        // Saturating: any value that saturates is far above the cap the caller
        // applies, so the exact digits stop mattering.
        acc = acc
            .saturating_mul(10)
            .saturating_add(i64::from(byte - b'0'));
    }
    Some((if negative { -acc } else { acc }, end))
}

/// Returns `true` when `n / d >= 2^exponent`, decided exactly.
fn ratio_at_least_pow2(n: &[u64], d: &[u64], exponent: i64) -> bool {
    if exponent >= 0 {
        mag_cmp(n, &mag_shl(d, exponent as u64)) != Ordering::Less
    } else {
        mag_cmp(&mag_shl(n, exponent.unsigned_abs()), d) != Ordering::Less
    }
}

/// Correctly rounds the positive ratio `n / d` to the `f64` bit pattern of its
/// magnitude (sign bit clear).
///
/// Both magnitudes must be non-empty. The returned pattern is `0` when the
/// value rounds to zero and [`F64_INFINITY_BITS`] when it overflows.
fn f64_bits_of_ratio(n: &[u64], d: &[u64]) -> u64 {
    debug_assert!(!n.is_empty() && !d.is_empty(), "ratio must be non-zero");
    let estimate = mag_bit_len(n) as i64 - mag_bit_len(d) as i64;
    // `2^(estimate - 1) < n/d < 2^(estimate + 1)`, so the true binade is either
    // `estimate` or `estimate - 1`. Bound the estimate before shifting so a
    // wildly out-of-range value never allocates a giant shifted operand.
    if estimate >= 1025 {
        return F64_INFINITY_BITS;
    }
    if estimate <= -1076 {
        return 0;
    }
    let exponent = if ratio_at_least_pow2(n, d, estimate) {
        estimate
    } else {
        estimate - 1
    };
    if exponent >= 1024 {
        return F64_INFINITY_BITS;
    }
    if exponent <= -1076 {
        // Strictly below half the smallest subnormal, so not even a tie.
        return 0;
    }
    // `shift` places the quotient so its integer part is the significand: 53
    // bits for a normal binade, and the fixed `2^-1074` grid for a subnormal.
    let normal = exponent >= -1022;
    let shift: i64 = if normal { 52 - exponent } else { 1074 };
    let (num, den) = if shift >= 0 {
        (mag_shl(n, shift as u64), SmallVec::from_slice(d))
    } else {
        (SmallVec::from_slice(n), mag_shl(d, shift.unsigned_abs()))
    };
    let (quotient, remainder) = mag_div_rem(&num, &den);
    let mut significand =
        mag_to_u128(&quotient).expect("a correctly placed significand fits in 54 bits");
    // Round half to even by comparing twice the remainder against the divisor.
    match mag_cmp(&mag_shl(&remainder, 1), &den) {
        Ordering::Greater => significand += 1,
        Ordering::Equal if significand & 1 == 1 => significand += 1,
        Ordering::Equal | Ordering::Less => {}
    }
    if !normal {
        // A subnormal significand is the whole trailing field. If rounding
        // carried it to `2^52` the pattern is already the smallest normal.
        return significand as u64;
    }
    // Rounding can carry the significand out of its binade and into the next.
    let mut binade = exponent;
    if significand == 1u128 << 53 {
        significand = 1u128 << 52;
        binade += 1;
    }
    if binade > 1023 {
        return F64_INFINITY_BITS;
    }
    let biased = (binade + 1023) as u64;
    (biased << 52) | ((significand as u64) - (1u64 << 52))
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Values chosen to straddle every representation boundary the limb layout
    /// has: single limb, the `u64` ceiling, the first two-limb value, and both
    /// `i128` extremes.
    fn oracle_table() -> Vec<i128> {
        vec![
            0,
            1,
            -1,
            2,
            -2,
            7,
            -7,
            12_345,
            -12_345,
            i128::from(u32::MAX),
            i128::from(u64::MAX),
            -i128::from(u64::MAX),
            i128::from(u64::MAX) + 1,
            -(i128::from(u64::MAX) + 1),
            i128::from(i64::MIN),
            1i128 << 100,
            -(1i128 << 100),
            i128::MAX,
            i128::MIN,
        ]
    }

    fn assert_int_canonical(value: &Int, context: &str) {
        assert!(
            value.magnitude.last() != Some(&0),
            "{context}: trailing zero limb in {value:?}"
        );
        assert!(
            !(value.negative && value.magnitude.is_empty()),
            "{context}: negative zero in {value:?}"
        );
    }

    fn int(value: i64) -> Int {
        Int::from_i64(value)
    }

    fn rat(num: i64, den: i64) -> Rat {
        Rat::new(int(num), int(den)).expect("test denominator is non-zero")
    }

    // -----------------------------------------------------------------------
    // 1. Canonicalization
    // -----------------------------------------------------------------------

    #[test]
    fn zero_has_exactly_one_representation() {
        let zero = Int::zero();
        assert_int_canonical(&zero, "zero");
        assert!(zero.is_zero());
        assert!(!zero.is_negative());
        assert_eq!(zero.signum(), 0);
        assert_eq!(zero, Int::default());
        assert_eq!(zero, Int::from_i64(0));
        assert_eq!(zero, Int::from_i128(0));
        assert_eq!(zero, Int::from_u64(0));
        assert_eq!(zero, zero.neg());
        assert_eq!(zero, zero.abs());
        assert_eq!(zero.to_string(), "0");
    }

    #[test]
    fn subtracting_a_value_from_itself_yields_canonical_zero() {
        for value in oracle_table() {
            let a = Int::from_i128(value);
            let difference = a.sub(&a);
            assert_int_canonical(&difference, "self-subtraction");
            assert_eq!(difference, Int::zero(), "{value} - {value}");
            assert!(!difference.is_negative());
        }
    }

    #[test]
    fn multiplying_by_zero_yields_canonical_zero() {
        for value in oracle_table() {
            let a = Int::from_i128(value);
            for zero in [Int::zero(), Int::zero().neg()] {
                let product = a.mul(&zero);
                assert_int_canonical(&product, "multiply by zero");
                assert_eq!(product, Int::zero());
                assert!(product.magnitude.is_empty());
            }
        }
    }

    #[test]
    fn every_operation_leaves_a_canonical_value() {
        for a_value in oracle_table() {
            for b_value in oracle_table() {
                let a = Int::from_i128(a_value);
                let b = Int::from_i128(b_value);
                assert_int_canonical(&a.add(&b), "add");
                assert_int_canonical(&a.sub(&b), "sub");
                assert_int_canonical(&a.mul(&b), "mul");
                assert_int_canonical(&a.gcd(&b), "gcd");
                assert_int_canonical(&a.abs(), "abs");
                assert_int_canonical(&a.neg(), "neg");
                assert_int_canonical(&a.shl(65), "shl");
                assert_int_canonical(&a.shr(65), "shr");
                if let Some((q, r)) = a.div_rem(&b) {
                    assert_int_canonical(&q, "div_rem quotient");
                    assert_int_canonical(&r, "div_rem remainder");
                }
            }
        }
    }

    #[test]
    fn cancelling_limbs_leaves_no_trailing_zero_limb() {
        // `2^128 - 2^128` and `(2^128 + 1) - 1` both retire a high limb.
        let big = Int::one().shl(128);
        assert_int_canonical(&big.sub(&big), "limb cancellation");
        assert!(big.sub(&big).is_zero());
        let plus_one = big.add(&Int::one());
        let back = plus_one.sub(&Int::one());
        assert_int_canonical(&back, "high limb retained");
        assert_eq!(back, big);
        let down = big.sub(&Int::one());
        assert_int_canonical(&down, "high limb dropped");
        assert_eq!(down.bit_len(), 128);
    }

    // -----------------------------------------------------------------------
    // 2. add / sub / mul against an i128 oracle
    // -----------------------------------------------------------------------

    #[test]
    fn add_matches_the_i128_oracle() {
        for a_value in oracle_table() {
            for b_value in oracle_table() {
                let Some(expected) = a_value.checked_add(b_value) else {
                    continue;
                };
                let got = Int::from_i128(a_value).add(&Int::from_i128(b_value));
                assert_eq!(got.to_i128(), Some(expected), "{a_value} + {b_value}");
            }
        }
    }

    #[test]
    fn sub_matches_the_i128_oracle() {
        for a_value in oracle_table() {
            for b_value in oracle_table() {
                let Some(expected) = a_value.checked_sub(b_value) else {
                    continue;
                };
                let got = Int::from_i128(a_value).sub(&Int::from_i128(b_value));
                assert_eq!(got.to_i128(), Some(expected), "{a_value} - {b_value}");
            }
        }
    }

    #[test]
    fn mul_matches_the_i128_oracle() {
        for a_value in oracle_table() {
            for b_value in oracle_table() {
                let Some(expected) = a_value.checked_mul(b_value) else {
                    continue;
                };
                let got = Int::from_i128(a_value).mul(&Int::from_i128(b_value));
                assert_eq!(got.to_i128(), Some(expected), "{a_value} * {b_value}");
            }
        }
    }

    #[test]
    fn arithmetic_crosses_the_limb_boundary_exactly() {
        let limb = Int::from_u64(u64::MAX);
        assert_eq!(limb.add(&Int::one()).to_i128(), Some(1i128 << 64));
        assert_eq!(limb.add(&Int::one()).bit_len(), 65);
        // (2^64 - 1)^2 == 2^128 - 2^65 + 1, which no primitive can hold.
        assert_eq!(
            limb.mul(&limb),
            Int::one()
                .shl(128)
                .sub(&Int::one().shl(65))
                .add(&Int::one())
        );
        assert_eq!(limb.mul(&limb).to_i128(), None);
        let two_limbs = Int::from_i128(1i128 << 64);
        assert_eq!(two_limbs.sub(&Int::one()), limb);
        assert_eq!(two_limbs.magnitude.len(), 2);
        assert_eq!(limb.magnitude.len(), 1);
    }

    #[test]
    fn multi_limb_products_exceed_i128_and_still_render_exactly() {
        // 2^64 * 2^64 * 2^64 = 2^192, which no primitive can hold.
        let limb = Int::one().shl(64);
        let cube = limb.mul(&limb).mul(&limb);
        assert_eq!(cube.to_i128(), None);
        assert_eq!(cube.bit_len(), 193);
        assert_eq!(cube, Int::one().shl(192));
        assert_eq!(cube.magnitude.len(), 4);
        let rendered = cube.to_string();
        // floor(192 * log10(2)) + 1 == 58 digits.
        assert_eq!(rendered.len(), 58);
        assert!(rendered.starts_with('6'), "2^192 begins with 6: {rendered}");
        assert!(rendered.ends_with("896"), "2^192 ends with 896: {rendered}");
        assert_eq!(Int::from_decimal_digits(&rendered), Some(cube));
    }

    #[test]
    fn to_i128_reports_the_representable_boundary() {
        assert_eq!(Int::from_i128(i128::MIN).to_i128(), Some(i128::MIN));
        assert_eq!(Int::from_i128(i128::MAX).to_i128(), Some(i128::MAX));
        // |i128::MIN| is one past i128::MAX, so its absolute value does not fit.
        assert_eq!(Int::from_i128(i128::MIN).abs().to_i128(), None);
        assert_eq!(Int::from_i128(i128::MAX).add(&Int::one()).to_i128(), None);
        assert_eq!(Int::from_i128(i128::MIN).sub(&Int::one()).to_i128(), None);
    }

    // -----------------------------------------------------------------------
    // 3. div_rem
    // -----------------------------------------------------------------------

    #[test]
    fn div_rem_truncates_toward_zero_like_rust() {
        for a_value in [7i128, -7, 12, -12, 0, 1, -1, i128::MAX, i128::MIN, 1 << 100] {
            for b_value in [3i128, -3, 1, -1, 5, -5, i128::MAX, i128::MIN, 1 << 64] {
                let (q, r) = Int::from_i128(a_value)
                    .div_rem(&Int::from_i128(b_value))
                    .expect("divisor is non-zero");
                let Some(expected_q) = a_value.checked_div(b_value) else {
                    // The only unrepresentable case: `i128::MIN / -1` is exactly
                    // `2^127`, which Rust wraps and this type does not.
                    assert_eq!((a_value, b_value), (i128::MIN, -1));
                    assert_eq!(q, Int::from_i128(i128::MAX).add(&Int::one()));
                    assert!(r.is_zero());
                    continue;
                };
                assert_eq!(
                    q.to_i128(),
                    Some(expected_q),
                    "quotient of {a_value} / {b_value}"
                );
                assert_eq!(
                    r.to_i128(),
                    a_value.checked_rem(b_value),
                    "remainder of {a_value} % {b_value}"
                );
            }
        }
    }

    #[test]
    fn div_rem_remainder_takes_the_sign_of_the_dividend() {
        let cases = [
            (7i64, 3i64, 2i64, 1i64),
            (-7, 3, -2, -1),
            (7, -3, -2, 1),
            (-7, -3, 2, -1),
        ];
        for (a, b, expected_q, expected_r) in cases {
            let (q, r) = int(a).div_rem(&int(b)).expect("divisor is non-zero");
            assert_eq!(q, int(expected_q), "quotient of {a} / {b}");
            assert_eq!(r, int(expected_r), "remainder of {a} % {b}");
            assert_eq!(q.mul(&int(b)).add(&r), int(a), "identity for {a} / {b}");
        }
    }

    #[test]
    fn div_rem_by_zero_is_none() {
        for value in oracle_table() {
            assert!(Int::from_i128(value).div_rem(&Int::zero()).is_none());
        }
        // The neighbouring valid case: dividing by one is always fine.
        for value in oracle_table() {
            let (q, r) = Int::from_i128(value)
                .div_rem(&Int::one())
                .expect("one is a valid divisor");
            assert_eq!(q, Int::from_i128(value));
            assert!(r.is_zero());
        }
    }

    #[test]
    fn div_rem_handles_a_multi_limb_dividend() {
        let dividend = Int::one().shl(200).add(&int(12_345));
        let divisor = Int::one().shl(100).add(&int(7));
        let (q, r) = dividend.div_rem(&divisor).expect("divisor is non-zero");
        assert_eq!(q.mul(&divisor).add(&r), dividend);
        assert!(!r.is_negative());
        assert!(r < divisor);
        assert!(q.bit_len() >= 99);
        // And the negated dividend mirrors it exactly.
        let (nq, nr) = dividend
            .neg()
            .div_rem(&divisor)
            .expect("divisor is non-zero");
        assert_eq!(nq, q.neg());
        assert_eq!(nr, r.neg());
    }

    #[test]
    fn div_rem_satisfies_its_identity_across_the_table() {
        for a_value in oracle_table() {
            for b_value in oracle_table() {
                let a = Int::from_i128(a_value);
                let b = Int::from_i128(b_value);
                let Some((q, r)) = a.div_rem(&b) else {
                    assert_eq!(b_value, 0);
                    continue;
                };
                assert_eq!(q.mul(&b).add(&r), a, "{a_value} / {b_value}");
                assert!(
                    r.abs() < b.abs(),
                    "remainder too large for {a_value}/{b_value}"
                );
                if !r.is_zero() {
                    assert_eq!(r.signum(), a.signum(), "remainder sign for {a_value}");
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // 4. gcd
    // -----------------------------------------------------------------------

    #[test]
    fn gcd_is_non_negative_and_absorbs_zero() {
        assert_eq!(Int::zero().gcd(&Int::zero()), Int::zero());
        for value in oracle_table() {
            let n = Int::from_i128(value);
            assert_eq!(Int::zero().gcd(&n), n.abs(), "gcd(0, {value})");
            assert_eq!(n.gcd(&Int::zero()), n.abs(), "gcd({value}, 0)");
            assert!(!n.gcd(&Int::one()).is_negative());
            assert_eq!(n.gcd(&Int::one()), Int::one(), "gcd({value}, 1)");
        }
    }

    #[test]
    fn gcd_ignores_the_signs_of_its_operands() {
        for (a, b, expected) in [
            (12i64, 18i64, 6i64),
            (-12, 18, 6),
            (12, -18, 6),
            (-12, -18, 6),
            (17, 5, 1),
            (100, 100, 100),
            (-100, 100, 100),
        ] {
            assert_eq!(int(a).gcd(&int(b)), int(expected), "gcd({a}, {b})");
        }
    }

    #[test]
    fn gcd_handles_a_multi_limb_pair() {
        // gcd(3 * 2^200, 5 * 2^200) == 2^200 — well past the u128 fast path.
        let base = Int::one().shl(200);
        let a = base.mul(&int(3));
        let b = base.mul(&int(5));
        assert_eq!(a.gcd(&b), base);
        assert_eq!(b.gcd(&a), base);
        assert_eq!(a.neg().gcd(&b.neg()), base);

        // gcd(p * 7, p * 11) == p, where p = 2^128 + 1.
        let p = Int::from_decimal_digits("340282366920938463463374607431768211457")
            .expect("literal is all digits");
        assert_eq!(p, Int::one().shl(128).add(&Int::one()));
        assert_eq!(p.mul(&int(7)).gcd(&p.mul(&int(11))), p);
    }

    // -----------------------------------------------------------------------
    // 5. sqrt_floor
    // -----------------------------------------------------------------------

    #[test]
    fn sqrt_floor_is_exact_around_every_small_perfect_square() {
        // `k = 0` is the one place `k^2 + 1` is not still `k`: sqrt(1) is 1.
        assert_eq!(Int::zero().sqrt_floor(), Some(Int::zero()));
        assert_eq!(Int::one().sqrt_floor(), Some(Int::one()));
        for k in 1i64..3_000 {
            let square = int(k * k);
            assert_eq!(square.sqrt_floor(), Some(int(k)), "sqrt({})", k * k);
            assert_eq!(
                square.sub(&Int::one()).sqrt_floor(),
                Some(int(k - 1)),
                "sqrt({} - 1)",
                k * k
            );
            assert_eq!(
                square.add(&Int::one()).sqrt_floor(),
                Some(int(k)),
                "sqrt({} + 1)",
                k * k
            );
        }
    }

    #[test]
    fn sqrt_floor_is_exact_for_a_large_multi_limb_square() {
        let base = Int::from_decimal_digits("123456789012345678901234567890123456789")
            .expect("literal is all digits");
        let square = base.mul(&base);
        assert!(square.bit_len() > 128, "the square must miss the u128 path");
        assert_eq!(square.sqrt_floor(), Some(base.clone()));
        assert_eq!(
            square.sub(&Int::one()).sqrt_floor(),
            Some(base.sub(&Int::one()))
        );
        assert_eq!(square.add(&Int::one()).sqrt_floor(), Some(base));

        // A power of two square, so the initial Newton guess is exactly on the
        // boundary of the bit-length estimate.
        let power = Int::one().shl(300);
        assert_eq!(power.mul(&power).sqrt_floor(), Some(power));
    }

    #[test]
    fn sqrt_floor_refuses_negatives_but_accepts_their_magnitudes() {
        for value in [-1i64, -2, -100, -1_000_000] {
            assert_eq!(int(value).sqrt_floor(), None, "sqrt({value})");
            // The neighbouring valid case must still succeed.
            assert!(
                int(value).abs().sqrt_floor().is_some(),
                "sqrt({}) must succeed",
                -value
            );
        }
        assert_eq!(Int::zero().sqrt_floor(), Some(Int::zero()));
        assert_eq!(Int::one().sqrt_floor(), Some(Int::one()));
        // sqrt(2^126) is exactly 2^63.
        assert_eq!(Int::one().shl(126).sqrt_floor(), Some(Int::one().shl(63)));
        // And the defining property holds at the top of the u128 fast path.
        let root = Int::from_i128(i128::MAX)
            .sqrt_floor()
            .expect("i128::MAX is non-negative");
        assert!(
            root.mul(&root) <= Int::from_i128(i128::MAX),
            "root is too large"
        );
        let next = root.add(&Int::one());
        assert!(
            next.mul(&next) > Int::from_i128(i128::MAX),
            "root is too small"
        );
    }

    // -----------------------------------------------------------------------
    // 6. Ord
    // -----------------------------------------------------------------------

    #[test]
    fn ord_is_the_numeric_order_across_signs_and_magnitudes() {
        let mut values: Vec<Int> = oracle_table().into_iter().map(Int::from_i128).collect();
        values.sort();
        let mut expected = oracle_table();
        expected.sort_unstable();
        let sorted: Vec<i128> = values
            .iter()
            .map(|v| v.to_i128().expect("table values fit i128"))
            .collect();
        assert_eq!(sorted, expected);
    }

    #[test]
    fn ord_agrees_with_i128_on_representable_values() {
        for a_value in oracle_table() {
            for b_value in oracle_table() {
                assert_eq!(
                    Int::from_i128(a_value).cmp(&Int::from_i128(b_value)),
                    a_value.cmp(&b_value),
                    "cmp({a_value}, {b_value})"
                );
            }
        }
    }

    #[test]
    fn ord_puts_negatives_below_zero_below_positives_beyond_i128() {
        let big = Int::one().shl(300);
        let small = Int::one().shl(299);
        assert!(big > small);
        assert!(big.neg() < small.neg());
        assert!(big.neg() < Int::zero());
        assert!(Int::zero() < small);
        assert!(big.neg() < big);
        assert_eq!(big.cmp(&big), Ordering::Equal);
        assert_eq!(big.neg().cmp(&big.neg()), Ordering::Equal);
    }

    // -----------------------------------------------------------------------
    // 7. Display / from_decimal_digits
    // -----------------------------------------------------------------------

    #[test]
    fn display_round_trips_through_from_decimal_digits() {
        let samples = [
            "0",
            "1",
            "9",
            "10",
            "12345",
            "18446744073709551615",
            "18446744073709551616",
            "340282366920938463463374607431768211455",
            "340282366920938463463374607431768211456",
            "1234567890123456789012345678901234567890123456789",
        ];
        for sample in samples {
            let parsed = Int::from_decimal_digits(sample).expect("sample is all digits");
            assert_eq!(parsed.to_string(), sample, "round trip of {sample}");
        }
    }

    #[test]
    fn display_writes_a_sign_only_for_negatives() {
        assert_eq!(int(0).to_string(), "0");
        assert_eq!(int(0).neg().to_string(), "0");
        assert_eq!(int(-1).to_string(), "-1");
        assert_eq!(int(-12_345).to_string(), "-12345");
        assert_eq!(Int::pow10(0).to_string(), "1");
        assert_eq!(Int::pow10(1).to_string(), "10");
        assert_eq!(Int::pow10(19).to_string(), format!("1{}", "0".repeat(19)));
        assert_eq!(Int::pow10(20).to_string(), format!("1{}", "0".repeat(20)));
        assert_eq!(
            Int::pow10(50).neg().to_string(),
            format!("-1{}", "0".repeat(50))
        );
    }

    #[test]
    fn from_decimal_digits_rejects_anything_but_a_digit_run() {
        for bad in [
            "", "-1", "+1", "1 ", " 1", "1_0", "1.0", "12a", "0x10", "1\n", "1e5",
        ] {
            assert!(
                Int::from_decimal_digits(bad).is_none(),
                "{bad:?} must be rejected"
            );
        }
        // The neighbouring valid cases: leading zeros and long runs are fine.
        assert_eq!(Int::from_decimal_digits("007").expect("digits"), int(7));
        assert_eq!(Int::from_decimal_digits("0").expect("digits"), Int::zero());
        assert_eq!(
            Int::from_decimal_digits("00000000000000000000000000000001").expect("digits"),
            Int::one()
        );
    }

    #[test]
    fn bit_len_shl_and_shr_agree_on_the_magnitude() {
        assert_eq!(Int::zero().bit_len(), 0);
        assert_eq!(int(1).bit_len(), 1);
        assert_eq!(int(255).bit_len(), 8);
        assert_eq!(int(256).bit_len(), 9);
        assert_eq!(Int::from_u64(u64::MAX).bit_len(), 64);
        assert_eq!(Int::one().shl(64).bit_len(), 65);
        assert_eq!(Int::one().shl(0), Int::one());
        assert_eq!(Int::zero().shl(1_000), Int::zero());
        assert_eq!(Int::zero().shr(1_000), Int::zero());
    }

    #[test]
    fn shr_truncates_the_magnitude_toward_zero() {
        assert_eq!(int(5).shr(1), int(2));
        assert_eq!(int(-5).shr(1), int(-2));
        assert_eq!(int(-1).shr(1), Int::zero());
        assert!(!int(-1).shr(1).is_negative());
        assert_eq!(int(-20).shl(2), int(-80));
        assert_eq!(int(-5).shl(2), int(-20));
        let big = Int::one().shl(200);
        assert_eq!(big.shr(200), Int::one());
        assert_eq!(big.shr(201), Int::zero());
        assert_eq!(big.neg().shr(199), int(-2));
    }

    // -----------------------------------------------------------------------
    // 8. Rat normalization
    // -----------------------------------------------------------------------

    #[test]
    fn rat_new_reduces_and_normalizes_the_sign() {
        assert_eq!(rat(2, 4), rat(1, 2));
        assert_eq!(rat(-1, -2), rat(1, 2));
        assert_eq!(rat(1, -2), rat(-1, 2));
        assert_eq!(rat(0, 5), Rat::zero());
        assert_eq!(rat(0, -5), Rat::zero());
        assert_eq!(*rat(0, 5).denominator(), Int::one());
        assert_eq!(rat(6, 3), Rat::from_i64(2));
        assert_eq!(rat(-6, 3), Rat::from_i64(-2));
        assert_eq!(*rat(1, -2).denominator(), int(2));
        assert_eq!(*rat(1, -2).numerator(), int(-1));
        assert!(!rat(1, -2).denominator().is_negative());
        assert_eq!(Rat::new(Int::one(), Int::zero()), None);
        // The neighbouring valid case: any non-zero denominator is accepted.
        assert!(Rat::new(Int::one(), int(-1)).is_some());
    }

    #[test]
    fn rat_predicates_report_the_normalized_form() {
        assert!(Rat::zero().is_zero());
        assert!(Rat::zero().is_integer());
        assert_eq!(Rat::zero().signum(), 0);
        assert_eq!(Rat::default(), Rat::zero());
        assert!(Rat::one().is_integer());
        assert!(!rat(1, 2).is_integer());
        assert!(rat(4, 2).is_integer());
        assert_eq!(rat(-3, 4).signum(), -1);
        assert_eq!(rat(3, 4).signum(), 1);
        assert_eq!(rat(-3, 4).abs(), rat(3, 4));
        assert_eq!(rat(3, 4).neg(), rat(-3, 4));
        assert_eq!(Rat::zero().neg(), Rat::zero());
        assert_eq!(Rat::zero().recip(), None);
        assert_eq!(rat(-2, 3).recip(), Some(rat(-3, 2)));
        assert_eq!(rat(2, 3).recip(), Some(rat(3, 2)));
    }

    // -----------------------------------------------------------------------
    // 9. Rat arithmetic identities
    // -----------------------------------------------------------------------

    fn rat_table() -> Vec<Rat> {
        vec![
            Rat::zero(),
            Rat::one(),
            rat(-1, 1),
            rat(1, 2),
            rat(-1, 2),
            rat(2, 3),
            rat(-5, 7),
            rat(22, 7),
            rat(1, 1_000_000),
            rat(999_999, 1_000_000),
            Rat::from_int(Int::one().shl(100)),
            Rat::new(Int::one(), Int::one().shl(100)).expect("non-zero denominator"),
            Rat::new(Int::one().shl(70), Int::from_u64(u64::MAX)).expect("non-zero denominator"),
        ]
    }

    #[test]
    fn rat_addition_and_subtraction_are_inverses() {
        for a in rat_table() {
            for b in rat_table() {
                assert_eq!(a.add(&b).sub(&b), a, "({a:?} + {b:?}) - {b:?}");
                assert_eq!(a.add(&b), b.add(&a), "commutativity");
                assert_eq!(a.sub(&b), a.add(&b.neg()), "sub is add of the negation");
                assert_eq!(a.add(&Rat::zero()), a, "additive identity");
                assert_eq!(a.add(&a.neg()), Rat::zero(), "additive inverse");
            }
        }
    }

    #[test]
    fn rat_multiplication_and_division_are_inverses() {
        for a in rat_table() {
            for b in rat_table() {
                let product = a.mul(&b);
                assert_eq!(product, b.mul(&a), "commutativity");
                assert_eq!(a.mul(&Rat::one()), a, "multiplicative identity");
                assert_eq!(a.mul(&Rat::zero()), Rat::zero(), "absorbing zero");
                if b.is_zero() {
                    assert_eq!(a.div(&b), None, "division by zero");
                } else {
                    assert_eq!(product.div(&b), Some(a.clone()), "({a:?} * {b:?}) / {b:?}");
                    assert_eq!(a.div(&b), Some(a.mul(&b.recip().expect("non-zero"))));
                }
            }
        }
    }

    #[test]
    fn rat_arithmetic_is_associative_and_distributive() {
        let table = rat_table();
        for a in &table {
            for b in &table {
                for c in &table {
                    assert_eq!(a.add(b).add(c), a.add(&b.add(c)), "additive associativity");
                    assert_eq!(
                        a.mul(b).mul(c),
                        a.mul(&b.mul(c)),
                        "multiplicative associativity"
                    );
                    assert_eq!(a.mul(&b.add(c)), a.mul(b).add(&a.mul(c)), "distributivity");
                }
            }
        }
    }

    #[test]
    fn rat_ordering_survives_arithmetic() {
        assert!(rat(1, 3) < rat(1, 2));
        assert!(rat(-1, 2) < rat(-1, 3));
        assert!(rat(-1, 3) < Rat::zero());
        assert!(Rat::zero() < rat(1, 1_000_000));
        assert!(rat(22, 7) > Rat::from_i64(3));
        let mut sorted = rat_table();
        sorted.sort();
        for pair in sorted.windows(2) {
            assert!(pair[0] <= pair[1], "sort produced {pair:?} out of order");
        }
        // Adding the same value to both sides preserves the order.
        let shift = rat(7, 11);
        for a in rat_table() {
            for b in rat_table() {
                assert_eq!(
                    a.cmp(&b),
                    a.add(&shift).cmp(&b.add(&shift)),
                    "order after a common shift"
                );
            }
        }
    }

    #[test]
    fn rat_from_decimal_is_the_exact_scaled_mantissa() {
        assert_eq!(Rat::from_decimal(int(1), 1), rat(1, 10));
        assert_eq!(Rat::from_decimal(int(-25), 4), rat(-1, 400));
        assert_eq!(Rat::from_decimal(int(0), 100), Rat::zero());
        assert_eq!(Rat::from_decimal(int(5), 0), Rat::from_i64(5));
        assert_eq!(
            Rat::from_decimal(int(1_000), 3),
            Rat::one(),
            "trailing zeros must reduce away"
        );
    }

    // -----------------------------------------------------------------------
    // 10. parse_decimal
    // -----------------------------------------------------------------------

    #[test]
    fn parse_decimal_accepts_the_whole_lexical_grammar() {
        let cases: [(&str, Rat); 12] = [
            ("1", Rat::one()),
            ("-1.5", rat(-3, 2)),
            ("+.5", rat(1, 2)),
            ("1.", Rat::one()),
            ("1e10", Rat::from_int(Int::pow10(10))),
            ("-2.5E-3", rat(-1, 400)),
            ("0", Rat::zero()),
            ("00.10", rat(1, 10)),
            ("+1", Rat::one()),
            ("-0", Rat::zero()),
            ("1E+2", Rat::from_i64(100)),
            ("0.000", Rat::zero()),
        ];
        for (text, expected) in cases {
            assert_eq!(
                Rat::parse_decimal(text),
                Some(expected),
                "{text:?} must parse exactly"
            );
        }
    }

    #[test]
    fn parse_decimal_rejects_everything_outside_the_grammar() {
        for bad in [
            "", ".", "e5", "1e", "1.2.3", "NaN", "INF", "-INF", "1 ", " 1", "0x10", "1_000", "+",
            "-", "1e+", "1e-", "--1", "1..2", ".e5", "1e1.5", "0b1",
        ] {
            assert_eq!(Rat::parse_decimal(bad), None, "{bad:?} must be rejected");
        }
        // The neighbouring valid cases for the trickiest rejections.
        assert!(Rat::parse_decimal(".5").is_some(), "a bare .5 is valid");
        assert!(Rat::parse_decimal("1e5").is_some(), "1e5 is valid");
        assert!(Rat::parse_decimal("1e-5").is_some(), "1e-5 is valid");
        assert!(Rat::parse_decimal("1.2").is_some(), "1.2 is valid");
        assert!(Rat::parse_decimal("-1").is_some(), "-1 is valid");
    }

    #[test]
    fn parse_decimal_never_touches_a_float() {
        // 0.1 is not representable in binary. If any step of the parse went
        // through an f64 this would not be exactly one tenth.
        assert_eq!(
            Rat::parse_decimal("0.1"),
            Some(Rat::new(Int::from_i64(1), Int::from_i64(10)).expect("non-zero denominator"))
        );
        assert_eq!(
            Rat::parse_decimal("0.1").expect("valid").numerator(),
            &Int::one()
        );
        assert_eq!(
            Rat::parse_decimal("0.1").expect("valid").denominator(),
            &int(10)
        );
        // A 30-digit mantissa survives verbatim; an f64 keeps 17.
        let long = "1.234567890123456789012345678901";
        let parsed = Rat::parse_decimal(long).expect("valid");
        assert_eq!(
            parsed,
            Rat::new(
                Int::from_decimal_digits("1234567890123456789012345678901").expect("digits"),
                Int::pow10(30)
            )
            .expect("non-zero denominator")
        );
        assert_eq!(parsed.to_decimal_string(30), long);
    }

    #[test]
    fn parse_decimal_caps_the_exponent_without_over_refusing() {
        // The refusal.
        assert_eq!(Rat::parse_decimal("1e100001"), None);
        assert_eq!(Rat::parse_decimal("1e-100001"), None);
        assert_eq!(Rat::parse_decimal("1e999999999999999999999"), None);
        assert_eq!(Rat::parse_decimal("0e100001"), None);
        // The over-refusal control: everything at or below the cap still parses.
        let at_cap = Rat::parse_decimal("1e100000").expect("the cap itself must parse");
        assert!(at_cap.is_integer());
        assert_eq!(at_cap.signum(), 1);
        assert_eq!(
            Rat::parse_decimal("1e100"),
            Some(Rat::from_int(Int::pow10(100)))
        );
        assert_eq!(
            Rat::parse_decimal("1e-100"),
            Rat::new(Int::one(), Int::pow10(100))
        );
        let at_cap_negative =
            Rat::parse_decimal("1e-100000").expect("the negative cap must parse too");
        assert!(!at_cap_negative.is_integer());
        assert_eq!(at_cap_negative.signum(), 1);
        assert_eq!(*at_cap_negative.numerator(), Int::one());
    }

    // -----------------------------------------------------------------------
    // 11. to_f64
    // -----------------------------------------------------------------------

    #[test]
    fn to_f64_is_exact_on_powers_of_two() {
        for exponent in -60i32..60 {
            let expected = 2.0f64.powi(exponent);
            let value = if exponent >= 0 {
                Rat::from_int(Int::one().shl(exponent as u32))
            } else {
                Rat::new(Int::one(), Int::one().shl(exponent.unsigned_abs()))
                    .expect("non-zero denominator")
            };
            assert_eq!(value.to_f64().to_bits(), expected.to_bits(), "2^{exponent}");
            assert_eq!(
                value.neg().to_f64().to_bits(),
                (-expected).to_bits(),
                "-2^{exponent}"
            );
        }
    }

    #[test]
    fn to_f64_is_exact_on_small_integers_and_dyadic_fractions() {
        for n in -1_000i64..1_000 {
            assert_eq!(Rat::from_i64(n).to_f64().to_bits(), (n as f64).to_bits());
            assert_eq!(
                rat(n, 4).to_f64().to_bits(),
                ((n as f64) / 4.0).to_bits(),
                "{n}/4"
            );
        }
    }

    #[test]
    fn to_f64_rounds_ties_to_even_in_both_directions() {
        // Doubles just above 1.0 are spaced 2^-52 apart, so `1 + 2^-53` sits
        // exactly on a tie. The even neighbour is 1.0, so it rounds down.
        let two_53 = Int::one().shl(53);
        let tie_down_numerator = two_53.add(&Int::one());
        let down = Rat::new(tie_down_numerator, two_53.clone()).expect("non-zero denominator");
        assert_eq!(down.to_f64().to_bits(), 1.0f64.to_bits());
        // `1 + 3 * 2^-53` is the next tie up; there the even neighbour is the
        // upper one, so the same rule rounds the other way.
        let tie_up_numerator = two_53.add(&int(3));
        let up = Rat::new(tie_up_numerator, two_53).expect("non-zero denominator");
        assert_eq!(up.to_f64().to_bits(), 1.0f64.to_bits() + 2);
        // A quarter of an ulp either side of the first tie resolves by
        // magnitude, not by parity: `1 + 2^-54` rounds down and `1 + 3 * 2^-54`
        // rounds up, even though both quotients are odd.
        let two_54 = Int::one().shl(54);
        let below_numerator = two_54.add(&Int::one());
        let below = Rat::new(below_numerator, two_54.clone()).expect("non-zero denominator");
        assert_eq!(below.to_f64().to_bits(), 1.0f64.to_bits());
        let above_numerator = two_54.add(&int(3));
        let above = Rat::new(above_numerator, two_54).expect("non-zero denominator");
        assert_eq!(above.to_f64().to_bits(), 1.0f64.to_bits() + 1);
    }

    #[test]
    fn to_f64_matches_the_rust_literal_for_inexact_values() {
        assert_eq!(rat(1, 3).to_f64().to_bits(), (1.0f64 / 3.0f64).to_bits());
        assert_eq!(
            rat(-1, 3).to_f64().to_bits(),
            (-(1.0f64 / 3.0f64)).to_bits()
        );
        assert_eq!(
            rat(-1, 3).to_f64().to_bits(),
            (-rat(1, 3).to_f64()).to_bits()
        );
        assert_eq!(
            Rat::parse_decimal("0.1").expect("valid").to_f64().to_bits(),
            0.1f64.to_bits()
        );
        assert_eq!(
            Rat::parse_decimal("-0.1")
                .expect("valid")
                .to_f64()
                .to_bits(),
            (-0.1f64).to_bits()
        );
        // The shortest decimal that round-trips through an f64 pi must land
        // back on exactly those bits.
        assert_eq!(
            Rat::parse_decimal("3.141592653589793")
                .expect("valid")
                .to_f64()
                .to_bits(),
            std::f64::consts::PI.to_bits()
        );
        assert_eq!(rat(2, 3).to_f64().to_bits(), (2.0f64 / 3.0f64).to_bits());
        assert_eq!(rat(1, 10).to_f64().to_bits(), 0.1f64.to_bits());
        assert_eq!(rat(1, 100).to_f64().to_bits(), 0.01f64.to_bits());
    }

    #[test]
    fn to_f64_reaches_the_subnormal_range_exactly() {
        // The smallest positive subnormal, 2^-1074.
        let tiny = Rat::new(Int::one(), Int::one().shl(1_074)).expect("non-zero denominator");
        assert_eq!(tiny.to_f64().to_bits(), 1);
        assert_eq!(tiny.to_f64(), f64::from_bits(1));
        assert_eq!(tiny.neg().to_f64().to_bits(), (1u64 << 63) | 1);
        // Three times it is still subnormal and still exact.
        let three = Rat::new(int(3), Int::one().shl(1_074)).expect("non-zero denominator");
        assert_eq!(three.to_f64().to_bits(), 3);
        // The largest subnormal, (2^52 - 1) * 2^-1074.
        let largest_subnormal =
            Rat::new(Int::one().shl(52).sub(&Int::one()), Int::one().shl(1_074))
                .expect("non-zero denominator");
        assert_eq!(largest_subnormal.to_f64().to_bits(), (1u64 << 52) - 1);
        // The smallest normal, 2^-1022.
        let smallest_normal =
            Rat::new(Int::one(), Int::one().shl(1_022)).expect("non-zero denominator");
        assert_eq!(
            smallest_normal.to_f64().to_bits(),
            f64::MIN_POSITIVE.to_bits()
        );
        // Rounding up out of the subnormal range lands on the smallest normal.
        let just_under = Rat::new(Int::one().shl(53).sub(&Int::one()), Int::one().shl(1_075))
            .expect("non-zero denominator");
        assert_eq!(just_under.to_f64().to_bits(), f64::MIN_POSITIVE.to_bits());
    }

    #[test]
    fn to_f64_underflows_to_positive_zero_and_overflows_to_infinity() {
        assert_eq!(Rat::zero().to_f64().to_bits(), 0);
        // Half the smallest subnormal is a tie that resolves to even, i.e. zero.
        let half_min = Rat::new(Int::one(), Int::one().shl(1_075)).expect("non-zero");
        assert_eq!(half_min.to_f64().to_bits(), 0);
        // Below that, everything underflows — and never to a negative zero.
        let far_under = Rat::new(Int::one(), Int::one().shl(2_000)).expect("non-zero");
        assert_eq!(far_under.to_f64().to_bits(), 0);
        assert_eq!(far_under.neg().to_f64().to_bits(), 0);
        assert!(far_under.neg().to_f64().is_sign_positive());
        // Just above the tie rounds up to the smallest subnormal instead.
        let just_over = Rat::new(int(3), Int::one().shl(1_076)).expect("non-zero");
        assert_eq!(just_over.to_f64().to_bits(), 1);

        assert_eq!(Rat::from_int(Int::pow10(400)).to_f64(), f64::INFINITY);
        assert_eq!(
            Rat::from_int(Int::pow10(400)).neg().to_f64(),
            f64::NEG_INFINITY
        );
        assert_eq!(Rat::from_int(Int::one().shl(1_024)).to_f64(), f64::INFINITY);
        // The largest finite double is still finite: (2^53 - 1) * 2^971.
        let max = Rat::from_int(Int::one().shl(53).sub(&Int::one()).shl(971));
        assert_eq!(max.to_f64().to_bits(), f64::MAX.to_bits());
        // One ulp of rounding above it overflows.
        let over = Rat::from_int(Int::one().shl(54).sub(&Int::one()).shl(970));
        assert_eq!(over.to_f64(), f64::INFINITY);
    }

    // -----------------------------------------------------------------------
    // 12. to_decimal_string and round_to_scale
    // -----------------------------------------------------------------------

    #[test]
    fn to_decimal_string_rounds_half_to_even_at_the_boundary() {
        assert_eq!(rat(1, 2).to_decimal_string(0), "0");
        assert_eq!(rat(3, 2).to_decimal_string(0), "2");
        assert_eq!(rat(5, 2).to_decimal_string(0), "2");
        assert_eq!(rat(7, 2).to_decimal_string(0), "4");
        assert_eq!(rat(-1, 2).to_decimal_string(0), "0");
        assert_eq!(rat(-3, 2).to_decimal_string(0), "-2");
        assert_eq!(rat(-5, 2).to_decimal_string(0), "-2");
        assert_eq!(rat(1, 3).to_decimal_string(0), "0");
        assert_eq!(rat(2, 3).to_decimal_string(0), "1");
    }

    #[test]
    fn to_decimal_string_suppresses_trailing_zeros_and_the_bare_point() {
        assert_eq!(rat(1, 2).to_decimal_string(5), "0.5");
        assert_eq!(rat(1, 8).to_decimal_string(10), "0.125");
        assert_eq!(rat(5, 1).to_decimal_string(7), "5");
        assert_eq!(rat(-5, 1).to_decimal_string(7), "-5");
        assert_eq!(Rat::zero().to_decimal_string(7), "0");
        assert_eq!(Rat::zero().to_decimal_string(0), "0");
        assert_eq!(rat(-1, 100_000).to_decimal_string(5), "-0.00001");
        assert_eq!(rat(-1, 100_000).to_decimal_string(4), "0");
        assert_eq!(rat(123, 100).to_decimal_string(4), "1.23");
        assert_eq!(rat(1, 3).to_decimal_string(5), "0.33333");
        assert_eq!(rat(2, 3).to_decimal_string(5), "0.66667");
        assert_eq!(rat(-2, 3).to_decimal_string(5), "-0.66667");
        assert_eq!(rat(10, 4).to_decimal_string(3), "2.5");
    }

    #[test]
    fn to_decimal_string_never_writes_a_negative_zero() {
        for value in [rat(-1, 2), rat(-1, 3), rat(-1, 1_000_000), rat(-1, 4)] {
            let rendered = value.to_decimal_string(0);
            assert_eq!(rendered, "0", "{value:?} at scale 0");
            assert!(!rendered.starts_with('-'));
        }
        assert_eq!(Rat::zero().neg().to_decimal_string(5), "0");
    }

    #[test]
    fn round_to_scale_rounds_half_to_even() {
        assert_eq!(rat(1, 2).round_to_scale(0), Int::zero());
        assert_eq!(rat(3, 2).round_to_scale(0), int(2));
        assert_eq!(rat(5, 2).round_to_scale(0), int(2));
        assert_eq!(rat(-5, 2).round_to_scale(0), int(-2));
        assert_eq!(rat(1, 4).round_to_scale(1), int(2));
        assert_eq!(rat(3, 4).round_to_scale(1), int(8));
        assert_eq!(rat(-3, 4).round_to_scale(1), int(-8));
        assert_eq!(rat(1, 3).round_to_scale(3), int(333));
        assert_eq!(rat(2, 3).round_to_scale(3), int(667));
        assert_eq!(Rat::from_i64(7).round_to_scale(2), int(700));
        assert_eq!(Rat::zero().round_to_scale(9), Int::zero());
    }

    #[test]
    fn round_to_scale_and_from_decimal_are_inverse_at_exact_scales() {
        for (num, den, scale) in [(1i64, 2i64, 1u32), (1, 8, 3), (-3, 4, 2), (22, 25, 2)] {
            let value = rat(num, den);
            let mantissa = value.round_to_scale(scale);
            assert_eq!(Rat::from_decimal(mantissa, scale), value);
        }
    }

    // -----------------------------------------------------------------------
    // 13. Determinism of the float boundary
    // -----------------------------------------------------------------------

    #[test]
    fn to_f64_depends_only_on_the_value_not_on_how_it_was_built() {
        let spellings = [
            rat(1, 2),
            rat(3, 6),
            rat(-2, -4),
            rat(50, 100),
            Rat::from_decimal(int(5), 1),
            Rat::parse_decimal("0.5").expect("valid"),
            Rat::parse_decimal(".5").expect("valid"),
            Rat::parse_decimal("5e-1").expect("valid"),
            Rat::parse_decimal("50e-2").expect("valid"),
            Rat::parse_decimal("+0.500").expect("valid"),
            Rat::one().div(&Rat::from_i64(2)).expect("non-zero divisor"),
            rat(1, 4).add(&rat(1, 4)),
        ];
        let first = spellings[0].clone();
        for spelling in &spellings {
            assert_eq!(*spelling, first, "{spelling:?} must be the same value");
            assert_eq!(
                spelling.to_f64().to_bits(),
                first.to_f64().to_bits(),
                "{spelling:?} must produce identical bits"
            );
        }
    }

    #[test]
    fn parse_decimal_is_bit_identical_across_lexical_spellings() {
        let groups: [(&[&str], Rat); 4] = [
            (
                &["1", "1.0", "1.", "+1", "01", "0.1e1", "10e-1"],
                Rat::one(),
            ),
            (
                &["0", "-0", "+0", "0.0", ".0", "0e100", "0.000e-5"],
                Rat::zero(),
            ),
            (
                &["-1.5", "-1.50", "-15e-1", "-0.15e1", "-150e-2"],
                rat(-3, 2),
            ),
            (&["0.1", ".1", "1e-1", "10e-2", "0.100", "00.1"], rat(1, 10)),
        ];
        for (spellings, expected) in groups {
            for spelling in spellings {
                let parsed = Rat::parse_decimal(spelling).expect("spelling is valid");
                assert_eq!(parsed, expected, "{spelling:?}");
                assert_eq!(
                    parsed.to_f64().to_bits(),
                    expected.to_f64().to_bits(),
                    "{spelling:?} bits"
                );
                assert_eq!(
                    parsed.numerator(),
                    expected.numerator(),
                    "{spelling:?} numerator"
                );
                assert_eq!(
                    parsed.denominator(),
                    expected.denominator(),
                    "{spelling:?} denominator"
                );
            }
        }
    }

    #[test]
    fn equal_values_hash_and_compare_identically() {
        use std::collections::HashSet;

        let mut set: HashSet<Rat> = HashSet::new();
        set.insert(rat(1, 2));
        assert!(set.contains(&rat(3, 6)));
        assert!(set.contains(&rat(-1, -2)));
        assert!(!set.insert(rat(50, 100)), "an equal value must not be new");
        assert_eq!(set.len(), 1);

        let mut ints: HashSet<Int> = HashSet::new();
        ints.insert(Int::zero());
        assert!(ints.contains(&Int::zero().neg()));
        assert!(ints.contains(&int(5).sub(&int(5))));
        assert_eq!(ints.len(), 1);
    }
}
