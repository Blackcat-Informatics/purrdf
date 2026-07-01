// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SHACL → JSON Schema (draft 2020-12) + OpenAPI 3.1 emitter (#700).
//!
//! Compiles a parsed [`Shapes`] graph into a closed-world JSON Schema describing
//! the JSON-LD projection of PurRDF instance data (see [`crate::instance`]). The
//! emitter and the projector share ONE CURIE-compaction / value-shaping
//! convention so a projected node always validates against the schema this
//! module produces (Task 6 proves the round trip over every slice example).
//!
//! # Conventions (must stay in lock-step with `instance.rs`)
//!
//! * **IRI compaction** — [`compact_iri`] maps a known namespace prefix to
//!   `prefix:LocalName`; otherwise the full IRI is kept verbatim.
//! * **Object (node) value** — a JSON object `{"@id": "<compacted-iri>"}`.
//! * **Typed literal value** — `{"@value": "<lexical>", "@type": "<compacted-datatype>"}`.
//!   For numeric / boolean datatypes the projector MAY also emit a bare JSON
//!   scalar, so the value schema accepts BOTH the scalar and the object form
//!   (`anyOf`).
//! * **Language-tagged literal** — `{"@value": "<lexical>", "@language": "<tag>"}`.
//! * **Plain string** — a bare JSON string.
//! * **Statement metadata** — an optional `@annotation` key on any property value
//!   object, referencing `#/$defs/Annotation` (RDF-1.2 reifier metadata, #699).
//!
//! # SPARQL losses
//!
//! `sh:sparql` / `sh:SPARQLTarget` constraints have no JSON Schema equivalent.
//! They are never silently skipped: each one is dropped, recorded as a
//! [`LossRecord`], and annotated with a `$comment` on the affected schema.

use std::collections::BTreeSet;

use serde_json::{json, Map, Value};

use crate::shapes::{Constraint, NodeKindValue, Path, Shape, Shapes, Target};
use crate::term::Term;

/// The PurRDF namespace (matches `crate::model::purrdf`).
const PURRDF_NS: &str = "https://blackcatinformatics.ca/purrdf/";
const LOGIC_NS: &str = "https://blackcatinformatics.ca/logic/";
const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema#";
const RDF_NS: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const RDFS_NS: &str = "http://www.w3.org/2000/01/rdf-schema#";
const OWL_NS: &str = "http://www.w3.org/2002/07/owl#";
const SH_NS: &str = "http://www.w3.org/ns/shacl#";
/// The two datatype IRIs whose literals project as a bare JSON string (no alloc
/// per literal — see [`crate::instance`] for the matching projection convention).
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";

/// The well-known prefix map, highest-specificity-first so e.g. the purrdf
/// namespace is matched before any shorter prefix could.  `(prefix, namespace)`.
pub const PREFIXES: &[(&str, &str)] = &[
    ("purrdf", PURRDF_NS),
    ("logic", LOGIC_NS),
    ("xsd", XSD_NS),
    ("rdf", RDF_NS),
    ("rdfs", RDFS_NS),
    ("owl", OWL_NS),
    ("sh", SH_NS),
];

/// Compact an IRI to `prefix:LocalName` when it begins with a known namespace;
/// otherwise return the full IRI unchanged.
///
/// This is the single shared compaction helper used by BOTH the schema emitter
/// and the instance projector ([`crate::instance`]).
pub fn compact_iri(iri: &str) -> String {
    for (prefix, ns) in PREFIXES {
        if let Some(local) = iri.strip_prefix(ns) {
            return format!("{prefix}:{local}");
        }
    }
    iri.to_owned()
}

/// The bare local name of an IRI: the substring after the last `#` or `/`.
pub fn local_name(iri: &str) -> String {
    let after_hash = iri.rsplit('#').next().unwrap_or(iri);
    // `rsplit('#')` returns the whole string when there is no `#`, so split on
    // `/` over that remainder.
    let local = after_hash.rsplit('/').next().unwrap_or(after_hash);
    local.to_owned()
}

/// Whether an IRI is in the PurRDF namespace (object refs to purrdf classes get a
/// `$ref`; external classes get a permissive node-ref / string).
fn is_purrdf(iri: &str) -> bool {
    iri.starts_with(PURRDF_NS)
}

/// Whether an IRI is in a known namespace (i.e. [`compact_iri`] would compact it
/// to a `prefix:Local` CURIE rather than returning it verbatim).
fn is_known_prefix(iri: &str) -> bool {
    compact_iri(iri) != iri
}

/// The `$defs`/discriminator key for a target class. A `purrdf:` class keeps its
/// bare local name (the historical keying — no schema-golden churn); any other
/// known-namespace class is keyed by its full CURIE (`logic:FormalizationCandidate`),
/// so cross-namespace local-name twins never collide. A `purrdf:` local name never
/// contains a `:`, while a CURIE always does — the discriminator
/// ([`node_def`]) relies on that to rebuild each `@type` const.
fn def_key(iri: &str) -> String {
    if is_purrdf(iri) {
        local_name(iri)
    } else {
        compact_iri(iri)
    }
}

/// A single un-mappable SHACL construct, recorded rather than silently dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LossRecord {
    /// The SHACL construct that could not be mapped (e.g. `"sh:sparql"`).
    pub construct: String,
    /// The IRI (or blank-node id) of the shape that carried it.
    pub shape_iri: String,
    /// A human-readable reason for the drop.
    pub reason: String,
}

/// The compiled artifacts: a JSON Schema document, an OpenAPI document, and the
/// list of constructs that could not be expressed.
#[derive(Debug, Clone)]
pub struct CompiledSchema {
    /// The JSON Schema (draft 2020-12), pretty-printed with a trailing newline.
    pub schema_json: String,
    /// The OpenAPI 3.1 document embedding the same `$defs`, same convention.
    pub openapi_json: String,
    /// Every dropped, un-mappable construct (never silently skipped).
    pub losses: Vec<LossRecord>,
}

// ── Compilation context ──────────────────────────────────────────────────────

/// Accumulates losses while compiling so every emitter helper can record one.
struct Ctx {
    losses: Vec<LossRecord>,
    /// The set of class local-names that WILL receive a `$def` — i.e. every
    /// `LocalName(target_class)` over all non-deactivated `Target::Class(..)`
    /// shapes. An object property's `sh:class C` may only emit a
    /// `#/$defs/<LocalName(C)>` ref when `LocalName(C)` is in this set;
    /// otherwise the ref would dangle (no shape ⇒ no `$def`).
    emitted_defs: BTreeSet<String>,
}

impl Ctx {
    fn new(emitted_defs: BTreeSet<String>) -> Self {
        Self {
            losses: Vec::new(),
            emitted_defs,
        }
    }

    fn record(&mut self, construct: &str, shape_iri: &str, reason: &str) {
        self.losses.push(LossRecord {
            construct: construct.to_owned(),
            shape_iri: shape_iri.to_owned(),
            reason: reason.to_owned(),
        });
    }
}

// ── Public entry points ──────────────────────────────────────────────────────

