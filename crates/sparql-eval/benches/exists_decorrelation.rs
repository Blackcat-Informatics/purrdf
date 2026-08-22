// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! `EXISTS` anti-join benchmark: naive per-row inner work vs the decorrelated path
//! that evaluates the inner pattern AND builds its probe index once per site.
//!
//! The `FILTER NOT EXISTS` shape tests its inner pattern once per outer row. The
//! decorrelated path (`EvalOptions::exists_memo`) evaluates the inner pattern once
//! and — crucially — builds the join/probe index over that result once, then
//! existence-probes each outer row against the reused index. The naive path
//! re-evaluates the inner *and* rebuilds its index on every outer row, which is
//! O(N · (inner_eval + |inner|)); the decorrelated path is
//! O(inner_eval + |inner| + N · probe). Each bench runs the SAME query and dataset
//! twice — memo off vs on — so the speedup is **measured, not asserted**.
//!
//! Three shapes, each over a synthetic dataset:
//!   - `trivial_inner` — single-row inner (|inner| = 1): isolates the inner-eval
//!     caching win; the index over one row is negligible.
//!   - `large_inner` — a wide inner result shared with the outer: rebuilding the
//!     inner index per outer row is the quadratic cost here, so this shape isolates
//!     the **index-reuse** win.
//!   - `unbound_scan` — outer rows that leave the shared variable unbound (a UNION
//!     branch), so each probe falls into the per-row compatibility scan. This makes
//!     the unbound-shared-column cliff visible to regression tracking — even with a
//!     reused index, an unbound probe column scans the full inner result.
//!   - `optional_inside_exists` — an `OPTIONAL` buried inside a `FILTER` inside the
//!     `EXISTS` inner (so the ENF `LeftJoin`-erasure law does not fire — the
//!     `LeftJoin` is off the emptiness-observed spine — and
//!     `crate::governor::soundness::probe_admissible` correctly refuses it): every
//!     outer row takes the per-row DEFINITION path. This isolates the
//!     **μ-restriction memo**'s win (`EvalOptions::exists_memo`, now shared by both
//!     evaluation strategies) rather than the probe path's index-reuse win the other
//!     three shapes measure: many outer rows share few distinct correlated-variable
//!     restrictions, so the memo turns `N` per-row substituted `LeftJoin`
//!     evaluations into `k`.
//!
//! Report-only, `make bench` lane only — excluded from `make check`.

use std::sync::Arc;

use criterion::{Criterion, criterion_group, criterion_main};

use purrdf_core::{RdfDataset, RdfDatasetBuilder};
use purrdf_sparql_algebra::SparqlParser;
use purrdf_sparql_eval::{EvalCtx, evaluate_query};

/// `:s{i} :knows :o{i}` for i in 0..n, plus `:o0 :member :club` so exactly one
/// subject survives the anti-join. N subjects → N outer rows for the EXISTS, and a
/// single-row inner result.
fn knows_dataset(n: usize) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let knows = b.intern_iri("http://ex/knows");
    let member = b.intern_iri("http://ex/member");
    let club = b.intern_iri("http://ex/club");
    let mut first_obj = None;
    for i in 0..n {
        let s = b.intern_iri(&format!("http://ex/s{i}"));
        let o = b.intern_iri(&format!("http://ex/o{i}"));
        b.push_quad(s, knows, o, None);
        if first_obj.is_none() {
            first_obj = Some(o);
        }
    }
    if let Some(o) = first_obj {
        b.push_quad(o, member, club, None);
    }
    b.freeze().expect("freeze")
}

/// `n_outer` subjects all `:knows :hub`, and `:hub :member :m{j}` for j in
/// 0..m_inner. The inner `{ ?o :member ?m }` therefore yields `m_inner` rows, all
/// keyed on `?o = :hub`, while the outer yields `n_outer` rows also keyed on
/// `?o = :hub` — so a per-row index rebuild is O(n_outer · m_inner) but a reused
/// index is O(m_inner + n_outer). Also adds `:s{i} :likes :z{i}` so a UNION can
/// produce outer rows that leave `?o` unbound (the scan-branch shape).
fn hub_dataset(n_outer: usize, m_inner: usize) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let knows = b.intern_iri("http://ex/knows");
    let likes = b.intern_iri("http://ex/likes");
    let member = b.intern_iri("http://ex/member");
    let hub = b.intern_iri("http://ex/hub");
    for i in 0..n_outer {
        let s = b.intern_iri(&format!("http://ex/s{i}"));
        let z = b.intern_iri(&format!("http://ex/z{i}"));
        b.push_quad(s, knows, hub, None);
        b.push_quad(s, likes, z, None);
    }
    for j in 0..m_inner {
        let m = b.intern_iri(&format!("http://ex/m{j}"));
        b.push_quad(hub, member, m, None);
    }
    b.freeze().expect("freeze")
}

