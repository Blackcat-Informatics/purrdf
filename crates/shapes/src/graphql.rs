// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Deterministic [`CompiledSchema`] → GraphQL September 2025 type-system emitter.
//!
//! The generated SDL is a type-system fragment, not an executable service:
//! operation roots, resolvers, authorization, and pagination are caller-owned.
//! Every structural object is emitted as paired output and input types. A
//! canonical name map and the package's value codec translate JSON property
//! names and finite JSON values without inventing a vocabulary.
//!
//! GraphQL variable coercion is not JSON Schema validation. Differences such
//! as singleton-list coercion, fixed input-field sets, custom-scalar behavior,
//! and required/null presence are recorded on [`GraphqlPackage::losses`] at
//! their source JSON Pointer locations.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Write as _};

use ::purrdf::RdfLocation;
use ::purrdf::loss::{LossEntry, LossLedger};
use serde::Serialize;
use serde_json::{Map, Value};

use crate::json_schema::CompiledSchema;
use crate::schema_catalog::{
    CompiledSchemaCatalog, definition_path, pointer_escape, reference_key, schema_array_keywords,
    schema_map_keywords, schema_single_keywords,
};

/// Fixed GraphQL language revision used by the emitter and loss registry.
pub const GRAPHQL_DIALECT: &str = "graphql-september-2025";

/// Relative path of the generated GraphQL type-system SDL artifact.
pub const GRAPHQL_SCHEMA_PATH: &str = "schema.graphql";

/// Relative path of the generated canonical JSON name-map artifact.
pub const GRAPHQL_NAME_MAP_PATH: &str = "name-map.json";

const LOSS_FROM: &str = "json-schema";
const LOSS_CONTEXT: &str = "graphql-emitter";
const MAX_SCHEMA_JSON_BYTES: usize = 16 * 1024 * 1024;
const MAX_ARTIFACT_BYTES: usize = 16 * 1024 * 1024;
const MAX_VALUE_JSON_BYTES: usize = 16 * 1024 * 1024;
const MAX_DEFINITIONS: usize = 65_536;
const MAX_FIELDS: usize = 65_536;
const MAX_ENUM_VALUES: usize = 65_536;
const MAX_SCHEMA_DEPTH: usize = 128;
const MAX_GRAPHQL_NAME_BYTES: usize = 255;

/// Caller-owned identity and prose for a generated GraphQL schema package.
///
/// There is intentionally no [`Default`] implementation. PurRDF does not
/// fabricate package identity, documentation, or scalar semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphqlConfig {
    schema_name: String,
    package_docstring: String,
    module_docstring: String,
    fallback_scalar_name: String,
}

impl GraphqlConfig {
    /// Validate and construct GraphQL emitter configuration.
    ///
    /// `schema_name` and `fallback_scalar_name` must be non-introspection
    /// GraphQL Names. The fallback scalar must not be a built-in scalar. Prose
    /// values must be caller-supplied, non-whitespace text; line endings are
    /// canonicalized to LF.
    ///
    /// # Errors
    ///
    /// Returns [`GraphqlError`] for invalid names, blank prose, or unsupported
    /// control characters.
    pub fn new(
        schema_name: impl Into<String>,
        package_docstring: impl Into<String>,
        module_docstring: impl Into<String>,
        fallback_scalar_name: impl Into<String>,
    ) -> Result<Self, GraphqlError> {
        let schema_name = schema_name.into();
        validate_graphql_name("schema name", &schema_name)?;
        let fallback_scalar_name = fallback_scalar_name.into();
        validate_graphql_name("fallback scalar name", &fallback_scalar_name)?;
        if is_builtin_type(&fallback_scalar_name) {
            return Err(GraphqlError::new(format!(
                "GraphQL fallback scalar name {fallback_scalar_name:?} collides with a built-in \
                 GraphQL type"
            )));
        }
        Ok(Self {
            schema_name,
            package_docstring: normalize_prose("package docstring", &package_docstring.into())?,
            module_docstring: normalize_prose("module docstring", &module_docstring.into())?,
            fallback_scalar_name,
        })
    }

    /// Caller-supplied package/schema identifier.
    #[must_use]
    pub fn schema_name(&self) -> &str {
        &self.schema_name
    }

    /// Caller-supplied package documentation.
    #[must_use]
    pub fn package_docstring(&self) -> &str {
        &self.package_docstring
    }

    /// Caller-supplied module documentation.
    #[must_use]
    pub fn module_docstring(&self) -> &str {
        &self.module_docstring
    }

    /// Caller-supplied custom scalar used for values without an exact built-in
    /// GraphQL carrier.
    #[must_use]
    pub fn fallback_scalar_name(&self) -> &str {
        &self.fallback_scalar_name
    }
}

/// Output/input GraphQL type expressions for one source `$defs` key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GraphqlDefinitionMap {
    /// Type expression used at GraphQL output position.
    pub output_type: String,
    /// Type expression used at GraphQL input position.
    pub input_type: String,
}

/// One finite source JSON value and its generated GraphQL enum symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GraphqlEnumValueMap {
    /// Exact source JSON value.
    pub source_value: Value,
    /// Generated GraphQL enum symbol.
    pub graphql_name: String,
}

/// Typed, deterministic source-name/value → GraphQL-name map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GraphqlNameMap {
    /// Fixed emitter dialect.
    pub dialect: String,
    /// Caller-owned schema/package identifier.
    pub schema_name: String,
    /// Source `$defs` key → output/input type expressions.
    pub definitions: BTreeMap<String, GraphqlDefinitionMap>,
    /// Object-schema JSON Pointer → source property → GraphQL field.
    pub fields: BTreeMap<String, BTreeMap<String, String>>,
    /// Finite-schema JSON Pointer → exact values and GraphQL enum symbols.
    pub enum_values: BTreeMap<String, Vec<GraphqlEnumValueMap>>,
}

/// Deterministic generated GraphQL package and its projection losses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphqlPackage {
    /// Caller-owned schema/package identifier copied from [`GraphqlConfig`].
    pub schema_name: String,
    /// Relative package paths → exact file bytes, sorted by path.
    pub artifacts: BTreeMap<String, Vec<u8>>,
    /// Typed copy of the canonical `name-map.json` contract.
    pub names: GraphqlNameMap,
    /// JSON Schema assertions not represented exactly by GraphQL coercion.
    pub losses: LossLedger,
    source_definitions: BTreeMap<String, Value>,
    representations: BTreeMap<String, Representation>,
    reference_targets: BTreeMap<String, String>,
}

impl GraphqlPackage {
    /// Translate one source JSON value into the generated GraphQL input naming
    /// and finite-enum transport representation.
    ///
    /// # Errors
    ///
    /// Returns [`GraphqlError`] for an unknown definition, an unmapped field or
    /// enum value, a structurally incompatible carrier, excessive nesting, or
    /// a value beyond the fixed byte limit.
    pub fn encode_input(&self, definition: &str, value: &Value) -> Result<Value, GraphqlError> {
        self.translate_definition(definition, value, CodecDirection::EncodeInput)
    }

    /// Translate one generated GraphQL output value back to source JSON field
    /// names and exact finite JSON values.
    ///
    /// GraphQL responses may contain a selected subset of output fields; this
    /// method translates present fields and does not invent omitted values.
    ///
    /// # Errors
    ///
    /// Returns [`GraphqlError`] for an unknown definition, field, or enum symbol,
    /// a structurally incompatible carrier, excessive nesting, or a value beyond
    /// the fixed byte limit.
    pub fn decode_output(&self, definition: &str, value: &Value) -> Result<Value, GraphqlError> {
        self.translate_definition(definition, value, CodecDirection::DecodeOutput)
    }

    fn translate_definition(
        &self,
        definition: &str,
        value: &Value,
        direction: CodecDirection,
    ) -> Result<Value, GraphqlError> {
        ensure_value_size(value)?;
        let schema = self.source_definitions.get(definition).ok_or_else(|| {
            GraphqlError::new(format!("unknown GraphQL source definition {definition:?}"))
        })?;
        let translated =
            self.translate_value(schema, &definition_path(definition), value, direction, 0)?;
        ensure_value_size(&translated)?;
        Ok(translated)
    }
}

/// A malformed GraphQL configuration, input schema, generated name graph, or
/// value-codec request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphqlError {
    message: String,
}

impl GraphqlError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for GraphqlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for GraphqlError {}

