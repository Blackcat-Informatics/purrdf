// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! GTS v2 term reservations, wire definitions and front coding.

use purrdf_gts::v2::{
    ContentError, FormedBudget, FrameField, Header, Item, LinkState, LiteralForm, MalformedReason,
    PayloadBudget, PayloadCapacity, SegmentFrame, SegmentVerifier, TermDefinition, TermText,
    TypedValue,
};

fn terms_frames(bytes: &[u8], count: usize) -> (SegmentVerifier, Vec<SegmentFrame<'_>>) {
    let opener = Item::first(bytes).unwrap().unwrap();
    let header = Header::decode(opener).unwrap();
    let mut verifier =
        SegmentVerifier::new(header.identifier().unwrap(), header.registry_version());
    let mut budget = PayloadBudget::new(PayloadCapacity {
        stored_bytes_per_frame: 1 << 26,
        decoded_bytes_per_frame: 1 << 26,
        decoded_bytes_per_read: 1 << 36,
    });
    let mut tail = &bytes[opener.bytes().len()..];
    let mut observed = Vec::new();
    while observed.len() < count {
        let item = Item::first(tail)
            .unwrap()
            .expect("the fixture contains the expected terms");
        tail = &tail[item.bytes().len()..];
        if let Some(frame) = item.frame().unwrap() {
            let checked = verifier.check(&frame, None, &mut budget).unwrap();
            if checked.terms.is_some() {
                observed.push(checked);
            }
        }
    }
    (verifier, observed)
}

fn malformed(error: &ContentError, reason: MalformedReason) {
    assert!(
        matches!(error, ContentError::Malformed(error) if error.reason == reason),
        "{error:?}"
    );
}

#[test]
fn reservations_survive_an_ill_typed_base_and_never_allocate_by_the_claimed_count() {
    let (_, ill_typed) = terms_frames(
        include_bytes!("fixtures/v2/P3/terms-id-base-ill-typed-reserves.bin"),
        1,
    );
    let reservation = ill_typed[0].terms.as_ref().unwrap();
    assert_eq!((reservation.start, reservation.end), (0, 3));
    assert!(reservation.content.is_none());
    assert_eq!(
        ill_typed[0].frame.malformed.unwrap().reason,
        MalformedReason::WrongType
    );

    let (mut verifier, frames) =
        terms_frames(include_bytes!("fixtures/v2/P3/terms-id-above-uint.bin"), 3);
    let limit = u128::from(u64::MAX);
    let ranges: Vec<_> = frames
        .iter()
        .map(|frame| {
            let reservation = frame.terms.as_ref().unwrap();
            (reservation.start, reservation.end)
        })
        .collect();
    assert_eq!(ranges, [(0, limit), (limit, limit), (limit, limit + 1)]);
    let mut budget = FormedBudget::new(1 << 37);
    malformed(
        &verifier
            .decode_terms(&frames[0], &mut budget)
            .unwrap()
            .unwrap_err(),
        MalformedReason::InconsistentCounts,
    );
    let error = frames[1]
        .terms
        .as_ref()
        .unwrap()
        .content
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap_err();
    assert!(
        matches!(error, ContentError::Malformed(error) if error.reason == MalformedReason::OutOfRange)
    );
    let last = verifier
        .decode_terms(&frames[2], &mut budget)
        .unwrap()
        .unwrap();
    assert_eq!(
        last.terms,
        [TermDefinition::Iri(TermText::Text("a".into()))]
    );
}

