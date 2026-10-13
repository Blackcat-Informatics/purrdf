// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native occurrence, recursive-detail and source-acquisition report proofs.

#[path = "support/turtle.rs"]
mod turtle;

use std::sync::Arc;

use purrdf_iri::vocab::sh;
use purrdf_shapes::engine::{PreparedShapes, validate_complete_dataset, validate_dataset};
use purrdf_shapes::report::CompleteValidationError;
use purrdf_shapes::shapes::{Constraint, Shapes};
use purrdf_shapes::term::Term;

const PREFIXES: &str =
    "@prefix ex: <http://example.org/> . @prefix sh: <http://www.w3.org/ns/shacl#> .";

#[global_allocator]
static ALLOCATOR: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;

fn shapes(text: &str) -> Arc<Shapes> {
    Arc::new(turtle::loads(PREFIXES, text))
}

fn property_count(dataset: &purrdf_rdf::RdfDataset, property: &str) -> usize {
    dataset
        .owned_quads()
        .filter(|quad| quad.predicate == property)
        .count()
}

#[test]
fn equal_queries_retain_distinct_actual_source_constraints() {
    let shapes = shapes(
        r#"
        ex:S a sh:NodeShape; sh:targetNode ex:n; sh:sparql ex:First, _:second .
        ex:First sh:select "SELECT $this WHERE {}" .
        _:second sh:select "SELECT $this WHERE {}" .
    "#,
    );
    let data = turtle::data(PREFIXES, "");
    let legacy = validate_dataset(&data, &shapes).unwrap();
    let complete = validate_complete_dataset(Arc::clone(&data), Arc::clone(&shapes)).unwrap();
    assert_eq!(complete.results().len(), 2);
    let mut sources: Vec<_> = complete
        .results()
        .map(|result| result.source_constraint().unwrap().to_string())
        .collect();
    sources.sort();
    assert_eq!(sources, ["<http://example.org/First>", "_:second"]);
    assert_eq!(
        property_count(&complete.to_dataset(), sh::SOURCE_CONSTRAINT),
        2
    );
    assert_eq!(
        property_count(&legacy.to_dataset(), sh::SOURCE_CONSTRAINT),
        0
    );
    assert_eq!(legacy.to_ntriples(), complete.legacy().to_ntriples());
    assert_eq!(
        legacy.to_ntriples(),
        validate_dataset(&data, &shapes).unwrap().to_ntriples()
    );
}

#[test]
fn independent_equal_blank_labels_have_distinct_source_acquisitions() {
    let shapes = shapes(
        r#"
        _:same a sh:NodeShape; sh:targetSubjectsOf ex:p; sh:sparql ex:C .
        ex:C sh:select "SELECT $this WHERE {}" .
    "#,
    );
    let data = turtle::data(PREFIXES, "_:same ex:p 1 .");
    let report = validate_complete_dataset(data, shapes).unwrap();
    let result = report.results().next().unwrap();
    let Term::BlankNode(focus) = &result.legacy().focus_node else {
        panic!("blank focus")
    };
    let Term::BlankNode(shape) = &result.legacy().source_shape else {
        panic!("blank source shape")
    };
    assert_ne!(focus, shape);
    let context = report.source_context();
    let focus = context.source_blank(focus).unwrap();
    let shape = context.source_blank(shape).unwrap();
    assert_eq!(focus.label(), "same");
    assert_eq!(shape.label(), "same");
    assert_eq!(focus.scope(), shape.scope());
    assert_ne!(focus, shape);
    assert_eq!(focus.source(), context.data());
    assert_eq!(shape.source(), context.shapes());
    assert!(!context.sources_share_identity());
    let graph = report.to_graph();
    let identities: Vec<_> = graph
        .dataset()
        .owned_quads()
        .filter(|quad| matches!(quad.predicate.as_str(), sh::FOCUS_NODE | sh::SOURCE_SHAPE))
        .map(|quad| {
            let purrdf_rdf::RdfTerm::BlankNode(label) = quad.object else {
                panic!("blank report term")
            };
            graph.blank_labels().source_of(&label).unwrap().clone()
        })
        .collect();
    assert_eq!(identities.len(), 2);
    assert_ne!(identities[0], identities[1]);
}

