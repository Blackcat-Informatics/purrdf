// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! GTS authoring hot-path benchmark.
//!
//! Report-only, `cargo bench -p purrdf-gts` (the `make bench` lane). This keeps
//! the core container work measurable: rsyncable zstd block compression and
//! deterministic snapshot emission over a representative folded graph.

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};

use purrdf_gts::codec::encode_chain;
use purrdf_gts::model::{Graph, Term, TermKind};
use purrdf_gts::reader::read;
use purrdf_gts::writer::{SnapshotOptions, Writer, snapshot_from_graph};

const PAYLOAD_LEN: usize = 512 * 1024;
const ROWS: usize = 2_000;

// Verify-bench container shape: a multi-segment `cat` file whose integrity
// checks are dominated by BLAKE3 content-id and blob-digest work. Roughly
// 4 segments x (4 x 2 MiB blobs + 256 term frames) ~= 32 MiB — big enough to
// exercise the parallel paths, small enough for CI-adjacent runs.
const VERIFY_SEGMENTS: usize = 4;
const VERIFY_BLOBS_PER_SEGMENT: usize = 4;
const VERIFY_BLOB_LEN: usize = 2 * 1024 * 1024;
const VERIFY_TERM_FRAMES_PER_SEGMENT: usize = 256;

fn deterministic_payload(len: usize) -> Vec<u8> {
    seeded_payload(len, 0)
}

fn seeded_payload(len: usize, seed: usize) -> Vec<u8> {
    (0..len)
        .map(|idx| {
            let value = idx
                .wrapping_mul(31)
                .wrapping_add(idx / 7)
                .wrapping_add(seed.wrapping_mul(131));
            value as u8
        })
        .collect()
}

fn iri(value: impl Into<String>) -> Term {
    Term {
        kind: TermKind::Iri,
        value: Some(value.into()),
        datatype: None,
        lang: None,
        direction: None,
        reifier: None,
    }
}

fn literal(value: impl Into<String>, datatype: usize) -> Term {
    Term {
        kind: TermKind::Literal,
        value: Some(value.into()),
        datatype: Some(datatype),
        lang: None,
        direction: None,
        reifier: None,
    }
}

fn graph_with_quads(rows: usize) -> Graph {
    let mut graph = Graph::default();
    let p = graph.terms.len();
    graph.terms.push(iri("http://example.org/p"));
    let datatype = graph.terms.len();
    graph
        .terms
        .push(iri("http://www.w3.org/2001/XMLSchema#integer"));
    let g = graph.terms.len();
    graph.terms.push(iri("http://example.org/g"));

    for idx in 0..rows {
        let s = graph.terms.len();
        graph.terms.push(iri(format!("http://example.org/s{idx}")));
        let o = graph.terms.len();
        graph.terms.push(literal(idx.to_string(), datatype));
        graph.quads.push((s, p, o, (idx % 5 == 0).then_some(g)));
    }
    graph
}

fn bench_rsyncable_zstd(c: &mut Criterion) {
    let payload = deterministic_payload(PAYLOAD_LEN);
    let chain = vec!["zstd-rsyncable".to_string()];

    let mut group = c.benchmark_group("gts_codec");
    group.throughput(Throughput::Bytes(payload.len() as u64));
    group.bench_function("zstd_rsyncable_512k", |bencher| {
        bencher.iter(|| {
            let encoded = encode_chain(black_box(&chain), black_box(&payload)).expect("encode");
            black_box(encoded);
        });
    });
    group.finish();
}

fn bench_snapshot_authoring(c: &mut Criterion) {
    let graph = graph_with_quads(ROWS);

    let mut group = c.benchmark_group("gts_authoring");
    group.throughput(Throughput::Elements(ROWS as u64));
    group.bench_function("snapshot_2k_quads", |bencher| {
        bencher.iter(|| {
            let bytes = snapshot_from_graph(
                black_box(&graph),
                black_box("bench"),
                SnapshotOptions::default(),
            )
            .expect("snapshot");
            black_box(bytes);
        });
    });
    group.finish();
}

/// Author a synthetic multi-segment container (§3.1 `cat` composition) with
/// sizeable inline blobs and a long per-segment frame chain.
fn verify_container() -> Vec<u8> {
    let mut data = Vec::new();
    for segment in 0..VERIFY_SEGMENTS {
        let mut writer = Writer::new("generic");
        for frame in 0..VERIFY_TERM_FRAMES_PER_SEGMENT {
            writer.add_terms(&[iri(format!("http://example.org/s{segment}/t{frame}"))]);
        }
        for blob in 0..VERIFY_BLOBS_PER_SEGMENT {
            let payload = seeded_payload(
                VERIFY_BLOB_LEN,
                segment * VERIFY_BLOBS_PER_SEGMENT + blob + 1,
            );
            writer.add_blob_owned(payload, Some("application/octet-stream"), None);
        }
        writer.add_index_with_mmr();
        data.extend_from_slice(&writer.into_bytes());
    }
    data
}

fn bench_verify(c: &mut Criterion) {
    let data = verify_container();

    let mut group = c.benchmark_group("gts_verify");
    group.throughput(Throughput::Bytes(data.len() as u64));
    group.sample_size(10);
    group.bench_function("read_multisegment_32mib", |bencher| {
        bencher.iter(|| {
            let graph = read(black_box(&data), true, None);
            assert!(graph.diagnostics.is_empty(), "container must verify clean");
            black_box(graph);
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_rsyncable_zstd,
    bench_snapshot_authoring,
    bench_verify
);
criterion_main!(benches);
