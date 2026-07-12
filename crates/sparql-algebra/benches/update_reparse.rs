// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! Multi-operation UPDATE reparse benchmark.
//!
//! Wall-clock samples from the shared development host are not acceptance
//! evidence. `DELETE WHERE { … }` and the short-form `CONSTRUCT WHERE { … }`
//! both intentionally reparse the same braced block twice (once to mint an
//! independent set of RDF 1.2 synthetic reifier blanks, see the doc comments
//! at the `fork_block` call sites in `parser.rs`), which needs a forked
//! sub-parser over a CLONE of the block's tokens. That fork is bounded to the
//! braced block itself (`Parser::fork_block`) rather than the whole remaining
//! token stream, so a `;`-separated multi-operation UPDATE stays linear in the
//! total request size: this fixture's `DELETE WHERE` operation count is what
//! would have made the unbounded clone (`self.tokens[self.pos..]` through EOF
//! for every operation) quadratic.

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use purrdf_sparql_algebra::SparqlParser;

const DELETE_WHERE_OPS: usize = 200;
const CONSTRUCT_ROWS: usize = 2_000;

/// A `;`-separated UPDATE request made of many `DELETE WHERE { … }`
/// operations. Each operation's forked reparse should cost O(block), so total
/// parse cost should stay O(N) in `DELETE_WHERE_OPS`, not O(N²).
fn delete_where_fixture(ops: usize) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(ops * 96);
    out.push_str("PREFIX ex: <http://example.org/ns#>\n");
    for idx in 0..ops {
        let _ = writeln!(out, "DELETE WHERE {{ ex:s{idx} ex:p{idx} ?o{idx} }} ;");
    }
    out
}

/// A large short-form `CONSTRUCT WHERE { … }` template, whose reparse forks
/// once (not per-row) but still clones the whole braced block's tokens.
fn construct_where_fixture(rows: usize) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(rows * 64);
    out.push_str("PREFIX ex: <http://example.org/ns#>\nCONSTRUCT WHERE {\n");
    for idx in 0..rows {
        let _ = writeln!(out, "  ex:s{idx} ex:p{idx} ex:o{idx} .");
    }
    out.push('}');
    out
}

fn bench_delete_where(c: &mut Criterion) {
    let parser = SparqlParser::new();
    let delete_where_text = delete_where_fixture(DELETE_WHERE_OPS);
    let mut group = c.benchmark_group("update_reparse");
    group.throughput(Throughput::Bytes(delete_where_text.len() as u64));
    group.bench_function("delete_where_200_ops", |bencher| {
        bencher.iter(|| {
            let update = parser
                .parse_update(black_box(&delete_where_text))
                .expect("parse update");
            black_box(update);
        });
    });
    group.finish();
}

fn bench_construct_where(c: &mut Criterion) {
    let parser = SparqlParser::new();
    let construct_where_text = construct_where_fixture(CONSTRUCT_ROWS);
    let mut group = c.benchmark_group("construct_where_reparse");
    group.throughput(Throughput::Bytes(construct_where_text.len() as u64));
    group.bench_function("construct_where_2k_rows", |bencher| {
        bencher.iter(|| {
            let query = parser
                .parse_query(black_box(&construct_where_text))
                .expect("parse query");
            black_box(query);
        });
    });
    group.finish();
}

criterion_group!(benches, bench_delete_where, bench_construct_where);
criterion_main!(benches);
