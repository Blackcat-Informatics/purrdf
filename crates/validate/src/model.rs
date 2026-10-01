// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The hand-rolled SARIF 2.1.0 object model PurRDF emits.
//!
//! This is a faithful SUBSET of the OASIS SARIF 2.1.0 schema — the objects a
//! validator/parser actually produces (log → run → tool/driver/rules →
//! results → locations → regions → logical/related locations) — with no
//! heavyweight SARIF dependency. Each type writes itself as a
//! [`purrdf_lex::json::Value`] (`to_json`), and [`to_json_pretty`] prints the log
//! with the workspace's one JSON writer.
//!
//! # Determinism
//!
//! Byte-deterministic output is a hard requirement (every serializer in this repo
//! is). Two properties guarantee it:
//!
//! * **Members follow field declaration order.** Each `to_json` writes its
//!   type's members in the order its fields are declared here, omitting an
//!   absent optional member or an empty optional list. So the field order you
//!   read below is the byte order emitted.
//! * **Open-ended maps are sorted.** [`PropertyBag`] is a `BTreeMap`, so property
//!   keys serialize in sorted order regardless of insertion order, and every
//!   object nested inside a property value is written with its members sorted
//!   by name ([`Value::sort_keys`]).
//!
//! There are no timestamps in the model except the optional, caller-supplied
//! [`Invocation`] times — nothing is sampled from the clock here.

use std::collections::BTreeMap;

use purrdf_lex::json::{self, Object, Value};

/// The SARIF version string this model targets.
pub const SARIF_VERSION: &str = "2.1.0";

/// The `$schema` hint emitted into every log. A URI hint only — it does not
/// affect validity; the CI schema-validation lane pins the vendored copy.
pub const SARIF_SCHEMA: &str = "https://json.schemastore.org/sarif-2.1.0.json";

/// A SARIF result/notification severity level (`error`/`warning`/`note`/`none`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// A problem that should block.
    Error,
    /// A problem that should be surfaced but need not block.
    Warning,
    /// An informational note.
    Note,
    /// Explicitly no severity.
    None,
}

/// A SARIF result `kind` (SARIF 2.1.0 §3.27.9): what the result says about the
/// artifact, as distinct from how severe it is.
///
/// Absent means `fail` ("If kind is absent, it SHALL default to 'fail'"). A
/// SHACL `sh:Debug` or `sh:Trace` result — "a debug message that is not a
/// constraint violation", "a trace message that is not a constraint violation" —
/// is [`ResultKind::Informational`] ("The tool is reporting an item of
/// information that does not imply a problem"), and SARIF then requires its
/// `level` to be `none`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultKind {
    /// The rule was evaluated and the result is not applicable.
    NotApplicable,
    /// The rule was evaluated and no problem was found.
    Pass,
    /// The rule was evaluated and a problem was found.
    Fail,
    /// The result requires human review.
    Review,
    /// The tool could not determine the result.
    Open,
    /// An item of information that does not imply a problem.
    Informational,
}

/// The top-level SARIF log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SarifLog {
    /// The schema hint (`$schema`).
    pub schema: &'static str,
    /// The SARIF version (`"2.1.0"`).
    pub version: &'static str,
    /// One or more analysis runs.
    pub runs: Vec<Run>,
}

impl SarifLog {
    /// A single-run log with the schema/version constants filled in.
    #[must_use]
    pub fn single_run(run: Run) -> Self {
        Self {
            schema: SARIF_SCHEMA,
            version: SARIF_VERSION,
            runs: vec![run],
        }
    }
}

