// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! GTS v2 item framing, classification, grammar and deterministic headers.

use core::num::NonZeroU64;

use purrdf_gts::v2::{
    CanonicalForm, FrameField, Generation, Header, Item, ItemKind, LinkState, MalformedReason,
    PayloadBudget, PayloadCapacity, ResyncScan, SegmentVerifier, UnknownFrameType, write_header,
};
use purrdf_lex::cbor::DecodeErrorKind;

const MINIMAL: &[u8] = include_bytes!("fixtures/v2/P1/header-minimal.bin");
const LINK_OK: &[u8] = include_bytes!("fixtures/v2/P1/link-ok.bin");
const RESYNC_CHAIN: &[u8] = include_bytes!("fixtures/v2/P1/resync-chain.with-frames.bin");

fn scan_budget(total: u64) -> PayloadBudget {
    PayloadBudget::new(PayloadCapacity {
        stored_bytes_per_frame: 0,
        decoded_bytes_per_frame: 0,
        decoded_bytes_per_read: total,
    })
}

#[test]
fn resync_scan_bounds_candidate_starts_and_skips_tested_occurrences() {
    let mut scan = ResyncScan::new(RESYNC_CHAIN);
    let mut budget = PayloadBudget::new(PayloadCapacity {
        stored_bytes_per_frame: 64,
        decoded_bytes_per_frame: 17,
        decoded_bytes_per_read: 4127,
    });
    let opener = first(RESYNC_CHAIN);
    let header = Header::decode(opener).unwrap();
    let mut verifier =
        SegmentVerifier::new(header.identifier().unwrap(), header.registry_version());
    let mut rest = &RESYNC_CHAIN[opener.bytes().len()..209];
    while !rest.is_empty() {
        let item = first(rest);
        verifier
            .check(&item.frame().unwrap().unwrap(), None, &mut budget)
            .unwrap();
        rest = &rest[item.bytes().len()..];
    }
    assert_eq!(budget.charged(), 31);
    assert!(scan.scan(0..209, &mut budget).unwrap().is_none());
    assert_eq!(budget.charged(), 31);

    // The candidate starts at the last byte in the run and crosses its end.
    let found = scan.scan(200..210, &mut budget).unwrap().unwrap();
    assert_eq!(found.offset, 209);
    assert_eq!(found.bytes, first(&RESYNC_CHAIN[209..]).bytes());
    assert_eq!(budget.charged(), 4127);
    assert!(
        scan.scan(208..RESYNC_CHAIN.len(), &mut budget)
            .unwrap()
            .is_none()
    );
}

#[test]
fn resync_scan_checks_place_and_held_hash_before_charging() {
    let later_hash = include_bytes!("fixtures/v2/P1/resync-candidate-hash-later-version.bin");
    let profile =
        include_bytes!("fixtures/v2/P1/resync-candidate-option-rule.profile-tag-next-charged.bin");
    let mut displaced = vec![0];
    displaced.extend_from_slice(RESYNC_CHAIN);
    for bytes in [later_hash.as_slice(), &displaced] {
        let mut budget = scan_budget(0);
        assert!(
            ResyncScan::new(bytes)
                .scan(0..bytes.len(), &mut budget)
                .unwrap()
                .is_none()
        );
        assert_eq!(budget.charged(), 0);
    }

    // The invalid profile is cost-free. Only the next occurrence reaches the charge.
    let mut scan = ResyncScan::new(profile);
    let mut budget = scan_budget(4095);
    let refused = scan.scan(0..profile.len(), &mut budget).unwrap_err();
    assert_eq!((refused.offset, refused.requested_total), (473, 4096));
    assert!(scan.scan(0..profile.len(), &mut budget).unwrap().is_none());
    assert_eq!(budget.charged(), 0);
}

#[test]
fn resync_scan_charges_before_checking_mark_order_and_identifier() {
    let marks = include_bytes!("fixtures/v2/P1/resync-candidate-marks-unordered.whole.bin");
    let mut wrong_identifier = RESYNC_CHAIN.to_vec();
    let opener = first(&RESYNC_CHAIN[209..]);
    wrong_identifier[209 + opener.bytes().len() - 1] ^= 1;
    for bytes in [marks.as_slice(), &wrong_identifier] {
        let mut budget = scan_budget(4096);
        assert!(
            ResyncScan::new(bytes)
                .scan(0..bytes.len(), &mut budget)
                .unwrap()
                .is_none()
        );
        assert_eq!(budget.charged(), 4096);
    }
}

fn first(bytes: &[u8]) -> Item<'_> {
    Item::first(bytes)
        .expect("well-formed fixture framing")
        .expect("fixture has an item")
}

