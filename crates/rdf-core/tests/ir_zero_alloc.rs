// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Gate 1 (C1): `RdfDataset::quads()` performs **zero allocations** and never
//! clones or formats a term, and `quad_refs()` resolves terms without allocating.
//!
//! The proof is operational, not a slogan: this test installs a process-global
//! counting allocator that increments an atomic counter on every `alloc` /
//! `realloc`. It snapshots the counter immediately before the iteration loop and
//! immediately after, and asserts the delta is exactly `0`. Because the allocator is
//! `#[global_allocator]`, it observes every heap allocation any code in the loop body
//! would make — there is nowhere for a hidden allocation to hide.

// Rich colored line-diffs on assert_eq! failure; shadows the std macro
// for this file. Identical behaviour on pass; insta snapshots are unaffected.
use pretty_assertions::assert_eq;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::hash::{Hash, Hasher};

use purrdf_core::{BlankScope, QuadIds, QuadRef, RdfDatasetBuilder, RdfLiteral, TermRef};

// A THREAD-LOCAL allocation counter, not a process-global atomic: `cargo test` runs
// every test in the binary concurrently on separate threads sharing one process and
// one `#[global_allocator]`, so a process-global counter would be contaminated by a
// sibling test thread's allocations between the before/after snapshots. A thread-local
// `Cell<usize>` counts only the measuring thread's own allocations, isolating the
// measurement regardless of how the harness schedules tests. (`Cell<usize>` is `Copy`
// and never heap-allocates, so the counter itself adds no allocations.)
thread_local! {
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

/// A pass-through allocator that counts allocations on the current thread.
/// Deallocations are ignored: we only care that the hot iteration path allocates
/// nothing new.
struct CountingAllocator;

/// Record one allocation on the current thread, tolerating TLS-init re-entrancy by
/// silently skipping the count when the thread-local is not yet available.
fn bump() {
    let _ = ALLOCATIONS.try_with(|c| c.set(c.get() + 1));
}

// SAFETY: every method forwards to the system allocator with the same layout; the
// only added behavior is a thread-local counter increment on allocation paths.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe {
            bump();
            System.alloc(layout)
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe {
            System.dealloc(ptr, layout);
        }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        unsafe {
            bump();
            System.realloc(ptr, layout, new_size)
        }
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

/// Snapshot of the current thread's allocation counter.
fn allocations() -> usize {
    ALLOCATIONS.with(Cell::get)
}

/// Build a non-trivial frozen dataset: many quads across the default graph and named
/// graphs, with IRIs, blanks, literals (typed + language-tagged + directional), and a
/// nested triple term — so the iteration genuinely resolves every term variant.
fn build_dataset() -> std::sync::Arc<purrdf_core::RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri("http://example.org/p");
    let g = b.intern_iri("http://example.org/g");

    for n in 0..64 {
        let s = b.intern_iri(&format!("http://example.org/s{n}"));
        let bnode = b.intern_blank(&format!("b{n}"), BlankScope(n % 3));
        let lit = b.intern_literal(RdfLiteral::language_tagged(format!("v{n}"), "EN"));
        let typed = b.intern_literal(RdfLiteral::typed(
            format!("{n}"),
            "http://www.w3.org/2001/XMLSchema#integer",
        ));
        b.push_quad(s, p, bnode, None);
        b.push_quad(s, p, lit, Some(g));
        b.push_quad(bnode, p, typed, None);
        // A nested triple term as object, exercising the Triple arm of resolve().
        let inner = b.intern_triple(s, p, typed);
        b.push_quad(s, p, inner, Some(g));
    }

    b.freeze().expect("dataset is structurally valid")
}

/// Fold a `Hash` value into a stack-allocated hasher — observes the value fully
/// without copying any owned/heap data out of the dataset.
fn fold<H: Hash>(hasher: &mut std::collections::hash_map::DefaultHasher, value: &H) {
    value.hash(hasher);
}

#[test]
fn quads_iteration_allocates_zero() {
    let ds = build_dataset();
    assert!(ds.quad_count() >= 64, "non-trivial dataset");

    // Warm any lazy one-time state outside the measured window (there is none today,
    // but this keeps the measurement robust to future internal lazies).
    let mut warm = std::collections::hash_map::DefaultHasher::new();
    for q in ds.quads() {
        fold(&mut warm, &q);
    }
    std::hint::black_box(warm.finish());

    let before = allocations();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for q in ds.quads() {
        // Consume the Copy QuadIds purely by value — no formatting, no clone, no
        // heap. Hashing a Copy struct is entirely on the stack.
        let q: QuadIds = q;
        fold(&mut hasher, &q);
    }
    let after = allocations();
    std::hint::black_box(hasher.finish());

    assert_eq!(
        after - before,
        0,
        "RdfDataset::quads() must perform zero allocations (Gate 1)"
    );
}

#[test]
fn quad_refs_resolution_allocates_zero() {
    let ds = build_dataset();

    // Warm.
    let mut warm = 0usize;
    for q in ds.quad_refs() {
        warm = warm.wrapping_add(quad_ref_len(&q));
    }
    std::hint::black_box(warm);

    let before = allocations();
    let mut acc: usize = 0;
    for q in ds.quad_refs() {
        // Resolve every position to a borrowed view and touch borrowed &str content
        // without copying it (sum byte lengths) — proves no allocation on resolve.
        acc = acc.wrapping_add(quad_ref_len(&q));
    }
    let after = allocations();
    std::hint::black_box(acc);

    assert_eq!(
        after - before,
        0,
        "RdfDataset::quad_refs() must resolve terms without allocating (Gate 1)"
    );
}

/// Sum the borrowed `&str` lengths reachable from a `QuadRef` without copying any of
/// them — exercises the resolved view of every position.
fn quad_ref_len(q: &QuadRef<'_>) -> usize {
    term_ref_len(q.s) + term_ref_len(q.p) + term_ref_len(q.o) + q.g.map_or(0, term_ref_len)
}

/// Touch a borrowed term's `&str` content without copying it — returns a length so
/// the borrow is genuinely observed by the optimizer. Triple-term components are
/// ids (no borrowed string content), so they contribute nothing here.
fn term_ref_len(t: TermRef<'_>) -> usize {
    match t {
        TermRef::Iri(s) => s.len(),
        TermRef::Blank { label, .. } => label.len(),
        TermRef::Literal {
            lexical, language, ..
        } => lexical.len() + language.map_or(0, str::len),
        TermRef::Triple { .. } => 0,
    }
}
