// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! IRI/URI parse+validate hot-path benchmark.
//!
//! Report-only, `cargo bench -p purrdf-iri --bench parse`. `purrdf_iri::parse`
//! validates every component character-by-character; this exercises the ASCII
//! character-class checks (the const class-bitmap LUT) across a representative
//! corpus — an http URL with path/query/fragment, an IPv6 IP-literal authority, a
//! long percent-encoded path, a non-ASCII `ucschar` IRI, and a scheme-heavy set.
//!
//! `iri_resolve/dot_segments` is the RFC-3986 §5 resolve path: relative references
//! dense in `.`/`..` segments against a deep base, so the §5.2.4 remove-dot-segments
//! walk (a borrowed cursor, no per-segment buffer rebuild) is the measured cost.

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use purrdf_iri::parse;

/// The representative corpus. Each entry is parsed+validated per iteration; the mix
/// keeps every component validator (scheme, authority/host, path, query, fragment) on
/// the hot path so the LUT change is measurable rather than dominated by one shape.
const CORPUS: &[&str] = &[
    "https://user@example.org:8443/path/to/resource?q=1&lang=en#section-2",
    "https://[2001:db8::7334]:443/api/v2/items?filter=active#top",
    "http://example.org/a/very/long/percent%20encoded/path%2Fsegment/with/many/components/deep",
    "https://例え.example.org/パス/ページ?クエリ=値#フラグメント",
    "urn:isbn:0451450523",
    "file:///home/user/documents/report.pdf",
    "mailto:someone@example.org",
    "ftp://ftp.example.org/pub/files/archive.tar.gz",
    "https://example.org/",
    "coap+tcp://node.example.org/.well-known/core",
];

/// Base for the resolve-heavy case: a deep path so `../` pops have segments to
/// remove and the merged buffer is long enough that the §5.2.4 dot-segment walk
/// dominates the resolve.
const RESOLVE_BASE: &str = "http://example.org/a/b/c/d/e/f/g/h?q=1";

/// Relative references heavy in `.`/`..` segments — the case B/C rewrites of
/// RFC-3986 §5.2.4 that the borrowed-cursor `remove_dot_segments` turns from
/// per-segment `String` rebuilds into slices.
const RESOLVE_REFS: &[&str] = &[
    "../../a/./b/../c",
    "./x/./y/../../z/./w",
    "../../../../../../../../deep/../../root",
    "g/./h/../i/./j/../k/./l/../m",
    "/a/./b/../c/./d/../e/./f/../g",
    "../.././../a/b/c/d/e/../../../../f",
    ".",
    "..",
    "./",
    "../",
];

fn bench_resolve(c: &mut Criterion) {
    let base = parse(RESOLVE_BASE).expect("resolve base is valid");
    let total_bytes: usize = RESOLVE_REFS.iter().map(|s| s.len()).sum();
    let mut group = c.benchmark_group("iri_resolve");
    group.throughput(Throughput::Bytes(total_bytes as u64));
    group.bench_function("dot_segments", |bencher| {
        bencher.iter(|| {
            for &reference in RESOLVE_REFS {
                let resolved = base
                    .resolve(black_box(reference))
                    .expect("reference resolves against the base");
                black_box(resolved);
            }
        });
    });
    group.finish();
}

fn bench_parse(c: &mut Criterion) {
    let total_bytes: usize = CORPUS.iter().map(|s| s.len()).sum();
    let mut group = c.benchmark_group("iri_parse");
    group.throughput(Throughput::Bytes(total_bytes as u64));
    group.bench_function("corpus", |bencher| {
        bencher.iter(|| {
            for &iri in CORPUS {
                let parsed = parse(black_box(iri)).expect("corpus IRI is valid");
                black_box(parsed);
            }
        });
    });
    group.finish();
}

criterion_group!(benches, bench_parse, bench_resolve);
criterion_main!(benches);
