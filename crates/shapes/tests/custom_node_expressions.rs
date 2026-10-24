// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Integration tests for CUSTOM node-expression functions and the SPARQL
//! functions declared from them.
//!
//! Three W3C sections, driven through the production surfaces only —
//! `engine::parse_shapes` for the shapes graph, `engine::validate_graphs` for
//! validation, `rules::entail_dataset` for rule entailment, and
//! `NativeSparqlEngine` for query text. Direct-entry controls also construct
//! original public function bodies to measure resources and processor refusals.
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

#[path = "support/turtle.rs"]
mod turtle;

#[path = "support/terms.rs"]
mod terms;

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use std::sync::Arc;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

use terms::ex_ns as ex_term;

use purrdf_rdf::{RdfDataset, SparqlRequest, SparqlResult};
use purrdf_shapes::data::ShaclData;
use purrdf_shapes::engine::{parse_shapes, validate_with};
use purrdf_shapes::expression::{
    CustomFunction, RecursionGuard, eval_custom_function_call, eval_node_expr,
};
use purrdf_shapes::report::ValidationReport;
use purrdf_shapes::rules::entail_dataset;
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;
use purrdf_shapes::{FailureClass, ProcessorFailure};
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

/// Evaluate the fixture's `sh:expression` over `data_ttl` from `ex:<focus>`.
fn outputs(data_ttl: &str, shapes_ttl: &str, focus: &str) -> Vec<String> {
    let expr = turtle::expression_of(PREFIXES, shapes_ttl);
    let store = store_of(data_ttl);
    let mut guard = RecursionGuard::default();
    eval_node_expr(&store, &ex_term(focus), &expr, &mut guard)
        .expect("node expression evaluates")
        .iter()
        .map(ToString::to_string)
        .collect()
}

/// The evaluation error the fixture's `sh:expression` produces.
fn eval_error(data_ttl: &str, shapes_ttl: &str, focus: &str) -> String {
    let expr = turtle::expression_of(PREFIXES, shapes_ttl);
    let store = store_of(data_ttl);
    let mut guard = RecursionGuard::default();
    eval_node_expr(&store, &ex_term(focus), &expr, &mut guard)
        .expect_err("the expression must be refused")
        .to_string()
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
    let shapes = parse_shapes(&format!("{PREFIXES}{shapes_ttl}"), None)
        .map_err(|error| error.to_string())?;
    validate_with(&store_of(data_ttl), &shapes).map_err(|error| error.to_string())
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
    let err = turtle::load_error(
        PREFIXES,
        r"
        ex:f a sh:ListParameterExpressionFunction ;
          sh:bodyExpression [ shnex:sum [ shnex:arg 3 ] ] ;
          sh:parameter [ sh:path shnex:arg0 ] .
        ",
    );
    assert!(err.contains("shnex:arg 3"), "got: {err}");
    assert!(err.contains("declared parameters"), "got: {err}");
}

#[test]
fn exact_argument_indices_preserve_the_u64_boundary() {
    let shape = |index: u128| {
        format!(
            "ex:f a sh:ListParameterExpressionFunction ; \
             sh:bodyExpression [ shnex:arg {index} ] ; \
             sh:parameter [ sh:path shnex:arg0 ] ."
        )
    };

    let largest = turtle::load_error(PREFIXES, &shape(u128::from(u64::MAX)));
    assert!(largest.contains("declared parameters"), "got: {largest}");
    assert!(!largest.contains("exceeds u64"), "got: {largest}");

    let too_large = turtle::load_error(PREFIXES, &shape(u128::from(u64::MAX) + 1));
    assert!(too_large.contains("exceeds u64"), "got: {too_large}");
}

