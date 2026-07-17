// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Native LinkML 1.11 to the shared deterministic schema-import model.

use std::collections::{BTreeMap, BTreeSet};

use ::purrdf::RdfLocation;
use ::purrdf::loss::{LossEntry, LossLedger, check_ledger_sound, schema_to_shacl_loss_ledger};
use serde_json::{Map, Value};

use super::{LinkmlDocument, LinkmlError, LinkmlPackage, parse_linkml, projection, write_linkml};
use crate::schema_import::{ImportedShapes, SchemaImportConfig, import_schema_value_from};
use crate::term::Term;

const SOURCE: &str = "linkml-1.11";
const TARGET: &str = "shacl";
const LOSS_CONTEXT: &str = "shapes:linkml-import";
const MAX_ELEMENTS: usize = 65_536;
const MAX_ELEMENT_DEPTH: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ElementKind {
    Class,
    Enum,
    Type,
}

pub(super) fn import_package(
    package: &LinkmlPackage,
    config: &SchemaImportConfig,
) -> Result<ImportedShapes, LinkmlError> {
    let canonical = write_linkml(&package.document)?;
    if canonical != package.yaml {
        return Err(LinkmlError::new(
            "LinkML package YAML differs from its canonical validated document",
        ));
    }
    if parse_linkml(&package.yaml)? != package.document {
        return Err(LinkmlError::new(
            "LinkML package YAML does not parse back to its validated document",
        ));
    }
    let visible = verify_element_names(package)?;
    import_document(&package.document, config, Some(&visible))
}

pub(super) fn import_document(
    document: &LinkmlDocument,
    config: &SchemaImportConfig,
    visible_classes: Option<&BTreeSet<String>>,
) -> Result<ImportedShapes, LinkmlError> {
    let mut importer = NativeImporter::new(document)?;
    let (pivot, ignored_class_iris) = importer.build_pivot(visible_classes)?;
    let mut imported = import_schema_value_from(SOURCE, &pivot, config)
        .map_err(|error| LinkmlError::new(format!("LinkML import model: {error}")))?;

    if !ignored_class_iris.is_empty() {
        imported.shapes.node_shapes.retain(|shape| {
            let Term::NamedNode(identity) = &shape.id else {
                return true;
            };
            !ignored_class_iris.contains(identity.as_str())
        });
    }

    let mut losses = importer.losses;
    for entry in imported.losses.entries() {
        let mut remapped = entry.clone();
        if let Some(location) = remapped.location.as_mut() {
            if let Some(subject) = location.subject.as_deref() {
                location.subject = Some(remap_location(subject, &importer.locations));
            }
            location.logical = Some(LOSS_CONTEXT.to_owned());
        }
        losses.record(remapped);
    }
    check_ledger_sound(&losses, SOURCE, TARGET).map_err(LinkmlError::new)?;
    imported.losses = losses;
    Ok(imported)
}

fn verify_element_names(package: &LinkmlPackage) -> Result<BTreeSet<String>, LinkmlError> {
    if package.yaml != package.canonical_yaml {
        return Err(LinkmlError::new(
            "LinkML package YAML differs from its retained emitted artifact",
        ));
    }
    if package.element_names != package.canonical_element_names {
        return Err(LinkmlError::new(
            "LinkML package element map differs from its retained emitted map",
        ));
    }
    let root = package
        .document
        .as_value()
        .as_object()
        .expect("LinkmlDocument validates an object root");
    let mut available = BTreeSet::new();
    for section in ["classes", "enums", "types"] {
        if let Some(elements) = root.get(section).and_then(Value::as_object) {
            available.extend(elements.keys().cloned());
        }
    }
    let mut reverse = BTreeMap::new();
    for (source_key, element_name) in &package.element_names {
        if projection::element_name(source_key) != *element_name {
            return Err(LinkmlError::new(format!(
                "LinkML package source key {source_key:?} does not normalize to element {element_name:?}"
            )));
        }
        if !available.contains(element_name) {
            return Err(LinkmlError::new(format!(
                "LinkML package element map targets missing element {element_name:?}"
            )));
        }
        if let Some(previous) = reverse.insert(element_name.clone(), source_key.clone()) {
            return Err(LinkmlError::new(format!(
                "LinkML package source keys {previous:?} and {source_key:?} target the same element {element_name:?}"
            )));
        }
        if let Some(class) = root
            .get("classes")
            .and_then(Value::as_object)
            .and_then(|classes| classes.get(element_name))
            .and_then(Value::as_object)
        {
            let alias = class.get("alias").and_then(Value::as_str);
            if source_key != element_name && alias != Some(source_key.as_str()) {
                return Err(LinkmlError::new(format!(
                    "LinkML package class {element_name:?} does not retain source alias {source_key:?}"
                )));
            }
            if alias.is_some_and(|alias| alias != source_key) {
                return Err(LinkmlError::new(format!(
                    "LinkML package class {element_name:?} alias disagrees with its source map"
                )));
            }
        }
    }
    Ok(reverse.into_keys().collect())
}

struct NativeImporter {
    root: Map<String, Value>,
    prefixes: BTreeMap<String, String>,
    default_prefix: String,
    classes: Map<String, Value>,
    enums: Map<String, Value>,
    types: Map<String, Value>,
    slots: Map<String, Value>,
    kinds: BTreeMap<String, ElementKind>,
    identities: BTreeMap<String, String>,
    reverse_identities: BTreeMap<String, String>,
    class_cache: BTreeMap<String, Value>,
    type_cache: BTreeMap<String, Value>,
    ignored_class_names: BTreeSet<String>,
    used_slots: BTreeSet<String>,
    locations: BTreeMap<String, String>,
    contract: LossLedger,
    losses: LossLedger,
    recorded_losses: BTreeSet<(String, String)>,
}

impl NativeImporter {
    fn new(document: &LinkmlDocument) -> Result<Self, LinkmlError> {
        let root = document
            .as_value()
            .as_object()
            .expect("LinkmlDocument validates an object root")
            .clone();
        let prefixes = document_prefixes(&root)?;
        let default_prefix =
            required_string(&root, "default_prefix", "#/default_prefix")?.to_owned();
        let classes = section(&root, "classes")?.clone();
        let enums = section(&root, "enums")?.clone();
        let types = section(&root, "types")?.clone();
        let slots = section(&root, "slots")?.clone();
        let count = classes.len() + enums.len() + types.len() + slots.len();
        if count > MAX_ELEMENTS {
            return Err(LinkmlError::new(format!(
                "LinkML document contains {count} elements; limit is {MAX_ELEMENTS}"
            )));
        }
        let mut importer = Self {
            root,
            prefixes,
            default_prefix,
            classes,
            enums,
            types,
            slots,
            kinds: BTreeMap::new(),
            identities: BTreeMap::new(),
            reverse_identities: BTreeMap::new(),
            class_cache: BTreeMap::new(),
            type_cache: BTreeMap::new(),
            ignored_class_names: BTreeSet::new(),
            used_slots: BTreeSet::new(),
            locations: BTreeMap::new(),
            contract: schema_to_shacl_loss_ledger(SOURCE),
            losses: LossLedger::new(),
            recorded_losses: BTreeSet::new(),
        };
        importer.prepare_elements()?;
        Ok(importer)
    }

    fn prepare_elements(&mut self) -> Result<(), LinkmlError> {
        for (kind, section_name, elements, identity_field) in [
            (
                ElementKind::Class,
                "classes",
                self.classes.clone(),
                "class_uri",
            ),
            (ElementKind::Enum, "enums", self.enums.clone(), "enum_uri"),
            (ElementKind::Type, "types", self.types.clone(), "uri"),
        ] {
            for (name, value) in elements {
                let path = element_path(section_name, &name);
                let object = value
                    .as_object()
                    .ok_or_else(|| LinkmlError::new(format!("{path} must be a mapping")))?;
                if self.kinds.insert(name.clone(), kind).is_some() {
                    return Err(LinkmlError::new(format!(
                        "LinkML element name {name:?} occurs in more than one section"
                    )));
                }
                let identity = object
                    .get(identity_field)
                    .map(|value| {
                        value.as_str().ok_or_else(|| {
                            LinkmlError::new(format!("{path}/{identity_field} must be a string"))
                        })
                    })
                    .transpose()?
                    .unwrap_or(&name);
                let identity = self.expand_iri(identity, &path)?;
                if let Some(previous) = self
                    .reverse_identities
                    .insert(identity.clone(), format!("{section_name}/{name}"))
                {
                    return Err(LinkmlError::new(format!(
                        "LinkML elements {previous:?} and {section_name}/{name} resolve to the same IRI {identity:?}"
                    )));
                }
                self.identities.insert(name, identity);
            }
        }
        Ok(())
    }

