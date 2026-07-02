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
//!   [`Namespaces::compact_iri`]).
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
use crate::json_schema::Namespaces;
use crate::model::rdf;
use crate::term::Term;

const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema#";
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";

/// Project the default graph of `dataset` into a JSON-LD `@graph` document.
///
/// `ns` is the SAME caller-supplied [`Namespaces`] the matching schema was
/// compiled with ([`crate::json_schema::compile`]) — it drives every CURIE
/// compaction and the emitted `@context`:
///
/// ```text
/// let ns = Namespaces::new("gmeow", &doc_prefixes)?;
/// let doc = project_graph(&dataset, &ns);
/// ```
pub fn project_graph(dataset: &std::sync::Arc<RdfDataset>, ns: &Namespaces) -> Value {
    let data = IrDataGraph::new(std::sync::Arc::clone(dataset));
    project_graph_data(&data, ns)
}

/// Project the default graph of a [`ShaclDataGraph`] into a JSON-LD `@graph` document.
fn project_graph_data<G: ShaclDataGraph>(data: &G, ns: &Namespaces) -> Value {
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
        nodes.push(project_subject_data(data, ns, subj));
    }

    json!({
        "@context": Value::Object(ns.context_object()),
        "@graph": Value::Array(nodes),
    })
}

/// Project a single subject term into a JSON-LD node object, compacting IRIs
/// through the caller-supplied [`Namespaces`].
pub fn project_subject(
    dataset: &std::sync::Arc<RdfDataset>,
    ns: &Namespaces,
    subject: &Term,
) -> Value {
    let data = IrDataGraph::new(std::sync::Arc::clone(dataset));
    project_subject_data(&data, ns, subject)
}

fn project_subject_data<G: ShaclDataGraph>(data: &G, ns: &Namespaces, subject: &Term) -> Value {
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
                types.push(ns.compact_iri(n.as_str()));
            }
            continue;
        }
        by_pred
            .entry(ns.compact_iri(pred.as_str()))
            .or_default()
            .push(quad.object);
    }

    let mut obj: Map<String, Value> = Map::new();

    // @id
    let id = match subject {
        Term::NamedNode(n) => ns.compact_iri(n.as_str()),
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
        let values: Vec<Value> = objects.iter().map(|t| project_value(t, ns)).collect();
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
fn project_value(term: &Term, ns: &Namespaces) -> Value {
    match term {
        Term::NamedNode(n) => json!({ "@id": ns.compact_iri(n.as_str()) }),
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
            json!({ "@value": lit.value(), "@type": ns.compact_iri(dt_iri) })
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
        @prefix meta: <https://example.org/meta/> .
    ";

    /// The fixture namespace table — the same `meta` declaration the Turtle
    /// fixtures use, supplied by the caller (nothing hardcoded in library code).
    fn fixture_ns() -> Namespaces {
        Namespaces::new(
            "meta",
            &[("meta".to_owned(), "https://example.org/meta/".to_owned())],
        )
        .expect("fixture namespaces are valid")
    }

    #[test]
    fn test_project_graph_envelope_and_context() {
        let store = load(&format!(
            r#"{PREFIXES_TTL}
            meta:alice a meta:Person ;
                meta:name "Alice" .
        "#
        ));
        let doc = project_graph(&store, &fixture_ns());
        // @context carries the prefix map (meta:).
        assert_eq!(doc["@context"]["meta"], json!("https://example.org/meta/"));
        let graph = doc["@graph"].as_array().expect("@graph array");
        assert_eq!(graph.len(), 1);
        let node = &graph[0];
        assert_eq!(node["@id"], json!("meta:alice"));
        assert_eq!(node["@type"], json!("meta:Person"));
        assert_eq!(node["meta:name"], json!("Alice"));
    }

    #[test]
    fn test_object_property_is_node_ref() {
        let store = load(&format!(
            r"{PREFIXES_TTL}
            meta:org meta:member meta:alice .
        "
        ));
        let doc = project_graph(&store, &fixture_ns());
        let graph = doc["@graph"].as_array().unwrap();
        let org = graph
            .iter()
            .find(|n| n["@id"] == json!("meta:org"))
            .expect("org node");
        assert_eq!(org["meta:member"], json!({ "@id": "meta:alice" }));
    }

    #[test]
    fn test_typed_and_numeric_and_lang_literals() {
        let store = load(&format!(
            r#"{PREFIXES_TTL}
            meta:e meta:count 3 ;
                meta:flag true ;
                meta:at "2026-06-23T00:00:00Z"^^xsd:dateTime ;
                meta:label "bonjour"@fr .
        "#
        ));
        let doc = project_graph(&store, &fixture_ns());
        let node = &doc["@graph"].as_array().unwrap()[0];
        // integer → bare scalar
        assert_eq!(node["meta:count"], json!(3));
        // boolean → bare scalar
        assert_eq!(node["meta:flag"], json!(true));
        // dateTime → typed-literal object
        assert_eq!(
            node["meta:at"],
            json!({ "@value": "2026-06-23T00:00:00Z", "@type": "xsd:dateTime" })
        );
        // lang literal → {@value,@language}
        assert_eq!(
            node["meta:label"],
            json!({ "@value": "bonjour", "@language": "fr" })
        );
    }

    #[test]
    fn test_multi_valued_predicate_is_array_and_sorted() {
        let store = load(&format!(
            r#"{PREFIXES_TTL}
            meta:x meta:tag "b", "a", "c" .
        "#
        ));
        let doc = project_graph(&store, &fixture_ns());
        let node = &doc["@graph"].as_array().unwrap()[0];
        let tags = node["meta:tag"].as_array().expect("array");
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
            meta:alice a meta:Person ; meta:name "Alice" .
            meta:graph_other {{
                meta:bob meta:name "Bob" .
            }}
        "#
        );
        let store = std::sync::Arc::new(
            ::purrdf::parse_dataset(trig.as_bytes(), "application/trig", None).expect("TriG parse"),
        );
        let doc = project_graph(&store, &fixture_ns());
        let graph = doc["@graph"].as_array().expect("@graph array");
        // Only the default-graph subject is projected — no named-graph leak.
        assert_eq!(graph.len(), 1, "named-graph subject must not appear");
        assert_eq!(graph[0]["@id"], json!("meta:alice"));
        assert!(
            graph.iter().all(|n| n["@id"] != json!("meta:bob")),
            "named-graph subject leaked into @graph"
        );
    }

    #[test]
    fn test_determinism_byte_stable() {
        let ttl = format!(
            r#"{PREFIXES_TTL}
            meta:alice a meta:Person ; meta:name "Alice" ; meta:age 30 .
            meta:bob a meta:Person ; meta:name "Bob" .
        "#
        );
        let a = serde_json::to_string_pretty(&project_graph(&load(&ttl), &fixture_ns())).unwrap();
        let b = serde_json::to_string_pretty(&project_graph(&load(&ttl), &fixture_ns())).unwrap();
        assert_eq!(a, b, "projection must be byte-stable");
    }
}
