// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Headline acceptance (Task 9 of the succinct-pack-codec feature): a SPARQL query
//! served DIRECTLY by the native evaluator over a [`PackView`] — the read-only
//! succinct pack codec's [`DatasetView`] implementation, opened from
//! [`PackBuilder::build_bytes`]'s output — returns the IDENTICAL [`SparqlResult`] as
//! the same prepared query evaluated over the production [`RdfDataset`] it was built
//! from, through the engine's public generic entry point
//! ([`NativeSparqlEngine::query_prepared_view`]).
//!
//! The shared fixture deliberately exercises every seam the pack codec's unified,
//! single-id-space dictionary claims to unify: a term used as both predicate AND
//! subject (`ex:knows`), a two-hop join chain, `>= 2` named graphs plus the default
//! graph, every literal shape (simple, typed, language-tagged, directional), a
//! scoped blank node, an RDF 1.2 triple term used as an asserted object, and a
//! reifier + annotation (including a graph-scoped annotation). Six queries — a join,
//! an unbound-subject probe, a predicate-constant-that-is-also-a-subject probe, a
//! named-graph query, an RDF 1.2 reifier query, and an absent-constant query — are
//! each evaluated over BOTH the source `RdfDataset` and the `PackView`, and the
//! resulting `SparqlResult::Solutions` are asserted equal (variables equal, and rows
//! equal after sorting, since SPARQL `SELECT` without `ORDER BY` yields a bag).

use std::sync::Arc;

use purrdf_core::{
    BlankScope, DatasetView, PackBuilder, PackView, RdfDataset, RdfDatasetBuilder, RdfLiteral,
    RdfTextDirection, SparqlResult, TermValue,
};
use purrdf_sparql_eval::NativeSparqlEngine;

// ── shared fixture ───────────────────────────────────────────────────────────────

/// An `example.org` IRI value.
fn iri(name: &str) -> TermValue {
    TermValue::iri(format!("http://example.org/{name}"))
}

/// Build the rich fixture `RdfDataset`: default graph (with a two-hop `knows` join
/// chain, a term used as both predicate and subject, every literal shape, a scoped
/// blank node, and an RDF 1.2 triple term asserted as an object) + 2 named graphs +
/// a reifier with a graph-scoped annotation.
fn build_fixture() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();

    let alice = b.intern_iri("http://example.org/alice");
    let bob = b.intern_iri("http://example.org/bob");
    let carol = b.intern_iri("http://example.org/carol");
    let knows = b.intern_iri("http://example.org/knows");
    let age = b.intern_iri("http://example.org/age");
    let name = b.intern_iri("http://example.org/name");
    let sees = b.intern_iri("http://example.org/sees");
    let likes = b.intern_iri("http://example.org/likes");
    let greeting = b.intern_iri("http://example.org/greeting");
    let confidence = b.intern_iri("http://example.org/confidence");
    let high = b.intern_iri("http://example.org/high");
    let source = b.intern_iri("http://example.org/source");
    let doc = b.intern_iri("http://example.org/doc");
    let meta = b.intern_iri("http://example.org/meta");
    let states_fact = b.intern_iri("http://example.org/statesFact");
    let reifier = b.intern_iri("http://example.org/r");
    let graph1 = b.intern_iri("http://example.org/graph1");
    let graph2 = b.intern_iri("http://example.org/graph2");

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
    let anon_name = b.intern_literal(RdfLiteral {
        lexical_form: "Anon".to_string(),
        datatype: None,
        language: None,
        direction: None,
    });
    let knows_label = b.intern_literal(RdfLiteral {
        lexical_form: "the knows predicate".to_string(),
        datatype: None,
        language: None,
        direction: None,
    });
    let hello_ltr = b.intern_literal(RdfLiteral {
        lexical_form: "Hello".to_string(),
        datatype: None,
        language: Some("en".to_string()),
        direction: Some(RdfTextDirection::Ltr),
    });
    let carol_age = b.intern_literal(RdfLiteral {
        lexical_form: "30".to_string(),
        datatype: Some("http://www.w3.org/2001/XMLSchema#integer".to_string()),
        language: None,
        direction: None,
    });

    // The RDF 1.2 triple term `<<( alice knows bob )>>`, asserted as the OBJECT of a
    // plain quad below AND reified further down — occupies both roles.
    let alice_knows_bob = b.intern_triple(alice, knows, bob);

    // -- Default graph ----------------------------------------------------------
    // A two-hop `knows` chain (alice -> bob -> carol) so a join query genuinely
    // unifies the shared variable `?y` across two triple patterns.
    b.push_quad(alice, knows, bob, None);
    b.push_quad(bob, knows, carol, None);
    b.push_quad(alice, age, forty_two, None);
    b.push_quad(alice, name, alice_name_en, None);
    // `knows` used as a SUBJECT here, and as a PREDICATE above — the unified-id
    // seam the pack dictionary's single-id-space model exists to prove.
    b.push_quad(knows, name, knows_label, None);
    b.push_quad(blank, name, anon_name, None);
    b.push_quad(bob, sees, blank, None);
    b.push_quad(alice, greeting, hello_ltr, None);
    b.push_quad(meta, states_fact, alice_knows_bob, None);

    // -- Named graph 1 ------------------------------------------------------------
    b.push_quad(carol, age, carol_age, Some(graph1));
    b.push_quad(bob, knows, carol, Some(graph1));

    // -- Named graph 2 ------------------------------------------------------------
    b.push_quad(carol, likes, alice, Some(graph2));

    // -- Reifier + annotation ------------------------------------------------------
    b.push_reifier(reifier, alice_knows_bob);
    b.push_annotation(reifier, confidence, high);
    // A second, graph-scoped annotation on the SAME reifier.
    b.push_annotation_in_graph(reifier, source, doc, Some(graph1));

    b.freeze().expect("fixture dataset must validate")
}