/// One analysis run: the tool plus its results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    /// The analysis tool that produced this run.
    pub tool: Tool,
    /// The results (violations/warnings/notes) for this run — ALWAYS serialized, an
    /// empty array included.
    ///
    /// SARIF 2.1.0 §3.14.23: "In all other circumstances, results SHALL be present and
    /// SHALL contain all results detected by the tool. If the tool did not detect any
    /// results, results SHALL be an empty array. If results is absent, it SHALL default
    /// to null." Only a tool that failed to start, or started but failed to begin its
    /// analysis, may leave it absent or `null`; every run this model is built for
    /// completed, so a run that found nothing says so with `[]` rather than claiming, by
    /// omission, that it could not compute results.
    pub results: Vec<SarifResult>,
    /// Optional invocation records (only when the caller supplied timing).
    pub invocations: Vec<Invocation>,
    /// Symbolic base-URI definitions (`uriBaseId` → base [`ArtifactLocation`])
    /// that artifact locations in this run are resolved against. Backed by a
    /// `BTreeMap` so keys serialize in sorted order (determinism), and skipped
    /// entirely when empty (the default, no-base-URI behavior).
    pub original_uri_base_ids: BTreeMap<String, ArtifactLocation>,
    /// A sorted-key property bag for run-level facts outside the core schema — for
    /// a SHACL report log, `shaclConforms` and `shaclConformanceDisallows`.
    pub properties: PropertyBag,
}

/// The analysis tool wrapper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tool {
    /// The tool's primary driver component.
    pub driver: Driver,
}

/// The tool driver: name, version, and rule metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Driver {
    /// The tool name (`"purrdf"`).
    pub name: String,
    /// The tool version, if the caller supplied one.
    pub version: Option<String>,
    /// A URI with more information about the tool.
    pub information_uri: Option<String>,
    /// The rule metadata referenced by `result.ruleId` / `result.ruleIndex`.
    pub rules: Vec<ReportingDescriptor>,
    /// The notification metadata a [`Notification::descriptor`] refers to (SARIF 2.1.0
    /// §3.19.24): for a SHACL report log, one descriptor per mandatory-diagnostic rule a
    /// notification states.
    pub notifications: Vec<ReportingDescriptor>,
}

/// Metadata for one rule (`reportingDescriptor`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportingDescriptor {
    /// The stable rule id referenced by results.
    pub id: String,
    /// A human-friendly rule name.
    pub name: Option<String>,
    /// A terse description.
    pub short_description: Option<Message>,
    /// A full description.
    pub full_description: Option<Message>,
    /// Actionable help text.
    pub help: Option<Message>,
    /// A URI to external documentation (e.g. a W3C SHACL spec anchor).
    pub help_uri: Option<String>,
    /// The default reporting configuration (severity level).
    pub default_configuration: Option<ReportingConfiguration>,
}

/// A rule's default configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportingConfiguration {
    /// The default severity level for the rule.
    pub level: Level,
}

/// A caller-supplied invocation record. Times are emitted VERBATIM; nothing is
/// sampled from the clock inside this crate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    /// Whether the tool run completed successfully.
    pub execution_successful: bool,
    /// Caller-supplied start time (ISO-8601 UTC).
    pub start_time_utc: Option<String>,
    /// Caller-supplied end time (ISO-8601 UTC).
    pub end_time_utc: Option<String>,
    /// Conditions the run detected that are not results (SARIF 2.1.0 §3.20.21): for a
    /// SHACL report log, the shapes graph's mandatory diagnostics, each at level `note`
    /// ("The notification is purely informational"), so the run did not fail.
    pub tool_execution_notifications: Vec<Notification>,
}

/// A SARIF notification (§3.58): a condition met during the run that is not a result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notification {
    /// The descriptor in `driver.notifications` that identifies this notification
    /// (§3.58.2: "SHOULD contain a property named descriptor").
    pub descriptor: ReportingDescriptorReference,
    /// The notification's severity level.
    pub level: Level,
    /// What was encountered (§3.58.5: "SHALL contain a property named message").
    pub message: Message,
    /// The locations the condition is relevant to.
    pub locations: Vec<Location>,
}

/// A reference to a `reportingDescriptor` (§3.52), by id and by index into the array
/// that holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportingDescriptorReference {
    /// The descriptor's id.
    pub id: String,
    /// The descriptor's index in its array.
    pub index: usize,
}

/// A single SARIF result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SarifResult {
    /// The rule this result is an instance of.
    pub rule_id: String,
    /// The index of `rule_id` in `driver.rules`, if the rule is registered.
    pub rule_index: Option<usize>,
    /// What the result says about the artifact; absent means `fail`.
    pub kind: Option<ResultKind>,
    /// The severity level.
    pub level: Level,
    /// The result message.
    pub message: Message,
    /// Primary location(s) — the focus of the result.
    pub locations: Vec<Location>,
    /// Secondary locations (e.g. "shape defined here").
    pub related_locations: Vec<Location>,
    /// A sorted-key property bag for anything outside the core schema.
    pub properties: PropertyBag,
}

