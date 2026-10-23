// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Independent observations through actual chronological query callers.

#[path = "support/source_snapshot.rs"]
mod source_snapshot;

use std::collections::BTreeSet;
use std::ops::ControlFlow;
use std::sync::Arc;

use purrdf_core::source::{SourceFamily, SourceRowRef, SourceSide, SourceVisit};
use purrdf_core::{
    DatasetView, FallibleDatasetView, QuadValues, RdfDataset, SegmentedLayerMetadata,
    SegmentedReadLimits, SegmentedResidentHead, SegmentedStack, SegmentedStackSnapshot,
};
use source_snapshot::dataset;

fn limits() -> SegmentedReadLimits {
    SegmentedReadLimits::new(256_000_000, 8, 2_000_000, 256_000_000, 32)
}

fn snapshot(dataset: Arc<RdfDataset>) -> SegmentedStackSnapshot {
    let mut stack = SegmentedStack::default();
    stack.set_head(Some(
        SegmentedResidentHead::new(dataset, SegmentedLayerMetadata::default()).unwrap(),
    ));
    stack.snapshot().unwrap()
}

fn value(row: SourceRowRef<'_>) -> QuadValues {
    QuadValues {
        s: row.subject.clone(),
        p: row.predicate.clone(),
        o: row.object.clone(),
        g: row.graph.cloned(),
    }
}

fn surface(snapshot: &SegmentedStackSnapshot) -> BTreeSet<(u8, QuadValues)> {
    let query = snapshot.query(limits()).unwrap();
    let mut result = BTreeSet::new();
    for (family, rows) in [
        (
            0,
            query
                .quads()
                .chain(query.annotation_quads())
                .collect::<Vec<_>>(),
        ),
        (1, query.reifier_quads().collect()),
    ] {
        for row in rows {
            result.insert((
                family,
                QuadValues {
                    s: query.term_value(row.s).unwrap().value().clone(),
                    p: query.term_value(row.p).unwrap().value().clone(),
                    o: query.term_value(row.o).unwrap().value().clone(),
                    g: row
                        .g
                        .map(|id| query.term_value(id).unwrap().value().clone()),
                },
            ));
        }
    }
    query.check_ready().unwrap();
    result
}

fn sealed(dataset: &RdfDataset) -> Arc<purrdf_core::SegmentedLayer> {
    sealed_where(dataset, |_| true)
}

fn sealed_where(
    dataset: &RdfDataset,
    keep: impl Fn(purrdf_core::QuadIds) -> bool,
) -> Arc<purrdf_core::SegmentedLayer> {
    sealed_with_metadata(dataset, keep, SegmentedLayerMetadata::default())
}

fn sealed_with_metadata(
    dataset: &RdfDataset,
    keep: impl Fn(purrdf_core::QuadIds) -> bool,
    metadata: SegmentedLayerMetadata,
) -> Arc<purrdf_core::SegmentedLayer> {
    let mut builder = purrdf_core::SegmentedBuilder::new(
        purrdf_core::SegmentedBuildLimits::new(200_000, 20_000_000, 200_000, 512, 4).unwrap(),
    )
    .unwrap();
    for (stream, rows) in [
        dataset.quads().collect::<Vec<_>>(),
        dataset.reifier_quads().collect(),
        dataset.annotation_quads().collect(),
    ]
    .into_iter()
    .enumerate()
    {
        for row in rows.into_iter().filter(|row| keep(*row)) {
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
                2 => builder.push_annotation(quad).unwrap(),
                _ => unreachable!(),
            }
        }
    }
    let image = builder.seal().unwrap();
    Arc::new(
        purrdf_core::SegmentedLayer::new(
            Arc::new(image.provider()),
            image.receipt().clone(),
            metadata,
        )
        .unwrap(),
    )
}

fn sealed_partition(
    dataset: &RdfDataset,
    buckets: usize,
    selected: &[usize],
) -> Arc<purrdf_core::SegmentedLayer> {
    sealed_where(dataset, |row| {
        let purrdf_core::TermRef::Iri(subject) = dataset.resolve(row.s) else {
            panic!("fixture subject is an IRI");
        };
        let index = subject
            .rsplit(':')
            .next()
            .unwrap()
            .parse::<usize>()
            .unwrap();
        selected.contains(&(index % buckets))
    })
}

