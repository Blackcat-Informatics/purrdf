// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The schema documents a compilation may reach, and how a URI finds a
//! schema in them (2020-12 Core §8.2 and §9, 2019-09 Core §8.2, draft-07
//! Core §8).
//!
//! A document is registered under a retrieval URI and scanned once, under the
//! rules of the dialect its `$schema` names. Every subschema that declares an
//! identifier becomes a *resource* with its own URI; every anchor is recorded
//! against the resource that contains it. The scan walks only the keywords the
//! dialect defines as holding subschemas, so an `$id` or `$anchor` inside
//! `enum`, `const`, `examples` or an unknown keyword is data, not an
//! identifier (2020-12 Core §9.4.2). The dialects differ in what an identifier
//! is:
//!
//! * **2020-12**: `$id` (no fragment), `$anchor` and `$dynamicAnchor`;
//! * **2019-09**: `$id` (no fragment), `$anchor`, and `$recursiveAnchor` on a
//!   resource root;
//! * **draft-07**: `$id`, whose plain-name fragment is an anchor; and an
//!   object holding `$ref` is that reference alone, so its sibling `$id` and
//!   subschemas identify nothing.
//!
//! A document whose `$schema` names a custom meta-schema is scanned under the
//! dialect that meta-schema is written in. Registered before its meta-schema,
//! it waits, and is scanned the moment the meta-schema arrives.
//!
//! The draft 2020-12, draft 2019-09 and draft-07 meta-schemas are registered in
//! every [`Registry`] from copies vendored into this crate, so no compilation
//! ever needs the network.

use std::collections::BTreeMap;
use std::fmt;

use serde_json::Value;

use crate::error::SchemaError;
use crate::pointer;

/// The draft 2020-12 meta-schema URI.
pub const DRAFT_2020_12: &str = "https://json-schema.org/draft/2020-12/schema";

/// The draft 2019-09 meta-schema URI.
pub const DRAFT_2019_09: &str = "https://json-schema.org/draft/2019-09/schema";

/// The draft-07 meta-schema URI (declared with or without the empty fragment,
/// `http://json-schema.org/draft-07/schema#`).
pub const DRAFT_07: &str = "http://json-schema.org/draft-07/schema";

/// `(URI, document)` for every vendored meta-schema and vocabulary
/// meta-schema.
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
    (
        DRAFT_2019_09,
        include_str!("../metaschemas/draft2019-09/schema.json"),
    ),
    (
        "https://json-schema.org/draft/2019-09/meta/core",
        include_str!("../metaschemas/draft2019-09/meta/core.json"),
    ),
    (
        "https://json-schema.org/draft/2019-09/meta/applicator",
        include_str!("../metaschemas/draft2019-09/meta/applicator.json"),
    ),
    (
        "https://json-schema.org/draft/2019-09/meta/validation",
        include_str!("../metaschemas/draft2019-09/meta/validation.json"),
    ),
    (
        "https://json-schema.org/draft/2019-09/meta/meta-data",
        include_str!("../metaschemas/draft2019-09/meta/meta-data.json"),
    ),
    (
        "https://json-schema.org/draft/2019-09/meta/format",
        include_str!("../metaschemas/draft2019-09/meta/format.json"),
    ),
    (
        "https://json-schema.org/draft/2019-09/meta/content",
        include_str!("../metaschemas/draft2019-09/meta/content.json"),
    ),
    (
        DRAFT_07,
        include_str!("../metaschemas/draft-07/schema.json"),
    ),
];

/// The meta-schema URIs of the published JSON Schema dialects this crate does
/// not implement. A document declaring one is recorded but never scanned: its
/// keywords (`id`, boolean `exclusiveMaximum`, …) mean something else, and
/// reading them under another dialect's rules would invent identifiers.
/// Referring to such a document is [`SchemaError::UnsupportedDialect`].
const OTHER_DIALECTS: &[&str] = &[
    "http://json-schema.org/schema",
    "http://json-schema.org/draft-03/schema",
    "http://json-schema.org/draft-04/schema",
    "http://json-schema.org/draft-05/schema",
    "http://json-schema.org/draft-06/schema",
    "https://json-schema.org/schema",
    "https://json-schema.org/draft-03/schema",
    "https://json-schema.org/draft-04/schema",
    "https://json-schema.org/draft-06/schema",
    "https://json-schema.org/draft-07/schema",
    "https://json-schema.org/draft/next/schema",
    "https://json-schema.org/v1",
];

