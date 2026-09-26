// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Deterministic schema-language → SHACL interpretation.
//!
//! JSON Schema draft 2020-12 is the shared semantic pivot. Native LinkML and
//! the fixed generated-package adapters normalize into this boundary, so CURIE
//! expansion, constraint lowering, loss accounting, and resource limits cannot
//! drift between five readers. Accepted omissions are always recorded on a
//! closed source-language → `shacl` loss profile.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::sync::{Arc, OnceLock};

use ::purrdf::RdfLocation;
use ::purrdf::loss::{LossEntry, LossLedger, check_ledger_sound, schema_to_shacl_loss_ledger};
use purrdf_xsd::ieee::Binary64Scope;
use serde_json::{Map, Number, Value};

use crate::json_schema::{CompiledSchema, Namespaces};
use crate::report::Severity;
use crate::shapes::{
    ClosedMode, Constraint, NodeKindValue, Path, PropertyShape, Shape, Shapes, Target,
};
use crate::term::{Literal, NamedNode, Term};

const JSON_SCHEMA_DIALECT: &str = "https://json-schema.org/draft/2020-12/schema";

/// A property schema split into its per-value schema, `minItems`, `maxItems`,
/// and the `sh:hasValue` constants an array form states under `contains`.
type CardinalitySplit = (Value, Option<u64>, Option<u64>, Vec<Value>);
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";
const RDF_DIR_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString";
const JSON_SCHEMA_SOURCE: &str = "json-schema";
const MAX_SCHEMA_BYTES: usize = 16 * 1024 * 1024;
const MAX_DEFINITIONS: usize = 65_536;
const MAX_PROPERTIES: usize = 65_536;
const MAX_SCHEMA_DEPTH: usize = 128;
const MAX_SCHEMA_NODES: usize = 1_000_000;
const MAX_STRING_BYTES: usize = 16 * 1024 * 1024;

/// Caller-owned RDF datatypes used when a scalar schema has no original RDF
/// literal attached to it.
///
/// There is intentionally no [`Default`] implementation. PurRDF does not guess
/// which RDF datatype vocabulary a caller wants for JSON strings, numbers, or
/// temporal formats.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaDatatypeMap {
    string: String,
    boolean: String,
    integer: String,
    number: String,
    date_time: String,
    date: String,
    time: String,
    uri: String,
}

impl SchemaDatatypeMap {
    /// Validate and construct the complete scalar → RDF datatype mapping.
    ///
    /// # Errors
    ///
    /// Returns [`SchemaImportError`] when any supplied value is not an absolute
    /// IRI. Empty or relative datatype names never receive a built-in fallback.
    #[allow(
        clippy::too_many_arguments,
        reason = "the eight mandatory scalar roles make missing vocabulary configuration unrepresentable"
    )]
    pub fn new(
        string: impl Into<String>,
        boolean: impl Into<String>,
        integer: impl Into<String>,
        number: impl Into<String>,
        date_time: impl Into<String>,
        date: impl Into<String>,
        time: impl Into<String>,
        uri: impl Into<String>,
    ) -> Result<Self, SchemaImportError> {
        let values = [
            ("string", string.into()),
            ("boolean", boolean.into()),
            ("integer", integer.into()),
            ("number", number.into()),
            ("date-time", date_time.into()),
            ("date", date.into()),
            ("time", time.into()),
            ("uri", uri.into()),
        ];
        for (label, value) in &values {
            validate_absolute_iri(&format!("{label} datatype"), value)?;
        }
        let [
            (_, string),
            (_, boolean),
            (_, integer),
            (_, number),
            (_, date_time),
            (_, date),
            (_, time),
            (_, uri),
        ] = values;
        Ok(Self {
            string,
            boolean,
            integer,
            number,
            date_time,
            date,
            time,
            uri,
        })
    }

    fn for_scalar(&self, kind: &str, format: Option<&str>) -> Option<&str> {
        match (kind, format) {
            ("string", Some("date-time")) => Some(&self.date_time),
            ("string", Some("date")) => Some(&self.date),
            ("string", Some("time")) => Some(&self.time),
            ("string", Some("uri")) => Some(&self.uri),
            ("string", _) => Some(&self.string),
            ("boolean", _) => Some(&self.boolean),
            ("integer", _) => Some(&self.integer),
            ("number", _) => Some(&self.number),
            _ => None,
        }
    }

    fn for_number(&self, integer: bool) -> &str {
        if integer { &self.integer } else { &self.number }
    }
}

/// Mandatory caller configuration for every schema importer.
#[derive(Debug, Clone)]
pub struct SchemaImportConfig {
    namespaces: Namespaces,
    datatypes: SchemaDatatypeMap,
}

impl SchemaImportConfig {
    /// Construct a schema importer configuration from caller-owned namespaces
    /// and scalar datatype identities.
    #[must_use]
    pub fn new(namespaces: Namespaces, datatypes: SchemaDatatypeMap) -> Self {
        Self {
            namespaces,
            datatypes,
        }
    }

    /// Namespace table used for every class, property, datatype, and value IRI.
    #[must_use]
    pub fn namespaces(&self) -> &Namespaces {
        &self.namespaces
    }

    /// Scalar datatype mapping used by the importer.
    #[must_use]
    pub fn datatypes(&self) -> &SchemaDatatypeMap {
        &self.datatypes
    }
}

/// Typed SHACL import result and its always-computed reverse loss ledger.
#[derive(Debug, Clone)]
pub struct ImportedShapes {
    /// Deterministically ordered SHACL node shapes.
    pub shapes: Shapes,
    /// Every accepted source construct without an exact SHACL representation.
    pub losses: LossLedger,
}

/// A malformed, ambiguous, unsupported-resource, or internally inconsistent
/// schema import request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaImportError {
    detail: String,
}

impl SchemaImportError {
    fn new(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
        }
    }

    /// Stable human-readable error detail.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for SchemaImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl Error for SchemaImportError {}

/// Import one JSON Schema draft 2020-12 document as SHACL shapes.
///
/// The accepted identity convention is the same one [`crate::json_schema`]
/// emits: colon-free `$defs` keys belong to the caller's primary namespace;
/// qualified keys must be absolute IRIs or use a caller-declared prefix.
///
/// # Errors
///
/// Returns [`SchemaImportError`] for malformed JSON/schema structures, a wrong
/// dialect, open or dangling references, invalid/ambiguous identities,
/// inconsistent cardinality wrappers, or fixed resource-limit exhaustion.
pub fn import_json_schema(
    input: &str,
    config: &SchemaImportConfig,
) -> Result<ImportedShapes, SchemaImportError> {
    import_json_schema_from(JSON_SCHEMA_SOURCE, input, config)
}

/// Import the JSON Schema artifact held by one [`CompiledSchema`]. Existing
/// forward losses remain on the caller's compiled value; the returned ledger
/// describes only this reverse transformation.
///
/// # Errors
///
/// Returns the same errors as [`import_json_schema`].
pub fn import_compiled_schema(
    compiled: &CompiledSchema,
    config: &SchemaImportConfig,
) -> Result<ImportedShapes, SchemaImportError> {
    import_json_schema(&compiled.schema_json, config)
}

pub(crate) fn import_json_schema_from(
    source: &'static str,
    input: &str,
    config: &SchemaImportConfig,
) -> Result<ImportedShapes, SchemaImportError> {
    if input.len() > MAX_SCHEMA_BYTES {
        return Err(SchemaImportError::new(format!(
            "schema input exceeds the {MAX_SCHEMA_BYTES}-byte limit"
        )));
    }
    // A JSON Schema number becomes a SHACL bound's lexical form: read it inside a
    // binary64 scope so the x87 rounds `serde_json`'s exact fast path once, like every
    // other unit (the workspace's `float_roundtrip` makes the rest correctly rounded).
    let document: Value = {
        let _binary64 = Binary64Scope::enter();
        serde_json::from_str(input)
    }
    .map_err(|error| SchemaImportError::new(format!("invalid JSON Schema JSON: {error}")))?;
    import_schema_value_from(source, &document, config)
}

pub(crate) fn import_schema_value_from(
    source: &'static str,
    document: &Value,
    config: &SchemaImportConfig,
) -> Result<ImportedShapes, SchemaImportError> {
    let mut nodes = 0;
    validate_value_limits(document, 0, &mut nodes, "#")?;
    let root = document
        .as_object()
        .ok_or_else(|| SchemaImportError::new("JSON Schema document root must be an object"))?;
    if let Some(dialect) = root.get("$schema") {
        let dialect = dialect
            .as_str()
            .ok_or_else(|| SchemaImportError::new("#/$schema must be a string"))?;
        if dialect != JSON_SCHEMA_DIALECT {
            return Err(SchemaImportError::new(format!(
                "#/$schema must be {JSON_SCHEMA_DIALECT:?}, got {dialect:?}"
            )));
        }
    }
    let definitions = root
        .get("$defs")
        .and_then(Value::as_object)
        .ok_or_else(|| SchemaImportError::new("JSON Schema must contain object-valued #/$defs"))?;
    if definitions.len() > MAX_DEFINITIONS {
        return Err(SchemaImportError::new(format!(
            "JSON Schema contains {} definitions; limit is {MAX_DEFINITIONS}",
            definitions.len()
        )));
    }

    validate_references(document, definitions, "#", 0)?;
    let generated_envelope = is_generated_envelope(root, definitions, &config.namespaces);
    let mut context = ImportContext::new(source, config, definitions, generated_envelope);
    context.audit_root(root, generated_envelope)?;
    let mut model = SchemaImportModel::default();
    let mut shape_identities = BTreeMap::new();
    for (key, schema) in definitions {
        if generated_envelope && matches!(key.as_str(), "Annotation" | "Node") {
            continue;
        }
        let path = definition_path(key);
        let Some(shape) = context.import_definition(key, schema, &path)? else {
            continue;
        };
        let Term::NamedNode(identity) = &shape.id else {
            unreachable!("top-level schema definitions always use caller-owned class IRIs");
        };
        if let Some(previous) = shape_identities.insert(identity.as_str().to_owned(), path.clone())
        {
            return Err(SchemaImportError::new(format!(
                "{path} and {previous} resolve to the same class IRI {:?}",
                identity.as_str()
            )));
        }
        model.node_shapes.push(shape);
    }
    model
        .node_shapes
        .sort_by_cached_key(|shape| shape.id.to_string());
    context.finish(model)
}

#[derive(Default)]
struct SchemaImportModel {
    node_shapes: Vec<Shape>,
}

struct ImportContext<'a> {
    source: &'static str,
    config: &'a SchemaImportConfig,
    definitions: &'a Map<String, Value>,
    contract: LossLedger,
    losses: LossLedger,
    nested_shape_counter: usize,
    generated_envelope: bool,
}

impl<'a> ImportContext<'a> {
    fn new(
        source: &'static str,
        config: &'a SchemaImportConfig,
        definitions: &'a Map<String, Value>,
        generated_envelope: bool,
    ) -> Self {
        Self {
            source,
            config,
            definitions,
            contract: schema_to_shacl_loss_ledger(source),
            losses: LossLedger::new(),
            nested_shape_counter: 0,
            generated_envelope,
        }
    }

    fn finish(self, model: SchemaImportModel) -> Result<ImportedShapes, SchemaImportError> {
        check_ledger_sound(&self.losses, self.source, "shacl").map_err(SchemaImportError::new)?;
        let shapes = Shapes {
            node_shapes: model.node_shapes,
            ..Shapes::default()
        };
        Ok(ImportedShapes {
            shapes,
            losses: self.losses,
        })
    }

    fn record(&mut self, code: &str, path: &str) {
        let contract = self
            .contract
            .entries()
            .iter()
            .find(|entry| entry.code == code)
            .unwrap_or_else(|| panic!("unregistered schema import loss code `{code}`"));
        self.losses.record(LossEntry {
            code: code.to_owned().into(),
            from: self.source.to_owned().into(),
            to: "shacl".to_owned().into(),
            note: contract.note.to_string().into(),
            location: Some(Box::new(
                RdfLocation::logical("schema-importer").with_subject(path),
            )),
        });
    }

    fn nested_shape_id(&mut self, path: &str) -> Term {
        let id = self.nested_shape_counter;
        self.nested_shape_counter += 1;
        Term::blank(format!("schema-import-{id:08x}-{}", fnv1a(path.as_bytes())))
    }
}

fn validate_absolute_iri(label: &str, value: &str) -> Result<(), SchemaImportError> {
    let iri = purrdf_iri::parse(value)
        .map_err(|error| SchemaImportError::new(format!("{label} is not a valid IRI: {error}")))?;
    if !iri.has_scheme() {
        return Err(SchemaImportError::new(format!(
            "{label} must be an absolute IRI"
        )));
    }
    Ok(())
}

fn definition_path(key: &str) -> String {
    format!("#/$defs/{}", pointer_escape(key))
}

fn pointer_escape(value: &str) -> Cow<'_, str> {
    if value.contains('~') || value.contains('/') {
        Cow::Owned(value.replace('~', "~0").replace('/', "~1"))
    } else {
        Cow::Borrowed(value)
    }
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// An imported property has no source RDF node. Preserve its source shape and
/// JSON Pointer as a collision-free blank-node label instead of minting an IRI.
/// This uses no shared counter, so adding a property does not renumber existing
/// nested node shapes.
fn property_shape_id(source: &Term, path: &str) -> Term {
    use std::fmt::Write as _;

    let source = source.to_string();
    let mut label = String::from("schema-property");
    for part in [source.as_str(), path] {
        label.push('-');
        for byte in part.bytes() {
            write!(label, "{byte:02x}").expect("writing to a String cannot fail");
        }
    }
    Term::blank(label)
}

