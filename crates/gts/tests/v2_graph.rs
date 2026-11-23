// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Generation dispatch and the GTS v2 RDF graph entry points.

use core::num::NonZeroU64;

use purrdf_gts::v2::{
    ContentError, FileEnd, FrameField, FramingDamage, Header, IndexStorageItem,
    IndexStorageProblem, Item, ItemKind, Line, Lineage, LineageFinding, MalformedReason,
    ParityFindingKind, ParityRepairFailure, PayloadDamage, PayloadRefusal, RdfGraph, ReadCapacity,
    ReadFailure, ReadGraph, SegmentWriteError, SegmentWriter, SignatureStatus, SignatureUnverified,
    Term, UnknownFrameType, ValueName, read, write_header,
};
use purrdf_lex::cbor::head;

fn second(bytes: &[u8]) -> RdfGraph<'_> {
    let ReadGraph::Second(graph) = read(bytes, ReadCapacity::default()).unwrap() else {
        panic!("the fixture opens a generation 2 file");
    };
    graph
}

fn relink_frame(bytes: &mut [u8], predecessor: &[u8; 32]) -> [u8; 32] {
    let item = Item::first(bytes).unwrap().unwrap();
    let frame = item.frame().unwrap().unwrap();
    let identifier = frame
        .check_link(predecessor, None)
        .computed_identifier
        .unwrap();
    let end = item.bytes().len() - frame.field(FrameField::Signatures).len();
    bytes[end - identifier.len()..end].copy_from_slice(&identifier);
    identifier
}

#[test]
fn unreadable_openers_and_unavailable_hashes_delimit_their_own_segments() {
    let malformed = include_bytes!("fixtures/v2/P1/header-outcome-unreadable-first.bin");
    let unavailable = include_bytes!("fixtures/v2/P1/lineage-previous-links-unavailable.bin");
    let first = Item::first(unavailable).unwrap().unwrap();
    let opener_end = first.bytes().len();
    let supported = &unavailable[opener_end..];

    let version = NonZeroU64::new(2).unwrap();
    for (opener, unavailable_hash) in [(malformed.as_slice(), false), (first.bytes(), true)] {
        let mut writer = SegmentWriter::new(Vec::new()).unwrap();
        writer.add_blob(b"skipped blob", None).unwrap();
        let written = writer.into_inner();
        let written_opener = Item::first(&written).unwrap().unwrap();
        let mut bytes = opener.to_vec();
        bytes.extend_from_slice(&written[written_opener.bytes().len()..]);
        bytes.push(0);
        bytes.extend_from_slice(supported);
        let graph = second(&bytes);

        let [segment] = graph.skipped_segments.as_slice() else {
            panic!("expected one skipped segment");
        };
        assert_eq!(
            (segment.ordinal, segment.opener.offset, segment.items),
            (0, 0, 3)
        );
        assert_eq!(graph.segment_authentication[0].segment, 1);
        assert_eq!(graph.blobs.len(), 0);
        assert_eq!(graph.frame_failures.len(), 0);
        assert_eq!(graph.opaque_items.len(), 0);
        assert_eq!(graph.lineage_findings, [] as [(usize, LineageFinding); 0]);
        assert_eq!(graph.end, FileEnd::NoCommit);
        if unavailable_hash {
            assert_eq!(graph.unsupported_registries, [(0, version)]);
        } else {
            assert_eq!(graph.unsupported_registries, [] as [(usize, NonZeroU64); 0]);
        }
        let error = graph.into_complete_rdf().unwrap_err();
        assert_eq!(error.offset, 0);
        if unavailable_hash {
            assert!(matches!(error.failure, ReadFailure::UnavailableHash(code) if code.get() == 9));
        } else {
            assert!(matches!(
                error.failure,
                ReadFailure::Wire(wire) if wire.reason == MalformedReason::UnregisteredValue
            ));
        }
    }

    let graph = second(include_bytes!(
        "fixtures/v2/P1/links-unavailable-lineage-not-compared.bin"
    ));
    assert_eq!(graph.rdf.lines().count(), 1);
    assert_eq!(graph.skipped_segments[0].ordinal, 1);
    assert_eq!(graph.lineage_findings, [] as [(usize, LineageFinding); 0]);
    assert_eq!(graph.end, FileEnd::NoCommit);

    let valid = include_bytes!("fixtures/v2/P1/header-minimal.bin");
    let mut bytes = valid.to_vec();
    bytes.extend_from_slice(first.bytes());
    assert_eq!(
        second(&bytes).lineage_findings,
        [(valid.len(), LineageFinding::Begins)]
    );
}

