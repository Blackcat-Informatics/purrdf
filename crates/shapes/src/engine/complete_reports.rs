// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native proofs of the report mapping laws, independently of dated XPath
//! request routing. These fixtures contain no regex-bearing expressions.

use super::{CompletePreparedValidator, PreparedShapes, parse_shapes};
use crate::profile::ShaclProfile;
use crate::report::{CompleteValidationError, Severity};
use crate::shapes::{AnnotatedConstraint, ConstraintAnnotation};
use crate::term::Literal;
use purrdf_rdf::RdfDatasetBuilder;
use std::sync::Arc;

const PREFIXES: &str =
    "@prefix ex: <http://example.org/> . @prefix sh: <http://www.w3.org/ns/shacl#> .";

fn binding(body: &str) -> CompletePreparedValidator {
    let shapes = parse_shapes(&format!("{PREFIXES}{body}"), None).unwrap();
    PreparedShapes::new(Arc::new(shapes))
        .bind_complete_shared_dataset(RdfDatasetBuilder::new().freeze().unwrap())
        .unwrap()
}

#[test]
fn restored_occurrence_cache_reuses_authenticated_source_and_warm_clone_identity() {
    use crate::product::{HostBindings, ShapesProduct, ShapesProfile};

    let prepared = PreparedShapes::new(Arc::new(
        parse_shapes(
            &format!(
                "{PREFIXES} ex:S a sh:NodeShape; sh:targetNode ex:n; sh:sparql _:source . \
                 _:source sh:select 'SELECT $this WHERE {{}}' ."
            ),
            None,
        )
        .unwrap(),
    ));
    let bytes = prepared.to_product(&ShapesProfile::CORE).unwrap();
    let restored = ShapesProduct::open(&bytes)
        .unwrap()
        .admit(&ShapesProfile::CORE, &HostBindings::empty())
        .unwrap();
    let shapes = restored.shapes();
    let retained = Arc::clone(&shapes.shapes_dataset);
    assert!(shapes.sparql_sources.get().is_none());
    assert_eq!(restored.to_product(&ShapesProfile::CORE).unwrap(), bytes);
    let rebuilt = shapes.report_sources().unwrap();
    let parsed = prepared.shapes().report_sources().unwrap();
    assert_eq!(rebuilt.len(), parsed.len());
    for (shape, slots) in parsed {
        let restored_slots = &rebuilt[shape];
        assert_eq!(slots.len(), restored_slots.len());
        for (slot, occurrence) in slots {
            let crate::shapes::ConstraintOccurrence::Sparql(original) = occurrence else {
                panic!("the fixture declares a SPARQL occurrence")
            };
            let crate::shapes::ConstraintOccurrence::Sparql(restored) = &restored_slots[slot]
            else {
                panic!("the restored occurrence retains its kind")
            };
            assert_eq!(restored.source_constraint, original.source_constraint);
            assert_eq!(restored.select, original.select);
            assert_eq!(restored.messages, original.messages);
            assert_eq!(restored.severity, original.severity);
            assert_eq!(restored.annotations, original.annotations);
        }
    }
    assert!(Arc::ptr_eq(&retained, &shapes.shapes_dataset));
    let warmed = shapes.sparql_sources.get().unwrap();
    let cloned = shapes.as_ref().clone();
    assert!(Arc::ptr_eq(warmed, cloned.sparql_sources.get().unwrap()));
    assert!(std::ptr::eq(
        cloned.report_sources().unwrap(),
        shapes.report_sources().unwrap()
    ));
    assert_eq!(restored.to_product(&ShapesProfile::CORE).unwrap(), bytes);
}

#[test]
fn row_message_precedes_declared_messages_and_preserves_literal_identity() {
    let binding = binding(
        r#"
        ex:S a sh:NodeShape; sh:targetNode ex:n; sh:message "shape"; sh:sparql ex:C .
        ex:C sh:message "constraint"; sh:select
          "SELECT $this ?message ?bound WHERE { BIND('row {$bound} {$missing}'@en AS ?message) BIND('actual' AS ?bound) }" .
        "#,
    );
    assert_eq!(
        binding.validate().unwrap().legacy().results[0].messages[0].value(),
        "constraint"
    );
    for profile in [ShaclProfile::REC_20170720, ShaclProfile::WD_20260918] {
        let report = binding
            .validate_report_with_focus_filter(profile, |_, _| true)
            .unwrap();
        let messages = &report.legacy().results[0].messages;
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].value(), "row actual {$missing}");
        assert_eq!(messages[0].language(), Some("en"));
    }
}

