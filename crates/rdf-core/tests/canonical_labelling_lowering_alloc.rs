// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The lowering of nested triple terms costs memory linear in the input.
//!
//! A chain `t(1) = <<a a a>>`, `t(k + 1) = <<a a t(k)>>` with a row
//! `quad(a, a, t(k))` for each term has written forms whose total size is
//! quadratic in its length. Lowered with term nodes, it is `n` lines and
//! `n - 1` records of bounded size, so the bytes requested and retained and the
//! allocations made while lowering grow linearly.

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow, Measurement};
use purrdf_core::canonical_labelling::{
    Limit, Scope, ViewError, label_view_with_memory, lines_of_view,
};
use purrdf_lex::allocation::{Ceiling, Memory, StorageError};

#[path = "support/nested_chain.rs"]
mod nested_chain;
use nested_chain::chain;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

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
    let retained =
        |m: &Measurement| u64::try_from(m.retained_bytes).expect("lowering retains its lines");
    assert!(
        within(retained(&small), retained(&large)),
        "retained bytes {small:?} then {large:?}"
    );
}

/// The working state a nested input keeps is admitted through the caller's
/// memory: the bytes the labelling of the chain still holds when it returns are
/// within its grant, the grant labels the chain at its exact size, and one byte
/// less refuses with the typed storage outcome.
#[test]
fn the_state_a_nested_input_keeps_is_within_its_grant() {
    let view = chain(1_000);
    let label = |memory: &mut Memory<'_, _>| {
        label_view_with_memory(
            view.as_ref(),
            &[],
            Scope::Dataset,
            Limit::ENGINE,
            None,
            memory,
        )
    };
    let mut open = Ceiling(usize::MAX);
    let window = CurrentThreadWindow::open();
    let (labelling, granted) = label(&mut Memory::new(&mut open)).expect("labelled");
    let measurement = window.close();
    let retained = usize::try_from(measurement.retained_bytes).expect("the labelling is held");
    assert!(
        retained <= granted,
        "retained {retained} bytes beyond the grant of {granted}"
    );
    assert_eq!(labelling.lines().len(), 1_999);
    drop(labelling);

    let mut at_grant = Ceiling(granted);
    assert!(label(&mut Memory::new(&mut at_grant)).is_ok());
    let mut under = Ceiling(granted - 1);
    assert_eq!(
        label(&mut Memory::new(&mut under)).err(),
        Some(ViewError::Storage(StorageError::AdmissionFailed))
    );
}
