// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Observed outcomes and EARL rendering, with no environmental failure coercion.

use purrdf_core::{BlankScope, RdfDataset, RdfDatasetBuilder};
use purrdf_iri::vocab::{earl, rdf};

use crate::GradeError;

/// The observed execution category. Environmental failures remain distinct from
/// the intended semantic/profile rejection that a negative case requires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutcomeKind {
    /// The normative result/report was compared successfully.
    Compared,
    /// The selected language/profile deliberately refused this construct.
    IntendedRejection,
    /// A validation solution explicitly reported the specified semantic failure.
    SemanticFailure,
    /// The engine does not implement the applicable capability.
    Unsupported,
    /// The selected profile/surface does not apply to the case.
    Inapplicable,
    /// The applicable case was not run.
    Unexecuted,
    /// A resource boundary prevented establishing the result.
    ResourceExhausted,
    /// Input/output did not satisfy its declared carrier shape.
    Malformed,
    /// The execution failed unexpectedly or crashed.
    Crash,
    /// A well-formed produced result differed from the oracle.
    Mismatch,
}

purrdf_lex::json_string_enum!(OutcomeKind {
    Compared => "compared",
    IntendedRejection => "intended-rejection",
    SemanticFailure => "semantic-failure",
    Unsupported => "unsupported",
    Inapplicable => "inapplicable",
    Unexecuted => "unexecuted",
    ResourceExhausted => "resource-exhausted",
    Malformed => "malformed",
    Crash => "crash",
    Mismatch => "mismatch",
});

/// A typed observation, including the stable rejection reason when relevant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observation {
    /// The actual execution category.
    pub kind: OutcomeKind,
    /// Stable reason identity; not an arbitrary display-message substring.
    pub reason: Option<String>,
    /// Human-readable evidence, never used to infer the category.
    pub message: String,
}

purrdf_lex::json_record!(Observation as "conformance observation" {
    "kind" => kind: required,
    "reason" => reason: optional,
    "message" => message: required,
});

/// Compare an observed category and stable reason with a declared expectation.
///
/// # Errors
/// Unexpected failure, incomplete execution, wrong rejection and mismatches fail.
pub fn grade(expected: &Observation, actual: &Observation) -> Result<(), GradeError> {
    if !matches!(
        expected.kind,
        OutcomeKind::Compared | OutcomeKind::IntendedRejection | OutcomeKind::SemanticFailure
    ) {
        return Err(GradeError::Malformed(
            "expectation must require comparison or an intended rejection".to_owned(),
        ));
    }
    if matches!(
        expected.kind,
        OutcomeKind::IntendedRejection | OutcomeKind::SemanticFailure
    ) && expected.reason.as_deref().is_none_or(str::is_empty)
    {
        return Err(GradeError::Malformed(
            "negative expectation must name a stable rejection reason".to_owned(),
        ));
    }
    if expected.kind == actual.kind && expected.reason == actual.reason {
        Ok(())
    } else {
        Err(GradeError::Mismatch(format!(
            "expected {:?}/{:?}, observed {:?}/{:?}: {}",
            expected.kind, expected.reason, actual.kind, actual.reason, actual.message
        )))
    }
}

/// One replayable conformance observation. `passed` is established by `grade`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// Stable case IRI.
    pub case: String,
    /// Selected public language/profile identity.
    pub profile: String,
    /// Actual engine/binding surface identity.
    pub surface: String,
    /// Declared expected category/reason.
    pub expected: Observation,
    /// Actual observed result.
    pub observed: Observation,
    /// True exactly for an established expected result/rejection.
    pub passed: bool,
    /// Content-addressed evidence artifacts, independently supplied by caller.
    pub artifacts: Vec<String>,
}

purrdf_lex::json_record!(Record as "conformance record" {
    "case" => case: required,
    "profile" => profile: required,
    "surface" => surface: required,
    "expected" => expected: required,
    "observed" => observed: required,
    "passed" => passed: required,
    "artifacts" => artifacts: required,
});

/// Render EARL assertions from exact typed observations. No clock or generated
/// namespace is consulted: asserted-by and test subject are explicit IRIs.
///
/// # Errors
/// Refuses a record whose claimed verdict differs from its actual grade.
pub fn to_earl(
    records: &[Record],
    asserted_by: &str,
    subject: &str,
) -> Result<std::sync::Arc<RdfDataset>, GradeError> {
    let mut builder = RdfDatasetBuilder::new();
    let rdf_type = builder.intern_iri(rdf::TYPE);
    let assertion_class = builder.intern_iri(earl::ASSERTION);
    let result_class = builder.intern_iri(earl::TEST_RESULT);
    let by_predicate = builder.intern_iri(earl::ASSERTED_BY);
    let subject_predicate = builder.intern_iri(earl::SUBJECT);
    let test_predicate = builder.intern_iri(earl::TEST);
    let result_predicate = builder.intern_iri(earl::RESULT);
    let outcome_predicate = builder.intern_iri(earl::OUTCOME);
    let mode_predicate = builder.intern_iri(earl::MODE);
    let automatic = builder.intern_iri(earl::AUTOMATIC);
    let by = builder.intern_iri(asserted_by);
    let subject = builder.intern_iri(subject);
    for (index, record) in records.iter().enumerate() {
        let graded = grade(&record.expected, &record.observed);
        if let Err(GradeError::Malformed(message)) = &graded {
            return Err(GradeError::Malformed(message.clone()));
        }
        if graded.is_ok() != record.passed {
            return Err(GradeError::Malformed(
                "record verdict contradicts its observation".to_owned(),
            ));
        }
        let assertion = builder.intern_blank(&format!("assertion{index}"), BlankScope::DEFAULT);
        let result = builder.intern_blank(&format!("result{index}"), BlankScope::DEFAULT);
        let test = builder.intern_iri(&record.case);
        let outcome = builder.intern_iri(if record.passed {
            earl::PASSED
        } else {
            match record.observed.kind {
                OutcomeKind::Inapplicable => earl::INAPPLICABLE,
                OutcomeKind::Unexecuted => earl::UNTESTED,
                OutcomeKind::Unsupported | OutcomeKind::ResourceExhausted | OutcomeKind::Crash => {
                    earl::CANT_TELL
                }
                _ => earl::FAILED,
            }
        });
        for (s, p, o) in [
            (assertion, rdf_type, assertion_class),
            (assertion, by_predicate, by),
            (assertion, subject_predicate, subject),
            (assertion, test_predicate, test),
            (assertion, result_predicate, result),
            (assertion, mode_predicate, automatic),
            (result, rdf_type, result_class),
            (result, outcome_predicate, outcome),
        ] {
            builder.push_quad(s, p, o, None);
        }
    }
    builder
        .freeze()
        .map_err(|error| GradeError::Malformed(error.to_string()))
}
