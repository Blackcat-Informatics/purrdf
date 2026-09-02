// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Integration tests for CUSTOM node-expression functions and the SPARQL
//! functions declared from them.
//!
//! Three W3C sections, driven through the production surfaces only —
//! `engine::parse_shapes` for the shapes graph, `engine::validate_graphs` for
//! validation, `rules::entail_dataset` for rule entailment, and
//! `NativeSparqlEngine` for query text. Nothing here hand-builds a
//! `NodeExpr`.
//!
//! * **SHACL 1.2 Node Expressions §6.1** — `sh:NamedParameterExpressionFunction`,
//!   `sh:keyParameter`, and the `[ shnex:arg <iri> ]` argument reference.
//! * **SHACL 1.2 Node Expressions §6.2 / §6.3** —
//!   `sh:ListParameterExpressionFunction`, `sh:bodyExpression`, and the
//!   `[ shnex:arg 0 ]` argument reference.
//! * **SHACL 1.2 SPARQL Extensions §7 / §7.3** — the same list-parameter
//!   declarations registered as callable SPARQL functions, evaluated with the
//!   scope keyed by argument index and the function's own IRI as focus node.
//!
//! Test IRIs live under `example.org`; every `sh:` / `shnex:` / `sparql:` term
//! used here is defined by the specification named beside it.

use std::sync::Arc;

use purrdf::{RdfDataset, SparqlRequest, SparqlResult};
use purrdf_shapes::data::ShaclData;
use purrdf_shapes::engine::{parse_shapes, validate_with};
use purrdf_shapes::expression::{NodeExpr, RecursionGuard, eval_node_expr};
use purrdf_shapes::report::ValidationReport;
use purrdf_shapes::rules::entail_dataset;
use purrdf_shapes::shapes::Constraint;
use purrdf_shapes::term::{NamedNode, Term};
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;
use purrdf_sparql_eval::{NativeSparqlEngine, QueryOptions};

const PREFIXES: &str = r"
@prefix ex:     <http://example.org/ns#> .
@prefix rdf:    <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs:   <http://www.w3.org/2000/01/rdf-schema#> .
@prefix sh:     <http://www.w3.org/ns/shacl#> .
@prefix shnex:  <http://www.w3.org/ns/shacl-node-expr#> .
@prefix sparql: <http://www.w3.org/ns/sparql#> .
@prefix xsd:    <http://www.w3.org/2001/XMLSchema#> .
";

/// The SHACL 1.2 SPARQL Extensions §7.2 shape of a list-parameter declaration:
/// one `sh:bodyExpression`, `shnex:arg0`-keyed parameters, and the
/// `rdfs:subClassOf` the specification's own examples carry.
///
/// `ex:incomeTotal(x)` is the sum of `x`'s `ex:income` values — a function of the
/// FOCUS GRAPH, which is what makes it a node expression rather than a value-level
/// closure.
const INCOME_TOTAL_FN: &str = r"
ex:incomeTotal a sh:ListParameterExpressionFunction ;
  rdfs:subClassOf sh:ListParameterExpression ;
  sh:bodyExpression [ shnex:sum [ shnex:pathValues ex:income ; shnex:focusNode [ shnex:arg 0 ] ] ] ;
  sh:parameter [ a sh:Parameter ; sh:path shnex:arg0 ; sh:nodeKind sh:IRI ] .
";

const INCOME_DATA: &str = r"
ex:alice ex:income 10, 20 .
ex:bob   ex:income 5 .
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

/// Evaluate the fixture's `sh:expression` over `data_ttl` from `ex:<focus>`.
fn outputs(data_ttl: &str, shapes_ttl: &str, focus: &str) -> Vec<String> {
    let expr = expression_of(shapes_ttl);
    let store = store_of(data_ttl);
    let mut guard = RecursionGuard::new();
    eval_node_expr(&store, &ex_term(focus), &expr, &mut guard)
        .expect("node expression evaluates")
        .iter()
        .map(ToString::to_string)
        .collect()
}

