// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Deterministic [`CompiledSchema`] → Pydantic v2 package emitter.
//!
//! The emitter is deliberately repository- and filesystem-free: it consumes the
//! JSON Schema already compiled by [`crate::json_schema`] and returns an in-memory
//! package artifact map. Package identity and human-facing module documentation
//! are caller configuration; PurRDF neither mints vocabulary nor fabricates a
//! downstream brand.
//!
//! Generated models use strict Pydantic scalar carriers, aliases matching the
//! JSON property names, and a class-owned schema hook taken from the same `$defs`
//! input. Consequently `model_json_schema(by_alias=True)`
//! reconstructs the originating definition (modulo the returned reversible
//! `$defs`-key → Python-class map). JSON Schema assertions that have no exact
//! Pydantic runtime-annotation equivalent remain on that schema surface and are
//! also recorded, at their JSON-pointer location, on [`PydanticPackage::losses`].
//!
//! This is a code generator, so a source-language reader is not meaningful.
//! Pydantic itself supplies the executable reverse surface through
//! `model_json_schema()`; the dev-only oracle exercises that surface against the
//! source [`CompiledSchema`].

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Write as _};

use ::purrdf::RdfLocation;
use ::purrdf::loss::{LossEntry, LossLedger};
use serde_json::{Map, Value};

use crate::json_schema::CompiledSchema;

const MODELS_MODULE: &str = "models";
const BASE_MODULE: &str = "_base";
const BASE_CLASS: &str = "PurrdfBaseModel";

/// Caller-owned configuration for a generated Pydantic package.
///
/// There is intentionally no [`Default`] implementation: package identity and
/// prose must come from the caller, never from a vocabulary baked into PurRDF.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PydanticConfig {
    package_name: String,
    package_docstring: String,
    models_docstring: String,
}

impl PydanticConfig {
    /// Validate and construct an emitter configuration.
    ///
    /// `package_name` may be a dotted Python package path. Every component must
    /// be a non-keyword ASCII Python identifier. Both docstrings must contain
    /// non-whitespace caller text.
    ///
    /// # Errors
    ///
    /// Returns [`PydanticError`] for an invalid package path or blank docstring.
    pub fn new(
        package_name: impl Into<String>,
        package_docstring: impl Into<String>,
        models_docstring: impl Into<String>,
    ) -> Result<Self, PydanticError> {
        let package_name = package_name.into();
        let package_docstring = package_docstring.into();
        let models_docstring = models_docstring.into();

        if package_name.is_empty()
            || package_name
                .split('.')
                .any(|part| !is_python_identifier(part) || is_python_keyword(part))
        {
            return Err(PydanticError::new(format!(
                "Pydantic package name {package_name:?} is not a dotted sequence of non-keyword \
                 ASCII Python identifiers"
            )));
        }
        if package_docstring.trim().is_empty() {
            return Err(PydanticError::new(
                "Pydantic package docstring must be caller-supplied non-whitespace text",
            ));
        }
        if models_docstring.trim().is_empty() {
            return Err(PydanticError::new(
                "Pydantic models-module docstring must be caller-supplied non-whitespace text",
            ));
        }

        Ok(Self {
            package_name,
            package_docstring,
            models_docstring,
        })
    }

    /// The dotted Python package name.
    #[must_use]
    pub fn package_name(&self) -> &str {
        &self.package_name
    }

    /// Caller-supplied package docstring.
    #[must_use]
    pub fn package_docstring(&self) -> &str {
        &self.package_docstring
    }

    /// Caller-supplied models-module docstring.
    #[must_use]
    pub fn models_docstring(&self) -> &str {
        &self.models_docstring
    }
}

/// Deterministic generated Pydantic package and its runtime-projection losses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PydanticPackage {
    /// Relative package paths → exact file bytes, sorted by path.
    pub artifacts: BTreeMap<String, Vec<u8>>,
    /// Source `$defs` key → importable generated model path, sorted by key.
    pub model_paths: BTreeMap<String, String>,
    /// JSON Schema assertions preserved on `model_json_schema()` but not exactly
    /// enforced by Pydantic runtime annotations.
    pub losses: LossLedger,
}

/// A malformed emitter configuration or input schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PydanticError {
    message: String,
}

impl PydanticError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for PydanticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for PydanticError {}

/// Emit a deterministic, filesystem-free Pydantic v2 package from one compiled
/// SHACL-derived JSON Schema.
///
/// Source-stage losses remain on [`CompiledSchema::losses`]. The returned ledger
/// is specifically the next projection step, `json-schema` → `pydantic-v2`.
///
/// # Errors
///
/// Returns [`PydanticError`] when `schema_json` is malformed, `$defs` is absent
/// or malformed, a reference is external/dangling, required/property declarations
/// disagree, or two source names collide after Python identifier normalization.
pub fn emit_pydantic(
    compiled: &CompiledSchema,
    config: &PydanticConfig,
) -> Result<PydanticPackage, PydanticError> {
    let document: Value = serde_json::from_str(&compiled.schema_json).map_err(|error| {
        PydanticError::new(format!(
            "CompiledSchema.schema_json is not valid JSON: {error}"
        ))
    })?;
    let root = document.as_object().ok_or_else(|| {
        PydanticError::new("CompiledSchema.schema_json root must be a JSON object")
    })?;
    let defs = root
        .get("$defs")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            PydanticError::new("CompiledSchema.schema_json must contain an object-valued `$defs`")
        })?;

    let names = definition_names(defs)?;
    for (key, definition) in defs {
        validate_references(
            definition,
            defs,
            &format!("#/$defs/{}", pointer_escape(key)),
        )?;
    }

    let mut renderer = Renderer::new(&names);
    for (key, definition) in defs {
        renderer.audit_schema(definition, &format!("#/$defs/{}", pointer_escape(key)));
    }

    let mut definitions = Vec::with_capacity(defs.len());
    let mut exports = Vec::with_capacity(defs.len());
    let mut model_paths = BTreeMap::new();
    for (key, definition) in defs {
        let class_name = names
            .get(key)
            .expect("definition_names covers every $defs key");
        definitions.push(renderer.render_definition(key, class_name, definition)?);
        exports.push(class_name.clone());
        model_paths.insert(
            key.clone(),
            format!("{}.{}.{}", config.package_name, MODELS_MODULE, class_name),
        );
    }
    let rewritten_defs = defs
        .iter()
        .map(|(key, definition)| {
            let class_name = names
                .get(key)
                .expect("definition_names covers every $defs key")
                .clone();
            rewrite_references(definition, &names).map(|value| (class_name, value))
        })
        .collect::<Result<Map<_, _>, _>>()?;
    let defs_literal = python_value(&Value::Object(rewritten_defs));

    let package_path = config.package_name.replace('.', "/");
    let mut artifacts = BTreeMap::new();
    artifacts.insert(
        format!("{package_path}/{BASE_MODULE}.py"),
        render_base(config.models_docstring()).into_bytes(),
    );
    artifacts.insert(
        format!("{package_path}/{MODELS_MODULE}.py"),
        renderer
            .finish_models(
                config.models_docstring(),
                &defs_literal,
                &definitions,
                &exports,
            )
            .into_bytes(),
    );
    artifacts.insert(
        format!("{package_path}/__init__.py"),
        render_init(config.package_docstring(), &exports).into_bytes(),
    );
    artifacts.insert(format!("{package_path}/py.typed"), Vec::new());

    Ok(PydanticPackage {
        artifacts,
        model_paths,
        losses: renderer.ledger,
    })
}

