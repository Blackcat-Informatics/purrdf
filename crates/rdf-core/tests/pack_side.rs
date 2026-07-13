// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Falsifiable acceptance tests for the RDF 1.2 reifier/annotation side-tables
//! codec (`purrdf_core::ir::pack::side`, Task 4 of the succinct-pack-codec
//! feature): `SideTablesRef::reifier_quads`/`annotation_quads`/
//! `annotations_of_with_graph` must set-equal `RdfDataset`'s own
//! `DatasetView::reifier_quads`/`annotation_quads`/`annotations_of_with_graph`
//! — before AND after a `to_bytes`/`from_bytes` round trip — and the computed
//! capability flags must match `RdfDataset::capabilities()`.

use std::collections::HashSet;

use purrdf_core::ir::pack::dict::PackDict;
use purrdf_core::ir::pack::side::{self, SideTables, SideTablesRef};
use purrdf_core::{RdfDataset, RdfDatasetBuilder, TermId, TermValue};

/// One resolved reifier/annotation row, in dataset-independent `TermValue` form.
type ValueQuad = (TermValue, TermValue, TermValue, Option<TermValue>);

/// The fixture: quoted/triple terms (one used as a base quad's object, one
/// side-table-only), multiple reifiers binding one triple, a reifier binding
/// across a named graph that owns NO base quad of its own (the side-table-only
/// named-graph path), and several annotations per reifier spanning the default
/// graph and named graphs.
struct Fixture {
    dataset: std::sync::Arc<RdfDataset>,
    dict: PackDict,
    side_bytes: Vec<u8>,
    r1: TermId,
    r2: TermId,
    r3: TermId,
}

fn build_fixture() -> Fixture {
    let mut b = RdfDatasetBuilder::new();

    let s1 = b.intern_iri("http://example.org/s1");
    let p1 = b.intern_iri("http://example.org/p1");
    let o1 = b.intern_iri("http://example.org/o1");
    let triple1 = b.intern_triple(s1, p1, o1);

    // A second, side-table-only triple term: never a base quad's S/P/O, only
    // reachable via `r3`'s reifier row — exercises the Task 4 closure's
    // recursive fold-in of a triple term's own components.
    let s4 = b.intern_iri("http://example.org/s4");
    let p4 = b.intern_iri("http://example.org/p4");
    let o4 = b.intern_iri("http://example.org/o4");
    let triple2 = b.intern_triple(s4, p4, o4);

    let s3 = b.intern_iri("http://example.org/s3");
    let p3 = b.intern_iri("http://example.org/p3");
    let s5 = b.intern_iri("http://example.org/s5");
    let p5 = b.intern_iri("http://example.org/p5");
    let o5 = b.intern_iri("http://example.org/o5");
    let g1 = b.intern_iri("http://example.org/g1");
    let g2 = b.intern_iri("http://example.org/g2");

    // -- Base quads -----------------------------------------------------------
    // `triple1` also appears as a base quad's OBJECT (a quoted-triple use, not
    // side-table-only); `g2` owns a base quad (so its `named_graphs` membership
    // is NOT purely a side-table artifact, unlike `g1` below).
    b.push_quad(s3, p3, triple1, None);
    b.push_quad(s5, p5, o5, Some(g2));

    // -- Reifiers ---------------------------------------------------------------
    let r1 = b.intern_iri("http://example.org/r1");
    let r2 = b.intern_iri("http://example.org/r2");
    let r3 = b.intern_iri("http://example.org/r3");
    b.push_reifier(r1, triple1); // multiple reifiers binding ONE triple...
    b.push_reifier(r2, triple1); // ...(r1, r2 both bind triple1).
    // `r3` binds `triple2` INSIDE named graph `g1`, which owns NO base quad of
    // its own — `g1` is a named graph purely via this side-table reference.
    b.push_reifier_in_graph(r3, triple2, Some(g1));

    // -- Annotations --------------------------------------------------------
    let ap1 = b.intern_iri("http://example.org/ap1");
    let ao1 = b.intern_iri("http://example.org/ao1");
    let ap2 = b.intern_iri("http://example.org/ap2");
    let ao2 = b.intern_iri("http://example.org/ao2");
    let ap3 = b.intern_iri("http://example.org/ap3");
    let ao3 = b.intern_iri("http://example.org/ao3");
    let ap4 = b.intern_iri("http://example.org/ap4");
    let ao4 = b.intern_iri("http://example.org/ao4");
    // `r1` gets TWO annotations: one in the default graph, one in `g2`.
    b.push_annotation(r1, ap1, ao1);
    b.push_annotation_in_graph(r1, ap2, ao2, Some(g2));
    // `r2` gets one default-graph annotation.
    b.push_annotation(r2, ap3, ao3);
    // `r3` gets one annotation in `g1` (the side-table-only named graph).
    b.push_annotation_in_graph(r3, ap4, ao4, Some(g1));

    let dataset = b.freeze().expect("valid dataset");

    let dict_bytes = PackDict::encode(&dataset).to_bytes();
    let dict = PackDict::open(&dict_bytes).expect("dict opens");
    let side_bytes = SideTables::encode(&dict, &dataset).to_bytes();

    Fixture {
        dataset,
        dict,
        side_bytes,
        r1,
        r2,
        r3,
    }
}

