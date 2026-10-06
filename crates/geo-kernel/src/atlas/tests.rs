// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use super::*;
use crate::{
    Crs, ExecutionLimits, ExecutionPolicy, GeoProfile, GeographicReference, PreparedGeometry,
};
use purrdf_iri::vocab::ogc;

fn region(text: &str) -> PreparedRegion {
    let literal = crate::wkt::parse(text, &Crs::new(ogc::CRS84).expect("IRI")).expect("carrier");
    PreparedGeometry::from_literal(&literal, &GeoProfile::standard())
        .expect("prepared")
        .region()
        .clone()
}
fn point(longitude: i64, latitude: i64) -> LonLat {
    LonLat::new(Rat::from_i64(longitude), Rat::from_i64(latitude)).expect("geography")
}

#[test]
fn complete_closed_support_union_keeps_contact_without_an_areal_arrangement() {
    let reference = GeographicReference::wgs84();
    let mut supports = Vec::new();
    for vertices in [[(0, 0), (2, 0)], [(2, 0), (0, 0)], [(1, -1), (1, 1)]] {
        let vertices = vertices.map(|(x, y)| Coord::xy(Rat::from_i64(x), Rat::from_i64(y)));
        let curve = PreparedCurve::from_source(&vertices, &reference).unwrap();
        supports.push(PreparedPolygon::from_selected_support(vec![curve], &reference).unwrap());
    }
    let source = PreparedRegion::polygons(supports);
    let mut context = MetricContext::wgs84().unwrap();
    for at in [point(0, 0), point(1, 0), point(2, 0), point(1, 1)] {
        assert_eq!(locate(&at, &source, &mut context).unwrap(), Set::Boundary);
        assert_eq!(context.current_workspace_bytes(), 0);
    }
    assert_eq!(
        locate(&point(3, 0), &source, &mut context).unwrap(),
        Set::Exterior
    );
    let complement = source.complement();
    for at in [point(1, 0), point(1, 1), point(3, 0)] {
        assert_eq!(
            locate(&at, &complement, &mut context).unwrap(),
            Set::Interior
        );
    }
}

#[test]
fn certified_polygon_bounds_preserve_semantics_identity_and_boundary_candidates() {
    let reference = GeographicReference::wgs84();
    let vertices = [(-2, -1), (2, -1), (2, 1), (-2, 1), (-2, -1)]
        .map(|(x, y)| Coord::xy(Rat::from_i64(x), Rat::from_i64(y)));
    let curve = PreparedCurve::from_source(&vertices, &reference).unwrap();
    let mut context = MetricContext::wgs84().unwrap();
    let original =
        PreparedPolygon::from_curves(vec![curve], crate::OrientedInterior::Left, &mut context)
            .unwrap();
    let proof_bytes = (size_of::<[Rat; 4]>() + 2 * size_of::<usize>()) as u64;
    let tight = original.clone().with_certified_angular_rectangle(
        [-2, -1, 2, 1].map(Rat::from_i64),
        None,
        proof_bytes,
    );
    let wide = original.clone().with_certified_angular_rectangle(
        [-3, -2, 3, 2].map(Rat::from_i64),
        None,
        proof_bytes,
    );
    let inner = original.clone().with_certified_angular_rectangle(
        [-2, -1, 2, 1].map(Rat::from_i64),
        Some([
            Rat::from_i64(-1),
            Rat::parse_decimal("-0.5").unwrap(),
            Rat::one(),
            Rat::parse_decimal("0.5").unwrap(),
        ]),
        2 * proof_bytes,
    );
    let exact = original.clone().with_certified_angular_rectangle(
        [-2, -1, 2, 1].map(Rat::from_i64),
        Some([-2, -1, 2, 1].map(Rat::from_i64)),
        2 * proof_bytes,
    );
    assert_eq!(original, tight);
    assert_eq!(tight, wide);
    assert_eq!(wide, inner);
    assert_eq!(inner, exact);
    let prepare = |polygon| {
        PreparedGeometry::from_parts(
            reference.clone(),
            Vec::new(),
            Vec::new(),
            PreparedRegion::polygons(vec![polygon]),
            ExecutionPolicy::geometry(),
        )
        .unwrap()
    };
    let base = prepare(original);
    let tight = prepare(tight);
    let wide = prepare(wide);
    let inner = prepare(inner);
    let exact = prepare(exact);
    assert_eq!(base.id(), tight.id());
    assert_eq!(tight.id(), wide.id());
    assert_eq!(wide.id(), inner.id());
    assert_eq!(inner.id(), exact.id());
    assert_eq!(
        tight.retained_workspace_bytes(),
        base.retained_workspace_bytes() + proof_bytes
    );
    assert_eq!(
        inner.retained_workspace_bytes(),
        base.retained_workspace_bytes() + 2 * proof_bytes
    );
    for (at, selected) in [
        (point(0, 0), Set::Interior),
        (point(1, 0), Set::Interior),
        (point(2, 0), Set::Boundary),
        (point(2, 1), Set::Boundary),
        (point(3, 0), Set::Exterior),
        (point(-180, 0), Set::Exterior),
        (point(180, 0), Set::Exterior),
        (point(37, 90), Set::Exterior),
    ] {
        for prepared in [&base, &tight, &wide, &inner, &exact] {
            assert_eq!(
                locate(&at, prepared.region(), &mut context).unwrap(),
                selected
            );
            let complement = prepared.region().clone().complement();
            assert_eq!(
                locate(&at, &complement, &mut context).unwrap(),
                super::complement(selected)
            );
        }
    }
}

