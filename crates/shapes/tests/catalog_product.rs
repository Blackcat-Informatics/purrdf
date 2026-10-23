// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exported models retain their complete computed-reference domain in products.

use std::sync::Arc;

use purrdf_shapes::engine::{PreparedShapes, parse_shapes};
use purrdf_shapes::product::{HostBindings, ShapesProduct, ShapesProfile};
use purrdf_shapes::term::{NamedNode, Term};
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;

#[test]
fn products_preserve_same_named_function_bodies_in_their_original_domains() {
    use purrdf_shapes::expression::{NodeExpr, RecursionGuard, eval_node_expr};
    use purrdf_shapes::shapes::Constraint;

    let mut shapes = parse_shapes(
        "@prefix sh: <http://www.w3.org/ns/shacl#> .
         <http://example.org/Root> a sh:NodeShape .",
        None,
    )
    .unwrap();
    let mut branches = Vec::new();
    for value in ["left", "right"] {
        let domain = parse_shapes(
            &format!(
                "@prefix sh: <http://www.w3.org/ns/shacl#> .
                 <http://example.org/function> a sh:ListParameterExpressionFunction;
                   sh:bodyExpression \"{value}\" ."
            ),
            None,
        )
        .unwrap();
        branches.push(NodeExpr::Owned {
            expr: Box::new(NodeExpr::CustomCall {
                func: NamedNode::from("http://example.org/function"),
                args: Vec::new(),
            }),
            owner: domain.owner,
        });
    }
    shapes.node_shapes[0]
        .constraints
        .push(Constraint::Expression {
            expr: NodeExpr::Union(branches),
            messages: Vec::new(),
            severity: None,
        });
    let prepared = PreparedShapes::new(Arc::new(shapes));
    let bytes = prepared.to_product(&ShapesProfile::CORE).unwrap();
    let restored = ShapesProduct::open(&bytes)
        .unwrap()
        .admit(&ShapesProfile::CORE, &HostBindings::empty())
        .unwrap();
    let dataset = parse_turtle_to_dataset("", None).unwrap();
    let data = purrdf_shapes::data::ShaclData::new(Arc::clone(&dataset), dataset, None);
    let focus = Term::BlankNode("outside".to_owned());
    for model in [&prepared, &restored] {
        let expr = model.shapes().node_shapes[0]
            .constraints
            .iter()
            .find_map(|constraint| {
                if let Constraint::Expression { expr, .. } = constraint {
                    Some(expr)
                } else {
                    None
                }
            })
            .expect("the authored expression remains present beside the original body reference");
        let values = eval_node_expr(&data, &focus, expr, &mut RecursionGuard::default()).unwrap();
        assert_eq!(
            values,
            ["left", "right"].map(|value| Term::Literal(
                purrdf_shapes::term::Literal::new_simple_literal(value)
            ))
        );
    }
    assert_eq!(restored.to_product(&ShapesProfile::CORE).unwrap(), bytes);
}

#[test]
fn a_product_keeps_definitions_reached_only_through_an_exported_root() {
    let mut shapes = parse_shapes(
        "@prefix sh: <http://www.w3.org/ns/shacl#> .
         @prefix ex: <http://example.org/> .
         ex:Root a sh:NodeShape; sh:targetNode ex:a, \"foreign\";
           sh:nodeByExpression ex:Leaf .
         ex:Leaf a sh:NodeShape; sh:nodeKind sh:IRI .",
        None,
    )
    .unwrap();
    let root = Term::NamedNode(NamedNode::new_unchecked("http://example.org/Root"));
    shapes.node_shapes.retain(|shape| shape.id == root);
    assert_eq!(shapes.node_shapes.len(), 1);
    let prepared = PreparedShapes::new(Arc::new(shapes));
    let dataset = parse_turtle_to_dataset("", None).unwrap();
    let original = prepared.bind_dataset(&dataset).unwrap().validate().unwrap();
    assert!(!original.conforms);
    assert_eq!(original.results.len(), 1);

    let bytes = prepared.to_product(&ShapesProfile::CORE).unwrap();
    let restored = ShapesProduct::open(&bytes)
        .unwrap()
        .admit(&ShapesProfile::CORE, &HostBindings::empty())
        .unwrap();
    let report = restored.bind_dataset(&dataset).unwrap().validate().unwrap();
    assert_eq!(report.conforms, original.conforms);
    assert_eq!(report.results.len(), original.results.len());
    assert_eq!(report.results[0].focus_node, original.results[0].focus_node);
    assert_eq!(
        report.results[0].source_shape,
        original.results[0].source_shape
    );
}

