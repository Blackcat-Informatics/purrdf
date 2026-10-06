// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use super::*;
use crate::{CoordDim, Crs, ExecutionLimits, Geometry, GeometryKind, Int};
use purrdf_iri::vocab::ogc;

#[test]
fn carrier_content_identity_normalizes_exact_values_and_excludes_admission() {
    let a =
        literal("GEOMETRYCOLLECTION ZM (POINT ZM (1.0 2.00 3 4),LINESTRING ZM (0 0 1 2,1 2 3 4))");
    let b = literal(
        "GEOMETRYCOLLECTION ZM (POINT ZM (1 2 3.00 4.0),LINESTRING ZM (0.0 0 1.0 2,1.00 2.0 3 4.00))",
    );
    let baseline = source_inventory(a.geometry(), &ExecutionLimits::GEOMETRY, None)
        .expect("complete original inventory");
    let mut raised = ExecutionLimits::GEOMETRY;
    raised.max_work_items *= 2;
    raised.max_workspace_bytes *= 2;
    let equivalent = source_inventory(b.geometry(), &raised, None).expect("equivalent carrier");
    assert_eq!(baseline.content_id, equivalent.content_id);
    assert_eq!(baseline.vertices, 3);
    assert_eq!(baseline.nesting_depth, 1);
    assert!(baseline.work_items > baseline.vertices);
    let changed =
        literal("GEOMETRYCOLLECTION ZM (POINT ZM (1 2 3 5),LINESTRING ZM (0 0 1 2,1 2 3 4))");
    assert_ne!(
        baseline.content_id,
        source_inventory(changed.geometry(), &raised, None)
            .expect("changed measure metadata")
            .content_id
    );
}

#[test]
fn endpoint_arcs_retain_exact_coordinates_and_refuse_multiple_shortest_branches() {
    let reference = GeographicReference::wgs84();
    let mut context = MetricContext::new(reference.clone(), crate::ExecutionPolicy::geometry())
        .expect("admission");
    let coordinate = |lon, lat| {
        PreparedCoordinate::new(
            Coord::xy(Rat::from_i64(lon), Rat::from_i64(lat)),
            &reference,
        )
        .expect("geographic source")
    };
    let start = coordinate(0, 0);
    let end = coordinate(1, 0);
    let arc = ShortestGeodesicArc::new(start.clone(), end.clone(), &mut context)
        .expect("unique equatorial branch");
    assert_eq!(arc.start(), &start);
    assert_eq!(arc.end(), &end);
    assert_eq!(
        arc.at(
            &Rat::one().div(&Rat::from_i64(2)).expect("half"),
            &mut context
        )
        .expect("true endpoint arc midpoint")
        .endpoint(),
        &LonLat::new(
            Rat::one().div(&Rat::from_i64(2)).expect("half"),
            Rat::zero()
        )
        .expect("equatorial midpoint")
    );
    assert!(arc.at(&Rat::from_i64(2), &mut context).is_err());
    assert_eq!(
        arc.inverse().branch_multiplicity(),
        ShortestBranchMultiplicity::Unique
    );
    assert_eq!(
        arc.proof().branch_multiplicity,
        ShortestBranchMultiplicity::Unique
    );
    assert!(matches!(
        ShortestGeodesicArc::new(start.clone(), coordinate(180, 0), &mut context),
        Err(GeoError::AmbiguousGeodesic)
    ));
    let near_antipode = PreparedCoordinate::new(
        Coord::xy(Rat::parse_decimal("179.9").unwrap(), Rat::zero()),
        &reference,
    )
    .unwrap();
    assert!(matches!(
        ShortestGeodesicArc::new(start.clone(), near_antipode, &mut context),
        Err(GeoError::AmbiguousGeodesic)
    ));
    assert!(matches!(
        ShortestGeodesicArc::new(coordinate(15, 90), coordinate(-73, -90), &mut context),
        Err(GeoError::AmbiguousGeodesic)
    ));
    let zero = ShortestGeodesicArc::new(start.clone(), start, &mut context)
        .expect("one zero image branch");
    assert!(zero.inverse().distance().value().exact().is_zero());
}

fn literal(text: &str) -> GeometryLiteral {
    crate::wkt::parse(text, &Crs::new(ogc::CRS84).expect("standard IRI")).expect("valid carrier")
}

