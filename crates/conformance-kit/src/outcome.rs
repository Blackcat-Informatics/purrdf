// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Observed outcomes and EARL rendering, with no environmental failure coercion.

use purrdf_core::{BlankScope, RdfDataset, RdfDatasetBuilder, RdfLiteral, TermId};
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

/// The object of a caller-supplied description statement.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum DescriptionTerm {
    /// An IRI.
    Iri(String),
    /// A plain string literal.
    Text(String),
}

/// Intern `text` as a plain (`xsd:string`) literal.
fn plain_literal(builder: &mut RdfDatasetBuilder, text: &str) -> Result<TermId, GradeError> {
    Ok(builder.intern_literal(RdfLiteral {
        lexical_form: text.to_owned(),
        datatype: None,
        language: None,
        direction: None,
    })?)
}

/// The common shape of a described resource: its IRI, its `rdf:type` classes and
/// caller-supplied `(predicate IRI, term)` pairs. Types and pairs are sorted and
/// deduplicated on construction, so insertion order never reaches the output.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Description {
    iri: String,
    types: Vec<String>,
    properties: Vec<(String, DescriptionTerm)>,
}

impl Description {
    fn new(
        iri: impl Into<String>,
        mut types: Vec<String>,
        mut properties: Vec<(String, DescriptionTerm)>,
    ) -> Self {
        types.sort();
        types.dedup();
        properties.sort();
        properties.dedup();
        Self {
            iri: iri.into(),
            types,
            properties,
        }
    }

    /// Intern the resource and write its statements, returning its term.
    fn emit(&self, builder: &mut RdfDatasetBuilder) -> Result<TermId, GradeError> {
        let node = builder.intern_iri(&self.iri)?;
        let rdf_type = builder.intern_iri(rdf::TYPE)?;
        for class in &self.types {
            let class = builder.intern_iri(class)?;
            builder.push_quad(node, rdf_type, class, None)?;
        }
        for (predicate, term) in &self.properties {
            let predicate = builder.intern_iri(predicate)?;
            let object = match term {
                DescriptionTerm::Iri(iri) => builder.intern_iri(iri)?,
                DescriptionTerm::Text(text) => plain_literal(builder, text)?,
            };
            builder.push_quad(node, predicate, object, None)?;
        }
        Ok(node)
    }
}

/// The `earl:subject` of a report: the implementation under test.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubjectDescription(Description);

impl SubjectDescription {
    /// Describe the subject `iri` by its classes and `(predicate, term)` pairs.
    #[must_use]
    pub fn new(
        iri: impl Into<String>,
        types: Vec<String>,
        properties: Vec<(String, DescriptionTerm)>,
    ) -> Self {
        Self(Description::new(iri, types, properties))
    }

    /// The subject IRI.
    #[must_use]
    pub fn iri(&self) -> &str {
        &self.0.iri
    }
}

/// The `earl:assertedBy` of a report: who stands behind the assertions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssertorDescription(Description);

impl AssertorDescription {
    /// Describe the assertor `iri` by its classes and `(predicate, term)` pairs.
    #[must_use]
    pub fn new(
        iri: impl Into<String>,
        types: Vec<String>,
        properties: Vec<(String, DescriptionTerm)>,
    ) -> Self {
        Self(Description::new(iri, types, properties))
    }

    /// The assertor IRI.
    #[must_use]
    pub fn iri(&self) -> &str {
        &self.0.iri
    }
}

/// One record of a [`Report`], with the optional `earl:info` text of its result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportRecord {
    /// The graded observation.
    pub record: Record,
    /// Free text for the result's `earl:info`, written verbatim when present.
    pub info: Option<String>,
}

impl From<Record> for ReportRecord {
    fn from(record: Record) -> Self {
        Self { record, info: None }
    }
}

/// One report structure with two projections, [`to_earl`] and
/// [`Report::scoreboard_line`], over the same sorted records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    subject: SubjectDescription,
    assertor: AssertorDescription,
    records: Vec<ReportRecord>,
}

impl Report {
    /// A report over `records`, sorted by case, profile and surface so that the
    /// caller's order never reaches the output.
    #[must_use]
    pub fn new(
        subject: SubjectDescription,
        assertor: AssertorDescription,
        mut records: Vec<ReportRecord>,
    ) -> Self {
        records.sort_by(|a, b| {
            (&a.record.case, &a.record.profile, &a.record.surface).cmp(&(
                &b.record.case,
                &b.record.profile,
                &b.record.surface,
            ))
        });
        Self {
            subject,
            assertor,
            records,
        }
    }