#[test]
fn multiple_output_regrouping_and_lazy_prefix_compaction_follow_only_changed_keys() {
    let mut measurements = Vec::new();
    for count in [96, 768, 3072] {
        let original = dataset(count, false, false);
        let current = dataset(count, true, true);
        let mut stack = SegmentedStack::default();
        stack
            .publish(
                vec![
                    sealed(&original),
                    sealed(&original),
                    sealed(&original),
                    sealed(&original),
                ],
                limits(),
            )
            .unwrap();
        let before = stack.snapshot().unwrap();
        let previous = before
            .query(limits())
            .unwrap()
            .source_version()
            .unwrap()
            .unwrap();
        let old_rows = surface(&snapshot(Arc::clone(&original)));
        let new_rows = surface(&snapshot(Arc::clone(&current)));
        let expected: BTreeSet<_> = old_rows
            .difference(&new_rows)
            .map(|row| (false, row.clone()))
            .chain(
                new_rows
                    .difference(&old_rows)
                    .map(|row| (true, row.clone())),
            )
            .collect();
        let parts: Vec<_> = (0..3)
            .map(|part| sealed_partition(&current, 3, &[part]))
            .collect();
        stack.publish(parts.clone(), limits()).unwrap();
        let after = stack.snapshot().unwrap();
        let query = after.query(limits()).unwrap();
        let initial = query.operation_status();
        let mut actual = BTreeSet::new();
        let observed = query
            .visit_source_changes(&previous, |side, family, row| {
                actual.insert((
                    side == SourceSide::Next,
                    (u8::from(family == SourceFamily::Reifiers), value(row)),
                ));
                ControlFlow::<()>::Continue(())
            })
            .unwrap();
        let SourceVisit::Complete { version, work } = observed else {
            panic!("complete regrouping");
        };
        let status = query.operation_status();
        assert_eq!(actual, expected);
        assert_eq!((work.candidates, work.emitted, work.physical), (6, 6, 6));
        assert_eq!(
            version.content_fingerprint(),
            query
                .source_version()
                .unwrap()
                .unwrap()
                .content_fingerprint()
        );
        let requests = status.evidence().request_count() - initial.evidence().request_count();
        let io = status.evidence().io_bytes() - initial.evidence().io_bytes();
        // The intermediate prefix was intentionally left lazy. Its next partial
        // compaction must compare the original directories without materializing
        // every retained key or retaining a predecessor's physical addresses.
        stack
            .publish(
                vec![
                    sealed_partition(&original, 3, &[0, 1]),
                    Arc::clone(&parts[2]),
                ],
                limits(),
            )
            .unwrap();
        let second = stack.snapshot().unwrap();
        let second = second.query(limits()).unwrap();
        let observed = second
            .visit_source_changes(&version, |_, _, _| ControlFlow::<()>::Continue(()))
            .unwrap();
        let SourceVisit::Complete {
            version,
            work: lazy,
        } = observed
        else {
            panic!("complete lazy prefix");
        };
        assert_eq!((lazy.candidates, lazy.emitted, lazy.physical), (6, 6, 6));
        assert_eq!(
            version.content_fingerprint(),
            previous.content_fingerprint()
        );
        stack
            .publish(
                (0..3)
                    .map(|part| sealed_partition(&original, 3, &[part]))
                    .collect(),
                limits(),
            )
            .unwrap();
        let third = stack.snapshot().unwrap();
        let third = third.query(limits()).unwrap();
        let observed = third
            .visit_source_changes(&version, |_, _, row| -> ControlFlow<()> {
                panic!("unchanged repartition emitted {row:?}");
            })
            .unwrap();
        let SourceVisit::Complete {
            work: unchanged, ..
        } = observed
        else {
            panic!("complete unchanged regrouping");
        };
        assert_eq!(
            (unchanged.candidates, unchanged.emitted, unchanged.physical),
            (0, 0, 0)
        );
        measurements.push((count, work, requests, io, lazy));
    }
    eprintln!(
        "multi-output regrouping (population, work, requests, bytes, lazy prefix): {measurements:?}"
    );
    assert!(measurements[2].1.directory.ranges < measurements[0].1.directory.ranges * 4);
    assert!(measurements[2].2 < measurements[0].2 * 4);
    assert!(measurements[2].3 < measurements[0].3 * 4);
}

#[test]
fn regrouping_overlapping_sources_preserves_set_membership() {
    let data = dataset(48, false, false);
    let mut stack = SegmentedStack::default();
    stack
        .publish(
            vec![
                sealed_partition(&data, 3, &[0, 1]),
                sealed_partition(&data, 3, &[1, 2]),
            ],
            limits(),
        )
        .unwrap();
    let before = stack.snapshot().unwrap();
    let previous = before
        .query(limits())
        .unwrap()
        .source_version()
        .unwrap()
        .unwrap();
    stack
        .publish(
            vec![sealed(&data), sealed_partition(&data, 3, &[0])],
            limits(),
        )
        .unwrap();
    let after = stack.snapshot().unwrap();
    let query = after.query(limits()).unwrap();
    let observed = query
        .visit_source_changes(&previous, |_, _, row| -> ControlFlow<()> {
            panic!("overlap regrouping changed {row:?}");
        })
        .unwrap();
    let SourceVisit::Complete { version, work } = observed else {
        panic!("complete overlapping regrouping");
    };
    assert_eq!(work.emitted, 0);
    assert_eq!(
        version.content_fingerprint(),
        previous.content_fingerprint()
    );
    assert_eq!(
        version.content_fingerprint(),
        query
            .source_version()
            .unwrap()
            .unwrap()
            .content_fingerprint()
    );
}

