// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SHACL → JSON Schema (draft 2020-12) + OpenAPI 3.1 emitter.
//!
//! Compiles a parsed [`Shapes`] graph into a closed-world JSON Schema describing
//! the JSON-LD projection of PurRDF instance data (see [`crate::instance`]). The
//! emitter and the projector share ONE CURIE-compaction / value-shaping
//! convention so a projected node always validates against the schema this
//! module produces (Task 6 proves the round trip over every slice example).
//!
//! # Conventions (must stay in lock-step with `instance.rs`)
//!
//! * **IRI compaction** — [`Namespaces::compact_iri`] maps a declared namespace
//!   prefix to `prefix:LocalName`; otherwise the full IRI is kept verbatim.
//! * **Object (node) value** — a JSON object `{"@id": "<compacted-iri>"}`.
//! * **Typed literal value** — `{"@value": "<lexical>", "@type": "<compacted-datatype>"}`.
//!   For numeric / boolean datatypes the projector MAY also emit a bare JSON
//!   scalar, so the value schema accepts BOTH the scalar and the object form
//!   (`anyOf`).
//! * **Language-tagged literal** — `{"@value": "<lexical>", "@language": "<tag>"}`.
//! * **Plain string** — a bare JSON string.
//! * **Statement metadata** — an optional `@annotation` key on any property value
//!   object, referencing `#/$defs/Annotation` (RDF-1.2 reifier metadata).
//!
//! # SPARQL losses
//!
//! `sh:sparql` / `sh:SPARQLTarget` constraints have no JSON Schema equivalent.
//! They are never silently skipped: each one is dropped, recorded as a
//! [`LossRecord`], and annotated with a `$comment` on the affected schema.
//!
//! # Value-vocabulary enum projection (opt-in, projection-only)
//!
//! [`compile_with_value_vocab`] optionally projects *value vocabularies* — an
//! `owl:Class` whose members are seeded named individuals, marked by a
//! caller-supplied [`ValueVocab`] — to standalone `{ClassLocal}Enum` `$defs`. Such
//! vocabularies are deliberately OPEN in the live SHACL validator ("anchor, not a
//! fence") and carry no `sh:in`; this derivation surfaces their anchor set as a
//! closed `enum` for codegen / API consumers WITHOUT ever injecting `sh:in` or
//! otherwise mutating the validating shape set. Members are `{"@id": curie}`
//! objects (the projector's encoding, so a projected instance validates); the flat
//! `enum` keyword stays load-bearing in every case, with member symbol names and
//! docs carried on the parallel `x-enum-varnames` / `x-enum-descriptions` arrays.
//! A property whose `sh:class` or ontology `rdfs:range` is such a vocabulary emits
//! a `$ref` to its enum `$def`, cardinality preserved.

use std::collections::{BTreeMap, BTreeSet};

use ::purrdf::RdfDataset;
use serde_json::{Map, Value, json};

use crate::data::{GraphFilter, native_quads};
use crate::model::{rdf, rdfs};
use crate::shapes::{Constraint, NodeKindValue, Path, Shape, Shapes, Target};
use crate::term::{NamedNode, Term};

const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema#";
const RDF_NS: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const RDFS_NS: &str = "http://www.w3.org/2000/01/rdf-schema#";
const OWL_NS: &str = "http://www.w3.org/2002/07/owl#";
const SH_NS: &str = "http://www.w3.org/ns/shacl#";
/// The two datatype IRIs whose literals project as a bare JSON string (no alloc
/// per literal — see [`crate::instance`] for the matching projection convention).
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";

/// The W3C builtin prefixes that are ALWAYS available for compaction, whatever
/// the shapes document declares. A document declaration of the same prefix name
/// wins on conflict.
const BUILTIN_PREFIXES: &[(&str, &str)] = &[
    ("xsd", XSD_NS),
    ("rdf", RDF_NS),
    ("rdfs", RDFS_NS),
    ("owl", OWL_NS),
    ("sh", SH_NS),
];

/// The caller-supplied namespace table driving ALL IRI compaction, `$defs`
/// keying, and `@type` discrimination — for BOTH the schema emitter
/// ([`compile`]) and the instance projector ([`crate::instance`]).
///
/// Nothing is hardcoded in the library: the *primary* namespace (whose classes
/// key their `$defs` by bare local name and discriminate `@type` as
/// `primary_prefix:LocalName`) and every other compactable namespace come from
/// the caller, typically the shapes document's own `@prefix` declarations
/// (see [`crate::shapes::from_dataset_with_prefixes`]).
///
/// # Example (downstream call pattern)
///
/// ```
/// use purrdf_shapes::json_schema::{compile, Namespaces};
///
/// // The shapes document's @prefix declarations (prefix → namespace).
/// let doc_prefixes = vec![(
///     "gmeow".to_owned(),
///     "https://blackcatinformatics.ca/gmeow/".to_owned(),
/// )];
/// let ns = Namespaces::new("gmeow", &doc_prefixes)?;
///
/// # let ttl = r"
/// #     @prefix sh:    <http://www.w3.org/ns/shacl#> .
/// #     @prefix xsd:   <http://www.w3.org/2001/XMLSchema#> .
/// #     @prefix gmeow: <https://blackcatinformatics.ca/gmeow/> .
/// #     gmeow:CatShape a sh:NodeShape ;
/// #         sh:targetClass gmeow:Cat ;
/// #         sh:property [ sh:path gmeow:name ; sh:minCount 1 ; sh:datatype xsd:string ] .
/// # ";
/// # let dataset = purrdf_shapes::text_ingest::parse_turtle_to_dataset(ttl).unwrap();
/// # let shapes = purrdf_shapes::shapes::from_dataset(&dataset).unwrap();
/// let out = compile(&shapes, &ns);
/// # assert!(out.schema_json.contains("gmeow:Cat"));
/// # Ok::<(), String>(())
/// ```
#[derive(Debug, Clone)]
pub struct Namespaces {
    /// `(prefix, namespace)` pairs sorted longest-namespace-first so the most
    /// specific namespace always wins compaction (prefix name breaks ties
    /// deterministically).
    prefixes: Vec<(String, String)>,
    /// The prefix whose classes key `$defs` by bare local name.
    primary_prefix: String,
    /// The namespace `primary_prefix` resolves to.
    primary_ns: String,
}

impl Namespaces {
    /// Build a namespace table from the primary prefix and the shapes
    /// document's `(prefix, namespace)` declarations.
    ///
    /// The W3C builtins (`xsd`, `rdf`, `rdfs`, `owl`, `sh`) are always merged
    /// in; a document declaration of the same prefix name wins on conflict.
    ///
    /// # Errors
    ///
    /// Returns `Err` when `primary_prefix` resolves in neither `doc_prefixes`
    /// nor the builtins.
    pub fn new(primary_prefix: &str, doc_prefixes: &[(String, String)]) -> Result<Self, String> {
        let mut merged: BTreeMap<String, String> = BUILTIN_PREFIXES
            .iter()
            .map(|(p, n)| ((*p).to_owned(), (*n).to_owned()))
            .collect();
        for (prefix, ns) in doc_prefixes {
            merged.insert(prefix.clone(), ns.clone());
        }
        let Some(primary_ns) = merged.get(primary_prefix).cloned() else {
            return Err(format!(
                "Namespaces: primary prefix {primary_prefix:?} is not declared — pass it in \
                 doc_prefixes (the shapes document's @prefix declarations) or use a W3C builtin"
            ));
        };
        let mut prefixes: Vec<(String, String)> = merged.into_iter().collect();
        // Longest-namespace-first so the most specific namespace is matched
        // before any shorter one that prefixes it; tie-break on prefix name for
        // run-to-run determinism.
        prefixes.sort_by(|(pa, na), (pb, nb)| nb.len().cmp(&na.len()).then_with(|| pa.cmp(pb)));
        Ok(Self {
            prefixes,
            primary_prefix: primary_prefix.to_owned(),
            primary_ns,
        })
    }

    /// Compact an IRI to `prefix:LocalName` when it begins with a declared
    /// namespace; otherwise return the full IRI unchanged.
    ///
    /// This is the single shared compaction helper used by BOTH the schema
    /// emitter and the instance projector ([`crate::instance`]).
    #[must_use]
    pub fn compact_iri(&self, iri: &str) -> String {
        for (prefix, ns) in &self.prefixes {
            if let Some(local) = iri.strip_prefix(ns.as_str()) {
                return format!("{prefix}:{local}");
            }
        }
        iri.to_owned()
    }

    /// Whether an IRI is in the primary namespace (object refs to primary
    /// classes get a `$ref`; external classes get a permissive node-ref /
    /// string).
    #[must_use]
    pub fn is_primary(&self, iri: &str) -> bool {
        iri.starts_with(self.primary_ns.as_str())
    }

    /// The `$defs`/discriminator key for a target class. A primary-namespace
    /// class keeps its bare local name (a valid OpenAPI `components/schemas`
    /// key); any other declared-namespace class is keyed by its full CURIE
    /// (`logic:FormalizationCandidate`), so cross-namespace local-name twins
    /// never collide; an undeclared-namespace IRI is returned verbatim (and
    /// rejected by [`compile`]'s keying guard). A primary local name never
    /// contains a `:`, while a CURIE always does — the discriminator
    /// (`node_def`) relies on that to rebuild each `@type` const.
    #[must_use]
    pub fn def_key(&self, iri: &str) -> String {
        if self.is_primary(iri) {
            local_name(iri)
        } else {
            self.compact_iri(iri)
        }
    }

    /// The JSON-LD `@context` prefix-map object (every declared prefix plus
    /// the merged builtins), for the instance projector's `@context`.
    #[must_use]
    pub fn context_object(&self) -> Map<String, Value> {
        let mut ctx = Map::new();
        for (prefix, ns) in &self.prefixes {
            ctx.insert(prefix.clone(), Value::String(ns.clone()));
        }
        ctx
    }

    /// The primary namespace IRI (drives the schema `$id`).
    #[must_use]
    pub fn primary_ns(&self) -> &str {
        &self.primary_ns
    }

    /// Whether an IRI is in a declared namespace (i.e. [`Self::compact_iri`]
    /// would compact it to a `prefix:Local` CURIE rather than returning it
    /// verbatim).
    fn is_known(&self, iri: &str) -> bool {
        self.compact_iri(iri) != iri
    }
}

/// The bare local name of an IRI: the substring after the last `#` or `/`.
pub fn local_name(iri: &str) -> String {
    let after_hash = iri.rsplit('#').next().unwrap_or(iri);
    // `rsplit('#')` returns the whole string when there is no `#`, so split on
    // `/` over that remainder.
    let local = after_hash.rsplit('/').next().unwrap_or(after_hash);
    local.to_owned()
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

// ── Value-vocabulary projection config ───────────────────────────────────────

/// A caller-supplied marker identifying *value-vocabulary* classes.
///
/// A value vocabulary is an `owl:Class` whose members are seeded named
/// individuals (e.g. `TermStability` = `stable` / `experimental` / `deprecated`).
/// These classes are deliberately OPEN in the live SHACL validator ("anchor, not
/// a fence") and so carry no `sh:in`; [`compile_with_value_vocab`] nonetheless
/// projects each one to a standalone `{ClassLocal}Enum` `$def` for JSON-Schema /
/// OpenAPI consumers, WITHOUT ever mutating the validating shape set.
///
/// `abstract_individual_type` is the IRI of the marker class: a class `V` is a
/// value vocabulary iff `V rdf:type <abstract_individual_type>`. PurRDF mints no
/// vocabulary IRIs, so this is caller configuration with NO `Default` — mirroring
/// [`crate::model::BoxRoleVocab`] and `SliceVocab`. A `None` projection (see
/// [`compile_with_value_vocab`]) means the feature is INACTIVE, not that a
/// required dependency is missing; compilation then matches [`compile`] exactly.
#[derive(Debug, Clone)]
pub struct ValueVocab {
    abstract_individual_type: String,
}

impl ValueVocab {
    /// Construct a value-vocabulary marker from the marker class IRI.
    #[must_use]
    pub fn new(abstract_individual_type: &str) -> Self {
        Self {
            abstract_individual_type: abstract_individual_type.to_owned(),
        }
    }

    /// The marker class IRI whose instances are value-vocabulary classes.
    #[must_use]
    pub fn abstract_individual_type(&self) -> &str {
        &self.abstract_individual_type
    }
}

/// The co-required inputs enabling the value-vocabulary enum projection: the
/// caller's [`ValueVocab`] marker AND the ontology/data graph holding the
/// individuals to enumerate.
///
/// Bundling the two makes "a marker with no ontology to enumerate" unrepresentable
/// (fail-closed by type). Enumeration additionally scans the shapes graph, so
/// range/label axioms declared shapes-side are honored (see
/// [`compile_with_value_vocab`]).
#[derive(Debug, Clone, Copy)]
pub struct ValueVocabProjection<'a> {
    /// The caller-supplied value-vocabulary marker.
    pub vocab: &'a ValueVocab,
    /// The ontology / data graph holding the value-vocabulary individuals.
    pub ontology: &'a RdfDataset,
}

// ── Compilation context ──────────────────────────────────────────────────────

/// Accumulates losses while compiling so every emitter helper can record one,
/// and carries the caller-supplied [`Namespaces`] every helper compacts with.
struct Ctx<'ns> {
    losses: Vec<LossRecord>,
    /// The set of `Namespaces::def_key` keys that WILL receive a `$def` — i.e.
    /// every `def_key(target_class)` over all non-deactivated
    /// `Target::Class(..)` shapes. This MUST use the same key function as the
    /// `$defs` map built in [`compile`] (`ns.def_key`, not a bare local name):
    /// a primary-namespace class and a non-primary class sharing a local name
    /// (e.g. `gmeow:Distribution` vs `math:Distribution`) are DISTINCT
    /// `def_key`s but would collapse to the same bare local name, which would
    /// make one twin's absence from `$defs` invisible to this set. An object
    /// property's `sh:class C` may only emit a `#/$defs/<def_key(C)>` ref when
    /// `def_key(C)` is in this set; otherwise the ref would dangle (no shape ⇒
    /// no `$def`).
    emitted_defs: BTreeSet<String>,
    /// Value-vocabulary classes → their `{Local}Enum` `$def` key. A `sh:class`
    /// pointing at one of these resolves to the enum `$ref` (projection-only)
    /// rather than the permissive node-ref. Empty unless a value-vocabulary
    /// projection is active.
    value_vocab_enums: BTreeMap<String, String>,
    /// Predicate IRIs whose `rdfs:range` is a value-vocabulary class → that
    /// class's `{Local}Enum` `$def` key. Drives the enum `$ref` for OPEN
    /// vocabularies that carry no `sh:class`. Empty unless a projection is active.
    predicate_ranges: BTreeMap<String, String>,
    /// The namespace table driving ALL compaction / keying decisions.
    ns: &'ns Namespaces,
}

