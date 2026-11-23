// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Generation dispatch and the GTS v2 RDF graph entry points.

use core::num::NonZeroU64;

use purrdf_gts::v2::{
    ContentError, FrameField, FramingDamage, Item, ItemKind, Line, MalformedReason, PayloadRefusal,
    RdfGraph, ReadCapacity, ReadFailure, ReadGraph, SegmentWriteError, SegmentWriter, Term,
    ValueName, read,
};
use purrdf_lex::cbor::head;

fn second(bytes: &[u8]) -> RdfGraph<'_> {
    let ReadGraph::Second(graph) = read(bytes, ReadCapacity::default()).unwrap() else {
        panic!("the fixture opens a generation 2 file");
    };
    graph
}

#[test]
fn legacy_opener_delegates_the_entire_input() {
    let bytes = include_bytes!("fixtures/v2/P9/reader__first-and-second.bin");
    let expected = purrdf_gts::reader::read(bytes, true, None);
    let ReadGraph::First(actual) = read(bytes, ReadCapacity::default()).unwrap() else {
        panic!("the fixture opens a generation 1 file");
    };
    assert_eq!(actual.terms, expected.terms);
    assert_eq!(actual.quads, expected.quads);
    assert_eq!(actual.segment_heads, expected.segment_heads);
    assert_eq!(
        format!("{:?}", actual.diagnostics),
        format!("{:?}", expected.diagnostics)
    );
}

#[test]
fn scanned_openers_preserve_the_difference_between_torn_and_undelimitable_prefixes() {
    let foreign = include_bytes!("fixtures/v2/P1/resync-first-byte-foreign.whole.bin");
    let torn = include_bytes!("fixtures/v2/P1/resync-first-item-torn.whole.bin");
    for (bytes, opener_offset, prefix, items) in [
        (
            foreign.as_slice(),
            2,
            FramingDamage::Undelimitable {
                offset: 0,
                length: 2,
            },
            4,
        ),
        (
            torn.as_slice(),
            10,
            FramingDamage::TornTail {
                offset: 0,
                length: 10,
            },
            6,
        ),
    ] {
        let capacity = ReadCapacity {
            items,
            ..ReadCapacity::default()
        };
        let ReadGraph::Second(graph) = read(bytes, capacity).unwrap() else {
            panic!("the scanned opener identifies a generation 2 file");
        };
        let [resumed] = graph.resynchronizations.as_slice() else {
            panic!("the input contains one scanned opener");
        };
        assert_eq!(resumed.offset, opener_offset);
        assert_eq!(resumed.damage, Some(prefix));
        assert!(matches!(graph.rdf.lines().next(), Some(Line::Quad(..))));
        let capacity = ReadCapacity {
            items: items - 1,
            ..capacity
        };
        assert!(matches!(
            read(bytes, capacity).unwrap_err().failure,
            ReadFailure::ItemCapacity(_)
        ));
    }
}

#[test]
fn a_resync_opener_in_a_damaged_prefix_respects_the_read_limits() {
    let bytes = include_bytes!("fixtures/v2/P1/resync-first-byte-foreign.whole.bin");
    let mut capacity = ReadCapacity {
        items: 1,
        ..ReadCapacity::default()
    };
    let error = read(bytes, capacity).unwrap_err();
    assert_eq!(error.offset, 0);
    assert!(matches!(error.failure, ReadFailure::Framing(_)));

    capacity.items = 4;
    capacity.payload.decoded_bytes_per_read = 4095;
    let error = read(bytes, capacity).unwrap_err();
    assert_eq!(error.offset, 2);
    assert!(matches!(
        error.failure,
        ReadFailure::PayloadRefusal(PayloadRefusal::TotalDecoded(4096))
    ));

    // The complete failing opener plus its two held frames reach capacity
    // before the candidate can be tested or charged.
    let complete = include_bytes!("fixtures/v2/P1/resync-first-item-torn.whole.bin");
    capacity.items = 3;
    let error = read(complete, capacity).unwrap_err();
    assert_eq!(error.offset, 0);
    assert!(matches!(error.failure, ReadFailure::Wire(_)));

    capacity.items = 2;
    let error = read(complete, capacity).unwrap_err();
    assert_eq!(error.offset, 177);
    assert!(matches!(error.failure, ReadFailure::ItemCapacity(2)));

    // An incomplete first item occupies no slot. The accepted opener
    // fills the one available slot, so the first following item is unread.
    let incomplete = include_bytes!(
        "fixtures/v2/P1/resync-incomplete-item-not-counted.first-item-incomplete.bin"
    );
    capacity.payload = ReadCapacity::default().payload;
    capacity.items = 1;
    let error = read(incomplete, capacity).unwrap_err();
    assert_eq!(error.offset, 264);
    assert!(matches!(error.failure, ReadFailure::ItemCapacity(1)));
}

