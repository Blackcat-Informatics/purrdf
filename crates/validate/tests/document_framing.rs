// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Two-document framing roles, native policy forwarding and separate authorities.

use purrdf_lex::{
    allocation::StorageError,
    json::{self, Value, record::DecodeError},
    yaml,
};
use purrdf_validate::document::{self, Operation};

fn framed(source: &[u8], frame: &[u8], config: &Value) -> Value {
    json::read(&document::frame(source, frame, json::write_compact(config).as_bytes()).unwrap())
        .unwrap()
}

#[test]
fn framing_preserves_native_documents_context_roles_and_phase_limits() {
    // The unchanged native framing witness owns the algorithm's expected document.
    let root = purrdf_testkit::paths::workspace_root().join("vectors/yaml-ld/tests/cases/frame");
    let source = std::fs::read(root.join("0001-in.yamlld")).unwrap();
    let frame = std::fs::read(root.join("0001-frame.yamlld")).unwrap();
    let expected =
        yaml::read(&std::fs::read_to_string(root.join("0001-out.yamlld")).unwrap()).unwrap();
    let mut config =
        json::read(r#"{"version":1,"format":"yamlld","output_base":{"mode":"none"}}"#).unwrap();
    assert_eq!(framed(&source, &frame, &config), expected);
    config["format"] = Value::from("jsonld");
    let source = json::write_compact(&yaml::read(std::str::from_utf8(&source).unwrap()).unwrap());
    let frame = json::write_compact(&yaml::read(std::str::from_utf8(&frame).unwrap()).unwrap());
    assert_eq!(
        framed(source.as_bytes(), frame.as_bytes(), &config),
        expected
    );

    let ordinary = json::read(r#"{"version":1,"format":"jsonld"}"#).unwrap();
    let source = br#"{"@id":"urn:a","s":"x"}"#;
    let mut contexts = json::read(
        r#"{"version":1,"format":"jsonld","context":{"s":"urn:p"},"frame_context":{"p":"urn:p"}}"#,
    )
    .unwrap();
    assert_eq!(
        framed(source, br#"{"p":{}}"#, &contexts),
        json::read(r#"{"@context":{},"@id":"urn:a","urn:p":"x"}"#).unwrap()
    );
    assert_eq!(
        framed(source, br#"{"@context":{"q":"urn:p"},"p":{}}"#, &contexts),
        json::read(r#"{"@context":{"q":"urn:p"},"@id":"urn:a","q":"x"}"#).unwrap()
    );
    contexts["frame_context"] = Value::Null;
    assert_eq!(
        framed(source, b"{}", &contexts)["@context"],
        json::read("{}").unwrap()
    );
    assert_eq!(
        framed(source, br#"{"@context":null}"#, &contexts)["@context"],
        Value::Null
    );

    let source = br#"{"@id":"https://frame.example/dir/item","urn:p":"x"}"#;
    let mut origin = json::read(r#"{"version":1,"format":"jsonld","base":"https://source.example/dir/doc","frame_context":{"pick":"@id"},"output_base":{"mode":"none"}}"#).unwrap();
    assert!(
        document::frame(
            source,
            br#"{"pick":"item"}"#,
            json::write_compact(&origin).as_bytes()
        )
        .is_err()
    );
    origin
        .as_object_mut()
        .unwrap()
        .insert("frame_retrieval_base", Value::Null);
    assert!(
        document::frame(
            source,
            br#"{"pick":"item"}"#,
            json::write_compact(&origin).as_bytes()
        )
        .is_err()
    );
    origin["frame_retrieval_base"] = Value::from("https://frame.example/dir/frame");
    assert_eq!(
        framed(source, br#"{"pick":"item"}"#, &origin)["@id"],
        "https://frame.example/dir/item"
    );
    origin["frame_context"] =
        json::read(r#"{"pick":"@id","@base":"https://frame.example/dir/"}"#).unwrap();
    origin["frame_retrieval_base"] = Value::from("https://other.example/frame");
    assert_eq!(
        framed(source, br#"{"pick":"item"}"#, &origin)["@id"],
        "https://frame.example/dir/item"
    );
    origin["frame_context"] = json::read(r#"{"pick":"@id","@base":null}"#).unwrap();
    assert!(
        document::frame(
            source,
            br#"{"pick":"item"}"#,
            json::write_compact(&origin).as_bytes()
        )
        .is_err()
    );

    let offline = json::read(r#"{"version":1,"format":"jsonld","base":"https://source.example/dir/doc","frame_retrieval_base":"https://frame.example/dir/frame","frame_context":"ctx","registry":[{"iri":"https://source.example/dir/ctx","format":"json","document":"{\"@context\":{\"pick\":\"urn:wrong\"}}"},{"iri":"https://frame.example/dir/ctx","format":"yaml","document":"'@context': {pick: '@id', p: 'urn:p'}\n","yaml_ld":{}}]}"#).unwrap();
    let named = br#"{"@id":"urn:a","urn:p":"x"}"#;
    assert_eq!(
        framed(named, br#"{"pick":"urn:a"}"#, &offline)["@id"],
        "urn:a"
    );
    assert_eq!(
        framed(named, br#"{"@context":"ctx","pick":"urn:a"}"#, &offline)["p"],
        "x"
    );

    let based = br#"{"@context":{"@base":"https://example.org/moved/"},"@id":"item","urn:p":"x"}"#;
    let pattern = br#"{"@id":"https://example.org/moved/item"}"#;
    let mut base =
        json::read(r#"{"version":1,"format":"jsonld","base":"https://example.org/original/doc"}"#)
            .unwrap();
    assert_eq!(framed(based, pattern, &base)["@id"], "item");
    base.as_object_mut().unwrap().insert(
        "compaction",
        json::read(r#"{"compact_to_relative":false}"#).unwrap(),
    );
    assert_eq!(
        framed(based, pattern, &base)["@id"],
        "https://example.org/moved/item"
    );
    base["compaction"] = json::read("{}").unwrap();
    base.as_object_mut()
        .unwrap()
        .insert("output_base", json::read(r#"{"mode":"none"}"#).unwrap());
    assert_eq!(
        framed(based, pattern, &base)["@id"],
        "https://example.org/moved/item"
    );
    base["output_base"] =
        json::read(r#"{"mode":"iri","iri":"https://example.org/moved/"}"#).unwrap();
    assert_eq!(framed(based, pattern, &base)["@id"], "item");
    assert_eq!(
        framed(
            based,
            br#"{"@context":{"@base":null},"@id":"https://example.org/moved/item"}"#,
            &base
        )["@id"],
        "https://example.org/moved/item"
    );

    let linked = br#"[{"@id":"urn:a","urn:link":{"@id":"urn:b"},"urn:extra":"keep"},{"@id":"urn:b","urn:name":"B"}]"#;
    let policy = json::read(r#"{"version":1,"format":"jsonld","framing":{"embed":"@never","explicit":true,"omit_graph":false,"omit_default":true,"require_all":true,"frame_default":false}}"#).unwrap();
    assert_eq!(
        framed(linked, br#"{"@id":"urn:a","urn:link":{}}"#, &policy),
        json::read(r#"{"@context":{},"@graph":[{"@id":"urn:a","urn:link":{"@id":"urn:b"}}]}"#)
            .unwrap()
    );
    let mut defaults = ordinary.clone();
    defaults
        .as_object_mut()
        .unwrap()
        .insert("framing", json::read(r#"{"omit_default":true}"#).unwrap());
    assert!(
        framed(named, br#"{"@id":"urn:a","urn:missing":{}}"#, &defaults)
            .get("urn:missing")
            .is_none()
    );
    let graph = br#"[{"@id":"urn:g","@graph":[{"@id":"urn:a","urn:p":"named"}]},{"@id":"urn:a","urn:q":"default"}]"#;
    assert_eq!(
        framed(graph, br#"{"@id":"urn:a"}"#, &ordinary)["urn:p"],
        "named"
    );
    defaults["framing"] = json::read(r#"{"frame_default":true}"#).unwrap();
    assert!(
        framed(graph, br#"{"@id":"urn:a"}"#, &defaults)
            .get("urn:p")
            .is_none()
    );
    defaults.as_object_mut().unwrap().insert(
        "compaction",
        json::read(r#"{"compact_arrays":false}"#).unwrap(),
    );
    assert!(
        framed(named, b"{}", &defaults)["@graph"]
            .as_array()
            .is_some()
    );

    let native = br#"{"@id":"urn:s","urn:t":{"@triple":{"@subject":{"@id":"urn:a"},"@predicate":"urn:p","@object":{"@id":"urn:o"}}},"urn:p":{"@value":"x","@annotation":[{"urn:source":{"@id":"urn:doc"}}]},"urn:raw":{"@value":{"@id":"opaque","n":1.0},"@type":"@json"}}"#;
    let native_config = json::read(r#"{"version":1,"format":"jsonld"}"#).unwrap();
    let native = framed(
        native,
        br#"{"@context":{"raw":{"@id":"urn:raw","@type":"@json"}},"@id":"urn:s"}"#,
        &native_config,
    );
    assert_eq!(
        native["raw"],
        json::read(r#"{"@id":"opaque","n":1.0}"#).unwrap()
    );
    let native_text = json::write_compact(&native);
    assert!(native_text.contains("@triple") && native_text.contains("@annotation"));

    for (invalid, pointer) in [
        (
            r#"{"version":1,"format":"jsonld","frame_context":{"@context":{}}}"#,
            "/frame_context",
        ),
        (
            r#"{"version":1,"format":"jsonld","frame_context":[{},42]}"#,
            "/frame_context",
        ),
        (
            r#"{"version":1,"format":"jsonld","frame_retrieval_base":false}"#,
            "/frame_retrieval_base",
        ),
        (r#"{"version":1,"format":"jsonld","output_context":{}}"#, ""),
        (
            r#"{"version":1,"format":"jsonld","output_context_base":null}"#,
            "",
        ),
        (
            r#"{"version":1,"format":"jsonld","frame_format":"yamlld"}"#,
            "",
        ),
        (r#"{"version":1,"format":"jsonld","frame_yaml_ld":{}}"#, ""),
        (
            r#"{"version":1,"format":"jsonld","framing":{"embed":true}}"#,
            "/framing/embed",
        ),
        (
            r#"{"version":1,"format":"jsonld","framing":{"embed":"@last"}}"#,
            "/framing/embed",
        ),
        (
            r#"{"version":1,"format":"jsonld","framing":{"max_work":-1}}"#,
            "/framing/max_work",
        ),
        (
            r#"{"version":1,"format":"jsonld","node_map":null}"#,
            "/node_map",
        ),
    ] {
        let error = document::request(invalid.as_bytes(), Operation::Frame).unwrap_err();
        assert_eq!(
            std::error::Error::source(&error)
                .unwrap()
                .downcast_ref::<DecodeError>()
                .unwrap()
                .pointer(),
            pointer
        );
        assert_eq!(
            document::frame(&[0xff], &[0xff], invalid.as_bytes())
                .unwrap_err()
                .code,
            error.code
        );
    }
    assert!(
        document::request(
            br#"{"version":1,"format":"jsonld","frame_retrieval_base":"relative"}"#,
            Operation::Frame
        )
        .is_err()
    );
    let scoped = br#"{"version":1,"format":"jsonld","frame_context":[null,{"child":{"@id":"urn:child","@context":{"p":"urn:p"}}}]}"#;
    assert!(document::request(scoped, Operation::Frame).is_ok());

    for phase in ["expansion", "node_map", "framing", "compaction", "output"] {
        let mut limited = ordinary.clone();
        limited
            .as_object_mut()
            .unwrap()
            .insert(phase, json::read(r#"{"max_allocation_bytes":0}"#).unwrap());
        assert_eq!(
            document::frame(named, b"{}", json::write_compact(&limited).as_bytes())
                .unwrap_err()
                .storage_error(),
            Some(StorageError::AdmissionFailed)
        );
    }
    for (field, code) in [
        ("max_work", "jsonld-framing-work-limit"),
        ("max_emitted_nodes", "jsonld-framing-emission-limit"),
    ] {
        let mut limited = ordinary.clone();
        limited.as_object_mut().unwrap().insert(
            "framing",
            json::read(&format!(r#"{{"{field}":0}}"#)).unwrap(),
        );
        assert_eq!(
            document::frame(named, b"{}", json::write_compact(&limited).as_bytes())
                .unwrap_err()
                .code,
            code
        );
    }
    let text = document::frame(named, b"{}", json::write_compact(&ordinary).as_bytes()).unwrap();
    let mut output = ordinary;
    output.as_object_mut().unwrap().insert(
        "output",
        json::read(&format!(r#"{{"max_bytes":{}}}"#, text.len())).unwrap(),
    );
    assert_eq!(
        document::frame(named, b"{}", json::write_compact(&output).as_bytes()).unwrap(),
        text
    );
    output["output"] = json::read(r#"{"max_bytes":0}"#).unwrap();
    assert_eq!(
        document::frame(named, b"{}", json::write_compact(&output).as_bytes())
            .unwrap_err()
            .code,
        "document-framing-output"
    );
}
