// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The schema documents a compilation may reach, and how a URI finds a
//! schema in them (2020-12 Core §8.2 and §9).
//!
//! A document is registered under a retrieval URI and scanned once. Every
//! subschema that declares `$id` becomes a *resource* with its own URI; every
//! `$anchor` and `$dynamicAnchor` is recorded against the resource that
//! contains it. The scan walks only the keywords 2020-12 defines as holding
//! subschemas, so an `$id` or `$anchor` inside `enum`, `const`, `examples` or
//! an unknown keyword is data, not an identifier (Core §9.4.2).
//!
//! The draft 2020-12 meta-schemas are registered in every [`Registry`] from
//! copies vendored into this crate, so no compilation ever needs the network.

use std::collections::BTreeMap;
use std::fmt;

use serde_json::Value;

use crate::error::SchemaError;
use crate::pointer;

/// The draft 2020-12 meta-schema URI.
pub const DRAFT_2020_12: &str = "https://json-schema.org/draft/2020-12/schema";

/// `(URI, document)` for the vendored 2020-12 meta-schema and its vocabulary
/// meta-schemas.
const BUILTIN: &[(&str, &str)] = &[
    (
        DRAFT_2020_12,
        include_str!("../metaschemas/draft2020-12/schema.json"),
    ),
    (
        "https://json-schema.org/draft/2020-12/meta/core",
        include_str!("../metaschemas/draft2020-12/meta/core.json"),
    ),
    (
        "https://json-schema.org/draft/2020-12/meta/applicator",
        include_str!("../metaschemas/draft2020-12/meta/applicator.json"),
    ),
    (
        "https://json-schema.org/draft/2020-12/meta/unevaluated",
        include_str!("../metaschemas/draft2020-12/meta/unevaluated.json"),
    ),
    (
        "https://json-schema.org/draft/2020-12/meta/validation",
        include_str!("../metaschemas/draft2020-12/meta/validation.json"),
    ),
    (
        "https://json-schema.org/draft/2020-12/meta/meta-data",
        include_str!("../metaschemas/draft2020-12/meta/meta-data.json"),
    ),
    (
        "https://json-schema.org/draft/2020-12/meta/format-annotation",
        include_str!("../metaschemas/draft2020-12/meta/format-annotation.json"),
    ),
    (
        "https://json-schema.org/draft/2020-12/meta/format-assertion",
        include_str!("../metaschemas/draft2020-12/meta/format-assertion.json"),
    ),
    (
        "https://json-schema.org/draft/2020-12/meta/content",
        include_str!("../metaschemas/draft2020-12/meta/content.json"),
    ),
];

/// The meta-schema URIs of the other published JSON Schema dialects. A
/// document declaring one is recorded but never scanned: its keywords (`id`,
/// `definitions`, array-form `items`, `$recursiveRef`) mean something else,
/// and reading them under 2020-12 rules would invent identifiers. Referring to
/// such a document is [`SchemaError::UnsupportedDialect`].
const OTHER_DIALECTS: &[&str] = &[
    "http://json-schema.org/schema",
    "http://json-schema.org/draft-03/schema",
    "http://json-schema.org/draft-04/schema",
    "http://json-schema.org/draft-05/schema",
    "http://json-schema.org/draft-06/schema",
    "http://json-schema.org/draft-07/schema",
    "https://json-schema.org/schema",
    "https://json-schema.org/draft-03/schema",
    "https://json-schema.org/draft-04/schema",
    "https://json-schema.org/draft-06/schema",
    "https://json-schema.org/draft-07/schema",
    "https://json-schema.org/draft/2019-09/schema",
    "https://json-schema.org/draft/next/schema",
    "https://json-schema.org/v1",
];

/// The 2020-12 vocabulary URIs this crate implements.
const VOCABULARY_PREFIX: &str = "https://json-schema.org/draft/2020-12/vocab/";