#[test]
fn a_later_undelimitable_region_consumes_one_item_before_recovery() {
    let bytes = include_bytes!("fixtures/v2/P1/resync-item-limit-region.whole.bin");
    let mut capacity = ReadCapacity {
        items: 3,
        ..ReadCapacity::default()
    };
    capacity.payload.decoded_bytes_per_read = 17;
    let error = read(bytes, capacity).unwrap_err();
    assert_eq!(error.offset, 114);
    assert!(matches!(error.failure, ReadFailure::Framing(_)));

    // The segment before the damage spends 17 bytes. The scan adds
    // another 4,096 and the resumed terms and row frames add 31.
    capacity.items = 6;
    capacity.payload.decoded_bytes_per_read = 4144;
    let ReadGraph::Second(graph) = read(bytes, capacity).unwrap() else {
        panic!("the input opens a generation 2 file");
    };
    let [resumed] = graph.resynchronizations.as_slice() else {
        panic!("the input contains one scanned opener");
    };
    assert_eq!(resumed.offset, 115);
    assert_eq!(
        resumed.damage,
        Some(FramingDamage::Undelimitable {
            offset: 114,
            length: 1
        })
    );
    assert!(matches!(graph.rdf.lines().next(), Some(Line::Quad(..))));

    capacity.payload.decoded_bytes_per_read = 4143;
    let refused = read(bytes, capacity).unwrap_err();
    assert!(matches!(
        refused.failure,
        ReadFailure::PayloadRefusal(PayloadRefusal::TotalDecoded(4144))
    ));
}

#[test]
fn a_later_torn_item_leaves_the_last_slot_for_the_resync_opener() {
    let bytes =
        include_bytes!("fixtures/v2/P1/resync-incomplete-item-not-counted.ends-at-candidate.bin");
    let capacity = ReadCapacity {
        items: 3,
        ..ReadCapacity::default()
    };
    let ReadGraph::Second(graph) = read(bytes, capacity).unwrap() else {
        panic!("the input opens a generation 2 file");
    };
    let [resumed] = graph.resynchronizations.as_slice() else {
        panic!("the input contains one scanned opener");
    };
    assert_eq!(resumed.offset, 199);
    assert_eq!(
        resumed.damage,
        Some(FramingDamage::TornTail {
            offset: 114,
            length: 85
        })
    );
}

#[test]
fn value_rebuild_precedes_term_resolution_across_segments() {
    for bytes in [
        include_bytes!("fixtures/v2/P3/terms-literal-ref-value-after.value-before.bin").as_slice(),
        include_bytes!("fixtures/v2/P3/content-digest-own-fold-value-elsewhere.bin").as_slice(),
    ] {
        let graph = second(bytes);
        let [Line::Quad([_, _, object], None)] = graph.rdf.lines().collect::<Vec<_>>()[..] else {
            panic!("the fixture contains one resolved quad");
        };
        assert!(matches!(graph.rdf.term(object), Some(Term::Typed { .. })));
    }
    for bytes in [
        include_bytes!("fixtures/v2/P3/terms-literal-ref-value-after.same-segment-after.bin")
            .as_slice(),
        include_bytes!("fixtures/v2/P3/terms-literal-ref-length-cross-segment.bin").as_slice(),
    ] {
        let error = read(bytes, ReadCapacity::default()).unwrap_err();
        assert!(matches!(
            error.failure,
            ReadFailure::Definitions(findings) if findings.missing_values.len() == 1
        ));
    }
}

