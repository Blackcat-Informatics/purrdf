// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Demonstrates that the pack format is genuinely **mmap-able** WITHOUT
//! putting any mmap/filesystem dependency into the published `purrdf-core`
//! library.
//!
//! Per **G5** in `docs/design/purrdf-backend-contract.md` ("the shipped
//! `PageProvider` is in-memory; durable tiers are external"): a
//! memory-mapped (or otherwise disk-backed) tier belongs to the **external
//! consumer**, not to any published crate, because every published crate must
//! stay `wasm32-unknown-unknown`-clean — no filesystem, no threads, no
//! wall-clock, no RNG. The pack codec honors that contract exactly the same
//! way the paged backend does: `purrdf-core` never mmaps anything itself.
//! Instead, [`purrdf_core::PackView::from_bytes`] is zero-copy over any
//! borrowed `&[u8]` the CALLER supplies — a file mapping, or the owned buffer
//! the production pack reader hands it when it does not map.
//!
//! This test file plays the role of that external consumer. It writes the pack
//! to a real file and reads it back with `std::fs::read`, because what it
//! asserts is the byte-level reader contract (a pack round-tripped through the
//! filesystem opens, verifies and scans identically to the in-memory bytes),
//! and no assertion depends on how the borrowed bytes are backed. The CLI's
//! own mapped tier is exercised in `purrdf-cli`. The whole file is gated off
//! the wasm32 target, which has no filesystem.
#![cfg(not(target_arch = "wasm32"))]

#[path = "support/values.rs"]
mod values;
use purrdf_core::term_fixture::iri;
use std::io::Write as _;
use std::sync::Arc;
use values::{collect_rows, row_key, to_value};

use purrdf_core::{
    BlankScope, DatasetView, GraphMatch, PackBuilder, PackView, RdfDataset, RdfDatasetBuilder,
    RdfLiteral, RdfTextDirection, TermValue, verify_pack,
};

/// Look up a bound `TermValue`'s id in `v` via `term_id_by_value` — exactly
/// how the evaluator resolves a pattern's bound constants before probing
/// (never by minting).
fn id_of<V: DatasetView>(v: &V, value: &TermValue) -> V::Id {
    v.term_id_by_value(value)
        .unwrap_or_else(|| panic!("value {value:?} must be interned"))
}

/// Build a rich fixture `RdfDataset`: default graph + a named graph, every
/// literal shape, a scoped blank node, and a reifier + annotation pair (the
/// same seams `tests/pack_dataset_view.rs` exercises), so the file-backed
/// query surface below is genuinely non-trivial rather than a smoke test.
fn build_fixture() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();

    let alice = b.intern_iri("http://example.org/alice");
    let bob = b.intern_iri("http://example.org/bob");
    let carol = b.intern_iri("http://example.org/carol");
    let knows = b.intern_iri("http://example.org/knows");
    let age = b.intern_iri("http://example.org/age");
    let name = b.intern_iri("http://example.org/name");
    let confidence = b.intern_iri("http://example.org/confidence");
    let high = b.intern_iri("http://example.org/high");
    let reifier = b.intern_iri("http://example.org/r");
    let graph1 = b.intern_iri("http://example.org/graph1");

    let blank = b.intern_blank("b1", BlankScope::DEFAULT);

    let forty_two = b.intern_literal(RdfLiteral {
        lexical_form: "42".to_string(),
        datatype: Some("http://www.w3.org/2001/XMLSchema#integer".to_string()),
        language: None,
        direction: None,
    });
    let alice_name_en = b.intern_literal(RdfLiteral {
        lexical_form: "Alice".to_string(),
        datatype: None,
        language: Some("en".to_string()),
        direction: None,
    });
    let hello_ltr = b.intern_literal(RdfLiteral {
        lexical_form: "Hello".to_string(),
        datatype: None,
        language: Some("en".to_string()),
        direction: Some(RdfTextDirection::Ltr),
    });

    let alice_knows_bob = b.intern_triple(alice, knows, bob);

    // -- Default graph --------------------------------------------------------
    b.push_quad(alice, knows, bob, None);
    b.push_quad(alice, age, forty_two, None);
    b.push_quad(alice, name, alice_name_en, None);
    b.push_quad(bob, name, hello_ltr, None);
    b.push_quad(blank, knows, alice, None);

    // -- Named graph ------------------------------------------------------------
    b.push_quad(bob, knows, carol, Some(graph1));

    // -- Reifier + annotation -----------------------------------------------------
    b.push_reifier(reifier, alice_knows_bob);
    b.push_annotation(reifier, confidence, high);

    b.freeze().expect("fixture dataset must validate")
}