/// Compile a parsed [`Shapes`] graph into a closed-world JSON Schema + OpenAPI.
pub fn compile(shapes: &Shapes) -> CompiledSchema {
    // Keying invariant (#700 Gap D, fail-closed): every `$def` is keyed by the
    // class LOCAL NAME and the `@type` discriminator is `purrdf:<LocalName>`. That
    // is sound ONLY while every target class is in the purrdf namespace and no two
    // distinct class IRIs share a local name. Local-name keys are deliberate — a
    // colon-bearing compact IRI (`purrdf:Event`) is not a valid OpenAPI
    // `components/schemas` key (`^[a-zA-Z0-9._-]+$`) — so this guard protects the
    // precondition rather than widening the keys. The whole corpus satisfies it
    // today; a future non-purrdf or colliding target class HARD-fails the build
    // here instead of silently mis-discriminating or clobbering a `$def`.
    assert_target_class_keys_are_unambiguous(shapes);

    // PASS 1: compute the set of class local-names that WILL receive a `$def`,
    // using the EXACT same iteration that builds the `$defs` map below (every
    // `Target::Class(..)` of every non-deactivated node shape). This lets the
    // per-property emitter decide whether a `sh:class C` ref can resolve before
    // the `$defs` map is fully built, so it never writes a dangling `$ref`.
    let mut emitted_defs: BTreeSet<String> = BTreeSet::new();
    for shape in &shapes.node_shapes {
        if shape.deactivated {
            continue;
        }
        for target in &shape.targets {
            if let Target::Class(c) = target {
                emitted_defs.insert(local_name(c.as_str()));
            }
        }
    }

    let mut ctx = Ctx::new(emitted_defs);

    // Build $defs: one entry per `sh:targetClass` of every active node shape,
    // keyed by the class local name; the body is the shape compiled as an object
    // schema.  Multiple target classes on one shape reuse the same body.
    let mut defs: Map<String, Value> = Map::new();
    for shape in &shapes.node_shapes {
        if shape.deactivated {
            continue;
        }
        let body = compile_object_schema(shape, &mut ctx);
        for target in &shape.targets {
            if let Target::Class(c) = target {
                let name = def_key(c.as_str());
                // First writer wins for a given class name; bodies are identical
                // per shape so this only matters if two shapes target the same
                // class (last one would otherwise clobber). Keep deterministic by
                // not overwriting an existing identical-by-construction entry.
                defs.entry(name).or_insert_with(|| body.clone());
            }
        }
    }

    // The shared statement-metadata fragment (#699).
    defs.insert("Annotation".to_owned(), annotation_def());

    let class_names: Vec<String> = defs
        .keys()
        .filter(|k| *k != "Annotation")
        .cloned()
        .collect();
    // `class_names` is already sorted because `defs` is a BTree-ordered Map iter.

    // The `@type`-discriminated `Node` schema (#700 closed-world enforcement):
    // a node typed `purrdf:Foo` MUST satisfy `#/$defs/Foo`. Inserted AFTER
    // `class_names` is snapshotted so `Node` itself is never treated as a class
    // branch.
    defs.insert("Node".to_owned(), node_def(&class_names));

    let schema = root_schema(&defs);
    let openapi = openapi_doc(&defs);

    CompiledSchema {
        schema_json: to_pretty(&schema),
        openapi_json: to_pretty(&openapi),
        losses: ctx.losses,
    }
}

/// Enforce the keying precondition (#700 Gap D): every active `sh:targetClass`
/// is in a KNOWN namespace (so [`def_key`] yields a stable `$defs` key and
/// [`node_def`] can rebuild its `@type` const) and those keys are collision-free.
/// Panics with a descriptive message otherwise (build-time, fail-closed).
fn assert_target_class_keys_are_unambiguous(shapes: &Shapes) {
    use std::collections::BTreeMap;
    let mut key_to_iri: BTreeMap<String, String> = BTreeMap::new();
    for shape in &shapes.node_shapes {
        if shape.deactivated {
            continue;
        }
        for target in &shape.targets {
            if let Target::Class(c) = target {
                let iri = c.as_str();
                assert!(
                    is_known_prefix(iri),
                    "json_schema: unknown-namespace sh:targetClass {iri:?} — the @type \
                     discriminator and `$defs` keys derive from a known prefix CURIE; \
                     register the namespace in PREFIXES (and confirm OpenAPI key encoding) \
                     before introducing target classes from a new namespace"
                );
                let key = def_key(iri);
                if let Some(prev) = key_to_iri.insert(key.clone(), iri.to_owned()) {
                    assert_eq!(
                        prev, iri,
                        "json_schema: distinct target classes share the `$defs` key \
                         {key:?} ({prev} vs {iri}) — their `$defs`/OpenAPI keys would \
                         collide; disambiguate before keying"
                    );
                }
            }
        }
    }
}

// ── Root envelope ────────────────────────────────────────────────────────────

/// Build the top-level JSON Schema envelope.
///
/// Every instance node — whether a `@graph` member or a bare single-node root —
/// is validated by the single `#/$defs/Node` schema, which discriminates on
/// `@type` (closed-world enforcement, #700).
fn root_schema(defs: &Map<String, Value>) -> Value {
    let node_ref = json!({ "$ref": "#/$defs/Node" });

    // The @graph envelope object: every member is a discriminated Node. The
    // envelope branch REQUIRES `@graph`, so a bare single-node document cannot
    // slip through this permissive branch and escape `Node` discrimination — a
    // bare node must satisfy the `node_ref` branch of the root `anyOf` instead
    // (closed-world: a bare incomplete node is rejected, #700).
    let graph_envelope = json!({
        "type": "object",
        "required": ["@graph"],
        "properties": {
            "@context": true,
            "@graph": {
                "type": "array",
                "items": node_ref
            }
        }
    });

    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://blackcatinformatics.ca/purrdf/schema/instance.schema.json",
        "title": "PURRDF instance schema (SHACL-derived, closed-world)",
        "$defs": Value::Object(defs.clone()),
        "type": "object",
        "anyOf": [graph_envelope, node_ref],
        "properties": {
            "@context": true,
            "@graph": {
                "type": "array",
                "items": node_ref
            }
        }
    })
}

