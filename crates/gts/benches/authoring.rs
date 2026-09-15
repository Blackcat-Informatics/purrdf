// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! GTS authoring and reader hot-path benchmarks.
//!
//! Report-only, `cargo bench -p purrdf-gts` (the `make bench` lane). This keeps
//! the core container work measurable: rsyncable zstd block compression and
//! deterministic snapshot emission over a representative folded graph.
//! Reader cases vary blob count independently of payload size and compare
//! standalone decryption with encrypted frame streaming. Allocation counters
//! report cumulative traffic on the calling thread, not peak memory or a
//! process-wide total; parallel full folds can allocate on other threads.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use ciborium::value::Value;
use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use ed25519_dalek::SigningKey;

use purrdf_gts::codec::encode_chain;
use purrdf_gts::compact::{CompactionParams, DictPlan, DictStrategy, compact_streamable};
use purrdf_gts::mmr;
use purrdf_gts::model::{Graph, Suppression, Term, TermKind};
use purrdf_gts::reader::{
    BlobPayload, ReadOptions, StreamingSink, read, read_to_sink_with_options,
};
use purrdf_gts::wire::{canonical, deterministic, encode};
use purrdf_gts::writer::{
    Encrypt0Options, FrameOptions, SnapshotOptions, Writer, digest_string, snapshot_from_graph,
};

thread_local! {
    static ALLOCATIONS: Cell<u64> = const { Cell::new(0) };
    static ALLOCATED_BYTES: Cell<u64> = const { Cell::new(0) };
}

struct CountingAllocator;

// SAFETY: every operation forwards the original pointer/layout to the system
// allocator; the thread-local counters are observational only.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.with(|count| count.set(count.get() + 1));
        ALLOCATED_BYTES.with(|bytes| bytes.set(bytes.get() + layout.size() as u64));
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.with(|count| count.set(count.get() + 1));
        ALLOCATED_BYTES.with(|bytes| bytes.set(bytes.get() + new_size as u64));
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn allocation_snapshot() -> (u64, u64) {
    (ALLOCATIONS.with(Cell::get), ALLOCATED_BYTES.with(Cell::get))
}

fn allocation_delta(before: (u64, u64), after: (u64, u64)) -> (u64, u64) {
    (after.0 - before.0, after.1 - before.1)
}

const PAYLOAD_LEN: usize = 512 * 1024;
const ROWS: usize = 2_000;

// Canonical-author bench shape: enough quads that the per-element key sorts
// dominate, plus suppressions (each keyed by a canonical CBOR map) and blobs
// carrying `pub` metadata (media type / representation lookup per blob).
const CANONICAL_ROWS: usize = 4_000;
const CANONICAL_SUPPRESSIONS: usize = 400;
const CANONICAL_BLOBS: usize = 64;
const CANONICAL_BLOB_LEN: usize = 256;

// MMR root bench: a few thousand 32-byte frame ids, as a long segment's
// `index.mmr` footer would commit.
const MMR_FRAME_IDS: usize = 4_096;

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
        triple: None,
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
        triple: None,
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

/// A folded graph shaped for the canonical `Writer::deterministic` path:
/// quads (some in a named graph), quad/term suppressions with reasons and
/// actors, and content-addressed blobs that all carry `mt`/`rep` metadata.
fn canonical_graph() -> Graph {
    let mut graph = graph_with_quads(CANONICAL_ROWS);
    let by = graph.terms.len();
    graph.terms.push(iri("http://example.org/auditor"));
    for idx in 0..CANONICAL_SUPPRESSIONS {
        let (s, p, o, _) = graph.quads[(idx * 7) % graph.quads.len()];
        let target = if idx % 3 == 0 {
            Value::Map(vec![
                ("kind".into(), "term".into()),
                ("id".into(), Value::from(s as u64)),
            ])
        } else {
            Value::Map(vec![
                ("kind".into(), "quad".into()),
                (
                    "q".into(),
                    Value::Array(vec![
                        Value::from(s as u64),
                        Value::from(p as u64),
                        Value::from(o as u64),
                    ]),
                ),
            ])
        };
        graph.suppressions.push(Suppression {
            targets: vec![target],
            reason: (idx % 2 == 0).then(|| format!("retracted claim {idx}")),
            by: (idx % 4 != 0).then_some(by),
        });
    }
    for idx in 0..CANONICAL_BLOBS {
        let data = seeded_payload(CANONICAL_BLOB_LEN, idx + 1);
        let digest = digest_string(&data);
        graph.set_blob_meta(
            digest.clone(),
            Value::Map(vec![
                ("mt".into(), "application/octet-stream".into()),
                ("rep".into(), format!("blob-{idx}").into()),
            ]),
        );
        graph.set_blob(digest, data);
    }
    graph
}

