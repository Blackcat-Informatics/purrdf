// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API, so the workspace `missing_docs` lint is
// not asked of their items.
#![allow(missing_docs)]

//! Report-only end-to-end measurements for the five-table columnar codec.

use purrdf_columnar::plain_bench::PlainInt64;
use purrdf_columnar::{Compression, WriteOptions, project, read, write, write_to};
use purrdf_testkit::bench::{BatchSize, Bench, BenchmarkId, Throughput, bench_group, bench_main};
use purrdf_testkit::rng::splitmix64_next as mix;

mod common;

fn bench_codec(c: &mut Bench) {
    for workload in [
        common::Workload::Mixed,
        common::Workload::Reifiers,
        common::Workload::LargeBlobs,
    ] {
        let (dataset, blobs) = common::fixture(workload);
        let encoded = write(&*dataset, &blobs, Compression::Zstd)
            .expect("benchmark fixture encodes as ZSTD Parquet");
        let projected = project(&*dataset, &blobs).expect("fixture projects");
        let mut group = c.benchmark_group(format!("columnar_codec_{workload:?}"));
        group.throughput(Throughput::Elements(u64::from(common::ROWS) * 3));
        group.bench_function("project", |bencher| {
            bencher.iter(|| {
                std::hint::black_box(project(&*dataset, &blobs).expect("fixture projects"))
            });
        });
        group.bench_function("write_uncompressed", |bencher| {
            bencher.iter(|| {
                std::hint::black_box(
                    write(&*dataset, &blobs, Compression::Uncompressed)
                        .expect("benchmark fixture encodes"),
                )
            });
        });
        group.bench_function("write_zstd", |bencher| {
            bencher.iter(|| {
                std::hint::black_box(
                    write(&*dataset, &blobs, Compression::Zstd).expect("benchmark fixture encodes"),
                )
            });
        });
        group.bench_function("read_zstd", |bencher| {
            bencher.iter(|| {
                std::hint::black_box(read(&encoded.files).expect("benchmark fixture decodes"))
            });
        });
        group.bench_function("projected_zstd", |bencher| {
            bencher.iter(|| {
                std::hint::black_box(
                    projected
                        .to_parquet(Compression::Zstd)
                        .expect("projected tables encode"),
                )
            });
        });
        group.bench_function("sink_zstd", |bencher| {
            let options = WriteOptions {
                compression: Compression::Zstd,
                ..WriteOptions::default()
            };
            bencher.iter(|| {
                std::hint::black_box(
                    write_to(&*dataset, &blobs, &options, |_, bytes| {
                        std::hint::black_box(bytes);
                        Ok::<_, core::convert::Infallible>(())
                    })
                    .expect("fixture writes to sink"),
                )
            });
        });
        group.finish();
    }
}

/// Rows per PLAIN `INT64` column: many chunk multiples, and past any small-size
/// special case.
const PLAIN_ROWS: usize = 65_536;

/// A column with `nulls_per_ten` rows in ten null, at seeded positions.
fn plain_column(nulls_per_ten: u64) -> PlainInt64 {
    let mut state = 0xC0DE_u64 ^ nulls_per_ten;
    PlainInt64::from_rows((0..PLAIN_ROWS).map(|_| {
        let null = mix(&mut state) % 10 < nulls_per_ten;
        let value = mix(&mut state).cast_signed();
        (!null).then_some(value)
    }))
}

/// The PLAIN `INT64` value codec on its own (the `columnar.plain-codec` site):
/// encode is the copy of the dense present values into the body, decode the
/// length check and the copy back, each at 0%, 10% and 90% nulls. The decode's
/// definition levels are cloned outside the timed region.
fn bench_plain_int64(c: &mut Bench) {
    let mut group = c.benchmark_group("columnar_plain_int64_65536_rows");
    group.throughput(Throughput::Elements(PLAIN_ROWS as u64));
    for (label, nulls_per_ten) in [("nulls_0pct", 0), ("nulls_10pct", 1), ("nulls_90pct", 9)] {
        let column = plain_column(nulls_per_ten);
        let body = column.encode();
        let decoded = PlainInt64::decode(&body, column.presence()).expect("the body decodes");
        assert_eq!(decoded, column, "the bench column round-trips");
        group.bench_with_input(
            BenchmarkId::new("encode", label),
            &column,
            |bencher, column| {
                bencher.iter(|| std::hint::black_box(column.encode()));
            },
        );
        group.bench_with_input(BenchmarkId::new("decode", label), &body, |bencher, body| {
            bencher.iter_batched(
                || column.presence(),
                |presence| {
                    std::hint::black_box(
                        PlainInt64::decode(body, presence).expect("the body decodes"),
                    )
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

bench_group!(benches, bench_codec, bench_plain_int64);
bench_main!(benches);
