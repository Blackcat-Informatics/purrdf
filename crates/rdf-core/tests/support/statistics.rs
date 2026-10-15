// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Compare chronological metadata with actual independently asserted surfaces.

use purrdf_core::{DatasetView, GraphMatch, ProbePattern, StatisticsAccuracy, StatisticsStream};

pub(super) fn assert_chronological_counts<D: DatasetView>(
    view: &D,
    predicate: D::Id,
    expected: [usize; 3],
    layered: bool,
) -> u64 {
    let statistics = view.statistics().expect("actual stack exposes statistics");
    let revision = statistics.fingerprint();
    let selected = [
        view.quads().filter(|row| row.p == predicate).count(),
        view.reifier_quads()
            .filter(|row| row.p == predicate)
            .count(),
        view.annotation_quads()
            .filter(|row| row.p == predicate)
            .count(),
    ];
    for ((stream, rows), selected) in [
        StatisticsStream::Asserted,
        StatisticsStream::Reifiers,
        StatisticsStream::Annotations,
    ]
    .into_iter()
    .zip(expected)
    .zip(selected)
    {
        let total = statistics.stream_rows(GraphMatch::Any, stream).unwrap();
        let marginal = statistics
            .predicate(predicate, GraphMatch::Any, stream)
            .unwrap();
        for (count, actual) in [(total, rows), (marginal.rows, selected)] {
            if count.accuracy == StatisticsAccuracy::Exact {
                assert_eq!(count.value, actual as u64);
            } else {
                assert!(layered);
                assert!(count.value >= actual as u64);
            }
        }
        if layered {
            assert!(marginal.subjects.is_none());
            assert!(marginal.objects.is_none());
        } else {
            assert_eq!(total.accuracy, StatisticsAccuracy::Exact);
            assert!(marginal.subjects.is_some());
            assert!(marginal.objects.is_some());
        }
    }
    let count = statistics
        .pattern(ProbePattern {
            s: None,
            p: None,
            o: None,
            g: GraphMatch::Any,
        })
        .unwrap();
    let actual = expected.into_iter().sum::<usize>() as u64;
    if layered {
        assert!(count.value >= actual);
    } else {
        assert_eq!(count.value, actual);
        assert_eq!(count.accuracy, StatisticsAccuracy::Exact);
    }
    assert_eq!(
        statistics.fingerprint(),
        revision,
        "reads do not change the snapshot revision"
    );
    assert!(view.read_error().is_none());
    revision
}