/// The evaluation error the fixture's `sh:expression` produces.
fn eval_error(data_ttl: &str, shapes_ttl: &str, focus: &str) -> String {
    let expr = expression_of(shapes_ttl);
    let store = store_of(data_ttl);
    let mut guard = RecursionGuard::new();
    eval_node_expr(&store, &ex_term(focus), &expr, &mut guard)
        .expect_err("the expression must be refused")
}

/// A `ShaclData` over `data_ttl`, data graph and SPARQL graph alike.
fn store_of(data_ttl: &str) -> ShaclData {
    let data = data_of(data_ttl);
    ShaclData::new(Arc::clone(&data), data, None)
}

/// The frozen data graph for `data_ttl`.
fn data_of(data_ttl: &str) -> Arc<RdfDataset> {
    parse_turtle_to_dataset(&format!("{PREFIXES}{data_ttl}"), None).expect("data parse")
}

/// Validate `data_ttl` against `shapes_ttl` through the production entry point.
fn validate(data_ttl: &str, shapes_ttl: &str) -> Result<ValidationReport, String> {
    let shapes = parse_shapes(&format!("{PREFIXES}{shapes_ttl}"), None)?;
    validate_with(&store_of(data_ttl), &shapes)
}

/// The shapes-load error a malformed fixture produces.
fn load_error(shapes_ttl: &str) -> String {
    parse_shapes(&format!("{PREFIXES}{shapes_ttl}"), None)
        .expect_err("the fixture must be refused at shapes-load")
}

/// The `ex:<local>` term.
fn ex_term(local: &str) -> Term {
    Term::NamedNode(NamedNode::new_unchecked(format!(
        "http://example.org/ns#{local}"
    )))
}

/// The canonical rendering of an `xsd:integer` literal.
fn int(value: &str) -> String {
    format!("\"{value}\"^^<http://www.w3.org/2001/XMLSchema#integer>")
}

// ── SHACL 1.2 Node Expressions §6.2 / §6.3 — list parameter functions ──────────

/// A `sh:ListParameterExpressionFunction` declared with a `sh:bodyExpression` is
/// callable from a `sh:expression` constraint, and returns what its body computes
/// over the focus graph.
#[test]
fn list_parameter_function_returns_its_body_s_value() {
    let shapes = format!(
        "{INCOME_TOTAL_FN}
        ex:S a sh:NodeShape ;
          sh:targetNode ex:alice ;
          sh:expression [ ex:incomeTotal ( sh:this ) ] ."
    );
    assert_eq!(
        outputs(INCOME_DATA, &shapes, "alice"),
        vec![int("30")],
        "the body sums ex:alice's two ex:income values through [ shnex:arg 0 ]"
    );
    assert_eq!(
        outputs(INCOME_DATA, &shapes, "bob"),
        vec![int("5")],
        "the same declaration answers a different argument from the same graph"
    );
}

/// The same declaration drives a real validation verdict: the constraint conforms
/// for the focus whose total matches and reports a violation for the one whose
/// total does not.
///
/// The negative half is the load-bearing one. A validator that could not evaluate
/// the function but still answered `conforms: true` would pass the positive half
/// and fail this.
#[test]
fn list_parameter_function_drives_a_validation_verdict() {
    let shapes = format!(
        "{INCOME_TOTAL_FN}
        ex:S a sh:NodeShape ;
          sh:targetNode ex:alice, ex:bob ;
          sh:expression [ sparql:equals ( [ ex:incomeTotal ( sh:this ) ] 30 ) ] ."
    );
    let report = validate(INCOME_DATA, &shapes).expect("validation runs");
    assert!(!report.conforms, "ex:bob's total is 5, not 30");
    let focus: Vec<String> = report
        .results
        .iter()
        .map(|r| r.focus_node.to_string())
        .collect();
    assert_eq!(
        focus,
        vec![ex_term("bob").to_string()],
        "exactly the focus node whose total differs is reported"
    );
}