fn definition_names(defs: &Map<String, Value>) -> Result<BTreeMap<String, String>, PydanticError> {
    let mut names = BTreeMap::new();
    let mut reverse = BTreeMap::<String, String>::new();
    for key in defs.keys() {
        let name = python_type_name(key, "SchemaModel");
        if reserved_type_names().contains(name.as_str()) {
            return Err(PydanticError::new(format!(
                "$defs key {key:?} normalizes to reserved generated/import name {name:?}"
            )));
        }
        if let Some(previous) = reverse.insert(name.clone(), key.clone()) {
            return Err(PydanticError::new(format!(
                "$defs keys {previous:?} and {key:?} collide on Python class name {name:?}"
            )));
        }
        names.insert(key.clone(), name);
    }
    Ok(names)
}

fn validate_references(
    value: &Value,
    defs: &Map<String, Value>,
    path: &str,
) -> Result<(), PydanticError> {
    let Value::Object(object) = value else {
        return if value.is_boolean() {
            Ok(())
        } else {
            Err(PydanticError::new(format!(
                "{path} must be an object or boolean JSON Schema"
            )))
        };
    };

    for keyword in ["$dynamicRef", "$recursiveRef"] {
        if object.contains_key(keyword) {
            return Err(PydanticError::new(format!(
                "{path}/{keyword} cannot be translated to a closed generated package"
            )));
        }
    }

    if let Some(reference) = object.get("$ref") {
        let reference = reference
            .as_str()
            .ok_or_else(|| PydanticError::new(format!("{path}/$ref must be a string")))?;
        let key = reference_key(reference).ok_or_else(|| {
            PydanticError::new(format!(
                "{path}/$ref is external or not a direct #/$defs reference: {reference:?}"
            ))
        })?;
        if !defs.contains_key(&key) {
            return Err(PydanticError::new(format!(
                "{path}/$ref targets missing $defs key {key:?}"
            )));
        }
    }
    for keyword in schema_map_keywords() {
        if let Some(children) = object.get(*keyword) {
            let children = children
                .as_object()
                .ok_or_else(|| PydanticError::new(format!("{path}/{keyword} must be an object")))?;
            for (key, child) in children {
                validate_references(
                    child,
                    defs,
                    &format!("{path}/{keyword}/{}", pointer_escape(key)),
                )?;
            }
        }
    }
    for keyword in schema_array_keywords() {
        if let Some(children) = object.get(*keyword) {
            let children = children
                .as_array()
                .ok_or_else(|| PydanticError::new(format!("{path}/{keyword} must be an array")))?;
            for (index, child) in children.iter().enumerate() {
                validate_references(child, defs, &format!("{path}/{keyword}/{index}"))?;
            }
        }
    }
    for keyword in schema_single_keywords() {
        if let Some(child) = object.get(*keyword) {
            validate_references(child, defs, &format!("{path}/{keyword}"))?;
        }
    }
    Ok(())
}

struct Renderer<'a> {
    names: &'a BTreeMap<String, String>,
    ledger: LossLedger,
    helpers: Vec<String>,
    helper_by_path: BTreeMap<String, String>,
    used_names: BTreeSet<String>,
}

impl<'a> Renderer<'a> {
    fn new(names: &'a BTreeMap<String, String>) -> Self {
        let mut used_names: BTreeSet<String> = names.values().cloned().collect();
        used_names.extend(reserved_type_names().into_iter().map(str::to_owned));
        Self {
            names,
            ledger: LossLedger::new(),
            helpers: Vec::new(),
            helper_by_path: BTreeMap::new(),
            used_names,
        }
    }

    fn render_definition(
        &mut self,
        key: &str,
        class_name: &str,
        definition: &Value,
    ) -> Result<String, PydanticError> {
        let path = format!("#/$defs/{}", pointer_escape(key));
        let rewritten = rewrite_references(definition, self.names)?;
        let schema_literal = python_value(&rewritten);

        if let Some(object) = definition.as_object()
            && is_record_definition(object)
        {
            return self.render_record(class_name, object, &path, &schema_literal);
        }

        self.render_root(class_name, definition, &path, &schema_literal)
    }

    fn render_record(
        &mut self,
        class_name: &str,
        definition: &Map<String, Value>,
        path: &str,
        schema_literal: &str,
    ) -> Result<String, PydanticError> {
        let properties = properties(definition, path)?;
        let required = required_names(definition, properties, path)?;
        let extra = extra_policy(definition, path)?;

        if matches!(
            definition.get("additionalProperties"),
            Some(Value::Object(_))
        ) {
            self.record(
                "inline-object-validation-widened",
                &format!("{path}/additionalProperties"),
                "Pydantic BaseModel extra fields are retained, but their JSON Schema \
                 additionalProperties value schema is not enforced at runtime",
            );
        }

        let mut seen_fields = BTreeMap::<String, String>::new();
        let mut fields = String::new();
        for (property, schema) in properties {
            let field_name = python_field_name(property);
            if let Some(previous) = seen_fields.insert(field_name.clone(), property.clone()) {
                return Err(PydanticError::new(format!(
                    "{path}/properties keys {previous:?} and {property:?} collide on Python field \
                     name {field_name:?}"
                )));
            }
            let property_path = format!("{path}/properties/{}", pointer_escape(property));
            let runtime_type = self.resolve_type(schema, &property_path)?;
            let mut field_args = Vec::new();
            if !required.contains(property.as_str()) {
                field_args.push("default=None".to_owned());
            }
            if let Some(description) = schema.get("description").and_then(Value::as_str) {
                field_args.push(format!("description={}", python_string(description)));
            }
            field_args.push(format!("alias={}", python_string(property)));
            writeln!(
                fields,
                "    {field_name}: {runtime_type} = Field({})",
                field_args.join(", ")
            )
            .expect("writing generated Python to a String cannot fail");
        }

        let mut out = format!("class {class_name}({BASE_CLASS}):\n");
        if let Some(description) = definition.get("description").and_then(Value::as_str) {
            writeln!(out, "    {}", python_string(description))
                .expect("writing generated Python to a String cannot fail");
        }
        out.push_str("    model_config = ConfigDict(\n");
        writeln!(out, "        extra=\"{extra}\",")
            .expect("writing generated Python to a String cannot fail");
        out.push_str("    )\n");
        append_schema_surface(&mut out, schema_literal);
        out.push('\n');
        if !fields.is_empty() {
            out.push_str(&fields);
            out.push('\n');
        }
        Ok(out)
    }

    fn render_root(
        &mut self,
        class_name: &str,
        definition: &Value,
        path: &str,
        schema_literal: &str,
    ) -> Result<String, PydanticError> {
        let mut out = String::new();
        let root_type = if let Some(values) = definition.get("enum").and_then(Value::as_array)
            && values.iter().all(Value::is_string)
        {
            let enum_name = format!("_{class_name}Value");
            out.push_str(&render_string_enum(&enum_name, values));
            if let Some(object) = definition.as_object() {
                apply_constraints(enum_name, object)
            } else {
                enum_name
            }
        } else {
            self.resolve_type(definition, path)?
        };

        writeln!(
            out,
            "class {class_name}(RootModel[ForwardRef({})]):",
            python_string(&root_type)
        )
        .expect("writing generated Python to a String cannot fail");
        if let Some(description) = definition.get("description").and_then(Value::as_str) {
            writeln!(out, "    {}", python_string(description))
                .expect("writing generated Python to a String cannot fail");
        }
        append_schema_surface(&mut out, schema_literal);
        out.push('\n');
        Ok(out)
    }

