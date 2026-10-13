// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Complete graph/source payloads and refusal status through the shared Rust door.

#![cfg(not(target_arch = "wasm32"))]

use std::sync::Arc;

use purrdf_core::xsd_regex::xpath::{Limits, Profile, Resource};
use purrdf_lex::json::{self, Value};
use purrdf_shapes::ShaclProfile;
use purrdf_shapes::engine::{PreparedShapes, ValidationOptions};
use purrdf_shapes::product::ShapesProfile;
use purrdf_validate::{
    CompleteValidationStatus, SarifOptions, complete_profile, complete_report_payload,
    complete_report_to_json, complete_report_to_sarif_string, complete_validation_status,
    validate_complete_documents, validate_complete_product, validate_complete_sources,
};

const SHAPES: &str = r#"
    @prefix sh: <http://www.w3.org/ns/shacl#> .
    @prefix ex: <http://example.org/> .
    _:same a sh:NodeShape; sh:targetSubjectsOf ex:p; sh:sparql _:constraint .
    _:constraint sh:select "SELECT $this WHERE {}"; sh:message "source message" .
"#;
const DATA: &str = "_:same <http://example.org/p> \"1\" .";

#[test]
fn full_payload_keeps_report_root_and_distinct_authored_blank_domains() {
    let options = ValidationOptions::default().with_profile(ShaclProfile::REC_20170720);
    let outcome = validate_complete_documents(SHAPES, None, DATA, &options, &[]);
    assert_eq!(
        complete_validation_status(&outcome),
        CompleteValidationStatus::Nonconforms
    );
    let report = outcome.unwrap();
    let payload = complete_report_payload(&report).unwrap();
    assert_eq!(
        payload["profile"].as_str(),
        Some(ShaclProfile::REC_20170720.id())
    );
    assert_eq!(payload["status"].as_str(), Some("nonconforms"));
    assert_eq!(
        payload["root"].as_str(),
        Some(report.to_graph().root().to_string().as_str())
    );
    assert_eq!(payload["sources"].as_array().unwrap().len(), 2);
    let correspondence = payload["correspondence"].as_array().unwrap();
    assert_eq!(correspondence.len(), 3);
    let same: Vec<_> = correspondence
        .iter()
        .filter(|row| row["label"].as_str() == Some("same"))
        .collect();
    assert_eq!(same.len(), 2);
    assert_ne!(same[0]["source"], same[1]["source"]);
    assert_ne!(same[0]["reportLabel"], same[1]["reportLabel"]);
    let graph = payload["graph"].as_str().unwrap();
    assert!(graph.contains("sourceConstraint"));
    assert!(graph.contains("source message"));
    let text = complete_report_to_json(&report).unwrap();
    assert_eq!(json::read(&text).unwrap(), payload);
    assert_eq!(complete_report_to_json(&report).unwrap(), text);
    let sarif =
        json::read(&complete_report_to_sarif_string(&report, &SarifOptions::default()).unwrap())
            .unwrap();
    assert_eq!(
        sarif["runs"][0]["properties"]["shaclCompleteReport"],
        payload
    );
    assert_eq!(sarif["runs"][0]["results"].as_array().unwrap().len(), 1);
}

#[test]
fn deliberate_shared_source_alias_is_serialized_once_with_both_roles() {
    let text = format!("{SHAPES}\n_:same <http://example.org/p> 1 .");
    let shapes = Arc::new(purrdf_shapes::engine::parse_shapes(&text, None).unwrap());
    let report = validate_complete_sources(
        Arc::clone(shapes.dataset()),
        shapes,
        &ValidationOptions::default(),
    )
    .unwrap();
    let payload = complete_report_payload(&report).unwrap();
    assert_eq!(payload["sources"].as_array().unwrap().len(), 1);
    assert_eq!(payload["dataSource"], payload["shapesSource"]);
    let original = payload["correspondence"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["label"].as_str() == Some("same"))
        .count();
    assert_eq!(original, 1);
}

