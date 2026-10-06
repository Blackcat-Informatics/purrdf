// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Pinned W3C JSON-LD 1.1 to-RDF conformance vectors.

use std::collections::BTreeSet;
use std::sync::Arc;

use purrdf_lex::json::{self, Object, Value};
use purrdf_rdf::native_codecs::jsonld::{
    CompiledJsonLdContext, JsonLdSerializeOptions, parse_jsonld,
    serialize_dataset_to_jsonld_with_options,
};
use purrdf_rdf::{canonical_flat_nquads, datasets_isomorphic, parse_dataset};

#[path = "support/digest.rs"]
mod digest;
use digest::sha256;

const EXPECTED_VECTOR_COUNT: usize = 73;
const EXPECTED_COMPACTION_VECTOR_COUNT: usize = 13;
const EXPECTED_REVISION: &str = "3e7fa5377b2b3c5176eacf8bde8e01fdb7c4a062";
const EXPECTED_TAG: &str = "REC-2020-07-16";

/// The members of a fixture object, refused unless they are exactly `names`.
fn exact_members<'a>(value: &'a Value, names: &[&str], what: &str) -> &'a Object {
    let object = value
        .as_object()
        .unwrap_or_else(|| panic!("{what} is a JSON object"));
    for name in object.keys() {
        assert!(
            names.contains(&name.as_str()),
            "{what} has unknown member `{name}`"
        );
    }
    assert_eq!(object.len(), names.len(), "{what} has exactly {names:?}");
    object
}

fn text(object: &Object, name: &str) -> String {
    object[name]
        .as_str()
        .unwrap_or_else(|| panic!("`{name}` is a string"))
        .to_owned()
}

fn count(object: &Object, name: &str) -> usize {
    object[name]
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or_else(|| panic!("`{name}` is a count"))
}

/// A pinned corpus header, and its vectors decoded by `vector`.
struct Corpus<T> {
    schema_version: usize,
    upstream_revision: String,
    upstream_tag: String,
    expected_vector_count: usize,
    vectors: Vec<T>,
}

impl<T> Corpus<T> {
    fn read(text_json: &str, vector: impl Fn(&Value) -> T) -> Self {
        let document = json::read(text_json).expect("decode pinned W3C vectors");
        let object = exact_members(
            &document,
            &[
                "schema_version",
                "upstream_revision",
                "upstream_tag",
                "expected_vector_count",
                "vectors",
            ],
            "corpus",
        );
        Self {
            schema_version: count(object, "schema_version"),
            upstream_revision: text(object, "upstream_revision"),
            upstream_tag: text(object, "upstream_tag"),
            expected_vector_count: count(object, "expected_vector_count"),
            vectors: object["vectors"]
                .as_array()
                .expect("`vectors` is an array")
                .iter()
                .map(vector)
                .collect(),
        }
    }
}

struct Vector {
    id: String,
    name: String,
    purpose: String,
    input_sha256: String,
    expected_sha256: String,
    input: String,
    expected_nquads: String,
}

impl Vector {
    fn from_json(value: &Value) -> Self {
        let object = exact_members(
            value,
            &[
                "id",
                "name",
                "purpose",
                "input_sha256",
                "expected_sha256",
                "input",
                "expected_nquads",
            ],
            "vector",
        );
        Self {
            id: text(object, "id"),
            name: text(object, "name"),
            purpose: text(object, "purpose"),
            input_sha256: text(object, "input_sha256"),
            expected_sha256: text(object, "expected_sha256"),
            input: text(object, "input"),
            expected_nquads: text(object, "expected_nquads"),
        }
    }
}

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

impl CompactionVector {
    fn from_json(value: &Value) -> Self {
        let object = exact_members(
            value,
            &[
                "id",
                "name",
                "purpose",
                "input_sha256",
                "context_sha256",
                "expected_sha256",
                "input",
                "context",
                "expected",
            ],
            "compaction vector",
        );
        Self {
            id: text(object, "id"),
            name: text(object, "name"),
            purpose: text(object, "purpose"),
            input_sha256: text(object, "input_sha256"),
            context_sha256: text(object, "context_sha256"),
            expected_sha256: text(object, "expected_sha256"),
            input: text(object, "input"),
            context: text(object, "context"),
            expected: text(object, "expected"),
        }
    }
}

/// `text` read as JSON, every object's members in name order: JSON-LD gives member
/// order no meaning, so documents compare as name-ordered trees.
fn read_unordered(text: &str) -> Result<Value, json::Error> {
    let mut value = json::read(text)?;
    value.sort_keys();
    Ok(value)
}

fn flatten_single_carrier_graph(mut value: Value) -> Value {
    let Some(object) = value.as_object_mut() else {
        return value;
    };
    let Some(mut graph) = object.remove("@graph") else {
        return value;
    };
    let single = graph
        .as_array_mut()
        .filter(|items| items.len() == 1)
        .and_then(|items| items[0].as_object_mut().map(std::mem::take));
    match single {
        Some(node) => {
            for (name, member) in node {
                object.insert(name, member);
            }
            object.sort_keys();
        }
        None => {
            object.insert("@graph", graph);
            object.sort_keys();
        }
    }
    value
}

#[test]
fn pinned_w3c_to_rdf_vectors_match_the_independent_nquads_oracle() {
    let corpus = Corpus::read(
        include_str!("fixtures/jsonld-w3c-rec/vectors.json"),
        Vector::from_json,
    );
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

        // No base: each pinned vector is self-contained and carries its own `@context`
        // `@base` where the W3C fixture defines one. Injecting a base here would resolve
        // references the oracle N-Quads resolved differently, so the comparison would be
        // against a document the suite never specified.
        let actual = match parse_jsonld(vector.input.as_bytes(), None) {
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
    let corpus = Corpus::read(
        include_str!("fixtures/jsonld-w3c-rec/compaction_vectors.json"),
        CompactionVector::from_json,
    );
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

        // No base, for the same reason as the to-RDF pass above: the vector is the
        // specification, and a base it did not declare would change what it means.
        let dataset = match parse_jsonld(vector.input.as_bytes(), None) {
            Ok(dataset) => dataset,
            Err(error) => {
                failures.push(format!("{} ({}): {error}", vector.id, vector.name));
                continue;
            }
        };
        let context = read_unordered(&vector.context)
            .unwrap_or_else(|error| panic!("{} invalid context fixture: {error}", vector.id));
        let compiled = CompiledJsonLdContext::compile(&context, None)
            .unwrap_or_else(|error| panic!("{} context did not compile: {error}", vector.id));
        let output = serialize_dataset_to_jsonld_with_options(
            &dataset,
            &JsonLdSerializeOptions::compiled(Arc::new(compiled)),
        )
        .unwrap_or_else(|error| panic!("{} compaction failed: {error}", vector.id));
        let actual = read_unordered(&output).expect("PurRDF emitted JSON");
        let expected = read_unordered(&vector.expected)
            .unwrap_or_else(|error| panic!("{} invalid expected JSON: {error}", vector.id));
        let actual = flatten_single_carrier_graph(actual);
        if actual == expected {
            passed += 1;
        } else {
            failures.push(format!(
                "{} ({}):\nexpected:\n{}\nactual:\n{}",
                vector.id,
                vector.name,
                json::write_pretty(&expected),
                json::write_pretty(&actual)
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