#[test]
fn selected_native_boundaries_remove_internal_walls_and_identify_shared_poles() {
    let mut context = MetricContext::wgs84().unwrap();
    let polygon = |vertices: &[(i64, i64)], context: &mut MetricContext| {
        let vertices = vertices
            .iter()
            .map(|(x, y)| Coord::xy(Rat::from_i64(*x), Rat::from_i64(*y)))
            .collect::<Vec<_>>();
        let ring = PreparedCurve::from_source(&vertices, context.reference()).unwrap();
        PreparedPolygon::from_curves(vec![ring], crate::OrientedInterior::Left, context).unwrap()
    };
    let first = polygon(
        &[(-4, -4), (0, -4), (0, 4), (-4, 4), (-4, -4)],
        &mut context,
    );
    let second = polygon(&[(0, -4), (4, -4), (4, 4), (0, 4), (0, -4)], &mut context);
    let joined = PreparedRegion::polygons(vec![first.clone(), second]);
    assert_eq!(
        locate(&point(0, 0), &joined, &mut context).unwrap(),
        Set::Interior
    );
    assert_eq!(
        locate(&point(0, 4), &joined, &mut context).unwrap(),
        Set::Boundary
    );
    assert_eq!(
        locate(&point(0, 0), &joined.complement(), &mut context).unwrap(),
        Set::Exterior
    );
    let whole = PreparedRegion::polygons(vec![
        first.clone(),
        first.with_interior(RegionInterior::Complement),
    ]);
    assert_eq!(
        locate(&point(0, 0), &whole, &mut context).unwrap(),
        Set::Interior
    );
    let west = polygon(&[(60, -90), (60, 0), (0, 0), (0, -90)], &mut context);
    let east = polygon(&[(90, -90), (90, 0), (30, 0), (30, -90)], &mut context);
    let union = PreparedRegion::polygons(vec![west, east]);
    for longitude in [-180, -47, 0, 60, 180] {
        assert_eq!(
            locate(&point(longitude, -90), &union, &mut context).unwrap(),
            Set::Boundary
        );
    }
}