#[test]
fn original_longitude_paths_metadata_and_dimension_inventories_are_preserved() {
    let prepared = PreparedGeometry::from_literal(
        &literal("GEOMETRYCOLLECTION ZM (POINT ZM (1 2 3 4), LINESTRING ZM (170 0 5 6,-170 0 7 8), POLYGON ZM ((0 0 1 2,2 0 3 4,2 2 5 6,0 0 1 2)))"),
        &GeoProfile::standard(),
    ).expect("prepared");
    assert_eq!(prepared.points().len(), 1);
    assert_eq!(prepared.curves().len(), 1);
    let PreparedEdge::SourceLinear(edge) = &prepared.curves()[0].edges()[0] else {
        panic!("source-linear")
    };
    assert_eq!(edge.delta_longitude(), Rat::from_i64(-340));
    assert_eq!(
        edge.at(&Rat::one().div(&Rat::from_i64(2)).expect("half"))
            .expect("midpoint"),
        LonLat::new(Rat::zero(), Rat::zero()).expect("origin")
    );
    assert_eq!(
        edge.at(&Rat::one().div(&Rat::from_i64(2)).expect("half")),
        SourceLinearEdge::interpolate(
            edge.start().point(),
            edge.end().point(),
            &Rat::one().div(&Rat::from_i64(2)).expect("half")
        )
    );
    assert!(
        SourceLinearEdge::interpolate(edge.start().point(), edge.end().point(), &Rat::from_i64(2))
            .is_err()
    );
    assert_eq!(edge.start().source().z(), Some(&Rat::from_i64(5)));
    assert_eq!(edge.end().source().m(), Some(&Rat::from_i64(8)));
    let midpoint = SourceLinearEdge::interpolate_coord(
        edge.start().source(),
        edge.end().source(),
        &Rat::one().div(&Rat::from_i64(2)).expect("half"),
    );
    assert_eq!(midpoint.z(), Some(&Rat::from_i64(6)));
    assert_eq!(midpoint.m(), Some(&Rat::from_i64(7)));
    assert!(
        edge.length_upper_bound(prepared.reference())
            .expect("matching reference")
            .exact()
            > &Rat::from_i64(37_000_000)
    );
    assert_eq!(
        edge.length_upper_bound(prepared.reference())
            .expect("reference"),
        SourceLinearEdge::upper_bound(
            edge.start().point(),
            edge.end().point(),
            prepared.reference()
        )
    );
    let PreparedRegion::Polygons(polygons) = prepared.region() else {
        panic!("areal region")
    };
    assert_eq!(polygons[0].rings().len(), 1);
    assert_eq!(
        polygons[0].signed_chart_ring_areas(),
        Some(vec![Rat::from_i64(2)])
    );
    assert!(matches!(
        edge.length_upper_bound(&GeographicReference::cgcs2000()),
        Err(GeoError::MissingOperation { .. })
    ));
}

#[test]
fn identities_bind_exact_source_axes_and_preserve_equivalent_policy() {
    let original = literal("POINT (1.0000000000000000001 2)");
    let rounded_neighbor = literal("POINT (1 2)");
    let one = PreparedGeometry::from_literal(&original, &GeoProfile::standard()).expect("prepared");
    let two = PreparedGeometry::from_literal(&rounded_neighbor, &GeoProfile::standard())
        .expect("prepared");
    assert_ne!(one.id(), two.id());
    let raised = GeoProfile::standard()
        .with_limits(ExecutionLimits {
            max_work_items: 524_288,
            ..ExecutionLimits::GEOMETRY
        })
        .expect("policy");
    assert_eq!(
        one.id(),
        PreparedGeometry::from_literal(&original, &raised)
            .expect("same law")
            .id()
    );
    let reference = GeographicReference::wgs84().with_axes(AxisOrder::LatLon);
    let swapped = PreparedCoordinate::new(
        Coord::xy(Rat::from_i64(49), Rat::from_i64(-123)),
        &reference,
    )
    .expect("declared axes");
    assert_eq!(
        swapped.point(),
        &LonLat::new(Rat::from_i64(-123), Rat::from_i64(49)).expect("geography")
    );
    let other = PreparedCoordinate::new(
        Coord::xy(Rat::zero(), Rat::zero()),
        &GeographicReference::cgcs2000(),
    )
    .expect("CGCS");
    assert!(matches!(
        SourceLinearEdge::new(swapped, other),
        Err(GeoError::MissingOperation { .. })
    ));
}

