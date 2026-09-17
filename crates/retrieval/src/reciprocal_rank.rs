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

/// The largest depth this crate will ever be asked about.
///
/// A plan records a per-stratum depth as a `u32`, so a monotone range at or
/// above that value cannot be exceeded by any depth a plan can carry and there
/// is nothing further to compute. It is a saturation point, not a policy.
const MAX_DEPTH: u64 = u32::MAX as u64;

/// The largest 1-based depth at which `weight`'s contributions are still
/// **strictly** decreasing with rank under smoothing `k`.
///
/// That is: the largest `r` such that `contribution(w, i, k) >
/// contribution(w, i + 1, k)` holds for every `i` in `1..r`. A depth of one is
/// always in range — a single rank has no adjacent pair to collide with — and
/// the result saturates at [`MAX_DEPTH`].
///
/// # Why it is searched rather than solved
///
/// The exact condition is `trunc(w · trunc(S / D) / S) == trunc(w · trunc(S /
/// (D + 1)) / S)` with `D = k + r` and `S = 10^SCALE_DIGITS`, and the two outer
/// truncations do not commute with the inner one: two contributions can differ
/// even when `w · (a − b) < S`, because the floors can straddle an integer. A
/// closed form would therefore have to be conservative, and a conservative bound
/// here refuses profiles whose ranks in fact order perfectly — the over-refusal
/// this repository treats as exactly as severe as a wrong answer. So the first
/// collision is found exactly.
///
/// It is not found by walking from rank one. `w · trunc(S / (D · (D + 1))) ≥ S`
/// is a sound *guarantee* of distinctness (proved below), it is monotone in `D`,
/// and a binary search over it lands the scan at the last rank that guarantee
/// covers; the exact walk runs from there. The guarantee's boundary sits at
/// `D ≈ sqrt(min(w, 1) · S)` and the first real collision a little above it, so
/// the walk is a few hundred steps whatever the weight — and never the million
/// a walk from rank one would be.
///
/// The guarantee: with `a = trunc(S / D)`, `b = trunc(S / (D + 1))` and
/// `q = trunc(S / (D · (D + 1)))`, we have `a > S/D − 1` and `b ≤ S/(D + 1)`, so
/// `a − b > S/(D · (D + 1)) − 1 ≥ q − 1`; both sides are integers, so
/// `a − b ≥ q`. Then `w·a/S − w·b/S ≥ w·q/S ≥ 1`, and `trunc(x) > trunc(y)`
/// whenever `x ≥ y + 1`.
pub(crate) fn monotone_depth(weight: Fixed, k: u32) -> u64 {
    // Both are fixed by a validated profile; neither has a meaningful answer,
    // and a single rank is the smallest range that is trivially ordered.
    if k == 0 || weight <= Fixed::ZERO {
        return 1;
    }
    let guaranteed = guaranteed_monotone_depth(weight.into_raw().unsigned_abs(), k);
    if guaranteed >= MAX_DEPTH {
        return MAX_DEPTH;
    }
    // Start on the last rank the guarantee covers rather than just past it: the
    // repeated pair costs one division and the walk can then only ever find a
    // collision the guarantee wrongly excluded, never miss one.
    let mut rank = guaranteed.max(1);
    let Ok(mut current) = contribution(weight, rank, k) else {
        return rank;
    };
    while rank < MAX_DEPTH {
        let Ok(next) = contribution(weight, rank + 1, k) else {
            return rank;
        };
        if next == current {
            return rank;
        }
        current = next;
        rank += 1;
    }
    MAX_DEPTH
}