#[test]
fn skipped_segments_count_only_the_retained_undelimitable_regions() {
    let unavailable = include_bytes!("fixtures/v2/P1/lineage-previous-links-unavailable.bin");
    let opener = Item::first(unavailable).unwrap().unwrap();
    let clean = include_bytes!("fixtures/v2/P1/resync-isolation-links-unavailable.clean-300.bin");
    let mut prefix = opener.bytes().to_vec();
    prefix.extend([0x82, 6]);
    head::write_bytes(&mut prefix, &[0; 273]).unwrap();
    assert_eq!(prefix.len(), 289);

    let mut clean_item = [0; 11];
    clean_item[..3].copy_from_slice(&[0x82, 6, 0x48]);
    // This string is torn at the join and rejects the array that follows.
    let mut torn = [0x40; 11];
    torn[0] = 0x5f;
    for (region, items, damage) in [
        (clean_item, 3, None),
        (
            [0xff; 11],
            3,
            Some(FramingDamage::Undelimitable {
                offset: 289,
                length: 11,
            }),
        ),
        (
            torn,
            2,
            Some(FramingDamage::TornTail {
                offset: 289,
                length: 11,
            }),
        ),
    ] {
        let mut bytes = prefix.clone();
        bytes.extend(region);
        bytes.extend_from_slice(&clean[300..]);
        let graph = second(&bytes);
        assert_eq!(graph.skipped_segments[0].items, items);
        assert_eq!(graph.rdf.lines().count(), 1);
        assert_eq!(
            graph.resynchronizations.len(),
            usize::from(damage.is_some())
        );
        if let Some(resumed) = graph.resynchronizations.first() {
            assert_eq!(resumed.offset, 300);
            assert_eq!(resumed.damage, damage);
        }
    }
}

#[test]
fn newer_registry_reports_opaque_items_and_checks_unknown_signature_algorithms() {
    let opener = include_bytes!("fixtures/v2/P1/header-registry-newer.bin");
    let version = NonZeroU64::new(2).unwrap();
    let graph = second(opener);
    assert_eq!(graph.unsupported_registries, [(0, version)]);
    graph.into_complete_rdf().unwrap();

    let signed = include_bytes!("fixtures/v2/P2/registry__signature__unknown.bin");
    let mut bytes = opener.to_vec();
    bytes.extend_from_slice(&[0x81, 6]);
    bytes.extend_from_slice(&signed[opener.len()..]);
    let graph = second(&bytes);
    assert_eq!(graph.unsupported_registries, [(0, version)]);
    let [opaque] = graph.opaque_items.as_slice() else {
        panic!("expected one opaque item");
    };
    assert_eq!(opaque.offset, opener.len());
    assert!(matches!(
        opaque.failure,
        ReadFailure::Item(ItemKind::Unknown(6))
    ));
    assert_eq!(graph.blobs.len(), 1);
    assert_eq!(graph.end, FileEnd::AtValidCommit);
    assert_eq!(
        graph.signatures[0].checks[0].status,
        SignatureStatus::Unverified(SignatureUnverified::UnknownAlgorithm)
    );
    assert!(graph.into_complete_rdf().is_err());

    bytes[opener.len() + 1] = 0;
    let graph = second(&bytes);
    assert!(matches!(
        graph.opaque_items[0].failure,
        ReadFailure::Classification(MalformedReason::UnregisteredValue)
    ));
    assert_eq!(graph.end, FileEnd::AtValidCommit);
}

#[test]
fn unknown_critical_frame_stops_only_its_segment_and_keeps_payload_checks() {
    let blob_file = include_bytes!("fixtures/v2/P2/registry__signature__unknown.bin");
    let opener = Item::first(blob_file).unwrap().unwrap();
    let blob = Item::first(&blob_file[opener.bytes().len()..])
        .unwrap()
        .unwrap();
    let envelope = blob.frame().unwrap().unwrap();
    let digest = blob.bytes().len() - envelope.field(FrameField::Signatures).len() - 32;
    let mut next = SegmentWriter::new(Vec::new()).unwrap();
    let after_reset = ValueName::of(b"next segment");
    next.add_blob(b"next segment", None).unwrap();
    next.add_commit().unwrap();
    let next = next.into_inner();

    for (source, expected) in [
        (
            include_bytes!("fixtures/v2/P1/frame-unknown-noncritical.bin").as_slice(),
            UnknownFrameType::Noncritical(99),
        ),
        (
            include_bytes!("fixtures/v2/P1/frame-unknown-critical.bin").as_slice(),
            UnknownFrameType::Critical(99),
        ),
    ] {
        let fixture = second(source);
        let [frame] = fixture.frame_failures.as_slice() else {
            panic!("expected one unsupported frame type");
        };
        assert!(matches!(frame.failure, ReadFailure::UnknownFrame(kind) if kind == expected));

        let unknown = Item::first(&source[frame.offset..]).unwrap().unwrap();
        let unknown_envelope = unknown.frame().unwrap().unwrap();
        let predecessor = unknown_envelope.field(FrameField::Identifier)[2..]
            .try_into()
            .unwrap();
        let blob_offset = frame.offset + unknown.bytes().len();
        let mut bytes = source[..blob_offset].to_vec();
        bytes.extend_from_slice(blob.bytes());
        relink_frame(&mut bytes[blob_offset..], predecessor);
        bytes.extend_from_slice(&next);
        let graph = second(&bytes);
        assert_eq!(graph.blobs.last().unwrap().name, after_reset);
        assert_eq!(graph.end, FileEnd::AtValidCommit);
        if matches!(expected, UnknownFrameType::Noncritical(_)) {
            assert_eq!(graph.stopped_frames, [] as [usize; 0]);
            assert_eq!(graph.blobs.len(), 2);
            continue;
        }
        assert_eq!(graph.stopped_frames, [blob_offset]);
        assert_eq!(graph.blobs.len(), 1);

        let payload_end = blob_offset + digest - 2;
        bytes[payload_end - 1] ^= 1;
        let graph = second(&bytes);
        assert_eq!(graph.stopped_frames, [blob_offset]);
        assert!(graph.frame_failures.iter().any(|failure| {
            failure.offset == blob_offset
                && matches!(
                    failure.failure,
                    ReadFailure::PayloadDamage(PayloadDamage::PayloadDigest)
                )
        }));
        assert_eq!(graph.blobs.len(), 1);
        assert_eq!(graph.blobs[0].name, after_reset);
    }
}

