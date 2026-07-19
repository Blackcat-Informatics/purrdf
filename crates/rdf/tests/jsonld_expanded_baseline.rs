// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Byte-compatibility baseline for every Rust route to expanded JSON-LD/YAML-LD.

use purrdf_rdf::native_codecs::jsonld::{serialize_dataset_to_jsonld, serialize_dataset_to_yamlld};
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
}