#[test]
fn preparation_refuses_complete_structure_and_payload_before_cloning() {
    let empty = Geometry::empty(CoordDim::Xy, GeometryKind::Point);
    let tree = Geometry::new(
        CoordDim::Xy,
        GeometryBody::GeometryCollection(vec![empty; 100]),
    )
    .expect("collection");
    let original = GeometryLiteral::new(Crs::new(ogc::CRS84).expect("IRI"), tree);
    let profile = GeoProfile::standard()
        .with_limits(ExecutionLimits {
            max_work_items: 50,
            ..ExecutionLimits::GEOMETRY
        })
        .expect("policy");
    assert!(matches!(
        PreparedGeometry::from_literal(&original, &profile),
        Err(GeoError::WorkExhausted { limit: 50 })
    ));
    let profile = GeoProfile::standard()
        .with_limits(ExecutionLimits {
            max_workspace_bytes: 65_535,
            ..ExecutionLimits::GEOMETRY
        })
        .expect("policy");
    assert!(matches!(
        PreparedGeometry::from_literal(&literal("POINT (1 2)"), &profile),
        Err(GeoError::MemoryExhausted { limit: 65_535 })
    ));
    let profile = GeoProfile::standard()
        .with_limits(ExecutionLimits {
            max_output_elements: 1,
            ..ExecutionLimits::GEOMETRY
        })
        .expect("policy");
    assert!(matches!(
        PreparedGeometry::from_literal(&literal("LINESTRING (0 0,1 1)"), &profile),
        Err(GeoError::OutputExhausted { limit: 1 })
    ));
    let mut coordinates = Vec::with_capacity(1_024);
    coordinates.push(Coord::xy(Rat::zero(), Rat::zero()));
    coordinates.push(Coord::xy(Rat::one(), Rat::one()));
    let source = Geometry::new(CoordDim::Xy, GeometryBody::LineString(coordinates))
        .expect("two coordinates with retained spare capacity");
    let original = GeometryLiteral::new(Crs::new(ogc::CRS84).expect("IRI"), source);
    let profile = GeoProfile::standard()
        .with_limits(ExecutionLimits {
            max_workspace_bytes: 100_000,
            ..ExecutionLimits::GEOMETRY
        })
        .expect("capacity admission");
    assert!(matches!(
        PreparedGeometry::from_literal(&original, &profile),
        Err(GeoError::MemoryExhausted { limit: 100_000 })
    ));
}

#[test]
fn native_parts_keep_explicit_whole_regions_and_original_reference_laws() {
    let reference = GeographicReference::wgs84();
    let coordinate = PreparedCoordinate::new(Coord::xy(Rat::one(), Rat::from_i64(2)), &reference)
        .expect("coordinate");
    let whole = PreparedGeometry::from_parts(
        reference.clone(),
        vec![coordinate.clone()],
        Vec::new(),
        PreparedRegion::Whole,
        crate::ExecutionPolicy::geometry(),
    )
    .expect("native whole");
    assert_eq!(whole.points(), core::slice::from_ref(&coordinate));
    assert_eq!(whole.region(), &PreparedRegion::Whole);
    let raised = crate::ExecutionPolicy::new(ExecutionLimits {
        max_work_items: 524_288,
        ..ExecutionLimits::GEOMETRY
    })
    .expect("policy");
    assert_eq!(
        whole.id(),
        PreparedGeometry::from_parts(
            reference.clone(),
            vec![coordinate.clone()],
            Vec::new(),
            PreparedRegion::Whole,
            raised
        )
        .expect("same law")
        .id()
    );
    assert_ne!(
        whole.id(),
        PreparedGeometry::from_parts(
            reference,
            vec![coordinate],
            Vec::new(),
            PreparedRegion::Empty,
            raised
        )
        .expect("explicit empty")
        .id()
    );
    assert!(matches!(
        PreparedGeometry::from_parts(
            GeographicReference::cgcs2000(),
            whole.points().to_vec(),
            Vec::new(),
            PreparedRegion::Whole,
            raised
        ),
        Err(GeoError::MissingOperation { .. })
    ));
}

struct PreparationObserver {
    work: u64,
    maximum: u64,
}
impl MetricWorkObserver for PreparationObserver {
    fn charge_chunk(&mut self, work: u64, _growth: u64) -> Result<(), GeoError> {
        self.work = self.work.saturating_add(work);
        if self.work > self.maximum {
            Err(GeoError::Cancelled)
        } else {
            Ok(())
        }
    }
}

