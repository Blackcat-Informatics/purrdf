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
    let bytes = prepared
        .to_product(
            &ShapesProfile::CORE,
            &mut purrdf_core::CanonicalizationControl::new(
                purrdf_core::RDFC_V2_CORPUS_POLICY,
                None,
            ),
        )
        .unwrap();
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
    assert_eq!(
        restored
            .to_product(
                &ShapesProfile::CORE,
                &mut purrdf_core::CanonicalizationControl::new(
                    purrdf_core::RDFC_V2_CORPUS_POLICY,
                    None
                )
            )
            .unwrap(),
        bytes
    );
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

    let bytes = prepared
        .to_product(
            &ShapesProfile::CORE,
            &mut purrdf_core::CanonicalizationControl::new(
                purrdf_core::RDFC_V2_CORPUS_POLICY,
                None,
            ),
        )
        .unwrap();
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
        let bytes = prepared
            .to_product(
                &ShapesProfile::CORE,
                &mut purrdf_core::CanonicalizationControl::new(
                    purrdf_core::RDFC_V2_CORPUS_POLICY,
                    None,
                ),
            )
            .unwrap();
        let restored = ShapesProduct::open(&bytes)
            .unwrap()
            .admit(&ShapesProfile::CORE, &HostBindings::empty())
            .unwrap();
        let after = restored.bind_dataset(&data).unwrap().validate().unwrap();
        assert_eq!(locations(&after), expected);
        assert_eq!(
            restored
                .to_product(
                    &ShapesProfile::CORE,
                    &mut purrdf_core::CanonicalizationControl::new(
                        purrdf_core::RDFC_V2_CORPUS_POLICY,
                        None
                    )
                )
                .unwrap(),
            bytes
        );
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
    let bytes = prepared
        .to_product(
            &ShapesProfile::CORE,
            &mut purrdf_core::CanonicalizationControl::new(
                purrdf_core::RDFC_V2_CORPUS_POLICY,
                None,
            ),
        )
        .unwrap();
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
    assert_eq!(
        restored
            .to_product(
                &ShapesProfile::CORE,
                &mut purrdf_core::CanonicalizationControl::new(
                    purrdf_core::RDFC_V2_CORPUS_POLICY,
                    None
                )
            )
            .unwrap(),
        bytes
    );
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
    let first = original
        .to_product(
            &ShapesProfile::CORE,
            &mut purrdf_core::CanonicalizationControl::new(
                purrdf_core::RDFC_V2_CORPUS_POLICY,
                None,
            ),
        )
        .unwrap();
    let second = empty
        .to_product(
            &ShapesProfile::CORE,
            &mut purrdf_core::CanonicalizationControl::new(
                purrdf_core::RDFC_V2_CORPUS_POLICY,
                None,
            ),
        )
        .unwrap();
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