/// §6.3: an arg expression whose key is not in the argument scope yields the empty
/// list — but a body that READS such a key is refused at shapes-load, because a
/// silently-empty argument at every call is indistinguishable from a function that
/// works.
#[test]
fn body_reading_an_undeclared_argument_is_a_load_error() {
    let err = load_error(
        r"
        ex:f a sh:ListParameterExpressionFunction ;
          sh:bodyExpression [ shnex:sum [ shnex:arg 3 ] ] ;
          sh:parameter [ sh:path shnex:arg0 ] .
        ",
    );
    assert!(err.contains("shnex:arg 3"), "got: {err}");
    assert!(err.contains("declared parameters"), "got: {err}");
}

/// §6.2 declares the argument parameters as the contiguous `shnex:arg0 …` block, so
/// a call with the wrong number of arguments has no reading at all and is refused at
/// shapes-load rather than quietly binding nothing.
#[test]
fn arity_mismatch_at_the_call_site_is_a_load_error() {
    let err = load_error(
        r"
        ex:pair a sh:ListParameterExpressionFunction ;
          sh:bodyExpression [ sparql:add ( [ shnex:arg 0 ] [ shnex:arg 1 ] ) ] ;
          sh:parameter [ sh:path shnex:arg0 ] ;
          sh:parameter [ sh:path shnex:arg1 ] .

        ex:S a sh:NodeShape ;
          sh:targetNode ex:alice ;
          sh:expression [ ex:pair ( 1 ) ] .
        ",
    );
    assert!(err.contains("with 1 argument(s)"), "got: {err}");
    assert!(err.contains("2..=2"), "got: {err}");
}

/// A declaring class with no `sh:bodyExpression` has no body to evaluate, and is a
/// shapes-LOAD failure rather than a function that answers nothing.
#[test]
fn a_declaration_without_a_body_is_a_load_error() {
    let err = load_error(
        r"
        ex:f a sh:ListParameterExpressionFunction ;
          sh:parameter [ sh:path shnex:arg0 ] .
        ",
    );
    assert!(err.contains("sh:bodyExpression"), "got: {err}");
    assert!(err.contains("exactly one"), "got: {err}");
}

/// A body that names no node-expression kind cannot be evaluated, so the
/// declaration is refused at load.
///
/// The body below carries `sh:then` — node-expression VOCABULARY — with no
/// expression key to own it, so it names no kind and calls no function. (It is
/// deliberately not `[ rdfs:label '…' ]`: SHACL 1.2 Node Expressions gives a
/// one-argument call the list-free form `[ <fn> <arg> ]`, so an arbitrary
/// predicate with an arbitrary object is a well-formed CALL, not a malformed
/// expression — see `a_one_argument_call_may_omit_the_argument_list`.)
#[test]
fn an_unparsable_body_is_a_load_error() {
    let err = load_error(
        r"
        ex:f a sh:ListParameterExpressionFunction ;
          sh:bodyExpression [ sh:then true ] ;
          sh:parameter [ sh:path shnex:arg0 ] .
        ",
    );
    assert!(err.contains("unusable sh:bodyExpression"), "got: {err}");
}

/// `sh:bodyExpression` on a node that is neither declaring class is refused: nothing
/// would ever evaluate that body, and loading it green would leave a `sh:SPARQLFunction`
/// with no reachable body at all.
#[test]
fn body_expression_without_a_declaring_class_is_a_load_error() {
    let err = load_error(
        r"
        ex:f a sh:SPARQLFunction ;
          sh:bodyExpression [ shnex:arg 0 ] ;
          sh:parameter [ sh:path ex:arg ] .
        ",
    );
    assert!(
        err.contains("sh:ListParameterExpressionFunction"),
        "got: {err}"
    );
}

