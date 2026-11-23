// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! GTS v2 byte-exact segment output and checks of range commitments.

use std::io::{self, Write};

use purrdf_gts::v2::{
    FrameField, FrameType, Header, Item, LinkState, ParityCheck, PayloadBudget, PayloadCapacity,
    PayloadRefusal, SegmentFrame, SegmentVerifier, SegmentWriteError, SegmentWriter,
};

const CAPACITY: PayloadCapacity = PayloadCapacity {
    stored_bytes_per_frame: 1 << 26,
    decoded_bytes_per_frame: 1 << 26,
    decoded_bytes_per_read: 1 << 36,
};

#[test]
fn a_plain_blob_and_commit_match_the_upstream_segment_bytes() {
    let mut writer = SegmentWriter::new(Vec::new()).unwrap();
    let digest = writer.add_blob(&[], None).unwrap();
    let expected = purrdf_hash::hex::decode_32(
        "e0fd1bfed8d0ebdc51d5dd4566a49697189d80a57468118c0488ffdc1f34f4be",
    )
    .unwrap();
    assert_eq!(digest, expected);
    let identifier = writer.add_commit().unwrap();
    assert_eq!(writer.head(), Some(&identifier));
    assert_eq!(
        writer.into_inner(),
        include_bytes!("fixtures/v2/P4/blob-empty.bin")
    );
}

#[test]
fn commit_count_and_root_use_the_observed_ordinary_prefix() {
    for (bytes, count_matches, root_matches) in [
        (
            include_bytes!("fixtures/v2/P5/commit-valid.bin").as_slice(),
            true,
            true,
        ),
        (
            include_bytes!("fixtures/v2/P5/commit-count-mismatch.bin").as_slice(),
            false,
            true,
        ),
        (
            include_bytes!("fixtures/v2/P5/commit-root-mismatch.bin").as_slice(),
            true,
            false,
        ),
    ] {
        let opener = Item::first(bytes).unwrap().unwrap();
        let header = Header::decode(opener).unwrap();
        let mut verifier =
            SegmentVerifier::new(header.identifier().unwrap(), header.registry_version());
        let mut budget = PayloadBudget::new(CAPACITY);
        let mut tail = &bytes[opener.bytes().len()..];
        let result = loop {
            let item = Item::first(tail)
                .unwrap()
                .expect("the fixture contains a commit");
            tail = &tail[item.bytes().len()..];
            let checked = verifier
                .check(&item.frame().unwrap().unwrap(), None, &mut budget)
                .unwrap();
            assert_eq!(checked.frame.link.state, LinkState::Linked);
            assert_eq!(checked.frame.malformed, None);
            assert!(!checked.frame.parity_envelope_damaged);
            if let Some(commit) = checked.commit {
                break commit;
            }
        };
        assert_eq!(result.actual_count, 2);
        assert_eq!(result.count_matches, count_matches);
        assert_eq!(result.root_matches, root_matches);
        assert_eq!(result.valid, count_matches && root_matches);
    }
}

#[test]
fn a_well_formed_signature_still_damages_the_parity_envelope() {
    let bytes = include_bytes!("fixtures/v2/P1/parity-signature.bin");
    let opener = Item::first(bytes).unwrap().unwrap();
    let header = Header::decode(opener).unwrap();
    let mut verifier =
        SegmentVerifier::new(header.identifier().unwrap(), header.registry_version());
    let mut budget = PayloadBudget::new(CAPACITY);
    let mut tail = &bytes[opener.bytes().len()..];
    loop {
        let item = Item::first(tail)
            .unwrap()
            .expect("the fixture contains a signed parity frame");
        tail = &tail[item.bytes().len()..];
        let predecessor = *verifier.head();
        let frame = item.frame().unwrap().unwrap();
        let checked = verifier.check(&frame, None, &mut budget).unwrap();
        if checked.frame.frame_type.and_then(FrameType::from_code) == Some(FrameType::Parity) {
            assert_eq!(checked.frame.link.state, LinkState::Linked);
            assert_eq!(checked.frame.malformed, None);
            assert!(checked.frame.parity_envelope_damaged);
            assert!(
                !item
                    .passes_framing_test(Some((header.registry_version(), &predecessor)), None)
                    .unwrap()
            );
            let signature_offset = item.bytes().len() - frame.field(FrameField::Signatures).len();
            let mut unsigned = item.bytes()[..signature_offset].to_vec();
            unsigned.push(0x80);
            assert!(
                Item::first(&unsigned)
                    .unwrap()
                    .unwrap()
                    .passes_framing_test(Some((header.registry_version(), &predecessor)), None)
                    .unwrap()
            );
            break;
        }
    }
}

fn checked_frames(bytes: &[u8]) -> impl Iterator<Item = SegmentFrame<'_>> {
    let opener = Item::first(bytes).unwrap().unwrap();
    let header = Header::decode(opener).unwrap();
    let mut verifier =
        SegmentVerifier::new(header.identifier().unwrap(), header.registry_version());
    let mut budget = PayloadBudget::new(CAPACITY);
    let mut tail = &bytes[opener.bytes().len()..];
    core::iter::from_fn(move || {
        while let Some(item) = Item::first(tail).unwrap() {
            tail = &tail[item.bytes().len()..];
            if let Some(frame) = item.frame().unwrap() {
                return Some(verifier.check(&frame, None, &mut budget).unwrap());
            }
        }
        None
    })
}

fn parity_checks(bytes: &[u8]) -> Vec<ParityCheck<'_>> {
    checked_frames(bytes)
        .filter_map(|frame| frame.parity)
        .collect()
}

