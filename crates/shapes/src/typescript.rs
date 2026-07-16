// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Deterministic [`CompiledSchema`] → TypeScript 7.0 declaration emitter.
//!
//! This is a filesystem-free code generator. It consumes the validated JSON
//! Schema `$defs` carried by [`CompiledSchema`] and returns one in-memory
//! declaration package. Package identity and all human-facing package/module
//! prose are caller configuration; PurRDF neither mints vocabulary nor embeds a
//! downstream brand.
//!
//! The emitted dialect is interpreted under TypeScript 7.0 with `strict` and
//! `exactOptionalPropertyTypes`. It uses type aliases because aliases preserve
//! unions, intersections, tuples, literal values, and recursive JSON object
//! compatibility without declaration merging. JSON Schema assertions outside
//! that structural assignability relation are recorded at their JSON Pointer
//! locations on [`TypeScriptPackage::losses`]. No output path uses `any`.
//!
//! An arbitrary TypeScript → JSON Schema reader is semantically undefined:
//! conditional/generic/ambient TypeScript declarations have no unique runtime
//! acceptance relation, and many distinct JSON Schemas intentionally project to
//! the same declaration. The authoritative reverse surface is therefore the
//! source [`CompiledSchema`], retained by the caller alongside the reversible
//! `$defs` key → exported type-name map.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Write as _};

use ::purrdf::RdfLocation;
use ::purrdf::loss::{LossEntry, LossLedger};
use serde_json::{Map, Value};

use crate::json_schema::CompiledSchema;
use crate::schema_catalog::{
    CompiledSchemaCatalog, definition_path, pointer_escape, reference_key, schema_array_keywords,
    schema_map_keywords, schema_single_keywords,
};

/// Fixed loss-registry target and compiler-dialect identifier.
pub const TYPESCRIPT_DIALECT: &str = "typescript-7.0";

/// Relative path of the generated declaration artifact.
pub const TYPESCRIPT_DECLARATION_PATH: &str = "index.d.ts";

const LOSS_FROM: &str = "json-schema";
const LOSS_CONTEXT: &str = "typescript-emitter";
const MAX_SCHEMA_JSON_BYTES: usize = 16 * 1024 * 1024;
const MAX_DECLARATION_BYTES: usize = 16 * 1024 * 1024;
const MAX_DEFINITIONS: usize = 65_536;
const MAX_SCHEMA_DEPTH: usize = 128;
const MAX_TUPLE_EXPANSION: usize = 32;
const RESERVED_TYPE_NAMES: &[&str] = &[
    "Any",
    "Array",
    "BigInt",
    "Boolean",
    "Date",
    "Function",
    "JsonObject",
    "JsonPrimitive",
    "JsonValue",
    "Map",
    "Never",
    "Null",
    "Number",
    "Object",
    "Promise",
    "ReadonlyArray",
    "Record",
    "Set",
    "String",
    "Symbol",
    "Undefined",
    "Unknown",
];

/// Caller-owned identity and prose for a generated TypeScript declaration
/// package.
///
/// There is intentionally no [`Default`] implementation: package identity and
/// documentation must be supplied by the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeScriptConfig {
    package_name: String,
    package_docstring: String,
    module_docstring: String,
}

impl TypeScriptConfig {
    /// Validate and construct TypeScript emitter configuration.
    ///
    /// `package_name` accepts a conservative npm-compatible lowercase package
    /// name, either unscoped (`example-types`) or scoped
    /// (`@example/schema-types`). Both prose values must contain non-whitespace
    /// caller text. CRLF and CR line endings are canonicalized to LF.
    ///
    /// # Errors
    ///
    /// Returns [`TypeScriptError`] for an invalid package name, blank prose, or
    /// unsupported control characters.
    pub fn new(
        package_name: impl Into<String>,
        package_docstring: impl Into<String>,
        module_docstring: impl Into<String>,
    ) -> Result<Self, TypeScriptError> {
        let package_name = package_name.into();
        if !is_package_name(&package_name) {
            return Err(TypeScriptError::new(format!(
                "TypeScript package name {package_name:?} is not a conservative lowercase npm \
                 package name"
            )));
        }
        let package_docstring = package_docstring.into();
        let module_docstring = module_docstring.into();
        let package_docstring = normalize_prose("package docstring", &package_docstring)?;
        let module_docstring = normalize_prose("module docstring", &module_docstring)?;
        Ok(Self {
            package_name,
            package_docstring,
            module_docstring,
        })
    }

    /// Caller-supplied package identifier.
    #[must_use]
    pub fn package_name(&self) -> &str {
        &self.package_name
    }

    /// Caller-supplied package documentation.
    #[must_use]
    pub fn package_docstring(&self) -> &str {
        &self.package_docstring
    }

    /// Caller-supplied declaration-module documentation.
    #[must_use]
    pub fn module_docstring(&self) -> &str {
        &self.module_docstring
    }
}

/// Deterministic generated declaration package and its projection losses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeScriptPackage {
    /// Caller-owned package identifier copied from [`TypeScriptConfig`].
    pub package_name: String,
    /// Relative package paths → exact file bytes, sorted by path.
    pub artifacts: BTreeMap<String, Vec<u8>>,
    /// Source `$defs` key → exported TypeScript type name, sorted by key.
    pub type_names: BTreeMap<String, String>,
    /// JSON Schema assertions not represented exactly by TypeScript 7.0
    /// assignability.
    pub losses: LossLedger,
}

/// A malformed TypeScript configuration, input schema, or declaration graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeScriptError {
    message: String,
}

impl TypeScriptError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for TypeScriptError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for TypeScriptError {}

