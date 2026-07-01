// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! JSON-LD `@graph` projector for PurRDF instance data (#700).
//!
//! Walks an oxigraph data graph (the default graph) and emits a JSON-LD document
//! `{ "@context": {<prefix map>}, "@graph": [ <node objects> ] }`. The projection
//! is the matched pair of the JSON Schema produced by [`crate::json_schema`]: a
//! projected node always validates against the schema the emitter derives from
//! the same shapes (Task 6 proves the round trip over every slice example).
//!
//! # Value conventions (lock-step with `json_schema.rs`)
//!
//! * **IRI / node ref** — `{"@id": "<compacted-iri>"}` (via
//!   [`crate::json_schema::compact_iri`]).
//! * **`rdf:type`** — folded into the node's `@type` (a CURIE string, or an array
//!   of CURIE strings when there are several).
//! * **Typed literal** — `{"@value": "<lexical>", "@type": "<compacted-datatype>"}`.
//!   Numeric (`xsd:integer`/…/`xsd:double`) and `xsd:boolean` literals are emitted
//!   as bare JSON scalars (the scalar branch the value schema's `anyOf` accepts);
//!   `xsd:string` / `rdf:langString` plain strings are emitted as bare strings.
//! * **Language-tagged literal** — `{"@value": "<lexical>", "@language": "<tag>"}`.
//!
//! Multi-valued predicates project as a JSON array; single-valued as the scalar.
//! Subjects, predicate keys, and array members are sorted for determinism.

use std::collections::BTreeMap;

use ::purrdf::RdfDataset;
use serde_json::{json, Map, Value};

use crate::data::{GraphFilter, IrDataGraph, ShaclDataGraph};
use crate::json_schema::{compact_iri, PREFIXES};
use crate::model::rdf;
use crate::term::Term;

const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema#";
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";

/// Project the default graph of `dataset` into a JSON-LD `@graph` document.
pub fn project_graph(dataset: &std::sync::Arc<RdfDataset>) -> Value {
    let data = IrDataGraph::new(std::sync::Arc::clone(dataset));
    project_graph_data(&data)
}

/// Project the default graph of a [`ShaclDataGraph`] into a JSON-LD `@graph` document.
fn project_graph_data<G: ShaclDataGraph>(data: &G) -> Value {
    // Collect distinct named-node / blank-node subjects of the default graph.
    let mut subjects: Vec<Term> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    // Scope to the DEFAULT graph only: an AnyGraph filter would match named graphs
    // too, leaking named-graph subjects into the projected `@graph`.
    for quad in data.quads_for_pattern(None, None, None, GraphFilter::DefaultGraph) {
        // Only IRI / blank-node subjects become @graph nodes (always true here).
        if quad.subject.is_subject() {
            let key = quad.subject.to_string();
            if seen.insert(key) {
                subjects.push(quad.subject);
            }
        }
    }
    subjects.sort_by_key(Term::to_string);

    let mut nodes: Vec<Value> = Vec::with_capacity(subjects.len());
    for subj in &subjects {
        nodes.push(project_subject_data(data, subj));
    }

    json!({
        "@context": context_object(),
        "@graph": Value::Array(nodes),
    })
}

/// Project a single subject term into a JSON-LD node object.
pub fn project_subject(dataset: &std::sync::Arc<RdfDataset>, subject: &Term) -> Value {
    let data = IrDataGraph::new(std::sync::Arc::clone(dataset));
    project_subject_data(&data, subject)
}

