// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Canonical LinkML 1.11 document boundary and schema-emitter types.
//!
//! The reader accepts the fixed PurRDF LinkML 1.11 dialect into a
//! JSON-compatible value tree. It rejects YAML-only semantics that cannot
//! survive a language-neutral round trip: duplicate keys, tags, non-string
//! mapping keys, and non-finite numbers. The writer emits one sorted,
//! byte-stable YAML representation while preserving fields the emitter does
//! not author.

use purrdf_iri::json_pointer;
use purrdf_iri::terminals::{is_ncname_char, is_ncname_start};
use std::collections::{BTreeMap, BTreeSet};

use crate::json_model::{Object, ToJson, Value, ValueKind};
use ::purrdf::loss::LossLedger;
use purrdf_iri::terminals;

use crate::json_schema::CompiledSchema;
use crate::schema_import::{ImportedShapes, SchemaImportConfig};

mod importer;
mod projection;

/// The exact LinkML metamodel version carried by this codec.
pub const LINKML_METAMODEL_VERSION: &str = "1.11.0";

const LINKML_PREFIX: &str = "linkml";
const MAX_LINKML_YAML_BYTES: usize = 16 * 1024 * 1024;
const MAX_LINKML_YAML_DEPTH: usize = 256;
const MAX_LINKML_YAML_NODES: usize = 1_000_000;
const MAX_LINKML_STRING_BYTES: usize = 16 * 1024 * 1024;
pub(super) const MAX_LINKML_SOURCE_KEY_BYTES: usize = 1024 * 1024;

const RESERVED_JSONLD_SLOTS: &[&str] = &[
    "@annotation",
    "@direction",
    "@id",
    "@language",
    "@list",
    "@type",
    "@value",
];

/// Policy for a JSON property that cannot be used directly as a LinkML slot name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SanitizePolicy {
    /// Emit a deterministic NCName-safe slot and report the mapping.
    Rename,
    /// Omit only the unsafe slot and return a located diagnostic and loss.
    Skip,
    /// Reject the projection with a contextual error.
    Fail,
}

/// Semantic effect of a LinkML slot-name policy decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LinkmlSlotDisposition {
    /// Only the LinkML attribute spelling changes; RDF identity is preserved.
    IdentityPreserved,
    /// Caller configuration assigns an unresolved source token a new RDF identity.
    IdentityRehomed,
    /// The selected policy omits the source property.
    Skipped,
}

/// Deterministic reason contributing to a slot rename or diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LinkmlSlotReason {
    /// The source local part contains a character outside LinkML's NCName grammar.
    InvalidCharacter,
    /// The source local part does not begin with an NCName start character.
    InvalidInitialCharacter,
    /// No caller prefix namespace can provide a directly usable source name.
    UnmatchedNamespace,
    /// An exact caller hint requests assignment under the default prefix.
    CallerRehome,
    /// A non-IRI source name requires caller-default RDF identity.
    BareName,
    /// The direct candidate exceeds the fixed generated-name byte limit.
    LengthBound,
    /// Another source slot already owns the direct candidate.
    Collision,
}

/// One deterministic source-slot to emitted-LinkML rename record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkmlSlotRename {
    /// Source `$defs` key, or the inline-class JSON Pointer.
    pub source_class: String,
    /// Emitted LinkML class name.
    pub emitted_class: String,
    /// Exact source property JSON Pointer.
    pub source_path: String,
    /// Exact source JSON property spelling.
    pub source_name: String,
    /// Resolvable source URI/CURIE before emission, when one exists.
    pub old_slot_uri: Option<String>,
    /// NCName-safe emitted LinkML attribute name.
    pub new_slot_name: String,
    /// Concrete URI/CURIE carried by the emitted `slot_uri`.
    pub emitted_slot_uri: String,
    /// Whether RDF identity was preserved or explicitly re-homed.
    pub disposition: LinkmlSlotDisposition,
    /// Ordered causes for the generated spelling.
    pub reasons: Vec<LinkmlSlotReason>,
}

impl LinkmlSlotRename {
    /// Required `(class, old_slot_uri, new_slot_name)` audit view.
    #[must_use]
    pub fn audit_tuple(&self) -> (&str, Option<&str>, &str) {
        (
            &self.source_class,
            self.old_slot_uri.as_deref(),
            &self.new_slot_name,
        )
    }
}

/// One located slot omitted by [`SanitizePolicy::Skip`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkmlSlotDiagnostic {
    /// Source `$defs` key, or the inline-class JSON Pointer.
    pub source_class: String,
    /// Emitted LinkML class name.
    pub emitted_class: String,
    /// Exact source property JSON Pointer.
    pub source_path: String,
    /// Exact source JSON property spelling.
    pub source_name: String,
    /// Resolvable source URI/CURIE before omission, when one exists.
    pub old_slot_uri: Option<String>,
    /// No emitted name exists for a skipped slot.
    pub new_slot_name: Option<String>,
    /// No emitted URI exists for a skipped slot.
    pub emitted_slot_uri: Option<String>,
    /// Always [`LinkmlSlotDisposition::Skipped`].
    pub disposition: LinkmlSlotDisposition,
    /// Ordered causes that made the source name unsafe.
    pub reasons: Vec<LinkmlSlotReason>,
    /// Stable human-readable policy result.
    pub detail: String,
}

