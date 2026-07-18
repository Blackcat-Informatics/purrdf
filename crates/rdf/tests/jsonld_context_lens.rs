// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Law tests for configured JSON-LD compaction and expansion.

mod common;

use std::fmt::Write as _;

use proptest::prelude::*;
use purrdf_rdf::native_codecs::jsonld::{
    CompiledJsonLdContext, JsonLdContextRegistry, JsonLdSerializeOptions, parse_jsonld,
    parse_jsonld_with_context, serialize_dataset_to_jsonld_with_options,
};
use purrdf_rdf::{
    RdfDatasetBuilder, RdfLiteral, canonical_flat_nquads, datasets_isomorphic, parse_dataset,
};
use serde_json::{Value, json};

fn parse_nquads(source: &str) -> std::sync::Arc<purrdf_rdf::RdfDataset> {
    parse_dataset(source.as_bytes(), "application/n-quads", None).expect("N-Quads fixture")
}

fn serialize_with_context(
    source: &str,
    context: &Value,
) -> (std::sync::Arc<purrdf_rdf::RdfDataset>, String) {
    let dataset = parse_nquads(source);
    let compiled = CompiledJsonLdContext::compile(context, None).expect("compile context");
    let json = serialize_dataset_to_jsonld_with_options(
        &dataset,
        &JsonLdSerializeOptions::compiled(std::sync::Arc::new(compiled)),
    )
    .expect("configured serialization");
    (dataset, json)
}

#[test]
fn aliases_base_vocab_and_coercions_compact_and_expand_losslessly() {
    let source = concat!(
        "<https://example.org/alice> <https://schema.org/name> \"Alice\"@en .\n",
        "<https://example.org/alice> <https://schema.org/knows> <https://example.org/bob> .\n",
        "<https://example.org/alice> <https://schema.org/age> \"42\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n",
        "<https://example.org/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://example.org/Person> .\n",
    );
    let context = json!({
        "@base": "https://example.org/",
        "ex": {"@id": "https://example.org/", "@prefix": true},
        "graph": "@graph",
        "id": "@id",
        "type": "@type",
        "schema": {"@id": "https://schema.org/", "@prefix": true},
        "xsd": {"@id": "http://www.w3.org/2001/XMLSchema#", "@prefix": true},
        "age": {"@id": "schema:age", "@type": "xsd:integer"},
        "knows": {"@id": "schema:knows", "@type": "@id"},
        "name": {"@id": "schema:name", "@language": "en"}
    });
    let (dataset, compacted) = serialize_with_context(source, &context);
    let value: Value = serde_json::from_str(&compacted).expect("JSON output");
    let node = &value["graph"][0];
    assert_eq!(node["id"], "ex:alice");
    assert_eq!(node["type"], "ex:Person");
    assert_eq!(node["name"], "Alice");
    assert_eq!(node["knows"], "ex:bob");
    assert_eq!(node["age"], "42");
    assert!(node.get("https://schema.org/name").is_none());

    let reparsed = parse_jsonld(compacted.as_bytes()).expect("expand compact JSON-LD");
    assert!(
        datasets_isomorphic(&dataset, &reparsed),
        "source:\n{}\nreparsed:\n{}",
        canonical_flat_nquads(&dataset).expect("canonical source"),
        canonical_flat_nquads(&reparsed).expect("canonical reparsed")
    );
}

#[test]
fn heterogeneous_values_partition_across_compatible_aliases() {
    let source = concat!(
        "<https://example.org/s> <https://example.org/p> <https://example.org/o> .\n",
        "<https://example.org/s> <https://example.org/p> \"hello\"@en .\n",
    );
    let context = json!({
        "ex": {"@id": "https://example.org/", "@prefix": true},
        "ref": {"@id": "ex:p", "@type": "@id"},
        "text": {"@id": "ex:p", "@language": "en"}
    });
    let (dataset, compacted) = serialize_with_context(source, &context);
    let value: Value = serde_json::from_str(&compacted).expect("JSON output");
    let node = &value["@graph"][0];
    assert_eq!(node["ref"], "ex:o");
    assert_eq!(node["text"], "hello");

    let reparsed = parse_jsonld(compacted.as_bytes()).expect("expand partitioned aliases");
    assert!(
        datasets_isomorphic(&dataset, &reparsed),
        "source:\n{}\nreparsed:\n{}\njson:\n{compacted}",
        canonical_flat_nquads(&dataset).expect("canonical source"),
        canonical_flat_nquads(&reparsed).expect("canonical reparsed")
    );
}

