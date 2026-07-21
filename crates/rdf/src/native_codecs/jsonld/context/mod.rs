// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Deterministic JSON-LD 1.1 context compilation and offline resolution.
//!
//! The types in this module form the semantic seam shared by JSON-LD and YAML-LD.
//! Context documents are caller-owned, decoded without duplicate members, resolved only
//! through an immutable local registry, and compiled into one active/inverse context.
//! No code in this module performs network I/O or supplies a vocabulary default.

mod compiler;
mod options;
mod strict;
#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use serde_json::Value;

use crate::RdfDiagnostic;

pub use options::{JsonLdSerializeMode, JsonLdSerializeOptions};

use self::strict::{StrictJsonLimits, parse_strict_json};

/// Version of the closed JSON-LD serialization-options document.
pub const JSON_LD_SERIALIZE_OPTIONS_VERSION: u32 = 1;

const DEFAULT_MAX_CONTEXT_BYTES: usize = 1_048_576;
const DEFAULT_MAX_REGISTRY_DOCUMENTS: usize = 128;
const DEFAULT_MAX_REGISTRY_BYTES: usize = 8_388_608;
const DEFAULT_MAX_TERMS: usize = 4_096;
const DEFAULT_MAX_NESTING: usize = 64;
const DEFAULT_MAX_EXPANSION_WORK: usize = 262_144;
const DEFAULT_MAX_DEFINITION_COMPLEXITY: usize = 131_072;
const MAX_JSON_LD_DOCUMENT_BYTES: usize = 256 * 1024 * 1024;
const MAX_JSON_LD_DOCUMENT_DEPTH: usize = 128;
// Raised to 2^23 in lock-step with `MAX_JSON_LD_CARRIER_ROWS`: a large whole-ontology
// document expands to millions of values, still inside the memory-safe decode envelope.
const MAX_JSON_LD_DOCUMENT_VALUES: usize = 8_388_608;

/// Fixed resource ceilings for context decoding and compilation.
///
/// Public compilation always uses [`Default`]. The fields are intentionally private:
/// every PurRDF consumer receives the same denial-of-service and portability envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JsonLdContextLimits {
    context_bytes: usize,
    registry_documents: usize,
    registry_bytes: usize,
    terms: usize,
    nesting: usize,
    expansion_work: usize,
    definition_complexity: usize,
}

impl JsonLdContextLimits {
    /// Maximum UTF-8 bytes in one supplied context or registry document.
    pub const fn max_context_bytes(self) -> usize {
        self.context_bytes
    }

    /// Maximum number of documents in an offline registry.
    pub const fn max_registry_documents(self) -> usize {
        self.registry_documents
    }

    /// Maximum aggregate bytes retained by an offline registry.
    pub const fn max_registry_bytes(self) -> usize {
        self.registry_bytes
    }

    /// Maximum UTF-8 bytes accepted by the shared serialization-options document.
    ///
    /// The envelope may contain one local context, the full aggregate offline
    /// registry, and bounded registry keys/JSON structure.
    pub const fn max_options_bytes(self) -> usize {
        self.context_bytes
            .saturating_add(self.registry_bytes)
            .saturating_add(self.context_bytes)
    }

    /// Maximum compiled term definitions across one compilation.
    pub const fn max_terms(self) -> usize {
        self.terms
    }

    /// Maximum JSON/context recursion depth.
    pub const fn max_nesting(self) -> usize {
        self.nesting
    }

    /// Maximum bounded context-processing operations.
    pub const fn max_expansion_work(self) -> usize {
        self.expansion_work
    }

    /// Maximum aggregate definition keys and scalar bytes processed.
    pub const fn max_definition_complexity(self) -> usize {
        self.definition_complexity
    }

    fn strict_json(self) -> StrictJsonLimits {
        StrictJsonLimits {
            bytes: self.context_bytes,
            depth: self.nesting,
            values: self.expansion_work,
        }
    }

    fn strict_options_json(self) -> StrictJsonLimits {
        StrictJsonLimits {
            bytes: self.max_options_bytes(),
            depth: self.nesting,
            values: self.expansion_work,
        }
    }
}

