// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench entry points and probe helpers are private measurement tooling, not API.
#![allow(missing_docs)]

//! Report-only current-thread allocation demand for the whole public GTS import.
//!
//! The source bytes and their construction are excluded. The returned RDF bundle
//! stays alive at window close; retained output and transient peak are reported
//! separately. This sequential reader/resolver/builder probe is not a process RSS
//! or whole-process quota measurement. Input still has a resident CBOR frame and
//! the resolver still buffers a segment.
//!
//! This is a plain main because allocator atomics make timing beside the memory
//! probe unsuitable. No throughput or speedup is claimed. The test shares this
//! exact fixture, measurement and outside-window output-equivalence oracle.
//!
//! Run with cargo bench -p purrdf-rdf --bench gts_import_alloc.

use purrdf_alloc_probe::CountingAllocator;

#[path = "../tests/support/gts_import.rs"]
mod fixture;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn main() {
    let mut expected = None;
    for snapshot in [false, true] {
        for repeats in [4_096, 16_384] {
            let bytes = fixture::source(repeats, snapshot);
            let (bundle, measured) = fixture::observe(&bytes);
            fixture::check_content(&bundle, &mut expected);
            println!(
                "observation snapshot={snapshot} rows={} source_bytes={} current_thread_peak_bytes={} retained_bytes={} requested_bytes={} allocations={}",
                repeats * 3,
                bytes.len(),
                measured.peak_working_bytes,
                measured.retained_bytes,
                measured.requested_bytes,
                measured.allocations,
            );
        }
    }
}
