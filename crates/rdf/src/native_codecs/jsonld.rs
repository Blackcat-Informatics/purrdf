// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! First-party JSON-LD-star / YAML-LD-star codec.
//!
//! Serializes the frozen [`RdfDataset`] to the PurRDF JSON-LD-star lead artifact and a
//! deterministic YAML-LD-star derivative, and parses both back into the native carrier.
//! The serializer walks the first-party `SerGraph` (the same shape the Turtle / TriG /
//! N-Triples / N-Quads serializers walk), built from the frozen IR via
//! `build_ser_graph` — so it shares one lowering
//! and never touches the external `purrdf-gts` codecs. GTS is exit-only.
//!
//! The JSON output is byte-deterministic: every map is a [`BTreeMap`] and every array is
//! explicitly sorted, so the document does not depend on input append order.

mod carrier;
/// Compiled JSON-LD 1.1 contexts, immutable offline registries, and configured
/// serialization options shared by every JSON-LD/YAML-LD surface.
pub mod context;
mod derived;
mod expand;

pub use context::{
    CompiledJsonLdContext, JSON_LD_SERIALIZE_OPTIONS_VERSION, JsonLdContainer, JsonLdContextLimits,
    JsonLdContextRegistry, JsonLdDirection, JsonLdNullable, JsonLdSerializeMode,
    JsonLdSerializeOptions, JsonLdTermDefinition, JsonLdTermSelection, JsonLdTermSelectionKind,
    JsonLdTypeMapping,
};

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::io::Write as IoWrite;
use std::sync::Arc;

use serde_json::Value;

use self::carrier::{
    Document as CarrierDocument, Literal as CarrierLiteral, NamedGraph as CarrierNamedGraph,
    Node as CarrierNode, Term as CarrierTerm, Triple as CarrierTriple, Value as CarrierValue,
};

use super::NativeRdfFormat;
use super::codec::RdfCodec;
use super::ser_model::{SerGraph, SerTerm, SerTermKind};
use super::serialize::build_ser_graph;
use super::text_parse::LineParseMode;
use crate::{DatasetView, RdfDataset, RdfDiagnostic, RdfQuad, RdfTerm, SerializeGraph};

// Literal datatype sentinels (read off the carrier's first-class literal fields).
const RDF_DIR_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString";
const RDF_FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
const RDF_JSON: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#JSON";
const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";
const RDF_NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";
const RDF_REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
/// Schema reference for the YAML-LD language-server header when the output is
/// consumed from the bundled `purrdf.gts` snapshot. The schema is shipped as
/// `schemas-archive/purrdf.schema.json`, so a bare member name resolves inside the
/// bundle.
const BUNDLED_SCHEMA_REF: &str = "purrdf.schema.json";
const MAX_JSON_LD_OUTPUT_BYTES: usize = 256 * 1024 * 1024;
const MAX_JSON_LD_CARRIER_ROWS: usize = 4_194_304;
const MAX_JSON_LD_CARRIER_TEXT_BYTES: usize = 256 * 1024 * 1024;
const ESTIMATED_CARRIER_ROW_BYTES: usize = 256;
const COMPACTED_CARRIER_WORKING_COPIES: usize = 3;

/// RDF 1.2 reifier predicate.
pub const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

/// The CALLER-SUPPLIED statement-metadata reification vocabulary the
/// JSON-LD-star downcast emits.
///
/// PurRDF is not an ontology and mints no vocabulary IRIs of its own, so there
/// is deliberately NO `Default`: the reified statement-metadata
/// class/predicates are always the CONSUMER's vocabulary (e.g. an
/// application's `app:StatementMetadata` / `app:qSubject`). The downcast entry
/// points ([`jsonld_to_statement_metadata_nquads`] /
/// [`yamlld_to_statement_metadata_nquads`]) take an `Option` of this vocab:
/// star-free input downcasts fine unconfigured, while input carrying quoted
/// triples / reifier annotations hard-fails without a configured vocab.
#[derive(Debug, Clone, Copy)]
pub struct StatementMetadataVocab<'a> {
    /// The reifier's `rdf:type` (the statement-metadata class IRI).
    pub statement_metadata: &'a str,
    /// The quoted-subject predicate IRI.
    pub q_subject: &'a str,
    /// The quoted-predicate predicate IRI.
    pub q_predicate: &'a str,
    /// The quoted-object predicate IRI (IRI / blank-node objects).
    pub q_object: &'a str,
    /// The quoted-object predicate IRI for literal objects.
    pub q_object_literal: &'a str,
}

/// Reifier lookup: base triple (s,p,o) in a given graph (`None` = default graph) ->
/// reifier ids that annotate it. Graph-scoped: the SAME base triple reified by
/// DIFFERENT reifiers in DIFFERENT named graphs must not cross-contaminate.
type ReifierIndex = BTreeMap<(usize, usize, usize, Option<usize>), Vec<usize>>;
/// Annotation lookup: (reifier id, graph) -> sorted annotation (predicate, value)
/// rows. Graph-scoped alongside [`ReifierIndex`] for the same reason.
type AnnotationIndex = BTreeMap<(usize, Option<usize>), Vec<(usize, usize)>>;

/// The two graph-scoped lookup indices the node/value-object builders always
/// consult together, bundled into one reference so a builder needing both takes one
/// parameter instead of two (keeping argument counts under the pedantic lint cap).
struct Indexes<'a> {
    reifier_of: &'a ReifierIndex,
    annotations_of: &'a AnnotationIndex,
}
/// Quads grouped by graph name and then by subject.
type QuadGroups = BTreeMap<Option<usize>, BTreeMap<usize, Vec<(usize, usize)>>>;
/// Reifier id to every `(subject, predicate, object, graph)` binding it owns.
type BindingsByReifier = BTreeMap<usize, Vec<(usize, usize, usize, Option<usize>)>>;

// ── serialize-side helpers over the first-party SerGraph ────────────────────────────

/// The datatype IRI of a literal term, resolved the way the carrier does.
///
/// The first-party [`SerGraph`] omits the datatype slot (`None`) for a plain literal
/// (no language, `xsd:string`) so the N-Quads serializer emits it WITHOUT an explicit
/// `^^<…>` (see `build_ser_graph`). The JSON-LD walk needs the resolved datatype to
/// decide between `@language` / `@type` / bare `@value`, so a `None` slot on a
/// language-free literal resolves to `xsd:string` (not the empty string, which would
/// wrongly trip the `@type` branch and round-trip back through the `@vocab`). A
/// language-tagged literal keeps its `None` datatype slot — the `@language` branch never
/// consults this helper for the datatype IRI. Non-literals resolve to `""`.
fn datatype_iri(g: &SerGraph, term: &SerTerm) -> String {
    match term.datatype {
        Some(dt) => g.terms[dt].value.clone().unwrap_or_default(),
        None if term.kind != SerTermKind::Literal => String::new(),
        // A language-tagged literal: `rdf:dirLangString` when a base direction is also
        // carried, else `rdf:langString` (the carrier's first-class representation).
        None if term.lang.is_some() && term.direction.is_some() => RDF_DIR_LANG_STRING.to_string(),
        None if term.lang.is_some() => RDF_LANG_STRING.to_string(),
        // A plain literal (no language) is `xsd:string`.
        None => XSD_STRING.to_string(),
    }
}

/// The `(s, p, o)` components of a quoted-triple term, resolved through its
/// self-reifier binding (the [`SerGraph`] carries triple-term components there).
fn triple_components(g: &SerGraph, term: &SerTerm) -> Option<(usize, usize, usize)> {
    term.reifier.and_then(|rid| g.reifier(rid))
}

/// Convert a sorted BTreeMap into a serde_json object value.
fn to_json_object(map: BTreeMap<String, Value>) -> Value {
    Value::Object(map.into_iter().collect())
}

/// The JSON-LD-star codec — the registry's behavior seam for `application/ld+json`.
///
/// Both `serialize` and `parse` route through the SAME cores the public free functions
/// use ([`serialize_ser_graph`] / [`parse_jsonld`]), so generic dispatch and the
/// side-door API are one code path, two entry points. Base IRI / parse mode are ignored:
/// JSON-LD derives its base from the document's own `@context`, and it has no
/// line/Turtle-family tokenizer toggle.
pub(super) struct JsonLdCodec;

impl RdfCodec for JsonLdCodec {
    fn parse(
        &self,
        text: &str,
        _base_iri: Option<&str>,
        _mode: LineParseMode,
    ) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
        parse_jsonld(text.as_bytes())
    }

    fn serialize(&self, graph: &SerGraph) -> Result<String, RdfDiagnostic> {
        serialize_ser_graph(graph)
    }
}

