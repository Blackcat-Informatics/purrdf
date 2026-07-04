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

use std::collections::BTreeMap;

use purrdf_core::{RdfDiagnostic, RdfLocation, RdfSeverity, UnitInterner};
use purrdf_rdf::SpanTable;
use purrdf_shapes::report::{Severity, ValidationReport, ValidationResult};
use purrdf_shapes::term::Term;

use crate::model::{
    ArtifactLocation, Driver, Level, Location, LogicalLocation, Message, PhysicalLocation,
    PropertyBag, Region, ReportingDescriptor, Run, SarifLog, SarifResult, Tool,
};
use crate::path_syntax::render_path;

/// The tool name emitted in `driver.name`.
pub const TOOL_NAME: &str = "purrdf";

/// The symbolic `uriBaseId` used for artifact locations resolved relative to
/// [`SarifOptions::source_root_uri`]. It names the entry defined in
/// `run.originalUriBaseIds` (SARIF's indirection for a shared base URI).
pub const SRCROOT_BASE_ID: &str = "SRCROOT";

/// A property-bag key carrying a custom (`sh:severity`) IRI verbatim, so an
/// open-world SHACL severity is never lost when coerced to a SARIF `level`.
pub const PROP_SHACL_SEVERITY: &str = "shaclSeverity";

/// Optional source context that upgrades results from logical-only to
/// source-traced. All fields are optional — absent context degrades gracefully
/// to logical locations (the SARIF spec permits results with no physical span).
#[derive(Debug, Default, Clone, Copy)]
pub struct SarifSources<'a> {
    /// The artifact (data document) URI results are traced into, e.g. `data.ttl`.
    /// Required for any `physicalLocation`.
    pub artifact_uri: Option<&'a str>,
    /// The opt-in subject→source-position table from a tracked parse. Joins a
    /// focus node to the line/column where it was asserted.
    pub spans: Option<&'a SpanTable>,
    /// The provenance unit interner, used to resolve an [`Attribution`]'s runtime
    /// `UnitId` to its public slice IRI (S0.5: the numeric id never leaves here).
    ///
    /// [`Attribution`]: purrdf_core::Attribution
    pub units: Option<&'a UnitInterner>,
}

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

/// Build a SARIF log from a SHACL [`ValidationReport`], with no source context
/// (logical locations only).
///
/// For source-traced results (physical file/line locations and resolved
/// provenance), use [`build_report_sarif_with`].
#[must_use]
pub fn build_report_sarif(report: &ValidationReport, options: &SarifOptions) -> SarifLog {
    build_report_sarif_with(report, options, &SarifSources::default())
}

/// Build a SARIF log from a SHACL [`ValidationReport`] with source context.
///
/// When `sources` supplies an artifact URI and a span table, each result gains a
/// `physicalLocation` tracing its focus node back to a source line/column; the
/// source shape becomes a `relatedLocation`; and attributions resolve to public
/// slice IRIs. Results are ordered deterministically by
/// `(level, artifactUri, startLine, startColumn, ruleId, message)`.
#[must_use]
pub fn build_report_sarif_with(
    report: &ValidationReport,
    options: &SarifOptions,
    sources: &SarifSources<'_>,
) -> SarifLog {
    let base_id = source_root_base_id(options);
    let mut results: Vec<SarifResult> = report
        .results
        .iter()
        .map(|r| result_to_sarif(r, sources, base_id))
        .collect();

    sort_results(&mut results);
    let rules = register_rules(&mut results);
    let run = assemble_run(rules, results, options);
    SarifLog::single_run(run)
}

/// The deterministic result ordering: severity, then physical location (artifact
/// URI, start line, start column), then rule id, then message text.
fn sort_results(results: &mut [SarifResult]) {
    results.sort_by(|a, b| {
        level_rank(a.level)
            .cmp(&level_rank(b.level))
            .then_with(|| location_sort_key(a).cmp(&location_sort_key(b)))
            .then_with(|| a.rule_id.cmp(&b.rule_id))
            .then_with(|| a.message.text.cmp(&b.message.text))
    });
}

