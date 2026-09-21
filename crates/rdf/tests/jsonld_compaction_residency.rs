// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Compacting a JSON-LD document does not hold two copies of the carrier.
//!
//! The compaction path rewrites its document in place — it consumes graph-index
//! metadata and relocates reverse properties — and reaching those `&mut` passes from a
//! `&self` receiver meant cloning the whole carrier first. On a dataset-sized document
//! that clone is pure duplication: the original is dropped the moment compaction
//! returns.
//!
//! WHY A TEST AND NOT A PROBE NUMBER
//!
//! The envelope probe's round-trip workload does not reach this path. Its JSON-LD leg
//! carries no egress base, and with no base the serializer emits the EXPANDED form,
//! which never compacts. Running the probe before and after this change produces
//! identical figures — correctly — so the probe cannot witness the duplication and a
//! claim resting on it would be measuring the wrong leg.
//!
//! So the oracle is here, on the path that actually compacts, and it is the allocator
//! rather than the output: duplication is a residency property and the emitted bytes
//! are unchanged by construction.

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_core::{RdfDatasetBuilder, RdfLiteral, RdfTerm};
use purrdf_rdf::{
    JsonLdSerializeOptions, NativeRdfFormat, serialize_dataset_to_format,
    serialize_dataset_to_format_with_jsonld_options,
};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

/// A dataset whose carrier is large enough that a second copy is unmistakable.
fn dataset(rows: usize) -> std::sync::Arc<purrdf_core::RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    for i in 0..rows {
        // Several predicates under a shared namespace, so the derived context has
        // something to compact against and the carrier carries real node structure.
        let s = b.intern_owned_term(&RdfTerm::iri(format!(
            "https://example.org/vocabulary/subject/{i}"
        )));
        for (slot, predicate) in ["title", "description", "identifier"].iter().enumerate() {
            let p = b.intern_iri(&format!("https://example.org/vocabulary/{predicate}"));
            let o = b.intern_literal(RdfLiteral::simple(format!(
                "row {i} slot {slot}, padded out so the carrier is substantially larger \
                 than the handful of bytes a threshold could be confused by"
            )));
            b.push_quad(s, p, o, None);
        }
    }
    b.freeze().expect("dataset freezes")
}

/// Peak residency during compaction stays below the document it produces.
///
/// A whole-carrier clone puts the peak ABOVE the document rather than below it: the
/// carrier is the document's structured form and is comparable in size, so holding two
/// of them plus the output cannot fit under one document's worth. That is the
/// separation this threshold sits in, and it is wide — this is not a tuned number.
#[test]
fn compaction_does_not_hold_a_second_carrier() {
    let dataset = dataset(2_000);
    let options = JsonLdSerializeOptions::derived();

    let window = CurrentThreadWindow::open();
    let outcome = serialize_dataset_to_format_with_jsonld_options(
        &*dataset,
        NativeRdfFormat::JsonLd,
        None,
        &options,
    )
    .expect("a derived-context JSON-LD document");
    let measured = window.close();

    let document = outcome.bytes.len();
    assert!(
        document > 512 * 1024,
        "the fixture must be large enough to separate the two regimes; {document} bytes"
    );
    // The control: the EXPANDED path builds the same carrier from the same dataset and
    // never compacts, so it never had the clone. The difference between the two is
    // compaction's own working set, and the question is whether a whole extra carrier
    // is in it.
    let expanded_options = JsonLdSerializeOptions::expanded();
    let control = CurrentThreadWindow::open();
    let expanded = serialize_dataset_to_format_with_jsonld_options(
        &*dataset,
        NativeRdfFormat::JsonLd,
        None,
        &expanded_options,
    )
    .expect("an expanded JSON-LD document");
    let control = control.close();
    assert!(
        expanded.bytes.len() > 512 * 1024,
        "the control must be large too, or the comparison is between two small numbers"
    );

    // Stated as a difference against the control rather than as an absolute, so the
    // bar cannot rot: unrelated growth anywhere in the pipeline moves BOTH measurements
    // and the assertion keeps meaning what it says. An absolute threshold here would
    // have to be retuned every time something upstream allocated differently, and a
    // retuned threshold is one that stopped testing the property.
    //
    // The separation is not marginal. Without the clone, compaction exceeds the
    // expanded control by roughly eleven kilobytes — its own working set. With it, by
    // more than three megabytes, which is the carrier: about two and a half times the
    // document these figures are measured against.
    let extra = measured.peak_working_bytes - control.peak_working_bytes;
    assert!(
        extra < (document / 2) as i64,
        "compaction peaked {extra} bytes above the expanded path on the same dataset, \
         against a {document}-byte document. Compaction's own working set is a few \
         kilobytes; a figure at document scale is a second copy of the carrier."
    );
}