    fn build_pivot(
        &mut self,
        visible_classes: Option<&BTreeSet<String>>,
    ) -> Result<(Value, BTreeSet<String>), LinkmlError> {
        self.audit_root()?;
        let mut definitions = Map::new();
        let ignored_class_names = visible_classes.map_or_else(BTreeSet::new, |visible| {
            self.classes
                .keys()
                .filter(|name| !visible.contains(*name) || self.is_package_infrastructure(name))
                .cloned()
                .collect()
        });
        self.ignored_class_names.clone_from(&ignored_class_names);

        for (name, _) in self.enums.clone() {
            let identity = self.identity(&name)?.to_owned();
            let schema = self.enum_schema(&name)?;
            self.map_location(&definition_path(&identity), &element_path("enums", &name));
            definitions.insert(identity, schema);
        }
        for (name, _) in self.types.clone() {
            let identity = self.identity(&name)?.to_owned();
            let schema = self.type_schema(&name, &mut BTreeSet::new(), 0)?;
            self.map_location(&definition_path(&identity), &element_path("types", &name));
            definitions.insert(identity, schema);
        }
        for (name, _) in self.classes.clone() {
            let identity = self.identity(&name)?.to_owned();
            let schema = self.class_schema(&name, &mut BTreeSet::new(), 0)?;
            if ignored_class_names.contains(&name) {
                continue;
            }
            self.map_location(&definition_path(&identity), &element_path("classes", &name));
            definitions.insert(identity, schema);
        }

        for (name, _) in self.slots.clone() {
            if !self.used_slots.contains(&name) {
                self.record(
                    "non-object-definition-dropped",
                    &element_path("slots", &name),
                );
            }
        }

        let ignored_class_iris = ignored_class_names
            .iter()
            .filter_map(|name| self.identities.get(name).cloned())
            .collect();
        Ok((
            Value::Object(Map::from_iter([(
                "$defs".to_owned(),
                Value::Object(definitions),
            )])),
            ignored_class_iris,
        ))
    }

    fn audit_root(&mut self) -> Result<(), LinkmlError> {
        for key in ["id", "name"] {
            if self.root.contains_key(key) {
                self.record(
                    "schema-identity-dropped",
                    &format!("#/{}", pointer_escape(key)),
                );
            }
        }
        if self.root.contains_key("description") {
            self.record("annotation-dropped", "#/description");
        }
        if let Some(imports) = self.root.get("imports") {
            let values: Vec<String> = match imports {
                Value::String(value) => vec![value.clone()],
                Value::Array(values) => values
                    .iter()
                    .map(|value| {
                        value
                            .as_str()
                            .map(str::to_owned)
                            .ok_or_else(|| LinkmlError::new("#/imports must contain only strings"))
                    })
                    .collect::<Result<_, _>>()?,
                _ => {
                    return Err(LinkmlError::new(
                        "#/imports must be a string or string array",
                    ));
                }
            };
            for (index, import) in values.into_iter().enumerate() {
                if import != "linkml:types" {
                    self.record("schema-identity-dropped", &format!("#/imports/{index}"));
                }
            }
        }
        if let Some(prefixes) = self
            .root
            .get("prefixes")
            .and_then(Value::as_object)
            .cloned()
        {
            for (prefix, definition) in prefixes {
                let Some(definition) = definition.as_object() else {
                    continue;
                };
                for key in definition.keys() {
                    if !matches!(key.as_str(), "prefix_prefix" | "prefix_reference") {
                        self.record(
                            "unknown-keyword-dropped",
                            &format!(
                                "#/prefixes/{}/{}",
                                pointer_escape(&prefix),
                                pointer_escape(key)
                            ),
                        );
                    }
                }
            }
        }
        for (key, _) in self.root.clone() {
            if !matches!(
                key.as_str(),
                "id" | "name"
                    | "description"
                    | "metamodel_version"
                    | "prefixes"
                    | "default_prefix"
                    | "imports"
                    | "classes"
                    | "enums"
                    | "types"
                    | "slots"
            ) {
                self.record(
                    "unknown-keyword-dropped",
                    &format!("#/{}", pointer_escape(&key)),
                );
            }
        }
        Ok(())
    }

