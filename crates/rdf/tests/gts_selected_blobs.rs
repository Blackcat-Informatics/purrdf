// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Native import with bounded selected payloads: product-independent carrier contracts.

use ciborium::Value;
use purrdf_gts::model::{Term, TermKind};
use purrdf_gts::reader::FrameContext;
use purrdf_gts::wire::digest_str;
use purrdf_gts::writer::Writer;
use purrdf_rdf::{
    DEFAULT_MAX_METADATA_BYTES, GtsBlobLimits, GtsBlobSelector, import_gts_events,
    import_gts_events_with_blobs,
};

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

/// A container cannot talk its way past the budget seam.
///
/// The selected importer treats a byte-ceiling refusal as non-fatal. That
/// decision must key off the reader's own diagnostic code, never off the
/// human-readable detail — a detail can embed text the container chose (a
/// header's `gts` field is echoed verbatim into the unsupported-version
/// message), so a prose match would let a crafted file pick its own verdict and
/// be admitted by the bounded importer while the authoritative one rejects it.
#[test]
fn container_supplied_text_cannot_forge_a_budget_refusal() {
    for forged in [
        "blob exceeds",
        "encoded blob exceeds 1 bytes",
        "decoded transform output exceeds 1 bytes",
    ] {
        let bytes = container_with_header_magic(forged);

        // The reader echoes the forged magic into a `DamagedFrame` detail, so
        // the detail now contains budget prose the container chose.
        let plain = import_gts_events(&bytes)
            .expect_err("an unsupported header magic is a hard fold failure");
        let selected = import_gts_events_with_blobs(&bytes, &[], limits())
            .expect_err("the bounded importer must refuse it too");
        assert_eq!(plain.code, selected.code, "forged magic {forged:?}");
        assert_eq!(
            selected.code, "rdf-ir-gts-fold-diagnostic",
            "forged magic {forged:?} was admitted as a budget refusal"
        );
    }
}

/// Author a container whose header magic is `magic`, with a valid self-hash.
///
/// The self-hash must be recomputed or the reader stops at "header self-hash
/// mismatch" and never reaches the magic check whose message echoes the input.
fn container_with_header_magic(magic: &str) -> Vec<u8> {
    // Header only: re-authoring it changes its id, which would break the `prev`
    // of any following frame and raise a second, unforgeable `BrokenChain`
    // diagnostic that would mask the very admission this test is probing.
    let authored = Writer::new("generic").into_bytes();

    let mut cursor = std::io::Cursor::new(&authored[..]);
    let item: Value = ciborium::de::from_reader(&mut cursor).expect("header item");
    let header_len = usize::try_from(cursor.position()).expect("header fits in usize");
    let tag = match &item {
        Value::Tag(tag, _) => Some(*tag),
        _ => None,
    };
    let mut entries = purrdf_gts::wire::unwrap_header(&item)
        .expect("authored header is a map")
        .clone();
    for (key, value) in &mut entries {
        if key.as_text() == Some("gts") {
            *value = Value::from(magic);
        }
    }
    let id = purrdf_gts::wire::header_id(&entries);
    for (key, value) in &mut entries {
        if key.as_text() == Some("id") {
            *value = Value::Bytes(id.clone());
        }
    }

    let rebuilt = Value::Map(entries);
    let rebuilt = match tag {
        Some(tag) => Value::Tag(tag, Box::new(rebuilt)),
        None => rebuilt,
    };
    let mut out = Vec::new();
    ciborium::ser::into_writer(&rebuilt, &mut out).expect("re-encode header");
    out.extend_from_slice(&authored[header_len..]);
    out
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
}

