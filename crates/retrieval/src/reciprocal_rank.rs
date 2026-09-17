// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Reciprocal-rank decay in exact fixed point.
//!
//! A stratum's contribution to a candidate at 1-based rank `r` is the value of
//!
//! ```text
//! weight / (K + r)
//! ```
//!
//! at the declared scale ([`SCALE_DIGITS`](purrdf_text::SCALE_DIGITS) fractional
//! digits), truncating toward zero, where `K` is the profile's smoothing
//! constant. There is no transcendental: the quantity is one division, so it
//! needs none of the target-dependent care a logarithm does, and the emitted
//! order is byte-identical wherever it is computed.
//!
//! # Two rules compute that quantity, and they are not the same numbers
//!
//! [`DecayRule::ReciprocalRank`] evaluates the *reciprocal* first and applies
//! the weight afterwards: `trunc(w_raw · trunc(S / D) / S)` with `D = K + r` and
//! `S = 10^SCALE_DIGITS`. Two truncations, and the inner one is a ceiling the
//! weight cannot lift — `trunc(S / D)` stops strictly decreasing once
//! `D² > S`, so every weight at or above one shares one monotone range ending
//! just past `D = 10^6`.
//!
//! [`DecayRule::WeightedReciprocalRank`] folds the weight into the numerator
//! instead. A weight's raw integer is already `w · S`, so `w / D` at the
//! declared scale is exactly `trunc(w_raw / D)`: **one** exactly-rounded
//! division rather than a division followed by a truncated multiply. It is the
//! same function computed more accurately — its value is never below the other
//! rule's and never more than `⌊w⌋ + 1` raw units above it — and because the weight
//! now sits in the numerator, adjacent values differ by `w_raw / (D · (D + 1))`
//! and stay distinct while that is at least one, which is `D ≲ sqrt(w · S) =
//! 10^6 · sqrt(w)`. Depth is bought with weight.
//!
//! Neither rule is a default and neither replaces the other: a profile names the
//! one it runs under and carries that choice in its identity.

use purrdf_text::{Fixed, SCALE_DIGITS};

use crate::error::FusionError;
use crate::fusion_profile::DecayRule;

/// `10^SCALE_DIGITS` — the scale of a [`Fixed`]'s raw integer.
const SCALE_RAW: i128 = 10_i128.pow(SCALE_DIGITS);

/// `K + rank` as a `u128`, or the typed refusal for an unusable operand.
///
/// Widened before it is summed so neither operand can wrap, and returned as a
/// `u128` because `K` and the rank are both full-range 32- and 64-bit values.
fn denominator(rank: u64, k: u32) -> Result<u128, FusionError> {
    if k == 0 {
        return Err(FusionError::InvalidK { k });
    }
    if rank == 0 {
        return Err(FusionError::InvalidRank { rank });
    }
    Ok(u128::from(k) + u128::from(rank))
}

