// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Property identity edges keep the original traversal and edge-local metadata.

use std::fmt::Write as _;
use std::sync::Arc;

#[path = "../examples/support/metaschemas.rs"]
mod metaschemas;

#[path = "support/terms.rs"]
mod terms;
use terms::example_org as id;

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};

use purrdf_shapes::constraints::{conforms, validate_shape, validate_shape_with};
use purrdf_shapes::data::ShaclData;
use purrdf_shapes::engine::parse_shapes;
use purrdf_shapes::report::{ConformanceDisallows, Severity, ValidationReport};
use purrdf_shapes::shapes::{Constraint, PropertyShape, Shape, Shapes};
use purrdf_shapes::term::{Literal, NamedNode, Term};
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn definition(shapes: &Shapes, local: &str) -> Shape {
    shapes.owner.definition(&id(local)).unwrap().clone()
}

fn exported_root(shapes: &Shapes) -> Shape {
    shapes
        .node_shapes
        .iter()
        .find(|shape| shape.id == id("Root"))
        .unwrap()
        .clone()
}

fn inline_root(shapes: &Shapes, property: PropertyShape) -> Shape {
    let mut root = definition(shapes, "Root");
    root.constraints
        .retain(|constraint| !matches!(constraint, Constraint::PropertyReference(_)));
    root.property_shapes = vec![property];
    root
}

fn data(source: &str) -> ShaclData {
    let dataset = parse_turtle_to_dataset(source, None).unwrap();
    ShaclData::new(Arc::clone(&dataset), dataset, None)
}

#[test]
fn shared_dag_admission_keeps_one_body_per_identity() {
    let prefix =
        "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .\n";
    drop(
        parse_shapes(
            &format!("{prefix} ex:S0 a sh:NodeShape; sh:nodeKind sh:IRI ."),
            None,
        )
        .unwrap(),
    );
    let store = data("");
    for depth in [1, 8, 32] {
        let mut source = format!("{prefix} ex:S0 a sh:NodeShape; sh:nodeKind sh:IRI .\n");
        for level in 1..=depth {
            let previous = level - 1;
            writeln!(
                source,
                "ex:S{level} a sh:NodeShape; sh:and (ex:S{previous} ex:S{previous}) ."
            )
            .unwrap();
        }
        let window = CurrentThreadWindow::open();
        let parsed = parse_shapes(&source, None).unwrap();
        let measured = window.close();
        eprintln!("{depth} shared levels: {measured:?}");
        let vertices = u64::try_from(depth + 1).unwrap();
        let signed_vertices = i64::try_from(vertices).unwrap();
        // A fixed parser envelope plus one definition and its two identity
        // edges per level. The bound is linear even though unfolding doubles
        // the dependency tree at every level; check before admitting the next
        // larger fixture so an exponential regression fails at the small case.
        assert!(measured.allocations <= 512 + 256 * vertices, "{measured:?}");
        assert!(
            measured.requested_bytes <= 32_768 + 36_864 * vertices,
            "{measured:?}"
        );
        assert!(
            measured.retained_bytes <= 4_096 + 5_120 * signed_vertices,
            "{measured:?}"
        );
        assert!(
            measured.peak_working_bytes <= 8_192 + 6_144 * signed_vertices,
            "{measured:?}"
        );
        assert_eq!(parsed.node_shapes.len(), depth + 1);
        for level in 1..=depth {
            let definition = parsed.owner.definition(&id(&format!("S{level}"))).unwrap();
            let [Constraint::And(members)] = definition.constraints.as_slice() else {
                panic!("one original conjunction body");
            };
            assert_eq!(members.len(), 2);
            for member in members {
                assert!(member.owner.is_none());
                assert!(member.property_shapes.is_empty());
                assert!(
                    matches!(member.constraints.as_slice(), [Constraint::Reference(target)] if *target == id(&format!("S{}", level - 1)))
                );
            }
        }
        if depth <= 8 {
            let root = parsed
                .node_shapes
                .iter()
                .find(|root| root.id == id(&format!("S{depth}")))
                .unwrap()
                .clone();
            drop(parsed);
            assert!(conforms(&store, &id("outside"), &root).unwrap());
            assert!(
                !conforms(
                    &store,
                    &Term::Literal(Literal::new_simple_literal("foreign")),
                    &root
                )
                .unwrap()
            );
        }
    }
}

