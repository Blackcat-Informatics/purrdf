// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Explicit completed scalar coordinate quanta, independent of proof precision.

use super::CoordinateUnit;
use crate::{GeoError, MetricContext, SemanticLawId, context::WorkProgress};
use purrdf_hash::Domain;

const GRID_LAW: Domain = Domain::new(b"purrdf-geo-kernel/transform-output-grid-law/v1");

/// Declared half-even output grids for angular XY and physical metre ordinates.
/// Unused unit grids coalesce with their defaults and cannot change an answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TransformOutputGrid {
    angular_decimal_places: u32,
    metric_decimal_places: u32,
}
impl TransformOutputGrid {
    /// The frozen standard output: fifteen angular and six metre decimal places.
    pub const DEFAULT: Self = Self::new(15, 6);

    /// Declare output quanta. Application admits the complete decimal powers
    /// and verifies the physical quantization bound before numerical execution.
    #[must_use]
    pub const fn new(angular_decimal_places: u32, metric_decimal_places: u32) -> Self {
        Self {
            angular_decimal_places,
            metric_decimal_places,
        }
    }
    /// Decimal places of longitude/latitude or other angular XY output.
    #[must_use]
    pub const fn angular_decimal_places(self) -> u32 {
        self.angular_decimal_places
    }
    /// Decimal places of Cartesian XY/XYZ and an actual height ordinate.
    #[must_use]
    pub const fn metric_decimal_places(self) -> u32 {
        self.metric_decimal_places
    }
    pub(super) const fn decimal_places(self, unit: CoordinateUnit, ordinate: usize) -> u32 {
        if matches!(unit, CoordinateUnit::Degrees) && ordinate < 2 {
            self.angular_decimal_places
        } else {
            self.metric_decimal_places
        }
    }
    pub(super) const fn normalized(self, unit: CoordinateUnit, height: bool) -> Self {
        match unit {
            CoordinateUnit::Metres => Self::new(15, self.metric_decimal_places),
            CoordinateUnit::Degrees => Self::new(
                self.angular_decimal_places,
                if height {
                    self.metric_decimal_places
                } else {
                    6
                },
            ),
        }
    }
    pub(super) fn law(self, original: SemanticLawId) -> SemanticLawId {
        if self == Self::DEFAULT {
            return original;
        }
        SemanticLawId::from_digest(crate::profile::hash_fields(GRID_LAW, [
            b"original-operation;half-even;explicit-final-only-grid;quantization-bound-metres=0.000001;inverse-forward-residual=original-contract;certificate=v1".as_slice(),
            original.digest().as_bytes(),
            &self.angular_decimal_places.to_be_bytes(),
            &self.metric_decimal_places.to_be_bytes(),
        ]))
    }
    pub(super) fn validate(
        self,
        unit: CoordinateUnit,
        height: bool,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
    ) -> Result<(), GeoError> {
        let grid = self.normalized(unit, height);
        if grid == Self::DEFAULT {
            return Ok(());
        }
        match unit {
            CoordinateUnit::Metres => crate::numerical::validate_metric_output_grid(
                grid.metric_decimal_places,
                context,
                progress,
            ),
            CoordinateUnit::Degrees => {
                let bytes = context.reference().ellipsoid().retained_limb_bytes();
                context.admit_workspace(bytes)?;
                let result = (|| {
                    let reference = crate::numerical::reference_clone(context, progress)?;
                    crate::numerical::validate_angular_output_grid(
                        grid.angular_decimal_places,
                        reference.ellipsoid().normal_metric_bounds_ref().1.exact(),
                        height.then_some(grid.metric_decimal_places),
                        context,
                        progress,
                    )
                })();
                context.release_workspace(bytes)?;
                result
            }
        }
    }
}

#[cfg(test)]
mod tests;
