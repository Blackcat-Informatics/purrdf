// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Compare query-reused global distance kernels and their preparation costs.
//! Timing is report-only; full-matrix differential tests establish equivalence.

use std::time::Duration;

use purrdf_testkit::bench::{Bench, Throughput, bench_group, bench_main, black_box};
use purrdf_text::phonetic::{DoubleMetaphone, PreparedDistance, PreparedMyers, canonicalize};

fn distance(bench: &mut Bench) {
    let mut group = bench.benchmark_group("purrdf_text_distance");
    group
        .sample_size(40)
        .warm_up_time(Duration::from_millis(150))
        .measurement_time(Duration::from_millis(500));
    for length in [8, 63, 64, 65, 127, 128, 129, 2048] {
        let query: String = "ABCD".chars().cycle().take(length).collect();
        let mut near: Vec<char> = query.chars().collect();
        near[length / 2] = 'X';
        let near: String = near.into_iter().collect();
        let shift = format!("X{}", &query[..query.len() - 1]);
        for threshold in [2, 64] {
            group.throughput(Throughput::Elements(length as u64));
            group.bench_function(format!("prepare_band/{length}/k{threshold}"), |b| {
                b.iter(|| {
                    black_box(
                        PreparedDistance::new(black_box(&query), threshold).expect("bounded query"),
                    )
                });
            });
            group.bench_function(format!("prepare_myers/{length}/k{threshold}"), |b| {
                b.iter(|| {
                    black_box(
                        PreparedMyers::new(black_box(&query), threshold).expect("bounded query"),
                    )
                });
            });
            for (class, candidate) in [("near", near.as_str()), ("shift", shift.as_str())] {
                let mut band = PreparedDistance::new(&query, threshold).expect("bounded query");
                let mut myers = PreparedMyers::new(&query, threshold).expect("bounded query");
                group.bench_function(format!("band/{class}/{length}/k{threshold}"), |b| {
                    b.iter(|| {
                        black_box(
                            band.distance(black_box(candidate))
                                .expect("bounded candidate"),
                        )
                    });
                });
                group.bench_function(format!("myers/{class}/{length}/k{threshold}"), |b| {
                    b.iter(|| {
                        black_box(
                            myers
                                .distance(black_box(candidate))
                                .expect("bounded candidate"),
                        )
                    });
                });
            }
        }
    }
    group.finish();
}

fn spelling(bench: &mut Bench) {
    let coder = DoubleMetaphone::new(4).expect("standard code bound");
    let mut group = bench.benchmark_group("purrdf_text_phonetic");
    for word in ["SCHMIDT", "PHILIPOWICZ", "D’Angelo", "Ångström", "Straße"] {
        group.bench_function(format!("canonical/{word}"), |b| {
            b.iter(|| black_box(canonicalize(black_box(word)).expect("Latin surface")));
        });
        group.bench_function(format!("encode/{word}"), |b| {
            b.iter(|| black_box(coder.encode(black_box(word)).expect("Latin surface")));
        });
    }
    group.finish();
}

bench_group!(benches, distance, spelling);
bench_main!(benches);