/// Build the `@type`-discriminated `Node` schema (#700).
///
/// A node carries `@id`/`@type`/`@annotation` permissively, then an `allOf` of
/// per-class conditionals (sorted by class name for determinism). Each entry
/// reads: *if* `@type` includes the class CURIE — `purrdf:<Class>` for a purrdf
/// class, or the full `prefix:<Class>` for any other known namespace (e.g.
/// `logic:FormalizationCandidate`) — as a bare string OR an array member,
/// *then* the node MUST satisfy that class's `#/$defs` body.
///
/// Closed-world semantics:
/// * An instance typed `purrdf:Foo` that is MISSING a required property triggers
///   Foo's `then` (`#/$defs/Foo`), fails Foo's `required`, and is REJECTED.
/// * A node typed only by an UNMODELED class (no `$def`) fires no `if`, so no
///   `then` applies and it stays permissively allowed — keeping the slice
///   example sweep (Task 6) green on unmodeled types.
fn node_def(class_names: &[String]) -> Value {
    // class_names arrives sorted (BTree-ordered defs iter); keep it explicit so
    // the conditional list is deterministic regardless of caller.
    let mut sorted: Vec<&String> = class_names.iter().collect();
    sorted.sort();

    let conditionals: Vec<Value> = sorted
        .iter()
        .map(|name| {
            // A `$defs` key carrying a `:` is already a CURIE (a non-purrdf class,
            // e.g. `logic:FormalizationCandidate`); a colon-free key is a bare
            // purrdf local name and takes the `purrdf:` prefix. Either way the
            // `@type` const matches the compact IRI an instance node carries.
            let type_const = if name.contains(':') {
                (*name).clone()
            } else {
                format!("purrdf:{name}")
            };
            json!({
                "if": {
                    "required": ["@type"],
                    "properties": {
                        "@type": {
                            "anyOf": [
                                { "const": type_const },
                                { "type": "array", "contains": { "const": type_const } }
                            ]
                        }
                    }
                },
                "then": { "$ref": format!("#/$defs/{name}") }
            })
        })
        .collect();

    json!({
        "type": "object",
        "title": "A single discriminated PURRDF instance node",
        "description": "Validated by @type: a node typed purrdf:Foo MUST satisfy #/$defs/Foo (closed-world, #700). Nodes typed only by unmodeled classes are permissively allowed.",
        "properties": {
            "@id": { "type": "string" },
            "@type": {
                "anyOf": [
                    { "type": "string" },
                    { "type": "array", "items": { "type": "string" } }
                ]
            },
            "@annotation": { "$ref": "#/$defs/Annotation" }
        },
        "allOf": conditionals
    })
}

/// The OpenAPI 3.1 document embedding the same `$defs` as `components/schemas`.
fn openapi_doc(defs: &Map<String, Value>) -> Value {
    json!({
        "openapi": "3.1.0",
        "info": {
            "title": "PURRDF",
            "version": crate::VERSION
        },
        "paths": {
            "/entities/{id}": {
                "get": {
                    "summary": "Fetch a single PURRDF entity by id",
                    "parameters": [{
                        "name": "id",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    }],
                    "responses": {
                        "200": {
                            "description": "The requested entity as a JSON-LD node.",
                            "content": {
                                "application/ld+json": {
                                    "schema": { "type": "object" }
                                }
                            }
                        }
                    }
                }
            }
        },
        "components": { "schemas": Value::Object(defs.clone()) }
    })
}

// ── The `@annotation` fragment (#699 statement metadata) ─────────────────────

/// The shared `$defs/Annotation` object schema: free-form statement metadata.
///
/// Permissive on purpose — #699 tightens it. Values may be node refs
/// (`{"@id":..}`), scalars, or typed literals (`{"@value":..,"@type":..}`).
fn annotation_def() -> Value {
    json!({
        "type": "object",
        "title": "RDF-1.2 statement metadata (reifier annotation)",
        "description": "Free-form metadata about an asserted triple (e.g. purrdf:accordingTo, purrdf:confidence, purrdf:assertedAt). Permissive; tightened by #699.",
        "additionalProperties": {
            "anyOf": [
                { "type": "string" },
                { "type": "number" },
                { "type": "boolean" },
                node_ref_schema(),
                typed_literal_schema()
            ]
        }
    })
}

/// The JSON-LD node-reference value schema: `{"@id": "<string>"}`.
fn node_ref_schema() -> Value {
    json!({
        "type": "object",
        "properties": { "@id": { "type": "string" } },
        "required": ["@id"]
    })
}

/// The JSON-LD typed-literal value schema: `{"@value":.., "@type":..}`.
fn typed_literal_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "@value": {},
            "@type": { "type": "string" }
        },
        "required": ["@value"]
    })
}

// ── Per-shape object schema ──────────────────────────────────────────────────

/// Compile a single node shape into a JSON Schema object schema (one `$defs`
/// body). Property shapes become `properties`; node-level logical/closed
/// constraints become `allOf`/`anyOf`/`oneOf`/`not`/`additionalProperties`.
fn compile_object_schema(shape: &Shape, ctx: &mut Ctx) -> Value {
    let shape_iri = shape.id.to_string();

    let mut properties: Map<String, Value> = Map::new();
    let mut required: Vec<String> = Vec::new();
    let mut comments: Vec<String> = Vec::new();

    // `@id` and `@type` are always allowed JSON-LD keywords.
    properties.insert("@id".to_owned(), json!({ "type": "string" }));
    properties.insert(
        "@type".to_owned(),
        json!({
            "anyOf": [
                { "type": "string" },
                { "type": "array", "items": { "type": "string" } }
            ]
        }),
    );

    // The optional statement-metadata key on the node itself.
    properties.insert(
        "@annotation".to_owned(),
        json!({ "$ref": "#/$defs/Annotation" }),
    );

    // Group property-shape constraints by JSON key. A node shape may carry
    // SEVERAL `sh:property` blocks for the SAME path (e.g. one `sh:minCount 1`
    // and one `sh:maxCount 1`), and their blank-node ids are randomly minted by
    // the RDF store — so iterating `property_shapes` directly and inserting per
    // shape would (a) be NON-DETERMINISTIC (last writer wins on a random order)
    // and (b) DROP one block's constraints. Merging the constraint lists per key
    // and compiling once is both order-independent and semantically complete.
    // `BTreeMap` keeps the emitted property order deterministic (sorted keys).
    let mut by_key: std::collections::BTreeMap<String, Vec<Constraint>> =
        std::collections::BTreeMap::new();
    for ps in &shape.property_shapes {
        // Inverse paths do not shape outgoing JSON properties: skip but note it.
        let pred = match &ps.path {
            Path::Predicate(p) => p,
            Path::Inverse(_) => {
                comments.push(
                    "an inverse-path property shape was skipped (inverse paths do not constrain outgoing JSON properties)".to_owned(),
                );
                continue;
            }
        };
        let key = compact_iri(pred.as_str());
        by_key
            .entry(key)
            .or_default()
            .extend(ps.constraints.iter().cloned());
    }

    for (key, constraints) in &by_key {
        let (value_schema, is_required) = compile_property(constraints, &shape_iri, key, ctx);
        if is_required {
            required.push(key.clone());
        }
        properties.insert(key.clone(), value_schema);
    }

    // ── Node-level constraints ──
    let mut all_of: Vec<Value> = Vec::new();
    let mut any_of: Vec<Value> = Vec::new();
    let mut one_of: Vec<Value> = Vec::new();
    let mut not_schema: Option<Value> = None;
    let mut additional_properties_false = false;
    let mut closed_ignored: Vec<String> = Vec::new();

    for c in &shape.constraints {
        match c {
            Constraint::And(members) => {
                for m in members {
                    all_of.push(compile_object_schema(m, ctx));
                }
            }
            Constraint::Or(members) => {
                for m in members {
                    any_of.push(compile_object_schema(m, ctx));
                }
            }
            Constraint::Xone(members) => {
                for m in members {
                    one_of.push(compile_object_schema(m, ctx));
                }
            }
            Constraint::Node(inner) => {
                all_of.push(compile_object_schema(inner, ctx));
            }
            Constraint::Not(inner) => {
                not_schema = Some(compile_object_schema(inner, ctx));
            }
            Constraint::Closed { ignored } => {
                additional_properties_false = true;
                for n in ignored {
                    closed_ignored.push(compact_iri(n.as_str()));
                }
            }
            Constraint::Sparql { .. } => {
                ctx.record(
                    "sh:sparql",
                    &shape_iri,
                    "SPARQL-AF constraint has no JSON Schema equivalent",
                );
                comments.push(
                    "a node-level sh:sparql constraint was dropped (no JSON Schema equivalent)"
                        .to_owned(),
                );
            }
            // Node-level value constraints (sh:class, sh:nodeKind, …) shape the
            // node identity rather than an object's JSON properties; for the
            // object-schema projection they are not expressed here.
            _ => {}
        }
    }

    // sh:closed: allow the ignored predicates as declared keys too.
    if additional_properties_false {
        for k in &closed_ignored {
            properties
                .entry(k.clone())
                .or_insert_with(|| Value::Bool(true));
        }
    }

    // Assemble.
    let mut obj: Map<String, Value> = Map::new();
    obj.insert("type".to_owned(), json!("object"));

    obj.insert("properties".to_owned(), Value::Object(properties));

    if !required.is_empty() {
        required.sort();
        required.dedup();
        obj.insert(
            "required".to_owned(),
            Value::Array(required.into_iter().map(Value::String).collect()),
        );
    }

    if additional_properties_false {
        obj.insert("additionalProperties".to_owned(), Value::Bool(false));
    }

    if !all_of.is_empty() {
        obj.insert("allOf".to_owned(), Value::Array(all_of));
    }
    if !any_of.is_empty() {
        obj.insert("anyOf".to_owned(), Value::Array(any_of));
    }
    if !one_of.is_empty() {
        obj.insert("oneOf".to_owned(), Value::Array(one_of));
    }
    if let Some(ns) = not_schema {
        obj.insert("not".to_owned(), ns);
    }

    if !comments.is_empty() {
        comments.sort();
        comments.dedup();
        obj.insert("$comment".to_owned(), json!(comments.join("; ")));
    }

    Value::Object(obj)
}

