// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use super::*;
use crate::cells::{PointCellIndex, PointIndexLimits, PointIndexPoint};
use crate::geodesic::PreparedGeodesic;

fn grid() -> CubeHilbertQ62V1 {
    CubeHilbertQ62V1::new(NativeGridProfile::Wgs84)
}
fn point(lon: i64, lat: i64) -> LonLat {
    LonLat::new(Rat::from_i64(lon), Rat::from_i64(lat)).expect("valid test point")
}
fn admitted() -> MixedCoverLimits {
    MixedCoverLimits {
        max_work_items: 100_000_000,
        ..MixedCoverLimits::DEFAULT
    }
}
fn points() -> Vec<LonLat> {
    let mut values = Vec::new();
    for lat in (-90..=90).step_by(15) {
        for lon in (-180..=180).step_by(15) {
            values.push(point(lon, lat));
        }
    }
    values
}
fn canonical(cover: &CellCover) {
    for pair in cover.cells().windows(2) {
        assert!(pair[0].key() < pair[1].key());
        assert!(pair[0].range_max() < pair[1].range_min());
    }
    assert_eq!(
        cover.cells().len() as u64,
        cover.receipt.level_histogram.iter().sum::<u64>()
    );
}

#[test]
fn sequential_cover_workers_reuse_the_largest_admitted_source_arena() {
    let mut budget = CoverBudget::new(admitted()).expect("cover admission");
    let reference = GeographicReference::wgs84();
    let cold = budget.context(reference.clone()).expect("initial arena");
    let initial_limbs = cold.integer_scratch().unwrap().limb_capacity();
    drop(cold);

    let original = Rat::new(Int::one(), Int::one().shl(4096)).expect("wide exact source");
    let grown = budget
        .context_for_sources(reference.clone(), &[&original])
        .expect("admitted source-aware arena growth");
    let grown_limbs = grown.integer_scratch().unwrap().limb_capacity();
    assert!(grown_limbs > initial_limbs);
    let retained = budget.retained;
    drop(grown);

    // This executable installs CountingAllocator in geodesic::preparation's
    // test module. Check the actual allocator before claiming a zero count.
    let instrument = purrdf_alloc_probe::CurrentThreadWindow::open();
    std::hint::black_box(Vec::<u8>::with_capacity(512));
    assert_eq!(instrument.close().allocations, 1);
    let measured = purrdf_alloc_probe::CurrentThreadWindow::open();
    let reused = budget
        .context_for_sources(reference, &[&original])
        .expect("borrow the existing larger source arena");
    let allocations = measured.close();
    assert_eq!(allocations.allocations, 0);
    assert_eq!(
        reused.integer_scratch().unwrap().limb_capacity(),
        grown_limbs
    );
    assert_eq!(budget.retained, retained);
}

#[test]
fn cover_coordinates_independently_reverse_hilbert_at_every_small_level() {
    for level in 0..=6 {
        for face in 0..6 {
            for path in 0..1_u64 << (2 * level) {
                let cell =
                    CellId::from_path(grid().profile_id(), face, path, level).expect("valid");
                let (x, y) = coordinates(cell);
                assert_eq!(
                    super::super::hilbert_path(
                        u32::try_from(x).expect("small"),
                        u32::try_from(y).expect("small"),
                        level
                    ),
                    path
                );
                let footprint = Footprint::new(cell).expect("chart");
                assert!(footprint.radius > Rat::zero());
                let guard = Rat::new(Int::one(), Int::one().shl(59)).expect("positive guard");
                assert!(footprint.radius <= Rat::from_i64(2).add(&guard));
            }
        }
    }
}

#[test]
fn full_surface_fixed_counts_logical_cells_and_mixed_coalesces() {
    let area = ClosedBox::new(
        Rat::from_i64(-180),
        Rat::from_i64(-90),
        Rat::from_i64(180),
        Rat::from_i64(90),
    )
    .expect("whole surface");
    let mixed = grid()
        .cover_box_mixed(&area, CoverLevels::new(0, 6).expect("levels"), admitted())
        .expect("whole surface mixed");
    assert_eq!(mixed.cells().len(), 6);
    assert!(mixed.cells().iter().all(|cell| cell.level() == 0));
    canonical(&mixed);
    let fixed = grid()
        .cover_box_fixed(
            &area,
            2,
            FixedCoverLimits {
                max_logical_cells: 96,
                max_work_items: 100_000_000,
                ..FixedCoverLimits::DEFAULT
            },
        )
        .expect("96 cells");
    assert_eq!(fixed.cells().len(), 96);
    assert!(fixed.cells().iter().all(|cell| cell.level() == 2));
    assert!(matches!(
        grid().cover_box_fixed(
            &area,
            2,
            FixedCoverLimits {
                max_logical_cells: 95,
                max_work_items: 100_000_000,
                ..FixedCoverLimits::DEFAULT
            }
        ),
        Err(GeoError::CoverCellsExhausted { limit: 95 })
    ));
    assert_eq!(
        fixed.cells(),
        grid()
            .cover_box_mixed(
                &area,
                CoverLevels::new(2, 2).expect("levels"),
                MixedCoverLimits {
                    max_emitted_cells: 96,
                    ..admitted()
                }
            )
            .expect("mixed same level")
            .cells()
    );
}

