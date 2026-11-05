// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Closed, versioned serialization options shared by Rust and foreign bindings.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use purrdf_lex::json::{self, Object, Value};

use super::{
    CompiledJsonLdContext, JSON_LD_SERIALIZE_OPTIONS_VERSION, JsonLdContextLimits,
    JsonLdContextRegistry, canonicalize, context_error, parse_strict_json,
    validate_yaml_schema_url,
};
use crate::RdfDiagnostic;

const OPTION_KEYS: &[&str] = &[
    "compact_arrays",
    "context",
    "document_iri",
    "mode",
    "prefixes",
    "registry",
    "version",
    "yaml_schema_url",
];

/// Explicit output mode for configured JSON-LD/YAML-LD serialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsonLdSerializeMode {
    /// Emit the frozen expanded compatibility representation with an empty context.
    Expanded,
    /// Compact through a reusable caller-supplied compiled context.
    Context(Arc<CompiledJsonLdContext>),
    /// Derive deterministic neutral aliases solely from IRIs present in the dataset.
    Derived,
}

/// Closed version-1 JSON-LD/YAML-LD serialization request.
///
/// Existing no-options codec functions remain expanded compatibility shims. Configured
/// entry points accept this type, whose constructors require an explicit mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonLdSerializeOptions {
    version: u32,
    mode: JsonLdSerializeMode,
    yaml_schema_url: Option<String>,
    compact_arrays: bool,
}

impl JsonLdSerializeOptions {
    /// Explicit expanded compatibility mode.
    pub const fn expanded() -> Self {
        Self {
            version: JSON_LD_SERIALIZE_OPTIONS_VERSION,
            mode: JsonLdSerializeMode::Expanded,
            yaml_schema_url: None,
            compact_arrays: true,
        }
    }

    /// Explicit deterministic dataset-derived mode.
    pub const fn derived() -> Self {
        Self {
            version: JSON_LD_SERIALIZE_OPTIONS_VERSION,
            mode: JsonLdSerializeMode::Derived,
            yaml_schema_url: None,
            compact_arrays: true,
        }
    }

    /// Explicit caller-context mode using an already compiled reusable context.
    pub fn compiled(context: Arc<CompiledJsonLdContext>) -> Self {
        Self {
            version: JSON_LD_SERIALIZE_OPTIONS_VERSION,
            mode: JsonLdSerializeMode::Context(context),
            yaml_schema_url: None,
            compact_arrays: true,
        }
    }

    /// Compile a caller-owned local context into explicit caller-context mode.
    pub fn context(context: &Value, document_iri: Option<&str>) -> Result<Self, RdfDiagnostic> {
        let compiled = CompiledJsonLdContext::compile(context, document_iri)?;
        Ok(Self::compiled(Arc::new(compiled)))
    }

    /// Compile a caller-owned context with immutable offline registry lookup.
    pub fn context_with_registry(
        context: &Value,
        document_iri: Option<&str>,
        registry: &JsonLdContextRegistry,
    ) -> Result<Self, RdfDiagnostic> {
        let compiled =
            CompiledJsonLdContext::compile_with_registry(context, document_iri, registry)?;
        Ok(Self::compiled(Arc::new(compiled)))
    }

