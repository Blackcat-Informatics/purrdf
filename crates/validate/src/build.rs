// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
//! Physical, logical, and related locations are attached inline here (see
//! `focus_physical_location` and `diagnostic_location`), alongside the result
//! spine where the ordering and rule table are established.

use std::collections::BTreeMap;

use purrdf_core::{RdfDiagnostic, RdfLocation, RdfSeverity, UnitInterner};
use purrdf_rdf::SpanTable;
use purrdf_shapes::engine::ValidationOptions;
use purrdf_shapes::report::{Severity, ValidationReport, ValidationResult};
use purrdf_shapes::term::{Literal, Term};

use crate::model::{
    ArtifactLocation, Driver, Level, Location, LogicalLocation, Message, PhysicalLocation,
    PropertyBag, Region, ReportingDescriptor, ResultKind, Run, SarifLog, SarifResult, Tool,
};
use crate::path_syntax::render_path;

/// The tool name emitted in `driver.name`.
pub const TOOL_NAME: &str = "purrdf";

/// The symbolic `uriBaseId` used for artifact locations resolved relative to
/// [`SarifOptions::source_root_uri`]. It names the entry defined in
/// `run.originalUriBaseIds` (SARIF's indirection for a shared base URI).
pub const SRCROOT_BASE_ID: &str = "SRCROOT";

/// A property-bag key carrying a `sh:severity` IRI verbatim wherever the SARIF
/// `level` cannot tell it apart — a custom severity, and the `sh:Debug` /
/// `sh:Trace` levels that both map to `none` — so no SHACL severity is lost.
pub const PROP_SHACL_SEVERITY: &str = "shaclSeverity";

/// A result property carrying EVERY `sh:resultMessage` of the SHACL result, in
/// the report's canonical order, each as `{"text": …}` plus `"language"`,
/// `"direction"` (`"ltr"`/`"rtl"`) and `"datatype"` when the literal has them
/// (`datatype` is omitted for `xsd:string` and the language-string types, which
/// the other two keys already say). SARIF's `message.text` is one plain string, so
/// it carries the [`primary_message`] and this bag carries all of them — present
/// whenever `message.text` alone would lose something, that is, unless the result
/// has no message or exactly one untagged `xsd:string` message.
pub const PROP_SHACL_MESSAGES: &str = "shaclMessages";

/// The run-level property carrying the SHACL report's `sh:conforms`.
///
/// A SARIF log alone cannot say whether the data conforms: `sh:Debug` and
/// `sh:Trace` results appear in the log of a conforming report, and whether a
/// `sh:Warning` or `sh:Info` result blocks conformance depends on the request's
/// conformance-disallow set. So a report log states it.
pub const PROP_SHACL_CONFORMS: &str = "shaclConforms";

/// The run-level property carrying the conformance-disallow set the report was
/// judged against, as severity IRIs in [`ConformanceDisallows::levels`] order.
///
/// [`ConformanceDisallows::levels`]: purrdf_shapes::report::ConformanceDisallows::levels
pub const PROP_SHACL_CONFORMANCE_DISALLOWS: &str = "shaclConformanceDisallows";

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
    /// The validation request's options — its conformance-disallow set — for the
    /// entry points that VALIDATE before rendering
    /// ([`crate::validate_to_sarif_string`], [`crate::validate_changes_to_sarif_string`]
    /// and the prepared-product validations). Defaults to the SHACL default set
    /// (`sh:Violation`, `sh:Warning`, `sh:Info`). Rendering an already-produced
    /// report reads the set the report carries, never this field.
    pub validation: ValidationOptions,
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