impl SanitizePolicy {
    /// The policy's kebab-case name (`rename`, `skip`, `fail`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Rename => "rename",
            Self::Skip => "skip",
            Self::Fail => "fail",
        }
    }
}

impl LinkmlSlotDisposition {
    /// The disposition's kebab-case name (`identity-preserved`, ...).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::IdentityPreserved => "identity-preserved",
            Self::IdentityRehomed => "identity-rehomed",
            Self::Skipped => "skipped",
        }
    }
}

impl LinkmlSlotReason {
    /// The reason's kebab-case name (`invalid-character`, ...).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidCharacter => "invalid-character",
            Self::InvalidInitialCharacter => "invalid-initial-character",
            Self::UnmatchedNamespace => "unmatched-namespace",
            Self::CallerRehome => "caller-rehome",
            Self::BareName => "bare-name",
            Self::LengthBound => "length-bound",
            Self::Collision => "collision",
        }
    }
}

impl ToJson for SanitizePolicy {
    fn to_json(&self) -> Value {
        Value::from(self.as_str())
    }
}

impl ToJson for LinkmlSlotDisposition {
    fn to_json(&self) -> Value {
        Value::from(self.as_str())
    }
}

impl ToJson for LinkmlSlotReason {
    fn to_json(&self) -> Value {
        Value::from(self.as_str())
    }
}

impl LinkmlSlotRename {
    /// The record as JSON: its fields as members, in declaration order, the
    /// enumerations by their kebab-case names and an absent URI as `null`.
    #[must_use]
    pub fn to_json(&self) -> Value {
        Value::from(
            Object::new()
                .with("source_class", &self.source_class)
                .with("emitted_class", &self.emitted_class)
                .with("source_path", &self.source_path)
                .with("source_name", &self.source_name)
                .with("old_slot_uri", self.old_slot_uri.as_deref())
                .with("new_slot_name", &self.new_slot_name)
                .with("emitted_slot_uri", &self.emitted_slot_uri)
                .with("disposition", self.disposition.to_json())
                .with("reasons", self.reasons.to_json()),
        )
    }
}

impl LinkmlSlotDiagnostic {
    /// The diagnostic as JSON: its fields as members, in declaration order,
    /// the enumerations by their kebab-case names and an absent value as
    /// `null`.
    #[must_use]
    pub fn to_json(&self) -> Value {
        Value::from(
            Object::new()
                .with("source_class", &self.source_class)
                .with("emitted_class", &self.emitted_class)
                .with("source_path", &self.source_path)
                .with("source_name", &self.source_name)
                .with("old_slot_uri", self.old_slot_uri.as_deref())
                .with("new_slot_name", self.new_slot_name.as_deref())
                .with("emitted_slot_uri", self.emitted_slot_uri.as_deref())
                .with("disposition", self.disposition.to_json())
                .with("reasons", self.reasons.to_json())
                .with("detail", &self.detail),
        )
    }
}

/// Caller-owned identity and vocabulary configuration for LinkML emission.
///
/// There is intentionally no Default implementation. The caller must supply
/// the schema IRI, schema name, prose, default vocabulary prefix, and every
/// prefix mapping. The reserved linkml prefix must also be supplied explicitly
/// because the emitted schema imports linkml:types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkmlConfig {
    schema_id: String,
    schema_name: String,
    description: String,
    default_prefix: String,
    prefixes: BTreeMap<String, String>,
    /// `prefixes` as the compaction table of [`purrdf_iri::contract`].
    curies: purrdf_iri::PrefixMap,
    sanitize_policy: SanitizePolicy,
    slot_rehomes: BTreeSet<String>,
}

impl LinkmlConfig {
    /// Validate and construct LinkML emitter configuration.
    ///
    /// # Errors
    ///
    /// Returns LinkmlError when an identity or prefix is missing, malformed,
    /// relative, or conflicts with the reserved linkml prefix.
    pub fn new(
        schema_id: impl Into<String>,
        schema_name: impl Into<String>,
        description: impl Into<String>,
        default_prefix: impl Into<String>,
        prefixes: BTreeMap<String, String>,
    ) -> Result<Self, LinkmlError> {
        let schema_id = schema_id.into();
        let schema_name = schema_name.into();
        let description = description.into();
        let default_prefix = default_prefix.into();

        validate_absolute_iri("LinkML schema id", &schema_id)?;
        validate_identifier("LinkML schema name", &schema_name)?;
        if description.trim().is_empty() {
            return Err(LinkmlError::new(
                "LinkML schema description must be caller-supplied non-whitespace text",
            ));
        }
        validate_identifier("LinkML default prefix", &default_prefix)?;
        validate_prefixes(&prefixes)?;
        if !prefixes.contains_key(&default_prefix) {
            return Err(LinkmlError::new(format!(
                "LinkML default prefix {default_prefix:?} is absent from the caller prefix map"
            )));
        }
        if default_prefix == LINKML_PREFIX {
            return Err(LinkmlError::new(
                "LinkML default prefix cannot reuse the reserved linkml metamodel prefix",
            ));
        }
        if !prefixes.contains_key(LINKML_PREFIX) {
            return Err(LinkmlError::new(
                "LinkML prefix map must caller-supply the reserved linkml namespace",
            ));
        }

        Ok(Self {
            schema_id,
            schema_name,
            description,
            default_prefix,
            curies: prefixes.iter().collect(),
            prefixes,
            sanitize_policy: SanitizePolicy::Rename,
            slot_rehomes: BTreeSet::new(),
        })
    }

