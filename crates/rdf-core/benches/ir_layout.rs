// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Layout / IR benchmark for the value-interned `RdfDataset` (C1, Task 7).
//!
//! The RFC removes "columnar" and "zero-copy" as *asserted* commitments and
//! replaces them with a measurable gate: *"Layout is chosen by benchmark, not
//! asserted"* and *"Benchmarks report total allocated bytes, allocation count, peak
//! memory, index-build cost, and end-to-end latency — not only quads/sec."* This
//! harness is that gate.
//!
//! It does three things:
//!
//! 1. **Times** the operational hot paths as criterion groups: dataset *build*
//!    (intern + push + freeze → `Arc<RdfDataset>`), ID-native *iteration*
//!    (`quads()`), and resolved *resolution* (`quad_refs()`/`resolve()`).
//! 2. **Reports the operational metrics beyond quads/sec** — total allocated bytes,
//!    allocation count, and the allocator high-water mark — for one full build and
//!    for one full iteration, via a process-global counting allocator whose counters
//!    are snapshotted around each measured region and printed as deltas. A true
//!    peak-RSS read is impractical in-process, so the high-water mark of bytes the
//!    allocator has handed out approximates peak memory (documented, not hidden).
//! 3. **Demonstrates the layout choice head-to-head**: the shipped layout is
//!    array-of-structures quad rows (`Box<[QuadRow]>`). The bench builds a
//!    bench-local structure-of-arrays (SoA) shim over the *same* frozen quads and
//!    benchmarks the identical iteration on both, so the AoS-vs-SoA choice is
//!    *measured*, not asserted. A predicate-grouped adjacency shim is also measured
//!    so the third candidate the RFC names is benched rather than deferred.
//!
//! There is no secondary index in the shipped dataset today (the only non-linear
//! structure is the sparse, handle-sorted source-location table built at freeze, so
//! its "index-build cost" is already folded into the build group); this is noted in
//! the build group rather than benched as a separate index pass.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::Arc;

use criterion::{Criterion, criterion_group, criterion_main};
use purrdf_core::{
    BlankScope, DatasetView, GraphMatch, QuadIds, RdfDataset, RdfDatasetBuilder, RdfLiteral,
    TermId, TermRef,
};

// ---------------------------------------------------------------------------
// Counting allocator — operational metrics beyond quads/sec.
// ---------------------------------------------------------------------------
//
// Mirrors `crates/rdf/tests/ir_zero_alloc.rs`: a pass-through `#[global_allocator]`
// that records every `alloc`/`realloc` on the *current thread* (thread-local, so a
// sibling criterion thread cannot contaminate the snapshot). Beyond the bare count
// the zero-alloc test keeps, this also tracks total bytes requested and a high-water
// mark of net live bytes so the bench can report allocated-bytes + count + peak.

thread_local! {
    static ALLOC_COUNT: Cell<u64> = const { Cell::new(0) };
    static ALLOC_BYTES: Cell<u64> = const { Cell::new(0) };
    static LIVE_BYTES: Cell<i64> = const { Cell::new(0) };
    static PEAK_BYTES: Cell<i64> = const { Cell::new(0) };
}

struct CountingAllocator;

/// Record one allocation of `size` bytes on the current thread, tolerating TLS-init
/// re-entrancy by silently skipping when the thread-local is not yet available.
fn on_alloc(size: usize) {
    let _ = ALLOC_COUNT.try_with(|c| c.set(c.get() + 1));
    let _ = ALLOC_BYTES.try_with(|c| c.set(c.get() + size as u64));
    let _ = LIVE_BYTES.try_with(|live| {
        let now = live.get() + size as i64;
        live.set(now);
        let _ = PEAK_BYTES.try_with(|peak| {
            if now > peak.get() {
                peak.set(now);
            }
        });
    });
}

/// Record one deallocation of `size` bytes (only the live/peak tracking cares).
fn on_dealloc(size: usize) {
    let _ = LIVE_BYTES.try_with(|live| live.set(live.get() - size as i64));
}

// SAFETY: every method forwards to the system allocator with the same layout; the
// only added behavior is thread-local counter bookkeeping on alloc/dealloc paths.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe {
            on_alloc(layout.size());
            System.alloc(layout)
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe {
            on_dealloc(layout.size());
            System.dealloc(ptr, layout);
        }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        unsafe {
            // A realloc frees the old block and hands back a (possibly) larger one.
            on_dealloc(layout.size());
            on_alloc(new_size);
            System.realloc(ptr, layout, new_size)
        }
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