#[test]
fn closed_boxes_preserve_wrapping_meridians_and_pole_aliases() {
    let areas = [
        ClosedBox::new(
            Rat::from_i64(170),
            Rat::from_i64(-30),
            Rat::from_i64(-170),
            Rat::from_i64(30),
        )
        .expect("wrap"),
        ClosedBox::new(
            Rat::from_i64(180),
            Rat::from_i64(-90),
            Rat::from_i64(180),
            Rat::from_i64(90),
        )
        .expect("cut meridian"),
        ClosedBox::new(
            Rat::zero(),
            Rat::from_i64(75),
            Rat::zero(),
            Rat::from_i64(90),
        )
        .expect("polar meridian"),
        ClosedBox::new(
            Rat::from_i64(-135),
            Rat::from_i64(-45),
            Rat::from_i64(135),
            Rat::from_i64(45),
        )
        .expect("wide wedge"),
        ClosedBox::new(
            Rat::from_i64(-180),
            Rat::zero(),
            Rat::from_i64(180),
            Rat::zero(),
        )
        .expect("equator"),
    ];
    for area in &areas {
        let cover = grid()
            .cover_box_mixed(area, CoverLevels::new(0, 3).expect("levels"), admitted())
            .expect("closed cover");
        canonical(&cover);
        for point in points() {
            if area.contains(&point) {
                assert!(
                    cover
                        .contains(grid().assign(&point, 3).expect("key"))
                        .expect("profile"),
                    "missed {point:?}"
                );
            }
        }
    }
    assert!(areas[1].contains(&point(-180, 0)));
    assert!(areas[2].contains(&point(-123, 90)));
    assert!(!areas[2].contains(&point(15, 75)));
}

#[test]
fn disk_covers_are_complete_under_both_public_threshold_laws() {
    let center = point(0, 0);
    let prepared = PreparedGeodesic::new(GeographicReference::wgs84());
    for radius in [0, 1, 1_000, 30_000, 400_000, 20_000_000] {
        let radius = Metres::new(Rat::from_i64(radius));
        let cover = grid()
            .cover_disk_mixed(
                &center,
                &radius,
                CoverLevels::new(0, 3).expect("levels"),
                admitted(),
            )
            .expect("physical disk");
        canonical(&cover);
        let threshold = XsdDoubleMetres::new(radius.exact().to_f64()).expect("finite");
        let reported = grid()
            .cover_reported_disk_mixed(
                &center,
                threshold,
                CoverLevels::new(0, 3).expect("levels"),
                admitted(),
            )
            .expect("reported disk");
        canonical(&reported);
        for candidate in [
            point(0, 0),
            point(15, 0),
            point(0, 15),
            point(180, 0),
            point(0, 90),
            point(90, 45),
        ] {
            let mut context = MetricContext::wgs84().expect("context");
            let answer = prepared
                .distance(&center, &candidate, &mut context)
                .expect("distance");
            let key = grid().assign(&candidate, 3).expect("key");
            if answer.value().exact().add(answer.rounding_bound().exact()) <= *radius.exact() {
                assert!(cover.contains(key).expect("profile"));
            }
            if answer.within_reported(threshold) {
                assert!(reported.contains(key).expect("profile"));
            }
        }
    }
    assert_eq!(
        grid()
            .cover_reported_disk_mixed(
                &center,
                XsdDoubleMetres::new(-1.0).expect("finite"),
                CoverLevels::new(0, 3).expect("levels"),
                admitted()
            )
            .expect("negative is empty")
            .cells(),
        []
    );
    assert!(matches!(
        grid().cover_disk_mixed(
            &center,
            &Metres::new(Rat::from_i64(-1)),
            CoverLevels::new(0, 3).expect("levels"),
            admitted()
        ),
        Err(GeoError::NegativePhysicalRadius(_))
    ));
}

