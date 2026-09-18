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

use core::convert::Infallible;

use purrdf_text::{Fixed, SCALE_DIGITS};

use crate::error::FusionError;
use crate::fusion_profile::DecayRule;

/// `10^SCALE_DIGITS` — the scale of a [`Fixed`]'s raw integer.
const SCALE_RAW: i128 = 10_i128.pow(SCALE_DIGITS);

/// The same scale as a `u64`, for the reciprocal divisions.
///
/// `10^12` is a third of the way into a `u64` and a denominator is under `2^33`,
/// so `S / D` is a narrow division. It is the innermost operation of the weight
/// search, where it runs hundreds of millions of times, and a `u128` divide
/// there is a software routine rather than an instruction.
const SCALE_NARROW: u64 = 10_u64.pow(SCALE_DIGITS);

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

/// A rule's smoothing constant, or the refusal that says the rule is not a law.
///
/// `K = 0` makes a rank-one contribution the whole weight rather than a fraction
/// of it, which is why [`FusionProfile::with_decay`](crate::FusionProfile::with_decay)
/// refuses it where a profile is declared. Everything below is reachable beneath
/// that check, so every entry point reads its constant through here **before it
/// reads the question it was asked**.
///
/// It is a separate step rather than a consequence of [`denominator`] because
/// the arithmetic is not always reached. A depth of one has no adjacent pair to
/// separate and a tolerance of one does no walking, so both would return from a
/// short-circuit without ever forming a denominator — and the number they would
/// return is the most favourable one the function can produce, quoted for a rule
/// that cannot be evaluated. That is the vacuous answer a depth of zero is
/// already refused for, wearing the same shape.
///
/// It is also why an unusable constant is refused *as* an unusable constant.
/// Past the short-circuit the walk would fail somewhere inside the search and
/// the failure would be told as whatever that point reports — for a depth of two
/// or more, that the decay rule ran out of separation at depth one. The rule did
/// not run out of anything: it was never a rule. A malformed law, a saturated
/// rule and a depth no plan can carry are three different facts about three
/// different things, and each carries its own variant.
fn usable_k(decay: DecayRule) -> Result<u32, FusionError> {
    let k = decay.k();
    if k == 0 {
        return Err(FusionError::InvalidK { k });
    }
    Ok(k)
}

/// A class-width tolerance, or the refusal that says it describes no class.
///
/// An indifference class always contains its own rank, so the narrowest class
/// that exists is one rank wide and a tolerance of zero admits none. This is
/// [`usable_k`]'s shape exactly — an operand with no evaluable content, read
/// before the question it qualifies — and it is refused rather than clamped for
/// the same reason. Clamping it to one answers the narrowest *real* tolerance
/// in its place, which is the deepest fully-separated depth this algebra can
/// report: the most favourable answer available, returned precisely where
/// nothing was asked.
///
/// It is not [`FusionError::InvalidRank`]. A rank of zero is a position that
/// does not exist on a 1-based axis; a tolerance of zero is a *count of ranks*
/// measured across that axis, and telling a caller that "rank must be at least
/// 1" points at an argument that was never at fault.
fn usable_width(max_width: u64) -> Result<u64, FusionError> {
    if max_width == 0 {
        return Err(FusionError::InvalidWidth { max_width });
    }
    Ok(max_width)
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

/// The deepest depth a plan is able to express.
///
/// A plan records a per-stratum depth as a `u32`, so nothing downstream can
/// carry a depth above this. It bounds what this file computes in two distinct
/// ways, and conflating them is the one mistake this constant invites. A
/// *measured* range that reaches it is reported as [`MonotoneDepth`]'s
/// saturating case, because there is no bound inside the expressible range to
/// report. A *requested* depth above it is refused as a limit of the depth
/// encoding a plan carries ([`FusionError::DepthBeyondPlanRange`]) — never as
/// the decay rule running out of separation, which is a different fact about a
/// different thing and which the rule's arithmetic may well not have done.
const MAX_DEPTH: u64 = u32::MAX as u64;

/// How deep a profile's contributions still tell adjacent ranks apart.
///
/// The bound is a saturating quantity, so it is spelled as one. A plan records a
/// per-stratum depth as a `u32`; a profile whose contributions never collide
/// inside that range has no bound to report, and handing back `u32::MAX` as
/// though it were a measured depth invites a caller to log it, plot it, or
/// divide by it. A saturation point is not a measurement, and this type is the
/// difference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MonotoneDepth {
    /// Every adjacent pair of ranks up to and including this 1-based depth
    /// carries a distinct contribution, and the pair immediately after it does
    /// not. Exact, not a conservative estimate.
    SeparatesTo(u64),
    /// No adjacent pair collides within any depth a plan is able to express.
    /// There is no bound to report, not an enormous one.
    SeparatesBeyondAnyPlan,
}

impl MonotoneDepth {
    /// Read a raw first-collision rank as the saturating quantity it is.
    #[must_use]
    pub(crate) const fn from_rank(rank: u64) -> Self {
        if rank >= MAX_DEPTH {
            Self::SeparatesBeyondAnyPlan
        } else {
            Self::SeparatesTo(rank)
        }
    }

    /// Whether reading to `depth` stays inside the range that still orders.
    #[must_use]
    pub const fn covers(self, depth: u64) -> bool {
        match self {
            Self::SeparatesBeyondAnyPlan => true,
            Self::SeparatesTo(bound) => depth <= bound,
        }
    }

    /// The bound as a number, when there is one to report.
    #[must_use]
    pub const fn rank(self) -> Option<u64> {
        match self {
            Self::SeparatesBeyondAnyPlan => None,
            Self::SeparatesTo(bound) => Some(bound),
        }
    }
}

/// How deep a profile can be read with every rank the read reaches sitting in
/// an indifference class no wider than a stated tolerance.
///
/// This is [`MonotoneDepth`]'s sibling, not a second spelling of it, and the
/// two are kept apart because their reported cases claim different things.
/// [`MonotoneDepth::SeparatesTo`] asserts that *every adjacent pair* up to the
/// bound carries a distinct contribution — the most favourable claim this
/// algebra makes. A tolerance above one buys depth by giving exactly that up:
/// at the depth reported here ranks do share contributions, just never more
/// than the tolerance of them within the read. Reusing the separating type to
/// carry a tolerated depth would attach a separation claim to a depth measured
/// under no such claim, which is the defect this whole algebra is written to
/// avoid.
///
/// The saturating case is spelled for the same reason it is on
/// [`MonotoneDepth`]: a plan records a per-stratum depth as a `u32`, so a
/// tolerance no depth in that range ever exceeds has no bound to report, and
/// handing back `u32::MAX` as though it were a measured depth invites a caller
/// to log it, plot it, or divide by it. Two weights fifty times apart both
/// saturate, and a bare number says they are the same depth; the saturating
/// case says only what is true, which is that neither has a bound inside any
/// plan's reach.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToleratedDepth {
    /// A read truncated at this 1-based depth puts every rank it reads in a
    /// class no wider than the tolerance, and a read one rank deeper does not.
    /// Exact, not a conservative estimate.
    ReadsTo(u64),
    /// No depth a plan is able to express exceeds the tolerance. There is no
    /// bound to report, not an enormous one.
    ReadsBeyondAnyPlan,
}

impl ToleratedDepth {
    /// Read a raw walked depth as the saturating quantity it is.
    #[must_use]
    pub(crate) const fn from_rank(rank: u64) -> Self {
        if rank >= MAX_DEPTH {
            Self::ReadsBeyondAnyPlan
        } else {
            Self::ReadsTo(rank)
        }
    }

    /// Whether reading to `depth` stays inside the tolerated range.
    #[must_use]
    pub const fn covers(self, depth: u64) -> bool {
        match self {
            Self::ReadsBeyondAnyPlan => true,
            Self::ReadsTo(bound) => depth <= bound,
        }
    }

    /// The depth as a number, when there is one to report.
    #[must_use]
    pub const fn rank(self) -> Option<u64> {
        match self {
            Self::ReadsBeyondAnyPlan => None,
            Self::ReadsTo(bound) => Some(bound),
        }
    }
}

/// How many consecutive ranks one indifference class runs to.
///
/// This is a **count of ranks**, not a depth, which is why it is neither
/// [`MonotoneDepth`] nor [`ToleratedDepth`] and cannot borrow either of their
/// cases. `MonotoneDepth::SeparatesTo` asserts that every adjacent pair up to a
/// depth is distinct and `ToleratedDepth::ReadsTo` asserts that a read stopping
/// at a depth stays inside a tolerance; a width asserts neither of those things
/// about anything. It says only how many ranks one contribution covers, and a
/// variant that carried a depth claim alongside it would assert something the
/// measurement never established — the precise defect that kept the two depth
/// types apart from each other.
///
/// The saturating case is spelled for the reason both siblings spell theirs. A
/// plan records a per-stratum depth as a `u32`, so the widest class that can be
/// counted end to end is one that ends inside that range; a class still running
/// at the last expressible rank has no counted end, and `u32::MAX` there is the
/// search's ceiling rather than a width anybody measured. Under the truncated
/// rule at a raw weight of one every contribution truncates to zero, so the run
/// is unbounded — and at a raw weight of fifty, fifty times heavier, it is
/// unbounded too. A bare number says those two classes are the same width and
/// invites a caller to log it, plot it, or divide by it; the saturating case
/// says only what is true, which is that neither class ends inside any plan's
/// reach.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClassWidth {
    /// The class is exactly this many consecutive ranks wide, both of its ends
    /// found inside the range a plan can express. One means the rank is
    /// separated from both of its neighbours. Exact, not a conservative
    /// estimate.
    SpansTo(u64),
    /// The class is still running at the deepest rank a plan is able to
    /// express, so no end was found inside that range and there is no width to
    /// report — not an enormous one. This claims that the far end was not
    /// found, which is all the search establishes; whether the class stops at
    /// that last rank or runs on past it is a question about ranks no plan can
    /// carry, and one the probe never asked.
    ExceedsAnyPlan,
}

impl ClassWidth {
    /// Read a counted class, given the 1-based ranks that bound it, as the
    /// saturating quantity it is.
    ///
    /// `last` at or past [`MAX_DEPTH`] is the search's ceiling rather than the
    /// class's end: the probe never looks deeper, so what it found is "still
    /// equal at the last rank a plan can express" and not "equal up to here and
    /// different after".
    #[must_use]
    pub(crate) const fn from_bounds(first: u64, last: u64) -> Self {
        if last >= MAX_DEPTH {
            Self::ExceedsAnyPlan
        } else {
            Self::SpansTo(last.saturating_sub(first).saturating_add(1))
        }
    }

