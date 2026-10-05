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