/// The YAML-LD-star codec — the registry's behavior seam for `application/ld+yaml`.
///
/// Serialize walks the shared JSON-LD-star core then re-emits as YAML;
/// parse bridges YAML→JSON ([`yamlld_to_jsonld`]) and reuses [`parse_jsonld`]. The
/// registry path uses the bundled schema reference (the custom-`schema_url` overload
/// stays on the public [`serialize_dataset_to_yamlld`]).
pub(super) struct YamlLdCodec;

impl RdfCodec for YamlLdCodec {
    fn parse(
        &self,
        text: &str,
        _base_iri: Option<&str>,
        _mode: LineParseMode,
    ) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
        let json = yamlld_to_jsonld(text.as_bytes())?;
        parse_jsonld(json.as_bytes())
    }

    fn serialize(&self, graph: &SerGraph) -> Result<String, RdfDiagnostic> {
        serialize_ser_graph_to_yamlld(graph, None)
    }
}

/// Serialize the carrier dataset to a deterministic JSON-LD-star document.
pub fn serialize_dataset_to_jsonld<D: DatasetView>(dataset: &D) -> Result<String, RdfDiagnostic> {
    // Build the same first-party graph shape the RDF text serializers walk. A
    // dataset-capable format (N-Quads) keeps named graphs; the full RDF 1.2 statement
    // layer participates.
    let graph = build_ser_graph(
        dataset,
        NativeRdfFormat::NQuads,
        SerializeGraph::Dataset,
        true,
    )?;
    serialize_ser_graph(&graph)
}

/// Serialize a carrier dataset under an explicitly selected JSON-LD mode.
///
/// Expanded mode is byte-identical to [`serialize_dataset_to_jsonld`]. A compiled
/// caller context is applied through the typed RDF 1.2 carrier; derived mode is
/// implemented by the deterministic dataset analysis layer.
pub fn serialize_dataset_to_jsonld_with_options<D: DatasetView>(
    dataset: &D,
    options: &JsonLdSerializeOptions,
) -> Result<String, RdfDiagnostic> {
    let graph = build_ser_graph(
        dataset,
        NativeRdfFormat::NQuads,
        SerializeGraph::Dataset,
        true,
    )?;
    serialize_ser_graph_with_options(&graph, options)
}

/// Serialize a dataset through an already compiled, reusable caller context.
///
/// This is the allocation-light overload for callers that retain one context across
/// many datasets. It is equivalent to context-mode [`JsonLdSerializeOptions`] without
/// cloning the compiled context.
pub fn serialize_dataset_to_jsonld_with_context<D: DatasetView>(
    dataset: &D,
    context: &CompiledJsonLdContext,
) -> Result<String, RdfDiagnostic> {
    let graph = build_ser_graph(
        dataset,
        NativeRdfFormat::NQuads,
        SerializeGraph::Dataset,
        true,
    )?;
    let carrier = build_carrier(&graph, true)?;
    serialize_carrier_compacted(&carrier, context)
}

/// Derive a deterministic, vocabulary-neutral JSON-LD context from dataset IRI slots.
///
/// Only reversible `#`, `/`, and URN-style `:` namespace boundaries that reduce total
/// encoded bytes are retained. Aliases are assigned as `ns0`, `ns1`, … from sorted
/// namespace IRIs; no `@vocab` mapping or caller vocabulary is invented.
pub fn derive_jsonld_context<D: DatasetView>(
    dataset: &D,
) -> Result<CompiledJsonLdContext, RdfDiagnostic> {
    let graph = build_ser_graph(
        dataset,
        NativeRdfFormat::NQuads,
        SerializeGraph::Dataset,
        true,
    )?;
    derived::derive_context(&build_carrier(&graph, true)?)
}

/// Serialize an already-materialized [`SerGraph`] to a deterministic JSON-LD-star
/// document.
fn serialize_ser_graph(graph: &SerGraph) -> Result<String, RdfDiagnostic> {
    serialize_carrier_expanded(&build_carrier(graph, false)?)
}

pub(crate) fn serialize_ser_graph_with_options(
    graph: &SerGraph,
    options: &JsonLdSerializeOptions,
) -> Result<String, RdfDiagnostic> {
    let fold_lists = !matches!(options.mode(), JsonLdSerializeMode::Expanded);
    let carrier = build_carrier(graph, fold_lists)?;
    match options.mode() {
        JsonLdSerializeMode::Expanded => serialize_carrier_expanded(&carrier),
        JsonLdSerializeMode::Context(context) => serialize_carrier_compacted(&carrier, context),
        JsonLdSerializeMode::Derived => {
            let context = derived::derive_context(&carrier)?;
            serialize_carrier_compacted(&carrier, &context)
        }
    }
}

fn serialize_carrier_expanded(carrier: &CarrierDocument) -> Result<String, RdfDiagnostic> {
    let mut output = BoundedJsonOutput::new(MAX_JSON_LD_OUTPUT_BYTES);
    carrier.write_expanded_json(&mut output, &build_context())?;
    finish_json_output(output)
}

fn serialize_carrier_compacted(
    carrier: &CarrierDocument,
    context: &CompiledJsonLdContext,
) -> Result<String, RdfDiagnostic> {
    let mut output = BoundedJsonOutput::new(MAX_JSON_LD_OUTPUT_BYTES);
    carrier.write_compacted_json(&mut output, context)?;
    finish_json_output(output)
}

fn finish_json_output(output: BoundedJsonOutput) -> Result<String, RdfDiagnostic> {
    String::from_utf8(output.into_bytes())
        .map_err(|source| decode(format!("JSON-LD output is not UTF-8: {source}")))
}

struct BoundedJsonOutput {
    bytes: Vec<u8>,
    limit: usize,
}

impl BoundedJsonOutput {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            limit,
        }
    }

    fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