/// Keywords whose value is an object of subschemas.
const SCHEMA_MAPS: &[&str] = &[
    "$defs",
    "definitions",
    "properties",
    "patternProperties",
    "dependentSchemas",
    "dependencies",
];
/// Keywords whose value is an array of subschemas.
const SCHEMA_ARRAYS: &[&str] = &["allOf", "anyOf", "oneOf", "prefixItems"];
/// Keywords whose value is one subschema.
const SCHEMA_SINGLES: &[&str] = &[
    "additionalProperties",
    "propertyNames",
    "items",
    "contains",
    "not",
    "if",
    "then",
    "else",
    "unevaluatedItems",
    "unevaluatedProperties",
    "contentSchema",
];

/// A 2020-12 vocabulary whose keywords assert or apply. The Core,
/// Meta-Data, Format-Annotation and Content vocabularies need no flag: their
/// keywords are identifiers or annotations whichever vocabularies are in force.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Vocabulary {
    Applicator = 1,
    Unevaluated = 2,
    Validation = 4,
    FormatAssertion = 8,
}

/// Which vocabularies a schema resource is evaluated under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Vocabularies(u8);

impl Vocabularies {
    /// The vocabularies of the 2020-12 meta-schema itself.
    const DEFAULT: Self = Self(
        Vocabulary::Applicator as u8 | Vocabulary::Unevaluated as u8 | Vocabulary::Validation as u8,
    );

    const NONE: Self = Self(0);

    const fn with(self, vocabulary: Vocabulary) -> Self {
        Self(self.0 | vocabulary as u8)
    }

    /// Whether `vocabulary` is in force.
    pub(crate) const fn has(self, vocabulary: Vocabulary) -> bool {
        self.0 & vocabulary as u8 != 0
    }
}

/// A registered document.
#[derive(Debug, Clone)]
pub(crate) struct Document {
    pub(crate) uri: String,
    pub(crate) value: Value,
    pub(crate) builtin: bool,
    /// The resource the document root is.
    root_resource: usize,
}

/// A schema resource: a document root or a subschema with `$id`.
#[derive(Debug, Clone)]
pub(crate) struct Resource {
    pub(crate) uri: String,
    pub(crate) doc: usize,
    /// The resource root's pointer within its document.
    pub(crate) pointer: String,
    parent: Option<usize>,
    /// The `$schema` its root declares, if any.
    schema: Option<String>,
    anchors: BTreeMap<String, String>,
    pub(crate) dynamic_anchors: BTreeMap<String, String>,
}

/// A location in a registered document: document index and pointer.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Location {
    pub(crate) doc: usize,
    pub(crate) pointer: String,
}

/// The registered schema documents. Construct with [`Registry::new`], add
/// documents with [`Registry::add_resource`], and compile with
/// [`Registry::compile`].
#[derive(Clone)]
pub struct Registry {
    pub(crate) docs: Vec<Document>,
    pub(crate) resources: Vec<Resource>,
    by_uri: BTreeMap<String, usize>,
    refused: BTreeMap<String, String>,
    locations: BTreeMap<(usize, String), usize>,
}

impl fmt::Debug for Registry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Registry")
            .field(
                "documents",
                &self.docs.iter().map(|doc| &doc.uri).collect::<Vec<_>>(),
            )
            .field("resources", &self.by_uri.keys().collect::<Vec<_>>())
            .field("other_dialects", &self.refused)
            .finish_non_exhaustive()
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

impl Registry {
    /// A registry holding the draft 2020-12 meta-schemas.
    pub fn new() -> Self {
        let mut registry = Self {
            docs: Vec::new(),
            resources: Vec::new(),
            by_uri: BTreeMap::new(),
            refused: BTreeMap::new(),
            locations: BTreeMap::new(),
        };
        for (uri, text) in BUILTIN {
            let value: Value =
                serde_json::from_str(text).unwrap_or_else(|_| unreachable!("vendored meta-schema"));
            registry
                .insert(uri, value, true)
                .unwrap_or_else(|_| unreachable!("vendored meta-schema"));
        }
        registry
    }

