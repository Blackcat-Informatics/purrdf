// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Source-aware index construction and exact positional versus exhaustive lookup.
//! Every timed query's admission and complete results are checked before timing.

use std::time::Duration;

use purrdf_testkit::bench::{Bench, Throughput, bench_group, bench_main, black_box};
use purrdf_text::{Analyzer, SubstringRefusalReason, SurfaceIndex, TextError};

fn corpus(size: usize) -> Vec<String> {
    (0..size)
        .map(|id| {
            let suffix = match id % 4 {
                0 => "abcd",
                1 => "abcXbcd",
                2 => "abcbcd",
                _ => "中文👩‍💻",
            };
            format!("rdf{id:04}:{suffix}")
        })
        .collect()
}

fn source_index(rows: &[String]) -> SurfaceIndex {
    SurfaceIndex::from_texts(
        Analyzer::empty_lexicon(),
        rows.iter()
            .enumerate()
            .map(|(id, text)| (id as u32, text.as_str())),
    )
    .expect("valid benchmark corpus")
}

fn construction(bench: &mut Bench) {
    let mut group = bench.benchmark_group("purrdf_text_surface_build");
    group
        .sample_size(30)
        .warm_up_time(Duration::from_millis(100))
        .measurement_time(Duration::from_millis(300));
    for size in [64, 1_024] {
        let rows = corpus(size);
        let bytes: usize = rows.iter().map(String::len).sum();
        group.throughput(Throughput::Bytes(bytes as u64));
        group.bench_function(format!("aligned/{size}"), |b| {
            b.iter(|| black_box(source_index(black_box(&rows))));
        });
    }
    group.finish();
}

fn queries(bench: &mut Bench) {
    let index = source_index(&corpus(1_024));
    let mut group = bench.benchmark_group("purrdf_text_surface_query");
    group
        .sample_size(40)
        .warm_up_time(Duration::from_millis(100))
        .measurement_time(Duration::from_millis(300));
    for (name, query, count) in [
        ("position_selective", "abcd", 256),
        ("rare", "rdf0000", 1),
        ("punctuation", ":中文", 256),
        ("missing_gram", "zzzz", 0),
    ] {
        let indexed = index.substring_report(query).expect("selective query");
        let exhaustive = index
            .substring_exhaustive_report(query)
            .expect("bounded exhaustive query");
        assert_eq!(indexed.matches.len(), count);
        assert_eq!(indexed.matches, exhaustive.matches);
        group.bench_function(format!("indexed/{name}"), |b| {
            b.iter(|| black_box(index.substring_report(black_box(query)).unwrap()));
        });
        group.bench_function(format!("exhaustive/{name}"), |b| {
            b.iter(|| black_box(index.substring_exhaustive_report(black_box(query)).unwrap()));
        });
    }
    assert!(matches!(
        index.substring("rdf"),
        Err(TextError::Substring(refusal))
            if refusal.reason == SubstringRefusalReason::NonSelective
    ));
    group.bench_function("universal_refusal", |b| {
        b.iter(|| black_box(index.substring_report(black_box("rdf")).unwrap_err()));
    });
    group.finish();
}

bench_group!(benches, construction, queries);
bench_main!(benches);