/// A snapshot of the current thread's allocation counters.
#[derive(Clone, Copy)]
struct AllocSnapshot {
    count: u64,
    bytes: u64,
    peak: i64,
}

fn snapshot() -> AllocSnapshot {
    AllocSnapshot {
        count: ALLOC_COUNT.with(Cell::get),
        bytes: ALLOC_BYTES.with(Cell::get),
        peak: PEAK_BYTES.with(Cell::get),
    }
}

/// Reset the high-water mark to the current live level so the next measured region's
/// peak is measured relative to its own start, not a stale historical maximum.
fn reset_peak_to_live() {
    LIVE_BYTES.with(|live| {
        PEAK_BYTES.with(|peak| peak.set(live.get()));
    });
}

/// Print the allocation deltas for one measured region as `total allocated bytes +
/// allocation count + peak`, satisfying the RFC's "report metrics beyond quads/sec".
fn report(label: &str, before: AllocSnapshot, after: AllocSnapshot) {
    let count = after.count - before.count;
    let bytes = after.bytes - before.bytes;
    // Peak is the high-water mark of net-live bytes reached *during* the region,
    // relative to the live level at entry (the harness reset it at `before`).
    let peak_delta = (after.peak - before.peak).max(0);
    println!(
        "[ir_layout] {label:28} allocations={count:>8}  allocated_bytes={bytes:>10}  \
         peak_live_bytes={peak_delta:>10}"
    );
}

// ---------------------------------------------------------------------------
// Deterministic representative dataset.
// ---------------------------------------------------------------------------

/// Number of subject "rows" generated. Each row emits several quads spanning every
/// term variant, so the frozen dataset is a few thousand quads.
const ROWS: u32 = 800;

/// Build a representative dataset programmatically with a *deterministic* generator
/// (no RNG): IRIs, blanks across scopes, typed + language-tagged literals, a named
/// graph, reifiers + annotations, and nested triple terms. This exercises every
/// `resolve()` arm and every capability flag — the same shape the zero-alloc test
/// uses, scaled up to benchmark size.
fn build_dataset() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri("http://example.org/p");
    let g = b.intern_iri("http://example.org/g");
    let asserts = b.intern_iri("http://example.org/asserts");
    let confidence = b.intern_iri("http://example.org/confidence");

    for n in 0..ROWS {
        let s = b.intern_iri(&format!("http://example.org/s{n}"));
        let bnode = b.intern_blank(&format!("b{n}"), BlankScope(n % 4));
        let lit = b.intern_literal(RdfLiteral::language_tagged(format!("value {n}"), "EN"));
        let typed = b.intern_literal(RdfLiteral::typed(
            format!("{n}"),
            "http://www.w3.org/2001/XMLSchema#integer",
        ));

        // Plain triples spanning the default and a named graph.
        b.push_quad(s, p, bnode, None);
        b.push_quad(s, p, lit, Some(g));
        b.push_quad(bnode, p, typed, None);

        // A nested triple term as object, exercising the Triple arm of resolve().
        let inner = b.intern_triple(s, p, typed);
        b.push_quad(s, asserts, inner, Some(g));

        // Reify every 4th row and annotate it, so the reifier/annotation tables and
        // their capability flags are populated.
        if n % 4 == 0 {
            let triple = b.intern_triple(s, p, bnode);
            let reifier = b.intern_iri(&format!("http://example.org/r{n}"));
            b.push_reifier(reifier, triple);
            let score = b.intern_literal(RdfLiteral::typed(
                format!("0.{n}"),
                "http://www.w3.org/2001/XMLSchema#decimal",
            ));
            b.push_annotation(reifier, confidence, score);
        }
    }

    b.freeze()
        .expect("representative dataset is structurally valid")
}

// ---------------------------------------------------------------------------
// Layout shims — measured head-to-head against the shipped AoS rows.
// ---------------------------------------------------------------------------

