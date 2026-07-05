// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Value-token-dense tokenizer benchmark.
//!
//! Report-only, `cargo bench -p purrdf-sparql-algebra --bench tokenize_values`.
//! Where `tokenize.rs` stresses the byte scans (long IRIREF bodies, string
//! bodies, comment tails), THIS fixture is dense in the VALUE tokens that the
//! zero-copy `Token<'a>` change accelerates: variables, prefixed names,
//! integers, and short unescaped string literals. Each of these now borrows a
//! `&str`/`Cow::Borrowed` slice of the source instead of allocating an owned
//! `String` per token, so this bench measures exactly the allocation the change
//! removes on the common no-escape path.

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use purrdf_sparql_algebra::lexer::tokenize;

const ROWS: usize = 4_000;

/// A SPARQL-shaped fixture whose body is dense in value tokens: many `?var`
/// variables, `ex:local` prefixed names, bare integers, and short string
/// literals WITHOUT escapes (so the lexer takes the `Cow::Borrowed` fast path).
fn value_fixture(rows: usize) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(rows * 96);
    out.push_str("PREFIX ex: <http://example.org/ns#>\nSELECT * WHERE {\n");
    for idx in 0..rows {
        // 4 value tokens per line: a variable, a prefixed name, an integer, and
        // a short unescaped string literal — all zero-copy borrow candidates.
        let _ = writeln!(
            out,
            "  ?s{idx} ex:p{idx} {idx} . FILTER(?s{idx} = \"v{idx}\")",
        );
    }
    out.push('}');
    out
}

fn bench_tokenize_values(c: &mut Criterion) {
    let text = value_fixture(ROWS);
    let mut group = c.benchmark_group("tokenize_values");
    group.throughput(Throughput::Bytes(text.len() as u64));
    group.bench_function("sparql_values_4k", |bencher| {
        bencher.iter(|| {
            let toks = tokenize(black_box(&text)).expect("tokenize");
            black_box(toks);
        });
    });
    group.finish();
}

criterion_group!(benches, bench_tokenize_values);
criterion_main!(benches);