impl Default for JsonLdContextLimits {
    fn default() -> Self {
        Self {
            context_bytes: DEFAULT_MAX_CONTEXT_BYTES,
            registry_documents: DEFAULT_MAX_REGISTRY_DOCUMENTS,
            registry_bytes: DEFAULT_MAX_REGISTRY_BYTES,
            terms: DEFAULT_MAX_TERMS,
            nesting: DEFAULT_MAX_NESTING,
            expansion_work: DEFAULT_MAX_EXPANSION_WORK,
            definition_complexity: DEFAULT_MAX_DEFINITION_COMPLEXITY,
        }
    }
}

/// Base direction carried by a JSON-LD 1.1 context or term definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JsonLdDirection {
    /// Left-to-right text.
    LeftToRight,
    /// Right-to-left text.
    RightToLeft,
}

impl JsonLdDirection {
    /// JSON-LD spelling of this direction.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LeftToRight => "ltr",
            Self::RightToLeft => "rtl",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "ltr" => Some(Self::LeftToRight),
            "rtl" => Some(Self::RightToLeft),
            _ => None,
        }
    }
}

/// Explicit nullable mapping in a JSON-LD term definition.
///
/// The surrounding `Option` used by term getters distinguishes an absent mapping from
/// either of these explicit states.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JsonLdNullable<T> {
    /// The context explicitly resets the inherited value with JSON `null`.
    Null,
    /// The context supplies a concrete value.
    Value(T),
}

impl<T> JsonLdNullable<T> {
    /// Borrow the concrete value while preserving the explicit-null state.
    pub const fn as_ref(&self) -> JsonLdNullable<&T> {
        match self {
            Self::Null => JsonLdNullable::Null,
            Self::Value(value) => JsonLdNullable::Value(value),
        }
    }
}

/// One JSON-LD 1.1 container mapping component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JsonLdContainer {
    /// Preserve a set even when it contains one value.
    Set,
    /// Compact an RDF collection through a list object.
    List,
    /// Index values by language.
    Language,
    /// Index values by `@index` or a mapped index property.
    Index,
    /// Index node objects by `@id`.
    Id,
    /// Index node objects by `@type`.
    Type,
    /// Compact named graph objects.
    Graph,
}

impl JsonLdContainer {
    /// JSON-LD keyword spelling of this container component.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Set => "@set",
            Self::List => "@list",
            Self::Language => "@language",
            Self::Index => "@index",
            Self::Id => "@id",
            Self::Type => "@type",
            Self::Graph => "@graph",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "@set" => Some(Self::Set),
            "@list" => Some(Self::List),
            "@language" => Some(Self::Language),
            "@index" => Some(Self::Index),
            "@id" => Some(Self::Id),
            "@type" => Some(Self::Type),
            "@graph" => Some(Self::Graph),
            _ => None,
        }
    }
}

/// Type coercion attached to a compiled JSON-LD term.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JsonLdTypeMapping {
    /// Coerce strings to node identifiers resolved document-relatively.
    Id,
    /// Coerce strings to node identifiers resolved against `@vocab`.
    Vocab,
    /// Carry an `rdf:JSON` literal as native JSON.
    Json,
    /// Explicitly select no type mapping.
    None,
    /// Coerce values to the supplied absolute datatype IRI.
    Datatype(String),
}

impl JsonLdTypeMapping {
    fn inverse_key(&self) -> &str {
        match self {
            Self::Id => "@id",
            Self::Vocab => "@vocab",
            Self::Json => "@json",
            Self::None => "@none",
            Self::Datatype(iri) => iri,
        }
    }
}

/// Compiled definition for one term or keyword alias.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonLdTermDefinition {
    iri_mapping: Option<String>,
    reverse_property: bool,
    prefix: bool,
    protected: bool,
    type_mapping: Option<JsonLdTypeMapping>,
    language_mapping: Option<JsonLdNullable<String>>,
    direction_mapping: Option<JsonLdNullable<JsonLdDirection>>,
    containers: BTreeSet<JsonLdContainer>,
    index_mapping: Option<String>,
    nest: Option<String>,
    scoped_context: Option<Value>,
    scoped_context_base: Option<String>,
}