impl ImportContext<'_> {
    fn audit_root(
        &mut self,
        root: &Map<String, Value>,
        generated_envelope: bool,
    ) -> Result<(), SchemaImportError> {
        for (keyword, value) in root {
            let path = format!("#/{}", pointer_escape(keyword));
            match keyword.as_str() {
                "$schema" | "$id" if generated_envelope => {}
                "$schema" | "$id" | "$anchor" | "$dynamicAnchor" => {
                    self.record("schema-identity-dropped", &path);
                }
                "title" if generated_envelope => {}
                "title" | "description" | "$comment" | "default" | "examples" | "deprecated"
                | "readOnly" | "writeOnly" => {
                    self.record("annotation-dropped", &path);
                }
                "$defs" => {}
                "type" | "anyOf" | "properties" if generated_envelope => {}
                keyword if is_known_assertion_keyword(keyword) => {
                    self.record(root_loss_code(keyword), &path);
                }
                keyword if is_annotation_keyword(keyword) => {
                    self.record("annotation-dropped", &path);
                }
                _ => {
                    validate_json_keyword_value(keyword, value, &path)?;
                    self.record("unknown-keyword-dropped", &path);
                }
            }
        }
        Ok(())
    }

    fn import_definition(
        &mut self,
        key: &str,
        schema: &Value,
        path: &str,
    ) -> Result<Option<Shape>, SchemaImportError> {
        if schema.is_boolean() {
            self.record("boolean-schema-dropped", path);
            return Ok(None);
        }
        let object = schema.as_object().ok_or_else(|| {
            SchemaImportError::new(format!("{path} must be an object or boolean schema"))
        })?;
        if !is_object_schema(object) {
            self.audit_non_object_definition(object, path)?;
            self.record("non-object-definition-dropped", path);
            return Ok(None);
        }
        let class_iri = self
            .config
            .namespaces
            .class_iri_for_def_key(key)
            .map_err(|error| SchemaImportError::new(format!("{path}: {error}")))?;
        let id = Term::NamedNode(NamedNode::new_unchecked(class_iri));
        let target = Target::ImplicitClass(id.clone());
        self.import_object_shape(object, path, id, vec![target])
            .map(Some)
    }

    fn audit_non_object_definition(
        &mut self,
        object: &Map<String, Value>,
        path: &str,
    ) -> Result<(), SchemaImportError> {
        for (keyword, value) in object {
            let location = format!("{path}/{}", pointer_escape(keyword));
            match keyword.as_str() {
                "$ref" => {}
                "title" | "description" | "$comment" | "default" | "examples" | "deprecated"
                | "readOnly" | "writeOnly" => {
                    self.record("annotation-dropped", &location);
                }
                "x-enum-varnames" | "x-enum-descriptions" => {
                    self.record("enum-metadata-dropped", &location);
                }
                keyword if is_known_assertion_keyword(keyword) => {
                    self.record(root_loss_code(keyword), &location);
                }
                keyword if is_annotation_keyword(keyword) => {
                    self.record("annotation-dropped", &location);
                }
                _ => {
                    validate_json_keyword_value(keyword, value, &location)?;
                    self.record("unknown-keyword-dropped", &location);
                }
            }
        }
        Ok(())
    }

    fn import_object_shape(
        &mut self,
        object: &Map<String, Value>,
        path: &str,
        id: Term,
        targets: Vec<Target>,
    ) -> Result<Shape, SchemaImportError> {
        validate_object_type(object.get("type"), path)?;
        if let Some(kind) = object.get("type")
            && schema_types(kind, &format!("{path}/type"))?.len() != 1
        {
            self.record("value-term-kind-widened", &format!("{path}/type"));
        }
        let properties = optional_object(object, "properties", path)?;
        if properties.len() > MAX_PROPERTIES {
            return Err(SchemaImportError::new(format!(
                "{path}/properties contains {} members; limit is {MAX_PROPERTIES}",
                properties.len()
            )));
        }
        let required = required_names(object, properties, path)?;
        let mut property_shapes = Vec::new();
        let mut closed_ignored = Vec::new();
        let mut imported_property_names = BTreeSet::new();
        let mut property_identities = BTreeMap::new();
        for (key, schema) in properties {
            if matches!(key.as_str(), "@id" | "@type" | "@annotation") {
                if !self.generated_envelope {
                    self.record(
                        "value-term-kind-widened",
                        &format!("{path}/properties/{}", pointer_escape(key)),
                    );
                }
                continue;
            }
            imported_property_names.insert(key.clone());
            let property_path = format!("{path}/properties/{}", pointer_escape(key));
            let predicate_iri = self
                .config
                .namespaces
                .expand_iri(key)
                .map_err(|error| SchemaImportError::new(format!("{property_path}: {error}")))?;
            if let Some(previous) =
                property_identities.insert(predicate_iri.clone(), property_path.clone())
            {
                return Err(SchemaImportError::new(format!(
                    "{property_path} and {previous} resolve to the same property IRI {predicate_iri:?}"
                )));
            }
            let predicate = NamedNode::new_unchecked(predicate_iri);
            if schema == &Value::Bool(true) && !required.contains(key) {
                closed_ignored.push(predicate);
                continue;
            }
            let mut constraints = self.import_property_schema(schema, &property_path)?;
            if required.contains(key)
                && !constraints.iter().any(
                    |constraint| matches!(constraint, Constraint::MinCount(value) if *value >= 1),
                )
            {
                constraints.push(Constraint::MinCount(1));
            }
            property_shapes.push(PropertyShape {
                id: property_shape_id(&id, &property_path),
                path: Path::Predicate(predicate),
                values: None,
                default_value: None,
                constraints,
                property_shapes: Vec::new(),
                reifier_shapes: Vec::new(),
                reification_required: false,
                severity: Severity::Violation,
                messages: vec![],
                constraint_annotations: vec![],
                deactivated: false,
                box_roles: Vec::new(),
            });
        }
        for key in required.difference(&imported_property_names) {
            if matches!(key.as_str(), "@id" | "@type" | "@annotation") {
                if !self.generated_envelope && !properties.contains_key(key) {
                    self.record("value-term-kind-widened", &format!("{path}/required"));
                }
                continue;
            }
            let predicate_iri = self
                .config
                .namespaces
                .expand_iri(key)
                .map_err(|error| SchemaImportError::new(format!("{path}/required: {error}")))?;
            let required_path = format!("{path}/required");
            if let Some(previous) =
                property_identities.insert(predicate_iri.clone(), required_path.clone())
            {
                return Err(SchemaImportError::new(format!(
                    "{required_path} property {key:?} and {previous} resolve to the same property IRI {predicate_iri:?}"
                )));
            }
            property_shapes.push(PropertyShape {
                id: property_shape_id(&id, &format!("{path}/properties/{}", pointer_escape(key))),
                path: Path::Predicate(NamedNode::new_unchecked(predicate_iri)),
                values: None,
                default_value: None,
                constraints: vec![Constraint::MinCount(1)],
                property_shapes: Vec::new(),
                reifier_shapes: Vec::new(),
                reification_required: false,
                severity: Severity::Violation,
                messages: vec![],
                constraint_annotations: vec![],
                deactivated: false,
                box_roles: Vec::new(),
            });
        }
        property_shapes.sort_by_cached_key(|property| match &property.path {
            Path::Predicate(predicate) => predicate.as_str().to_owned(),
            _ => unreachable!("schema imports create direct predicate paths only"),
        });

        let mut constraints = Vec::new();
        if let Some(class) = type_discriminator_class(object, &self.config.namespaces)? {
            constraints.push(Constraint::Class(vec![class]));
        }
        if let Some(additional) = object.get("additionalProperties") {
            match additional {
                Value::Bool(false) => {
                    // The projection carries `rdf:type` as the always-declared
                    // `@type`, never as a property, so a closed object permits it.
                    if !closed_ignored.iter().any(|n| n.as_str() == RDF_TYPE) {
                        closed_ignored.push(NamedNode::new_unchecked(RDF_TYPE));
                    }
                    constraints.push(Constraint::Closed {
                        ignored: closed_ignored,
                        mode: ClosedMode::Declared,
                    });
                }
                Value::Bool(true) => {}
                Value::Object(_) => self.record(
                    "additional-properties-schema-widened",
                    &format!("{path}/additionalProperties"),
                ),
                _ => {
                    return Err(SchemaImportError::new(format!(
                        "{path}/additionalProperties must be a boolean or schema"
                    )));
                }
            }
        }
        self.import_object_logic(object, path, &mut constraints)?;
        self.audit_object_keywords(object, path)?;
        Ok(Shape {
            id,
            targets,
            constraints,
            property_shapes,
            severity: Severity::Violation,
            messages: vec![],
            constraint_annotations: vec![],
            deactivated: false,
            box_roles: Vec::new(),
            rules: Vec::new(),
        })
    }

    fn import_object_logic(
        &mut self,
        object: &Map<String, Value>,
        path: &str,
        constraints: &mut Vec<Constraint>,
    ) -> Result<(), SchemaImportError> {
        if let Some(branches) = object.get("allOf") {
            let branches = branches
                .as_array()
                .ok_or_else(|| SchemaImportError::new(format!("{path}/allOf must be an array")))?;
            let mut members = Vec::new();
            for (index, branch) in branches.iter().enumerate() {
                let branch_path = format!("{path}/allOf/{index}");
                if self.import_node_list(branch, &branch_path, constraints)? {
                    continue;
                }
                if let Some(negand) = branch
                    .as_object()
                    .and_then(|map| (map.len() == 1).then(|| map.get("not")).flatten())
                {
                    if let Some(inner) =
                        self.import_nested_shape(negand, &format!("{branch_path}/not"))?
                    {
                        constraints.push(Constraint::Not(Box::new(inner)));
                    }
                } else if let Some(member) = self.import_nested_shape(branch, &branch_path)? {
                    members.push(member);
                }
            }
            if !members.is_empty() {
                constraints.push(Constraint::And(members));
            }
        }
        for (keyword, xone) in [("anyOf", false), ("oneOf", true)] {
            let Some(branches) = object.get(keyword) else {
                continue;
            };
            let branches = branches.as_array().ok_or_else(|| {
                SchemaImportError::new(format!("{path}/{keyword} must be an array"))
            })?;
            let mut members = Vec::new();
            for (index, branch) in branches.iter().enumerate() {
                if let Some(member) =
                    self.import_nested_shape(branch, &format!("{path}/{keyword}/{index}"))?
                {
                    members.push(member);
                }
            }
            if !members.is_empty() {
                constraints.push(if xone {
                    Constraint::Xone(members)
                } else {
                    Constraint::Or(members)
                });
            }
        }
        if let Some(negand) = object.get("not")
            && let Some(inner) = self.import_nested_shape(negand, &format!("{path}/not"))?
        {
            constraints.push(Constraint::Not(Box::new(inner)));
        }
        Ok(())
    }

    fn import_nested_shape(
        &mut self,
        schema: &Value,
        path: &str,
    ) -> Result<Option<Shape>, SchemaImportError> {
        let Some(object) = schema.as_object() else {
            self.record("schema-applicator-dropped", path);
            return Ok(None);
        };
        if !is_object_schema(object) {
            self.record("schema-applicator-dropped", path);
            return Ok(None);
        }
        let id = self.nested_shape_id(path);
        self.import_object_shape(object, path, id, Vec::new())
            .map(Some)
    }

    fn audit_object_keywords(
        &mut self,
        object: &Map<String, Value>,
        path: &str,
    ) -> Result<(), SchemaImportError> {
        for (keyword, value) in object {
            let location = format!("{path}/{}", pointer_escape(keyword));
            match keyword.as_str() {
                "type"
                | "properties"
                | "required"
                | "additionalProperties"
                | "allOf"
                | "anyOf"
                | "oneOf"
                | "not" => {}
                "title" | "description" | "$comment" | "default" | "examples" | "deprecated"
                | "readOnly" | "writeOnly" => {
                    self.record("annotation-dropped", &location);
                }
                "$id" | "$anchor" | "$dynamicAnchor" => {
                    self.record("schema-identity-dropped", &location);
                }
                "patternProperties" | "propertyNames" => {
                    self.record("object-key-validation-dropped", &location);
                }
                "minProperties" | "maxProperties" => {
                    validate_nonnegative_integer(value, &location)?;
                    self.record("property-count-validation-dropped", &location);
                }
                "dependentRequired" | "dependentSchemas" => {
                    self.record("dependency-validation-dropped", &location);
                }
                "if" | "then" | "else" => {
                    self.record("conditional-validation-dropped", &location);
                }
                "unevaluatedProperties" => {
                    self.record("unevaluated-validation-dropped", &location);
                }
                keyword if is_annotation_keyword(keyword) => {
                    self.record("annotation-dropped", &location);
                }
                _ => {
                    validate_json_keyword_value(keyword, value, &location)?;
                    self.record("unknown-keyword-dropped", &location);
                }
            }
        }
        Ok(())
    }

    fn import_property_schema(
        &mut self,
        schema: &Value,
        path: &str,
    ) -> Result<Vec<Constraint>, SchemaImportError> {
        if schema == &Value::Bool(false) {
            return Ok(vec![Constraint::MaxCount(0)]);
        }
        if schema == &Value::Bool(true) {
            return Ok(Vec::new());
        }
        let (scalar, min_count, max_count, has_values) = self.split_cardinality(schema, path)?;
        let mut constraints = Vec::new();
        if let Some(minimum) = min_count {
            constraints.push(Constraint::MinCount(minimum));
        }
        if let Some(maximum) = max_count {
            constraints.push(Constraint::MaxCount(maximum));
        }
        for (index, value) in has_values.iter().enumerate() {
            if let Some(term) = self.import_term(value, &format!("{path}/contains/{index}"))? {
                constraints.push(Constraint::HasValue(term));
            }
        }
        self.import_scalar_schema(&scalar, path, &mut constraints)?;
        Ok(constraints)
    }

    /// Split a property schema into its per-value schema, its cardinality, and
    /// the `sh:hasValue` terms an array form states as `contains` (a lone value
    /// then states each as `const`, which the per-value schema returned here
    /// omits, since it constrains every value).
    fn split_cardinality(
        &mut self,
        schema: &Value,
        path: &str,
    ) -> Result<CardinalitySplit, SchemaImportError> {
        let object = schema.as_object().ok_or_else(|| {
            SchemaImportError::new(format!("{path} must be a boolean or object schema"))
        })?;
        if is_array_schema(object) {
            return self.array_cardinality(object, path);
        }
        if let Some(branches) = object.get("anyOf").and_then(Value::as_array)
            && branches.len() == 2
        {
            let left_array = branches[0]
                .as_object()
                .filter(|branch| is_array_schema(branch));
            let right_array = branches[1]
                .as_object()
                .filter(|branch| is_array_schema(branch));
            let (single, array, array_path) = match (left_array, right_array) {
                (Some(array), None) => (&branches[1], array, format!("{path}/anyOf/0")),
                (None, Some(array)) => (&branches[0], array, format!("{path}/anyOf/1")),
                _ => return Ok((schema.clone(), None, Some(1), Vec::new())),
            };
            let (items, min_count, max_count, has_values) =
                self.array_cardinality(array, &array_path)?;
            if &with_has_values(&items, &has_values) != single {
                return Err(SchemaImportError::new(format!(
                    "{path}/anyOf single-value branch must equal the array items schema"
                )));
            }
            return Ok((items, min_count, max_count, has_values));
        }
        Ok((schema.clone(), None, Some(1), Vec::new()))
    }

    fn array_cardinality(
        &mut self,
        object: &Map<String, Value>,
        path: &str,
    ) -> Result<CardinalitySplit, SchemaImportError> {
        // Omitting `items` is the draft-2020-12 identity schema. Keep the
        // cardinality carrier without inventing an item constraint.
        let items = object.get("items").cloned().unwrap_or(Value::Bool(true));
        // `contains: {const: v}` — alone, or one per term under `allOf` — is how
        // the emitter states `sh:hasValue v` over several values.
        let has_values = contained_constants(object);
        let min_count = object
            .get("minItems")
            .map(|value| nonnegative_integer(value, &format!("{path}/minItems")))
            .transpose()?;
        let max_count = object
            .get("maxItems")
            .map(|value| nonnegative_integer(value, &format!("{path}/maxItems")))
            .transpose()?;
        let contains_is_has_value = has_values.is_some();
        let has_values = has_values.unwrap_or_default();
        for (keyword, code) in [
            ("contains", "array-contains-validation-dropped"),
            ("minContains", "array-contains-validation-dropped"),
            ("maxContains", "array-contains-validation-dropped"),
            ("prefixItems", "tuple-validation-widened"),
            ("additionalItems", "tuple-validation-widened"),
            ("unevaluatedItems", "unevaluated-validation-dropped"),
            ("uniqueItems", "unique-items-validation-dropped"),
        ] {
            if object.contains_key(keyword) && !(contains_is_has_value && keyword == "contains") {
                self.record(code, &format!("{path}/{keyword}"));
            }
        }
        for (keyword, value) in object {
            let location = format!("{path}/{}", pointer_escape(keyword));
            match keyword.as_str() {
                "allOf" if contains_is_has_value => {}
                "type" | "items" | "minItems" | "maxItems" | "contains" | "minContains"
                | "maxContains" | "prefixItems" | "additionalItems" | "unevaluatedItems"
                | "uniqueItems" => {}
                "title" | "description" | "$comment" | "default" | "examples" | "deprecated"
                | "readOnly" | "writeOnly" => {
                    self.record("annotation-dropped", &location);
                }
                _ => {
                    validate_json_keyword_value(keyword, value, &location)?;
                    self.record("unknown-keyword-dropped", &location);
                }
            }
        }
        Ok((items, min_count, max_count, has_values))
    }

    fn import_scalar_schema(
        &mut self,
        schema: &Value,
        path: &str,
        constraints: &mut Vec<Constraint>,
    ) -> Result<(), SchemaImportError> {
        match schema {
            Value::Bool(true) => return Ok(()),
            Value::Bool(false) => {
                self.record("boolean-schema-dropped", path);
                return Ok(());
            }
            Value::Object(_) => {}
            _ => {
                return Err(SchemaImportError::new(format!(
                    "{path} must be an object or boolean schema"
                )));
            }
        }
        let object = schema.as_object().expect("matched object");
        let mut handled = BTreeSet::new();

        if let Some(datatype) = self.value_datatype(schema, path)? {
            constraints.push(Constraint::Datatype(vec![datatype]));
            return Ok(());
        }
        if let Some(kinds) = node_kind_union(std::slice::from_ref(schema)) {
            constraints.push(Constraint::NodeKind(kinds));
            return Ok(());
        }
        if is_language_literal_schema(object) {
            if let Some(tags) = language_tags(object) {
                constraints.push(Constraint::LanguageIn(tags));
            } else {
                self.record("value-term-kind-widened", path);
            }
            return Ok(());
        }
        if is_node_ref_schema(schema) {
            if let Some(class) = node_ref_class(object, &self.config.namespaces)? {
                constraints.push(Constraint::Class(vec![class]));
            } else {
                if object.contains_key("$comment") {
                    self.record("annotation-dropped", &format!("{path}/$comment"));
                }
                constraints.push(Constraint::NodeKind(vec![NodeKindValue::BlankNodeOrIri]));
            }
            return Ok(());
        }

        if let Some(reference) = object.get("$ref") {
            let reference = reference
                .as_str()
                .ok_or_else(|| SchemaImportError::new(format!("{path}/$ref must be a string")))?;
            self.import_reference(reference, &format!("{path}/$ref"), constraints)?;
            handled.insert("$ref");
        }

        if let Some(branches) = object.get("allOf") {
            let branches = branches
                .as_array()
                .ok_or_else(|| SchemaImportError::new(format!("{path}/allOf must be an array")))?;
            for (index, branch) in branches.iter().enumerate() {
                self.import_scalar_schema(branch, &format!("{path}/allOf/{index}"), constraints)?;
            }
            handled.insert("allOf");
        }

        if let Some(branches) = object.get("anyOf") {
            let branches = branches
                .as_array()
                .ok_or_else(|| SchemaImportError::new(format!("{path}/anyOf must be an array")))?;
            self.import_scalar_alternatives(branches, &format!("{path}/anyOf"), constraints)?;
            handled.insert("anyOf");
        }

        if let Some(enum_values) = object.get("enum") {
            let enum_values = enum_values
                .as_array()
                .ok_or_else(|| SchemaImportError::new(format!("{path}/enum must be an array")))?;
            let mut terms = Vec::new();
            for (index, value) in enum_values.iter().enumerate() {
                if let Some(term) = self.import_term(value, &format!("{path}/enum/{index}"))? {
                    terms.push(term);
                }
            }
            crate::term::sort_terms_canonical(&mut terms);
            terms.dedup();
            constraints.push(Constraint::In(terms));
            handled.insert("enum");
        }
        if let Some(value) = object.get("const") {
            if let Some(term) = self.import_term(value, &format!("{path}/const"))? {
                constraints.push(Constraint::HasValue(term));
            }
            handled.insert("const");
        }

        if let Some(kind) = object.get("type") {
            self.import_scalar_type(kind, object.get("format"), path, constraints)?;
            handled.insert("type");
            if object.contains_key("format") {
                handled.insert("format");
            }
        } else if object.contains_key("format") {
            self.record("format-validation-widened", &format!("{path}/format"));
            handled.insert("format");
        }

        if let Some(pattern) = object.get("pattern") {
            let regex = pattern.as_str().ok_or_else(|| {
                SchemaImportError::new(format!("{path}/pattern must be a string"))
            })?;
            constraints.push(Constraint::Pattern {
                regex: purrdf_core::xsd_regex::from_ecma_262(regex).map_err(|error| {
                    SchemaImportError::new(format!("cannot import {path}/pattern: {error}"))
                })?,
                flags: None,
                compiled: Arc::new(OnceLock::new()),
            });
            handled.insert("pattern");
        }
        for (keyword, constructor) in [
            ("minLength", Constraint::MinLength as fn(u64) -> Constraint),
            ("maxLength", Constraint::MaxLength as fn(u64) -> Constraint),
        ] {
            if let Some(value) = object.get(keyword) {
                constraints.push(constructor(nonnegative_integer(
                    value,
                    &format!("{path}/{keyword}"),
                )?));
                handled.insert(keyword);
            }
        }
        for (keyword, bound_kind) in [
            ("minimum", BoundKind::Minimum),
            ("maximum", BoundKind::Maximum),
            ("exclusiveMinimum", BoundKind::ExclusiveMinimum),
            ("exclusiveMaximum", BoundKind::ExclusiveMaximum),
        ] {
            if let Some(value) = object.get(keyword) {
                let term = self.numeric_term(value, &format!("{path}/{keyword}"))?;
                constraints.push(bound_kind.constraint(term));
                handled.insert(keyword);
            }
        }
        if let Some(negand) = object.get("not") {
            self.import_rejections(negand, &format!("{path}/not"), constraints)?;
            handled.insert("not");
        }
        if object.contains_key("multipleOf") {
            self.record(
                "multiple-of-validation-dropped",
                &format!("{path}/multipleOf"),
            );
            handled.insert("multipleOf");
        }

        for (keyword, value) in object {
            if handled.contains(keyword.as_str()) {
                continue;
            }
            let location = format!("{path}/{}", pointer_escape(keyword));
            match keyword.as_str() {
                "title" | "description" | "$comment" | "default" | "examples" | "deprecated"
                | "readOnly" | "writeOnly" => {
                    self.record("annotation-dropped", &location);
                }
                "x-enum-varnames" | "x-enum-descriptions" => {
                    self.record("enum-metadata-dropped", &location);
                }
                "oneOf" | "not" => self.record("schema-applicator-dropped", &location),
                "if" | "then" | "else" => {
                    self.record("conditional-validation-dropped", &location);
                }
                "contentEncoding" | "contentMediaType" | "contentSchema" => {
                    self.record("content-validation-dropped", &location);
                }
                keyword if is_annotation_keyword(keyword) => {
                    self.record("annotation-dropped", &location);
                }
                _ => {
                    validate_json_keyword_value(keyword, value, &location)?;
                    self.record("unknown-keyword-dropped", &location);
                }
            }
        }
        Ok(())
    }

    /// The datatype whose value schema `schema` is exactly — the emitter's
    /// projection of one `sh:datatype` value — or `None`.
    fn value_datatype(
        &self,
        schema: &Value,
        path: &str,
    ) -> Result<Option<NamedNode>, SchemaImportError> {
        if let Some(curie) = typed_datatype_constant(schema) {
            return self.expand_datatype(curie, path).map(Some);
        }
        if is_lang_string_schema(schema) {
            return Ok(Some(NamedNode::new_unchecked(RDF_LANG_STRING)));
        }
        if is_dir_lang_string_schema(schema) {
            return Ok(Some(NamedNode::new_unchecked(RDF_DIR_LANG_STRING)));
        }
        if let Some([scalar, typed]) = schema
            .get("anyOf")
            .and_then(Value::as_array)
            .and_then(|branches| <&[Value; 2]>::try_from(branches.as_slice()).ok())
            .filter(|_| schema.as_object().is_some_and(|object| object.len() == 1))
            && let Some(curie) = typed_datatype_constant(typed)
            && ["integer", "number", "boolean"]
                .iter()
                .any(|kind| is_exact_type_schema(scalar, kind))
        {
            return self.expand_datatype(curie, path).map(Some);
        }
        Ok(None)
    }

    fn expand_datatype(&self, curie: &str, path: &str) -> Result<NamedNode, SchemaImportError> {
        self.config
            .namespaces
            .expand_iri(curie)
            .map(NamedNode::new_unchecked)
            .map_err(|error| SchemaImportError::new(format!("{path} datatype {curie:?}: {error}")))
    }

    /// Import a value schema's `not`: the projected values some constraint
    /// rejects, as the emitter states them.
    ///
    /// Each alternative is read back as the constraint that rejects it — a
    /// `sh:singleLine true` line break, a `sh:pattern` a string or `@value` fails,
    /// a `sh:minLength` / `sh:maxLength` bound an `@value` breaks — or is
    /// recognized as implied by constraints already read (a blank node, which
    /// every lexical-form constraint rejects; a value that no numeric bound can
    /// compare). Anything else is a negation SHACL cannot state here, and its loss
    /// is recorded.
    fn import_rejections(
        &mut self,
        negand: &Value,
        path: &str,
        constraints: &mut Vec<Constraint>,
    ) -> Result<(), SchemaImportError> {
        let (alternatives, alternative_path) = match negand.get("anyOf").and_then(Value::as_array) {
            Some(branches) if negand.as_object().is_some_and(|object| object.len() == 1) => {
                (branches.clone(), format!("{path}/anyOf"))
            }
            _ => (vec![negand.clone()], path.to_owned()),
        };
        let lexical = |constraints: &[Constraint]| {
            constraints.iter().any(|constraint| {
                matches!(
                    constraint,
                    Constraint::Pattern { .. }
                        | Constraint::MinLength(_)
                        | Constraint::MaxLength(_)
                )
            })
        };
        let bounded = |constraints: &[Constraint]| {
            constraints.iter().any(|constraint| {
                matches!(
                    constraint,
                    Constraint::MinInclusive(_)
                        | Constraint::MaxInclusive(_)
                        | Constraint::MinExclusive(_)
                        | Constraint::MaxExclusive(_)
                )
            })
        };
        let mut derived: Vec<Constraint> = Vec::new();
        let mut implied_blank = false;
        let mut implied_triple = false;
        let mut implied_bound = false;
        let mut implied_constant = false;
        let mut typed_bounds: Vec<(String, &Value)> = Vec::new();
        let mut unread: Vec<String> = Vec::new();
        for (index, alternative) in alternatives.iter().enumerate() {
            let location = if alternatives.len() == 1 && alternative_path == path {
                path.to_owned()
            } else {
                format!("{alternative_path}/{index}")
            };
            match classify_rejection(alternative) {
                Some(Rejection::LineBreak) => {
                    if !derived
                        .iter()
                        .any(|constraint| matches!(constraint, Constraint::SingleLine(true)))
                    {
                        derived.push(Constraint::SingleLine(true));
                    }
                }
                Some(Rejection::Pattern(pattern)) => {
                    let regex =
                        purrdf_core::xsd_regex::from_ecma_262(&pattern).map_err(|error| {
                            SchemaImportError::new(format!("cannot import {location}: {error}"))
                        })?;
                    let known = constraints.iter().chain(&derived).any(|constraint| {
                        matches!(constraint, Constraint::Pattern { regex: existing, .. } if *existing == regex)
                    });
                    if !known {
                        derived.push(Constraint::Pattern {
                            regex,
                            flags: None,
                            compiled: Arc::new(OnceLock::new()),
                        });
                    }
                }
                Some(Rejection::MinLength(n)) => {
                    if !constraints
                        .iter()
                        .any(|constraint| matches!(constraint, Constraint::MinLength(m) if *m == n))
                    {
                        derived.push(Constraint::MinLength(n));
                    }
                }
                Some(Rejection::MaxLength(n)) => {
                    if !constraints
                        .iter()
                        .any(|constraint| matches!(constraint, Constraint::MaxLength(m) if *m == n))
                    {
                        derived.push(Constraint::MaxLength(n));
                    }
                }
                Some(Rejection::BlankNode) => implied_blank = true,
                Some(Rejection::TripleTerm) => implied_triple = true,
                Some(Rejection::NonNumeric) => implied_bound = true,
                Some(Rejection::Implied) => implied_constant = true,
                Some(Rejection::TypedBound) => typed_bounds.push((location, alternative)),
                None => unread.push(location),
            }
        }
        let has_lexical = lexical(constraints) || lexical(&derived);
        constraints.extend(derived);
        if implied_constant && !has_lexical && !bounded(constraints) {
            unread.push(path.to_owned());
        }
        // A typed bound rejection reads back only as exactly the one the bounds
        // already read project to; any other is a negation SHACL cannot state
        // here.
        let expected = crate::json_schema::expected_typed_bound_rejections(
            constraints,
            &self.config.namespaces,
        );
        for (location, alternative) in typed_bounds {
            if !expected.contains(alternative) {
                unread.push(location);
            }
        }
        if implied_blank && !has_lexical {
            unread.push(path.to_owned());
        }
        if implied_bound && !bounded(constraints) {
            unread.push(path.to_owned());
        }
        if implied_triple && !has_lexical && !bounded(constraints) {
            unread.push(path.to_owned());
        }
        unread.sort();
        unread.dedup();
        for location in unread {
            self.record("schema-applicator-dropped", &location);
        }
        Ok(())
    }

    fn import_reference(
        &mut self,
        reference: &str,
        path: &str,
        constraints: &mut Vec<Constraint>,
    ) -> Result<(), SchemaImportError> {
        let key = reference_key(reference).ok_or_else(|| {
            SchemaImportError::new(format!(
                "{path} must be a direct local #/$defs reference, got {reference:?}"
            ))
        })?;
        let target = self.definitions.get(&key).ok_or_else(|| {
            SchemaImportError::new(format!("{path} targets missing definition {key:?}"))
        })?;
        if target.as_object().is_some_and(is_object_schema) {
            let iri = self
                .config
                .namespaces
                .class_iri_for_def_key(&key)
                .map_err(|error| SchemaImportError::new(format!("{path}: {error}")))?;
            constraints.push(Constraint::Class(vec![NamedNode::new_unchecked(iri)]));
            return Ok(());
        }
        if let Some(values) = target
            .as_object()
            .and_then(|object| object.get("enum"))
            .and_then(Value::as_array)
        {
            let mut terms = Vec::new();
            for (index, value) in values.iter().enumerate() {
                if let Some(term) =
                    self.import_term(value, &format!("{}/enum/{index}", definition_path(&key)))?
                {
                    terms.push(term);
                }
            }
            crate::term::sort_terms_canonical(&mut terms);
            terms.dedup();
            constraints.push(Constraint::In(terms));
            self.record("non-object-definition-dropped", &definition_path(&key));
            return Ok(());
        }
        self.record("non-object-definition-dropped", &definition_path(&key));
        Ok(())
    }

    fn import_scalar_alternatives(
        &mut self,
        branches: &[Value],
        path: &str,
        constraints: &mut Vec<Constraint>,
    ) -> Result<(), SchemaImportError> {
        // A node-kind union (SHACL 1.2: a list value is a disjunction).
        if let Some(kinds) = node_kind_union(branches) {
            constraints.push(Constraint::NodeKind(kinds));
            return Ok(());
        }
        // One datatype's scalar-or-typed-object pair, or a list of datatypes.
        if let Some(datatype) = self.value_datatype(&json_any_of(branches), path)? {
            constraints.push(Constraint::Datatype(vec![datatype]));
            return Ok(());
        }
        if branches.len() >= 2 {
            let mut datatypes = Vec::new();
            for (index, branch) in branches.iter().enumerate() {
                match self.value_datatype(branch, &format!("{path}/{index}"))? {
                    Some(datatype) => datatypes.push(datatype),
                    None => {
                        datatypes.clear();
                        break;
                    }
                }
            }
            if !datatypes.is_empty() {
                constraints.push(Constraint::Datatype(datatypes));
                return Ok(());
            }
        }
        if branches.len() == 2 {
            let typed = branches.iter().position(is_typed_literal_schema);
            if let Some(typed_index) = typed {
                let scalar_index = usize::from(typed_index == 0);
                if let Some(scalar) = branches[scalar_index].as_object()
                    && is_simple_scalar_carrier(scalar)
                    && let Some(kind) = scalar.get("type")
                {
                    self.import_scalar_type(
                        kind,
                        scalar.get("format"),
                        &format!("{path}/{scalar_index}"),
                        constraints,
                    )?;
                    return Ok(());
                }
            }
        }

        // The list components: a list object judging the members, beside the
        // node reference a list kept as linked nodes projects as.
        if let [left, right] = branches {
            let pair = match (list_members(left), list_members(right)) {
                (Some(members), None) if is_plain_node_ref(right) => Some((members, 0)),
                (None, Some(members)) if is_plain_node_ref(left) => Some((members, 1)),
                _ => None,
            };
            if let Some((members, index)) = pair {
                let members_path = format!("{path}/{index}/properties/@list");
                self.import_list_members(members, &members_path, 0, constraints)?;
                return Ok(());
            }
        }

        let mut node_branches = 0_usize;
        let mut nil_branches = 0_usize;
        let mut typed_branches = 0_usize;
        let mut language_branches = 0_usize;
        let mut reference_branches = 0_usize;
        let mut scalar_branches = 0_usize;
        let mut saw_true = false;
        let mut unsupported = false;
        let mut node_constraints = Vec::new();
        let mut reference_constraints = Vec::new();
        let mut scalar_constraints = Vec::new();
        for (index, branch) in branches.iter().enumerate() {
            let branch_path = format!("{path}/{index}");
            if branch == &Value::Bool(true) {
                saw_true = true;
                continue;
            }
            if branch == &Value::Bool(false) {
                continue;
            }
            // `rdf:nil`'s empty list, which a class admits as it admits node
            // references (see the emitter's `sh:class` projection).
            if list_form(branch) == Some(ListForm::Nil) {
                nil_branches += 1;
                continue;
            }
            if is_node_ref_schema(branch) {
                node_branches += 1;
                let object = branch.as_object().expect("node carrier is an object");
                if let Some(class) = node_ref_class(object, &self.config.namespaces)? {
                    node_constraints.push(Constraint::Class(vec![class]));
                } else {
                    if object.contains_key("$comment") {
                        self.record("annotation-dropped", &format!("{branch_path}/$comment"));
                    }
                    node_constraints
                        .push(Constraint::NodeKind(vec![NodeKindValue::BlankNodeOrIri]));
                }
                continue;
            }
            if is_typed_literal_schema(branch) {
                typed_branches += 1;
                continue;
            }
            let Some(object) = branch.as_object() else {
                unreachable!("reference validation accepts only object or boolean schemas");
            };
            if is_language_literal_schema(object) {
                language_branches += 1;
                if let Some(tags) = language_tags(object) {
                    scalar_constraints.push(Constraint::LanguageIn(tags));
                } else {
                    self.record("value-term-kind-widened", &branch_path);
                }
                continue;
            }
            if let Some(reference) = object.get("$ref").and_then(Value::as_str) {
                reference_branches += 1;
                self.import_reference(
                    reference,
                    &format!("{branch_path}/$ref"),
                    &mut reference_constraints,
                )?;
                continue;
            }
            if object.get("type").is_some() {
                scalar_branches += 1;
                if !is_simple_scalar_carrier(object) {
                    unsupported = true;
                }
                self.import_scalar_schema(branch, &branch_path, &mut scalar_constraints)?;
                continue;
            }
            let mut discarded = Vec::new();
            self.import_scalar_schema(branch, &branch_path, &mut discarded)?;
            unsupported = true;
        }

        if saw_true {
            return Ok(());
        }
        if nil_branches > 0 && node_branches + reference_branches == 0 {
            unsupported = true;
        }
        let effective_branches = node_branches
            + typed_branches
            + language_branches
            + reference_branches
            + scalar_branches
            + usize::from(unsupported);
        if effective_branches == 0 {
            self.record("boolean-schema-dropped", path);
            return Ok(());
        }
        if !unsupported && effective_branches == 1 {
            if node_branches == 1 {
                constraints.extend(node_constraints);
            } else if typed_branches == 1 {
                constraints.push(Constraint::NodeKind(vec![NodeKindValue::Literal]));
            } else if reference_branches == 1 {
                constraints.extend(reference_constraints);
            } else {
                constraints.extend(scalar_constraints);
            }
            return Ok(());
        }
        if !unsupported
            && reference_branches == 1
            && node_branches > 0
            && effective_branches == reference_branches + node_branches
        {
            constraints.extend(reference_constraints);
            return Ok(());
        }
        if !unsupported
            && reference_branches == 0
            && node_branches > 0
            && typed_branches > 0
            && scalar_branches > 0
            && language_branches == 0
            && effective_branches
                == node_branches + typed_branches + language_branches + scalar_branches
        {
            constraints.push(Constraint::NodeKind(vec![NodeKindValue::IriOrLiteral]));
            return Ok(());
        }
        self.record("schema-applicator-dropped", path);
        Ok(())
    }

    /// Read a list's members schema back as the SHACL 1.2 list components:
    /// `minItems`/`maxItems` (offset by `skipped` members judged elsewhere) as
    /// `sh:minListLength`/`sh:maxListLength`, `uniqueItems` as
    /// `sh:uniqueMembers true`, `items` as a `sh:memberShape` whose own
    /// constraints are the items schema's. A list with none of them requires
    /// only that the value be a list, which `sh:uniqueMembers false` states.
    fn import_list_members(
        &mut self,
        members: &Map<String, Value>,
        path: &str,
        skipped: u64,
        constraints: &mut Vec<Constraint>,
    ) -> Result<(), SchemaImportError> {
        let start = constraints.len();
        for (keyword, value) in members {
            let location = format!("{path}/{}", pointer_escape(keyword));
            match keyword.as_str() {
                "type" => {}
                "minItems" => {
                    let n = nonnegative_integer(value, &location)?;
                    constraints.push(Constraint::MinListLength(n + skipped));
                }
                "maxItems" => {
                    let n = nonnegative_integer(value, &location)?;
                    constraints.push(Constraint::MaxListLength(n + skipped));
                }
                "uniqueItems" if value == &Value::Bool(true) => {
                    constraints.push(Constraint::UniqueMembers(true));
                }
                "items" => {
                    let shape = self.import_member_shape(value, &location)?;
                    constraints.push(Constraint::MemberShape(Box::new(shape)));
                }
                "$comment" => self.record("annotation-dropped", &location),
                _ => {
                    validate_json_keyword_value(keyword, value, &location)?;
                    self.record("unknown-keyword-dropped", &location);
                }
            }
        }
        if constraints.len() == start {
            constraints.push(Constraint::UniqueMembers(false));
        }
        Ok(())
    }

    /// The member shape whose value schema `schema` is.
    fn import_member_shape(
        &mut self,
        schema: &Value,
        path: &str,
    ) -> Result<Shape, SchemaImportError> {
        let mut constraints = Vec::new();
        self.import_scalar_schema(schema, path, &mut constraints)?;
        Ok(Shape {
            id: self.nested_shape_id(path),
            targets: Vec::new(),
            constraints,
            property_shapes: Vec::new(),
            severity: Severity::Violation,
            messages: vec![],
            constraint_annotations: vec![],
            deactivated: false,
            box_roles: Vec::new(),
            rules: Vec::new(),
        })
    }

    /// Read a node shape's list conjunct back (see the emitter's
    /// `ListComponents::node_schema`): `rdf:nil`, and a cell whose `rdf:first`
    /// is one member and whose `rdf:rest` holds the others.
    fn import_node_list(
        &mut self,
        branch: &Value,
        path: &str,
        constraints: &mut Vec<Constraint>,
    ) -> Result<bool, SchemaImportError> {
        let Some(alternatives) = branch
            .as_object()
            .filter(|object| object.len() == 1)
            .and_then(|object| object.get("anyOf"))
            .and_then(Value::as_array)
        else {
            return Ok(false);
        };
        let first = self
            .config
            .namespaces
            .compact_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#first");
        let rest = self
            .config
            .namespaces
            .compact_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#rest");
        let nil = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";
        let nil_form = serde_json::json!({
            "type": "object",
            "properties": {
                "@id": { "const": nil },
                first.clone(): false,
                rest.clone(): false
            },
            "required": ["@id"]
        });
        let mut has_nil = false;
        let mut cons: Option<(usize, &Map<String, Value>)> = None;
        for (index, alternative) in alternatives.iter().enumerate() {
            if alternative == &nil_form {
                has_nil = true;
                continue;
            }
            let Some(object) = alternative.as_object() else {
                return Ok(false);
            };
            let shaped = object.len() == 4
                && object.get("type").and_then(Value::as_str) == Some("object")
                && is_exact_required(object.get("required"), &[first.as_str(), rest.as_str()])
                && object.get("not")
                    == Some(&serde_json::json!({
                        "properties": { "@id": { "const": nil } },
                        "required": ["@id"]
                    }));
            if !shaped || cons.is_some() {
                return Ok(false);
            }
            cons = Some((index, object));
        }
        let mut read = Vec::new();
        match cons {
            None if has_nil => read.push(Constraint::MaxListLength(0)),
            None => return Ok(false),
            Some((index, object)) => {
                let base = format!("{path}/anyOf/{index}/properties");
                let properties = object
                    .get("properties")
                    .and_then(Value::as_object)
                    .filter(|properties| properties.len() == 2);
                let Some(properties) = properties else {
                    return Ok(false);
                };
                let single =
                    serde_json::json!({ "type": ["boolean", "number", "object", "string"] });
                let head = match properties.get(&first) {
                    Some(schema) if schema == &single => None,
                    Some(schema) => match schema
                        .get("allOf")
                        .and_then(Value::as_array)
                        .map(Vec::as_slice)
                    {
                        Some([only, member]) if only == &single => Some(member),
                        _ => return Ok(false),
                    },
                    None => return Ok(false),
                };
                let tail = properties
                    .get(&rest)
                    .and_then(|schema| schema.get("anyOf"))
                    .and_then(Value::as_array);
                let Some([tail_list, tail_node]) = tail.map(Vec::as_slice) else {
                    return Ok(false);
                };
                let Some(members) = list_members(tail_list) else {
                    return Ok(false);
                };
                if !is_plain_node_ref(tail_node) {
                    return Ok(false);
                }
                let tail_path =
                    format!("{base}/{}/anyOf/0/properties/@list", pointer_escape(&rest));
                let mut listed = Vec::new();
                self.import_list_members(members, &tail_path, 1, &mut listed)?;
                // The tail's own member shape repeats the head's; a lone
                // `sh:uniqueMembers false` stands for the list requirement.
                listed.retain(|constraint| {
                    !matches!(
                        constraint,
                        Constraint::MemberShape(_) | Constraint::UniqueMembers(false)
                    )
                });
                if !has_nil
                    && !listed
                        .iter()
                        .any(|c| matches!(c, Constraint::MinListLength(_)))
                {
                    listed.push(Constraint::MinListLength(1));
                }
                if let Some(member) = head {
                    let shape = self.import_member_shape(
                        member,
                        &format!("{base}/{}/allOf/1", pointer_escape(&first)),
                    )?;
                    listed.push(Constraint::MemberShape(Box::new(shape)));
                }
                if listed.is_empty() {
                    listed.push(Constraint::UniqueMembers(false));
                }
                read.extend(listed);
            }
        }
        constraints.extend(read);
        Ok(true)
    }

    fn import_scalar_type(
        &mut self,
        value: &Value,
        format: Option<&Value>,
        path: &str,
        constraints: &mut Vec<Constraint>,
    ) -> Result<(), SchemaImportError> {
        let kinds = schema_types(value, &format!("{path}/type"))?;
        let format = format
            .map(|value| {
                value.as_str().ok_or_else(|| {
                    SchemaImportError::new(format!("{path}/format must be a string"))
                })
            })
            .transpose()?;
        let mut datatypes = BTreeSet::new();
        let mut has_unmapped_kind = false;
        for kind in kinds {
            let Some(datatype) = self.config.datatypes.for_scalar(&kind, format) else {
                has_unmapped_kind = true;
                continue;
            };
            if kind == "string"
                && format.is_some()
                && !matches!(format, Some("date-time" | "date" | "time" | "uri"))
            {
                self.record("format-validation-widened", &format!("{path}/format"));
            }
            datatypes.insert(datatype);
        }
        if has_unmapped_kind || datatypes.len() > 1 {
            self.record("value-term-kind-widened", &format!("{path}/type"));
        } else if let Some(datatype) = datatypes.into_iter().next() {
            constraints.push(Constraint::Datatype(vec![NamedNode::new_unchecked(
                datatype,
            )]));
        }
        Ok(())
    }

    fn import_term(
        &mut self,
        value: &Value,
        path: &str,
    ) -> Result<Option<Term>, SchemaImportError> {
        match value {
            Value::String(value) => Ok(Some(Term::Literal(Literal::new_typed_literal(
                value,
                NamedNode::new_unchecked(&self.config.datatypes.string),
            )))),
            Value::Bool(value) => Ok(Some(Term::Literal(Literal::new_typed_literal(
                value.to_string(),
                NamedNode::new_unchecked(&self.config.datatypes.boolean),
            )))),
            Value::Number(number) => {
                self.record("numeric-lexical-form-normalized", path);
                Ok(Some(self.number_term(number)))
            }
            Value::Object(object) => self.import_term_object(object, path),
            Value::Null | Value::Array(_) => {
                self.record("value-term-kind-widened", path);
                Ok(None)
            }
        }
    }

    fn import_term_object(
        &mut self,
        object: &Map<String, Value>,
        path: &str,
    ) -> Result<Option<Term>, SchemaImportError> {
        if let Some(id) = object.get("@id") {
            if object.len() != 1 {
                return Err(SchemaImportError::new(format!(
                    "{path} @id value object cannot contain additional members"
                )));
            }
            if let Some(embedded) = id.as_object() {
                return self.import_triple_term(embedded, &format!("{path}/@id"));
            }
            let id = id
                .as_str()
                .ok_or_else(|| SchemaImportError::new(format!("{path}/@id must be a string")))?;
            if let Some(label) = id.strip_prefix("_:") {
                if label.is_empty() {
                    return Err(SchemaImportError::new(format!(
                        "{path}/@id blank-node label cannot be empty"
                    )));
                }
                return Ok(Some(Term::blank(label)));
            }
            let iri = self
                .config
                .namespaces
                .expand_iri(id)
                .map_err(|error| SchemaImportError::new(format!("{path}/@id: {error}")))?;
            return Ok(Some(Term::NamedNode(NamedNode::new_unchecked(iri))));
        }
        let Some(raw) = object.get("@value") else {
            self.record("value-term-kind-widened", path);
            return Ok(None);
        };
        if let Some(language) = object.get("@language") {
            if object.len() != 2 {
                return Err(SchemaImportError::new(format!(
                    "{path} language value object has unexpected members"
                )));
            }
            let lexical = raw.as_str().ok_or_else(|| {
                SchemaImportError::new(format!("{path}/@value must be a string with @language"))
            })?;
            let language = language.as_str().ok_or_else(|| {
                SchemaImportError::new(format!("{path}/@language must be a string"))
            })?;
            return Ok(Some(Term::Literal(
                Literal::new_language_tagged_literal_unchecked(lexical, language),
            )));
        }
        let datatype = object.get("@type").ok_or_else(|| {
            SchemaImportError::new(format!(
                "{path} typed value object must contain @type or @language"
            ))
        })?;
        if object.len() != 2 {
            return Err(SchemaImportError::new(format!(
                "{path} typed value object has unexpected members"
            )));
        }
        let datatype = datatype
            .as_str()
            .ok_or_else(|| SchemaImportError::new(format!("{path}/@type must be a string")))?;
        let datatype = self
            .config
            .namespaces
            .expand_iri(datatype)
            .map_err(|error| SchemaImportError::new(format!("{path}/@type: {error}")))?;
        let lexical = json_scalar_lexical(raw).ok_or_else(|| {
            SchemaImportError::new(format!("{path}/@value must be a JSON scalar"))
        })?;
        Ok(Some(Term::Literal(Literal::new_typed_literal(
            lexical,
            NamedNode::new_unchecked(datatype),
        ))))
    }

    /// The RDF 1.2 triple term a JSON-LD-star embedded node states: its `@id`
    /// is the subject, and its one other member the predicate and object.
    fn import_triple_term(
        &mut self,
        embedded: &Map<String, Value>,
        path: &str,
    ) -> Result<Option<Term>, SchemaImportError> {
        let [(first_key, first), (second_key, second)] = embedded
            .iter()
            .collect::<Vec<_>>()
            .try_into()
            .map_err(|_| {
                SchemaImportError::new(format!(
                    "{path} embedded triple must state @id and exactly one predicate"
                ))
            })?;
        let ((_, subject), (predicate, object)) = if first_key == "@id" {
            ((first_key, first), (second_key, second))
        } else if second_key == "@id" {
            ((second_key, second), (first_key, first))
        } else {
            return Err(SchemaImportError::new(format!(
                "{path} embedded triple must state its subject as @id"
            )));
        };
        let mut reference = Map::new();
        reference.insert("@id".to_owned(), subject.clone());
        let Some(subject) = self.import_term_object(&reference, &format!("{path}/@id"))? else {
            return Ok(None);
        };
        let predicate = self
            .config
            .namespaces
            .expand_iri(predicate)
            .map_err(|error| SchemaImportError::new(format!("{path} predicate: {error}")))?;
        let object_path = format!("{path}/{}", pointer_escape(predicate.as_str()));
        let Some(object) = self.import_term(object, &object_path)? else {
            return Ok(None);
        };
        Ok(Some(Term::Triple(Box::new(crate::term::Triple::new(
            subject,
            NamedNode::new_unchecked(predicate),
            object,
        )))))
    }

    fn numeric_term(&self, value: &Value, path: &str) -> Result<Term, SchemaImportError> {
        let number = value.as_number().ok_or_else(|| {
            SchemaImportError::new(format!("{path} must be a finite JSON number"))
        })?;
        Ok(self.number_term(number))
    }

    fn number_term(&self, number: &Number) -> Term {
        let datatype = self
            .config
            .datatypes
            .for_number(number.is_i64() || number.is_u64());
        Term::Literal(Literal::new_typed_literal(
            number_lexical(number),
            NamedNode::new_unchecked(datatype),
        ))
    }
}

