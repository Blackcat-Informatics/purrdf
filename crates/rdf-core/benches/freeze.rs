// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Complete public freeze costs after terms are interned and rows are appended.
//! Set `PURRDF_FREEZE_BENCH_ROWS` to scale the default 100,000 unique statements.
//! At 11,700,000 rows the named-graph fixture has 1,870,000 distinct terms and
//! 117 graphs. Half its object terms are literals; the rest are IRIs.
//! Setup, validation for the materialization case, and output destruction run
//! outside timing. Results are report-only; no speed threshold is asserted.

use std::time::Duration;

use purrdf_core::{RdfDatasetBuilder, RdfLiteral};
use purrdf_testkit::bench::{BatchSize, Bench, BenchmarkId, Throughput, bench_group, bench_main};

fn row_count() -> usize {
    let rows = match std::env::var("PURRDF_FREEZE_BENCH_ROWS") {
        Ok(value) => value
            .parse()
            .expect("benchmark row count must be an integer"),
        Err(std::env::VarError::NotPresent) => 100_000,
        Err(error) => panic!("cannot read benchmark row count: {error}"),
    };
    assert!(rows >= 10_000, "benchmark needs at least 10,000 rows");
    rows
}

fn fixture(rows: usize, graph_count: usize) -> RdfDatasetBuilder {
    const PREDICATES: usize = 10;
    let subjects = rows.div_ceil(PREDICATES);
    let terms = rows
        .checked_mul(187)
        .expect("fixture term count fits usize")
        / 1170;
    let objects = terms
        .checked_sub(subjects + PREDICATES + graph_count + 1)
        .filter(|&count| count != 0)
        .expect("fixture needs distinct object terms");
    let mut builder = RdfDatasetBuilder::new();
    let predicates: Vec<_> = (0..PREDICATES)
        .map(|index| {
            builder
                .intern_iri(&format!("https://example.org/freeze/p/{index}"))
                .expect("fixture predicate is valid")
        })
        .collect();
    let graphs: Vec<_> = (0..graph_count)
        .map(|index| {
            builder
                .intern_iri(&format!("https://example.org/freeze/g/{index}"))
                .expect("fixture graph is valid")
        })
        .collect();
    let subjects: Vec<_> = (0..subjects)
        .map(|index| {
            builder
                .intern_iri(&format!("https://example.org/freeze/s/{index}"))
                .expect("fixture subject is valid")
        })
        .collect();
    let objects: Vec<_> = (0..objects)
        .map(|index| {
            if index % 2 == 0 {
                builder
                    .intern_literal(RdfLiteral::simple(format!("source value {index}")))
                    .expect("fixture literal is valid")
            } else {
                builder
                    .intern_iri(&format!("https://example.org/freeze/o/{index}"))
                    .expect("fixture object is valid")
            }
        })
        .collect();
    for index in 0..rows {
        let graph = if graphs.is_empty() {
            None
        } else {
            Some(graphs[index % graphs.len()])
        };
        // Every subject/predicate pair is unique, so validation cannot collapse
        // the requested population through statement deduplication.
        builder
            .push_quad(
                subjects[index / PREDICATES],
                predicates[index % PREDICATES],
                objects[index % objects.len()],
                graph,
            )
            .expect("fixture statement is valid");
    }
    builder
}

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
