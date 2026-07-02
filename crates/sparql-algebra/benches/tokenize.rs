// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Tokenizer hot-path benchmark.
//!
//! Report-only, `cargo bench -p purrdf-sparql-algebra --bench tokenize`. The
//! fixture is a large Turtle-shaped document that exercises exactly the scans the
//! byte-cursor lexer accelerates: long `IRIREF` bodies (memchr `>`), string-literal
//! bodies (memchr2 `"`/`\`), and `#` comment tails (memchr `\n`) — plus the removal
//! of the former `char_indices().collect()` full-input materialization.

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use purrdf_sparql_algebra::lexer::{tokenize, tokenize_turtle};

const ROWS: usize = 4_000;

/// A Turtle-shaped fixture: each row has a long subject/predicate/object IRI, a
/// string-literal object with an escape, and a trailing `#` comment.
fn turtle_fixture(rows: usize) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(rows * 200);
    for idx in 0..rows {
        let _ = writeln!(
            out,
            "<https://example.org/dataset/entity/{idx}> <https://example.org/vocab/label> \"row \\\"{idx}\\\" value with some prose\" . # comment {idx}",
        );
    }
    out
}

fn bench_tokenize(c: &mut Criterion) {
    let text = turtle_fixture(ROWS);
    let mut group = c.benchmark_group("tokenize");
    group.throughput(Throughput::Bytes(text.len() as u64));
    // The Turtle entry (bare `/` in PN_LOCAL) — the codec-facing path.
    group.bench_function("turtle_4k", |bencher| {
        bencher.iter(|| {
            let toks = tokenize_turtle(black_box(&text)).expect("tokenize");
            black_box(toks);
        });
    });
    // The SPARQL entry over the same bytes, for the query-stack path.
    group.bench_function("sparql_4k", |bencher| {
        bencher.iter(|| {
            let toks = tokenize(black_box(&text)).expect("tokenize");
            black_box(toks);
        });
    });
    group.finish();
}

criterion_group!(benches, bench_tokenize);
criterion_main!(benches);