    fn resolve_type(&mut self, schema: &Value, path: &str) -> Result<String, PydanticError> {
        match schema {
            Value::Bool(_) => Ok("Any".to_owned()),
            Value::Object(object) => {
                let (base, apply_outer_constraints) = if let Some(reference) = object.get("$ref") {
                    let reference = reference.as_str().ok_or_else(|| {
                        PydanticError::new(format!("{path}/$ref must be a string"))
                    })?;
                    let key = reference_key(reference).ok_or_else(|| {
                        PydanticError::new(format!("{path}/$ref is not a direct #/$defs reference"))
                    })?;
                    let resolved = self.names.get(&key).cloned().ok_or_else(|| {
                        PydanticError::new(format!("{path}/$ref targets missing key {key:?}"))
                    })?;
                    (resolved, true)
                } else if let Some(values) = object.get("enum").and_then(Value::as_array) {
                    (self.resolve_enum(values, path)?, true)
                } else if let Some(value) = object.get("const") {
                    (self.resolve_const(value, path)?, true)
                } else if let Some(branches) = object.get("anyOf").and_then(Value::as_array) {
                    (
                        self.resolve_composed_union(branches, object, &format!("{path}/anyOf"))?,
                        false,
                    )
                } else if let Some(branches) = object.get("oneOf").and_then(Value::as_array) {
                    (
                        self.resolve_composed_union(branches, object, &format!("{path}/oneOf"))?,
                        false,
                    )
                } else if let Some(branches) = object.get("allOf").and_then(Value::as_array) {
                    if let Some(first) = branches.first() {
                        (self.resolve_type(first, &format!("{path}/allOf/0"))?, false)
                    } else {
                        ("Any".to_owned(), false)
                    }
                } else {
                    (self.resolve_declared_type(object, path)?, true)
                };
                if apply_outer_constraints {
                    Ok(apply_constraints(base, object))
                } else {
                    Ok(base)
                }
            }
            _ => Err(PydanticError::new(format!(
                "{path} must be a JSON Schema object or boolean"
            ))),
        }
    }

    fn resolve_declared_type(
        &mut self,
        object: &Map<String, Value>,
        path: &str,
    ) -> Result<String, PydanticError> {
        let Some(declared) = object.get("type") else {
            if object.contains_key("properties") || object.contains_key("additionalProperties") {
                return self.resolve_object(object, path);
            }
            return Ok("Any".to_owned());
        };

        match declared {
            Value::String(kind) => self.resolve_type_name(kind, object, path),
            Value::Array(kinds) => {
                let mut resolved = Vec::with_capacity(kinds.len());
                for (index, kind) in kinds.iter().enumerate() {
                    let kind = kind.as_str().ok_or_else(|| {
                        PydanticError::new(format!("{path}/type/{index} must be a string"))
                    })?;
                    resolved.push(self.resolve_type_name(kind, object, path)?);
                }
                Ok(join_union(resolved))
            }
            _ => Err(PydanticError::new(format!(
                "{path}/type must be a string or array of strings"
            ))),
        }
    }

    fn resolve_type_name(
        &mut self,
        kind: &str,
        object: &Map<String, Value>,
        path: &str,
    ) -> Result<String, PydanticError> {
        match kind {
            "null" => Ok("None".to_owned()),
            "boolean" => Ok("StrictBool".to_owned()),
            "integer" => Ok("StrictInt".to_owned()),
            "number" => Ok("StrictFloat | StrictInt".to_owned()),
            "string" => Ok(match object.get("format").and_then(Value::as_str) {
                Some("date-time") => "datetime".to_owned(),
                Some("date") => "date".to_owned(),
                Some("time") => "time".to_owned(),
                _ => "StrictStr".to_owned(),
            }),
            "array" => {
                let items = object.get("items").unwrap_or(&Value::Bool(true));
                let item_type = self.resolve_type(items, &format!("{path}/items"))?;
                Ok(format!("list[{item_type}]"))
            }
            "object" => self.resolve_object(object, path),
            other => Err(PydanticError::new(format!(
                "{path}/type names unsupported JSON Schema type {other:?}"
            ))),
        }
    }

    fn resolve_object(
        &mut self,
        object: &Map<String, Value>,
        path: &str,
    ) -> Result<String, PydanticError> {
        let properties = properties(object, path)?;
        if properties.is_empty() {
            return match object.get("additionalProperties") {
                Some(Value::Bool(false)) => self.render_typed_dict(object, path),
                Some(Value::Object(schema)) => {
                    let value_type = self.resolve_type(
                        &Value::Object(schema.clone()),
                        &format!("{path}/additionalProperties"),
                    )?;
                    Ok(format!("dict[StrictStr, {value_type}]"))
                }
                Some(Value::Bool(true)) | None => Ok("dict[StrictStr, Any]".to_owned()),
                Some(_) => Err(PydanticError::new(format!(
                    "{path}/additionalProperties must be a boolean or schema"
                ))),
            };
        }
        self.render_typed_dict(object, path)
    }

    fn render_typed_dict(
        &mut self,
        object: &Map<String, Value>,
        path: &str,
    ) -> Result<String, PydanticError> {
        if let Some(name) = self.helper_by_path.get(path) {
            return Ok(name.clone());
        }
        let name = self.allocate_helper_name(path);
        self.helper_by_path.insert(path.to_owned(), name.clone());

        let properties = properties(object, path)?;
        let required = required_names(object, properties, path)?;
        let mut fields = Vec::with_capacity(properties.len());
        for (property, schema) in properties {
            let child_path = format!("{path}/properties/{}", pointer_escape(property));
            let child_type = self.resolve_type(schema, &child_path)?;
            let marker = if required.contains(property.as_str()) {
                "Required"
            } else {
                "NotRequired"
            };
            fields.push(format!(
                "{}: {marker}[ForwardRef({})]",
                python_string(property),
                python_string(&child_type)
            ));
        }
        let policy = extra_policy(object, path)?;
        if matches!(object.get("additionalProperties"), Some(Value::Object(_))) {
            self.record(
                "inline-object-validation-widened",
                &format!("{path}/additionalProperties"),
                "A TypedDict with named properties cannot enforce the value schema of arbitrary \
                 additional keys; those keys remain allowed and the exact rule remains on the \
                 JSON-schema surface",
            );
        }

        let declaration = format!(
            "{name} = TypedDict({}, {{{}}})\n{name}.__pydantic_config__ = \
             ConfigDict(extra=\"{policy}\")\n",
            python_string(&name),
            fields.join(", ")
        );
        self.helpers.push(declaration);
        Ok(name)
    }

    fn resolve_enum(&mut self, values: &[Value], path: &str) -> Result<String, PydanticError> {
        if values.is_empty() {
            return Ok("Never".to_owned());
        }
        let mut variants = Vec::with_capacity(values.len());
        let mut scalars = Vec::new();
        for (index, value) in values.iter().enumerate() {
            match value {
                Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {
                    scalars.push(python_value(value));
                }
                Value::Object(member) => {
                    let member_schema = enum_object_schema(member);
                    variants
                        .push(self.resolve_type(&member_schema, &format!("{path}/enum/{index}"))?);
                }
                Value::Array(_) => {
                    self.record(
                        "keyword-validation-dropped",
                        &format!("{path}/enum/{index}"),
                        "A JSON array enum member is not a legal Python Literal parameter; the \
                         exact member remains on model_json_schema()",
                    );
                    variants.push("list[Any]".to_owned());
                }
            }
        }
        if !scalars.is_empty() {
            variants.push(format!("Literal[{}]", scalars.join(", ")));
        }
        Ok(join_union(variants))
    }

