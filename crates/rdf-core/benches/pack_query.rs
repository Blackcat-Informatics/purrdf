// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API, so the workspace `missing_docs` lint is
// not asked of their items.
#![allow(missing_docs)]

//! End-to-end latency harness for the succinct `pack` codec:
//! [`PackBuilder::build_bytes`] (encode),
//! [`PackView::from_bytes`] (open), [`DatasetView::quads_for_pattern`] over a
//! [`PackView`] for a few representative pattern shapes, [`verify_pack`] (the
//! certified-projection RDFC-1.0 recompute), and the named-graph surface: encode
//! and [`DatasetView::named_graphs`] over a many-graph dataset with and without
//! declaration-only graphs. Report-only — no timing/speedup
//! assertion, matching this workspace's bench discipline (see
//! `crates/rdf-core/benches/pack_bits.rs` and `ir_layout.rs`); the harness's stdout
//! summary is the report.

use std::sync::Arc;

use purrdf_core::ir::pack::dict::PackDict;
use purrdf_core::{
    BlankScope, DatasetView, GraphMatch, PackBuilder, PackView, RdfDataset, RdfDatasetBuilder,
    RdfLiteral, RdfTextDirection, restore_pack, verify_pack,
};
use purrdf_testkit::bench::{Bench, bench_group, bench_main};

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

/// Subject rows in the literal-heavy dictionary fixture. Each row interns five
/// distinct literal terms (see [`build_literal_heavy_dataset`]), so the dictionary
/// PFC-encodes on the order of ten thousand literal records per sample.
const DICT_ROWS: u32 = 2_000;

/// A dataset whose dictionary is dominated by literal terms of every shape the
/// canonical byte-record distinguishes — plain `xsd:string`, typed, language-
/// tagged, directional language-tagged, and long lexical forms — plus a quoted
/// triple term per row (whose `s`/`p`/`o` the closure step must resolve), so
/// [`PackDict::encode`]'s record encoder and closure worklist are both busy.
///
/// The triple term is asserted in the OBJECT position (`s q <<s p plain>>`): RDF
/// 1.2 admits a triple term only there, and the freeze gate refuses it as a
/// subject (`rdf-ir-triple-subject`). The fixture that shipped with the sweep
/// asserted it as the subject, so `freeze` panicked and the bench measured
/// nothing.
fn build_literal_heavy_dataset() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri("http://example.org/p");
    let q = b.intern_iri("http://example.org/q");

    for n in 0..DICT_ROWS {
        let s = b.intern_iri(&format!("http://example.org/s{n}"));
        let plain = b.intern_literal(RdfLiteral::simple(format!("plain value {n}")));
        let typed = b.intern_literal(RdfLiteral::typed(
            format!("{n}.5"),
            "http://www.w3.org/2001/XMLSchema#decimal",
        ));
        let tagged = b.intern_literal(RdfLiteral::language_tagged(
            format!("tagged value {n}"),
            "en-gb",
        ));
        let directional = b.intern_literal(RdfLiteral {
            lexical_form: format!("directional value {n}"),
            datatype: None,
            language: Some("ar".to_owned()),
            direction: Some(RdfTextDirection::Rtl),
        });
        let long = b.intern_literal(RdfLiteral::simple(format!(
            "a much longer lexical form sharing a long common prefix with its neighbours {n}"
        )));

        b.push_quad(s, p, plain, None);
        b.push_quad(s, p, typed, None);
        b.push_quad(s, p, tagged, None);
        b.push_quad(s, p, directional, None);
        b.push_quad(s, p, long, None);
        let quoted = b.intern_triple(s, p, plain);
        b.push_quad(s, q, quoted, None);
    }

    b.freeze()
        .expect("literal-heavy dataset is structurally valid")
}

/// Named graphs that own rows in [`build_graph_heavy_dataset`].
const GRAPH_ROWS: u32 = 1_000;

/// Many named graphs, each with one base quad, a reifier-only graph every tenth row,
/// and — when `declared` — as many again declared with no row. The two variants have
/// identical rows, so their encode costs differ only by the declaration handling.
fn build_graph_heavy_dataset(declared: bool) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri("http://example.org/p");
    let o = b.intern_iri("http://example.org/o");
    for n in 0..GRAPH_ROWS {
        let s = b.intern_iri(&format!("http://example.org/s{n}"));
        let g = b.intern_iri(&format!("http://example.org/g{n}"));
        b.push_quad(s, p, o, Some(g));
        if n % 10 == 0 {
            let triple = b.intern_triple(s, p, o);
            let r = b.intern_iri(&format!("http://example.org/r{n}"));
            let side = b.intern_iri(&format!("http://example.org/side{n}"));
            b.push_reifier_in_graph(r, triple, Some(side));
        }
        if declared {
            let empty = b.intern_iri(&format!("http://example.org/empty{n}"));
            b.declare_named_graph(empty);
        }
    }
    b.freeze()
        .expect("graph-heavy dataset is structurally valid")
}

