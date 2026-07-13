// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! End-to-end latency harness for the succinct `pack` codec:
//! [`PackBuilder::build_bytes`] (encode),
//! [`PackView::from_bytes`] (open), [`DatasetView::quads_for_pattern`] over a
//! [`PackView`] for a few representative pattern shapes, and [`verify_pack`] (the
//! certified-projection RDFC-1.0 recompute). Report-only — no timing/speedup
//! assertion, matching this workspace's bench discipline (see
//! `crates/rdf-core/benches/pack_bits.rs` and `ir_layout.rs`); criterion's stdout
//! summary is the report.

use std::sync::Arc;

use criterion::{Criterion, criterion_group, criterion_main};
use purrdf_core::{
    BlankScope, DatasetView, GraphMatch, PackBuilder, PackView, RdfDataset, RdfDatasetBuilder,
    RdfLiteral, verify_pack,
};

/// Number of subject "rows" generated. Each row emits four quads (see
/// [`build_dataset`]), so the frozen dataset — and the pack built from it — is a
/// few thousand quads: large enough that encode/open/pattern-query costs are
/// measurable, small enough that `cargo bench -- --test` stays fast.
const ROWS: u32 = 500;

/// Build a representative, deterministic (no RNG) `RdfDataset`: many distinct
/// subjects sharing one predicate (`p`), a blank node per row, a language-tagged
/// literal in a named graph, a typed literal, and every row's subject also pointing
/// at ONE shared object (`common`). This shape gives each of the four benched
/// pattern queries a different, non-trivial cardinality:
///
/// - subject-bound (`s0`, `p`, `_`) — a handful of matches (one subject's rows).
/// - predicate-bound (`_`, `p`, `_`) — nearly every quad (the shared predicate).
/// - object-bound (`_`, `_`, `common`) — exactly `ROWS` matches (the shared object).
/// - full scan (`_`, `_`, `_`) — every quad.
fn build_dataset() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri("http://example.org/p");
    let g = b.intern_iri("http://example.org/g");
    let common = b.intern_iri("http://example.org/common");

    for n in 0..ROWS {
        let s = b.intern_iri(&format!("http://example.org/s{n}"));
        let bnode = b.intern_blank(&format!("b{n}"), BlankScope(n % 4));
        let lit = b.intern_literal(RdfLiteral::language_tagged(format!("value {n}"), "en"));
        let typed = b.intern_literal(RdfLiteral::typed(
            format!("{n}"),
            "http://www.w3.org/2001/XMLSchema#integer",
        ));

        b.push_quad(s, p, bnode, None);
        b.push_quad(s, p, lit, Some(g));
        b.push_quad(bnode, p, typed, None);
        b.push_quad(s, p, common, None);
    }

    b.freeze()
        .expect("representative dataset is structurally valid")
}

/// Encode: [`PackBuilder::build_bytes`] over the representative dataset.
fn bench_build_bytes(c: &mut Criterion) {
    let ds = build_dataset();
    let mut group = c.benchmark_group("pack_query_build_bytes");
    group.bench_function("build_bytes", |b| {
        b.iter(|| {
            std::hint::black_box(
                PackBuilder::build_bytes(&ds).expect("representative dataset packs"),
            )
        });
    });
    group.finish();
}

/// Open: [`PackView::from_bytes`] over already-built pack bytes (magic/version/
/// section-digest verification plus dictionary decode).
fn bench_from_bytes(c: &mut Criterion) {
    let ds = build_dataset();
    let bytes = PackBuilder::build_bytes(&ds).expect("representative dataset packs");
    let mut group = c.benchmark_group("pack_query_from_bytes");
    group.bench_function("from_bytes", |b| {
        b.iter(|| std::hint::black_box(PackView::from_bytes(&bytes).expect("pack opens")));
    });
    group.finish();
}

/// Query the compressed form: `quads_for_pattern`, iterated to completion, over a
/// warm [`PackView`], for the four representative shapes [`build_dataset`] sets up.
fn bench_quads_for_pattern(c: &mut Criterion) {
    let ds = build_dataset();
    let bytes = PackBuilder::build_bytes(&ds).expect("representative dataset packs");
    let pack = PackView::from_bytes(&bytes).expect("pack opens");

    let p_id = pack
        .term_id_by_value(&purrdf_core::TermValue::iri("http://example.org/p"))
        .expect("predicate interned");
    let s0_id = pack
        .term_id_by_value(&purrdf_core::TermValue::iri("http://example.org/s0"))
        .expect("s0 interned");
    let common_id = pack
        .term_id_by_value(&purrdf_core::TermValue::iri("http://example.org/common"))
        .expect("common object interned");

    let mut group = c.benchmark_group("pack_query_pattern");

    group.bench_function("subject_bound", |b| {
        b.iter(|| {
            std::hint::black_box(
                pack.quads_for_pattern(Some(s0_id), None, None, GraphMatch::Any)
                    .count(),
            )
        });
    });
    group.bench_function("predicate_bound", |b| {
        b.iter(|| {
            std::hint::black_box(
                pack.quads_for_pattern(None, Some(p_id), None, GraphMatch::Any)
                    .count(),
            )
        });
    });
    group.bench_function("object_bound", |b| {
        b.iter(|| {
            std::hint::black_box(
                pack.quads_for_pattern(None, None, Some(common_id), GraphMatch::Any)
                    .count(),
            )
        });
    });
    group.bench_function("full_scan", |b| {
        b.iter(|| {
            std::hint::black_box(
                pack.quads_for_pattern(None, None, None, GraphMatch::Any)
                    .count(),
            )
        });
    });

    group.finish();
}

/// The certified-projection verifier: [`verify_pack`]'s independent RDFC-1.0
/// reconstruct-and-recompute over already-built pack bytes.
fn bench_verify_pack(c: &mut Criterion) {
    let ds = build_dataset();
    let bytes = PackBuilder::build_bytes(&ds).expect("representative dataset packs");
    let mut group = c.benchmark_group("pack_query_verify");
    group.bench_function("verify_pack", |b| {
        b.iter(|| std::hint::black_box(verify_pack(&bytes).expect("pack verifies")));
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_build_bytes,
    bench_from_bytes,
    bench_quads_for_pattern,
    bench_verify_pack
);
criterion_main!(benches);
