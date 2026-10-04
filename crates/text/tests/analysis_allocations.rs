// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Allocation scaling of many protected emoji after a nonidentity transformation.
use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow, Measurement};
use purrdf_text::Analyzer;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn emoji_measurement(count: usize) -> Measurement {
    let input = format!("A{}", "😀".repeat(count));
    let window = CurrentThreadWindow::open();
    let analysis = Analyzer::empty_lexicon().projections(&input).unwrap();
    let measurement = window.close();
    assert_eq!(analysis.surface.len(), count + 1);
    assert_eq!(analysis.normalized.text, input.to_lowercase());
    measurement
}

#[test]
fn repeated_emoji_metadata_grows_linearly() {
    let mut previous = emoji_measurement(256);
    for count in [512, 1_024, 2_048, 4_096] {
        let current = emoji_measurement(count);
        // Doubling a linear input must remain well below the fourfold traffic
        // of per-atom exact reallocations; include every persisted projection.
        assert!(
            current.requested_bytes <= previous.requested_bytes * 3,
            "emoji count {count}: {previous:?} -> {current:?}"
        );
        previous = current;
    }
}
