// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Whole update allocations, including source preparation and public projections.

use std::sync::Arc;

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_core::{
    DatasetView, FallibleDatasetView, QuadIds, RdfDataset, RdfDatasetBuilder, SegmentedBuildLimits,
    SegmentedBuilder, SegmentedLayer, SegmentedLayerMetadata, SegmentedReadLimits,
    SegmentedResidentHead, SegmentedStack, TermFactory, TermValue,
};
use purrdf_text::{Analyzer, GraphSelector, HanCharacterIndex, TextIndex, TextIndexConfig};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

const PREDICATE: &str = "https://example.org/resource/text";

fn values(index: usize) -> [TermValue; 3] {
    [
        TermValue::iri(format!("https://example.org/document/{index:08}")),
        TermValue::iri(PREDICATE),
        TermValue::simple_literal("bank 北京大学 river"),
    ]
}

fn addition() -> [TermValue; 3] {
    [
        TermValue::iri("https://example.org/000000"),
        TermValue::iri(PREDICATE),
        TermValue::simple_literal("aardvark 北京银行 bank"),
    ]
}

fn dataset(rows: impl IntoIterator<Item = [TermValue; 3]>) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    for [s, p, o] in rows {
        let s = builder.intern_value(&s).unwrap();
        let p = builder.intern_value(&p).unwrap();
        let o = builder.intern_value(&o).unwrap();
        builder.push_quad(s, p, o, None).unwrap();
    }
    builder.freeze().unwrap()
}

fn sealed(count: usize) -> Arc<SegmentedLayer> {
    let mut builder = SegmentedBuilder::new(
        SegmentedBuildLimits::new(100_000, 20_000_000, 100_000, 4096, 128).unwrap(),
    )
    .unwrap();
    for index in 0..count {
        let mut ids = [None; 3];
        let mut at = 0;
        builder
            .intern_batch(&values(index), |_, id| {
                ids[at] = Some(id);
                at += 1;
            })
            .unwrap();
        builder
            .push_quad(QuadIds {
                s: ids[0].unwrap(),
                p: ids[1].unwrap(),
                o: ids[2].unwrap(),
                g: None,
            })
            .unwrap();
    }
    let image = builder.seal().unwrap();
    Arc::new(
        SegmentedLayer::new(
            Arc::new(image.provider()),
            image.receipt().clone(),
            SegmentedLayerMetadata::default(),
        )
        .unwrap(),
    )
}

fn limits() -> SegmentedReadLimits {
    SegmentedReadLimits::new(512_000_000, 32, 4_000_000, 2_000_000_000, 32)
}

fn build<D: DatasetView>(source: &D, config: &TextIndexConfig, han: bool) -> TextIndex
where
    D::ReadError: purrdf_hash::error::StructuredError,
{
    if han {
        HanCharacterIndex::from_dataset(source, config)
            .unwrap()
            .index()
            .clone()
    } else {
        TextIndex::from_dataset(source, config).unwrap()
    }
}

fn assert_fresh(actual: &TextIndex, expected: &TextIndex) {
    assert_eq!(actual.fingerprint(), expected.fingerprint());
    assert_eq!(actual.source_fingerprint(), expected.source_fingerprint());
    assert_eq!(actual.source_coverage(), expected.source_coverage());
    assert!(actual.documents().eq(expected.documents()));
    assert!(actual.partitions().eq(expected.partitions()));
    assert!(actual.terms().eq(expected.terms()));
    for document in 0..u32::try_from(expected.document_count()).unwrap() {
        assert_eq!(
            actual.field_lengths(document),
            expected.field_lengths(document)
        );
        assert!(
            actual
                .token_occurrences(document)
                .unwrap()
                .eq(expected.token_occurrences(document).unwrap())
        );
    }
    for (partition, _) in expected.partitions() {
        assert_eq!(
            actual.field_totals(partition),
            expected.field_totals(partition)
        );
        assert_eq!(
            actual.field_populations(partition),
            expected.field_populations(partition)
        );
        for term in expected.terms() {
            assert!(
                actual
                    .postings(partition, term)
                    .eq(expected.postings(partition, term))
            );
            assert_eq!(
                actual.document_frequency(partition, term),
                expected.document_frequency(partition, term)
            );
        }
    }
    let actual = actual.surface_index().unwrap();
    let expected = expected.surface_index().unwrap();
    assert!(actual.words().eq(expected.words()));
    assert!(actual.spans().eq(expected.spans()));
}