impl<'ns> Ctx<'ns> {
    fn new(
        emitted_defs: BTreeSet<String>,
        value_vocab_enums: BTreeMap<String, String>,
        predicate_ranges: BTreeMap<String, String>,
        ns: &'ns Namespaces,
    ) -> Self {
        Self {
            losses: Vec::new(),
            emitted_defs,
            value_vocab_enums,
            predicate_ranges,
            ns,
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
///
/// Every compaction / keying decision — CURIE compaction, `$defs` keys, the
/// `@type` discriminator, the schema `$id` — flows through the caller-supplied
/// [`Namespaces`], so downstream shape corpora in any namespace compile without
/// touching this crate:
///
/// ```text
/// let ns = Namespaces::new("gmeow", &doc_prefixes)?;
/// let out = compile(&shapes, &ns);
/// ```
///
/// # Panics
///
/// Panics (build-time, fail-closed) when an active `sh:targetClass` is in a
/// namespace with no declared prefix, or when two distinct target classes
/// would share a `$defs` key — see [`Namespaces::def_key`].
pub fn compile(shapes: &Shapes, ns: &Namespaces) -> CompiledSchema {
    compile_with_value_vocab(shapes, ns, None)
}

/// Compile a parsed [`Shapes`] graph, optionally projecting *value vocabularies*
/// to enum `$defs`.
///
/// Identical to [`compile`] when `projection` is `None`. When `Some`, each class
/// `V` marked as a value vocabulary (`V rdf:type <marker>`, the marker taken from
/// [`ValueVocab`]) is projected to a standalone `{LocalName(V)}Enum` `$def` whose
/// members are `V`'s named individuals — enumerated from the projection's ontology
/// graph UNIONED with the shapes graph — encoded as `{"@id": curie}` objects (the
/// same encoding the instance projector emits, so a projected instance validates).
/// The flat JSON-Schema `enum` keyword is always the load-bearing form; member
/// symbol names and docs ride the parallel `x-enum-varnames` / `x-enum-descriptions`
/// extension arrays. The projection is DERIVATION-ONLY: it never injects `sh:in`
/// and never mutates the validating shape set, so the live SHACL validator stays
/// open-world.
///
/// # Panics
///
/// Panics (build-time, fail-closed) under the same conditions as [`compile`], and
/// additionally when a value-vocabulary class is in an undeclared namespace or two
/// value-vocabulary classes would share a `{LocalName}Enum` `$def` key (or an enum
/// key collides with a class `$def` key).
pub fn compile_with_value_vocab(
    shapes: &Shapes,
    ns: &Namespaces,
    projection: Option<&ValueVocabProjection<'_>>,
) -> CompiledSchema {
    // Keying invariant (Gap D, fail-closed): every primary-namespace `$def`
    // is keyed by the class LOCAL NAME and the `@type` discriminator is
    // `<primary_prefix>:<LocalName>`. That is sound ONLY while every target
    // class is in a declared namespace and no two distinct class IRIs share a
    // key. Local-name keys are deliberate — a colon-bearing compact IRI is not
    // a valid OpenAPI `components/schemas` key (`^[a-zA-Z0-9._-]+$`) — so this
    // guard protects the precondition rather than widening the keys. A target
    // class from an undeclared namespace or a colliding key HARD-fails the
    // build here instead of silently mis-discriminating or clobbering a `$def`.
    assert_target_class_keys_are_unambiguous(shapes, ns);

    // Value-vocabulary enum `$defs` (projection-only): class IRI → (enum key,
    // `$def` body). Empty when `projection` is `None`. Enum keys are guarded for
    // collisions against each other and the class `$def` keys before use.
    let (vocab_enums, vocab_losses) = value_vocab_enum_defs(shapes, ns, projection);
    assert_value_vocab_enum_keys_unambiguous(shapes, ns, &vocab_enums);

    // PASS 1: compute the set of `def_key`s that WILL receive a `$def`, using
    // the EXACT same iteration AND the EXACT same key function (`ns.def_key`)
    // as the `$defs` map built below (every `Target::Class(..)` of every
    // non-deactivated node shape). Keying this set by bare local name instead
    // would conflate a primary class with any non-primary class sharing its
    // local name (e.g. `gmeow:Distribution` vs `math:Distribution` both
    // reducing to `"Distribution"`), making one twin's presence mask the
    // other's absence from `$defs` — a dangling `$ref`. This lets the
    // per-property emitter decide whether a `sh:class C` ref can resolve
    // before the `$defs` map is fully built, so it never writes a dangling
    // `$ref`.
    let mut emitted_defs: BTreeSet<String> = BTreeSet::new();
    for shape in &shapes.node_shapes {
        if shape.deactivated {
            continue;
        }
        for target in &shape.targets {
            if let Target::Class(c) = target {
                emitted_defs.insert(ns.def_key(c.as_str()));
            }
        }
    }
    // The value-vocabulary enum `$defs` are always emitted, so a `$ref` to one
    // (the referencing-property path) never dangles.
    for (key, _def) in vocab_enums.values() {
        emitted_defs.insert(key.clone());
    }

    // The referencing-property lookups: `sh:class` → enum key (class IRI keyed),
    // and predicate `rdfs:range` → enum key (predicate IRI keyed).
    let value_vocab_enums: BTreeMap<String, String> = vocab_enums
        .iter()
        .map(|(class_iri, (key, _def))| (class_iri.clone(), key.clone()))
        .collect();
    let predicate_ranges = value_vocab_predicate_ranges(shapes, projection, &vocab_enums);

    let mut ctx = Ctx::new(emitted_defs, value_vocab_enums, predicate_ranges, ns);
    // Enum enumeration losses (empty vocab, blank-node member) recorded first,
    // deterministically, ahead of the per-shape losses.
    ctx.losses.extend(vocab_losses);

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
                let name = ns.def_key(c.as_str());
                // First writer wins for a given class name; bodies are identical
                // per shape so this only matters if two shapes target the same
                // class (last one would otherwise clobber). Keep deterministic by
                // not overwriting an existing identical-by-construction entry.
                defs.entry(name).or_insert_with(|| body.clone());
            }
        }
    }

    // The shared statement-metadata fragment.
    defs.insert("Annotation".to_owned(), annotation_def());

    let class_names: Vec<String> = defs
        .keys()
        .filter(|k| *k != "Annotation")
        .cloned()
        .collect();
    // `class_names` is already sorted because `defs` is a BTree-ordered Map iter.

    // The `@type`-discriminated `Node` schema (closed-world enforcement):
    // a node typed `<primary>:Foo` MUST satisfy `#/$defs/Foo`. Inserted AFTER
    // `class_names` is snapshotted so `Node` itself is never treated as a class
    // branch.
    defs.insert("Node".to_owned(), node_def(&class_names, ns));

    // Value-vocabulary enum `$defs` are inserted AFTER the `class_names` snapshot
    // (mirroring `Node`) so they are never treated as `@type` class branches, yet
    // still flow into both surfaces (JSON Schema `$defs` + OpenAPI
    // `components/schemas`) via the shared `defs` map.
    for (key, def) in vocab_enums.values() {
        defs.insert(key.clone(), def.clone());
    }

    let schema = root_schema(&defs, ns);
    let openapi = openapi_doc(&defs);

    CompiledSchema {
        schema_json: to_pretty(&schema),
        openapi_json: to_pretty(&openapi),
        losses: ctx.losses,
    }
}

/// A single value-vocabulary member: its compacted CURIE (the `enum` value id),
/// its identifier-safe symbol name (`x-enum-varnames`), and its doc string
/// (`x-enum-descriptions`, empty when the member carries neither `rdfs:comment`
/// nor `rdfs:label`).
struct VocabMember {
    curie: String,
    varname: String,
    description: String,
}

/// Derive the value-vocabulary enum `$defs`: `class IRI → (enum key, $def body)`.
///
/// Enumerates value-vocabulary classes (`?V rdf:type <marker>`) and each one's
/// named individuals (`?x rdf:type V`) over the ontology graph UNIONED with the
/// shapes graph, deterministically (sorted, de-duplicated). Returns the map plus
/// any recorded losses (empty vocabulary, blank-node member). Empty when
/// `projection` is `None`.
fn value_vocab_enum_defs(
    shapes: &Shapes,
    ns: &Namespaces,
    projection: Option<&ValueVocabProjection<'_>>,
) -> (BTreeMap<String, (String, Value)>, Vec<LossRecord>) {
    let mut out: BTreeMap<String, (String, Value)> = BTreeMap::new();
    let mut losses: Vec<LossRecord> = Vec::new();
    let Some(proj) = projection else {
        return (out, losses);
    };

    let datasets: [&RdfDataset; 2] = [proj.ontology, shapes.shapes_dataset.as_ref()];
    let type_term = Term::NamedNode(NamedNode::from(rdf::TYPE));
    let marker_term = Term::NamedNode(NamedNode::from(proj.vocab.abstract_individual_type()));

    // Value-vocabulary classes = named subjects of `?V rdf:type <marker>`,
    // de-duplicated and sorted (BTreeSet) across both datasets.
    let mut class_iris: BTreeSet<String> = BTreeSet::new();
    for ds in datasets {
        for (subject, _pred, _obj) in native_quads(
            ds,
            None,
            Some(&type_term),
            Some(&marker_term),
            GraphFilter::AnyGraph,
        ) {
            if let Term::NamedNode(n) = subject {
                class_iris.insert(n.as_str().to_owned());
            }
        }
    }

    for class_iri in &class_iris {
        assert!(
            ns.is_known(class_iri),
            "json_schema: value-vocabulary class {class_iri:?} has no declared namespace \
             prefix — its `{{Local}}Enum` `$def` key and members derive from a prefix CURIE; \
             declare its prefix in the shapes document / Namespaces before marking it"
        );
        let key = format!("{}Enum", local_name(class_iri));
        let (members, member_losses) = members_of(&datasets, class_iri, ns);
        losses.extend(member_losses);
        if members.is_empty() {
            losses.push(LossRecord {
                construct: "value-vocabulary".to_owned(),
                shape_iri: class_iri.clone(),
                reason: "no named individuals; emitting an empty enum".to_owned(),
            });
        }
        out.insert(class_iri.clone(), (key, build_enum_def(&members)));
    }

    (out, losses)
}

/// Map each predicate whose `rdfs:range` is a value-vocabulary class to that
/// class's `{Local}Enum` `$def` key, scanning the ontology graph UNIONED with the
/// shapes graph (range axioms commonly live shapes-side). Empty when the
/// projection is inactive or there are no value-vocabulary classes.
///
/// A single `(?P, rdfs:range, ?O)` scan per dataset is performed — O(1) queries
/// in the number of vocab classes, rather than one scan per class — filtering
/// each matched object against `vocab_enums` by IRI string. When a predicate's
/// `rdfs:range` is declared over multiple distinct vocab classes, the class
/// with the lexicographically largest IRI wins, matching the deterministic
/// last-write-wins tiebreak of a sorted-by-`class_iri` resolution.
fn value_vocab_predicate_ranges(
    shapes: &Shapes,
    projection: Option<&ValueVocabProjection<'_>>,
    vocab_enums: &BTreeMap<String, (String, Value)>,
) -> BTreeMap<String, String> {
    let mut out: BTreeMap<String, String> = BTreeMap::new();
    let Some(proj) = projection else {
        return out;
    };
    if vocab_enums.is_empty() {
        return out;
    }
    let datasets: [&RdfDataset; 2] = [proj.ontology, shapes.shapes_dataset.as_ref()];
    let range_term = Term::NamedNode(NamedNode::from(rdfs::RANGE));
    // predicate -> (winning class_iri, enum_key); on conflict the larger class_iri wins.
    let mut winners: BTreeMap<String, (&str, &str)> = BTreeMap::new();
    for ds in datasets {
        for (subject, _pred, object) in
            native_quads(ds, None, Some(&range_term), None, GraphFilter::AnyGraph)
        {
            let Term::NamedNode(p) = subject else {
                continue;
            };
            let Term::NamedNode(o) = &object else {
                continue;
            };
            let Some((class_iri, (enum_key, _def))) = vocab_enums.get_key_value(o.as_str()) else {
                continue;
            };
            winners
                .entry(p.as_str().to_owned())
                .and_modify(|winner| {
                    if class_iri.as_str() > winner.0 {
                        *winner = (class_iri.as_str(), enum_key.as_str());
                    }
                })
                .or_insert((class_iri.as_str(), enum_key.as_str()));
        }
    }
    for (predicate, (_class_iri, enum_key)) in winners {
        out.insert(predicate, enum_key.to_owned());
    }
    out
}

/// Enumerate the named individuals of `class_iri` (`?x rdf:type class_iri`) across
/// the datasets, as sorted, de-duplicated [`VocabMember`]s. Blank-node members are
/// dropped WITH a recorded loss (never silently). Members are ordered by CURIE.
fn members_of(
    datasets: &[&RdfDataset; 2],
    class_iri: &str,
    ns: &Namespaces,
) -> (Vec<VocabMember>, Vec<LossRecord>) {
    let type_term = Term::NamedNode(NamedNode::from(rdf::TYPE));
    let class_term = Term::NamedNode(NamedNode::from(class_iri));

    // De-duplicate member IRIs (sorted) and blank-node labels (sorted) across both
    // datasets before shaping, so the output is order-independent of scan order.
    let mut member_iris: BTreeSet<String> = BTreeSet::new();
    let mut blank_labels: BTreeSet<String> = BTreeSet::new();
    for ds in datasets {
        for (subject, _pred, _obj) in native_quads(
            ds,
            None,
            Some(&type_term),
            Some(&class_term),
            GraphFilter::AnyGraph,
        ) {
            match subject {
                Term::NamedNode(n) => {
                    member_iris.insert(n.as_str().to_owned());
                }
                Term::BlankNode(label) => {
                    blank_labels.insert(label);
                }
                Term::Literal(_) | Term::Triple(_) => {}
            }
        }
    }

    let losses: Vec<LossRecord> = blank_labels
        .into_iter()
        .map(|label| LossRecord {
            construct: "value-vocabulary member".to_owned(),
            shape_iri: class_iri.to_owned(),
            reason: format!("blank-node individual _:{label} cannot be an enum member; dropped"),
        })
        .collect();

    let mut members: Vec<VocabMember> = member_iris
        .iter()
        .map(|iri| VocabMember {
            curie: ns.compact_iri(iri),
            varname: local_name(iri),
            description: member_description(datasets, iri),
        })
        .collect();
    // Order by the `enum` value (CURIE); dedup identical CURIEs.
    members.sort_by(|a, b| a.curie.cmp(&b.curie));
    members.dedup_by(|a, b| a.curie == b.curie);
    (members, losses)
}

/// The doc string for a value-vocabulary member: the canonically-first
/// `rdfs:comment`, falling back to the canonically-first `rdfs:label`, else empty.
///
/// "Canonically first" = sort all candidate lexical values and take the smallest,
/// so selection is independent of dataset/interner iteration order (determinism).
fn member_description(datasets: &[&RdfDataset; 2], member_iri: &str) -> String {
    first_literal(datasets, member_iri, rdfs::COMMENT)
        .or_else(|| first_literal(datasets, member_iri, rdfs::LABEL))
        .unwrap_or_default()
}

/// The canonically-first (sorted-smallest) literal object of `(subject, predicate)`
/// across the datasets, or `None` when there is no literal value.
fn first_literal(
    datasets: &[&RdfDataset; 2],
    subject_iri: &str,
    predicate: &str,
) -> Option<String> {
    let subject = Term::NamedNode(NamedNode::from(subject_iri));
    let pred = Term::NamedNode(NamedNode::from(predicate));
    let mut values: BTreeSet<String> = BTreeSet::new();
    for ds in datasets {
        for (_subject, _pred, obj) in
            native_quads(ds, Some(&subject), Some(&pred), None, GraphFilter::AnyGraph)
        {
            if let Term::Literal(lit) = obj {
                values.insert(lit.value().to_owned());
            }
        }
    }
    values.into_iter().next()
}

/// Build a value-vocabulary enum `$def`. The flat `enum` keyword is ALWAYS the
/// load-bearing form (mainstream codegen turns a JSON-Schema `enum` array into a
/// native enum type); member metadata rides the parallel `x-enum-varnames` /
/// `x-enum-descriptions` arrays, aligned index-for-index to the sorted `enum`.
fn build_enum_def(members: &[VocabMember]) -> Value {
    let enum_values: Vec<Value> = members.iter().map(|m| json!({ "@id": m.curie })).collect();
    let varnames: Vec<Value> = members
        .iter()
        .map(|m| Value::String(m.varname.clone()))
        .collect();

    let mut def = Map::new();
    def.insert("enum".to_owned(), Value::Array(enum_values));
    def.insert("x-enum-varnames".to_owned(), Value::Array(varnames));
    // Only emit descriptions when at least one member carries a doc, so a
    // metadata-free vocabulary stays a bare `enum` (no empty parallel array).
    if members.iter().any(|m| !m.description.is_empty()) {
        let descriptions: Vec<Value> = members
            .iter()
            .map(|m| Value::String(m.description.clone()))
            .collect();
        def.insert("x-enum-descriptions".to_owned(), Value::Array(descriptions));
    }
    def.insert(
        "$comment".to_owned(),
        Value::String("projection of an open value vocabulary (anchor, not a fence)".to_owned()),
    );
    Value::Object(def)
}

/// Enforce that the value-vocabulary enum `$def` keys are collision-free
/// (build-time, fail-closed): no two value-vocabulary classes share a
/// `{Local}Enum` key (cross-namespace local-name twins), and no enum key clashes
/// with a class `$def` key.
fn assert_value_vocab_enum_keys_unambiguous(
    shapes: &Shapes,
    ns: &Namespaces,
    vocab_enums: &BTreeMap<String, (String, Value)>,
) {
    // The class `$def` keys (every active target class), which enum keys must not clash with.
    let mut class_keys: BTreeSet<String> = BTreeSet::new();
    for shape in &shapes.node_shapes {
        if shape.deactivated {
            continue;
        }
        for target in &shape.targets {
            if let Target::Class(c) = target {
                class_keys.insert(ns.def_key(c.as_str()));
            }
        }
    }

    let mut key_to_class: BTreeMap<String, String> = BTreeMap::new();
    for (class_iri, (key, _def)) in vocab_enums {
        assert!(
            !class_keys.contains(key),
            "json_schema: value-vocabulary enum key {key:?} (from {class_iri}) collides with a \
             class `$def` key — disambiguate before projecting"
        );
        if let Some(prev) = key_to_class.insert(key.clone(), class_iri.clone()) {
            assert_eq!(
                &prev, class_iri,
                "json_schema: distinct value-vocabulary classes share the enum `$def` key \
                 {key:?} ({prev} vs {class_iri}) — cross-namespace local-name twins; \
                 disambiguate before projecting"
            );
        }
    }
}

