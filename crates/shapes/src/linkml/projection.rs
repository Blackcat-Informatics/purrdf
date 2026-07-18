// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Capability-driven JSON Schema to LinkML 1.11 projection.

use std::collections::{BTreeMap, BTreeSet};

use ::purrdf::RdfLocation;
use ::purrdf::loss::{LossEntry, LossLedger};
use serde_json::{Map, Value};

use super::{
    LINKML_METAMODEL_VERSION, LinkmlConfig, LinkmlDocument, LinkmlError, LinkmlPackage,
    LinkmlSlotDisposition, LinkmlSlotReason, MAX_LINKML_SOURCE_KEY_BYTES, is_linkml_identifier,
    is_reserved_jsonld_slot, write_linkml,
};
use crate::json_schema::CompiledSchema;
use crate::schema_catalog::{
    CompiledSchemaCatalog, definition_path, pointer_escape, reference_key, schema_array_keywords,
    schema_map_keywords, schema_single_keywords,
};

const LOSS_FROM: &str = "json-schema";
const LOSS_TO: &str = "linkml-1.11";
const LOSS_CONTEXT: &str = "shapes:linkml";
const MAX_GENERATED_SLOT_NAME_BYTES: usize = 255;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ElementKind {
    Class,
    Enum,
    Type,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ElementInfo {
    name: String,
    kind: ElementKind,
}

pub(super) fn emit(
    compiled: &CompiledSchema,
    config: &LinkmlConfig,
) -> Result<LinkmlPackage, LinkmlError> {
    let catalog = CompiledSchemaCatalog::parse(compiled)
        .map_err(|error| LinkmlError::new(error.to_string()))?;
    let definitions = catalog.definitions();
    let elements = element_infos(definitions)?;
    let element_names: BTreeMap<String, String> = elements
        .iter()
        .map(|(key, info)| (key.clone(), info.name.clone()))
        .collect();

    let mut renderer = Renderer::new(config, &elements);
    for (key, definition) in definitions {
        renderer.audit_schema(definition, &definition_path(key))?;
    }
    for (key, definition) in definitions {
        let info = elements
            .get(key)
            .expect("element_infos covers every definition")
            .clone();
        renderer.render_definition(key, &info, definition)?;
    }

    let mut root = Map::new();
    root.insert(
        "id".to_owned(),
        Value::String(config.schema_id().to_owned()),
    );
    root.insert(
        "name".to_owned(),
        Value::String(config.schema_name().to_owned()),
    );
    root.insert(
        "description".to_owned(),
        Value::String(config.description().to_owned()),
    );
    root.insert(
        "metamodel_version".to_owned(),
        Value::String(LINKML_METAMODEL_VERSION.to_owned()),
    );
    root.insert(
        "prefixes".to_owned(),
        Value::Object(
            config
                .prefixes()
                .iter()
                .map(|(prefix, namespace)| (prefix.clone(), Value::String(namespace.clone())))
                .collect(),
        ),
    );
    root.insert(
        "default_prefix".to_owned(),
        Value::String(config.default_prefix().to_owned()),
    );
    root.insert(
        "imports".to_owned(),
        Value::Array(vec![Value::String("linkml:types".to_owned())]),
    );
    if !renderer.types.is_empty() {
        root.insert("types".to_owned(), Value::Object(renderer.types));
    }
    if !renderer.enums.is_empty() {
        root.insert("enums".to_owned(), Value::Object(renderer.enums));
    }
    if !renderer.classes.is_empty() {
        root.insert("classes".to_owned(), Value::Object(renderer.classes));
    }

    let document = LinkmlDocument::from_value(Value::Object(root))?;
    let yaml = write_linkml(&document)?;
    Ok(LinkmlPackage {
        document,
        canonical_yaml: yaml.clone(),
        yaml,
        canonical_element_names: element_names.clone(),
        element_names,
        losses: renderer.ledger,
    })
}

fn element_infos(
    definitions: &Map<String, Value>,
) -> Result<BTreeMap<String, ElementInfo>, LinkmlError> {
    let mut names = BTreeMap::new();
    let mut reverse = BTreeMap::<String, String>::new();
    for key in definitions.keys() {
        let name = element_name(key);
        if reserved_element_names().contains(&name.as_str()) {
            return Err(LinkmlError::new(format!(
                "$defs key {key:?} normalizes to reserved LinkML element name {name:?}"
            )));
        }
        if let Some(previous) = reverse.insert(name.clone(), key.clone()) {
            return Err(LinkmlError::new(format!(
                "$defs keys {previous:?} and {key:?} collide on LinkML element name {name:?}"
            )));
        }
        names.insert(key.clone(), name);
    }

    let mut kinds = BTreeMap::new();
    let mut visiting = BTreeSet::new();
    for key in definitions.keys() {
        resolve_element_kind(key, definitions, &mut kinds, &mut visiting)?;
    }

    Ok(names
        .into_iter()
        .map(|(key, name)| {
            let kind = kinds
                .get(&key)
                .copied()
                .expect("every definition kind is resolved");
            (key, ElementInfo { name, kind })
        })
        .collect())
}

fn resolve_element_kind(
    key: &str,
    definitions: &Map<String, Value>,
    resolved: &mut BTreeMap<String, ElementKind>,
    visiting: &mut BTreeSet<String>,
) -> Result<ElementKind, LinkmlError> {
    if let Some(kind) = resolved.get(key) {
        return Ok(*kind);
    }
    if !visiting.insert(key.to_owned()) {
        return Err(LinkmlError::new(format!(
            "cyclic alias-only $defs chain includes {key:?}"
        )));
    }

    let definition = definitions
        .get(key)
        .expect("resolve_element_kind receives an existing key");
    let kind = match definition {
        Value::Object(object) if is_enum_definition(object) => ElementKind::Enum,
        Value::Object(object) if is_object_schema(object) => ElementKind::Class,
        Value::Object(object) if is_alias_only(object) => {
            let reference = object
                .get("$ref")
                .and_then(Value::as_str)
                .expect("is_alias_only requires a string reference");
            let target = reference_key(reference).ok_or_else(|| {
                LinkmlError::new(format!(
                    "{} contains a non-local alias reference {reference:?}",
                    definition_path(key)
                ))
            })?;
            resolve_element_kind(&target, definitions, resolved, visiting)?
        }
        _ => ElementKind::Type,
    };

    visiting.remove(key);
    resolved.insert(key.to_owned(), kind);
    Ok(kind)
}

fn is_enum_definition(object: &Map<String, Value>) -> bool {
    object
        .get("enum")
        .and_then(Value::as_array)
        .is_some_and(|values| {
            values.iter().all(|value| {
                value.is_string()
                    || value
                        .as_object()
                        .and_then(|member| member.get("@id"))
                        .is_some_and(Value::is_string)
            })
        })
}

fn is_object_schema(object: &Map<String, Value>) -> bool {
    matches!(object.get("type"), Some(Value::String(kind)) if kind == "object")
        || [
            "properties",
            "required",
            "additionalProperties",
            "patternProperties",
            "propertyNames",
            "minProperties",
            "maxProperties",
            "dependentRequired",
            "dependentSchemas",
        ]
        .iter()
        .any(|keyword| object.contains_key(*keyword))
}

fn is_alias_only(object: &Map<String, Value>) -> bool {
    object.get("$ref").is_some_and(Value::is_string)
        && object
            .keys()
            .all(|key| key == "$ref" || is_annotation_keyword(key))
}

pub(super) fn element_name(raw: &str) -> String {
    let mut output = String::new();
    let mut capitalize = true;
    for character in raw.chars() {
        if character.is_alphanumeric() || character == '_' {
            if capitalize {
                output.extend(character.to_uppercase());
            } else {
                output.push(character);
            }
            capitalize = false;
        } else {
            capitalize = true;
        }
    }
    if output.is_empty() {
        output.push_str("SchemaElement");
    }
    if output.chars().next().is_some_and(char::is_numeric) {
        output.insert(0, 'N');
    }
    if output.len() > 120 {
        output = format!("SchemaElement{:016x}", fnv1a(raw.as_bytes()));
    }
    debug_assert!(is_linkml_identifier(&output));
    output
}

fn reserved_element_names() -> &'static [&'static str] {
    &[
        "Any",
        "Boolean",
        "Date",
        "DateOrDatetime",
        "Datetime",
        "Decimal",
        "Double",
        "Float",
        "Integer",
        "Jsonpath",
        "Jsonpointer",
        "Ncname",
        "Nodeidentifier",
        "Objectidentifier",
        "Sparqlpath",
        "String",
        "Time",
        "Uri",
        "Uriorcurie",
    ]
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SlotSourceKind {
    Reserved,
    RegisteredCurie,
    AbsoluteIri,
    Bare,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SlotNameSeed {
    source_kind: SlotSourceKind,
    source_name: String,
    direct_name: String,
    old_slot_uri: Option<String>,
    emitted_slot_uri: Option<String>,
    disposition: LinkmlSlotDisposition,
    reasons: Vec<LinkmlSlotReason>,
}

impl SlotNameSeed {
    fn requires_rename(&self) -> bool {
        self.direct_name != self.source_name
    }
}

fn slot_name_seed(config: &LinkmlConfig, source: &str) -> Result<SlotNameSeed, LinkmlError> {
    if source.len() > MAX_LINKML_SOURCE_KEY_BYTES {
        return Err(LinkmlError::new(format!(
            "JSON property name exceeds {MAX_LINKML_SOURCE_KEY_BYTES} bytes"
        )));
    }
    if is_reserved_jsonld_slot(source) {
        return Ok(SlotNameSeed {
            source_kind: SlotSourceKind::Reserved,
            source_name: source.to_owned(),
            direct_name: source.to_owned(),
            old_slot_uri: None,
            emitted_slot_uri: None,
            disposition: LinkmlSlotDisposition::IdentityPreserved,
            reasons: Vec::new(),
        });
    }

    if config.slot_rehomes().contains(source) {
        let mut reasons = vec![LinkmlSlotReason::CallerRehome];
        let local = sanitized_local(trailing_local(source), &mut reasons);
        let direct_name = bounded_curie(config.default_prefix(), &local, source, &mut reasons)?;
        return Ok(SlotNameSeed {
            source_kind: SlotSourceKind::Bare,
            source_name: source.to_owned(),
            old_slot_uri: None,
            emitted_slot_uri: Some(direct_name.clone()),
            direct_name,
            disposition: LinkmlSlotDisposition::IdentityRehomed,
            reasons,
        });
    }

    if let Some((prefix, local)) = source.split_once(':')
        && config.prefixes().contains_key(prefix)
    {
        if is_linkml_identifier(local) && source.len() <= MAX_GENERATED_SLOT_NAME_BYTES {
            return Ok(SlotNameSeed {
                source_kind: SlotSourceKind::RegisteredCurie,
                source_name: source.to_owned(),
                direct_name: source.to_owned(),
                old_slot_uri: Some(source.to_owned()),
                emitted_slot_uri: Some(source.to_owned()),
                disposition: LinkmlSlotDisposition::IdentityPreserved,
                reasons: Vec::new(),
            });
        }
        let mut reasons = Vec::new();
        let local = sanitized_local(local, &mut reasons);
        let direct_name = bounded_curie(prefix, &local, source, &mut reasons)?;
        return Ok(SlotNameSeed {
            source_kind: SlotSourceKind::RegisteredCurie,
            source_name: source.to_owned(),
            direct_name,
            old_slot_uri: Some(source.to_owned()),
            emitted_slot_uri: Some(source.to_owned()),
            disposition: LinkmlSlotDisposition::IdentityPreserved,
            reasons,
        });
    }

    if purrdf_iri::parse(source).is_ok_and(|iri| iri.has_scheme()) {
        let matched = longest_namespace_match(config, source);
        if let Some((prefix, local)) = matched
            && is_linkml_identifier(local)
            && source.len() <= MAX_GENERATED_SLOT_NAME_BYTES
        {
            return Ok(SlotNameSeed {
                source_kind: SlotSourceKind::AbsoluteIri,
                source_name: source.to_owned(),
                direct_name: source.to_owned(),
                old_slot_uri: Some(source.to_owned()),
                emitted_slot_uri: Some(format!("{prefix}:{local}")),
                disposition: LinkmlSlotDisposition::IdentityPreserved,
                reasons: Vec::new(),
            });
        }

        let mut reasons = Vec::new();
        let (prefix, local) = if let Some((prefix, local)) = matched {
            (prefix, local)
        } else {
            reasons.push(LinkmlSlotReason::UnmatchedNamespace);
            (config.default_prefix(), trailing_local(source))
        };
        let local = sanitized_local(local, &mut reasons);
        let direct_name = bounded_curie(prefix, &local, source, &mut reasons)?;
        return Ok(SlotNameSeed {
            source_kind: SlotSourceKind::AbsoluteIri,
            source_name: source.to_owned(),
            direct_name,
            old_slot_uri: Some(source.to_owned()),
            emitted_slot_uri: Some(source.to_owned()),
            disposition: LinkmlSlotDisposition::IdentityPreserved,
            reasons,
        });
    }

    if is_linkml_identifier(source) && source.len() <= MAX_GENERATED_SLOT_NAME_BYTES {
        return Ok(SlotNameSeed {
            source_kind: SlotSourceKind::Bare,
            source_name: source.to_owned(),
            direct_name: source.to_owned(),
            old_slot_uri: None,
            emitted_slot_uri: Some(format!("{}:{source}", config.default_prefix())),
            disposition: LinkmlSlotDisposition::IdentityPreserved,
            reasons: Vec::new(),
        });
    }

    let mut reasons = vec![LinkmlSlotReason::BareName];
    let local = sanitized_local(trailing_local(source), &mut reasons);
    let direct_name = bounded_curie(config.default_prefix(), &local, source, &mut reasons)?;
    Ok(SlotNameSeed {
        source_kind: SlotSourceKind::Bare,
        source_name: source.to_owned(),
        old_slot_uri: None,
        emitted_slot_uri: Some(direct_name.clone()),
        direct_name,
        disposition: LinkmlSlotDisposition::IdentityRehomed,
        reasons,
    })
}

fn longest_namespace_match<'a>(
    config: &'a LinkmlConfig,
    source: &'a str,
) -> Option<(&'a str, &'a str)> {
    config
        .prefixes()
        .iter()
        .filter_map(|(prefix, namespace)| {
            source
                .strip_prefix(namespace)
                .filter(|local| !local.is_empty())
                .map(|local| (namespace.len(), prefix.as_str(), local))
        })
        .min_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(right.1)))
        .map(|(_, prefix, local)| (prefix, local))
}

