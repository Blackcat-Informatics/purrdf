// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact vertical decomposition of the complete source-linear region union.
//!
//! Events include every vertex and segment crossing. Between successive events
//! edge order cannot change, so an exact interior representative labels the
//! entire open strip. Chart cuts and identified poles have zero surface measure.

use crate::context::WorkProgress;
use crate::{GeoError, LonLat, MetricContext, PreparedRegion, Rat};

/// One disjoint open atlas strip with exact affine lower and upper boundaries.
pub(crate) struct AreaStrip {
    pub(crate) lower: [LonLat; 2],
    pub(crate) upper: [LonLat; 2],
}

pub(crate) struct AreaArrangement {
    pub(crate) strips: Vec<AreaStrip>,
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
    let (count, bits) = match region {
        PreparedRegion::Polygons(polygons) | PreparedRegion::ComplementOfPolygons(polygons) => {
            polygons
                .iter()
                .fold((0_u64, 0_u64), |(count, bits), polygon| {
                    (
                        count.saturating_add(polygon.coordinate_count()),
                        bits.saturating_add(polygon.coordinate_bits()),
                    )
                })
        }
        PreparedRegion::Empty | PreparedRegion::Whole => (0, 0),
    };
    // Each exact intersection uses a fixed number of rational products. This
    // intentionally generous bound admits those products before computing them.
    let scratch = bits
        .div_ceil(8)
        .saturating_mul(128)
        .saturating_add(count.saturating_mul(256))
        .saturating_add(65_536);
    context.admit_workspace(scratch)?;
    let mut retained = scratch;
    let result = decompose(region, coalesce_interior, context, &mut retained, progress);
    match result {
        Ok(strips) => Ok(AreaArrangement {
            strips,
            workspace_bytes: retained,
        }),
        Err(error) => {
            context.release_workspace(retained)?;
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
) -> Result<Vec<AreaStrip>, GeoError> {
    if matches!(region, PreparedRegion::Empty) {
        return Ok(Vec::new());
    }
    let mut edges = Vec::new();
    let mut native_edges = Vec::new();
    if let PreparedRegion::Polygons(polygons) | PreparedRegion::ComplementOfPolygons(polygons) =
        region
    {
        for polygon in polygons.iter() {
            if let Some(chart) = polygon.chart() {
                let crate::GeometryBody::Polygon(rings) = chart.body() else {
                    unreachable!("prepared polygon has a polygon chart");
                };
                for ring in rings {
                    for pair in ring.windows(2) {
                        context.retain_workspace(
                            size_of::<(&crate::Coord, &crate::Coord)>() as u64,
                            retained,
                        )?;
                        edges.push((&pair[0], &pair[1]));
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
                    let operands = points
                        .iter()
                        .flat_map(|point| [point.longitude(), point.latitude()])
                        .collect::<purrdf_core::SmallVec<[&Rat; 4]>>();
                    let bytes = operands
                        .iter()
                        .try_fold(
                            (2 * size_of::<crate::Coord>()
                                + size_of::<(&crate::Coord, &crate::Coord)>())
                                as u64,
                            |bytes, value| bytes.checked_add(value.allocated_bytes() as u64),
                        )
                        .ok_or(GeoError::ArithmeticOverflow(
                            "native atlas affine edge storage",
                        ))?;
                    context.retain_workspace(bytes, retained)?;
                    let pair = crate::numerical::ExactAdmission::new(context, progress).rational(
                        purrdf_xsd::integer::ExactOperation::Linear,
                        &operands,
                        4,
                        || {
                            Ok(points.map(|point| {
                                crate::Coord::xy(
                                    point.longitude().clone(),
                                    point.latitude().clone(),
                                )
                            }))
                        },
                    )?;
                    native_edges.push(pair);
                }
            }
        }
    }
    edges.extend(native_edges.iter().map(|pair| (&pair[0], &pair[1])));
    let domain = [
        Rat::from_i64(-180),
        Rat::from_i64(180),
        Rat::from_i64(-90),
        Rat::from_i64(90),
    ];
    let index = super::chart_index::ChartIndex::prepare(region, context, progress, retained)?;
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
    context.release_retained_workspace(index_bytes, retained)?;
    let strips = strips?;
    let original_slots = (strips.capacity() as u64)
        .checked_mul(size_of::<crate::topology::arrangement::PlanarStrip>() as u64)
        .ok_or(GeoError::ArithmeticOverflow("original atlas strip slots"))?;
    let count = strips.len();
    let destination_slots = (count as u64)
        .checked_mul(size_of::<AreaStrip>() as u64)
        .ok_or(GeoError::ArithmeticOverflow("physical atlas strip slots"))?;
    // Converting owned coordinates moves their Rat owners, but the enclosing
    // allocations overlap until the complete original iterator is dropped.
    context.charge_work(count as u64)?;
    context.retain_workspace(destination_slots, retained)?;
    progress.context_poll(context)?;
    let mut converted = Vec::new();
    converted
        .try_reserve_exact(count)
        .map_err(|_| GeoError::MemoryExhausted {
            limit: context.policy().limits().max_workspace_bytes,
        })?;
    if converted.capacity() != count {
        return Err(GeoError::ArithmeticOverflow(
            "atlas strip allocation capacity",
        ));
    }
    for strip in strips {
        let mut convert = |points: [crate::Coord; 2]| {
            let [a, b] = points;
            Ok([
                physical_owned(a, context, progress)?,
                physical_owned(b, context, progress)?,
            ])
        };
        converted.push(AreaStrip {
            lower: convert(strip.lower)?,
            upper: convert(strip.upper)?,
        });
    }
    context.release_retained_workspace(original_slots, retained)?;
    Ok(converted)
}

fn physical_owned(
    point: crate::Coord,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<LonLat, GeoError> {
    let cost = crate::numerical::rational_cost(
        purrdf_xsd::integer::ExactOperation::RationalCompare,
        &[point.x(), point.y()],
        4,
    )
    .ok_or(GeoError::ArithmeticOverflow(
        "atlas cell coordinate validation",
    ))?;
    progress.exact(context, cost, || {
        let (x, y) = point.into_xy();
        LonLat::new(x, y)
    })
}