/// Emit deterministic paired GraphQL output/input SDL and a canonical name map
/// from one compiled SHACL-derived JSON Schema.
///
/// Source-stage losses remain on [`CompiledSchema::losses`]. The returned
/// ledger describes only the `json-schema` → `graphql-september-2025`
/// projection.
///
/// # Errors
///
/// Returns [`GraphqlError`] when configuration or schema input is malformed or
/// too large, a reference is open/dangling, generated names collide, a schema
/// value exceeds a fixed resource limit, or the resulting artifacts exceed
/// their fixed byte limits.
pub fn emit_graphql(
    compiled: &CompiledSchema,
    config: &GraphqlConfig,
) -> Result<GraphqlPackage, GraphqlError> {
    if compiled.schema_json.len() > MAX_SCHEMA_JSON_BYTES {
        return Err(GraphqlError::new(format!(
            "CompiledSchema.schema_json exceeds the {MAX_SCHEMA_JSON_BYTES}-byte GraphQL emitter \
             input limit"
        )));
    }
    let catalog = CompiledSchemaCatalog::parse(compiled)
        .map_err(|error| GraphqlError::new(error.to_string()))?;
    let definitions = catalog.definitions();
    if definitions.len() > MAX_DEFINITIONS {
        return Err(GraphqlError::new(format!(
            "CompiledSchema contains {} definitions; GraphQL emission is limited to \
             {MAX_DEFINITIONS}",
            definitions.len()
        )));
    }
    for (key, definition) in definitions {
        validate_schema_keywords(definition, &definition_path(key), 0)?;
    }

    let mut planner = Planner::new(definitions, config)?;
    planner.plan()?;
    planner.audit()?;
    planner.relax_invalid_input_cycles()?;
    let definition_maps = planner.definition_maps()?;
    let sdl = planner.render_sdl()?;
    if sdl.len() > MAX_ARTIFACT_BYTES {
        return Err(GraphqlError::new(format!(
            "generated GraphQL SDL exceeds the {MAX_ARTIFACT_BYTES}-byte output limit"
        )));
    }
    let names = GraphqlNameMap {
        dialect: GRAPHQL_DIALECT.to_owned(),
        schema_name: config.schema_name.clone(),
        definitions: definition_maps,
        fields: planner.fields.clone(),
        enum_values: planner.enum_values.clone(),
    };
    let mut name_map_json = serde_json::to_string_pretty(&names).map_err(|error| {
        GraphqlError::new(format!("cannot serialize GraphQL name map: {error}"))
    })?;
    name_map_json.push('\n');
    if name_map_json.len() > MAX_ARTIFACT_BYTES {
        return Err(GraphqlError::new(format!(
            "generated GraphQL name map exceeds the {MAX_ARTIFACT_BYTES}-byte output limit"
        )));
    }

    let mut artifacts = BTreeMap::new();
    artifacts.insert(GRAPHQL_NAME_MAP_PATH.to_owned(), name_map_json.into_bytes());
    artifacts.insert(GRAPHQL_SCHEMA_PATH.to_owned(), sdl.into_bytes());
    Ok(GraphqlPackage {
        schema_name: config.schema_name.clone(),
        artifacts,
        names,
        losses: planner.ledger,
        source_definitions: definitions
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
        representations: planner.representations,
        reference_targets: planner.reference_targets,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Representation {
    Enum,
    Object,
    Array,
    Reference(String),
    String,
    Boolean,
    Int,
    Fallback,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ObjectNames {
    output: String,
    input: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TypePosition {
    Output,
    Input,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CodecDirection {
    EncodeInput,
    DecodeOutput,
}

struct Planner<'a> {
    definitions: &'a Map<String, Value>,
    config: &'a GraphqlConfig,
    representations: BTreeMap<String, Representation>,
    schemas: BTreeMap<String, Value>,
    objects: BTreeMap<String, ObjectNames>,
    enums: BTreeMap<String, String>,
    fields: BTreeMap<String, BTreeMap<String, String>>,
    enum_values: BTreeMap<String, Vec<GraphqlEnumValueMap>>,
    used_type_names: BTreeMap<String, String>,
    reference_targets: BTreeMap<String, String>,
    ledger: LossLedger,
    recorded_losses: BTreeSet<(String, String)>,
    relaxed_fields: BTreeSet<String>,
}

impl<'a> Planner<'a> {
    fn new(
        definitions: &'a Map<String, Value>,
        config: &'a GraphqlConfig,
    ) -> Result<Self, GraphqlError> {
        let mut used_type_names = BTreeMap::new();
        used_type_names.insert(
            config.fallback_scalar_name.clone(),
            "caller-supplied fallback scalar".to_owned(),
        );
        Ok(Self {
            definitions,
            config,
            representations: BTreeMap::new(),
            schemas: BTreeMap::new(),
            objects: BTreeMap::new(),
            enums: BTreeMap::new(),
            fields: BTreeMap::new(),
            enum_values: BTreeMap::new(),
            used_type_names,
            reference_targets: BTreeMap::new(),
            ledger: LossLedger::new(),
            recorded_losses: BTreeSet::new(),
            relaxed_fields: BTreeSet::new(),
        })
    }

    fn plan(&mut self) -> Result<(), GraphqlError> {
        for (key, schema) in self.definitions {
            let path = definition_path(key);
            let base = graphql_type_name(key, "SchemaType");
            self.plan_schema(schema, &path, &base, 0)?;
        }
        self.resolve_reference_targets()?;
        Ok(())
    }

    fn resolve_reference_targets(&mut self) -> Result<(), GraphqlError> {
        for start in self.definitions.keys() {
            if self.reference_targets.contains_key(start) {
                continue;
            }
            let mut chain = Vec::<String>::new();
            let mut positions = BTreeMap::<String, usize>::new();
            let mut current = start.clone();
            let endpoint = loop {
                if let Some(endpoint) = self.reference_targets.get(&current) {
                    break endpoint.clone();
                }
                if let Some(index) = positions.insert(current.clone(), chain.len()) {
                    let mut cycle = chain[index..].to_vec();
                    cycle.push(current);
                    return Err(GraphqlError::new(format!(
                        "CompiledSchema contains an unguarded GraphQL alias cycle: {}",
                        cycle.join(" -> ")
                    )));
                }
                chain.push(current.clone());
                match self.representations.get(&definition_path(&current)) {
                    Some(Representation::Reference(next)) => current.clone_from(next),
                    Some(_) => break current,
                    None => {
                        return Err(GraphqlError::new(format!(
                            "missing planned GraphQL representation for definition {current:?}"
                        )));
                    }
                }
            };
            for key in chain {
                self.reference_targets.insert(key, endpoint.clone());
            }
        }
        Ok(())
    }

    fn plan_schema(
        &mut self,
        schema: &Value,
        path: &str,
        base: &str,
        depth: usize,
    ) -> Result<(), GraphqlError> {
        ensure_depth(depth, path)?;
        if self.representations.contains_key(path) {
            return Ok(());
        }
        let representation = classify_schema(schema, path)?;
        self.schemas.insert(path.to_owned(), schema.clone());
        self.representations
            .insert(path.to_owned(), representation.clone());

        match representation {
            Representation::Enum => {
                self.reserve_type_name(base, path)?;
                let values = finite_values(schema, path)?.ok_or_else(|| {
                    GraphqlError::new(format!("{path} was classified as a finite GraphQL enum"))
                })?;
                if values.len() > MAX_ENUM_VALUES {
                    return Err(GraphqlError::new(format!(
                        "{path} contains {} enum values; GraphQL emission is limited to \
                         {MAX_ENUM_VALUES}",
                        values.len()
                    )));
                }
                let mappings = values
                    .into_iter()
                    .enumerate()
                    .map(|(index, source_value)| GraphqlEnumValueMap {
                        source_value,
                        graphql_name: format!("VALUE_{index}"),
                    })
                    .collect();
                self.enums.insert(path.to_owned(), base.to_owned());
                self.enum_values.insert(path.to_owned(), mappings);
            }
            Representation::Object => {
                let object = schema
                    .as_object()
                    .expect("object representation comes from an object schema");
                let properties = object
                    .get("properties")
                    .and_then(Value::as_object)
                    .cloned()
                    .unwrap_or_default();
                let required = required_names(object, path)?;
                if matches!(object.get("additionalProperties"), Some(Value::Bool(false))) {
                    let property_names = properties.keys().cloned().collect::<BTreeSet<_>>();
                    if let Some(name) = required.difference(&property_names).next() {
                        return Err(GraphqlError::new(format!(
                            "{path}/required names {name:?}, but the closed object has no matching \
                             property schema"
                        )));
                    }
                }
                let source_fields: BTreeSet<String> = properties
                    .keys()
                    .cloned()
                    .chain(required.iter().cloned())
                    .collect();
                if source_fields.len() > MAX_FIELDS {
                    return Err(GraphqlError::new(format!(
                        "{path} contains {} fields; GraphQL emission is limited to {MAX_FIELDS}",
                        source_fields.len()
                    )));
                }
                let mut field_map = BTreeMap::new();
                let mut reverse = BTreeMap::<String, String>::new();
                for source in source_fields {
                    let graphql = graphql_field_name(&source, "field");
                    validate_graphql_name("generated field name", &graphql)?;
                    if let Some(previous) = reverse.insert(graphql.clone(), source.clone()) {
                        return Err(GraphqlError::new(format!(
                            "{path} properties {previous:?} and {source:?} collide on GraphQL field \
                             name {graphql:?}"
                        )));
                    }
                    field_map.insert(source, graphql);
                }
                let output = base.to_owned();
                let input = checked_graphql_name(&format!("{base}Input"), "generated input type")?;
                self.reserve_type_name(&output, path)?;
                self.reserve_type_name(&input, path)?;
                self.objects
                    .insert(path.to_owned(), ObjectNames { output, input });
                self.fields.insert(path.to_owned(), field_map.clone());

                for (source, child) in properties {
                    let child_path = format!("{path}/properties/{}", pointer_escape(&source));
                    let graphql = field_map
                        .get(&source)
                        .expect("field map covers every property");
                    let child_base = graphql_type_name(&format!("{base} {graphql}"), "InlineValue");
                    self.plan_schema(&child, &child_path, &child_base, depth + 1)?;
                }
            }
            Representation::Array => {
                let object = schema
                    .as_object()
                    .expect("array representation comes from an object schema");
                if let Some(items) = object.get("items") {
                    let child_base = graphql_type_name(&format!("{base} item"), "ArrayItem");
                    self.plan_schema(items, &format!("{path}/items"), &child_base, depth + 1)?;
                }
            }
            Representation::Reference(_)
            | Representation::String
            | Representation::Boolean
            | Representation::Int
            | Representation::Fallback => {}
        }
        Ok(())
    }

    fn reserve_type_name(&mut self, name: &str, path: &str) -> Result<(), GraphqlError> {
        validate_graphql_name("generated type name", name)?;
        if is_builtin_type(name) {
            return Err(GraphqlError::new(format!(
                "{path} normalizes to built-in GraphQL type name {name:?}"
            )));
        }
        if let Some(previous) = self
            .used_type_names
            .insert(name.to_owned(), path.to_owned())
        {
            return Err(GraphqlError::new(format!(
                "GraphQL type name {name:?} collides between {previous} and {path}"
            )));
        }
        Ok(())
    }

    fn audit(&mut self) -> Result<(), GraphqlError> {
        for (key, schema) in self.definitions {
            self.audit_schema(schema, &definition_path(key), 0)?;
        }
        Ok(())
    }

    fn audit_schema(
        &mut self,
        schema: &Value,
        path: &str,
        depth: usize,
    ) -> Result<(), GraphqlError> {
        ensure_depth(depth, path)?;
        let representation = self
            .representations
            .get(path)
            .cloned()
            .unwrap_or(Representation::Fallback);
        let Value::Object(object) = schema else {
            if representation == Representation::Fallback {
                self.record(
                    "custom-scalar-validation-delegated",
                    path,
                    "a boolean JSON Schema is carried by the caller-owned fallback scalar",
                );
            }
            return Ok(());
        };

        if representation == Representation::Fallback {
            self.record(
                "custom-scalar-validation-delegated",
                path,
                "this schema has no exact built-in GraphQL carrier",
            );
        }
        if object.contains_key("allOf") {
            self.record(
                "intersection-validation-delegated",
                &format!("{path}/allOf"),
                "GraphQL has no input intersection type",
            );
        }
        if object.contains_key("anyOf") && representation != Representation::Enum {
            self.record(
                "union-validation-delegated",
                &format!("{path}/anyOf"),
                "GraphQL has no general input union type",
            );
        }
        if object.contains_key("oneOf") && representation != Representation::Enum {
            self.record(
                "one-of-validation-delegated",
                &format!("{path}/oneOf"),
                "GraphQL has no exactly-one input union validation",
            );
        }
        if object.contains_key("not") {
            self.record(
                "negation-validation-delegated",
                &format!("{path}/not"),
                "GraphQL has no input value-set complement",
            );
        }
        if object
            .get("prefixItems")
            .and_then(Value::as_array)
            .is_some_and(|values| !values.is_empty())
        {
            self.record(
                "tuple-array-validation-delegated",
                &format!("{path}/prefixItems"),
                "GraphQL lists cannot express position-specific tuple item schemas",
            );
        }
        let type_kinds = declared_types(object, path)?;
        if type_kinds
            .as_ref()
            .is_some_and(|kinds| kinds.iter().filter(|kind| **kind != JsonKind::Null).count() > 1)
        {
            self.record(
                "union-validation-delegated",
                &format!("{path}/type"),
                "GraphQL has no general input union for a JSON Schema type array",
            );
        }
        if type_kinds.as_ref().is_some_and(|kinds| {
            kinds.contains(&JsonKind::Integer) && representation != Representation::Int
        }) {
            self.record(
                "integer-domain-validation-delegated",
                &format!("{path}/type"),
                "the JSON integer domain is not exactly GraphQL's signed 32-bit Int domain",
            );
        }

        let active_conditional = object.contains_key("if")
            && (object.contains_key("then") || object.contains_key("else"));
        if active_conditional {
            for keyword in ["if", "then", "else"] {
                if object.contains_key(keyword) {
                    self.record(
                        "conditional-validation-dropped",
                        &format!("{path}/{keyword}"),
                        "GraphQL input fields cannot express content-dependent branches",
                    );
                }
            }
        }
        for keyword in ["unevaluatedItems", "unevaluatedProperties"] {
            if object.contains_key(keyword) {
                self.record(
                    "unevaluated-validation-dropped",
                    &format!("{path}/{keyword}"),
                    "GraphQL coercion does not expose JSON Schema applicator evaluation state",
                );
            }
        }
        for key in object.keys() {
            if !known_schema_keyword(key) && !is_annotation_keyword(key) {
                self.record(
                    "keyword-validation-delegated",
                    &format!("{path}/{}", pointer_escape(key)),
                    &format!(
                        "JSON Schema assertion keyword {key:?} is outside the closed GraphQL \
                         capability table"
                    ),
                );
            }
        }
        if object.contains_key("additionalItems") {
            self.record(
                "keyword-validation-delegated",
                &format!("{path}/additionalItems"),
                "additionalItems is outside the draft 2020-12 GraphQL capability table",
            );
        }

        match representation {
            Representation::Object => self.audit_object(object, path, depth)?,
            Representation::Array => self.audit_array(object, path, depth)?,
            Representation::String => {
                for keyword in [
                    "minLength",
                    "maxLength",
                    "pattern",
                    "format",
                    "contentEncoding",
                    "contentMediaType",
                    "contentSchema",
                ] {
                    if object.contains_key(keyword) {
                        self.record(
                            "string-validation-dropped",
                            &format!("{path}/{keyword}"),
                            "GraphQL String does not express this runtime string predicate",
                        );
                    }
                }
            }
            Representation::Int => {
                for keyword in [
                    "minimum",
                    "maximum",
                    "exclusiveMinimum",
                    "exclusiveMaximum",
                    "multipleOf",
                ] {
                    if object.contains_key(keyword) && !matches!(keyword, "minimum" | "maximum") {
                        self.record(
                            "numeric-validation-dropped",
                            &format!("{path}/{keyword}"),
                            "GraphQL Int does not express this runtime numeric predicate",
                        );
                    }
                }
            }
            Representation::Enum
            | Representation::Reference(_)
            | Representation::Boolean
            | Representation::Fallback => {}
        }
        Ok(())
    }

    fn audit_object(
        &mut self,
        object: &Map<String, Value>,
        path: &str,
        depth: usize,
    ) -> Result<(), GraphqlError> {
        if !matches!(object.get("additionalProperties"), Some(Value::Bool(false))) {
            self.record(
                "additional-properties-validation-narrowed",
                object
                    .contains_key("additionalProperties")
                    .then(|| format!("{path}/additionalProperties"))
                    .as_deref()
                    .unwrap_or(path),
                "GraphQL input objects reject source keys outside the declared field set",
            );
        }
        if let Some(patterns) = object.get("patternProperties").and_then(Value::as_object) {
            for pattern in patterns.keys() {
                self.record(
                    "pattern-properties-validation-changed",
                    &format!("{path}/patternProperties/{}", pointer_escape(pattern)),
                    "GraphQL fixed fields cannot preserve regex-selected dynamic or overlapping keys",
                );
            }
        }
        if object.contains_key("propertyNames") {
            self.record(
                "property-name-validation-changed",
                &format!("{path}/propertyNames"),
                "GraphQL fixed fields do not apply the source schema to each runtime key name",
            );
        }
        for keyword in ["minProperties", "maxProperties"] {
            if object.contains_key(keyword) {
                self.record(
                    "property-count-validation-dropped",
                    &format!("{path}/{keyword}"),
                    "GraphQL cannot count supplied input object fields",
                );
            }
        }
        for keyword in ["dependentRequired", "dependentSchemas"] {
            if object.contains_key(keyword) {
                self.record(
                    "dependency-validation-dropped",
                    &format!("{path}/{keyword}"),
                    "GraphQL input object definitions cannot enforce cross-field dependencies",
                );
            }
        }

        let empty = Map::new();
        let properties = object
            .get("properties")
            .and_then(Value::as_object)
            .unwrap_or(&empty);
        let required = required_names(object, path)?;
        for (source, child) in properties {
            let child_path = format!("{path}/properties/{}", pointer_escape(source));
            let nullable = schema_allows_null(child, self.definitions, &mut BTreeSet::new())?;
            if (required.contains(source) && nullable) || (!required.contains(source) && !nullable)
            {
                self.record(
                    "nullable-presence-validation-widened",
                    &child_path,
                    "GraphQL nullable fields cannot preserve this source presence/null distinction",
                );
            }
            self.audit_schema(child, &child_path, depth + 1)?;
        }
        let property_names = properties.keys().cloned().collect::<BTreeSet<_>>();
        for source in required.difference(&property_names) {
            let required_path = format!("{path}/required/{}", pointer_escape(source));
            self.record(
                "custom-scalar-validation-delegated",
                &required_path,
                "a required property without a named schema uses the caller-owned fallback scalar",
            );
            self.record(
                "nullable-presence-validation-widened",
                &required_path,
                "GraphQL cannot require presence while allowing null for an unconstrained value",
            );
        }
        Ok(())
    }

    fn audit_array(
        &mut self,
        object: &Map<String, Value>,
        path: &str,
        depth: usize,
    ) -> Result<(), GraphqlError> {
        self.record(
            "singleton-list-coercion-widened",
            path,
            "GraphQL variable coercion wraps a non-list input as a singleton list",
        );
        for keyword in ["minItems", "maxItems"] {
            if object.contains_key(keyword) {
                self.record(
                    "array-cardinality-validation-dropped",
                    &format!("{path}/{keyword}"),
                    "GraphQL list types cannot constrain list length",
                );
            }
        }
        if object.contains_key("contains") {
            for keyword in ["contains", "minContains", "maxContains"] {
                if object.contains_key(keyword) {
                    self.record(
                        "array-contains-validation-dropped",
                        &format!("{path}/{keyword}"),
                        "GraphQL list coercion cannot quantify matching elements",
                    );
                }
            }
        }
        if object.get("uniqueItems") == Some(&Value::Bool(true)) {
            self.record(
                "unique-items-validation-dropped",
                &format!("{path}/uniqueItems"),
                "GraphQL list coercion cannot enforce pairwise-distinct values",
            );
        }
        if let Some(items) = object.get("items") {
            self.audit_schema(items, &format!("{path}/items"), depth + 1)?;
        } else {
            self.record(
                "custom-scalar-validation-delegated",
                &format!("{path}/items"),
                "an unconstrained list item uses the caller-owned fallback scalar",
            );
        }
        Ok(())
    }

    fn record(&mut self, code: &str, path: &str, note: &str) {
        if !self
            .recorded_losses
            .insert((code.to_owned(), path.to_owned()))
        {
            return;
        }
        self.ledger.record(LossEntry {
            code: code.to_owned().into(),
            from: LOSS_FROM.into(),
            to: GRAPHQL_DIALECT.into(),
            note: note.to_owned().into(),
            location: Some(Box::new(
                RdfLocation::logical(LOSS_CONTEXT).with_subject(path),
            )),
        });
    }

    fn definition_maps(&self) -> Result<BTreeMap<String, GraphqlDefinitionMap>, GraphqlError> {
        let mut maps = BTreeMap::new();
        for (key, schema) in self.definitions {
            let path = definition_path(key);
            maps.insert(
                key.clone(),
                GraphqlDefinitionMap {
                    output_type: self.type_expression(schema, &path, TypePosition::Output)?,
                    input_type: self.type_expression(schema, &path, TypePosition::Input)?,
                },
            );
        }
        Ok(maps)
    }

    fn type_expression(
        &self,
        schema: &Value,
        path: &str,
        position: TypePosition,
    ) -> Result<String, GraphqlError> {
        let representation = self
            .representations
            .get(path)
            .cloned()
            .unwrap_or(Representation::Fallback);
        match representation {
            Representation::Enum => self.enums.get(path).cloned().ok_or_else(|| {
                GraphqlError::new(format!("missing generated enum name for {path}"))
            }),
            Representation::Object => {
                let names = self.objects.get(path).ok_or_else(|| {
                    GraphqlError::new(format!("missing generated object names for {path}"))
                })?;
                Ok(match position {
                    TypePosition::Output => names.output.clone(),
                    TypePosition::Input => names.input.clone(),
                })
            }
            Representation::Array => {
                let object = schema
                    .as_object()
                    .expect("array representation comes from an object schema");
                let (item_schema, item_path) = object.get("items").map_or_else(
                    || (None, format!("{path}/items")),
                    |items| (Some(items), format!("{path}/items")),
                );
                let item_type = if let Some(items) = item_schema {
                    self.type_expression(items, &item_path, position)?
                } else {
                    self.config.fallback_scalar_name.clone()
                };
                let nullable = item_schema.is_none_or(|items| {
                    schema_allows_null(items, self.definitions, &mut BTreeSet::new())
                        .unwrap_or(true)
                });
                Ok(if nullable {
                    format!("[{item_type}]")
                } else {
                    format!("[{item_type}!]")
                })
            }
            Representation::Reference(key) => {
                let endpoint = self.reference_targets.get(&key).ok_or_else(|| {
                    GraphqlError::new(format!("missing resolved GraphQL reference for {key:?}"))
                })?;
                let target = self.definitions.get(endpoint).ok_or_else(|| {
                    GraphqlError::new(format!("{path}/$ref targets missing $defs key {key:?}"))
                })?;
                self.type_expression(target, &definition_path(endpoint), position)
            }
            Representation::String => Ok("String".to_owned()),
            Representation::Boolean => Ok("Boolean".to_owned()),
            Representation::Int => Ok("Int".to_owned()),
            Representation::Fallback => Ok(self.config.fallback_scalar_name.clone()),
        }
    }

    fn render_sdl(&self) -> Result<String, GraphqlError> {
        let mut output = String::new();
        write_comment(&mut output, &self.config.package_docstring);
        write_comment(&mut output, &self.config.module_docstring);
        writeln!(output, "# Package: {}", self.config.schema_name)
            .expect("writing GraphQL SDL to a String cannot fail");
        writeln!(output).expect("writing GraphQL SDL to a String cannot fail");
        writeln!(output, "scalar {}", self.config.fallback_scalar_name)
            .expect("writing GraphQL SDL to a String cannot fail");

        let mut declarations = Vec::<(String, DeclarationKind, String)>::new();
        for (path, name) in &self.enums {
            declarations.push((name.clone(), DeclarationKind::Enum, path.clone()));
        }
        for (path, names) in &self.objects {
            declarations.push((names.output.clone(), DeclarationKind::Output, path.clone()));
            declarations.push((names.input.clone(), DeclarationKind::Input, path.clone()));
        }
        declarations.sort();
        for (_, kind, path) in declarations {
            output.push('\n');
            match kind {
                DeclarationKind::Enum => self.render_enum(&mut output, &path)?,
                DeclarationKind::Output => {
                    self.render_object_type(&mut output, &path, TypePosition::Output)?;
                }
                DeclarationKind::Input => {
                    self.render_object_type(&mut output, &path, TypePosition::Input)?;
                }
            }
        }
        Ok(finish_text(output))
    }

    fn render_enum(&self, output: &mut String, path: &str) -> Result<(), GraphqlError> {
        let name = self
            .enums
            .get(path)
            .expect("rendered enum has a planned name");
        if let Some(description) = schema_doc(
            self.schemas
                .get(path)
                .expect("rendered enum has a retained schema"),
        )? {
            writeln!(output, "{}", graphql_string(description))
                .expect("writing GraphQL SDL to a String cannot fail");
        }
        writeln!(output, "enum {name} {{").expect("writing GraphQL SDL to a String cannot fail");
        for mapping in self
            .enum_values
            .get(path)
            .expect("rendered enum has value mappings")
        {
            writeln!(output, "  {}", mapping.graphql_name)
                .expect("writing GraphQL SDL to a String cannot fail");
        }
        writeln!(output, "}}").expect("writing GraphQL SDL to a String cannot fail");
        Ok(())
    }

    fn render_object_type(
        &self,
        output: &mut String,
        path: &str,
        position: TypePosition,
    ) -> Result<(), GraphqlError> {
        let schema = self
            .schemas
            .get(path)
            .expect("rendered object has a retained schema");
        let object = schema
            .as_object()
            .expect("rendered object has an object schema");
        if let Some(description) = schema_doc(schema)? {
            writeln!(output, "{}", graphql_string(description))
                .expect("writing GraphQL SDL to a String cannot fail");
        }
        let names = self
            .objects
            .get(path)
            .expect("rendered object has planned names");
        let (keyword, name) = match position {
            TypePosition::Output => ("type", names.output.as_str()),
            TypePosition::Input => ("input", names.input.as_str()),
        };
        writeln!(output, "{keyword} {name} {{")
            .expect("writing GraphQL SDL to a String cannot fail");

        let empty = Map::new();
        let properties = object
            .get("properties")
            .and_then(Value::as_object)
            .unwrap_or(&empty);
        let required = required_names(object, path)?;
        let field_map = self
            .fields
            .get(path)
            .expect("rendered object has a field map");
        let mut ordered_fields = field_map.iter().collect::<Vec<_>>();
        ordered_fields.sort_by(|left, right| left.1.cmp(right.1));
        for (source, graphql) in ordered_fields {
            let child_path = format!("{path}/properties/{}", pointer_escape(source));
            let child = properties.get(source);
            if let Some(description) = child.map(schema_doc).transpose()?.flatten() {
                writeln!(output, "  {}", graphql_string(description))
                    .expect("writing GraphQL SDL to a String cannot fail");
            }
            let mut expression = if let Some(child) = child {
                self.type_expression(child, &child_path, position)?
            } else {
                self.config.fallback_scalar_name.clone()
            };
            let source_non_null = required.contains(source)
                && child.is_some_and(|schema| {
                    !schema_allows_null(schema, self.definitions, &mut BTreeSet::new())
                        .unwrap_or(true)
                });
            let relaxed =
                position == TypePosition::Input && self.relaxed_fields.contains(&child_path);
            if source_non_null && !relaxed {
                expression.push('!');
            }
            writeln!(output, "  {graphql}: {expression}")
                .expect("writing GraphQL SDL to a String cannot fail");
        }
        writeln!(output, "}}").expect("writing GraphQL SDL to a String cannot fail");
        Ok(())
    }

    fn relax_invalid_input_cycles(&mut self) -> Result<(), GraphqlError> {
        let edges = self.mandatory_input_edges()?;
        while let Some(path) = find_cycle_edge(&self.objects, &edges, &self.relaxed_fields) {
            self.relaxed_fields.insert(path.clone());
            self.record(
                "recursive-input-nullability-relaxed",
                &path,
                "this singular required non-null edge closes an invalid GraphQL input cycle",
            );
        }
        Ok(())
    }

    fn mandatory_input_edges(&self) -> Result<Vec<InputEdge>, GraphqlError> {
        let mut edges = Vec::new();
        for (path, schema) in &self.schemas {
            if self.representations.get(path) != Some(&Representation::Object) {
                continue;
            }
            let object = schema
                .as_object()
                .expect("planned object representation has an object schema");
            let empty = Map::new();
            let properties = object
                .get("properties")
                .and_then(Value::as_object)
                .unwrap_or(&empty);
            let required = required_names(object, path)?;
            for (source, child) in properties {
                if !required.contains(source)
                    || schema_allows_null(child, self.definitions, &mut BTreeSet::new())?
                {
                    continue;
                }
                let child_path = format!("{path}/properties/{}", pointer_escape(source));
                if let Some(target) = self.resolve_object_path(&child_path)? {
                    edges.push(InputEdge {
                        source: path.clone(),
                        target,
                        field_path: child_path,
                    });
                }
            }
        }
        edges.sort();
        Ok(edges)
    }

    fn resolve_object_path(&self, path: &str) -> Result<Option<String>, GraphqlError> {
        match self.representations.get(path) {
            Some(Representation::Object) => Ok(Some(path.to_owned())),
            Some(Representation::Reference(key)) => {
                let endpoint = self.reference_targets.get(key).ok_or_else(|| {
                    GraphqlError::new(format!("missing resolved GraphQL reference for {key:?}"))
                })?;
                let target_path = definition_path(endpoint);
                Ok(
                    (self.representations.get(&target_path) == Some(&Representation::Object))
                        .then_some(target_path),
                )
            }
            _ => Ok(None),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum DeclarationKind {
    Enum,
    Output,
    Input,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct InputEdge {
    source: String,
    target: String,
    field_path: String,
}

impl GraphqlPackage {
    fn translate_value(
        &self,
        schema: &Value,
        path: &str,
        value: &Value,
        direction: CodecDirection,
        depth: usize,
    ) -> Result<Value, GraphqlError> {
        ensure_depth(depth, path)?;
        let representation = self
            .representations
            .get(path)
            .cloned()
            .unwrap_or(Representation::Fallback);
        if representation != Representation::Enum && value.is_null() {
            return Ok(Value::Null);
        }
        match representation {
            Representation::Enum => self.translate_enum(path, value, direction),
            Representation::Object => self.translate_object(schema, path, value, direction, depth),
            Representation::Array => {
                let values = value.as_array().ok_or_else(|| {
                    GraphqlError::new(format!("GraphQL value at {path} must be a JSON array"))
                })?;
                let object = schema
                    .as_object()
                    .expect("array representation comes from an object schema");
                let items = object.get("items");
                let mut translated = Vec::with_capacity(values.len());
                for (index, item) in values.iter().enumerate() {
                    translated.push(if let Some(item_schema) = items {
                        self.translate_value(
                            item_schema,
                            &format!("{path}/items"),
                            item,
                            direction,
                            depth + 1,
                        )?
                    } else {
                        item.clone()
                    });
                    ensure_depth(depth + 1, &format!("{path}/value/{index}"))?;
                }
                Ok(Value::Array(translated))
            }
            Representation::Reference(key) => {
                let endpoint = self.reference_targets.get(&key).ok_or_else(|| {
                    GraphqlError::new(format!("missing resolved GraphQL reference for {key:?}"))
                })?;
                let target = self.source_definitions.get(endpoint).ok_or_else(|| {
                    GraphqlError::new(format!("{path}/$ref targets missing definition {key:?}"))
                })?;
                self.translate_value(
                    target,
                    &definition_path(endpoint),
                    value,
                    direction,
                    depth + 1,
                )
            }
            Representation::String => {
                if value.is_string() {
                    Ok(value.clone())
                } else {
                    Err(GraphqlError::new(format!(
                        "GraphQL value at {path} must be a JSON string"
                    )))
                }
            }
            Representation::Boolean => {
                if value.is_boolean() {
                    Ok(value.clone())
                } else {
                    Err(GraphqlError::new(format!(
                        "GraphQL value at {path} must be a JSON boolean"
                    )))
                }
            }
            Representation::Int => {
                if value
                    .as_i64()
                    .is_some_and(|integer| i32::try_from(integer).is_ok())
                    || value
                        .as_u64()
                        .is_some_and(|integer| i32::try_from(integer).is_ok())
                {
                    Ok(value.clone())
                } else {
                    Err(GraphqlError::new(format!(
                        "GraphQL value at {path} must be a signed 32-bit integer"
                    )))
                }
            }
            Representation::Fallback => Ok(value.clone()),
        }
    }

    fn translate_enum(
        &self,
        path: &str,
        value: &Value,
        direction: CodecDirection,
    ) -> Result<Value, GraphqlError> {
        let mappings =
            self.names.enum_values.get(path).ok_or_else(|| {
                GraphqlError::new(format!("GraphQL enum at {path} has no value map"))
            })?;
        match direction {
            CodecDirection::EncodeInput => mappings
                .iter()
                .find(|mapping| mapping.source_value == *value)
                .map(|mapping| Value::String(mapping.graphql_name.clone()))
                .ok_or_else(|| {
                    GraphqlError::new(format!(
                        "source value at {path} is not present in the generated GraphQL enum map"
                    ))
                }),
            CodecDirection::DecodeOutput => {
                let symbol = value.as_str().ok_or_else(|| {
                    GraphqlError::new(format!(
                        "GraphQL enum output at {path} must be a string symbol"
                    ))
                })?;
                mappings
                    .iter()
                    .find(|mapping| mapping.graphql_name == symbol)
                    .map(|mapping| mapping.source_value.clone())
                    .ok_or_else(|| {
                        GraphqlError::new(format!(
                            "GraphQL enum symbol {symbol:?} is not mapped at {path}"
                        ))
                    })
            }
        }
    }

    fn translate_object(
        &self,
        schema: &Value,
        path: &str,
        value: &Value,
        direction: CodecDirection,
        depth: usize,
    ) -> Result<Value, GraphqlError> {
        let value = value.as_object().ok_or_else(|| {
            GraphqlError::new(format!("GraphQL value at {path} must be a JSON object"))
        })?;
        let schema = schema
            .as_object()
            .expect("object representation comes from an object schema");
        let empty = Map::new();
        let properties = schema
            .get("properties")
            .and_then(Value::as_object)
            .unwrap_or(&empty);
        let field_map = self.names.fields.get(path).ok_or_else(|| {
            GraphqlError::new(format!("GraphQL object at {path} has no field map"))
        })?;
        let reverse = field_map
            .iter()
            .map(|(source, graphql)| (graphql.as_str(), source.as_str()))
            .collect::<BTreeMap<_, _>>();
        let mut output = Map::new();
        for (input_key, input_value) in value {
            let (source_key, output_key) = match direction {
                CodecDirection::EncodeInput => {
                    let graphql = field_map.get(input_key).ok_or_else(|| {
                        GraphqlError::new(format!(
                            "source field {input_key:?} is not representable by GraphQL object \
                             at {path}"
                        ))
                    })?;
                    (input_key.as_str(), graphql.as_str())
                }
                CodecDirection::DecodeOutput => {
                    let source = reverse.get(input_key.as_str()).ok_or_else(|| {
                        GraphqlError::new(format!(
                            "GraphQL field {input_key:?} is not mapped at {path}"
                        ))
                    })?;
                    (*source, *source)
                }
            };
            let child_path = format!("{path}/properties/{}", pointer_escape(source_key));
            let translated = if let Some(child) = properties.get(source_key) {
                self.translate_value(child, &child_path, input_value, direction, depth + 1)?
            } else {
                input_value.clone()
            };
            if output.insert(output_key.to_owned(), translated).is_some() {
                return Err(GraphqlError::new(format!(
                    "GraphQL value translation produced duplicate field {output_key:?} at {path}"
                )));
            }
        }
        Ok(Value::Object(output))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum JsonKind {
    Null,
    Boolean,
    Integer,
    Number,
    String,
    Array,
    Object,
}

fn classify_schema(schema: &Value, path: &str) -> Result<Representation, GraphqlError> {
    let Value::Object(object) = schema else {
        return Ok(Representation::Fallback);
    };
    if finite_values(schema, path)?.is_some() {
        return Ok(Representation::Enum);
    }
    if let Some(reference) = pure_reference(object) {
        let key = reference_key(reference).ok_or_else(|| {
            GraphqlError::new(format!("{path}/$ref is not a direct #/$defs reference"))
        })?;
        return Ok(Representation::Reference(key));
    }
    if has_unknown_assertion(object)
        || object.contains_key("allOf")
        || object.contains_key("anyOf")
        || object.contains_key("oneOf")
        || object.contains_key("not")
    {
        return Ok(Representation::Fallback);
    }
    let Some(kinds) = declared_types(object, path)? else {
        return Ok(Representation::Fallback);
    };
    let non_null = kinds
        .iter()
        .copied()
        .filter(|kind| *kind != JsonKind::Null)
        .collect::<BTreeSet<_>>();
    if non_null.len() != 1 {
        return Ok(Representation::Fallback);
    }
    Ok(match *non_null.iter().next().expect("length checked") {
        JsonKind::String => Representation::String,
        JsonKind::Boolean => Representation::Boolean,
        JsonKind::Integer if is_exact_graphql_int_domain(object) => Representation::Int,
        JsonKind::Integer | JsonKind::Number => Representation::Fallback,
        JsonKind::Array
            if object
                .get("prefixItems")
                .and_then(Value::as_array)
                .is_none_or(Vec::is_empty)
                && object.get("items") != Some(&Value::Bool(false)) =>
        {
            Representation::Array
        }
        JsonKind::Object if object_field_count(object, path)? > 0 => Representation::Object,
        JsonKind::Null | JsonKind::Array | JsonKind::Object => Representation::Fallback,
    })
}

fn finite_values(schema: &Value, path: &str) -> Result<Option<Vec<Value>>, GraphqlError> {
    let Value::Object(object) = schema else {
        return Ok(None);
    };
    if object.keys().any(|key| {
        !is_annotation_keyword(key)
            && !matches!(key.as_str(), "type" | "enum" | "const" | "anyOf" | "oneOf")
    }) {
        return Ok(None);
    }
    let values = if let Some(value) = object.get("const") {
        if object.contains_key("enum")
            || object.contains_key("anyOf")
            || object.contains_key("oneOf")
        {
            return Ok(None);
        }
        vec![value.clone()]
    } else if let Some(values) = object.get("enum") {
        if object.contains_key("anyOf") || object.contains_key("oneOf") {
            return Ok(None);
        }
        values
            .as_array()
            .expect("enum shape is validated before classification")
            .clone()
    } else if let Some(branches) = object.get("anyOf").or_else(|| object.get("oneOf")) {
        let mut values = Vec::new();
        for (index, branch) in branches
            .as_array()
            .expect("applicator shape is validated before classification")
            .iter()
            .enumerate()
        {
            let Some(branch_values) = finite_values(branch, &format!("{path}/branch/{index}"))?
            else {
                return Ok(None);
            };
            values.extend(branch_values);
        }
        values
    } else if object.get("type").is_some_and(|value| value == "null") {
        vec![Value::Null]
    } else {
        return Ok(None);
    };

    if let Some(kinds) = declared_types(object, path)?
        && values
            .iter()
            .any(|value| !value_matches_types(value, &kinds))
    {
        return Ok(None);
    }
    let mut canonical = BTreeMap::<String, Value>::new();
    for value in values {
        let key = serde_json::to_string(&value)
            .map_err(|error| GraphqlError::new(format!("cannot inspect {path}: {error}")))?;
        canonical.insert(key, value);
    }
    if canonical.is_empty() {
        return Ok(None);
    }
    Ok(Some(canonical.into_values().collect()))
}

fn pure_reference(object: &Map<String, Value>) -> Option<&str> {
    let reference = object.get("$ref")?.as_str()?;
    object
        .keys()
        .all(|key| key == "$ref" || is_annotation_keyword(key))
        .then_some(reference)
}

fn declared_types(
    object: &Map<String, Value>,
    path: &str,
) -> Result<Option<BTreeSet<JsonKind>>, GraphqlError> {
    let Some(value) = object.get("type") else {
        return Ok(None);
    };
    let values = match value {
        Value::String(value) => vec![(value.as_str(), format!("{path}/type"))],
        Value::Array(values) if !values.is_empty() => values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                value
                    .as_str()
                    .map(|value| (value, format!("{path}/type/{index}")))
                    .ok_or_else(|| {
                        GraphqlError::new(format!("{path}/type/{index} must be a string"))
                    })
            })
            .collect::<Result<Vec<_>, _>>()?,
        Value::Array(_) => {
            return Err(GraphqlError::new(format!(
                "{path}/type array cannot be empty"
            )));
        }
        _ => {
            return Err(GraphqlError::new(format!(
                "{path}/type must be a string or non-empty array of strings"
            )));
        }
    };
    let mut kinds = BTreeSet::new();
    for (value, value_path) in values {
        let kind = match value {
            "null" => JsonKind::Null,
            "boolean" => JsonKind::Boolean,
            "integer" => JsonKind::Integer,
            "number" => JsonKind::Number,
            "string" => JsonKind::String,
            "array" => JsonKind::Array,
            "object" => JsonKind::Object,
            _ => {
                return Err(GraphqlError::new(format!(
                    "{value_path} names unsupported JSON Schema type {value:?}"
                )));
            }
        };
        if !kinds.insert(kind) {
            return Err(GraphqlError::new(format!(
                "{path}/type repeats type {value:?}"
            )));
        }
    }
    Ok(Some(kinds))
}

fn value_matches_types(value: &Value, kinds: &BTreeSet<JsonKind>) -> bool {
    let kind = match value {
        Value::Null => JsonKind::Null,
        Value::Bool(_) => JsonKind::Boolean,
        Value::Number(number) if number.is_i64() || number.is_u64() => JsonKind::Integer,
        Value::Number(_) => JsonKind::Number,
        Value::String(_) => JsonKind::String,
        Value::Array(_) => JsonKind::Array,
        Value::Object(_) => JsonKind::Object,
    };
    kinds.contains(&kind) || (kind == JsonKind::Integer && kinds.contains(&JsonKind::Number))
}

fn is_exact_graphql_int_domain(object: &Map<String, Value>) -> bool {
    object.get("minimum").and_then(Value::as_i64) == Some(i64::from(i32::MIN))
        && object.get("maximum").and_then(Value::as_i64) == Some(i64::from(i32::MAX))
        && !object.contains_key("exclusiveMinimum")
        && !object.contains_key("exclusiveMaximum")
}

fn object_field_count(object: &Map<String, Value>, path: &str) -> Result<usize, GraphqlError> {
    let properties = object
        .get("properties")
        .and_then(Value::as_object)
        .map_or(0, Map::len);
    let required = required_names(object, path)?;
    let property_names = object
        .get("properties")
        .and_then(Value::as_object)
        .map(|properties| properties.keys().cloned().collect::<BTreeSet<_>>())
        .unwrap_or_default();
    Ok(properties + required.difference(&property_names).count())
}

fn validate_schema_keywords(schema: &Value, path: &str, depth: usize) -> Result<(), GraphqlError> {
    ensure_depth(depth, path)?;
    let Value::Object(object) = schema else {
        return if schema.is_boolean() {
            Ok(())
        } else {
            Err(GraphqlError::new(format!(
                "{path} must be an object or boolean JSON Schema"
            )))
        };
    };
    let _ = declared_types(object, path)?;
    if let Some(value) = object.get("enum") {
        let values = value
            .as_array()
            .ok_or_else(|| GraphqlError::new(format!("{path}/enum must be an array")))?;
        if values.is_empty() {
            return Err(GraphqlError::new(format!("{path}/enum cannot be empty")));
        }
        if values.len() > MAX_ENUM_VALUES {
            return Err(GraphqlError::new(format!(
                "{path}/enum exceeds the {MAX_ENUM_VALUES}-value limit"
            )));
        }
        let mut seen = BTreeSet::new();
        for (index, value) in values.iter().enumerate() {
            let canonical = serde_json::to_string(value).map_err(|error| {
                GraphqlError::new(format!("cannot inspect {path}/enum/{index}: {error}"))
            })?;
            if !seen.insert(canonical) {
                return Err(GraphqlError::new(format!(
                    "{path}/enum repeats value at index {index}"
                )));
            }
        }
    }
    for keyword in ["allOf", "anyOf", "oneOf"] {
        if object
            .get(keyword)
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty)
        {
            return Err(GraphqlError::new(format!(
                "{path}/{keyword} cannot be empty"
            )));
        }
    }
    for keyword in [
        "title",
        "description",
        "pattern",
        "format",
        "contentEncoding",
        "contentMediaType",
    ] {
        if object.get(keyword).is_some_and(|value| !value.is_string()) {
            return Err(GraphqlError::new(format!(
                "{path}/{keyword} must be a string"
            )));
        }
    }
    for keyword in [
        "minimum",
        "maximum",
        "exclusiveMinimum",
        "exclusiveMaximum",
        "multipleOf",
    ] {
        if object.get(keyword).is_some_and(|value| !value.is_number()) {
            return Err(GraphqlError::new(format!(
                "{path}/{keyword} must be a number"
            )));
        }
    }
    if object
        .get("multipleOf")
        .and_then(Value::as_f64)
        .is_some_and(|value| value <= 0.0)
    {
        return Err(GraphqlError::new(format!(
            "{path}/multipleOf must be greater than zero"
        )));
    }
    for keyword in [
        "minLength",
        "maxLength",
        "minItems",
        "maxItems",
        "minContains",
        "maxContains",
        "minProperties",
        "maxProperties",
    ] {
        if object.contains_key(keyword) {
            validate_nonnegative_integer(
                object
                    .get(keyword)
                    .expect("present keyword was just checked"),
                &format!("{path}/{keyword}"),
            )?;
        }
    }
    for keyword in ["uniqueItems", "deprecated", "readOnly", "writeOnly"] {
        if object.get(keyword).is_some_and(|value| !value.is_boolean()) {
            return Err(GraphqlError::new(format!(
                "{path}/{keyword} must be a boolean"
            )));
        }
    }
    let _ = required_names(object, path)?;
    if let Some(dependencies) = object.get("dependentRequired") {
        let dependencies = dependencies.as_object().ok_or_else(|| {
            GraphqlError::new(format!("{path}/dependentRequired must be an object"))
        })?;
        for (property, names) in dependencies {
            let names = names.as_array().ok_or_else(|| {
                GraphqlError::new(format!(
                    "{path}/dependentRequired/{} must be an array",
                    pointer_escape(property)
                ))
            })?;
            let mut seen = BTreeSet::new();
            for (index, name) in names.iter().enumerate() {
                let name = name.as_str().ok_or_else(|| {
                    GraphqlError::new(format!(
                        "{path}/dependentRequired/{}/{index} must be a string",
                        pointer_escape(property)
                    ))
                })?;
                if !seen.insert(name) {
                    return Err(GraphqlError::new(format!(
                        "{path}/dependentRequired/{} repeats property {name:?}",
                        pointer_escape(property)
                    )));
                }
            }
        }
    }

    for keyword in schema_map_keywords() {
        if let Some(children) = object.get(*keyword).and_then(Value::as_object) {
            for (key, child) in children {
                validate_schema_keywords(
                    child,
                    &format!("{path}/{keyword}/{}", pointer_escape(key)),
                    depth + 1,
                )?;
            }
        }
    }
    for keyword in schema_array_keywords() {
        if let Some(children) = object.get(*keyword).and_then(Value::as_array) {
            for (index, child) in children.iter().enumerate() {
                validate_schema_keywords(child, &format!("{path}/{keyword}/{index}"), depth + 1)?;
            }
        }
    }
    for keyword in schema_single_keywords() {
        if let Some(child) = object.get(*keyword) {
            validate_schema_keywords(child, &format!("{path}/{keyword}"), depth + 1)?;
        }
    }
    Ok(())
}

fn validate_nonnegative_integer(value: &Value, path: &str) -> Result<(), GraphqlError> {
    if value.as_u64().is_some()
        || value
            .as_f64()
            .is_some_and(|number| number >= 0.0 && number.fract() == 0.0)
    {
        Ok(())
    } else {
        Err(GraphqlError::new(format!(
            "{path} must be a non-negative integer"
        )))
    }
}

fn required_names(
    object: &Map<String, Value>,
    path: &str,
) -> Result<BTreeSet<String>, GraphqlError> {
    let Some(value) = object.get("required") else {
        return Ok(BTreeSet::new());
    };
    let values = value
        .as_array()
        .ok_or_else(|| GraphqlError::new(format!("{path}/required must be an array")))?;
    let mut required = BTreeSet::new();
    for (index, value) in values.iter().enumerate() {
        let name = value.as_str().ok_or_else(|| {
            GraphqlError::new(format!("{path}/required/{index} must be a string"))
        })?;
        if !required.insert(name.to_owned()) {
            return Err(GraphqlError::new(format!(
                "{path}/required repeats property {name:?}"
            )));
        }
    }
    Ok(required)
}

fn schema_allows_null(
    schema: &Value,
    definitions: &Map<String, Value>,
    active_references: &mut BTreeSet<String>,
) -> Result<bool, GraphqlError> {
    let mut current = schema;
    loop {
        match current {
            Value::Bool(value) => return Ok(*value),
            Value::Object(object) => {
                if let Some(values) = finite_values(current, "nullable schema")? {
                    return Ok(values.iter().any(Value::is_null));
                }
                if let Some(reference) = pure_reference(object) {
                    let key = reference_key(reference).ok_or_else(|| {
                        GraphqlError::new(
                            "nullable schema reference is not a direct $defs reference",
                        )
                    })?;
                    if !active_references.insert(key.clone()) {
                        return Ok(true);
                    }
                    current = definitions.get(&key).ok_or_else(|| {
                        GraphqlError::new(format!(
                            "nullable schema targets missing definition {key:?}"
                        ))
                    })?;
                    continue;
                }
                let Some(kinds) = declared_types(object, "nullable schema")? else {
                    return Ok(true);
                };
                return Ok(kinds.contains(&JsonKind::Null));
            }
            _ => {
                return Err(GraphqlError::new(
                    "nullable schema must be an object or boolean JSON Schema",
                ));
            }
        }
    }
}

fn find_cycle_edge(
    objects: &BTreeMap<String, ObjectNames>,
    edges: &[InputEdge],
    relaxed: &BTreeSet<String>,
) -> Option<String> {
    let mut adjacency = objects
        .keys()
        .map(|path| (path.clone(), Vec::<&InputEdge>::new()))
        .collect::<BTreeMap<_, _>>();
    for edge in edges {
        if !relaxed.contains(&edge.field_path) {
            adjacency.entry(edge.source.clone()).or_default().push(edge);
        }
    }
    for outgoing in adjacency.values_mut() {
        outgoing.sort_by(|left, right| {
            left.target
                .cmp(&right.target)
                .then_with(|| left.field_path.cmp(&right.field_path))
        });
    }

    let mut colors = BTreeMap::<String, u8>::new();
    for start in objects.keys() {
        if colors.get(start).copied().unwrap_or(0) != 0 {
            continue;
        }
        colors.insert(start.clone(), 1);
        let mut stack = vec![(start.clone(), 0_usize)];
        while let Some((node, index)) = stack.last_mut() {
            let outgoing = adjacency.get(node).map(Vec::as_slice).unwrap_or_default();
            if *index >= outgoing.len() {
                colors.insert(node.clone(), 2);
                stack.pop();
                continue;
            }
            let edge = outgoing[*index];
            *index += 1;
            match colors.get(&edge.target).copied().unwrap_or(0) {
                0 => {
                    colors.insert(edge.target.clone(), 1);
                    stack.push((edge.target.clone(), 0));
                }
                1 => return Some(edge.field_path.clone()),
                _ => {}
            }
        }
    }
    None
}

fn has_unknown_assertion(object: &Map<String, Value>) -> bool {
    object
        .keys()
        .any(|key| !known_schema_keyword(key) && !is_annotation_keyword(key))
        || object.contains_key("additionalItems")
}

fn known_schema_keyword(keyword: &str) -> bool {
    matches!(
        keyword,
        "$ref"
            | "$defs"
            | "type"
            | "enum"
            | "const"
            | "allOf"
            | "anyOf"
            | "oneOf"
            | "not"
            | "if"
            | "then"
            | "else"
            | "properties"
            | "required"
            | "patternProperties"
            | "additionalProperties"
            | "dependentRequired"
            | "dependentSchemas"
            | "propertyNames"
            | "minProperties"
            | "maxProperties"
            | "items"
            | "prefixItems"
            | "additionalItems"
            | "contains"
            | "minContains"
            | "maxContains"
            | "uniqueItems"
            | "minItems"
            | "maxItems"
            | "unevaluatedItems"
            | "unevaluatedProperties"
            | "minimum"
            | "maximum"
            | "exclusiveMinimum"
            | "exclusiveMaximum"
            | "multipleOf"
            | "minLength"
            | "maxLength"
            | "pattern"
            | "format"
            | "contentEncoding"
            | "contentMediaType"
            | "contentSchema"
    )
}

fn is_annotation_keyword(keyword: &str) -> bool {
    keyword.starts_with("x-")
        || matches!(
            keyword,
            "$schema"
                | "$id"
                | "$anchor"
                | "$dynamicAnchor"
                | "$vocabulary"
                | "$comment"
                | "title"
                | "description"
                | "default"
                | "examples"
                | "deprecated"
                | "readOnly"
                | "writeOnly"
        )
}

fn schema_doc(schema: &Value) -> Result<Option<&str>, GraphqlError> {
    let Some(object) = schema.as_object() else {
        return Ok(None);
    };
    if let Some(description) = object.get("description") {
        return description
            .as_str()
            .map(Some)
            .ok_or_else(|| GraphqlError::new("JSON Schema description must be a string"));
    }
    if let Some(title) = object.get("title") {
        return title
            .as_str()
            .map(Some)
            .ok_or_else(|| GraphqlError::new("JSON Schema title must be a string"));
    }
    Ok(None)
}

fn validate_graphql_name(label: &str, value: &str) -> Result<(), GraphqlError> {
    if value.len() > MAX_GRAPHQL_NAME_BYTES || !is_graphql_name(value) || value.starts_with("__") {
        return Err(GraphqlError::new(format!(
            "GraphQL {label} {value:?} must be a non-introspection GraphQL Name no longer than \
             {MAX_GRAPHQL_NAME_BYTES} bytes"
        )));
    }
    Ok(())
}

fn checked_graphql_name(value: &str, label: &str) -> Result<String, GraphqlError> {
    validate_graphql_name(label, value)?;
    Ok(value.to_owned())
}

fn is_graphql_name(value: &str) -> bool {
    let mut characters = value.chars();
    characters
        .next()
        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn is_builtin_type(value: &str) -> bool {
    matches!(value, "Boolean" | "Float" | "ID" | "Int" | "String")
}

fn graphql_type_name(raw: &str, fallback: &str) -> String {
    let mut output = String::new();
    let mut capitalize = true;
    for character in raw.chars() {
        if character.is_ascii_alphanumeric() {
            if capitalize && character.is_ascii_alphabetic() {
                output.push(character.to_ascii_uppercase());
            } else {
                output.push(character);
            }
            capitalize = false;
        } else {
            capitalize = true;
        }
    }
    if output.is_empty() {
        output.push_str(fallback);
    }
    if output
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_digit())
    {
        output.insert(0, 'N');
    }
    if output.starts_with("__") {
        output.insert(0, 'G');
    }
    output
}

fn graphql_field_name(raw: &str, fallback: &str) -> String {
    if is_graphql_name(raw) && !raw.starts_with("__") {
        return raw.to_owned();
    }
    let type_name = graphql_type_name(raw, fallback);
    let mut characters = type_name.chars();
    let Some(first) = characters.next() else {
        return fallback.to_owned();
    };
    let mut output = first.to_ascii_lowercase().to_string();
    output.extend(characters);
    if output.starts_with("__") {
        output.insert(0, 'g');
    }
    output
}

fn normalize_prose(label: &str, value: &str) -> Result<String, GraphqlError> {
    let normalized = value.replace("\r\n", "\n").replace('\r', "\n");
    if normalized.trim().is_empty() {
        return Err(GraphqlError::new(format!(
            "GraphQL {label} must be caller-supplied non-whitespace text"
        )));
    }
    if normalized
        .chars()
        .any(|character| character.is_control() && character != '\n' && character != '\t')
    {
        return Err(GraphqlError::new(format!(
            "GraphQL {label} contains an unsupported control character"
        )));
    }
    Ok(normalized)
}

fn graphql_string(value: &str) -> String {
    serde_json::to_string(value).expect("serializing a Rust string to JSON cannot fail")
}

fn write_comment(output: &mut String, value: &str) {
    for line in value.lines() {
        if line.is_empty() {
            output.push_str("#\n");
        } else {
            writeln!(output, "# {line}").expect("writing GraphQL SDL to a String cannot fail");
        }
    }
}

fn ensure_depth(depth: usize, path: &str) -> Result<(), GraphqlError> {
    if depth > MAX_SCHEMA_DEPTH {
        Err(GraphqlError::new(format!(
            "GraphQL schema expression at {path} exceeds depth {MAX_SCHEMA_DEPTH}"
        )))
    } else {
        Ok(())
    }
}

fn ensure_value_size(value: &Value) -> Result<(), GraphqlError> {
    let mut stack = vec![(value, 0_usize)];
    while let Some((current, depth)) = stack.pop() {
        if depth > MAX_SCHEMA_DEPTH {
            return Err(GraphqlError::new(format!(
                "GraphQL JSON value exceeds depth {MAX_SCHEMA_DEPTH}"
            )));
        }
        match current {
            Value::Array(values) => {
                stack.extend(values.iter().map(|value| (value, depth + 1)));
            }
            Value::Object(object) => {
                stack.extend(object.values().map(|value| (value, depth + 1)));
            }
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
        }
    }
    let bytes = serde_json::to_vec(value).map_err(|error| {
        GraphqlError::new(format!("cannot inspect GraphQL JSON value: {error}"))
    })?;
    if bytes.len() > MAX_VALUE_JSON_BYTES {
        Err(GraphqlError::new(format!(
            "GraphQL JSON value exceeds the {MAX_VALUE_JSON_BYTES}-byte codec limit"
        )))
    } else {
        Ok(())
    }
}

fn finish_text(mut text: String) -> String {
    while text.ends_with("\n\n") {
        text.pop();
    }
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::purrdf::loss::{check_ledger_complete, check_ledger_sound};
    use proptest::prelude::*;
    use serde_json::json;

    fn compiled(schema: &Value) -> CompiledSchema {
        CompiledSchema {
            schema_json: format!(
                "{}\n",
                serde_json::to_string_pretty(schema).expect("fixture serializes")
            ),
            openapi_json: "{}\n".to_owned(),
            losses: LossLedger::new(),
        }
    }

    fn config() -> GraphqlConfig {
        GraphqlConfig::new(
            "ExampleSchema",
            "Caller-owned package documentation.",
            "Caller-owned GraphQL module documentation.",
            "JsonCarrier",
        )
        .expect("valid config")
    }

    fn sdl(package: &GraphqlPackage) -> &str {
        std::str::from_utf8(
            package
                .artifacts
                .get(GRAPHQL_SCHEMA_PATH)
                .expect("SDL artifact exists"),
        )
        .expect("SDL is UTF-8")
    }

    fn exact_schema() -> Value {
        json!({
            "$defs": {
                "Alias": { "$ref": "#/$defs/Person" },
                "Choice": { "enum": [true, { "state": "open" }] },
                "Int32": {
                    "type": "integer",
                    "minimum": -2_147_483_648_i64,
                    "maximum": 2_147_483_647_i64
                },
                "Person": {
                    "type": "object",
                    "title": "Person",
                    "description": "A caller-described person.\nSecond line.",
                    "additionalProperties": false,
                    "properties": {
                        "@id": {
                            "type": "string",
                            "description": "Stable caller identifier."
                        },
                        "ex:choice": { "$ref": "#/$defs/Choice" },
                        "ex:maybe": { "type": ["string", "null"] }
                    },
                    "required": ["@id", "ex:choice"]
                }
            }
        })
    }

    #[test]
    fn exact_projection_is_deterministic_lossless_and_reversible() {
        let schema = exact_schema();
        let first = emit_graphql(&compiled(&schema), &config()).expect("exact schema emits");
        let second = emit_graphql(&compiled(&schema), &config()).expect("exact schema emits");
        assert_eq!(first, second);
        assert!(first.losses.is_empty(), "{}", first.losses.render_json());
        assert_eq!(first.schema_name, "ExampleSchema");
        assert_eq!(first.artifacts.len(), 2);
        assert_eq!(
            first.names.definitions["Alias"],
            GraphqlDefinitionMap {
                output_type: "Person".to_owned(),
                input_type: "PersonInput".to_owned(),
            }
        );
        assert_eq!(first.names.definitions["Int32"].input_type, "Int");
        assert_eq!(first.names.fields["#/$defs/Person"]["@id"], "id");
        assert_eq!(
            first.names.fields["#/$defs/Person"]["ex:choice"],
            "exChoice"
        );

        let source = json!({
            "@id": "ex:person",
            "ex:choice": { "state": "open" },
            "ex:maybe": null
        });
        let encoded = first
            .encode_input("Person", &source)
            .expect("source value encodes");
        assert_eq!(
            encoded,
            json!({
                "id": "ex:person",
                "exChoice": "VALUE_1",
                "exMaybe": null
            })
        );
        assert_eq!(
            first
                .decode_output("Person", &encoded)
                .expect("GraphQL value decodes"),
            source
        );

        let schema_source = sdl(&first);
        assert!(schema_source.ends_with('\n'));
        assert!(schema_source.contains("scalar JsonCarrier"));
        assert!(schema_source.contains("enum Choice"));
        assert!(schema_source.contains("type Person {"));
        assert!(schema_source.contains("input PersonInput {"));
        assert!(schema_source.contains("id: String!"));
        assert!(schema_source.contains("exChoice: Choice!"));
        assert!(schema_source.contains("exMaybe: String"));
        assert!(!schema_source.contains("type Query"));
        assert!(!schema_source.contains("blackcatinformatics.ca"));
        assert!(!schema_source.to_ascii_lowercase().contains("gmeow"));

        let artifact: Value = serde_json::from_slice(
            first
                .artifacts
                .get(GRAPHQL_NAME_MAP_PATH)
                .expect("name map artifact exists"),
        )
        .expect("name map is JSON");
        assert_eq!(artifact["dialect"], GRAPHQL_DIALECT);
        assert_eq!(artifact["schema_name"], "ExampleSchema");
    }

    #[test]
    fn lossy_projection_exercises_the_entire_closed_profile() {
        let schema = json!({
            "$defs": {
                "ArrayLoss": {
                    "type": "array",
                    "items": { "type": "string" },
                    "minItems": 1,
                    "maxItems": 4,
                    "contains": { "const": "match" },
                    "minContains": 1,
                    "maxContains": 2,
                    "uniqueItems": true,
                    "unevaluatedItems": false
                },
                "ConditionalObject": {
                    "type": "object",
                    "properties": {
                        "plain": { "type": "string", "pattern": "^[A-Z]" },
                        "nullableRequired": { "type": ["string", "null"] }
                    },
                    "required": ["nullableRequired"],
                    "patternProperties": { "^dynamic": { "type": "string" } },
                    "propertyNames": { "pattern": "^[A-Za-z]" },
                    "minProperties": 1,
                    "maxProperties": 10,
                    "dependentRequired": { "plain": ["nullableRequired"] },
                    "dependentSchemas": { "plain": { "required": ["nullableRequired"] } },
                    "if": { "properties": { "plain": { "const": "A" } } },
                    "then": { "required": ["nullableRequired"] },
                    "else": { "maxProperties": 2 },
                    "unevaluatedProperties": false
                },
                "GenericInteger": { "type": "integer" },
                "IntPredicate": {
                    "type": "integer",
                    "minimum": -2_147_483_648_i64,
                    "maximum": 2_147_483_647_i64,
                    "multipleOf": 2
                },
                "Intersection": {
                    "allOf": [{ "type": "string" }, { "minLength": 1 }]
                },
                "Negated": { "type": "string", "not": { "const": "forbidden" } },
                "OneOf": { "oneOf": [{ "type": "string" }, { "type": "boolean" }] },
                "Tuple": {
                    "type": "array",
                    "prefixItems": [{ "type": "string" }, { "type": "boolean" }],
                    "items": false
                },
                "Union": { "anyOf": [{ "type": "string" }, { "type": "boolean" }] },
                "Unknown": { "type": "string", "unsupportedAssertion": true },
                "A": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": { "b": { "$ref": "#/$defs/B" } },
                    "required": ["b"]
                },
                "B": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": { "a": { "$ref": "#/$defs/A" } },
                    "required": ["a"]
                }
            }
        });
        let package = emit_graphql(&compiled(&schema), &config()).expect("lossy schema emits");
        let expected = [
            "additional-properties-validation-narrowed",
            "array-cardinality-validation-dropped",
            "array-contains-validation-dropped",
            "conditional-validation-dropped",
            "custom-scalar-validation-delegated",
            "dependency-validation-dropped",
            "integer-domain-validation-delegated",
            "intersection-validation-delegated",
            "keyword-validation-delegated",
            "negation-validation-delegated",
            "nullable-presence-validation-widened",
            "numeric-validation-dropped",
            "one-of-validation-delegated",
            "pattern-properties-validation-changed",
            "property-count-validation-dropped",
            "property-name-validation-changed",
            "recursive-input-nullability-relaxed",
            "singleton-list-coercion-widened",
            "string-validation-dropped",
            "tuple-array-validation-delegated",
            "unevaluated-validation-dropped",
            "union-validation-delegated",
            "unique-items-validation-dropped",
        ];
        check_ledger_sound(&package.losses, LOSS_FROM, GRAPHQL_DIALECT)
            .expect("all GraphQL losses are registered");
        check_ledger_complete(&package.losses, &expected)
            .expect("the complete closed GraphQL profile is exercised");
        assert!(package.losses.entries().iter().all(|entry| {
            entry
                .location
                .as_ref()
                .and_then(|location| location.subject.as_deref())
                .is_some_and(|subject| subject.starts_with("#/$defs/"))
        }));
        let source = sdl(&package);
        let a_required = source.contains("a: AInput!");
        let b_required = source.contains("b: BInput!");
        assert_ne!(a_required, b_required, "exactly one cycle edge is relaxed");
    }

    #[test]
    fn config_schema_and_codec_ambiguities_hard_fail() {
        for config in [
            GraphqlConfig::new("__Schema", "package", "module", "JsonCarrier"),
            GraphqlConfig::new("Schema", " ", "module", "JsonCarrier"),
            GraphqlConfig::new("Schema", "package", "module", "String"),
        ] {
            assert!(config.is_err());
        }

        let collision = json!({
            "$defs": {
                "a-b": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": { "value": { "type": "string" } }
                },
                "a_b": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": { "value": { "type": "string" } }
                }
            }
        });
        assert!(
            emit_graphql(&compiled(&collision), &config())
                .expect_err("type collision must fail")
                .to_string()
                .contains("collides")
        );

        let closed_required = json!({
            "$defs": {
                "Broken": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["missing"]
                }
            }
        });
        assert!(
            emit_graphql(&compiled(&closed_required), &config())
                .expect_err("unsatisfiable closed required field must fail")
                .to_string()
                .contains("no matching property schema")
        );

        let package = emit_graphql(&compiled(&exact_schema()), &config()).expect("exact emits");
        assert!(
            package
                .encode_input("Person", &json!({ "unknown": true }))
                .expect_err("unmapped source field must fail")
                .to_string()
                .contains("not representable")
        );
        assert!(
            package
                .decode_output("Choice", &json!("NOT_A_SYMBOL"))
                .expect_err("unmapped enum symbol must fail")
                .to_string()
                .contains("not mapped")
        );
    }

    #[test]
    fn inline_objects_and_lists_share_the_public_value_map() {
        let schema = json!({
            "$defs": {
                "Bag": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "ex:entries": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "additionalProperties": false,
                                "properties": {
                                    "ex:value": { "type": "string" }
                                },
                                "required": ["ex:value"]
                            }
                        }
                    },
                    "required": ["ex:entries"]
                }
            }
        });
        let package = emit_graphql(&compiled(&schema), &config()).expect("inline schema emits");
        check_ledger_complete(&package.losses, &["singleton-list-coercion-widened"])
            .expect("only GraphQL singleton-list coercion differs");
        let source = json!({
            "ex:entries": [
                { "ex:value": "first" },
                { "ex:value": "second" }
            ]
        });
        let encoded = package.encode_input("Bag", &source).expect("encodes");
        assert_eq!(
            encoded,
            json!({
                "exEntries": [
                    { "exValue": "first" },
                    { "exValue": "second" }
                ]
            })
        );
        assert_eq!(
            package.decode_output("Bag", &encoded).expect("decodes"),
            source
        );
        let source = sdl(&package);
        assert!(source.contains("type BagExEntriesItem {"));
        assert!(source.contains("input BagExEntriesItemInput {"));
        assert!(source.contains("exEntries: [BagExEntriesItemInput!]!"));
    }

    #[test]
    fn long_alias_catalog_is_iterative_and_alias_cycles_fail() {
        const ALIASES: usize = 8_192;
        let mut definitions = Map::new();
        for index in 0..ALIASES {
            let key = format!("Alias{index:04}");
            let next = if index + 1 == ALIASES {
                json!({ "type": "string" })
            } else {
                json!({ "$ref": format!("#/$defs/Alias{:04}", index + 1) })
            };
            definitions.insert(key, next);
        }
        let package = emit_graphql(&compiled(&json!({ "$defs": definitions })), &config())
            .expect("long acyclic alias catalog emits iteratively");
        assert!(package.losses.is_empty());
        assert_eq!(package.names.definitions.len(), ALIASES);
        assert_eq!(package.names.definitions["Alias0000"].input_type, "String");

        let cycle = json!({
            "$defs": {
                "A": { "$ref": "#/$defs/B" },
                "B": { "$ref": "#/$defs/A" }
            }
        });
        assert!(
            emit_graphql(&compiled(&cycle), &config())
                .expect_err("alias cycle must fail")
                .to_string()
                .contains("alias cycle")
        );
    }

    #[test]
    fn malformed_schema_values_and_field_collisions_fail_at_source_locations() {
        for (schema, expected) in [
            (
                json!({ "$defs": { "Broken": { "enum": [] } } }),
                "enum cannot be empty",
            ),
            (
                json!({ "$defs": { "Broken": { "$ref": "#/$defs/Missing" } } }),
                "targets missing $defs key",
            ),
            (
                json!({
                    "$defs": {
                        "Broken": {
                            "type": "object",
                            "additionalProperties": false,
                            "properties": {
                                "a-b": { "type": "string" },
                                "a:b": { "type": "string" }
                            }
                        }
                    }
                }),
                "collide on GraphQL field name",
            ),
        ] {
            let error = emit_graphql(&compiled(&schema), &config())
                .expect_err("malformed schema must fail");
            assert!(error.to_string().contains(expected), "{error}");
        }
    }

    #[test]
    fn schema_name_and_value_depth_limits_hard_fail() {
        let fallback_collision = json!({
            "$defs": {
                "JsonCarrier": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": { "value": { "type": "string" } }
                }
            }
        });
        assert!(
            emit_graphql(&compiled(&fallback_collision), &config())
                .expect_err("fallback scalar collision must fail")
                .to_string()
                .contains("collides")
        );

        let long_field = "a".repeat(MAX_GRAPHQL_NAME_BYTES + 1);
        let long_name_schema = json!({
            "$defs": {
                "Record": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": { (long_field): { "type": "string" } }
                }
            }
        });
        assert!(
            emit_graphql(&compiled(&long_name_schema), &config())
                .expect_err("overlong field name must fail")
                .to_string()
                .contains("no longer than")
        );

        let mut nested_schema = json!({ "type": "string" });
        for _ in 0..=MAX_SCHEMA_DEPTH {
            nested_schema = json!({ "type": "array", "items": nested_schema });
        }
        let error = emit_graphql(
            &compiled(&json!({ "$defs": { "Deep": nested_schema } })),
            &config(),
        )
        .expect_err("excessive schema depth must fail");
        assert!(
            error.to_string().contains("exceeds depth")
                || error.to_string().contains("recursion limit exceeded"),
            "{error}"
        );

        let package = emit_graphql(
            &compiled(&json!({ "$defs": { "Anything": true } })),
            &config(),
        )
        .expect("fallback package emits");
        let mut nested_value = Value::Null;
        for _ in 0..=MAX_SCHEMA_DEPTH {
            nested_value = Value::Array(vec![nested_value]);
        }
        let error = package
            .encode_input("Anything", &nested_value)
            .expect_err("excessive value depth must fail");
        assert!(error.to_string().contains("exceeds depth"), "{error}");
    }

    proptest! {
        #[test]
        fn arbitrary_source_field_names_round_trip(
            source_field in "[A-Za-z0-9:@/_-]{1,24}",
            source_value in "[A-Za-z0-9 ]{0,32}",
        ) {
            let schema = json!({
                "$defs": {
                    "Record": {
                        "type": "object",
                        "additionalProperties": false,
                        "properties": { (source_field.clone()): { "type": "string" } },
                        "required": [source_field.as_str()]
                    }
                }
            });
            let package = emit_graphql(&compiled(&schema), &config()).expect("generated field emits");
            prop_assert!(package.losses.is_empty(), "{}", package.losses.render_json());
            let source = json!({ (source_field): source_value });
            let encoded = package.encode_input("Record", &source).expect("encodes");
            let decoded = package.decode_output("Record", &encoded).expect("decodes");
            prop_assert_eq!(decoded, source);
        }
    }
}
