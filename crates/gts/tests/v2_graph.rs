// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Generation dispatch and the GTS v2 RDF graph entry points.

use core::num::NonZeroU64;

use purrdf_core::canonical_labelling::{
    LabellingCapacity, Limit, LoweringRefusal, Quantity, Refusal, ViewError,
};
use purrdf_gts::reader::GtsReadResource;
use purrdf_gts::v2::{
    ContentError, ContentLabellingRefusal, DetectionCapacity, FileEnd, FrameField, FramingDamage,
    Header, IndexStorageItem, IndexStorageProblem, Item, ItemKind, Line, Lineage, LineageFinding,
    LinkState, MalformedReason, ParityFindingKind, ParityRepairFailure, PayloadDamage,
    PayloadOutcome, PayloadRefusal, PayloadUnchecked, RdfFold, RdfGraph, ReadCapacity, ReadFailure,
    ReadGraph, ReadLimits, RebuildState, ResyncPast, ResyncReach, SegmentWriteError, SegmentWriter,
    SignatureStatus, SignatureUnverified, Term, UnknownFrameType, ValueName, read,
    read_with_limits, write_header, write_indexed_rdf_segment,
};
use purrdf_lex::cbor::{DecodeErrorKind, StructuralError, head};

fn second(bytes: &[u8]) -> RdfGraph<'_> {
    second_at_capacity(bytes, ReadCapacity::default())
}

