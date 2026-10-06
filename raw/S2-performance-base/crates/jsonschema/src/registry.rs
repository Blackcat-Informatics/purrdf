// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The schema documents a compilation may reach, and how a URI finds a
//! schema in them (2020-12 Core §8.2 and §9, 2019-09 Core §8.2, draft-07
//! Core §8).
//!
//! A document is registered under a retrieval URI and scanned once. Every
//! subschema that declares an identifier becomes a *resource* with its own
//! URI; every anchor is recorded against the resource that contains it. Each
//! resource is read in its own dialect (see [`Dialect`]), decided at
//! registration, and the scan walks only the keywords that dialect defines
//! as holding subschemas, so an `$id` inside `enum`, `const`, `examples` or
//! an unknown keyword is data, not an identifier.
//!
//! No meta-schema is built in. The caller registers the ones it needs —
//! usually once, as a shared [`Metaschemas`] set that
//! [`Registry::with_metaschemas`] starts every registry from.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use purrdf_lex::json::{Object, Value};

use crate::dialect::{self, Dialect, Vocabularies};
use crate::error::SchemaError;
use crate::meta_set::Metaschemas;
use crate::pointer;
use purrdf_iri::percent;

/// The draft 2020-12 meta-schema URI.
pub const DRAFT_2020_12: &str = "https://json-schema.org/draft/2020-12/schema";

/// A registered document. Immutable once registered, and shared between a
/// registry and its clones.
#[derive(Debug)]
pub(crate) struct Document {
    pub(crate) uri: String,
    pub(crate) value: Value,
    /// Registered as a member of a [`Metaschemas`] set: checked against its
    /// own meta-schema when the set was built, never again.
    pub(crate) meta: bool,
    /// The resource the document root is.
    pub(crate) root_resource: usize,
}

/// A schema resource: a document root or a subschema with an identifier.
#[derive(Debug, Clone)]
pub(crate) struct Resource {
    pub(crate) uri: String,
    pub(crate) doc: usize,
    /// The resource root's pointer within its document.
    pub(crate) pointer: String,
    /// The meta-schema the resource is evaluated under: its own `$schema`
    /// (without the empty fragment), else its enclosing resource's, else its
    /// document's dialect.
    pub(crate) metaschema: String,
    /// The dialect that meta-schema is built on, or the unsupported dialect
    /// identifier it declares.
    pub(crate) dialect: Result<Dialect, String>,
    anchors: BTreeMap<String, String>,
    pub(crate) dynamic_anchors: BTreeMap<String, String>,
    /// 2019-09 `$recursiveAnchor: true` at the resource root.
    pub(crate) recursive_anchor: bool,
}

/// A location in a registered document: document index and pointer.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Location {
    pub(crate) doc: usize,
    pub(crate) pointer: String,
}

/// The registered schema documents.
///
/// Construct with [`Registry::new`] (empty) or [`Registry::with_metaschemas`]
/// (holding a shared meta-schema set), add documents with
/// [`Registry::add_resource`], and compile with [`Registry::compile`].
///
/// # Meta-schemas
///
/// Every compiled document is checked against the meta-schema its `$schema`
/// names, and no meta-schema is built into this crate: the caller registers
/// them. There are two ways, and both may be mixed:
///
/// * a [`Metaschemas`] set, built once and shared: [`Registry::with_metaschemas`]
///   starts a registry holding its documents, whose meta-validators the set
///   compiles once and caches for every registry that uses it;
/// * [`Registry::add_resource`] with the meta-schema document under its
///   identifier, compiled afresh for each use — the way a custom meta-schema
///   that one schema uses is usually supplied.
///
/// A needed meta-schema registered neither way is
/// [`SchemaError::MissingMetaschema`], naming its URI.
///
/// Cloning is cheap: documents are shared, never re-parsed or copied.
#[derive(Clone)]
pub struct Registry {
    pub(crate) docs: Vec<Arc<Document>>,
    pub(crate) resources: Vec<Resource>,
    by_uri: BTreeMap<String, usize>,
    locations: BTreeMap<(usize, String), usize>,
    default_dialect: Dialect,
    pub(crate) format_assertion: bool,
    pub(crate) metaschemas: Option<Metaschemas>,
}