fn trailing_local(source: &str) -> &str {
    source.rsplit(['#', '/', ':']).next().unwrap_or(source)
}

fn sanitized_local(source: &str, reasons: &mut Vec<LinkmlSlotReason>) -> String {
    let mut output = String::with_capacity(source.len().saturating_add(1));
    let mut characters = source.chars();
    let Some(first) = characters.next() else {
        push_reason(reasons, LinkmlSlotReason::InvalidInitialCharacter);
        return "_".to_owned();
    };

    if is_linkml_start(first) {
        output.push(first);
    } else if is_linkml_continue(first) {
        push_reason(reasons, LinkmlSlotReason::InvalidInitialCharacter);
        output.push('_');
        output.push(first);
    } else {
        push_reason(reasons, LinkmlSlotReason::InvalidCharacter);
        output.push('_');
    }
    for character in characters {
        if is_linkml_continue(character) {
            output.push(character);
        } else {
            push_reason(reasons, LinkmlSlotReason::InvalidCharacter);
            output.push('_');
        }
    }
    debug_assert!(is_linkml_identifier(&output));
    output
}

fn bounded_curie(
    prefix: &str,
    local: &str,
    source: &str,
    reasons: &mut Vec<LinkmlSlotReason>,
) -> Result<String, LinkmlError> {
    let direct = format!("{prefix}:{local}");
    if direct.len() <= MAX_GENERATED_SLOT_NAME_BYTES {
        return Ok(direct);
    }
    push_reason(reasons, LinkmlSlotReason::LengthBound);
    let hashed = format!("{prefix}:Slot{:016x}", fnv1a(source.as_bytes()));
    if hashed.len() > MAX_GENERATED_SLOT_NAME_BYTES {
        return Err(LinkmlError::new(format!(
            "LinkML prefix {prefix:?} leaves no room within the {MAX_GENERATED_SLOT_NAME_BYTES}-byte generated slot-name limit"
        )));
    }
    Ok(hashed)
}

fn push_reason(reasons: &mut Vec<LinkmlSlotReason>, reason: LinkmlSlotReason) {
    if !reasons.contains(&reason) {
        reasons.push(reason);
        reasons.sort_unstable();
    }
}

fn is_linkml_start(character: char) -> bool {
    character == '_' || character.is_alphabetic()
}

fn is_linkml_continue(character: char) -> bool {
    character == '_'
        || character == '-'
        || character == '.'
        || character.is_alphanumeric()
        || matches!(
            character,
            '\u{300}'..='\u{36f}' | '\u{203f}'..='\u{2040}' | '\u{b7}'
        )
}

struct Renderer<'a> {
    config: &'a LinkmlConfig,
    elements: &'a BTreeMap<String, ElementInfo>,
    classes: Map<String, Value>,
    enums: Map<String, Value>,
    types: Map<String, Value>,
    ledger: LossLedger,
    recorded_losses: BTreeSet<(String, String)>,
    used_names: BTreeMap<String, String>,
    inline_classes: BTreeMap<String, String>,
    inline_enums: BTreeMap<String, String>,
}

impl<'a> Renderer<'a> {
    fn new(config: &'a LinkmlConfig, elements: &'a BTreeMap<String, ElementInfo>) -> Self {
        let mut used_names = BTreeMap::new();
        for (key, info) in elements {
            used_names.insert(info.name.clone(), definition_path(key));
        }
        for reserved in reserved_element_names() {
            used_names.insert((*reserved).to_owned(), "LinkML imported type".to_owned());
        }
        Self {
            config,
            elements,
            classes: Map::new(),
            enums: Map::new(),
            types: Map::new(),
            ledger: LossLedger::new(),
            recorded_losses: BTreeSet::new(),
            used_names,
            inline_classes: BTreeMap::new(),
            inline_enums: BTreeMap::new(),
        }
    }

    fn render_definition(
        &mut self,
        key: &str,
        info: &ElementInfo,
        definition: &Value,
    ) -> Result<(), LinkmlError> {
        let path = definition_path(key);
        match info.kind {
            ElementKind::Class => {
                let rendered = self.render_class(&info.name, Some(key), definition, &path)?;
                self.classes
                    .insert(info.name.clone(), Value::Object(rendered));
            }
            ElementKind::Enum => {
                let rendered = self.render_enum(&info.name, definition, &path)?;
                self.enums
                    .insert(info.name.clone(), Value::Object(rendered));
            }
            ElementKind::Type => {
                let rendered = self.render_type(&info.name, definition, &path)?;
                self.types
                    .insert(info.name.clone(), Value::Object(rendered));
            }
        }
        Ok(())
    }
}

