// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The three ways a host names the node expression [`eval_node_expr_to_terms`] evaluates
//! — the node itself, a walk of predicates from a named node, and an inline Turtle
//! expression — each beside its refusals, and each refusal beside a valid neighbour that
//! differs from it in the one respect the refusal is about.

use purrdf_validate::{
    ExprSelector, ExprSelectorError, NodeExprRequest, ShapesError, eval_node_expr_to_terms,
};

/// A property shape computing its values with an anonymous expression, a node carrying two
/// anonymous expressions under one predicate, a shape an inline expression references,
/// and two labelled blank nodes an inline expression's own labels must not land on.
const SHAPES: &str = r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix shnex: <http://www.w3.org/ns/shacl-node-expr#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
@prefix ex: <http://example.org/ns#> .

ex:Person a sh:NodeShape ; sh:property ex:Label .
ex:Label sh:path ex:label ; sh:values [ sh:path ex:name ] .
ex:Twice ex:computes [ sh:path ex:name ] , [ sh:path ex:age ] .
ex:Once ex:computes [ sh:path ex:age ] .
ex:Adult a sh:NodeShape ; sh:property [ sh:path ex:age ; sh:minInclusive 18 ] .
_:e shnex:var "wrong" .
_:expre shnex:var "wrong" .
"#;

const DATA: &str = "<http://example.org/ns#a> <http://example.org/ns#name> \"Ada\" .\n\
<http://example.org/ns#a> <http://example.org/ns#age> \"36\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n\
<http://example.org/ns#a> <http://example.org/ns#knows> <http://example.org/ns#b> .\n\
<http://example.org/ns#a> <http://example.org/ns#knows> <http://example.org/ns#c> .\n\
<http://example.org/ns#b> <http://example.org/ns#age> \"12\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n\
<http://example.org/ns#c> <http://example.org/ns#age> \"40\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n";

const SH_VALUES: &str = "http://www.w3.org/ns/shacl#values";
const COMPUTES: &str = "http://example.org/ns#computes";

fn eval(expr: ExprSelector<'_>) -> Result<Vec<String>, ShapesError> {
    eval_scoped(expr, &[])
}

fn eval_scoped(expr: ExprSelector<'_>, scope: &[(&str, &str)]) -> Result<Vec<String>, ShapesError> {
    eval_node_expr_to_terms(&NodeExprRequest {
        shapes_ttl: SHAPES,
        shapes_base: None,
        data_nt: DATA,
        expr,
        focus: "http://example.org/ns#a",
        scope,
        imports: &[],
    })
}

fn message(error: &ShapesError) -> String {
    error.to_string()
}

/// A walk from a named node reaches the anonymous expression it carries; a step that
/// reaches two values, or none, is refused naming the count, beside the one-value
/// neighbour under the same predicate.
#[test]
fn a_walk_names_an_anonymous_expression() {
    assert_eq!(
        eval(ExprSelector::At {
            node: "http://example.org/ns#Label",
            via: &[SH_VALUES],
        })
        .expect("evaluates"),
        ["\"Ada\""]
    );
    assert_eq!(
        eval(ExprSelector::At {
            node: "http://example.org/ns#Once",
            via: &[COMPUTES],
        })
        .expect("the one-value neighbour evaluates"),
        ["\"36\"^^<http://www.w3.org/2001/XMLSchema#integer>"]
    );
    let two = eval(ExprSelector::At {
        node: "http://example.org/ns#Twice",
        via: &[COMPUTES],
    })
    .expect_err("two values are refused");
    assert!(
        message(&two).contains("step 1, <http://example.org/ns#Twice> <http://example.org/ns#computes>, reaches 2 values"),
        "{two}"
    );
    let none = eval(ExprSelector::At {
        node: "http://example.org/ns#Label",
        via: &[SH_VALUES, SH_VALUES],
    })
    .expect_err("a second step from the expression reaches nothing");
    assert!(message(&none).contains("step 2"), "{none}");
    assert!(message(&none).contains("reaches 0 values"), "{none}");
}

/// The selector's argv-level refusals, each decided before a document is read, beside
/// the valid spelling.
#[test]
fn a_walk_is_decided_before_any_document() {
    let parse = |selector: ExprSelector<'_>| selector.parse().map(|_| ());
    assert_eq!(
        parse(ExprSelector::At {
            node: "http://example.org/ns#Label",
            via: &[],
        }),
        Err(ExprSelectorError::EmptyPath)
    );
    assert!(matches!(
        parse(ExprSelector::At {
            node: "\"x\"",
            via: &[SH_VALUES],
        }),
        Err(ExprSelectorError::StartNotANode { .. })
    ));
    assert!(matches!(
        parse(ExprSelector::At {
            node: "http://example.org/ns#Label",
            via: &["_:p"],
        }),
        Err(ExprSelectorError::PredicateNotIri { step: 1, .. })
    ));
    assert!(matches!(
        parse(ExprSelector::At {
            node: "http://example.org/ns#Label",
            via: &["values"],
        }),
        Err(ExprSelectorError::Term { .. })
    ));
    assert_eq!(
        parse(ExprSelector::At {
            node: "_:e",
            via: &[SH_VALUES],
        }),
        Ok(())
    );
}

