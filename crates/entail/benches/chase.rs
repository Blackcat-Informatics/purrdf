// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Forward-materialization chase benchmark.
//!
//! Builds a synthetic subclass chain `C0 ⊑ C1 ⊑ … ⊑ C{n}` with one instance per
//! class, then materializes the RDFS closure. The subClassOf-transitivity +
//! instance-typing rules produce O(n²) inferred triples, so this makes the
//! semi-naive fixpoint cost visible to regression tracking. Report-only.

use std::sync::Arc;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

use purrdf_core::{RdfDataset, RdfDatasetBuilder};
use purrdf_entail::{materialize, Regime};

const SUBCLASSOF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
const TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";

/// `C{i} subClassOf C{i+1}` for i in 0..n, plus `x{i} a C{i}`.
fn hierarchy(n: usize) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let sco = b.intern_iri(SUBCLASSOF);
    let ty = b.intern_iri(TYPE);
    for i in 0..n {
        let ci = b.intern_iri(&format!("http://ex/C{i}"));
        let cj = b.intern_iri(&format!("http://ex/C{}", i + 1));
        b.push_quad(ci, sco, cj, None);
        let xi = b.intern_iri(&format!("http://ex/x{i}"));
        b.push_quad(xi, ty, ci, None);
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

criterion_group!(benches, bench_chase);
criterion_main!(benches);
