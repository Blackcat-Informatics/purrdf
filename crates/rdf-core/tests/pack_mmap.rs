// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

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
//! borrowed `&[u8]` the CALLER supplies — including a slice backed by a
//! memory-mapped file. This test file plays the role of that external
//! consumer: it is the ONLY place in this crate's test suite that depends on
//! `memmap2`, and it does so strictly as a `[dev-dependencies]` entry (see
//! `crates/rdf-core/Cargo.toml`), never a runtime dependency. The whole file
//! is gated off the wasm32 target, where mmap has no meaning.

#![cfg(not(target_arch = "wasm32"))]

use std::io::Write as _;
use std::sync::Arc;

use purrdf_core::{
    BlankScope, DatasetView, GraphMatch, PackBuilder, PackView, RdfDataset, RdfDatasetBuilder,
    RdfLiteral, RdfTextDirection, TermRef, TermValue, verify_pack,
};

/// An `example.org` IRI value.
fn iri(name: &str) -> TermValue {
    TermValue::iri(format!("http://example.org/{name}"))
}

/// Resolve a view id to its dataset-INDEPENDENT `TermValue`, recursing through
/// a literal's datatype and a triple term's components. Generic over any
/// `DatasetView`, mirroring the by-value resolution pattern shared by
/// `tests/pack_dataset_view.rs` and `tests/paged_backend.rs` — the SAME
/// routine reads the reference `RdfDataset`, the heap-backed `PackView`, and
/// the mmap-backed `PackView` under test, so their rows can be compared by
/// value (each view mints its own unrelated id space).
fn to_value<V: DatasetView>(v: &V, id: V::Id) -> TermValue {
    match v.resolve(id) {
        TermRef::Iri(s) => TermValue::iri(s),
        TermRef::Blank { label, scope } => TermValue::Blank {
            label: label.to_owned(),
            scope,
        },
        TermRef::Literal {
            lexical,
            datatype,
            language,
            direction,
        } => {
            let datatype = match v.resolve(datatype) {
                TermRef::Iri(s) => s.to_owned(),
                other => panic!("literal datatype must resolve to an IRI, got {other:?}"),
            };
            TermValue::Literal {
                lexical_form: lexical.to_owned(),
                datatype,
                language: language.map(str::to_owned),
                direction,
            }
        }
        TermRef::Triple { s, p, o } => TermValue::Triple {
            s: Box::new(to_value(v, s)),
            p: Box::new(to_value(v, p)),
            o: Box::new(to_value(v, o)),
        },
    }
}

/// A deterministic sort key for a value row (`TermValue` is not `Ord`; its
/// `Debug` form is total and dataset-independent).
fn row_key(row: &[TermValue]) -> String {
    format!("{row:?}")
}

/// Collect every quad of the view as sorted `[s, p, o, g?]` value rows via the
/// generic trait surface only.
fn collect_rows<V: DatasetView>(v: &V) -> Vec<Vec<TermValue>> {
    let mut rows: Vec<Vec<TermValue>> = v
        .quads_for_pattern(None, None, None, GraphMatch::Any)
        .map(|q| {
            let mut row = vec![to_value(v, q.s), to_value(v, q.p), to_value(v, q.o)];
            row.push(q.g.map_or_else(|| TermValue::iri("urn:default-graph"), |g| to_value(v, g)));
            row
        })
        .collect();
    rows.sort_by_key(|r| row_key(r));
    rows
}

/// Look up a bound `TermValue`'s id in `v` via `term_id_by_value` — exactly
/// how the evaluator resolves a pattern's bound constants before probing
/// (never by minting).
fn id_of<V: DatasetView>(v: &V, value: &TermValue) -> V::Id {
    v.term_id_by_value(value)
        .unwrap_or_else(|| panic!("value {value:?} must be interned"))
}