/// SHACL 1.2 SPARQL Extensions §7.2's `ex:spacedConcat`: a `sh:sparqlExpr` body
/// that names its arguments as `$arg0` / `$arg1`.
///
/// The section writes the body that way, so the argument scope has to reach the
/// query as pre-bound variables — otherwise the specification's own example
/// evaluates over unbound variables and answers nothing.
#[test]
fn a_sparql_expr_body_sees_its_arguments_as_pre_bound_variables() {
    let shapes = r#"
        ex:spacedConcat a sh:ListParameterExpressionFunction ;
          rdfs:subClassOf sh:ListParameterExpression ;
          sh:bodyExpression [ sh:sparqlExpr "CONCAT($arg0, ' ', $arg1)" ] ;
          sh:parameter [ sh:path shnex:arg0 ; sh:datatype xsd:string ] ;
          sh:parameter [ sh:path shnex:arg1 ; sh:datatype xsd:string ] .

        ex:S a sh:NodeShape ;
          sh:targetNode ex:alice ;
          sh:expression [ ex:spacedConcat ( "John" "Doe" ) ] .
    "#;
    assert_eq!(
        outputs(INCOME_DATA, shapes, "alice"),
        vec!["\"John Doe\"".to_owned()]
    );
}

/// The `sh:select` spelling of the same seam, and a body that READS THE GRAPH:
/// `$arg0` is pre-bound to the argument and the query counts that node's incomes.
#[test]
fn a_select_body_sees_its_arguments_and_the_focus_graph() {
    let shapes = r#"
        ex:incomeCount a sh:ListParameterExpressionFunction ;
          sh:bodyExpression [ sh:select """
            SELECT (COUNT(?income) AS ?result) WHERE { $arg0 ex:income ?income }
          """ ] ;
          sh:parameter [ sh:path shnex:arg0 ; sh:nodeKind sh:IRI ] .

        ex:S a sh:NodeShape ;
          sh:targetNode ex:alice ;
          sh:expression [ ex:incomeCount ( sh:this ) ] .
    "#;
    assert_eq!(outputs(INCOME_DATA, shapes, "alice"), vec![int("2")]);
    assert_eq!(outputs(INCOME_DATA, shapes, "bob"), vec![int("1")]);
}

// ── SHACL 1.2 Node Expressions §6.1 — named parameter functions ────────────────

/// The specification's own §6.1 example, verbatim in shape: `ex:AverageExpression`
/// with a `sh:keyParameter true` parameter, a `sh:bodyExpression` dividing the sum
/// of the argument by its count, and a call site that names only the KEY parameter.
#[test]
fn named_parameter_function_evaluates_its_body_under_the_argument_scope() {
    let shapes = r"
        ex:AverageExpression a sh:NamedParameterExpressionFunction ;
          rdfs:subClassOf sh:NamedParameterExpression ;
          sh:parameter ex:AverageExpression-average ;
          sh:bodyExpression [ sparql:divide (
            [ shnex:sum   [ shnex:arg ex:average ] ]
            [ shnex:count [ shnex:arg ex:average ] ]
          ) ] .

        ex:AverageExpression-average a sh:Parameter ;
          sh:path ex:average ;
          sh:keyParameter true .

        ex:S a sh:NodeShape ;
          sh:targetNode ex:alice ;
          sh:expression [ ex:average [ shnex:pathValues ex:income ] ] .
    ";
    assert_eq!(
        outputs(INCOME_DATA, shapes, "alice"),
        vec!["\"15\"^^<http://www.w3.org/2001/XMLSchema#decimal>".to_owned()],
        "(10 + 20) / 2 — the MULTI-valued named argument is evaluated where each \
         shnex:arg reads it"
    );
}

/// §6.1 requires at least one `sh:keyParameter true`: without one no call site could
/// ever be recognised, so the declaration is refused at load.
#[test]
fn named_parameter_function_without_a_key_parameter_is_a_load_error() {
    let err = load_error(
        r"
        ex:f a sh:NamedParameterExpressionFunction ;
          sh:bodyExpression [ shnex:arg ex:v ] ;
          sh:parameter [ sh:path ex:v ] .
        ",
    );
    assert!(err.contains("sh:keyParameter"), "got: {err}");
}