/// §6.2 declares the argument parameters as the contiguous `shnex:arg0 …` block, so
/// a call with the wrong number of arguments has no reading at all and is refused at
/// shapes-load rather than quietly binding nothing.
#[test]
fn arity_mismatch_at_the_call_site_is_a_load_error() {
    let err = turtle::load_error(
        PREFIXES,
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
    let err = turtle::load_error(
        PREFIXES,
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
    let err = turtle::load_error(
        PREFIXES,
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
/// with no reachable body at all — "SPARQL-based functions have exactly one value for
/// either sh:ask or sh:select" (SHACL Advanced Features, `SPARQLFunction-query`). The
/// refusal holds although nothing calls the function. The neighbour, the same body
/// under a declaring class, loads and evaluates (`INCOME_TOTAL_FN` throughout this
/// file).
#[test]
fn body_expression_without_a_declaring_class_is_a_load_error() {
    let err = turtle::load_error(
        PREFIXES,
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
    assert!(
        err.contains("the sh:SPARQLFunction <http://example.org/ns#f>")
            && err.contains("[syntax rule SPARQLFunction-query]"),
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
        vec!["\"15.0\"^^<http://www.w3.org/2001/XMLSchema#decimal>".to_owned()],
        "(10 + 20) / 2 — the MULTI-valued named argument is evaluated where each \
         shnex:arg reads it"
    );
}

/// §6.1 requires at least one `sh:keyParameter true`: without one no call site could
/// ever be recognised, so the declaration is refused at load.
#[test]
fn named_parameter_function_without_a_key_parameter_is_a_load_error() {
    let err = turtle::load_error(
        PREFIXES,
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
    let err = turtle::load_error(
        PREFIXES,
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
    let err = turtle::load_error(
        PREFIXES,
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
            QueryOptions::new()
                .with_functions(
                    &purrdf_shapes::sparql::bind_in_current_env(&shapes, None)
                        .expect("the fixture's bodies parse and admit"),
                )
                .with_focus_graph(Some(&data)),
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

/// The first column of every row `query` returns over `data_ttl` with the functions
/// `shapes_ttl` declares registered, `None` for an unbound value.
fn sparql_column(data_ttl: &str, shapes_ttl: &str, query: &str) -> Vec<Option<String>> {
    let shapes = parse_shapes(&format!("{PREFIXES}{shapes_ttl}"), None).expect("shapes parse");
    let data = data_of(data_ttl);
    let result = NativeSparqlEngine::new()
        .query_with_options_view(
            &data,
            SparqlRequest {
                query: &format!("PREFIX ex: <http://example.org/ns#>\n{query}"),
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions::new()
                .with_functions(
                    &purrdf_shapes::sparql::bind_in_current_env(&shapes, None)
                        .expect("the fixture's bodies parse and admit"),
                )
                .with_focus_graph(Some(&data)),
        )
        .expect("the query evaluates: a function error is a SPARQL error, not a failed query");
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("expected solutions");
    };
    rows.iter()
        .map(|row| {
            row.first()
                .and_then(Option::as_ref)
                .map(|v| format!("{v:?}"))
        })
        .collect()
}

/// SHACL 1.2 SPARQL Extensions §7.3 (editor's draft `shacl-ed-2026-10-16`): "an argument
/// that evaluates to a blank node leads to an evaluation failure and thus an error" — the call is unbound, where the same
/// function over an IRI argument returns it. An OPTIONAL parameter turns that error into
/// an argument "unbound in the scope", and the arguments after it keep their indexes.
#[test]
fn a_blank_node_argument_is_an_error_unless_its_parameter_is_optional() {
    const FNS: &str = r"
        ex:identity a sh:ListParameterExpressionFunction ;
          sh:bodyExpression [ shnex:arg 0 ] ;
          sh:parameter [ sh:path shnex:arg0 ] .
        ex:total a sh:ListParameterExpressionFunction ;
          sh:bodyExpression [ shnex:sum [ shnex:concat (
              [ shnex:arg 0 ] [ shnex:arg 1 ] [ shnex:arg 2 ] ) ] ] ;
          sh:parameter [ sh:path shnex:arg0 ] ;
          sh:parameter [ sh:path shnex:arg1 ; sh:optional true ] ;
          sh:parameter [ sh:path shnex:arg2 ; sh:optional true ] .
        ";
    let data = "ex:alice ex:node [ ex:label \"blank\" ] ; ex:iri ex:b .";
    let column = |call: &str| {
        sparql_column(
            data,
            FNS,
            &format!(
                "SELECT ?v WHERE {{ ex:alice ex:node ?blank ; ex:iri ?iri . BIND ({call} AS ?v) }}"
            ),
        )
    };
    assert_eq!(
        column("ex:identity(?blank)"),
        vec![None],
        "a blank node argument to a required parameter is an error"
    );
    assert_eq!(
        column("ex:identity(?iri)").len(),
        1,
        "the neighbouring IRI argument is accepted"
    );
    assert!(column("ex:identity(?iri)")[0].is_some());
    assert!(
        column("ex:total(1, ?blank, 5)")[0]
            .as_ref()
            .is_some_and(|v| v.contains('6')),
        "an optional blank node argument is unbound and the next argument keeps index 2: {:?}",
        column("ex:total(1, ?blank, 5)")
    );
}

/// A blank node as an argument is an error under the editor's draft only. Its text adds
/// "Note that blank nodes cannot be passed as arguments: they are not constant node
/// expressions, so an argument that evaluates to a blank node leads to an evaluation
/// failure and thus an error"; the working draft's §7.3 states only that an argument
/// whose evaluation is an error makes the call an error, so under it the blank node is
/// an ordinary argument and the call returns it.
///
/// Both laws run the same SPARQL-based constraint (the W3C `blankNodeArgument-001`
/// shape) over the same data, which holds a blank node and, beside it, an IRI. The IRI
/// call is the neighbour: it returns its argument under both laws.
#[test]
fn a_blank_node_argument_is_an_error_under_the_editors_draft_and_an_argument_under_the_working_draft()
 {
    use purrdf_shapes::ShaclProfile;
    use purrdf_shapes::ShapesImports;
    use purrdf_shapes::engine::{ValidationOptions, validate_graphs_with_options};

    let shapes = format!(
        r#"{PREFIXES}
        ex:identity a sh:ListParameterExpressionFunction ;
          sh:bodyExpression [ shnex:arg 0 ] ;
          sh:parameter [ sh:path shnex:arg0 ] .
        ex:Shape a sh:NodeShape ; sh:targetNode ex:alice ; sh:sparql [
          sh:select """
            SELECT $this ?value WHERE {{
              $this ex:node ?blank ; ex:iri ?iri .
              BIND (ex:identity(?blank) AS ?fromBlank)
              BIND (ex:identity(?iri) AS ?fromIri)
              BIND (CONCAT(str(bound(?fromBlank)), " ", str(?fromIri)) AS ?value)
            }}""" ] .
        "#
    );
    let data = "<http://example.org/ns#alice> <http://example.org/ns#node> _:b .\n\
                <http://example.org/ns#alice> <http://example.org/ns#iri> <http://example.org/ns#b> .\n";
    let value_under = |law: ShaclProfile| {
        let report = validate_graphs_with_options(
            data,
            &shapes,
            None,
            &ValidationOptions::default().with_profile(law),
            &ShapesImports::new(),
        )
        .unwrap_or_else(|error| panic!("{law}: {error}"));
        let [result] = report.results.as_slice() else {
            panic!("{law}: one result expected, got {}", report.results.len());
        };
        result.value.as_ref().map(ToString::to_string)
    };
    assert_eq!(
        value_under(ShaclProfile::ED_20261016).as_deref(),
        Some("\"false http://example.org/ns#b\""),
        "the editor's draft makes the blank node argument an error, the IRI call returns it"
    );
    assert_eq!(
        value_under(ShaclProfile::WD_20260918).as_deref(),
        Some("\"true http://example.org/ns#b\""),
        "the working draft passes the blank node, and the IRI call is unchanged"
    );
    assert_eq!(
        value_under(ShaclProfile::LEGACY),
        value_under(ShaclProfile::ED_20261016),
        "the compatibility law reads the editor's draft"
    );
}

/// §7.3: "If the list of output nodes `rs` has exactly one member, then return that
/// node. Otherwise, or if the evaluation produces an evaluation failure, the result of
/// the SPARQL function call is an error." A body producing two nodes, or none, is a SPARQL
/// error — an unbound `BIND` — and does not fail the query around it; the neighbouring
/// body producing exactly one node returns it.
#[test]
fn a_body_that_does_not_produce_exactly_one_node_is_a_sparql_error() {
    const FN: &str = r"
        ex:instanceOf a sh:ListParameterExpressionFunction ;
          sh:bodyExpression [ shnex:instancesOf [ shnex:arg 0 ] ] ;
          sh:parameter [ sh:path shnex:arg0 ] .
        ";
    let data = "ex:one a ex:One . ex:first a ex:Two . ex:second a ex:Two .";
    let column = |class: &str| {
        sparql_column(
            data,
            FN,
            &format!("SELECT ?v WHERE {{ BIND (ex:instanceOf({class}) AS ?v) }}"),
        )
    };
    assert_eq!(column("ex:Two"), vec![None], "two nodes: an error");
    assert_eq!(column("ex:None"), vec![None], "no node: an error");
    let one = column("ex:One");
    assert!(
        one[0].as_ref().is_some_and(|v| v.contains("one")),
        "exactly one node is returned: {one:?}"
    );
}

/// An evaluation failure INSIDE a function body is the call's SPARQL error, not a
/// failure of the validation around it. §7.3: "Otherwise, or if the evaluation produces
/// an evaluation failure, the result of the SPARQL function call is an error."
///
/// The body `sparql:abs(<select>)` over a select that returns two nodes is a
/// list-argument cardinality failure ("argument 0 produced 2 nodes"). Called from
/// `BIND`, the variable stays unbound, so `str(bound(?r))` is `"false"`, and the
/// validation produces a report rather than an error.
#[test]
fn a_cardinality_failure_in_a_body_is_a_sparql_error_in_a_report() {
    const FN: &str = r#"
        ex:twoNodes a sh:ListParameterExpressionFunction ;
          sh:bodyExpression [ sparql:abs ( [ sh:select "SELECT ?v WHERE { ?x <http://example.org/ns#n> ?v }" ] ) ] .
        ex:S a sh:NodeShape ;
          sh:targetNode ex:alice ;
          sh:sparql [
            sh:message "unbound" ;
            sh:select """
              SELECT $this ?value WHERE {
                BIND (<http://example.org/ns#twoNodes>() AS ?r)
                BIND (str(bound(?r)) AS ?value)
              }
            """ ;
          ] .
        "#;
    let data = "ex:a ex:n 1 . ex:b ex:n 2 . ex:alice ex:n 3 .";
    let report =
        validate(data, FN).expect("an evaluation failure in the body does not fail validation");
    let values: Vec<String> = report
        .results
        .iter()
        .filter_map(|r| r.value.as_ref().map(ToString::to_string))
        .collect();
    assert_eq!(
        values,
        vec!["\"false\"".to_owned()],
        "the call is unbound: {:?}",
        report.results
    );
    // Neighbour: the same function over ONE node returns its value, so the unbound
    // result above is the body's failure, not the harness.
    let one = sparql_column(
        "ex:a ex:n -1 .",
        r#"ex:oneNode a sh:ListParameterExpressionFunction ;
             sh:bodyExpression [ sparql:abs ( [ sh:select "SELECT ?v WHERE { ?x <http://example.org/ns#n> ?v }" ] ) ] ."#,
        "SELECT ?v WHERE { BIND (ex:oneNode() AS ?v) }",
    );
    assert!(one[0].as_ref().is_some_and(|v| v.contains('1')), "{one:?}");
}

/// A second kind of evaluation failure: a shape argument that names no shape of the
/// shapes graph. Unbound for the call; the neighbouring call naming a real shape returns
/// its verdict. The declared built-in `shnex:conformsToShape` called directly from SPARQL
/// follows the same rule.
#[test]
fn an_unresolvable_shape_in_a_body_is_a_sparql_error() {
    let shapes = format!(
        "ex:conforms a sh:ListParameterExpressionFunction ;
           sh:bodyExpression [ shnex:conformsToShape ( [ shnex:arg 0 ] [ shnex:arg 1 ] ) ] ;
           sh:parameter [ sh:path shnex:arg0 ], [ sh:path shnex:arg1 ] .
         shnex:conformsToShape a sh:ListParameterExpressionFunction .
         {NAMED_SHAPE}"
    );
    let column = |call: &str| {
        sparql_column(
            "ex:a ex:name \"a\" .",
            &shapes,
            &format!("SELECT ?v WHERE {{ BIND ({call} AS ?v) }}"),
        )
    };
    assert_eq!(
        column("ex:conforms(ex:a, ex:Missing)"),
        vec![None],
        "custom body"
    );
    assert_eq!(
        column("<http://www.w3.org/ns/shacl-node-expr#conformsToShape>(ex:a, ex:Missing)"),
        vec![None],
        "built-in"
    );
    for call in [
        "ex:conforms(ex:a, ex:Named)",
        "<http://www.w3.org/ns/shacl-node-expr#conformsToShape>(ex:a, ex:Named)",
    ] {
        let ok = column(call);
        assert!(
            ok[0].as_ref().is_some_and(|v| v.contains("true")),
            "{call}: {ok:?}"
        );
    }
}

/// Neighbour of the evaluation-failure test above: a failure of the PROCESSOR inside a body that is reached through
/// SPARQL query text is not the call's SPARQL error; validation fails, and the typed cause
/// is a `ShapesError::Processor`, not a message.
///
/// `ex:pingpong` calls itself through `BIND` in its own `sh:select` body, so each level is
/// a SPARQL call; the evaluator's own function-depth bound (32) trips first and arrives as the
/// propagated query-engine failure (`ProcessorFailure::Query`), which every enclosing level
/// must pass on. Were that failure read as an evaluation failure, every level would turn it into an unbound value
/// and the validation would complete with a report. Unlike a governor trip, nothing is
/// latched on a shared state, so this test observes the classification alone.
#[test]
fn a_processor_failure_inside_a_sparql_called_body_fails_validation_with_a_typed_cause() {
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
    let parsed = parse_shapes(&format!("{PREFIXES}{shapes}"), None).expect("shapes parse");
    let error = validate_with(&store_of(INCOME_DATA), &parsed)
        .expect_err("a processor failure inside the body must fail the validation");
    assert_eq!(error.failure_class(), FailureClass::Processor, "{error}");
    let Some(ProcessorFailure::Query { code, message }) = error.as_processor() else {
        panic!("expected the query engine's propagated failure, got {error:?}");
    };
    assert_eq!(
        code,
        purrdf_sparql_eval::EvalError::FUNCTION_OPERATIONAL_CODE
    );
    assert!(
        message.contains("recursion exceeded the depth bound of 32"),
        "{message}"
    );

    // Neighbour: the same shape of call that terminates is answered, so the failure above
    // is the depth bound and not the harness.
    let ok = sparql_column(
        INCOME_DATA,
        INCOME_TOTAL_FN,
        "SELECT ?v WHERE { BIND (ex:incomeTotal(ex:alice) AS ?v) }",
    );
    assert!(ok[0].as_ref().is_some_and(|v| v.contains("30")), "{ok:?}");
}

/// The direct form of the same fact: the body-level entry point returns the typed
/// `CallDepth` cause when the guard has no depth left, and an internal defect (a function
/// whose body was never installed) is `ProcessorFailure::Internal`. Both are of class
/// `Processor`, so neither can be mapped to a SPARQL error.
#[test]
fn body_level_processor_failures_are_typed() {
    use purrdf_shapes::expression::{ArgKey, CustomFnKind, NodeExpr};
    let function = |body: Option<NodeExpr>| {
        let installed = std::sync::OnceLock::new();
        if let Some(body) = body {
            installed.set(body).expect("a fresh lock is empty");
        }
        Arc::new(CustomFunction {
            iri: purrdf_shapes::term::NamedNode::new_unchecked("http://example.org/ns#direct"),
            kind: CustomFnKind::ListParameter,
            params: vec![ArgKey::Index(0)],
            required: 1,
            body: installed,
        })
    };
    let store = store_of(INCOME_DATA);
    let args = [Some(ex_term("alice"))];

    let with_body = function(Some(NodeExpr::Constant(ex_term("answer"))));
    let owner =
        purrdf_shapes::shapes::ModelOwner::from_functions(vec![Arc::clone(&with_body)]).unwrap();
    let mut exhausted = RecursionGuard::with_depth(purrdf_shapes::expression::MAX_RECURSION_DEPTH);
    let error = eval_custom_function_call(&store, &owner, &with_body, &args, &mut exhausted)
        .expect_err("no re-entry depth is left");
    assert!(
        matches!(
            error.as_processor(),
            Some(ProcessorFailure::CallDepth { limit: 64, .. })
        ),
        "{error:?}"
    );
    assert_eq!(error.failure_class(), FailureClass::Processor);
    // Neighbour: with depth to spare the same function answers.
    let answer = eval_custom_function_call(
        &store,
        &owner,
        &with_body,
        &args,
        &mut RecursionGuard::default(),
    )
    .expect("the function evaluates");
    assert_eq!(answer, Some(ex_term("answer")));

    let reserve = purrdf_stack::reserve(purrdf_stack::remaining());
    let margin = eval_custom_function_call(
        &store,
        &owner,
        &with_body,
        &args,
        &mut RecursionGuard::default(),
    );
    drop(reserve);
    let margin = margin.expect_err("standalone lowering refuses the physical stack margin");
    assert_eq!(margin.failure_class(), FailureClass::Processor);
    assert!(margin.to_string().contains("stack margin"));

    let uninstalled = function(None);
    let owner =
        purrdf_shapes::shapes::ModelOwner::from_functions(vec![Arc::clone(&uninstalled)]).unwrap();
    let error = eval_custom_function_call(
        &store,
        &owner,
        &uninstalled,
        &args,
        &mut RecursionGuard::default(),
    )
    .expect_err("a function without a body is an internal defect");
    assert!(
        matches!(
            error.as_processor(),
            Some(ProcessorFailure::Internal { .. })
        ),
        "{error:?}"
    );
    assert_eq!(error.failure_class(), FailureClass::Processor);
}

/// Already evaluated values preserve holes in both argument and query lookup.
#[test]
fn evaluated_function_arguments_keep_optional_indices_and_query_bindings() {
    use purrdf_shapes::expression::{ArgKey, CustomFnKind, NodeExpr};
    let store = store_of(INCOME_DATA);
    let args = [Some(ex_term("alice")), None, Some(ex_term("answer"))];
    for (body, expected) in [
        (NodeExpr::Arg(ArgKey::Index(0)), args[0].clone()),
        (NodeExpr::Arg(ArgKey::Index(1)), None),
        (NodeExpr::Arg(ArgKey::Index(2)), args[2].clone()),
        (
            NodeExpr::Select {
                query: "SELECT (IF(BOUND(?arg1), ?arg1, ?arg2) AS ?result) WHERE {}".to_owned(),
                variable: "result".to_owned(),
                key: "sh:select",
            },
            args[2].clone(),
        ),
    ] {
        let function = Arc::new(CustomFunction {
            iri: purrdf_shapes::term::NamedNode::new_unchecked("http://example.org/ns#optional"),
            kind: CustomFnKind::ListParameter,
            params: (0..3).map(ArgKey::Index).collect(),
            required: 0,
            body: std::sync::OnceLock::from(body),
        });
        let owner =
            purrdf_shapes::shapes::ModelOwner::from_functions(vec![Arc::clone(&function)]).unwrap();
        assert_eq!(
            eval_custom_function_call(
                &store,
                &owner,
                &function,
                &args,
                &mut RecursionGuard::default()
            )
            .unwrap(),
            expected
        );
    }
}

/// Measure the actual public direct-call entry without timing or a query wrapper.
#[test]
fn direct_custom_function_calls_bound_argument_allocation_growth() {
    use purrdf_shapes::expression::{ArgKey, CustomFnKind, NodeExpr};

    let store = store_of(INCOME_DATA);
    let mut retained = [None; 2];
    for count in [1_usize, 8, 32, 512] {
        for (body_index, (body, expected)) in [
            (
                NodeExpr::Constant(ex_term("answer")),
                Some(ex_term("answer")),
            ),
            (
                NodeExpr::Arg(ArgKey::Index(u64::try_from(count - 1).unwrap())),
                Some(ex_term("alice")),
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let function = Arc::new(CustomFunction {
                iri: purrdf_shapes::term::NamedNode::new_unchecked(
                    "http://example.org/ns#allocation",
                ),
                kind: CustomFnKind::ListParameter,
                params: (0..count)
                    .map(|index| ArgKey::Index(u64::try_from(index).unwrap()))
                    .collect(),
                required: count,
                body: std::sync::OnceLock::from(body),
            });
            let args = vec![Some(ex_term("alice")); count];
            let owner =
                purrdf_shapes::shapes::ModelOwner::from_functions(vec![Arc::clone(&function)])
                    .unwrap();
            assert_eq!(
                eval_custom_function_call(
                    &store,
                    &owner,
                    &function,
                    &args,
                    &mut RecursionGuard::default()
                )
                .unwrap(),
                expected
            );
            let window = CurrentThreadWindow::open();
            let answer = eval_custom_function_call(
                &store,
                &owner,
                &function,
                &args,
                &mut RecursionGuard::default(),
            );
            let measured = window.close();
            assert_eq!(answer.unwrap(), expected);
            assert!(measured.allocations <= 5, "arguments={count}: {measured:?}");
            assert!(
                measured.requested_bytes <= 214,
                "arguments={count}: {measured:?}"
            );
            assert!(measured.retained_bytes <= 28, "{measured:?}");
            assert!(measured.peak_working_bytes <= 214, "{measured:?}");
            if let Some(previous) = retained[body_index] {
                assert_eq!(
                    measured.retained_bytes, previous,
                    "argument buffers must not be retained by the scalar answer"
                );
            }
            retained[body_index] = Some(measured.retained_bytes);
            eprintln!(
                "arguments={count}, body={body_index}: allocations={}, requested={}, retained={}, peak={}",
                measured.allocations,
                measured.requested_bytes,
                measured.retained_bytes,
                measured.peak_working_bytes,
            );
        }
    }
}

/// Transfer the owned result while bounding traffic to one payload copy.
#[test]
fn direct_custom_function_large_result_is_moved_without_a_second_payload_copy() {
    use purrdf_shapes::expression::{ArgKey, CustomFnKind, NodeExpr};
    use purrdf_shapes::term::{Literal, NamedNode, Term};
    let store = store_of(INCOME_DATA);
    let function = Arc::new(CustomFunction {
        iri: NamedNode::from("http://example.org/ns#payload"),
        kind: CustomFnKind::ListParameter,
        params: vec![ArgKey::Index(0)],
        required: 1,
        body: std::sync::OnceLock::from(NodeExpr::Arg(ArgKey::Index(0))),
    });
    let owner =
        purrdf_shapes::shapes::ModelOwner::from_functions(vec![Arc::clone(&function)]).unwrap();
    for length in [1024_usize, 1024 * 1024] {
        let args = [Some(Term::Literal(Literal::new_simple_literal(
            "x".repeat(length),
        )))];
        let window = CurrentThreadWindow::open();
        let result = eval_custom_function_call(
            &store,
            &owner,
            &function,
            &args,
            &mut RecursionGuard::default(),
        )
        .unwrap()
        .unwrap();
        let measured = window.close();
        eprintln!("payload={length}: {measured:?}");
        let bound = u64::try_from(length).unwrap() + 1024;
        let signed_bound = i64::try_from(bound).unwrap();
        assert!(measured.allocations <= 6, "{measured:?}");
        assert!(
            measured.requested_bytes <= bound,
            "one owned output copy plus fixed call overhead: {measured:?}"
        );
        assert!(measured.retained_bytes <= signed_bound, "{measured:?}");
        assert!(measured.peak_working_bytes <= signed_bound, "{measured:?}");
        drop(args);
        let Term::Literal(literal) = result else {
            panic!("the output remains its original literal")
        };
        assert_eq!(literal.value().len(), length);
        assert!(literal.value().bytes().all(|byte| byte == b'x'));
    }
}

/// A declared built-in list function follows the same §7.3 rule as a custom one (the code
/// path is shared): one output node is the value and no output node is the call's SPARQL
/// error. (A built-in cannot produce MORE than one node: every `sparql:` form is a scalar
/// and `shnex:conformsToShape` yields one boolean, so that arm of the rule is not
/// constructible from a built-in; the custom-body case is tested above.)
#[test]
fn a_declared_builtin_follows_the_same_value_and_error_rule() {
    let shapes = "sparql:abs a sh:ListParameterExpressionFunction .";
    let column = |call: &str| {
        sparql_column(
            "ex:a ex:n 1 .",
            shapes,
            &format!(
                "SELECT ?v WHERE {{ BIND (<http://www.w3.org/ns/sparql#abs>({call}) AS ?v) }}"
            ),
        )
    };
    let value = column("-3");
    assert!(
        value[0].as_ref().is_some_and(|v| v.contains('3')),
        "{value:?}"
    );
    assert_eq!(
        column("\"not a number\""),
        vec![None],
        "an error of the call"
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
            QueryOptions::new().with_functions(
                &purrdf_shapes::sparql::bind_in_current_env(&shapes, None)
                    .expect("the fixture's bodies parse and admit"),
            ),
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
            QueryOptions::new()
                .with_functions(
                    &purrdf_shapes::sparql::bind_in_current_env(&shapes, None)
                        .expect("the fixture's bodies parse and admit"),
                )
                .with_focus_graph(Some(&data)),
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
    dataset
        .as_ref()
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
        let err = turtle::load_error(
            PREFIXES,
            &format!(
                "{FN}
             ex:S a sh:NodeShape ; sh:targetNode ex:alice ;
                 sh:expression [ ex:sumOrPlusOne {call} ] ."
            ),
        );
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
    let err = turtle::load_error(
        PREFIXES,
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

// ── SHACL 1.2 SPARQL Extensions §7.3 — built-in list functions ─────────────────

/// A shape a node conforms to only when it has an `ex:name`.
const NAMED_SHAPE: &str = r"
ex:Named a sh:NodeShape ;
  sh:property [ sh:path ex:name ; sh:minCount 1 ] .
";

/// `ex:S` asks, through SPARQL text, whether each `ex:C` conforms to `ex:Named`.
const CONFORMS_IN_SPARQL_EXPR: &str = r#"
ex:S a sh:NodeShape ;
  sh:targetClass ex:C ;
  sh:expression [ sh:sparqlExpr "shnex:conformsToShape($this, ex:Named)" ] .
"#;

const NAMED_DATA: &str = r#"
ex:a a ex:C ; ex:name "a" .
ex:b a ex:C .
"#;

/// §7.3: "SPARQL engines SHOULD register a function for any SHACL instance of
/// sh:ListParameterExpressionFunction from any provided shapes graph." The
/// vocabulary's bare declaration of the BUILT-IN `shnex:conformsToShape` is such an
/// instance, so it is registered with its NATIVE implementation and is callable
/// from `sh:sparqlExpr`: `ex:b` (no name) fails, `ex:a` conforms.
#[test]
fn a_declared_builtin_list_function_is_callable_from_sparql_expr() {
    let shapes = format!(
        "shnex:conformsToShape a sh:ListParameterExpressionFunction .
         {NAMED_SHAPE}{CONFORMS_IN_SPARQL_EXPR}"
    );
    let report = validate(NAMED_DATA, &shapes).expect("validation runs");
    let focus: Vec<String> = report
        .results
        .iter()
        .map(|r| r.focus_node.to_string())
        .collect();
    assert_eq!(focus, vec![ex_term("b").to_string()]);
}

/// Native and registered SPARQL calls ask the same conformance question under
/// the validation request's severity policy, including foreign focus nodes.
#[test]
fn request_severities_survive_native_and_sparql_function_boundaries() {
    use purrdf_shapes::engine::ValidationOptions;
    use purrdf_shapes::report::{ConformanceDisallows, Severity};

    let invocations = [
        ("native", "shnex:conformsToShape ( sh:this ex:Checked )"),
        ("native custom", "ex:conforms ( sh:this )"),
        (
            "SPARQL built-in",
            r#"sh:sparqlExpr "shnex:conformsToShape($this, ex:Checked)""#,
        ),
        ("SPARQL custom", r#"sh:sparqlExpr "ex:conforms($this)""#),
    ];
    for (severity, levels, expected) in [
        ("sh:Warning", vec![Severity::Violation], true),
        (
            "ex:Advisory",
            vec![
                Severity::Violation,
                Severity::Other(purrdf_shapes::term::NamedNode::from(
                    "http://example.org/ns#Advisory",
                )),
            ],
            false,
        ),
    ] {
        let options = ValidationOptions::default().with_conformance_disallows(
            ConformanceDisallows::new(levels).expect("a nonempty request policy"),
        );
        for data in ["", "ex:n ex:p ex:o ."] {
            let store = store_of(data);
            let observed: Vec<_> = invocations
                .iter()
                .map(|&(label, expression)| {
                    let source = format!(
                        "{PREFIXES}
                         shnex:conformsToShape a sh:ListParameterExpressionFunction .
                         ex:conforms a sh:ListParameterExpressionFunction ;
                           sh:parameter [ sh:path shnex:arg0 ] ;
                           sh:bodyExpression [ shnex:conformsToShape (
                             [ shnex:arg 0 ] ex:Checked ) ] .
                         ex:Checked a sh:NodeShape ; sh:severity {severity} ;
                           sh:nodeKind sh:Literal .
                         ex:S a sh:NodeShape ; sh:targetNode ex:n ;
                           sh:expression [ {expression} ] ."
                    );
                    let mut shapes = parse_shapes(&source, None).expect("valid declarations");
                    let default = validate_with(&store, &shapes).expect("default validation");
                    assert_eq!(default.conforms, severity == "ex:Advisory", "{label}");
                    shapes.set_validation_options(options.clone());
                    let report = validate_with(&store, &shapes).expect("scoped validation");
                    (label, report.conforms)
                })
                .collect();
            assert_eq!(
                observed,
                invocations
                    .iter()
                    .map(|&(label, _)| (label, expected))
                    .collect::<Vec<_>>(),
                "severity={severity}, data={data:?}"
            );
        }
    }
}

/// A bound request owns its policy and declaration domain, but never owns the
/// answer or the dataset of a previous query.
#[test]
fn registered_requests_survive_input_drop_and_keep_independent_graphs_and_policies() {
    use purrdf_rdf::TermValue;
    use purrdf_shapes::engine::ValidationOptions;
    use purrdf_shapes::report::{ConformanceDisallows, Severity};

    let registries = {
        let source = format!(
            "{PREFIXES}
             shnex:conformsToShape a sh:ListParameterExpressionFunction .
             ex:conforms a sh:ListParameterExpressionFunction ;
               sh:parameter [ sh:path shnex:arg0 ] ;
               sh:bodyExpression [ shnex:conformsToShape (
                 [ shnex:arg 0 ] ex:Checked ) ] .
             ex:Checked a sh:NodeShape ; sh:class ex:Top .
             ex:Warning a sh:NodeShape ; sh:severity sh:Warning ; sh:nodeKind sh:Literal .
             ex:nested a sh:ListParameterExpressionFunction ;
               sh:parameter [ sh:path shnex:arg0 ] ;
               sh:bodyExpression [ sh:sparqlExpr \"shnex:conformsToShape($arg0, ex:Checked)\" ] .
             ex:Bottom rdfs:subClassOf ex:Top ."
        );
        let mut shapes = parse_shapes(&source, None).unwrap();
        let defaults = purrdf_shapes::sparql::bind_in_current_env(&shapes, None).unwrap();
        shapes.set_validation_options(
            ValidationOptions::default()
                .with_subclass_of_in_shapes_graph(true)
                .with_conformance_disallows(
                    ConformanceDisallows::new([Severity::Violation]).unwrap(),
                ),
        );
        let scoped = purrdf_shapes::sparql::bind_in_current_env(&shapes, None).unwrap();
        [defaults, scoped]
    };
    // The public query seam borrows a registry value. Moving it out of the
    // factory's Arc and cloning it must preserve both nested calls and ownership.
    let registries = registries.map(|registry| Arc::try_unwrap(registry).unwrap());
    let window = CurrentThreadWindow::open();
    let copies = registries.clone();
    let measured = window.close();
    assert_eq!(measured.allocations, 0, "{measured:?}");
    assert_eq!(measured.requested_bytes, 0, "{measured:?}");
    drop(registries);
    let registries = copies;
    let engine = NativeSparqlEngine::new();
    for (graph, class_conforms) in [
        ("ex:n a ex:Bottom .", true),
        ("ex:n a ex:Other .", false),
        ("ex:n a ex:Bottom .", true),
    ] {
        let data = data_of(graph);
        for (scoped, functions) in registries.iter().enumerate() {
            let result = engine
                .query_with_options_view(
                    &data,
                    SparqlRequest {
                        query: "PREFIX ex: <http://example.org/ns#>
                            PREFIX shnex: <http://www.w3.org/ns/shacl-node-expr#>
                            SELECT ?custom ?builtin ?warning ?nested WHERE {
                              BIND(ex:conforms(ex:n) AS ?custom)
                              BIND(shnex:conformsToShape(ex:n, ex:Checked) AS ?builtin)
                              BIND(shnex:conformsToShape(ex:n, ex:Warning) AS ?warning)
                              BIND(ex:nested(ex:n) AS ?nested)
                            }",
                        base_iri: None,
                        substitutions: &[],
                    },
                    QueryOptions::new()
                        .with_functions(functions)
                        .with_focus_graph(Some(&data)),
                )
                .unwrap();
            let SparqlResult::Solutions { rows, .. } = result else {
                panic!("solutions")
            };
            assert_eq!(
                rows,
                vec![vec![
                    Some(TermValue::boolean(scoped == 1 && class_conforms)),
                    Some(TermValue::boolean(scoped == 1 && class_conforms)),
                    Some(TermValue::boolean(scoped == 1)),
                    Some(TermValue::boolean(scoped == 1 && class_conforms)),
                ]],
                "graph={graph}, scoped={scoped}"
            );
        }
    }
}

/// Restoring a preparation changes storage, not the request used by callbacks,
/// targets, nested native conformance or complete report collection.
#[test]
fn prepared_and_restored_callback_requests_agree_in_complete_reports() {
    use purrdf_shapes::engine::{PreparedShapes, ValidationOptions};
    use purrdf_shapes::product::{HostBindings, ShapesProduct, ShapesProfile};
    use purrdf_shapes::report::{ConformanceDisallows, Severity};

    let prepared = {
        let source = format!(
            r#"{PREFIXES}
             shnex:conformsToShape a sh:ListParameterExpressionFunction .
             ex:Warning a sh:NodeShape ; sh:severity sh:Warning ; sh:nodeKind sh:Literal .
             ex:Select a sh:NodeShape ;
               sh:targetNode [ sh:sparqlExpr "IF(shnex:conformsToShape(ex:n, ex:Warning), ex:n, ex:other)" ] ;
               sh:in ( ex:n ) .
             ex:S a sh:NodeShape ; sh:targetNode ex:n ;
               sh:expression [ sh:sparqlExpr "shnex:conformsToShape($this, ex:Warning)" ] ."#
        );
        PreparedShapes::new(Arc::new(parse_shapes(&source, None).unwrap()))
    };
    let bytes = prepared.to_product(&ShapesProfile::CORE).unwrap();
    let options = ValidationOptions::default()
        .with_conformance_disallows(ConformanceDisallows::new([Severity::Violation]).unwrap());
    let restored = ShapesProduct::open(&bytes)
        .unwrap()
        .admit_with_options(&ShapesProfile::CORE, &HostBindings::empty(), &options)
        .unwrap();
    let configured = prepared.with_validation_options(options);
    drop(bytes);
    let data = data_of("");
    for model in [&configured, &restored] {
        let report = model
            .bind_shared_dataset(Arc::clone(&data))
            .unwrap()
            .validate()
            .unwrap();
        assert!(report.conforms, "{report:?}");
        assert!(report.results.is_empty());
        let complete = model
            .bind_complete_shared_dataset(Arc::clone(&data))
            .unwrap()
            .validate()
            .unwrap()
            .into_legacy();
        assert!(complete.conforms, "{complete:?}");
        assert!(complete.results.is_empty());
    }
}

/// The binding's explicit graph placement overrides the parsed document's
/// default, and remains available inside a shape reached from either callback.
#[test]
fn registered_conformance_preserves_the_bindings_actual_shapes_graph() {
    use purrdf_shapes::engine::PreparedShapes;
    let shapes = parse_shapes(
        &format!(
            r#"{PREFIXES}
             shnex:conformsToShape a sh:ListParameterExpressionFunction .
             ex:conforms a sh:ListParameterExpressionFunction ;
               sh:parameter [ sh:path shnex:arg0 ] ;
               sh:bodyExpression [ shnex:conformsToShape ( [ shnex:arg 0 ] ex:Checked ) ] .
             ex:Marker ex:p ex:o .
             ex:Checked a sh:NodeShape ; sh:sparql [ sh:select """
               SELECT $this WHERE {{
                 FILTER(!BOUND($shapesGraph) || $shapesGraph != <http://example.org/actual>
                   || NOT EXISTS {{ GRAPH $shapesGraph {{
                     <http://example.org/ns#Marker> <http://example.org/ns#p> <http://example.org/ns#o>
                   }} }})
               }}""" ] .
             ex:S a sh:NodeShape ; sh:targetNode ex:n ; sh:expression [ sh:sparqlExpr
               "ex:conforms($this) && shnex:conformsToShape($this, ex:Checked)" ] ."#
        ),
        None,
    ).unwrap();
    let prepared = PreparedShapes::new(Arc::new(shapes));
    let data = data_of("");
    let without = prepared
        .bind_shared_dataset(Arc::clone(&data))
        .unwrap()
        .validate()
        .unwrap();
    assert!(
        !without.conforms,
        "an absent graph context must fail the inner shape"
    );
    let bound = prepared
        .bind_shared_dataset_with_shapes_graph(
            Arc::clone(&data),
            Some("http://example.org/actual"),
            purrdf_rdf::ir::ViewLimits::default(),
        )
        .unwrap();
    assert!(bound.validate().unwrap().conforms);
    let complete = prepared
        .bind_complete_shared_dataset_with_shapes_graph(data, Some("http://example.org/actual"))
        .unwrap()
        .validate()
        .unwrap();
    assert!(complete.into_legacy().conforms);
}

/// Naming the shapes graph makes its rows visible to GRAPH queries; those rows
/// do not become Core data when a registered function re-enters conformance.
#[test]
fn named_shapes_rows_do_not_become_callback_core_data() {
    use purrdf_shapes::engine::PreparedShapes;
    let source = format!(
        r#"{PREFIXES}
         shnex:conformsToShape a sh:ListParameterExpressionFunction .
         ex:n a ex:Top ; ex:flag 1 .
         ex:Checked a sh:NodeShape ; sh:class ex:Top .
         ex:Empty a sh:NodeShape ; sh:property [ sh:path ex:flag ; sh:maxCount 0 ] .
         ex:Native a sh:NodeShape ; sh:targetNode ex:n ;
           sh:expression [ shnex:conformsToShape ( sh:this ex:Checked ) ],
                         [ shnex:conformsToShape ( sh:this ex:Empty ) ] .
         ex:Registered a sh:NodeShape ; sh:targetNode ex:n ;
           sh:expression [ sh:sparqlExpr "shnex:conformsToShape($this, ex:Checked)" ],
                         [ sh:sparqlExpr "shnex:conformsToShape($this, ex:Empty)" ] ."#
    );
    let prepared = PreparedShapes::new(Arc::new(parse_shapes(&source, None).unwrap()));
    for (data, violations) in [("", 2), ("ex:n a ex:Top .", 0)] {
        let data = data_of(data);
        for graph in [None, Some("http://example.org/shapes")] {
            let report = prepared
                .bind_shared_dataset_with_shapes_graph(
                    Arc::clone(&data),
                    graph,
                    purrdf_rdf::ir::ViewLimits::default(),
                )
                .unwrap()
                .validate()
                .unwrap();
            assert_eq!(
                report.results.len(),
                violations,
                "graph={graph:?}: {report:?}"
            );
        }
    }
}

/// Rules use the same request for their native heads, registered calls and
/// conditions, including the fresh class view of a later inference round.
#[test]
fn rule_heads_and_conditions_share_the_callback_validation_request() {
    use purrdf_shapes::engine::ValidationOptions;
    use purrdf_shapes::report::{ConformanceDisallows, Severity};

    let source = format!(
        r#"{PREFIXES}
         shnex:conformsToShape a sh:ListParameterExpressionFunction .
         ex:Warning a sh:NodeShape ; sh:severity sh:Warning ; sh:nodeKind sh:Literal .
         ex:Class a sh:NodeShape ; sh:class ex:Top .
         ex:Bottom rdfs:subClassOf ex:Top .
         ex:S a sh:NodeShape ; sh:targetNode ex:n ;
           sh:rule [ a sh:TripleRule ; sh:subject sh:this ; sh:predicate ex:native ;
             sh:object [ shnex:conformsToShape ( sh:this ex:Warning ) ] ] ;
           sh:rule [ a sh:TripleRule ; sh:subject sh:this ; sh:predicate ex:registered ;
             sh:object [ sh:sparqlExpr "shnex:conformsToShape($this, ex:Warning)" ] ] ;
           sh:rule [ a sh:TripleRule ; sh:subject sh:this ; sh:predicate ex:nativeClass ;
             sh:object [ shnex:conformsToShape ( sh:this ex:Class ) ] ] ;
           sh:rule [ a sh:TripleRule ; sh:subject sh:this ; sh:predicate ex:registeredClass ;
             sh:object [ sh:sparqlExpr "shnex:conformsToShape($this, ex:Class)" ] ] ;
           sh:rule [ a sh:TripleRule ; sh:condition ex:Warning ;
             sh:subject sh:this ; sh:predicate ex:condition ; sh:object ex:yes ] ."#
    );
    let data = data_of("ex:n a ex:Bottom .");
    let mut shapes = parse_shapes(&source, None).unwrap();
    for enabled in [false, true, false] {
        shapes.set_validation_options(if enabled {
            ValidationOptions::default()
                .with_subclass_of_in_shapes_graph(true)
                .with_conformance_disallows(
                    ConformanceDisallows::new([Severity::Violation]).unwrap(),
                )
        } else {
            ValidationOptions::default()
        });
        let entailed = entail_dataset(&data, &shapes).unwrap();
        let triples = triples_of(&entailed);
        for predicate in ["native", "registered", "nativeClass", "registeredClass"] {
            let values: Vec<_> = triples
                .iter()
                .filter(|(_, p, _)| *p == format!("http://example.org/ns#{predicate}"))
                .map(|(_, _, value)| value.as_str())
                .collect();
            assert_eq!(
                values,
                [if enabled { "true" } else { "false" }],
                "{predicate}, enabled={enabled}"
            );
        }
        assert_eq!(
            triples
                .iter()
                .any(|(_, p, _)| p == "http://example.org/ns#condition"),
            enabled
        );
    }
}

/// Binding retains large request payloads once. Repeated callback evaluation
/// must not copy either custom severity IRIs or a long graph-context IRI.
#[test]
fn callback_allocation_does_not_scale_with_retained_request_payloads() {
    use purrdf_shapes::engine::ValidationOptions;
    use purrdf_shapes::report::{ConformanceDisallows, Severity};
    use purrdf_shapes::term::NamedNode;

    let mut shapes = parse_shapes(
        &format!(
            "{PREFIXES}
         shnex:conformsToShape a sh:ListParameterExpressionFunction .
         ex:Warning a sh:NodeShape ; sh:severity sh:Warning ; sh:nodeKind sh:Literal ."
        ),
        None,
    )
    .unwrap();
    let data = data_of("");
    let engine = NativeSparqlEngine::new();
    let query = "PREFIX ex: <http://example.org/ns#>
                 PREFIX shnex: <http://www.w3.org/ns/shacl-node-expr#>
                 SELECT ?result WHERE { BIND(shnex:conformsToShape(ex:n, ex:Warning) AS ?result) }";
    let mut baseline = None;
    for population in [1_usize, 32, 512] {
        let levels = std::iter::once(Severity::Violation).chain((0..population).map(|index| {
            Severity::Other(NamedNode::new_unchecked(format!(
                "http://example.org/severity/{index}"
            )))
        }));
        shapes.set_validation_options(
            ValidationOptions::default()
                .with_conformance_disallows(ConformanceDisallows::new(levels).unwrap()),
        );
        let graph = format!("http://example.org/{}", "g".repeat(population));
        let functions = purrdf_shapes::sparql::bind_in_current_env(&shapes, Some(&graph)).unwrap();
        let evaluate = || {
            engine.query_with_options_view(
                &data,
                SparqlRequest {
                    query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions::new()
                    .with_functions(&functions)
                    .with_focus_graph(Some(&data)),
            )
        };
        evaluate().unwrap();
        let window = CurrentThreadWindow::open();
        let result = evaluate();
        let measured = window.close();
        let SparqlResult::Solutions { rows, .. } = result.unwrap() else {
            panic!("solutions")
        };
        assert_eq!(rows, vec![vec![Some(purrdf_rdf::TermValue::boolean(true))]]);
        let costs = (
            measured.allocations,
            measured.requested_bytes,
            measured.peak_working_bytes,
            measured.retained_bytes,
        );
        match baseline {
            None => baseline = Some(costs),
            Some(expected) => assert_eq!(
                costs, expected,
                "request population={population}: {measured:?}"
            ),
        }
    }
}

/// Cross the production worker threshold under an explicit two-worker pool,
/// then repeat under a different request to expose any retained worker policy.
#[test]
fn callback_request_policy_reaches_serial_and_parallel_focus_chunks() {
    use purrdf_shapes::engine::ValidationOptions;
    use purrdf_shapes::report::{ConformanceDisallows, Severity};
    use std::fmt::Write as _;

    let mut shapes = parse_shapes(
        &format!(
            r#"{PREFIXES}
         shnex:conformsToShape a sh:ListParameterExpressionFunction .
         ex:Warning a sh:NodeShape ; sh:severity sh:Warning ; sh:nodeKind sh:Literal .
         ex:S a sh:NodeShape ; sh:targetSubjectsOf ex:p ;
           sh:expression [ sh:sparqlExpr "shnex:conformsToShape($this, ex:Warning)" ] ."#
        ),
        None,
    )
    .unwrap();
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .unwrap();
    for population in [1, 1025] {
        let mut triples = String::new();
        for index in 0..population {
            writeln!(triples, "ex:n{index} ex:p 1 .").unwrap();
        }
        let data = store_of(&triples);
        for ignore_warning in [true, false, true] {
            shapes.set_validation_options(if ignore_warning {
                ValidationOptions::default().with_conformance_disallows(
                    ConformanceDisallows::new([Severity::Violation]).unwrap(),
                )
            } else {
                ValidationOptions::default()
            });
            let report = pool.install(|| validate_with(&data, &shapes)).unwrap();
            assert_eq!(report.conforms, ignore_warning);
            assert_eq!(
                report.results.len(),
                if ignore_warning { 0 } else { population }
            );
        }
    }
}

/// The neighbour: WITHOUT the declaration the graph provides no instance of the
/// class, so nothing registers the IRI and the SPARQL call has no function to
/// reach — the validation is refused rather than answered, which is a different
/// row from the declared case's single result.
#[test]
fn an_undeclared_builtin_list_function_is_not_registered() {
    let shapes = format!("{NAMED_SHAPE}{CONFORMS_IN_SPARQL_EXPR}");
    let outcome = validate(NAMED_DATA, &shapes);
    assert!(
        outcome.is_err(),
        "an unregistered SPARQL function IRI must not evaluate: {:?}",
        outcome.map(|r| r.results.len())
    );
}

/// A declared `sparql:` built-in is registered too, with its SPARQL meaning.
#[test]
fn a_declared_sparql_builtin_is_callable_from_sparql_expr() {
    let shapes = r#"
        sparql:abs a sh:ListParameterExpressionFunction .
        ex:S a sh:NodeShape ; sh:targetNode ex:alice ;
          sh:expression [ sh:sparqlExpr "sparql:abs(-30) = 30" ] .
    "#;
    let report = validate(INCOME_DATA, shapes).expect("validation runs");
    assert!(report.conforms, "{:?}", report.results);
}

/// A call that cannot be an evaluation failure is not turned into one: SHACL 1.2 SPARQL
/// Extensions §7.3 makes the result of a call an error "if the evaluation produces an
/// evaluation failure", and an evaluation failure is a property of the argument values
/// (a type error, a missing node). A call with the wrong NUMBER of arguments fails the
/// same way for every value, so it is a defect of the query as written, known when the
/// query is built, and fails the query, for a built-in operator and for a custom
/// function alike; a value error of the same function is the call's SPARQL error,
/// unbound. Each refusal is set beside the call that is valid and the value error.
#[test]
fn a_call_of_the_wrong_arity_fails_the_query_while_a_value_error_leaves_it_unbound() {
    let attempt = |shapes: &str, call: &str| {
        let shapes = parse_shapes(&format!("{PREFIXES}{shapes}"), None).expect("shapes parse");
        let data = data_of("ex:a ex:n 1 .");
        let functions = purrdf_shapes::sparql::bind_in_current_env(&shapes, None)
            .expect("the fixture's bodies parse and admit");
        NativeSparqlEngine::new()
            .query_with_options_view(
                &data,
                SparqlRequest {
                    query: &format!(
                        "PREFIX ex: <http://example.org/ns#>\n\
                         SELECT ?v WHERE {{ BIND ({call} AS ?v) }}"
                    ),
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions::new()
                    .with_functions(&functions)
                    .with_focus_graph(Some(&data)),
            )
            .map(|result| match result {
                SparqlResult::Solutions { rows, .. } => rows[0][0].is_some(),
                _ => panic!("expected solutions"),
            })
            .map_err(|error| error.to_string())
    };
    let add = "<http://www.w3.org/ns/sparql#add>";
    let declared = "sparql:add a sh:ListParameterExpressionFunction .";
    // Built-in: valid, value error, wrong arity (both directions).
    assert_eq!(attempt(declared, &format!("{add}(1, 2)")), Ok(true));
    assert_eq!(attempt(declared, &format!("{add}(1, \"x\")")), Ok(false));
    for call in [format!("{add}(1)"), format!("{add}(1, 2, 3)")] {
        let error = attempt(declared, &call).expect_err("the wrong arity fails the query");
        assert!(
            error.contains("takes exactly 2 arguments"),
            "{call}: {error}"
        );
    }
    // A keyword built-in: the same split.
    let abs = "<http://www.w3.org/ns/sparql#abs>";
    let declared_abs = "sparql:abs a sh:ListParameterExpressionFunction .";
    assert_eq!(attempt(declared_abs, &format!("{abs}(-1)")), Ok(true));
    for call in [format!("{abs}()"), format!("{abs}(1, 2)")] {
        assert!(
            attempt(declared_abs, &call).is_err(),
            "{call} fails the query"
        );
    }
    // Custom function: valid, value error, wrong arity.
    assert_eq!(attempt(INCOME_TOTAL_FN, "ex:incomeTotal(ex:a)"), Ok(true));
    assert_eq!(
        attempt(INCOME_TOTAL_FN, "ex:incomeTotal(?unbound)"),
        Ok(false)
    );
    let error = attempt(INCOME_TOTAL_FN, "ex:incomeTotal(ex:a, ex:b)")
        .expect_err("the wrong arity fails the query");
    assert!(error.contains("expects exactly 1 argument"), "{error}");
}
