// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! One-shot allocation probes for the pre-change expanded JSON-LD codec.
//!
//! This process is separate from Criterion because allocator accounting would
//! contaminate latency measurements. It reports allocation count, requested bytes,
//! retained bytes, peak working bytes, and output bytes for the exact fixtures used by
//! the timed bench, each phase bracketed by a whole-process window from the
//! workspace's shared instrument so a codec that fans out is counted where it
//! allocates.

use std::hint::black_box;

use purrdf_alloc_probe::{CountingAllocator, Measurement, WholeProcessWindow};
use purrdf_rdf::native_codecs::jsonld::{
    CompiledJsonLdContext, JsonLdSerializeOptions, parse_jsonld, serialize_dataset_to_jsonld,
    serialize_dataset_to_jsonld_with_options,
};

#[path = "support/jsonld.rs"]
mod fixture;

use fixture::{LARGE_ROWS, SMALL_ROWS, build_dataset};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn report(label: &str, measured: Measurement, artifact_bytes: usize) {
    println!(
        "[jsonld_alloc] {label}: allocations={} requested_bytes={} retained_bytes={} peak_working_bytes={} artifact_bytes={artifact_bytes}",
        measured.allocations,
        measured.requested_bytes,
        measured.retained_bytes,
        measured.peak_working_bytes,
    );
}

fn probe(rows: usize) {
    let dataset = build_dataset(rows);
    let window = WholeProcessWindow::open();
    let json = serialize_dataset_to_jsonld(&dataset).expect("expanded JSON-LD serialization");
    let measured = window.close();
    report(
        &format!("serialize_expanded_rows_{rows}"),
        measured,
        json.len(),
    );

    let window = WholeProcessWindow::open();
    // No base: the generated corpus spells every IRI absolutely, so resolution never
    // runs and the measurement stays on expansion rather than on base arithmetic.
    let parsed = parse_jsonld(json.as_bytes(), None).expect("expanded JSON-LD parse");
    let measured = window.close();
    report(&format!("parse_expanded_rows_{rows}"), measured, json.len());
    black_box(parsed);
    black_box(json);

    let caller = JsonLdSerializeOptions::compiled(std::sync::Arc::new(
        CompiledJsonLdContext::from_prefixes([
            ("ex", "https://example.org/"),
            ("p", "https://example.org/p/"),
            ("o", "https://example.org/o/"),
        ])
        .expect("compile context"),
    ));
    for (mode, options) in [
        ("caller", caller),
        ("derived", JsonLdSerializeOptions::derived()),
    ] {
        let window = WholeProcessWindow::open();
        let json = serialize_dataset_to_jsonld_with_options(&dataset, &options)
            .expect("configured JSON-LD serialization");
        let measured = window.close();
        report(
            &format!("serialize_{mode}_rows_{rows}"),
            measured,
            json.len(),
        );

        let window = WholeProcessWindow::open();
        // No base, as above: the corpus is absolute-only by construction.
        let parsed = parse_jsonld(json.as_bytes(), None).expect("configured JSON-LD parse");
        let measured = window.close();
        report(&format!("parse_{mode}_rows_{rows}"), measured, json.len());
        black_box(parsed);
        black_box(json);
    }
}

fn main() {
    probe(SMALL_ROWS);
    probe(LARGE_ROWS);
}