#[test]
fn deliberate_shared_source_retains_shared_blank_identity() {
    let shapes = shapes(
        r#"
        _:same a sh:NodeShape; sh:targetSubjectsOf ex:p; sh:sparql ex:C; ex:p 1 .
        ex:C sh:select "SELECT $this WHERE {}" .
    "#,
    );
    let data = Arc::clone(shapes.dataset());
    let report = validate_complete_dataset(data, shapes).unwrap();
    assert!(report.source_context().sources_share_identity());
    let result = report.results().next().unwrap();
    assert_eq!(result.legacy().focus_node, result.legacy().source_shape);
    let Term::BlankNode(label) = &result.legacy().focus_node else {
        panic!("shared blank")
    };
    assert_eq!(
        report
            .source_context()
            .source_blank(label)
            .unwrap()
            .source(),
        report.source_context().data()
    );
}

#[test]
fn equal_graph_bytes_from_independent_acquisitions_do_not_share_identity() {
    let text = r#"
        _:same a sh:NodeShape; sh:targetSubjectsOf ex:p; sh:sparql ex:C; ex:p 1 .
        ex:C sh:select "SELECT $this WHERE {}" .
    "#;
    let shapes = shapes(text);
    let data = turtle::data(PREFIXES, text);
    let report = validate_complete_dataset(data, shapes).unwrap();
    assert!(!report.source_context().sources_share_identity());
    let result = report.results().next().unwrap();
    assert_ne!(result.legacy().focus_node, result.legacy().source_shape);
}

#[test]
fn native_data_blanks_keep_scoped_identity_without_shapes_blanks() {
    let mut data = purrdf_rdf::RdfDatasetBuilder::new();
    let subject = data.intern_blank("original", purrdf_rdf::BlankScope(48));
    let predicate = data.intern_iri("http://example.org/p");
    let object = data.intern_iri("http://example.org/o");
    data.push_quad(subject, predicate, object, None);
    let report = validate_complete_dataset(
        data.freeze().unwrap(),
        shapes("ex:S a sh:NodeShape; sh:targetSubjectsOf ex:p; sh:nodeKind sh:IRI ."),
    )
    .unwrap();
    assert_eq!(report.results().len(), 1);
    let Term::BlankNode(label) = &report.legacy().results[0].focus_node else {
        panic!("blank focus")
    };
    let original = report.source_context().source_blank(label).unwrap();
    assert_eq!(original.label(), "original");
    assert_eq!(original.scope(), purrdf_rdf::BlankScope(48));
    assert_eq!(original.source(), report.source_context().data());
    assert_ne!(original.source(), report.source_context().shapes());
}

#[test]
fn nested_source_refusal_survives_a_later_conforming_some_value() {
    let original = shapes(
        r#"
        ex:S a sh:NodeShape; sh:targetNode ex:n;
            sh:property [ sh:path ex:p; sh:someValue [
                sh:or ( [ sh:class ex:Duck ] ex:Query )
            ] ] .
        ex:Query sh:sparql ex:C .
        ex:C sh:select "SELECT $this WHERE {}" .
    "#,
    );
    let data = turtle::data(
        PREFIXES,
        "ex:n ex:p ex:aBad, ex:zGood . ex:zGood a ex:Duck .",
    );
    let mut changed = (*original).clone();
    let target = changed
        .node_shapes
        .iter_mut()
        .find(|shape| shape.id.to_string() == "<http://example.org/S>")
        .expect("the actual target shape");
    let Constraint::SomeValue(candidate) = &mut target.property_shapes[0].constraints[0] else {
        panic!("the actual SomeValue child")
    };
    let Constraint::Or(members) = &mut candidate.constraints[0] else {
        panic!("the actual candidate alternatives")
    };
    let Constraint::Sparql { select, .. } = &mut members[1].constraints[0] else {
        panic!("the actual query occurrence")
    };
    select.push_str(" LIMIT 1");
    let changed = PreparedShapes::new(Arc::new(changed))
        .bind_complete_shared_dataset(Arc::clone(&data))
        .unwrap();
    for _ in 0..2 {
        let error = changed.validate().unwrap_err();
        let CompleteValidationError::SourceConstraint(refusal) = error else {
            panic!("the hard source refusal cannot become conformance: {error:?}")
        };
        assert_eq!(refusal.shape().to_string(), "<http://example.org/Query>");
        assert!(!refusal.is_property());
        assert_eq!(refusal.index(), 0);
    }
    let neighbour = PreparedShapes::new(original)
        .bind_complete_shared_dataset(data)
        .unwrap();
    let report = neighbour.validate().unwrap();
    assert!(report.legacy().conforms);
    assert_eq!(report.results().len(), 0);
}

