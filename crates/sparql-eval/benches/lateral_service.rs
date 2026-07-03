// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! LATERAL variable-endpoint SERVICE benchmark.
//!
//! `SERVICE ?g` (a variable endpoint) is evaluated as a LATERAL join: the left
//! pattern binds `?g` per row, and the SERVICE is re-evaluated (substitute +
//! forward) once per distinct left binding. This is inherently O(N) forwards in
//! the number of left rows, versus the O(1) single forward of a fixed-IRI
//! `SERVICE <ep>` (a plain Join). This bench runs both shapes over an offline
//! [`LocalRemoteQuerySource`] at two scales so the per-left-row substitute+forward
//! cost of the LATERAL path — and its linear scaling — is **measured**, and any
//! future endpoint-grouping optimization has a baseline to beat.
//!
//! Report-only, `make bench` lane only — excluded from `make check`.

use std::sync::Arc;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral};
use purrdf_sparql_algebra::SparqlParser;
use purrdf_sparql_eval::{evaluate_query, EvalCtx, LocalRemoteQuerySource, RemoteQuerySource};

const ENDPOINT_BASE: &str = "http://ex/ep";

/// Local graph: `:row{i} :endpoint <http://ex/ep{i}>` for i in 0..n — so the left
/// pattern `?x :endpoint ?g` yields n rows, each binding `?g` to a distinct endpoint.
fn local_dataset(n: usize) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let endpoint = b.intern_iri("http://ex/endpoint");
    for i in 0..n {
        let row = b.intern_iri(&format!("http://ex/row{i}"));
        let ep = b.intern_iri(&format!("{ENDPOINT_BASE}{i}"));
        b.push_quad(row, endpoint, ep, None);
    }
    b.freeze().expect("freeze local")
}

/// One endpoint graph: `:s :name "ep{i}"`.
fn endpoint_dataset(i: usize) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let name = b.intern_iri("http://ex/name");
    let s = b.intern_iri("http://ex/s");
    let lit = b.intern_literal(RdfLiteral::simple(format!("ep{i}")));
    b.push_quad(s, name, lit, None);
    b.freeze().expect("freeze endpoint")
}

/// A source registering n distinct endpoints `<http://ex/ep{i}>`.
fn source(n: usize) -> LocalRemoteQuerySource {
    let mut src = LocalRemoteQuerySource::new();
    for i in 0..n {
        src = src.with_endpoint(format!("{ENDPOINT_BASE}{i}"), endpoint_dataset(i));
    }
    src
}

fn run(ds: &Arc<RdfDataset>, src: &(dyn RemoteQuerySource + Sync), query: &str) -> usize {
    let parsed = SparqlParser::new().parse_query(query).expect("parse");
    let mut ctx = EvalCtx::new(ds).with_remote(src);
    match evaluate_query(&parsed, &mut ctx).expect("eval") {
        purrdf_sparql_eval::Outcome::Solutions(seq) => seq.len(),
        _ => 0,
    }
}

fn bench_lateral_service(c: &mut Criterion) {
    let mut group = c.benchmark_group("lateral_service");
    for &n in &[16usize, 256] {
        let ds = local_dataset(n);
        let src = source(n);

        // LATERAL path: `SERVICE ?g` re-evaluated once per left row (n forwards).
        let lateral = "SELECT * WHERE { ?x <http://ex/endpoint> ?g . \
                       SERVICE ?g { ?s <http://ex/name> ?name } }";
        group.bench_with_input(BenchmarkId::new("variable_endpoint", n), &n, |bch, _| {
            bch.iter(|| run(&ds, &src, lateral));
        });

        // Join baseline: a fixed `SERVICE <ep0>` — one forward regardless of n.
        let fixed = format!(
            "SELECT * WHERE {{ ?x <http://ex/endpoint> ?g . \
             SERVICE <{ENDPOINT_BASE}0> {{ ?s <http://ex/name> ?name }} }}"
        );
        group.bench_with_input(BenchmarkId::new("fixed_endpoint", n), &n, |bch, _| {
            bch.iter(|| run(&ds, &src, &fixed));
        });
    }
    group.finish();
}

criterion_group!(benches, bench_lateral_service);
criterion_main!(benches);