impl JsonLdTermDefinition {
    /// Expanded IRI or keyword selected by this term; `None` is a null mapping.
    pub fn iri_mapping(&self) -> Option<&str> {
        self.iri_mapping.as_deref()
    }

    /// Whether the term is a reverse property.
    pub const fn is_reverse_property(&self) -> bool {
        self.reverse_property
    }

    /// Whether the term may be used as a compact-IRI prefix.
    pub const fn is_prefix(&self) -> bool {
        self.prefix
    }

    /// Whether later contexts are forbidden from changing this definition.
    pub const fn is_protected(&self) -> bool {
        self.protected
    }

    /// Optional type coercion.
    pub const fn type_mapping(&self) -> Option<&JsonLdTypeMapping> {
        self.type_mapping.as_ref()
    }

    /// Explicit language mapping; outer `None` means the member was absent.
    pub fn language_mapping(&self) -> Option<JsonLdNullable<&str>> {
        self.language_mapping.as_ref().map(|value| match value {
            JsonLdNullable::Null => JsonLdNullable::Null,
            JsonLdNullable::Value(language) => JsonLdNullable::Value(language.as_str()),
        })
    }

    /// Explicit direction mapping; outer `None` means the member was absent.
    pub const fn direction_mapping(&self) -> Option<JsonLdNullable<JsonLdDirection>> {
        match &self.direction_mapping {
            None => None,
            Some(JsonLdNullable::Null) => Some(JsonLdNullable::Null),
            Some(JsonLdNullable::Value(direction)) => Some(JsonLdNullable::Value(*direction)),
        }
    }

    /// Deterministically ordered container mapping.
    pub const fn containers(&self) -> &BTreeSet<JsonLdContainer> {
        &self.containers
    }

    /// Expanded custom index-property mapping, when configured.
    pub fn index_mapping(&self) -> Option<&str> {
        self.index_mapping.as_deref()
    }

    /// `@nest` keyword or alias used by the term.
    pub fn nest(&self) -> Option<&str> {
        self.nest.as_deref()
    }

    /// Canonical term-scoped local context, when present.
    pub const fn scoped_context(&self) -> Option<&Value> {
        self.scoped_context.as_ref()
    }

