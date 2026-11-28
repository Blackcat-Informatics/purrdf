// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API, so the workspace `missing_docs` lint is
// not asked of their items.
#![allow(missing_docs)]

//! Byte-class scanner and JSON string escape benchmark.
//!
//! Report-only, `cargo bench -p purrdf-lex --bench scan`.
//!
//! `lex_scan/*` runs each chunked byte-class scanner in `purrdf_lex::terminals`
//! over one long clean run ending in the byte it stops at, so the sixteen-byte
//! chunk loop is the measured cost, and over a short token-sized run, so the
//! per-call setup is.
//!
//! `lex_json_escape/*` runs the shared JSON string escape law
//! (`purrdf_lex::json_escape`) in each of its four spellings, over a long clean
//! ASCII run (the chunked stop scan is the cost) and over mixed text dense in
//! stops (the per-`char` escape table is).

use purrdf_lex::json_escape::{JsonEscapes, push_body};
use purrdf_lex::terminals::{
    find_first_iri_body_special, find_first_json_string_special, find_first_trivia,
    find_first_xml_special,
};
use purrdf_testkit::bench::{Bench, Throughput, bench_group, bench_main, black_box};

/// A scanner under measurement and the byte its input ends at.
type ScanCase = (&'static str, fn(&[u8]) -> Option<usize>, u8, u8);

fn bench_scan(c: &mut Bench) {
    // (name, scanner, the clean byte a run is made of, the byte that ends it)
    let cases: [ScanCase; 4] = [
        ("trivia", find_first_trivia, b' ', b'?'),
        ("iri_body", find_first_iri_body_special, b'a', b'>'),
        ("json_string", find_first_json_string_special, b'a', b'"'),
        ("xml", find_first_xml_special, b'a', b'<'),
    ];
    for (run, label) in [(4096_usize, "long"), (7, "token")] {
        let mut group = c.benchmark_group(format!("lex_scan_{label}"));
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

fn bench_json_escape(c: &mut Bench) {
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
        let mut group = c.benchmark_group(format!("lex_json_escape_{label}"));
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

bench_group!(benches, bench_scan, bench_json_escape);
bench_main!(benches);