#[test]
fn invalid_flags_do_not_stop_the_fold_and_stopped_indexes_are_checked() {
    let malformed =
        include_bytes!("fixtures/v2/P1/frame-unknown-transparent-not-bool.critical-true.bin");
    let opener = Item::first(malformed).unwrap().unwrap();
    let frame = Item::first(&malformed[opener.bytes().len()..])
        .unwrap()
        .unwrap();
    let malformed_end = opener.bytes().len() + frame.bytes().len();
    let mut bytes = malformed[..malformed_end].to_vec();
    let predecessor = frame
        .frame()
        .unwrap()
        .unwrap()
        .field(FrameField::Identifier)[2..]
        .try_into()
        .unwrap();
    let mut writer = SegmentWriter::new(Vec::new()).unwrap();
    writer.add_blob(b"still folded", None).unwrap();
    let written = writer.into_inner();
    let written_opener = Item::first(&written).unwrap().unwrap();
    bytes.extend_from_slice(&written[written_opener.bytes().len()..]);
    relink_frame(&mut bytes[malformed_end..], predecessor);
    let graph = second(&bytes);
    assert_eq!(graph.stopped_frames, [] as [usize; 0]);
    assert_eq!(graph.blobs.len(), 1);
    assert!(matches!(
        graph.frame_failures.as_slice(),
        [error] if matches!(error.failure, ReadFailure::Wire(wire) if wire.reason == MalformedReason::WrongType)
    ));

    for (bytes, type_runs_match) in [
        (
            include_bytes!("fixtures/v2/P5/index-after-stop.exact.bin").as_slice(),
            true,
        ),
        (
            include_bytes!("fixtures/v2/P5/index-after-stop.wrong-type-runs.bin").as_slice(),
            false,
        ),
    ] {
        let graph = second(bytes);
        let [index] = graph.index_checks.as_slice() else {
            panic!("expected the stopped segment's index check");
        };
        assert!(graph.stopped_frames.contains(&index.offset));
        let prefix = index.prefix.as_ref().unwrap();
        assert!(prefix.head_matches);
        assert!(prefix.root_matches);
        assert_eq!(prefix.type_runs_match, type_runs_match);
    }
}

#[test]
fn a_stopped_newer_parity_geometry_is_opaque_without_a_geometry_finding() {
    let graph = second(include_bytes!(
        "fixtures/v2/P1/parity-later-version-after-stop.bin"
    ));
    assert_eq!(graph.stopped_frames, [289]);
    assert!(!graph.parity_findings.iter().any(|finding| {
        matches!(
            finding.kind,
            ParityFindingKind::UnknownGeometry(_) | ParityFindingKind::Damaged
        )
    }));
}

