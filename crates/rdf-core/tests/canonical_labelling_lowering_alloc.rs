// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The lowering of nested triple terms costs memory linear in the input.
//!
//! A chain `t(1) = <<a a a>>`, `t(k + 1) = <<a a t(k)>>` with a row
//! `quad(a, a, t(k))` for each term has written forms whose total size is
//! quadratic in its length. Lowered with term nodes, it is `n` lines and
//! `n - 1` records of bounded size, so the bytes requested and retained and the
//! allocations made while lowering grow linearly.

use std::sync::Arc;

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow, Measurement};
use purrdf_core::canonical_labelling::lines_of_view;
use purrdf_core::{RdfDataset, RdfDatasetBuilder};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

/// The chain of `n` nested triple terms, one row for each.
fn chain(n: usize) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let a = builder.intern_iri("http://example.org/a").expect("interns");
    let mut term = builder.intern_triple(a, a, a).expect("interns");
    builder.push_quad(a, a, term, None).expect("pushes");
    for _ in 1..n {
        term = builder.intern_triple(a, a, term).expect("interns");
        builder.push_quad(a, a, term, None).expect("pushes");
    }
    builder.freeze().expect("freezes")
}

/// The allocation of lowering the chain of `n` terms, the lowered lines held
/// until the window closes.
fn lowering(n: usize) -> Measurement {
    let view = chain(n);
    let window = CurrentThreadWindow::open();
    let lines = lines_of_view(view.as_ref(), &[]).expect("lowered");
    let measurement = window.close();
    assert_eq!(lines.len(), 2 * n - 1, "n lines and n - 1 records");
    drop(lines);
    measurement
}

/// Ten times the chain costs at most twelve times the bytes requested, the
/// bytes retained and the allocations: linear growth with room for the
/// doubling of growable tables, where the written forms would cost a hundred
/// times.
#[test]
fn lowering_a_chain_of_nested_terms_costs_linear_memory() {
    let small = lowering(1_000);
    let large = lowering(10_000);
    let within = |small: u64, large: u64| large <= small.saturating_mul(12);
    assert!(
        within(small.requested_bytes, large.requested_bytes),
        "requested bytes {small:?} then {large:?}"
    );
    assert!(
        within(small.allocations, large.allocations),
        "allocations {small:?} then {large:?}"
    );
    let retained = |m: &Measurement| u64::try_from(m.retained_bytes).unwrap_or(0);
    assert!(
        within(retained(&small), retained(&large)),
        "retained bytes {small:?} then {large:?}"
    );
}
