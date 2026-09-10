// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Native import with bounded selected payloads: product-independent carrier contracts.

use ciborium::Value;
use purrdf_gts::model::{Term, TermKind};
use purrdf_gts::reader::FrameContext;
use purrdf_gts::wire::digest_str;
use purrdf_gts::writer::Writer;
use purrdf_rdf::{GtsBlobLimits, GtsBlobSelector, import_gts_events, import_gts_events_with_blobs};

fn limits() -> GtsBlobLimits {
    GtsBlobLimits::new(1024 * 1024, 2 * 1024 * 1024)
}
fn meta(digest: &str, rep: &str) -> Value {
    Value::Map(vec![
        ("digest".into(), digest.into()),
        ("rep".into(), rep.into()),
    ])
}
fn term(kind: TermKind, value: &str) -> Term {
    Term {
        kind,
        value: Some(value.into()),
        datatype: None,
        lang: None,
        direction: None,
        reifier: None,
        triple: None,
    }
}

#[test]
fn selected_import_preserves_native_scope_and_late_rdf_with_exact_provenance() {
    let mut bytes = Vec::new();
    for segment in 0..2 {
        let mut writer = Writer::new("generic");
        writer.add_blob(
            b"native laws",
            Some("application/octet-stream"),
            Some("laws"),
        );
        // RDF descriptions may follow the blob; they cannot affect pub.rep selection.
        writer.add_terms(&[
            term(TermKind::Iri, "http://example.org/s"),
            term(TermKind::Iri, "http://example.org/p"),
            term(TermKind::Bnode, "b1"),
        ]);
        writer.add_quads(&[(0, 1, 2, None)]);
        if segment == 1 {
            writer.add_blob(b"unused", None, Some("other"));
        }
        bytes.extend(writer.into_bytes());
    }
    let old = import_gts_events(&bytes).expect("legacy native import");
    let imported =
        import_gts_events_with_blobs(&bytes, &[GtsBlobSelector::Representation("laws")], limits())
            .expect("selected import");
    assert_eq!(imported.bundle.envelope, old.envelope);
    assert_eq!(
        imported.bundle.dataset.owned_quads().collect::<Vec<_>>(),
        old.dataset.owned_quads().collect::<Vec<_>>()
    );
    assert_eq!(
        imported.bundle.dataset.quad_count(),
        2,
        "segment blank scopes must stay distinct"
    );
    let [blob] = imported.blobs.as_slice() else {
        panic!("one selected digest");
    };
    assert_eq!(&*blob.bytes, b"native laws");
    assert_eq!(blob.digest, digest_str(b"native laws"));
    assert_eq!(blob.segment_index, 1);
    assert_eq!(blob.frame_index, 0);
    assert_eq!(blob.segment_head.len(), 32);
    let ctx = FrameContext {
        segment_index: blob.segment_index,
        frame_index: blob.frame_index,
        content_id: &blob.frame_id,
        range: blob.frame_range.clone(),
        frame_type: "blob",
        valid: true,
    };
    assert!(ctx.verify(&bytes));
    let source = blob
        .metadata_source
        .as_ref()
        .expect("explicit public metadata");
    assert_eq!(source.frame_id, blob.frame_id);
    assert_eq!(source.segment_head, blob.segment_head);
}

#[test]
fn selected_import_decodes_each_supported_transform_with_bounded_growth() {
    let data = vec![b'x'; 100_000];
    for codec in ["identity", "gzip", "zstd", "zstd-rsyncable"] {
        let mut writer = Writer::new("generic");
        writer
            .add_blob_transformed(data.clone(), None, Some("wanted"), &[codec.into()], None)
            .unwrap();
        let bytes = writer.into_bytes();
        let exact = GtsBlobLimits::new(data.len(), data.len());
        let result = import_gts_events_with_blobs(
            &bytes,
            &[GtsBlobSelector::Representation("wanted")],
            exact,
        )
        .expect("exact decoded bound");
        assert_eq!(&*result.blobs[0].bytes, data);
        let too_small = GtsBlobLimits::new(data.len() - 1, data.len());
        let error = import_gts_events_with_blobs(
            &bytes,
            &[GtsBlobSelector::Representation("wanted")],
            too_small,
        )
        .unwrap_err();
        assert!(
            matches!(
                error.code.as_str(),
                "rdf-ir-gts-blob-decode" | "rdf-ir-gts-blob-limit"
            ),
            "{error:?}"
        );
    }
}

