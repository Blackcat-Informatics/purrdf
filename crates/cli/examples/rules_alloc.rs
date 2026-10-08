// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Allocation observer for the actual CLI entry point. Run with ordinary CLI argv;
//! `PURRDF_RULES_ALLOC_RECEIPT` names a mandatory separate receipt file. This
//! instrument is deliberately separate from production latency measurements.

use purrdf_alloc_probe::{CountingAllocator, WholeProcessWindow};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn main() {
    let receipt = std::env::var_os("PURRDF_RULES_ALLOC_RECEIPT")
        .expect("PURRDF_RULES_ALLOC_RECEIPT must name the allocation receipt");
    let window = WholeProcessWindow::open();
    purrdf_cli::run();
    let measured = window.close();
    std::fs::write(receipt, format!(
        "allocation_calls\t{}\nrequested_bytes\t{}\nretained_bytes\t{}\npeak_working_bytes\t{}\n",
        measured.allocations, measured.requested_bytes, measured.retained_bytes,
        measured.peak_working_bytes,
    )).expect("write allocation receipt after successful actual CLI dispatch");
}
