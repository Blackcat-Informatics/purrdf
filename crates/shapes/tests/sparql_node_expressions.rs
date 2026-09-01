// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Integration tests for the SPARQL-facing node expressions.
//!
//! Two W3C sections are covered, and every test drives the PRODUCTION surface
//! (`engine::parse_shapes` for the shapes graph, `expression::eval_node_expr` for
//! evaluation) and asserts the exact node list the section states:
//!
//! * **SHACL 1.2 SPARQL Extensions §6.1 / §6.2** — `sh:select` (function name
//!   `sh:SelectExpression`) and `sh:sparqlExpr` (function name
//!   `sh:SPARQLExprExpression`).
//! * **SHACL 1.2 Node Expressions §5** — the `sparql:<NAME>` call form over the
//!   W3C SPARQL 1.2 term vocabulary (`http://www.w3.org/ns/sparql#`).
//!
//! Test IRIs live under `example.org`; PurRDF mints no vocabulary of its own, and
//! every `sh:` / `sparql:` term used here is defined by the specification (or, for
//! the `sparql:` namespace, by the SPARQL Working Group's own `sparql-ns.ttl`,
//! which SHACL 1.2 Node Expressions §5 makes callable).

use std::sync::Arc;

use purrdf_shapes::data::ShaclData;
use purrdf_shapes::engine::parse_shapes;
use purrdf_shapes::expression::{NodeExpr, RecursionGuard, eval_node_expr};
use purrdf_shapes::shapes::Constraint;
use purrdf_shapes::term::{NamedNode, Term};
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;

const PREFIXES: &str = r"
@prefix ex:     <http://example.org/ns#> .
@prefix rdf:    <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix sh:     <http://www.w3.org/ns/shacl#> .
@prefix sparql: <http://www.w3.org/ns/sparql#> .
@prefix xsd:    <http://www.w3.org/2001/XMLSchema#> .
";

/// The single `sh:expression` node expression a fixture declares.
fn expression_of(shapes_ttl: &str) -> NodeExpr {
    let shapes = parse_shapes(&format!("{PREFIXES}{shapes_ttl}")).expect("shapes parse");
    let mut found: Vec<NodeExpr> = shapes
        .node_shapes
        .iter()
        .flat_map(|shape| &shape.constraints)
        .filter_map(|c| match c {
            Constraint::Expression { expr, .. } => Some(expr.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        found.len(),
        1,
        "the fixture must declare exactly one sh:expression"
    );
    found.remove(0)
}

/// Evaluate the fixture's `sh:expression` over `data_ttl` from `ex:<focus>`,
/// returning the output nodes in the order the evaluator produced them.
///
/// The SPARQL view is the DATA graph, which is what a `sh:select` node expression
/// is defined to query ("executed against the focus graph").
fn outputs(data_ttl: &str, shapes_ttl: &str, focus: &str) -> Vec<String> {
    let expr = expression_of(shapes_ttl);
    let data: Arc<_> =
        parse_turtle_to_dataset(&format!("{PREFIXES}{data_ttl}")).expect("data parse");
    let store = ShaclData::new(Arc::clone(&data), data, None);
    let focus_term = Term::NamedNode(NamedNode::new_unchecked(format!(
        "http://example.org/ns#{focus}"
    )));
    let mut guard = RecursionGuard::new();
    eval_node_expr(&store, &focus_term, &expr, &mut guard)
        .expect("node expression evaluates")
        .iter()
        .map(ToString::to_string)
        .collect()
}

/// The shapes-load error a malformed fixture produces.
fn load_error(shapes_ttl: &str) -> String {
    parse_shapes(&format!("{PREFIXES}{shapes_ttl}"))
        .expect_err("the fixture must be refused at shapes-load")
}

/// The canonical rendering of `ex:<local>`.
fn ex(local: &str) -> String {
    format!("<http://example.org/ns#{local}>")
}

/// The canonical rendering of an `xsd:integer` literal.
fn int(n: &str) -> String {
    format!("\"{n}\"^^<http://www.w3.org/2001/XMLSchema#integer>")
}

// ── SPARQL Extensions §6.1 Select expressions ───────────────────────────────────

/// §6.1: the output nodes are "bindings of the single projected SELECT variable
/// when executed against the focus graph with focusNode pre-bound to variable
/// `$this`".
#[test]
fn select_expression_binds_this_to_the_focus_node() {
    let out = outputs(
        "ex:a ex:child ex:b, ex:c . ex:d ex:child ex:zzz .",
        r#"ex:S a sh:NodeShape ; sh:expression [
             sh:select "SELECT ?child WHERE { $this ex:child ?child } ORDER BY ?child" ] ."#,
        "a",
    );
    assert_eq!(
        out,
        vec![ex("b"), ex("c")],
        "only the focus node's children, and not ex:d's"
    );
}

/// The query's own `ORDER BY` decides the output order, so a select expression is
/// SEQUENCE-valued: the answer is not re-sorted behind the author's back.
#[test]
fn select_expression_preserves_the_query_s_own_order() {
    let ascending = outputs(
        "ex:a ex:n 1, 2, 10 .",
        r#"ex:S a sh:NodeShape ; sh:expression [
             sh:select "SELECT ?n WHERE { $this ex:n ?n } ORDER BY ?n" ] ."#,
        "a",
    );
    assert_eq!(ascending, vec![int("1"), int("2"), int("10")]);

    let descending = outputs(
        "ex:a ex:n 1, 2, 10 .",
        r#"ex:S a sh:NodeShape ; sh:expression [
             sh:select "SELECT ?n WHERE { $this ex:n ?n } ORDER BY DESC(?n)" ] ."#,
        "a",
    );
    assert_eq!(
        descending,
        vec![int("10"), int("2"), int("1")],
        "the descending query must not be re-sorted into ascending order"
    );
}