/// Builds the fixture pack once, writes it to a `NamedTempFile`, and opens
/// THREE views over it: the source `RdfDataset`, a heap-backed `PackView`
/// (over an owned copy of the built bytes), and a file-backed `PackView` (over
/// the bytes read back from disk with `std::fs::read`). Every assertion below
/// compares all three by value.
#[test]
fn file_backed_pack_view_matches_heap_and_source_by_value() {
    let dataset = build_fixture();
    let bytes = PackBuilder::build_bytes(&dataset).expect("pack build must succeed");

    // Write the pack bytes to a real temp file and read them back from disk.
    let mut tmp = purrdf_testkit::temp_file!().expect("create temp file");
    tmp.write_all(&bytes).expect("write pack bytes");
    tmp.flush().expect("flush temp file");
    let disk = std::fs::read(tmp.path()).expect("read pack file back");

    // The library-side seam under test: `PackView::from_bytes` is zero-copy
    // over WHATEVER borrowed `&[u8]` the consumer hands it — here, the
    // bytes read back from disk — with no filesystem awareness inside purrdf-core.
    let pack_disk = PackView::from_bytes(&disk[..]).expect("pack opens over disk-read bytes");

    // The heap-backed twin, over an owned `Vec<u8>` copy of the identical
    // bytes, for a direct disk-vs-heap parity comparison.
    let heap_bytes = bytes.clone();
    let pack_heap = PackView::from_bytes(&heap_bytes[..]).expect("pack opens over heap bytes");

    // -- Certificate verification over the disk-read slice --------------------------
    let disk_digest =
        verify_pack(&disk[..]).expect("verify_pack must succeed over the disk-read slice");
    let heap_digest =
        verify_pack(&heap_bytes[..]).expect("verify_pack must succeed over the heap slice");
    assert_eq!(
        disk_digest, heap_digest,
        "verify_pack's certified digest must be identical whether the bytes are read from disk or heap-resident"
    );
    assert_eq!(
        disk_digest.as_bytes(),
        &pack_disk.rdfc_digest(),
        "verify_pack's certified digest must match the disk-read view's own header digest"
    );

    // -- Whole-dataset scan parity: source vs heap vs disk ------------------------
    let source_rows = collect_rows(&*dataset);
    let heap_rows = collect_rows(&pack_heap);
    let disk_rows = collect_rows(&pack_disk);
    assert_eq!(
        source_rows, heap_rows,
        "heap-backed PackView must scan identically to the source RdfDataset"
    );
    assert_eq!(
        heap_rows, disk_rows,
        "file-backed PackView must scan IDENTICALLY to the heap-backed PackView (zero-copy parity)"
    );
    // Falsifiability: the fixture is non-trivial.
    assert!(disk_rows.len() >= 6);

    // -- A bound pattern query, driven over the disk-read view -----------------------
    let knows_id = id_of(&pack_disk, &iri("knows"));
    let alice_id = id_of(&pack_disk, &iri("alice"));
    let mut pattern_rows: Vec<Vec<TermValue>> = pack_disk
        .quads_for_pattern(Some(alice_id), Some(knows_id), None, GraphMatch::Any)
        .map(|q| {
            vec![
                to_value(&pack_disk, q.s),
                to_value(&pack_disk, q.p),
                to_value(&pack_disk, q.o),
            ]
        })
        .collect();
    pattern_rows.sort_by_key(|r| row_key(r));
    assert_eq!(
        pattern_rows,
        vec![vec![iri("alice"), iri("knows"), iri("bob")]],
        "quads_for_pattern over the disk-read view must return the expected bound match"
    );

    // -- Reifier/annotation side-table read over the disk-read view ------------------
    let disk_reifiers: Vec<Vec<TermValue>> = pack_disk
        .reifier_quads()
        .map(|q| {
            vec![
                to_value(&pack_disk, q.s),
                to_value(&pack_disk, q.p),
                to_value(&pack_disk, q.o),
            ]
        })
        .collect();
    let heap_reifiers: Vec<Vec<TermValue>> = pack_heap
        .reifier_quads()
        .map(|q| {
            vec![
                to_value(&pack_heap, q.s),
                to_value(&pack_heap, q.p),
                to_value(&pack_heap, q.o),
            ]
        })
        .collect();
    assert_eq!(
        disk_reifiers, heap_reifiers,
        "reifier_quads parity between the file-backed and heap-backed views"
    );
    assert_eq!(disk_reifiers.len(), 1, "the fixture carries one reifier");

    drop(tmp);
}