    /// Base URL retained for normative processing of the term-scoped context.
    pub fn scoped_context_base_iri(&self) -> Option<&str> {
        self.scoped_context_base.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ActiveContext {
    original_base_iri: Option<String>,
    base_iri: Option<String>,
    vocab_mapping: Option<String>,
    default_language: Option<String>,
    default_direction: Option<JsonLdDirection>,
    propagate: bool,
    previous_context: Option<Arc<Self>>,
    terms: BTreeMap<String, JsonLdTermDefinition>,
}

impl ActiveContext {
    fn new(base_iri: Option<String>) -> Self {
        Self {
            original_base_iri: base_iri.clone(),
            base_iri,
            vocab_mapping: None,
            default_language: None,
            default_direction: None,
            propagate: true,
            previous_context: None,
            terms: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct InverseSelection {
    types: BTreeMap<String, String>,
    languages: BTreeMap<String, String>,
    fallback: BTreeMap<String, String>,
}

type InverseContext = BTreeMap<String, BTreeMap<String, InverseSelection>>;

/// Immutable collection of caller-supplied context documents keyed by absolute IRI.
///
/// Construction validates identifiers and fixed resource ceilings. Compilation can
/// resolve context IRIs and `@import` only through this collection; no network loader
/// exists.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JsonLdContextRegistry {
    documents: Arc<BTreeMap<String, Arc<[u8]>>>,
    total_bytes: usize,
}

impl JsonLdContextRegistry {
    /// Construct a duplicate-free immutable registry from JSON document bytes.
    pub fn new<I, K, B>(documents: I) -> Result<Self, RdfDiagnostic>
    where
        I: IntoIterator<Item = (K, B)>,
        K: Into<String>,
        B: Into<Vec<u8>>,
    {
        Self::new_with_limits(documents, JsonLdContextLimits::default())
    }

    fn new_with_limits<I, K, B>(
        documents: I,
        limits: JsonLdContextLimits,
    ) -> Result<Self, RdfDiagnostic>
    where
        I: IntoIterator<Item = (K, B)>,
        K: Into<String>,
        B: Into<Vec<u8>>,
    {
        let mut retained = BTreeMap::new();
        let mut total_bytes = 0usize;
        for (iri, bytes) in documents {
            if retained.len() == limits.max_registry_documents() {
                return Err(context_limit(format!(
                    "offline context registry exceeds {} documents",
                    limits.max_registry_documents()
                )));
            }
            let iri = iri.into();
            validate_absolute_iri(&iri, "offline context document IRI")?;
            let bytes = bytes.into();
            if bytes.len() > limits.max_context_bytes() {
                return Err(context_limit(format!(
                    "offline context document `{iri}` is {} bytes; limit is {}",
                    bytes.len(),
                    limits.max_context_bytes()
                )));
            }
            total_bytes = total_bytes
                .checked_add(bytes.len())
                .ok_or_else(|| context_limit("offline context registry byte count overflow"))?;
            if total_bytes > limits.max_registry_bytes() {
                return Err(context_limit(format!(
                    "offline context registry exceeds {} aggregate bytes",
                    limits.max_registry_bytes()
                )));
            }
            if retained.insert(iri.clone(), Arc::from(bytes)).is_some() {
                return Err(context_error(format!(
                    "duplicate offline context document IRI `{iri}`"
                )));
            }
        }
        Ok(Self {
            documents: Arc::new(retained),
            total_bytes,
        })
    }

    /// Number of registered context documents.
    pub fn len(&self) -> usize {
        self.documents.len()
    }

    /// Whether no context documents are registered.
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }

    /// Aggregate number of retained JSON bytes.
    pub const fn total_bytes(&self) -> usize {
        self.total_bytes
    }

    fn get(&self, iri: &str) -> Option<&[u8]> {
        self.documents.get(iri).map(AsRef::as_ref)
    }
}

/// Immutable compiled JSON-LD 1.1 active and inverse context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledJsonLdContext {
    canonical_context: Value,
    active: ActiveContext,
    inverse: InverseContext,
    registry: JsonLdContextRegistry,
}

impl CompiledJsonLdContext {
    /// Compile a caller-owned context JSON value without a registry.
    pub fn compile(context: &Value, document_iri: Option<&str>) -> Result<Self, RdfDiagnostic> {
        Self::compile_with_registry(context, document_iri, &JsonLdContextRegistry::default())
    }

    /// Strictly decode and compile caller-owned JSON bytes without a registry.
    pub fn compile_json(bytes: &[u8], document_iri: Option<&str>) -> Result<Self, RdfDiagnostic> {
        Self::compile_json_with_registry(bytes, document_iri, &JsonLdContextRegistry::default())
    }

    /// Compile a context value with immutable offline context-IRI and `@import` lookup.
    pub fn compile_with_registry(
        context: &Value,
        document_iri: Option<&str>,
        registry: &JsonLdContextRegistry,
    ) -> Result<Self, RdfDiagnostic> {
        compiler::compile(
            context,
            document_iri,
            registry,
            JsonLdContextLimits::default(),
        )
    }

    /// Strictly decode and compile context JSON with immutable offline lookup.
    pub fn compile_json_with_registry(
        bytes: &[u8],
        document_iri: Option<&str>,
        registry: &JsonLdContextRegistry,
    ) -> Result<Self, RdfDiagnostic> {
        let limits = JsonLdContextLimits::default();
        let context = parse_strict_json(bytes, limits.strict_json(), "JSON-LD context")?;
        compiler::compile(&context, document_iri, registry, limits)
    }

    /// Compile a context IRI resolved solely through an immutable offline registry.
    pub fn compile_registry_context(
        context_iri: &str,
        registry: &JsonLdContextRegistry,
    ) -> Result<Self, RdfDiagnostic> {
        Self::compile_with_registry(&Value::String(context_iri.to_owned()), None, registry)
    }

    /// Compile deterministic `@prefix: true` definitions from a caller prefix map.
    pub fn from_prefixes<I, K, V>(prefixes: I) -> Result<Self, RdfDiagnostic>
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let mut context = BTreeMap::new();
        let limits = JsonLdContextLimits::default();
        for (prefix, namespace) in prefixes {
            if context.len() == limits.max_terms() {
                return Err(context_limit(format!(
                    "JSON-LD prefix map exceeds {} entries",
                    limits.max_terms()
                )));
            }
            let prefix = prefix.into();
            let namespace = namespace.into();
            if context.contains_key(&prefix) {
                return Err(context_error(format!(
                    "duplicate JSON-LD prefix `{prefix}`"
                )));
            }
            let definition = Value::Object(
                BTreeMap::from([
                    ("@id".to_owned(), Value::String(namespace)),
                    ("@prefix".to_owned(), Value::Bool(true)),
                ])
                .into_iter()
                .collect(),
            );
            context.insert(prefix, definition);
        }
        Self::compile(&Value::Object(context.into_iter().collect()), None)
    }

    /// Canonical caller context retained for deterministic emission.
    pub const fn canonical_context(&self) -> &Value {
        &self.canonical_context
    }

    /// Immutable offline registry retained for term-scoped context reuse.
    pub const fn registry(&self) -> &JsonLdContextRegistry {
        &self.registry
    }

    /// Canonical compact JSON bytes for the retained context.
    pub fn canonical_json(&self) -> String {
        serde_json::to_string(&self.canonical_context)
            .expect("serde_json::Value serialization is infallible")
    }

    /// Active base IRI, when configured.
    pub fn base_iri(&self) -> Option<&str> {
        self.active.base_iri.as_deref()
    }

    /// Active vocabulary mapping, when configured.
    pub fn vocab_mapping(&self) -> Option<&str> {
        self.active.vocab_mapping.as_deref()
    }

    /// Active default language, normalized to lowercase.
    pub fn default_language(&self) -> Option<&str> {
        self.active.default_language.as_deref()
    }

    /// Active default base direction.
    pub const fn default_direction(&self) -> Option<JsonLdDirection> {
        self.active.default_direction
    }

    /// Whether this context propagates to nested node objects.
    pub const fn propagates(&self) -> bool {
        self.active.propagate
    }

    /// Whether `@propagate: false` retained a previous active context.
    pub const fn has_previous_context(&self) -> bool {
        self.active.previous_context.is_some()
    }

    /// Deterministically ordered compiled term definitions.
    pub const fn terms(&self) -> &BTreeMap<String, JsonLdTermDefinition> {
        &self.active.terms
    }

    /// Definition for a term or keyword alias.
    pub fn term(&self, term: &str) -> Option<&JsonLdTermDefinition> {
        self.active.terms.get(term)
    }

    /// Compile the term-scoped context attached to `term` over this active context.
    ///
    /// Returns `None` when the term has no scoped context. Offline registry and base-URL
    /// state are retained; protected definitions may be overridden as required by the
    /// JSON-LD 1.1 scoped-context algorithm.
    pub fn scoped_context(&self, term: &str) -> Result<Option<Self>, RdfDiagnostic> {
        let Some(definition) = self.term(term) else {
            return Ok(None);
        };
        compiler::compile_scoped(self, definition)
    }

    /// Apply a node-local context over this compiled active context.
    pub fn apply_local_context(&self, context: &Value) -> Result<Self, RdfDiagnostic> {
        compiler::apply_context(self, context, self.base_iri(), false)
    }

    /// Active context inherited by a nested node object under `@propagate` rules.
    #[must_use]
    pub fn child_context(&self) -> Self {
        compiler::child_context(self)
    }

    /// Expand an IRI, compact IRI, term, or keyword under this active context.
    ///
    /// `vocab` selects vocabulary-relative term expansion; `document_relative`
    /// selects resolution against the active `@base`. A null term mapping returns
    /// `Ok(None)`.
    pub fn expand_iri(
        &self,
        value: &str,
        vocab: bool,
        document_relative: bool,
    ) -> Result<Option<String>, RdfDiagnostic> {
        compiler::expand_iri(&self.active, value, vocab, document_relative)
    }

    /// Compact one absolute IRI without an associated value.
    ///
    /// This follows JSON-LD's `value = null` compaction branches. Vocabulary
    /// positions consult the inverse context and prefix definitions; document
    /// positions compact against `@base` where the round-trip is exact.
    pub fn compact_iri(&self, iri: &str, vocab: bool) -> Result<String, RdfDiagnostic> {
        compiler::compact_iri(&self.active, &self.inverse, iri, vocab, None)
    }

    /// Compact an absolute vocabulary IRI using explicit inverse-context value-shape
    /// preferences.
    ///
    /// A present selection represents an associated non-null value. `None` has the
    /// same `value = null` semantics as [`Self::compact_iri`].
    pub fn compact_iri_with_selection(
        &self,
        iri: &str,
        vocab: bool,
        selection: Option<&JsonLdTermSelection>,
    ) -> Result<String, RdfDiagnostic> {
        compiler::compact_iri(&self.active, &self.inverse, iri, vocab, selection)
    }
}

/// Inverse-context branch used while selecting a compact term.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonLdTermSelectionKind {
    /// Select by type coercion (`@id`, `@vocab`, datatype, or `@none`).
    Type,
    /// Select by language/base-direction preference.
    Language,
    /// Select the shortest/lexically first term independent of value coercion.
    Any,
}