#[derive(Clone, Copy)]
enum BoundKind {
    Minimum,
    Maximum,
    ExclusiveMinimum,
    ExclusiveMaximum,
}

impl BoundKind {
    fn constraint(self, term: Term) -> Constraint {
        match self {
            Self::Minimum => Constraint::MinInclusive(term),
            Self::Maximum => Constraint::MaxInclusive(term),
            Self::ExclusiveMinimum => Constraint::MinExclusive(term),
            Self::ExclusiveMaximum => Constraint::MaxExclusive(term),
        }
    }
}

fn validate_value_limits(
    value: &Value,
    depth: usize,
    nodes: &mut usize,
    path: &str,
) -> Result<(), SchemaImportError> {
    if depth > MAX_SCHEMA_DEPTH {
        return Err(SchemaImportError::new(format!(
            "{path} exceeds the schema nesting limit of {MAX_SCHEMA_DEPTH}"
        )));
    }
    *nodes = nodes
        .checked_add(1)
        .ok_or_else(|| SchemaImportError::new("schema node count overflow"))?;
    if *nodes > MAX_SCHEMA_NODES {
        return Err(SchemaImportError::new(format!(
            "schema exceeds the {MAX_SCHEMA_NODES}-node limit"
        )));
    }
    match value {
        Value::String(value) if value.len() > MAX_STRING_BYTES => Err(SchemaImportError::new(
            format!("{path} string exceeds the {MAX_STRING_BYTES}-byte limit"),
        )),
        Value::Array(values) => {
            for (index, value) in values.iter().enumerate() {
                validate_value_limits(value, depth + 1, nodes, &format!("{path}/{index}"))?;
            }
            Ok(())
        }
        Value::Object(values) => {
            for (key, value) in values {
                if key.len() > MAX_STRING_BYTES {
                    return Err(SchemaImportError::new(format!(
                        "{path} member name exceeds the {MAX_STRING_BYTES}-byte limit"
                    )));
                }
                validate_value_limits(
                    value,
                    depth + 1,
                    nodes,
                    &format!("{path}/{}", pointer_escape(key)),
                )?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn validate_references(
    value: &Value,
    definitions: &Map<String, Value>,
    path: &str,
    depth: usize,
) -> Result<(), SchemaImportError> {
    if depth > MAX_SCHEMA_DEPTH {
        return Err(SchemaImportError::new(format!(
            "{path} exceeds the reference scan depth limit"
        )));
    }
    let Value::Object(object) = value else {
        return if value.is_boolean() {
            Ok(())
        } else {
            Err(SchemaImportError::new(format!(
                "{path} must be an object or boolean schema"
            )))
        };
    };
    for (keyword, value) in object {
        validate_json_keyword_value(
            keyword,
            value,
            &format!("{path}/{}", pointer_escape(keyword)),
        )?;
    }
    for keyword in ["$dynamicRef", "$recursiveRef"] {
        if object.contains_key(keyword) {
            return Err(SchemaImportError::new(format!(
                "{path}/{keyword} is not a closed direct reference"
            )));
        }
    }
    if path != "#" && object.contains_key("$id") {
        return Err(SchemaImportError::new(format!(
            "{path}/$id cannot rebase a closed schema import"
        )));
    }
    if path != "#" && object.contains_key("$schema") {
        return Err(SchemaImportError::new(format!(
            "{path}/$schema cannot change the fixed draft-2020-12 dialect"
        )));
    }
    if let Some(reference) = object.get("$ref") {
        let reference = reference
            .as_str()
            .ok_or_else(|| SchemaImportError::new(format!("{path}/$ref must be a string")))?;
        let key = reference_key(reference).ok_or_else(|| {
            SchemaImportError::new(format!(
                "{path}/$ref is external or not a direct #/$defs reference: {reference:?}"
            ))
        })?;
        if !definitions.contains_key(&key) {
            return Err(SchemaImportError::new(format!(
                "{path}/$ref targets missing definition {key:?}"
            )));
        }
    }
    for keyword in [
        "$defs",
        "properties",
        "patternProperties",
        "dependentSchemas",
    ] {
        if let Some(children) = object.get(keyword) {
            let children = children.as_object().ok_or_else(|| {
                SchemaImportError::new(format!("{path}/{keyword} must be an object"))
            })?;
            for (key, child) in children {
                validate_references(
                    child,
                    definitions,
                    &format!("{path}/{keyword}/{}", pointer_escape(key)),
                    depth + 1,
                )?;
            }
        }
    }
    for keyword in ["allOf", "anyOf", "oneOf", "prefixItems"] {
        if let Some(children) = object.get(keyword) {
            let children = children.as_array().ok_or_else(|| {
                SchemaImportError::new(format!("{path}/{keyword} must be an array"))
            })?;
            for (index, child) in children.iter().enumerate() {
                validate_references(
                    child,
                    definitions,
                    &format!("{path}/{keyword}/{index}"),
                    depth + 1,
                )?;
            }
        }
    }
    for keyword in [
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
    ] {
        if let Some(child) = object.get(keyword) {
            validate_references(child, definitions, &format!("{path}/{keyword}"), depth + 1)?;
        }
    }
    Ok(())
}

fn is_generated_envelope(
    root: &Map<String, Value>,
    definitions: &Map<String, Value>,
    namespaces: &Namespaces,
) -> bool {
    let Some(node) = definitions.get("Node").and_then(Value::as_object) else {
        return false;
    };
    let Some(annotation) = definitions.get("Annotation") else {
        return false;
    };
    let node_ref = serde_json::json!({ "$ref": "#/$defs/Node" });
    let graph_envelope = serde_json::json!({
        "type": "object",
        "required": ["@graph"],
        "properties": {
            "@context": true,
            "@graph": { "type": "array", "items": node_ref }
        }
    });
    let expected_properties = serde_json::json!({
        "@context": true,
        "@graph": { "type": "array", "items": node_ref }
    });
    let expected_annotation = serde_json::json!({
        "type": "object",
        "title": "RDF-1.2 statement metadata (reifier annotation)",
        "description": "Free-form metadata about an asserted triple (e.g. meta:accordingTo, meta:confidence, meta:assertedAt). Permissive.",
        "additionalProperties": {
            "anyOf": [
                { "type": "string" },
                { "type": "number" },
                { "type": "boolean" },
                {
                    "type": "object",
                    "properties": { "@id": { "type": "string" } },
                    "required": ["@id"]
                },
                {
                    "type": "object",
                    "properties": {
                        "@value": {},
                        "@type": { "type": "string" }
                    },
                    "required": ["@value"]
                }
            ]
        }
    });
    let node_properties = node.get("properties").and_then(Value::as_object);
    let has_node_contract = node.get("type").and_then(Value::as_str) == Some("object")
        && node.get("title").and_then(Value::as_str)
            == Some("A single discriminated PURRDF instance node")
        && node.get("allOf").is_some_and(Value::is_array)
        && node_properties.is_some_and(|properties| {
            properties.len() == 3
                && ["@id", "@type", "@annotation"]
                    .iter()
                    .all(|key| properties.contains_key(*key))
        });
    root.get("$schema").and_then(Value::as_str) == Some(JSON_SCHEMA_DIALECT)
        && root.get("$id").and_then(Value::as_str)
            == Some(format!("{}schema/instance.schema.json", namespaces.primary_ns()).as_str())
        && root.get("title").and_then(Value::as_str)
            == Some("PURRDF instance schema (SHACL-derived, closed-world)")
        && root.get("type").and_then(Value::as_str) == Some("object")
        && root.get("anyOf") == Some(&serde_json::json!([graph_envelope, node_ref]))
        && root.get("properties") == Some(&expected_properties)
        && annotation == &expected_annotation
        && has_node_contract
}

fn is_object_schema(object: &Map<String, Value>) -> bool {
    schema_type_contains(object.get("type"), "object")
        || object.contains_key("properties")
        || object.contains_key("required")
        || object.contains_key("additionalProperties")
        || ["allOf", "anyOf", "oneOf"].iter().any(|keyword| {
            object
                .get(*keyword)
                .and_then(Value::as_array)
                .is_some_and(|branches| {
                    branches
                        .iter()
                        .any(|branch| branch.as_object().is_some_and(is_object_schema))
                })
        })
}

fn is_array_schema(object: &Map<String, Value>) -> bool {
    schema_type_contains(object.get("type"), "array") || object.contains_key("items")
}

fn is_simple_scalar_carrier(object: &Map<String, Value>) -> bool {
    object.contains_key("type")
        && object
            .keys()
            .all(|keyword| matches!(keyword.as_str(), "type" | "format"))
}

fn schema_type_contains(value: Option<&Value>, expected: &str) -> bool {
    match value {
        Some(Value::String(value)) => value == expected,
        Some(Value::Array(values)) => values.iter().any(|value| value.as_str() == Some(expected)),
        _ => false,
    }
}

fn optional_object<'a>(
    object: &'a Map<String, Value>,
    keyword: &str,
    path: &str,
) -> Result<&'a Map<String, Value>, SchemaImportError> {
    static EMPTY: OnceLock<Map<String, Value>> = OnceLock::new();
    object
        .get(keyword)
        .map(|value| {
            value.as_object().ok_or_else(|| {
                SchemaImportError::new(format!("{path}/{keyword} must be an object"))
            })
        })
        .transpose()
        .map(|value| value.unwrap_or_else(|| EMPTY.get_or_init(Map::new)))
}

fn required_names(
    object: &Map<String, Value>,
    _properties: &Map<String, Value>,
    path: &str,
) -> Result<BTreeSet<String>, SchemaImportError> {
    let Some(required) = object.get("required") else {
        return Ok(BTreeSet::new());
    };
    let required = required
        .as_array()
        .ok_or_else(|| SchemaImportError::new(format!("{path}/required must be an array")))?;
    let mut names = BTreeSet::new();
    for (index, name) in required.iter().enumerate() {
        let name = name.as_str().ok_or_else(|| {
            SchemaImportError::new(format!("{path}/required/{index} must be a string"))
        })?;
        if !names.insert(name.to_owned()) {
            return Err(SchemaImportError::new(format!(
                "{path}/required contains duplicate property {name:?}"
            )));
        }
    }
    Ok(names)
}

fn validate_object_type(value: Option<&Value>, path: &str) -> Result<(), SchemaImportError> {
    let Some(value) = value else {
        return Ok(());
    };
    let kinds = schema_types(value, &format!("{path}/type"))?;
    if !kinds.iter().any(|kind| kind == "object") {
        return Err(SchemaImportError::new(format!(
            "{path}/type must include object for an object definition"
        )));
    }
    Ok(())
}

fn schema_types(value: &Value, path: &str) -> Result<Vec<String>, SchemaImportError> {
    let mut kinds = match value {
        Value::String(kind) => vec![kind.clone()],
        Value::Array(values) => values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                value.as_str().map(str::to_owned).ok_or_else(|| {
                    SchemaImportError::new(format!("{path}/{index} must be a string"))
                })
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => {
            return Err(SchemaImportError::new(format!(
                "{path} must be a string or array of strings"
            )));
        }
    };
    kinds.sort();
    if kinds.is_empty() || kinds.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(SchemaImportError::new(format!(
            "{path} must contain unique schema primitive names"
        )));
    }
    for kind in &kinds {
        if !matches!(
            kind.as_str(),
            "null" | "boolean" | "object" | "array" | "number" | "string" | "integer"
        ) {
            return Err(SchemaImportError::new(format!(
                "{path} contains unknown JSON Schema type {kind:?}"
            )));
        }
    }
    Ok(kinds)
}

fn nonnegative_integer(value: &Value, path: &str) -> Result<u64, SchemaImportError> {
    value
        .as_u64()
        .ok_or_else(|| SchemaImportError::new(format!("{path} must be a non-negative integer")))
}

fn validate_nonnegative_integer(value: &Value, path: &str) -> Result<(), SchemaImportError> {
    nonnegative_integer(value, path).map(|_| ())
}

fn reference_key(reference: &str) -> Option<String> {
    let encoded = reference.strip_prefix("#/$defs/")?;
    if encoded.contains('/') {
        return None;
    }
    pointer_unescape(encoded)
}

fn pointer_unescape(value: &str) -> Option<String> {
    let mut output = String::with_capacity(value.len());
    let mut characters = value.chars();
    while let Some(character) = characters.next() {
        if character != '~' {
            output.push(character);
            continue;
        }
        match characters.next()? {
            '0' => output.push('~'),
            '1' => output.push('/'),
            _ => return None,
        }
    }
    Some(output)
}

fn is_typed_literal_schema(value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    let Some(properties) = object.get("properties").and_then(Value::as_object) else {
        return false;
    };
    object.len() == 3
        && object.get("type").and_then(Value::as_str) == Some("object")
        && properties.len() == 2
        && properties
            .get("@value")
            .and_then(Value::as_object)
            .is_some_and(Map::is_empty)
        && properties
            .get("@type")
            .is_some_and(|schema| is_exact_type_schema(schema, "string"))
        && is_exact_required(object.get("required"), &["@value"])
}

/// The value-schema alternative one constraint's `not` names, read back.
enum Rejection {
    /// `sh:singleLine true`: a string or `@value` holding a line break.
    LineBreak,
    /// `sh:pattern` (ECMA-262 source): a string or `@value` it does not match.
    Pattern(String),
    /// `sh:minLength n`: an `@value` shorter than `n`.
    MinLength(u64),
    /// `sh:maxLength n`: an `@value` longer than `n`.
    MaxLength(u64),
    /// A blank node, which every lexical-form constraint rejects.
    BlankNode,
    /// A triple term, which every lexical-form constraint and numeric bound
    /// rejects.
    TripleTerm,
    /// A value no numeric bound can compare.
    NonNumeric,
    /// A constant whose lexical form a lexical-form constraint judged at
    /// compile time — `rdf:nil`'s list, `true`, `false`, an `sh:in` member —
    /// or a range of bare integers a length bound or numeric bound excludes.
    Implied,
    /// A typed literal of an integer-family or decimal datatype a numeric
    /// bound rejects.
    TypedBound,
}

/// The ECMA-262 source a string containing a line break matches.
const LINE_BREAK_PATTERN: &str = "[\\n\\r\\u000B\\u000C]";

fn classify_rejection(value: &Value) -> Option<Rejection> {
    let object = value.as_object()?;
    if object.len() == 1 && object.contains_key("const") {
        return Some(Rejection::Implied);
    }
    if list_form(value) == Some(ListForm::Nil) {
        return Some(Rejection::Implied);
    }
    if list_form(value) == Some(ListForm::NonEmpty) {
        return Some(Rejection::BlankNode);
    }
    if list_form(value) == Some(ListForm::Any) {
        return Some(Rejection::NonNumeric);
    }
    let kind = object.get("type").and_then(Value::as_str)?;
    if kind == "integer"
        && object
            .keys()
            .all(|key| matches!(key.as_str(), "type" | "minimum" | "maximum" | "not"))
    {
        return Some(Rejection::Implied);
    }
    if kind == "string" {
        if object.len() == 2
            && object.get("pattern").and_then(Value::as_str) == Some(LINE_BREAK_PATTERN)
        {
            return Some(Rejection::LineBreak);
        }
        if object.len() == 2
            && let Some(pattern) = object.get("not").and_then(negated_pattern)
        {
            return Some(Rejection::Pattern(pattern));
        }
        return (object.len() == 1).then_some(Rejection::NonNumeric);
    }
    if kind == "boolean" {
        return (object.len() == 1).then_some(Rejection::NonNumeric);
    }
    if kind != "object" {
        return None;
    }
    if is_node_ref_schema(value) && !object.contains_key("$comment") {
        return Some(Rejection::NonNumeric);
    }
    if is_blank_ref_schema(value) {
        return Some(Rejection::BlankNode);
    }
    if is_triple_ref_schema(value) {
        return Some(Rejection::TripleTerm);
    }
    if is_language_carrier(object) {
        return Some(Rejection::NonNumeric);
    }
    let properties = object.get("properties").and_then(Value::as_object)?;
    if object.len() == 3 && is_exact_required(object.get("required"), &["@value", "@type"]) {
        let at_type = properties.get("@type")?;
        if properties.len() == 2
            && properties.get("@value") == Some(&Value::Object(Map::new()))
            && at_type.get("not").and_then(|not| not.get("enum")).is_some()
        {
            return Some(Rejection::NonNumeric);
        }
        if properties.len() == 2
            && at_type.get("const").is_some_and(Value::is_string)
            && properties
                .get("@value")
                .and_then(|at_value| at_value.get("not"))
                .is_some()
        {
            return Some(Rejection::TypedBound);
        }
        return None;
    }
    // An IRI whose `@id` (the IRI itself) breaks a lexical-form constraint.
    if object.len() == 3 && is_exact_required(object.get("required"), &["@id"]) {
        let at_id = properties.get("@id").and_then(Value::as_object)?;
        if properties.len() != 1
            || at_id.len() != 3
            || at_id.get("type").and_then(Value::as_str) != Some("string")
            || at_id.get("pattern").and_then(Value::as_str) != Some("^(?:[^_]|_(?:[^:]|$))")
        {
            return None;
        }
        if let Some(pattern) = at_id.get("not").and_then(negated_pattern) {
            return Some(Rejection::Pattern(pattern));
        }
        if let Some(n) = at_id.get("maxLength").and_then(Value::as_u64) {
            return Some(Rejection::MinLength(n + 1));
        }
        if let Some(n) = at_id.get("minLength").and_then(Value::as_u64)
            && n > 0
        {
            return Some(Rejection::MaxLength(n - 1));
        }
        return None;
    }
    // A literal object whose `@value` string breaks a lexical-form constraint.
    if object.len() != 3
        || properties.len() != 1
        || !is_exact_required(object.get("required"), &["@value"])
    {
        return None;
    }
    let at_value = properties.get("@value").and_then(Value::as_object)?;
    if at_value.get("type").and_then(Value::as_str) != Some("string") || at_value.len() != 2 {
        return None;
    }
    if at_value.get("pattern").and_then(Value::as_str) == Some(LINE_BREAK_PATTERN) {
        return Some(Rejection::LineBreak);
    }
    if let Some(pattern) = at_value.get("not").and_then(negated_pattern) {
        return Some(Rejection::Pattern(pattern));
    }
    if let Some(n) = at_value.get("maxLength").and_then(Value::as_u64) {
        return Some(Rejection::MinLength(n + 1));
    }
    if let Some(n) = at_value.get("minLength").and_then(Value::as_u64)
        && n > 0
    {
        return Some(Rejection::MaxLength(n - 1));
    }
    None
}

/// `P` of `{"pattern": P}`.
fn negated_pattern(value: &Value) -> Option<String> {
    let object = value.as_object()?;
    (object.len() == 1)
        .then(|| {
            object
                .get("pattern")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .flatten()
}

/// The `@type` constant of the emitter's typed-literal object for one datatype:
/// `{"type": "object", "properties": {"@value": {"type": "string", …},
/// "@type": {"const": d}}, "required": ["@value", "@type"]}`.
fn typed_datatype_constant(value: &Value) -> Option<&str> {
    let object = value.as_object()?;
    let properties = object.get("properties").and_then(Value::as_object)?;
    if object.len() != 3
        || object.get("type").and_then(Value::as_str) != Some("object")
        || properties.len() != 2
        || !is_exact_required(object.get("required"), &["@value", "@type"])
    {
        return None;
    }
    let at_value = properties.get("@value").and_then(Value::as_object)?;
    if at_value.get("type").and_then(Value::as_str) != Some("string")
        || !at_value
            .keys()
            .all(|key| matches!(key.as_str(), "type" | "format" | "pattern" | "allOf"))
    {
        return None;
    }
    let at_type = properties.get("@type").and_then(Value::as_object)?;
    (at_type.len() == 1)
        .then(|| at_type.get("const").and_then(Value::as_str))
        .flatten()
}

/// Whether `value` is the language-tagged object with any tag and no base
/// direction — the projection of `rdf:langString`.
fn is_lang_string_schema(value: &Value) -> bool {
    value.as_object().is_some_and(|object| {
        let Some(properties) = object.get("properties").and_then(Value::as_object) else {
            return false;
        };
        let mut rest = object.clone();
        let mut rest_properties = properties.clone();
        rest_properties.remove("@direction").as_ref() == Some(&Value::Bool(false)) && {
            rest.insert("properties".to_owned(), Value::Object(rest_properties));
            is_any_language_object(&Value::Object(rest))
        }
    })
}

/// Whether `value` is the language-tagged object with a base direction — the
/// projection of `rdf:dirLangString`.
fn is_dir_lang_string_schema(value: &Value) -> bool {
    value.as_object().is_some_and(|object| {
        object.len() == 3
            && object.get("type").and_then(Value::as_str) == Some("object")
            && object
                .get("properties")
                .and_then(Value::as_object)
                .is_some_and(|properties| {
                    properties.len() == 3
                        && properties
                            .get("@value")
                            .is_some_and(|schema| is_exact_type_schema(schema, "string"))
                        && properties
                            .get("@language")
                            .is_some_and(|schema| is_exact_type_schema(schema, "string"))
                        && properties
                            .get("@direction")
                            .and_then(|schema| schema.get("enum"))
                            == Some(&Value::Array(vec![
                                Value::String("ltr".to_owned()),
                                Value::String("rtl".to_owned()),
                            ]))
                })
            && is_exact_required(
                object.get("required"),
                &["@value", "@language", "@direction"],
            )
    })
}

/// Whether `value` is the language-tagged object with any tag, direction or
/// none — the value a numeric bound cannot compare.
fn is_any_language_object(value: &Value) -> bool {
    value.as_object().is_some_and(|object| {
        object.len() == 3
            && object.get("type").and_then(Value::as_str) == Some("object")
            && object
                .get("properties")
                .and_then(Value::as_object)
                .is_some_and(|properties| {
                    properties.len() == 2
                        && properties
                            .get("@value")
                            .is_some_and(|schema| is_exact_type_schema(schema, "string"))
                        && properties
                            .get("@language")
                            .is_some_and(|schema| is_exact_type_schema(schema, "string"))
                })
            && is_exact_required(object.get("required"), &["@value", "@language"])
    })
}

/// Whether `object` is a language-tagged literal object (any tag, or a
/// `sh:languageIn` pattern).
fn is_language_carrier(object: &Map<String, Value>) -> bool {
    is_language_literal_schema(object) || is_any_language_object(&Value::Object(object.clone()))
}

/// Whether `value` is the projection of a blank node: a node reference whose
/// `@id` is a blank-node label.
fn is_blank_ref_schema(value: &Value) -> bool {
    is_id_pattern_schema(value, "^_:")
}

/// Whether `value` is the projection of an RDF 1.2 triple term: the JSON-LD-star
/// embedded node, whose `@id` is an object.
fn is_triple_ref_schema(value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    object.len() == 3
        && object.get("type").and_then(Value::as_str) == Some("object")
        && is_exact_required(object.get("required"), &["@id"])
        && object
            .get("properties")
            .and_then(Value::as_object)
            .is_some_and(|properties| {
                properties.len() == 1
                    && properties
                        .get("@id")
                        .is_some_and(|id| is_exact_type_schema(id, "object"))
            })
}

/// Whether `value` is the projection of an IRI: a node reference whose `@id` is
/// not a blank-node label.
fn is_iri_ref_schema(value: &Value) -> bool {
    is_id_pattern_schema(value, "^(?:[^_]|_(?:[^:]|$))")
}

fn is_id_pattern_schema(value: &Value, pattern: &str) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    object.len() == 3
        && object.get("type").and_then(Value::as_str) == Some("object")
        && is_exact_required(object.get("required"), &["@id"])
        && object
            .get("properties")
            .and_then(Value::as_object)
            .and_then(|properties| {
                (properties.len() == 1)
                    .then(|| properties.get("@id"))
                    .flatten()
            })
            .and_then(Value::as_object)
            .is_some_and(|id| {
                id.len() == 2
                    && id.get("type").and_then(Value::as_str) == Some("string")
                    && id.get("pattern").and_then(Value::as_str) == Some(pattern)
            })
}

/// The `sh:nodeKind` list whose projection is exactly the alternatives
/// `branches`: any union of the literal forms (all four together), IRI
/// references, blank-node references and node references.
fn node_kind_union(branches: &[Value]) -> Option<Vec<NodeKindValue>> {
    let branches: Vec<&Value> = if let [only] = branches
        && let Some(inner) = only.get("anyOf").and_then(Value::as_array)
        && only.as_object().is_some_and(|object| object.len() == 1)
    {
        inner.iter().collect()
    } else {
        branches.iter().collect()
    };
    let (mut string, mut number, mut boolean, mut typed) = (false, false, false, false);
    let (mut iri, mut blank, mut node, mut triple) = (false, false, false, false);
    let (mut nil_list, mut nonempty_list, mut any_list) = (false, false, false);
    for branch in &branches {
        if is_exact_type_schema(branch, "string") {
            string = true;
        } else if is_exact_type_schema(branch, "number") {
            number = true;
        } else if is_exact_type_schema(branch, "boolean") {
            boolean = true;
        } else if is_typed_literal_schema(branch) {
            typed = true;
        } else if is_iri_ref_schema(branch) {
            iri = true;
        } else if is_blank_ref_schema(branch) {
            blank = true;
        } else if is_triple_ref_schema(branch) {
            triple = true;
        } else if is_node_ref_schema(branch) && branch.get("$comment").is_none() {
            node = true;
        } else {
            // `rdf:nil` (an IRI) and a non-empty list (whose head is a blank
            // node) project as lists; the emitter pairs each node kind with
            // exactly the lists it admits.
            match list_form(branch)? {
                ListForm::Nil => nil_list = true,
                ListForm::NonEmpty => nonempty_list = true,
                ListForm::Any => any_list = true,
            }
        }
    }
    if nil_list != iri || nonempty_list != blank || any_list != node {
        return None;
    }
    let literal = string && number && boolean && typed;
    if (string || number || boolean || typed) && !literal {
        return None;
    }
    // A lone node reference is `sh:nodeKind sh:BlankNodeOrIRI` or a `sh:class`
    // without a definition; the class reading is the caller's.
    if !literal && !iri && !blank && !triple {
        return None;
    }
    let mut kinds = match (literal, iri || node, blank || node) {
        (true, true, true) => vec![NodeKindValue::BlankNodeOrIri, NodeKindValue::Literal],
        (true, true, false) => vec![NodeKindValue::IriOrLiteral],
        (true, false, true) => vec![NodeKindValue::BlankNodeOrLiteral],
        (true, false, false) => vec![NodeKindValue::Literal],
        (false, true, true) => vec![NodeKindValue::BlankNodeOrIri],
        (false, true, false) => vec![NodeKindValue::Iri],
        (false, false, true) => vec![NodeKindValue::BlankNode],
        (false, false, false) => Vec::new(),
    };
    if triple {
        kinds.push(NodeKindValue::TripleTerm);
    }
    Some(kinds)
}

/// `{"anyOf": branches}`.
fn json_any_of(branches: &[Value]) -> Value {
    let mut object = Map::new();
    object.insert("anyOf".to_owned(), Value::Array(branches.to_vec()));
    Value::Object(object)
}

/// The `sh:hasValue` constants an array schema states: `contains: {const: v}`,
/// or one such `contains` per term under `allOf`. `None` when the array states
/// none that way.
fn contained_constants(object: &Map<String, Value>) -> Option<Vec<Value>> {
    let constant = |contains: &Value| {
        contains
            .as_object()
            .filter(|object| object.len() == 1)
            .and_then(|object| object.get("const"))
            .cloned()
    };
    if let Some(contains) = object.get("contains") {
        return constant(contains).map(|value| vec![value]);
    }
    let branches = object.get("allOf")?.as_array()?;
    branches
        .iter()
        .map(|branch| {
            branch
                .as_object()
                .filter(|object| object.len() == 1)
                .and_then(|object| object.get("contains"))
                .and_then(constant)
        })
        .collect()
}

/// The single-value schema the emitter pairs with an array whose items are
/// `items` and whose `sh:hasValue` terms are `has_values`: a lone value is the
/// term (`const`), or each of several (`allOf` of `const`s).
fn with_has_values(items: &Value, has_values: &[Value]) -> Value {
    let mut single = items.clone();
    let Some(object) = single.as_object_mut() else {
        return single;
    };
    match has_values {
        [] => {}
        [only] => {
            object.insert("const".to_owned(), only.clone());
        }
        several => {
            let mut all = object
                .get("allOf")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            all.extend(several.iter().map(|value| {
                let mut constant = Map::new();
                constant.insert("const".to_owned(), value.clone());
                Value::Object(constant)
            }));
            object.insert("allOf".to_owned(), Value::Array(all));
        }
    }
    single
}

/// The members schema `M` of the projection of a list,
/// `{"type": "object", "properties": {"@list": M}, "required": ["@list"]}`,
/// when `M` is an array schema.
fn list_members(value: &Value) -> Option<&Map<String, Value>> {
    let object = value.as_object()?;
    if object.len() != 3
        || object.get("type").and_then(Value::as_str) != Some("object")
        || !is_exact_required(object.get("required"), &["@list"])
    {
        return None;
    }
    let properties = object.get("properties")?.as_object()?;
    let members = (properties.len() == 1)
        .then(|| properties.get("@list"))
        .flatten()?
        .as_object()?;
    (members.get("type").and_then(Value::as_str) == Some("array")).then_some(members)
}

/// Which list the projection of a list names by its length alone: any list,
/// `rdf:nil`'s empty list, or a non-empty one.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ListForm {
    Any,
    Nil,
    NonEmpty,
}

fn list_form(value: &Value) -> Option<ListForm> {
    let members = list_members(value)?;
    match (
        members.len(),
        members.get("maxItems").and_then(Value::as_u64),
        members.get("minItems").and_then(Value::as_u64),
    ) {
        (1, None, None) => Some(ListForm::Any),
        (2, Some(0), None) => Some(ListForm::Nil),
        (2, None, Some(1)) => Some(ListForm::NonEmpty),
        _ => None,
    }
}

/// A node reference with no annotation.
fn is_plain_node_ref(value: &Value) -> bool {
    is_node_ref_schema(value) && value.get("$comment").is_none()
}

fn is_node_ref_schema(value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    let Some(properties) = object.get("properties").and_then(Value::as_object) else {
        return false;
    };
    (object.len() == 3 || (object.len() == 4 && object.contains_key("$comment")))
        && object.keys().all(|key| {
            matches!(
                key.as_str(),
                "type" | "properties" | "required" | "$comment"
            )
        })
        && object.get("type").and_then(Value::as_str) == Some("object")
        && properties.len() == 1
        && properties
            .get("@id")
            .is_some_and(|schema| is_exact_type_schema(schema, "string"))
        && is_exact_required(object.get("required"), &["@id"])
        && object.get("$comment").is_none_or(Value::is_string)
}

fn is_exact_type_schema(value: &Value, expected: &str) -> bool {
    value.as_object().is_some_and(|object| {
        object.len() == 1 && object.get("type").and_then(Value::as_str) == Some(expected)
    })
}

fn is_exact_required(value: Option<&Value>, expected: &[&str]) -> bool {
    value.and_then(Value::as_array).is_some_and(|values| {
        values.len() == expected.len()
            && values
                .iter()
                .zip(expected)
                .all(|(value, expected)| value.as_str() == Some(*expected))
    })
}

fn node_ref_class(
    object: &Map<String, Value>,
    namespaces: &Namespaces,
) -> Result<Option<NamedNode>, SchemaImportError> {
    let Some(comment) = object.get("$comment").and_then(Value::as_str) else {
        return Ok(None);
    };
    let identity = comment
        .strip_prefix("external class ")
        .or_else(|| comment.strip_suffix(" has no NodeShape; node reference only"));
    let Some(identity) = identity else {
        return Ok(None);
    };
    let iri = namespaces.expand_iri(identity).map_err(|error| {
        SchemaImportError::new(format!("node-reference class {identity:?}: {error}"))
    })?;
    Ok(Some(NamedNode::new_unchecked(iri)))
}

fn type_discriminator_class(
    object: &Map<String, Value>,
    namespaces: &Namespaces,
) -> Result<Option<NamedNode>, SchemaImportError> {
    let required = object.get("required").and_then(Value::as_array);
    if !required.is_some_and(|required| required.iter().any(|value| value == "@type")) {
        return Ok(None);
    }
    let Some(branches) = object
        .get("properties")
        .and_then(Value::as_object)
        .and_then(|properties| properties.get("@type"))
        .and_then(|schema| schema.get("anyOf"))
        .and_then(Value::as_array)
    else {
        return Ok(None);
    };
    let direct = branches.iter().find_map(|branch| branch.get("const"));
    let contained = branches
        .iter()
        .find_map(|branch| branch.get("contains"))
        .and_then(|contains| contains.get("const"));
    let (Some(direct), Some(contained)) = (direct, contained) else {
        return Ok(None);
    };
    if direct != contained {
        return Err(SchemaImportError::new(
            "@type discriminator direct and array-member constants disagree",
        ));
    }
    let identity = direct
        .as_str()
        .ok_or_else(|| SchemaImportError::new("@type discriminator constant must be a string"))?;
    let iri = namespaces.expand_iri(identity).map_err(|error| {
        SchemaImportError::new(format!("@type discriminator {identity:?}: {error}"))
    })?;
    Ok(Some(NamedNode::new_unchecked(iri)))
}

fn is_language_literal_schema(object: &Map<String, Value>) -> bool {
    let Some(properties) = object.get("properties").and_then(Value::as_object) else {
        return false;
    };
    object.len() == 3
        && object.get("type").and_then(Value::as_str) == Some("object")
        && properties.len() == 2
        && properties
            .get("@value")
            .is_some_and(|schema| is_exact_type_schema(schema, "string"))
        && properties
            .get("@language")
            .and_then(Value::as_object)
            .is_some_and(|schema| {
                schema.len() == 2
                    && schema.get("type").and_then(Value::as_str) == Some("string")
                    && schema.get("pattern").is_some_and(Value::is_string)
            })
        && is_exact_required(object.get("required"), &["@value", "@language"])
}

fn language_tags(object: &Map<String, Value>) -> Option<Vec<String>> {
    let pattern = object
        .get("properties")?
        .get("@language")?
        .get("pattern")?
        .as_str()?;
    let mut tags = if let Some(inner) = pattern
        .strip_prefix("^(?:")
        .and_then(|rest| rest.strip_suffix(")(?:-.*)?$"))
    {
        inner
            .split('|')
            .map(case_insensitive_tag)
            .collect::<Option<Vec<_>>>()?
    } else {
        let inner = pattern.strip_prefix("^(")?.strip_suffix(")(-.*)?$")?;
        inner
            .split('|')
            .map(unescape_regex_literal)
            .collect::<Option<Vec<_>>>()?
    };
    tags.sort();
    tags.dedup();
    (!tags.is_empty()).then_some(tags)
}

/// A `sh:languageIn` tag from the emitter's case-insensitive alternative: each
/// letter a `[xX]` class, a digit or `-` itself, anything else `\u{HEX}`.
fn case_insensitive_tag(value: &str) -> Option<String> {
    let mut output = String::with_capacity(value.len() / 2);
    let mut rest = value;
    while let Some(first) = rest.chars().next() {
        if first == '[' {
            let mut class = rest[1..].chars();
            let lower = class.next()?;
            let upper = class.next()?;
            if class.next()? != ']'
                || !lower.is_ascii_lowercase()
                || upper != lower.to_ascii_uppercase()
            {
                return None;
            }
            output.push(lower);
            rest = &rest[4..];
        } else if first.is_ascii_digit() || first == '-' {
            output.push(first);
            rest = &rest[1..];
        } else {
            let escape = rest.strip_prefix("\\u{")?;
            let end = escape.find('}')?;
            output.push(char::from_u32(
                u32::from_str_radix(&escape[..end], 16).ok()?,
            )?);
            rest = &escape[end + 1..];
        }
    }
    (!output.is_empty()).then_some(output)
}

fn unescape_regex_literal(value: &str) -> Option<String> {
    let mut output = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(character) = chars.next() {
        if character == '\\' {
            output.push(chars.next()?);
        } else {
            output.push(character);
        }
    }
    Some(output)
}

/// A JSON number's lexical form as an RDF numeric literal.
///
/// A non-integral number's own `Display` is `serde_json`'s shortest form, which switches
/// to an exponent (`2.2e-230`) outside `[1e-5, 1e16)`: not in the lexical space of
/// `xsd:decimal`, the usual carrier. The binary64's `Display` writes the same shortest
/// digits positionally, a lexical form every numeric datatype accepts.
fn number_lexical(number: &Number) -> String {
    match number.as_f64() {
        Some(value) if !(number.is_i64() || number.is_u64()) => value.to_string(),
        _ => number.to_string(),
    }
}

fn json_scalar_lexical(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Bool(value) => Some(value.to_string()),
        Value::Number(value) => Some(number_lexical(value)),
        Value::Null | Value::Array(_) | Value::Object(_) => None,
    }
}