impl IoWrite for BoundedJsonOutput {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let next = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .ok_or_else(|| std::io::Error::other("JSON-LD output length overflow"))?;
        if next > self.limit {
            return Err(std::io::Error::other(format!(
                "JSON-LD output exceeds {} bytes",
                self.limit
            )));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Serialize the carrier dataset to deterministic YAML-LD-star bytes.
///
/// The JSON-LD-star document is re-serialized to YAML with sorted keys, block style, no
/// anchors/aliases, and an explicit `@context`. The header carries a YAML
/// language-server schema reference.
pub fn serialize_dataset_to_yamlld<D: DatasetView>(
    dataset: &D,
    schema_url: Option<&str>,
) -> Result<String, RdfDiagnostic> {
    let graph = build_ser_graph(
        dataset,
        NativeRdfFormat::NQuads,
        SerializeGraph::Dataset,
        true,
    )?;
    serialize_ser_graph_to_yamlld(&graph, schema_url)
}

/// Serialize a dataset to deterministic YAML-LD under an explicitly selected mode.
///
/// The optional schema reference is carried by [`JsonLdSerializeOptions`] so direct,
/// generic, CLI, and foreign-language routes all produce the same header bytes.
pub fn serialize_dataset_to_yamlld_with_options<D: DatasetView>(
    dataset: &D,
    options: &JsonLdSerializeOptions,
) -> Result<String, RdfDiagnostic> {
    let graph = build_ser_graph(
        dataset,
        NativeRdfFormat::NQuads,
        SerializeGraph::Dataset,
        true,
    )?;
    serialize_ser_graph_to_yamlld_with_options(&graph, options)
}

/// Serialize a dataset to deterministic YAML-LD through an already compiled,
/// reusable caller context.
pub fn serialize_dataset_to_yamlld_with_context<D: DatasetView>(
    dataset: &D,
    context: &CompiledJsonLdContext,
    schema_url: Option<&str>,
) -> Result<String, RdfDiagnostic> {
    let graph = build_ser_graph(
        dataset,
        NativeRdfFormat::NQuads,
        SerializeGraph::Dataset,
        true,
    )?;
    let carrier = build_carrier(&graph, true)?;
    let json = serialize_carrier_compacted(&carrier, context)?;
    jsonld_to_yaml(&json, schema_url)
}

/// Serialize an already-materialized [`SerGraph`] to deterministic YAML-LD-star bytes —
/// the graph-level core shared by [`serialize_dataset_to_yamlld`] and [`YamlLdCodec`].
fn serialize_ser_graph_to_yamlld(
    graph: &SerGraph,
    schema_url: Option<&str>,
) -> Result<String, RdfDiagnostic> {
    let json = serialize_ser_graph(graph)?;
    jsonld_to_yaml(&json, schema_url)
}

pub(crate) fn serialize_ser_graph_to_yamlld_with_options(
    graph: &SerGraph,
    options: &JsonLdSerializeOptions,
) -> Result<String, RdfDiagnostic> {
    let json = serialize_ser_graph_with_options(graph, options)?;
    jsonld_to_yaml(&json, options.yaml_schema_url())
}

fn jsonld_to_yaml(json: &str, schema_url: Option<&str>) -> Result<String, RdfDiagnostic> {
    let value: Value =
        serde_json::from_str(json).map_err(|e| decode(format!("parse JSON-LD for YAML: {e}")))?;
    let body =
        serde_yaml::to_string(&value).map_err(|e| decode(format!("YAML-LD serialization: {e}")))?;
    let url = schema_url.unwrap_or(BUNDLED_SCHEMA_REF);
    let header = format!(
        "# yaml-language-server: $schema={url}\n\
         # The default reference is the bundled purrdf.schema.json; pass an explicit\n\
         # schema_url to point editors at a hosted copy.\n"
    );
    Ok(header + &body)
}

/// Build the deliberately empty JSON-LD `@context` for the byte-frozen legacy route.
///
/// Configured serializers compact through the caller's context or an explicitly
/// selected deterministic derived context. No-options entry points retain this exact
/// expanded representation for compatibility.
fn build_context() -> Value {
    to_json_object(BTreeMap::new())
}

/// Build the typed expanded carrier from the first-party serialization graph.
fn build_carrier(graph: &SerGraph, fold_lists: bool) -> Result<CarrierDocument, RdfDiagnostic> {
    validate_source_carrier_budget(graph)?;
    // Reifier index: base triple (s,p,o) in graph g -> reifier ids that annotate it.
    let mut reifier_of: ReifierIndex = BTreeMap::new();
    for &(rid, (s, p, o), g) in &graph.reifiers {
        // A triple term is self-reifying: its `reifier` row's "reifier" is the triple
        // term itself (kind `Triple`), carrying the term's components — NOT a real
        // IRI/blank-node reifier. `term_id` has no @id for a `Triple`-kind term, so it
        // must never enter the sortable reifier index; skip it here (mirrors the
        // orphan-reifier guard below).
        if graph.terms[rid].kind == SerTermKind::Triple {
            continue;
        }
        reifier_of.entry((s, p, o, g)).or_default().push(rid);
    }
    for list in reifier_of.values_mut() {
        // Sort by the reifier's stable @id, not its input-order term id.
        list.sort_by(|a, b| {
            let a_id = term_id(&graph.terms[*a]).expect("reifier must be IRI or blank node");
            let b_id = term_id(&graph.terms[*b]).expect("reifier must be IRI or blank node");
            a_id.cmp(&b_id)
        });
    }

    // Annotation index: (reifier id, graph) -> sorted annotation (predicate, value) rows.
    let mut annotations_of: AnnotationIndex = BTreeMap::new();
    for &(r, p, v, g) in &graph.annotations {
        annotations_of.entry((r, g)).or_default().push((p, v));
    }
    for list in annotations_of.values_mut() {
        // Sort by stable predicate @id then stable value key, not raw term ids.
        list.sort_by(|(ap, av), (bp, bv)| {
            let a_pred = term_id(&graph.terms[*ap]).expect("annotation predicate must be IRI");
            let b_pred = term_id(&graph.terms[*bp]).expect("annotation predicate must be IRI");
            a_pred.cmp(&b_pred).then_with(|| {
                term_sort_key(graph, &graph.terms[*av])
                    .cmp(&term_sort_key(graph, &graph.terms[*bv]))
            })
        });
    }

    validate_materialized_carrier_budget(graph, &annotations_of)?;

    let indexes = Indexes {
        reifier_of: &reifier_of,
        annotations_of: &annotations_of,
    };

    let mut bindings_by_reifier = BindingsByReifier::new();
    for &(rid, (s, p, o), g) in &graph.reifiers {
        if graph.terms[rid].kind != SerTermKind::Triple {
            bindings_by_reifier
                .entry(rid)
                .or_default()
                .push((s, p, o, g));
        }
    }

    // Group quads by graph name (None = default graph) and then by subject.
    let mut by_graph: QuadGroups = BTreeMap::new();
    for &(s, p, o, g) in &graph.quads {
        by_graph
            .entry(g)
            .or_default()
            .entry(s)
            .or_default()
            .push((p, o));
    }

    // A reifier whose base triple is NOT asserted as a quad has no value object to carry
    // its compact `@annotation`, so emit it as a standalone node with an explicit
    // `rdf:reifies` @triple value plus its annotation properties — otherwise the reifier
    // and its annotations are silently dropped. (An asserted base triple keeps the
    // compact `@annotation` form.) Keyed by `(s,p,o,g)` — GRAPH-SCOPED — to match the
    // `@annotation` attachment in `build_value_object`, which now also keys on graph
    // name: the same base triple may be asserted in one graph and orphaned (reified
    // without assertion) in another.
    let asserted_base: BTreeSet<(usize, usize, usize, Option<usize>)> = graph
        .quads
        .iter()
        .map(|&(s, p, o, g)| (s, p, o, g))
        .collect();
    let mut orphan_by_graph: BTreeMap<Option<usize>, Vec<CarrierNode>> = BTreeMap::new();
    for &(rid, (s, p, o), g) in &graph.reifiers {
        // A triple term is self-reifying: its `reifier` row's "reifier" is the triple
        // term itself (kind `Triple`), carrying the term's components — NOT a real
        // IRI/blank-node reifier. Those are emitted as `@triple` objects where they
        // appear; only genuine reifiers become standalone nodes here.
        if graph.terms[rid].kind == SerTermKind::Triple {
            continue;
        }
        if !asserted_base.contains(&(s, p, o, g)) {
            let node = build_orphan_reifier_node(graph, rid, s, p, o, g, &indexes)?;
            orphan_by_graph.entry(g).or_default().push(node);
        }
    }
    // An annotation row has its own graph and need not share that graph with the
    // reifier declaration. When another graph owns the declaration, emit an
    // annotation-only node in this graph; the explicit/implicit reifier binding in its
    // original graph still identifies the subject as a reifier during the two-pass
    // fold. Without this fragment a graph-scoped annotation is silently omitted.
    for &(rid, annotation_graph) in annotations_of.keys() {
        let represented_in_graph = bindings_by_reifier
            .get(&rid)
            .is_some_and(|bindings| bindings.iter().any(|binding| binding.3 == annotation_graph));
        if !represented_in_graph {
            orphan_by_graph
                .entry(annotation_graph)
                .or_default()
                .push(build_annotation_node(
                    graph,
                    rid,
                    annotation_graph,
                    &indexes,
                )?);
        }
    }

    let mut default_nodes: BTreeMap<String, CarrierNode> = BTreeMap::new();
    let mut named_graphs: BTreeMap<String, CarrierNamedGraph> = BTreeMap::new();

    // Iterate the union of graph names carrying asserted quads OR orphan reifiers (a graph
    // may carry only orphan reifiers, so `by_graph` alone would miss it).
    let graph_keys: BTreeSet<Option<usize>> = by_graph
        .keys()
        .copied()
        .chain(orphan_by_graph.keys().copied())
        .collect();
    for g in graph_keys {
        let mut nodes: Vec<CarrierNode> = Vec::new();
        if let Some(subjects) = by_graph.remove(&g) {
            for (s, pos) in subjects {
                let node = build_node(graph, s, pos, g, &indexes)?;
                nodes.push(node);
            }
        }
        if let Some(orphans) = orphan_by_graph.remove(&g) {
            nodes.extend(orphans);
        }
        let mut by_id: BTreeMap<String, CarrierNode> = BTreeMap::new();
        for mut node in nodes {
            let target = by_id
                .entry(node.id.clone())
                .or_insert_with(|| CarrierNode::new(node.id.clone()));
            target.types.append(&mut node.types);
            target.types.sort();
            target.types.dedup();
            for (property, mut values) in node.properties {
                target
                    .properties
                    .entry(property)
                    .or_default()
                    .append(&mut values);
            }
            target.sort_values();
        }
        let nodes: Vec<CarrierNode> = by_id.into_values().collect();

        match g {
            None => {
                for node in nodes {
                    default_nodes.insert(node.id.clone(), node);
                }
            }
            Some(gid) => {
                let graph_term = &graph.terms[gid];
                let graph_id = term_id(graph_term)?;
                named_graphs.insert(
                    graph_id.clone(),
                    CarrierNamedGraph {
                        id: graph_id,
                        nodes,
                    },
                );
            }
        }
    }

    let mut document = CarrierDocument {
        default_nodes: default_nodes.into_values().collect(),
        named_graphs: named_graphs.into_values().collect(),
    };
    if fold_lists {
        fold_document_rdf_lists(&mut document);
    }
    Ok(document)
}

fn validate_source_carrier_budget(graph: &SerGraph) -> Result<(), RdfDiagnostic> {
    let rows = graph
        .terms
        .len()
        .checked_add(graph.quads.len())
        .and_then(|count| count.checked_add(graph.reifiers.len()))
        .and_then(|count| count.checked_add(graph.annotations.len()))
        .ok_or_else(|| decode("JSON-LD carrier row count overflow"))?;
    if rows > MAX_JSON_LD_CARRIER_ROWS {
        return Err(decode(format!(
            "JSON-LD carrier requires {rows} rows; limit is {MAX_JSON_LD_CARRIER_ROWS}"
        )));
    }
    let retained_text = graph.terms.iter().try_fold(0usize, |total, term| {
        [
            term.value.as_deref(),
            term.lang.as_deref(),
            term.direction.as_deref(),
        ]
        .into_iter()
        .flatten()
        .try_fold(total, |total, value| total.checked_add(value.len()))
    });
    let retained_text =
        retained_text.ok_or_else(|| decode("JSON-LD carrier retained-text byte count overflow"))?;
    if retained_text > MAX_JSON_LD_CARRIER_TEXT_BYTES {
        return Err(decode(format!(
            "JSON-LD carrier retains {retained_text} text bytes; limit is \
             {MAX_JSON_LD_CARRIER_TEXT_BYTES}"
        )));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, Default)]
struct CarrierFootprint {
    rows: usize,
    text_bytes: usize,
}

impl CarrierFootprint {
    fn add(&mut self, other: Self) -> Result<(), RdfDiagnostic> {
        self.rows = self
            .rows
            .checked_add(other.rows)
            .ok_or_else(|| decode("JSON-LD materialized carrier row count overflow"))?;
        self.text_bytes = self
            .text_bytes
            .checked_add(other.text_bytes)
            .ok_or_else(|| decode("JSON-LD materialized carrier text count overflow"))?;
        Ok(())
    }

    fn add_term(
        &mut self,
        graph: &SerGraph,
        term: usize,
        memo: &mut BTreeMap<usize, Self>,
        visiting: &mut BTreeSet<usize>,
    ) -> Result<(), RdfDiagnostic> {
        self.add(carrier_term_footprint(graph, term, memo, visiting)?)
    }
}

/// Reject source graphs whose typed-carrier expansion would amplify shared terms or
/// annotations beyond the fixed working-memory envelope.
///
/// The source interner stores text once, while the immutable carrier intentionally owns
/// strings at each semantic occurrence.  Count those occurrences before construction,
/// including every copy of an annotation node attached to a proposition.  The final
/// estimate also reserves space for B-tree/vector/string bookkeeping, the prepared
/// compaction copy, and the largest transient compacted subtree.
fn validate_materialized_carrier_budget(
    graph: &SerGraph,
    annotations_of: &AnnotationIndex,
) -> Result<(), RdfDiagnostic> {
    validate_materialized_carrier_budget_with_limits(
        graph,
        annotations_of,
        MAX_JSON_LD_CARRIER_ROWS,
        MAX_JSON_LD_CARRIER_TEXT_BYTES,
    )
}

fn validate_materialized_carrier_budget_with_limits(
    graph: &SerGraph,
    annotations_of: &AnnotationIndex,
    max_rows: usize,
    max_working_bytes: usize,
) -> Result<(), RdfDiagnostic> {
    let mut footprint = CarrierFootprint::default();
    let mut memo = BTreeMap::new();
    let mut visiting = BTreeSet::new();

    for &(subject, predicate, object, graph_name) in &graph.quads {
        footprint.rows = footprint
            .rows
            .checked_add(1)
            .ok_or_else(|| decode("JSON-LD materialized carrier row count overflow"))?;
        for term in [Some(subject), Some(predicate), Some(object), graph_name]
            .into_iter()
            .flatten()
        {
            footprint.add_term(graph, term, &mut memo, &mut visiting)?;
        }
    }

    let asserted: BTreeSet<(usize, usize, usize, Option<usize>)> = graph
        .quads
        .iter()
        .map(|&(subject, predicate, object, graph_name)| (subject, predicate, object, graph_name))
        .collect();
    let mut represented_annotations = BTreeSet::new();
    for &(reifier, (subject, predicate, object), graph_name) in &graph.reifiers {
        if graph.terms[reifier].kind == SerTermKind::Triple {
            continue;
        }
        represented_annotations.insert((reifier, graph_name));
        footprint.rows = footprint
            .rows
            .checked_add(1)
            .ok_or_else(|| decode("JSON-LD materialized carrier row count overflow"))?;
        for term in [
            Some(reifier),
            Some(subject),
            Some(predicate),
            Some(object),
            graph_name,
        ]
        .into_iter()
        .flatten()
        {
            footprint.add_term(graph, term, &mut memo, &mut visiting)?;
        }
        add_annotation_footprint(
            graph,
            annotations_of.get(&(reifier, graph_name)),
            &mut footprint,
            &mut memo,
            &mut visiting,
        )?;
        // An asserted binding owns the annotation node on its value object; an orphan
        // additionally owns the explicit rdf:reifies triple represented above.
        if !asserted.contains(&(subject, predicate, object, graph_name)) {
            footprint.rows = footprint
                .rows
                .checked_add(1)
                .ok_or_else(|| decode("JSON-LD materialized carrier row count overflow"))?;
        }
    }
    for (&(reifier, graph_name), annotations) in annotations_of {
        if represented_annotations.contains(&(reifier, graph_name)) {
            continue;
        }
        footprint.rows = footprint
            .rows
            .checked_add(1)
            .ok_or_else(|| decode("JSON-LD materialized carrier row count overflow"))?;
        footprint.add_term(graph, reifier, &mut memo, &mut visiting)?;
        if let Some(graph_name) = graph_name {
            footprint.add_term(graph, graph_name, &mut memo, &mut visiting)?;
        }
        add_annotation_footprint(
            graph,
            Some(annotations),
            &mut footprint,
            &mut memo,
            &mut visiting,
        )?;
    }

    let structural_bytes = footprint
        .rows
        .checked_mul(ESTIMATED_CARRIER_ROW_BYTES)
        .and_then(|bytes| bytes.checked_add(footprint.text_bytes))
        .and_then(|bytes| bytes.checked_mul(COMPACTED_CARRIER_WORKING_COPIES))
        .ok_or_else(|| decode("JSON-LD carrier working-byte estimate overflow"))?;
    if footprint.rows > max_rows || structural_bytes > max_working_bytes {
        return Err(decode(format!(
            "JSON-LD materialized carrier requires {} rows and {structural_bytes} working bytes; \
             limits are {max_rows} rows and {max_working_bytes} bytes",
            footprint.rows
        )));
    }
    Ok(())
}

fn add_annotation_footprint(
    graph: &SerGraph,
    annotations: Option<&Vec<(usize, usize)>>,
    footprint: &mut CarrierFootprint,
    memo: &mut BTreeMap<usize, CarrierFootprint>,
    visiting: &mut BTreeSet<usize>,
) -> Result<(), RdfDiagnostic> {
    let Some(annotations) = annotations else {
        return Ok(());
    };
    for &(predicate, value) in annotations {
        footprint.rows = footprint
            .rows
            .checked_add(1)
            .ok_or_else(|| decode("JSON-LD materialized carrier row count overflow"))?;
        footprint.add_term(graph, predicate, memo, visiting)?;
        footprint.add_term(graph, value, memo, visiting)?;
    }
    Ok(())
}

fn carrier_term_footprint(
    graph: &SerGraph,
    term_id: usize,
    memo: &mut BTreeMap<usize, CarrierFootprint>,
    visiting: &mut BTreeSet<usize>,
) -> Result<CarrierFootprint, RdfDiagnostic> {
    if let Some(footprint) = memo.get(&term_id) {
        return Ok(*footprint);
    }
    if !visiting.insert(term_id) {
        return Err(decode(
            "cyclic RDF triple term cannot be materialized as JSON-LD",
        ));
    }
    let term = graph.terms.get(term_id).ok_or_else(|| {
        decode(format!(
            "JSON-LD carrier term index {term_id} is out of range"
        ))
    })?;
    let mut footprint = CarrierFootprint {
        rows: 1,
        text_bytes: term
            .value
            .as_deref()
            .map_or(0, str::len)
            .checked_add(term.lang.as_deref().map_or(0, str::len))
            .and_then(|bytes| bytes.checked_add(term.direction.as_deref().map_or(0, str::len)))
            .ok_or_else(|| decode("JSON-LD materialized carrier text count overflow"))?,
    };
    if term.kind == SerTermKind::Bnode {
        footprint.text_bytes = footprint
            .text_bytes
            .checked_add(2)
            .ok_or_else(|| decode("JSON-LD materialized carrier text count overflow"))?;
    }
    if let Some(datatype) = term.datatype {
        footprint.add_term(graph, datatype, memo, visiting)?;
    }
    if term.kind == SerTermKind::Triple {
        let (subject, predicate, object) = triple_components(graph, term)
            .ok_or_else(|| decode("RDF triple term has no component binding"))?;
        for component in <[usize; 3]>::from((subject, predicate, object)) {
            footprint.add_term(graph, component, memo, visiting)?;
        }
    }
    visiting.remove(&term_id);
    memo.insert(term_id, footprint);
    Ok(footprint)
}

fn fold_document_rdf_lists(document: &mut CarrierDocument) {
    let mut graph_usage = Vec::with_capacity(document.named_graphs.len() + 1);
    graph_usage.push(carrier_node_usage(&document.default_nodes));
    graph_usage.extend(
        document
            .named_graphs
            .iter()
            .map(|graph| carrier_node_usage(&graph.nodes)),
    );
    for (index, graph) in document.named_graphs.iter().enumerate() {
        graph_usage[index + 1].insert(graph.id.clone());
    }

    let external_for = |index: usize| -> BTreeSet<String> {
        graph_usage
            .iter()
            .enumerate()
            .filter(|(other, _)| *other != index)
            .flat_map(|(_, usage)| usage.iter().cloned())
            .collect()
    };
    fold_rdf_lists(&mut document.default_nodes, &external_for(0));
    for (index, graph) in document.named_graphs.iter_mut().enumerate() {
        let mut externally_used = external_for(index + 1);
        // A graph name and a node/list identifier inhabit the same RDF blank-node
        // identity space.  Keep that identity explicit even when the only other use
        // of the identifier is as the name of the graph currently being folded.
        externally_used.insert(graph.id.clone());
        fold_rdf_lists(&mut graph.nodes, &externally_used);
    }
}

fn carrier_node_usage(nodes: &[CarrierNode]) -> BTreeSet<String> {
    let mut usage = BTreeSet::new();
    for node in nodes {
        usage.insert(node.id.clone());
        for values in node.properties.values() {
            for value in values {
                collect_carrier_value_ids(value, &mut usage, true);
            }
        }
    }
    usage
}

fn collect_carrier_value_ids(
    value: &CarrierValue,
    output: &mut BTreeSet<String>,
    annotation_subjects: bool,
) {
    collect_carrier_term_ids(&value.term, output);
    for annotation in &value.annotations {
        if annotation_subjects {
            output.insert(annotation.id.clone());
        }
        for values in annotation.properties.values() {
            for value in values {
                collect_carrier_value_ids(value, output, annotation_subjects);
            }
        }
    }
}

fn collect_carrier_term_ids(term: &CarrierTerm, output: &mut BTreeSet<String>) {
    match term {
        CarrierTerm::Id(id) => {
            output.insert(id.clone());
        }
        CarrierTerm::Triple(triple) => {
            collect_carrier_term_ids(&triple.subject, output);
            collect_carrier_term_ids(&triple.object, output);
        }
        CarrierTerm::List(values) => {
            for value in values {
                collect_carrier_value_ids(value, output, true);
            }
        }
        CarrierTerm::Literal(_) => {}
    }
}

fn fold_rdf_lists(nodes: &mut Vec<CarrierNode>, externally_used: &BTreeSet<String>) {
    let mut annotation_subjects = BTreeSet::new();
    for node in nodes.iter() {
        collect_annotation_subjects(node, &mut annotation_subjects);
    }
    let node_by_id: BTreeMap<String, usize> = nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.clone(), index))
        .collect();
    let candidates: BTreeSet<String> = nodes
        .iter()
        .filter(|node| {
            node.id.starts_with("_:")
                && !externally_used.contains(&node.id)
                && !annotation_subjects.contains(&node.id)
                && node.types.is_empty()
                && node.properties.len() == 2
                && node
                    .properties
                    .get(RDF_FIRST)
                    .is_some_and(|values| values.len() == 1 && values[0].annotations.is_empty())
                && node
                    .properties
                    .get(RDF_REST)
                    .is_some_and(|values| values.len() == 1 && values[0].annotations.is_empty())
        })
        .map(|node| node.id.clone())
        .collect();