#[test]
fn unsupported_admission_semantic_and_resource_refusals_are_not_verdicts() {
    let (status, unsupported) = complete_profile("shacl-rec-2017-07-21").unwrap_err();
    assert_eq!(status, CompleteValidationStatus::UnsupportedProfile);
    assert_eq!(unsupported.id(), "shacl-rec-2017-07-21");
    assert_eq!(
        complete_profile(ShaclProfile::REC_20170720.id()).unwrap(),
        ShaclProfile::REC_20170720
    );
    let options = ValidationOptions::default().with_profile(ShaclProfile::REC_20170720);
    let minus = SHAPES.replace(
        "SELECT $this WHERE {}",
        "SELECT $this WHERE { MINUS { $this ?p ?o } }",
    );
    assert_eq!(
        complete_validation_status(&validate_complete_documents(
            &minus,
            None,
            DATA,
            &options,
            &[]
        )),
        CompleteValidationStatus::AdmissionRefused
    );
    let failure = SHAPES.replace(
        "SELECT $this WHERE {}",
        "SELECT $this ?failure WHERE { BIND(true AS ?failure) }",
    );
    assert_eq!(
        complete_validation_status(&validate_complete_documents(
            &failure,
            None,
            DATA,
            &options,
            &[]
        )),
        CompleteValidationStatus::SemanticFailure
    );
    let pattern = "@prefix sh: <http://www.w3.org/ns/shacl#> . <http://example.org/S> a sh:NodeShape; sh:pattern \"a\" .";
    let low = options.clone().with_xpath_regex(
        Profile::Xpath20,
        Limits::new().with(Resource::PatternBytes, 0),
    );
    assert_eq!(
        complete_validation_status(&validate_complete_documents(pattern, None, "", &low, &[])),
        CompleteValidationStatus::ResourceRefused
    );
    assert_eq!(
        complete_validation_status(&validate_complete_documents(
            pattern,
            None,
            "",
            &options,
            &[]
        )),
        CompleteValidationStatus::Conforms
    );
    assert_eq!(
        complete_validation_status(&validate_complete_documents(
            SHAPES,
            None,
            DATA,
            &options,
            &[]
        )),
        CompleteValidationStatus::Nonconforms
    );
}

#[test]
fn reached_native_query_resources_use_machine_codes_and_recover_with_current_limits() {
    let query = SHAPES.replace(
        "SELECT $this WHERE {}",
        "SELECT $this WHERE { FILTER(REGEX('a', 'a')) }",
    );
    for profile in [ShaclProfile::REC_20170720, ShaclProfile::WD_20260918] {
        let options = ValidationOptions::default().with_profile(profile);
        let low = options.clone().with_xpath_regex(
            profile.xpath_profile().unwrap(),
            Limits::new().with(Resource::MatchSteps, 0),
        );
        let refused = validate_complete_documents(&query, None, DATA, &low, &[]);
        assert!(matches!(&refused,
            Err(purrdf_shapes::report::CompleteValidationError::XPath(error))
                if matches!(error.as_ref(), purrdf_shapes::xpath::XPathValidationError::Query(diagnostic)
                    if diagnostic.code == Resource::MatchSteps.code())));
        assert_eq!(
            complete_validation_status(&refused),
            CompleteValidationStatus::ResourceRefused
        );
        assert_eq!(
            complete_validation_status(&validate_complete_documents(
                &query,
                None,
                DATA,
                &options,
                &[]
            )),
            CompleteValidationStatus::Nonconforms
        );
    }
}