fn is_annotation_keyword(keyword: &str) -> bool {
    matches!(
        keyword,
        "title"
            | "description"
            | "$comment"
            | "default"
            | "examples"
            | "deprecated"
            | "readOnly"
            | "writeOnly"
    )
}

fn is_known_assertion_keyword(keyword: &str) -> bool {
    matches!(
        keyword,
        "type"
            | "enum"
            | "const"
            | "multipleOf"
            | "maximum"
            | "exclusiveMaximum"
            | "minimum"
            | "exclusiveMinimum"
            | "maxLength"
            | "minLength"
            | "pattern"
            | "maxItems"
            | "minItems"
            | "uniqueItems"
            | "maxContains"
            | "minContains"
            | "contains"
            | "prefixItems"
            | "items"
            | "additionalItems"
            | "unevaluatedItems"
            | "maxProperties"
            | "minProperties"
            | "properties"
            | "patternProperties"
            | "additionalProperties"
            | "propertyNames"
            | "required"
            | "dependentRequired"
            | "dependentSchemas"
            | "allOf"
            | "anyOf"
            | "oneOf"
            | "not"
            | "if"
            | "then"
            | "else"
            | "format"
            | "contentEncoding"
            | "contentMediaType"
            | "contentSchema"
            | "unevaluatedProperties"
    )
}

