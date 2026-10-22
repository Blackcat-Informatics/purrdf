// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The binary floating-point edge of the exact tower: the exact value of a finite
//! `f64`/`f32`, and the correctly rounded `f64`/`f32` nearest an exact ratio.
//!
//! # One rounding
//!
//! [`round_ratio`] delegates the workspace's single integer-only IEEE ratio
//! rounding body, including its exact subnormal grid and halfway decisions.
//! Finite input decomposition delegates the same normalized dyadic decoder the
//! certified numerical and geometric paths use. No platform float arithmetic or
//! second rounding algorithm executes at this boundary.

use crate::bigint::BigInt;

/// The parameters of an IEEE 754 binary interchange format that rounding needs.
pub(crate) struct Format {
    /// Significand precision in bits, the hidden bit included (53, 24).
    precision: u32,
}

/// IEEE 754 binary64.
pub(crate) const BINARY64: Format = Format { precision: 53 };

/// IEEE 754 binary32.
pub(crate) const BINARY32: Format = Format { precision: 24 };

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

/// Borrow source limbs and invoke the one IEEE rounding body with checked
/// native destination admission. A caller's original refusal is unchanged.
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
    let sign = if format.precision == 53 { 63 } else { 31 };
    let format = if format.precision == 53 {
        crate::ieee::ratio::Format::BINARY64
    } else {
        crate::ieee::ratio::Format::BINARY32
    };
    let bits = crate::ieee::ratio::ratio_bits_admitted(
        n,
        d,
        crate::ieee::ratio::Rounding::NearestEven,
        format,
        storage,
        admit,
    )?
    .expect("positive exact denominator");
    if n.is_zero() {
        Ok(0)
    } else {
        Ok((bits & !(1_u64 << sign)) | (u64::from(negative) << sign))
    }
}

/// The exact value of a finite `f64` as `(negative, significand, exponent)`
/// with `|value| = significand × 2^exponent`; `None` for `NaN` and infinities.
pub(crate) fn decompose_f64(value: f64) -> Option<(bool, u64, i32)> {
    let value = crate::ieee::dyadic::Binary64Dyadic::decode(value)?;
    Some((value.negative(), value.significand(), value.exponent()))
}

/// [`decompose_f64`] for an `f32` (which widens to `f64` exactly).
pub(crate) fn decompose_f32(value: f32) -> Option<(bool, u64, i32)> {
    decompose_f64(f64::from(value))
}
