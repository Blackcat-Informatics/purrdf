// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! First-party JSON-LD-star / YAML-LD-star codec.
//!
//! Serializes the frozen [`RdfDataset`] to the PurRDF JSON-LD-star lead artifact and a
//! deterministic YAML-LD-star derivative, and parses both back into the native carrier.
//! The serializer walks the first-party [`SerGraph`] (the same shape the Turtle / TriG /
//! N-Triples / N-Quads serializers walk), built from the frozen IR via
//! [`build_ser_graph`](super::serialize::build_ser_graph) — so it shares one lowering
//! and never touches the external `purrdf-gts` codecs. GTS is exit-only.
//!
//! The JSON output is byte-deterministic: every map is a [`BTreeMap`] and every array is
//! explicitly sorted, so the document does not depend on input append order.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::Arc;

use serde_json::Value;

use super::ser_model::{SerGraph, SerTerm, SerTermKind};
use super::serialize::build_ser_graph;
use super::NativeRdfFormat;
use crate::{
    RdfDataset, RdfDiagnostic, RdfLiteral, RdfQuad, RdfTerm, RdfTextDirection, RdfTriple,
    SerializeGraph,
};

// Literal datatype sentinels (read off the carrier's first-class literal fields).
const RDF_DIR_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString";
const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
/// Schema reference for the YAML-LD language-server header when the output is
/// consumed from the bundled `purrdf.gts` snapshot. The schema is shipped as
/// `schemas-archive/purrdf.schema.json`, so a bare member name resolves inside the
/// bundle.
const BUNDLED_SCHEMA_REF: &str = "purrdf.schema.json";

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

// Longest-namespace-first prefix table of well-known PUBLIC namespaces
// (mirrors `src/purrdf_tools/config.py`).
include!("lpg_prefixes.rs");

/// Default-graph and named-graph node maps returned by [`build_graphs`].
type GraphNodes = (BTreeMap<String, Value>, BTreeMap<String, Value>);
/// Reifier lookup: base triple (s,p,o) -> reifier ids that annotate it.
type ReifierIndex = BTreeMap<(usize, usize, usize), Vec<usize>>;
/// Annotation lookup: reifier id -> sorted annotation (predicate, value) rows.
type AnnotationIndex = BTreeMap<usize, Vec<(usize, usize)>>;
/// Quads grouped by graph name and then by subject.
type QuadGroups = BTreeMap<Option<usize>, BTreeMap<usize, Vec<(usize, usize)>>>;

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