fn root_loss_code(keyword: &str) -> &'static str {
    match keyword {
        "contains" | "minContains" | "maxContains" => "array-contains-validation-dropped",
        "prefixItems" | "additionalItems" => "tuple-validation-widened",
        "uniqueItems" => "unique-items-validation-dropped",
        "dependentRequired" | "dependentSchemas" => "dependency-validation-dropped",
        "if" | "then" | "else" => "conditional-validation-dropped",
        "unevaluatedProperties" | "unevaluatedItems" => "unevaluated-validation-dropped",
        "patternProperties" | "propertyNames" => "object-key-validation-dropped",
        "minProperties" | "maxProperties" => "property-count-validation-dropped",
        "contentEncoding" | "contentMediaType" | "contentSchema" => "content-validation-dropped",
        "format" => "format-validation-widened",
        "multipleOf" => "multiple-of-validation-dropped",
        "allOf" | "anyOf" | "oneOf" | "not" => "schema-applicator-dropped",
        _ => "unknown-keyword-dropped",
    }
}

fn validate_json_keyword_value(
    keyword: &str,
    value: &Value,
    path: &str,
) -> Result<(), SchemaImportError> {
    let invalid =
        |expected: &str| Err(SchemaImportError::new(format!("{path} must be {expected}")));
    match keyword {
        "$schema" | "$id" | "$anchor" | "$dynamicAnchor" | "$ref" | "$dynamicRef"
        | "$recursiveRef" | "title" | "description" | "$comment" | "pattern" | "format"
        | "contentEncoding" | "contentMediaType" => {
            if value.is_string() {
                Ok(())
            } else {
                invalid("a string")
            }
        }
        "$recursiveAnchor" | "deprecated" | "readOnly" | "writeOnly" | "uniqueItems" => {
            if value.is_boolean() {
                Ok(())
            } else {
                invalid("a boolean")
            }
        }
        "$vocabulary" => {
            let Some(entries) = value.as_object() else {
                return invalid("an object of IRI-to-boolean entries");
            };
            if entries.values().all(Value::is_boolean) {
                Ok(())
            } else {
                invalid("an object of IRI-to-boolean entries")
            }
        }
        "$defs" | "properties" | "patternProperties" | "dependentSchemas" => {
            if value.is_object() {
                Ok(())
            } else {
                invalid("an object of schemas")
            }
        }
        "type" => schema_types(value, path).map(|_| ()),
        "enum" => validate_array(value, path, true, true),
        "examples" => {
            if value.is_array() {
                Ok(())
            } else {
                invalid("an array")
            }
        }
        "required" => validate_unique_string_array(value, path),
        "dependentRequired" => {
            let Some(entries) = value.as_object() else {
                return invalid("an object of unique string arrays");
            };
            for (name, names) in entries {
                validate_unique_string_array(names, &format!("{path}/{}", pointer_escape(name)))?;
            }
            Ok(())
        }
        "allOf" | "anyOf" | "oneOf" | "prefixItems" => validate_array(value, path, true, false),
        "multipleOf" => {
            if value.as_f64().is_some_and(|number| number > 0.0) {
                Ok(())
            } else {
                invalid("a positive JSON number")
            }
        }
        "maximum" | "exclusiveMaximum" | "minimum" | "exclusiveMinimum" => {
            if value.is_number() {
                Ok(())
            } else {
                invalid("a JSON number")
            }
        }
        "maxLength" | "minLength" | "maxItems" | "minItems" | "maxContains" | "minContains"
        | "maxProperties" | "minProperties" => validate_nonnegative_integer(value, path),
        "additionalProperties"
        | "propertyNames"
        | "items"
        | "additionalItems"
        | "contains"
        | "not"
        | "if"
        | "then"
        | "else"
        | "unevaluatedProperties"
        | "unevaluatedItems"
        | "contentSchema" => {
            if value.is_object() || value.is_boolean() {
                Ok(())
            } else {
                invalid("an object or boolean schema")
            }
        }
        // `const`, `default`, and unknown extension keywords accept any JSON
        // value. Unknown keywords remain valid annotations under 2020-12 and
        // are ledgered by the importer rather than rejected.
        _ => Ok(()),
    }
}