#[test]
fn seeded_global_disks_preserve_every_certified_qualifying_stored_key() {
    let mut rng = purrdf_testkit::rng::SplitMix64::new(0x2a6f_8319_d007_6b1e);
    for profile in [NativeGridProfile::Wgs84, NativeGridProfile::Cgcs2000] {
        let grid = CubeHilbertQ62V1::new(profile);
        let reference = native_reference(profile);
        let prepared = PreparedGeodesic::new(reference.clone());
        for base_radius in [1, 10, 100, 1_000, 30_000, 400_000, 10_000_000, 20_000_000] {
            let longitude = i64::try_from(rng.below(360_001)).expect("bounded") - 180_000;
            let latitude = i64::try_from(rng.below(180_001)).expect("bounded") - 90_000;
            let center = LonLat::new(
                Rat::new(Int::from_i64(longitude), Int::from_i64(1_000)).expect("decimal"),
                Rat::new(Int::from_i64(latitude), Int::from_i64(1_000)).expect("decimal"),
            )
            .expect("source ranges");
            let radius = Metres::new(Rat::from_i64(base_radius));
            let cover = grid
                .cover_disk_mixed(
                    &center,
                    &radius,
                    CoverLevels::new(0, 4).unwrap(),
                    admitted(),
                )
                .unwrap_or_else(|error| {
                    panic!("complete global disk profile={profile:?} radius={base_radius}: {error}")
                });
            canonical(&cover);
            let ranges = cover.ranges(8).expect("stored ranges");
            let mut candidates = vec![center.clone(), point(-180, 90), point(180, -90)];
            // These exact decimal neighbours densely exercise tiny radii;
            // the remaining stream spans the entire closed coordinate domain.
            for step in -4..=4 {
                let longitude = center.longitude().add(
                    &Rat::new(Int::from_i64(step), Int::from_i64(1_000_000))
                        .expect("nearby exact degree"),
                );
                if let Ok(neighbour) = LonLat::new(longitude, center.latitude().clone()) {
                    candidates.push(neighbour);
                }
            }
            for _ in 0..32 {
                candidates.push(point(
                    i64::try_from(rng.below(361)).expect("bounded") - 180,
                    i64::try_from(rng.below(181)).expect("bounded") - 90,
                ));
            }
            let mut context = MetricContext::new(reference.clone(), ExecutionPolicy::geometry())
                .expect("native context");
            for candidate in candidates {
                if prepared
                    .within_physical(&center, &candidate, &radius, &mut context)
                    .expect("certified physical predicate")
                {
                    let key = grid.assign(&candidate, 8).expect("stored key");
                    assert!(cover.contains(key).expect("profile"));
                    assert!(ranges.iter().any(|range| range.contains(key).unwrap()));
                    assert!(
                        cover
                            .contains(grid.assign(&candidate, 30).unwrap())
                            .unwrap()
                    );
                }
            }
        }
    }
}

#[test]
fn complete_cover_caps_and_adequate_policies_do_not_change_geometry() {
    let area = ClosedBox::new(
        Rat::from_i64(170),
        Rat::from_i64(-15),
        Rat::from_i64(-170),
        Rat::from_i64(15),
    )
    .expect("wrap");
    let levels = CoverLevels::new(0, 3).expect("levels");
    let cover = grid()
        .cover_box_mixed(&area, levels, admitted())
        .expect("complete");
    let count = cover.cells().len() as u64;
    assert!(count > 1);
    let exact = grid()
        .cover_box_mixed(
            &area,
            levels,
            MixedCoverLimits {
                max_emitted_cells: count,
                ..admitted()
            },
        )
        .expect("exact emitted admission");
    assert_eq!(cover.cells(), exact.cells());
    assert_eq!(cover.law_id(), exact.law_id());
    assert!(matches!(
        grid().cover_box_mixed(
            &area,
            levels,
            MixedCoverLimits {
                max_emitted_cells: count - 1,
                ..admitted()
            }
        ),
        Err(GeoError::CoverCellsExhausted { .. })
    ));
    assert!(matches!(
        grid().cover_box_mixed(
            &area,
            levels,
            MixedCoverLimits {
                max_work_items: 1,
                ..admitted()
            }
        ),
        Err(GeoError::WorkExhausted { limit: 1 })
    ));
    assert!(matches!(
        grid().cover_box_mixed(
            &area,
            levels,
            MixedCoverLimits {
                max_workspace_bytes: 65_535,
                ..admitted()
            }
        ),
        Err(GeoError::MemoryExhausted { limit: 65_535 })
    ));
    assert!(!GeoError::CoverCellsExhausted { limit: 1 }.is_expression_error());
    assert!(matches!(
        CoverLevels::new(4, 3),
        Err(GeoError::InvalidCoverLevels { .. })
    ));
}

