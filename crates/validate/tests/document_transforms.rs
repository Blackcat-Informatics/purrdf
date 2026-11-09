// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Structural compaction/flattening, base roles and independent phase boundaries.

use purrdf_lex::{
    allocation::StorageError,
    json::{self, Value, record::DecodeError},
    yaml,
};
use purrdf_validate::document::{self, Operation};

fn transform(source: &[u8], config: &Value, operation: Operation) -> Value {
    json::read(
        &document::execute(source, json::write_compact(config).as_bytes(), operation).unwrap(),
    )
    .unwrap()
}

#[test]
fn compaction_and_context_aware_flattening_preserve_structure_and_independent_bases() {
    let minimal = br#"{"urn:p":"x"}"#;
    let mut config =
        json::read(r#"{"version":1,"format":"jsonld","output_context":null}"#).unwrap();
    assert_eq!(
        transform(minimal, &config, Operation::Compact),
        json::read(r#"{"urn:p":"x"}"#).unwrap()
    );
    config["output_context"] = json::read("{}").unwrap();
    assert_eq!(
        transform(minimal, &config, Operation::Compact),
        json::read(r#"{"@context":{},"urn:p":"x"}"#).unwrap()
    );
    let array_context =
        json::read(r#"{"version":1,"format":"jsonld","output_context":[null,{"p":"urn:p"}]}"#)
            .unwrap();
    assert_eq!(
        transform(minimal, &array_context, Operation::Compact),
        json::read(r#"{"@context":[null,{"p":"urn:p"}],"p":"x"}"#).unwrap()
    );
    config.as_object_mut().unwrap().insert(
        "compaction",
        json::read(r#"{"compact_arrays":false}"#).unwrap(),
    );
    assert_eq!(
        transform(minimal, &config, Operation::Compact),
        json::read(r#"{"@context":{},"@graph":[{"urn:p":["x"]}]}"#).unwrap()
    );

    let expanded = json::read(r#"[{"@id":"urn:s","urn:p":[{"@value":"x"}]}]"#).unwrap();
    let named = br#"{"@id":"urn:s","urn:p":"x"}"#;
    let missing =
        json::read(r#"{"version":1,"format":"jsonld","compaction":{"max_allocation_bytes":0}}"#)
            .unwrap();
    assert_eq!(transform(named, &missing, Operation::Flatten), expanded);
    let mut null = missing;
    null.as_object_mut()
        .unwrap()
        .insert("output_context", Value::Null);
    assert_eq!(transform(named, &null, Operation::Flatten), expanded);
    null["output_context"] = json::read("{}").unwrap();
    assert_eq!(
        document::execute(
            named,
            json::write_compact(&null).as_bytes(),
            Operation::Flatten
        )
        .unwrap_err()
        .storage_error(),
        Some(StorageError::AdmissionFailed)
    );
    null.as_object_mut().unwrap().remove("compaction");
    assert_eq!(
        transform(named, &null, Operation::Flatten),
        json::read(r#"{"@context":{},"@id":"urn:s","urn:p":"x"}"#).unwrap()
    );

    let source = json::read(r#"[{"@id":"urn:s","urn:p":{"@value":"one","@annotation":[{"urn:source":{"@id":"urn:doc"}}]},"urn:child":{"urn:p":"nested"},"urn:raw":{"@value":{"@id":"opaque","n":1.0},"@type":"@json"},"urn:t":{"@triple":{"@subject":{"@id":"urn:s"},"@predicate":"urn:p","@object":{"@id":"urn:o"}}}},{"@id":"urn:s","urn:p":"two"}]"#).unwrap();
    let source_json = json::write_compact(&source);
    let source_yaml = yaml::write(&source);
    let config = json::read(r#"{"version":1,"format":"jsonld","output_context":{"p":"urn:p","child":{"@id":"urn:child","@context":{"p":"urn:p"}},"raw":{"@id":"urn:raw","@type":"@json"}}}"#).unwrap();
    for operation in [Operation::Compact, Operation::Flatten] {
        let actual = transform(source_json.as_bytes(), &config, operation);
        let mut yaml_config = config.clone();
        yaml_config["format"] = Value::from("yamlld");
        assert_eq!(
            transform(source_yaml.as_bytes(), &yaml_config, operation),
            actual
        );
        let nodes = actual["@graph"].as_array().unwrap();
        assert_eq!(nodes.len(), 2);
        let subject = nodes.iter().find(|node| node["@id"] == "urn:s").unwrap();
        assert_eq!(
            subject["raw"],
            json::read(r#"{"@id":"opaque","n":1.0}"#).unwrap()
        );
        assert!(subject["urn:t"]["@triple"].as_object().is_some());
        if operation == Operation::Compact {
            assert_eq!(nodes[1]["@id"], "urn:s");
            assert!(subject["child"].get("@id").is_none());
            assert_eq!(
                subject["p"]["@annotation"],
                json::read(r#"{"urn:source":{"@id":"urn:doc"}}"#).unwrap()
            );
        } else {
            assert_eq!(subject["p"].as_array().unwrap().len(), 2);
            assert!(subject["child"]["@id"].as_str().unwrap().starts_with("_:"));
        }
    }

    let based = br#"{"@context":{"@base":"https://example.org/moved/"},"@id":"item","urn:p":"v"}"#;
    let base_config = json::read(r#"{"version":1,"format":"jsonld","base":"https://example.org/original/doc","output_context":{}}"#).unwrap();
    let cleared =
        br#"{"@context":{"@base":null},"@id":"https://example.org/moved/item","urn:p":"v"}"#;
    for operation in [Operation::Compact, Operation::Flatten] {
        assert_eq!(transform(based, &base_config, operation)["@id"], "item");
        assert_eq!(
            transform(cleared, &base_config, operation)["@id"],
            "https://example.org/moved/item"
        );
    }
    let mut controls = base_config.clone();
    controls
        .as_object_mut()
        .unwrap()
        .insert("output_base", json::read(r#"{"mode":"none"}"#).unwrap());
    assert_eq!(
        transform(based, &controls, Operation::Compact)["@id"],
        "https://example.org/moved/item"
    );
    controls["output_base"] =
        json::read(r#"{"mode":"iri","iri":"https://example.org/moved/"}"#).unwrap();
    assert_eq!(
        transform(based, &controls, Operation::Compact)["@id"],
        "item"
    );
    controls["output_context"] = json::read(r#"{"@base":null}"#).unwrap();
    assert_eq!(
        transform(based, &controls, Operation::Compact)["@id"],
        "https://example.org/moved/item"
    );
    controls["output_context"] = json::read(r#"{"@base":"https://example.org/moved/"}"#).unwrap();
    controls["output_base"] = json::read(r#"{"mode":"none"}"#).unwrap();
    assert_eq!(
        transform(based, &controls, Operation::Compact)["@id"],
        "item"
    );
    let mut absolute = base_config;
    absolute["output_context"] = json::read(r#"{"@vocab":"./vocab/"}"#).unwrap();
    absolute.as_object_mut().unwrap().insert(
        "compaction",
        json::read(r#"{"compact_to_relative":false}"#).unwrap(),
    );
    let vocabulary_source = br#"{"@context":{"@base":"https://example.org/moved/"},"@id":"item","https://example.org/moved/vocab/p":"v"}"#;
    let actual = transform(vocabulary_source, &absolute, Operation::Compact);
    assert_eq!(actual["@id"], "https://example.org/moved/item");
    assert_eq!(actual["p"], "v");

    let offline = json::read(r#"{"version":1,"format":"jsonld","base":"https://retrieval.example/input/doc","output_context":"context","registry":[{"iri":"https://retrieval.example/input/context","format":"json","document":"{\"@context\":{\"p\":\"urn:p\"}}"},{"iri":"https://retrieval.example/output/context","format":"yaml","document":"'@context': {q: 'urn:p'}\n","yaml_ld":{}}]}"#).unwrap();
    assert_eq!(transform(based, &offline, Operation::Compact)["p"], "v");
    let mut retrieval = offline;
    retrieval.as_object_mut().unwrap().insert(
        "output_context_base",
        Value::from("https://retrieval.example/output/"),
    );
    assert_eq!(transform(based, &retrieval, Operation::Compact)["q"], "v");
    retrieval["output_context_base"] = Value::Null;
    assert!(
        document::execute(
            based,
            json::write_compact(&retrieval).as_bytes(),
            Operation::Compact
        )
        .is_err()
    );
    retrieval["output_context"] = Value::from("https://retrieval.example/input/context");
    assert_eq!(transform(based, &retrieval, Operation::Compact)["p"], "v");

    for (operation, invalid, pointer) in [
        (Operation::Compact, r#"{"version":1,"format":"jsonld"}"#, ""),
        (
            Operation::Compact,
            r#"{"version":1,"format":"jsonld","output_context":{},"flattening":{}}"#,
            "",
        ),
        (
            Operation::Flatten,
            r#"{"version":1,"format":"jsonld","output_context":{"@context":{}}}"#,
            "/output_context",
        ),
        (
            Operation::Compact,
            r#"{"version":1,"format":"jsonld","output_context":false}"#,
            "/output_context",
        ),
        (
            Operation::Flatten,
            r#"{"version":1,"format":"jsonld","output_context":[{},42]}"#,
            "/output_context",
        ),
        (
            Operation::Flatten,
            r#"{"version":1,"format":"jsonld","output_base":null}"#,
            "/output_base",
        ),
        (
            Operation::Compact,
            r#"{"version":1,"format":"jsonld","output_context":{},"output_base":{"mode":"none","iri":"https://example.org/"}}"#,
            "/output_base",
        ),
        (
            Operation::Flatten,
            r#"{"version":1,"format":"jsonld","compaction":{"compact_arrays":0}}"#,
            "/compaction/compact_arrays",
        ),
        (
            Operation::Flatten,
            r#"{"version":1,"format":"jsonld","flattening":{"max_allocation_bytes":-1}}"#,
            "/flattening/max_allocation_bytes",
        ),
    ] {
        let error = document::request(invalid.as_bytes(), operation).unwrap_err();
        assert_eq!(
            std::error::Error::source(&error)
                .unwrap()
                .downcast_ref::<DecodeError>()
                .unwrap()
                .pointer(),
            pointer
        );
        assert_eq!(
            document::execute(&[0xff], invalid.as_bytes(), operation)
                .unwrap_err()
                .code,
            error.code
        );
    }
    for invalid in [
        r#"{"version":1,"format":"jsonld","output_context":null,"output_context_base":"relative"}"#,
        r#"{"version":1,"format":"jsonld","output_base":{"mode":"iri","iri":"relative"}}"#,
    ] {
        assert!(document::request(invalid.as_bytes(), Operation::Flatten).is_err());
    }
    assert_eq!(transform(named, &json::read(r#"{"version":1,"format":"jsonld","output_context_base":null,"output_base":{"mode":"iri","iri":"https://example.org/"},"compaction":{"max_allocation_bytes":0}}"#).unwrap(), Operation::Flatten), expanded);

    let ordinary = json::read(r#"{"version":1,"format":"jsonld","output_context":{}}"#).unwrap();
    let output = document::execute(
        minimal,
        json::write_compact(&ordinary).as_bytes(),
        Operation::Compact,
    )
    .unwrap();
    for maximum in [u64::try_from(output.len()).unwrap(), u64::MAX] {
        let mut bounded = ordinary.clone();
        bounded.as_object_mut().unwrap().insert(
            "output",
            json::read(&format!(r#"{{"max_bytes":{maximum}}}"#)).unwrap(),
        );
        assert_eq!(
            document::execute(
                minimal,
                json::write_compact(&bounded).as_bytes(),
                Operation::Compact
            )
            .unwrap(),
            output
        );
    }
    let mut refused = ordinary.clone();
    refused.as_object_mut().unwrap().insert(
        "output",
        json::read(&format!(r#"{{"max_bytes":{}}}"#, output.len() - 1)).unwrap(),
    );
    assert_eq!(
        document::execute(
            minimal,
            json::write_compact(&refused).as_bytes(),
            Operation::Compact
        )
        .unwrap_err()
        .code,
        "document-compaction-output"
    );
    for (operation, phase) in [
        (Operation::Compact, "expansion"),
        (Operation::Compact, "compaction"),
        (Operation::Compact, "output"),
        (Operation::Flatten, "flattening"),
    ] {
        let mut limited = ordinary.clone();
        limited
            .as_object_mut()
            .unwrap()
            .insert(phase, json::read(r#"{"max_allocation_bytes":0}"#).unwrap());
        assert_eq!(
            document::execute(minimal, json::write_compact(&limited).as_bytes(), operation)
                .unwrap_err()
                .storage_error(),
            Some(StorageError::AdmissionFailed)
        );
        assert!(
            document::execute(
                minimal,
                json::write_compact(&ordinary).as_bytes(),
                operation
            )
            .is_ok()
        );
    }

    // Reuse unchanged pinned local-context compaction and Flatten 0044 expectations.
    let root = purrdf_testkit::paths::workspace_root!().join("vectors/yaml-ld/tests");
    let manifest =
        json::read(&std::fs::read_to_string(root.join("manifest.jsonld")).unwrap()).unwrap();
    for id in [
        "#compact-local-json-ld-context",
        "#compact-local-yaml-ld-context",
        "#flatten",
    ] {
        let entry = manifest["sequence"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["@id"] == id)
            .unwrap();
        let context_path = entry["context"].as_str().unwrap();
        let context_bytes = std::fs::read(root.join(context_path)).unwrap();
        let context = if context_path.ends_with(".jsonld") {
            json::read_slice(&context_bytes, json::Limits::DEFAULT).unwrap()
        } else {
            yaml::read(std::str::from_utf8(&context_bytes).unwrap()).unwrap()
        };
        let mut config = json::read(r#"{"version":1,"format":"yamlld"}"#).unwrap();
        let fields = config.as_object_mut().unwrap();
        fields.insert(
            "base",
            Value::String(format!(
                "{}{}",
                manifest["baseIri"].as_str().unwrap(),
                entry["input"].as_str().unwrap()
            )),
        );
        fields.insert(
            "output_context",
            context.get("@context").unwrap_or(&context).clone(),
        );
        fields.insert(
            "compaction",
            json::read(&format!(
                r#"{{"compact_arrays":{}}}"#,
                entry["option"]
                    .get("compactArrays")
                    .and_then(Value::as_bool)
                    .unwrap_or(true)
            ))
            .unwrap(),
        );
        let bytes = std::fs::read(root.join(entry["input"].as_str().unwrap())).unwrap();
        let expected = yaml::read(
            &std::fs::read_to_string(root.join(entry["expect"].as_str().unwrap())).unwrap(),
        )
        .unwrap();
        assert_eq!(
            transform(
                &bytes,
                &config,
                if id == "#flatten" {
                    Operation::Flatten
                } else {
                    Operation::Compact
                }
            ),
            expected,
            "{id}"
        );
    }
}
