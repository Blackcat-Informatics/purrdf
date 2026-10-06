// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Geographic DE-9IM under the same identified source atlas as membership.
//! Exact noding partitions every written edge. Open fragments have constant
//! classes, and every open chart face meets a latitude band between consecutive
//! vertices/crossings. The scan is restricted to the physical chart; its cuts
//! and poles are classified by the identified atlas, never as planar exteriors.

use super::boundary::physical_point;
use crate::context::WorkProgress;
use crate::numerical::ExactAdmission;
use crate::{
    Coord, Dim, GeoError, IntersectionMatrix, LonLat, MetricContext, MetricWorkObserver,
    PreparedEdge, PreparedGeometry, PreparedRegion, Rat, SemanticLawId, Set,
};
use purrdf_hash::Domain;
use purrdf_xsd::integer::ExactOperation;

const LAW: Domain = Domain::new(b"purrdf-geo-kernel/identified-surface-topology/v3");

/// Exact source-law topology, including physical longitude and pole aliases.
#[must_use]
pub fn topology_law_id() -> SemanticLawId {
    SemanticLawId::from_digest(crate::profile::hash_fields(LAW, [
        b"exact-source-law-sets;written-coordinate-linear-and-selected-geodesic-branches;actual-operation-images;identified-longitude-seam-and-poles;closed-regions-holes-complements-whole;union-interiors;complete-fragment-and-node-complement-sectors;selected-physical-strata;isolated-point-interior;deduplicated-fragment-mod2-curve-boundary;dimension-max-DE9IM;v3".as_slice(),
    ]))
}

/// Compute the exact physical intersection matrix of two prepared sources.
///
/// # Errors
/// Refuses reference mismatch, an unresolved modeled-curve arrangement, and any
/// incomplete work, memory, cancellation or precision admission.
pub fn relate(
    a: &PreparedGeometry,
    b: &PreparedGeometry,
    context: &mut MetricContext,
) -> Result<IntersectionMatrix, GeoError> {
    relate_inner(a, b, None, context, None)
}

/// Compute the same matrix with bounded governor polling.
///
/// # Errors
/// Adds the observer's original refusal to [`relate`].
pub fn relate_metered(
    a: &PreparedGeometry,
    b: &PreparedGeometry,
    context: &mut MetricContext,
    observer: &mut dyn MetricWorkObserver,
) -> Result<IntersectionMatrix, GeoError> {
    relate_inner(a, b, None, context, Some(observer))
}

/// Continue under the original admission of both immutable prepared sources.
///
/// # Errors
/// Adds a mismatched or absent source receipt to [`relate`]'s failures.
pub fn relate_prepared(
    a: &PreparedGeometry,
    b: &PreparedGeometry,
    receipts: [&crate::PreparedSourceReceipt; 2],
    context: &mut MetricContext,
) -> Result<IntersectionMatrix, GeoError> {
    relate_inner(a, b, Some(receipts), context, None)
}

/// Continue the same admitted relation with bounded external governor polling.
///
/// # Errors
/// Adds observer refusal to [`relate_prepared`].
pub fn relate_prepared_metered(
    a: &PreparedGeometry,
    b: &PreparedGeometry,
    receipts: [&crate::PreparedSourceReceipt; 2],
    context: &mut MetricContext,
    observer: &mut dyn MetricWorkObserver,
) -> Result<IntersectionMatrix, GeoError> {
    relate_inner(a, b, Some(receipts), context, Some(observer))
}

/// Greatest declared carrier dimension, with an empty source returning `None`.
/// Actual image and set dimensions come from a completed relation matrix's
/// [`IntersectionMatrix::input_dimensions`]. A declared curve can be a constant
/// image, and a closed region intersection can have lower dimension.
#[must_use]
pub fn dimension(source: &PreparedGeometry) -> Option<u8> {
    if !matches!(source.region(), PreparedRegion::Empty) {
        Some(2)
    } else if !source.curves().is_empty() {
        Some(1)
    } else if !source.points().is_empty() || !source.symbolic_points().is_empty() {
        Some(0)
    } else {
        None
    }
}

/// Original preparation work and retained bytes required by a source pair.
/// A source borrowed twice is counted once. Distinct prepared objects retain
/// separate admissions even when their content identities are equal.
///
/// # Errors
/// Refuses an overflowing sum of the original source receipts.
pub fn relation_source_admission(
    a: &PreparedGeometry,
    b: &PreparedGeometry,
) -> Result<(u64, u64), GeoError> {
    let first = (a.preparation_work_items(), a.retained_workspace_bytes());
    if core::ptr::eq(a, b) {
        return Ok(first);
    }
    Ok((
        first
            .0
            .checked_add(b.preparation_work_items())
            .ok_or(GeoError::ArithmeticOverflow("geographic source work"))?,
        first
            .1
            .checked_add(b.retained_workspace_bytes())
            .ok_or(GeoError::ArithmeticOverflow("geographic source storage"))?,
    ))
}

fn relate_inner(
    a: &PreparedGeometry,
    b: &PreparedGeometry,
    receipts: Option<[&crate::PreparedSourceReceipt; 2]>,
    context: &mut MetricContext,
    observer: Option<&mut dyn MetricWorkObserver>,
) -> Result<IntersectionMatrix, GeoError> {
    let (source_work, source_storage) = relation_source_admission(a, b)?;
    if let Some(receipts) = receipts {
        for (source, receipt) in [a, b].into_iter().zip(receipts) {
            if *receipt != source.source_receipt() {
                return Err(GeoError::config(
                    "geographic relation receipt names a different source",
                ));
            }
            receipt.validate_context(context)?;
        }
        if context.retained_workspace_bytes() < source_storage
            || context.preparation_work_items() < source_work
        {
            return Err(GeoError::config(
                "geographic relation requires both complete source admissions",
            ));
        }
    }
    context.begin(1)?;
    let mut progress = WorkProgress::new(observer);
    progress.context_poll(context)?;
    let reference = crate::numerical::reference_clone(context, &mut progress)?;
    if a.reference().id() != reference.id() || b.reference().id() != reference.id() {
        return Err(GeoError::config(
            "geographic topology requires one actual prepared reference",
        ));
    }
    let initial = if receipts.is_some() {
        0_u64
    } else {
        source_storage
    }
    .checked_add(65_536)
    .ok_or(GeoError::ArithmeticOverflow(
        "geographic relation inventory",
    ))?;
    context.admit_workspace(initial)?;
    let mut retained = initial;
    let result = written_matrix(a, b, context, &mut progress, &mut retained);
    context.release_workspace(retained)?;
    result
}

fn source_edges(source: &PreparedGeometry) -> impl Iterator<Item = &PreparedEdge> {
    let polygons: &[crate::PreparedPolygon] = match source.region() {
        PreparedRegion::Polygons(polygons) | PreparedRegion::ComplementOfPolygons(polygons) => {
            polygons
        }
        PreparedRegion::Empty | PreparedRegion::Whole => &[],
    };
    source
        .curves()
        .iter()
        .chain(polygons.iter().flat_map(crate::PreparedPolygon::rings))
        .flat_map(crate::PreparedCurve::edges)
}