// ── Per-property value schema ────────────────────────────────────────────────

/// Compile one property shape's constraints into `(value_schema, is_required)`.
///
/// `value_schema` already accounts for cardinality: a single value when
/// `sh:maxCount 1`, otherwise an `array` wrapper with `minItems`/`maxItems`.
fn compile_property(
    constraints: &[Constraint],
    shape_iri: &str,
    key: &str,
    ctx: &mut Ctx,
) -> (Value, bool) {
    // The "scalar" value schema (a single value, pre-cardinality).
    let mut value: Map<String, Value> = Map::new();
    // anyOf alternatives accumulated across datatype/class/nodekind constraints.
    let mut alts: Vec<Value> = Vec::new();
    let mut enum_values: Vec<Value> = Vec::new();
    let mut comments: Vec<String> = Vec::new();

    let mut min_count: Option<u64> = None;
    let mut max_count: Option<u64> = None;

    for c in constraints {
        match c {
            Constraint::MinCount(n) => min_count = Some(*n),
            Constraint::MaxCount(n) => max_count = Some(*n),
            Constraint::Datatype(dt) => {
                alts.push(datatype_value_schema(dt.as_str()));
            }
            Constraint::Class(c) => {
                if is_purrdf(c.as_str()) {
                    let name = local_name(c.as_str());
                    if ctx.emitted_defs.contains(&name) {
                        // The class has a NodeShape ⇒ a `$def` is emitted for it.
                        // Object property: a node ref OR the class `$ref`.
                        alts.push(node_ref_schema());
                        alts.push(json!({ "$ref": format!("#/$defs/{name}") }));
                    } else {
                        // The class has NO NodeShape ⇒ no `$def` is emitted, so a
                        // `$ref` to it would dangle and make the schema
                        // uncompilable. Closed-world correct behaviour: instances
                        // reference such nodes by `@id` only; the node simply is
                        // not further constrained here. Emit the node-reference
                        // form WITHOUT the `$ref` branch.
                        let mut node_ref = node_ref_schema();
                        if let Value::Object(map) = &mut node_ref {
                            map.insert(
                                "$comment".to_owned(),
                                json!(format!(
                                    "purrdf:{name} has no NodeShape; node reference only"
                                )),
                            );
                        }
                        alts.push(node_ref);
                    }
                } else {
                    alts.push(json!({
                        "type": "string",
                        "$comment": format!("external class {}", c.as_str())
                    }));
                }
            }
            Constraint::NodeKind(nk) => match nk {
                NodeKindValue::Literal => {
                    alts.push(json!({ "type": "string" }));
                    alts.push(typed_literal_schema());
                }
                NodeKindValue::Iri | NodeKindValue::BlankNode | NodeKindValue::BlankNodeOrIri => {
                    alts.push(node_ref_schema());
                }
                NodeKindValue::IriOrLiteral | NodeKindValue::BlankNodeOrLiteral => {
                    alts.push(node_ref_schema());
                    alts.push(json!({ "type": "string" }));
                    alts.push(typed_literal_schema());
                }
            },
            Constraint::In(terms) => {
                for t in terms {
                    enum_values.push(json!(term_enum_value(t)));
                }
            }
            Constraint::HasValue(v) => {
                value.insert("const".to_owned(), term_const_value(v));
            }
            Constraint::Pattern { regex, .. } => {
                value.insert("pattern".to_owned(), json!(regex));
            }
            Constraint::MinLength(n) => {
                value.insert("minLength".to_owned(), json!(n));
            }
            Constraint::MaxLength(n) => {
                value.insert("maxLength".to_owned(), json!(n));
            }
            Constraint::MinInclusive(t) => {
                insert_numeric(&mut value, "minimum", t, &mut comments);
            }
            Constraint::MaxInclusive(t) => {
                insert_numeric(&mut value, "maximum", t, &mut comments);
            }
            Constraint::MinExclusive(t) => {
                insert_numeric(&mut value, "exclusiveMinimum", t, &mut comments);
            }
            Constraint::MaxExclusive(t) => {
                insert_numeric(&mut value, "exclusiveMaximum", t, &mut comments);
            }
            Constraint::LanguageIn(tags) => {
                alts.push(lang_literal_schema(tags));
            }
            Constraint::Sparql { .. } => {
                ctx.record(
                    "sh:sparql",
                    shape_iri,
                    "SPARQL-AF constraint has no JSON Schema equivalent",
                );
                comments.push(format!(
                    "a sh:sparql constraint on property {key} was dropped (no JSON Schema equivalent)"
                ));
            }
            // Counts handled above; node-shape-only constraints (Closed/And/…)
            // do not appear on a property shape's value schema.
            _ => {}
        }
    }

    if !enum_values.is_empty() {
        enum_values.sort_by_key(ToString::to_string);
        enum_values.dedup();
        value.insert("enum".to_owned(), Value::Array(enum_values));
    }

    if !alts.is_empty() {
        // Stable order, de-duplicated.
        alts.sort_by_key(ToString::to_string);
        alts.dedup();
        if alts.len() == 1 {
            // Fold the single alternative into the value map.
            if let Value::Object(only) = alts.remove(0) {
                for (k, v) in only {
                    value.entry(k).or_insert(v);
                }
            }
        } else {
            value.insert("anyOf".to_owned(), Value::Array(alts));
        }
    }

    if !comments.is_empty() {
        comments.sort();
        comments.dedup();
        value.insert("$comment".to_owned(), json!(comments.join("; ")));
    }

    let single = Value::Object(value);

    // Required iff minCount >= 1.
    let is_required = min_count.is_some_and(|n| n >= 1);

    // Cardinality wrapping: maxCount==1 → single; else array.
    //
    // JSON-LD convention (and the [`crate::instance`] projector's exact
    // behaviour): a property with a SINGLE value is emitted UNWRAPPED — a bare
    // scalar / `{"@id":..}` / `{"@value":..}` — and only multi-valued properties
    // are wrapped in a JSON array. So an array-cardinality property schema must
    // accept BOTH the bare single form and the array form, or it would reject
    // SHACL-conformant single-value data the projector legitimately emits.
    //
    // Soundness: accepting the bare single form is sound iff `minCount <= 1`. The
    // projector only emits the bare form when the data has EXACTLY ONE value;
    // such data conforms to SHACL only when `minCount <= 1` (a `minCount >= 2`
    // shape rejects single-value data, putting it out of scope). When
    // `minCount >= 2` we therefore keep the strict array form so the schema does
    // not admit data SHACL rejects.
    let schema = if max_count == Some(1) {
        single
    } else {
        let mut arr: Map<String, Value> = Map::new();
        arr.insert("type".to_owned(), json!("array"));
        arr.insert("items".to_owned(), single.clone());
        if let Some(n) = min_count {
            if n > 0 {
                arr.insert("minItems".to_owned(), json!(n));
            }
        }
        if let Some(n) = max_count {
            arr.insert("maxItems".to_owned(), json!(n));
        }
        let array_form = Value::Object(arr);

        // A single value is permissible exactly when minCount <= 1.
        let allow_single = min_count.is_none_or(|n| n <= 1);
        if allow_single {
            json!({ "anyOf": [single, array_form] })
        } else {
            array_form
        }
    };

    (schema, is_required)
}