    /// Caller-supplied absolute schema IRI.
    #[must_use]
    pub fn schema_id(&self) -> &str {
        &self.schema_id
    }

    /// Caller-supplied LinkML schema name.
    #[must_use]
    pub fn schema_name(&self) -> &str {
        &self.schema_name
    }

    /// Caller-supplied schema description.
    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Prefix used to derive unqualified element URIs.
    #[must_use]
    pub fn default_prefix(&self) -> &str {
        &self.default_prefix
    }

    /// Ordered caller-supplied prefix map.
    #[must_use]
    pub fn prefixes(&self) -> &BTreeMap<String, String> {
        &self.prefixes
    }

    /// The prefix map as the compaction table of [`purrdf_iri::contract`].
    pub(crate) const fn curies(&self) -> &purrdf_iri::PrefixMap {
        &self.curies
    }

    /// Policy applied to a source property without a directly usable LinkML name.
    #[must_use]
    pub const fn sanitize_policy(&self) -> SanitizePolicy {
        self.sanitize_policy
    }

    /// Select the unsafe-slot policy while retaining the caller-owned identity config.
    #[must_use]
    pub fn with_sanitize_policy(mut self, policy: SanitizePolicy) -> Self {
        self.sanitize_policy = policy;
        self
    }

    /// Assign exact unresolved source tokens to the caller's default prefix.
    ///
    /// A hint cannot target a declared CURIE or reserved JSON-LD carrier. During
    /// emission every configured token must occur at least once; stale hints
    /// fail closed instead of silently changing classification intent.
    ///
    /// # Errors
    ///
    /// Returns [`LinkmlError`] for an empty/oversized source token, a reserved
    /// JSON-LD carrier, or a CURIE whose prefix is already declared.
    pub fn with_slot_rehomes(
        mut self,
        slot_rehomes: BTreeSet<String>,
    ) -> Result<Self, LinkmlError> {
        for source in &slot_rehomes {
            if source.is_empty() {
                return Err(LinkmlError::new(
                    "LinkML slot re-home source token cannot be empty",
                ));
            }
            if source.len() > MAX_LINKML_SOURCE_KEY_BYTES {
                return Err(LinkmlError::new(format!(
                    "LinkML slot re-home source token exceeds {MAX_LINKML_SOURCE_KEY_BYTES} bytes"
                )));
            }
            if is_reserved_jsonld_slot(source) {
                return Err(LinkmlError::new(format!(
                    "LinkML slot re-home source token {source:?} is a reserved JSON-LD carrier"
                )));
            }
            if let Some((prefix, _)) = source.split_once(':')
                && self.prefixes.contains_key(prefix)
            {
                return Err(LinkmlError::new(format!(
                    "LinkML slot re-home source token {source:?} uses declared prefix {prefix:?}"
                )));
            }
        }
        self.slot_rehomes = slot_rehomes;
        Ok(self)
    }

    /// Exact source tokens assigned to the caller's default prefix.
    #[must_use]
    pub fn slot_rehomes(&self) -> &BTreeSet<String> {
        &self.slot_rehomes
    }
}

/// One validated LinkML 1.11 document, including unknown metamodel fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkmlDocument {
    value: Value,
}

impl LinkmlDocument {
    /// Validate a JSON-compatible LinkML 1.11 value tree.
    ///
    /// The document is held with every mapping's keys in name order, the
    /// order canonical YAML writes them in.
    ///
    /// # Errors
    ///
    /// Returns LinkmlError when the fixed dialect envelope is malformed, or a
    /// mapping repeats a key (YAML 1.2 §3.2.1.1: mapping keys are unique).
    pub fn from_value(mut value: Value) -> Result<Self, LinkmlError> {
        validate_document(&value)?;
        value.sort_keys();
        Ok(Self { value })
    }

    /// Borrow the complete JSON-compatible document tree.
    #[must_use]
    pub fn as_value(&self) -> &Value {
        &self.value
    }

    /// Consume this wrapper and return the complete document tree.
    #[must_use]
    pub fn into_value(self) -> Value {
        self.value
    }
}

impl TryFrom<Value> for LinkmlDocument {
    type Error = LinkmlError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        Self::from_value(value)
    }
}

/// Deterministic emitted LinkML document, bytes, element map, and losses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkmlPackage {
    /// Validated semantic document.
    pub document: LinkmlDocument,
    /// Canonical LinkML YAML with exactly one trailing newline.
    pub yaml: String,
    /// Source definition key to emitted LinkML element name, sorted by key.
    pub element_names: BTreeMap<String, String>,
    /// JSON Schema assertions not represented exactly in LinkML 1.11.
    pub losses: LossLedger,
    /// Ordered source-slot to emitted-name mappings.
    pub slot_renames: Vec<LinkmlSlotRename>,
    /// Ordered located diagnostics for slots omitted by [`SanitizePolicy::Skip`].
    pub slot_diagnostics: Vec<LinkmlSlotDiagnostic>,
    canonical_yaml: String,
    canonical_element_names: BTreeMap<String, String>,
    canonical_losses: LossLedger,
    canonical_slot_renames: Vec<LinkmlSlotRename>,
    canonical_slot_diagnostics: Vec<LinkmlSlotDiagnostic>,
}

