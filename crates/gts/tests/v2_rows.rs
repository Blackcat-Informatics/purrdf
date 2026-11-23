// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! GTS v2 row columns, graph runs and payload refusal ordering.

use core::num::NonZeroU64;

use purrdf_gts::v2::{
    ContentError, DecodedRows, FormedBudget, Header, Item, Line, MalformedReason, PayloadBudget,
    PayloadCapacity, RdfFold, Row, RowDisposition, RowOrder, SegmentDictionary, SegmentVerifier,
    ValueStore,
};

fn rows(bytes: &[u8], count: usize) -> Vec<Result<DecodedRows, ContentError>> {
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
            .expect("the fixture contains a row frame");
        tail = &tail[item.bytes().len()..];
        if let Some(frame) = item.frame().unwrap() {
            let checked = verifier.check(&frame, None, &mut budget).unwrap();
            if let Some(decoded) = verifier.decode_rows(&checked) {
                observed.push(decoded);
            }
        }
    }
    observed
}

#[test]
fn row_columns_decode_ids_multiplicity_and_graph_context() {
    let quads = rows(
        include_bytes!("fixtures/v2/P3/rows-order-canonical-two-runs.bin"),
        1,
    );
    let quads = quads[0].as_ref().unwrap();
    assert_eq!(quads.order, Some(RowOrder::Canonical));
    assert_eq!(
        quads.rows,
        [Row::Quad([0, 1, 2], None), Row::Quad([0, 1, 2], Some(3))]
    );

    let annotations = rows(
        include_bytes!("fixtures/v2/P3/annotations-counts-add.bin"),
        2,
    );
    for (decoded, count) in annotations.into_iter().zip([1, 2]) {
        assert_eq!(
            decoded.unwrap().rows,
            [Row::Annotation(
                [4, 1, 2],
                NonZeroU64::new(count).unwrap(),
                None
            ),]
        );
    }

    let empty = rows(include_bytes!("fixtures/v2/P3/empty-graphs-column.bin"), 1);
    assert_eq!(
        empty[0].as_ref().unwrap().rows,
        [0, 1, 3].map(Row::EmptyGraph)
    );

    let reifiers = rows(
        include_bytes!("fixtures/v2/P3/reifier-bindings-triple-not-triple.bin"),
        1,
    );
    assert_eq!(
        reifiers[0].as_ref().unwrap().rows,
        [Row::ReifierBinding([4, 2], None)]
    );
}

#[test]
fn payload_value_checks_precede_column_count_checks() {
    let invalid = rows(
        include_bytes!("fixtures/v2/P3/rows-order-type-before-count.bin"),
        1,
    );
    assert!(matches!(
        invalid[0].as_ref(),
        Err(ContentError::Malformed(error)) if error.reason == MalformedReason::OutOfRange
    ));
    let mismatch = rows(include_bytes!("fixtures/v2/P3/rows-run-column-sum.bin"), 1);
    assert!(matches!(
        mismatch[0].as_ref(),
        Err(ContentError::Malformed(error)) if error.reason == MalformedReason::InconsistentCounts
    ));
}

#[test]
fn a_missing_head_and_a_complete_head_of_the_wrong_type_are_distinct() {
    for (bytes, reason) in [
        (
            include_bytes!("fixtures/v2/P1/payload-ill-formed-head.1f.bin").as_slice(),
            MalformedReason::WrongLength,
        ),
        (
            include_bytes!("fixtures/v2/P1/payload-ill-formed-head.f8-18.bin").as_slice(),
            MalformedReason::WrongLength,
        ),
        (
            include_bytes!("fixtures/v2/P1/payload-ill-formed-head.text-cut.bin").as_slice(),
            MalformedReason::WrongType,
        ),
    ] {
        let decoded = rows(bytes, 1);
        assert!(
            matches!(decoded[0].as_ref(), Err(ContentError::Malformed(error)) if error.reason == reason)
        );
    }
}

fn fold_fixture(bytes: &[u8]) -> (SegmentDictionary, RdfFold, Vec<DecodedRows>) {
    let opener = Item::first(bytes).unwrap().unwrap();
    let header = Header::decode(opener).unwrap();
    let mut verifier =
        SegmentVerifier::new(header.identifier().unwrap(), header.registry_version());
    let mut budget = PayloadBudget::new(PayloadCapacity {
        stored_bytes_per_frame: 1 << 26,
        decoded_bytes_per_frame: 1 << 26,
        decoded_bytes_per_read: 1 << 36,
    });
    let mut formed = FormedBudget::new(1 << 26);
    let mut dictionary = SegmentDictionary::new(0);
    let mut fold = RdfFold::default();
    let values = ValueStore::default();
    let mut tail = &bytes[opener.bytes().len()..];
    let mut rows = Vec::new();
    while let Some(item) = Item::first(tail).unwrap() {
        tail = &tail[item.bytes().len()..];
        if let Some(frame) = item.frame().unwrap() {
            let checked = verifier.check(&frame, None, &mut budget).unwrap();
            if let Some(reservation) = &checked.terms {
                let decoded = verifier
                    .decode_terms(&checked, &mut formed)
                    .transpose()
                    .unwrap();
                dictionary
                    .observe_terms(&mut fold, reservation, decoded, &values)
                    .unwrap();
            }
            if let Some(decoded) = verifier.decode_rows(&checked) {
                let decoded = decoded.unwrap();
                dictionary.fold_rows(&mut fold, &decoded).unwrap();
                rows.push(decoded);
            }
        }
    }
    (dictionary, fold, rows)
}

#[test]
fn reserved_terms_and_position_rules_have_distinct_row_outcomes() {
    let (mut dictionary, fold, rows) = fold_fixture(include_bytes!(
        "fixtures/v2/P3/reifier-bindings-triple-not-triple.bin"
    ));
    let invalid = rows[0].rows[0];
    assert_eq!(
        dictionary.resolve_row(invalid),
        RowDisposition::PositionConstraint
    );
    assert_eq!(fold.lines().count(), 0);
    let valid = Row::Quad([0, 1, 2], None);
    assert!(matches!(
        dictionary.resolve_row(valid),
        RowDisposition::Accepted(Line::Quad(..))
    ));
    dictionary.mark_doubt();
    assert_eq!(dictionary.resolve_row(invalid), RowDisposition::Held);
    assert_eq!(
        dictionary.resolve_row(Row::Quad([0, 1, u64::MAX], None)),
        RowDisposition::ForwardReference
    );
}

#[test]
fn equal_annotation_lines_add_counts_in_the_fold() {
    let (_, fold, _) = fold_fixture(include_bytes!("fixtures/v2/P3/annotations-counts-add.bin"));
    let lines: Vec<_> = fold.lines().collect();
    assert!(matches!(
        lines.as_slice(),
        [Line::Annotation(_, count, None)] if count.get() == 3
    ));
}