#[test]
fn certified_rectangle_sectors_match_original_full_union_and_complement() {
    let mut context = MetricContext::wgs84().unwrap();
    let proof_bytes = 2 * (size_of::<[Rat; 4]>() + 2 * size_of::<usize>()) as u64;
    let mut originals = Vec::new();
    let mut certified = Vec::new();
    for (west, east, south, north) in [(-2, 0, -2, 0), (0, 2, -2, 0), (-2, 0, 0, 2), (0, 2, 0, 2)] {
        let vertices = [
            (west, south),
            (east, south),
            (east, north),
            (west, north),
            (west, south),
        ]
        .map(|(x, y)| Coord::xy(Rat::from_i64(x), Rat::from_i64(y)));
        let curve = PreparedCurve::from_source(&vertices, context.reference()).unwrap();
        let polygon =
            PreparedPolygon::from_curves(vec![curve], crate::OrientedInterior::Left, &mut context)
                .unwrap();
        certified.push(polygon.clone().with_certified_angular_rectangle(
            [west, south, east, north].map(Rat::from_i64),
            Some([west, south, east, north].map(Rat::from_i64)),
            proof_bytes,
        ));
        originals.push(polygon);
    }
    let original = PreparedRegion::polygons(originals);
    let certified = PreparedRegion::polygons(certified);
    assert_eq!(original, certified);
    let mut oracle = MetricContext::new(
        GeographicReference::wgs84(),
        ExecutionPolicy::new(ExecutionLimits {
            max_work_items: 2_000_000,
            ..ExecutionLimits::GEOMETRY
        })
        .unwrap(),
    )
    .unwrap();
    for (at, selected) in [
        (point(0, 0), Set::Interior),
        (point(0, 1), Set::Interior),
        (point(0, 2), Set::Boundary),
        (point(2, 2), Set::Boundary),
        (point(3, 0), Set::Exterior),
    ] {
        assert_eq!(locate(&at, &original, &mut oracle).unwrap(), selected);
        assert_eq!(locate(&at, &certified, &mut context).unwrap(), selected);
        assert_eq!(
            locate(&at, &original.clone().complement(), &mut oracle).unwrap(),
            complement(selected)
        );
        assert_eq!(
            locate(&at, &certified.clone().complement(), &mut context).unwrap(),
            complement(selected)
        );
        assert_eq!(context.current_workspace_bytes(), 0);
    }
}

#[test]
fn complemented_native_union_removes_internal_vertices_and_preserves_outer_boundary() {
    // Sixteen original edges retain all polygon incidences. Admit their
    // complete contact/sector inventory explicitly; the default two-polygon
    // wall and pole controls above retain their original admission.
    let policy = ExecutionPolicy::new(ExecutionLimits {
        max_work_items: 2_000_000,
        ..ExecutionLimits::GEOMETRY
    })
    .unwrap();
    let mut context = MetricContext::new(GeographicReference::wgs84(), policy).unwrap();
    let mut polygons = Vec::new();
    for (west, east, south, north) in [(-4, 0, -4, 0), (0, 4, -4, 0), (-4, 0, 0, 4), (0, 4, 0, 4)] {
        let vertices = [
            (west, south),
            (east, south),
            (east, north),
            (west, north),
            (west, south),
        ]
        .map(|(x, y)| Coord::xy(Rat::from_i64(x), Rat::from_i64(y)));
        let ring = PreparedCurve::from_source(&vertices, context.reference()).unwrap();
        polygons.push(
            PreparedPolygon::from_curves(vec![ring], crate::OrientedInterior::Left, &mut context)
                .unwrap(),
        );
    }
    let positive = PreparedRegion::polygons(polygons);
    let complement = positive.clone().complement();
    for (at, selected) in [
        (point(0, 0), Set::Exterior),
        (point(0, 2), Set::Exterior),
        (point(0, 4), Set::Boundary),
        (point(4, 4), Set::Boundary),
        (point(5, 0), Set::Interior),
    ] {
        assert_eq!(locate(&at, &complement, &mut context).unwrap(), selected);
    }
    assert_eq!(
        locate(&point(0, 0), &positive, &mut context).unwrap(),
        Set::Interior
    );
    let support = PreparedCurve::from_source(
        &[
            Coord::xy(Rat::zero(), Rat::zero()),
            Coord::xy(Rat::one(), Rat::zero()),
        ],
        context.reference(),
    )
    .unwrap();
    let collapsed = PreparedRegion::polygons(vec![
        PreparedPolygon::from_selected_support(vec![support], context.reference()).unwrap(),
    ])
    .complement();
    for at in [point(0, 0), point(1, 0), point(0, 1)] {
        assert_eq!(
            locate(&at, &collapsed, &mut context).unwrap(),
            Set::Interior
        );
    }
}