/// Ordered inverse-context preferences for one IRI-compaction decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonLdTermSelection {
    containers: Vec<String>,
    kind: JsonLdTermSelectionKind,
    preferred_values: Vec<String>,
}

impl JsonLdTermSelection {
    /// Construct an ordered selection from container sets and preferred inverse keys.
    ///
    /// An empty container set is represented as JSON-LD's `@none` container. Container
    /// sets and preference values retain caller order; members inside each set are
    /// canonicalized lexically.
    pub fn new<I, C, P, S>(
        containers: I,
        kind: JsonLdTermSelectionKind,
        preferred_values: P,
    ) -> Self
    where
        I: IntoIterator<Item = C>,
        C: IntoIterator<Item = JsonLdContainer>,
        P: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let containers = containers
            .into_iter()
            .map(|container| {
                let mut values: Vec<&str> =
                    container.into_iter().map(JsonLdContainer::as_str).collect();
                if values.is_empty() {
                    "@none".to_owned()
                } else {
                    values.sort_unstable();
                    values.concat()
                }
            })
            .collect();
        Self {
            containers,
            kind,
            preferred_values: preferred_values.into_iter().map(Into::into).collect(),
        }
    }

    /// Ordered canonical container keys consulted by inverse selection.
    pub fn container_keys(&self) -> &[String] {
        &self.containers
    }

    /// Selected inverse-context branch.
    pub const fn kind(&self) -> JsonLdTermSelectionKind {
        self.kind
    }

    /// Ordered type or language/direction preference keys.
    pub fn preferred_values(&self) -> &[String] {
        &self.preferred_values
    }
}