    /// Register `document` under the absolute retrieval URI `uri`.
    ///
    /// The document is scanned for `$id`, `$anchor` and `$dynamicAnchor`
    /// immediately, so a malformed identifier, or a URI another resource
    /// already claims, is refused here. A document declaring another
    /// dialect's `$schema` is recorded without being scanned; compiling
    /// anything that reaches it is [`SchemaError::UnsupportedDialect`].
    pub fn add_resource(&mut self, uri: &str, document: Value) -> Result<(), SchemaError> {
        self.insert(uri, document, false)
    }

    fn insert(&mut self, uri: &str, document: Value, builtin: bool) -> Result<(), SchemaError> {
        let uri = absolute_without_fragment(uri)?;
        if self.by_uri.contains_key(&uri) || self.refused.contains_key(&uri) {
            return Err(SchemaError::DuplicateResource { uri });
        }
        if let Some(dialect) = declared_schema(&document)
            && is_other_dialect(&dialect)
        {
            self.refused.insert(uri, dialect);
            return Ok(());
        }
        let doc = self.docs.len();
        let staged = Scan::run(self, doc, &uri, &document)?;
        let root_resource = staged.offset;
        for claimed in staged.by_uri.keys() {
            if self.by_uri.contains_key(claimed) || self.refused.contains_key(claimed) {
                return Err(SchemaError::DuplicateResource {
                    uri: claimed.clone(),
                });
            }
        }
        self.by_uri.extend(staged.by_uri);
        self.locations.extend(staged.locations);
        self.resources.extend(staged.resources);
        self.docs.push(Document {
            uri,
            value: document,
            builtin,
            root_resource,
        });
        Ok(())
    }

    /// Find the schema an absolute URI (with or without a fragment) names.
    pub(crate) fn locate(&self, absolute: &str) -> Result<Location, Locate> {
        let (base, fragment) = match absolute.split_once('#') {
            Some((base, fragment)) => (base, Some(fragment)),
            None => (absolute, None),
        };
        let Some(&resource) = self.by_uri.get(base) else {
            return Err(match self.refused.get(base) {
                Some(dialect) => Locate::OtherDialect(dialect.clone()),
                None => Locate::Unknown,
            });
        };
        let resource = &self.resources[resource];
        let fragment = match fragment {
            None | Some("") => {
                return Ok(Location {
                    doc: resource.doc,
                    pointer: resource.pointer.clone(),
                });
            }
            Some(fragment) => pointer::percent_decode(fragment).ok_or(Locate::Unknown)?,
        };
        let pointer = if fragment.starts_with('/') {
            let tokens = pointer::tokens(&fragment).ok_or(Locate::Unknown)?;
            let root = pointer::lookup_str(&self.docs[resource.doc].value, &resource.pointer)
                .ok_or(Locate::Unknown)?;
            pointer::lookup(root, &tokens).ok_or(Locate::Unknown)?;
            let mut joined = resource.pointer.clone();
            for token in &tokens {
                joined = pointer::push_token(&joined, token);
            }
            joined
        } else {
            resource
                .anchors
                .get(&fragment)
                .ok_or(Locate::Unknown)?
                .clone()
        };
        Ok(Location {
            doc: resource.doc,
            pointer,
        })
    }

    /// The resource registered under `uri` (no fragment).
    pub(crate) fn resource_by_uri(&self, uri: &str) -> Option<usize> {
        self.by_uri.get(uri).copied()
    }

    /// The resource that contains `location`: the one the scan recorded for
    /// it, or for its nearest scanned ancestor (a location inside an unknown
    /// keyword belongs to the schema that holds the keyword).
    pub(crate) fn resource_of(&self, location: &Location) -> usize {
        let mut pointer = location.pointer.as_str();
        loop {
            if let Some(&resource) = self.locations.get(&(location.doc, pointer.to_owned())) {
                return resource;
            }
            match pointer.rfind('/') {
                Some(cut) => pointer = &pointer[..cut],
                None => return self.docs[location.doc].root_resource,
            }
        }
    }

