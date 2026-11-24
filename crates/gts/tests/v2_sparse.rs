// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Indexed range reads against frozen GTS generation 2 fixtures.

use std::io;

use purrdf_gts::v2::{
    FrameField, Item, RangeAnswer, RangeContent, RangeQuery, RangeReason, RangeTarget,
    ReadCapacity, ValueName, read_range,
};

fn read<'a>(
    bytes: &[u8],
    target: RangeTarget<'a>,
    suppression: bool,
    capacity: ReadCapacity,
    mut serve: impl FnMut(usize, &mut [u8]) -> io::Result<()>,
) -> RangeAnswer<'a> {
    let length = bytes.len() as u64;
    let query = RangeQuery {
        segment_end: length,
        target,
        suppression,
    };
    read_range(length, query, capacity, |offset, buffer| {
        serve(usize::try_from(offset).unwrap(), buffer)
    })
    .unwrap()
}

fn read_file<'a>(bytes: &[u8], target: RangeTarget<'a>, suppression: bool) -> RangeAnswer<'a> {
    read(
        bytes,
        target,
        suppression,
        ReadCapacity::default(),
        |offset, into| {
            into.copy_from_slice(&bytes[offset..offset + into.len()]);
            Ok(())
        },
    )
}

#[test]
fn sparse_read_refusal_preserves_the_holder_error() {
    let bytes = include_bytes!("fixtures/v2/P7/idx-charges-once.compressed-R-plus-D.bin");
    let answer = read(
        bytes,
        RangeTarget::Graph(None),
        false,
        ReadCapacity::default(),
        |_, _| Err(io::ErrorKind::TimedOut.into()),
    );
    assert_eq!(answer.outcome.unwrap_err(), RangeReason::Read);
    assert_eq!(answer.read_error.unwrap().kind(), io::ErrorKind::TimedOut);
}

#[test]
fn sparse_budget_charges_ranges_once_and_also_charges_compressed_output() {
    let bytes = include_bytes!("fixtures/v2/P7/idx-charges-once.compressed-R-plus-D.bin");
    for limit in [514, 515] {
        let mut capacity = ReadCapacity::default();
        capacity.payload.decoded_bytes_per_read = limit;
        let mut read_bytes = [false; 1095];
        let answer = read(
            bytes,
            RangeTarget::Graph(None),
            false,
            capacity,
            |offset, into| {
                let end = offset + into.len();
                assert!(!read_bytes[offset..end].contains(&true));
                read_bytes[offset..end].fill(true);
                into.copy_from_slice(&bytes[offset..end]);
                Ok(())
            },
        );
        assert_eq!(read_bytes.into_iter().filter(|&read| read).count(), 501);
        if limit == 514 {
            assert_eq!(answer.outcome.unwrap_err(), RangeReason::Capacity);
        } else {
            let RangeContent::Graph(graph) = answer.outcome.unwrap() else {
                panic!("the index selects the default graph");
            };
            assert_eq!(graph.rows.len(), 1);
            assert_eq!(graph.rows[0].position, 2);
        }
    }
}

#[test]
fn sparse_suppression_hides_rows_and_refuses_when_it_needs_a_value() {
    let bytes = include_bytes!("fixtures/v2/P7/idx-suppress.true.bin");
    let target = RangeTarget::Graph(None);
    let RangeContent::Graph(graph) = read_file(bytes, target, true).outcome.unwrap() else {
        panic!("the index selects the default graph");
    };
    let visible: Vec<_> = graph.rows.iter().map(|row| row.visible).collect();
    assert_eq!(visible, [Some(false), Some(true)]);

    let bytes = include_bytes!("fixtures/v2/P7/idx-suppress-by-reference.true.bin");
    assert_eq!(
        read_file(bytes, target, true).outcome.unwrap_err(),
        RangeReason::Value
    );
    let RangeContent::Graph(graph) = read_file(bytes, target, false).outcome.unwrap() else {
        panic!("disabling suppression leaves the plain row readable");
    };
    assert_eq!(graph.rows.len(), 1);
    assert_eq!(graph.rows[0].visible, None);

    let bytes = include_bytes!("fixtures/v2/P7/idx-suppress-by-reference.literal-ref-file.bin");
    let RangeContent::Graph(graph) = read_file(bytes, target, true).outcome.unwrap() else {
        panic!("the default graph can carry a value by reference");
    };
    assert_eq!(graph.rows.len(), 1);
    assert!(graph.rows[0].by_reference);
    assert_eq!(graph.rows[0].visible, Some(true));
}

#[test]
fn sparse_unit_lookup_verifies_the_claimed_pack_member() {
    let bytes = include_bytes!("fixtures/v2/P7/idx-unit-pack.bin");
    let digest = [
        0x9d, 0x03, 0xbc, 0xac, 0x13, 0xcc, 0x8d, 0x71, 0x59, 0x0f, 0xa1, 0xa4, 0x63, 0x0b, 0x0b,
        0x75, 0xbc, 0x53, 0x62, 0xb1, 0xcc, 0x55, 0xf8, 0xc4, 0xa1, 0x28, 0xc4, 0x8c, 0xb6, 0xc2,
        0x15, 0x6b,
    ];
    let RangeContent::Units(units) = read_file(bytes, RangeTarget::Unit(&digest), false)
        .outcome
        .unwrap()
    else {
        panic!("the index selects a pack member");
    };
    let [unit] = units.as_slice() else {
        panic!("only one member identifies the requested unit");
    };
    assert_eq!((unit.position, unit.member), (1, Some(1)));
    assert_eq!(ValueName::of(&unit.bytes).digest, digest);
}

#[test]
fn sparse_unit_lookup_rejects_a_chunk_with_no_body() {
    let bytes = include_bytes!("fixtures/v2/P7/idx-chunk-empty.bin");
    let digest = ValueName::of(b"").digest;
    let answer = read_file(bytes, RangeTarget::Unit(&digest), false);
    assert_eq!(answer.outcome.unwrap_err(), RangeReason::Payload);
    assert_eq!(answer.scope.unwrap().frame_count, Some(2));
}

#[test]
fn sparse_unit_lookup_validates_the_storage_head_of_a_nonmatching_unit() {
    let fixture = include_bytes!("fixtures/v2/P7/idx-unit-chunk.first-frame.bin");
    let target = RangeTarget::Unit(&[0; 32]);
    let RangeContent::Units(units) = read_file(fixture, target, false).outcome.unwrap() else {
        panic!("the absent digest selects no unit");
    };
    assert!(units.is_empty());

    let opener = Item::first(fixture).unwrap().unwrap();
    let item = Item::first(&fixture[opener.bytes().len()..])
        .unwrap()
        .unwrap();
    let frame = item.frame().unwrap().unwrap();
    let storage = frame.field(FrameField::Storage);
    assert_eq!(storage, [0x81, 1]);
    let offset = storage.as_ptr() as usize - fixture.as_ptr() as usize;
    let mut invalid = *fixture;
    invalid[offset + 1] = 0;
    assert_eq!(
        read_file(&invalid, target, false).outcome.unwrap_err(),
        RangeReason::Payload
    );
}
