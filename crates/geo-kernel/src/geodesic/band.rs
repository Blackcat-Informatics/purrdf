// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Completion goals of the certified shortest-distance solver.

use purrdf_xsd::{
    BigInt,
    math::{CertifiedInterval, IntervalContext, MathError},
};

use super::point_decisive_with;
use crate::{Int, Rat};

#[derive(Clone, Copy)]
pub(super) enum DistanceGoal<'a> {
    Point(Option<&'a Rat>),
    // A proof-only goal for an endpoint-defined line. Both metre and degree
    // enclosures must have width <=2^-fractional_bits; completed grids stay fixed.
    ArcProof(u32),
    // Any certified enclosure of the true shortest distance; nothing is rounded.
    Enclosure,
}
impl<'a> DistanceGoal<'a> {
    pub(super) fn physical_threshold(self) -> Option<&'a Rat> {
        match self {
            Self::Point(threshold) => threshold,
            Self::ArcProof(_) | Self::Enclosure => None,
        }
    }
    pub(super) fn decisive<I: CertifiedInterval>(
        self,
        distance: &I,
        physical: Option<&I>,
        math: &mut IntervalContext<'_>,
    ) -> Result<bool, MathError> {
        match self {
            Self::Point(_) => point_decisive_with(distance, physical, math),
            Self::ArcProof(_) => Ok(point_decisive_with(distance, None, math)?
                && self.enclosure_decisive(distance, math)?),
            Self::Enclosure => Ok(true),
        }
    }
    pub(super) fn enclosure_decisive<I: CertifiedInterval>(
        self,
        value: &I,
        math: &mut IntervalContext<'_>,
    ) -> Result<bool, MathError> {
        let Self::ArcProof(bits) = self else {
            return Ok(true);
        };
        let quantum = I::from_ratio(
            &BigInt::from_i128(1),
            &BigInt::from_i128(1).mul_pow2(bits),
            math,
        )?;
        Ok(value.width(math)?.upper() <= quantum.lower())
    }
    pub(super) fn exact_decisive(self, lower: &Rat, upper: &Rat) -> bool {
        let lower_rounded = lower.round_to_scale(6);
        let upper_rounded = upper.round_to_scale(6);
        match self {
            Self::Point(threshold) => {
                lower_rounded == upper_rounded
                    && threshold.is_none_or(|threshold| upper <= threshold || lower > threshold)
            }
            Self::ArcProof(bits) => {
                lower_rounded == upper_rounded
                    && upper.sub(lower)
                        <= Rat::new(Int::one(), Int::one().shl(bits)).expect("positive dyadic")
            }
            Self::Enclosure => true,
        }
    }
}