    fn class_schema(
        &mut self,
        name: &str,
        visiting: &mut BTreeSet<String>,
        depth: usize,
    ) -> Result<Value, LinkmlError> {
        if let Some(schema) = self.class_cache.get(name) {
            return Ok(schema.clone());
        }
        self.enter_element("class", name, visiting, depth)?;
        let path = element_path("classes", name);
        let object = self
            .classes
            .get(name)
            .and_then(Value::as_object)
            .ok_or_else(|| LinkmlError::new(format!("{path} must be a mapping")))?
            .clone();

        let mut schema = Map::from_iter([("type".to_owned(), Value::String("object".to_owned()))]);
        let mut properties = Map::new();
        let mut required = BTreeSet::new();

        let mut local_slots = BTreeMap::<String, (Map<String, Value>, String)>::new();
        if let Some(names) = object.get("slots") {
            let names = names
                .as_array()
                .ok_or_else(|| LinkmlError::new(format!("{path}/slots must be an array")))?;
            for (index, slot_name) in names.iter().enumerate() {
                let slot_name = slot_name.as_str().ok_or_else(|| {
                    LinkmlError::new(format!("{path}/slots/{index} must be a string"))
                })?;
                let slot = self
                    .slots
                    .get(slot_name)
                    .and_then(Value::as_object)
                    .ok_or_else(|| {
                        LinkmlError::new(format!(
                            "{path}/slots/{index} targets missing global slot {slot_name:?}"
                        ))
                    })?
                    .clone();
                self.used_slots.insert(slot_name.to_owned());
                if local_slots
                    .insert(
                        slot_name.to_owned(),
                        (slot, element_path("slots", slot_name)),
                    )
                    .is_some()
                {
                    return Err(LinkmlError::new(format!(
                        "{path}/slots contains duplicate slot {slot_name:?}"
                    )));
                }
            }
        }
        if let Some(usages) = object.get("slot_usage") {
            let usages = usages
                .as_object()
                .ok_or_else(|| LinkmlError::new(format!("{path}/slot_usage must be a mapping")))?;
            for (slot_name, usage) in usages {
                let usage = usage.as_object().ok_or_else(|| {
                    LinkmlError::new(format!(
                        "{path}/slot_usage/{} must be a mapping",
                        pointer_escape(slot_name)
                    ))
                })?;
                let usage_path = format!("{path}/slot_usage/{}", pointer_escape(slot_name));
                if let Some((slot, slot_path)) = local_slots.get_mut(slot_name) {
                    slot.extend(usage.clone());
                    *slot_path = usage_path;
                } else if object.contains_key("is_a") || object.contains_key("mixins") {
                    // LinkML permits slot_usage to override an inherited slot,
                    // but its inherited slot_uri is not necessarily derivable
                    // from this local name. Validate the complete expression,
                    // retain the ancestor contract, and ledger the override
                    // rather than inventing a predicate identity.
                    let _ = self.slot_schema(usage, &usage_path, visiting, depth + 1)?;
                    self.record("schema-applicator-dropped", &usage_path);
                } else {
                    return Err(LinkmlError::new(format!(
                        "{path}/slot_usage names {slot_name:?}, absent from this class's slots or ancestors"
                    )));
                }
            }
        }
        if let Some(attributes) = object.get("attributes") {
            let attributes = attributes
                .as_object()
                .ok_or_else(|| LinkmlError::new(format!("{path}/attributes must be a mapping")))?;
            for (slot_name, slot) in attributes {
                let slot = slot.as_object().ok_or_else(|| {
                    LinkmlError::new(format!(
                        "{path}/attributes/{} must be a mapping",
                        pointer_escape(slot_name)
                    ))
                })?;
                if local_slots
                    .insert(
                        slot_name.clone(),
                        (
                            slot.clone(),
                            format!("{path}/attributes/{}", pointer_escape(slot_name)),
                        ),
                    )
                    .is_some()
                {
                    return Err(LinkmlError::new(format!(
                        "{path} defines slot {slot_name:?} in both slots and attributes"
                    )));
                }
            }
        }

        let mut property_identities = BTreeMap::new();
        for (slot_name, (slot, slot_path)) in local_slots {
            if matches!(
                slot_name.as_str(),
                "@id" | "@type" | "@annotation" | "@value" | "@language"
            ) {
                self.record("value-term-kind-widened", &slot_path);
                continue;
            }
            let (property_schema, is_required) =
                self.slot_schema(&slot, &slot_path, visiting, depth + 1)?;
            let property_identity = self.slot_identity(&slot_name, &slot, &slot_path)?;
            if let Some(previous) =
                property_identities.insert(property_identity.clone(), slot_path.clone())
            {
                return Err(LinkmlError::new(format!(
                    "{slot_path} and {previous} resolve to the same slot IRI {property_identity:?}"
                )));
            }
            let pivot_path = format!(
                "{}/properties/{}",
                definition_path(self.identity(name)?),
                pointer_escape(&property_identity)
            );
            self.map_location(&pivot_path, &slot_path);
            self.map_expression_locations(&pivot_path, &slot_path, &slot, &property_schema);
            for (native, schema_keyword) in [
                ("minimum_cardinality", "minItems"),
                ("maximum_cardinality", "maxItems"),
                ("list_elements_unique", "uniqueItems"),
            ] {
                if slot.contains_key(native) {
                    self.map_location(
                        &format!("{pivot_path}/{schema_keyword}"),
                        &format!("{slot_path}/{native}"),
                    );
                }
            }
            properties.insert(property_identity.clone(), property_schema);
            if is_required {
                required.insert(property_identity);
            }
        }
        schema.insert("properties".to_owned(), Value::Object(properties));
        if !required.is_empty() {
            schema.insert(
                "required".to_owned(),
                Value::Array(required.into_iter().map(Value::String).collect()),
            );
            self.map_location(
                &format!("{}/required", definition_path(self.identity(name)?)),
                &format!("{path}/attributes"),
            );
        }

        if let Some(extra) = object.get("extra_slots") {
            let extra = extra
                .as_object()
                .ok_or_else(|| LinkmlError::new(format!("{path}/extra_slots must be a mapping")))?;
            let allowed = required_bool(extra, "allowed", &format!("{path}/extra_slots/allowed"))?;
            if let Some(expression) = extra.get("range_expression") {
                if !allowed {
                    return Err(LinkmlError::new(format!(
                        "{path}/extra_slots cannot carry range_expression when allowed is false"
                    )));
                }
                let expression = expression.as_object().ok_or_else(|| {
                    LinkmlError::new(format!(
                        "{path}/extra_slots/range_expression must be a mapping"
                    ))
                })?;
                let expression_path = format!("{path}/extra_slots/range_expression");
                schema.insert(
                    "additionalProperties".to_owned(),
                    self.expression_schema(expression, &expression_path, visiting, depth + 1)?,
                );
                self.audit_expression_fields(expression, &expression_path)?;
            } else {
                schema.insert("additionalProperties".to_owned(), Value::Bool(allowed));
            }
            for key in extra.keys() {
                if !matches!(key.as_str(), "allowed" | "range_expression") {
                    self.record(
                        "unknown-keyword-dropped",
                        &format!("{path}/extra_slots/{}", pointer_escape(key)),
                    );
                }
            }
            self.map_location(
                &format!(
                    "{}/additionalProperties",
                    definition_path(self.identity(name)?)
                ),
                &format!("{path}/extra_slots"),
            );
        }

        let mut inherited = Vec::new();
        if let Some(parent) = object.get("is_a") {
            let parent = parent
                .as_str()
                .ok_or_else(|| LinkmlError::new(format!("{path}/is_a must be a string")))?;
            inherited.push(self.class_schema(parent, visiting, depth + 1)?);
            self.record("schema-identity-dropped", &format!("{path}/is_a"));
        }
        if let Some(mixins) = object.get("mixins") {
            let mixins = mixins
                .as_array()
                .ok_or_else(|| LinkmlError::new(format!("{path}/mixins must be an array")))?;
            for (index, mixin) in mixins.iter().enumerate() {
                let mixin = mixin.as_str().ok_or_else(|| {
                    LinkmlError::new(format!("{path}/mixins/{index} must be a string"))
                })?;
                inherited.push(self.class_schema(mixin, visiting, depth + 1)?);
                self.record("schema-identity-dropped", &format!("{path}/mixins/{index}"));
            }
        }
        self.apply_compositions(&object, &path, visiting, depth, &mut schema, true)?;
        if !inherited.is_empty() {
            let existing = schema
                .remove("allOf")
                .and_then(|value| value.as_array().cloned())
                .unwrap_or_default();
            inherited.extend(existing);
            schema.insert("allOf".to_owned(), Value::Array(inherited));
        }

        self.audit_element_fields(
            &object,
            &path,
            &[
                "class_uri",
                "alias",
                "title",
                "description",
                "slots",
                "slot_usage",
                "attributes",
                "extra_slots",
                "is_a",
                "mixins",
                "any_of",
                "all_of",
                "exactly_one_of",
                "none_of",
            ],
        )?;
        self.map_expression_locations(
            &definition_path(self.identity(name)?),
            &path,
            &object,
            &Value::Object(schema.clone()),
        );
        visiting.remove(&format!("class:{name}"));
        let value = Value::Object(schema);
        self.class_cache.insert(name.to_owned(), value.clone());
        Ok(value)
    }

    fn slot_schema(
        &mut self,
        slot: &Map<String, Value>,
        path: &str,
        visiting: &mut BTreeSet<String>,
        depth: usize,
    ) -> Result<(Value, bool), LinkmlError> {
        let required = optional_bool(slot, "required", path)?.unwrap_or(false);
        let minimum = optional_u64(slot, "minimum_cardinality", path)?;
        let maximum = optional_u64(slot, "maximum_cardinality", path)?;
        let multivalued = optional_bool(slot, "multivalued", path)?.unwrap_or(false);
        let scalar = self.expression_schema(slot, path, visiting, depth)?;
        let wrap = multivalued || minimum.is_some() || maximum.is_some();
        let schema = if wrap {
            let mut array = Map::from_iter([
                ("type".to_owned(), Value::String("array".to_owned())),
                ("items".to_owned(), scalar),
            ]);
            if let Some(value) = minimum {
                array.insert("minItems".to_owned(), Value::from(value));
            }
            if let Some(value) = maximum {
                array.insert("maxItems".to_owned(), Value::from(value));
            }
            if let Some(unique) = optional_bool(slot, "list_elements_unique", path)? {
                array.insert("uniqueItems".to_owned(), Value::Bool(unique));
            }
            if optional_bool(slot, "list_elements_ordered", path)?.unwrap_or(false) {
                self.record(
                    "value-term-kind-widened",
                    &format!("{path}/list_elements_ordered"),
                );
            }
            Value::Object(array)
        } else {
            if slot.contains_key("list_elements_unique")
                || slot.contains_key("list_elements_ordered")
            {
                return Err(LinkmlError::new(format!(
                    "{path} applies list semantics to a non-multivalued slot"
                )));
            }
            scalar
        };
        for field in ["inlined", "inlined_as_list"] {
            if optional_bool(slot, field, path)?.is_some() {
                self.record("value-term-kind-widened", &format!("{path}/{field}"));
            }
        }
        self.audit_slot_fields(slot, path)?;
        Ok((schema, required || minimum.is_some_and(|value| value > 0)))
    }

