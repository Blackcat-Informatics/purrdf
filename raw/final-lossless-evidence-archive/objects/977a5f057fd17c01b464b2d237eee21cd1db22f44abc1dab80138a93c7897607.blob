// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
//! Execute existing frozen vendor input data through the native Rust route.
//! No Python execution or upstream implementation body is reused.
use purrdf_core::TermValue;
use purrdf_sparql_eval::{NativeSparqlEngine, QueryOptions};
use std::{collections::BTreeSet, env, fs};

fn main() {
    let path = env::args().nth(1).expect("frozen fixture path required");
    let source = fs::read_to_string(&path).expect("read frozen fixture");
    let source = source
        .split_once("def test_nested_filter_outermost_binding_propagation()")
        .expect("exact frozen test identity")
        .1;
    // These are existing Turtle and SPARQL fixture literals, not Python code.
    let turtle = source
        .split_once("historic_ontology_graph_data = \"\"\"\\\n")
        .expect("frozen Turtle start")
        .1
        .split_once("\"\"\"")
        .expect("frozen Turtle end")
        .0;
    let query = source
        .split_once("query = \"\"\"\\\n")
        .expect("frozen SPARQL start")
        .1
        .split_once("\"\"\"")
        .expect("frozen SPARQL end")
        .0;
    let dataset = purrdf_rdf::parse_dataset(turtle.as_bytes(), "text/turtle", None)
        .expect("native Turtle parse");
    let engine = NativeSparqlEngine::new();
    let prepared = engine
        .prepare_rdflib_query(query, None, &[], QueryOptions::EMPTY)
        .expect("native contextual prepare");
    let result = engine
        .query_rdflib_prepared_view(&*dataset, &prepared, QueryOptions::EMPTY)
        .expect("native contextual execution");
    let (columns, rows) = result.solutions().expect("SELECT result");
    println!("fixture={path}\ncolumns={columns:?}\nrows={rows:?}");
    let answers: BTreeSet<_> = rows.iter().cloned().collect();
    assert_eq!(
        answers,
        BTreeSet::from([vec![
            Some(TermValue::iri("http://example.org/kb/action-1-2")),
            Some(TermValue::iri("http://example.org/kb/record-123-1")),
            Some(TermValue::iri("http://example.org/kb/record-1-2")),
        ]])
    );
    println!("exact frozen vendor witness native PASS");
}
