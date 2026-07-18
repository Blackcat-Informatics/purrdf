// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`.
#![allow(missing_docs)]

//! Timed Criterion benchmarks for the PURREMB companion format.
//!
//! This process intentionally uses Rust's normal global allocator. Allocation
//! traffic is measured by the separate `purremb_alloc` process so atomic
//! accounting cannot distort these timing samples.

use std::io::Cursor;
use std::time::Duration;

use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use purrdf_core::{
    EmbeddingStreamWriter, EmbeddingView, ResidentEmbeddingCertificate, reopen_prevalidated,
    verify_embedding,
};

#[path = "support/purremb.rs"]
mod fixture;

use fixture::{
    COARSE_DIMENSION, CatalogFixture, F32_DIMENSION, F32_ROWS, F32Fixture, F64_ROWS, F64Fixture,
    FULL_DIMENSION, RECALL_K, RERANK_CANDIDATES, build_catalog_fixture, build_f32_fixture,
    build_f64_fixture, effective_row, rerank, top_k, usize_to_u64,
};

fn certificate(bytes: &[u8]) -> ResidentEmbeddingCertificate<'_> {
    let mut view = EmbeddingView::from_bytes(bytes).expect("certificate view");
    verify_embedding(&mut view)
        .expect("certificate verification")
        .into_certificate()
}

fn bench_f32(c: &mut Criterion, fixture: &F32Fixture) {
    let bytes = usize_to_u64(fixture.bytes.len());
    let certificate = certificate(&fixture.bytes);
    {
        let mut open = c.benchmark_group("purremb_f32_open");
        open.sample_size(10);
        open.warm_up_time(Duration::from_secs(1));
        open.measurement_time(Duration::from_secs(5));
        open.throughput(Throughput::Bytes(bytes));
        open.bench_function("full_validation", |benchmark| {
            benchmark.iter(|| {
                let mut view = EmbeddingView::from_bytes(std::hint::black_box(&fixture.bytes))
                    .expect("structural benchmark view");
                std::hint::black_box(
                    verify_embedding(&mut view).expect("full benchmark verification"),
                );
            });
        });
        open.bench_function("resident_prevalidated_reopen", |benchmark| {
            benchmark.iter(|| {
                std::hint::black_box(
                    reopen_prevalidated(std::hint::black_box(&fixture.bytes), &certificate)
                        .expect("resident reopen"),
                );
            });
        });
        open.finish();
    }

    let view = reopen_prevalidated(&fixture.bytes, &certificate).expect("hot view");
    let target_set = view
        .target_set(fixture.target_set.id)
        .expect("benchmark target set");
    let raw = view
        .effective_matrix(fixture.target_set.id, fixture.raw_space)
        .expect("raw lookup")
        .expect("raw matrix");
    let coarse = view
        .effective_matrix(fixture.target_set.id, fixture.coarse_space)
        .expect("coarse lookup")
        .expect("coarse matrix");
    let full = view
        .effective_matrix(fixture.target_set.id, fixture.full_space)
        .expect("full lookup")
        .expect("full matrix");
    let query_coarse = effective_row(coarse, 0);
    let query_full = effective_row(full, 0);

    let full_truth = top_k(full, &query_full, RECALL_K, 0);
    let coarse_candidates = top_k(coarse, &query_coarse, RERANK_CANDIDATES, 0);
    let reranked = rerank(full, &query_full, &coarse_candidates, RECALL_K);
    assert_eq!(
        full_truth.len(),
        RECALL_K,
        "exact scan must return recall@k"
    );
    assert_eq!(reranked.len(), RECALL_K, "rerank must return recall@k");
    let recovered = full_truth
        .iter()
        .filter(|truth| reranked.iter().any(|candidate| candidate.row == truth.row))
        .count();
    let recall = f64::from(u32::try_from(recovered).expect("small recall count"))
        / f64::from(u32::try_from(RECALL_K).expect("small recall denominator"));
    println!(
        "[purremb] coarse_prefix={COARSE_DIMENSION} candidates={RERANK_CANDIDATES} rerank_prefix={FULL_DIMENSION} recall@{RECALL_K}={recall:.3}"
    );

    {
        let mut access = c.benchmark_group("purremb_f32_access");
        access.bench_function("target_by_row", |benchmark| {
            let mut row = 0usize;
            benchmark.iter(|| {
                let target = target_set.target(row).expect("target row");
                row = (row + 1) % F32_ROWS;
                std::hint::black_box(target);
            });
        });
        access.bench_function("target_by_id", |benchmark| {
            let target = target_set.target(F32_ROWS / 2).expect("middle target");
            benchmark.iter(|| {
                std::hint::black_box(
                    view.target(std::hint::black_box(target))
                        .expect("target lookup"),
                )
            });
        });
        access.bench_function("raw_prefix_32", |benchmark| {
            let mut row = 0u64;
            benchmark.iter(|| {
                std::hint::black_box(raw.raw_prefix_bytes(row).expect("raw prefix"));
                row = (row + 1) % u64::try_from(F32_ROWS).expect("row count");
            });
        });
        access.bench_function("deterministic_l2_prefix_64", |benchmark| {
            let mut row = 0u64;
            benchmark.iter(|| {
                let sum = coarse
                    .f32_row(row)
                    .expect("coarse row")
                    .map(|value| value.expect("finite coordinate"))
                    .sum::<f32>();
                row = (row + 1) % u64::try_from(F32_ROWS).expect("row count");
                std::hint::black_box(sum);
            });
        });
        access.bench_function("native_full_row", |benchmark| {
            let matrix = full.matrix();
            let mut row = 0u64;
            benchmark.iter(|| {
                std::hint::black_box(matrix.native_f32_row(row).expect("native f32 row"));
                row = (row + 1) % u64::try_from(F32_ROWS).expect("row count");
            });
        });
        access.finish();
    }

    {
        let mut search = c.benchmark_group("purremb_f32_search");
        search.sample_size(10);
        search.warm_up_time(Duration::from_secs(1));
        search.measurement_time(Duration::from_secs(5));
        search.throughput(Throughput::Elements(
            u64::try_from(F32_ROWS).expect("row count"),
        ));
        search.bench_function("exact_full_prefix_scan", |benchmark| {
            benchmark.iter(|| {
                std::hint::black_box(top_k(full, &query_full, RECALL_K, 0));
            });
        });
        search.bench_function("coarse_prefix_then_full_rerank", |benchmark| {
            benchmark.iter(|| {
                let candidates = top_k(coarse, &query_coarse, RERANK_CANDIDATES, 0);
                std::hint::black_box(rerank(full, &query_full, &candidates, RECALL_K));
            });
        });
        search.finish();
    }

    let streamed = fixture.stream_once();
    assert_eq!(streamed, fixture.bytes, "streaming output is canonical");
    {
        let mut streaming = c.benchmark_group("purremb_f32_streaming_write");
        streaming.sample_size(10);
        streaming.warm_up_time(Duration::from_secs(1));
        streaming.measurement_time(Duration::from_secs(5));
        streaming.throughput(Throughput::Bytes(bytes));
        streaming.bench_function("complete_artifact", |benchmark| {
            benchmark.iter_batched(
                || {
                    (
                        fixture.metadata(),
                        Cursor::new(Vec::with_capacity(fixture.bytes.len())),
                    )
                },
                |(metadata, output)| {
                    let mut writer = EmbeddingStreamWriter::from_typed_metadata(
                        output,
                        metadata,
                        vec![fixture.commitment.clone()],
                    )
                    .expect("stream writer");
                    writer
                        .write_f32_matrix(
                            fixture
                                .target_set
                                .targets
                                .iter()
                                .copied()
                                .zip(fixture.row_values.chunks_exact(F32_DIMENSION)),
                        )
                        .expect("stream matrix");
                    let (output, root) = writer.finish().expect("finish stream");
                    std::hint::black_box((output.into_inner().len(), root));
                },
                BatchSize::LargeInput,
            );
        });
        streaming.finish();
    }
}