/// Build a rich fixture `RdfDataset`: default graph + a named graph, every
/// literal shape, a scoped blank node, and a reifier + annotation pair (the
/// same seams `tests/pack_dataset_view.rs` exercises), so the mmap-backed
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
/// (over an owned `Vec<u8>`), and an mmap-backed `PackView` (over the same
/// bytes read back through `memmap2::Mmap`). Every assertion below compares
/// all three by value.
#[test]
fn mmap_backed_pack_view_matches_heap_and_source_by_value() {
    let dataset = build_fixture();
    let bytes = PackBuilder::build_bytes(&dataset).expect("pack build must succeed");

    // Write the pack bytes to a real temp file the OS can mmap.
    let mut tmp = tempfile::NamedTempFile::new().expect("create temp file");
    tmp.write_all(&bytes).expect("write pack bytes");
    tmp.flush().expect("flush temp file");
    let path = tmp.path().to_path_buf();

    // Re-open the temp file read-only and mmap it. This is the ONLY unsafe
    // block in this crate's test suite, and it is the entire point of the
    // test: `Mmap::map` is unsafe because the OS cannot guarantee the backing
    // file won't be mutated out from under the mapping by another process,
    // but the mapping is confined to this test's scope over a freshly written
    // temp file this test exclusively owns for its whole duration (it is
    // dropped, unmapping, before `tmp`'s own `Drop` deletes the file).
    let file = std::fs::File::open(&path).expect("reopen temp file read-only");
    // SAFETY: `file` is a freshly-written `NamedTempFile` created above and
    // exclusively owned by this test; nothing else opens, truncates, or
    // mutates it while `mmap` is alive, so the memory-safety precondition of
    // `Mmap::map` (no concurrent external mutation of the backing file) holds
    // for the mapping's entire lifetime, which ends before `tmp` is dropped.
    let mmap = unsafe { memmap2::Mmap::map(&file).expect("mmap pack file") };

    // The library-side seam under test: `PackView::from_bytes` is zero-copy
    // over WHATEVER borrowed `&[u8]` the consumer hands it — here, the
    // mmap'd slice — with no mmap/filesystem awareness inside purrdf-core.
    let pack_mmap = PackView::from_bytes(&mmap[..]).expect("pack opens over mmap'd bytes");

    // The heap-backed twin, over an owned `Vec<u8>` copy of the identical
    // bytes, for a direct mmap-vs-heap parity comparison.
    let heap_bytes = bytes.clone();
    let pack_heap = PackView::from_bytes(&heap_bytes[..]).expect("pack opens over heap bytes");

    // -- Certificate verification over the mmap'd slice --------------------------
    let mmap_digest =
        verify_pack(&mmap[..]).expect("verify_pack must succeed over the mmap'd slice");
    let heap_digest =
        verify_pack(&heap_bytes[..]).expect("verify_pack must succeed over the heap slice");
    assert_eq!(
        mmap_digest, heap_digest,
        "verify_pack's certified digest must be identical whether the bytes are mmap'd or heap-resident"
    );
    assert_eq!(
        mmap_digest.as_bytes(),
        &pack_mmap.rdfc_digest(),
        "verify_pack's certified digest must match the mmap'd view's own header digest"
    );

    // -- Whole-dataset scan parity: source vs heap vs mmap ------------------------
    let source_rows = collect_rows(&*dataset);
    let heap_rows = collect_rows(&pack_heap);
    let mmap_rows = collect_rows(&pack_mmap);
    assert_eq!(
        source_rows, heap_rows,
        "heap-backed PackView must scan identically to the source RdfDataset"
    );
    assert_eq!(
        heap_rows, mmap_rows,
        "mmap-backed PackView must scan IDENTICALLY to the heap-backed PackView (zero-copy parity)"
    );
    // Falsifiability: the fixture is non-trivial.
    assert!(mmap_rows.len() >= 6);

    // -- A bound pattern query, driven over the mmap'd view -----------------------
    let knows_id = id_of(&pack_mmap, &iri("knows"));
    let alice_id = id_of(&pack_mmap, &iri("alice"));
    let mut pattern_rows: Vec<Vec<TermValue>> = pack_mmap
        .quads_for_pattern(Some(alice_id), Some(knows_id), None, GraphMatch::Any)
        .map(|q| {
            vec![
                to_value(&pack_mmap, q.s),
                to_value(&pack_mmap, q.p),
                to_value(&pack_mmap, q.o),
            ]
        })
        .collect();
    pattern_rows.sort_by_key(|r| row_key(r));
    assert_eq!(
        pattern_rows,
        vec![vec![iri("alice"), iri("knows"), iri("bob")]],
        "quads_for_pattern over the mmap'd view must return the expected bound match"
    );

    // -- Reifier/annotation side-table read over the mmap'd view ------------------
    let mmap_reifiers: Vec<Vec<TermValue>> = pack_mmap
        .reifier_quads()
        .map(|q| {
            vec![
                to_value(&pack_mmap, q.s),
                to_value(&pack_mmap, q.p),
                to_value(&pack_mmap, q.o),
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
        mmap_reifiers, heap_reifiers,
        "reifier_quads parity between the mmap-backed and heap-backed views"
    );
    assert_eq!(mmap_reifiers.len(), 1, "the fixture carries one reifier");

    // The mapping (and the file) are dropped here, before `tmp`'s own `Drop`
    // deletes the underlying temp file — the mapping never outlives the file
    // it maps.
    drop(mmap);
    drop(file);
}