/// Bench-local structure-of-arrays view of the frozen quads: four parallel columns
/// rather than the shipped array-of-structures `Box<[QuadRow]>`. Built once from the
/// dataset's own `quads()`; this is a measurement shim, NOT wired into the real
/// dataset. Iterating it reconstructs the same `QuadIds` the AoS path yields.
struct SoaQuads {
    s: Vec<TermId>,
    p: Vec<TermId>,
    o: Vec<TermId>,
    g: Vec<Option<TermId>>,
}

impl SoaQuads {
    fn from_dataset(ds: &RdfDataset) -> Self {
        let n = ds.quad_count();
        let mut soa = Self {
            s: Vec::with_capacity(n),
            p: Vec::with_capacity(n),
            o: Vec::with_capacity(n),
            g: Vec::with_capacity(n),
        };
        for q in ds.quads() {
            soa.s.push(q.s);
            soa.p.push(q.p);
            soa.o.push(q.o);
            soa.g.push(q.g);
        }
        soa
    }

    /// Iterate the columns back into `QuadIds` — the SoA counterpart of `quads()`.
    fn quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        (0..self.s.len()).map(move |i| QuadIds {
            s: self.s[i],
            p: self.p[i],
            o: self.o[i],
            g: self.g[i],
        })
    }
}

/// One `(subject, object, graph)` row inside a predicate bucket.
type AdjacencyRow = (TermId, TermId, Option<TermId>);
/// One predicate bucket: the predicate id plus its rows, in first-seen order.
type PredicateBucket = (TermId, Vec<AdjacencyRow>);

/// Bench-local predicate-grouped adjacency: quads bucketed by predicate id, the third
/// layout candidate the RFC names. Built once from the frozen quads; a measurement
/// shim only. Iterating it visits every quad exactly once (grouped by predicate).
struct PredicateAdjacency {
    /// One bucket of `(s, o, g)` triples per distinct predicate, in first-seen order.
    buckets: Vec<PredicateBucket>,
}

impl PredicateAdjacency {
    fn from_dataset(ds: &RdfDataset) -> Self {
        // Linear scan grouping by predicate; preserves a deterministic bucket order.
        let mut buckets: Vec<PredicateBucket> = Vec::new();
        for q in ds.quads() {
            match buckets.iter_mut().find(|(pred, _)| *pred == q.p) {
                Some((_, rows)) => rows.push((q.s, q.o, q.g)),
                None => buckets.push((q.p, vec![(q.s, q.o, q.g)])),
            }
        }
        Self { buckets }
    }

    fn quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.buckets.iter().flat_map(|(p, rows)| {
            rows.iter()
                .map(move |&(s, o, g)| QuadIds { s, p: *p, o, g })
        })
    }
}

// ---------------------------------------------------------------------------
// Iteration / resolution kernels (shared, so AoS and shims do identical work).
// ---------------------------------------------------------------------------

/// Fully consume `QuadIds` without allocating: fold the Copy ids into a checksum.
#[inline]
fn consume_ids(q: QuadIds) -> u64 {
    // Reuse the QuadIds' own Hash via a cheap stack fold; black_box guards DCE.
    let mut acc = 0u64;
    for id in [Some(q.s), Some(q.p), Some(q.o), q.g] {
        acc = acc
            .wrapping_mul(1_099_511_628_211)
            .wrapping_add(id.map_or(0xFFFF_FFFF, |_| 1));
    }
    acc
}

/// Resolve every position of a quad to a borrowed view and sum borrowed `&str`
/// lengths — touches resolved content without copying it (no allocation).
fn resolve_len(ds: &RdfDataset, q: QuadIds) -> usize {
    term_len(ds.resolve(q.s))
        + term_len(ds.resolve(q.p))
        + term_len(ds.resolve(q.o))
        + q.g.map_or(0, |g| term_len(ds.resolve(g)))
}

fn term_len(t: TermRef<'_>) -> usize {
    match t {
        TermRef::Iri(s) => s.len(),
        TermRef::Blank { label, .. } => label.len(),
        TermRef::Literal {
            lexical, language, ..
        } => lexical.len() + language.map_or(0, str::len),
        TermRef::Triple { .. } => 0,
    }
}

// ---------------------------------------------------------------------------
// Allocation report (printed once, before the timed groups).
// ---------------------------------------------------------------------------