/// §6.1 lets a select expression carry its own `sh:prefixes`, which the shapes
/// document's own `@prefix` map also supplies as a fallback. Both reach the query.
#[test]
fn select_expression_resolves_prefixed_names_through_sh_prefixes() {
    let out = outputs(
        "ex:a ex:child ex:b .",
        r#"
        <http://example.org/decl> sh:declare [ sh:prefix "p" ; sh:namespace "http://example.org/ns#"^^xsd:anyURI ] .
        ex:S a sh:NodeShape ; sh:expression [
            sh:prefixes <http://example.org/decl> ;
            sh:select "SELECT ?child WHERE { $this p:child ?child }" ] ."#,
        "a",
    );
    assert_eq!(out, vec![ex("b")]);
}

/// §6.1: the query "must be a valid SPARQL 1.2 SELECT query projecting exactly one
/// variable". A two-variable projection has no single answer column, so it is a
/// shapes-LOAD failure rather than a silent take-the-first-column.
#[test]
fn select_expression_projecting_two_variables_is_a_load_error() {
    let err = load_error(
        r#"ex:S a sh:NodeShape ; sh:expression [
             sh:select "SELECT ?a ?b WHERE { $this ex:child ?a . $this ex:child ?b }" ] ."#,
    );
    assert!(
        err.contains("exactly one variable"),
        "the refusal must name the projection rule: {err}"
    );
}

/// An unparsable `sh:select` is refused at shapes-load, not at the first focus node.
#[test]
fn select_expression_with_an_unparsable_query_is_a_load_error() {
    let err = load_error(
        r#"ex:S a sh:NodeShape ; sh:expression [ sh:select "SELECT ?x WHERE { $this" ] ."#,
    );
    assert!(
        err.contains("sh:select node expression") && err.contains("unparsable"),
        "got: {err}"
    );
}

