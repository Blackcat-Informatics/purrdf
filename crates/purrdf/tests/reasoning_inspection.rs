// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original query, entailment and materialization failures through the umbrella.

use purrdf::error::ErrorValue;
use purrdf::ir::import::DatasetImportError;
use purrdf::iri::vocab::{owl, rdf};
use purrdf::sparql::{NativeSparqlEngine, QueryOptions};
use purrdf::{
    ClosureRelations, DatasetMaterializationError, QueryEntailment, RdfDatasetBuilder,
    ReasoningError, SparqlRequest, query_with_entailment,
};
use purrdf_testkit::error_capture::{CallerRefusal, Capture};

fn actual_query_and_entailment_failures_retain_their_original_owners() {
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_iri("https://example.org/original").unwrap();
    let predicate = builder.intern_iri(rdf::TYPE).unwrap();
    let object = builder.intern_iri(owl::NOTHING).unwrap();
    builder.push_quad(subject, predicate, object, None).unwrap();
    let dataset = builder.freeze().unwrap();
    let engine = NativeSparqlEngine::new();
    let errors = [
        ("SELECT WHERE {", QueryEntailment::Simple, "reasoning-query"),
        (
            "SELECT ?s WHERE { ?s ?p ?o }",
            QueryEntailment::OwlRl,
            "reasoning-entailment",
        ),
    ]
    .map(|(query, regime, expected)| {
        let error = query_with_entailment(
            &engine,
            &dataset,
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &[],
            },
            regime,
            &QueryOptions::EMPTY,
            &ClosureRelations::NONE,
        )
        .unwrap_err();
        (error, expected)
    });
    assert!(
        query_with_entailment(
            &engine,
            &dataset,
            SparqlRequest {
                query: "SELECT ?s WHERE { ?s ?p ?o }",
                base_iri: None,
                substitutions: &[],
            },
            QueryEntailment::Simple,
            &QueryOptions::EMPTY,
            &ClosureRelations::NONE,
        )
        .is_ok()
    );
    drop(engine);
    drop(dataset);
    for (error, expected) in errors {
        let fields = Capture::read(&error);
        assert_eq!(fields.code, expected);
        let original = fields.child("cause");
        match error {
            ReasoningError::Query(ref cause) => {
                assert_eq!(original.code, "rdf-diagnostic");
                assert!(matches!(original.field("code"), ErrorValue::Code(code)
                    if code.as_ptr() == cause.code.as_ptr()));
            }
            ReasoningError::Entailment(_) => {
                assert_eq!(original.code, "entailment-inconsistent");
                assert_eq!(
                    original
                        .child("run")
                        .child("witness")
                        .records("premises")
                        .len(),
                    1
                );
            }
            other => panic!("wrong actual reasoning phase: {other:?}"),
        }
    }
}

fn original_reader_failures_cross_both_generic_reasoning_boundaries() {
    use purrdf::entail::EntailError;
    for error in [
        ReasoningError::Entailment(EntailError::SourceRead(CallerRefusal { limit: u128::MAX })),
        ReasoningError::Materialization(DatasetMaterializationError::Import(
            DatasetImportError::Read(CallerRefusal { limit: u128::MAX }),
        )),
    ] {
        let fields = Capture::read(&error);
        let mut cause = fields.child("cause");
        if fields.code == "reasoning-materialization" {
            assert_eq!(cause.code, "dataset-materialization-import");
            cause = cause.child("cause");
            assert_eq!(cause.code, "dataset-import-read");
        } else {
            assert_eq!(fields.code, "reasoning-entailment");
            assert_eq!(cause.code, "entailment-source-read");
        }
        assert!(matches!(
            cause.child("cause").field("limit"),
            ErrorValue::Unsigned(u128::MAX)
        ));
        let mut source: &(dyn std::error::Error + 'static) = &error;
        while let Some(next) = source.source() {
            source = next;
        }
        assert_eq!(
            source.downcast_ref::<CallerRefusal>().unwrap().limit,
            u128::MAX
        );
    }
}

purrdf_testkit::harness_main!(
    actual_query_and_entailment_failures_retain_their_original_owners,
    original_reader_failures_cross_both_generic_reasoning_boundaries,
);
