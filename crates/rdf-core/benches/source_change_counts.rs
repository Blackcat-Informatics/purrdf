// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API, so the workspace `missing_docs` lint is
// not asked of their items.
#![allow(missing_docs)]

//! Fixed work of the segmented seal and the source change feed: allocations,
//! requested and retained bytes, and retired user-mode instructions.
//!
//! `cargo bench -p purrdf-core --bench source_change_counts [-- <row filter>]`
//! prints one allocation line per row, measured in process. With `--counts`
//! (Linux and `perf`), each row also runs in fresh child processes under the
//! test kit's counter protocol and prints its instruction samples. Rows:
//!
//! * `seal`: `SegmentedBuilder::seal`, which writes and certifies the image.
//! * `capture`: the first source version of a one-image stack.
//! * `removal_visibility`: an ordinary scan across three nonempty removal
//!   carriers and a newer resident reinsertion.
//! * `feed_layers`: the change feed after replacing that image with one whose
//!   middle row changed.
//! * `feed_head`: the same change in a resident head.
//! * `feed_regroup`: the change feed after replacing that image with three
//!   images that partition the changed rows by subject identifier, so every carrier
//!   differs while only the middle row changed.
//! * `statistics`: `TripleDatasetBuilder::freeze` of the same rows as
//!   triples, which builds the eager statistics and reads no change feed.
//! * `identity_resident`: the first content identity of the resident freeze
//!   of the fixture, which has no eager statistics and reads every row.
//! * `identity_admitted`: the first content identity of the admitted freeze,
//!   which reads its eager statistics' record directory.
//! * `identity_delta`: the first content identity of a mutation snapshot that
//!   removes the middle quad and inserts its changed revision, over a base
//!   whose identity is already sealed.
//! * `version_fingerprint`: `SourceVersion::content_fingerprint` of the first
//!   source version of a one-image stack.
//!
//! The fixture has `PURRDF_SOURCE_CHANGE_ROWS` rows (2048 by default), each a
//! quad (a triple in `statistics`), a reifier and an annotation. Fixture construction, sealing of the
//! compared images and the previous version are outside every window. The
//! counting allocator also runs in the instruction samples; its work is the
//! same for every build compared.

use std::hint::black_box;
use std::ops::ControlFlow;
use std::sync::Arc;

use purrdf_alloc_probe::CountingAllocator;
use purrdf_core::source::SourceVisit;
use purrdf_core::{
    ContentIdentified, DatasetMut, DatasetView, MutableDataset, QuadValues, RdfDataset, RdfLiteral,
    SegmentedBuildLimits, SegmentedBuilder, SegmentedLayer, SegmentedLayerMetadata,
    SegmentedReadLimits, SegmentedResidentHead, SegmentedStack, TermId, TripleDatasetBuilder,
};

#[path = "../tests/support/source_snapshot.rs"]
mod source_snapshot;

use purrdf_testkit::bench::counter::{CountRows, CountWork, run_count_rows};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

const ROWS: [&str; 11] = [
    "seal",
    "capture",
    "removal_visibility",
    "feed_layers",
    "feed_head",
    "feed_regroup",
    "statistics",
    "identity_resident",
    "identity_admitted",
    "identity_delta",
    "version_fingerprint",
];

fn limits() -> SegmentedReadLimits {
    SegmentedReadLimits::new(1 << 30, 64, 20_000_000, 1 << 34, 32)
}

fn builder(dataset: &RdfDataset) -> SegmentedBuilder {
    builder_of(dataset, |_| true)
}