/// A non-SELECT body is likewise refused at load.
#[test]
fn select_expression_that_is_an_ask_is_a_load_error() {
    let err =
        load_error(r#"ex:S a sh:NodeShape ; sh:expression [ sh:select "ASK { $this ?p ?o }" ] ."#);
    assert!(err.contains("must be a SELECT query"), "got: {err}");
}

// ── SPARQL Extensions §6.2 SPARQL expr expressions ──────────────────────────────

/// §6.2's own worked example: `sh:sparqlExpr "STRLEN(STR($this))"` computes the
/// length of the focus node's IRI. The section defines the expression to be
/// embedded into `SELECT ($EXPR$ AS ?result) WHERE {}`, so the answer is a single
/// node.
#[test]
fn sparql_expr_expression_computes_the_focus_node_uri_length() {
    let out = outputs(
        "ex:a ex:p ex:b .",
        r#"ex:S a sh:NodeShape ; sh:expression [ sh:sparqlExpr "STRLEN(STR($this))" ] ."#,
        "a",
    );
    // "http://example.org/ns#a" is 23 characters.
    assert_eq!(out, vec![int("23")]);
}

/// The `sh:sparqlExpr` spelling and its `sh:select` "equivalent expanded form"
/// (the specification's own words) produce the same node — one expression
/// language, not two.
#[test]
fn sparql_expr_and_its_expanded_select_agree() {
    let short = outputs(
        "ex:a ex:p ex:b .",
        r#"ex:S a sh:NodeShape ; sh:expression [ sh:sparqlExpr "STRLEN(STR($this))" ] ."#,
        "a",
    );
    let expanded = outputs(
        "ex:a ex:p ex:b .",
        r#"ex:S a sh:NodeShape ; sh:expression [
             sh:select "SELECT (STRLEN(STR($this)) AS ?result) WHERE { }" ] ."#,
        "a",
    );
    assert_eq!(short, expanded);
    assert_eq!(short, vec![int("23")]);
}

/// A `sh:sparqlExpr` that is not a SPARQL expression at all is refused at load,
/// and the refusal names the key the author actually wrote.
#[test]
fn sparql_expr_that_does_not_parse_is_a_load_error() {
    let err = load_error(r#"ex:S a sh:NodeShape ; sh:expression [ sh:sparqlExpr "STRLEN(" ] ."#);
    assert!(
        err.contains("sh:sparqlExpr node expression") && err.contains("unparsable"),
        "got: {err}"
    );
}

/// A node carrying BOTH SPARQL-based keys is ambiguous, exactly as a node carrying
/// two different node-expression kinds already is.
#[test]
fn a_node_with_both_select_and_sparql_expr_is_ambiguous() {
    let err = load_error(
        r#"ex:S a sh:NodeShape ; sh:expression [
             sh:select "SELECT ?x WHERE {}" ; sh:sparqlExpr "1" ] ."#,
    );
    assert!(err.contains("ambiguous node expression"), "got: {err}");
}

/// §6.1/§6.2 evaluate "with … scope variables pre-bound with matching names".
///
/// An expression constraint (Node Expressions §7.1) evaluates as
/// `evalExpr(expr, data graph, focusNode, {value: v})`, so a SPARQL-based node
/// expression inside one must see `?value` bound to the value node under test.
/// The fixture proves the binding is REAL and PER-VALUE: only the value that
/// fails the comparison is reported, which is impossible if `?value` were unbound
/// (an unbound `?value` makes the comparison an error for every value, so BOTH
/// would be reported).
#[test]
fn a_sparql_based_expression_sees_the_value_node_in_its_scope() {
    let data = concat!(
        "<http://example.org/ns#a> <http://example.org/ns#n> \
         \"1\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n",
        "<http://example.org/ns#a> <http://example.org/ns#n> \
         \"9\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n",
    );
    let shapes = format!(
        r#"{PREFIXES}
        ex:S a sh:NodeShape ;
            sh:targetNode ex:a ;
            sh:property [
                sh:path ex:n ;
                sh:expression [ sh:sparqlExpr "?value > 2" ] ;
            ] .
        "#
    );
    let report = purrdf_shapes::engine::validate_graphs(data, &shapes).expect("validation runs");
    let offenders: Vec<String> = report
        .results
        .iter()
        .filter_map(|r| r.value.as_ref().map(ToString::to_string))
        .collect();
    assert_eq!(
        offenders,
        vec![int("1")],
        "exactly the value that fails `?value > 2` is reported"
    );
}

// ── Node Expressions §5: the `sparql:<NAME>` call form ──────────────────────────

/// §5 with a plain SPARQL function name: `sparql:concat` is `CONCAT`.
#[test]
fn sparql_ns_call_resolves_a_plain_function_name() {
    let out = outputs(
        "",
        r#"ex:S a sh:NodeShape ; sh:expression [ sparql:concat ( "a" "b" "c" ) ] ."#,
        "a",
    );
    assert_eq!(out, vec!["\"abc\"".to_owned()]);
}

