// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The binary floating-point edge of the exact tower: the exact value of a finite
//! `f64`/`f32`, and the correctly rounded `f64`/`f32` nearest an exact ratio.
//!
//! # One rounding
//!
//! [`round_ratio`] forms an integer quotient `q = ⌊n · 2^k / d⌋` of between 116
//! and 126 bits plus a sticky bit for a nonzero remainder, then rounds `q` once,
//! with integer operations only, at the bit position the target format's grid
//! has at the value's binade — the subnormal grid included — and assembles the
//! result from its bit pattern. No floating-point operation runs, so the answer
//! is the same on every target, the x87 included, and it is never rounded twice:
//! the 63-plus guard bits and the sticky bit decide every tie exactly.

use crate::bigint::BigInt;

/// The parameters of an IEEE 754 binary interchange format that rounding needs.
pub(crate) struct Format {
    /// Significand precision in bits, the hidden bit included (53, 24).
    precision: u32,
    /// The smallest normal exponent (-1022, -126).
    min_exponent: i64,
    /// The largest finite exponent (1023, 127).
    max_exponent: i64,
}

/// IEEE 754 binary64.
pub(crate) const BINARY64: Format = Format {
    precision: 53,
    min_exponent: -1022,
    max_exponent: 1023,
};

/// IEEE 754 binary32.
pub(crate) const BINARY32: Format = Format {
    precision: 24,
    min_exponent: -126,
    max_exponent: 127,
};

/// `log2(10) × 10^15`, truncated: the slope of the binade estimate.
const LOG2_10_E15: i128 = 3_321_928_094_887_362;

/// `⌊x · log2(10)⌋` within one unit, from integer arithmetic.
pub(crate) fn log2_of_pow10(x: i64) -> i64 {
    let scaled = i128::from(x) * LOG2_10_E15;
    // Within `|x| × 10^-15` of the true product, far inside the slack the
    // callers add; the result fits i64 for every digit count a `u64` holds.
    i64::try_from(scaled.div_euclid(1_000_000_000_000_000)).unwrap_or(if x < 0 {
        i64::MIN / 2
    } else {
        i64::MAX / 2
    })
}

/// Round `±n / d` (`n`, `d` non-negative, `d` nonzero) to `format`, to nearest
/// with ties to even, returning the bit pattern (the low 32 bits for binary32).
/// A zero `n` is `+0`; a nonzero value that rounds to zero keeps its sign; a value
/// at or beyond the overflow threshold is the signed infinity.
pub(crate) fn round_ratio(negative: bool, n: &BigInt, d: &BigInt, format: &Format) -> u64 {
    round_ratio_admitted_using(
        negative,
        n,
        d,
        format,
        &crate::bigint::scratch::Unbounded,
        &mut |_| Ok(()),
    )
    .expect("unbounded integer storage")
}