#[test]
fn regrouping_removals_and_read_refusal_preserve_the_original_source() {
    let data = dataset(48, false, false);
    let removed = dataset(1, false, false);
    let empty = purrdf_core::RdfDatasetBuilder::new().freeze().unwrap();
    let mut stack = SegmentedStack::default();
    stack
        .publish(
            vec![
                sealed(&data),
                sealed_with_metadata(
                    &empty,
                    |_| true,
                    SegmentedLayerMetadata::new(Some(Arc::clone(&removed)), None).unwrap(),
                ),
            ],
            limits(),
        )
        .unwrap();
    let before = stack.snapshot().unwrap();
    let previous = before
        .query(limits())
        .unwrap()
        .source_version()
        .unwrap()
        .unwrap();
    let original = previous.content_fingerprint();
    stack
        .publish(
            vec![
                sealed_partition(&data, 3, &[0]),
                sealed_partition(&data, 3, &[1, 2]),
            ],
            limits(),
        )
        .unwrap();
    let after = stack.snapshot().unwrap();
    let constrained = after
        .query(SegmentedReadLimits::new(
            256_000_000,
            8,
            100,
            256_000_000,
            32,
        ))
        .unwrap();
    assert!(matches!(
        constrained.visit_source_changes(&previous, |_, _, _| ControlFlow::<()>::Continue(())),
        Err(purrdf_core::SegmentedError::EvidenceExhausted)
    ));
    assert_eq!(previous.content_fingerprint(), original);
    let query = after.query(limits()).unwrap();
    let old_rows = surface(&before);
    let new_rows = surface(&snapshot(Arc::clone(&data)));
    let expected: BTreeSet<_> = new_rows.difference(&old_rows).cloned().collect();
    let mut actual = BTreeSet::new();
    let observed = query
        .visit_source_changes(&previous, |side, family, row| {
            assert_eq!(side, SourceSide::Next);
            actual.insert((u8::from(family == SourceFamily::Reifiers), value(row)));
            ControlFlow::<()>::Continue(())
        })
        .unwrap();
    let SourceVisit::Complete { version, .. } = observed else {
        panic!("complete regrouping after refusal");
    };
    assert_eq!(actual, expected);
    assert!(!actual.is_empty());
    assert_eq!(
        version.content_fingerprint(),
        query
            .source_version()
            .unwrap()
            .unwrap()
            .content_fingerprint()
    );
    stack
        .publish(
            vec![
                sealed_partition(&data, 3, &[0]),
                sealed_with_metadata(
                    &empty,
                    |_| true,
                    SegmentedLayerMetadata::new(Some(data), None).unwrap(),
                ),
            ],
            limits(),
        )
        .unwrap();
    let removed_all = stack.snapshot().unwrap();
    let query = removed_all.query(limits()).unwrap();
    let mut actual = BTreeSet::new();
    let observed = query
        .visit_source_changes(&version, |side, family, row| {
            assert_eq!(side, SourceSide::Previous);
            actual.insert((u8::from(family == SourceFamily::Reifiers), value(row)));
            ControlFlow::<()>::Continue(())
        })
        .unwrap();
    let SourceVisit::Complete { version, .. } = observed else {
        panic!("complete regrouping into removals");
    };
    assert_eq!(actual, new_rows);
    assert_eq!(version.statement_count(), 0);
    assert!(surface(&removed_all).is_empty());
    assert_eq!(
        version.content_fingerprint(),
        query
            .source_version()
            .unwrap()
            .unwrap()
            .content_fingerprint()
    );
}

#[test]
fn sealed_regrouping_with_a_changed_large_head_skips_its_retained_rows() {
    for count in [96, 384, 1536] {
        let base = dataset(count, false, false);
        let old_head = dataset(count * 2, false, false);
        let new_head = dataset(count * 2, true, true);
        let mut stack = SegmentedStack::default();
        stack.publish(vec![sealed(&base)], limits()).unwrap();
        stack.set_head(Some(
            SegmentedResidentHead::new(Arc::clone(&old_head), SegmentedLayerMetadata::default())
                .unwrap(),
        ));
        let before = stack.snapshot().unwrap();
        let previous = before
            .query(limits())
            .unwrap()
            .source_version()
            .unwrap()
            .unwrap();
        stack
            .publish(
                (0..3)
                    .map(|part| sealed_partition(&base, 3, &[part]))
                    .collect(),
                limits(),
            )
            .unwrap();
        stack.set_head(Some(
            SegmentedResidentHead::new(Arc::clone(&new_head), SegmentedLayerMetadata::default())
                .unwrap(),
        ));
        let after = stack.snapshot().unwrap();
        let query = after.query(limits()).unwrap();
        let old_rows = surface(&snapshot(old_head));
        let new_rows = surface(&snapshot(new_head));
        let expected: BTreeSet<_> = old_rows
            .difference(&new_rows)
            .map(|row| (false, row.clone()))
            .chain(
                new_rows
                    .difference(&old_rows)
                    .map(|row| (true, row.clone())),
            )
            .collect();
        let mut actual = BTreeSet::new();
        let observed = query
            .visit_source_changes(&previous, |side, family, row| {
                actual.insert((
                    side == SourceSide::Next,
                    (u8::from(family == SourceFamily::Reifiers), value(row)),
                ));
                ControlFlow::<()>::Continue(())
            })
            .unwrap();
        let SourceVisit::Complete { version, work } = observed else {
            panic!("complete regrouping with changed head");
        };
        assert_eq!(actual, expected);
        assert_eq!(work.emitted, 6);
        assert!(
            work.physical <= 12,
            "retained head enumerated at {count}: {work:?}"
        );
        assert_eq!(
            version.content_fingerprint(),
            query
                .source_version()
                .unwrap()
                .unwrap()
                .content_fingerprint()
        );
    }
}