#[test]
fn constraint_messages_precede_only_the_draft_core_fallback() {
    for (declared, rec, draft) in [
        ("sh:message 'constraint';", "constraint", "constraint"),
        ("", "", "shape"),
    ] {
        let binding = binding(&format!(
            "ex:S a sh:NodeShape; sh:targetNode ex:n; sh:message 'shape'; sh:sparql ex:C . \
             ex:C {declared} sh:select 'SELECT $this WHERE {{}}' ."
        ));
        for (profile, expected) in [
            (ShaclProfile::REC_20170720, rec),
            (ShaclProfile::WD_20260918, draft),
        ] {
            let report = binding
                .validate_report_with_focus_filter(profile, |_, _| true)
                .unwrap();
            let messages = &report.legacy().results[0].messages;
            if expected.is_empty() {
                assert_eq!(messages.as_slice(), []);
            } else {
                assert_eq!(messages.len(), 1);
                assert_eq!(messages[0].value(), expected);
            }
        }
    }
}

#[test]
fn selected_row_message_keeps_datatype_language_direction_and_template_bindings() {
    for (expression, expected) in [
        (
            "'row {$bound}'^^<http://example.org/Message>",
            Literal::new_typed_literal(
                "row actual",
                crate::term::NamedNode::new_unchecked("http://example.org/Message"),
            ),
        ),
        (
            "'row {$bound}'@ar--rtl",
            Literal::new_directional_language_tagged_literal_unchecked(
                "row actual",
                "ar",
                purrdf_rdf::RdfTextDirection::Rtl,
            ),
        ),
    ] {
        let binding = binding(&format!(
            "ex:S a sh:NodeShape; sh:targetNode ex:n; sh:sparql ex:C . \
             ex:C sh:message 'constraint'; sh:select \"SELECT $this ?message ?bound WHERE {{ \
             BIND({expression} AS ?message) BIND('actual' AS ?bound) }}\" ."
        ));
        for profile in [ShaclProfile::REC_20170720, ShaclProfile::WD_20260918] {
            let report = binding
                .validate_report_with_focus_filter(profile, |_, _| true)
                .unwrap();
            assert_eq!(
                report.legacy().results[0].messages.as_slice(),
                std::slice::from_ref(&expected)
            );
            assert_eq!(
                report
                    .to_dataset()
                    .owned_quads()
                    .filter(|quad| quad.predicate == crate::model::sh::RESULT_MESSAGE)
                    .map(|quad| quad.object)
                    .collect::<Vec<_>>(),
                [crate::term::Term::Literal(expected.clone()).to_rdf_term()],
            );
        }
    }
}

#[test]
fn component_message_order_is_validator_then_component_then_draft_core() {
    for (validator, component, rec, draft) in [
        (
            "sh:message 'validator';",
            "sh:message 'component';",
            "validator",
            "validator",
        ),
        ("", "sh:message 'component';", "component", "component"),
        ("", "", "", "shape"),
    ] {
        let binding = binding(&format!(
            "ex:Component a sh:ConstraintComponent; {component} \
             sh:parameter [ sh:path ex:param ]; sh:nodeValidator ex:Validator . \
             ex:Validator a sh:SPARQLSelectValidator; {validator} sh:select 'SELECT $this WHERE {{}}' . \
             ex:S a sh:NodeShape; sh:targetNode ex:n; sh:message 'shape'; ex:param 1 ."
        ));
        assert_eq!(
            binding.validate().unwrap().legacy().results[0].messages[0].value(),
            "shape"
        );
        for (profile, expected) in [
            (ShaclProfile::REC_20170720, rec),
            (ShaclProfile::WD_20260918, draft),
        ] {
            let report = binding
                .validate_report_with_focus_filter(profile, |_, _| true)
                .unwrap();
            let messages = &report.legacy().results[0].messages;
            if expected.is_empty() {
                assert_eq!(messages.as_slice(), []);
            } else {
                assert_eq!(messages.len(), 1);
                assert_eq!(messages[0].value(), expected);
            }
            assert_eq!(report.results().next().unwrap().source_constraint(), None);
        }
    }
}

