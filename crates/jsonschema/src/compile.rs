// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Compiling a registered schema into a [`Schema`].
//!
//! Subschemas are compiled on demand from a work queue, each location once:
//! a `$ref` cycle is an edge back to an index already allocated, never a
//! recursion. Every schema resource a compiled subschema belongs to has its
//! `$dynamicAnchor`s compiled too, because the dynamic scope of an evaluation
//! may reach them from any `$dynamicRef`.
//!
//! After the keywords compile, every non-built-in document the schema reaches
//! is validated against its own meta-schema; a document that fails is
//! refused, never half-used.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use serde_json::{Map, Value};

use crate::ecma;
use crate::error::SchemaError;
use crate::format::Format;
use crate::number::Decimal;
use crate::pointer;
use crate::registry::{DRAFT_2020_12, Locate, Location, Registry, Vocabularies, Vocabulary};
use crate::schema::{Body, JsonType, Keyword, Kind, Node, NodeId, Pattern, Schema};

/// Compile the schema at the absolute URI `uri` (a fragment may select a
/// subschema by JSON Pointer or anchor).
pub(crate) fn compile(registry: &Registry, uri: &str) -> Result<Schema, SchemaError> {
    let (schema, documents) = compile_unchecked(registry, uri)?;
    for doc in documents {
        check_against_metaschema(registry, doc)?;
    }
    Ok(schema)
}

fn compile_unchecked(
    registry: &Registry,
    uri: &str,
) -> Result<(Schema, BTreeSet<usize>), SchemaError> {
    let parsed = purrdf_iri::parse(uri).map_err(|error| SchemaError::InvalidUri {
        uri: uri.to_owned(),
        reason: error.to_string(),
    })?;
    if !parsed.has_scheme() {
        return Err(SchemaError::InvalidUri {
            uri: uri.to_owned(),
            reason: "a schema is compiled from an absolute URI".to_owned(),
        });
    }
    let location = registry
        .locate(uri)
        .map_err(|failure| located(failure, uri, uri, uri))?;
    let mut compiler = Compiler {
        registry,
        nodes: Vec::new(),
        memo: BTreeMap::new(),
        queue: Vec::new(),
        resources: Vec::new(),
        resource_index: BTreeMap::new(),
        vocabularies: BTreeMap::new(),
        documents: BTreeSet::new(),
    };
    let root = compiler.node(location);
    while let Some((id, location)) = compiler.queue.pop() {
        compiler.build(id, &location)?;
    }
    let schema = Schema {
        nodes: compiler.nodes,
        resources: compiler.resources,
        root,
    };
    Ok((schema, compiler.documents))
}

fn located(failure: Locate, at: &str, reference: &str, resolved: &str) -> SchemaError {
    match failure {
        Locate::Unknown => SchemaError::UnresolvedReference {
            location: at.to_owned(),
            reference: reference.to_owned(),
            resolved: resolved.to_owned(),
        },
        Locate::OtherDialect(dialect) => SchemaError::UnsupportedDialect {
            resource: resolved.split('#').next().unwrap_or(resolved).to_owned(),
            dialect,
        },
    }
}

/// The draft 2020-12 meta-schema, compiled once per process.
fn metaschema() -> &'static Schema {
    static META: OnceLock<Schema> = OnceLock::new();
    META.get_or_init(|| {
        compile_unchecked(&Registry::new(), DRAFT_2020_12).map_or_else(
            |error| unreachable!("the vendored meta-schema compiles: {error}"),
            |(schema, _)| schema,
        )
    })
}

fn check_against_metaschema(registry: &Registry, doc: usize) -> Result<(), SchemaError> {
    let document = &registry.docs[doc];
    if document.builtin {
        return Ok(());
    }
    let dialect = registry.document_dialect(doc);
    let custom;
    let meta = if dialect == DRAFT_2020_12 {
        metaschema()
    } else {
        custom = compile(registry, &dialect)?;
        &custom
    };
    let output = meta.evaluate(&document.value);
    if output.is_valid() {
        return Ok(());
    }
    Err(SchemaError::InvalidSchema {
        uri: document.uri.clone(),
        errors: output
            .errors()
            .map(|unit| {
                (
                    unit.keyword_location.clone(),
                    unit.instance_location.clone(),
                    unit.error.clone().unwrap_or_default(),
                )
            })
            .collect(),
    })
}

