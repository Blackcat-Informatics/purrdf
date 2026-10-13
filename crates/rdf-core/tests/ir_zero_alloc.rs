// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Gate 1 (C1): `RdfDataset::quads()` performs **zero allocations** and never
//! clones or formats a term, and `quad_refs()` resolves terms without allocating.
//!
//! The proof is operational, not a slogan: this test installs the workspace's
//! shared counting allocator and brackets the iteration loop in a measurement
//! window, then asserts the window saw exactly `0` allocations. Because the
//! allocator is `#[global_allocator]`, it observes every heap allocation any code
//! in the loop body would make — there is nowhere for a hidden allocation to hide.

#[path = "support/borrowed.rs"]
mod borrowed;
use borrowed::term_ref_len;
use std::hash::{Hash, Hasher};

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_core::{BlankScope, QuadIds, QuadRef, RdfDatasetBuilder, RdfLiteral};

// A CURRENT-THREAD window, not a whole-process one: `cargo test` runs every test in
// the binary concurrently on separate threads sharing one process and one
// `#[global_allocator]`, so a process-wide counter would be contaminated by a
// sibling test thread's allocations between the before/after snapshots. The
// per-thread window counts only the measuring thread's own allocations, isolating
// the measurement regardless of how the harness schedules tests. Nothing measured
// here fans out over worker threads, which is the one situation in which that
// choice would be the wrong one.
#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

#[test]
fn destructive_term_drop_allocates_no_traversal_storage() {
    use purrdf_core::{TermBox, TermValue};
    let leaf = || TermValue::iri("http://example.org/leaf");
    let mut chain = leaf();
    for depth in 0..5000 {
        let [s, p, o] = match depth % 3 {
            0 => [chain, leaf(), leaf()],
            1 => [leaf(), chain, leaf()],
            _ => [leaf(), leaf(), chain],
        };
        chain = TermValue::Triple {
            s: TermBox::new(s),
            p: TermBox::new(p),
            o: TermBox::new(o),
        };
    }
    let mut branching = leaf();
    for _ in 0..7 {
        branching = TermValue::Triple {
            s: TermBox::new(branching.clone()),
            p: TermBox::new(branching.clone()),
            o: TermBox::new(branching),
        };
    }
    let leaf_bytes = "http://example.org/leaf".len();
    let branch_leaves = 3usize.pow(7);
    let branch_boxes = 3 * (branch_leaves - 1) / 2;
    let expected =
        (5000 * 3 + branch_boxes) * size_of::<TermValue>() + (10001 + branch_leaves) * leaf_bytes;
    assert_eq!(size_of::<TermBox>(), 2 * size_of::<Box<TermValue>>());
    let window = CurrentThreadWindow::open();
    drop(chain);
    drop(branching);
    let measured = window.close();
    assert_eq!(
        measured.allocations, 0,
        "destructive drop must remain possible after allocator refusal"
    );
    assert_eq!(measured.requested_bytes, 0);
    assert_eq!(
        measured.retained_bytes,
        -i64::try_from(expected).expect("small fixture layout"),
        "every original box and lexical allocation is freed exactly once"
    );
}