#[test]
fn catalog_references_project_object_and_member_definitions_without_losing_constraints() {
    let parsed = parse_shapes(
        "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .
         ex:Root a sh:NodeShape; sh:targetClass ex:C; sh:property ex:P .
         ex:P sh:path ex:p; sh:minCount 1; sh:maxCount 1; sh:node ex:Leaf .
         ex:Leaf a sh:NodeShape; sh:nodeKind sh:IRI .",
        None,
    )
    .unwrap();
    let ns = purrdf_shapes::json_schema::Namespaces::new(
        "ex",
        &[("ex".to_owned(), "http://example.org/".to_owned())],
    )
    .unwrap();
    let compiled = purrdf_shapes::json_schema::compile(&parsed, &ns).unwrap();
    assert!(
        !compiled
            .losses
            .entries()
            .iter()
            .any(|entry| entry.code == "shape-reference")
    );
    let document = purrdf_lex::json::read(&compiled.schema_json).unwrap();
    let mut registry = purrdf_jsonschema::Registry::with_metaschemas(metaschemas::metaschemas());
    registry
        .add_resource("mem:///catalog.schema.json", document)
        .unwrap();
    let schema = registry
        .compile("mem:///catalog.schema.json#/$defs/C")
        .unwrap();
    let key = ns.compact_iri("http://example.org/p");
    for (value, expected) in [
        ("{\"@id\":\"http://example.org/value\"}", true),
        ("\"wrong\"", false),
    ] {
        let instance = purrdf_lex::json::read(&format!(
            "{{\"@id\":\"http://example.org/focus\",\"{key}\":{value}}}"
        ))
        .unwrap();
        assert_eq!(
            schema.is_valid(&instance).unwrap(),
            expected,
            "{instance:?}"
        );
    }
}

#[test]
fn missing_projection_definitions_return_the_original_native_refusal() {
    let mut parsed = parse_shapes(
        "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .
         ex:Root a sh:NodeShape; sh:targetClass ex:C .",
        None,
    )
    .unwrap();
    let absent = id("Absent");
    parsed.node_shapes[0].constraints = vec![Constraint::Reference(absent.clone())];
    let ns = purrdf_shapes::json_schema::Namespaces::new(
        "ex",
        &[("ex".to_owned(), "http://example.org/".to_owned())],
    )
    .unwrap();
    let positive = purrdf_shapes::json_schema::compile(&parsed, &ns).unwrap_err();
    let mut inner = definition(&parsed, "Root");
    inner.constraints = vec![Constraint::Reference(absent.clone())];
    parsed.node_shapes[0].constraints = vec![Constraint::Not(Box::new(inner))];
    let negative = purrdf_shapes::json_schema::compile(&parsed, &ns).unwrap_err();
    drop(parsed);
    for error in [positive, negative] {
        match error {
            purrdf_shapes::json_schema::SchemaCompileError::Materialization(
                purrdf_shapes::ShapesError::Preparation(
                    purrdf_shapes::PreparationRefusal::UnknownReference(original),
                ),
            ) => assert_eq!(*original, absent),
            other => panic!("original native missing-definition refusal: {other:?}"),
        }
    }
}

#[test]
fn authored_aliases_preserve_shaped_property_coverage() {
    use purrdf_shapes::json_schema::{
        SchemaCompileRequest, SchemaCoverageStatus, SchemaSurfaceMode,
    };
    let mut parsed = parse_shapes(
        "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .
         ex:Root a sh:NodeShape; sh:targetClass ex:C; sh:property ex:P .
         ex:P sh:path ex:p; sh:minCount 1 .",
        None,
    )
    .unwrap();
    let mut root = definition(&parsed, "Root");
    let mut alias = root.clone();
    alias.id = id("Alias");
    alias.targets.clear();
    root.constraints = vec![Constraint::Reference(alias.id.clone())];
    let property = definition(&parsed, "P");
    parsed.owner =
        purrdf_shapes::shapes::ModelOwner::from_definitions(vec![root, alias, property]).unwrap();
    parsed.node_shapes = vec![parsed.owner.root(&id("Root")).unwrap()];
    let ns = purrdf_shapes::json_schema::Namespaces::new(
        "ex",
        &[("ex".to_owned(), "http://example.org/".to_owned())],
    )
    .unwrap();
    let ontology = parse_turtle_to_dataset(
        "@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
         <http://example.org/p> a rdf:Property .",
        None,
    )
    .unwrap();
    let coverage =
        SchemaCompileRequest::new(&parsed, &ns, &ontology, SchemaSurfaceMode::ShapedOnly)
            .coverage_report()
            .unwrap();
    let property = coverage
        .properties
        .iter()
        .find(|property| property.property_iri == "http://example.org/p")
        .unwrap();
    assert!(
        property
            .classes
            .iter()
            .any(|class| class.class_iri == "http://example.org/C"
                && class.status == SchemaCoverageStatus::HasShape)
    );
}