    /// Whether this class is no wider than `max_width`.
    ///
    /// A class with no end inside any plan's reach is never within a tolerance,
    /// which is the opposite polarity to [`MonotoneDepth::covers`] and
    /// [`ToleratedDepth::covers`] and is so for a reason: saturating there means
    /// "no bound was ever exceeded", while saturating here means "the bound was
    /// exceeded before the range ran out".
    #[must_use]
    pub const fn fits_within(self, max_width: u64) -> bool {
        match self {
            Self::ExceedsAnyPlan => false,
            Self::SpansTo(width) => width <= max_width,
        }
    }

    /// The width as a number, when there is one to report.
    #[must_use]
    pub const fn width(self) -> Option<u64> {
        match self {
            Self::ExceedsAnyPlan => None,
            Self::SpansTo(width) => Some(width),
        }
    }
}

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
///
/// # Errors
///
/// [`FusionError::InvalidK`] when the rule's smoothing constant is zero. That
/// is a malformed law rather than a weight that fails to separate, so it is
/// refused before a rank is read — see [`usable_k`] for why answering it with
/// the smallest ordered range would be a vacuous answer wearing a measurement's
/// shape.
///
/// Otherwise whatever the selected rule refuses at a rank the walk reads: in
/// practice [`FusionError::Overflow`] alone, for a contribution that leaves the
/// fixed-point range.
///
/// No profile reaches that last arm today. The one operand that could — a
/// non-positive weight — is short-circuited below before any rank is read, and
/// [`FusionProfile::with_decay`](crate::FusionProfile::with_decay) refuses it
/// and the zero constant outright; past those guards the walk reads ranks at or
/// above one under a constant at or above one, where neither rule's arithmetic
/// can leave the range. The refusal is nonetheless carried rather than rendered
/// as a separation bound, so that moving a guard surfaces the failure instead of
/// quietly converting it into the most flattering depth available.
pub(crate) fn monotone_depth(decay: DecayRule, weight: Fixed) -> Result<u64, FusionError> {
    // The law before the measurement. An unusable constant is refused rather
    // than answered with the smallest ordered range, because that range would be
    // a real measurement's shape carrying no measurement.
    let k = usable_k(decay)?;
    // A weight of zero maps every rank to zero, so the first adjacent pair
    // already collides and a single rank is the *measured* answer rather than a
    // stand-in for one. A negative weight is not a weight a profile can hold —
    // `FusionProfile::with_decay` refuses one — and is read the same way here so
    // that the short-circuit covers the whole non-positive half.
    if weight <= Fixed::ZERO {
        return Ok(1);
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
        return Ok(MAX_DEPTH);
    }
    // Start on the last rank the guarantee covers rather than just past it: the
    // repeated pair costs one division and the walk can then only ever find a
    // collision the guarantee wrongly excluded, never miss one.
    first_collision(weight, k, guaranteed.max(1), value)
}

/// The first rank at or after `start` whose contribution equals its successor's,
/// found exactly by walking.
///
/// # Errors
///
/// Whatever `value` refuses, at the first rank the walk reads it at:
/// [`FusionError::InvalidK`] for a zero smoothing constant,
/// [`FusionError::InvalidRank`] for a `start` of zero, and
/// [`FusionError::Overflow`] for a value that leaves the fixed-point range.
///
/// A refusal is reported and never rendered as a bound. The rank the walk
/// stopped at is where the arithmetic gave out, not a rank this weight was
/// measured to separate; handing it back as one would make the most favourable
/// claim this function can make — "every adjacent pair below here is distinct" —
/// at exactly the point where no pair was compared at all.
fn first_collision(
    weight: Fixed,
    k: u32,
    start: u64,
    value: fn(Fixed, u64, u32) -> Result<Fixed, FusionError>,
) -> Result<u64, FusionError> {
    let mut rank = start;
    let mut current = value(weight, rank, k)?;
    while rank < MAX_DEPTH {
        let next = value(weight, rank + 1, k)?;
        if next == current {
            return Ok(rank);
        }
        current = next;
        rank += 1;
    }
    Ok(MAX_DEPTH)
}

/// The number of consecutive ranks around `rank` that carry one contribution.
///
/// This is the width of `rank`'s **indifference class**: the set of ranks whose
/// contributions this profile cannot tell apart. One means the rank is still
/// separated from both its neighbours; `w` means the fused score treats `w`
/// consecutive ranks as equal and their relative order falls through to the
/// declared tie-break's later keys.
///
/// [`monotone_depth`] is the single point where this first exceeds one. The
/// width is the whole curve, and past the bound it grows: under the truncated
/// rule it runs about `D²/S`, so it is roughly four ranks at two million, a
/// hundred at ten million, and ten thousand at a hundred million. Reporting only
/// whether the bound was crossed would flatten that into one bit.
///
/// The growth has an end, which is why the answer is a [`ClassWidth`] rather
/// than a number. A class that is still running at the deepest rank a plan can
/// express has no counted width — under the truncated rule at a raw weight of
/// one every contribution truncates to zero, so rank one's class is the whole
/// expressible range, and so it is at a raw weight of fifty. That case is
/// [`ClassWidth::ExceedsAnyPlan`], not the ceiling handed back as a count.
///
/// Both ends are found with [`largest_rank_satisfying_checked`] rather than by
/// walking: contributions are non-increasing in the rank, so "is this rank's
/// contribution at least `v`" and "is it strictly above `v`" are both
/// non-increasing predicates, and their boundaries are the class's last and
/// first rank.
///
/// # Errors
///
/// [`FusionError::InvalidK`] when the rule's smoothing constant is zero,
/// refused before the anchor is formed because it is a malformed law rather
/// than a rank this law cannot resolve.
///
/// Then whatever [`contribution_under`] refuses, at the anchor rank or at any
/// rank the two searches probe: [`FusionError::InvalidRank`] for a rank of
/// zero, and [`FusionError::Overflow`] for a product that leaves the
/// fixed-point range.
///
/// A refusal is reported and never rendered as a width. A width of one is the
/// claim "this rank is separated from both its neighbours", which is the most
/// favourable thing this function can say about a profile's resolution; saying
/// it because the arithmetic failed would be a false claim about the quality of
/// an answer, made exactly where no answer was computed at all.
pub(crate) fn class_width(
    decay: DecayRule,
    weight: Fixed,
    rank: u64,
) -> Result<ClassWidth, FusionError> {
    // The law before the question, and stated here rather than left to the
    // anchor's own arithmetic: the three entry points over this file must agree
    // about what an unusable constant is, and agreeing by accident of which
    // inner step happens to be reached first is not agreeing.
    usable_k(decay)?;
    let value = contribution_under(decay, weight, rank)?;
    // Rank zero is not a rank; it anchors both searches so the predicate is
    // non-increasing over the whole `0..=MAX_DEPTH` span the search covers.
    let last = largest_rank_satisfying_checked(|probe| -> Result<bool, FusionError> {
        if probe == 0 {
            return Ok(true);
        }
        Ok(contribution_under(decay, weight, probe)? >= value)
    })?;
    let before_first = largest_rank_satisfying_checked(|probe| -> Result<bool, FusionError> {
        if probe == 0 {
            return Ok(true);
        }
        Ok(contribution_under(decay, weight, probe)? > value)
    })?;
    let first = before_first.saturating_add(1);
    // `last` at the ceiling is where the search stopped looking, not where the
    // class stopped. Counting to it would quote the walk's own limit as a width.
    Ok(ClassWidth::from_bounds(first, last))
}