    fn resolve_const(&mut self, value: &Value, path: &str) -> Result<String, PydanticError> {
        match value {
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {
                Ok(format!("Literal[{}]", python_value(value)))
            }
            Value::Object(member) => {
                self.resolve_type(&enum_object_schema(member), &format!("{path}/const"))
            }
            Value::Array(_) => {
                self.record(
                    "keyword-validation-dropped",
                    &format!("{path}/const"),
                    "A JSON array const is not a legal Python Literal parameter; runtime \
                     validation retains only the array carrier",
                );
                Ok("list[Any]".to_owned())
            }
        }
    }

    fn resolve_composed_union(
        &mut self,
        branches: &[Value],
        parent: &Map<String, Value>,
        path: &str,
    ) -> Result<String, PydanticError> {
        let mut types = Vec::with_capacity(branches.len());
        for (index, branch) in branches.iter().enumerate() {
            let branch_path = format!("{path}/{index}");
            let (augmented, merge_conflict) = branch_with_parent_constraints(branch, parent);
            if merge_conflict {
                self.record(
                    "keyword-validation-dropped",
                    &branch_path,
                    "A composed branch and its parent carry overlapping assertion keywords that \
                     cannot both be represented by one Pydantic Field annotation",
                );
            }
            if augmented == *branch
                && has_runtime_constraints(parent)
                && branch
                    .as_object()
                    .is_some_and(|object| object.contains_key("$ref"))
            {
                self.record(
                    "keyword-validation-dropped",
                    &branch_path,
                    "A sibling assertion on a composed $ref cannot be assigned to a concrete \
                     Pydantic runtime carrier; it remains on model_json_schema()",
                );
            }
            types.push(self.resolve_type(&augmented, &branch_path)?);
        }
        Ok(join_union(types))
    }

    fn allocate_helper_name(&mut self, path: &str) -> String {
        let raw = format!("Inline {path} Object");
        let mut stem = format!("_{}", python_type_name(&raw, "InlineObject"));
        if stem.len() > 112 {
            stem = format!("_InlineObject{:016x}", fnv1a(path.as_bytes()));
        }
        let mut candidate = stem.clone();
        let mut suffix = 2_u32;
        while self.used_names.contains(&candidate) {
            candidate = format!("{stem}{suffix}");
            suffix += 1;
        }
        self.used_names.insert(candidate.clone());
        candidate
    }

    fn audit_schema(&mut self, schema: &Value, path: &str) {
        let Value::Object(object) = schema else {
            if schema == &Value::Bool(false) {
                self.record(
                    "keyword-validation-dropped",
                    path,
                    "The false JSON Schema rejects every value; no stable Pydantic annotation \
                     represents an uninhabited JSON carrier",
                );
            }
            return;
        };

        if object.contains_key("oneOf") {
            self.record(
                "one-of-validation-widened",
                &format!("{path}/oneOf"),
                "Pydantic's runtime union accepts any matching branch and cannot enforce \
                 exactly-one branch semantics",
            );
        }
        if object.contains_key("anyOf")
            && [
                "$ref",
                "enum",
                "const",
                "type",
                "properties",
                "required",
                "additionalProperties",
            ]
            .iter()
            .any(|keyword| object.contains_key(*keyword))
        {
            self.record(
                "intersection-validation-widened",
                &format!("{path}/anyOf"),
                "JSON Schema anyOf is conjunctive with its structural siblings; the emitted \
                 Pydantic union cannot prove or enforce that general intersection",
            );
        }
        if object.contains_key("allOf") {
            self.record(
                "intersection-validation-widened",
                &format!("{path}/allOf"),
                "Pydantic annotations cannot express a general JSON Schema intersection",
            );
        }
        if object.contains_key("not") {
            self.record(
                "negation-validation-dropped",
                &format!("{path}/not"),
                "Pydantic annotations cannot express general JSON Schema negation",
            );
        }
        if object.contains_key("if") || object.contains_key("then") || object.contains_key("else") {
            self.record(
                "conditional-validation-dropped",
                path,
                "Pydantic annotations cannot express JSON Schema if/then/else dependent \
                 validation",
            );
        }
        if object.contains_key("contains")
            || object.contains_key("minContains")
            || object.contains_key("maxContains")
        {
            self.record(
                "array-contains-validation-dropped",
                path,
                "Pydantic list annotations cannot enforce JSON Schema contains cardinality",
            );
        }
        if let Some(format) = object.get("format").and_then(Value::as_str)
            && !matches!(format, "date-time" | "date" | "time")
        {
            self.record(
                "format-validation-widened",
                &format!("{path}/format"),
                &format!(
                    "JSON Schema format {format:?} is retained on model_json_schema() while the \
                     runtime carrier is a strict string"
                ),
            );
        }
        if has_runtime_constraints(object) {
            let type_is_union = object.get("type").is_some_and(Value::is_array);
            let constraints_are_distributed =
                object.contains_key("anyOf") || object.contains_key("oneOf");
            let type_is_missing = !object.contains_key("type");
            if type_is_union || (type_is_missing && !constraints_are_distributed) {
                self.record(
                    "keyword-validation-dropped",
                    path,
                    "Type-conditional JSON Schema assertions cannot be attached safely to this \
                     untyped or multi-type Pydantic carrier; they remain on \
                     model_json_schema()",
                );
            }
        }

        for key in object.keys() {
            if !known_schema_keyword(key) && !is_annotation_keyword(key) {
                self.record(
                    "keyword-validation-dropped",
                    &format!("{path}/{}", pointer_escape(key)),
                    &format!(
                        "JSON Schema assertion keyword {key:?} is preserved on \
                         model_json_schema() but has no emitted runtime validator"
                    ),
                );
            }
        }

        for key in [
            "$defs",
            "properties",
            "patternProperties",
            "dependentSchemas",
        ] {
            if let Some(children) = object.get(key).and_then(Value::as_object) {
                for (child_key, child) in children {
                    self.audit_schema(
                        child,
                        &format!(
                            "{path}/{}/{}",
                            pointer_escape(key),
                            pointer_escape(child_key)
                        ),
                    );
                }
            }
        }
        for key in ["allOf", "anyOf", "oneOf", "prefixItems"] {
            if let Some(children) = object.get(key).and_then(Value::as_array) {
                for (index, child) in children.iter().enumerate() {
                    self.audit_schema(child, &format!("{path}/{key}/{index}"));
                }
            }
        }
        for key in [
            "items",
            "not",
            "if",
            "then",
            "else",
            "contains",
            "propertyNames",
            "unevaluatedProperties",
        ] {
            if let Some(child) = object.get(key)
                && matches!(child, Value::Object(_) | Value::Bool(_))
            {
                self.audit_schema(child, &format!("{path}/{key}"));
            }
        }
        if let Some(Value::Object(child)) = object.get("additionalProperties") {
            self.audit_schema(
                &Value::Object(child.clone()),
                &format!("{path}/additionalProperties"),
            );
        }
    }

    fn record(&mut self, code: &str, path: &str, note: &str) {
        self.ledger.record(LossEntry {
            code: code.to_owned().into(),
            from: "json-schema".into(),
            to: "pydantic-v2".into(),
            note: note.to_owned().into(),
            location: Some(Box::new(
                RdfLocation::logical("pydantic-emitter").with_subject(path),
            )),
        });
    }