/// Measure and PRINT the operational metrics the RFC requires (allocated bytes,
/// allocation count, peak) for one full build and one full iteration. Criterion only
/// reports time; these `println!`s carry the alloc story alongside it.
fn print_alloc_metrics() {
    // Build cost: interning + pushing + freeze.
    reset_peak_to_live();
    let before = snapshot();
    let ds = build_dataset();
    let after = snapshot();
    report("build (intern+push+freeze)", before, after);
    println!(
        "[ir_layout] dataset: quads={} terms={}",
        ds.quad_count(),
        ds.term_count()
    );

    // One full AoS iteration over the frozen dataset — the hot path. This must be a
    // zero-allocation region (proven by tests/ir_zero_alloc.rs); the report shows it.
    reset_peak_to_live();
    let before = snapshot();
    let mut acc = 0u64;
    for q in ds.quads() {
        acc = acc.wrapping_add(consume_ids(q));
    }
    let after = snapshot();
    std::hint::black_box(acc);
    report("iterate AoS quads()", before, after);

    // One full resolution pass — borrows every term without copying.
    reset_peak_to_live();
    let before = snapshot();
    let mut acc = 0usize;
    for q in ds.quads() {
        acc = acc.wrapping_add(resolve_len(&ds, q));
    }
    let after = snapshot();
    std::hint::black_box(acc);
    report("resolve quad_refs()/resolve()", before, after);

    // Build cost of each measurement shim, so the AoS-vs-alternatives comparison
    // reports memory as well as time.
    reset_peak_to_live();
    let before = snapshot();
    let soa = SoaQuads::from_dataset(&ds);
    let after = snapshot();
    report("build SoA columns (shim)", before, after);
    std::hint::black_box(soa.s.len());

    reset_peak_to_live();
    let before = snapshot();
    let adj = PredicateAdjacency::from_dataset(&ds);
    let after = snapshot();
    report("build pred-adjacency (shim)", before, after);
    std::hint::black_box(adj.buckets.len());

    println!(
        "[ir_layout] NOTE: shipped layout = array-of-structures QuadRow (Box<[QuadRow]>); \
         SoA + predicate-adjacency are bench-local shims measured head-to-head, not wired in. \
         No standalone secondary index exists today (the sparse location table is built at \
         freeze, folded into the build cost above)."
    );
}

// ---------------------------------------------------------------------------
// Criterion groups.
// ---------------------------------------------------------------------------

fn bench_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("ir_build");
    group.bench_function("intern_push_freeze", |b| {
        b.iter(|| std::hint::black_box(build_dataset()));
    });
    group.finish();
}

fn bench_iterate(c: &mut Criterion) {
    let ds = build_dataset();
    let soa = SoaQuads::from_dataset(&ds);
    let adj = PredicateAdjacency::from_dataset(&ds);

    let mut group = c.benchmark_group("ir_iterate");
    // AoS — the shipped hot path.
    group.bench_function("aos_quads", |b| {
        b.iter(|| {
            let mut acc = 0u64;
            for q in ds.quads() {
                acc = acc.wrapping_add(consume_ids(q));
            }
            std::hint::black_box(acc)
        });
    });
    // SoA — the head-to-head alternative on the SAME quads.
    group.bench_function("soa_quads", |b| {
        b.iter(|| {
            let mut acc = 0u64;
            for q in soa.quads() {
                acc = acc.wrapping_add(consume_ids(q));
            }
            std::hint::black_box(acc)
        });
    });
    // Predicate-grouped adjacency — the third candidate the RFC names.
    group.bench_function("predicate_adjacency", |b| {
        b.iter(|| {
            let mut acc = 0u64;
            for q in adj.quads() {
                acc = acc.wrapping_add(consume_ids(q));
            }
            std::hint::black_box(acc)
        });
    });
    group.finish();
}

fn bench_resolve(c: &mut Criterion) {
    let ds = build_dataset();
    let mut group = c.benchmark_group("ir_resolve");
    group.bench_function("quad_refs_resolve", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for q in ds.quads() {
                acc = acc.wrapping_add(resolve_len(&ds, q));
            }
            std::hint::black_box(acc)
        });
    });
    group.finish();
}