#[test]
fn dated_some_value_discards_ordinary_query_failures_but_retains_resource_refusals() {
    let shapes = r"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape; sh:targetNode ex:n;
            sh:property [ sh:path ex:p; sh:someValue [
                sh:or ( [ sh:class ex:Duck ] [ sh:sparql [
                    sh:select 'SELECT $this WHERE { FILTER(<http://example.org/missingFunction>($this)) }'
                ] ] )
            ] ] .
    ";
    let first = "<http://example.org/n> <http://example.org/p> <http://example.org/aBad> .";
    let data = format!(
        "{first}\n<http://example.org/n> <http://example.org/p> <http://example.org/zGood> .\n\
         <http://example.org/zGood> <{}> <http://example.org/Duck> .",
        purrdf_iri::vocab::rdf::TYPE
    );
    let resource_shapes = shapes.replace(
        "<http://example.org/missingFunction>($this)",
        "REGEX(STR($this), \"bad\", \"i\")",
    );
    for profile in [ShaclProfile::REC_20170720, ShaclProfile::WD_20260918] {
        let options = ValidationOptions::default().with_profile(profile);
        let discarded = validate_complete_documents(shapes, None, &data, &options, &[]);
        assert_eq!(
            complete_validation_status(&discarded),
            CompleteValidationStatus::Conforms,
            "{profile:?}: the later Duck conforms despite the earlier ordinary failure"
        );
        assert!(discarded.unwrap().results().next().is_none());
        assert_eq!(
            complete_validation_status(&validate_complete_documents(
                shapes,
                None,
                first,
                &options,
                &[]
            )),
            CompleteValidationStatus::ExecutionFailure,
            "{profile:?}: without a conforming value the ordinary failure remains"
        );
        let low = options.clone().with_xpath_regex(
            profile.xpath_profile().unwrap(),
            Limits::new().with(Resource::MatchSteps, 0),
        );
        let refused = validate_complete_documents(&resource_shapes, None, &data, &low, &[]);
        assert_eq!(
            complete_validation_status(&refused),
            CompleteValidationStatus::ResourceRefused,
            "{profile:?}: a later conforming value cannot erase execution refusal"
        );
        assert!(matches!(&refused,
            Err(purrdf_shapes::report::CompleteValidationError::XPath(error))
                if matches!(error.as_ref(), purrdf_shapes::xpath::XPathValidationError::Query(diagnostic)
                    if diagnostic.code == Resource::MatchSteps.code())));
        assert_eq!(
            complete_validation_status(&validate_complete_documents(
                &resource_shapes,
                None,
                &data,
                &options,
                &[]
            )),
            CompleteValidationStatus::Conforms,
            "{profile:?}: current higher limits recover the same authored request"
        );
    }
}

#[test]
fn shared_product_doors_apply_the_current_bundle_before_source_admission() {
    let shapes = Arc::new(purrdf_shapes::engine::parse_shapes(SHAPES, None).unwrap());
    let prepared = PreparedShapes::new(shapes);
    let bytes = prepared.to_product(&ShapesProfile::CORE).unwrap();
    let identity = *purrdf_shapes::product::ShapesProduct::open(&bytes)
        .unwrap()
        .declared_identity()
        .digest();
    for profile in [ShaclProfile::REC_20170720, ShaclProfile::WD_20260918] {
        let options = ValidationOptions::default().with_profile(profile);
        for rebuild in [false, true] {
            for expected in [None, Some(&identity)] {
                let report =
                    validate_complete_product(&bytes, DATA, &options, expected, rebuild).unwrap();
                assert_eq!(report.profile(), profile);
                assert_eq!(report.results().len(), 1);
                assert!(matches!(
                    complete_report_payload(&report).unwrap()["graph"],
                    Value::String(_)
                ));
            }
        }
    }
    let wrong = [0; 32];
    assert_eq!(
        complete_validation_status(&validate_complete_product(
            &bytes,
            DATA,
            &ValidationOptions::default(),
            Some(&wrong),
            false
        )),
        CompleteValidationStatus::ProductRefused
    );
    assert!(
        validate_complete_product(
            &bytes,
            DATA,
            &ValidationOptions::default(),
            Some(&identity),
            false
        )
        .is_ok()
    );
}

#[test]
fn existing_default_sarif_body_is_the_complete_compatibility_projection() {
    let options = SarifOptions::default();
    let legacy =
        purrdf_validate::validate_to_sarif_string(SHAPES, None, DATA, &options, &[]).unwrap();
    let complete =
        validate_complete_documents(SHAPES, None, DATA, &options.validation, &[]).unwrap();
    assert_eq!(
        purrdf_validate::report_to_sarif_string(complete.legacy(), &options),
        legacy
    );
    let mut rich =
        json::read(&complete_report_to_sarif_string(&complete, &options).unwrap()).unwrap();
    rich["runs"][0]["properties"]
        .as_object_mut()
        .unwrap()
        .remove("shaclCompleteReport");
    assert_eq!(rich, json::read(&legacy).unwrap());
}

