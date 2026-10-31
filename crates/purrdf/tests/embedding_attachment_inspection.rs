// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Inspect original graph payloads and actual native refusals without rendering.

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_hash::error::ErrorValue;
use purrdf_testkit::error_capture::Capture;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn embedding_attachment_failures_keep_original_pack_causes_after_input_drop() {
    use purrdf::{
        CertifiedEmbeddingSource, ContentDigest, EmbeddingError, EmbeddingSource,
        EmbeddingSourceError, ExternalBindingContract, ExternalScope, PackError, RdfDatasetBuilder,
        SourceAttachmentError,
    };
    let dataset = RdfDatasetBuilder::new().freeze().unwrap();
    let (_, mut bytes) = CertifiedEmbeddingSource::from_dataset(
        &dataset,
        &mut purrdf_core::CanonicalizationControl::new(purrdf_core::RDFC_V2_CORPUS_POLICY, None),
    )
    .unwrap();
    bytes[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
    let contract = ExternalBindingContract {
        role: "https://example.org/role/source".into(),
        media_type: "application/vnd.purrdf.pack".into(),
        stable_identifier: None,
        revision: None,
        policy_reference: None,
    };
    let scope = ExternalScope::Source(ContentDigest::of(b"owner"));
    let source = CertifiedEmbeddingSource::certify(
        EmbeddingSource::Purrpck(&bytes),
        &mut purrdf_core::CanonicalizationControl::new(purrdf_core::RDFC_V2_CORPUS_POLICY, None),
    )
    .unwrap_err();
    let external: SourceAttachmentError = purrdf::bind_external_pack(scope, &bytes, &contract)
        .unwrap_err()
        .into();
    drop(bytes);
    drop(dataset);
    drop(contract);
    for (error, code) in [
        (source, "purremb-invalid-source-pack"),
        (external, "purremb-invalid-external-pack"),
    ] {
        let window = CurrentThreadWindow::open();
        let retained = error.clone();
        assert_eq!(window.close().allocations, 0);
        drop(error);
        let SourceAttachmentError::Embedding(embedded) = &retained else {
            panic!("native pack error")
        };
        let owner = match embedded {
            EmbeddingError::InvalidSourcePack(cause)
            | EmbeddingError::InvalidExternalPack(cause) => cause,
            other => panic!("unexpected attachment refusal: {other:?}"),
        };
        assert!(matches!(
            owner.as_ref(),
            EmbeddingSourceError::Pack(PackError::UnsupportedVersion(u32::MAX))
        ));
        let mut native = std::error::Error::source(&retained).unwrap();
        while let Some(source) = native.source() {
            native = source;
        }
        assert_eq!(
            native.downcast_ref::<PackError>(),
            Some(&PackError::UnsupportedVersion(u32::MAX))
        );
        let buffer = Vec::with_capacity(32);
        let window = CurrentThreadWindow::open();
        let captured = Capture::read_with_buffer(&retained, buffer);
        assert_eq!(window.close().allocations, 0);
        assert_eq!(captured.code, "purremb-attachment-embedding");
        let captured = captured.child("cause");
        assert_eq!(captured.code, code);
        let native = captured.child("cause").child("cause");
        assert_eq!(native.code, "pack-unsupported-version");
        assert!(matches!(
            native.field("version"),
            ErrorValue::Unsigned(value) if value == u128::from(u32::MAX)
        ));
    }
}

purrdf_testkit::harness_main!(
    embedding_attachment_failures_keep_original_pack_causes_after_input_drop,
);