    fn expression_schema(
        &mut self,
        expression: &Map<String, Value>,
        path: &str,
        visiting: &mut BTreeSet<String>,
        depth: usize,
    ) -> Result<Value, LinkmlError> {
        if depth > MAX_ELEMENT_DEPTH {
            return Err(LinkmlError::new(format!(
                "{path} exceeds LinkML expression depth {MAX_ELEMENT_DEPTH}"
            )));
        }
        let mut schema = match expression.get("range") {
            Some(range) => {
                let range = range
                    .as_str()
                    .ok_or_else(|| LinkmlError::new(format!("{path}/range must be a string")))?;
                self.range_schema(range, visiting, depth + 1, &format!("{path}/range"))?
            }
            None => Value::Object(Map::new()),
        };
        let object = schema.as_object_mut().ok_or_else(|| {
            LinkmlError::new(format!(
                "{path} range cannot be combined with this expression"
            ))
        })?;
        self.apply_scalar_fields(expression, object, path)?;
        self.apply_compositions(expression, path, visiting, depth, object, false)?;
        Ok(schema)
    }

    fn range_schema(
        &mut self,
        range: &str,
        visiting: &mut BTreeSet<String>,
        depth: usize,
        path: &str,
    ) -> Result<Value, LinkmlError> {
        let scalar = match range {
            "string" | "ncname" | "nodeidentifier" | "objectidentifier" | "jsonpath"
            | "jsonpointer" | "sparqlpath" => Some(("string", None)),
            "boolean" => Some(("boolean", None)),
            "integer" => Some(("integer", None)),
            "float" | "double" | "decimal" => Some(("number", None)),
            "datetime" => Some(("string", Some("date-time"))),
            "date" => Some(("string", Some("date"))),
            "time" => Some(("string", Some("time"))),
            "uri" => Some(("string", Some("uri"))),
            "uriorcurie" => {
                self.record("format-validation-widened", path);
                Some(("string", None))
            }
            "Any" | "any" => {
                self.record("value-term-kind-widened", path);
                return Ok(Value::Object(Map::new()));
            }
            _ => None,
        };
        if let Some((kind, format)) = scalar {
            let mut schema = Map::from_iter([("type".to_owned(), Value::String(kind.to_owned()))]);
            if let Some(format) = format {
                schema.insert("format".to_owned(), Value::String(format.to_owned()));
            }
            return Ok(Value::Object(schema));
        }
        match self.kinds.get(range).copied() {
            Some(ElementKind::Class) if self.ignored_class_names.contains(range) => {
                if let Some(carrier) = self.package_carrier_schema(range)? {
                    Ok(carrier)
                } else {
                    self.class_schema(range, visiting, depth)
                }
            }
            Some(ElementKind::Class | ElementKind::Enum) => Ok(Value::Object(Map::from_iter([(
                "$ref".to_owned(),
                Value::String(format!("#/$defs/{}", pointer_escape(self.identity(range)?))),
            )]))),
            Some(ElementKind::Type) => self.type_schema(range, visiting, depth),
            None => Err(LinkmlError::new(format!(
                "{path} names unknown LinkML range {range:?}"
            ))),
        }
    }

    fn type_schema(
        &mut self,
        name: &str,
        visiting: &mut BTreeSet<String>,
        depth: usize,
    ) -> Result<Value, LinkmlError> {
        if let Some(schema) = self.type_cache.get(name) {
            return Ok(schema.clone());
        }
        self.enter_element("type", name, visiting, depth)?;
        let path = element_path("types", name);
        let object = self
            .types
            .get(name)
            .and_then(Value::as_object)
            .ok_or_else(|| LinkmlError::new(format!("{path} must be a mapping")))?
            .clone();
        let base = required_string(&object, "typeof", &format!("{path}/typeof"))?;
        let mut schema = self.range_schema(base, visiting, depth + 1, &format!("{path}/typeof"))?;
        let schema_object = schema.as_object_mut().ok_or_else(|| {
            LinkmlError::new(format!("{path}/typeof does not form a scalar schema"))
        })?;
        self.apply_scalar_fields(&object, schema_object, &path)?;
        self.apply_compositions(&object, &path, visiting, depth, schema_object, false)?;
        self.audit_element_fields(
            &object,
            &path,
            &[
                "uri",
                "typeof",
                "title",
                "description",
                "pattern",
                "minimum_value",
                "maximum_value",
                "equals_string",
                "equals_number",
                "any_of",
                "all_of",
                "exactly_one_of",
                "none_of",
            ],
        )?;
        self.map_expression_locations(
            &definition_path(self.identity(name)?),
            &path,
            &object,
            &schema,
        );
        visiting.remove(&format!("type:{name}"));
        self.type_cache.insert(name.to_owned(), schema.clone());
        Ok(schema)
    }

    fn enum_schema(&mut self, name: &str) -> Result<Value, LinkmlError> {
        let path = element_path("enums", name);
        let object = self
            .enums
            .get(name)
            .and_then(Value::as_object)
            .ok_or_else(|| LinkmlError::new(format!("{path} must be a mapping")))?
            .clone();
        let values = object
            .get("permissible_values")
            .and_then(Value::as_object)
            .ok_or_else(|| {
                LinkmlError::new(format!("{path}/permissible_values must be a mapping"))
            })?;
        if values.is_empty() {
            return Err(LinkmlError::new(format!(
                "{path}/permissible_values cannot be empty"
            )));
        }
        let mut members = Vec::new();
        for (text, definition) in values {
            let member_path = format!("{path}/permissible_values/{}", pointer_escape(text));
            let member = match definition {
                Value::Null => Value::String(text.clone()),
                Value::String(description) => {
                    if !description.trim().is_empty() {
                        self.record("enum-metadata-dropped", &member_path);
                    }
                    Value::String(text.clone())
                }
                Value::Object(definition) => {
                    let value = if let Some(meaning) = definition.get("meaning") {
                        let meaning = meaning.as_str().ok_or_else(|| {
                            LinkmlError::new(format!("{member_path}/meaning must be a string"))
                        })?;
                        Value::Object(Map::from_iter([(
                            "@id".to_owned(),
                            Value::String(self.expand_iri(meaning, &member_path)?),
                        )]))
                    } else {
                        Value::String(text.clone())
                    };
                    for (key, value) in definition {
                        if matches!(key.as_str(), "title" | "description") {
                            if !value.is_string() {
                                return Err(LinkmlError::new(format!(
                                    "{member_path}/{} must be a string",
                                    pointer_escape(key)
                                )));
                            }
                            self.record(
                                "enum-metadata-dropped",
                                &format!("{member_path}/{}", pointer_escape(key)),
                            );
                        } else if key != "meaning" {
                            self.record(
                                "unknown-keyword-dropped",
                                &format!("{member_path}/{}", pointer_escape(key)),
                            );
                        }
                    }
                    value
                }
                _ => {
                    return Err(LinkmlError::new(format!(
                        "{member_path} must be null, a string, or a mapping"
                    )));
                }
            };
            members.push(member);
        }
        for (key, value) in &object {
            if matches!(key.as_str(), "title" | "description") {
                if !value.is_string() {
                    return Err(LinkmlError::new(format!(
                        "{path}/{} must be a string",
                        pointer_escape(key)
                    )));
                }
                self.record(
                    "annotation-dropped",
                    &format!("{path}/{}", pointer_escape(key)),
                );
            } else if !matches!(key.as_str(), "enum_uri" | "permissible_values") {
                self.record(
                    "unknown-keyword-dropped",
                    &format!("{path}/{}", pointer_escape(key)),
                );
            }
        }
        Ok(Value::Object(Map::from_iter([(
            "enum".to_owned(),
            Value::Array(members),
        )])))
    }