#[test]
fn changed_constraint_cannot_inherit_a_source_occurrence() {
    let original = shapes(
        r#"
        ex:S a sh:NodeShape; sh:targetNode ex:n; sh:sparql ex:C .
        ex:C sh:select "SELECT $this WHERE {}" .
    "#,
    );
    let mut changed = (*original).clone();
    let Constraint::Sparql { select, .. } = &mut changed.node_shapes[0].constraints[0] else {
        panic!("SPARQL constraint")
    };
    select.push_str(" LIMIT 1");
    let error =
        validate_complete_dataset(turtle::data(PREFIXES, ""), Arc::new(changed)).unwrap_err();
    let CompleteValidationError::SourceConstraint(refusal) = error else {
        panic!("typed source refusal: {error}")
    };
    assert_eq!(refusal.shape().to_string(), "<http://example.org/S>");
    assert!(!refusal.is_property());
    assert_eq!(refusal.index(), 0);
    assert_eq!(
        validate_complete_dataset(turtle::data(PREFIXES, ""), original)
            .unwrap()
            .results()
            .len(),
        1
    );
}

#[test]
fn result_multiplicity_survives_complete_collection_and_emission() {
    let shapes = shapes(
        r#"
        ex:S a sh:NodeShape; sh:targetNode ex:n; sh:sparql ex:C .
        ex:C sh:select "SELECT $this WHERE { { BIND(1 AS ?x) } UNION { BIND(1 AS ?x) } }" .
    "#,
    );
    let report = validate_complete_dataset(turtle::data(PREFIXES, ""), shapes).unwrap();
    assert_eq!(report.results().len(), 2);
    assert_eq!(property_count(&report.to_dataset(), sh::RESULT), 2);
    assert_eq!(
        property_count(&report.to_dataset(), sh::SOURCE_CONSTRAINT),
        2
    );
}

#[test]
fn duplicate_public_root_identity_is_refused_before_complete_binding() {
    let original = shapes("ex:S a sh:NodeShape; sh:targetNode ex:n; sh:nodeKind sh:BlankNode .");
    let mut changed = (*original).clone();
    changed.node_shapes.push(changed.node_shapes[0].clone());
    assert!(matches!(
        PreparedShapes::new(Arc::new(changed))
            .bind_complete_shared_dataset(turtle::data(PREFIXES, "")),
        Err(CompleteValidationError::SourceContext(_))
    ));
    assert_eq!(
        validate_complete_dataset(turtle::data(PREFIXES, ""), original)
            .unwrap()
            .results()
            .len(),
        1
    );
}

#[test]
fn recursive_details_keep_their_own_source_constraints() {
    let shapes = shapes(
        r#"
        ex:S a sh:NodeShape; sh:targetNode ex:n; sh:property [ sh:path ex:p; sh:memberShape [ sh:sparql ex:C ] ] .
        ex:C sh:select "SELECT $this WHERE {}" .
    "#,
    );
    let report =
        validate_complete_dataset(turtle::data(PREFIXES, "ex:n ex:p (1 2) ."), shapes).unwrap();
    assert_eq!(report.results().len(), 1);
    let result = report.results().next().unwrap();
    assert_eq!(result.source_constraint(), None);
    assert_eq!(result.details().len(), 2);
    for detail in result.details() {
        assert_eq!(
            detail.source_constraint().unwrap().to_string(),
            "<http://example.org/C>"
        );
    }
    let graph = report.to_dataset();
    assert_eq!(property_count(&graph, sh::DETAIL), 2);
    assert_eq!(property_count(&graph, sh::SOURCE_CONSTRAINT), 2);
}

