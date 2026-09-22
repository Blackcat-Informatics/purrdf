// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Tests for the post-tree linking pass.
//!
//! Every refusal here is executed BESIDE its valid neighbour. A refusal is a claim,
//! and a topology check is exactly the kind of claim that hides an over-refusal:
//! tightening `Arc::ptr_eq` until it rejects a legitimately shared handle would look
//! like correct strictness and would reject every real shapes graph.

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use purrdf::{RdfDataset, SparqlRequest, SparqlResult};
use purrdf_sparql_eval::{NativeSparqlEngine, QueryOptions, UserFunctionRegistry};

use super::{ShapeIndex, link_shapes};
use crate::engine::parse_shapes;
use crate::expression::{ArgKey, CustomFnKind, CustomFunction, NodeExpr, ShapeArg};
use crate::product::ProductDimension;
use crate::report::Severity;
use crate::shapes::{Constraint, Shape};
use crate::term::{NamedNode, Term};

// ── Fixture vocabulary (example.org, per the repository's fixture rule) ─────────

/// The prefix header every Turtle fixture below is parsed with.
const PREFIXES: &str = r"
@prefix ex:     <http://example.org/ns#> .
@prefix sh:     <http://www.w3.org/ns/shacl#> .
@prefix shnex:  <http://www.w3.org/ns/shacl-node-expr#> .
@prefix sparql: <http://www.w3.org/ns/sparql#> .
";

/// An `example.org` IRI.
fn ex(local: &str) -> NamedNode {
    NamedNode::new_unchecked(format!("http://example.org/ns#{local}"))
}

/// A node shape with nothing on it but an identity.
fn leaf_shape(id: &str) -> Shape {
    Shape {
        id: Term::NamedNode(ex(id)),
        targets: Vec::new(),
        constraints: Vec::new(),
        property_shapes: Vec::new(),
        severity: Severity::Violation,
        message: None,
        deactivated: false,
        box_roles: Vec::new(),
        rules: Vec::new(),
    }
}

/// A `sh:nodeByExpression` constraint carrying `index` as its resolution table.
fn node_by_expression(index: &ShapeIndex) -> Constraint {
    Constraint::NodeByExpression {
        expr: NodeExpr::This,
        shapes: Arc::clone(index),
        message: None,
        severity: None,
    }
}

/// A computed `shnex:conformsToShape` argument carrying `index`.
fn conforms_to_computed(index: &ShapeIndex) -> Constraint {
    Constraint::Expression {
        expr: NodeExpr::ConformsToShape {
            node: Box::new(NodeExpr::This),
            shape: ShapeArg::Computed {
                expr: Box::new(NodeExpr::This),
                shapes: Arc::clone(index),
            },
        },
        message: None,
        severity: None,
    }
}

/// A list-parameter declaration with one argument and no body installed.
fn declaration() -> Arc<CustomFunction> {
    Arc::new(CustomFunction {
        iri: ex("identity"),
        kind: CustomFnKind::ListParameter,
        params: vec![ArgKey::Index(0)],
        required: 1,
        body: OnceLock::new(),
    })
}

/// Every shape-index handle a parsed shapes graph's top-level constraints carry.
fn handles_of(shapes: &crate::shapes::Shapes) -> Vec<ShapeIndex> {
    let mut found = Vec::new();
    for shape in &shapes.node_shapes {
        for constraint in &shape.constraints {
            if let Constraint::NodeByExpression { shapes, .. } = constraint {
                found.push(Arc::clone(shapes));
            }
        }
    }
    found
}

/// Parse a data graph fixture.
fn data_of(data_ttl: &str) -> Arc<RdfDataset> {
    crate::text_ingest::parse_turtle_to_dataset(&format!("{PREFIXES}{data_ttl}"), None)
        .expect("data parse")
}

// ── The shared index is ONE allocation ─────────────────────────────────────────

/// Three `sh:nodeByExpression` constraints, one index.
///
/// This is the property no downstream test can see: a per-constraint
/// `Arc::new(OnceLock::new())` type-checks, costs one index per constraint, and
/// leaves every clone but one permanently empty — so the constraints resolve no
/// shape at all and the shapes graph validates green while checking nothing.
#[test]
fn shared_shape_index_is_one_allocation() {
    let shapes = parse_shapes(
        &format!(
            "{PREFIXES}
            ex:T a sh:NodeShape ; sh:targetNode ex:t .
            ex:A a sh:NodeShape ; sh:targetNode ex:a ; sh:nodeByExpression ex:T .
            ex:B a sh:NodeShape ; sh:targetNode ex:b ; sh:nodeByExpression ex:T .
            ex:C a sh:NodeShape ; sh:targetNode ex:c ; sh:nodeByExpression ex:T ."
        ),
        None,
    )
    .expect("shapes parse");

    let handles = handles_of(&shapes);
    assert_eq!(
        handles.len(),
        3,
        "the fixture declares three sh:nodeByExpression constraints",
    );
    for handle in &handles {
        assert!(
            Arc::ptr_eq(handle, &handles[0]),
            "every constraint must share ONE index allocation",
        );
        let filled = handle.get().expect("linking fills the shared index");
        assert!(
            filled.contains_key(&Term::NamedNode(ex("T"))),
            "the one write must reach every constraint",
        );
    }
}

