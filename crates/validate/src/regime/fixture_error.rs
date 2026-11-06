// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Typed grammar and comparison failures of the shipping reasoning fixtures.

/// The original line and directive that a fixture parser cannot admit.
#[derive(Debug)]
#[non_exhaustive]
pub enum FixtureGrammarError {
    /// Text occurred outside a declared body.
    OutsideSection {
        /// Original one-based line.
        line: usize,
        /// Original text outside a section.
        text: String,
    },
    /// A named directive has no argument.
    MissingName {
        /// Original one-based line.
        line: usize,
        /// Directive requiring an argument.
        directive: String,
    },
    /// A new case opened before the previous case ended.
    NestedCase {
        /// Original one-based line of the nested directive.
        line: usize,
    },
    /// A completed case already has this name.
    DuplicateCase {
        /// Original one-based line.
        line: usize,
        /// Original repeated case name.
        name: String,
    },
    /// A case ended before a required section occurred.
    MissingSection {
        /// Original one-based end-directive line.
        line: usize,
        /// Required section that was absent.
        section: String,
    },
    /// A directive is unknown or invalid at its original position.
    Directive {
        /// Original one-based line.
        line: usize,
        /// Original directive spelling.
        keyword: String,
        /// Whether this parser also refuses known directives in this position.
        misplaced: bool,
    },
    /// The final case was never closed.
    UnclosedCase {
        /// Final one-based document line.
        line: usize,
        /// Original case that remained open.
        name: String,
    },
}

impl core::fmt::Display for FixtureGrammarError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::OutsideSection { line, text } => {
                write!(f, "line {line}: text outside any section: {text}")
            }
            Self::MissingName { line, directive } => {
                write!(f, "line {line}: @{directive} needs a name")
            }
            Self::NestedCase { line } => write!(f, "line {line}: @case inside an unclosed case"),
            Self::DuplicateCase { line, name } => {
                write!(f, "line {line}: duplicate case name \"{name}\"")
            }
            Self::MissingSection { line, section } => {
                write!(f, "line {line}: @end before @{section}")
            }
            Self::Directive {
                line,
                keyword,
                misplaced: false,
            } => write!(f, "line {line}: unknown directive @{keyword}"),
            Self::Directive {
                line,
                keyword,
                misplaced: true,
            } => write!(f, "line {line}: unknown or misplaced directive @{keyword}"),
            Self::UnclosedCase { .. } => {
                f.write_str("the artifact ends inside an unclosed case (missing @end)")
            }
        }
    }
}

impl core::error::Error for FixtureGrammarError {}

impl purrdf_hash::error::StructuredError for FixtureGrammarError {
    purrdf_hash::error_visit! {
        OutsideSection { line, text } => "reasoning-fixture-outside-section" { "line" => Unsigned(*line as u128), "text" => Text(text) },
        MissingName { line, directive } => "reasoning-fixture-missing-name" { "line" => Unsigned(*line as u128), "directive" => Code(directive) },
        NestedCase { line } => "reasoning-fixture-nested-case" { "line" => Unsigned(*line as u128) },
        DuplicateCase { line, name } => "reasoning-fixture-duplicate-case" { "line" => Unsigned(*line as u128), "name" => Text(name) },
        MissingSection { line, section } => "reasoning-fixture-missing-section" { "line" => Unsigned(*line as u128), "section" => Code(section) },
        Directive { line, keyword, misplaced } => "reasoning-fixture-directive" { "line" => Unsigned(*line as u128), "keyword" => Code(keyword), "misplaced" => Boolean(*misplaced) },
        UnclosedCase { line, name } => "reasoning-fixture-unclosed-case" { "line" => Unsigned(*line as u128), "name" => Text(name) },
    }
}

/// The fixture inventory whose expected output is being checked.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum FixtureKind {
    /// Fixed-rule materialization cases.
    Regime,
    /// Recorded DL proof cases.
    DlProof,
}

impl FixtureKind {
    pub(super) const fn description(self) -> &'static str {
        match self {
            Self::Regime => "regime",
            Self::DlProof => "DL proof",
        }
    }

    pub(super) const fn coverage(self) -> &'static str {
        match self {
            Self::Regime => "regime",
            Self::DlProof => "service",
        }
    }
}

