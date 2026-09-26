// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Integration tests for the W3C "SHACL 1.2 Node Expressions" language.
//!
//! Every test here drives the PRODUCTION surface — `engine::parse_shapes` for the
//! shapes graph, `expression::eval_node_expr` (or `engine::validate_graphs` for the
//! constraint components) for evaluation — and asserts the result set the spec
//! section under test states. A kind that parses but produces nothing is a
//! FAILURE, not a pass: each test names the nodes it expects.
//!
//! Test IRIs live under `example.org`; PurRDF mints no vocabulary of its own, and
//! every `sh:` / `shnex:` term used here is defined by the specification named in
//! the test's doc comment.

use std::sync::Arc;

use purrdf_shapes::data::ShaclData;
use purrdf_shapes::engine::{parse_shapes, validate_graphs};
use purrdf_shapes::expression::{NodeExpr, RecursionGuard, eval_node_expr};
use purrdf_shapes::shapes::Constraint;
use purrdf_shapes::term::Term;
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;

const PREFIXES: &str = r"
@prefix ex:    <http://example.org/ns#> .
@prefix rdf:   <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs:  <http://www.w3.org/2000/01/rdf-schema#> .
@prefix sh:    <http://www.w3.org/ns/shacl#> .
@prefix shnex: <http://www.w3.org/ns/shacl-node-expr#> .
@prefix sparql: <http://www.w3.org/ns/sparql#> .
@prefix xsd:   <http://www.w3.org/2001/XMLSchema#> .
";

/// Parse `shapes_ttl` and return the single `sh:expression` node expression it
/// declares — the production parse path, not a test-only constructor.
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

/// Evaluate the fixture's `sh:expression` over `data_ttl` from focus node
/// `ex:<focus>`, returning the output nodes in the order the evaluator produced
/// them (rendered canonically so ordering assertions are exact).
fn outputs(data_ttl: &str, shapes_ttl: &str, focus: &str) -> Vec<String> {
    let expr = expression_of(shapes_ttl);
    let data: Arc<_> =
        parse_turtle_to_dataset(&format!("{PREFIXES}{data_ttl}"), None).expect("data parse");
    let shapes_ds: Arc<_> = parse_turtle_to_dataset(&format!("{PREFIXES}{shapes_ttl}"), None)
        .expect("shapes data parse");
    let store = ShaclData::new(Arc::clone(&data), shapes_ds, None);
    let focus_term = Term::NamedNode(purrdf_shapes::term::NamedNode::new_unchecked(format!(
        "http://example.org/ns#{focus}"
    )));
    let mut guard = RecursionGuard::new();
    eval_node_expr(&store, &focus_term, &expr, &mut guard)
        .expect("node expression evaluates")
        .iter()
        .map(ToString::to_string)
        .collect()
}

/// The canonical rendering of `ex:<local>` as the evaluator emits it.
fn ex(local: &str) -> String {
    format!("<http://example.org/ns#{local}>")
}

// ── §3.1.3 Triple term expressions ──────────────────────────────────────────────

/// SHACL 1.2 Node Expressions §3.1.3: "The output nodes of a triple term
/// expression are the list consisting of exactly the node expression itself."
///
/// The produced term must BE an RDF 1.2 triple term — not an IRI, not a blank
/// reifier — so this asserts the variant, not just the rendering.
#[test]
fn triple_term_expression_yields_a_triple_term() {
    let shapes = "ex:S a sh:NodeShape ; sh:expression <<( ex:s ex:p ex:o )>> .";
    let expr = expression_of(shapes);
    let data: Arc<_> = parse_turtle_to_dataset(PREFIXES, None).expect("data parse");
    let store = ShaclData::new(Arc::clone(&data), Arc::clone(&data), None);
    let focus = Term::NamedNode(purrdf_shapes::term::NamedNode::new_unchecked(
        "http://example.org/ns#a",
    ));
    let mut guard = RecursionGuard::new();
    let out = eval_node_expr(&store, &focus, &expr, &mut guard).expect("triple term evaluates");
    assert_eq!(
        out.len(),
        1,
        "a triple term expression yields exactly itself"
    );
    let Term::Triple(triple) = &out[0] else {
        panic!("expected an RDF 1.2 triple term, got {:?}", out[0]);
    };
    assert_eq!(triple.subject.to_string(), ex("s"));
    assert_eq!(triple.predicate.as_str(), "http://example.org/ns#p");
    assert_eq!(triple.object.to_string(), ex("o"));
}

/// An RDF 1.2 triple term is a first-class VALUE throughout the node-expression
/// language: a path reaches one out of the data graph, a sequence-valued kind
/// carries it, and it is still a triple term on the way out — never flattened to
/// an IRI or a blank reifier.
#[test]
fn a_triple_term_flows_through_the_expression_language_as_a_value() {
    let expr = expression_of(
        "ex:S a sh:NodeShape ;
             sh:expression [ shnex:concat ( [ shnex:pathValues ex:says ] ex:tail ) ] .",
    );
    let data: Arc<_> = parse_turtle_to_dataset(
        &format!("{PREFIXES} ex:a ex:says <<( ex:s ex:p ex:o )>> ."),
        None,
    )
    .expect("data parse");
    let store = ShaclData::new(Arc::clone(&data), Arc::clone(&data), None);
    let focus = Term::NamedNode(purrdf_shapes::term::NamedNode::new_unchecked(
        "http://example.org/ns#a",
    ));
    let mut guard = RecursionGuard::new();
    let out = eval_node_expr(&store, &focus, &expr, &mut guard).expect("triple-term value flows");
    assert_eq!(
        out.len(),
        2,
        "the path value and the tail constant: {out:?}"
    );
    let Term::Triple(triple) = &out[0] else {
        panic!("expected an RDF 1.2 triple term value, got {:?}", out[0]);
    };
    assert_eq!(triple.subject.to_string(), ex("s"));
    assert_eq!(triple.predicate.as_str(), "http://example.org/ns#p");
    assert_eq!(triple.object.to_string(), ex("o"));
    assert_eq!(out[1].to_string(), ex("tail"));
}

// ── §4.1.1 Empty expressions ────────────────────────────────────────────────────

/// §4.1.1: "A blank node that is not the subject of any triple is called an empty
/// expression"; "An empty expression has the empty list `[]` as its output nodes."
#[test]
fn empty_expression_yields_no_nodes() {
    let out = outputs(
        "ex:a ex:p ex:b .",
        "ex:S a sh:NodeShape ; sh:expression [] .",
        "a",
    );
    assert!(
        out.is_empty(),
        "an empty expression yields nothing: {out:?}"
    );
}

// ── §4.1.2 Var expressions ──────────────────────────────────────────────────────

/// §4.1.2 case 1: `shnex:var "focusNode"` resolves to the focus node, ahead of any
/// scope lookup.
#[test]
fn var_focus_node_resolves_to_the_focus() {
    let out = outputs(
        "ex:a ex:p ex:b .",
        r#"ex:S a sh:NodeShape ; sh:expression [ shnex:var "focusNode" ] ."#,
        "a",
    );
    assert_eq!(out, vec![ex("a")]);
}

/// §4.1.2 case 3: a name that is not in scope yields the EMPTY LIST — an absence,
/// not an error.
#[test]
fn var_unbound_name_yields_no_nodes() {
    let out = outputs(
        "ex:a ex:p ex:b .",
        r#"ex:S a sh:NodeShape ; sh:expression [ shnex:var "nobodyBoundThis" ] ."#,
        "a",
    );
    assert!(out.is_empty(), "an unbound var yields nothing: {out:?}");
}