    /// The subject description.
    #[must_use]
    pub fn subject(&self) -> &SubjectDescription {
        &self.subject
    }

    /// The assertor description.
    #[must_use]
    pub fn assertor(&self) -> &AssertorDescription {
        &self.assertor
    }

    /// The sorted records.
    #[must_use]
    pub fn records(&self) -> &[ReportRecord] {
        &self.records
    }

    /// The EARL-scope scoreboard line, `LABEL: passed P, FAILED_LABEL F of N`,
    /// counting the records whose EARL outcome is `earl:passed` and `earl:failed`
    /// out of all `N` records. It is the second projection of the records that
    /// [`to_earl`] renders.
    #[must_use]
    pub fn scoreboard_line(&self, label: &str, failed_label: &str) -> String {
        let count = |outcome: &str| {
            self.records
                .iter()
                .filter(|entry| earl_outcome(&entry.record) == outcome)
                .count()
        };
        format!(
            "{label}: passed {}, {failed_label} {} of {}",
            count(earl::PASSED),
            count(earl::FAILED),
            self.records.len()
        )
    }
}

/// The EARL outcome IRI of one record.
fn earl_outcome(record: &Record) -> &'static str {
    if record.passed {
        return earl::PASSED;
    }
    match record.observed.kind {
        OutcomeKind::Inapplicable => earl::INAPPLICABLE,
        OutcomeKind::Unexecuted => earl::UNTESTED,
        OutcomeKind::Unsupported | OutcomeKind::ResourceExhausted | OutcomeKind::Crash => {
            earl::CANT_TELL
        }
        _ => earl::FAILED,
    }
}

/// Render a [`Report`] as EARL: the described subject and assertor once, then one
/// `earl:Assertion` per record in the report's order. No clock or generated
/// namespace is consulted, and the assertion nodes are labelled by position in the
/// sorted records, so equal reports render to equal datasets.
///
/// # Errors
/// Refuses a record whose claimed verdict differs from its actual grade.
pub fn to_earl(report: &Report) -> Result<std::sync::Arc<RdfDataset>, GradeError> {
    let mut builder = RdfDatasetBuilder::new();
    let rdf_type = builder.intern_iri(rdf::TYPE)?;
    let assertion_class = builder.intern_iri(earl::ASSERTION)?;
    let result_class = builder.intern_iri(earl::TEST_RESULT)?;
    let by_predicate = builder.intern_iri(earl::ASSERTED_BY)?;
    let subject_predicate = builder.intern_iri(earl::SUBJECT)?;
    let test_predicate = builder.intern_iri(earl::TEST)?;
    let result_predicate = builder.intern_iri(earl::RESULT)?;
    let outcome_predicate = builder.intern_iri(earl::OUTCOME)?;
    let mode_predicate = builder.intern_iri(earl::MODE)?;
    let info_predicate = builder.intern_iri(earl::INFO)?;
    let automatic = builder.intern_iri(earl::AUTOMATIC)?;
    let by = report.assertor.0.emit(&mut builder)?;
    let subject = report.subject.0.emit(&mut builder)?;
    for (index, entry) in report.records.iter().enumerate() {
        let record = &entry.record;
        let graded = grade(&record.expected, &record.observed);
        if let Err(GradeError::Malformed(message)) = &graded {
            return Err(GradeError::Malformed(message.clone()));
        }
        if graded.is_ok() != record.passed {
            return Err(GradeError::Malformed(
                "record verdict contradicts its observation".to_owned(),
            ));
        }
        let assertion = builder.intern_blank(&format!("assertion{index}"), BlankScope::DEFAULT)?;
        let result = builder.intern_blank(&format!("result{index}"), BlankScope::DEFAULT)?;
        let test = builder.intern_iri(&record.case)?;
        let outcome = builder.intern_iri(earl_outcome(record))?;
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
            builder.push_quad(s, p, o, None)?;
        }
        if let Some(info) = &entry.info {
            let info = plain_literal(&mut builder, info)?;
            builder.push_quad(result, info_predicate, info, None)?;
        }
    }
    builder.freeze().map_err(GradeError::from)
}