    /// The value at `location`.
    pub(crate) fn value(&self, location: &Location) -> Option<&Value> {
        pointer::lookup_str(&self.docs[location.doc].value, &location.pointer)
    }

    /// The URI a resource's `$dynamicAnchor` named `name` is reachable at,
    /// if it declares one.
    pub(crate) fn dynamic_anchor(&self, resource: usize, name: &str) -> Option<Location> {
        let resource = &self.resources[resource];
        resource.dynamic_anchors.get(name).map(|pointer| Location {
            doc: resource.doc,
            pointer: pointer.clone(),
        })
    }

    /// The vocabularies `resource` is evaluated under, from the `$schema` its
    /// root declares or the nearest enclosing resource's.
    pub(crate) fn vocabularies(&self, resource: usize) -> Result<Vocabularies, SchemaError> {
        self.vocabularies_at(resource, 0)
    }

    fn vocabularies_at(&self, resource: usize, depth: usize) -> Result<Vocabularies, SchemaError> {
        let mut current = resource;
        let declared = loop {
            if let Some(schema) = &self.resources[current].schema {
                break schema.clone();
            }
            match self.resources[current].parent {
                Some(parent) => current = parent,
                None => break DRAFT_2020_12.to_owned(),
            }
        };
        let refuse = || SchemaError::UnsupportedDialect {
            resource: self.resources[resource].uri.clone(),
            dialect: declared.clone(),
        };
        let metaschema_uri = declared.strip_suffix('#').unwrap_or(&declared);
        if metaschema_uri == DRAFT_2020_12 {
            return Ok(Vocabularies::DEFAULT);
        }
        // A custom meta-schema must itself be a 2020-12 schema: follow its own
        // `$schema` chain, which ends at the 2020-12 meta-schema or is refused.
        let Some(&meta) = self.by_uri.get(metaschema_uri) else {
            return Err(refuse());
        };
        if depth > 32 || self.vocabularies_at(meta, depth + 1).is_err() {
            return Err(refuse());
        }
        let meta_resource = &self.resources[meta];
        let root = self
            .value(&Location {
                doc: meta_resource.doc,
                pointer: meta_resource.pointer.clone(),
            })
            .ok_or_else(refuse)?;
        let Some(declared_vocabularies) = root.get("$vocabulary") else {
            return Ok(Vocabularies::DEFAULT);
        };
        let Some(declared_vocabularies) = declared_vocabularies.as_object() else {
            return Err(SchemaError::InvalidKeyword {
                location: format!("{metaschema_uri}#/$vocabulary"),
                reason: "`$vocabulary` must be an object".to_owned(),
            });
        };
        let mut vocabularies = Vocabularies::NONE;
        for (vocabulary, required) in declared_vocabularies {
            let known = vocabulary
                .strip_prefix(VOCABULARY_PREFIX)
                .and_then(|name| match name {
                    "core" | "format-annotation" | "meta-data" | "content" => Some(None),
                    "applicator" => Some(Some(Vocabulary::Applicator)),
                    "unevaluated" => Some(Some(Vocabulary::Unevaluated)),
                    "validation" => Some(Some(Vocabulary::Validation)),
                    "format-assertion" => Some(Some(Vocabulary::FormatAssertion)),
                    _ => None,
                });
            if let Some(Some(flag)) = known {
                vocabularies = vocabularies.with(flag);
            }
            if known.is_none() && required.as_bool() != Some(false) {
                return Err(SchemaError::UnsupportedVocabulary {
                    metaschema: metaschema_uri.to_owned(),
                    vocabulary: vocabulary.clone(),
                });
            }
        }
        Ok(vocabularies)
    }

    /// The meta-schema URI a document's root declares, defaulting to 2020-12.
    pub(crate) fn document_dialect(&self, doc: usize) -> String {
        declared_schema(&self.docs[doc].value).map_or_else(
            || DRAFT_2020_12.to_owned(),
            |schema| schema.strip_suffix('#').unwrap_or(&schema).to_owned(),
        )
    }
}