#[test]
fn preparation_charges_construction_and_identity_before_complete_publication() {
    let line = Geometry::new(
        CoordDim::Xy,
        GeometryBody::LineString(
            (-100..100)
                .map(|lon| Coord::xy(Rat::from_i64(lon), Rat::zero()))
                .collect(),
        ),
    )
    .expect("line");
    let source = GeometryLiteral::new(Crs::new(ogc::CRS84).expect("IRI"), line);
    let profile = GeoProfile::standard();
    let plain = PreparedGeometry::from_literal(&source, &profile).expect("prepared");
    // Structural work alone was 604 items. Source cloning and exact range
    // comparisons now contribute their independently bounded limb work.
    assert!(plain.preparation_work_items() > 604);
    for maximum in [300, 500] {
        let mut observer = PreparationObserver { work: 0, maximum };
        assert!(matches!(
            PreparedGeometry::from_literal_metered(&source, &profile, &mut observer),
            Err(GeoError::Cancelled)
        ));
        assert!(observer.work > maximum);
    }
    let mut observer = PreparationObserver {
        work: 0,
        maximum: u64::MAX,
    };
    let metered = PreparedGeometry::from_literal_metered(&source, &profile, &mut observer)
        .expect("complete metered preparation");
    assert_eq!(plain.id(), metered.id());
    assert_eq!(
        plain.preparation_work_items(),
        metered.preparation_work_items()
    );
    assert_eq!(
        plain.retained_workspace_bytes(),
        metered.retained_workspace_bytes()
    );
    assert_eq!(observer.work, plain.preparation_work_items());
    let reference = GeographicReference::wgs84();
    let points: Vec<_> = (0..128)
        .map(|lon| {
            PreparedCoordinate::new(Coord::xy(Rat::from_i64(lon), Rat::zero()), &reference)
                .expect("source")
        })
        .collect();
    let mut observer = PreparationObserver {
        work: 0,
        maximum: 200,
    };
    assert!(matches!(
        PreparedGeometry::from_parts_metered(
            reference,
            points,
            Vec::new(),
            PreparedRegion::Empty,
            crate::ExecutionPolicy::geometry(),
            &mut observer
        ),
        Err(GeoError::Cancelled)
    ));
}

#[test]
fn native_oriented_bases_retain_side_and_have_no_written_chart() {
    let reference = GeographicReference::wgs84();
    let mut context = MetricContext::wgs84().expect("context");
    let curve = |vertices: &[(i64, i64)]| {
        PreparedCurve::from_source(
            &vertices
                .iter()
                .map(|(x, y)| Coord::xy(Rat::from_i64(*x), Rat::from_i64(*y)))
                .collect::<Vec<_>>(),
            &reference,
        )
        .expect("original curve")
    };
    let outer = crate::atlas::oriented::OrientedRing::new(
        curve(&[(-40, -30), (40, -30), (40, 30), (-40, 30), (-40, -30)]),
        &mut context,
    )
    .expect("outer Jordan");
    let hole = crate::atlas::oriented::OrientedRing::new(
        curve(&[(-10, -10), (-10, 10), (10, 10), (10, -10), (-10, -10)]),
        &mut context,
    )
    .expect("hole Jordan");
    let polygon =
        PreparedPolygon::from_oriented_rings(vec![outer, hole], OrientedInterior::Left, &reference)
            .expect("native hole intersection");
    assert!(polygon.chart().is_none());
    assert!(polygon.signed_chart_ring_areas().is_none());
    assert_eq!(polygon.oriented_interior(), Some(OrientedInterior::Left));
    let point = |x, y| LonLat::new(Rat::from_i64(x), Rat::from_i64(y)).expect("point");
    let region = PreparedRegion::Polygons(vec![polygon.clone()].into());
    assert_eq!(
        crate::atlas::locate(&point(20, 0), &region, &mut context).expect("between outer and hole"),
        crate::Set::Interior
    );
    assert_eq!(
        crate::atlas::locate(&point(0, 0), &region, &mut context).expect("hole"),
        crate::Set::Exterior
    );
    assert_eq!(
        crate::atlas::locate(&point(0, 90), &region, &mut context).expect("outside pole"),
        crate::Set::Exterior
    );
    let complement =
        PreparedRegion::Polygons(vec![polygon.with_interior(RegionInterior::Complement)].into());
    assert_eq!(
        crate::atlas::locate(&point(0, 0), &complement, &mut context).expect("complement hole"),
        crate::Set::Interior
    );
    assert_eq!(
        crate::atlas::locate(&point(0, 90), &complement, &mut context)
            .expect("large complement pole"),
        crate::Set::Interior
    );
}