#[test]
fn safe_rdf_lists_use_list_containers_and_reconstruct_isomorphically() {
    let source = concat!(
        "<https://example.org/s> <https://example.org/items> _:head .\n",
        "_:head <http://www.w3.org/1999/02/22-rdf-syntax-ns#first> \"one\" .\n",
        "_:head <http://www.w3.org/1999/02/22-rdf-syntax-ns#rest> _:tail .\n",
        "_:tail <http://www.w3.org/1999/02/22-rdf-syntax-ns#first> <https://example.org/two> .\n",
        "_:tail <http://www.w3.org/1999/02/22-rdf-syntax-ns#rest> <http://www.w3.org/1999/02/22-rdf-syntax-ns#nil> .\n",
    );
    let context = json!({
        "ex": {"@id": "https://example.org/", "@prefix": true},
        "items": {"@container": "@list", "@id": "ex:items"}
    });
    let (dataset, compacted) = serialize_with_context(source, &context);
    let value: Value = serde_json::from_str(&compacted).expect("JSON output");
    assert_eq!(value["@graph"][0]["items"][0], "one");
    assert_eq!(value["@graph"][0]["items"][1]["@id"], "ex:two");
    assert!(!compacted.contains("rdf-syntax-ns#first"));

    let reparsed = parse_jsonld(compacted.as_bytes()).expect("reconstruct RDF list");
    assert!(datasets_isomorphic(&dataset, &reparsed));
}

#[test]
fn duplicate_members_and_unmapped_index_metadata_fail_closed() {
    let duplicate = br#"{"@context":{},"@graph":[],"@graph":[]}"#;
    let error = parse_jsonld(duplicate).expect_err("duplicate JSON member");
    assert_eq!(error.code, "jsonld-json-input");

    let indexed = br#"{
      "@context": {"p": {"@id": "https://example.org/p", "@container": "@index"}},
      "@id": "https://example.org/s",
      "p": {"source-row": {"@id": "https://example.org/o"}}
    }"#;
    let error = parse_jsonld(indexed).expect_err("unmapped @index metadata");
    assert!(error.message.contains("no RDF dataset representation"));
}

#[test]
fn direct_set_objects_expand_and_lossy_value_shapes_fail_closed() {
    let direct_set = br#"{
      "@context": {"ex": {"@id": "https://example.org/", "@prefix": true}},
      "@id": "ex:s",
      "ex:p": {"@set": [{"@id": "ex:o1"}, {"@id": "ex:o2"}]}
    }"#;
    let expected = parse_nquads(concat!(
        "<https://example.org/s> <https://example.org/p> <https://example.org/o1> .\n",
        "<https://example.org/s> <https://example.org/p> <https://example.org/o2> .\n",
    ));
    let actual = parse_jsonld(direct_set).expect("expand direct @set object");
    assert!(
        datasets_isomorphic(&expected, &actual),
        "expected:\n{}\nactual:\n{}",
        canonical_flat_nquads(&expected).expect("canonical expected"),
        canonical_flat_nquads(&actual).expect("canonical actual")
    );

    for (description, input) in [
        (
            "value object property",
            br#"{"@id":"https://example.org/s","https://example.org/p":{"@value":"v","https://example.org/lost":"x"}}"#.as_slice(),
        ),
        (
            "list object property",
            br#"{"@id":"https://example.org/s","https://example.org/p":{"@list":[],"https://example.org/lost":"x"}}"#.as_slice(),
        ),
        (
            "set object property",
            br#"{"@id":"https://example.org/s","https://example.org/p":{"@set":[],"https://example.org/lost":"x"}}"#.as_slice(),
        ),
        (
            "null value annotation",
            br#"{"@id":"https://example.org/s","https://example.org/p":{"@value":null,"@annotation":{"@id":"https://example.org/r"}}}"#.as_slice(),
        ),
    ] {
        let error = parse_jsonld(input).expect_err(description);
        assert!(
            error.message.contains("unexpected member")
                || error.message.contains("cannot carry @annotation"),
            "{description}: {error}"
        );
    }
}