#[test]
fn physical_zero_index_uses_proved_identity_and_preserves_reported_neighbors() {
    let tiny = LonLat::new(
        Rat::new(Int::one(), Int::one().shl(50)).unwrap(),
        Rat::zero(),
    )
    .unwrap();
    let cases = [
        (point(0, 0), [point(0, 0), point(1, 0), tiny], vec![7]),
        (
            point(180, 0),
            [point(-180, 0), point(180, 0), point(179, 0)],
            vec![7, 11],
        ),
        (
            point(0, 90),
            [point(180, 90), point(-100, 90), point(0, 89)],
            vec![7, 11],
        ),
    ];
    for (case, (center, points, expected)) in cases.into_iter().enumerate() {
        let input = [7, 11, 13]
            .into_iter()
            .zip(points)
            .map(|(key, point)| PointIndexPoint { key, point })
            .collect();
        let index = PointCellIndex::new(
            grid(),
            2,
            GeographicReference::wgs84(),
            None,
            input,
            PointIndexLimits::DEFAULT,
        )
        .unwrap();
        let identity = index.id();
        let zero = Metres::new(Rat::zero());
        assert_eq!(
            index
                .search_physical(
                    &center,
                    &zero,
                    MixedCoverLimits::DEFAULT,
                    PointIndexLimits::DEFAULT,
                )
                .expect("complete exact-zero index search under defaults"),
            expected
        );
        assert_eq!(index.id(), identity);
        assert!(matches!(
            index.search_physical(
                &center,
                &zero,
                MixedCoverLimits {
                    max_work_items: 1,
                    ..MixedCoverLimits::DEFAULT
                },
                PointIndexLimits::DEFAULT,
            ),
            Err(GeoError::WorkExhausted { limit: 1 })
        ));
        let mut cancelled = RefusingObserver {
            work: 0,
            calls: 0,
            refused_after: 0,
        };
        assert_eq!(
            index
                .search_physical_metered(
                    &center,
                    &zero,
                    MixedCoverLimits::DEFAULT,
                    PointIndexLimits::DEFAULT,
                    &mut cancelled,
                )
                .unwrap_err(),
            GeoError::Cancelled
        );
        if case != 0 {
            assert!(matches!(
                index.search_physical(
                    &center,
                    &zero,
                    MixedCoverLimits::DEFAULT,
                    PointIndexLimits {
                        max_points: 1,
                        ..PointIndexLimits::DEFAULT
                    },
                ),
                Err(GeoError::OutputExhausted { limit: 1 })
            ));
        }
        if case == 0 {
            // Positive distances below the frozen report quantum can report
            // zero. That threshold retains its ordinary padded cover path.
            let limits = PointIndexLimits {
                max_work_items: 100_000_000,
                ..PointIndexLimits::DEFAULT
            };
            assert_eq!(
                index
                    .search_reported(
                        &center,
                        XsdDoubleMetres::new(0.0).unwrap(),
                        admitted(),
                        limits,
                    )
                    .unwrap(),
                vec![7, 13]
            );
            let cover = grid()
                .cover_disk_mixed(&center, &zero, CoverLevels::new(0, 2).unwrap(), admitted())
                .unwrap();
            assert!(
                cover.cells().len() > 1,
                "the standalone cover law is retained"
            );
        }
    }
}