/// Static where and conformance operands retain each original owner domain.
#[test]
fn where_products_keep_blank_operands_in_their_original_domains() {
    use purrdf_shapes::engine::{ValidationOptions, parse_shapes_with_options};
    use purrdf_shapes::recursion::{RecursionOptions, RecursionSemantics};
    for conformance in [false, true] {
        for semantics in [RecursionSemantics::Lfp, RecursionSemantics::Gfp] {
            let request = ValidationOptions::default().with_recursion(RecursionOptions {
                semantics,
                max_pairs: 4_096,
                max_iterations: 4_096,
            });
            let mut first = None;
            let mut roots = Vec::new();
            let mut operands = Vec::new();
            for (root, kind) in [("Left", "IRI"), ("Right", "Literal")] {
                let constraint = if conformance {
                    "sh:targetNode ex:a, \"value\" ; sh:expression [ shnex:conformsToShape ( sh:this _:same ) ]"
                } else {
                    "sh:targetWhere _:same ; sh:in ()"
                };
                let mut shapes = parse_shapes_with_options(
                &format!("@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> . @prefix shnex: <http://www.w3.org/ns/shacl-node-expr#> .
                    ex:{root} a sh:NodeShape ; {constraint} .
                    _:same a sh:NodeShape ; sh:nodeKind sh:{kind} ."),
                None, None, None, &purrdf_shapes::ShapesImports::new(), &request,
            ).unwrap();
                let id = NamedNode::new_unchecked(format!("http://example.org/{root}")).into_term();
                let original = shapes.owner.definition(&id).unwrap();
                let operand = if conformance {
                    let purrdf_shapes::shapes::Constraint::Expression {
                        expr:
                            purrdf_shapes::expression::NodeExpr::ConformsToShape {
                                shape: purrdf_shapes::expression::ShapeArg::Named(shape),
                                ..
                            },
                        ..
                    } = &original.constraints[0]
                    else {
                        unreachable!()
                    };
                    &shape.id
                } else {
                    let purrdf_shapes::shapes::Target::Where(shape) = &original.targets[0] else {
                        unreachable!()
                    };
                    &shape.id
                };
                operands.push(operand.clone());
                roots.push(shapes.owner.root(&id).unwrap());
                if first.is_none() {
                    shapes.node_shapes.clear();
                    first = Some(shapes);
                }
            }
            assert_eq!(
                operands[0], operands[1],
                "the owners must share a native blank spelling"
            );
            let mut shapes = first.unwrap();
            shapes.node_shapes = roots;
            let prepared = PreparedShapes::new(Arc::new(shapes));
            let bytes = prepared
                .to_product(
                    &ShapesProfile::CORE,
                    &mut purrdf_core::CanonicalizationControl::new(
                        purrdf_core::RDFC_V2_CORPUS_POLICY,
                        None,
                    ),
                )
                .unwrap();
            let restored = ShapesProduct::open(&bytes)
                .unwrap()
                .admit_with_options(&ShapesProfile::CORE, &HostBindings::empty(), &request)
                .unwrap();
            let data = parse_turtle_to_dataset(
                "<http://example.org/a> <http://example.org/p> \"value\" .",
                None,
            )
            .unwrap();
            for model in [&prepared, &restored] {
                let report = model.bind_dataset(&data).unwrap().validate().unwrap();
                let mut rows: Vec<_> = report
                    .results
                    .iter()
                    .map(|r| (r.source_shape.to_string(), r.focus_node.to_string()))
                    .collect();
                rows.sort();
                assert_eq!(
                    rows,
                    vec![
                        (
                            "<http://example.org/Left>".to_owned(),
                            if conformance {
                                "\"value\""
                            } else {
                                "<http://example.org/a>"
                            }
                            .to_owned()
                        ),
                        (
                            "<http://example.org/Right>".to_owned(),
                            if conformance {
                                "<http://example.org/a>"
                            } else {
                                "\"value\""
                            }
                            .to_owned()
                        ),
                    ]
                );
            }
            assert_eq!(
                restored
                    .to_product(
                        &ShapesProfile::CORE,
                        &mut purrdf_core::CanonicalizationControl::new(
                            purrdf_core::RDFC_V2_CORPUS_POLICY,
                            None
                        )
                    )
                    .unwrap(),
                bytes
            );
        }
    }
}

/// Static operand RDF scopes survive product admission and source rebuild.
#[test]
fn where_products_keep_equal_blank_labels_in_distinct_rdf_scopes() {
    use purrdf_rdf::{BlankScope, RdfDatasetBuilder};
    use purrdf_shapes::engine::ValidationOptions;
    use purrdf_shapes::recursion::{RecursionOptions, RecursionSemantics};
    for conformance in [false, true] {
        let mut source = RdfDatasetBuilder::new();
        let target = source
            .intern_iri("http://www.w3.org/ns/shacl#targetWhere")
            .unwrap();
        let kind = source
            .intern_iri("http://www.w3.org/ns/shacl#nodeKind")
            .unwrap();
        let all = if conformance {
            let all = source.intern_iri("http://example.org/All").unwrap();
            let rdf_type = source
                .intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
                .unwrap();
            let node_shape = source
                .intern_iri("http://www.w3.org/ns/shacl#NodeShape")
                .unwrap();
            source.push_quad(all, rdf_type, node_shape, None).unwrap();
            Some(all)
        } else {
            None
        };
        for (root, scope, value) in [("Left", 4, "IRI"), ("Right", 5, "Literal")] {
            let root = source
                .intern_iri(&format!("http://example.org/{root}"))
                .unwrap();
            let blank = source.intern_blank("same", BlankScope(scope)).unwrap();
            let value = source
                .intern_iri(&format!("http://www.w3.org/ns/shacl#{value}"))
                .unwrap();
            if let Some(all) = all {
                let expression = source
                    .intern_iri("http://www.w3.org/ns/shacl#expression")
                    .unwrap();
                let conforms = source
                    .intern_iri("http://www.w3.org/ns/shacl-node-expr#conformsToShape")
                    .unwrap();
                let this = source
                    .intern_iri("http://www.w3.org/ns/shacl#this")
                    .unwrap();
                let first = source
                    .intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#first")
                    .unwrap();
                let rest = source
                    .intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#rest")
                    .unwrap();
                let nil = source
                    .intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#nil")
                    .unwrap();
                let expr = source
                    .intern_blank(&format!("{scope}expr"), BlankScope::DEFAULT)
                    .unwrap();
                let head = source
                    .intern_blank(&format!("{scope}head"), BlankScope::DEFAULT)
                    .unwrap();
                let tail = source
                    .intern_blank(&format!("{scope}tail"), BlankScope::DEFAULT)
                    .unwrap();
                source.push_quad(root, target, all, None).unwrap();
                source.push_quad(root, expression, expr, None).unwrap();
                source.push_quad(expr, conforms, head, None).unwrap();
                source.push_quad(head, first, this, None).unwrap();
                source.push_quad(head, rest, tail, None).unwrap();
                source.push_quad(tail, first, blank, None).unwrap();
                source.push_quad(tail, rest, nil, None).unwrap();
            } else {
                source.push_quad(root, target, blank, None).unwrap();
                let not = source.intern_iri("http://www.w3.org/ns/shacl#not").unwrap();
                source.push_quad(root, not, blank, None).unwrap();
            }
            source.push_quad(blank, kind, value, None).unwrap();
        }
        let source = source.freeze().unwrap();
        for semantics in [RecursionSemantics::Lfp, RecursionSemantics::Gfp] {
            let request = ValidationOptions::default().with_recursion(RecursionOptions {
                semantics,
                max_pairs: 4_096,
                max_iterations: 4_096,
            });
            let shapes = purrdf_shapes::shapes::from_dataset_with_options(
                &source,
                None,
                &[],
                None,
                None,
                &purrdf_shapes::ShapesImports::new(),
                &request,
            )
            .unwrap();
            let prepared = PreparedShapes::new(Arc::new(shapes));
            let bytes = prepared
                .to_product(
                    &ShapesProfile::CORE,
                    &mut purrdf_core::CanonicalizationControl::new(
                        purrdf_core::RDFC_V2_CORPUS_POLICY,
                        None,
                    ),
                )
                .unwrap();
            for (graph, iri, literal) in [
                (
                    "<http://example.org/a> <http://example.org/p> \"value\" .",
                    "a",
                    "value",
                ),
                (
                    "<http://example.org/fresh> <http://example.org/q> \"other\" .",
                    "fresh",
                    "other",
                ),
            ] {
                let data = parse_turtle_to_dataset(graph, None).unwrap();
                let report = prepared.bind_dataset(&data).unwrap().validate().unwrap();
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
                assert_eq!(
                    rows,
                    vec![
                        (
                            "<http://example.org/Left>".to_owned(),
                            if conformance {
                                format!("\"{literal}\"")
                            } else {
                                format!("<http://example.org/{iri}>")
                            }
                        ),
                        (
                            "<http://example.org/Right>".to_owned(),
                            if conformance {
                                format!("<http://example.org/{iri}>")
                            } else {
                                format!("\"{literal}\"")
                            }
                        ),
                    ]
                );
                let expected = report.to_ntriples().unwrap();
                for rebuild in [false, true] {
                    let product = ShapesProduct::open(&bytes).unwrap();
                    let restored = if rebuild {
                        product.rebuild_with_options(
                            &ShapesProfile::CORE,
                            &HostBindings::empty(),
                            &request,
                        )
                    } else {
                        product.admit_with_options(
                            &ShapesProfile::CORE,
                            &HostBindings::empty(),
                            &request,
                        )
                    }
                    .unwrap();
                    assert_eq!(
                        restored
                            .bind_dataset(&data)
                            .unwrap()
                            .validate()
                            .unwrap()
                            .to_ntriples()
                            .unwrap(),
                        expected
                    );
                    assert_eq!(
                        restored
                            .to_product(
                                &ShapesProfile::CORE,
                                &mut purrdf_core::CanonicalizationControl::new(
                                    purrdf_core::RDFC_V2_CORPUS_POLICY,
                                    None
                                )
                            )
                            .unwrap(),
                        bytes
                    );
                }
            }
        }
    }
}
