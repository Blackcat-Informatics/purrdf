// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact vertical decomposition of the complete source-linear region union.
//!
//! Events include every vertex and segment crossing. Between successive events
//! edge order cannot change, so an exact interior representative labels the
//! entire open strip. Chart cuts and identified poles have zero surface measure.

use crate::context::{RetainedBuilder, WorkProgress, persistent_numeric};
use crate::{GeoError, LonLat, MetricContext, PreparedRegion, Rat};

/// One disjoint open atlas strip with exact affine lower and upper boundaries.
pub(crate) struct AreaStrip {
    pub(crate) lower: [LonLat; 2],
    pub(crate) upper: [LonLat; 2],
}

pub(crate) struct AreaArrangement {
    pub(crate) strips: RetainedBuilder<AreaStrip>,
    pub(crate) workspace_bytes: u64,
}

/// Decompose only after admitting input, intersection scratch and every retained
/// event/strip. The returned reservation stays live until the metric consumes it.
pub(crate) fn area_arrangement(
    region: &PreparedRegion,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<AreaArrangement, GeoError> {
    arrangement(region, false, context, progress)
}

/// Materialization consumes maximal selected ordinate runs in each slab.
/// Their shared internal walls cancel in the exact union, while metrics keep
/// the original individual strips and their frozen quadrature subdivision.
pub(crate) fn area_arrangement_for_union(
    region: &PreparedRegion,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<AreaArrangement, GeoError> {
    arrangement(region, true, context, progress)
}

fn arrangement(
    region: &PreparedRegion,
    coalesce_interior: bool,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<AreaArrangement, GeoError> {
    super::check_region_reference(region, context, progress)?;
    let mut retained = 0;
    let result = decompose(region, coalesce_interior, context, &mut retained, progress);
    match result {
        Ok(strips) => Ok(AreaArrangement {
            strips,
            workspace_bytes: retained,
        }),
        Err(error) => {
            let _ = context.release_workspace(retained);
            Err(error)
        }
    }
}

fn decompose(
    region: &PreparedRegion,
    coalesce_interior: bool,
    context: &mut MetricContext,
    retained: &mut u64,
    progress: &mut WorkProgress<'_>,
) -> Result<RetainedBuilder<AreaStrip>, GeoError> {
    let mut temporary = 0;
    let result = decompose_inner(
        region,
        coalesce_interior,
        context,
        retained,
        &mut temporary,
        progress,
    );
    // Borrowed indexes and affine-edge copies are private working producers.
    // Their native owners are destroyed before their phase allowance is freed.
    let released = context.release_workspace(temporary);
    result.and_then(|strips| released.map(|()| strips))
}

fn decompose_inner(
    region: &PreparedRegion,
    coalesce_interior: bool,
    context: &mut MetricContext,
    retained: &mut u64,
    temporary: &mut u64,
    progress: &mut WorkProgress<'_>,
) -> Result<RetainedBuilder<AreaStrip>, GeoError> {
    if matches!(region, PreparedRegion::Empty) {
        return Ok(RetainedBuilder::default());
    }
    let mut native_edges = RetainedBuilder::default();
    let mut edges = RetainedBuilder::default();
    if let PreparedRegion::Polygons(polygons) | PreparedRegion::ComplementOfPolygons(polygons) =
        region
    {
        for polygon in polygons {
            if let Some(chart) = polygon.chart() {
                let crate::GeometryBody::Polygon(rings) = chart.body() else {
                    unreachable!("prepared polygon has a polygon chart");
                };
                for ring in rings {
                    for pair in ring.windows(2) {
                        edges.push_admitted((&pair[0], &pair[1]), context, progress, temporary)?;
                    }
                }
            } else {
                // Native oriented rings retain their original physical law.
                // A pole-closed wedge or full parallel need not have an ordinary
                // planar Polygon spelling. Its original affine edges still
                // give every vertical event; source membership labels the faces.
                // The shared decomposition supplies domain/seam walls itself.
                for edge in polygon.rings().iter().flat_map(crate::PreparedCurve::edges) {
                    let crate::PreparedEdge::SourceLinear(line) = edge else {
                        return Err(GeoError::PrecisionExhausted {
                            bits: context.policy().limits().max_precision_bits,
                        });
                    };
                    let points = [line.start().point(), line.end().point()];
                    let operands = [
                        points[0].longitude(),
                        points[0].latitude(),
                        points[1].longitude(),
                        points[1].latitude(),
                    ];
                    let pair = crate::numerical::ExactAdmission::new(context, progress)
                        .expression_result(
                            purrdf_xsd::integer::ExactOperation::Linear,
                            &operands,
                            4,
                            |values| {
                                Ok([
                                    values.planar_location(points[0])?,
                                    values.planar_location(points[1])?,
                                ])
                            },
                        )?;
                    let [a, b] = pair;
                    let pair = [
                        persistent_numeric(a, context, progress, temporary)?,
                        persistent_numeric(b, context, progress, temporary)?,
                    ];
                    native_edges.push_admitted(pair, context, progress, temporary)?;
                }
            }
        }
    }
    for pair in &native_edges {
        edges.push_admitted((&pair[0], &pair[1]), context, progress, temporary)?;
    }
    let domain = [
        Rat::from_i64(-180),
        Rat::from_i64(180),
        Rat::from_i64(-90),
        Rat::from_i64(90),
    ];
    let index = super::chart_index::ChartIndex::prepare(region, context, progress, temporary)?;
    let index_bytes = index
        .as_ref()
        .map_or(0, super::chart_index::ChartIndex::storage_bytes);
    let strips = crate::topology::arrangement::decompose(
        &edges,
        domain.each_ref(),
        coalesce_interior,
        context,
        progress,
        retained,
        |point, context, progress| {
            let point = super::boundary::physical_point(point, context, progress)?;
            super::locate_with_chart_index(&point, region, index.as_ref(), context, progress)
        },
    );
    drop(index);
    let released = context.release_retained_workspace(index_bytes, temporary);
    let strips = strips.and_then(|strips| released.map(|()| strips))?;
    let original_slots = u64::try_from(strips.admitted_bytes())
        .map_err(|_| GeoError::ArithmeticOverflow("original atlas strip slots"))?;
    let count = strips.len();
    // Converting owned coordinates moves their Rat owners, but the enclosing
    // allocations overlap until the complete original iterator is dropped.
    context.charge_work(count as u64)?;
    let mut converted = RetainedBuilder::default();
    converted.reserve_admitted(count, context, progress, retained)?;
    for strip in strips {
        let mut convert = |points: [crate::Coord; 2]| -> Result<_, GeoError> {
            let [a, b] = points;
            Ok([
                physical_owned(a, context, progress)?,
                physical_owned(b, context, progress)?,
            ])
        };
        converted.push_admitted(
            AreaStrip {
                lower: convert(strip.lower)?,
                upper: convert(strip.upper)?,
            },
            context,
            progress,
            retained,
        )?;
    }
    context.release_retained_workspace(original_slots, retained)?;
    Ok(converted)
}

fn physical_owned(
    point: crate::Coord,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<LonLat, GeoError> {
    let mut admission = crate::numerical::ExactAdmission::new(context, progress);
    admission.expression_result(
        purrdf_xsd::integer::ExactOperation::RationalCompare,
        &[point.x(), point.y()],
        4,
        |values| Ok(LonLat::validate_with(point.x(), point.y(), values)?),
    )??;
    let (x, y) = point.into_xy();
    Ok(LonLat::from_validated(x, y))
}