/// Insert a numeric bound (`minimum`/`maximum`/…) parsed from a term's lexical
/// form. Non-numeric lexical values are skipped with a `$comment` note.
fn insert_numeric(
    value: &mut Map<String, Value>,
    key: &str,
    term: &Term,
    comments: &mut Vec<String>,
) {
    let lex = term_lexical(term);
    if let Ok(n) = lex.parse::<f64>() {
        if let Some(num) = serde_json::Number::from_f64(n) {
            value.insert(key.to_owned(), Value::Number(num));
            return;
        }
    }
    comments.push(format!(
        "{key} bound on non-numeric value {lex:?} was skipped"
    ));
}

// ── Datatype → JSON type/format mapping ──────────────────────────────────────

/// Map an xsd datatype IRI to a JSON value schema, accepting BOTH the bare
/// scalar form and the JSON-LD `{"@value":..,"@type":..}` typed-literal object.
fn datatype_value_schema(dt_iri: &str) -> Value {
    let scalar = scalar_schema_for_datatype(dt_iri);
    json!({
        "anyOf": [
            scalar,
            typed_literal_schema()
        ]
    })
}

/// The bare-scalar schema for an xsd datatype (no JSON-LD wrapper).
fn scalar_schema_for_datatype(dt_iri: &str) -> Value {
    let Some(local) = dt_iri.strip_prefix(XSD_NS) else {
        // Non-xsd datatype: treat the lexical form as a string.
        return json!({ "type": "string" });
    };
    match local {
        "string" | "normalizedString" | "token" | "language" | "Name" | "NCName" => {
            json!({ "type": "string" })
        }
        "boolean" => json!({ "type": "boolean" }),
        "integer" | "int" | "long" | "short" | "byte" | "nonNegativeInteger"
        | "positiveInteger" | "nonPositiveInteger" | "negativeInteger" | "unsignedLong"
        | "unsignedInt" | "unsignedShort" | "unsignedByte" => json!({ "type": "integer" }),
        "decimal" | "double" | "float" => json!({ "type": "number" }),
        "dateTime" | "dateTimeStamp" => json!({ "type": "string", "format": "date-time" }),
        "date" => json!({ "type": "string", "format": "date" }),
        "time" => json!({ "type": "string", "format": "time" }),
        "anyURI" => json!({ "type": "string", "format": "uri" }),
        // Unknown xsd:* → string.
        _ => json!({ "type": "string" }),
    }
}

/// The language-tagged-literal value schema for a `sh:languageIn` tag set.
///
/// Tags use RFC4647 basic-filtering semantics: a value tag matches an entry iff
/// it equals it or is a subtag (`en` matches `en-US`). Expressed as a regex
/// `pattern` on `@language` like `^(en|fr)(-.*)?$`.
fn lang_literal_schema(tags: &[String]) -> Value {
    let mut sorted: Vec<String> = tags.iter().map(|t| regex::escape(t)).collect();
    sorted.sort();
    sorted.dedup();
    let alternation = sorted.join("|");
    let pattern = format!("^({alternation})(-.*)?$");
    json!({
        "type": "object",
        "properties": {
            "@value": { "type": "string" },
            "@language": { "type": "string", "pattern": pattern }
        },
        "required": ["@value", "@language"]
    })
}

// ── Term → JSON value helpers (must match instance.rs) ───────────────────────

/// The lexical form of a term (literal value, IRI string, or blank-node id).
fn term_lexical(term: &Term) -> String {
    match term {
        Term::Literal(lit) => lit.value().to_owned(),
        Term::NamedNode(n) => n.as_str().to_owned(),
        Term::BlankNode(b) => b.as_str().to_owned(),
        other @ Term::Triple(_) => other.to_string(),
    }
}

/// The `sh:in` enum member value, matching what the projector emits.
///
/// IRIs project as the compacted CURIE/IRI string; literals as their lexical.
fn term_enum_value(term: &Term) -> Value {
    match term {
        Term::NamedNode(n) => Value::String(compact_iri(n.as_str())),
        Term::Literal(lit) => Value::String(lit.value().to_owned()),
        Term::BlankNode(b) => Value::String(b.as_str().to_owned()),
        other @ Term::Triple(_) => Value::String(other.to_string()),
    }
}

/// The `sh:hasValue` const value (projected form).
fn term_const_value(term: &Term) -> Value {
    match term {
        Term::NamedNode(n) => json!({ "@id": compact_iri(n.as_str()) }),
        Term::Literal(lit) => {
            if let Some(lang) = lit.language() {
                json!({ "@value": lit.value(), "@language": lang })
            } else {
                let dt = lit.datatype();
                if dt.as_str() == RDF_LANG_STRING || dt.as_str() == XSD_STRING {
                    Value::String(lit.value().to_owned())
                } else {
                    json!({ "@value": lit.value(), "@type": compact_iri(dt.as_str()) })
                }
            }
        }
        Term::BlankNode(b) => json!({ "@id": format!("_:{}", b.as_str()) }),
        other @ Term::Triple(_) => Value::String(other.to_string()),
    }
}

// ── Serialization ────────────────────────────────────────────────────────────

