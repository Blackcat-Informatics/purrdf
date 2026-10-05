// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Named graphs a dataset declares but gives no row survive a pack.
//!
//! A pack carries a declaration-only graph as a zero-row named partition of the
//! TRIPLES section, beside the partitions of the graphs that own rows. These tests
//! pin the four halves of that contract:
//!
//! * the round trip — IRI- and blank-named declarations reach `PackView`'s graph
//!   enumeration and every reconstruction of it;
//! * byte identity — a dataset with no declaration-only graph writes exactly the
//!   bytes the writer wrote before the partition was emitted (pinned digests taken
//!   from that writer, and the committed golden);
//! * old packs — a pack written before the change opens and reads exactly as before;
//! * the container frame — a declaration-carrying pack keeps format version 1 and
//!   its three sections, so every v1 decoder (an older release's included) reads it.

use std::sync::Arc;

use purrdf_core::ir::pack::container::{PackBuilder, PackView};
use purrdf_core::term_fixture::iri;
use purrdf_core::{
    BlankScope, DatasetView, GraphMatch, RdfDataset, RdfDatasetBuilder, RdfLiteral, TermValue,
    dataset_from_view, restore_pack, verify_pack,
};
use purrdf_testkit::vectors::sha256_hex;

const GOLDEN_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/goldens/pack_small.bin");

fn blank(label: &str) -> TermValue {
    TermValue::Blank {
        label: label.to_owned(),
        scope: BlankScope::DEFAULT,
    }
}

/// One default-graph row, one named graph with a row, and two declared graphs that
/// own nothing: `ex:empty` (IRI-named) and `_:bg` (blank-named).
fn declared_fixture() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri("http://example.org/s");
    let p = b.intern_iri("http://example.org/p");
    let o = b.intern_iri("http://example.org/o");
    let g1 = b.intern_iri("http://example.org/g1");
    b.push_quad(s, p, o, None);
    b.push_quad(s, p, o, Some(g1));
    let empty = b.intern_iri("http://example.org/empty");
    b.declare_named_graph(empty);
    let bg = b.intern_blank("bg", BlankScope::DEFAULT);
    b.declare_named_graph(bg);
    b.freeze().expect("declared fixture freezes")
}

/// The same rows as [`declared_fixture`] with no declaration-only graph.
fn undeclared_twin() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri("http://example.org/s");
    let p = b.intern_iri("http://example.org/p");
    let o = b.intern_iri("http://example.org/o");
    let g1 = b.intern_iri("http://example.org/g1");
    b.push_quad(s, p, o, None);
    b.push_quad(s, p, o, Some(g1));
    b.freeze().expect("undeclared twin freezes")
}

/// The named graphs `view` enumerates, as values, sorted.
fn graph_values<D: DatasetView>(view: &D) -> Vec<TermValue> {
    let mut values: Vec<TermValue> = view
        .named_graphs()
        .map(|id| view.term_value(id).expect("a graph name resolves"))
        .collect();
    values.sort();
    values
}

fn expected_graphs() -> Vec<TermValue> {
    let mut expected = vec![iri("g1"), iri("empty"), blank("bg")];
    expected.sort();
    expected
}

#[test]
fn a_pack_view_enumerates_iri_and_blank_named_declared_empty_graphs() {
    let dataset = declared_fixture();
    assert_eq!(
        graph_values(&*dataset),
        expected_graphs(),
        "fixture premise"
    );

    let bytes = PackBuilder::build_bytes(&dataset).expect("builds");
    let view = PackView::from_bytes(&bytes).expect("opens");
    assert_eq!(
        graph_values(&view),
        expected_graphs(),
        "the pack must enumerate the IRI- and blank-named declared graphs"
    );

    // A declared graph is addressable and empty, not absent from the dictionary.
    for name in [iri("empty"), blank("bg")] {
        let id = view
            .term_id_by_value(&name)
            .ok()
            .flatten()
            .unwrap_or_else(|| panic!("{name:?} resolves in the pack dictionary"));
        assert_eq!(
            view.quads_for_pattern(None, None, None, GraphMatch::Named(id))
                .count(),
            0,
            "{name:?} holds no rows"
        );
    }
    // The rows are untouched.
    assert_eq!(view.quads().count(), 2);
}

