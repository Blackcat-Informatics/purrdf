// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Whole public-import residency with many wire rows and a small RDF 1.2 result.

use purrdf_alloc_probe::CountingAllocator;

#[path = "support/gts_import.rs"]
mod fixture;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

#[test]
fn large_row_frames_import_without_duplicate_payload_residency() {
    let mut expected = None;
    for snapshot in [false, true] {
        for repeats in [4_096, 16_384] {
            let bytes = fixture::source(repeats, snapshot);
            let (bundle, measured) = fixture::observe(&bytes);
            fixture::check_content(&bundle, &mut expected);
            // At both scales the public call measured below 280 bytes per wire
            // row. Both former paths exceeded 416 bytes per wire row.
            // This budget separates those regimes with room for fixed import
            // overhead; it does not pin CBOR or private resolver layouts.
            let peak_budget = i64::try_from(repeats * 3 * 352 + 64 * 1024)
                .expect("the fixture allocation budget fits i64");
            assert!(
                measured.peak_working_bytes <= peak_budget,
                "snapshot={snapshot} rows={} peak budget={peak_budget}: {measured:?}",
                repeats * 3,
            );
            println!(
                "snapshot={snapshot} rows={} source={} peak={} retained={}",
                repeats * 3,
                bytes.len(),
                measured.peak_working_bytes,
                measured.retained_bytes
            );
        }
    }
}