    let mut incoming: BTreeMap<String, usize> = BTreeMap::new();
    let mut external_heads = BTreeSet::new();
    for node in nodes.iter() {
        for (property, values) in &node.properties {
            for value in values {
                count_list_references(
                    value,
                    &candidates,
                    &mut incoming,
                    &mut external_heads,
                    property == RDF_REST && candidates.contains(&node.id),
                );
            }
        }
    }

    let mut lists = BTreeMap::new();
    let mut consumed = BTreeSet::new();
    for head in external_heads {
        if incoming.get(&head) != Some(&1) {
            continue;
        }
        let mut current = head.clone();
        let mut seen = BTreeSet::new();
        let mut values = Vec::new();
        let mut chain = Vec::new();
        let valid = loop {
            if !seen.insert(current.clone()) || incoming.get(&current) != Some(&1) {
                break false;
            }
            let Some(node) = node_by_id.get(&current).and_then(|index| nodes.get(*index)) else {
                break false;
            };
            let Some(first) = node.properties.get(RDF_FIRST).and_then(|rows| rows.first()) else {
                break false;
            };
            let Some(rest) = node.properties.get(RDF_REST).and_then(|rows| rows.first()) else {
                break false;
            };
            values.push(first.clone());
            chain.push(current.clone());
            let CarrierTerm::Id(rest_id) = &rest.term else {
                break false;
            };
            if rest_id == RDF_NIL {
                break true;
            }
            if !candidates.contains(rest_id) {
                break false;
            }
            current.clone_from(rest_id);
        };
        if valid && chain.iter().all(|id| !consumed.contains(id)) {
            consumed.extend(chain);
            lists.insert(head, values);
        }
    }