#[test]
fn every_reconstruction_of_a_pack_keeps_its_declared_graphs() {
    let dataset = declared_fixture();
    let bytes = PackBuilder::build_bytes(&dataset).expect("builds");

    let restored = restore_pack(&bytes).expect("restores");
    assert_eq!(graph_values(&*restored), expected_graphs(), "restore_pack");

    let view = PackView::from_bytes(&bytes).expect("opens");
    let rebuilt = dataset_from_view(&view).expect("reconstructs");
    assert_eq!(
        graph_values(&*rebuilt),
        expected_graphs(),
        "dataset_from_view"
    );

    // The round trip is a fixed point: re-packing what came out writes the same bytes.
    assert_eq!(
        PackBuilder::build_bytes(&rebuilt).expect("rebuilds"),
        bytes,
        "pack -> dataset -> pack must reproduce the pack"
    );
}

#[test]
fn declarations_change_the_pack_bytes_but_not_its_canonical_digest() {
    let declared = PackBuilder::build_bytes(&declared_fixture()).expect("builds");
    let plain = PackBuilder::build_bytes(&undeclared_twin()).expect("builds");
    assert!(
        declared != plain,
        "a declaration is content the pack carries, so the bytes differ"
    );

    // RDFC-1.0 N-Quads has no spelling for an empty graph, so the header's canonical
    // digest and the certified digest are the rows' alone.
    let declared_view = PackView::from_bytes(&declared).expect("opens");
    let plain_view = PackView::from_bytes(&plain).expect("opens");
    assert_eq!(declared_view.rdfc_digest(), plain_view.rdfc_digest());
    assert_eq!(
        verify_pack(&declared).expect("certifies"),
        verify_pack(&plain).expect("certifies")
    );
}

#[test]
fn a_declaration_pack_keeps_the_v1_frame() {
    let bytes = PackBuilder::build_bytes(&declared_fixture()).expect("builds");
    assert_eq!(&bytes[0..8], b"PURRPCK1");
    assert_eq!(
        u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
        1,
        "version"
    );
    assert_eq!(
        u32::from_le_bytes(bytes[24..28].try_into().unwrap()),
        3,
        "section count"
    );
    let view = PackView::from_bytes(&bytes).expect("opens");
    assert!(
        view.capabilities().named_graphs,
        "a declared named graph is a named graph the pack carries"
    );
}

// ---------------------------------------------------------------------------
// Byte identity for datasets with no declaration-only graph.
// ---------------------------------------------------------------------------

/// A named graph that owns only a reifier row and one that owns only an annotation
/// row: neither is declaration-only, so neither may gain a partition.
fn side_only_graphs_fixture() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri("http://example.org/s");
    let p = b.intern_iri("http://example.org/p");
    let o = b.intern_iri("http://example.org/o");
    b.push_quad(s, p, o, None);
    let triple = b.intern_triple(s, p, o);
    let r = b.intern_iri("http://example.org/r");
    let g_reifier = b.intern_iri("http://example.org/g-reifier");
    b.push_reifier_in_graph(r, triple, Some(g_reifier));
    let ap = b.intern_iri("http://example.org/ap");
    let ao = b.intern_literal(RdfLiteral {
        lexical_form: "x".to_owned(),
        datatype: None,
        language: Some("en".to_owned()),
        direction: None,
    });
    let g_annotation = b.intern_blank("ga", BlankScope::DEFAULT);
    b.push_annotation_in_graph(r, ap, ao, Some(g_annotation));
    b.freeze().expect("side-only fixture freezes")
}

fn default_only_fixture() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri("http://example.org/s");
    let p = b.intern_iri("http://example.org/p");
    let o = b.intern_blank("o", BlankScope::DEFAULT);
    b.push_quad(s, p, o, None);
    b.freeze().expect("default-only fixture freezes")
}

/// A graph that is declared AND owns a row is not declaration-only.
fn declared_with_rows_fixture() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri("http://example.org/s");
    let p = b.intern_iri("http://example.org/p");
    let o = b.intern_iri("http://example.org/o");
    let bg = b.intern_blank("bg", BlankScope::DEFAULT);
    b.push_quad(s, p, o, Some(bg));
    b.declare_named_graph(bg);
    b.freeze().expect("declared-with-rows fixture freezes")
}

fn empty_fixture() -> Arc<RdfDataset> {
    RdfDatasetBuilder::new()
        .freeze()
        .expect("empty dataset freezes")
}