#[test]
fn destructive_drop_resumes_uneven_leaf_tails_without_allocation() {
    use purrdf_core::{TermBox, TermValue};
    let mut nodes = Vec::new();
    let mut expected = 0usize;
    for position in 0..3 {
        let mut term = TermValue::iri("tail");
        expected += 4;
        for _ in 0..(position + 1) * 97 {
            let [s, p, o] = match position {
                0 => [term, TermValue::iri("p"), TermValue::iri("o")],
                1 => [TermValue::iri("s"), term, TermValue::iri("o")],
                _ => [TermValue::iri("s"), TermValue::iri("p"), term],
            };
            term = TermValue::Triple {
                s: TermBox::new(s),
                p: TermBox::new(p),
                o: TermBox::new(o),
            };
            expected += 3 * size_of::<TermValue>() + 2;
        }
        nodes.push(term);
    }
    expected += nodes.capacity() * size_of::<TermValue>();
    let window = CurrentThreadWindow::open();
    drop(nodes);
    let measured = window.close();
    assert_eq!(measured.allocations, 0);
    assert_eq!(measured.requested_bytes, 0);
    assert_eq!(
        measured.retained_bytes,
        -i64::try_from(expected).expect("small fixture layout")
    );
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
fn fold<H: Hash>(hasher: &mut purrdf_hash::fixed::FixedHasher, value: &H) {
    value.hash(hasher);
}

#[test]
fn quads_iteration_allocates_zero() {
    let ds = build_dataset();
    assert!(ds.quad_count() >= 64, "non-trivial dataset");

    // Warm any lazy one-time state outside the measured window (there is none today,
    // but this keeps the measurement robust to future internal lazies).
    let mut warm = purrdf_hash::fixed::FixedHasher::default();
    for q in ds.quads() {
        fold(&mut warm, &q);
    }
    std::hint::black_box(warm.finish());

    let window = CurrentThreadWindow::open();
    let mut hasher = purrdf_hash::fixed::FixedHasher::default();
    for q in ds.quads() {
        // Consume the Copy QuadIds purely by value — no formatting, no clone, no
        // heap. Hashing a Copy struct is entirely on the stack.
        let q: QuadIds = q;
        fold(&mut hasher, &q);
    }
    let measured = window.close();
    std::hint::black_box(hasher.finish());

    assert_eq!(
        measured.allocations, 0,
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

    let window = CurrentThreadWindow::open();
    let mut acc: usize = 0;
    for q in ds.quad_refs() {
        // Resolve every position to a borrowed view and touch borrowed &str content
        // without copying it (sum byte lengths) — proves no allocation on resolve.
        acc = acc.wrapping_add(quad_ref_len(&q));
    }
    let measured = window.close();
    std::hint::black_box(acc);

    assert_eq!(
        measured.allocations, 0,
        "RdfDataset::quad_refs() must resolve terms without allocating (Gate 1)"
    );
}

/// Sum the borrowed `&str` lengths reachable from a `QuadRef` without copying any of
/// them — exercises the resolved view of every position.
fn quad_ref_len(q: &QuadRef<'_>) -> usize {
    term_ref_len(q.s) + term_ref_len(q.p) + term_ref_len(q.o) + q.g.map_or(0, term_ref_len)
}

/// Exercise the generic contract, including its pinned rows and both batch paths.
fn resident_generic_reads<D>(
    view: &D,
    ids: &[purrdf_core::TermId],
    values: &[purrdf_core::TermValue],
) -> usize
where
    D: purrdf_core::DatasetView<Id = purrdf_core::TermId, ReadError = std::convert::Infallible>,
{
    use purrdf_core::dataset_view::{TermGuard as _, WorkspaceReservation as _};
    let mut total = 0usize;
    let mut reservation = view.reserve_workspace(0).unwrap();
    reservation.resize(u64::MAX).unwrap();
    for id in ids {
        let guard = view.resolve(*id).unwrap();
        total = total.wrapping_add(term_ref_len(guard.term()));
    }
    view.resolve_batch(ids, |_, term| {
        total = total.wrapping_add(term_ref_len(term));
    })
    .unwrap();
    for value in values {
        total = total.wrapping_add(usize::from(view.term_id_by_value(value).unwrap().is_some()));
    }
    view.lookup_batch(values, |_, id| {
        total = total.wrapping_add(usize::from(id.is_some()));
    })
    .unwrap();
    for row in view.quad_refs() {
        let row = row.unwrap();
        total = total.wrapping_add(quad_ref_len(&row.as_ref()));
    }
    reservation.resize(0).unwrap();
    total
}

#[test]
fn generic_resident_guards_batches_and_workspace_allocate_zero() {
    use purrdf_core::{DatasetView, TermId, TermRef, TermValue};
    // This assignment also proves the resident GAT is still the borrowed TermRef.
    fn borrowed_guard(view: &purrdf_core::RdfDataset, id: TermId) -> TermRef<'_> {
        DatasetView::resolve(view, id).unwrap()
    }
    assert_eq!(size_of::<TermId>(), 4);
    assert_eq!(size_of::<QuadIds>(), 16);
    let dataset = build_dataset();
    let first = dataset.quads().next().unwrap();
    let ids = [first.s, first.p, first.o];
    let values = [
        TermValue::iri("http://example.org/p"),
        TermValue::integer(7),
        TermValue::blank("b0"),
        TermValue::iri("http://example.org/absent"),
    ];
    assert!(dataset.as_ref().term_id_by_value(&values[0]).is_some());
    assert!(dataset.as_ref().term_id_by_value(&values[3]).is_none());
    std::hint::black_box(borrowed_guard(dataset.as_ref(), first.s));
    let expected = resident_generic_reads(dataset.as_ref(), &ids, &values);
    assert!(expected > 0);
    let window = CurrentThreadWindow::open();
    let actual = resident_generic_reads(dataset.as_ref(), &ids, &values);
    let measured = window.close();
    assert_eq!(actual, expected);
    assert_eq!(
        measured.allocations, 0,
        "resident generic reads must allocate zero"
    );
}