/// Emit deterministic TypeScript 7.0 declarations from one compiled
/// SHACL-derived JSON Schema.
///
/// Source-stage losses remain on [`CompiledSchema::losses`]. The returned
/// ledger describes only the `json-schema` → `typescript-7.0` projection.
///
/// # Example
///
/// ```
/// use purrdf::loss::LossLedger;
/// use purrdf_shapes::json_schema::CompiledSchema;
/// use purrdf_shapes::{
///     TYPESCRIPT_DECLARATION_PATH, TypeScriptConfig, emit_typescript,
/// };
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let compiled = CompiledSchema {
///     schema_json: r#"{
///       "$defs": {
///         "Person": {
///           "type": "object",
///           "properties": { "name": { "type": "string" } },
///           "required": ["name"]
///         }
///       }
///     }"#.to_owned(),
///     openapi_json: "{}\n".to_owned(),
///     losses: LossLedger::new(),
/// };
/// let config = TypeScriptConfig::new(
///     "example-schema-types",
///     "Schema types published by the caller.",
///     "Declarations generated from the caller's compiled schema.",
/// )?;
/// let package = emit_typescript(&compiled, &config)?;
/// let declaration = std::str::from_utf8(
///     &package.artifacts[TYPESCRIPT_DECLARATION_PATH],
/// )?;
///
/// assert!(declaration.contains("export type Person"));
/// assert_eq!(package.type_names["Person"], "Person");
/// assert!(package.losses.is_empty());
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// Returns [`TypeScriptError`] when the schema is malformed or too large, lacks
/// a valid `$defs` catalog, contains an open/dangling reference, uses malformed
/// keyword values, or has source keys that collide on a reserved/exported
/// TypeScript name.
pub fn emit_typescript(
    compiled: &CompiledSchema,
    config: &TypeScriptConfig,
) -> Result<TypeScriptPackage, TypeScriptError> {
    if compiled.schema_json.len() > MAX_SCHEMA_JSON_BYTES {
        return Err(TypeScriptError::new(format!(
            "CompiledSchema.schema_json exceeds the {MAX_SCHEMA_JSON_BYTES}-byte TypeScript \
             emitter input limit"
        )));
    }
    let catalog = CompiledSchemaCatalog::parse(compiled)
        .map_err(|error| TypeScriptError::new(error.to_string()))?;
    let definitions = catalog.definitions();
    if definitions.len() > MAX_DEFINITIONS {
        return Err(TypeScriptError::new(format!(
            "CompiledSchema contains {} definitions; TypeScript emission is limited to \
             {MAX_DEFINITIONS}",
            definitions.len()
        )));
    }

    let type_names = definition_names(definitions)?;
    validate_unguarded_reference_cycles(definitions)?;
    let (declaration, losses) = {
        let mut renderer = Renderer::new(&type_names);
        for (key, definition) in definitions {
            renderer.audit_schema(definition, &definition_path(key), 0)?;
        }
        let declaration = renderer.render_document(config, definitions)?;
        (declaration, renderer.ledger)
    };
    if declaration.len() > MAX_DECLARATION_BYTES {
        return Err(TypeScriptError::new(format!(
            "generated TypeScript declaration exceeds the {MAX_DECLARATION_BYTES}-byte output \
             limit"
        )));
    }

    let mut artifacts = BTreeMap::new();
    artifacts.insert(
        TYPESCRIPT_DECLARATION_PATH.to_owned(),
        declaration.into_bytes(),
    );
    Ok(TypeScriptPackage {
        package_name: config.package_name.clone(),
        artifacts,
        type_names,
        losses,
    })
}

fn validate_unguarded_reference_cycles(
    definitions: &Map<String, Value>,
) -> Result<(), TypeScriptError> {
    let graph = definitions
        .iter()
        .map(|(key, definition)| {
            let references = match unguarded_relation(definition) {
                UnguardedRelation::Never | UnguardedRelation::Universal => BTreeSet::new(),
                UnguardedRelation::Specific(references) => references,
            };
            (key.clone(), references)
        })
        .collect::<BTreeMap<_, _>>();
    let mut complete = BTreeSet::new();
    for key in definitions.keys() {
        if complete.contains(key) {
            continue;
        }
        let mut active = Vec::new();
        let mut active_positions = BTreeMap::new();
        let mut stack = vec![(key.clone(), false)];
        while let Some((current, exiting)) = stack.pop() {
            if exiting {
                let removed_position = active_positions.remove(&current);
                debug_assert_eq!(removed_position, Some(active.len().saturating_sub(1)));
                let removed = active.pop();
                debug_assert_eq!(removed.as_deref(), Some(current.as_str()));
                complete.insert(current);
                continue;
            }
            if complete.contains(&current) {
                continue;
            }
            if let Some(start) = active_positions.get(&current).copied() {
                let mut cycle = active[start..].to_vec();
                cycle.push(current);
                return Err(TypeScriptError::new(format!(
                    "CompiledSchema contains an unguarded recursive TypeScript alias cycle: {}",
                    cycle.join(" -> ")
                )));
            }

            active_positions.insert(current.clone(), active.len());
            active.push(current.clone());
            stack.push((current.clone(), true));
            if let Some(references) = graph.get(&current) {
                for reference in references.iter().rev() {
                    stack.push((reference.clone(), false));
                }
            }
        }
    }
    Ok(())
}

enum UnguardedRelation {
    Never,
    Universal,
    Specific(BTreeSet<String>),
}

fn unguarded_relation(schema: &Value) -> UnguardedRelation {
    let Value::Object(object) = schema else {
        return if schema == &Value::Bool(false) {
            UnguardedRelation::Never
        } else {
            UnguardedRelation::Universal
        };
    };

    let mut conjuncts = Vec::new();
    if object.contains_key("type") || has_object_shape(object) || has_array_shape(object) {
        conjuncts.push(UnguardedRelation::Specific(BTreeSet::new()));
    }
    if let Some(reference) = object
        .get("$ref")
        .and_then(Value::as_str)
        .and_then(reference_key)
    {
        conjuncts.push(UnguardedRelation::Specific(BTreeSet::from([reference])));
    }
    if let Some(values) = object.get("enum").and_then(Value::as_array) {
        if values.is_empty() {
            conjuncts.push(UnguardedRelation::Never);
        } else {
            conjuncts.push(UnguardedRelation::Specific(BTreeSet::new()));
        }
    }
    if object.contains_key("const") {
        conjuncts.push(UnguardedRelation::Specific(BTreeSet::new()));
    }
    if let Some(branches) = object.get("allOf").and_then(Value::as_array) {
        conjuncts.push(join_unguarded_intersection(
            branches.iter().map(unguarded_relation),
        ));
    }
    for keyword in ["anyOf", "oneOf"] {
        if let Some(branches) = object.get(keyword).and_then(Value::as_array) {
            conjuncts.push(join_unguarded_union(
                branches.iter().map(unguarded_relation),
            ));
        }
    }
    join_unguarded_intersection(conjuncts)
}

fn join_unguarded_union(
    relations: impl IntoIterator<Item = UnguardedRelation>,
) -> UnguardedRelation {
    let mut references = BTreeSet::new();
    let mut has_specific = false;
    for relation in relations {
        match relation {
            UnguardedRelation::Never => {}
            UnguardedRelation::Universal => return UnguardedRelation::Universal,
            UnguardedRelation::Specific(current) => {
                has_specific = true;
                references.extend(current);
            }
        }
    }
    if has_specific {
        UnguardedRelation::Specific(references)
    } else {
        UnguardedRelation::Never
    }
}

fn join_unguarded_intersection(
    relations: impl IntoIterator<Item = UnguardedRelation>,
) -> UnguardedRelation {
    let mut references = BTreeSet::new();
    let mut has_specific = false;
    for relation in relations {
        match relation {
            UnguardedRelation::Never => return UnguardedRelation::Never,
            UnguardedRelation::Universal => {}
            UnguardedRelation::Specific(current) => {
                has_specific = true;
                references.extend(current);
            }
        }
    }
    if has_specific {
        UnguardedRelation::Specific(references)
    } else {
        UnguardedRelation::Universal
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum JsonKind {
    Null,
    Boolean,
    Number,
    String,
    Array,
    Object,
}

struct Renderer<'a> {
    names: &'a BTreeMap<String, String>,
    ledger: LossLedger,
    recorded_losses: BTreeSet<(String, String)>,
}

impl<'a> Renderer<'a> {
    fn new(names: &'a BTreeMap<String, String>) -> Self {
        Self {
            names,
            ledger: LossLedger::new(),
            recorded_losses: BTreeSet::new(),
        }
    }

    fn render_document(
        &mut self,
        config: &TypeScriptConfig,
        definitions: &Map<String, Value>,
    ) -> Result<String, TypeScriptError> {
        let mut output = String::new();
        let header = format!(
            "{}\n\n{}\n\nPackage: {}",
            config.package_docstring, config.module_docstring, config.package_name
        );
        write_doc_comment(&mut output, 0, &header, Some("@packageDocumentation"));
        output.push_str("export type JsonPrimitive = string | number | boolean | null;\n");
        output.push_str("export type JsonObject = { readonly [key: string]: JsonValue };\n");
        output.push_str(
            "export type JsonValue = JsonPrimitive | JsonObject | readonly JsonValue[];\n\n",
        );

        for (key, definition) in definitions {
            let name = self
                .names
                .get(key)
                .expect("definition_names covers every validated definition");
            if let Some(description) = schema_doc(definition)? {
                write_doc_comment(&mut output, 0, description, None);
            }
            let expression = self.render_schema(definition, &definition_path(key), 0)?;
            writeln!(output, "export type {name} = {expression};\n")
                .expect("writing TypeScript to a String cannot fail");
        }
        Ok(finish_text(output))
    }

    fn render_schema(
        &mut self,
        schema: &Value,
        path: &str,
        depth: usize,
    ) -> Result<String, TypeScriptError> {
        ensure_depth(depth, path)?;
        match schema {
            Value::Bool(true) => Ok("JsonValue".to_owned()),
            Value::Bool(false) => Ok("never".to_owned()),
            Value::Object(object) => {
                let mut conjuncts = Vec::new();
                if let Some(carrier) = self.render_carrier_relation(object, path, depth)? {
                    conjuncts.push(carrier);
                }
                if let Some(reference) = object.get("$ref") {
                    let reference = reference.as_str().ok_or_else(|| {
                        TypeScriptError::new(format!("{path}/$ref must be a string"))
                    })?;
                    let key = reference_key(reference).ok_or_else(|| {
                        TypeScriptError::new(format!(
                            "{path}/$ref is not a direct #/$defs reference"
                        ))
                    })?;
                    let name = self.names.get(&key).ok_or_else(|| {
                        TypeScriptError::new(format!(
                            "{path}/$ref targets missing $defs key {key:?}"
                        ))
                    })?;
                    conjuncts.push(name.clone());
                }
                if let Some(values) = object.get("enum").and_then(Value::as_array) {
                    let mut variants = Vec::with_capacity(values.len());
                    for (index, value) in values.iter().enumerate() {
                        variants.push(Self::render_literal(
                            value,
                            &format!("{path}/enum/{index}"),
                            depth + 1,
                        )?);
                    }
                    conjuncts.push(join_union(variants));
                }
                if let Some(value) = object.get("const") {
                    conjuncts.push(Self::render_literal(
                        value,
                        &format!("{path}/const"),
                        depth + 1,
                    )?);
                }
                if let Some(branches) = object.get("allOf").and_then(Value::as_array) {
                    let mut expressions = Vec::with_capacity(branches.len());
                    for (index, branch) in branches.iter().enumerate() {
                        expressions.push(self.render_schema(
                            branch,
                            &format!("{path}/allOf/{index}"),
                            depth + 1,
                        )?);
                    }
                    conjuncts.push(join_intersection(expressions));
                }
                for keyword in ["anyOf", "oneOf"] {
                    if let Some(branches) = object.get(keyword).and_then(Value::as_array) {
                        let mut expressions = Vec::with_capacity(branches.len());
                        for (index, branch) in branches.iter().enumerate() {
                            expressions.push(self.render_schema(
                                branch,
                                &format!("{path}/{keyword}/{index}"),
                                depth + 1,
                            )?);
                        }
                        conjuncts.push(join_union(expressions));
                    }
                }
                Ok(join_intersection(conjuncts))
            }
            _ => Err(TypeScriptError::new(format!(
                "{path} must be a JSON Schema object or boolean"
            ))),
        }
    }

    fn render_carrier_relation(
        &mut self,
        object: &Map<String, Value>,
        path: &str,
        depth: usize,
    ) -> Result<Option<String>, TypeScriptError> {
        let explicit_type = object.contains_key("type");
        let object_shape = has_object_shape(object);
        let array_shape = has_array_shape(object);
        if !explicit_type && !object_shape && !array_shape {
            return Ok(None);
        }

        let (kinds, _, _) = declared_kinds(object, path)?;
        let mut variants = Vec::with_capacity(kinds.len());
        for kind in kinds {
            let expression = match kind {
                JsonKind::Null => "null".to_owned(),
                JsonKind::Boolean => "boolean".to_owned(),
                JsonKind::Number => "number".to_owned(),
                JsonKind::String => "string".to_owned(),
                JsonKind::Array if explicit_type || array_shape => {
                    self.render_array(object, path, depth + 1)?
                }
                JsonKind::Array => "readonly JsonValue[]".to_owned(),
                JsonKind::Object if explicit_type || object_shape => {
                    self.render_object(object, path, depth + 1)?
                }
                JsonKind::Object => "JsonObject".to_owned(),
            };
            variants.push(expression);
        }
        Ok(Some(join_union(variants)))
    }

    fn render_object(
        &mut self,
        object: &Map<String, Value>,
        path: &str,
        depth: usize,
    ) -> Result<String, TypeScriptError> {
        ensure_depth(depth, path)?;
        let empty = Map::new();
        let properties = object
            .get("properties")
            .and_then(Value::as_object)
            .unwrap_or(&empty);
        let required = required_names(object, path)?;
        let pattern_properties = object
            .get("patternProperties")
            .and_then(Value::as_object)
            .unwrap_or(&empty);

        let mut field_types = BTreeMap::new();
        for (property, schema) in properties {
            let property_path = format!("{path}/properties/{}", pointer_escape(property));
            field_types.insert(
                property.clone(),
                self.render_schema(schema, &property_path, depth + 1)?,
            );
        }
        let property_names: BTreeSet<String> = properties.keys().cloned().collect();
        for property in required.difference(&property_names) {
            field_types.insert(
                property.clone(),
                self.render_required_only_property(object, pattern_properties, path, depth + 1)?,
            );
        }

        let mut output = String::from("{\n");
        let field_indent = indentation(depth);
        for (property, field_type) in &field_types {
            if let Some(schema) = properties.get(property)
                && let Some(description) = schema_doc(schema)?
            {
                write_doc_comment(&mut output, depth, description, None);
            }
            let optional = if required.contains(property) { "" } else { "?" };
            writeln!(
                output,
                "{field_indent}readonly {}{optional}: {field_type};",
                json_string(property)
            )
            .expect("writing TypeScript to a String cannot fail");
        }

        let mut index_types = Vec::new();
        for (pattern, schema) in pattern_properties {
            index_types.push(self.render_schema(
                schema,
                &format!("{path}/patternProperties/{}", pointer_escape(pattern)),
                depth + 1,
            )?);
        }
        match object.get("additionalProperties") {
            None | Some(Value::Bool(true)) => index_types.push("JsonValue".to_owned()),
            Some(Value::Bool(false)) => {}
            Some(schema @ Value::Object(_)) => index_types.push(self.render_schema(
                schema,
                &format!("{path}/additionalProperties"),
                depth + 1,
            )?),
            Some(_) => unreachable!("schema catalog validates additionalProperties schemas"),
        }
        if !index_types.is_empty() {
            index_types.extend(field_types.values().cloned());
            writeln!(
                output,
                "{field_indent}readonly [key: string]: {};",
                join_union(index_types)
            )
            .expect("writing TypeScript to a String cannot fail");
        } else if field_types.is_empty() {
            writeln!(output, "{field_indent}readonly [key: string]: never;")
                .expect("writing TypeScript to a String cannot fail");
        }
        write!(output, "{}}}", indentation(depth.saturating_sub(1)))
            .expect("writing TypeScript to a String cannot fail");
        Ok(output)
    }

    fn render_required_only_property(
        &mut self,
        object: &Map<String, Value>,
        patterns: &Map<String, Value>,
        path: &str,
        depth: usize,
    ) -> Result<String, TypeScriptError> {
        let mut variants = Vec::new();
        for (pattern, schema) in patterns {
            variants.push(self.render_schema(
                schema,
                &format!("{path}/patternProperties/{}", pointer_escape(pattern)),
                depth + 1,
            )?);
        }
        match object.get("additionalProperties") {
            None | Some(Value::Bool(true)) => variants.push("JsonValue".to_owned()),
            Some(Value::Bool(false)) => {}
            Some(schema @ Value::Object(_)) => variants.push(self.render_schema(
                schema,
                &format!("{path}/additionalProperties"),
                depth + 1,
            )?),
            Some(_) => unreachable!("schema catalog validates additionalProperties schemas"),
        }
        if variants.is_empty() {
            Ok("never".to_owned())
        } else {
            Ok(join_union(variants))
        }
    }

    fn render_array(
        &mut self,
        object: &Map<String, Value>,
        path: &str,
        depth: usize,
    ) -> Result<String, TypeScriptError> {
        ensure_depth(depth, path)?;
        let mut prefix = Vec::new();
        if let Some(items) = object.get("prefixItems").and_then(Value::as_array) {
            prefix.reserve(items.len());
            for (index, item) in items.iter().enumerate() {
                prefix.push(self.render_schema(
                    item,
                    &format!("{path}/prefixItems/{index}"),
                    depth + 1,
                )?);
            }
        }
        let mut rest = match object.get("items") {
            None | Some(Value::Bool(true)) => "JsonValue".to_owned(),
            Some(Value::Bool(false)) => "never".to_owned(),
            Some(schema @ Value::Object(_)) => {
                self.render_schema(schema, &format!("{path}/items"), depth + 1)?
            }
            Some(_) => unreachable!("schema catalog validates items schemas"),
        };
        let mut min_items = array_bound(object, "minItems", path)?.unwrap_or(0);
        let mut max_items = array_bound(object, "maxItems", path)?;
        let has_explicit_max = max_items.is_some();

        if rest == "never" {
            max_items = Some(max_items.map_or(prefix.len(), |value| value.min(prefix.len())));
        }
        if prefix.len().saturating_add(1) > MAX_TUPLE_EXPANSION {
            self.record(
                "tuple-array-validation-widened",
                &format!("{path}/prefixItems"),
                "The prefixItems tuple exceeds the fixed TypeScript declaration expansion budget",
            );
            let mut common = prefix;
            if rest != "never" {
                common.push(rest);
            }
            rest = join_union(common);
            prefix = Vec::new();
            if !has_explicit_max {
                max_items = None;
            }
        }
        if min_items > MAX_TUPLE_EXPANSION {
            self.record(
                "array-cardinality-validation-widened",
                &format!("{path}/minItems"),
                "minItems exceeds the fixed TypeScript tuple expansion budget",
            );
            min_items = 0;
        }
        if max_items.is_some_and(|maximum| maximum > MAX_TUPLE_EXPANSION) {
            self.record(
                "array-cardinality-validation-widened",
                &format!("{path}/maxItems"),
                "maxItems exceeds the fixed TypeScript tuple expansion budget",
            );
            max_items = None;
        }
        if let Some(maximum) = max_items
            && maximum >= min_items
            && maximum - min_items + 1 > MAX_TUPLE_EXPANSION
        {
            self.record(
                "array-cardinality-validation-widened",
                &format!("{path}/maxItems"),
                "the minItems/maxItems interval exceeds the fixed TypeScript tuple union budget",
            );
            max_items = None;
        }

        render_array_relation(&prefix, &rest, min_items, max_items)
    }

    fn render_literal(value: &Value, path: &str, depth: usize) -> Result<String, TypeScriptError> {
        ensure_depth(depth, path)?;
        match value {
            Value::Null => Ok("null".to_owned()),
            Value::Bool(value) => Ok(value.to_string()),
            Value::Number(value) => Ok(value.to_string()),
            Value::String(value) => Ok(json_string(value)),
            Value::Array(values) => {
                let mut items = Vec::with_capacity(values.len());
                for (index, value) in values.iter().enumerate() {
                    items.push(Self::render_literal(
                        value,
                        &format!("{path}/{index}"),
                        depth + 1,
                    )?);
                }
                Ok(render_tuple(&items))
            }
            Value::Object(object) => {
                let mut output = String::from("{\n");
                let field_indent = indentation(depth + 1);
                for (key, value) in object {
                    let value = Self::render_literal(
                        value,
                        &format!("{path}/{}", pointer_escape(key)),
                        depth + 1,
                    )?;
                    writeln!(
                        output,
                        "{field_indent}readonly {}: {value};",
                        json_string(key)
                    )
                    .expect("writing TypeScript to a String cannot fail");
                }
                write!(output, "{}}}", indentation(depth))
                    .expect("writing TypeScript to a String cannot fail");
                Ok(output)
            }
        }
    }

    fn audit_schema(
        &mut self,
        schema: &Value,
        path: &str,
        depth: usize,
    ) -> Result<(), TypeScriptError> {
        ensure_depth(depth, path)?;
        let Value::Object(object) = schema else {
            return if schema.is_boolean() {
                Ok(())
            } else {
                Err(TypeScriptError::new(format!(
                    "{path} must be a JSON Schema object or boolean"
                )))
            };
        };
        validate_keyword_values(object, path)?;
        let (kinds, has_integer, has_number) = declared_kinds(object, path)?;
        let has_kind = |kind| kinds.contains(&kind);

        if has_integer && !has_number {
            self.record(
                "integer-validation-widened",
                &format!("{path}/type"),
                "TypeScript number cannot enforce the JSON integer subset",
            );
        }
        if has_kind(JsonKind::Object) {
            let properties = object
                .get("properties")
                .and_then(Value::as_object)
                .map_or(0, Map::len);
            let required = required_names(object, path)?.len();
            if matches!(object.get("additionalProperties"), Some(Value::Bool(false)))
                && properties + required > 0
            {
                self.record(
                    "additional-properties-validation-widened",
                    &format!("{path}/additionalProperties"),
                    "TypeScript structural compatibility cannot close an object with named fields",
                );
            } else if matches!(object.get("additionalProperties"), Some(Value::Object(_)))
                && properties + required > 0
            {
                self.record(
                    "additional-properties-validation-widened",
                    &format!("{path}/additionalProperties"),
                    "a TypeScript string index signature also constrains named fields",
                );
            }
            if let Some(patterns) = object.get("patternProperties").and_then(Value::as_object) {
                for pattern in patterns.keys() {
                    self.record(
                        "pattern-properties-validation-dropped",
                        &format!("{path}/patternProperties/{}", pointer_escape(pattern)),
                        "TypeScript cannot select arbitrary string keys with a JSON Schema regex",
                    );
                }
            }
            for keyword in ["minProperties", "maxProperties"] {
                if object.contains_key(keyword) {
                    self.record(
                        "property-count-validation-dropped",
                        &format!("{path}/{keyword}"),
                        "TypeScript cannot count runtime object properties",
                    );
                }
            }
            if object.contains_key("propertyNames") {
                self.record(
                    "property-name-validation-dropped",
                    &format!("{path}/propertyNames"),
                    "TypeScript cannot apply a schema to every runtime property name",
                );
            }
            for keyword in ["dependentRequired", "dependentSchemas"] {
                if object.contains_key(keyword) {
                    self.record(
                        "dependency-validation-dropped",
                        &format!("{path}/{keyword}"),
                        "TypeScript cannot enforce a general cross-property dependency",
                    );
                }
            }
        }
        if has_kind(JsonKind::Array) {
            if object.contains_key("contains") {
                for keyword in ["contains", "minContains", "maxContains"] {
                    if object.contains_key(keyword) {
                        self.record(
                            "array-contains-validation-dropped",
                            &format!("{path}/{keyword}"),
                            "TypeScript cannot quantify elements matching an array predicate",
                        );
                    }
                }
            }
            if object.get("uniqueItems") == Some(&Value::Bool(true)) {
                self.record(
                    "unique-items-validation-dropped",
                    &format!("{path}/uniqueItems"),
                    "TypeScript cannot require pairwise-distinct runtime array values",
                );
            }
        }
        if has_kind(JsonKind::Number) {
            for keyword in [
                "minimum",
                "maximum",
                "exclusiveMinimum",
                "exclusiveMaximum",
                "multipleOf",
            ] {
                if object.contains_key(keyword) {
                    self.record(
                        "numeric-validation-dropped",
                        &format!("{path}/{keyword}"),
                        "TypeScript number types do not express runtime numeric predicates",
                    );
                }
            }
        }
        if has_kind(JsonKind::String) {
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
                        "TypeScript string types do not express this runtime string predicate",
                    );
                }
            }
        }
        if object.contains_key("oneOf") {
            self.record(
                "one-of-validation-widened",
                &format!("{path}/oneOf"),
                "TypeScript unions cannot enforce exactly-one matching branch",
            );
        }
        if object.contains_key("not") {
            self.record(
                "negation-validation-dropped",
                &format!("{path}/not"),
                "TypeScript has no general JSON value-set complement",
            );
        }
        let has_active_conditional = object.contains_key("if")
            && (object.contains_key("then") || object.contains_key("else"));
        if has_active_conditional {
            for keyword in ["if", "then", "else"] {
                if object.contains_key(keyword) {
                    self.record(
                        "conditional-validation-dropped",
                        &format!("{path}/{keyword}"),
                        "TypeScript cannot apply a schema branch conditionally to runtime content",
                    );
                }
            }
        }
        for keyword in ["unevaluatedItems", "unevaluatedProperties"] {
            if object.contains_key(keyword) {
                self.record(
                    "unevaluated-validation-dropped",
                    &format!("{path}/{keyword}"),
                    "TypeScript does not expose JSON Schema applicator evaluation state",
                );
            }
        }
        if let Some(value) = object.get("const") {
            self.audit_literal(value, &format!("{path}/const"), depth + 1)?;
        }
        if let Some(values) = object.get("enum").and_then(Value::as_array) {
            for (index, value) in values.iter().enumerate() {
                self.audit_literal(value, &format!("{path}/enum/{index}"), depth + 1)?;
            }
        }
        if object.contains_key("additionalItems") {
            self.record(
                "keyword-validation-dropped",
                &format!("{path}/additionalItems"),
                "additionalItems is outside the closed draft 2020-12 TypeScript capability table",
            );
        }
        for key in object.keys() {
            if !known_schema_keyword(key) && !is_annotation_keyword(key) {
                self.record(
                    "keyword-validation-dropped",
                    &format!("{path}/{}", pointer_escape(key)),
                    &format!(
                        "JSON Schema assertion keyword {key:?} is outside the closed TypeScript \
                         7.0 capability table"
                    ),
                );
            }
        }

        for keyword in schema_map_keywords() {
            if *keyword == "$defs" {
                continue;
            }
            if let Some(children) = object.get(*keyword).and_then(Value::as_object) {
                for (key, child) in children {
                    self.audit_schema(
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
                    self.audit_schema(child, &format!("{path}/{keyword}/{index}"), depth + 1)?;
                }
            }
        }
        for keyword in schema_single_keywords() {
            if matches!(*keyword, "if" | "then" | "else") && !has_active_conditional {
                continue;
            }
            if let Some(child) = object.get(*keyword) {
                self.audit_schema(child, &format!("{path}/{keyword}"), depth + 1)?;
            }
        }
        Ok(())
    }

    fn audit_literal(
        &mut self,
        value: &Value,
        path: &str,
        depth: usize,
    ) -> Result<(), TypeScriptError> {
        ensure_depth(depth, path)?;
        match value {
            Value::Number(number) if integer_exceeds_typescript_exact_range(number) => {
                self.record(
                    "numeric-validation-dropped",
                    path,
                    "TypeScript number literals cannot preserve this exact JSON integer",
                );
            }
            Value::Array(values) => {
                for (index, value) in values.iter().enumerate() {
                    self.audit_literal(value, &format!("{path}/{index}"), depth + 1)?;
                }
            }
            Value::Object(object) => {
                self.record(
                    "object-literal-validation-widened",
                    path,
                    "TypeScript object literal types cannot close structural compatibility",
                );
                for (key, value) in object {
                    self.audit_literal(
                        value,
                        &format!("{path}/{}", pointer_escape(key)),
                        depth + 1,
                    )?;
                }
            }
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
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
            to: TYPESCRIPT_DIALECT.into(),
            note: note.to_owned().into(),
            location: Some(Box::new(
                RdfLocation::logical(LOSS_CONTEXT).with_subject(path),
            )),
        });
    }
}