#[test]
fn changed_sealed_replacements_and_compaction_skip_retained_physical_rows() {
    let mut measurements = Vec::new();
    for compact in [false, true] {
        for count in [128, 1024, 4096] {
            let original = dataset(count, false, false);
            let current = dataset(count, true, true);
            let mut stack = SegmentedStack::default();
            let mut layers = vec![sealed(&original)];
            if compact {
                // This small overlapping tail is folded into the replacement.
                // The next image also changes a different subject in the base.
                layers.push(sealed(&dataset(1, false, false)));
            }
            stack.publish(layers, limits()).unwrap();
            let before = stack.snapshot().unwrap();
            let old_rows = surface(&snapshot(Arc::clone(&original)));
            let version = before
                .query(limits())
                .unwrap()
                .source_version()
                .unwrap()
                .unwrap();
            stack.publish(vec![sealed(&current)], limits()).unwrap();
            let after = stack.snapshot().unwrap();
            let next_rows = surface(&snapshot(Arc::clone(&current)));
            let expected: BTreeSet<_> = old_rows
                .difference(&next_rows)
                .map(|row| (false, row.clone()))
                .chain(
                    next_rows
                        .difference(&old_rows)
                        .map(|row| (true, row.clone())),
                )
                .collect();
            let query = after.query(limits()).unwrap();
            let initial = query.operation_status();
            let initial_requests = initial.evidence().request_count();
            let initial_io = initial.evidence().io_bytes();
            let mut actual = BTreeSet::new();
            let result = query
                .visit_source_changes(&version, |side, family, row| {
                    actual.insert((
                        side == SourceSide::Next,
                        (u8::from(family == SourceFamily::Reifiers), value(row)),
                    ));
                    ControlFlow::<()>::Continue(())
                })
                .unwrap();
            let SourceVisit::Complete { version, work } = result else {
                panic!("complete sealed replacement");
            };
            assert_eq!(actual, expected);
            assert_eq!(work.emitted, 6);
            assert_eq!(work.directory.changed, 6);
            assert_eq!(work.physical, 6);
            let status = query.operation_status();
            assert!(status.error().is_none());
            let requests = status.evidence().request_count() - initial_requests;
            let io = status.evidence().io_bytes() - initial_io;
            assert_eq!(
                version.content_fingerprint(),
                query
                    .source_version()
                    .unwrap()
                    .unwrap()
                    .content_fingerprint()
            );
            measurements.push((compact, count, work, requests, io));
        }
    }
    eprintln!("sealed source replacement work: {measurements:?}");
    for group in measurements.as_chunks::<3>().0 {
        assert!(group[2].2.directory.ranges < group[0].2.directory.ranges * 4);
        assert!(group[2].2.directory.members < group[0].2.directory.members * 4);
        assert!(group[2].3 < group[0].3 * 4);
        assert!(group[2].4 < group[0].4 * 4);
    }
}