// ── The shape-index topology refusal, and its valid neighbour ──────────────────

/// INVALID: one constraint holds a handle of its own.
#[test]
fn link_refuses_unshared_index() {
    let index: ShapeIndex = Arc::new(OnceLock::new());
    let stray: ShapeIndex = Arc::new(OnceLock::new());
    let mut shape = leaf_shape("A");
    shape.constraints = vec![node_by_expression(&index), node_by_expression(&stray)];

    let mut registry = UserFunctionRegistry::new();
    let error = link_shapes(&[shape], &index, &[], BTreeMap::new(), &mut registry)
        .expect_err("a private handle must be refused");

    assert_eq!(error.dimension(), ProductDimension::Malformed);
    assert!(
        error.message().contains("NOT the one handle"),
        "the refusal must name the topology, got {error}",
    );
    assert!(
        stray.get().is_none(),
        "the stray handle is exactly the one the fill never reaches",
    );
}

/// VALID NEIGHBOUR: the same two constraints, both sharing the one handle — plus a
/// computed shape argument, the second site that carries one.
#[test]
fn link_accepts_correctly_shared_index() {
    let index: ShapeIndex = Arc::new(OnceLock::new());
    let mut shape = leaf_shape("A");
    shape.constraints = vec![
        node_by_expression(&index),
        node_by_expression(&index),
        conforms_to_computed(&index),
    ];

    let mut registry = UserFunctionRegistry::new();
    link_shapes(
        std::slice::from_ref(&shape),
        &index,
        &[],
        BTreeMap::new(),
        &mut registry,
    )
    .expect("a correctly shared index must link");

    let filled = index.get().expect("the shared index is filled");
    assert!(filled.contains_key(&Term::NamedNode(ex("A"))));
}

/// VALID NEIGHBOUR: a graph with NO shape-index site at all still links, and the
/// index is left unbuilt because nothing will ever read it.
#[test]
fn link_accepts_a_graph_with_no_shape_index_site() {
    let index: ShapeIndex = Arc::new(OnceLock::new());
    let mut registry = UserFunctionRegistry::new();
    link_shapes(
        &[leaf_shape("A")],
        &index,
        &[],
        BTreeMap::new(),
        &mut registry,
    )
    .expect("a graph with no sh:nodeByExpression must link");

    assert!(
        index.get().is_none(),
        "nothing holds the handle, so the shape clones are never built",
    );
}

// ── The unfilled-body refusal, and its valid neighbour ─────────────────────────

/// INVALID: a declaration reaches the end of linking with an empty body cell.
#[test]
fn link_refuses_unfilled_custom_function_body() {
    let index: ShapeIndex = Arc::new(OnceLock::new());
    let func = declaration();
    let mut registry = UserFunctionRegistry::new();

    let error = link_shapes(
        &[leaf_shape("A")],
        &index,
        std::slice::from_ref(&func),
        BTreeMap::new(),
        &mut registry,
    )
    .expect_err("a body-less declaration must be refused");

    assert_eq!(error.dimension(), ProductDimension::Malformed);
    assert!(
        error.message().contains("installed sh:bodyExpression"),
        "the refusal must name the missing body, got {error}",
    );
    assert!(
        registry.resolve_expr(ex("identity").as_str()).is_none(),
        "a function with no body must never reach the registry",
    );
}

/// VALID NEIGHBOUR: the same declaration, with its body supplied.
#[test]
fn link_accepts_a_supplied_custom_function_body() {
    let index: ShapeIndex = Arc::new(OnceLock::new());
    let func = declaration();
    let mut bodies = BTreeMap::new();
    bodies.insert(
        func.iri.as_str().to_owned(),
        NodeExpr::Arg(ArgKey::Index(0)),
    );
    let mut registry = UserFunctionRegistry::new();

    link_shapes(
        &[leaf_shape("A")],
        &index,
        std::slice::from_ref(&func),
        bodies,
        &mut registry,
    )
    .expect("a supplied body must link");

    assert!(func.body.get().is_some(), "the body cell is filled");
    assert!(
        registry.resolve_expr(ex("identity").as_str()).is_some(),
        "§7.3 registers every list-parameter declaration",
    );
}

