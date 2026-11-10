// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Allocation costs of public freeze, validated materialization and build plus
//! freeze, using the timing benchmark's exact fixture in a separate process.
//! Set `PURRDF_FREEZE_BENCH_ROWS` to select the population. An optional `default`
//! or `named` argument selects one graph layout; both run otherwise.
//! Measurements are report-only. Output stays alive until the window closes;
//! fixture checks, reporting and output destruction are outside each window.

use std::hint::black_box;
use std::sync::Arc;

use purrdf_alloc_probe::{CountingAllocator, WholeProcessWindow};
use purrdf_core::RdfDataset;

#[path = "support/freeze_fixture.rs"]
mod freeze_fixture;
use freeze_fixture::{fixture, row_count};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn measure(label: &str, rows: usize, graphs: usize, operation: impl FnOnce() -> Arc<RdfDataset>) {
    let window = WholeProcessWindow::open();
    let dataset = black_box(operation());
    let measured = window.close();
    assert_eq!(dataset.quad_count(), rows);
    assert_eq!(dataset.term_count(), rows * 187 / 1170);
    assert_eq!(dataset.named_graphs().count(), graphs);
    println!(
        "[freeze_alloc] {label}: rows={rows} graphs={graphs} allocations={} \
         requested_bytes={} retained_bytes={} peak_working_bytes={}",
        measured.allocations,
        measured.requested_bytes,
        measured.retained_bytes,
        measured.peak_working_bytes,
    );
}

fn main() {
    let selected = std::env::args().skip(1).find(|arg| arg != "--bench");
    assert!(
        selected
            .as_deref()
            .is_none_or(|name| matches!(name, "default" | "named")),
        "optional fixture selection must be default or named",
    );
    let rows = row_count();
    for (name, graphs) in [("default", 0), ("named", 117)] {
        if selected.as_deref().is_some_and(|selected| selected != name) {
            continue;
        }
        let builder = fixture(rows, graphs);
        measure(&format!("freeze_{name}"), rows, graphs, || {
            builder.freeze().expect("fixture freezes")
        });
        let builder = fixture(rows, graphs).validate().expect("fixture validates");
        assert_eq!(builder.statement_count(), rows);
        assert_eq!(builder.term_count(), rows * 187 / 1170);
        measure(&format!("materialize_{name}"), rows, graphs, || {
            builder.freeze().expect("validated fixture materializes")
        });
        measure(&format!("build_freeze_{name}"), rows, graphs, || {
            fixture(rows, graphs).freeze().expect("fixture freezes")
        });
    }
}
