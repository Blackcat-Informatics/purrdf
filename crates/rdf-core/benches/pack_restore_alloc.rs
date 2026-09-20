// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// This bench is a plain `main`, but the workspace `missing_docs` lint applies to its
// items just the same; the reporting helpers below are internal probes, not API.
#![allow(missing_docs)]

//! Allocation evidence for the pack RESTORE path, phase by phase.
//!
//! **Report-only. No figure here is asserted, compared against a baseline, or gated
//! on.** It exists because restoring a pack is a cold-start cost in a process that
//! then serves for a long time, and because the restore path's allocation profile is
//! otherwise invisible: `pack_query` times the same calls, and a timing harness on a
//! contended machine cannot tell "this phase allocates once per term" from noise.
//!
//! # What is measured, and why it splits this way
//!
//! [`purrdf_core::restore_pack`] is exactly two phases, and they have opposite
//! shapes, so a single figure over both would hide either one:
//!
//! * **`open`** — `PackView::from_bytes`. Verifies the container and decodes the
//!   DICTIONARY into its owned arena; the bitmap-triples and side-table sections are
//!   borrowed in place and cost nothing. Its allocation count is therefore a property
//!   of the dictionary decoder alone, and the interesting risk is per-TERM work
//!   creeping back into it (an owned record, an owned string, a parsed IRI kept only
//!   to be dropped) — which shows up here as a count that scales with `n_terms`
//!   instead of staying flat.
//! * **`materialize`** — `dataset_from_view`. Replays the opened view into a fresh
//!   `RdfDatasetBuilder` and freezes it. This is where the restored dataset's own
//!   tables are allocated, so its count is a small constant (one allocation per
//!   retained table) and its RETAINED figure is the real output.
//!
//! Both are reported for two fixtures whose difference is structural rather than
//! merely bigger: an RDF 1.1 graph (no side-tables, no triple terms) and an RDF 1.2
//! graph carrying reifier bindings, statement annotations, quoted triples, named
//! graphs and directional language literals. A regression that only touches the
//! RDF 1.2 decode paths is invisible in the first and obvious in the second.
//!
//! Everything is under `example.org`: PurRDF mints no vocabulary IRIs, and a
//! benchmark fixture is no more entitled to invent one than a release build is.
//! Nothing here reads a clock, the filesystem, the environment, or a source of
//! randomness, so every figure is one the same revision reproduces.
//!
//! Run with `cargo bench -p purrdf-core --bench pack_restore_alloc` (the `make bench`
//! lane) — excluded from `make check`.

use std::fmt::Write as _;
use std::hint::black_box;
use std::sync::Arc;

use purrdf_alloc_probe::{CountingAllocator, Measurement, WholeProcessWindow};
use purrdf_core::{
    PackBuilder, PackView, RdfDataset, RdfDatasetBuilder, RdfLiteral, RdfTextDirection,
    dataset_from_view, restore_pack,
};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

/// Print one phase's four figures.
fn report(label: &str, measured: Measurement) {
    println!(
        "[pack_restore_alloc] {label}: allocations={} requested_bytes={} retained_bytes={} \
         peak_working_bytes={}",
        measured.allocations,
        measured.requested_bytes,
        measured.retained_bytes,
        measured.peak_working_bytes
    );
}

/// The fixture namespace. Caller-supplied, and `example.org` by rule.
const NS: &str = "http://example.org/pack#";

/// How many subjects each fixture graph carries.
const SUBJECTS: usize = 64;

/// An RDF 1.1 graph: one default graph, IRIs and plain literals only, no
/// side-tables and no quoted triples. The dictionary decode's baseline shape.
fn rdf_11_dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let mut iri = String::new();
    let mut name = |suffix: &str| {
        iri.clear();
        let _ = write!(iri, "{NS}{suffix}");
        iri.clone()
    };
    let rdf_type = builder.intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
    let class = builder.intern_iri(&name("Thing"));
    let label = builder.intern_iri(&name("label"));
    let peer = builder.intern_iri(&name("peer"));

    let subjects: Vec<_> = (0..SUBJECTS)
        .map(|i| builder.intern_iri(&name(&format!("s{i}"))))
        .collect();
    for (i, &s) in subjects.iter().enumerate() {
        builder.push_quad(s, rdf_type, class, None);
        let text = builder.intern_literal(RdfLiteral::simple(format!("label of subject {i}")));
        builder.push_quad(s, label, text, None);
        builder.push_quad(s, peer, subjects[(i + 1) % SUBJECTS], None);
    }
    builder.freeze().expect("the RDF 1.1 fixture freezes")
}