/// `Writer::deterministic` over the canonical graph: term remap (identity
/// keys + nesting depth), the cached-key quad/suppression sorts, and the
/// per-blob metadata lookup. Report-only.
fn bench_canonical_authoring(c: &mut Criterion) {
    let graph = canonical_graph();
    let before = allocation_snapshot();
    let bytes = Writer::deterministic(&graph, "bench")
        .expect("canonical author")
        .into_bytes();
    let alloc = allocation_delta(before, allocation_snapshot());
    println!(
        "[gts_authoring] canonical author: {} bytes out; allocations={} bytes={}",
        bytes.len(),
        alloc.0,
        alloc.1
    );

    let mut group = c.benchmark_group("gts_authoring");
    group.throughput(Throughput::Elements(CANONICAL_ROWS as u64));
    group.bench_function("canonical_author_4k_quads_suppressions_blobs", |bencher| {
        bencher.iter(|| {
            let bytes = Writer::deterministic(black_box(&graph), black_box("bench"))
                .expect("canonical author")
                .into_bytes();
            black_box(bytes);
        });
    });
    group.finish();
}

/// `mmr::root` over a long frame-id list (the peaks-only fold). Report-only.
fn bench_mmr_root(c: &mut Criterion) {
    let frame_ids: Vec<Vec<u8>> = (0..MMR_FRAME_IDS)
        .map(|idx| seeded_payload(32, idx))
        .collect();
    let before = allocation_snapshot();
    let root = mmr::root(&frame_ids);
    let alloc = allocation_delta(before, allocation_snapshot());
    println!(
        "[gts_mmr] root over {MMR_FRAME_IDS} frame ids: {} bytes; allocations={} bytes={}",
        root.len(),
        alloc.0,
        alloc.1
    );

    let mut group = c.benchmark_group("gts_mmr");
    group.throughput(Throughput::Elements(MMR_FRAME_IDS as u64));
    group.bench_function("root_4k_frame_ids", |bencher| {
        bencher.iter(|| {
            let root = mmr::root(black_box(&frame_ids));
            black_box(root);
        });
    });
    group.finish();
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
    let payload = graph.snapshot_payload();
    let before = allocation_snapshot();
    let legacy = encode(&deterministic(&payload));
    let legacy_alloc = allocation_delta(before, allocation_snapshot());
    let before = allocation_snapshot();
    let borrowed = canonical(&payload);
    let borrowed_alloc = allocation_delta(before, allocation_snapshot());
    assert_eq!(
        borrowed, legacy,
        "borrowed canonical bytes must match oracle"
    );
    println!(
        "[gts_authoring] canonical snapshot: recursive allocations={} bytes={}; borrowed allocations={} bytes={}",
        legacy_alloc.0, legacy_alloc.1, borrowed_alloc.0, borrowed_alloc.1
    );

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

#[derive(Default)]
struct ReaderSink {
    blobs: usize,
    bytes: usize,
}

impl StreamingSink for ReaderSink {
    fn blob_payload(&mut self, payload: BlobPayload<'_>) {
        self.blobs += 1;
        self.bytes += payload.bytes.map_or(0, <[u8]>::len);
    }
}

fn read_blob_stream(data: &[u8], options: ReadOptions<'_>) -> ReaderSink {
    let mut sink = ReaderSink::default();
    let result = read_to_sink_with_options(data, options, &mut sink);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    sink
}

fn many_blob_container(count: usize, metadata: bool) -> Vec<u8> {
    let mut writer = Writer::new("generic");
    for index in 0..count {
        let mut payload = deterministic_payload(256);
        // Keep every blob unique even beyond the payload generator's byte period.
        payload[..8].copy_from_slice(&(index as u64).to_le_bytes());
        if metadata {
            writer.add_blob_owned(payload, Some("application/octet-stream"), None);
        } else {
            writer.add_frame("blob", None, Some(payload), None, None);
        }
    }
    writer.into_bytes()
}

fn bench_reader_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("gts_reader_scaling");
    group.sample_size(10);
    for count in [256, 1024, 4096, 16384] {
        group.throughput(Throughput::Elements(count as u64));
        for metadata in [false, true] {
            let data = many_blob_container(count, metadata);
            let label = if metadata { "metadata" } else { "plain" };
            let before = allocation_snapshot();
            let sink = read_blob_stream(&data, ReadOptions::new(true, None));
            let allocated = allocation_delta(before, allocation_snapshot());
            assert_eq!((sink.blobs, sink.bytes), (count, count * 256));
            println!(
                "[gts_reader] stream_{label}/{count}: encoded={} allocations={} allocated_bytes={}",
                data.len(),
                allocated.0,
                allocated.1
            );
            group.bench_with_input(
                BenchmarkId::new(format!("stream_{label}"), count),
                &data,
                |bencher, data| {
                    bencher.iter(|| {
                        black_box(read_blob_stream(
                            black_box(data),
                            ReadOptions::new(true, None),
                        ))
                    });
                },
            );
            let before = allocation_snapshot();
            let graph = read(&data, true, None);
            let allocated = allocation_delta(before, allocation_snapshot());
            assert!(graph.diagnostics.is_empty(), "{:?}", graph.diagnostics);
            assert_eq!(graph.blobs.len(), count);
            println!(
                "[gts_reader] fold_{label}/{count}: encoded={} allocations={} allocated_bytes={}",
                data.len(),
                allocated.0,
                allocated.1
            );
            group.bench_with_input(
                BenchmarkId::new(format!("fold_{label}"), count),
                &data,
                |bencher, data| {
                    bencher.iter(|| {
                        let graph = read(black_box(data), true, None);
                        assert!(graph.diagnostics.is_empty(), "{:?}", graph.diagnostics);
                        black_box(graph)
                    });
                },
            );
        }
    }
    group.finish();
}

