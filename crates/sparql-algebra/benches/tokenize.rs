// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! Tokenizer hot-path benchmark.
//!
//! Report-only, `cargo bench -p purrdf-sparql-algebra --bench tokenize`. The
//! fixture is a large Turtle-shaped document that exercises exactly the scans the
//! byte-cursor lexer accelerates: long `IRIREF` bodies (memchr `>`), string-literal
//! bodies (memchr2 `"`/`\`), and `#` comment tails (memchr `\n`) — plus the removal
//! of the former `char_indices().collect()` full-input materialization.
//!
//! `tokenize/modifier_query` is a SPARQL query dense in the token shapes the
//! byte-lookahead and case-fold changes touch: `GROUP BY`/`HAVING`/`ORDER BY`
//! continuation lists (the keyword-terminator peeks), decimal/exponent numbers,
//! long and short string literals, prefixed names, and `@en--ltr` literals.

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use purrdf_sparql_algebra::lexer::{tokenize, tokenize_turtle};

const ROWS: usize = 4_000;
const MODIFIER_BLOCKS: usize = 500;

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

/// A SPARQL query with `GROUP BY`/`HAVING`/`ORDER BY` continuation lists, numeric
/// literals with `.`/exponents, short and long strings, and RDF 1.2 `@lang--dir`
/// literals — repeated `blocks` times as UNION branches so the lookahead-heavy
/// token shapes dominate.
fn modifier_query_fixture(blocks: usize) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(blocks * 700);
    out.push_str(
        "PREFIX ex: <https://example.org/vocab/>\nSELECT ?s (COUNT(?o) AS ?n) (SUM(?v) AS ?total) WHERE {\n",
    );
    for idx in 0..blocks {
        let _ = writeln!(
            out,
            "  {{ ?s ex:p{idx} ?o ; ex:v ?v ; ex:label \"label {idx}\"@en--ltr ; ex:note \"\"\"long\nnote {idx}\"\"\"@ar--rtl ; ex:tag 'x{idx}'@fr--LTR .\n    FILTER(?v > {idx}.5 && ?v < 1.5e{idx} && ?v != .25 && STRLEN(STR(?o)) > 3) }}\n  UNION",
        );
    }
    out.push_str("  { ?s ex:p ?o ; ex:v ?v . _:b ex:q ?o . }\n}\n");
    out.push_str("GROUP BY ?s STR(?o) LCASE(STR(?s)) ex:f(?v) DATATYPE(?v)\n");
    out.push_str("HAVING (COUNT(?o) > 1) SUM(?v) ex:g(?v) BOUND(?s)\n");
    out.push_str("ORDER BY ?s DESC(?n) STR(?s) ex:h(?total) ASC(?total)\n");
    out.push_str("LIMIT 100 OFFSET 10\n");
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

/// Solution-modifier lists, numeric lookahead, quoted forms, `@lang--dir`.
fn bench_tokenize_modifiers(c: &mut Criterion) {
    let query = modifier_query_fixture(MODIFIER_BLOCKS);
    let mut group = c.benchmark_group("tokenize");
    group.throughput(Throughput::Bytes(query.len() as u64));
    group.bench_function("modifier_query", |bencher| {
        bencher.iter(|| {
            let toks = tokenize(black_box(&query)).expect("tokenize");
            black_box(toks);
        });
    });
    group.finish();
}

criterion_group!(benches, bench_tokenize, bench_tokenize_modifiers);
criterion_main!(benches);
