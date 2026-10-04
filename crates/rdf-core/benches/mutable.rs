// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API, so the workspace `missing_docs` lint is
// not asked of their items.
#![allow(missing_docs)]

//! The copy-on-write `MutableDataset` measured hypothesis.
//!
//! The PLAN frames COW as *"a measured hypothesis, not an assumed win — benchmark it
//! against a simpler hash-indexed mutable store before committing"*. This harness is
//! that gate: it runs the SAME representative mutate workload (N inserts + M removes +
//! pattern queries) against
//!
//! 1. the shipped COW [`MutableDataset`] (`base ∪ added − suppressed`, tagged-handle
//!    delta), and
//! 2. a bench-local **simple hash-indexed store** — a `HashSet` of owned value-quad
//!    tuples, the head-to-head comparand (mirrors the `SoaQuads`/`PredicateAdjacency`
//!    shims in `ir_layout.rs`).
//!
//! It reports build / mutate / query time for both. It deliberately asserts NO winner
//! — it just measures the two side-by-side so the COW choice is data, not assertion.

use std::sync::Arc;

use purrdf_core::{
    DatasetMut, DatasetView, FastHasher, FastSet, GraphMatchValue, MutableDataset, QuadValues,
    RdfDataset, RdfDatasetBuilder, TermValue,
};
use purrdf_testkit::bench::{Bench, bench_group, bench_main};

/// Number of base quads the COW base / simple store start from.
const BASE_QUADS: u32 = 2000;
/// Inserts applied in the mutate workload (brand-new delta quads).
const INSERTS: u32 = 400;
/// Removes applied in the mutate workload (existing base quads).
const REMOVES: u32 = 400;

fn iri(n: &str) -> TermValue {
    TermValue::Iri(format!("http://example.org/{n}"))
}

/// A deterministic base of `BASE_QUADS` quads: `(s{n}, p, o{n})`.
fn build_base() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri("http://example.org/p");
    for n in 0..BASE_QUADS {
        let s = b.intern_iri(&format!("http://example.org/s{n}"));
        let o = b.intern_iri(&format!("http://example.org/o{n}"));
        b.push_quad(s, p, o, None);
    }
    b.freeze().expect("base freezes")
}

/// The mutate workload as value-quads: `INSERTS` brand-new quads then `REMOVES`
/// existing base quads. Shared by both stores so they do identical work.
fn workload() -> (Vec<QuadValues>, Vec<QuadValues>) {
    let inserts = (0..INSERTS)
        .map(|n| QuadValues::triple(iri(&format!("new{n}")), iri("p"), iri(&format!("no{n}"))))
        .collect();
    let removes = (0..REMOVES)
        .map(|n| QuadValues::triple(iri(&format!("s{n}")), iri("p"), iri(&format!("o{n}"))))
        .collect();
    (inserts, removes)
}

// --------------------------------------------------------------------------------
// The simple hash-indexed comparand: a HashSet of owned value-quad tuples.
// --------------------------------------------------------------------------------

type QuadTuple = (TermValue, TermValue, TermValue, Option<TermValue>);

/// A naive mutable RDF store: every quad held by value in one `HashSet`. No COW, no
/// delta, no base sharing — the simplest thing that could possibly work, and the
/// thing COW must beat to earn its complexity.
struct SimpleStore {
    quads: FastSet<QuadTuple>,
}

impl SimpleStore {
    /// Materialize the whole base into the set (the simple store has no sharing, so a
    /// branch is a full copy — the cost COW avoids).
    fn from_base(base: &RdfDataset) -> Self {
        let mut quads = FastSet::with_capacity_and_hasher(base.quad_count(), FastHasher::default());
        for q in base.quads() {
            quads.insert((
                resolve(base, q.s),
                resolve(base, q.p),
                resolve(base, q.o),
                q.g.map(|g| resolve(base, g)),
            ));
        }
        Self { quads }
    }

    fn insert(&mut self, q: QuadValues) -> bool {
        self.quads.insert((q.s, q.p, q.o, q.g))
    }

    fn remove(&mut self, q: &QuadValues) -> bool {
        self.quads
            .remove(&(q.s.clone(), q.p.clone(), q.o.clone(), q.g.clone()))
    }

    fn quads_for_pattern(&self, p: Option<&TermValue>) -> usize {
        self.quads
            .iter()
            .filter(|(_, qp, _, _)| p.is_none_or(|want| qp == want))
            .count()
    }
}

/// Resolve a base id to its value-tuple component (IRIs only in this workload).
fn resolve(base: &RdfDataset, id: purrdf_core::TermId) -> TermValue {
    match base.resolve(id) {
        purrdf_core::TermRef::Iri(s) => TermValue::Iri(s.to_string()),
        other => TermValue::Iri(format!("{other:?}")),
    }
}

// --------------------------------------------------------------------------------
// Bench groups: build / mutate / query, COW vs simple, head-to-head.
// --------------------------------------------------------------------------------

