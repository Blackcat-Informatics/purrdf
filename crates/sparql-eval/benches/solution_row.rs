// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Micro-benchmark isolating the per-row machinery of a [`Solution`] — the
//! inline-`SmallVec` row that the structural pass switched from `Vec` to
//! `SmallVec<[Option<SolutionTerm>; 4]>` (inline capacity 4, the distribution
//! mode: most queries bind ≤4-8 variables).
//!
//! Where `query_eval.rs` measures whole-query evaluation end-to-end, this bench
//! drives ONLY the row operations that dominate BGP/join execution, on the
//! crate's public `Solution`/`compatible` surface — so the two regimes the change
//! targets are each visible in isolation:
//! - the **inline** regime (≤4 bound columns): the row lives on the stack with no
//!   heap allocation;
//! - the **spilled** regime (>4 bound columns): the row spills to the heap, the
//!   `Vec`-equivalent path.
//!
//! Each timed closure loops over a batch of synthetic rows so the row machinery —
//! not per-sample setup — is what is measured. The `merge` case mirrors the join
//! output-row construction in `binop.rs` (one exact-size row seeded from the left
//! prefix, then the right's non-shared cells filled in) through the public API, so
//! it is not reaching into private internals.
//!
//! Report-only, `cargo bench -p purrdf-sparql-eval --bench solution_row` (the
//! `make bench` lane) — excluded from `make check`. It asserts no threshold: the
//! machine is not quiet, so the numbers are evidence, never a pass/fail gate.

use criterion::{Criterion, black_box, criterion_group, criterion_main};

use purrdf_core::TermId;
use purrdf_sparql_eval::{Solution, SolutionTerm, compatible};

/// Batch size: rows per timed iteration, so the row machinery (not the closure
/// call) dominates each sample.
const BATCH: usize = 1_024;

/// Inline width — fits the `SmallVec<[_; 4]>` inline buffer with no heap alloc.
const INLINE_COLS: usize = 4;

/// Spilled width — exceeds inline capacity 4, so the row heap-allocates (the
/// former `Vec` path).
const SPILLED_COLS: usize = 9;

/// A distinct `Existing` term for synthetic index `i`.
fn term(i: usize) -> SolutionTerm {
    SolutionTerm::Existing(TermId::from_index(
        u32::try_from(i).expect("index fits u32"),
    ))
}

/// A fully-bound row of `cols` distinct terms, seeded from `base` so successive
/// rows differ (defeats any constant-folding of the payload).
fn row(base: usize, cols: usize) -> Solution {
    (0..cols).map(|c| Some(term(base + c))).collect()
}

/// A batch of `BATCH` fully-bound rows of the given width.
fn rows(cols: usize) -> Vec<Solution> {
    (0..BATCH).map(|i| row(i, cols)).collect()
}

/// Mirror of `binop::merge`: one exact-size output row seeded from the left
/// prefix, resized to the output width, then the right row's non-shared cells
/// filled into their output positions if still unbound. Kept on the public
/// `Solution` API so the bench compiles against the crate's external surface.
fn merge(left: &Solution, right: &Solution, right_to_out: &[usize], out_len: usize) -> Solution {
    let mut merged = Solution::with_capacity(out_len);
    merged.extend_from_slice(left);
    merged.resize(out_len, None);
    for (j, &cell) in right.iter().enumerate() {
        let p = right_to_out[j];
        if merged[p].is_none() {
            merged[p] = cell;
        }
    }
    merged
}

/// Construction: build a fresh fully-bound row of `cols` terms, `BATCH` times.
fn bench_construct(c: &mut Criterion, label: &str, cols: usize) {
    c.bench_function(label, |bencher| {
        bencher.iter(|| {
            for i in 0..BATCH {
                black_box(row(black_box(i), black_box(cols)));
            }
        });
    });
}

/// Clone/extend: clone an existing row (the copy every join/OPTIONAL output row
/// pays), then push one more binding onto the copy.
fn bench_clone_extend(c: &mut Criterion, label: &str, cols: usize) {
    let batch = rows(cols);
    c.bench_function(label, |bencher| {
        bencher.iter(|| {
            for r in &batch {
                let mut copy = black_box(r).clone();
                copy.push(Some(term(cols)));
                black_box(copy);
            }
        });
    });
}

fn bench_solution_row(c: &mut Criterion) {
    // Row construction — the two regimes the SmallVec change straddles.
    bench_construct(c, "construct_inline_4col", INLINE_COLS);
    bench_construct(c, "construct_spilled_9col", SPILLED_COLS);

    // Clone + extend — the per-output-row cost of a join/OPTIONAL, both regimes.
    bench_clone_extend(c, "clone_extend_inline_4col", INLINE_COLS);
    bench_clone_extend(c, "clone_extend_spilled_9col", SPILLED_COLS);

    // Bind a single position on an already-materialized row (the per-cell write
    // in BGP unification and BIND).
    let mut bind_batch = rows(INLINE_COLS);
    c.bench_function("bind_position_inline", |bencher| {
        bencher.iter(|| {
            for (i, r) in bind_batch.iter_mut().enumerate() {
                r[i % INLINE_COLS] = Some(term(i));
                black_box(&*r);
            }
        });
    });

    // Compatibility probe: the join predicate over the shared columns. Left rows
    // share their first two columns with the right rows (indices pair 1:1), and
    // every other pair matches on one column and differs on the other, so both the
    // equal and unequal branches are exercised.
    let left = rows(INLINE_COLS);
    let right = rows(INLINE_COLS);
    let shared: &[(usize, usize)] = &[(0, 0), (1, 1)];
    c.bench_function("compatible_probe_inline", |bencher| {
        bencher.iter(|| {
            for (l, r) in left.iter().zip(&right) {
                black_box(compatible(black_box(l), black_box(r), black_box(shared)));
            }
        });
    });

    // Join output materialization mirroring `binop::merge`. Inline: two 4-col rows
    // sharing their first two columns → a 6-col output (spills), and a narrow 3-col
    // variant that stays inline, so both output regimes are visible.
    let l_wide = rows(INLINE_COLS);
    let r_wide = rows(INLINE_COLS);
    // right cols 0,1 are shared (map onto left's 0,1); cols 2,3 append at 4,5.
    let right_to_out_wide: &[usize] = &[0, 1, 4, 5];
    c.bench_function("merge_join_spilled_6col", |bencher| {
        bencher.iter(|| {
            for (l, r) in l_wide.iter().zip(&r_wide) {
                black_box(merge(
                    black_box(l),
                    black_box(r),
                    black_box(right_to_out_wide),
                    6,
                ));
            }
        });
    });

    let l_narrow: Vec<Solution> = (0..BATCH).map(|i| row(i, 2)).collect();
    let r_narrow: Vec<Solution> = (0..BATCH).map(|i| row(i, 2)).collect();
    // right col 0 shared (onto left's 0), col 1 appends at 2 → a 3-col inline output.
    let right_to_out_narrow: &[usize] = &[0, 2];
    c.bench_function("merge_join_inline_3col", |bencher| {
        bencher.iter(|| {
            for (l, r) in l_narrow.iter().zip(&r_narrow) {
                black_box(merge(
                    black_box(l),
                    black_box(r),
                    black_box(right_to_out_narrow),
                    3,
                ));
            }
        });
    });
}

criterion_group!(benches, bench_solution_row);
criterion_main!(benches);
