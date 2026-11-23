// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Generation dispatch and the GTS v2 RDF graph entry points.

use core::num::NonZeroU64;

use purrdf_gts::v2::{
    ContentError, FrameField, FramingDamage, IndexStorageItem, IndexStorageProblem, Item, ItemKind,
    Line, Lineage, MalformedReason, ParityFindingKind, ParityRepairFailure, PayloadDamage,
    PayloadRefusal, RdfGraph, ReadCapacity, ReadFailure, ReadGraph, SegmentWriteError,
    SegmentWriter, Term, ValueName, read, write_header,
};
use purrdf_lex::cbor::head;

fn second(bytes: &[u8]) -> RdfGraph<'_> {
    let ReadGraph::Second(graph) = read(bytes, ReadCapacity::default()).unwrap() else {
        panic!("the fixture opens a generation 2 file");
    };
    graph
}

#[test]
fn offsets_and_locators_compare_against_the_walked_sequence() {
    let intact = include_bytes!("fixtures/v2/P5/index-intact.bin");
    let graph = second(intact);
    assert_eq!(graph.index_storage_findings, []);
    assert_eq!(graph.index_checks.len(), 1);

    let mut wrong_length = intact.to_vec();
    assert_eq!(wrong_length[703], 11);
    wrong_length[703] = 12;
    let mut wrong_locator = intact.to_vec();
    *wrong_locator.last_mut().unwrap() ^= 1;
    for (bytes, offset, kind) in [
        (wrong_length.as_slice(), 700, IndexStorageItem::Offsets),
        (wrong_locator.as_slice(), 714, IndexStorageItem::Locator),
    ] {
        let graph = second(bytes);
        let [finding] = graph.index_storage_findings.as_slice() else {
            panic!(
                "expected one storage claim mismatch: {:?}",
                graph.index_storage_findings
            );
        };
        assert_eq!(
            (finding.offset, finding.item, finding.problem),
            (offset, kind, IndexStorageProblem::ClaimMismatch)
        );
    }
}

#[test]
fn a_locator_names_the_closing_index_even_if_it_does_not_verify() {
    let bytes = include_bytes!("fixtures/v2/P5/index-intact.bin");
    let index_offset = 468;
    let item = Item::first(&bytes[index_offset..]).unwrap().unwrap();
    let frame = item.frame().unwrap().unwrap();
    let signatures_start =
        index_offset + item.bytes().len() - frame.field(FrameField::Signatures).len();
    let identifier_start = signatures_start - frame.field(FrameField::Identifier).len();
    for position in [identifier_start - 1, signatures_start - 1] {
        let mut changed = bytes.to_vec();
        changed[position] ^= 1;
        let graph = second(&changed);
        assert_eq!(graph.index_storage_findings, []);
        let [index] = graph.index_checks.as_slice() else {
            panic!("expected one index verification failure");
        };
        assert_eq!(index.offset, index_offset);
        if position < identifier_start {
            assert!(matches!(
                index.prefix,
                Err(ReadFailure::PayloadDamage(PayloadDamage::PayloadDigest))
            ));
        } else {
            assert!(matches!(index.prefix, Err(ReadFailure::BrokenLink)));
        }
    }
}

#[test]
fn a_later_commit_or_repeated_locator_invalidates_the_hints() {
    let bytes = include_bytes!("fixtures/v2/P1/resync-cosigned-not-scanned.bin");
    let prefix = &bytes[..666];
    let graph = second(prefix);
    assert_eq!(graph.index_storage_findings, []);
    assert_eq!(
        graph.parity_findings,
        second(&prefix[..643]).parity_findings
    );

    let mut repeated_locator = prefix.to_vec();
    repeated_locator.extend_from_slice(&prefix[655..]);
    let graph = second(&repeated_locator);
    assert!(
        graph
            .index_storage_findings
            .iter()
            .map(|finding| (finding.offset, finding.item, finding.problem))
            .eq([
                (
                    643,
                    IndexStorageItem::Offsets,
                    IndexStorageProblem::Misplaced
                ),
                (
                    655,
                    IndexStorageItem::Locator,
                    IndexStorageProblem::Misplaced
                ),
                (
                    666,
                    IndexStorageItem::Locator,
                    IndexStorageProblem::Misplaced
                ),
            ])
    );

    let graph = second(bytes);
    assert!(
        graph
            .index_storage_findings
            .iter()
            .map(|finding| (finding.offset, finding.item, finding.problem))
            .eq([
                (
                    643,
                    IndexStorageItem::Offsets,
                    IndexStorageProblem::Misplaced
                ),
                (
                    655,
                    IndexStorageItem::Locator,
                    IndexStorageProblem::Misplaced
                ),
            ])
    );
    assert_eq!(graph.resynchronizations, []);
}