    fn finish_models(
        &self,
        docstring: &str,
        defs_literal: &str,
        definitions: &[String],
        exports: &[String],
    ) -> String {
        let mut out = String::new();
        out.push_str(&python_string(docstring));
        out.push_str("\nfrom __future__ import annotations\n\n");
        out.push_str("from copy import deepcopy\n");
        out.push_str("from datetime import date, datetime, time\n");
        out.push_str("from enum import StrEnum\n");
        out.push_str("from typing import Annotated, Any, ClassVar, ForwardRef, Literal, Never\n\n");
        out.push_str("from pydantic import (\n");
        out.push_str("    ConfigDict,\n    Field,\n    RootModel,\n");
        out.push_str("    StrictBool,\n    StrictFloat,\n    StrictInt,\n    StrictStr,\n");
        out.push_str(")\n");
        out.push_str("from typing_extensions import NotRequired, Required, TypedDict\n\n");
        writeln!(out, "from .{BASE_MODULE} import {BASE_CLASS}\n\n")
            .expect("writing generated Python to a String cannot fail");
        writeln!(out, "_PURRDF_DEFS = {defs_literal}\n")
            .expect("writing generated Python to a String cannot fail");

        for helper in &self.helpers {
            out.push_str(helper);
            out.push_str("\n\n");
        }
        for definition in definitions {
            out.push_str(definition);
            out.push('\n');
        }

        if !exports.is_empty() {
            out.push_str("_REBUILD_NAMESPACE = dict(globals())\n");
            out.push_str("for _model in (");
            for name in exports {
                out.push_str(name);
                out.push_str(", ");
            }
            out.push_str("):\n");
            out.push_str(
                "    _model.model_rebuild(force=True, _types_namespace=_REBUILD_NAMESPACE)\n",
            );
        }
        finish_text(out)
    }
}

fn append_schema_surface(out: &mut String, schema_literal: &str) {
    writeln!(
        out,
        "    __purrdf_schema__: ClassVar[Any] = {schema_literal}"
    )
    .expect("writing generated Python to a String cannot fail");
    out.push_str("\n    @classmethod\n");
    out.push_str("    def model_json_schema(cls, **kwargs: Any) -> Any:\n");
    out.push_str("        super().model_json_schema(**kwargs)\n");
    out.push_str("        schema = deepcopy(cls.__purrdf_schema__)\n");
    out.push_str("        if isinstance(schema, dict):\n");
    out.push_str("            schema[\"$defs\"] = deepcopy(_PURRDF_DEFS)\n");
    out.push_str("        return schema\n");
}

fn properties<'a>(
    object: &'a Map<String, Value>,
    path: &str,
) -> Result<&'a Map<String, Value>, PydanticError> {
    match object.get("properties") {
        Some(Value::Object(properties)) => Ok(properties),
        Some(_) => Err(PydanticError::new(format!(
            "{path}/properties must be an object"
        ))),
        None => {
            static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
            Ok(EMPTY.get_or_init(Map::new))
        }
    }
}

fn required_names(
    object: &Map<String, Value>,
    properties: &Map<String, Value>,
    path: &str,
) -> Result<BTreeSet<String>, PydanticError> {
    let mut required = BTreeSet::new();
    let Some(values) = object.get("required") else {
        return Ok(required);
    };
    let values = values
        .as_array()
        .ok_or_else(|| PydanticError::new(format!("{path}/required must be an array")))?;
    for (index, value) in values.iter().enumerate() {
        let name = value.as_str().ok_or_else(|| {
            PydanticError::new(format!("{path}/required/{index} must be a string"))
        })?;
        if !properties.contains_key(name) {
            return Err(PydanticError::new(format!(
                "{path}/required names {name:?}, which is absent from properties"
            )));
        }
        required.insert(name.to_owned());
    }
    Ok(required)
}

fn extra_policy(object: &Map<String, Value>, path: &str) -> Result<&'static str, PydanticError> {
    match object.get("additionalProperties") {
        Some(Value::Bool(false)) => Ok("forbid"),
        Some(Value::Bool(true) | Value::Object(_)) | None => Ok("allow"),
        Some(_) => Err(PydanticError::new(format!(
            "{path}/additionalProperties must be a boolean or schema"
        ))),
    }
}

fn is_record_definition(object: &Map<String, Value>) -> bool {
    let property_count = object
        .get("properties")
        .and_then(Value::as_object)
        .map_or(0, Map::len);
    property_count > 0
        || object.get("additionalProperties") == Some(&Value::Bool(false))
        || object
            .get("required")
            .and_then(Value::as_array)
            .is_some_and(|required| !required.is_empty())
}

fn apply_constraints(base: String, object: &Map<String, Value>) -> String {
    let mut arguments = Vec::new();
    let declared_type = object.get("type").and_then(Value::as_str);
    if matches!(declared_type, Some("integer" | "number")) {
        for (keyword, pydantic) in [
            ("minimum", "ge"),
            ("maximum", "le"),
            ("exclusiveMinimum", "gt"),
            ("exclusiveMaximum", "lt"),
            ("multipleOf", "multiple_of"),
        ] {
            if let Some(value) = object.get(keyword).and_then(Value::as_number) {
                arguments.push(format!("{pydantic}={value}"));
            }
        }
    }
    if declared_type == Some("array") {
        for (keyword, pydantic) in [("minItems", "min_length"), ("maxItems", "max_length")] {
            if let Some(value) = object.get(keyword).and_then(Value::as_u64) {
                arguments.push(format!("{pydantic}={value}"));
            }
        }
    } else if declared_type == Some("string") {
        for (keyword, pydantic) in [("minLength", "min_length"), ("maxLength", "max_length")] {
            if let Some(value) = object.get(keyword).and_then(Value::as_u64) {
                arguments.push(format!("{pydantic}={value}"));
            }
        }
        if let Some(pattern) = object.get("pattern").and_then(Value::as_str) {
            arguments.push(format!("pattern={}", python_string(pattern)));
        }
    }
    if arguments.is_empty() {
        base
    } else {
        format!("Annotated[{base}, Field({})]", arguments.join(", "))
    }
}

fn branch_with_parent_constraints(branch: &Value, parent: &Map<String, Value>) -> (Value, bool) {
    let Some(branch_object) = branch.as_object() else {
        return (branch.clone(), false);
    };
    let nested_composition = branch_object.contains_key("anyOf")
        || branch_object.contains_key("oneOf")
        || branch_object.contains_key("allOf");
    let declared_type = branch_object.get("type").and_then(Value::as_str);
    let mut augmented = branch_object.clone();
    let mut conflict = false;

    for &key in runtime_constraint_keys() {
        let Some(value) = parent.get(key) else {
            continue;
        };
        let applies = nested_composition
            || match declared_type {
                Some("string") => matches!(key, "minLength" | "maxLength" | "pattern" | "format"),
                Some("array") => matches!(key, "minItems" | "maxItems"),
                Some("integer" | "number") => matches!(
                    key,
                    "minimum" | "maximum" | "exclusiveMinimum" | "exclusiveMaximum" | "multipleOf"
                ),
                _ => false,
            };
        if !applies {
            continue;
        }
        if augmented.contains_key(key) {
            conflict = true;
        } else {
            augmented.insert(key.to_owned(), value.clone());
        }
    }
    (Value::Object(augmented), conflict)
}

fn has_runtime_constraints(object: &Map<String, Value>) -> bool {
    runtime_constraint_keys()
        .iter()
        .any(|key| object.contains_key(*key))
}

fn runtime_constraint_keys() -> &'static [&'static str] {
    &[
        "minimum",
        "maximum",
        "exclusiveMinimum",
        "exclusiveMaximum",
        "multipleOf",
        "minLength",
        "maxLength",
        "pattern",
        "minItems",
        "maxItems",
        "format",
    ]
}