#[test]
fn unselected_payload_integrity_remains_lazy_while_selected_integrity_is_mandatory() {
    let mut writer = Writer::new("generic");
    writer.add_blob(b"wanted", None, Some("wanted"));
    writer.add_frame(
        "blob",
        None,
        Some(b"wrong digest".to_vec()),
        None,
        Some(meta(&digest_str(b"something else"), "unused")),
    );
    let bytes = writer.into_bytes();
    assert!(import_gts_events(&bytes).is_ok());
    let selected = import_gts_events_with_blobs(
        &bytes,
        &[GtsBlobSelector::Representation("wanted")],
        limits(),
    )
    .unwrap();
    assert_eq!(selected.blobs.len(), 1);
    assert_eq!(&*selected.blobs[0].bytes, b"wanted");
    let error = import_gts_events_with_blobs(
        &bytes,
        &[GtsBlobSelector::Representation("unused")],
        limits(),
    )
    .unwrap_err();
    assert_eq!(error.code, "rdf-ir-gts-blob-digest");
}

#[test]
fn final_replacement_evicts_old_representation_and_reuses_one_digest() {
    let mut writer = Writer::new("generic");
    writer.add_blob(b"first", None, Some("wanted"));
    writer.add_blob(b"first", None, Some("other"));
    writer.add_blob(b"second", None, Some("wanted"));
    writer.add_blob(b"second", None, Some("wanted"));
    let result = import_gts_events_with_blobs(
        &writer.into_bytes(),
        &[
            GtsBlobSelector::Representation("wanted"),
            GtsBlobSelector::Digest(&digest_str(b"second")),
        ],
        limits(),
    )
    .unwrap();
    assert_eq!(result.blobs.len(), 1);
    assert_eq!(&*result.blobs[0].bytes, b"second");
    assert_eq!(result.blobs[0].frame_index, 3);
}

#[test]
fn missing_and_ambiguous_and_duplicate_selectors_fail_closed() {
    let mut writer = Writer::new("generic");
    writer.add_blob(b"first", None, Some("wanted"));
    writer.add_blob(b"second", None, Some("wanted"));
    let bytes = writer.into_bytes();
    for selectors in [
        vec![GtsBlobSelector::Representation("absent")],
        vec![GtsBlobSelector::Representation("wanted")],
        vec![GtsBlobSelector::Digest("x"), GtsBlobSelector::Digest("x")],
    ] {
        assert_eq!(
            import_gts_events_with_blobs(&bytes, &selectors, limits())
                .unwrap_err()
                .code,
            "rdf-ir-gts-blob-selection"
        );
    }
    assert!(
        import_gts_events_with_blobs(&bytes, &[], limits())
            .unwrap()
            .blobs
            .is_empty()
    );
}

