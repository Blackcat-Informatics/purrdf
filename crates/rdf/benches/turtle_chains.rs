// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Actual structural rendering at shallow, 100,000 and 1,000,000 blank levels.

#[path = "../tests/support/turtle_chains.rs"]
mod inputs;

use purrdf_testkit::bench::{Bench, BenchmarkId, Throughput, bench_group, bench_main};
use std::{hint::black_box, time::Duration};

fn render(bench: &mut Bench) {
    let mut group = bench.benchmark_group("turtle_chain");
    for depth in [8, 100_000, 1_000_000] {
        let dataset = inputs::chain(depth, false, false);
        group.throughput(Throughput::Elements(depth as u64));
        group.bench_function(BenchmarkId::from_parameter(depth), |sample| {
            sample.iter(|| {
                black_box(purrdf_core::render_canonical_turtle(
                    black_box(&dataset),
                    &[],
                ))
            });
        });
    }
    group.finish();
}

bench_group! {
    name = turtle_chains;
    config = Bench::default().sample_size(10)
        .warm_up_time(Duration::from_millis(100))
        .measurement_time(Duration::from_secs(1));
    targets = render
}
bench_main!(turtle_chains);