/// The contribution of a stratum weight at a 1-based rank under smoothing `K`,
/// under [`DecayRule::ReciprocalRank`].
///
/// The reciprocal is formed as the single integer division `10^scale / (K + r)`,
/// truncated toward zero, and then multiplied by `weight` through `Fixed`'s
/// wide-intermediate checked multiply, so an intermediate product that exceeds
/// `i128` is handled without either wrapping or a spurious overflow.
///
/// These numbers are frozen: they are the ones every profile issued under this
/// rule has already produced. A caller that wants the weight folded into the
/// numerator selects [`DecayRule::WeightedReciprocalRank`] and gets
/// [`weighted_contribution`].
///
/// # Errors
///
/// * [`FusionError::InvalidK`] when `k == 0`.
/// * [`FusionError::InvalidRank`] when `rank == 0`.
/// * [`FusionError::Overflow`] when the product leaves the fixed-point range.
pub fn contribution(weight: Fixed, rank: u64, k: u32) -> Result<Fixed, FusionError> {
    let denominator = denominator(rank, k)?;
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

/// The contribution of a stratum weight at a 1-based rank under smoothing `K`,
/// under [`DecayRule::WeightedReciprocalRank`].
///
/// A [`Fixed`]'s raw integer is the value times `10^SCALE_DIGITS`, so the value
/// `w / (K + r)` at that same scale is exactly `trunc(w_raw / (K + r))`: one
/// division, truncated toward zero, and nothing else. That is the correctly
/// rounded answer by construction, which is what [`contribution`] cannot be —
/// it truncates the reciprocal before the weight reaches it.
///
/// The sign is carried separately so the truncation is toward zero for a
/// negative weight as well, matching every other rounding in the layer. A
/// validated profile never holds one, but this function is reachable from a
/// caller's own arithmetic and answers for the whole domain rather than for the
/// part a profile happens to use.
///
/// # Errors
///
/// * [`FusionError::InvalidK`] when `k == 0`.
/// * [`FusionError::InvalidRank`] when `rank == 0`.
/// * [`FusionError::Overflow`] when the quotient leaves the fixed-point range.
pub fn weighted_contribution(weight: Fixed, rank: u64, k: u32) -> Result<Fixed, FusionError> {
    let denominator = denominator(rank, k)?;
    let raw = weight.into_raw();
    // `K >= 1` and `rank >= 1` above, so the divisor is at least two and the
    // quotient is at most half of `2^127` — inside `i128` even for the one
    // magnitude (`i128::MIN`'s) that is not representable positive. The
    // conversion is still checked, so widening either operand later cannot turn
    // this into a silent wrap.
    let magnitude = raw.unsigned_abs() / denominator;
    let quotient = i128::try_from(magnitude).map_err(|_| FusionError::Overflow)?;
    Ok(Fixed::from_raw(if raw < 0 { -quotient } else { quotient }))
}

/// The contribution of a stratum weight at a 1-based rank, under whichever rule
/// `decay` names — the one entry point a fusion path should call.
///
/// The smoothing constant is read from the rule rather than passed alongside it,
/// because the rule carries it and the two are never independently chosen.
///
/// # Errors
///
/// Whatever the selected rule refuses: see [`contribution`] and
/// [`weighted_contribution`].
pub fn contribution_under(
    decay: DecayRule,
    weight: Fixed,
    rank: u64,
) -> Result<Fixed, FusionError> {
    match decay {
        DecayRule::ReciprocalRank { k } => contribution(weight, rank, k),
        DecayRule::WeightedReciprocalRank { k } => weighted_contribution(weight, rank, k),
    }
}

/// The largest depth this crate will ever be asked about.
///
/// A plan records a per-stratum depth as a `u32`, so a monotone range at or
/// above that value cannot be exceeded by any depth a plan can carry and there
/// is nothing further to compute. It is a saturation point, not a policy.
const MAX_DEPTH: u64 = u32::MAX as u64;

/// The largest 1-based depth at which `weight`'s contributions are still
/// **strictly** decreasing with rank under `decay`.
///
/// That is: the largest `r` such that `value(w, i) > value(w, i + 1)` holds for
/// every `i` in `1..r`, where `value` is the rule's own arithmetic. A depth of
/// one is always in range — a single rank has no adjacent pair to collide with —
/// and the result saturates at [`MAX_DEPTH`].
///
/// # Why it is searched rather than solved
///
/// Under either rule the exact collision condition involves a truncation that
/// does not commute with the arithmetic around it: two contributions can differ
/// even when the ideal difference is below one unit, because the floors can
/// straddle an integer. A closed form would therefore have to be conservative,
/// and a conservative bound here refuses profiles whose ranks in fact order
/// perfectly — the over-refusal this repository treats as exactly as severe as a
/// wrong answer. So the first collision is found exactly.
///
/// It is not found by walking from rank one. Each rule carries a closed-form
/// *guarantee* of distinctness that is monotone in the rank; a binary search
/// over that guarantee lands the scan on the last rank it covers, and the exact
/// walk runs from there. See [`truncated_guarantee`] and
/// [`weighted_guarantee`] for the two predicates and their proofs.
///
/// # How long the walk is
///
/// The walk covers the gap between the guarantee's boundary and the first real
/// collision, and that gap is not a constant. Both guarantees turn off near
/// `D = sqrt(B)` for their own `B` (`min(w, 1) · S` and `w · S` respectively),
/// and past that boundary a pair collides only when the interval
/// `(B/(D + 1), B/D]` misses an integer, which it does with probability roughly
/// `2 (D - sqrt(B)) / sqrt(B)`. The first miss therefore arrives after about
/// `B^(1/4)` steps. For the truncated rule `B` never exceeds `S`, so the walk is
/// a few hundred to a thousand steps; for the weighted rule `B` grows with the
/// weight, and the walk is bounded by the point where the guarantee alone
/// reaches [`MAX_DEPTH`] and no walk runs at all — under sixty thousand steps at
/// the worst weight, each one division.
pub(crate) fn monotone_depth(decay: DecayRule, weight: Fixed) -> u64 {
    let k = decay.k();
    // Both are fixed by a validated profile; neither has a meaningful answer,
    // and a single rank is the smallest range that is trivially ordered.
    if k == 0 || weight <= Fixed::ZERO {
        return 1;
    }
    let weight_raw = weight.into_raw().unsigned_abs();
    let value: fn(Fixed, u64, u32) -> Result<Fixed, FusionError>;
    let guaranteed = match decay {
        DecayRule::ReciprocalRank { .. } => {
            value = contribution;
            truncated_guarantee(weight_raw, k)
        }
        DecayRule::WeightedReciprocalRank { .. } => {
            value = weighted_contribution;
            weighted_guarantee(weight_raw, k)
        }
    };
    if guaranteed >= MAX_DEPTH {
        return MAX_DEPTH;
    }
    // Start on the last rank the guarantee covers rather than just past it: the
    // repeated pair costs one division and the walk can then only ever find a
    // collision the guarantee wrongly excluded, never miss one.
    first_collision(weight, k, guaranteed.max(1), value)
}

/// The first rank at or after `start` whose contribution equals its successor's,
/// found exactly by walking.
///
/// A rank whose contribution cannot be computed ends the range there: there is
/// no value to order against, so the last rank that had one is the bound.
fn first_collision(
    weight: Fixed,
    k: u32,
    start: u64,
    value: fn(Fixed, u64, u32) -> Result<Fixed, FusionError>,
) -> u64 {
    let mut rank = start;
    let Ok(mut current) = value(weight, rank, k) else {
        return rank;
    };
    while rank < MAX_DEPTH {
        let Ok(next) = value(weight, rank + 1, k) else {
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

/// The largest rank in `0..=MAX_DEPTH` for which `guarantees` holds, or zero
/// when it holds for none.
///
/// Every predicate handed here is non-increasing in the rank, so the search is
/// exact rather than a heuristic jump.
fn largest_rank_satisfying(guarantees: impl Fn(u64) -> bool) -> u64 {
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

/// The largest rank up to which [`contribution`]'s distinctness is guaranteed in
/// closed form, or zero when it is guaranteed for no rank at all.
///
/// The guarantee: with `a = trunc(S / D)`, `b = trunc(S / (D + 1))` and
/// `q = trunc(S / (D · (D + 1)))`, we have `a > S/D − 1` and `b ≤ S/(D + 1)`, so
/// `a − b > S/(D · (D + 1)) − 1 ≥ q − 1`; both sides are integers, so
/// `a − b ≥ q`. Then `w·a/S − w·b/S ≥ w·q/S ≥ 1` whenever `w · q ≥ S`, and
/// `trunc(x) > trunc(y)` whenever `x ≥ y + 1`.
///
/// The inner `trunc` is why this predicate collapses rather than scaling with
/// the weight: once `D · (D + 1) > S` the quotient `q` is zero and the product
/// is zero for every weight, so the guarantee ends just below `D = sqrt(S)` no
/// matter how heavy the stratum is.
fn truncated_guarantee(weight_raw: u128, k: u32) -> u64 {
    let scale = SCALE_RAW.unsigned_abs();
    largest_rank_satisfying(|rank| {
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
    })
}

/// The largest rank up to which [`weighted_contribution`]'s distinctness is
/// guaranteed in closed form, or zero when it is guaranteed for no rank at all.
///
/// The guarantee is the clean integer test `D · (D + 1) ≤ w_raw` with
/// `D = K + r`, and it needs no inner truncation to state: the exact difference
/// between adjacent contributions is `w_raw/D − w_raw/(D + 1) =
/// w_raw/(D · (D + 1))`, so the test says exactly "that difference is at least
/// one unit", and `trunc(x) > trunc(y)` whenever `x ≥ y + 1`.
///
/// The left-hand side is formed in `u128`. `D` is at most
/// `u32::MAX + MAX_DEPTH < 2^33`, so `D · (D + 1) < 2^67` — above a `u64` and
/// far inside a `u128`, which is why the arithmetic is not narrowed. `w_raw` is
/// a [`Fixed`]'s magnitude, at most `2^127`.
fn weighted_guarantee(weight_raw: u128, k: u32) -> u64 {
    largest_rank_satisfying(|rank| {
        let denominator = u128::from(k) + u128::from(rank);
        denominator
            .checked_mul(denominator + 1)
            .is_some_and(|product| product <= weight_raw)
    })
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_DEPTH, contribution, contribution_under, monotone_depth, weighted_contribution,
        weighted_guarantee,
    };
    use crate::fusion_profile::DecayRule;
    use purrdf_text::{Fixed, SCALE_DIGITS};

    /// `10^SCALE_DIGITS`, the raw value of a weight of exactly one.
    const SCALE: i128 = 10_i128.pow(SCALE_DIGITS);

    fn value(weight: Fixed, rank: u64, k: u32) -> Fixed {
        contribution(weight, rank, k).expect("a positive weight and a 1-based rank do not overflow")
    }

    fn weighted(weight: Fixed, rank: u64, k: u32) -> Fixed {
        weighted_contribution(weight, rank, k)
            .expect("a positive weight and a 1-based rank do not overflow")
    }

    fn truncated(k: u32) -> DecayRule {
        DecayRule::ReciprocalRank { k }
    }

    fn folded(k: u32) -> DecayRule {
        DecayRule::WeightedReciprocalRank { k }
    }

    /// The property the bound is defined by, asserted on both sides of it: every
    /// adjacent pair at or below the bound is strictly ordered, and the pair
    /// immediately beyond it is not.
    fn assert_bound_is_exact(weight: Fixed, k: u32) {
        let bound = monotone_depth(truncated(k), weight);
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
    /// weight of 10^6 buys a bound near 10^9; under
    /// [`DecayRule::ReciprocalRank`] it does not, because the reciprocal is
    /// truncated to the scale *before* the weight is applied, so no weight at or
    /// above one can recover a distinction the reciprocal has already lost.
    /// Every weight at or above one therefore shares one bound.
    #[test]
    fn a_weight_above_one_buys_no_depth_under_the_truncated_rule() {
        let unit = monotone_depth(truncated(60), Fixed::ONE);
        for raw in [SCALE, SCALE * 2, SCALE * 1_000, SCALE * 1_000_000] {
            assert_eq!(
                monotone_depth(truncated(60), Fixed::from_raw(raw)),
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

    /// The same weights under [`DecayRule::WeightedReciprocalRank`]: each factor
    /// of one hundred in the weight buys a factor of ten in the bound, because
    /// the bound tracks `sqrt(w · S)`.
    #[test]
    fn a_weight_above_one_buys_depth_under_the_weighted_rule() {
        let unit = monotone_depth(folded(60), Fixed::ONE);
        let mut previous = unit;
        for weight_units in [4_i128, 100, 10_000] {
            let bound = monotone_depth(folded(60), Fixed::from_raw(weight_units * SCALE));
            assert!(
                bound > previous,
                "weight {weight_units} must buy depth over {previous}, got {bound}"
            );
            // sqrt(w · S) = 10^6 · sqrt(w), to within the walk from the
            // guarantee to the first real collision.
            let predicted =
                1_000_000_u64 * integer_sqrt(u128::try_from(weight_units).expect("positive"));
            assert!(
                bound >= predicted && bound < predicted + predicted / 100,
                "bound {bound} must sit just above the predicted {predicted}"
            );
            previous = bound;
        }
    }

    /// An integer square root by binary search — no float anywhere near the
    /// bound this file reasons about.
    fn integer_sqrt(value: u128) -> u64 {
        let mut low = 0_u64;
        let mut high = u64::MAX / 2;
        while low < high {
            let mid = low + (high - low).div_ceil(2);
            if u128::from(mid) * u128::from(mid) <= value {
                low = mid;
            } else {
                high = mid - 1;
            }
        }
        low
    }

    /// Below one the weight binds under both rules, and lowering it lowers the
    /// bound.
    #[test]
    fn a_weight_below_one_lowers_the_bound() {
        for decay in [truncated(1), folded(1)] {
            let mut previous = monotone_depth(decay, Fixed::ONE);
            for raw in [SCALE / 100, SCALE / 10_000, SCALE / 1_000_000, 100, 1] {
                let bound = monotone_depth(decay, Fixed::from_raw(raw));
                assert!(
                    bound < previous,
                    "weight raw {raw} under {decay:?} must lower the bound below {previous}, got {bound}"
                );
                previous = bound;
            }
        }
    }

    /// A weight so small that its every contribution truncates to zero orders
    /// nothing at all, and the bound says so with the smallest range there is
    /// rather than with a number that would license a depth.
    #[test]
    fn a_weight_that_truncates_to_nothing_has_a_bound_of_one() {
        let weight = Fixed::from_raw(1);
        assert_eq!(monotone_depth(truncated(60), weight), 1);
        assert_eq!(value(weight, 1, 60), value(weight, 2, 60));
        assert_eq!(monotone_depth(folded(60), weight), 1);
        assert_eq!(weighted(weight, 1, 60), weighted(weight, 2, 60));
    }

    /// The smoothing constant is the third quantity in the coupling, and it
    /// shifts the bound by exactly what it adds to the denominator — under both
    /// rules, because both divide by `K + r`.
    #[test]
    fn the_smoothing_constant_shifts_the_bound_one_for_one() {
        let weight = Fixed::from_raw(1_000_000);
        for rule in [truncated as fn(u32) -> DecayRule, folded] {
            let base = monotone_depth(rule(1), weight);
            for k in [2_u32, 10, 60, 500] {
                assert_eq!(
                    monotone_depth(rule(k), weight) + u64::from(k),
                    base + 1,
                    "the bound plus K is the first colliding denominator, whatever K is"
                );
            }
        }
    }

    /// The guarantee each search jumps on must never claim a distinctness the
    /// exact walk does not find, so the accelerated answer is compared against
    /// the unaccelerated one over a range small enough to walk from rank one.
    #[test]
    fn the_accelerated_search_agrees_with_a_walk_from_rank_one() {
        for raw in [1_i128, 7, 1_000, 999_983, 1_000_000, 12_345_678] {
            let weight = Fixed::from_raw(raw);
            for k in [1_u32, 3, 60] {
                for decay in [truncated(k), folded(k)] {
                    let at = |rank: u64| {
                        contribution_under(decay, weight, rank).expect("a 1-based rank contributes")
                    };
                    let expected = (1..MAX_DEPTH)
                        .find(|rank| at(*rank) == at(rank + 1))
                        .expect("every weight collides at some rank");
                    assert_eq!(
                        monotone_depth(decay, weight),
                        expected,
                        "weight raw {raw} under {decay:?}"
                    );
                }
            }
        }
    }

    /// The weighted rule's closed-form guarantee is exactly `D · (D + 1) ≤
    /// w_raw`, asserted on both sides of its boundary rather than trusted.
    #[test]
    fn the_weighted_guarantee_is_the_integer_test_it_claims_to_be() {
        let weight_raw = 200_u128 * 10_u128.pow(SCALE_DIGITS);
        let k = 1_u32;
        let rank = weighted_guarantee(weight_raw, k);
        let inside = u128::from(k) + u128::from(rank);
        assert!(
            inside * (inside + 1) <= weight_raw,
            "the boundary rank must satisfy the test"
        );
        let outside = inside + 1;
        assert!(
            outside * (outside + 1) > weight_raw,
            "and the next rank must not"
        );
    }

    /// The weighted rule is the *same function*, computed with one truncation
    /// instead of two: never below the truncated rule's value, and above it by
    /// at most `⌊w⌋ + 1` raw units — which is exactly one unit in the last place
    /// wherever the weight is at most one, the whole range in which the
    /// truncated rule is usable at all.
    #[test]
    fn the_weighted_rule_is_the_truncated_rule_computed_more_accurately() {
        for weight_raw in [1_i128, 1_000_000, SCALE / 2, SCALE, 7 * SCALE, 200 * SCALE] {
            let weight = Fixed::from_raw(weight_raw);
            let slack = weight_raw / SCALE + 1;
            for k in [1_u32, 60, 500] {
                for rank in [1_u64, 2, 17, 1_000, 999_999, 1_000_001, 8_508_044] {
                    let exact = weighted(weight, rank, k);
                    // The exactly-rounded value, spelled out independently of
                    // the implementation.
                    assert_eq!(
                        exact.into_raw(),
                        weight_raw / (i128::from(k) + i128::from(rank)),
                        "the weighted rule must be trunc(w_raw / (K + r))"
                    );
                    let truncated_value = value(weight, rank, k);
                    let gap = exact.into_raw() - truncated_value.into_raw();
                    assert!(
                        gap >= 0,
                        "the truncated rule must never exceed the exactly-rounded value"
                    );
                    assert!(
                        gap <= slack,
                        "gap {gap} at weight raw {weight_raw}, K {k}, rank {rank} exceeds {slack}"
                    );
                    if weight_raw <= SCALE {
                        assert!(gap <= 1, "at a weight of at most one the gap is one ULP");
                    }
                }
            }
        }
    }

    /// The dispatcher runs the rule it is handed and nothing else.
    #[test]
    fn the_dispatcher_selects_the_named_rule() {
        let weight = Fixed::from_raw(200 * SCALE);
        assert_eq!(
            contribution_under(truncated(60), weight, 7).expect("fits"),
            contribution(weight, 7, 60).expect("fits")
        );
        assert_eq!(
            contribution_under(folded(60), weight, 7).expect("fits"),
            weighted_contribution(weight, 7, 60).expect("fits")
        );
    }

    /// Both rules refuse the same unusable operands with the same typed errors.
    #[test]
    fn both_rules_refuse_a_zero_constant_and_a_zero_rank() {
        assert!(weighted_contribution(Fixed::ONE, 1, 0).is_err());
        assert!(weighted_contribution(Fixed::ONE, 0, 60).is_err());
        assert!(contribution(Fixed::ONE, 1, 0).is_err());
        assert!(contribution(Fixed::ONE, 0, 60).is_err());
    }

    /// A negative weight truncates toward zero under the weighted rule, the same
    /// direction every other rounding in the layer takes.
    #[test]
    fn a_negative_weight_truncates_toward_zero() {
        let negative = Fixed::from_raw(-7);
        assert_eq!(weighted(negative, 1, 1).into_raw(), -3);
        assert_eq!(weighted(Fixed::from_raw(7), 1, 1).into_raw(), 3);
    }
}
