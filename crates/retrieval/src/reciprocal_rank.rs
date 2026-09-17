// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Reciprocal-rank decay in exact fixed point.
//!
//! A stratum's contribution to a candidate at 1-based rank `r` is
//!
//! ```text
//! weight * recip(K + r)
//! ```
//!
//! where `recip` is the reciprocal evaluated once at the declared scale
//! ([`SCALE_DIGITS`](purrdf_text::SCALE_DIGITS) fractional digits) truncating
//! toward zero, and `K` is the profile's smoothing constant. There is no
//! transcendental: a reciprocal is one exactly-rounded division, so it needs
//! none of the target-dependent care a logarithm does, and the emitted order is
//! byte-identical wherever it is computed.

use purrdf_text::{Fixed, SCALE_DIGITS};

use crate::error::FusionError;

/// `10^SCALE_DIGITS` — the scale of a [`Fixed`]'s raw integer.
const SCALE_RAW: i128 = 10_i128.pow(SCALE_DIGITS);

/// The contribution of a stratum weight at a 1-based rank under smoothing `K`.
///
/// The reciprocal is formed as the single integer division `10^scale / (K + r)`,
/// truncated toward zero, and then multiplied by `weight` through `Fixed`'s
/// wide-intermediate checked multiply, so an intermediate product that exceeds
/// `i128` is handled without either wrapping or a spurious overflow.
///
/// # Errors
///
/// * [`FusionError::InvalidK`] when `k == 0`.
/// * [`FusionError::InvalidRank`] when `rank == 0`.
/// * [`FusionError::Overflow`] when the product leaves the fixed-point range.
pub fn contribution(weight: Fixed, rank: u64, k: u32) -> Result<Fixed, FusionError> {
    if k == 0 {
        return Err(FusionError::InvalidK { k });
    }
    if rank == 0 {
        return Err(FusionError::InvalidRank { rank });
    }

    // K + rank as an unsigned value wide enough that neither operand can wrap.
    let denominator = u128::from(k) + u128::from(rank);
    // recip = 1 / (K + r), at the declared scale. A rank beyond the scale
    // truncates to zero, which is the exact value of every representable
    // reciprocal below 10^-12.
    let reciprocal_raw = u128::try_from(SCALE_RAW)
        .map_err(|_| FusionError::Overflow)?
        .checked_div(denominator)
        .ok_or(FusionError::Overflow)?;
    let reciprocal_raw = i128::try_from(reciprocal_raw).map_err(|_| FusionError::Overflow)?;

    weight
        .checked_mul(Fixed::from_raw(reciprocal_raw))
        .map_err(|_| FusionError::Overflow)
}