fn bench_reader_decryption(c: &mut Criterion) {
    use purrdf_gts::cose::{decrypt0, encrypt0};

    let mut key = [0; 32];
    getrandom::fill(&mut key).expect("benchmark encryption randomness");
    let resolve = |kid: &str| (kid == "reader-benchmark").then_some(key);
    let mut group = c.benchmark_group("gts_reader_decryption");
    group.sample_size(10);
    for (index, length) in [256, 65536, 1_048_576].into_iter().enumerate() {
        let plaintext = deterministic_payload(length);
        let iv = [u8::try_from(index).unwrap(); 12];
        let envelope = encrypt0(&plaintext, "reader-benchmark", &key, &iv);
        let mut writer = Writer::new("generic");
        writer
            .add_frame_with_options(
                "blob",
                FrameOptions {
                    raw: Some(plaintext.clone()),
                    encrypt: Some(Encrypt0Options {
                        kid: "reader-benchmark".into(),
                        key,
                        iv,
                    }),
                    ..FrameOptions::default()
                },
            )
            .unwrap();
        let container = writer.into_bytes();
        group.throughput(Throughput::Bytes(length as u64));
        let before = allocation_snapshot();
        assert_eq!(decrypt0(&envelope, resolve).unwrap(), plaintext);
        let allocated = allocation_delta(before, allocation_snapshot());
        println!(
            "[gts_decryption] standalone/{length}: allocations={} allocated_bytes={}",
            allocated.0, allocated.1
        );
        group.bench_with_input(
            BenchmarkId::new("standalone", length),
            &envelope,
            |b, data| {
                b.iter(|| black_box(decrypt0(black_box(data), resolve).unwrap()));
            },
        );
        let before = allocation_snapshot();
        let sink = read_blob_stream(
            &container,
            ReadOptions::new(true, None).with_content_key(&resolve),
        );
        let allocated = allocation_delta(before, allocation_snapshot());
        assert_eq!((sink.blobs, sink.bytes), (1, length));
        println!(
            "[gts_decryption] stream/{length}: allocations={} allocated_bytes={}",
            allocated.0, allocated.1
        );
        group.bench_with_input(BenchmarkId::new("stream", length), &container, |b, data| {
            b.iter(|| {
                black_box(read_blob_stream(
                    black_box(data),
                    ReadOptions::new(true, None).with_content_key(&resolve),
                ))
            });
        });
    }
    group.finish();
}

/// A fixed multi-blob source with repeated structure — the corpus a pack
/// dictionary strategy actually has something to train on (mirrors
/// `purrdf_gts::compact::tests::source_with_blobs`).
const DICT_BLOB_COUNT: u32 = 64;

fn dict_compaction_source() -> Vec<u8> {
    let mut w = Writer::new("purrdf.gts");
    for i in 0..DICT_BLOB_COUNT {
        let blob = format!(
            "<https://example.org/s{}> <https://example.org/p> \"claim {} about cats\" .\n",
            i % 37,
            i
        )
        .into_bytes();
        w.add_blob_owned(blob, Some("text/plain"), None);
    }
    w.into_bytes()
}

/// Streamable compaction with a FastCOVER-trained in-band pack dictionary
/// (GTS-SPEC §5 `"dct"`, §8.5 `zstd` `dct` parameter) — report-only, no
/// speedup assertion.
fn bench_dict_compaction(c: &mut Criterion) {
    let source = dict_compaction_source();

    let mut group = c.benchmark_group("gts_compact");
    group.throughput(Throughput::Bytes(source.len() as u64));
    group.bench_function("trained_dict_64_blobs", |bencher| {
        bencher.iter(|| {
            let packed = compact_streamable(
                black_box(&source),
                CompactionParams {
                    timestamp: "2026-01-01T00:00:00Z",
                    seal_original: false,
                    plan: DictPlan::single(DictStrategy::Trained),
                    content_digest: None,
                    packaging_signer: (SigningKey::from_bytes(&[42u8; 32]), "bench".to_string()),
                },
            )
            .expect("trained-dict compaction succeeds");
            black_box(packed);
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_rsyncable_zstd,
    bench_snapshot_authoring,
    bench_canonical_authoring,
    bench_mmr_root,
    bench_verify,
    bench_dict_compaction,
    bench_reader_scaling,
    bench_reader_decryption
);
criterion_main!(benches);