    fn apply_scalar_fields(
        &self,
        source: &Map<String, Value>,
        target: &mut Map<String, Value>,
        path: &str,
    ) -> Result<(), LinkmlError> {
        if let Some(pattern) = source.get("pattern") {
            let pattern = pattern
                .as_str()
                .ok_or_else(|| LinkmlError::new(format!("{path}/pattern must be a string")))?;
            target.insert("pattern".to_owned(), Value::String(pattern.to_owned()));
        }
        for (native, schema) in [("minimum_value", "minimum"), ("maximum_value", "maximum")] {
            if let Some(value) = source.get(native) {
                if !value.is_number() {
                    return Err(LinkmlError::new(format!(
                        "{path}/{native} must be a number"
                    )));
                }
                target.insert(schema.to_owned(), value.clone());
            }
        }
        let equality = match (source.get("equals_string"), source.get("equals_number")) {
            (Some(_), Some(_)) => {
                return Err(LinkmlError::new(format!(
                    "{path} cannot declare both equals_string and equals_number"
                )));
            }
            (Some(value), None) => {
                if !value.is_string() {
                    return Err(LinkmlError::new(format!(
                        "{path}/equals_string must be a string"
                    )));
                }
                Some(value.clone())
            }
            (None, Some(value)) => {
                if !value.is_number() {
                    return Err(LinkmlError::new(format!(
                        "{path}/equals_number must be a number"
                    )));
                }
                Some(value.clone())
            }
            (None, None) => None,
        };
        if let Some(equality) = equality {
            target.insert("const".to_owned(), equality);
        }
        Ok(())
    }

    fn apply_compositions(
        &mut self,
        source: &Map<String, Value>,
        path: &str,
        visiting: &mut BTreeSet<String>,
        depth: usize,
        target: &mut Map<String, Value>,
        class_expression: bool,
    ) -> Result<(), LinkmlError> {
        for (native, schema) in [
            ("any_of", "anyOf"),
            ("all_of", "allOf"),
            ("exactly_one_of", "oneOf"),
        ] {
            let Some(branches) = source.get(native) else {
                continue;
            };
            let branches = branches
                .as_array()
                .ok_or_else(|| LinkmlError::new(format!("{path}/{native} must be an array")))?;
            if branches.is_empty() {
                return Err(LinkmlError::new(format!("{path}/{native} cannot be empty")));
            }
            let mut translated = Vec::new();
            for (index, branch) in branches.iter().enumerate() {
                let branch_path = format!("{path}/{native}/{index}");
                let branch = branch
                    .as_object()
                    .ok_or_else(|| LinkmlError::new(format!("{branch_path} must be a mapping")))?;
                translated.push(if class_expression {
                    self.class_expression_schema(branch, &branch_path, visiting, depth + 1)?
                } else {
                    let translated =
                        self.expression_schema(branch, &branch_path, visiting, depth + 1)?;
                    self.audit_expression_fields(branch, &branch_path)?;
                    translated
                });
            }
            target.insert(schema.to_owned(), Value::Array(translated));
        }
        if let Some(branches) = source.get("none_of") {
            let branches = branches
                .as_array()
                .ok_or_else(|| LinkmlError::new(format!("{path}/none_of must be an array")))?;
            if branches.is_empty() {
                return Err(LinkmlError::new(format!("{path}/none_of cannot be empty")));
            }
            let mut translated = Vec::new();
            for (index, branch) in branches.iter().enumerate() {
                let branch_path = format!("{path}/none_of/{index}");
                let branch = branch
                    .as_object()
                    .ok_or_else(|| LinkmlError::new(format!("{branch_path} must be a mapping")))?;
                translated.push(if class_expression {
                    self.class_expression_schema(branch, &branch_path, visiting, depth + 1)?
                } else {
                    let translated =
                        self.expression_schema(branch, &branch_path, visiting, depth + 1)?;
                    self.audit_expression_fields(branch, &branch_path)?;
                    translated
                });
            }
            let negated = if translated.len() == 1 {
                translated.pop().expect("length checked")
            } else {
                Value::Object(Map::from_iter([(
                    "anyOf".to_owned(),
                    Value::Array(translated),
                )]))
            };
            target.insert("not".to_owned(), negated);
        }
        Ok(())
    }

    fn class_expression_schema(
        &mut self,
        expression: &Map<String, Value>,
        path: &str,
        visiting: &mut BTreeSet<String>,
        depth: usize,
    ) -> Result<Value, LinkmlError> {
        let parent = if let Some(parent) = expression.get("is_a") {
            let parent = parent
                .as_str()
                .ok_or_else(|| LinkmlError::new(format!("{path}/is_a must be a string")))?;
            self.record("schema-identity-dropped", &format!("{path}/is_a"));
            Some(self.class_schema(parent, visiting, depth + 1)?)
        } else {
            None
        };
        let mut own = Map::from_iter([("type".to_owned(), Value::String("object".to_owned()))]);
        self.apply_compositions(expression, path, visiting, depth, &mut own, true)?;
        if let Some(slot_conditions) = expression.get("slot_conditions") {
            let slot_conditions = slot_conditions.as_object().ok_or_else(|| {
                LinkmlError::new(format!("{path}/slot_conditions must be a mapping"))
            })?;
            for (slot, condition) in slot_conditions {
                if !condition.is_object() {
                    return Err(LinkmlError::new(format!(
                        "{path}/slot_conditions/{} must be a mapping",
                        pointer_escape(slot)
                    )));
                }
            }
            self.record(
                "schema-applicator-dropped",
                &format!("{path}/slot_conditions"),
            );
        }
        self.audit_element_fields(
            expression,
            path,
            &[
                "is_a",
                "slot_conditions",
                "any_of",
                "all_of",
                "exactly_one_of",
                "none_of",
            ],
        )?;
        let has_own_semantics = own.len() > 1;
        if parent.is_none() && !has_own_semantics && !expression.contains_key("slot_conditions") {
            self.record("schema-applicator-dropped", path);
        }
        if let Some(parent) = parent {
            if has_own_semantics {
                Ok(Value::Object(Map::from_iter([
                    (
                        "allOf".to_owned(),
                        Value::Array(vec![parent, Value::Object(own)]),
                    ),
                    ("type".to_owned(), Value::String("object".to_owned())),
                ])))
            } else {
                Ok(parent)
            }
        } else {
            Ok(Value::Object(own))
        }
    }

    fn slot_identity(
        &self,
        name: &str,
        slot: &Map<String, Value>,
        path: &str,
    ) -> Result<String, LinkmlError> {
        if name.starts_with('@') {
            if slot.contains_key("slot_uri") {
                return Err(LinkmlError::new(format!(
                    "{path} reserved JSON-LD slot {name:?} cannot declare slot_uri"
                )));
            }
            return Ok(name.to_owned());
        }
        let identity = slot
            .get("slot_uri")
            .map(|value| {
                value
                    .as_str()
                    .ok_or_else(|| LinkmlError::new(format!("{path}/slot_uri must be a string")))
            })
            .transpose()?
            .unwrap_or(name);
        self.expand_iri(identity, path)
    }

    fn expand_iri(&self, value: &str, path: &str) -> Result<String, LinkmlError> {
        if let Some((prefix, local)) = value.split_once(':')
            && let Some(namespace) = self.prefixes.get(prefix)
        {
            if local.is_empty() {
                return Err(LinkmlError::new(format!(
                    "{path} CURIE {value:?} has an empty local part"
                )));
            }
            return validate_absolute(&format!("{namespace}{local}"), path);
        }
        if purrdf_iri::parse(value).is_ok_and(|iri| iri.has_scheme()) {
            return Ok(value.to_owned());
        }
        if value.is_empty() || value.contains(':') {
            return Err(LinkmlError::new(format!(
                "{path} identity {value:?} is neither an absolute IRI nor a declared CURIE"
            )));
        }
        let namespace = self.prefixes.get(&self.default_prefix).ok_or_else(|| {
            LinkmlError::new("LinkML default prefix is missing from the validated prefix map")
        })?;
        validate_absolute(&format!("{namespace}{value}"), path)
    }