/// Why [`Registry::locate`] found nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Locate {
    /// No registered resource, anchor or pointer target has that URI.
    Unknown,
    /// The URI names a document declaring this other dialect.
    OtherDialect(String),
}

fn declared_schema(document: &Value) -> Option<String> {
    document
        .get("$schema")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn is_other_dialect(declared: &str) -> bool {
    let bare = declared.strip_suffix('#').unwrap_or(declared);
    OTHER_DIALECTS.contains(&bare)
}

/// Resolve `reference` against `base` with the workspace's RFC 3986 resolver.
pub(crate) fn resolve(base: &str, reference: &str) -> Result<String, SchemaError> {
    let base_iri = purrdf_iri::parse(base).map_err(|error| SchemaError::InvalidUri {
        uri: base.to_owned(),
        reason: error.to_string(),
    })?;
    base_iri
        .resolve(reference)
        .map(|resolved| resolved.as_str().to_owned())
        .map_err(|error| SchemaError::InvalidUri {
            uri: reference.to_owned(),
            reason: error.to_string(),
        })
}

/// `uri` as an absolute IRI without its fragment; an empty fragment is
/// dropped and a non-empty one refused (2020-12 Core §8.2.1).
fn absolute_without_fragment(uri: &str) -> Result<String, SchemaError> {
    let parsed = purrdf_iri::parse(uri).map_err(|error| SchemaError::InvalidUri {
        uri: uri.to_owned(),
        reason: error.to_string(),
    })?;
    if !parsed.has_scheme() {
        return Err(SchemaError::InvalidUri {
            uri: uri.to_owned(),
            reason: "a retrieval URI must be absolute".to_owned(),
        });
    }
    strip_empty_fragment(uri)
}

fn strip_empty_fragment(uri: &str) -> Result<String, SchemaError> {
    match uri.split_once('#') {
        None => Ok(uri.to_owned()),
        Some((base, "")) => Ok(base.to_owned()),
        Some(_) => Err(SchemaError::InvalidUri {
            uri: uri.to_owned(),
            reason: "a resource URI may not carry a non-empty fragment".to_owned(),
        }),
    }
}

/// `^[A-Za-z_][-A-Za-z0-9._]*$` — the 2020-12 `anchorString`.
fn is_anchor_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_'))
}

/// The identifiers found in one document, committed only if all are valid.
struct Scan {
    resources: Vec<Resource>,
    by_uri: BTreeMap<String, usize>,
    locations: BTreeMap<(usize, String), usize>,
    offset: usize,
    doc: usize,
}

impl Scan {
    fn run(
        registry: &Registry,
        doc: usize,
        retrieval: &str,
        root: &Value,
    ) -> Result<Self, SchemaError> {
        let mut scan = Self {
            resources: Vec::new(),
            by_uri: BTreeMap::new(),
            locations: BTreeMap::new(),
            offset: registry.resources.len(),
            doc,
        };
        let uri = match root.get("$id") {
            Some(id) => scan.identifier(retrieval, id, "")?,
            None => retrieval.to_owned(),
        };
        let resource = scan.add_resource(uri.clone(), String::new(), None, root)?;
        if uri != retrieval {
            scan.claim(retrieval.to_owned(), resource)?;
        }
        let mut stack = vec![(String::new(), root, resource)];
        while let Some((pointer, value, resource)) = stack.pop() {
            scan.locations.insert((doc, pointer.clone()), resource);
            let Some(object) = value.as_object() else {
                continue;
            };
            scan.anchors(object, &pointer, resource)?;
            let mut children: Vec<(String, &Value)> = Vec::new();
            for &keyword in SCHEMA_MAPS {
                if let Some(Value::Object(map)) = object.get(keyword) {
                    let here = pointer::push_token(&pointer, keyword);
                    for (name, child) in map {
                        children.push((pointer::push_token(&here, name), child));
                    }
                }
            }
            for &keyword in SCHEMA_ARRAYS {
                if let Some(Value::Array(items)) = object.get(keyword) {
                    let here = pointer::push_token(&pointer, keyword);
                    for (index, child) in items.iter().enumerate() {
                        children.push((pointer::push_token(&here, &index.to_string()), child));
                    }
                }
            }
            for &keyword in SCHEMA_SINGLES {
                if let Some(child) = object.get(keyword) {
                    children.push((pointer::push_token(&pointer, keyword), child));
                }
            }
            for (child_pointer, child) in children {
                if !(child.is_object() || child.is_boolean()) {
                    continue;
                }
                let child_resource = match child.get("$id") {
                    Some(id) => {
                        let base = scan.resource(resource).uri.clone();
                        let uri = scan.identifier(&base, id, &child_pointer)?;
                        scan.add_resource(uri, child_pointer.clone(), Some(resource), child)?
                    }
                    None => resource,
                };
                stack.push((child_pointer, child, child_resource));
            }
        }
        Ok(scan)
    }

