// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Map PurRDF reports and diagnostics onto the SARIF [`model`](crate::model).
//!
//! This module owns the semantic mappings the SARIF spec requires:
//!
//! * severity → SARIF `level` (SHACL `Severity` and [`RdfSeverity`] alike),
//! * constraint-component / diagnostic code → SARIF `ruleId` (+ a deduplicated,
//!   sorted `driver.rules` table with `ruleIndex`),
//! * a validation message, or a synthesized actionable one when none is given
//!   (never a bare IRI dump), → `message.text`,
//! * a deterministic result ordering.
//!
//! Physical/logical locations are attached by [`crate::locate`]; this module
//! produces the location-free spine and is where the ordering and rule table are
//! established.

use purrdf_core::{RdfDiagnostic, RdfSeverity};
use purrdf_shapes::report::{Severity, ValidationReport, ValidationResult};

use crate::model::{
    Driver, Level, Message, PropertyBag, ReportingDescriptor, Run, SarifLog, SarifResult, Tool,
};

/// The tool name emitted in `driver.name`.
pub const TOOL_NAME: &str = "purrdf";

/// A property-bag key carrying a custom (`sh:severity`) IRI verbatim, so an
/// open-world SHACL severity is never lost when coerced to a SARIF `level`.
pub const PROP_SHACL_SEVERITY: &str = "shaclSeverity";

/// Caller-supplied SARIF emission options. Everything is optional; the defaults
/// produce a minimal, timestamp-free, deterministic log.
#[derive(Debug, Clone, Default)]
pub struct SarifOptions {
    /// The tool version to emit as `driver.version`.
    pub tool_version: Option<String>,
    /// A URI for `driver.informationUri`.
    pub information_uri: Option<String>,
    /// Caller-supplied `(startTimeUtc, endTimeUtc)` for a single invocation
    /// record. Emitted verbatim — the crate never samples the clock.
    pub invocation_times: Option<(String, String)>,
    /// A base URI the artifact locations are relative to (`uriBaseId`/root).
    pub source_root_uri: Option<String>,
}

/// SARIF `level` for a SHACL [`Severity`]. Open-world `Other` maps to `note`; the
/// verbatim IRI is preserved in the result's property bag (see
/// [`PROP_SHACL_SEVERITY`]) so the mapping is non-lossy.
#[must_use]
pub fn shacl_level(severity: &Severity) -> Level {
    match severity {
        Severity::Violation => Level::Error,
        Severity::Warning => Level::Warning,
        Severity::Info | Severity::Other(_) => Level::Note,
    }
}

/// SARIF `level` for an [`RdfSeverity`]. `Note` and `Info` both map to SARIF
/// `note` (SARIF has no distinct "info").
#[must_use]
pub fn rdf_level(severity: RdfSeverity) -> Level {
    match severity {
        RdfSeverity::Error => Level::Error,
        RdfSeverity::Warning => Level::Warning,
        RdfSeverity::Note | RdfSeverity::Info => Level::Note,
    }
}

/// Ordering rank for a level (most severe first). Part of the deterministic
/// result sort key.
#[must_use]
pub fn level_rank(level: Level) -> u8 {
    match level {
        Level::Error => 0,
        Level::Warning => 1,
        Level::Note => 2,
        Level::None => 3,
    }
}

/// Build a SARIF log from a SHACL [`ValidationReport`].
///
/// Results are location-free here (see [`crate::locate`] for physical/logical
/// locations) and ordered by `(level, ruleId, message)` for determinism.
#[must_use]
pub fn build_report_sarif(report: &ValidationReport, options: &SarifOptions) -> SarifLog {
    let mut results: Vec<SarifResult> = report.results.iter().map(result_to_sarif).collect();

    // Deterministic order: severity, then rule id, then message text.
    results.sort_by(|a, b| {
        level_rank(a.level)
            .cmp(&level_rank(b.level))
            .then_with(|| a.rule_id.cmp(&b.rule_id))
            .then_with(|| a.message.text.cmp(&b.message.text))
    });

    let rules = register_rules(&mut results);
    let run = assemble_run(rules, results, options);
    SarifLog::single_run(run)
}

