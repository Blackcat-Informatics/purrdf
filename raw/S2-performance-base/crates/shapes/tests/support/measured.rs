// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The measurement discipline the shapes allocation and evaluation-count tests
//! share: one lock serializing every measured region of a test binary, the
//! whole-process allocation window around a region, the minimum over repeated
//! executions, the parallel-path precondition, and the source scan that proves
//! every `#[test]` of a binary takes the lock first.

// The module is included into more than one integration-test binary, and no single binary
// uses every helper; an unused-here helper is used there.
#![allow(dead_code, unreachable_pub)]

use std::sync::{Mutex, MutexGuard, PoisonError};

use purrdf_alloc_probe::{Measurement, WholeProcessWindow};

/// Serializes every measured region in the including binary.
///
/// A [`WholeProcessWindow`] (or a process-global counter) reads one
/// process-global ledger and `cargo test` runs test functions concurrently, so
/// two measurements in flight at once would each report the union of both
/// regions while appearing to report their own.
static MEASURE_LOCK: Mutex<()> = Mutex::new(());

/// Take [`MEASURE_LOCK`], absorbing poison.
///
/// A panicking assertion inside a measured region poisons the mutex. Propagating
/// that would turn one real failure into a cascade of unrelated ones and bury the
/// diagnosis; the lock guards a counter, not an invariant that a panic could have
/// left half-written.
pub fn measure_lock() -> MutexGuard<'static, ()> {
    MEASURE_LOCK.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Run `operation` inside a whole-process allocation window.
///
/// The caller is responsible for holding [`measure_lock`] and for having warmed
/// the operation first; this helper only brackets it.
pub fn measure<T>(operation: impl FnOnce() -> T) -> (T, Measurement) {
    let window = WholeProcessWindow::open();
    let value = operation();
    (value, window.close())
}

/// How many times [`measure_min`] executes a region before keeping the smallest
/// allocation count it saw.
///
/// Three, and the three is derived rather than tuned. `rayon`'s global injector
/// queue allocates a fresh block every `crossbeam_deque` `BLOCK_CAP` = **63**
/// pushes; a change-path validation pushes one job per parallel submission, which
/// for every measured shape is a small single-digit number. Three consecutive
/// executions therefore make well under 63 pushes between them, so AT MOST ONE of
/// the three windows can straddle a block boundary and at least two of them
/// cannot. Any `k >= 2` satisfying `k * pushes_per_validation < 63` would do; the
/// third execution is margin, not calibration, and no value of it can hide a
/// per-focus-node term, because a term that is present is present in all three.
pub const REPETITIONS: usize = 3;

/// At least two executions, or the property the constant is chosen for — that one
/// of them must miss the block boundary — is not available at all.
const _: () = assert!(
    REPETITIONS >= 2,
    "a single execution cannot exclude the injector's block allocation"
);

/// Execute a measured region [`REPETITIONS`] times and keep the smallest.
///
/// This is the same kind of instrument as the warm-up calls beside it, aimed at a
/// different once-in-a-while cost. A warm-up removes first-touch work by making
/// sure it has already happened; this removes `rayon`'s injector block allocation
/// by making sure at least one execution falls between two of them. Both remove a
/// cost that is NOT the measured code's, and neither changes what is compared: the
/// figures that come out are still exact allocation counts, still compared with
/// `==`, and still fail on a difference of one.
///
/// What it deliberately is not is a tolerance. A tolerance would let a real
/// per-focus-node term of the same magnitude through; a minimum cannot, because
/// such a term is charged to every execution and so to the minimum as well. It
/// also cannot mask a term that is merely intermittent in the CODE — the smallest
/// figure is still a figure the code really produced.
///
/// The value returned is the one produced by the execution the reported
/// measurement came from, so a caller's assertions about the report and its
/// assertions about the count describe the same run.
pub fn measure_min<T>(mut operation: impl FnMut() -> T) -> (T, Measurement) {
    let mut best: Option<(T, Measurement)> = None;
    for _ in 0..REPETITIONS {
        let (value, measured) = measure(&mut operation);
        if best
            .as_ref()
            .is_none_or(|(_, seen)| measured.allocations < seen.allocations)
        {
            best = Some((value, measured));
        }
    }
    best.expect("REPETITIONS is non-zero, so at least one execution was measured")
}

/// Refuse to report a figure from a run where the parallel path cannot be taken.
///
/// The sizes under measurement are above the parallel threshold at compile time,
/// but a single-threaded `rayon` pool keeps validation serial whatever the sizes
/// are, and a serial figure asserted under a parallel test name is the "green
/// because it measured almost nothing" failure these instruments exist to avoid.
/// That is a property of the host, so it is checked at run time.
pub fn assert_parallel_path_is_reachable() {
    assert!(
        rayon::current_num_threads() > 1,
        "SHACL validation stays serial on a single-threaded rayon pool, so this host cannot \
         exercise the parallel path these assertions are written about"
    );
}

/// Whether `attrs` carries a bare `#[test]` attribute.
///
/// Matches by attribute PATH, not by scanning the source text for the word
/// "test": a doc comment or a code comment that happens to contain that word
/// must never be read as marking a function.
fn is_test_attr(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("test"))
}

/// Whether `block`'s FIRST statement is a `let` binding whose initializer is a
/// call to `measure_lock()`.
///
/// Not "somewhere in the body": a process-global ledger is read for the whole
/// test, so a lock taken after even one allocation has already let that
/// allocation land unguarded. It must also be a `let` binding and not a bare
/// `measure_lock();` statement — the returned [`MutexGuard`] is a temporary that
/// drops at the end of a bare statement, which releases the lock immediately
/// rather than holding it for the test.
fn first_statement_holds_measure_lock(block: &syn::Block) -> bool {
    let Some(syn::Stmt::Local(local)) = block.stmts.first() else {
        return false;
    };
    let Some(init) = &local.init else {
        return false;
    };
    matches!(
        init.expr.as_ref(),
        syn::Expr::Call(call)
            if matches!(
                call.func.as_ref(),
                syn::Expr::Path(path) if path.path.is_ident("measure_lock")
            )
    )
}

/// Every `#[test]` function declared anywhere in the scanned file, in source
/// order.
///
/// Walks the whole file rather than only its top-level items, so a `#[test]`
/// nested inside a `mod` block cannot go unseen.
#[derive(Default)]
struct TestFns(Vec<syn::ItemFn>);

impl<'ast> syn::visit::Visit<'ast> for TestFns {
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        if is_test_attr(&item.attrs) {
            self.0.push(item.clone());
        }
        syn::visit::visit_item_fn(self, item);
    }
}

/// The `#[test]` functions of `source` (a test binary's own text), and the
/// sorted names of those whose body does not take [`measure_lock`] as its first
/// statement.
///
/// A source scan rather than a runtime check: nothing observable at runtime
/// distinguishes "this test forgot to take the lock" from "this test never
/// needed it", so the only place the distinction is visible is the source
/// itself.
pub fn tests_and_unlocked(source: &str) -> (usize, Vec<String>) {
    let parsed = syn::parse_file(source)
        .unwrap_or_else(|error| panic!("this file must parse as Rust: {error}"));
    let mut collector = TestFns::default();
    syn::visit::Visit::visit_file(&mut collector, &parsed);
    let mut offenders: Vec<String> = collector
        .0
        .iter()
        .filter(|item| !first_statement_holds_measure_lock(&item.block))
        .map(|item| item.sig.ident.to_string())
        .collect();
    offenders.sort();
    (collector.0.len(), offenders)
}