/// A named invariant checked by the existing proof and refusal fixtures.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum FixtureInvariant {
    /// The count boundary must retain its native range refusal.
    CardinalityRefusal,
    /// A refused schema-preparation answer must remain unknown.
    SchemaUnknown,
    /// A refused recorded service must carry a receipt.
    SchemaReceipt,
    /// Encoding and decoding must preserve the original refusal receipt.
    SchemaReceiptIdentity,
    /// Text reconstruction must preserve the original preparation obstruction.
    SchemaCertificate,
    /// Replay must refuse a different original obstruction.
    SchemaOriginalCause,
    /// A sufficient retry must finish against the same source.
    SchemaRetry,
    /// Native schema construction must honor the latched stop.
    SchemaStop,
    /// Disjoint classes with a shared instance must be refused.
    InconsistencyRefused,
    /// Every causal refutation must be closed.
    CausalClosed,
    /// Causal capture must retain the nested reserved descriptor.
    CausalDescriptor,
    /// An unrecorded session must carry no native proof.
    NoUnrecordedProof,
    /// An unrecorded answer must carry the canonical absent-proof document.
    AbsentDocument,
    /// Checking an absent proof must be refused.
    AbsentRefused,
    /// The refusal must explain that nothing was recorded.
    AbsentExplanation,
    /// A recorded answer must declare its availability.
    RecordedAvailability,
}

impl FixtureInvariant {
    pub(super) const fn message(self) -> &'static str {
        match self {
            Self::CardinalityRefusal => {
                "portable boundary did not preserve the typed unrepresentable-count refusal"
            }
            Self::SchemaUnknown => "native schema refusal was reported as a decided answer",
            Self::SchemaReceipt => "refused recorded service has no receipt",
            Self::SchemaReceiptIdentity => "typed portable refusal receipt changed",
            Self::SchemaCertificate => "portable certificate lost its preparation obstruction",
            Self::SchemaOriginalCause => "portable refusal accepted a different original cause",
            Self::SchemaRetry => "sufficient native retry did not complete the same source",
            Self::SchemaStop => "native schema construction ignored a latched stop",
            Self::InconsistencyRefused => {
                "two disjoint classes with a shared instance must be refused by cax-dw"
            }
            Self::CausalClosed => "causal vector has an unclosed refutation",
            Self::CausalDescriptor => "causal vector lost its original nested reserved descriptor",
            Self::NoUnrecordedProof => "a session that records nothing must carry no proof term",
            Self::AbsentDocument => "an unrecorded answer must carry the absent-proof document",
            Self::AbsentRefused => {
                "checking an absent proof reported a result; an answer nobody recorded must never be presented as a verified one"
            }
            Self::AbsentExplanation => "the refusal must say that nothing was recorded",
            Self::RecordedAvailability => "a recorded answer must say so",
        }
    }
}

/// A fixture failure preserving the original expected output or native case failure.
#[derive(Debug)]
#[non_exhaustive]
pub enum FixtureError {
    /// A schema receipt or certificate changed its original output.
    SchemaOutput {
        /// The named schema expectation.
        condition: FixtureInvariant,
        /// Complete expected original output.
        expected: String,
        /// Complete actual output.
        produced: String,
    },
    /// A required authored schema case is absent.
    SchemaMissing {
        /// Original required name.
        name: String,
    },
    /// A schema case no longer carries its original authored controls.
    SchemaControl {
        /// Original case name.
        name: String,
        /// Expected input, answer, service and argument, in that order.
        expected: [String; 4],
        /// Actual input, answer, service and argument, in that order.
        produced: [String; 4],
    },
    /// The number of schema cases disagrees with the authored inventory.
    SchemaInventory {
        /// Actual authored case count.
        expected: usize,
        /// Actual committed case count.
        produced: usize,
    },
    /// The fixture's own grammar is malformed.
    Grammar(FixtureGrammarError),
    /// The fixture has no cases.
    Empty {
        /// Original fixture inventory.
        kind: FixtureKind,
    },
    /// A named case failed through the real reasoning API.
    Case {
        /// Original case name.
        name: String,
        /// Original native execution failure.
        cause: Box<super::ReasoningError>,
    },
    /// A named output differs byte for byte.
    Mismatch {
        /// Original case name.
        name: String,
        /// Compared output field.
        field: String,
        /// Complete expected bytes as text.
        expected: String,
        /// Complete actual bytes as text.
        produced: String,
    },
    /// The declared inventory omits a required regime or service.
    Coverage {
        /// Original fixture inventory.
        kind: FixtureKind,
        /// Required regime or service absent from that inventory.
        missing: String,
    },
    /// The proof inventory omitted its original causal case.
    MissingCausalCase,
    /// An observed result violates the fixture's named invariant.
    Invariant {
        /// The precise expectation that failed.
        condition: FixtureInvariant,
        /// Original observed output, when the expectation compares output.
        actual: Option<String>,
        /// Original native failure, when the expectation inspects a refusal.
        cause: Option<Box<super::ReasoningError>>,
    },
    /// An original refusal omitted required presentation evidence.
    RefusalEvidence {
        /// Required original evidence fragment.
        expected: String,
        /// Original native refusal whose presentation lacked the fragment.
        cause: Box<super::ReasoningError>,
    },
}