#[test]
fn point_index_identity_order_and_refinement_match_scalar_laws() {
    let input = vec![
        PointIndexPoint {
            key: 9,
            point: point(0, 0),
        },
        PointIndexPoint {
            key: 4,
            point: point(15, 0),
        },
        PointIndexPoint {
            key: 3,
            point: point(180, 0),
        },
        PointIndexPoint {
            key: 7,
            point: point(0, 90),
        },
    ];
    let limits = PointIndexLimits {
        max_work_items: 100_000_000,
        ..PointIndexLimits::DEFAULT
    };
    let index = PointCellIndex::new(
        grid(),
        3,
        GeographicReference::wgs84(),
        None,
        input.clone(),
        limits,
    )
    .expect("index");
    let mut reversed = input.clone();
    reversed.reverse();
    assert_eq!(
        index.id(),
        PointCellIndex::new(
            grid(),
            3,
            GeographicReference::wgs84(),
            None,
            reversed,
            limits
        )
        .expect("reordered")
        .id()
    );
    let changed = PointCellIndex::new(
        grid(),
        3,
        GeographicReference::wgs84(),
        Some(GeographicReference::cgcs2000().id()),
        input.clone(),
        limits,
    )
    .expect("declared conversion");
    assert_ne!(index.id(), changed.id());
    let center = point(0, 0);
    for metres in [0, 1_000, 2_000_000, 20_000_000] {
        let radius = Metres::new(Rat::from_i64(metres));
        let threshold = XsdDoubleMetres::new(radius.exact().to_f64()).expect("finite");
        let reported = index
            .search_reported(&center, threshold, admitted(), limits)
            .expect("reported scan");
        let physical = index
            .search_physical(&center, &radius, admitted(), limits)
            .expect("physical scan");
        let mut reported_expected = Vec::new();
        let mut physical_expected = Vec::new();
        let prepared = PreparedGeodesic::new(GeographicReference::wgs84());
        for point in &input {
            let mut context = MetricContext::wgs84().expect("context");
            if prepared
                .distance(&center, &point.point, &mut context)
                .expect("distance")
                .within_reported(threshold)
            {
                reported_expected.push(point.key);
            }
            if prepared
                .within_physical(&center, &point.point, &radius, &mut context)
                .expect("physical comparison")
            {
                physical_expected.push(point.key);
            }
        }
        reported_expected.sort_unstable();
        physical_expected.sort_unstable();
        assert_eq!(reported, reported_expected);
        assert_eq!(physical, physical_expected);
    }
    assert!(matches!(
        PointCellIndex::new(
            grid(),
            3,
            GeographicReference::cgcs2000(),
            None,
            input.clone(),
            limits
        ),
        Err(GeoError::PointIndexReferenceMismatch)
    ));
    let duplicate = vec![input[0].clone(), input[0].clone()];
    assert!(matches!(
        PointCellIndex::new(
            grid(),
            3,
            GeographicReference::wgs84(),
            None,
            duplicate,
            limits
        ),
        Err(GeoError::DuplicatePointKey(9))
    ));
    assert!(matches!(
        PointCellIndex::new(
            grid(),
            3,
            GeographicReference::wgs84(),
            None,
            input,
            PointIndexLimits {
                max_points: 3,
                ..limits
            }
        ),
        Err(GeoError::OutputExhausted { limit: 3 })
    ));
}

#[test]
fn finite_submetre_neighbors_and_quantized_zero_use_distinct_refinement_laws() {
    let center = point(0, 0);
    let neighbor = |text: &str| LonLat::new(decimal(text), Rat::zero()).expect("exact neighbor");
    let source = vec![
        PointIndexPoint {
            key: 0,
            point: center.clone(),
        },
        PointIndexPoint {
            key: 1,
            point: neighbor("0.00000000000449"),
        },
        PointIndexPoint {
            key: 2,
            point: neighbor("-0.00000001"),
        },
        PointIndexPoint {
            key: 3,
            point: neighbor("0.00000001"),
        },
        PointIndexPoint {
            key: 4,
            point: neighbor("0.0001"),
        },
    ];
    let limits = PointIndexLimits {
        max_work_items: 100_000_000,
        ..PointIndexLimits::DEFAULT
    };
    let index = PointCellIndex::new(
        grid(),
        16,
        GeographicReference::wgs84(),
        None,
        source.clone(),
        limits,
    )
    .expect("fine index");
    let zero = Metres::new(Rat::zero());
    assert_eq!(
        index
            .search_physical(&center, &zero, admitted(), limits)
            .expect("true zero"),
        [0]
    );
    assert_eq!(
        index
            .search_reported(
                &center,
                XsdDoubleMetres::new(0.0).expect("zero"),
                admitted(),
                limits
            )
            .expect("reported zero"),
        [0, 1]
    );
    assert_eq!(
        index
            .search_physical(&center, &Metres::new(Rat::one()), admitted(), limits)
            .expect("one metre"),
        [0, 1, 2, 3]
    );
    let physical = grid()
        .cover_disk_mixed(
            &center,
            &Metres::new(Rat::one()),
            CoverLevels::new(0, 16).expect("levels"),
            admitted(),
        )
        .expect("small disk");
    for value in &source[..4] {
        assert!(
            physical
                .contains(grid().assign(&value.point, 16).expect("key"))
                .expect("profile")
        );
    }
    let pole = point(37, 90);
    let polar = grid()
        .cover_disk_mixed(
            &pole,
            &Metres::new(Rat::one()),
            CoverLevels::new(0, 16).expect("levels"),
            admitted(),
        )
        .expect("polar disk");
    for lon in [-180, -123, 0, 37, 180] {
        for lat in ["90", "89.999999999"] {
            let value = LonLat::new(Rat::from_i64(lon), decimal(lat)).expect("polar neighbor");
            assert!(
                polar
                    .contains(grid().assign(&value, 16).expect("key"))
                    .expect("profile")
            );
        }
    }
}