#[test]
fn metadata_only_updates_preserve_payload_provenance_and_reject_unretained_ordering() {
    let data = b"native";
    let digest = digest_str(data);
    for initially_selected in [false, true] {
        let mut first = Writer::new("generic");
        first.add_blob(
            data,
            None,
            Some(if initially_selected {
                "wanted"
            } else {
                "other"
            }),
        );
        let first_head = first.head().to_vec();
        let mut bytes = first.into_bytes();
        let mut next = Writer::new("generic");
        let updated = Value::Map(vec![
            ("digest".into(), digest.clone().into()),
            ("rep".into(), "wanted".into()),
            (
                "custom".into(),
                Value::Array(vec![Value::Bytes(vec![1, 2, 3])]),
            ),
        ]);
        let metadata_frame = next.add_frame("blob", None, None, None, Some(updated.clone()));
        let metadata_head = next.head().to_vec();
        bytes.extend(next.into_bytes());
        assert!(
            import_gts_events(&bytes).is_ok(),
            "legacy metadata-only behavior is preserved"
        );
        let result = import_gts_events_with_blobs(
            &bytes,
            &[GtsBlobSelector::Representation("wanted")],
            limits(),
        );
        if initially_selected {
            let result = result.unwrap();
            let blob = &result.blobs[0];
            assert_eq!(&*blob.bytes, data);
            assert_eq!(
                purrdf_gts::wire::canonical(blob.metadata.as_ref().unwrap()),
                purrdf_gts::wire::canonical(&updated)
            );
            assert_eq!(blob.segment_index, 0);
            assert_eq!(blob.segment_head, first_head);
            let source = blob.metadata_source.as_ref().unwrap();
            assert_eq!(source.segment_index, 1);
            assert_eq!(source.frame_id, metadata_frame);
            assert_eq!(source.segment_head, metadata_head);
        } else {
            assert_eq!(result.unwrap_err().code, "rdf-ir-gts-blob-selection-order");
        }
    }
}

#[test]
fn absent_public_metadata_preserves_previous_segment_metadata() {
    let data = b"native";
    let mut first = Writer::new("generic");
    first.add_blob(data, None, Some("wanted"));
    let first_head = first.head().to_vec();
    let mut bytes = first.into_bytes();
    let mut next = Writer::new("generic");
    next.add_frame("blob", None, Some(data.to_vec()), None, None);
    bytes.extend(next.into_bytes());
    let result = import_gts_events_with_blobs(
        &bytes,
        &[GtsBlobSelector::Representation("wanted")],
        limits(),
    )
    .unwrap();
    let blob = &result.blobs[0];
    assert_eq!(blob.segment_index, 1);
    assert_eq!(&*blob.bytes, data);
    let source = blob.metadata_source.as_ref().unwrap();
    assert_eq!(source.segment_index, 0);
    assert_eq!(source.segment_head, first_head);
}

#[test]
fn retained_total_wire_and_metadata_budgets_are_independent() {
    let mut writer = Writer::new("generic");
    writer.add_blob(b"first", None, Some("a"));
    writer.add_blob(b"second", None, Some("b"));
    let bytes = writer.into_bytes();
    let selectors = [
        GtsBlobSelector::Representation("a"),
        GtsBlobSelector::Representation("b"),
    ];
    let mut total = limits();
    total.max_total_decoded_bytes = 10;
    assert_eq!(
        import_gts_events_with_blobs(&bytes, &selectors, total)
            .unwrap_err()
            .code,
        "rdf-ir-gts-blob-decode"
    );
    let mut wire = limits();
    wire.max_encoded_bytes = 4;
    assert_eq!(
        import_gts_events_with_blobs(&bytes, &selectors, wire)
            .unwrap_err()
            .code,
        "rdf-ir-gts-blob-limit"
    );
    let mut metadata = limits();
    metadata.max_metadata_bytes = 2;
    assert_eq!(
        import_gts_events_with_blobs(&bytes, &selectors, metadata)
            .unwrap_err()
            .code,
        "rdf-ir-gts-blob-limit"
    );
}

#[test]
fn malformed_selected_metadata_and_declared_lengths_fail() {
    let data = b"native";
    for (key, value, expected) in [
        ("len", Value::from(99), "rdf-ir-gts-blob-length"),
        // A second "rep" entry makes the metadata map ambiguous.
        ("rep", Value::from("wanted"), "rdf-ir-gts-blob-metadata"),
        ("mt", Value::from(99), "rdf-ir-gts-blob-metadata"),
    ] {
        let mut fields = vec![
            ("digest".into(), digest_str(data).into()),
            ("rep".into(), "wanted".into()),
        ];
        fields.push((key.into(), value));
        let mut writer = Writer::new("generic");
        writer.add_frame(
            "blob",
            None,
            Some(data.to_vec()),
            None,
            Some(Value::Map(fields)),
        );
        assert_eq!(
            import_gts_events_with_blobs(
                &writer.into_bytes(),
                &[GtsBlobSelector::Representation("wanted")],
                limits()
            )
            .unwrap_err()
            .code,
            expected
        );
    }
}