/// Resolve `RdfDataset::reifier_quads`/`annotation_quads`'s `QuadIds` stream to
/// dataset-independent `TermValue` tuples.
fn source_value_quads(
    dataset: &RdfDataset,
    rows: impl Iterator<Item = purrdf_core::QuadIds>,
) -> HashSet<ValueQuad> {
    rows.map(|q| {
        (
            dataset.term_value(q.s),
            dataset.term_value(q.p),
            dataset.term_value(q.o),
            q.g.map(|g| dataset.term_value(g)),
        )
    })
    .collect()
}

/// Resolve `SideTablesRef`'s unified-id row stream to `TermValue` tuples.
fn pack_value_quads(
    dict: &PackDict,
    rows: impl Iterator<Item = (u64, u64, u64, Option<u64>)>,
) -> HashSet<ValueQuad> {
    rows.map(|(s, p, o, g)| {
        (
            dict.term_value(s),
            dict.term_value(p),
            dict.term_value(o),
            g.map(|id| dict.term_value(id)),
        )
    })
    .collect()
}

#[test]
fn reifier_quads_set_equals_source() {
    let fx = build_fixture();
    let side = SideTablesRef::from_bytes(&fx.side_bytes).expect("opens");

    let expected = source_value_quads(&fx.dataset, fx.dataset.reifier_quads());
    let actual = pack_value_quads(&fx.dict, side.reifier_quads());
    assert_eq!(actual, expected);
    assert_eq!(actual.len(), 3, "r1, r2, r3");
}

#[test]
fn annotation_quads_set_equals_source() {
    let fx = build_fixture();
    let side = SideTablesRef::from_bytes(&fx.side_bytes).expect("opens");

    let expected = source_value_quads(&fx.dataset, fx.dataset.annotation_quads());
    let actual = pack_value_quads(&fx.dict, side.annotation_quads());
    assert_eq!(actual, expected);
    assert_eq!(actual.len(), 4, "ap1/ao1, ap2/ao2, ap3/ao3, ap4/ao4");
}

#[test]
fn annotations_of_with_graph_set_equals_source_for_every_reifier() {
    let fx = build_fixture();
    let side = SideTablesRef::from_bytes(&fx.side_bytes).expect("opens");

    for &reifier in &[fx.r1, fx.r2, fx.r3] {
        let reifier_value = fx.dataset.term_value(reifier);
        let pack_reifier = fx
            .dict
            .id_by_value(&reifier_value)
            .or_else(|| fx.dict.predicate_id_by_value(&reifier_value))
            .expect("reifier present in dict");

        let expected: HashSet<(TermValue, TermValue, Option<TermValue>)> = fx
            .dataset
            .annotations_of_with_graph(reifier)
            .map(|(p, o, g)| {
                (
                    fx.dataset.term_value(p),
                    fx.dataset.term_value(o),
                    g.map(|g| fx.dataset.term_value(g)),
                )
            })
            .collect();
        let actual: HashSet<(TermValue, TermValue, Option<TermValue>)> = side
            .annotations_of_with_graph(pack_reifier)
            .map(|(p, o, g)| {
                (
                    fx.dict.term_value(p),
                    fx.dict.term_value(o),
                    g.map(|g| fx.dict.term_value(g)),
                )
            })
            .collect();
        assert_eq!(actual, expected, "mismatch for reifier {reifier_value:?}");
    }
}