fn canonicalize(value: &Value) -> Value {
    match value {
        Value::Array(values) => Value::Array(values.iter().map(canonicalize).collect()),
        Value::Object(values) => {
            let sorted: BTreeMap<String, Value> = values
                .iter()
                .map(|(key, value)| (key.clone(), canonicalize(value)))
                .collect();
            Value::Object(sorted.into_iter().collect())
        }
        scalar => scalar.clone(),
    }
}

pub(super) fn parse_document(bytes: &[u8]) -> Result<Value, RdfDiagnostic> {
    parse_strict_json(
        bytes,
        StrictJsonLimits {
            bytes: MAX_JSON_LD_DOCUMENT_BYTES,
            depth: MAX_JSON_LD_DOCUMENT_DEPTH,
            values: MAX_JSON_LD_DOCUMENT_VALUES,
        },
        "JSON-LD document",
    )
}

fn validate_absolute_iri(iri: &str, description: &str) -> Result<(), RdfDiagnostic> {
    let parsed = purrdf_iri::parse(iri)
        .map_err(|source| context_error(format!("invalid {description} `{iri}`: {source}")))?;
    if !parsed.has_scheme() {
        return Err(context_error(format!(
            "{description} must be absolute: `{iri}`"
        )));
    }
    Ok(())
}

fn context_error(message: impl Into<String>) -> RdfDiagnostic {
    RdfDiagnostic::error("jsonld-context-invalid", message)
}

fn context_limit(message: impl Into<String>) -> RdfDiagnostic {
    RdfDiagnostic::error("jsonld-context-limit", message)
}