/// A SARIF message.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Message {
    /// The message text.
    pub text: String,
    /// An optional message id into the rule's message strings.
    pub id: Option<String>,
    /// Optional message arguments.
    pub arguments: Vec<String>,
}

impl Message {
    /// A plain-text message.
    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..Self::default()
        }
    }
}

/// A SARIF location: a physical span and/or logical location(s).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Location {
    /// The physical (file/region) location, when a source span is known.
    pub physical_location: Option<PhysicalLocation>,
    /// The logical location(s) (focus node, shape, component, attribution …).
    pub logical_locations: Vec<LogicalLocation>,
    /// An optional message for this location (e.g. a related-location note).
    pub message: Option<Message>,
}

/// A physical location: an artifact plus a region within it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysicalLocation {
    /// The artifact (file) this location refers to.
    pub artifact_location: ArtifactLocation,
    /// The region within the artifact.
    pub region: Option<Region>,
}

/// A reference to an artifact (source document).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactLocation {
    /// The artifact URI (typically a `file:`/relative path).
    pub uri: String,
    /// An optional base id the `uri` is relative to.
    pub uri_base_id: Option<String>,
}

/// A region within an artifact. All coordinates are 1-based; byte offsets are 0-based.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Region {
    /// 1-based start line.
    pub start_line: Option<u64>,
    /// 1-based start column.
    pub start_column: Option<u32>,
    /// 1-based end column.
    pub end_column: Option<u32>,
    /// 0-based byte offset of the region start.
    pub byte_offset: Option<u64>,
    /// Byte length of the region.
    pub byte_length: Option<u64>,
}

/// A logical location (a program element identified by name/kind rather than a
/// source span) — for PurRDF: the focus node, result path, source shape,
/// constraint component, GTS index, or slice attribution role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalLocation {
    /// The location's name (e.g. the focus IRI, or an attribution slice IRI).
    pub name: String,
    /// A fully-qualified name (e.g. a result path rendered as a SPARQL path).
    pub fully_qualified_name: Option<String>,
    /// The kind of logical location (e.g. `"focusNode"`, `"sourceShape"`,
    /// `"constraintComponent"`, or an attribution role id).
    pub kind: Option<String>,
}

/// A SARIF property bag: a sorted-key string→JSON map for out-of-schema data.
///
/// Backed by a `BTreeMap` so keys always serialize in sorted order — a
/// determinism guarantee for the byte goldens. [`PropertyBag::to_json`] also
/// sorts the members of every object nested in a value.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PropertyBag(pub BTreeMap<String, Value>);

impl PropertyBag {
    /// An empty property bag.
    #[must_use]
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    /// Whether the bag has no properties (used to skip serialization).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Insert (or overwrite) a property.
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<Value>) {
        self.0.insert(key.into(), value.into());
    }

    /// The bag as a JSON object: keys in sorted order, and every object nested
    /// in a value with its members sorted by name, so the bytes do not depend
    /// on the order a value was built in.
    #[must_use]
    pub fn to_json(&self) -> Value {
        Value::object(self.0.iter().map(|(key, value)| {
            let mut value = value.clone();
            value.sort_keys();
            (key.as_str(), value)
        }))
    }
}

/// Serialize a SARIF log to pretty-printed JSON with a trailing newline.
///
/// Deterministic: same log → same bytes (see the [module docs](self)).
#[must_use]
pub fn to_json_pretty(log: &SarifLog) -> String {
    let mut out = json::write_pretty(&log.to_json());
    out.push('\n');
    out
}

/// Append `name: text` when `text` is present.
fn push_str(object: &mut Object, name: &str, text: Option<&String>) {
    if let Some(text) = text {
        object.push(name, text.as_str());
    }
}

/// Append `name: n` when `n` is present.
fn push_count<N: Into<Value> + Copy>(object: &mut Object, name: &str, n: Option<N>) {
    if let Some(n) = n {
        object.push(name, n);
    }
}