#[test]
fn prepared_binding_and_egress_retain_sources_after_input_drop() {
    let shapes = shapes(
        r#"
        _:report a sh:NodeShape; sh:targetSubjectsOf ex:p; sh:sparql _:r0 .
        _:r0 sh:select "SELECT $this WHERE {}" .
    "#,
    );
    let data = turtle::data(PREFIXES, "_:report ex:p 1 .");
    let binding = PreparedShapes::new(Arc::clone(&shapes))
        .bind_complete_shared_dataset(Arc::clone(&data))
        .unwrap();
    drop(data);
    drop(shapes);
    let first = binding.validate().unwrap();
    let second = binding.validate_with_focus_filter(|_, _| true).unwrap();
    assert_eq!(first.legacy().to_ntriples(), second.legacy().to_ntriples());
    let graph = first.to_graph();
    drop(binding);
    drop(first);
    assert_eq!(graph.source_context().data().dataset().quad_count(), 1);
    let Term::BlankNode(root) = graph.root() else {
        panic!("report root")
    };
    assert_eq!(graph.blank_labels().source_of(root), None);
    assert_eq!(property_count(graph.dataset(), sh::RESULT), 1);
    assert_eq!(property_count(graph.dataset(), sh::SOURCE_CONSTRAINT), 1);
}

#[test]
fn complete_governor_refusal_is_typed_and_each_call_has_fresh_fuel() {
    let binding = PreparedShapes::new(shapes(
        r#"
        ex:S a sh:NodeShape; sh:targetNode ex:n; sh:sparql ex:C .
        ex:C sh:select "SELECT $this WHERE {}" .
    "#,
    ))
    .bind_complete_shared_dataset(turtle::data(PREFIXES, ""))
    .unwrap();
    let zero = purrdf_sparql_eval::QueryGovernors::UNBOUNDED.with_fuel(0);
    for _ in 0..2 {
        let error = binding.validate_with_governors(&zero).unwrap_err();
        let CompleteValidationError::Resource(refusal) = error else {
            panic!("typed resource refusal: {error}")
        };
        assert!(matches!(
            refusal.tripped(),
            purrdf_core::TrippedGovernor::Budget {
                dimension: purrdf_core::ResourceDimension::Fuel,
                ..
            }
        ));
        let report = binding
            .validate_with_governors(&purrdf_sparql_eval::QueryGovernors::UNBOUNDED)
            .unwrap();
        assert_eq!(report.results().len(), 1);
        assert!(
            report
                .results()
                .next()
                .unwrap()
                .source_constraint()
                .is_some()
        );
    }
}

#[test]
fn complete_core_validation_spends_no_sparql_fuel() {
    let binding = PreparedShapes::new(shapes(
        "ex:S a sh:NodeShape; sh:targetNode ex:n; sh:nodeKind sh:BlankNode .",
    ))
    .bind_complete_shared_dataset(turtle::data(PREFIXES, ""))
    .unwrap();
    let report = binding
        .validate_with_governors(&purrdf_sparql_eval::QueryGovernors::UNBOUNDED.with_fuel(0))
        .unwrap();
    assert_eq!(report.results().len(), 1);
}