#[test]
fn a_resync_opener_that_displaces_an_index_witness_leaves_a_broken_index() {
    let source = include_bytes!("fixtures/v2/P1/resync-cosigned-not-scanned.bin");
    // Move the embedded opener into the parity witness's final field, so
    // the witness is a complete item whose signature grammar fails.
    let offset = 642_u16;
    let opener = resync_opener_at(offset);

    let mut changed = source[..usize::from(offset)].to_vec();
    changed[420] ^= 1;
    changed[546] ^= 1;
    changed.extend_from_slice(&opener);

    let graph = second(&changed);
    let [index] = graph.index_checks.as_slice() else {
        panic!("expected the index retained before the scanned opener");
    };
    assert_eq!(index.offset, 209);
    assert!(matches!(index.prefix, Err(ReadFailure::BrokenLink)));
    let [resync] = graph.resynchronizations.as_slice() else {
        panic!("expected the opener inside the discarded witness");
    };
    assert_eq!(resync.offset, usize::from(offset));
}

fn resync_opener_at(offset: u16) -> Vec<u8> {
    let source = include_bytes!("fixtures/v2/P1/resync-cosigned-not-scanned.bin");
    let mut opener = source[542..608].to_vec();
    assert_eq!(opener[26..29], [0x19, 0x02, 0x1e]);
    opener[27..29].copy_from_slice(&offset.to_be_bytes());
    let identifier = opener.len() - 32;
    let mut hash = purrdf_hash::blake3::Hasher::new_derive_key("gts2 header");
    hash.update(&[0x8a]);
    hash.update(&opener[1..identifier - 2]);
    opener[identifier..].copy_from_slice(hash.finalize().as_bytes());
    opener
}

#[test]
fn the_graph_reader_admits_a_closing_index_before_parity() {
    let intact = include_bytes!("fixtures/v2/P5/parity-base.bin");
    let graph = second(intact);
    assert_eq!(graph.blobs.len(), 1);
    let [index] = graph.index_checks.as_slice() else {
        panic!("expected one closing index");
    };
    let prefix = index.prefix.as_ref().unwrap();
    assert_eq!(
        (
            prefix.head_matches,
            prefix.root_matches,
            prefix.type_runs_match
        ),
        (true, true, true)
    );
    assert_eq!(graph.parity_findings, []);

    let damaged = include_bytes!("fixtures/v2/P5/parity-damaged-shard.bin");
    let mut capacity = ReadCapacity::default();
    capacity.payload.decoded_bytes_per_read = 589;
    let ReadGraph::Second(graph) = read(damaged, capacity).unwrap() else {
        panic!("the fixture opens a generation 2 file");
    };
    assert_eq!(graph.blobs.len(), 1);
    let [damage, mismatch, repair] = graph.parity_findings.as_slice() else {
        panic!(
            "expected parity damage and a parity shard mismatch: {:?}",
            graph.parity_findings
        );
    };
    assert_eq!(
        (damage.offset, damage.kind),
        (481, ParityFindingKind::Damaged)
    );
    assert_eq!(
        (mismatch.offset, mismatch.kind),
        (481, ParityFindingKind::ParityMismatch(1))
    );
    assert_eq!(
        (repair.offset, repair.kind),
        (481, ParityFindingKind::Repaired(1))
    );
    assert_eq!(graph.repaired.as_deref(), Some(intact.as_slice()));
}