pub(in crate::cells) struct RefusingObserver {
    work: u64,
    calls: u64,
    refused_after: u64,
}
impl RefusingObserver {
    pub(in crate::cells) const fn after(refused_after: u64) -> Self {
        Self {
            work: 0,
            calls: 0,
            refused_after,
        }
    }
}

#[test]
fn original_radius_and_box_precision_are_admitted_before_exact_products() {
    let tiny = Rat::new(Int::one(), Int::one().shl(65_536)).expect("exact positive source");
    let levels = CoverLevels::new(0, 0).expect("roots");
    let center = point(0, 0);
    let radius = Metres::new(tiny.clone());
    let disk = grid()
        .cover_disk_mixed(&center, &radius, levels, MixedCoverLimits::DEFAULT)
        .expect("the complete source fits actual default admission");
    let compact = Metres::new(Rat::new(Int::one(), Int::one().shl(256)).unwrap());
    let ordinary = grid()
        .cover_disk_mixed(&center, &compact, levels, MixedCoverLimits::DEFAULT)
        .expect("independent compact source in the same strict cap bands");
    assert_eq!(disk.cells(), ordinary.cells());
    assert_eq!(disk.law_id(), ordinary.law_id());
    assert!(disk.receipt().work_items <= MixedCoverLimits::DEFAULT.max_work_items);
    assert!(disk.receipt().workspace_peak <= MixedCoverLimits::DEFAULT.max_workspace_bytes);
    assert!(disk.receipt().workspace_peak > ordinary.receipt().workspace_peak);
    assert!(matches!(
        grid().cover_disk_mixed(
            &center,
            &radius,
            levels,
            MixedCoverLimits {
                max_work_items: 10_000,
                ..MixedCoverLimits::DEFAULT
            },
        ),
        Err(GeoError::WorkExhausted { limit: 10_000 })
    ));
    // The measured peak itself is an adequate limit; one byte less refuses.
    let peak = disk.receipt().workspace_peak;
    let exact = grid()
        .cover_disk_mixed(
            &center,
            &radius,
            levels,
            MixedCoverLimits {
                max_workspace_bytes: peak,
                ..MixedCoverLimits::DEFAULT
            },
        )
        .expect("the measured peak admits the same cover");
    assert_eq!(exact.cells(), disk.cells());
    assert!(matches!(
        grid().cover_disk_mixed(
            &center,
            &radius,
            levels,
            MixedCoverLimits {
                max_workspace_bytes: peak - 1,
                ..MixedCoverLimits::DEFAULT
            },
        ),
        Err(GeoError::MemoryExhausted { limit }) if limit == peak - 1
    ));
    let rectangle = ClosedBox::new(tiny, Rat::zero(), Rat::one(), Rat::one())
        .expect("closed original-coordinate box");
    let rectangle = grid().cover_box_mixed(&rectangle, levels, MixedCoverLimits::DEFAULT);
    assert!(
        matches!(rectangle, Err(GeoError::WorkExhausted { limit: 262_144 })),
        "original-box admission: {rectangle:?}"
    );
}

#[test]
fn reported_threshold_conversion_refuses_before_extreme_owner_allocation() {
    let center = point(0, 0);
    let levels = CoverLevels::new(0, 0).unwrap();
    let instrument = purrdf_alloc_probe::CurrentThreadWindow::open();
    std::hint::black_box(Vec::<u8>::with_capacity(512));
    assert_eq!(instrument.close().allocations, 1);
    for bits in [1, f64::MAX.to_bits()] {
        let threshold = XsdDoubleMetres::new(f64::from_bits(bits)).unwrap();
        for limits in [
            MixedCoverLimits {
                max_work_items: 0,
                ..MixedCoverLimits::DEFAULT
            },
            MixedCoverLimits {
                max_work_items: 1,
                ..MixedCoverLimits::DEFAULT
            },
            MixedCoverLimits {
                max_workspace_bytes: 1,
                ..MixedCoverLimits::DEFAULT
            },
        ] {
            let measured = purrdf_alloc_probe::CurrentThreadWindow::open();
            let result = grid().cover_reported_disk_mixed(&center, threshold, levels, limits);
            let allocations = measured.close();
            assert!(matches!(
                result,
                Err(GeoError::InvalidExecutionPolicy(_)
                    | GeoError::WorkExhausted { limit: 1 }
                    | GeoError::MemoryExhausted { limit: 1 })
            ));
            assert_eq!(allocations.allocations, 0);
            assert_eq!(allocations.requested_bytes, 0);
            let mut observer = RefusingObserver::after(u64::MAX);
            let measured = purrdf_alloc_probe::CurrentThreadWindow::open();
            let observed = grid().cover_reported_disk_mixed_metered(
                &center,
                threshold,
                levels,
                limits,
                &mut observer,
            );
            let allocations = measured.close();
            assert_eq!(observed, result);
            assert_eq!(allocations.allocations, 0);
            assert_eq!(allocations.requested_bytes, 0);
            assert!(observer.calls > 0);
        }
        let mut observer = RefusingObserver::after(1);
        let measured = purrdf_alloc_probe::CurrentThreadWindow::open();
        let result = grid().cover_reported_disk_mixed_metered(
            &center,
            threshold,
            levels,
            MixedCoverLimits::DEFAULT,
            &mut observer,
        );
        let allocations = measured.close();
        assert_eq!(result, Err(GeoError::Cancelled));
        assert_eq!(allocations.allocations, 0);
    }
}