/// Append `name: [..]` unless `items` is empty.
fn push_list<T>(object: &mut Object, name: &str, items: &[T], item: impl Fn(&T) -> Value) {
    if !items.is_empty() {
        object.push(name, Value::Array(items.iter().map(item).collect()));
    }
}

/// Append the property bag as `properties` unless it is empty.
fn push_properties(object: &mut Object, properties: &PropertyBag) {
    if !properties.is_empty() {
        object.push("properties", properties.to_json());
    }
}

impl Level {
    /// The SARIF spelling: `error`, `warning`, `note` or `none`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Note => "note",
            Self::None => "none",
        }
    }
}

impl ResultKind {
    /// The SARIF spelling (§3.27.9): `notApplicable`, `pass`, `fail`, `review`,
    /// `open` or `informational`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotApplicable => "notApplicable",
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::Review => "review",
            Self::Open => "open",
            Self::Informational => "informational",
        }
    }
}

impl SarifLog {
    /// The log as a JSON value: `$schema`, `version`, `runs`.
    #[must_use]
    pub fn to_json(&self) -> Value {
        Object::new()
            .with("$schema", self.schema)
            .with("version", self.version)
            .with("runs", Value::array(self.runs.iter().map(Run::to_json)))
            .into()
    }
}

impl Run {
    /// The run as a JSON value; `results` is always present (§3.14.23).
    #[must_use]
    pub fn to_json(&self) -> Value {
        let mut object = Object::new().with("tool", self.tool.to_json()).with(
            "results",
            Value::array(self.results.iter().map(SarifResult::to_json)),
        );
        push_list(
            &mut object,
            "invocations",
            &self.invocations,
            Invocation::to_json,
        );
        if !self.original_uri_base_ids.is_empty() {
            object.push(
                "originalUriBaseIds",
                Value::object(
                    self.original_uri_base_ids
                        .iter()
                        .map(|(id, location)| (id.as_str(), location.to_json())),
                ),
            );
        }
        push_properties(&mut object, &self.properties);
        object.into()
    }
}

impl Tool {
    /// The tool as a JSON value.
    #[must_use]
    pub fn to_json(&self) -> Value {
        Object::new().with("driver", self.driver.to_json()).into()
    }
}

impl Driver {
    /// The driver as a JSON value.
    #[must_use]
    pub fn to_json(&self) -> Value {
        let mut object = Object::new().with("name", self.name.as_str());
        push_str(&mut object, "version", self.version.as_ref());
        push_str(&mut object, "informationUri", self.information_uri.as_ref());
        push_list(
            &mut object,
            "rules",
            &self.rules,
            ReportingDescriptor::to_json,
        );
        push_list(
            &mut object,
            "notifications",
            &self.notifications,
            ReportingDescriptor::to_json,
        );
        object.into()
    }
}

impl ReportingDescriptor {
    /// The descriptor as a JSON value.
    #[must_use]
    pub fn to_json(&self) -> Value {
        let mut object = Object::new().with("id", self.id.as_str());
        push_str(&mut object, "name", self.name.as_ref());
        if let Some(message) = &self.short_description {
            object.push("shortDescription", message.to_json());
        }
        if let Some(message) = &self.full_description {
            object.push("fullDescription", message.to_json());
        }
        if let Some(message) = &self.help {
            object.push("help", message.to_json());
        }
        push_str(&mut object, "helpUri", self.help_uri.as_ref());
        if let Some(configuration) = &self.default_configuration {
            object.push("defaultConfiguration", configuration.to_json());
        }
        object.into()
    }
}

impl ReportingConfiguration {
    /// The configuration as a JSON value.
    #[must_use]
    pub fn to_json(&self) -> Value {
        Object::new().with("level", self.level.as_str()).into()
    }
}

impl Invocation {
    /// The invocation as a JSON value; its times verbatim.
    #[must_use]
    pub fn to_json(&self) -> Value {
        let mut object = Object::new().with("executionSuccessful", self.execution_successful);
        push_str(&mut object, "startTimeUtc", self.start_time_utc.as_ref());
        push_str(&mut object, "endTimeUtc", self.end_time_utc.as_ref());
        push_list(
            &mut object,
            "toolExecutionNotifications",
            &self.tool_execution_notifications,
            Notification::to_json,
        );
        object.into()
    }
}