/// A one-argument function name, resolved through the same table.
#[test]
fn sparql_ns_call_resolves_strlen() {
    let out = outputs(
        "",
        r#"ex:S a sh:NodeShape ; sh:expression [ sparql:strlen ( "abcd" ) ] ."#,
        "a",
    );
    assert_eq!(out, vec![int("4")]);
}

/// `sparql:add` is the SPARQL `+` OPERATOR, not a keyword call — the dispatch
/// lowers it to infix form.
#[test]
fn sparql_ns_call_resolves_the_add_operator() {
    let out = outputs(
        "",
        r"ex:S a sh:NodeShape ; sh:expression [ sparql:add ( 38 4 ) ] .",
        "a",
    );
    assert_eq!(out, vec![int("42")]);
}

/// `sparql:unary-minus` is the unary `-` operator.
#[test]
fn sparql_ns_call_resolves_the_unary_minus_operator() {
    let out = outputs(
        "",
        r"ex:S a sh:NodeShape ; sh:expression [ sparql:unary-minus ( 7 ) ] .",
        "a",
    );
    assert_eq!(out, vec![int("-7")]);
}

/// `sparql:if` is a functional form spelled as a keyword call.
#[test]
fn sparql_ns_call_resolves_the_if_functional_form() {
    let out = outputs(
        "",
        r"ex:S a sh:NodeShape ; sh:expression [ sparql:if ( true ex:yes ex:no ) ] .",
        "a",
    );
    assert_eq!(out, vec![ex("yes")]);
}

/// `sparql:in` is the membership form `(a0 IN (a1, …))`, not a function call.
#[test]
fn sparql_ns_call_resolves_the_in_membership_form() {
    let hit = outputs(
        "",
        r"ex:S a sh:NodeShape ; sh:expression [ sparql:in ( 2 1 2 3 ) ] .",
        "a",
    );
    assert_eq!(
        hit,
        vec!["\"true\"^^<http://www.w3.org/2001/XMLSchema#boolean>".to_owned()]
    );

    let miss = outputs(
        "",
        r"ex:S a sh:NodeShape ; sh:expression [ sparql:in ( 9 1 2 3 ) ] .",
        "a",
    );
    assert_eq!(
        miss,
        vec!["\"false\"^^<http://www.w3.org/2001/XMLSchema#boolean>".to_owned()]
    );
}

/// `sparql:equals` is the `=` operator; `sparql:sameValue` and
/// `sparql:RDFterm-equal` name the same operator (the specification says
/// `sameValue` "cannot be used directly in a query" — `=` IS its call form), so
/// all three must agree.
#[test]
fn sparql_ns_equality_names_all_lower_to_the_same_operator() {
    for local in ["equals", "sameValue", "RDFterm-equal"] {
        let out = outputs(
            "",
            &format!(
                "ex:S a sh:NodeShape ; sh:expression [ <http://www.w3.org/ns/sparql#{local}> ( 3 3 ) ] ."
            ),
            "a",
        );
        assert_eq!(
            out,
            vec!["\"true\"^^<http://www.w3.org/2001/XMLSchema#boolean>".to_owned()],
            "sparql:{local} must answer true for 3 = 3"
        );
    }
}

/// **RDF 1.2, first class**: `sparql:triple` CONSTRUCTS a triple term, and the
/// produced node is an RDF 1.2 triple term — not an IRI, not a blank reifier.
#[test]
fn sparql_triple_constructs_an_rdf12_triple_term() {
    let expr = expression_of(
        r"ex:S a sh:NodeShape ; sh:expression [ sparql:triple ( ex:s ex:p ex:o ) ] .",
    );
    let data: Arc<_> = parse_turtle_to_dataset(PREFIXES).expect("data parse");
    let store = ShaclData::new(Arc::clone(&data), data, None);
    let focus = Term::NamedNode(NamedNode::new_unchecked("http://example.org/ns#a"));
    let mut guard = RecursionGuard::new();
    let out = eval_node_expr(&store, &focus, &expr, &mut guard).expect("sparql:triple evaluates");

    let [Term::Triple(triple)] = out.as_slice() else {
        panic!("sparql:triple must yield exactly one RDF 1.2 triple term, got {out:?}");
    };
    assert_eq!(triple.subject.to_string(), ex("s"));
    assert_eq!(triple.predicate.as_str(), "http://example.org/ns#p");
    assert_eq!(triple.object.to_string(), ex("o"));
}