/// VALID NEIGHBOUR: a body ALREADY installed — the decoder's case, where the byte
/// stream's own two-pass layout must fill the cell before the shapes that call the
/// function can be read at all. Supplying no body for it must not be refused.
#[test]
fn link_accepts_a_body_installed_before_linking() {
    let index: ShapeIndex = Arc::new(OnceLock::new());
    let func = declaration();
    func.body
        .set(NodeExpr::CustomCall {
            func: Arc::clone(&func),
            args: vec![(ArgKey::Index(0), NodeExpr::Arg(ArgKey::Index(0)))],
        })
        .expect("the fixture installs once");

    let mut registry = UserFunctionRegistry::new();
    link_shapes(
        &[leaf_shape("A")],
        &index,
        std::slice::from_ref(&func),
        BTreeMap::new(),
        &mut registry,
    )
    .expect("an already-installed self-recursive body must link");

    assert!(
        registry.resolve_expr(ex("identity").as_str()).is_some(),
        "the declaration is registered",
    );
}

/// INVALID: a body arrives for a function this model does not declare.
#[test]
fn link_refuses_a_body_with_no_declaration() {
    let index: ShapeIndex = Arc::new(OnceLock::new());
    let mut bodies = BTreeMap::new();
    bodies.insert(ex("ghost").as_str().to_owned(), NodeExpr::This);
    let mut registry = UserFunctionRegistry::new();

    let error = link_shapes(&[leaf_shape("A")], &index, &[], bodies, &mut registry)
        .expect_err("an orphan body must be refused");
    assert_eq!(error.dimension(), ProductDimension::Malformed);
    assert!(error.message().contains("ghost"), "got {error}");
}

// ── Recursion survives linking ─────────────────────────────────────

/// A self-recursive `sh:bodyExpression` links AND evaluates.
///
/// `ex:countdown(n)` answers `n` itself when `n` is not positive, and otherwise
/// calls ITSELF with `0`. The call site inside the body therefore has to resolve to
/// the very declaration the body belongs to — a cycle in the value graph — and
/// `ex:countdown(3)` can only answer `0` if that cycle was linked: the body must have
/// been installed into the INTERNED declaration rather than into a copy, and the
/// §7.3 registration must have closed over that same handle. An unlinked body
/// answers nothing at all, and an unshared one answers `3`.
///
/// The recursive call passes a CONSTANT rather than a decremented argument on
/// purpose: `NodeExpr::Arg` evaluates a bound argument expression under
/// `Scope::EMPTY`, so an argument that referred to `shnex:arg 0` would read the empty
/// scope and produce nothing. That is a property of the evaluator, not of this
/// fixture — passing `[ sparql:subtract ( [ shnex:arg 0 ] 1 ) ]` here looks more
/// natural and silently evaluates to no value.
#[test]
fn linked_custom_function_recursion_resolves() {
    let shapes = parse_shapes(
        &format!(
            "{PREFIXES}
            ex:countdown a sh:ListParameterExpressionFunction ;
              sh:bodyExpression [
                  sh:if   [ sparql:greater-than ( [ shnex:arg 0 ] 0 ) ] ;
                  sh:then [ ex:countdown ( 0 ) ] ;
                  sh:else [ shnex:arg 0 ]
              ] ;
              sh:parameter [ sh:path shnex:arg0 ] ."
        ),
        None,
    )
    .expect("shapes parse");

    let data = data_of("ex:a a ex:Thing .");
    let engine = NativeSparqlEngine::new();
    let result = engine
        .query_with_options_view(
            &data,
            SparqlRequest {
                query: "PREFIX ex: <http://example.org/ns#> \
                        SELECT ?n WHERE { BIND (ex:countdown(3) AS ?n) }",
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions {
                functions: &crate::sparql::bind_in_current_env(&shapes.functions)
                    .expect("the fixture's bodies parse and admit"),
                focus_graph: Some(&data),
                ..QueryOptions::EMPTY
            },
        )
        .expect("the registered recursive function resolves");

    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("expected solutions");
    };
    let bound: Vec<String> = rows
        .iter()
        .filter_map(|row| row.first().and_then(Option::as_ref))
        .map(|value| format!("{value:?}"))
        .collect();
    assert_eq!(bound.len(), 1, "one call, one answer: {bound:?}");
    assert!(
        bound[0].contains("lexical_form: \"0\""),
        "the recursive branch must have been taken — an unrecursed body answers 3: {bound:?}",
    );
}
