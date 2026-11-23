// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! GTS v2 byte-exact segment output and checks of range commitments.

use std::io::{self, Write};

use purrdf_gts::v2::{
    FrameField, FrameType, Header, Item, LinkState, PayloadBudget, PayloadCapacity,
    SegmentVerifier, SegmentWriteError, SegmentWriter,
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