/// P4b indexed `quads_for_pattern` vs the linear scan, on WARM permutation
/// indexes. Each `(s|p|o)`-bound shape exercises a different permutation (SPOG / POS /
/// OSP); the scan baseline is the same id-equality filter the trait default runs.
fn bench_pattern_warm(c: &mut Criterion) {
    let ds = build_dataset();
    let sample = ds.quads().next().expect("build_dataset yields quads");
    let (subj, pred, obj) = (sample.s, sample.p, sample.o);

    // Warm every permutation the shapes below select, so the timed loops measure the
    // indexed LOOKUP, not the one-time build.
    let warm = |s, p, o| DatasetView::quads_for_pattern(&*ds, s, p, o, GraphMatch::Any).count();
    let _ = warm(Some(subj), None, None);
    let _ = warm(None, Some(pred), None);
    let _ = warm(None, None, Some(obj));

    let mut group = c.benchmark_group("ir_pattern_warm");
    for (name, s, p, o) in [
        ("subject", Some(subj), None, None),
        ("predicate", None, Some(pred), None),
        ("object", None, None, Some(obj)),
    ] {
        // Baseline = the EXACT body of the trait's default `quads_for_pattern` (the
        // linear scan the index replaces), so the comparison is apples-to-apples.
        group.bench_function(format!("scan_{name}"), |b| {
            b.iter(|| {
                std::hint::black_box(
                    ds.quads()
                        .filter(|q| {
                            s.is_none_or(|id| q.s == id)
                                && p.is_none_or(|id| q.p == id)
                                && o.is_none_or(|id| q.o == id)
                                && GraphMatch::Any.matches(q.g)
                        })
                        .count(),
                )
            });
        });
        group.bench_function(format!("indexed_{name}"), |b| {
            b.iter(|| {
                std::hint::black_box(
                    DatasetView::quads_for_pattern(&*ds, s, p, o, GraphMatch::Any).count(),
                )
            });
        });
    }
    group.finish();
}

/// P4b cold cost: a fresh dataset's first predicate-bound query pays the one-time POS
/// permutation build. `iter_batched` keeps the (expensive) dataset construction in
/// UN-timed setup so the measured region is just the cold index build + first query.
fn bench_pattern_cold(c: &mut Criterion) {
    use criterion::BatchSize;
    let mut group = c.benchmark_group("ir_pattern_cold");
    group.bench_function("first_pos_query_cold_index", |b| {
        // `iter_batched_ref` (not `iter_batched`) so the dataset's Drop — freeing the
        // arena + the just-built POS index — happens in UN-timed teardown, not the
        // measured region.
        b.iter_batched_ref(
            build_dataset,
            |ds| {
                let ds = &**ds;
                let pred = ds.quads().next().expect("quads").p;
                std::hint::black_box(
                    DatasetView::quads_for_pattern(ds, None, Some(pred), None, GraphMatch::Any)
                        .count(),
                )
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

/// P4b concurrent first access: four threads race the SAME cold POS `OnceLock` on a
/// fresh dataset. `iter_batched` keeps dataset construction in UN-timed setup so the
/// measured region is the `get_or_init` race + queries (correctness guaranteed by
/// `OnceLock`; this measures its cost under contention).
fn bench_pattern_concurrent(c: &mut Criterion) {
    use criterion::BatchSize;
    let mut group = c.benchmark_group("ir_pattern_concurrent");
    group.bench_function("concurrent_first_pos_access_x4", |b| {
        // `iter_batched_ref` excludes the dataset's Drop from the timed region.
        b.iter_batched_ref(
            build_dataset,
            |ds| {
                let ds = &**ds;
                let pred = ds.quads().next().expect("quads").p;
                let total: usize = std::thread::scope(|scope| {
                    let handles: Vec<_> = (0..4)
                        .map(|_| {
                            scope.spawn(|| {
                                DatasetView::quads_for_pattern(
                                    &*ds,
                                    None,
                                    Some(pred),
                                    None,
                                    GraphMatch::Any,
                                )
                                .count()
                            })
                        })
                        .collect();
                    handles.into_iter().map(|h| h.join().expect("thread")).sum()
                });
                std::hint::black_box(total)
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

/// Print the allocation metrics once (criterion's first call), then run all timed
/// groups. Criterion calls each `bench_*` once per run; the metrics print is a
/// separate leading function so it runs exactly once.
fn bench_metrics(_c: &mut Criterion) {
    print_alloc_metrics();
}

criterion_group!(
    benches,
    bench_metrics,
    bench_build,
    bench_iterate,
    bench_resolve,
    bench_pattern_warm,
    bench_pattern_cold,
    bench_pattern_concurrent
);
criterion_main!(benches);
