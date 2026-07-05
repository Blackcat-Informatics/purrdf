// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The copy-on-write `MutableDataset` measured hypothesis (purrdf P5).
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

use std::collections::HashSet;
use std::sync::Arc;

use criterion::{Criterion, criterion_group, criterion_main};
use purrdf_core::{
    DatasetMut, GraphMatchValue, MutableDataset, QuadValues, RdfDataset, RdfDatasetBuilder,
    TermValue,
};

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
    quads: HashSet<QuadTuple>,
}

impl SimpleStore {
    /// Materialize the whole base into the set (the simple store has no sharing, so a
    /// branch is a full copy — the cost COW avoids).
    fn from_base(base: &RdfDataset) -> Self {
        let mut quads = HashSet::with_capacity(base.quad_count());
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
// Criterion groups: build / mutate / query, COW vs simple, head-to-head.
// --------------------------------------------------------------------------------

fn bench_build(c: &mut Criterion) {
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

fn bench_mutate(c: &mut Criterion) {
    let base = build_base();
    let (inserts, removes) = workload();

    let mut group = c.benchmark_group("mut_mutate");
    group.bench_function("cow", |b| {
        b.iter(|| {
            let mut m = MutableDataset::new(Arc::clone(&base));
            for q in &inserts {
                m.insert(q.clone());
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

fn bench_query(c: &mut Criterion) {
    let base = build_base();
    let (inserts, removes) = workload();

    // Pre-mutate both stores so the query group measures pattern lookup over the
    // post-mutation effective set.
    let mut cow = MutableDataset::new(Arc::clone(&base));
    for q in &inserts {
        cow.insert(q.clone());
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

/// Print the relative head-to-head context once: how many quads each store holds, so
/// the timed numbers are read against the same effective set. No winner asserted.
fn bench_context(_c: &mut Criterion) {
    let base = build_base();
    let (inserts, removes) = workload();
    let mut cow = MutableDataset::new(Arc::clone(&base));
    for q in &inserts {
        cow.insert(q.clone());
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

criterion_group!(
    benches,
    bench_context,
    bench_build,
    bench_mutate,
    bench_query
);
criterion_main!(benches);