fn project_subject_data<G: ShaclDataGraph>(data: &G, subject: &Term) -> Value {
    if !subject.is_subject() {
        // Literals (and quoted triples) are never node subjects.
        return Value::Object(Map::new());
    }

    // Gather predicate → [objects], grouping by compacted predicate key.
    let mut by_pred: BTreeMap<String, Vec<Term>> = BTreeMap::new();
    let mut types: Vec<String> = Vec::new();

    for quad in data.quads_for_pattern(Some(subject), None, None, GraphFilter::DefaultGraph) {
        let pred = quad.predicate;
        if pred.as_str() == rdf::TYPE {
            if let Term::NamedNode(n) = &quad.object {
                types.push(compact_iri(n.as_str()));
            }
            continue;
        }
        by_pred
            .entry(compact_iri(pred.as_str()))
            .or_default()
            .push(quad.object);
    }

    let mut obj: Map<String, Value> = Map::new();

    // @id
    let id = match subject {
        Term::NamedNode(n) => compact_iri(n.as_str()),
        Term::BlankNode(b) => format!("_:{b}"),
        _ => String::new(),
    };
    obj.insert("@id".to_owned(), Value::String(id));

    // @type (string or array; sorted/deduped)
    if !types.is_empty() {
        types.sort();
        types.dedup();
        if types.len() == 1 {
            obj.insert("@type".to_owned(), Value::String(types.remove(0)));
        } else {
            obj.insert(
                "@type".to_owned(),
                Value::Array(types.into_iter().map(Value::String).collect()),
            );
        }
    }

    for (key, mut objects) in by_pred {
        objects.sort_by_key(ToString::to_string);
        let values: Vec<Value> = objects.iter().map(project_value).collect();
        let v = if values.len() == 1 {
            values.into_iter().next().unwrap()
        } else {
            Value::Array(values)
        };
        obj.insert(key, v);
    }

    Value::Object(obj)
}

/// Project a single object term into its JSON-LD value form.
///
/// MUST match the value-schema convention in [`crate::json_schema`].
fn project_value(term: &Term) -> Value {
    match term {
        Term::NamedNode(n) => json!({ "@id": compact_iri(n.as_str()) }),
        Term::BlankNode(b) => json!({ "@id": format!("_:{}", b.as_str()) }),
        Term::Literal(lit) => {
            if let Some(lang) = lit.language() {
                return json!({ "@value": lit.value(), "@language": lang });
            }
            let dt = lit.datatype();
            let dt_iri = dt.as_str();
            // Plain string / langString without a tag → bare string.
            if dt_iri == XSD_STRING || dt_iri == RDF_LANG_STRING {
                return Value::String(lit.value().to_owned());
            }
            // Numeric / boolean → bare JSON scalar (scalar branch of anyOf).
            if let Some(scalar) = numeric_or_bool_scalar(dt_iri, lit.value()) {
                return scalar;
            }
            // Other typed literals → the {"@value","@type"} object form.
            json!({ "@value": lit.value(), "@type": compact_iri(dt_iri) })
        }
        // Quoted triple (RDF-1.2) and any other term: stringify (statement-layer
        // reifiers are projected via @annotation, not as plain object values).
        other @ Term::Triple(_) => Value::String(other.to_string()),
    }
}

/// Map a numeric / boolean xsd datatype's lexical value to a bare JSON scalar,
/// or `None` if the datatype is not a JSON-native scalar (so the caller falls
/// back to the `{"@value","@type"}` object form).
fn numeric_or_bool_scalar(dt_iri: &str, lexical: &str) -> Option<Value> {
    let local = dt_iri.strip_prefix(XSD_NS)?;
    match local {
        "boolean" => match lexical {
            "true" | "1" => Some(Value::Bool(true)),
            "false" | "0" => Some(Value::Bool(false)),
            _ => None,
        },
        "integer" | "int" | "long" | "short" | "byte" | "nonNegativeInteger"
        | "positiveInteger" | "nonPositiveInteger" | "negativeInteger" | "unsignedLong"
        | "unsignedInt" | "unsignedShort" | "unsignedByte" => {
            lexical.parse::<i64>().ok().map(|n| json!(n))
        }
        "decimal" | "double" | "float" => lexical
            .parse::<f64>()
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map(Value::Number),
        _ => None,
    }
}

