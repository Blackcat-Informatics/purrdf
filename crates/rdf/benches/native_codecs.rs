// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! Native RDF codec hot-path benchmark.
//!
//! Report-only, `cargo bench -p purrdf --bench native_codecs`. The fixture is
//! deterministic N-Quads with default and named graph rows so both parser and
//! serializer exercise dataset-capable paths.

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use purrdf_rdf::native_codecs::jsonld::{
    CompiledJsonLdContext, JsonLdSerializeOptions, derive_jsonld_context, parse_jsonld,
    serialize_dataset_to_jsonld, serialize_dataset_to_jsonld_with_options,
};
use purrdf_rdf::{
    ParseOptions, SerializeGraph, parse_dataset, parse_dataset_with, serialize_dataset,
};

#[path = "support/jsonld.rs"]
mod jsonld_fixture;

use jsonld_fixture::{LARGE_ROWS as JSONLD_LARGE_ROWS, SMALL_ROWS as JSONLD_SMALL_ROWS};

const ROWS: usize = 2_000;

/// Rows for the parallel-vs-sequential parse comparison: ~50k rows is ~4.6 MiB,
/// comfortably above the 1 MiB chunk-parallel threshold in
/// `native_codecs::text_parse` while keeping the in-memory fixture small.
const LARGE_ROWS: usize = 50_000;

fn nquads_fixture(rows: usize) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(rows * 140);
    for idx in 0..rows {
        let _ = writeln!(
            out,
            "<https://example.org/s{idx}> <https://example.org/p> \"{idx}\" <https://example.org/g{}> .",
            idx % 8
        );
    }
    out
}

fn bench_parse_nquads(c: &mut Criterion) {
    let text = nquads_fixture(ROWS);
    let mut group = c.benchmark_group("native_codecs_parse");
    group.throughput(Throughput::Bytes(text.len() as u64));
    group.bench_function("nquads_2k", |bencher| {
        bencher.iter(|| {
            let dataset = parse_dataset(black_box(text.as_bytes()), "application/n-quads", None)
                .expect("parse");
            black_box(dataset);
        });
    });
    group.finish();
}

/// Chunk-parallel vs forced-sequential N-Quads parse over the SAME large fixture.
/// `parse_dataset` auto-selects the parallel path above the 1 MiB threshold;
/// `parse_dataset_forced_sequential` (bench-only, `#[doc(hidden)]`) pins the
/// single-threaded pipeline as the baseline. Outputs are byte-identical (the
/// determinism tests in `native_codecs::text_parse` are the gate); only wall time
/// differs.
fn bench_parse_nquads_parallel_vs_sequential(c: &mut Criterion) {
    let text = nquads_fixture(LARGE_ROWS);
    let mut group = c.benchmark_group("native_codecs_parse_50k");
    group.sample_size(10);
    group.throughput(Throughput::Bytes(text.len() as u64));
    group.bench_function("nquads_50k_sequential", |bencher| {
        bencher.iter(|| {
            let dataset = purrdf_rdf::native_codecs::parse_dataset_forced_sequential(
                black_box(text.as_bytes()),
                "application/n-quads",
                None,
            )
            .expect("parse");
            black_box(dataset);
        });
    });
    group.bench_function("nquads_50k_parallel", |bencher| {
        bencher.iter(|| {
            let dataset = parse_dataset(black_box(text.as_bytes()), "application/n-quads", None)
                .expect("parse");
            black_box(dataset);
        });
    });
    group.finish();
}

/// N-Quads fixture whose literals carry quote / backslash / tab / newline / C0-control
/// escapes. Parsing decodes them to raw chars, so serialization drives the escape
/// fast-path's BOUNDARY branch (not just the clean-run copy the numeric fixture
/// exercises). Every escape is a valid N-Quads ECHAR/UCHAR so the fixture round-trips.
fn nquads_fixture_escape_heavy(rows: usize) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(rows * 180);
    for idx in 0..rows {
        let _ = writeln!(
            out,
            "<https://example.org/s{idx}> <https://example.org/p> \"q\\\"{idx}\\\" back\\\\ tab\\t nl\\n ctl\\u0001\" <https://example.org/g{}> .",
            idx % 8
        );
    }
    out
}