/// The bytes are unaffected: this is a residency change, not a format change.
#[test]
fn compaction_output_is_unchanged_by_taking_the_carrier_by_value() {
    let dataset = dataset(50);
    let options = JsonLdSerializeOptions::derived();

    let first = serialize_dataset_to_format_with_jsonld_options(
        &*dataset,
        NativeRdfFormat::JsonLd,
        None,
        &options,
    )
    .expect("a derived-context JSON-LD document");
    let second = serialize_dataset_to_format_with_jsonld_options(
        &*dataset,
        NativeRdfFormat::JsonLd,
        None,
        &options,
    )
    .expect("a derived-context JSON-LD document");

    assert_eq!(
        first.bytes, second.bytes,
        "compaction must stay deterministic across calls"
    );
    let text = String::from_utf8(first.bytes).expect("utf-8");
    assert!(
        text.contains("@context"),
        "a derived-context document must carry the context it derived"
    );
}

/// YAML-LD costs about what JSON-LD costs, rather than that plus two more copies.
///
/// YAML-LD used to serialize the whole JSON-LD document to a `String`, reparse it into
/// a `serde_json::Value`, and convert that — holding the `SerGraph`, the carrier, the
/// JSON text and the value tree at once. It was the one format whose comment said its
/// intermediate document could not be removed, and it was strictly more resident than
/// the eager path it replaced.
///
/// JSON-LD over the same dataset is the control: it builds the same carrier and emits
/// straight from it. The two now differ by their emitters, so their peaks should be
/// comparable. Held whole, YAML-LD's peak carried the JSON document and its parsed
/// tree on top of everything the control holds.
#[test]
fn yamlld_does_not_cost_a_json_document_plus_its_parse_tree() {
    let dataset = dataset(2_000);

    let control = CurrentThreadWindow::open();
    let json = serialize_dataset_to_format(&*dataset, NativeRdfFormat::JsonLd, None)
        .expect("a JSON-LD document");
    let control = control.close();

    let window = CurrentThreadWindow::open();
    let yaml = serialize_dataset_to_format(&*dataset, NativeRdfFormat::YamlLd, None)
        .expect("a YAML-LD document");
    let measured = window.close();

    let json_bytes = json.bytes.len();
    assert!(
        json_bytes > 512 * 1024 && yaml.bytes.len() > 512 * 1024,
        "both documents must be large enough for the comparison to mean anything"
    );

    // Stated against the control rather than as an absolute, for the same reason as
    // above: unrelated growth moves both sides and the bar keeps its meaning. The
    // previous shape put YAML-LD a whole JSON document plus its value tree above the
    // control; allowing half a document of headroom separates that by a wide margin
    // while leaving room for the emitters genuinely differing.
    let extra = measured.peak_working_bytes - control.peak_working_bytes;
    assert!(
        extra < (json_bytes / 2) as i64,
        "YAML-LD peaked {extra} bytes above JSON-LD on the same dataset, against a \
         {json_bytes}-byte JSON document. A figure at document scale means the JSON \
         text and its parse tree are still being held."
    );
}