    if lists.is_empty() {
        return;
    }
    for node in nodes.iter_mut() {
        for values in node.properties.values_mut() {
            for value in values {
                rewrite_folded_lists(value, &lists);
            }
        }
        node.sort_values();
    }
    nodes.retain(|node| !consumed.contains(&node.id));
}

fn collect_annotation_subjects(node: &CarrierNode, output: &mut BTreeSet<String>) {
    for values in node.properties.values() {
        for value in values {
            for annotation in &value.annotations {
                output.insert(annotation.id.clone());
                collect_annotation_subjects(annotation, output);
            }
        }
    }
}

fn count_list_references(
    value: &CarrierValue,
    candidates: &BTreeSet<String>,
    incoming: &mut BTreeMap<String, usize>,
    external_heads: &mut BTreeSet<String>,
    direct_rest: bool,
) {
    count_list_term_references(
        &value.term,
        candidates,
        incoming,
        external_heads,
        direct_rest,
    );
    for annotation in &value.annotations {
        for values in annotation.properties.values() {
            for value in values {
                count_list_references(value, candidates, incoming, external_heads, false);
            }
        }
    }
}

fn count_list_term_references(
    term: &CarrierTerm,
    candidates: &BTreeSet<String>,
    incoming: &mut BTreeMap<String, usize>,
    external_heads: &mut BTreeSet<String>,
    direct_rest: bool,
) {
    match term {
        CarrierTerm::Id(id) if candidates.contains(id) => {
            *incoming.entry(id.clone()).or_default() += 1;
            if !direct_rest {
                external_heads.insert(id.clone());
            }
        }
        CarrierTerm::Triple(triple) => {
            count_list_term_references(
                &triple.subject,
                candidates,
                incoming,
                external_heads,
                false,
            );
            count_list_term_references(&triple.object, candidates, incoming, external_heads, false);
        }
        CarrierTerm::List(values) => {
            for value in values {
                count_list_references(value, candidates, incoming, external_heads, false);
            }
        }
        CarrierTerm::Id(_) | CarrierTerm::Literal(_) => {}
    }
}

fn rewrite_folded_lists(value: &mut CarrierValue, lists: &BTreeMap<String, Vec<CarrierValue>>) {
    rewrite_folded_term(&mut value.term, lists);
    for annotation in &mut value.annotations {
        for values in annotation.properties.values_mut() {
            for value in values {
                rewrite_folded_lists(value, lists);
            }
        }
        annotation.sort_values();
    }
}

fn rewrite_folded_term(term: &mut CarrierTerm, lists: &BTreeMap<String, Vec<CarrierValue>>) {
    match term {
        CarrierTerm::Id(id) => {
            if let Some(values) = lists.get(id) {
                let mut values = values.clone();
                for value in &mut values {
                    rewrite_folded_lists(value, lists);
                }
                *term = CarrierTerm::List(values);
            }
        }
        CarrierTerm::Triple(triple) => {
            rewrite_folded_term(&mut triple.subject, lists);
            rewrite_folded_term(&mut triple.object, lists);
        }
        CarrierTerm::List(values) => {
            for value in values {
                rewrite_folded_lists(value, lists);
            }
        }
        CarrierTerm::Literal(_) => {}
    }
}