purrdf_lex::message_error! {
    /// A malformed LinkML configuration, document, or projection input.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct LinkmlError, detail;
}

/// Parse a LinkML 1.11 YAML document without accepting lossy YAML semantics.
///
/// # Errors
///
/// Returns LinkmlError for invalid YAML, duplicate keys, tags, non-string
/// mapping keys, non-finite numbers, resource-limit violations, or an invalid
/// fixed-dialect envelope.
pub fn parse_linkml(input: &str) -> Result<LinkmlDocument, LinkmlError> {
    if input.len() > MAX_LINKML_YAML_BYTES {
        return Err(LinkmlError::new(format!(
            "LinkML YAML exceeds the {MAX_LINKML_YAML_BYTES}-byte input limit"
        )));
    }

    let value = purrdf_lex::yaml::read_with(
        input,
        purrdf_lex::yaml::Limits {
            max_depth: MAX_LINKML_YAML_DEPTH,
            max_nodes: u64::try_from(MAX_LINKML_YAML_NODES).unwrap_or(u64::MAX),
            aliases: true,
        },
    )
    .map_err(|error| LinkmlError::new(format!("invalid LinkML YAML: {error}")))?;
    LinkmlDocument::from_value(value)
}

/// Serialize one validated LinkML document to canonical sorted block YAML.
///
/// # Errors
///
/// Returns LinkmlError when the document envelope is invalid or YAML
/// serialization fails.
pub fn write_linkml(document: &LinkmlDocument) -> Result<String, LinkmlError> {
    validate_document(document.as_value())?;
    let serialized = purrdf_lex::yaml::write(document.as_value());
    let mut canonical = serialized.trim_end_matches('\n').to_owned();
    canonical.push('\n');
    if parse_linkml(&canonical)? != *document {
        return Err(LinkmlError::new(
            "LinkML document does not read back identically from its canonical YAML",
        ));
    }
    Ok(canonical)
}

/// Project one compiled SHACL-derived JSON Schema to deterministic LinkML 1.11.
///
/// Source-stage losses remain on [`CompiledSchema::losses`]. The returned
/// ledger covers only this projection step, `json-schema` → `linkml-1.11`.
/// Every emitted identity and vocabulary IRI comes from [`LinkmlConfig`].
///
/// # Errors
///
/// Returns [`LinkmlError`] when the compiled schema is malformed, a reference
/// is external or dangling, a required-property declaration is inconsistent,
/// or source names collide after deterministic LinkML normalization.
pub fn emit_linkml(
    compiled: &CompiledSchema,
    config: &LinkmlConfig,
) -> Result<LinkmlPackage, LinkmlError> {
    projection::emit(compiled, config)
}

/// Import one validated native LinkML 1.11 document as SHACL shapes.
///
/// Every accepted native construct without an exact SHACL interpretation is
/// returned in the always-computed `linkml-1.11` → `shacl` loss ledger. Element
/// and slot identities come only from the document's caller-supplied prefix
/// map; scalar RDF datatypes come only from [`SchemaImportConfig`].
///
/// # Errors
///
/// Returns [`LinkmlError`] for malformed native class/slot/type/enum
/// structures, unknown or cyclic references, identity collisions, or a shared
/// schema-import failure.
pub fn import_linkml(
    document: &LinkmlDocument,
    config: &SchemaImportConfig,
) -> Result<ImportedShapes, LinkmlError> {
    importer::import_document(document, config, None)
}

/// Import a deterministic PurRDF-emitted LinkML package as SHACL shapes after
/// verifying its YAML bytes and source-definition → element-name map.
///
/// # Errors
///
/// Returns [`LinkmlError`] when the package artifacts or reversible element
/// map drift from the validated document, or when [`import_linkml`] fails.
pub fn import_linkml_package(
    package: &LinkmlPackage,
    config: &SchemaImportConfig,
) -> Result<ImportedShapes, LinkmlError> {
    importer::import_package(package, config)
}