/// The deepest **depth** that can be read with every rank in it sitting in a
/// class no wider than `max_width`.
///
/// This is a depth, not a rank property, and the distinction is load-bearing at
/// the boundary. If the first collision is at rank `r`, then reading to depth `r`
/// yields `r` mutually distinct contributions — rank `r + 1`, the one it collides
/// with, was never read. So depth `r` is fully separated even though rank `r`'s
/// class in the unbounded sequence has width two. Asking instead for the deepest
/// rank whose own class is a singleton answers `r - 1`, understating by one what
/// the arithmetic in fact delivers; a conservative bound here would discourage a
/// depth that orders perfectly, which is the same defect as refusing it.
///
/// With `max_width` of one this therefore agrees exactly with
/// [`monotone_depth`] — and [`class_width`] at the returned depth never reports
/// a class of one, for exactly that reason: the class the *unbounded* curve puts
/// that rank in also looks at rank `r + 1`, the one rank the bounded read never
/// reaches.
///
/// That offset is a law at every tolerance. The walk returns `D` only once
/// `run_start..=D + 1` — `max_width + 1` consecutive ranks, `D` among them —
/// have been observed to share one contribution, and `class_width(D)` measures
/// the whole maximal run containing `D`. So `class_width` at this function's
/// answer is **never** `max_width` and never one; where it counts a width at all
/// that width is at least `max_width + 1`. It is exactly `max_width + 1` when
/// the run that ended the walk is exactly one rank longer than the tolerance,
/// which is the ordinary case wherever the curve is smooth at that depth, and it
/// is wider — sometimes far wider — where the run is longer still. Under the
/// truncated rule at a raw weight of `10^3` a tolerance of two lands on a depth
/// whose full class is four; at a raw weight of `10^2` a tolerance of one lands
/// on depth one, whose full class is forty, so the bound is the only claim that
/// holds across weights and the `max_width + 1` equality is the smooth case
/// rather than the rule. Where the run reaches the end of the expressible range
/// — a tolerance of 4096 at a raw weight of `10^3`, or any tolerance at a raw
/// weight of one, where every contribution has truncated to the same value —
/// there is no width to compare at all and [`class_width`] answers
/// [`ClassWidth::ExceedsAnyPlan`]. The two functions are agreeing in all of
/// those cases, not disagreeing; they are answering a depth question and a rank
/// question.
///
/// The same truncation argument generalizes to any `max_width`. Suppose a run
/// of mutually colliding ranks begins at `run_start`, and the first depth at
/// which it would exceed the tolerance is `run_start + max_width` — that is,
/// `run_start` through `run_start + max_width` is `max_width + 1` ranks
/// sharing one contribution, one more than the tolerance allows. Truncating
/// the read at some depth `D` inside that run never observes the ranks past
/// `D`, so the class it puts `run_start` in has width exactly
/// `D - run_start + 1`, not the run's full length. That stays within
/// `max_width` for every `D` up to `run_start + max_width - 1`, and first
/// exceeds it at `D = run_start + max_width`. So the deepest admissible depth
/// is `run_start + max_width - 1`, the last rank before the run tips over —
/// not `run_start`, which is what a conservative reader would guess and which
/// understates the true bound by `max_width - 1`. At `max_width` of one the
/// two expressions coincide, which is why that boundary case cannot by itself
/// reveal an off-by-`max_width - 1` error in the general rule.
///
/// # Why this walks rather than bisects
///
/// The class width is **not** monotone in the rank, so a binary search over
/// "is this rank's class narrow enough" is unsound and silently over-reports.
/// The trend grows, but locally the floors straddle: under the truncated rule at
/// a weight of `10^6` raw units, rank 972 already shares its contribution with
/// 973, while rank 1038 is alone again. Bisecting that predicate answers 1039
/// for a tolerance of one — claiming a depth fully separated when it passed a
/// collision sixty ranks earlier. That is a false claim about the quality of an
/// answer, which is worse than declining to make one.
///
/// So the run lengths are counted exactly. The walk starts at
/// [`monotone_depth`] rather than at rank one, because every class below that
/// point has width one by its definition and cannot be what exceeds
/// `max_width`. A `max_width` of zero is refused rather than read as one: a
/// class always contains its own rank, so a tolerance of zero describes no
/// class at all, and reading it as the narrowest real tolerance would answer a
/// question nobody asked with the most favourable depth that question has.
///
/// # How long the walk is
///
/// It is a walk over ranks, so its length is the answer itself minus the depth
/// it started from, and both ends are known in closed form up to the straddling
/// the exact count exists to resolve.
///
/// Under either rule adjacent contributions differ by about `B / (D · (D + 1))`
/// for that rule's own `B` (`min(w, 1) · S` for the truncated rule, `w · S` for
/// the folded one, as [`monotone_depth`] sets out), so a class at depth `D` is
/// about `D² / B` ranks wide and the separating depth is about `sqrt(B)`.
/// Setting `D² / B = max_width` puts the answer at about `sqrt(max_width)`
/// times the separating depth, and the walk at about `sqrt(max_width) - 1`
/// times it — measured across both rules and four weights, the ratio of answer
/// to separating depth tracks `sqrt(max_width)` to three digits. A tolerance of
/// one walks not at all, four costs one separating depth's worth of steps,
/// 4096 costs about sixty-three. Each step is one integer division.
///
/// The separating depth is therefore what sets the scale, and it is near `10^6`
/// under the truncated rule at every weight but near `10^6 · sqrt(w)` under the
/// folded one — so a large tolerance at a heavy folded weight is where this
/// walks farthest.
///
/// It never walks past [`MAX_DEPTH`]: above that there is no bound inside the
/// expressible range to report, and the answer is
/// [`ToleratedDepth::ReadsBeyondAnyPlan`] rather than the ceiling dressed as a
/// reading. That bounds the whole walk at fewer than `2^32` steps no matter
/// what it is asked. A saturating call is the slowest one there is and it
/// terminates; none of this is a hazard to be guarded against, but it is a cost
/// worth knowing before putting the call somewhere it runs more than once.
///
/// # Errors
///
/// [`FusionError::InvalidK`] when the rule's smoothing constant is zero,
/// refused before anything is measured or walked. A tolerance of one does no
/// walking at all, so leaving this to the walk would let the widest tolerance
/// refuse and the narrowest one answer under the very same unusable rule.
///
/// [`FusionError::InvalidWidth`] when `max_width` is zero, refused on the same
/// terms and for the same reason: a class always contains its own rank, so a
/// tolerance of zero is not a tolerance. It is checked beside the constant
/// rather than at the walk, because the tolerance decides how far the walk
/// runs and a malformed one has no walk to refuse it.
///
/// Then whatever [`contribution_under`] refuses at any rank the walk reads:
/// [`FusionError::Overflow`] for a product that leaves the fixed-point range.
/// The walk starts at rank one or deeper, so it never forms the zero rank
/// [`FusionError::InvalidRank`] names.
///
/// A refusal is reported and never rendered as a depth. The rank the walk
/// stopped at is where the arithmetic gave out, not a depth this profile was
/// measured to deliver, and returning it as one would quote a resolution
/// nothing established.
pub(crate) fn deepest_rank_within_width(
    decay: DecayRule,
    weight: Fixed,
    max_width: u64,
) -> Result<ToleratedDepth, FusionError> {
    // The law before the question. [`monotone_depth`] below refuses an unusable
    // constant too, so this is not what makes the refusal reachable; it is what
    // keeps it reachable if a short-circuit is ever hoisted above the
    // measurement, which is exactly how a tolerance of one came to answer for a
    // rule that could not be evaluated.
    usable_k(decay)?;
    // And the question before the measurement. A class always contains its own
    // rank, so zero describes no class; normalising it up to one answered the
    // narrowest real tolerance in its place, which is the most favourable thing
    // this function can say, said where nothing was asked.
    let ceiling = usable_width(max_width)?;
    let start = monotone_depth(decay, weight)?;
    if ceiling == 1 || start >= MAX_DEPTH {
        return Ok(ToleratedDepth::from_rank(start));
    }

    // Walk forward, counting consecutive ranks that carry one contribution. The
    // first run longer than `ceiling` ends the range at the rank that run began,
    // because reading that far would put `ceiling + 1` ranks in one class.
    let mut current = contribution_under(decay, weight, start)?;
    let mut run_start = start;
    let mut rank = start;
    while rank < MAX_DEPTH {
        let next = contribution_under(decay, weight, rank + 1)?;
        if next == current {
            if rank + 1 - run_start + 1 > ceiling {
                return Ok(ToleratedDepth::ReadsTo(rank));
            }
        } else {
            current = next;
            run_start = rank + 1;
        }
        rank += 1;
    }
    Ok(ToleratedDepth::ReadsBeyondAnyPlan)
}

/// What one adjacent-rank constraint says about one candidate weight.
///
/// A profile reaches depth `D` exactly when every constraint in the range holds,
/// so the search below is an intersection over these, and the middle variant is
/// what makes that intersection cheap to walk: a failing constraint does not
/// merely say "no", it says where the next weight that could say "yes" is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Separation {
    /// The two ranks already carry distinct contributions at this weight.
    Holds,
    /// They collide here, and this is the **next** weight that can separate
    /// them: strictly heavier than the one probed, with every weight in between
    /// ruled out by the same algebra that ruled this one out.
    NextWeight(i128),
    /// They collide at every weight. Only [`DecayRule::ReciprocalRank`] can say
    /// this, and it is the rule's real saturation rather than a shortfall in the
    /// search.
    Impossible,
}

/// Whether `weight` separates the adjacent ranks whose denominators are
/// `denominator` and `denominator + 1`, and if not, where to look next.
///
/// # The algebra, one rule at a time
///
/// Write `S` for [`SCALE_RAW`], `w` for the weight's raw integer and `a` for the
/// smaller denominator.
///
/// **Folded** ([`DecayRule::WeightedReciprocalRank`]) contributes
/// `trunc(w / a)`, so the pair separates iff `⌊w/a⌋ > ⌊w/(a + 1)⌋`. Writing
/// `w = q·a + s`, that is `w < q·(a + 1)`, i.e. `s < q` — one division to
/// decide. Restated over `w`, the constraint holds exactly on the union of
/// intervals `[m·a, m·(a + 1) − 1]` for integer `m ≥ 1`, which are disjoint and
/// increasing. A failing `w` therefore sits strictly above the interval for
/// `m = ⌊w/(a + 1)⌋` and strictly below the next one, so `(m + 1)·a` is the next
/// satisfying weight and nothing between the two is skipped.
///
/// **Truncated** ([`DecayRule::ReciprocalRank`]) contributes
/// `trunc(w·I / S)` with `I = ⌊S/a⌋`, and `I` does not depend on the weight. If
/// `I` repeats at `a + 1` the two contributions are equal for *every* weight,
/// which is [`Separation::Impossible`]. Otherwise the pair collides exactly when
/// `⌊w·I₁/S⌋ = ⌊w·I₂/S⌋ = m`, and since `⌊w·I₁/S⌋` is non-decreasing in `w` and
/// bounds `⌊w·I₂/S⌋` from above, no weight below `⌈(m + 1)·S / I₁⌉` can lift
/// either side off `m`. That ceiling is the next candidate.
///
/// Both next-candidate formulas are strictly greater than the weight probed,
/// which is what makes the search terminate.
fn separation_at(decay: DecayRule, weight: i128, denominator: u64) -> Separation {
    match decay {
        DecayRule::WeightedReciprocalRank { .. } => {
            let narrow = i128::from(denominator);
            let quotient = weight / narrow;
            if weight % narrow < quotient {
                Separation::Holds
            } else {
                Separation::NextWeight((weight / (narrow + 1) + 1) * narrow)
            }
        }
        DecayRule::ReciprocalRank { .. } => {
            // Both `S` and a denominator fit a `u64` — the scale is `10^12` and
            // a denominator is under `2^33` — so the two reciprocals are narrow
            // divisions rather than wide ones. That is the sweep's inner loop.
            let here = i128::from(SCALE_NARROW / denominator);
            let beyond = i128::from(SCALE_NARROW / (denominator + 1));
            if here == beyond {
                return Separation::Impossible;
            }
            // The exact gap between the two contributions is `w·(I₁ − I₂)/S`,
            // and `trunc(x) > trunc(y)` whenever `x ≥ y + 1`, so a gap of a
            // whole unit settles the pair without the two truncations below.
            // This is the common case away from the saturation point, and
            // taking it early is what keeps the sweep affordable at depths near
            // a million.
            if weight * (here - beyond) >= SCALE_RAW {
                return Separation::Holds;
            }
            let level = weight * here / SCALE_RAW;
            if level > weight * beyond / SCALE_RAW {
                Separation::Holds
            } else {
                // ⌈(level + 1)·S / here⌉, formed without leaving the integers.
                let target = (level + 1) * SCALE_RAW;
                Separation::NextWeight((target + here - 1) / here)
            }
        }
    }
}