impl Notification {
    /// The notification as a JSON value.
    #[must_use]
    pub fn to_json(&self) -> Value {
        let mut object = Object::new()
            .with("descriptor", self.descriptor.to_json())
            .with("level", self.level.as_str())
            .with("message", self.message.to_json());
        push_list(&mut object, "locations", &self.locations, Location::to_json);
        object.into()
    }
}

impl ReportingDescriptorReference {
    /// The reference as a JSON value.
    #[must_use]
    pub fn to_json(&self) -> Value {
        Object::new()
            .with("id", self.id.as_str())
            .with("index", self.index)
            .into()
    }
}

impl SarifResult {
    /// The result as a JSON value.
    #[must_use]
    pub fn to_json(&self) -> Value {
        let mut object = Object::new().with("ruleId", self.rule_id.as_str());
        push_count(&mut object, "ruleIndex", self.rule_index);
        if let Some(kind) = self.kind {
            object.push("kind", kind.as_str());
        }
        object.push("level", self.level.as_str());
        object.push("message", self.message.to_json());
        push_list(&mut object, "locations", &self.locations, Location::to_json);
        push_list(
            &mut object,
            "relatedLocations",
            &self.related_locations,
            Location::to_json,
        );
        push_properties(&mut object, &self.properties);
        object.into()
    }
}

impl Message {
    /// The message as a JSON value.
    #[must_use]
    pub fn to_json(&self) -> Value {
        let mut object = Object::new().with("text", self.text.as_str());
        push_str(&mut object, "id", self.id.as_ref());
        push_list(&mut object, "arguments", &self.arguments, |argument| {
            Value::from(argument.as_str())
        });
        object.into()
    }
}

impl Location {
    /// The location as a JSON value.
    #[must_use]
    pub fn to_json(&self) -> Value {
        let mut object = Object::new();
        if let Some(physical) = &self.physical_location {
            object.push("physicalLocation", physical.to_json());
        }
        push_list(
            &mut object,
            "logicalLocations",
            &self.logical_locations,
            LogicalLocation::to_json,
        );
        if let Some(message) = &self.message {
            object.push("message", message.to_json());
        }
        object.into()
    }
}

impl PhysicalLocation {
    /// The physical location as a JSON value.
    #[must_use]
    pub fn to_json(&self) -> Value {
        let mut object = Object::new().with("artifactLocation", self.artifact_location.to_json());
        if let Some(region) = &self.region {
            object.push("region", region.to_json());
        }
        object.into()
    }
}

impl ArtifactLocation {
    /// The artifact location as a JSON value.
    #[must_use]
    pub fn to_json(&self) -> Value {
        let mut object = Object::new().with("uri", self.uri.as_str());
        push_str(&mut object, "uriBaseId", self.uri_base_id.as_ref());
        object.into()
    }
}

impl Region {
    /// The region as a JSON value.
    #[must_use]
    pub fn to_json(&self) -> Value {
        let mut object = Object::new();
        push_count(&mut object, "startLine", self.start_line);
        push_count(&mut object, "startColumn", self.start_column);
        push_count(&mut object, "endColumn", self.end_column);
        push_count(&mut object, "byteOffset", self.byte_offset);
        push_count(&mut object, "byteLength", self.byte_length);
        object.into()
    }
}