fn written_matrix(
    a: &PreparedGeometry,
    b: &PreparedGeometry,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: &mut u64,
) -> Result<IntersectionMatrix, GeoError> {
    if let Some(matrix) = point_region_matrix(a, b, context, progress)? {
        return Ok(matrix);
    }
    if original_contact_graph(a, context, progress)?
        || original_contact_graph(b, context, progress)?
    {
        return super::arcs::arrangement::relation_matrix(a, b, context, progress);
    }
    let mut edges = Vec::new();
    let mut seeds = Vec::new();
    for point in a.points().iter().chain(b.points()) {
        seeds.push(copy_point(point.point(), context, progress)?);
    }
    for edge in source_edges(a).chain(source_edges(b)) {
        let PreparedEdge::SourceLinear(edge) = edge else {
            unreachable!("admitted written inventory")
        };
        let mut start = copy_point(edge.start().point(), context, progress)?;
        let mut end = copy_point(edge.end().point(), context, progress)?;
        // A complete cut meridian has one physical representative. Written
        // long edges retain their original longitude lift and interpolation.
        if start.x() == &Rat::from_i64(180) && end.x() == &Rat::from_i64(180) {
            start = Coord::xy(Rat::from_i64(-180), start.y().clone());
            end = Coord::xy(Rat::from_i64(-180), end.y().clone());
        }
        retain_coord(&start, context, retained)?;
        retain_coord(&end, context, retained)?;
        edges.push((start, end));
    }
    // These chart sentinels supply the poles and every open latitude band even
    // for constant Empty/Whole regions. They do not invent geometry boundaries.
    seeds.push(Coord::xy(Rat::from_i64(-180), Rat::from_i64(-90)));
    seeds.push(Coord::xy(Rat::from_i64(180), Rat::from_i64(90)));
    let borrowed = edges
        .iter()
        .map(|(start, end)| (start, end))
        .collect::<Vec<_>>();
    let events = super::boundary::identified_events(
        &borrowed,
        seeds
            .iter()
            .chain(borrowed.iter().flat_map(|&pair| <[_; 2]>::from(pair))),
        context,
        progress,
        retained,
    )?;
    let mut matrix = IntersectionMatrix::new();
    scan_faces(a, b, &events, &borrowed, &mut matrix, context, progress)?;
    // A finite point interior is completely enumerated here. Later positive
    // dimensional witnesses meet its exterior after removing finitely many
    // positions, even when a convenient midpoint is one of those positions.
    for point in a.points().iter().chain(b.points()) {
        record_witness(
            point.point(),
            a,
            b,
            Dim::Zero,
            &mut matrix,
            context,
            progress,
        )?;
    }
    for edge in borrowed.iter().copied() {
        let fragments = crate::topology::noding::fragments_admitted(
            edge, &events, context, progress, retained,
        )?;
        for (start, end) in fragments {
            let midpoint = crate::topology::midpoint_admitted(
                start,
                end,
                &mut ExactAdmission::new(context, progress),
            )?;
            let point = physical_point(&midpoint, context, progress)?;
            let dimension = if start.y() == end.y() && point.is_pole() {
                Dim::Zero
            } else {
                Dim::One
            };
            record_witness(&point, a, b, dimension, &mut matrix, context, progress)?;
        }
    }
    // Faces and open fragments can saturate a row at a greater dimension.
    // No subsequent isolated witness can change those entries, so avoid
    // repeating a complete polygon membership proof for each of its vertices.
    for event in &events {
        let point = physical_point(event, context, progress)?;
        record_witness(&point, a, b, Dim::Zero, &mut matrix, context, progress)?;
    }
    Ok(matrix)
}