/// Build one node object for a subject from its predicate/object rows.
fn build_node(
    graph: &SerGraph,
    subject: usize,
    pos: Vec<(usize, usize)>,
    g: Option<usize>,
    indexes: &Indexes<'_>,
) -> Result<CarrierNode, RdfDiagnostic> {
    let subject_term = &graph.terms[subject];
    let mut node = CarrierNode::new(term_id(subject_term)?);

    // Group predicate -> objects, preserving rdf:type separately.
    for (p, o) in pos {
        let predicate_term = &graph.terms[p];
        let predicate_iri = predicate_term
            .value
            .as_deref()
            .ok_or_else(|| parse("predicate missing IRI value".to_string()))?;
        let object_term = &graph.terms[o];

        if predicate_iri == RDF_TYPE {
            node.types.push(term_id(object_term)?);
        } else {
            let key = absolute_iri(predicate_iri);
            let value = build_value_object(graph, subject, p, o, g, object_term, indexes)?;
            node.properties.entry(key).or_default().push(value);
        }
    }
    node.sort_values();
    Ok(node)
}

/// Build a value object for a quad object, attaching `@annotation` when the
/// base triple is reified.
fn build_value_object(
    graph: &SerGraph,
    subject: usize,
    predicate: usize,
    object: usize,
    g: Option<usize>,
    object_term: &SerTerm,
    indexes: &Indexes<'_>,
) -> Result<CarrierValue, RdfDiagnostic> {
    let term = if object_term.kind == SerTermKind::Triple {
        build_triple_term_value(graph, object_term)?
    } else {
        term_to_value(graph, object_term)?
    };
    let mut value = CarrierValue::plain(term);
    if let Some(reifiers) = indexes.reifier_of.get(&(subject, predicate, object, g)) {
        let annotations: Result<Vec<CarrierNode>, _> = reifiers
            .iter()
            .map(|&rid| build_annotation_node(graph, rid, g, indexes))
            .collect();
        value.annotations = annotations?;
    }

    Ok(value)
}

/// Render a triple term as its distinguishable JSON-LD-star `@triple` object, resolving
/// its `(s,p,o)` components through the term's own self-reifier binding.
fn build_triple_term_value(graph: &SerGraph, term: &SerTerm) -> Result<CarrierTerm, RdfDiagnostic> {
    let (s, p, o) = triple_components(graph, term)
        .ok_or_else(|| parse("triple term with no components".to_string()))?;
    build_nested_triple_node(graph, s, p, o)
}

/// Build the distinguishable JSON-LD-star `@triple` object for a quoted triple (s,p,o).
///
/// A triple term serializes to `{"@triple": {"@subject": …, "@predicate": "<iri>",
/// "@object": …}}`. The reserved `@triple` key makes it unambiguous vs an `@id` node
/// object or an `@value` literal, and every part round-trips: `@subject`/`@object` recurse
/// through the same encoding (nested triple terms work), and `@predicate` is the full
/// source IRI. Keys are `BTreeMap`-ordered, so the output is byte-deterministic.
fn build_nested_triple_node(
    graph: &SerGraph,
    s: usize,
    p: usize,
    o: usize,
) -> Result<CarrierTerm, RdfDiagnostic> {
    let subject = encode_triple_component(graph, s)?;
    let object = encode_triple_component(graph, o)?;
    let p_term = &graph.terms[p];
    let p_iri = p_term
        .value
        .as_deref()
        .ok_or_else(|| parse("triple-term predicate missing IRI".to_string()))?;

    Ok(CarrierTerm::Triple(Box::new(CarrierTriple {
        subject: Box::new(subject),
        predicate: absolute_iri(p_iri),
        object: Box::new(object),
    })))
}

/// Encode one component (subject or object) of a triple term, recursing on nested triple
/// terms so `<<( <<( … )>> p o )>>` round-trips.
fn encode_triple_component(graph: &SerGraph, idx: usize) -> Result<CarrierTerm, RdfDiagnostic> {
    let term = &graph.terms[idx];
    if term.kind == SerTermKind::Triple {
        build_triple_term_value(graph, term)
    } else {
        term_to_value(graph, term)
    }
}

/// Convert a single RDF term to its JSON-LD value-object form.
fn term_to_value(graph: &SerGraph, term: &SerTerm) -> Result<CarrierTerm, RdfDiagnostic> {
    match term.kind {
        SerTermKind::Iri | SerTermKind::Bnode => Ok(CarrierTerm::Id(term_id(term)?)),
        SerTermKind::Literal => {
            let datatype = datatype_iri(graph, term);
            // Key @language / @direction off the carrier's FIRST-CLASS language /
            // direction fields, not solely the datatype IRI: the native model carries a
            // directional-language string as `rdf:langString` + a separate `direction`,
            // so a datatype-only test would drop @direction.
            let language = term.lang.clone();
            let direction = term.direction.clone();
            let datatype = if datatype == RDF_DIR_LANG_STRING
                || datatype == RDF_LANG_STRING
                || datatype == XSD_STRING
                || language.is_some()
            {
                None
            } else {
                Some(absolute_iri(&datatype))
            };
            Ok(CarrierTerm::Literal(CarrierLiteral {
                lexical: term.value.clone().unwrap_or_default(),
                datatype,
                language,
                direction,
            }))
        }
        SerTermKind::Triple => Err(parse(
            "term_to_value does not handle triple terms; caller should use build_value_object"
                .to_string(),
        )),
    }
}

/// Build the annotation node object for a single reifier.
fn build_annotation_node(
    graph: &SerGraph,
    reifier_id: usize,
    g: Option<usize>,
    indexes: &Indexes<'_>,
) -> Result<CarrierNode, RdfDiagnostic> {
    let reifier_term = &graph.terms[reifier_id];
    let mut node = CarrierNode::new(term_id(reifier_term)?);

    if let Some(anns) = indexes.annotations_of.get(&(reifier_id, g)) {
        for &(p, v) in anns {
            let p_term = &graph.terms[p];
            let p_iri = p_term
                .value
                .as_deref()
                .ok_or_else(|| parse("annotation predicate missing IRI".to_string()))?;
            let v_term = &graph.terms[v];
            let value = CarrierValue::plain(simple_term_value(graph, v_term)?);
            node.properties
                .entry(absolute_iri(p_iri))
                .or_default()
                .push(value);
        }
    }
    node.sort_values();
    Ok(node)
}

/// Build a standalone node for a reifier whose base triple is not asserted:
/// `{"@id": r, "rdf:reifies": {"@triple": …}, <annotation props>}`. On re-parse the
/// `rdf:reifies` row folds back into the RDF-1.2 statement layer with its annotations, so
/// the reifier round-trips instead of being dropped for want of a base value object.
fn build_orphan_reifier_node(
    graph: &SerGraph,
    reifier_id: usize,
    s: usize,
    p: usize,
    o: usize,
    g: Option<usize>,
    indexes: &Indexes<'_>,
) -> Result<CarrierNode, RdfDiagnostic> {
    let mut node = build_annotation_node(graph, reifier_id, g, indexes)?;
    node.properties
        .entry(absolute_iri(RDF_REIFIES))
        .or_default()
        .push(CarrierValue::plain(build_nested_triple_node(
            graph, s, p, o,
        )?));
    node.sort_values();
    Ok(node)
}

/// Convert a term to a value object without recursive triple-term handling.
fn simple_term_value(graph: &SerGraph, term: &SerTerm) -> Result<CarrierTerm, RdfDiagnostic> {
    if term.kind == SerTermKind::Triple {
        build_triple_term_value(graph, term)
    } else {
        term_to_value(graph, term)
    }
}

/// Return a stable `@id` string for an IRI or blank node term.
fn term_id(term: &SerTerm) -> Result<String, RdfDiagnostic> {
    match term.kind {
        SerTermKind::Iri => Ok(term
            .value
            .as_deref()
            .map_or_else(|| "_:missing-iri".to_string(), absolute_iri)),
        SerTermKind::Bnode => Ok(format!(
            "_:{}",
            term.value.as_deref().unwrap_or("missing-bnode")
        )),
        SerTermKind::Literal => Err(parse("expected IRI or blank node, got literal".to_string())),
        SerTermKind::Triple => Err(parse(
            "expected IRI or blank node, got triple term".to_string(),
        )),
    }
}