#[test]
fn rdf_json_collapses_only_when_its_lexical_form_is_canonical() {
    let context = json!({
        "ex": {"@id": "https://example.org/", "@prefix": true},
        "json": {"@id": "ex:json", "@type": "@json"},
        "rdf": {"@id": "http://www.w3.org/1999/02/22-rdf-syntax-ns#", "@prefix": true}
    });
    let canonical = concat!(
        "<https://example.org/s> <https://example.org/json> ",
        "\"{\\\"a\\\":1}\"^^<http://www.w3.org/1999/02/22-rdf-syntax-ns#JSON> .\n",
    );
    let (dataset, compacted) = serialize_with_context(canonical, &context);
    let value: Value = serde_json::from_str(&compacted).expect("JSON output");
    assert_eq!(value["@graph"][0]["json"], json!({"a": 1}));
    let reparsed = parse_jsonld(compacted.as_bytes()).expect("expand canonical rdf:JSON");
    assert!(
        datasets_isomorphic(&dataset, &reparsed),
        "source:\n{}\nreparsed:\n{}\njson:\n{compacted}",
        canonical_flat_nquads(&dataset).expect("canonical source"),
        canonical_flat_nquads(&reparsed).expect("canonical reparsed")
    );

    let noncanonical = concat!(
        "<https://example.org/s> <https://example.org/json> ",
        "\"{ \\\"a\\\": 1 }\"^^<http://www.w3.org/1999/02/22-rdf-syntax-ns#JSON> .\n",
    );
    let (dataset, compacted) = serialize_with_context(noncanonical, &context);
    let value: Value = serde_json::from_str(&compacted).expect("JSON output");
    assert_eq!(value["@graph"][0]["ex:json"]["@value"], "{ \"a\": 1 }");
    assert_eq!(value["@graph"][0]["ex:json"]["@type"], "rdf:JSON");
    let reparsed = parse_jsonld(compacted.as_bytes()).expect("expand lexical rdf:JSON");
    assert!(
        datasets_isomorphic(&dataset, &reparsed),
        "source:\n{}\nreparsed:\n{}\njson:\n{compacted}",
        canonical_flat_nquads(&dataset).expect("canonical source"),
        canonical_flat_nquads(&reparsed).expect("canonical reparsed")
    );
}

#[test]
fn language_and_id_maps_compact_and_expand_losslessly() {
    let source = concat!(
        "<https://example.org/s> <https://example.org/label> \"hello\"@en .\n",
        "<https://example.org/s> <https://example.org/label> \"bonjour\"@fr .\n",
        "<https://example.org/s> <https://example.org/member> <https://example.org/alice> .\n",
        "<https://example.org/s> <https://example.org/member> <https://example.org/bob> .\n",
    );
    let context = json!({
        "ex": {"@id": "https://example.org/", "@prefix": true},
        "label": {"@id": "ex:label", "@container": "@language"},
        "member": {"@id": "ex:member", "@container": "@id"}
    });
    let (dataset, compacted) = serialize_with_context(source, &context);
    let value: Value = serde_json::from_str(&compacted).expect("JSON output");
    assert_eq!(value["@graph"][0]["label"]["en"], "hello");
    assert_eq!(value["@graph"][0]["label"]["fr"], "bonjour");
    assert_eq!(value["@graph"][0]["member"]["ex:alice"], json!({}));
    assert_eq!(value["@graph"][0]["member"]["ex:bob"], json!({}));
    let reparsed = parse_jsonld(compacted.as_bytes()).expect("expand map containers");
    assert!(datasets_isomorphic(&dataset, &reparsed));
}