/// Span-tracking OFF vs ON over the SAME N-Quads fixture, REPORT-ONLY. The `off`
/// arm is `parse_dataset_with(track_source_spans=false)`, which threads the
/// zero-sized `NoSpans` collector — the same disabled-recording path
/// `parse_dataset` compiles to; the `on` arm sets `track_source_spans=true`,
/// forcing the sequential line pipeline and populating a `SpanTable`. This exists
/// so the `off` path can be OBSERVED in the criterion report to be unchanged; it
/// asserts NOTHING about relative timing (the machine is not quiet and numbers are
/// indicative only) — the byte-identical dataset guarantee is proven by the
/// `parse::tests` (`tracking_off_returns_no_table`, `dataset_is_identical_with_tracking`).
fn bench_parse_nquads_span_tracking(c: &mut Criterion) {
    let text = nquads_fixture(ROWS);
    let mut group = c.benchmark_group("native_codecs_parse_span_tracking");
    group.throughput(Throughput::Bytes(text.len() as u64));
    group.bench_function("nquads_2k_spans_off", |bencher| {
        bencher.iter(|| {
            let options = ParseOptions {
                track_source_spans: false,
            };
            let (dataset, table) = parse_dataset_with(
                black_box(text.as_bytes()),
                "application/n-quads",
                None,
                black_box(&options),
            )
            .expect("parse");
            black_box((dataset, table));
        });
    });
    group.bench_function("nquads_2k_spans_on", |bencher| {
        bencher.iter(|| {
            let options = ParseOptions {
                track_source_spans: true,
            };
            let (dataset, table) = parse_dataset_with(
                black_box(text.as_bytes()),
                "application/n-quads",
                None,
                black_box(&options),
            )
            .expect("parse");
            black_box((dataset, table));
        });
    });
    group.finish();
}

fn bench_serialize_nquads(c: &mut Criterion) {
    let clean = nquads_fixture(ROWS);
    let clean_ds = parse_dataset(clean.as_bytes(), "application/n-quads", None).expect("parse");
    let dirty = nquads_fixture_escape_heavy(ROWS);
    let dirty_ds = parse_dataset(dirty.as_bytes(), "application/n-quads", None).expect("parse");

    let mut group = c.benchmark_group("native_codecs_serialize");
    group.throughput(Throughput::Elements(clean_ds.quad_count() as u64));
    // Clean literals: the escape scan finds no boundary, one wholesale copy per literal.
    group.bench_function("nquads_2k", |bencher| {
        bencher.iter(|| {
            let bytes = serialize_dataset(
                black_box(&clean_ds),
                "application/n-quads",
                SerializeGraph::Dataset,
            )
            .expect("serialize");
            black_box(bytes);
        });
    });
    // Escape-heavy literals: every literal has multiple boundary chars, exercising the
    // per-char fallback interleaved with clean-run copies.
    group.bench_function("nquads_2k_escape_heavy", |bencher| {
        bencher.iter(|| {
            let bytes = serialize_dataset(
                black_box(&dirty_ds),
                "application/n-quads",
                SerializeGraph::Dataset,
            )
            .expect("serialize");
            black_box(bytes);
        });
    });
    group.finish();
}

/// Pre-change expanded JSON-LD parse/serialize timing over one deterministic RDF 1.2
/// fixture at two scales. Allocation and peak-memory metrics live in the separate
/// `jsonld_alloc` process so allocator atomics cannot perturb Criterion timings.
fn bench_jsonld_expanded(c: &mut Criterion) {
    let mut group = c.benchmark_group("jsonld_expanded_baseline");
    for rows in [JSONLD_SMALL_ROWS, JSONLD_LARGE_ROWS] {
        let dataset = jsonld_fixture::build_dataset(rows);
        let json = serialize_dataset_to_jsonld(&dataset).expect("prepare expanded JSON-LD");
        group.throughput(Throughput::Elements(
            u64::try_from(dataset.quad_count()).expect("quad count fits in u64"),
        ));
        group.bench_with_input(
            BenchmarkId::new("serialize", rows),
            &dataset,
            |bencher, ds| {
                bencher.iter(|| {
                    let output = serialize_dataset_to_jsonld(black_box(ds))
                        .expect("expanded JSON-LD serialization");
                    black_box(output);
                });
            },
        );
        group.bench_with_input(BenchmarkId::new("parse", rows), &json, |bencher, text| {
            bencher.iter(|| {
                let dataset =
                    parse_jsonld(black_box(text.as_bytes())).expect("expanded JSON-LD parse");
                black_box(dataset);
            });
        });
        eprintln!(
            "[jsonld_baseline] rows={rows} quads={} output_bytes={}",
            dataset.quad_count(),
            json.len()
        );
    }
    group.finish();
}