/// Build the pack bytes for `dataset` and open a `PackView` over them.
fn build_pack_bytes(dataset: &RdfDataset) -> Vec<u8> {
    PackBuilder::build_bytes(dataset).expect("pack build must succeed for a well-formed fixture")
}

// ── engine harness ───────────────────────────────────────────────────────────────

/// Prepare and evaluate `query` over any [`DatasetView`] backend through the
/// engine's public generic entry point — the exact seam the SPARQL evaluator uses
/// for a `PackView` (id type [`purrdf_core::PackId`]) as well as for the production
/// `RdfDataset` (id type [`purrdf_core::TermId`]).
fn run<D: DatasetView + Sync>(dataset: &D, query: &str) -> SparqlResult {
    let engine = NativeSparqlEngine::new();
    let prepared = engine.prepare_query(query, None).expect("prepare");
    engine
        .query_prepared_view(dataset, &prepared, &[])
        .expect("query")
}

/// Destructure a `SparqlResult` into `(variables, rows)`, panicking on any other
/// shape.
fn solutions(result: SparqlResult) -> (Vec<String>, Vec<Vec<Option<TermValue>>>) {
    match result {
        SparqlResult::Solutions {
            variables, rows, ..
        } => (variables, rows),
        other => panic!("expected solutions, got {other:?}"),
    }
}

/// A deterministic sort key for a solution row (`TermValue` is not `Ord`; its
/// `Debug` form is total and dataset-independent).
fn row_key(row: &[Option<TermValue>]) -> String {
    format!("{row:?}")
}

/// Run `query` over the source `RdfDataset` and over a `PackView` built from it, and
/// assert the two `SparqlResult::Solutions` are equal: same projected variables, and
/// same rows once both are sorted (a `SELECT` without `ORDER BY` is a bag). `min_rows`
/// guards against a vacuous pass (empty == empty).
fn assert_pack_matches_source(
    query: &str,
    min_rows: usize,
) -> (Vec<String>, Vec<Vec<Option<TermValue>>>) {
    let single = build_fixture();
    let bytes = build_pack_bytes(&single);
    let pack = PackView::from_bytes(&bytes).expect("pack opens");

    let (single_vars, mut single_rows) = solutions(run(&*single, query));
    let (pack_vars, mut pack_rows) = solutions(run(&pack, query));

    assert!(
        single_rows.len() >= min_rows,
        "non-vacuous: expected at least {min_rows} rows, got {} for query: {query}",
        single_rows.len()
    );
    assert_eq!(
        single_vars, pack_vars,
        "same projected variables for query: {query}"
    );

    single_rows.sort_by_key(|r| row_key(r));
    pack_rows.sort_by_key(|r| row_key(r));
    assert_eq!(
        single_rows, pack_rows,
        "source and PackView results diverge for query: {query}"
    );

    (single_vars, single_rows)
}

// ── Query 1 — BGP join across a shared variable ──────────────────────────────────

const JOIN_QUERY: &str = "\
PREFIX ex: <http://example.org/>
SELECT ?x ?y ?z WHERE {
  ?x ex:knows ?y .
  ?y ex:knows ?z .
}";

#[test]
fn join_query_parity() {
    let (vars, rows) = assert_pack_matches_source(JOIN_QUERY, 1);
    assert_eq!(vars, vec!["x", "y", "z"]);
    assert_eq!(rows.len(), 1, "only alice->bob->carol chains in two hops");
    assert_eq!(
        rows[0],
        vec![Some(iri("alice")), Some(iri("bob")), Some(iri("carol"))]
    );
}