fn schema_map_keywords() -> &'static [&'static str] {
    &[
        "$defs",
        "properties",
        "patternProperties",
        "dependentSchemas",
    ]
}

fn schema_array_keywords() -> &'static [&'static str] {
    &["allOf", "anyOf", "oneOf", "prefixItems"]
}

fn schema_single_keywords() -> &'static [&'static str] {
    &[
        "items",
        "additionalItems",
        "unevaluatedItems",
        "additionalProperties",
        "unevaluatedProperties",
        "propertyNames",
        "contains",
        "not",
        "if",
        "then",
        "else",
        "contentSchema",
    ]
}

fn enum_object_schema(member: &Map<String, Value>) -> Value {
    let mut properties = Map::new();
    let mut required = Vec::new();
    for (key, value) in member {
        let mut constraint = Map::new();
        constraint.insert("const".to_owned(), value.clone());
        properties.insert(key.clone(), Value::Object(constraint));
        required.push(Value::String(key.clone()));
    }
    let mut schema = Map::new();
    schema.insert("type".to_owned(), Value::String("object".to_owned()));
    schema.insert("properties".to_owned(), Value::Object(properties));
    schema.insert("required".to_owned(), Value::Array(required));
    schema.insert("additionalProperties".to_owned(), Value::Bool(false));
    Value::Object(schema)
}

fn render_string_enum(name: &str, values: &[Value]) -> String {
    let mut out = format!("class {name}(StrEnum):\n");
    if values.is_empty() {
        out.push_str("    pass\n\n");
        return out;
    }
    let mut used = BTreeSet::new();
    for value in values {
        let text = value
            .as_str()
            .expect("render_string_enum is called only for string enum values");
        let stem = python_type_name(local_token(text), "Value");
        let mut member = stem.clone();
        let mut suffix = 2_u32;
        while used.contains(&member) {
            member = format!("{stem}{suffix}");
            suffix += 1;
        }
        used.insert(member.clone());
        writeln!(out, "    {member} = {}", python_string(text))
            .expect("writing generated Python to a String cannot fail");
    }
    out.push('\n');
    out
}

fn render_base(docstring: &str) -> String {
    let mut out = String::new();
    out.push_str(&python_string(docstring));
    out.push_str("\nfrom __future__ import annotations\n\n");
    out.push_str("from pydantic import BaseModel, ConfigDict\n\n\n");
    writeln!(out, "class {BASE_CLASS}(BaseModel):")
        .expect("writing generated Python to a String cannot fail");
    out.push_str("    model_config = ConfigDict(\n");
    out.push_str("        populate_by_name=True,\n");
    out.push_str("    )\n");
    finish_text(out)
}

fn render_init(docstring: &str, exports: &[String]) -> String {
    let mut out = String::new();
    out.push_str(&python_string(docstring));
    out.push_str("\nfrom __future__ import annotations\n\n");
    if !exports.is_empty() {
        writeln!(out, "from .{MODELS_MODULE} import (")
            .expect("writing generated Python to a String cannot fail");
        for name in exports {
            writeln!(out, "    {name},").expect("writing generated Python to a String cannot fail");
        }
        out.push_str(")\n\n");
    }
    out.push_str("__all__ = (\n");
    for name in exports {
        writeln!(out, "    {},", python_string(name))
            .expect("writing generated Python to a String cannot fail");
    }
    out.push_str(")\n");
    finish_text(out)
}

fn rewrite_references(
    value: &Value,
    names: &BTreeMap<String, String>,
) -> Result<Value, PydanticError> {
    let Value::Object(object) = value else {
        return Ok(value.clone());
    };
    let mut rewritten = object.clone();
    if let Some(reference) = object.get("$ref") {
        let reference = reference
            .as_str()
            .ok_or_else(|| PydanticError::new("JSON Schema $ref must be a string"))?;
        let def_key = reference_key(reference).ok_or_else(|| {
            PydanticError::new(format!(
                "external/non-$defs reference cannot be emitted: {reference:?}"
            ))
        })?;
        let class_name = names
            .get(&def_key)
            .ok_or_else(|| PydanticError::new(format!("dangling $defs reference {def_key:?}")))?;
        rewritten.insert(
            "$ref".to_owned(),
            Value::String(format!("#/$defs/{class_name}")),
        );
    }
    for keyword in schema_map_keywords() {
        if let Some(children) = object.get(*keyword).and_then(Value::as_object) {
            let mapped = children
                .iter()
                .map(|(key, child)| {
                    rewrite_references(child, names).map(|value| (key.clone(), value))
                })
                .collect::<Result<Map<_, _>, _>>()?;
            rewritten.insert((*keyword).to_owned(), Value::Object(mapped));
        }
    }
    for keyword in schema_array_keywords() {
        if let Some(children) = object.get(*keyword).and_then(Value::as_array) {
            let mapped = children
                .iter()
                .map(|child| rewrite_references(child, names))
                .collect::<Result<Vec<_>, _>>()?;
            rewritten.insert((*keyword).to_owned(), Value::Array(mapped));
        }
    }
    for keyword in schema_single_keywords() {
        if let Some(child) = object.get(*keyword) {
            rewritten.insert((*keyword).to_owned(), rewrite_references(child, names)?);
        }
    }
    Ok(Value::Object(rewritten))
}

fn reference_key(reference: &str) -> Option<String> {
    let encoded = reference.strip_prefix("#/$defs/")?;
    if encoded.contains('/') {
        return None;
    }
    pointer_unescape(encoded)
}

fn pointer_escape(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

fn pointer_unescape(value: &str) -> Option<String> {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '~' {
            out.push(ch);
            continue;
        }
        match chars.next()? {
            '0' => out.push('~'),
            '1' => out.push('/'),
            _ => return None,
        }
    }
    Some(out)
}

fn python_value(value: &Value) -> String {
    match value {
        Value::Null => "None".to_owned(),
        Value::Bool(true) => "True".to_owned(),
        Value::Bool(false) => "False".to_owned(),
        Value::Number(number) => number.to_string(),
        Value::String(text) => python_string(text),
        Value::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(python_value)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Value::Object(object) => format!(
            "{{{}}}",
            object
                .iter()
                .map(|(key, value)| format!("{}: {}", python_string(key), python_value(value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn python_string(value: &str) -> String {
    serde_json::to_string(value).expect("serializing a Rust string to JSON cannot fail")
}

fn python_field_name(raw: &str) -> String {
    let mut candidate = String::new();
    for ch in local_token(raw).chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            candidate.push(ch);
        } else if !candidate.ends_with('_') {
            candidate.push('_');
        }
    }
    candidate = candidate.trim_matches('_').to_owned();
    if candidate.is_empty() {
        candidate.push_str("field");
    }
    if candidate
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_digit())
    {
        candidate.insert_str(0, "field_");
    }
    if is_python_keyword(&candidate) || candidate.starts_with("model_") {
        candidate.push('_');
    }
    candidate
}

fn python_type_name(raw: &str, fallback: &str) -> String {
    let mut out = String::new();
    let mut capitalize = true;
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            if capitalize && ch.is_ascii_alphabetic() {
                out.push(ch.to_ascii_uppercase());
            } else {
                out.push(ch);
            }
            capitalize = false;
        } else {
            capitalize = true;
        }
    }
    if out.is_empty() {
        out.push_str(fallback);
    }
    if out
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_digit())
    {
        out.insert(0, 'N');
    }
    if is_python_keyword(&out) {
        out.push_str("Model");
    }
    out
}