#[test]
fn nested_property_references_preserve_child_focus_path_and_serialized_report() {
    let parsed = parse_shapes(
        "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .
         ex:Root a sh:NodeShape; sh:property ex:P .
         ex:P sh:path ex:p; sh:property ex:Q .
         ex:Q sh:path ex:q; sh:nodeKind sh:IRI .",
        None,
    )
    .unwrap();
    let mut property = definition(&parsed, "P").property_shapes.pop().unwrap();
    property
        .constraints
        .retain(|constraint| !matches!(constraint, Constraint::PropertyReference(_)));
    property.property_shapes = vec![definition(&parsed, "Q").property_shapes.pop().unwrap()];
    let original = inline_root(&parsed, property);
    let exported = exported_root(&parsed);
    drop(parsed);
    let store = data(
        "<http://example.org/focus> <http://example.org/p> <http://example.org/middle> .
        <http://example.org/middle> <http://example.org/q> \"wrong\" .
        <http://example.org/p> <http://example.org/graphBoxRole> <http://example.org/ParentPathRole> .
        <http://example.org/q> <http://example.org/graphBoxRole> <http://example.org/ChildPathRole> .",
    );
    let vocabulary = purrdf_shapes::model::BoxRoleVocab::for_namespace("http://example.org/");
    let expected = validate_shape_with(&store, &id("focus"), &original, Some(&vocabulary)).unwrap();
    let observed = validate_shape_with(&store, &id("focus"), &exported, Some(&vocabulary)).unwrap();
    assert_eq!(expected.len(), 1);
    assert_eq!(observed.len(), expected.len());
    assert_eq!(observed[0].focus_node, id("middle"));
    assert_eq!(observed[0].result_path, Some(id("q")));
    assert_eq!(observed[0].source_shape, id("Q"));
    assert_eq!(
        observed[0].path_box_roles,
        vec![NamedNode::from("http://example.org/ChildPathRole")]
    );
    assert!(!conforms(&store, &id("focus"), &exported).unwrap());
    assert_eq!(
        ValidationReport::from_results(observed, ConformanceDisallows::default())
            .to_ntriples()
            .unwrap(),
        ValidationReport::from_results(expected, ConformanceDisallows::default())
            .to_ntriples()
            .unwrap(),
    );
}

#[test]
fn property_edge_overrides_stay_below_original_constraint_annotations() {
    let parsed = parse_shapes(
        "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .
         ex:Root a sh:NodeShape; sh:property ex:P {| sh:severity sh:Info; sh:message \"edge\" |} .
         ex:P sh:path ex:p; sh:severity sh:Warning;
           sh:nodeKind sh:IRI {| sh:severity sh:Trace; sh:message \"inner\" |} .",
        None,
    )
    .unwrap();
    let mut property = definition(&parsed, "P").property_shapes.pop().unwrap();
    property.severity = Severity::Info;
    property.messages = vec![Literal::new_simple_literal("edge")];
    let original = inline_root(&parsed, property);
    let root = exported_root(&parsed);
    drop(parsed);
    let store = data("<http://example.org/focus> <http://example.org/p> \"wrong\" .");
    let expected = validate_shape(&store, &id("focus"), &original).unwrap();
    let observed = validate_shape(&store, &id("focus"), &root).unwrap();
    assert_eq!(observed.len(), expected.len());
    assert_eq!(observed.len(), 1);
    assert_eq!(observed[0].severity, Severity::Trace);
    assert_eq!(
        observed[0].messages,
        vec![Literal::new_simple_literal("inner")]
    );
    assert!(conforms(&store, &id("focus"), &root).unwrap());
    assert_eq!(
        ValidationReport::from_results(observed, ConformanceDisallows::default())
            .to_ntriples()
            .unwrap(),
        ValidationReport::from_results(expected, ConformanceDisallows::default())
            .to_ntriples()
            .unwrap(),
    );
}

#[test]
fn a_deactivated_property_edge_still_permits_its_path_in_closed_shapes() {
    let parsed = parse_shapes(
        "@prefix sh: <http://www.w3.org/ns/shacl#> . @prefix ex: <http://example.org/> .
         ex:Root a sh:NodeShape; sh:closed true;
           sh:property ex:P {| sh:deactivated true |} .
         ex:P sh:path ex:p; sh:nodeKind sh:IRI .",
        None,
    )
    .unwrap();
    let mut property = definition(&parsed, "P").property_shapes.pop().unwrap();
    property.deactivated = true;
    let original = inline_root(&parsed, property);
    let root = exported_root(&parsed);
    drop(parsed);
    let store = data("<http://example.org/focus> <http://example.org/p> \"allowed\" .");
    assert!(conforms(&store, &id("focus"), &original).unwrap());
    assert!(conforms(&store, &id("focus"), &root).unwrap());
    assert_eq!(
        validate_shape(&store, &id("focus"), &root).unwrap().len(),
        0
    );
}