fn second_at_capacity(bytes: &[u8], capacity: ReadCapacity) -> RdfGraph<'_> {
    let ReadGraph::Second(graph) = read(bytes, capacity).unwrap() else {
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
fn long_form_envelope_stays_opaque_and_preserves_the_opener_link() {
    let fixture = include_bytes!("fixtures/v2/P1/frame-test-array-long-form.bin");
    let opener = Item::first(fixture).unwrap().unwrap();
    let identifier = Header::decode(opener).unwrap().identifier().unwrap();
    let graph = second(fixture);
    let [opaque] = graph.opaque_items.as_slice() else {
        panic!("the long-form envelope is the only opaque item");
    };
    assert_eq!(opaque.offset, opener.bytes().len());
    assert!(matches!(
        opaque.failure,
        ReadFailure::Classification(MalformedReason::NotDeterministic)
    ));
    assert!(graph.frame_failures.is_empty());
    assert!(graph.segment_durability[0].uncommitted_tail.is_none());

    let mut writer = SegmentWriter::new(Vec::new()).unwrap();
    writer.add_blob(b"after opaque item", None).unwrap();
    let written = writer.into_inner();
    let written_opener = Item::first(&written).unwrap().unwrap();
    let mut frame = written[written_opener.bytes().len()..].to_vec();
    relink_frame(&mut frame, &identifier);
    let mut extended = fixture.to_vec();
    extended.extend_from_slice(&frame);
    let graph = second(&extended);
    assert!(graph.frame_failures.is_empty());
    assert_eq!(graph.blobs.len(), 1);
}

#[test]
fn unchecked_profiles_are_reported_once_per_readable_segment() {
    let dist = include_bytes!("fixtures/v2/P7/profile__not-checked.profile-dist.bin");
    let unknown = include_bytes!("fixtures/v2/P7/profile__not-checked.profile-99.bin");
    let plain = SegmentWriter::new(Vec::new()).unwrap().into_inner();
    let bytes = [dist.as_slice(), &plain, unknown.as_slice()].concat();
    let graph = second(&bytes);
    let [first, last] = graph.unchecked_profiles.as_slice() else {
        panic!("the dist and unknown profiles are unchecked");
    };
    assert_eq!(
        (first.segment, first.offset, first.profile.get()),
        (0, 0, 1)
    );
    assert_eq!(
        (last.segment, last.offset, last.profile.get()),
        (2, dist.len() + plain.len(), 99),
    );
    assert_eq!(graph.unsupported_registries, [] as [(usize, NonZeroU64); 0]);
    assert!(graph.into_complete_rdf().unwrap().lines().count() > 0);
}

#[test]
fn labelling_size_counts_values_and_nested_candidates() {
    for (fixture, expected) in [
        (
            include_bytes!("fixtures/v2/P3/labelling-size-values.bin").as_slice(),
            3,
        ),
        (
            include_bytes!("fixtures/v2/P3/labelling-size-at-capacity.bin").as_slice(),
            8,
        ),
        (
            include_bytes!("fixtures/v2/P3/lowering-shared-nested-term.bin").as_slice(),
            4,
        ),
    ] {
        assert_eq!(second(fixture).labelling_size(), Some(expected));
    }
}

#[test]
fn labelling_size_counts_rows_before_the_file_union() {
    let once = include_bytes!("fixtures/v2/P3/labelling-size-equal-folds.a.bin").as_slice();
    let twice = include_bytes!("fixtures/v2/P3/labelling-size-equal-folds.b.bin").as_slice();
    for (fixture, expected) in [(once, 1), (twice, 2)] {
        let graph = second(fixture);
        assert_eq!(graph.rdf.lines().count(), 1);
        assert_eq!(graph.labelling_size(), Some(expected));
    }
    let combined = [once, twice].concat();
    let graph = second(&combined);
    assert_eq!(graph.rdf.lines().count(), 1);
    assert_eq!(graph.labelling_size(), Some(3));
}

#[test]
fn canonical_fold_digests_match_the_frozen_rdf_and_record_cases() {
    let cases: &[(&str, &[u8], &str)] = &[
        (
            "terms-language-case-distinct.bin",
            include_bytes!("fixtures/v2/P3/terms-language-case-distinct.bin"),
            "71882498cb465cd1899e49e1c5e95be6833190dead0f2bfc26d2eef007dd9185",
        ),
        (
            "record-binding-payload.bin",
            include_bytes!("fixtures/v2/P3/record-binding-payload.bin"),
            "f61c86e9e46b46bb0e9e700d6eea2046414de00636197dd096ac781b6602e806",
        ),
        (
            "record-blob-declaration-payload.bin",
            include_bytes!("fixtures/v2/P3/record-blob-declaration-payload.bin"),
            "9e465bb2fed64a5a232ddab87aea45d2f08eecbf559c2b2c2ce2414fadd2534f",
        ),
        (
            "record-suppression-payload-each-kind.term.bin",
            include_bytes!("fixtures/v2/P3/record-suppression-payload-each-kind.term.bin"),
            "dcba7df16bea0b1060d6dbb93bae06cc4853d1deebf441b62efeb6caacb92737",
        ),
        (
            "record-suppression-payload-each-kind.quad.bin",
            include_bytes!("fixtures/v2/P3/record-suppression-payload-each-kind.quad.bin"),
            "5e2f3c1ffcd2d196114610050215dcf2f5ed51a8cbaae53a30584e7993088f81",
        ),
        (
            "record-suppression-payload-each-kind.reifier.bin",
            include_bytes!("fixtures/v2/P3/record-suppression-payload-each-kind.reifier.bin"),
            "ffa642259223eda7f41c2152c78684e2b1a395d8b6df41f5e92648b73b2dbbeb",
        ),
        (
            "record-suppression-payload-each-kind.graph.bin",
            include_bytes!("fixtures/v2/P3/record-suppression-payload-each-kind.graph.bin"),
            "ef6b05f48eece7f2e0d6934180bc7f822722a4d98419c3688c2232216b06cc3f",
        ),
        (
            "record-suppression-payload-each-kind.blob.bin",
            include_bytes!("fixtures/v2/P3/record-suppression-payload-each-kind.blob.bin"),
            "4abec9369e801e0340e826fba9c1cbe794ac033ad24555a24a7fabd16dc5b0b4",
        ),
        (
            "record-suppression-payload-each-kind.reifier_binding.bin",
            include_bytes!(
                "fixtures/v2/P3/record-suppression-payload-each-kind.reifier_binding.bin"
            ),
            "e19a212bbbc470cef4db2fa4f12d0625a216d187463dc3ac0633bd3944dbf882",
        ),
        (
            "record-suppression-payload-each-kind.annotation.bin",
            include_bytes!("fixtures/v2/P3/record-suppression-payload-each-kind.annotation.bin"),
            "762fd916c9588ca99e788c43f0cdd2aefe1476fba38660d9d45b794d9c42ad07",
        ),
        (
            "record-suppression-payload-each-kind.binding.bin",
            include_bytes!("fixtures/v2/P3/record-suppression-payload-each-kind.binding.bin"),
            "c561c18521fe961a096ad606f2b51c19d3c9d46122606c93b61d350d80496435",
        ),
        (
            "record-value-payload.bin",
            include_bytes!("fixtures/v2/P3/record-value-payload.bin"),
            "7e9b8b112bc850aea958dcf0ca078c6bc15765881f7c03bfa6ff34db504a31a6",
        ),
        (
            "record-unnamed-slot.bin",
            include_bytes!("fixtures/v2/P3/record-unnamed-slot.bin"),
            "172c9494c28835931c353d53ad0a9c3d762e9ab9d1408398c3b80e9c7c63ec6f",
        ),
        (
            "record-triple-slot.bin",
            include_bytes!("fixtures/v2/P3/record-triple-slot.bin"),
            "c485d86fd38be9ca53e5541cc22defa9521b9859d50576aeefdc1a9f8fa59475",
        ),
        (
            "lowering-shared-nested-term.bin",
            include_bytes!("fixtures/v2/P3/lowering-shared-nested-term.bin"),
            "853c3df985fae8a03b883886688d4ee7e3b4cb6abfc5beb8754f7e3678e72226",
        ),
        (
            "canonical-annotations-sum.source.bin",
            include_bytes!("fixtures/v2/P3/canonical-annotations-sum.source.bin"),
            "107d2c5c119361a3e38695dab12dc664188db3fbf048e683bbdf97b2fdb3bab6",
        ),
        (
            "canonical-annotations-sum.canonical.bin",
            include_bytes!("fixtures/v2/P3/canonical-annotations-sum.canonical.bin"),
            "107d2c5c119361a3e38695dab12dc664188db3fbf048e683bbdf97b2fdb3bab6",
        ),
    ];
    for &(name, bytes, digest) in cases {
        let graph = second(bytes);
        let mut capacity = LabellingCapacity::new(u64::MAX, u64::MAX);
        let result = graph
            .label_content(Limit::new(0), &mut capacity)
            .unwrap()
            .unwrap();
        assert_eq!(
            result.content_digest,
            Some(purrdf_hash::hex::decode_32_canonical(digest).unwrap()),
            "{name}",
        );
        let emitted = result
            .canonical_lines()
            .iter()
            .map(|line| line.len() as u64)
            .sum::<u64>();
        assert_eq!(capacity.total(), emitted, "{name}");
    }
}

#[test]
fn canonical_fold_checks_limits_in_order_and_labels_readable_held_content() {
    let width = second(include_bytes!(
        "fixtures/v2/P3/annotations-count-overflow.bin"
    ));
    assert!(matches!(
        width.label_content(Limit::new(0), &mut LabellingCapacity::new(0, 0)),
        Err(ViewError::Lowering(LoweringRefusal::Size { .. }))
    ));
    assert!(matches!(
        width.label_content(Limit::new(0), &mut LabellingCapacity::new(u64::MAX, 0)),
        Err(ViewError::Lowering(LoweringRefusal::Weight { .. }))
    ));
    assert!(matches!(
        width.label_content(
            Limit::new(0),
            &mut LabellingCapacity::new(u64::MAX, u64::MAX)
        ),
        Err(ViewError::Refused(Refusal::Width {
            quantity: Quantity::Count
        }))
    ));

    let nested = second(include_bytes!(
        "fixtures/v2/P3/lowering-shared-nested-term.bin"
    ));
    let mut capacity = LabellingCapacity::new(4, 270);
    let labelled = nested
        .label_content(Limit::new(0), &mut capacity)
        .unwrap()
        .unwrap();
    assert_eq!(labelled.lines.len(), 3);
    assert_eq!(capacity.total(), 270);
    assert!(matches!(
        nested.label_content(Limit::new(0), &mut LabellingCapacity::new(4, 269)),
        Err(ViewError::Lowering(LoweringRefusal::Weight {
            weight: 270,
            ..
        }))
    ));
    let held = second(include_bytes!("fixtures/v2/P3/labelling-held-fold.bin"));
    assert!(matches!(
        held.label_content(
            Limit::new(0),
            &mut LabellingCapacity::new(u64::MAX, u64::MAX)
        ),
        Err(ViewError::Refused(Refusal::Limit { .. }))
    ));
    let labelled = held
        .label_content(
            Limit::new(1 << 40),
            &mut LabellingCapacity::new(u64::MAX, u64::MAX),
        )
        .unwrap()
        .unwrap();
    assert_eq!(labelled.content_digest, None);
    assert!(!labelled.lines.is_empty());
}

#[test]
fn own_fold_only_resolves_values_from_its_segment() {
    let graph = second(include_bytes!(
        "fixtures/v2/P3/content-digest-own-fold-value-elsewhere.bin"
    ));
    let report = graph
        .label_read(
            Limit::new(0),
            &mut LabellingCapacity::new(u64::MAX, u64::MAX),
        )
        .unwrap();
    let [declaring, referencing] = report.segments.as_slice() else {
        panic!("the vector has two readable segments");
    };
    assert_eq!(declaring.segment, 0);
    assert_eq!(referencing.segment, 1);
    let digest = purrdf_hash::hex::decode_32_canonical;
    assert_eq!(
        *declaring.content_digest.as_ref().unwrap(),
        digest("bf4b40a613c1cc97ababeccefc1c1c810dde52818d8928603f2180376f06fb5e")
    );
    assert_eq!(*referencing.content_digest.as_ref().unwrap(), None);
    assert_eq!(
        report.file.unwrap().unwrap().content_digest,
        digest("32eaa574d4ae4e3e6ac525138ba9413b07a0dffb9f03be55fb84d00196b223dd")
    );
}

#[test]
fn own_folds_charge_the_shared_weight_before_the_file() {
    use purrdf_gts::v2::{CompactionOutcome, CompactionStop, compact, compact_segment};

    let graph = second(include_bytes!(
        "fixtures/v2/P3/labelling-weight-segments.bin"
    ));
    let digest = purrdf_hash::hex::decode_32_canonical;
    let first = digest("f6647933b9cceeb3019f29a32e081a6f50e527cda08eabc85248469f4a1b1d7e");
    let second = digest("a42ff597f065d0796001bc63dc89a1a2976520f81c82217a0c9bfe6793453b21");
    let file = digest("3fcec7b269705554deedfa6e5a74cdce6f7969618664099f6640fb9b5b76e3a0");
    for (limit, admitted) in [(240, 240), (239, 120), (119, 60)] {
        let mut capacity = LabellingCapacity::new(u64::MAX, limit);
        let report = graph.label_read(Limit::new(0), &mut capacity).unwrap();
        assert_eq!(capacity.total(), admitted);
        let [one, two] = report.segments.as_slice() else {
            panic!("the vector has two readable segments");
        };
        assert_eq!(*one.content_digest.as_ref().unwrap(), first);
        if limit == 119 {
            assert!(matches!(
                &two.content_digest,
                Err(ContentLabellingRefusal::Lowering(LoweringRefusal::Weight {
                    weight: 60,
                    total: 60,
                    ..
                }))
            ));
        } else {
            assert_eq!(*two.content_digest.as_ref().unwrap(), second);
        }
        if limit == 240 {
            assert_eq!(report.file.unwrap().unwrap().content_digest, file);
        } else {
            assert!(matches!(report.file, Err(ContentLabellingRefusal::Lowering(
                LoweringRefusal::Weight { weight: 120, total, .. }
            )) if total == admitted));
        }
        let mut fresh = LabellingCapacity::new(u64::MAX, limit);
        let mut sink = Vec::new();
        let compacted = compact(&graph, None, Limit::new(0), &mut fresh, &mut sink).unwrap();
        if limit == 240 {
            assert!(matches!(compacted.outcome, CompactionOutcome::Written(_)));
        } else {
            assert!(matches!(
                compacted.outcome,
                CompactionOutcome::Stopped(CompactionStop::Labelling(
                    ContentLabellingRefusal::Lowering(LoweringRefusal::Weight { total, .. })
                )) if total == admitted
            ));
            assert_eq!(sink, [] as [u8; 0]);
        }
        assert_eq!(fresh.total(), admitted);
    }

    for weight in [119, 120] {
        let mut capacity = LabellingCapacity::new(u64::MAX, weight);
        let mut sink = Vec::new();
        let result = compact_segment(&graph, 1, None, Limit::new(0), &mut capacity, &mut sink)
            .unwrap()
            .unwrap();
        if weight == 119 {
            assert!(matches!(
                result.outcome,
                CompactionOutcome::Stopped(CompactionStop::Labelling(
                    ContentLabellingRefusal::Lowering(LoweringRefusal::Weight { total: 60, .. })
                ))
            ));
            assert_eq!(sink, [] as [u8; 0]);
            assert_eq!(capacity.total(), 60);
        } else {
            assert!(matches!(result.outcome, CompactionOutcome::Written(_)));
            assert_eq!(capacity.total(), 120);
            let graph = second_at_capacity(&sink, ReadCapacity::default());
            let report = graph
                .label_read(
                    Limit::new(0),
                    &mut LabellingCapacity::new(u64::MAX, u64::MAX),
                )
                .unwrap();
            assert_eq!(report.file.unwrap().unwrap().content_digest, second);
        }
    }
}

#[test]
fn compacting_an_own_fold_preserves_local_blank_nodes_and_reports_file_coverage() {
    use purrdf_gts::v2::{
        CanonicalLabellingCheck, CanonicalStructureCheck, CompactionOutcome, compact_segment,
    };

    let fixture = include_bytes!("fixtures/v2/P7/canonical__labels-automorphic.bin");
    let input = [fixture.as_slice(), fixture.as_slice(), &[0x82]].concat();
    let graph = second(&input);
    let limit = Limit::new(u64::MAX);
    let mut budget = LabellingCapacity::new(u64::MAX, u64::MAX);
    let report = graph.label_read(limit, &mut budget).unwrap();
    let expected = *report.segments[1].content_digest.as_ref().unwrap();
    assert!(expected.is_some());
    assert_ne!(report.file.unwrap().unwrap().content_digest, expected);

    let mut budget = LabellingCapacity::new(u64::MAX, u64::MAX);
    let compacted = compact_segment(&graph, 1, None, limit, &mut budget, Vec::new())
        .unwrap()
        .unwrap();
    assert_eq!((compacted.end, compacted.left_out), (FileEnd::Torn, 1));
    let CompactionOutcome::Written(bytes) = compacted.outcome else {
        panic!("the selected own fold is fully readable");
    };
    let report = second(&bytes)
        .label_read(limit, &mut LabellingCapacity::new(u64::MAX, u64::MAX))
        .unwrap();
    assert_eq!(report.file.unwrap().unwrap().content_digest, expected);
    assert_eq!(
        report.segments[0].canonical.structure,
        CanonicalStructureCheck::Verified
    );
    assert_eq!(
        report.segments[0].canonical.labelling,
        CanonicalLabellingCheck::Verified
    );
}

#[test]
fn indexed_writer_closes_the_written_fold_and_checks_labelling_before_output() {
    let mut fold = RdfFold::default();
    let subject = fold
        .intern_term(Term::Blank {
            segment: 8,
            label: b"source".as_slice().into(),
        })
        .unwrap();
    let predicate = fold
        .intern_term(Term::Iri("https://example.org/p".into()))
        .unwrap();
    let object = fold
        .intern_term(Term::Iri("https://example.org/o".into()))
        .unwrap();
    let graph_name = fold
        .intern_term(Term::Iri("https://example.org/g".into()))
        .unwrap();
    let row = [subject, predicate, object];
    fold.add_line(Line::Quad(row, Some(graph_name))).unwrap();
    fold.add_line(Line::Quad(row, None)).unwrap();

    let mut output = Vec::new();
    assert!(matches!(
        write_indexed_rdf_segment(
            &fold,
            Limit::new(0),
            &mut LabellingCapacity::new(1, u64::MAX),
            &mut output
        ),
        Err(SegmentWriteError::Labelling(ViewError::Lowering(
            LoweringRefusal::Size { .. }
        )))
    ));
    assert_eq!(output, [] as [u8; 0]);

    let bytes = write_indexed_rdf_segment(
        &fold,
        Limit::new(0),
        &mut LabellingCapacity::new(2, u64::MAX),
        output,
    )
    .unwrap();
    let graph = second(&bytes);
    assert_eq!(graph.rdf.lines().count(), 2);
    let [index] = graph.index_checks.as_slice() else {
        panic!("one closing index was written");
    };
    let prefix = index.prefix.as_ref().unwrap();
    assert!(prefix.head_matches && prefix.root_matches && prefix.type_runs_match);
    assert_eq!(prefix.graph_directory_matches, Some(true));
    assert_eq!(prefix.pack_members_match, Some(true));
    let parity = graph.frame_wire.last().unwrap();
    assert_eq!(
        parity.frame_type,
        Some(purrdf_gts::v2::FrameType::Parity as u64)
    );
    assert!(parity.offset > index.offset);
    assert_eq!(graph.parity_findings, []);
    assert_eq!(graph.index_storage_findings, []);
    let locator_offset = bytes.len() - 11;
    let locator = &bytes[locator_offset..];
    assert_eq!(&locator[..3], &[0x82, 0x03, 0x1b]);
    assert_eq!(
        u64::from_be_bytes(locator[3..].try_into().unwrap()),
        (locator_offset - index.offset) as u64,
    );
    let report = graph
        .label_read(Limit::new(0), &mut LabellingCapacity::new(2, u64::MAX))
        .unwrap();
    let claims = report.segments[0].closing_index.unwrap();
    assert!(claims.statistics_match && claims.content_digest_matches);
}

#[test]
fn canonical_structure_locates_differences_at_the_upstream_positions() {
    use purrdf_gts::v2::CanonicalStructureDifference::{
        Bindings, BlobRegion, Commit, FrameSequence, Header, Index, Rows, Suppression, Terms,
        ValueRegion,
    };
    use purrdf_gts::v2::{CanonicalDifferenceReason, CanonicalStructureCheck};

    macro_rules! fixture {
        ($name:literal) => {
            include_bytes!(concat!("fixtures/v2/P7/", $name, ".bin")).as_slice()
        };
    }
    let check = |bytes: &[u8]| {
        let graph = second(bytes);
        let report = graph
            .label_read(
                Limit::new(0),
                &mut LabellingCapacity::new(u64::MAX, u64::MAX),
            )
            .unwrap();
        report.segments[0].canonical
    };
    for (bytes, rule, at) in [
        (
            fixture!("canonical__check__first-difference__header.registry-version-two"),
            Header,
            0,
        ),
        (
            fixture!("canonical__check__first-difference__frame_sequence.locator-removed"),
            FrameSequence,
            11,
        ),
        (
            fixture!("canonical__check__first-difference__terms.terms-out-of-order"),
            Terms,
            2,
        ),
        (
            fixture!("canonical__check__first-difference__rows.rows-out-of-order"),
            Rows,
            3,
        ),
        (
            fixture!("canonical__check__first-difference__value_region.values-swapped"),
            ValueRegion,
            1,
        ),
        (
            fixture!("canonical__check__first-difference__blob_region.members-reversed"),
            BlobRegion,
            1,
        ),
        (
            fixture!("canonical__check__first-difference__bindings.entries-swapped"),
            Bindings,
            5,
        ),
        (
            fixture!("canonical__check__first-difference__suppression.entries-swapped"),
            Suppression,
            6,
        ),
        (
            fixture!("canonical__check__first-difference__commit.count-one-too-high"),
            Commit,
            7,
        ),
        (
            fixture!("canonical__check__first-difference__index.frame-count-one-too-high"),
            Index,
            8,
        ),
        (
            fixture!("canonical__check__two-differences.one-position"),
            Index,
            8,
        ),
        (
            fixture!("canonical__check__parity-state.frame-removed"),
            FrameSequence,
            7,
        ),
    ] {
        let actual = check(bytes);
        assert!(
            matches!(actual.structure, CanonicalStructureCheck::Failed { position, difference, .. }
            if position == at && difference == rule),
            "{rule:?} at {at}: {actual:?}"
        );
        let first = actual.first_difference.unwrap();
        assert_eq!(
            (first.position, first.reason),
            (at, CanonicalDifferenceReason::Structure(rule))
        );
    }
    for bytes in [
        fixture!("canonical__check__parity-state.table-altered"),
        fixture!("canonical__check__signature-added"),
        fixture!("canonical__check__first-difference__labelling.label-of-no-position"),
        fixture!("canonical__check__first-difference__content_digest.digest-of-zeros"),
        fixture!("canonical__labels-automorphic"),
    ] {
        assert_eq!(check(bytes).structure, CanonicalStructureCheck::Verified);
    }
}

#[test]
fn canonical_structure_reports_short_parity_after_an_intermediate_index() {
    use purrdf_gts::v2::{CanonicalStructureCheck, CanonicalStructureDifference, FrameType};

    let fixture = include_bytes!("fixtures/v2/P7/canonical__construction.bin");
    let source = second(fixture);
    let index = source
        .frame_wire
        .iter()
        .find(|frame| frame.frame_type == Some(FrameType::Index as u64))
        .unwrap();
    let parity = source
        .frame_wire
        .iter()
        .find(|frame| frame.frame_type == Some(FrameType::Parity as u64))
        .unwrap();
    let item = Item::first(&fixture[index.offset..]).unwrap().unwrap();
    let frame = item.frame().unwrap().unwrap();
    let identifier = frame.field(FrameField::Identifier)[2..].try_into().unwrap();
    let mut bytes = fixture.to_vec();
    bytes.extend_from_slice(item.bytes());
    relink_frame(&mut bytes[fixture.len()..], identifier);
    let graph = second(&bytes);
    assert_eq!(graph.index_checks.len(), 2);
    assert!(!graph.index_checks[1].prefix.as_ref().unwrap().head_matches);
    let report = graph
        .label_read(
            Limit::new(0),
            &mut LabellingCapacity::new(u64::MAX, u64::MAX),
        )
        .unwrap();
    assert_eq!(
        report.segments[0].canonical.structure,
        CanonicalStructureCheck::Failed {
            offset: parity.offset,
            position: parity.position,
            difference: CanonicalStructureDifference::FrameSequence,
        }
    );
}

#[test]
fn canonical_structure_reports_a_healed_ordinary_identifier() {
    use purrdf_gts::v2::{CanonicalStructureCheck, CanonicalStructureDifference, FrameType};

    let fixture = include_bytes!("fixtures/v2/P7/canonical__construction.bin");
    let source = second(fixture);
    let terms = source
        .frame_wire
        .iter()
        .find(|frame| frame.frame_type == Some(FrameType::Terms as u64))
        .unwrap();
    let item = Item::first(&fixture[terms.offset..]).unwrap().unwrap();
    let frame = item.frame().unwrap().unwrap();
    let field = frame.field(FrameField::Identifier);
    let last_byte = field.as_ptr().addr() - fixture.as_ptr().addr() + field.len() - 1;
    let mut damaged = fixture.to_vec();
    damaged[last_byte] ^= 1;
    let graph = second(&damaged);
    assert!(graph.frame_wire.iter().any(|frame| {
        frame.offset == terms.offset
            && frame.link == LinkState::Healed
            && frame.payload == PayloadOutcome::Verified
    }));
    let report = graph
        .label_read(
            Limit::new(0),
            &mut LabellingCapacity::new(u64::MAX, u64::MAX),
        )
        .unwrap();
    assert_eq!(
        report.segments[0].canonical.structure,
        CanonicalStructureCheck::Failed {
            offset: terms.offset,
            position: terms.position,
            difference: CanonicalStructureDifference::Terms,
        }
    );
}

#[test]
fn canonical_structure_counts_only_segment_items_when_the_file_ends_torn() {
    use purrdf_gts::v2::{CanonicalStructureCheck, CanonicalStructureDifference};

    let fixture = include_bytes!("fixtures/v2/P7/canonical__construction.bin");
    let mut bytes = [fixture.as_slice(), fixture.as_slice()].concat();
    let end = bytes.len();
    bytes.push(0x82);
    let read = |bytes| {
        let graph = second(bytes);
        assert_eq!(graph.end, FileEnd::Torn);
        graph
            .label_read(
                Limit::new(0),
                &mut LabellingCapacity::new(u64::MAX, u64::MAX),
            )
            .unwrap()
    };
    let report = read(&bytes);
    let [first, second] = report.segments.as_slice() else {
        panic!("the complete openers delimit two segments");
    };
    assert_eq!(first.canonical.structure, CanonicalStructureCheck::Verified);
    assert_eq!(
        second.canonical.structure,
        CanonicalStructureCheck::Verified
    );

    let truncated = read(&bytes[..end - 1]);
    let missing_locator = end - 11;
    assert!(
        matches!(truncated.segments[1].canonical.structure, CanonicalStructureCheck::Failed {
        offset, difference: CanonicalStructureDifference::FrameSequence, ..
    } if offset == missing_locator)
    );
}

#[test]
fn canonical_structure_keeps_complete_items_that_contain_a_resync() {
    use purrdf_gts::v2::{CanonicalStructureCheck, CanonicalStructureDifference, FrameType};

    let intact = include_bytes!("fixtures/v2/P7/canonical__construction.bin");
    let graph = second(intact);
    let parity = graph.frame_wire.last().unwrap();
    assert_eq!(parity.frame_type, Some(FrameType::Parity as u64));
    let parity_end = parity.offset + parity.length;
    let mut input = intact[..parity_end].to_vec();
    let start = parity.offset + 200;
    let opener = resync_opener_at(u16::try_from(start).unwrap());
    let end = start + opener.len();
    input[start..end].copy_from_slice(&opener);
    input[end] = 0xff;
    input.push(0xff);
    let graph = second(&input);
    assert_eq!(graph.resynchronizations[0].offset, start);
    assert!(graph.frame_wire.iter().any(|frame| frame.segment == 0
        && frame.offset == parity.offset
        && frame.length == parity.length));
    let report = graph
        .label_read(
            Limit::new(0),
            &mut LabellingCapacity::new(u64::MAX, u64::MAX),
        )
        .unwrap();
    assert_eq!(
        report.segments[0].canonical.structure,
        CanonicalStructureCheck::Failed {
            position: parity.position + 1,
            offset: parity_end,
            difference: CanonicalStructureDifference::FrameSequence,
        }
    );

    let mut input = intact[..parity_end].to_vec();
    // The first four octets of the resync are the offsets list's sole length.
    input.extend_from_slice(&[0x82, 0x04, 0x81, 0x1a]);
    let start = input.len();
    input.extend_from_slice(&resync_opener_at(u16::try_from(start).unwrap()));
    let graph = second(&input);
    assert_eq!(graph.resynchronizations[0].offset, start);
    assert!(graph.index_storage_findings.iter().any(|finding| {
        finding.offset == parity_end && finding.item == IndexStorageItem::Offsets
    }));
    let report = graph
        .label_read(
            Limit::new(0),
            &mut LabellingCapacity::new(u64::MAX, u64::MAX),
        )
        .unwrap();
    assert_eq!(
        report.segments[0].canonical.structure,
        CanonicalStructureCheck::Failed {
            position: parity.position + 2,
            offset: start + 4,
            difference: CanonicalStructureDifference::FrameSequence,
        }
    );
}

#[test]
fn canonical_structure_counts_a_dropped_binding_at_its_frame() {
    use purrdf_gts::v2::{
        CanonicalForm, CanonicalStructureCheck, CanonicalStructureDifference, FrameType,
    };

    let source = include_bytes!("fixtures/v2/P3/bindings-forward-reference.bin");
    let original = second(source);
    let mut bytes = Vec::new();
    write_header(&mut bytes, None, Some(CanonicalForm::V1), None).unwrap();
    let header = Header::decode(Item::first(&bytes).unwrap().unwrap()).unwrap();
    let mut predecessor = header.identifier().unwrap();
    let mut binding_offset = 0;
    for kind in [FrameType::Terms, FrameType::Bindings] {
        let frame = original
            .frame_wire
            .iter()
            .find(|frame| frame.frame_type == Some(kind as u64))
            .unwrap();
        let at = bytes.len();
        bytes.extend_from_slice(&source[frame.offset..frame.offset + frame.length]);
        predecessor = relink_frame(&mut bytes[at..], &predecessor);
        if kind == FrameType::Bindings {
            binding_offset = at;
        }
    }
    let graph = second(&bytes);
    assert!(graph.reference_findings[0].forward_reference);
    let report = graph
        .label_read(
            Limit::new(0),
            &mut LabellingCapacity::new(u64::MAX, u64::MAX),
        )
        .unwrap();
    assert_eq!(
        report.segments[0].canonical.structure,
        CanonicalStructureCheck::Failed {
            position: 2,
            offset: binding_offset,
            difference: CanonicalStructureDifference::Bindings,
        }
    );
}

#[test]
fn canonical_structure_counts_an_unreferenced_packed_chunk_before_a_bad_commit() {
    use purrdf_gts::v2::{CanonicalForm, CanonicalStructureCheck, CanonicalStructureDifference};

    let source = include_bytes!("fixtures/v2/P4/pack-compressed.one-frame.bin");
    let mut bytes = Vec::new();
    write_header(&mut bytes, None, Some(CanonicalForm::V1), None).unwrap();
    let pack_offset = bytes.len();
    let header = Header::decode(Item::first(&bytes).unwrap().unwrap()).unwrap();
    let predecessor = header.identifier().unwrap();
    let original_header = Item::first(source).unwrap().unwrap().bytes().len();
    bytes.extend_from_slice(&source[original_header..]);
    let pack = Item::first(&bytes[pack_offset..]).unwrap().unwrap();
    let commit_offset = pack_offset + pack.bytes().len();
    let pack_id = relink_frame(&mut bytes[pack_offset..commit_offset], &predecessor);
    relink_frame(&mut bytes[commit_offset..], &pack_id);

    let graph = second(&bytes);
    assert_eq!(graph.invalid_commits[0].0, commit_offset);
    let [unit] = graph.unreferenced_units.as_slice() else {
        panic!("the folded pack has one unreferenced member");
    };
    assert_eq!((unit.offset, unit.member), (pack_offset, Some(0)));
    let limited = second_at_capacity(
        &bytes,
        ReadCapacity {
            items: 2,
            ..ReadCapacity::default()
        },
    );
    assert_eq!(limited.item_refusal.unwrap().offset, commit_offset);
    assert_eq!(limited.unreferenced_units, [*unit]);
    let report = graph
        .label_read(
            Limit::new(0),
            &mut LabellingCapacity::new(u64::MAX, u64::MAX),
        )
        .unwrap();
    assert_eq!(
        report.segments[0].canonical.structure,
        CanonicalStructureCheck::Failed {
            position: 1,
            offset: pack_offset,
            difference: CanonicalStructureDifference::BlobRegion,
        }
    );
}

#[test]
fn canonical_labelling_accepts_automorphisms_and_locates_label_and_digest_differences() {
    use purrdf_gts::v2::{CanonicalLabellingCheck, CanonicalLabellingDifference, FrameType};

    let check = |graph: &RdfGraph<'_>| {
        let report = graph
            .label_read(
                Limit::new(0),
                &mut LabellingCapacity::new(u64::MAX, u64::MAX),
            )
            .unwrap();
        assert_eq!(report.segments.len(), 1);
        report.segments[0].canonical.labelling
    };
    let automorphic = second(include_bytes!(
        "fixtures/v2/P7/canonical__labels-automorphic.bin"
    ));
    assert_eq!(check(&automorphic), CanonicalLabellingCheck::Verified);

    let labels = include_bytes!(
        "fixtures/v2/P7/canonical__check__first-difference__labelling.label-of-no-position.bin"
    );
    let digest = include_bytes!(
        "fixtures/v2/P7/canonical__check__first-difference__content_digest.digest-of-zeros.bin"
    );
    for (fixture, frame_type, reason) in [
        (
            labels.as_slice(),
            FrameType::Terms,
            CanonicalLabellingDifference::BlankLabels,
        ),
        (
            digest.as_slice(),
            FrameType::Index,
            CanonicalLabellingDifference::ContentDigest,
        ),
    ] {
        let graph = second(fixture);
        let frame = graph
            .frame_wire
            .iter()
            .find(|frame| frame.frame_type == Some(frame_type as u64))
            .unwrap();
        assert_eq!(
            check(&graph),
            CanonicalLabellingCheck::Failed {
                offset: frame.offset,
                position: frame.position,
                difference: reason,
            }
        );
    }
}