/// The triple term `sparql:triple` builds is a first-class value the rest of the
/// language reads: `sparql:object` takes it apart again.
#[test]
fn sparql_object_reads_a_constructed_triple_term() {
    let out = outputs(
        "",
        r"ex:S a sh:NodeShape ; sh:expression [
             sparql:object ( [ sparql:triple ( ex:s ex:p ex:o ) ] ) ] .",
        "a",
    );
    assert_eq!(out, vec![ex("o")]);
}

/// `sparql:isTriple` recognises the constructed term as a triple term.
#[test]
fn sparql_is_triple_recognises_a_constructed_triple_term() {
    let out = outputs(
        "",
        r"ex:S a sh:NodeShape ; sh:expression [
             sparql:isTriple ( [ sparql:triple ( ex:s ex:p ex:o ) ] ) ] .",
        "a",
    );
    assert_eq!(
        out,
        vec!["\"true\"^^<http://www.w3.org/2001/XMLSchema#boolean>".to_owned()]
    );
}

/// `sparql:encodeForUri` is the one function-call name whose SPARQL keyword is not
/// its own uppercasing (`ENCODE_FOR_URI`), so it exercises the explicit half of
/// the dispatch table.
#[test]
fn sparql_ns_call_resolves_encode_for_uri() {
    let out = outputs(
        "",
        r#"ex:S a sh:NodeShape ; sh:expression [ sparql:encodeForUri ( "a b" ) ] ."#,
        "a",
    );
    assert_eq!(out, vec!["\"a%20b\"".to_owned()]);
}

/// An argument is itself a node expression, so a `sparql:` call composes with the
/// rest of the language — here `sh:this` feeds `sparql:str`.
#[test]
fn sparql_ns_call_arguments_are_node_expressions() {
    let out = outputs(
        "",
        r"ex:S a sh:NodeShape ; sh:expression [ sparql:strlen ( [ sparql:str ( sh:this ) ] ) ] .",
        "a",
    );
    assert_eq!(out, vec![int("23")]);
}

/// A SPARQL AGGREGATE is not a scalar function, so `sparql:agg-sum` is refused at
/// shapes-load and the refusal names the SHACL aggregates that ARE node
/// expressions. A refusal, never a silent empty answer.
#[test]
fn sparql_ns_aggregate_name_is_a_load_error() {
    let err = load_error(r"ex:S a sh:NodeShape ; sh:expression [ sparql:agg-sum ( 1 2 ) ] .");
    assert!(err.contains("SPARQL AGGREGATE"), "got: {err}");
    assert!(err.contains("shnex:sum"), "got: {err}");
}

/// `sparql:filter-exists` is a functional form over a GRAPH PATTERN, which a node
/// expression has no way to supply, so it is refused with a pointer at
/// `shnex:exists`.
#[test]
fn sparql_ns_exists_functional_form_is_a_load_error() {
    let err =
        load_error(r"ex:S a sh:NodeShape ; sh:expression [ sparql:filter-exists ( ex:a ) ] .");
    assert!(err.contains("GRAPH PATTERN"), "got: {err}");
    assert!(err.contains("shnex:exists"), "got: {err}");
}

/// A local name the SPARQL vocabulary does not define is refused at load rather
/// than evaluated into nothing.
#[test]
fn an_unknown_sparql_ns_name_is_a_load_error() {
    let err = load_error(r"ex:S a sh:NodeShape ; sh:expression [ sparql:notAFunction ( 1 ) ] .");
    assert!(
        err.contains("is not a callable SPARQL 1.2 function name"),
        "got: {err}"
    );
}

/// An operator applied to the wrong number of operands is an arity error at
/// shapes-load, named as such.
#[test]
fn a_sparql_ns_operator_with_the_wrong_arity_is_a_load_error() {
    let err = load_error(r"ex:S a sh:NodeShape ; sh:expression [ sparql:add ( 1 2 3 ) ] .");
    assert!(err.contains("exactly 2 arguments"), "got: {err}");
}