/// SHA-256 of each fixture's pack as written by the writer BEFORE declaration-only
/// graphs were carried (the same digests that writer produced on the release line).
/// None of these datasets has a declaration-only graph, so none may move.
#[test]
fn packs_without_declaration_only_graphs_are_byte_identical_to_the_previous_writer() {
    let cases: [(&str, Arc<RdfDataset>, &str); 5] = [
        (
            "undeclared twin",
            undeclared_twin(),
            "75b3d660de2236d3eaa857d137af56aefafca7cc9095c34b54bfb1eb738808c8",
        ),
        (
            "side-only graphs",
            side_only_graphs_fixture(),
            "bac9776b24d572b5c451beeb4bb0ae05856ebab35b8db5938b1287a27546cf20",
        ),
        (
            "default only",
            default_only_fixture(),
            "d551952c605d965fe8920e75937cab948d8ba78e8a08e18392fc2cfa3b87ef22",
        ),
        (
            "declared with rows",
            declared_with_rows_fixture(),
            "7eca9cb5e0564c6278b619a91ffdc1dff332bfebc72a98c2de9eeb447714688f",
        ),
        (
            "empty",
            empty_fixture(),
            "9cc1aae139a9a0e47b0717234da430426f1df579608288ae564484903570afe7",
        ),
    ];
    let mut failures = Vec::new();
    for (name, dataset, expected) in cases {
        let actual = sha256_hex(&PackBuilder::build_bytes(&dataset).expect("builds"));
        if actual != expected {
            failures.push(format!("{name}: expected {expected}, got {actual}"));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn the_committed_golden_pack_reads_and_rewrites_unchanged() {
    let golden = std::fs::read(GOLDEN_PATH).expect("the committed golden pack");
    let view = PackView::from_bytes(&golden).expect("an existing pack opens");
    assert_eq!(graph_values(&view), vec![iri("g1")]);
    let restored = restore_pack(&golden).expect("restores");
    assert_eq!(graph_values(&*restored), vec![iri("g1")]);
    assert_eq!(
        PackBuilder::build_bytes(&restored).expect("rebuilds"),
        golden,
        "re-packing an existing pack's content must reproduce it byte for byte"
    );
}

/// A graph that owns only statement-layer rows is a named graph of the dataset, and
/// the pack enumerates it exactly as the frozen dataset does.
#[test]
fn a_pack_view_enumerates_graphs_that_own_only_statement_layer_rows() {
    let dataset = side_only_graphs_fixture();
    let bytes = PackBuilder::build_bytes(&dataset).expect("builds");
    let view = PackView::from_bytes(&bytes).expect("opens");
    let mut expected = vec![iri("g-reifier"), blank("ga")];
    expected.sort();
    assert_eq!(graph_values(&*dataset), expected, "fixture premise");
    assert_eq!(graph_values(&view), expected);
}

/// Partitions, side-table graphs and declarations interleave and overlap; the pack
/// lists each graph once, in ascending id order, exactly as the frozen dataset does.
#[test]
fn a_pack_view_merges_every_graph_source_once_in_ascending_order() {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri("http://example.org/s");
    let p = b.intern_iri("http://example.org/p");
    let o = b.intern_iri("http://example.org/o");
    let triple = b.intern_triple(s, p, o);
    let r = b.intern_iri("http://example.org/r");
    for n in 0..6 {
        let g = b.intern_iri(&format!("http://example.org/g{n}"));
        match n % 3 {
            0 => b.push_quad(s, p, o, Some(g)),
            1 => b.push_reifier_in_graph(r, triple, Some(g)),
            _ => b.declare_named_graph(g),
        }
        if n == 3 {
            // A graph with rows of both kinds.
            b.push_reifier_in_graph(r, triple, Some(g));
        }
    }
    let dataset = b.freeze().expect("freezes");
    let bytes = PackBuilder::build_bytes(&dataset).expect("builds");
    let view = PackView::from_bytes(&bytes).expect("opens");
    let ids: Vec<_> = view.named_graphs().collect();
    assert!(ids.windows(2).all(|w| w[0] < w[1]), "strictly ascending");
    assert_eq!(graph_values(&view), graph_values(&*dataset));
    assert_eq!(ids.len(), 6);
}