#[test]
fn rdf_writer_preserves_resolved_rows_and_typed_values() {
    let annotations = second(include_bytes!("fixtures/v2/P3/annotations-counts-add.bin"));
    let values = second(include_bytes!("fixtures/v2/P3/terms-forms-and-values.bin"));
    let quads = second(include_bytes!(
        "fixtures/v2/P3/rows-order-canonical-two-runs.bin"
    ));
    let mut writer = SegmentWriter::new(Vec::new()).unwrap();
    for graph in [&annotations, &values, &quads] {
        writer.add_rdf(&graph.rdf).unwrap();
    }
    writer.add_commit().unwrap();
    let bytes = writer.into_inner();
    let decoded = second(&bytes);
    assert_eq!(
        decoded
            .rdf
            .lines()
            .filter(|line| matches!(line, Line::Quad(..)))
            .count(),
        quads.rdf.lines().count() + values.rdf.lines().count()
    );
    assert_eq!(decoded.rdf.values().len(), values.rdf.values().len());
    assert!(decoded.rdf.lines().any(|line| matches!(
        line, Line::Annotation(_, count, None) if count.get() == 3
    )));
}

#[test]
fn one_annotation_frame_rejects_repeated_written_definitions() {
    for bytes in [
        include_bytes!("fixtures/v2/P3/annotations-repeat.bin").as_slice(),
        include_bytes!("fixtures/v2/P3/annotations-repeat-after-resolution.bin").as_slice(),
        include_bytes!("fixtures/v2/P3/annotations-triple-components-written-alike.components-written-alike.bin").as_slice(),
    ] {
        let error = read(bytes, ReadCapacity::default()).unwrap_err();
        assert!(matches!(
            error.failure,
            ReadFailure::Content(ContentError::Malformed(wire))
                if wire.reason == MalformedReason::UnorderedOrRepeated
        ), "{error:?}");
    }
    let graph = second(include_bytes!(
        "fixtures/v2/P3/annotations-triple-components-written-alike.components-differ.bin"
    ));
    assert!(
        graph
            .rdf
            .lines()
            .any(|line| matches!(line, Line::Annotation(..)))
    );
}

#[test]
fn a_broken_order_claim_is_reported_and_the_rows_still_fold() {
    let graph = second(include_bytes!(
        "fixtures/v2/P3/rows-order-canonical-across-frames.bin"
    ));
    assert_eq!(graph.row_order_violations.len(), 1);
    assert_eq!(graph.rdf.lines().count(), 2);
}

#[test]
fn a_stored_mismatch_does_not_refuse_verified_payload() {
    let source = include_bytes!("fixtures/v2/P1/zstd-ok.bin");
    let opener_len = Item::first(source).unwrap().unwrap().bytes().len();
    let item = Item::first(&source[opener_len..]).unwrap().unwrap();
    let frame = item.frame().unwrap().unwrap();
    let storage = frame.field(FrameField::Storage);
    let storage_end = storage.as_ptr().addr() - source.as_ptr().addr() + storage.len();
    let mut damaged_check = source.to_vec();
    damaged_check[storage_end - 1] ^= 1;
    let graph = second(&damaged_check);
    assert_eq!(graph.stored_digest_mismatches.len(), 1);
    assert_eq!(graph.blobs[0].bytes, second(source).blobs[0].bytes);

    let damaged_payload = include_bytes!("fixtures/v2/P1/stored-mismatch.bin");
    let error = read(damaged_payload, ReadCapacity::default()).unwrap_err();
    assert!(matches!(error.failure, ReadFailure::PayloadDamage(_)));
}

