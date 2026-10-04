// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Named-graph state at the serialize boundary.
//!
//! Two defects are pinned here:
//!
//! * selecting a named graph the dataset does not contain emitted the DEFAULT graph's
//!   rows, because the absent name resolved to "no term" and the default graph's rows
//!   carry "no graph" — the two `None`s compared equal. An absent selection emits no
//!   rows at all;
//! * a declared EMPTY named graph — one TriG already writes as `<g> { }` — was dropped
//!   by the JSON-LD, YAML-LD and TriX writers and by the TriX reader, so a round trip
//!   through those carriers silently lost a graph the dataset declared.
//!
//! Each refusal-shaped assertion is paired with a neighbouring case that must still
//! succeed: selecting an existing named graph and the default graph still emit their
//! rows, and a graph that has triples still round-trips beside the empty one.

use std::sync::Arc;

use purrdf_core::RdfDataset;
use purrdf_rdf::native_codecs::{
    NativeRdfFormat, SerializeOptions, StatementLayer, parse_dataset, serialize_dataset_to_format,
    serialize_dataset_with,
};
use purrdf_rdf::{SerializeGraph, TermValue};

const SOURCE: &str = "<http://example.org/s> <http://example.org/p> <http://example.org/default> .\n\
<http://example.org/g1> { <http://example.org/s> <http://example.org/p> <http://example.org/in-g1> . }\n\
<http://example.org/empty> { }\n\
_:blank { }\n";

fn source() -> Arc<RdfDataset> {
    parse_dataset(SOURCE.as_bytes(), NativeRdfFormat::TriG.media_type(), None)
        .expect("fixture parses")
}

fn select(dataset: &RdfDataset, format: NativeRdfFormat, selection: SerializeGraph<'_>) -> String {
    let outcome = serialize_dataset_with(
        dataset,
        format,
        None,
        &SerializeOptions {
            selection,
            statement_layer: StatementLayer::Emit,
            jsonld_options: None,
        },
    )
    .unwrap_or_else(|e| panic!("{format:?} serializes the selection: {e}"));
    String::from_utf8(outcome.bytes).expect("utf-8 output")
}

fn declares_graph(dataset: &RdfDataset, name: &TermValue) -> bool {
    dataset
        .term_id_by_value(name)
        .is_some_and(|id| dataset.named_graphs().any(|g| g == id))
}

fn declared_blank_graphs(dataset: &RdfDataset) -> usize {
    dataset
        .named_graphs()
        .filter(|&g| matches!(dataset.term_value(g), TermValue::Blank { .. }))
        .count()
}

#[test]
fn an_absent_named_selection_emits_no_rows() {
    let dataset = source();
    let absent = TermValue::Iri("http://example.org/not-in-the-dataset".to_owned());
    for format in [
        NativeRdfFormat::NTriples,
        NativeRdfFormat::Turtle,
        NativeRdfFormat::NQuads,
        NativeRdfFormat::TriG,
    ] {
        let text = select(&dataset, format, SerializeGraph::Named(&absent));
        assert!(
            !text.contains("http://example.org/default") && !text.contains("in-g1"),
            "{format:?}: an absent named graph must emit no rows, got:\n{text}"
        );
    }
}

#[test]
fn present_named_and_default_selections_still_emit_their_rows() {
    let dataset = source();
    let g1 = TermValue::Iri("http://example.org/g1".to_owned());
    let named = select(
        &dataset,
        NativeRdfFormat::NTriples,
        SerializeGraph::Named(&g1),
    );
    assert!(named.contains("<http://example.org/in-g1>"), "{named}");
    assert!(!named.contains("<http://example.org/default>"), "{named}");

    let default = select(
        &dataset,
        NativeRdfFormat::NTriples,
        SerializeGraph::DefaultGraph,
    );
    assert!(
        default.contains("<http://example.org/default>"),
        "{default}"
    );
    assert!(!default.contains("<http://example.org/in-g1>"), "{default}");

    // An existing but EMPTY named graph is a real selection with no rows.
    let empty = TermValue::Iri("http://example.org/empty".to_owned());
    let empty_text = select(
        &dataset,
        NativeRdfFormat::NTriples,
        SerializeGraph::Named(&empty),
    );
    assert!(empty_text.trim().is_empty(), "{empty_text}");
}

fn assert_empty_graphs_round_trip(format: NativeRdfFormat, blank_supported: bool) {
    let dataset = source();
    assert!(declares_graph(
        &dataset,
        &TermValue::Iri("http://example.org/empty".to_owned())
    ));
    assert_eq!(declared_blank_graphs(&dataset), 1);

    let bytes = serialize_dataset_to_format(dataset.as_ref(), format, None)
        .unwrap_or_else(|e| panic!("{format:?} serializes: {e}"))
        .bytes;
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let reparsed = parse_dataset(&bytes, format.media_type(), None)
        .unwrap_or_else(|e| panic!("{format:?} reparses: {e}\n{text}"));

    assert!(
        declares_graph(
            &reparsed,
            &TermValue::Iri("http://example.org/empty".to_owned())
        ),
        "{format:?} lost the declared empty IRI-named graph:\n{text}"
    );
    assert!(
        declares_graph(
            &reparsed,
            &TermValue::Iri("http://example.org/g1".to_owned())
        ),
        "{format:?} lost the populated named graph:\n{text}"
    );
    if blank_supported {
        assert_eq!(
            declared_blank_graphs(&reparsed),
            1,
            "{format:?} lost the declared empty blank-named graph:\n{text}"
        );
    }
    assert_eq!(reparsed.quad_count(), dataset.quad_count(), "{format:?}");
    assert_eq!(
        reparsed.named_graphs().count(),
        dataset.named_graphs().count(),
        "{format:?}:\n{text}"
    );
}

