// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Independently frozen sources, real row changes and fixed-delta work growth.

use std::collections::BTreeSet;

#[path = "support/source_snapshot.rs"]
mod source_snapshot;
use source_snapshot::dataset;

use purrdf_core::source::SourceSide;
use purrdf_core::{QuadIds, RdfDataset, StatisticsStream, TermId, TermValue};
use purrdf_lex::allocation::{Memory, Resident, StorageError};

type Row = (u8, Option<TermValue>, TermValue, TermValue, TermValue);

fn row(dataset: &RdfDataset, stream: StatisticsStream, quad: QuadIds<TermId>) -> Row {
    (
        match stream {
            StatisticsStream::Asserted => 0,
            StatisticsStream::Reifiers => 1,
            StatisticsStream::Annotations => 2,
        },
        quad.g.map(|id| dataset.term_value(id)),
        dataset.term_value(quad.s),
        dataset.term_value(quad.p),
        dataset.term_value(quad.o),
    )
}

fn rows(dataset: &RdfDataset) -> BTreeSet<Row> {
    dataset
        .quads()
        .map(|quad| row(dataset, StatisticsStream::Asserted, quad))
        .chain(
            dataset
                .reifier_quads()
                .map(|quad| row(dataset, StatisticsStream::Reifiers, quad)),
        )
        .chain(
            dataset
                .annotation_quads()
                .map(|quad| row(dataset, StatisticsStream::Annotations, quad)),
        )
        .collect()
}

#[test]
fn independent_native_directories_find_exact_changes_after_all_ids_move() {
    let old = dataset(257, false, false);
    let next = dataset(257, true, true);
    let before = rows(&old);
    let after = rows(&next);
    let expected: BTreeSet<_> = before
        .difference(&after)
        .map(|row| (false, row.clone()))
        .chain(after.difference(&before).map(|row| (true, row.clone())))
        .collect();
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    let mut changes = Vec::new();
    let work = old
        .visit_source_changes_with_memory(&next, &mut memory, |side, stream, row, memory| {
            memory.push(&mut changes, (side, stream, row))
        })
        .unwrap();
    assert_eq!(work.changed, 6);
    let actual = changes
        .into_iter()
        .map(|(side, stream, quad)| {
            (
                side == SourceSide::Next,
                row(
                    if side == SourceSide::Next {
                        &next
                    } else {
                        &old
                    },
                    stream,
                    quad,
                ),
            )
        })
        .collect();
    assert_eq!(expected, actual);
    let same = dataset(257, false, true);
    let unchanged = old
        .visit_source_changes_with_memory(
            &same,
            &mut memory,
            |_, _, _, _| -> Result<(), StorageError> {
                panic!("dictionary rekeying is not a source change")
            },
        )
        .unwrap();
    assert_eq!(unchanged.changed, 0);
    assert_eq!(unchanged.ranges, 1);
}

#[test]
fn fixed_delta_directory_work_follows_prefixes_instead_of_source_population() {
    let mut measurements = Vec::new();
    for count in [128, 2048, 32768] {
        let old = dataset(count, false, false);
        let next = dataset(count, true, true);
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let mut visited = 0;
        let work = old
            .visit_source_changes_with_memory(
                &next,
                &mut memory,
                |_, _, _, _| -> Result<(), StorageError> {
                    visited += 1;
                    Ok(())
                },
            )
            .unwrap();
        assert_eq!(visited, 6);
        assert_eq!(work.changed, visited);
        measurements.push((count, work));
    }
    eprintln!("native directory work: {measurements:?}");
    let first = measurements[0].1;
    let last = measurements[2].1;
    assert!(last.ranges < first.ranges * 4);
    assert!(last.members < first.members * 4);
    assert!(last.members < 32768);
}

#[test]
fn visitor_failure_keeps_its_admitted_output_owned_until_payload_drops() {
    let old = dataset(8, false, false);
    let next = dataset(8, true, true);
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    let mut changes = Vec::new();
    let result =
        old.visit_source_changes_with_memory(&next, &mut memory, |side, stream, row, memory| {
            memory.push(&mut changes, (side, stream, row))?;
            Err::<(), _>(StorageError::SizeOverflow)
        });
    assert!(matches!(result, Err(StorageError::SizeOverflow)));
    assert_eq!(changes.len(), 1);
    assert_eq!(
        memory.admitted_bytes(),
        changes.capacity() * size_of::<(SourceSide, StatisticsStream, QuadIds)>()
    );
    memory.release_vec(changes).unwrap();
    assert_eq!(memory.admitted_bytes(), 0);
}