/// The same one-rounding body borrowing source limbs and allocating through
/// the selected native destination provider. Admission covers fresh live
/// destinations; source ownership is retained by the calling frame.
pub(crate) fn round_ratio_admitted_using(
    negative: bool,
    n: &BigInt,
    d: &BigInt,
    format: &Format,
    storage: &impl crate::bigint::scratch::Allocate,
    admit: &mut impl FnMut(
        super::cost::NumericOperationLayout,
    ) -> Result<(), crate::bigint::LimbScratchError>,
) -> Result<u64, crate::bigint::LimbScratchError> {
    let precision = format.precision;
    let fraction_bits = precision - 1;
    let exponent_bits: u32 = if precision == 53 { 11 } else { 8 };
    let sign = u64::from(negative) << (fraction_bits + exponent_bits);
    if n.is_zero() {
        return Ok(0);
    }
    let infinity = sign | (((1_u64 << exponent_bits) - 1) << fraction_bits);
    // n ∈ [2^(bn−1), 2^bn) and d ∈ [2^(bd−1), 2^bd), so
    // log2(n/d) ∈ (bn − bd − 1, bn − bd + 1). Reading binary
    // lengths needs no allocation or decimal conversion.
    let delta = i128::from(n.bit_len()) - i128::from(d.bit_len());
    let clamp = |value: i128| {
        i64::try_from(value).unwrap_or(if value < 0 {
            i64::MIN / 4
        } else {
            i64::MAX / 4
        })
    };
    let low = clamp(delta - 1);
    let high = clamp(delta + 1);
    // value ≥ 2^low: beyond the largest binade, it overflows.
    if low > format.max_exponent + 1 {
        return Ok(infinity);
    }
    // value < 2^high ≤ half the smallest subnormal: it rounds to a signed zero.
    let min_quantum = format.min_exponent - i64::from(fraction_bits);
    if high < min_quantum {
        return Ok(sign);
    }
    // q = ⌊value · 2^k⌋ < 2^126, and ≥ 2^(126 − (high − low)) ≥ 2^124.
    let k = 126 - high;
    let shift = u32::try_from(k.unsigned_abs())
        .map_err(|_| crate::bigint::LimbScratchError::SizeOverflow)?;
    let selected = if k >= 0 { n } else { d };
    admit(super::cost::binary_shift_layout(
        selected.bit_len(),
        shift,
        0,
    )?)?;
    let shifted = selected.shl_using(shift, storage)?;
    let (numerator, denominator) = if k >= 0 { (&shifted, d) } else { (n, &shifted) };
    admit(
        super::cost::integer_division_layout(
            numerator.limb_len() as u64,
            denominator.limb_len() as u64,
        )?
        .with_live(shifted.allocated_bytes())?,
    )?;
    let (quotient, remainder) = numerator
        .div_rem_using(denominator, storage)?
        .expect("the denominator is nonzero");
    // Only magnitudes enter this algorithm. Taking an owned positive sign
    // permits negative borrowed sources without an absolute-magnitude copy.
    let quotient = quotient.with_sign(false);
    let sticky = !remainder.is_zero();
    let q = u128::try_from(quotient.to_i128().expect("q < 2^126 fits i128"))
        .expect("q is non-negative");
    // value ∈ [2^e, 2^(e+1)) with e the leading bit of q, less k.
    let leading = i64::from(q.ilog2());
    let binade = leading - k;
    let quantum = binade.max(format.min_exponent) - i64::from(fraction_bits);
    // q counts units of 2^−k; the grid's unit is 2^quantum.
    let drop = quantum + k;
    debug_assert!(drop > 0, "at least 60 guard bits sit below the grid");
    let (kept, inexact_half) = if drop >= 128 {
        // q < 2^126 < half a grid unit.
        (0_u128, std::cmp::Ordering::Less)
    } else {
        let drop = u32::try_from(drop).expect("a shift below 128");
        let kept = q >> drop;
        let rest = q & ((1_u128 << drop) - 1);
        let half = 1_u128 << (drop - 1);
        let mut order = rest.cmp(&half);
        if order == std::cmp::Ordering::Equal && sticky {
            order = std::cmp::Ordering::Greater;
        }
        (kept, order)
    };
    let up = match inexact_half {
        std::cmp::Ordering::Greater => true,
        std::cmp::Ordering::Equal => kept & 1 == 1,
        std::cmp::Ordering::Less => false,
    };
    let mut significand = u64::try_from(kept + u128::from(up)).expect("precision + 1 bits");
    let mut exponent = quantum;
    if significand == 1_u64 << precision {
        significand >>= 1;
        exponent += 1;
    }
    if significand == 0 {
        return Ok(sign);
    }
    if exponent + i64::from(fraction_bits) > format.max_exponent {
        return Ok(infinity);
    }
    let bits = if significand < 1_u64 << fraction_bits {
        // Subnormal: the exponent field is zero and the quantum is the minimum.
        significand
    } else {
        let bias = (1_i64 << (exponent_bits - 1)) - 1;
        let biased = u64::try_from(exponent + i64::from(fraction_bits) + bias)
            .expect("a normal exponent is positive");
        (biased << fraction_bits) | (significand & ((1_u64 << fraction_bits) - 1))
    };
    Ok(sign | bits)
}

/// The exact value of a finite `f64` as `(negative, significand, exponent)`
/// with `|value| = significand × 2^exponent`; `None` for `NaN` and infinities.
pub(crate) fn decompose_f64(value: f64) -> Option<(bool, u64, i32)> {
    if !value.is_finite() {
        return None;
    }
    let bits = value.to_bits();
    let biased = i32::try_from((bits >> 52) & 0x7ff).expect("an 11-bit field");
    let fraction = bits & ((1_u64 << 52) - 1);
    let (significand, exponent) = if biased == 0 {
        (fraction, -1074)
    } else {
        (fraction | (1_u64 << 52), biased - 1075)
    };
    Some((bits >> 63 == 1, significand, exponent))
}

/// [`decompose_f64`] for an `f32` (which widens to `f64` exactly).
pub(crate) fn decompose_f32(value: f32) -> Option<(bool, u64, i32)> {
    decompose_f64(f64::from(value))
}
