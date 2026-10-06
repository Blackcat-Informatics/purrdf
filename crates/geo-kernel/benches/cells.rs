// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Report-only integer grid assignment and caller-buffer allocation evidence.
#![allow(missing_docs)]

use purrdf_geo_kernel::{
    LonLat, Rat,
    cells::{CellId, CubeHilbertQ62V1, NativeGridProfile},
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
bench_group!(benches, cells);
bench_main!(benches);