impl Renderer<'_> {
    fn render_enum(
        &self,
        name: &str,
        schema: &Value,
        path: &str,
    ) -> Result<Map<String, Value>, LinkmlError> {
        let object = schema
            .as_object()
            .ok_or_else(|| LinkmlError::new(format!("{path} enum must be an object schema")))?;
        let values = object
            .get("enum")
            .and_then(Value::as_array)
            .ok_or_else(|| LinkmlError::new(format!("{path}/enum must be an array")))?;

        let mut enumeration = Map::new();
        enumeration.insert(
            "enum_uri".to_owned(),
            Value::String(self.element_curie(name)),
        );
        self.copy_element_annotations(object, &mut enumeration, path)?;

        let varnames = string_extension_array(object, "x-enum-varnames");
        let descriptions = string_extension_array(object, "x-enum-descriptions");
        let mut permissible_values = Map::new();
        for (index, value) in values.iter().enumerate() {
            let (text, meaning) = match value {
                Value::String(text) => (text.clone(), None),
                Value::Object(member) => {
                    let identifier = member.get("@id").and_then(Value::as_str).ok_or_else(|| {
                        LinkmlError::new(format!(
                            "{path}/enum/{index} object member requires a string @id for LinkML enumeration"
                        ))
                    })?;
                    (identifier.to_owned(), self.permissible_meaning(identifier))
                }
                _ => {
                    return Err(LinkmlError::new(format!(
                        "{path}/enum/{index} is not a string or @id object and cannot form a named LinkML enum"
                    )));
                }
            };
            if permissible_values.contains_key(&text) {
                return Err(LinkmlError::new(format!(
                    "{path}/enum has duplicate LinkML permissible value {text:?}"
                )));
            }
            let mut permissible = Map::new();
            if let Some(meaning) = meaning {
                permissible.insert("meaning".to_owned(), Value::String(meaning));
            }
            if let Some(title) = varnames.as_ref().and_then(|values| values.get(index)) {
                permissible.insert("title".to_owned(), Value::String((*title).to_owned()));
            }
            if let Some(description) = descriptions.as_ref().and_then(|values| values.get(index))
                && !description.trim().is_empty()
            {
                permissible.insert(
                    "description".to_owned(),
                    Value::String((*description).to_owned()),
                );
            }
            permissible_values.insert(text, Value::Object(permissible));
        }
        enumeration.insert(
            "permissible_values".to_owned(),
            Value::Object(permissible_values),
        );
        Ok(enumeration)
    }

    fn render_type(
        &mut self,
        name: &str,
        schema: &Value,
        path: &str,
    ) -> Result<Map<String, Value>, LinkmlError> {
        let mut definition = Map::new();
        definition.insert("uri".to_owned(), Value::String(self.element_curie(name)));

        let Value::Object(object) = schema else {
            self.record(
                "keyword-validation-dropped",
                path,
                "A boolean JSON Schema has no exact LinkML type definition; string is retained as a deterministic carrier",
            );
            definition.insert("typeof".to_owned(), Value::String("string".to_owned()));
            return Ok(definition);
        };

        self.copy_element_annotations(object, &mut definition, path)?;
        let base = self.type_definition_base(object, path)?;
        definition.insert("typeof".to_owned(), Value::String(base.clone()));
        self.apply_scalar_constraints(object, &mut definition, path)?;

        if let Some(values) = object.get("enum").and_then(Value::as_array) {
            let mut expressions = Vec::new();
            for (index, value) in values.iter().enumerate() {
                if let Some(expression) =
                    self.equality_expression(value, &format!("{path}/enum/{index}"), "enum member")
                {
                    expressions.push(Value::Object(expression));
                }
            }
            if !expressions.is_empty() {
                definition.insert("any_of".to_owned(), Value::Array(expressions));
            }
        }
        if let Some(value) = object.get("const")
            && let Some(expression) =
                self.equality_expression(value, &format!("{path}/const"), "const value")
        {
            definition.extend(expression);
        }

        self.apply_type_compositions(object, &mut definition, &base, path)?;
        Ok(definition)
    }

    fn type_definition_base(
        &mut self,
        object: &Map<String, Value>,
        path: &str,
    ) -> Result<String, LinkmlError> {
        if let Some(reference) = object.get("$ref") {
            let target = self
                .reference_info(reference, &format!("{path}/$ref"))?
                .clone();
            if target.kind == ElementKind::Type {
                return Ok(target.name);
            }
            self.record(
                "keyword-validation-dropped",
                &format!("{path}/$ref"),
                "A LinkML type definition cannot alias a class or enum target; string is retained as its carrier",
            );
            return Ok("string".to_owned());
        }

        if let Some(format) = object.get("format") {
            let format = format
                .as_str()
                .ok_or_else(|| LinkmlError::new(format!("{path}/format must be a string")))?;
            if let Some(range) = format_range(format) {
                return Ok(range.to_owned());
            }
        }

        match object.get("type") {
            Some(Value::String(kind)) => {
                let range = scalar_range(kind).ok_or_else(|| {
                    LinkmlError::new(format!("{path}/type names unsupported type {kind:?}"))
                })?;
                if matches!(kind.as_str(), "array" | "object" | "null") {
                    self.record(
                        "keyword-validation-dropped",
                        &format!("{path}/type"),
                        "A standalone LinkML type cannot retain this JSON container or null carrier; string is used as the deterministic fallback",
                    );
                }
                Ok(range.to_owned())
            }
            Some(Value::Array(kinds)) => {
                let mut ranges = BTreeSet::new();
                for (index, kind) in kinds.iter().enumerate() {
                    let kind = kind.as_str().ok_or_else(|| {
                        LinkmlError::new(format!("{path}/type/{index} must be a string"))
                    })?;
                    let range = scalar_range(kind).ok_or_else(|| {
                        LinkmlError::new(format!(
                            "{path}/type/{index} names unsupported type {kind:?}"
                        ))
                    })?;
                    if kind != "null" {
                        ranges.insert(range);
                    }
                }
                if ranges.len() == 1 {
                    Ok((*ranges.first().expect("length checked")).to_owned())
                } else {
                    self.record(
                        "keyword-validation-dropped",
                        &format!("{path}/type"),
                        "A named LinkML type has one base carrier and cannot retain a heterogeneous JSON Schema type union",
                    );
                    Ok(ranges.first().copied().unwrap_or("string").to_owned())
                }
            }
            Some(_) => Err(LinkmlError::new(format!(
                "{path}/type must be a string or array of strings"
            ))),
            None => {
                if object
                    .get("enum")
                    .and_then(Value::as_array)
                    .is_some_and(|values| values.iter().all(Value::is_number))
                    || object.contains_key("minimum")
                    || object.contains_key("maximum")
                    || object.contains_key("exclusiveMinimum")
                    || object.contains_key("exclusiveMaximum")
                {
                    Ok("double".to_owned())
                } else if object
                    .get("enum")
                    .and_then(Value::as_array)
                    .is_some_and(|values| values.iter().all(Value::is_boolean))
                {
                    Ok("boolean".to_owned())
                } else {
                    self.record(
                        "keyword-validation-dropped",
                        path,
                        "An unconstrained JSON Schema carrier has no LinkML Any type; string is used as the deterministic fallback",
                    );
                    Ok("string".to_owned())
                }
            }
        }
    }

    fn apply_type_compositions(
        &mut self,
        object: &Map<String, Value>,
        definition: &mut Map<String, Value>,
        base: &str,
        path: &str,
    ) -> Result<(), LinkmlError> {
        for (source, target) in [
            ("anyOf", "any_of"),
            ("oneOf", "exactly_one_of"),
            ("allOf", "all_of"),
        ] {
            let Some(branches) = object.get(source) else {
                continue;
            };
            let branches = branches
                .as_array()
                .ok_or_else(|| LinkmlError::new(format!("{path}/{source} must be an array")))?;
            let mut expressions = Vec::new();
            for (index, branch) in branches.iter().enumerate() {
                if let Some(expression) =
                    self.render_type_expression(branch, base, &format!("{path}/{source}/{index}"))?
                {
                    expressions.push(Value::Object(expression));
                }
            }
            if !expressions.is_empty() {
                definition.insert(target.to_owned(), Value::Array(expressions));
            }
        }
        if let Some(negated) = object.get("not")
            && let Some(expression) =
                self.render_type_expression(negated, base, &format!("{path}/not"))?
        {
            definition.insert(
                "none_of".to_owned(),
                Value::Array(vec![Value::Object(expression)]),
            );
        }
        Ok(())
    }

    fn render_type_expression(
        &mut self,
        schema: &Value,
        base: &str,
        path: &str,
    ) -> Result<Option<Map<String, Value>>, LinkmlError> {
        let Value::Object(object) = schema else {
            self.record(
                "keyword-validation-dropped",
                path,
                "A boolean branch has no LinkML anonymous-type expression",
            );
            return Ok(None);
        };
        if let Some(kind) = object.get("type").and_then(Value::as_str) {
            let branch_base = scalar_range(kind).ok_or_else(|| {
                LinkmlError::new(format!("{path}/type names unsupported type {kind:?}"))
            })?;
            if branch_base != base {
                self.record(
                    "keyword-validation-dropped",
                    &format!("{path}/type"),
                    "A LinkML anonymous-type expression cannot change the named type's base carrier",
                );
                return Ok(None);
            }
        }
        let mut expression = Map::new();
        self.apply_scalar_constraints(object, &mut expression, path)?;
        if let Some(value) = object.get("const")
            && let Some(equality) =
                self.equality_expression(value, &format!("{path}/const"), "const value")
        {
            expression.extend(equality);
        }
        if expression.is_empty() {
            self.record(
                "keyword-validation-dropped",
                path,
                "This JSON Schema branch has no LinkML anonymous-type constraint",
            );
            Ok(None)
        } else {
            Ok(Some(expression))
        }
    }

    fn apply_scalar_constraints(
        &self,
        object: &Map<String, Value>,
        target: &mut Map<String, Value>,
        path: &str,
    ) -> Result<(), LinkmlError> {
        if let Some(pattern) = object.get("pattern") {
            let pattern = pattern
                .as_str()
                .ok_or_else(|| LinkmlError::new(format!("{path}/pattern must be a string")))?;
            target.insert("pattern".to_owned(), Value::String(pattern.to_owned()));
        }
        for (source, output) in [
            ("minimum", "minimum_value"),
            ("maximum", "maximum_value"),
            ("exclusiveMinimum", "minimum_value"),
            ("exclusiveMaximum", "maximum_value"),
        ] {
            if let Some(value) = object.get(source) {
                if !value.is_number() {
                    return Err(LinkmlError::new(format!(
                        "{path}/{source} must be a number"
                    )));
                }
                target.insert(output.to_owned(), value.clone());
            }
        }
        Ok(())
    }

    fn equality_expression(
        &mut self,
        value: &Value,
        path: &str,
        label: &str,
    ) -> Option<Map<String, Value>> {
        let mut expression = Map::new();
        match value {
            Value::String(text) => {
                expression.insert("equals_string".to_owned(), Value::String(text.clone()));
            }
            Value::Number(number) => {
                expression.insert("equals_number".to_owned(), Value::Number(number.clone()));
            }
            _ => {
                self.record(
                    "keyword-validation-dropped",
                    path,
                    &format!("A JSON {label} of this carrier has no LinkML equality expression"),
                );
                return None;
            }
        }
        Some(expression)
    }

    fn permissible_meaning(&self, value: &str) -> Option<String> {
        if let Some((prefix, local)) = value.split_once(':')
            && self.config.prefixes().contains_key(prefix)
            && is_linkml_identifier(local)
        {
            return Some(value.to_owned());
        }
        if purrdf_iri::parse(value).is_ok_and(|iri| iri.has_scheme()) {
            return Some(value.to_owned());
        }
        None
    }

    fn render_class(
        &mut self,
        name: &str,
        source_key: Option<&str>,
        schema: &Value,
        path: &str,
    ) -> Result<Map<String, Value>, LinkmlError> {
        let mut class = Map::new();
        class.insert(
            "class_uri".to_owned(),
            Value::String(self.element_curie(name)),
        );
        if let Some(source_key) = source_key
            && source_key != name
        {
            class.insert("alias".to_owned(), Value::String(source_key.to_owned()));
        }

        let Value::Object(object) = schema else {
            self.record(
                "keyword-validation-dropped",
                path,
                "A boolean JSON Schema cannot be represented as a LinkML class; the emitted class retains only its caller-owned identity",
            );
            class.insert("extra_slots".to_owned(), extra_slots(true, None));
            return Ok(class);
        };

        self.copy_element_annotations(object, &mut class, path)?;

        if let Some(reference) = object.get("$ref") {
            let target = self.reference_info(reference, &format!("{path}/$ref"))?;
            if target.kind == ElementKind::Class {
                class.insert("is_a".to_owned(), Value::String(target.name.clone()));
            } else {
                self.record(
                    "keyword-validation-dropped",
                    &format!("{path}/$ref"),
                    "A LinkML class cannot inherit from a JSON Schema alias whose target is not a class",
                );
            }
        }

        if object
            .get("type")
            .is_some_and(|value| !matches!(value, Value::String(kind) if kind == "object"))
        {
            self.record(
                "keyword-validation-dropped",
                &format!("{path}/type"),
                "An object-shaped definition also declares a non-object JSON carrier; LinkML retains the class structure but cannot preserve that carrier intersection",
            );
        }

        let properties = object_map(object, "properties", path)?;
        let required = required_names(object, properties, path)?;
        if !properties.is_empty() {
            let mut attributes = Map::new();
            let mut slot_uris = BTreeMap::<String, String>::new();
            for (property, property_schema) in properties {
                let property_path = format!("{path}/properties/{}", pointer_escape(property));
                let mut slot = self.render_slot_expression(property_schema, &property_path)?;
                slot.insert("alias".to_owned(), Value::String(property.clone()));
                if let Some(slot_uri) = self.slot_uri(property)? {
                    if let Some(previous) = slot_uris.insert(slot_uri.clone(), property.clone()) {
                        return Err(LinkmlError::new(format!(
                            "{path}/properties keys {previous:?} and {property:?} derive the same caller-vocabulary slot URI {slot_uri:?}"
                        )));
                    }
                    slot.insert("slot_uri".to_owned(), Value::String(slot_uri));
                }
                slot.insert(
                    "required".to_owned(),
                    Value::Bool(required.contains(property.as_str())),
                );
                attributes.insert(property.clone(), Value::Object(slot));
            }
            class.insert("attributes".to_owned(), Value::Object(attributes));
        }

        let extra = match object.get("additionalProperties") {
            None | Some(Value::Bool(true)) => extra_slots(true, None),
            Some(Value::Bool(false)) => extra_slots(false, None),
            Some(schema @ Value::Object(_)) => {
                let expression =
                    self.render_slot_expression(schema, &format!("{path}/additionalProperties"))?;
                extra_slots(true, Some(expression))
            }
            Some(_) => {
                return Err(LinkmlError::new(format!(
                    "{path}/additionalProperties must be a boolean or schema"
                )));
            }
        };
        class.insert("extra_slots".to_owned(), extra);

        self.apply_class_compositions(object, &mut class, path)?;
        Ok(class)
    }

    fn apply_class_compositions(
        &mut self,
        object: &Map<String, Value>,
        class: &mut Map<String, Value>,
        path: &str,
    ) -> Result<(), LinkmlError> {
        for (source, target) in [
            ("anyOf", "any_of"),
            ("oneOf", "exactly_one_of"),
            ("allOf", "all_of"),
        ] {
            let Some(branches) = object.get(source) else {
                continue;
            };
            let branches = branches
                .as_array()
                .ok_or_else(|| LinkmlError::new(format!("{path}/{source} must be an array")))?;
            let mut expressions = Vec::with_capacity(branches.len());
            for (index, branch) in branches.iter().enumerate() {
                let branch_path = format!("{path}/{source}/{index}");
                if let Some(expression) = self.render_class_expression(branch, &branch_path)? {
                    expressions.push(Value::Object(expression));
                }
            }
            if !expressions.is_empty() {
                class.insert(target.to_owned(), Value::Array(expressions));
            }
        }
        if let Some(negated) = object.get("not")
            && let Some(expression) =
                self.render_class_expression(negated, &format!("{path}/not"))?
        {
            class.insert(
                "none_of".to_owned(),
                Value::Array(vec![Value::Object(expression)]),
            );
        }
        Ok(())
    }

    fn render_class_expression(
        &mut self,
        schema: &Value,
        path: &str,
    ) -> Result<Option<Map<String, Value>>, LinkmlError> {
        let Value::Object(object) = schema else {
            self.record(
                "keyword-validation-dropped",
                path,
                "A boolean JSON Schema branch has no LinkML anonymous-class expression",
            );
            return Ok(None);
        };

        if let Some(reference) = object.get("$ref") {
            let target = self.reference_info(reference, &format!("{path}/$ref"))?;
            if target.kind == ElementKind::Class {
                let mut expression = Map::new();
                expression.insert("is_a".to_owned(), Value::String(target.name.clone()));
                return Ok(Some(expression));
            }
        }

        if is_object_schema(object) {
            let inline = self.ensure_inline_class(schema, path)?;
            let mut expression = Map::new();
            expression.insert("is_a".to_owned(), Value::String(inline));
            return Ok(Some(expression));
        }

        let mut expression = Map::new();
        for (source, target) in [
            ("anyOf", "any_of"),
            ("oneOf", "exactly_one_of"),
            ("allOf", "all_of"),
        ] {
            let Some(branches) = object.get(source).and_then(Value::as_array) else {
                continue;
            };
            let mut nested = Vec::new();
            for (index, branch) in branches.iter().enumerate() {
                if let Some(branch) =
                    self.render_class_expression(branch, &format!("{path}/{source}/{index}"))?
                {
                    nested.push(Value::Object(branch));
                }
            }
            if !nested.is_empty() {
                expression.insert(target.to_owned(), Value::Array(nested));
            }
        }
        if let Some(negated) = object.get("not")
            && let Some(negated) = self.render_class_expression(negated, &format!("{path}/not"))?
        {
            expression.insert(
                "none_of".to_owned(),
                Value::Array(vec![Value::Object(negated)]),
            );
        }
        if expression.is_empty() {
            self.record(
                "keyword-validation-dropped",
                path,
                "This JSON Schema branch has no LinkML anonymous-class expression and is omitted from the class composition",
            );
            Ok(None)
        } else {
            Ok(Some(expression))
        }
    }

    fn ensure_inline_class(&mut self, schema: &Value, path: &str) -> Result<String, LinkmlError> {
        if let Some(name) = self.inline_classes.get(path) {
            return Ok(name.clone());
        }
        let name = self.allocate_inline_name(path, "Object")?;
        self.inline_classes.insert(path.to_owned(), name.clone());
        self.classes.insert(name.clone(), Value::Object(Map::new()));
        let rendered = self.render_class(&name, None, schema, path)?;
        self.classes.insert(name.clone(), Value::Object(rendered));
        Ok(name)
    }

    fn slot_uri(&self, property: &str) -> Result<Option<String>, LinkmlError> {
        let seed = slot_name_seed(self.config, property)?;
        match seed.source_kind {
            SlotSourceKind::Reserved => Ok(None),
            SlotSourceKind::RegisteredCurie => {
                if seed.requires_rename() {
                    return Err(LinkmlError::new(format!(
                        "JSON property {property:?} has a non-NCName CURIE local part"
                    )));
                }
                Ok(seed.emitted_slot_uri)
            }
            SlotSourceKind::AbsoluteIri => {
                if seed.requires_rename() {
                    return Err(LinkmlError::new(format!(
                        "absolute JSON property {property:?} is outside every caller-supplied LinkML prefix namespace"
                    )));
                }
                Ok(seed.emitted_slot_uri)
            }
            SlotSourceKind::Bare => {
                // Bare source identity follows the caller-default prefix rule;
                // attribute-name allocation is class-scoped.
                let local = if is_linkml_identifier(property) {
                    property.to_owned()
                } else {
                    element_name(property)
                };
                Ok(Some(format!("{}:{local}", self.config.default_prefix())))
            }
        }
    }

    fn element_curie(&self, name: &str) -> String {
        format!("{}:{name}", self.config.default_prefix())
    }

    fn allocate_inline_name(&mut self, path: &str, suffix: &str) -> Result<String, LinkmlError> {
        let name = element_name(&format!("Inline {path} {suffix}"));
        if let Some(previous) = self.used_names.get(&name) {
            return Err(LinkmlError::new(format!(
                "schema locations {previous:?} and {path:?} collide on synthesized LinkML element name {name:?}"
            )));
        }
        self.used_names.insert(name.clone(), path.to_owned());
        Ok(name)
    }

    fn reference_info(&self, value: &Value, path: &str) -> Result<&ElementInfo, LinkmlError> {
        let reference = value
            .as_str()
            .ok_or_else(|| LinkmlError::new(format!("{path} must be a string")))?;
        let key = reference_key(reference).ok_or_else(|| {
            LinkmlError::new(format!(
                "{path} is not a direct local #/$defs reference: {reference:?}"
            ))
        })?;
        self.elements
            .get(&key)
            .ok_or_else(|| LinkmlError::new(format!("{path} targets missing $defs key {key:?}")))
    }

    fn render_slot_expression(
        &mut self,
        schema: &Value,
        path: &str,
    ) -> Result<Map<String, Value>, LinkmlError> {
        let Value::Object(object) = schema else {
            self.record(
                "keyword-validation-dropped",
                path,
                if schema == &Value::Bool(false) {
                    "The false JSON Schema rejects every value and has no inhabited LinkML slot expression; string is retained as a deterministic carrier"
                } else {
                    "The unconstrained true JSON Schema has no LinkML Any range; string is retained as a deterministic carrier"
                },
            );
            return Ok(Map::from_iter([(
                "range".to_owned(),
                Value::String("string".to_owned()),
            )]));
        };

        let mut slot = Map::new();
        self.copy_element_annotations(object, &mut slot, path)?;
        let mut has_carrier = if let Some(reference) = object.get("$ref") {
            let target = self
                .reference_info(reference, &format!("{path}/$ref"))?
                .clone();
            slot.insert("range".to_owned(), Value::String(target.name));
            if target.kind == ElementKind::Class {
                slot.insert("inlined".to_owned(), Value::Bool(true));
            }
            true
        } else {
            false
        };

        if let Some(values) = object.get("enum") {
            let values = values
                .as_array()
                .ok_or_else(|| LinkmlError::new(format!("{path}/enum must be an array")))?;
            if values.iter().all(|value| {
                value.is_string()
                    || value
                        .as_object()
                        .and_then(|member| member.get("@id"))
                        .is_some_and(Value::is_string)
            }) {
                let name = self.ensure_inline_enum(schema, path)?;
                slot.insert("range".to_owned(), Value::String(name));
                has_carrier = true;
            } else {
                let mut expressions = Vec::new();
                for (index, value) in values.iter().enumerate() {
                    if let Some(expression) = self.equality_expression(
                        value,
                        &format!("{path}/enum/{index}"),
                        "enum member",
                    ) {
                        expressions.push(Value::Object(expression));
                    }
                }
                if !expressions.is_empty() {
                    conjoin_expression(&mut slot, "any_of", expressions);
                    has_carrier = true;
                }
            }
        }

        if let Some(value) = object.get("const")
            && let Some(expression) =
                self.equality_expression(value, &format!("{path}/const"), "const value")
        {
            slot.extend(expression);
            has_carrier = true;
        }

        if !has_carrier {
            match object.get("type") {
                Some(Value::String(kind)) => {
                    self.apply_slot_type(kind, object, &mut slot, path)?;
                    has_carrier = kind != "null";
                }
                Some(Value::Array(kinds)) => {
                    let mut expressions = Vec::new();
                    let mut seen = BTreeSet::new();
                    for (index, kind) in kinds.iter().enumerate() {
                        let kind = kind.as_str().ok_or_else(|| {
                            LinkmlError::new(format!("{path}/type/{index} must be a string"))
                        })?;
                        if !seen.insert(kind) {
                            return Err(LinkmlError::new(format!(
                                "{path}/type repeats type {kind:?}"
                            )));
                        }
                        let mut branch = object.clone();
                        branch.insert("type".to_owned(), Value::String(kind.to_owned()));
                        branch.remove("anyOf");
                        branch.remove("oneOf");
                        branch.remove("allOf");
                        branch.remove("not");
                        let branch_path = format!("{path}/type/{index}");
                        if kind == "null" {
                            self.record(
                                "keyword-validation-dropped",
                                &branch_path,
                                "Explicit JSON null is not a LinkML scalar range; optionality remains distinct from accepting a null value",
                            );
                            continue;
                        }
                        expressions.push(Value::Object(
                            self.render_slot_expression(&Value::Object(branch), &branch_path)?,
                        ));
                    }
                    if !expressions.is_empty() {
                        conjoin_expression(&mut slot, "any_of", expressions);
                        has_carrier = true;
                    }
                }
                Some(_) => {
                    return Err(LinkmlError::new(format!(
                        "{path}/type must be a string or array of strings"
                    )));
                }
                None if is_object_schema(object) => {
                    let inline = self.ensure_inline_class(schema, path)?;
                    slot.insert("range".to_owned(), Value::String(inline));
                    slot.insert("inlined".to_owned(), Value::Bool(true));
                    has_carrier = true;
                }
                None => {}
            }
        }

        self.apply_scalar_constraints(object, &mut slot, path)?;
        self.apply_slot_compositions(object, &mut slot, path)?;

        if !has_carrier
            && ![
                "any_of",
                "exactly_one_of",
                "all_of",
                "none_of",
                "equals_string",
                "equals_number",
            ]
            .iter()
            .any(|key| slot.contains_key(*key))
        {
            self.record(
                "keyword-validation-dropped",
                path,
                "An unconstrained JSON Schema carrier has no LinkML Any range; string is used as the deterministic fallback",
            );
            slot.insert("range".to_owned(), Value::String("string".to_owned()));
        }
        Ok(slot)
    }

    fn apply_slot_type(
        &mut self,
        kind: &str,
        object: &Map<String, Value>,
        slot: &mut Map<String, Value>,
        path: &str,
    ) -> Result<(), LinkmlError> {
        match kind {
            "null" => {
                self.record(
                    "keyword-validation-dropped",
                    &format!("{path}/type"),
                    "Explicit JSON null is not a LinkML scalar range; optionality remains distinct from accepting a null value",
                );
            }
            "object" => {
                let inline = self.ensure_inline_class(&Value::Object(object.clone()), path)?;
                slot.insert("range".to_owned(), Value::String(inline));
                slot.insert("inlined".to_owned(), Value::Bool(true));
            }
            "array" => self.apply_array(object, slot, path)?,
            scalar => {
                let mut range = scalar_range(scalar).ok_or_else(|| {
                    LinkmlError::new(format!("{path}/type names unsupported type {scalar:?}"))
                })?;
                if scalar == "string"
                    && let Some(format) = object.get("format")
                {
                    let format = format.as_str().ok_or_else(|| {
                        LinkmlError::new(format!("{path}/format must be a string"))
                    })?;
                    if let Some(formatted) = format_range(format) {
                        range = formatted;
                    }
                }
                slot.insert("range".to_owned(), Value::String(range.to_owned()));
            }
        }
        Ok(())
    }

    fn apply_array(
        &mut self,
        object: &Map<String, Value>,
        slot: &mut Map<String, Value>,
        path: &str,
    ) -> Result<(), LinkmlError> {
        let item_schema = if let Some(prefix_items) = object.get("prefixItems") {
            let prefix_items = prefix_items
                .as_array()
                .ok_or_else(|| LinkmlError::new(format!("{path}/prefixItems must be an array")))?;
            let mut branches = prefix_items.clone();
            if let Some(items) = object.get("items") {
                branches.push(items.clone());
            }
            let mut synthetic = Map::new();
            synthetic.insert("anyOf".to_owned(), Value::Array(branches));
            Value::Object(synthetic)
        } else {
            object.get("items").cloned().unwrap_or(Value::Bool(true))
        };
        let item_path = if object.contains_key("prefixItems") {
            format!("{path}/prefixItems")
        } else {
            format!("{path}/items")
        };
        let item = self.render_slot_expression(&item_schema, &item_path)?;
        for (key, value) in item {
            if !matches!(
                key.as_str(),
                "title"
                    | "description"
                    | "required"
                    | "multivalued"
                    | "minimum_cardinality"
                    | "maximum_cardinality"
                    | "list_elements_ordered"
                    | "list_elements_unique"
                    | "alias"
                    | "slot_uri"
            ) {
                slot.insert(key, value);
            }
        }
        slot.insert("multivalued".to_owned(), Value::Bool(true));
        slot.insert("list_elements_ordered".to_owned(), Value::Bool(true));
        if slot.get("inlined") == Some(&Value::Bool(true)) {
            slot.insert("inlined_as_list".to_owned(), Value::Bool(true));
        }
        for (source, target) in [
            ("minItems", "minimum_cardinality"),
            ("maxItems", "maximum_cardinality"),
        ] {
            if let Some(value) = object.get(source) {
                if value.as_u64().is_none() {
                    return Err(LinkmlError::new(format!(
                        "{path}/{source} must be a non-negative integer"
                    )));
                }
                slot.insert(target.to_owned(), value.clone());
            }
        }
        if let Some(unique) = object.get("uniqueItems") {
            let unique = unique
                .as_bool()
                .ok_or_else(|| LinkmlError::new(format!("{path}/uniqueItems must be a boolean")))?;
            slot.insert("list_elements_unique".to_owned(), Value::Bool(unique));
        }
        Ok(())
    }

    fn apply_slot_compositions(
        &mut self,
        object: &Map<String, Value>,
        slot: &mut Map<String, Value>,
        path: &str,
    ) -> Result<(), LinkmlError> {
        for (source, target) in [
            ("anyOf", "any_of"),
            ("oneOf", "exactly_one_of"),
            ("allOf", "all_of"),
        ] {
            let Some(branches) = object.get(source) else {
                continue;
            };
            let branches = branches
                .as_array()
                .ok_or_else(|| LinkmlError::new(format!("{path}/{source} must be an array")))?;
            let mut expressions = Vec::with_capacity(branches.len());
            for (index, branch) in branches.iter().enumerate() {
                expressions.push(Value::Object(
                    self.render_slot_expression(branch, &format!("{path}/{source}/{index}"))?,
                ));
            }
            conjoin_expression(slot, target, expressions);
        }
        if let Some(negated) = object.get("not") {
            let expression = self.render_slot_expression(negated, &format!("{path}/not"))?;
            conjoin_expression(slot, "none_of", vec![Value::Object(expression)]);
        }
        Ok(())
    }

    fn ensure_inline_enum(&mut self, schema: &Value, path: &str) -> Result<String, LinkmlError> {
        if let Some(name) = self.inline_enums.get(path) {
            return Ok(name.clone());
        }
        let name = self.allocate_inline_name(path, "Enum")?;
        self.inline_enums.insert(path.to_owned(), name.clone());
        let rendered = self.render_enum(&name, schema, path)?;
        self.enums.insert(name.clone(), Value::Object(rendered));
        Ok(name)
    }

    fn audit_schema(&mut self, schema: &Value, path: &str) -> Result<(), LinkmlError> {
        let Value::Object(object) = schema else {
            return if schema.is_boolean() {
                Ok(())
            } else {
                Err(LinkmlError::new(format!(
                    "{path} must be an object or boolean JSON Schema"
                )))
            };
        };

        self.validate_keyword_values(object, path)?;

        if ["if", "then", "else"]
            .iter()
            .any(|keyword| object.contains_key(*keyword))
        {
            let keyword = ["if", "then", "else"]
                .into_iter()
                .find(|keyword| object.contains_key(*keyword))
                .expect("presence checked");
            self.record(
                "conditional-validation-dropped",
                &format!("{path}/{keyword}"),
                "JSON Schema if/then/else dependent validation has no LinkML 1.11 expression",
            );
        }
        if ["contains", "minContains", "maxContains"]
            .iter()
            .any(|keyword| object.contains_key(*keyword))
        {
            let keyword = ["contains", "minContains", "maxContains"]
                .into_iter()
                .find(|keyword| object.contains_key(*keyword))
                .expect("presence checked");
            self.record(
                "array-contains-validation-dropped",
                &format!("{path}/{keyword}"),
                "LinkML list expressions retain item and list cardinality constraints but cannot enforce a contains predicate or its match count",
            );
        }
        for keyword in ["dependentRequired", "dependentSchemas"] {
            if object.contains_key(keyword) {
                self.record(
                    "dependency-validation-dropped",
                    &format!("{path}/{keyword}"),
                    "LinkML 1.11 has no cross-property dependency expression",
                );
            }
        }
        for keyword in ["exclusiveMinimum", "exclusiveMaximum"] {
            if object.contains_key(keyword) {
                self.record(
                    "exclusive-bound-validation-widened",
                    &format!("{path}/{keyword}"),
                    "The exclusive JSON Schema bound is retained as LinkML's corresponding inclusive bound",
                );
            }
        }
        if let Some(format) = object.get("format") {
            self.record(
                "format-validation-widened",
                &format!("{path}/format"),
                &format!(
                    "JSON Schema format {:?} does not have byte-identical validation semantics in LinkML 1.11",
                    format.as_str().expect("validated string")
                ),
            );
        }
        if object.contains_key("multipleOf") {
            self.record(
                "multiple-of-validation-dropped",
                &format!("{path}/multipleOf"),
                "LinkML 1.11 has no numeric divisibility expression",
            );
        }
        for keyword in ["minProperties", "maxProperties"] {
            if object.contains_key(keyword) {
                self.record(
                    "property-count-validation-dropped",
                    &format!("{path}/{keyword}"),
                    "LinkML class structure cannot constrain the total number of JSON object properties",
                );
            }
        }
        for keyword in ["minLength", "maxLength"] {
            if object.contains_key(keyword) {
                self.record(
                    "string-length-validation-dropped",
                    &format!("{path}/{keyword}"),
                    "LinkML 1.11 has no code-point string-length expression",
                );
            }
        }
        if object.contains_key("prefixItems") {
            self.record(
                "tuple-array-validation-widened",
                &format!("{path}/prefixItems"),
                "Position-specific tuple items are widened to a homogeneous LinkML list whose item expression is the union of tuple branches",
            );
        }
        for keyword in ["unevaluatedProperties", "unevaluatedItems"] {
            if object.contains_key(keyword) {
                self.record(
                    "unevaluated-validation-dropped",
                    &format!("{path}/{keyword}"),
                    "LinkML 1.11 does not expose JSON Schema applicator evaluation state",
                );
            }
        }
        if let Some(values) = object.get("enum").and_then(Value::as_array) {
            for (index, value) in values.iter().enumerate() {
                if matches!(value, Value::Array(_) | Value::Object(_)) {
                    self.record(
                        "non-scalar-enum-validation-widened",
                        &format!("{path}/enum/{index}"),
                        "A non-scalar JSON enum member is projected to a LinkML permissible-value identifier rather than its original JSON carrier",
                    );
                }
            }
        }
        if let Some(value) = object.get("const")
            && !matches!(value, Value::String(_) | Value::Number(_))
        {
            self.record(
                "keyword-validation-dropped",
                &format!("{path}/const"),
                "This JSON const carrier has no LinkML equality expression",
            );
        }

        for key in object.keys() {
            if !known_schema_keyword(key) && !is_annotation_keyword(key) {
                self.record(
                    "keyword-validation-dropped",
                    &format!("{path}/{}", pointer_escape(key)),
                    &format!(
                        "JSON Schema assertion keyword {key:?} is outside the closed LinkML 1.11 capability table"
                    ),
                );
            }
        }

        for keyword in schema_map_keywords() {
            if let Some(children) = object.get(*keyword).and_then(Value::as_object) {
                for (key, child) in children {
                    self.audit_schema(child, &format!("{path}/{keyword}/{}", pointer_escape(key)))?;
                }
            }
        }
        for keyword in schema_array_keywords() {
            if let Some(children) = object.get(*keyword).and_then(Value::as_array) {
                for (index, child) in children.iter().enumerate() {
                    self.audit_schema(child, &format!("{path}/{keyword}/{index}"))?;
                }
            }
        }
        for keyword in schema_single_keywords() {
            if let Some(child) = object.get(*keyword) {
                self.audit_schema(child, &format!("{path}/{keyword}"))?;
            }
        }
        Ok(())
    }

    fn validate_keyword_values(
        &self,
        object: &Map<String, Value>,
        path: &str,
    ) -> Result<(), LinkmlError> {
        if let Some(value) = object.get("type") {
            let mut seen = BTreeSet::new();
            match value {
                Value::String(kind) => validate_json_type(kind, &format!("{path}/type"))?,
                Value::Array(kinds) if !kinds.is_empty() => {
                    for (index, kind) in kinds.iter().enumerate() {
                        let kind = kind.as_str().ok_or_else(|| {
                            LinkmlError::new(format!("{path}/type/{index} must be a string"))
                        })?;
                        validate_json_type(kind, &format!("{path}/type/{index}"))?;
                        if !seen.insert(kind) {
                            return Err(LinkmlError::new(format!(
                                "{path}/type repeats type {kind:?}"
                            )));
                        }
                    }
                }
                Value::Array(_) => {
                    return Err(LinkmlError::new(format!(
                        "{path}/type array cannot be empty"
                    )));
                }
                _ => {
                    return Err(LinkmlError::new(format!(
                        "{path}/type must be a string or non-empty array of strings"
                    )));
                }
            }
        }
        if object.get("enum").is_some_and(|value| !value.is_array()) {
            return Err(LinkmlError::new(format!("{path}/enum must be an array")));
        }
        for keyword in ["title", "description", "pattern", "format"] {
            if object.get(keyword).is_some_and(|value| !value.is_string()) {
                return Err(LinkmlError::new(format!(
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
                return Err(LinkmlError::new(format!(
                    "{path}/{keyword} must be a number"
                )));
            }
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
            if object
                .get(keyword)
                .is_some_and(|value| value.as_u64().is_none())
            {
                return Err(LinkmlError::new(format!(
                    "{path}/{keyword} must be a non-negative integer"
                )));
            }
        }
        if object
            .get("uniqueItems")
            .is_some_and(|value| !value.is_boolean())
        {
            return Err(LinkmlError::new(format!(
                "{path}/uniqueItems must be a boolean"
            )));
        }
        if let Some(Value::Object(dependencies)) = object.get("dependentRequired") {
            for (property, values) in dependencies {
                let values = values.as_array().ok_or_else(|| {
                    LinkmlError::new(format!(
                        "{path}/dependentRequired/{} must be an array",
                        pointer_escape(property)
                    ))
                })?;
                if values.iter().any(|value| !value.is_string()) {
                    return Err(LinkmlError::new(format!(
                        "{path}/dependentRequired/{} must contain only strings",
                        pointer_escape(property)
                    )));
                }
            }
        } else if object.contains_key("dependentRequired") {
            return Err(LinkmlError::new(format!(
                "{path}/dependentRequired must be an object"
            )));
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
            to: LOSS_TO.into(),
            note: note.to_owned().into(),
            location: Some(Box::new(
                RdfLocation::logical(LOSS_CONTEXT).with_subject(path),
            )),
        });
    }

    fn copy_element_annotations(
        &self,
        source: &Map<String, Value>,
        target: &mut Map<String, Value>,
        path: &str,
    ) -> Result<(), LinkmlError> {
        for key in ["title", "description"] {
            if let Some(value) = source.get(key) {
                let text = value
                    .as_str()
                    .ok_or_else(|| LinkmlError::new(format!("{path}/{key} must be a string")))?;
                if key != "description" || !text.trim().is_empty() {
                    target.insert(key.to_owned(), Value::String(text.to_owned()));
                }
            }
        }
        Ok(())
    }
}

fn extra_slots(allowed: bool, range_expression: Option<Map<String, Value>>) -> Value {
    let mut extra = Map::new();
    extra.insert("allowed".to_owned(), Value::Bool(allowed));
    if let Some(range_expression) = range_expression {
        extra.insert(
            "range_expression".to_owned(),
            Value::Object(range_expression),
        );
    }
    Value::Object(extra)
}

fn object_map<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    path: &str,
) -> Result<&'a Map<String, Value>, LinkmlError> {
    match object.get(key) {
        Some(Value::Object(values)) => Ok(values),
        Some(_) => Err(LinkmlError::new(format!("{path}/{key} must be an object"))),
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
) -> Result<BTreeSet<String>, LinkmlError> {
    let mut required = BTreeSet::new();
    let Some(values) = object.get("required") else {
        return Ok(required);
    };
    let values = values
        .as_array()
        .ok_or_else(|| LinkmlError::new(format!("{path}/required must be an array")))?;
    for (index, value) in values.iter().enumerate() {
        let property = value
            .as_str()
            .ok_or_else(|| LinkmlError::new(format!("{path}/required/{index} must be a string")))?;
        if !properties.contains_key(property) {
            return Err(LinkmlError::new(format!(
                "{path}/required names {property:?}, absent from properties"
            )));
        }
        if !required.insert(property.to_owned()) {
            return Err(LinkmlError::new(format!(
                "{path}/required repeats property {property:?}"
            )));
        }
    }
    Ok(required)
}