/// Enforce the keying precondition (Gap D): every active `sh:targetClass`
/// is in a DECLARED namespace (so [`Namespaces::def_key`] yields a stable
/// `$defs` key and [`node_def`] can rebuild its `@type` const) and those keys
/// are collision-free. Panics with a descriptive message otherwise
/// (build-time, fail-closed).
fn assert_target_class_keys_are_unambiguous(shapes: &Shapes, ns: &Namespaces) {
    let mut key_to_iri: BTreeMap<String, String> = BTreeMap::new();
    for shape in &shapes.node_shapes {
        if shape.deactivated {
            continue;
        }
        for target in &shape.targets {
            if let Target::Class(c) = target {
                let iri = c.as_str();
                assert!(
                    ns.is_known(iri),
                    "json_schema: sh:targetClass {iri:?} has no declared namespace prefix — \
                     the @type discriminator and `$defs` keys derive from a prefix CURIE; \
                     declare its prefix in the shapes document / Namespaces (and confirm \
                     OpenAPI key encoding) before introducing target classes from it"
                );
                let key = ns.def_key(iri);
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
/// `@type` (closed-world enforcement).
fn root_schema(defs: &Map<String, Value>, ns: &Namespaces) -> Value {
    let node_ref = json!({ "$ref": "#/$defs/Node" });

    // The @graph envelope object: every member is a discriminated Node. The
    // envelope branch REQUIRES `@graph`, so a bare single-node document cannot
    // slip through this permissive branch and escape `Node` discrimination — a
    // bare node must satisfy the `node_ref` branch of the root `anyOf` instead
    // (closed-world: a bare incomplete node is rejected).
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
        "$id": format!("{}schema/instance.schema.json", ns.primary_ns()),
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

/// The `@type`-discriminated match for a single class CURIE: a node whose
/// `@type` is (or contains) `curie`, either as a bare string or an array member.
///
/// This is the SOLE source of the identity model. `node_def` uses it as the
/// `if` of each positive per-class conditional, and the `sh:not` negation
/// (`compile_negand`) negates it (`{"not": type_discriminator(..)}`), so a
/// negation rejects EXACTLY the nodes the positive path would type-match — the
/// two can never drift.
fn type_discriminator(curie: &str) -> Value {
    json!({
        "required": ["@type"],
        "properties": {
            "@type": {
                "anyOf": [
                    { "const": curie },
                    { "type": "array", "contains": { "const": curie } }
                ]
            }
        }
    })
}

/// Build the `@type`-discriminated `Node` schema.
///
/// A node carries `@id`/`@type`/`@annotation` permissively, then an `allOf` of
/// per-class conditionals (sorted by class name for determinism). Each entry
/// reads: *if* `@type` includes the class CURIE — `<primary>:<Class>` for a
/// primary-namespace class, or the full `prefix:<Class>` for any other declared
/// namespace (e.g. `logic:FormalizationCandidate`) — as a bare string OR an
/// array member, *then* the node MUST satisfy that class's `#/$defs` body.
///
/// Closed-world semantics:
/// * An instance typed `<primary>:Foo` that is MISSING a required property
///   triggers Foo's `then` (`#/$defs/Foo`), fails Foo's `required`, and is
///   REJECTED.
/// * A node typed only by an UNMODELED class (no `$def`) fires no `if`, so no
///   `then` applies and it stays permissively allowed — keeping the slice
///   example sweep (Task 6) green on unmodeled types.
fn node_def(class_names: &[String], ns: &Namespaces) -> Value {
    // class_names arrives sorted (BTree-ordered defs iter); keep it explicit so
    // the conditional list is deterministic regardless of caller.
    let mut sorted: Vec<&String> = class_names.iter().collect();
    sorted.sort();

    let conditionals: Vec<Value> = sorted
        .iter()
        .map(|name| {
            // A `$defs` key carrying a `:` is already a CURIE (a non-primary
            // class, e.g. `logic:FormalizationCandidate`); a colon-free key is a
            // bare primary local name and takes the primary prefix. Either way
            // the `@type` const matches the compact IRI an instance node carries.
            let type_const = if name.contains(':') {
                (*name).clone()
            } else {
                format!("{}:{name}", ns.primary_prefix)
            };
            json!({
                "if": type_discriminator(&type_const),
                "then": { "$ref": format!("#/$defs/{name}") }
            })
        })
        .collect();

    json!({
        "type": "object",
        "title": "A single discriminated PURRDF instance node",
        "description": format!(
            "Validated by @type: a node typed {}:Foo MUST satisfy #/$defs/Foo (closed-world). Nodes typed only by unmodeled classes are permissively allowed.",
            ns.primary_prefix
        ),
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

// ── The `@annotation` fragment (statement metadata) ─────────────────────

/// The shared `$defs/Annotation` object schema: free-form statement metadata.
///
/// Permissive on purpose — future work tightens it. Values may be node refs
/// (`{"@id":..}`), scalars, or typed literals (`{"@value":..,"@type":..}`).
fn annotation_def() -> Value {
    json!({
        "type": "object",
        "title": "RDF-1.2 statement metadata (reifier annotation)",
        "description": "Free-form metadata about an asserted triple (e.g. meta:accordingTo, meta:confidence, meta:assertedAt). Permissive.",
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
fn compile_object_schema(shape: &Shape, ctx: &mut Ctx<'_>) -> Value {
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
    // Each entry: compacted JSON key → (raw predicate IRI, merged constraints).
    // The raw IRI is retained so the value-vocabulary `rdfs:range` lookup can key
    // by predicate IRI (the compacted key would lose the namespace).
    let mut by_key: BTreeMap<String, (String, Vec<Constraint>)> = BTreeMap::new();
    for ps in &shape.property_shapes {
        // Only direct predicate paths shape outgoing JSON properties; inverse
        // and composite paths (sequence/alternative/closures) are skipped with
        // a note.
        let Path::Predicate(pred) = &ps.path else {
            comments.push(
                "a non-predicate-path property shape was skipped (only direct predicate paths constrain outgoing JSON properties)".to_owned(),
            );
            continue;
        };
        let key = ctx.ns.compact_iri(pred.as_str());
        by_key
            .entry(key)
            .or_insert_with(|| (pred.as_str().to_owned(), Vec::new()))
            .1
            .extend(ps.constraints.iter().cloned());
    }

    for (key, (pred_iri, constraints)) in &by_key {
        let (value_schema, is_required) =
            compile_property(constraints, &shape_iri, key, pred_iri, ctx);
        if is_required {
            required.push(key.clone());
        }
        properties.insert(key.clone(), value_schema);
    }

    // ── Node-level constraints ──
    let mut all_of: Vec<Value> = Vec::new();
    let mut any_of: Vec<Value> = Vec::new();
    let mut one_of: Vec<Value> = Vec::new();
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
            Constraint::Not(inner) => match compile_negand(inner, ctx) {
                // The inner is losslessly expressible as the conjunction
                // `A ∧ B ∧ …` of its parts. SHACL's `sh:not` conforms iff a node
                // fails AT LEAST ONE conjunct — i.e. `¬(A ∧ B ∧ …)`, the negation
                // of the whole conjunction (De Morgan: `¬A ∨ ¬B ∨ …`), NOT each
                // conjunct negated independently (`¬A ∧ ¬B`, which is strictly
                // too narrow and false-rejects a node failing only one conjunct).
                // A single part negates directly; several parts negate their
                // `allOf`. Multiple SEPARATE `sh:not` constraints (the
                // `owl:AllDisjointClasses` projection) each still emit their own
                // independent `{"not": …}` entry, so they never clobber one
                // another.
                Some(mut parts) => {
                    let negand = if parts.len() == 1 {
                        parts.pop().expect("len == 1")
                    } else {
                        json!({ "allOf": parts })
                    };
                    all_of.push(json!({ "not": negand }));
                }
                // Negative position is unsound to widen: an inner constraint the
                // object projection would silently drop turns `not` into a negation
                // of the permissive base — rejecting every node. Record the loss
                // and emit no `not` (the SHACL/JSON-Schema Option-2 outcome).
                None => {
                    ctx.record(
                        "sh:not",
                        &shape_iri,
                        "sh:not inner shape is not losslessly expressible in the \
                         object projection; omitted rather than emitting a \
                         base-negating (vacuous) not",
                    );
                    comments.push(
                        "a node-level sh:not was dropped (inner shape not soundly \
                         expressible in the object projection)"
                            .to_owned(),
                    );
                }
            },
            Constraint::Closed { ignored } => {
                additional_properties_false = true;
                for n in ignored {
                    closed_ignored.push(ctx.ns.compact_iri(n.as_str()));
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
            Constraint::Expression { .. } => {
                ctx.record(
                    "sh:expression",
                    &shape_iri,
                    "SHACL-AF expression constraint has no JSON Schema equivalent",
                );
                comments.push(
                    "a node-level sh:expression constraint was dropped (no JSON Schema equivalent)"
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

    if !comments.is_empty() {
        comments.sort();
        comments.dedup();
        obj.insert("$comment".to_owned(), json!(comments.join("; ")));
    }

    Value::Object(obj)
}

// ── Per-property value schema ────────────────────────────────────────────────

/// Whether a value-level constraint is an EXACT complement under negation — the
/// only case in which a `sh:not` property inner may be emitted rather than
/// routed to a recorded loss.
///
/// Reduced to `sh:minCount` ALONE. Cardinality PRESENCE (`sh:minCount >= 1`) is
/// the sole value-level constraint whose positive projection is an exact
/// complement of the SHACL constraint under negation: it is expressed purely as
/// `required`, with NO value schema to widen. "Key present" is logically
/// equivalent to "the property has >= 1 value", so negating it is exactly "key
/// absent" ⟺ "< 1 value" — the SHACL complement.
///
/// Every VALUE constraint (`sh:datatype`, `sh:nodeKind`, `sh:in`,
/// `sh:languageIn`, `sh:pattern`, `sh:minLength`/`sh:maxLength`, the numeric
/// bounds `sh:minInclusive`/`sh:maxInclusive`/`sh:minExclusive`/`sh:maxExclusive`,
/// `sh:hasValue`) is deliberately excluded, because its positive projection
/// through [`compile_property`] is NOT an exact complement:
/// * Array/type vacuity — `sh:pattern`, the length bounds, the numeric bounds
///   and `sh:in` (a bare `enum` keyword) project to a JSON-Schema keyword that
///   applies only to its target primitive and passes vacuously for every other
///   type. For an array-valued node the scalar alternative is vacuously
///   satisfied, so under negation the negand widens to reject EVERY array-valued
///   node — a false-reject.
/// * Encoding mismatch — `sh:datatype`'s typed-literal object form projects an
///   instance encoding (`@type`-tagged objects) that does not line up
///   value-for-value with the negated instance, so the complement is not exact.
/// * Quantifier mismatch — `sh:hasValue` is EXISTENTIAL ("at least one value
///   equals `V`"), yet its `const` projection expresses the UNIVERSAL reading
///   ("p absent, OR every value equals `V`"). Negating the universal projection
///   is not negating the existential constraint; the two coincide only for a
///   single-value node, not in general.
///
/// So node-identity (`sh:class` → the shared `@type` [`type_discriminator`]) and
/// property presence (`sh:minCount >= 1` → `required`) are the ONLY two exact
/// projections under negation. Any other value constraint MUST route to a
/// recorded `sh:not` loss rather than an unsound `not`.
fn negand_value_constraint_ok(c: &Constraint) -> bool {
    matches!(c, Constraint::MinCount(_))
}

/// Losslessly project the inner shape of a `sh:not` into the list of AND-conjunct
/// schemas — the parts whose conjunction `A ∧ B ∧ …` IS the inner (a node
/// conforms to the inner iff it satisfies EVERY part) — or `None` if any part of
/// the inner cannot be expressed exactly.
///
/// This function only returns the inner's conjuncts; it does NOT negate them.
/// The CALLER negates their conjunction as a whole (`¬(A ∧ B ∧ …)`), which is the
/// correct SHACL `sh:not` semantics — negating each part independently would be
/// strictly too narrow.
///
/// SOUND BY CONSTRUCTION: this is the polarity contract for negation. In negative
/// position every silently-dropped constraint *widens* the negand, so a single
/// unhandled construct turns the caller's assembled `{"not": …}` into a negation
/// of the permissive "any node" base — rejecting every node. This projector
/// therefore returns `Some(parts)` ONLY when it can express the whole inner, and
/// `None` (⇒ record a loss, emit no `not`) otherwise. It never drops-and-negates.
///
/// Expressible parts:
/// * node-level `sh:class X` ⇒ [`type_discriminator`] on `X`'s CURIE — but only
///   when `X` is in a declared namespace; an undeclared-namespace class would
///   compact to a full IRI that no instance's compacted `@type` can match (a
///   silently never-firing negation), so it is a loss instead.
/// * a direct-predicate `sh:property` whose ONLY constraints are `sh:minCount`
///   (with at least one n >= 1) ⇒ a `{properties, required}` conjunct expressing
///   pure PRESENCE via `required` (see [`negand_value_constraint_ok`]). A
///   property inner carrying ANY value constraint routes to a loss instead — no
///   value projection is an exact complement under negation.
///
/// Everything else (`sh:nodeKind`/`sh:datatype`/`sh:in`/… at node level, nested
/// `sh:not`/`sh:and`/`sh:or`/`sh:xone`/`sh:node`, non-predicate paths, nested or
/// reifier property shapes, an empty inner) ⇒ `None`.
fn compile_negand(inner: &Shape, ctx: &mut Ctx<'_>) -> Option<Vec<Value>> {
    // An empty inner is the permissive base; negating it rejects everything.
    // Never emit a vacuous negation for it.
    if inner.constraints.is_empty() && inner.property_shapes.is_empty() {
        return None;
    }

    let mut parts: Vec<Value> = Vec::new();

    // Node-level constraints: only `sh:class` is a losslessly-expressible
    // node-identity discriminator; anything else is not expressible in the
    // object projection and forces the whole negation to a loss.
    for c in &inner.constraints {
        let Constraint::Class(class) = c else {
            return None;
        };
        if !ctx.ns.is_known(class.as_str()) {
            return None;
        }
        parts.push(type_discriminator(&ctx.ns.compact_iri(class.as_str())));
    }

    // Property shapes: express only direct-predicate paths whose every value
    // constraint round-trips through `compile_property`.
    let inner_iri = inner.id.to_string();
    for ps in &inner.property_shapes {
        if ps.deactivated
            || !ps.property_shapes.is_empty()
            || !ps.reifier_shapes.is_empty()
            || ps.reification_required
        {
            return None;
        }
        let Path::Predicate(pred) = &ps.path else {
            return None;
        };
        if !ps.constraints.iter().all(negand_value_constraint_ok) {
            return None;
        }
        let key = ctx.ns.compact_iri(pred.as_str());
        // Presence-only negand (constraints are all `negand_value_constraint_ok`,
        // i.e. `sh:minCount`): the value-vocabulary `rdfs:range` enrichment is a
        // POSITIVE value projection and MUST NOT widen a negand (it would make the
        // emitted `not` reject the wrong nodes). Pass an empty predicate IRI so the
        // range lookup never fires here.
        let (value_schema, is_required) =
            compile_property(&ps.constraints, &inner_iri, &key, "", ctx);
        // A property conjunct that neither requires the key nor restricts its
        // value matches every node (the permissive base); negating it would
        // reject all — the vacuous-not bug. Route such a bare/zero-cardinality
        // inner (only `sh:minCount 0`, or an empty constraint set) to a loss.
        let restricts_value = ps
            .constraints
            .iter()
            .any(|c| !matches!(c, Constraint::MinCount(_)));
        if !is_required && !restricts_value {
            return None;
        }
        let mut props: Map<String, Value> = Map::new();
        props.insert(key.clone(), value_schema);
        let mut obj: Map<String, Value> = Map::new();
        obj.insert("type".to_owned(), json!("object"));
        obj.insert("properties".to_owned(), Value::Object(props));
        if is_required {
            obj.insert("required".to_owned(), json!([key]));
        }
        parts.push(Value::Object(obj));
    }

    if parts.is_empty() {
        return None;
    }

    // Deterministic order, independent of constraint-vector position.
    // The canonical sorter serializes each part once instead of on every
    // comparison.
    crate::term::sort_canonical(&mut parts);
    Some(parts)
}

/// Compile one property shape's constraints into `(value_schema, is_required)`.
///
/// `value_schema` already accounts for cardinality: a single value when
/// `sh:maxCount 1`, otherwise an `array` wrapper with `minItems`/`maxItems`.
fn compile_property(
    constraints: &[Constraint],
    shape_iri: &str,
    key: &str,
    pred_iri: &str,
    ctx: &mut Ctx<'_>,
) -> (Value, bool) {
    // The "scalar" value schema (a single value, pre-cardinality).
    let mut value: Map<String, Value> = Map::new();
    // anyOf alternatives accumulated across datatype/class/nodekind constraints.
    let mut alts: Vec<Value> = Vec::new();
    let mut enum_values: Vec<Value> = Vec::new();
    let mut comments: Vec<String> = Vec::new();

    let mut min_count: Option<u64> = None;
    let mut max_count: Option<u64> = None;

    // Value-vocabulary precedence (E4): whether ANY `sh:class` was present, and
    // the enum key emitted by a vocab-resolving `sh:class` (if any). Drives
    // whether the `rdfs:range` enum `$ref` applies after the loop.
    let mut had_any_class = false;
    let mut class_enum_key: Option<String> = None;

    for c in constraints {
        match c {
            Constraint::MinCount(n) => min_count = Some(*n),
            Constraint::MaxCount(n) => max_count = Some(*n),
            Constraint::Datatype(dt) => {
                alts.push(datatype_value_schema(dt.as_str()));
            }
            Constraint::Class(c) => {
                had_any_class = true;
                // A `sh:class` pointing at a value-vocabulary class resolves to the
                // enum `$ref` (projection-only), tightening the open node-ref to the
                // anchor set WITHOUT touching the validating shape.
                if let Some(enum_key) = ctx.value_vocab_enums.get(c.as_str()) {
                    alts.push(json!({ "$ref": format!("#/$defs/{enum_key}") }));
                    class_enum_key = Some(enum_key.clone());
                    continue;
                }
                // Resolve/emit by `def_key` — the SAME key the `$defs` map
                // (built in `compile`) and `emitted_defs` (PASS 1, above) use.
                // A bare local name would conflate a primary class with any
                // non-primary class sharing its local name; `def_key` keeps
                // them distinct (bare local name for the primary namespace,
                // full CURIE — e.g. `math:Distribution` — for any other
                // declared namespace), so the ref written here always matches
                // an existing `$defs` entry or falls back to a permissive
                // node-ref, never a dangling `$ref`.
                let def_key = ctx.ns.def_key(c.as_str());
                let has_def = ctx.emitted_defs.contains(&def_key);
                if ctx.ns.is_primary(c.as_str()) {
                    if has_def {
                        // The class has a NodeShape ⇒ a `$def` is emitted for it.
                        // Object property: a node ref OR the class `$ref`.
                        alts.push(node_ref_schema());
                        alts.push(json!({ "$ref": format!("#/$defs/{def_key}") }));
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
                                    "{} has no NodeShape; node reference only",
                                    ctx.ns.compact_iri(c.as_str())
                                )),
                            );
                        }
                        alts.push(node_ref);
                    }
                } else if has_def {
                    // Non-primary class WITH a NodeShape ⇒ it DOES have a `$def`
                    // (keyed by its full CURIE — see `Namespaces::def_key`), so
                    // resolve to it exactly like a primary class, just under the
                    // CURIE key (e.g. `#/$defs/math:Distribution`). A JSON
                    // pointer reference segment does not need to escape `:`, so
                    // this pointer form is valid without further encoding.
                    alts.push(node_ref_schema());
                    alts.push(json!({ "$ref": format!("#/$defs/{def_key}") }));
                } else {
                    // External (non-primary) class with NO NodeShape: the value
                    // is still an RDF node reference, never a string literal, but
                    // a `$ref` would dangle (no `$def` for it), so emit the
                    // permissive node-reference form that accepts an `@id`
                    // object, keeping a `$comment` noting the external class.
                    let mut node_ref = node_ref_schema();
                    if let Value::Object(map) = &mut node_ref {
                        map.insert(
                            "$comment".to_owned(),
                            json!(format!("external class {}", c.as_str())),
                        );
                    }
                    alts.push(node_ref);
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
                    enum_values.push(json!(term_enum_value(t, ctx.ns)));
                }
            }
            Constraint::HasValue(v) => {
                value.insert("const".to_owned(), term_const_value(v, ctx.ns));
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
            Constraint::Expression { .. } => {
                ctx.record(
                    "sh:expression",
                    shape_iri,
                    "SHACL-AF expression constraint has no JSON Schema equivalent",
                );
                comments.push(format!(
                    "a sh:expression constraint on property {key} was dropped (no JSON Schema equivalent)"
                ));
            }
            Constraint::Not(_) => {
                // A value-position `sh:not` has no lossless value-schema
                // projection here. Surface it (never swallow it): emitting a
                // negation off a base value schema would be vacuous, exactly the
                // node-level bug this change removes.
                ctx.record(
                    "sh:not",
                    shape_iri,
                    "property-level sh:not has no lossless value-schema projection",
                );
                comments.push(format!(
                    "a sh:not constraint on property {key} was dropped (no lossless value-schema projection)"
                ));
            }
            // Counts handled above; node-shape-only constraints (Closed/And/…)
            // do not appear on a property shape's value schema.
            _ => {}
        }
    }

    // Value-vocabulary `rdfs:range` path (E4 precedence): an explicit `sh:class`
    // always wins. Only a property with NO `sh:class` takes the range-derived enum
    // `$ref`; a `sh:class` that coexists with a value-vocabulary range records a
    // conflict loss (the range enum is suppressed).
    if let Some(range_key) = ctx.predicate_ranges.get(pred_iri).cloned() {
        match (&class_enum_key, had_any_class) {
            (Some(class_key), _) => {
                // A vocab `sh:class` already emitted its enum `$ref`; it wins.
                if *class_key != range_key {
                    ctx.record(
                        "rdfs:range",
                        shape_iri,
                        &format!(
                            "predicate {pred_iri} has a value-vocabulary sh:class ({class_key}) \
                             and a conflicting value-vocabulary rdfs:range ({range_key}); \
                             sh:class wins, range enum suppressed"
                        ),
                    );
                }
            }
            (None, true) => {
                // A non-vocab `sh:class` is present; it wins, range enum suppressed.
                ctx.record(
                    "rdfs:range",
                    shape_iri,
                    &format!(
                        "predicate {pred_iri} has a sh:class and a value-vocabulary \
                         rdfs:range ({range_key}); sh:class wins, range enum suppressed"
                    ),
                );
            }
            (None, false) => {
                // Open vocabulary (no sh:class): the enum `$ref` is the value schema.
                alts.push(json!({ "$ref": format!("#/$defs/{range_key}") }));
            }
        }
    }

    if !enum_values.is_empty() {
        crate::term::sort_canonical(&mut enum_values);
        enum_values.dedup();
        value.insert("enum".to_owned(), Value::Array(enum_values));
    }

    if !alts.is_empty() {
        // Stable order, de-duplicated.
        crate::term::sort_canonical(&mut alts);
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
        if let Some(n) = min_count
            && n > 0
        {
            arr.insert("minItems".to_owned(), json!(n));
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
    if let Ok(n) = lex.parse::<f64>()
        && let Some(num) = serde_json::Number::from_f64(n)
    {
        value.insert(key.to_owned(), Value::Number(num));
        return;
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

/// The `sh:in` enum member value, matching EXACTLY what the projector emits for a
/// value of the same term.
///
/// Delegates to [`crate::instance::project_value`] — the single source of the
/// value encoding — so an `sh:in` member and the projected instance value it
/// constrains can never drift. In particular an IRI member is `{"@id": curie}`
/// (NOT a bare CURIE string): a projected node value is the object form, so a
/// bare-string enum would reject the very data the shape accepts.
fn term_enum_value(term: &Term, ns: &Namespaces) -> Value {
    crate::instance::project_value(term, ns)
}

/// The `sh:hasValue` const value (projected form).
fn term_const_value(term: &Term, ns: &Namespaces) -> Value {
    match term {
        Term::NamedNode(n) => json!({ "@id": ns.compact_iri(n.as_str()) }),
        Term::Literal(lit) => {
            if let Some(lang) = lit.language() {
                json!({ "@value": lit.value(), "@language": lang })
            } else {
                let dt = lit.datatype();
                if dt.as_str() == RDF_LANG_STRING || dt.as_str() == XSD_STRING {
                    Value::String(lit.value().to_owned())
                } else {
                    json!({ "@value": lit.value(), "@type": ns.compact_iri(dt.as_str()) })
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
        @prefix meta: <https://example.org/meta/> .
    ";

    /// The fixture namespace table: `meta` primary, plus the `logic` prefix
    /// the cross-namespace tests declare. Nothing is hardcoded in library code
    /// — these are DECLARED here exactly as a shapes document would.
    fn fixture_ns() -> Namespaces {
        Namespaces::new(
            "meta",
            &[
                ("meta".to_owned(), "https://example.org/meta/".to_owned()),
                (
                    "logic".to_owned(),
                    "https://blackcatinformatics.ca/logic/".to_owned(),
                ),
            ],
        )
        .expect("fixture namespaces are valid")
    }

    fn compile_ttl(body: &str) -> CompiledSchema {
        let ttl = format!("{PREFIXES}{body}");
        let dataset = crate::text_ingest::parse_turtle_to_dataset(&ttl).expect("Turtle parse");
        let shapes = from_dataset(&dataset).expect("shape parse");
        compile(&shapes, &fixture_ns())
    }

    fn schema_of(c: &CompiledSchema) -> Value {
        serde_json::from_str(&c.schema_json).expect("schema is valid JSON")
    }

    fn def<'a>(schema: &'a Value, name: &str) -> &'a Value {
        &schema["$defs"][name]
    }

    /// Validate a JSON-LD instance node against the emitted `schema_json` with a
    /// trusted external JSON-Schema (draft 2020-12) validator, returning whether
    /// the instance is ACCEPTED.
    ///
    /// This is the production-surface observation the acceptance criteria demand:
    /// it exercises the exact string `CompiledSchema.schema_json` the way a
    /// downstream consumer (e.g. gmeow-ontology) would, rather than asserting the
    /// schema's JSON shape.
    fn validates(schema_json: &str, instance: &Value) -> bool {
        use boon::{Compiler, Schemas};
        let schema_val: Value = serde_json::from_str(schema_json).expect("schema is valid JSON");
        let loc = "mem:///instance.schema.json";
        let mut schemas = Schemas::new();
        let mut compiler = Compiler::new();
        compiler
            .add_resource(loc, schema_val)
            .expect("schema registers as a boon resource");
        let sch = compiler
            .compile(loc, &mut schemas)
            .expect("emitted schema compiles under draft 2020-12");
        schemas.validate(instance, sch).is_ok()
    }

    #[test]
    #[should_panic(expected = "declare its prefix in the shapes document / Namespaces")]
    fn unknown_namespace_target_class_hard_fails() {
        // A target class from an UNDECLARED namespace has no prefix CURIE to key
        // its `$defs`/discriminator by; the keying guard must reject it loudly
        // (Gap D). A DECLARED non-primary prefix (e.g. logic:) is accepted —
        // see `logic_target_class_keyed_by_curie`.
        compile_ttl(
            r"
            @prefix ex: <https://example.org/> .
            ex:PersonShape a sh:NodeShape ;
                sh:targetClass ex:Person ;
                sh:property [ sh:path meta:name ; sh:minCount 1 ] .
        ",
        );
    }

    #[test]
    fn logic_target_class_keyed_by_curie() {
        // A non-primary but KNOWN-prefix target class (logic:) keys its `$defs` body
        // by the full CURIE and discriminates `@type` on that same CURIE, so a
        // closed-world logic node is enforced exactly like a primary-namespace node.
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
        let ns = fixture_ns();
        assert_eq!(
            ns.compact_iri("https://example.org/meta/Person"),
            "meta:Person"
        );
        // Builtins are always merged in, even though the fixture never declares xsd.
        assert_eq!(
            ns.compact_iri("http://www.w3.org/2001/XMLSchema#integer"),
            "xsd:integer"
        );
        assert_eq!(
            ns.compact_iri("http://example.org/Foo"),
            "http://example.org/Foo"
        );
        assert_eq!(local_name("https://example.org/meta/Person"), "Person");
        assert_eq!(
            local_name("http://www.w3.org/2001/XMLSchema#integer"),
            "integer"
        );
    }

    #[test]
    fn test_namespaces_primary_and_def_key() {
        let ns = fixture_ns();
        assert!(ns.is_primary("https://example.org/meta/Person"));
        assert!(!ns.is_primary("https://blackcatinformatics.ca/logic/Claim"));
        assert_eq!(ns.primary_ns(), "https://example.org/meta/");
        // Primary → bare local name; other declared → CURIE; undeclared → full IRI.
        assert_eq!(ns.def_key("https://example.org/meta/Person"), "Person");
        assert_eq!(
            ns.def_key("https://blackcatinformatics.ca/logic/FormalizationCandidate"),
            "logic:FormalizationCandidate"
        );
        assert_eq!(
            ns.def_key("http://example.org/Foo"),
            "http://example.org/Foo"
        );
    }

    #[test]
    fn test_namespaces_unresolved_primary_prefix_is_err() {
        let err = Namespaces::new("gmeow", &[]).expect_err("undeclared primary must fail");
        assert!(
            err.contains("gmeow") && err.contains("doc_prefixes"),
            "error must name the prefix and the fix, got {err:?}"
        );
    }

    #[test]
    fn test_namespaces_doc_declaration_wins_over_builtin() {
        // A document may rebind a builtin prefix name; the document wins.
        let ns = Namespaces::new(
            "xsd",
            &[(
                "xsd".to_owned(),
                "https://example.org/custom-xsd/".to_owned(),
            )],
        )
        .expect("rebound builtin");
        assert_eq!(ns.primary_ns(), "https://example.org/custom-xsd/");
        assert_eq!(
            ns.compact_iri("https://example.org/custom-xsd/int"),
            "xsd:int"
        );
        // The other builtins are still merged in.
        assert_eq!(
            ns.compact_iri("http://www.w3.org/ns/shacl#NodeShape"),
            "sh:NodeShape"
        );
    }

    #[test]
    fn test_namespaces_context_object_carries_all_prefixes() {
        let ctx = fixture_ns().context_object();
        assert_eq!(ctx.get("meta"), Some(&json!("https://example.org/meta/")));
        assert_eq!(
            ctx.get("logic"),
            Some(&json!("https://blackcatinformatics.ca/logic/"))
        );
        assert_eq!(ctx.get("xsd"), Some(&json!(XSD_NS)));
        assert_eq!(ctx.get("sh"), Some(&json!(SH_NS)));
    }

    #[test]
    fn non_purrdf_primary_namespace_compiles() {
        // The downstream (gmeow) call pattern: a completely different primary
        // namespace, declared by the caller — nothing namespace-specific remains
        // in the emitter's keying, discrimination, or `$id`.
        let ttl = r"
            @prefix sh:    <http://www.w3.org/ns/shacl#> .
            @prefix xsd:   <http://www.w3.org/2001/XMLSchema#> .
            @prefix gmeow: <https://blackcatinformatics.ca/gmeow/> .
            gmeow:CatShape a sh:NodeShape ;
                sh:targetClass gmeow:Cat ;
                sh:property [ sh:path gmeow:name ; sh:minCount 1 ; sh:maxCount 1 ; sh:datatype xsd:string ] .
        ";
        let dataset = crate::text_ingest::parse_turtle_to_dataset(ttl).expect("Turtle parse");
        let shapes = from_dataset(&dataset).expect("shape parse");
        let ns = Namespaces::new(
            "gmeow",
            &[(
                "gmeow".to_owned(),
                "https://blackcatinformatics.ca/gmeow/".to_owned(),
            )],
        )
        .expect("gmeow namespaces");
        let schema = schema_of(&compile(&shapes, &ns));

        // Primary-namespace class keys its $def by bare local name.
        assert!(
            def(&schema, "Cat").is_object(),
            "gmeow class keyed by local name"
        );
        // The $id derives from the primary namespace.
        assert_eq!(
            schema["$id"],
            json!("https://blackcatinformatics.ca/gmeow/schema/instance.schema.json")
        );
        // The Node discriminator fires on the gmeow-prefixed @type const.
        let conds = def(&schema, "Node")["allOf"]
            .as_array()
            .expect("Node allOf");
        assert!(
            conds.iter().any(|c| {
                c["then"]["$ref"] == "#/$defs/Cat"
                    && c["if"]["properties"]["@type"]["anyOf"][0]["const"] == "gmeow:Cat"
            }),
            "Node must discriminate gmeow:Cat"
        );
        // The property key compacts through the declared prefix.
        assert!(def(&schema, "Cat")["properties"]["gmeow:name"].is_object());
    }

    #[test]
    fn external_object_class_projects_to_node_ref_not_string() {
        // Soundness: a value constrained by `sh:class` to an EXTERNAL (non-primary)
        // class is always an RDF node reference (`@id`), never a string literal.
        // The old projection emitted `{"type":"string"}`, which REJECTS valid
        // `{"@id":..}` instance data. It must project to a permissive node ref
        // (no dangling `$ref`, since external classes have no `$def`) while a
        // genuine `xsd:string` datatype value keeps its string projection.
        let schema = schema_of(&compile_ttl(
            r"
            @prefix math: <https://blackcatinformatics.ca/math/> .
            meta:BasisShape a sh:NodeShape ;
                sh:targetClass meta:BasisHolder ;
                sh:property [ sh:path meta:basis ; sh:maxCount 1 ; sh:class math:Basis ] ;
                sh:property [ sh:path meta:label ; sh:maxCount 1 ; sh:datatype xsd:string ] .
        ",
        ));
        let basis = &def(&schema, "BasisHolder")["properties"]["meta:basis"];
        // NOT the unsound string projection.
        assert_ne!(
            basis["type"],
            json!("string"),
            "external object class must NOT project to a string: {basis}"
        );
        // A permissive node reference: an object requiring `@id`.
        assert_eq!(
            basis["type"],
            json!("object"),
            "external object class projects to a node-ref object: {basis}"
        );
        assert!(
            basis["properties"]["@id"].is_object(),
            "node ref carries an @id property: {basis}"
        );
        assert!(
            basis["required"]
                .as_array()
                .is_some_and(|r| r.iter().any(|v| v == "@id")),
            "node ref requires @id: {basis}"
        );
        // The external-class provenance comment is preserved.
        assert!(
            basis["$comment"]
                .as_str()
                .is_some_and(|s| s.contains("external class")),
            "external-class comment preserved: {basis}"
        );
        // No dangling $ref to a nonexistent external $def.
        assert!(
            basis["$ref"].is_null(),
            "external class must NOT emit a dangling $ref: {basis}"
        );

        // Production surface: the value schema ACCEPTS an @id object and REJECTS a
        // bare string, verified with a trusted draft-2020-12 validator.
        let basis_json = serde_json::to_string(basis).expect("value schema serializes");
        assert!(
            validates(
                &basis_json,
                &json!({ "@id": "https://blackcatinformatics.ca/x" })
            ),
            "external object class must accept an @id node reference"
        );
        assert!(
            !validates(&basis_json, &json!("https://blackcatinformatics.ca/x")),
            "external object class must reject a bare string literal"
        );

        // Guard against over-correction: a genuine xsd:string datatype value still
        // projects to a string (a bare-scalar string alternative), and the value
        // schema still accepts a plain string literal.
        let label = &def(&schema, "BasisHolder")["properties"]["meta:label"];
        assert!(
            label["anyOf"]
                .as_array()
                .is_some_and(|alts| alts.iter().any(|a| a["type"] == json!("string"))),
            "datatype xsd:string still projects to a string alternative: {label}"
        );
        let label_json = serde_json::to_string(label).expect("value schema serializes");
        assert!(
            validates(&label_json, &json!("hello")),
            "datatype xsd:string must still accept a bare string literal"
        );
    }

    #[test]
    fn test_required_from_min_count_and_array_vs_single() {
        let c = compile_ttl(
            r"
            meta:PersonShape a sh:NodeShape ;
                sh:targetClass meta:Person ;
                sh:property [ sh:path meta:name ; sh:minCount 1 ; sh:maxCount 1 ; sh:datatype xsd:string ] ;
                sh:property [ sh:path meta:nickname ; sh:datatype xsd:string ] .
            ",
        );
        let schema = schema_of(&c);
        let person = def(&schema, "Person");
        // required contains meta:name (minCount 1)
        let required = person["required"].as_array().expect("required array");
        assert!(required.iter().any(|v| v == "meta:name"));
        // name (maxCount 1) is a single value, NOT an array
        let name = &person["properties"]["meta:name"];
        assert_ne!(name["type"], json!("array"), "maxCount 1 → single value");
        // nickname (no maxCount, minCount<=1) accepts BOTH the bare single form
        // AND the array form: the projector emits a bare scalar for a single
        // value and an array only for multiple values, so the schema must accept
        // either or it would reject SHACL-conformant single-value data.
        let nickname = &person["properties"]["meta:nickname"];
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
            meta:EventShape a sh:NodeShape ;
                sh:targetClass meta:Event ;
                sh:property [ sh:path meta:at ; sh:maxCount 1 ; sh:datatype xsd:dateTime ] ;
                sh:property [ sh:path meta:count ; sh:maxCount 1 ; sh:datatype xsd:integer ] .
            ",
        );
        let schema = schema_of(&c);
        let event = def(&schema, "Event");
        // dateTime → anyOf containing {type:string, format:date-time}
        let at = &event["properties"]["meta:at"];
        let at_alts = at["anyOf"].as_array().expect("anyOf");
        assert!(
            at_alts
                .iter()
                .any(|alt| alt["format"] == json!("date-time"))
        );
        // integer → anyOf containing {type:integer}
        let count = &event["properties"]["meta:count"];
        let count_alts = count["anyOf"].as_array().expect("anyOf");
        assert!(count_alts.iter().any(|alt| alt["type"] == json!("integer")));
    }

    #[test]
    fn test_enum_from_sh_in() {
        let c = compile_ttl(
            r#"
            meta:ColorShape a sh:NodeShape ;
                sh:targetClass meta:Color ;
                sh:property [ sh:path meta:value ; sh:maxCount 1 ; sh:in ( "red" "green" "blue" ) ] .
            "#,
        );
        let schema = schema_of(&c);
        let value = &def(&schema, "Color")["properties"]["meta:value"];
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
    fn sh_in_over_iris_uses_id_object_members() {
        // Regression: a `sh:in` list of IRIs must project members as `{"@id":curie}`
        // objects (the projector's encoding), NOT bare CURIE strings — otherwise a
        // projected instance value `{"@id":..}` fails the enum it should satisfy.
        let schema = schema_of(&compile_ttl(
            r"
            meta:StateShape a sh:NodeShape ;
                sh:targetClass meta:State ;
                sh:property [ sh:path meta:kind ; sh:maxCount 1 ;
                              sh:in ( meta:open meta:closed ) ] .
        ",
        ));
        assert_eq!(
            def(&schema, "State")["properties"]["meta:kind"]["enum"],
            json!([{ "@id": "meta:closed" }, { "@id": "meta:open" }]),
            "IRI sh:in members are {{\"@id\":curie}} objects, sorted"
        );
    }

    #[test]
    fn sh_in_over_iris_round_trips_through_projection() {
        // The production-surface proof: an instance projected via the instance
        // projector VALIDATES against the `sh:in`-derived schema. Before the fix
        // the bare-string enum rejected the projected `{"@id":..}` value.
        let ttl = format!(
            "{PREFIXES}\
             meta:StateShape a sh:NodeShape ;\n\
                 sh:targetClass meta:State ;\n\
                 sh:property [ sh:path meta:kind ; sh:maxCount 1 ;\n\
                               sh:in ( meta:open meta:closed ) ] .\n\
             meta:s1 a meta:State ; meta:kind meta:open ."
        );
        let dataset = crate::text_ingest::parse_turtle_to_dataset(&ttl).expect("parse");
        let shapes = from_dataset(&dataset).expect("shapes");
        let compiled = compile(&shapes, &fixture_ns());
        let node = crate::instance::project_subject(&dataset, &fixture_ns(), &meta_term("s1"));
        assert!(
            validates(&compiled.schema_json, &node),
            "a projected instance must validate against its own sh:in enum; projected = {node}"
        );
    }

    #[test]
    fn test_pattern() {
        let c = compile_ttl(
            r#"
            meta:CodeShape a sh:NodeShape ;
                sh:targetClass meta:Code ;
                sh:property [ sh:path meta:code ; sh:maxCount 1 ; sh:pattern "^[A-Z]+$" ] .
            "#,
        );
        let schema = schema_of(&c);
        let code = &def(&schema, "Code")["properties"]["meta:code"];
        assert_eq!(code["pattern"], json!("^[A-Z]+$"));
    }

    #[test]
    fn test_closed_additional_properties_false() {
        let c = compile_ttl(
            r"
            meta:ClosedShape a sh:NodeShape ;
                sh:targetClass meta:Sealed ;
                sh:closed true ;
                sh:ignoredProperties ( rdf:type ) ;
                sh:property [ sh:path meta:only ; sh:maxCount 1 ; sh:datatype xsd:string ] .
            ",
        );
        let schema = schema_of(&c);
        let sealed = def(&schema, "Sealed");
        assert_eq!(sealed["additionalProperties"], json!(false));
        // The single declared property key is present.
        assert!(sealed["properties"]["meta:only"].is_object());
    }

    #[test]
    fn test_not_class_accepts_own_instance() {
        // The reported reproduction: PersonShape carries
        // `sh:not [ sh:class meta:Organization ]`. A node typed only meta:Person
        // MUST be accepted by $defs/Person; a node ALSO typed meta:Organization
        // MUST be rejected (the constraint is preserved, not silently dropped).
        let c = compile_ttl(
            r"
            meta:PersonShape a sh:NodeShape ;
                sh:targetClass meta:Person ;
                sh:not [ sh:class meta:Organization ] .
            meta:OrganizationShape a sh:NodeShape ;
                sh:targetClass meta:Organization .
            ",
        );
        // Structure: Person's def carries an `allOf` negation of the
        // Organization @type discriminator — not a base-negating `not`.
        let schema = schema_of(&c);
        let person = def(&schema, "Person");
        let all_of = person["allOf"].as_array().expect("Person has an allOf");
        assert!(
            all_of
                .iter()
                .any(|e| e["not"]["properties"]["@type"]["anyOf"][0]["const"]
                    == json!("meta:Organization")),
            "expected an allOf `not` over the meta:Organization @type discriminator, got {all_of:?}"
        );

        // Behaviour, on the production surface (validate against schema_json):
        assert!(
            validates(&c.schema_json, &json!({ "@type": "meta:Person" })),
            "a node typed only meta:Person must be ACCEPTED"
        );
        assert!(
            !validates(
                &c.schema_json,
                &json!({ "@type": ["meta:Person", "meta:Organization"] })
            ),
            "a node also typed meta:Organization must be REJECTED"
        );
    }

    #[test]
    fn test_not_class_multi_disjoint() {
        // The owl:AllDisjointClasses projection: one shape carries SEVERAL
        // `sh:not [ sh:class … ]`. Each must become its own independent `allOf`
        // negation (no clobber), and an instance of the shape's own class is
        // accepted while an instance also carrying either disjoint class is
        // rejected.
        let c = compile_ttl(
            r"
            meta:PersonShape a sh:NodeShape ;
                sh:targetClass meta:Person ;
                sh:not [ sh:class meta:Organization ] ;
                sh:not [ sh:class meta:Event ] .
            meta:OrganizationShape a sh:NodeShape ;
                sh:targetClass meta:Organization .
            meta:EventShape a sh:NodeShape ;
                sh:targetClass meta:Event .
            ",
        );
        let schema = schema_of(&c);
        let person = def(&schema, "Person");
        let negated: Vec<Value> = person["allOf"]
            .as_array()
            .expect("Person has an allOf")
            .iter()
            .filter(|e| e.get("not").is_some())
            .map(|e| e["not"]["properties"]["@type"]["anyOf"][0]["const"].clone())
            .collect();
        assert!(
            negated.contains(&json!("meta:Organization")) && negated.contains(&json!("meta:Event")),
            "both disjoint classes must be negated independently, got {negated:?}"
        );

        assert!(
            validates(&c.schema_json, &json!({ "@type": "meta:Person" })),
            "a plain meta:Person must be ACCEPTED"
        );
        assert!(
            !validates(
                &c.schema_json,
                &json!({ "@type": ["meta:Person", "meta:Event"] })
            ),
            "a meta:Person also typed meta:Event must be REJECTED"
        );
    }

    #[test]
    fn test_not_multi_conjunct_inner() {
        // A SINGLE `sh:not` whose inner is a multi-conjunct shape
        // (`sh:class meta:X ; sh:class meta:Y`). The inner is `X ∧ Y`, so
        // `sh:not` conforms iff a node fails AT LEAST ONE conjunct
        // (`¬(X ∧ Y)`), NOT `¬X ∧ ¬Y`. A node typed only meta:X must be
        // ACCEPTED (it fails the Y conjunct); a node typed both must be
        // REJECTED.
        let c = compile_ttl(
            r"
            meta:PersonShape a sh:NodeShape ;
                sh:targetClass meta:Person ;
                sh:not [ sh:class meta:X ; sh:class meta:Y ] .
            meta:XShape a sh:NodeShape ;
                sh:targetClass meta:X .
            meta:YShape a sh:NodeShape ;
                sh:targetClass meta:Y .
            ",
        );
        let schema = schema_of(&c);
        let person = def(&schema, "Person");
        let all_of = person["allOf"].as_array().expect("Person has an allOf");
        // Exactly ONE `{"not": {"allOf": […]}}` entry for this inner — the
        // conjunction is negated as a whole, NOT split into two independent
        // `{"not": …}` entries.
        let whole_conjunction_nots = all_of
            .iter()
            .filter(|e| e.get("not").and_then(|n| n.get("allOf")).is_some())
            .count();
        assert_eq!(
            whole_conjunction_nots, 1,
            "expected exactly one `not` over the inner's `allOf`, got {all_of:?}"
        );
        // No independent per-conjunct `{"not": {"properties": {"@type": …}}}`
        // entries were emitted for this inner.
        let independent_type_nots = all_of
            .iter()
            .filter(|e| e["not"]["properties"]["@type"].is_object())
            .count();
        assert_eq!(
            independent_type_nots, 0,
            "the multi-conjunct inner must NOT be split into per-conjunct negations, got {all_of:?}"
        );

        // Behaviour on the production surface. The `sh:not` lives in the Person
        // body, so instances must be typed meta:Person for it to apply (matching
        // the sibling `test_not_*` tests). The ADDED types drive the inner.
        assert!(
            validates(
                &c.schema_json,
                &json!({ "@type": ["meta:Person", "meta:X"] })
            ),
            "a node typed only meta:X (not meta:Y) fails the meta:Y conjunct, so \
             `not(X ∧ Y)` must ACCEPT it"
        );
        assert!(
            !validates(
                &c.schema_json,
                &json!({ "@type": ["meta:Person", "meta:X", "meta:Y"] })
            ),
            "a node typed both meta:X and meta:Y satisfies the inner, so \
             `not(X ∧ Y)` must REJECT it"
        );

        // A mixed class + property-shape inner: `X ∧ (has meta:p)`. `sh:not`
        // conforms iff a node fails at least one conjunct.
        let c2 = compile_ttl(
            r"
            meta:ThingShape a sh:NodeShape ;
                sh:targetClass meta:Thing ;
                sh:not [ sh:class meta:X ;
                         sh:property [ sh:path meta:p ; sh:minCount 1 ] ] .
            meta:XShape a sh:NodeShape ;
                sh:targetClass meta:X .
            ",
        );
        assert!(
            validates(
                &c2.schema_json,
                &json!({ "@type": ["meta:Thing", "meta:X"] })
            ),
            "typed meta:X but WITHOUT meta:p fails the property conjunct, so it \
             must be ACCEPTED"
        );
        assert!(
            !validates(
                &c2.schema_json,
                &json!({ "@type": ["meta:Thing", "meta:X"], "meta:p": "v" })
            ),
            "typed meta:X and WITH meta:p satisfies the inner, so it must be \
             REJECTED"
        );
    }

    #[test]
    fn test_not_mixed_inner_records_loss() {
        // Rb1: an inner mixing `sh:class` with an off-allowlist node constraint
        // (`sh:nodeKind`) is NOT losslessly expressible. The whole `sh:not` must
        // become a recorded loss with no `not` emitted — the vacuous-negation bug
        // must NOT reappear via a "structural" fallback.
        let c = compile_ttl(
            r"
            meta:PersonShape a sh:NodeShape ;
                sh:targetClass meta:Person ;
                sh:not [ sh:class meta:Organization ; sh:nodeKind sh:IRI ] .
            ",
        );
        assert!(
            c.losses.iter().any(|l| l.construct == "sh:not"),
            "a mixed inner must record a sh:not LossRecord, got {:?}",
            c.losses
        );
        let schema = schema_of(&c);
        let person = def(&schema, "Person");
        assert!(
            person.get("allOf").is_none() && person.get("not").is_none(),
            "no negation may be emitted for a non-expressible mixed inner, got {person:?}"
        );
        // The bug does not reappear: a plain Person still validates.
        assert!(
            validates(&c.schema_json, &json!({ "@type": "meta:Person" })),
            "a plain meta:Person must still be ACCEPTED (no vacuous rejection)"
        );
    }

    #[test]
    fn test_not_undeclared_namespace_class_records_loss() {
        // F6: an inner `sh:class` in an UNDECLARED namespace compacts to a full
        // IRI that no instance's compacted @type can match — a silently
        // never-firing negation. It must be a recorded loss instead, never an
        // emitted (never-matching) const.
        let c = compile_ttl(
            r"
            @prefix ex: <https://example.org/> .
            meta:PersonShape a sh:NodeShape ;
                sh:targetClass meta:Person ;
                sh:not [ sh:class <http://unknown.example/Foo> ] .
            ",
        );
        assert!(
            c.losses.iter().any(|l| l.construct == "sh:not"),
            "an undeclared-namespace inner class must record a sh:not LossRecord, got {:?}",
            c.losses
        );
        let schema = schema_of(&c);
        let person = def(&schema, "Person");
        assert!(
            person.get("allOf").is_none() && person.get("not").is_none(),
            "no never-matching negation may be emitted, got {person:?}"
        );
    }

    #[test]
    fn test_not_structural_property_preserved() {
        // An expressible property-shape inner keeps a SOUND structural negation:
        // `sh:not [ sh:property [ sh:path meta:p ; sh:minCount 1 ] ]` rejects
        // nodes that HAVE meta:p and accepts nodes that lack it.
        let c = compile_ttl(
            r"
            meta:NoPShape a sh:NodeShape ;
                sh:targetClass meta:NoP ;
                sh:not [ sh:property [ sh:path meta:p ; sh:minCount 1 ] ] .
            ",
        );
        assert!(
            !c.losses.iter().any(|l| l.construct == "sh:not"),
            "an expressible property-shape inner must NOT record a loss, got {:?}",
            c.losses
        );
        let schema = schema_of(&c);
        let nop = def(&schema, "NoP");
        assert!(
            nop["allOf"]
                .as_array()
                .is_some_and(|a| a.iter().any(|e| e.get("not").is_some())),
            "expected a structural `not` negation, got {nop:?}"
        );
        assert!(
            validates(&c.schema_json, &json!({ "@type": "meta:NoP" })),
            "a node WITHOUT meta:p must be ACCEPTED"
        );
        assert!(
            !validates(
                &c.schema_json,
                &json!({ "@type": "meta:NoP", "meta:p": "x" })
            ),
            "a node WITH meta:p must be REJECTED"
        );
    }

    #[test]
    fn test_not_property_maxcount_records_loss() {
        // F1 (the vacuous-total false-reject, reintroduced via the property
        // path): a `sh:not`
        // property inner whose ONLY constraint is `sh:maxCount` is NOT faithfully
        // negatable — `compile_property` widens a bare maxCount to the permissive
        // base, so emitting a `not` would reject EVERY node of the target class (a
        // vacuous-total false-reject). It must route to a recorded loss instead.
        let c = compile_ttl(
            r"
            meta:MaxShape a sh:NodeShape ;
                sh:targetClass meta:MaxT ;
                sh:not [ sh:property [ sh:path meta:p ; sh:maxCount 1 ] ] .
            ",
        );
        assert!(
            c.losses.iter().any(|l| l.construct == "sh:not"),
            "a bare-maxCount sh:not property inner must record a sh:not LossRecord, got {:?}",
            c.losses
        );
        let schema = schema_of(&c);
        let maxt = def(&schema, "MaxT");
        assert!(
            !serde_json::to_string(maxt)
                .expect("def serializes")
                .contains("\"not\""),
            "the vacuous-total `not` must be GONE from the def, got {maxt:?}"
        );
        // Behavioural: the sh:not is honestly DROPPED (a recorded loss), so the
        // total false-reject is gone — every node of the target class is accepted
        // regardless of how many values it has for meta:p.
        assert!(
            validates(
                &c.schema_json,
                &json!({ "@type": "meta:MaxT", "meta:p": ["a", "b"] })
            ),
            "a node with 2 values for meta:p must be ACCEPTED (sh:not[maxCount 1] must accept it)"
        );
        assert!(
            validates(
                &c.schema_json,
                &json!({ "@type": "meta:MaxT", "meta:p": "a" })
            ),
            "a node with 1 value for meta:p must be ACCEPTED"
        );
        assert!(
            validates(&c.schema_json, &json!({ "@type": "meta:MaxT" })),
            "a node with 0 values for meta:p must be ACCEPTED"
        );
    }

    #[test]
    fn test_not_property_mincount_maxcount_records_loss() {
        // `sh:not [ sh:property [ sh:path meta:p ; sh:minCount 1 ; sh:maxCount 1 ] ]`:
        // the maxCount upper bound is not faithfully expressible under negation
        // (the `required` presence alone would reject nodes that HAVE the key,
        // which is unsound — SHACL's inner `1..1` fails for a node with 2 values,
        // so sh:not must ACCEPT it). Must route to a recorded loss, not a `not`.
        let c = compile_ttl(
            r"
            meta:ExactShape a sh:NodeShape ;
                sh:targetClass meta:ExactT ;
                sh:not [ sh:property [ sh:path meta:p ; sh:minCount 1 ; sh:maxCount 1 ] ] .
            ",
        );
        assert!(
            c.losses.iter().any(|l| l.construct == "sh:not"),
            "a minCount+maxCount sh:not property inner must record a sh:not LossRecord, got {:?}",
            c.losses
        );
        let schema = schema_of(&c);
        let exact = def(&schema, "ExactT");
        assert!(
            !serde_json::to_string(exact)
                .expect("def serializes")
                .contains("\"not\""),
            "the vacuous `not` must be GONE from the def, got {exact:?}"
        );
        assert!(
            validates(
                &c.schema_json,
                &json!({ "@type": "meta:ExactT", "meta:p": ["a", "b"] })
            ),
            "a node with 2 values for meta:p must be ACCEPTED (no false-reject)"
        );
    }

    #[test]
    fn test_not_property_datatype_records_loss() {
        // A value-restriction inner is NOT an exact complement under negation:
        // `sh:datatype`'s positive projection through `compile_property` widens
        // (typed-literal object form; array-type vacuity), so it routes to a
        // recorded loss rather than an unsound `not`.
        // `sh:not [ sh:property [ sh:path meta:p ; sh:datatype xsd:string ] ]`.
        let c = compile_ttl(
            r"
            meta:DtShape a sh:NodeShape ;
                sh:targetClass meta:DtT ;
                sh:not [ sh:property [ sh:path meta:p ; sh:datatype xsd:string ] ] .
            ",
        );
        assert!(
            c.losses.iter().any(|l| l.construct == "sh:not"),
            "a sh:datatype sh:not property inner must record a sh:not LossRecord, got {:?}",
            c.losses
        );
        let schema = schema_of(&c);
        let dtt = def(&schema, "DtT");
        assert!(
            !serde_json::to_string(dtt)
                .expect("def serializes")
                .contains("\"not\""),
            "no `not` must be emitted for a dropped value-restriction negand, got {dtt:?}"
        );
        // The constraint is honestly DROPPED (recorded loss): a node WITH the
        // property is ACCEPTED regardless of value type — no false-reject.
        assert!(
            validates(
                &c.schema_json,
                &json!({ "@type": "meta:DtT", "meta:p": "hello" })
            ),
            "a node whose p is a string must be ACCEPTED (constraint dropped)"
        );
        assert!(
            validates(&c.schema_json, &json!({ "@type": "meta:DtT", "meta:p": 5 })),
            "a node whose p is a non-string must be ACCEPTED (constraint dropped)"
        );
    }

    #[test]
    fn test_not_property_datatype_array_records_loss() {
        // The array cardinality path is likewise routed to a loss — no value
        // projection of `sh:datatype` is an exact complement under negation, so
        // an array-valued node must not be false-rejected.
        // `sh:not [ sh:property [ sh:path meta:p ; sh:datatype xsd:string ] ]`.
        let c = compile_ttl(
            r"
            meta:DtArrShape a sh:NodeShape ;
                sh:targetClass meta:DtArrT ;
                sh:not [ sh:property [ sh:path meta:p ; sh:datatype xsd:string ] ] .
            ",
        );
        assert!(
            c.losses.iter().any(|l| l.construct == "sh:not"),
            "a sh:datatype sh:not property inner must record a sh:not LossRecord, got {:?}",
            c.losses
        );
        let schema = schema_of(&c);
        let dtt = def(&schema, "DtArrT");
        assert!(
            !serde_json::to_string(dtt)
                .expect("def serializes")
                .contains("\"not\""),
            "no `not` must be emitted for a dropped value-restriction negand, got {dtt:?}"
        );
        // Both an all-string array and a mixed array must be ACCEPTED (dropped).
        assert!(
            validates(
                &c.schema_json,
                &json!({ "@type": "meta:DtArrT", "meta:p": ["a", "b"] })
            ),
            "an all-string array node must be ACCEPTED (constraint dropped)"
        );
        assert!(
            validates(
                &c.schema_json,
                &json!({ "@type": "meta:DtArrT", "meta:p": ["a", 5] })
            ),
            "a mixed array node must be ACCEPTED (constraint dropped)"
        );
    }

    #[test]
    fn test_not_property_nodekind_records_loss() {
        // A `sh:nodeKind` value restriction is not an exact complement under
        // negation (its `type`-tagged alternatives widen), so it routes to a
        // recorded loss: a node WITH the property is ACCEPTED.
        let c = compile_ttl(
            r"
            meta:NkShape a sh:NodeShape ;
                sh:targetClass meta:NkT ;
                sh:not [ sh:property [ sh:path meta:p ; sh:nodeKind sh:Literal ] ] .
            ",
        );
        assert!(
            c.losses.iter().any(|l| l.construct == "sh:not"),
            "a sh:nodeKind sh:not property inner must record a sh:not LossRecord, got {:?}",
            c.losses
        );
        let schema = schema_of(&c);
        let nk = def(&schema, "NkT");
        assert!(
            !serde_json::to_string(nk)
                .expect("def serializes")
                .contains("\"not\""),
            "no `not` must be emitted for a dropped value-restriction negand, got {nk:?}"
        );
        assert!(
            validates(
                &c.schema_json,
                &json!({ "@type": "meta:NkT", "meta:p": "x" })
            ),
            "a node with p must be ACCEPTED (constraint dropped)"
        );
    }

    #[test]
    fn test_not_property_languagein_records_loss() {
        // A `sh:languageIn` value restriction is not an exact complement under
        // negation, so it routes to a recorded loss: a node WITH the property is
        // ACCEPTED.
        let c = compile_ttl(
            r#"
            meta:LiShape a sh:NodeShape ;
                sh:targetClass meta:LiT ;
                sh:not [ sh:property [ sh:path meta:p ; sh:languageIn ( "en" ) ] ] .
            "#,
        );
        assert!(
            c.losses.iter().any(|l| l.construct == "sh:not"),
            "a sh:languageIn sh:not property inner must record a sh:not LossRecord, got {:?}",
            c.losses
        );
        let schema = schema_of(&c);
        let li = def(&schema, "LiT");
        assert!(
            !serde_json::to_string(li)
                .expect("def serializes")
                .contains("\"not\""),
            "no `not` must be emitted for a dropped value-restriction negand, got {li:?}"
        );
        assert!(
            validates(
                &c.schema_json,
                &json!({ "@type": "meta:LiT", "meta:p": "x" })
            ),
            "a node with p must be ACCEPTED (constraint dropped)"
        );
    }

    #[test]
    fn test_not_property_pattern_records_loss() {
        // F1 (value-restriction variant): `sh:pattern` projects a BARE
        // JSON-Schema `pattern` keyword with no `type` guard. JSON Schema applies
        // `pattern` only to strings and passes non-strings vacuously, so under
        // negation the scalar alternative is vacuously satisfied by an
        // array-valued node — widening the negand to false-reject EVERY array.
        // It must route to a recorded loss and emit NO `not`.
        let c = compile_ttl(
            r#"
            meta:PatShape a sh:NodeShape ;
                sh:targetClass meta:PatT ;
                sh:not [ sh:property [ sh:path meta:p ; sh:pattern "^A" ] ] .
            "#,
        );
        assert!(
            c.losses.iter().any(|l| l.construct == "sh:not"),
            "a sh:pattern sh:not property inner must record a sh:not LossRecord, got {:?}",
            c.losses
        );
        let schema = schema_of(&c);
        let pat = def(&schema, "PatT");
        assert!(
            !serde_json::to_string(pat)
                .expect("def serializes")
                .contains("\"not\""),
            "the vacuous `not` must be GONE from the def, got {pat:?}"
        );
        // Behavioural (boon): the false-reject is gone — the constraint is
        // honestly dropped, so every array-valued node is ACCEPTED.
        assert!(
            validates(
                &c.schema_json,
                &json!({ "@type": "meta:PatT", "meta:p": ["Apple", "Banana"] })
            ),
            "a node with one violating value must be ACCEPTED (no false-reject)"
        );
        assert!(
            validates(
                &c.schema_json,
                &json!({ "@type": "meta:PatT", "meta:p": ["Apple"] })
            ),
            "a node whose values all match ^A must be ACCEPTED (loss dropped)"
        );
        assert!(
            validates(
                &c.schema_json,
                &json!({ "@type": "meta:PatT", "meta:p": ["Banana"] })
            ),
            "a node whose value violates ^A must be ACCEPTED (no false-reject)"
        );
    }

    #[test]
    fn test_not_property_numeric_bound_records_loss() {
        // F1 (numeric-bound variant): `sh:minInclusive` projects a BARE
        // `minimum` keyword with no `type` guard; JSON Schema passes non-numbers
        // vacuously, so under negation an array-valued node is vacuously matched
        // and false-rejected. Must route to a recorded loss and emit NO `not`.
        let c = compile_ttl(
            r"
            meta:NumShape a sh:NodeShape ;
                sh:targetClass meta:NumT ;
                sh:not [ sh:property [ sh:path meta:p ; sh:minInclusive 10 ] ] .
            ",
        );
        assert!(
            c.losses.iter().any(|l| l.construct == "sh:not"),
            "a sh:minInclusive sh:not property inner must record a sh:not LossRecord, got {:?}",
            c.losses
        );
        let schema = schema_of(&c);
        let num = def(&schema, "NumT");
        assert!(
            !serde_json::to_string(num)
                .expect("def serializes")
                .contains("\"not\""),
            "the vacuous `not` must be GONE from the def, got {num:?}"
        );
        // Behavioural (boon): a node with meta:p = [5] is ACCEPTED — the
        // false-reject is gone.
        assert!(
            validates(
                &c.schema_json,
                &json!({ "@type": "meta:NumT", "meta:p": [5] })
            ),
            "a node with meta:p = [5] must be ACCEPTED (no false-reject)"
        );
    }

    #[test]
    fn test_not_property_hasvalue_records_loss() {
        // F1 (existential variant): `sh:hasValue V` is EXISTENTIAL — a node
        // conforms iff AT LEAST ONE value of p equals V. `compile_property`
        // projects it as a `const` (array `items: {const}`) conjunct, which
        // expresses the UNIVERSAL reading ("p absent, OR every value equals V").
        // Negating that universal projection is NOT negating the existential
        // constraint: it false-accepts a node carrying V alongside other values
        // and false-rejects a node with p absent. It must therefore route to a
        // recorded loss and emit NO `not`.
        let c = compile_ttl(
            r#"
            meta:HasValShape a sh:NodeShape ;
                sh:targetClass meta:Person ;
                sh:not [ sh:property [ sh:path meta:p ; sh:hasValue "boss" ] ] .
            "#,
        );
        assert!(
            c.losses.iter().any(|l| l.construct == "sh:not"),
            "a sh:hasValue sh:not property inner must record a sh:not LossRecord, got {:?}",
            c.losses
        );
        let schema = schema_of(&c);
        let person = def(&schema, "Person");
        assert!(
            !serde_json::to_string(person)
                .expect("def serializes")
                .contains("\"not\""),
            "the unsound `not` must be GONE from the def, got {person:?}"
        );
        // Behavioural (boon): the multivalue + absent cases that expose the
        // existential axis — single-value alone would NOT catch it. With the
        // constraint honestly dropped, the node is unconstrained by this sh:not,
        // so every case is ACCEPTED (no false-reject, no unsound `not`).
        assert!(
            validates(
                &c.schema_json,
                &json!({ "@type": "meta:Person", "meta:p": ["boss", "peon"] })
            ),
            "a node whose p carries \"boss\" alongside other values must be \
             ACCEPTED (was false-accepted by the unsound `not`; now a dropped loss)"
        );
        assert!(
            validates(&c.schema_json, &json!({ "@type": "meta:Person" })),
            "a node with p absent must be ACCEPTED (was false-rejected before)"
        );
        assert!(
            validates(
                &c.schema_json,
                &json!({ "@type": "meta:Person", "meta:p": ["boss"] })
            ),
            "a single-value node must be ACCEPTED (dropped loss)"
        );
    }

    #[test]
    fn test_not_property_in_records_loss() {
        // `sh:in` value restriction is not an exact complement under negation
        // (IRI-member/enum encoding mismatch; array vacuity), so it routes to a
        // recorded loss: both an in-set and an out-of-set node are ACCEPTED
        // (the constraint is honestly dropped).
        // `sh:not [ sh:property [ sh:path meta:p ; sh:in ( "a" "b" ) ] ]`.
        let c = compile_ttl(
            r#"
            meta:InShape a sh:NodeShape ;
                sh:targetClass meta:InT ;
                sh:not [ sh:property [ sh:path meta:p ; sh:in ( "a" "b" ) ] ] .
            "#,
        );
        assert!(
            c.losses.iter().any(|l| l.construct == "sh:not"),
            "a sh:in sh:not property inner must record a sh:not LossRecord, got {:?}",
            c.losses
        );
        let schema = schema_of(&c);
        let it = def(&schema, "InT");
        assert!(
            !serde_json::to_string(it)
                .expect("def serializes")
                .contains("\"not\""),
            "no `not` must be emitted for a dropped value-restriction negand, got {it:?}"
        );
        assert!(
            validates(
                &c.schema_json,
                &json!({ "@type": "meta:InT", "meta:p": "a" })
            ),
            "an in-set node must be ACCEPTED (constraint dropped)"
        );
        assert!(
            validates(
                &c.schema_json,
                &json!({ "@type": "meta:InT", "meta:p": "c" })
            ),
            "an out-of-set node must be ACCEPTED (constraint dropped)"
        );
    }

    #[test]
    fn test_not_self_disjoint() {
        // Rb4: a (contradictory) self-disjoint shape must reject its own
        // instances soundly, without panic or dangling refs.
        let c = compile_ttl(
            r"
            meta:PersonShape a sh:NodeShape ;
                sh:targetClass meta:Person ;
                sh:not [ sh:class meta:Person ] .
            ",
        );
        assert!(
            !validates(&c.schema_json, &json!({ "@type": "meta:Person" })),
            "a self-disjoint Person must reject every Person instance"
        );
    }

    #[test]
    fn test_not_class_without_nodeshape() {
        // Rb5: the negated class need not have its own NodeShape — the @type
        // discriminator needs no `$def`, so no `$ref` dangles.
        let c = compile_ttl(
            r"
            meta:PersonShape a sh:NodeShape ;
                sh:targetClass meta:Person ;
                sh:not [ sh:class meta:Ghost ] .
            ",
        );
        assert!(
            !c.losses.iter().any(|l| l.construct == "sh:not"),
            "a declared-namespace class needs no NodeShape to be negated, got {:?}",
            c.losses
        );
        assert!(
            validates(&c.schema_json, &json!({ "@type": "meta:Person" })),
            "a plain meta:Person must be ACCEPTED"
        );
        assert!(
            !validates(
                &c.schema_json,
                &json!({ "@type": ["meta:Person", "meta:Ghost"] })
            ),
            "a meta:Person also typed meta:Ghost must be REJECTED"
        );
    }

    #[test]
    fn test_not_nodekind_records_loss_no_vacuous_not() {
        // `sh:not [ sh:nodeKind sh:Literal ]` is not losslessly expressible in
        // the object projection (a JSON-LD node is always an object, never a
        // literal), so the old emitter produced a `not` over the permissive base
        // that rejected EVERY node. The sound outcome is Option 2: record a loss
        // and emit no `not`.
        let c = compile_ttl(
            r"
            meta:NotShape a sh:NodeShape ;
                sh:targetClass meta:Thing ;
                sh:not [ sh:nodeKind sh:Literal ] .
            ",
        );
        assert!(
            c.losses.iter().any(|l| l.construct == "sh:not"),
            "sh:not over a non-expressible inner must record a LossRecord, got {:?}",
            c.losses
        );
        let schema = schema_of(&c);
        let thing = def(&schema, "Thing");
        assert!(
            thing.get("not").is_none(),
            "no `not` may be emitted for a non-expressible inner, got {:?}",
            thing.get("not")
        );
        assert!(
            thing["$comment"]
                .as_str()
                .unwrap_or_default()
                .contains("sh:not"),
            "expected a $comment noting the dropped sh:not, got {:?}",
            thing.get("$comment")
        );
    }

    #[test]
    fn test_sparql_constraint_records_loss_and_comment() {
        let c = compile_ttl(
            r#"
            meta:SparqlShape a sh:NodeShape ;
                sh:targetClass meta:Guarded ;
                sh:sparql [
                    sh:select "SELECT $this WHERE { $this <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://example.org/meta/Guarded> . }" ;
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
            meta:OrgShape a sh:NodeShape ;
                sh:targetClass meta:Organization ;
                sh:property [ sh:path meta:member ; sh:maxCount 1 ; sh:class meta:Person ] .
            meta:PersonShape a sh:NodeShape ;
                sh:targetClass meta:Person .
            ",
        );
        let schema = schema_of(&c);
        let member = &def(&schema, "Organization")["properties"]["meta:member"];
        // anyOf includes a node ref {"@id":..} and a $ref to #/$defs/Person.
        let alts = member["anyOf"].as_array().expect("anyOf");
        assert!(alts.iter().any(|a| a["$ref"] == json!("#/$defs/Person")));
        assert!(alts.iter().any(|a| a["properties"]["@id"].is_object()));
    }

    /// Walk every `$ref` string reachable from `schema` and assert it resolves
    /// to an existing key under `$defs` — the general "zero dangling refs"
    /// acceptance criterion, independent of which property happened to trigger
    /// a given `$ref`.
    fn assert_no_dangling_defs_refs(schema: &Value) {
        let defs = schema["$defs"].as_object().expect("$defs is an object");
        let mut stack: Vec<&Value> = vec![schema];
        while let Some(v) = stack.pop() {
            match v {
                Value::Object(map) => {
                    if let Some(Value::String(r)) = map.get("$ref")
                        && let Some(key) = r.strip_prefix("#/$defs/")
                    {
                        assert!(
                            defs.contains_key(key),
                            "dangling $ref: {r:?} has no matching $defs entry"
                        );
                    }
                    for val in map.values() {
                        stack.push(val);
                    }
                }
                Value::Array(arr) => stack.extend(arr.iter()),
                _ => {}
            }
        }
    }

    #[test]
    fn cross_namespace_local_name_twins_never_dangle_a_ref() {
        // Regression for the PASS-1/`$defs` keying mismatch: `emitted_defs` used
        // to be keyed by bare local name while the `$defs` map (and the
        // `emitted_defs` set, after this fix) are keyed by `Namespaces::def_key`
        // — bare local name for the PRIMARY namespace, full CURIE for any other
        // declared namespace. A primary class and a non-primary class sharing a
        // local name (here `meta:Distribution` vs `logic:Distribution`, mirroring
        // the real `gmeow:Distribution` / `math:Distribution` case) used to
        // collapse to the SAME `emitted_defs` entry even though only the
        // `logic:` twin has an actual NodeShape/`$def` — so a `sh:class` ref to
        // the shape-less `meta:` twin resolved as if it had a `$def` and wrote a
        // `#/$defs/Distribution` `$ref` that DANGLED (no such entry exists,
        // since `logic:Distribution` is keyed `logic:Distribution`, not
        // `Distribution`).
        let c = compile_ttl(
            r"
            @prefix logic: <https://blackcatinformatics.ca/logic/> .
            logic:DistributionShape a sh:NodeShape ;
                sh:targetClass logic:Distribution ;
                sh:property [ sh:path logic:mean ; sh:maxCount 1 ; sh:datatype xsd:decimal ] .
            meta:HolderShape a sh:NodeShape ;
                sh:targetClass meta:Holder ;
                sh:property [ sh:path meta:primaryDist ; sh:maxCount 1 ; sh:class meta:Distribution ] ;
                sh:property [ sh:path meta:logicDist ; sh:maxCount 1 ; sh:class logic:Distribution ] .
            ",
        );
        let schema = schema_of(&c);

        // (a) Both defs are present, under their CORRECT, DISTINCT keys.
        assert!(
            def(&schema, "logic:Distribution").is_object(),
            "the non-primary class keeps its CURIE-keyed $def"
        );
        assert!(
            schema["$defs"]["Distribution"].is_null(),
            "meta:Distribution has no NodeShape of its own; it must NOT leak a bare-local-name \
             $def borrowed from its logic: twin"
        );

        // (c) A ref to the shape-less primary twin does not dangle: it must
        // project to a permissive node-ref, never a `$ref`.
        let primary_dist = &def(&schema, "Holder")["properties"]["meta:primaryDist"];
        assert!(
            primary_dist["$ref"].is_null(),
            "meta:Distribution has no $def; a ref to it must not dangle a $ref: {primary_dist}"
        );
        assert_eq!(
            primary_dist["type"],
            json!("object"),
            "meta:Distribution projects to a permissive node-ref: {primary_dist}"
        );
        assert!(
            primary_dist["$comment"]
                .as_str()
                .is_some_and(|s| s.contains("no NodeShape")),
            "expected a $comment noting the missing NodeShape: {primary_dist}"
        );

        // The non-primary twin WITH a NodeShape resolves to its CURIE-keyed
        // `$def`, exactly like a primary class resolves to its local-name-keyed
        // `$def`.
        let logic_dist = &def(&schema, "Holder")["properties"]["meta:logicDist"];
        let alts = logic_dist["anyOf"].as_array().expect("anyOf");
        assert!(
            alts.iter()
                .any(|a| a["$ref"] == json!("#/$defs/logic:Distribution")),
            "logic:Distribution has a NodeShape; its ref must resolve to its CURIE-keyed $def: \
             {logic_dist}"
        );

        // (b) ZERO dangling `#/$defs/...` refs anywhere in the compiled schema.
        assert_no_dangling_defs_refs(&schema);

        // Production surface: the emitted schema actually compiles and
        // validates end-to-end under a real draft-2020-12 validator — proving
        // the colon-bearing `$defs` key + JSON-pointer `$ref` form (already used
        // elsewhere for non-primary target classes) is tolerated here too, not
        // just structurally present.
        assert!(
            validates(
                &c.schema_json,
                &json!({
                    "@type": "meta:Holder",
                    "meta:logicDist": { "@id": "https://example.org/meta/d1" }
                })
            ),
            "a Holder referencing a shaped logic:Distribution must validate"
        );
    }

    #[test]
    fn test_annotation_def_present_and_root_envelope() {
        let c = compile_ttl(
            r"
            meta:PersonShape a sh:NodeShape ;
                sh:targetClass meta:Person ;
                sh:property [ sh:path meta:name ; sh:datatype xsd:string ] .
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
            meta:GoneShape a sh:NodeShape ;
                sh:targetClass meta:Gone ;
                sh:deactivated true ;
                sh:property [ sh:path meta:x ; sh:datatype xsd:string ] .
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
            meta:PersonShape a sh:NodeShape ;
                sh:targetClass meta:Person ;
                sh:property [ sh:path meta:name ; sh:datatype xsd:string ] .
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
                if let Some(Value::String(r)) = map.get("$ref")
                    && let Some(name) = r.strip_prefix("#/$defs/")
                {
                    out.push(name.to_owned());
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
    /// the bug guard — an object property whose `sh:class` points at a
    /// class with NO NodeShape must NOT emit a dangling `$ref`.
    #[test]
    fn every_ref_resolves() {
        // meta:Organization HAS a shape; meta:Ghost (the sh:class target of the
        // `haunts` property) has NONE — so no `$defs/Ghost` is emitted and a ref
        // to it would dangle. Also exercise sh:node (inline) and @annotation.
        let c = compile_ttl(
            r"
            meta:OrgShape a sh:NodeShape ;
                sh:targetClass meta:Organization ;
                sh:node [ sh:property [ sh:path meta:label ; sh:datatype xsd:string ] ] ;
                sh:property [ sh:path meta:member ; sh:maxCount 1 ; sh:class meta:Person ] ;
                sh:property [ sh:path meta:haunts ; sh:maxCount 1 ; sh:class meta:Ghost ] .
            meta:PersonShape a sh:NodeShape ;
                sh:targetClass meta:Person .
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
        let haunts = &def(&schema, "Organization")["properties"]["meta:haunts"];
        let comment = haunts["$comment"].as_str().unwrap_or("");
        assert!(
            comment.contains("Ghost") && comment.contains("no NodeShape"),
            "expected a node-reference-only $comment for meta:Ghost, got {haunts:?}"
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
        // A class with a required property: a node typed meta:Thing that is
        // missing meta:req must (structurally) be funnelled through Thing's
        // `then` and fail Thing's `required` — i.e. the discrimination exists and
        // Thing actually requires meta:req.
        let c = compile_ttl(
            r"
            meta:ThingShape a sh:NodeShape ;
                sh:targetClass meta:Thing ;
                sh:property [ sh:path meta:req ; sh:minCount 1 ; sh:maxCount 1 ; sh:datatype xsd:string ] .
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

        // Its `if` requires @type and matches @type == "meta:Thing" both as a
        // bare const and as an array `contains`.
        let if_clause = &thing_cond["if"];
        assert_eq!(if_clause["required"], json!(["@type"]));
        let type_alts = if_clause["properties"]["@type"]["anyOf"]
            .as_array()
            .expect("@type discrimination anyOf");
        assert!(
            type_alts.iter().any(|a| a["const"] == json!("meta:Thing")),
            "expected a bare const meta:Thing branch, got {type_alts:?}"
        );
        assert!(
            type_alts
                .iter()
                .any(|a| a["type"] == json!("array")
                    && a["contains"]["const"] == json!("meta:Thing")),
            "expected an array-contains meta:Thing branch, got {type_alts:?}"
        );

        // And Thing actually requires meta:req — so an incomplete node IS
        // rejected once routed through Thing's `then`.
        let thing = def(&schema, "Thing");
        let required = thing["required"].as_array().expect("Thing.required array");
        assert!(
            required.iter().any(|v| v == "meta:req"),
            "Thing must require meta:req, got {required:?}"
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
            meta:PersonShape a sh:NodeShape ;
                sh:targetClass meta:Person ;
                sh:property [ sh:path meta:name ; sh:minCount 1 ; sh:datatype xsd:string ] ;
                sh:property [ sh:path meta:age ; sh:maxCount 1 ; sh:datatype xsd:integer ] .
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

    // ── Value-vocabulary enum $def projection ────────────────────────────────

    /// The marker IRI a shapes document / consumer would declare for value
    /// vocabularies. Caller-supplied; nothing is minted in library code.
    const MARKER: &str = "https://blackcatinformatics.ca/logic/AbstractIndividualType";

    /// Compile `body` (prefixed) WITH the value-vocabulary projection active
    /// (marker = [`MARKER`], ontology = the same parsed dataset). The `logic:`
    /// prefix (the fixture's non-primary vocabulary namespace) is declared here so
    /// bodies can use `logic:` terms directly.
    fn compile_vocab(body: &str) -> CompiledSchema {
        let ttl =
            format!("{PREFIXES}@prefix logic: <https://blackcatinformatics.ca/logic/> .\n{body}");
        let dataset = crate::text_ingest::parse_turtle_to_dataset(&ttl).expect("Turtle parse");
        let shapes = from_dataset(&dataset).expect("shape parse");
        let vocab = ValueVocab::new(MARKER);
        let projection = ValueVocabProjection {
            vocab: &vocab,
            ontology: dataset.as_ref(),
        };
        compile_with_value_vocab(&shapes, &fixture_ns(), Some(&projection))
    }

    #[test]
    fn value_vocab_standalone_enum_def_even_when_unreferenced() {
        // R1/R2/R3: a marked class with NO referencing property still projects a
        // `{Local}Enum` $def whose members are its sorted `{"@id":curie}` individuals.
        let schema = schema_of(&compile_vocab(
            r"
            logic:TermStability a logic:AbstractIndividualType .
            logic:stable       a logic:TermStability .
            logic:experimental a logic:TermStability .
            logic:deprecated   a logic:TermStability .
        ",
        ));
        assert_eq!(
            def(&schema, "TermStabilityEnum")["enum"],
            json!([
                { "@id": "logic:deprecated" },
                { "@id": "logic:experimental" },
                { "@id": "logic:stable" },
            ]),
            "members are the sorted individuals, encoded as {{\"@id\":curie}} objects"
        );
    }

    #[test]
    fn value_vocab_enum_is_load_bearing_with_parallel_metadata_arrays() {
        // E1/E2: the `enum` keyword is ALWAYS present (never oneOf/const); member
        // symbol names ride `x-enum-varnames` and docs ride `x-enum-descriptions`,
        // aligned index-for-index to the sorted `enum` (rdfs:comment wins over label).
        let schema = schema_of(&compile_vocab(
            r#"
            logic:TermStability a logic:AbstractIndividualType .
            logic:stable       a logic:TermStability ; rdfs:label "Stable label" ; rdfs:comment "A stable term." .
            logic:experimental a logic:TermStability ; rdfs:comment "Experimental term." .
            logic:deprecated   a logic:TermStability .
        "#,
        ));
        let enum_def = def(&schema, "TermStabilityEnum");
        assert!(
            enum_def["enum"].is_array(),
            "the `enum` keyword is load-bearing"
        );
        assert!(
            enum_def.get("oneOf").is_none() && enum_def.get("const").is_none(),
            "a value-vocabulary $def never emits oneOf/const"
        );
        assert_eq!(
            enum_def["x-enum-varnames"],
            json!(["deprecated", "experimental", "stable"]),
            "varnames = sorted member local names, aligned to the enum"
        );
        assert_eq!(
            enum_def["x-enum-descriptions"],
            json!(["", "Experimental term.", "A stable term."]),
            "descriptions parallel to enum; comment wins over label; empty where absent"
        );
    }

    #[test]
    fn value_vocab_metadata_free_omits_descriptions() {
        // A vocabulary whose members carry no doc emits `enum` + `x-enum-varnames`
        // but NO `x-enum-descriptions` (no empty parallel array).
        let schema = schema_of(&compile_vocab(
            r"
            logic:Color a logic:AbstractIndividualType .
            logic:red   a logic:Color .
            logic:green a logic:Color .
        ",
        ));
        let enum_def = def(&schema, "ColorEnum");
        assert!(enum_def["enum"].is_array());
        assert!(enum_def["x-enum-varnames"].is_array());
        assert!(
            enum_def.get("x-enum-descriptions").is_none(),
            "no descriptions array when no member has a doc"
        );
    }

    #[test]
    fn value_vocab_enum_def_appears_in_both_surfaces() {
        // The enum $def (with its x-enum-* arrays) mirrors identically into the
        // OpenAPI `components/schemas` surface.
        let compiled = compile_vocab(
            r#"
            logic:Color a logic:AbstractIndividualType .
            logic:red   a logic:Color ; rdfs:comment "Red." .
            logic:green a logic:Color .
        "#,
        );
        let schema = schema_of(&compiled);
        let openapi: Value =
            serde_json::from_str(&compiled.openapi_json).expect("openapi is valid JSON");
        assert_eq!(
            schema["$defs"]["ColorEnum"], openapi["components"]["schemas"]["ColorEnum"],
            "the enum $def is byte-identical across JSON Schema $defs and OpenAPI components/schemas"
        );
    }

    #[test]
    fn value_vocab_output_is_byte_deterministic() {
        let body = r#"
            logic:TermStability a logic:AbstractIndividualType .
            logic:stable       a logic:TermStability ; rdfs:comment "Stable." .
            logic:deprecated   a logic:TermStability .
        "#;
        let a = compile_vocab(body);
        let b = compile_vocab(body);
        assert_eq!(
            a.schema_json, b.schema_json,
            "schema output must be byte-stable"
        );
        assert_eq!(
            a.openapi_json, b.openapi_json,
            "openapi output must be byte-stable"
        );
    }

    #[test]
    fn value_vocab_description_selection_is_load_order_independent() {
        // A member with MULTIPLE rdfs:comment values, presented in two DIFFERENT
        // triple orders, yields the SAME (canonically-first) description — proving
        // sort-then-pick, not iteration-first.
        let order_a = compile_vocab(
            r#"
            logic:Color a logic:AbstractIndividualType .
            logic:red   a logic:Color ; rdfs:comment "Zeta" ; rdfs:comment "Alpha" .
        "#,
        );
        let order_b = compile_vocab(
            r#"
            logic:Color a logic:AbstractIndividualType .
            logic:red   a logic:Color ; rdfs:comment "Alpha" ; rdfs:comment "Zeta" .
        "#,
        );
        let desc_a = schema_of(&order_a)["$defs"]["ColorEnum"]["x-enum-descriptions"][0].clone();
        let desc_b = schema_of(&order_b)["$defs"]["ColorEnum"]["x-enum-descriptions"][0].clone();
        assert_eq!(
            desc_a,
            json!("Alpha"),
            "picks the canonically-first comment"
        );
        assert_eq!(desc_a, desc_b, "selection is independent of triple order");
    }

    #[test]
    fn value_vocab_none_and_zero_match_equal_plain_compile() {
        // R7: None == compile(); AND a marker that matches zero classes == compile()
        // (the realistic misconfiguration — wrong marker IRI — must not perturb output).
        let ttl = format!(
            "{PREFIXES}@prefix logic: <https://blackcatinformatics.ca/logic/> .\n{}",
            r"
            meta:CatShape a sh:NodeShape ;
                sh:targetClass meta:Cat ;
                sh:property [ sh:path meta:name ; sh:minCount 1 ] .
            logic:TermStability a logic:AbstractIndividualType .
            logic:stable a logic:TermStability .
        "
        );
        let dataset = crate::text_ingest::parse_turtle_to_dataset(&ttl).expect("Turtle parse");
        let shapes = from_dataset(&dataset).expect("shape parse");
        let plain = compile(&shapes, &fixture_ns());
        let none = compile_with_value_vocab(&shapes, &fixture_ns(), None);
        assert_eq!(
            plain.schema_json, none.schema_json,
            "None must equal compile()"
        );
        assert_eq!(plain.openapi_json, none.openapi_json);

        let vocab = ValueVocab::new("https://example.org/meta/NoSuchMarker");
        let proj = ValueVocabProjection {
            vocab: &vocab,
            ontology: dataset.as_ref(),
        };
        let zero = compile_with_value_vocab(&shapes, &fixture_ns(), Some(&proj));
        assert_eq!(
            plain.schema_json, zero.schema_json,
            "a marker matching zero classes must equal compile()"
        );
    }

    #[test]
    fn value_vocab_empty_records_loss_and_empty_enum() {
        let compiled = compile_vocab(
            r"
            logic:Empty a logic:AbstractIndividualType .
        ",
        );
        assert_eq!(
            schema_of(&compiled)["$defs"]["EmptyEnum"]["enum"],
            json!([]),
            "an empty vocabulary emits an empty enum"
        );
        assert!(
            compiled
                .losses
                .iter()
                .any(|l| l.construct == "value-vocabulary" && l.shape_iri.ends_with("Empty")),
            "an empty vocabulary is recorded as a loss, not silently dropped"
        );
    }

    #[test]
    fn value_vocab_blank_node_member_is_dropped_with_loss() {
        let compiled = compile_vocab(
            r"
            logic:HasBlank a logic:AbstractIndividualType .
            logic:named a logic:HasBlank .
            [] a logic:HasBlank .
        ",
        );
        assert_eq!(
            schema_of(&compiled)["$defs"]["HasBlankEnum"]["enum"],
            json!([{ "@id": "logic:named" }]),
            "the named member survives; the blank-node member is dropped"
        );
        assert!(
            compiled
                .losses
                .iter()
                .any(|l| l.construct == "value-vocabulary member"),
            "the dropped blank-node member is recorded as a loss"
        );
    }

    #[test]
    #[should_panic(expected = "cross-namespace local-name twins")]
    fn value_vocab_local_name_twins_hard_fail() {
        // Two vocab classes with the same local name in different declared
        // namespaces both key to `ColorEnum` — the collision guard must reject it.
        compile_vocab(
            r"
            meta:Color  a logic:AbstractIndividualType .
            logic:Color a logic:AbstractIndividualType .
        ",
        );
    }

    #[test]
    #[should_panic(expected = "has no declared namespace prefix")]
    fn value_vocab_undeclared_namespace_class_hard_fails() {
        // A vocab class in a namespace the Namespaces table does not declare has
        // no prefix CURIE to key its `{Local}Enum` $def / members by.
        compile_vocab(
            r"
            @prefix und: <https://undeclared.example/> .
            und:Foo a logic:AbstractIndividualType .
            und:bar a und:Foo .
        ",
        );
    }

    #[test]
    #[should_panic(expected = "collides with a class `$def` key")]
    fn value_vocab_enum_key_class_def_key_clash_hard_fails() {
        // A vocab class `logic:Color` enum-keys to `ColorEnum`, which is the SAME
        // `$def` key a primary-namespace target class `meta:ColorEnum` receives
        // (local-name keyed) — the enum-key-vs-class-key clash guard must reject
        // it, distinct from both the twins guard and the undeclared-namespace
        // guard (both `logic:` and `meta:` are declared namespaces here).
        compile_vocab(
            r"
            logic:Color a logic:AbstractIndividualType .
            logic:red   a logic:Color .

            meta:ColorEnumShape a sh:NodeShape ;
                sh:targetClass meta:ColorEnum ;
                sh:property [ sh:path meta:name ; sh:minCount 1 ] .
        ",
        );
    }

    // ── Referencing $ref (Task 3) ────────────────────────────────────────────

    /// Compile with a SEPARATE ontology dataset and shapes dataset (both `logic:`
    /// declared), so a `rdfs:range` declared shapes-side but absent ontology-side
    /// still resolves via the union scan.
    fn compile_split(ontology_body: &str, shapes_body: &str) -> CompiledSchema {
        let logic = "@prefix logic: <https://blackcatinformatics.ca/logic/> .\n";
        let onto = crate::text_ingest::parse_turtle_to_dataset(&format!(
            "{PREFIXES}{logic}{ontology_body}"
        ))
        .expect("ontology Turtle parse");
        let shapes_ds =
            crate::text_ingest::parse_turtle_to_dataset(&format!("{PREFIXES}{logic}{shapes_body}"))
                .expect("shapes Turtle parse");
        let shapes = from_dataset(&shapes_ds).expect("shape parse");
        let vocab = ValueVocab::new(MARKER);
        let projection = ValueVocabProjection {
            vocab: &vocab,
            ontology: onto.as_ref(),
        };
        compile_with_value_vocab(&shapes, &fixture_ns(), Some(&projection))
    }

    const STABILITY_VOCAB: &str = r"
        logic:TermStability a logic:AbstractIndividualType .
        logic:stable     a logic:TermStability .
        logic:deprecated a logic:TermStability .
    ";

    #[test]
    fn value_vocab_sh_class_single_emits_scalar_ref() {
        // R4: sh:class → vocab, maxCount 1 ⇒ a bare scalar $ref to the enum $def.
        let schema = schema_of(&compile_vocab(&format!(
            "{STABILITY_VOCAB}
            meta:TermShape a sh:NodeShape ;
                sh:targetClass meta:Term ;
                sh:property [ sh:path meta:stability ; sh:class logic:TermStability ; sh:maxCount 1 ] ."
        )));
        assert_eq!(
            def(&schema, "Term")["properties"]["meta:stability"],
            json!({ "$ref": "#/$defs/TermStabilityEnum" }),
            "single-valued vocab property is a scalar $ref"
        );
        assert_no_dangling_defs_refs(&schema);
    }

    #[test]
    fn value_vocab_sh_class_multi_wraps_ref_in_array() {
        // R4: multi-valued ⇒ anyOf[scalar $ref, array-of-$ref], cardinality preserved.
        let schema = schema_of(&compile_vocab(&format!(
            "{STABILITY_VOCAB}
            meta:TermShape a sh:NodeShape ;
                sh:targetClass meta:Term ;
                sh:property [ sh:path meta:stability ; sh:class logic:TermStability ] ."
        )));
        let prop = &def(&schema, "Term")["properties"]["meta:stability"];
        let alts = prop["anyOf"]
            .as_array()
            .expect("multi-valued property is anyOf");
        assert!(
            alts.contains(&json!({ "$ref": "#/$defs/TermStabilityEnum" })),
            "the bare scalar $ref is one alternative"
        );
        assert!(
            alts.iter().any(|a| a["type"] == json!("array")
                && a["items"] == json!({ "$ref": "#/$defs/TermStabilityEnum" })),
            "the array form wraps the same $ref in items"
        );
        assert_no_dangling_defs_refs(&schema);
    }

    #[test]
    fn value_vocab_rdfs_range_open_property_emits_ref() {
        // R4: a property with NO sh:class but a vocab rdfs:range ⇒ enum $ref.
        let schema = schema_of(&compile_vocab(&format!(
            "{STABILITY_VOCAB}
            meta:stability rdfs:range logic:TermStability .
            meta:TermShape a sh:NodeShape ;
                sh:targetClass meta:Term ;
                sh:property [ sh:path meta:stability ; sh:maxCount 1 ] ."
        )));
        assert_eq!(
            def(&schema, "Term")["properties"]["meta:stability"],
            json!({ "$ref": "#/$defs/TermStabilityEnum" }),
            "an open (no sh:class) property with a vocab range resolves to the enum $ref"
        );
        assert_no_dangling_defs_refs(&schema);
    }

    #[test]
    fn value_vocab_rdfs_range_declared_shapes_side_still_resolves() {
        // The rdfs:range lives ONLY in the shapes dataset (not the ontology one);
        // the union scan must still find it.
        let schema = schema_of(&compile_split(
            STABILITY_VOCAB,
            r"
            meta:stability rdfs:range logic:TermStability .
            meta:TermShape a sh:NodeShape ;
                sh:targetClass meta:Term ;
                sh:property [ sh:path meta:stability ; sh:maxCount 1 ] .",
        ));
        assert_eq!(
            def(&schema, "Term")["properties"]["meta:stability"],
            json!({ "$ref": "#/$defs/TermStabilityEnum" }),
            "a shapes-side rdfs:range resolves via the ontology∪shapes union scan"
        );
    }

    #[test]
    fn value_vocab_multi_range_picks_largest_class_iri_deterministically() {
        // When ONE predicate declares rdfs:range over TWO distinct value-vocab
        // classes, the winner is the class with the lexicographically LARGEST
        // class IRI (".../logic/Beta" > ".../logic/Alpha"), not load order.
        let forward = schema_of(&compile_vocab(
            r"
            logic:Alpha a logic:AbstractIndividualType .
            logic:a1    a logic:Alpha .
            logic:Beta  a logic:AbstractIndividualType .
            logic:b1    a logic:Beta .
            meta:choice rdfs:range logic:Alpha, logic:Beta .
            meta:ThingShape a sh:NodeShape ;
                sh:targetClass meta:Thing ;
                sh:property [ sh:path meta:choice ; sh:maxCount 1 ] .
        ",
        ));
        assert_eq!(
            def(&forward, "Thing")["properties"]["meta:choice"],
            json!({ "$ref": "#/$defs/BetaEnum" }),
            "the larger class IRI (logic:Beta) wins the multi-range tiebreak"
        );

        // Reversed triple order (Beta declared before Alpha) must yield the
        // IDENTICAL winner — the tiebreak is by value, not by scan order.
        let reversed = schema_of(&compile_vocab(
            r"
            logic:Beta  a logic:AbstractIndividualType .
            logic:b1    a logic:Beta .
            logic:Alpha a logic:AbstractIndividualType .
            logic:a1    a logic:Alpha .
            meta:choice rdfs:range logic:Beta, logic:Alpha .
            meta:ThingShape a sh:NodeShape ;
                sh:targetClass meta:Thing ;
                sh:property [ sh:path meta:choice ; sh:maxCount 1 ] .
        ",
        ));
        assert_eq!(
            def(&reversed, "Thing")["properties"]["meta:choice"],
            json!({ "$ref": "#/$defs/BetaEnum" }),
            "the winner is order-independent: still logic:Beta when declared first"
        );
    }

    #[test]
    fn value_vocab_sh_class_wins_over_conflicting_range() {
        // E4 precedence: a non-vocab sh:class coexisting with a vocab rdfs:range
        // keeps the sh:class behavior and records a conflict loss (no enum $ref).
        let compiled = compile_vocab(&format!(
            "{STABILITY_VOCAB}
            meta:stability rdfs:range logic:TermStability .
            meta:TermShape a sh:NodeShape ;
                sh:targetClass meta:Term ;
                sh:property [ sh:path meta:stability ; sh:class meta:OtherNode ; sh:maxCount 1 ] ."
        ));
        let schema = schema_of(&compiled);
        let prop = def(&schema, "Term")["properties"]["meta:stability"].to_string();
        assert!(
            !prop.contains("TermStabilityEnum"),
            "an explicit sh:class wins; the range enum $ref is suppressed"
        );
        assert!(
            compiled
                .losses
                .iter()
                .any(|l| l.construct == "rdfs:range" && l.reason.contains("sh:class wins")),
            "the class/range conflict is recorded as a loss"
        );
        assert_no_dangling_defs_refs(&schema);
    }

    // ── Acceptance proofs (Task 4): round-trip + open-validator decoupling ────

    /// The `meta:` term IRI for a fixture instance subject.
    fn meta_term(local: &str) -> Term {
        Term::NamedNode(NamedNode::from(
            format!("https://example.org/meta/{local}").as_str(),
        ))
    }

    #[test]
    fn value_vocab_projected_instance_round_trips_through_enum() {
        // Encoding AC: a projected instance carrying an ANCHOR individual validates
        // against the compiled enum $ref (both metadata-free and metadata-bearing —
        // the x-enum-* arrays are annotations and must not affect validation); a
        // NON-anchor value is rejected. This proves members use the {"@id":curie}
        // encoding (not bare strings) on the real projector + validator surfaces.
        for member_meta in ["", r#"; rdfs:comment "A stable term.""#] {
            let compiled = compile_vocab(&format!(
                "logic:TermStability a logic:AbstractIndividualType .
                logic:stable     a logic:TermStability {member_meta} .
                logic:deprecated a logic:TermStability .
                meta:TermShape a sh:NodeShape ;
                    sh:targetClass meta:Term ;
                    sh:property [ sh:path meta:stability ; sh:class logic:TermStability ; sh:maxCount 1 ] .
                meta:anchor    a meta:Term ; meta:stability logic:stable .
                meta:offanchor a meta:Term ; meta:stability logic:bogus ."
            ));
            // Re-parse the same document to project instances from it.
            let ttl = format!(
                "{PREFIXES}@prefix logic: <https://blackcatinformatics.ca/logic/> .\n\
                 logic:TermStability a logic:AbstractIndividualType .\n\
                 logic:stable a logic:TermStability {member_meta} .\n\
                 meta:anchor a meta:Term ; meta:stability logic:stable .\n\
                 meta:offanchor a meta:Term ; meta:stability logic:bogus ."
            );
            let dataset = crate::text_ingest::parse_turtle_to_dataset(&ttl).expect("parse");
            let anchor =
                crate::instance::project_subject(&dataset, &fixture_ns(), &meta_term("anchor"));
            let off =
                crate::instance::project_subject(&dataset, &fixture_ns(), &meta_term("offanchor"));
            assert!(
                validates(&compiled.schema_json, &anchor),
                "anchor instance must validate (member_meta={member_meta:?}); projected = {anchor}"
            );
            assert!(
                !validates(&compiled.schema_json, &off),
                "a non-anchor value must be rejected by the enum $ref"
            );
        }
    }

    #[test]
    fn value_vocab_projection_leaves_the_live_validator_open() {
        // R5: on an rdfs:range-associated OPEN property (no sh:class), a non-anchor
        // IRI value CONFORMS to the live SHACL validator, yet its projection is
        // REJECTED by the closed enum schema — proving the projection never closes
        // the validator. Also pins that no sh:in was injected into the shape IR.
        let ttl = format!(
            "{PREFIXES}@prefix logic: <https://blackcatinformatics.ca/logic/> .\n\
             logic:TermStability a logic:AbstractIndividualType .\n\
             logic:stable a logic:TermStability .\n\
             meta:stability rdfs:range logic:TermStability .\n\
             meta:TermShape a sh:NodeShape ;\n\
                 sh:targetClass meta:Term ;\n\
                 sh:property [ sh:path meta:stability ; sh:maxCount 1 ] .\n\
             meta:t1 a meta:Term ; meta:stability logic:bogus ."
        );
        let dataset = crate::text_ingest::parse_turtle_to_dataset(&ttl).expect("parse");
        let shapes = from_dataset(&dataset).expect("shapes");

        // No sh:in was introduced anywhere in the validating shape IR.
        let has_sh_in = shapes.node_shapes.iter().any(|s| {
            s.property_shapes.iter().any(|ps| {
                ps.constraints
                    .iter()
                    .any(|c| matches!(c, Constraint::In(_)))
            })
        });
        assert!(
            !has_sh_in,
            "the projection must not inject sh:in into the shape set"
        );

        // The live SHACL validator conforms the non-anchor value (open world).
        let report =
            crate::engine::validate_dataset(dataset.as_ref(), &shapes).expect("validation runs");
        assert!(
            report.conforms,
            "the open property conforms a non-anchor value in the live validator"
        );

        // The SAME value is rejected by the closed enum projection.
        let vocab = ValueVocab::new(MARKER);
        let proj = ValueVocabProjection {
            vocab: &vocab,
            ontology: dataset.as_ref(),
        };
        let compiled = compile_with_value_vocab(&shapes, &fixture_ns(), Some(&proj));
        let node = crate::instance::project_subject(&dataset, &fixture_ns(), &meta_term("t1"));
        assert!(
            !validates(&compiled.schema_json, &node),
            "the projection is closed: the non-anchor value fails the enum $ref"
        );
    }
}
