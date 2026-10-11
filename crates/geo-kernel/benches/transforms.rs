// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Report-only offline transformation, context and caller-buffer allocation costs.
#![allow(missing_docs)]

use std::time::Duration;

use purrdf_geo_kernel::{
    ExecutionLimits, ExecutionPolicy, GeographicReference, MetricContext, PreparedEllipsoid, Rat,
    operation::{
        Applicability, CoordinateOperation, CoordinateUnit, Hemisphere, OperationModel,
        OperationPoint, OperationReference, RotationConvention, Similarity2d, TransverseMercator,
        ZoneFamily,
    },
};
use purrdf_hash::hex::Digest32;
use purrdf_testkit::bench::{Bench, Throughput, bench_group, bench_main, black_box};

#[global_allocator]
static GLOBAL: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;

fn reference(identity: u8, unit: CoordinateUnit) -> OperationReference {
    OperationReference {
        realization: Digest32::new([identity; 32]),
        unit,
        swapped_axes: false,
    }
}

fn transforms(bench: &mut Bench) {
    let source = OperationPoint {
        x: Rat::from_i64(120),
        y: Rat::from_i64(30),
        z: Some(Rat::from_i64(10)),
        epoch: None,
    };
    let domain = Applicability::new(
        Rat::from_i64(110),
        Rat::from_i64(130),
        Rat::from_i64(20),
        Rat::from_i64(40),
    )
    .unwrap();
    let cases = [
        (
            "gcj",
            CoordinateUnit::Degrees,
            CoordinateUnit::Degrees,
            OperationModel::GcjRationalHarmonicV1(domain),
        ),
        (
            "bd09ll",
            CoordinateUnit::Degrees,
            CoordinateUnit::Degrees,
            OperationModel::Bd09LlV1,
        ),
        (
            "analytic_mercator",
            CoordinateUnit::Degrees,
            CoordinateUnit::Metres,
            OperationModel::BaiduMercatorAnalyticV1,
        ),
        (
            "cgcs2000_geocentric",
            CoordinateUnit::Degrees,
            CoordinateUnit::Metres,
            OperationModel::GeographicToGeocentric {
                ellipsoid: PreparedEllipsoid::cgcs2000(),
            },
        ),
        (
            "similarity",
            CoordinateUnit::Metres,
            CoordinateUnit::Metres,
            OperationModel::Similarity2d(Similarity2d {
                translation: [Rat::from_i64(1), Rat::from_i64(2)],
                scale: Rat::one(),
                rotation_degrees: Rat::from_i64(30),
                convention: RotationConvention::PositionVector,
                inverse: false,
            }),
        ),
    ];
    // Batches share one deliberately raised work policy; measurements report
    // actual work/peak instead of turning exhausted default limits into timings.
    let policy = ExecutionPolicy::new(ExecutionLimits {
        max_work_items: u64::MAX,
        ..ExecutionLimits::GEOMETRY
    })
    .unwrap();
    let mut cases: Vec<_> = cases
        .into_iter()
        .map(|(name, source_unit, target_unit, model)| {
            (name, source_unit, target_unit, model, source.clone())
        })
        .collect();
    let projection_source = OperationPoint {
        x: Rat::parse_decimal("120.75").unwrap(),
        y: Rat::from_i64(30),
        z: Some(Rat::from_i64(10)),
        epoch: None,
    };
    for (forward_name, inverse_name, parameters) in [
        (
            "cgcs2000_gk3",
            "cgcs2000_gk3_inverse",
            TransverseMercator {
                ellipsoid: PreparedEllipsoid::cgcs2000(),
                family: ZoneFamily::GaussKruger3,
                zone: 40,
                central_meridian: Rat::from_i64(120),
                scale: Rat::one(),
                false_easting: Rat::from_i64(500_000),
                false_northing: Rat::zero(),
                hemisphere: Hemisphere::North,
                zone_prefix: false,
            },
        ),
        (
            "cgcs2000_utm",
            "cgcs2000_utm_inverse",
            TransverseMercator {
                ellipsoid: PreparedEllipsoid::cgcs2000(),
                family: ZoneFamily::Utm,
                zone: 51,
                central_meridian: Rat::from_i64(123),
                scale: Rat::parse_decimal("0.9996").unwrap(),
                false_easting: Rat::from_i64(500_000),
                false_northing: Rat::zero(),
                hemisphere: Hemisphere::North,
                zone_prefix: false,
            },
        ),
    ] {
        let operation = CoordinateOperation::compile(
            reference(1, CoordinateUnit::Degrees),
            reference(2, CoordinateUnit::Metres),
            OperationModel::TransverseMercator(Box::new(parameters.clone())),
        )
        .unwrap();
        let mut preparation = MetricContext::new(GeographicReference::cgcs2000(), policy).unwrap();
        let projected = operation
            .apply(&projection_source, &mut preparation)
            .unwrap()
            .into_point();
        cases.push((
            forward_name,
            CoordinateUnit::Degrees,
            CoordinateUnit::Metres,
            OperationModel::TransverseMercator(Box::new(parameters.clone())),
            projection_source.clone(),
        ));
        cases.push((
            inverse_name,
            CoordinateUnit::Metres,
            CoordinateUnit::Degrees,
            OperationModel::TransverseMercatorInverse(Box::new(parameters)),
            projected,
        ));
    }
    let mut group = bench.benchmark_group("offline_transform");
    group.sample_size(10);
    group.warm_up_time(Duration::from_millis(20));
    group.measurement_time(Duration::from_millis(100));
    for (name, source_unit, target_unit, model, source) in cases {
        let operation = CoordinateOperation::compile(
            reference(1, source_unit),
            reference(2, target_unit),
            model,
        )
        .unwrap();
        group.throughput(Throughput::Elements(1));
        group.bench_function(format!("{name}/fresh_context"), |sample| {
            sample.iter(|| {
                let mut context = MetricContext::new(GeographicReference::wgs84(), policy).unwrap();
                black_box(operation.apply(black_box(&source), &mut context).unwrap());
            });
        });
        let mut context = MetricContext::new(GeographicReference::wgs84(), policy).unwrap();
        operation.apply(&source, &mut context).unwrap();
        group.bench_function(format!("{name}/warmed_context"), |sample| {
            sample.iter(|| {
                black_box(operation.apply(black_box(&source), &mut context).unwrap());
            });
        });
        for count in [1_usize, 4, 16, 256, 4096] {
            let points = vec![source.clone(); count];
            let mut output = vec![None; count];
            operation
                .apply_batch(&points, &mut output, &mut context)
                .unwrap();
            let window = purrdf_alloc_probe::CurrentThreadWindow::open();
            operation
                .apply_batch(&points, &mut output, &mut context)
                .unwrap();
            let allocations = window.close();
            println!(
                "transform receipt model={name} count={count} work={} workspace_peak={} {allocations:?}",
                context.work_items(),
                context.workspace_peak(),
            );
            group.throughput(Throughput::Elements(count as u64));
            group.bench_function(format!("{name}/batch{count}"), |sample| {
                sample.iter(|| {
                    operation
                        .apply_batch(black_box(&points), black_box(&mut output), &mut context)
                        .unwrap();
                    black_box(&output);
                });
            });
        }
    }
    group.finish();
}

bench_group!(benches, transforms);
bench_main!(benches);