#[test]
fn certified_rank_lost_areal_support_retains_closed_contacts_and_whole_complement() {
    let reference = GeographicReference::wgs84();
    let source = [
        Coord::xy(Rat::zero(), Rat::zero()),
        Coord::xy(Rat::from_i64(2), Rat::zero()),
        Coord::xy(Rat::zero(), Rat::zero()),
    ];
    let curve = PreparedCurve::from_source(&source, &reference).unwrap();
    let polygon = PreparedPolygon::from_selected_support(vec![curve], &reference).unwrap();
    assert!(polygon.closed_support());
    assert!(polygon.chart().is_none());
    assert!(polygon.oriented_interior().is_none());
    assert!(!crate::atlas::polygon_base_membership(&polygon, &[true]).unwrap());
    let inside = LonLat::new(Rat::one(), Rat::zero()).unwrap();
    let outside = LonLat::new(Rat::one(), Rat::one()).unwrap();
    let mut context = MetricContext::wgs84().unwrap();
    let mut progress = WorkProgress::new(None);
    context.begin(1).unwrap();
    assert_eq!(
        crate::atlas::polygon_location(&inside, &polygon, &mut context, &mut progress).unwrap(),
        crate::Set::Boundary
    );
    assert_eq!(
        crate::atlas::polygon_location(&outside, &polygon, &mut context, &mut progress).unwrap(),
        crate::Set::Exterior
    );
    let complement = polygon.clone().with_interior(RegionInterior::Complement);
    for point in [&inside, &outside] {
        assert_eq!(
            crate::atlas::polygon_location(point, &complement, &mut context, &mut progress)
                .unwrap(),
            crate::Set::Interior
        );
    }
    let region = PreparedRegion::polygons(vec![polygon]);
    assert_eq!(
        crate::atlas::locate(&inside, &region, &mut context).unwrap(),
        crate::Set::Boundary
    );
    for point in [&inside, &outside] {
        assert_eq!(
            crate::atlas::locate(point, &region.clone().complement(), &mut context).unwrap(),
            crate::Set::Interior
        );
    }
    assert_eq!(
        context.current_workspace_bytes(),
        context.retained_workspace_bytes()
    );
}

#[test]
fn closed_areal_image_metrics_use_selected_support_once() {
    let reference = GeographicReference::wgs84();
    let origin = Coord::xy(Rat::zero(), Rat::zero());
    let endpoint = Coord::xy(Rat::from_i64(2), Rat::zero());
    let retraced =
        PreparedCurve::from_source(&[origin.clone(), endpoint, origin], &reference).unwrap();
    let support = PreparedPolygon::from_selected_support(vec![retraced], &reference).unwrap();
    let geometry = PreparedGeometry::from_parts(
        reference,
        Vec::new(),
        Vec::new(),
        PreparedRegion::polygons(vec![support]),
        crate::ExecutionPolicy::geometry(),
    )
    .unwrap();
    let line =
        PreparedGeometry::from_literal(&literal("LINESTRING (0 0,2 0)"), &GeoProfile::standard())
            .unwrap();
    let point =
        PreparedGeometry::from_literal(&literal("POINT (1 0)"), &GeoProfile::standard()).unwrap();
    let mut context = MetricContext::wgs84().unwrap();
    let expected = crate::ellipsoidal::length(&line, &mut context).unwrap();
    assert!(!expected.exact().is_zero());
    assert_eq!(
        crate::ellipsoidal::length(&geometry, &mut context)
            .unwrap()
            .exact(),
        expected.exact()
    );
    assert_eq!(
        crate::ellipsoidal::perimeter(&geometry, &mut context)
            .unwrap()
            .exact(),
        expected.exact()
    );
    assert!(
        crate::ellipsoidal::area(&geometry, &mut context)
            .unwrap()
            .exact()
            .is_zero()
    );
    assert!(
        crate::ellipsoidal::distance(&geometry, &point, &mut context)
            .unwrap()
            .exact()
            .is_zero()
    );
    assert_eq!(
        crate::ellipsoidal::region_area(&geometry.region().clone().complement(), &mut context)
            .unwrap()
            .exact(),
        crate::ellipsoidal::region_area(&PreparedRegion::Whole, &mut context)
            .unwrap()
            .exact()
    );
    assert_eq!(
        context.current_workspace_bytes(),
        context.retained_workspace_bytes()
    );
}

#[test]
fn admitted_coordinate_factories_preserve_original_axes_and_refuse_before_clone() {
    let reference = GeographicReference::wgs84().with_axes(AxisOrder::LatLon);
    let source = Coord::new(
        Rat::from_i64(23),
        Rat::from_i64(117),
        Some(Rat::from_i64(8)),
        Some(Rat::from_i64(9)),
    );
    let mut context =
        MetricContext::new(reference.clone(), crate::ExecutionPolicy::geometry()).unwrap();
    let admitted =
        PreparedCoordinate::from_source_in_context(&source, &reference, &mut context).unwrap();
    assert_eq!(admitted.source(), &source);
    assert_eq!(admitted.point().longitude(), &Rat::from_i64(117));
    assert_eq!(admitted.point().latitude(), &Rat::from_i64(23));
    let work = context.work_items();
    assert!(work > 4);
    let mut observer = PreparationObserver {
        work: 0,
        maximum: 0,
    };
    assert!(matches!(
        PreparedCoordinate::from_source_in_context_metered(
            &source,
            &reference,
            &mut context,
            &mut observer
        ),
        Err(GeoError::Cancelled)
    ));
    let tiny = crate::ExecutionPolicy::new(ExecutionLimits {
        max_work_items: 1,
        ..ExecutionLimits::GEOMETRY
    })
    .unwrap();
    let mut context = MetricContext::new(reference.clone(), tiny).unwrap();
    assert!(matches!(
        PreparedCoordinate::from_source_in_context(&source, &reference, &mut context),
        Err(GeoError::WorkExhausted { .. })
    ));
}