    /// Compile deterministic caller prefix definitions into explicit context mode.
    pub fn prefixes<I, K, V>(prefixes: I) -> Result<Self, RdfDiagnostic>
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let compiled = CompiledJsonLdContext::from_prefixes(prefixes)?;
        Ok(Self::compiled(Arc::new(compiled)))
    }

    /// Strictly decode the shared versioned JSON options document.
    ///
    /// Duplicate members, unknown fields, incompatible mode fields, malformed
    /// contexts, and resource-limit excesses are hard failures with stable diagnostic
    /// codes.
    pub fn from_json(bytes: &[u8]) -> Result<Self, RdfDiagnostic> {
        let limits = JsonLdContextLimits::default();
        let value = parse_strict_json(bytes, limits.strict_options_json(), "JSON-LD options")?;
        decode_options(&value, limits)
    }

    /// Version of this options document.
    pub const fn version(&self) -> u32 {
        self.version
    }

    /// Explicit selected serialization mode.
    pub const fn mode(&self) -> &JsonLdSerializeMode {
        &self.mode
    }

    /// Optional caller-selected YAML language-server schema reference.
    pub fn yaml_schema_url(&self) -> Option<&str> {
        self.yaml_schema_url.as_deref()
    }

    /// Whether eligible singleton collections compact to their sole value.
    /// Defaults to `true`; scalar keyword positions and list-item arrays are separate.
    pub const fn compact_arrays(&self) -> bool {
        self.compact_arrays
    }

    /// Select singleton-array compaction for context or derived output.
    /// # Errors
    /// Rejects expanded compatibility mode, including an explicit `true`.
    pub fn with_compact_arrays(mut self, compact_arrays: bool) -> Result<Self, RdfDiagnostic> {
        if matches!(self.mode, JsonLdSerializeMode::Expanded) {
            return Err(context_error(
                "JSON-LD mode `expanded` does not accept compact_arrays",
            ));
        }
        self.compact_arrays = compact_arrays;
        Ok(self)
    }

    /// Attach a non-empty YAML language-server schema IRI reference.
    pub fn with_yaml_schema_url(
        mut self,
        schema_url: impl Into<String>,
    ) -> Result<Self, RdfDiagnostic> {
        let schema_url = schema_url.into();
        validate_yaml_schema_url(&schema_url)?;
        self.yaml_schema_url = Some(schema_url);
        Ok(self)
    }

    /// JSON Schema for the shared version-1 options document.
    pub fn json_schema() -> Value {
        let mut schema = json::read(&format!(
            r#"{{
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "additionalProperties": false,
            "properties": {{
                "compact_arrays": {{"type": "boolean", "default": true}},
                "context": {{}},
                "document_iri": {{"type": "string"}},
                "mode": {{"enum": ["expanded", "context", "derived"]}},
                "prefixes": {{
                    "additionalProperties": {{"type": "string"}},
                    "type": "object"
                }},
                "registry": {{
                    "additionalProperties": {{
                        "properties": {{"@context": {{}}}},
                        "required": ["@context"],
                        "type": "object"
                    }},
                    "type": "object"
                }},
                "version": {{"const": {JSON_LD_SERIALIZE_OPTIONS_VERSION}}},
                "yaml_schema_url": {{"type": "string"}}
            }},
            "allOf": [
                {{
                    "if": {{"properties": {{"mode": {{"const": "expanded"}}}}, "required": ["mode"]}},
                    "then": {{"not": {{"required": ["compact_arrays"]}}}}
                }},
                {{
                    "if": {{
                        "properties": {{"mode": {{"enum": ["expanded", "derived"]}}}},
                        "required": ["mode"]
                    }},
                    "then": {{
                        "not": {{
                            "anyOf": [
                                {{"required": ["context"]}},
                                {{"required": ["document_iri"]}},
                                {{"required": ["prefixes"]}},
                                {{"required": ["registry"]}}
                            ]
                        }}
                    }}
                }},
                {{
                    "if": {{
                        "properties": {{"mode": {{"const": "context"}}}},
                        "required": ["mode"]
                    }},
                    "then": {{
                        "oneOf": [
                            {{
                                "required": ["context"],
                                "not": {{"required": ["prefixes"]}}
                            }},
                            {{
                                "required": ["prefixes"],
                                "not": {{
                                    "anyOf": [
                                        {{"required": ["context"]}},
                                        {{"required": ["document_iri"]}},
                                        {{"required": ["registry"]}}
                                    ]
                                }}
                            }}
                        ]
                    }}
                }}
            ],
            "required": ["version", "mode"],
            "type": "object"
        }}"#
        ))
        .expect("the options schema is JSON");
        schema.sort_keys();
        schema
    }
}