#[test]
fn repaired_data_requires_rechecking_its_frame_with_the_shared_budget() {
    let intact = include_bytes!("fixtures/v2/P5/parity-base.bin");
    let mut damaged = intact.to_vec();
    damaged[479] ^= 1;
    let mut capacity = ReadCapacity::default();
    for budget in [673, 674] {
        capacity.payload.decoded_bytes_per_read = budget;
        let ReadGraph::Second(graph) = read(&damaged, capacity).unwrap() else {
            panic!("the fixture opens a generation 2 file");
        };
        let [mismatch, repair] = graph.parity_findings.as_slice() else {
            panic!(
                "expected a data mismatch and a repair outcome: {:?}",
                graph.parity_findings
            );
        };
        assert_eq!(
            (mismatch.offset, mismatch.kind),
            (481, ParityFindingKind::DataMismatch(1))
        );
        if budget == 674 {
            assert_eq!(repair.kind, ParityFindingKind::Repaired(1));
            assert_eq!(graph.repaired.as_deref(), Some(intact.as_slice()));
        } else {
            assert_eq!(
                repair.kind,
                ParityFindingKind::RepairFailed(ParityRepairFailure::VerifyFailed)
            );
            assert_eq!(graph.repaired, None);
        }
    }
}

#[test]
fn a_damaged_ordinary_payload_remains_opaque_through_parity_repair() {
    let intact = include_bytes!("fixtures/v2/P5/parity-base.bin");
    let (blob_offset, damaged) = damage_first_payload(intact);

    let graph = second(&damaged);
    let [failure] = graph.frame_failures.as_slice() else {
        panic!("expected the original blob payload failure");
    };
    assert_eq!(failure.offset, blob_offset);
    assert!(matches!(
        failure.failure,
        ReadFailure::PayloadDamage(PayloadDamage::PayloadDigest)
    ));
    assert!(graph.blobs.is_empty());
    assert_eq!(graph.repaired.as_deref(), Some(intact.as_slice()));
    let repaired = second(graph.repaired.as_deref().unwrap());
    assert_eq!(repaired.blobs.len(), 1);
    assert_eq!(repaired.frame_failures.len(), 0);
}

#[test]
fn a_damaged_terms_payload_reserves_the_ids_named_by_later_rows() {
    let source =
        include_bytes!("fixtures/v2/P5/recovery-parity-in-resync-segment.undelimitable.bin");
    let intact = &source[..209];
    assert_eq!(second(intact).rdf.lines().count(), 1);
    let (terms_offset, damaged) = damage_first_payload(intact);
    let graph = second(&damaged);
    assert!(graph.rdf.has_held_rows());
    assert_eq!(graph.rdf.lines().count(), 0);
    let [terms, rows] = graph.frame_failures.as_slice() else {
        panic!("expected the terms digest failure and the held row");
    };
    assert_eq!(terms.offset, terms_offset);
    assert_eq!(rows.offset, 114);
    assert!(matches!(
        terms.failure,
        ReadFailure::PayloadDamage(PayloadDamage::PayloadDigest)
    ));
    assert!(matches!(&rows.failure, ReadFailure::Rows(finding)
        if finding.opaque_term && !finding.forward_reference));
}

fn damage_first_payload(intact: &[u8]) -> (usize, Vec<u8>) {
    let offset = Item::first(intact).unwrap().unwrap().bytes().len();
    let frame = Item::first(&intact[offset..])
        .unwrap()
        .unwrap()
        .frame()
        .unwrap()
        .unwrap();
    let payload = frame.field(FrameField::Payload);
    let last_payload_byte = payload.as_ptr().addr() - intact.as_ptr().addr() + payload.len() - 1;
    let mut damaged = intact.to_vec();
    damaged[last_payload_byte] ^= 1;
    (offset, damaged)
}

