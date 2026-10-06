// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `PreparedExecution::check_parameters_mentioned`: a declared parameter the query never
//! mentions binds nothing, so a host that wants that refused gets it named — and every
//! place a query CAN mention a variable still counts as a mention.
//!
//! `prepare_execution` itself does not refuse an unmentioned parameter: SHACL-SPARQL
//! declares `$this` for every constraint whether or not its body reads it, which is valid
//! SHACL. `prepare_still_admits_an_unmentioned_parameter` pins that the check is a
//! separate, opt-in step rather than a change to what `prepare_execution` admits.

use purrdf_core::{RdfDatasetBuilder, TermValue};
use purrdf_sparql_eval::{InternedOutcome, NativeSparqlEngine, QueryOptions};

const P: &str = "http://example.org/p";
const SELECT: &str = "SELECT ?o WHERE { ?this <http://example.org/p> ?o }";

fn prepared(query: &str, parameters: &[&str]) -> purrdf_sparql_eval::PreparedExecution {
    NativeSparqlEngine::new()
        .prepare_execution(query, None, parameters, QueryOptions::EMPTY)
        .unwrap_or_else(|error| panic!("{query}: {error}"))
}

#[test]
fn an_unmentioned_parameter_is_refused_naming_it() {
    let error = prepared(SELECT, &["zz"])
        .check_parameters_mentioned()
        .expect_err("a parameter the query never mentions");
    let message = error.to_string();
    assert!(
        message.contains("\"zz\"") && message.contains("never mentions"),
        "{message}"
    );
    let presentation = error.presentation().expect("a typed presentation");
    assert_eq!(
        presentation.message_id(),
        "sparql-prepared-parameter-unmentioned"
    );
}

#[test]
fn only_the_unmentioned_parameters_are_named_in_declaration_order() {
    let error = prepared(SELECT, &["zz", "this", "aa"])
        .check_parameters_mentioned()
        .expect_err("two unmentioned parameters");
    let message = error.to_string();
    assert!(message.contains("\"zz\", \"aa\""), "{message}");
    assert!(!message.contains("\"this\""), "{message}");
}

#[test]
fn a_declared_used_parameter_passes_and_runs() {
    let mut builder = RdfDatasetBuilder::new();
    let s = builder.intern_iri("http://example.org/s");
    let p = builder.intern_iri(P);
    let o = builder.intern_iri("http://example.org/o");
    builder.push_quad(s, p, o, None);
    let dataset = builder.freeze().expect("the one-triple fixture");

    let engine = NativeSparqlEngine::new();
    let mut execution = engine
        .prepare_execution(SELECT, None, &["this"], QueryOptions::EMPTY)
        .expect("prepare");
    execution
        .check_parameters_mentioned()
        .expect("a mentioned parameter");
    execution
        .bind(0, TermValue::Iri("http://example.org/s".to_owned()))
        .expect("bind");
    let rows = engine
        .execute(&mut execution, &*dataset, QueryOptions::EMPTY, |outcome| {
            let InternedOutcome::Solutions(solutions) = outcome else {
                panic!("expected solutions");
            };
            solutions.len()
        })
        .expect("execute");
    assert_eq!(rows, 1);
}

#[test]
fn every_position_a_query_can_mention_a_variable_counts() {
    for query in [
        // A triple term, either sigil.
        "SELECT ?o WHERE { $this <http://example.org/p> ?o }",
        // A variable predicate, and a graph name.
        "SELECT ?o WHERE { ?s ?this ?o }",
        "SELECT ?o WHERE { GRAPH ?this { ?s ?p ?o } }",
        // A quoted triple term.
        "SELECT ?o WHERE { ?s <http://example.org/p> <<( ?this ?p ?o )>> }",
        // A property path.
        "SELECT ?o WHERE { ?this <http://example.org/p>+ ?o }",
        // Expression-only reads: FILTER, inside EXISTS / NOT EXISTS, BOUND, ORDER BY.
        "SELECT ?o WHERE { ?s <http://example.org/p> ?o FILTER(?o != ?this) }",
        "SELECT ?o WHERE { ?s <http://example.org/p> ?o FILTER EXISTS { ?this ?p ?o } }",
        "SELECT ?o WHERE { ?s <http://example.org/p> ?o FILTER NOT EXISTS { ?this ?p ?o } }",
        "SELECT ?o WHERE { ?s <http://example.org/p> ?o FILTER(BOUND(?this)) }",
        "SELECT ?o WHERE { ?s <http://example.org/p> ?o } ORDER BY ?this",
        // OPTIONAL, MINUS, a sub-SELECT.
        "SELECT ?o WHERE { ?s <http://example.org/p> ?o OPTIONAL { ?this ?p ?o } }",
        "SELECT ?o WHERE { ?s <http://example.org/p> ?o MINUS { ?this ?p ?o } }",
        "SELECT ?o WHERE { { SELECT ?o WHERE { ?this <http://example.org/p> ?o } } }",
        // Projection, GROUP BY, an aggregate argument.
        "SELECT ?this WHERE { }",
        "SELECT (COUNT(*) AS ?n) WHERE { ?s ?p ?o } GROUP BY ?this",
        "SELECT (SAMPLE(?this) AS ?n) WHERE { ?s ?p ?o }",
        // BIND's expression and VALUES.
        "SELECT ?o WHERE { BIND(?this AS ?o) }",
        "SELECT ?o WHERE { VALUES ?this { <http://example.org/s> } ?this ?p ?o }",
        // The head of the other query forms.
        "CONSTRUCT { ?this <http://example.org/q> ?o } WHERE { ?s <http://example.org/p> ?o }",
        "CONSTRUCT { GRAPH ?this { ?s ?p ?o } } WHERE { ?s ?p ?o }",
        "DESCRIBE ?this",
        "ASK { ?this ?p ?o }",
    ] {
        prepared(query, &["this"])
            .check_parameters_mentioned()
            .unwrap_or_else(|error| panic!("{query}: {error}"));
    }
}

#[test]
fn no_parameter_is_nothing_to_refuse() {
    prepared("ASK { }", &[])
        .check_parameters_mentioned()
        .expect("no parameter declared");
}

#[test]
fn prepare_still_admits_an_unmentioned_parameter() {
    let execution = prepared(SELECT, &["this", "other"]);
    assert_eq!(execution.parameters().len(), 2);
    assert_eq!(execution.slot("other"), Some(1));
}