fn validate_document(value: &Value) -> Result<(), LinkmlError> {
    let mut nodes = 0;
    validate_json_value(value, 0, &mut nodes, "#")?;
    let root = value
        .as_object()
        .ok_or_else(|| LinkmlError::new("LinkML document root must be a mapping"))?;

    let schema_id = required_string(root, "id", "LinkML id")?;
    validate_absolute_iri("LinkML document id", schema_id)?;
    validate_identifier(
        "LinkML document name",
        required_string(root, "name", "LinkML name")?,
    )?;

    let metamodel_version = required_string(root, "metamodel_version", "LinkML metamodel_version")?;
    if metamodel_version != LINKML_METAMODEL_VERSION {
        return Err(LinkmlError::new(format!(
            "LinkML metamodel_version must be {LINKML_METAMODEL_VERSION:?}, got {metamodel_version:?}"
        )));
    }

    if let Some(description) = root.get("description") {
        let description = description
            .as_str()
            .ok_or_else(|| LinkmlError::new("LinkML description must be a string"))?;
        if description.trim().is_empty() {
            return Err(LinkmlError::new(
                "LinkML description must contain non-whitespace text when present",
            ));
        }
    }

    let prefixes = root
        .get("prefixes")
        .and_then(Value::as_object)
        .ok_or_else(|| LinkmlError::new("LinkML prefixes must be a mapping"))?;
    validate_document_prefixes(prefixes)?;

    let default_prefix = required_string(root, "default_prefix", "LinkML default_prefix")?;
    validate_identifier("LinkML document default_prefix", default_prefix)?;
    if !prefixes.contains_key(default_prefix) {
        return Err(LinkmlError::new(format!(
            "LinkML default_prefix {default_prefix:?} is absent from prefixes"
        )));
    }
    if default_prefix == LINKML_PREFIX {
        return Err(LinkmlError::new(
            "LinkML default_prefix cannot reuse the reserved linkml metamodel prefix",
        ));
    }
    if !prefixes.contains_key(LINKML_PREFIX) {
        return Err(LinkmlError::new(
            "LinkML prefixes must include the caller-supplied linkml namespace",
        ));
    }

    for section in ["classes", "enums", "slots", "types"] {
        if root.get(section).is_some_and(|value| !value.is_object()) {
            return Err(LinkmlError::new(format!(
                "LinkML {section} must be a mapping when present"
            )));
        }
    }
    if let Some(imports) = root.get("imports") {
        match imports {
            Value::String(_) => {}
            Value::Array(values) if values.iter().all(Value::is_string) => {}
            _ => {
                return Err(LinkmlError::new(
                    "LinkML imports must be a string or an array of strings",
                ));
            }
        }
    }

    Ok(())
}

fn validate_json_value(
    value: &Value,
    depth: usize,
    nodes: &mut usize,
    path: &str,
) -> Result<(), LinkmlError> {
    if depth > MAX_LINKML_YAML_DEPTH {
        return Err(LinkmlError::new(format!(
            "LinkML document at {path} exceeds depth {MAX_LINKML_YAML_DEPTH}"
        )));
    }
    *nodes = nodes
        .checked_add(1)
        .ok_or_else(|| LinkmlError::new("LinkML document node count overflow"))?;
    if *nodes > MAX_LINKML_YAML_NODES {
        return Err(LinkmlError::new(format!(
            "LinkML document exceeds {MAX_LINKML_YAML_NODES} nodes"
        )));
    }
    match value {
        Value::String(value) if value.len() > MAX_LINKML_STRING_BYTES => Err(LinkmlError::new(
            format!("LinkML string at {path} exceeds {MAX_LINKML_STRING_BYTES} bytes"),
        )),
        Value::Array(values) => {
            for (index, child) in values.iter().enumerate() {
                validate_json_value(child, depth + 1, nodes, &format!("{path}/{index}"))?;
            }
            Ok(())
        }
        Value::Object(values) => {
            if let Some(key) = values.first_duplicate() {
                return Err(LinkmlError::new(format!(
                    "LinkML mapping at {path} repeats the key {key:?}"
                )));
            }
            for (key, child) in values {
                if key.len() > MAX_LINKML_STRING_BYTES {
                    return Err(LinkmlError::new(format!(
                        "LinkML key at {path} exceeds {MAX_LINKML_STRING_BYTES} bytes"
                    )));
                }
                validate_json_value(
                    child,
                    depth + 1,
                    nodes,
                    &format!("{path}/{}", json_pointer::escape_token(key)),
                )?;
            }
            Ok(())
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => Ok(()),
    }
}

fn validate_prefixes(prefixes: &BTreeMap<String, String>) -> Result<(), LinkmlError> {
    if prefixes.is_empty() {
        return Err(LinkmlError::new("LinkML caller prefix map cannot be empty"));
    }
    for (prefix, namespace) in prefixes {
        validate_identifier("LinkML prefix", prefix)?;
        validate_absolute_iri(&format!("LinkML prefix {prefix:?} namespace"), namespace)?;
    }
    Ok(())
}

fn validate_document_prefixes(prefixes: &Object) -> Result<(), LinkmlError> {
    if prefixes.is_empty() {
        return Err(LinkmlError::new("LinkML prefixes cannot be empty"));
    }
    for (prefix, definition) in prefixes {
        validate_identifier("LinkML prefix", prefix)?;
        match definition {
            Value::String(namespace) => {
                validate_absolute_iri(&format!("LinkML prefix {prefix:?} namespace"), namespace)?;
            }
            Value::Object(object) => {
                if let Some(declared_prefix) = object.get("prefix_prefix") {
                    let declared_prefix = declared_prefix.as_str().ok_or_else(|| {
                        LinkmlError::new(format!(
                            "LinkML prefix {prefix:?} prefix_prefix must be a string"
                        ))
                    })?;
                    if declared_prefix != prefix {
                        return Err(LinkmlError::new(format!(
                            "LinkML prefix {prefix:?} conflicts with prefix_prefix {declared_prefix:?}"
                        )));
                    }
                }
                let namespace = object
                    .get("prefix_reference")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        LinkmlError::new(format!(
                            "LinkML prefix {prefix:?} object requires string prefix_reference"
                        ))
                    })?;
                validate_absolute_iri(
                    &format!("LinkML prefix {prefix:?} prefix_reference"),
                    namespace,
                )?;
            }
            _ => {
                return Err(LinkmlError::new(format!(
                    "LinkML prefix {prefix:?} must be a namespace string or prefix object"
                )));
            }
        }
    }
    Ok(())
}