fn validate_array(
    value: &Value,
    path: &str,
    require_nonempty: bool,
    require_unique: bool,
) -> Result<(), SchemaImportError> {
    let values = value
        .as_array()
        .ok_or_else(|| SchemaImportError::new(format!("{path} must be an array")))?;
    if require_nonempty && values.is_empty() {
        return Err(SchemaImportError::new(format!(
            "{path} must contain at least one schema"
        )));
    }
    if require_unique {
        let mut seen = BTreeSet::new();
        for item in values {
            let canonical = serde_json::to_string(item).map_err(|error| {
                SchemaImportError::new(format!("{path} cannot be canonicalized: {error}"))
            })?;
            if !seen.insert(canonical) {
                return Err(SchemaImportError::new(format!(
                    "{path} must not contain duplicate values"
                )));
            }
        }
    }
    Ok(())
}

fn validate_unique_string_array(value: &Value, path: &str) -> Result<(), SchemaImportError> {
    let values = value
        .as_array()
        .ok_or_else(|| SchemaImportError::new(format!("{path} must be an array of strings")))?;
    let mut seen = BTreeSet::new();
    for (index, value) in values.iter().enumerate() {
        let value = value
            .as_str()
            .ok_or_else(|| SchemaImportError::new(format!("{path}/{index} must be a string")))?;
        if !seen.insert(value) {
            return Err(SchemaImportError::new(format!(
                "{path} must not contain duplicate strings"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

    fn config() -> SchemaImportConfig {
        let namespaces = Namespaces::new(
            "ex",
            &[("ex".to_owned(), "https://example.org/".to_owned())],
        )
        .expect("namespace config");
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
        .expect("datatype config");
        SchemaImportConfig::new(namespaces, datatypes)
    }

    fn compile_turtle(body: &str) -> CompiledSchema {
        let source = format!(
            r"
            @prefix ex:  <https://example.org/> .
            @prefix sh:  <http://www.w3.org/ns/shacl#> .
            @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
            @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
            {body}
            "
        );
        let dataset =
            crate::text_ingest::parse_turtle_to_dataset(&source, None).expect("parse shape");
        let shapes = crate::shapes::from_dataset(&dataset).expect("type shape");
        crate::json_schema::compile(&shapes, config().namespaces()).expect("schema compilation")
    }

    #[test]
    fn imported_patterns_execute_with_xpath_semantics() {
        use crate::engine::PreparedShapes;

        for (pattern, input, expected) in [
            ("a+", "aaa", true),
            ("a+", "bbb", false),
            ("a|b", "a", true),
            ("a|b", "b", true),
            ("a|b", "c", false),
            ("", "", true),
            ("", "anything", true),
            ("(?:)", "", true),
            (r"[^\u{0}-\u{10ffff}]", "", false),
            (r"[^\u{0}-\u{10ffff}]", "a", false),
            (r"^\u{1f980}+$", "🦀🦀", true),
            (r"^\u{1f980}+$", "a", false),
        ] {
            let schema = json!({
                "$schema": JSON_SCHEMA_DIALECT,
                "$defs": {"Probe": {
                    "type": "object",
                    "properties": {"ex:code": {"type": "string", "pattern": pattern}}
                }}
            });
            let imported =
                import_json_schema(&schema.to_string(), &config()).expect("import schema");
            let prepared = PreparedShapes::new(Arc::new(imported.shapes));
            let data = crate::text_ingest::parse_turtle_to_dataset(
                &format!(
                    "@prefix ex: <https://example.org/> . ex:probe a ex:Probe ; ex:code {} .",
                    serde_json::to_string(input).expect("literal")
                ),
                None,
            )
            .expect("data");
            let report = prepared
                .bind_dataset(&data)
                .expect("bind data")
                .validate()
                .expect("validate");
            assert_eq!(report.conforms, expected, "{pattern:?} on {input:?}");
        }
    }

    /// A JSON Schema bound becomes the SHACL bound its decimal spells: a lexical form
    /// valid for the configured decimal carrier whose value is the binary64 nearest the
    /// schema's number. The witnesses are numbers a reader that is not correctly rounded
    /// (or the x87's double-rounded fast path) reads as a neighbour.
    #[test]
    fn numeric_bounds_import_as_the_correctly_rounded_value() {
        use purrdf_xsd::ieee::reference as soft;

        let mut lexicals = soft::misread_decimals(30);
        lexicals.extend(soft::x87_fast_path_decimals(30));
        let properties = lexicals
            .iter()
            .enumerate()
            .map(|(index, lexical)| {
                format!(r#""ex:v{index}":{{"type":"number","minimum":{lexical}}}"#)
            })
            .collect::<Vec<_>>()
            .join(",");
        let schema = format!(
            r#"{{"$schema":"{JSON_SCHEMA_DIALECT}","$defs":{{"Probe":{{"type":"object","properties":{{{properties}}}}}}}}}"#
        );
        let imported = import_json_schema(&schema, &config()).expect("import schema");
        let mut bounds = BTreeMap::new();
        for shape in &imported.shapes.node_shapes {
            for property in &shape.property_shapes {
                let Path::Predicate(predicate) = &property.path else {
                    continue;
                };
                for constraint in &property.constraints {
                    if let Constraint::MinInclusive(Term::Literal(literal)) = constraint {
                        bounds.insert(predicate.as_str().to_owned(), literal.clone());
                    }
                }
            }
        }
        assert_eq!(bounds.len(), lexicals.len());
        for (index, lexical) in lexicals.iter().enumerate() {
            let literal = &bounds[&format!("https://example.org/v{index}")];
            let correct: f64 = lexical.parse().expect("decimal");
            assert_eq!(
                literal.value().parse::<f64>().map(f64::to_bits),
                Ok(correct.to_bits()),
                "{lexical} imported as {}",
                literal.value()
            );
            // In the decimal carrier's lexical space. This crate's decimal may still
            // refuse the value as past its representable range (a scale above 18 digits,
            // a magnitude past `i128`), which is a range and not a lexical verdict.
            let parsed = purrdf_xsd::parse_by_iri(literal.value(), literal.datatype_str());
            assert!(
                matches!(
                    parsed,
                    Ok(Some(_)) | Err(purrdf_xsd::XsdError::OutOfRange { .. })
                ),
                "{lexical} imported as `{}`, not a valid <{}>: {parsed:?}",
                literal.value(),
                literal.datatype_str()
            );
        }
    }

    #[test]
    fn emitted_json_schema_round_trips_byte_exactly() {
        let compiled = compile_turtle(
            r#"
            ex:PersonShape a sh:NodeShape ;
                sh:targetClass ex:Person ;
                sh:closed true ;
                sh:ignoredProperties ( ex:legacy ) ;
                sh:property [
                    sh:path ex:name ;
                    sh:datatype xsd:string ;
                    sh:minCount 1 ;
                    sh:maxCount 1 ;
                    sh:minLength 2 ;
                    sh:maxLength 80 ;
                    sh:pattern "^[A-Z]"
                ] ;
                sh:property [
                    sh:path ex:age ;
                    sh:datatype xsd:integer ;
                    sh:maxCount 1 ;
                    sh:minInclusive 0 ;
                    sh:maxExclusive 150
                ] ;
                sh:property [
                    sh:path ex:tag ;
                    sh:datatype xsd:string ;
                    sh:minCount 2 ;
                    sh:maxCount 4
                ] ;
                sh:property [
                    sh:path ex:friend ;
                    sh:class ex:Person ;
                    sh:maxCount 1
                ] ;
                sh:property [
                    sh:path ex:state ;
                    sh:maxCount 1 ;
                    sh:in ( ex:active "inactive" )
                ] ;
                sh:property [
                    sh:path ex:label ;
                    sh:maxCount 1 ;
                    sh:languageIn ( "en" "fr" )
                ] ;
                sh:property [
                    sh:path ex:resource ;
                    sh:maxCount 1 ;
                    sh:nodeKind sh:IRI
                ] ;
                sh:property [
                    sh:path ex:unshaped ;
                    sh:maxCount 1 ;
                    sh:class ex:Unshaped
                ] .
            "#,
        );
        // The projection carries every numeric literal's datatype and lexical
        // form, so the numeric property projects without a forward loss, and
        // the whole schema reads back with none.
        assert!(
            compiled.losses.is_empty(),
            "{}",
            compiled.losses.render_json()
        );
        let imported = import_compiled_schema(&compiled, &config()).expect("import schema");
        assert!(
            imported.losses.is_empty(),
            "unexpected reverse losses: {}",
            imported.losses.render_json()
        );
        let recompiled = crate::json_schema::compile(&imported.shapes, config().namespaces())
            .expect("schema compilation");
        assert_eq!(recompiled.schema_json, compiled.schema_json);
        assert_eq!(recompiled.openapi_json, compiled.openapi_json);
        assert_eq!(
            recompiled.losses.entries().len(),
            compiled.losses.entries().len()
        );
    }

    #[test]
    fn emitted_logical_shapes_round_trip_byte_exactly() {
        let compiled = compile_turtle(
            r"
            ex:ChoiceShape a sh:NodeShape ;
                sh:targetClass ex:Choice ;
                sh:and (
                    [ sh:property [ sh:path ex:base ; sh:minCount 1 ] ]
                    [ sh:property [ sh:path ex:enabled ; sh:hasValue true ] ]
                ) ;
                sh:or (
                    [ sh:property [ sh:path ex:left ; sh:minCount 1 ] ]
                    [ sh:property [ sh:path ex:right ; sh:minCount 1 ] ]
                ) ;
                sh:xone (
                    [ sh:property [ sh:path ex:alpha ; sh:minCount 1 ] ]
                    [ sh:property [ sh:path ex:beta ; sh:minCount 1 ] ]
                ) ;
                sh:not [ sh:property [ sh:path ex:blocked ; sh:minCount 1 ] ] ;
                sh:not [ sh:class ex:Other ] .

            ex:OtherShape a sh:NodeShape ;
                sh:targetClass ex:Other .
            ",
        );
        assert!(compiled.losses.is_empty());
        let imported = import_compiled_schema(&compiled, &config()).expect("import schema");
        assert!(
            imported.losses.is_empty(),
            "unexpected reverse losses: {}",
            imported.losses.render_json()
        );
        let recompiled = crate::json_schema::compile(&imported.shapes, config().namespaces())
            .expect("schema compilation");
        assert_eq!(recompiled.schema_json, compiled.schema_json);
        assert!(recompiled.losses.is_empty());
    }

    #[test]
    fn valid_unrepresentable_keywords_are_all_located_and_sound() {
        let schema = json!({
            "$schema": JSON_SCHEMA_DIALECT,
            "$id": "https://example.org/schema.json",
            "$defs": {
                "Record": {
                    "type": "object",
                    "description": "Source documentation.",
                    "properties": {
                        "ex:value": {
                            "type": "number",
                            "multipleOf": 2,
                            "x-example-rule": true
                        }
                    },
                    "patternProperties": { "^x-": { "type": "string" } },
                    "minProperties": 1,
                    "dependentRequired": { "ex:value": ["ex:other"] },
                    "if": { "required": ["ex:value"] },
                    "then": { "required": ["ex:other"] },
                    "unevaluatedProperties": false
                },
                "Scalar": { "type": "string" },
                "Impossible": false
            }
        });
        let imported = import_json_schema(&schema.to_string(), &config()).expect("import schema");
        let observed = imported
            .losses
            .entries()
            .iter()
            .map(|entry| {
                (
                    entry.code.as_ref(),
                    entry
                        .location
                        .as_ref()
                        .and_then(|location| location.subject.as_deref())
                        .expect("located loss"),
                )
            })
            .collect::<BTreeSet<_>>();
        for expected in [
            ("schema-identity-dropped", "#/$schema"),
            ("schema-identity-dropped", "#/$id"),
            ("annotation-dropped", "#/$defs/Record/description"),
            (
                "multiple-of-validation-dropped",
                "#/$defs/Record/properties/ex:value/multipleOf",
            ),
            (
                "unknown-keyword-dropped",
                "#/$defs/Record/properties/ex:value/x-example-rule",
            ),
            (
                "object-key-validation-dropped",
                "#/$defs/Record/patternProperties",
            ),
            (
                "property-count-validation-dropped",
                "#/$defs/Record/minProperties",
            ),
            (
                "dependency-validation-dropped",
                "#/$defs/Record/dependentRequired",
            ),
            ("conditional-validation-dropped", "#/$defs/Record/if"),
            ("conditional-validation-dropped", "#/$defs/Record/then"),
            (
                "unevaluated-validation-dropped",
                "#/$defs/Record/unevaluatedProperties",
            ),
            ("non-object-definition-dropped", "#/$defs/Scalar"),
            ("boolean-schema-dropped", "#/$defs/Impossible"),
        ] {
            assert!(
                observed.contains(&expected),
                "missing {expected:?}: {observed:?}"
            );
        }
        check_ledger_sound(&imported.losses, JSON_SCHEMA_SOURCE, "shacl")
            .expect("runtime ledger stays in the closed profile");
    }

    #[test]
    fn malformed_dialect_references_and_identities_fail_closed() {
        for (schema, expected) in [
            (
                json!({ "$schema": "https://example.org/other", "$defs": {} }),
                "#/$schema must be",
            ),
            (
                json!({ "$defs": { "Record": { "$ref": "https://example.org/open" } } }),
                "external or not a direct",
            ),
            (
                json!({ "$defs": { "Record": { "$ref": "#/$defs/Missing" } } }),
                "targets missing definition",
            ),
            (
                json!({ "$defs": { "Record": {
                    "type": "object",
                    "properties": { "ex:": { "type": "string" } }
                } } }),
                "non-empty local part",
            ),
        ] {
            let error = import_json_schema(&schema.to_string(), &config())
                .expect_err("malformed schema must fail");
            assert!(error.to_string().contains(expected), "{error}");
        }
    }

    #[test]
    fn malformed_keyword_values_and_identity_collisions_fail_closed() {
        for (schema, expected) in [
            (
                json!({ "$defs": { "Record": {
                    "type": "object",
                    "properties": { "ex:value": { "multipleOf": 0 } }
                } } }),
                "positive JSON number",
            ),
            (
                json!({ "$defs": { "Record": {
                    "type": "object",
                    "required": ["ex:value", "ex:value"]
                } } }),
                "duplicate strings",
            ),
            (
                json!({ "$defs": { "Record": {
                    "type": "object",
                    "anyOf": []
                } } }),
                "at least one schema",
            ),
            (
                json!({ "$defs": { "Record": {
                    "type": "object",
                    "properties": {
                        "ex:value": {
                            "$schema": "https://json-schema.org/draft/2019-09/schema"
                        }
                    }
                } } }),
                "cannot change the fixed",
            ),
            (
                json!({ "$defs": {
                    "Record": { "type": "object" },
                    "ex:Record": { "type": "object" }
                } }),
                "same class IRI",
            ),
            (
                json!({ "$defs": { "Record": {
                    "type": "object",
                    "properties": {
                        "ex:value": true,
                        "https://example.org/value": true
                    }
                } } }),
                "same property IRI",
            ),
        ] {
            let error = import_json_schema(&schema.to_string(), &config())
                .expect_err("invalid or ambiguous schema must fail");
            assert!(error.to_string().contains(expected), "{error}");
        }
    }

    #[test]
    fn valid_native_widenings_are_explicit_and_cardinality_stays_representable() {
        let schema = json!({
            "$defs": {
                "Record": {
                    "type": "object",
                    "required": ["@id"],
                    "properties": {
                        "@id": { "type": "string" },
                        "ex:values": {
                            "type": "array",
                            "minItems": 2
                        },
                        "ex:choice": {
                            "type": ["number", "string"]
                        },
                        "ex:alternative": {
                            "anyOf": [
                                { "type": "number" },
                                { "type": "string" }
                            ]
                        }
                    }
                }
            }
        });
        let imported = import_json_schema(&schema.to_string(), &config()).expect("valid schema");
        let shape = &imported.shapes.node_shapes[0];
        let values = shape
            .property_shapes
            .iter()
            .find(|property| {
                matches!(&property.path, Path::Predicate(predicate) if predicate.as_str() == "https://example.org/values")
            })
            .expect("values property");
        assert!(
            values
                .constraints
                .iter()
                .any(|constraint| matches!(constraint, Constraint::MinCount(2)))
        );
        let choice = shape
            .property_shapes
            .iter()
            .find(|property| {
                matches!(&property.path, Path::Predicate(predicate) if predicate.as_str() == "https://example.org/choice")
            })
            .expect("choice property");
        assert!(
            choice
                .constraints
                .iter()
                .all(|constraint| !matches!(constraint, Constraint::Datatype(_))),
            "a JSON union must not become an impossible SHACL datatype conjunction"
        );
        let alternative = shape
            .property_shapes
            .iter()
            .find(|property| {
                matches!(&property.path, Path::Predicate(predicate) if predicate.as_str() == "https://example.org/alternative")
            })
            .expect("alternative property");
        assert!(
            alternative
                .constraints
                .iter()
                .all(|constraint| !matches!(constraint, Constraint::Datatype(_))),
            "anyOf alternatives must not be imported as a conjunction"
        );
        let observed = imported
            .losses
            .entries()
            .iter()
            .map(|entry| {
                (
                    entry.code.as_ref(),
                    entry
                        .location
                        .as_ref()
                        .and_then(|location| location.subject.as_deref()),
                )
            })
            .collect::<BTreeSet<_>>();
        assert!(observed.contains(&(
            "value-term-kind-widened",
            Some("#/$defs/Record/properties/@id")
        )));
        assert!(observed.contains(&(
            "value-term-kind-widened",
            Some("#/$defs/Record/properties/ex:choice/type")
        )));
        assert!(observed.contains(&(
            "schema-applicator-dropped",
            Some("#/$defs/Record/properties/ex:alternative/anyOf")
        )));
    }

    #[test]
    fn datatype_and_namespace_configuration_has_no_defaults() {
        assert!(
            SchemaDatatypeMap::new(
                "relative",
                format!("{XSD}boolean"),
                format!("{XSD}integer"),
                format!("{XSD}decimal"),
                format!("{XSD}dateTime"),
                format!("{XSD}date"),
                format!("{XSD}time"),
                format!("{XSD}anyURI"),
            )
            .is_err()
        );
        let namespaces = config().namespaces;
        assert_eq!(
            namespaces.expand_iri("ex:Record").as_deref(),
            Ok("https://example.org/Record")
        );
        assert_eq!(
            namespaces.class_iri_for_def_key("Record").as_deref(),
            Ok("https://example.org/Record")
        );
        assert!(namespaces.expand_iri("unqualified").is_err());
    }
}