#[test]
fn bounded_complete_routes_use_the_same_source_evidence_and_refuse_foreign_ids() {
    let prepared = PreparedShapes::new(shapes(
        r#"
        ex:S a sh:NodeShape; sh:targetSubjectsOf ex:p; sh:sparql _:constraint .
        _:constraint sh:select "SELECT $this WHERE {}" .
    "#,
    ));
    let data = turtle::data(PREFIXES, "ex:n ex:p 1 . ex:other ex:p 2 .");
    let binding = prepared
        .bind_complete_shared_dataset(Arc::clone(&data))
        .unwrap();
    let foreign = prepared.bind_complete_shared_dataset(data).unwrap();
    let focus = Term::NamedNode(purrdf_shapes::term::NamedNode::new_unchecked(
        "http://example.org/n",
    ));
    let id = binding.legacy_binding().term_id(&focus).unwrap();
    let terms = binding
        .validate_focus_nodes(&[focus.clone(), focus.clone()])
        .unwrap();
    let ids = binding.validate_focus_node_ids(&[id, id]).unwrap();
    assert_eq!(terms.results().len(), 1);
    assert_eq!(
        terms.to_dataset().owned_quads().collect::<Vec<_>>(),
        ids.to_dataset().owned_quads().collect::<Vec<_>>()
    );
    assert_eq!(
        terms.results().next().unwrap().source_constraint(),
        Some(&Term::BlankNode("constraint".to_owned()))
    );
    assert!(matches!(
        foreign.validate_focus_node_ids(&[id]),
        Err(CompleteValidationError::Shapes(_))
    ));
    let valid = foreign.legacy_binding().term_id(&focus).unwrap();
    assert_eq!(
        foreign
            .validate_focus_node_ids(&[valid])
            .unwrap()
            .results()
            .len(),
        1
    );
}

#[test]
fn query_minted_blanks_cannot_impersonate_retained_shape_blanks() {
    let report = validate_complete_dataset(
        turtle::data(PREFIXES, ""),
        shapes(
            r#"
            ex:S a sh:NodeShape; sh:targetNode ex:n; sh:sparql _:bnode1 .
            _:bnode1 sh:select "SELECT $this ?value WHERE { BIND(BNODE() AS ?value) }" .
        "#,
        ),
    )
    .unwrap();
    let result = report.results().next().unwrap();
    let Some(Term::BlankNode(label)) = &result.legacy().value else {
        panic!("query minted a blank value")
    };
    assert_ne!(label, "bnode1");
    assert_eq!(report.source_context().source_blank(label), None);
    let graph = report.to_graph();
    let sources: Vec<_> = graph
        .dataset()
        .owned_quads()
        .filter(|quad| matches!(quad.predicate.as_str(), sh::VALUE | sh::SOURCE_CONSTRAINT))
        .map(|quad| {
            let purrdf_rdf::RdfTerm::BlankNode(label) = quad.object else {
                panic!("blank report term")
            };
            (
                quad.predicate,
                graph.blank_labels().source_of(&label).cloned(),
            )
        })
        .collect();
    assert_eq!(sources.len(), 2);
    assert!(
        sources
            .iter()
            .any(|(predicate, source)| predicate == sh::VALUE && source.is_none())
    );
    assert!(
        sources
            .iter()
            .any(|(predicate, source)| predicate == sh::SOURCE_CONSTRAINT && source.is_some())
    );
}

#[test]
fn isomorphic_report_only_blanks_do_not_authorize_changed_source_identity() {
    let original =
        shapes("_:original a sh:NodeShape; sh:targetNode ex:n; sh:nodeKind sh:BlankNode .");
    let mut changed = (*original).clone();
    changed.node_shapes[0].id = Term::BlankNode("unheld".to_owned());
    let data = turtle::data(PREFIXES, "");
    let legacy = validate_dataset(&data, &changed).unwrap();
    assert_eq!(legacy.results.len(), 1);
    assert!(matches!(
        validate_complete_dataset(Arc::clone(&data), Arc::new(changed)),
        Err(CompleteValidationError::SourceContext(_))
    ));
    let valid = validate_complete_dataset(data, original).unwrap();
    assert_eq!(valid.results().len(), 1);
    assert!(valid.source_context().source_blank("original").is_some());
}