#[test]
fn cold_wide_reference_entries_refuse_before_rendering_or_copying() {
    let policy = crate::ExecutionPolicy::new(ExecutionLimits {
        max_work_items: 1_000_000_000,
        ..ExecutionLimits::GEOMETRY
    })
    .unwrap();
    let mut budget = crate::PreparationBudget::new(policy);
    let reference = GeographicReference::new(
        crate::PreparedEllipsoid::new_in_budget(
            Rat::from_int(Int::one().shl(4096).add(&Int::one())),
            Rat::from_i64(300),
            &mut budget,
        )
        .unwrap(),
        Digest32::new([73; 32]),
        AxisOrder::LonLat,
    );
    let source = Coord::xy(Rat::zero(), Rat::zero());
    let start = PreparedCoordinate::new(source.clone(), &reference).unwrap();
    let end = PreparedCoordinate::new(Coord::xy(Rat::one(), Rat::zero()), &reference).unwrap();
    let edge = PreparedEdge::SourceLinear(Box::new(
        SourceLinearEdge::new(start.clone(), end.clone()).unwrap(),
    ));
    // Rebuild an equivalent declaration so each public entry really receives
    // a cold identity cache, despite the already prepared endpoint bindings.
    for limits in [
        ExecutionLimits {
            max_work_items: 1,
            ..ExecutionLimits::GEOMETRY
        },
        ExecutionLimits {
            max_workspace_bytes: 1,
            ..ExecutionLimits::GEOMETRY
        },
    ] {
        for entry in 0..5 {
            let cold = GeographicReference::new(
                reference.ellipsoid().clone(),
                reference.datum(),
                reference.axes(),
            );
            let mut context =
                MetricContext::new(cold, crate::ExecutionPolicy::new(limits).unwrap()).unwrap();
            let owned_start = start.clone();
            let owned_end = end.clone();
            let azimuth = Rat::from_i64(90);
            let length = Metres::new(Rat::one());
            let window = purrdf_alloc_probe::CurrentThreadWindow::open();
            let result = match entry {
                0 => PreparedCoordinate::from_source_in_context(&source, &reference, &mut context)
                    .map(|_| ()),
                1 => ShortestGeodesicArc::new(owned_start, owned_end, &mut context).map(|_| ()),
                2 => AzimuthLengthArc::new(owned_start, azimuth, length, &mut context).map(|_| ()),
                3 => edge.geodesic_view(112, &mut context).map(|_| ()),
                _ => crate::atlas::locate(start.point(), &PreparedRegion::Empty, &mut context)
                    .map(|_| ()),
            };
            let measured = window.close();
            if limits.max_work_items == 1 {
                assert!(matches!(result, Err(GeoError::WorkExhausted { .. })));
            } else {
                assert!(matches!(result, Err(GeoError::MemoryExhausted { .. })));
            }
            assert_eq!(measured.allocations, 0, "public entry {entry}");
            assert_eq!(measured.requested_bytes, 0, "public entry {entry}");
            assert_eq!(context.current_workspace_bytes(), 0);
        }
    }
}

#[derive(Default)]
struct ArcReceipt {
    work: u64,
    peak: u64,
}
impl MetricWorkObserver for ArcReceipt {
    fn charge_chunk(&mut self, work: u64, growth: u64) -> Result<(), GeoError> {
        self.work = self.work.checked_add(work).unwrap();
        self.peak = self.peak.checked_add(growth).unwrap();
        Ok(())
    }
}