/// An RDF 1.2 graph exercising every component the RDF 1.1 fixture omits: named
/// graphs, blank nodes, quoted triples, reifier bindings, statement annotations, and
/// literals whose base direction is part of their identity.
fn rdf_12_dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let mut iri = String::new();
    let mut name = |suffix: &str| {
        iri.clear();
        let _ = write!(iri, "{NS}{suffix}");
        iri.clone()
    };
    let says = builder.intern_iri(&name("says"));
    let note = builder.intern_iri(&name("note"));
    let source = builder.intern_iri(&name("source"));
    let graph = builder.intern_iri(&name("g"));

    for i in 0..SUBJECTS {
        let subject = builder.intern_iri(&name(&format!("s{i}")));
        let object = builder.intern_literal(RdfLiteral {
            lexical_form: format!("statement {i}"),
            datatype: None,
            language: Some("ar".to_owned()),
            direction: Some(if i % 2 == 0 {
                RdfTextDirection::Ltr
            } else {
                RdfTextDirection::Rtl
            }),
        });
        let in_graph = (i % 4 == 0).then_some(graph);
        builder.push_quad(subject, says, object, in_graph);

        // A quoted triple, bound to a blank-node reifier and annotated.
        let quoted = builder.intern_triple(subject, says, object);
        let reifier = builder.intern_blank(&format!("r{i}"), purrdf_core::BlankScope::DEFAULT);
        builder.push_reifier_in_graph(reifier, quoted, in_graph);
        let annotation = builder.intern_literal(RdfLiteral::simple(format!("annotation {i}")));
        builder.push_annotation_in_graph(reifier, note, annotation, in_graph);
        builder.push_quad(reifier, source, subject, in_graph);
    }
    builder.freeze().expect("the RDF 1.2 fixture freezes")
}

/// Report `open`, `materialize` and the whole `restore_pack` over one fixture.
fn probe(label: &str, dataset: &Arc<RdfDataset>) {
    let bytes = PackBuilder::build_bytes(dataset.as_ref()).expect("the fixture packs");
    println!(
        "[pack_restore_alloc] {label}: pack_bytes={} quads={} terms={}",
        bytes.len(),
        dataset.quad_count(),
        dataset.term_count(),
    );

    // Outside every measured region: the first restore of the process pays for lazily
    // initialized state that belongs to no phase below.
    black_box(restore_pack(&bytes).expect("the fixture restores"));

    {
        let window = WholeProcessWindow::open();
        let view = PackView::from_bytes(&bytes).expect("the fixture opens");
        let measured = window.close();
        // Read something off the view inside the black box, so an open that decoded
        // nothing could not be optimized away and reported as free.
        black_box(view.dict().n_terms());
        report(&format!("{label}/open"), measured);
    }

    {
        let view = PackView::from_bytes(&bytes).expect("the fixture opens");
        let window = WholeProcessWindow::open();
        let restored = dataset_from_view(&view).expect("the opened view materializes");
        let measured = window.close();
        // A materialization that lost rows would post a flatteringly small figure;
        // the comparison is outside the measured region.
        assert_eq!(
            restored.quad_count(),
            dataset.quad_count(),
            "{label}: the restored dataset must carry every quad",
        );
        assert_eq!(
            restored.term_count(),
            dataset.term_count(),
            "{label}: the restored dataset must carry every term",
        );
        report(&format!("{label}/materialize"), measured);
    }

    {
        let window = WholeProcessWindow::open();
        let restored = restore_pack(&bytes).expect("the fixture restores");
        let measured = window.close();
        black_box(restored.quad_count());
        report(&format!("{label}/restore_pack"), measured);
    }
}

fn main() {
    let rdf_11 = rdf_11_dataset();
    let rdf_12 = rdf_12_dataset();

    // The RDF 1.2 fixture must actually carry the components it exists to exercise,
    // or its figures describe the RDF 1.1 decode path under another name.
    assert!(
        rdf_12.reifier_quads().count() > 0
            && rdf_12.annotation_quads().count() > 0
            && rdf_12.named_graphs().count() > 0,
        "the RDF 1.2 fixture must carry reifiers, annotations and a named graph",
    );

    probe("rdf_11", &rdf_11);
    probe("rdf_12", &rdf_12);
}