/// A JSON Schema dialect this crate implements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Dialect {
    Draft07,
    Draft2019_09,
    Draft2020_12,
}

impl Dialect {
    /// The dialect a meta-schema URI (without its empty fragment) names.
    fn from_metaschema(bare: &str) -> Option<Self> {
        match bare {
            DRAFT_2020_12 => Some(Self::Draft2020_12),
            DRAFT_2019_09 => Some(Self::Draft2019_09),
            DRAFT_07 => Some(Self::Draft07),
            _ => None,
        }
    }

    /// The dialect's own meta-schema.
    pub(crate) const fn metaschema(self) -> &'static str {
        match self {
            Self::Draft07 => DRAFT_07,
            Self::Draft2019_09 => DRAFT_2019_09,
            Self::Draft2020_12 => DRAFT_2020_12,
        }
    }

    /// Where the dialect's vocabulary URIs live; draft-07 has no vocabularies.
    const fn vocabulary_prefix(self) -> Option<&'static str> {
        match self {
            Self::Draft07 => None,
            Self::Draft2019_09 => Some("https://json-schema.org/draft/2019-09/vocab/"),
            Self::Draft2020_12 => Some("https://json-schema.org/draft/2020-12/vocab/"),
        }
    }

    /// The flags a vocabulary of this dialect sets, `None` when the name is
    /// not one of the dialect's vocabularies. `required` is the vocabulary's
    /// `$vocabulary` value: the 2019-09 Format vocabulary asserts only when
    /// required (2019-09 Validation §7.2.1), while the 2020-12
    /// Format-Assertion vocabulary asserts whenever it is declared.
    fn vocabulary(self, name: &str, required: bool) -> Option<u8> {
        let flag = |vocabulary: Vocabulary| Some(vocabulary as u8);
        match (self, name) {
            (Self::Draft07, _) => None,
            (Self::Draft2020_12, "core" | "format-annotation" | "meta-data" | "content")
            | (Self::Draft2019_09, "core" | "meta-data" | "content") => Some(0),
            (Self::Draft2020_12, "applicator") => flag(Vocabulary::Applicator),
            (Self::Draft2020_12, "unevaluated") => flag(Vocabulary::Unevaluated),
            (Self::Draft2020_12, "format-assertion") => flag(Vocabulary::FormatAssertion),
            // 2019-09 has no Unevaluated vocabulary: its unevaluated keywords
            // belong to the Applicator vocabulary.
            (Self::Draft2019_09, "applicator") => {
                Some(Vocabulary::Applicator as u8 | Vocabulary::Unevaluated as u8)
            }
            (Self::Draft2019_09, "format") => Some(if required {
                Vocabulary::FormatAssertion as u8
            } else {
                0
            }),
            (_, "validation") => flag(Vocabulary::Validation),
            _ => None,
        }
    }

    /// The vocabularies of the dialect's own meta-schema.
    const fn default_vocabularies(self) -> Vocabularies {
        Vocabularies {
            dialect: self,
            flags: Vocabulary::Applicator as u8
                | Vocabulary::Unevaluated as u8
                | Vocabulary::Validation as u8,
        }
    }

    /// Whether `name` is an anchor name: 2020-12's `anchorString`
    /// `^[A-Za-z_][-A-Za-z0-9._]*$`; 2019-09's `$anchor` pattern and the
    /// draft-07 plain-name fragment, `^[A-Za-z][-A-Za-z0-9.:_]*$`.
    fn is_anchor_name(self, name: &str) -> bool {
        let mut bytes = name.bytes();
        let Some(first) = bytes.next() else {
            return false;
        };
        match self {
            Self::Draft2020_12 => {
                (first.is_ascii_alphabetic() || first == b'_')
                    && bytes.all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_')
                    })
            }
            Self::Draft2019_09 | Self::Draft07 => {
                first.is_ascii_alphabetic()
                    && bytes.all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b':' | b'_')
                    })
            }
        }
    }

    /// The anchor-name pattern [`Self::is_anchor_name`] implements.
    const fn anchor_pattern(self) -> &'static str {
        match self {
            Self::Draft2020_12 => "^[A-Za-z_][-A-Za-z0-9._]*$",
            Self::Draft2019_09 | Self::Draft07 => "^[A-Za-z][-A-Za-z0-9.:_]*$",
        }
    }

    /// Keywords whose value is an object of subschemas.
    const fn schema_maps(self) -> &'static [&'static str] {
        match self {
            Self::Draft07 => &[
                "definitions",
                "properties",
                "patternProperties",
                "dependencies",
            ],
            Self::Draft2019_09 | Self::Draft2020_12 => &[
                "$defs",
                "definitions",
                "properties",
                "patternProperties",
                "dependentSchemas",
                "dependencies",
            ],
        }
    }

    /// Keywords whose value is an array of subschemas.
    const fn schema_arrays(self) -> &'static [&'static str] {
        match self {
            Self::Draft07 | Self::Draft2019_09 => &["allOf", "anyOf", "oneOf", "items"],
            Self::Draft2020_12 => &["allOf", "anyOf", "oneOf", "prefixItems"],
        }
    }

    /// Keywords whose value is one subschema.
    const fn schema_singles(self) -> &'static [&'static str] {
        match self {
            Self::Draft07 => &[
                "additionalItems",
                "additionalProperties",
                "propertyNames",
                "items",
                "contains",
                "not",
                "if",
                "then",
                "else",
            ],
            Self::Draft2019_09 => &[
                "additionalItems",
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
            ],
            Self::Draft2020_12 => &[
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
            ],
        }
    }
}