/// Configured context compilation and caller/derived serialization are measured
/// separately so context reuse is visible rather than hidden in codec throughput.
fn bench_jsonld_configured(c: &mut Criterion) {
    use std::sync::Arc;

    {
        let mut compile = c.benchmark_group("jsonld_context_compile");
        compile.bench_function("prefixes", |bencher| {
            bencher.iter(|| {
                let context = CompiledJsonLdContext::from_prefixes(black_box([
                    ("ex", "https://example.org/"),
                    ("p", "https://example.org/p/"),
                    ("o", "https://example.org/o/"),
                ]))
                .expect("compile context");
                black_box(context);
            });
        });
        compile.finish();
    }

    let context = Arc::new(
        CompiledJsonLdContext::from_prefixes([
            ("ex", "https://example.org/"),
            ("p", "https://example.org/p/"),
            ("o", "https://example.org/o/"),
        ])
        .expect("compile context"),
    );
    let caller = JsonLdSerializeOptions::compiled(context);
    let derived = JsonLdSerializeOptions::derived();
    let mut group = c.benchmark_group("jsonld_configured");
    for rows in [JSONLD_SMALL_ROWS, JSONLD_LARGE_ROWS] {
        let dataset = jsonld_fixture::build_dataset(rows);
        for (mode, options) in [("caller", &caller), ("derived", &derived)] {
            let prepared = serialize_dataset_to_jsonld_with_options(&dataset, options)
                .expect("prepare configured JSON-LD");
            group.bench_with_input(
                BenchmarkId::new(format!("serialize_{mode}"), rows),
                &dataset,
                |bencher, value| {
                    bencher.iter(|| {
                        black_box(
                            serialize_dataset_to_jsonld_with_options(black_box(value), options)
                                .expect("configured JSON-LD serialization"),
                        );
                    });
                },
            );
            group.bench_with_input(
                BenchmarkId::new(format!("parse_{mode}"), rows),
                &prepared,
                |bencher, text| {
                    bencher.iter(|| {
                        black_box(
                            parse_jsonld(black_box(text.as_bytes()))
                                .expect("configured JSON-LD parse"),
                        );
                    });
                },
            );
            eprintln!(
                "[jsonld_configured] mode={mode} rows={rows} quads={} output_bytes={}",
                dataset.quad_count(),
                prepared.len()
            );
        }
    }
    group.finish();
}

/// Context derivation has an explicit work ceiling; this adversarial shape keeps
/// many profitable namespaces and distinct IRIs below it while exposing scaling.
fn bench_jsonld_derived_many_namespaces(c: &mut Criterion) {
    const NAMESPACES: usize = 64;
    const IRIS_PER_NAMESPACE: usize = 32;

    let dataset = jsonld_fixture::build_many_namespace_dataset(NAMESPACES, IRIS_PER_NAMESPACE);
    let mut group = c.benchmark_group("jsonld_derived_context");
    group.sample_size(10);
    group.bench_function("64_namespaces_32_iris", |bencher| {
        bencher.iter(|| {
            black_box(derive_jsonld_context(black_box(&dataset)).expect("derive bounded context"));
        });
    });
    group.finish();
}

/// Named-graph usage accounting and multivalue ordering are independent carrier
/// hot paths, so keep adversarial shapes for both in the report-only benchmark.
fn bench_jsonld_carrier_stress(c: &mut Criterion) {
    let context = std::sync::Arc::new(
        CompiledJsonLdContext::from_prefixes([("bench", "https://bench.example/")])
            .expect("compile benchmark context"),
    );
    let options = JsonLdSerializeOptions::compiled(context);
    let mut group = c.benchmark_group("jsonld_carrier_stress");
    group.sample_size(10);
    for (name, dataset) in [
        (
            "named_graphs_512",
            jsonld_fixture::build_many_named_graph_dataset(512),
        ),
        (
            "multivalue_4096",
            jsonld_fixture::build_multivalue_dataset(4_096),
        ),
    ] {
        group.bench_with_input(name, &dataset, |bencher, dataset| {
            bencher.iter(|| {
                black_box(
                    serialize_dataset_to_jsonld_with_options(black_box(dataset), &options)
                        .expect("serialize carrier stress fixture"),
                );
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_parse_nquads,
    bench_parse_nquads_parallel_vs_sequential,
    bench_parse_nquads_span_tracking,
    bench_serialize_nquads,
    bench_jsonld_expanded,
    bench_jsonld_configured,
    bench_jsonld_derived_many_namespaces,
    bench_jsonld_carrier_stress
);
criterion_main!(benches);