#[test]
fn newer_enumerations_in_critical_blob_content_stop_the_current_segment() {
    let original = include_bytes!("fixtures/v2/P4/external-ok.bin");
    let original_header = Item::first(original).unwrap().unwrap();
    let external = Item::first(&original[original_header.bytes().len()..])
        .unwrap()
        .unwrap();
    let content = external
        .frame()
        .unwrap()
        .unwrap()
        .field(FrameField::Content);
    let algorithm = content.as_ptr().addr() - external.bytes().as_ptr().addr() + 2;
    assert_eq!(external.bytes()[algorithm], 1);

    let newer = include_bytes!("fixtures/v2/P1/header-registry-newer.bin");
    let header = Header::decode(Item::first(newer).unwrap().unwrap()).unwrap();
    let predecessor = header.identifier().unwrap();
    let mut writer = SegmentWriter::new(Vec::new()).unwrap();
    writer.add_blob(b"still folded", None).unwrap();
    let written = writer.into_inner();
    let written_opener = Item::first(&written).unwrap().unwrap();
    let blob = &written[written_opener.bytes().len()..];
    let blob_offset = newer.len() + external.bytes().len();
    for code in 0..=2 {
        let mut bytes = newer.to_vec();
        bytes.extend_from_slice(external.bytes());
        let frame = &mut bytes[newer.len()..];
        frame[algorithm] = code;
        let identifier = relink_frame(frame, &predecessor);
        bytes.extend_from_slice(blob);
        relink_frame(&mut bytes[blob_offset..], &identifier);
        if code == 0 {
            let error = read(&bytes, ReadCapacity::default()).unwrap_err();
            assert!(matches!(
                error.failure,
                ReadFailure::Wire(wire) if wire.reason == MalformedReason::UnregisteredValue
            ));
        } else {
            let graph = second(&bytes);
            if code == 1 {
                assert_eq!(graph.external_blobs.len(), 1);
                assert_eq!(graph.blobs.len(), 1);
                graph.into_complete_rdf().unwrap();
            } else {
                assert_eq!(graph.external_blobs.len(), 0);
                assert_eq!(graph.blobs.len(), 0);
                assert_eq!(graph.stopped_frames, [blob_offset]);
                assert!(matches!(
                    graph.into_complete_rdf().unwrap_err().failure,
                    ReadFailure::Content(ContentError::UnknownEnumeration { code: 2, .. })
                ));
            }
        }
    }

    let original = include_bytes!("fixtures/v2/P2/registry__unknown__member_kind.bin");
    let original_header = Item::first(original).unwrap().unwrap();
    let pack = Item::first(&original[original_header.bytes().len()..])
        .unwrap()
        .unwrap();
    let error = read(original, ReadCapacity::default()).unwrap_err();
    assert!(matches!(
        error.failure,
        ReadFailure::Wire(wire) if wire.reason == MalformedReason::UnregisteredValue
    ));
    let mut bytes = newer.to_vec();
    bytes.extend_from_slice(pack.bytes());
    relink_frame(&mut bytes[newer.len()..], &predecessor);
    assert!(matches!(
        second(&bytes).into_complete_rdf().unwrap_err().failure,
        ReadFailure::Content(ContentError::UnknownEnumeration { code: 5, .. })
    ));
}