#[test]
fn a_parity_candidate_resumes_across_a_lost_item_and_repairs_its_bytes() {
    let intact = include_bytes!("fixtures/v2/P5/parity-base.bin");
    let index_offset = second(intact).index_checks[0].offset;
    let parity_offset = 481;
    let mut damaged = intact.to_vec();
    damaged[index_offset] = 0xff;
    let graph = second(&damaged);
    let [recovery] = graph.parity_recoveries.as_slice() else {
        panic!("expected the parity frame to resume its own segment");
    };
    assert_eq!(recovery.offset, parity_offset);
    assert!(recovery.detached);
    assert_eq!(
        recovery.damage,
        FramingDamage::Undelimitable {
            offset: index_offset,
            length: parity_offset - index_offset,
        }
    );
    assert_eq!(graph.blobs.len(), 1);
    assert!(graph.index_checks.is_empty());
    assert_eq!(graph.repaired.as_deref(), Some(intact.as_slice()));

    let mut capacity = ReadCapacity::default();
    for limit in [588, 589] {
        capacity.payload.decoded_bytes_per_read = limit;
        let ReadGraph::Second(graph) = read(&damaged, capacity).unwrap() else {
            panic!("expected the parity candidate to continue the segment");
        };
        assert_eq!(graph.parity_recoveries.len(), 1);
        let expected = (limit == 589).then_some(intact.as_slice());
        assert_eq!(graph.repaired.as_deref(), expected);
    }

    let index = Item::first(&intact[index_offset..])
        .unwrap()
        .unwrap()
        .frame()
        .unwrap()
        .unwrap();
    let mut claim = index.field(FrameField::Content);
    assert_eq!(head::read_argument(&mut claim, head::ARRAY), Some(2));
    assert!(head::read_argument(&mut claim, head::UNSIGNED).is_some());
    let retained_head = head::read_bytes(&mut claim).unwrap().try_into().unwrap();
    let parity_anchor = index
        .field(FrameField::Identifier)
        .strip_prefix(&[0x58, 32])
        .unwrap()
        .try_into()
        .unwrap();
    for (previous_head, valid) in [(retained_head, true), (parity_anchor, false)] {
        let mut next_segment = damaged.clone();
        write_header(
            &mut next_segment,
            None,
            None,
            Some(Lineage {
                previous_head,
                ordinal: NonZeroU64::MIN,
            }),
        )
        .unwrap();
        let result = read(&next_segment, ReadCapacity::default());
        if valid {
            assert!(matches!(result, Ok(ReadGraph::Second(_))));
        } else {
            assert!(matches!(result.unwrap_err().failure, ReadFailure::Lineage));
        }
    }
}

#[test]
fn parity_recovery_uses_the_resync_opener_and_counts_each_region_as_one_item() {
    let incomplete =
        include_bytes!("fixtures/v2/P5/recovery-parity-in-resync-segment.incomplete-item.bin");
    let undelimitable =
        include_bytes!("fixtures/v2/P5/recovery-parity-in-resync-segment.undelimitable.bin");
    let mut repaired = None;
    for bytes in [incomplete.as_slice(), undelimitable.as_slice()] {
        let mut capacity = ReadCapacity {
            items: 6,
            ..ReadCapacity::default()
        };
        let ReadGraph::Second(graph) = read(bytes, capacity).unwrap() else {
            panic!("expected a recovered generation 2 segment");
        };
        assert_eq!(graph.resynchronizations, []);
        let [recovery] = graph.parity_recoveries.as_slice() else {
            panic!("expected one accepted parity candidate");
        };
        assert_eq!(recovery.offset, 586);
        assert!(recovery.detached);
        assert_eq!(
            recovery.damage,
            FramingDamage::Undelimitable {
                offset: 274,
                length: 312,
            }
        );
        assert!(graph.parity_findings.iter().any(|finding| {
            finding.offset == 586 && finding.kind == ParityFindingKind::Repaired(1)
        }));
        let output = graph.repaired.unwrap();
        assert_eq!(second(&output).parity_recoveries, []);
        assert_eq!(repaired.get_or_insert_with(|| output.clone()), &output);

        capacity.items -= 1;
        assert!(matches!(
            read(bytes, capacity).unwrap_err().failure,
            ReadFailure::Framing(_)
        ));
    }

    let mut capacity = ReadCapacity::default();
    capacity.payload.decoded_bytes_per_read = 462;
    let refusal = read(incomplete, capacity).unwrap_err();
    assert_eq!(refusal.offset, 586);
    assert!(matches!(
        refusal.failure,
        ReadFailure::PayloadRefusal(PayloadRefusal::TotalDecoded(463))
    ));
    capacity.payload.decoded_bytes_per_read = 463;
    assert!(matches!(
        read(incomplete, capacity),
        Ok(ReadGraph::Second(_))
    ));
}

#[test]
fn recovery_accepts_whichever_valid_candidate_is_earlier() {
    let intact = include_bytes!("fixtures/v2/P5/parity-base.bin");
    let index = second(intact).index_checks[0].offset;
    let parity = 481;
    let mut damaged = intact.to_vec();
    damaged[index] = 0xff;
    let mut resync_first = damaged[..=index].to_vec();
    resync_first.extend(resync_opener_at(u16::try_from(index + 1).unwrap()));
    resync_first.extend_from_slice(&intact[parity..]);
    let graph = second(&resync_first);
    assert_eq!(graph.parity_recoveries, []);
    let [opener] = graph.resynchronizations.as_slice() else {
        panic!("expected the earlier resync opener to end the lost region");
    };
    assert_eq!(opener.offset, index + 1);

    damaged.extend(resync_opener_at(u16::try_from(intact.len()).unwrap()));
    let graph = second(&damaged);
    assert_eq!(graph.resynchronizations, []);
    let [recovery] = graph.parity_recoveries.as_slice() else {
        panic!("expected the earlier parity frame to end the lost region");
    };
    assert_eq!(recovery.offset, parity);
}