#[test]
fn changed_full_compaction_folds_large_layers_and_heads_without_a_source_walk() {
    let mut measurements = Vec::new();
    for head_mode in 0..4 {
        for count in [128, 1024, 4096] {
            let original = dataset(count, false, false);
            let old_head = Arc::downgrade(&original);
            let current = dataset(count, true, true);
            let mut stack = SegmentedStack::default();
            let layers = vec![sealed(&original), sealed(&original), sealed(&original)];
            let old_layer = Arc::downgrade(&layers[0]);
            stack.publish(layers, limits()).unwrap();
            stack.set_head(Some(
                SegmentedResidentHead::new(
                    Arc::clone(&original),
                    SegmentedLayerMetadata::default(),
                )
                .unwrap(),
            ));
            let before = stack.snapshot().unwrap();
            let previous = before
                .query(limits())
                .unwrap()
                .source_version()
                .unwrap()
                .unwrap();
            let old_rows = surface(&snapshot(Arc::clone(&original)));
            let next_rows = surface(&snapshot(Arc::clone(&current)));
            let expected: BTreeSet<_> = old_rows
                .difference(&next_rows)
                .map(|row| (false, row.clone()))
                .chain(
                    next_rows
                        .difference(&old_rows)
                        .map(|row| (true, row.clone())),
                )
                .collect();
            let head = match head_mode {
                0 => None,
                1 | 3 => Some(Arc::clone(&current)),
                2 => Some(dataset(1, false, false)),
                _ => unreachable!(),
            };
            stack.set_head(head.map(|data| {
                SegmentedResidentHead::new(data, SegmentedLayerMetadata::default()).unwrap()
            }));
            stack
                .publish(
                    if head_mode == 3 {
                        Vec::new()
                    } else {
                        vec![sealed(&current)]
                    },
                    limits(),
                )
                .unwrap();
            let after = stack.snapshot().unwrap();
            let query = after.query(limits()).unwrap();
            let initial = query.operation_status();
            let mut actual = BTreeSet::new();
            let observed = query
                .visit_source_changes(&previous, |side, family, row| {
                    actual.insert((
                        side == SourceSide::Next,
                        (u8::from(family == SourceFamily::Reifiers), value(row)),
                    ));
                    ControlFlow::<()>::Continue(())
                })
                .unwrap();
            let SourceVisit::Complete { version, work } = observed else {
                panic!("complete changed compaction");
            };
            let status = query.operation_status();
            assert_eq!(actual, expected);
            assert_eq!(work.emitted, 6);
            assert_eq!(work.candidates, if head_mode == 2 { 9 } else { 6 });
            assert_eq!(work.physical, [6, 12, 9, 6][head_mode]);
            assert_eq!(
                version.content_fingerprint(),
                query
                    .source_version()
                    .unwrap()
                    .unwrap()
                    .content_fingerprint()
            );
            let requests = status.evidence().request_count() - initial.evidence().request_count();
            let io = status.evidence().io_bytes() - initial.evidence().io_bytes();
            drop(previous);
            drop(before);
            drop(original);
            assert!(
                old_head.upgrade().is_none(),
                "successor retained its predecessor's head"
            );
            assert!(
                old_layer.upgrade().is_none(),
                "successor retained a retired layer"
            );
            measurements.push((head_mode, count, work, requests, io));
        }
    }
    eprintln!("changed full compaction work: {measurements:?}");
    for group in measurements.as_chunks::<3>().0 {
        assert!(group[2].2.directory.ranges < group[0].2.directory.ranges * 4);
        assert!(group[2].3 < group[0].3 * 4);
        assert!(group[2].4 < group[0].4 * 4);
    }
}

#[test]
fn changed_prefix_compaction_retains_a_large_suffix_and_its_next_update() {
    let mut measurements = Vec::new();
    for count in [128, 1024, 4096] {
        let original = dataset(count, false, false);
        let current = dataset(count, true, true);
        // This large suffix overlaps every row in the first half. It excludes
        // the changed subject, whose ordinal is count/2, so both full-world
        // oracles are the independently constructed original/current datasets.
        let suffix = sealed(&dataset(count / 2, false, true));
        let mut stack = SegmentedStack::default();
        stack
            .publish(
                vec![sealed(&original), sealed(&original), Arc::clone(&suffix)],
                limits(),
            )
            .unwrap();
        let before = stack.snapshot().unwrap();
        let previous = before
            .query(limits())
            .unwrap()
            .source_version()
            .unwrap()
            .unwrap();
        let old_rows = surface(&snapshot(Arc::clone(&original)));
        let next_rows = surface(&snapshot(Arc::clone(&current)));
        let expected: BTreeSet<_> = old_rows
            .difference(&next_rows)
            .map(|row| (false, row.clone()))
            .chain(
                next_rows
                    .difference(&old_rows)
                    .map(|row| (true, row.clone())),
            )
            .collect();
        stack
            .publish(vec![sealed(&current), Arc::clone(&suffix)], limits())
            .unwrap();
        let after = stack.snapshot().unwrap();
        let query = after.query(limits()).unwrap();
        let initial = query.operation_status();
        let mut actual = BTreeSet::new();
        let observed = query
            .visit_source_changes(&previous, |side, family, row| {
                actual.insert((
                    side == SourceSide::Next,
                    (u8::from(family == SourceFamily::Reifiers), value(row)),
                ));
                ControlFlow::<()>::Continue(())
            })
            .unwrap();
        let SourceVisit::Complete { version, work } = observed else {
            panic!("complete prefix compaction");
        };
        let status = query.operation_status();
        assert_eq!(actual, expected);
        assert_eq!((work.candidates, work.emitted, work.physical), (6, 6, 6));
        assert_eq!(
            version.content_fingerprint(),
            query
                .source_version()
                .unwrap()
                .unwrap()
                .content_fingerprint()
        );
        let requests = status.evidence().request_count() - initial.evidence().request_count();
        let io = status.evidence().io_bytes() - initial.evidence().io_bytes();
        // Reuse the newly maintained prefixes in a second changed compaction.
        stack
            .publish(vec![sealed(&original), suffix], limits())
            .unwrap();
        let restored = stack.snapshot().unwrap();
        let restored = restored.query(limits()).unwrap();
        let observed = restored
            .visit_source_changes(&version, |_, _, _| ControlFlow::<()>::Continue(()))
            .unwrap();
        let SourceVisit::Complete {
            version,
            work: second,
        } = observed
        else {
            panic!("complete next prefix compaction");
        };
        assert_eq!(
            (second.candidates, second.emitted, second.physical),
            (6, 6, 6)
        );
        assert_eq!(
            version.content_fingerprint(),
            previous.content_fingerprint()
        );
        measurements.push((count, work, requests, io));
    }
    eprintln!("changed prefix compaction work: {measurements:?}");
    assert!(measurements[2].2 < measurements[0].2 * 4);
    assert!(measurements[2].3 < measurements[0].3 * 4);
}