#[test]
fn canonical_labelling_observes_claim_and_capacity_before_comparing() {
    use purrdf_gts::v2::{
        CanonicalLabellingCheck::Unchecked, CanonicalLabellingUnchecked, CanonicalStorageCheck,
        CanonicalStructureCheck,
    };

    let not_claimed = include_bytes!("fixtures/v2/P7/canonical__check__not-claimed.bin");
    let index = include_bytes!("fixtures/v2/P7/canonical__check__index-above-capacity.bin");
    let refused = include_bytes!("fixtures/v2/P7/canonical__check__refused.bin");
    let mut small = ReadCapacity::default();
    small.payload.decoded_bytes_per_frame = 203;
    for (graph, expected, structure, storage) in [
        (
            second(not_claimed),
            CanonicalLabellingUnchecked::NotClaimed,
            CanonicalStructureCheck::NotClaimed,
            CanonicalStorageCheck::NotClaimed,
        ),
        (
            second_at_capacity(index, small),
            CanonicalLabellingUnchecked::ExceedsCapacity,
            CanonicalStructureCheck::ExceedsCapacity,
            CanonicalStorageCheck::ExceedsCapacity,
        ),
        (
            second(refused),
            CanonicalLabellingUnchecked::LabellingRefused,
            CanonicalStructureCheck::Failed {
                offset: Item::first(refused).unwrap().unwrap().bytes().len(),
                position: 1,
                difference: purrdf_gts::v2::CanonicalStructureDifference::Terms,
            },
            CanonicalStorageCheck::EncoderNotHeld,
        ),
    ] {
        let report = graph
            .label_read(
                Limit::new(0),
                &mut LabellingCapacity::new(u64::MAX, u64::MAX),
            )
            .unwrap();
        let canonical = report.segments[0].canonical;
        assert_eq!(canonical.labelling, Unchecked(expected));
        assert_eq!(canonical.structure, structure);
        assert_eq!(canonical.storage, storage);
    }
}