/// Encode and enumerate over the graph-heavy fixture: `build_bytes` pays the
/// declaration scan (and, with `declared`, writes a zero-row partition per declared
/// graph); `named_graphs` merges the TRIPLES partitions with the side-table graphs.
fn bench_named_graphs(c: &mut Bench) {
    let mut group = c.benchmark_group("pack_query_named_graphs");
    for (label, declared) in [("rows_only", false), ("declared_empty", true)] {
        let ds = build_graph_heavy_dataset(declared);
        group.bench_function(format!("build_bytes_{label}"), |b| {
            b.iter(|| {
                std::hint::black_box(PackBuilder::build_bytes(&ds).expect("graph-heavy packs"))
            });
        });
        let bytes = PackBuilder::build_bytes(&ds).expect("graph-heavy packs");
        let pack = PackView::from_bytes(&bytes).expect("pack opens");
        println!(
            "[pack_query_named_graphs] {label}: {} named graphs, {} pack bytes",
            pack.named_graphs().count(),
            bytes.len()
        );
        group.bench_function(format!("named_graphs_{label}"), |b| {
            b.iter(|| std::hint::black_box(pack.named_graphs().count()));
        });
    }
    group.finish();
}

/// Dictionary encode alone: [`PackDict::encode`] (term collection, closure,
/// canonical sort, PFC record encoding) over the literal-heavy fixture.
fn bench_dict_encode(c: &mut Bench) {
    let ds = build_literal_heavy_dataset();
    let encoded = PackDict::encode(&ds);
    println!(
        "[pack_query_dict_encode] {} terms, {} value bytes",
        encoded.n_terms,
        encoded.to_bytes().len()
    );
    let mut group = c.benchmark_group("pack_query_dict_encode");
    group.bench_function("literal_heavy_2k_rows", |b| {
        b.iter(|| std::hint::black_box(PackDict::encode(std::hint::black_box(&ds))));
    });
    group.finish();
}

/// Encode: [`PackBuilder::build_bytes`] over the representative dataset.
fn bench_build_bytes(c: &mut Bench) {
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
fn bench_from_bytes(c: &mut Bench) {
    let ds = build_dataset();
    let bytes = PackBuilder::build_bytes(&ds).expect("representative dataset packs");
    let mut group = c.benchmark_group("pack_query_from_bytes");
    group.bench_function("from_bytes", |b| {
        b.iter(|| std::hint::black_box(PackView::from_bytes(&bytes).expect("pack opens")));
    });
    group.finish();
}

/// Restore: open an already-built pack and materialize the complete frozen
/// `RdfDataset` without passing through an RDF text serialization.
fn bench_restore_pack(c: &mut Bench) {
    let ds = build_dataset();
    let bytes = PackBuilder::build_bytes(&ds).expect("representative dataset packs");
    let mut group = c.benchmark_group("pack_query_restore");
    group.bench_function("restore_pack", |b| {
        b.iter(|| std::hint::black_box(restore_pack(&bytes).expect("pack restores")));
    });
    group.finish();
}

/// Query the compressed form: `quads_for_pattern`, iterated to completion, over a
/// warm [`PackView`], for the four representative shapes [`build_dataset`] sets up.
fn bench_quads_for_pattern(c: &mut Bench) {
    let ds = build_dataset();
    let bytes = PackBuilder::build_bytes(&ds).expect("representative dataset packs");
    let pack = PackView::from_bytes(&bytes).expect("pack opens");

    let p_id = pack
        .term_id_by_value(&purrdf_core::TermValue::iri("http://example.org/p"))
        .expect("dictionary read succeeds")
        .expect("predicate interned");
    let s0_id = pack
        .term_id_by_value(&purrdf_core::TermValue::iri("http://example.org/s0"))
        .expect("dictionary read succeeds")
        .expect("s0 interned");
    let common_id = pack
        .term_id_by_value(&purrdf_core::TermValue::iri("http://example.org/common"))
        .expect("dictionary read succeeds")
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
fn bench_verify_pack(c: &mut Bench) {
    let ds = build_dataset();
    let bytes = PackBuilder::build_bytes(&ds).expect("representative dataset packs");
    let mut group = c.benchmark_group("pack_query_verify");
    group.bench_function("verify_pack", |b| {
        b.iter(|| std::hint::black_box(verify_pack(&bytes).expect("pack verifies")));
    });
    group.finish();
}

bench_group!(
    benches,
    bench_dict_encode,
    bench_build_bytes,
    bench_from_bytes,
    bench_restore_pack,
    bench_quads_for_pattern,
    bench_verify_pack,
    bench_named_graphs
);
bench_main!(benches);