    fn identity(&self, name: &str) -> Result<&str, LinkmlError> {
        self.identities
            .get(name)
            .map(String::as_str)
            .ok_or_else(|| LinkmlError::new(format!("unknown LinkML element reference {name:?}")))
    }

    fn enter_element(
        &self,
        kind: &str,
        name: &str,
        visiting: &mut BTreeSet<String>,
        depth: usize,
    ) -> Result<(), LinkmlError> {
        if depth > MAX_ELEMENT_DEPTH {
            return Err(LinkmlError::new(format!(
                "LinkML {kind} {name:?} exceeds reference depth {MAX_ELEMENT_DEPTH}"
            )));
        }
        if !visiting.insert(format!("{kind}:{name}")) {
            return Err(LinkmlError::new(format!(
                "cyclic LinkML {kind} reference includes {name:?}"
            )));
        }
        Ok(())
    }

    fn is_package_infrastructure(&self, name: &str) -> bool {
        let Some(class) = self.classes.get(name).and_then(Value::as_object) else {
            return false;
        };
        if name == "Node" {
            return class
                .get("attributes")
                .and_then(Value::as_object)
                .is_some_and(|attributes| {
                    ["@id", "@type", "@annotation"]
                        .iter()
                        .all(|key| attributes.contains_key(*key))
                });
        }
        name == "Annotation"
            && class.get("title").and_then(Value::as_str)
                == Some("RDF-1.2 statement metadata (reifier annotation)")
    }

    fn package_carrier_schema(&self, name: &str) -> Result<Option<Value>, LinkmlError> {
        let Some(class) = self.classes.get(name).and_then(Value::as_object) else {
            return Ok(None);
        };
        let Some(attributes) = class.get("attributes").and_then(Value::as_object) else {
            return Ok(None);
        };
        if attributes.len() == 1 && attributes.contains_key("@id") {
            let id = attributes["@id"].as_object().ok_or_else(|| {
                LinkmlError::new(format!("#/classes/{name}/attributes/@id must be a mapping"))
            })?;
            if optional_bool(id, "required", "#/classes/helper/attributes/@id")? == Some(true) {
                return Ok(Some(serde_json::json!({
                    "type": "object",
                    "properties": { "@id": { "type": "string" } },
                    "required": ["@id"]
                })));
            }
        }
        if attributes.len() == 2
            && attributes.contains_key("@value")
            && attributes.contains_key("@type")
        {
            let value = attributes["@value"].as_object().ok_or_else(|| {
                LinkmlError::new(format!(
                    "#/classes/{name}/attributes/@value must be a mapping"
                ))
            })?;
            let datatype = attributes["@type"].as_object().ok_or_else(|| {
                LinkmlError::new(format!(
                    "#/classes/{name}/attributes/@type must be a mapping"
                ))
            })?;
            if optional_bool(value, "required", "#/classes/helper/attributes/@value")? == Some(true)
                && datatype.get("range").and_then(Value::as_str) == Some("string")
            {
                return Ok(Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "@value": {},
                        "@type": { "type": "string" }
                    },
                    "required": ["@value"]
                })));
            }
        }
        if attributes.len() == 2
            && attributes.contains_key("@value")
            && attributes.contains_key("@language")
        {
            let value = attributes["@value"].as_object().ok_or_else(|| {
                LinkmlError::new(format!(
                    "#/classes/{name}/attributes/@value must be a mapping"
                ))
            })?;
            let language = attributes["@language"].as_object().ok_or_else(|| {
                LinkmlError::new(format!(
                    "#/classes/{name}/attributes/@language must be a mapping"
                ))
            })?;
            if optional_bool(value, "required", "#/classes/helper/attributes/@value")? == Some(true)
                && optional_bool(
                    language,
                    "required",
                    "#/classes/helper/attributes/@language",
                )? == Some(true)
            {
                let pattern = language
                    .get("pattern")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        LinkmlError::new(format!(
                            "#/classes/{name}/attributes/@language/pattern must be a string"
                        ))
                    })?;
                return Ok(Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "@value": { "type": "string" },
                        "@language": { "type": "string", "pattern": pattern }
                    },
                    "required": ["@value", "@language"]
                })));
            }
        }
        Ok(None)
    }

    fn audit_element_fields(
        &mut self,
        object: &Map<String, Value>,
        path: &str,
        known: &[&str],
    ) -> Result<(), LinkmlError> {
        for (key, value) in object {
            if matches!(key.as_str(), "title" | "description") {
                if !value.is_string() {
                    return Err(LinkmlError::new(format!(
                        "{path}/{} must be a string",
                        pointer_escape(key)
                    )));
                }
                self.record(
                    "annotation-dropped",
                    &format!("{path}/{}", pointer_escape(key)),
                );
            } else if key == "alias" {
                if !value.is_string() {
                    return Err(LinkmlError::new(format!("{path}/alias must be a string")));
                }
                self.record("schema-identity-dropped", &format!("{path}/alias"));
            } else if !known.contains(&key.as_str()) {
                self.record(
                    "unknown-keyword-dropped",
                    &format!("{path}/{}", pointer_escape(key)),
                );
            }
        }
        Ok(())
    }

    fn audit_slot_fields(
        &mut self,
        object: &Map<String, Value>,
        path: &str,
    ) -> Result<(), LinkmlError> {
        let known = [
            "slot_uri",
            "alias",
            "title",
            "description",
            "range",
            "required",
            "multivalued",
            "minimum_cardinality",
            "maximum_cardinality",
            "list_elements_ordered",
            "list_elements_unique",
            "inlined",
            "inlined_as_list",
            "pattern",
            "minimum_value",
            "maximum_value",
            "equals_string",
            "equals_number",
            "any_of",
            "all_of",
            "exactly_one_of",
            "none_of",
        ];
        self.audit_element_fields(object, path, &known)
    }

    fn audit_expression_fields(
        &mut self,
        object: &Map<String, Value>,
        path: &str,
    ) -> Result<(), LinkmlError> {
        self.audit_element_fields(
            object,
            path,
            &[
                "range",
                "pattern",
                "minimum_value",
                "maximum_value",
                "equals_string",
                "equals_number",
                "any_of",
                "all_of",
                "exactly_one_of",
                "none_of",
            ],
        )
    }

    fn map_expression_locations(
        &mut self,
        pivot_path: &str,
        native_path: &str,
        source: &Map<String, Value>,
        translated: &Value,
    ) {
        let scalar_path = if translated
            .as_object()
            .and_then(|object| object.get("type"))
            .and_then(Value::as_str)
            == Some("array")
        {
            format!("{pivot_path}/items")
        } else {
            pivot_path.to_owned()
        };
        if source.contains_key("range") {
            for keyword in ["type", "format", "$ref"] {
                self.map_location(
                    &format!("{scalar_path}/{keyword}"),
                    &format!("{native_path}/range"),
                );
            }
        }
        for (native, schema) in [
            ("pattern", "pattern"),
            ("minimum_value", "minimum"),
            ("maximum_value", "maximum"),
            ("equals_string", "const"),
            ("equals_number", "const"),
            ("any_of", "anyOf"),
            ("all_of", "allOf"),
            ("exactly_one_of", "oneOf"),
            ("none_of", "not"),
        ] {
            if source.contains_key(native) {
                self.map_location(
                    &format!("{scalar_path}/{schema}"),
                    &format!("{native_path}/{native}"),
                );
            }
        }
    }

    fn record(&mut self, code: &str, path: &str) {
        if !self
            .recorded_losses
            .insert((code.to_owned(), path.to_owned()))
        {
            return;
        }
        let contract = self
            .contract
            .entries()
            .iter()
            .find(|entry| entry.code == code)
            .unwrap_or_else(|| panic!("unregistered LinkML import loss code `{code}`"));
        self.losses.record(LossEntry {
            code: code.to_owned().into(),
            from: SOURCE.to_owned().into(),
            to: TARGET.to_owned().into(),
            note: contract.note.to_string().into(),
            location: Some(Box::new(
                RdfLocation::logical(LOSS_CONTEXT).with_subject(path),
            )),
        });
    }

    fn map_location(&mut self, pivot: &str, native: &str) {
        self.locations.insert(pivot.to_owned(), native.to_owned());
    }
}