#[test]
fn to_bytes_from_bytes_round_trip_preserves_every_view() {
    let fx = build_fixture();

    // Re-derive from scratch a second time (encode determinism) and open THAT.
    let dict_bytes2 = PackDict::encode(&fx.dataset).to_bytes();
    let dict2 = PackDict::open(&dict_bytes2).expect("dict opens");
    let side_bytes2 = SideTables::encode(&dict2, &fx.dataset).to_bytes();
    assert_eq!(
        fx.side_bytes, side_bytes2,
        "SideTables::encode is deterministic"
    );

    let before = SideTablesRef::from_bytes(&fx.side_bytes).expect("opens");
    let after = SideTablesRef::from_bytes(&side_bytes2).expect("opens");

    let expected_reifier = source_value_quads(&fx.dataset, fx.dataset.reifier_quads());
    let expected_annotation = source_value_quads(&fx.dataset, fx.dataset.annotation_quads());

    assert_eq!(
        pack_value_quads(&fx.dict, before.reifier_quads()),
        expected_reifier
    );
    assert_eq!(
        pack_value_quads(&dict2, after.reifier_quads()),
        expected_reifier
    );
    assert_eq!(
        pack_value_quads(&fx.dict, before.annotation_quads()),
        expected_annotation
    );
    assert_eq!(
        pack_value_quads(&dict2, after.annotation_quads()),
        expected_annotation
    );

    for &reifier in &[fx.r1, fx.r2, fx.r3] {
        let reifier_value = fx.dataset.term_value(reifier);
        let expected: HashSet<(TermValue, TermValue, Option<TermValue>)> = fx
            .dataset
            .annotations_of_with_graph(reifier)
            .map(|(p, o, g)| {
                (
                    fx.dataset.term_value(p),
                    fx.dataset.term_value(o),
                    g.map(|g| fx.dataset.term_value(g)),
                )
            })
            .collect();

        for (dict, side) in [(&fx.dict, &before), (&dict2, &after)] {
            let pack_reifier = dict
                .id_by_value(&reifier_value)
                .or_else(|| dict.predicate_id_by_value(&reifier_value))
                .expect("reifier present in dict");
            let actual: HashSet<(TermValue, TermValue, Option<TermValue>)> = side
                .annotations_of_with_graph(pack_reifier)
                .map(|(p, o, g)| {
                    (
                        dict.term_value(p),
                        dict.term_value(o),
                        g.map(|g| dict.term_value(g)),
                    )
                })
                .collect();
            assert_eq!(actual, expected);
        }
    }
}

#[test]
fn capability_flags_match_source() {
    let fx = build_fixture();
    let side = SideTablesRef::from_bytes(&fx.side_bytes).expect("opens");

    // Stand-in for what Task 6's `Triples`-derived flag would supply: whether
    // any BASE quad names a graph.
    let base_named_graphs = fx.dataset.quads().any(|q| q.g.is_some());
    assert!(base_named_graphs, "g2 owns a base quad in this fixture");

    let expected = fx.dataset.capabilities();
    let actual = side::capabilities(&fx.dict, &side, base_named_graphs);

    assert_eq!(actual.quoted_triples, expected.quoted_triples);
    assert_eq!(actual.reifiers, expected.reifiers);
    assert_eq!(actual.annotations, expected.annotations);
    assert_eq!(actual.named_graphs, expected.named_graphs);
    assert!(actual.quoted_triples);
    assert!(actual.reifiers);
    assert!(actual.annotations);
    assert!(actual.named_graphs);
}

#[test]
fn capability_named_graphs_true_from_side_table_alone() {
    // A dataset whose ONLY named-graph reference is a reifier row (no base quad
    // anywhere names a graph) — proves `side::capabilities` does not silently
    // depend on `base_named_graphs` being true.
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri("http://example.org/s");
    let p = b.intern_iri("http://example.org/p");
    let o = b.intern_iri("http://example.org/o");
    let triple = b.intern_triple(s, p, o);
    let r = b.intern_iri("http://example.org/r");
    let g = b.intern_iri("http://example.org/g");
    b.push_reifier_in_graph(r, triple, Some(g));
    let dataset = b.freeze().expect("valid dataset");
    assert!(dataset.quads().next().is_none(), "no base quads at all");

    let dict = PackDict::open(&PackDict::encode(&dataset).to_bytes()).expect("dict opens");
    let side_bytes = SideTables::encode(&dict, &dataset).to_bytes();
    let side = SideTablesRef::from_bytes(&side_bytes).expect("opens");

    let expected = dataset.capabilities();
    let actual = side::capabilities(&dict, &side, false);
    assert_eq!(actual.named_graphs, expected.named_graphs);
    assert!(actual.named_graphs);
}