#[test]
fn native_allocation_refusals_keep_resource_status_across_pattern_and_query() {
    use purrdf_core::RdfDiagnostic;
    use purrdf_core::xsd_regex::xpath::Error;
    use purrdf_shapes::report::CompleteValidationError;
    use purrdf_shapes::xpath::XPathValidationError;
    use purrdf_sparql_eval::EvalError;

    // Constructed typed refusals exercise conversion without forcing host OOM.
    for resource in [
        Resource::PatternBytes,
        Resource::CompileSteps,
        Resource::ProgramNodes,
        Resource::CompileSlots,
        Resource::MatchSteps,
        Resource::MatchStates,
        Resource::MatchSlots,
        Resource::OutputBytes,
    ] {
        let cause = Error::Allocation { resource, units: 7 };
        let pattern = Err(CompleteValidationError::XPath(Box::new(
            XPathValidationError::Pattern(cause.clone()),
        )));
        assert_eq!(
            complete_validation_status(&pattern),
            CompleteValidationStatus::ResourceRefused,
            "{resource:?}: direct Pattern allocation",
        );

        let evaluator = EvalError::XPathRegex(cause);
        let diagnostic =
            RdfDiagnostic::error(evaluator.code().unwrap(), "unrelated diagnostic words");
        let query = Err(CompleteValidationError::XPath(Box::new(
            XPathValidationError::Query(diagnostic),
        )));
        assert_eq!(
            complete_validation_status(&query),
            CompleteValidationStatus::ResourceRefused,
            "{resource:?}: public SPARQL code to Query status",
        );
    }

    let generic_query = Err(CompleteValidationError::XPath(Box::new(
        XPathValidationError::Query(RdfDiagnostic::error(
            "native-sparql-xpath-operational",
            Resource::MatchSlots.code(),
        )),
    )));
    assert_eq!(
        complete_validation_status(&generic_query),
        CompleteValidationStatus::NativeXPathRefused,
        "resource wording cannot confer resource classification",
    );
    let poisoned = Err(CompleteValidationError::XPath(Box::new(
        XPathValidationError::CachePoisoned,
    )));
    assert_eq!(
        complete_validation_status(&poisoned),
        CompleteValidationStatus::NativeXPathRefused,
    );
}

