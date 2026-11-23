// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! GTS v2 checks for stored bytes, bounded decoding and decoded payloads.

use purrdf_gts::v2::{
    FrameCheck, FrameField, Header, Item, LinkState, MalformedReason, PayloadBudget,
    PayloadCapacity, PayloadDamage, PayloadRefusal, PayloadState, StoredState,
};

const STANDARD_CAPACITY: PayloadCapacity = PayloadCapacity {
    stored_bytes_per_frame: 1 << 26,
    decoded_bytes_per_frame: 1 << 26,
    decoded_bytes_per_read: 1 << 36,
};

fn first(bytes: &[u8]) -> Item<'_> {
    Item::first(bytes).unwrap().unwrap()
}

fn check_single(bytes: &[u8], capacity: PayloadCapacity) -> FrameCheck<'_> {
    let opener = first(bytes);
    let header = Header::decode(opener).unwrap();
    let tail = &bytes[opener.bytes().len()..];
    let item = first(tail);
    assert_eq!(item.bytes().len(), tail.len());
    item.frame()
        .unwrap()
        .unwrap()
        .check(
            &header.identifier().unwrap(),
            None,
            header.registry_version(),
            &mut PayloadBudget::new(capacity),
        )
        .unwrap()
}

#[test]
fn compressed_payload_requires_the_generated_header_and_exactly_one_frame() {
    let valid = check_single(
        include_bytes!("fixtures/v2/P1/zstd-ok.bin"),
        STANDARD_CAPACITY,
    )
    .payload;
    assert_eq!(valid.stored, Some(StoredState::Matches));
    assert_eq!(valid.malformed, None);
    let PayloadState::Verified(decoded) = valid.state else {
        panic!("expected the verified 300-byte payload");
    };
    assert_eq!(decoded.len(), 300);

    let checksum = check_single(
        include_bytes!("fixtures/v2/P1/zstd-header-differs.checksum-flag.bin"),
        STANDARD_CAPACITY,
    )
    .payload;
    assert_eq!(checksum.stored, Some(StoredState::Matches));
    assert_eq!(
        checksum.malformed.unwrap().reason,
        MalformedReason::CodecConstraint
    );
    assert!(matches!(
        checksum.state,
        PayloadState::Damaged(PayloadDamage::Malformed)
    ));

    let concatenated = check_single(
        include_bytes!("fixtures/v2/P1/zstd-two-frames.bin"),
        STANDARD_CAPACITY,
    )
    .payload;
    assert_eq!(concatenated.stored, Some(StoredState::Matches));
    assert_eq!(concatenated.malformed, None);
    assert!(matches!(
        concatenated.state,
        PayloadState::Damaged(PayloadDamage::TrailingCompressedBytes)
    ));

    let short = check_single(
        include_bytes!("fixtures/v2/P1/zstd-output-short.bin"),
        STANDARD_CAPACITY,
    )
    .payload;
    assert_eq!(short.stored, Some(StoredState::Matches));
    assert_eq!(short.malformed, None);
    assert!(matches!(
        short.state,
        PayloadState::Damaged(PayloadDamage::DecodeFailed(_))
    ));
}

#[test]
fn stored_check_runs_for_unchecked_payloads() {
    let capacity = PayloadCapacity {
        stored_bytes_per_frame: 50,
        ..STANDARD_CAPACITY
    };
    let oversized = check_single(
        include_bytes!("fixtures/v2/P1/stored-above-capacity.bin"),
        capacity,
    )
    .payload;
    assert_eq!(oversized.stored, Some(StoredState::Matches));
    assert!(matches!(
        oversized.state,
        PayloadState::Unchecked(PayloadRefusal::StoredSize(276))
    ));

    let unknown = check_single(
        include_bytes!("fixtures/v2/P1/payload-unknown-codec.bin"),
        STANDARD_CAPACITY,
    )
    .payload;
    assert_eq!(unknown.stored, Some(StoredState::Matches));
    assert!(matches!(
        unknown.state,
        PayloadState::Unchecked(PayloadRefusal::UnknownCodec(2))
    ));
}