#[test]
fn authored_reverse_nest_and_scoped_contexts_expand_through_one_lens() {
    let source = br#"{
      "@context": {
        "ex": {"@id": "https://example.org/", "@prefix": true},
        "details": "@nest",
        "friend": {
          "@id": "ex:friend",
          "@context": {"label": "ex:label"}
        },
        "knownBy": {"@reverse": "ex:knows", "@type": "@id"},
        "name": {"@id": "ex:name", "@nest": "details"}
      },
      "@id": "ex:alice",
      "knownBy": "ex:bob",
      "details": {"name": "Alice"},
      "friend": {"@id": "ex:carol", "label": "Carol"}
    }"#;
    let expected = parse_nquads(concat!(
        "<https://example.org/bob> <https://example.org/knows> <https://example.org/alice> .\n",
        "<https://example.org/alice> <https://example.org/name> \"Alice\" .\n",
        "<https://example.org/alice> <https://example.org/friend> <https://example.org/carol> .\n",
        "<https://example.org/carol> <https://example.org/label> \"Carol\" .\n",
    ));
    let actual = parse_jsonld(source).expect("expand reverse/nest/scoped context");
    assert!(datasets_isomorphic(&expected, &actual));
}

#[test]
fn mapped_index_type_and_set_containers_preserve_rdf() {
    let source = br#"{
      "@context": {
        "ex": {"@id": "https://example.org/", "@prefix": true},
        "entries": {"@id": "ex:entry", "@container": "@index", "@index": "ex:source"},
        "members": {"@id": "ex:member", "@container": "@type"},
        "tags": {"@id": "ex:tag", "@container": "@set"}
      },
      "@id": "ex:s",
      "entries": {"row-1": {"@id": "ex:o"}},
      "members": {"ex:Person": {"@id": "ex:bob"}},
      "tags": ["one", "two"]
    }"#;
    let expected = parse_nquads(concat!(
        "<https://example.org/s> <https://example.org/entry> <https://example.org/o> .\n",
        "<https://example.org/o> <https://example.org/source> \"row-1\" .\n",
        "<https://example.org/s> <https://example.org/member> <https://example.org/bob> .\n",
        "<https://example.org/bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://example.org/Person> .\n",
        "<https://example.org/s> <https://example.org/tag> \"one\" .\n",
        "<https://example.org/s> <https://example.org/tag> \"two\" .\n",
    ));
    let actual = parse_jsonld(source).expect("expand index/type/set containers");
    assert!(datasets_isomorphic(&expected, &actual));
}

#[test]
fn graph_id_and_mapped_graph_index_containers_preserve_named_graph_scope() {
    let source = br#"{
      "@context": {
        "ex": {"@id": "https://example.org/", "@prefix": true},
        "graphs": {"@id": "ex:containsGraph", "@container": ["@graph", "@id"]},
        "indexedGraphs": {
          "@id": "ex:indexedGraph",
          "@container": ["@graph", "@index"],
          "@index": "ex:key"
        },
        "name": "ex:name"
      },
      "@id": "ex:catalog",
      "graphs": {
        "ex:g": {"@id": "ex:item", "name": "Item"}
      },
      "indexedGraphs": {
        "row-1": {"@id": "ex:item2", "name": "Two"}
      }
    }"#;
    let expected = parse_nquads(concat!(
        "<https://example.org/catalog> <https://example.org/containsGraph> <https://example.org/g> .\n",
        "<https://example.org/item> <https://example.org/name> \"Item\" <https://example.org/g> .\n",
        "<https://example.org/catalog> <https://example.org/indexedGraph> _:graph .\n",
        "_:graph <https://example.org/key> \"row-1\" .\n",
        "<https://example.org/item2> <https://example.org/name> \"Two\" _:graph .\n",
    ));
    let actual = parse_jsonld(source).expect("expand @graph map containers");
    assert!(
        datasets_isomorphic(&expected, &actual),
        "expected:\n{}\nactual:\n{}",
        canonical_flat_nquads(&expected).expect("canonical expected"),
        canonical_flat_nquads(&actual).expect("canonical actual")
    );
}