#[test]
fn prefixes_can_end_inside_a_character_and_restarts_are_relative_to_each_run() {
    let mut budget = FormedBudget::new(1 << 37);
    let (mut verifier, frames) = terms_frames(
        include_bytes!("fixtures/v2/P3/terms-prefix-splits-character.bin"),
        1,
    );
    let decoded = verifier
        .decode_terms(&frames[0], &mut budget)
        .unwrap()
        .unwrap();
    assert_eq!(
        decoded.terms,
        [
            TermDefinition::Iri(TermText::Text("http://e/é".into())),
            TermDefinition::Iri(TermText::Text("http://e/è".into())),
        ]
    );
    assert_eq!(budget.charged(), 10);

    let (mut verifier, frames) =
        terms_frames(include_bytes!("fixtures/v2/P3/terms-six-terms.bin"), 1);
    let decoded = verifier
        .decode_terms(&frames[0], &mut budget)
        .unwrap()
        .unwrap();
    assert_eq!(
        decoded.terms,
        [
            TermDefinition::Iri(TermText::Text("http://e/a".into())),
            TermDefinition::Iri(TermText::Text("http://e/ab".into())),
            TermDefinition::Iri(TermText::Text("http://e/p".into())),
            TermDefinition::Iri(TermText::Text("http://e/t".into())),
            TermDefinition::Literal(TermText::Text("x".into()), LiteralForm::Typed(3)),
            TermDefinition::Blank(b"b0"),
        ]
    );
}

#[test]
fn formed_capacity_refuses_subsequent_frames_and_keeps_their_ids() {
    let (mut verifier, frames) = terms_frames(
        include_bytes!("fixtures/v2/P3/terms-formed-then-small.bin"),
        3,
    );
    let mut budget = FormedBudget::new(120);
    assert_eq!(
        verifier
            .decode_terms(&frames[0], &mut budget)
            .unwrap()
            .unwrap()
            .terms
            .len(),
        2
    );
    for (frame, count, ids) in [(&frames[1], 150, (2, 4)), (&frames[2], 110, (4, 6))] {
        assert!(matches!(
            verifier.decode_terms(frame, &mut budget).unwrap(),
            Err(ContentError::FormedCapacity(actual)) if actual == count
        ));
        let reservation = frame.terms.as_ref().unwrap();
        assert_eq!((reservation.start, reservation.end), ids);
    }
    assert_eq!(budget.charged(), 100);
}

#[test]
fn typed_values_retain_their_exact_values_and_time_boundaries() {
    let (mut verifier, frames) = terms_frames(
        include_bytes!("fixtures/v2/P3/terms-value-each-kind.bin"),
        1,
    );
    let decoded = verifier
        .decode_terms(&frames[0], &mut FormedBudget::new(0))
        .unwrap()
        .unwrap();
    assert_eq!(
        decoded.terms,
        [
            TypedValue::Boolean(false),
            TypedValue::Unsigned(0),
            TypedValue::Integer(-1),
            TypedValue::Time {
                seconds: -62_135_596_800,
                nanoseconds: 0
            },
            TypedValue::Time {
                seconds: 253_402_300_799,
                nanoseconds: 999_999_999
            },
            TypedValue::Time {
                seconds: 0,
                nanoseconds: 120_000_000
            },
            TypedValue::Octets(b""),
        ]
        .map(TermDefinition::Value)
    );
}

#[test]
fn language_indices_follow_first_use_across_inline_and_reference_blocks() {
    let (mut verifier, frames) = terms_frames(
        include_bytes!("fixtures/v2/P3/terms-language-table-both-blocks.first-use.bin"),
        1,
    );
    let decoded = verifier
        .decode_terms(&frames[0], &mut FormedBudget::new(0))
        .unwrap()
        .unwrap();
    assert_eq!(decoded.languages, ["fr", "en"]);
    assert!(matches!(&decoded.terms[..], [
        TermDefinition::LiteralRef(reference, LiteralForm::Language(0)),
        TermDefinition::Literal(_, LiteralForm::Language(1)),
        TermDefinition::Literal(_, LiteralForm::Language(0)),
    ] if reference.length == 3));

    let (mut verifier, frames) = terms_frames(
        include_bytes!("fixtures/v2/P3/terms-language-table-both-blocks.reverse.bin"),
        1,
    );
    malformed(
        &verifier
            .decode_terms(&frames[0], &mut FormedBudget::new(0))
            .unwrap()
            .unwrap_err(),
        MalformedReason::NotDeterministic,
    );
}