/// A vocabulary whose keywords assert or apply. The Core, Meta-Data,
/// Format-Annotation and Content vocabularies need no flag: their keywords
/// are identifiers or annotations whichever vocabularies are in force.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Vocabulary {
    Applicator = 1,
    Unevaluated = 2,
    Validation = 4,
    FormatAssertion = 8,
}

/// The dialect a schema resource is evaluated under, and which of its
/// vocabularies are in force.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Vocabularies {
    pub(crate) dialect: Dialect,
    flags: u8,
}

impl Vocabularies {
    /// Whether `vocabulary` is in force.
    pub(crate) const fn has(self, vocabulary: Vocabulary) -> bool {
        self.flags & vocabulary as u8 != 0
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

/// A schema resource: a document root or a subschema with an identifier.
#[derive(Debug, Clone)]
pub(crate) struct Resource {
    pub(crate) uri: String,
    pub(crate) doc: usize,
    /// The resource root's pointer within its document.
    pub(crate) pointer: String,
    parent: Option<usize>,
    /// The `$schema` its root declares, if any.
    schema: Option<String>,
    /// The dialect its identifiers were scanned under.
    pub(crate) dialect: Dialect,
    /// The `$schema` of an embedded resource declaring a dialect this crate
    /// does not implement; its contents were not scanned.
    foreign: Option<String>,
    /// Whether its root declares `"$recursiveAnchor": true` (2019-09).
    pub(crate) recursive_anchor: bool,
    anchors: BTreeMap<String, String>,
    pub(crate) dynamic_anchors: BTreeMap<String, String>,
}

/// A location in a registered document: document index and pointer.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Location {
    pub(crate) doc: usize,
    pub(crate) pointer: String,
}

/// A document whose `$schema` (or an embedded resource's) names a custom
/// meta-schema not yet registered.
#[derive(Debug, Clone)]
struct Pending {
    value: Value,
    builtin: bool,
    /// The dialect of its root when it declares none.
    inherited: Dialect,
    /// The meta-schema URI it waits for.
    awaiting: String,
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
    pending: BTreeMap<String, Pending>,
    /// Documents that waited for a meta-schema and failed their scan when it
    /// arrived: the failure is reported to whatever compiles them.
    broken: BTreeMap<String, SchemaError>,
    locations: BTreeMap<(usize, String), usize>,
    /// The dialect of a document that declares no `$schema`.
    default_dialect: Dialect,
}

impl fmt::Debug for Registry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Registry")
            .field(
                "documents",
                &self.docs.iter().map(|doc| &doc.uri).collect::<Vec<_>>(),
            )
            .field("resources", &self.by_uri.keys().collect::<Vec<_>>())
            .field("default_dialect", &self.default_dialect.metaschema())
            .field("other_dialects", &self.refused)
            .field(
                "awaiting_metaschema",
                &self
                    .pending
                    .iter()
                    .map(|(uri, pending)| (uri, &pending.awaiting))
                    .collect::<Vec<_>>(),
            )
            .finish_non_exhaustive()
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

/// What a declared `$schema` makes of the document declaring it.
enum Lookup {
    /// Scan and compile it under this dialect.
    Known(Dialect),
    /// It is another published dialect: record it, refuse to use it.
    Other(String),
    /// It names a meta-schema not registered yet: wait for it.
    Unknown(String),
}

/// Why a scan stopped.
enum ScanFailure {
    Error(SchemaError),
    /// An embedded resource names a meta-schema not registered yet.
    Awaiting(String),
}

impl From<SchemaError> for ScanFailure {
    fn from(error: SchemaError) -> Self {
        Self::Error(error)
    }
}

impl Registry {
    /// A registry holding the draft 2020-12, draft 2019-09 and draft-07
    /// meta-schemas.
    pub fn new() -> Self {
        let mut registry = Self {
            docs: Vec::new(),
            resources: Vec::new(),
            by_uri: BTreeMap::new(),
            refused: BTreeMap::new(),
            pending: BTreeMap::new(),
            broken: BTreeMap::new(),
            locations: BTreeMap::new(),
            default_dialect: Dialect::Draft2020_12,
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
    /// The document is scanned for identifiers and anchors immediately, under
    /// the dialect its `$schema` names, so a malformed identifier, or a URI
    /// another resource already claims, is refused here. A document declaring
    /// a dialect this crate does not implement is recorded without being
    /// scanned; compiling anything that reaches it is
    /// [`SchemaError::UnsupportedDialect`]. A document whose `$schema` names a
    /// custom meta-schema that is not registered yet is scanned when that
    /// meta-schema is registered.
    pub fn add_resource(&mut self, uri: &str, document: Value) -> Result<(), SchemaError> {
        self.insert(uri, document, false)
    }

    /// Set the dialect of documents registered from now on that declare no
    /// `$schema`, by its meta-schema URI: [`DRAFT_2020_12`] (the default),
    /// [`DRAFT_2019_09`] or [`DRAFT_07`], with or without the empty fragment.
    /// Any other URI is [`SchemaError::UnsupportedDialect`], and leaves the
    /// default as it was.
    pub fn set_default_dialect(&mut self, metaschema: &str) -> Result<(), SchemaError> {
        let bare = metaschema.strip_suffix('#').unwrap_or(metaschema);
        let dialect =
            Dialect::from_metaschema(bare).ok_or_else(|| SchemaError::UnsupportedDialect {
                resource: "the registry's default dialect".to_owned(),
                dialect: metaschema.to_owned(),
            })?;
        self.default_dialect = dialect;
        Ok(())
    }

    fn claimed(&self, uri: &str) -> bool {
        self.by_uri.contains_key(uri)
            || self.refused.contains_key(uri)
            || self.pending.contains_key(uri)
            || self.broken.contains_key(uri)
    }

    fn insert(&mut self, uri: &str, document: Value, builtin: bool) -> Result<(), SchemaError> {
        let uri = absolute_without_fragment(uri)?;
        if self.claimed(&uri) {
            return Err(SchemaError::DuplicateResource { uri });
        }
        self.admit(uri, document, builtin, self.default_dialect)?;
        self.settle();
        Ok(())
    }

    /// What `declared` (a `$schema` value, or its absence under a resource of
    /// dialect `inherited`) makes of a resource.
    fn lookup(&self, declared: Option<&str>, inherited: Dialect) -> Lookup {
        let Some(declared) = declared else {
            return Lookup::Known(inherited);
        };
        let bare = declared.strip_suffix('#').unwrap_or(declared);
        if let Some(dialect) = Dialect::from_metaschema(bare) {
            return Lookup::Known(dialect);
        }
        if OTHER_DIALECTS.contains(&bare) || self.refused.contains_key(bare) {
            return Lookup::Other(declared.to_owned());
        }
        match self.by_uri.get(bare) {
            Some(&meta) => Lookup::Known(self.resources[meta].dialect),
            None => Lookup::Unknown(bare.to_owned()),
        }
    }

    /// Scan and commit one document, or record why it cannot be scanned yet.
    fn admit(
        &mut self,
        uri: String,
        document: Value,
        builtin: bool,
        inherited: Dialect,
    ) -> Result<(), SchemaError> {
        let dialect = match self.lookup(declared_schema(&document).as_deref(), inherited) {
            Lookup::Known(dialect) => dialect,
            Lookup::Other(dialect) => {
                self.refused.insert(uri, dialect);
                return Ok(());
            }
            Lookup::Unknown(awaiting) => {
                self.pending.insert(
                    uri,
                    Pending {
                        value: document,
                        builtin,
                        inherited,
                        awaiting,
                    },
                );
                return Ok(());
            }
        };
        let doc = self.docs.len();
        let staged = match Scan::run(self, doc, &uri, &document, dialect) {
            Ok(staged) => staged,
            Err(ScanFailure::Error(error)) => return Err(error),
            Err(ScanFailure::Awaiting(awaiting)) => {
                self.pending.insert(
                    uri,
                    Pending {
                        value: document,
                        builtin,
                        inherited,
                        awaiting,
                    },
                );
                return Ok(());
            }
        };
        let root_resource = staged.offset;
        for claimed in staged.by_uri.keys() {
            if self.claimed(claimed) {
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

    /// Scan every waiting document whose meta-schema has arrived, until none
    /// is left that can be.
    fn settle(&mut self) {
        loop {
            let ready = self.pending.iter().find_map(|(uri, pending)| {
                (!matches!(
                    self.lookup(Some(&pending.awaiting), Dialect::Draft2020_12),
                    Lookup::Unknown(_)
                ))
                .then(|| uri.clone())
            });
            let Some(uri) = ready else {
                return;
            };
            let Some(pending) = self.pending.remove(&uri) else {
                return;
            };
            if let Err(error) = self.admit(
                uri.clone(),
                pending.value,
                pending.builtin,
                pending.inherited,
            ) {
                self.broken.insert(uri, error);
            }
        }
    }

    /// Find the schema an absolute URI (with or without a fragment) names.
    pub(crate) fn locate(&self, absolute: &str) -> Result<Location, Locate> {
        let (base, fragment) = match absolute.split_once('#') {
            Some((base, fragment)) => (base, Some(fragment)),
            None => (absolute, None),
        };
        let Some(&resource) = self.by_uri.get(base) else {
            if let Some(dialect) = self.refused.get(base) {
                return Err(Locate::OtherDialect(dialect.clone()));
            }
            if let Some(pending) = self.pending.get(base) {
                return Err(Locate::OtherDialect(pending.awaiting.clone()));
            }
            if let Some(error) = self.broken.get(base) {
                return Err(Locate::Broken(error.clone()));
            }
            return Err(Locate::Unknown);
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

    /// The location of a resource's root.
    pub(crate) fn root_of(&self, resource: usize) -> Location {
        let resource = &self.resources[resource];
        Location {
            doc: resource.doc,
            pointer: resource.pointer.clone(),
        }
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

    /// The dialect and vocabularies `resource` is evaluated under, from the
    /// `$schema` its root declares or the nearest enclosing resource's.
    pub(crate) fn vocabularies(&self, resource: usize) -> Result<Vocabularies, SchemaError> {
        self.vocabularies_at(resource, 0)
    }

    fn vocabularies_at(&self, resource: usize, depth: usize) -> Result<Vocabularies, SchemaError> {
        let mut current = resource;
        let declared = loop {
            let entry = &self.resources[current];
            if let Some(foreign) = &entry.foreign {
                return Err(SchemaError::UnsupportedDialect {
                    resource: entry.uri.clone(),
                    dialect: foreign.clone(),
                });
            }
            if let Some(schema) = &entry.schema {
                break schema.clone();
            }
            match entry.parent {
                Some(parent) => current = parent,
                None => return Ok(self.resources[resource].dialect.default_vocabularies()),
            }
        };
        let refuse = || SchemaError::UnsupportedDialect {
            resource: self.resources[resource].uri.clone(),
            dialect: declared.clone(),
        };
        let metaschema_uri = declared.strip_suffix('#').unwrap_or(&declared);
        if let Some(dialect) = Dialect::from_metaschema(metaschema_uri) {
            return Ok(dialect.default_vocabularies());
        }
        // A custom meta-schema must itself be a schema of an implemented
        // dialect: follow its own `$schema` chain, which ends at one of the
        // dialect meta-schemas or is refused.
        let Some(&meta) = self.by_uri.get(metaschema_uri) else {
            return Err(refuse());
        };
        if depth > 32 {
            return Err(refuse());
        }
        let dialect = self
            .vocabularies_at(meta, depth + 1)
            .map_err(|_| refuse())?
            .dialect;
        let Some(prefix) = dialect.vocabulary_prefix() else {
            return Ok(dialect.default_vocabularies());
        };
        let root = self.value(&self.root_of(meta)).ok_or_else(refuse)?;
        let Some(declared_vocabularies) = root.get("$vocabulary") else {
            return Ok(dialect.default_vocabularies());
        };
        let Some(declared_vocabularies) = declared_vocabularies.as_object() else {
            return Err(SchemaError::InvalidKeyword {
                location: format!("{metaschema_uri}#/$vocabulary"),
                reason: "`$vocabulary` must be an object".to_owned(),
            });
        };
        let mut flags = 0;
        for (vocabulary, required) in declared_vocabularies {
            let required = required.as_bool() != Some(false);
            let known = vocabulary
                .strip_prefix(prefix)
                .and_then(|name| dialect.vocabulary(name, required));
            match known {
                Some(flag) => flags |= flag,
                None if required => {
                    return Err(SchemaError::UnsupportedVocabulary {
                        metaschema: metaschema_uri.to_owned(),
                        vocabulary: vocabulary.clone(),
                    });
                }
                None => {}
            }
        }
        Ok(Vocabularies { dialect, flags })
    }

    /// The meta-schema URI a document's root declares, or its dialect's.
    pub(crate) fn document_dialect(&self, doc: usize) -> String {
        declared_schema(&self.docs[doc].value).map_or_else(
            || {
                self.resources[self.docs[doc].root_resource]
                    .dialect
                    .metaschema()
                    .to_owned()
            },
            |schema| schema.strip_suffix('#').unwrap_or(&schema).to_owned(),
        )
    }
}

/// Why [`Registry::locate`] found nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Locate {
    /// No registered resource, anchor or pointer target has that URI.
    Unknown,
    /// The URI names a document declaring this dialect, which this crate does
    /// not implement or whose meta-schema is not registered.
    OtherDialect(String),
    /// The URI names a document that could not be scanned.
    Broken(SchemaError),
}

fn declared_schema(document: &Value) -> Option<String> {
    document
        .get("$schema")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
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

/// What an object's `$id` identifies.
struct Identifier {
    /// The absolute URI without its fragment.
    uri: String,
    /// A draft-07 plain-name fragment: an anchor.
    anchor: Option<String>,
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
        dialect: Dialect,
    ) -> Result<Self, ScanFailure> {
        let mut scan = Self {
            resources: Vec::new(),
            by_uri: BTreeMap::new(),
            locations: BTreeMap::new(),
            offset: registry.resources.len(),
            doc,
        };
        let identifier = Self::identifier(retrieval, root, "", dialect)?;
        let uri = identifier
            .as_ref()
            .map_or_else(|| retrieval.to_owned(), |identifier| identifier.uri.clone());
        let resource = scan.add_resource(
            uri.clone(),
            String::new(),
            None,
            root,
            dialect,
            declared_schema(root),
            None,
        )?;
        if uri != retrieval {
            scan.claim(retrieval.to_owned(), resource)?;
        }
        if let Some(anchor) = identifier.and_then(|identifier| identifier.anchor) {
            scan.anchor(resource, &anchor, "", false)?;
        }
        let mut stack = vec![(String::new(), root, resource, dialect)];
        while let Some((pointer, value, resource, dialect)) = stack.pop() {
            scan.locations.insert((doc, pointer.clone()), resource);
            let Some(object) = value.as_object() else {
                continue;
            };
            // Draft-07 Core §8.3: every other keyword of an object holding
            // `$ref` is ignored, so it declares nothing.
            if dialect == Dialect::Draft07 && object.contains_key("$ref") {
                continue;
            }
            scan.anchors(object, &pointer, resource, dialect)?;
            for (child_pointer, child) in Self::children(object, &pointer, dialect) {
                let (child_resource, child_dialect) =
                    scan.child_resource(registry, resource, dialect, &child_pointer, child)?;
                if scan.resource(child_resource).foreign.is_some() {
                    scan.locations.insert((doc, child_pointer), child_resource);
                    continue;
                }
                stack.push((child_pointer, child, child_resource, child_dialect));
            }
        }
        Ok(scan)
    }

    /// The subschemas directly beneath a schema object, by the dialect's
    /// applicator keywords.
    fn children<'v>(
        object: &'v serde_json::Map<String, Value>,
        pointer: &str,
        dialect: Dialect,
    ) -> Vec<(String, &'v Value)> {
        let mut children: Vec<(String, &Value)> = Vec::new();
        for &keyword in dialect.schema_maps() {
            if let Some(Value::Object(map)) = object.get(keyword) {
                let here = pointer::push_token(pointer, keyword);
                for (name, child) in map {
                    children.push((pointer::push_token(&here, name), child));
                }
            }
        }
        for &keyword in dialect.schema_arrays() {
            if let Some(Value::Array(items)) = object.get(keyword) {
                let here = pointer::push_token(pointer, keyword);
                for (index, child) in items.iter().enumerate() {
                    children.push((pointer::push_token(&here, &index.to_string()), child));
                }
            }
        }
        for &keyword in dialect.schema_singles() {
            if let Some(child) = object.get(keyword) {
                children.push((pointer::push_token(pointer, keyword), child));
            }
        }
        children.retain(|(_, child)| child.is_object() || child.is_boolean());
        children
    }

    /// The resource a subschema belongs to: a new one when it declares an
    /// identifier, else its parent's.
    fn child_resource(
        &mut self,
        registry: &Registry,
        parent: usize,
        dialect: Dialect,
        child_pointer: &str,
        child: &Value,
    ) -> Result<(usize, Dialect), ScanFailure> {
        let base = self.resource(parent).uri.clone();
        let Some(identifier) = Self::identifier(&base, child, child_pointer, dialect)? else {
            return Ok((parent, dialect));
        };
        if dialect == Dialect::Draft07 && identifier.uri == base {
            // `"$id": "#name"`: an anchor in the enclosing resource.
            if let Some(anchor) = &identifier.anchor {
                self.anchor(parent, anchor, child_pointer, false)?;
            }
            return Ok((parent, dialect));
        }
        // Draft-07 forbids `$schema` below the document root; 2019-09 and
        // 2020-12 let an embedded resource declare its own dialect.
        let declared = if dialect == Dialect::Draft07 {
            None
        } else {
            declared_schema(child)
        };
        let (child_dialect, foreign) = match registry.lookup(declared.as_deref(), dialect) {
            Lookup::Known(child_dialect) => (child_dialect, None),
            Lookup::Other(other) => (dialect, Some(other)),
            Lookup::Unknown(awaiting) => return Err(ScanFailure::Awaiting(awaiting)),
        };
        let resource = self.add_resource(
            identifier.uri,
            child_pointer.to_owned(),
            Some(parent),
            child,
            child_dialect,
            declared,
            foreign,
        )?;
        if let Some(anchor) = &identifier.anchor {
            self.anchor(resource, anchor, child_pointer, false)?;
        }
        Ok((resource, child_dialect))
    }

    fn resource(&self, index: usize) -> &Resource {
        &self.resources[index - self.offset]
    }

    fn resource_mut(&mut self, index: usize) -> &mut Resource {
        &mut self.resources[index - self.offset]
    }

    /// The identifier an object's `$id` declares, resolved against `base`.
    fn identifier(
        base: &str,
        value: &Value,
        at: &str,
        dialect: Dialect,
    ) -> Result<Option<Identifier>, SchemaError> {
        let Some(object) = value.as_object() else {
            return Ok(None);
        };
        let Some(id) = object.get("$id") else {
            return Ok(None);
        };
        if dialect == Dialect::Draft07 && object.contains_key("$ref") {
            return Ok(None);
        }
        let location = || format!("{base}#{at}/$id");
        let Some(id) = id.as_str() else {
            return Err(SchemaError::InvalidIdentifier {
                location: location(),
                reason: "`$id` must be a string".to_owned(),
            });
        };
        let resolved = resolve(base, id)?;
        let (uri, fragment) = match resolved.split_once('#') {
            Some((uri, fragment)) => (uri.to_owned(), fragment),
            None => (resolved.clone(), ""),
        };
        if fragment.is_empty() {
            return Ok(Some(Identifier { uri, anchor: None }));
        }
        if dialect != Dialect::Draft07 {
            return Err(SchemaError::InvalidIdentifier {
                location: location(),
                reason: format!("`$id` {id:?} may not carry a non-empty fragment"),
            });
        }
        let fragment = pointer::percent_decode(fragment).unwrap_or_default();
        if fragment.starts_with('/') {
            // A JSON Pointer fragment restates a location the pointer already
            // reaches; only the URI part identifies anything.
            return Ok(Some(Identifier { uri, anchor: None }));
        }
        if !dialect.is_anchor_name(&fragment) {
            return Err(SchemaError::InvalidIdentifier {
                location: location(),
                reason: format!(
                    "the fragment of `$id` {id:?} is neither empty, a JSON Pointer nor a \
                     plain name (`{}`)",
                    dialect.anchor_pattern()
                ),
            });
        }
        Ok(Some(Identifier {
            uri,
            anchor: Some(fragment),
        }))
    }

    #[allow(clippy::too_many_arguments)]
    fn add_resource(
        &mut self,
        uri: String,
        pointer: String,
        parent: Option<usize>,
        root: &Value,
        dialect: Dialect,
        schema: Option<String>,
        foreign: Option<String>,
    ) -> Result<usize, SchemaError> {
        let index = self.offset + self.resources.len();
        self.claim(uri.clone(), index)?;
        let recursive_anchor = dialect == Dialect::Draft2019_09
            && root.get("$recursiveAnchor") == Some(&Value::Bool(true));
        self.resources.push(Resource {
            uri,
            doc: self.doc,
            pointer,
            parent,
            schema,
            dialect,
            foreign,
            recursive_anchor,
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

    /// The `$anchor` (and, in 2020-12, `$dynamicAnchor`) of one object.
    fn anchors(
        &mut self,
        object: &serde_json::Map<String, Value>,
        pointer: &str,
        resource: usize,
        dialect: Dialect,
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
                    reason: format!(
                        "{anchor} is not an anchor name (`{}`)",
                        dialect.anchor_pattern()
                    ),
                });
            };
            self.anchor(resource, name, pointer, dynamic)?;
        }
        Ok(())
    }

    fn anchor(
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
}