fn bench_build(c: &mut Bench) {
    let base = build_base();
    let mut group = c.benchmark_group("mut_build");
    // COW branch = clone the Arc + empty delta (O(1)).
    group.bench_function("cow_branch", |b| {
        b.iter(|| std::hint::black_box(MutableDataset::new(Arc::clone(&base))));
    });
    // Simple store branch = a full copy of the base into a HashSet.
    group.bench_function("simple_copy", |b| {
        b.iter(|| std::hint::black_box(SimpleStore::from_base(&base)));
    });
    group.finish();
}

fn bench_mutate(c: &mut Bench) {
    let base = build_base();
    let (inserts, removes) = workload();

    let mut group = c.benchmark_group("mut_mutate");
    group.bench_function("cow", |b| {
        b.iter(|| {
            let mut m = MutableDataset::new(Arc::clone(&base));
            for q in &inserts {
                let _ = m.insert(q.clone()).expect("bench fixtures are absolute");
            }
            for q in &removes {
                m.remove(q);
            }
            std::hint::black_box(m.added_len() + m.suppressed_len())
        });
    });
    group.bench_function("simple", |b| {
        b.iter(|| {
            let mut s = SimpleStore::from_base(&base);
            for q in &inserts {
                s.insert(q.clone());
            }
            for q in &removes {
                s.remove(q);
            }
            std::hint::black_box(s.quads.len())
        });
    });
    group.finish();
}

fn bench_query(c: &mut Bench) {
    let base = build_base();
    let (inserts, removes) = workload();

    // Pre-mutate both stores so the query group measures pattern lookup over the
    // post-mutation effective set.
    let mut cow = MutableDataset::new(Arc::clone(&base));
    for q in &inserts {
        let _ = cow.insert(q.clone()).expect("bench fixtures are absolute");
    }
    for q in &removes {
        cow.remove(q);
    }
    let mut simple = SimpleStore::from_base(&base);
    for q in &inserts {
        simple.insert(q.clone());
    }
    for q in &removes {
        simple.remove(q);
    }
    let pred = iri("p");

    let mut group = c.benchmark_group("mut_query");
    group.bench_function("cow_predicate_scan", |b| {
        b.iter(|| {
            std::hint::black_box(
                cow.quads_for_pattern(None, Some(&pred), None, GraphMatchValue::Any)
                    .len(),
            )
        });
    });
    group.bench_function("simple_predicate_scan", |b| {
        b.iter(|| std::hint::black_box(simple.quads_for_pattern(Some(&pred))));
    });
    group.finish();
}

/// `MutableDataset::freeze` of the post-mutation effective set: every surviving
/// base quad's four terms are carried into a fresh builder. The base shares one
/// predicate across all quads and every subject/object recurs, so the per-base-id
/// memo (one slot read per repeat, no owned `TermValue` rebuild) is what this
/// measures. Report-only.
fn bench_freeze(c: &mut Bench) {
    let base = build_base();
    let (inserts, removes) = workload();
    let mut cow = MutableDataset::new(Arc::clone(&base));
    for q in &inserts {
        let _ = cow.insert(q.clone()).expect("bench fixtures are absolute");
    }
    for q in &removes {
        cow.remove(q);
    }

    let mut group = c.benchmark_group("mut_freeze");
    group.bench_function("cow_freeze", |b| {
        b.iter(|| {
            std::hint::black_box(
                cow.freeze()
                    .expect("mutated bench fixture freezes")
                    .quad_count(),
            )
        });
    });
    group.finish();
}

/// Named graphs in the snapshot-enumeration base.
const GRAPHS: u32 = 200;
/// Declared empty graphs in the snapshot-enumeration base.
const EMPTY_GRAPHS: u32 = 50;

/// `BASE_QUADS` quads spread over `GRAPHS` named graphs (`(s{n}, p, o{n}, g{n % GRAPHS})`),
/// plus `EMPTY_GRAPHS` declared empty graphs.
fn build_graph_base() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri("http://example.org/p");
    for n in 0..BASE_QUADS {
        let s = b.intern_iri(&format!("http://example.org/s{n}"));
        let o = b.intern_iri(&format!("http://example.org/o{n}"));
        let g = b.intern_iri(&format!("http://example.org/g{}", n % GRAPHS));
        b.push_quad(s, p, o, Some(g));
    }
    for n in 0..EMPTY_GRAPHS {
        let g = b.intern_iri(&format!("http://example.org/empty{n}"));
        b.declare_named_graph(g);
    }
    b.freeze().expect("graph base freezes")
}

