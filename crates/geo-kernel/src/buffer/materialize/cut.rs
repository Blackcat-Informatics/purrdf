// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact cuts of a continuous longitude lift. A cut replaces a crossed affine
//! carrier edge by its exact intersection and a meridian closure. At most two
//! crossings certify a single connected piece; unresolved multiple pieces
//! refuse. Longitude shifts are exact complete chart changes, not new edges
//! connecting the two sides of the date line.

use super::{admit_vertices, coordinate_order};
use crate::carrier::MaterializationStorage;
use crate::context::WorkProgress;
use crate::numerical::{ExactAdmission, compare_rat, copy_rat};
use crate::{AxisOrder, Coord, GeoError, LonLat, MetricContext, Rat, SourceLinearEdge};
use purrdf_xsd::integer::ExactOperation::{Linear, RationalAdd, RationalDivide, RationalReduce};

pub(super) fn lifted(
    point: &LonLat,
    center: &LonLat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Coord, GeoError> {
    lifted_from_longitude(point, center.longitude(), context, progress)
}

pub(super) fn lifted_from_longitude(
    point: &LonLat,
    origin: &Rat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Coord, GeoError> {
    let shape = |value: &Rat| (value.numerator().bit_len(), value.denominator().bit_len());
    let cost = purrdf_xsd::integer::ExactArithmeticCost::rational_difference_integer_interval(
        shape(point.longitude()),
        shape(origin),
        8, // Both integer bounds have magnitude 180, whose width is eight bits.
    )
    .ok_or(GeoError::ArithmeticOverflow(
        "buffer longitude lift admission",
    ))?;
    let unchanged = progress.exact_context(context, cost, |context| {
        if let Some(scratch) = context.integer_scratch() {
            point
                .longitude()
                .difference_in_integer_interval_in(origin, -180, 180, scratch)
                .map_err(GeoError::NumericalScratch)
        } else {
            Ok(point
                .longitude()
                .difference_in_integer_interval(origin, -180, 180))
        }
    })?;
    if unchanged {
        // The normalized difference is already in its half-open chart, so
        // center + normalize(point - center) is exactly the original point.
        return Ok(Coord::xy(
            copy_rat(point.longitude(), context, progress)?,
            copy_rat(point.latitude(), context, progress)?,
        ));
    }
    let mut admission = ExactAdmission::new(context, progress);
    let difference = admission.rational(RationalAdd, &[point.longitude(), origin], 1, || {
        Ok(point.longitude().sub(origin))
    })?;
    let difference = admission.rational(RationalReduce, &[&difference], 1, || {
        Ok(crate::geographic::normalize_degrees(&difference, -180))
    })?;
    let longitude = admission.rational(RationalAdd, &[origin, &difference], 1, || {
        Ok(origin.add(&difference))
    })?;
    let latitude = admission.rational(Linear, &[point.latitude()], 1, || {
        Ok(point.latitude().clone())
    })?;
    Ok(Coord::xy(longitude, latitude))
}

pub(super) fn polygons(
    ring: Vec<Coord>,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    total: &mut u64,
    retained: &mut MaterializationStorage,
) -> Result<Vec<crate::Rings>, GeoError> {
    polygons_in_turns(ring, -1..=1, context, progress, total, retained)
}

pub(super) fn atlas_polygons(
    ring: Vec<Coord>,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    total: &mut u64,
    retained: &mut MaterializationStorage,
) -> Result<Vec<crate::Rings>, GeoError> {
    polygons_in_turns(ring, -2..=2, context, progress, total, retained)
}

fn polygons_in_turns(
    mut ring: Vec<Coord>,
    turns: core::ops::RangeInclusive<i64>,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    total: &mut u64,
    retained: &mut MaterializationStorage,
) -> Result<Vec<crate::Rings>, GeoError> {
    let west = Rat::from_i64(-180);
    let east = Rat::from_i64(180);
    let mut contained = true;
    for point in &ring {
        contained &= !compare_rat(point.x(), &west, context, progress)?.is_lt()
            && !compare_rat(point.x(), &east, context, progress)?.is_gt();
    }
    if contained {
        if context.reference().axes() == AxisOrder::LatLon {
            for point in &mut ring {
                *point = Coord::xy(
                    copy_rat(point.y(), context, progress)?,
                    copy_rat(point.x(), context, progress)?,
                );
            }
        }
        canonicalize(&mut ring, context, progress)?;
        admit_vertices(ring.len() as u64, total, context, retained)?;
        return Ok(vec![vec![ring]]);
    }
    let mut temporary = 0;
    let result = (|| {
        let mut output = Vec::new();
        for turn in turns {
            let shift = Rat::from_i64(360 * turn);
            let west = Rat::from_i64(-180 + 360 * turn);
            let east = Rat::from_i64(180 + 360 * turn);
            let left = clip(&ring, &west, false, context, progress, &mut temporary)?;
            if left.len() < 4 {
                continue;
            }
            let mut piece = clip(&left, &east, true, context, progress, &mut temporary)?;
            if piece.len() < 4 {
                continue;
            }
            for point in &mut piece {
                let longitude = ExactAdmission::new(context, progress).rational(
                    RationalAdd,
                    &[point.x(), &shift],
                    1,
                    || Ok(point.x().sub(&shift)),
                )?;
                let latitude = copy_rat(point.y(), context, progress)?;
                *point = match context.reference().axes() {
                    AxisOrder::LonLat => Coord::xy(longitude, latitude),
                    AxisOrder::LatLon => Coord::xy(latitude, longitude),
                };
            }
            canonicalize(&mut piece, context, progress)?;
            admit_vertices(piece.len() as u64, total, context, retained)?;
            output.push(vec![piece]);
        }
        Ok(output)
    })();
    context.release_workspace(temporary)?;
    result
}

fn clip(
    ring: &[Coord],
    cut: &Rat,
    upper: bool,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    temporary: &mut u64,
) -> Result<Vec<Coord>, GeoError> {
    let bytes = (ring.len() as u64).saturating_add(4).saturating_mul(2048);
    context.admit_workspace(bytes)?;
    *temporary = temporary
        .checked_add(bytes)
        .ok_or(GeoError::ArithmeticOverflow("buffer cut storage"))?;
    let mut result = Vec::with_capacity(ring.len() + 4);
    let mut crossings = 0;
    for edge in ring.windows(2) {
        let left = inside(&edge[0], cut, upper, context, progress)?;
        let right = inside(&edge[1], cut, upper, context, progress)?;
        if left != right {
            crossings += 1;
            if crossings > 2 {
                return Err(GeoError::PrecisionExhausted {
                    bits: context.policy().limits().max_precision_bits,
                });
            }
            let mut admission = ExactAdmission::new(context, progress);
            let offset = admission.rational(RationalAdd, &[cut, edge[0].x()], 1, || {
                Ok(cut.sub(edge[0].x()))
            })?;
            let width = admission.rational(RationalAdd, &[edge[1].x(), edge[0].x()], 1, || {
                Ok(edge[1].x().sub(edge[0].x()))
            })?;
            let parameter = admission.rational(RationalDivide, &[&offset, &width], 1, || {
                offset.div(&width).ok_or_else(|| {
                    GeoError::domain("crossed meridian has zero longitude variation")
                })
            })?;
            let latitude = SourceLinearEdge::interpolate_ordinate_admitted(
                edge[0].y(),
                edge[1].y(),
                &parameter,
                &mut admission,
            )?;
            push(
                Coord::xy(copy_rat(cut, context, progress)?, latitude),
                &mut result,
                context,
                progress,
            )?;
        }
        if right {
            let point = ExactAdmission::new(context, progress).rational(
                Linear,
                &[edge[1].x(), edge[1].y()],
                2,
                || Ok(edge[1].clone()),
            )?;
            push(point, &mut result, context, progress)?;
        }
    }
    if !result.is_empty()
        && !coordinate_order(
            &result[0],
            result.last().expect("nonempty"),
            context,
            progress,
        )?
        .is_eq()
    {
        let closing = ExactAdmission::new(context, progress).rational(
            Linear,
            &[result[0].x(), result[0].y()],
            2,
            || Ok(result[0].clone()),
        )?;
        result.push(closing);
    }
    Ok(result)
}
fn inside(
    point: &Coord,
    cut: &Rat,
    upper: bool,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    let order = compare_rat(point.x(), cut, context, progress)?;
    Ok(if upper { order.is_le() } else { order.is_ge() })
}
fn push(
    point: Coord,
    output: &mut Vec<Coord>,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<(), GeoError> {
    if let Some(last) = output.last()
        && coordinate_order(last, &point, context, progress)?.is_eq()
    {
        return Ok(());
    }
    output.push(point);
    Ok(())
}

pub(super) fn canonicalize(
    ring: &mut [Coord],
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<(), GeoError> {
    let count = ring.len().saturating_sub(1);
    if count == 0 {
        return Ok(());
    }
    let mut start = 0;
    for index in 1..count {
        if coordinate_order(&ring[index], &ring[start], context, progress)?.is_lt() {
            start = index;
        }
    }
    ring[..count].rotate_left(start);
    let first = &ring[0];
    let closing = ExactAdmission::new(context, progress).rational(
        Linear,
        &[first.x(), first.y()],
        2,
        || Ok(first.clone()),
    )?;
    ring[count] = closing;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_half_open_lifts_preserve_exact_vertices_and_antipodes() {
        let mut context = MetricContext::wgs84().unwrap();
        context.prepare_integer_scratch().unwrap();
        let mut progress = WorkProgress::new(None);
        for (point, center, expected) in [
            (-180, 0, -180),
            (180, 0, -180),
            (0, 180, 0),
            (0, -180, -360),
            (-179, 179, 181),
            (179, -179, -181),
            (7, 9, 7),
        ] {
            context.begin(1).unwrap();
            let point = LonLat::new(Rat::from_i64(point), Rat::from_i64(37)).unwrap();
            let center = LonLat::new(Rat::from_i64(center), Rat::zero()).unwrap();
            let result = lifted(&point, &center, &mut context, &mut progress).unwrap();
            assert_eq!(result.x(), &Rat::from_i64(expected));
            assert_eq!(result.y(), point.latitude());
        }
        let epsilon = Rat::new(crate::Int::one(), crate::Int::one().shl(448)).unwrap();
        let point = LonLat::new(Rat::zero(), Rat::zero()).unwrap();
        for center in [
            Rat::from_i64(-180).add(&epsilon),
            Rat::from_i64(180).sub(&epsilon),
        ] {
            context.begin(1).unwrap();
            let center = LonLat::new(center, Rat::zero()).unwrap();
            let result = lifted(&point, &center, &mut context, &mut progress).unwrap();
            assert_eq!(result, Coord::xy(Rat::zero(), Rat::zero()));
        }
    }

    #[test]
    fn original_lift_predicate_is_admitted_before_source_copies() {
        let source = Rat::new(crate::Int::one(), crate::Int::one().shl(32_768)).unwrap();
        let point = LonLat::new(source, Rat::zero()).unwrap();
        let center = LonLat::new(Rat::zero(), Rat::zero()).unwrap();
        let policy = crate::ExecutionPolicy::new(crate::ExecutionLimits {
            max_work_items: 64,
            ..crate::ExecutionLimits::GEOMETRY
        })
        .unwrap();
        let mut context = MetricContext::new(crate::GeographicReference::wgs84(), policy).unwrap();
        context.begin(1).unwrap();
        let mut progress = WorkProgress::new(None);
        assert_eq!(
            lifted(&point, &center, &mut context, &mut progress),
            Err(GeoError::WorkExhausted { limit: 64 })
        );
        assert_eq!(context.work_items(), 0);
        assert_eq!(context.retained_workspace_bytes(), 0);
        assert!(context.integer_scratch().is_none());
    }
}