#[test]
fn explicit_solution_failure_is_distinct_from_a_violation_and_resource_trip() {
    let binding = binding(
        r#"
        ex:S a sh:NodeShape; sh:targetNode ex:n; sh:sparql ex:C .
        ex:C sh:select "SELECT $this ?failure ?message WHERE { BIND(true AS ?failure) BIND('message' AS ?message) }" .
        "#,
    );
    assert_eq!(binding.validate().unwrap().results().len(), 1);
    for profile in [ShaclProfile::REC_20170720, ShaclProfile::WD_20260918] {
        let error = binding
            .validate_report_with_focus_filter(profile, |_, _| true)
            .unwrap_err();
        let CompleteValidationError::Semantic(failure) = error else {
            panic!("semantic failure: {error}")
        };
        assert_eq!(failure.focus().to_string(), "<http://example.org/n>");
        assert_eq!(
            failure.source_constraint().unwrap().to_string(),
            "<http://example.org/C>"
        );
        let state = Arc::new(purrdf_sparql_eval::GovernorState::new(
            &purrdf_sparql_eval::QueryGovernors::UNBOUNDED.with_fuel(0),
        ));
        let _scope = crate::sparql::enter_governor_scope(state);
        assert!(matches!(
            binding.validate_report_with_focus_filter(profile, |_, _| true),
            Err(CompleteValidationError::Resource(_))
        ));
    }
}

#[test]
fn rec_core_messages_ignore_draft_reifier_overrides_and_legacy_keeps_them() {
    let mut shapes = parse_shapes(
        &format!("{PREFIXES}ex:S a sh:NodeShape; sh:targetNode ex:n; sh:nodeKind sh:BlankNode; sh:message 'shape' ."),
        None,
    ).unwrap();
    shapes.node_shapes[0].constraint_annotations = vec![ConstraintAnnotation {
        constraint: AnnotatedConstraint::Constraint(0),
        severity: Some(Severity::Info),
        messages: vec![Literal::new_simple_literal("reifier")],
    }];
    let binding = PreparedShapes::new(Arc::new(shapes))
        .bind_complete_shared_dataset(RdfDatasetBuilder::new().freeze().unwrap())
        .unwrap();
    for (profile, message, severity) in [
        (ShaclProfile::LEGACY, "reifier", Severity::Info),
        (ShaclProfile::WD_20260918, "reifier", Severity::Info),
        (ShaclProfile::REC_20170720, "shape", Severity::Violation),
    ] {
        let report = binding
            .validate_report_with_focus_filter(profile, |_, _| true)
            .unwrap();
        let result = &report.legacy().results[0];
        assert_eq!(result.messages[0].value(), message);
        assert_eq!(result.severity, severity);
    }
}

#[test]
fn fresh_query_mint_prefixes_agree_under_serial_and_parallel_chunk_geometries() {
    let foci = (0..129)
        .map(|index| format!("ex:n{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let binding = binding(&format!(
        "ex:S a sh:NodeShape; sh:targetNode {foci}; sh:sparql ex:C . \
         ex:C sh:select 'SELECT $this ?value WHERE {{ BIND(BNODE() AS ?value) }}' ."
    ));
    let mut baseline = None;
    for (parallel, chunk) in [(false, 64), (true, 1), (true, 7), (true, 64)] {
        let _scheduler = crate::parallel::force_scheduler_for_test(parallel, chunk);
        let report = binding.validate().unwrap();
        assert_eq!(report.results().len(), 129);
        let values: purrdf_rdf::FastSet<_> = report
            .legacy()
            .results
            .iter()
            .map(|result| result.value.as_ref().unwrap())
            .collect();
        assert_eq!(values.len(), 129);
        let graph = report.to_dataset().owned_quads().collect::<Vec<_>>();
        if let Some(expected) = &baseline {
            assert_eq!(&graph, expected);
        } else {
            baseline = Some(graph);
        }
    }
}

#[test]
fn typed_parallel_failure_names_the_first_canonical_root_focus() {
    let foci = (0..129)
        .map(|index| format!("ex:n{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let binding = binding(&format!(
        "ex:S a sh:NodeShape; sh:targetNode {foci}; sh:sparql ex:C . \
         ex:C sh:select 'SELECT $this ?failure WHERE {{ BIND(true AS ?failure) }}' ."
    ));
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    pool.install(|| {
        for (parallel, chunk) in [(false, 64), (true, 1), (true, 7), (true, 64)] {
            let _scheduler = crate::parallel::force_scheduler_for_test(parallel, chunk);
            for _ in 0..20 {
                let error = binding
                    .validate_report_with_focus_filter(ShaclProfile::REC_20170720, |_, _| true)
                    .unwrap_err();
                let CompleteValidationError::Semantic(failure) = error else {
                    panic!("typed semantic failure: {error}")
                };
                assert_eq!(failure.focus().to_string(), "<http://example.org/n0>");
            }
        }
    });
}

