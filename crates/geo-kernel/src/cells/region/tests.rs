// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use super::super::NativeGridProfile;
use super::*;
use crate::atlas::segment_rectangle;
use crate::{
    Coord, Crs, GeoProfile, GeographicReference, GeometryBody, MetricContext, PreparedGeometry,
    PreparedPolygon, Rat, RegionInterior,
};
use purrdf_iri::vocab::ogc;

fn region(text: &str) -> PreparedRegion {
    let literal = crate::wkt::parse(text, &Crs::new(ogc::CRS84).expect("IRI")).expect("carrier");
    PreparedGeometry::from_literal(&literal, &GeoProfile::standard())
        .expect("prepared")
        .region()
        .clone()
}
fn limits() -> MixedCoverLimits {
    MixedCoverLimits {
        max_work_items: 100_000_000,
        ..MixedCoverLimits::DEFAULT
    }
}
fn point(lon: i64, lat: i64) -> crate::LonLat {
    crate::LonLat::new(Rat::from_i64(lon), Rat::from_i64(lat)).expect("geography")
}

#[test]
fn prepared_holes_seams_poles_and_complements_cover_every_qualifying_key() {
    let mut regions = vec![
        region("POLYGON ((-20 -20,20 -20,20 20,-20 20,-20 -20),(-5 -5,5 -5,5 5,-5 5,-5 -5))"),
        region(
            "MULTIPOLYGON (((170 -10,180 -10,180 10,170 10,170 -10)),((-180 -10,-170 -10,-170 10,-180 10,-180 -10)))",
        ),
        region("POLYGON ((-180 80,180 80,180 90,-180 90,-180 80))"),
    ];
    let PreparedRegion::Polygons(polygons) = &regions[0] else {
        panic!("polygon")
    };
    let GeometryBody::Polygon(rings) = polygons[0].chart().expect("written source chart").body()
    else {
        panic!("chart")
    };
    regions.push(PreparedRegion::polygons(vec![
        PreparedPolygon::from_source(
            rings,
            &GeographicReference::wgs84(),
            RegionInterior::Complement,
        )
        .expect("explicit complement"),
    ]));
    regions.push(regions[1].clone().complement());
    let grid = CubeHilbertQ62V1::new(NativeGridProfile::Wgs84);
    let mut context = MetricContext::wgs84().expect("context");
    for region in &regions {
        let cover = grid
            .cover_region_mixed(region, CoverLevels::new(0, 4).expect("levels"), limits())
            .expect("complete region cover");
        assert_eq!(cover.law_id(), CoverLawId::prepared_region());
        assert_ne!(cover.law_id(), CoverLawId::native());
        for lat in (-90..=90).step_by(5) {
            for lon in (-180..=180).step_by(5) {
                let point = point(lon, lat);
                if region
                    .locate(&point, &mut context)
                    .expect("exact membership")
                    != Set::Exterior
                {
                    assert!(
                        cover
                            .contains(grid.assign(&point, 4).expect("key"))
                            .expect("profile"),
                        "missed ({lon},{lat})"
                    );
                }
            }
        }
        for pair in cover.cells().windows(2) {
            assert!(pair[0].range_max() < pair[1].range_min());
        }
        let raised = grid
            .cover_region_mixed(
                region,
                CoverLevels::new(0, 4).expect("levels"),
                MixedCoverLimits {
                    max_work_items: 200_000_000,
                    ..limits()
                },
            )
            .expect("adequate policy");
        assert_eq!(cover.cells(), raised.cells());
    }
}