#[test]
fn the_item_limit_applies_before_framing_an_opener() {
    let capacity = ReadCapacity {
        items: 0,
        ..ReadCapacity::default()
    };
    let error = read(&[0x87], capacity).unwrap_err();
    assert!(matches!(error.failure, ReadFailure::ItemCapacity(0)));

    let capacity = ReadCapacity {
        items: 1,
        ..capacity
    };
    let bytes = include_bytes!("fixtures/v2/P1/header-minimal.bin");
    assert!(matches!(read(bytes, capacity), Ok(ReadGraph::Second(_))));
}

#[test]
fn recipes_use_prior_frames_and_every_unit_in_their_own_pack() {
    let compressed = second(include_bytes!(
        "fixtures/v2/P4/pack-compressed.one-frame.bin"
    ));
    assert!(compressed.blobs.is_empty());

    let earlier = second(include_bytes!(
        "fixtures/v2/P4/rebuild-unit-after-recipe.units-first.bin"
    ));
    assert_eq!(earlier.blobs.len(), 1);
    for bytes in [
        include_bytes!("fixtures/v2/P4/rebuild-unit-after-recipe.pack-recipe-first.bin").as_slice(),
        include_bytes!("fixtures/v2/P4/rebuild-unit-after-recipe.pack-recipe-last.bin").as_slice(),
    ] {
        let packed = second(bytes);
        assert_eq!(packed.blobs.len(), 1);
        assert_eq!(packed.blobs[0].name, earlier.blobs[0].name);
        assert_eq!(packed.blobs[0].bytes, earlier.blobs[0].bytes);
    }
    let later = include_bytes!("fixtures/v2/P4/rebuild-unit-after-recipe.recipe-first.bin");
    let error = read(later, ReadCapacity::default()).unwrap_err();
    assert!(matches!(error.failure, ReadFailure::MissingUnit(_)));

    let malformed = include_bytes!("fixtures/v2/P4/pack-recipe-trailing-count.bin");
    let error = read(malformed, ReadCapacity::default()).unwrap_err();
    assert!(matches!(
        error.failure,
        ReadFailure::Wire(wire) if wire.reason == MalformedReason::WrongLength
    ));
}

#[test]
fn external_blobs_round_trip_as_declarations() {
    let source = second(include_bytes!("fixtures/v2/P4/external-ok.bin"));
    let [expected] = source.external_blobs.as_slice() else {
        panic!("the fixture contains one external declaration");
    };
    let mut writer = SegmentWriter::new(Vec::new()).unwrap();
    writer
        .add_external_blob(expected.name, expected.media_type)
        .unwrap();
    writer.add_commit().unwrap();
    let bytes = writer.into_inner();
    let decoded = second(&bytes);
    assert!(decoded.blobs.is_empty());
    let [actual] = decoded.external_blobs.as_slice() else {
        panic!("the writer emits one external declaration");
    };
    assert_eq!(actual.name, expected.name);
    assert_eq!(actual.media_type, expected.media_type);
}

#[test]
fn authored_blob_recipes_use_the_specification_cut_points() {
    let source = include_bytes!("fixtures/v2/P4/chunker-cut-loose.bin");
    let mut writer = SegmentWriter::new(Vec::new()).unwrap();
    let expected = writer
        .add_chunked_blob(source, Some("application/octet-stream"))
        .unwrap();
    let before = writer.head().copied();
    assert!(matches!(
        writer.add_value(&[]),
        Err(SegmentWriteError::EmptyValue)
    ));
    assert_eq!(writer.head().copied(), before);
    let value = writer.add_value(b"lexical value").unwrap();
    assert_eq!(value, ValueName::of(b"lexical value"));
    writer.add_commit().unwrap();
    let bytes = writer.into_inner();
    let mut items = bytes.as_slice();
    let mut lengths = Vec::new();
    while let Some(item) = Item::first(items).unwrap() {
        items = &items[item.bytes().len()..];
        if let Some(frame) = item.frame().unwrap()
            && frame.field(FrameField::FrameType) == [8]
        {
            let mut length = frame.field(FrameField::DecodedLength);
            lengths.push(head::read_argument(&mut length, head::UNSIGNED).unwrap());
        }
    }
    assert_eq!(
        lengths,
        [81_019, 71_750, 18_295, 69_586, 77_918, 68_321, 13_111, 13]
    );
    let graph = second(&bytes);
    let [blob] = graph.blobs.as_slice() else {
        panic!("the recipe declares one blob");
    };
    assert_eq!(blob.name, expected);
    assert_eq!(blob.media_type.as_deref(), Some("application/octet-stream"));
    assert_eq!(&*blob.bytes, source);
}

