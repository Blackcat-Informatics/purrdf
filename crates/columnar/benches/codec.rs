// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`.
#![allow(missing_docs)]

//! Report-only end-to-end measurements for the five-table columnar codec.

use std::sync::Arc;

use criterion::{Criterion, criterion_group, criterion_main};
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

criterion_group!(benches, bench_codec);
criterion_main!(benches);
