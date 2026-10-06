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

#[test]
fn incomplete_tag_runs_and_lexical_cleanup_use_linear_storage() {
    for pattern in ["☕\u{200c} ", "👩\u{e0020} "] {
        let mut previous: Option<Measurement> = None;
        for count in [256, 512, 1_024, 2_048, 4_096] {
            let input = pattern.repeat(count);
            let window = CurrentThreadWindow::open();
            let analysis = Analyzer::empty_lexicon().projections(&input).unwrap();
            let current = window.close();
            assert_eq!(analysis.lexical.len(), count);
            assert_eq!(analysis.surface.len(), count);
            assert_eq!(analysis.spans.len(), count);
            assert!(
                analysis
                    .lexical
                    .iter()
                    .all(|projection| projection.text.chars().count() == 1)
            );
            if let Some(previous) = previous {
                assert!(
                    current.requested_bytes <= previous.requested_bytes * 3,
                    "{pattern:?}, count {count}: {previous:?} -> {current:?}"
                );
            }
            previous = Some(current);
        }
    }
    // A single unfinished tag suffix must not allocate one protection record
    // per tag, or repeatedly copy the remaining suffix at each scalar.
    for count in [1_024, 16_384, 262_144] {
        let input = "👩".to_owned() + &"\u{e0020}".repeat(count);
        let window = CurrentThreadWindow::open();
        let analysis = Analyzer::empty_lexicon().projections(&input).unwrap();
        let measurement = window.close();
        assert_eq!(analysis.normalized.text, "👩");
        assert!(
            measurement.requested_bytes < 16_384,
            "{count}: {measurement:?}"
        );
    }
}