#[test]
fn sealing_with_a_fresh_head_keeps_the_complete_prefix_for_later_compaction() {
    for count in [128, 512, 2048] {
        let original = dataset(count, false, false);
        let head = dataset(count * 2, false, false);
        let base = sealed(&original);
        let mut stack = SegmentedStack::default();
        stack.publish(vec![Arc::clone(&base)], limits()).unwrap();
        stack.set_head(Some(
            SegmentedResidentHead::new(Arc::clone(&head), SegmentedLayerMetadata::default())
                .unwrap(),
        ));
        let initial = stack.snapshot().unwrap();
        let mut previous = initial
            .query(limits())
            .unwrap()
            .source_version()
            .unwrap()
            .unwrap();
        let sealed_head = sealed(&head);
        let tiny = dataset(1, false, false);
        stack
            .publish(vec![Arc::clone(&base), Arc::clone(&sealed_head)], limits())
            .unwrap();
        stack.set_head(Some(
            SegmentedResidentHead::new(Arc::clone(&tiny), SegmentedLayerMetadata::default())
                .unwrap(),
        ));
        let first = stack.snapshot().unwrap();
        let observed = first
            .query(limits())
            .unwrap()
            .visit_source_changes(&previous, |_, _, row| -> ControlFlow<()> {
                panic!("head sealing changed {row:?}");
            })
            .unwrap();
        let SourceVisit::Complete { version, .. } = observed else {
            panic!("complete first sealing");
        };
        previous = version;
        let suffix = sealed(&tiny);
        stack
            .publish(vec![base, sealed_head, Arc::clone(&suffix)], limits())
            .unwrap();
        stack.set_head(Some(
            SegmentedResidentHead::new(
                purrdf_core::RdfDatasetBuilder::new().freeze().unwrap(),
                SegmentedLayerMetadata::default(),
            )
            .unwrap(),
        ));
        let second = stack.snapshot().unwrap();
        let observed = second
            .query(limits())
            .unwrap()
            .visit_source_changes(&previous, |_, _, row| -> ControlFlow<()> {
                panic!("second sealing changed {row:?}");
            })
            .unwrap();
        let SourceVisit::Complete { version, .. } = observed else {
            panic!("complete second sealing");
        };
        previous = version;
        stack
            .publish(
                vec![sealed(&dataset(count * 2, true, true)), suffix],
                limits(),
            )
            .unwrap();
        let next = stack.snapshot().unwrap();
        let query = next.query(limits()).unwrap();
        let observed = query
            .visit_source_changes(&previous, |_, _, _| ControlFlow::<()>::Continue(()))
            .unwrap();
        let SourceVisit::Complete { version, work } = observed else {
            panic!("complete post-sealing compaction");
        };
        assert_eq!((work.physical, work.candidates, work.emitted), (6, 6, 6));
        assert_eq!(
            version.content_fingerprint(),
            query
                .source_version()
                .unwrap()
                .unwrap()
                .content_fingerprint()
        );
    }
}

#[test]
fn observations_preserve_sealing_overlap_and_compaction_without_rewalking_retained_rows() {
    let data = dataset(64, false, false);
    let resident = snapshot(Arc::clone(&data));
    let expected = surface(&resident);
    let mut version = resident
        .query(limits())
        .unwrap()
        .source_version()
        .unwrap()
        .unwrap();
    let layer = sealed(&data);
    let mut stack = SegmentedStack::default();
    stack.publish(vec![Arc::clone(&layer)], limits()).unwrap();
    let snapshots = [
        stack.snapshot().unwrap(),
        {
            stack
                .publish(
                    vec![Arc::clone(&layer), sealed(&dataset(64, false, true))],
                    limits(),
                )
                .unwrap();
            stack.snapshot().unwrap()
        },
        {
            stack
                .publish(vec![sealed(&dataset(64, false, true))], limits())
                .unwrap();
            stack.snapshot().unwrap()
        },
    ];
    for (step, next) in snapshots.into_iter().enumerate() {
        assert_eq!(surface(&next), expected);
        let query = next.query(limits()).unwrap();
        let result = query
            .visit_source_changes(&version, |_, _, row| -> ControlFlow<()> {
                panic!("unchanged surface emitted {row:?}");
            })
            .unwrap();
        let SourceVisit::Complete {
            version: observed,
            work,
        } = result
        else {
            panic!("complete observation");
        };
        assert_eq!(
            observed.content_fingerprint(),
            version.content_fingerprint()
        );
        assert_eq!(
            observed.content_fingerprint(),
            query
                .source_version()
                .unwrap()
                .unwrap()
                .content_fingerprint()
        );
        assert_eq!(work.emitted, 0);
        if step != 1 {
            assert_eq!(
                work.physical, 0,
                "sealing/compaction must reuse native content evidence"
            );
        }
        version = observed;
    }
}