impl LogicalLocation {
    /// The logical location as a JSON value.
    #[must_use]
    pub fn to_json(&self) -> Value {
        let mut object = Object::new().with("name", self.name.as_str());
        push_str(
            &mut object,
            "fullyQualifiedName",
            self.fully_qualified_name.as_ref(),
        );
        push_str(&mut object, "kind", self.kind.as_ref());
        object.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exemplar() -> SarifLog {
        let mut properties = PropertyBag::new();
        properties.insert("shaclSeverity", "http://example.org/CustomSeverity");
        properties.insert("focusNode", "http://example.org/alice");

        SarifLog::single_run(Run {
            tool: Tool {
                driver: Driver {
                    name: "purrdf".to_owned(),
                    version: None,
                    information_uri: None,
                    rules: vec![ReportingDescriptor {
                        id: "sh:DatatypeConstraintComponent".to_owned(),
                        name: None,
                        short_description: Some(Message::text("Datatype constraint")),
                        full_description: None,
                        help: None,
                        help_uri: Some(
                            "https://www.w3.org/TR/shacl/#DatatypeConstraintComponent".to_owned(),
                        ),
                        default_configuration: Some(ReportingConfiguration {
                            level: Level::Error,
                        }),
                    }],
                    notifications: vec![],
                },
            },
            results: vec![SarifResult {
                rule_id: "sh:DatatypeConstraintComponent".to_owned(),
                rule_index: Some(0),
                kind: None,
                level: Level::Error,
                message: Message::text("Value \"foo\" fails sh:datatype xsd:integer"),
                locations: vec![Location {
                    physical_location: Some(PhysicalLocation {
                        artifact_location: ArtifactLocation {
                            uri: "data.ttl".to_owned(),
                            uri_base_id: None,
                        },
                        region: Some(Region {
                            start_line: Some(14),
                            start_column: Some(3),
                            ..Region::default()
                        }),
                    }),
                    logical_locations: vec![LogicalLocation {
                        name: "http://example.org/alice".to_owned(),
                        fully_qualified_name: None,
                        kind: Some("focusNode".to_owned()),
                    }],
                    message: None,
                }],
                related_locations: vec![],
                properties,
            }],
            invocations: vec![],
            original_uri_base_ids: BTreeMap::new(),
            properties: PropertyBag::new(),
        })
    }

    #[test]
    fn serialization_is_byte_deterministic() {
        let log = exemplar();
        let a = to_json_pretty(&log);
        let b = to_json_pretty(&log);
        assert_eq!(a, b, "the same SARIF log must serialize to identical bytes");
    }

    #[test]
    fn property_bag_keys_are_sorted() {
        // Inserted in reverse order; must serialize sorted (focusNode < shaclSeverity).
        let json = to_json_pretty(&exemplar());
        let focus = json.find("\"focusNode\"").expect("focusNode present");
        let sev = json
            .find("\"shaclSeverity\"")
            .expect("shaclSeverity present");
        assert!(
            focus < sev,
            "property bag keys must serialize in sorted order"
        );
    }

    #[test]
    fn property_bag_sorts_members_of_nested_objects() {
        let mut properties = PropertyBag::new();
        properties.insert(
            "entries",
            Value::array([Object::new().with("text", "t").with("language", "en")]),
        );
        assert_eq!(
            json::write_compact(&properties.to_json()),
            r#"{"entries":[{"language":"en","text":"t"}]}"#
        );
    }

    #[test]
    fn matches_golden() {
        // Inline byte golden: the exemplar's exact SARIF JSON. If the model's
        // field order or naming changes, this catches it.
        let expected = r#"{
  "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
  "version": "2.1.0",
  "runs": [
    {
      "tool": {
        "driver": {
          "name": "purrdf",
          "rules": [
            {
              "id": "sh:DatatypeConstraintComponent",
              "shortDescription": {
                "text": "Datatype constraint"
              },
              "helpUri": "https://www.w3.org/TR/shacl/#DatatypeConstraintComponent",
              "defaultConfiguration": {
                "level": "error"
              }
            }
          ]
        }
      },
      "results": [
        {
          "ruleId": "sh:DatatypeConstraintComponent",
          "ruleIndex": 0,
          "level": "error",
          "message": {
            "text": "Value \"foo\" fails sh:datatype xsd:integer"
          },
          "locations": [
            {
              "physicalLocation": {
                "artifactLocation": {
                  "uri": "data.ttl"
                },
                "region": {
                  "startLine": 14,
                  "startColumn": 3
                }
              },
              "logicalLocations": [
                {
                  "name": "http://example.org/alice",
                  "kind": "focusNode"
                }
              ]
            }
          ],
          "properties": {
            "focusNode": "http://example.org/alice",
            "shaclSeverity": "http://example.org/CustomSeverity"
          }
        }
      ]
    }
  ]
}
"#;
        assert_eq!(to_json_pretty(&exemplar()), expected);
    }
}