/// `n_outer` subjects `:knows` cycling through `m_hubs` distinct objects (`?x`).
/// Each hub has its own `:p` value (so the correlated `EXISTS` inner's `Bgp{?x :p
/// ?w}` differs per hub) and, for every other hub, a `:q` edge (so the buried
/// `OPTIONAL` has real per-hub data to match or not match, rather than always
/// padding a no-op). `m_hubs` distinct hubs ⇒ `m_hubs` distinct correlated-variable
/// restrictions for the μ-restriction memo to collapse `n_outer` rows onto.
fn hub_optional_dataset(n_outer: usize, m_hubs: usize) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let knows = b.intern_iri("http://ex/knows");
    let p = b.intern_iri("http://ex/p");
    let q = b.intern_iri("http://ex/q");
    let mut hubs = Vec::with_capacity(m_hubs);
    for j in 0..m_hubs {
        let hub = b.intern_iri(&format!("http://ex/hub{j}"));
        let w = b.intern_iri(&format!("http://ex/w{j}"));
        b.push_quad(hub, p, w, None);
        if j % 2 == 0 {
            let y = b.intern_iri(&format!("http://ex/y{j}"));
            b.push_quad(hub, q, y, None);
        }
        hubs.push(hub);
    }
    for i in 0..n_outer {
        let s = b.intern_iri(&format!("http://ex/s{i}"));
        b.push_quad(s, knows, hubs[i % m_hubs], None);
    }
    b.freeze().expect("freeze")
}

/// Single-row inner: the outer `?o` appears only in the inner BGP triple position.
const TRIVIAL_QUERY: &str = "SELECT ?s ?o WHERE { ?s <http://ex/knows> ?o \
                             FILTER NOT EXISTS { ?o <http://ex/member> ?m } }";

/// Wide inner result shared on `?o` — the quadratic per-row-rebuild shape.
const LARGE_INNER_QUERY: &str = "SELECT ?s WHERE { ?s <http://ex/knows> ?o \
                                 FILTER NOT EXISTS { ?o <http://ex/member> ?m } }";

/// A UNION whose `:likes` branch leaves `?o` unbound, so half the outer rows probe
/// the inner with an unbound shared column (the per-row compatibility scan branch).
const UNBOUND_SCAN_QUERY: &str = "SELECT ?s WHERE { { ?s <http://ex/knows> ?o } UNION { ?s <http://ex/likes> ?z } \
     FILTER NOT EXISTS { ?o <http://ex/member> ?m } }";

/// An `OPTIONAL` buried inside a `FILTER` inside the `EXISTS` inner: the `LeftJoin`
/// is off the ENF spine (the enclosing `Filter` is not a spine-transparent wrapper),
/// so it survives normalization and `probe_admissible` correctly refuses it — every
/// outer row takes the per-row DEFINITION path, and the μ-restriction memo is what
/// keeps `n_outer` rows from paying `n_outer` separate substituted evaluations.
const OPTIONAL_INSIDE_EXISTS_QUERY: &str = "SELECT ?s WHERE { ?s <http://ex/knows> ?x \
     FILTER EXISTS { \
       ?x <http://ex/p> ?w \
       OPTIONAL { ?x <http://ex/q> ?y } \
       FILTER(?w != <http://ex/nobody>) \
     } }";

fn run(ds: &RdfDataset, query: &str, memo: bool) {
    let parsed = SparqlParser::new().parse_query(query).expect("parse");
    let mut ctx = EvalCtx::new(ds);
    ctx.options.exists_memo = memo;
    let outcome = evaluate_query(&parsed, &mut ctx).expect("eval");
    criterion::black_box(outcome);
}

/// Bench one query/dataset twice (memo off vs on) under `group_name`.
fn bench_pair(c: &mut Criterion, group_name: &str, ds: &RdfDataset, query: &str) {
    let mut group = c.benchmark_group(group_name);
    group.bench_function("naive_per_row_rebuild", |bencher| {
        bencher.iter(|| run(ds, query, false));
    });
    group.bench_function("decorrelated_reused_index", |bencher| {
        bencher.iter(|| run(ds, query, true));
    });
    group.finish();
}

fn bench_exists_decorrelation(c: &mut Criterion) {
    // Single-row inner: isolates inner-eval caching (index is trivial).
    bench_pair(
        c,
        "exists_trivial_inner_1k_rows",
        &knows_dataset(1_000),
        TRIVIAL_QUERY,
    );
    // Wide inner shared on the probe key: isolates the index-reuse win (the
    // quadratic per-row-rebuild anti-join shape). 1k outer rows × a 1k-row inner = a per-row rebuild of 1M vs a
    // single 1k build plus 1k O(1) probes.
    let hub = hub_dataset(1_000, 1_000);
    bench_pair(c, "exists_large_inner_antijoin", &hub, LARGE_INNER_QUERY);
    // Unbound shared column on half the outer rows: the scan-branch cliff. Reusing
    // the index still helps the bound half, but the unbound half scans the inner.
    bench_pair(c, "exists_unbound_scan_antijoin", &hub, UNBOUND_SCAN_QUERY);
    // OPTIONAL buried inside a FILTER inside EXISTS: the definition path's
    // μ-restriction memo win, not the probe path's index-reuse win — 1000 outer
    // rows over 20 distinct correlated-variable restrictions.
    let hub_optional = hub_optional_dataset(1_000, 20);
    bench_pair(
        c,
        "exists_optional_inside_exists_definition_memo",
        &hub_optional,
        OPTIONAL_INSIDE_EXISTS_QUERY,
    );
}

criterion_group!(benches, bench_exists_decorrelation);
criterion_main!(benches);