/// How far below its ideal value [`contribution`] truncates at `denominator`,
/// for a weight `shortfall` raw units below one.
///
/// With `w = S − f` and `I = ⌊S/D⌋`, the contribution is
/// `⌊w·I/S⌋ = I − ⌈f·I/S⌉` — an exact integer identity, because
/// `⌊−x⌋ = −⌈x⌉`. This returns that second term, and it is the whole reason the
/// truncated rule's sweep is affordable.
///
/// `I` is non-increasing in the denominator and `f` is not negative, so the
/// shortfall is non-increasing too. Adjacent ranks therefore separate exactly
/// when `I` falls by more than the shortfall does, and wherever the shortfall is
/// **flat** across a stretch of denominators every pair in that stretch
/// separates — one comparison clears the whole stretch instead of one probe per
/// rank. Near the saturation point that is hundreds of thousands of ranks
/// cleared by two divisions.
fn shortfall_at(shortfall: i128, denominator: u64) -> i128 {
    let reciprocal = i128::from(SCALE_NARROW / denominator);
    // ⌈f·I / S⌉, formed without leaving the integers. `f ≤ S` and `I ≤ S/2`, so
    // the product stays far inside an `i128`.
    (shortfall * reciprocal + SCALE_RAW - 1) / SCALE_RAW
}

/// What one sweep of the constraints over a weight found.
///
/// Three outcomes and not two: "nothing failed" is the answer, "something
/// failed" carries where to look next, and "something can never hold" is the
/// truncated rule's saturation rather than either of those.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Lap {
    /// Every constraint in the range separates its pair of ranks at this weight.
    Separated,
    /// At least one did not, and this is the heaviest next candidate any of them
    /// named — the lightest weight above this one that could satisfy all of them.
    Advance(i128),
    /// A pair in the range collides at every weight there is.
    Unreachable,
}

impl From<Option<i128>> for Lap {
    fn from(heaviest: Option<i128>) -> Self {
        heaviest.map_or(Self::Separated, Self::Advance)
    }
}

/// What one sweep of the constraints in `[floor, widest]` found at `weight`.
///
/// # Why the heaviest and not the first
///
/// Every weight that reaches the depth satisfies every constraint, so it is at
/// or above each failing constraint's next candidate, and therefore at or above
/// their maximum. Jumping to the maximum skips no solution and covers far more
/// ground per lap than jumping to whichever constraint the sweep met first.
///
/// # Why the truncated rule sometimes steps rather than scans
///
/// Under [`DecayRule::ReciprocalRank`] a flat stretch of [`shortfall_at`] is a
/// stretch of ranks that all separate, so the sweep can jump from one step of
/// the shortfall to the next and probe only those, and each jump is closed form
/// rather than a search: the shortfall first falls below `h` at the denominator
/// where `I(D)` first falls to `⌊(h − 1)·S / f⌋`, which inverts to
/// `D = ⌊S / (that + 1)⌋ + 1`. Near the saturation point that is a couple of
/// dozen probes in place of three hundred thousand.
///
/// Far from it the shortfall steps at nearly every rank and the jumps cost more
/// than they save, so the count of steps — `shortfall_at(floor) −
/// shortfall_at(widest + 1)`, one subtraction — picks between the two sweeps.
/// The choice is an accelerator either way: both probe every rank that can fail.
///
/// The stepping sweep reads distinctness off `I(D) > I(D + 1)` alone, which is
/// the saturation condition the caller has already established for this whole
/// range by checking that a weight of one reaches the depth. Under
/// [`DecayRule::WeightedReciprocalRank`] no such shortcut is needed: that rule's
/// closed-form guarantee is exact, so the range it leaves open is short.
fn sweep(decay: DecayRule, weight: i128, floor: u64, widest: u64) -> Lap {
    let mut heaviest: Option<i128> = None;
    let probe =
        |cursor: u64, heaviest: &mut Option<i128>| match separation_at(decay, weight, cursor) {
            Separation::Holds => true,
            Separation::Impossible => false,
            Separation::NextWeight(next) => {
                *heaviest = Some(heaviest.map_or(next, |far: i128| far.max(next)));
                true
            }
        };

    let span = i128::from(widest - floor + 1);
    // The folded rule has no shortfall to step on, and the truncated rule only
    // gains by stepping when the shortfall steps less often than once a rank.
    let by_shortfall = match decay {
        DecayRule::ReciprocalRank { .. } => {
            let shortfall = SCALE_RAW - weight;
            let settled = shortfall_at(shortfall, widest + 1);
            let steps = shortfall_at(shortfall, floor) - settled;
            (steps < span).then_some((shortfall, settled))
        }
        DecayRule::WeightedReciprocalRank { .. } => None,
    };
    let Some((shortfall, settled)) = by_shortfall else {
        for cursor in floor..=widest {
            if !probe(cursor, &mut heaviest) {
                return Lap::Unreachable;
            }
        }
        return Lap::from(heaviest);
    };

    let mut cursor = floor;
    while cursor <= widest {
        let here = shortfall_at(shortfall, cursor);
        if here == settled {
            // The shortfall is flat from here to the end of the range, so every
            // remaining pair separates.
            break;
        }
        // The first denominator whose shortfall has fallen below `here`. It is
        // strictly past `cursor` — `cursor`'s own shortfall is `here` — and at
        // or before `widest + 1`, because the shortfall is `settled` there and
        // `settled` is below `here`.
        let reach = SCALE_RAW / ((here - 1) * SCALE_RAW / shortfall + 1) + 1;
        // Every pair from `cursor` up to the one before that denominator spans a
        // flat shortfall and so separates. The pair at the denominator just
        // before it is the one that steps, and the only one in the stretch that
        // has to be probed. The clamp restates the bound above rather than
        // relaxing it, and keeps the walk total.
        let stepped = u64::try_from(reach - 1).unwrap_or(widest).min(widest);
        if !probe(stepped, &mut heaviest) {
            return Lap::Unreachable;
        }
        cursor = stepped + 1;
    }
    Lap::from(heaviest)
}

/// A weight **below** which no profile separates every adjacent pair of ranks
/// with denominators in `[narrowest, widest]`, proven by counting rather than by
/// probing.
///
/// # The count
///
/// Write `F(a)` for the contribution at denominator `a`. Separating every pair
/// from `p` to `q` means `F` drops by at least one at each of the `q − p + 1`
/// steps, so `F(p) − F(q + 1) ≥ q − p + 1`. Both ends are truncations of a known
/// real quantity — `F(p) ≤ ideal(p)` and `F(q + 1) > ideal(q + 1) − 1` — so that
/// count turns into a straight inequality on the weight:
///
/// * truncated: `w·(I(p) − I(q + 1)) ≥ S·(q − p) + 1`, with `I(a) = ⌊S/a⌋`;
/// * folded: `w·(q + 1 − p) ≥ p·((q + 1)·(q − p) + 1)`.
///
/// Every `p` yields a valid bound and the strongest one is wanted, so several
/// are tried. The optimum sits near `q − sqrt(q)` — far enough down for the
/// accumulated drop to bite, near enough that the interval's own reciprocal
/// curvature has not overtaken it — and a window of a few multiples of `sqrt(q)`
/// around that point brackets it. Missing the very best `p` costs speed and
/// nothing else: the search that follows is exact from **any** valid lower
/// bound, so this is an accelerator and never an answer.
///
/// That matters at depth. Near the truncated rule's saturation point the true
/// minimum sits within a part in twenty thousand of a weight of one, and walking
/// up to it from a single raw unit takes millions of candidate steps. The bound
/// removes the great majority of them without probing a single rank.
fn heaviest_counting_bound(decay: DecayRule, narrowest: u64, widest: u64) -> i128 {
    // `sqrt` of a quantity under 2^33, so the window is under a million wide
    // even at the deepest expressible plan. The root is found with the same
    // bisection the rest of the file uses rather than by a cast.
    let root = largest_rank_satisfying(|probe| {
        probe
            .checked_mul(probe)
            .is_some_and(|square| square <= widest)
    });
    let from = widest.saturating_sub(root * 8 + 16).max(narrowest);
    let beyond = i128::from(widest) + 1;
    let outermost = SCALE_RAW / beyond;
    let mut bound: i128 = 1;
    for p in from..widest {
        let steps = i128::from(widest - p);
        let narrow = i128::from(p);
        let candidate = match decay {
            DecayRule::ReciprocalRank { .. } => {
                let gap = SCALE_RAW / narrow - outermost;
                if gap <= 0 {
                    continue;
                }
                SCALE_RAW * steps / gap + 1
            }
            DecayRule::WeightedReciprocalRank { .. } => {
                let numerator = narrow * (beyond * steps + 1);
                let divisor = beyond - narrow;
                (numerator + divisor - 1) / divisor
            }
        };
        bound = bound.max(candidate);
    }
    bound
}

