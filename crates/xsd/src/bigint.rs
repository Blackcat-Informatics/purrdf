// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The workspace's arbitrary-precision signed binary integer.
//!
//! Three initialized inline limbs hold magnitudes below 2^192; larger values
//! use a canonical little-endian vector. Every operation is integer-only.
//! The exact numeric tower and allocation-bounded arithmetic share this home.

use core::{cmp::Ordering, fmt};
mod compat;
pub use compat::KARATSUBA_THRESHOLD;
#[cfg(test)]
mod arithmetic_tests;
pub(crate) mod scratch;
mod storage;
#[cfg(test)]
mod tower_tests;
use scratch::{Allocate, Unbounded};
pub use scratch::{LimbScratch, LimbScratchError};
pub(crate) use storage::Mag;

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
    if v == 0 {
        Mag::new()
    } else {
        Mag::from_slice(&[v])
    }
}

/// Builds a magnitude from a `u128`.
fn mag_from_u128(v: u128) -> Mag {
    let lo = v as u64;
    let hi = (v >> 64) as u64;
    if hi == 0 {
        mag_from_u64(lo)
    } else {
        Mag::from_slice(&[lo, hi])
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

/// Number of trailing zero bits; zero (which has none) reports zero.
fn mag_trailing_zeros(a: &[u64]) -> u64 {
    match a.iter().position(|&limb| limb != 0) {
        None => 0,
        Some(i) => (i as u64) * 64 + u64::from(a[i].trailing_zeros()),
    }
}

/// Schoolbook addition of two magnitudes.
fn mag_add(a: &[u64], b: &[u64], storage: &impl Allocate) -> Result<Mag, LimbScratchError> {
    let (long, short) = if a.len() >= b.len() { (a, b) } else { (b, a) };
    let mut out: Mag = storage.destination(long.len() + 1, 0)?;
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
    Ok(out)
}

/// Schoolbook subtraction of two magnitudes; `a` must be at least `b`.
fn mag_sub(a: &[u64], b: &[u64], storage: &impl Allocate) -> Result<Mag, LimbScratchError> {
    let mut out = storage.copy(a)?;
    mag_sub_in_place(&mut out, b);
    Ok(out)
}

/// The one borrow propagation used by subtraction and binary GCD.
fn mag_sub_in_place(a: &mut Mag, b: &[u64]) {
    debug_assert!(mag_cmp(a, b) != Ordering::Less, "mag_sub would go negative");
    let mut borrow = false;
    for (i, limb) in a.iter_mut().enumerate() {
        let (partial, borrowed_a) = limb.overflowing_sub(b.get(i).copied().unwrap_or(0));
        let (difference, borrowed_b) = partial.overflowing_sub(u64::from(borrow));
        *limb = difference;
        borrow = borrowed_a || borrowed_b;
    }
    debug_assert!(!borrow, "mag_sub called with a < b");
    mag_trim(a);
}

/// Schoolbook multiplication of two magnitudes.
fn mag_mul(a: &[u64], b: &[u64], storage: &impl Allocate) -> Result<Mag, LimbScratchError> {
    if a.is_empty() || b.is_empty() {
        return Ok(Mag::new());
    }
    if let (Some(x), Some(y)) = (mag_to_u64(a), mag_to_u64(b)) {
        // Fast path: the whole product fits in a `u128`. This is the hot path
        // for coordinate arithmetic, where operands are small integers.
        return Ok(mag_from_u128(u128::from(x) * u128::from(y)));
    }
    let mut out: Mag = storage.destination(
        a.len()
            .checked_add(b.len())
            .ok_or(LimbScratchError::SizeOverflow)?,
        a.len()
            .checked_add(b.len())
            .ok_or(LimbScratchError::SizeOverflow)?,
    )?;
    write_mul(a, b, &mut out);
    mag_trim(&mut out);
    Ok(out)
}

/// The single schoolbook product into initialized, complete destination limbs.
fn write_mul(a: &[u64], b: &[u64], out: &mut [u64]) {
    debug_assert_eq!(out.len(), a.len() + b.len());
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
}

/// Borrow a product's exact limbs directly into a following operation. Common
/// binary64/fixed products use bounded stack storage; longer products use the
/// same explicit allocator and never grow a scratch destination.
fn with_product<T>(
    a: &[u64],
    b: &[u64],
    storage: &impl Allocate,
    evaluate: impl FnOnce(&[u64]) -> Result<T, LimbScratchError>,
) -> Result<T, LimbScratchError> {
    let length = a
        .len()
        .checked_add(b.len())
        .ok_or(LimbScratchError::SizeOverflow)?;
    if a.is_empty() || b.is_empty() {
        return evaluate(&[]);
    }
    if length <= 8 {
        let mut words = [0; 8];
        write_mul(a, b, &mut words[..length]);
        let retained = words[..length]
            .iter()
            .rposition(|&word| word != 0)
            .map_or(0, |index| index + 1);
        evaluate(&words[..retained])
    } else {
        let product = mag_mul(a, b, storage)?;
        evaluate(&product)
    }
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
fn mag_mul_small_add(
    a: &[u64],
    multiplier: u64,
    addend: u64,
    storage: &impl Allocate,
) -> Result<Mag, LimbScratchError> {
    let mut out: Mag = storage.destination(
        a.len()
            .checked_add(1)
            .ok_or(LimbScratchError::SizeOverflow)?,
        0,
    )?;
    let mut carry = u128::from(addend);
    for &limb in a {
        let cur = u128::from(limb) * u128::from(multiplier) + carry;
        storage.push(&mut out, cur as u64)?;
        carry = cur >> 64;
    }
    while carry != 0 {
        storage.push(&mut out, carry as u64)?;
        carry >>= 64;
    }
    mag_trim(&mut out);
    Ok(out)
}

/// Divides a magnitude by a single non-zero limb, returning quotient and remainder.
fn mag_divmod_small(
    a: &[u64],
    divisor: u64,
    storage: &impl Allocate,
) -> Result<(Mag, u64), LimbScratchError> {
    debug_assert_ne!(divisor, 0, "division by zero");
    let d = u128::from(divisor);
    let mut out: Mag = storage.destination(a.len(), a.len())?;
    let mut rem: u128 = 0;
    for (slot, &limb) in out.iter_mut().zip(a.iter()).rev() {
        let cur = (rem << 64) | u128::from(limb);
        *slot = (cur / d) as u64;
        rem = cur % d;
    }
    mag_trim(&mut out);
    Ok((out, rem as u64))
}

/// Writes a sub-limb left shift into initialized storage.
///
/// The destination must have room for a nonzero final carry. Normalized
/// division and arbitrary left shifts share this one carry propagation.
fn write_shl(a: &[u64], bits: u32, out: &mut [u64]) {
    debug_assert!(bits < 64);
    debug_assert!(out.len() >= a.len());
    if bits == 0 {
        out[..a.len()].copy_from_slice(a);
        return;
    }
    let mut carry = 0;
    for (&limb, slot) in a.iter().zip(out.iter_mut()) {
        *slot = (limb << bits) | carry;
        carry = limb >> (64 - bits);
    }
    if out.len() > a.len() {
        out[a.len()] = carry;
    } else {
        debug_assert_eq!(carry, 0, "shift destination omits nonzero carry");
    }
}

/// Shifts a magnitude left by `bits`.
fn mag_shl(a: &[u64], bits: u64, storage: &impl Allocate) -> Result<Mag, LimbScratchError> {
    if a.is_empty() {
        return Ok(Mag::new());
    }
    let limb_shift = usize::try_from(bits / 64).map_err(|_| LimbScratchError::SizeOverflow)?;
    let bit_shift = (bits % 64) as u32;
    let carry = bit_shift != 0 && a[a.len() - 1] >> (64 - bit_shift) != 0;
    let length = a
        .len()
        .checked_add(limb_shift)
        .and_then(|n| n.checked_add(usize::from(carry)))
        .ok_or(LimbScratchError::SizeOverflow)?;
    let mut out = storage.destination(length, length)?;
    write_shl(a, bit_shift, &mut out[limb_shift..]);
    Ok(out)
}

/// Shifts a magnitude right by `bits`, discarding the bits shifted out.
fn mag_shr(a: &[u64], bits: u64, storage: &impl Allocate) -> Result<Mag, LimbScratchError> {
    let length = mag_bit_len(a).saturating_sub(bits).div_ceil(64) as usize;
    let mut out = storage.destination(length, length)?;
    let offset = (bits / 64) as usize;
    let partial = (bits % 64) as u32;
    for (index, word) in out.iter_mut().enumerate() {
        *word = shifted_right_limb(a, index + offset, partial);
    }
    Ok(out)
}

fn shifted_right_limb(a: &[u64], source: usize, partial: u32) -> u64 {
    if partial == 0 {
        a[source]
    } else {
        (a[source] >> partial) | (a.get(source + 1).copied().unwrap_or(0) << (64 - partial))
    }
}

/// Right-shift propagation shared by division denormalization and binary GCD.
fn mag_shr_in_place(a: &mut Mag, bits: u64) {
    let limb_shift = (bits / 64) as usize;
    if limb_shift >= a.len() {
        a.resize(0, 0);
        return;
    }
    let bit_shift = (bits % 64) as u32;
    let length = a.len() - limb_shift;
    // Every source index is at or above its destination. Forward writes cannot
    // overwrite a source before its final read, including zero whole limbs.
    for i in 0..length {
        let source = i + limb_shift;
        a[i] = shifted_right_limb(a, source, bit_shift);
    }
    a.resize(length, 0);
    mag_trim(a);
}

/// Divides normalized base-2^64 limbs, leaving the normalized remainder in `u`.
///
/// Let B=2^64 and n=v.len(). The top divisor limb is at least B/2, and u has
/// one explicit guard limb. At quotient position j, division of the top two
/// numerator limbs estimates the digit from above. Testing the next divisor
/// limb reduces that estimate by at most two; subtraction of the full product
/// needs at most one add-back. Thus each quotient digit costs O(n), without
/// constructing a magnitude at every dividend bit.
fn divide_normalized(u: &mut [u64], v: &[u64], quotient: &mut [u64]) {
    let n = v.len();
    debug_assert!(n >= 2 && v[n - 1] >> 63 == 1);
    debug_assert_eq!(u.len(), n + quotient.len());
    let base = 1u128 << 64;
    let top = u128::from(v[n - 1]);
    for j in (0..quotient.len()).rev() {
        debug_assert!(u128::from(u[j + n]) <= top);
        let (mut digit, mut remainder) = if u128::from(u[j + n]) == top {
            (u64::MAX, u128::from(u[j + n - 1]) + top)
        } else {
            let pair = (u128::from(u[j + n]) << 64) | u128::from(u[j + n - 1]);
            ((pair / top) as u64, pair % top)
        };
        while remainder < base
            && u128::from(digit) * u128::from(v[n - 2])
                > (remainder << 64) + u128::from(u[j + n - 2])
        {
            digit -= 1;
            remainder += top;
        }
        let mut carry = 0u128;
        for (i, &limb) in v.iter().enumerate() {
            let product = u128::from(digit) * u128::from(limb) + carry;
            let (difference, borrowed) = u[j + i].overflowing_sub(product as u64);
            u[j + i] = difference;
            carry = (product >> 64) + u128::from(borrowed);
        }
        let high = u128::from(u[j + n]);
        u[j + n] = high.wrapping_sub(carry) as u64;
        if high < carry {
            digit -= 1;
            let mut add_carry = 0u128;
            for (i, &limb) in v.iter().enumerate() {
                let sum = u128::from(u[j + i]) + u128::from(limb) + add_carry;
                u[j + i] = sum as u64;
                add_carry = sum >> 64;
            }
            u[j + n] = u[j + n].wrapping_add(add_carry as u64);
        }
        quotient[j] = digit;
    }
}

/// Divides magnitudes, returning `(quotient, remainder)`; `b` must be non-zero.
///
/// Single-limb, u128 and power-of-two divisors have specialized exact paths.
/// Other operands use normalized limb division. The three-limb common path
/// uses stack scratch, preserving allocation-free inline integer arithmetic.
fn mag_div_rem(
    a: &[u64],
    b: &[u64],
    storage: &impl Allocate,
) -> Result<(Mag, Mag), LimbScratchError> {
    debug_assert!(!b.is_empty(), "division by zero");
    if mag_cmp(a, b) == Ordering::Less {
        return Ok((Mag::new(), storage.copy(a)?));
    }
    if b.len() == 1 {
        let (q, r) = mag_divmod_small(a, b[0], storage)?;
        return Ok((q, mag_from_u64(r)));
    }
    if let (Some(x), Some(y)) = (mag_to_u128(a), mag_to_u128(b)) {
        return Ok((mag_from_u128(x / y), mag_from_u128(x % y)));
    }
    let power = mag_trailing_zeros(b);
    if power + 1 == mag_bit_len(b) {
        let quotient = mag_shr(a, power, storage)?;
        let whole = (power / 64) as usize;
        let partial = (power % 64) as u32;
        let retained = whole + usize::from(partial != 0);
        let mut remainder = storage.copy(&a[..retained.min(a.len())])?;
        if partial != 0 && whole < remainder.len() {
            remainder[whole] &= (1u64 << partial) - 1;
        }
        mag_trim(&mut remainder);
        return Ok((quotient, remainder));
    }
    let shift = b[b.len() - 1].leading_zeros();
    let quotient_length = a.len() - b.len() + 1;
    if a.len() <= 8 {
        let mut numerator = [0; 9];
        let mut divisor = [0; 8];
        let mut digits = [0; 8];
        write_shl(a, shift, &mut numerator[..=a.len()]);
        write_shl(b, shift, &mut divisor[..b.len()]);
        divide_normalized(
            &mut numerator[..=a.len()],
            &divisor[..b.len()],
            &mut digits[..quotient_length],
        );
        let mut quotient = storage.copy(&digits[..quotient_length])?;
        let mut remainder = storage.copy(&numerator[..b.len()])?;
        mag_trim(&mut quotient);
        mag_trim(&mut remainder);
        mag_shr_in_place(&mut remainder, u64::from(shift));
        return Ok((quotient, remainder));
    }
    let length = a
        .len()
        .checked_add(1)
        .ok_or(LimbScratchError::SizeOverflow)?;
    let mut numerator = storage.destination(length, length)?;
    let mut divisor = storage.destination(b.len(), b.len())?;
    let mut quotient = storage.destination(quotient_length, quotient_length)?;
    write_shl(a, shift, &mut numerator);
    write_shl(b, shift, &mut divisor);
    divide_normalized(&mut numerator, &divisor, &mut quotient);
    numerator.resize(b.len(), 0);
    mag_trim(&mut quotient);
    mag_trim(&mut numerator);
    mag_shr_in_place(&mut numerator, u64::from(shift));
    Ok((quotient, numerator))
}

/// Stein's binary greatest common divisor on magnitudes.
fn mag_gcd(a: &[u64], b: &[u64], storage: &impl Allocate) -> Result<Mag, LimbScratchError> {
    if a.is_empty() {
        return storage.copy(b);
    }
    if b.is_empty() {
        return storage.copy(a);
    }
    if mag_is_one(a) || mag_is_one(b) {
        return Ok(Mag::from_slice(&[1]));
    }
    if let (Some(x), Some(y)) = (mag_to_u128(a), mag_to_u128(b)) {
        return Ok(mag_from_u128(crate::wide::gcd(x, y)));
    }
    let common = mag_trailing_zeros(a).min(mag_trailing_zeros(b));
    let mut u = mag_shr(a, mag_trailing_zeros(a), storage)?;
    let mut v: Mag = storage.copy(b)?;
    loop {
        let zeros = mag_trailing_zeros(&v);
        mag_shr_in_place(&mut v, zeros);
        if mag_cmp(&u, &v) == Ordering::Greater {
            core::mem::swap(&mut u, &mut v);
        }
        mag_sub_in_place(&mut v, &u);
        if v.is_empty() {
            break;
        }
    }
    mag_shl(&u, common, storage)
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

fn mag_sqrt_floor(value: &[u64], storage: &impl Allocate) -> Result<Mag, LimbScratchError> {
    if value.is_empty() {
        return Ok(Mag::new());
    }
    if let Some(v) = mag_to_u128(value) {
        return Ok(mag_from_u128(isqrt_u128(v)));
    }
    let bits = mag_bit_len(value);
    // `x0 = 2^ceil(bits/2)` is at least `sqrt(self)`, which is what makes
    // the monotone-descent termination test below correct.
    let mut x = mag_shl(&[1u64], bits.div_ceil(2), storage)?;
    loop {
        let (d, _) = mag_div_rem(value, &x, storage)?;
        let y = mag_shr(&mag_add(&x, &d, storage)?, 1, storage)?;
        if mag_cmp(&y, &x) != Ordering::Less {
            break;
        }
        x = y;
    }
    Ok(x)
}

/// Accumulates a run of ASCII digit bytes into a magnitude.
///
/// The caller must have already established that every byte is an ASCII digit.
fn mag_from_digit_bytes(
    digits: impl Iterator<Item = u8>,
    storage: &impl Allocate,
) -> Result<Option<Mag>, LimbScratchError> {
    let mut acc = Mag::new();
    let mut any = false;
    let mut chunk_value = 0u64;
    let mut chunk_len = 0usize;
    for byte in digits {
        if !byte.is_ascii_digit() {
            return Ok(None);
        }
        any = true;
        chunk_value = chunk_value * 10 + u64::from(byte - b'0');
        chunk_len += 1;
        if chunk_len == DECIMAL_CHUNK {
            acc = mag_mul_small_add(&acc, CHUNK_BASE, chunk_value, storage)?;
            chunk_value = 0;
            chunk_len = 0;
        }
    }
    if chunk_len > 0 {
        acc = mag_mul_small_add(&acc, POW10_U64[chunk_len], chunk_value, storage)?;
    }
    Ok(any.then_some(acc))
}

// ---------------------------------------------------------------------------
// BigInt
// ---------------------------------------------------------------------------

/// Prepared decimal groups from the native 10^19 conversion, with no lexical String.
/// Callers retain their admitted render frame while this temporary is alive.
#[derive(Debug)]
pub struct PreparedDecimalDigits {
    negative: bool,
    groups: Mag,
    digits: usize,
}

impl PreparedDecimalDigits {
    /// Exact unsigned coefficient length, available before output allocation.
    #[must_use]
    pub const fn digits_len(&self) -> usize {
        self.digits
    }
    /// Sign of the original value.
    #[must_use]
    pub const fn is_negative(&self) -> bool {
        self.negative
    }
    /// Exact signed text length.
    #[must_use]
    pub fn len(&self) -> usize {
        self.digits + usize::from(self.negative)
    }
    /// Decimal text is never empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        false
    }
    /// Actual temporary group capacity that remains live after conversion.
    #[must_use]
    pub fn heap_bytes(&self) -> usize {
        self.groups.allocated_bytes()
    }

    pub(crate) fn write_unsigned<W: fmt::Write + ?Sized>(&self, out: &mut W) -> fmt::Result {
        let mut groups = self.groups.iter().rev();
        let leading = groups.next().expect("prepared zero includes one group");
        write!(out, "{leading}")?;
        for group in groups {
            write!(out, "{group:019}")?;
        }
        Ok(())
    }

    pub(crate) fn write_fmt<W: fmt::Write + ?Sized>(&self, out: &mut W) -> fmt::Result {
        if self.negative {
            out.write_str("-")?;
        }
        self.write_unsigned(out)
    }

    /// Append the complete lexical without growing a caller-owned String.
    /// # Errors
    /// Refuses all insufficient capacity before the first output write.
    pub fn write_to(&self, out: &mut String) -> Result<(), crate::numeric::NumericRenderError> {
        let available = out.capacity() - out.len();
        if self.len() > available {
            return Err(crate::numeric::NumericRenderError::DestinationTooSmall {
                required_bytes: self.len(),
                available_bytes: available,
            });
        }
        self.write_fmt(out)
            .expect("prepared digits fit the checked String capacity");
        Ok(())
    }
}

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
pub struct BigInt {
    /// Sign flag; always `false` when the magnitude is empty.
    negative: bool,
    /// Little-endian base-2^64 magnitude with no trailing zero limbs.
    magnitude: Mag,
}

impl BigInt {
    /// The value zero.
    #[must_use]
    pub const fn zero() -> Self {
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
            magnitude: Mag::from_slice(&[1]),
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

    /// Copy an operand through the selected native destination seam.
    pub(crate) fn copy_using(&self, storage: &impl Allocate) -> Result<Self, LimbScratchError> {
        Ok(Self::from_parts(
            self.negative,
            storage.copy(&self.magnitude)?,
        ))
    }

    pub(crate) fn abs_using(&self, storage: &impl Allocate) -> Result<Self, LimbScratchError> {
        Ok(self.copy_using(storage)?.with_sign(false))
    }

    pub(crate) fn neg_using(&self, storage: &impl Allocate) -> Result<Self, LimbScratchError> {
        Ok(self.copy_using(storage)?.with_sign(!self.negative))
    }

    /// Change only the sign of an owned magnitude; no clone or allocation.
    pub(crate) fn with_sign(mut self, negative: bool) -> Self {
        self.negative = negative && !self.magnitude.is_empty();
        self
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
        self.add_using(other, &Unbounded)
            .expect("unbounded integer storage")
    }

    /// The same exact operation using only admitted reusable limb destinations.
    /// # Errors
    /// Refuses exhausted destination count or limb capacity without allocating.
    pub fn add_in(&self, other: &Self, scratch: &LimbScratch) -> Result<Self, LimbScratchError> {
        self.add_using(other, scratch)
    }

    pub(crate) fn add_using(
        &self,
        other: &Self,
        storage: &impl Allocate,
    ) -> Result<Self, LimbScratchError> {
        self.combine_using(other, other.negative, storage)
    }

    fn combine_using(
        &self,
        other: &Self,
        other_negative: bool,
        storage: &impl Allocate,
    ) -> Result<Self, LimbScratchError> {
        if self.negative == other_negative {
            Ok(Self::from_parts(
                self.negative,
                mag_add(&self.magnitude, &other.magnitude, storage)?,
            ))
        } else {
            match mag_cmp(&self.magnitude, &other.magnitude) {
                Ordering::Equal => Ok(Self::zero()),
                Ordering::Greater => Ok(Self::from_parts(
                    self.negative,
                    mag_sub(&self.magnitude, &other.magnitude, storage)?,
                )),
                Ordering::Less => Ok(Self::from_parts(
                    other_negative,
                    mag_sub(&other.magnitude, &self.magnitude, storage)?,
                )),
            }
        }
    }

    /// Returns `self - other`.
    #[must_use]
    pub fn sub(&self, other: &Self) -> Self {
        self.sub_using(other, &Unbounded)
            .expect("unbounded integer storage")
    }

    /// The same exact operation using only admitted reusable limb destinations.
    /// # Errors
    /// Refuses exhausted destination count or limb capacity without allocating.
    pub fn sub_in(&self, other: &Self, scratch: &LimbScratch) -> Result<Self, LimbScratchError> {
        self.sub_using(other, scratch)
    }

    pub(crate) fn sub_using(
        &self,
        other: &Self,
        storage: &impl Allocate,
    ) -> Result<Self, LimbScratchError> {
        self.combine_using(other, !other.negative, storage)
    }

    /// Returns `self * other`.
    #[must_use]
    pub fn mul(&self, other: &Self) -> Self {
        self.mul_using(other, &Unbounded)
            .expect("unbounded integer storage")
    }

    /// The same exact operation using only admitted reusable limb destinations.
    /// # Errors
    /// Refuses exhausted destination count or limb capacity without allocating.
    pub fn mul_in(&self, other: &Self, scratch: &LimbScratch) -> Result<Self, LimbScratchError> {
        self.mul_using(other, scratch)
    }

    pub(crate) fn mul_using(
        &self,
        other: &Self,
        storage: &impl Allocate,
    ) -> Result<Self, LimbScratchError> {
        Ok(Self::from_parts(
            self.negative != other.negative,
            mag_mul(&self.magnitude, &other.magnitude, storage)?,
        ))
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
        self.div_rem_using(other, &Unbounded)
            .expect("unbounded integer storage")
    }

    /// The same exact operation using only admitted reusable limb destinations.
    /// # Errors
    /// Refuses exhausted destination count or limb capacity without allocating.
    pub fn div_rem_in(
        &self,
        other: &Self,
        scratch: &LimbScratch,
    ) -> Result<Option<(Self, Self)>, LimbScratchError> {
        self.div_rem_using(other, scratch)
    }

    pub(crate) fn div_rem_using(
        &self,
        other: &Self,
        storage: &impl Allocate,
    ) -> Result<Option<(Self, Self)>, LimbScratchError> {
        if other.is_zero() {
            return Ok(None);
        }
        let (q, r) = mag_div_rem(&self.magnitude, &other.magnitude, storage)?;
        Ok(Some((
            Self::from_parts(self.negative != other.negative, q),
            Self::from_parts(self.negative, r),
        )))
    }

    /// Returns the greatest common divisor of `self` and `other`.
    ///
    /// The result is always non-negative; signs of the operands are ignored.
    /// `gcd(0, n) == |n|`, `gcd(n, 0) == |n|` and `gcd(0, 0) == 0`.
    #[must_use]
    pub fn gcd(&self, other: &Self) -> Self {
        self.gcd_using(other, &Unbounded)
            .expect("unbounded integer storage")
    }

    /// The same exact operation using only admitted reusable limb destinations.
    /// # Errors
    /// Refuses exhausted destination count or limb capacity without allocating.
    pub fn gcd_in(&self, other: &Self, scratch: &LimbScratch) -> Result<Self, LimbScratchError> {
        self.gcd_using(other, scratch)
    }

    pub(crate) fn gcd_using(
        &self,
        other: &Self,
        storage: &impl Allocate,
    ) -> Result<Self, LimbScratchError> {
        Ok(Self::from_parts(
            false,
            mag_gcd(&self.magnitude, &other.magnitude, storage)?,
        ))
    }

    /// Returns 10<sup>`exp`</sup>.
    #[must_use]
    pub fn pow10(exp: u32) -> Self {
        Self::pow10_using(exp, &Unbounded).expect("unbounded integer storage")
    }

    /// The same exact operation using only admitted reusable limb destinations.
    /// # Errors
    /// Refuses exhausted destination count or limb capacity without allocating.
    pub fn pow10_in(exp: u32, scratch: &LimbScratch) -> Result<Self, LimbScratchError> {
        Self::pow10_using(exp, scratch)
    }

    pub(crate) fn pow10_using(exp: u32, storage: &impl Allocate) -> Result<Self, LimbScratchError> {
        Self::one().mul_power_using(10, exp, DECIMAL_CHUNK as u32, storage)
    }

    /// Exact multiplication by a power of five through limb-sized factors.
    #[must_use]
    pub fn mul_pow5(&self, exp: u32) -> Self {
        self.mul_pow5_using(exp, &Unbounded)
            .expect("unbounded integer storage")
    }

    /// The same power multiplication in admitted reusable destinations.
    /// # Errors
    /// Refuses destination exhaustion or limb capacity without allocating.
    pub fn mul_pow5_in(&self, exp: u32, scratch: &LimbScratch) -> Result<Self, LimbScratchError> {
        self.mul_pow5_using(exp, scratch)
    }

    pub(crate) fn mul_pow5_using(
        &self,
        exp: u32,
        storage: &impl Allocate,
    ) -> Result<Self, LimbScratchError> {
        self.mul_power_using(5, exp, 27, storage)
    }

    fn mul_power_using(
        &self,
        base: u64,
        exp: u32,
        chunk: u32,
        storage: &impl Allocate,
    ) -> Result<Self, LimbScratchError> {
        self.mul_power_wide_using(base, u64::from(exp), chunk, storage)
    }

    pub(crate) fn pow10_wide_using(
        exp: u64,
        storage: &impl Allocate,
    ) -> Result<Self, LimbScratchError> {
        Self::one().mul_power_wide_using(10, exp, DECIMAL_CHUNK as u32, storage)
    }

    fn mul_power_wide_using(
        &self,
        base: u64,
        exp: u64,
        chunk: u32,
        storage: &impl Allocate,
    ) -> Result<Self, LimbScratchError> {
        let mut magnitude = storage.copy(&self.magnitude)?;
        let mut remaining = exp;
        while remaining != 0 {
            let step =
                u32::try_from(remaining.min(u64::from(chunk))).expect("bounded by a machine chunk");
            magnitude = mag_mul_small_add(&magnitude, base.pow(step), 0, storage)?;
            remaining -= u64::from(step);
        }
        Ok(Self::from_parts(self.negative, magnitude))
    }

    /// Remove at most `limit` exact factors of five. Full 27-factor chunks
    /// precede at most 26 single-factor divisions. Sign is preserved.
    pub(crate) fn strip_fives_using(
        &self,
        limit: u32,
        storage: &impl Allocate,
    ) -> Result<(Self, u32), LimbScratchError> {
        let (value, removed) = self.strip_fives_wide_using(u64::from(limit), storage)?;
        Ok((value, removed as u32))
    }

    /// The identical prime-removal body with a full integer-width counter.
    /// Returned counts are bounded by both `limit` and the input bit length.
    pub(crate) fn strip_fives_wide_using(
        &self,
        limit: u64,
        storage: &impl Allocate,
    ) -> Result<(Self, u64), LimbScratchError> {
        if self.is_zero() {
            return Ok((Self::zero(), 0));
        }
        let mut value = Self::from_parts(self.negative, storage.copy(&self.magnitude)?);
        let mut remaining = limit;
        let block = Self::from_u64(5_u64.pow(27));
        while remaining >= 27 {
            let (quotient, remainder) = value
                .div_rem_using(&block, storage)?
                .expect("nonzero factor");
            if !remainder.is_zero() {
                break;
            }
            value = quotient;
            remaining -= 27;
        }
        let factor = Self::from_u64(5);
        for _ in 0..remaining.min(26) {
            let (quotient, remainder) = value
                .div_rem_using(&factor, storage)?
                .expect("nonzero factor");
            if !remainder.is_zero() {
                break;
            }
            value = quotient;
            remaining -= 1;
        }
        Ok((value, limit - remaining))
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

    /// Number of low zero magnitude bits, or `None` for the integer zero.
    #[must_use]
    pub fn trailing_zeros(&self) -> Option<u64> {
        (!self.is_zero()).then(|| mag_trailing_zeros(&self.magnitude))
    }

    /// Shifts the magnitude left by `bits`, preserving the sign.
    ///
    /// This multiplies by 2<sup>`bits`</sup> for every value, negative included.
    #[must_use]
    pub fn shl(&self, bits: u32) -> Self {
        self.shl_using(bits, &Unbounded)
            .expect("unbounded integer storage")
    }

    /// The same exact operation using only admitted reusable limb destinations.
    /// # Errors
    /// Refuses exhausted destination count or limb capacity without allocating.
    pub fn shl_in(&self, bits: u32, scratch: &LimbScratch) -> Result<Self, LimbScratchError> {
        self.shl_using(bits, scratch)
    }

    pub(crate) fn shl_using(
        &self,
        bits: u32,
        storage: &impl Allocate,
    ) -> Result<Self, LimbScratchError> {
        Ok(Self::from_parts(
            self.negative,
            mag_shl(&self.magnitude, u64::from(bits), storage)?,
        ))
    }

    /// Shifts the magnitude right by `bits`, preserving the sign.
    ///
    /// The shift is applied to the **magnitude**, so it truncates **toward
    /// zero** rather than toward negative infinity: `(-5).shr(1)` is `-2`, not
    /// `-3`. This is division by 2<sup>`bits`</sup> under the same rounding rule
    /// [`BigInt::div_rem`] uses, not Rust's arithmetic `>>` on a signed primitive.
    #[must_use]
    pub fn shr(&self, bits: u32) -> Self {
        self.shr_using(bits, &Unbounded)
            .expect("unbounded integer storage")
    }

    /// The same exact operation using only admitted reusable limb destinations.
    /// # Errors
    /// Refuses exhausted destination count or limb capacity without allocating.
    pub fn shr_in(&self, bits: u32, scratch: &LimbScratch) -> Result<Self, LimbScratchError> {
        self.shr_using(bits, scratch)
    }

    pub(crate) fn shr_using(
        &self,
        bits: u32,
        storage: &impl Allocate,
    ) -> Result<Self, LimbScratchError> {
        self.shr_wide_using(u64::from(bits), storage)
    }

    /// The same magnitude shift at the integer home's full bit-count width.
    pub(crate) fn shr_wide_using(
        &self,
        bits: u64,
        storage: &impl Allocate,
    ) -> Result<Self, LimbScratchError> {
        Ok(Self::from_parts(
            self.negative,
            mag_shr(&self.magnitude, bits, storage)?,
        ))
    }

    /// Returns the exact integer floor square root, or `None` when negative.
    ///
    /// The result `r` satisfies `r * r <= self < (r + 1) * (r + 1)`.
    pub fn sqrt_floor(&self) -> Option<Self> {
        self.sqrt_floor_using(&Unbounded)
            .expect("unbounded integer storage")
    }

    /// The same exact operation using only admitted reusable limb destinations.
    /// # Errors
    /// Refuses exhausted destination count or limb capacity without allocating.
    pub fn sqrt_floor_in(&self, scratch: &LimbScratch) -> Result<Option<Self>, LimbScratchError> {
        self.sqrt_floor_using(scratch)
    }

    fn sqrt_floor_using(&self, storage: &impl Allocate) -> Result<Option<Self>, LimbScratchError> {
        if self.negative {
            return Ok(None);
        }
        Ok(Some(Self::from_parts(
            false,
            mag_sqrt_floor(&self.magnitude, storage)?,
        )))
    }

    /// Exact product followed by truncated division, without retaining a heap
    /// product. The same product/division bodies serve ordinary operations.
    /// # Errors
    /// Refuses scratch exhaustion; a zero divisor returns `Ok(None)`.
    pub fn mul_div_rem_in(
        &self,
        factor: &Self,
        divisor: &Self,
        scratch: &LimbScratch,
    ) -> Result<Option<(Self, Self)>, LimbScratchError> {
        self.mul_div_rem_using(factor, divisor, scratch)
    }

    /// Exact fused product/division using ordinary integer storage.
    #[must_use]
    pub fn mul_div_rem(&self, factor: &Self, divisor: &Self) -> Option<(Self, Self)> {
        self.mul_div_rem_using(factor, divisor, &Unbounded)
            .expect("unbounded integer storage")
    }
    pub(crate) fn mul_div_rem_using(
        &self,
        factor: &Self,
        divisor: &Self,
        storage: &impl Allocate,
    ) -> Result<Option<(Self, Self)>, LimbScratchError> {
        if divisor.is_zero() {
            return Ok(None);
        }
        with_product(&self.magnitude, &factor.magnitude, storage, |product| {
            let (q, r) = mag_div_rem(product, &divisor.magnitude, storage)?;
            let negative = self.negative != factor.negative;
            Ok(Some((
                Self::from_parts(negative != divisor.negative, q),
                Self::from_parts(negative, r),
            )))
        })
    }

    /// Exact floor root of a nonnegative product without retaining that product.
    /// # Errors
    /// Refuses scratch exhaustion; a negative product returns `Ok(None)`.
    pub fn sqrt_product_floor_in(
        &self,
        factor: &Self,
        scratch: &LimbScratch,
    ) -> Result<Option<Self>, LimbScratchError> {
        self.sqrt_product_using(factor, scratch)
    }
    /// Exact fused product/root using ordinary integer storage.
    #[must_use]
    pub fn sqrt_product_floor(&self, factor: &Self) -> Option<Self> {
        self.sqrt_product_using(factor, &Unbounded)
            .expect("unbounded integer storage")
    }
    fn sqrt_product_using(
        &self,
        factor: &Self,
        storage: &impl Allocate,
    ) -> Result<Option<Self>, LimbScratchError> {
        if self.is_zero() || factor.is_zero() {
            return Ok(Some(Self::zero()));
        }
        if self.negative != factor.negative {
            return Ok(None);
        }
        with_product(&self.magnitude, &factor.magnitude, storage, |product| {
            Ok(Some(Self::from_parts(
                false,
                mag_sqrt_floor(product, storage)?,
            )))
        })
    }

    /// Compare two exact products without retaining either common stack product.
    /// # Errors
    /// Refuses scratch limb/destination exhaustion without allocating a fallback.
    pub fn compare_products_in(
        a: &Self,
        b: &Self,
        c: &Self,
        d: &Self,
        scratch: &LimbScratch,
    ) -> Result<Ordering, LimbScratchError> {
        Self::compare_products_using(a, b, c, d, scratch)
    }
    /// Compare two exact products using ordinary integer storage.
    #[must_use]
    pub fn compare_products(a: &Self, b: &Self, c: &Self, d: &Self) -> Ordering {
        Self::compare_products_using(a, b, c, d, &Unbounded).expect("unbounded integer storage")
    }
    pub(crate) fn compare_products_using(
        a: &Self,
        b: &Self,
        c: &Self,
        d: &Self,
        storage: &impl Allocate,
    ) -> Result<Ordering, LimbScratchError> {
        let first_negative = !a.is_zero() && !b.is_zero() && a.negative != b.negative;
        let second_negative = !c.is_zero() && !d.is_zero() && c.negative != d.negative;
        if first_negative != second_negative {
            return Ok(if first_negative {
                Ordering::Less
            } else {
                Ordering::Greater
            });
        }
        with_product(&a.magnitude, &b.magnitude, storage, |first| {
            with_product(&c.magnitude, &d.magnitude, storage, |second| {
                let order = mag_cmp(first, second);
                Ok(if first_negative {
                    order.reverse()
                } else {
                    order
                })
            })
        })
    }

    /// Parses a non-empty run of ASCII digits, with no sign and no separators.
    ///
    /// Returns `None` for an empty string or for any byte that is not `0`–`9`.
    /// Leading zeros are accepted and carry no meaning.
    pub fn from_decimal_digits(digits: &str) -> Option<Self> {
        Self::from_decimal_bytes(digits.bytes())
    }

    /// Parses a nonempty sequence of ASCII digits without allocating a string.
    /// Sign and separators are refused; leading zeros are accepted.
    pub fn from_decimal_bytes(digits: impl Iterator<Item = u8>) -> Option<Self> {
        Self::from_decimal_bytes_using(digits, false, &Unbounded)
            .expect("unbounded integer storage")
    }

    /// Parse decimal digit bytes with fallible owned destinations. A sign flag
    /// consumes the resulting magnitude, so a negative input never clones it.
    /// Invalid digits return None, separately from physical storage refusal.
    ///
    /// # Errors
    /// Returns checked layout or allocator refusal without an infallible fallback.
    pub fn try_from_decimal_bytes(
        digits: impl Iterator<Item = u8>,
        negative: bool,
    ) -> Result<Option<Self>, LimbScratchError> {
        Self::from_decimal_bytes_using(digits, negative, &scratch::Fallible)
    }

    pub(crate) fn from_decimal_bytes_using(
        digits: impl Iterator<Item = u8>,
        negative: bool,
        storage: &impl Allocate,
    ) -> Result<Option<Self>, LimbScratchError> {
        mag_from_digit_bytes(digits, storage)
            .map(|magnitude| magnitude.map(|magnitude| Self::from_parts(negative, magnitude)))
    }

    /// Builds a non-negative integer from every `u128` magnitude.
    #[must_use]
    pub fn from_u128(value: u128) -> Self {
        Self::from_parts(false, mag_from_u128(value))
    }

    /// Canonical little-endian binary limbs, with no most-significant zero.
    #[must_use]
    pub fn limbs(&self) -> &[u64] {
        &self.magnitude
    }

    /// Compare only the two borrowed canonical magnitudes, without sign copies.
    #[must_use]
    pub fn cmp_abs(&self, other: &Self) -> Ordering {
        mag_cmp(&self.magnitude, &other.magnitude)
    }

    /// Heap bytes owned by this value, excluding its three inline limbs.
    #[must_use]
    pub fn allocated_bytes(&self) -> usize {
        self.magnitude.allocated_bytes()
    }

    /// The exact unsigned magnitude, when it fits in `u128`.
    #[must_use]
    pub fn unsigned_abs_u128(&self) -> Option<u128> {
        mag_to_u128(&self.magnitude)
    }

    /// Copy immutable limbs into an admitted reusable destination. Inline
    /// magnitudes remain inline and require no arena slot.
    /// # Errors
    /// Refuses scratch capacity or destination exhaustion without allocating.
    pub fn copy_in(&self, scratch: &LimbScratch) -> Result<Self, LimbScratchError> {
        if matches!(self.magnitude, Mag::Pooled(_)) {
            return Ok(self.clone());
        }
        Ok(Self::from_parts(
            self.negative,
            scratch.copy(&self.magnitude)?,
        ))
    }

    /// Detach an immutable value from reusable destinations into ordinary owned
    /// storage. Numerical preparations admit this copy before calling it.
    #[must_use]
    pub fn detached(&self) -> Self {
        Self::from_parts(self.negative, Mag::from_slice(&self.magnitude))
    }

    /// Retain canonical immutable limbs in shared owned storage. Inline values
    /// stay inline; subsequent clones share longer limbs without allocating.
    /// Preparing shared storage is an admitted caller operation, never an
    /// arithmetic fallback. Arithmetic always writes into fresh destinations.
    #[must_use]
    pub fn into_shared(mut self) -> Self {
        self.magnitude = match self.magnitude {
            Mag::Heap(words) => Mag::Shared(std::sync::Arc::new(words)),
            Mag::Pooled(words) => Mag::Shared(std::sync::Arc::new(words.words().to_vec())),
            magnitude => magnitude,
        };
        self
    }

    /// Exact heap retained after immutable sharing. This can be checked before
    /// preparing ownership headers or copying reusable destination limbs.
    #[must_use]
    pub fn shared_owned_bytes(&self) -> usize {
        match &self.magnitude {
            Mag::Inline { .. } => 0,
            Mag::Heap(words) => {
                words.capacity() * size_of::<u64>() + size_of::<Vec<u64>>() + 2 * size_of::<usize>()
            }
            Mag::Shared(_) => self.allocated_bytes(),
            Mag::Pooled(words) => {
                size_of_val(words.words()) + size_of::<Vec<u64>>() + 2 * size_of::<usize>()
            }
        }
    }

    /// Heap bytes the detached canonical magnitude will retain.
    #[must_use]
    pub fn detached_heap_bytes(&self) -> usize {
        if self.magnitude.len() <= 3 {
            0
        } else {
            self.magnitude.len().saturating_mul(size_of::<u64>())
        }
    }

    /// Checked native temporary/destination layout for decimal rendering.
    /// # Errors
    /// Refuses addressable byte overflow before any conversion allocation.
    pub fn decimal_render_layout(
        &self,
        scale: u32,
    ) -> Result<crate::exact::cost::NumericRenderLayout, LimbScratchError> {
        crate::exact::cost::numeric_render_layout(
            self.bit_len(),
            self.limb_len(),
            scale,
            self.negative,
        )
    }

    /// Convert through the existing division kernel into admitted decimal groups.
    /// No source magnitude clone or lexical String is constructed.
    /// # Errors
    /// Returns checked layout or physical native destination refusal.
    pub fn prepare_decimal_digits(&self) -> Result<PreparedDecimalDigits, LimbScratchError> {
        self.prepare_decimal_digits_using(&scratch::Fallible)
    }

    pub(crate) fn prepare_decimal_digits_using(
        &self,
        storage: &impl Allocate,
    ) -> Result<PreparedDecimalDigits, LimbScratchError> {
        let layout = self.decimal_render_layout(0)?;
        let mut groups = storage.destination(layout.group_capacity(), 0)?;
        if self.is_zero() {
            storage.push(&mut groups, 0)?;
        } else {
            let mut quotient: Option<Mag> = None;
            loop {
                let current: &[u64] = quotient.as_ref().map_or(&self.magnitude, |value| value);
                if current.is_empty() {
                    break;
                }
                let (next, remainder) = mag_divmod_small(current, CHUNK_BASE, storage)?;
                storage.push(&mut groups, remainder)?;
                quotient = Some(next);
            }
        }
        let leading = groups.last().copied().expect("zero includes one group");
        let leading_digits = if leading == 0 {
            1
        } else {
            usize::try_from(leading.ilog10()).map_err(|_| LimbScratchError::SizeOverflow)? + 1
        };
        let digits = groups
            .len()
            .checked_sub(1)
            .and_then(|n| n.checked_mul(DECIMAL_CHUNK))
            .and_then(|n| n.checked_add(leading_digits))
            .ok_or(LimbScratchError::SizeOverflow)?;
        digits
            .checked_add(usize::from(self.negative))
            .ok_or(LimbScratchError::SizeOverflow)?;
        Ok(PreparedDecimalDigits {
            negative: self.negative,
            groups,
            digits,
        })
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
    #[must_use]
    pub fn is_one(&self) -> bool {
        !self.negative && mag_is_one(&self.magnitude)
    }

    /// Returns `true` when the magnitude is odd.
    #[must_use]
    pub fn is_odd(&self) -> bool {
        self.magnitude.first().copied().unwrap_or(0) & 1 == 1
    }

    /// Additive inverse, preserving canonical zero.
    #[must_use]
    pub fn negated(&self) -> Self {
        self.neg()
    }

    /// Exact bit length, also a conservative numerical workspace bound.
    #[must_use]
    pub fn bits_upper_bound(&self) -> usize {
        usize::try_from(self.bit_len()).unwrap_or(usize::MAX)
    }

    /// Multiply by an exact power of two.
    #[must_use]
    pub fn mul_pow2(&self, exp: u32) -> Self {
        self.shl(exp)
    }

    /// Exact power-of-two product with bounded reusable storage.
    /// # Errors
    /// Refuses scratch limb/destination exhaustion without allocating.
    pub fn mul_pow2_in(&self, exp: u32, scratch: &LimbScratch) -> Result<Self, LimbScratchError> {
        self.shl_in(exp, scratch)
    }

    /// Multiply by an exact power of ten.
    #[must_use]
    pub fn mul_pow10(&self, exp: u32) -> Self {
        self.mul_pow10_using(exp, &Unbounded)
            .expect("unbounded integer storage")
    }

    pub(crate) fn mul_pow10_using(
        &self,
        exp: u32,
        storage: &impl Allocate,
    ) -> Result<Self, LimbScratchError> {
        self.mul_using(&Self::pow10_using(exp, storage)?, storage)
    }

    /// Exact power-of-ten product with bounded reusable storage.
    /// # Errors
    /// Refuses scratch limb/destination exhaustion without allocating.
    pub fn mul_pow10_in(&self, exp: u32, scratch: &LimbScratch) -> Result<Self, LimbScratchError> {
        self.mul_pow10_using(exp, scratch)
    }

    /// Exact multiplication by a machine factor.
    #[must_use]
    pub fn mul_small(&self, factor: u32) -> Self {
        self.mul(&Self::from_u64(u64::from(factor)))
    }

    /// Truncated quotient and unsigned remainder of a positive machine divisor.
    /// Zero divisor returns `None`.
    #[must_use]
    pub fn div_rem_u64(&self, divisor: u64) -> Option<(Self, u64)> {
        let (quotient, remainder) = self.div_rem(&Self::from_u64(divisor))?;
        Some((quotient, remainder.unsigned_abs_u128()? as u64))
    }
}

impl Ord for BigInt {
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

impl PartialOrd for BigInt {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for BigInt {
    /// Writes the canonical decimal form: a `-` only when negative, no leading
    /// zeros, and `0` for zero.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.prepare_decimal_digits()
            .map_err(|_| fmt::Error)?
            .write_fmt(f)
    }
}

/// Exact floor of `sqrt(numerator/denominator) * 2^fractional_bits`.
/// Returns `None` for a negative numerator, nonpositive denominator, or shift
/// overflow. Resource admission is the caller's responsibility, as for [`BigInt`].
#[must_use]
pub fn sqrt_ratio_floor(
    numerator: &BigInt,
    denominator: &BigInt,
    fractional_bits: u32,
) -> Option<BigInt> {
    sqrt_ratio_floor_using(numerator, denominator, fractional_bits, &Unbounded)
        .expect("unbounded integer storage")
}

/// The same floor-square-root equation using admitted reusable destinations.
///
/// # Errors
/// Refuses exhausted destination count or limb capacity without heap fallback.
/// Invalid signed inputs or shift overflow return `Ok(None)` before allocation.
pub fn sqrt_ratio_floor_in(
    numerator: &BigInt,
    denominator: &BigInt,
    fractional_bits: u32,
    scratch: &LimbScratch,
) -> Result<Option<BigInt>, LimbScratchError> {
    sqrt_ratio_floor_using(numerator, denominator, fractional_bits, scratch)
}

fn sqrt_ratio_floor_using(
    numerator: &BigInt,
    denominator: &BigInt,
    fractional_bits: u32,
    storage: &impl Allocate,
) -> Result<Option<BigInt>, LimbScratchError> {
    if numerator.is_negative() || denominator <= &BigInt::zero() {
        return Ok(None);
    }
    let Some(shift) = fractional_bits.checked_mul(2) else {
        return Ok(None);
    };
    let shifted = numerator.shl_using(shift, storage)?;
    let (quotient, _) = shifted
        .div_rem_using(denominator, storage)?
        .expect("validated positive denominator");
    quotient.sqrt_floor_using(storage)
}

#[cfg(test)]
mod tests {
    use super::BigInt;
    use purrdf_alloc_probe::CurrentThreadWindow;
    use purrdf_hash::mix;

    #[test]
    fn detaching_owned_coefficients_returns_their_reusable_destinations() {
        let expected = BigInt::one().shl(448).add(&BigInt::from_i64(19)).neg();
        let detached = {
            let scratch = super::LimbScratch::new(2, 16).unwrap();
            let source = expected.copy_in(&scratch).unwrap();
            assert_eq!(scratch.available(), 1);
            let bytes = source.detached_heap_bytes();
            let detached = source.detached();
            assert_eq!(detached.allocated_bytes(), bytes);
            assert_eq!(detached, expected);
            drop(source);
            assert_eq!(scratch.available(), scratch.destination_capacity());
            detached
        };
        assert_eq!(detached, expected);
        assert_eq!(BigInt::from_i64(-17).detached_heap_bytes(), 0);
    }

    #[test]
    fn scaled_rational_square_root_is_exact_at_boundaries() {
        for bits in [0, 1, 63, 96, 256] {
            let denominator = BigInt::from_i64(7);
            let numerator = BigInt::from_i64(2);
            let root = super::sqrt_ratio_floor(&numerator, &denominator, bits).unwrap();
            let scaled = numerator.shl(2 * bits);
            assert!(root.mul(&root).mul(&denominator) <= scaled);
            let next = root.add(&BigInt::one());
            assert!(next.mul(&next).mul(&denominator) > scaled);
        }
        assert_eq!(
            super::sqrt_ratio_floor(&BigInt::zero(), &BigInt::one(), 96),
            Some(BigInt::zero())
        );
        assert_eq!(
            super::sqrt_ratio_floor(&BigInt::from_i64(9), &BigInt::from_i64(4), 1),
            Some(BigInt::from_i64(3))
        );
        assert!(super::sqrt_ratio_floor(&BigInt::from_i64(-1), &BigInt::one(), 96).is_none());
        assert!(super::sqrt_ratio_floor(&BigInt::one(), &BigInt::zero(), 96).is_none());
        assert!(super::sqrt_ratio_floor(&BigInt::one(), &BigInt::from_i64(-1), 96).is_none());
        assert!(super::sqrt_ratio_floor(&BigInt::one(), &BigInt::one(), u32::MAX).is_none());
    }

    #[test]
    fn admitted_ratio_roots_match_and_refuse_without_heap_fallback() {
        let scratch = super::LimbScratch::new(32, 32).unwrap();
        for bits in [0, 1, 63, 96, 256] {
            for (numerator, denominator) in [(0, 1), (9, 4), (2, 7), (17, 3)] {
                let numerator = BigInt::from_i64(numerator);
                let denominator = BigInt::from_i64(denominator);
                let expected = super::sqrt_ratio_floor(&numerator, &denominator, bits);
                let window = CurrentThreadWindow::open();
                let actual =
                    super::sqrt_ratio_floor_in(&numerator, &denominator, bits, &scratch).unwrap();
                assert_eq!(window.close().allocations, 0);
                assert_eq!(actual, expected);
            }
        }
        let small = super::LimbScratch::new(1, 3).unwrap();
        let window = CurrentThreadWindow::open();
        assert!(
            super::sqrt_ratio_floor_in(&BigInt::one(), &BigInt::from_i64(7), 256, &small).is_err()
        );
        assert_eq!(window.close().allocations, 0);
        for (numerator, denominator, bits) in
            [(-1, 1, 96), (1, 0, 96), (1, -1, 96), (1, 1, u32::MAX)]
        {
            let window = CurrentThreadWindow::open();
            assert_eq!(
                super::sqrt_ratio_floor_in(
                    &BigInt::from_i64(numerator),
                    &BigInt::from_i64(denominator),
                    bits,
                    &small
                ),
                Ok(None)
            );
            assert_eq!(window.close().allocations, 0);
        }
    }

    #[test]
    fn inline_magnitude_survives_carry_borrow_and_spill() {
        let inline = BigInt::one().shl(191).sub(&BigInt::one());
        assert_eq!(inline.allocated_bytes(), 0);
        let spilled = inline.shl(2);
        assert_eq!(spilled.limbs().len(), 4);
        assert!(spilled.allocated_bytes() >= 32);
        let restored = spilled.shr(2);
        assert_eq!(restored, inline);
        assert_eq!(restored.allocated_bytes(), 0);
        let window = CurrentThreadWindow::open();
        let result = inline.add(&BigInt::one()).sub(&BigInt::one());
        let counters = window.close();
        assert_eq!(result, inline);
        assert_eq!(counters.allocations, 0);
    }

    #[test]
    fn binary_grid_division_matches_reconstruction_across_limb_edges() {
        let mut seed = 0x28f6_8194_52cb_970d;
        for _ in 0..512 {
            let low = mix::splitmix64_next(&mut seed);
            let high = mix::splitmix64_next(&mut seed);
            let source = BigInt::from_u128((u128::from(high) << 64) | u128::from(low))
                .shl(129)
                .add(&BigInt::from_u64(low));
            let shift = (mix::splitmix64_next(&mut seed) % 260) as u32;
            let divisor = BigInt::one().shl(shift);
            let (q, r) = source.div_rem(&divisor).unwrap();
            assert_eq!(q.mul(&divisor).add(&r), source);
            assert!(r >= BigInt::zero() && r < divisor);
            assert_eq!(q, source.shr(shift));
        }
    }

    #[test]
    fn normalized_division_reconstructs_all_limb_normalizations() {
        let mut seed = 0x385a_9e14_2db7_8cf0;
        for length in 2..=32 {
            for normalization in 0..64 {
                let mut denominator_words = super::Mag::zeroed(length);
                for limb in &mut denominator_words {
                    *limb = mix::splitmix64_next(&mut seed);
                }
                denominator_words[length - 1] = (mix::splitmix64_next(&mut seed) >> normalization)
                    | (1 << (63 - normalization));
                let denominator = BigInt {
                    negative: false,
                    magnitude: denominator_words,
                };
                let wanted_quotient = BigInt::from_u128(
                    (u128::from(mix::splitmix64_next(&mut seed)) << 64)
                        | u128::from(mix::splitmix64_next(&mut seed)),
                )
                .shl((length as u32 % 5) * 64);
                let wanted_remainder = denominator.sub(&BigInt::one());
                let numerator = wanted_quotient.mul(&denominator).add(&wanted_remainder);
                let (quotient, remainder) = numerator.div_rem(&denominator).unwrap();
                assert_eq!(quotient, wanted_quotient);
                assert_eq!(remainder, wanted_remainder);
                assert_eq!(quotient.mul(&denominator).add(&remainder), numerator);
                assert!(remainder >= BigInt::zero() && remainder < denominator);
            }
        }
    }

    #[test]
    fn normalized_division_corrects_estimate_and_addback_edges() {
        for words in [
            [1, 0, 1 << 63],
            [u64::MAX, 0, 1 << 63],
            [u64::MAX, u64::MAX, 1 << 63],
            [u64::MAX, u64::MAX, u64::MAX],
            [0x1234, u64::MAX, 1],
        ] {
            let divisor = BigInt {
                negative: false,
                magnitude: super::Mag::from_slice(&words),
            };
            for quotient in [
                BigInt::one(),
                BigInt::from_i64(2),
                BigInt::from_u64(u64::MAX),
                BigInt::one().shl(192).sub(&BigInt::one()),
            ] {
                let remainder = divisor.sub(&BigInt::one());
                let numerator = quotient.mul(&divisor).add(&remainder);
                for negative_numerator in [false, true] {
                    for negative_divisor in [false, true] {
                        let numerator = if negative_numerator {
                            numerator.neg()
                        } else {
                            numerator.clone()
                        };
                        let signed_divisor = if negative_divisor {
                            divisor.neg()
                        } else {
                            divisor.clone()
                        };
                        let (q, r) = numerator.div_rem(&signed_divisor).unwrap();
                        assert_eq!(q.mul(&signed_divisor).add(&r), numerator);
                        assert_eq!(q.abs(), quotient);
                        assert_eq!(r.abs(), remainder);
                    }
                }
            }
            // With n=3 and zero middle divisor limb, the estimate 2 for
            // 2*v-1 passes the second-limb test and needs the full add-back.
            let numerator = divisor.mul(&BigInt::from_i64(2)).sub(&BigInt::one());
            assert_eq!(
                numerator.div_rem(&divisor).unwrap(),
                (BigInt::one(), divisor.sub(&BigInt::one()))
            );
        }
    }

    #[test]
    fn inplace_binary_gcd_preserves_exact_common_factors() {
        let mut seed = 0x9cd6_418a_30f5_e272;
        for length in 3..=24 {
            let mut words = super::Mag::zeroed(length);
            for limb in &mut words {
                *limb = mix::splitmix64_next(&mut seed);
            }
            words[length - 1] |= 1;
            let factor = BigInt {
                negative: false,
                magnitude: words,
            };
            let x = factor.mul(&BigInt::from_i64(17)).shl(65);
            let y = factor.mul(&BigInt::from_i64(19)).shl(127);
            let expected = factor.shl(65);
            assert_eq!(x.gcd(&y), expected);
            assert_eq!(y.gcd(&x.neg()), expected);
            assert!(x.div_rem(&expected).unwrap().1.is_zero());
            assert!(y.div_rem(&expected).unwrap().1.is_zero());
        }
    }

    #[test]
    fn common_normalized_division_and_gcd_keep_inline_storage() {
        let numerator = BigInt::one().shl(191).add(&BigInt::from_u128(
            0x81eb_e24d_e521_0389_9bc0_726e_1554_02af,
        ));
        let denominator = BigInt::one()
            .shl(127)
            .add(&BigInt::from_u64(0xc61e_53a4_5e87_19cb));
        let window = CurrentThreadWindow::open();
        let (quotient, remainder) = numerator.div_rem(&denominator).unwrap();
        let gcd = numerator.gcd(&denominator);
        let counters = window.close();
        assert_eq!(counters.allocations, 0);
        assert_eq!(quotient.mul(&denominator).add(&remainder), numerator);
        assert!(numerator.div_rem(&gcd).unwrap().1.is_zero());
        assert!(denominator.div_rem(&gcd).unwrap().1.is_zero());
    }

    #[test]
    fn decimal_byte_parser_validates_without_a_string_intermediate() {
        assert_eq!(
            BigInt::from_decimal_bytes(
                b"000170141183460469231731687303715884105728"
                    .iter()
                    .copied()
            )
            .unwrap(),
            BigInt::one().shl(127)
        );
        assert!(BigInt::from_decimal_bytes(core::iter::empty()).is_none());
        assert!(BigInt::from_decimal_bytes(b"12x".iter().copied()).is_none());
        assert!(BigInt::from_decimal_bytes(b"-1".iter().copied()).is_none());
    }
}