#[test]
fn full_compaction_with_the_same_resident_head_does_not_enumerate_that_head() {
    let original = dataset(64, false, false);
    let current = dataset(64, true, true);
    let mut stack = SegmentedStack::default();
    stack.publish(vec![sealed(&original)], limits()).unwrap();
    stack.set_head(Some(
        SegmentedResidentHead::new(
            Arc::clone(&current),
            SegmentedLayerMetadata::new(Some(original), None).unwrap(),
        )
        .unwrap(),
    ));
    let before = stack.snapshot().unwrap();
    let expected = surface(&before);
    assert_eq!(expected, surface(&snapshot(Arc::clone(&current))));
    let previous = before
        .query(limits())
        .unwrap()
        .source_version()
        .unwrap()
        .unwrap();
    stack.publish(vec![sealed(&current)], limits()).unwrap();
    let after = stack.snapshot().unwrap();
    assert_eq!(surface(&after), expected);
    let query = after.query(limits()).unwrap();
    let observed = query
        .visit_source_changes(&previous, |_, _, row| -> ControlFlow<()> {
            panic!("unchanged head emitted {row:?}");
        })
        .unwrap();
    let SourceVisit::Complete { version, work } = observed else {
        panic!("complete compacted source observation");
    };
    assert_eq!(work.physical, 0);
    assert_eq!(work.candidates, 0);
    assert_eq!(work.emitted, 0);
    assert_eq!(
        version.content_fingerprint(),
        previous.content_fingerprint()
    );
    assert_eq!(
        version.content_fingerprint(),
        query
            .source_version()
            .unwrap()
            .unwrap()
            .content_fingerprint()
    );
}

#[test]
fn stopped_observation_does_not_publish_a_partial_successor() {
    let old = snapshot(dataset(64, false, false));
    let next = snapshot(dataset(64, true, true));
    let version = old
        .query(limits())
        .unwrap()
        .source_version()
        .unwrap()
        .unwrap();
    let original = version.content_fingerprint();
    let query = next.query(limits()).unwrap();
    let stopped = query
        .visit_source_changes(&version, |_, _, _| ControlFlow::Break(37))
        .unwrap();
    assert!(matches!(stopped, SourceVisit::Stopped(37)));
    assert_eq!(version.content_fingerprint(), original);
    let complete = query
        .visit_source_changes(&version, |_, _, _| ControlFlow::<()>::Continue(()))
        .unwrap();
    let SourceVisit::Complete {
        version: actual,
        work,
    } = complete
    else {
        panic!("complete transition after consumer stop");
    };
    assert_eq!(work.emitted, 6);
    assert_eq!(
        actual.content_fingerprint(),
        query
            .source_version()
            .unwrap()
            .unwrap()
            .content_fingerprint()
    );
}

#[test]
fn successor_observation_releases_predecessor_head_and_retains_only_its_own_owner() {
    let data = dataset(8, false, false);
    let old_owner = Arc::downgrade(&data);
    let old = snapshot(data);
    let previous = old
        .query(limits())
        .unwrap()
        .source_version()
        .unwrap()
        .unwrap();
    drop(old);
    assert!(old_owner.upgrade().is_some());
    let data = dataset(8, true, true);
    let next_owner = Arc::downgrade(&data);
    let next = snapshot(data);
    let query = next.query(limits()).unwrap();
    let result = query
        .visit_source_changes(&previous, |_, _, _| ControlFlow::<()>::Continue(()))
        .unwrap();
    let SourceVisit::Complete { version, .. } = result else {
        panic!("complete successor");
    };
    drop(previous);
    assert!(
        old_owner.upgrade().is_none(),
        "successor must not retain a chain of historical heads"
    );
    drop(query);
    drop(next);
    assert!(next_owner.upgrade().is_some());
    drop(version);
    assert!(next_owner.upgrade().is_none());
}

