// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Report-only plain/prepared and caller-buffer shortest-geodesic costs.
#![allow(missing_docs)]

use purrdf_geo_kernel::{
    ExecutionLimits, ExecutionPolicy, GeographicReference, LonLat, MetricContext, PreparedGeodesic,
    Rat, geodesic,
};
use purrdf_hash::Backend as _;
use purrdf_testkit::bench::{Bench, Throughput, bench_group, bench_main, black_box};
use purrdf_xsd::math::FloatProductBackend;
use std::time::Duration;

fn pair() -> (LonLat, LonLat) {
    let point = |longitude, latitude| {
        LonLat::new(
            Rat::parse_decimal(longitude).unwrap(),
            Rat::parse_decimal(latitude).unwrap(),
        )
        .unwrap()
    };
    // Independently published CC0 GeodTest row; this is a common fast path.
    (
        point("0", "36.530042355041"),
        point("5.762344694676510456", "-48.164270779097768864"),
    )
}

fn benchmark(c: &mut Bench) {
    let mut group = c.benchmark_group("ellipsoidal_point");
    group.sample_size(10);
    group.warm_up_time(Duration::from_millis(20));
    group.measurement_time(Duration::from_millis(100));
    group.bench_function("context_construct", |b| {
        b.iter(|| black_box(MetricContext::wgs84().unwrap()));
    });
    let mut context = MetricContext::wgs84().unwrap();
    group.bench_function("context_validate", |b| {
        b.iter(|| {
            context.checkpoint().unwrap();
            black_box(());
        });
    });
    let (a, b) = pair();
    group.bench_function("plain", |bench| {
        bench.iter(|| black_box(geodesic::distance(a.clone(), b.clone()).unwrap()));
    });
    let reference = GeographicReference::wgs84();
    group.bench_function("coefficient_prepare", |bench| {
        bench.iter(|| {
            black_box(PreparedGeodesic::prepare(reference.clone(), &mut context).unwrap())
        });
    });
    let prepared = PreparedGeodesic::prepare(reference.clone(), &mut context).unwrap();
    context.prepare_arithmetic().unwrap();
    group.bench_function("prepared", |bench| {
        bench.iter(|| black_box(prepared.distance(&a, &b, &mut context).unwrap()));
    });
    group.bench_function("inverse_metadata", |bench| {
        bench.iter(|| black_box(prepared.inverse(&a, &b, &mut context).unwrap()));
    });
    let inverse = prepared.inverse(&a, &b, &mut context).unwrap();
    let azimuth = inverse.forward_azimuth().unwrap();
    let travel = inverse.distance().value().clone();
    group.bench_function("direct", |bench| {
        bench.iter(|| black_box(prepared.direct(&a, azimuth, &travel, &mut context).unwrap()));
    });
    group.bench_function("point_prepare", |bench| {
        bench.iter(|| black_box(prepared.prepare_point(&a, &mut context).unwrap()));
    });
    let point = prepared.prepare_point(&a, &mut context).unwrap();
    group.bench_function("prepared_point", |bench| {
        bench.iter(|| {
            black_box(
                prepared
                    .distance_from_prepared(&point, &b, &mut context)
                    .unwrap(),
            )
        });
    });
    let policy = ExecutionPolicy::new(ExecutionLimits {
        max_work_items: u64::MAX,
        ..ExecutionLimits::GEOMETRY
    })
    .unwrap();
    for count in [1usize, 4, 16, 256, 4096] {
        let mut output = vec![None; count];
        let mut inverses = vec![None; count];
        let mut direct = vec![None; count];
        let mut context = MetricContext::new(reference.clone(), policy).unwrap();
        context.prepare_arithmetic().unwrap();
        group.throughput(Throughput::Elements(count as u64));
        group.bench_function(format!("batch{count}"), |bench| {
            bench.iter(|| {
                prepared
                    .distance_batch_borrowed(
                        std::iter::repeat_n((&a, &b), count),
                        black_box(&mut output),
                        black_box(&mut context),
                    )
                    .unwrap();
                black_box(&output);
            });
        });
        group.bench_function(format!("inverse_batch{count}"), |bench| {
            bench.iter(|| {
                prepared
                    .inverse_batch_borrowed(
                        std::iter::repeat_n((&a, &b), count),
                        black_box(&mut inverses),
                        black_box(&mut context),
                    )
                    .unwrap();
                black_box(&inverses);
            });
        });
        group.bench_function(format!("direct_batch{count}"), |bench| {
            bench.iter(|| {
                prepared
                    .direct_batch_borrowed(
                        std::iter::repeat_n((&a, azimuth, &travel), count),
                        black_box(&mut direct),
                        geodesic::DirectOutputGrid::DEGREE15,
                        black_box(&mut context),
                    )
                    .unwrap();
                black_box(&direct);
            });
        });
    }
    group.finish();
}

fn backend_paths(c: &mut Bench) {
    let mut group = c.benchmark_group("ellipsoidal_backend");
    group.sample_size(20);
    group.warm_up_time(Duration::from_millis(100));
    group.measurement_time(Duration::from_millis(500));
    let (a, b) = pair();
    let mut builder = MetricContext::wgs84().unwrap();
    builder.prepare_arithmetic().unwrap();
    let prepared = PreparedGeodesic::prepare(builder.reference().clone(), &mut builder).unwrap();
    let source = prepared.prepare_point(&a, &mut builder).unwrap();
    for path in FloatProductBackend::all_available() {
        let mut worker = MetricContext::wgs84().unwrap();
        worker.set_binary64_backend(path).unwrap();
        worker.set_prepared_arithmetic(builder.prepared_arithmetic().unwrap());
        group.bench_function(format!("{}/prepared", path.name()), |bench| {
            bench.iter(|| black_box(prepared.distance(&a, &b, &mut worker).unwrap()));
        });
        group.bench_function(format!("{}/prepared_point", path.name()), |bench| {
            bench.iter(|| {
                black_box(
                    prepared
                        .distance_from_prepared(&source, &b, &mut worker)
                        .unwrap(),
                )
            });
        });
    }
    group.finish();
}

bench_group!(benches, benchmark, backend_paths);
bench_main!(benches);
