// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Prepared source-region covers using certified footprints and exact atlas membership.

use super::cover::chart::ChartEnclosure;
use super::cover::{Classification, CoverBudget, Footprint, build, fixed_limits, native_reference};
use super::{
    CellCover, CellId, CoverLawId, CoverLevels, CubeHilbertQ62V1, FixedCoverLimits,
    MixedCoverLimits,
};
use crate::{GeoError, MetricWorkObserver, PreparedRegion, Set};

const REGION_CLASSIFIER: &[u8] = b"cube-rational-midpoint-cap;assignment-guard=2^-59;axis=RN-degree15;axis-guard=pi-upper*10^-15/180;closed-latitude-longitude-footprint;cos-lower=1-abs(latitude)/90;selected-physical-closed-strata;complete-fragment-and-node-complement-sectors;retained-isolated-points-and-curve-components;shared-original-edge-closed-rectangle-separation-and-certified-intersection;identified-seam-pole-region-membership;strict-min-max;completed-four-siblings-to-min;sorted-disjoint;v3";

impl CoverLawId {
    /// Frozen prepared-region classifier, distinct from point keys and disk/box covers.
    #[must_use]
    pub fn prepared_region() -> Self {
        Self::from_classifier(REGION_CLASSIFIER)
    }
}

impl CubeHilbertQ62V1 {
    /// Cover a prepared closed region at one fixed level, counting logical cells.
    ///
    /// # Errors
    /// Refuses a reference mismatch and incomplete precision/work/memory/output admission.
    pub fn cover_region_fixed(
        self,
        region: &PreparedRegion,
        level: u8,
        limits: FixedCoverLimits,
    ) -> Result<CellCover, GeoError> {
        self.cover_region_mixed(
            region,
            CoverLevels::new(level, level)?,
            fixed_limits(limits),
        )
    }
    /// Cover a prepared region through complete strict mixed-level traversal.
    /// Boundary separation uses exact segment/rectangle predicates against a
    /// certified complete footprint, including longitude aliases and pole bands.
    ///
    /// # Errors
    /// Refuses a reference mismatch and incomplete precision/work/memory/output admission.
    pub fn cover_region_mixed(
        self,
        region: &PreparedRegion,
        levels: CoverLevels,
        limits: MixedCoverLimits,
    ) -> Result<CellCover, GeoError> {
        self.cover_region(region, levels, limits, None)
    }
    fn cover_region(
        self,
        region: &PreparedRegion,
        levels: CoverLevels,
        limits: MixedCoverLimits,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<CellCover, GeoError> {
        let mut budget = CoverBudget::new_metered(limits, observer)?;
        let reference = native_reference(self.profile);
        region.check_reference_with(&reference, &mut |work| budget.charge(work))?;
        if let PreparedRegion::Polygons(polygons) | PreparedRegion::ComplementOfPolygons(polygons) =
            region
        {
            for polygon in polygons.iter() {
                budget.charge(1)?;
                budget.reserve(
                    polygon
                        .coordinate_count()
                        .saturating_mul(2048)
                        .saturating_add(polygon.coordinate_bits().div_ceil(8).saturating_mul(32)),
                )?;
            }
        }
        let boundary = if matches!(
            region,
            PreparedRegion::Polygons(_) | PreparedRegion::ComplementOfPolygons(_)
        ) {
            let mut context = budget.context(reference)?;
            let result = {
                let mut nested = budget.nested();
                let mut progress = crate::context::WorkProgress::new(Some(&mut nested));
                progress.initial()?;
                crate::atlas::boundary::region_boundary(region, &mut context, &mut progress)
            };
            budget.context_receipt(&context)?;
            let boundary = result.map_err(|error| budget.invocation_error(error))?;
            budget.reserve(boundary.workspace_bytes)?;
            context.release_workspace(boundary.workspace_bytes)?;
            Some(boundary)
        } else {
            None
        };
        let result = build(self, levels, limits, &mut budget, |cell, budget| {
            classify(region, boundary.as_ref(), cell, budget, self)
        })
        .map(|cover| cover.with_law(CoverLawId::prepared_region()));
        if let Some(boundary) = boundary {
            let bytes = boundary.workspace_bytes;
            drop(boundary);
            budget.release(bytes)?;
        }
        result
    }

    /// Meter a complete mixed prepared-region cover through bounded chunks.
    /// # Errors
    /// Adds observer refusal to the complete region-cover contract.
    pub fn cover_region_mixed_metered(
        self,
        region: &PreparedRegion,
        levels: CoverLevels,
        limits: MixedCoverLimits,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<CellCover, GeoError> {
        self.cover_region(region, levels, limits, Some(observer))
    }
    /// Meter a complete fixed prepared-region cover.
    /// # Errors
    /// Refuses invalid level or incomplete observer/resource admission.
    pub fn cover_region_fixed_metered(
        self,
        region: &PreparedRegion,
        level: u8,
        limits: FixedCoverLimits,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<CellCover, GeoError> {
        self.cover_region(
            region,
            CoverLevels::new(level, level)?,
            fixed_limits(limits),
            Some(observer),
        )
    }
}

fn classify(
    region: &PreparedRegion,
    boundary: Option<&crate::atlas::boundary::RegionBoundary>,
    cell: CellId,
    budget: &mut CoverBudget<'_>,
    grid: CubeHilbertQ62V1,
) -> Result<Classification, GeoError> {
    match region {
        PreparedRegion::Empty => return Ok(Classification::Outside),
        PreparedRegion::Whole => return Ok(Classification::Inside),
        PreparedRegion::Polygons(_) | PreparedRegion::ComplementOfPolygons(_) => {}
    }
    let footprint = Footprint::new_admitted(cell, budget)?;
    let axis = footprint.point(budget)?;
    let boundary = boundary.expect("polygon physical boundary admitted before traversal");
    let limit = budget.max_precision_bits();
    let mut precision = 128.min(limit);
    let intersects = loop {
        let enclosure = match footprint.chart_enclosure(&axis, precision, budget) {
            Err(GeoError::PrecisionExhausted { .. }) if precision < limit => {
                precision = precision.saturating_mul(2).min(limit);
                continue;
            }
            result => result?,
        };
        let result = boundary_intersects(boundary, &enclosure, budget, grid);
        let retained = enclosure.retained;
        drop(enclosure);
        budget.release(retained)?;
        match result? {
            Some(intersects) => break intersects,
            None if precision < limit => {
                precision = precision.saturating_mul(2).min(limit);
            }
            None => return Err(GeoError::PrecisionExhausted { bits: limit }),
        }
    };
    if intersects {
        return Ok(Classification::Straddling);
    }
    let mut context = budget.context(native_reference(grid.profile))?;
    let location = if budget.metered {
        crate::atlas::locate_inner_metered(&axis, region, &mut context, &mut budget.nested())
    } else {
        crate::atlas::locate_inner(&axis, region, &mut context)
    };
    budget.context_receipt(&context)?;
    Ok(
        match location.map_err(|error| budget.invocation_error(error))? {
            Set::Interior => Classification::Inside,
            Set::Exterior => Classification::Outside,
            Set::Boundary => Classification::Straddling,
        },
    )
}

fn boundary_intersects(
    boundary: &crate::atlas::boundary::RegionBoundary,
    enclosure: &ChartEnclosure,
    budget: &mut CoverBudget<'_>,
    grid: CubeHilbertQ62V1,
) -> Result<Option<bool>, GeoError> {
    let bounds = &enclosure.bounds;
    let mut uncertain = false;
    for (west, east) in &bounds.longitudes {
        let outer = [&west.0, &east.1, &bounds.south.0, &bounds.north.1];
        if !rectangle_intersects(boundary, outer, budget, grid)? {
            continue;
        }
        let inner = [&west.1, &east.0, &bounds.south.1, &bounds.north.0];
        let valid = budget.rational(
            purrdf_xsd::integer::ExactOperation::RationalCompare,
            &inner,
            2,
            || Ok(inner[0] <= inner[1] && inner[2] <= inner[3]),
        )?;
        if valid && rectangle_intersects(boundary, inner, budget, grid)? {
            return Ok(Some(true));
        }
        uncertain = true;
    }
    Ok(if uncertain { None } else { Some(false) })
}

fn rectangle_intersects(
    boundary: &crate::atlas::boundary::RegionBoundary,
    rectangle: [&crate::Rat; 4],
    budget: &mut CoverBudget<'_>,
    grid: CubeHilbertQ62V1,
) -> Result<bool, GeoError> {
    let mut context = budget.context(native_reference(grid.profile))?;
    let result = {
        let mut nested = budget.nested();
        let mut progress = crate::context::WorkProgress::new(Some(&mut nested));
        progress.initial()?;
        crate::atlas::enclosure::selected_boundary_intersects_rectangle(
            boundary,
            rectangle,
            &mut context,
            &mut progress,
        )
    };
    budget.context_receipt(&context)?;
    result.map_err(|error| budget.invocation_error(error))
}

#[cfg(test)]
mod tests;