#[test]
fn named_graph_references_compact_into_graph_id_maps() {
    let source = concat!(
        "<https://example.org/catalog> <https://example.org/containsGraph> <https://example.org/g> .\n",
        "<https://example.org/item> <https://example.org/name> \"Item\" <https://example.org/g> .\n",
    );
    let context = json!({
        "ex": {"@id": "https://example.org/", "@prefix": true},
        "graphs": {"@id": "ex:containsGraph", "@container": ["@graph", "@id"]},
        "name": "ex:name"
    });
    let (dataset, compacted) = serialize_with_context(source, &context);
    let value: Value = serde_json::from_str(&compacted).expect("JSON output");
    let catalog = value["@graph"]
        .as_array()
        .expect("top-level graph")
        .iter()
        .find(|node| node["@id"] == "ex:catalog")
        .expect("catalog node");
    assert_eq!(catalog["graphs"]["ex:g"]["@id"], "ex:item");
    assert_eq!(catalog["graphs"]["ex:g"]["name"], "Item");
    assert_eq!(value["@graph"].as_array().expect("graph").len(), 1);

    let reparsed = parse_jsonld(compacted.as_bytes()).expect("expand graph id map");
    assert!(datasets_isomorphic(&dataset, &reparsed));
}

#[test]
fn configured_context_preserves_named_graphs_and_rdf_1_2_statement_structures() {
    let dataset = common::build_fixture();
    let compiled = CompiledJsonLdContext::compile(
        &json!({
            "ex": {"@id": "http://example.org/", "@prefix": true},
            "rdf": {"@id": "http://www.w3.org/1999/02/22-rdf-syntax-ns#", "@prefix": true},
            "xsd": {"@id": "http://www.w3.org/2001/XMLSchema#", "@prefix": true}
        }),
        None,
    )
    .expect("compile context");
    let compacted = serialize_dataset_to_jsonld_with_options(
        &dataset,
        &JsonLdSerializeOptions::compiled(std::sync::Arc::new(compiled)),
    )
    .expect("serialize rich RDF 1.2 fixture");
    assert!(compacted.contains("@triple"));
    assert!(compacted.contains("@annotation"));
    let reparsed = parse_jsonld(compacted.as_bytes()).expect("expand rich RDF 1.2 fixture");
    assert!(
        datasets_isomorphic(&dataset, &reparsed),
        "source:\n{}\nreparsed:\n{}\njson:\n{compacted}",
        canonical_flat_nquads(&dataset).expect("canonical source"),
        canonical_flat_nquads(&reparsed).expect("canonical reparsed")
    );
}

#[test]
fn configured_context_preserves_many_to_many_reifier_bindings() {
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_iri("https://example.org/s");
    let predicate = builder.intern_iri("https://example.org/p");
    let first_object = builder.intern_iri("https://example.org/o1");
    let second_object = builder.intern_iri("https://example.org/o2");
    let first = builder.intern_triple(subject, predicate, first_object);
    let second = builder.intern_triple(subject, predicate, second_object);
    let shared = builder.intern_iri("https://example.org/shared");
    let alternate = builder.intern_iri("https://example.org/alternate");
    let confidence = builder.intern_iri("https://example.org/confidence");
    let high = builder.intern_literal(RdfLiteral::simple("high"));
    builder.push_quad(subject, predicate, first_object, None);
    builder.push_quad(subject, predicate, second_object, None);
    builder.push_reifier(shared, first);
    builder.push_reifier(shared, second);
    builder.push_reifier(alternate, first);
    builder.push_annotation(shared, confidence, high);
    let dataset = builder.freeze().expect("freeze many-to-many reifiers");

    let compiled = CompiledJsonLdContext::compile(
        &json!({"ex": {"@id": "https://example.org/", "@prefix": true}}),
        None,
    )
    .expect("compile context");
    let compacted = serialize_dataset_to_jsonld_with_options(
        &dataset,
        &JsonLdSerializeOptions::compiled(std::sync::Arc::new(compiled)),
    )
    .expect("serialize many-to-many reifiers");
    let reparsed = parse_jsonld(compacted.as_bytes()).expect("expand many-to-many reifiers");
    assert!(datasets_isomorphic(&dataset, &reparsed));
}