impl fmt::Debug for Registry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Registry")
            .field(
                "documents",
                &self.docs.iter().map(|doc| &doc.uri).collect::<Vec<_>>(),
            )
            .field("resources", &self.by_uri.keys().collect::<Vec<_>>())
            .field("default_dialect", &self.default_dialect)
            .field("format_assertion", &self.format_assertion)
            .finish_non_exhaustive()
    }
}

purrdf_hash::default_from_new!(Registry);

impl Registry {
    /// An empty registry: no documents, no meta-schemas, documents without
    /// `$schema` read as draft 2020-12, `format` an annotation.
    pub const fn new() -> Self {
        Self {
            docs: Vec::new(),
            resources: Vec::new(),
            by_uri: BTreeMap::new(),
            locations: BTreeMap::new(),
            default_dialect: Dialect::Draft2020_12,
            format_assertion: false,
            metaschemas: None,
        }
    }

    /// A registry holding every document of `metaschemas`, so that
    /// meta-validation and `$ref`s into those URIs resolve through them.
    ///
    /// The documents are shared with the set, and the meta-validators the
    /// set compiled are reused: starting a registry this way parses and
    /// compiles nothing.
    pub fn with_metaschemas(metaschemas: &Metaschemas) -> Self {
        let mut registry = metaschemas.registry().clone();
        registry.metaschemas = Some(metaschemas.clone());
        registry
    }

    /// The dialect a document registered afterwards with
    /// [`Registry::add_resource`] is read in when its root declares no
    /// `$schema`. Draft 2020-12 unless changed.
    pub const fn set_default_dialect(&mut self, dialect: Dialect) {
        self.default_dialect = dialect;
    }

    /// Whether `format` asserts in schemas compiled afterwards.
    ///
    /// Off by default: every supported draft makes `format` an annotation
    /// unless configured otherwise. On, every format the schema's dialect
    /// defines is checked, and a format name the dialect does not define
    /// stays an annotation — except under a 2020-12 meta-schema declaring
    /// the Format-Assertion vocabulary, which refuses such a name with
    /// [`SchemaError::UnsupportedFormat`] either way (2020-12 Validation
    /// §7.2.3). Meta-schema validation always reads `format` as an
    /// annotation.
    pub const fn set_format_assertion(&mut self, assert: bool) {
        self.format_assertion = assert;
    }

    /// Register `document` under the absolute retrieval URI `uri`, in the
    /// registry's default dialect when its root declares no `$schema`.
    ///
    /// The document is scanned for identifiers and anchors immediately, so a
    /// malformed identifier, or a URI another resource already claims, is
    /// refused here. A `$schema` naming a custom meta-schema needs that
    /// meta-schema registered first
    /// ([`SchemaError::MissingMetaschema`] otherwise): the dialect it is
    /// built on decides how the document's identifiers are read. A document
    /// declaring an unsupported dialect is recorded without being scanned;
    /// compiling anything that reaches it is
    /// [`SchemaError::UnsupportedDialect`].
    pub fn add_resource(&mut self, uri: &str, document: Value) -> Result<(), SchemaError> {
        self.insert(uri, document, false, self.default_dialect)
    }

    /// [`Registry::add_resource`], reading the document in `dialect` when its
    /// root declares no `$schema` — for a document whose dialect is known
    /// from where it came from rather than from its content.
    pub fn add_resource_with_dialect(
        &mut self,
        uri: &str,
        document: Value,
        dialect: Dialect,
    ) -> Result<(), SchemaError> {
        self.insert(uri, document, false, dialect)
    }

    pub(crate) fn insert(
        &mut self,
        uri: &str,
        mut document: Value,
        meta: bool,
        dialect: Dialect,
    ) -> Result<(), SchemaError> {
        let uri = absolute_without_fragment(uri)?;
        if self.by_uri.contains_key(&uri) {
            return Err(SchemaError::DuplicateResource { uri });
        }
        refuse_repeated_members(&uri, &document)?;
        // A schema object is a set of keywords (2020-12 Core §4.3.1): its
        // member order carries nothing, so the document is held in name order
        // and every walk over it — compilation, identifier scanning,
        // meta-validation, annotations copied into output — is deterministic
        // whatever order the caller wrote.
        document.sort_keys();
        let doc = self.docs.len();
        let Scan {
            resources,
            by_uri,
            locations,
            offset: root_resource,
            ..
        } = Scan::run(self, doc, &uri, &document, dialect)?;
        for claimed in by_uri.keys() {
            if self.by_uri.contains_key(claimed) {
                return Err(SchemaError::DuplicateResource {
                    uri: claimed.clone(),
                });
            }
        }
        self.by_uri.extend(by_uri);
        self.locations.extend(locations);
        self.resources.extend(resources);
        self.docs.push(Arc::new(Document {
            uri,
            value: document,
            meta,
            root_resource,
        }));
        Ok(())
    }