#[test]
fn framing_test_checks_shape_before_content_values_or_codecs() {
    let bytes = include_bytes!("fixtures/v2/P1/framing-test-content-value.bin");
    let opener = first(bytes);
    let header = Header::decode(opener).unwrap();
    let mut head = header.identifier().unwrap();
    let mut tail = &bytes[opener.bytes().len()..];
    while !tail.is_empty() {
        let item = first(tail);
        assert!(
            item.passes_framing_test(Some((header.registry_version(), &head)), None)
                .unwrap()
        );
        let link = item.frame().unwrap().unwrap().check_link(&head, None);
        head = link.effective_identifier.unwrap();
        tail = &tail[item.bytes().len()..];
    }

    let bytes = include_bytes!("fixtures/v2/P1/zstd-header-differs.checksum-flag.bin");
    let opener = first(bytes);
    let header = Header::decode(opener).unwrap();
    let frame = first(&bytes[opener.bytes().len()..]);
    assert!(
        frame
            .passes_framing_test(
                Some((header.registry_version(), &header.identifier().unwrap())),
                None
            )
            .unwrap()
    );
}

#[test]
fn framing_test_checks_closing_item_grammar_and_version_one_classification() {
    let link = Some((NonZeroU64::MIN, &[0; 32]));
    let newer = Some((NonZeroU64::new(2).unwrap(), &[0; 32]));
    let unknown = first(&[0x81, 6]);
    assert!(!unknown.passes_framing_test(link, None).unwrap());
    assert!(unknown.passes_framing_test(newer, None).unwrap());
    assert!(unknown.passes_framing_test(None, None).unwrap());

    let locator = first(&[0x82, 3, 0x1b, 0, 0, 0, 0, 0, 0, 0, 0]);
    assert!(locator.passes_framing_test(link, None).unwrap());
    let short_locator = first(&[0x82, 3, 0]);
    assert!(!short_locator.passes_framing_test(link, None).unwrap());
    assert!(short_locator.passes_framing_test(None, None).unwrap());
    let offsets = first(&[0x82, 4, 0x81, 0]);
    assert!(offsets.passes_framing_test(link, None).unwrap());
    let ill_typed_offsets = first(&[0x82, 4, 0x81, 0xf4]);
    assert!(!ill_typed_offsets.passes_framing_test(link, None).unwrap());
}

#[test]
fn header_bytes_and_identifier_match_the_v2_corpus() {
    let mut bytes = Vec::new();
    write_header(&mut bytes, None, Some(CanonicalForm::V1), None).expect("write a header");
    assert_eq!(bytes, MINIMAL);
    let item = first(&bytes);
    assert_eq!(item.generation().unwrap(), Generation::Second);

    // P1 §10.2 gives the identity for the original 12-byte header.
    let expected = purrdf_hash::hex::decode_32(
        "6c19be0fed72079771f9c8e0006a1f2ef9050d710ba4ff5b5f8ac044ddf5acc0",
    );
    assert_eq!(Header::decode(item).unwrap().identifier(), expected);
}

#[test]
fn file_identification_preserves_version_grammar_and_first_key_order() {
    let cases = [
        (MINIMAL, Generation::Second),
        (
            include_bytes!("fixtures/v2/P9/reader__first-and-second.bin").as_slice(),
            Generation::First,
        ),
        (
            include_bytes!("fixtures/v2/P9/reader__version-two-map.bin").as_slice(),
            Generation::Unsupported(2),
        ),
        (
            include_bytes!("fixtures/v2/P1/gen-first-generation-v-twice.bin").as_slice(),
            Generation::Unrecognized,
        ),
        // The first legacy key is indefinite, with an empty chunk before v.
        (
            &[0xa1, 0x7f, 0x60, 0x61, b'v', 0xff, 0x01],
            Generation::First,
        ),
        (&[0x82, 0x01, 0x01], Generation::Unrecognized),
        (&[0x82, 0x05, 0x03], Generation::Unsupported(3)),
    ];
    for (bytes, expected) in cases {
        assert_eq!(first(bytes).generation().unwrap(), expected, "{bytes:02x?}");
    }
}

#[test]
fn framing_and_generation_identification_do_not_require_shortest_heads() {
    for bytes in [
        include_bytes!("fixtures/v2/P1/header-long-form-first-item.bin").as_slice(),
        &[0x9f, 1, 2, 0xff],
    ] {
        let item = first(bytes);
        assert_eq!(item.generation().unwrap(), Generation::Second);
        assert_eq!(
            Header::decode(item).unwrap_err().reason,
            MalformedReason::NotDeterministic
        );
    }
    assert!(Header::decode(first(MINIMAL)).is_ok());

    // The format does not fix a nesting limit for its framing walk.
    let mut deep = vec![0x81; 300];
    deep.push(0);
    assert_eq!(first(&deep).bytes(), deep);
    first(&deep).validate_grammar().unwrap();
}