#[test]
fn total_decoded_refusal_sticks_and_counts_only_admitted_lengths() {
    let bytes = include_bytes!("fixtures/v2/P1/payload-total-after-refusal.bin");
    let opener = first(bytes);
    let header = Header::decode(opener).unwrap();
    let mut predecessor = header.identifier().unwrap();
    let mut budget = PayloadBudget::new(PayloadCapacity {
        decoded_bytes_per_read: 8,
        ..STANDARD_CAPACITY
    });
    let mut tail = &bytes[opener.bytes().len()..];
    for expected in [None, None, Some(9), Some(8)] {
        let item = first(tail);
        tail = &tail[item.bytes().len()..];
        let checked = item
            .frame()
            .unwrap()
            .unwrap()
            .check(&predecessor, None, header.registry_version(), &mut budget)
            .unwrap();
        predecessor = checked.link.effective_identifier.unwrap();
        assert_eq!(checked.link.state, LinkState::Linked);
        assert_eq!(checked.malformed, None);
        let payload = checked.payload;
        assert_eq!(payload.stored, None);
        assert_eq!(payload.malformed, None);
        match expected {
            None => assert!(matches!(payload.state, PayloadState::Verified(_))),
            Some(count) => assert!(matches!(
                payload.state,
                PayloadState::Unchecked(PayloadRefusal::TotalDecoded(actual)) if actual == count
            )),
        }
    }
    assert_eq!(tail, []);
    assert_eq!(budget.charged(), 8);
}

#[test]
fn zero_compressed_length_precedes_the_type_of_the_payload_field() {
    let bytes = include_bytes!("fixtures/v2/P1/link-ok.bin");
    let opener = first(bytes);
    let item = first(&bytes[opener.bytes().len()..]);
    let envelope = item.frame().unwrap().unwrap();
    let suffix =
        envelope.field(FrameField::Identifier).len() + envelope.field(FrameField::Signatures).len();
    let storage_offset = item.bytes().len()
        - suffix
        - envelope.field(FrameField::Storage).len()
        - envelope.field(FrameField::Payload).len();
    let mut altered = bytes[..opener.bytes().len() + storage_offset].to_vec();
    altered.extend_from_slice(&[0x83, 2, 1, 0x50]);
    altered.extend_from_slice(&[0; 16]);
    altered.push(0xf4);
    altered.extend_from_slice(&item.bytes()[item.bytes().len() - suffix..]);

    let valid = check_single(
        &bytes[..opener.bytes().len() + item.bytes().len()],
        STANDARD_CAPACITY,
    );
    assert_eq!(valid.link.state, LinkState::Linked);
    assert!(matches!(valid.payload.state, PayloadState::Verified(_)));

    let checked = check_single(&altered, STANDARD_CAPACITY);
    assert!(matches!(
        checked.payload.state,
        PayloadState::Damaged(PayloadDamage::Malformed)
    ));
    let error = checked.malformed.unwrap();
    assert_eq!(error.reason, MalformedReason::ForbiddenCombination);
    assert_eq!(error.offset, storage_offset);
}

#[test]
fn envelope_diagnoses_flag_agreement_and_signature_order() {
    let flags = check_single(
        include_bytes!("fixtures/v2/P1/frame-flags-order.bin"),
        STANDARD_CAPACITY,
    );
    assert_eq!(flags.link.state, LinkState::Broken);
    assert_eq!(
        flags.link.identity_error.unwrap().reason,
        MalformedReason::WrongType
    );
    assert_eq!(
        flags.malformed.unwrap().reason,
        MalformedReason::ForbiddenCombination
    );

    let signatures = check_single(
        include_bytes!("fixtures/v2/P1/frame-signatures-unordered.bin"),
        STANDARD_CAPACITY,
    );
    assert_eq!(signatures.link.state, LinkState::Linked);
    assert!(matches!(
        signatures.payload.state,
        PayloadState::Verified(_)
    ));
    assert_eq!(
        signatures.malformed.unwrap().reason,
        MalformedReason::UnorderedOrRepeated
    );
}

#[test]
fn signature_grammar_has_priority_over_the_later_codec_check() {
    let bytes = include_bytes!("fixtures/v2/P1/zstd-header-differs.checksum-flag.bin");
    let original = check_single(bytes, STANDARD_CAPACITY);
    assert_eq!(
        original.malformed.unwrap().reason,
        MalformedReason::CodecConstraint
    );

    let opener = first(bytes);
    let frame = first(&bytes[opener.bytes().len()..])
        .frame()
        .unwrap()
        .unwrap();
    let signatures = frame.field(FrameField::Signatures);
    let mut altered = bytes[..bytes.len() - signatures.len()].to_vec();
    altered.push(0xf4);
    let checked = check_single(&altered, STANDARD_CAPACITY);
    assert_eq!(checked.link.state, LinkState::Linked);
    assert_eq!(
        checked.malformed.unwrap().reason,
        MalformedReason::WrongType
    );
    assert_eq!(
        checked.payload.malformed.unwrap().reason,
        MalformedReason::CodecConstraint
    );
}