#[test]
fn snapshot_payload_selection_and_reader_diagnostics_remain_native() {
    let data = b"snapshot";
    let mut writer = Writer::new("generic");
    writer.add_frame(
        "snapshot",
        Some(Value::Map(vec![(
            "blobs".into(),
            Value::Map(vec![("ignored-key".into(), Value::Bytes(data.to_vec()))]),
        )])),
        None,
        None,
        None,
    );
    let result = import_gts_events_with_blobs(
        &writer.into_bytes(),
        &[GtsBlobSelector::Digest(&digest_str(data))],
        limits(),
    )
    .unwrap();
    assert_eq!(&*result.blobs[0].bytes, data);
    assert!(result.blobs[0].metadata.is_none());
    for malformed in [b"not gts".as_slice(), b"".as_slice()] {
        assert_eq!(
            import_gts_events_with_blobs(malformed, &[], limits())
                .unwrap_err()
                .code,
            import_gts_events(malformed).unwrap_err().code
        );
    }
}

#[test]
fn inherited_same_segment_metadata_keeps_its_original_frame_identity() {
    let mut writer = Writer::new("generic");
    let first_frame = writer.add_blob(b"native", None, Some("wanted"));
    writer.add_frame("blob", None, Some(b"native".to_vec()), None, None);
    let head = writer.head().to_vec();
    let imported = import_gts_events_with_blobs(
        &writer.into_bytes(),
        &[GtsBlobSelector::Representation("wanted")],
        limits(),
    )
    .unwrap();
    let blob = &imported.blobs[0];
    assert_eq!(blob.frame_index, 1);
    let source = blob.metadata_source.as_ref().unwrap();
    assert_eq!(source.frame_index, 0);
    assert_eq!(source.frame_id, first_frame);
    assert_eq!(source.segment_head, head);
}

#[test]
fn selection_count_and_unadvertised_blob_expansion_are_bounded() {
    let selectors: Vec<_> = (0..1025).map(|index| format!("rep-{index}")).collect();
    let selectors: Vec<_> = selectors
        .iter()
        .map(|rep| GtsBlobSelector::Representation(rep))
        .collect();
    let bytes = Writer::new("generic").into_bytes();
    assert_eq!(
        import_gts_events_with_blobs(&bytes, &selectors, limits())
            .unwrap_err()
            .code,
        "rdf-ir-gts-blob-selection"
    );
    let mut writer = Writer::new("generic");
    for index in 0_u32..1025 {
        writer.add_blob(&index.to_be_bytes(), None, Some("wanted"));
    }
    assert_eq!(
        import_gts_events_with_blobs(
            &writer.into_bytes(),
            &[GtsBlobSelector::Representation("wanted")],
            limits()
        )
        .unwrap_err()
        .code,
        "rdf-ir-gts-blob-limit"
    );
    let mut writer = Writer::new("generic");
    writer.add_frame(
        "blob",
        None,
        Some(vec![b'x'; 100_000]),
        Some(&["zstd".into()]),
        None,
    );
    let bytes = writer.into_bytes();
    assert!(import_gts_events(&bytes).is_ok());
    assert_eq!(
        import_gts_events_with_blobs(&bytes, &[], GtsBlobLimits::new(100, 100))
            .unwrap_err()
            .code,
        "rdf-ir-gts-fold-diagnostic"
    );
}

#[test]
fn malformed_payload_cannot_masquerade_as_a_metadata_only_update() {
    let data = b"native";
    let mut writer = Writer::new("generic");
    writer.add_blob(data, None, Some("wanted"));
    writer.add_frame(
        "blob",
        Some(Value::from("not bytes")),
        None,
        None,
        Some(meta(&digest_str(data), "wanted")),
    );
    let bytes = writer.into_bytes();
    assert!(
        import_gts_events(&bytes).is_ok(),
        "legacy metadata observation remains compatible"
    );
    assert_eq!(
        import_gts_events_with_blobs(
            &bytes,
            &[GtsBlobSelector::Representation("wanted")],
            limits()
        )
        .unwrap_err()
        .code,
        "rdf-ir-gts-blob-payload"
    );
}