fn decode_options(
    value: &Value,
    limits: JsonLdContextLimits,
) -> Result<JsonLdSerializeOptions, RdfDiagnostic> {
    let object = value
        .as_object()
        .ok_or_else(|| context_error("JSON-LD options must be an object"))?;
    for key in object.keys() {
        if !OPTION_KEYS.contains(&key.as_str()) {
            return Err(context_error(format!(
                "unknown JSON-LD options member `{key}`"
            )));
        }
    }
    let version = object
        .get("version")
        .and_then(Value::as_u64)
        .ok_or_else(|| context_error("JSON-LD options require integer version 1"))?;
    if version != u64::from(JSON_LD_SERIALIZE_OPTIONS_VERSION) {
        return Err(context_error(format!(
            "unsupported JSON-LD options version `{version}`; expected {JSON_LD_SERIALIZE_OPTIONS_VERSION}"
        )));
    }
    let mode = object
        .get("mode")
        .and_then(Value::as_str)
        .ok_or_else(|| context_error("JSON-LD options require string mode"))?;

    let context_fields: BTreeSet<&str> = ["context", "document_iri", "prefixes", "registry"]
        .into_iter()
        .filter(|key| object.contains_key(key))
        .collect();
    let mut options = match mode {
        "expanded" => {
            reject_fields(mode, &context_fields)?;
            JsonLdSerializeOptions::expanded()
        }
        "derived" => {
            reject_fields(mode, &context_fields)?;
            JsonLdSerializeOptions::derived()
        }
        "context" => decode_context_mode(object, limits)?,
        other => {
            return Err(context_error(format!(
                "unknown JSON-LD serialization mode `{other}`"
            )));
        }
    };
    if let Some(compact_arrays) = object.get("compact_arrays") {
        let compact_arrays = compact_arrays
            .as_bool()
            .ok_or_else(|| context_error("compact_arrays must be a boolean"))?;
        options = options.with_compact_arrays(compact_arrays)?;
    }
    if let Some(schema_url) = object.get("yaml_schema_url") {
        let schema_url = schema_url
            .as_str()
            .ok_or_else(|| context_error("yaml_schema_url must be a string"))?;
        options = options.with_yaml_schema_url(schema_url)?;
    }
    Ok(options)
}

fn reject_fields(mode: &str, fields: &BTreeSet<&str>) -> Result<(), RdfDiagnostic> {
    if fields.is_empty() {
        return Ok(());
    }
    Err(context_error(format!(
        "JSON-LD mode `{mode}` does not accept {}",
        fields.iter().copied().collect::<Vec<_>>().join(", ")
    )))
}

fn decode_context_mode(
    object: &Object,
    limits: JsonLdContextLimits,
) -> Result<JsonLdSerializeOptions, RdfDiagnostic> {
    let has_context = object.contains_key("context");
    let has_prefixes = object.contains_key("prefixes");
    if has_context == has_prefixes {
        return Err(context_error(
            "JSON-LD context mode requires exactly one of context or prefixes",
        ));
    }
    if has_prefixes && (object.contains_key("document_iri") || object.contains_key("registry")) {
        return Err(context_error(
            "prefix-map context mode does not accept document_iri or registry",
        ));
    }
    if let Some(prefixes) = object.get("prefixes") {
        let prefixes = prefixes
            .as_object()
            .ok_or_else(|| context_error("JSON-LD prefixes must be an object"))?;
        let mappings: Result<Vec<(String, String)>, RdfDiagnostic> = prefixes
            .iter()
            .map(|(prefix, namespace)| {
                let namespace = namespace.as_str().ok_or_else(|| {
                    context_error(format!("prefix `{prefix}` namespace must be a string"))
                })?;
                Ok((prefix.clone(), namespace.to_owned()))
            })
            .collect();
        return JsonLdSerializeOptions::prefixes(mappings?);
    }

    let registry = decode_registry(object.get("registry"), limits)?;
    let document_iri = object.get("document_iri").map(|value| {
        value
            .as_str()
            .ok_or_else(|| context_error("document_iri must be a string"))
    });
    let document_iri = document_iri.transpose()?;
    JsonLdSerializeOptions::context_with_registry(
        object
            .get("context")
            .expect("context mode proved context is present"),
        document_iri,
        &registry,
    )
}

fn decode_registry(
    value: Option<&Value>,
    limits: JsonLdContextLimits,
) -> Result<JsonLdContextRegistry, RdfDiagnostic> {
    let Some(value) = value else {
        return Ok(JsonLdContextRegistry::default());
    };
    let object = value
        .as_object()
        .ok_or_else(|| context_error("JSON-LD registry must be an object"))?;
    let mut documents = BTreeMap::new();
    for (iri, document) in object {
        if documents.len() == limits.max_registry_documents() {
            return Err(super::context_limit(format!(
                "JSON-LD options registry exceeds {} documents",
                limits.max_registry_documents()
            )));
        }
        if !document
            .as_object()
            .is_some_and(|wrapper| wrapper.contains_key("@context"))
        {
            return Err(context_error(format!(
                "registry document `{iri}` must be an object containing @context"
            )));
        }
        let bytes = json::write_compact(&canonicalize(document)?).into_bytes();
        documents.insert(iri.clone(), bytes);
    }
    JsonLdContextRegistry::new_with_limits(documents, limits)
}
