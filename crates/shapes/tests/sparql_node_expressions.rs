// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
    let shapes = parse_shapes(&format!("{PREFIXES}{shapes_ttl}"), None).expect("shapes parse");
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
        parse_turtle_to_dataset(&format!("{PREFIXES}{data_ttl}"), None).expect("data parse");
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
    parse_shapes(&format!("{PREFIXES}{shapes_ttl}"), None)
        .expect_err("the fixture must be refused at shapes-load")
        .to_string()
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
    let report =
        purrdf_shapes::engine::validate_graphs(data, &shapes, None).expect("validation runs");
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
    let data: Arc<_> = parse_turtle_to_dataset(PREFIXES, None).expect("data parse");
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

// ── The list-free one-argument call form ────────────────────────────────────────

/// SHACL 1.2 Node Expressions gives a function call's object THREE forms, and the
/// third is the list-free abbreviation: "If the function call has one argument,
/// and the argument is a well-formed node expression, then the argument can be
/// given without the list … Example: `[ sparql:abs -42 ]`, which is equivalent to
/// `[ sparql:abs ( -42 ) ]`."
///
/// This asserts the EQUIVALENCE the spec states, not merely that both parse: the
/// abbreviated and the list form must produce the identical output. Refusing the
/// abbreviation would reject a document copied verbatim out of the specification.
#[test]
fn a_one_argument_call_may_omit_the_argument_list() {
    let abbreviated = "ex:S a sh:NodeShape ; sh:expression [ sparql:abs -42 ] .";
    let listed = "ex:S a sh:NodeShape ; sh:expression [ sparql:abs ( -42 ) ] .";
    let expected = vec![r#""42"^^<http://www.w3.org/2001/XMLSchema#integer>"#.to_owned()];
    assert_eq!(
        outputs("ex:a ex:p ex:b .", abbreviated, "a"),
        expected,
        "the spec's own `[ sparql:abs -42 ]` example must evaluate"
    );
    assert_eq!(
        outputs("ex:a ex:p ex:b .", listed, "a"),
        expected,
        "the list form must agree with the abbreviation"
    );
}

/// The abbreviation carries a STRUCTURED node expression too, not only a literal:
/// the spec's condition is "the argument is a well-formed node expression", and a
/// path expression is one.
#[test]
fn the_list_free_form_accepts_a_structured_argument() {
    let shapes = "ex:S a sh:NodeShape ; sh:expression [ sparql:strlen [ sh:path ex:name ] ] .";
    assert_eq!(
        outputs(r#"ex:a ex:name "abcd" ."#, shapes, "a"),
        vec![r#""4"^^<http://www.w3.org/2001/XMLSchema#integer>"#.to_owned()],
        "a one-argument call over a path expression must work without the list"
    );
}

/// The zero-argument and two-argument list forms are untouched by the
/// abbreviation — `rdf:nil` is still the empty argument list, and a genuine list
/// is still read as one rather than as a single blank-node argument.
#[test]
fn the_empty_and_multi_argument_list_forms_still_parse() {
    let two = "ex:S a sh:NodeShape ; sh:expression [ sparql:add ( 38 4 ) ] .";
    assert_eq!(
        outputs("ex:a ex:p ex:b .", two, "a"),
        vec![r#""42"^^<http://www.w3.org/2001/XMLSchema#integer>"#.to_owned()],
        "a two-argument list must not be read as one blank-node argument"
    );
    // `rdf:nil` — the empty argument list. `sparql:concat` of nothing is the empty
    // string, which proves the list was read as EMPTY rather than as the single
    // argument `rdf:nil`.
    let none = "ex:S a sh:NodeShape ; sh:expression [ sparql:concat () ] .";
    assert_eq!(
        outputs("ex:a ex:p ex:b .", none, "a"),
        vec![r#""""#.to_owned()],
        "rdf:nil must stay the EMPTY argument list, not a one-element one"
    );
}

/// A SHACL annotation authored alongside a function call is an annotation, not a
/// second candidate function.
///
/// `parse_constraints` reads `sh:message` and `sh:severity` off the expression
/// node itself, so authoring them there is routine — and the STRUCTURAL
/// expressions have always accepted them, because that path short-circuits before
/// the function-candidate scan. A call site refusing the very same annotation as
/// "ambiguous" was an asymmetry, not a rule.
#[test]
fn a_call_site_may_carry_the_annotations_an_expression_constraint_reads() {
    let shapes = r#"ex:S a sh:NodeShape ;
        sh:expression [ sparql:strlen [ sh:path ex:name ] ;
                        sh:message "the name length" ;
                        sh:severity sh:Warning ] ."#;
    assert_eq!(
        outputs(r#"ex:a ex:name "abcd" ."#, shapes, "a"),
        vec![r#""4"^^<http://www.w3.org/2001/XMLSchema#integer>"#.to_owned()],
        "sh:message / sh:severity must not be mistaken for a function predicate"
    );
}

/// The neighbouring INVALID case: two genuine function predicates on one node are
/// still ambiguous. Ignoring the annotations narrowed the candidate set; it did
/// not remove the check.
#[test]
fn two_real_function_predicates_on_one_node_are_still_ambiguous() {
    let err = load_error(
        "ex:S a sh:NodeShape ;
             sh:expression [ sparql:strlen ( \"a\" ) ; sparql:abs ( -1 ) ] .",
    );
    assert!(
        err.contains("ambiguous function-call node expression"),
        "got: {err}"
    );
}

/// SHACL-AF lets `sh:prefixes` sit on the SHAPE as well as on the constraint node,
/// and `sh:sparql` honours both. A `sh:select` node expression must too.
///
/// It did not: the header was built from the expression node alone, so the very
/// same declaration that works for `sh:sparql` came back as
/// "has an unparsable query" — a legal document refused, and reported as a syntax
/// error in the author's SPARQL rather than as the unresolved prefix it was. The
/// sibling test above pins the expression-node spelling; this pins the shape one,
/// and `sh:sparqlExpr` gets the same treatment because it shares the arm.
#[test]
fn a_select_expression_resolves_sh_prefixes_declared_on_the_shape() {
    let declaration = r#"<http://example.org/decl> sh:declare
             [ sh:prefix "p" ; sh:namespace "http://example.org/ns#"^^xsd:anyURI ] ."#;
    assert_eq!(
        outputs(
            "ex:a ex:child ex:b .",
            &format!(
                r#"{declaration}
                 ex:S a sh:NodeShape ;
                     sh:prefixes <http://example.org/decl> ;
                     sh:expression [ sh:select "SELECT ?child WHERE {{ $this p:child ?child }}" ] ."#
            ),
            "a",
        ),
        vec![ex("b")],
        "sh:prefixes on the shape must reach a sh:select node expression"
    );
    assert_eq!(
        outputs(
            "ex:a ex:child ex:b .",
            &format!(
                r#"{declaration}
                 ex:S a sh:NodeShape ;
                     sh:prefixes <http://example.org/decl> ;
                     sh:expression [ sh:sparqlExpr "STRLEN(STR(p:child))" ] ."#
            ),
            "a",
        ),
        // `STRLEN` of the expanded IRI: `http://example.org/ns#child` is 27
        // characters, so this also proves the prefix EXPANDED rather than merely
        // parsed.
        vec![r#""27"^^<http://www.w3.org/2001/XMLSchema#integer>"#.to_owned()],
        "sh:prefixes on the shape must reach a sh:sparqlExpr node expression"
    );
}

/// The NEIGHBOURING INVALID case: an UNDECLARED prefix is still a load error.
/// Widening the header's owners added a second place to look, not a fallback that
/// invents namespaces.
#[test]
fn an_undeclared_prefix_in_a_select_expression_is_still_a_load_error() {
    let err = load_error(
        r#"ex:S a sh:NodeShape ;
             sh:expression [ sh:select "SELECT ?child WHERE { $this nosuch:child ?child }" ] ."#,
    );
    assert!(err.contains("unparsable query"), "got: {err}");
}

// ── Built-in declarations bind natively ─────────────────────────────────────────

/// A shapes graph that merges the W3C SHACL 1.2 vocabulary carries, among much
/// else, this declaration of the built-in `sh:SPARQLExprExpression` — quoted here
/// verbatim, as a user met it after resolving `owl:imports` into one stand-alone
/// shapes graph. It has no `sh:bodyExpression`, because the engine implements it.
const SPARQL_EXPR_DECLARATION: &str = r#"
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

sh:SPARQLExprExpression a sh:NamedParameterExpressionFunction ;
  rdfs:label "SPARQL expr expression"@en ;
  rdfs:comment "The class of node expressions based on SPARQL expressions (sh:sparqlExpr)."@en ;
  rdfs:isDefinedBy sh: ;
  rdfs:subClassOf sh:NamedParameterExpression,
  sh:SPARQLExecutable ;
  sh:parameter sh:SPARQLExprExpression-prefixes,
  sh:SPARQLExprExpression-sparqlExpr .

sh:SPARQLExprExpression-prefixes a sh:Parameter ;
  rdfs:isDefinedBy sh: ;
  sh:description "The prefixes that shall be applied before parsing the SPARQL query that gets derived from the sh:sparqlExpr expression. The object should define those prefixes using sh:declare."@en ;
  sh:name "prefixes"@en ;
  sh:nodeKind sh:BlankNodeOrIRI ;
  sh:path sh:prefixes .

sh:SPARQLExprExpression-sparqlExpr a sh:Parameter ;
  rdfs:isDefinedBy sh: ;
  sh:datatype xsd:string ;
  sh:description "The SPARQL expression that is executed during evaluation of this node expression."@en ;
  sh:keyParameter true ;
  sh:name "SPARQL expr"@en ;
  sh:path sh:sparqlExpr .
"#;

/// A node shape whose `sh:sparqlExpr` holds only for focus IRIs longer than 30
/// characters.
const LONG_IRI_SHAPE: &str = r#"
@prefix ex: <http://example.org/> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
ex:S a sh:NodeShape ;
  sh:targetClass ex:C ;
  sh:expression [ sh:sparqlExpr "STRLEN(STR($this)) > 30" ] .
"#;

/// Two `ex:C` instances: one with an IRI of 54 characters, one of 20.
const TWO_INSTANCES: &str = r"
@prefix ex: <http://example.org/> .
<http://example.org/a-very-long-focus-node-iri-xxxxxxxx> a ex:C .
<http://example.org/s> a ex:C .
";

/// Validate through `validate_dataset_with_shapes_graph`, the entry point the
/// report was filed against.
fn validate_graphs(
    shapes_ttl: &str,
    data_ttl: &str,
) -> Result<purrdf_shapes::report::ValidationReport, String> {
    let shapes = parse_shapes(shapes_ttl, None)?;
    let data = parse_turtle_to_dataset(data_ttl, None).expect("data parse");
    purrdf_shapes::engine::validate_dataset_with_shapes_graph(&data, &shapes, None)
}

/// The built-in's declaration loads, and `sh:sparqlExpr` still evaluates
/// NATIVELY: exactly the short IRI violates. The control — the same shapes graph
/// without the declaration — produces a byte-identical report, so the declaration
/// changed nothing about what the expression means.
#[test]
fn the_builtin_sparql_expr_declaration_loads_and_evaluates_natively() {
    let treatment = validate_graphs(
        &format!("{SPARQL_EXPR_DECLARATION}{LONG_IRI_SHAPE}"),
        TWO_INSTANCES,
    )
    .expect("the shapes graph carrying the built-in's declaration loads and validates");
    assert_eq!(treatment.results.len(), 1, "exactly the short IRI fails");
    assert_eq!(
        treatment.results[0].focus_node.to_string(),
        "<http://example.org/s>"
    );
    let control = validate_graphs(LONG_IRI_SHAPE, TWO_INSTANCES).expect("control validates");
    assert_eq!(
        treatment.to_ntriples(),
        control.to_ntriples(),
        "the declaration must not change the report by a single byte"
    );
}

/// `sh:prefixes` is observed through the declaration: the expression names `ex:s`
/// through a `sh:declare`, and the answer depends on which namespace it declares.
/// The treatment row (ex: → `http://example.org/`) singles out `<http://example.org/s>`;
/// the control row (ex: → another namespace) singles out nothing. Both carry the
/// built-in's declaration.
#[test]
fn sh_prefixes_is_honoured_beside_the_builtin_declaration() {
    let shapes = |namespace: &str| {
        format!(
            r#"{SPARQL_EXPR_DECLARATION}
            @prefix ex: <http://example.org/> .
            ex:Decls sh:declare [ sh:prefix "ex" ; sh:namespace "{namespace}"^^xsd:anyURI ] .
            ex:S a sh:NodeShape ;
              sh:targetClass ex:C ;
              sh:expression [ sh:sparqlExpr "$this != ex:s" ; sh:prefixes ex:Decls ] ."#
        )
    };
    let treatment = validate_graphs(&shapes("http://example.org/"), TWO_INSTANCES)
        .expect("treatment validates");
    let control = validate_graphs(&shapes("http://example.org/elsewhere/"), TWO_INSTANCES)
        .expect("control validates");
    let focus = |report: &purrdf_shapes::report::ValidationReport| -> Vec<String> {
        report
            .results
            .iter()
            .map(|r| r.focus_node.to_string())
            .collect()
    };
    assert_eq!(focus(&treatment), vec!["<http://example.org/s>".to_owned()]);
    assert!(
        focus(&control).is_empty(),
        "under another namespace ex:s names neither instance"
    );
}

// ── The shnex-sparql.ttl spellings ─────────────────────────────────────────────

/// `shnex-sparql.ttl` spells the SPARQL `+` operator `sparql:plus`; `sparql-ns.ttl`
/// spells it `sparql:add`. Both answer 42.
#[test]
fn sparql_plus_is_the_add_operator() {
    let plus = outputs(
        "",
        r"ex:S a sh:NodeShape ; sh:expression [ sparql:plus ( 38 4 ) ] .",
        "a",
    );
    let add = outputs(
        "",
        r"ex:S a sh:NodeShape ; sh:expression [ sparql:add ( 38 4 ) ] .",
        "a",
    );
    assert_eq!(plus, vec![int("42")]);
    assert_eq!(plus, add);
}

/// `shnex-sparql.ttl` spells `ENCODE_FOR_URI` `sparql:encode`; `sparql-ns.ttl`
/// spells it `sparql:encodeForUri`.
#[test]
fn sparql_encode_is_encode_for_uri() {
    let out = outputs(
        "",
        r#"ex:S a sh:NodeShape ; sh:expression [ sparql:encode ( "a b" ) ] ."#,
        "a",
    );
    assert_eq!(out, vec!["\"a%20b\"".to_owned()]);
}

/// The aliases are two NAMES, not a namespace wildcard: a name neither vocabulary
/// defines is still refused.
#[test]
fn the_spelling_aliases_do_not_admit_unknown_names() {
    let err = load_error(r"ex:S a sh:NodeShape ; sh:expression [ sparql:notAFunction ( 1 ) ] .");
    assert!(
        err.contains("is not a callable SPARQL 1.2 function name"),
        "got: {err}"
    );
}

/// A prepared product keeps the spelling the author wrote: encoded and restored,
/// the call is still `sparql:plus`, and it still answers 42.
#[test]
fn a_product_restore_keeps_the_authored_spelling() {
    use purrdf_shapes::engine::PreparedShapes;
    use purrdf_shapes::expression::FnCall;
    use purrdf_shapes::product::{HostBindings, ShapesProduct, ShapesProfile};

    let shapes = parse_shapes(
        &format!(
            "{PREFIXES}ex:S a sh:NodeShape ; sh:targetNode ex:a ;
               sh:expression [ sparql:equals ( [ sparql:plus ( 38 4 ) ] 42 ) ] ."
        ),
        None,
    )
    .expect("shapes parse");
    let bytes = PreparedShapes::new(Arc::new(shapes))
        .to_product(&ShapesProfile::CORE)
        .expect("the shapes graph packs");
    let restored = ShapesProduct::open(&bytes)
        .expect("the product opens")
        .rebuild(&ShapesProfile::CORE, &HostBindings::empty())
        .expect("the product rebuilds");
    let Some(Constraint::Expression { expr, .. }) =
        restored.shapes().node_shapes[0].constraints.first()
    else {
        panic!("the restored shape keeps its expression constraint");
    };
    let NodeExpr::Call(FnCall::Sparql { args, .. }) = expr else {
        panic!("the restored expression is a sparql: call");
    };
    let NodeExpr::Call(FnCall::Sparql {
        iri,
        expr: rendered,
        ..
    }) = &args[0]
    else {
        panic!("the inner call is a sparql: call");
    };
    assert_eq!(iri.as_str(), "http://www.w3.org/ns/sparql#plus");
    assert_eq!(rendered, "(?a0 + ?a1)");
}