/// Build a SARIF log from a set of parse/ingest [`RdfDiagnostic`]s.
///
/// The diagnostic `code` is the `ruleId`; `severity` maps through
/// [`rdf_level`]; `message` (plus any `detail`) becomes the result message.
#[must_use]
pub fn build_diagnostics_sarif(diagnostics: &[RdfDiagnostic], options: &SarifOptions) -> SarifLog {
    let mut results: Vec<SarifResult> = diagnostics.iter().map(diagnostic_to_sarif).collect();

    results.sort_by(|a, b| {
        level_rank(a.level)
            .cmp(&level_rank(b.level))
            .then_with(|| a.rule_id.cmp(&b.rule_id))
            .then_with(|| a.message.text.cmp(&b.message.text))
    });

    let rules = register_rules(&mut results);
    let run = assemble_run(rules, results, options);
    SarifLog::single_run(run)
}

/// Serialize [`build_report_sarif`] to deterministic pretty JSON.
#[must_use]
pub fn report_to_sarif_string(report: &ValidationReport, options: &SarifOptions) -> String {
    crate::model::to_json_pretty(&build_report_sarif(report, options))
}

/// Serialize [`build_diagnostics_sarif`] to deterministic pretty JSON.
#[must_use]
pub fn diagnostics_to_sarif_string(diagnostics: &[RdfDiagnostic], options: &SarifOptions) -> String {
    crate::model::to_json_pretty(&build_diagnostics_sarif(diagnostics, options))
}

// ── internal ────────────────────────────────────────────────────────────────

fn result_to_sarif(result: &ValidationResult) -> SarifResult {
    let mut properties = PropertyBag::new();
    if let Severity::Other(iri) = &result.severity {
        properties.insert(PROP_SHACL_SEVERITY, iri.as_str().to_owned());
    }

    let message = result
        .message
        .clone()
        .unwrap_or_else(|| synthesize_message(result));

    SarifResult {
        rule_id: result.source_constraint_component.as_str().to_owned(),
        rule_index: None,
        level: shacl_level(&result.severity),
        message: Message::text(message),
        locations: Vec::new(),
        related_locations: Vec::new(),
        properties,
    }
}

fn diagnostic_to_sarif(diagnostic: &RdfDiagnostic) -> SarifResult {
    let mut text = diagnostic.message.clone();
    if let Some(detail) = &diagnostic.detail {
        text.push_str(" (");
        text.push_str(detail);
        text.push(')');
    }

    SarifResult {
        rule_id: diagnostic.code.clone(),
        rule_index: None,
        level: rdf_level(diagnostic.severity),
        message: Message::text(text),
        locations: Vec::new(),
        related_locations: Vec::new(),
        properties: PropertyBag::new(),
    }
}

/// Synthesize an actionable message from a result's structured parts — never a
/// bare IRI dump. Example:
/// `Value "foo" on path ex:age violates sh:datatype at focus node ex:alice (shape ex:PersonShape)`.
fn synthesize_message(result: &ValidationResult) -> String {
    let mut clause = String::new();
    if let Some(value) = &result.value {
        clause.push_str("Value ");
        clause.push_str(&value.to_string());
        clause.push(' ');
    }
    if let Some(path) = &result.result_path {
        clause.push_str("on path ");
        clause.push_str(&path.to_string());
        clause.push(' ');
    }
    format!(
        "{clause}violates {} at focus node {} (shape {})",
        result.source_constraint_component.as_str(),
        result.focus_value(),
        result.source_shape,
    )
}

/// Deduplicate the rule ids used by `results` into a sorted `driver.rules` table
/// and stamp each result's `ruleIndex`.
fn register_rules(results: &mut [SarifResult]) -> Vec<ReportingDescriptor> {
    let mut ids: Vec<String> = results.iter().map(|r| r.rule_id.clone()).collect();
    ids.sort_unstable();
    ids.dedup();

    for result in results.iter_mut() {
        result.rule_index = ids.iter().position(|id| *id == result.rule_id);
    }

    ids.into_iter()
        .map(|id| ReportingDescriptor {
            id,
            name: None,
            short_description: None,
            full_description: None,
            help: None,
            help_uri: None,
            default_configuration: None,
        })
        .collect()
}