fn scalar_range(kind: &str) -> Option<&'static str> {
    match kind {
        "string" => Some("string"),
        "boolean" => Some("boolean"),
        "integer" => Some("integer"),
        "number" => Some("double"),
        "object" | "array" | "null" => Some("string"),
        _ => None,
    }
}

fn format_range(format: &str) -> Option<&'static str> {
    match format {
        "date-time" => Some("datetime"),
        "date" => Some("date"),
        "time" => Some("time"),
        "uri" | "iri" | "uri-reference" | "iri-reference" => Some("uri"),
        _ => None,
    }
}

fn string_extension_array<'a>(object: &'a Map<String, Value>, key: &str) -> Option<Vec<&'a str>> {
    let values = object.get(key)?.as_array()?;
    values.iter().map(Value::as_str).collect()
}

fn validate_json_type(kind: &str, path: &str) -> Result<(), LinkmlError> {
    if matches!(
        kind,
        "null" | "boolean" | "object" | "array" | "number" | "string" | "integer"
    ) {
        Ok(())
    } else {
        Err(LinkmlError::new(format!(
            "{path} names unsupported JSON Schema type {kind:?}"
        )))
    }
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
            | "additionalProperties"
            | "dependentRequired"
            | "dependentSchemas"
            | "items"
            | "prefixItems"
            | "contains"
            | "minContains"
            | "maxContains"
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
            | "minItems"
            | "maxItems"
            | "uniqueItems"
            | "minProperties"
            | "maxProperties"
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

fn conjoin_expression(target: &mut Map<String, Value>, key: &str, values: Vec<Value>) {
    if values.is_empty() {
        return;
    }
    if key == "all_of" {
        match target.get_mut("all_of") {
            Some(Value::Array(existing)) => existing.extend(values),
            _ => {
                target.insert("all_of".to_owned(), Value::Array(values));
            }
        }
        return;
    }
    let Some(existing) = target.remove(key) else {
        target.insert(key.to_owned(), Value::Array(values));
        return;
    };

    let mut all_of = match target.remove("all_of") {
        Some(Value::Array(existing)) => existing,
        _ => Vec::new(),
    };
    all_of.push(Value::Object(Map::from_iter([(key.to_owned(), existing)])));
    all_of.push(Value::Object(Map::from_iter([(
        key.to_owned(),
        Value::Array(values),
    )])));
    target.insert("all_of".to_owned(), Value::Array(all_of));
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use ::purrdf::loss::check_ledger_sound;
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

    fn config() -> LinkmlConfig {
        LinkmlConfig::new(
            "https://example.org/schema",
            "Example-Schema",
            "Caller-owned exact projection fixture.",
            "ex",
            BTreeMap::from([
                ("ex".to_owned(), "https://example.org/".to_owned()),
                ("linkml".to_owned(), "https://w3id.org/linkml/".to_owned()),
            ]),
        )
        .expect("valid config")
    }

    #[test]
    fn slot_name_seed_separates_syntax_identity_and_caller_rehomes() {
        let config = config();

        let registered = slot_name_seed(&config, "ex:a/b").expect("registered CURIE seed");
        assert_eq!(registered.direct_name, "ex:a_b");
        assert_eq!(registered.old_slot_uri.as_deref(), Some("ex:a/b"));
        assert_eq!(registered.emitted_slot_uri.as_deref(), Some("ex:a/b"));
        assert_eq!(
            registered.disposition,
            LinkmlSlotDisposition::IdentityPreserved
        );
        assert_eq!(registered.reasons, vec![LinkmlSlotReason::InvalidCharacter]);

        let matched =
            slot_name_seed(&config, "https://example.org/name").expect("matched absolute IRI seed");
        assert!(!matched.requires_rename());
        assert_eq!(matched.emitted_slot_uri.as_deref(), Some("ex:name"));

        let unmatched = slot_name_seed(&config, "https://outside.example/definition")
            .expect("unmatched absolute IRI seed");
        assert_eq!(unmatched.direct_name, "ex:definition");
        assert_eq!(
            unmatched.emitted_slot_uri.as_deref(),
            Some("https://outside.example/definition")
        );
        assert_eq!(
            unmatched.reasons,
            vec![LinkmlSlotReason::UnmatchedNamespace]
        );

        for (source, expected) in [
            ("urn:example:part", "ex:part"),
            ("did:example:123", "ex:_123"),
            ("mailto:cat@example.org", "ex:cat_example.org"),
            ("custom:alpha/beta", "ex:beta"),
        ] {
            let seed = slot_name_seed(&config, source).expect("non-HTTP absolute IRI seed");
            assert_eq!(seed.direct_name, expected);
            assert_eq!(seed.old_slot_uri.as_deref(), Some(source));
            assert_eq!(seed.emitted_slot_uri.as_deref(), Some(source));
            assert_eq!(seed.disposition, LinkmlSlotDisposition::IdentityPreserved);
        }

        let bare = slot_name_seed(&config, "9 bad").expect("bare source seed");
        assert_eq!(bare.direct_name, "ex:_9_bad");
        assert_eq!(bare.old_slot_uri, None);
        assert_eq!(bare.emitted_slot_uri.as_deref(), Some("ex:_9_bad"));
        assert_eq!(bare.disposition, LinkmlSlotDisposition::IdentityRehomed);
        assert_eq!(
            bare.reasons,
            vec![
                LinkmlSlotReason::InvalidCharacter,
                LinkmlSlotReason::InvalidInitialCharacter,
                LinkmlSlotReason::BareName,
            ]
        );

        let reserved = slot_name_seed(&config, "@id").expect("reserved source seed");
        assert_eq!(reserved.source_kind, SlotSourceKind::Reserved);
        assert_eq!(reserved.emitted_slot_uri, None);
        let unknown = slot_name_seed(&config, "@unknown").expect("unknown at-name seed");
        assert_eq!(unknown.direct_name, "ex:_unknown");
        assert_eq!(unknown.disposition, LinkmlSlotDisposition::IdentityRehomed);

        let rehome_config = config
            .with_slot_rehomes(BTreeSet::from(["skos:definition".to_owned()]))
            .expect("caller re-home config");
        let rehomed =
            slot_name_seed(&rehome_config, "skos:definition").expect("re-homed source seed");
        assert_eq!(rehomed.direct_name, "ex:definition");
        assert_eq!(rehomed.old_slot_uri, None);
        assert_eq!(rehomed.emitted_slot_uri.as_deref(), Some("ex:definition"));
        assert_eq!(rehomed.disposition, LinkmlSlotDisposition::IdentityRehomed);
        assert_eq!(rehomed.reasons, vec![LinkmlSlotReason::CallerRehome]);
    }

    #[test]
    fn slot_name_seed_has_lexical_namespace_ties_and_bounded_names() {
        let tied = LinkmlConfig::new(
            "https://example.org/schema",
            "Example-Schema",
            "Caller-owned tie fixture.",
            "zz",
            BTreeMap::from([
                ("aa".to_owned(), "https://example.org/".to_owned()),
                ("linkml".to_owned(), "https://w3id.org/linkml/".to_owned()),
                ("zz".to_owned(), "https://example.org/".to_owned()),
            ]),
        )
        .expect("valid tied config");
        let seed = slot_name_seed(&tied, "https://example.org/name").expect("tied seed");
        assert_eq!(seed.emitted_slot_uri.as_deref(), Some("aa:name"));

        let source = format!("ex:{}", "x".repeat(MAX_GENERATED_SLOT_NAME_BYTES));
        let bounded = slot_name_seed(&config(), &source).expect("bounded seed");
        assert!(bounded.direct_name.len() <= MAX_GENERATED_SLOT_NAME_BYTES);
        assert!(bounded.direct_name.starts_with("ex:Slot"));
        assert!(bounded.reasons.contains(&LinkmlSlotReason::LengthBound));
        assert_eq!(bounded.old_slot_uri.as_deref(), Some(source.as_str()));
    }

    fn exact_schema() -> Value {
        json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$defs": {
                "Color": {
                    "title": "Color",
                    "description": "Allowed colors.",
                    "type": "string",
                    "enum": ["ex:blue", "ex:red"]
                },
                "Person": {
                    "type": "object",
                    "title": "Person",
                    "description": "A represented person.",
                    "additionalProperties": false,
                    "properties": {
                        "@id": { "type": "string" },
                        "ex:active": { "type": "boolean" },
                        "ex:age": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 130
                        },
                        "ex:child": {
                            "type": "object",
                            "additionalProperties": false,
                            "properties": {
                                "ex:label": { "type": "string" }
                            },
                            "required": ["ex:label"]
                        },
                        "ex:color": { "$ref": "#/$defs/Color" },
                        "ex:name": {
                            "type": "string",
                            "pattern": "^[A-Z]"
                        },
                        "ex:score": { "type": "number", "maximum": 1.0 },
                        "ex:tags": {
                            "type": "array",
                            "items": { "type": "string" },
                            "minItems": 1,
                            "maxItems": 3,
                            "uniqueItems": true
                        },
                        "ex:value": {
                            "anyOf": [
                                { "type": "string" },
                                { "type": "integer" }
                            ]
                        }
                    },
                    "required": ["ex:age", "ex:name"]
                },
                "PersonAlias": {
                    "$ref": "#/$defs/Person",
                    "description": "A direct class alias."
                }
            }
        })
    }

    #[test]
    fn exact_projection_is_deterministic_reversible_and_lossless() {
        let compiled = compiled(&exact_schema());
        let first = emit(&compiled, &config()).expect("emit");
        let second = emit(&compiled, &config()).expect("emit again");
        assert_eq!(first, second);
        assert!(first.losses.is_empty(), "{}", first.losses.render_json());
        check_ledger_sound(&first.losses, LOSS_FROM, LOSS_TO).expect("empty ledger is sound");
        assert_eq!(
            first.element_names,
            BTreeMap::from([
                ("Color".to_owned(), "Color".to_owned()),
                ("Person".to_owned(), "Person".to_owned()),
                ("PersonAlias".to_owned(), "PersonAlias".to_owned()),
            ])
        );

        let root = first.document.as_value();
        assert_eq!(root["classes"]["Person"]["class_uri"], "ex:Person");
        assert_eq!(root["classes"]["Person"]["extra_slots"]["allowed"], false);
        assert_eq!(
            root["classes"]["Person"]["attributes"]["ex:name"]["alias"],
            "ex:name"
        );
        assert_eq!(
            root["classes"]["Person"]["attributes"]["ex:name"]["slot_uri"],
            "ex:name"
        );
        assert_eq!(
            root["classes"]["Person"]["attributes"]["ex:age"]["required"],
            true
        );
        assert_eq!(
            root["classes"]["Person"]["attributes"]["ex:tags"]["list_elements_unique"],
            true
        );
        assert_eq!(
            root["classes"]["PersonAlias"]["is_a"],
            Value::String("Person".to_owned())
        );
        assert_eq!(root["enums"]["Color"]["enum_uri"], "ex:Color");
        let classes = root["classes"].as_object().unwrap();
        assert_eq!(classes.len(), 3);
        assert!(classes.keys().any(|name| name.starts_with("Inline")));

        let reparsed = super::super::parse_linkml(&first.yaml).expect("read emitted YAML");
        assert_eq!(reparsed, first.document);
        assert_eq!(write_linkml(&reparsed).expect("rewrite"), first.yaml);
        assert!(!first.yaml.contains("gmeow"));
        assert!(!first.yaml.contains("blackcatinformatics.ca"));
    }

    #[test]
    fn lossy_projection_records_the_closed_profile_at_exact_locations() {
        let schema = json!({
            "$defs": {
                "Lossy": {
                    "type": "object",
                    "additionalProperties": { "type": "integer" },
                    "minProperties": 1,
                    "maxProperties": 8,
                    "dependentRequired": { "ex:a": ["ex:b"] },
                    "if": { "properties": { "ex:a": { "const": true } } },
                    "then": { "required": ["ex:b"], "properties": { "ex:b": true } },
                    "unevaluatedProperties": false,
                    "propertyNames": { "pattern": "^ex:" },
                    "properties": {
                        "ex:array": {
                            "type": "array",
                            "prefixItems": [
                                { "type": "string" },
                                { "type": "integer" }
                            ],
                            "contains": { "const": 7 },
                            "minContains": 1,
                            "unevaluatedItems": false
                        },
                        "ex:choice": {
                            "enum": [
                                { "@id": "ex:open" },
                                { "@id": "ex:closed" }
                            ]
                        },
                        "ex:label": {
                            "type": "string",
                            "minLength": 2,
                            "maxLength": 12,
                            "format": "email"
                        },
                        "ex:number": {
                            "type": "number",
                            "exclusiveMinimum": 0,
                            "exclusiveMaximum": 10,
                            "multipleOf": 0.5
                        }
                    }
                }
            }
        });
        let output = emit(&compiled(&schema), &config()).expect("emit lossy schema");
        check_ledger_sound(&output.losses, LOSS_FROM, LOSS_TO)
            .expect("all losses belong to the profile");
        let codes = output
            .losses
            .entries()
            .iter()
            .map(|entry| entry.code.as_ref())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            codes,
            BTreeSet::from([
                "array-contains-validation-dropped",
                "conditional-validation-dropped",
                "dependency-validation-dropped",
                "exclusive-bound-validation-widened",
                "format-validation-widened",
                "keyword-validation-dropped",
                "multiple-of-validation-dropped",
                "non-scalar-enum-validation-widened",
                "property-count-validation-dropped",
                "string-length-validation-dropped",
                "tuple-array-validation-widened",
                "unevaluated-validation-dropped",
            ])
        );
        let rendered = output.losses.render_json();
        assert!(rendered.contains("#/$defs/Lossy/properties/ex:label/minLength"));
        assert!(rendered.contains("#/$defs/Lossy/properties/ex:number/multipleOf"));
        assert!(rendered.contains("#/$defs/Lossy/propertyNames"));
        assert_eq!(
            output.document.as_value()["classes"]["Lossy"]["extra_slots"]["range_expression"]["range"],
            "integer"
        );
    }

    #[test]
    fn expressions_and_array_contract_are_carried_on_the_public_document() {
        let schema = json!({
            "$defs": {
                "ExpressionHolder": {
                    "type": "object",
                    "properties": {
                        "ex:any": {
                            "anyOf": [
                                { "type": "string" },
                                { "type": "integer" }
                            ]
                        },
                        "ex:all": {
                            "allOf": [
                                { "type": "string" },
                                { "pattern": "^A" }
                            ]
                        },
                        "ex:exact": {
                            "oneOf": [
                                { "const": "yes" },
                                { "const": "no" }
                            ]
                        },
                        "ex:not": { "not": { "const": "forbidden" } },
                        "ex:list": {
                            "type": "array",
                            "items": { "$ref": "#/$defs/Target" },
                            "minItems": 2,
                            "maxItems": 4,
                            "uniqueItems": false
                        }
                    }
                },
                "Target": {
                    "type": "object",
                    "additionalProperties": true,
                    "properties": { "ex:name": { "type": "string" } }
                }
            }
        });
        let output = emit(&compiled(&schema), &config()).expect("emit expressions");
        let attributes = &output.document.as_value()["classes"]["ExpressionHolder"]["attributes"];
        assert!(attributes["ex:any"]["any_of"].is_array());
        assert!(attributes["ex:all"]["all_of"].is_array());
        assert!(attributes["ex:exact"]["exactly_one_of"].is_array());
        assert!(attributes["ex:not"]["none_of"].is_array());
        assert_eq!(attributes["ex:list"]["range"], "Target");
        assert_eq!(attributes["ex:list"]["multivalued"], true);
        assert_eq!(attributes["ex:list"]["inlined_as_list"], true);
        assert_eq!(attributes["ex:list"]["minimum_cardinality"], 2);
        assert_eq!(attributes["ex:list"]["maximum_cardinality"], 4);
        assert_eq!(attributes["ex:list"]["list_elements_ordered"], true);
        assert_eq!(attributes["ex:list"]["list_elements_unique"], false);
    }

    #[test]
    fn malformed_names_requiredness_and_vocabularies_fail_closed() {
        let collision = json!({
            "$defs": {
                "a-b": { "type": "string" },
                "a b": { "type": "string" }
            }
        });
        assert!(
            emit(&compiled(&collision), &config())
                .unwrap_err()
                .to_string()
                .contains("collide")
        );

        let reserved = json!({ "$defs": { "string": { "type": "string" } } });
        assert!(
            emit(&compiled(&reserved), &config())
                .unwrap_err()
                .to_string()
                .contains("reserved")
        );

        let missing_required = json!({
            "$defs": {
                "Broken": {
                    "type": "object",
                    "properties": {},
                    "required": ["ex:missing"]
                }
            }
        });
        assert!(
            emit(&compiled(&missing_required), &config())
                .unwrap_err()
                .to_string()
                .contains("absent")
        );

        let unknown_prefix = json!({
            "$defs": {
                "Broken": {
                    "type": "object",
                    "properties": { "other:value": { "type": "string" } }
                }
            }
        });
        assert!(
            emit(&compiled(&unknown_prefix), &config())
                .unwrap_err()
                .to_string()
                .contains("prefix")
        );

        let malformed_type = json!({ "$defs": { "Broken": { "type": [] } } });
        assert!(
            emit(&compiled(&malformed_type), &config())
                .unwrap_err()
                .to_string()
                .contains("cannot be empty")
        );
    }

    #[test]
    fn absolute_property_uses_the_longest_matching_caller_prefix() {
        let config = LinkmlConfig::new(
            "https://example.org/schema",
            "Example-Schema",
            "Caller-owned longest-prefix fixture.",
            "ex",
            BTreeMap::from([
                ("ex".to_owned(), "https://example.org/".to_owned()),
                (
                    "people".to_owned(),
                    "https://example.org/people/".to_owned(),
                ),
                ("linkml".to_owned(), "https://w3id.org/linkml/".to_owned()),
            ]),
        )
        .expect("valid config");
        let schema = json!({
            "$defs": {
                "Person": {
                    "type": "object",
                    "properties": {
                        "https://example.org/people/name": { "type": "string" }
                    }
                }
            }
        });

        let output = emit(&compiled(&schema), &config).expect("emit absolute property");
        assert_eq!(
            output.document.as_value()["classes"]["Person"]["attributes"]["https://example.org/people/name"]
                ["slot_uri"],
            "people:name"
        );
    }
}