/// §4.1.2 case 2 reached through the §7.1 expression-constraint scope: an
/// expression constraint evaluates as `evalExpr(expr, data graph, focusNode,
/// {value: v})`, so `shnex:var "value"` resolves to the value node under test.
///
/// The fixture asserts the binding is REAL and per-value: only the value node that
/// is `ex:ok` satisfies the constraint, so exactly the other one is reported.
#[test]
fn var_value_resolves_to_the_value_node_under_test() {
    let data = concat!(
        "<http://example.org/ns#a> <http://example.org/ns#p> <http://example.org/ns#ok> .\n",
        "<http://example.org/ns#a> <http://example.org/ns#p> <http://example.org/ns#bad> .\n",
    );
    let shapes = format!(
        r#"{PREFIXES}
        ex:S a sh:NodeShape ;
            sh:targetNode ex:a ;
            sh:property [
                sh:path ex:p ;
                sh:expression [
                    shnex:conformsToShape ( [ shnex:var "value" ] ex:OkShape )
                ] ;
            ] .
        ex:OkShape a sh:NodeShape ; sh:in ( ex:ok ) .
        "#
    );
    let report = validate_graphs(data, &shapes, None).expect("validation runs");
    let offenders: Vec<String> = report
        .results
        .iter()
        .filter_map(|r| r.value.as_ref().map(ToString::to_string))
        .collect();
    assert_eq!(
        offenders,
        vec![ex("bad")],
        "shnex:var \"value\" must resolve per value node"
    );
}

// ── §4.1.3 List expressions ─────────────────────────────────────────────────────

/// §4.1.3: "The output nodes of a list expression are the members of the list
/// expression, in the same order as in the list."
///
/// SEQUENCE-valued: the assertion is the authored order, NOT the canonical sort
/// order (which would put `ex:a` first).
#[test]
fn list_expression_preserves_authored_order() {
    let out = outputs(
        "ex:a ex:p ex:b .",
        "ex:S a sh:NodeShape ; sh:expression ( ex:c ex:a ex:b ) .",
        "a",
    );
    assert_eq!(out, vec![ex("c"), ex("a"), ex("b")]);
}

// ── §4.1.4 Path values expressions ──────────────────────────────────────────────

/// §4.1.4 without `shnex:focusNode`: the value nodes of the path from the
/// evaluation context's focus node.
#[test]
fn path_values_walks_from_the_context_focus() {
    let out = outputs(
        "ex:a ex:p ex:b, ex:c .",
        "ex:S a sh:NodeShape ; sh:expression [ shnex:pathValues ex:p ] .",
        "a",
    );
    assert_eq!(out, vec![ex("b"), ex("c")]);
}

/// §4.1.4 with `shnex:focusNode`: the path is walked from the single node the
/// focus expression produces.
#[test]
fn path_values_walks_from_a_computed_focus() {
    let out = outputs(
        "ex:a ex:p ex:b . ex:b ex:q ex:z .",
        "ex:S a sh:NodeShape ;
             sh:expression [ shnex:pathValues ex:q ; shnex:focusNode [ shnex:pathValues ex:p ] ] .",
        "a",
    );
    assert_eq!(out, vec![ex("z")]);
}

/// §4.1.4: "If `N` has more than 1 member, an evaluation failure is reported."
#[test]
fn path_values_multi_valued_focus_is_a_failure() {
    let expr = expression_of(
        "ex:S a sh:NodeShape ;
             sh:expression [ shnex:pathValues ex:q ; shnex:focusNode [ shnex:pathValues ex:p ] ] .",
    );
    let data: Arc<_> = parse_turtle_to_dataset(&format!("{PREFIXES} ex:a ex:p ex:b, ex:c ."), None)
        .expect("data parse");
    let store = ShaclData::new(Arc::clone(&data), Arc::clone(&data), None);
    let focus = Term::NamedNode(purrdf_shapes::term::NamedNode::new_unchecked(
        "http://example.org/ns#a",
    ));
    let mut guard = RecursionGuard::new();
    let err = eval_node_expr(&store, &focus, &expr, &mut guard)
        .expect_err("a multi-valued shnex:focusNode is an evaluation failure");
    assert!(err.contains("shnex:focusNode"), "got: {err}");
}

// ── §4.2.3 Concat expressions ───────────────────────────────────────────────────

/// §4.2.3: "The output nodes ... are the concatenation of all output nodes for
/// each node expression `NE` in `members`."
///
/// SEQUENCE-valued: operand order is preserved AND duplicates survive, which is
/// exactly what separates `shnex:concat` from the SHACL-AF set union `sh:union`.
#[test]
fn concat_preserves_operand_order_and_duplicates() {
    let out = outputs(
        "ex:a ex:p ex:b .",
        "ex:S a sh:NodeShape ; sh:expression [ shnex:concat ( ex:z [ shnex:pathValues ex:p ] ex:z ) ] .",
        "a",
    );
    assert_eq!(out, vec![ex("z"), ex("b"), ex("z")]);
}

// ── §4.2.4 Remove expressions ───────────────────────────────────────────────────

/// §4.2.4: the nodes of `shnex:nodes` "except those that are also in `M`,
/// preserving the order of `N`".
///
/// Modelled on the spec's own Example 6 (superclasses minus the roots).
#[test]
fn remove_drops_the_named_nodes_preserving_order() {
    let out = outputs(
        "ex:a ex:p ex:b, ex:c, ex:d .",
        "ex:S a sh:NodeShape ;
             sh:expression [
                 shnex:nodes [ shnex:concat ( ex:d ex:c ex:b ) ] ;
                 shnex:remove ( ex:c ) ;
             ] .",
        "a",
    );
    assert_eq!(
        out,
        vec![ex("d"), ex("b")],
        "removal keeps the input sequence's order"
    );
}

