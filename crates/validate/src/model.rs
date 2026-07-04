// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The hand-rolled SARIF 2.1.0 object model PurRDF emits.
//!
//! This is a faithful SUBSET of the OASIS SARIF 2.1.0 schema — the objects a
//! validator/parser actually produces (log → run → tool/driver/rules →
//! results → locations → regions → logical/related locations) — with no
//! heavyweight SARIF dependency. `serde`/`serde_json` (already workspace deps)
//! carry it.
//!
//! # Determinism
//!
//! Byte-deterministic output is a hard requirement (every serializer in this repo
//! is). Two properties guarantee it:
//!
//! * **Struct field order is declaration order.** `serde` serializes derived
//!   structs field-by-field in the order written here, and `serde_json` writes
//!   them in that order — it never reorders struct fields. So the field order you
//!   read below is the byte order emitted.
//! * **Open-ended maps are sorted.** [`PropertyBag`] is a `BTreeMap`, so property
//!   keys serialize in sorted order regardless of insertion order.
//!
//! There are no timestamps in the model except the optional, caller-supplied
//! [`Invocation`] times — nothing is sampled from the clock here.

use std::collections::BTreeMap;

use serde::Serialize;

/// The SARIF version string this model targets.
pub const SARIF_VERSION: &str = "2.1.0";

/// The `$schema` hint emitted into every log. A URI hint only — it does not
/// affect validity; the CI schema-validation lane pins the vendored copy.
pub const SARIF_SCHEMA: &str = "https://json.schemastore.org/sarif-2.1.0.json";

/// A SARIF result/notification severity level (`error`/`warning`/`note`/`none`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
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

/// The top-level SARIF log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SarifLog {
    /// The schema hint (`$schema`).
    #[serde(rename = "$schema")]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Run {
    /// The analysis tool that produced this run.
    pub tool: Tool,
    /// The results (violations/warnings/notes) for this run.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub results: Vec<SarifResult>,
    /// Optional invocation records (only when the caller supplied timing).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub invocations: Vec<Invocation>,
    /// Symbolic base-URI definitions (`uriBaseId` → base [`ArtifactLocation`])
    /// that artifact locations in this run are resolved against. Backed by a
    /// `BTreeMap` so keys serialize in sorted order (determinism), and skipped
    /// entirely when empty (the default, no-base-URI behavior).
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub original_uri_base_ids: BTreeMap<String, ArtifactLocation>,
}

/// The analysis tool wrapper.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    /// The tool's primary driver component.
    pub driver: Driver,
}

/// The tool driver: name, version, and rule metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Driver {
    /// The tool name (`"purrdf"`).
    pub name: String,
    /// The tool version, if the caller supplied one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// A URI with more information about the tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub information_uri: Option<String>,
    /// The rule metadata referenced by `result.ruleId` / `result.ruleIndex`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<ReportingDescriptor>,
}

/// Metadata for one rule (`reportingDescriptor`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportingDescriptor {
    /// The stable rule id referenced by results.
    pub id: String,
    /// A human-friendly rule name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// A terse description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short_description: Option<Message>,
    /// A full description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_description: Option<Message>,
    /// Actionable help text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<Message>,
    /// A URI to external documentation (e.g. a W3C SHACL spec anchor).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help_uri: Option<String>,
    /// The default reporting configuration (severity level).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_configuration: Option<ReportingConfiguration>,
}

/// A rule's default configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportingConfiguration {
    /// The default severity level for the rule.
    pub level: Level,
}

/// A caller-supplied invocation record. Times are emitted VERBATIM; nothing is
/// sampled from the clock inside this crate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Invocation {
    /// Whether the tool run completed successfully.
    pub execution_successful: bool,
    /// Caller-supplied start time (ISO-8601 UTC).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time_utc: Option<String>,
    /// Caller-supplied end time (ISO-8601 UTC).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time_utc: Option<String>,
}

/// A single SARIF result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SarifResult {
    /// The rule this result is an instance of.
    pub rule_id: String,
    /// The index of `rule_id` in `driver.rules`, if the rule is registered.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_index: Option<usize>,
    /// The severity level.
    pub level: Level,
    /// The result message.
    pub message: Message,
    /// Primary location(s) — the focus of the result.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub locations: Vec<Location>,
    /// Secondary locations (e.g. "shape defined here").
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub related_locations: Vec<Location>,
    /// A sorted-key property bag for anything outside the core schema.
    #[serde(skip_serializing_if = "PropertyBag::is_empty")]
    pub properties: PropertyBag,
}

/// A SARIF message.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    /// The message text.
    pub text: String,
    /// An optional message id into the rule's message strings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Optional message arguments.
    #[serde(skip_serializing_if = "Vec::is_empty")]
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
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    /// The physical (file/region) location, when a source span is known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub physical_location: Option<PhysicalLocation>,
    /// The logical location(s) (focus node, shape, component, attribution …).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub logical_locations: Vec<LogicalLocation>,
    /// An optional message for this location (e.g. a related-location note).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<Message>,
}

/// A physical location: an artifact plus a region within it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PhysicalLocation {
    /// The artifact (file) this location refers to.
    pub artifact_location: ArtifactLocation,
    /// The region within the artifact.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<Region>,
}

/// A reference to an artifact (source document).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactLocation {
    /// The artifact URI (typically a `file:`/relative path).
    pub uri: String,
    /// An optional base id the `uri` is relative to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri_base_id: Option<String>,
}

/// A region within an artifact. All coordinates are 1-based; byte offsets are 0-based.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Region {
    /// 1-based start line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_line: Option<u32>,
    /// 1-based start column.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_column: Option<u32>,
    /// 1-based end column.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_column: Option<u32>,
    /// 0-based byte offset of the region start.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub byte_offset: Option<usize>,
    /// Byte length of the region.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub byte_length: Option<usize>,
}

/// A logical location (a program element identified by name/kind rather than a
/// source span) — for PurRDF: the focus node, result path, source shape,
/// constraint component, GTS index, or slice attribution role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogicalLocation {
    /// The location's name (e.g. the focus IRI, or an attribution slice IRI).
    pub name: String,
    /// A fully-qualified name (e.g. a result path rendered as a SPARQL path).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fully_qualified_name: Option<String>,
    /// The kind of logical location (e.g. `"focusNode"`, `"sourceShape"`,
    /// `"constraintComponent"`, or an attribution role id).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

/// A SARIF property bag: a sorted-key string→JSON map for out-of-schema data.
///
/// Backed by a `BTreeMap` so keys always serialize in sorted order — a
/// determinism guarantee for the byte goldens.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[serde(transparent)]
pub struct PropertyBag(pub BTreeMap<String, serde_json::Value>);

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
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) {
        self.0.insert(key.into(), value.into());
    }
}

/// Serialize a SARIF log to pretty-printed JSON with a trailing newline.
///
/// Deterministic: same log → same bytes (see the [module docs](self)).
#[must_use]
pub fn to_json_pretty(log: &SarifLog) -> String {
    let mut out = serde_json::to_string_pretty(log)
        .expect("the SARIF model is composed entirely of infallibly-serializable types");
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

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
                },
            },
            results: vec![SarifResult {
                rule_id: "sh:DatatypeConstraintComponent".to_owned(),
                rule_index: Some(0),
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