#[test]
fn fixed_batch_preparation_clone_and_complete_update_have_bounded_allocation_growth() {
    let config = TextIndexConfig::new(
        vec![TermValue::iri(PREDICATE)],
        GraphSelector::Default,
        Analyzer::empty_lexicon(),
    )
    .unwrap();
    let empty = dataset([]);
    let mut measurements = Vec::new();
    for han in [false, true] {
        for count in [128, 4096, 16384] {
            // The retained image and initial index are initial construction, not
            // a hidden part of the next mutation. No compaction occurs here.
            let mut stack = SegmentedStack::default();
            let layer = sealed(count);
            let proof_levels =
                usize::try_from(layer.receipt().block_count().next_power_of_two().ilog2()).unwrap();
            stack.publish(vec![layer], limits()).unwrap();
            let before = stack.snapshot().unwrap();
            let source = before.query(limits()).unwrap();
            let index = build(&source, &config, han);
            let original_fingerprint = index.fingerprint();
            drop(source);

            let window = CurrentThreadWindow::open();
            let snapshot = index.clone();
            let cloned = window.close();
            drop(snapshot);

            // Measure the actual new native batch, its independent metadata,
            // head publication and snapshot together. The retained image stays.
            let window = CurrentThreadWindow::open();
            let added = dataset([addition()]);
            stack.set_head(Some(
                SegmentedResidentHead::new(Arc::clone(&added), SegmentedLayerMetadata::default())
                    .unwrap(),
            ));
            let next = stack.snapshot().unwrap();
            let prepared = window.close();

            let window = CurrentThreadWindow::open();
            let source = next.query(limits()).unwrap();
            let updated = index.clone().with_delta(&source, &*added, &*empty).unwrap();
            let status = source.operation_status();
            assert!(status.error().is_none());
            let reads = status.evidence().request_count();
            let bytes = status.evidence().io_bytes();
            let (payload_reads, proof_reads) = status.evidence().with_requests(|requests| {
                requests
                    .iter()
                    .fold((0_usize, 0_usize), |(payload, proof), request| {
                        assert!(request.completed());
                        match request.bytes() {
                            4096 => (payload + 1, proof),
                            32 => (payload, proof + 1),
                            bytes => panic!("unexpected encoded read width {bytes}"),
                        }
                    })
            });
            assert_eq!(reads, payload_reads + proof_reads);
            assert!(proof_reads <= payload_reads * proof_levels);
            assert_eq!(
                bytes,
                u64::try_from(payload_reads * 4096 + proof_reads * 32).unwrap()
            );
            drop(status);
            drop(source);
            let updated_allocations = window.close();
            assert_eq!(index.fingerprint(), original_fingerprint);
            assert_eq!(index.document_count(), count as u64);

            // A fully reconstructed native source is ONLY the independent
            // oracle, outside the production preparation/update windows.
            let oracle = dataset((0..count).map(values).chain([addition()]));
            let window = CurrentThreadWindow::open();
            let fresh = build(&*oracle, &config, han);
            let fresh_allocations = window.close();
            assert_fresh(&updated, &fresh);
            assert_eq!(
                updated.document(0).unwrap().subject(),
                &TermValue::iri("https://example.org/000000")
            );
            assert!(updated_allocations.requested_bytes < fresh_allocations.requested_bytes);
            measurements.push((
                han,
                count,
                prepared,
                cloned,
                updated_allocations,
                reads,
                bytes,
                fresh_allocations,
                payload_reads,
                proof_reads,
                proof_levels,
            ));
        }
    }
    eprintln!(
        "text delta allocations (Han, retained documents, prepare, clone, update, reads, I/O, fresh, payload reads, proof reads, proof levels): {measurements:?}"
    );
    for group in measurements.as_chunks::<3>().0 {
        let first = &group[0];
        let last = &group[2];
        assert_eq!(
            first.2, last.2,
            "fixed new-batch preparation depends on retained corpus"
        );
        assert_eq!(first.3, last.3, "snapshot clone copies retained storage");
        // Corpus population grows 128-fold. Allow routing/tree growth, while
        // rejecting source-sized allocation traffic or retained/peak copies.
        assert!(last.4.allocations < first.4.allocations * 8);
        assert!(last.4.requested_bytes < first.4.requested_bytes * 8);
        assert!(last.4.retained_bytes < first.4.retained_bytes * 8);
        assert!(last.4.peak_working_bytes < first.4.peak_working_bytes * 8);
        // The exact ledger also counts each hash on the certified proof path.
        // Bound payload growth separately; all proof requests remain counted
        // and checked against the actual certified tree depth above.
        assert!(last.8 < first.8 * 8);
        assert!(last.6 < first.6 * 8);
    }
}