/// The smallest weight whose contributions still separate every adjacent pair
/// of ranks up to `depth` under `decay`.
///
/// # Errors
///
/// * [`FusionError::InvalidK`] when the rule's smoothing constant is zero. That
///   is a malformed law rather than a request this rule cannot meet, so it is
///   refused first, before the depth is read at all — see [`usable_k`]. The
///   three refusals below are the rule declining a *depth*; this one says there
///   was no rule to decline it.
/// * [`FusionError::InvalidRank`] when `depth == 0`. A depth counts 1-based
///   ranks read from rank one, so zero names no rank and there is nothing for a
///   weight to separate; answering the lightest weight there is would be a
///   vacuous answer dressed as a real one.
/// * [`FusionError::DepthBeyondPlanRange`] when `depth` exceeds [`MAX_DEPTH`],
///   the deepest depth a plan's `u32` depth field can carry. This is a property
///   of that encoding and **not** of the rule: under
///   [`DecayRule::WeightedReciprocalRank`] a heavy enough weight separates ranks
///   well past this point, and the refusal says so rather than claiming the
///   arithmetic gave out.
/// * [`FusionError::DepthUnreachable`] when **no** weight achieves a `depth`
///   that is inside that range, which under [`DecayRule::ReciprocalRank`] is a
///   real condition rather than a conservative one. That rule truncates the
///   reciprocal *before* applying the weight: once
///   `trunc(S / D) == trunc(S / (D + 1))` the two ranks are equal at the point
///   the weight is applied, so every weight maps them to one value and the depth
///   is unreachable by construction. The saturation rank is therefore exact, and
///   is reported.
///
///   [`DecayRule::WeightedReciprocalRank`] cannot raise it at all, and that is
///   settled by reading the two places it is raised rather than by trusting the
///   shape of the arithmetic. [`Lap::Unreachable`] descends only from
///   [`Separation::Impossible`], which [`separation_at`] returns from the
///   truncated arm alone — the folded arm has no branch that can produce it. The
///   `separates_to < depth` guard is the other, and under the folded rule
///   `sufficient` is `(K + depth)²`: with `w = (K + depth)²` and any denominator
///   `a` in `[K + 1, K + depth]`, the quotient `q = ⌊w/a⌋` is at least `K + depth`
///   and hence at least `a`, while the remainder `s = w mod a` is below `a`, so
///   `s < q` — which is exactly this rule's separation condition. Every pair in
///   the range is therefore distinct at `sufficient`, `separates_to` exceeds
///   `depth`, and the guard does not fire. Both statements hold only because
///   `K >= 1` is enforced at the top of this function; a zero constant would
///   reach the guard with a measured depth of one and report saturation for a
///   rule that was never evaluated.
/// * [`FusionError::Overflow`] is carried by the signature but cannot be
///   provoked once `depth` is inside the range a plan can record: the
///   sufficient weight is then at most `(u32::MAX + u32::MAX)²`, far inside an
///   `i128`. The checked steps that would raise it are kept so that widening
///   that range cannot silently wrap instead.
///
/// # Why it is not bisected
///
/// "Does this weight reach `depth`" is **not** monotone in the weight, so a
/// binary search over it is unsound and silently over-reports. Under the folded
/// rule at `K = 60` the raw weight 62 reaches depth two and 63 does not; under
/// the truncated rule 62 does and 63 does not. The predicate flips on single raw
/// units all the way up, so bisection lands on whichever side of an oscillation
/// its probes happened to sample — at depth two under the truncated rule it
/// answers 2868 against a true minimum of 62, a factor of forty-six. Weights are
/// read as *ratios*, so over-quoting one silently re-scales that stratum's share
/// of every fused score: an over-estimate here is a wrong answer, not a safe one.
///
/// # How the true minimum is found instead
///
/// Reaching `depth` is the conjunction of one constraint per adjacent rank pair,
/// and [`separation_at`] answers each one in a couple of divisions — and when it
/// fails, names the next weight that could possibly satisfy *that* constraint,
/// skipping only weights it has ruled out. So the search walks:
///
/// 1. Start at [`heaviest_counting_bound`], the heaviest weight proven too light
///    without probing anything.
/// 2. Sweep the whole constraint range once. The closed-form guarantee
///    ([`truncated_guarantee`], [`weighted_guarantee`]) already settles every
///    rank below its boundary, so only the tail above it is probed. A lap with
///    nothing failing is the answer.
/// 3. Otherwise advance to the **heaviest** candidate the lap named. Every
///    solution above the current weight satisfies every failing constraint, so
///    it is at or above each of their candidates, and therefore at or above
///    their maximum. Taking the first candidate instead would be equally
///    correct and far slower: the strides would be whichever constraint the
///    sweep met first rather than the longest one the whole lap proved.
///
/// Because neither the bound nor any candidate ever skips a satisfying weight,
/// the first weight that completes a lap is the global minimum.
///
/// # What it costs
///
/// Three things bound the work, and none of them is the range of weights. The
/// walk starts at the counting bound rather than at a single raw unit, which
/// removes the great majority of the candidate steps. A lap probes only the tail
/// the closed-form guarantee leaves open, and that tail shrinks as the weight
/// grows. And under the truncated rule the lap can skip whole stretches of that
/// tail at a time — see [`sweep`].
///
/// Ordinary depths therefore settle in a few laps over a few ranks each. The
/// expensive case is the deepest depth the truncated rule can express: its
/// answer sits within a part in twenty thousand of a weight of one, so the
/// candidate steps between the bound and it are correspondingly fine and there
/// are a great many of them.
pub(crate) fn minimum_weight_for_depth(decay: DecayRule, depth: u64) -> Result<Fixed, FusionError> {
    // The law before the question. A rule with a zero smoothing constant is not
    // a rule to price a depth under, and the two ways this reads without the
    // check are both wrong in the same direction: a depth of one takes the
    // short-circuit below and answers the lightest weight there is — a real
    // price for a law that does not exist — while a deeper one walks into the
    // search and comes back saying the *rule saturated at depth one*, which
    // tells a story about arithmetic that was never run.
    let k = usable_k(decay)?;
    // Zero is not a depth. The `u64` signature admits it, and answering it with
    // the lightest weight there is would be a vacuous answer wearing the shape
    // of a real one: nothing was separated, because no rank was named.
    if depth == 0 {
        return Err(FusionError::InvalidRank { rank: depth });
    }
    // Deeper than any plan can record. This refusal is about the depth encoding
    // a plan carries and nothing else — it is *not* the decay rule giving out,
    // and under the folded rule the arithmetic below would in fact answer — so
    // it is its own refusal, naming that encoding as the limit.
    if depth > MAX_DEPTH {
        return Err(FusionError::DepthBeyondPlanRange {
            depth,
            limit: MAX_DEPTH,
        });
    }

    // The lightest weight there is. With fewer than two ranks to order there is
    // no adjacent pair and therefore no constraint, so it is already the answer.
    let lightest = Fixed::from_raw(1);
    let Some(constraints) = depth.checked_sub(1).filter(|count| *count > 0) else {
        return Ok(lightest);
    };

    // A weight known to reach `depth`, which bounds the walk. Under the folded
    // rule constraint `a` fails only on `w ∈ [m·(a + 1), (m + 1)·a − 1]` for some
    // `m ≤ a − 1`, so the heaviest failing weight is `a² − 1` and `(K + depth)²`
    // clears every constraint in the range at once. Under the truncated rule the
    // inner truncation caps what any weight can buy: at a weight of exactly one
    // the contribution is `⌊S/a⌋` itself, which separates wherever the bare
    // reciprocal does, and no heavier weight can recover a distinction the
    // reciprocal has already lost — so if one does not reach `depth`, nothing
    // does.
    let sufficient = match decay {
        DecayRule::ReciprocalRank { .. } => SCALE_RAW,
        DecayRule::WeightedReciprocalRank { k } => {
            let widest = u128::from(k)
                .checked_add(u128::from(depth))
                .ok_or(FusionError::Overflow)?;
            let square = widest.checked_mul(widest).ok_or(FusionError::Overflow)?;
            i128::try_from(square).map_err(|_| FusionError::Overflow)?
        }
    };
    // How deep the rule separates at all, measured once. `depth` is inside the
    // plan's range, so a value below it is a real rank the rule reached and
    // stopped at — never the saturating case, which is only ever at or above
    // `MAX_DEPTH`. That is what makes it honest to report as an exact number.
    let separates_to = monotone_depth(decay, Fixed::from_raw(sufficient))?;
    let unreachable = || FusionError::DepthUnreachable {
        depth,
        saturates_at: separates_to,
    };
    if separates_to < depth {
        return Err(unreachable());
    }

    // `depth` is at or below `MAX_DEPTH` here — the range check above refuses
    // anything deeper — so the widest denominator is under 2^33 and the sweep's
    // cursor arithmetic is exact.
    let narrowest = u64::from(k) + 1;
    let widest = u64::from(k) + constraints;

    // Ranks the closed-form guarantee already settles need no probe at all, and
    // as the weight grows that becomes almost the whole range. The guarantee is
    // non-decreasing in the weight, so a value read at a lighter weight stays
    // true at a heavier one — it only leaves a longer tail than it has to. It is
    // therefore re-read when the weight has doubled rather than every lap: at
    // most a handful of times over a walk, instead of once per candidate.
    let settled = |weight: i128| {
        let magnitude = weight.unsigned_abs();
        match decay {
            DecayRule::ReciprocalRank { .. } => truncated_guarantee(magnitude, k),
            DecayRule::WeightedReciprocalRank { .. } => weighted_guarantee(magnitude, k),
        }
    };

    let mut weight = heaviest_counting_bound(decay, narrowest, widest).max(1);
    let mut guaranteed = settled(weight);
    let mut stale_above = weight.saturating_mul(2);
    while weight <= sufficient {
        if weight >= stale_above {
            guaranteed = settled(weight);
            stale_above = weight.saturating_mul(2);
        }
        if guaranteed >= constraints {
            return Ok(Fixed::from_raw(weight));
        }
        // The tail the guarantee leaves open: ranks `guaranteed + 1 ..=
        // constraints`, which are the denominators below `widest` by that many.
        let floor = widest - (constraints - guaranteed) + 1;
        match sweep(decay, weight, floor, widest) {
            Lap::Separated => return Ok(Fixed::from_raw(weight)),
            // Retained rather than reachable. Under the truncated rule a lap
            // reports this only where `trunc(S / a) == trunc(S / (a + 1))`, and
            // that is exactly where a unit weight — which is `sufficient` for
            // that rule — already collides, so `separates_to` would have been
            // below `depth` and the guard above would have refused first. The
            // arm is kept so that changing `sufficient`, the guard, or the
            // sweep's range cannot turn a genuine saturation into a wrong
            // answer, and it names the same exact rank the guard would have.
            Lap::Unreachable => return Err(unreachable()),
            Lap::Advance(next) => {
                debug_assert!(next > weight, "a candidate must advance the search");
                weight = next;
            }
        }
    }
    // Unreachable: `sufficient` satisfies every constraint in the range and no
    // step skips a satisfying weight, so the lap completes at or below it. The
    // bound is honoured rather than asserted, and `sufficient` is a correct if
    // not minimal answer, so the walk cannot run away.
    Ok(Fixed::from_raw(sufficient))
}

/// The largest rank in `0..=MAX_DEPTH` for which `guarantees` holds, or zero
/// when it holds for none.
///
/// Every predicate handed here is non-increasing in the rank, so the search is
/// exact rather than a heuristic jump.
///
/// This is the infallible spelling, for a predicate that is pure integer
/// algebra and has no operand it can refuse. A predicate that probes the decay
/// rule's own arithmetic uses [`largest_rank_satisfying_checked`] instead, so
/// that a refusal travels rather than being read as a rank that failed the
/// test.
fn largest_rank_satisfying(guarantees: impl Fn(u64) -> bool) -> u64 {
    match largest_rank_satisfying_checked(|rank| Ok::<bool, Infallible>(guarantees(rank))) {
        Ok(rank) => rank,
        // [`Infallible`] has no values, so this arm is a proof rather than a
        // branch: the predicate above cannot refuse, and the compiler checks it.
        Err(never) => match never {},
    }
}