/// The string member `key` of `object`, or a refusal naming `path` — the one
/// "required string" reader of every LinkML document surface.
fn required_string<'a>(object: &'a Object, key: &str, path: &str) -> Result<&'a str, LinkmlError> {
    object
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| LinkmlError::new(format!("{path} must be a string")))
}

fn validate_absolute_iri(label: &str, value: &str) -> Result<(), LinkmlError> {
    match purrdf_iri::BaseIri::parse(value) {
        Ok(_) => Ok(()),
        Err(purrdf_iri::IriError::NonAbsoluteBase(_)) => Err(LinkmlError::new(format!(
            "{label} {value:?} must be absolute"
        ))),
        Err(error) => Err(LinkmlError::new(format!(
            "{label} {value:?} is invalid: {error}"
        ))),
    }
}

fn validate_identifier(label: &str, value: &str) -> Result<(), LinkmlError> {
    if !is_linkml_identifier(value) {
        return Err(LinkmlError::new(format!(
            "{label} {value:?} is not a LinkML NCName"
        )));
    }
    Ok(())
}

/// Whether `value` is an XML `NCName`.
///
/// `NCName ::= NCNameStartChar NCNameChar*` (*Namespaces in XML 1.0 (Third
/// Edition)* §3 `[4]`) — the production [`validate_identifier`] refuses by name,
/// spelled through [`terminals::is_ncname`].
fn is_linkml_identifier(value: &str) -> bool {
    terminals::is_ncname(value)
}

