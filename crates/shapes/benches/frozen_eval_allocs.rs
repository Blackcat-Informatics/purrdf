// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API, so the workspace `missing_docs` lint is
// not asked of their items.
#![allow(missing_docs)]

//! Allocations per row of the frozen-dataset workload `frozen_eval_counts` counts
//! instructions for.
//!
//! `cargo bench -p purrdf-shapes --bench frozen_eval_allocs [-- <id filter>]`.
//! The rows, fixture and sizes are the counting bench's own. Each row is verified
//! first (rows of one query or shape set must agree), then built afresh, run once
//! unmeasured when it is repeatable, and measured once in a whole-process window,
//! so worker threads are included. The counting allocator lives in its own
//! binary because it adds work to every allocation the instruction counts would
//! otherwise include. Output lines are
//! `allocs <quads> <row id> allocations=<n> requested_bytes=<n> retained_bytes=<n> witness=<count>`.

use purrdf_alloc_probe::{CountingAllocator, WholeProcessWindow};

#[path = "support/frozen_eval.rs"]
mod fixture;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn main() {
    use fixture::{Detail, Fixtures};
    let args: Vec<String> = std::env::args().skip(1).collect();
    let filters: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    for quads in fixture::sizes("PURRDF_FROZEN_EVAL_QUADS") {
        let fx = Fixtures::new(quads);
        let rows = fixture::rows();
        let verified = fixture::verify(&fx, &rows);
        for (row, (id, expected)) in rows.iter().zip(&verified) {
            if !filters.is_empty() && !filters.iter().any(|f| id.contains(f.as_str())) {
                continue;
            }
            let mut op = fixture::op(row, &fx);
            if row.spec.repeatable() {
                let _ = op.run(Detail::Count);
            }
            // The global pool's workers exist before the window opens.
            rayon::broadcast(|_| ());
            let window = WholeProcessWindow::open();
            let witness = op.run(Detail::Count);
            let measured = window.close();
            if !id.starts_with("shacl_bind/") {
                assert_eq!(
                    witness.count, expected.count,
                    "{id}: the measured run did different work from the verified run"
                );
            }
            println!(
                "allocs {quads} {id} allocations={} requested_bytes={} retained_bytes={} witness={}",
                measured.allocations,
                measured.requested_bytes,
                measured.retained_bytes,
                expected.count
            );
        }
    }
}