/// Pretty-print a JSON value with 2-space indent + a single trailing newline.
///
/// `serde_json::Value` uses a BTreeMap-backed `Map` (no `preserve_order`
/// feature), so object keys serialize in sorted order; arrays were sorted at
/// build time — output is therefore byte-stable run-to-run. UTF-8, LF only.
fn to_pretty(value: &Value) -> String {
    let mut s =
        serde_json::to_string_pretty(value).expect("serde_json::Value never fails to serialize");
    s.push('\n');
    s
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shapes::from_dataset;

    const PREFIXES: &str = r"
        @prefix sh:    <http://www.w3.org/ns/shacl#> .
        @prefix xsd:   <http://www.w3.org/2001/XMLSchema#> .
        @prefix rdf:   <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
        @prefix rdfs:  <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix purrdf: <https://blackcatinformatics.ca/purrdf/> .
    ";

    fn compile_ttl(body: &str) -> CompiledSchema {
        let ttl = format!("{PREFIXES}{body}");
        let dataset = crate::text_ingest::parse_turtle_to_dataset(&ttl).expect("Turtle parse");
        let shapes = from_dataset(&dataset).expect("shape parse");
        compile(&shapes)
    }

    fn schema_of(c: &CompiledSchema) -> Value {
        serde_json::from_str(&c.schema_json).expect("schema is valid JSON")
    }

    fn def<'a>(schema: &'a Value, name: &str) -> &'a Value {
        &schema["$defs"][name]
    }

    #[test]
    #[should_panic(expected = "unknown-namespace sh:targetClass")]
    fn unknown_namespace_target_class_hard_fails() {
        // A target class from an UNREGISTERED namespace has no prefix CURIE to key
        // its `$defs`/discriminator by; the keying guard must reject it loudly
        // (#700 Gap D). A KNOWN non-purrdf prefix (e.g. logic:) is accepted — see
        // `logic_target_class_keyed_by_curie`.
        compile_ttl(
            r"
            @prefix ex: <https://example.org/> .
            ex:PersonShape a sh:NodeShape ;
                sh:targetClass ex:Person ;
                sh:property [ sh:path purrdf:name ; sh:minCount 1 ] .
        ",
        );
    }

    #[test]
    fn logic_target_class_keyed_by_curie() {
        // A non-purrdf but KNOWN-prefix target class (logic:) keys its `$defs` body
        // by the full CURIE and discriminates `@type` on that same CURIE, so a
        // closed-world logic node is enforced exactly like a purrdf node (#772).
        let schema = schema_of(&compile_ttl(
            r"
            @prefix logic: <https://blackcatinformatics.ca/logic/> .
            logic:CandidateShape a sh:NodeShape ;
                sh:targetClass logic:FormalizationCandidate ;
                sh:property [ sh:path logic:candidateSourceHash ;
                              sh:minCount 1 ; sh:datatype xsd:string ] .
        ",
        ));
        // The body is keyed by the CURIE, not a bare local name.
        assert!(
            def(&schema, "logic:FormalizationCandidate").is_object(),
            "logic class must be keyed by its CURIE in $defs"
        );
        assert!(
            schema["$defs"]["FormalizationCandidate"].is_null(),
            "a logic class must NOT leak under a bare local-name key"
        );
        // The Node discriminator fires on the CURIE @type const and refs the CURIE key.
        let node = def(&schema, "Node");
        let conds = node["allOf"].as_array().expect("Node allOf");
        let fires = conds.iter().any(|c| {
            c["then"]["$ref"] == "#/$defs/logic:FormalizationCandidate"
                && c["if"]["properties"]["@type"]["anyOf"][0]["const"]
                    == "logic:FormalizationCandidate"
        });
        assert!(
            fires,
            "Node must discriminate logic:FormalizationCandidate on its CURIE @type"
        );
    }

    #[test]
    fn test_curie_compaction_and_local_name() {
        assert_eq!(
            compact_iri("https://blackcatinformatics.ca/purrdf/Person"),
            "purrdf:Person"
        );
        assert_eq!(
            compact_iri("http://www.w3.org/2001/XMLSchema#integer"),
            "xsd:integer"
        );
        assert_eq!(
            compact_iri("http://example.org/Foo"),
            "http://example.org/Foo"
        );
        assert_eq!(
            local_name("https://blackcatinformatics.ca/purrdf/Person"),
            "Person"
        );
        assert_eq!(
            local_name("http://www.w3.org/2001/XMLSchema#integer"),
            "integer"
        );
    }

    #[test]
    fn test_required_from_min_count_and_array_vs_single() {
        let c = compile_ttl(
            r"
            purrdf:PersonShape a sh:NodeShape ;
                sh:targetClass purrdf:Person ;
                sh:property [ sh:path purrdf:name ; sh:minCount 1 ; sh:maxCount 1 ; sh:datatype xsd:string ] ;
                sh:property [ sh:path purrdf:nickname ; sh:datatype xsd:string ] .
            ",
        );
        let schema = schema_of(&c);
        let person = def(&schema, "Person");
        // required contains purrdf:name (minCount 1)
        let required = person["required"].as_array().expect("required array");
        assert!(required.iter().any(|v| v == "purrdf:name"));
        // name (maxCount 1) is a single value, NOT an array
        let name = &person["properties"]["purrdf:name"];
        assert_ne!(name["type"], json!("array"), "maxCount 1 → single value");
        // nickname (no maxCount, minCount<=1) accepts BOTH the bare single form
        // AND the array form: the projector emits a bare scalar for a single
        // value and an array only for multiple values, so the schema must accept
        // either or it would reject SHACL-conformant single-value data (#700).
        let nickname = &person["properties"]["purrdf:nickname"];
        let alts = nickname["anyOf"]
            .as_array()
            .expect("no-maxCount property is an anyOf of single|array");
        assert_eq!(alts.len(), 2, "anyOf of single + array forms");
        assert!(
            alts.iter().any(|a| a["type"] == json!("array")),
            "one alternative is the array form: {alts:?}"
        );
        assert!(
            alts.iter().any(|a| a["type"] != json!("array")),
            "one alternative is the bare single form: {alts:?}"
        );
    }

    #[test]
    fn test_datatype_type_and_format() {
        let c = compile_ttl(
            r"
            purrdf:EventShape a sh:NodeShape ;
                sh:targetClass purrdf:Event ;
                sh:property [ sh:path purrdf:at ; sh:maxCount 1 ; sh:datatype xsd:dateTime ] ;
                sh:property [ sh:path purrdf:count ; sh:maxCount 1 ; sh:datatype xsd:integer ] .
            ",
        );
        let schema = schema_of(&c);
        let event = def(&schema, "Event");
        // dateTime → anyOf containing {type:string, format:date-time}
        let at = &event["properties"]["purrdf:at"];
        let at_alts = at["anyOf"].as_array().expect("anyOf");
        assert!(at_alts
            .iter()
            .any(|alt| alt["format"] == json!("date-time")));
        // integer → anyOf containing {type:integer}
        let count = &event["properties"]["purrdf:count"];
        let count_alts = count["anyOf"].as_array().expect("anyOf");
        assert!(count_alts.iter().any(|alt| alt["type"] == json!("integer")));
    }

    #[test]
    fn test_enum_from_sh_in() {
        let c = compile_ttl(
            r#"
            purrdf:ColorShape a sh:NodeShape ;
                sh:targetClass purrdf:Color ;
                sh:property [ sh:path purrdf:value ; sh:maxCount 1 ; sh:in ( "red" "green" "blue" ) ] .
            "#,
        );
        let schema = schema_of(&c);
        let value = &def(&schema, "Color")["properties"]["purrdf:value"];
        let en = value["enum"].as_array().expect("enum array");
        // sorted: blue, green, red
        assert_eq!(en.len(), 3);
        assert!(en.iter().any(|v| v == "red"));
        // Determinism: sorted ascending.
        let strs: Vec<&str> = en.iter().filter_map(|v| v.as_str()).collect();
        let mut sorted = strs.clone();
        sorted.sort_unstable();
        assert_eq!(strs, sorted, "enum must be sorted");
    }

    #[test]
    fn test_pattern() {
        let c = compile_ttl(
            r#"
            purrdf:CodeShape a sh:NodeShape ;
                sh:targetClass purrdf:Code ;
                sh:property [ sh:path purrdf:code ; sh:maxCount 1 ; sh:pattern "^[A-Z]+$" ] .
            "#,
        );
        let schema = schema_of(&c);
        let code = &def(&schema, "Code")["properties"]["purrdf:code"];
        assert_eq!(code["pattern"], json!("^[A-Z]+$"));
    }

    #[test]
    fn test_closed_additional_properties_false() {
        let c = compile_ttl(
            r"
            purrdf:ClosedShape a sh:NodeShape ;
                sh:targetClass purrdf:Sealed ;
                sh:closed true ;
                sh:ignoredProperties ( rdf:type ) ;
                sh:property [ sh:path purrdf:only ; sh:maxCount 1 ; sh:datatype xsd:string ] .
            ",
        );
        let schema = schema_of(&c);
        let sealed = def(&schema, "Sealed");
        assert_eq!(sealed["additionalProperties"], json!(false));
        // The single declared property key is present.
        assert!(sealed["properties"]["purrdf:only"].is_object());
    }

    #[test]
    fn test_not_constraint() {
        let c = compile_ttl(
            r"
            purrdf:NotShape a sh:NodeShape ;
                sh:targetClass purrdf:Thing ;
                sh:not [ sh:nodeKind sh:Literal ] .
            ",
        );
        let schema = schema_of(&c);
        let thing = def(&schema, "Thing");
        assert!(thing["not"].is_object(), "expected a `not` subschema");
    }

    #[test]
    fn test_sparql_constraint_records_loss_and_comment() {
        let c = compile_ttl(
            r#"
            purrdf:SparqlShape a sh:NodeShape ;
                sh:targetClass purrdf:Guarded ;
                sh:sparql [
                    sh:select "SELECT $this WHERE { $this <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://blackcatinformatics.ca/purrdf/Guarded> . }" ;
                ] .
            "#,
        );
        assert!(!c.losses.is_empty(), "sh:sparql must record a LossRecord");
        let loss = &c.losses[0];
        assert_eq!(loss.construct, "sh:sparql");
        assert!(loss.reason.contains("SPARQL"));
        // The affected schema carries a $comment noting the drop.
        let schema = schema_of(&c);
        let guarded = def(&schema, "Guarded");
        assert!(
            guarded["$comment"]
                .as_str()
                .unwrap_or("")
                .contains("sparql"),
            "expected a $comment noting the dropped sh:sparql, got {:?}",
            guarded["$comment"]
        );
    }

    #[test]
    fn test_object_property_uses_ref() {
        let c = compile_ttl(
            r"
            purrdf:OrgShape a sh:NodeShape ;
                sh:targetClass purrdf:Organization ;
                sh:property [ sh:path purrdf:member ; sh:maxCount 1 ; sh:class purrdf:Person ] .
            purrdf:PersonShape a sh:NodeShape ;
                sh:targetClass purrdf:Person .
            ",
        );
        let schema = schema_of(&c);
        let member = &def(&schema, "Organization")["properties"]["purrdf:member"];
        // anyOf includes a node ref {"@id":..} and a $ref to #/$defs/Person.
        let alts = member["anyOf"].as_array().expect("anyOf");
        assert!(alts.iter().any(|a| a["$ref"] == json!("#/$defs/Person")));
        assert!(alts.iter().any(|a| a["properties"]["@id"].is_object()));
    }

    #[test]
    fn test_annotation_def_present_and_root_envelope() {
        let c = compile_ttl(
            r"
            purrdf:PersonShape a sh:NodeShape ;
                sh:targetClass purrdf:Person ;
                sh:property [ sh:path purrdf:name ; sh:datatype xsd:string ] .
            ",
        );
        let schema = schema_of(&c);
        // $defs/Annotation exists.
        assert!(schema["$defs"]["Annotation"].is_object());
        // Root envelope keys.
        assert_eq!(
            schema["$schema"],
            json!("https://json-schema.org/draft/2020-12/schema")
        );
        assert!(schema["properties"]["@graph"].is_object());
        assert!(schema["anyOf"].is_array(), "root anyOf graph|bare-node");
        // Each node schema carries an @annotation key referencing the fragment.
        let person = def(&schema, "Person");
        assert_eq!(
            person["properties"]["@annotation"]["$ref"],
            json!("#/$defs/Annotation")
        );
    }

    #[test]
    fn test_deactivated_shape_skipped() {
        let c = compile_ttl(
            r"
            purrdf:GoneShape a sh:NodeShape ;
                sh:targetClass purrdf:Gone ;
                sh:deactivated true ;
                sh:property [ sh:path purrdf:x ; sh:datatype xsd:string ] .
            ",
        );
        let schema = schema_of(&c);
        assert!(
            schema["$defs"]["Gone"].is_null(),
            "deactivated shape must not produce a $def"
        );
    }

    #[test]
    fn test_openapi_embeds_components_schemas() {
        let c = compile_ttl(
            r"
            purrdf:PersonShape a sh:NodeShape ;
                sh:targetClass purrdf:Person ;
                sh:property [ sh:path purrdf:name ; sh:datatype xsd:string ] .
            ",
        );
        let openapi: Value = serde_json::from_str(&c.openapi_json).expect("openapi JSON");
        assert_eq!(openapi["openapi"], json!("3.1.0"));
        assert!(openapi["components"]["schemas"]["Person"].is_object());
        assert!(openapi["paths"]["/entities/{id}"]["get"].is_object());
        // trailing newline convention
        assert!(c.openapi_json.ends_with("}\n"));
    }

    /// Recursively collect every `"$ref": "#/$defs/<name>"` `<name>` reachable
    /// from a JSON value.
    fn collect_def_refs(v: &Value, out: &mut Vec<String>) {
        match v {
            Value::Object(map) => {
                if let Some(Value::String(r)) = map.get("$ref") {
                    if let Some(name) = r.strip_prefix("#/$defs/") {
                        out.push(name.to_owned());
                    }
                }
                for child in map.values() {
                    collect_def_refs(child, out);
                }
            }
            Value::Array(items) => {
                for child in items {
                    collect_def_refs(child, out);
                }
            }
            _ => {}
        }
    }

    /// Self-consistency invariant: EVERY `#/$defs/<name>` ref the emitter writes
    /// must resolve to an actually-emitted key in the top-level `$defs`. This is
    /// the bug guard for #700 — an object property whose `sh:class` points at a
    /// class with NO NodeShape must NOT emit a dangling `$ref`.
    #[test]
    fn every_ref_resolves() {
        // purrdf:Organization HAS a shape; purrdf:Ghost (the sh:class target of the
        // `haunts` property) has NONE — so no `$defs/Ghost` is emitted and a ref
        // to it would dangle. Also exercise sh:node (inline) and @annotation.
        let c = compile_ttl(
            r"
            purrdf:OrgShape a sh:NodeShape ;
                sh:targetClass purrdf:Organization ;
                sh:node [ sh:property [ sh:path purrdf:label ; sh:datatype xsd:string ] ] ;
                sh:property [ sh:path purrdf:member ; sh:maxCount 1 ; sh:class purrdf:Person ] ;
                sh:property [ sh:path purrdf:haunts ; sh:maxCount 1 ; sh:class purrdf:Ghost ] .
            purrdf:PersonShape a sh:NodeShape ;
                sh:targetClass purrdf:Person .
            ",
        );
        let schema = schema_of(&c);

        // Collect the set of emitted $defs keys.
        let defs: BTreeSet<String> = schema["$defs"]
            .as_object()
            .expect("$defs object")
            .keys()
            .cloned()
            .collect();

        // Walk the ENTIRE schema and assert every $ref resolves.
        let mut refs = Vec::new();
        collect_def_refs(&schema, &mut refs);
        assert!(
            !refs.is_empty(),
            "expected at least the Annotation/class refs"
        );
        for name in &refs {
            assert!(
                defs.contains(name),
                "dangling $ref #/$defs/{name}: not an emitted def (have {defs:?})"
            );
        }

        // The Ghost class (no shape) must NOT have produced a $ref anywhere.
        assert!(
            !refs.iter().any(|r| r == "Ghost"),
            "a class with no NodeShape must not be $ref'd"
        );
        // …and the haunts property must carry the node-reference-only form with a
        // $comment noting Ghost has no shape.
        let haunts = &def(&schema, "Organization")["properties"]["purrdf:haunts"];
        let comment = haunts["$comment"].as_str().unwrap_or("");
        assert!(
            comment.contains("Ghost") && comment.contains("no NodeShape"),
            "expected a node-reference-only $comment for purrdf:Ghost, got {haunts:?}"
        );
        // The Person ref (class WITH a shape) is still present.
        assert!(refs.iter().any(|r| r == "Person"));

        // The discriminated Node schema is emitted and itself only $refs emitted
        // defs (the `if` consts are plain strings, not refs). Walk Node directly
        // and assert every ref it carries resolves.
        let node = def(&schema, "Node");
        assert!(node.is_object(), "expected a $defs/Node schema");
        let mut node_refs = Vec::new();
        collect_def_refs(node, &mut node_refs);
        for name in &node_refs {
            assert!(
                defs.contains(name),
                "Node carries a dangling $ref #/$defs/{name} (have {defs:?})"
            );
        }
        // Node references each emitted class def in a `then`, plus Annotation.
        assert!(node_refs.iter().any(|r| r == "Organization"));
        assert!(node_refs.iter().any(|r| r == "Person"));
        assert!(node_refs.iter().any(|r| r == "Annotation"));
        // …and never an unmodeled class.
        assert!(
            !node_refs.iter().any(|r| r == "Ghost"),
            "Node must not $ref an unmodeled class"
        );
    }

    #[test]
    fn closed_world_rejects_incomplete_typed_node() {
        // A class with a required property: a node typed purrdf:Thing that is
        // missing purrdf:req must (structurally) be funnelled through Thing's
        // `then` and fail Thing's `required` — i.e. the discrimination exists and
        // Thing actually requires purrdf:req.
        let c = compile_ttl(
            r"
            purrdf:ThingShape a sh:NodeShape ;
                sh:targetClass purrdf:Thing ;
                sh:property [ sh:path purrdf:req ; sh:minCount 1 ; sh:maxCount 1 ; sh:datatype xsd:string ] .
            ",
        );
        let schema = schema_of(&c);

        // The discriminated Node schema exists.
        let node = def(&schema, "Node");
        assert!(node.is_object(), "expected a $defs/Node schema");

        // Node carries permissive @id/@type/@annotation.
        assert!(node["properties"]["@id"].is_object());
        assert!(node["properties"]["@type"].is_object());
        assert_eq!(
            node["properties"]["@annotation"]["$ref"],
            json!("#/$defs/Annotation")
        );

        // It carries an allOf conditional list.
        let conds = node["allOf"].as_array().expect("Node.allOf array");

        // Find the conditional whose `then` is the Thing ref.
        let thing_cond = conds
            .iter()
            .find(|c| c["then"]["$ref"] == json!("#/$defs/Thing"))
            .expect("a conditional whose then $refs #/$defs/Thing");

        // Its `if` requires @type and matches @type == "purrdf:Thing" both as a
        // bare const and as an array `contains`.
        let if_clause = &thing_cond["if"];
        assert_eq!(if_clause["required"], json!(["@type"]));
        let type_alts = if_clause["properties"]["@type"]["anyOf"]
            .as_array()
            .expect("@type discrimination anyOf");
        assert!(
            type_alts
                .iter()
                .any(|a| a["const"] == json!("purrdf:Thing")),
            "expected a bare const purrdf:Thing branch, got {type_alts:?}"
        );
        assert!(
            type_alts
                .iter()
                .any(|a| a["type"] == json!("array")
                    && a["contains"]["const"] == json!("purrdf:Thing")),
            "expected an array-contains purrdf:Thing branch, got {type_alts:?}"
        );

        // And Thing actually requires purrdf:req — so an incomplete node IS
        // rejected once routed through Thing's `then`.
        let thing = def(&schema, "Thing");
        let required = thing["required"].as_array().expect("Thing.required array");
        assert!(
            required.iter().any(|v| v == "purrdf:req"),
            "Thing must require purrdf:req, got {required:?}"
        );

        // Thing itself must NOT require @type (discrimination lives in Node).
        assert!(
            !required.iter().any(|v| v == "@type"),
            "per-class def must not require @type"
        );

        // The root envelope routes every node through Node.
        assert_eq!(
            schema["properties"]["@graph"]["items"]["$ref"],
            json!("#/$defs/Node")
        );
        let root_anyof = schema["anyOf"].as_array().expect("root anyOf");
        assert!(
            root_anyof
                .iter()
                .any(|b| b["$ref"] == json!("#/$defs/Node")),
            "bare-node root alternative must $ref Node"
        );
    }

    #[test]
    fn test_determinism_byte_stable() {
        let body = r"
            purrdf:PersonShape a sh:NodeShape ;
                sh:targetClass purrdf:Person ;
                sh:property [ sh:path purrdf:name ; sh:minCount 1 ; sh:datatype xsd:string ] ;
                sh:property [ sh:path purrdf:age ; sh:maxCount 1 ; sh:datatype xsd:integer ] .
        ";
        let a = compile_ttl(body);
        let b = compile_ttl(body);
        assert_eq!(
            a.schema_json, b.schema_json,
            "schema output must be byte-stable"
        );
        assert_eq!(
            a.openapi_json, b.openapi_json,
            "openapi output must be byte-stable"
        );
        // pretty-printed (2-space) + trailing newline
        assert!(a.schema_json.ends_with("}\n"));
        assert!(a.schema_json.contains("\n  \""), "expected 2-space indent");
    }
}