#[test]
fn mixed_graph_nodes_numeric_lexicals_and_annotation_types_survive_expansion() {
    let mixed = br#"{
      "@context": {
        "ex": {"@id": "https://example.org/", "@prefix": true},
        "measure": {"@id": "https://example.org/measure#", "@prefix": true}
      },
      "@graph": [{
        "@id": "ex:g",
        "@type": "ex:Graph",
        "ex:source": {"@id": "ex:reference"},
        "@graph": {"@id": "ex:s", "measure:cups": 5.3}
      }]
    }"#;
    let expected = parse_nquads(concat!(
        "<https://example.org/g> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://example.org/Graph> .\n",
        "<https://example.org/g> <https://example.org/source> <https://example.org/reference> .\n",
        "<https://example.org/s> <https://example.org/measure#cups> \"5.3E0\"^^<http://www.w3.org/2001/XMLSchema#double> <https://example.org/g> .\n",
    ));
    let actual = parse_jsonld(mixed).expect("expand mixed graph node");
    assert!(datasets_isomorphic(&expected, &actual));

    let annotated = br#"{
      "@id": "https://example.org/s",
      "https://example.org/p": {
        "@id": "https://example.org/o",
        "@annotation": {
          "@id": "_:r",
          "@type": "https://example.org/Annotation"
        }
      }
    }"#;
    let expected = parse_nquads(concat!(
        "<https://example.org/s> <https://example.org/p> <https://example.org/o> .\n",
        "_:r <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> <<( <https://example.org/s> <https://example.org/p> <https://example.org/o> )>> .\n",
        "_:r <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://example.org/Annotation> .\n",
    ));
    let actual = parse_jsonld(annotated).expect("expand typed annotation node");
    assert!(datasets_isomorphic(&expected, &actual));
}

#[test]
fn reverse_aliases_and_graph_index_maps_compact_losslessly() {
    let reverse_source =
        "<https://example.org/bob> <https://example.org/knows> <https://example.org/alice> .\n";
    let reverse_context = json!({
        "ex": {"@id": "https://example.org/", "@prefix": true},
        "knownBy": {"@reverse": "ex:knows", "@type": "@id"}
    });
    let (dataset, compacted) = serialize_with_context(reverse_source, &reverse_context);
    let value: Value = serde_json::from_str(&compacted).expect("JSON output");
    let alice = value["@graph"]
        .as_array()
        .expect("top-level graph")
        .iter()
        .find(|node| node["@id"] == "ex:alice")
        .expect("reverse target node");
    assert_eq!(alice["knownBy"], "ex:bob");
    let reparsed = parse_jsonld(compacted.as_bytes()).expect("expand reverse alias");
    assert!(datasets_isomorphic(&dataset, &reparsed));

    let indexed_source = concat!(
        "<https://example.org/catalog> <https://example.org/indexedGraph> _:graph .\n",
        "_:graph <https://example.org/key> \"row-1\" .\n",
        "<https://example.org/item> <https://example.org/name> \"Item\" _:graph .\n",
    );
    let indexed_context = json!({
        "ex": {"@id": "https://example.org/", "@prefix": true},
        "indexedGraphs": {
          "@id": "ex:indexedGraph",
          "@container": ["@graph", "@index"],
          "@index": "ex:key"
        },
        "name": "ex:name"
    });
    let (dataset, compacted) = serialize_with_context(indexed_source, &indexed_context);
    let value: Value = serde_json::from_str(&compacted).expect("JSON output");
    assert_eq!(value["@graph"][0]["indexedGraphs"]["row-1"]["name"], "Item");
    assert_eq!(value["@graph"].as_array().expect("graph").len(), 1);
    let reparsed = parse_jsonld(compacted.as_bytes()).expect("expand graph index map");
    assert!(datasets_isomorphic(&dataset, &reparsed));
}

#[test]
fn graph_index_containers_apply_inside_named_graphs() {
    let source = concat!(
        "<https://example.org/catalog> <https://example.org/indexedGraph> _:target _:outer .\n",
        "_:target <https://example.org/key> \"row-1\" _:outer .\n",
        "<https://example.org/item> <https://example.org/name> \"Item\" _:target .\n",
    );
    let context = json!({
        "ex": {"@id": "https://example.org/", "@prefix": true},
        "indexedGraphs": {
          "@id": "ex:indexedGraph",
          "@container": ["@graph", "@index"],
          "@index": "ex:key"
        },
        "name": "ex:name"
    });
    let (dataset, compacted) = serialize_with_context(source, &context);
    let value: Value = serde_json::from_str(&compacted).expect("JSON output");
    let outer = value["@graph"]
        .as_array()
        .expect("top-level graph")
        .iter()
        .find(|node| node["@id"] == "_:outer")
        .expect("outer named graph");
    let catalog = outer["@graph"]
        .as_array()
        .expect("outer graph contents")
        .iter()
        .find(|node| node["@id"] == "ex:catalog")
        .expect("catalog node");
    assert_eq!(catalog["indexedGraphs"]["row-1"]["name"], "Item");

    let reparsed = parse_jsonld(compacted.as_bytes()).expect("expand nested graph index map");
    assert!(datasets_isomorphic(&dataset, &reparsed));
}