/// A builder over the rows of `dataset` whose subject `keep` admits.
///
/// The construction limits are the floor of 2^20 terms, 2^28 term bytes and 2^20
/// rows, raised to the fixture's own term count, arena bytes and row count, so
/// every fixture size builds and the default size keeps its floor.
fn builder_of(dataset: &RdfDataset, keep: impl Fn(TermId) -> bool) -> SegmentedBuilder {
    let fit = |actual: usize, floor: u32| {
        u32::try_from(actual)
            .expect("fixture within the u32 construction bounds")
            .max(floor)
    };
    let limits = SegmentedBuildLimits::new(
        fit(dataset.term_count(), 1 << 20),
        fit(dataset.rdf_text_bytes(), 1 << 28),
        fit(dataset.rdf_row_count(), 1 << 20),
        4096,
        64,
    )
    .unwrap();
    let mut builder = SegmentedBuilder::new(limits).unwrap();
    for (stream, rows) in [
        dataset.quads().collect::<Vec<_>>(),
        dataset.reifier_quads().collect(),
        dataset.annotation_quads().collect(),
    ]
    .into_iter()
    .enumerate()
    {
        for row in rows {
            if !keep(row.s) {
                continue;
            }
            let mut values = vec![
                dataset.term_value(row.s),
                dataset.term_value(row.p),
                dataset.term_value(row.o),
            ];
            values.extend(row.g.map(|id| dataset.term_value(id)));
            let mut ids = Vec::new();
            builder.intern_batch(&values, |_, id| ids.push(id)).unwrap();
            let quad = purrdf_core::QuadIds {
                s: ids[0],
                p: ids[1],
                o: ids[2],
                g: ids.get(3).copied(),
            };
            match stream {
                0 => builder.push_quad(quad).unwrap(),
                1 => builder.push_reifier(quad.s, quad.o, quad.g).unwrap(),
                _ => builder.push_annotation(quad).unwrap(),
            }
        }
    }
    builder
}

fn layer(dataset: &RdfDataset) -> Arc<SegmentedLayer> {
    sealed(builder(dataset))
}

fn sealed(builder: SegmentedBuilder) -> Arc<SegmentedLayer> {
    sealed_with_metadata(builder, SegmentedLayerMetadata::default())
}

fn sealed_with_metadata(
    builder: SegmentedBuilder,
    metadata: SegmentedLayerMetadata,
) -> Arc<SegmentedLayer> {
    let image = builder.seal().unwrap();
    Arc::new(
        SegmentedLayer::new(
            Arc::new(image.provider()),
            image.receipt().clone(),
            metadata,
        )
        .unwrap(),
    )
}

fn head(dataset: Arc<RdfDataset>) -> SegmentedResidentHead {
    SegmentedResidentHead::new(dataset, SegmentedLayerMetadata::default()).unwrap()
}