#[test]
fn walked_resync_uses_the_computed_identifier_for_frame_links() {
    let source = include_bytes!("fixtures/v2/P1/resync-chain.with-frames.bin");
    let mut offset = 0;
    let resync_len = loop {
        let item = Item::first(&source[offset..]).unwrap().unwrap();
        if item.classify(NonZeroU64::MIN) == Ok(ItemKind::Resync) {
            break item.bytes().len();
        }
        offset += item.bytes().len();
    };
    let rows = second(source).rdf.lines().count();
    assert!(rows > 0);

    let mut changed = source.to_vec();
    changed[offset + resync_len - 1] ^= 1;
    assert_eq!(second(&changed).rdf.lines().count(), rows);

    changed[offset + 4] ^= 1;
    let error = read(&changed, ReadCapacity::default()).unwrap_err();
    assert!(matches!(
        error.failure,
        ReadFailure::Wire(wire) if wire.reason == MalformedReason::OutOfRange
    ));
}

#[test]
fn walked_resync_checks_mark_order_and_grammar_before_the_byte_limit() {
    for (bytes, reason) in [
        (
            include_bytes!("fixtures/v2/P1/marks-cut-inside-mark.at-4094.bin").as_slice(),
            MalformedReason::NotDeterministic,
        ),
        (
            include_bytes!("fixtures/v2/P1/marks-cut-inside-mark.at-4097.bin").as_slice(),
            MalformedReason::OutOfRange,
        ),
        (
            include_bytes!("fixtures/v2/P1/marks-order-at-one-byte.order-before-deeper-head.bin")
                .as_slice(),
            MalformedReason::UnorderedOrRepeated,
        ),
        (
            include_bytes!("fixtures/v2/P1/marks-order-at-one-byte.head-before-order.bin")
                .as_slice(),
            MalformedReason::NotDeterministic,
        ),
    ] {
        let error = read(bytes, ReadCapacity::default()).unwrap_err();
        assert!(
            matches!(error.failure, ReadFailure::Wire(wire) if wire.reason == reason),
            "{error:?}"
        );
    }
}

#[test]
fn a_duplicate_resync_mark_is_reported_at_its_start() {
    let marker = purrdf_hash::blake3::derive_key("gts2 header", &[0x81, 5]);
    let mut opener = vec![0x8b, 5, 2];
    head::write_bytes(&mut opener, &marker[..16]).unwrap();
    opener.extend([1, 1, 0x81, 1, 0x81, 1, 0, 0x81, 1]);
    let mut mark = vec![0x82, 9];
    head::write_bytes(&mut mark, &[0; 2060]).unwrap();

    let build = |count| {
        let mut bytes = opener.clone();
        head::write_head(&mut bytes, head::ARRAY, count).unwrap();
        for _ in 0..count {
            bytes.extend(&mark);
        }
        head::write_bytes(&mut bytes, &[0; 32]).unwrap();
        bytes
    };
    assert!(matches!(
        read(&build(1), ReadCapacity::default()),
        Ok(ReadGraph::Second(_))
    ));

    let bytes = build(2);
    let err = read(&bytes, ReadCapacity::default()).unwrap_err();
    assert!(matches!(err.failure, ReadFailure::Wire(wire)
        if wire.reason == MalformedReason::UnorderedOrRepeated
            && wire.offset == opener.len() + 1 + mark.len()));
}