// ── Query 2 — unbound subject (object/predicate index path) ─────────────────────

const UNBOUND_SUBJECT_QUERY: &str = "\
PREFIX ex: <http://example.org/>
SELECT ?s WHERE { ?s ex:knows ex:bob }";

#[test]
fn unbound_subject_query_parity() {
    let (vars, rows) = assert_pack_matches_source(UNBOUND_SUBJECT_QUERY, 1);
    assert_eq!(vars, vec!["s"]);
    assert_eq!(rows, vec![vec![Some(iri("alice"))]]);
}

// ── Query 3 — predicate constant that is also used as a subject elsewhere ───────

const PREDICATE_AS_SUBJECT_QUERY: &str = "\
PREFIX ex: <http://example.org/>
SELECT ?o WHERE { ex:knows ex:name ?o }";

#[test]
fn predicate_constant_also_a_subject_query_parity() {
    let (vars, rows) = assert_pack_matches_source(PREDICATE_AS_SUBJECT_QUERY, 1);
    assert_eq!(vars, vec!["o"]);
    assert_eq!(
        rows,
        vec![vec![Some(TermValue::simple_literal("the knows predicate"))]]
    );
}

// ── Query 4 — named-graph query (graph partitioning / GraphMatch) ───────────────

const NAMED_GRAPH_QUERY: &str = "\
PREFIX ex: <http://example.org/>
SELECT ?s ?p ?o WHERE { GRAPH ex:graph1 { ?s ?p ?o } }";

#[test]
fn named_graph_query_parity() {
    let (vars, rows) = assert_pack_matches_source(NAMED_GRAPH_QUERY, 3);
    assert_eq!(vars, vec!["s", "p", "o"]);
    // The `carol age 30` / `bob knows carol` plain quads, plus the graph-scoped
    // `r source doc` annotation row (the reification layer folds into ordinary BGP
    // matching within its own graph — see `crates/sparql-eval/src/bgp.rs`).
    assert_eq!(rows.len(), 3, "graph1 carries exactly three quads");
}

// ── Query 5 — RDF 1.2 content: the reifier's virtual `rdf:reifies` edge ─────────
//
// The evaluator folds RDF 1.2 reification into ordinary BGP matching via a virtual
// `rdf:reifies` predicate whose object is a triple-term pattern `<<( ?s ?p ?o )>>`
// (see `crates/sparql-eval/src/bgp.rs`'s `reified_graph` test fixtures). This is the
// queryable surface the evaluator actually exposes for reifier content, so it is
// what this parity guard exercises.

const REIFIES_QUERY: &str = "\
PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
PREFIX ex: <http://example.org/>
SELECT ?s ?p ?o WHERE { ex:r rdf:reifies <<( ?s ?p ?o )>> . }";

#[test]
fn rdf12_reifies_triple_term_query_parity() {
    let (vars, rows) = assert_pack_matches_source(REIFIES_QUERY, 1);
    assert_eq!(vars, vec!["s", "p", "o"]);
    assert_eq!(
        rows,
        vec![vec![
            Some(iri("alice")),
            Some(iri("knows")),
            Some(iri("bob"))
        ]]
    );
}

// ── Query 5b — RDF 1.2 content: default-scoped AND graph-scoped annotations ─────
//
// `ex:r ex:confidence ex:high` is a default-graph annotation; `ex:r ex:source
// ex:doc` is scoped to `ex:graph1`. A `UNION` of the two scopes proves both
// annotation rows of the same reifier resolve identically through the pack.

const ANNOTATION_QUERY: &str = "\
PREFIX ex: <http://example.org/>
SELECT ?p ?o WHERE {
  { ex:r ?p ?o } UNION { GRAPH ex:graph1 { ex:r ?p ?o } }
  FILTER(?p = ex:confidence || ?p = ex:source)
}";

#[test]
fn rdf12_annotation_query_parity() {
    let (vars, rows) = assert_pack_matches_source(ANNOTATION_QUERY, 2);
    assert_eq!(vars, vec!["p", "o"]);
    assert_eq!(rows.len(), 2, "both the plain and graph-scoped annotation");
}

// ── Query 6 — absent constant proves absence -> empty, not error ────────────────

const ABSENT_CONSTANT_QUERY: &str = "\
PREFIX ex: <http://example.org/>
SELECT ?o WHERE { ex:alice ex:doesNotExist ?o }";

#[test]
fn absent_constant_query_yields_empty_not_error() {
    let (vars, rows) = assert_pack_matches_source(ABSENT_CONSTANT_QUERY, 0);
    assert_eq!(vars, vec!["o"]);
    assert!(
        rows.is_empty(),
        "an absent predicate constant must match nothing, not error"
    );
}