#[test]
fn coordinate_linear_whole_surface_carrier_has_no_physical_boundary() {
    let whole = region("MULTIPOLYGON (((-180 -90,180 -90,180 90,-180 90,-180 -90)))");
    assert!(matches!(whole, PreparedRegion::Polygons(_)));
    let complement = whole.clone().complement();
    let mut context = MetricContext::wgs84().unwrap();
    for latitude in [-90, -89, -45, 0, 45, 89, 90] {
        for longitude in [-180, -179, -90, 0, 90, 179, 180] {
            let point = point(longitude, latitude);
            assert_eq!(whole.locate(&point, &mut context).unwrap(), Set::Interior);
            assert_eq!(
                complement.locate(&point, &mut context).unwrap(),
                Set::Exterior,
            );
        }
    }
    context.begin(1).unwrap();
    let boundary =
        boundary::region_boundary(&whole, &mut context, &mut WorkProgress::new(None)).unwrap();
    assert_eq!(boundary.edges, [] as [[LonLat; 2]; 0]);
    assert_eq!(boundary.curves, [] as [PreparedCurve; 0]);
    assert!(boundary.native.is_none());
    context.release_workspace(boundary.workspace_bytes).unwrap();
}

#[test]
fn complete_written_fragments_classify_only_their_two_physical_sides() {
    let square =
        region("POLYGON ((-10 -10,10 -10,10 10,-10 10,-10 -10),(-2 -2,2 -2,2 2,-2 2,-2 -2))");
    let joined = region(
        "MULTIPOLYGON (((-10 -10,0 -10,0 10,-10 10,-10 -10)),((0 -10,10 -10,10 10,0 10,0 -10)))",
    );
    let band = region("POLYGON ((-180 -10,180 -10,180 10,-180 10,-180 -10))");
    let pole = region("POLYGON ((-180 80,180 80,180 90,-180 90,-180 80))");
    let xy = |x, y| Coord::xy(Rat::from_i64(x), Rat::from_i64(y));
    for (region, at, from, to, expected) in [
        (&square, xy(10, 0), xy(10, -10), xy(10, 10), true),
        (&square, xy(2, 0), xy(2, -2), xy(2, 2), true),
        (&joined, xy(0, 0), xy(0, -10), xy(0, 10), false),
        (&band, xy(180, 0), xy(180, -10), xy(180, 10), false),
        (&pole, xy(0, 90), xy(-180, 90), xy(180, 90), false),
    ] {
        for region in [region.clone(), region.clone().complement()] {
            let mut context = MetricContext::wgs84().unwrap();
            context.begin(1).unwrap();
            assert_eq!(
                written_boundary_sides(
                    &at,
                    &from,
                    &to,
                    &region,
                    &mut context,
                    &mut WorkProgress::new(None)
                )
                .unwrap(),
                expected,
            );
        }
    }
}

#[test]
fn unrounded_boxes_prove_uniform_union_membership_without_export_samples() {
    use purrdf_xsd::math::{CoordinateMath, MathLimits};
    let mut math = CoordinateMath::new(MathLimits {
        precision_bits: 80,
        max_work: 1_000_000,
        max_workspace_bytes: 64 * 1024 * 1024,
    })
    .expect("bounded math");
    let mut location = |region: &PreparedRegion, w, e, s, n| {
        let longitude =
            crate::numerical::fixed_from_bounds(&Rat::from_i64(w), &Rat::from_i64(e), &mut math)
                .expect("exact degree box");
        let latitude =
            crate::numerical::fixed_from_bounds(&Rat::from_i64(s), &Rat::from_i64(n), &mut math)
                .expect("exact degree box");
        let mut context = MetricContext::wgs84().expect("environment");
        context.begin(1).expect("new request");
        locate_enclosure(
            &longitude,
            &latitude,
            region,
            &mut context,
            &mut WorkProgress::new(None),
        )
        .unwrap_or_else(|error| {
            panic!(
                "complete uniform membership ({w},{e},{s},{n}): {error:?}, work={}",
                context.work_items()
            )
        })
    };
    let square =
        region("POLYGON ((-10 -10,10 -10,10 10,-10 10,-10 -10),(-2 -2,2 -2,2 2,-2 2,-2 -2))");
    assert_eq!(location(&square, 4, 6, 4, 6), Some(Set::Interior));
    assert_eq!(location(&square, -1, 1, -1, 1), Some(Set::Exterior));
    assert_eq!(location(&square, 1, 3, 0, 1), None);
    assert_eq!(location(&square, 10, 10, 0, 0), Some(Set::Boundary));
    let cap = region("POLYGON ((-180 80,180 80,180 90,-180 90,-180 80))");
    assert_eq!(location(&cap, 170, 180, 85, 90), Some(Set::Interior));
    assert_eq!(location(&cap, -180, 180, 90, 90), Some(Set::Interior));
    assert_eq!(location(&cap, 170, 180, 75, 85), None);
    assert_eq!(
        location(&PreparedRegion::Whole, -180, 180, -90, 90),
        Some(Set::Interior)
    );
}
fn check(region: &PreparedRegion, points: &[(i64, i64, Set)]) {
    let mut context = MetricContext::wgs84().expect("context");
    for &(lon, lat, expected) in points {
        assert_eq!(
            region
                .locate(&point(lon, lat), &mut context)
                .expect("exact atlas"),
            expected,
            "({lon},{lat})"
        );
    }
}