#[test]
fn list_coercion_and_shared_list_identity_are_preserved() {
    let language_list = concat!(
        "<https://example.org/s> <https://example.org/items> _:head .\n",
        "_:head <http://www.w3.org/1999/02/22-rdf-syntax-ns#first> \"one\"@en .\n",
        "_:head <http://www.w3.org/1999/02/22-rdf-syntax-ns#rest> _:tail .\n",
        "_:tail <http://www.w3.org/1999/02/22-rdf-syntax-ns#first> \"two\"@en .\n",
        "_:tail <http://www.w3.org/1999/02/22-rdf-syntax-ns#rest> <http://www.w3.org/1999/02/22-rdf-syntax-ns#nil> .\n",
    );
    let context = json!({
        "ex": {"@id": "https://example.org/", "@prefix": true},
        "items": {"@id": "ex:items", "@container": "@list", "@language": "en"}
    });
    let (dataset, compacted) = serialize_with_context(language_list, &context);
    let value: Value = serde_json::from_str(&compacted).expect("JSON output");
    assert_eq!(value["@graph"][0]["items"], json!(["one", "two"]));
    let reparsed = parse_jsonld(compacted.as_bytes()).expect("expand coerced list");
    assert!(datasets_isomorphic(&dataset, &reparsed));

    let shared = concat!(
        "<https://example.org/s> <https://example.org/items> _:head .\n",
        "<https://example.org/x> <https://example.org/quotes> <<( <https://example.org/a> <https://example.org/p> _:head )>> .\n",
        "_:head <http://www.w3.org/1999/02/22-rdf-syntax-ns#first> \"one\" .\n",
        "_:head <http://www.w3.org/1999/02/22-rdf-syntax-ns#rest> <http://www.w3.org/1999/02/22-rdf-syntax-ns#nil> .\n",
    );
    let (dataset, compacted) = serialize_with_context(shared, &context);
    assert!(compacted.contains("rdf-syntax-ns#first"));
    let reparsed = parse_jsonld(compacted.as_bytes()).expect("expand shared list identity");
    assert!(datasets_isomorphic(&dataset, &reparsed));
}

#[test]
fn graph_names_are_not_folded_as_list_heads() {
    let source = concat!(
        "<https://example.org/s> <https://example.org/items> _:head _:head .\n",
        "_:head <http://www.w3.org/1999/02/22-rdf-syntax-ns#first> \"one\" _:head .\n",
        "_:head <http://www.w3.org/1999/02/22-rdf-syntax-ns#rest> <http://www.w3.org/1999/02/22-rdf-syntax-ns#nil> _:head .\n",
    );
    let context = json!({
        "ex": {"@id": "https://example.org/", "@prefix": true},
        "items": {"@id": "ex:items", "@container": "@list"}
    });
    let (dataset, compacted) = serialize_with_context(source, &context);
    assert!(compacted.contains("rdf-syntax-ns#first"));
    let reparsed = parse_jsonld(compacted.as_bytes()).expect("expand graph-name list identity");
    assert!(datasets_isomorphic(&dataset, &reparsed));
}

#[test]
fn generated_list_cells_do_not_collide_with_nested_annotation_ids() {
    let source = br#"{
      "@id": "https://example.org/s",
      "https://example.org/p": {
        "@list": [{"@id": "https://example.org/o"}],
        "@annotation": {
          "@id": "_:jsonld_list_0",
          "https://example.org/confidence": "high"
        }
      }
    }"#;
    let expected = parse_nquads(concat!(
        "<https://example.org/s> <https://example.org/p> _:head .\n",
        "_:head <http://www.w3.org/1999/02/22-rdf-syntax-ns#first> <https://example.org/o> .\n",
        "_:head <http://www.w3.org/1999/02/22-rdf-syntax-ns#rest> <http://www.w3.org/1999/02/22-rdf-syntax-ns#nil> .\n",
        "_:jsonld_list_0 <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> <<( <https://example.org/s> <https://example.org/p> _:head )>> .\n",
        "_:jsonld_list_0 <https://example.org/confidence> \"high\" .\n",
    ));
    let actual = parse_jsonld(source).expect("parse annotated list");
    assert!(datasets_isomorphic(&expected, &actual));
}