#[test]
fn whole_empty_fixed_mixed_and_complete_caps_preserve_the_level_contract() {
    let grid = CubeHilbertQ62V1::new(NativeGridProfile::Wgs84);
    let whole = grid
        .cover_region_mixed(
            &PreparedRegion::Whole,
            CoverLevels::new(0, 30).expect("levels"),
            limits(),
        )
        .expect("six roots");
    assert_eq!(whole.cells().len(), 6);
    assert!(whole.cells().iter().all(|cell| cell.level() == 0));
    assert_eq!(
        grid.cover_region_mixed(
            &PreparedRegion::Empty,
            CoverLevels::new(0, 30).expect("levels"),
            limits()
        )
        .expect("empty")
        .cells(),
        []
    );
    let fixed_limits = FixedCoverLimits {
        max_work_items: 100_000_000,
        ..FixedCoverLimits::DEFAULT
    };
    let fixed = grid
        .cover_region_fixed(&PreparedRegion::Whole, 2, fixed_limits)
        .expect("96 cells");
    assert_eq!(fixed.cells().len(), 96);
    assert_eq!(
        fixed.cells(),
        grid.cover_region_mixed(
            &PreparedRegion::Whole,
            CoverLevels::new(2, 2).expect("same levels"),
            limits()
        )
        .expect("mixed equal levels")
        .cells()
    );
    assert!(matches!(
        grid.cover_region_fixed(
            &PreparedRegion::Whole,
            2,
            FixedCoverLimits {
                max_logical_cells: 95,
                ..fixed_limits
            }
        ),
        Err(GeoError::CoverCellsExhausted { limit: 95 })
    ));
    let shape = region("POLYGON ((0 0,20 0,20 20,0 20,0 0))");
    let cover = grid
        .cover_region_mixed(&shape, CoverLevels::new(0, 3).expect("levels"), limits())
        .expect("complete");
    let count = cover.cells().len() as u64;
    assert_eq!(
        cover.cells(),
        grid.cover_region_mixed(
            &shape,
            CoverLevels::new(0, 3).expect("levels"),
            MixedCoverLimits {
                max_emitted_cells: count,
                ..limits()
            }
        )
        .expect("exact cap")
        .cells()
    );
    assert!(matches!(
        grid.cover_region_mixed(
            &shape,
            CoverLevels::new(0, 3).expect("levels"),
            MixedCoverLimits {
                max_emitted_cells: count - 1,
                ..limits()
            }
        ),
        Err(GeoError::CoverCellsExhausted { .. })
    ));
    assert!(matches!(
        CubeHilbertQ62V1::new(NativeGridProfile::Cgcs2000).cover_region_mixed(
            &shape,
            CoverLevels::new(0, 3).expect("levels"),
            limits()
        ),
        Err(GeoError::MissingOperation { .. })
    ));
}

#[test]
fn exact_rectangle_separation_includes_tangencies_and_degenerate_boundaries() {
    let coord = |x, y| Coord::xy(Rat::from_i64(x), Rat::from_i64(y));
    let west = Rat::zero();
    let east = Rat::one();
    let south = Rat::zero();
    let north = Rat::one();
    for (a, b, expected) in [
        (coord(-1, 1), coord(2, 1), true),
        (coord(-1, -1), coord(0, 0), true),
        (coord(0, 0), coord(0, 0), true),
        (coord(-1, 2), coord(2, 2), false),
    ] {
        assert_eq!(
            segment_rectangle(&a, &b, &west, &east, &south, &north),
            expected
        );
        let mut context = MetricContext::wgs84().expect("context");
        context.begin(1).expect("entry");
        assert_eq!(
            crate::atlas::segment_rectangle_admitted(
                &a,
                &b,
                [&west, &east, &south, &north],
                &mut crate::numerical::ExactAdmission::new(
                    &mut context,
                    &mut crate::context::WorkProgress::new(None),
                ),
            )
            .expect("complete actual-pair admission"),
            expected,
        );
    }
}

#[test]
fn internal_union_walls_do_not_change_the_complete_physical_cover() {
    let grid = CubeHilbertQ62V1::new(NativeGridProfile::Wgs84);
    let single = region("POLYGON ((-20 -20,20 -20,20 20,-20 20,-20 -20))");
    let joined = region(
        "MULTIPOLYGON (((-20 -20,0 -20,0 20,-20 20,-20 -20)),((0 -20,20 -20,20 20,0 20,0 -20)))",
    );
    for (a, b) in [
        (single.clone(), joined.clone()),
        (single.complement(), joined.complement()),
    ] {
        let levels = CoverLevels::new(0, 3).expect("levels");
        let whole_boundary = grid
            .cover_region_mixed(&a, levels, limits())
            .expect("complete single boundary");
        let selected_union = grid
            .cover_region_mixed(&b, levels, limits())
            .expect("complete selected union boundary");
        assert_eq!(whole_boundary.cells(), selected_union.cells());
        assert_eq!(whole_boundary.law_id(), selected_union.law_id());
    }
}

