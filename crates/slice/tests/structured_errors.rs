// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original producer payloads through slice and shared native error boundaries.

use purrdf_hash::error::{ErrorValue, StructuredError};
use purrdf_lex::json::Value;
use purrdf_slice::{ArtifactRecord, ArtifactRole, NamedNode, SliceError};
use purrdf_testkit::error_capture::{Capture, Event};

#[derive(Debug)]
struct NativeIo {
    offset: u64,
}

impl core::fmt::Display for NativeIo {
    fn fmt(&self, _: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        panic!("capturing a native I/O cause must not render it")
    }
}

impl std::error::Error for NativeIo {}

impl StructuredError for NativeIo {
    purrdf_hash::error_visit! {
        @record self { offset } => "slice-native-io-control" {
            "offset" => ByteCount(u128::from(self.offset)),
        }
    }
}

fn native_io_ownership_conversion_never_renders_the_original_cause() {
    let error = SliceError::from(purrdf_hash::error::io_error(
        std::io::ErrorKind::Other,
        NativeIo { offset: u64::MAX },
    ));
    let captured = Capture::read(&error);
    assert_eq!(captured.code, "slice-io");
    let native = captured.child("cause").child("cause").child("cause");
    assert_eq!(native.code, "slice-native-io-control");
    assert!(
        matches!(native.field("offset"), ErrorValue::ByteCount(value) if value == u128::from(u64::MAX))
    );
    let mut source = std::error::Error::source(&error);
    loop {
        let cause = source.expect("the original native cause remains reachable");
        if let Some(native) = cause.downcast_ref::<NativeIo>() {
            assert_eq!(native.offset, u64::MAX);
            break;
        }
        source = cause.source();
    }
}

fn artifact_json_retains_the_original_invalid_member_after_input_disposal() {
    let record = ArtifactRecord {
        role: ArtifactRole::Manifest,
        logical_path: "manifest.ttl".into(),
        media_type: "text/turtle".into(),
        raw_digest: "ab".repeat(32),
        semantic_digest: None,
        content: vec![0, 255],
    };
    assert_eq!(
        ArtifactRecord::from_json(&record.to_json()).unwrap(),
        record
    );
    let mut input = record.to_json();
    *input.get_mut("content").unwrap() = purrdf_lex::json::read("[18446744073709551615]").unwrap();
    let error = ArtifactRecord::from_json(&input).unwrap_err();
    drop(input);
    let captured = Capture::read(&error).child("reason");
    assert_eq!(captured.code, "slice-json-member");
    assert!(matches!(
        captured.field("name"),
        ErrorValue::Text("content")
    ));
    assert_eq!(captured.child("expected").code, "slice-json-bytes");
    let values = captured.child("value");
    assert_eq!(values.code, "json-array");
    let number = values
        .events
        .iter()
        .find_map(|event| match event {
            Event::Value(None, ErrorValue::Record(value)) => Some(Capture::read(*value)),
            _ => None,
        })
        .unwrap();
    assert!(matches!(
        number.field("lexeme"),
        ErrorValue::Text("18446744073709551615")
    ));
    let SliceError::Json(purrdf_slice::error::JsonRefusal::Member { value, .. }) = &error else {
        panic!("the original invalid member must remain typed");
    };
    assert!(matches!(value, Value::Array(values) if values.len() == 1));
}

fn iri_and_rdf_producers_keep_original_causes_and_context() {
    let input = "https://example.org/original bad iri".to_owned();
    let error = NamedNode::new(input.clone()).unwrap_err();
    drop(input);
    let captured = Capture::read(&error);
    assert_eq!(captured.code, "slice-iri");
    assert!(matches!(
        captured.field("iri"),
        ErrorValue::Iri("https://example.org/original bad iri")
    ));
    assert_eq!(captured.child("cause").code, "sparql-parse-iri");
    NamedNode::new("https://example.org/valid").unwrap();

    let context = "original-source.ttl".to_owned();
    let error =
        purrdf_slice::rdf_query::Dataset::parse_turtle(b"<urn:s> <urn:p> .", None, &context)
            .unwrap_err();
    drop(context);
    let SliceError::Rdf { cause, .. } = &error else {
        panic!("the parser diagnostic must remain native");
    };
    let source = std::error::Error::source(&error)
        .unwrap()
        .downcast_ref::<purrdf_rdf::RdfDiagnostic>()
        .unwrap();
    assert!(std::ptr::eq(source, cause.as_ref()));
    let captured = Capture::read(&error);
    assert!(matches!(
        captured.field("context"),
        ErrorValue::Text("syntax error in original-source.ttl")
    ));
}

fn flat_canonicalization_returns_the_native_reserved_vocabulary_refusal() {
    let rejected = format!("{}foreign", purrdf_core::RESERVED_NAMESPACE);
    let text = format!("<{rejected}> <urn:p> <urn:o> .");
    let dataset =
        purrdf_slice::rdf_query::Dataset::parse_turtle(text.as_bytes(), None, "reserved").unwrap();
    let error = dataset.canonical_nquads_flat().unwrap_err();
    drop(dataset);
    drop(text);
    let captured = Capture::read(&error);
    assert_eq!(captured.code, "canonical-reserved");
    assert!(
        matches!(captured.child("cause").field("iri"), ErrorValue::Iri(value) if value == rejected)
    );
    let accepted = purrdf_slice::rdf_query::Dataset::parse_turtle(
        b"<urn:s> <urn:p> <urn:o> .",
        None,
        "accepted",
    )
    .unwrap();
    assert_eq!(
        accepted.canonical_nquads_flat().unwrap(),
        "<urn:s> <urn:p> <urn:o> .\n"
    );
}

#[cfg(not(target_arch = "wasm32"))]
fn missing_retrieval_path_keeps_the_original_native_path_and_io_kind() {
    let directory = purrdf_testkit::TempDir::for_unit_test().unwrap();
    let path = directory.path().join("uncreated.ttl");
    let expected = path.as_os_str().as_encoded_bytes().to_vec();
    let error = purrdf_slice::retrieval_base_iri(&path).unwrap_err();
    drop(path);
    drop(directory);
    let captured = Capture::read(&error).child("reason");
    assert_eq!(captured.code, "slice-path-canonicalize");
    assert!(matches!(captured.field("path"), ErrorValue::Bytes(value) if value == expected));
    let SliceError::InvalidPath(purrdf_slice::error::PathRefusal::Canonicalize { cause, .. }) =
        &error
    else {
        panic!("the filesystem refusal must remain native");
    };
    assert_eq!(cause.kind(), std::io::ErrorKind::NotFound);
}

purrdf_testkit::harness_main!(
    native_io_ownership_conversion_never_renders_the_original_cause,
    artifact_json_retains_the_original_invalid_member_after_input_disposal,
    iri_and_rdf_producers_keep_original_causes_and_context,
    flat_canonicalization_returns_the_native_reserved_vocabulary_refusal,
    #[cfg(not(target_arch = "wasm32"))]
    missing_retrieval_path_keeps_the_original_native_path_and_io_kind,
);
