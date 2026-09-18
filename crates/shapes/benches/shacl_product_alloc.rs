// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// This bench is a plain `main`, but the workspace `missing_docs` lint applies to its
// items just the same; the reporting helpers below are internal probes, not API.
#![allow(missing_docs)]

//! Allocation and peak-memory evidence for the SHACL prepared-shapes product,
//! phase by phase.
//!
//! **Report-only. No figure here is asserted, compared against a baseline, or
//! gated on.** A prepared product is a structural change, so it needs no invented
//! speedup threshold; what it does need is an honest statement of what each phase
//! costs in memory, because the interesting risk in a "cache the parse" feature is
//! not that restoring is slow, it is that producing the cache has a peak the
//! producer never budgeted for.
//!
//! It runs in a separate process from the timed `shacl_product_reuse` target and
//! reads the same fixture module (`benches/support/product.rs`), because the
//! global allocator installed below accounts for every allocation and would
//! contaminate any latency measurement sharing the process.
//!
//! # What each line reports
//!
//! Each phase is bracketed by a [`WholeProcessWindow`] from the workspace's
//! shared instrument, which is the window that counts **every** thread —
//! `bind/dataset` and `eval/validate` reach SHACL validation, which fans focus
//! nodes out over `rayon`, and a per-thread window would report those phases as
//! nearly free. It reports four quantities and never blends them:
//!
//! * **`allocations`** — how many allocation calls the phase made.
//! * **`requested_bytes`** — total bytes requested through the global allocator
//!   during the phase, including memory freed again before it ended. This is the
//!   allocator *traffic*, not the footprint.
//! * **`retained_bytes`** — the change in live bytes across the phase: what the
//!   phase's result is still holding when it returns.
//! * **`peak_working_bytes`** — the high-water mark of live bytes during the
//!   phase, relative to where it started. Traffic and peak are reported
//!   separately on purpose: a phase that allocates and frees a large buffer
//!   repeatedly has high traffic and a modest peak, while a phase that assembles
//!   one large structure has the reverse, and conflating them would hide exactly
//!   the distinction between `encode/to_product` (which re-packs and
//!   canonicalizes the shapes dataset) and the restore paths.
//!
//! # The intermediate bytes
//!
//! The encoded product's length is printed here as `artifact_bytes`, and that is
//! the "intermediate bytes" the product interposes between preparation and
//! restore. It is NOT only a bench-log fact: the same quantity is pinned for the
//! determinism fixture as the asserted constant `GOLDEN_LEN` in
//! `crates/shapes/tests/product_determinism.rs`, so a codec change that doubled
//! the artifact fails the build rather than quietly changing a number in a log
//! nobody re-reads. The figure printed here is for this target's own, larger
//! fixture and is the illustration; the test's is the enforced fact.
//!
//! Run with `cargo bench -p purrdf-shapes --bench shacl_product_alloc` (the
//! `make bench` lane) — excluded from `make check`.

use std::hint::black_box;
use std::sync::Arc;

use purrdf_alloc_probe::{CountingAllocator, Measurement, WholeProcessWindow};
use purrdf_shapes::engine::{PreparedShapes, parse_shapes};
use purrdf_shapes::product::{HostBindings, ShapesProduct, ShapesProfile};

#[path = "support/product.rs"]
mod fixture;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

/// Print one phase's four figures.
fn report(label: &str, measured: Measurement) {
    println!(
        "[shacl_product_alloc] {label}: allocations={} requested_bytes={} retained_bytes={} \
         peak_working_bytes={}",
        measured.allocations,
        measured.requested_bytes,
        measured.retained_bytes,
        measured.peak_working_bytes
    );
}

fn main() {
    // Untimed, unmeasured: the fixture's own source text and data graph are the
    // input to every phase below, not one of the phases.
    let source = fixture::shapes_source();
    let data = fixture::data();
    let host = HostBindings::empty();
    println!(
        "[shacl_product_alloc] fixture shapes source is {} bytes",
        source.len()
    );

    // The cold path a product replaces: Turtle text to a reusable preparation.
    let window = WholeProcessWindow::open();
    let prepared = fixture::prepared(&source);
    report("cold/parse_and_prepare", window.close());

    // The reusable preparation alone, over an already-parsed shapes graph: the
    // class-catalog derivation both restore paths repeat rather than carry.
    let parsed = Arc::new(parse_shapes(&source, None).expect("the bench shapes graph parses"));
    let window = WholeProcessWindow::open();
    let reusable = PreparedShapes::new(Arc::clone(&parsed));
    report("prepare/reusable", window.close());
    drop(reusable);

    // The producer's cost, and the pipeline's peak-memory event: the shapes
    // dataset is re-packed and canonicalized so the product can state an identity
    // it has actually established.
    let window = WholeProcessWindow::open();
    let product = fixture::encode(&prepared);
    report("encode/to_product", window.close());
    println!(
        "[shacl_product_alloc] artifact_bytes={} (the intermediate bytes; the determinism \
         fixture's equivalent is an asserted constant, not only a log line)",
        product.len()
    );

    // The structural tier alone: envelope framing and digests, admitting nothing.
    {
        let window = WholeProcessWindow::open();
        let view = ShapesProduct::open(&product).expect("the product opens");
        let measured = window.close();
        black_box(view.section_kinds());
        report("restore/open", measured);
    }

    // The memo path. The open is outside the measured region so this line reports
    // admission rather than admission plus the structural tier above.
    {
        let view = ShapesProduct::open(&product).expect("the product opens");
        let window = WholeProcessWindow::open();
        let admitted = view
            .admit(&ShapesProfile::CORE, &host)
            .expect("the product admits");
        let measured = window.close();
        report("restore/admit", measured);
        drop(admitted);
    }

    // The memo-free path: ignore the AST section, re-derive from the carried
    // dataset. Its retained and peak figures against `restore/admit` are what say
    // whether the model section is paying for itself.
    {
        let view = ShapesProduct::open(&product).expect("the product opens");
        let window = WholeProcessWindow::open();
        let rebuilt = view
            .rebuild(&ShapesProfile::CORE, &host)
            .expect("the product rebuilds");
        let measured = window.close();
        report("restore/rebuild", measured);
        drop(rebuilt);
    }

    // Per-dataset work, reported apart from every line above: paid once per
    // snapshot no matter how the preparation was obtained.
    let window = WholeProcessWindow::open();
    let validator = prepared
        .bind_shared_dataset(Arc::clone(&data))
        .expect("the bench data graph binds");
    report("bind/dataset", window.close());

    let window = WholeProcessWindow::open();
    let validation = validator.validate().expect("validation runs");
    let measured = window.close();
    report("eval/validate", measured);
    // Outside the measured region: a phase that reached no constraint would
    // otherwise report a small, tidy, meaningless number.
    fixture::assert_non_vacuous(&validation);
    println!(
        "[shacl_product_alloc] eval/validate produced {} result(s), conforms={}",
        validation.results.len(),
        validation.conforms
    );
}