#[test]
fn equal_values_sort_with_source_constraint_and_recursive_evidence() {
    let binding = binding(
        "ex:S a sh:NodeShape; sh:targetNode ex:n; sh:sparql ex:C, ex:D . \
         ex:C sh:select 'SELECT $this WHERE {}' . \
         ex:D sh:select 'SELECT $this WHERE {}' .",
    );
    let capture =
        crate::report::ReportCapture::new(&binding.validator.shapes, ShaclProfile::LEGACY);
    let acquired = binding
        .validator
        .validate_results_using(
            super::EvidenceReporting(&capture),
            None::<fn(&crate::shapes::Shape, &crate::term::Term) -> bool>,
        )
        .unwrap();
    assert_eq!(acquired.len(), 2);
    assert_eq!(
        crate::report::result_sort_key(&acquired[0].value),
        crate::report::result_sort_key(&acquired[1].value),
    );
    for nested in [false, true] {
        let mut baseline = None;
        for reversed in [false, true] {
            // These are two independently acquired evaluator records whose
            // legacy values agree. Reverse acquisition order before sorting;
            // put the differing evidence in details as the second case.
            let records = (0..2)
                .map(|index| {
                    let original = &acquired[if reversed { 1 - index } else { index }];
                    let mut value = original.value.clone();
                    let mut evidence = original.evidence.clone();
                    if nested {
                        value.details = vec![original.value.clone()];
                        evidence = crate::report::ResultEvidence {
                            source_constraint: None,
                            details: vec![evidence],
                        };
                    }
                    crate::report::ResultRecord { value, evidence }
                })
                .collect();
            let graph = crate::report::CompleteValidationReport::from_records(
                records,
                &binding.validator.shapes,
                Arc::clone(&binding.context),
                ShaclProfile::LEGACY,
            )
            .to_dataset()
            .owned_quads()
            .collect::<Vec<_>>();
            if let Some(expected) = &baseline {
                assert_eq!(&graph, expected, "nested={nested}, reversed={reversed}");
            } else {
                baseline = Some(graph);
            }
        }
    }
}

#[test]
fn failure_selection_keeps_canonical_literal_order_and_untyped_precedence() {
    let shapes = Arc::new(
        parse_shapes(
            &format!("{PREFIXES}ex:S a sh:NodeShape; sh:nodeKind sh:IRI ."),
            None,
        )
        .unwrap(),
    );
    let mut data = RdfDatasetBuilder::new();
    let first = data.intern_literal(purrdf_rdf::RdfLiteral::typed(
        "1",
        crate::model::xsd::STRING,
    ));
    let later = data.intern_literal(purrdf_rdf::RdfLiteral::typed(
        "1",
        crate::model::xsd::INTEGER,
    ));
    let binding = PreparedShapes::new(shapes)
        .bind_complete_shared_dataset(data.freeze().unwrap())
        .unwrap();
    let dataset = binding.validator.data.core_view();
    let first = super::FocusNode::Interned(first);
    let later = super::FocusNode::Interned(later);
    assert_ne!(
        first.to_term(dataset).to_string(),
        later.to_term(dataset).to_string()
    );
    assert_ne!(first.to_term(dataset), later.to_term(dataset));
    let typed = |focus: &super::FocusNode| {
        let local =
            crate::report::ReportCapture::new(&binding.validator.shapes, ShaclProfile::LEGACY);
        local.semantic_failure(&focus.to_term(dataset), None);
        local.take_failure()
    };
    for untyped_first in [false, true] {
        let operation =
            crate::report::ReportCapture::new(&binding.validator.shapes, ShaclProfile::LEGACY);
        operation.record_focus_failure(dataset, &later, typed(&later));
        operation.record_focus_failure(
            dataset,
            &first,
            if untyped_first { None } else { typed(&first) },
        );
        // A later worker recording its failure again cannot replace either
        // the first typed focus or an earlier ordinary execution failure.
        operation.record_focus_failure(dataset, &later, typed(&later));
        if untyped_first {
            assert!(operation.take_failure().is_none());
        } else {
            let error = operation.take_failure().unwrap().into_public();
            let CompleteValidationError::Semantic(failure) = error else {
                panic!("semantic evidence: {error}")
            };
            assert_eq!(failure.focus(), &first.to_term(dataset));
        }
    }
}
