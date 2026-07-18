// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Standard-allocator Criterion instrument for deterministic Pydantic emission.

use std::time::Duration;

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_main};

#[path = "support/pydantic.rs"]
mod pydantic_support;
use pydantic_support::{Fixture, Mode, SIZES};

fn bench_pydantic_package(c: &mut Criterion) {
    let fixtures = SIZES
        .into_iter()
        .flat_map(|definitions| {
            Mode::ALL
                .into_iter()
                .map(move |mode| Fixture::new(definitions, mode))
        })
        .collect::<Vec<_>>();
    let mut group = c.benchmark_group("pydantic_package_emission");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(3));
    group.warm_up_time(Duration::from_secs(1));

    for fixture in &fixtures {
        black_box(fixture.emit());
        group.throughput(Throughput::Elements(
            fixture
                .definitions
                .try_into()
                .expect("definition count fits u64"),
        ));
        group.bench_with_input(
            BenchmarkId::new(fixture.mode.label(), fixture.definitions),
            fixture,
            |bencher, fixture| {
                bencher.iter(|| black_box(fixture.emit()));
            },
        );
    }
    group.finish();
}

/// Run the Pydantic package benchmark group.
pub fn benches() {
    let mut criterion = Criterion::default().configure_from_args();
    bench_pydantic_package(&mut criterion);
}

criterion_main!(benches);