fn section<'a>(
    root: &'a Map<String, Value>,
    name: &str,
) -> Result<&'a Map<String, Value>, LinkmlError> {
    static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
    root.get(name)
        .map(|value| {
            value
                .as_object()
                .ok_or_else(|| LinkmlError::new(format!("#/{name} must be a mapping")))
        })
        .transpose()
        .map(|value| value.unwrap_or_else(|| EMPTY.get_or_init(Map::new)))
}

fn document_prefixes(root: &Map<String, Value>) -> Result<BTreeMap<String, String>, LinkmlError> {
    let prefixes = root
        .get("prefixes")
        .and_then(Value::as_object)
        .ok_or_else(|| LinkmlError::new("#/prefixes must be a mapping"))?;
    prefixes
        .iter()
        .map(|(prefix, value)| {
            let namespace = match value {
                Value::String(namespace) => namespace,
                Value::Object(object) => object
                    .get("prefix_reference")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        LinkmlError::new(format!(
                            "#/prefixes/{} requires string prefix_reference",
                            pointer_escape(prefix)
                        ))
                    })?,
                _ => {
                    return Err(LinkmlError::new(format!(
                        "#/prefixes/{} must be a string or mapping",
                        pointer_escape(prefix)
                    )));
                }
            };
            Ok((prefix.clone(), namespace.to_owned()))
        })
        .collect()
}

fn validate_absolute(value: &str, path: &str) -> Result<String, LinkmlError> {
    let iri = purrdf_iri::parse(value).map_err(|error| {
        LinkmlError::new(format!("{path} forms invalid IRI {value:?}: {error}"))
    })?;
    if !iri.has_scheme() {
        return Err(LinkmlError::new(format!(
            "{path} forms relative IRI {value:?}"
        )));
    }
    Ok(value.to_owned())
}

fn required_string<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    path: &str,
) -> Result<&'a str, LinkmlError> {
    object
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| LinkmlError::new(format!("{path} must be a string")))
}

fn required_bool(object: &Map<String, Value>, key: &str, path: &str) -> Result<bool, LinkmlError> {
    object
        .get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| LinkmlError::new(format!("{path} must be a boolean")))
}

fn optional_bool(
    object: &Map<String, Value>,
    key: &str,
    path: &str,
) -> Result<Option<bool>, LinkmlError> {
    object
        .get(key)
        .map(|value| {
            value
                .as_bool()
                .ok_or_else(|| LinkmlError::new(format!("{path}/{key} must be a boolean")))
        })
        .transpose()
}

fn optional_u64(
    object: &Map<String, Value>,
    key: &str,
    path: &str,
) -> Result<Option<u64>, LinkmlError> {
    object
        .get(key)
        .map(|value| {
            value.as_u64().ok_or_else(|| {
                LinkmlError::new(format!("{path}/{key} must be a non-negative integer"))
            })
        })
        .transpose()
}

fn element_path(section: &str, name: &str) -> String {
    format!("#/{section}/{}", pointer_escape(name))
}

fn definition_path(key: &str) -> String {
    format!("#/$defs/{}", pointer_escape(key))
}