#[test]
fn legal_public_digest_encodings_preserve_raw_metadata_and_native_identity() {
    let data = b"native digest encodings";
    let digest = digest_str(data);
    let hex = digest.strip_prefix("blake3:").unwrap();
    for declaration in [
        Value::Text(digest.clone()),
        Value::Text(hex.to_owned()),
        Value::Bytes(purrdf_gts::wire::blake3_256(data)),
    ] {
        let metadata = Value::Map(vec![
            ("digest".into(), declaration),
            ("rep".into(), "wanted".into()),
        ]);
        let mut writer = Writer::new("generic");
        writer.add_frame(
            "blob",
            None,
            Some(data.to_vec()),
            None,
            Some(metadata.clone()),
        );
        let bytes = writer.into_bytes();
        let legacy = import_gts_events(&bytes).unwrap();
        let selected = import_gts_events_with_blobs(
            &bytes,
            &[
                GtsBlobSelector::Digest(&digest),
                GtsBlobSelector::Representation("wanted"),
            ],
            limits(),
        )
        .unwrap();
        assert_eq!(selected.bundle.envelope, legacy.envelope);
        assert_eq!(selected.blobs.len(), 1);
        assert_eq!(selected.blobs[0].digest, digest);
        assert_eq!(&*selected.blobs[0].bytes, data);
        assert_eq!(
            purrdf_gts::wire::canonical(selected.blobs[0].metadata.as_ref().unwrap()),
            purrdf_gts::wire::canonical(&metadata),
            "metadata keeps its original digest representation"
        );
    }
}

#[test]
fn eager_digest_resolution_enforces_the_original_encoded_length() {
    for data in [vec![b'x'], vec![b'x'; 10_000]] {
        for codec in ["gzip", "zstd", "zstd-rsyncable"] {
            let chain = [codec.to_owned()];
            let encoded_len = purrdf_gts::codec::encode_chain(&chain, &data)
                .unwrap()
                .len();
            assert_ne!(
                encoded_len,
                data.len(),
                "fixture distinguishes wire and decoded lengths"
            );
            let mut writer = Writer::new("generic");
            writer.add_frame("blob", None, Some(data.clone()), Some(&chain), None);
            let bytes = writer.into_bytes();
            assert!(import_gts_events(&bytes).is_ok());
            let digest = digest_str(&data);
            let mut exact = GtsBlobLimits::new(data.len(), data.len());
            exact.max_encoded_bytes = encoded_len;
            let selected =
                import_gts_events_with_blobs(&bytes, &[GtsBlobSelector::Digest(&digest)], exact)
                    .expect("independent exact wire and decoded bounds");
            assert_eq!(&*selected.blobs[0].bytes, data);
            let mut too_small = exact;
            too_small.max_encoded_bytes -= 1;
            let error = import_gts_events_with_blobs(
                &bytes,
                &[GtsBlobSelector::Digest(&digest)],
                too_small,
            )
            .unwrap_err();
            assert_eq!(error.code, "rdf-ir-gts-fold-diagnostic");
            assert!(error.message.contains("encoded blob exceeds"), "{error:?}");
        }
    }
}

#[test]
fn malformed_public_metadata_is_not_silently_treated_as_absent() {
    let data = b"native";
    let mut writer = Writer::new("generic");
    writer.add_frame(
        "blob",
        None,
        Some(data.to_vec()),
        None,
        Some(Value::from("not a map")),
    );
    let bytes = writer.into_bytes();
    assert!(
        import_gts_events(&bytes).is_ok(),
        "legacy importer behavior remains unchanged"
    );
    let error = import_gts_events_with_blobs(
        &bytes,
        &[GtsBlobSelector::Digest(&digest_str(data))],
        limits(),
    )
    .unwrap_err();
    assert_eq!(error.code, "rdf-ir-gts-blob-metadata");
}