fn original_contact_graph(
    source: &PreparedGeometry,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    context.charge_work(1)?;
    progress.context_poll(context)?;
    if !source.symbolic_points().is_empty() || source.curves().len() > 1 {
        // A complete physical fragment family deduplicates partial and repeated
        // overlaps before assigning its mod-two endpoint boundary.
        return Ok(true);
    }
    if let PreparedRegion::Polygons(polygons) | PreparedRegion::ComplementOfPolygons(polygons) =
        source.region()
    {
        for polygon in polygons.iter() {
            context.charge_work(1)?;
            progress.context_poll(context)?;
            // Native closed intersections can retain curves or isolated points;
            // their declared areal carrier is not a proof of dimension two.
            if polygon.chart().is_none() {
                return Ok(true);
            }
        }
    }
    for edge in source_edges(source) {
        context.charge_work(1)?;
        progress.context_poll(context)?;
        if !matches!(edge, PreparedEdge::SourceLinear(_)) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn point_region_matrix(
    a: &PreparedGeometry,
    b: &PreparedGeometry,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<IntersectionMatrix>, GeoError> {
    let (points, region, reverse) = if finite_points(a) && a.symbolic_points().is_empty() {
        (a, b, false)
    } else if finite_points(b) && b.symbolic_points().is_empty() {
        (b, a, true)
    } else {
        return Ok(None);
    };
    if !region.points().is_empty()
        || !region.symbolic_points().is_empty()
        || !region.curves().is_empty()
    {
        return Ok(None);
    }
    let possible: &[(Set, Dim)] = match region.region() {
        PreparedRegion::Empty => &[(Set::Exterior, Dim::Two)],
        PreparedRegion::Whole => &[(Set::Interior, Dim::Two)],
        PreparedRegion::Polygons(polygons) | PreparedRegion::ComplementOfPolygons(polygons)
            if polygons.len() == 1
                && polygons[0].oriented_interior().is_some()
                && polygons[0].rings().len() == 1 =>
        {
            // Preparation privately proves this ring is a nonconstant Jordan
            // separator of the sphere. Both open sides have dimension two and
            // its boundary dimension one, independently of selected orientation.
            &[
                (Set::Interior, Dim::Two),
                (Set::Boundary, Dim::One),
                (Set::Exterior, Dim::Two),
            ]
        }
        PreparedRegion::Polygons(_) | PreparedRegion::ComplementOfPolygons(_) => return Ok(None),
    };
    let mut matrix = IntersectionMatrix::new();
    let mut raise = |first, second, dimension| {
        if reverse {
            matrix.raise(second, first, dimension);
        } else {
            matrix.raise(first, second, dimension);
        }
    };
    for point in points.points() {
        let location =
            super::locate_with_progress(point.point(), region.region(), context, progress)?;
        raise(Set::Interior, location, Dim::Zero);
    }
    // Removing a finite point set cannot lower a positive topological dimension.
    // Every cell below is established by the retained Jordan proof, not sampling.
    for &(location, dimension) in possible {
        raise(Set::Exterior, location, dimension);
    }
    Ok(Some(matrix))
}

fn finite_points(source: &PreparedGeometry) -> bool {
    matches!(source.region(), PreparedRegion::Empty) && source.curves().is_empty()
}

fn record_witness(
    point: &LonLat,
    a: &PreparedGeometry,
    b: &PreparedGeometry,
    dimension: Dim,
    matrix: &mut IntersectionMatrix,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<(), GeoError> {
    let first_finite = finite_points(a);
    let second_finite = finite_points(b);
    let first = if first_finite && dimension > Dim::Zero {
        Some(Set::Exterior)
    } else if first_finite {
        Some(locate(point, a, context, progress)?)
    } else {
        None
    };
    if let Some(first) = first
        && Set::ALL
            .iter()
            .all(|&second| matrix.get(first, second) >= dimension)
    {
        return Ok(());
    }
    let second = if second_finite && dimension > Dim::Zero {
        Set::Exterior
    } else {
        locate(point, b, context, progress)?
    };
    if first.is_none()
        && Set::ALL
            .iter()
            .all(|&first| matrix.get(first, second) >= dimension)
    {
        return Ok(());
    }
    let first = match first {
        Some(first) => first,
        None => locate(point, a, context, progress)?,
    };
    matrix.raise(first, second, dimension);
    Ok(())
}

fn retain_coord(
    point: &Coord,
    context: &mut MetricContext,
    retained: &mut u64,
) -> Result<(), GeoError> {
    let bytes = (2 * size_of::<Coord>()) as u64
        + 2 * (point.x().allocated_bytes() + point.y().allocated_bytes()) as u64
        + 64;
    context.admit_workspace(bytes)?;
    *retained = retained
        .checked_add(bytes)
        .ok_or(GeoError::ArithmeticOverflow("geographic relation storage"))?;
    Ok(())
}

pub(crate) fn copy_point(
    point: &LonLat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Coord, GeoError> {
    ExactAdmission::new(context, progress).rational(
        ExactOperation::Linear,
        &[point.longitude(), point.latitude()],
        2,
        || {
            Ok(Coord::xy(
                point.longitude().clone(),
                point.latitude().clone(),
            ))
        },
    )
}

fn average(
    a: &Rat,
    b: &Rat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Rat, GeoError> {
    let mut admission = ExactAdmission::new(context, progress);
    let sum = admission.rational(ExactOperation::RationalAdd, &[a, b], 1, || Ok(a.add(b)))?;
    let two = Rat::from_i64(2);
    admission.rational(ExactOperation::RationalDivide, &[&sum, &two], 1, || {
        Ok(sum.div(&two).expect("positive divisor"))
    })
}

fn locate(
    point: &LonLat,
    source: &PreparedGeometry,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Set, GeoError> {
    let region = super::locate_with_progress(point, source.region(), context, progress)?;
    if region != Set::Exterior {
        // A lower-dimensional member already in an areal stratum cannot
        // promote a physical areal boundary to interior.
        return Ok(region);
    }
    let mut point_member = false;
    for member in source.points() {
        if ExactAdmission::new(context, progress).rational(
            ExactOperation::RationalCompare,
            &[
                point.longitude(),
                point.latitude(),
                member.point().longitude(),
                member.point().latitude(),
            ],
            8,
            || Ok(member.point().same_location(point)),
        )? {
            point_member = true;
        }
    }
    let mut on_curve = false;
    let mut odd_ends = false;
    for curve in source.curves() {
        for edge in curve.edges() {
            on_curve |= super::point::contact_with_progress(edge, point, context, progress)?;
        }
        let Some(first) = curve.edges().first() else {
            continue;
        };
        let Some(last) = curve.edges().last() else {
            continue;
        };
        let PreparedEdge::SourceLinear(last) = last else {
            unreachable!("written arrangement")
        };
        for endpoint in [
            first.start().expect("written start").point(),
            last.end().point(),
        ] {
            odd_ends ^= ExactAdmission::new(context, progress).rational(
                ExactOperation::RationalCompare,
                &[
                    point.longitude(),
                    point.latitude(),
                    endpoint.longitude(),
                    endpoint.latitude(),
                ],
                8,
                || Ok(endpoint.same_location(point)),
            )?;
        }
    }
    Ok(if on_curve {
        if odd_ends {
            Set::Boundary
        } else {
            Set::Interior
        }
    } else if point_member {
        Set::Interior
    } else {
        Set::Exterior
    })
}

fn sort_unique(
    values: &mut Vec<Rat>,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<(), GeoError> {
    purrdf_lex::walk::try_sort_unstable_by(values, |a, b| {
        ExactAdmission::new(context, progress).rational(
            ExactOperation::RationalCompare,
            &[a, b],
            1,
            || Ok(a.cmp(b)),
        )
    })?;
    // Rat equality compares canonical integer limbs; charge its complete scans
    // before the infallible in-place deduplication.
    let operands = values.iter().collect::<Vec<_>>();
    ExactAdmission::new(context, progress).rational(
        ExactOperation::Linear,
        &operands,
        values.len() as u64,
        || Ok(()),
    )?;
    values.dedup();
    Ok(())
}

fn scan_faces(
    a: &PreparedGeometry,
    b: &PreparedGeometry,
    events: &[Coord],
    edges: &[(&Coord, &Coord)],
    matrix: &mut IntersectionMatrix,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<(), GeoError> {
    let mut bands = Vec::with_capacity(events.len());
    for event in events {
        bands.push(ExactAdmission::new(context, progress).rational(
            ExactOperation::Linear,
            &[event.y()],
            1,
            || Ok(event.y().clone()),
        )?);
    }
    sort_unique(&mut bands, context, progress)?;
    for pair in bands.windows(2) {
        if faces_complete(a.region(), b.region(), matrix) {
            return Ok(());
        }
        let height = average(&pair[0], &pair[1], context, progress)?;
        let mut crossings = vec![Rat::from_i64(-180), Rat::from_i64(180)];
        for &(start, end) in edges {
            if let Some(crossing) = crate::topology::horizontal_crossing_admitted(
                start,
                end,
                &height,
                &mut ExactAdmission::new(context, progress),
            )? && ExactAdmission::new(context, progress).rational(
                ExactOperation::RationalCompare,
                &[&crossing],
                2,
                || Ok(crossing > Rat::from_i64(-180) && crossing < Rat::from_i64(180)),
            )? {
                crossings.push(crossing);
            }
        }
        sort_unique(&mut crossings, context, progress)?;
        for ends in crossings.windows(2) {
            if faces_complete(a.region(), b.region(), matrix) {
                return Ok(());
            }
            let longitude = average(&ends[0], &ends[1], context, progress)?;
            let point = LonLat::new(longitude, height.clone())?;
            // Noding and the strict scan partitions exclude every curve and
            // isolated point from this open face. Removing lower-dimensional
            // members cannot change its two-dimensional intersection class.
            let first = super::locate_with_progress(&point, a.region(), context, progress)?;
            let second = super::locate_with_progress(&point, b.region(), context, progress)?;
            if first == Set::Boundary || second == Set::Boundary {
                return Err(GeoError::PrecisionExhausted {
                    bits: context.policy().limits().max_precision_bits,
                });
            }
            matrix.raise(first, second, Dim::Two);
        }
    }
    Ok(())
}

fn faces_complete(a: &PreparedRegion, b: &PreparedRegion, matrix: &IntersectionMatrix) -> bool {
    fn possible(region: &PreparedRegion, set: Set) -> bool {
        match region {
            PreparedRegion::Empty => set == Set::Exterior,
            PreparedRegion::Whole => set == Set::Interior,
            PreparedRegion::Polygons(_) | PreparedRegion::ComplementOfPolygons(_) => true,
        }
    }
    [Set::Interior, Set::Exterior].into_iter().all(|first| {
        !possible(a, first)
            || [Set::Interior, Set::Exterior]
                .into_iter()
                .all(|second| !possible(b, second) || matrix.get(first, second) == Dim::Two)
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{
        Crs, GeoProfile, GeographicReference, OrientedInterior, PreparedCoordinate, PreparedCurve,
        PreparedPolygon,
    };

    fn source(text: &str) -> PreparedGeometry {
        let crs = Crs::new(purrdf_iri::vocab::ogc::CRS84).unwrap();
        let literal = crate::wkt::parse(text, &crs).unwrap();
        PreparedGeometry::from_literal(&literal, &GeoProfile::standard()).unwrap()
    }

    #[test]
    fn relation_admission_counts_original_objects_without_content_deduplication() {
        let first = source("POINT (0 0)");
        let second = source("POINT (0 0)");
        assert_eq!(first.id(), second.id());
        let original = (
            first.preparation_work_items(),
            first.retained_workspace_bytes(),
        );
        assert_eq!(relation_source_admission(&first, &first).unwrap(), original);
        assert_eq!(
            relation_source_admission(&first, &second).unwrap(),
            (original.0 * 2, original.1 * 2)
        );
    }

    fn shortest_curve(
        start: (i64, i64),
        end: (i64, i64),
        context: &mut MetricContext,
    ) -> PreparedGeometry {
        let coordinate = |(longitude, latitude)| {
            PreparedCoordinate::new(
                Coord::xy(Rat::from_i64(longitude), Rat::from_i64(latitude)),
                context.reference(),
            )
            .unwrap()
        };
        let edge = PreparedEdge::ShortestGeodesic(Box::new(
            crate::ShortestGeodesicArc::new(coordinate(start), coordinate(end), context).unwrap(),
        ));
        PreparedGeometry::from_parts(
            context.reference().clone(),
            Vec::new(),
            vec![PreparedCurve::new(vec![edge])],
            PreparedRegion::Empty,
            context.policy(),
        )
        .unwrap()
    }

    #[test]
    fn native_curves_node_crossings_overlaps_and_reversed_endpoint_boundaries() {
        let mut context = MetricContext::wgs84().unwrap();
        let first = shortest_curve((-10, 0), (10, 0), &mut context);
        for (start, end, expected) in [
            ((0, -10), (0, 10), "0F1FF0102"),
            ((0, 0), (20, 0), "1010F0102"),
            ((10, 0), (-10, 0), "1FFF0FFF2"),
            ((20, 5), (30, 5), "FF1FF0102"),
        ] {
            let second = shortest_curve(start, end, &mut context);
            let matrix = relate(&first, &second, &mut context).unwrap_or_else(|error| {
                panic!(
                    "case {start:?}->{end:?}: {error:?}, work={}",
                    context.work_items()
                )
            });
            assert_eq!(matrix.to_string(), expected);
            assert_eq!(
                relate(&second, &first, &mut context).unwrap(),
                crate::relations::transpose(&matrix)
            );
        }
    }

    #[test]
    fn native_curve_boundary_nodes_use_physical_pole_and_seam_aliases() {
        let mut context = MetricContext::wgs84().unwrap();
        let first = shortest_curve((0, 80), (0, 90), &mut context);
        let second = shortest_curve((60, 90), (60, 80), &mut context);
        assert_eq!(
            relate(&first, &second, &mut context).unwrap().to_string(),
            "FF1F00102"
        );
        let first = shortest_curve((170, 0), (180, 0), &mut context);
        let second = shortest_curve((-180, 0), (-170, 0), &mut context);
        assert_eq!(
            relate(&first, &second, &mut context).unwrap().to_string(),
            "FF1F00102"
        );
    }

    #[test]
    fn curved_equator_overlap_uses_the_original_written_formal_faces() {
        let mut context = MetricContext::wgs84().unwrap();
        let curve = shortest_curve((-10, 0), (10, 0), &mut context);
        let polygon = source("POLYGON ((-5 0,5 0,5 5,-5 5,-5 0))");
        assert_eq!(
            relate(&curve, &polygon, &mut context).unwrap().to_string(),
            "F11FF0212"
        );
        let short = shortest_curve((160, 0), (-160, 0), &mut context);
        let written = source("LINESTRING (-170 0,170 0)");
        assert_eq!(
            relate(&short, &written, &mut context).unwrap().to_string(),
            "1010FF1F2"
        );
    }

    fn mercator_mapping(context: &MetricContext) -> std::sync::Arc<crate::OperationChain> {
        use crate::operation::{
            CoordinateOperation, CoordinateUnit, OperationChain, OperationModel, OperationReference,
        };
        use purrdf_hash::hex::Digest32;
        use std::sync::Arc;
        let operation = CoordinateOperation::compile(
            OperationReference {
                realization: Digest32::new([3; 32]),
                unit: CoordinateUnit::Metres,
                swapped_axes: false,
            },
            OperationReference {
                realization: context.reference().id().digest(),
                unit: CoordinateUnit::Degrees,
                swapped_axes: false,
            },
            OperationModel::MercatorToGeographic {
                radius: Rat::from_i64(6_378_137),
                eccentricity_squared: Rat::zero(),
                square_domain: true,
            },
        )
        .unwrap();
        Arc::new(OperationChain::compile(vec![operation]).unwrap())
    }

    fn symbolic_point(longitude_source: i64, context: &MetricContext) -> PreparedGeometry {
        let point = crate::OperationImagePoint::new(
            Coord::xy(Rat::from_i64(longitude_source), Rat::zero()),
            mercator_mapping(context),
            context.reference().clone(),
            None,
            context.policy(),
        )
        .unwrap();
        PreparedGeometry::from_parts_with_symbolic(
            context.reference().clone(),
            Vec::new(),
            vec![point],
            Vec::new(),
            PreparedRegion::Empty,
            context.policy(),
        )
        .unwrap()
    }

    fn transformed_curve(
        start: (i64, i64),
        end: (i64, i64),
        constant: bool,
        context: &MetricContext,
    ) -> PreparedGeometry {
        use crate::operation::{
            CoordinateOperation, CoordinateUnit, OperationChain, OperationImageCurve,
            OperationModel, OperationReference, Polynomial2d, PolynomialTerm,
        };
        use purrdf_hash::hex::Digest32;
        let mut chain = mercator_mapping(context);
        if constant {
            let reference = OperationReference {
                realization: Digest32::new([3; 32]),
                unit: CoordinateUnit::Metres,
                swapped_axes: false,
            };
            let polynomial = Polynomial2d::new(
                0,
                [Rat::zero(), Rat::zero()],
                [Rat::one(), Rat::one()],
                vec![PolynomialTerm {
                    x_power: 0,
                    y_power: 0,
                    x_coefficient: Rat::zero(),
                    y_coefficient: Rat::zero(),
                }],
            )
            .unwrap();
            let mut operations = vec![
                CoordinateOperation::compile(
                    reference,
                    reference,
                    OperationModel::Polynomial2d(polynomial),
                )
                .unwrap(),
            ];
            operations.extend(chain.operations().iter().cloned());
            chain = std::sync::Arc::new(OperationChain::compile(operations).unwrap());
        }
        let coordinate = |(x, y)| Coord::xy(Rat::from_i64(x), Rat::from_i64(y));
        let image = OperationImageCurve::new(
            coordinate(start),
            coordinate(end),
            chain,
            context.reference().clone(),
            None,
            context.policy(),
        )
        .unwrap();
        PreparedGeometry::from_parts(
            context.reference().clone(),
            Vec::new(),
            vec![PreparedCurve::new(vec![PreparedEdge::Transformed(
                Box::new(image),
            )])],
            PreparedRegion::Empty,
            context.policy(),
        )
        .unwrap()
    }

    fn polynomial_image_curve(
        start: i64,
        end: i64,
        bent: bool,
        context: &MetricContext,
    ) -> PreparedGeometry {
        use crate::operation::{Polynomial2d, PolynomialTerm};
        let mut terms = vec![PolynomialTerm {
            x_power: 2,
            y_power: 0,
            x_coefficient: Rat::one(),
            y_coefficient: Rat::zero(),
        }];
        if bent {
            terms.push(PolynomialTerm {
                x_power: 4,
                y_power: 0,
                x_coefficient: Rat::zero(),
                y_coefficient: Rat::one(),
            });
        }
        let polynomial = Polynomial2d::new(
            if bent { 4 } else { 2 },
            [Rat::zero(), Rat::zero()],
            [Rat::one(), Rat::one()],
            terms,
        )
        .unwrap();
        polynomial_prefix_curve(start, end, polynomial, context)
    }

    fn polynomial_prefix_curve(
        start: i64,
        end: i64,
        polynomial: crate::operation::Polynomial2d,
        context: &MetricContext,
    ) -> PreparedGeometry {
        use crate::operation::{
            CoordinateOperation, CoordinateUnit, OperationChain, OperationImageCurve,
            OperationModel, OperationReference,
        };
        use std::sync::Arc;
        let reference = OperationReference {
            realization: purrdf_hash::hex::Digest32::new([3; 32]),
            unit: CoordinateUnit::Metres,
            swapped_axes: false,
        };
        let mut operations = vec![
            CoordinateOperation::compile(
                reference,
                reference,
                OperationModel::Polynomial2d(polynomial),
            )
            .unwrap(),
        ];
        operations.extend(mercator_mapping(context).operations().iter().cloned());
        let chain = Arc::new(OperationChain::compile(operations).unwrap());
        let image = OperationImageCurve::new(
            Coord::xy(Rat::from_i64(start), Rat::zero()),
            Coord::xy(Rat::from_i64(end), Rat::zero()),
            chain,
            context.reference().clone(),
            None,
            context.policy(),
        )
        .unwrap();
        PreparedGeometry::from_parts(
            context.reference().clone(),
            Vec::new(),
            vec![PreparedCurve::new(vec![PreparedEdge::Transformed(
                Box::new(image),
            )])],
            PreparedRegion::Empty,
            context.policy(),
        )
        .unwrap()
    }

    #[test]
    fn exact_polynomial_retraversal_uses_the_selected_set_without_changing_source_metrics() {
        let mut context = MetricContext::wgs84().unwrap();
        for bent in [false, true] {
            let folded = polynomial_image_curve(-1, 1, bent, &context);
            let forward = polynomial_image_curve(0, 1, bent, &context);
            let reversed = polynomial_image_curve(1, 0, bent, &context);
            let id = folded.id();
            assert_equivalent(&folded, &forward, Dim::One, &mut context);
            assert_equivalent(&forward, &folded, Dim::One, &mut context);
            assert_equivalent(&folded, &reversed, Dim::One, &mut context);
            assert_equivalent(&folded, &folded, Dim::One, &mut context);
            let origin = source("POINT (0 0)");
            assert_eq!(
                relate(&folded, &origin, &mut context)
                    .unwrap_or_else(|error| panic!("origin contact, bent={bent}: {error:?}"))
                    .to_string(),
                "FF10F0FF2"
            );
            assert_eq!(
                relate(&origin, &folded, &mut context).unwrap().to_string(),
                "F0FFFF102"
            );
            assert_eq!(folded.id(), id);
        }
    }

    #[test]
    fn nonsymmetric_polynomial_critical_branches_have_the_complete_selected_range() {
        use crate::operation::{Polynomial2d, PolynomialTerm};
        let mut context = MetricContext::wgs84().unwrap();
        let cubic = Polynomial2d::new(
            3,
            [Rat::zero(), Rat::zero()],
            [Rat::one(), Rat::one()],
            vec![
                PolynomialTerm {
                    x_power: 1,
                    y_power: 0,
                    x_coefficient: Rat::from_i64(-3),
                    y_coefficient: Rat::zero(),
                },
                PolynomialTerm {
                    x_power: 3,
                    y_power: 0,
                    x_coefficient: Rat::one(),
                    y_coefficient: Rat::zero(),
                },
            ],
        )
        .unwrap();
        let folded = polynomial_prefix_curve(-2, 2, cubic, &context);
        let control = transformed_curve((-2, 0), (2, 0), false, &context);
        let id = folded.id();
        assert_equivalent(&folded, &control, Dim::One, &mut context);
        assert_equivalent(&control, &folded, Dim::One, &mut context);
        assert_equivalent(&folded, &folded, Dim::One, &mut context);
        for x in [-2, 0, 2] {
            let point = symbolic_point(x, &context);
            let expected = relate(&control, &point, &mut context).unwrap();
            assert_eq!(relate(&folded, &point, &mut context).unwrap(), expected);
            assert_eq!(
                relate(&point, &folded, &mut context).unwrap(),
                crate::relations::transpose(&expected)
            );
        }
        assert_eq!(folded.id(), id);
    }

    #[test]
    fn a_single_nonaxis_polynomial_loop_has_the_complete_selected_contact_graph() {
        use crate::operation::{Polynomial2d, PolynomialTerm};
        let mut context = MetricContext::wgs84().unwrap();
        let mut limits = *context.policy().limits();
        limits.max_work_items = 4_000_000;
        context = MetricContext::new(
            context.reference().clone(),
            crate::ExecutionPolicy::new(limits).unwrap(),
        )
        .unwrap();
        let polynomial = Polynomial2d::new(
            3,
            [Rat::zero(), Rat::zero()],
            [Rat::one(), Rat::one()],
            vec![
                PolynomialTerm {
                    x_power: 0,
                    y_power: 0,
                    x_coefficient: Rat::from_i64(-1),
                    y_coefficient: Rat::zero(),
                },
                PolynomialTerm {
                    x_power: 1,
                    y_power: 0,
                    x_coefficient: Rat::zero(),
                    y_coefficient: Rat::from_i64(-1),
                },
                PolynomialTerm {
                    x_power: 2,
                    y_power: 0,
                    x_coefficient: Rat::one(),
                    y_coefficient: Rat::zero(),
                },
                PolynomialTerm {
                    x_power: 3,
                    y_power: 0,
                    x_coefficient: Rat::zero(),
                    y_coefficient: Rat::one(),
                },
            ],
        )
        .unwrap();
        let complete = polynomial_prefix_curve(-2, 2, polynomial.clone(), &context);
        let first = polynomial_prefix_curve(-2, 0, polynomial.clone(), &context);
        let second = polynomial_prefix_curve(0, 2, polynomial, &context);
        let split = PreparedGeometry::from_parts(
            context.reference().clone(),
            Vec::new(),
            vec![PreparedCurve::new(vec![
                first.curves()[0].edges()[0].clone(),
                second.curves()[0].edges()[0].clone(),
            ])],
            PreparedRegion::Empty,
            context.policy(),
        )
        .unwrap();
        let id = complete.id();
        assert_equivalent(&complete, &split, Dim::One, &mut context);
        assert_equivalent(&split, &complete, Dim::One, &mut context);
        assert_equivalent(&complete, &complete, Dim::One, &mut context);
        let crossing = source("POINT (0 0)");
        assert_eq!(
            relate(&complete, &crossing, &mut context)
                .unwrap()
                .to_string(),
            "0F1FF0FF2"
        );
        assert_eq!(
            relate(&crossing, &complete, &mut context)
                .unwrap()
                .to_string(),
            "0FFFFF102"
        );
        assert_eq!(complete.id(), id);
    }

    #[test]
    fn repeated_equatorial_coverage_has_no_artificial_endpoint_boundary() {
        let mut context = MetricContext::wgs84().unwrap();
        let circle = source("LINESTRING (-180 0,180 0)");
        for (azimuth, distance) in [(90, 43_000_000), (-90, 80_000_000), (450, 80_000_000)] {
            let start = PreparedCoordinate::new(
                Coord::xy(Rat::from_i64(17), Rat::zero()),
                context.reference(),
            )
            .unwrap();
            let arc = crate::AzimuthLengthArc::new(
                start,
                Rat::from_i64(azimuth),
                crate::Metres::new(Rat::from_i64(distance)),
                &mut context,
            )
            .unwrap();
            let original_length = arc.length().clone();
            let selected = PreparedGeometry::from_parts(
                context.reference().clone(),
                Vec::new(),
                vec![PreparedCurve::new(vec![PreparedEdge::AzimuthLength(
                    Box::new(arc),
                )])],
                PreparedRegion::Empty,
                context.policy(),
            )
            .unwrap();
            let id = selected.id();
            assert_eq!(
                relate(&selected, &circle, &mut context)
                    .unwrap()
                    .to_string(),
                "1FFFFFFF2"
            );
            assert_equivalent(&circle, &selected, Dim::One, &mut context);
            assert_equivalent(&selected, &selected, Dim::One, &mut context);
            for point in ["POINT (17 0)", "POINT (-180 0)", "POINT (0 0)"] {
                assert_eq!(
                    relate(&selected, &source(point), &mut context)
                        .unwrap()
                        .to_string(),
                    "0F1FFFFF2"
                );
            }
            assert_eq!(selected.id(), id);
            let PreparedEdge::AzimuthLength(arc) = &selected.curves()[0].edges()[0] else {
                unreachable!()
            };
            assert_eq!(arc.length(), &original_length);
        }
    }

    #[test]
    fn nonaxis_direct_arcs_exclude_the_parameter_diagonal_without_losing_endpoints() {
        let mut context = MetricContext::wgs84().unwrap();
        let start = PreparedCoordinate::new(
            Coord::xy(Rat::from_i64(17), Rat::from_i64(20)),
            context.reference(),
        )
        .unwrap();
        let arc = crate::AzimuthLengthArc::new(
            start,
            Rat::from_i64(43),
            crate::Metres::new(Rat::from_i64(1_000_000)),
            &mut context,
        )
        .unwrap();
        let selected = PreparedGeometry::from_parts(
            context.reference().clone(),
            Vec::new(),
            vec![PreparedCurve::new(vec![PreparedEdge::AzimuthLength(
                Box::new(arc),
            )])],
            PreparedRegion::Empty,
            context.policy(),
        )
        .unwrap();
        let id = selected.id();
        assert_eq!(
            relate(&selected, &selected, &mut context)
                .unwrap()
                .to_string(),
            "1FFF0FFF2"
        );
        assert_eq!(
            relate(&selected, &source("POINT (17 20)"), &mut context)
                .unwrap()
                .to_string(),
            "FF10F0FF2"
        );
        assert_eq!(selected.id(), id);
    }

    #[test]
    fn repeated_meridian_coverage_uses_original_pole_frames_and_has_no_boundary() {
        let mut context = MetricContext::wgs84().unwrap();
        for (longitude, latitude, azimuth, plane) in [
            (17, 20, 0, 17),
            (17, 20, 180, 17),
            (0, -90, 30, 30),
            (0, 90, 30, 150),
        ] {
            let start = PreparedCoordinate::new(
                Coord::xy(Rat::from_i64(longitude), Rat::from_i64(latitude)),
                context.reference(),
            )
            .unwrap();
            let length = Rat::from_i64(43_000_000);
            let arc = crate::AzimuthLengthArc::new(
                start,
                Rat::from_i64(azimuth),
                crate::Metres::new(length.clone()),
                &mut context,
            )
            .unwrap();
            let selected = PreparedGeometry::from_parts(
                context.reference().clone(),
                Vec::new(),
                vec![PreparedCurve::new(vec![PreparedEdge::AzimuthLength(
                    Box::new(arc),
                )])],
                PreparedRegion::Empty,
                context.policy(),
            )
            .unwrap();
            let opposite = if plane > 0 { plane - 180 } else { plane + 180 };
            let circle = source(&format!(
                "MULTILINESTRING (({plane} -90,{plane} 90),({opposite} 90,{opposite} -90))"
            ));
            let id = selected.id();
            let before = crate::ellipsoidal::length(&selected, &mut context).unwrap();
            assert_eq!(before.exact(), &length);
            assert_equivalent(&selected, &circle, Dim::One, &mut context);
            assert_equivalent(&selected, &selected, Dim::One, &mut context);
            for latitude in [-90, 0, 90] {
                let point = source(&format!("POINT ({plane} {latitude})"));
                assert_eq!(
                    relate(&selected, &point, &mut context).unwrap().to_string(),
                    "0F1FFFFF2"
                );
            }
            assert_eq!(
                crate::ellipsoidal::length(&selected, &mut context).unwrap(),
                before
            );
            assert_eq!(selected.id(), id);
        }
    }

    #[test]
    fn transformed_constant_images_have_point_dimension_despite_distinct_source_endpoints() {
        let mut context = MetricContext::wgs84().unwrap();
        let image = transformed_curve((-7, 3), (11, 5), true, &context);
        let point = source("POINT (0 0)");
        let matrix = relate(&image, &point, &mut context).unwrap();
        assert_eq!(matrix.to_string(), "0FFFFFFF2");
        assert_eq!(matrix.input_dimensions(), [Dim::Zero; 2]);
        assert_eq!(dimension(&image), Some(1));
        assert_eq!(
            relate(&point, &image, &mut context).unwrap(),
            crate::relations::transpose(&matrix)
        );
        let separated = source("POINT (1 1)");
        assert_eq!(
            relate(&image, &separated, &mut context)
                .unwrap()
                .to_string(),
            "FF0FFF0F2"
        );
    }

    #[test]
    fn original_transformed_and_nonaxis_shortest_curves_node_all_transverse_contacts() {
        let mut context = MetricContext::wgs84().unwrap();
        let first = transformed_curve((-10, -10), (10, 10), false, &context);
        let second = transformed_curve((-10, 10), (10, -10), false, &context);
        let matrix = relate(&first, &second, &mut context).unwrap();
        assert_eq!(matrix.to_string(), "0F1FF0102");
        assert_eq!(
            relate(&second, &first, &mut context).unwrap(),
            crate::relations::transpose(&matrix)
        );
        let first = shortest_curve((-10, -10), (10, 10), &mut context);
        let second = shortest_curve((-10, 10), (10, -10), &mut context);
        let matrix = relate(&first, &second, &mut context).unwrap();
        assert_eq!(matrix.to_string(), "0F1FF0102");
        assert_eq!(
            relate(&second, &first, &mut context).unwrap(),
            crate::relations::transpose(&matrix)
        );
    }

    fn shortest_polygon(vertices: &[(i64, i64)], context: &mut MetricContext) -> PreparedGeometry {
        let mut edges = Vec::new();
        for pair in vertices.windows(2) {
            let coordinate = |(longitude, latitude)| {
                PreparedCoordinate::new(
                    Coord::xy(Rat::from_i64(longitude), Rat::from_i64(latitude)),
                    context.reference(),
                )
                .unwrap()
            };
            edges.push(PreparedEdge::ShortestGeodesic(Box::new(
                crate::ShortestGeodesicArc::new(coordinate(pair[0]), coordinate(pair[1]), context)
                    .unwrap(),
            )));
        }
        let polygon = PreparedPolygon::from_curves(
            vec![PreparedCurve::new(edges)],
            OrientedInterior::Left,
            context,
        )
        .unwrap();
        PreparedGeometry::from_parts(
            context.reference().clone(),
            Vec::new(),
            Vec::new(),
            PreparedRegion::polygons(vec![polygon]),
            context.policy(),
        )
        .unwrap()
    }

    #[test]
    fn native_geodesic_regions_node_original_faces_complements_and_transverse_boundaries() {
        let reference = GeographicReference::wgs84();
        let policy = crate::ExecutionPolicy::new(crate::ExecutionLimits {
            max_work_items: 20_000_000,
            ..*crate::ExecutionPolicy::geometry().limits()
        })
        .unwrap();
        let mut context = MetricContext::new(reference, policy).unwrap();
        let first = shortest_polygon(
            &[(-4, -4), (4, -4), (4, 4), (-4, 4), (-4, -4)],
            &mut context,
        );
        assert_eq!(
            relate(&first, &first, &mut context).unwrap().to_string(),
            "2FFF1FFF2"
        );
        let opposite = PreparedGeometry::from_parts(
            context.reference().clone(),
            Vec::new(),
            Vec::new(),
            first.region().clone().complement(),
            context.policy(),
        )
        .unwrap();
        assert_eq!(
            relate(&first, &opposite, &mut context).unwrap().to_string(),
            "FF2F1F2FF"
        );
        let second = shortest_polygon(&[(0, 0), (8, 0), (8, 8), (0, 8), (0, 0)], &mut context);
        let matrix = relate(&first, &second, &mut context).unwrap();
        assert_eq!(matrix.to_string(), "212101212");
        assert_eq!(
            relate(&second, &first, &mut context).unwrap(),
            crate::relations::transpose(&matrix)
        );
    }

    #[test]
    fn symbolic_points_retain_original_equality_and_prove_positive_separation() {
        let mut context = MetricContext::wgs84().unwrap();
        let first = symbolic_point(0, &context);
        let same = symbolic_point(0, &context);
        assert_eq!(
            relate(&first, &same, &mut context).unwrap().to_string(),
            "0FFFFFFF2"
        );
        let other = symbolic_point(1, &context);
        assert_eq!(
            relate(&first, &other, &mut context).unwrap().to_string(),
            "FF0FFF0F2"
        );
        let polygon = source("POLYGON ((-2 -2,2 -2,2 2,-2 2,-2 -2))");
        assert_eq!(
            relate(&first, &polygon, &mut context).unwrap().to_string(),
            "0FFFFF212"
        );
    }

    pub(crate) fn closed_intersection(
        rings: &[&[(i64, i64)]],
        context: &mut MetricContext,
    ) -> PreparedGeometry {
        let rings = rings
            .iter()
            .map(|ring| {
                let coordinates = ring
                    .iter()
                    .map(|(lon, lat)| Coord::xy(Rat::from_i64(*lon), Rat::from_i64(*lat)))
                    .collect::<Vec<_>>();
                PreparedCurve::from_source(&coordinates, context.reference()).unwrap()
            })
            .collect();
        let polygon = PreparedPolygon::from_curves(rings, OrientedInterior::Left, context).unwrap();
        PreparedGeometry::from_parts(
            context.reference().clone(),
            Vec::new(),
            Vec::new(),
            PreparedRegion::polygons(vec![polygon]),
            context.policy(),
        )
        .unwrap()
    }

    fn assert_equivalent(
        a: &PreparedGeometry,
        b: &PreparedGeometry,
        dimension: Dim,
        context: &mut MetricContext,
    ) {
        let matrix = relate(a, b, context).unwrap();
        assert_eq!(matrix.input_dimensions(), [dimension; 2]);
        assert_eq!(matrix.get(Set::Interior, Set::Interior), dimension);
        assert_eq!(matrix.get(Set::Interior, Set::Boundary), Dim::Empty);
        assert_eq!(matrix.get(Set::Boundary, Set::Interior), Dim::Empty);
        assert_eq!(matrix.get(Set::Interior, Set::Exterior), Dim::Empty);
        assert_eq!(matrix.get(Set::Boundary, Set::Exterior), Dim::Empty);
        assert_eq!(matrix.get(Set::Exterior, Set::Interior), Dim::Empty);
        assert_eq!(matrix.get(Set::Exterior, Set::Boundary), Dim::Empty);
        assert_eq!(
            relate(b, a, context).unwrap(),
            crate::relations::transpose(&matrix)
        );
    }

    fn selected_support(source: &PreparedGeometry, context: &MetricContext) -> PreparedGeometry {
        let polygon =
            PreparedPolygon::from_selected_support(source.curves().to_vec(), context.reference())
                .unwrap();
        PreparedGeometry::from_parts(
            context.reference().clone(),
            Vec::new(),
            Vec::new(),
            PreparedRegion::polygons(vec![polygon]),
            context.policy(),
        )
        .unwrap()
    }

    #[test]
    fn rank_lost_regions_retain_the_complete_selected_curve_and_point_support() {
        let policy = crate::ExecutionPolicy::new(crate::ExecutionLimits {
            max_work_items: 4_000_000,
            ..*crate::ExecutionPolicy::geometry().limits()
        })
        .unwrap();
        let mut context = MetricContext::new(GeographicReference::wgs84(), policy).unwrap();
        for (curve, dimension) in [
            (source("LINESTRING (0 0,0 0)"), Dim::Zero),
            (source("LINESTRING (0 0,1 0)"), Dim::One),
            (source("MULTILINESTRING ((0 0,1 0),(2 0,3 0))"), Dim::One),
            (
                transformed_curve((-2, 0), (2, 0), true, &context),
                Dim::Zero,
            ),
            (polynomial_image_curve(-1, 1, false, &context), Dim::One),
            (polynomial_image_curve(-1, 1, true, &context), Dim::One),
        ] {
            let id = curve.id();
            let support = selected_support(&curve, &context);
            assert_equivalent(&support, &curve, dimension, &mut context);
            assert_equivalent(&support, &support, dimension, &mut context);
            assert_eq!(curve.id(), id);
            let boundary =
                super::super::arcs::arrangement::native_boundary(support.region(), &mut context)
                    .unwrap();
            assert!(!boundary.has_areal_faces());
            assert!(boundary.fragments().iter().all(|fragment| {
                fragment.stratum()
                    == super::super::arcs::arrangement::SelectedFragmentStratum::CurveInterior
            }));
            if dimension == Dim::Zero {
                assert!(boundary.fragments().is_empty());
                assert_eq!(boundary.isolated_points().len(), 1);
            } else {
                assert!(!boundary.fragments().is_empty());
            }
            let whole = PreparedGeometry::from_parts(
                context.reference().clone(),
                Vec::new(),
                Vec::new(),
                PreparedRegion::Whole,
                policy,
            )
            .unwrap();
            let complement = PreparedGeometry::from_parts(
                context.reference().clone(),
                Vec::new(),
                Vec::new(),
                support.region().clone().complement(),
                policy,
            )
            .unwrap();
            assert_equivalent(&complement, &whole, Dim::Two, &mut context);
        }
        let curve = source("LINESTRING (0 0,1 0)");
        let support = selected_support(&curve, &context);
        let area = source("POLYGON ((-1 -1,2 -1,2 1,-1 1,-1 -1))");
        let (PreparedRegion::Polygons(support_polygons), PreparedRegion::Polygons(area_polygons)) =
            (support.region(), area.region())
        else {
            unreachable!();
        };
        let covered = PreparedGeometry::from_parts(
            context.reference().clone(),
            Vec::new(),
            Vec::new(),
            PreparedRegion::polygons(vec![support_polygons[0].clone(), area_polygons[0].clone()]),
            policy,
        )
        .unwrap();
        assert_equivalent(&covered, &area, Dim::Two, &mut context);
        let own_complement = PreparedGeometry::from_parts(
            context.reference().clone(),
            Vec::new(),
            Vec::new(),
            PreparedRegion::polygons(vec![
                support_polygons[0]
                    .clone()
                    .with_interior(crate::RegionInterior::Complement),
            ]),
            policy,
        )
        .unwrap();
        assert_eq!(
            relate(&own_complement, &own_complement, &mut context)
                .unwrap()
                .to_string(),
            "2FFFFFFFF"
        );
    }

    #[test]
    fn native_closed_intersections_retain_boundary_only_curves_and_points() {
        let mut context = MetricContext::wgs84().unwrap();
        let region = closed_intersection(
            &[&[(-180, 0), (180, 0)], &[(180, 0), (-180, 0)]],
            &mut context,
        );
        let curve = shortest_curve((-10, 0), (10, 0), &mut context);
        let matrix = relate(&curve, &region, &mut context).unwrap();
        assert_eq!(matrix.to_string(), "1FF0FF1F2");
        assert_eq!(
            relate(&region, &curve, &mut context).unwrap(),
            crate::relations::transpose(&matrix)
        );
        let circle = source("LINESTRING (-180 0,180 0)");
        assert_equivalent(&region, &circle, Dim::One, &mut context);
        assert_eq!(
            relate(&region, &region, &mut context).unwrap().to_string(),
            "1FFFFFFF2"
        );
        let empty = source("GEOMETRYCOLLECTION EMPTY");
        assert_eq!(
            relate(&region, &empty, &mut context).unwrap().to_string(),
            "FF1FFFFF2"
        );
        let complement = PreparedGeometry::from_parts(
            context.reference().clone(),
            Vec::new(),
            Vec::new(),
            region.region().clone().complement(),
            context.policy(),
        )
        .unwrap();
        let whole = PreparedGeometry::from_parts(
            context.reference().clone(),
            Vec::new(),
            Vec::new(),
            PreparedRegion::Whole,
            context.policy(),
        )
        .unwrap();
        assert_equivalent(&complement, &whole, Dim::Two, &mut context);
    }

    #[test]
    fn isolated_native_nodes_and_selected_curve_endpoints_use_relative_strata() {
        let mut default = MetricContext::wgs84().unwrap();
        let isolated = closed_intersection(
            &[
                &[(0, 0), (1, 0), (1, 1), (0, 1), (0, 0)],
                &[(1, 1), (2, 1), (2, 2), (1, 2), (1, 1)],
            ],
            &mut default,
        );
        let point = source("POINT (1 1)");
        assert_eq!(
            relate(&point, &isolated, &mut default).unwrap().to_string(),
            "0FFFFFFF2"
        );
        // Admit the larger equivalent-set inventory explicitly.
        let policy = crate::ExecutionPolicy::new(crate::ExecutionLimits {
            max_work_items: 2_000_000,
            ..*default.policy().limits()
        })
        .unwrap();
        let mut context = MetricContext::new(default.reference().clone(), policy).unwrap();
        assert_eq!(
            crate::atlas::locate(point.points()[0].point(), isolated.region(), &mut context)
                .unwrap(),
            Set::Boundary
        );
        assert_equivalent(&isolated, &point, Dim::Zero, &mut context);
        assert_eq!(
            relate(&isolated, &isolated, &mut context)
                .unwrap()
                .to_string(),
            "0FFFFFFF2"
        );
        let empty = source("GEOMETRYCOLLECTION EMPTY");
        assert_eq!(
            relate(&isolated, &empty, &mut context).unwrap().to_string(),
            "FF0FFFFF2"
        );
        let complement = PreparedGeometry::from_parts(
            context.reference().clone(),
            Vec::new(),
            Vec::new(),
            isolated.region().clone().complement(),
            policy,
        )
        .unwrap();
        let whole = PreparedGeometry::from_parts(
            context.reference().clone(),
            Vec::new(),
            Vec::new(),
            PreparedRegion::Whole,
            policy,
        )
        .unwrap();
        assert_equivalent(&complement, &whole, Dim::Two, &mut context);
        let segment = closed_intersection(
            &[
                &[(-1, 0), (0, 0), (0, 1), (-1, 1), (-1, 0)],
                &[(0, 0), (1, 0), (1, 1), (0, 1), (0, 0)],
            ],
            &mut context,
        );
        let line = source("LINESTRING (0 0,0 1)");
        assert_equivalent(&segment, &line, Dim::One, &mut context);
        let endpoint = source("POINT (0 0)");
        assert_eq!(
            relate(&segment, &endpoint, &mut context)
                .unwrap()
                .to_string(),
            "FF10F0FF2"
        );
    }

    #[test]
    fn native_mixed_dimensional_sets_ignore_redundant_lower_members() {
        let policy = crate::ExecutionPolicy::new(crate::ExecutionLimits {
            max_work_items: 2_000_000,
            ..*crate::ExecutionPolicy::geometry().limits()
        })
        .unwrap();
        let mut context = MetricContext::new(GeographicReference::wgs84(), policy).unwrap();
        let isolated = closed_intersection(
            &[
                &[(0, 0), (1, 0), (1, 1), (0, 1), (0, 0)],
                &[(1, 1), (2, 1), (2, 2), (1, 2), (1, 1)],
            ],
            &mut context,
        );
        let area = source("POLYGON ((10 10,11 10,11 11,10 11,10 10))");
        let PreparedRegion::Polygons(isolated_polygons) = isolated.region() else {
            panic!("selected native point")
        };
        let PreparedRegion::Polygons(area_polygons) = area.region() else {
            panic!("written area")
        };
        let selected = PreparedGeometry::from_parts(
            context.reference().clone(),
            Vec::new(),
            Vec::new(),
            PreparedRegion::polygons(vec![isolated_polygons[0].clone(), area_polygons[0].clone()]),
            policy,
        )
        .unwrap();
        let carrier =
            source("GEOMETRYCOLLECTION (POINT (1 1),POLYGON ((10 10,11 10,11 11,10 11,10 10)))");
        assert_equivalent(&selected, &carrier, Dim::Two, &mut context);
        let boundary_point = source("POINT (10 10)");
        let redundant = PreparedGeometry::from_parts(
            context.reference().clone(),
            boundary_point.points().to_vec(),
            area.curves().to_vec(),
            area.region().clone(),
            policy,
        )
        .unwrap();
        // Adding a point already in a two-dimensional boundary changes no set.
        assert_equivalent(
            &selected,
            &PreparedGeometry::from_parts(
                context.reference().clone(),
                Vec::new(),
                Vec::new(),
                PreparedRegion::polygons(vec![
                    isolated_polygons[0].clone(),
                    area_polygons[0].clone(),
                ]),
                policy,
            )
            .unwrap(),
            Dim::Two,
            &mut context,
        );
        assert_equivalent(&area, &redundant, Dim::Two, &mut context);
    }

    #[test]
    fn identified_cut_meridians_and_poles_have_physical_matrices() {
        let mut context = MetricContext::wgs84().unwrap();
        let a = source("LINESTRING (180 -10,180 10)");
        let b = source("LINESTRING (-180 -5,-180 5)");
        assert_eq!(
            relate(&a, &b, &mut context).unwrap().to_string(),
            "101FF0FF2"
        );
        let a = source("POINT (50 90)");
        let b = source("POINT (-20 90)");
        assert_eq!(
            relate(&a, &b, &mut context).unwrap().to_string(),
            "0FFFFFFF2"
        );
    }

    #[test]
    fn written_long_paths_holes_and_native_parallel_regions_share_membership() {
        let mut context = MetricContext::wgs84().unwrap();
        let long = source("LINESTRING (170 0,-170 0)");
        let origin = source("POINT (0 0)");
        let opposite = source("POINT (180 0)");
        assert_eq!(
            relate(&long, &origin, &mut context)
                .unwrap()
                .get(Set::Interior, Set::Interior),
            Dim::Zero
        );
        assert_eq!(
            relate(&long, &opposite, &mut context)
                .unwrap()
                .get(Set::Interior, Set::Interior),
            Dim::Empty
        );
        let hole = source("POLYGON ((-2 -2,2 -2,2 2,-2 2,-2 -2),(-1 -1,1 -1,1 1,-1 1,-1 -1))");
        assert_eq!(
            relate(&origin, &hole, &mut context).unwrap().to_string(),
            "FF0FFF212"
        );
        let reference = GeographicReference::wgs84();
        let ring = PreparedCurve::from_source(
            &[
                Coord::xy(Rat::from_i64(-180), Rat::from_i64(30)),
                Coord::xy(Rat::from_i64(180), Rat::from_i64(30)),
            ],
            &reference,
        )
        .unwrap();
        let polygon =
            PreparedPolygon::from_curves(vec![ring], OrientedInterior::Left, &mut context).unwrap();
        let cap = PreparedGeometry::from_parts(
            reference,
            Vec::new(),
            Vec::new(),
            PreparedRegion::polygons(vec![polygon]),
            context.policy(),
        )
        .unwrap();
        let pole = source("POINT (0 90)");
        assert_eq!(
            relate(&cap, &pole, &mut context).unwrap().to_string(),
            "0F2FF1FF2"
        );
    }

    #[test]
    fn finite_points_use_original_geodesic_jordan_proof() {
        // Curved winding proofs are intentionally admitted by the caller; the
        // output law and matrix do not change with an adequate work allowance.
        let mut limits = *crate::ExecutionPolicy::geometry().limits();
        limits.max_work_items = 2_000_000;
        let mut context = MetricContext::new(
            GeographicReference::wgs84(),
            crate::ExecutionPolicy::new(limits).unwrap(),
        )
        .unwrap();
        let vertices = [(0, 0), (90, 0), (90, 90), (0, 0)];
        let mut edges = Vec::new();
        for pair in vertices.windows(2) {
            let coordinate = |(longitude, latitude)| {
                PreparedCoordinate::new(
                    Coord::xy(Rat::from_i64(longitude), Rat::from_i64(latitude)),
                    context.reference(),
                )
                .unwrap()
            };
            let start = coordinate(pair[0]);
            let end = coordinate(pair[1]);
            edges.push(PreparedEdge::ShortestGeodesic(Box::new(
                crate::ShortestGeodesicArc::new(start, end, &mut context).unwrap(),
            )));
        }
        let polygon = PreparedPolygon::from_curves(
            vec![PreparedCurve::new(edges)],
            OrientedInterior::Left,
            &mut context,
        )
        .unwrap();
        let region = PreparedGeometry::from_parts(
            context.reference().clone(),
            Vec::new(),
            Vec::new(),
            PreparedRegion::polygons(vec![polygon]),
            context.policy(),
        )
        .unwrap();
        for (text, expected) in [
            ("POINT (30 30)", "0FFFFF212"),
            ("POINT (-30 30)", "FF0FFF212"),
            ("POINT (0 90)", "F0FFFF212"),
        ] {
            let point = source(text);
            let forward = relate(&point, &region, &mut context).unwrap();
            assert_eq!(forward.to_string(), expected);
            let reverse = relate(&region, &point, &mut context).unwrap();
            for first in Set::ALL {
                for second in Set::ALL {
                    assert_eq!(forward.get(first, second), reverse.get(second, first));
                }
            }
        }
    }

    #[test]
    fn curved_relation_refuses_before_allocating_and_preserves_midphase_cancellation() {
        let mut preparing = MetricContext::wgs84().unwrap();
        let first = shortest_curve((-10, -10), (10, 10), &mut preparing);
        let second = shortest_curve((-10, 10), (10, -10), &mut preparing);
        let instrument = purrdf_alloc_probe::CurrentThreadWindow::open();
        std::hint::black_box(Vec::<u8>::with_capacity(512));
        assert_eq!(instrument.close().allocations, 1);
        for memory in [false, true] {
            let mut limits = *crate::ExecutionPolicy::geometry().limits();
            if memory {
                limits.max_workspace_bytes = 64;
            } else {
                limits.max_work_items = 1;
            }
            let mut context = MetricContext::new(
                GeographicReference::wgs84(),
                crate::ExecutionPolicy::new(limits).unwrap(),
            )
            .unwrap();
            let window = purrdf_alloc_probe::CurrentThreadWindow::open();
            let result = relate(&first, &second, &mut context);
            let measurement = window.close();
            assert_eq!(measurement.allocations, 0);
            if memory {
                assert!(matches!(result, Err(GeoError::MemoryExhausted { .. })));
            } else {
                assert!(matches!(result, Err(GeoError::WorkExhausted { .. })));
            }
        }
        let mut complete = MetricContext::wgs84().unwrap();
        let expected = relate(&first, &second, &mut complete).unwrap();
        struct StopAt {
            stop: u64,
            charged: u64,
            calls: u64,
        }
        impl MetricWorkObserver for StopAt {
            fn charge_chunk(&mut self, work: u64, _: u64) -> Result<(), GeoError> {
                self.charged += work;
                self.calls += 1;
                if self.charged >= self.stop {
                    Err(GeoError::Cancelled)
                } else {
                    Ok(())
                }
            }
        }
        for stop in [1, complete.work_items() / 3, 2 * complete.work_items() / 3] {
            let mut context = MetricContext::wgs84().unwrap();
            let mut observer = StopAt {
                stop,
                charged: 0,
                calls: 0,
            };
            assert_eq!(
                relate_metered(&first, &second, &mut context, &mut observer),
                Err(GeoError::Cancelled)
            );
            assert!(observer.charged >= stop);
            assert!(observer.calls > 1);
            assert_eq!(
                context.current_workspace_bytes(),
                context.retained_workspace_bytes()
            );
            assert_eq!(relate(&first, &second, &mut context).unwrap(), expected);
        }
    }

    #[test]
    fn whole_surface_has_no_exterior_and_refusals_publish_no_matrix() {
        let mut context = MetricContext::wgs84().unwrap();
        let whole = PreparedGeometry::from_parts(
            context.reference().clone(),
            Vec::new(),
            Vec::new(),
            PreparedRegion::Whole,
            context.policy(),
        )
        .unwrap();
        let empty = source("GEOMETRYCOLLECTION EMPTY");
        assert_eq!(
            relate(&whole, &whole, &mut context).unwrap().to_string(),
            "2FFFFFFFF"
        );
        assert_eq!(
            relate(&whole, &empty, &mut context).unwrap().to_string(),
            "FF2FFFFFF"
        );
        struct Stop;
        impl MetricWorkObserver for Stop {
            fn charge_chunk(&mut self, _: u64, _: u64) -> Result<(), GeoError> {
                Err(GeoError::Cancelled)
            }
        }
        assert!(matches!(
            relate_metered(&whole, &empty, &mut context, &mut Stop),
            Err(GeoError::Cancelled)
        ));
    }
}