/// Extract `(artifactUri, startLine, startColumn)` from a result's primary
/// physical location for stable ordering. Results without a physical location
/// sort together (empty uri, line/column 0).
fn location_sort_key(result: &SarifResult) -> (String, u32, u32) {
    let phys = result
        .locations
        .first()
        .and_then(|l| l.physical_location.as_ref());
    let uri = phys
        .map(|p| p.artifact_location.uri.clone())
        .unwrap_or_default();
    let region = phys.and_then(|p| p.region.as_ref());
    let line = region.and_then(|r| r.start_line).unwrap_or(0);
    let column = region.and_then(|r| r.start_column).unwrap_or(0);
    (uri, line, column)
}

/// Build a SARIF log from a set of parse/ingest [`RdfDiagnostic`]s.
///
/// The diagnostic `code` is the `ruleId`; `severity` maps through
/// [`rdf_level`]; `message` (plus any `detail`) becomes the result message.
#[must_use]
pub fn build_diagnostics_sarif(diagnostics: &[RdfDiagnostic], options: &SarifOptions) -> SarifLog {
    let base_id = source_root_base_id(options);
    let mut results: Vec<SarifResult> = diagnostics
        .iter()
        .map(|d| diagnostic_to_sarif(d, base_id))
        .collect();

    sort_results(&mut results);
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
pub fn diagnostics_to_sarif_string(
    diagnostics: &[RdfDiagnostic],
    options: &SarifOptions,
) -> String {
    crate::model::to_json_pretty(&build_diagnostics_sarif(diagnostics, options))
}

/// SARIF rendering as a method on a validation report.
///
/// This is the ergonomic surface: with `use purrdf_validate::SarifReport;` in
/// scope, `report.to_sarif(&opts)` reads as a method — yet the writer never
/// leaves this boundary crate, so `purrdf-shapes` stays free of any SARIF/serde
/// concern.
///
/// # Examples
///
/// ```
/// use purrdf_validate::{SarifOptions, SarifReport};
/// use purrdf_shapes::report::ValidationReport;
///
/// let report = ValidationReport { conforms: true, results: vec![] };
/// let sarif = report.to_sarif(&SarifOptions::default());
/// assert!(sarif.contains("\"version\": \"2.1.0\""));
/// ```
pub trait SarifReport {
    /// Render this report to a SARIF 2.1.0 JSON string (logical locations only).
    fn to_sarif(&self, options: &SarifOptions) -> String;

    /// Render this report to SARIF with source context (physical locations,
    /// resolved provenance) from `sources`.
    fn to_sarif_with(&self, options: &SarifOptions, sources: &SarifSources<'_>) -> String;
}

impl SarifReport for ValidationReport {
    fn to_sarif(&self, options: &SarifOptions) -> String {
        report_to_sarif_string(self, options)
    }

    fn to_sarif_with(&self, options: &SarifOptions, sources: &SarifSources<'_>) -> String {
        crate::model::to_json_pretty(&build_report_sarif_with(self, options, sources))
    }
}

// ── internal ────────────────────────────────────────────────────────────────

fn result_to_sarif(
    result: &ValidationResult,
    sources: &SarifSources<'_>,
    base_id: Option<&str>,
) -> SarifResult {
    let mut properties = PropertyBag::new();
    if let Severity::Other(iri) = &result.severity {
        properties.insert(PROP_SHACL_SEVERITY, iri.as_str().to_owned());
    }

    let message = result
        .message
        .clone()
        .unwrap_or_else(|| synthesize_message(result));

    // Primary location: the focus node, with a physical span when the source is
    // tracked, plus logical locations for focus / result path / component.
    let mut logical = vec![LogicalLocation {
        name: result.focus_value(),
        fully_qualified_name: None,
        kind: Some("focusNode".to_owned()),
    }];
    if let Some(path) = &result.result_path {
        logical.push(LogicalLocation {
            name: path.to_string(),
            fully_qualified_name: result.path_structure.as_ref().map(render_path),
            kind: Some("resultPath".to_owned()),
        });
    }
    logical.push(LogicalLocation {
        name: result.source_constraint_component.as_str().to_owned(),
        fully_qualified_name: None,
        kind: Some("constraintComponent".to_owned()),
    });
    // Resolved slice attributions (S0.5: numeric UnitId -> public slice IRI here).
    if let Some(units) = sources.units {
        for attribution in &result.attributions {
            logical.push(LogicalLocation {
                name: units.name(attribution.unit).to_owned(),
                fully_qualified_name: attribution.evidence.clone(),
                kind: Some(attribution.role.as_str().to_owned()),
            });
        }
    }

    let physical = focus_physical_location(result, sources, base_id);
    let primary = Location {
        physical_location: physical,
        logical_locations: logical,
        message: None,
    };

    // The source shape as a related location ("shape defined here").
    let related = vec![Location {
        physical_location: None,
        logical_locations: vec![LogicalLocation {
            name: result.source_shape.to_string(),
            fully_qualified_name: None,
            kind: Some("sourceShape".to_owned()),
        }],
        message: Some(Message::text("shape defined here")),
    }];

    SarifResult {
        rule_id: result.source_constraint_component.as_str().to_owned(),
        rule_index: None,
        level: shacl_level(&result.severity),
        message: Message::text(message),
        locations: vec![primary],
        related_locations: related,
        properties,
    }
}

/// Join a result's focus node to a source position via the span table, producing
/// a `physicalLocation`. Requires both an artifact URI and a tracked span table;
/// otherwise `None` (the logical locations carry the result).
fn focus_physical_location(
    result: &ValidationResult,
    sources: &SarifSources<'_>,
    base_id: Option<&str>,
) -> Option<PhysicalLocation> {
    let uri = sources.artifact_uri?;
    let spans = sources.spans?;
    let key = focus_span_key(&result.focus_node)?;
    let position = spans.position_for_subject(&key)?;
    Some(PhysicalLocation {
        artifact_location: ArtifactLocation {
            uri: uri.to_owned(),
            uri_base_id: base_id.map(ToOwned::to_owned),
        },
        region: Some(Region {
            start_line: Some(position.line),
            start_column: Some(position.column),
            byte_offset: Some(position.byte_offset),
            ..Region::default()
        }),
    })
}

/// The span-table lookup key for a focus node: the bare IRI for a named node,
/// `_:label` for a blank node (matching the parser's subject-key convention).
fn focus_span_key(term: &Term) -> Option<String> {
    match term {
        Term::NamedNode(n) => Some(n.as_str().to_owned()),
        Term::BlankNode(label) => Some(format!("_:{label}")),
        Term::Literal(_) | Term::Triple(_) => None,
    }
}

fn diagnostic_to_sarif(diagnostic: &RdfDiagnostic, base_id: Option<&str>) -> SarifResult {
    let mut text = diagnostic.message.clone();
    if let Some(detail) = &diagnostic.detail {
        text.push_str(" (");
        text.push_str(detail);
        text.push(')');
    }

    let (locations, properties) = diagnostic.location.as_deref().map_or_else(
        || (Vec::new(), PropertyBag::new()),
        |location| diagnostic_location(location, base_id),
    );

    SarifResult {
        rule_id: diagnostic.code.clone(),
        rule_index: None,
        level: rdf_level(diagnostic.severity),
        message: Message::text(text),
        locations,
        related_locations: Vec::new(),
        properties,
    }
}

/// Map an [`RdfLocation`] on a diagnostic to a SARIF location: a
/// `physicalLocation` when a path/line is present, plus a property bag carrying
/// the GTS index anchors (`gts_quad_index`, etc.) that have no physical span.
fn diagnostic_location(
    location: &RdfLocation,
    base_id: Option<&str>,
) -> (Vec<Location>, PropertyBag) {
    let mut properties = PropertyBag::new();
    for (key, value) in [
        ("gtsTermId", location.gts_term_id),
        ("gtsQuadIndex", location.gts_quad_index),
        ("gtsReifierId", location.gts_reifier_id),
        ("gtsFrameIndex", location.gts_frame_index),
        ("gtsSegmentIndex", location.gts_segment_index),
    ] {
        if let Some(index) = value {
            properties.insert(key, index);
        }
    }

    let mut logical = Vec::new();
    if let Some(name) = &location.logical {
        logical.push(LogicalLocation {
            name: name.clone(),
            fully_qualified_name: None,
            kind: Some("logical".to_owned()),
        });
    }

    let physical = location.path.as_ref().map(|path| PhysicalLocation {
        artifact_location: ArtifactLocation {
            uri: path.clone(),
            uri_base_id: base_id.map(ToOwned::to_owned),
        },
        region: (location.line.is_some()).then(|| Region {
            start_line: location.line,
            start_column: location.column,
            ..Region::default()
        }),
    });

    if physical.is_none() && logical.is_empty() {
        return (Vec::new(), properties);
    }
    (
        vec![Location {
            physical_location: physical,
            logical_locations: logical,
            message: None,
        }],
        properties,
    )
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

    ids.iter()
        .map(|id| crate::rules::descriptor_for(id))
        .collect()
}

/// The symbolic base id to stamp on source-relative artifact locations, if the
/// caller pinned a `source_root_uri` (otherwise `None`, preserving the bare-URI
/// default).
fn source_root_base_id(options: &SarifOptions) -> Option<&'static str> {
    options.source_root_uri.as_ref().map(|_| SRCROOT_BASE_ID)
}

fn assemble_run(
    rules: Vec<ReportingDescriptor>,
    results: Vec<SarifResult>,
    options: &SarifOptions,
) -> Run {
    let mut original_uri_base_ids = BTreeMap::new();
    if let Some(root) = &options.source_root_uri {
        original_uri_base_ids.insert(
            SRCROOT_BASE_ID.to_owned(),
            ArtifactLocation {
                uri: root.clone(),
                uri_base_id: None,
            },
        );
    }

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
        original_uri_base_ids,
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
            source_shape: Term::NamedNode(NamedNode::new_unchecked(
                "http://example.org/PersonShape",
            )),
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
            r.properties
                .0
                .get(PROP_SHACL_SEVERITY)
                .and_then(|v| v.as_str()),
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
        assert!(
            text.contains("Value"),
            "message should name the value: {text}"
        );
        assert!(
            text.contains("on path"),
            "message should name the path: {text}"
        );
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
    fn physical_location_traces_focus_to_source_line() {
        use purrdf_rdf::{parse_dataset_with, ParseOptions};
        // alice is asserted on line 2 (leading blank line).
        let data = "\n<http://example.org/alice> <http://example.org/age> \"x\" .\n";
        let (_ds, spans) = parse_dataset_with(
            data.as_bytes(),
            "application/n-triples",
            None,
            &ParseOptions {
                track_source_spans: true,
            },
        )
        .expect("parse");
        let spans = spans.expect("span table present when tracking");

        let report = ValidationReport {
            conforms: false,
            results: vec![result(
                "http://www.w3.org/ns/shacl#DatatypeConstraintComponent",
                Severity::Violation,
                Some("bad"),
            )],
        };
        let sources = SarifSources {
            artifact_uri: Some("data.ttl"),
            spans: Some(&spans),
            units: None,
        };
        let log = build_report_sarif_with(&report, &SarifOptions::default(), &sources);
        let phys = log.runs[0].results[0].locations[0]
            .physical_location
            .as_ref()
            .expect("physical location present");
        assert_eq!(phys.artifact_location.uri, "data.ttl");
        assert_eq!(phys.region.as_ref().and_then(|r| r.start_line), Some(2));
    }

    #[test]
    fn physical_location_emits_zero_byte_offset() {
        use purrdf_rdf::{parse_dataset_with, ParseOptions};
        // alice is asserted on the FIRST line, so its subject starts at document
        // byte offset 0. That 0 must be emitted as `byteOffset: 0`, not dropped.
        let data = "<http://example.org/alice> <http://example.org/age> \"x\" .\n";
        let (_ds, spans) = parse_dataset_with(
            data.as_bytes(),
            "application/n-triples",
            None,
            &ParseOptions {
                track_source_spans: true,
            },
        )
        .expect("parse");
        let spans = spans.expect("span table present when tracking");

        let report = ValidationReport {
            conforms: false,
            results: vec![result(
                "http://www.w3.org/ns/shacl#DatatypeConstraintComponent",
                Severity::Violation,
                Some("bad"),
            )],
        };
        let sources = SarifSources {
            artifact_uri: Some("data.ttl"),
            spans: Some(&spans),
            units: None,
        };
        let log = build_report_sarif_with(&report, &SarifOptions::default(), &sources);
        let region = log.runs[0].results[0].locations[0]
            .physical_location
            .as_ref()
            .and_then(|p| p.region.as_ref())
            .expect("physical region present");
        assert_eq!(region.start_line, Some(1), "alice is on the first line");
        assert_eq!(
            region.byte_offset,
            Some(0),
            "a start-of-file byte offset of 0 must be emitted, not omitted"
        );
    }

    #[test]
    fn source_shape_is_a_related_location() {
        let report = ValidationReport {
            conforms: false,
            results: vec![result(
                "http://www.w3.org/ns/shacl#DatatypeConstraintComponent",
                Severity::Violation,
                Some("bad"),
            )],
        };
        let log = build_report_sarif(&report, &SarifOptions::default());
        let related = &log.runs[0].results[0].related_locations;
        assert_eq!(related.len(), 1);
        assert_eq!(
            related[0].logical_locations[0].kind.as_deref(),
            Some("sourceShape")
        );
        assert_eq!(
            related[0].message.as_ref().map(|m| m.text.as_str()),
            Some("shape defined here")
        );
    }

    #[test]
    fn complex_path_renders_as_sparql_path_syntax() {
        use purrdf_shapes::shapes::Path;
        let mut r = result(
            "http://www.w3.org/ns/shacl#MinCountConstraintComponent",
            Severity::Violation,
            Some("bad"),
        );
        r.path_structure = Some(Path::Inverse(Box::new(Path::Predicate(
            NamedNode::new_unchecked("http://example.org/parent"),
        ))));
        let report = ValidationReport {
            conforms: false,
            results: vec![r],
        };
        let log = build_report_sarif(&report, &SarifOptions::default());
        let path_loc = log.runs[0].results[0].locations[0]
            .logical_locations
            .iter()
            .find(|l| l.kind.as_deref() == Some("resultPath"))
            .expect("resultPath logical location");
        assert_eq!(
            path_loc.fully_qualified_name.as_deref(),
            Some("^<http://example.org/parent>")
        );
    }

    #[test]
    fn diagnostic_location_becomes_physical_location() {
        let diag = RdfDiagnostic::error("native-codec-parse", "unexpected token")
            .with_location(RdfLocation::file("data.ttl").with_line(3).with_column(5));
        let log = build_diagnostics_sarif(&[diag], &SarifOptions::default());
        let phys = log.runs[0].results[0].locations[0]
            .physical_location
            .as_ref()
            .expect("physical location");
        assert_eq!(phys.artifact_location.uri, "data.ttl");
        let region = phys.region.as_ref().expect("region");
        assert_eq!(region.start_line, Some(3));
        assert_eq!(region.start_column, Some(5));
    }

    #[test]
    fn emitted_sarif_satisfies_structural_invariants() {
        // A dependency-free structural check of the SARIF 2.1.0 shape, so the
        // `make check` gate (which does not run the Python jsonschema lane)
        // still guards the emitted structure. The Python binding test validates
        // the same output against the full vendored OASIS schema.
        let report = ValidationReport {
            conforms: false,
            results: vec![
                result(
                    "http://www.w3.org/ns/shacl#DatatypeConstraintComponent",
                    Severity::Violation,
                    None,
                ),
                result(
                    "http://www.w3.org/ns/shacl#MinCountConstraintComponent",
                    Severity::Warning,
                    Some("min"),
                ),
            ],
        };
        let json = report_to_sarif_string(&report, &SarifOptions::default());
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

        assert_eq!(value["version"], "2.1.0");
        assert!(value["$schema"].is_string());
        let runs = value["runs"].as_array().expect("runs array");
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0]["tool"]["driver"]["name"], "purrdf");

        let allowed = ["error", "warning", "note", "none"];
        for r in runs[0]["results"].as_array().expect("results array") {
            assert!(r["ruleId"].is_string(), "ruleId must be a string");
            assert!(r["message"]["text"].is_string(), "message.text required");
            let level = r["level"].as_str().expect("level string");
            assert!(
                allowed.contains(&level),
                "level {level} must be a SARIF level"
            );
            // Every result has a rule registered in the driver.
            let idx = r["ruleIndex"].as_u64().expect("ruleIndex") as usize;
            assert_eq!(runs[0]["tool"]["driver"]["rules"][idx]["id"], r["ruleId"]);
        }
    }

    #[test]
    fn registered_rules_carry_spec_metadata() {
        let report = ValidationReport {
            conforms: false,
            results: vec![result(
                "http://www.w3.org/ns/shacl#DatatypeConstraintComponent",
                Severity::Violation,
                Some("bad"),
            )],
        };
        let log = build_report_sarif(&report, &SarifOptions::default());
        let rule = &log.runs[0].tool.driver.rules[0];
        assert_eq!(rule.name.as_deref(), Some("DatatypeConstraintComponent"));
        assert_eq!(
            rule.help_uri.as_deref(),
            Some("https://www.w3.org/TR/shacl/#DatatypeConstraintComponent")
        );
        assert!(rule.short_description.is_some());
    }

    #[test]
    fn source_root_uri_wires_base_id_and_original_uri_base_ids() {
        use purrdf_rdf::{parse_dataset_with, ParseOptions};
        let data = "<http://example.org/alice> <http://example.org/age> \"x\" .\n";
        let (_ds, spans) = parse_dataset_with(
            data.as_bytes(),
            "application/n-triples",
            None,
            &ParseOptions {
                track_source_spans: true,
            },
        )
        .expect("parse");
        let spans = spans.expect("span table present when tracking");

        let report = ValidationReport {
            conforms: false,
            results: vec![result(
                "http://www.w3.org/ns/shacl#DatatypeConstraintComponent",
                Severity::Violation,
                Some("bad"),
            )],
        };
        let sources = SarifSources {
            artifact_uri: Some("alice.ttl"),
            spans: Some(&spans),
            units: None,
        };
        let options = SarifOptions {
            source_root_uri: Some("file:///src/".into()),
            ..SarifOptions::default()
        };
        let log = build_report_sarif_with(&report, &options, &sources);
        let run = &log.runs[0];

        // (a) the source-relative artifact location carries the SRCROOT base id.
        let phys = run.results[0].locations[0]
            .physical_location
            .as_ref()
            .expect("physical location present");
        assert_eq!(
            phys.artifact_location.uri_base_id.as_deref(),
            Some("SRCROOT")
        );

        // (b) run.originalUriBaseIds["SRCROOT"].uri is the pinned source root.
        let base = run
            .original_uri_base_ids
            .get("SRCROOT")
            .expect("SRCROOT base defined");
        assert_eq!(base.uri, "file:///src/");
        assert_eq!(base.uri_base_id, None);
    }

    #[test]
    fn default_emits_no_base_id_and_no_original_uri_base_ids() {
        use purrdf_rdf::{parse_dataset_with, ParseOptions};
        let data = "<http://example.org/alice> <http://example.org/age> \"x\" .\n";
        let (_ds, spans) = parse_dataset_with(
            data.as_bytes(),
            "application/n-triples",
            None,
            &ParseOptions {
                track_source_spans: true,
            },
        )
        .expect("parse");
        let spans = spans.expect("span table present when tracking");

        let report = ValidationReport {
            conforms: false,
            results: vec![result(
                "http://www.w3.org/ns/shacl#DatatypeConstraintComponent",
                Severity::Violation,
                Some("bad"),
            )],
        };
        let sources = SarifSources {
            artifact_uri: Some("alice.ttl"),
            spans: Some(&spans),
            units: None,
        };
        let log = build_report_sarif_with(&report, &SarifOptions::default(), &sources);
        let run = &log.runs[0];

        let phys = run.results[0].locations[0]
            .physical_location
            .as_ref()
            .expect("physical location present");
        assert_eq!(
            phys.artifact_location.uri_base_id, None,
            "no source_root_uri -> no uriBaseId"
        );
        assert!(
            run.original_uri_base_ids.is_empty(),
            "no source_root_uri -> no originalUriBaseIds"
        );

        // Absent from the serialized bytes entirely.
        let json = crate::model::to_json_pretty(&log);
        assert!(!json.contains("uriBaseId"), "uriBaseId must be omitted");
        assert!(
            !json.contains("originalUriBaseIds"),
            "originalUriBaseIds must be omitted"
        );
    }

    #[test]
    fn diagnostic_source_root_uri_stamps_base_id() {
        let diag = RdfDiagnostic::error("native-codec-parse", "unexpected token")
            .with_location(RdfLocation::file("data.ttl").with_line(3).with_column(5));
        let options = SarifOptions {
            source_root_uri: Some("file:///src/".into()),
            ..SarifOptions::default()
        };
        let log = build_diagnostics_sarif(&[diag], &options);
        let run = &log.runs[0];
        let phys = run.results[0].locations[0]
            .physical_location
            .as_ref()
            .expect("physical location");
        assert_eq!(
            phys.artifact_location.uri_base_id.as_deref(),
            Some("SRCROOT")
        );
        assert_eq!(
            run.original_uri_base_ids
                .get("SRCROOT")
                .map(|b| b.uri.as_str()),
            Some("file:///src/")
        );
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
                invocation_times: Some((
                    "2026-07-04T00:00:00Z".into(),
                    "2026-07-04T00:00:01Z".into(),
                )),
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