#[test]
fn reported_threshold_padding_preserves_original_cover_and_cumulative_receipt() {
    let center = point(0, 0);
    let levels = CoverLevels::new(0, 0).unwrap();
    for bits in [0, 1, f64::MAX.to_bits()] {
        let threshold = XsdDoubleMetres::new(f64::from_bits(bits)).unwrap();
        // Independent original caller radius uses the frozen reported law.
        let exact = Rat::from_binary64(threshold.reported()).unwrap();
        let padding = decimal("0.000001").add(&crate::metric::binary64_half_ulp(bits));
        let radius = Metres::new(exact.add(&padding));
        let physical = grid()
            .cover_disk_mixed(&center, &radius, levels, admitted())
            .unwrap();
        let mut observer = RefusingObserver::after(u64::MAX);
        let reported = grid()
            .cover_reported_disk_mixed_metered(
                &center,
                threshold,
                levels,
                admitted(),
                &mut observer,
            )
            .unwrap();
        assert_eq!(reported.cells(), physical.cells());
        assert_eq!(reported.law_id(), physical.law_id());
        assert!(reported.receipt().work_items > physical.receipt().work_items);
        assert_eq!(observer.work, reported.receipt().work_items);
        assert!(reported.receipt().workspace_peak >= physical.receipt().workspace_peak);
    }
}

#[test]
fn index_admission_accounts_for_owned_input_capacity_before_reconstruction() {
    let maximum = 1_048_576_u64;
    let capacity =
        usize::try_from(maximum).expect("small admission") / size_of::<PointIndexPoint>() + 1;
    let mut values = Vec::with_capacity(capacity);
    values.push(PointIndexPoint {
        key: 1,
        point: point(0, 0),
    });
    assert!(matches!(
        PointCellIndex::new(
            grid(),
            6,
            GeographicReference::wgs84(),
            None,
            values,
            PointIndexLimits {
                max_workspace_bytes: maximum,
                ..PointIndexLimits::DEFAULT
            }
        ),
        Err(GeoError::MemoryExhausted { limit: 1_048_576 })
    ));
}
impl MetricWorkObserver for RefusingObserver {
    fn charge_chunk(&mut self, work: u64, _growth: u64) -> Result<(), GeoError> {
        self.work = self.work.saturating_add(work);
        self.calls += 1;
        if self.work > self.refused_after {
            Err(GeoError::Cancelled)
        } else {
            Ok(())
        }
    }
}