/// Every `sh:ValidationResult` property, read from one complete result.
///
/// SHACL defines `sh:sourceConstraint` only on a SPARQL-based constraint's
/// result, and only Core's nesting components produce `sh:detail`, so no single
/// node can carry both. One result tree covers all nine: the member-shape parent
/// carries focus node, result path, value, source shape, source constraint
/// component, severity, message and detail, and its SPARQL detail carries the
/// source constraint and a result annotation.
#[test]
fn one_complete_result_carries_every_validation_result_property() {
    const SHAPES: &str = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetNode ex:n ; sh:property ex:P .
        ex:P sh:path ex:p ; sh:severity sh:Warning ; sh:message "member failed"@en ;
            sh:memberShape ex:M .
        ex:M a sh:NodeShape ; sh:sparql ex:C .
        ex:C sh:message "constraint message" ;
            sh:resultAnnotation [ sh:annotationProperty ex:tag ] ;
            sh:select "SELECT $this ?tag WHERE { BIND(\"tagged\" AS ?tag) }" .
    "#;
    const DATA: &str = "<http://example.org/n> <http://example.org/p> _:list .\n\
        _:list <http://www.w3.org/1999/02/22-rdf-syntax-ns#first> <http://example.org/a> .\n\
        _:list <http://www.w3.org/1999/02/22-rdf-syntax-ns#rest> \
        <http://www.w3.org/1999/02/22-rdf-syntax-ns#nil> .";
    const EX: &str = "http://example.org/";
    let options = ValidationOptions::default().with_profile(ShaclProfile::WD_20260918);
    let report = validate_complete_documents(SHAPES, None, DATA, &options, &[]).unwrap();
    assert!(!report.legacy().conforms);
    let [parent] = report.results().collect::<Vec<_>>()[..] else {
        panic!("one top-level result");
    };
    let legacy = parent.legacy();
    assert_eq!(legacy.focus_node.to_string(), format!("<{EX}n>"));
    assert_eq!(
        legacy.result_path.as_ref().unwrap().to_string(),
        format!("<{EX}p>")
    );
    assert!(matches!(
        legacy.value,
        Some(purrdf_shapes::term::Term::BlankNode(_))
    ));
    assert_eq!(legacy.source_shape.to_string(), format!("<{EX}P>"));
    assert_eq!(
        legacy.source_constraint_component.as_str(),
        "http://www.w3.org/ns/shacl#MemberShapeConstraintComponent"
    );
    assert_eq!(legacy.severity.iri(), "http://www.w3.org/ns/shacl#Warning");
    assert_eq!(
        legacy
            .messages
            .iter()
            .map(|message| (message.value(), message.language()))
            .collect::<Vec<_>>(),
        [("member failed", Some("en"))]
    );
    assert_eq!(parent.source_constraint(), None);
    let [detail] = parent.details().collect::<Vec<_>>()[..] else {
        panic!("one detail");
    };
    let inner = detail.legacy();
    assert_eq!(inner.focus_node.to_string(), format!("<{EX}a>"));
    assert_eq!(
        inner.value.as_ref().unwrap().to_string(),
        format!("<{EX}a>")
    );
    assert_eq!(inner.source_shape.to_string(), format!("<{EX}M>"));
    assert_eq!(
        inner.source_constraint_component.as_str(),
        "http://www.w3.org/ns/shacl#SPARQLConstraintComponent"
    );
    assert_eq!(
        detail.source_constraint().unwrap().to_string(),
        format!("<{EX}C>")
    );
    assert_eq!(
        inner
            .messages
            .iter()
            .map(|message| (message.value(), message.language()))
            .collect::<Vec<_>>(),
        [("constraint message", None)]
    );
    assert_eq!(
        inner
            .annotations
            .iter()
            .map(|(property, value)| (property.as_str().to_owned(), value.to_string()))
            .collect::<Vec<_>>(),
        [(format!("{EX}tag"), "\"tagged\"".to_owned())]
    );

    // The emitted complete graph states exactly these properties on the two
    // result nodes, and the parent's value keeps its data-graph identity.
    let graph = report.to_graph();
    let dataset = graph.dataset();
    let iri = |id| match dataset.term_value(id) {
        purrdf_core::TermValue::Iri(iri) => Some(iri),
        _ => None,
    };
    let mut by_subject = std::collections::BTreeMap::<_, std::collections::BTreeSet<_>>::new();
    for quad in dataset.quads() {
        by_subject
            .entry(quad.s)
            .or_default()
            .insert(iri(quad.p).unwrap());
    }
    let sh = |local: &str| format!("http://www.w3.org/ns/shacl#{local}");
    let rdf_type = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_owned();
    let parent_properties: std::collections::BTreeSet<_> = [
        "focusNode",
        "resultPath",
        "value",
        "sourceShape",
        "sourceConstraintComponent",
        "resultSeverity",
        "resultMessage",
        "detail",
    ]
    .into_iter()
    .map(sh)
    .chain([rdf_type.clone()])
    .collect();
    let detail_properties: std::collections::BTreeSet<_> = [
        "focusNode",
        "value",
        "sourceShape",
        "sourceConstraintComponent",
        "sourceConstraint",
        "resultSeverity",
        "resultMessage",
    ]
    .into_iter()
    .map(sh)
    .chain([rdf_type, format!("{EX}tag")])
    .collect();
    assert_eq!(
        by_subject
            .values()
            .filter(|properties| **properties == parent_properties)
            .count(),
        1
    );
    assert_eq!(
        by_subject
            .values()
            .filter(|properties| **properties == detail_properties)
            .count(),
        1
    );
    let all: std::collections::BTreeSet<_> = parent_properties
        .union(&detail_properties)
        .cloned()
        .collect();
    for property in [
        "focusNode",
        "resultPath",
        "value",
        "sourceShape",
        "sourceConstraintComponent",
        "detail",
        "resultMessage",
        "resultSeverity",
        "sourceConstraint",
    ] {
        assert!(all.contains(&sh(property)), "{property}");
    }
    let value = by_subject
        .iter()
        .find(|(_, properties)| **properties == parent_properties)
        .map(|(subject, _)| *subject)
        .unwrap();
    let value_term = dataset
        .quads()
        .find(|quad| quad.s == value && iri(quad.p).as_deref() == Some(&sh("value")))
        .map(|quad| dataset.term_value(quad.o))
        .unwrap();
    let purrdf_core::TermValue::Blank { label, .. } = value_term else {
        panic!("the list value is a blank node");
    };
    let source = graph.blank_labels().source_of(&label).unwrap();
    assert_eq!(source.source(), graph.source_context().data());
    assert_eq!(source.label(), "list");
}