#[test]
fn effective_removals_are_observed_and_readding_restores_only_the_original_rows() {
    let original = dataset(2, false, false);
    let mut stack = SegmentedStack::default();
    stack.publish(vec![sealed(&original)], limits()).unwrap();
    let before = stack.snapshot().unwrap();
    let version = before
        .query(limits())
        .unwrap()
        .source_version()
        .unwrap()
        .unwrap();
    let empty = purrdf_core::RdfDatasetBuilder::new().freeze().unwrap();
    stack.set_head(Some(
        SegmentedResidentHead::new(
            empty,
            SegmentedLayerMetadata::new(Some(Arc::clone(&original)), None).unwrap(),
        )
        .unwrap(),
    ));
    let removed = stack.snapshot().unwrap();
    assert!(surface(&removed).is_empty());
    let query = removed.query(limits()).unwrap();
    let mut observed = BTreeSet::new();
    let result = query
        .visit_source_changes(&version, |side, family, row| {
            assert_eq!(side, SourceSide::Previous);
            observed.insert((u8::from(family == SourceFamily::Reifiers), value(row)));
            ControlFlow::<()>::Continue(())
        })
        .unwrap();
    assert_eq!(observed, surface(&before));
    let SourceVisit::Complete { version, .. } = result else {
        panic!("removed source observation");
    };
    assert_eq!(version.statement_count(), 0);
    assert_eq!(
        version.content_fingerprint(),
        query
            .source_version()
            .unwrap()
            .unwrap()
            .content_fingerprint()
    );
    stack.set_head(Some(
        SegmentedResidentHead::new(
            Arc::clone(&original),
            SegmentedLayerMetadata::new(Some(original), None).unwrap(),
        )
        .unwrap(),
    ));
    let restored = stack.snapshot().unwrap();
    let query = restored.query(limits()).unwrap();
    let mut actual = BTreeSet::new();
    let result = query
        .visit_source_changes(&version, |side, family, row| {
            assert_eq!(side, SourceSide::Next);
            actual.insert((u8::from(family == SourceFamily::Reifiers), value(row)));
            ControlFlow::<()>::Continue(())
        })
        .unwrap();
    assert_eq!(actual, surface(&before));
    let SourceVisit::Complete { version, .. } = result else {
        panic!("restored source observation");
    };
    assert_eq!(version.statement_count(), actual.len() as u128);
    assert_eq!(
        version.content_fingerprint(),
        query
            .source_version()
            .unwrap()
            .unwrap()
            .content_fingerprint()
    );
}

#[test]
fn head_observation_reports_actual_changes_and_reuses_an_unchanged_snapshot() {
    let old = snapshot(dataset(129, false, false));
    let next = snapshot(dataset(129, true, true));
    let old_rows = surface(&old);
    let next_rows = surface(&next);
    let expected: BTreeSet<_> = old_rows
        .difference(&next_rows)
        .map(|row| (false, row.clone()))
        .chain(
            next_rows
                .difference(&old_rows)
                .map(|row| (true, row.clone())),
        )
        .collect();
    let query = old.query(limits()).unwrap();
    let version = query.source_version().unwrap().unwrap();
    assert_eq!(version.statement_count(), old_rows.len() as u128);
    drop(query);
    let query = next.query(limits()).unwrap();
    let mut actual = BTreeSet::new();
    let result = query
        .visit_source_changes(&version, |side, family, row| {
            actual.insert((
                side == SourceSide::Next,
                (u8::from(family == SourceFamily::Reifiers), value(row)),
            ));
            ControlFlow::<std::convert::Infallible>::Continue(())
        })
        .unwrap();
    let SourceVisit::Complete { version, work } = result else {
        panic!("native source must observe its change");
    };
    assert_eq!(actual, expected);
    assert_eq!(work.directory.changed, 6);
    assert_eq!(version.statement_count(), next_rows.len() as u128);
    assert_eq!(
        version.content_fingerprint(),
        query
            .source_version()
            .unwrap()
            .unwrap()
            .content_fingerprint()
    );
    let same = query
        .visit_source_changes(&version, |_, _, _| -> ControlFlow<()> {
            panic!("same snapshot has no changes");
        })
        .unwrap();
    assert!(
        matches!(same, SourceVisit::Complete { work, .. } if work.physical == 0 && work.candidates == 0)
    );
}

#[test]
fn changed_head_observation_work_grows_with_digest_prefixes() {
    let mut measurements = Vec::new();
    for count in [128, 2048] {
        let old = snapshot(dataset(count, false, false));
        let next = snapshot(dataset(count, true, true));
        let version = old
            .query(limits())
            .unwrap()
            .source_version()
            .unwrap()
            .unwrap();
        let query = next.query(limits()).unwrap();
        let result = query
            .visit_source_changes(&version, |_, _, _| {
                ControlFlow::<std::convert::Infallible>::Continue(())
            })
            .unwrap();
        let SourceVisit::Complete { version, work } = result else {
            panic!("complete transition");
        };
        assert_eq!(version.statement_count(), count as u128 * 3);
        assert_eq!(
            version.content_fingerprint(),
            query
                .source_version()
                .unwrap()
                .unwrap()
                .content_fingerprint()
        );
        assert_eq!(work.physical, 6);
        assert_eq!(work.candidates, 6);
        assert_eq!(work.emitted, 6);
        measurements.push((count, work, query.touched_terms()));
    }
    eprintln!("chronological head observation: {measurements:?}");
    assert!(measurements[1].1.directory.members < measurements[0].1.directory.members * 3);
    assert_eq!(measurements[0].2, measurements[1].2);
}
