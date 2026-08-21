// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! Correlated `LATERAL` substitution benchmark (gap R9/G10, the per-row
//! Values-Insertion allocation path in `purrdf-sparql-eval`'s internal
//! `expr` module).
//!
//! `LATERAL`'s right operand is re-evaluated once per left row
//! (`eval_correlated`): each row builds a `SubstitutionRow`
//! (`outer_bindings_for_substitution`) and walks the right operand's
//! pattern tree, joining a one-row `VALUES` block onto every `Bgp`/`Path`
//! leaf and expression-bearing node that needs the outer binding
//! (`substitute_pattern`'s Values Insertion). The right operand here is
//! itself a sub-`SELECT`, so the walk also crosses a `Project` boundary per
//! row (`SubstitutionRow::narrow_to`).
//!
//! This measures that per-row substitution cost directly, at two left-row
//! scales, so a future change to the substitution path (or to the `Arc<str>`
//! term-text storage the row's clones rely on being cheap — see
//! `purrdf_sparql_algebra::NamedNode`'s doc) has a baseline to compare
//! against. Report-only: no timing assertion, no speedup claim — `make
//! bench` lane only, excluded from `make check`.

use std::sync::Arc;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

use purrdf_core::{RdfDataset, RdfDatasetBuilder};
use purrdf_sparql_algebra::SparqlParser;
use purrdf_sparql_eval::{EvalCtx, evaluate_query};

/// `:row{i} <http://ex/val> <urn:v{i}>` for i in `0..n`, plus, for each `v{i}`, a
/// small fan-out `<urn:v{i}> <http://ex/rel> <urn:w{i}-{0,1,2}>` (three rows per
/// left row) so the substituted `Bgp` leaf's `VALUES` join actually narrows a
/// multi-row scan rather than probing an already-singleton pattern.
fn dataset(n: usize) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let val = b.intern_iri("http://ex/val");
    let rel = b.intern_iri("http://ex/rel");
    for i in 0..n {
        let row = b.intern_iri(&format!("http://ex/row{i}"));
        let v = b.intern_iri(&format!("urn:v{i}"));
        b.push_quad(row, val, v, None);
        for k in 0..3 {
            let w = b.intern_iri(&format!("urn:w{i}-{k}"));
            b.push_quad(v, rel, w, None);
        }
    }
    b.freeze().expect("freeze")
}

/// `LATERAL`'s right operand is a sub-`SELECT`, so the substitution walk crosses
/// a `Project` boundary (`SubstitutionRow::narrow_to`) before reaching the `Bgp`
/// leaf that Values Insertion joins on `?v`.
const LATERAL_SUBSELECT_QUERY: &str = "SELECT * WHERE { \
    ?x <http://ex/val> ?v . \
    LATERAL { SELECT ?v ?w WHERE { ?v <http://ex/rel> ?w } } \
}";

/// The non-correlated equivalent (a plain `Join`, no per-row substitution) over
/// the SAME data and SAME result shape — not a speedup baseline (this bench
/// asserts none), but a fixed point that lets a reader judge whether the
/// correlated path's report-only numbers below are in the same neighborhood.
const JOIN_QUERY: &str = "SELECT * WHERE { \
    ?x <http://ex/val> ?v . \
    ?v <http://ex/rel> ?w \
}";

fn run(ds: &RdfDataset, query: &str) -> usize {
    let parsed = SparqlParser::new().parse_query(query).expect("parse");
    let mut ctx = EvalCtx::new(ds);
    match evaluate_query(&parsed, &mut ctx).expect("eval") {
        purrdf_sparql_eval::Outcome::Solutions(seq) => seq.len(),
        _ => 0,
    }
}

fn bench_lateral_substitution(c: &mut Criterion) {
    let mut group = c.benchmark_group("lateral_substitution");
    for &n in &[100usize, 1_000] {
        let ds = dataset(n);

        group.bench_with_input(BenchmarkId::new("lateral_subselect", n), &n, |bch, _| {
            bch.iter(|| run(&ds, LATERAL_SUBSELECT_QUERY));
        });
        group.bench_with_input(BenchmarkId::new("plain_join", n), &n, |bch, _| {
            bch.iter(|| run(&ds, JOIN_QUERY));
        });
    }
    group.finish();
}

criterion_group!(benches, bench_lateral_substitution);
criterion_main!(benches);