/// Return a stable, lexical sort key for an RDF term.
///
/// Unlike raw term ids, this key is independent of the order in which terms
/// were appended to the graph, so it is safe to use when normalizing output.
fn term_sort_key(graph: &SerGraph, term: &SerTerm) -> String {
    match term.kind {
        SerTermKind::Iri | SerTermKind::Bnode => term_id(term).unwrap_or_default(),
        SerTermKind::Literal => {
            let mut key = format!("lit:{}", term.value.as_deref().unwrap_or_default());
            if let Some(lang) = &term.lang {
                let _ = write!(key, "@{lang}");
            }
            if let Some(dir) = &term.direction {
                let _ = write!(key, "^{dir}");
            }
            let _ = write!(key, "^^{}", datatype_iri(graph, term));
            key
        }
        SerTermKind::Triple => match triple_components(graph, term) {
            Some((s, p, o)) => format!("triple:{s}:{p}:{o}"),
            None => "triple:none".to_string(),
        },
    }
}

/// Preserve a source IRI verbatim at the vocabulary-free serialization boundary.
fn absolute_iri(iri: &str) -> String {
    iri.to_string()
}

/// Deterministic comparison of JSON-LD value objects.
fn cmp_value(a: &Value, b: &Value) -> std::cmp::Ordering {
    let key = |v: &Value| -> String {
        if let Some(s) = v.as_str() {
            return format!("0:{s}");
        }
        if let Some(obj) = v.as_object() {
            let mut parts: Vec<String> = Vec::new();
            if let Some(id) = obj.get("@id").and_then(Value::as_str) {
                parts.push(format!("id={id}"));
            }
            if let Some(val) = obj.get("@value").and_then(Value::as_str) {
                parts.push(format!("value={val}"));
            }
            if let Some(lang) = obj.get("@language").and_then(Value::as_str) {
                parts.push(format!("lang={lang}"));
            }
            if let Some(dir) = obj.get("@direction").and_then(Value::as_str) {
                parts.push(format!("dir={dir}"));
            }
            if let Some(dt) = obj.get("@type").and_then(Value::as_str) {
                parts.push(format!("dt={dt}"));
            }
            parts.sort();
            return format!("1:{}", parts.join("|"));
        }
        format!("2:{v}")
    };
    key(a).cmp(&key(b))
}

// ── parse side: JSON-LD-star → native carrier ───────────────────────────────────────

/// Parse JSON-LD-star bytes into the native carrier [`RdfDataset`].
///
/// This is the inverse of [`serialize_dataset_to_jsonld`]: it interprets the
/// `@annotation` idiom produced by the PurRDF JSON-LD-star emitter and reconstructs RDF
/// 1.2 reifier quads (`rdf:reifies` with quoted triple objects) plus graph-scoped
/// annotation triples. Those rows are folded into the dataset's RDF 1.2 statement layer
/// at freeze time. Named graphs and directional language strings are preserved; a shape
/// that cannot be represented by the RDF dataset fails before data is discarded.
pub fn parse_jsonld(json_bytes: &[u8]) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
    let context = CompiledJsonLdContext::compile(&to_json_object(BTreeMap::new()), None)?;
    parse_jsonld_with_context(json_bytes, &context)
}

/// Parse JSON-LD-star bytes through a reusable compiled active context.
///
/// The compiled context supplies both the initial active context and the immutable
/// offline registry used by any context IRI or `@import` found in the document. This is
/// the inverse surface for registry-backed configured serialization; no network lookup
/// is attempted.
pub fn parse_jsonld_with_context(
    json_bytes: &[u8],
    context: &CompiledJsonLdContext,
) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
    let value = context::parse_document(json_bytes)?;
    let carrier = expand::expand_document(&value, context)?;
    expand::carrier_to_dataset(&carrier)
}

/// Validate `iri` as an absolute IRI and return the native term.
fn validated_iri_term(iri: &str) -> Result<RdfTerm, RdfDiagnostic> {
    purrdf_sparql_algebra::NamedNode::new(iri.to_owned())
        .map_err(|source| decode(source.to_string()))?;
    Ok(RdfTerm::iri(iri))
}

// ── statement-metadata downcast ─────────────────────────────────────────────────────

/// Convert a JSON-LD-star document to statement-metadata N-Quads in the
/// caller's reification vocabulary (see [`StatementMetadataVocab`]).
///
/// RDF 1.2 quoted triples (`?r rdf:reifies <<( ?s ?p ?o )>>`) cannot be represented by
/// rdflib-based consumers, so this downcast re-expresses each annotated statement as a
/// flat statement-metadata cell in the CALLER's vocabulary (shown here with an
/// illustrative `meta:` consumer namespace):
///
/// ```turtle
/// ?r a meta:StatementMetadata ;
///    meta:qSubject ?s ;
///    meta:qPredicate ?p ;
///    meta:qObject ?o | meta:qObjectLiteral ?o ;
///    <annotation-pred> <annotation-value> .
/// ```
///
/// The base triple `?s ?p ?o` is retained, and every annotation triple on the reifier is
/// carried through unchanged. The output contains no quoted triples, so it is safe for
/// the rdflib-compat up-projection lane.
///
/// PurRDF mints no vocabulary of its own, so there is NO default vocabulary:
/// input carrying quoted triples / reifier annotations hard-fails when `vocab`
/// is `None`, while star-free input downcasts fine unconfigured.
pub fn jsonld_to_statement_metadata_nquads(
    json_bytes: &[u8],
    vocab: Option<&StatementMetadataVocab<'_>>,
) -> Result<String, RdfDiagnostic> {
    let dataset = parse_jsonld(json_bytes)?;

    // Flatten the carrier back to the source-faithful quad stream, re-materializing the
    // RDF 1.2 statement overlay as un-folded `rdf:reifies` reifier rows + annotation
    // rows (the exact inverse of the `dataset_from_quads` fold).
    let quads = crate::flat_rdf_quads_from_dataset(&dataset);

    // Identify reifiers and the quoted triple each one refers to.
    let mut reifier_quotes: std::collections::HashMap<RdfTerm, (RdfTerm, String, RdfTerm)> =
        std::collections::HashMap::new();
    for quad in &quads {
        if quad.predicate == RDF_REIFIES
            && let RdfTerm::Triple(triple) = &quad.object
        {
            reifier_quotes.insert(
                quad.subject.clone(),
                (
                    triple.subject.clone(),
                    triple.predicate.clone(),
                    triple.object.clone(),
                ),
            );
        }
    }

    let mut out: Vec<RdfQuad> = Vec::new();

    for quad in &quads {
        if quad.predicate == RDF_REIFIES {
            // The statement-metadata skeleton is minted in the CALLER's
            // vocabulary — star input without a configured vocab fails closed.
            let Some(vocab) = vocab else {
                return Err(parse(
                    "JSON-LD-star downcast requires a statement-metadata vocabulary; \
                     supply StatementMetadataVocab"
                        .to_string(),
                ));
            };
            // Emit the statement-metadata skeleton for this reifier.
            let Some((s, p, o)) = reifier_quotes.get(&quad.subject) else {
                continue;
            };
            let r = quad.subject.clone();
            out.push(RdfQuad::new(
                r.clone(),
                RDF_TYPE,
                RdfTerm::iri(vocab.statement_metadata),
            ));
            out.push(RdfQuad::new(r.clone(), vocab.q_subject, s.clone()));
            out.push(RdfQuad::new(
                r.clone(),
                vocab.q_predicate,
                RdfTerm::iri(p.clone()),
            ));
            let q_object_pred = if matches!(o, RdfTerm::Literal(_)) {
                vocab.q_object_literal
            } else {
                vocab.q_object
            };
            out.push(RdfQuad::new(r.clone(), q_object_pred, o.clone()));
        } else if reifier_quotes.contains_key(&quad.subject) {
            // Annotation triple on a reifier: keep it, but in the default graph so the
            // downstream rdflib-compat graph (single-graph) sees it.
            out.push(RdfQuad::new(
                quad.subject.clone(),
                quad.predicate.clone(),
                quad.object.clone(),
            ));
        } else {
            // Plain base triple or named-graph triple (graph name preserved).
            out.push(quad.clone());
        }
    }

    // `out` holds only the downcast-flat statement-metadata cells (no object-position
    // quoted triples), so the native N-Quads serializer applies.
    let ir = crate::dataset_from_quads(&out).map_err(|e| decode(format!("quads → IR: {e}")))?;
    let buf = crate::serialize_dataset(&ir, "application/n-quads", SerializeGraph::Dataset)
        .map_err(|e| decode(format!("serialize N-Quads: {e}")))?;
    String::from_utf8(buf).map_err(|e| decode(format!("N-Quads are not UTF-8: {e}")))
}