/// §4.2.4: "Nodes must be equal using term equality, i.e., `\"01\"^^xsd:integer` is
/// distinct from `\"1\"^^xsd:integer`."
#[test]
fn remove_uses_term_equality_not_value_equality() {
    let out = outputs(
        "ex:a ex:p ex:b .",
        r#"ex:S a sh:NodeShape ;
             sh:expression [
                 shnex:nodes ( "01"^^xsd:integer ) ;
                 shnex:remove ( "1"^^xsd:integer ) ;
             ] ."#,
        "a",
    );
    assert_eq!(
        out,
        vec![r#""01"^^<http://www.w3.org/2001/XMLSchema#integer>"#.to_owned()],
        "value-equal but term-distinct literals must NOT be removed"
    );
}

// ── §4.3.1 FlatMap expressions ──────────────────────────────────────────────────

/// §4.3.1: `shnex:flatMap` is evaluated once per input node WITH THAT NODE AS
/// FOCUS, and the resulting sequences are concatenated in input order.
///
/// Modelled on the spec's own department/revenue example: the sum of the flattened
/// revenues is the point, so the flattening must actually visit each department.
#[test]
fn flat_map_rebinds_the_focus_per_input_node() {
    let out = outputs(
        "ex:a ex:dept ex:d1, ex:d2 . ex:d1 ex:revenue ex:r1 . ex:d2 ex:revenue ex:r2 .",
        "ex:S a sh:NodeShape ;
             sh:expression [
                 shnex:nodes [ shnex:pathValues ex:dept ] ;
                 shnex:flatMap [ shnex:pathValues ex:revenue ] ;
             ] .",
        "a",
    );
    assert_eq!(out, vec![ex("r1"), ex("r2")]);
}

/// §4.3.1: "`shnex:nodes` ... If omitted, defaults to the focus node."
#[test]
fn flat_map_defaults_its_input_to_the_focus_node() {
    let out = outputs(
        "ex:a ex:p ex:b, ex:c .",
        "ex:S a sh:NodeShape ; sh:expression [ shnex:flatMap [ shnex:pathValues ex:p ] ] .",
        "a",
    );
    assert_eq!(out, vec![ex("b"), ex("c")]);
}

/// SHACL Advanced Features 1.1, "Path Expressions": "For the path expression $expr
/// that has the property path P as its value for sh:path and the node expression N as
/// its value for sh:nodes (defaulting to the focus node expression if absent)", the
/// output is "the list of values of all nodes produced by Eval(N, $this) for the
/// property path P" — the path walked from EVERY input node, which is the SHACL 1.2
/// flatMap of the path over N, with the same output. Two input nodes, so a form that
/// read only one of them, or walked from the focus node instead, differs.
#[test]
fn an_af_path_expression_walks_from_each_of_its_sh_nodes() {
    let data = "ex:a ex:dept ex:d1, ex:d2 . ex:d1 ex:revenue ex:r1 . ex:d2 ex:revenue ex:r2 .
                ex:a ex:revenue ex:own .";
    let af = outputs(
        data,
        "ex:S a sh:NodeShape ;
             sh:expression [ sh:path ex:revenue ; sh:nodes [ sh:path ex:dept ] ] .",
        "a",
    );
    let flat_map = outputs(
        data,
        "ex:S a sh:NodeShape ;
             sh:expression [
                 shnex:nodes [ shnex:pathValues ex:dept ] ;
                 shnex:flatMap [ shnex:pathValues ex:revenue ] ;
             ] .",
        "a",
    );
    assert_eq!(af, vec![ex("r1"), ex("r2")]);
    assert_eq!(af, flat_map);
    // The neighbour without sh:nodes walks from the focus node.
    let from_focus = outputs(
        data,
        "ex:S a sh:NodeShape ; sh:expression [ sh:path ex:revenue ] .",
        "a",
    );
    assert_eq!(from_focus, vec![ex("own")]);
}

/// A path expression given both `sh:nodes` and `shnex:focusNode` names two sources
/// for the nodes its path starts from, and is refused; each alone loads (above, and
/// `path_values_walks_from_a_computed_focus`).
#[test]
fn a_path_expression_with_sh_nodes_and_a_focus_node_is_refused() {
    let error = parse_shapes(
        &format!(
            "{PREFIXES}
             ex:S a sh:NodeShape ;
                 sh:expression [ sh:path ex:q ; sh:nodes [ sh:path ex:p ] ;
                                 shnex:focusNode [ sh:path ex:p ] ] ."
        ),
        None,
    )
    .expect_err("two sources for the path's start nodes");
    assert!(
        error.to_string().contains("shnex:focusNode") && error.to_string().contains("sh:nodes"),
        "{error}"
    );
}

// ── §4.3.2 FindFirst expressions ────────────────────────────────────────────────

/// §4.3.2: "the first node `n` in `N` that conforms to the shape `shape`, or an
/// empty sequence if no such node exists."
#[test]
fn find_first_returns_the_first_conforming_node() {
    let shapes = "ex:S a sh:NodeShape ;
         sh:expression [
             shnex:nodes [ shnex:concat ( ex:x ex:y ex:z ) ] ;
             shnex:findFirst ex:SeniorShape ;
         ] .
     ex:SeniorShape a sh:NodeShape ; sh:in ( ex:y ex:z ) .";
    let out = outputs("ex:a ex:p ex:b .", shapes, "a");
    assert_eq!(out, vec![ex("y")], "the FIRST match in input order wins");
}

/// §4.3.2: no conforming node yields the empty sequence.
#[test]
fn find_first_yields_nothing_when_no_node_conforms() {
    let shapes = "ex:S a sh:NodeShape ;
         sh:expression [
             shnex:nodes [ shnex:concat ( ex:x ex:y ) ] ;
             shnex:findFirst ex:NeverShape ;
         ] .
     ex:NeverShape a sh:NodeShape ; sh:in ( ex:q ) .";
    let out = outputs("ex:a ex:p ex:b .", shapes, "a");
    assert!(out.is_empty(), "no match yields nothing: {out:?}");
}

// ── §4.3.3 MatchAll expressions ─────────────────────────────────────────────────

/// §4.3.3: "( true ) if every node `n` in `N` conforms to the shape ... otherwise
/// the output nodes are ( false )."
#[test]
fn match_all_is_true_only_when_every_node_conforms() {
    let all = "ex:S a sh:NodeShape ;
         sh:expression [
             shnex:nodes [ shnex:concat ( ex:x ex:y ) ] ;
             shnex:matchAll ex:ActiveShape ;
         ] .
     ex:ActiveShape a sh:NodeShape ; sh:in ( ex:x ex:y ) .";
    assert_eq!(outputs("ex:a ex:p ex:b .", all, "a"), vec![bool_lit(true)]);

    let some = "ex:S a sh:NodeShape ;
         sh:expression [
             shnex:nodes [ shnex:concat ( ex:x ex:q ) ] ;
             shnex:matchAll ex:ActiveShape ;
         ] .
     ex:ActiveShape a sh:NodeShape ; sh:in ( ex:x ex:y ) .";
    assert_eq!(
        outputs("ex:a ex:p ex:b .", some, "a"),
        vec![bool_lit(false)]
    );
}

// ── §4.5.1 InstancesOf expressions ──────────────────────────────────────────────

/// §4.5.1: "the nodes that are SHACL instances of `type` in the focus graph", and
/// the spec's own note: "the definition of SHACL instance includes instances of
/// subclasses of the given class."
#[test]
fn instances_of_includes_subclass_instances() {
    let out = outputs(
        "ex:Sub rdfs:subClassOf ex:Super .
         ex:direct a ex:Super .
         ex:viaSub a ex:Sub .
         ex:unrelated a ex:Other .",
        "ex:S a sh:NodeShape ; sh:expression [ shnex:instancesOf ex:Super ] .",
        "a",
    );
    assert_eq!(out, vec![ex("direct"), ex("viaSub")]);
}

// ── §4.5.2 NodesMatching expressions ────────────────────────────────────────────

/// §4.5.2: "the nodes in the focus graph that conform to `shape`."
///
/// Modelled on the spec's own "companies with at least N employees" example.
#[test]
fn nodes_matching_selects_every_conforming_node_of_the_graph() {
    let shapes = "ex:S a sh:NodeShape ; sh:expression [ shnex:nodesMatching ex:BigShape ] .
     ex:BigShape a sh:NodeShape ;
         sh:property [ sh:path ex:employee ; sh:minCount 2 ] .";
    let out = outputs(
        "ex:big ex:employee ex:e1, ex:e2 .
         ex:small ex:employee ex:e3 .",
        shapes,
        "a",
    );
    assert!(
        out.contains(&ex("big")),
        "the conforming node must be selected: {out:?}"
    );
    assert!(
        !out.contains(&ex("small")),
        "the non-conforming node must be excluded: {out:?}"
    );
}

// ── §4.5.3 ConformsToShape expressions ──────────────────────────────────────────

/// §4.5.3: "( true ) if and only if `node` conforms to `shape` ... and ( false )
/// otherwise."
///
/// Modelled on the spec's own `shnex:conformsToShape ( [ shnex:var "focusNode" ]
/// ex:HasDirectorShape )` example.
#[test]
fn conforms_to_shape_reports_the_conformance_of_one_node() {
    let shapes = r#"ex:S a sh:NodeShape ;
         sh:expression [
             shnex:conformsToShape ( [ shnex:var "focusNode" ] ex:HasDirectorShape )
         ] .
     ex:HasDirectorShape a sh:NodeShape ;
         sh:property [ sh:path ex:director ; sh:minCount 1 ] ."#;
    let data = "ex:withDirector ex:director ex:d . ex:without ex:name ex:n .";
    assert_eq!(outputs(data, shapes, "withDirector"), vec![bool_lit(true)]);
    assert_eq!(outputs(data, shapes, "without"), vec![bool_lit(false)]);
}

// ── §7.2 sh:nodeByExpression ────────────────────────────────────────────────────

/// SHACL 1.2 Node Expressions §7.2: for each value node `v`, a conformance check
/// of `v` against each output node of `evalExpr(expr, data graph, v, {})`; a
/// non-conforming `v` produces a validation result with `v` as its `sh:value`.
///
/// The result must carry `sh:NodeByExpressionConstraintComponent` — the component
/// IRI the spec names — and NOT the `sh:expression` component.
#[test]
fn node_by_expression_reports_its_own_constraint_component() {
    let data = concat!(
        "<http://example.org/ns#a> <http://example.org/ns#p> <http://example.org/ns#good> .\n",
        "<http://example.org/ns#a> <http://example.org/ns#p> <http://example.org/ns#bad> .\n",
        "<http://example.org/ns#good> <http://example.org/ns#kind> \
         <http://example.org/ns#Shape> .\n",
        "<http://example.org/ns#bad> <http://example.org/ns#kind> \
         <http://example.org/ns#Shape> .\n",
        "<http://example.org/ns#good> <http://example.org/ns#name> \
         <http://example.org/ns#n> .\n",
    );
    // The expression computes the shape from the value node itself: every value
    // node points at ex:Shape via ex:kind, so both are judged against it, and only
    // the one lacking ex:name violates.
    let shapes = format!(
        "{PREFIXES}
        ex:S a sh:NodeShape ;
            sh:targetNode ex:a ;
            sh:property [
                sh:path ex:p ;
                sh:nodeByExpression [ shnex:pathValues ex:kind ] ;
            ] .
        ex:Shape a sh:NodeShape ;
            sh:property [ sh:path ex:name ; sh:minCount 1 ] .
        "
    );
    let report = validate_graphs(data, &shapes, None).expect("validation runs");
    assert!(!report.conforms, "the missing ex:name must be reported");
    assert_eq!(report.results.len(), 1, "exactly one violation");
    let result = &report.results[0];
    assert_eq!(
        result.source_constraint_component.as_str(),
        "http://www.w3.org/ns/shacl#NodeByExpressionConstraintComponent",
    );
    assert_eq!(
        result.value.as_ref().map(ToString::to_string),
        Some(ex("bad")),
        "the offending value node is reported as sh:value"
    );
}

// ── Dual spelling: one IR, one evaluator, two spec-defined surfaces ─────────────

/// A kind spelled with SHACL Advanced Features `sh:` terms and the same kind
/// spelled with SHACL 1.2 Node Expressions `shnex:` terms must produce
/// BYTE-IDENTICAL results: there is one intermediate representation and one
/// evaluation path behind both surfaces.
#[test]
fn sh_and_shnex_spellings_produce_identical_results() {
    let data = "ex:a ex:p ex:b, ex:c, ex:d ; ex:n 1, 2, 3 .
                ex:b ex:ok true . ex:c ex:ok true .";
    // One pair per DUAL-SPELLED kind. The set is not a judgement call: the unit
    // test `the_dual_spelled_kinds_are_exactly_these` (in
    // `crates/shapes/src/shapes/parser/node_expr.rs`) reads `PRIMARY_KEYS` and
    // pins the eleven names below EXACTLY, so a twelfth kind fails there rather
    // than quietly going uncovered here.
    let pairs: [(&str, &str); 11] = [
        ("[ sh:path ex:p ]", "[ shnex:pathValues ex:p ]"),
        (
            "[ sh:min [ sh:path ex:n ] ]",
            "[ shnex:min [ shnex:pathValues ex:n ] ]",
        ),
        (
            "[ sh:max [ sh:path ex:n ] ]",
            "[ shnex:max [ shnex:pathValues ex:n ] ]",
        ),
        (
            "[ sh:sum [ sh:path ex:n ] ]",
            "[ shnex:sum [ shnex:pathValues ex:n ] ]",
        ),
        (
            "[ sh:filterShape [ sh:nodeKind sh:IRI ] ; sh:nodes [ sh:path ex:p ] ]",
            "[ shnex:filterShape [ sh:nodeKind sh:IRI ] ; \
              shnex:nodes [ shnex:pathValues ex:p ] ]",
        ),
        (
            "[ sh:count [ sh:path ex:p ] ]",
            "[ shnex:count [ shnex:pathValues ex:p ] ]",
        ),
        (
            "[ sh:distinct [ sh:path ex:p ] ]",
            "[ shnex:distinct [ shnex:pathValues ex:p ] ]",
        ),
        (
            "[ sh:exists [ sh:path ex:p ] ]",
            "[ shnex:exists [ shnex:pathValues ex:p ] ]",
        ),
        (
            "[ sh:intersection ( [ sh:path ex:p ] ( ex:b ex:c ) ) ]",
            "[ shnex:intersection ( [ shnex:pathValues ex:p ] ( ex:b ex:c ) ) ]",
        ),
        // SHACL-AF 1.1 `sh:minus` + `sh:nodes` against SHACL 1.2 `shnex:remove` +
        // `shnex:nodes`: `ex:p`'s values minus `( ex:c )` — `( ex:b ex:d )`, in
        // path order.
        (
            "[ sh:minus ( ex:c ) ; sh:nodes [ sh:path ex:p ] ]",
            "[ shnex:remove ( ex:c ) ; shnex:nodes [ shnex:pathValues ex:p ] ]",
        ),
        (
            "[ sh:if [ sh:exists [ sh:path ex:p ] ] ; sh:then ex:yes ; sh:else ex:no ]",
            "[ shnex:if [ shnex:exists [ shnex:pathValues ex:p ] ] ; \
              shnex:then ex:yes ; shnex:else ex:no ]",
        ),
    ];
    for (af, ne) in pairs {
        let af_out = outputs(
            data,
            &format!("ex:S a sh:NodeShape ; sh:expression {af} ."),
            "a",
        );
        let ne_out = outputs(
            data,
            &format!("ex:S a sh:NodeShape ; sh:expression {ne} ."),
            "a",
        );
        assert_eq!(
            af_out, ne_out,
            "the sh: and shnex: spellings of {af} must agree"
        );
        assert!(
            !af_out.is_empty(),
            "the dual-spelling fixture {af} must actually produce nodes"
        );
    }
}

/// The `shnex:` paging expressions are named-parameter functions carrying their
/// own `shnex:nodes` operand, where the SHACL-AF keys WRAP the node's own core
/// expression. Different syntax, same `NodeExpr` arm, same result.
#[test]
fn sh_and_shnex_paging_spellings_agree() {
    let data = "ex:a ex:p ex:b, ex:c, ex:d .";
    let af = outputs(
        data,
        "ex:S a sh:NodeShape ; sh:expression [ sh:path ex:p ; sh:limit 2 ; sh:offset 1 ] .",
        "a",
    );
    let ne = outputs(
        data,
        "ex:S a sh:NodeShape ;
             sh:expression [
                 shnex:limit 2 ;
                 shnex:nodes [ shnex:offset 1 ; shnex:nodes [ shnex:pathValues ex:p ] ] ;
             ] .",
        "a",
    );
    assert_eq!(af, vec![ex("c"), ex("d")]);
    assert_eq!(af, ne, "paging must agree across the two spellings");
}

/// The ORDERING half of the paging surface: `sh:orderby` + `sh:desc` (wrappers)
/// and `shnex:orderBy` + `shnex:desc` (a named-parameter core) must agree.
///
/// Ordering was the half of the paging surface no test reached, in either
/// spelling — and it is the half where a dropped key is invisible, because a
/// mis-read `desc` does not fail, it silently returns the sequence reversed.
#[test]
fn sh_and_shnex_ordering_spellings_agree() {
    let data = "ex:a ex:n 1, 2, 3 .";
    let ascending = vec![
        r#""1"^^<http://www.w3.org/2001/XMLSchema#integer>"#.to_owned(),
        r#""2"^^<http://www.w3.org/2001/XMLSchema#integer>"#.to_owned(),
        r#""3"^^<http://www.w3.org/2001/XMLSchema#integer>"#.to_owned(),
    ];
    let descending: Vec<String> = ascending.iter().rev().cloned().collect();

    for (desc, expected) in [(false, &ascending), (true, &descending)] {
        let af = outputs(
            data,
            &format!(
                "ex:S a sh:NodeShape ;
                     sh:expression [ sh:path ex:n ; sh:orderby sh:this ; sh:desc {desc} ] ."
            ),
            "a",
        );
        let ne = outputs(
            data,
            &format!(
                "ex:S a sh:NodeShape ;
                     sh:expression [ shnex:orderBy sh:this ;
                                     shnex:nodes [ shnex:pathValues ex:n ] ;
                                     shnex:desc {desc} ] ."
            ),
            "a",
        );
        assert_eq!(&af, expected, "sh:orderby with sh:desc {desc}");
        assert_eq!(
            af, ne,
            "the two ordering spellings must agree at sh:desc {desc}"
        );
    }
    assert_ne!(
        ascending, descending,
        "the fixture must actually distinguish the two directions"
    );
}

/// A node carrying BOTH the `sh:` and the `shnex:` spelling of the same kind is
/// AMBIGUOUS and hard-fails — the writer is asked which they meant, never silently
/// given one of them.
#[test]
fn both_spellings_of_one_kind_on_one_node_is_ambiguous() {
    let err = parse_shapes(
        &format!(
            "{PREFIXES}
         ex:S a sh:NodeShape ;
             sh:expression [ sh:count [ sh:path ex:p ] ; shnex:count [ shnex:pathValues ex:p ] ] ."
        ),
        None,
    )
    .expect_err("two spellings of one kind must hard-fail")
    .to_string();
    assert!(err.contains("ambiguous node expression"), "got: {err}");
}

/// Two DIFFERENT kinds on one node stay ambiguous too — the pre-existing guard is
/// untouched by the dual-spelling table.
#[test]
fn two_different_kinds_on_one_node_is_ambiguous() {
    let err = parse_shapes(
        &format!(
            "{PREFIXES}
         ex:S a sh:NodeShape ;
             sh:expression [ sh:count [ sh:path ex:p ] ; shnex:sum [ shnex:pathValues ex:p ] ] ."
        ),
        None,
    )
    .expect_err("two kinds on one node must hard-fail")
    .to_string();
    assert!(err.contains("ambiguous node expression"), "got: {err}");
}

/// The canonical `xsd:boolean` rendering the evaluator emits.
fn bool_lit(b: bool) -> String {
    let lexical = if b { "true" } else { "false" };
    format!(r#""{lexical}"^^<http://www.w3.org/2001/XMLSchema#boolean>"#)
}

// ── Shape-valued operands must name a shape (silent-vacuous-success guard) ──────

/// Parse `shapes_ttl` and return the load error, asserting the load FAILED.
fn load_error(shapes_ttl: &str) -> String {
    parse_shapes(&format!("{PREFIXES}{shapes_ttl}"), None)
        .expect_err("this shapes graph must fail to load")
        .to_string()
}

/// Parse `shapes_ttl` and assert it LOADS, returning nothing but the proof.
fn loads(shapes_ttl: &str) {
    parse_shapes(&format!("{PREFIXES}{shapes_ttl}"), None).expect("this shapes graph must load");
}

/// A node expression whose SHAPE operand names a node the shapes graph never
/// described as a shape must be a LOAD error at every one of the five shape-valued
/// kinds — never a green load.
///
/// This is the `sh:condition` defect generalised. `parse_inline_shape` answers an
/// undescribed node with an EMPTY shape and every node conforms to an empty shape,
/// so `shnex:matchAll ex:Typo` does not fail, it reports `true`; `shnex:findFirst`
/// returns the first input node; `sh:filterShape` filters nothing out;
/// `shnex:conformsToShape` reports `true`; `shnex:nodesMatching` matches
/// everything. Each is a silent success the author never earned.
#[test]
fn a_shape_operand_that_is_not_a_shape_fails_the_load_at_every_kind() {
    // (fixture, the operand's owner as the diagnostic must name it)
    let cases: [(&str, &str); 5] = [
        (
            "sh:expression [ shnex:matchAll ex:NotAShape ; shnex:nodes [ shnex:pathValues ex:p ] ]",
            "shnex:matchAll",
        ),
        (
            "sh:expression [ shnex:findFirst ex:NotAShape ; shnex:nodes [ shnex:pathValues ex:p ] ]",
            "shnex:findFirst",
        ),
        (
            "sh:expression [ shnex:filterShape ex:NotAShape ; shnex:nodes [ shnex:pathValues ex:p ] ]",
            "shnex:filterShape",
        ),
        (
            "sh:expression [ shnex:conformsToShape ( [ shnex:var \"focusNode\" ] ex:NotAShape ) ]",
            "shnex:conformsToShape",
        ),
        (
            "sh:expression [ shnex:nodesMatching ex:NotAShape ]",
            "shnex:nodesMatching",
        ),
    ];
    for (body, owner) in cases {
        let err = load_error(&format!(
            "ex:S a sh:NodeShape ; sh:targetNode ex:a ; {body} ."
        ));
        assert!(
            err.contains("does not describe as a shape"),
            "{owner} must refuse an undefined shape operand, got: {err}"
        );
        assert!(
            err.contains("vacuously"),
            "the diagnostic must say WHY silence is the danger, got: {err}"
        );
        assert!(
            err.contains(owner),
            "the diagnostic must name the operand's owner {owner}, got: {err}"
        );
    }
}

/// The NEIGHBOURING VALID case for every one of the five: the very same fixtures
/// with a shape operand the shapes graph really does describe must still LOAD.
///
/// Three spellings of "is a shape" are exercised, because
/// [`Parser::node_is_a_shape`] accepts all three and narrowing it to `a
/// sh:NodeShape` would reject legal SHACL: an explicitly typed node shape, an
/// UNTYPED node the shapes graph makes a SHACL statement about (`sh:nodeKind`),
/// and an ANONYMOUS inline shape.
#[test]
fn the_same_five_kinds_still_load_when_the_operand_really_is_a_shape() {
    let declarations = [
        // Explicitly typed.
        "ex:Op a sh:NodeShape ; sh:nodeKind sh:IRI .",
        // Untyped, but the shapes graph makes a SHACL statement about it.
        "ex:Op sh:nodeKind sh:IRI .",
        // A top-level sh:PropertyShape.
        "ex:Op a sh:PropertyShape ; sh:path ex:name ; sh:minCount 1 .",
    ];
    let bodies = [
        "sh:expression [ shnex:matchAll ex:Op ; shnex:nodes [ shnex:pathValues ex:p ] ]",
        "sh:expression [ shnex:findFirst ex:Op ; shnex:nodes [ shnex:pathValues ex:p ] ]",
        "sh:expression [ shnex:filterShape ex:Op ; shnex:nodes [ shnex:pathValues ex:p ] ]",
        "sh:expression [ shnex:conformsToShape ( [ shnex:var \"focusNode\" ] ex:Op ) ]",
        "sh:expression [ shnex:nodesMatching ex:Op ]",
    ];
    for declaration in declarations {
        for body in bodies {
            loads(&format!(
                "{declaration}\n ex:S a sh:NodeShape ; sh:targetNode ex:a ; {body} ."
            ));
        }
    }
    // And the ANONYMOUS inline shape, which has no IRI to declare.
    loads(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
             sh:expression [ shnex:matchAll [ sh:nodeKind sh:IRI ] ;
                             shnex:nodes [ shnex:pathValues ex:p ] ] .",
    );
}

/// The refusal changes the ANSWER, not just the diagnostic: with the operand
/// declared as a shape, `shnex:matchAll` over literal value nodes reports `false`.
/// Before the guard, the identical shapes graph with an undefined operand reported
/// `true` — the "all clear" reading — which is the reason silence here is a bug
/// and not a nicety.
#[test]
fn match_all_against_a_declared_shape_still_answers_false_for_non_conforming_nodes() {
    let shapes = "ex:MustBeIri a sh:NodeShape ; sh:nodeKind sh:IRI .
         ex:S a sh:NodeShape ;
             sh:expression [ shnex:matchAll ex:MustBeIri ;
                             shnex:nodes [ shnex:pathValues ex:score ] ] .";
    assert_eq!(
        outputs("ex:a ex:score 1, 2, 3 .", shapes, "a"),
        vec![bool_lit(false)],
        "literal value nodes are not IRIs, so matchAll must answer false"
    );
}

// ── Every authored key must be one the selected kind reads ──────────────────────

/// A node-expression key the SELECTED kind does not read is a LOAD error, not a
/// silently discarded operand.
///
/// Each pair below is the SAME expression written with one key spelled for the
/// other surface. Before this check the mis-spelled key was accepted and dropped,
/// and the drop CHANGED THE ANSWER: the `shnex:nodes` operand fell back to the
/// focus node, both `if` branches became the empty expression, and the `desc` flag
/// silently reverted to ascending.
#[test]
fn a_key_the_selected_kind_does_not_read_is_refused_rather_than_dropped() {
    let cases: [(&str, &str); 5] = [
        // `shnex:matchAll` reads `shnex:nodes`; `sh:nodes` is the other surface.
        (
            "sh:expression [ shnex:matchAll ex:Op ; sh:nodes [ shnex:pathValues ex:p ] ]",
            "http://www.w3.org/ns/shacl#nodes",
        ),
        // `shnex:if` reads `shnex:then`/`shnex:else`.
        (
            "sh:expression [ shnex:if true ; sh:then true ; shnex:else false ]",
            "http://www.w3.org/ns/shacl#then",
        ),
        // `sh:if` reads `sh:then`/`sh:else`.
        (
            "sh:expression [ sh:if true ; shnex:then true ; sh:else false ]",
            "http://www.w3.org/ns/shacl-node-expr#then",
        ),
        // `shnex:orderBy` reads `shnex:desc`; `sh:desc` modifies the `sh:orderby`
        // WRAPPER, which this node does not carry.
        (
            "sh:expression [ shnex:orderBy sh:this ; shnex:nodes [ shnex:pathValues ex:p ] ;
                             sh:desc true ]",
            "http://www.w3.org/ns/shacl#desc",
        ),
        // A path expression has no `shnex:nodes` operand at all.
        (
            "sh:expression [ shnex:pathValues ex:p ; shnex:nodes [ shnex:pathValues ex:q ] ]",
            "http://www.w3.org/ns/shacl-node-expr#nodes",
        ),
    ];
    for (body, dropped) in cases {
        let err = load_error(&format!(
            "ex:Op a sh:NodeShape ; sh:nodeKind sh:IRI .
             ex:S a sh:NodeShape ; sh:targetNode ex:a ; {body} ."
        ));
        assert!(
            err.contains(dropped),
            "the diagnostic must name the key that would have been dropped ({dropped}), got: {err}"
        );
        assert!(
            err.contains("silently discarded"),
            "the diagnostic must say the key would have been dropped, got: {err}"
        );
    }
}

/// The NEIGHBOURING VALID case for each of the five: spelled consistently, every
/// one of those expressions loads.
#[test]
fn the_same_five_expressions_load_when_the_keys_are_spelled_consistently() {
    let bodies = [
        "sh:expression [ shnex:matchAll ex:Op ; shnex:nodes [ shnex:pathValues ex:p ] ]",
        "sh:expression [ shnex:if true ; shnex:then true ; shnex:else false ]",
        "sh:expression [ sh:if true ; sh:then true ; sh:else false ]",
        "sh:expression [ shnex:orderBy sh:this ; shnex:nodes [ shnex:pathValues ex:p ] ;
                         shnex:desc true ]",
        "sh:expression [ shnex:pathValues ex:p ]",
    ];
    for body in bodies {
        loads(&format!(
            "ex:Op a sh:NodeShape ; sh:nodeKind sh:IRI .
             ex:S a sh:NodeShape ; sh:targetNode ex:a ; {body} ."
        ));
    }
    // `sh:desc` IS accepted when the `sh:orderby` wrapper it modifies is present —
    // the check is about a dropped key, not about the `sh:` spelling.
    loads(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
             sh:expression [ shnex:pathValues ex:p ; sh:orderby sh:this ; sh:desc true ] .",
    );
}