#[test]
fn file_end_uses_the_last_valid_commit_and_retained_item_classes() {
    for (input, expected) in [
        (
            include_bytes!("fixtures/v2/P7/file-end-no-commit.bin").as_slice(),
            FileEnd::NoCommit,
        ),
        (
            include_bytes!("fixtures/v2/P7/file-end-at-valid-commit.bin").as_slice(),
            FileEnd::AtValidCommit,
        ),
        (
            include_bytes!("fixtures/v2/P7/file-end-opaque-after-commit.bin").as_slice(),
            FileEnd::AfterCommit,
        ),
        (
            include_bytes!("fixtures/v2/P7/file-end-region-after-commit.bin").as_slice(),
            FileEnd::AfterCommit,
        ),
        (
            include_bytes!("fixtures/v2/P7/end-locator-own-grammar.bin").as_slice(),
            FileEnd::AtValidCommit,
        ),
        (
            include_bytes!("fixtures/v2/P7/end-frame-no-effective-identifier.bin").as_slice(),
            FileEnd::AfterCommit,
        ),
    ] {
        assert_eq!(second(input).end, expected);
    }

    let mut writer = SegmentWriter::new(Vec::new()).unwrap();
    writer.add_commit().unwrap();
    let mut bytes = writer.into_inner();
    bytes.extend_from_slice(&[0x98, 0x02, 0x03, 0x01]);
    let graph = second(&bytes);
    assert_eq!(graph.opaque_items.len(), 1);
    assert_eq!(graph.end, FileEnd::AtValidCommit);
    graph.into_complete_rdf().unwrap();

    let source = include_bytes!("fixtures/v2/P7/file-end-at-valid-commit.bin");
    let mut malformed_commit = source.to_vec();
    assert_eq!(malformed_commit.pop(), Some(0x80));
    malformed_commit.extend_from_slice(&[0x81, 0x00]);
    let graph = second(&malformed_commit);
    assert!(
        matches!(graph.frame_failures.as_slice(), [error] if matches!(error.failure, ReadFailure::Wire(_)))
    );
    assert_eq!(
        graph
            .into_complete_rdf()
            .unwrap()
            .lines()
            .collect::<Vec<_>>(),
        second(source).rdf.lines().collect::<Vec<_>>()
    );
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
fn index_content_claims_compare_against_their_covered_rows_and_members() {
    let intact = include_bytes!("fixtures/v2/P5/index-intact.bin");
    let pack = include_bytes!("fixtures/v2/P7/idx-unit-pack.bin");
    for (bytes, directory, members) in [
        (intact.as_slice(), true, true),
        (
            include_bytes!("fixtures/v2/P5/index-each-claim.graph_directory.bin").as_slice(),
            false,
            true,
        ),
        (
            include_bytes!("fixtures/v2/P5/index-each-claim.pack_members.bin").as_slice(),
            true,
            false,
        ),
        (pack.as_slice(), true, true),
    ] {
        let graph = second(bytes);
        let prefix = graph.index_checks[0].prefix.as_ref().unwrap();
        assert_eq!(
            (prefix.graph_directory_matches, prefix.pack_members_match),
            (Some(directory), Some(members))
        );
    }

    let (_, damaged_pack) = damage_first_payload(pack);
    let graph = second(&damaged_pack);
    let prefix = graph.index_checks[0].prefix.as_ref().unwrap();
    assert_eq!(
        (prefix.graph_directory_matches, prefix.pack_members_match),
        (Some(true), None)
    );

    let damaged_rows = damage_payload_at(intact, 152);
    let graph = second(&damaged_rows);
    let prefix = graph.index_checks[0].prefix.as_ref().unwrap();
    assert_eq!(
        (prefix.graph_directory_matches, prefix.pack_members_match),
        (None, Some(true))
    );
    let refusal = graph.into_complete_rdf().unwrap_err();
    assert_eq!(refusal.offset, 152);
    assert!(matches!(refusal.failure, ReadFailure::PayloadDamage(_)));
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
            let mut capacity = ReadCapacity::default();
            capacity.payload.decoded_bytes_per_read = 90;
            let ReadGraph::Second(graph) = read(&changed, capacity).unwrap() else {
                panic!("expected both checks of the broken index");
            };
            assert!(matches!(
                graph.index_checks[0].prefix,
                Err(ReadFailure::BrokenLink)
            ));
            assert!(graph.frame_failures.iter().any(|error| {
                error.offset == index_offset
                    && matches!(
                        error.failure,
                        ReadFailure::PayloadRefusal(PayloadRefusal::TotalDecoded(206))
                    )
            }));
            assert_eq!(graph.index_storage_findings, []);
            graph.into_complete_rdf().unwrap();
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
fn an_unrecoverable_tail_preserves_the_checked_prefix_and_its_repairs() {
    let damaged = include_bytes!("fixtures/v2/P5/parity-damaged-shard.bin");
    let intact = include_bytes!("fixtures/v2/P5/parity-base.bin");
    let offset = damaged.len();
    for (tail, expected, end) in [
        (
            [0x8b, 0x02],
            FramingDamage::TornTail { offset, length: 2 },
            FileEnd::Torn,
        ),
        (
            [0xff, 0x00],
            FramingDamage::Undelimitable { offset, length: 2 },
            FileEnd::Undelimitable,
        ),
    ] {
        let input = [damaged.as_slice(), tail.as_slice()].concat();
        let graph = second(&input);
        assert_eq!(graph.framing_tail, Some(expected));
        assert_eq!(graph.end, end);
        assert_eq!(graph.blobs.len(), 1);
        assert_eq!(graph.parity_recoveries, []);
        let repaired = [intact.as_slice(), tail.as_slice()].concat();
        assert_eq!(graph.repaired.as_deref(), Some(repaired.as_slice()));
    }
}

#[test]
fn a_tail_can_overlap_a_preceding_segments_retained_parity_frame() {
    let intact = include_bytes!("fixtures/v2/P5/parity-base.bin");
    let opener_offset = 970_u16;
    let opener = resync_opener_at(opener_offset);
    let start = usize::from(opener_offset);
    let end = start + opener.len();
    let mut input = intact.to_vec();
    input[start..end].copy_from_slice(&opener);
    input[end] = 0xff;
    input.push(0);

    let graph = second(&input);
    let [resync] = graph.resynchronizations.as_slice() else {
        panic!("expected the opener embedded in the retained parity frame");
    };
    assert_eq!(resync.offset, start);
    assert_eq!(
        graph.framing_tail,
        Some(FramingDamage::Undelimitable {
            offset: end,
            length: input.len() - end,
        })
    );
    let mut expected = intact.to_vec();
    expected.push(0);
    assert_eq!(graph.repaired.as_deref(), Some(expected.as_slice()));

    let mut capacity = ReadCapacity::default();
    capacity.payload.decoded_bytes_per_read = 589;
    for suffix in [&[0][..], &[0x82, 0x03, 0x01]] {
        input.truncate(intact.len());
        input.extend_from_slice(suffix);
        let ReadGraph::Second(graph) = read(&input, capacity).unwrap() else {
            panic!("expected the retained parity item after the opener was refused");
        };
        let refusal = graph.recovery_refusal.unwrap();
        assert_eq!(
            (
                refusal.segment,
                refusal.position,
                refusal.offset,
                refusal.length,
                refusal.requested_total,
            ),
            (0, 4, start, 4096, 4685)
        );
        assert_eq!(graph.resynchronizations, []);
        if suffix.len() == 3 {
            let [finding] = graph.index_storage_findings.as_slice() else {
                panic!("expected the complete malformed locator to retain its class");
            };
            assert_eq!(finding.offset, intact.len());
            assert_eq!(finding.item, IndexStorageItem::Locator);
            assert!(matches!(finding.problem, IndexStorageProblem::Malformed(_)));
            assert_eq!(graph.opaque_items.len(), 0);
        }
    }
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

#[test]
fn a_complete_opaque_item_puts_the_dictionary_in_doubt_after_the_scan() {
    let source =
        include_bytes!("fixtures/v2/P5/recovery-parity-in-resync-segment.undelimitable.bin");
    let intact = &source[..209];
    let row_offset = 114;
    let mut damaged = intact[..row_offset].to_vec();
    damaged.push(0);
    damaged.extend_from_slice(&intact[row_offset..]);

    let mut capacity = ReadCapacity {
        items: 4,
        ..ReadCapacity::default()
    };
    let ReadGraph::Second(graph) = read(&damaged, capacity).unwrap() else {
        panic!("expected the opaque item to count once");
    };
    let [item] = graph.opaque_items.as_slice() else {
        panic!("expected the unclassified complete item to be retained");
    };
    assert_eq!(item.offset, row_offset);
    assert!(matches!(
        item.failure,
        ReadFailure::Classification(MalformedReason::WrongType)
    ));
    assert!(graph.rdf.has_held_rows());
    assert_eq!(graph.rdf.lines().count(), 0);
    let [rows] = graph.frame_failures.as_slice() else {
        panic!("expected the later quad frame to hold its rows");
    };
    assert_eq!(rows.offset, row_offset + 1);
    assert!(matches!(&rows.failure, ReadFailure::Rows(finding)
        if finding.opaque_term && !finding.forward_reference));

    capacity.items -= 1;
    let refusal = read(&damaged, capacity).unwrap_err();
    assert_eq!(refusal.offset, row_offset + 1);
    assert!(matches!(refusal.failure, ReadFailure::ItemCapacity(3)));
}

fn damage_first_payload(intact: &[u8]) -> (usize, Vec<u8>) {
    let offset = Item::first(intact).unwrap().unwrap().bytes().len();
    (offset, damage_payload_at(intact, offset))
}

fn damage_payload_at(intact: &[u8], offset: usize) -> Vec<u8> {
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
    damaged
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
        let graph = second(&next_segment);
        let finding = graph
            .lineage_findings
            .iter()
            .find(|(offset, _)| *offset == damaged.len())
            .map(|(_, finding)| *finding);
        assert_eq!(finding, (!valid).then_some(LineageFinding::HeadMismatch));
    }
}

#[test]
fn parity_recovery_uses_the_resync_opener_and_counts_each_region_as_one_item() {
    let incomplete =
        include_bytes!("fixtures/v2/P5/recovery-parity-in-resync-segment.incomplete-item.bin");
    let undelimitable =
        include_bytes!("fixtures/v2/P5/recovery-parity-in-resync-segment.undelimitable.bin");
    let mut repaired = None;
    for (bytes, torn) in [
        (incomplete.as_slice(), true),
        (undelimitable.as_slice(), false),
    ] {
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
        let ReadGraph::Second(graph) = read(bytes, capacity).unwrap() else {
            panic!("expected the checked prefix at the item limit");
        };
        let offset = 274;
        let length = bytes.len() - offset;
        let tail = if torn {
            FramingDamage::TornTail { offset, length }
        } else {
            FramingDamage::Undelimitable { offset, length }
        };
        assert_eq!(graph.framing_tail, Some(tail));
        assert_eq!(graph.parity_recoveries, []);
    }

    let mut capacity = ReadCapacity::default();
    capacity.payload.decoded_bytes_per_read = 462;
    let ReadGraph::Second(graph) = read(incomplete, capacity).unwrap() else {
        panic!("expected the checked prefix after the parity candidate was refused");
    };
    let refusal = graph.recovery_refusal.unwrap();
    let length = Item::first(&incomplete[586..])
        .unwrap()
        .unwrap()
        .bytes()
        .len();
    assert_eq!(
        (
            refusal.segment,
            refusal.position,
            refusal.offset,
            refusal.length,
            refusal.requested_total
        ),
        (1, 1, 586, length, 463)
    );
    assert_eq!(
        graph.framing_tail,
        Some(FramingDamage::TornTail {
            offset: 274,
            length: incomplete.len() - 274,
        })
    );
    assert_eq!(graph.parity_recoveries, []);
    capacity.payload.decoded_bytes_per_read = 463;
    let ReadGraph::Second(graph) = read(incomplete, capacity).unwrap() else {
        panic!("expected the candidate to pass at the budget boundary");
    };
    assert!(graph.recovery_refusal.is_none());
    assert_eq!(graph.parity_recoveries.len(), 1);
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

    let complete = include_bytes!("fixtures/v2/P1/resync-first-item-torn.whole.bin");
    let ReadGraph::Second(graph) = read(complete, capacity).unwrap() else {
        panic!("expected the first malformed opener to survive the refusal");
    };
    assert_eq!(graph.skipped_segments[0].items, 3);
    let refusal = graph.recovery_refusal.unwrap();
    assert_eq!(
        (
            refusal.segment,
            refusal.position,
            refusal.offset,
            refusal.requested_total
        ),
        (0, 0, 10, 4096)
    );
    assert!(matches!(
        graph.into_complete_rdf().unwrap_err().failure,
        ReadFailure::Wire(_)
    ));

    // With three slots, the opener and its held frames fill capacity.
    capacity.items = 3;
    let ReadGraph::Second(graph) = read(complete, capacity).unwrap() else {
        panic!("expected a skipped generation 2 segment");
    };
    assert_eq!(graph.skipped_segments[0].items, 3);
    let error = graph.into_complete_rdf().unwrap_err();
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
    let ReadGraph::Second(graph) = read(bytes, capacity).unwrap() else {
        panic!("expected the checked prefix at the item limit");
    };
    assert_eq!(graph.resynchronizations, []);
    assert_eq!(
        graph.framing_tail,
        Some(FramingDamage::Undelimitable {
            offset: 114,
            length: bytes.len() - 114
        })
    );

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
    let ReadGraph::Second(graph) = read(bytes, capacity).unwrap() else {
        panic!("expected the result with a refused row payload");
    };
    assert!(graph.recovery_refusal.is_none());
    assert_eq!(graph.resynchronizations.len(), 1);
    let [refused] = graph.frame_failures.as_slice() else {
        panic!("expected one payload refusal");
    };
    assert!(matches!(
        refused.failure,
        ReadFailure::PayloadRefusal(PayloadRefusal::TotalDecoded(4144))
    ));

    let offset = refused.offset;
    let item = Item::first(&bytes[offset..]).unwrap().unwrap();
    let frame = item.frame().unwrap().unwrap();
    let signature_start = offset + item.bytes().len() - frame.field(FrameField::Signatures).len();
    let mut broken = bytes.to_vec();
    broken[signature_start - 1] ^= 1;
    let ReadGraph::Second(graph) = read(&broken, capacity).unwrap() else {
        panic!("expected both checks of the broken row frame");
    };
    let [link, payload] = graph.frame_failures.as_slice() else {
        panic!("expected independent link and payload findings");
    };
    assert_eq!((link.offset, payload.offset), (offset, offset));
    assert!(matches!(link.failure, ReadFailure::BrokenLink));
    assert!(matches!(
        payload.failure,
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
    let ReadGraph::Second(graph) = read(bytes, capacity).unwrap() else {
        panic!("expected the complete items after the refused candidate");
    };
    let refused = graph.recovery_refusal.unwrap();
    assert_eq!(
        (
            refused.segment,
            refused.position,
            refused.offset,
            refused.length,
            refused.requested_total
        ),
        (0, 2, 370, 4096, 4114)
    );
    assert_eq!(graph.resynchronizations, []);
    assert!(
        graph
            .frame_failures
            .iter()
            .any(|error| { error.offset == 114 && matches!(error.failure, ReadFailure::Wire(_)) })
    );
    assert!(graph.frame_failures.iter().any(|error| matches!(
        error.failure,
        ReadFailure::PayloadRefusal(PayloadRefusal::TotalDecoded(_))
    )));

    // The failed frame and both held items reach the item limit at EOF.
    // The candidate is never charged and the original failure survives.
    capacity.items = 5;
    let ReadGraph::Second(graph) = read(bytes, capacity).unwrap() else {
        panic!("expected the complete items at the exact item limit");
    };
    assert!(graph.recovery_refusal.is_none());
    assert!(
        graph
            .frame_failures
            .iter()
            .any(|error| { error.offset == 114 && matches!(error.failure, ReadFailure::Wire(_)) })
    );
}

#[test]
fn a_replaced_healing_witness_leaves_the_previous_frame_broken() {
    let bytes = include_bytes!("fixtures/v2/P1/resync-heal-witness-removed.ample-whole.bin");
    let mut capacity = ReadCapacity::default();
    let graph = second(bytes);
    let [broken] = graph.frame_failures.as_slice() else {
        panic!("expected the replaced witness to leave one broken frame");
    };
    assert_eq!(broken.offset, 114);
    assert!(matches!(broken.failure, ReadFailure::BrokenLink));
    assert_eq!(graph.resynchronizations.len(), 1);

    // When the scan cannot admit the new opener, the witness remains.
    capacity.payload.decoded_bytes_per_read = 4114;
    let ReadGraph::Second(graph) = read(bytes, capacity).unwrap() else {
        panic!("expected the retained healing witness after the refusal");
    };
    let refused = graph.recovery_refusal.unwrap();
    assert_eq!(
        (
            refused.segment,
            refused.position,
            refused.offset,
            refused.length,
            refused.requested_total
        ),
        (0, 3, 316, 4096, 4184)
    );
    assert!(
        graph
            .frame_failures
            .iter()
            .all(|failure| failure.offset != 114)
    );
    let [healed] = graph.blobs.as_slice() else {
        panic!(
            "expected the healed blob to fold, failures: {:?}",
            graph.frame_failures
        );
    };
    assert_eq!(healed.bytes.as_ref(), b"xx");
    let [witness] = graph.frame_failures.as_slice() else {
        panic!("expected the witness's signature grammar finding");
    };
    assert_eq!(witness.offset, 231);
    assert!(
        matches!(
            witness.failure,
            ReadFailure::Wire(error)
                if error.reason == MalformedReason::WrongType && error.offset == 186
        ),
        "{witness:?}"
    );
}

#[test]
fn a_later_commit_exempts_rows_from_the_broken_link_rule() {
    let intact = include_bytes!("fixtures/v2/P3/broken-link-commit-ends.whole.bin");
    let committed = second(intact);
    let no_commit = second(include_bytes!(
        "fixtures/v2/P3/broken-link-commit-ends.no-commit.bin"
    ));
    let doubt_stays = second(include_bytes!(
        "fixtures/v2/P3/broken-link-commit-doubt-stays.whole.bin"
    ));
    assert!(!committed.rdf.has_held_rows());
    assert_eq!(committed.rdf.lines().count(), 3);
    assert!(no_commit.rdf.has_held_rows());
    assert_eq!(no_commit.rdf.lines().count(), 1);
    assert!(doubt_stays.rdf.has_held_rows());
    assert_eq!(doubt_stays.rdf.lines().count(), 2);
    assert!(doubt_stays.frame_failures.iter().any(|failure| {
        failure.offset == 402
            && matches!(&failure.failure, ReadFailure::Rows(findings) if findings.opaque_term)
    }));
    for graph in [&committed, &no_commit, &doubt_stays] {
        assert!(
            graph
                .frame_failures
                .iter()
                .any(|failure| matches!(failure.failure, ReadFailure::BrokenLink))
        );
    }

    // A commit's count and range root can hold even if its empty payload's
    // digest is wrong. Recompute its link after changing the payload digest.
    let last_quad_offset = 399;
    let last_quad = Item::first(&intact[last_quad_offset..]).unwrap().unwrap();
    let commit_offset = last_quad_offset + last_quad.bytes().len();
    let quad = last_quad.frame().unwrap().unwrap();
    let mut identifier = quad.field(FrameField::Identifier);
    let predecessor = head::read_bytes(&mut identifier)
        .unwrap()
        .try_into()
        .unwrap();
    let commit = Item::first(&intact[commit_offset..])
        .unwrap()
        .unwrap()
        .frame()
        .unwrap()
        .unwrap();
    let digest = commit.field(FrameField::PayloadDigest);
    let digest_end = digest.as_ptr().addr() - intact.as_ptr().addr() + digest.len();
    let mut wrong_payload = intact.to_vec();
    wrong_payload[digest_end - 1] ^= 1;
    let changed = Item::first(&wrong_payload[commit_offset..])
        .unwrap()
        .unwrap()
        .frame()
        .unwrap()
        .unwrap();
    let identifier = changed
        .check_link(predecessor, None)
        .computed_identifier
        .unwrap();
    let identifier_end = wrong_payload.len() - 1;
    wrong_payload[identifier_end - identifier.len()..identifier_end].copy_from_slice(&identifier);
    let graph = second(&wrong_payload);
    assert!(!graph.rdf.has_held_rows());
    assert_eq!(graph.rdf.lines().count(), 3);
    assert!(graph.frame_failures.iter().any(|failure| {
        failure.offset == commit_offset
            && matches!(
                &failure.failure,
                ReadFailure::PayloadDamage(PayloadDamage::PayloadDigest)
            )
    }));
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
fn refused_value_remains_a_candidate_for_later_terms() {
    let bytes = include_bytes!("fixtures/v2/P4/rebuild-charge-value-term.bin");
    let admitted = second(bytes);
    assert_eq!(admitted.rdf.lines().count(), 1);
    assert!(!admitted.rdf.has_held_rows());

    let capacity = ReadCapacity {
        rebuilt_bytes: 2,
        ..ReadCapacity::default()
    };
    let ReadGraph::Second(refused) = read(bytes, capacity).unwrap() else {
        panic!("the fixture is generation 2");
    };
    assert!(refused.rdf.has_held_rows());
    let [declaration, row] = refused.frame_failures.as_slice() else {
        panic!("expected the value refusal and the held row");
    };
    assert!(matches!(
        declaration.failure,
        ReadFailure::RebuildCapacity(name) if name.length > 2
    ));
    assert!(matches!(&row.failure, ReadFailure::Rows(findings) if findings.opaque_term));
    assert!(declaration.offset < row.offset);

    let recipe = include_bytes!("fixtures/v2/P4/rebuild-unit-after-recipe.units-first.bin");
    let mut writer = SegmentWriter::new(recipe.to_vec()).unwrap();
    let later = writer.add_value(b"d").unwrap();
    writer.add_commit().unwrap();
    let bytes = writer.into_inner();
    assert!(second(&bytes).frame_failures.is_empty());
    let ReadGraph::Second(refused) = read(&bytes, capacity).unwrap() else {
        panic!("the writer emits generation 2");
    };
    let [first_refusal, later_refusal] = refused.frame_failures.as_slice() else {
        panic!("both declarations are refused after the budget stops");
    };
    assert!(matches!(
        first_refusal.failure,
        ReadFailure::RebuildCapacity(name) if name.length > 2
    ));
    assert!(matches!(
        later_refusal.failure,
        ReadFailure::RebuildCapacity(name) if name == later
    ));
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
    let graph = second(later);
    assert!(graph.blobs.is_empty());
    let [error] = graph.frame_failures.as_slice() else {
        panic!("expected one unavailable unit finding");
    };
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
    let error = second(&changed).into_complete_rdf().unwrap_err();
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
        let error = second(bytes).into_complete_rdf().unwrap_err();
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
    let err = second(&bytes).into_complete_rdf().unwrap_err();
    assert!(matches!(err.failure, ReadFailure::Wire(wire)
        if wire.reason == MalformedReason::UnorderedOrRepeated
            && wire.offset == opener.len() + 1 + mark.len()));
}