/// §6.1 requires key parameters to be disjoint across functions; two functions
/// claiming one key would make a call site ambiguous, so the collision is a load
/// failure.
#[test]
fn colliding_key_parameters_are_a_load_error() {
    let err = load_error(
        r"
        ex:f a sh:NamedParameterExpressionFunction ;
          sh:bodyExpression [ shnex:arg ex:v ] ;
          sh:parameter [ sh:path ex:v ; sh:keyParameter true ] .

        ex:g a sh:NamedParameterExpressionFunction ;
          sh:bodyExpression [ shnex:arg ex:v ] ;
          sh:parameter [ sh:path ex:v ; sh:keyParameter true ] .
        ",
    );
    assert!(err.contains("disjoint"), "got: {err}");
}

/// A named-parameter function has no positional call form — §7.3 registers only the
/// LIST parameter class — so the `[ ex:f ( … ) ]` spelling is refused rather than
/// silently read as a builtin call.
#[test]
fn a_named_parameter_function_has_no_positional_call_form() {
    let err = load_error(
        r"
        ex:f a sh:NamedParameterExpressionFunction ;
          sh:bodyExpression [ shnex:arg ex:v ] ;
          sh:parameter [ sh:path ex:v ; sh:keyParameter true ] .

        ex:S a sh:NodeShape ;
          sh:targetNode ex:alice ;
          sh:expression [ ex:f ( 1 ) ] .
        ",
    );
    assert!(err.contains("no positional call form"), "got: {err}");
}

// ── SHACL 1.2 SPARQL Extensions §7.3 — registration and evaluation ────────────

/// §7.3: the same `sh:ListParameterExpressionFunction` declaration resolves from
/// SPARQL QUERY TEXT — `BIND (ex:incomeTotal(ex:alice) AS ?total)` — with the scope
/// keyed by argument index and the query's own graph as focus graph.
#[test]
fn a_list_parameter_function_resolves_from_sparql_query_text() {
    let shapes = parse_shapes(&format!("{PREFIXES}{INCOME_TOTAL_FN}"), None).expect("shapes parse");
    let data = data_of(INCOME_DATA);
    let engine = NativeSparqlEngine::new();
    let query = "PREFIX ex: <http://example.org/ns#> \
                 SELECT ?total WHERE { BIND (ex:incomeTotal(ex:alice) AS ?total) }";
    let result = engine
        .query_with_options_view(
            &data,
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions {
                functions: &shapes.functions,
                focus_graph: Some(&data),
                ..QueryOptions::EMPTY
            },
        )
        .expect("the registered function resolves");
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("expected solutions");
    };
    let bound: Vec<String> = rows
        .iter()
        .filter_map(|row| row.first().and_then(Option::as_ref))
        .map(|value| format!("{value:?}"))
        .collect();
    assert_eq!(bound.len(), 1, "one row, one bound total");
    assert!(
        bound[0].contains("30"),
        "the function summed the focus graph: {bound:?}"
    );
}

/// §7.3 without a focus graph: the call is REFUSED, never answered.
///
/// A body that reads a graph cannot be evaluated against no graph. Returning an
/// unbound value here would make "this function was never evaluated" look exactly
/// like "this function found nothing", which is the failure mode this project
/// forbids.
#[test]
fn a_registered_function_without_a_focus_graph_refuses_rather_than_answering() {
    let shapes = parse_shapes(&format!("{PREFIXES}{INCOME_TOTAL_FN}"), None).expect("shapes parse");
    let data = data_of(INCOME_DATA);
    let engine = NativeSparqlEngine::new();
    let query = "PREFIX ex: <http://example.org/ns#> \
                 SELECT ?total WHERE { BIND (ex:incomeTotal(ex:alice) AS ?total) }";
    let err = engine
        .query_with_options_view(
            &data,
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions {
                functions: &shapes.functions,
                ..QueryOptions::EMPTY
            },
        )
        .expect_err("a call with no focus graph must be refused");
    let message = err.to_string();
    assert!(message.contains("focus graph"), "got: {message}");
}