fn pointer_escape(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

fn remap_location(subject: &str, mappings: &BTreeMap<String, String>) -> String {
    mappings
        .iter()
        .filter(|(pivot, _)| {
            subject == pivot.as_str()
                || subject
                    .strip_prefix(pivot.as_str())
                    .is_some_and(|suffix| suffix.starts_with('/'))
        })
        .max_by_key(|(pivot, _)| pivot.len())
        .map_or_else(
            || subject.to_owned(),
            |(pivot, native)| format!("{native}{}", &subject[pivot.len()..]),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json_schema::{CompiledSchema, Namespaces};
    use crate::linkml::{LinkmlConfig, emit_linkml};
    use crate::schema_import::SchemaDatatypeMap;
    use crate::shapes::{Constraint, Path};
    use serde_json::json;

    const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

    fn config() -> SchemaImportConfig {
        let namespaces = Namespaces::new(
            "ex",
            &[("ex".to_owned(), "https://example.org/".to_owned())],
        )
        .expect("namespaces");
        let datatypes = SchemaDatatypeMap::new(
            format!("{XSD}string"),
            format!("{XSD}boolean"),
            format!("{XSD}integer"),
            format!("{XSD}decimal"),
            format!("{XSD}dateTime"),
            format!("{XSD}date"),
            format!("{XSD}time"),
            format!("{XSD}anyURI"),
        )
        .expect("datatypes");
        SchemaImportConfig::new(namespaces, datatypes)
    }

    fn linkml_config() -> LinkmlConfig {
        LinkmlConfig::new(
            "https://example.org/schema/linkml",
            "Example-Schema",
            "Caller document.",
            "ex",
            BTreeMap::from([
                ("ex".to_owned(), "https://example.org/".to_owned()),
                ("linkml".to_owned(), "https://w3id.org/linkml/".to_owned()),
            ]),
        )
        .expect("LinkML config")
    }

    fn native_document() -> LinkmlDocument {
        LinkmlDocument::from_value(json!({
            "id": "https://example.org/schema/linkml",
            "name": "Example-Schema",
            "description": "Native source.",
            "metamodel_version": "1.11.0",
            "prefixes": {
                "ex": "https://example.org/",
                "linkml": "https://w3id.org/linkml/"
            },
            "default_prefix": "ex",
            "imports": ["linkml:types"],
            "types": {
                "PositiveInteger": {
                    "uri": "ex:PositiveInteger",
                    "typeof": "integer",
                    "minimum_value": 0
                }
            },
            "enums": {
                "Color": {
                    "enum_uri": "ex:Color",
                    "permissible_values": {
                        "red": { "meaning": "ex:red", "title": "Red" },
                        "plain": {}
                    }
                }
            },
            "slots": {
                "name": {
                    "slot_uri": "ex:name",
                    "range": "string",
                    "required": true,
                    "pattern": "^[A-Z]"
                }
            },
            "classes": {
                "Address": {
                    "class_uri": "ex:Address",
                    "attributes": {
                        "city": {
                            "slot_uri": "ex:city",
                            "range": "string",
                            "required": true
                        }
                    },
                    "extra_slots": { "allowed": false }
                },
                "Person": {
                    "class_uri": "ex:Person",
                    "slots": ["name"],
                    "attributes": {
                        "age": {
                            "slot_uri": "ex:age",
                            "range": "PositiveInteger",
                            "required": true
                        },
                        "color": {
                            "slot_uri": "ex:color",
                            "range": "Color"
                        },
                        "friend": {
                            "slot_uri": "ex:friend",
                            "range": "Person",
                            "inlined": true
                        },
                        "tags": {
                            "slot_uri": "ex:tags",
                            "range": "string",
                            "multivalued": true,
                            "minimum_cardinality": 2,
                            "maximum_cardinality": 4,
                            "list_elements_unique": true
                        }
                    },
                    "extra_slots": { "allowed": false }
                }
            }
        }))
        .expect("native LinkML")
    }

    fn compile_turtle(body: &str) -> CompiledSchema {
        let source = format!(
            r"
            @prefix ex:  <https://example.org/> .
            @prefix sh:  <http://www.w3.org/ns/shacl#> .
            @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
            {body}
            "
        );
        let dataset = crate::text_ingest::parse_turtle_to_dataset(&source).expect("parse");
        let shapes = crate::shapes::from_dataset(&dataset).expect("shapes");
        crate::json_schema::compile(&shapes, config().namespaces())
    }

    #[test]
    fn native_classes_slots_types_and_enums_import_deterministically() {
        let document = native_document();
        let imported = import_document(&document, &config(), None).expect("import");
        assert_eq!(imported.shapes.node_shapes.len(), 2);
        let person = imported
            .shapes
            .node_shapes
            .iter()
            .find(|shape| shape.id.to_string().contains("Person"))
            .expect("Person shape");
        let tags = person
            .property_shapes
            .iter()
            .find(|property| {
                matches!(&property.path, Path::Predicate(predicate) if predicate.as_str() == "https://example.org/tags")
            })
            .expect("tags");
        assert!(
            tags.constraints
                .iter()
                .any(|constraint| matches!(constraint, Constraint::MinCount(2)))
        );
        assert!(
            tags.constraints
                .iter()
                .any(|constraint| matches!(constraint, Constraint::MaxCount(4)))
        );
        let observed = imported
            .losses
            .entries()
            .iter()
            .filter_map(|entry| {
                Some((
                    entry.code.as_ref(),
                    entry.location.as_ref()?.subject.as_deref()?,
                ))
            })
            .collect::<BTreeSet<_>>();
        assert!(observed.contains(&(
            "unique-items-validation-dropped",
            "#/classes/Person/attributes/tags/list_elements_unique"
        )));
        assert!(observed.contains(&(
            "enum-metadata-dropped",
            "#/enums/Color/permissible_values/red/title"
        )));
        check_ledger_sound(&imported.losses, SOURCE, TARGET).expect("sound ledger");

        let repeated = import_document(&document, &config(), None).expect("repeat import");
        assert_eq!(imported.losses.render_json(), repeated.losses.render_json());
        let first = crate::json_schema::compile(&imported.shapes, config().namespaces());
        let second = crate::json_schema::compile(&repeated.shapes, config().namespaces());
        assert_eq!(first.schema_json, second.schema_json);

        let yaml = write_linkml(&document).expect("canonical YAML");
        let reparsed = parse_linkml(&yaml).expect("parse canonical YAML");
        let from_yaml = import_document(&reparsed, &config(), None).expect("import YAML");
        assert_eq!(
            imported.losses.render_json(),
            from_yaml.losses.render_json()
        );
        let yaml_compiled = crate::json_schema::compile(&from_yaml.shapes, config().namespaces());
        assert_eq!(first.schema_json, yaml_compiled.schema_json);
    }

    #[test]
    fn emitted_package_verifies_artifacts_and_imports_without_helpers() {
        let compiled = compile_turtle(
            r#"
            ex:AddressShape a sh:NodeShape ;
                sh:targetClass ex:Address ;
                sh:closed true ;
                sh:property [
                    sh:path ex:city ;
                    sh:datatype xsd:string ;
                    sh:minCount 1 ;
                    sh:maxCount 1 ;
                    sh:pattern "^[A-Z]"
                ] .

            ex:PersonShape a sh:NodeShape ;
                sh:targetClass ex:Person ;
                sh:closed true ;
                sh:property [
                    sh:path ex:name ;
                    sh:datatype xsd:string ;
                    sh:minCount 1 ;
                    sh:maxCount 1 ;
                    sh:pattern "^[A-Z]"
                ] ;
                sh:property [
                    sh:path ex:address ;
                    sh:class ex:Address ;
                    sh:maxCount 1
                ] ;
                sh:property [
                    sh:path ex:age ;
                    sh:datatype xsd:integer ;
                    sh:maxCount 1 ;
                    sh:minInclusive 0 ;
                    sh:maxInclusive 130
                ] ;
                sh:property [
                    sh:path ex:tags ;
                    sh:datatype xsd:string ;
                    sh:minCount 2 ;
                    sh:maxCount 4
                ] ;
                sh:property [
                    sh:path ex:state ;
                    sh:maxCount 1 ;
                    sh:in ( ex:active "inactive" )
                ] .
            "#,
        );
        let package = emit_linkml(&compiled, &linkml_config()).expect("emit LinkML");
        let imported = import_package(&package, &config()).expect("import package");
        assert_eq!(imported.shapes.node_shapes.len(), 2);
        assert!(
            imported
                .shapes
                .node_shapes
                .iter()
                .any(|shape| shape.id.to_string().contains("Person"))
        );
        let recompiled = crate::json_schema::compile(&imported.shapes, config().namespaces());
        assert_eq!(recompiled.schema_json, compiled.schema_json);

        let mut bad_yaml = package.clone();
        bad_yaml.yaml.push_str("# drift\n");
        assert!(import_package(&bad_yaml, &config()).is_err());

        let mut bad_map = package;
        *bad_map
            .element_names
            .values_mut()
            .next()
            .expect("element map") = "MissingElement".to_owned();
        assert!(import_package(&bad_map, &config()).is_err());
    }

    #[test]
    fn accepted_native_omissions_are_located_and_malformed_fields_fail_closed() {
        let mut value = native_document().into_value();
        value["prefixes"]["ex"] = json!({
            "prefix_prefix": "ex",
            "prefix_reference": "https://example.org/",
            "x-prefix-extension": true
        });
        value["classes"]["Child"] = json!({
            "class_uri": "ex:Child",
            "is_a": "Person",
            "slot_usage": {
                "name": { "required": false }
            },
            "extra_slots": {
                "allowed": true,
                "range_expression": {
                    "range": "string",
                    "x-range-extension": true
                },
                "x-extra-extension": true
            },
            "any_of": [{
                "slot_conditions": {
                    "name": { "required": true }
                },
                "x-branch-extension": true
            }]
        });
        let document = LinkmlDocument::from_value(value).expect("extended native document");
        let imported = import_document(&document, &config(), None).expect("ledgered import");
        assert_eq!(imported.shapes.node_shapes.len(), 3);
        let observed = imported
            .losses
            .entries()
            .iter()
            .filter_map(|entry| {
                Some((
                    entry.code.as_ref(),
                    entry.location.as_ref()?.subject.as_deref()?,
                ))
            })
            .collect::<BTreeSet<_>>();
        for expected in [
            (
                "unknown-keyword-dropped",
                "#/prefixes/ex/x-prefix-extension",
            ),
            (
                "schema-applicator-dropped",
                "#/classes/Child/slot_usage/name",
            ),
            (
                "schema-applicator-dropped",
                "#/classes/Child/any_of/0/slot_conditions",
            ),
            (
                "unknown-keyword-dropped",
                "#/classes/Child/any_of/0/x-branch-extension",
            ),
            (
                "unknown-keyword-dropped",
                "#/classes/Child/extra_slots/range_expression/x-range-extension",
            ),
            (
                "unknown-keyword-dropped",
                "#/classes/Child/extra_slots/x-extra-extension",
            ),
        ] {
            assert!(observed.contains(&expected), "missing {expected:?}");
        }
        check_ledger_sound(&imported.losses, SOURCE, TARGET).expect("sound ledger");

        let mut value = native_document().into_value();
        value["classes"]["Person"]["attributes"]["tags"]["inlined"] = json!("yes");
        let document = LinkmlDocument::from_value(value).expect("valid envelope");
        assert!(
            import_document(&document, &config(), None)
                .expect_err("malformed inlined flag")
                .to_string()
                .contains("must be a boolean")
        );

        let mut value = native_document().into_value();
        value["enums"]["Color"]["permissible_values"]["red"]["title"] = json!(7);
        let document = LinkmlDocument::from_value(value).expect("valid envelope");
        assert!(
            import_document(&document, &config(), None)
                .expect_err("malformed enum title")
                .to_string()
                .contains("must be a string")
        );
    }

    #[test]
    fn malformed_native_references_and_identity_collisions_fail_closed() {
        let mut value = native_document().into_value();
        value["classes"]["Person"]["attributes"]["friend"]["range"] =
            Value::String("Missing".to_owned());
        let document = LinkmlDocument::from_value(value).expect("envelope remains valid");
        assert!(
            import_document(&document, &config(), None)
                .expect_err("missing range")
                .to_string()
                .contains("unknown LinkML range")
        );

        let mut value = native_document().into_value();
        value["classes"]["Address"]["class_uri"] = Value::String("ex:Person".to_owned());
        let document = LinkmlDocument::from_value(value).expect("envelope remains valid");
        assert!(
            import_document(&document, &config(), None)
                .expect_err("identity collision")
                .to_string()
                .contains("same IRI")
        );
    }
}