/// Prepare one row outside the window; the returned closure is the measured
/// work and returns its witness.
fn prepare(row: &str, count: usize) -> CountWork {
    let original = source_snapshot::dataset(count, false, false);
    let changed = source_snapshot::dataset(count, true, false);
    match row {
        "seal" => {
            let builder = builder(&original);
            Box::new(move || black_box(builder.seal().unwrap()).bytes().len() as u64)
        }
        "capture" => {
            let mut stack = SegmentedStack::default();
            stack.publish(vec![layer(&original)], limits()).unwrap();
            let snapshot = stack.snapshot().unwrap();
            Box::new(move || {
                let query = snapshot.query(limits()).unwrap();
                let version = black_box(query.source_version().unwrap().unwrap());
                u64::try_from(version.statement_count()).unwrap()
            })
        }
        "removal_visibility" => {
            let restored = source_snapshot::dataset(1, false, false);
            let other_revision = source_snapshot::dataset(1, true, false);
            let mut layers = vec![layer(&original)];
            for removed in [Arc::clone(&restored), other_revision, Arc::clone(&restored)] {
                let metadata = SegmentedLayerMetadata::new(Some(removed), None).unwrap();
                layers.push(sealed_with_metadata(
                    builder_of(&original, |_| false),
                    metadata,
                ));
            }
            let mut stack = SegmentedStack::default();
            stack.publish(layers, limits()).unwrap();
            stack.set_head(Some(head(restored)));
            let snapshot = stack.snapshot().unwrap();
            Box::new(move || {
                let query = snapshot.query(limits()).unwrap();
                let visible = black_box(query.quads().count());
                query.check_ready().unwrap();
                u64::try_from(visible).unwrap()
            })
        }
        "identity_resident" => {
            let dataset = source_snapshot::dataset(count, false, false);
            Box::new(move || first_byte(&dataset.content_identity().unwrap()))
        }
        "identity_admitted" => {
            let builder = source_snapshot::builder(count, false, false);
            let mut admission = purrdf_lex::allocation::Resident;
            let bytes = builder.native_buffer_bytes().unwrap();
            let dataset = builder
                .freeze_with_memory(&mut purrdf_lex::allocation::Memory::resume(
                    &mut admission,
                    bytes,
                ))
                .unwrap();
            Box::new(move || first_byte(&dataset.content_identity().unwrap()))
        }
        "identity_delta" => {
            let base = source_snapshot::dataset(count, false, false);
            base.content_identity().unwrap();
            let changed = source_snapshot::dataset(count, true, false);
            let middle = |dataset: &RdfDataset| {
                let row = dataset
                    .quads()
                    .find(|row| {
                        dataset.term_value(row.s)
                            == purrdf_core::TermValue::iri(format!("urn:source:{}", count / 2))
                    })
                    .unwrap();
                QuadValues {
                    s: dataset.term_value(row.s),
                    p: dataset.term_value(row.p),
                    o: dataset.term_value(row.o),
                    g: row.g.map(|id| dataset.term_value(id)),
                }
            };
            let mut mutation = MutableDataset::new(Arc::clone(&base));
            assert!(mutation.remove(&middle(&base)));
            assert!(mutation.insert(middle(&changed)).unwrap());
            let view = mutation.snapshot_view().unwrap();
            Box::new(move || first_byte(&view.content_identity().unwrap()))
        }
        "version_fingerprint" => {
            let mut stack = SegmentedStack::default();
            stack.publish(vec![layer(&original)], limits()).unwrap();
            let version = stack
                .snapshot()
                .unwrap()
                .query(limits())
                .unwrap()
                .source_version()
                .unwrap()
                .unwrap();
            Box::new(move || u64::from(black_box(version.content_fingerprint())[0]))
        }
        "statistics" => {
            let builder = triples(count);
            Box::new(move || u64::try_from(black_box(builder.freeze().unwrap()).len()).unwrap())
        }
        "feed_layers" | "feed_head" | "feed_regroup" => {
            let mut stack = SegmentedStack::default();
            if row == "feed_head" {
                stack.set_head(Some(head(original)));
            } else {
                stack.publish(vec![layer(&original)], limits()).unwrap();
            }
            let previous = stack
                .snapshot()
                .unwrap()
                .query(limits())
                .unwrap()
                .source_version()
                .unwrap()
                .unwrap();
            match row {
                "feed_head" => stack.set_head(Some(head(changed))),
                "feed_layers" => stack.publish(vec![layer(&changed)], limits()).unwrap(),
                _ => stack
                    .publish(
                        (0..3)
                            .map(|part| {
                                sealed(builder_of(&changed, |subject| subject.index() % 3 == part))
                            })
                            .collect(),
                        limits(),
                    )
                    .unwrap(),
            }
            let snapshot = stack.snapshot().unwrap();
            Box::new(move || {
                let query = snapshot.query(limits()).unwrap();
                let mut emitted = 0_u64;
                let observed = query
                    .visit_source_changes(&previous, |_, _, _| {
                        emitted += 1;
                        ControlFlow::<()>::Continue(())
                    })
                    .unwrap();
                let SourceVisit::Complete { version, .. } = observed else {
                    panic!("the change feed completes");
                };
                black_box(version);
                emitted
            })
        }
        _ => panic!("unknown row {row}"),
    }
}

/// A witness that depends on the whole identity.
fn first_byte(identity: &purrdf_hash::hex::Digest32) -> u64 {
    u64::from(black_box(identity.as_bytes())[0])
}

/// The `statistics` fixture: the source rows as triples, reifiers and
/// annotations in a compact builder, which is frozen in the window.
fn triples(count: usize) -> TripleDatasetBuilder {
    let mut builder = TripleDatasetBuilder::default();
    let p = builder.intern_iri("urn:source:predicate").unwrap();
    let annotation = builder.intern_iri("urn:source:annotation").unwrap();
    for index in 0..count {
        let subject = builder.intern_iri(&format!("urn:source:{index}")).unwrap();
        let reifier = builder
            .intern_iri(&format!("urn:source:reifier:{index}"))
            .unwrap();
        let object = builder
            .intern_literal(RdfLiteral::simple(format!("row {index} revision 0")))
            .unwrap();
        let note = builder
            .intern_literal(RdfLiteral::simple(format!("note {index} revision 0")))
            .unwrap();
        builder.push_triple(subject, p, object).unwrap();
        let triple = builder.intern_triple(subject, p, object).unwrap();
        builder.push_reifier(reifier, triple).unwrap();
        builder.push_annotation(reifier, annotation, note).unwrap();
    }
    builder
}

const SPEC: CountRows<'static> = CountRows {
    bench: "source_change_counts",
    rows: &ROWS,
    size: ("PURRDF_SOURCE_CHANGE_ROWS", 2048),
    samples: "PURRDF_SOURCE_CHANGE_COUNT_SAMPLES",
};

/// Run this target's rows through the shared count runner.
fn main() {
    run_count_rows(&SPEC, &prepare, &purrdf_alloc_probe::whole_process_report);
}