#[test]
fn public_endpoint_arc_keeps_identity_work_across_solver_resets() {
    let reference = GeographicReference::wgs84();
    let start = PreparedCoordinate::new(Coord::xy(Rat::zero(), Rat::zero()), &reference).unwrap();
    let end = PreparedCoordinate::new(Coord::xy(Rat::one(), Rat::zero()), &reference).unwrap();
    let policy = crate::ExecutionPolicy::geometry();
    let prepared = PreparedGeodesic::new(reference.clone());
    let mut scalar_context = MetricContext::new(reference.clone(), policy).unwrap();
    let (scalar, _) = prepared
        .inverse_with_proof(start.point(), end.point(), &mut scalar_context)
        .unwrap();
    let mut context = MetricContext::new(reference.clone(), policy).unwrap();
    let mut observer = ArcReceipt::default();
    let arc =
        ShortestGeodesicArc::new_metered(start.clone(), end.clone(), &mut context, &mut observer)
            .unwrap();
    assert_eq!(arc.inverse().distance(), scalar.distance());
    assert_eq!(observer.work, context.work_items());
    assert_eq!(observer.peak, context.workspace_peak());
    assert_eq!(
        context.current_workspace_bytes(),
        context.retained_workspace_bytes()
    );
    assert!(context.work_items() > scalar_context.work_items());
    // This budget admits the independently measured complete inverse. It must
    // refuse the arc entry's additional original-reference admission rather
    // than resetting that work when entering the same numerical solver.
    let limited = crate::ExecutionPolicy::new(ExecutionLimits {
        max_work_items: scalar_context.work_items(),
        ..ExecutionLimits::GEOMETRY
    })
    .unwrap();
    let mut limited_context = MetricContext::new(reference.clone(), limited).unwrap();
    assert!(
        prepared
            .inverse_with_proof(start.point(), end.point(), &mut limited_context)
            .is_ok()
    );
    let mut limited_context = MetricContext::new(reference, limited).unwrap();
    assert!(matches!(
        ShortestGeodesicArc::new(start, end, &mut limited_context),
        Err(GeoError::WorkExhausted { .. })
    ));
    assert_eq!(limited_context.current_workspace_bytes(), 0);
}

#[test]
fn borrowed_azimuth_parameters_remain_admitted_during_complete_direct_solve() {
    let reference = GeographicReference::wgs84();
    let start = PreparedCoordinate::new(Coord::xy(Rat::zero(), Rat::zero()), &reference).unwrap();
    let scale = Int::one().shl(256);
    let azimuth = Rat::new(
        scale.mul(&Int::from_i64(90)).add(&Int::one()),
        scale.clone(),
    )
    .unwrap();
    let length =
        Metres::new(Rat::new(scale.mul(&Int::from_i64(1000)).add(&Int::one()), scale).unwrap());
    let policy = crate::ExecutionPolicy::new(ExecutionLimits {
        max_work_items: 8_000_000,
        ..ExecutionLimits::GEOMETRY
    })
    .unwrap();
    let mut owned_context = MetricContext::new(reference.clone(), policy).unwrap();
    let owned = AzimuthLengthArc::new(
        start.clone(),
        azimuth.clone(),
        length.clone(),
        &mut owned_context,
    )
    .unwrap();
    let mut context = MetricContext::new(reference.clone(), policy).unwrap();
    let borrowed =
        AzimuthLengthArc::from_source_in_context(start.clone(), &azimuth, &length, &mut context)
            .unwrap();
    assert_eq!(borrowed, owned);
    assert_eq!(borrowed.azimuth(), &azimuth);
    assert_eq!(borrowed.length(), &length);
    assert!(context.work_items() > owned_context.work_items());
    assert!(context.workspace_peak() > owned_context.workspace_peak());
    assert_eq!(
        context.current_workspace_bytes(),
        context.retained_workspace_bytes()
    );
    let limited = crate::ExecutionPolicy::new(ExecutionLimits {
        max_work_items: owned_context.work_items(),
        ..ExecutionLimits::GEOMETRY
    })
    .unwrap();
    let mut context = MetricContext::new(reference, limited).unwrap();
    assert!(matches!(
        AzimuthLengthArc::from_source_in_context(start, &azimuth, &length, &mut context),
        Err(GeoError::WorkExhausted { .. })
    ));
    // Successfully installed immutable numerical coefficients can survive a
    // later refusal. Copied branch-parameter temporaries cannot survive it.
    assert_eq!(
        context.current_workspace_bytes(),
        context.retained_workspace_bytes()
    );
    context.begin(1).unwrap();
}

