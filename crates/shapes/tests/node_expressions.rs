// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

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
@prefix xsd:   <http://www.w3.org/2001/XMLSchema#> .
";

/// Parse `shapes_ttl` and return the single `sh:expression` node expression it
/// declares — the production parse path, not a test-only constructor.
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

/// Evaluate the fixture's `sh:expression` over `data_ttl` from focus node
/// `ex:<focus>`, returning the output nodes in the order the evaluator produced
/// them (rendered canonically so ordering assertions are exact).
fn outputs(data_ttl: &str, shapes_ttl: &str, focus: &str) -> Vec<String> {
    let expr = expression_of(shapes_ttl);
    let data: Arc<_> =
        parse_turtle_to_dataset(&format!("{PREFIXES}{data_ttl}")).expect("data parse");
    let shapes_ds: Arc<_> =
        parse_turtle_to_dataset(&format!("{PREFIXES}{shapes_ttl}")).expect("shapes data parse");
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
    let data: Arc<_> = parse_turtle_to_dataset(PREFIXES).expect("data parse");
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
    let data: Arc<_> =
        parse_turtle_to_dataset(&format!("{PREFIXES} ex:a ex:says <<( ex:s ex:p ex:o )>> ."))
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
    let report = validate_graphs(data, &shapes).expect("validation runs");
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
    let data: Arc<_> =
        parse_turtle_to_dataset(&format!("{PREFIXES} ex:a ex:p ex:b, ex:c .")).expect("data parse");
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
    let report = validate_graphs(data, &shapes).expect("validation runs");
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
    let data = "ex:a ex:p ex:b, ex:c, ex:d . ex:b ex:ok true . ex:c ex:ok true .";
    let pairs: [(&str, &str); 6] = [
        ("[ sh:path ex:p ]", "[ shnex:pathValues ex:p ]"),
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

/// A node carrying BOTH the `sh:` and the `shnex:` spelling of the same kind is
/// AMBIGUOUS and hard-fails — the writer is asked which they meant, never silently
/// given one of them.
#[test]
fn both_spellings_of_one_kind_on_one_node_is_ambiguous() {
    let err = parse_shapes(&format!(
        "{PREFIXES}
         ex:S a sh:NodeShape ;
             sh:expression [ sh:count [ sh:path ex:p ] ; shnex:count [ shnex:pathValues ex:p ] ] ."
    ))
    .expect_err("two spellings of one kind must hard-fail");
    assert!(err.contains("ambiguous node expression"), "got: {err}");
}

/// Two DIFFERENT kinds on one node stay ambiguous too — the pre-existing guard is
/// untouched by the dual-spelling table.
#[test]
fn two_different_kinds_on_one_node_is_ambiguous() {
    let err = parse_shapes(&format!(
        "{PREFIXES}
         ex:S a sh:NodeShape ;
             sh:expression [ sh:count [ sh:path ex:p ] ; shnex:sum [ shnex:pathValues ex:p ] ] ."
    ))
    .expect_err("two kinds on one node must hard-fail");
    assert!(err.contains("ambiguous node expression"), "got: {err}");
}

/// The canonical `xsd:boolean` rendering the evaluator emits.
fn bool_lit(b: bool) -> String {
    let lexical = if b { "true" } else { "false" };
    format!(r#""{lexical}"^^<http://www.w3.org/2001/XMLSchema#boolean>"#)
}