    /// Find the schema an absolute URI (with or without a fragment) names;
    /// `None` when nothing registered has that URI.
    pub(crate) fn locate(&self, absolute: &str) -> Option<Location> {
        let (base, fragment) = match absolute.split_once('#') {
            Some((base, fragment)) => (base, Some(fragment)),
            None => (absolute, None),
        };
        let resource = &self.resources[*self.by_uri.get(base)?];
        let fragment = match fragment {
            None | Some("") => {
                return Some(Location {
                    doc: resource.doc,
                    pointer: resource.pointer.clone(),
                });
            }
            Some(fragment) => percent::decode(fragment).ok()?.into_owned(),
        };
        let pointer = if fragment.starts_with('/') {
            // An escaped pointer has one spelling per token sequence, so the
            // fragment appends to the resource's location as written.
            let mut joined = resource.pointer.clone();
            joined.push_str(&fragment);
            self.docs[resource.doc].value.pointer(&joined)?;
            joined
        } else {
            resource.anchors.get(&fragment)?.clone()
        };
        Some(Location {
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
        self.docs[location.doc].value.pointer(&location.pointer)
    }

    /// The location of the `$dynamicAnchor` named `name` in `resource`, if it
    /// declares one.
    pub(crate) fn dynamic_anchor(&self, resource: usize, name: &str) -> Option<Location> {
        let resource = &self.resources[resource];
        resource.dynamic_anchors.get(name).map(|pointer| Location {
            doc: resource.doc,
            pointer: pointer.clone(),
        })
    }

    /// The dialect and vocabularies `resource` is evaluated under.
    pub(crate) fn vocabularies(
        &self,
        resource: usize,
    ) -> Result<(Dialect, Vocabularies), SchemaError> {
        let entry = &self.resources[resource];
        let dialect =
            entry
                .dialect
                .clone()
                .map_err(|declared| SchemaError::UnsupportedDialect {
                    resource: entry.uri.clone(),
                    dialect: declared,
                })?;
        if Dialect::from_uri(&entry.metaschema).is_some() || dialect == Dialect::Draft07 {
            return Ok((dialect, dialect.default_vocabularies()));
        }
        // A custom meta-schema, registered before the resource was (the scan
        // refuses it otherwise): its `$vocabulary` decides.
        let metaschema = &entry.metaschema;
        let root = self
            .resource_by_uri(metaschema)
            .and_then(|meta| {
                let meta = &self.resources[meta];
                self.value(&Location {
                    doc: meta.doc,
                    pointer: meta.pointer.clone(),
                })
            })
            .ok_or_else(|| SchemaError::MissingMetaschema {
                metaschema: metaschema.clone(),
                resource: entry.uri.clone(),
            })?;
        let Some(declared) = root.get("$vocabulary") else {
            return Ok((dialect, dialect.default_vocabularies()));
        };
        let Some(declared) = declared.as_object() else {
            return Err(SchemaError::InvalidKeyword {
                location: format!("{metaschema}#/$vocabulary"),
                reason: "`$vocabulary` must be an object".to_owned(),
            });
        };
        let mut vocabularies = Vocabularies::NONE;
        for (vocabulary, required) in declared {
            let required = required.as_bool() != Some(false);
            match dialect.vocabulary(vocabulary, required) {
                Some(flags) => vocabularies = vocabularies.union(flags),
                None if required => {
                    return Err(SchemaError::UnsupportedVocabulary {
                        metaschema: metaschema.clone(),
                        vocabulary: vocabulary.clone(),
                    });
                }
                None => {}
            }
        }
        Ok((dialect, vocabularies))
    }
}

/// Refuse a document in which some object repeats a member name: JSON leaves
/// the meaning of a repeat open (RFC 8259 §4), and a schema read by one of the
/// two occurrences would silently drop the other. The walk is over a heap
/// stack, so any nesting depth is checked, and a value's location is spelled
/// out only for the refusal.
fn refuse_repeated_members(uri: &str, document: &Value) -> Result<(), SchemaError> {
    // Every container reached: its parent's entry and the step from it.
    let mut steps: Vec<(usize, Step<'_>)> = vec![(0, Step::Root)];
    let mut stack = vec![(0_usize, document)];
    while let Some((at, value)) = stack.pop() {
        match value {
            Value::Object(object) => {
                if let Some(name) = object.first_duplicate() {
                    return Err(SchemaError::InvalidKeyword {
                        location: format!("{uri}#{}", location(&steps, at)),
                        reason: format!("the object repeats the member name {name:?}"),
                    });
                }
                for (name, member) in object.iter() {
                    descend(&mut steps, &mut stack, at, Step::Member(name), member);
                }
            }
            Value::Array(items) => {
                for (index, item) in items.iter().enumerate() {
                    descend(&mut steps, &mut stack, at, Step::Item(index), item);
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// One step from a container to a child, as the repeated-member walk records
/// it.
enum Step<'v> {
    Root,
    Member(&'v str),
    Item(usize),
}

/// Record `child` of the container at `at` for the walk, when it is itself a
/// container.
fn descend<'v>(
    steps: &mut Vec<(usize, Step<'v>)>,
    stack: &mut Vec<(usize, &'v Value)>,
    at: usize,
    step: Step<'v>,
    child: &'v Value,
) {
    if matches!(child, Value::Object(_) | Value::Array(_)) {
        stack.push((steps.len(), child));
        steps.push((at, step));
    }
}

/// The escaped JSON Pointer of the container recorded at `at`.
fn location(steps: &[(usize, Step<'_>)], mut at: usize) -> String {
    let mut tokens = Vec::new();
    loop {
        match &steps[at] {
            (_, Step::Root) => break,
            (parent, Step::Member(name)) => {
                tokens.push((*name).to_owned());
                at = *parent;
            }
            (parent, Step::Item(index)) => {
                tokens.push(index.to_string());
                at = *parent;
            }
        }
    }
    tokens.iter().rev().fold(String::new(), |here, token| {
        pointer::push_token(&here, token)
    })
}

fn declared_schema(document: &Value) -> Option<&str> {
    document.get("$schema").and_then(Value::as_str)
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
    match uri.split_once('#') {
        None => Ok(uri.to_owned()),
        Some((base, "")) => Ok(base.to_owned()),
        Some(_) => Err(SchemaError::InvalidUri {
            uri: uri.to_owned(),
            reason: "a resource URI may not carry a non-empty fragment".to_owned(),
        }),
    }
}

/// The dialect a resource inherits or declares.
#[derive(Debug, Clone)]
struct Declared {
    metaschema: String,
    dialect: Result<Dialect, String>,
}

/// The identifiers found in one document, committed only if all are valid.
struct Scan<'r> {
    registry: &'r Registry,
    resources: Vec<Resource>,
    by_uri: BTreeMap<String, usize>,
    locations: BTreeMap<(usize, String), usize>,
    offset: usize,
    doc: usize,
}

impl<'r> Scan<'r> {
    fn run(
        registry: &'r Registry,
        doc: usize,
        retrieval: &str,
        root: &Value,
        dialect: Dialect,
    ) -> Result<Self, SchemaError> {
        let mut scan = Self {
            registry,
            resources: Vec::new(),
            by_uri: BTreeMap::new(),
            locations: BTreeMap::new(),
            offset: registry.resources.len(),
            doc,
        };
        let inherited = Declared {
            metaschema: dialect.uri().to_owned(),
            dialect: Ok(dialect),
        };
        let declared = scan.declared(root, &inherited, retrieval)?;
        let (uri, root_anchor) = match (&declared.dialect, root.get("$id")) {
            (Ok(dialect), Some(id)) if !ref_hides_siblings(*dialect, root) => {
                identifier(*dialect, retrieval, id, "")?
            }
            _ => (retrieval.to_owned(), None),
        };
        let resource = scan.add_resource(uri.clone(), String::new(), declared)?;
        if uri != retrieval {
            scan.claim(retrieval.to_owned(), resource)?;
        }
        if let Some(anchor) = root_anchor {
            scan.add_anchor(resource, &anchor, "", false)?;
        }
        let mut stack = vec![(String::new(), root, resource)];
        while let Some((pointer, value, resource)) = stack.pop() {
            scan.locations.insert((doc, pointer.clone()), resource);
            let Some(object) = value.as_object() else {
                continue;
            };
            let Ok(dialect) = scan.resource(resource).dialect else {
                // An unsupported dialect's keywords mean something else:
                // nothing inside is read as an identifier.
                continue;
            };
            if ref_hides_siblings(dialect, value) {
                continue;
            }
            scan.anchors(dialect, object, &pointer, resource)?;
            for (child_pointer, child) in children(dialect, object, &pointer) {
                if !matches!(child, Value::Object(_) | Value::Bool(_)) {
                    continue;
                }
                let child_resource = scan.child(dialect, resource, &child_pointer, child)?;
                stack.push((child_pointer, child, child_resource));
            }
        }
        Ok(scan)
    }

    /// The resource `child` (a subschema of `parent`, read in `dialect`) is
    /// in: a new one when it declares an identifier, else its parent's.
    fn child(
        &mut self,
        dialect: Dialect,
        parent: usize,
        pointer: &str,
        child: &Value,
    ) -> Result<usize, SchemaError> {
        if ref_hides_siblings(dialect, child) {
            return Ok(parent);
        }
        let Some(id) = child.get("$id") else {
            return Ok(parent);
        };
        let base = self.resource(parent).uri.clone();
        let (target, anchor) = identifier(dialect, &base, id, pointer)?;
        let resource = if dialect == Dialect::Draft07 && target == base {
            // A draft-07 `$id` that is only a plain-name fragment names a
            // location in the current resource.
            parent
        } else {
            let inherited = {
                let parent = self.resource(parent);
                Declared {
                    metaschema: parent.metaschema.clone(),
                    dialect: parent.dialect.clone(),
                }
            };
            let declared = self.declared(child, &inherited, &target)?;
            self.add_resource(target, pointer.to_owned(), declared)?
        };
        if let Some(anchor) = anchor {
            self.add_anchor(resource, &anchor, pointer, false)?;
        }
        Ok(resource)
    }

    /// The dialect of a resource root: its `$schema`, else `inherited`.
    fn declared(
        &self,
        root: &Value,
        inherited: &Declared,
        resource: &str,
    ) -> Result<Declared, SchemaError> {
        let Some(schema) = declared_schema(root) else {
            return Ok(inherited.clone());
        };
        if let Some(dialect) = Dialect::from_uri(schema) {
            return Ok(Declared {
                metaschema: dialect.uri().to_owned(),
                dialect: Ok(dialect),
            });
        }
        let bare = schema.strip_suffix('#').unwrap_or(schema);
        if dialect::is_unsupported(schema) {
            return Ok(Declared {
                metaschema: bare.to_owned(),
                dialect: Err(schema.to_owned()),
            });
        }
        // A custom meta-schema: the dialect it is itself written in.
        let Some(meta) = self.registry.resource_by_uri(bare) else {
            return Err(SchemaError::MissingMetaschema {
                metaschema: bare.to_owned(),
                resource: resource.to_owned(),
            });
        };
        Ok(Declared {
            metaschema: bare.to_owned(),
            dialect: self.registry.resources[meta].dialect.clone(),
        })
    }

    fn resource(&self, index: usize) -> &Resource {
        &self.resources[index - self.offset]
    }

    fn resource_mut(&mut self, index: usize) -> &mut Resource {
        &mut self.resources[index - self.offset]
    }

    fn add_resource(
        &mut self,
        uri: String,
        pointer: String,
        declared: Declared,
    ) -> Result<usize, SchemaError> {
        let index = self.offset + self.resources.len();
        self.claim(uri.clone(), index)?;
        self.resources.push(Resource {
            uri,
            doc: self.doc,
            pointer,
            metaschema: declared.metaschema,
            dialect: declared.dialect,
            anchors: BTreeMap::new(),
            dynamic_anchors: BTreeMap::new(),
            recursive_anchor: false,
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

    fn add_anchor(
        &mut self,
        resource: usize,
        name: &str,
        pointer: &str,
        dynamic: bool,
    ) -> Result<(), SchemaError> {
        let entry = self.resource_mut(resource);
        if let Some(existing) = entry.anchors.get(name)
            && existing != pointer
        {
            return Err(SchemaError::InvalidIdentifier {
                location: format!("{}#{pointer}", entry.uri),
                reason: format!("the anchor {name:?} is declared twice in one resource"),
            });
        }
        entry.anchors.insert(name.to_owned(), pointer.to_owned());
        if dynamic {
            entry
                .dynamic_anchors
                .insert(name.to_owned(), pointer.to_owned());
        }
        Ok(())
    }

    fn anchors(
        &mut self,
        dialect: Dialect,
        object: &Object,
        pointer: &str,
        resource: usize,
    ) -> Result<(), SchemaError> {
        let keywords: &[(&str, bool)] = match dialect {
            Dialect::Draft07 => &[],
            Dialect::Draft2019_09 => &[("$anchor", false)],
            Dialect::Draft2020_12 => &[("$anchor", false), ("$dynamicAnchor", true)],
        };
        for &(keyword, dynamic) in keywords {
            let Some(anchor) = object.get(keyword) else {
                continue;
            };
            let Some(name) = anchor.as_str().filter(|name| dialect.is_anchor_name(name)) else {
                return Err(SchemaError::InvalidIdentifier {
                    location: format!("{}#{pointer}/{keyword}", self.resource(resource).uri),
                    reason: format!("{anchor} is not an anchor name"),
                });
            };
            self.add_anchor(resource, name, pointer, dynamic)?;
        }
        if dialect == Dialect::Draft2019_09
            && self.resource(resource).pointer == pointer
            && object.get("$recursiveAnchor") == Some(&Value::Bool(true))
        {
            self.resource_mut(resource).recursive_anchor = true;
        }
        Ok(())
    }
}

/// Draft-07 Core §8.3: every sibling of `$ref`, `$id` included, is ignored.
fn ref_hides_siblings(dialect: Dialect, value: &Value) -> bool {
    dialect == Dialect::Draft07 && value.get("$ref").is_some()
}

/// The subschemas directly beneath `object`, as `(pointer, value)`.
fn children<'v>(dialect: Dialect, object: &'v Object, pointer: &str) -> Vec<(String, &'v Value)> {
    let keywords = dialect.subschemas();
    let mut children = Vec::new();
    for &keyword in keywords.maps {
        if let Some(Value::Object(map)) = object.get(keyword) {
            let here = pointer::push_token(pointer, keyword);
            for (name, child) in map {
                children.push((pointer::push_token(&here, name), child));
            }
        }
    }
    for &keyword in keywords.arrays {
        if let Some(Value::Array(items)) = object.get(keyword) {
            let here = pointer::push_token(pointer, keyword);
            for (index, child) in items.iter().enumerate() {
                children.push((pointer::push_token(&here, &index.to_string()), child));
            }
        }
    }
    for &keyword in keywords.singles {
        if let Some(child) = object.get(keyword) {
            children.push((pointer::push_token(pointer, keyword), child));
        }
    }
    children
}

/// Read an `$id` in `dialect` against `base`: the resource URI it names and,
/// for a draft-07 plain-name fragment, the anchor it declares.
fn identifier(
    dialect: Dialect,
    base: &str,
    id: &Value,
    at: &str,
) -> Result<(String, Option<String>), SchemaError> {
    let location = || format!("{base}#{at}/$id");
    let Some(id) = id.as_str() else {
        return Err(SchemaError::InvalidIdentifier {
            location: location(),
            reason: "`$id` must be a string".to_owned(),
        });
    };
    let resolved = resolve(base, id)?;
    match resolved.split_once('#') {
        None => Ok((resolved, None)),
        Some((uri, "")) => Ok((uri.to_owned(), None)),
        Some((uri, fragment)) if dialect == Dialect::Draft07 => {
            // Draft-07 Core §8.2.3: a plain-name fragment is a
            // location-independent identifier; any other fragment has no
            // defined meaning.
            let name = percent::decode(fragment)
                .ok()
                .map(std::borrow::Cow::into_owned)
                .filter(|name| dialect.is_anchor_name(name))
                .ok_or_else(|| SchemaError::InvalidIdentifier {
                    location: location(),
                    reason: format!("`$id` {id:?} has a fragment that is not a plain name"),
                })?;
            Ok((uri.to_owned(), Some(name)))
        }
        Some(_) => Err(SchemaError::InvalidIdentifier {
            location: location(),
            reason: format!("`$id` {id:?} may not carry a non-empty fragment"),
        }),
    }
}