#[test]
fn trig_round_trips_declared_empty_graphs() {
    assert_empty_graphs_round_trip(NativeRdfFormat::TriG, true);
}

#[test]
fn jsonld_round_trips_declared_empty_graphs() {
    assert_empty_graphs_round_trip(NativeRdfFormat::JsonLd, true);
}

#[test]
fn yamlld_round_trips_declared_empty_graphs() {
    assert_empty_graphs_round_trip(NativeRdfFormat::YamlLd, true);
}

#[test]
fn trix_round_trips_declared_empty_graphs() {
    assert_empty_graphs_round_trip(NativeRdfFormat::TriX, true);
}

/// The TriX reader on its own: an empty `<graph>` block naming a graph is a declared
/// empty graph, while an empty block with no name is the (always present) default graph.
#[test]
fn trix_parser_reads_an_empty_graph_block_as_a_declared_graph() {
    let doc = br#"<?xml version="1.0"?>
<TriX xmlns="http://www.w3.org/2004/03/trix/trix-1/">
  <graph><uri>http://example.org/empty</uri></graph>
  <graph></graph>
</TriX>"#;
    let parsed = parse_dataset(doc, NativeRdfFormat::TriX.media_type(), None).expect("parses");
    assert!(declares_graph(
        &parsed,
        &TermValue::Iri("http://example.org/empty".to_owned())
    ));
    assert_eq!(parsed.named_graphs().count(), 1);
    assert_eq!(parsed.quad_count(), 0);
}

/// Every format that writes the whole dataset either spells a declared empty graph or
/// lists it among `empty_named_graphs_dropped` — never neither. The list is exact: it
/// names the IRI- and blank-named empty graphs and never `g1`, which owns a row.
#[test]
fn a_whole_dataset_serialization_lists_each_empty_graph_it_cannot_spell() {
    let dataset = source();
    for format in NativeRdfFormat::all() {
        let dropped =
            purrdf_rdf::empty_named_graphs_dropped(&*dataset, format, SerializeGraph::Dataset)
                .expect("lists");
        let text = select(&dataset, format, SerializeGraph::Dataset);
        let spelled = text.contains("http://example.org/empty");
        if format.carries_empty_named_graphs() {
            assert!(spelled, "{format:?} writes the empty graph, got:\n{text}");
            assert!(dropped.is_empty(), "{format:?}: {dropped:?}");
        } else {
            assert!(
                !spelled,
                "{format:?} has no empty-graph spelling, got:\n{text}"
            );
            assert_eq!(dropped.len(), 2, "{format:?}: {dropped:?}");
            assert_eq!(
                dropped[0],
                TermValue::Iri("http://example.org/empty".to_owned()),
                "{format:?}"
            );
            assert!(matches!(dropped[1], TermValue::Blank { .. }), "{format:?}");
        }
    }
    assert!(NativeRdfFormat::TriG.carries_empty_named_graphs());
    assert!(!NativeRdfFormat::NQuads.carries_empty_named_graphs());
    assert!(!NativeRdfFormat::HexTuples.carries_empty_named_graphs());
}

/// The neighbouring cases list nothing: a dataset with no declared empty graph, and a
/// selection the caller spelled out (default graph, or one named graph).
#[test]
fn nothing_is_listed_without_a_declared_empty_graph_or_for_a_chosen_selection() {
    let plain = parse_dataset(
        "<http://example.org/g1> { <http://example.org/s> <http://example.org/p> <http://example.org/o> . }\n"
            .as_bytes(),
        NativeRdfFormat::TriG.media_type(),
        None,
    )
    .expect("parses");
    let declared = source();
    let g1 = TermValue::Iri("http://example.org/g1".to_owned());
    for format in [
        NativeRdfFormat::NQuads,
        NativeRdfFormat::HexTuples,
        NativeRdfFormat::Turtle,
    ] {
        let list = |dataset: &RdfDataset, selection| {
            purrdf_rdf::empty_named_graphs_dropped(dataset, format, selection).expect("lists")
        };
        assert!(
            list(&plain, SerializeGraph::Dataset).is_empty(),
            "{format:?}"
        );
        assert!(
            list(&declared, SerializeGraph::DefaultGraph).is_empty(),
            "{format:?}"
        );
        assert!(
            list(&declared, SerializeGraph::Named(&g1)).is_empty(),
            "{format:?}"
        );
    }
}