/// The same bisection over a predicate that may refuse a probe.
///
/// A refusal is **not** the same fact as "this rank does not satisfy the
/// predicate". Reading it as one would move the boundary — downwards for the
/// class's last rank, upwards for its first — and hand back a width that was
/// never measured. So the first refusal ends the search and is returned.
///
/// # Errors
///
/// Whatever `guarantees` refuses, at the first probe that refuses it.
fn largest_rank_satisfying_checked<E>(
    guarantees: impl Fn(u64) -> Result<bool, E>,
) -> Result<u64, E> {
    let mut low = 0_u64;
    let mut high = MAX_DEPTH;
    while low < high {
        let mid = low + (high - low).div_ceil(2);
        if guarantees(mid)? {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    Ok(low)
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
        ClassWidth, MAX_DEPTH, MonotoneDepth, ToleratedDepth, class_width, contribution,
        contribution_under, deepest_rank_within_width, first_collision, minimum_weight_for_depth,
        monotone_depth, shortfall_at, weighted_contribution, weighted_guarantee,
    };
    use crate::error::FusionError;
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

    /// The separation bound, with the decay rule's refusal surfaced as a panic.
    ///
    /// Every fixture that reads a bound below names a usable smoothing constant
    /// and a positive weight, so a refusal here would be a broken fixture rather
    /// than a case under test. The refusal itself is exercised directly against
    /// [`first_collision`], which is where it is raised.
    fn measured_bound(decay: DecayRule, weight: Fixed) -> u64 {
        monotone_depth(decay, weight)
            .expect("a usable smoothing constant and a positive weight measure a bound")
    }

    /// The property the bound is defined by, asserted on both sides of it: every
    /// adjacent pair at or below the bound is strictly ordered, and the pair
    /// immediately beyond it is not.
    fn assert_bound_is_exact(weight: Fixed, k: u32) {
        let bound = measured_bound(truncated(k), weight);
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
        let unit = measured_bound(truncated(60), Fixed::ONE);
        for raw in [SCALE, SCALE * 2, SCALE * 1_000, SCALE * 1_000_000] {
            assert_eq!(
                measured_bound(truncated(60), Fixed::from_raw(raw)),
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
        let unit = measured_bound(folded(60), Fixed::ONE);
        let mut previous = unit;
        for weight_units in [4_i128, 100, 10_000] {
            let bound = measured_bound(folded(60), Fixed::from_raw(weight_units * SCALE));
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
            let mut previous = measured_bound(decay, Fixed::ONE);
            for raw in [SCALE / 100, SCALE / 10_000, SCALE / 1_000_000, 100, 1] {
                let bound = measured_bound(decay, Fixed::from_raw(raw));
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
        assert_eq!(measured_bound(truncated(60), weight), 1);
        assert_eq!(value(weight, 1, 60), value(weight, 2, 60));
        assert_eq!(measured_bound(folded(60), weight), 1);
        assert_eq!(weighted(weight, 1, 60), weighted(weight, 2, 60));
    }

    /// The smoothing constant is the third quantity in the coupling, and it
    /// shifts the bound by exactly what it adds to the denominator — under both
    /// rules, because both divide by `K + r`.
    #[test]
    fn the_smoothing_constant_shifts_the_bound_one_for_one() {
        let weight = Fixed::from_raw(1_000_000);
        for rule in [truncated as fn(u32) -> DecayRule, folded] {
            let base = measured_bound(rule(1), weight);
            for k in [2_u32, 10, 60, 500] {
                assert_eq!(
                    measured_bound(rule(k), weight) + u64::from(k),
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
                        measured_bound(decay, weight),
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

    /// The precision floor a caller choosing a weight is warned about, measured
    /// rather than asserted: a contribution is an integer count of raw units, so
    /// a weight near the raw unit loses a visible share of *every* contribution
    /// before anything is summed. At `w_raw = 1000` and `K = 60` the rank-1 value
    /// is `1000 / 61 = 16.39…` and both rules emit exactly 16 raw units. The same
    /// division at the whole-number weight `10^12` keeps eleven digits more,
    /// which is the two anchors
    /// [`FusionProfile::with_decay`](crate::FusionProfile::with_decay) quotes
    /// for the relation `(K + rank) / w_raw`.
    #[test]
    fn a_weight_near_the_raw_unit_truncates_a_measurable_share_of_each_contribution() {
        let small = Fixed::from_raw(1_000);
        let emitted = weighted(small, 1, 60).into_raw();
        assert_eq!(emitted, 16, "1000 / 61 = 16.39… truncates to 16 raw units");
        assert_eq!(
            value(small, 1, 60).into_raw(),
            emitted,
            "the truncated rule lands on the same 16 at this weight"
        );

        // The shortfall exactly, in thousandths of a raw unit, so the ~2.4% the
        // docs quote is a measurement and not a rounding: 1000·1000/61 = 16393
        // thousandths wanted, 16000 emitted, 393 lost.
        let wanted_thousandths = 1_000_i128 * 1_000 / 61;
        assert_eq!(wanted_thousandths, 16_393);
        assert_eq!(wanted_thousandths - emitted * 1_000, 393);

        // And the loss is bounded by one raw unit, which is the `(K + rank) /
        // w_raw` relation: the emitted value is the floor of the exact quotient.
        assert!(emitted * 61 <= 1_000 && (emitted + 1) * 61 > 1_000);

        // The other anchor. A whole-number weight keeps the same one-unit loss
        // against a contribution ten orders of magnitude larger.
        let whole = Fixed::from_raw(SCALE);
        let whole_emitted = weighted(whole, 1, 60).into_raw();
        assert_eq!(whole_emitted, 16_393_442_622);
        assert_eq!(whole_emitted, SCALE / 61);
        assert!(whole_emitted * 61 <= SCALE && (whole_emitted + 1) * 61 > SCALE);
    }

    /// A negative weight truncates toward zero under the weighted rule, the same
    /// direction every other rounding in the layer takes.
    #[test]
    fn a_negative_weight_truncates_toward_zero() {
        let negative = Fixed::from_raw(-7);
        assert_eq!(weighted(negative, 1, 1).into_raw(), -3);
        assert_eq!(weighted(Fixed::from_raw(7), 1, 1).into_raw(), 3);
    }

    /// Non-increase is a property of the decay rule, not a per-row observation.
    ///
    /// Fusion needs contributions to be non-increasing with rank: the threshold
    /// over the stream heads is an upper bound only while they are, and
    /// certification would otherwise be unsound. The engine's one enforcement of
    /// that is the re-derivation — a row's contribution must equal
    /// `contribution_under(decay, weight, rank)` for its stratum's weight, at a
    /// rank already held contiguous and ascending, or it is refused as
    /// [`ProtocolError::ContributionMismatch`](crate::ProtocolError::ContributionMismatch).
    /// Which makes non-increase entirely a property of *this* function, provable
    /// only here, at its source, rather than sampled one adjacent pair at a time
    /// by whatever streams happen to be fused.
    ///
    /// The weights below are all strictly positive, which is the whole domain: a
    /// [`FusionProfile`](crate::FusionProfile) refuses a weight at or below zero
    /// at construction and at decode, so no weight this function sees in fusion
    /// is outside it. (A negative weight does rise with rank — the magnitude
    /// shrinks toward zero from below — which is exactly why the profile refuses
    /// one rather than leaving the fusion loop to discover it.)
    ///
    /// Both rules, across the full span where the arithmetic changes character:
    /// below the collision bound, across it, and far past it where every pair is
    /// a plateau.
    #[test]
    fn every_decay_rule_is_non_increasing_in_the_rank() {
        let weights = [
            Fixed::from_raw(1),
            Fixed::from_raw(1_000),
            Fixed::from_raw(1_000_000),
            Fixed::from_raw(SCALE),
        ];
        for decay in [truncated(60), folded(60), truncated(1), folded(1)] {
            for weight in weights {
                let mut previous = contribution_under(decay, weight, 1)
                    .expect("a positive weight at rank one does not overflow");
                for rank in 2..4_000 {
                    let current = contribution_under(decay, weight, rank)
                        .expect("a positive weight at a 1-based rank does not overflow");
                    assert!(
                        current <= previous,
                        "{decay:?} at weight {weight:?} rose from {previous:?} to {current:?} \
                         between ranks {} and {rank}",
                        rank - 1
                    );
                    previous = current;
                }
            }
        }
    }

    /// The identity the truncated rule's sweep skips on, asserted rather than
    /// trusted: `⌊w·I/S⌋ = I − ⌈(S − w)·I/S⌉` exactly, for every weight the
    /// search can hold and across the whole span of denominators.
    ///
    /// Everything the skip claims rests on this. If it ever stopped holding, a
    /// flat shortfall would no longer mean a separated stretch of ranks and the
    /// sweep would step straight past a collision.
    #[test]
    fn the_shortfall_is_exactly_what_the_contribution_loses() {
        for weight_raw in [1_i128, 2, 1_000, 999_983, SCALE / 2, SCALE - 1, SCALE] {
            let shortfall = SCALE - weight_raw;
            for denominator in [2_u64, 3, 61, 1_000, 999_983, 1_000_000, 1_048_577] {
                let reciprocal = SCALE / i128::from(denominator);
                assert_eq!(
                    weight_raw * reciprocal / SCALE,
                    reciprocal - shortfall_at(shortfall, denominator),
                    "weight raw {weight_raw} at denominator {denominator}"
                );
            }
        }
    }

    /// A flat shortfall across a stretch of denominators means every adjacent
    /// pair in that stretch separates — the lemma the sweep skips whole blocks
    /// on, checked directly against the rule's own arithmetic.
    #[test]
    fn a_flat_shortfall_spans_only_separated_ranks() {
        let k = 60_u32;
        // The third weight is the true minimum for the truncated rule's deepest
        // expressible depth, and its shortfall only flattens out where that
        // depth actually binds — so each fixture is walked where its own
        // arithmetic is interesting rather than all of them from rank one.
        for (weight_raw, from) in [
            (SCALE - 1, 1_u64),
            (SCALE - 97, 1),
            (SCALE - 44_559_030, 700_000),
        ] {
            let weight = Fixed::from_raw(weight_raw);
            let shortfall = SCALE - weight_raw;
            let mut flat = 0_u64;
            for rank in from..from + 4_000 {
                let here = shortfall_at(shortfall, u64::from(k) + rank);
                let next = shortfall_at(shortfall, u64::from(k) + rank + 1);
                if here != next {
                    continue;
                }
                flat += 1;
                assert!(
                    value(weight, rank, k) > value(weight, rank + 1, k),
                    "weight raw {weight_raw}: ranks {rank} and {} share a flat shortfall \
                     and must therefore separate",
                    rank + 1
                );
            }
            assert!(flat > 0, "weight raw {weight_raw} must exercise the lemma");
        }
    }

    /// The minimum weight, checked against an exhaustive search over every
    /// lighter weight, at smoothing constants and depths small enough to walk.
    ///
    /// Both rules, and both of the sweeps the truncated rule chooses between —
    /// the accelerators are only allowed to change how long the answer takes.
    #[test]
    fn the_minimum_weight_agrees_with_an_exhaustive_search() {
        for k in [1_u32, 2, 7, 60] {
            for depth in [2_u64, 3, 5, 9, 17, 40] {
                for decay in [
                    DecayRule::ReciprocalRank { k },
                    DecayRule::WeightedReciprocalRank { k },
                ] {
                    let named = minimum_weight_for_depth(decay, depth)
                        .expect("these depths are reachable under both rules");
                    let expected = (1..=named.into_raw())
                        .find(|raw| measured_bound(decay, Fixed::from_raw(*raw)) >= depth)
                        .expect("the named weight itself reaches the depth");
                    assert_eq!(
                        named.into_raw(),
                        expected,
                        "{decay:?} depth {depth}: the lightest weight that reaches it is \
                         {expected}, not {}",
                        named.into_raw()
                    );
                }
            }
        }
    }

    /// A depth of one has no adjacent pair to separate, so the lightest
    /// representable weight already answers it — while a depth of none names no
    /// rank at all and is refused rather than answered vacuously. The two sit in
    /// one test because they are one step apart and the refusal is only honest
    /// if its neighbour still succeeds.
    #[test]
    fn a_depth_of_one_costs_the_lightest_weight_and_a_depth_of_none_is_refused() {
        for decay in [
            DecayRule::ReciprocalRank { k: 60 },
            DecayRule::WeightedReciprocalRank { k: 60 },
        ] {
            assert_eq!(
                minimum_weight_for_depth(decay, 1).expect("nothing to separate"),
                Fixed::from_raw(1),
                "{decay:?} depth 1"
            );
            assert!(
                matches!(
                    minimum_weight_for_depth(decay, 0),
                    Err(FusionError::InvalidRank { rank: 0 })
                ),
                "{decay:?}: a depth of zero names no rank and must be refused"
            );
        }
    }

    /// **A zero smoothing constant is a malformed law, and all three resolution
    /// questions say so — including the two that answer from a short-circuit
    /// without ever touching the rule's arithmetic.**
    ///
    /// The hole this closes was not that the constant went unchecked
    /// everywhere. It was that whether it was checked at all depended on the
    /// *other* argument. A depth of one has no adjacent pair to separate and a
    /// tolerance of one does no walking, so both returned before a denominator
    /// was formed and quoted a real number under a rule that cannot be
    /// evaluated. One step further along either axis did refuse — but refused
    /// with [`FusionError::DepthUnreachable`], reporting that the decay rule had
    /// separated to depth one and no further. It had not separated anything. It
    /// was never a rule.
    ///
    /// So both halves are asserted: the variant, and the sentence a caller
    /// actually reads. A refusal that names the wrong cause sends the caller to
    /// the wrong remedy — saturation's remedy is the other decay rule, and
    /// switching rules does not make a zero constant usable.
    ///
    /// Every case is executed at the neighbouring constant of one, the smallest
    /// usable law there is, which must still answer.
    #[test]
    fn a_zero_smoothing_constant_is_refused_as_a_malformed_law_on_every_path() {
        let weight = Fixed::from_raw(1_000_000);
        for rule in [truncated as fn(u32) -> DecayRule, folded] {
            // Read a refusal both ways: the variant it carries, and the story it
            // tells. `1` is the short-circuiting argument on both axes and the
            // larger values walk.
            let refuses_as_malformed_law = |what: &str, argument: u64, got: &FusionError| {
                assert!(
                    matches!(got, FusionError::InvalidK { k: 0 }),
                    "{:?} {what} {argument}: a zero constant is a malformed law, got {got:?}",
                    rule(0)
                );
                let told = got.to_string();
                assert!(
                    told.contains("K must be at least 1"),
                    "{:?} {what} {argument}: the message must name the constant, got {told:?}",
                    rule(0)
                );
                assert!(
                    !told.contains("no further"),
                    "{:?} {what} {argument}: this is not the decay rule saturating, got {told:?}",
                    rule(0)
                );
            };

            for depth in [1_u64, 2, 64] {
                let refused = minimum_weight_for_depth(rule(0), depth)
                    .expect_err("a zero constant prices no depth");
                refuses_as_malformed_law("depth", depth, &refused);
                assert!(
                    minimum_weight_for_depth(rule(1), depth).is_ok(),
                    "{:?} depth {depth}: the smallest usable constant still prices it",
                    rule(1)
                );
            }

            for rank in [1_u64, 2, 64] {
                let refused =
                    class_width(rule(0), weight, rank).expect_err("a zero constant has no width");
                refuses_as_malformed_law("rank", rank, &refused);
                assert_eq!(
                    class_width(rule(1), weight, rank)
                        .expect("the smallest usable constant measures a width"),
                    ClassWidth::SpansTo(1),
                    "{:?} rank {rank}: this weight separates it from both neighbours",
                    rule(1)
                );
            }

            for tolerance in [1_u64, 2, 64] {
                let refused = deepest_rank_within_width(rule(0), weight, tolerance)
                    .expect_err("a zero constant reads to no depth");
                refuses_as_malformed_law("tolerance", tolerance, &refused);
                assert!(
                    deepest_rank_within_width(rule(1), weight, tolerance)
                        .expect("the smallest usable constant measures a depth")
                        .covers(2),
                    "{:?} tolerance {tolerance}: the smallest usable constant reads past \
                     a single rank",
                    rule(1)
                );
            }
        }
    }

    /// Both resolution measurements hand back the decay rule's own refusal
    /// rather than a number, and the numbers they would otherwise have handed
    /// back are the two most flattering ones there are: a width of one is
    /// "perfectly separated from both neighbours", and a depth is a resolution
    /// the profile was measured to deliver.
    ///
    /// A smoothing constant of zero is the operand that reaches both walks.
    /// `FusionProfile::with_decay` refuses one, so this is the lowest level a
    /// caller can still present it at; the neighbouring constant of one is
    /// asserted in the same test, because a refusal is only honest if its
    /// neighbour still answers.
    #[test]
    fn an_unusable_smoothing_constant_is_propagated_rather_than_rendered_as_a_measurement() {
        let weight = Fixed::from_raw(1_000_000);
        assert!(
            matches!(
                class_width(truncated(0), weight, 4),
                Err(FusionError::InvalidK { k: 0 })
            ),
            "a width measured from an unusable constant is no width at all"
        );
        assert!(
            matches!(
                deepest_rank_within_width(truncated(0), weight, 4),
                Err(FusionError::InvalidK { k: 0 })
            ),
            "and the rank the walk stopped at is not a depth it measured"
        );

        assert_eq!(
            class_width(truncated(1), weight, 4).expect("a constant of one is usable"),
            ClassWidth::SpansTo(1),
            "rank four is inside the separating range at this weight"
        );
        assert!(
            deepest_rank_within_width(truncated(1), weight, 4)
                .expect("a constant of one is usable")
                .covers(2),
            "and tolerating a class of four reads deeper than a single rank"
        );
    }

    /// The cost `deepest_rank_within_width` documents, checked rather than
    /// asserted in prose: the answer is about `sqrt(max_width)` times the
    /// separating depth, so the walk behind it is about `sqrt(max_width) - 1`
    /// times that depth.
    ///
    /// A caller's only handle on what this call costs is that relation, and a
    /// documented cost nothing measures is a claim like any other. The
    /// tolerances here are perfect squares so the expected ratio is exact, and
    /// the assertion allows one part in a thousand for the straddling floors —
    /// tight enough that a walk with a different shape fails it, loose enough
    /// that the exact first-collision rank may move within a class.
    #[test]
    fn the_tolerance_walk_lands_near_the_square_root_of_the_tolerance() {
        for decay in [truncated(60), folded(60)] {
            let weight = Fixed::from_raw(SCALE);
            let separating = measured_bound(decay, weight);
            assert_eq!(
                deepest_rank_within_width(decay, weight, 1).expect("a unit weight measures"),
                ToleratedDepth::ReadsTo(separating),
                "{decay:?}: a tolerance of one is the separating depth and walks not at all"
            );
            for (tolerance, root) in [(4_u64, 2_u64), (16, 4), (64, 8)] {
                let depth = deepest_rank_within_width(decay, weight, tolerance)
                    .expect("a unit weight measures")
                    .rank()
                    .expect("a unit weight's tolerated depth is inside a plan's range");
                let expected = separating * root;
                let slack = expected / 1_000;
                assert!(
                    depth.abs_diff(expected) <= slack,
                    "{decay:?}: a tolerance of {tolerance} should land near {expected}, \
                     {root} times the separating depth {separating}, and landed at {depth}"
                );
            }
        }
    }

    /// The ceiling a plan's depth encoding sets is a wall, and the tolerance
    /// walk reports it as one rather than as a depth it measured.
    ///
    /// Two weights fifty times apart both run to that ceiling. A bare
    /// `u32::MAX` from each says they reach the same depth — a number a caller
    /// can log, plot or divide by — where the only true statement is that
    /// neither has a bound inside any plan's reach. The neighbouring weight
    /// just below the wall is asserted in the same test, because a saturating
    /// case is only honest if its neighbour still reports a real depth.
    ///
    /// The other two entry points over this file are asserted here too, at the
    /// identical boundary. They agree by law rather than by which one was
    /// called: a *measured* range that reaches the ceiling is the saturating
    /// case of a typed quantity, and a *requested* depth past it is refused as
    /// a limit of the depth encoding a plan carries.
    #[test]
    fn a_tolerated_depth_that_saturates_is_reported_as_saturation_and_not_as_a_number() {
        let decay = folded(60);

        for weight in [
            Fixed::from_raw(20_000_000 * SCALE),
            Fixed::from_raw(1_000_000_000 * SCALE),
        ] {
            assert_eq!(
                deepest_rank_within_width(decay, weight, 1).expect("the arithmetic evaluates"),
                ToleratedDepth::ReadsBeyondAnyPlan,
                "{decay:?}: a weight that outruns every expressible depth has no bound to \
                 report, not an enormous one"
            );
            assert_eq!(
                MonotoneDepth::from_rank(
                    monotone_depth(decay, weight).expect("the arithmetic evaluates")
                ),
                MonotoneDepth::SeparatesBeyondAnyPlan,
                "{decay:?}: and the separation question hits the same wall and spells it the \
                 same way"
            );
        }

        // The neighbouring weight, below the wall: a measured depth, reported
        // as one. Two million raw units lighter than the first saturating
        // weight above, and the answer changes shape rather than degree.
        let below = Fixed::from_raw(18_000_000 * SCALE);
        let bound = monotone_depth(decay, below).expect("the arithmetic evaluates");
        assert!(
            bound < MAX_DEPTH,
            "the neighbouring case must be inside the expressible range, or it proves nothing"
        );
        assert_eq!(
            deepest_rank_within_width(decay, below, 1).expect("the arithmetic evaluates"),
            ToleratedDepth::ReadsTo(bound),
            "{decay:?}: a bound inside a plan's range is a measurement and is reported as one"
        );

        // The walk's own saturating exit, not only the short-circuit above: the
        // smallest weight that separates to one rank short of the ceiling,
        // walked at a tolerance of two, steps off the end of the expressible
        // range instead of stopping inside it.
        let at_the_edge = minimum_weight_for_depth(decay, MAX_DEPTH - 1)
            .expect("the folded rule prices the deepest expressible depths");
        assert_eq!(
            monotone_depth(decay, at_the_edge).expect("the arithmetic evaluates"),
            MAX_DEPTH - 1,
            "the minimum weight for a depth separates to exactly that depth"
        );
        assert_eq!(
            deepest_rank_within_width(decay, at_the_edge, 2).expect("the arithmetic evaluates"),
            ToleratedDepth::ReadsBeyondAnyPlan,
            "a walk that runs to the ceiling found no bound a plan could carry"
        );

        // And the entry point that is handed a *requested* depth rather than
        // measuring one refuses at the same boundary, as a limit of the depth
        // encoding a plan carries — with the neighbouring in-range depth still
        // priced, so the refusal is about the depth and nothing else.
        assert!(
            matches!(
                minimum_weight_for_depth(decay, MAX_DEPTH + 1),
                Err(FusionError::DepthBeyondPlanRange {
                    limit: MAX_DEPTH,
                    ..
                })
            ),
            "a requested depth past the wall is a range refusal, never a saturated measurement"
        );
        assert!(
            minimum_weight_for_depth(decay, MAX_DEPTH).is_ok(),
            "and the deepest expressible depth is still priced"
        );
    }

    /// A tolerance of zero is refused rather than read as the narrowest real
    /// one, and the neighbouring tolerance of one still answers.
    ///
    /// A class always contains its own rank, so a tolerance of zero describes
    /// no class at all — the same shape as a smoothing constant of zero
    /// describing no law. Normalising it up to one answered the narrowest real
    /// tolerance in its place, which is the deepest fully-separated depth this
    /// algebra reports: the most favourable answer available, returned
    /// precisely where nothing was asked.
    #[test]
    fn a_tolerance_of_zero_is_refused_rather_than_read_as_the_narrowest_real_one() {
        let weight = Fixed::from_raw(1_000);
        for decay in [truncated(60), folded(60)] {
            assert!(
                matches!(
                    deepest_rank_within_width(decay, weight, 0),
                    Err(FusionError::InvalidWidth { max_width: 0 })
                ),
                "{decay:?}: a tolerance of zero is not a tolerance, and it is refused as one \
                 rather than as a rank"
            );

            // The neighbouring valid case: the narrowest tolerance that does
            // describe a class, at the same weight under the same rule.
            let bound = monotone_depth(decay, weight).expect("the arithmetic evaluates");
            assert_eq!(
                deepest_rank_within_width(decay, weight, 1).expect("the arithmetic evaluates"),
                ToleratedDepth::ReadsTo(bound),
                "{decay:?}: a tolerance of one is a tolerance and reads to the separating depth"
            );
        }
    }

    /// The other operand either walk can be handed: a rank of zero, which is
    /// not a rank at all under 1-based numbering. It is refused where the class
    /// is anchored, and rank one — the neighbouring valid case — still
    /// measures.
    ///
    /// The depth walk never forms it: it starts at the monotone bound, which is
    /// at least one.
    #[test]
    fn a_rank_of_zero_is_refused_rather_than_anchoring_a_class_of_one() {
        for decay in [truncated(60), folded(60)] {
            assert!(
                matches!(
                    class_width(decay, Fixed::ONE, 0),
                    Err(FusionError::InvalidRank { rank: 0 })
                ),
                "{decay:?}: rank zero names no rank for a class to form around"
            );
            assert_eq!(
                class_width(decay, Fixed::ONE, 1).expect("rank one is a rank"),
                ClassWidth::SpansTo(1),
                "{decay:?}: rank one is separated from its neighbour at a unit weight"
            );
        }
    }

    /// The separation walk reports the decay rule's refusal rather than the rank
    /// at which the arithmetic stopped.
    ///
    /// The rank a failed probe stands on is the most flattering number the walk
    /// could name — "every adjacent pair below here is distinct" — and nothing
    /// compared a pair at all. Both unusable operands are exercised at the walk
    /// itself, because [`monotone_depth`] short-circuits them before the walk
    /// runs; each is paired with the neighbouring usable operand, which must
    /// still measure a real bound.
    #[test]
    fn the_separation_walk_propagates_an_unusable_operand_rather_than_naming_the_rank_it_failed_at()
    {
        let weight = Fixed::from_raw(1_000_000);
        assert!(
            matches!(
                first_collision(weight, 0, 1, contribution),
                Err(FusionError::InvalidK { k: 0 })
            ),
            "a bound walked under an unusable constant is no bound at all"
        );
        assert!(
            matches!(
                first_collision(weight, 60, 0, weighted_contribution),
                Err(FusionError::InvalidRank { rank: 0 })
            ),
            "and a walk anchored on a rank that is not a rank measures nothing"
        );

        assert!(
            first_collision(weight, 1, 1, contribution).expect("a constant of one is usable") > 1,
            "the neighbouring usable constant still separates more than a single rank"
        );
        assert!(
            first_collision(weight, 60, 1, weighted_contribution).expect("rank one is a rank") > 1,
            "and the neighbouring 1-based anchor still walks to a real collision"
        );
    }

    /// The two unusable operands [`monotone_depth`] can be handed are handled
    /// differently on purpose, and the difference is which of them names a
    /// measurement.
    ///
    /// A smoothing constant of zero is refused before anything is read. It
    /// describes no law, so there is nothing to measure and a depth of one would
    /// be the shape of a measurement carrying none.
    ///
    /// A weight of zero is measured and the measurement is one: every rank's
    /// contribution is zero, so the first adjacent pair already collides and one
    /// is the exact first-collision rank rather than a stand-in for it. The
    /// negative half is short-circuited alongside it — `FusionProfile::with_decay`
    /// refuses a negative weight where one is declared — so the walk is never
    /// entered with a weight whose contributions do not decrease.
    ///
    /// Each case is paired with its usable neighbour, which must still measure a
    /// deeper range.
    #[test]
    fn an_unusable_constant_is_refused_while_a_zero_weight_measures_the_single_rank_it_orders() {
        let weight = Fixed::from_raw(1_000_000);
        for rule in [truncated as fn(u32) -> DecayRule, folded] {
            let unusable_k = monotone_depth(rule(0), weight);
            assert!(
                matches!(unusable_k, Err(FusionError::InvalidK { k: 0 })),
                "an unusable smoothing constant is no law to measure under, \
                 got {unusable_k:?}"
            );
            assert!(
                measured_bound(rule(1), weight) > 1,
                "and the neighbouring constant of one still measures a deeper range"
            );

            for raw in [0_i128, -1, -1_000_000] {
                let unusable_weight = monotone_depth(rule(60), Fixed::from_raw(raw));
                assert!(
                    matches!(unusable_weight, Ok(1)),
                    "a weight of raw {raw} orders a single rank and is not walked, \
                     got {unusable_weight:?}"
                );
            }
            assert!(
                measured_bound(rule(60), weight) > 1,
                "and the neighbouring positive weight still measures a deeper range"
            );
        }
    }

    /// Every operand a validated profile can present is measured rather than
    /// refused, across the whole weight range up to the heaviest representable
    /// one and both rules.
    ///
    /// This is the other half of the refusal above: now that the walk carries
    /// the decay rule's refusal, an over-refusal would surface here as a profile
    /// that no longer builds. Nothing in this range may refuse.
    #[test]
    fn every_positive_weight_under_a_usable_constant_measures_a_bound_rather_than_refusing() {
        for k in [1_u32, 60, u32::MAX] {
            for raw in [
                1_i128,
                1_000,
                SCALE / 2,
                SCALE,
                SCALE * 1_000_000,
                i128::MAX / 2,
                i128::MAX,
            ] {
                let weight = Fixed::from_raw(raw);
                for decay in [truncated(k), folded(k)] {
                    let depth = monotone_depth(decay, weight);
                    assert!(
                        matches!(depth, Ok(rank) if rank >= 1),
                        "{decay:?} at weight raw {raw} must measure a bound, got {depth:?}"
                    );
                }
            }
        }
    }

    /// The plan's depth encoding and the rule's saturation are different facts,
    /// refused by different variants. The pair is one step apart: the deepest
    /// depth a plan can record is priced under the folded rule, and one past it
    /// is refused for the encoding rather than for the arithmetic.
    #[test]
    fn the_deepest_expressible_depth_is_priced_and_one_past_it_is_a_plan_range_refusal() {
        let folded = DecayRule::WeightedReciprocalRank { k: 60 };
        assert!(
            minimum_weight_for_depth(folded, MAX_DEPTH).is_ok(),
            "the folded rule separates to the deepest depth a plan can record"
        );
        assert!(
            matches!(
                minimum_weight_for_depth(folded, MAX_DEPTH + 1),
                Err(FusionError::DepthBeyondPlanRange {
                    limit: MAX_DEPTH,
                    ..
                })
            ),
            "one past the plan's range is a range refusal, not a saturation report"
        );
        assert!(
            matches!(
                minimum_weight_for_depth(DecayRule::ReciprocalRank { k: 60 }, MAX_DEPTH),
                Err(FusionError::DepthUnreachable { .. })
            ),
            "the truncated rule saturates well inside the plan's range"
        );
    }
}
