// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Authored identity catalogs through the actual lowering and conformance path.

use std::sync::Arc;

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_shapes::constraints::{conforms, validate_shape, validate_shape_with};
use purrdf_shapes::data::ShaclData;
use purrdf_shapes::engine::parse_shapes;
use purrdf_shapes::model::BoxRoleVocab;
use purrdf_shapes::report::{ConformanceDisallows, ValidationReport};
use purrdf_shapes::shapes::{Constraint, ModelOwner};
use purrdf_shapes::term::{NamedNode, Term};
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;
use purrdf_shapes::{PreparationRefusal, ShapesError};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

#[test]
fn function_domain_admission_has_no_per_declaration_allocation() {
    use purrdf_shapes::expression::{CustomFnKind, CustomFunction, NodeExpr};
    use std::sync::OnceLock;

    let mut first = None;
    for population in [1, 8, 32] {
        let functions: Vec<_> = (0..population)
            .rev()
            .map(|index| {
                Arc::new(CustomFunction {
                    iri: NamedNode::new_unchecked(format!("http://example.org/f{index}")),
                    kind: CustomFnKind::ListParameter,
                    params: Vec::new(),
                    required: 0,
                    body: OnceLock::from(NodeExpr::Empty),
                })
            })
            .collect();
        let originals = functions.clone();
        let window = CurrentThreadWindow::open();
        let owner = ModelOwner::from_functions(functions).unwrap();
        let measured = window.close();
        eprintln!("{population} functions: {measured:?}");
        assert_eq!(measured.allocations, 2);
        if let Some(previous) = first {
            assert_eq!(
                measured, previous,
                "domain admission must not copy declaration keys"
            );
        }
        first = Some(measured);
        for original in &originals {
            assert!(Arc::ptr_eq(
                owner.function(&original.iri).unwrap(),
                original
            ));
        }
    }
}

#[test]
fn catalog_admission_allocation_growth_and_retained_reference_execution() {
    let mut parsed = parse_shapes(
        "@prefix sh: <http://www.w3.org/ns/shacl#> .
         <http://example.org/S> a sh:NodeShape; sh:class <http://example.org/Expected> .",
        None,
    )
    .unwrap();
    let mut prototype = parsed.node_shapes.pop().unwrap();
    prototype.owner = None;
    drop(parsed);
    let dataset = parse_turtle_to_dataset("", None).unwrap();
    let data = ShaclData::new(Arc::clone(&dataset), dataset, None);
    let focus = Term::BlankNode("focus".to_owned());
    // The complete domain also retains its immutable declaration slice header.
    // Keep the original map allocation and peak limits; account for that one
    // fixed header separately from definition-dependent storage.
    let declaration_header = size_of::<Box<[Arc<purrdf_shapes::expression::CustomFunction>]>>();
    for (population, bounds) in [
        (1, (4, 1670, 1358, 1590)),
        (8, (11, 6400, 3904, 6320)),
        (32, (35, 25_334, 15_350, 25_254)),
    ] {
        let definitions = (0..population)
            .map(|index| {
                let mut definition = prototype.clone();
                definition.id = Term::BlankNode(format!("s{index}"));
                definition
            })
            .collect();
        let window = CurrentThreadWindow::open();
        let catalog = ModelOwner::from_definitions(definitions).unwrap();
        let measured = window.close();
        eprintln!("{population} definitions: {measured:?}");
        let (allocations, requested, retained, peak) = bounds;
        assert!(measured.allocations <= allocations);
        assert!(measured.requested_bytes <= requested + u64::try_from(declaration_header).unwrap());
        assert!(measured.retained_bytes <= retained + i64::try_from(declaration_header).unwrap());
        assert!(measured.peak_working_bytes <= peak);
        let root = catalog.root(&Term::BlankNode("s0".to_owned())).unwrap();
        drop(catalog);
        assert!(!conforms(&data, &focus, &root).unwrap());
    }

    let mut reference = prototype.clone();
    reference.id = Term::BlankNode("reference".to_owned());
    reference.constraints = vec![Constraint::Reference(prototype.id.clone())];
    let reference_id = reference.id.clone();
    let catalog = ModelOwner::from_definitions(vec![prototype, reference]).unwrap();
    let root = catalog.root(&reference_id).unwrap();
    drop(catalog);
    assert!(!conforms(&data, &focus, &root).unwrap());
}