#[test]
fn registry_backed_compaction_has_a_matching_parse_lens() {
    let context_iri = "https://example.org/context.jsonld";
    let context_document = br#"{
      "@context": {
        "ex": {"@id": "https://example.org/", "@prefix": true},
        "p": {"@id": "ex:p", "@type": "@id"}
      }
    }"#;
    let registry = JsonLdContextRegistry::new([(context_iri, context_document.as_slice())])
        .expect("offline registry");
    let compiled = CompiledJsonLdContext::compile_registry_context(context_iri, &registry)
        .expect("registry context");
    let dataset =
        parse_nquads("<https://example.org/s> <https://example.org/p> <https://example.org/o> .\n");
    let compacted = serialize_dataset_to_jsonld_with_options(
        &dataset,
        &JsonLdSerializeOptions::compiled(std::sync::Arc::new(compiled.clone())),
    )
    .expect("registry-backed compaction");
    let reparsed = parse_jsonld_with_context(compacted.as_bytes(), &compiled)
        .expect("registry-backed expansion");
    assert!(datasets_isomorphic(&dataset, &reparsed));
}

#[test]
fn compaction_bytes_ignore_input_insertion_order_and_are_idempotent() {
    let forward = concat!(
        "<https://example.org/a> <https://example.org/p> <https://example.org/b> .\n",
        "<https://example.org/a> <https://example.org/q> \"two\" .\n",
        "<https://example.org/a> <https://example.org/q> \"one\" .\n",
    );
    let reverse = concat!(
        "<https://example.org/a> <https://example.org/q> \"one\" .\n",
        "<https://example.org/a> <https://example.org/q> \"two\" .\n",
        "<https://example.org/a> <https://example.org/p> <https://example.org/b> .\n",
    );
    let context = json!({
        "ex": {"@id": "https://example.org/", "@prefix": true},
        "p": {"@id": "ex:p", "@type": "@id"},
        "q": "ex:q"
    });
    let (_, first) = serialize_with_context(forward, &context);
    let (_, second) = serialize_with_context(reverse, &context);
    assert_eq!(first, second);

    let reparsed = parse_jsonld(first.as_bytes()).expect("expand compacted document");
    let compiled = CompiledJsonLdContext::compile(&context, None).expect("compile context");
    let reserialized = serialize_dataset_to_jsonld_with_options(
        &reparsed,
        &JsonLdSerializeOptions::compiled(std::sync::Arc::new(compiled)),
    )
    .expect("compact expanded dataset again");
    assert_eq!(first, reserialized);
}

proptest! {
    #[test]
    fn compact_expand_is_an_isomorphic_idempotent_lens(
        rows in prop::collection::btree_set(("[a-z]{1,8}", "[a-z]{1,8}"), 1..24)
    ) {
        let source = rows.iter().fold(String::new(), |mut source, (predicate, object)| {
            writeln!(
                source,
                "<https://example.org/s> <https://example.org/{predicate}> <https://example.org/{object}> ."
            )
            .expect("writing to String cannot fail");
            source
        });
        let context = json!({
            "ex": {"@id": "https://example.org/", "@prefix": true},
            "id": "@id"
        });
        let (dataset, compacted) = serialize_with_context(&source, &context);
        let reparsed = parse_jsonld(compacted.as_bytes()).expect("expand generated compact document");
        prop_assert!(datasets_isomorphic(&dataset, &reparsed));

        let compiled = CompiledJsonLdContext::compile(&context, None).expect("compile context");
        let reserialized = serialize_dataset_to_jsonld_with_options(
            &reparsed,
            &JsonLdSerializeOptions::compiled(std::sync::Arc::new(compiled)),
        )
        .expect("recompact generated dataset");
        prop_assert_eq!(compacted, reserialized);
    }
}
