// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API, so the workspace `missing_docs` lint is
// not asked of their items.
#![allow(missing_docs)]

//! The whole-tree walks over a parsed query algebra: `Clone`, `Drop`, `==`, `Hash`,
//! `Debug` and the serializer, over deep trees and wide ones.
//!
//! Trees, each parsed once before sampling:
//! - `deep/optional_spine/<n>` — `n` sibling `OPTIONAL` groups: a `LeftJoin` spine
//!   `n` levels tall, built by a loop in the parser.
//! - `deep/nested_calls/<n>` — `ABS(ABS(…?o…))` `n` calls deep in a `FILTER`, the
//!   calls nested through their argument lists.
//! - `deep/nested_groups/<n>` — `{ { … } }` `n` groups deep around a `FILTER`.
//! - `wide/bgp_4096` — one basic graph pattern of 4 096 triple patterns.
//! - `wide/union_1024` — 1 024 `UNION` arms.
//! - `wide/values_4096` — a `VALUES` block of 4 096 two-column rows.
//!
//! Walks, per tree:
//! - `clone` — `Query::clone`, the copy dropped outside the sample;
//! - `drop` — dropping a copy made outside the sample;
//! - `eq` — `==` between the tree and an equal copy, which walks every node;
//! - `hash` — `Hash` into a `FixedHasher`, the same hasher on every revision so
//!   only the walk moves;
//! - `debug` — `format!("{:?}")`;
//! - `serialize` — `pattern_to_select_query` over the pattern under the projection.
//!
//! [`DEEP_LEVELS`] is the list of deep heights; extending it is one line. Each tree is
//! parsed once, before sampling, and every walk runs on the bench thread: neither the
//! parser nor any walk recurses. All IRIs are `example.org` fixtures.
//!
//! Report-only, `cargo bench -p purrdf-sparql-algebra --bench algebra_walks` (the
//! `make bench` lane) — excluded from `make check`. No timing is asserted.

use std::fmt::Write as _;
use std::hash::{Hash, Hasher};

use purrdf_sparql_algebra::{GraphPattern, Query, SparqlParser, pattern_to_select_query};
use purrdf_testkit::bench::{BatchSize, Bench, bench_group, bench_main, black_box};

const EX: &str = "http://example.org/";

/// The heights every deep tree is measured at.
const DEEP_LEVELS: &[usize] = &[64, 512, 100_000];

/// `open` written `n` times around `core`, closed by `close` written `n` times.
fn nested(open: &str, core: &str, close: &str, n: usize) -> String {
    format!("{}{core}{}", open.repeat(n), close.repeat(n))
}

/// The deep trees at `n` levels, as `(bench id, query text)`.
fn deep_queries(n: usize) -> [(String, String); 3] {
    [
        (
            format!("deep/optional_spine/{n}"),
            format!(
                "SELECT ?s WHERE {{ ?s <{EX}p> ?o {} }}",
                format!("OPTIONAL {{ ?s <{EX}q> ?z }} ").repeat(n)
            ),
        ),
        (
            format!("deep/nested_calls/{n}"),
            format!(
                "SELECT ?s WHERE {{ ?s <{EX}p> ?o FILTER({} = 1) }}",
                nested("ABS(", "?o - 3", ")", n)
            ),
        ),
        (
            format!("deep/nested_groups/{n}"),
            format!(
                "SELECT ?s WHERE {{ {} }}",
                nested("{ ", &format!("?s <{EX}p> ?o FILTER(?o = 1)"), " }", n)
            ),
        ),
    ]
}

/// The wide trees, as `(bench id, query text)`.
fn wide_queries() -> [(String, String); 3] {
    let mut bgp = String::from("SELECT * WHERE {");
    for i in 0..4096 {
        let _ = write!(bgp, " ?s <{EX}p{i}> ?o{i} .");
    }
    bgp.push_str(" }");
    let union = (0..1024)
        .map(|i| format!("{{ ?s <{EX}p{i}> ?o }}"))
        .collect::<Vec<_>>()
        .join(" UNION ");
    let mut values = String::from("SELECT * WHERE { ?s <");
    values.push_str(EX);
    values.push_str("p> ?o VALUES (?s ?o) {");
    for i in 0..4096 {
        let _ = write!(values, " (<{EX}s{i}> {i})");
    }
    values.push_str(" } }");
    [
        ("wide/bgp_4096".to_owned(), bgp),
        (
            "wide/union_1024".to_owned(),
            format!("SELECT ?s ?o WHERE {{ {union} }}"),
        ),
        ("wide/values_4096".to_owned(), values),
    ]
}

/// The pattern the serializer renders: the one under the `SELECT`'s projection.
fn serialized_pattern(query: &Query) -> &GraphPattern {
    let Query::Select { pattern, .. } = query else {
        panic!("every tree here is a SELECT");
    };
    match pattern {
        GraphPattern::Project { inner, .. } => inner,
        other => other,
    }
}

fn hash_of(query: &Query) -> u64 {
    let mut hasher = purrdf_hash::fixed::FixedHasher::default();
    query.hash(&mut hasher);
    hasher.finish()
}

/// `text`, parsed.
fn parse(id: &str, text: &str) -> Query {
    SparqlParser::new()
        .parse_query(text)
        .unwrap_or_else(|error| panic!("{id} parses: {error}"))
}

fn bench_tree(c: &mut Bench, id: &str, text: &str) {
    let tree = parse(id, text);
    let copy = tree.clone();
    assert!(tree == copy, "{id}: a clone equals its original");
    assert_eq!(
        hash_of(&tree),
        hash_of(&copy),
        "{id}: equal trees hash equal"
    );
    assert!(
        !pattern_to_select_query(serialized_pattern(&tree)).is_empty(),
        "{id}: the serializer renders the tree"
    );

    let mut group = c.benchmark_group(format!("algebra_walks/{id}"));
    group.sample_size(20);
    group.bench_function("clone", |b| {
        b.iter_with_large_drop(|| tree.clone());
    });
    group.bench_function("drop", |b| {
        b.iter_batched(|| tree.clone(), drop, BatchSize::SmallInput);
    });
    group.bench_function("eq", |b| {
        b.iter(|| black_box(black_box(&tree) == black_box(&copy)));
    });
    group.bench_function("hash", |b| {
        b.iter(|| black_box(hash_of(black_box(&tree))));
    });
    group.bench_function("debug", |b| {
        b.iter(|| black_box(format!("{:?}", black_box(&tree))));
    });
    group.bench_function("serialize", |b| {
        b.iter(|| {
            black_box(pattern_to_select_query(serialized_pattern(black_box(
                &tree,
            ))))
        });
    });
    group.finish();
}

fn bench_algebra_walks(c: &mut Bench) {
    for &n in DEEP_LEVELS {
        for (id, text) in deep_queries(n) {
            bench_tree(c, &id, &text);
        }
    }
    for (id, text) in wide_queries() {
        bench_tree(c, &id, &text);
    }
}

bench_group!(benches, bench_algebra_walks);
bench_main!(benches);