/// An unrecognised term in the `shnex:` namespace is a misspelling, not an
/// extension point: that namespace is entirely node-expression vocabulary, so a
/// typo is refused rather than accepted and ignored.
#[test]
fn an_unrecognised_shnex_term_on_an_expression_node_is_refused() {
    let err = load_error(
        "ex:Op a sh:NodeShape ; sh:nodeKind sh:IRI .
         ex:S a sh:NodeShape ; sh:targetNode ex:a ;
             sh:expression [ shnex:matchAll ex:Op ; shnex:nodez [ shnex:pathValues ex:p ] ] .",
    );
    assert!(
        err.contains("shnex:nodez") || err.contains("shacl-node-expr#nodez"),
        "the diagnostic must name the misspelled term, got: {err}"
    );
    assert!(
        err.contains("not a term of the SHACL 1.2 Node Expressions vocabulary"),
        "the diagnostic must say the term is not vocabulary, got: {err}"
    );
}

/// The check is NOT a total key scan, and that is deliberate: the annotations an
/// expression constraint legitimately carries (`sh:message`, `sh:severity`),
/// `rdf:type`, and any application vocabulary hung off the node must all still
/// load. A total scan here would be the mirror bug — refusing valid documents.
#[test]
fn annotations_and_application_vocabulary_on_an_expression_node_still_load() {
    loads(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
             sh:expression [ a ex:MyAnnotation ;
                             shnex:pathValues ex:p ;
                             sh:message \"the path must yield an IRI\" ;
                             sh:severity sh:Warning ;
                             ex:authoredBy \"someone\" ] .",
    );
}

/// A `shnex:ListExpression` member may be an RDF 1.2 TRIPLE TERM.
///
/// §3.1.3 makes a triple term a first-class constant expression, a bare one
/// parses as exactly that, and `shnex:concat` carries triple terms through the
/// sequence-valued kinds — so refusing one only inside a list would make a triple
/// term legal everywhere in the language EXCEPT there.
#[test]
fn a_list_expression_member_may_be_a_triple_term() {
    let shapes = "ex:S a sh:NodeShape ; sh:expression ( ex:c <<( ex:s ex:p ex:o )>> ) .";
    assert_eq!(
        outputs("ex:a ex:p ex:b .", shapes, "a"),
        vec![
            ex("c"),
            format!("<<( {} {} {} )>>", ex("s"), ex("p"), ex("o")),
        ],
        "a triple term must survive a list expression in authored order"
    );
}

/// The NEIGHBOURING INVALID case: a BLANK NODE member is still refused. The
/// widening admitted triple terms, which are values; it did not admit an
/// unevaluated structure smuggled in as one.
#[test]
fn a_list_expression_member_may_still_not_be_a_blank_node() {
    let err = load_error("ex:S a sh:NodeShape ; sh:expression ( ex:c [ sh:path ex:p ] ) .");
    assert!(
        err.contains("must be an IRI, a literal or a triple term"),
        "got: {err}"
    );
}

// ── §7.2 sh:nodeByExpression resolves its named shapes at LOAD ──────────────────

/// A `sh:nodeByExpression` naming a shape that does not exist must fail the LOAD,
/// even when the shape carrying it targets NOTHING.
///
/// The constraint resolves its produced shape IRIs per value node, during
/// validation. For a computed expression that is the only time it can. For a
/// CONSTANT it is not: the answer is decided at load, and deferring it meant a
/// shape whose target selected no nodes shipped a constraint naming a shape that
/// does not exist — green load, green report, nothing checked. That is the
/// resolve-at-firing-time defect `sh:condition` carried.
#[test]
fn node_by_expression_naming_a_missing_shape_fails_the_load_even_with_no_targets() {
    let err = load_error(
        "ex:S a sh:NodeShape ;
             sh:targetClass ex:NobodyHasThisClass ;
             sh:nodeByExpression ex:NotAShape .",
    );
    assert!(
        err.contains("is not a shape of this shapes graph"),
        "got: {err}"
    );
    assert!(
        err.contains("checking nothing"),
        "the diagnostic must say what the silence would have cost, got: {err}"
    );
}

/// The list and conditional spellings are resolved at load too — a named shape is
/// named whether it is written bare, inside a list, or as an `sh:if` branch.
#[test]
fn node_by_expression_resolves_named_shapes_through_lists_and_branches() {
    for expr in [
        "( ex:Good ex:NotAShape )",
        "[ sh:if true ; sh:then ex:Good ; sh:else ex:NotAShape ]",
        "[ sh:union ( ex:Good ex:NotAShape ) ]",
    ] {
        let err = load_error(&format!(
            "ex:Good a sh:NodeShape ; sh:nodeKind sh:IRI .
             ex:S a sh:NodeShape ; sh:targetClass ex:NobodyHasThisClass ;
                 sh:nodeByExpression {expr} ."
        ));
        assert!(
            err.contains("ex:NotAShape") || err.contains("ns#NotAShape"),
            "the diagnostic must name the missing shape in {expr}, got: {err}"
        );
    }
}

/// The NEIGHBOURING VALID cases: the same three spellings naming shapes that DO
/// exist still load, and a COMPUTED expression — whose shape IRIs genuinely are
/// not known until validation — is not touched by the load-time check at all.
#[test]
fn node_by_expression_still_loads_for_named_shapes_and_computed_expressions() {
    for expr in [
        "ex:Good",
        "( ex:Good ex:Other )",
        "[ sh:if true ; sh:then ex:Good ; sh:else ex:Other ]",
        "[ sh:union ( ex:Good ex:Other ) ]",
        // Computed: the shape IRI comes out of the DATA graph at validation time,
        // so no load-time resolution is possible and none is attempted.
        "[ sh:path ex:shapeOf ]",
        "[ shnex:pathValues ex:shapeOf ]",
    ] {
        loads(&format!(
            "ex:Good a sh:NodeShape ; sh:nodeKind sh:IRI .
             ex:Other a sh:NodeShape ; sh:nodeKind sh:IRI .
             ex:S a sh:NodeShape ; sh:targetClass ex:NobodyHasThisClass ;
                 sh:nodeByExpression {expr} ."
        ));
    }
}

// ── §4.5.3 the shape argument is a node EXPRESSION ──────────────────────────────

/// §4.5.3's second argument is a NODE EXPRESSION constrained to produce "the IRI
/// of a well-formed shape" — *produce* being the node-expression verb. So the
/// shape may be COMPUTED out of the data graph, and this is the specification's
/// own shape: the node under test carries a property whose value names the shape
/// it must conform to.
///
/// Requiring the argument to BE an IRI refused this outright, at shapes-load, with
/// "requires its second argument to be the IRI of a shape".
#[test]
fn conforms_to_shape_accepts_a_computed_shape_argument() {
    let shapes = "ex:HasDirectorShape a sh:NodeShape ;
             sh:property [ sh:path ex:director ; sh:minCount 1 ] .
         ex:S a sh:NodeShape ;
             sh:expression [ shnex:conformsToShape (
                 [ shnex:var \"focusNode\" ]
                 [ shnex:pathValues ex:kind ] ) ] .";
    // `ex:withDirector` has a director, so it conforms to the shape its own
    // `ex:kind` names.
    assert_eq!(
        outputs(
            "ex:withDirector ex:director ex:d ; ex:kind ex:HasDirectorShape .",
            shapes,
            "withDirector",
        ),
        vec![bool_lit(true)]
    );
    // The SAME shapes graph must answer `false` for a node that does not conform —
    // otherwise the test would pass on a stub that always says `true`, which is
    // exactly the vacuous answer an unresolved shape used to give.
    assert_eq!(
        outputs(
            "ex:noDirector ex:kind ex:HasDirectorShape .",
            shapes,
            "noDirector",
        ),
        vec![bool_lit(false)]
    );
}

/// A computed shape argument that produces an IRI which is NOT a shape of the
/// shapes graph is a hard error at evaluation — the only moment it can be known,
/// since the IRI came out of the data.
#[test]
fn a_computed_shape_argument_that_names_no_shape_is_an_error() {
    let expr = expression_of(
        "ex:Real a sh:NodeShape ; sh:nodeKind sh:IRI .
         ex:S a sh:NodeShape ;
             sh:expression [ shnex:conformsToShape (
                 [ shnex:var \"focusNode\" ] [ shnex:pathValues ex:kind ] ) ] .",
    );
    let data: Arc<_> =
        parse_turtle_to_dataset(&format!("{PREFIXES} ex:a ex:kind ex:NotAShape ."), None)
            .expect("data parse");
    let store = ShaclData::new(Arc::clone(&data), Arc::clone(&data), None);
    let focus = Term::NamedNode(purrdf_shapes::term::NamedNode::new_unchecked(
        "http://example.org/ns#a",
    ));
    let mut guard = RecursionGuard::new();
    let err = eval_node_expr(&store, &focus, &expr, &mut guard)
        .expect_err("an unresolvable computed shape must be an error, never a vacuous true");
    assert!(
        err.contains("is not a shape of this shapes graph"),
        "got: {err}"
    );
}

/// The NAMED spelling keeps its LOAD-time refusal: a bare IRI the shapes graph
/// never described as a shape is a constant that evaluates to itself, so deferring
/// it would only reach the same answer later. Admitting computed arguments must
/// not turn every typo into one.
#[test]
fn conforms_to_shape_still_refuses_a_named_shape_that_does_not_exist() {
    let err = load_error(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
             sh:expression [ shnex:conformsToShape (
                 [ shnex:var \"focusNode\" ] ex:NotAShape ) ] .",
    );
    assert!(err.contains("does not describe as a shape"), "got: {err}");
    assert!(err.contains("vacuously"), "got: {err}");
}

/// A computed shape argument producing more than one IRI has no single shape to
/// check against, and one producing none has no shape at all.
#[test]
fn a_computed_shape_argument_must_produce_exactly_one_iri() {
    let expr = expression_of(
        "ex:Real a sh:NodeShape ; sh:nodeKind sh:IRI .
         ex:S a sh:NodeShape ;
             sh:expression [ shnex:conformsToShape (
                 [ shnex:var \"focusNode\" ] [ shnex:pathValues ex:kind ] ) ] .",
    );
    for (data_ttl, count) in [
        ("ex:a ex:kind ex:Real, ex:Other .", 2),
        ("ex:a ex:p ex:b .", 0),
    ] {
        let data: Arc<_> =
            parse_turtle_to_dataset(&format!("{PREFIXES}{data_ttl}"), None).expect("data parse");
        let store = ShaclData::new(Arc::clone(&data), Arc::clone(&data), None);
        let focus = Term::NamedNode(purrdf_shapes::term::NamedNode::new_unchecked(
            "http://example.org/ns#a",
        ));
        let mut guard = RecursionGuard::new();
        let err = eval_node_expr(&store, &focus, &expr, &mut guard)
            .expect_err("a shape argument that is not exactly one IRI must be an error");
        assert!(
            err.contains("must produce exactly one shape IRI"),
            "at {count} produced IRIs, got: {err}"
        );
    }
}

// ── Per-arm sequence contracts (the fixes that came with them) ─────────────────

/// As [`outputs`], but returning the evaluation failure instead of asserting its
/// absence.
fn try_outputs(data_ttl: &str, shapes_ttl: &str, focus: &str) -> Result<Vec<String>, String> {
    let expr = expression_of(shapes_ttl);
    let data: Arc<_> =
        parse_turtle_to_dataset(&format!("{PREFIXES}{data_ttl}"), None).expect("data parse");
    let shapes_ds: Arc<_> = parse_turtle_to_dataset(&format!("{PREFIXES}{shapes_ttl}"), None)
        .expect("shapes data parse");
    let store = ShaclData::new(Arc::clone(&data), shapes_ds, None);
    let focus_term = Term::NamedNode(purrdf_shapes::term::NamedNode::new_unchecked(format!(
        "http://example.org/ns#{focus}"
    )));
    let mut guard = RecursionGuard::new();
    eval_node_expr(&store, &focus_term, &expr, &mut guard)
        .map(|out| out.iter().map(ToString::to_string).collect())
}

/// The canonical rendering of an `xsd:integer` literal.
fn integer(lexical: &str) -> String {
    format!(r#""{lexical}"^^<http://www.w3.org/2001/XMLSchema#integer>"#)
}

/// §4.2.5: "the output nodes of evalExpr(nodes, …) except those that do not
/// conform to the shape filterShape, preserving the order in the list". The
/// survivors keep their input ORDER and their MULTIPLICITY: `3` passes twice and
/// stays in front of `1`, and the non-integer `2.5` is the only node dropped.
#[test]
fn filter_shape_preserves_input_order_and_duplicates() {
    let out = outputs(
        "",
        "ex:Integers a sh:NodeShape ; sh:datatype xsd:integer .
         ex:S a sh:NodeShape ;
             sh:expression [ shnex:filterShape ex:Integers ;
                             shnex:nodes [ shnex:concat ( ( 3 1 ) ( 2.5 3 ) ) ] ] .",
        "a",
    );
    assert_eq!(out, vec![integer("3"), integer("1"), integer("3")]);
}

/// §4.2.1: "the list of nodes in input in the same order but with duplicates
/// eliminated (the first occurrences of each node shall be kept, the others
/// removed). Nodes are compared using term equality". So `4 2 4 1 2` is `4 2 1`
/// — not the sorted `1 2 4` — and `"04"^^xsd:integer` is a different term from
/// `4`, so it survives beside it.
#[test]
fn distinct_keeps_first_occurrences_by_term_equality() {
    let shapes =
        |list: &str| format!("ex:S a sh:NodeShape ; sh:expression [ shnex:distinct {list} ] .");
    assert_eq!(
        outputs("", &shapes("( 4 2 4 1 2 )"), "a"),
        vec![integer("4"), integer("2"), integer("1")]
    );
    assert_eq!(
        outputs("", &shapes(r#"( 4 "04"^^xsd:integer 4 )"#), "a"),
        vec![integer("4"), integer("04")]
    );
}

/// `shnex:findFirst` and `shnex:matchAll` take "A well-formed shape", and the
/// authored empty shape `[]` — a blank node that is the subject of no triple — is
/// one: a shape with no constraints, to which every node conforms. It loads, and
/// it answers as that shape does.
///
/// The neighbouring refusal stands: an IRI the shapes graph never describes as a
/// shape is still refused, because an undefined IRI may be a misspelled reference
/// to a real shape, where a `[]` is written in place and can be nothing else.
#[test]
fn find_first_and_match_all_accept_the_empty_shape_but_not_an_undefined_iri() {
    for (kind, expected) in [
        ("shnex:findFirst", vec![integer("3")]),
        ("shnex:matchAll", vec![bool_lit(true)]),
        (
            "shnex:filterShape",
            vec![integer("3"), integer("1"), integer("3")],
        ),
    ] {
        assert_eq!(
            outputs(
                "",
                &format!(
                    "ex:S a sh:NodeShape ; sh:expression [ {kind} [] ; shnex:nodes ( 3 1 3 ) ] ."
                ),
                "a",
            ),
            expected,
            "{kind} over the empty shape"
        );
        let err = load_error(&format!(
            "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
                 sh:expression [ {kind} ex:Undefined ; shnex:nodes ( 3 1 3 ) ] ."
        ));
        assert!(
            err.contains("does not describe as a shape"),
            "{kind} must still refuse an undefined shape IRI, got: {err}"
        );
    }
}

/// SHACL 1.2 Node Expressions §5 passes each argument's node to the SPARQL
/// function, and an argument that produces NO node reaches SPARQL as an unbound
/// variable — which is what makes `sparql:bound` answer `false` and
/// `sparql:coalesce` fall through to its next argument. The neighbours bind the
/// same argument and get the other answer, so the two runs differ only in
/// whether the argument produced a node.
#[test]
fn an_argument_that_produces_no_node_reaches_sparql_as_unbound() {
    let expression = |body: &str| format!("ex:S a sh:NodeShape ; sh:expression {body} .");
    assert_eq!(
        outputs(
            "",
            &expression(r#"[ sparql:bound ( [ shnex:var "absent" ] ) ]"#),
            "a"
        ),
        vec![bool_lit(false)]
    );
    assert_eq!(
        outputs(
            "",
            &expression(r#"[ sparql:bound ( [ shnex:var "focusNode" ] ) ]"#),
            "a"
        ),
        vec![bool_lit(true)]
    );
    assert_eq!(
        outputs(
            "",
            &expression(r#"[ sparql:coalesce ( [ shnex:var "absent" ] "fallback" ) ]"#),
            "a"
        ),
        vec![r#""fallback""#.to_owned()]
    );
    assert_eq!(
        outputs(
            "",
            &expression(r#"[ sparql:coalesce ( "given" "fallback" ) ]"#),
            "a"
        ),
        vec![r#""given""#.to_owned()]
    );
}

/// A SPARQL function is a LIST parameter function (§3.2.2): "An evaluation
/// failure must be produced if there is more than one output node" for an
/// argument. The neighbour passes the same function a one-node argument and gets
/// its value.
#[test]
fn a_sparql_function_argument_producing_two_nodes_is_an_evaluation_failure() {
    let expression =
        |arg: &str| format!("ex:S a sh:NodeShape ; sh:expression [ sparql:abs ( {arg} ) ] .");
    let err = try_outputs("", &expression("( -1 -2 )"), "a")
        .expect_err("a two-node argument to a SPARQL function must fail");
    assert!(
        err.contains("produced 2 nodes") && err.contains("list parameter function"),
        "the failure must say which argument produced how many nodes, got: {err}"
    );
    assert_eq!(outputs("", &expression("( -1 )"), "a"), vec![integer("1")]);
}

/// §6.2 evaluates a custom LIST parameter expression's body "where an evaluation
/// failure is reported when there is more than 1 output node". A NAMED parameter
/// function has no such bound, and a list parameter function whose body produces
/// one node is fine — the two neighbours run the same two-node body and the same
/// function shape respectively.
#[test]
fn a_list_parameter_function_body_producing_two_nodes_is_an_evaluation_failure() {
    let list_fn = |body: &str| {
        format!(
            "ex:pair a sh:ListParameterExpressionFunction ;
               rdfs:subClassOf sh:ListParameterExpression ;
               sh:bodyExpression {body} .
             ex:S a sh:NodeShape ; sh:expression [ ex:pair () ] ."
        )
    };
    let err = try_outputs("", &list_fn("( 1 2 )"), "a")
        .expect_err("a list parameter function producing two nodes must fail");
    assert!(
        err.contains("produced 2 output nodes"),
        "the failure must name the count, got: {err}"
    );
    assert_eq!(outputs("", &list_fn("( 1 )"), "a"), vec![integer("1")]);

    let named = "ex:PairExpression a sh:NamedParameterExpressionFunction ;
           rdfs:subClassOf sh:NamedParameterExpression ;
           sh:parameter [ sh:path ex:pairOf ; sh:keyParameter true ] ;
           sh:bodyExpression [ shnex:concat ( [ shnex:arg ex:pairOf ] [ shnex:arg ex:pairOf ] ) ] .
         ex:S a sh:NodeShape ; sh:expression [ ex:pairOf 7 ] .";
    assert_eq!(outputs("", named, "a"), vec![integer("7"), integer("7")]);
}

/// §4.5.1 makes the class operand a node expression ("A node expression returning
/// the class(es) that the output nodes must be instances of"), and the output is
/// "the distinct nodes that are SHACL instances in the focus graph of each member
/// of types" — so two computed classes sharing an instance yield it once. A
/// computed member that is not an IRI is the evaluation failure the clause names.
#[test]
fn instances_of_takes_a_computed_class_expression() {
    let data = "ex:a ex:kind ex:Cat, ex:Dog .
         ex:tom a ex:Cat .
         ex:rex a ex:Dog .
         ex:both a ex:Cat, ex:Dog .
         ex:b ex:kind \"Cat\" .";
    let shapes = "ex:S a sh:NodeShape ;
         sh:expression [ shnex:instancesOf [ shnex:pathValues ex:kind ] ] .";
    assert_eq!(
        outputs(data, shapes, "a"),
        vec![ex("both"), ex("rex"), ex("tom")]
    );
    let err = try_outputs(data, shapes, "b").expect_err("a literal class must fail");
    assert!(err.contains("not an IRI"), "got: {err}");
    // A literal written as the operand is a well-formed constant expression, so
    // the shapes graph loads — an untaken `shnex:if` branch never evaluates it —
    // and evaluating it is the same §4.5.1 failure.
    let literal = "ex:S a sh:NodeShape ; sh:expression [ shnex:instancesOf \"Cat\" ] .";
    let err = try_outputs(data, literal, "a").expect_err("a literal class must fail");
    assert!(err.contains("not an IRI"), "got: {err}");
    assert_eq!(
        outputs(
            data,
            "ex:S a sh:NodeShape ;
                 sh:expression [ shnex:if false ;
                                 shnex:then [ shnex:instancesOf \"Cat\" ] ;
                                 shnex:else ex:fine ] .",
            "a"
        ),
        vec![ex("fine")],
        "a literal operand in a branch that is never taken is never a failure"
    );
}

/// §7.2 checks each value node against "each output node" of the expression, and
/// a validation result is keyed by the value node and the shape: the node shapes
/// form a set. An order-preserving expression that yields the same shape twice is
/// therefore checked, and reported, once; the neighbour yields two DIFFERENT
/// failing shapes and gets two results.
#[test]
fn node_by_expression_checks_a_repeated_shape_once() {
    let data = "<http://example.org/ns#a> <http://example.org/ns#p> <http://example.org/ns#v> .\n";
    let shapes = |shapes: &str| {
        format!(
            "{PREFIXES}
            ex:S a sh:NodeShape ;
                sh:targetNode ex:a ;
                sh:property [ sh:path ex:p ; sh:nodeByExpression [ shnex:concat {shapes} ] ] .
            ex:NeedsName a sh:NodeShape ; sh:property [ sh:path ex:name ; sh:minCount 1 ] .
            ex:NeedsLabel a sh:NodeShape ; sh:property [ sh:path ex:label ; sh:minCount 1 ] .
            "
        )
    };
    let repeated =
        validate_graphs(data, &shapes("( ex:NeedsName ex:NeedsName )"), None).expect("validates");
    assert_eq!(repeated.results.len(), 1, "one shape, one result");
    let distinct =
        validate_graphs(data, &shapes("( ex:NeedsName ex:NeedsLabel )"), None).expect("validates");
    assert_eq!(distinct.results.len(), 2, "two shapes, two results");
}