impl core::fmt::Display for FixtureError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::SchemaOutput { condition, .. } => f.write_str(condition.message()),
            Self::SchemaMissing { name } => {
                write!(f, "missing canonical native/portable schema vector {name}")
            }
            Self::SchemaControl { name, .. } => write!(
                f,
                "schema vector {name} no longer carries the actual authored control"
            ),
            Self::SchemaInventory { .. } => {
                f.write_str("schema preparation vector inventory mismatch")
            }
            Self::Grammar(cause) => core::fmt::Display::fmt(cause, f),
            Self::Empty { kind } => write!(
                f,
                "the {} golden vector artifact holds no cases",
                kind.description()
            ),
            Self::Case { name, cause } => write!(f, "case \"{name}\": {cause}"),
            Self::Mismatch {
                name,
                field,
                expected,
                produced,
            } => write!(
                f,
                "case \"{name}\": {field} mismatch\n--- expected ---\n{expected}--- produced ---\n{produced}"
            ),
            Self::Coverage { kind, missing } => write!(
                f,
                "the {} golden vector artifact no longer covers {} \"{missing}\"",
                kind.description(),
                kind.coverage()
            ),
            Self::MissingCausalCase => {
                f.write_str("the DL proof golden artifact is missing its causal NI case")
            }
            Self::Invariant {
                condition: FixtureInvariant::AbsentDocument,
                actual,
                ..
            } => write!(
                f,
                "an unrecorded answer must carry the absent-proof document, and it carries:\n{}",
                actual.as_deref().unwrap_or_default()
            ),
            Self::Invariant {
                condition: FixtureInvariant::AbsentExplanation,
                cause,
                ..
            } => {
                f.write_str("the refusal must say that nothing was recorded")?;
                if let Some(cause) = cause {
                    write!(f, ", and it says: {cause}")?;
                }
                Ok(())
            }
            Self::Invariant { condition, .. } => f.write_str(condition.message()),
            Self::RefusalEvidence { expected, cause } => write!(
                f,
                "the refusal must carry {expected:?}\n--- refusal ---\n{cause}"
            ),
        }
    }
}

impl core::error::Error for FixtureError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Grammar(cause) => Some(cause),
            Self::Case { cause, .. } | Self::RefusalEvidence { cause, .. } => Some(cause.as_ref()),
            Self::Invariant { cause, .. } => cause.as_deref().map(|cause| cause as _),
            Self::Empty { .. }
            | Self::Mismatch { .. }
            | Self::Coverage { .. }
            | Self::MissingCausalCase
            | Self::SchemaMissing { .. }
            | Self::SchemaControl { .. }
            | Self::SchemaInventory { .. } => None,
            Self::SchemaOutput { .. } => None,
        }
    }
}

impl purrdf_hash::error::StructuredError for FixtureError {
    purrdf_hash::error_visit! {
        SchemaOutput { condition, expected, produced } => "reasoning-fixture-schema-output" {
            "condition" => Text(condition.message()), "expected" => Text(expected), "produced" => Text(produced),
        },
        SchemaMissing { name } => "reasoning-fixture-schema-missing" { "name" => Text(name) },
        SchemaControl { name, expected, produced } => "reasoning-fixture-schema-control" {
            "name" => Text(name),
            "expected" => List(expected.iter().map(|value| purrdf_hash::error::ErrorValue::Text(value))),
            "produced" => List(produced.iter().map(|value| purrdf_hash::error::ErrorValue::Text(value))),
        },
        SchemaInventory { expected, produced } => "reasoning-fixture-schema-inventory" {
            "expected" => Unsigned(*expected as u128), "produced" => Unsigned(*produced as u128),
        },
        Grammar(cause) => "reasoning-fixture-grammar" { "cause" => Cause(cause) },
        Empty { kind } => "reasoning-fixture-empty" { "kind" => Code(kind.description()) },
        Case { name, cause } => "reasoning-fixture-case" { "name" => Text(name), "cause" => Cause(cause.as_ref()) },
        Mismatch { name, field, expected, produced } => "reasoning-fixture-mismatch" {
            "name" => Text(name), "field" => Code(field), "expected" => Text(expected), "produced" => Text(produced),
        },
        Coverage { kind, missing } => "reasoning-fixture-coverage" { "kind" => Code(kind.description()), "missing" => Text(missing) },
        MissingCausalCase => "reasoning-fixture-missing-causal-case" { "name" => Code("recursive-nominal-introduction") },
        Invariant { condition, actual, cause } => "reasoning-fixture-invariant" {
            "condition" => Text(condition.message()),
            "actual" => Value(actual.as_deref().map_or(purrdf_hash::error::ErrorValue::Absent, purrdf_hash::error::ErrorValue::Text)),
            "cause" => Value(cause.as_deref().map_or(purrdf_hash::error::ErrorValue::Absent, |cause| purrdf_hash::error::ErrorValue::Cause(cause))),
        },
        RefusalEvidence { expected, cause } => "reasoning-fixture-refusal-evidence" { "expected" => Text(expected), "cause" => Cause(cause.as_ref()) },
    }
}