#[test]
fn nested_rdf12_blanks_keep_independent_original_scopes() {
    use purrdf_rdf::{BlankScope, RdfDatasetBuilder};
    let mut data = RdfDatasetBuilder::new();
    let subject = data.intern_iri("http://example.org/n");
    let predicate = data.intern_iri("http://example.org/p");
    let inner_subject = data.intern_blank("same", BlankScope(4));
    let inner_object = data.intern_blank("same", BlankScope(5));
    let triple = data.intern_triple(inner_subject, predicate, inner_object);
    let graph = data.intern_iri("http://example.org/partition");
    data.push_quad(subject, predicate, triple, Some(graph));
    let report = validate_complete_dataset(
        data.freeze().unwrap(),
        shapes("ex:S a sh:NodeShape; sh:targetNode ex:n; sh:property [ sh:path ex:p; sh:nodeKind sh:IRI ] ."),
    ).unwrap();
    assert_eq!(report.results().len(), 1);
    let Some(Term::Triple(triple)) = &report.legacy().results[0].value else {
        panic!("nested triple value")
    };
    let mut scopes = Vec::new();
    for term in [&triple.subject, &triple.object] {
        let Term::BlankNode(label) = term else {
            panic!("nested blank")
        };
        let source = report.source_context().source_blank(label).unwrap();
        assert_eq!(source.label(), "same");
        assert_eq!(source.source(), report.source_context().data());
        scopes.push(source.scope().ordinal());
    }
    assert_eq!(scopes, [4, 5]);
    let graph = report.to_graph();
    let values: Vec<_> = graph
        .dataset()
        .owned_quads()
        .filter(|quad| quad.predicate == sh::VALUE)
        .collect();
    assert_eq!(values.len(), 1);
    let purrdf_rdf::RdfTerm::Triple(triple) = &values[0].object else {
        panic!("carried triple")
    };
    let mut scopes = Vec::new();
    for term in [&triple.subject, &triple.object] {
        let purrdf_rdf::RdfTerm::BlankNode(label) = term else {
            panic!("carried nested blank")
        };
        scopes.push(
            graph
                .blank_labels()
                .source_of(label)
                .unwrap()
                .scope()
                .ordinal(),
        );
    }
    assert_eq!(scopes, [4, 5]);
}

#[test]
fn sequence_inverse_path_nodes_are_minted_without_source_correspondence() {
    let report = validate_complete_dataset(
        turtle::data(PREFIXES, "ex:n ex:p ex:middle . ex:other ex:q ex:middle ."),
        shapes("ex:S a sh:NodeShape; sh:targetNode ex:n; sh:property [ sh:path (ex:p [ sh:inversePath ex:q ]); sh:nodeKind sh:BlankNode ] ."),
    ).unwrap();
    assert_eq!(report.results().len(), 1);
    assert!(report.legacy().results[0].path_structure.is_some());
    assert_eq!(
        report.legacy().results[0]
            .value
            .as_ref()
            .unwrap()
            .to_string(),
        "<http://example.org/other>"
    );
    let graph = report.to_graph();
    assert_eq!(property_count(graph.dataset(), sh::INVERSE_PATH), 1);
    assert_eq!(
        property_count(graph.dataset(), purrdf_iri::vocab::rdf::FIRST),
        2
    );
    assert_eq!(
        property_count(graph.dataset(), purrdf_iri::vocab::rdf::REST),
        2
    );
    for quad in graph
        .dataset()
        .owned_quads()
        .filter(|quad| matches!(quad.predicate.as_str(), sh::RESULT_PATH | sh::INVERSE_PATH))
    {
        if let purrdf_rdf::RdfTerm::BlankNode(label) = quad.object {
            assert_eq!(graph.blank_labels().source_of(&label), None);
        }
    }
}

#[test]
fn restored_product_reconstructs_evidence_without_changing_carried_bytes() {
    use purrdf_shapes::product::{HostBindings, ShapesProduct, ShapesProfile};
    let prepared = PreparedShapes::new(shapes(
        r#"
        _:report a sh:NodeShape; sh:targetNode ex:n; sh:sparql _:r0 .
        _:r0 sh:select "SELECT $this WHERE {}" .
        "#,
    ));
    let bytes = prepared.to_product(&ShapesProfile::CORE).unwrap();
    let restored = ShapesProduct::open(&bytes)
        .unwrap()
        .admit(&ShapesProfile::CORE, &HostBindings::empty())
        .unwrap();
    assert_eq!(restored.to_product(&ShapesProfile::CORE).unwrap(), bytes);
    let data = turtle::data(PREFIXES, "");
    let original = prepared
        .bind_complete_shared_dataset(Arc::clone(&data))
        .unwrap()
        .validate()
        .unwrap();
    let binding = restored.bind_complete_shared_dataset(data).unwrap();
    for _ in 0..2 {
        let report = binding.validate().unwrap();
        assert_eq!(
            report.to_dataset().owned_quads().collect::<Vec<_>>(),
            original.to_dataset().owned_quads().collect::<Vec<_>>()
        );
        assert_eq!(
            report.results().next().unwrap().source_constraint(),
            Some(&Term::BlankNode("r0".to_owned()))
        );
        assert_eq!(restored.to_product(&ShapesProfile::CORE).unwrap(), bytes);
    }
    assert_eq!(restored.to_product(&ShapesProfile::CORE).unwrap(), bytes);
}

