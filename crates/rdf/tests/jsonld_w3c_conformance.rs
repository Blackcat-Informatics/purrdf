// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Pinned W3C JSON-LD 1.1 to-RDF conformance vectors.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::sync::Arc;

use purrdf_rdf::native_codecs::jsonld::{
    CompiledJsonLdContext, JsonLdSerializeOptions, parse_jsonld,
    serialize_dataset_to_jsonld_with_options,
};
use purrdf_rdf::{canonical_flat_nquads, datasets_isomorphic, parse_dataset};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

const EXPECTED_VECTOR_COUNT: usize = 73;
const EXPECTED_COMPACTION_VECTOR_COUNT: usize = 13;
const EXPECTED_REVISION: &str = "3e7fa5377b2b3c5176eacf8bde8e01fdb7c4a062";
const EXPECTED_TAG: &str = "REC-2020-07-16";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    schema_version: u32,
    upstream_revision: String,
    upstream_tag: String,
    expected_vector_count: usize,
    vectors: Vec<Vector>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Vector {
    id: String,
    name: String,
    purpose: String,
    input_sha256: String,
    expected_sha256: String,
    input: String,
    expected_nquads: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CompactionCorpus {
    schema_version: u32,
    upstream_revision: String,
    upstream_tag: String,
    expected_vector_count: usize,
    vectors: Vec<CompactionVector>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CompactionVector {
    id: String,
    name: String,
    purpose: String,
    input_sha256: String,
    context_sha256: String,
    expected_sha256: String,
    input: String,
    context: String,
    expected: String,
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut output, byte| {
            write!(output, "{byte:02x}").expect("writing to String cannot fail");
            output
        })
}

fn flatten_single_carrier_graph(mut value: Value) -> Value {
    let Some(object) = value.as_object_mut() else {
        return value;
    };
    let Some(Value::Array(mut graph)) = object.remove("@graph") else {
        return value;
    };
    if graph.len() != 1 {
        object.insert("@graph".to_owned(), Value::Array(graph));
        return value;
    }
    let Value::Object(node) = graph.remove(0) else {
        object.insert("@graph".to_owned(), Value::Array(graph));
        return value;
    };
    object.extend(node);
    value
}

#[test]
fn pinned_w3c_to_rdf_vectors_match_the_independent_nquads_oracle() {
    let corpus: Corpus = serde_json::from_str(include_str!("fixtures/jsonld-w3c-rec/vectors.json"))
        .expect("decode pinned W3C vectors");
    assert_eq!(corpus.schema_version, 1);
    assert_eq!(corpus.upstream_revision, EXPECTED_REVISION);
    assert_eq!(corpus.upstream_tag, EXPECTED_TAG);
    assert_eq!(corpus.expected_vector_count, EXPECTED_VECTOR_COUNT);
    assert_eq!(corpus.vectors.len(), EXPECTED_VECTOR_COUNT);

    let mut identifiers = BTreeSet::new();
    let mut passed = 0;
    let mut failures = Vec::new();
    for vector in &corpus.vectors {
        assert!(
            identifiers.insert(vector.id.as_str()),
            "duplicate vector id"
        );
        assert!(
            !vector.name.is_empty(),
            "{} has no upstream name",
            vector.id
        );
        assert!(
            !vector.purpose.is_empty(),
            "{} has no upstream purpose",
            vector.id
        );
        assert_eq!(
            sha256(vector.input.as_bytes()),
            vector.input_sha256,
            "{} input checksum drift",
            vector.id
        );
        assert_eq!(
            sha256(vector.expected_nquads.as_bytes()),
            vector.expected_sha256,
            "{} oracle checksum drift",
            vector.id
        );

        let actual = match parse_jsonld(vector.input.as_bytes()) {
            Ok(dataset) => dataset,
            Err(error) => {
                failures.push(format!("{} ({}): {error}", vector.id, vector.name));
                continue;
            }
        };
        let expected = parse_dataset(
            vector.expected_nquads.as_bytes(),
            "application/n-quads",
            None,
        )
        .unwrap_or_else(|error| panic!("{} invalid pinned N-Quads: {error}", vector.id));
        if datasets_isomorphic(&actual, &expected) {
            passed += 1;
        } else {
            failures.push(format!(
                "{} ({}):\nexpected:\n{}\nactual:\n{}",
                vector.id,
                vector.name,
                canonical_flat_nquads(&expected).expect("canonical expected dataset"),
                canonical_flat_nquads(&actual).expect("canonical actual dataset")
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "W3C JSON-LD vector failures:\n{}",
        failures.join("\n\n")
    );
    assert_eq!(passed, EXPECTED_VECTOR_COUNT, "exact W3C pass count");
}

#[test]
fn pinned_w3c_compaction_vectors_match_the_independent_json_oracle() {
    let corpus: CompactionCorpus = serde_json::from_str(include_str!(
        "fixtures/jsonld-w3c-rec/compaction_vectors.json"
    ))
    .expect("decode pinned W3C compaction vectors");
    assert_eq!(corpus.schema_version, 1);
    assert_eq!(corpus.upstream_revision, EXPECTED_REVISION);
    assert_eq!(corpus.upstream_tag, EXPECTED_TAG);
    assert_eq!(
        corpus.expected_vector_count,
        EXPECTED_COMPACTION_VECTOR_COUNT
    );
    assert_eq!(corpus.vectors.len(), EXPECTED_COMPACTION_VECTOR_COUNT);

    let mut identifiers = BTreeSet::new();
    let mut passed = 0;
    let mut failures = Vec::new();
    for vector in &corpus.vectors {
        assert!(
            identifiers.insert(vector.id.as_str()),
            "duplicate vector id"
        );
        assert!(
            !vector.name.is_empty(),
            "{} has no upstream name",
            vector.id
        );
        assert!(
            !vector.purpose.is_empty(),
            "{} has no upstream purpose",
            vector.id
        );
        for (description, bytes, expected) in [
            ("input", vector.input.as_bytes(), &vector.input_sha256),
            ("context", vector.context.as_bytes(), &vector.context_sha256),
            (
                "expected",
                vector.expected.as_bytes(),
                &vector.expected_sha256,
            ),
        ] {
            assert_eq!(
                sha256(bytes),
                *expected,
                "{} {description} checksum drift",
                vector.id
            );
        }

        let dataset = match parse_jsonld(vector.input.as_bytes()) {
            Ok(dataset) => dataset,
            Err(error) => {
                failures.push(format!("{} ({}): {error}", vector.id, vector.name));
                continue;
            }
        };
        let context: Value = serde_json::from_str(&vector.context)
            .unwrap_or_else(|error| panic!("{} invalid context fixture: {error}", vector.id));
        let compiled = CompiledJsonLdContext::compile(&context, None)
            .unwrap_or_else(|error| panic!("{} context did not compile: {error}", vector.id));
        let output = serialize_dataset_to_jsonld_with_options(
            &dataset,
            &JsonLdSerializeOptions::compiled(Arc::new(compiled)),
        )
        .unwrap_or_else(|error| panic!("{} compaction failed: {error}", vector.id));
        let actual: Value = serde_json::from_str(&output).expect("PurRDF emitted JSON");
        let expected: Value = serde_json::from_str(&vector.expected)
            .unwrap_or_else(|error| panic!("{} invalid expected JSON: {error}", vector.id));
        let actual = flatten_single_carrier_graph(actual);
        if actual == expected {
            passed += 1;
        } else {
            failures.push(format!(
                "{} ({}):\nexpected:\n{}\nactual:\n{}",
                vector.id,
                vector.name,
                serde_json::to_string_pretty(&expected).expect("expected JSON"),
                serde_json::to_string_pretty(&actual).expect("actual JSON")
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "W3C JSON-LD compaction vector failures:\n{}",
        failures.join("\n\n")
    );
    assert_eq!(
        passed, EXPECTED_COMPACTION_VECTOR_COUNT,
        "exact W3C compaction pass count"
    );
}
