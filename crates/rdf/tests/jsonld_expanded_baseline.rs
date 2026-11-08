// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Byte-compatibility baseline for every Rust route to expanded JSON-LD/YAML-LD.

use purrdf_rdf::native_codecs::jsonld::{
    CompiledJsonLdContext, JsonLdSerializeOptions, serialize_dataset_to_jsonld,
    serialize_dataset_to_yamlld, serialize_dataset_to_yamlld_with_context,
};
use purrdf_rdf::{SerializeGraph, parse_dataset, serialize_dataset};

const NQUADS: &str = "<https://example.org/alice> <https://schema.org/name> \"Alice\" .\n";

const JSONLD: &str = r#"{
  "@context": {},
  "@graph": [
    {
      "@id": "https://example.org/alice",
      "https://schema.org/name": {
        "@value": "Alice"
      }
    }
  ]
}"#;

const YAMLLD: &str = concat!(
    "# yaml-language-server: $schema=purrdf.schema.json\n",
    "# The default reference is the bundled purrdf.schema.json; pass an explicit\n",
    "# schema_url to point editors at a hosted copy.\n",
    "'@context': {}\n",
    "'@graph':\n",
    "- '@id': https://example.org/alice\n",
    "  https://schema.org/name:\n",
    "    '@value': Alice\n",
);

#[test]
fn direct_and_generic_expanded_bytes_are_frozen() {
    let dataset = parse_dataset(NQUADS.as_bytes(), "application/n-quads", None)
        .expect("parse baseline N-Quads");

    assert_eq!(
        serialize_dataset_to_jsonld(&dataset).expect("direct JSON-LD"),
        JSONLD
    );
    assert_eq!(
        serialize_dataset(&dataset, "application/ld+json", SerializeGraph::Dataset)
            .expect("generic JSON-LD"),
        JSONLD.as_bytes()
    );
    assert_eq!(
        serialize_dataset_to_yamlld(&dataset, None).expect("direct YAML-LD"),
        YAMLLD
    );
    assert_eq!(
        serialize_dataset(&dataset, "application/ld+yaml", SerializeGraph::Dataset)
            .expect("generic YAML-LD"),
        YAMLLD.as_bytes()
    );

    let context =
        CompiledJsonLdContext::compile_json(br#"{"name":"https://schema.org/name"}"#, None)
            .expect("compile output context");
    for invalid in [
        "schema.json\n'@graph': []",
        "",
        "schema with spaces.json",
        "%zz",
    ] {
        for error in [
            serialize_dataset_to_yamlld(&dataset, Some(invalid)).expect_err("expanded refusal"),
            serialize_dataset_to_yamlld_with_context(&dataset, &context, Some(invalid))
                .expect_err("context refusal"),
            JsonLdSerializeOptions::expanded()
                .with_yaml_schema_url(invalid)
                .expect_err("configured refusal"),
        ] {
            assert_eq!(error.code, "jsonld-context-invalid");
        }
    }

    let schema = "../schemas/rdf.json?version=1#document";
    JsonLdSerializeOptions::expanded()
        .with_yaml_schema_url(schema)
        .expect("configured relative schema reference");
    let contextual_default = serialize_dataset_to_yamlld_with_context(&dataset, &context, None)
        .expect("default context YAML-LD");
    for (output, default) in [
        (
            serialize_dataset_to_yamlld(&dataset, Some(schema)).expect("relative schema reference"),
            YAMLLD,
        ),
        (
            serialize_dataset_to_yamlld_with_context(&dataset, &context, Some(schema))
                .expect("context relative schema reference"),
            contextual_default.as_str(),
        ),
    ] {
        let (header, body) = output.split_once('\n').expect("editor header");
        assert_eq!(header, format!("# yaml-language-server: $schema={schema}"));
        assert_eq!(body, default.split_once('\n').expect("default header").1);
        let reparsed = parse_dataset(output.as_bytes(), "application/ld+yaml", None)
            .expect("reparse schema-bearing YAML-LD");
        assert_eq!(
            serialize_dataset(&reparsed, "application/n-quads", SerializeGraph::Dataset)
                .expect("reparsed RDF"),
            NQUADS.as_bytes()
        );
    }
}