#[test]
fn metered_cover_and_index_refuse_between_complete_bounded_chunks() {
    let whole = ClosedBox::new(
        Rat::from_i64(-180),
        Rat::from_i64(-90),
        Rat::from_i64(180),
        Rat::from_i64(90),
    )
    .expect("whole");
    let mut observer = RefusingObserver {
        work: 0,
        calls: 0,
        refused_after: 128,
    };
    assert!(matches!(
        grid().cover_box_fixed_metered(
            &whole,
            5,
            FixedCoverLimits {
                max_work_items: 100_000_000,
                ..FixedCoverLimits::DEFAULT
            },
            &mut observer
        ),
        Err(GeoError::Cancelled)
    ));
    assert!(observer.calls > 2);
    let values: Vec<_> = (0..256_u64)
        .rev()
        .map(|key| PointIndexPoint {
            key,
            point: point(0, 0),
        })
        .collect();
    let mut observer = RefusingObserver {
        work: 0,
        calls: 0,
        refused_after: 300,
    };
    assert!(matches!(
        PointCellIndex::new_metered(
            grid(),
            6,
            GeographicReference::wgs84(),
            None,
            values,
            PointIndexLimits {
                max_work_items: 100_000_000,
                ..PointIndexLimits::DEFAULT
            },
            &mut observer
        ),
        Err(GeoError::Cancelled)
    ));
    assert!(observer.calls > 3);
    let index = PointCellIndex::new(
        grid(),
        6,
        GeographicReference::wgs84(),
        None,
        vec![
            PointIndexPoint {
                key: 1,
                point: point(1, 1),
            },
            PointIndexPoint {
                key: 2,
                point: point(2, 2),
            },
        ],
        PointIndexLimits::DEFAULT,
    )
    .expect("prepared index");
    let mut observer = RefusingObserver {
        work: 0,
        calls: 0,
        refused_after: 500,
    };
    assert!(matches!(
        index.search_reported_metered(
            &point(0, 0),
            XsdDoubleMetres::new(30_000_000.0).expect("threshold"),
            admitted(),
            PointIndexLimits {
                max_work_items: 100_000_000,
                ..PointIndexLimits::DEFAULT
            },
            &mut observer
        ),
        Err(GeoError::Cancelled)
    ));
    let mut observer = RefusingObserver {
        work: 0,
        calls: 0,
        refused_after: u64::MAX,
    };
    let metered = grid()
        .cover_box_mixed_metered(
            &whole,
            CoverLevels::new(0, 4).expect("levels"),
            admitted(),
            &mut observer,
        )
        .expect("complete observed");
    assert_eq!(
        metered.cells(),
        grid()
            .cover_box_mixed(&whole, CoverLevels::new(0, 4).expect("levels"), admitted())
            .expect("ordinary")
            .cells()
    );
}

/// Street-scale disks complete under the default limits. Every probe point the
/// certified direct problem places inside the disk lies in an emitted cell, at
/// every azimuth and just inside the boundary, where straddling cells decide.
#[test]
fn street_scale_disks_complete_under_default_limits_without_false_negatives() {
    let center = point(116, 40);
    let mut context = MetricContext::wgs84().expect("context");
    let prepared = PreparedGeodesic::prepare(GeographicReference::wgs84(), &mut context)
        .expect("prepared geodesic");
    for (radius, level) in [
        (1_000, 16),
        (30_000, 16),
        (400_000, 16),
        (1_000, 17),
        (30_000, 17),
        (400_000, 17),
    ] {
        let metres = Metres::new(Rat::from_i64(radius));
        let cover = grid()
            .cover_disk_mixed(
                &center,
                &metres,
                CoverLevels::new(0, level).expect("levels"),
                MixedCoverLimits::DEFAULT,
            )
            .unwrap_or_else(|error| panic!("{radius} m at level {level}: {error}"));
        canonical(&cover);
        assert!(cover.cells().len() as u64 <= MixedCoverLimits::DEFAULT.max_emitted_cells);
        assert!(cover.receipt().work_items <= MixedCoverLimits::DEFAULT.max_work_items);
        for azimuth in (0..360).step_by(15) {
            for fraction in ["0.999999", "0.5", "0"] {
                let travel = Metres::new(metres.exact().mul(&decimal(fraction)));
                let probe = prepared
                    .direct(&center, &Rat::from_i64(azimuth), &travel, &mut context)
                    .expect("certified probe");
                let key = grid().assign(probe.endpoint(), 30).expect("leaf");
                assert!(
                    cover.contains(key).expect("profile"),
                    "{radius} m level {level} azimuth {azimuth} fraction {fraction}"
                );
            }
        }
    }
}

/// The emitted-cell cap refuses exactly past the cover's own size, and the
/// cover's size itself is admitted with identical cells.
#[test]
fn the_emitted_cell_cap_refuses_only_past_the_complete_cover() {
    let center = point(116, 40);
    let radius = Metres::new(Rat::from_i64(30_000));
    let levels = CoverLevels::new(0, 16).expect("levels");
    let complete = grid()
        .cover_disk_mixed(&center, &radius, levels, MixedCoverLimits::DEFAULT)
        .expect("default street-scale cover");
    let count = complete.cells().len() as u64;
    let exact = grid()
        .cover_disk_mixed(
            &center,
            &radius,
            levels,
            MixedCoverLimits {
                max_emitted_cells: count,
                ..MixedCoverLimits::DEFAULT
            },
        )
        .expect("the exact count is admitted");
    assert_eq!(exact.cells(), complete.cells());
    assert!(matches!(
        grid().cover_disk_mixed(
            &center,
            &radius,
            levels,
            MixedCoverLimits {
                max_emitted_cells: count - 1,
                ..MixedCoverLimits::DEFAULT
            },
        ),
        Err(GeoError::CoverCellsExhausted { limit }) if limit == count - 1
    ));
}