#[test]
fn canonical_labelling_skips_an_index_after_the_fold_stops() {
    use purrdf_gts::v2::{CanonicalForm, CanonicalLabellingCheck};

    let fixture = include_bytes!("fixtures/v2/P5/index-after-stop.exact.bin");
    let opener = Item::first(fixture).unwrap().unwrap();
    let mut rest = &fixture[opener.bytes().len()..];
    let mut bytes = Vec::new();
    write_header(&mut bytes, None, Some(CanonicalForm::V1), None).unwrap();
    bytes[3] = 2; // Registry 2 admits the fixture's unknown critical frame.
    let mut predecessor = Header::decode(Item::first(&bytes).unwrap().unwrap())
        .unwrap()
        .identifier()
        .unwrap();
    while let Some(item) = Item::first(rest).unwrap() {
        let at = bytes.len();
        bytes.extend_from_slice(item.bytes());
        predecessor = relink_frame(&mut bytes[at..], &predecessor);
        rest = &rest[item.bytes().len()..];
    }
    let mut capacity = ReadCapacity::default();
    capacity.payload.decoded_bytes_per_frame = 14;
    let graph = second_at_capacity(&bytes, capacity);
    let [index] = graph.index_checks.as_slice() else {
        panic!("the reader retains the stopped index's independent check");
    };
    assert!(graph.stopped_frames.contains(&index.offset));
    assert!(matches!(
        index.prefix,
        Err(ReadFailure::PayloadRefusal(PayloadRefusal::DecodedSize(_)))
    ));
    let report = graph
        .label_read(
            Limit::new(0),
            &mut LabellingCapacity::new(u64::MAX, u64::MAX),
        )
        .unwrap();
    assert_eq!(
        report.segments[0].canonical.labelling,
        CanonicalLabellingCheck::Verified
    );
}

#[test]
fn compaction_matches_the_frozen_canonical_head_and_index() {
    assert_compaction_head(include_bytes!("fixtures/v2/P7/canonical__construction.bin"));
}

#[test]
fn compaction_externalizes_values_at_the_frozen_storage_thresholds() {
    for fixture in [
        include_bytes!("fixtures/v2/P7/canonical__constants__threshold.octets-524287.output.bin")
            .as_slice(),
        include_bytes!("fixtures/v2/P7/canonical__constants__threshold.octets-524288.output.bin")
            .as_slice(),
        include_bytes!(
            "fixtures/v2/P7/canonical__constants__threshold.language-1048575.output.bin"
        )
        .as_slice(),
        include_bytes!(
            "fixtures/v2/P7/canonical__constants__threshold.language-1048576.output.bin"
        )
        .as_slice(),
    ] {
        assert_compaction_head(fixture);
    }
}

