// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use crate::v2::{PayloadOutcome, PayloadUnchecked, RdfGraph};

use super::{
    CanonicalLabellingCheck, CanonicalLabellingDifference, CanonicalLabellingUnchecked,
    CanonicalStructureCheck, CanonicalStructureDifference,
};

/// The canonical check of a segment in a whole-file labelling report.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct SegmentCanonicalCheck {
    /// The opener claims a canonical form this reader recognizes.
    pub claimed: bool,
    /// The structure result.
    pub structure: CanonicalStructureCheck,
    /// The labelling result.
    pub labelling: CanonicalLabellingCheck,
    /// The stored-byte comparison status.
    pub storage: CanonicalStorageCheck,
    /// The first difference across the structure and labelling checks.
    pub first_difference: Option<CanonicalDifference>,
}

/// The reason this reader could not compare storage with the reference encoder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CanonicalStorageCheck {
    /// The segment makes no known canonical claim.
    NotClaimed,
    /// The item, stored-payload or decoded-payload capacity prevented comparison.
    ExceedsCapacity,
    /// The reader does not hold the format's reference encoder.
    EncoderNotHeld,
}

/// The canonical violation at the first differing item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct CanonicalDifference {
    /// File-relative byte offset of the difference.
    pub offset: usize,
    /// Item position within its segment.
    pub position: usize,
    /// The rule that failed.
    pub reason: CanonicalDifferenceReason,
}

/// A rule that failed at either canonical check level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CanonicalDifferenceReason {
    /// A structure rule.
    Structure(CanonicalStructureDifference),
    /// A labelling rule.
    Labelling(CanonicalLabellingDifference),
}

impl SegmentCanonicalCheck {
    pub(super) const fn without_fold(claimed: bool) -> Self {
        let (structure, labelling, storage) = if claimed {
            (
                CanonicalStructureCheck::ExceedsCapacity,
                CanonicalLabellingUnchecked::ExceedsCapacity,
                CanonicalStorageCheck::ExceedsCapacity,
            )
        } else {
            (
                CanonicalStructureCheck::NotClaimed,
                CanonicalLabellingUnchecked::NotClaimed,
                CanonicalStorageCheck::NotClaimed,
            )
        };
        Self {
            claimed,
            structure,
            labelling: CanonicalLabellingCheck::Unchecked(labelling),
            storage,
            first_difference: None,
        }
    }

    pub(super) fn from_levels(
        fold: &RdfGraph<'_>,
        structure: CanonicalStructureCheck,
        labelling: CanonicalLabellingCheck,
    ) -> Self {
        let structure_failure = match structure {
            CanonicalStructureCheck::Failed {
                offset,
                position,
                difference,
            } => Some(CanonicalDifference {
                offset,
                position,
                reason: CanonicalDifferenceReason::Structure(difference),
            }),
            _ => None,
        };
        let labelling_failure = match labelling {
            CanonicalLabellingCheck::Failed {
                offset,
                position,
                difference,
            } => Some(CanonicalDifference {
                offset,
                position,
                reason: CanonicalDifferenceReason::Labelling(difference),
            }),
            _ => None,
        };
        // P2 assigns structure differences codes 1 to 10, then labelling
        // differences 11 and 12. The earlier entry wins a position tie.
        let first_difference = [structure_failure, labelling_failure]
            .into_iter()
            .flatten()
            .min_by_key(|failure| failure.position);
        let storage = if !fold.canonical_claim {
            CanonicalStorageCheck::NotClaimed
        } else if fold.frame_wire.iter().any(|frame| {
            frame.payload == PayloadOutcome::Unchecked(PayloadUnchecked::ExceedsCapacity)
        }) {
            CanonicalStorageCheck::ExceedsCapacity
        } else {
            CanonicalStorageCheck::EncoderNotHeld
        };
        Self {
            claimed: fold.canonical_claim,
            structure,
            labelling,
            storage,
            first_difference,
        }
    }
}