/// Snapshot publication and `GRAPH ?g` enumeration after removals spread over many
/// named graphs: the first `REMOVES` base quads, which empties `REMOVES / (BASE_QUADS
/// / GRAPHS)` graphs whole and leaves the rest partly populated, plus the withdrawal of
/// half the declared empty graphs. The mutation keeps each touched graph's live row
/// count, so publication shares the set of emptied graphs instead of probing the
/// base; this lane and the two below measure that it stays so. Report-only.
fn bench_snapshot_graphs(c: &mut Bench) {
    let base = build_graph_base();
    let mut cow = MutableDataset::new(Arc::clone(&base));
    for n in 0..REMOVES {
        cow.remove(&QuadValues::quad(
            iri(&format!("s{n}")),
            iri("p"),
            iri(&format!("o{n}")),
            iri(&format!("g{}", n % GRAPHS)),
        ));
    }
    for n in 0..EMPTY_GRAPHS / 2 {
        cow.withdraw_graph_declaration(&iri(&format!("empty{n}")));
    }

    let annotated = annotated_after_small_drop();
    let dropped = after_large_drop();

    let mut group = c.benchmark_group("mut_snapshot_graphs");
    for (name, mutation) in [
        ("snapshot_and_enumerate", &cow),
        ("annotated_base_small_drop", &annotated),
        ("large_drop_repeated", &dropped),
    ] {
        group.bench_function(name, |b| {
            b.iter(|| {
                let view = mutation
                    .snapshot_view()
                    .expect("mutated bench fixture publishes");
                std::hint::black_box(view.named_graphs().count())
            });
        });
    }
    group.finish();
}

/// Statements in the annotated base: each a reified, annotated triple in `other`.
const STATEMENTS: u32 = 100_000;
/// Quads in the large graph a DROP removes.
const LARGE_GRAPH: u32 = 100_000;

/// Lane (i): a base of `STATEMENTS` reified and annotated statements in the graph
/// `other`, plus a one-quad graph `small`, after `DROP GRAPH small` (its quad
/// removed and its declaration withdrawn). A snapshot's graph enumeration must cost
/// nothing proportional to the untouched statement tables.
fn annotated_after_small_drop() -> MutableDataset {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri("http://example.org/p");
    let note = b.intern_iri("http://example.org/note");
    let other = b.intern_iri("http://example.org/other");
    let small = b.intern_iri("http://example.org/small");
    for n in 0..STATEMENTS {
        let s = b.intern_iri(&format!("http://example.org/s{n}"));
        let o = b.intern_iri(&format!("http://example.org/o{n}"));
        let r = b.intern_iri(&format!("http://example.org/r{n}"));
        b.push_quad(s, p, o, Some(other));
        let triple = b.intern_triple(s, p, o);
        b.push_reifier_in_graph(r, triple, Some(other));
        b.push_annotation_in_graph(r, note, o, Some(other));
    }
    let s = b.intern_iri("http://example.org/s0");
    let o = b.intern_iri("http://example.org/o0");
    b.push_quad(s, p, o, Some(small));
    let mut cow = MutableDataset::new(b.freeze().expect("annotated base freezes"));
    cow.remove(&QuadValues::quad(
        iri("s0"),
        iri("p"),
        iri("o0"),
        iri("small"),
    ));
    cow.withdraw_graph_declaration(&iri("small"));
    cow
}

/// Lane (ii): a base whose graph `big` holds `LARGE_GRAPH` quads, after `DROP GRAPH
/// big` removed every one of them. Repeated snapshots must not re-walk the dropped
/// run.
fn after_large_drop() -> MutableDataset {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri("http://example.org/p");
    let big = b.intern_iri("http://example.org/big");
    let other = b.intern_iri("http://example.org/other");
    for n in 0..LARGE_GRAPH {
        let s = b.intern_iri(&format!("http://example.org/s{n}"));
        let o = b.intern_iri(&format!("http://example.org/o{n}"));
        b.push_quad(s, p, o, Some(big));
    }
    let s = b.intern_iri("http://example.org/s0");
    let o = b.intern_iri("http://example.org/o0");
    b.push_quad(s, p, o, Some(other));
    let mut cow = MutableDataset::new(b.freeze().expect("large base freezes"));
    for n in 0..LARGE_GRAPH {
        cow.remove(&QuadValues::quad(
            iri(&format!("s{n}")),
            iri("p"),
            iri(&format!("o{n}")),
            iri("big"),
        ));
    }
    cow.withdraw_graph_declaration(&iri("big"));
    cow
}

/// Print the relative head-to-head context once: how many quads each store holds, so
/// the timed numbers are read against the same effective set. No winner asserted.
fn bench_context(_c: &mut Bench) {
    let base = build_base();
    let (inserts, removes) = workload();
    let mut cow = MutableDataset::new(Arc::clone(&base));
    for q in &inserts {
        let _ = cow.insert(q.clone()).expect("bench fixtures are absolute");
    }
    for q in &removes {
        cow.remove(q);
    }
    println!(
        "[mutable] base_quads={} inserts={} removes={} cow_effective={} (added={}, suppressed={})",
        base.quad_count(),
        INSERTS,
        REMOVES,
        cow.effective_count(),
        cow.added_len(),
        cow.suppressed_len(),
    );
    println!(
        "[mutable] NOTE: COW = Arc-shared base + tagged-handle delta (O(1) branch); \
         simple = HashSet of owned value-quad tuples (full-copy branch). Head-to-head only, \
         no winner asserted — the PLAN's measured-hypothesis gate."
    );
}

bench_group!(
    benches,
    bench_context,
    bench_build,
    bench_mutate,
    bench_query,
    bench_freeze,
    bench_snapshot_graphs
);
bench_main!(benches);
