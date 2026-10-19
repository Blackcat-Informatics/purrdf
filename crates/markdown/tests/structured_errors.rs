// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original source identity survives the producer and its borrowed input.

use purrdf_hash::error::ErrorValue;
use purrdf_markdown::{Profile, SourceDocument, Vocabulary, analyze};
use purrdf_testkit::error_capture::Capture;

fn source_iri_cause_survives_input_disposal() {
    let profile = Profile::new(
        "inspection",
        1,
        Vocabulary::under("https://example.org/slice/").unwrap(),
    );
    let id = String::from("https://example.org/%");
    let error = analyze(
        &SourceDocument {
            id: &id,
            bytes: b"# Heading\n",
        },
        &profile,
    )
    .unwrap_err();
    drop(id);
    let captured = Capture::read(&error);
    assert_eq!(captured.code, "markdown-malformed-source-id");
    assert!(matches!(
        captured.field("id"),
        ErrorValue::Iri("https://example.org/%")
    ));
    let cause = std::error::Error::source(&error).unwrap();
    assert!(cause.downcast_ref::<purrdf_core::IriError>().is_some());
    let cause = captured.child("cause");
    assert_eq!(cause.code, "iri-bad-percent-encoding");
    assert!(matches!(cause.field("offset"), ErrorValue::ByteCount(20)));
    assert!(
        analyze(
            &SourceDocument {
                id: "https://example.org/doc",
                bytes: b"# Heading\n"
            },
            &profile
        )
        .is_ok()
    );
}

fn tampered_unit_keeps_original_embedding_cause() {
    use purrdf_core::{
        ContentDigest,
        embedding::{CorpusTarget, DocumentTarget},
    };
    let profile = Profile::new(
        "inspection",
        1,
        Vocabulary::under("https://example.org/slice/").unwrap(),
    );
    let source = SourceDocument {
        id: "urn:example:document",
        bytes: b"original paragraph\n",
    };
    let document = analyze(&source, &profile).unwrap();
    let corpus = CorpusTarget {
        manifest_digest: ContentDigest::of(b"manifest"),
        manifest_media_type: "application/example".into(),
        logical_id_digest: ContentDigest::of(b"corpus"),
    }
    .into_target(false)
    .unwrap();
    let target = DocumentTarget::from_content(
        corpus.id,
        ContentDigest::of(source.id.as_bytes()),
        "text/markdown;charset=utf-8",
        source.bytes,
    )
    .unwrap()
    .into_target(false)
    .unwrap();
    let unit = &document.units()[0];
    let error = purrdf_markdown::verify_unit(
        b"changed! paragraph\n",
        unit,
        target.id,
        profile.chunking_id(),
    )
    .unwrap_err();
    let captured = Capture::read(&error);
    assert_eq!(captured.code, "markdown-tampered-unit");
    assert!(
        matches!(captured.field("start"), ErrorValue::ByteCount(value) if value == u128::from(unit.span().start))
    );
    let cause = captured.child("cause");
    assert_eq!(cause.code, "purremb-content-mismatch");
    assert!(matches!(
        cause.field("context"),
        ErrorValue::Text("chunk coordinates or digest")
    ));
    assert!(
        std::error::Error::source(&error)
            .unwrap()
            .downcast_ref::<purrdf_core::embedding::EmbeddingError>()
            .is_some()
    );
    purrdf_markdown::verify_unit(source.bytes, unit, target.id, profile.chunking_id()).unwrap();
}

purrdf_testkit::harness_main!(
    source_iri_cause_survives_input_disposal,
    tampered_unit_keeps_original_embedding_cause
);
