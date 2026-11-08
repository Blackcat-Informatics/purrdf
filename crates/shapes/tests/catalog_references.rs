// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Shared definitions retain their owners and original reporting semantics.

use std::sync::Arc;

use purrdf_shapes::constraints::{conforms, validate_shape};
use purrdf_shapes::data::ShaclData;
use purrdf_shapes::engine::parse_shapes;
use purrdf_shapes::shapes::{Constraint, ModelOwner, Shape};
use purrdf_shapes::term::{Literal, NamedNode, Term};
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;

#[test]
fn logical_members_keep_distinct_original_models_with_equal_shape_names() {
    let prefixes = "@prefix sh: <http://www.w3.org/ns/shacl#> .
                    @prefix ex: <http://example.org/> .";
    let dataset = parse_turtle_to_dataset(
        &format!("{prefixes} ex:focus ex:predicate ex:object ."),
        None,
    )
    .unwrap();
    let data = ShaclData::new(Arc::clone(&dataset), dataset, None);
    for declaration in ["ex:Shared", "_:shared"] {
        let export = |kind: &str| {
            let parsed = parse_shapes(
                &format!(
                    "{prefixes} {declaration} a sh:NodeShape ;
                 sh:targetNode ex:focus ; sh:nodeKind {kind} ."
                ),
                None,
            )
            .unwrap();
            assert_eq!(parsed.node_shapes.len(), 1);
            parsed.node_shapes[0].clone()
        };
        // Both factories are gone; each exported handle keeps its own body.
        let accepting = export("sh:IRI");
        let refusing = export("sh:BlankNode");
        assert_eq!(accepting.id, refusing.id);
        let ambient_owner = accepting.owner.clone();
        let bodies = [&accepting, &refusing].map(|shape| {
            shape
                .owner
                .as_ref()
                .unwrap()
                .definition(&shape.id)
                .unwrap()
                .clone()
        });
        // Also exercise actual unowned bodies, whose equal names have no
        // shared catalog guaranteeing equal meaning.
        for [accepting, refusing] in [[accepting, refusing], bodies] {
            for owner in [None, ambient_owner.clone()] {
                for focus in ["http://example.org/focus", "http://example.org/foreign"] {
                    let focus = Term::NamedNode(NamedNode::new_unchecked(focus));
                    assert!(conforms(&data, &focus, &accepting).unwrap());
                    assert!(!conforms(&data, &focus, &refusing).unwrap());
                    for reverse in [false, true] {
                        for (operation, expected) in [
                            (Constraint::And as fn(Vec<Shape>) -> Constraint, false),
                            (Constraint::Or, true),
                            (Constraint::Xone, true),
                        ] {
                            let mut members = vec![accepting.clone(), refusing.clone()];
                            if reverse {
                                members.reverse();
                            }
                            let mut root = accepting.clone();
                            root.id = Term::NamedNode(NamedNode::from("http://example.org/Root"));
                            root.owner = owner.clone();
                            root.constraints = vec![operation(members)];
                            assert_eq!(
                                conforms(&data, &focus, &root).unwrap(),
                                expected,
                                "{declaration}, {focus}, reverse={reverse}, {:?}",
                                root.constraints
                            );
                            let results = validate_shape(&data, &focus, &root).unwrap();
                            assert_eq!(results.is_empty(), expected);
                            if !expected {
                                assert_eq!(results.len(), 1);
                                assert_eq!(results[0].focus_node, focus);
                                assert_eq!(results[0].source_shape, root.id);
                                assert_eq!(
                                    results[0].source_constraint_component.as_str(),
                                    purrdf_iri::vocab::sh::AND_CONSTRAINT_COMPONENT
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

/// An exported handle's local deactivation is part of the evaluated body.
#[test]
fn equal_named_handles_keep_local_deactivation_distinct() {
    let parsed = parse_shapes(
        "@prefix sh: <http://www.w3.org/ns/shacl#> .
         <http://example.org/Shared> a sh:NodeShape ; sh:nodeKind sh:BlankNode .",
        None,
    )
    .unwrap();
    let refusing = parsed.node_shapes[0].clone();
    drop(parsed);
    let mut inactive = refusing.clone();
    inactive.deactivated = true;
    let dataset = parse_turtle_to_dataset(
        "<http://example.org/focus> <http://example.org/p> <http://example.org/o> .",
        None,
    )
    .unwrap();
    let data = ShaclData::new(Arc::clone(&dataset), dataset, None);
    let focus = Term::NamedNode(NamedNode::from("http://example.org/focus"));
    assert!(!conforms(&data, &focus, &refusing).unwrap());
    assert!(conforms(&data, &focus, &inactive).unwrap());
    for members in [
        vec![refusing.clone(), inactive.clone()],
        vec![inactive, refusing.clone()],
    ] {
        let mut root = refusing.clone();
        root.id = Term::NamedNode(NamedNode::from("http://example.org/Root"));
        root.owner = None;
        root.constraints = vec![Constraint::Xone(members)];
        assert!(conforms(&data, &focus, &root).unwrap());
        assert!(validate_shape(&data, &focus, &root).unwrap().is_empty());
    }
}

#[test]
fn metadata_follows_references_without_activating_unused_catalog_definitions() {
    use purrdf_shapes::expression::{FnCall, NodeExpr};
    let mut shapes = parse_shapes(
        "@prefix sh: <http://www.w3.org/ns/shacl#> .
         <http://example.org/Root> a sh:NodeShape .",
        None,
    )
    .unwrap();
    let root_id = shapes.node_shapes[0].id.clone();
    let prototype = shapes.owner.definition(&root_id).unwrap();
    let members: Vec<_> = ["used", "unused"].into_iter().map(|name| {
        let mut definition = prototype.clone();
        definition.id = Term::NamedNode(NamedNode::new_unchecked(format!("http://example.org/{name}")));
        definition.constraints = vec![Constraint::Expression {
            expr: NodeExpr::Union(vec![
                NodeExpr::Call(FnCall::UserDefined {
                    iri: NamedNode::new_unchecked(format!("http://example.org/{name}-call")),
                    args: Vec::new(),
                }),
                NodeExpr::Select {
                    query: format!("SELECT ?value WHERE {{ ?subject <http://example.org/{name}-relation> ?value }}"),
                    variable: "value".to_owned(), key: "sh:select",
                },
            ]), messages: Vec::new(), severity: None,
        }];
        definition
    }).collect();
    let mut root = prototype.clone();
    root.constraints = vec![Constraint::Reference(members[0].id.clone())];
    shapes.owner =
        ModelOwner::from_definitions(std::iter::once(root).chain(members).collect()).unwrap();
    shapes.node_shapes = vec![shapes.owner.root(&root_id).unwrap()];
    let resolution = shapes.function_resolution().unwrap();
    assert!(
        resolution
            .sites()
            .any(|site| site.function == "http://example.org/used-call")
    );
    assert!(
        !resolution
            .sites()
            .any(|site| site.function == "http://example.org/unused-call")
    );
    let usage = shapes
        .extension_usage(purrdf_sparql_eval::ExtensionEnv::empty())
        .unwrap();
    assert!(usage.data().contains("http://example.org/used-relation"));
    assert!(!usage.data().contains("http://example.org/unused-relation"));
}

#[test]
fn metadata_returns_native_missing_definition_and_function_body_refusals() {
    use purrdf_shapes::PreparationRefusal;
    use purrdf_shapes::expression::{CustomFnKind, CustomFunction, NodeExpr};
    use std::sync::OnceLock;
    let mut shapes = parse_shapes(
        "@prefix sh: <http://www.w3.org/ns/shacl#> .
         <http://example.org/Root> a sh:NodeShape .",
        None,
    )
    .unwrap();
    let absent = Term::NamedNode(NamedNode::from("http://example.org/Absent"));
    shapes.node_shapes[0].constraints = vec![Constraint::Reference(absent.clone())];
    let function_error = shapes.function_resolution().unwrap_err();
    let usage_error = shapes
        .extension_usage(purrdf_sparql_eval::ExtensionEnv::empty())
        .unwrap_err();
    assert_eq!(function_error, usage_error);
    assert!(matches!(&function_error, PreparationRefusal::UnknownReference(id) if **id == absent));

    let iri = NamedNode::from("http://example.org/function");
    shapes.node_shapes[0].constraints = vec![Constraint::Expression {
        expr: NodeExpr::CustomCall {
            func: iri.clone(),
            args: Vec::new(),
        },
        messages: Vec::new(),
        severity: None,
    }];
    let unknown = shapes.function_resolution().unwrap_err();
    assert_eq!(unknown, PreparationRefusal::UnknownFunction(iri.clone()));
    assert_eq!(
        shapes
            .extension_usage(purrdf_sparql_eval::ExtensionEnv::empty())
            .unwrap_err(),
        unknown
    );
    let owner = ModelOwner::from_functions(vec![Arc::new(CustomFunction {
        iri: iri.clone(),
        kind: CustomFnKind::ListParameter,
        params: Vec::new(),
        required: 0,
        body: OnceLock::new(),
    })])
    .unwrap();
    shapes.node_shapes[0].owner = Some(Arc::clone(&owner));
    shapes.owner = owner;
    let missing_body = shapes.function_resolution().unwrap_err();
    assert_eq!(missing_body, PreparationRefusal::MissingFunctionBody(iri));
    assert_eq!(
        shapes
            .extension_usage(purrdf_sparql_eval::ExtensionEnv::empty())
            .unwrap_err(),
        missing_body
    );
    drop(shapes);
    assert!(matches!(&function_error, PreparationRefusal::UnknownReference(id) if **id == absent));
}

#[test]
fn nested_same_named_domains_keep_both_function_and_relation_inventory() {
    use purrdf_shapes::expression::{CustomFnKind, CustomFunction, FnCall, NodeExpr};
    use std::sync::OnceLock;

    let name = NamedNode::from("http://example.org/function");
    let marker = "http://example.org/marker";
    let relation = "http://example.org/relation";
    let call = || NodeExpr::CustomCall {
        func: name.clone(),
        args: Vec::new(),
    };
    let domain = |body| {
        ModelOwner::from_functions(vec![Arc::new(CustomFunction {
            iri: name.clone(),
            kind: CustomFnKind::ListParameter,
            params: Vec::new(),
            required: 0,
            body: OnceLock::from(body),
        })])
        .unwrap()
    };
    let inner = domain(NodeExpr::Union(vec![
        NodeExpr::Call(FnCall::UserDefined {
            iri: NamedNode::from(marker),
            args: Vec::new(),
        }),
        NodeExpr::Select {
            query: format!("SELECT ?value WHERE {{ ?subject <{relation}> ?value }}"),
            variable: "value".to_owned(),
            key: "sh:select",
        },
    ]));
    let outer = domain(NodeExpr::Owned {
        expr: Box::new(call()),
        owner: inner,
    });
    let mut shapes = parse_shapes(
        "@prefix sh: <http://www.w3.org/ns/shacl#> .
         <http://example.org/Root> a sh:NodeShape .",
        None,
    )
    .unwrap();
    shapes.node_shapes[0]
        .constraints
        .push(Constraint::Expression {
            expr: NodeExpr::Owned {
                expr: Box::new(call()),
                owner: outer,
            },
            messages: Vec::new(),
            severity: None,
        });
    assert!(
        shapes
            .function_resolution()
            .unwrap()
            .sites()
            .any(|site| site.function == marker)
    );
    let usage = shapes
        .extension_usage(purrdf_sparql_eval::ExtensionEnv::empty())
        .unwrap();
    assert!(usage.is_complete(), "{usage:?}");
    assert!(usage.data().contains(relation), "{usage:?}");
}

#[test]
fn same_named_functions_evaluate_their_original_domain_bodies() {
    use purrdf_shapes::expression::{
        CustomFnKind, CustomFunction, NodeExpr, RecursionGuard, eval_node_expr,
    };
    use std::sync::OnceLock;
    let name = NamedNode::from("http://example.org/function");
    let expected: Vec<_> = ["http://example.org/Left", "http://example.org/Right"]
        .into_iter()
        .map(|iri| Term::NamedNode(NamedNode::from(iri)))
        .collect();
    let branches = expected
        .iter()
        .map(|value| {
            let declaration = Arc::new(CustomFunction {
                iri: name.clone(),
                kind: CustomFnKind::ListParameter,
                params: Vec::new(),
                required: 0,
                body: OnceLock::from(NodeExpr::Constant(value.clone())),
            });
            NodeExpr::Owned {
                expr: Box::new(NodeExpr::CustomCall {
                    func: name.clone(),
                    args: Vec::new(),
                }),
                owner: ModelOwner::from_functions(vec![declaration]).unwrap(),
            }
        })
        .collect();
    let expression = NodeExpr::Union(branches);
    let dataset = parse_turtle_to_dataset("", None).unwrap();
    let data = ShaclData::new(Arc::clone(&dataset), dataset, None);
    let focus = Term::BlankNode("outside".to_owned());
    assert_eq!(
        eval_node_expr(&data, &focus, &expression, &mut RecursionGuard::default()).unwrap(),
        expected
    );
}

#[test]
fn exported_flat_reference_keeps_its_original_model_domain() {
    let shapes = parse_shapes(
        "@prefix sh: <http://www.w3.org/ns/shacl#> .
         @prefix ex: <http://example.org/> .
         ex:Root a sh:NodeShape .
         ex:Leaf a sh:NodeShape; sh:nodeKind sh:IRI .",
        None,
    )
    .unwrap();
    let root_id = Term::NamedNode(NamedNode::new_unchecked("http://example.org/Root"));
    let leaf_id = Term::NamedNode(NamedNode::new_unchecked("http://example.org/Leaf"));
    let mut root = shapes
        .node_shapes
        .iter()
        .find(|shape| shape.id == root_id)
        .unwrap()
        .clone();
    root.constraints = vec![Constraint::Reference(leaf_id.clone())];
    drop(shapes);
    let dataset = parse_turtle_to_dataset("", None).unwrap();
    let data = ShaclData::new(Arc::clone(&dataset), dataset, None);
    for (focus, expected) in [
        (root_id, true),
        (Term::Literal(Literal::new_simple_literal("foreign")), false),
    ] {
        assert_eq!(conforms(&data, &focus, &root).unwrap(), expected);
        let results = validate_shape(&data, &focus, &root).unwrap();
        assert_eq!(results.is_empty(), expected);
        if !expected {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].source_shape, leaf_id);
            assert_eq!(results[0].focus_node, focus);
        }
    }
}

#[test]
fn catalog_chain_keeps_conformance_reporting_and_owner_after_export() {
    let parsed = parse_shapes(
        "@prefix sh: <http://www.w3.org/ns/shacl#> .
         @prefix ex: <http://example.org/> .
         ex:Root a sh:NodeShape . ex:Middle a sh:NodeShape .
         ex:Leaf a sh:NodeShape; sh:nodeKind sh:IRI .",
        None,
    )
    .unwrap();
    let mut definitions: Vec<_> = parsed
        .node_shapes
        .iter()
        .map(|root| parsed.owner.definition(&root.id).unwrap().clone())
        .collect();
    drop(parsed);
    let root_id = Term::NamedNode(NamedNode::new_unchecked("http://example.org/Root"));
    let middle_id = Term::NamedNode(NamedNode::new_unchecked("http://example.org/Middle"));
    let leaf_id = Term::NamedNode(NamedNode::new_unchecked("http://example.org/Leaf"));
    for definition in &mut definitions {
        if definition.id == root_id {
            definition.constraints = vec![Constraint::Reference(middle_id.clone())];
        } else if definition.id == middle_id {
            definition.constraints = vec![Constraint::Reference(leaf_id.clone())];
        }
    }
    let catalog = ModelOwner::from_definitions(definitions).unwrap();
    let weak = Arc::downgrade(&catalog);
    let root = catalog.root(&root_id).unwrap();
    drop(catalog);
    let dataset = parse_turtle_to_dataset("", None).unwrap();
    let data = ShaclData::new(Arc::clone(&dataset), dataset, None);
    for (focus, expected) in [
        (root_id, true),
        (Term::Literal(Literal::new_simple_literal("foreign")), false),
    ] {
        assert_eq!(conforms(&data, &focus, &root).unwrap(), expected);
        let results = validate_shape(&data, &focus, &root).unwrap();
        assert_eq!(results.is_empty(), expected);
        if !expected {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].source_shape, leaf_id);
            assert_eq!(results[0].focus_node, focus);
        }
    }
    assert!(weak.upgrade().is_some());
    drop(root);
    assert!(weak.upgrade().is_none(), "catalog references own no cycle");
}

#[test]
fn parsed_computed_shape_index_releases_its_owner_with_the_model() {
    let shapes = parse_shapes(
        "@prefix sh: <http://www.w3.org/ns/shacl#> .
         @prefix ex: <http://example.org/> .
         ex:Root a sh:NodeShape; sh:nodeByExpression ex:Leaf .
         ex:Leaf a sh:NodeShape; sh:nodeKind sh:IRI .",
        None,
    )
    .unwrap();
    let weak = shapes
        .node_shapes
        .iter()
        .flat_map(|shape| &shapes.owner.definition(&shape.id).unwrap().constraints)
        .find_map(|constraint| match constraint {
            Constraint::NodeByExpression { shapes, .. } => Some(shapes.clone()),
            _ => None,
        })
        .expect("the parsed expression retains its actual shared shape index");
    assert!(weak.upgrade().is_some());
    let root = shapes
        .node_shapes
        .iter()
        .find(|shape| {
            shape.id == Term::NamedNode(NamedNode::new_unchecked("http://example.org/Root"))
        })
        .unwrap()
        .clone();
    drop(shapes);
    assert!(
        weak.upgrade().is_some(),
        "the exported root retains its resolution domain"
    );
    let dataset = parse_turtle_to_dataset("", None).unwrap();
    let data = ShaclData::new(Arc::clone(&dataset), dataset, None);
    assert!(
        conforms(
            &data,
            &Term::NamedNode(NamedNode::new_unchecked("http://example.org/focus")),
            &root
        )
        .unwrap()
    );
    assert!(
        !conforms(
            &data,
            &Term::Literal(Literal::new_simple_literal("foreign")),
            &root
        )
        .unwrap()
    );
    drop(root);
    assert!(
        weak.upgrade().is_none(),
        "an indexed definition must not own its index"
    );
}

#[test]
fn recursive_expression_declaration_releases_its_owner_with_the_model() {
    let shapes = parse_shapes(
        "@prefix sh: <http://www.w3.org/ns/shacl#> .
         @prefix shnex: <http://www.w3.org/ns/shacl-node-expr#> .
         @prefix ex: <http://example.org/> .
         ex:loop a sh:ListParameterExpressionFunction;
           sh:bodyExpression [ ex:loop ( [ shnex:arg 0 ] ) ];
           sh:parameter [ sh:path shnex:arg0 ] .
         ex:Root a sh:NodeShape; sh:expression [ ex:loop ( sh:this ) ] .",
        None,
    )
    .unwrap();
    let iri = NamedNode::new_unchecked("http://example.org/loop");
    let declaration = shapes
        .owner
        .function(&iri)
        .expect("the actual parsed recursive declaration");
    let weak = Arc::downgrade(declaration);
    assert!(weak.upgrade().is_some());
    drop(shapes);
    assert!(
        weak.upgrade().is_none(),
        "a function body must not own its recursive declaration"
    );
}

#[test]
fn standalone_function_keeps_its_model_after_the_factory_is_dropped() {
    use purrdf_shapes::expression::{RecursionGuard, eval_node_expr};
    use purrdf_shapes::imports::ShapesImports;
    let source = parse_turtle_to_dataset(
        "@prefix sh: <http://www.w3.org/ns/shacl#> .
         @prefix shnex: <http://www.w3.org/ns/shacl-node-expr#> .
         @prefix ex: <http://example.org/> .
         ex:id a sh:ListParameterExpressionFunction;
           sh:parameter [ sh:path shnex:arg0 ];
           sh:bodyExpression [ shnex:arg 0 ] .
         _:expression ex:id ( [ ex:id (\"kept\") ] ) .",
        None,
    )
    .unwrap();
    let id = Term::BlankNode("expression".into());
    let (shapes, mut expressions) = purrdf_shapes::shapes::from_dataset_with_node_expressions(
        &source,
        &[],
        None,
        std::slice::from_ref(&id),
        &ShapesImports::default(),
    )
    .unwrap();
    let weak = Arc::downgrade(&shapes.owner);
    let expression = expressions.pop().unwrap();
    drop(shapes);
    drop(source);
    assert!(weak.upgrade().is_some());
    let dataset = parse_turtle_to_dataset("", None).unwrap();
    let data = ShaclData::new(Arc::clone(&dataset), dataset, None);
    assert_eq!(
        eval_node_expr(&data, &id, &expression, &mut RecursionGuard::default()).unwrap(),
        vec![Term::Literal(Literal::new_simple_literal("kept"))],
    );
    drop(expression);
    assert!(
        weak.upgrade().is_none(),
        "the final exported expression releases its model"
    );
}

#[test]
fn computed_targets_and_unique_grouping_retain_their_function_domain() {
    use purrdf_shapes::engine::{PreparedShapes, validate_with};
    let shapes = parse_shapes(
        "@prefix sh: <http://www.w3.org/ns/shacl#> .
         @prefix shnex: <http://www.w3.org/ns/shacl-node-expr#> .
         @prefix ex: <http://example.org/> .
         ex:id a sh:ListParameterExpressionFunction;
           sh:parameter [ sh:path shnex:arg0 ];
           sh:bodyExpression [ shnex:arg 0 ] .
         ex:Root a sh:NodeShape;
           sh:targetNode [ shnex:concat ([ ex:id (ex:a) ] [ ex:id (ex:b) ]) ];
           sh:uniqueValuesFor ex:key .",
        None,
    )
    .unwrap();
    for second_key in ["same", "different"] {
        let dataset = parse_turtle_to_dataset(
            &format!(
                "<http://example.org/a> <http://example.org/key> \"same\" .
                       <http://example.org/b> <http://example.org/key> \"{second_key}\" ."
            ),
            None,
        )
        .unwrap();
        let data = ShaclData::new(Arc::clone(&dataset), Arc::clone(&dataset), None);
        let expected = if second_key == "same" { 2 } else { 0 };
        let report = validate_with(&data, &shapes).unwrap();
        assert_eq!(report.results.len(), expected);
        let prepared = PreparedShapes::new(Arc::new(shapes.clone()));
        let validator = prepared.bind_dataset(&dataset).unwrap();
        let report = validator.validate().unwrap();
        assert_eq!(report.results.len(), expected);
        let report = validator
            .validate_focus_nodes(&[Term::NamedNode(NamedNode::new_unchecked(
                "http://example.org/a",
            ))])
            .unwrap();
        assert_eq!(report.results.len(), usize::from(expected != 0));
    }
}