#[test]
fn borrowed_coordinate_owner_survives_identity_and_range_admission() {
    let reference = GeographicReference::wgs84().with_axes(AxisOrder::LatLon);
    let scale = Int::one().shl(256);
    let source = Coord::new(
        Rat::from_i64(23),
        Rat::from_i64(117),
        Some(Rat::from_int(scale.add(&Int::one()))),
        Some(Rat::new(Int::one(), scale).unwrap()),
    );
    let policy = crate::ExecutionPolicy::geometry();
    let mut owned_context = MetricContext::new(reference.clone(), policy).unwrap();
    let owned =
        PreparedCoordinate::new_in_context(source.clone(), &reference, &mut owned_context).unwrap();
    let mut context = MetricContext::new(reference.clone(), policy).unwrap();
    let mut observer = ArcReceipt::default();
    let borrowed = PreparedCoordinate::from_source_in_context_metered(
        &source,
        &reference,
        &mut context,
        &mut observer,
    )
    .unwrap();
    assert_eq!(borrowed, owned);
    assert_eq!(borrowed.source(), &source);
    assert!(context.work_items() > owned_context.work_items());
    assert!(context.workspace_peak() > owned_context.workspace_peak());
    assert_eq!(observer.work, context.work_items());
    assert_eq!(observer.peak, context.workspace_peak());
    assert_eq!(context.current_workspace_bytes(), 0);
    let limited = crate::ExecutionPolicy::new(ExecutionLimits {
        max_workspace_bytes: owned_context.workspace_peak(),
        ..ExecutionLimits::GEOMETRY
    })
    .unwrap();
    let mut context = MetricContext::new(reference.clone(), limited).unwrap();
    assert!(PreparedCoordinate::new_in_context(source.clone(), &reference, &mut context).is_ok());
    let mut context = MetricContext::new(reference.clone(), limited).unwrap();
    assert!(matches!(
        PreparedCoordinate::from_source_in_context(&source, &reference, &mut context),
        Err(GeoError::MemoryExhausted { .. })
    ));
    assert_eq!(context.current_workspace_bytes(), 0);
}

#[test]
fn symbolic_points_preserve_dimension_zero_original_content_and_empty_ids() {
    use crate::operation::{
        CoordinateOperation, CoordinateUnit, OperationChain, OperationModel, OperationReference,
    };
    let reference = GeographicReference::wgs84();
    let empty = PreparedGeometry::from_parts(
        reference.clone(),
        Vec::new(),
        Vec::new(),
        PreparedRegion::Empty,
        crate::ExecutionPolicy::geometry(),
    )
    .unwrap();
    let empty_symbolic = PreparedGeometry::from_parts_with_symbolic(
        reference.clone(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        PreparedRegion::Empty,
        crate::ExecutionPolicy::geometry(),
    )
    .unwrap();
    assert_eq!(empty.id(), empty_symbolic.id());
    let chain = Arc::new(
        OperationChain::compile(vec![
            CoordinateOperation::compile(
                OperationReference {
                    realization: Digest32::new([17; 32]),
                    unit: CoordinateUnit::Metres,
                    swapped_axes: false,
                },
                OperationReference {
                    realization: reference.id().digest(),
                    unit: CoordinateUnit::Degrees,
                    swapped_axes: false,
                },
                OperationModel::MercatorToGeographic {
                    radius: Rat::from_i64(6_378_137),
                    eccentricity_squared: Rat::zero(),
                    square_domain: true,
                },
            )
            .unwrap(),
        ])
        .unwrap(),
    );
    let source = Coord::new(
        Rat::one(),
        Rat::zero(),
        Some(Rat::from_i64(10)),
        Some(Rat::from_i64(20)),
    );
    let point = OperationImagePoint::new(
        source.clone(),
        chain,
        reference.clone(),
        None,
        crate::ExecutionPolicy::geometry(),
    )
    .unwrap();
    let original = PreparedGeometry::from_parts_with_symbolic(
        reference.clone(),
        Vec::new(),
        vec![point.clone()],
        Vec::new(),
        PreparedRegion::Empty,
        crate::ExecutionPolicy::geometry(),
    )
    .unwrap();
    assert_eq!(original.points(), []);
    assert_eq!(original.curves(), []);
    assert_eq!(original.symbolic_points()[0].source(), &source);
    assert_ne!(original.id(), empty.id());
    let raised = crate::ExecutionPolicy::new(ExecutionLimits {
        max_work_items: 1_000_000,
        ..ExecutionLimits::GEOMETRY
    })
    .unwrap();
    assert_eq!(
        original.id(),
        PreparedGeometry::from_parts_with_symbolic(
            reference,
            Vec::new(),
            vec![point],
            Vec::new(),
            PreparedRegion::Empty,
            raised
        )
        .unwrap()
        .id()
    );
    let receipt = original
        .clone()
        .with_preparation_receipt(1_234_567, 65_536_000);
    assert_eq!(receipt.id(), original.id());
    assert_eq!(receipt.preparation_work_items(), 1_234_567);
    assert_eq!(receipt.retained_workspace_bytes(), 65_536_000);
}