/// Convert YAML-LD-star bytes to JSON-LD-star JSON, hard-failing on YAML
/// anchors/aliases (extended YAML is out of scope).
///
/// The conversion is purely structural: YAML scalars/sequences/mappings map one-to-one
/// onto JSON, so the resulting JSON is consumable by [`parse_jsonld`] and the
/// statement-metadata downcast.
pub fn yamlld_to_jsonld(yaml_bytes: &[u8]) -> Result<String, RdfDiagnostic> {
    let text = std::str::from_utf8(yaml_bytes)
        .map_err(|e| decode(format!("YAML-LD-star bytes are not UTF-8: {e}")))?;
    // Reject anchors/aliases BEFORE deserializing — extended YAML is out of scope.
    // Detection is structural (node-position only), so `&`/`*` inside scalar definition
    // prose does not false-positive.
    if yaml_uses_anchor_or_alias(text) {
        return Err(decode("YAML-LD-star must not use anchors or aliases"));
    }
    let value: serde_yaml::Value =
        serde_yaml::from_str(text).map_err(|e| decode(format!("parse YAML-LD-star: {e}")))?;
    serde_json::to_string(&value).map_err(|e| decode(format!("YAML-LD-star -> JSON-LD-star: {e}")))
}

/// Structural YAML anchor/alias detector (node-position only), so a `&`/`*` that appears
/// inside scalar prose (e.g. a `skos:definition` value) does not false-positive.
fn yaml_uses_anchor_or_alias(text: &str) -> bool {
    let mut block_scalar_indent: Option<usize> = None;
    for line in text.lines() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        // Inside a block scalar: its content is indented deeper than the header.
        if let Some(header_indent) = block_scalar_indent {
            if trimmed.is_empty() || indent > header_indent {
                continue;
            }
            block_scalar_indent = None;
        }
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        // Strip leading block-sequence indicators (possibly nested: "- - x").
        let mut rest = trimmed;
        while let Some(r) = rest.strip_prefix("- ") {
            rest = r.trim_start();
        }
        if rest == "-" {
            continue;
        }
        // The node value is the text after the mapping separator (`: `), or the
        // whole remainder when this line is a bare sequence/scalar node.
        let value = block_mapping_value(rest).unwrap_or(rest).trim_start();
        // A block-scalar header (`|`, `>`, `|-`, `>+2`, …) opens here; skip its body.
        if value.starts_with('|') || value.starts_with('>') {
            block_scalar_indent = Some(indent);
            continue;
        }
        if value.starts_with('&') || value.starts_with('*') {
            return true;
        }
    }
    false
}

/// The block-mapping node value: the text after the `: ` separator, or `None` if the
/// line is not a `key: value` mapping entry. A quoted key is skipped first so a `:`
/// inside it is not mistaken for the separator, and IRIs/curies (`https://…`,
/// `ex:foo`) keep their `:`-without-space.
fn block_mapping_value(s: &str) -> Option<&str> {
    let bytes = s.as_bytes();
    let mut i = 0;
    if let Some(&q @ (b'\'' | b'"')) = bytes.first() {
        i = 1;
        while i < bytes.len() && bytes[i] != q {
            i += 1;
        }
        i = (i + 1).min(bytes.len());
    }
    while i < bytes.len() {
        if bytes[i] == b':' && (i + 1 == bytes.len() || bytes[i + 1] == b' ') {
            return Some(if i + 2 <= bytes.len() {
                &s[i + 1..]
            } else {
                ""
            });
        }
        i += 1;
    }
    None
}

/// Downcast YAML-LD-star bytes to statement-metadata N-Quads in the caller's
/// reification vocabulary (see [`StatementMetadataVocab`]).
///
/// Routes through [`yamlld_to_jsonld`] then [`jsonld_to_statement_metadata_nquads`], so
/// the output contains no quoted triple terms and is safe for the rdflib-compat
/// up-projection lane. As with the JSON-LD-star downcast there is NO default
/// vocabulary: star input hard-fails when `vocab` is `None`.
pub fn yamlld_to_statement_metadata_nquads(
    yaml_bytes: &[u8],
    vocab: Option<&StatementMetadataVocab<'_>>,
) -> Result<String, RdfDiagnostic> {
    jsonld_to_statement_metadata_nquads(yamlld_to_jsonld(yaml_bytes)?.as_bytes(), vocab)
}

// ── diagnostic constructors ─────────────────────────────────────────────────────────

/// A JSON-LD/YAML-LD decode diagnostic (malformed input / surface-encoding error).
fn decode(message: impl Into<String>) -> RdfDiagnostic {
    RdfDiagnostic::error("native-jsonld-decode", message)
}

/// A JSON-LD/YAML-LD parse diagnostic (well-formed surface that does not map to RDF).
fn parse(message: impl Into<String>) -> RdfDiagnostic {
    RdfDiagnostic::error("native-jsonld-parse", message)
}

#[cfg(test)]
mod carrier_law_tests {
    use proptest::prelude::*;
    use serde_json::json;

    use super::*;

    fn assert_exact_carrier_lens(dataset: &RdfDataset, context_value: &Value) {
        let graph = build_ser_graph(
            dataset,
            NativeRdfFormat::NQuads,
            SerializeGraph::Dataset,
            true,
        )
        .expect("serialization graph");
        let expanded = build_carrier(&graph, true).expect("typed expanded carrier");
        let context = CompiledJsonLdContext::compile(context_value, None).expect("context");
        let compacted = serialize_carrier_compacted(&expanded, &context).expect("compaction");
        let document = context::parse_document(compacted.as_bytes()).expect("strict JSON");
        let initial = CompiledJsonLdContext::compile(&json!({}), None).expect("empty context");
        let reexpanded = expand::expand_document(&document, &initial).expect("re-expansion");
        assert_eq!(expanded, reexpanded, "compacted document:\n{compacted}");
    }

    #[test]
    fn rich_rdf_1_2_carrier_is_an_exact_context_lens() {
        let source = concat!(
            "<https://example.org/s> <https://example.org/items> _:head .\n",
            "_:head <http://www.w3.org/1999/02/22-rdf-syntax-ns#first> \"one\"@en .\n",
            "_:head <http://www.w3.org/1999/02/22-rdf-syntax-ns#rest> <http://www.w3.org/1999/02/22-rdf-syntax-ns#nil> .\n",
            "<https://example.org/s> <https://example.org/p> <https://example.org/o> <https://example.org/g> .\n",
            "<https://example.org/meta> <https://example.org/quotes> <<( <https://example.org/s> <https://example.org/p> <https://example.org/o> )>> .\n",
            "_:r <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> <<( <https://example.org/s> <https://example.org/p> <https://example.org/o> )>> .\n",
            "_:r <https://example.org/source> <https://example.org/doc> .\n",
        );
        let dataset = crate::parse_dataset(source.as_bytes(), "application/n-quads", None)
            .expect("rich RDF 1.2 fixture");
        assert_exact_carrier_lens(
            &dataset,
            &json!({
                "ex": {"@id": "https://example.org/", "@prefix": true},
                "items": {"@id": "ex:items", "@container": "@list", "@language": "en"}
            }),
        );
    }

    #[test]
    fn materialized_budget_counts_reused_term_payload_per_occurrence() {
        let literal = "x".repeat(1_024);
        let one = format!("<https://example.org/s0> <https://example.org/p> \"{literal}\" .\n");
        let mut many = String::new();
        for index in 0..4 {
            writeln!(
                many,
                "<https://example.org/s{index}> <https://example.org/p> \"{literal}\" ."
            )
            .expect("write budget fixture");
        }
        let build = |source: &str| {
            let dataset = crate::parse_dataset(source.as_bytes(), "application/n-quads", None)
                .expect("budget fixture");
            build_ser_graph(
                &dataset,
                NativeRdfFormat::NQuads,
                SerializeGraph::Dataset,
                true,
            )
            .expect("serialization graph")
        };
        let annotations = AnnotationIndex::new();
        assert!(
            validate_materialized_carrier_budget_with_limits(
                &build(&one),
                &annotations,
                MAX_JSON_LD_CARRIER_ROWS,
                10_000,
            )
            .is_ok()
        );
        let error = validate_materialized_carrier_budget_with_limits(
            &build(&many),
            &annotations,
            MAX_JSON_LD_CARRIER_ROWS,
            10_000,
        )
        .expect_err("reused literal clones exceed materialized budget");
        assert_eq!(error.code, "native-jsonld-decode");
        assert!(error.message.contains("working bytes"));
    }

    proptest! {
        #[test]
        fn generated_carriers_obey_exact_compact_expand_equality(
            rows in prop::collection::btree_set(("[a-z]{1,8}", "[a-z]{1,8}"), 1..32)
        ) {
            let mut source = String::new();
            for (predicate, object) in rows {
                writeln!(
                    source,
                    "<https://example.org/s> <https://example.org/{predicate}> <https://example.org/{object}> ."
                )
                .expect("write fixture");
            }
            let dataset = crate::parse_dataset(source.as_bytes(), "application/n-quads", None)
                .expect("generated fixture");
            assert_exact_carrier_lens(
                &dataset,
                &json!({"ex": {"@id": "https://example.org/", "@prefix": true}}),
            );
        }
    }
}
