// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Structural AST goldens (purrdf S5 /).
//!
//! The corpus suite asserts only that queries parse (`Ok`); it does not pin the
//! *shape* of the produced algebra, so a refactor could silently change what a
//! query lowers to while every test stays green (exactly how the aggregate /
//! ORDER BY gaps hid). These `insta` snapshots over the `{:#?}` of the parsed
//! `Query` lock the algebra shape for representative in-scope features. A
//! `proptest` additionally pins the no-panic contract on arbitrary input.

use proptest::prelude::*;
use purrdf_sparql_algebra::SparqlParser;

const PREFIXES: &str = "PREFIX purrdf: <https://x/>\n\
     PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>\n\
     PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>\n";

fn parse(body: &str) -> impl std::fmt::Debug {
    SparqlParser::new()
        .parse_query(&format!("{PREFIXES}{body}"))
        .expect("snapshot fixture must parse")
}

fn parse_update(body: &str) -> impl std::fmt::Debug {
    SparqlParser::new()
        .parse_update(&format!("{PREFIXES}{body}"))
        .expect("snapshot update fixture must parse")
}

#[test]
fn snapshot_quoted_triple_paren() {
    // RDF 1.2 quoted-triple term → TermPattern::Triple (codec shape).
    insta::assert_debug_snapshot!(parse("SELECT ?r WHERE { ?r rdf:reifies <<( ?s ?p ?o )>> }"));
}

#[test]
fn snapshot_quoted_triple_bare() {
    // A bare `<< s p o >>` is a *reifying triple* (RDF 1.2), NOT a triple term: it
    // mints a fresh reifier `_:b`, emits `_:b rdf:reifies <<( s p o )>>`, and the
    // reifier stands in object position. This is distinct from the paren triple-term
    // form `<<( s p o )>>` (a value), which lowers to a single triple.
    insta::assert_debug_snapshot!(parse(
        "SELECT ?r WHERE { ?r rdf:reifies << ?s purrdf:p ?o >> }"
    ));
}

#[test]
fn snapshot_aggregate_group_by() {
    // COUNT lifts into Group; the projection references the synthetic agg var.
    insta::assert_debug_snapshot!(parse(
        "SELECT ?t (COUNT(?x) AS ?c) WHERE { ?x a ?t } GROUP BY ?t"
    ));
}

#[test]
fn snapshot_property_path() {
    // `/` + `*` property path → Path with a Sequence/ZeroOrMore expression.
    insta::assert_debug_snapshot!(parse(
        "SELECT ?x WHERE { ?d purrdf:members/rdf:rest*/rdf:first ?x }"
    ));
}

#[test]
fn snapshot_optional_union_bind() {
    // OPTIONAL → LeftJoin, UNION → Union, BIND → Extend in one query.
    insta::assert_debug_snapshot!(parse(
        "SELECT ?k WHERE { { ?a a purrdf:X } UNION { ?a a purrdf:Y } OPTIONAL { ?a purrdf:p ?b } BIND(\"x\" AS ?k) }"
    ));
}

#[test]
fn snapshot_update_insert_data() {
    // INSERT DATA lowers to ground quads (one default-graph, one GRAPH-scoped).
    insta::assert_debug_snapshot!(parse_update(
        "INSERT DATA { purrdf:s purrdf:p purrdf:o . GRAPH purrdf:g { purrdf:s purrdf:p purrdf:o2 } }"
    ));
}

#[test]
fn snapshot_update_delete_insert_modify() {
    // DELETE/INSERT modify: templates + the shared WHERE pattern.
    insta::assert_debug_snapshot!(parse_update(
        "DELETE { ?s purrdf:p ?o } INSERT { ?s purrdf:q ?o } WHERE { ?s purrdf:p ?o }"
    ));
}

proptest! {
    // The parser must never panic on arbitrary input — it returns Ok or a typed
    // ParseError. Restrict to a SPARQL-ish alphabet so the lexer is exercised
    // deeply rather than rejecting on the first non-ASCII byte.
    #[test]
    fn parse_never_panics(s in "[a-zA-Z0-9 ?<>{}().*+/^!|:_\"@-]{0,80}") {
        let _ = SparqlParser::new().parse_query(&s);
    }
}
