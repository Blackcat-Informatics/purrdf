// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Complete public freeze costs after terms are interned and rows are appended.
//! Set `PURRDF_FREEZE_BENCH_ROWS` to scale the default 100,000 unique statements.
//! At 11,700,000 rows the named-graph fixture has 1,870,000 distinct terms and
//! 117 graphs. Half its object terms are literals; the rest are IRIs.
//! Setup, validation for the materialization case, and output destruction run
//! outside timing. Results are report-only; no speed threshold is asserted.

use std::time::Duration;

#[path = "support/freeze_fixture.rs"]
mod freeze_fixture;
use freeze_fixture::{fixture, row_count};

use purrdf_testkit::bench::{BatchSize, Bench, BenchmarkId, Throughput, bench_group, bench_main};

fn bench_freeze(bench: &mut Bench) {
    let rows = row_count();
    let mut group = bench.benchmark_group("native_freeze");
    group
        .sample_size(10)
        .warm_up_time(Duration::from_millis(100))
        .measurement_time(Duration::from_secs(1))
        .throughput(Throughput::Elements(rows as u64));
    for (name, graphs) in [("default", 0), ("named", 117)] {
        group.bench_function(
            BenchmarkId::new(format!("freeze_{name}"), rows),
            |bencher| {
                bencher.iter_batched(
                    || fixture(rows, graphs),
                    |builder| builder.freeze().expect("fixture freezes"),
                    BatchSize::LargeInput,
                );
            },
        );
        group.bench_function(
            BenchmarkId::new(format!("materialize_{name}"), rows),
            |bencher| {
                bencher.iter_batched(
                    || {
                        let builder = fixture(rows, graphs).validate().expect("fixture validates");
                        assert_eq!(builder.statement_count(), rows);
                        assert_eq!(builder.term_count(), rows * 187 / 1170);
                        builder
                    },
                    |builder| builder.freeze().expect("validated fixture materializes"),
                    BatchSize::LargeInput,
                );
            },
        );
    }
    group.finish();
}

bench_group!(benches, bench_freeze);
bench_main!(benches);