#[test]
fn source_linear_long_paths_holes_and_complements_keep_explicit_interiors() {
    let written =
        region("POLYGON ((170 -10,-170 -10,-170 10,170 10,170 -10),(-5 -5,5 -5,5 5,-5 5,-5 -5))");
    check(
        &written,
        &[
            (0, 0, Set::Exterior),
            (0, 7, Set::Interior),
            (175, 0, Set::Exterior),
            (170, 0, Set::Boundary),
            (-170, 0, Set::Boundary),
            (5, 0, Set::Boundary),
        ],
    );
    let PreparedRegion::Polygons(polygons) = written else {
        panic!("polygon")
    };
    let complement = PreparedRegion::polygons(
        polygons
            .iter()
            .map(|polygon| {
                let GeometryBody::Polygon(rings) =
                    polygon.chart().expect("written source chart").body()
                else {
                    panic!("chart")
                };
                PreparedPolygon::from_source(
                    rings,
                    &GeographicReference::wgs84(),
                    RegionInterior::Complement,
                )
                .expect("complement")
            })
            .collect(),
    );
    check(
        &complement,
        &[
            (0, 0, Set::Interior),
            (0, 7, Set::Exterior),
            (175, 0, Set::Interior),
            (170, 0, Set::Boundary),
        ],
    );
    check(
        &PreparedRegion::Whole,
        &[(0, 0, Set::Interior), (180, 90, Set::Interior)],
    );
    check(
        &PreparedRegion::Empty,
        &[(0, 0, Set::Exterior), (-180, -90, Set::Exterior)],
    );
}

#[test]
fn longitude_cut_edges_are_removed_only_when_both_physical_sides_are_inside() {
    let band = region("POLYGON ((-180 -10,180 -10,180 10,-180 10,-180 -10))");
    check(
        &band,
        &[
            (-180, 0, Set::Interior),
            (180, 0, Set::Interior),
            (-180, 10, Set::Boundary),
            (180, -10, Set::Boundary),
            (0, 11, Set::Exterior),
        ],
    );
    let wedge = region("POLYGON ((170 -10,180 -10,180 10,170 10,170 -10))");
    check(
        &wedge,
        &[
            (180, 0, Set::Boundary),
            (-180, 0, Set::Boundary),
            (175, 0, Set::Interior),
            (-175, 0, Set::Exterior),
        ],
    );
    let split = region("MULTIPOLYGON (((-2 -2,0 -2,0 2,-2 2,-2 -2)),((0 -2,2 -2,2 2,0 2,0 -2)))");
    check(
        &split,
        &[
            (0, 0, Set::Interior),
            (0, 2, Set::Boundary),
            (-2, 0, Set::Boundary),
        ],
    );
}