    fn resource(&self, index: usize) -> &Resource {
        &self.resources[index - self.offset]
    }

    fn resource_mut(&mut self, index: usize) -> &mut Resource {
        &mut self.resources[index - self.offset]
    }

    fn identifier(&self, base: &str, id: &Value, at: &str) -> Result<String, SchemaError> {
        let Some(id) = id.as_str() else {
            return Err(SchemaError::InvalidIdentifier {
                location: format!("{base}#{at}/$id"),
                reason: "`$id` must be a string".to_owned(),
            });
        };
        let resolved = resolve(base, id)?;
        strip_empty_fragment(&resolved).map_err(|_| SchemaError::InvalidIdentifier {
            location: format!("{base}#{at}/$id"),
            reason: format!("`$id` {id:?} may not carry a non-empty fragment"),
        })
    }

    fn add_resource(
        &mut self,
        uri: String,
        pointer: String,
        parent: Option<usize>,
        root: &Value,
    ) -> Result<usize, SchemaError> {
        let index = self.offset + self.resources.len();
        self.claim(uri.clone(), index)?;
        self.resources.push(Resource {
            uri,
            doc: self.doc,
            pointer,
            parent,
            schema: declared_schema(root),
            anchors: BTreeMap::new(),
            dynamic_anchors: BTreeMap::new(),
        });
        Ok(index)
    }

    fn claim(&mut self, uri: String, resource: usize) -> Result<(), SchemaError> {
        if self.by_uri.contains_key(&uri) {
            return Err(SchemaError::DuplicateResource { uri });
        }
        self.by_uri.insert(uri, resource);
        Ok(())
    }

    fn anchors(
        &mut self,
        object: &serde_json::Map<String, Value>,
        pointer: &str,
        resource: usize,
    ) -> Result<(), SchemaError> {
        for (keyword, dynamic) in [("$anchor", false), ("$dynamicAnchor", true)] {
            let Some(anchor) = object.get(keyword) else {
                continue;
            };
            let location = format!("{}#{pointer}/{keyword}", self.resource(resource).uri);
            let Some(name) = anchor.as_str().filter(|name| is_anchor_name(name)) else {
                return Err(SchemaError::InvalidIdentifier {
                    location,
                    reason: format!(
                        "{anchor} is not an anchor name (`^[A-Za-z_][-A-Za-z0-9._]*$`)"
                    ),
                });
            };
            let entry = self.resource_mut(resource);
            if let Some(existing) = entry.anchors.get(name)
                && existing != pointer
            {
                return Err(SchemaError::InvalidIdentifier {
                    location,
                    reason: format!("the anchor {name:?} is declared twice in one resource"),
                });
            }
            entry.anchors.insert(name.to_owned(), pointer.to_owned());
            if dynamic {
                entry
                    .dynamic_anchors
                    .insert(name.to_owned(), pointer.to_owned());
            }
        }
        Ok(())
    }
}