struct Compiler<'r> {
    registry: &'r Registry,
    nodes: Vec<Node>,
    memo: BTreeMap<Location, NodeId>,
    queue: Vec<(NodeId, Location)>,
    resources: Vec<BTreeMap<String, NodeId>>,
    resource_index: BTreeMap<usize, usize>,
    vocabularies: BTreeMap<usize, Vocabularies>,
    documents: BTreeSet<usize>,
}

/// Where a keyword is, for diagnostics.
fn at(location: &str, keyword: &str) -> String {
    format!(
        "{location}/{}",
        pointer::fragment_encode(&pointer::escape_token(keyword))
    )
}

fn invalid(location: &str, keyword: &str, reason: &str) -> SchemaError {
    SchemaError::InvalidKeyword {
        location: at(location, keyword),
        reason: reason.to_owned(),
    }
}

impl Compiler<'_> {
    /// The node for `location`, allocating and queueing it on first sight.
    fn node(&mut self, location: Location) -> NodeId {
        if let Some(&id) = self.memo.get(&location) {
            return id;
        }
        let id = self.nodes.len();
        self.nodes.push(Node {
            location: String::new(),
            resource: 0,
            body: Body::Bool(true),
        });
        self.memo.insert(location.clone(), id);
        self.queue.push((id, location));
        id
    }

    fn child(&mut self, parent: &Location, tokens: &[&str]) -> NodeId {
        let mut pointer = parent.pointer.clone();
        for token in tokens {
            pointer = pointer::push_token(&pointer, token);
        }
        self.node(Location {
            doc: parent.doc,
            pointer,
        })
    }

    /// The compiled resource for a registry resource, compiling its dynamic
    /// anchors the first time it is seen.
    fn resource(&mut self, registry_resource: usize) -> usize {
        if let Some(&index) = self.resource_index.get(&registry_resource) {
            return index;
        }
        let index = self.resources.len();
        self.resources.push(BTreeMap::new());
        self.resource_index.insert(registry_resource, index);
        let anchors: Vec<String> = self.registry.resources[registry_resource]
            .dynamic_anchors
            .keys()
            .cloned()
            .collect();
        for name in anchors {
            if let Some(location) = self.registry.dynamic_anchor(registry_resource, &name) {
                let node = self.node(location);
                self.resources[index].insert(name, node);
            }
        }
        index
    }

    fn vocabularies(&mut self, registry_resource: usize) -> Result<Vocabularies, SchemaError> {
        if let Some(&vocabularies) = self.vocabularies.get(&registry_resource) {
            return Ok(vocabularies);
        }
        let vocabularies = self.registry.vocabularies(registry_resource)?;
        self.vocabularies.insert(registry_resource, vocabularies);
        Ok(vocabularies)
    }

    fn build(&mut self, id: NodeId, location: &Location) -> Result<(), SchemaError> {
        let registry = self.registry;
        let registry_resource = registry.resource_of(location);
        let resource = &registry.resources[registry_resource];
        let relative = location
            .pointer
            .strip_prefix(resource.pointer.as_str())
            .unwrap_or(&location.pointer);
        let absolute = format!("{}#{}", resource.uri, pointer::fragment_encode(relative));
        let compiled_resource = self.resource(registry_resource);
        self.documents.insert(location.doc);
        let vocabularies = self.vocabularies(registry_resource)?;
        let Some(value) = registry.value(location) else {
            return Err(SchemaError::UnresolvedReference {
                location: absolute.clone(),
                reference: absolute.clone(),
                resolved: absolute,
            });
        };
        let body = match value {
            Value::Bool(flag) => Body::Bool(*flag),
            Value::Object(map) => {
                let context = Context {
                    base: &resource.uri,
                    absolute: &absolute,
                    location,
                    vocabularies,
                };
                self.keywords(&context, map)?
            }
            _ => {
                return Err(SchemaError::InvalidKeyword {
                    location: absolute,
                    reason: "a schema must be an object or a boolean".to_owned(),
                });
            }
        };
        self.nodes[id] = Node {
            location: absolute,
            resource: compiled_resource,
            body,
        };
        Ok(())
    }

    fn reference(
        &mut self,
        context: &Context<'_>,
        keyword: &str,
        value: &Value,
    ) -> Result<(NodeId, String), SchemaError> {
        let Some(reference) = value.as_str() else {
            return Err(invalid(context.absolute, keyword, "must be a string"));
        };
        let resolved = crate::registry::resolve(context.base, reference)?;
        let target = self.registry.locate(&resolved).map_err(|failure| {
            located(
                failure,
                &at(context.absolute, keyword),
                reference,
                &resolved,
            )
        })?;
        Ok((self.node(target), resolved))
    }

    fn schemas(
        &mut self,
        context: &Context<'_>,
        keyword: &str,
        value: &Value,
    ) -> Result<Vec<NodeId>, SchemaError> {
        let Some(items) = value.as_array().filter(|items| !items.is_empty()) else {
            return Err(invalid(
                context.absolute,
                keyword,
                "must be a non-empty array of schemas",
            ));
        };
        Ok((0..items.len())
            .map(|index| self.child(context.location, &[keyword, &index.to_string()]))
            .collect())
    }

    fn schema_map(
        &mut self,
        context: &Context<'_>,
        keyword: &str,
        value: &Value,
    ) -> Result<Vec<(String, NodeId)>, SchemaError> {
        let Some(map) = value.as_object() else {
            return Err(invalid(
                context.absolute,
                keyword,
                "must be an object of schemas",
            ));
        };
        Ok(map
            .keys()
            .map(|name| (name.clone(), self.child(context.location, &[keyword, name])))
            .collect())
    }

    fn pattern(context: &Context<'_>, keyword: &str, source: &str) -> Result<Pattern, SchemaError> {
        ecma::compile(source)
            .map(|regex| Pattern {
                source: source.to_owned(),
                regex,
            })
            .map_err(|error| SchemaError::Pattern {
                location: at(context.absolute, keyword),
                pattern: source.to_owned(),
                error,
            })
    }

    #[allow(clippy::too_many_lines)]
    fn keywords(
        &mut self,
        context: &Context<'_>,
        map: &Map<String, Value>,
    ) -> Result<Body, SchemaError> {
        let vocabularies = context.vocabularies;
        let mut references = Vec::new();
        let mut assertions = Vec::new();
        let mut in_place = Vec::new();
        let mut children = Vec::new();
        let mut annotations = Vec::new();
        let mut unevaluated = Vec::new();
        for (name, value) in map {
            let keyword = name.as_str();
            let kind = match keyword {
                "$ref" => {
                    let (target, _) = self.reference(context, keyword, value)?;
                    references.push(Keyword {
                        name: name.clone(),
                        kind: Kind::Ref(target),
                    });
                    continue;
                }
                "$dynamicRef" => {
                    let (target, resolved) = self.reference(context, keyword, value)?;
                    let anchor = resolved.split_once('#').and_then(|(base, fragment)| {
                        let name = pointer::percent_decode(fragment)?;
                        if name.is_empty() || name.starts_with('/') {
                            return None;
                        }
                        let resource = self.registry.resource_by_uri(base)?;
                        self.registry.dynamic_anchor(resource, &name).map(|_| name)
                    });
                    references.push(Keyword {
                        name: name.clone(),
                        kind: Kind::DynamicRef { target, anchor },
                    });
                    continue;
                }
                "$id" | "$schema" | "$anchor" | "$dynamicAnchor" | "$vocabulary" | "$comment"
                | "$defs" | "definitions" => continue,
                _ => keyword,
            };
            let validation = vocabularies.has(Vocabulary::Validation);
            let applicator = vocabularies.has(Vocabulary::Applicator);
            let bucket_kind: Option<(&mut Vec<Keyword>, Kind)> = match kind {
                "type" if validation => Some((&mut assertions, Kind::Type(types(context, value)?))),
                "enum" if validation => {
                    let Some(values) = value.as_array() else {
                        return Err(invalid(context.absolute, keyword, "must be an array"));
                    };
                    Some((&mut assertions, Kind::Enum(values.clone())))
                }
                "const" if validation => Some((&mut assertions, Kind::Const(value.clone()))),
                "multipleOf" if validation => {
                    let number = number(context, keyword, value)?;
                    if !number.is_positive() {
                        return Err(invalid(context.absolute, keyword, "must be greater than 0"));
                    }
                    Some((&mut assertions, Kind::MultipleOf(number, value.clone())))
                }
                "maximum" if validation => Some((
                    &mut assertions,
                    Kind::Maximum(number(context, keyword, value)?, value.clone()),
                )),
                "exclusiveMaximum" if validation => Some((
                    &mut assertions,
                    Kind::ExclusiveMaximum(number(context, keyword, value)?, value.clone()),
                )),
                "minimum" if validation => Some((
                    &mut assertions,
                    Kind::Minimum(number(context, keyword, value)?, value.clone()),
                )),
                "exclusiveMinimum" if validation => Some((
                    &mut assertions,
                    Kind::ExclusiveMinimum(number(context, keyword, value)?, value.clone()),
                )),
                "maxLength" if validation => Some((
                    &mut assertions,
                    Kind::MaxLength(count(context, keyword, value)?),
                )),
                "minLength" if validation => Some((
                    &mut assertions,
                    Kind::MinLength(count(context, keyword, value)?),
                )),
                "pattern" if validation => {
                    let Some(source) = value.as_str() else {
                        return Err(invalid(context.absolute, keyword, "must be a string"));
                    };
                    Some((
                        &mut assertions,
                        Kind::Pattern(Box::new(Self::pattern(context, keyword, source)?)),
                    ))
                }
                "maxItems" if validation => Some((
                    &mut assertions,
                    Kind::MaxItems(count(context, keyword, value)?),
                )),
                "minItems" if validation => Some((
                    &mut assertions,
                    Kind::MinItems(count(context, keyword, value)?),
                )),
                "uniqueItems" if validation => match value.as_bool() {
                    Some(true) => Some((&mut assertions, Kind::UniqueItems)),
                    Some(false) => None,
                    None => return Err(invalid(context.absolute, keyword, "must be a boolean")),
                },
                "maxProperties" if validation => Some((
                    &mut assertions,
                    Kind::MaxProperties(count(context, keyword, value)?),
                )),
                "minProperties" if validation => Some((
                    &mut assertions,
                    Kind::MinProperties(count(context, keyword, value)?),
                )),
                "required" if validation => Some((
                    &mut assertions,
                    Kind::Required(strings(context, keyword, value)?),
                )),
                "dependentRequired" if validation => Some((
                    &mut assertions,
                    Kind::DependentRequired(dependent_required(context, keyword, value)?),
                )),
                "maxContains" | "minContains" if validation => {
                    count(context, keyword, value)?;
                    None
                }
                "format" => {
                    let Some(format) = value.as_str() else {
                        return Err(invalid(context.absolute, keyword, "must be a string"));
                    };
                    let check = if vocabularies.has(Vocabulary::FormatAssertion) {
                        Some(Format::from_name(format).ok_or_else(|| {
                            SchemaError::UnsupportedFormat {
                                location: at(context.absolute, keyword),
                                format: format.to_owned(),
                            }
                        })?)
                    } else {
                        None
                    };
                    Some((&mut assertions, Kind::Format(format.to_owned(), check)))
                }
                "allOf" if applicator => Some((
                    &mut in_place,
                    Kind::AllOf(self.schemas(context, keyword, value)?),
                )),
                "anyOf" if applicator => Some((
                    &mut in_place,
                    Kind::AnyOf(self.schemas(context, keyword, value)?),
                )),
                "oneOf" if applicator => Some((
                    &mut in_place,
                    Kind::OneOf(self.schemas(context, keyword, value)?),
                )),
                "not" if applicator => Some((
                    &mut in_place,
                    Kind::Not(self.child(context.location, &[keyword])),
                )),
                "if" if applicator => {
                    let condition = self.child(context.location, &["if"]);
                    let then = map
                        .contains_key("then")
                        .then(|| self.child(context.location, &["then"]));
                    let otherwise = map
                        .contains_key("else")
                        .then(|| self.child(context.location, &["else"]));
                    Some((
                        &mut in_place,
                        Kind::If {
                            condition,
                            then,
                            otherwise,
                        },
                    ))
                }
                "then" | "else" if applicator => None,
                "dependentSchemas" if applicator => Some((
                    &mut in_place,
                    Kind::DependentSchemas(self.schema_map(context, keyword, value)?),
                )),
                "dependencies" => {
                    // The pre-2019-09 keyword the 2020-12 meta-schema still
                    // describes: array members are `dependentRequired`, schema
                    // members `dependentSchemas`.
                    let Some(members) = value.as_object() else {
                        return Err(invalid(context.absolute, keyword, "must be an object"));
                    };
                    let mut required = Vec::new();
                    let mut schemas = Vec::new();
                    for (property, member) in members {
                        if member.is_array() {
                            if validation {
                                required
                                    .push((property.clone(), strings(context, keyword, member)?));
                            }
                        } else if applicator {
                            schemas.push((
                                property.clone(),
                                self.child(context.location, &[keyword, property]),
                            ));
                        }
                    }
                    if !required.is_empty() {
                        assertions.push(Keyword {
                            name: name.clone(),
                            kind: Kind::DependentRequired(required),
                        });
                    }
                    if !schemas.is_empty() {
                        in_place.push(Keyword {
                            name: name.clone(),
                            kind: Kind::DependentSchemas(schemas),
                        });
                    }
                    None
                }
                "prefixItems" if applicator => Some((
                    &mut children,
                    Kind::PrefixItems(self.schemas(context, keyword, value)?),
                )),
                "items" if applicator => {
                    let skip = map
                        .get("prefixItems")
                        .and_then(Value::as_array)
                        .map_or(0, Vec::len);
                    Some((
                        &mut children,
                        Kind::Items {
                            schema: self.child(context.location, &[keyword]),
                            skip,
                        },
                    ))
                }
                "contains" if applicator => {
                    let bound = |name: &str| -> Result<Option<u64>, SchemaError> {
                        if !validation {
                            return Ok(None);
                        }
                        map.get(name)
                            .map(|value| count(context, name, value))
                            .transpose()
                    };
                    let min = bound("minContains")?.unwrap_or(1);
                    let max = bound("maxContains")?;
                    Some((
                        &mut children,
                        Kind::Contains {
                            schema: self.child(context.location, &[keyword]),
                            min,
                            max,
                        },
                    ))
                }
                "properties" if applicator => {
                    let members = self.schema_map(context, keyword, value)?;
                    Some((
                        &mut children,
                        Kind::Properties(members.into_iter().collect()),
                    ))
                }
                "patternProperties" if applicator => {
                    let members = self.schema_map(context, keyword, value)?;
                    let mut compiled = Vec::with_capacity(members.len());
                    for (source, node) in members {
                        compiled.push((Self::pattern(context, keyword, &source)?, node));
                    }
                    Some((&mut children, Kind::PatternProperties(compiled)))
                }
                "additionalProperties" if applicator => {
                    let properties = map
                        .get("properties")
                        .and_then(Value::as_object)
                        .map(|members| members.keys().cloned().collect())
                        .unwrap_or_default();
                    let mut patterns = Vec::new();
                    if let Some(members) = map.get("patternProperties").and_then(Value::as_object) {
                        for source in members.keys() {
                            patterns
                                .push(Self::pattern(context, "patternProperties", source)?.regex);
                        }
                    }
                    Some((
                        &mut children,
                        Kind::AdditionalProperties {
                            schema: self.child(context.location, &[keyword]),
                            properties,
                            patterns,
                        },
                    ))
                }
                "propertyNames" if applicator => Some((
                    &mut children,
                    Kind::PropertyNames(self.child(context.location, &[keyword])),
                )),
                "unevaluatedItems" if vocabularies.has(Vocabulary::Unevaluated) => Some((
                    &mut unevaluated,
                    Kind::UnevaluatedItems(self.child(context.location, &[keyword])),
                )),
                "unevaluatedProperties" if vocabularies.has(Vocabulary::Unevaluated) => Some((
                    &mut unevaluated,
                    Kind::UnevaluatedProperties(self.child(context.location, &[keyword])),
                )),
                // Meta-data and content keywords, keywords of a vocabulary the
                // meta-schema does not declare, and unknown keywords: each is an
                // annotation carrying its value (Core §6.5, §7.7.1).
                _ => Some((&mut annotations, Kind::Annotation(value.clone()))),
            };
            if let Some((bucket, kind)) = bucket_kind {
                bucket.push(Keyword {
                    name: name.clone(),
                    kind,
                });
            }
        }
        // `unevaluatedItems` is ordered before `unevaluatedProperties` only for
        // determinism; they read disjoint annotations.
        unevaluated.sort_by(|a, b| a.name.cmp(&b.name));
        let tracks = !unevaluated.is_empty();
        let mut keywords = references;
        keywords.append(&mut assertions);
        keywords.append(&mut in_place);
        keywords.append(&mut children);
        keywords.append(&mut annotations);
        keywords.append(&mut unevaluated);
        Ok(Body::Keywords { keywords, tracks })
    }
}