/// The JSON-LD `@context` prefix-map object (the shared [`PREFIXES`] set).
fn context_object() -> Value {
    let mut ctx: Map<String, Value> = Map::new();
    for (prefix, ns) in PREFIXES {
        ctx.insert((*prefix).to_owned(), Value::String((*ns).to_owned()));
    }
    Value::Object(ctx)
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn load(ttl: &str) -> std::sync::Arc<RdfDataset> {
        crate::text_ingest::parse_turtle_to_dataset(ttl).expect("Turtle parse")
    }

    const PREFIXES_TTL: &str = r"
        @prefix xsd:   <http://www.w3.org/2001/XMLSchema#> .
        @prefix rdf:   <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
        @prefix purrdf: <https://blackcatinformatics.ca/purrdf/> .
    ";

    #[test]
    fn test_project_graph_envelope_and_context() {
        let store = load(&format!(
            r#"{PREFIXES_TTL}
            purrdf:alice a purrdf:Person ;
                purrdf:name "Alice" .
        "#
        ));
        let doc = project_graph(&store);
        // @context carries the prefix map (purrdf:).
        assert_eq!(
            doc["@context"]["purrdf"],
            json!("https://blackcatinformatics.ca/purrdf/")
        );
        let graph = doc["@graph"].as_array().expect("@graph array");
        assert_eq!(graph.len(), 1);
        let node = &graph[0];
        assert_eq!(node["@id"], json!("purrdf:alice"));
        assert_eq!(node["@type"], json!("purrdf:Person"));
        assert_eq!(node["purrdf:name"], json!("Alice"));
    }

    #[test]
    fn test_object_property_is_node_ref() {
        let store = load(&format!(
            r"{PREFIXES_TTL}
            purrdf:org purrdf:member purrdf:alice .
        "
        ));
        let doc = project_graph(&store);
        let graph = doc["@graph"].as_array().unwrap();
        let org = graph
            .iter()
            .find(|n| n["@id"] == json!("purrdf:org"))
            .expect("org node");
        assert_eq!(org["purrdf:member"], json!({ "@id": "purrdf:alice" }));
    }

    #[test]
    fn test_typed_and_numeric_and_lang_literals() {
        let store = load(&format!(
            r#"{PREFIXES_TTL}
            purrdf:e purrdf:count 3 ;
                purrdf:flag true ;
                purrdf:at "2026-06-23T00:00:00Z"^^xsd:dateTime ;
                purrdf:label "bonjour"@fr .
        "#
        ));
        let doc = project_graph(&store);
        let node = &doc["@graph"].as_array().unwrap()[0];
        // integer → bare scalar
        assert_eq!(node["purrdf:count"], json!(3));
        // boolean → bare scalar
        assert_eq!(node["purrdf:flag"], json!(true));
        // dateTime → typed-literal object
        assert_eq!(
            node["purrdf:at"],
            json!({ "@value": "2026-06-23T00:00:00Z", "@type": "xsd:dateTime" })
        );
        // lang literal → {@value,@language}
        assert_eq!(
            node["purrdf:label"],
            json!({ "@value": "bonjour", "@language": "fr" })
        );
    }

    #[test]
    fn test_multi_valued_predicate_is_array_and_sorted() {
        let store = load(&format!(
            r#"{PREFIXES_TTL}
            purrdf:x purrdf:tag "b", "a", "c" .
        "#
        ));
        let doc = project_graph(&store);
        let node = &doc["@graph"].as_array().unwrap()[0];
        let tags = node["purrdf:tag"].as_array().expect("array");
        assert_eq!(tags.len(), 3);
        // sorted by term string → "a","b","c"
        assert_eq!(tags, &[json!("a"), json!("b"), json!("c")]);
    }

    #[test]
    fn test_named_graph_data_is_excluded() {
        // alice lives in the default graph; bob lives ONLY in a named graph.
        // A TriG document expresses both; the native codec preserves the named
        // graph, and the projector must scope to the default graph only.
        let trig = format!(
            r#"{PREFIXES_TTL}
            purrdf:alice a purrdf:Person ; purrdf:name "Alice" .
            purrdf:graph_other {{
                purrdf:bob purrdf:name "Bob" .
            }}
        "#
        );
        let store = std::sync::Arc::new(
            ::purrdf::parse_dataset(trig.as_bytes(), "application/trig", None).expect("TriG parse"),
        );
        let doc = project_graph(&store);
        let graph = doc["@graph"].as_array().expect("@graph array");
        // Only the default-graph subject is projected — no named-graph leak.
        assert_eq!(graph.len(), 1, "named-graph subject must not appear");
        assert_eq!(graph[0]["@id"], json!("purrdf:alice"));
        assert!(
            graph.iter().all(|n| n["@id"] != json!("purrdf:bob")),
            "named-graph subject leaked into @graph"
        );
    }

    #[test]
    fn test_determinism_byte_stable() {
        let ttl = format!(
            r#"{PREFIXES_TTL}
            purrdf:alice a purrdf:Person ; purrdf:name "Alice" ; purrdf:age 30 .
            purrdf:bob a purrdf:Person ; purrdf:name "Bob" .
        "#
        );
        let a = serde_json::to_string_pretty(&project_graph(&load(&ttl))).unwrap();
        let b = serde_json::to_string_pretty(&project_graph(&load(&ttl))).unwrap();
        assert_eq!(a, b, "projection must be byte-stable");
    }
}