/// An inline expression is read under the shapes document's prefixes and merged into the
/// shapes graph, so its shape reference resolves there.
#[test]
fn an_inline_expression_binds_into_the_shapes_graph() {
    assert_eq!(
        eval(ExprSelector::Turtle("[ sh:path ex:name ] .")).expect("evaluates"),
        ["\"Ada\""]
    );
    assert_eq!(
        eval(ExprSelector::Turtle(
            "[ shnex:filterShape ex:Adult ; shnex:nodes [ sh:path ex:knows ] ] ."
        ))
        .expect("the shape reference resolves in the shapes graph"),
        ["<http://example.org/ns#c>"]
    );
    // The fragment's own directive outranks the shapes document's.
    assert_eq!(
        eval(ExprSelector::Turtle(
            "@prefix ex: <http://example.org/other#> .\n[ sh:path ex:name ] ."
        ))
        .expect("evaluates"),
        Vec::<String>::new()
    );
}

/// The fragment's blank nodes are standardized apart from the shapes document's: its
/// `_:e` is neither the shapes graph's `_:e` nor its `_:expre`.
#[test]
fn an_inline_expression_is_standardized_apart() {
    let scope = [("right", "\"R\""), ("wrong", "\"W\"")];
    assert_eq!(
        eval_scoped(ExprSelector::Turtle("_:e shnex:var \"right\" ."), &scope).expect("evaluates"),
        ["\"R\""]
    );
    assert_eq!(
        eval_scoped(ExprSelector::Node("_:e"), &scope).expect("the shapes graph's _:e"),
        ["\"W\""]
    );
}

/// No root, or two, is refused naming the count, beside the one-root neighbour; a
/// fragment that is not Turtle is refused as one.
#[test]
fn an_inline_expression_has_exactly_one_root() {
    let two = eval(ExprSelector::Turtle(
        "[ sh:path ex:name ] . [ sh:path ex:age ] .",
    ))
    .expect_err("two roots");
    assert!(message(&two).contains("has 2 root blank nodes"), "{two}");
    let none = eval(ExprSelector::Turtle("ex:E sh:path ex:name .")).expect_err("no root");
    assert!(message(&none).contains("has 0 root blank nodes"), "{none}");
    assert_eq!(
        eval(ExprSelector::Turtle(
            "[ sh:path ex:name ] . ex:E sh:path ex:age ."
        ))
        .expect("an IRI subject beside the root is not a root"),
        ["\"Ada\""]
    );
    let syntax = eval(ExprSelector::Turtle("[ sh:path ex:name ]")).expect_err("no `.`");
    assert!(
        message(&syntax).contains("not a Turtle document"),
        "{syntax}"
    );
}

/// The node form is unchanged: an IRI names a constant expression.
#[test]
fn the_node_form_still_names_the_node() {
    assert_eq!(
        eval(ExprSelector::Node("http://example.org/ns#Nowhere")).expect("constant"),
        ["<http://example.org/ns#Nowhere>"]
    );
}

/// A host's optional arguments name exactly one selector: none, two, and predicates with
/// no walk start are refused, beside each single spelling.
#[test]
fn a_host_names_exactly_one_selector() {
    let via = [SH_VALUES];
    assert_eq!(
        ExprSelector::from_parts(None, None, &[], None),
        Err(ExprSelectorError::NotOneSelector { count: 0 })
    );
    assert_eq!(
        ExprSelector::from_parts(Some("_:e"), None, &[], Some("[] .")),
        Err(ExprSelectorError::NotOneSelector { count: 2 })
    );
    assert_eq!(
        ExprSelector::from_parts(None, None, &via, Some("[] .")),
        Err(ExprSelectorError::ViaWithoutAt)
    );
    assert_eq!(
        ExprSelector::from_parts(Some("_:e"), None, &[], None),
        Ok(ExprSelector::Node("_:e"))
    );
    assert_eq!(
        ExprSelector::from_parts(None, Some("_:e"), &via, None),
        Ok(ExprSelector::At {
            node: "_:e",
            via: &via
        })
    );
    assert_eq!(
        ExprSelector::from_parts(None, None, &[], Some("[] .")),
        Ok(ExprSelector::Turtle("[] ."))
    );
}
