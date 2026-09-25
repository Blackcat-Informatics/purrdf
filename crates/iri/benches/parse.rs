// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
//!
//! `iri_scan/*` runs each chunked byte-class scanner in `purrdf_iri::terminals`
//! over one long clean run ending in the byte it stops at, so the sixteen-byte
//! chunk loop is the measured cost, and over a short token-sized run, so the
//! per-call setup is.
//!
//! `iri_json_escape/*` runs the shared JSON string escape law
//! (`purrdf_iri::json_escape`) in each of its four spellings, over a long clean
//! ASCII run (the chunked stop scan is the cost) and over mixed text dense in
//! stops (the per-`char` escape table is).

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use purrdf_iri::json_escape::{JsonEscapes, push_body};
use purrdf_iri::parse;
use purrdf_iri::terminals::{
    find_first_iri_body_special, find_first_json_string_special, find_first_trivia,
    find_first_xml_special,
};

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

/// A scanner under measurement and the byte its input ends at.
type ScanCase = (&'static str, fn(&[u8]) -> Option<usize>, u8, u8);

fn bench_scan(c: &mut Criterion) {
    // (name, scanner, the clean byte a run is made of, the byte that ends it)
    let cases: [ScanCase; 4] = [
        ("trivia", find_first_trivia, b' ', b'?'),
        ("iri_body", find_first_iri_body_special, b'a', b'>'),
        ("json_string", find_first_json_string_special, b'a', b'"'),
        ("xml", find_first_xml_special, b'a', b'<'),
    ];
    for (run, label) in [(4096_usize, "long"), (7, "token")] {
        let mut group = c.benchmark_group(format!("iri_scan_{label}"));
        group.throughput(Throughput::Bytes(run as u64 + 1));
        for (name, scan, clean, end) in cases {
            let mut input = vec![clean; run];
            input.push(end);
            group.bench_function(name, |bencher| {
                bencher.iter(|| {
                    let at = scan(black_box(&input)).expect("the input ends at a member");
                    black_box(at);
                });
            });
        }
        group.finish();
    }
}

fn bench_json_escape(c: &mut Criterion) {
    let mut clean = "a".repeat(4096);
    clean.push('"');
    let mixed =
        "caf\u{e9} \"quoted\"\tline\n\u{1}\u{7f}\u{85} \u{4e2d}\u{6587} \u{1f431} ".repeat(64);
    let spellings = [
        ("minimal", JsonEscapes::Minimal),
        ("short_forms", JsonEscapes::ShortForms),
        ("controls", JsonEscapes::Controls),
        ("ascii", JsonEscapes::Ascii),
    ];
    for (label, value) in [("clean", &clean), ("mixed", &mixed)] {
        let mut group = c.benchmark_group(format!("iri_json_escape_{label}"));
        group.throughput(Throughput::Bytes(value.len() as u64));
        for (name, escapes) in spellings {
            let mut out = String::with_capacity(value.len() * 6);
            group.bench_function(name, |bencher| {
                bencher.iter(|| {
                    out.clear();
                    push_body(&mut out, black_box(value), escapes);
                    black_box(out.len());
                });
            });
        }
        group.finish();
    }
}

criterion_group!(
    benches,
    bench_parse,
    bench_resolve,
    bench_scan,
    bench_json_escape
);
criterion_main!(benches);