#[test]
fn selected_curve_and_isolated_point_covers_match_their_complete_closed_support() {
    let mut context = MetricContext::wgs84().unwrap();
    let grid = CubeHilbertQ62V1::new(NativeGridProfile::Wgs84);
    let levels = CoverLevels::new(0, 3).unwrap();
    for (rings, south) in [
        (
            [
                &[(0, 0), (1, 0), (1, 1), (0, 1), (0, 0)][..],
                &[(1, 0), (2, 0), (2, 1), (1, 1), (1, 0)][..],
            ],
            0,
        ),
        (
            [
                &[(0, 0), (1, 0), (1, 1), (0, 1), (0, 0)][..],
                &[(1, 1), (2, 1), (2, 2), (1, 2), (1, 1)][..],
            ],
            1,
        ),
    ] {
        let native = crate::atlas::test_closed_intersection(&rings, &mut context);
        let box_support =
            super::super::ClosedBox::new(Rat::one(), Rat::from_i64(south), Rat::one(), Rat::one())
                .unwrap();
        let source = [south, 1].map(|latitude| Coord::xy(Rat::one(), Rat::from_i64(latitude)));
        let curve = crate::PreparedCurve::from_source(&source, context.reference()).unwrap();
        let direct = PreparedRegion::polygons(vec![
            PreparedPolygon::from_selected_support(vec![curve], context.reference()).unwrap(),
        ]);
        for region in [native.region(), &direct] {
            let cover = grid.cover_region_mixed(region, levels, limits()).unwrap();
            let control = grid
                .cover_box_mixed(&box_support, levels, limits())
                .unwrap();
            assert_eq!(cover.cells(), control.cells());
            assert_eq!(cover.law_id(), CoverLawId::prepared_region());
            assert!(
                cover
                    .contains(grid.assign(&point(1, 1), 3).unwrap())
                    .unwrap()
            );
            let raised = grid
                .cover_region_mixed(
                    region,
                    levels,
                    MixedCoverLimits {
                        max_work_items: 200_000_000,
                        ..limits()
                    },
                )
                .unwrap();
            assert_eq!(cover.cells(), raised.cells());
            for (work, memory) in [
                (1, limits().max_workspace_bytes),
                (limits().max_work_items, 1),
            ] {
                let result = grid.cover_region_mixed(
                    region,
                    levels,
                    MixedCoverLimits {
                        max_work_items: work,
                        max_workspace_bytes: memory,
                        ..limits()
                    },
                );
                assert!(matches!(
                    result,
                    Err(GeoError::WorkExhausted { .. } | GeoError::MemoryExhausted { .. })
                ));
            }
            let mut cancel = super::super::cover::tests::RefusingObserver::after(128);
            assert_eq!(
                grid.cover_region_mixed_metered(region, levels, limits(), &mut cancel),
                Err(GeoError::Cancelled)
            );
        }
    }
}

#[test]
fn intersecting_native_geodesic_regions_keep_complete_original_fragment_covers() {
    use crate::{
        ExecutionLimits, ExecutionPolicy, OrientedInterior, PreparedCoordinate, PreparedCurve,
        PreparedEdge, ShortestGeodesicArc,
    };
    let mut policy = ExecutionLimits::GEOMETRY;
    policy.max_work_items = 200_000_000;
    let mut context = MetricContext::new(
        GeographicReference::wgs84(),
        ExecutionPolicy::new(policy).unwrap(),
    )
    .unwrap();
    let mut rectangle = |bounds: [i64; 4]| {
        let [west, east, south, north] = bounds;
        let coordinates = [
            (west, south),
            (east, south),
            (east, north),
            (west, north),
            (west, south),
        ];
        let mut edges = Vec::new();
        for pair in coordinates.windows(2) {
            let endpoint = |(x, y)| {
                PreparedCoordinate::new(
                    Coord::xy(Rat::from_i64(x), Rat::from_i64(y)),
                    &GeographicReference::wgs84(),
                )
                .unwrap()
            };
            edges.push(PreparedEdge::ShortestGeodesic(Box::new(
                ShortestGeodesicArc::new(endpoint(pair[0]), endpoint(pair[1]), &mut context)
                    .unwrap_or_else(|error| panic!("original edge {pair:?}: {error:?}")),
            )));
        }
        PreparedPolygon::from_curves(
            vec![PreparedCurve::new(edges)],
            OrientedInterior::Left,
            &mut context,
        )
        .unwrap()
    };
    let native = PreparedRegion::polygons(vec![rectangle([-4, 4, -4, 4]), rectangle([0, 8, 0, 8])]);
    let grid = CubeHilbertQ62V1::new(NativeGridProfile::Wgs84);
    let levels = CoverLevels::new(0, 2).unwrap();
    let admission = MixedCoverLimits {
        max_work_items: 200_000_000,
        ..MixedCoverLimits::DEFAULT
    };
    for shape in [native.clone(), native.complement()] {
        let cover = grid.cover_region_mixed(&shape, levels, admission).unwrap();
        let raised = grid
            .cover_region_mixed(
                &shape,
                levels,
                MixedCoverLimits {
                    max_work_items: 400_000_000,
                    ..admission
                },
            )
            .unwrap();
        assert_eq!(cover.cells(), raised.cells());
        for (longitude, latitude) in [
            (-4, -4),
            (4, -4),
            (4, 4),
            (-4, 4),
            (0, 0),
            (8, 0),
            (8, 8),
            (0, 8),
            (-2, -2),
            (2, 2),
            (6, 6),
            (-90, -45),
            (90, 45),
            (180, 90),
            (-180, -90),
        ] {
            let point = point(longitude, latitude);
            if shape.locate(&point, &mut context).unwrap() != Set::Exterior {
                assert!(cover.contains(grid.assign(&point, 2).unwrap()).unwrap());
            }
        }
    }
}