/// §7.3 fixes the focus node: "the `focusNode` passed into a custom SPARQL function
/// based on a node expression is the IRI of the function itself".
///
/// The body here is `sh:this` — nothing else — so the returned node IS the focus node
/// the engine passed in, and the assertion reads it directly.
#[test]
fn the_focus_node_of_a_sparql_call_is_the_function_s_own_iri() {
    let shapes = parse_shapes(
        &format!(
            "{PREFIXES}
        ex:whoAmI a sh:ListParameterExpressionFunction ;
          sh:bodyExpression sh:this ;
          sh:parameter [ sh:path shnex:arg0 ] ."
        ),
        None,
    )
    .expect("shapes parse");
    let data = data_of(INCOME_DATA);
    let engine = NativeSparqlEngine::new();
    let query = "PREFIX ex: <http://example.org/ns#> \
                 SELECT ?who WHERE { BIND (ex:whoAmI(1) AS ?who) }";
    let result = engine
        .query_with_options_view(
            &data,
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions {
                functions: &shapes.functions,
                focus_graph: Some(&data),
                ..QueryOptions::EMPTY
            },
        )
        .expect("the registered function resolves");
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("expected solutions");
    };
    let bound: Vec<String> = rows
        .iter()
        .filter_map(|row| row.first().and_then(Option::as_ref))
        .map(|value| format!("{value:?}"))
        .collect();
    assert_eq!(bound.len(), 1);
    assert!(
        bound[0].contains("http://example.org/ns#whoAmI"),
        "got: {bound:?}"
    );
}

// ── Re-entrancy ───────────────────────────────────────────────────────────────

/// A self-recursive expression-bodied function hits the re-entry ceiling and returns
/// a hard `Err` that names it.
///
/// The test BINARY exiting normally is half the assertion: unbounded native recursion
/// in Rust aborts the process, which no `Result` can carry and no caller can catch.
#[test]
fn a_self_recursive_function_fails_closed_at_the_depth_bound() {
    let shapes = r"
        ex:loop a sh:ListParameterExpressionFunction ;
          sh:bodyExpression [ ex:loop ( [ shnex:arg 0 ] ) ] ;
          sh:parameter [ sh:path shnex:arg0 ] .

        ex:S a sh:NodeShape ;
          sh:targetNode ex:alice ;
          sh:expression [ ex:loop ( sh:this ) ] .
    ";
    let err = eval_error(INCOME_DATA, shapes, "alice");
    assert!(err.contains("<http://example.org/ns#loop>"), "got: {err}");
    assert!(err.contains("64"), "the error must name the limit: {err}");
}

/// The same cycle reaching validation is a hard validation failure, not a report
/// claiming conformance over a constraint that was never evaluated.
#[test]
fn a_recursive_function_makes_validation_fail_rather_than_conform() {
    let shapes = r"
        ex:loop a sh:ListParameterExpressionFunction ;
          sh:bodyExpression [ ex:loop ( [ shnex:arg 0 ] ) ] ;
          sh:parameter [ sh:path shnex:arg0 ] .

        ex:S a sh:NodeShape ;
          sh:targetNode ex:alice ;
          sh:expression [ ex:loop ( sh:this ) ] .
    ";
    let err = validate(INCOME_DATA, shapes).expect_err("validation must refuse");
    assert!(err.contains("64"), "got: {err}");
}

