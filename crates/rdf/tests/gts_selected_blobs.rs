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
