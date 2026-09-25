// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`.
#![allow(missing_docs)]

//! Report-only end-to-end measurements for the five-table columnar codec.

use std::sync::Arc;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use purrdf_columnar::plain_bench::PlainInt64;
use purrdf_columnar::test_rng::mix;
use purrdf_columnar::{Compression, read, write};
use purrdf_core::{BlankScope, ContentStore, RdfDataset, RdfDatasetBuilder, RdfLiteral};

const ROWS: u32 = 500;

fn fixture() -> (Arc<RdfDataset>, ContentStore) {
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri("https://example.org/p");
    let graph = builder.intern_iri("https://example.org/g");
    for row in 0..ROWS {
        let subject = builder.intern_iri(&format!("https://example.org/s{row}"));
        let blank = builder.intern_blank(&format!("b{row}"), BlankScope(row % 8));
        let literal = builder.intern_literal(RdfLiteral::typed(
            row.to_string(),
            "http://www.w3.org/2001/XMLSchema#integer",
        ));
        builder.push_quad(subject, predicate, blank, None);
        builder.push_quad(blank, predicate, literal, Some(graph));
    }
    let mut blobs = ContentStore::new();
    for row in 0..16 {
        blobs.insert(format!("payload-{row:04}").into_bytes());
    }
    (
        builder.freeze().expect("benchmark fixture is valid RDF"),
        blobs,
    )
}

fn bench_codec(c: &mut Criterion) {
    let (dataset, blobs) = fixture();
    let encoded = write(&*dataset, &blobs, Compression::Zstd)
        .expect("benchmark fixture encodes as ZSTD Parquet");
    let mut group = c.benchmark_group("columnar_codec_1000_quads");
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
    group.finish();
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
fn bench_plain_int64(c: &mut Criterion) {
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

criterion_group!(benches, bench_codec, bench_plain_int64);
criterion_main!(benches);
