// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Shared structural document expansion and its closed request/phase boundaries.

use purrdf_lex::{
    allocation::StorageError,
    json::{self, Value},
    yaml,
};
use purrdf_validate::document_expansion::{expand, expand_decoded, request};

#[test]
fn version_one_expansion_preserves_documents_and_enforces_phase_limits() {
    let source = r#"[{"@id":"urn:s","p":{"@value":"one","@annotation":[{"urn:source":{"@id":"urn:doc"}}]},"q":{"p":"nested"},"j":{"@id":"opaque"},"urn:t":{"@triple":{"@subject":{"@id":"urn:s"},"@predicate":"urn:p","@object":{"@id":"urn:o"}}}},{"@id":"urn:s","p":"two"}]"#;
    let expected = json::read(r#"[{"@id":"urn:s","urn:p":[{"@value":"one","@annotation":[{"urn:source":[{"@id":"urn:doc"}]}]}],"urn:q":[{"urn:p":[{"@value":"nested"}]}],"urn:j":[{"@value":{"@id":"opaque"},"@type":"@json"}],"urn:t":[{"@triple":{"@subject":{"@id":"urn:s"},"@predicate":"urn:p","@object":{"@id":"urn:o"}}}]},{"@id":"urn:s","urn:p":[{"@value":"two"}]}]"#).unwrap();
    let config = br#"{"version":1,"format":"jsonld","context":{"p":"urn:p","q":"urn:q","j":{"@id":"urn:j","@type":"@json"}}}"#;
    let expanded = expand(source.as_bytes(), config).unwrap();
    assert_eq!(json::read(&expanded).unwrap(), expected);
    assert_eq!(
        expand_decoded(source.as_bytes(), &request(config).unwrap()).unwrap(),
        expanded
    );
    let mut yaml_config = json::read_slice(config, json::Limits::DEFAULT).unwrap();
    yaml_config
        .as_object_mut()
        .unwrap()
        .insert("format", "yamlld");
    let yaml_source = yaml::write(&json::read(source).unwrap());
    assert_eq!(
        json::read(
            &expand(
                yaml_source.as_bytes(),
                json::write_compact(&yaml_config).as_bytes()
            )
            .unwrap()
        )
        .unwrap(),
        expected
    );

    let offline = br#"{"version":1,"format":"jsonld","base":"https://base.test/doc","context":"https://context.test/schema","registry":[{"iri":"https://context.test/schema","format":"yaml","document":"'@context': {p: 'urn:p'}\n","yaml_ld":{}}]}"#;
    assert_eq!(
        json::read(&expand(br#"{"@id":"item","p":"x"}"#, offline).unwrap()).unwrap(),
        json::read(r#"[{"@id":"https://base.test/item","urn:p":[{"@value":"x"}]}]"#).unwrap()
    );
    let scoped = br#"{"version":1,"format":"jsonld","context":{"p":{"@id":"urn:p","@context":{"q":"urn:q"}}}}"#;
    assert_eq!(
        json::read(&expand(br#"{"p":{"q":"x"}}"#, scoped).unwrap()).unwrap(),
        json::read(r#"[{"urn:p":[{"urn:q":[{"@value":"x"}]}]}]"#).unwrap()
    );
    assert!(
        request(br#"{"version":1,"format":"jsonld"}"#)
            .unwrap()
            .initial_context()
            .is_none()
    );
    assert_eq!(
        request(br#"{"version":1,"format":"jsonld","context":null}"#)
            .unwrap()
            .initial_context(),
        Some(&Value::Null)
    );
    let stream = "'@id': first\nurn:p: one\n---\n'@id': second\nurn:p: two\n";
    for (controls, count) in [("{}", 1), (r#"{"extract_all_scripts":true}"#, 2)] {
        let config = format!(
            r#"{{"version":1,"format":"yamlld","base":"https://base.test/","yaml_ld":{controls}}}"#
        );
        let output = json::read(&expand(stream.as_bytes(), config.as_bytes()).unwrap()).unwrap();
        assert_eq!(output.as_array().unwrap().len(), count);
        assert_eq!(
            output.pointer("/0/@id").and_then(Value::as_str),
            Some("https://base.test/first")
        );
    }
    for invalid in [
        r#"{"version":2,"format":"jsonld"}"#,
        r#"{"version":1,"format":"turtle"}"#,
        r#"{"version":1,"format":"unknown"}"#,
        r#"{"version":1,"version":1,"format":"jsonld"}"#,
        r#"{"version":1,"format":"jsonld","unknown":true}"#,
        r#"{"version":1,"format":null}"#,
        r#"{"version":1,"format":"jsonld","base":"relative"}"#,
        r#"{"version":1,"format":"jsonld","context":{"@context":{}}}"#,
        r#"{"version":1,"format":"jsonld","context":[{},[{"@context":null}]]}"#,
        r#"{"version":1,"format":"jsonld","yaml_ld":{}}"#,
        r#"{"version":1,"format":"yamlld","yaml_ld":{"syntax":{"max_input_bytes":4294967296}}}"#,
        r#"{"version":1,"format":"jsonld","output":null}"#,
        r#"{"version":1,"format":"jsonld","output":{"max_bytes":1e3}}"#,
        r#"{"version":1,"format":"jsonld","output":{"max_bytes":18446744073709551616}}"#,
        r#"{"version":1,"format":"jsonld","expansion":{"unknown":0}}"#,
    ] {
        let expected_code = request(invalid.as_bytes()).unwrap_err().code;
        assert_eq!(
            expand(&[0xff], invalid.as_bytes()).unwrap_err().code,
            expected_code,
            "{invalid}"
        );
    }
    let minimal = br#"{"urn:p":"x"}"#;
    let exact = r#"[{"urn:p":[{"@value":"x"}]}]"#;
    for maximum in [u64::try_from(exact.len()).unwrap(), 4_294_967_296, u64::MAX] {
        let config = format!(
            r#"{{"version":1,"format":"jsonld","output":{{"max_bytes":{maximum},"max_allocation_bytes":34359738368}}}}"#
        );
        assert_eq!(expand(minimal, config.as_bytes()).unwrap(), exact);
    }
    for maximum in [0, exact.len() - 1] {
        let config = format!(
            r#"{{"version":1,"format":"jsonld","output":{{"max_bytes":{maximum},"max_allocation_bytes":1048576}}}}"#
        );
        let error = expand(minimal, config.as_bytes()).unwrap_err();
        assert_eq!(error.code, "document-expansion-output");
        assert!(error.native_cause().is_some());
        assert!(error.storage_error().is_none());
    }
    for phase in ["expansion", "output"] {
        let config =
            format!(r#"{{"version":1,"format":"jsonld","{phase}":{{"max_allocation_bytes":0}}}}"#);
        assert_eq!(
            expand(minimal, config.as_bytes())
                .unwrap_err()
                .storage_error(),
            Some(StorageError::AdmissionFailed)
        );
    }
}
