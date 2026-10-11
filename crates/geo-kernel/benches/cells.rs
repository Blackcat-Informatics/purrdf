// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Report-only integer grid assignment and caller-buffer allocation evidence.
#![allow(missing_docs)]

use purrdf_geo_kernel::{
    GeographicReference, LonLat, Metres, Rat,
    cells::{
        CellId, CoverLevels, CubeHilbertQ62V1, MixedCoverLimits, NativeGridProfile, PointCellIndex,
        PointIndexLimits, PointIndexPoint,
    },
};
use purrdf_testkit::bench::{Bench, Throughput, bench_group, bench_main, black_box};
use std::time::Duration;

#[global_allocator]
static GLOBAL: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;

fn cells(bench: &mut Bench) {
    let grid = CubeHilbertQ62V1::new(NativeGridProfile::Wgs84);
    let point = LonLat::new(
        Rat::parse_decimal("116.391").unwrap(),
        Rat::parse_decimal("39.9075").unwrap(),
    )
    .unwrap();
    let root = CellId::root(grid.profile_id(), 0).unwrap();
    let mut group = bench.benchmark_group("native_grid");
    group.sample_size(10);
    group.warm_up_time(Duration::from_millis(20));
    group.measurement_time(Duration::from_millis(100));
    for count in [1_usize, 4, 16, 256, 4096] {
        let points = vec![point.clone(); count];
        let mut output = vec![root; count];
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        grid.assign_batch(&points, 30, &mut output).unwrap();
        let allocations = window.close();
        println!("allocation receipt count={count} {allocations:?}");
        group.throughput(Throughput::Elements(count as u64));
        group.bench_function(format!("assignment_batch{count}"), |sample| {
            sample.iter(|| {
                grid.assign_batch(black_box(&points), 30, black_box(&mut output))
                    .unwrap();
                black_box(&output);
            });
        });
    }
    group.finish();
}
fn point_index(bench: &mut Bench) {
    let grid = CubeHilbertQ62V1::new(NativeGridProfile::Wgs84);
    let center = LonLat::new(Rat::from_i64(116), Rat::from_i64(40)).unwrap();
    let radius = Metres::new(Rat::from_i64(500));
    let step = Rat::parse_decimal("0.001").unwrap();
    let points: Vec<_> = (-8_i64..=8)
        .flat_map(|x| (-8_i64..=8).map(move |y| (x, y)))
        .enumerate()
        .map(|(key, (x, y))| PointIndexPoint {
            key: key as u64,
            point: LonLat::new(
                Rat::from_i64(116).add(&step.mul(&Rat::from_i64(x))),
                Rat::from_i64(40).add(&step.mul(&Rat::from_i64(y))),
            )
            .unwrap(),
        })
        .collect();
    // Measure the complete declared workload, including every refinement.
    let limits = PointIndexLimits {
        max_work_items: 20_000_000,
        ..PointIndexLimits::DEFAULT
    };
    let mut group = bench.benchmark_group("native_point_index");
    group.sample_size(10);
    group.warm_up_time(Duration::from_millis(20));
    group.measurement_time(Duration::from_millis(100));
    let mut expected = None;
    for level in [12, 17] {
        let index = PointCellIndex::new(
            grid,
            level,
            GeographicReference::wgs84(),
            None,
            points.clone(),
            limits,
        )
        .unwrap();
        // Count the exact candidate set admitted by the same mixed cover used
        // by search_physical. Refinement and all result destruction stay timed.
        let cover = grid
            .cover_disk_mixed(
                &center,
                &radius,
                CoverLevels::new(0, level).unwrap(),
                MixedCoverLimits::DEFAULT,
            )
            .unwrap();
        let candidates = points
            .iter()
            .filter(|entry| {
                cover
                    .contains(grid.assign(&entry.point, level).unwrap())
                    .unwrap()
            })
            .count();
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let matches = index
            .search_physical(&center, &radius, MixedCoverLimits::DEFAULT, limits)
            .unwrap();
        let allocations = window.close();
        assert!(!matches.is_empty());
        assert!(candidates >= matches.len());
        if let Some(keys) = &expected {
            assert_eq!(matches.keys(), keys);
        } else {
            expected = Some(matches.keys().to_vec());
        }
        println!(
            "index receipt level={level} points={} candidates={candidates} accepted={} index_bytes={} result_bytes={} work_limit={} allocations={allocations:?}",
            points.len(),
            matches.len(),
            index.retained_workspace_bytes(),
            matches.retained_workspace_bytes(),
            limits.max_work_items,
        );
        drop(matches);
        drop(cover);
        group.bench_function(format!("physical_level{level}"), |sample| {
            sample.iter(|| {
                black_box(
                    index
                        .search_physical(
                            black_box(&center),
                            black_box(&radius),
                            MixedCoverLimits::DEFAULT,
                            limits,
                        )
                        .unwrap(),
                );
            });
        });
    }
    group.finish();
}

bench_group!(benches, cells, point_index);
bench_main!(benches);
