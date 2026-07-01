// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! GTS authoring hot-path benchmark.
//!
//! Report-only, `cargo bench -p purrdf-gts` (the `make bench` lane). This keeps
//! the core container work measurable: rsyncable zstd block compression and
//! deterministic snapshot emission over a representative folded graph.

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

use purrdf_gts::codec::encode_chain;
use purrdf_gts::model::{Graph, Term, TermKind};
use purrdf_gts::writer::{snapshot_from_graph, SnapshotOptions};

const PAYLOAD_LEN: usize = 512 * 1024;
const ROWS: usize = 2_000;

fn deterministic_payload(len: usize) -> Vec<u8> {
    (0..len)
        .map(|idx| {
            let value = idx.wrapping_mul(31).wrapping_add(idx / 7);
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

criterion_group!(benches, bench_rsyncable_zstd, bench_snapshot_authoring);
criterion_main!(benches);