/// An oversized *unrelated* payload must not refuse the archive.
///
/// The byte budget names what this import will retain, not what the container is
/// allowed to contain. A caller asking for one small blob out of an archive that
/// also holds a large one it never named must still get its dataset — the large
/// payload is simply neither decoded nor retained. Both sides are exercised here:
/// the unrelated payload is admitted past, and a payload the caller *did* name
/// still fails closed when it exceeds the same budget.
#[test]
fn oversized_unselected_payload_does_not_refuse_the_import() {
    // A digest-less compressed payload far over budget, alongside a small named
    // one. Only the digest-less frame reaches the reader's byte checks.
    let mut writer = Writer::new("generic");
    writer.add_frame(
        "blob",
        None,
        Some(vec![b'x'; 100_000]),
        Some(&["zstd".into()]),
        None,
    );
    writer.add_blob(b"small", None, Some("wanted"));
    let bytes = writer.into_bytes();

    // The authoritative importer has always accepted these bytes.
    assert!(import_gts_events(&bytes).is_ok());

    let tight = GtsBlobLimits::new(100, 100);

    // Neighbouring valid case 1: no selectors at all. This previously failed
    // with `rdf-ir-gts-fold-diagnostic`.
    let none = import_gts_events_with_blobs(&bytes, &[], tight).expect("empty selection imports");
    assert!(
        none.blobs.is_empty(),
        "nothing was selected, so nothing is retained"
    );

    // Neighbouring valid case 2: select only the small blob. The oversized
    // payload is skipped rather than fatal, and is not retained.
    let selected =
        import_gts_events_with_blobs(&bytes, &[GtsBlobSelector::Representation("wanted")], tight)
            .expect("selecting a small blob past a large one imports");
    assert_eq!(selected.blobs.len(), 1);
    assert_eq!(&*selected.blobs[0].bytes, b"small");

    // The dataset is identical to the one the unbudgeted importer produces, and
    // the envelope differs in exactly one documented way: the refused payload is
    // an `over-budget` opaque node rather than a blob record, because the digest
    // that would identify it is only computable by doing the decode the budget
    // declined. Asserting quads alone would let any other divergence hide.
    let plain = import_gts_events(&bytes).expect("plain import");
    assert_eq!(
        selected.bundle.dataset.owned_quads().collect::<Vec<_>>(),
        plain.dataset.owned_quads().collect::<Vec<_>>()
    );
    let (plain_side, selected_side) = (
        &plain.envelope.lookaside,
        &selected.bundle.envelope.lookaside,
    );
    assert_eq!(
        plain_side.blobs.len(),
        2,
        "unbudgeted decodes and records both"
    );
    assert_eq!(plain_side.opaque_nodes, &[]);
    assert_eq!(
        selected_side.blobs.len(),
        1,
        "only the admitted payload is identified"
    );
    let [opaque] = selected_side.opaque_nodes.as_slice() else {
        panic!(
            "the refused payload is recorded: {:?}",
            selected_side.opaque_nodes
        );
    };
    assert_eq!(
        opaque.reason, "over-budget",
        "not conflated with corruption"
    );
    assert_eq!(selected_side.segments, plain_side.segments);
    assert_eq!(selected_side.signatures, plain_side.signatures);
    assert_eq!(selected_side.suppressions, plain_side.suppressions);
    assert_eq!(selected_side.resources, plain_side.resources);
    assert_eq!(selected_side.metadata, plain_side.metadata);

    // Invalid case: a payload the caller *named* that exceeds the same budget
    // still fails closed.
    let mut writer = Writer::new("generic");
    writer.add_blob(&[b'z'; 4096], None, Some("too-big"));
    assert_eq!(
        import_gts_events_with_blobs(
            &writer.into_bytes(),
            &[GtsBlobSelector::Representation("too-big")],
            tight
        )
        .unwrap_err()
        .code,
        "rdf-ir-gts-blob-limit"
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
            // The payload is refused before it can be retained, so the selector
            // finds nothing to resolve against. Still terminal — a caller never
            // receives a blob it asked for and did not get.
            assert_eq!(error.code, "rdf-ir-gts-blob-selection");
            assert!(error.message.contains("resolves to 0 blobs"), "{error:?}");
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

/// A byte budget bounds retention; it must never decide selection.
///
/// An archive holds two payloads that both advertise `rep = "wanted"`. Naming
/// that representation is ambiguous and must fail closed. If a refused payload
/// simply vanished, a tight budget would delete one candidate and hand the
/// caller the survivor as though it were the unique answer — a wrong result
/// produced by a knob that is supposed to govern memory, not meaning.
#[test]
fn a_budget_refusal_cannot_resolve_an_ambiguous_selector() {
    let mut writer = Writer::new("generic");
    writer.add_blob(b"small", None, Some("wanted"));
    // Digest-less, so it is decoded (and therefore budget-checked) rather than
    // resolved eagerly from public metadata, yet it still advertises the rep.
    writer.add_frame(
        "blob",
        None,
        Some(vec![b'x'; 100_000]),
        Some(&["zstd".into()]),
        Some(Value::Map(vec![("rep".into(), "wanted".into())])),
    );
    let bytes = writer.into_bytes();

    for (label, limits) in [
        ("generous", GtsBlobLimits::new(1024 * 1024, 2 * 1024 * 1024)),
        ("tight", GtsBlobLimits::new(100, 100)),
    ] {
        let error = import_gts_events_with_blobs(
            &bytes,
            &[GtsBlobSelector::Representation("wanted")],
            limits,
        )
        .expect_err(&format!("{label} budget must not resolve the ambiguity"));
        assert_eq!(
            error.code, "rdf-ir-gts-blob-selection",
            "{label}: {error:?}"
        );
        assert!(
            error.message.contains("resolves to 2 blobs"),
            "{label}: {error:?}"
        );
    }
}

/// Naming a payload the budget refused is reported as a budget refusal.
///
/// "Resolves to 0 blobs" would be a false statement about an archive that does
/// contain the payload, and indistinguishable from naming a digest that is
/// genuinely absent.
#[test]
fn naming_a_refused_payload_reports_the_budget_not_an_empty_match() {
    let mut writer = Writer::new("generic");
    writer.add_frame(
        "blob",
        None,
        Some(vec![b'x'; 100_000]),
        Some(&["zstd".into()]),
        Some(Value::Map(vec![("rep".into(), "wanted".into())])),
    );
    let bytes = writer.into_bytes();

    let refused = import_gts_events_with_blobs(
        &bytes,
        &[GtsBlobSelector::Representation("wanted")],
        GtsBlobLimits::new(100, 100),
    )
    .expect_err("a named over-budget payload fails closed");
    assert_eq!(refused.code, "rdf-ir-gts-blob-limit", "{refused:?}");
    assert!(refused.message.contains("budget refused"), "{refused:?}");

    // The neighbouring valid case: the same archive, a budget that admits it.
    let ok = import_gts_events_with_blobs(
        &bytes,
        &[GtsBlobSelector::Representation("wanted")],
        limits(),
    )
    .expect("the same selection succeeds within budget");
    assert_eq!(ok.blobs.len(), 1);
    assert_eq!(ok.refused.len(), 0);

    // A genuinely absent digest stays a selection failure, not a budget one.
    let absent = import_gts_events_with_blobs(
        &bytes,
        &[GtsBlobSelector::Digest(&digest_str(
            b"never in this archive",
        ))],
        limits(),
    )
    .expect_err("an absent digest fails closed");
    assert_eq!(absent.code, "rdf-ir-gts-blob-selection", "{absent:?}");
}

/// A successful import never hides what the budget cost the caller.
#[test]
fn an_unselected_refusal_is_reported_rather_than_dropped() {
    let mut writer = Writer::new("generic");
    writer.add_frame(
        "blob",
        None,
        Some(vec![b'x'; 100_000]),
        Some(&["zstd".into()]),
        None,
    );
    writer.add_blob(b"small", None, Some("wanted"));
    let bytes = writer.into_bytes();

    let result = import_gts_events_with_blobs(
        &bytes,
        &[GtsBlobSelector::Representation("wanted")],
        GtsBlobLimits::new(100, 100),
    )
    .expect("imports");
    assert_eq!(result.blobs.len(), 1);
    let [refused] = result.refused.as_slice() else {
        panic!("the oversized payload is reported: {:?}", result.refused);
    };
    assert!(refused.digest.is_none(), "no digest was declared");
    assert_eq!(refused.encoded_len, 100_000_usize.min(refused.encoded_len));
    assert_ne!(refused.detail, "");
}

/// A snapshot must not be a way around the blob ceilings.
///
/// A snapshot frame carries blob bytes *inside* an RDF frame. Bounding only the
/// blob path leaves the decode that actually spends the memory unbounded, so a
/// compressed snapshot could hand a bounded consumer arbitrarily many bytes.
/// Both ceilings are exercised here, each against a neighbouring case that must
/// still import.
#[test]
fn a_snapshot_cannot_smuggle_payloads_past_the_ceilings() {
    let embedded = vec![b'x'; 100_000];
    let snapshot = Value::Map(vec![(
        "blobs".into(),
        Value::Map(vec![("0".into(), Value::Bytes(embedded.clone()))]),
    )]);
    let mut writer = Writer::new("generic");
    writer
        .add_frame_with_options(
            "snapshot",
            purrdf_gts::writer::FrameOptions {
                payload: Some(snapshot),
                transform: vec!["zstd".into()],
                ..purrdf_gts::writer::FrameOptions::default()
            },
        )
        .expect("compressed snapshot frame");
    let bytes = writer.into_bytes();

    // Neighbouring valid case: ceilings that admit it still import it.
    let generous = import_gts_events_with_blobs(
        &bytes,
        &[GtsBlobSelector::Digest(&digest_str(&embedded))],
        limits(),
    )
    .expect("an in-budget snapshot payload is selectable");
    assert_eq!(&*generous.blobs[0].bytes, &embedded[..]);

    // The per-blob ceiling refuses the embedded entry rather than retaining it.
    let mut tight = limits();
    tight.max_decoded_bytes = 1_000;
    let refused = import_gts_events_with_blobs(&bytes, &[], tight).expect("import still succeeds");
    assert_eq!(
        refused.refused.len(),
        1,
        "the embedded entry is refused, not retained: {:?}",
        refused.refused
    );
    assert!(refused.blobs.is_empty());

    // The enclosing frame ceiling bounds the decode that spends the memory.
    let mut framed = limits();
    framed.max_frame_decoded_bytes = 1_000;
    let error = import_gts_events_with_blobs(&bytes, &[], framed)
        .expect_err("a refused frame removes rows, so it cannot be reported as success");
    assert_eq!(error.code, "rdf-ir-gts-fold-diagnostic", "{error:?}");
}

/// An unknown extension key never refuses a payload.
///
/// `blob-pub` admits `* extension-key => any`, and the spec is explicit that a
/// reader must not reject a payload map merely because it carries a key the
/// reader does not know. `len` is one such key: nothing in this workspace emits
/// it, and no profile here assigns it meaning, so a value that disagrees with
/// the decoded length is a claim this reader has no standing to police.
#[test]
fn an_unknown_extension_key_does_not_refuse_a_payload() {
    let data = b"payload";
    for value in [Value::from(99), Value::from("not even a number")] {
        let mut writer = Writer::new("generic");
        writer.add_frame(
            "blob",
            None,
            Some(data.to_vec()),
            None,
            Some(Value::Map(vec![
                ("digest".into(), digest_str(data).into()),
                ("rep".into(), "wanted".into()),
                ("len".into(), value.clone()),
            ])),
        );
        let result = import_gts_events_with_blobs(
            &writer.into_bytes(),
            &[GtsBlobSelector::Representation("wanted")],
            limits(),
        )
        .unwrap_or_else(|error| panic!("len={value:?} must not refuse the payload: {error:?}"));
        assert_eq!(&*result.blobs[0].bytes, data);
    }
}

/// A digest selector names the canonical spelling, and says so when it does not.
///
/// The reader normalizes the three wire spellings of a published digest to
/// `blake3:<hex>`. A selector is matched against that canonical form, so bare
/// hex resolves to nothing — a refusal worth pinning, because the neighbouring
/// canonical spelling must keep working.
#[test]
fn a_digest_selector_requires_the_canonical_spelling() {
    let data = b"payload";
    let mut writer = Writer::new("generic");
    writer.add_blob(data, None, Some("wanted"));
    let bytes = writer.into_bytes();

    let canonical = digest_str(data);
    let found =
        import_gts_events_with_blobs(&bytes, &[GtsBlobSelector::Digest(&canonical)], limits())
            .expect("the canonical spelling resolves");
    assert_eq!(&*found.blobs[0].bytes, data);

    let bare = canonical
        .strip_prefix("blake3:")
        .expect("digest_str publishes the prefixed spelling");
    let error = import_gts_events_with_blobs(&bytes, &[GtsBlobSelector::Digest(bare)], limits())
        .expect_err("bare hex is not the selector spelling");
    assert_eq!(error.code, "rdf-ir-gts-blob-selection", "{error:?}");
}

/// An external blob is refused for the reason it is actually refused.
///
/// The spec makes external blobs first class: the payload field is absent and
/// `pub.digest` names bytes held elsewhere. That is not the same fact as a
/// metadata-only update naming a payload this import declined to retain, and
/// reporting it as one asserts something false about retention ordering.
#[test]
fn an_external_blob_is_distinguished_from_a_metadata_only_update() {
    let data = b"held elsewhere";
    let digest = digest_str(data);

    // External: no payload field anywhere in the container.
    let mut writer = Writer::new("generic");
    writer.add_frame("blob", None, None, None, Some(meta(&digest, "wanted")));
    let external = import_gts_events_with_blobs(
        &writer.into_bytes(),
        &[GtsBlobSelector::Representation("wanted")],
        limits(),
    )
    .expect_err("external bytes cannot be produced by an inline read");
    assert_eq!(external.code, "rdf-ir-gts-blob-external", "{external:?}");
    assert!(external.message.contains("held elsewhere"), "{external:?}");

    // Neighbouring valid case: an inline payload followed by a metadata-only
    // update to the same digest still resolves, and still returns the bytes.
    let mut writer = Writer::new("generic");
    writer.add_blob(data, None, Some("wanted"));
    writer.add_frame("blob", None, None, None, Some(meta(&digest, "wanted")));
    let updated = import_gts_events_with_blobs(
        &writer.into_bytes(),
        &[GtsBlobSelector::Representation("wanted")],
        limits(),
    )
    .expect("a metadata-only update reuses the retained payload");
    assert_eq!(&*updated.blobs[0].bytes, data);
}

/// The transient-candidate cap refuses at 1025, not at 1024.
///
/// The negative side of an off-by-one is invisible: a cap that fired one entry
/// early would still look like correct strictness. The boundary case must be
/// shown to pass through to ordinary selection instead.
#[test]
fn the_transient_candidate_cap_admits_its_own_boundary() {
    for (count, expected) in [
        (1024_u32, "rdf-ir-gts-blob-selection"),
        (1025, "rdf-ir-gts-blob-limit"),
    ] {
        let mut writer = Writer::new("generic");
        for index in 0..count {
            writer.add_blob(&index.to_be_bytes(), None, Some("wanted"));
        }
        let error = import_gts_events_with_blobs(
            &writer.into_bytes(),
            &[GtsBlobSelector::Representation("wanted")],
            limits(),
        )
        .expect_err("every blob shares one representation, so selection is ambiguous");
        assert_eq!(error.code, expected, "count={count}: {error:?}");
    }
}

/// Metadata that names a digest it cannot be read as is refused.
///
/// Distinct from the decoded-digest mismatch: here the public metadata carries a
/// `digest` entry that is not a readable digest at all, so the blob's identity
/// comes from its bytes while its metadata claims something unreadable.
#[test]
fn metadata_naming_an_unreadable_digest_is_refused() {
    let data = b"payload";
    let mut writer = Writer::new("generic");
    writer.add_frame(
        "blob",
        None,
        Some(data.to_vec()),
        None,
        // A digest-shaped value of the wrong length: not resolvable, so the
        // blob falls to identity-by-content while its metadata disagrees.
        Some(Value::Map(vec![
            ("digest".into(), Value::Bytes(vec![0_u8; 31])),
            ("rep".into(), "wanted".into()),
        ])),
    );
    let error = import_gts_events_with_blobs(
        &writer.into_bytes(),
        &[GtsBlobSelector::Representation("wanted")],
        limits(),
    )
    .expect_err("metadata must not name a digest that is not this blob");
    assert_eq!(error.code, "rdf-ir-gts-blob-digest", "{error:?}");

    // Neighbouring valid case: the same payload, metadata naming it correctly.
    let mut writer = Writer::new("generic");
    writer.add_frame(
        "blob",
        None,
        Some(data.to_vec()),
        None,
        Some(meta(&digest_str(data), "wanted")),
    );
    let ok = import_gts_events_with_blobs(
        &writer.into_bytes(),
        &[GtsBlobSelector::Representation("wanted")],
        limits(),
    )
    .expect("correct metadata still imports");
    assert_eq!(&*ok.blobs[0].bytes, data);
}

/// An untransformed payload is bounded by the decoded ceiling too.
///
/// The encoded and decoded ceilings are separate knobs, and a payload with no
/// codec chain reaches only the decoded one. Without a case here, a caller
/// raising the encoded ceiling would silently lose the decoded bound.
#[test]
fn an_untransformed_payload_obeys_the_decoded_ceiling() {
    let data = vec![b'x'; 4096];
    let mut writer = Writer::new("generic");
    // Digest-less, so the reader decodes it and the ceilings apply.
    writer.add_frame("blob", None, Some(data.clone()), None, None);
    let bytes = writer.into_bytes();

    let mut tight = GtsBlobLimits::new(data.len() - 1, data.len());
    tight.max_encoded_bytes = data.len() * 2;
    let refused = import_gts_events_with_blobs(&bytes, &[], tight).expect("not fatal");
    assert_eq!(refused.refused.len(), 1, "{:?}", refused.refused);
    assert!(
        refused.refused[0].detail.contains("decoded blob exceeds"),
        "the decoded ceiling is what fired: {:?}",
        refused.refused[0]
    );

    // Neighbouring valid case: a decoded ceiling that exactly admits it.
    let mut exact = GtsBlobLimits::new(data.len(), data.len());
    exact.max_encoded_bytes = data.len() * 2;
    let ok = import_gts_events_with_blobs(
        &bytes,
        &[GtsBlobSelector::Digest(&digest_str(&data))],
        exact,
    )
    .expect("an exact decoded ceiling admits the payload");
    assert_eq!(&*ok.blobs[0].bytes, &data[..]);
}

/// A frame ceiling must never hand back a silently truncated graph.
///
/// Refusing one blob payload leaves the dataset intact, so the import can
/// continue. Refusing a whole frame removes rows the caller was relying on, and
/// returning `Ok` there would answer a question about the archive's contents
/// with a graph that is missing part of it.
#[test]
fn a_refused_frame_fails_rather_than_truncating_the_dataset() {
    // Terms declared through the writer, then a COMPRESSED quads frame: the
    // frame ceiling only reaches transformed frames, and this one carries rows
    // whose loss would be visible in the dataset.
    let mut writer = Writer::new("generic");
    let mut declared = Vec::new();
    for index in 0..150_u32 {
        declared.push(term(TermKind::Iri, &format!("http://example.org/t{index}")));
    }
    writer.add_terms(&declared);
    let rows: Vec<Value> = (0..50_u64)
        .map(|index| {
            Value::Array(vec![
                Value::from(index * 3),
                Value::from(index * 3 + 1),
                Value::from(index * 3 + 2),
            ])
        })
        .collect();
    writer
        .add_frame_with_options(
            "quads",
            purrdf_gts::writer::FrameOptions {
                payload: Some(Value::Array(rows)),
                transform: vec!["zstd".into()],
                ..purrdf_gts::writer::FrameOptions::default()
            },
        )
        .expect("compressed quads frame");
    let bytes = writer.into_bytes();

    let plain = import_gts_events(&bytes).expect("authoritative import");
    assert!(plain.dataset.quad_count() > 0, "the fixture carries rows");

    // Neighbouring valid case: a frame ceiling that admits it returns the same
    // dataset the authoritative importer builds.
    let admitted =
        import_gts_events_with_blobs(&bytes, &[], limits()).expect("an in-budget frame imports");
    assert_eq!(
        admitted.bundle.dataset.owned_quads().collect::<Vec<_>>(),
        plain.dataset.owned_quads().collect::<Vec<_>>()
    );

    // A ceiling that refuses the frame is terminal, not a quiet truncation.
    let mut tight = limits();
    tight.max_frame_decoded_bytes = 64;
    let error = import_gts_events_with_blobs(&bytes, &[], tight)
        .expect_err("a dropped frame cannot be reported as success");
    assert_eq!(error.code, "rdf-ir-gts-fold-diagnostic", "{error:?}");
}

/// One blob stored twice is still one blob, whichever copy the budget refuses.
///
/// A container may hold the same payload compressed and uncompressed. Refusing
/// the copy that does not fit must not invent a second candidate and turn a
/// unique selector into an ambiguous one.
#[test]
fn a_refused_duplicate_copy_does_not_invent_an_ambiguity() {
    let data = vec![b'x'; 50_000];
    let digest = digest_str(&data);
    let mut writer = Writer::new("generic");
    // Copy A: compressed, so it fits a tight encoded ceiling.
    writer
        .add_blob_transformed(data.clone(), None, Some("wanted"), &["zstd".into()], None)
        .expect("compressed copy");
    // Copy B: the same bytes uncompressed, which a tight ceiling refuses.
    writer.add_frame("blob", None, Some(data.clone()), None, None);
    let bytes = writer.into_bytes();

    // Generous: both copies are admitted, and they are one blob.
    let generous = import_gts_events_with_blobs(
        &bytes,
        &[GtsBlobSelector::Representation("wanted")],
        limits(),
    )
    .expect("one blob stored twice is not ambiguous");
    assert_eq!(generous.blobs.len(), 1);
    assert_eq!(generous.blobs[0].digest, digest);
    assert_eq!(generous.refused.len(), 0);

    // Tight: an encoded ceiling that admits the compressed copy and refuses the
    // uncompressed one. The refusal must actually happen, or this proves nothing.
    let mut tight = limits();
    tight.max_encoded_bytes = data.len() / 2;
    let result =
        import_gts_events_with_blobs(&bytes, &[GtsBlobSelector::Representation("wanted")], tight)
            .expect("refusing the second copy of one blob is not an ambiguity");
    assert_eq!(
        result.refused.len(),
        1,
        "the uncompressed copy must really be refused: {:?}",
        result.refused
    );
    assert_eq!(
        result.refused[0].digest.as_deref(),
        Some(digest.as_str()),
        "an untransformed refusal knows its own identity"
    );
    assert_eq!(result.blobs.len(), 1);
    assert_eq!(result.blobs[0].digest, digest);
}

/// A metadata-only frame may precede its own inline payload.
///
/// Whether a blob is external is only decidable once the container has been
/// read: the bytes may be two frames further on. Deciding at the occurrence
/// reported a file the authoritative importer accepts as external.
#[test]
fn a_metadata_frame_may_precede_the_payload_it_describes() {
    let data = b"arrives later";
    let digest = digest_str(data);

    let mut writer = Writer::new("generic");
    writer.add_frame("blob", None, None, None, Some(meta(&digest, "wanted")));
    writer.add_blob(data, None, Some("wanted"));
    let bytes = writer.into_bytes();

    assert!(
        import_gts_events(&bytes).is_ok(),
        "the authoritative importer accepts a forward reference"
    );
    let result = import_gts_events_with_blobs(
        &bytes,
        &[GtsBlobSelector::Representation("wanted")],
        limits(),
    )
    .expect("a forward inline reference is not an external blob");
    assert_eq!(&*result.blobs[0].bytes, data);
}

/// A representation selector reads the FINAL metadata, including when deciding
/// whether an unresolved digest was ever really named.
///
/// A digest can be tagged with a representation and then retagged away before
/// the container ends. Judging it by the tag it briefly held refuses a file the
/// authoritative importer accepts, over a blob the selection never reaches.
#[test]
fn a_retagged_digest_is_not_refused_over_a_representation_it_lost() {
    let absent = digest_str(b"never inline anywhere");
    let present = b"the one actually named";

    let mut writer = Writer::new("generic");
    // Transiently claims "wanted", with no payload...
    writer.add_frame("blob", None, None, None, Some(meta(&absent, "wanted")));
    // ...then gives it up.
    writer.add_frame("blob", None, None, None, Some(meta(&absent, "other")));
    // A different blob is what finally answers the selector.
    writer.add_blob(present, None, Some("wanted"));
    let bytes = writer.into_bytes();

    assert!(
        import_gts_events(&bytes).is_ok(),
        "the authoritative importer accepts this container"
    );
    let result = import_gts_events_with_blobs(
        &bytes,
        &[GtsBlobSelector::Representation("wanted")],
        limits(),
    )
    .expect("the selector resolves to the blob that finally carries the rep");
    assert_eq!(result.blobs.len(), 1);
    assert_eq!(&*result.blobs[0].bytes, present);
}

/// A container cannot suppress an ambiguity by claiming a digest it never proves.
///
/// A payload refused before any decode has no checked identity. Honouring the
/// container's `pub.digest` there would let a hostile file name an already
/// retained blob on an oversized frame and make a real ambiguity vanish — the
/// same "input picks its own verdict" shape the diagnostic-code seam refuses.
#[test]
fn an_unverified_declared_digest_cannot_collapse_two_candidates() {
    let retained = b"small and wanted";
    let retained_digest = digest_str(retained);

    let mut writer = Writer::new("generic");
    writer.add_blob(retained, None, Some("wanted"));
    // An ENCRYPTED oversized payload: the encrypt branch reaches the reader's
    // ceilings even when public metadata declares a digest, so this is the one
    // route by which a refusal can carry an identity nothing ever checked.
    // It falsely claims the retained blob's digest.
    writer
        .add_frame_with_options(
            "blob",
            purrdf_gts::writer::FrameOptions {
                raw: Some(vec![b'z'; 80_000]),
                pub_meta: Some(meta(&retained_digest, "wanted")),
                encrypt: Some(purrdf_gts::writer::Encrypt0Options {
                    kid: "recipient".into(),
                    // Derived, not written: a fixed key literal is a finding in
                    // its own right, and a derived one replays exactly.
                    key: purrdf_gts::wire::blake3_256(b"selected-blob decoy key")
                        .try_into()
                        .expect("a 256-bit digest is 32 bytes"),
                    iv: purrdf_gts::wire::blake3_256(b"selected-blob decoy nonce")[..12]
                        .try_into()
                        .expect("a 256-bit digest is at least 12 bytes"),
                }),
                ..purrdf_gts::writer::FrameOptions::default()
            },
        )
        .expect("encrypted decoy frame");
    let bytes = writer.into_bytes();

    let mut tight = limits();
    tight.max_encoded_bytes = 1_024;
    let error =
        import_gts_events_with_blobs(&bytes, &[GtsBlobSelector::Representation("wanted")], tight)
            .expect_err("an unproved identity must not collapse the candidates");
    assert_eq!(error.code, "rdf-ir-gts-blob-selection", "{error:?}");

    // The refusal records that it never proved what it was.
    let observed = import_gts_events_with_blobs(&bytes, &[], tight).expect("no selectors");
    assert_eq!(observed.refused.len(), 1);
    // The record reports the container's claim faithfully — and marks it as a
    // claim. That flag is the whole difference: the digest here is the retained
    // blob's, and honouring it would have collapsed the two candidates.
    assert_eq!(
        observed.refused[0].digest.as_deref(),
        Some(retained_digest.as_str()),
        "the container did declare the retained blob's digest"
    );
    assert!(
        !observed.refused[0].digest_computed,
        "a payload refused before decoding cannot have a proved digest: {:?}",
        observed.refused[0]
    );
}

/// Declared metadata carries across a segment boundary, as documented.
///
/// The reader's per-occurrence metadata inherits only within a segment, because
/// each segment starts a fresh lookaside. A payload arriving in a later segment
/// than its declaration therefore reaches the collector bare, and matching on
/// that alone refuses a container the authoritative importer accepts — while the
/// same file selected by digest succeeds, proving the bytes were retainable all
/// along.
#[test]
fn declared_metadata_is_inherited_across_segment_boundaries() {
    let data = b"cross segment payload";
    let digest = digest_str(data);

    // Segment 0 declares the representation; segment 1 carries the bytes.
    let mut first = Writer::new("generic");
    first.add_frame("blob", None, None, None, Some(meta(&digest, "wanted")));
    let mut bytes = first.into_bytes();
    let mut second = Writer::new("generic");
    second.add_frame("blob", None, Some(data.to_vec()), None, None);
    bytes.extend(second.into_bytes());

    assert!(
        import_gts_events(&bytes).is_ok(),
        "the authoritative importer accepts this container"
    );

    // Selecting by digest never depended on inheritance, and is the control
    // proving the payload is present and retainable.
    let by_digest =
        import_gts_events_with_blobs(&bytes, &[GtsBlobSelector::Digest(&digest)], limits())
            .expect("digest selection reaches the payload");
    assert_eq!(&*by_digest.blobs[0].bytes, data);

    // Selecting by the representation declared a segment earlier must too.
    let by_rep = import_gts_events_with_blobs(
        &bytes,
        &[GtsBlobSelector::Representation("wanted")],
        limits(),
    )
    .expect("a representation declared in an earlier segment still names it");
    assert_eq!(&*by_rep.blobs[0].bytes, data);
}

/// A refused candidate is judged by the final metadata, like every other.
///
/// Two payloads both tagged `wanted`, one refused by the budget: ambiguous, and
/// must fail. Retag the refused one away before the container ends and exactly
/// one candidate remains, so the same selector must now succeed. Returning the
/// identical error for both files would mean the refusal carries no information.
#[test]
fn a_refused_candidate_retagged_away_stops_being_a_candidate() {
    let small = b"small and wanted";
    let large = vec![b'x'; 50_000];
    let large_digest = digest_str(&large);

    let build = |retag: bool| {
        let mut writer = Writer::new("generic");
        writer.add_blob(small, None, Some("wanted"));
        // Digest-less, so the reader itself refuses it on the encoded ceiling
        // and computes its identity from the untransformed bytes.
        writer.add_frame(
            "blob",
            None,
            Some(large.clone()),
            None,
            Some(Value::Map(vec![("rep".into(), "wanted".into())])),
        );
        if retag {
            writer.add_frame("blob", None, None, None, Some(meta(&large_digest, "other")));
        }
        writer.into_bytes()
    };

    let mut tight = limits();
    tight.max_encoded_bytes = 1_024;

    let ambiguous = build(false);
    assert!(import_gts_events(&ambiguous).is_ok());
    let error = import_gts_events_with_blobs(
        &ambiguous,
        &[GtsBlobSelector::Representation("wanted")],
        tight,
    )
    .expect_err("two payloads still carry the representation");
    assert_eq!(error.code, "rdf-ir-gts-blob-selection", "{error:?}");

    let retagged = build(true);
    assert!(import_gts_events(&retagged).is_ok());
    let result = import_gts_events_with_blobs(
        &retagged,
        &[GtsBlobSelector::Representation("wanted")],
        tight,
    )
    .expect("the refused candidate gave up the representation before the end");
    assert_eq!(&*result.blobs[0].bytes, small);
}

/// A payload the budget refused is present, and is reported as present.
///
/// Letting the ceiling decide whether the importer claims the archive contains
/// a blob would make the answer to "is this blob here?" depend on how much
/// memory the caller offered.
#[test]
fn a_refused_payload_is_not_reported_as_held_elsewhere() {
    let data = vec![b'y'; 50_000];
    let digest = digest_str(&data);

    let mut writer = Writer::new("generic");
    // Metadata-only occurrence first, then the oversized inline payload.
    writer.add_frame("blob", None, None, None, Some(meta(&digest, "wanted")));
    writer.add_frame("blob", None, Some(data.clone()), None, None);
    let bytes = writer.into_bytes();

    let mut tight = limits();
    tight.max_encoded_bytes = 1_024;
    let refused =
        import_gts_events_with_blobs(&bytes, &[GtsBlobSelector::Representation("wanted")], tight)
            .expect_err("the caller named a payload this budget would not admit");
    assert_eq!(refused.code, "rdf-ir-gts-blob-limit", "{refused:?}");
    assert!(
        refused.message.contains("present in this container"),
        "the bytes are inline, not elsewhere: {refused:?}"
    );

    // Neighbouring valid case: a budget that admits it returns the payload.
    let ok = import_gts_events_with_blobs(
        &bytes,
        &[GtsBlobSelector::Representation("wanted")],
        limits(),
    )
    .expect("a budget that admits it returns the payload");
    assert_eq!(&*ok.blobs[0].bytes, &data[..]);
}

/// A refused occurrence's own declaration is what finally names it.
///
/// A refused payload never reaches the payload path, so its declaration would
/// otherwise be missing from the container-global record and an earlier, staler
/// one would answer in its place. That hands the byte ceiling the power to
/// decide what a representation names — the precise defect this whole surface
/// exists to prevent. Both directions are exercised: a ceiling must not invent
/// a unique answer, and must not withhold a real one.
#[test]
fn a_ceiling_never_decides_what_a_representation_finally_names() {
    let large = vec![b'q'; 50_000];
    let large_digest = digest_str(&large);
    let small = b"small and wanted";

    // `first` is a stale earlier declaration for the oversized payload; the
    // payload's own occurrence then declares `own`, which is what counts.
    let build = |first: &str, own: &str| {
        let mut writer = Writer::new("generic");
        writer.add_frame("blob", None, None, None, Some(meta(&large_digest, first)));
        writer.add_frame(
            "blob",
            None,
            Some(large.clone()),
            None,
            Some(Value::Map(vec![("rep".into(), own.into())])),
        );
        writer.add_blob(small, None, Some("wanted"));
        writer.into_bytes()
    };

    let mut tight = limits();
    tight.max_encoded_bytes = 1_024;

    // Two blobs finally carry "wanted": ambiguous under EVERY budget.
    let ambiguous = build("other", "wanted");
    assert!(import_gts_events(&ambiguous).is_ok());
    for (label, budget) in [("tight", tight), ("generous", limits())] {
        let error = import_gts_events_with_blobs(
            &ambiguous,
            &[GtsBlobSelector::Representation("wanted")],
            budget,
        )
        .expect_err("a ceiling must not resolve a real ambiguity");
        assert_eq!(
            error.code, "rdf-ir-gts-blob-selection",
            "{label}: {error:?}"
        );
        assert!(
            error.message.contains("resolves to 2 blobs"),
            "{label}: {error:?}"
        );
    }

    // One blob finally carries "wanted": resolvable under EVERY budget.
    let unique = build("wanted", "other");
    assert!(import_gts_events(&unique).is_ok());
    for (label, budget) in [("tight", tight), ("generous", limits())] {
        let result = import_gts_events_with_blobs(
            &unique,
            &[GtsBlobSelector::Representation("wanted")],
            budget,
        )
        .unwrap_or_else(|error| {
            panic!("{label}: a ceiling must not withhold a unique answer: {error:?}")
        });
        assert_eq!(&*result.blobs[0].bytes, small, "{label}");
    }
}

/// Presence and identity are separate facts, and the importer says which it has.
///
/// A refused frame proves the container carried a payload under some label. That
/// is enough to stop calling the digest "held elsewhere" — the bytes are two
/// frames back. It is not enough to treat those bytes as that digest's, which is
/// why the message distinguishes the two and why deduplication still demands a
/// proved hash. Nothing here may depend on the ceiling the caller chose.
#[test]
fn an_unverified_refusal_is_never_called_held_elsewhere() {
    let large = vec![b'w'; 80_000];
    let small = b"small and wanted";

    // An encrypted payload is refused before decryption, so its identity is the
    // container's claim. Its own later declaration retags it away from "wanted".
    let key: [u8; 32] = purrdf_gts::wire::blake3_256(b"unverified refusal key")
        .try_into()
        .expect("a 256-bit digest is 32 bytes");
    let iv: [u8; 12] = purrdf_gts::wire::blake3_256(b"unverified refusal nonce")[..12]
        .try_into()
        .expect("a 256-bit digest is at least 12 bytes");
    let claimed = digest_str(&large);

    let mut writer = Writer::new("generic");
    writer.add_frame("blob", None, None, None, Some(meta(&claimed, "wanted")));
    writer
        .add_frame_with_options(
            "blob",
            purrdf_gts::writer::FrameOptions {
                raw: Some(large),
                pub_meta: Some(meta(&claimed, "other")),
                encrypt: Some(purrdf_gts::writer::Encrypt0Options {
                    kid: "recipient".into(),
                    key,
                    iv,
                }),
                ..purrdf_gts::writer::FrameOptions::default()
            },
        )
        .expect("encrypted oversized frame");
    writer.add_blob(small, None, Some("wanted"));
    let bytes = writer.into_bytes();

    let mut tight = limits();
    tight.max_encoded_bytes = 1_024;

    // The encrypted occurrence claims to BE that digest and declares it "other",
    // but nothing verified the claim. An unproved claim may not rename a digest
    // the container declared, so the earlier declaration stands, the selector
    // still reaches a payload this import could not produce, and it fails closed
    // rather than quietly handing back the other blob.
    let ambiguous =
        import_gts_events_with_blobs(&bytes, &[GtsBlobSelector::Representation("wanted")], tight)
            .expect_err("an unverified claim cannot retag a declared digest");
    assert_eq!(ambiguous.code, "rdf-ir-gts-blob-limit", "{ambiguous:?}");

    // Neighbouring valid case: the same shape with the retag PROVED. An
    // untransformed digest-less payload is hashed by the reader, so its own
    // declaration does bind, and the selector reaches the remaining blob.
    let proved_payload = vec![b'p'; 80_000];
    let mut writer = Writer::new("generic");
    writer.add_frame(
        "blob",
        None,
        None,
        None,
        Some(meta(&digest_str(&proved_payload), "wanted")),
    );
    writer.add_frame(
        "blob",
        None,
        Some(proved_payload),
        None,
        Some(Value::Map(vec![("rep".into(), "other".into())])),
    );
    writer.add_blob(small, None, Some("wanted"));
    let resolved = import_gts_events_with_blobs(
        &writer.into_bytes(),
        &[GtsBlobSelector::Representation("wanted")],
        tight,
    )
    .expect("a proved identity may retag itself away from this representation");
    assert_eq!(&*resolved.blobs[0].bytes, small);

    // Naming the refused digest reports the budget, never "held elsewhere".
    let refused = import_gts_events_with_blobs(&bytes, &[GtsBlobSelector::Digest(&claimed)], tight)
        .expect_err("the caller named a payload this budget would not admit");
    assert_eq!(refused.code, "rdf-ir-gts-blob-limit", "{refused:?}");
    assert!(
        !refused.message.contains("held elsewhere"),
        "the bytes are inline, two frames back: {refused:?}"
    );
    assert!(
        refused.message.contains("not verified"),
        "an unproved identity must say so: {refused:?}"
    );
}

/// The metadata ceiling binds the refusal path too.
///
/// A refused declaration is retained and handed back like any other, so a bound
/// that only covered the payload path would be a ceiling a refused frame walks
/// straight past — once per refused frame, with container-chosen bytes.
#[test]
fn a_refused_declaration_obeys_the_metadata_ceiling() {
    let build = |filler: usize| {
        let mut writer = Writer::new("generic");
        writer.add_frame(
            "blob",
            None,
            Some(vec![b'v'; 50_000]),
            None,
            Some(Value::Map(vec![
                ("rep".into(), "wanted".into()),
                ("note".into(), Value::Text("z".repeat(filler))),
            ])),
        );
        writer.into_bytes()
    };

    let mut tight = limits();
    tight.max_encoded_bytes = 1_024;
    tight.max_metadata_bytes = 64 * 1024;

    // Over the ceiling: the declaration is dropped, and the refusal says so.
    let over = import_gts_events_with_blobs(&build(400_000), &[], tight).expect("import succeeds");
    assert_eq!(over.refused.len(), 1);
    assert!(
        over.refused[0].metadata.is_none(),
        "an over-budget declaration is not retained: {:?}",
        over.refused[0]
    );
    assert!(
        over.refused[0].detail.contains("retention budgets"),
        "and the reason is stated: {:?}",
        over.refused[0]
    );

    // Neighbouring valid case: a declaration within the ceiling is kept intact.
    let under = import_gts_events_with_blobs(&build(1_024), &[], tight).expect("import succeeds");
    assert_eq!(under.refused.len(), 1);
    assert!(
        under.refused[0].metadata.is_some(),
        "an in-budget declaration is retained: {:?}",
        under.refused[0]
    );
}

/// A retention ceiling never changes how many blobs a selector counts.
///
/// The metadata ceiling governs how much of a blob's description this import
/// keeps. It must not govern whether the blob is a candidate: a caller who
/// lowers a memory budget would otherwise receive one arbitrary payload where
/// two match, and the default ceiling would decide it silently.
#[test]
fn the_metadata_ceiling_does_not_decide_which_blob_a_selector_finds() {
    let bulky = |rep: &str, filler: usize| {
        Value::Map(vec![
            ("rep".into(), rep.into()),
            ("note".into(), Value::Text("z".repeat(filler))),
        ])
    };

    // Two payloads both finally carry "wanted"; one has a huge description.
    let mut writer = Writer::new("generic");
    writer.add_blob(b"small and wanted", None, Some("wanted"));
    writer.add_frame(
        "blob",
        None,
        Some(vec![b'u'; 80_000]),
        None,
        Some(bulky("wanted", 400_000)),
    );
    let bytes = writer.into_bytes();

    let mut tight = limits();
    tight.max_encoded_bytes = 1_024;

    // The shipped default is 64 KiB, well under this description.
    for (label, metadata_ceiling) in [("default", DEFAULT_MAX_METADATA_BYTES), ("roomy", 1 << 20)] {
        let mut budget = tight;
        budget.max_metadata_bytes = metadata_ceiling;
        let error = import_gts_events_with_blobs(
            &bytes,
            &[GtsBlobSelector::Representation("wanted")],
            budget,
        )
        .expect_err("two payloads finally carry this representation");
        assert_eq!(
            error.code, "rdf-ir-gts-blob-selection",
            "{label}: {error:?}"
        );
        assert!(
            error.message.contains("resolves to 2 blobs"),
            "{label}: the count must not depend on the metadata ceiling: {error:?}"
        );
    }
}

/// The selection record keeps names, not descriptions.
///
/// What a blob is *called* has to survive a ceiling that discards how it is
/// *described*, and it has to do so without becoming a second unbounded copy of
/// the metadata. A value longer than any selector cannot match one, so nothing
/// larger than a selector is ever kept for matching.
#[test]
fn an_oversized_description_is_neither_retained_nor_lost_for_matching() {
    let mut writer = Writer::new("generic");
    writer.add_frame(
        "blob",
        None,
        Some(vec![b'u'; 80_000]),
        None,
        Some(Value::Map(vec![
            ("rep".into(), "wanted".into()),
            ("note".into(), Value::Text("z".repeat(400_000))),
        ])),
    );
    let bytes = writer.into_bytes();

    let mut budget = limits();
    budget.max_encoded_bytes = 1_024;
    budget.max_metadata_bytes = DEFAULT_MAX_METADATA_BYTES;

    // The description is dropped from what the caller gets back...
    let observed = import_gts_events_with_blobs(&bytes, &[], budget).expect("import succeeds");
    assert_eq!(observed.refused.len(), 1);
    assert!(
        observed.refused[0].metadata.is_none(),
        "the over-budget description is not retained: {:?}",
        observed.refused[0]
    );

    // ...and the name still names it.
    let error =
        import_gts_events_with_blobs(&bytes, &[GtsBlobSelector::Representation("wanted")], budget)
            .expect_err("the caller named a payload this budget would not admit");
    assert_eq!(error.code, "rdf-ir-gts-blob-limit", "{error:?}");

    // A representation longer than any selector cannot match one, so it is not
    // kept for matching either — the record holds names, never bulk.
    let mut writer = Writer::new("generic");
    writer.add_blob(b"unreachable", None, Some(&"y".repeat(8192)));
    let unreachable = import_gts_events_with_blobs(
        &writer.into_bytes(),
        &[GtsBlobSelector::Representation("wanted")],
        limits(),
    )
    .expect_err("no selector can carry that value");
    assert_eq!(
        unreachable.code, "rdf-ir-gts-blob-selection",
        "{unreachable:?}"
    );
}

/// The selection projection keeps exactly what a selector can carry.
///
/// Discarding a longer representation is only lossless if the two bounds agree
/// on the same unit and the same boundary. They are both byte lengths, and both
/// admit a value of exactly the maximum, so the longest nameable representation
/// still matches and the shortest unnameable one could never have matched.
#[test]
fn the_longest_nameable_representation_still_matches() {
    let longest = "n".repeat(4096);
    let mut writer = Writer::new("generic");
    writer.add_blob(b"at the boundary", None, Some(&longest));
    let result = import_gts_events_with_blobs(
        &writer.into_bytes(),
        &[GtsBlobSelector::Representation(&longest)],
        limits(),
    )
    .expect("a representation of exactly the maximum length is nameable");
    assert_eq!(&*result.blobs[0].bytes, b"at the boundary");

    // One byte further, the selector itself is refused at construction, so no
    // blob could have matched it and dropping such a representation loses
    // nothing that was ever reachable.
    let beyond = "n".repeat(4097);
    let mut writer = Writer::new("generic");
    writer.add_blob(b"unreachable", None, Some(&beyond));
    let error = import_gts_events_with_blobs(
        &writer.into_bytes(),
        &[GtsBlobSelector::Representation(&beyond)],
        limits(),
    )
    .expect_err("no selector may carry more than the maximum");
    assert_eq!(error.code, "rdf-ir-gts-blob-selection", "{error:?}");
}

/// A declaration reaches a payload in a later segment, whole and attributed.
///
/// The reader's own inheritance resets at every segment boundary, so a bare
/// occurrence arrives with nothing. The documented contract says absent metadata
/// preserves the previous declaration across those boundaries — which means the
/// caller must get the declaration's full content, not merely be able to select
/// on it, and must be told which frame declared it.
#[test]
fn a_later_segments_payload_inherits_the_whole_declaration() {
    let data = b"cross segment body";
    let digest = digest_str(data);

    let mut first = Writer::new("generic");
    first.add_frame(
        "blob",
        None,
        None,
        None,
        Some(Value::Map(vec![
            ("digest".into(), digest.into()),
            ("rep".into(), "wanted".into()),
            ("mt".into(), "image/png".into()),
        ])),
    );
    let first_head = first.head().to_vec();
    let mut bytes = first.into_bytes();
    let mut second = Writer::new("generic");
    second.add_frame("blob", None, Some(data.to_vec()), None, None);
    bytes.extend(second.into_bytes());

    assert!(import_gts_events(&bytes).is_ok());
    let result = import_gts_events_with_blobs(
        &bytes,
        &[GtsBlobSelector::Representation("wanted")],
        limits(),
    )
    .expect("a representation declared a segment earlier still names it");
    let blob = &result.blobs[0];
    assert_eq!(&*blob.bytes, data);

    // Selected BY the representation, so it must not come back claiming to have
    // none — and the media type declared alongside it must survive too.
    let metadata = blob
        .metadata
        .as_ref()
        .unwrap_or_else(|| panic!("the inherited declaration is returned: {blob:?}"));
    let Value::Map(fields) = metadata else {
        panic!("public metadata is a map: {metadata:?}");
    };
    let field = |name: &str| {
        fields
            .iter()
            .find(|(key, _)| key.as_text() == Some(name))
            .and_then(|(_, value)| value.as_text())
            .map(ToString::to_string)
    };
    assert_eq!(field("rep").as_deref(), Some("wanted"));
    assert_eq!(field("mt").as_deref(), Some("image/png"));

    // Attribution points at the frame that declared it, not the one carrying
    // the bytes, and carries that segment's verified head.
    let source = blob
        .metadata_source
        .as_ref()
        .unwrap_or_else(|| panic!("the declaration's source is returned: {blob:?}"));
    assert_eq!(source.segment_index, 0);
    assert_eq!(blob.segment_index, 1);
    assert_eq!(source.segment_head, first_head);
}

/// A first declaration over the metadata ceiling is refused; under it, admitted.
///
/// This does NOT isolate where the bound fires. The importer copies public
/// metadata into its lookaside before any payload event, and the pre-copy gate
/// exists to bound it there rather than afterwards — but both orderings refuse
/// the same container with the same code, so the difference is peak allocation
/// and is not observable through this API. What is pinned here is the refusal
/// itself and its neighbouring acceptance.
#[test]
fn a_first_declaration_over_the_metadata_ceiling_is_refused() {
    let build = |filler: usize| {
        let mut writer = Writer::new("generic");
        writer.add_frame(
            "blob",
            None,
            Some(b"body".to_vec()),
            None,
            Some(Value::Map(vec![
                ("rep".into(), "wanted".into()),
                ("note".into(), Value::Text("z".repeat(filler))),
            ])),
        );
        writer.into_bytes()
    };

    let mut tight = limits();
    tight.max_metadata_bytes = DEFAULT_MAX_METADATA_BYTES;
    let error = import_gts_events_with_blobs(
        &build(400_000),
        &[GtsBlobSelector::Representation("wanted")],
        tight,
    )
    .expect_err("an over-budget declaration is refused before it is copied");
    assert_eq!(error.code, "rdf-ir-gts-blob-limit", "{error:?}");

    // Neighbouring valid case: the same shape within the ceiling still imports.
    let ok = import_gts_events_with_blobs(
        &build(1_024),
        &[GtsBlobSelector::Representation("wanted")],
        tight,
    )
    .expect("an in-budget declaration is admitted");
    assert_eq!(&*ok.blobs[0].bytes, b"body");
}

/// "External" is only claimed when nothing unidentified was refused.
///
/// Saying a payload is held elsewhere is a factual claim about the archive. A
/// payload refused before it could be identified might be those very bytes, so
/// the claim is not available; the truthful verdict is the budget's.
#[test]
fn externality_is_not_asserted_over_an_unidentified_refusal() {
    let absent = digest_str(b"genuinely somewhere else");

    // Nothing unidentified was refused, so externality is assertable.
    let mut writer = Writer::new("generic");
    writer.add_frame("blob", None, None, None, Some(meta(&absent, "wanted")));
    let external = import_gts_events_with_blobs(
        &writer.into_bytes(),
        &[GtsBlobSelector::Representation("wanted")],
        limits(),
    )
    .expect_err("a blob with no payload anywhere is external");
    assert_eq!(external.code, "rdf-ir-gts-blob-external", "{external:?}");

    // Same selection, but an encrypted frame was refused before identification.
    // Those bytes could be the ones named, so the claim is withheld.
    let key: [u8; 32] = purrdf_gts::wire::blake3_256(b"externality probe key")
        .try_into()
        .expect("a 256-bit digest is 32 bytes");
    let iv: [u8; 12] = purrdf_gts::wire::blake3_256(b"externality probe nonce")[..12]
        .try_into()
        .expect("a 256-bit digest is at least 12 bytes");
    let mut writer = Writer::new("generic");
    writer.add_frame("blob", None, None, None, Some(meta(&absent, "wanted")));
    writer
        .add_frame_with_options(
            "blob",
            purrdf_gts::writer::FrameOptions {
                raw: Some(vec![b'e'; 80_000]),
                encrypt: Some(purrdf_gts::writer::Encrypt0Options {
                    kid: "recipient".into(),
                    key,
                    iv,
                }),
                ..purrdf_gts::writer::FrameOptions::default()
            },
        )
        .expect("encrypted oversized frame");
    let mut tight = limits();
    tight.max_encoded_bytes = 1_024;
    let withheld = import_gts_events_with_blobs(
        &writer.into_bytes(),
        &[GtsBlobSelector::Representation("wanted")],
        tight,
    )
    .expect_err("the selection still fails closed");
    assert_eq!(withheld.code, "rdf-ir-gts-blob-limit", "{withheld:?}");
    assert!(
        !withheld.message.contains("held elsewhere"),
        "externality is not assertable here: {withheld:?}"
    );
}