#[test]
fn framing_distinguishes_clean_end_torn_items_and_bad_heads() {
    assert!(Item::first(&[]).unwrap().is_none());
    assert!(matches!(
        Item::first(&MINIMAL[..MINIMAL.len() - 1])
            .unwrap_err()
            .kind(),
        DecodeErrorKind::Truncated
    ));
    assert!(matches!(
        Item::first(&[0x81, 0xff]).unwrap_err().kind(),
        DecodeErrorKind::Malformed(_)
    ));
    assert_eq!(first(MINIMAL).bytes(), MINIMAL);
}

#[test]
fn segment_classification_keeps_malformed_openers_and_complete_envelopes() {
    let cases: &[(&[u8], Result<ItemKind, MalformedReason>)] = &[
        (&[0x9f, 1, 0xff], Ok(ItemKind::Header)),
        (&[0x81, 0x18, 5], Ok(ItemKind::Resync)),
        (&[0x9f, 2, 0xff], Err(MalformedReason::NotDeterministic)),
        (&[0x81, 0x18, 2], Err(MalformedReason::NotDeterministic)),
        (&[0x81, 2], Err(MalformedReason::WrongLength)),
        (
            &[0x8b, 2, 0xf6, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            Ok(ItemKind::Frame),
        ),
        (
            &[0x98, 11, 2, 0xf6, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            Err(MalformedReason::NotDeterministic),
        ),
        (&[0x81, 3], Ok(ItemKind::Locator)),
        (&[0x81, 4], Ok(ItemKind::Offsets)),
        (&[0x80], Err(MalformedReason::WrongType)),
        (&[0x81, 0], Err(MalformedReason::UnregisteredValue)),
        (&[0x81, 6], Err(MalformedReason::UnregisteredValue)),
    ];
    for &(bytes, expected) in cases {
        assert_eq!(
            first(bytes).classify(NonZeroU64::MIN),
            expected,
            "{bytes:02x?}"
        );
    }
    let newer_registry = NonZeroU64::new(2).unwrap();
    assert_eq!(
        first(&[0x81, 6]).classify(newer_registry),
        Ok(ItemKind::Unknown(6))
    );
    assert_eq!(
        first(&[0x81, 0]).classify(newer_registry),
        Err(MalformedReason::UnregisteredValue)
    );
}

#[test]
fn invalid_utf8_in_unknown_frame_content_does_not_lose_framing() {
    let bytes = include_bytes!("fixtures/v2/P1/frame-unknown-content-bad-utf8.bin");
    let opener = first(bytes);
    let header = Header::decode(opener).unwrap();
    let frame = first(&bytes[opener.bytes().len()..]);
    assert_eq!(
        frame.classify(header.registry_version()),
        Ok(ItemKind::Frame)
    );
    let envelope = frame.frame().unwrap().unwrap();
    assert_eq!(envelope.field(FrameField::Content), [0x62, 0xc3, 0x28]);
    assert_eq!(envelope.identity_fields(), &frame.bytes()[2..44]);
    let error = frame.validate_grammar().unwrap_err();
    assert_eq!(error.reason, MalformedReason::WrongType);
    assert_eq!(error.offset, 6);
    let checked = envelope
        .check(
            &header.identifier().unwrap(),
            None,
            header.registry_version(),
            &mut PayloadBudget::new(PayloadCapacity {
                stored_bytes_per_frame: 1 << 26,
                decoded_bytes_per_frame: 1 << 26,
                decoded_bytes_per_read: 1 << 36,
            }),
        )
        .unwrap();
    assert_eq!(
        checked.unknown_type,
        Some(UnknownFrameType::Noncritical(99))
    );
    assert_eq!(checked.malformed, Some(error));
    let link = checked.link;
    assert_eq!(link.state, LinkState::Broken);
    assert_eq!(link.identity_error, Some(error));
    assert_eq!(link.computed_identifier, None);

    // A neighboring two-byte text value passes the closed grammar.
    let mut valid = frame.bytes().to_vec();
    valid[error.offset + 2] = 0xa9;
    first(&valid).validate_grammar().unwrap();
}

#[test]
fn frame_links_follow_the_effective_predecessor_and_only_the_next_item() {
    use LinkState::{Broken, Healed, Linked};

    let cases: &[(&[u8], &[Option<LinkState>])] = &[
        (LINK_OK, &[Some(Linked); 4]),
        (
            include_bytes!("fixtures/v2/P1/link-healed-effective.bin"),
            &[Some(Healed), Some(Linked)],
        ),
        (
            include_bytes!("fixtures/v2/P1/link-heal-opaque-between.bin"),
            &[Some(Linked), Some(Broken), None, Some(Broken), Some(Linked)],
        ),
        (
            include_bytes!("fixtures/v2/P1/frame-identifier-type.bin"),
            &[Some(Broken), Some(Linked), Some(Linked)],
        ),
        (
            include_bytes!("fixtures/v2/P1/link-transparent-damaged.bin"),
            &[
                Some(Linked),
                Some(Linked),
                Some(Linked),
                Some(Broken),
                Some(Linked),
            ],
        ),
    ];
    for &(bytes, expected) in cases {
        let opener = first(bytes);
        let mut predecessor = Header::decode(opener).unwrap().identifier().unwrap();
        let mut tail = &bytes[opener.bytes().len()..];
        for &expected in expected {
            let item = first(tail);
            tail = &tail[item.bytes().len()..];
            let frame = item.frame().unwrap();
            let witness = Item::first(tail)
                .unwrap()
                .and_then(|next| next.frame().unwrap());
            let link = frame.map(|frame| frame.check_link(&predecessor, witness.as_ref()));
            assert_eq!(link.map(|link| link.state), expected);
            if let Some(effective) = link.and_then(|link| link.effective_identifier) {
                predecessor = effective;
            }
        }
        assert_eq!(tail, []);
    }
}

#[test]
fn nonminimal_stored_identifier_retains_its_bytes_for_linking() {
    let opener = first(LINK_OK);
    let predecessor = Header::decode(opener).unwrap().identifier().unwrap();
    let item = first(&LINK_OK[opener.bytes().len()..]);
    let frame = item.frame().unwrap().unwrap();
    let identifier = frame.field(FrameField::Identifier);
    let signatures = frame.field(FrameField::Signatures);
    let offset = item.bytes().len() - signatures.len() - identifier.len();
    let mut nonminimal = item.bytes()[..offset].to_vec();
    nonminimal.extend_from_slice(&[0x59, 0x00, 0x20]);
    nonminimal.extend_from_slice(&identifier[2..]);
    nonminimal.extend_from_slice(signatures);
    let link = first(&nonminimal)
        .frame()
        .unwrap()
        .unwrap()
        .check_link(&predecessor, None);
    assert_eq!(link.state, LinkState::Linked);
    assert_eq!(link.stored_identifier, link.effective_identifier);
    let error = link.identifier_error.unwrap();
    assert_eq!(error.reason, MalformedReason::NotDeterministic);
    assert_eq!(error.offset, offset);
}

#[test]
fn malformed_headers_report_the_first_violation_in_wire_order() {
    let two = include_bytes!("fixtures/v2/P1/header-two-violations-byte-order.bin");
    let later = first(&two[first(two).bytes().len()..]);
    let error = Header::decode(later).unwrap_err();
    assert_eq!(error.reason, MalformedReason::OutOfRange);
    assert_eq!(error.offset, 2);

    let option =
        include_bytes!("fixtures/v2/P1/opener-option-tag-first.lineage-tag-then-long-head.bin");
    let error = Header::decode(first(&option[209..])).unwrap_err();
    assert_eq!(error.reason, MalformedReason::UnregisteredValue);
    assert_eq!(error.offset, 10);
    assert!(Header::decode(first(MINIMAL)).is_ok());
}

#[test]
fn newer_registry_checks_the_ordinal_without_assuming_a_digest_length() {
    let newer = include_bytes!("fixtures/v2/P1/header-previous-head-unheld-algorithm.bin");
    let item = first(&newer[209..]);
    let error = Header::decode(item).unwrap_err();
    assert_eq!(error.reason, MalformedReason::OutOfRange);
    assert_eq!(error.offset, 22);

    // Replacing the zero ordinal makes the same eight-byte digest admissible.
    let mut valid = item.bytes().to_vec();
    valid[error.offset] = 1;
    let header = Header::decode(first(&valid)).unwrap();
    assert_eq!(header.registry_version().get(), 2);
    assert_eq!(header.hash_algorithm().get(), 99);
    assert_eq!(header.lineage().unwrap().previous_head.len(), 8);
    assert_eq!(header.identifier(), None);

    let unregistered = include_bytes!("fixtures/v2/P1/header-hash-unregistered.bin");
    assert_eq!(
        Header::decode(first(unregistered)).unwrap_err().reason,
        MalformedReason::UnregisteredValue
    );
    let profile = include_bytes!("fixtures/v2/P1/header-profile-unknown.bin");
    let header = Header::decode(first(profile)).unwrap();
    assert_eq!(header.profile().unwrap().get(), 99);
    assert!(header.identifier().is_some());
}