fn bench_f64(c: &mut Criterion, fixture: &F64Fixture) {
    let certificate = certificate(&fixture.bytes);
    {
        let mut validation = c.benchmark_group("purremb_f64_validation");
        validation.sample_size(10);
        validation.throughput(Throughput::Bytes(usize_to_u64(fixture.bytes.len())));
        validation.bench_function("full_validation", |benchmark| {
            benchmark.iter(|| {
                let mut view = EmbeddingView::from_bytes(std::hint::black_box(&fixture.bytes))
                    .expect("structural f64 view");
                std::hint::black_box(verify_embedding(&mut view).expect("full f64 verification"));
            });
        });
        validation.finish();
    }
    let view = reopen_prevalidated(&fixture.bytes, &certificate).expect("hot f64 view");
    let matrix = view.matrices().next().expect("f64 matrix");
    c.bench_function("purremb_f64_access/native_row", |benchmark| {
        let mut row = 0u64;
        benchmark.iter(|| {
            std::hint::black_box(matrix.native_f64_row(row).expect("native f64 row"));
            row = (row + 1) % u64::try_from(F64_ROWS).expect("f64 row count");
        });
    });
}

fn bench_catalog(c: &mut Criterion, fixture: &CatalogFixture) {
    let view = EmbeddingView::from_bytes(&fixture.bytes).expect("catalog view");
    let mut group = c.benchmark_group("purremb_chunk_catalog");
    group.bench_function("target_by_id", |benchmark| {
        benchmark.iter(|| {
            std::hint::black_box(
                view.target(std::hint::black_box(fixture.sample_target))
                    .expect("catalog target"),
            );
        });
    });
    group.bench_function("document_relation_range", |benchmark| {
        benchmark.iter(|| {
            std::hint::black_box(
                view.relations_for(std::hint::black_box(fixture.sample_document))
                    .count(),
            );
        });
    });
    group.finish();
}

fn purremb_benchmarks(c: &mut Criterion) {
    let f32_fixture = build_f32_fixture();
    println!(
        "[purremb] f32_fixture rows={} dimensions={} artifact_bytes={}",
        F32_ROWS,
        F32_DIMENSION,
        f32_fixture.bytes.len()
    );
    bench_f32(c, &f32_fixture);

    let f64_fixture = build_f64_fixture();
    println!(
        "[purremb] f64_fixture rows={} dimensions={} artifact_bytes={}",
        F64_ROWS,
        fixture::F64_DIMENSION,
        f64_fixture.bytes.len()
    );
    bench_f64(c, &f64_fixture);

    let catalog = build_catalog_fixture();
    println!(
        "[purremb] chunk_catalog chunks={} artifact_bytes={}",
        catalog.chunk_count,
        catalog.bytes.len()
    );
    bench_catalog(c, &catalog);
}

criterion_group!(benches, purremb_benchmarks);
criterion_main!(benches);