#[test]
fn whole_payload_type_is_checked_before_column_counts() {
    let (mut verifier, frames) = terms_frames(
        include_bytes!("fixtures/v2/P3/terms-order-type-before-count.bin"),
        1,
    );
    let mut budget = FormedBudget::new(0);
    malformed(
        &verifier
            .decode_terms(&frames[0], &mut budget)
            .unwrap()
            .unwrap_err(),
        MalformedReason::UnregisteredValue,
    );
    assert_eq!(budget.charged(), 0);
}

#[test]
fn an_unknown_payload_enumeration_stops_content_but_keeps_later_reservations() {
    let original = include_bytes!("fixtures/v2/P2/registry__unknown__value_kind.bin");
    let (mut verifier, frames) = terms_frames(original, 1);
    malformed(
        &verifier
            .decode_terms(&frames[0], &mut FormedBudget::new(0))
            .unwrap()
            .unwrap_err(),
        MalformedReason::UnregisteredValue,
    );
    assert!(!verifier.content_stopped());

    // The otherwise identical type becomes an unknown value when the
    // segment declares the newer registry.
    let opener = Item::first(original).unwrap().unwrap();
    let mut newer_opener = opener.bytes().to_vec();
    newer_opener[3] = 2;
    let header = Header::decode(Item::first(&newer_opener).unwrap().unwrap()).unwrap();
    let mut verifier =
        SegmentVerifier::new(header.identifier().unwrap(), header.registry_version());
    let mut budget = PayloadBudget::new(PayloadCapacity {
        stored_bytes_per_frame: 1 << 26,
        decoded_bytes_per_frame: 1 << 26,
        decoded_bytes_per_read: 1 << 36,
    });
    let unknown = Item::first(&original[opener.bytes().len()..])
        .unwrap()
        .unwrap();
    let mut unknown_bytes = unknown.bytes().to_vec();
    relink(&mut unknown_bytes, verifier.head());
    let frame = Item::first(&unknown_bytes)
        .unwrap()
        .unwrap()
        .frame()
        .unwrap()
        .unwrap();
    let checked = verifier.check(&frame, None, &mut budget).unwrap();
    assert_eq!(checked.frame.link.state, LinkState::Linked);
    assert!(matches!(
        verifier
            .decode_terms(&checked, &mut FormedBudget::new(0))
            .unwrap(),
        Err(ContentError::UnknownEnumeration { code: 6, .. })
    ));
    assert!(verifier.content_stopped());

    let known = include_bytes!("fixtures/v2/P3/terms-prefix-splits-character.bin");
    let opener = Item::first(known).unwrap().unwrap();
    let item = Item::first(&known[opener.bytes().len()..])
        .unwrap()
        .unwrap();
    let next = item.frame().unwrap().unwrap();
    let base = 3
        + next.field(FrameField::FrameType).len()
        + next.field(FrameField::Critical).len()
        + next.field(FrameField::Transparent).len();
    let mut next_bytes = item.bytes().to_vec();
    next_bytes[base] = 1;
    relink(&mut next_bytes, verifier.head());
    let frame = Item::first(&next_bytes)
        .unwrap()
        .unwrap()
        .frame()
        .unwrap()
        .unwrap();
    let checked = verifier.check(&frame, None, &mut budget).unwrap();
    assert_eq!(checked.frame.link.state, LinkState::Linked);
    let reservation = checked.terms.as_ref().unwrap();
    assert_eq!((reservation.start, reservation.end), (1, 3));
    assert!(reservation.content.is_none());
}

fn relink(bytes: &mut [u8], predecessor: &[u8; 32]) {
    let frame = Item::first(bytes)
        .unwrap()
        .unwrap()
        .frame()
        .unwrap()
        .unwrap();
    let identifier = frame
        .check_link(predecessor, None)
        .computed_identifier
        .unwrap();
    let end = bytes.len() - frame.field(FrameField::Signatures).len();
    let start = end - identifier.len();
    bytes[start..end].copy_from_slice(&identifier);
}