#[test]
fn identical_fresh_blank_spellings_from_four_query_acquisitions_remain_distinct() {
    let shapes = shapes(
        r#"
        ex:S a sh:NodeShape; sh:targetSubjectsOf ex:p; sh:sparql ex:First, ex:Second .
        ex:First sh:select "SELECT $this ?value WHERE { BIND(BNODE() AS ?value) }" .
        ex:Second sh:select "SELECT $this ?value WHERE { BIND(BNODE() AS ?value) }" .
        "#,
    );
    let data = turtle::data(PREFIXES, "ex:a ex:p 1 . ex:b ex:p 1 .");
    // Compatibility remains unchanged, including the old per-query spelling.
    let legacy = validate_dataset(&data, &shapes).unwrap();
    let legacy_values: purrdf_rdf::FastSet<_> = legacy
        .results
        .iter()
        .map(|r| r.value.as_ref().unwrap())
        .collect();
    assert_eq!(legacy.results.len(), 4);
    assert_eq!(legacy_values.len(), 1);
    let binding = PreparedShapes::new(shapes)
        .bind_complete_shared_dataset(data)
        .unwrap();
    for _ in 0..4 {
        let report = binding.validate().unwrap();
        let values: purrdf_rdf::FastSet<_> = report
            .legacy()
            .results
            .iter()
            .map(|r| r.value.as_ref().unwrap())
            .collect();
        assert_eq!(values.len(), 4);
        for value in values {
            let Term::BlankNode(label) = value else {
                panic!("fresh query value")
            };
            assert!(report.source_context().source_blank(label).is_none());
        }
        let graph = report.to_graph();
        let values: std::collections::BTreeSet<_> = graph
            .dataset()
            .owned_quads()
            .filter(|quad| quad.predicate == sh::VALUE)
            .map(|quad| quad.object.to_string())
            .collect();
        assert_eq!(values.len(), 4);
        assert_eq!(property_count(graph.dataset(), sh::SOURCE_CONSTRAINT), 4);
    }
}