fn definition_names(
    definitions: &Map<String, Value>,
) -> Result<BTreeMap<String, String>, TypeScriptError> {
    let mut names = BTreeMap::new();
    let mut reverse = BTreeMap::<String, String>::new();
    for key in definitions.keys() {
        let name = typescript_type_name(key, "SchemaType");
        if RESERVED_TYPE_NAMES.binary_search(&name.as_str()).is_ok() || is_typescript_keyword(&name)
        {
            return Err(TypeScriptError::new(format!(
                "$defs key {key:?} normalizes to reserved TypeScript type name {name:?}"
            )));
        }
        if let Some(previous) = reverse.insert(name.clone(), key.clone()) {
            return Err(TypeScriptError::new(format!(
                "$defs keys {previous:?} and {key:?} collide on TypeScript type name {name:?}"
            )));
        }
        names.insert(key.clone(), name);
    }
    Ok(names)
}

fn validate_keyword_values(object: &Map<String, Value>, path: &str) -> Result<(), TypeScriptError> {
    let _ = declared_kinds(object, path)?;
    if let Some(value) = object.get("enum") {
        let values = value
            .as_array()
            .ok_or_else(|| TypeScriptError::new(format!("{path}/enum must be an array")))?;
        let mut seen = BTreeSet::new();
        for (index, value) in values.iter().enumerate() {
            let key = serde_json::to_string(value).map_err(|error| {
                TypeScriptError::new(format!("cannot inspect {path}/enum/{index}: {error}"))
            })?;
            if !seen.insert(key) {
                return Err(TypeScriptError::new(format!(
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
            return Err(TypeScriptError::new(format!(
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
            return Err(TypeScriptError::new(format!(
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
            return Err(TypeScriptError::new(format!(
                "{path}/{keyword} must be a number"
            )));
        }
    }
    if object
        .get("multipleOf")
        .and_then(Value::as_f64)
        .is_some_and(|value| value <= 0.0)
    {
        return Err(TypeScriptError::new(format!(
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
            let _ = nonnegative_usize(object, keyword, path)?;
        }
    }
    for keyword in ["uniqueItems", "deprecated", "readOnly", "writeOnly"] {
        if object.get(keyword).is_some_and(|value| !value.is_boolean()) {
            return Err(TypeScriptError::new(format!(
                "{path}/{keyword} must be a boolean"
            )));
        }
    }
    let _ = required_names(object, path)?;
    if let Some(dependencies) = object.get("dependentRequired") {
        let dependencies = dependencies.as_object().ok_or_else(|| {
            TypeScriptError::new(format!("{path}/dependentRequired must be an object"))
        })?;
        for (property, names) in dependencies {
            let names = names.as_array().ok_or_else(|| {
                TypeScriptError::new(format!(
                    "{path}/dependentRequired/{} must be an array",
                    pointer_escape(property)
                ))
            })?;
            let mut seen = BTreeSet::new();
            for (index, name) in names.iter().enumerate() {
                let name = name.as_str().ok_or_else(|| {
                    TypeScriptError::new(format!(
                        "{path}/dependentRequired/{}/{index} must be a string",
                        pointer_escape(property)
                    ))
                })?;
                if !seen.insert(name) {
                    return Err(TypeScriptError::new(format!(
                        "{path}/dependentRequired/{} repeats property {name:?}",
                        pointer_escape(property)
                    )));
                }
            }
        }
    }
    Ok(())
}

fn declared_kinds(
    object: &Map<String, Value>,
    path: &str,
) -> Result<(BTreeSet<JsonKind>, bool, bool), TypeScriptError> {
    let mut kinds = BTreeSet::new();
    let mut has_integer = false;
    let mut has_number = false;
    let values: Vec<(&str, String)> = match object.get("type") {
        None => {
            kinds.extend([
                JsonKind::Null,
                JsonKind::Boolean,
                JsonKind::Number,
                JsonKind::String,
                JsonKind::Array,
                JsonKind::Object,
            ]);
            return Ok((kinds, false, true));
        }
        Some(Value::String(kind)) => vec![(kind, format!("{path}/type"))],
        Some(Value::Array(values)) if !values.is_empty() => values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                value
                    .as_str()
                    .map(|kind| (kind, format!("{path}/type/{index}")))
                    .ok_or_else(|| {
                        TypeScriptError::new(format!("{path}/type/{index} must be a string"))
                    })
            })
            .collect::<Result<_, _>>()?,
        Some(Value::Array(_)) => {
            return Err(TypeScriptError::new(format!(
                "{path}/type array cannot be empty"
            )));
        }
        Some(_) => {
            return Err(TypeScriptError::new(format!(
                "{path}/type must be a string or non-empty array of strings"
            )));
        }
    };
    let mut seen = BTreeSet::new();
    for (kind, kind_path) in values {
        if !seen.insert(kind) {
            return Err(TypeScriptError::new(format!(
                "{path}/type repeats type {kind:?}"
            )));
        }
        match kind {
            "null" => {
                kinds.insert(JsonKind::Null);
            }
            "boolean" => {
                kinds.insert(JsonKind::Boolean);
            }
            "number" => {
                has_number = true;
                kinds.insert(JsonKind::Number);
            }
            "integer" => {
                has_integer = true;
                kinds.insert(JsonKind::Number);
            }
            "string" => {
                kinds.insert(JsonKind::String);
            }
            "array" => {
                kinds.insert(JsonKind::Array);
            }
            "object" => {
                kinds.insert(JsonKind::Object);
            }
            _ => {
                return Err(TypeScriptError::new(format!(
                    "{kind_path} names unsupported JSON Schema type {kind:?}"
                )));
            }
        }
    }
    Ok((kinds, has_integer, has_number))
}

fn required_names(
    object: &Map<String, Value>,
    path: &str,
) -> Result<BTreeSet<String>, TypeScriptError> {
    let Some(value) = object.get("required") else {
        return Ok(BTreeSet::new());
    };
    let values = value
        .as_array()
        .ok_or_else(|| TypeScriptError::new(format!("{path}/required must be an array")))?;
    let mut required = BTreeSet::new();
    for (index, value) in values.iter().enumerate() {
        let name = value.as_str().ok_or_else(|| {
            TypeScriptError::new(format!("{path}/required/{index} must be a string"))
        })?;
        if !required.insert(name.to_owned()) {
            return Err(TypeScriptError::new(format!(
                "{path}/required repeats property {name:?}"
            )));
        }
    }
    Ok(required)
}

fn array_bound(
    object: &Map<String, Value>,
    keyword: &str,
    path: &str,
) -> Result<Option<usize>, TypeScriptError> {
    object
        .contains_key(keyword)
        .then(|| nonnegative_usize(object, keyword, path))
        .transpose()
}

fn nonnegative_usize(
    object: &Map<String, Value>,
    keyword: &str,
    path: &str,
) -> Result<usize, TypeScriptError> {
    let value = object
        .get(keyword)
        .expect("nonnegative_usize is called only for a present keyword");
    // Only values through the fixed tuple budget affect emitted structure.
    // Saturating larger integers keeps native and wasm32 behavior identical.
    let over_budget = MAX_TUPLE_EXPANSION + 1;
    if let Some(value) = value.as_u64() {
        return Ok(usize::try_from(value.min(over_budget as u64))
            .expect("the fixed expansion-budget sentinel fits every supported usize"));
    }
    if let Some(value) = value.as_f64()
        && value >= 0.0
        && value.fract() == 0.0
    {
        return Ok(if value > over_budget as f64 {
            over_budget
        } else {
            value as usize
        });
    }
    Err(TypeScriptError::new(format!(
        "{path}/{keyword} must be a non-negative integer"
    )))
}

fn integer_exceeds_typescript_exact_range(number: &serde_json::Number) -> bool {
    const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;
    if let Some(value) = number.as_i64() {
        !(-MAX_SAFE_INTEGER..=MAX_SAFE_INTEGER).contains(&value)
    } else {
        number
            .as_u64()
            .is_some_and(|value| value > MAX_SAFE_INTEGER as u64)
    }
}

fn render_array_relation(
    prefix: &[String],
    rest: &str,
    min_items: usize,
    max_items: Option<usize>,
) -> Result<String, TypeScriptError> {
    if max_items.is_some_and(|maximum| maximum < min_items) {
        return Ok("never".to_owned());
    }
    if let Some(maximum) = max_items {
        let mut variants = Vec::with_capacity(maximum - min_items + 1);
        for length in min_items..=maximum {
            if length > prefix.len() && rest == "never" {
                continue;
            }
            variants.push(render_tuple_for_length(prefix, rest, length));
        }
        return Ok(join_union(variants));
    }
    if prefix.is_empty() && min_items == 0 {
        return Ok(format!("readonly ({rest})[]"));
    }
    if rest == "never" {
        let mut variants = Vec::new();
        for length in min_items..=prefix.len() {
            variants.push(render_tuple_for_length(prefix, rest, length));
        }
        return Ok(join_union(variants));
    }

    let threshold = prefix.len().max(min_items);
    let mut variants = Vec::new();
    for length in min_items..threshold {
        variants.push(render_tuple_for_length(prefix, rest, length));
    }
    if threshold == 0 {
        variants.push(format!("readonly ({rest})[]"));
    } else {
        let mut head = Vec::with_capacity(threshold);
        for index in 0..threshold {
            head.push(
                prefix
                    .get(index)
                    .cloned()
                    .unwrap_or_else(|| rest.to_owned()),
            );
        }
        variants.push(render_tuple_with_rest(&head, rest));
    }
    Ok(join_union(variants))
}

fn render_tuple_for_length(prefix: &[String], rest: &str, length: usize) -> String {
    let mut items = Vec::with_capacity(length);
    for index in 0..length {
        items.push(
            prefix
                .get(index)
                .cloned()
                .unwrap_or_else(|| rest.to_owned()),
        );
    }
    render_tuple(&items)
}

fn render_tuple(items: &[String]) -> String {
    if items.is_empty() {
        "readonly []".to_owned()
    } else {
        format!("readonly [{}]", items.join(", "))
    }
}

fn render_tuple_with_rest(head: &[String], rest: &str) -> String {
    let mut parts = head.to_vec();
    parts.push(format!("...Array<{rest}>"));
    format!("readonly [{}]", parts.join(", "))
}

fn join_union(expressions: Vec<String>) -> String {
    let mut unique = Vec::new();
    let mut seen = BTreeSet::new();
    for expression in expressions {
        if expression == "never" {
            continue;
        }
        if expression == "JsonValue" {
            return "JsonValue".to_owned();
        }
        if seen.insert(expression.clone()) {
            unique.push(expression);
        }
    }
    match unique.len() {
        0 => "never".to_owned(),
        1 => unique.pop().expect("length checked"),
        _ => format!("({})", unique.join(" | ")),
    }
}

fn join_intersection(expressions: Vec<String>) -> String {
    let mut unique = Vec::new();
    let mut seen = BTreeSet::new();
    for expression in expressions {
        if expression == "never" {
            return "never".to_owned();
        }
        if expression == "JsonValue" {
            continue;
        }
        if seen.insert(expression.clone()) {
            unique.push(expression);
        }
    }
    match unique.len() {
        0 => "JsonValue".to_owned(),
        1 => unique.pop().expect("length checked"),
        _ => format!("({})", unique.join(" & ")),
    }
}

fn has_object_shape(object: &Map<String, Value>) -> bool {
    [
        "properties",
        "required",
        "patternProperties",
        "additionalProperties",
    ]
    .iter()
    .any(|keyword| object.contains_key(*keyword))
}

fn has_array_shape(object: &Map<String, Value>) -> bool {
    ["items", "prefixItems", "minItems", "maxItems"]
        .iter()
        .any(|keyword| object.contains_key(*keyword))
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

fn schema_doc(schema: &Value) -> Result<Option<&str>, TypeScriptError> {
    let Some(object) = schema.as_object() else {
        return Ok(None);
    };
    if let Some(description) = object.get("description") {
        return description
            .as_str()
            .map(Some)
            .ok_or_else(|| TypeScriptError::new("JSON Schema description must be a string"));
    }
    if let Some(title) = object.get("title") {
        return title
            .as_str()
            .map(Some)
            .ok_or_else(|| TypeScriptError::new("JSON Schema title must be a string"));
    }
    Ok(None)
}

fn normalize_prose(label: &str, value: &str) -> Result<String, TypeScriptError> {
    let normalized = value.replace("\r\n", "\n").replace('\r', "\n");
    if normalized.trim().is_empty() {
        return Err(TypeScriptError::new(format!(
            "TypeScript {label} must be caller-supplied non-whitespace text"
        )));
    }
    if normalized
        .chars()
        .any(|character| character.is_control() && character != '\n' && character != '\t')
    {
        return Err(TypeScriptError::new(format!(
            "TypeScript {label} contains an unsupported control character"
        )));
    }
    Ok(normalized)
}

fn write_doc_comment(output: &mut String, depth: usize, text: &str, tag: Option<&str>) {
    let pad = indentation(depth);
    writeln!(output, "{pad}/**").expect("writing TypeScript to a String cannot fail");
    let sanitized = if text.contains("*/") {
        Cow::Owned(text.replace("*/", "* /"))
    } else {
        Cow::Borrowed(text)
    };
    for line in sanitized.lines() {
        if line.is_empty() {
            writeln!(output, "{pad} *").expect("writing TypeScript to a String cannot fail");
        } else {
            writeln!(output, "{pad} * {line}").expect("writing TypeScript to a String cannot fail");
        }
    }
    if let Some(tag) = tag {
        writeln!(output, "{pad} * {tag}").expect("writing TypeScript to a String cannot fail");
    }
    writeln!(output, "{pad} */").expect("writing TypeScript to a String cannot fail");
}

fn indentation(depth: usize) -> String {
    "  ".repeat(depth)
}

fn ensure_depth(depth: usize, path: &str) -> Result<(), TypeScriptError> {
    if depth > MAX_SCHEMA_DEPTH {
        Err(TypeScriptError::new(format!(
            "TypeScript schema expression at {path} exceeds depth {MAX_SCHEMA_DEPTH}"
        )))
    } else {
        Ok(())
    }
}

fn json_string(value: &str) -> String {
    serde_json::to_string(value).expect("serializing a Rust string to JSON cannot fail")
}

fn typescript_type_name(raw: &str, fallback: &str) -> String {
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
    output
}

fn is_typescript_keyword(value: &str) -> bool {
    matches!(
        value,
        "Abstract"
            | "Any"
            | "As"
            | "Asserts"
            | "Async"
            | "Await"
            | "Bigint"
            | "Boolean"
            | "Break"
            | "Case"
            | "Catch"
            | "Class"
            | "Const"
            | "Constructor"
            | "Continue"
            | "Debugger"
            | "Declare"
            | "Default"
            | "Delete"
            | "Do"
            | "Else"
            | "Enum"
            | "Export"
            | "Extends"
            | "False"
            | "Finally"
            | "For"
            | "From"
            | "Function"
            | "Get"
            | "If"
            | "Implements"
            | "Import"
            | "In"
            | "Infer"
            | "Instanceof"
            | "Interface"
            | "Is"
            | "Keyof"
            | "Let"
            | "Module"
            | "Namespace"
            | "Never"
            | "New"
            | "Null"
            | "Number"
            | "Object"
            | "Of"
            | "Package"
            | "Private"
            | "Protected"
            | "Public"
            | "Readonly"
            | "Require"
            | "Return"
            | "Set"
            | "Static"
            | "String"
            | "Super"
            | "Switch"
            | "Symbol"
            | "This"
            | "Throw"
            | "True"
            | "Try"
            | "Type"
            | "Typeof"
            | "Undefined"
            | "Unique"
            | "Unknown"
            | "Var"
            | "Void"
            | "While"
            | "With"
            | "Yield"
    )
}

fn is_package_name(value: &str) -> bool {
    if value.is_empty() || value.len() > 214 || value == "." || value == ".." {
        return false;
    }
    let valid_part = |part: &str| {
        !part.is_empty()
            && !part.starts_with(['.', '_'])
            && part.chars().all(|character| {
                character.is_ascii_lowercase()
                    || character.is_ascii_digit()
                    || matches!(character, '-' | '_' | '.')
            })
    };
    if let Some(scoped) = value.strip_prefix('@') {
        let Some((scope, name)) = scoped.split_once('/') else {
            return false;
        };
        !name.contains('/') && valid_part(scope) && valid_part(name)
    } else {
        !value.contains('/') && valid_part(value)
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

    fn config() -> TypeScriptConfig {
        TypeScriptConfig::new(
            "@example/schema-types",
            "Caller-owned package documentation.",
            "Caller-owned module documentation.",
        )
        .expect("valid config")
    }

    fn declaration(package: &TypeScriptPackage) -> &str {
        std::str::from_utf8(
            package
                .artifacts
                .get(TYPESCRIPT_DECLARATION_PATH)
                .expect("declaration artifact exists"),
        )
        .expect("declaration is UTF-8")
    }

    fn exact_schema() -> Value {
        json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$id": "https://example.org/schema/types.json",
            "$defs": {
                "Alias": { "$ref": "#/$defs/Person" },
                "Choice": {
                    "anyOf": [
                        { "const": "ex:open" },
                        { "const": true }
                    ]
                },
                "ClosedEmpty": {
                    "type": "object",
                    "additionalProperties": false
                },
                "Person": {
                    "type": "object",
                    "title": "Person",
                    "description": "A caller-described person.",
                    "properties": {
                        "@id": { "type": "string" },
                        "ex:choice": { "$ref": "#/$defs/Choice" },
                        "ex:nullable": { "type": ["string", "null"] },
                        "ex:tags": {
                            "type": "array",
                            "items": { "type": "string" },
                            "minItems": 1,
                            "maxItems": 2
                        },
                        "ex:tuple": {
                            "type": "array",
                            "prefixItems": [
                                { "type": "string" },
                                { "type": "number" }
                            ],
                            "items": false
                        }
                    },
                    "required": ["@id"]
                },
                "path/with~token": { "enum": [null, 7, "mapped"] }
            }
        })
    }

    #[test]
    fn exact_projection_is_deterministic_reversible_and_lossless() {
        let schema = exact_schema();
        let first = emit_typescript(&compiled(&schema), &config()).expect("emits");
        let second = emit_typescript(&compiled(&schema), &config()).expect("emits");
        assert_eq!(first, second);
        assert!(first.losses.is_empty(), "{}", first.losses.render_json());
        assert_eq!(first.package_name, "@example/schema-types");
        assert_eq!(first.artifacts.len(), 1);
        assert_eq!(
            first.type_names,
            BTreeMap::from([
                ("Alias".to_owned(), "Alias".to_owned()),
                ("Choice".to_owned(), "Choice".to_owned()),
                ("ClosedEmpty".to_owned(), "ClosedEmpty".to_owned()),
                ("Person".to_owned(), "Person".to_owned()),
                ("path/with~token".to_owned(), "PathWithToken".to_owned()),
            ])
        );

        let source = declaration(&first);
        assert!(source.ends_with('\n'));
        assert!(source.contains("@packageDocumentation"));
        assert!(source.contains("export type Alias = Person;"));
        assert!(source.contains("export type Choice = (\"ex:open\" | true);"));
        assert!(source.contains("readonly \"@id\": string;"));
        assert!(source.contains("readonly \"ex:choice\"?: Choice;"));
        assert!(
            source.contains(
                "readonly \"ex:tags\"?: (readonly [string] | readonly [string, string]);"
            )
        );
        assert!(source.contains(
            "readonly \"ex:tuple\"?: (readonly [] | readonly [string] | readonly [string, number]);"
        ));
        assert!(source.contains("readonly [key: string]: never;"));
        assert!(source.contains("export type PathWithToken = (null | 7 | \"mapped\");"));
        assert!(!source.contains("gmeow"));
        assert!(!source.contains("blackcatinformatics.ca"));
        assert!(
            !source
                .split(|character: char| !character.is_ascii_alphanumeric())
                .any(|token| token == "any")
        );
    }

    #[test]
    fn lossy_projection_exercises_the_entire_closed_profile() {
        let prefix_items = (0..=MAX_TUPLE_EXPANSION)
            .map(|index| {
                if index % 2 == 0 {
                    json!({ "type": "string" })
                } else {
                    json!({ "type": "number" })
                }
            })
            .collect::<Vec<_>>();
        let mut schema = json!({
            "$defs": {
                "Lossy": {
                    "type": "object",
                    "additionalProperties": { "type": "integer" },
                    "patternProperties": {
                        "^ex:": { "type": "string", "pattern": "^[A-Z]" }
                    },
                    "minProperties": 1,
                    "maxProperties": 20,
                    "propertyNames": { "pattern": "^ex:" },
                    "dependentRequired": { "ex:a": ["ex:b"] },
                    "dependentSchemas": { "ex:a": { "required": ["ex:b"] } },
                    "if": { "properties": { "ex:a": { "const": true } } },
                    "then": { "required": ["ex:b"] },
                    "else": { "required": ["ex:c"] },
                    "unevaluatedProperties": false,
                    "properties": {
                        "ex:array": {
                            "type": "array",
                            "prefixItems": [],
                            "items": { "type": "string" },
                            "additionalItems": false,
                            "minItems": 40,
                            "maxItems": 100,
                            "contains": { "const": "match" },
                            "minContains": 1,
                            "maxContains": 2,
                            "uniqueItems": true,
                            "unevaluatedItems": false
                        },
                        "ex:literal": { "enum": [{ "state": "open" }] },
                        "ex:negated": { "not": { "type": "boolean" } },
                        "ex:number": {
                            "type": "number",
                            "minimum": 0,
                            "exclusiveMaximum": 10,
                            "multipleOf": 0.5
                        },
                        "ex:choice": {
                            "oneOf": [{ "type": "string" }, { "const": "overlap" }]
                        },
                        "ex:unsupported": {
                            "unsupportedAssertion": true
                        }
                    }
                }
            }
        });
        schema["$defs"]["Lossy"]["properties"]["ex:array"]["prefixItems"] =
            Value::Array(prefix_items);

        let package = emit_typescript(&compiled(&schema), &config()).expect("lossy schema emits");
        let expected = [
            "additional-properties-validation-widened",
            "array-cardinality-validation-widened",
            "array-contains-validation-dropped",
            "conditional-validation-dropped",
            "dependency-validation-dropped",
            "integer-validation-widened",
            "keyword-validation-dropped",
            "negation-validation-dropped",
            "numeric-validation-dropped",
            "object-literal-validation-widened",
            "one-of-validation-widened",
            "pattern-properties-validation-dropped",
            "property-count-validation-dropped",
            "property-name-validation-dropped",
            "string-validation-dropped",
            "tuple-array-validation-widened",
            "unevaluated-validation-dropped",
            "unique-items-validation-dropped",
        ];
        check_ledger_sound(&package.losses, LOSS_FROM, TYPESCRIPT_DIALECT)
            .expect("all losses are registered");
        check_ledger_complete(&package.losses, &expected).expect("profile is fully exercised");
        assert!(package.losses.entries().iter().all(|entry| {
            entry
                .location
                .as_ref()
                .and_then(|location| location.subject.as_deref())
                .is_some_and(|subject| subject.starts_with("#/$defs/Lossy"))
        }));
        let source = declaration(&package);
        assert!(source.contains("export type Lossy ="));
        assert!(!source.contains("any"));
    }

    #[test]
    fn config_is_caller_owned_canonical_and_comment_safe() {
        let config = TypeScriptConfig::new(
            "@caller/schema.types",
            "Package */ prose.\r\nSecond line.",
            "Module prose.",
        )
        .expect("valid config");
        assert_eq!(config.package_name(), "@caller/schema.types");
        assert_eq!(
            config.package_docstring(),
            "Package */ prose.\nSecond line."
        );
        assert_eq!(config.module_docstring(), "Module prose.");
        let package = emit_typescript(&compiled(&json!({ "$defs": {} })), &config)
            .expect("empty package emits");
        assert!(declaration(&package).contains("Package * / prose.\n * Second line."));

        for package_name in ["", "UPPER", "../escape", "@scope", "@scope/name/extra"] {
            assert!(TypeScriptConfig::new(package_name, "package", "module").is_err());
        }
        assert!(TypeScriptConfig::new("valid-name", " ", "module").is_err());
        assert!(TypeScriptConfig::new("valid-name", "package", "\0module").is_err());
    }

    #[test]
    fn name_reference_and_keyword_failures_are_hard_errors() {
        for (schema, expected) in [
            (
                json!({ "$defs": { "a-b": true, "a_b": true } }),
                "collide on TypeScript type name",
            ),
            (
                json!({ "$defs": { "JsonValue": true } }),
                "reserved TypeScript type name",
            ),
            (
                json!({ "$defs": { "Broken": { "type": [] } } }),
                "type array cannot be empty",
            ),
            (
                json!({ "$defs": { "Broken": { "required": [7] } } }),
                "required/0 must be a string",
            ),
            (
                json!({
                    "$defs": {
                        "Broken": { "dependentRequired": { "ex:value": true } }
                    }
                }),
                "dependentRequired/ex:value must be an array",
            ),
            (
                json!({ "$defs": { "Broken": { "anyOf": [] } } }),
                "anyOf cannot be empty",
            ),
            (
                json!({ "$defs": { "Broken": { "multipleOf": 0 } } }),
                "multipleOf must be greater than zero",
            ),
            (
                json!({ "$defs": { "Broken": { "$ref": "#/$defs/Missing" } } }),
                "targets missing $defs key",
            ),
            (
                json!({ "$defs": { "Broken": { "$ref": "https://example.org/open" } } }),
                "external or not a direct",
            ),
            (
                json!({ "$defs": { "Broken": { "$dynamicRef": "#/$defs/Broken" } } }),
                "cannot be translated to a closed generated package",
            ),
            (
                json!({
                    "$defs": {
                        "Broken": {
                            "$id": "nested.json",
                            "$ref": "#/$defs/Target"
                        },
                        "Target": true
                    }
                }),
                "$id cannot rebase a closed generated package",
            ),
            (
                json!({ "$defs": { "Broken": { "$ref": "#/$defs/Broken" } } }),
                "unguarded recursive TypeScript alias cycle: Broken -> Broken",
            ),
            (
                json!({
                    "$defs": {
                        "Alpha": { "anyOf": [{ "$ref": "#/$defs/Beta" }] },
                        "Beta": { "allOf": [{ "$ref": "#/$defs/Alpha" }] }
                    }
                }),
                "unguarded recursive TypeScript alias cycle: Alpha -> Beta -> Alpha",
            ),
        ] {
            let error = emit_typescript(&compiled(&schema), &config())
                .expect_err("malformed schema must fail");
            assert!(error.to_string().contains(expected), "{error}");
        }
    }

    #[test]
    fn reference_cycles_guarded_by_carriers_remain_supported() {
        let schema = json!({
            "$defs": {
                "Alias": { "$ref": "#/$defs/Node" },
                "Always": {
                    "anyOf": [{ "$ref": "#/$defs/Always" }, true]
                },
                "Node": {
                    "type": "object",
                    "properties": {
                        "children": {
                            "type": "array",
                            "items": { "$ref": "#/$defs/Alias" }
                        }
                    }
                },
                "Bottom": {
                    "allOf": [{ "$ref": "#/$defs/Bottom" }, false]
                }
            }
        });
        let package = emit_typescript(&compiled(&schema), &config()).expect("guarded cycle emits");
        let source = declaration(&package);
        assert!(source.contains("export type Alias = Node;"));
        assert!(source.contains("export type Always = JsonValue;"));
        assert!(source.contains("readonly \"children\"?: readonly (Alias)[];"));
        assert!(source.contains("export type Bottom = never;"));
    }

    #[test]
    fn long_unguarded_alias_chains_are_checked_without_recursion() {
        const DEFINITION_COUNT: usize = 8_192;
        let mut definitions = Map::new();
        for index in 0..DEFINITION_COUNT {
            let definition = if index + 1 == DEFINITION_COUNT {
                Value::Bool(true)
            } else {
                json!({ "$ref": format!("#/$defs/Node{}", index + 1) })
            };
            definitions.insert(format!("Node{index}"), definition);
        }
        validate_unguarded_reference_cycles(&definitions).expect("long alias chain is acyclic");
    }

    #[test]
    fn required_only_closed_property_and_contradictory_arrays_are_uninhabited() {
        let schema = json!({
            "$defs": {
                "ImpossibleObject": {
                    "type": "object",
                    "required": ["ex:missing"],
                    "additionalProperties": false
                },
                "ImpossibleArray": {
                    "type": "array",
                    "items": { "type": "string" },
                    "minItems": 3,
                    "maxItems": 2
                }
            }
        });
        let package = emit_typescript(&compiled(&schema), &config()).expect("emits");
        let source = declaration(&package);
        assert!(source.contains("readonly \"ex:missing\": never;"));
        assert!(source.contains("export type ImpossibleArray = never;"));
        assert!(package.losses.entries().iter().any(|entry| {
            entry.code == "additional-properties-validation-widened"
                && entry
                    .location
                    .as_ref()
                    .and_then(|location| location.subject.as_deref())
                    == Some("#/$defs/ImpossibleObject/additionalProperties")
        }));
    }

    #[test]
    fn inactive_schema_keywords_do_not_claim_losses() {
        let schema = json!({
            "$defs": {
                "ContainsBoundsOnly": {
                    "type": "array",
                    "minContains": 1,
                    "maxContains": 2
                },
                "IfOnly": {
                    "if": { "type": "integer", "minimum": 2 }
                },
                "IgnoredBranches": {
                    "then": { "type": "integer" },
                    "else": { "pattern": "x" }
                },
                "NestedDefinitions": {
                    "$defs": {
                        "Inert": { "type": "string", "pattern": "x" }
                    }
                }
            }
        });
        let package = emit_typescript(&compiled(&schema), &config()).expect("emits");
        assert!(
            package.losses.is_empty(),
            "{}",
            package.losses.render_json()
        );
    }

    #[test]
    fn literal_trees_and_portable_array_bounds_are_audited_exactly() {
        let schema = json!({
            "$defs": {
                "IntegralFloatBounds": {
                    "type": "array",
                    "minItems": 1.0,
                    "maxItems": 2.0
                },
                "LargeBound": {
                    "type": "array",
                    "minItems": 4_294_967_296_u64
                },
                "NestedLiterals": {
                    "enum": [[{
                        "state": "open",
                        "sequence": [9_007_199_254_740_993_u64]
                    }]]
                }
            }
        });
        let package = emit_typescript(&compiled(&schema), &config()).expect("emits");
        let source = declaration(&package);
        assert!(source.contains(
            "export type IntegralFloatBounds = (readonly [JsonValue] | readonly [JsonValue, \
             JsonValue]);"
        ));
        for (code, location) in [
            (
                "array-cardinality-validation-widened",
                "#/$defs/LargeBound/minItems",
            ),
            (
                "object-literal-validation-widened",
                "#/$defs/NestedLiterals/enum/0/0",
            ),
            (
                "numeric-validation-dropped",
                "#/$defs/NestedLiterals/enum/0/0/sequence/0",
            ),
        ] {
            assert!(package.losses.entries().iter().any(|entry| {
                entry.code == code
                    && entry
                        .location
                        .as_ref()
                        .and_then(|value| value.subject.as_deref())
                        == Some(location)
            }));
        }
    }
}