/// What a keyword compiles against.
struct Context<'a> {
    base: &'a str,
    absolute: &'a str,
    location: &'a Location,
    vocabularies: Vocabularies,
}

fn number(context: &Context<'_>, keyword: &str, value: &Value) -> Result<Decimal, SchemaError> {
    value
        .as_number()
        .map(Decimal::from_number)
        .ok_or_else(|| invalid(context.absolute, keyword, "must be a number"))
}

fn count(context: &Context<'_>, keyword: &str, value: &Value) -> Result<u64, SchemaError> {
    value
        .as_number()
        .and_then(|number| Decimal::from_number(number).to_u64_saturating())
        .ok_or_else(|| invalid(context.absolute, keyword, "must be a non-negative integer"))
}

fn strings(
    context: &Context<'_>,
    keyword: &str,
    value: &Value,
) -> Result<Vec<String>, SchemaError> {
    value
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .map(|item| item.as_str().map(ToOwned::to_owned))
                .collect::<Option<Vec<_>>>()
        })
        .ok_or_else(|| invalid(context.absolute, keyword, "must be an array of strings"))
}

fn dependent_required(
    context: &Context<'_>,
    keyword: &str,
    value: &Value,
) -> Result<Vec<(String, Vec<String>)>, SchemaError> {
    let Some(members) = value.as_object() else {
        return Err(invalid(
            context.absolute,
            keyword,
            "must be an object of string arrays",
        ));
    };
    members
        .iter()
        .map(|(property, required)| Ok((property.clone(), strings(context, keyword, required)?)))
        .collect()
}

fn types(context: &Context<'_>, value: &Value) -> Result<Vec<JsonType>, SchemaError> {
    let names: Vec<&Value> = match value {
        Value::Array(items) => items.iter().collect(),
        single => vec![single],
    };
    names
        .into_iter()
        .map(|name| {
            name.as_str()
                .and_then(JsonType::parse)
                .ok_or_else(|| invalid(context.absolute, "type", "must name JSON Schema types"))
        })
        .collect()
}