pub(super) fn is_reserved_jsonld_slot(value: &str) -> bool {
    RESERVED_JSONLD_SLOTS.binary_search(&value).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json_model::json;

    #[test]
    fn an_ncname_is_exactly_the_production_and_not_a_unicode_property() {
        // The valid neighbours FIRST, so the refusals below read as exactness and
        // not as a narrowed alphabet.
        for valid in [
            "café",        // NFC: U+00E9 lies in `[#xD8-#xF6]`
            "cafe\u{301}", // NFD: U+0301 lies in `[#x300-#x36F]`, a NameChar
            "_x",          // `'_'` is a NameStartChar
            "a-b.c",       // `'-'` and `'.'` are NameChars
            "a0",          // a digit CONTINUES a name
            "\u{200C}x",   // ZWNJ opens one: `[#x200C-#x200D]`
            "x\u{200D}",   // ZWJ continues one
            "\u{4E2D}\u{6587}",
            "x\u{B7}y", // U+00B7 MIDDLE DOT is a NameChar
        ] {
            assert!(
                is_linkml_identifier(valid),
                "{valid:?} is an NCName and must be accepted"
            );
        }

        // Over-accepted before: `Alphabetic` but below the production's first
        // non-ASCII range `[#xC0-#xD6]`, so not a NameStartChar.
        for over in ["\u{AA}", "\u{B5}", "\u{BA}"] {
            assert!(
                !is_linkml_identifier(over),
                "{over:?} is Alphabetic and is NOT an NCNameStartChar"
            );
        }

        // The colon is subtracted in BOTH positions — that is what `NCName` is.
        assert!(!is_linkml_identifier("ns:local"));
        assert!(!is_linkml_identifier(":local"));

        // Position dependence, and the empty string.
        assert!(!is_linkml_identifier("0a"));
        assert!(!is_linkml_identifier("-a"));
        assert!(!is_linkml_identifier(".a"));
        assert!(!is_linkml_identifier(""));

        // `No`/`Nl` numerals are `is_alphanumeric` and are not `NameChar`s.
        assert!(!is_linkml_identifier("a\u{B2}"));
    }

    #[test]
    fn the_zero_width_joiners_are_name_characters_in_both_positions() {
        // Under-accepted before: U+200C/U+200D are `Cf`, so neither
        // `is_alphabetic` nor `is_alphanumeric` admits them, and the production
        // names them outright at `[#x200C-#x200D]`.
        assert!(!'\u{200C}'.is_alphabetic());
        assert!(!'\u{200D}'.is_alphanumeric());
        assert!(is_ncname_start('\u{200C}'));
        assert!(is_ncname_start('\u{200D}'));
        assert!(is_ncname_char('\u{200C}'));
        assert!(is_ncname_char('\u{200D}'));
    }

    fn prefixes() -> BTreeMap<String, String> {
        BTreeMap::from([
            ("ex".to_owned(), "https://example.org/".to_owned()),
            (
                "linkml".to_owned(),
                "https://example.org/linkml/".to_owned(),
            ),
        ])
    }

    fn valid_value() -> Value {
        json!({
            "id": "https://example.org/schema",
            "name": "Example-Schema",
            "description": "Caller schema.",
            "metamodel_version": LINKML_METAMODEL_VERSION,
            "prefixes": {
                "ex": "https://example.org/",
                "linkml": "https://example.org/linkml/"
            },
            "default_prefix": "ex",
            "classes": {}
        })
    }

    #[test]
    fn config_requires_caller_identity_vocabulary_and_docs() {
        let config = LinkmlConfig::new(
            "https://example.org/schema",
            "Example-Schema",
            "Caller schema.",
            "ex",
            prefixes(),
        )
        .expect("valid configuration");
        assert_eq!(config.schema_id(), "https://example.org/schema");
        assert_eq!(config.schema_name(), "Example-Schema");
        assert_eq!(config.description(), "Caller schema.");
        assert_eq!(config.default_prefix(), "ex");
        assert_eq!(config.prefixes(), &prefixes());
        assert_eq!(config.sanitize_policy(), SanitizePolicy::Rename);
        assert!(config.slot_rehomes().is_empty());

        let configured = config
            .clone()
            .with_sanitize_policy(SanitizePolicy::Skip)
            .with_slot_rehomes(BTreeSet::from(["skos:definition".to_owned()]))
            .expect("unresolved source token is caller-owned");
        assert_eq!(configured.sanitize_policy(), SanitizePolicy::Skip);
        assert_eq!(
            configured.slot_rehomes(),
            &BTreeSet::from(["skos:definition".to_owned()])
        );
        assert!(
            config
                .clone()
                .with_slot_rehomes(BTreeSet::from(["ex:name".to_owned()]))
                .expect_err("declared CURIE cannot be re-homed")
                .detail()
                .contains("declared prefix")
        );
        assert!(
            config
                .with_slot_rehomes(BTreeSet::from(["@id".to_owned()]))
                .expect_err("reserved carrier cannot be re-homed")
                .detail()
                .contains("reserved JSON-LD")
        );

        assert!(LinkmlConfig::new("/relative", "Schema", "docs", "ex", prefixes()).is_err());
        assert!(
            LinkmlConfig::new(
                "https://example.org/schema",
                "9bad",
                "docs",
                "ex",
                prefixes()
            )
            .is_err()
        );
        assert!(
            LinkmlConfig::new(
                "https://example.org/schema",
                "Schema",
                " ",
                "ex",
                prefixes()
            )
            .is_err()
        );
        assert!(
            LinkmlConfig::new(
                "https://example.org/schema",
                "Schema",
                "docs",
                "missing",
                prefixes()
            )
            .is_err()
        );
        assert!(
            LinkmlConfig::new(
                "https://example.org/schema",
                "Schema",
                "docs",
                "linkml",
                prefixes()
            )
            .is_err()
        );

        let mut missing_linkml = prefixes();
        missing_linkml.remove("linkml");
        assert!(
            LinkmlConfig::new(
                "https://example.org/schema",
                "Schema",
                "docs",
                "ex",
                missing_linkml,
            )
            .is_err()
        );
        let bad_namespace = BTreeMap::from([
            ("ex".to_owned(), "relative".to_owned()),
            (
                "linkml".to_owned(),
                "https://example.org/linkml/".to_owned(),
            ),
        ]);
        assert!(
            LinkmlConfig::new(
                "https://example.org/schema",
                "Schema",
                "docs",
                "ex",
                bad_namespace,
            )
            .is_err()
        );
    }

    #[test]
    fn canonical_codec_preserves_unknown_fields_and_is_byte_stable() {
        let source = r"
x-extension:
  nested:
    - null
    - answer: 42
prefixes:
  linkml:
    prefix_reference: https://example.org/linkml/
    prefix_prefix: linkml
  ex: https://example.org/
name: Example-Schema
metamodel_version: 1.11.0
id: https://example.org/schema
description: Caller schema.
default_prefix: ex
classes:
  Person:
    attributes:
      ex:name:
        range: string
";
        let document = parse_linkml(source).expect("parse");
        assert_eq!(
            document.as_value()["x-extension"]["nested"][1]["answer"],
            json!(42)
        );

        let first = write_linkml(&document).expect("write");
        let reparsed = parse_linkml(&first).expect("reparse");
        let second = write_linkml(&reparsed).expect("rewrite");
        assert_eq!(reparsed, document);
        assert_eq!(second, first);
        assert!(first.ends_with('\n'));
        assert!(!first.ends_with("\n\n"));
        assert!(first.starts_with("classes:\n"));
    }

    #[test]
    fn document_value_round_trips_through_canonical_yaml() {
        let document = LinkmlDocument::from_value(valid_value()).expect("valid document");
        let yaml = write_linkml(&document).expect("write");
        let reparsed = parse_linkml(&yaml).expect("parse");
        assert_eq!(reparsed, document);
        assert_eq!(reparsed.into_value(), valid_value());
    }

    #[test]
    fn yaml_writer_preserves_marker_keys_and_number_lexemes() {
        let mut value = valid_value();
        value["x-extension"] = json!({"$serde_json::private::Number": "123", "n": 0.25});
        let document = LinkmlDocument::from_value(value.clone()).expect("document");
        assert_eq!(
            parse_linkml(&write_linkml(&document).expect("write")).expect("read"),
            document
        );
        // Numbers are written as their lexemes, so one beyond u64, one with
        // more digits than binary64 holds and one beyond the binary64 range
        // all read back exactly.
        for lexical in [
            "18446744073709551617",
            "0.123456789012345678901",
            "1e400",
            "1.50",
        ] {
            value["x-extension"]["n"] = purrdf_lex::json::read(lexical).expect("exact number");
            let document = LinkmlDocument::from_value(value.clone()).expect("document");
            let yaml = write_linkml(&document).expect(lexical);
            assert!(yaml.contains(&format!("n: {lexical}\n")), "{yaml}");
            assert_eq!(parse_linkml(&yaml).expect("read"), document, "{lexical}");
        }
    }

    #[test]
    fn programmatic_documents_refuse_a_repeated_key_and_accept_distinct_keys() {
        let mut value = valid_value();
        value["x-extension"] = Value::from(Object::new().with("a", 1).with("b", 2));
        assert!(LinkmlDocument::from_value(value.clone()).is_ok());
        let mut repeated = Object::new().with("a", 1);
        repeated.push("a", 2);
        value["x-extension"] = Value::from(repeated);
        assert!(
            LinkmlDocument::from_value(value)
                .expect_err("repeated key")
                .to_string()
                .contains("repeats the key \"a\"")
        );
    }

    #[test]
    fn parser_accepts_distinct_keys_beside_the_refused_repeat() {
        let distinct = r"
id: https://example.org/schema
name: First
title: Second
metamodel_version: 1.11.0
prefixes:
  ex: https://example.org/
  linkml: https://example.org/linkml/
default_prefix: ex
";
        let document = parse_linkml(distinct).expect("distinct keys");
        assert_eq!(document.as_value()["title"], "Second");
    }

    #[test]
    fn parser_rejects_yaml_semantics_that_are_not_json_compatible() {
        let duplicate = r"
id: https://example.org/schema
name: First
name: Second
metamodel_version: 1.11.0
prefixes:
  ex: https://example.org/
  linkml: https://example.org/linkml/
default_prefix: ex
";
        assert!(
            parse_linkml(duplicate)
                .unwrap_err()
                .to_string()
                .contains("repeats a key")
        );

        let tagged = r"
id: https://example.org/schema
name: Schema
metamodel_version: 1.11.0
prefixes:
  ex: https://example.org/
  linkml: https://example.org/linkml/
default_prefix: ex
x-value: !caller tagged
";
        assert!(
            parse_linkml(tagged)
                .unwrap_err()
                .to_string()
                .contains("a tag other than a core-schema tag")
        );

        let non_string_key = r"
id: https://example.org/schema
name: Schema
metamodel_version: 1.11.0
prefixes:
  ex: https://example.org/
  linkml: https://example.org/linkml/
default_prefix: ex
x-value:
  7: seven
";
        assert!(
            parse_linkml(non_string_key)
                .unwrap_err()
                .to_string()
                .contains("a mapping key must be a string")
        );

        let non_finite = r"
id: https://example.org/schema
name: Schema
metamodel_version: 1.11.0
prefixes:
  ex: https://example.org/
  linkml: https://example.org/linkml/
default_prefix: ex
x-value: .nan
";
        assert!(
            parse_linkml(non_finite)
                .unwrap_err()
                .to_string()
                .contains("infinite or NaN")
        );
    }

    #[test]
    fn parser_rejects_invalid_fixed_dialect_envelopes() {
        let mut value = valid_value();
        value["metamodel_version"] = json!("1.12.0");
        assert!(LinkmlDocument::from_value(value).is_err());

        let mut value = valid_value();
        value["id"] = json!("/relative");
        assert!(LinkmlDocument::from_value(value).is_err());

        let mut value = valid_value();
        value["name"] = json!("bad:name");
        assert!(LinkmlDocument::from_value(value).is_err());

        let mut value = valid_value();
        value["prefixes"].as_object_mut().unwrap().remove("linkml");
        assert!(LinkmlDocument::from_value(value).is_err());

        let mut value = valid_value();
        value["classes"] = json!([]);
        assert!(LinkmlDocument::from_value(value).is_err());

        let mut value = valid_value();
        value["imports"] = json!([7]);
        assert!(LinkmlDocument::from_value(value).is_err());
    }

    #[test]
    fn programmatic_documents_enforce_the_same_depth_limit_as_yaml() {
        let mut nested = Value::Null;
        for _ in 0..=MAX_LINKML_YAML_DEPTH {
            nested = Value::Array(vec![nested]);
        }
        let mut value = valid_value();
        value["x-too-deep"] = nested;
        assert!(
            LinkmlDocument::from_value(value)
                .expect_err("over-depth document")
                .to_string()
                .contains("exceeds depth")
        );
    }

    #[test]
    fn parser_accepts_structured_prefixes_without_erasing_extensions() {
        let mut value = valid_value();
        value["prefixes"]["ex"] = json!({
            "prefix_prefix": "ex",
            "prefix_reference": "https://example.org/",
            "x-prefix-extension": true
        });
        value["x-document-extension"] = json!({"kept": ["yes"]});
        let document = LinkmlDocument::from_value(value.clone()).expect("valid document");
        let reparsed = parse_linkml(&write_linkml(&document).unwrap()).unwrap();
        assert_eq!(reparsed.into_value(), value);
    }
}