/// A cycle that LEAVES the node-expression evaluator through a `sh:select` body and
/// re-enters through the §7.3 SPARQL registration is bounded too.
///
/// Without the call depth travelling with the query, the fresh evaluation context the
/// `sh:select` builds would restart the count at zero and the cycle would never end.
///
/// Driven through `validate_with`, because that is the entry point that installs the
/// shapes graph's function registry for the query the body runs.
#[test]
fn a_cycle_through_sparql_query_text_is_bounded_too() {
    let shapes = r#"
        ex:pingpong a sh:ListParameterExpressionFunction ;
          sh:bodyExpression [ sh:select """
            SELECT ?result WHERE { BIND (ex:pingpong($this) AS ?result) }
          """ ] ;
          sh:parameter [ sh:path shnex:arg0 ] .

        ex:S a sh:NodeShape ;
          sh:targetNode ex:alice ;
          sh:expression [ ex:pingpong ( sh:this ) ] .
    "#;
    let err = validate(INCOME_DATA, shapes).expect_err("the cycle must be refused");
    assert!(
        err.contains("depth bound"),
        "the cycle must be refused by a depth bound: {err}"
    );
}

// ── Freshness across a rules fixpoint ─────────────────────────────────────────

/// A function called during round *n* of the rules fixpoint reads round *n*'s graph.
///
/// Stratum 0 derives `ex:a ex:derived ex:seedValue`. Stratum 1 runs a `sh:SPARQLRule`
/// whose `CONSTRUCT` calls `ex:hasDerived` — a registered expression-bodied function
/// whose body reads `ex:derived`. The copied triple can only exist if the function
/// saw a fact that did not exist when the shapes graph was loaded, so its presence IS
/// the freshness assertion.
#[test]
fn a_function_called_during_the_rules_fixpoint_sees_the_current_round_s_graph() {
    let shapes_ttl = format!(
        "{PREFIXES}
        ex:hasDerived a sh:ListParameterExpressionFunction ;
          sh:bodyExpression [ shnex:pathValues ex:derived ; shnex:focusNode [ shnex:arg 0 ] ] ;
          sh:parameter [ a sh:Parameter ; sh:path shnex:arg0 ; sh:nodeKind sh:IRI ] .

        ex:S a sh:NodeShape ;
          sh:targetNode ex:a ;
          sh:rule [ a sh:TripleRule ;
                    sh:order 0 ;
                    sh:subject sh:this ;
                    sh:predicate ex:derived ;
                    sh:object ex:seedValue ] ;
          sh:rule [ a sh:SPARQLRule ;
                    sh:order 1 ;
                    sh:construct \"CONSTRUCT {{ $this ex:copied ?v }} \
                                  WHERE {{ BIND (ex:hasDerived($this) AS ?v) }}\" ] ."
    );
    let shapes = parse_shapes(&shapes_ttl, None).expect("shapes parse");
    let data = data_of("ex:a a ex:Thing .");
    let entailed = entail_dataset(&data, &shapes).expect("rules run");

    let derived = triples_of(&entailed);
    assert!(
        derived.contains(&(
            "http://example.org/ns#a".to_owned(),
            "http://example.org/ns#derived".to_owned(),
            "http://example.org/ns#seedValue".to_owned(),
        )),
        "stratum 0 must derive ex:a ex:derived ex:seedValue: {derived:?}"
    );
    assert!(
        derived.contains(&(
            "http://example.org/ns#a".to_owned(),
            "http://example.org/ns#copied".to_owned(),
            "http://example.org/ns#seedValue".to_owned(),
        )),
        "stratum 1's function must have READ stratum 0's output; a stale graph would \
         have made ex:hasDerived answer nothing and the copied triple absent: {derived:?}"
    );
}

/// Every `(subject, predicate, object)` of `dataset` rendered as plain strings.
fn triples_of(dataset: &Arc<RdfDataset>) -> Vec<(String, String, String)> {
    use purrdf::DatasetView;
    dataset
        .quad_refs()
        .map(|quad| {
            (
                format!("{:?}", quad.s),
                format!("{:?}", quad.p),
                format!("{:?}", quad.o),
            )
        })
        .map(|(s, p, o)| (strip(&s), strip(&p), strip(&o)))
        .collect()
}

/// The IRI inside a debug-rendered term value.
fn strip(rendered: &str) -> String {
    rendered
        .split_once('"')
        .and_then(|(_, rest)| rest.split_once('"'))
        .map_or_else(|| rendered.to_owned(), |(iri, _)| iri.to_owned())
}

// ── §6.2 optional list parameters (the arity RANGE) ───────────────────────────

/// A `sh:ListParameterExpressionFunction` whose trailing parameter is
/// `sh:optional true` is callable with OR without that argument.
///
/// The arity check refuses a call outside `required..=params.len()`, which is a
/// RANGE, but no fixture anywhere declared `sh:optional`, so the range never had a
/// width: every test exercised `required == params.len()`. The whole optional-
/// parameter surface — the width of that range and the "required after optional"
/// refusal that guards it — was carried by code no test reached.
#[test]
fn an_optional_trailing_parameter_may_be_omitted_or_supplied() {
    const FN: &str = r"
        ex:sumOrPlusOne a sh:ListParameterExpressionFunction ;
          sh:bodyExpression [ shnex:sum [ shnex:concat (
              [ shnex:arg 0 ] [ shnex:arg 1 ] ) ] ] ;
          sh:parameter [ sh:path shnex:arg0 ] ;
          sh:parameter [ sh:path shnex:arg1 ; sh:optional true ] .
        ";
    // Both arguments supplied: the body sums them.
    assert_eq!(
        outputs(
            "ex:alice ex:p ex:b .",
            &format!(
                "{FN}
                 ex:S a sh:NodeShape ; sh:targetNode ex:alice ;
                     sh:expression [ ex:sumOrPlusOne ( 4 38 ) ] ."
            ),
            "alice",
        ),
        vec![int("42")],
        "a two-argument call must supply both parameters"
    );
    // The optional argument OMITTED: `shnex:arg 1` is unbound, contributes
    // nothing, and the call is still legal.
    assert_eq!(
        outputs(
            "ex:alice ex:p ex:b .",
            &format!(
                "{FN}
                 ex:S a sh:NodeShape ; sh:targetNode ex:alice ;
                     sh:expression [ ex:sumOrPlusOne ( 4 ) ] ."
            ),
            "alice",
        ),
        vec![int("4")],
        "omitting the sh:optional parameter must be legal, not an arity error"
    );
}

/// The NEIGHBOURING INVALID cases: the range is bounded on BOTH sides. Below
/// `required` and above `params.len()` are still arity errors, so declaring a
/// parameter optional widened the range rather than removing the check.
#[test]
fn an_optional_parameter_widens_the_arity_range_without_removing_its_bounds() {
    const FN: &str = r"
        ex:sumOrPlusOne a sh:ListParameterExpressionFunction ;
          sh:bodyExpression [ shnex:sum [ shnex:concat (
              [ shnex:arg 0 ] [ shnex:arg 1 ] ) ] ] ;
          sh:parameter [ sh:path shnex:arg0 ] ;
          sh:parameter [ sh:path shnex:arg1 ; sh:optional true ] .
        ";
    for call in ["( )", "( 1 2 3 )"] {
        let err = load_error(&format!(
            "{FN}
             ex:S a sh:NodeShape ; sh:targetNode ex:alice ;
                 sh:expression [ ex:sumOrPlusOne {call} ] ."
        ));
        assert!(
            err.contains("but it declares 1..=2"),
            "the call {call} must be refused against the declared range, got: {err}"
        );
    }
}

/// A REQUIRED parameter declared after an optional one leaves the arity
/// ambiguous, and is refused at load. The refusal existed but nothing reached it,
/// because no fixture declared `sh:optional` at all.
#[test]
fn a_required_parameter_after_an_optional_one_is_a_load_error() {
    let err = load_error(
        r"
        ex:f a sh:ListParameterExpressionFunction ;
          sh:bodyExpression [ shnex:arg 0 ] ;
          sh:parameter [ sh:path shnex:arg0 ] ;
          sh:parameter [ sh:path shnex:arg1 ; sh:optional true ] ;
          sh:parameter [ sh:path shnex:arg2 ] .
        ",
    );
    assert!(err.contains("after an optional one"), "got: {err}");
}