fn assert_compaction_head(fixture: &[u8]) {
    use purrdf_gts::v2::{CompactionOutcome, FrameType, compact};

    let source = second(fixture);
    let report = compact(
        &source,
        None,
        Limit::new(0),
        &mut LabellingCapacity::new(u64::MAX, u64::MAX),
        Vec::new(),
    )
    .unwrap();
    let CompactionOutcome::Written(bytes) = report.outcome else {
        panic!("the frozen fold admits compaction");
    };
    let graph = second(&bytes);
    let opener = |bytes: &[u8]| {
        Header::decode(Item::first(bytes).unwrap().unwrap())
            .unwrap()
            .identifier()
    };
    assert_eq!(opener(&bytes), opener(fixture));
    let ids = |bytes: &[u8], graph: &RdfGraph<'_>| {
        graph
            .frame_wire
            .iter()
            .filter(|wire| wire.frame_type != Some(FrameType::Parity as u64))
            .map(|wire| {
                let item = Item::first(&bytes[wire.offset..]).unwrap().unwrap();
                let frame = item.frame().unwrap().unwrap();
                (
                    wire.frame_type,
                    frame.field(FrameField::Identifier).to_vec(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(&bytes, &graph), ids(fixture, &source));
    assert!(graph.frame_failures.is_empty());
    assert_eq!(graph.parity_findings, []);
    assert_eq!(graph.index_storage_findings, []);
    let report = graph
        .label_read(
            Limit::new(0),
            &mut LabellingCapacity::new(u64::MAX, u64::MAX),
        )
        .unwrap();
    let claims = report.segments[0].closing_index.unwrap();
    assert!(claims.statistics_match && claims.content_digest_matches);
}

#[test]
fn compaction_checks_formed_capacity_before_held_rows_and_keeps_stopped_sinks_empty() {
    use purrdf_gts::v2::{CompactionOutcome, CompactionStop, compact};

    let formed = include_bytes!("fixtures/v2/P7/canonical__compaction__formed-refused.bin");
    let missing = include_bytes!("fixtures/v2/P7/canonical__compaction__missing-unit.bin");
    let mut sink = Vec::new();
    let mut capacity = LabellingCapacity::new(u64::MAX, u64::MAX);
    let source = second_at_capacity(
        formed,
        ReadCapacity {
            formed_bytes: 8,
            ..ReadCapacity::default()
        },
    );
    assert!(matches!(
        compact(&source, None, Limit::new(0), &mut capacity, &mut sink)
            .unwrap()
            .outcome,
        CompactionOutcome::Stopped(CompactionStop::TotalFormed)
    ));
    assert_eq!(sink, [] as [u8; 0]);
    assert!(matches!(
        compact(
            &second(missing),
            None,
            Limit::new(0),
            &mut capacity,
            &mut sink
        )
        .unwrap()
        .outcome,
        CompactionOutcome::Stopped(CompactionStop::MissingChunk)
    ));
    assert_eq!(sink, [] as [u8; 0]);
    assert!(matches!(
        compact(
            &second(formed),
            None,
            Limit::new(0),
            &mut capacity,
            &mut sink
        )
        .unwrap()
        .outcome,
        CompactionOutcome::Written(_)
    ));
}

#[test]
fn indexed_writer_places_full_parity_before_rows_and_short_parity_after_index() {
    use purrdf_gts::v2::FrameType::{Commit, Index, Parity, Quads, Terms};

    let mut fold = RdfFold::default();
    let subject = fold
        .intern_term(Term::Iri("https://example.org/s".into()))
        .unwrap();
    let predicate = fold
        .intern_term(Term::Iri("https://example.org/p".into()))
        .unwrap();
    let object = fold
        .intern_term(Term::Language {
            lexical: "x".repeat(4 * 1024 * 1024).into(),
            language: "en".into(),
            direction: None,
        })
        .unwrap();
    fold.add_line(Line::Quad([subject, predicate, object], None))
        .unwrap();
    let bytes = write_indexed_rdf_segment(
        &fold,
        Limit::new(0),
        &mut LabellingCapacity::new(1, u64::MAX),
        Vec::new(),
    )
    .unwrap();
    let graph = second(&bytes);
    let types: Vec<_> = graph
        .frame_wire
        .iter()
        .map(|frame| frame.frame_type)
        .collect();
    assert_eq!(
        types,
        [Terms, Parity, Quads, Commit, Index, Parity].map(|kind| Some(kind as u64))
    );
    assert_eq!(graph.parity_findings, []);
    assert_eq!(graph.index_storage_findings, []);
}

#[test]
fn closing_indexes_compare_digest_and_statistics_independently() {
    let cases: &[(&[u8], bool, bool)] = &[
        (
            include_bytes!("fixtures/v2/P5/index-each-claim.content_digest.bin"),
            true,
            false,
        ),
        (
            include_bytes!("fixtures/v2/P5/index-each-claim.statistics-lines.bin"),
            false,
            true,
        ),
        (
            include_bytes!("fixtures/v2/P5/index-each-claim.statistics-records.bin"),
            false,
            true,
        ),
    ];
    for &(bytes, statistics_match, content_digest_matches) in cases {
        let graph = second(bytes);
        let report = graph
            .label_read(
                Limit::new(0),
                &mut LabellingCapacity::new(u64::MAX, u64::MAX),
            )
            .unwrap();
        let comparison = report.segments[0].closing_index.unwrap();
        assert_eq!(
            (
                comparison.statistics_match,
                comparison.content_digest_matches
            ),
            (statistics_match, content_digest_matches),
        );
        assert_eq!(comparison.offset, graph.index_checks[0].offset);
    }
}

#[test]
fn closing_index_checks_the_absence_of_a_digest_for_held_content() {
    for (bytes, matches) in [
        (
            include_bytes!("fixtures/v2/P5/index-digest-absent.absent.bin").as_slice(),
            true,
        ),
        (
            include_bytes!("fixtures/v2/P5/index-digest-absent.present.bin").as_slice(),
            false,
        ),
    ] {
        let graph = second(bytes);
        let report = graph
            .label_read(
                Limit::new(0),
                &mut LabellingCapacity::new(u64::MAX, u64::MAX),
            )
            .unwrap();
        let segment = &report.segments[0];
        assert_eq!(*segment.content_digest.as_ref().unwrap(), None);
        let check = segment.closing_index.unwrap();
        // The held quad still counts as a written row of predicate ID zero.
        assert!(!check.statistics_match);
        assert_eq!(check.content_digest_matches, matches);
    }
}

#[test]
fn broken_ordinary_parity_does_not_block_closing_index_comparisons() {
    let fixture = include_bytes!("fixtures/v2/P5/parity-wrong-transparent.bin");
    let graph = second(fixture);
    let parity = graph.frame_wire.last().unwrap();
    let end = parity.offset + parity.length;
    let item = Item::first(&fixture[parity.offset..end]).unwrap().unwrap();
    let envelope = item.frame().unwrap().unwrap();
    let identifier = end - envelope.field(FrameField::Signatures).len() - 32;
    let mut bytes = fixture[..end].to_vec();
    bytes[identifier] ^= 1;
    let predecessor = bytes[identifier..identifier + 32].try_into().unwrap();

    let index_offset = graph.index_checks[0].offset;
    let index = Item::first(&fixture[index_offset..]).unwrap().unwrap();
    let mut closing = index.bytes().to_vec();
    relink_frame(&mut closing, &predecessor);
    bytes.extend_from_slice(&closing);

    let graph = second(&bytes);
    let before_index = &graph.frame_wire[graph.frame_wire.len() - 2];
    assert_eq!(before_index.offset, parity.offset);
    assert_eq!(before_index.link, LinkState::Broken);
    let report = graph
        .label_read(
            Limit::new(0),
            &mut LabellingCapacity::new(u64::MAX, u64::MAX),
        )
        .unwrap();
    let check = report.segments[0].closing_index.unwrap();
    assert_eq!(check.offset, end);
    assert!(check.statistics_match);
    assert!(check.content_digest_matches);
}

#[test]
fn index_fold_claims_stay_uncompared_without_an_eligible_own_fold() {
    let mut narrow = ReadCapacity::default();
    narrow.payload.decoded_bytes_per_frame = 150;
    let cases = [
        (
            include_bytes!("fixtures/v2/P5/index-nonterminal-claims.bin").as_slice(),
            ReadCapacity::default(),
            false,
        ),
        (
            include_bytes!("fixtures/v2/P5/index-last-unfolded.bin").as_slice(),
            narrow,
            false,
        ),
        (
            include_bytes!("fixtures/v2/P5/index-labelling-refused.bin").as_slice(),
            ReadCapacity::default(),
            true,
        ),
    ];
    for (bytes, read_capacity, refused) in cases {
        let graph = second_at_capacity(bytes, read_capacity);
        assert!(graph.index_checks.iter().all(|index| index.prefix.is_ok()));
        assert!(!graph.index_checks.is_empty());
        let report = graph
            .label_read(
                Limit::new(0),
                &mut LabellingCapacity::new(u64::MAX, u64::MAX),
            )
            .unwrap();
        assert_eq!(report.segments[0].closing_index, None);
        assert_eq!(report.segments[0].content_digest.is_err(), refused);
    }
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
        assert_eq!(graph.left_out, 2);
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

    let mut prefix = malformed.to_vec();
    prefix.push(0xff);
    let mut limits = ReadLimits::default();
    limits.second_generation.items = 2;
    let ReadGraph::Second(graph) = read_with_limits(&prefix, limits).unwrap() else {
        panic!("the unreadable opener still selects generation 2");
    };
    assert_eq!(graph.end, FileEnd::Undelimitable);
    assert!(graph.item_refusal.is_none());
    assert_eq!(graph.left_out, 1);
    assert_eq!(graph.skipped_segments[0].items, 2);

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
            let graph = second(&bytes);
            assert!(graph.external_blobs.is_empty());
            assert_eq!(graph.blobs.len(), 1);
            let error = graph.into_complete_rdf().unwrap_err();
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
    let graph = second(original);
    assert_eq!(graph.end, FileEnd::AtValidCommit);
    assert!(graph.blobs.is_empty());
    let error = graph.into_complete_rdf().unwrap_err();
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
fn durability_counts_the_last_commit_and_the_following_ordinary_frames() {
    let mut writer = SegmentWriter::new(Vec::new()).unwrap();
    writer.add_commit().unwrap();
    writer.add_blob(b"covered", None).unwrap();
    writer.add_commit().unwrap();
    writer.add_blob(b"tail", None).unwrap();
    writer.add_blob(b"second", None).unwrap();
    let mut bytes = writer.into_inner();
    let mut uncommitted = SegmentWriter::new(Vec::new()).unwrap();
    uncommitted.add_blob(b"first", None).unwrap();
    bytes.extend_from_slice(&uncommitted.into_inner());

    let graph = second(&bytes);
    let [committed, uncommitted] = graph.segment_durability.as_slice() else {
        panic!("expected durability for two segments");
    };
    assert_eq!((committed.segment, committed.committed_frames), (0, 3));
    let tail = committed.uncommitted_tail.unwrap();
    assert_eq!((tail.position, tail.count), (4, 2));
    assert_eq!((uncommitted.segment, uncommitted.committed_frames), (1, 0));
    let tail = uncommitted.uncommitted_tail.unwrap();
    assert_eq!((tail.position, tail.count), (1, 1));
    assert_eq!(graph.blobs.len(), 4);
}

#[test]
fn walked_resync_past_uses_its_own_decoded_byte_count() {
    let bytes = include_bytes!("fixtures/v2/P1/resync-past-not-checked-count.bin");
    let offset = 209;
    for (limit, past) in [(208, ResyncPast::NotChecked), (209, ResyncPast::Matches)] {
        let mut capacity = ReadCapacity::default();
        capacity.payload.decoded_bytes_per_read = limit;
        let ReadGraph::Second(graph) = read(bytes, capacity).unwrap() else {
            panic!("expected a generation 2 graph");
        };
        let [opener] = graph.resync_openers.as_slice() else {
            panic!("expected one resync opener");
        };
        assert_eq!(
            (opener.segment, opener.offset, opener.declared_offset),
            (1, offset, offset as u64)
        );
        assert_eq!(opener.reach, ResyncReach::Walked);
        assert_eq!(opener.past, past);
        assert_eq!(graph.left_out, 0);
        assert_eq!(graph.rdf.lines().count(), 1);
    }

    let mut extended = bytes.to_vec();
    // Copy the opener, replacing its two-byte offset and retaining the digest
    // of the original prefix. Walking accepts the computed opener identifier.
    extended.extend_from_slice(&bytes[offset..235]);
    head::write_head(&mut extended, head::UNSIGNED, bytes.len() as u64).unwrap();
    extended.extend_from_slice(&bytes[237..308]);
    let mut capacity = ReadCapacity::default();
    capacity.payload.decoded_bytes_per_read = bytes.len() as u64;
    let ReadGraph::Second(graph) = read(&extended, capacity).unwrap() else {
        panic!("expected a generation 2 graph");
    };
    assert!(
        graph
            .resync_openers
            .iter()
            .map(|opener| opener.past)
            .eq([ResyncPast::Matches, ResyncPast::Differs,])
    );

    let mut empty_past = bytes.to_vec();
    empty_past.extend_from_slice(&bytes[offset..235]);
    head::write_head(&mut empty_past, head::UNSIGNED, 0).unwrap();
    empty_past.extend_from_slice(&bytes[237..308]);
    for (limit, expected) in [
        (208, [ResyncPast::NotChecked, ResyncPast::NotChecked]),
        (209, [ResyncPast::Matches, ResyncPast::Differs]),
    ] {
        capacity.payload.decoded_bytes_per_read = limit;
        let ReadGraph::Second(graph) = read(&empty_past, capacity).unwrap() else {
            panic!("expected a generation 2 graph");
        };
        assert!(
            graph
                .resync_openers
                .iter()
                .map(|opener| opener.past)
                .eq(expected)
        );
    }

    let displaced = second(&bytes[offset..]);
    let opener = &displaced.resync_openers[0];
    assert_eq!((opener.offset, opener.declared_offset), (0, offset as u64));
    assert_eq!(opener.past, ResyncPast::NotChecked);
    assert_eq!(displaced.rdf.lines().count(), 1);
}

#[test]
fn file_end_uses_the_last_valid_commit_and_retained_item_classes() {
    for (input, expected_end, expected_left_out) in [
        (
            include_bytes!("fixtures/v2/P7/file-end-no-commit.bin").as_slice(),
            FileEnd::NoCommit,
            0,
        ),
        (
            include_bytes!("fixtures/v2/P7/file-end-at-valid-commit.bin").as_slice(),
            FileEnd::AtValidCommit,
            0,
        ),
        (
            include_bytes!("fixtures/v2/P7/file-end-opaque-after-commit.bin").as_slice(),
            FileEnd::AfterCommit,
            3,
        ),
        (
            include_bytes!("fixtures/v2/P7/file-end-region-after-commit.bin").as_slice(),
            FileEnd::AfterCommit,
            1,
        ),
        (
            include_bytes!("fixtures/v2/P7/end-locator-own-grammar.bin").as_slice(),
            FileEnd::AtValidCommit,
            0,
        ),
        (
            include_bytes!("fixtures/v2/P7/end-frame-no-effective-identifier.bin").as_slice(),
            FileEnd::AfterCommit,
            1,
        ),
    ] {
        let graph = second(input);
        assert_eq!(
            (graph.end, graph.left_out),
            (expected_end, expected_left_out)
        );
    }

    let mut writer = SegmentWriter::new(Vec::new()).unwrap();
    writer.add_commit().unwrap();
    let mut bytes = writer.into_inner();
    bytes.extend_from_slice(&[0x98, 0x02, 0x03, 0x01]);
    let graph = second(&bytes);
    assert_eq!(graph.opaque_items.len(), 1);
    assert_eq!(graph.end, FileEnd::AtValidCommit);
    assert_eq!(graph.left_out, 1);
    graph.into_complete_rdf().unwrap();

    let no_identifier = second(include_bytes!(
        "fixtures/v2/P7/end-frame-no-effective-identifier.bin"
    ));
    assert_eq!(no_identifier.segment_durability[0].uncommitted_tail, None);

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
fn a_malformed_envelope_keeps_an_independent_payload_failure() {
    let mut writer = SegmentWriter::new(Vec::new()).unwrap();
    writer.add_blob(b"verified blob", None).unwrap();
    let mut bytes = writer.into_inner();
    let graph = second(&bytes);
    assert_eq!(graph.blobs.len(), 1);
    assert_eq!(graph.frame_wire[0].link, LinkState::Linked);
    assert_eq!(graph.frame_wire[0].payload, PayloadOutcome::Verified);
    drop(graph);

    let header = Item::first(&bytes).unwrap().unwrap();
    let frame_offset = header.bytes().len();
    let item = Item::first(&bytes[frame_offset..]).unwrap().unwrap();
    let payload = item.frame().unwrap().unwrap().field(FrameField::Payload);
    let payload_end = payload.as_ptr_range().end.addr() - bytes.as_ptr().addr();
    bytes[payload_end - 1] ^= 1;
    let graph = second(&bytes);
    assert!(matches!(
        graph.frame_failures.as_slice(),
        [error] if matches!(error.failure, ReadFailure::PayloadDamage(PayloadDamage::PayloadDigest))
    ));

    assert_eq!(bytes.pop(), Some(0x80));
    bytes.extend_from_slice(&[0x81, 0]);
    let graph = second(&bytes);
    let [envelope, payload] = graph.frame_failures.as_slice() else {
        panic!("expected independent envelope and payload findings");
    };
    let [wire] = graph.frame_wire.as_slice() else {
        panic!("expected the retained damaged frame");
    };
    assert_eq!(
        (wire.segment, wire.position, wire.offset),
        (0, 1, frame_offset)
    );
    assert_eq!(wire.length, bytes.len() - frame_offset);
    assert_eq!(
        wire.frame_type,
        Some(purrdf_gts::v2::FrameType::Blob as u64)
    );
    assert_eq!(wire.link, LinkState::Linked);
    assert_eq!(wire.payload, PayloadOutcome::Damaged);
    assert_eq!(wire.stored, None);
    assert_eq!(
        (envelope.offset, payload.offset),
        (frame_offset, frame_offset)
    );
    assert!(matches!(
        envelope.failure,
        ReadFailure::Wire(wire) if wire.reason == MalformedReason::WrongType
    ));
    assert!(matches!(
        payload.failure,
        ReadFailure::PayloadDamage(PayloadDamage::PayloadDigest)
    ));
    assert!(matches!(
        graph.into_complete_rdf().unwrap_err().failure,
        ReadFailure::Wire(_)
    ));
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

    let rows = Item::first(&intact[152..]).unwrap().unwrap();
    let mut excluded = rows.bytes().to_vec();
    assert_eq!(excluded[4], 0xf4);
    excluded[4] = 0xf5;
    let index = second(intact).index_checks[0].offset;
    let extended = [&intact[..index], &excluded, &intact[index..]].concat();
    let graph = second(&extended);
    let prefix = graph.index_checks[0].prefix.as_ref().unwrap();
    assert_eq!(prefix.graph_directory_matches, Some(true));
    assert_eq!(prefix.pack_members_match, Some(true));
    assert!(
        graph
            .frame_failures
            .iter()
            .any(|finding| finding.offset == index)
    );
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
fn a_formed_capacity_refusal_reserves_unreadable_terms_for_later_rows() {
    let bytes = include_bytes!("fixtures/v2/P7/canonical__compaction__formed-refused.bin");
    let accepted = second(bytes);
    assert_eq!(accepted.into_complete_rdf().unwrap().lines().count(), 1);

    let capacity = ReadCapacity {
        formed_bytes: 8,
        ..ReadCapacity::default()
    };
    let ReadGraph::Second(graph) = read(bytes, capacity).unwrap() else {
        panic!("expected a generation 2 graph with held rows");
    };
    assert!(graph.rdf.has_held_rows());
    assert_eq!(graph.rdf.lines().count(), 0);
    assert!(matches!(
        graph.frame_failures.as_slice(),
        [terms, row]
            if matches!(terms.failure, ReadFailure::Content(ContentError::FormedCapacity(9)))
                && matches!(row.failure, ReadFailure::Rows(findings) if findings.opaque_term)
    ));
    assert_eq!(graph.left_out, 1);
    assert!(matches!(
        graph.into_complete_rdf().unwrap_err().failure,
        ReadFailure::Content(ContentError::FormedCapacity(9))
    ));
}

#[test]
fn a_malformed_terms_frame_leaves_formed_capacity_for_later_definitions() {
    let bytes = include_bytes!("fixtures/v2/P3/terms-formed-malformed-first.bin");
    let capacity = ReadCapacity {
        formed_bytes: 5,
        ..ReadCapacity::default()
    };
    let ReadGraph::Second(graph) = read(bytes, capacity).unwrap() else {
        panic!("expected a generation 2 graph with later readable terms");
    };
    assert!(graph.rdf.terms().next().is_some());
    assert!(matches!(
        graph.frame_failures.as_slice(),
        [error]
            if matches!(
                error.failure,
                ReadFailure::Content(ContentError::Malformed(wire))
                    if wire.reason == MalformedReason::InconsistentCounts
            )
    ));
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

    let mut longer = damaged;
    longer.push(0);
    let stopped = second_at_capacity(
        &longer,
        ReadCapacity {
            items: 3,
            ..ReadCapacity::default()
        },
    );
    assert_eq!(stopped.end, FileEnd::NotRead);
    assert_eq!(stopped.left_out, 1);
    assert!(matches!(
        stopped.frame_failures.as_slice(),
        [error] if matches!(error.failure, ReadFailure::PayloadDamage(PayloadDamage::PayloadDigest))
    ));
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
    let graph = second_at_capacity(&damaged, capacity);
    assert_eq!(graph.end, FileEnd::NotRead);
    assert_eq!(graph.opaque_items[0].offset, row_offset);
    assert_eq!(graph.rdf.terms().len(), 0);
    let item_refusal = graph.item_refusal.unwrap();
    assert_eq!(
        (item_refusal.offset, item_refusal.limit),
        (row_offset + 1, 3)
    );
    assert_eq!(item_refusal.length, damaged.len() - item_refusal.offset);
    let refusal = graph.into_complete_rdf().unwrap_err();
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
    let intact_graph = second(intact);
    let index_offset = intact_graph.index_checks[0].offset;
    let parity_offset = 481;
    let mut damaged = intact.to_vec();
    damaged[index_offset] = 0xff;
    let graph = second(&damaged);
    let [recovery] = graph.parity_recoveries.as_slice() else {
        panic!("expected the parity frame to resume its own segment");
    };
    assert_eq!(recovery.offset, parity_offset);
    assert!(recovery.detached);
    let ordinary = intact_graph
        .frame_wire
        .iter()
        .find(|wire| wire.offset == parity_offset)
        .unwrap();
    let rescanned = graph
        .frame_wire
        .iter()
        .find(|wire| wire.offset == parity_offset)
        .unwrap();
    assert_eq!(rescanned, ordinary);
    assert_eq!(rescanned.link, LinkState::Linked);
    assert_eq!(rescanned.payload, PayloadOutcome::Verified);
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

    let mut limits = ReadLimits::default();
    limits.second_generation_nesting_depth = 3;
    let ReadGraph::Second(graph) = read_with_limits(&damaged, limits).unwrap() else {
        panic!("the bounded reader admits the parity candidate at its structural depth");
    };
    assert_eq!(graph.parity_recoveries.len(), 1);
    limits.second_generation_nesting_depth -= 1;
    let error = read_with_limits(&damaged, limits).unwrap_err();
    assert!(matches!(
        error.failure,
        ReadFailure::ItemDepth {
            required: 3,
            limit: 2,
            ..
        }
    ));

    let decoy_depth = 8;
    let mut decoy = vec![0x8b];
    decoy.extend(std::iter::repeat_n(0x81, decoy_depth));
    decoy.push(0);
    let mut with_decoy = damaged[..=index_offset].to_vec();
    with_decoy.extend_from_slice(&decoy);
    with_decoy.extend_from_slice(&damaged[index_offset + 1..]);
    limits.second_generation_nesting_depth = 3;
    let ReadGraph::Second(graph) = read_with_limits(&with_decoy, limits).unwrap() else {
        panic!("a non-parity decoy must not preempt the later parity candidate");
    };
    assert_eq!(
        graph.parity_recoveries[0].offset,
        parity_offset + decoy.len()
    );

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

    let mut limits = ReadLimits::default();
    limits.second_generation_nesting_depth = 3;
    let ReadGraph::Second(graph) = read_with_limits(&resync_first, limits).unwrap() else {
        panic!("the bounded reader admits the later resync opener at its structural depth");
    };
    assert_eq!(graph.resynchronizations[0].offset, index + 1);
    limits.second_generation_nesting_depth -= 1;
    let error = read_with_limits(&resync_first, limits).unwrap_err();
    assert!(matches!(
        error.failure,
        ReadFailure::ItemDepth {
            required: 3,
            limit: 2,
            ..
        }
    ));

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

    let mut limits = ReadLimits::default();
    limits.second_generation.items = 0;
    limits.first_generation.encoded_input_bytes = bytes.len();
    assert!(matches!(
        read_with_limits(bytes, limits).unwrap(),
        ReadGraph::First(_)
    ));
    limits.first_generation.encoded_input_bytes -= 1;
    let error = read_with_limits(bytes, limits).unwrap_err();
    let ReadFailure::FirstGenerationCapacity(refusal) = error.failure else {
        panic!("expected the generation 1 input-byte capacity refusal");
    };
    assert_eq!(
        (
            refusal.resource,
            refusal.required,
            refusal.limit,
            refusal.usage.encoded_input_bytes,
        ),
        (
            GtsReadResource::EncodedInputBytes,
            bytes.len(),
            bytes.len() - 1,
            bytes.len(),
        )
    );
}

#[test]
fn generation_detection_bounds_opener_bytes_work_and_depth() {
    let bytes = include_bytes!("fixtures/v2/P9/reader__first-and-second.bin");
    let opener_bytes = Item::first(bytes).unwrap().unwrap().bytes().len();
    let mut limits = ReadLimits::default();
    limits.detection.encoded_opener_bytes = opener_bytes;
    limits.detection.work = usize::MAX;
    assert!(matches!(
        read_with_limits(bytes, limits).unwrap(),
        ReadGraph::First(_)
    ));

    limits.detection.encoded_opener_bytes -= 1;
    let error = read_with_limits(bytes, limits).unwrap_err();
    assert!(matches!(
        error.failure,
        ReadFailure::DetectionBytes { required, limit, .. }
            if required == opener_bytes && limit == opener_bytes - 1
    ));
    limits.detection.encoded_opener_bytes = 0;
    let error = read_with_limits(bytes, limits).unwrap_err();
    assert!(matches!(
        error.failure,
        ReadFailure::DetectionBytes {
            required: 1,
            limit: 0,
            ..
        }
    ));

    let mut tagged_opener = vec![0xd9, 0xd9, 0xf7, 0xa2, 0x61, b'a', 0x58, 0x40];
    tagged_opener.extend(std::iter::repeat_n(0, 64));
    tagged_opener.extend_from_slice(&[0x61, b'v', 0x01]);
    let tagged_work = tagged_opener.len() * DetectionCapacity::OPENER_WORK_PER_BYTE;
    limits.detection.encoded_opener_bytes = usize::MAX;
    limits.detection.work = tagged_work;
    assert!(matches!(
        read_with_limits(&tagged_opener, limits).unwrap(),
        ReadGraph::First(_)
    ));
    limits.detection.work -= 1;
    let error = read_with_limits(&tagged_opener, limits).unwrap_err();
    assert!(matches!(
        error.failure,
        ReadFailure::DetectionWork { required, limit, .. }
            if required == tagged_work && limit == tagged_work - 1
    ));
    limits.detection.work = 0;
    let error = read_with_limits(&tagged_opener, limits).unwrap_err();
    assert!(matches!(
        error.failure,
        ReadFailure::DetectionWork {
            required: 1,
            limit: 0,
            ..
        }
    ));

    let nested_arrays = 4;
    let exact_depth = nested_arrays + 1;
    let mut deep_opener = vec![0xa2, 0x61, b'a'];
    deep_opener.extend(std::iter::repeat_n(0x81, nested_arrays));
    deep_opener.extend_from_slice(&[0x00, 0x61, b'v', 0x01]);
    limits.detection.encoded_opener_bytes = usize::MAX;
    limits.detection.work = usize::MAX;
    limits.detection.nesting_depth = exact_depth;
    assert!(matches!(
        read_with_limits(&deep_opener, limits).unwrap(),
        ReadGraph::First(_)
    ));
    limits.detection.nesting_depth -= 1;
    let error = read_with_limits(&deep_opener, limits).unwrap_err();
    assert!(matches!(
        &error.failure,
        ReadFailure::DetectionDepth { required, limit, .. }
            if *required == exact_depth && *limit == exact_depth - 1
    ));
    let cause = std::error::Error::source(&error)
        .and_then(|source| source.downcast_ref::<StructuralError>())
        .expect("the native structural depth refusal remains inspectable");
    assert_eq!(
        cause.kind(),
        DecodeErrorKind::Depth {
            limit: exact_depth - 1
        }
    );
    limits.detection.nesting_depth = 0;
    let error = read_with_limits(&deep_opener, limits).unwrap_err();
    assert!(matches!(
        error.failure,
        ReadFailure::DetectionDepth {
            required: 1,
            limit: 0,
            ..
        }
    ));

    let recovered = include_bytes!("fixtures/v2/P1/resync-first-byte-foreign.whole.bin");
    limits.detection.nesting_depth = 2;
    assert!(matches!(
        read_with_limits(recovered, limits).unwrap(),
        ReadGraph::Second(_)
    ));
    limits.detection.nesting_depth = 1;
    let error = read_with_limits(recovered, limits).unwrap_err();
    assert!(matches!(
        error.failure,
        ReadFailure::DetectionDepth {
            required: 2,
            limit: 1,
            ..
        }
    ));

    limits.detection.nesting_depth = 2;
    limits.second_generation_nesting_depth = 1;
    let error = read_with_limits(recovered, limits).unwrap_err();
    assert!(matches!(
        error.failure,
        ReadFailure::ItemDepth {
            required: 2,
            limit: 1,
            ..
        }
    ));
}

#[test]
fn bounded_recovery_and_body_framing_apply_their_nesting_limits() {
    let source = include_bytes!("fixtures/v2/P1/resync-first-item-torn.whole.bin");
    let opener_len = Item::first(source).unwrap().unwrap().bytes().len();
    let held_len = Item::first(&source[opener_len..])
        .unwrap()
        .unwrap()
        .bytes()
        .len();
    let held_depth = held_len - 1;
    let mut deep_held = vec![0x81; held_depth];
    deep_held.push(0);
    let mut damaged = source.to_vec();
    damaged[opener_len..opener_len + held_len].copy_from_slice(&deep_held);

    let mut limits = ReadLimits::default();
    limits.detection.nesting_depth = held_depth;
    assert!(matches!(
        read_with_limits(source, limits).unwrap(),
        ReadGraph::Second(_)
    ));
    let ReadGraph::Second(graph) = read_with_limits(&damaged, limits).unwrap() else {
        panic!("the bounded reader recognizes generation 2");
    };
    assert!(
        graph
            .frame_failures
            .iter()
            .any(|error| matches!(error.failure, ReadFailure::BrokenLink))
    );
    limits.detection.nesting_depth -= 1;
    let error = read_with_limits(&damaged, limits).unwrap_err();
    assert!(matches!(
        &error.failure,
        ReadFailure::DetectionDepth { required, limit, .. }
            if *required == held_depth && *limit == held_depth - 1
    ));

    let mut body = include_bytes!("fixtures/v2/P1/header-minimal.bin").to_vec();
    let body_depth = 8;
    body.extend(std::iter::repeat_n(0x81, body_depth));
    body.push(0);
    limits = ReadLimits::default();
    limits.second_generation_nesting_depth = body_depth;
    assert!(matches!(
        read_with_limits(&body, limits).unwrap(),
        ReadGraph::Second(_)
    ));
    limits.second_generation_nesting_depth -= 1;
    let error = read_with_limits(&body, limits).unwrap_err();
    assert!(matches!(
        &error.failure,
        ReadFailure::ItemDepth { required, limit, .. }
            if *required == body_depth && *limit == body_depth - 1
    ));
    let cause = std::error::Error::source(&error)
        .and_then(|source| source.downcast_ref::<StructuralError>())
        .expect("the native item depth refusal remains inspectable");
    assert_eq!(
        cause.kind(),
        DecodeErrorKind::Depth {
            limit: body_depth - 1
        }
    );

    let opener = include_bytes!("fixtures/v2/P1/header-minimal.bin");
    limits = ReadLimits::default();
    limits.second_generation_nesting_depth = 2;
    assert!(matches!(
        read_with_limits(opener, limits).unwrap(),
        ReadGraph::Second(_)
    ));
    limits.second_generation_nesting_depth -= 1;
    let error = read_with_limits(opener, limits).unwrap_err();
    assert!(matches!(
        error.failure,
        ReadFailure::ItemDepth {
            required: 2,
            limit: 1,
            ..
        }
    ));
}

#[test]
fn bounded_framing_reports_the_item_start_and_retains_the_native_offset() {
    let opener = include_bytes!("fixtures/v2/P1/header-minimal.bin");
    let mut valid = opener.to_vec();
    valid.extend_from_slice(&[0x82, 0x00, 0x00]);
    assert!(matches!(
        read_with_limits(&valid, ReadLimits::default()).unwrap(),
        ReadGraph::Second(_)
    ));

    let mut malformed = opener.to_vec();
    malformed.extend_from_slice(&[0x82, 0x00, 0xff]);
    let ReadGraph::Second(graph) = read_with_limits(&malformed, ReadLimits::default()).unwrap()
    else {
        panic!("the bounded reader recognizes generation 2");
    };
    let error = graph.into_complete_rdf().unwrap_err();
    assert_eq!(error.offset, opener.len());
    let cause = std::error::Error::source(&error)
        .and_then(|source| source.downcast_ref::<StructuralError>())
        .expect("the malformed body item retains its native framing refusal");
    assert_eq!(cause.offset(), 2);
}

#[test]
fn a_partial_recovery_scan_does_not_exhaust_the_larger_detection_window() {
    let bytes = [0x00, 0x00, 0x00];
    let mut limits = ReadLimits::default();
    limits.detection.encoded_opener_bytes = 2;
    limits.detection.work = usize::MAX;
    let error = read_with_limits(&bytes, limits).unwrap_err();
    assert!(matches!(
        error.failure,
        ReadFailure::Generation(purrdf_gts::v2::Generation::Unrecognized)
    ));

    limits.detection.encoded_opener_bytes = 1;
    let error = read_with_limits(&bytes, limits).unwrap_err();
    assert!(matches!(
        error.failure,
        ReadFailure::DetectionBytes {
            required: 2,
            limit: 1,
            ..
        }
    ));
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
        let graph = second_at_capacity(bytes, capacity);
        assert_eq!(graph.end, FileEnd::NotRead);
        assert_eq!(graph.item_refusal.unwrap().limit, items - 1);
        assert!(matches!(
            graph.into_complete_rdf().unwrap_err().failure,
            ReadFailure::ItemCapacity(_)
        ));
    }
}

#[test]
fn a_resync_opener_in_a_damaged_prefix_respects_the_read_limits() {
    let bytes = include_bytes!("fixtures/v2/P1/resync-first-byte-foreign.whole.bin");
    let mut limits = ReadLimits::default();
    limits.second_generation.items = 1;
    let error = read_with_limits(bytes, limits).unwrap_err();
    assert_eq!(error.offset, 0);
    assert!(matches!(error.failure, ReadFailure::Framing(_)));

    // Detection consumed its whole work budget, but the item limit prevents
    // the bounded scanner from running, so the original refusal still wins.
    limits.detection.work = 1;
    let error = read_with_limits(bytes, limits).unwrap_err();
    assert!(matches!(error.failure, ReadFailure::Framing(_)));

    // Once scanning is admitted, the same empty recovery window is genuinely
    // exhausted and reports its typed work refusal with the framing cause.
    limits.second_generation.items = 2;
    let error = read_with_limits(bytes, limits).unwrap_err();
    assert!(matches!(
        &error.failure,
        ReadFailure::DetectionWork {
            required,
            limit: 1,
            ..
        } if *required == 1 + DetectionCapacity::RECOVERY_WORK_PER_BYTE
    ));
    let cause = std::error::Error::source(&error)
        .and_then(|source| source.downcast_ref::<StructuralError>())
        .expect("the exhausted empty window retains its malformed framing cause");
    assert!(matches!(cause.kind(), DecodeErrorKind::Malformed(_)));

    limits = ReadLimits::default();
    limits.second_generation.items = 4;
    limits.second_generation.payload.decoded_bytes_per_read = 4095;
    let error = read_with_limits(bytes, limits).unwrap_err();
    assert_eq!(error.offset, 2);
    assert!(matches!(
        error.failure,
        ReadFailure::PayloadRefusal(PayloadRefusal::TotalDecoded(4096))
    ));

    limits.second_generation.payload = ReadCapacity::default().payload;
    let opener_end = 2 + Item::first(&bytes[2..]).unwrap().unwrap().bytes().len();
    limits.detection.encoded_opener_bytes = opener_end;
    limits.detection.work = usize::MAX;
    assert!(matches!(
        read_with_limits(bytes, limits).unwrap(),
        ReadGraph::Second(_)
    ));
    limits.detection.encoded_opener_bytes -= 1;
    let error = read_with_limits(bytes, limits).unwrap_err();
    assert!(matches!(
        &error.failure,
        ReadFailure::DetectionBytes { required, limit, .. }
            if *required == opener_end && *limit == opener_end - 1
    ));
    let cause = std::error::Error::source(&error)
        .and_then(|source| source.downcast_ref::<StructuralError>())
        .expect("the malformed opener remains the recovery refusal's source");
    assert!(matches!(cause.kind(), DecodeErrorKind::Malformed(_)));

    let recovery_work = 1 + DetectionCapacity::recovery_work(opener_end);
    limits.detection.encoded_opener_bytes = usize::MAX;
    limits.detection.work = recovery_work;
    assert!(matches!(
        read_with_limits(bytes, limits).unwrap(),
        ReadGraph::Second(_)
    ));
    limits.detection.work -= 1;
    let error = read_with_limits(bytes, limits).unwrap_err();
    assert!(matches!(
        error.failure,
        ReadFailure::DetectionWork { required, limit, .. }
            if required == recovery_work && limit == recovery_work - 1
    ));
    limits.detection = ReadLimits::default().detection;

    // The complete failing opener plus its two held frames reach capacity
    // before the candidate can be tested or charged.
    let complete = include_bytes!("fixtures/v2/P1/resync-first-item-torn.whole.bin");
    let complete_opener = Item::first(complete).unwrap().unwrap();
    let candidate_end = 10 + Item::first(&complete[10..]).unwrap().unwrap().bytes().len();
    let complete_work = complete_opener.bytes().len() * DetectionCapacity::OPENER_WORK_PER_BYTE
        + DetectionCapacity::recovery_work(candidate_end);
    limits.second_generation.items = 6;
    limits.detection.encoded_opener_bytes = usize::MAX;
    limits.detection.work = complete_work;
    assert!(matches!(
        read_with_limits(complete, limits).unwrap(),
        ReadGraph::Second(_)
    ));
    limits.detection.work -= 1;
    let error = read_with_limits(complete, limits).unwrap_err();
    assert!(matches!(
        error.failure,
        ReadFailure::DetectionWork { required, limit, .. }
            if required == complete_work && limit == complete_work - 1
    ));
    limits.detection = ReadLimits::default().detection;

    limits.second_generation.items = 4;
    limits.second_generation.payload.decoded_bytes_per_read = 4095;
    let ReadGraph::Second(graph) = read_with_limits(complete, limits).unwrap() else {
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

    limits.second_generation.items = 3;
    let ReadGraph::Second(graph) = read_with_limits(complete, limits).unwrap() else {
        panic!("expected a skipped generation 2 segment");
    };
    assert_eq!(graph.skipped_segments[0].items, 3);
    let error = graph.into_complete_rdf().unwrap_err();
    assert_eq!(error.offset, 0);
    assert!(matches!(error.failure, ReadFailure::Wire(_)));

    limits.second_generation.items = 2;
    let ReadGraph::Second(graph) = read_with_limits(complete, limits).unwrap() else {
        panic!("the bounded reader retains the checked prefix");
    };
    assert_eq!(graph.end, FileEnd::NotRead);
    assert_eq!(graph.skipped_segments[0].items, 2);
    let error = graph.into_complete_rdf().unwrap_err();
    assert_eq!(error.offset, 177);
    assert!(matches!(error.failure, ReadFailure::ItemCapacity(2)));

    // An incomplete first item occupies no slot. The accepted opener
    // fills the one available slot, so the first following item is unread.
    let incomplete = include_bytes!(
        "fixtures/v2/P1/resync-incomplete-item-not-counted.first-item-incomplete.bin"
    );
    limits.second_generation.payload = ReadCapacity::default().payload;
    limits.second_generation.items = 1;
    let ReadGraph::Second(graph) = read_with_limits(incomplete, limits).unwrap() else {
        panic!("the bounded reader retains the checked prefix");
    };
    assert_eq!(graph.end, FileEnd::NotRead);
    assert_eq!(graph.resynchronizations.len(), 1);
    let error = graph.into_complete_rdf().unwrap_err();
    assert_eq!(error.offset, 264);
    assert!(matches!(error.failure, ReadFailure::ItemCapacity(1)));
}

#[test]
fn a_later_undelimitable_region_consumes_one_item_before_recovery() {
    let bytes = include_bytes!("fixtures/v2/P1/resync-item-limit-region.whole.bin");
    let opener_bytes = Item::first(bytes).unwrap().unwrap().bytes().len();
    let mut limits = ReadLimits::default();
    limits.detection.encoded_opener_bytes = opener_bytes;
    limits.detection.work = opener_bytes * DetectionCapacity::OPENER_WORK_PER_BYTE;
    assert!(matches!(
        read_with_limits(bytes, limits).unwrap(),
        ReadGraph::Second(_)
    ));

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
    let wire = graph
        .frame_wire
        .iter()
        .find(|frame| frame.offset == offset)
        .unwrap();
    assert_eq!(wire.link, LinkState::Broken);
    assert_eq!(
        wire.payload,
        PayloadOutcome::Unchecked(PayloadUnchecked::ExceedsCapacity)
    );
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
        let error = second(bytes).into_complete_rdf().unwrap_err();
        assert!(matches!(
            error.failure,
            ReadFailure::Definitions(findings) if findings.missing_values.len() == 1
        ));
    }
}

#[test]
fn a_forward_reference_drops_its_row_and_retains_the_readable_rows() {
    let graph = second(include_bytes!("fixtures/v2/P3/rows-forward-reference.bin"));
    assert_eq!(graph.rdf.lines().count(), 2);
    assert!(!graph.rdf.has_held_rows());
    assert!(matches!(
        graph.frame_failures.as_slice(),
        [error] if matches!(
            error.failure,
            ReadFailure::Rows(findings) if findings.forward_reference && !findings.opaque_term
        )
    ));
    assert!(matches!(
        graph.into_complete_rdf().unwrap_err().failure,
        ReadFailure::Rows(findings) if findings.forward_reference
    ));
}

#[test]
fn refused_value_remains_a_candidate_for_later_terms() {
    let bytes = include_bytes!("fixtures/v2/P4/rebuild-charge-value-term.bin");
    let admitted = second(bytes);
    assert_eq!(admitted.rdf.lines().count(), 1);
    assert!(!admitted.rdf.has_held_rows());
    assert_eq!(admitted.rebuilds[0].state, RebuildState::Verified);

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
    let result = refused.rebuilds[0];
    assert_eq!(result.offset, declaration.offset);
    assert_eq!(result.member, None);
    assert_eq!(
        result.state,
        RebuildState::Refused {
            count: result.name.length
        }
    );

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
    assert_eq!(
        refused.rebuilds[1].state,
        RebuildState::Refused { count: 1 }
    );

    let mut writer = SegmentWriter::new(Vec::new()).unwrap();
    for value in [b"abcdef".as_slice(), b"vwxyz", b"g"] {
        writer.add_value(value).unwrap();
    }
    let bytes = writer.into_inner();
    let graph = second_at_capacity(
        &bytes,
        ReadCapacity {
            rebuilt_bytes: 10,
            ..capacity
        },
    );
    let [admitted, refused, latched] = graph.rebuilds.as_slice() else {
        panic!("expected one outcome for each declaration");
    };
    assert_eq!(admitted.state, RebuildState::Verified);
    for (rebuild, count) in [(refused, 11), (latched, 7)] {
        let state = purrdf_testkit::error_capture::Capture::read(rebuild).child("state");
        assert!(
            matches!(state.field("count"), purrdf_hash::error::ErrorValue::ByteCount(actual) if actual == count)
        );
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
fn annotation_width_refusal_keeps_the_fold_and_its_saturated_count() {
    let bytes = include_bytes!("fixtures/v2/P3/annotations-count-overflow.bin");
    let overflowing_frame = 273;
    let exact = second(&bytes[..overflowing_frame]);
    assert!(!exact.rdf.annotation_width_exceeded());
    let exact = exact.into_complete_rdf().unwrap();
    assert!(exact.lines().any(|line| matches!(
        line, Line::Annotation(_, count, _) if count.get() == u64::MAX
    )));

    let refused = second(bytes);
    assert!(refused.rdf.annotation_width_exceeded());
    assert_eq!(refused.end, FileEnd::AtValidCommit);
    assert_eq!(refused.left_out, 0);
    assert_eq!(
        refused.rdf.lines().collect::<Vec<_>>(),
        exact.lines().collect::<Vec<_>>()
    );
    let error = refused.into_complete_rdf().unwrap_err();
    assert_eq!(error.offset, overflowing_frame);
    assert!(matches!(error.failure, ReadFailure::AnnotationWidth));
}

#[test]
fn one_annotation_frame_rejects_repeated_written_definitions() {
    for bytes in [
        include_bytes!("fixtures/v2/P3/annotations-repeat.bin").as_slice(),
        include_bytes!("fixtures/v2/P3/annotations-repeat-after-resolution.bin").as_slice(),
        include_bytes!("fixtures/v2/P3/annotations-triple-components-written-alike.components-written-alike.bin").as_slice(),
    ] {
        let graph = second(bytes);
        assert!(!graph.rdf.lines().any(|line| matches!(line, Line::Annotation(..))));
        let error = graph.into_complete_rdf().unwrap_err();
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
fn canonical_term_order_matches_the_frozen_dictionary_cases() {
    use purrdf_gts::v2::FrameType;

    macro_rules! case {
        ($name:literal, $violation:literal) => {
            (
                $name,
                include_bytes!(concat!("fixtures/v2/P3/", $name, ".bin")).as_slice(),
                $violation,
            )
        };
    }

    for (name, bytes, expected) in [
        case!("terms-canonical-order-ties.ascending", false),
        case!("terms-canonical-order-ties.swap-2", true),
        case!("terms-canonical-triple-depth.ascending", false),
        case!("terms-canonical-triple-depth.swapped", true),
        case!("terms-canonical-across-frames.arrival-between", false),
        case!("terms-canonical-across-frames.descending-junction", true),
        case!("terms-canonical-follows-opaque", false),
        case!(
            "terms-canonical-skipped-term.skipped-at-the-end-of-the-earlier-frame",
            false
        ),
        case!(
            "terms-canonical-skipped-term.skipped-at-the-end-descending-junction",
            true
        ),
        case!("terms-canonical-literal-value-ref-order.swapped", true),
        case!("terms-canonical-value-order.swap-0", true),
        case!("terms-duplicate-canonical-kinds.arrival", false),
        case!("terms-duplicate-canonical-kinds.canonical", true),
    ] {
        let graph = second(bytes);
        if expected {
            let frame = graph
                .frame_wire
                .iter()
                .rfind(|frame| frame.frame_type == Some(FrameType::Terms as u64))
                .unwrap();
            assert_eq!(graph.term_order_violations, [frame.offset], "{name}");
            assert_eq!(frame.payload, PayloadOutcome::Verified, "{name}");
        } else {
            assert_eq!(graph.term_order_violations, [], "{name}");
        }
    }

    let (_, arrival, _) = case!("terms-duplicate-canonical-kinds.arrival", false);
    let (_, canonical, _) = case!("terms-duplicate-canonical-kinds.canonical", true);
    let arrival = second(arrival);
    let canonical = second(canonical);
    assert!(canonical.rdf.terms().eq(arrival.rdf.terms()));
    assert!(canonical.rdf.lines().eq(arrival.rdf.lines()));
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

    let mut limits = ReadLimits::default();
    limits.second_generation.items = 0;
    let error = read_with_limits(bytes, limits).unwrap_err();
    assert!(matches!(error.failure, ReadFailure::ItemCapacity(0)));

    let mut invalid = bytes.to_vec();
    invalid[0] = 0x88;
    invalid.push(0xf6);
    let ReadGraph::Second(graph) = read_with_limits(&invalid, ReadLimits::default()).unwrap()
    else {
        panic!("the invalid opener still declares generation 2");
    };
    let error = graph.into_complete_rdf().unwrap_err();
    assert!(matches!(error.failure, ReadFailure::Wire(_)));
    let error = read_with_limits(&invalid, limits).unwrap_err();
    assert!(matches!(error.failure, ReadFailure::ItemCapacity(0)));

    let mut writer = SegmentWriter::new(Vec::new()).unwrap();
    writer.add_blob(b"checked prefix", None).unwrap();
    writer.add_commit().unwrap();
    let bytes = writer.into_inner();
    let stopped = second_at_capacity(
        &bytes,
        ReadCapacity {
            items: 2,
            ..capacity
        },
    );
    assert_eq!(stopped.end, FileEnd::NotRead);
    assert!(stopped.blobs.is_empty());
    assert_eq!(stopped.labelling_size(), None);
    assert!(
        stopped
            .label_content(Limit::new(0), &mut LabellingCapacity::new(0, 0))
            .unwrap()
            .is_none()
    );
    assert_eq!(second(&bytes).blobs[0].bytes.as_ref(), b"checked prefix");
}

#[test]
fn the_item_limit_retains_canonical_claims_without_forming_a_fold() {
    use purrdf_gts::v2::{
        CanonicalLabellingCheck, CanonicalLabellingUnchecked, CanonicalStorageCheck,
        CanonicalStructureCheck,
    };

    let bytes = include_bytes!("fixtures/v2/P7/canonical__construction.bin");
    let preceding = SegmentWriter::new(Vec::new()).unwrap().into_inner();
    let mut unreadable = include_bytes!("fixtures/v2/P1/header-minimal.bin").to_vec();
    unreadable[0] = 0x88;
    unreadable.push(0xf6);
    let joined = [&unreadable, preceding.as_slice(), bytes.as_slice()].concat();
    let limits = ReadCapacity {
        items: 3,
        ..ReadCapacity::default()
    };
    let stopped = second_at_capacity(&joined, limits);
    assert_eq!(stopped.end, FileEnd::NotRead);
    assert_eq!(stopped.labelling_size(), None);
    let mut budget = LabellingCapacity::new(u64::MAX, u64::MAX);
    let report = stopped.label_read(Limit::new(0), &mut budget).unwrap();
    assert_eq!(budget.total(), 0);
    assert!(report.file.unwrap().is_none());
    let [unreadable, unclaimed, claimed] = report.segments.as_slice() else {
        panic!("all three openers were observed before the item limit");
    };
    for (ordinal, segment) in [unreadable, unclaimed, claimed].into_iter().enumerate() {
        assert_eq!(segment.segment, ordinal as u64);
        assert!(matches!(segment.content_digest, Ok(None)));
        assert_eq!(segment.closing_index, None);
        assert_eq!(segment.canonical.first_difference, None);
    }
    assert!(!unreadable.canonical.claimed);
    assert_eq!(unreadable.canonical, unclaimed.canonical);
    assert_eq!(
        unclaimed.canonical.structure,
        CanonicalStructureCheck::NotClaimed
    );
    assert!(claimed.canonical.claimed);
    assert_eq!(
        claimed.canonical.structure,
        CanonicalStructureCheck::ExceedsCapacity
    );
    assert_eq!(
        claimed.canonical.labelling,
        CanonicalLabellingCheck::Unchecked(CanonicalLabellingUnchecked::ExceedsCapacity)
    );
    assert_eq!(
        claimed.canonical.storage,
        CanonicalStorageCheck::ExceedsCapacity
    );

    let complete = second(&joined)
        .label_read(Limit::new(0), &mut budget)
        .unwrap();
    assert!(complete.file.unwrap().is_some());
    assert_eq!(complete.segments[0].canonical, unreadable.canonical);
    assert_eq!(
        complete.segments[2].canonical.structure,
        CanonicalStructureCheck::Verified
    );
}

#[test]
fn whole_read_returns_the_requested_canonical_report() {
    use purrdf_gts::v2::{CanonicalLabellingCheck, CanonicalStructureCheck, ReadLabellingLimits};

    let bytes = include_bytes!("fixtures/v2/P7/canonical__construction.bin");
    let mut limits = ReadLimits::default();
    limits.labelling = Some(ReadLabellingLimits::default());
    let ReadGraph::Second(graph) = read_with_limits(bytes, limits).unwrap() else {
        panic!("the fixture opens a generation 2 file");
    };
    let report = graph.read_labelling().unwrap();
    let [segment] = report.segments.as_slice() else {
        panic!("the fixture holds one complete segment");
    };
    assert_eq!(
        segment.canonical.structure,
        CanonicalStructureCheck::Verified
    );
    assert_eq!(
        segment.canonical.labelling,
        CanonicalLabellingCheck::Verified
    );
    let file = report.file.as_ref().unwrap().as_ref().unwrap();
    assert!(file.content_digest.is_some());
    assert_eq!(
        *segment.content_digest.as_ref().unwrap(),
        file.content_digest
    );

    limits.second_generation.items = 1;
    let ReadGraph::Second(graph) = read_with_limits(bytes, limits).unwrap() else {
        panic!("the claimed segment's opener fits the item limit");
    };
    let report = graph.read_labelling().unwrap();
    assert!(matches!(report.file, Ok(None)));
    assert_eq!(
        report.segments[0].canonical.structure,
        CanonicalStructureCheck::ExceedsCapacity
    );
}

#[test]
fn recipes_use_prior_frames_and_every_unit_in_their_own_pack() {
    use purrdf_gts::v2::UnreferencedUnit;

    let compressed = second(include_bytes!(
        "fixtures/v2/P4/pack-compressed.one-frame.bin"
    ));
    assert!(compressed.blobs.is_empty());

    let earlier = second(include_bytes!(
        "fixtures/v2/P4/rebuild-unit-after-recipe.units-first.bin"
    ));
    assert_eq!(earlier.blobs.len(), 1);
    assert_eq!(earlier.unreferenced_units, [] as [UnreferencedUnit; 0]);
    for (bytes, member) in [
        (
            include_bytes!("fixtures/v2/P4/rebuild-unit-after-recipe.pack-recipe-first.bin")
                .as_slice(),
            0,
        ),
        (
            include_bytes!("fixtures/v2/P4/rebuild-unit-after-recipe.pack-recipe-last.bin")
                .as_slice(),
            2,
        ),
    ] {
        let packed = second(bytes);
        assert_eq!(packed.blobs.len(), 1);
        assert_eq!(packed.unreferenced_units, [] as [UnreferencedUnit; 0]);
        let [result] = packed.rebuilds.as_slice() else {
            panic!("expected one rebuild from the pack");
        };
        assert_eq!(
            (result.member, result.state),
            (Some(member), RebuildState::Verified)
        );
        assert_eq!(packed.blobs[0].name, earlier.blobs[0].name);
        assert_eq!(packed.blobs[0].bytes, earlier.blobs[0].bytes);
    }
    let later = include_bytes!("fixtures/v2/P4/rebuild-unit-after-recipe.recipe-first.bin");
    let graph = second(later);
    assert!(graph.blobs.is_empty());
    assert_eq!(graph.unreferenced_units, [] as [UnreferencedUnit; 0]);
    let [error] = graph.frame_failures.as_slice() else {
        panic!("expected one unavailable unit finding");
    };
    assert!(matches!(error.failure, ReadFailure::MissingUnit(_)));
    assert_eq!(graph.rebuilds[0].state, RebuildState::MissingUnit);

    let malformed = include_bytes!("fixtures/v2/P4/pack-recipe-trailing-count.bin");
    let graph = second(malformed);
    assert!(graph.blobs.is_empty());
    assert!(graph.blob_set.is_empty());
    assert_eq!(graph.end, FileEnd::AtValidCommit);
    let error = graph.into_complete_rdf().unwrap_err();
    assert!(matches!(
        error.failure,
        ReadFailure::Wire(wire) if wire.reason == MalformedReason::WrongLength
    ));
}

#[test]
fn failed_pack_unit_digests_leave_other_declarations_and_the_commit_readable() {
    let one = include_bytes!("fixtures/v2/P4/pack-member-digest.one-failing.bin");
    let named_blob = second(one).blobs[0].name;
    for (bytes, failed, verified) in [
        (one.as_slice(), 1, 1),
        (
            include_bytes!("fixtures/v2/P4/pack-member-digest.two-failing.bin").as_slice(),
            2,
            0,
        ),
    ] {
        let graph = second(bytes);
        assert_eq!(graph.frame_failures.len(), failed);
        assert!(graph.frame_failures.iter().all(|finding| {
            finding.offset == graph.frame_failures[0].offset
                && matches!(finding.failure, ReadFailure::BlobDigest)
        }));
        assert_eq!(graph.blobs.len(), verified);
        assert_eq!(graph.left_out, 0);
        assert!(
            graph
                .blob_set
                .iter()
                .any(|entry| entry.name == named_blob && entry.present)
        );
        assert_eq!(graph.end, FileEnd::AtValidCommit);
        assert!(matches!(
            graph.into_complete_rdf().unwrap_err().failure,
            ReadFailure::BlobDigest
        ));
    }
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
    assert_eq!(decoded.blob_set[0].name, expected.name);
    assert!(!decoded.blob_set[0].present);
}

#[test]
fn blob_declarations_union_by_digest_and_keep_the_smallest_length() {
    let body = b"named blob";
    let name = ValueName::of(body);
    let conflicting = ValueName {
        length: name.length + 1,
        ..name
    };
    let mut writer = SegmentWriter::new(Vec::new()).unwrap();
    writer
        .add_external_blob(conflicting, Some("a/long"))
        .unwrap();
    writer.add_blob(body, Some("z/b")).unwrap();
    writer.add_external_blob(name, Some("a/a")).unwrap();
    writer.add_external_blob(name, Some("a/a")).unwrap();
    writer.add_commit().unwrap();
    let bytes = writer.into_inner();
    let graph = second(&bytes);
    let [entry] = graph.blob_set.as_slice() else {
        panic!("expected declarations of the same digest to form one entry");
    };
    assert_eq!(entry.name, name);
    assert!(entry.present);
    assert_eq!(graph.labelling_size(), Some(4));
    assert_eq!(entry.media_types, ["a/a", "z/b", "a/long"]);
    assert_eq!(
        graph.blob_length_conflicts,
        [Item::first(&bytes).unwrap().unwrap().bytes().len()]
    );
}

#[test]
fn authored_blob_recipes_use_the_specification_cut_points() {
    use std::io::{self, Read};

    struct BrokenSourceEnd;

    impl Read for BrokenSourceEnd {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::ErrorKind::UnexpectedEof.into())
        }
    }

    let source = include_bytes!("fixtures/v2/P4/chunker-cut-loose.bin");
    let media_type = Some("application/octet-stream");
    let mut writer = SegmentWriter::new(Vec::new()).unwrap();
    let expected = writer.add_chunked_blob(source, media_type).unwrap();
    let mut streamed = SegmentWriter::new(Vec::new()).unwrap();
    let limit = std::num::NonZeroUsize::new(64).unwrap();
    let actual = streamed
        .add_chunked_blob_from_reader(source.as_slice(), source.len() as u64, limit, media_type)
        .unwrap();
    assert_eq!(actual, expected);
    assert_eq!(streamed.head(), writer.head());
    let mut refused = SegmentWriter::new(Vec::new()).unwrap();
    assert!(matches!(
        refused.add_chunked_blob_from_reader(
            source.as_slice(),
            source.len() as u64 - 1,
            limit,
            media_type,
        ),
        Err(SegmentWriteError::BlobSource(_))
    ));
    assert!(matches!(
        refused.add_commit(),
        Err(SegmentWriteError::IncompleteFrame)
    ));
    let mut source_failed = SegmentWriter::new(Vec::new()).unwrap();
    assert!(matches!(
        source_failed.add_chunked_blob_from_reader(
            source.as_slice().chain(BrokenSourceEnd),
            source.len() as u64,
            limit,
            media_type,
        ),
        Err(SegmentWriteError::BlobSource(error)) if error.kind() == io::ErrorKind::UnexpectedEof
    ));
    assert!(matches!(
        source_failed.add_commit(),
        Err(SegmentWriteError::IncompleteFrame)
    ));
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
    assert!(bytes.starts_with(&streamed.into_inner()));
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

/// The streaming writer can consume an oversized blob through bounded reads.
#[test]
fn oversized_blob_authoring_uses_bounded_source_reads() {
    use std::io::{self, Read};

    struct Source(u64);

    impl Read for Source {
        fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
            assert!(output.len() <= 256 * 1024);
            let count = self.0.min(output.len() as u64) as usize;
            output[..count].fill(0xa5);
            self.0 -= count as u64;
            Ok(count)
        }
    }

    let length = 128 * 1024 * 1024 + 1;
    let mut source = Source(length);
    let mut writer = SegmentWriter::new(io::sink()).unwrap();
    let name = writer
        .add_chunked_blob_from_reader(
            &mut source,
            length,
            std::num::NonZeroUsize::new(8193).unwrap(),
            None,
        )
        .unwrap();
    assert_eq!(source.0, 0);
    assert_eq!(name.length, length);
    writer.add_commit().unwrap();
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
