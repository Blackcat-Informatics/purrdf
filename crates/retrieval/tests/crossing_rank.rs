// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! **The threshold-crossing derivation refuses a weight it cannot search over,
//! and answers exactly for the neighbouring weights it can.**
//!
//! [`crossing_rank_at`] bisects over the head rank, which is exact only while
//! the threshold is non-increasing in that rank — and it is non-increasing only
//! while every weight summed into it is strictly positive. A zero or negative
//! weight is therefore refused as [`FusionError::NonPositiveCrossingWeight`],
//! carrying the weight, wherever it appears: in the list of strata that named
//! the candidate or in the list of strata that share its block.
//!
//! A refusal is a claim too, so every refused case here has a valid neighbour
//! that is executed and must answer: the same call with the offending weight
//! replaced by a positive one, down to the smallest positive raw unit. The
//! neighbours' answers are distinct from an error by construction, and the
//! principal one is pinned to its exact rank and then re-derived from
//! [`threshold_at`] — the rank at which the threshold is strictly below the
//! candidate's bound, and one rank shallower it is not — so the pin is checked
//! against the arithmetic rather than merely recorded.

use pretty_assertions::assert_eq;
use purrdf_retrieval::{
    CrossingRank, DecayRule, Fixed, FusionError, crossing_rank_at, threshold_at,
};

/// The smoothing constant every case here runs under.
const K: u32 = 60;

/// The rank the candidate sits at in the stratum that named it.
const AT_RANK: u64 = 5;

/// Both decay rules, so a refusal that held under one and not the other would
/// show.
const RULES: [DecayRule; 2] = [
    DecayRule::ReciprocalRank { k: K },
    DecayRule::WeightedReciprocalRank { k: K },
];

/// The two non-positive weights the derivation must refuse: zero, which
/// contributes nothing at any rank, and the smallest negative raw unit, which
/// makes the threshold rise with the rank.
const REFUSED: [Fixed; 2] = [Fixed::ZERO, Fixed::from_raw(-1)];

/// Assert `result` is the non-positive-weight refusal, carrying `weight`.
fn assert_refused(result: Result<CrossingRank, FusionError>, weight: Fixed, case: &str) {
    match result {
        Err(FusionError::NonPositiveCrossingWeight { weight: carried }) => assert_eq!(
            carried, weight,
            "{case}: the refusal carries the weight that was rejected"
        ),
        other => panic!("{case}: a weight of {weight:?} must be refused, got {other:?}"),
    }
}

/// A non-positive weight is refused whether the stratum carrying it named the
/// candidate or only shares its block.
#[test]
fn a_non_positive_weight_is_refused_in_either_list() {
    for rule in RULES {
        for weight in REFUSED {
            assert_refused(
                crossing_rank_at(rule, &[weight], AT_RANK, &[Fixed::ONE, Fixed::ONE]),
                weight,
                &format!("{rule:?}, naming weight"),
            );
            assert_refused(
                crossing_rank_at(rule, &[Fixed::ONE], AT_RANK, &[Fixed::ONE, weight]),
                weight,
                &format!("{rule:?}, sharing weight"),
            );
            // `threshold_at` is where the refusal is raised, and it is public
            // in its own right.
            assert!(
                matches!(
                    threshold_at(rule, &[Fixed::ONE, weight], AT_RANK),
                    Err(FusionError::NonPositiveCrossingWeight { weight: carried })
                        if carried == weight
                ),
                "{rule:?}: threshold_at refuses a weight of {weight:?}"
            );
        }
    }
}

/// The valid neighbour of every refusal: one stratum named the candidate at rank
/// five, two equal strata share its block, and the threshold has to fall to half
/// the candidate's bound. Under a smoothing constant of 60 that is the first
/// head rank past 70.
#[test]
fn the_valid_neighbour_crosses_at_its_exact_rank() {
    for rule in RULES {
        let crossing = crossing_rank_at(rule, &[Fixed::ONE], AT_RANK, &[Fixed::ONE, Fixed::ONE])
            .unwrap_or_else(|error| panic!("{rule:?}: positive weights are searched: {error}"));
        assert_eq!(crossing, CrossingRank::CrossesAt(71), "{rule:?}");

        // The pin re-derived: at 71 the threshold is strictly below the bound,
        // and at 70 it is not.
        let bound = threshold_at(rule, &[Fixed::ONE], AT_RANK).expect("in range");
        let sharing = [Fixed::ONE, Fixed::ONE];
        assert!(
            bound > threshold_at(rule, &sharing, 71).expect("in range"),
            "{rule:?}: the threshold is below the bound at the crossing"
        );
        assert!(
            bound <= threshold_at(rule, &sharing, 70).expect("in range"),
            "{rule:?}: one rank shallower the threshold has not fallen below the bound"
        );
    }
}

/// "Positive" reaches all the way down to one raw unit: a derivation that
/// refused anything it judged too small to matter would be refusing exactly the
/// low-weight strata a host tunes by hand.
///
/// A one-unit sharer's contribution truncates to nothing at these ranks, so it
/// is searched and crosses exactly where a lone sharer does. A half-weight sharer
/// is visible in the arithmetic, and lands strictly between the lone sharer and
/// the equal one — so the three neighbours are three different answers, and a
/// derivation that dropped or refused the second sharer could not produce them.
#[test]
fn small_positive_weights_are_searched_rather_than_refused() {
    let tiny = Fixed::from_raw(1);
    let half = Fixed::from_raw(Fixed::ONE.into_raw() / 2);
    for rule in RULES {
        let search = |sharing: &[Fixed]| {
            crossing_rank_at(rule, &[Fixed::ONE], AT_RANK, sharing)
                .unwrap_or_else(|error| panic!("{rule:?}: {sharing:?} is searched: {error}"))
        };
        let alone = search(&[Fixed::ONE]);
        assert_eq!(
            alone,
            CrossingRank::CrossesAt(AT_RANK + 1),
            "{rule:?}: lone sharer"
        );
        assert_eq!(
            search(&[Fixed::ONE, tiny]),
            alone,
            "{rule:?}: a one-unit sharer is accepted and contributes nothing here"
        );

        let sharing = [Fixed::ONE, half];
        let rank = search(&sharing)
            .rank()
            .unwrap_or_else(|| panic!("{rule:?}: a positive bound crosses inside the range"));
        assert!(
            AT_RANK + 1 < rank && rank < 71,
            "{rule:?}: a half-weight sharer crosses between a lone sharer and an equal one, \
             not at {rank}"
        );
        let bound = threshold_at(rule, &[Fixed::ONE], AT_RANK).expect("in range");
        assert!(
            bound > threshold_at(rule, &sharing, rank).expect("in range")
                && bound <= threshold_at(rule, &sharing, rank - 1).expect("in range"),
            "{rule:?}: {rank} is the exact crossing against a half-weight sharer"
        );

        // A naming weight of one raw unit collects a bound the law may round to
        // nothing; whatever it collects, the call answers rather than refuses.
        crossing_rank_at(rule, &[tiny], AT_RANK, &[tiny, Fixed::ONE])
            .unwrap_or_else(|error| panic!("{rule:?}: a raw naming weight of one: {error}"));
    }
}
