// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Named-graph existence across SPARQL **UPDATE**.
//!
//! The update engine's store model is that a named graph exists iff it holds a quad,
//! with one carve-out: a graph the INPUT declared empty (a TriG `GRAPH <g> {}`) keeps
//! existing until an operation removes it. These tests pin both halves through the
//! public engine, at the three places a graph is observable: the frozen dataset's
//! `named_graphs()`, a later `GRAPH ?g` query over it, and `GRAPH ?g` evaluated
//! inside the same request (which reads the request's snapshot).

use std::sync::Arc;

use purrdf_core::{
    BlankScope, RdfDataset, RdfDatasetBuilder, SparqlEngine, SparqlRequest, SparqlResult, TermValue,
};
use purrdf_sparql_eval::NativeSparqlEngine;

const EX: &str = "http://example.org/";

/// The default graph holds `ex:a ex:p ex:c`; `ex:g` and `_:bg` are declared empty;
/// `ex:h` holds `ex:a ex:p ex:c`; `ex:k` holds `ex:a ex:p ex:d`.
fn fixture() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let a = b.intern_iri(&format!("{EX}a"));
    let p = b.intern_iri(&format!("{EX}p"));
    let c = b.intern_iri(&format!("{EX}c"));
    let d = b.intern_iri(&format!("{EX}d"));
    let g = b.intern_iri(&format!("{EX}g"));
    let h = b.intern_iri(&format!("{EX}h"));
    let k = b.intern_iri(&format!("{EX}k"));
    let bg = b.intern_blank("bg", BlankScope::DEFAULT);
    b.push_quad(a, p, c, None);
    b.declare_named_graph(g);
    b.declare_named_graph(bg);
    b.push_quad(a, p, c, Some(h));
    b.push_quad(a, p, d, Some(k));
    b.freeze().expect("fixture freezes")
}

fn name(value: &TermValue) -> String {
    match value {
        TermValue::Iri(iri) => iri.trim_start_matches(EX).to_owned(),
        TermValue::Blank { label, .. } => format!("_:{label}"),
        other => panic!("a graph name is an IRI or a blank node, not {other:?}"),
    }
}

fn request(text: &str) -> SparqlRequest<'_> {
    SparqlRequest {
        query: text,
        base_iri: None,
        substitutions: &[],
    }
}

fn select_names(engine: &NativeSparqlEngine, ds: &Arc<RdfDataset>, query: &str) -> Vec<String> {
    let text = format!("PREFIX ex: <{EX}>\n{query}");
    let SparqlResult::Solutions { rows, .. } = engine.query(ds, request(&text)).expect("query")
    else {
        panic!("SELECT returns solutions")
    };
    let mut names: Vec<String> = rows
        .into_iter()
        .map(|row| name(row[0].as_ref().expect("bound")))
        .collect();
    names.sort();
    names
}

/// Apply `update` to the fixture and return the named graphs that exist afterwards,
/// asserting that the frozen dataset, a `GRAPH ?g` query over it, and a `GRAPH ?g`
/// evaluated at the end of the same request all agree.
fn graphs_after(update: &str) -> Vec<String> {
    let engine = NativeSparqlEngine::new();
    let mut ds = fixture();
    let text = format!("PREFIX ex: <{EX}>\n{update}");
    engine.update(&mut ds, request(&text)).expect("update");
    let mut frozen: Vec<String> = ds
        .named_graphs()
        .map(|g| name(&RdfDataset::term_value(&ds, g)))
        .collect();
    frozen.sort();
    let queried = select_names(&engine, &ds, "SELECT ?g WHERE { GRAPH ?g {} }");
    assert_eq!(frozen, queried, "`GRAPH ?g` enumerates the frozen graphs");

    // The same request, with a trailing operation whose WHERE enumerates `GRAPH ?g`
    // over the request's snapshot and records each name into a log graph.
    let mut within = fixture();
    let text = format!(
        "PREFIX ex: <{EX}>\n{update} ;\n\
         INSERT {{ GRAPH ex:log {{ ex:log ex:saw ?g }} }} WHERE {{ GRAPH ?g {{}} }}"
    );
    engine.update(&mut within, request(&text)).expect("update");
    let seen = select_names(
        &engine,
        &within,
        "SELECT ?g WHERE { GRAPH ex:log { ex:log ex:saw ?g } }",
    );
    assert_eq!(frozen, seen, "`GRAPH ?g` inside the request agrees");
    frozen
}

fn default_quads_after(update: &str) -> usize {
    let engine = NativeSparqlEngine::new();
    let mut ds = fixture();
    let text = format!("PREFIX ex: <{EX}>\n{update}");
    engine.update(&mut ds, request(&text)).expect("update");
    ds.quads().filter(|q| q.g.is_none()).count()
}

const ALL: [&str; 4] = ["_:bg", "g", "h", "k"];

// -- operations that remove a graph -------------------------------------------------

#[test]
fn drop_graph_removes_a_populated_graph() {
    assert_eq!(graphs_after("DROP GRAPH ex:h"), ["_:bg", "g", "k"]);
}

