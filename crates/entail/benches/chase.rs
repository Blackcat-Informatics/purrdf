// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! Forward-materialization chase benchmark.
//!
//! Builds a synthetic subclass chain `C0 ⊑ C1 ⊑ … ⊑ C{n}` with one instance per
//! class, then materializes the RDFS closure. The subClassOf-transitivity +
//! instance-typing rules produce O(n²) inferred triples, so this makes the
//! semi-naive fixpoint cost visible to regression tracking. Report-only.
//!
//! # The multi-graph lane
//!
//! `rdfs_dataset` closes the SAME terminology with the instances spread over `g`
//! named graphs — the layout PurRDF's defined dataset semantics exists for. Each
//! named graph is closed against the union of itself and the default graph, so the
//! run is `1 + g` evaluations of one program and the cost is expected to grow with
//! `g`. It is measured rather than asserted: this bench exists so a later change to
//! the per-graph seeding has a number to move, not so a speedup can be claimed.

use std::sync::Arc;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

use purrdf_core::{RdfDataset, RdfDatasetBuilder};
use purrdf_entail::{Regime, materialize};

const SUBCLASSOF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
const TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";

/// The fixture namespace. `example.org` per the project rule: a bench mints no
/// vocabulary of its own, and a reserved-for-documentation authority is the only
/// one it may put in a term.
const EX: &str = "http://example.org/";

/// `C{i} subClassOf C{i+1}` for i in 0..n, plus `x{i} a C{i}`.
fn hierarchy(n: usize) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let sco = b.intern_iri(SUBCLASSOF);
    let ty = b.intern_iri(TYPE);
    for i in 0..n {
        let ci = b.intern_iri(&format!("{EX}C{i}"));
        let cj = b.intern_iri(&format!("{EX}C{}", i + 1));
        b.push_quad(ci, sco, cj, None);
        let xi = b.intern_iri(&format!("{EX}x{i}"));
        b.push_quad(xi, ty, ci, None);
    }
    b.freeze().expect("freeze")
}

/// The same terminology in the DEFAULT graph, with the `n` instances dealt round-robin
/// into `graphs` named graphs.
///
/// The real-world layout, and the one the defined dataset semantics is for: schema in the
/// default graph, instances in named graphs. `graphs == 0` is the single-graph control —
/// every instance in the default graph — so the two ends of the sweep are directly
/// comparable.
fn dataset_hierarchy(n: usize, graphs: usize) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let sco = b.intern_iri(SUBCLASSOF);
    let ty = b.intern_iri(TYPE);
    let names: Vec<_> = (0..graphs)
        .map(|g| b.intern_iri(&format!("{EX}g{g}")))
        .collect();
    for i in 0..n {
        let ci = b.intern_iri(&format!("{EX}C{i}"));
        let cj = b.intern_iri(&format!("{EX}C{}", i + 1));
        b.push_quad(ci, sco, cj, None);
        let xi = b.intern_iri(&format!("{EX}x{i}"));
        let g = names.get(i % graphs.max(1)).copied();
        b.push_quad(xi, ty, ci, g);
    }
    b.freeze().expect("freeze")
}

fn bench_chase(c: &mut Criterion) {
    let mut group = c.benchmark_group("rdfs_chase");
    for &n in &[16usize, 64] {
        let ds = hierarchy(n);
        group.bench_with_input(BenchmarkId::from_parameter(n), &ds, |bch, ds| {
            bch.iter(|| materialize(ds, Regime::Rdfs).expect("materialize"));
        });
    }
    group.finish();
}

/// The per-graph closure's cost as the graph COUNT grows, with the work held fixed.
fn bench_dataset(c: &mut Criterion) {
    let mut group = c.benchmark_group("rdfs_dataset");
    for &graphs in &[0usize, 1, 4, 16] {
        let ds = dataset_hierarchy(32, graphs);
        group.bench_with_input(BenchmarkId::from_parameter(graphs), &ds, |bch, ds| {
            bch.iter(|| materialize(ds, Regime::Rdfs).expect("materialize"));
        });
    }
    group.finish();
}

criterion_group!(benches, bench_chase, bench_dataset);
criterion_main!(benches);
