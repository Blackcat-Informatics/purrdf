// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Separate allocation and expansion probes over the timed benchmark's inputs.
//!
//! Each phase is bracketed by a whole-process window from the workspace's shared
//! instrument, the window that counts every thread — the same scope the
//! hand-rolled process counters this replaced had.

use purrdf_alloc_probe::{CountingAllocator, Measurement, WholeProcessWindow};
use purrdf_json::{SourceDocument, analyze, decode_document, project};
use purrdf_rdf::{NativeRdfFormat, serialize_dataset_to_format};
use std::hint::black_box;

#[path = "support/fixture.rs"]
mod fixture;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn report(label: &str, measured: Measurement, artifact_bytes: usize) {
    println!(
        "[ordered_json_alloc] {label}: allocations={} requested_bytes={} retained_bytes={} peak_working_bytes={} artifact_bytes={artifact_bytes}",
        measured.allocations,
        measured.requested_bytes,
        measured.retained_bytes,
        measured.peak_working_bytes,
    );
}

fn probe(rows: usize) {
    let profile = fixture::profile();
    let text = fixture::document(rows);
    let source = SourceDocument {
        id: fixture::SOURCE,
        bytes: text.as_bytes(),
    };
    // The label is formatted before the window is closed, exactly as the counters
    // this replaced were read after it: the phase owns that one allocation.
    let window = WholeProcessWindow::open();
    let model = analyze(source, &profile).unwrap();
    report(&format!("analyze_rows_{rows}"), window.close(), text.len());
    let window = WholeProcessWindow::open();
    let dataset = project(&model, &profile).unwrap();
    report(&format!("project_rows_{rows}"), window.close(), text.len());
    let window = WholeProcessWindow::open();
    let decoded = decode_document(&dataset, model.id(), &profile).unwrap();
    report(
        &format!("decode_rows_{rows}"),
        window.close(),
        decoded.len(),
    );
    let rdf = serialize_dataset_to_format(&*dataset, NativeRdfFormat::NTriples, None).unwrap();
    println!(
        "[ordered_json_alloc] expansion_rows_{rows}: source_bytes={} rdf_bytes={} rdf_statements={} values={} structure_runs={} value_model_bytes={}",
        text.len(),
        rdf.bytes.len(),
        dataset.quads().count(),
        model.values().len(),
        model.structure().len(),
        size_of::<purrdf_json::Value>()
    );
    black_box(decoded);
    black_box(dataset);
    black_box(model);
}

fn main() {
    for rows in fixture::SIZES {
        probe(rows);
    }
}