#[test]
fn exact_poles_identify_all_longitudes_and_preserve_partial_wedges() {
    let cap = region("POLYGON ((-180 80,180 80,180 90,-180 90,-180 80))");
    check(
        &cap,
        &[
            (-180, 90, Set::Interior),
            (0, 90, Set::Interior),
            (180, 90, Set::Interior),
            (20, 85, Set::Interior),
            (20, 80, Set::Boundary),
            (0, 79, Set::Exterior),
        ],
    );
    let wedge = region("POLYGON ((0 80,10 80,10 90,0 90,0 80))");
    check(
        &wedge,
        &[
            (-180, 90, Set::Boundary),
            (0, 90, Set::Boundary),
            (100, 90, Set::Boundary),
            (5, 85, Set::Interior),
            (20, 85, Set::Exterior),
        ],
    );
    let south = region("POLYGON ((-180 -90,180 -90,180 -80,-180 -80,-180 -90))");
    check(
        &south,
        &[(0, -90, Set::Interior), (180, -90, Set::Interior)],
    );
}

#[test]
fn atlas_reference_work_memory_and_cancellation_refusals_are_fatal() {
    let source = region("POLYGON ((0 0,2 0,2 2,0 2,0 0))");
    let mut other =
        MetricContext::new(GeographicReference::cgcs2000(), ExecutionPolicy::geometry())
            .expect("context");
    assert!(matches!(
        source.locate(&point(1, 1), &mut other),
        Err(GeoError::MissingOperation { .. })
    ));
    for (limits, memory) in [
        (
            ExecutionLimits {
                max_work_items: 1,
                ..ExecutionLimits::GEOMETRY
            },
            false,
        ),
        (
            ExecutionLimits {
                max_workspace_bytes: 65_535,
                ..ExecutionLimits::GEOMETRY
            },
            true,
        ),
    ] {
        let mut context = MetricContext::new(
            GeographicReference::wgs84(),
            ExecutionPolicy::new(limits).expect("policy"),
        )
        .expect("context");
        let error = source
            .locate(&point(0, 0), &mut context)
            .expect_err("complete refusal");
        if memory {
            assert!(matches!(error, GeoError::MemoryExhausted { .. }));
        } else {
            assert!(matches!(error, GeoError::WorkExhausted { .. }));
        }
        assert!(!error.is_expression_error());
    }
    let mut context = MetricContext::wgs84().expect("context");
    context.cancel();
    assert!(matches!(
        source.locate(&point(1, 1), &mut context),
        Err(GeoError::Cancelled)
    ));
}

#[test]
fn whole_union_complement_uses_de_morgan_and_removes_internal_cut_edges() {
    let source =
        region("MULTIPOLYGON (((-4 -2,-2 -2,-2 2,-4 2,-4 -2)),((2 -2,4 -2,4 2,2 2,2 -2)))");
    let complementary = source.clone().complement();
    check(
        &complementary,
        &[
            (-3, 0, Set::Exterior),
            (3, 0, Set::Exterior),
            (0, 0, Set::Interior),
            (2, 0, Set::Boundary),
        ],
    );
    assert_eq!(complementary.complement(), source);
    let cap = region("POLYGON ((-180 80,180 80,180 90,-180 90,-180 80))");
    check(
        &cap.complement(),
        &[
            (0, 90, Set::Exterior),
            (180, 90, Set::Exterior),
            (-180, 85, Set::Exterior),
            (0, 80, Set::Boundary),
            (0, 70, Set::Interior),
        ],
    );
    let whole = region("POLYGON ((-180 -90,180 -90,180 90,-180 90,-180 -90))");
    check(
        &whole.clone().complement(),
        &[
            (-180, 0, Set::Exterior),
            (0, 90, Set::Exterior),
            (0, -90, Set::Exterior),
        ],
    );
    let PreparedRegion::Polygons(polygons) = whole else {
        panic!("polygon")
    };
    check(
        &PreparedRegion::polygons(vec![
            polygons[0]
                .clone()
                .with_interior(RegionInterior::Complement),
        ]),
        &[(180, 0, Set::Exterior), (0, 90, Set::Exterior)],
    );
    let collapsed = region("POLYGON ((0 0,0 0,0 0,0 0))");
    check(&collapsed, &[(0, 0, Set::Boundary), (1, 1, Set::Exterior)]);
    check(
        &collapsed.complement(),
        &[(0, 0, Set::Interior), (1, 1, Set::Interior)],
    );
}