#[test]
fn products_keep_same_named_definitions_in_distinct_retained_domains() {
    for flat in [false, true] {
        let mut roots = Vec::new();
        let mut first = None;
        for (root, kind) in [("Left", "IRI"), ("Right", "Literal")] {
            let mut shapes = parse_shapes(
                &format!(
                "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .
             ex:{root} a sh:NodeShape; sh:targetNode ex:a, \"foreign\";
               sh:nodeByExpression ex:Leaf .
             ex:Leaf a sh:NodeShape; sh:nodeKind sh:{kind} ."
            ),
                None,
            )
            .unwrap();
            let id = Term::NamedNode(NamedNode::new_unchecked(format!(
                "http://example.org/{root}"
            )));
            let mut exported = shapes
                .node_shapes
                .iter()
                .find(|shape| shape.id == id)
                .unwrap()
                .clone();
            if flat {
                exported.constraints = vec![purrdf_shapes::shapes::Constraint::Reference(
                    Term::NamedNode(NamedNode::new_unchecked("http://example.org/Leaf")),
                )];
            }
            roots.push(exported);
            if first.is_none() {
                shapes.node_shapes.clear();
                first = Some(shapes);
            }
        }
        let mut shapes = first.unwrap();
        shapes.node_shapes = roots;
        let prepared = PreparedShapes::new(Arc::new(shapes));
        let data = parse_turtle_to_dataset("", None).unwrap();
        let before = prepared.bind_dataset(&data).unwrap().validate().unwrap();
        let locations = |report: &purrdf_shapes::report::ValidationReport| {
            let mut rows: Vec<_> = report
                .results
                .iter()
                .map(|result| {
                    (
                        result.source_shape.to_string(),
                        result.focus_node.to_string(),
                    )
                })
                .collect();
            rows.sort();
            rows
        };
        let expected = vec![
            (
                format!(
                    "<http://example.org/{}>",
                    if flat { "Leaf" } else { "Left" }
                ),
                "\"foreign\"".to_owned(),
            ),
            (
                format!(
                    "<http://example.org/{}>",
                    if flat { "Leaf" } else { "Right" }
                ),
                "<http://example.org/a>".to_owned(),
            ),
        ];
        assert_eq!(locations(&before), expected);
        let bytes = prepared.to_product(&ShapesProfile::CORE).unwrap();
        let restored = ShapesProduct::open(&bytes)
            .unwrap()
            .admit(&ShapesProfile::CORE, &HostBindings::empty())
            .unwrap();
        let after = restored.bind_dataset(&data).unwrap().validate().unwrap();
        assert_eq!(locations(&after), expected);
        assert_eq!(restored.to_product(&ShapesProfile::CORE).unwrap(), bytes);
    }
}

#[test]
fn products_keep_declarations_when_no_shape_root_is_exported() {
    use purrdf_shapes::expression::{RecursionGuard, eval_custom_function_call};
    let mut shapes = parse_shapes(
        "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .
         ex:f a sh:ListParameterExpressionFunction; sh:bodyExpression \"retained\" .",
        None,
    )
    .unwrap();
    shapes.node_shapes.clear();
    let prepared = PreparedShapes::new(Arc::new(shapes));
    let bytes = prepared.to_product(&ShapesProfile::CORE).unwrap();
    let restored = ShapesProduct::open(&bytes)
        .unwrap()
        .admit(&ShapesProfile::CORE, &HostBindings::empty())
        .unwrap();
    let iri = NamedNode::new_unchecked("http://example.org/f");
    let dataset = parse_turtle_to_dataset("", None).unwrap();
    let data = purrdf_shapes::data::ShaclData::new(Arc::clone(&dataset), dataset, None);
    for model in [&prepared, &restored] {
        assert!(model.shapes().node_shapes.is_empty());
        assert!(
            model
                .shapes()
                .functions
                .resolve_expr(iri.as_str())
                .is_some()
        );
        let owner = &model.shapes().owner;
        let function = owner.function(&iri).unwrap();
        assert_eq!(
            eval_custom_function_call(&data, owner, function, &[], &mut RecursionGuard::default())
                .unwrap(),
            Some(Term::Literal(
                purrdf_shapes::term::Literal::new_simple_literal("retained")
            ))
        );
    }
    assert_eq!(restored.to_product(&ShapesProfile::CORE).unwrap(), bytes);
}

#[test]
fn product_identity_changes_with_the_exported_root_set() {
    let mut shapes = parse_shapes(
        "@prefix sh: <http://www.w3.org/ns/shacl#> .
         <http://example.org/Root> a sh:NodeShape; sh:targetNode \"foreign\"; sh:nodeKind sh:IRI .",
        None,
    )
    .unwrap();
    let original = PreparedShapes::new(Arc::new(shapes.clone()));
    shapes.node_shapes.clear();
    let empty = PreparedShapes::new(Arc::new(shapes));
    let dataset = parse_turtle_to_dataset("", None).unwrap();
    assert!(
        !original
            .bind_dataset(&dataset)
            .unwrap()
            .validate()
            .unwrap()
            .conforms
    );
    assert!(
        empty
            .bind_dataset(&dataset)
            .unwrap()
            .validate()
            .unwrap()
            .conforms
    );
    let first = original.to_product(&ShapesProfile::CORE).unwrap();
    let second = empty.to_product(&ShapesProfile::CORE).unwrap();
    assert_ne!(
        ShapesProduct::open(&first)
            .unwrap()
            .declared_identity()
            .digest(),
        ShapesProduct::open(&second)
            .unwrap()
            .declared_identity()
            .digest(),
        "distinct validators over the same retained source must have distinct product cache identities"
    );
}
