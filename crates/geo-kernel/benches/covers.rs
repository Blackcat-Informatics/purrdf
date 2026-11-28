// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Report-only cover costs and complete resource/level receipts.
#![allow(missing_docs)]

use purrdf_geo_kernel::{
    Crs, ExecutionLimits, ExecutionPolicy, GeoProfile, GeographicReference, LonLat, Metres,
    MetricContext, OffsetRegion, PreparedGeometry, Rat,
    cells::{CoverLevels, CubeHilbertQ62V1, MixedCoverLimits, NativeGridProfile},
    ellipsoidal, wkt,
};
use purrdf_testkit::bench::{Bench, bench_group, bench_main, black_box};
use std::{sync::Arc, time::Duration};

#[global_allocator]
static GLOBAL: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;

fn covers(bench: &mut Bench) {
    let grid = CubeHilbertQ62V1::new(NativeGridProfile::Wgs84);
    let center =
        LonLat::new(Rat::from_i64(116), Rat::from_i64(40)).expect("fixed public synthetic point");
    let level17 = grid
        .level_for_max_edge_length(&Metres::new(Rat::from_i64(150)))
        .expect("attainable scale");
    assert_eq!(level17, 17);
    for level in [16, level17] {
        for radius in [1_000, 30_000, 400_000] {
            let radius = Metres::new(Rat::from_i64(radius));
            let levels = CoverLevels::new(0, level).expect("supported");
            let limits = MixedCoverLimits::DEFAULT;
            let cover = grid
                .cover_disk_mixed(&center, &radius, levels, limits)
                .expect("the measured disk must produce a complete cover");
            println!(
                "receipt radius={radius:?} levels={levels:?} emitted={} {:?}",
                cover.cells().len(),
                cover.receipt()
            );
            drop(cover);
            bench.bench_function(
                &format!(
                    "native-covers/radius-{}-level-{level}",
                    radius.exact().numerator()
                ),
                |sample| {
                    sample.iter(|| {
                        black_box(
                            grid.cover_disk_mixed(
                                black_box(&center),
                                black_box(&radius),
                                levels,
                                limits,
                            )
                            .expect("the measured disk must produce a complete cover"),
                        )
                    });
                },
            );
        }
    }
}
fn geometry_metrics(bench: &mut Bench) {
    let profile = GeoProfile::standard();
    let crs = Crs::new(purrdf_iri::vocab::ogc::CRS84).unwrap();
    let literal = wkt::parse("POLYGON((0 0,0.01 0,0.01 0.01,0 0.01,0 0))", &crs).unwrap();
    let geometry = PreparedGeometry::from_literal(&literal, &profile).unwrap();
    let mut context = MetricContext::wgs84().unwrap();
    for (name, metric) in [
        (
            "length",
            ellipsoidal::length as fn(&PreparedGeometry, &mut MetricContext) -> _,
        ),
        ("perimeter", ellipsoidal::perimeter),
        ("area", ellipsoidal::area),
    ] {
        let expected = metric(&geometry, &mut context).unwrap();
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let output = metric(&geometry, &mut context).unwrap();
        let allocations = window.close();
        assert_eq!(output, expected);
        println!(
            "geometry receipt metric={name} work={} workspace_peak={} retained_bytes={} allocations={allocations:?}",
            context.work_items(),
            context.workspace_peak(),
            geometry.retained_workspace_bytes(),
        );
        bench.bench_function(&format!("geographic_polygon/{name}"), |sample| {
            sample.iter(|| black_box(metric(black_box(&geometry), &mut context).unwrap()));
        });
    }
    let point =
        PreparedGeometry::from_literal(&wkt::parse("POINT(0 1)", &crs).unwrap(), &profile).unwrap();
    let line = PreparedGeometry::from_literal(
        &wkt::parse("LINESTRING(-2 0,2 0)", &crs).unwrap(),
        &profile,
    )
    .unwrap();
    let expected_distance = ellipsoidal::distance(&point, &line, &mut context).unwrap();
    assert_eq!(
        expected_distance.exact(),
        &Rat::parse_decimal("110574.388558").unwrap()
    );
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let distance = ellipsoidal::distance(&point, &line, &mut context).unwrap();
    let allocations = window.close();
    assert_eq!(distance, expected_distance);
    println!(
        "geometry receipt metric=distance work={} workspace_peak={} allocations={allocations:?}",
        context.work_items(),
        context.workspace_peak(),
    );
    bench.bench_function("geographic_geometry/point_to_edge_distance", |sample| {
        sample.iter(|| {
            black_box(
                ellipsoidal::distance(black_box(&point), black_box(&line), &mut context).unwrap(),
            )
        });
    });
}

fn geometry_buffer(bench: &mut Bench) {
    let profile = GeoProfile::standard();
    let crs = Crs::new(purrdf_iri::vocab::ogc::CRS84).unwrap();
    // Complete polygon materialization exceeds the ordinary scalar work cap.
    // Report the explicitly admitted cost rather than timing a refusal.
    let policy = ExecutionPolicy::new(ExecutionLimits {
        max_work_items: 20_000_000,
        ..ExecutionLimits::GEOMETRY
    })
    .unwrap();
    let mut context = MetricContext::new(GeographicReference::wgs84(), policy).unwrap();
    let point = wkt::parse("POINT(116 40)", &crs).unwrap();
    let source = Arc::new(PreparedGeometry::from_literal(&point, &profile).unwrap());
    let offset = OffsetRegion::new(source, Metres::new(Rat::from_i64(10)), policy).unwrap();
    let expected = offset.materialize(&profile, &crs, &mut context).unwrap();
    let expected_literal = expected.literal().clone();
    let receipt = expected.output_receipt();
    drop(expected);
    context.release_materialized_output(receipt).unwrap();
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let output = offset.materialize(&profile, &crs, &mut context).unwrap();
    assert_eq!(output.literal(), &expected_literal);
    let receipt = output.output_receipt();
    let retained_output_bytes = receipt.workspace_bytes();
    drop(output);
    context.release_materialized_output(receipt).unwrap();
    let allocations = window.close();
    println!(
        "buffer receipt radius_m=10 work={} workspace_peak={} retained_source_bytes={} retained_output_bytes={retained_output_bytes} work_limit={} allocations={allocations:?}",
        context.work_items(),
        context.workspace_peak(),
        offset.retained_workspace_bytes(),
        policy.limits().max_work_items,
    );
    bench.bench_function("geographic_buffer/point_materialization", |sample| {
        sample.iter(|| {
            let output = black_box(offset.materialize(&profile, &crs, &mut context).unwrap());
            let receipt = output.output_receipt();
            drop(output);
            context.release_materialized_output(receipt).unwrap();
        });
    });
}

bench_group! {name=benches;config=Bench::default().sample_size(10).warm_up_time(Duration::from_millis(10)).measurement_time(Duration::from_millis(100));targets=covers, geometry_metrics, geometry_buffer}
bench_main!(benches);