/// Serialize the carrier dataset to a deterministic JSON-LD-star document.
pub fn serialize_dataset_to_jsonld(dataset: &RdfDataset) -> Result<String, RdfDiagnostic> {
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

/// Serialize an already-materialized [`SerGraph`] to a deterministic JSON-LD-star
/// document.
fn serialize_ser_graph(graph: &SerGraph) -> Result<String, RdfDiagnostic> {
    let mut doc = BTreeMap::new();
    doc.insert("@context".to_string(), build_context());

    let (default_nodes, named_graphs) = build_graphs(graph)?;

    let mut top_graph: Vec<Value> = default_nodes.into_values().collect();
    for (_, graph_obj) in named_graphs {
        top_graph.push(graph_obj);
    }
    // Deterministic order: default-graph nodes by @id, then named graphs by @id.
    top_graph.sort_by_key(json_key);

    if !top_graph.is_empty() {
        doc.insert("@graph".to_string(), Value::Array(top_graph));
    }

    let value = to_json_object(doc);
    serde_json::to_string_pretty(&value).map_err(|e| decode(format!("JSON-LD serialization: {e}")))
}

/// Serialize the carrier dataset to deterministic YAML-LD-star bytes.
///
/// The JSON-LD-star document is re-serialized to YAML with sorted keys, block style, no
/// anchors/aliases, and an explicit `@context`. The header carries a YAML
/// language-server schema reference.
pub fn serialize_dataset_to_yamlld(
    dataset: &RdfDataset,
    schema_url: Option<&str>,
) -> Result<String, RdfDiagnostic> {
    let json = serialize_dataset_to_jsonld(dataset)?;
    let value: Value =
        serde_json::from_str(&json).map_err(|e| decode(format!("parse JSON-LD for YAML: {e}")))?;
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

/// Build the JSON-LD `@context` from the public prefix registry.
///
/// No `@vocab` is emitted: PurRDF mints no vocabulary namespace of its own,
/// and the emitter always writes CURIEs or absolute IRIs (never
/// vocab-relative terms), so a default vocabulary would be pure fabrication.
fn build_context() -> Value {
    let mut ctx = BTreeMap::new();
    for (prefix, namespace) in PREFIXES_BY_LEN.iter().rev() {
        // Reverse gives prefix-name order for deterministic insertion, but
        // BTreeMap sorts by key anyway.
        ctx.insert(prefix.to_string(), Value::String(namespace.to_string()));
    }
    to_json_object(ctx)
}

/// Build default-graph nodes and named-graph objects.
fn build_graphs(graph: &SerGraph) -> Result<GraphNodes, RdfDiagnostic> {
    // Reifier index: base triple (s,p,o) -> reifier ids that annotate it.
    let mut reifier_of: ReifierIndex = BTreeMap::new();
    for &(rid, (s, p, o), _g) in &graph.reifiers {
        reifier_of.entry((s, p, o)).or_default().push(rid);
    }
    for list in reifier_of.values_mut() {
        // Sort by the reifier's stable @id, not its input-order term id.
        list.sort_by(|a, b| {
            let a_id = term_id(&graph.terms[*a]).expect("reifier must be IRI or blank node");
            let b_id = term_id(&graph.terms[*b]).expect("reifier must be IRI or blank node");
            a_id.cmp(&b_id)
        });
    }

    // Annotation index: reifier id -> sorted annotation (predicate, value) rows.
    let mut annotations_of: AnnotationIndex = BTreeMap::new();
    for &(r, p, v, _g) in &graph.annotations {
        annotations_of.entry(r).or_default().push((p, v));
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

    let mut default_nodes: BTreeMap<String, Value> = BTreeMap::new();
    let mut named_graphs: BTreeMap<String, Value> = BTreeMap::new();

    for (g, subjects) in by_graph {
        let mut nodes: Vec<Value> = Vec::new();
        for (s, pos) in subjects {
            let node = build_node(graph, s, pos, &reifier_of, &annotations_of)?;
            nodes.push(node);
        }
        // Sort nodes by their @id (or lexical key for bnodes).
        nodes.sort_by_key(node_id_key);

        match g {
            None => {
                for node in nodes {
                    if let Some(Value::String(id)) = node.get("@id") {
                        default_nodes.insert(id.clone(), node);
                    } else {
                        // Bnode subject without @id should not happen because we always
                        // emit _:label; keep a stable fallback key.
                        default_nodes.insert(format!("__bnode:{node:?}"), node);
                    }
                }
            }
            Some(gid) => {
                let graph_term = &graph.terms[gid];
                let graph_id = term_id(graph_term)?;
                let mut graph_obj = BTreeMap::new();
                graph_obj.insert("@id".to_string(), Value::String(graph_id.clone()));
                graph_obj.insert("@graph".to_string(), Value::Array(nodes));
                named_graphs.insert(graph_id, to_json_object(graph_obj));
            }
        }
    }

    Ok((default_nodes, named_graphs))
}

/// Build one node object for a subject from its predicate/object rows.
fn build_node(
    graph: &SerGraph,
    subject: usize,
    pos: Vec<(usize, usize)>,
    reifier_of: &ReifierIndex,
    annotations_of: &AnnotationIndex,
) -> Result<Value, RdfDiagnostic> {
    let subject_term = &graph.terms[subject];
    let mut node = BTreeMap::new();
    node.insert("@id".to_string(), Value::String(term_id(subject_term)?));

    // Group predicate -> objects, preserving rdf:type separately.
    let mut types: Vec<Value> = Vec::new();
    let mut props: BTreeMap<String, Vec<Value>> = BTreeMap::new();

    for (p, o) in pos {
        let predicate_term = &graph.terms[p];
        let predicate_iri = predicate_term
            .value
            .as_deref()
            .ok_or_else(|| parse("predicate missing IRI value".to_string()))?;
        let object_term = &graph.terms[o];

        if predicate_iri == RDF_TYPE {
            types.push(term_ref_value(object_term)?);
        } else {
            let key = curie(predicate_iri);
            let value = build_value_object(
                graph,
                subject,
                p,
                o,
                object_term,
                reifier_of,
                annotations_of,
            )?;
            props.entry(key).or_default().push(value);
        }
    }

    if !types.is_empty() {
        types.sort_by(cmp_value);
        node.insert("@type".to_string(), Value::Array(types));
    }

    for (key, mut values) in props {
        values.sort_by(cmp_value);
        let value = if values.len() == 1 {
            values.into_iter().next().unwrap()
        } else {
            Value::Array(values)
        };
        node.insert(key, value);
    }

    Ok(to_json_object(node))
}

/// Build a value object for a quad object, attaching `@annotation` when the
/// base triple is reified.
fn build_value_object(
    graph: &SerGraph,
    subject: usize,
    predicate: usize,
    object: usize,
    object_term: &SerTerm,
    reifier_of: &ReifierIndex,
    annotations_of: &AnnotationIndex,
) -> Result<Value, RdfDiagnostic> {
    let mut value = if object_term.kind == SerTermKind::Triple {
        build_triple_term_value(graph, object_term, reifier_of, annotations_of)?
    } else {
        term_to_value(graph, object_term)?
    };

    if let Some(reifiers) = reifier_of.get(&(subject, predicate, object)) {
        let annotations: Result<Vec<Value>, _> = reifiers
            .iter()
            .map(|&rid| build_annotation_node(graph, rid, annotations_of))
            .collect();
        let annotations = annotations?;
        let ann_value = if annotations.len() == 1 {
            annotations.into_iter().next().unwrap()
        } else {
            Value::Array(annotations)
        };
        // Attach @annotation to the value object.
        if let Value::Object(ref mut map) = value {
            map.insert("@annotation".to_string(), ann_value);
        } else {
            // Wrap a non-object value (should not happen for annotated triples)
            // into a value object with @annotation.
            let mut wrapper = BTreeMap::new();
            wrapper.insert("@value".to_string(), value);
            wrapper.insert("@annotation".to_string(), ann_value);
            value = to_json_object(wrapper);
        }
    }

    Ok(value)
}

/// Render a triple term (object position) as its JSON-LD-star annotated node
/// object, using the term's own reifier binding.
fn build_triple_term_value(
    graph: &SerGraph,
    term: &SerTerm,
    reifier_of: &ReifierIndex,
    annotations_of: &AnnotationIndex,
) -> Result<Value, RdfDiagnostic> {
    let (s, p, o) = triple_components(graph, term)
        .ok_or_else(|| parse("triple term with no components".to_string()))?;
    build_nested_triple_node(graph, s, p, o, reifier_of, annotations_of)
}

/// Build the JSON-LD-star annotated node object for a quoted triple (s,p,o).
fn build_nested_triple_node(
    _graph: &SerGraph,
    _s: usize,
    _p: usize,
    _o: usize,
    _reifier_of: &ReifierIndex,
    _annotations_of: &AnnotationIndex,
) -> Result<Value, RdfDiagnostic> {
    // A bare quoted-triple in object position would have to be encoded as
    // `{"@id": s, <p-curie>: <object>}`, which (a) is indistinguishable from an
    // ordinary node object and (b) is not parseable back: the parser's `@id`
    // branch returns only the subject IRI and drops the predicate/object,
    // silently corrupting the triple term. Rather than emit that lossy,
    // ambiguous form we fail closed. The lossless, supported representation for
    // RDF-1.2-star here is the rdf:reifies / `@annotation` form, which is
    // unaffected by this guard. Full lossless nested-triple-term support
    // (object-position and annotation-value triple terms) is a deferred
    // extension requiring a distinguishable JSON-LD-star encoding.
    Err(parse(
        "quoted-triple object terms are not yet losslessly serializable in JSON-LD-star; \
         use the rdf:reifies/@annotation form"
            .to_string(),
    ))
}

/// Convert a single RDF term to its JSON-LD value-object form.
fn term_to_value(graph: &SerGraph, term: &SerTerm) -> Result<Value, RdfDiagnostic> {
    match term.kind {
        SerTermKind::Iri | SerTermKind::Bnode => {
            let mut map = BTreeMap::new();
            map.insert("@id".to_string(), Value::String(term_id(term)?));
            Ok(to_json_object(map))
        }
        SerTermKind::Literal => {
            let mut map = BTreeMap::new();
            map.insert(
                "@value".to_string(),
                Value::String(term.value.clone().unwrap_or_default()),
            );
            let datatype = datatype_iri(graph, term);
            // Key @language / @direction off the carrier's FIRST-CLASS language /
            // direction fields, not solely the datatype IRI: the native model carries a
            // directional-language string as `rdf:langString` + a separate `direction`,
            // so a datatype-only test would drop @direction.
            if datatype == RDF_DIR_LANG_STRING || term.direction.is_some() {
                if let Some(lang) = &term.lang {
                    map.insert("@language".to_string(), Value::String(lang.clone()));
                }
                if let Some(dir) = &term.direction {
                    map.insert("@direction".to_string(), Value::String(dir.clone()));
                }
            } else if datatype == RDF_LANG_STRING || term.lang.is_some() {
                if let Some(lang) = &term.lang {
                    map.insert("@language".to_string(), Value::String(lang.clone()));
                }
            } else if datatype != XSD_STRING {
                map.insert("@type".to_string(), Value::String(curie(&datatype)));
            }
            Ok(to_json_object(map))
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
    annotations_of: &AnnotationIndex,
) -> Result<Value, RdfDiagnostic> {
    let reifier_term = &graph.terms[reifier_id];
    let mut node = BTreeMap::new();
    node.insert("@id".to_string(), Value::String(term_id(reifier_term)?));

    if let Some(anns) = annotations_of.get(&reifier_id) {
        let mut props: BTreeMap<String, Vec<Value>> = BTreeMap::new();
        for &(p, v) in anns {
            let p_term = &graph.terms[p];
            let p_iri = p_term
                .value
                .as_deref()
                .ok_or_else(|| parse("annotation predicate missing IRI".to_string()))?;
            let v_term = &graph.terms[v];
            let value = simple_term_value(graph, v_term)?;
            props.entry(curie(p_iri)).or_default().push(value);
        }
        for (key, mut values) in props {
            values.sort_by(cmp_value);
            let value = if values.len() == 1 {
                values.into_iter().next().unwrap()
            } else {
                Value::Array(values)
            };
            node.insert(key, value);
        }
    }

    Ok(to_json_object(node))
}

/// Convert a term to a value object without recursive triple-term handling.
fn simple_term_value(graph: &SerGraph, term: &SerTerm) -> Result<Value, RdfDiagnostic> {
    match term.kind {
        SerTermKind::Iri | SerTermKind::Bnode => {
            let mut map = BTreeMap::new();
            map.insert("@id".to_string(), Value::String(term_id(term)?));
            Ok(to_json_object(map))
        }
        SerTermKind::Literal => {
            let mut map = BTreeMap::new();
            map.insert(
                "@value".to_string(),
                Value::String(term.value.clone().unwrap_or_default()),
            );
            let datatype = datatype_iri(graph, term);
            // Same first-class language/direction handling as `term_to_value`.
            if datatype == RDF_DIR_LANG_STRING || term.direction.is_some() {
                if let Some(lang) = &term.lang {
                    map.insert("@language".to_string(), Value::String(lang.clone()));
                }
                if let Some(dir) = &term.direction {
                    map.insert("@direction".to_string(), Value::String(dir.clone()));
                }
            } else if datatype == RDF_LANG_STRING || term.lang.is_some() {
                if let Some(lang) = &term.lang {
                    map.insert("@language".to_string(), Value::String(lang.clone()));
                }
            } else if datatype != XSD_STRING {
                map.insert("@type".to_string(), Value::String(curie(&datatype)));
            }
            Ok(to_json_object(map))
        }
        // Triple-valued annotation objects (an annotation whose value is itself a
        // quoted triple term) have no distinguishable, losslessly parseable
        // JSON-LD-star encoding here yet. Emitting a placeholder literal would
        // silently corrupt RDF-1.2-star data, so we fail closed. Full lossless
        // nested-triple-term support (both object-position and annotation-value
        // triple terms) is a deferred extension that requires a distinguishable
        // JSON-LD-star encoding; until then "lossless or hard-fail" wins.
        SerTermKind::Triple => Err(parse(
            "triple-valued annotation objects are not yet losslessly serializable; \
             refusing to emit a lossy placeholder"
                .to_string(),
        )),
    }
}

/// A value object for `rdf:type` targets (always IRI/bnode references).
fn term_ref_value(term: &SerTerm) -> Result<Value, RdfDiagnostic> {
    let mut map = BTreeMap::new();
    map.insert("@id".to_string(), Value::String(term_id(term)?));
    Ok(to_json_object(map))
}

/// Return a stable `@id` string for an IRI or blank node term.
fn term_id(term: &SerTerm) -> Result<String, RdfDiagnostic> {
    match term.kind {
        SerTermKind::Iri => Ok(term
            .value
            .as_deref()
            .map_or_else(|| "_:missing-iri".to_string(), curie)),
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

/// Compact an IRI to a CURIE using the longest matching prefix.
fn curie(iri: &str) -> String {
    for (prefix, ns) in PREFIXES_BY_LEN {
        if let Some(rest) = iri.strip_prefix(ns) {
            return format!("{prefix}:{rest}");
        }
    }
    iri.to_string()
}

/// Sort key for a top-level @graph entry (named graph object or default node).
fn json_key(value: &Value) -> String {
    match value {
        Value::Object(map) => map
            .get("@id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        _ => String::new(),
    }
}

/// Sort key for a node object used while ordering the default-graph nodes.
fn node_id_key(value: &Value) -> String {
    json_key(value)
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
/// 1.2 reifier quads (`rdf:reifies` with quoted triple objects) plus annotation triples
/// in the default graph. Those reifier/annotation rows are FOLDED into the dataset's RDF
/// 1.2 statement layer at freeze time (`dataset_from_quads`). Named graphs and
/// directional language strings are preserved. Unsupported JSON-LD features hard-fail.
pub fn parse_jsonld(json_bytes: &[u8]) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
    let json = std::str::from_utf8(json_bytes)
        .map_err(|e| decode(format!("JSON-LD-star bytes are not UTF-8: {e}")))?;
    let value: Value =
        serde_json::from_str(json).map_err(|e| decode(format!("parse JSON-LD-star: {e}")))?;
    let mut prefixes: BTreeMap<String, String> = BTreeMap::new();
    let mut vocab = String::new();
    if let Some(Value::Object(ctx)) = value.get("@context") {
        for (k, v) in ctx {
            if k == "@vocab" {
                if let Some(ns) = v.as_str() {
                    vocab = ns.to_string();
                }
            } else if let Some(ns) = v.as_str() {
                prefixes.insert(k.clone(), ns.to_string());
            }
        }
    }

    let expand = |curie_or_iri: &str| -> String {
        if curie_or_iri.starts_with("http://") || curie_or_iri.starts_with("https://") {
            return curie_or_iri.to_string();
        }
        if let Some((p, local)) = curie_or_iri.split_once(':') {
            if let Some(ns) = prefixes.get(p) {
                return format!("{ns}{local}");
            }
        }
        if !vocab.is_empty() && !curie_or_iri.contains(':') {
            return format!("{vocab}{curie_or_iri}");
        }
        curie_or_iri.to_string()
    };

    // Accumulate native quads (including un-folded `rdf:reifies` rows); the fold to the
    // RDF 1.2 statement layer happens at `dataset_from_quads` freeze time.
    let quads: std::cell::RefCell<Vec<RdfQuad>> = std::cell::RefCell::new(Vec::new());

    let emit_node = |node: &Value, graph_iri: Option<&str>| -> Result<(), RdfDiagnostic> {
        let id = node
            .get("@id")
            .and_then(Value::as_str)
            .ok_or_else(|| decode("node without @id".to_string()))?;
        let subject: RdfTerm = node_id_term(id, &expand)?;
        // Validate the named-graph IRI (mirrors the old `NamedNode::new` Result path).
        let graph_name: Option<RdfTerm> = graph_iri
            .map(|g| validated_iri_term(&expand(g)))
            .transpose()?;

        if let Some(types) = node.get("@type") {
            // `@type` is a single value or an array; each entry is a string CURIE/IRI
            // (standard JSON-LD) or a `{"@id": …}` node reference (the PurRDF emitter
            // form). Both expand to an rdf:type object IRI.
            let entries: Vec<&Value> = match types {
                Value::Array(arr) => arr.iter().collect(),
                other => vec![other],
            };
            for t in entries {
                let t_id = t
                    .as_str()
                    .or_else(|| t.get("@id").and_then(Value::as_str))
                    .ok_or_else(|| decode("@type value is neither a string nor an @id node"))?;
                let obj = validated_iri_term(&expand(t_id))?;
                push_quad(&quads, subject.clone(), RDF_TYPE, obj, graph_name.clone());
            }
        }

        // The `@id` extraction above (`node.get("@id")…ok_or_else`) returns Err for any
        // non-object node, so reaching here guarantees `node` is a JSON object.
        let node_obj = node
            .as_object()
            .expect("emit_node already proved `node` is an object via its @id lookup");
        for (key, val) in node_obj {
            if matches!(key.as_str(), "@id" | "@type" | "@context" | "@graph") {
                continue;
            }
            let predicate = expand(key);
            // Validate the predicate IRI (mirrors the old `NamedNode::new` Result path).
            validated_iri_term(&predicate)?;
            let values = if let Value::Array(arr) = val {
                arr.clone()
            } else {
                vec![val.clone()]
            };
            for v in values {
                emit_value_quad(
                    &quads,
                    &subject,
                    &predicate,
                    graph_name.clone(),
                    &v,
                    &expand,
                )?;
            }
        }
        Ok(())
    };

    match &value {
        Value::Array(entries) => {
            for entry in entries {
                emit_graph_entry(entry, &emit_node)?;
            }
        }
        Value::Object(obj) if obj.contains_key("@graph") => {
            let graphs = obj
                .get("@graph")
                .and_then(Value::as_array)
                .ok_or_else(|| decode("@graph must be an array".to_string()))?;
            for entry in graphs {
                emit_graph_entry(entry, &emit_node)?;
            }
        }
        Value::Object(_) => {
            emit_node(&value, None)?;
        }
        _ => {
            return Err(decode(
                "JSON-LD document must be an object or array of objects".to_string(),
            ));
        }
    }

    // Freeze + fold the RDF 1.2 statement layer (a `rdf:reifies` triple-term object
    // becomes a reifier binding; a reifier subject's other triples become annotations).
    crate::dataset_from_quads(&quads.into_inner())
        .map_err(|e| parse(format!("freeze JSON-LD-star quads: {e}")))
}

/// Build an [`RdfTerm`] for a node `@id` (`_:label` blank node or expanded IRI),
/// validating the IRI through the SPARQL-algebra parser (mirrors the old
/// `oxigraph::model::NamedNode::new` Result discrimination).
fn node_id_term(id: &str, expand: &dyn Fn(&str) -> String) -> Result<RdfTerm, RdfDiagnostic> {
    if let Some(label) = id.strip_prefix("_:") {
        Ok(RdfTerm::blank_node(label.to_string()))
    } else {
        validated_iri_term(&expand(id))
    }
}

/// Validate `iri` as an absolute IRI (preserving the old `NamedNode::new` Ok/Err
/// discrimination) and return it as an [`RdfTerm`].
fn validated_iri_term(iri: &str) -> Result<RdfTerm, RdfDiagnostic> {
    purrdf_sparql_algebra::NamedNode::new(iri.to_string()).map_err(|e| decode(e.to_string()))?;
    Ok(RdfTerm::iri(iri.to_string()))
}

/// Push a base quad (optionally in a named graph) into the native accumulator.
fn push_quad(
    quads: &std::cell::RefCell<Vec<RdfQuad>>,
    subject: RdfTerm,
    predicate: &str,
    object: RdfTerm,
    graph_name: Option<RdfTerm>,
) {
    let mut quad = RdfQuad::new(subject, predicate, object);
    if let Some(g) = graph_name {
        quad = quad.in_graph(g);
    }
    quads.borrow_mut().push(quad);
}

type EmitNodeFn<'a> = dyn Fn(&Value, Option<&str>) -> Result<(), RdfDiagnostic> + 'a;

fn emit_graph_entry(entry: &Value, emit_node: &EmitNodeFn<'_>) -> Result<(), RdfDiagnostic> {
    if entry.get("@graph").is_some() {
        let graph_id = entry
            .get("@id")
            .and_then(Value::as_str)
            .ok_or_else(|| decode("named graph object must have @id".to_string()))?;
        for node in entry
            .get("@graph")
            .and_then(Value::as_array)
            .ok_or_else(|| decode("@graph must be an array".to_string()))?
        {
            emit_node(node, Some(graph_id))?;
        }
    } else {
        emit_node(entry, None)?;
    }
    Ok(())
}

fn emit_value_quad(
    quads: &std::cell::RefCell<Vec<RdfQuad>>,
    subject: &RdfTerm,
    predicate: &str,
    graph_name: Option<RdfTerm>,
    value: &Value,
    expand: &dyn Fn(&str) -> String,
) -> Result<(), RdfDiagnostic> {
    let (object, annotation) = parse_value_object(value, expand)?;
    push_quad(
        quads,
        subject.clone(),
        predicate,
        object.clone(),
        graph_name,
    );

    if let Some(ann) = annotation {
        // The emitter may attach one annotation object or an array when several
        // distinct reifiers annotate the same base triple.
        let annotations: Vec<&Value> = match &ann {
            Value::Array(arr) => arr.iter().collect(),
            other => vec![other],
        };
        for ann_node in annotations {
            let reifier_subject = ann_node
                .get("@id")
                .and_then(Value::as_str)
                .ok_or_else(|| decode("annotation without @id".to_string()))?;
            let reifier: RdfTerm = node_id_term(reifier_subject, expand)?;
            // The `rdf:reifies` quoted-triple row is pushed un-folded; the
            // `dataset_from_quads` freeze folds it into the reifier table.
            let quoted =
                RdfTerm::triple(RdfTriple::new(subject.clone(), predicate, object.clone()));
            // Reifier bindings + annotations always land in the DEFAULT graph.
            push_quad(quads, reifier.clone(), RDF_REIFIES, quoted, None);

            // The `@id` extraction above (`ann_node.get("@id")…ok_or_else`) returns Err for
            // any non-object node, so reaching here guarantees `ann_node` is a JSON object.
            let ann_obj = ann_node
                .as_object()
                .expect("the @id lookup above already proved `ann_node` is an object");
            for (key, val) in ann_obj {
                if key == "@id" {
                    continue;
                }
                let ann_predicate = expand(key);
                validated_iri_term(&ann_predicate)?;
                let vals = if let Value::Array(arr) = val {
                    arr.clone()
                } else {
                    vec![val.clone()]
                };
                for v in vals {
                    let (ann_object, _) = parse_value_object(&v, expand)?;
                    push_quad(quads, reifier.clone(), &ann_predicate, ann_object, None);
                }
            }
        }
    }

    Ok(())
}

fn parse_value_object(
    value: &Value,
    expand: &dyn Fn(&str) -> String,
) -> Result<(RdfTerm, Option<Value>), RdfDiagnostic> {
    if let Some(s) = value.as_str() {
        return Ok((validated_iri_term(&expand(s))?, None));
    }
    let obj = value
        .as_object()
        .ok_or_else(|| decode(format!("expected value object, got {value}")))?;
    let annotation = obj.get("@annotation").cloned();

    if let Some(id) = obj.get("@id").and_then(Value::as_str) {
        return Ok((node_id_term(id, expand)?, annotation));
    }

    let lex = obj
        .get("@value")
        .and_then(Value::as_str)
        .ok_or_else(|| decode("literal without @value".to_string()))?
        .to_string();
    let lang = obj.get("@language").and_then(Value::as_str);
    let direction = obj.get("@direction").and_then(Value::as_str);
    let datatype = obj.get("@type").and_then(Value::as_str);

    // The native model preserves the project's long private-use language subtags
    // (`x-purrdf-norwegiannynorsk`, >8 chars) verbatim — there is no strict tag
    // validation to reject them, matching the end-to-end preservation and the
    // lenient codecs that produced this JSON-LD-star input.
    let literal = match (lang, direction, datatype) {
        (Some(lang), Some(dir), _) => {
            let dir = match dir {
                "ltr" => RdfTextDirection::Ltr,
                "rtl" => RdfTextDirection::Rtl,
                _ => return Err(decode(format!("invalid direction {dir}"))),
            };
            RdfLiteral {
                lexical_form: lex,
                datatype: None,
                language: Some(lang.to_string()),
                direction: Some(dir),
            }
        }
        (Some(lang), None, _) => RdfLiteral::language_tagged(lex, lang),
        (None, _, Some(dt)) => {
            let dt = expand(dt);
            validated_iri_term(&dt)?;
            RdfLiteral::typed(lex, dt)
        }
        _ => RdfLiteral::simple(lex),
    };

    Ok((RdfTerm::literal(literal), annotation))
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
        if quad.predicate == RDF_REIFIES {
            if let RdfTerm::Triple(triple) = &quad.object {
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