#[test]
fn a_broken_transparent_copy_does_not_hide_the_placed_parity_or_spend_an_extra_item() {
    let bytes = include_bytes!("fixtures/v2/P5/parity-broken-copy-no-first.bin");
    let capacity = ReadCapacity {
        items: 5,
        ..ReadCapacity::default()
    };
    let ReadGraph::Second(graph) = read(bytes, capacity).unwrap() else {
        panic!("the fixture opens a generation 2 file");
    };
    assert!(matches!(graph.rdf.lines().next(), Some(Line::Quad(..))));
    let [finding] = graph.parity_findings.as_slice() else {
        panic!(
            "expected damage to the first parity frame: {:?}",
            graph.parity_findings
        );
    };
    assert_eq!(finding.offset, 209);
    assert_eq!(finding.kind, ParityFindingKind::Damaged);

    let first_parity = Item::first(&bytes[209..]).unwrap().unwrap();
    let mut without_broken_copy = bytes[..209].to_vec();
    without_broken_copy.extend_from_slice(&bytes[209 + first_parity.bytes().len()..]);
    assert_eq!(second(&without_broken_copy).parity_findings, []);

    let mut two_broken_copies = bytes[..209].to_vec();
    two_broken_copies.extend_from_slice(first_parity.bytes());
    two_broken_copies.extend_from_slice(&bytes[209..]);
    let capacity = ReadCapacity {
        items: 6,
        ..capacity
    };
    let ReadGraph::Second(graph) = read(&two_broken_copies, capacity).unwrap() else {
        panic!("the input opens a generation 2 file");
    };
    assert_eq!(graph.parity_findings.len(), 2);
    assert!(
        graph
            .parity_findings
            .iter()
            .all(|finding| finding.kind == ParityFindingKind::Damaged)
    );
}

#[test]
fn changing_ordinary_storage_makes_short_parity_stale_before_the_shard_check() {
    let bytes = include_bytes!("fixtures/v2/P5/parity-broken-copy-no-first.bin");
    let signed = include_bytes!("fixtures/v2/P1/parity-signature.bin");
    let mut tail = signed.as_slice();
    let signatures = loop {
        let item = Item::first(tail).unwrap().expect("a signed frame");
        tail = &tail[item.bytes().len()..];
        if let Some(frame) = item.frame().unwrap() {
            let signatures = frame.field(FrameField::Signatures);
            if signatures != [0x80] {
                break signatures;
            }
        }
    };
    // The last ordinary frame ends in an empty signature list. Changing it
    // preserves the ordinary identifier and parity anchor but extends S.
    assert_eq!(bytes[208], 0x80);
    let first_parity = Item::first(&bytes[209..]).unwrap().unwrap();
    let mut rewritten = bytes[..208].to_vec();
    rewritten.extend_from_slice(signatures);
    let parity_start = rewritten.len();
    rewritten.extend_from_slice(&bytes[209 + first_parity.bytes().len()..]);

    let graph = second(&rewritten);
    let [stale, tail] = graph.parity_findings.as_slice() else {
        panic!(
            "expected stale parity and an unprotected tail: {:?}",
            graph.parity_findings
        );
    };
    assert_eq!(
        (stale.offset, stale.kind),
        (parity_start, ParityFindingKind::Stale)
    );
    assert_eq!(
        (tail.offset, tail.kind),
        (11, ParityFindingKind::UnprotectedTail(parity_start - 11))
    );
}