/// SARIF `level` for a SHACL [`Severity`].
///
/// `sh:Violation` is `error`, `sh:Warning` is `warning`, and `sh:Info` — "a
/// non-critical constraint violation indicating an informative message" — is
/// `note`, as is an open-world `Other` severity. `sh:Debug` and `sh:Trace` are "not
/// a constraint violation", so their result is SARIF `kind` `informational`
/// ([`shacl_kind`]), and SARIF 2.1.0 §3.27.10 then fixes the level: "If kind has
/// any value other than 'fail', then if level is absent, it SHALL default to
/// 'none', and if it is present, it SHALL have the value 'none'." Wherever the
/// level cannot tell two severities apart (`Other`, `Debug`, `Trace`), the
/// verbatim IRI is preserved in the result's property bag (see
/// [`PROP_SHACL_SEVERITY`]) so the mapping is non-lossy.
#[must_use]
pub fn shacl_level(severity: &Severity) -> Level {
    match severity {
        Severity::Violation => Level::Error,
        Severity::Warning => Level::Warning,
        Severity::Info | Severity::Other(_) => Level::Note,
        Severity::Debug | Severity::Trace => Level::None,
    }
}

/// SARIF result `kind` for a SHACL [`Severity`]: `informational` for `sh:Debug`
/// and `sh:Trace`, which SHACL defines as "not a constraint violation", and absent
/// (SARIF's default, `fail`) for every level that is one.
#[must_use]
pub const fn shacl_kind(severity: &Severity) -> Option<ResultKind> {
    match severity {
        Severity::Debug | Severity::Trace => Some(ResultKind::Informational),
        Severity::Violation | Severity::Warning | Severity::Info | Severity::Other(_) => None,
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
    let mut run = assemble_run(rules, results, options);
    run.properties.insert(
        PROP_SHACL_CONFORMS,
        serde_json::Value::Bool(report.conforms),
    );
    run.properties.insert(
        PROP_SHACL_CONFORMANCE_DISALLOWS,
        report.conformance_disallows.iris(),
    );
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
    let uri = phys.map_or_default(|p| p.artifact_location.uri.clone());
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
/// use purrdf_shapes::report::{ConformanceDisallows, ValidationReport};
///
/// let report = ValidationReport::from_results(vec![], ConformanceDisallows::default());
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
    if matches!(
        result.severity,
        Severity::Other(_) | Severity::Debug | Severity::Trace
    ) {
        properties.insert(PROP_SHACL_SEVERITY, result.severity.iri().to_owned());
    }

    let message = primary_message(&result.messages).unwrap_or_else(|| synthesize_message(result));
    // A lone untagged `xsd:string` message is carried whole by `message.text`;
    // any other message set — several messages, a language tag, a direction, an
    // `rdf:HTML` literal — needs the bag, or something would be lost.
    let text_carries_all = match result.messages.as_slice() {
        [only] => only.language().is_none() && only.datatype_str() == XSD_STRING,
        _ => result.messages.is_empty(),
    };
    if !text_carries_all {
        properties.insert(PROP_SHACL_MESSAGES, shacl_messages(&result.messages));
    }

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

    // The source shape as a related location ("shape defined here"), then every
    // nested result that details this one.
    let mut related = vec![Location {
        physical_location: None,
        logical_locations: vec![LogicalLocation {
            name: result.source_shape.to_string(),
            fully_qualified_name: None,
            kind: Some("sourceShape".to_owned()),
        }],
        message: Some(Message::text("shape defined here")),
    }];
    push_detail_locations(result, sources, base_id, &mut related);

    SarifResult {
        rule_id: result.source_constraint_component.as_str().to_owned(),
        rule_index: None,
        kind: shacl_kind(&result.severity),
        level: shacl_level(&result.severity),
        message: Message::text(message),
        locations: vec![primary],
        related_locations: related,
        properties,
    }
}

/// Append one related location per result that details `result` (`sh:detail`,
/// SHACL 1.2 Core): for a `sh:memberShape` result, each non-conforming list
/// member's results; for `sh:uniqueMembers`, each duplicated member's.
///
/// SARIF's `relatedLocations` is a flat list, so a detail's own details follow
/// it, depth first, in the report's deterministic order. Each location carries
/// the detail's focus node (with its source span when tracked), result path,
/// constraint component and source shape as logical locations, and its message
/// — the detail's own, or one synthesized from its parts — prefixed `sh:detail`,
/// so nothing a nested result says is lost.
fn push_detail_locations(
    result: &ValidationResult,
    sources: &SarifSources<'_>,
    base_id: Option<&str>,
    related: &mut Vec<Location>,
) {
    let mut pending: Vec<&ValidationResult> = result.details.iter().rev().collect();
    while let Some(detail) = pending.pop() {
        let mut logical = vec![LogicalLocation {
            name: detail.focus_value(),
            fully_qualified_name: None,
            kind: Some("focusNode".to_owned()),
        }];
        if let Some(path) = &detail.result_path {
            logical.push(LogicalLocation {
                name: path.to_string(),
                fully_qualified_name: detail.path_structure.as_ref().map(render_path),
                kind: Some("resultPath".to_owned()),
            });
        }
        logical.push(LogicalLocation {
            name: detail.source_constraint_component.as_str().to_owned(),
            fully_qualified_name: None,
            kind: Some("constraintComponent".to_owned()),
        });
        logical.push(LogicalLocation {
            name: detail.source_shape.to_string(),
            fully_qualified_name: None,
            kind: Some("sourceShape".to_owned()),
        });
        let text = primary_message(&detail.messages).unwrap_or_else(|| synthesize_message(detail));
        related.push(Location {
            physical_location: focus_physical_location(detail, sources, base_id),
            logical_locations: logical,
            message: Some(Message::text(format!("sh:detail: {text}"))),
        });
        pending.extend(detail.details.iter().rev());
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
        kind: None,
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

/// The one message SARIF's `message.text` carries: the first untagged `xsd:string`
/// message in the report's canonical order (the language-neutral one, when the
/// shapes graph declares one), else the first message in that order. `None` when
/// the result has no message, so the text is synthesized. Every message, tagged
/// or not, is in the [`PROP_SHACL_MESSAGES`] property.
fn primary_message(messages: &[Literal]) -> Option<String> {
    messages
        .iter()
        .find(|m| m.language().is_none() && m.datatype_str() == XSD_STRING)
        .or_else(|| messages.first())
        .map(|m| m.value().to_owned())
}

/// The [`PROP_SHACL_MESSAGES`] value for `messages`.
fn shacl_messages(messages: &[Literal]) -> serde_json::Value {
    serde_json::Value::Array(
        messages
            .iter()
            .map(|m| {
                let mut entry = serde_json::Map::new();
                entry.insert("text".to_owned(), m.value().into());
                if let Some(language) = m.language() {
                    entry.insert("language".to_owned(), language.into());
                }
                if let Some(direction) = m.direction() {
                    entry.insert(
                        "direction".to_owned(),
                        match direction {
                            purrdf_core::RdfTextDirection::Ltr => "ltr",
                            purrdf_core::RdfTextDirection::Rtl => "rtl",
                        }
                        .into(),
                    );
                }
                if m.language().is_none() && m.datatype_str() != XSD_STRING {
                    entry.insert("datatype".to_owned(), m.datatype_str().into());
                }
                serde_json::Value::Object(entry)
            })
            .collect(),
    )
}

/// `xsd:string`.
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

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
        .map_or_default(|(start, end)| {
            vec![crate::model::Invocation {
                execution_successful: true,
                start_time_utc: Some(start.clone()),
                end_time_utc: Some(end.clone()),
            }]
        });

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
        properties: PropertyBag::new(),
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
            messages: message
                .map(|m| vec![Literal::new_simple_literal(m)])
                .unwrap_or_default(),
            source_box_roles: vec![],
            path_box_roles: vec![],
            result_box_roles: vec![],
            attributions: vec![],
            details: vec![],
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
            conformance_disallows: purrdf_shapes::report::ConformanceDisallows::default(),
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

    /// `sh:Debug` and `sh:Trace` are "not a constraint violation": SARIF kind
    /// `informational`, which fixes the level at `none`, with the two told apart
    /// by the verbatim IRI; `sh:Info` stays a `fail`-kind `note` (the control).
    #[test]
    fn sarif_debug_and_trace_are_informational_level_none() {
        let report = ValidationReport::from_results(
            vec![
                result(
                    "http://www.w3.org/ns/shacl#DatatypeConstraintComponent",
                    Severity::Debug,
                    Some("debug"),
                ),
                result(
                    "http://www.w3.org/ns/shacl#DatatypeConstraintComponent",
                    Severity::Trace,
                    Some("trace"),
                ),
                result(
                    "http://www.w3.org/ns/shacl#DatatypeConstraintComponent",
                    Severity::Info,
                    Some("info"),
                ),
            ],
            purrdf_shapes::report::ConformanceDisallows::default(),
        );
        let log = build_report_sarif(&report, &SarifOptions::default());
        let by_message = |text: &str| {
            log.runs[0]
                .results
                .iter()
                .find(|r| r.message.text == text)
                .expect("the result is rendered")
        };
        for (text, iri) in [
            ("debug", "http://www.w3.org/ns/shacl#Debug"),
            ("trace", "http://www.w3.org/ns/shacl#Trace"),
        ] {
            let r = by_message(text);
            assert_eq!(r.level, Level::None, "{text}");
            assert_eq!(r.kind, Some(ResultKind::Informational), "{text}");
            assert_eq!(
                r.properties
                    .0
                    .get(PROP_SHACL_SEVERITY)
                    .and_then(|v| v.as_str()),
                Some(iri)
            );
        }
        let info = by_message("info");
        assert_eq!(info.level, Level::Note);
        assert_eq!(info.kind, None);
        assert!(!info.properties.0.contains_key(PROP_SHACL_SEVERITY));
        let json = crate::model::to_json_pretty(&log);
        assert!(json.contains("\"kind\": \"informational\""));
        assert!(json.contains("\"level\": \"none\""));
    }

    /// A report log states `sh:conforms` and the set it was judged against: a
    /// conforming report can carry results, so the results alone cannot say.
    #[test]
    fn sarif_run_properties_carry_conforms_and_the_disallow_set() {
        let debug_only = ValidationReport::from_results(
            vec![result(
                "http://www.w3.org/ns/shacl#DatatypeConstraintComponent",
                Severity::Debug,
                None,
            )],
            purrdf_shapes::report::ConformanceDisallows::default(),
        );
        let log = build_report_sarif(&debug_only, &SarifOptions::default());
        let properties = &log.runs[0].properties.0;
        assert_eq!(
            properties.get(PROP_SHACL_CONFORMS),
            Some(&serde_json::Value::Bool(true))
        );
        assert_eq!(
            properties.get(PROP_SHACL_CONFORMANCE_DISALLOWS),
            Some(&serde_json::json!([
                "http://www.w3.org/ns/shacl#Violation",
                "http://www.w3.org/ns/shacl#Warning",
                "http://www.w3.org/ns/shacl#Info"
            ]))
        );
        let warning_only = ValidationReport::from_results(
            vec![result(
                "http://www.w3.org/ns/shacl#DatatypeConstraintComponent",
                Severity::Warning,
                None,
            )],
            purrdf_shapes::report::ConformanceDisallows::new([Severity::Violation])
                .expect("non-empty"),
        );
        let log = build_report_sarif(&warning_only, &SarifOptions::default());
        let properties = &log.runs[0].properties.0;
        assert_eq!(
            properties.get(PROP_SHACL_CONFORMS),
            Some(&serde_json::Value::Bool(true))
        );
        assert_eq!(
            properties.get(PROP_SHACL_CONFORMANCE_DISALLOWS),
            Some(&serde_json::json!(["http://www.w3.org/ns/shacl#Violation"]))
        );
        // A diagnostics log is not a SHACL report and states neither.
        let diagnostics = build_diagnostics_sarif(&[], &SarifOptions::default());
        assert!(diagnostics.runs[0].properties.is_empty());
    }

    /// Every message reaches SARIF: `message.text` is the untagged one when there
    /// is one (else the first in canonical order), and `shaclMessages` carries
    /// them all with their languages.
    #[test]
    fn sarif_carries_every_message_with_its_language() {
        let mut tagged = result(
            "http://www.w3.org/ns/shacl#MaxLengthConstraintComponent",
            Severity::Violation,
            None,
        );
        tagged.messages = purrdf_shapes::report::canonical_messages(vec![
            Literal::new_language_tagged_literal_unchecked("Zu viele", "de"),
            Literal::new_language_tagged_literal_unchecked("Too many", "en"),
        ]);
        let mut mixed = tagged.clone();
        mixed.messages.push(Literal::new_simple_literal("plain"));
        mixed.messages = purrdf_shapes::report::canonical_messages(mixed.messages);
        for (r, primary) in [(tagged, "Too many"), (mixed, "plain")] {
            let report = ValidationReport::from_results(
                vec![r],
                purrdf_shapes::report::ConformanceDisallows::default(),
            );
            let log = build_report_sarif(&report, &SarifOptions::default());
            let out = &log.runs[0].results[0];
            assert_eq!(out.message.text, primary);
            let all = out
                .properties
                .0
                .get(PROP_SHACL_MESSAGES)
                .expect("every message");
            let languages: Vec<Option<&str>> = all
                .as_array()
                .expect("array")
                .iter()
                .map(|m| m.get("language").and_then(|l| l.as_str()))
                .collect();
            assert!(languages.contains(&Some("en")) && languages.contains(&Some("de")));
        }
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
            conformance_disallows: purrdf_shapes::report::ConformanceDisallows::default(),
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
            conformance_disallows: purrdf_shapes::report::ConformanceDisallows::default(),
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
        use purrdf_rdf::{ParseOptions, parse_dataset_with};
        // alice is asserted on line 2 (leading blank line).
        let data = "\n<http://example.org/alice> <http://example.org/age> \"x\" .\n";
        let spans = parse_dataset_with(
            data.as_bytes(),
            "application/n-triples",
            None,
            &ParseOptions {
                track_source_spans: true,
            },
        )
        .expect("parse")
        .spans
        .expect("span table present when tracking");

        let report = ValidationReport {
            conforms: false,
            results: vec![result(
                "http://www.w3.org/ns/shacl#DatatypeConstraintComponent",
                Severity::Violation,
                Some("bad"),
            )],
            conformance_disallows: purrdf_shapes::report::ConformanceDisallows::default(),
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
        use purrdf_rdf::{ParseOptions, parse_dataset_with};
        // alice is asserted on the FIRST line, so its subject starts at document
        // byte offset 0. That 0 must be emitted as `byteOffset: 0`, not dropped.
        let data = "<http://example.org/alice> <http://example.org/age> \"x\" .\n";
        let spans = parse_dataset_with(
            data.as_bytes(),
            "application/n-triples",
            None,
            &ParseOptions {
                track_source_spans: true,
            },
        )
        .expect("parse")
        .spans
        .expect("span table present when tracking");

        let report = ValidationReport {
            conforms: false,
            results: vec![result(
                "http://www.w3.org/ns/shacl#DatatypeConstraintComponent",
                Severity::Violation,
                Some("bad"),
            )],
            conformance_disallows: purrdf_shapes::report::ConformanceDisallows::default(),
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
            conformance_disallows: purrdf_shapes::report::ConformanceDisallows::default(),
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
            conformance_disallows: purrdf_shapes::report::ConformanceDisallows::default(),
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
            conformance_disallows: purrdf_shapes::report::ConformanceDisallows::default(),
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
            conformance_disallows: purrdf_shapes::report::ConformanceDisallows::default(),
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
        use purrdf_rdf::{ParseOptions, parse_dataset_with};
        let data = "<http://example.org/alice> <http://example.org/age> \"x\" .\n";
        let spans = parse_dataset_with(
            data.as_bytes(),
            "application/n-triples",
            None,
            &ParseOptions {
                track_source_spans: true,
            },
        )
        .expect("parse")
        .spans
        .expect("span table present when tracking");

        let report = ValidationReport {
            conforms: false,
            results: vec![result(
                "http://www.w3.org/ns/shacl#DatatypeConstraintComponent",
                Severity::Violation,
                Some("bad"),
            )],
            conformance_disallows: purrdf_shapes::report::ConformanceDisallows::default(),
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
        use purrdf_rdf::{ParseOptions, parse_dataset_with};
        let data = "<http://example.org/alice> <http://example.org/age> \"x\" .\n";
        let spans = parse_dataset_with(
            data.as_bytes(),
            "application/n-triples",
            None,
            &ParseOptions {
                track_source_spans: true,
            },
        )
        .expect("parse")
        .spans
        .expect("span table present when tracking");

        let report = ValidationReport {
            conforms: false,
            results: vec![result(
                "http://www.w3.org/ns/shacl#DatatypeConstraintComponent",
                Severity::Violation,
                Some("bad"),
            )],
            conformance_disallows: purrdf_shapes::report::ConformanceDisallows::default(),
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
            conformance_disallows: purrdf_shapes::report::ConformanceDisallows::default(),
        };
        let none = build_report_sarif(&report, &SarifOptions::default());
        assert_eq!(none.runs[0].invocations, [] as [_; 0]);

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

    #[test]
    fn attribution_resolves_unit_id_to_slice_iri_and_never_leaks_numeric_id() {
        use purrdf_core::{Attribution, AttributionRole};

        // Mint a UnitId for a public slice IRI via the interner (the sole minter).
        const SLICE_IRI: &str = "http://example.org/slice/1";
        let mut interner = UnitInterner::new();
        let unit = interner.intern(SLICE_IRI);

        // A result carrying a structured attribution referencing that unit.
        let mut vr = result(
            "http://www.w3.org/ns/shacl#DatatypeConstraintComponent",
            Severity::Violation,
            Some("bad value"),
        );
        vr.attributions = vec![Attribution {
            unit,
            role: AttributionRole::ShapeOwner,
            evidence: None,
        }];
        let report = ValidationReport {
            conforms: false,
            results: vec![vr],
            conformance_disallows: purrdf_shapes::report::ConformanceDisallows::default(),
        };

        // The numeric id's Display form is the interner ordinal (`unit` + index) —
        // it MUST NOT appear anywhere in the emitted SARIF.
        let numeric = unit.to_string();
        assert!(
            numeric.starts_with("unit") && numeric.ends_with('0'),
            "sanity: interner mints ids from the first ordinal"
        );

        let sources = SarifSources {
            units: Some(&interner),
            ..SarifSources::default()
        };
        let log = build_report_sarif_with(&report, &SarifOptions::default(), &sources);

        // The attribution must surface as a resolved logical location.
        let logical = &log.runs[0].results[0].locations[0].logical_locations;
        let attribution_loc = logical
            .iter()
            .find(|l| l.kind.as_deref() == Some(AttributionRole::ShapeOwner.as_str()))
            .expect("attribution logical location present");
        assert_eq!(
            attribution_loc.name, SLICE_IRI,
            "attribution name must resolve to the public slice IRI, not the numeric UnitId"
        );
        assert_eq!(
            attribution_loc.kind.as_deref(),
            Some("shape-owner"),
            "attribution kind must be the AttributionRole::as_str() value"
        );

        // S0.5: the numeric UnitId must appear NOWHERE in the serialized SARIF.
        let serialized = report.to_sarif_with(&SarifOptions::default(), &sources);
        assert!(
            serialized.contains(SLICE_IRI),
            "serialized SARIF must carry the resolved slice IRI"
        );
        assert!(
            !serialized.contains(&numeric),
            "numeric UnitId ({numeric}) must never leak into serialized SARIF"
        );
        assert!(
            !serialized.contains("unit#"),
            "no numeric UnitId Display form may leak into serialized SARIF"
        );
    }
}