fn assemble_run(
    rules: Vec<ReportingDescriptor>,
    results: Vec<SarifResult>,
    options: &SarifOptions,
) -> Run {
    let invocations = options
        .invocation_times
        .as_ref()
        .map(|(start, end)| {
            vec![crate::model::Invocation {
                execution_successful: true,
                start_time_utc: Some(start.clone()),
                end_time_utc: Some(end.clone()),
            }]
        })
        .unwrap_or_default();

    Run {
        tool: Tool {
            driver: Driver {
                name: TOOL_NAME.to_owned(),
                version: options.tool_version.clone(),
                information_uri: options.information_uri.clone(),
                rules,
            },
        },
        results,
        invocations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_shapes::term::{Literal, NamedNode, Term};

    fn result(component: &str, severity: Severity, message: Option<&str>) -> ValidationResult {
        ValidationResult {
            focus_node: Term::NamedNode(NamedNode::new_unchecked("http://example.org/alice")),
            result_path: Some(Term::NamedNode(NamedNode::new_unchecked(
                "http://example.org/age",
            ))),
            path_structure: None,
            value: Some(Term::Literal(Literal::new_simple_literal("foo"))),
            source_constraint_component: NamedNode::new_unchecked(component),
            source_shape: Term::NamedNode(NamedNode::new_unchecked("http://example.org/PersonShape")),
            severity,
            message: message.map(ToOwned::to_owned),
            source_box_roles: vec![],
            path_box_roles: vec![],
            result_box_roles: vec![],
            attributions: vec![],
        }
    }

    #[test]
    fn severity_maps_and_other_is_non_lossy() {
        let custom = Severity::Other(NamedNode::new_unchecked("http://example.org/Critical"));
        let report = ValidationReport {
            conforms: false,
            results: vec![result(
                "http://www.w3.org/ns/shacl#DatatypeConstraintComponent",
                custom,
                None,
            )],
        };
        let log = build_report_sarif(&report, &SarifOptions::default());
        let r = &log.runs[0].results[0];
        assert_eq!(r.level, Level::Note); // Other -> note
        assert_eq!(
            r.properties.0.get(PROP_SHACL_SEVERITY).and_then(|v| v.as_str()),
            Some("http://example.org/Critical"),
            "custom severity IRI must be preserved verbatim"
        );
    }

    #[test]
    fn synthesized_message_is_actionable_not_a_bare_iri() {
        let report = ValidationReport {
            conforms: false,
            results: vec![result(
                "http://www.w3.org/ns/shacl#DatatypeConstraintComponent",
                Severity::Violation,
                None,
            )],
        };
        let log = build_report_sarif(&report, &SarifOptions::default());
        let text = &log.runs[0].results[0].message.text;
        assert!(text.contains("Value"), "message should name the value: {text}");
        assert!(text.contains("on path"), "message should name the path: {text}");
        assert!(
            text.contains("focus node http://example.org/alice"),
            "message should name the focus node: {text}"
        );
    }

    #[test]
    fn results_are_sorted_and_rules_deduplicated() {
        let report = ValidationReport {
            conforms: false,
            results: vec![
                // A warning (less severe) listed first — must sort AFTER the violation.
                result(
                    "http://www.w3.org/ns/shacl#MinCountConstraintComponent",
                    Severity::Warning,
                    Some("warn"),
                ),
                result(
                    "http://www.w3.org/ns/shacl#DatatypeConstraintComponent",
                    Severity::Violation,
                    Some("boom"),
                ),
                // Duplicate component id — must collapse to one rule.
                result(
                    "http://www.w3.org/ns/shacl#DatatypeConstraintComponent",
                    Severity::Violation,
                    Some("boom2"),
                ),
            ],
        };
        let log = build_report_sarif(&report, &SarifOptions::default());
        let run = &log.runs[0];
        // Violations (error) sort before the warning.
        assert_eq!(run.results[0].level, Level::Error);
        assert_eq!(run.results[2].level, Level::Warning);
        // Two distinct component ids -> two rules.
        assert_eq!(run.tool.driver.rules.len(), 2);
        // ruleIndex points into the rules table.
        for r in &run.results {
            let idx = r.rule_index.expect("rule index set");
            assert_eq!(run.tool.driver.rules[idx].id, r.rule_id);
        }
    }

    #[test]
    fn caller_times_emit_an_invocation_and_default_omits_it() {
        let report = ValidationReport {
            conforms: true,
            results: vec![],
        };
        let none = build_report_sarif(&report, &SarifOptions::default());
        assert!(none.runs[0].invocations.is_empty());

        let timed = build_report_sarif(
            &report,
            &SarifOptions {
                invocation_times: Some(("2026-07-04T00:00:00Z".into(), "2026-07-04T00:00:01Z".into())),
                ..SarifOptions::default()
            },
        );
        assert_eq!(timed.runs[0].invocations.len(), 1);
        assert_eq!(
            timed.runs[0].invocations[0].start_time_utc.as_deref(),
            Some("2026-07-04T00:00:00Z")
        );
    }
}