#[test]
fn repeated_detail_query_acquisitions_mint_distinct_blanks() {
    let report = validate_complete_dataset(
        turtle::data(PREFIXES, "ex:n ex:p (1); ex:q (1) ."),
        shapes(r#"
            ex:S a sh:NodeShape; sh:targetNode ex:n; sh:property [ sh:path ex:p; sh:memberShape ex:M ], [ sh:path ex:q; sh:memberShape ex:M ] .
            ex:M sh:sparql ex:C .
            ex:C sh:select "SELECT $this ?value WHERE { BIND(BNODE() AS ?value) }" .
        "#),
    ).unwrap();
    assert_eq!(report.results().len(), 2);
    let details: Vec<_> = report
        .results()
        .flat_map(|result| result.details())
        .collect();
    assert_eq!(details.len(), 2);
    assert_ne!(details[0].legacy().value, details[1].legacy().value);
    assert_eq!(
        details[0].source_constraint(),
        details[1].source_constraint()
    );
}

#[test]
fn fresh_query_identity_is_stable_across_focus_execution() {
    let mut data = purrdf_rdf::RdfDatasetBuilder::new();
    let p = data.intern_iri("http://example.org/p");
    let o = data.intern_iri("http://example.org/o");
    for index in 0..129 {
        let s = data.intern_iri(&format!("http://example.org/n{index}"));
        data.push_quad(s, p, o, None);
    }
    let binding = PreparedShapes::new(shapes(
        r#"
        ex:S a sh:NodeShape; sh:targetSubjectsOf ex:p; sh:sparql ex:C .
        ex:C sh:select "SELECT $this ?value WHERE { BIND(BNODE() AS ?value) }" .
    "#,
    ))
    .bind_complete_shared_dataset(data.freeze().unwrap())
    .unwrap();
    let first = binding
        .validate()
        .unwrap()
        .to_dataset()
        .owned_quads()
        .collect::<Vec<_>>();
    for _ in 0..5 {
        let report = binding.validate().unwrap();
        assert_eq!(report.results().len(), 129);
        let values: purrdf_rdf::FastSet<_> = report
            .legacy()
            .results
            .iter()
            .map(|r| r.value.as_ref().unwrap())
            .collect();
        assert_eq!(values.len(), 129);
        assert_eq!(report.to_dataset().owned_quads().collect::<Vec<_>>(), first);
    }
}

#[test]
fn warm_complete_core_allocation_count_has_no_per_focus_growth() {
    let mut data = purrdf_rdf::RdfDatasetBuilder::new();
    let p = data.intern_iri("http://example.org/p");
    let o = data.intern_iri("http://example.org/o");
    let mut terms = Vec::new();
    for index in 0..32 {
        let iri = format!("http://example.org/n{index}");
        let s = data.intern_iri(&iri);
        data.push_quad(s, p, o, None);
        terms.push(Term::NamedNode(
            purrdf_shapes::term::NamedNode::new_unchecked(iri),
        ));
    }
    let binding = PreparedShapes::new(shapes(
        "ex:S a sh:NodeShape; sh:targetSubjectsOf ex:p; sh:nodeKind sh:IRI .",
    ))
    .bind_complete_shared_dataset(data.freeze().unwrap())
    .unwrap();
    let ids: Vec<_> = terms
        .iter()
        .map(|term| binding.legacy_binding().term_id(term).unwrap())
        .collect();
    let mut counts = Vec::new();
    // Both requests stay below the production parallel threshold. The window
    // therefore observes every allocation of the measured validation, while
    // unrelated libtest threads cannot contaminate this thread's ledger.
    for requested in [&ids[..16], &ids[..32]] {
        assert!(
            binding
                .validate_focus_node_ids(requested)
                .unwrap()
                .legacy()
                .conforms
        );
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let report = binding.validate_focus_node_ids(requested).unwrap();
        let measurement = window.close();
        assert!(report.legacy().conforms);
        assert_eq!(report.results().len(), 0);
        counts.push(measurement.allocations);
    }
    assert_eq!(counts[0], counts[1]);
    eprintln!(
        "complete Core warm focus allocations: N=16 {}, N=32 {}",
        counts[0], counts[1]
    );
}

#[test]
fn projected_and_shapes_graph_bindings_retain_actual_source_identity() {
    let graph = shapes(
        r#"
        _:shape a sh:NodeShape; sh:targetNode ex:n; sh:sparql ex:C; ex:marker _:authored .
        ex:C sh:select "SELECT $this ?value WHERE { GRAPH $shapesGraph { $currentShape <http://example.org/marker> ?value } }" .
    "#,
    );
    let prepared = PreparedShapes::new(graph);
    let data = turtle::data(PREFIXES, "");
    let binding = prepared
        .bind_complete_shared_dataset_with_shapes_graph(
            Arc::clone(&data),
            Some("http://example.org/shapes"),
        )
        .unwrap();
    let report = binding.validate().unwrap();
    assert_eq!(report.results().len(), 1);
    let Some(Term::BlankNode(label)) = &report.legacy().results[0].value else {
        panic!("shapes graph blank value")
    };
    let source = report.source_context().source_blank(label).unwrap();
    assert_eq!(source.label(), "authored");
    assert_eq!(source.source(), report.source_context().shapes());
    let core = PreparedShapes::new(shapes(
        "ex:S a sh:NodeShape; sh:targetNode ex:n; sh:nodeKind sh:BlankNode .",
    ));
    let projected = core
        .bind_complete_projected_dataset(data)
        .unwrap()
        .validate()
        .unwrap();
    assert_eq!(projected.results().len(), 1);
}