fn local_token(value: &str) -> &str {
    value
        .rsplit([':', '#', '/'])
        .next()
        .filter(|part| !part.is_empty())
        .unwrap_or(value)
}

fn join_union(types: Vec<String>) -> String {
    let mut unique = Vec::new();
    let mut seen = BTreeSet::new();
    for ty in types {
        if seen.insert(ty.clone()) {
            unique.push(ty);
        }
    }
    match unique.len() {
        0 => "Any".to_owned(),
        1 => unique.pop().expect("length checked"),
        _ => unique.join(" | "),
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

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn known_schema_keyword(keyword: &str) -> bool {
    matches!(
        keyword,
        "$ref"
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
            | "additionalProperties"
            | "items"
            | "contains"
            | "minContains"
            | "maxContains"
            | "minimum"
            | "maximum"
            | "exclusiveMinimum"
            | "exclusiveMaximum"
            | "multipleOf"
            | "minLength"
            | "maxLength"
            | "pattern"
            | "minItems"
            | "maxItems"
            | "format"
    )
}

fn is_annotation_keyword(keyword: &str) -> bool {
    keyword.starts_with("x-")
        || matches!(
            keyword,
            "$schema"
                | "$id"
                | "$anchor"
                | "$comment"
                | "$defs"
                | "title"
                | "description"
                | "default"
                | "examples"
                | "deprecated"
                | "readOnly"
                | "writeOnly"
        )
}

fn is_python_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn is_python_keyword(value: &str) -> bool {
    matches!(
        value,
        "False"
            | "None"
            | "True"
            | "and"
            | "as"
            | "assert"
            | "async"
            | "await"
            | "break"
            | "class"
            | "continue"
            | "def"
            | "del"
            | "elif"
            | "else"
            | "except"
            | "finally"
            | "for"
            | "from"
            | "global"
            | "if"
            | "import"
            | "in"
            | "is"
            | "lambda"
            | "nonlocal"
            | "not"
            | "or"
            | "pass"
            | "raise"
            | "return"
            | "try"
            | "while"
            | "with"
            | "yield"
    )
}

fn reserved_type_names() -> BTreeSet<&'static str> {
    BTreeSet::from([
        BASE_CLASS,
        "Annotated",
        "Any",
        "ClassVar",
        "ConfigDict",
        "Field",
        "ForwardRef",
        "Literal",
        "Never",
        "NotRequired",
        "Required",
        "RootModel",
        "StrEnum",
        "StrictBool",
        "StrictFloat",
        "StrictInt",
        "StrictStr",
        "TypedDict",
        "date",
        "datetime",
        "time",
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::purrdf::loss::check_ledger_sound;
    use serde_json::json;

    fn compiled(schema: &Value) -> CompiledSchema {
        CompiledSchema {
            schema_json: format!(
                "{}\n",
                serde_json::to_string_pretty(&schema).expect("fixture serializes")
            ),
            openapi_json: "{}\n".to_owned(),
            losses: LossLedger::new(),
        }
    }

    fn config() -> PydanticConfig {
        PydanticConfig::new(
            "example_models",
            "Caller package documentation.",
            "Caller model documentation.",
        )
        .expect("valid config")
    }

    fn lossless_schema() -> Value {
        json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$defs": {
                "Color": {
                    "title": "Color",
                    "enum": ["ex:blue", "ex:red"]
                },
                "Person": {
                    "type": "object",
                    "description": "A caller-described person.",
                    "additionalProperties": false,
                    "properties": {
                        "@id": { "type": "string" },
                        "ex:active": { "type": "boolean" },
                        "ex:age": { "type": "integer", "minimum": 0 },
                        "ex:color": { "$ref": "#/$defs/Color" },
                        "ex:name": {
                            "type": "string",
                            "minLength": 1,
                            "pattern": "^[A-Z]"
                        },
                        "ex:score": { "type": "number", "maximum": 1.0 },
                        "ex:tags": {
                            "type": "array",
                            "items": { "type": "string" },
                            "minItems": 1
                        },
                        "ex:when": { "type": "string", "format": "date-time" },
                        "ex:value": {
                            "anyOf": [
                                { "type": "string" },
                                { "type": "integer" }
                            ]
                        }
                    },
                    "required": ["ex:name", "ex:age"]
                }
            }
        })
    }

    #[test]
    fn config_requires_caller_identity_and_docs() {
        assert!(PydanticConfig::new("", "package", "models").is_err());
        assert!(PydanticConfig::new("bad-name", "package", "models").is_err());
        assert!(PydanticConfig::new("class", "package", "models").is_err());
        assert!(PydanticConfig::new("pkg", " ", "models").is_err());
        assert!(PydanticConfig::new("pkg", "package", "\n").is_err());
        assert!(PydanticConfig::new("org.example_models", "p", "m").is_ok());
    }

    #[test]
    fn emits_deterministic_lossless_package() {
        let compiled = compiled(&lossless_schema());
        let first = emit_pydantic(&compiled, &config()).expect("emit");
        let second = emit_pydantic(&compiled, &config()).expect("emit again");
        assert_eq!(first, second);
        assert!(first.losses.is_empty(), "{}", first.losses.render_json());
        assert_eq!(
            first.model_paths.get("Person").map(String::as_str),
            Some("example_models.models.Person")
        );
        assert_eq!(first.artifacts.len(), 4);
        assert!(first.artifacts.contains_key("example_models/__init__.py"));
        assert!(first.artifacts.contains_key("example_models/_base.py"));
        assert!(first.artifacts.contains_key("example_models/models.py"));
        assert!(first.artifacts.contains_key("example_models/py.typed"));

        let models = std::str::from_utf8(&first.artifacts["example_models/models.py"])
            .expect("generated Python is UTF-8");
        assert!(models.contains("class Color(RootModel["));
        assert!(models.contains("class _ColorValue(StrEnum):"));
        assert!(models.contains("class Person(PurrdfBaseModel):"));
        assert!(models.contains("extra=\"forbid\""));
        assert!(models.contains("age: Annotated[StrictInt, Field(ge=0)]"));
        assert!(models.contains("Field(alias=\"ex:age\")"));
        assert!(models.contains("default=None, alias=\"@id\""));
        assert!(models.contains("\"$ref\": \"#/$defs/Color\""));
        assert!(!models.contains("blackcatinformatics.ca"));
        assert!(!models.contains("gmeow"));
    }

    #[test]
    fn caller_docstrings_are_the_only_module_prose() {
        let out = emit_pydantic(&compiled(&lossless_schema()), &config()).expect("emit");
        let init = std::str::from_utf8(&out.artifacts["example_models/__init__.py"]).unwrap();
        let models = std::str::from_utf8(&out.artifacts["example_models/models.py"]).unwrap();
        assert!(init.starts_with("\"Caller package documentation.\""));
        assert!(models.starts_with("\"Caller model documentation.\""));
    }

    #[test]
    fn consumes_the_public_shacl_compiler_surface() {
        let turtle = r"
            @prefix ex:  <https://example.org/> .
            @prefix sh:  <http://www.w3.org/ns/shacl#> .
            @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

            ex:PersonShape a sh:NodeShape ;
                sh:targetClass ex:Person ;
                sh:closed true ;
                sh:property [
                    sh:path ex:name ;
                    sh:datatype xsd:string ;
                    sh:minCount 1 ;
                    sh:maxCount 1 ;
                    sh:minLength 1
                ] .
        ";
        let dataset = crate::text_ingest::parse_turtle_to_dataset(turtle).expect("parse SHACL");
        let shapes = crate::shapes::from_dataset(&dataset).expect("type SHACL");
        let namespaces = crate::json_schema::Namespaces::new(
            "ex",
            &[("ex".to_owned(), "https://example.org/".to_owned())],
        )
        .expect("caller namespace");
        let compiled = crate::json_schema::compile(&shapes, &namespaces);
        let out = emit_pydantic(&compiled, &config()).expect("emit compiled schema");

        assert!(out.model_paths.contains_key("Person"));
        assert!(out.model_paths.contains_key("Node"));
        assert!(out.model_paths.contains_key("Annotation"));
        check_ledger_sound(&out.losses, "json-schema", "pydantic-v2")
            .expect("compiler-produced losses stay profile-sound");
        let models = std::str::from_utf8(&out.artifacts["example_models/models.py"]).unwrap();
        assert!(models.contains("class Person(PurrdfBaseModel):"));
        assert!(models.contains("name: Annotated[StrictStr, Field(min_length=1)] |"));
        assert!(models.contains(" = Field(alias=\"ex:name\")"));
    }

    #[test]
    fn object_enum_members_remain_object_carriers() {
        let schema = json!({
            "$defs": {
                "State": {
                    "enum": [
                        { "@id": "ex:open" },
                        { "@id": "ex:closed" }
                    ]
                },
                "Holder": {
                    "type": "object",
                    "properties": {
                        "ex:state": { "$ref": "#/$defs/State" }
                    },
                    "required": ["ex:state"]
                }
            }
        });
        let out = emit_pydantic(&compiled(&schema), &config()).expect("emit");
        let models = std::str::from_utf8(&out.artifacts["example_models/models.py"]).unwrap();
        assert!(models.contains("TypedDict("));
        assert!(models.contains("\"@id\": Required[ForwardRef(\"Literal[\\\"ex:open\\\"]\")"));
        assert!(out.losses.is_empty(), "{}", out.losses.render_json());
    }

    #[test]
    fn records_every_unprojectable_runtime_construct_soundly() {
        let schema = json!({
            "$defs": {
                "Lossy": {
                    "type": "object",
                    "properties": {
                        "ex:conditional": {
                            "if": { "type": "string" },
                            "then": { "minLength": 2 }
                        },
                        "ex:contains": {
                            "type": "array",
                            "items": { "type": "integer" },
                            "contains": { "const": 1 },
                            "minContains": 2
                        },
                        "ex:conjoined": {
                            "type": "string",
                            "anyOf": [{ "const": "a" }, { "const": "b" }]
                        },
                        "ex:format": { "type": "string", "format": "email" },
                        "ex:intersection": {
                            "allOf": [{ "type": "integer" }, { "minimum": 1 }]
                        },
                        "ex:negated": { "not": { "type": "null" } },
                        "ex:odd": { "uniqueItems": true, "type": "array" },
                        "ex:one": {
                            "oneOf": [{ "type": "integer" }, { "type": "number" }]
                        }
                    }
                }
            }
        });
        let out = emit_pydantic(&compiled(&schema), &config()).expect("emit");
        let codes: BTreeSet<&str> = out
            .losses
            .entries()
            .iter()
            .map(|entry| entry.code.as_ref())
            .collect();
        assert_eq!(
            codes,
            BTreeSet::from([
                "array-contains-validation-dropped",
                "conditional-validation-dropped",
                "format-validation-widened",
                "intersection-validation-widened",
                "keyword-validation-dropped",
                "negation-validation-dropped",
                "one-of-validation-widened",
            ])
        );
        check_ledger_sound(&out.losses, "json-schema", "pydantic-v2")
            .expect("every runtime code is in the closed profile");
        assert_eq!(
            out.losses
                .entries()
                .iter()
                .filter(|entry| entry.code.as_ref() == "intersection-validation-widened")
                .count(),
            2,
            "both allOf and anyOf-with-siblings are ledgered as intersections"
        );
        assert!(
            out.losses
                .entries()
                .iter()
                .all(|entry| entry.location.is_some())
        );
    }

    #[test]
    fn hard_fails_dangling_external_and_malformed_references() {
        for reference in ["#/$defs/Missing", "https://example.org/schema"] {
            let schema = json!({
                "$defs": {
                    "Holder": {
                        "type": "object",
                        "properties": { "ex:value": { "$ref": reference } }
                    }
                }
            });
            assert!(emit_pydantic(&compiled(&schema), &config()).is_err());
        }
        let schema = json!({
            "$defs": {
                "Holder": {
                    "type": "object",
                    "properties": { "ex:value": { "$ref": 7 } }
                }
            }
        });
        assert!(emit_pydantic(&compiled(&schema), &config()).is_err());

        for keyword in ["$dynamicRef", "$recursiveRef"] {
            let schema = json!({
                "$defs": {
                    "Holder": {
                        "type": "object",
                        "properties": {
                            "ex:value": { keyword: "#/$defs/Holder" }
                        }
                    }
                }
            });
            assert!(emit_pydantic(&compiled(&schema), &config()).is_err());
        }
    }

    #[test]
    fn hard_fails_name_collisions_and_required_drift() {
        let collision = json!({
            "$defs": {
                "a-b": { "type": "string" },
                "a_b": { "type": "string" }
            }
        });
        assert!(emit_pydantic(&compiled(&collision), &config()).is_err());

        let field_collision = json!({
            "$defs": {
                "Holder": {
                    "type": "object",
                    "properties": {
                        "a:value": { "type": "string" },
                        "b:value": { "type": "string" }
                    }
                }
            }
        });
        assert!(emit_pydantic(&compiled(&field_collision), &config()).is_err());

        let required = json!({
            "$defs": {
                "Holder": {
                    "type": "object",
                    "properties": { "ex:value": { "type": "string" } },
                    "required": ["ex:missing"]
                }
            }
        });
        assert!(emit_pydantic(&compiled(&required), &config()).is_err());
    }

    #[test]
    fn escaped_json_pointer_definition_keys_are_reversible() {
        let schema = json!({
            "$defs": {
                "path/with~token": { "type": "string" },
                "Holder": {
                    "type": "object",
                    "properties": {
                        "ex:value": { "$ref": "#/$defs/path~1with~0token" }
                    }
                }
            }
        });
        let out = emit_pydantic(&compiled(&schema), &config()).expect("emit");
        assert_eq!(
            out.model_paths.get("path/with~token").map(String::as_str),
            Some("example_models.models.PathWithToken")
        );
        let models = std::str::from_utf8(&out.artifacts["example_models/models.py"]).unwrap();
        assert!(models.contains("#/$defs/PathWithToken"));
        assert!(models.contains("\"PathWithToken\": {\"type\": \"string\"}"));
        assert!(!models.contains("\"path/with~token\": {\"type\": \"string\"}"));
    }

    #[test]
    fn ref_named_properties_and_enum_data_are_not_schema_references() {
        let schema = json!({
            "$defs": {
                "Target": { "type": "string" },
                "Holder": {
                    "type": "object",
                    "properties": {
                        "$ref": { "type": "string" },
                        "ex:choice": { "enum": [{ "$ref": "literal-data" }] },
                        "ex:target": { "$ref": "#/$defs/Target" }
                    }
                }
            }
        });
        let out = emit_pydantic(&compiled(&schema), &config()).expect("emit");
        let models = std::str::from_utf8(&out.artifacts["example_models/models.py"]).unwrap();
        assert!(models.contains("\"enum\": [{\"$ref\": \"literal-data\"}]"));
        assert!(models.contains("\"ex:target\": {\"$ref\": \"#/$defs/Target\"}"));
        assert!(models.contains("ref: StrictStr = Field(default=None, alias=\"$ref\")"));
    }
}
