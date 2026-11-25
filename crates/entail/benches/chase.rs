// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API, so the workspace `missing_docs` lint is
// not asked of their items.
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
//!
//! The simple entailment lane measures endpoint selection against a growing
//! predicate bucket, with a broad enumeration over the same closure as a control.

use std::sync::Arc;

use purrdf_testkit::bench::{Bench, BenchmarkId, bench_group, bench_main};

use purrdf_core::{RdfDataset, RdfDatasetBuilder, TermValue};
use purrdf_entail::{
    ImportMap, Materialization, QNode, QTriple, Regime, certain_answers, materialize,
};

use purrdf_iri::vocab::rdf::TYPE;
use purrdf_iri::vocab::rdfs::SUB_CLASS_OF as SUBCLASSOF;

/// The fixture namespace. `example.org` per the project rule: a bench mints no
/// vocabulary of its own, and a reserved-for-documentation authority is the only
/// one it may put in a term.
const EX: &str = "http://example.org/";

/// `C{i} subClassOf C{i+1}` for i in 0..n, plus `x{i} a C{i}`.
fn hierarchy(n: usize) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let sco = b.intern_iri(SUBCLASSOF).unwrap();
    let ty = b.intern_iri(TYPE).unwrap();
    for i in 0..n {
        let ci = b.intern_iri(&format!("{EX}C{i}")).unwrap();
        let cj = b.intern_iri(&format!("{EX}C{}", i + 1)).unwrap();
        b.push_quad(ci, sco, cj, None).unwrap();
        let xi = b.intern_iri(&format!("{EX}x{i}")).unwrap();
        b.push_quad(xi, ty, ci, None).unwrap();
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
    let sco = b.intern_iri(SUBCLASSOF).unwrap();
    let ty = b.intern_iri(TYPE).unwrap();
    let names: Vec<_> = (0..graphs)
        .map(|g| b.intern_iri(&format!("{EX}g{g}")).unwrap())
        .collect();
    for i in 0..n {
        let ci = b.intern_iri(&format!("{EX}C{i}")).unwrap();
        let cj = b.intern_iri(&format!("{EX}C{}", i + 1)).unwrap();
        b.push_quad(ci, sco, cj, None).unwrap();
        let xi = b.intern_iri(&format!("{EX}x{i}")).unwrap();
        let g = names.get(i % graphs.max(1)).copied();
        b.push_quad(xi, ty, ci, g).unwrap();
    }
    b.freeze().expect("freeze")
}

fn bench_chase(c: &mut Bench) {
    let mut group = c.benchmark_group("rdfs_chase");
    for &n in &[16usize, 64] {
        let ds = hierarchy(n);
        group.bench_with_input(BenchmarkId::from_parameter(n), &ds, |bch, ds| {
            bch.iter(|| materialize(ds, Materialization::Rdfs).expect("materialize"));
        });
    }
    group.finish();
}

/// The per-graph closure's cost as the graph COUNT grows, with the work held fixed.
fn bench_dataset(c: &mut Bench) {
    let mut group = c.benchmark_group("rdfs_dataset");
    for &graphs in &[0usize, 1, 4, 16] {
        let ds = dataset_hierarchy(32, graphs);
        group.bench_with_input(BenchmarkId::from_parameter(graphs), &ds, |bch, ds| {
            bch.iter(|| materialize(ds, Materialization::Rdfs).expect("materialize"));
        });
    }
    group.finish();
}

/// Vary the number of triples sharing a predicate. The ground subject
/// has one answer, while the open subject ranges over the whole bucket.
fn bench_certain_answers(c: &mut Bench) {
    let mut group = c.benchmark_group("simple_certain_answers");
    let predicate = format!("{EX}predicate");
    let imports = ImportMap::default();
    for n in [1_024, 8_192] {
        let mut b = RdfDatasetBuilder::new();
        let p = b.intern_iri(&predicate).expect("predicate");
        for i in 0..n {
            let s = b.intern_iri(&format!("{EX}s{i}")).expect("subject");
            let o = b.intern_iri(&format!("{EX}o{i}")).expect("object");
            b.push_quad(s, p, o, None).expect("triple");
        }
        let ds = b.freeze().expect("freeze");
        for (name, subject) in [
            (
                "ground",
                QNode::Term(TermValue::Iri(format!("{EX}s{}", n - 1))),
            ),
            ("open", QNode::Var("s".to_owned())),
        ] {
            let pattern = [QTriple {
                s: subject,
                p: QNode::Term(TermValue::iri(&predicate)),
                o: QNode::Var("o".to_owned()),
            }];
            group.bench_with_input(BenchmarkId::new(name, n), &pattern, |bch, pattern| {
                bch.iter(|| {
                    certain_answers(&ds, pattern, Regime::Simple, &imports).expect("answers")
                });
            });
        }
    }
    group.finish();
}

bench_group!(benches, bench_chase, bench_dataset, bench_certain_answers);
bench_main!(benches);