#[test]
fn catalog_root_export_and_clone_share_constraint_bodies() {
    let mut parsed = parse_shapes(
        "@prefix sh: <http://www.w3.org/ns/shacl#> .
         <http://example.org/S> a sh:NodeShape; sh:class <http://example.org/Expected> .",
        None,
    )
    .unwrap();
    let mut prototype = parsed.node_shapes.pop().unwrap();
    prototype.owner = None;
    let dataset = parse_turtle_to_dataset("", None).unwrap();
    let data = ShaclData::new(Arc::clone(&dataset), dataset, None);
    let focus = Term::BlankNode("focus".to_owned());
    for population in [1, 8, 32] {
        let mut definition = prototype.clone();
        definition.constraints = vec![prototype.constraints[0].clone(); population];
        let expected = validate_shape(&data, &focus, &definition).unwrap();
        assert_eq!(expected.len(), population);
        let expected = ValidationReport::from_results(expected, ConformanceDisallows::default())
            .to_ntriples()
            .unwrap();
        let catalog = ModelOwner::from_definitions(vec![definition]).unwrap();
        let window = CurrentThreadWindow::open();
        let root = catalog.root(&prototype.id).unwrap();
        let copy = root.clone();
        let measured = window.close();
        eprintln!("{population} constraints, export+clone: {measured:?}");
        assert!(measured.allocations <= 6);
        assert!(measured.requested_bytes <= 544);
        assert!(measured.retained_bytes <= 544);
        assert!(measured.peak_working_bytes <= 544);
        drop((root, catalog));
        let observed = ValidationReport::from_results(
            validate_shape(&data, &focus, &copy).unwrap(),
            ConformanceDisallows::default(),
        )
        .to_ntriples()
        .unwrap();
        assert_eq!(observed, expected);
        assert!(!conforms(&data, &focus, &copy).unwrap());
    }
}

#[test]
fn identity_reference_report_retains_caller_path_role_vocabulary() {
    let mut parsed = parse_shapes(
        "@prefix sh: <http://www.w3.org/ns/shacl#> .
         <http://example.org/S> a sh:NodeShape;
           sh:property [ sh:path <http://example.org/p>; sh:nodeKind sh:IRI ] .",
        None,
    )
    .unwrap();
    let mut definition = parsed.node_shapes.pop().unwrap();
    definition.owner = None;
    let id = definition.id.clone();
    let dataset = parse_turtle_to_dataset(
        "<http://example.org/focus> <http://example.org/p> \"bad\" .
         <http://example.org/p> <http://example.org/graphBoxRole> <http://example.org/PathRole> .",
        None,
    )
    .unwrap();
    let data = ShaclData::new(Arc::clone(&dataset), dataset, None);
    let focus = Term::NamedNode(NamedNode::from("http://example.org/focus"));
    let vocabulary = BoxRoleVocab::for_namespace("http://example.org/");
    let expected = validate_shape_with(&data, &focus, &definition, Some(&vocabulary)).unwrap();
    assert_eq!(expected.len(), 1);
    assert_eq!(expected[0].path_box_roles.len(), 1);
    let expected = ValidationReport::from_results(expected, ConformanceDisallows::default())
        .to_ntriples()
        .unwrap();
    let catalog = ModelOwner::from_definitions(vec![definition]).unwrap();
    let root = catalog.root(&id).unwrap();
    drop((catalog, parsed));
    let observed = ValidationReport::from_results(
        validate_shape_with(&data, &focus, &root, Some(&vocabulary)).unwrap(),
        ConformanceDisallows::default(),
    )
    .to_ntriples()
    .unwrap();
    assert_eq!(observed, expected);
}

#[test]
fn missing_catalog_identity_retains_the_original_term_without_clone_allocation() {
    let mut parsed = parse_shapes(
        "@prefix sh: <http://www.w3.org/ns/shacl#> .
         <http://example.org/S> a sh:NodeShape .",
        None,
    )
    .unwrap();
    let mut definition = parsed.node_shapes.pop().unwrap();
    definition.owner = None;
    let missing = Term::BlankNode("absent".to_owned());
    definition.constraints = vec![Constraint::Reference(missing.clone())];
    let id = definition.id.clone();
    let catalog = ModelOwner::from_definitions(vec![definition]).unwrap();
    let root = catalog.root(&id).unwrap();
    let dataset = parse_turtle_to_dataset("", None).unwrap();
    let data = ShaclData::new(Arc::clone(&dataset), dataset, None);
    let error = conforms(&data, &id, &root).unwrap_err();
    let window = CurrentThreadWindow::open();
    let copy = error.clone();
    let measured = window.close();
    assert_eq!(measured.allocations, 0);
    assert_eq!(measured.requested_bytes, 0);
    drop((error, root, catalog, data, parsed));
    let ShapesError::Preparation(PreparationRefusal::UnknownReference(original)) = copy else {
        panic!("an absent catalog identity must be a preparation refusal");
    };
    assert_eq!(*original, missing);
}
