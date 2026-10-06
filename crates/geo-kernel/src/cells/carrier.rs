// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The complete coordinate-linear carrier enclosure of a native cube cell.
//! Chart walls are rounded outward before the physical enclosure is bounded.

use super::CellId;
use super::cover::chart::{Arithmetic, Interval, directed};
use super::cover::{ChartBounds, CoverBudget, Footprint, MixedCoverLimits};
use crate::context::WorkProgress;
use crate::numerical::frozen_decimal;
use crate::{GeoError, LonLat, Metres, MetricContext, Rat};
use purrdf_xsd::ieee::ratio::Rounding;
use purrdf_xsd::integer::{ExactArithmeticCost, ExactOperation};

/// A closed atlas carrier containing the entire assigned cell footprint.
/// `bbox_radius` encloses the complete carrier, including outward grid walls.
pub(crate) struct CellCarrierBounds {
    pub(crate) axis: LonLat,
    pub(crate) south: Rat,
    pub(crate) north: Rat,
    pub(crate) longitudes: Vec<(Rat, Rat)>,
    pub(crate) bbox_radius: Metres,
    /// The caller releases this admitted result storage after dropping it.
    pub(crate) retained_workspace_bytes: u64,
}

/// Reuse the native footprint, atlas cap, exact arithmetic and numerical homes.
/// This borrows the active invocation and never resets cumulative work.
pub(crate) fn cell_carrier_bounds(
    cell: CellId,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<CellCarrierBounds, GeoError> {
    context.prepare_integer_scratch_for_observed(128, progress)?;
    let occupied = context
        .policy()
        .limits()
        .max_workspace_bytes
        .saturating_sub(context.remaining_workspace());
    let policy = context
        .policy()
        .remaining_after(context.work_items(), occupied)?;
    let (result, work, peak) = {
        let mut observer =
            progress.nested(context.work_items(), occupied, context.workspace_peak());
        let mut child_progress = WorkProgress::new(Some(&mut observer));
        child_progress.initial()?;
        let mut budget = CoverBudget::from_progress(
            MixedCoverLimits {
                max_emitted_cells: 1,
                max_work_items: policy.limits().max_work_items,
                max_workspace_bytes: policy.limits().max_workspace_bytes,
            },
            child_progress,
            true,
        )?;
        budget.borrow_context(context);
        let result =
            enclose(cell, context, &mut budget).and_then(|value| budget.poll().map(|()| value));
        (result, budget.work_items(), budget.workspace_peak())
    };
    context.charge_work(work)?;
    context.admit_workspace(peak)?;
    let retained = result
        .as_ref()
        .map_or(0, |value| value.retained_workspace_bytes);
    context.release_workspace(peak.saturating_sub(retained))?;
    progress.context_poll(context)?;
    result
}

fn enclose(
    cell: CellId,
    context: &MetricContext,
    budget: &mut CoverBudget<'_>,
) -> Result<CellCarrierBounds, GeoError> {
    let upper = context
        .reference()
        .ellipsoid()
        .normal_metric_bounds_ref()
        .1
        .exact();
    let operand_bits = upper
        .numerator()
        .bit_len()
        .max(upper.denominator().bit_len())
        .max(512);
    let cost =
        ExactArithmeticCost::for_operation(ExactOperation::RationalMultiply, operand_bits, 1)
            .ok_or(GeoError::ArithmeticOverflow(
                "carrier live storage admission",
            ))?;
    // The frozen chart has at most two longitude intervals. All its original
    // generated values have at most 512-bit operands; the only caller-sized
    // operand is R. Reserve live result/intermediate storage before any clone
    // or vector allocation, independently of each operation's scratch.
    let reservation = cost
        .workspace_bytes
        .checked_mul(4)
        .and_then(|bytes| bytes.checked_add(65_536))
        .ok_or(GeoError::ArithmeticOverflow(
            "carrier live storage admission",
        ))?;
    budget.reserve(reservation)?;
    let result = enclose_inner(cell, context, budget);
    let retained = result
        .as_ref()
        .map_or(0, |bounds| bounds.retained_workspace_bytes);
    if retained > reservation {
        budget.release(reservation)?;
        return Err(GeoError::ArithmeticOverflow(
            "carrier retained storage bound",
        ));
    }
    budget.release(reservation - retained)?;
    result
}

fn enclose_inner(
    cell: CellId,
    context: &MetricContext,
    budget: &mut CoverBudget<'_>,
) -> Result<CellCarrierBounds, GeoError> {
    let footprint = Footprint::new_admitted(cell, budget)?;
    let ChartBounds {
        axis,
        south,
        north,
        longitudes,
    } = footprint.chart_bounds_outward(budget)?;
    let walls = longitudes;
    let upper = context
        .reference()
        .ellipsoid()
        .normal_metric_bounds_ref()
        .1
        .exact();
    let mut sources = purrdf_core::SmallVec::<[&Rat; 9]>::new();
    sources.extend([axis.longitude(), axis.latitude(), &south, &north, upper]);
    for (west, east) in &walls {
        sources.extend([west, east]);
    }
    let bbox_radius = Metres::new(budget.math_for_sources(&sources, |math, progress| {
        let physical = physical_radius(&axis, &south, &north, &walls, upper, &mut Interval(math))?;
        directed(&physical, 6, Rounding::Up, math, progress)
    })?);
    drop(sources);
    let storage = (size_of::<CellCarrierBounds>() as u64)
        .saturating_add((walls.capacity() * size_of::<(Rat, Rat)>()) as u64)
        .saturating_add(axis.longitude().allocated_bytes() as u64)
        .saturating_add(axis.latitude().allocated_bytes() as u64)
        .saturating_add(south.allocated_bytes() as u64)
        .saturating_add(north.allocated_bytes() as u64)
        .saturating_add(bbox_radius.exact().allocated_bytes() as u64)
        .saturating_add(walls.iter().fold(0u64, |bytes, (a, b)| {
            bytes
                .saturating_add(a.allocated_bytes() as u64)
                .saturating_add(b.allocated_bytes() as u64)
        }));
    Ok(CellCarrierBounds {
        axis,
        south,
        north,
        longitudes: walls,
        bbox_radius,
        retained_workspace_bytes: storage,
    })
}

fn physical_radius<A: Arithmetic>(
    axis: &LonLat,
    south: &Rat,
    north: &Rat,
    walls: &[(Rat, Rat)],
    upper: &Rat,
    arithmetic: &mut A,
) -> Result<A::Number, A::Error> {
    let latitude = arithmetic.original(axis.latitude())?;
    let south = arithmetic.original(south)?;
    let north = arithmetic.original(north)?;
    let low = arithmetic.subtract(&south, &latitude)?;
    let high = arithmetic.subtract(&north, &latitude)?;
    let low = arithmetic.absolute(&low)?;
    let high = arithmetic.absolute(&high)?;
    let delta_latitude = arithmetic.maximum(low, high)?;
    let half_turn = arithmetic.original(&Rat::from_i64(180))?;
    let mut delta_longitude = arithmetic.original(&Rat::zero())?;
    for (west, east) in walls {
        let west = arithmetic.original(west)?;
        let east = arithmetic.original(east)?;
        let mut interval = arithmetic.original(&Rat::from_i64(180))?;
        for sign in [-1, 0, 1] {
            let shift = arithmetic.original(&Rat::from_i64(sign * 360))?;
            let longitude = arithmetic.original(axis.longitude())?;
            let center = arithmetic.add(&longitude, &shift)?;
            let west_delta = arithmetic.subtract(&west, &center)?;
            let east_delta = arithmetic.subtract(&east, &center)?;
            let west_delta = arithmetic.absolute(&west_delta)?;
            let east_delta = arithmetic.absolute(&east_delta)?;
            let extent = arithmetic.maximum(west_delta, east_delta)?;
            interval = arithmetic.minimum(interval, extent)?;
        }
        delta_longitude = arithmetic.maximum(delta_longitude, interval)?;
    }
    let pole = arithmetic.original(&Rat::from_i64(90))?;
    let phi = arithmetic.absolute(&latitude)?;
    let colatitude = arithmetic.subtract(&pole, &phi)?;
    let pi = arithmetic.original(&frozen_decimal(
        "3.14159265358979323846264338327950288419716939937511",
    ))?;
    let cosine = arithmetic.multiply(&colatitude, &pi)?;
    let cosine = arithmetic.divide(&cosine, &half_turn)?;
    let one = arithmetic.original(&Rat::one())?;
    let cosine = arithmetic.minimum(one, cosine)?;
    let longitude = arithmetic.multiply(&cosine, &delta_longitude)?;
    let parallel = arithmetic.add(&delta_latitude, &longitude)?;
    let via_north = arithmetic.subtract(&half_turn, &latitude)?;
    let via_north = arithmetic.subtract(&via_north, &south)?;
    let via_south = arithmetic.add(&half_turn, &latitude)?;
    let via_south = arithmetic.add(&via_south, &north)?;
    let angular = arithmetic.minimum(parallel, via_north)?;
    let angular = arithmetic.minimum(angular, via_south)?;
    let radians = arithmetic.multiply(&angular, &pi)?;
    let radians = arithmetic.divide(&radians, &half_turn)?;
    let upper = arithmetic.original(upper)?;
    arithmetic.multiply(&upper, &radians)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cells::{CubeHilbertQ62V1, NativeGridProfile};

    #[test]
    fn outward_carriers_enclose_leaf_sources_and_shrink_near_poles_and_cuts() {
        let grid = CubeHilbertQ62V1::new(NativeGridProfile::Wgs84);
        for (longitude, latitude) in [
            (0, 0),
            (45, 35),
            (135, -35),
            (-135, 35),
            (-45, -35),
            (0, 80),
            (123, 80),
            (-179, -80),
            (180, 0),
            (-180, 0),
            (17, 90),
            (-179, -90),
        ] {
            let source = LonLat::new(Rat::from_i64(longitude), Rat::from_i64(latitude)).unwrap();
            let cell = grid.assign(&source, 30).unwrap();
            let mut context = MetricContext::wgs84().unwrap();
            context.begin(1).unwrap();
            let mut progress = WorkProgress::new(None);
            progress.initial().unwrap();
            let bounds = cell_carrier_bounds(cell, &mut context, &mut progress).unwrap();
            assert!(bounds.south <= *source.latitude() && *source.latitude() <= bounds.north);
            assert!(
                source.is_pole()
                    || bounds.longitudes.iter().any(|(west, east)| {
                        (west <= source.longitude() && source.longitude() <= east)
                            || (source.longitude().abs() == Rat::from_i64(180)
                                && (west == &Rat::from_i64(-180) || east == &Rat::from_i64(180)))
                    })
            );
            assert!(bounds.bbox_radius.exact() < &frozen_decimal("0.05"));
            for value in core::iter::once(&bounds.south)
                .chain(core::iter::once(&bounds.north))
                .chain(bounds.longitudes.iter().flat_map(|(a, b)| [a, b]))
            {
                assert_eq!(
                    value.round_to_scale_with(15, Rounding::Down),
                    value.round_to_scale_with(15, Rounding::Up)
                );
            }
            let retained = bounds.retained_workspace_bytes;
            let axis = bounds.axis.clone();
            let radius = bounds.bbox_radius.clone();
            let corners = bounds
                .longitudes
                .iter()
                .flat_map(|(a, b)| {
                    [a, b].into_iter().flat_map(|longitude| {
                        [&bounds.south, &bounds.north]
                            .into_iter()
                            .map(move |latitude| {
                                LonLat::new(longitude.clone(), latitude.clone()).unwrap()
                            })
                    })
                })
                .collect::<Vec<_>>();
            drop(bounds);
            context.release_workspace(retained).unwrap();
            let prepared = crate::PreparedGeodesic::new(context.reference().clone());
            for corner in corners {
                let distance = prepared.distance(&axis, &corner, &mut context).unwrap();
                assert!(
                    distance
                        .value()
                        .exact()
                        .sub(distance.rounding_bound().exact())
                        <= *radius.exact()
                );
            }
        }
    }
}