#[test]
fn index_head_root_and_runs_are_independent_prefix_claims() {
    for (bytes, expected) in [
        (
            include_bytes!("fixtures/v2/P5/index-intact.bin").as_slice(),
            (true, true, true),
        ),
        (
            include_bytes!("fixtures/v2/P5/index-each-claim.head.bin").as_slice(),
            (false, true, true),
        ),
        (
            include_bytes!("fixtures/v2/P5/index-each-claim.range_root.bin").as_slice(),
            (true, false, true),
        ),
        (
            include_bytes!("fixtures/v2/P5/index-each-claim.type_runs.bin").as_slice(),
            (true, true, false),
        ),
    ] {
        let index = checked_frames(bytes)
            .find_map(|frame| frame.index)
            .unwrap()
            .unwrap();
        assert_eq!(
            (
                index.head_matches,
                index.root_matches,
                index.type_runs_match
            ),
            expected
        );
    }
}

#[test]
fn index_runs_include_ordinary_frames_that_do_not_fold() {
    for (bytes, expected) in [
        (
            include_bytes!("fixtures/v2/P5/index-after-stop.exact.bin").as_slice(),
            true,
        ),
        (
            include_bytes!("fixtures/v2/P5/index-after-stop.wrong-type-runs.bin").as_slice(),
            false,
        ),
        (
            include_bytes!("fixtures/v2/P5/index-covers-frame-of-no-type.long-form-15-omitted.bin")
                .as_slice(),
            false,
        ),
    ] {
        let index = checked_frames(bytes)
            .find_map(|frame| frame.index)
            .unwrap()
            .unwrap();
        assert!(index.head_matches);
        assert!(index.root_matches);
        assert_eq!(index.type_runs_match, expected);
    }
}

#[test]
fn parity_shards_can_be_checked_when_the_table_survives_payload_damage() {
    let intact = include_bytes!("fixtures/v2/P5/parity-base.bin");
    let damaged = include_bytes!("fixtures/v2/P5/parity-damaged-shard.bin");
    for (bytes, damaged_shards) in [(intact.as_slice(), &[][..]), (damaged.as_slice(), &[3][..])] {
        let checks = parity_checks(bytes);
        let [
            ParityCheck::Table {
                table,
                payload_damaged,
            },
        ] = checks.as_slice()
        else {
            panic!("expected one verified parity table: {checks:?}");
        };
        assert_eq!(*payload_damaged, !damaged_shards.is_empty());
        assert_eq!(table.span(), 0..481);
        let report = table.compare_shards(&bytes[..481]).unwrap();
        assert_eq!(report.data_shards().count(), 0);
        assert!(report.parity_shards().eq(damaged_shards.iter().copied()));
        let wrong_size = table.compare_shards(&bytes[..480]).unwrap_err();
        assert_eq!((wrong_size.expected, wrong_size.supplied), (481, 480));
    }
}

#[test]
fn parity_checks_envelope_damage_even_when_capacity_prevents_reading_the_table() {
    let unchecked = include_bytes!("fixtures/v2/P5/parity-above-capacity-wrong-table.bin");
    assert!(matches!(
        parity_checks(unchecked).as_slice(),
        [ParityCheck::Unchecked(PayloadRefusal::DecodedSize(
            134_217_728
        ))]
    ));
    for bytes in [
        include_bytes!("fixtures/v2/P5/parity-above-capacity-and-damaged.bin").as_slice(),
        include_bytes!("fixtures/v2/P5/parity-damaged-each.5.bin").as_slice(),
    ] {
        assert!(matches!(
            parity_checks(bytes).as_slice(),
            [ParityCheck::Damaged]
        ));
    }
}

#[test]
fn parity_history_distinguishes_unsupported_geometry_from_a_damaged_span() {
    let newer = include_bytes!(
        "fixtures/v2/P5/parity-condition-six-first-by-case.later-version-geometry.bin"
    );
    assert!(matches!(
        parity_checks(newer).as_slice(),
        [
            ParityCheck::UnknownGeometry {
                code: 9,
                report: true
            },
            ParityCheck::Table { .. }
        ]
    ));
    let invalid = include_bytes!(
        "fixtures/v2/P5/parity-condition-six-first-by-case.own-version-geometry-zero.bin"
    );
    assert!(matches!(
        parity_checks(invalid).as_slice(),
        [ParityCheck::Damaged, ParityCheck::Damaged]
    ));
    let broken = include_bytes!("fixtures/v2/P5/parity-broken-copy-no-first.bin");
    assert!(matches!(
        parity_checks(broken).as_slice(),
        [ParityCheck::Damaged, ParityCheck::Table { .. }]
    ));
}

struct ShortSink {
    bytes: Vec<u8>,
    limit: usize,
}

impl Write for ShortSink {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let available = self.limit - self.bytes.len();
        if available == 0 {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        let count = bytes.len().min(available);
        self.bytes.extend_from_slice(&bytes[..count]);
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn incomplete_sink_write_prevents_a_new_frame_from_using_the_old_head() {
    let mut writer = SegmentWriter::new(ShortSink {
        bytes: Vec::new(),
        limit: 15,
    })
    .unwrap();
    let error = writer.add_blob(b"blob", None).unwrap_err();
    assert!(
        matches!(error, SegmentWriteError::Io(cause) if cause.kind() == io::ErrorKind::BrokenPipe)
    );
    assert_eq!(writer.head(), None);
    assert!(matches!(
        writer.add_commit(),
        Err(SegmentWriteError::IncompleteFrame)
    ));
    assert_eq!(writer.into_inner().bytes.len(), 15);
}