#[test]
fn drop_graph_removes_a_declared_empty_graph() {
    assert_eq!(graphs_after("DROP GRAPH ex:g"), ["_:bg", "h", "k"]);
}

#[test]
fn clear_graph_is_drop_graph() {
    assert_eq!(graphs_after("CLEAR GRAPH ex:h"), ["_:bg", "g", "k"]);
    assert_eq!(graphs_after("CLEAR GRAPH ex:g"), ["_:bg", "h", "k"]);
}

#[test]
fn drop_named_and_clear_named_remove_every_named_graph_and_keep_the_default() {
    for op in ["DROP NAMED", "CLEAR NAMED"] {
        assert!(graphs_after(op).is_empty(), "{op}");
        assert_eq!(default_quads_after(op), 1, "{op} keeps the default graph");
    }
}

#[test]
fn drop_all_and_clear_all_remove_everything() {
    for op in ["DROP ALL", "CLEAR ALL"] {
        assert!(graphs_after(op).is_empty(), "{op}");
        assert_eq!(default_quads_after(op), 0, "{op} empties the default graph");
    }
}

#[test]
fn move_removes_its_source_graph() {
    assert_eq!(graphs_after("MOVE ex:h TO ex:m"), ["_:bg", "g", "k", "m"]);
    // Into a declared empty graph: the destination is populated, the source is gone.
    assert_eq!(graphs_after("MOVE ex:h TO ex:g"), ["_:bg", "g", "k"]);
    // From a declared empty graph: both the source and the cleared destination go.
    assert_eq!(graphs_after("MOVE ex:g TO ex:k"), ["_:bg", "h"]);
}

#[test]
fn copy_of_an_empty_source_removes_its_destination() {
    assert_eq!(graphs_after("COPY ex:g TO ex:h"), ["_:bg", "g", "k"]);
    assert_eq!(graphs_after("COPY DEFAULT TO ex:g"), ALL);
}

#[test]
fn deleting_the_last_quad_of_a_graph_removes_it() {
    assert_eq!(
        graphs_after("DELETE DATA { GRAPH ex:h { ex:a ex:p ex:c } }"),
        ["_:bg", "g", "k"]
    );
    assert_eq!(
        graphs_after("DELETE WHERE { GRAPH ex:k { ?s ?p ?o } }"),
        ["_:bg", "g", "h"]
    );
}

// -- neighbouring cases that must keep their graphs ---------------------------------

#[test]
fn a_no_op_update_keeps_every_graph_including_the_declared_empty_ones() {
    assert_eq!(graphs_after("DELETE DATA { ex:none ex:p ex:none }"), ALL);
    // Moving, copying or adding a graph onto itself is a no-op too.
    assert_eq!(graphs_after("MOVE ex:g TO ex:g"), ALL);
    assert_eq!(graphs_after("COPY ex:g TO ex:g"), ALL);
}

#[test]
fn an_unrelated_insert_keeps_the_declared_empty_graphs() {
    assert_eq!(graphs_after("INSERT DATA { ex:x ex:p ex:y }"), ALL);
    assert_eq!(
        graphs_after("INSERT DATA { GRAPH ex:new { ex:x ex:p ex:y } }"),
        ["_:bg", "g", "h", "k", "new"]
    );
}

#[test]
fn adding_an_empty_graph_creates_nothing() {
    assert_eq!(graphs_after("ADD ex:g TO ex:m"), ALL);
}

#[test]
fn deleting_part_of_a_graph_keeps_it() {
    assert_eq!(
        graphs_after("DELETE DATA { GRAPH ex:h { ex:a ex:p ex:none } }"),
        ALL
    );
}

#[test]
fn a_graph_repopulated_after_drop_in_the_same_request_exists() {
    assert_eq!(
        graphs_after("DROP GRAPH ex:h ; INSERT DATA { GRAPH ex:h { ex:x ex:p ex:y } }"),
        ALL
    );
    assert_eq!(
        graphs_after("DROP GRAPH ex:g ; INSERT DATA { GRAPH ex:g { ex:x ex:p ex:y } }"),
        ALL
    );
    assert_eq!(
        graphs_after("DROP ALL ; INSERT DATA { GRAPH ex:k { ex:x ex:p ex:y } }"),
        ["k"]
    );
}

#[test]
fn dropping_the_default_graph_keeps_every_named_graph() {
    assert_eq!(graphs_after("DROP DEFAULT"), ALL);
    assert_eq!(default_quads_after("DROP DEFAULT"), 0);
}

#[test]
fn dropping_a_missing_graph_succeeds_and_changes_nothing() {
    // The store has no missing-graph condition: DROP without SILENT succeeds.
    assert_eq!(graphs_after("DROP GRAPH ex:missing"), ALL);
    assert_eq!(graphs_after("DROP SILENT GRAPH ex:missing"), ALL);
}

#[test]
fn create_graph_registers_nothing() {
    assert_eq!(graphs_after("CREATE GRAPH ex:new"), ALL);
}