#[test]
fn damaged_parity_leaves_a_logical_tail_after_the_opener() {
    let bytes = include_bytes!(
        "fixtures/v2/P5/parity-condition-six-first-by-case.own-version-geometry-zero.bin"
    );
    let graph = second(bytes);
    let [first, second, tail] = graph.parity_findings.as_slice() else {
        panic!(
            "expected damage to both parity frames and an unprotected tail: {:?}",
            graph.parity_findings
        );
    };
    assert_eq!(
        (first.offset, first.kind),
        (209, ParityFindingKind::Damaged)
    );
    assert_eq!(
        (second.offset, second.kind),
        (762, ParityFindingKind::Damaged)
    );
    assert_eq!(
        (tail.offset, tail.kind),
        (11, ParityFindingKind::UnprotectedTail(198))
    );
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
fn a_resync_opener_inside_a_complete_failed_frame_restores_later_rows() {
    let bytes = include_bytes!("fixtures/v2/P1/resync-after-torn-swallowed-failing.whole.bin");
    let graph = second(bytes);
    let [resumed] = graph.resynchronizations.as_slice() else {
        panic!("the input contains one scanned opener");
    };
    assert_eq!(resumed.offset, 206);
    assert_eq!(
        resumed.damage,
        Some(FramingDamage::TornTail {
            offset: 114,
            length: 92
        })
    );
    assert!(matches!(graph.rdf.lines().next(), Some(Line::Quad(..))));
}

#[test]
fn a_complete_failed_frame_charges_its_payload_before_scanning() {
    let bytes = include_bytes!("fixtures/v2/P1/resync-order-held-after-g.whole.bin");
    let mut capacity = ReadCapacity {
        items: 6,
        ..ReadCapacity::default()
    };
    // Terms charge 17 bytes and the rejected frame charges one. The held
    // frames after it are discarded without using the decoded-byte budget.
    capacity.payload.decoded_bytes_per_read = 4114;
    let ReadGraph::Second(graph) = read(bytes, capacity).unwrap() else {
        panic!("the input opens a generation 2 file");
    };
    let [resumed] = graph.resynchronizations.as_slice() else {
        panic!("the input contains one scanned opener");
    };
    assert_eq!(resumed.offset, 370);

    capacity.payload.decoded_bytes_per_read = 4113;
    let refused = read(bytes, capacity).unwrap_err();
    assert_eq!(refused.offset, 370);
    assert!(matches!(
        refused.failure,
        ReadFailure::PayloadRefusal(PayloadRefusal::TotalDecoded(4114))
    ));

    // The failed frame and both held items reach the item limit at EOF.
    // The candidate is never charged and the original failure survives.
    capacity.items = 5;
    let refused = read(bytes, capacity).unwrap_err();
    assert_eq!(refused.offset, 114);
    assert!(matches!(refused.failure, ReadFailure::Wire(_)));
}

#[test]
fn a_replaced_healing_witness_leaves_the_previous_frame_broken() {
    let bytes = include_bytes!("fixtures/v2/P1/resync-heal-witness-removed.ample-whole.bin");
    let mut capacity = ReadCapacity::default();
    let broken = read(bytes, capacity).unwrap_err();
    assert_eq!(broken.offset, 114);
    assert!(matches!(broken.failure, ReadFailure::BrokenLink));

    // When the scan cannot admit the new opener, the witness remains.
    capacity.payload.decoded_bytes_per_read = 4114;
    let refused = read(bytes, capacity).unwrap_err();
    assert_eq!(refused.offset, 316);
    assert!(matches!(
        refused.failure,
        ReadFailure::PayloadRefusal(PayloadRefusal::TotalDecoded(4184))
    ));
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
    let damaged = second(damaged_payload);
    assert_eq!(damaged.stored_digest_mismatches, [opener_len]);
    let [failure] = damaged.frame_failures.as_slice() else {
        panic!("expected one damaged compressed payload");
    };
    assert_eq!(failure.offset, opener_len);
    assert!(matches!(failure.failure, ReadFailure::PayloadDamage(_)));

    let mut raw_payload = frame.field(FrameField::Payload);
    let body = head::read_bytes(&mut raw_payload).unwrap();
    let payload_start = body.as_ptr().addr() - source.as_ptr().addr();
    let mut bad_header = source.to_vec();
    bad_header[payload_start] ^= 1;
    let damaged = second(&bad_header);
    assert_eq!(damaged.stored_digest_mismatches, [opener_len]);
    let [failure] = damaged.frame_failures.as_slice() else {
        panic!("expected one malformed compression header");
    };
    assert_eq!(failure.offset, opener_len);
    assert!(matches!(
        failure.failure,
        ReadFailure::Wire(error) if error.reason == MalformedReason::CodecConstraint
    ));
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