/// The largest rank up to which the closed-form guarantee on
/// [`monotone_depth`] holds, or zero when it holds for no rank at all.
///
/// The predicate is non-increasing in the rank (`q` is), so a binary search over
/// it is exact.
fn guaranteed_monotone_depth(weight_raw: u128, k: u32) -> u64 {
    let scale = SCALE_RAW.unsigned_abs();
    let guarantees = |rank: u64| -> bool {
        let denominator = u128::from(k) + u128::from(rank);
        let Some(product) = denominator.checked_mul(denominator + 1) else {
            return false;
        };
        let quotient = scale / product;
        // An intermediate too large for `u128` is far above the threshold; it is
        // a guarantee held, not a guarantee lost.
        weight_raw
            .checked_mul(quotient)
            .is_none_or(|bound| bound >= scale)
    };
    let mut low = 0_u64;
    let mut high = MAX_DEPTH;
    while low < high {
        let mid = low + (high - low).div_ceil(2);
        if guarantees(mid) {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    low
}

#[cfg(test)]
mod tests {
    use super::{MAX_DEPTH, contribution, monotone_depth};
    use purrdf_text::{Fixed, SCALE_DIGITS};

    /// `10^SCALE_DIGITS`, the raw value of a weight of exactly one.
    const SCALE: i128 = 10_i128.pow(SCALE_DIGITS);

    fn value(weight: Fixed, rank: u64, k: u32) -> Fixed {
        contribution(weight, rank, k).expect("a positive weight and a 1-based rank do not overflow")
    }

    /// The property the bound is defined by, asserted on both sides of it: every
    /// adjacent pair at or below the bound is strictly ordered, and the pair
    /// immediately beyond it is not.
    fn assert_bound_is_exact(weight: Fixed, k: u32) {
        let bound = monotone_depth(weight, k);
        assert!(bound >= 1, "a single rank is always in range");
        assert!(
            bound < MAX_DEPTH,
            "this fixture must reach a real collision"
        );
        for rank in 1..bound {
            assert!(
                value(weight, rank, k) > value(weight, rank + 1, k),
                "ranks {rank} and {} collide at or below the bound {bound}",
                rank + 1
            );
        }
        assert_eq!(
            value(weight, bound, k),
            value(weight, bound + 1, k),
            "ranks {bound} and {} must be the first adjacent pair to collide",
            bound + 1
        );
    }

    /// The fixture the coupling is documented by: two adjacent ranks produce
    /// distinct contributions at and below the bound, and the same contribution
    /// immediately beyond it.
    #[test]
    fn adjacent_ranks_stay_distinct_to_the_bound_and_collide_beyond_it() {
        // A weight of 10^-6, so the bound is in the low thousands and the whole
        // walk is legible.
        let weight = Fixed::from_raw(1_000_000);
        assert_bound_is_exact(weight, 1);
        assert_bound_is_exact(weight, 60);
    }

    /// The same assertion at a weight of exactly one, where the reciprocal's own
    /// resolution is what binds.
    #[test]
    fn the_bound_is_exact_at_unit_weight() {
        assert_bound_is_exact(Fixed::ONE, 60);
    }

    /// The correction to the reviewed model. `r ≲ sqrt(w·S) − K` predicts that a
    /// weight of 10^6 buys a bound near 10^9; it does not, because the
    /// reciprocal is truncated to the scale *before* the weight is applied, so
    /// no weight at or above one can recover a distinction the reciprocal has
    /// already lost. Every weight at or above one therefore shares one bound.
    #[test]
    fn a_weight_above_one_buys_no_depth() {
        let unit = monotone_depth(Fixed::ONE, 60);
        for raw in [SCALE, SCALE * 2, SCALE * 1_000, SCALE * 1_000_000] {
            assert_eq!(
                monotone_depth(Fixed::from_raw(raw), 60),
                unit,
                "weight raw {raw} must share the unit weight's bound"
            );
        }
        // And that shared bound is set by sqrt(S), not by the weight: it is just
        // above 10^6 minus the smoothing constant.
        let sqrt_scale = 1_000_000_u64;
        assert!(
            unit + 60 >= sqrt_scale && unit + 60 < sqrt_scale * 2,
            "the shared bound {unit} must sit at the reciprocal's own resolution"
        );
    }

    /// Below one the weight does bind, and lowering it lowers the bound — the
    /// half of the reviewed model that is right.
    #[test]
    fn a_weight_below_one_lowers_the_bound() {
        let mut previous = monotone_depth(Fixed::ONE, 1);
        for raw in [SCALE / 100, SCALE / 10_000, SCALE / 1_000_000, 100, 1] {
            let bound = monotone_depth(Fixed::from_raw(raw), 1);
            assert!(
                bound < previous,
                "weight raw {raw} must lower the bound below {previous}, got {bound}"
            );
            previous = bound;
        }
    }

    /// A weight so small that its every contribution truncates to zero orders
    /// nothing at all, and the bound says so with the smallest range there is
    /// rather than with a number that would license a depth.
    #[test]
    fn a_weight_that_truncates_to_nothing_has_a_bound_of_one() {
        let weight = Fixed::from_raw(1);
        assert_eq!(monotone_depth(weight, 60), 1);
        assert_eq!(value(weight, 1, 60), value(weight, 2, 60));
    }

    /// The smoothing constant is the third quantity in the coupling, and it
    /// shifts the bound by exactly what it adds to the denominator.
    #[test]
    fn the_smoothing_constant_shifts_the_bound_one_for_one() {
        let weight = Fixed::from_raw(1_000_000);
        let base = monotone_depth(weight, 1);
        for k in [2_u32, 10, 60, 500] {
            assert_eq!(
                monotone_depth(weight, k) + u64::from(k),
                base + 1,
                "the bound plus K is the first colliding denominator, whatever K is"
            );
        }
    }

    /// The guarantee the search jumps on must never claim a distinctness the
    /// exact walk does not find, so the accelerated answer is compared against
    /// the unaccelerated one over a range small enough to walk from rank one.
    #[test]
    fn the_accelerated_search_agrees_with_a_walk_from_rank_one() {
        for raw in [1_i128, 7, 1_000, 999_983, 1_000_000, 12_345_678] {
            let weight = Fixed::from_raw(raw);
            for k in [1_u32, 3, 60] {
                let expected = (1..MAX_DEPTH)
                    .find(|rank| value(weight, *rank, k) == value(weight, rank + 1, k))
                    .expect("every weight collides at some rank");
                assert_eq!(
                    monotone_depth(weight, k),
                    expected,
                    "weight raw {raw} at K {k}"
                );
            }
        }
    }
}
