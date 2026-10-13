// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Actual dictionary/window/calendar producers with literal preflight controls.
//! Timings and original source/transition/output/allocation counts stay separate.

#[path = "../tests/support/shape_cases.rs"]
#[expect(
    dead_code,
    reason = "this bench reuses the native/portable corpus constructors; its full acceptance functions run in the shared test target"
)]
mod shape_cases;
use purrdf_sparql_eval::{NativeSparqlEngine, QueryOptions, WorkspaceCapability};
use purrdf_testkit::bench::{Bench, Throughput, bench_group, bench_main, black_box};
use purrdf_text::dictionary::{DictionaryCache, DictionaryEntry};
use shape_cases::{Fixture, bytes, engine, phrase_query, positions, request, window_query};
use std::time::Duration;

#[global_allocator]
static GLOBAL: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;

fn dictionary(bench: &mut Bench) {
    let mut group = bench.benchmark_group("physical_dictionary");
    group
        .sample_size(20)
        .warm_up_time(Duration::from_millis(100))
        .measurement_time(Duration::from_millis(300));
    for tokens in [256usize, 2_048, 16_384] {
        let text = std::iter::repeat_n("lana audley", tokens / 2)
            .collect::<Vec<_>>()
            .join(" ");
        let fixture = Fixture::text(&[("https://example.org/document".into(), text, None, false)]);
        for count in [1usize, 64, 534, 4_097] {
            let phrases: Vec<_> = (0..count)
                .map(|index| {
                    if index == 0 {
                        "lana audley".to_owned()
                    } else {
                        format!("absent token{index}")
                    }
                })
                .collect();
            let identities: Vec<_> = (0..count)
                .map(|index| {
                    purrdf_core::TermValue::iri(format!("https://example.org/entry/{index}"))
                })
                .collect();
            let entries: Vec<_> = identities
                .iter()
                .zip(&phrases)
                .map(|(identity, phrase)| DictionaryEntry {
                    identity,
                    phrase,
                    payload: &[],
                })
                .collect();
            let capability = WorkspaceCapability::resident();
            let cache = DictionaryCache::default();
            let compiled = cache.get(&fixture.index, &entries, &capability).unwrap();
            let matches = compiled
                .scan(fixture.index.clone(), None, &capability)
                .unwrap();
            assert_eq!(matches.work().source_visits, tokens as u64);
            assert_eq!(matches.work().output_rows, (tokens / 2) as u64);
            let query = phrase_query(&phrases);
            let expected = fixture.equal(&query);
            let allocation = purrdf_alloc_probe::CurrentThreadWindow::open();
            let result = fixture.answer(&query, false);
            let measured = allocation.close();
            assert_eq!(bytes(&result), bytes(&expected));
            let report = fixture.report(&query, "dictionary-match");
            eprintln!(
                "dictionary entries={count} tokens={tokens} work={:?} allocations={measured:?} plan={:?}",
                matches.work(),
                report.shapes()
            );
            group.throughput(Throughput::Elements(tokens as u64));
            group.bench_function(format!("scan/{tokens}/{count}"), |b| {
                b.iter(|| {
                    black_box(
                        compiled
                            .scan(fixture.index.clone(), None, &capability)
                            .unwrap(),
                    )
                });
            });
            group.bench_function(format!("cache_hit/{tokens}/{count}"), |b| {
                b.iter(|| {
                    black_box(
                        cache
                            .get(&fixture.index, black_box(&entries), &capability)
                            .unwrap(),
                    )
                });
            });
            group.bench_function(format!("cache_miss/{tokens}/{count}"), |b| {
                b.iter(|| {
                    cache.clear();
                    black_box(
                        cache
                            .get(&fixture.index, black_box(&entries), &capability)
                            .unwrap(),
                    )
                });
            });
            group.bench_function(format!("public_query/{tokens}/{count}"), |b| {
                b.iter(|| black_box(fixture.answer(black_box(&query), false)));
            });
        }
    }
    group.finish();
}

fn windows(bench: &mut Bench) {
    let mut group = bench.benchmark_group("physical_window");
    group
        .sample_size(20)
        .warm_up_time(Duration::from_millis(100))
        .measurement_time(Duration::from_millis(300));
    for count in [64usize, 256, 1_024] {
        let dataset = positions(
            &(0..count)
                .map(|index| index.to_string())
                .collect::<Vec<_>>(),
        );
        for width in [0, 2, 16] {
            let query = window_query(&width.to_string());
            let literal = engine(true)
                .query_with_options_view(&*dataset, request(&query), QueryOptions::EMPTY)
                .unwrap();
            let allocation = purrdf_alloc_probe::CurrentThreadWindow::open();
            let shaped = engine(false)
                .query_with_options_view(&*dataset, request(&query), QueryOptions::EMPTY)
                .unwrap();
            let measured = allocation.close();
            assert_eq!(bytes(&shaped), bytes(&literal));
            let report = engine(false).explain_query(&dataset, &query, None).unwrap();
            assert!(
                report
                    .shapes()
                    .iter()
                    .any(|shape| shape.operator == Some("ordered-window"))
            );
            eprintln!(
                "window rows={count} width={width} allocations={measured:?} work={:?}",
                report.shapes()
            );
            group.throughput(Throughput::Elements(count as u64));
            for disabled in [false, true] {
                let native = engine(disabled);
                group.bench_function(
                    format!(
                        "{}/{count}/{width}",
                        if disabled { "literal" } else { "ordered" }
                    ),
                    |b| {
                        b.iter(|| {
                            black_box(
                                native
                                    .query_with_options_view(
                                        &*dataset,
                                        request(black_box(&query)),
                                        QueryOptions::EMPTY,
                                    )
                                    .unwrap(),
                            )
                        });
                    },
                );
            }
        }
    }
    group.finish();
}

fn buckets(bench: &mut Bench) {
    let dataset = purrdf_core::RdfDatasetBuilder::new().freeze().unwrap();
    let mut group = bench.benchmark_group("physical_bucket");
    group
        .sample_size(20)
        .warm_up_time(Duration::from_millis(100))
        .measurement_time(Duration::from_millis(300));
    for count in [128usize, 1_024, 8_192] {
        let values = (0..count)
            .map(|index| {
                format!(
                    "(\"2024-{:02}-{:02}T12:00:00Z\"^^<{}> {})",
                    1 + index % 12,
                    1 + index % 28,
                    purrdf_xsd::datatype::XSD_DATE_TIME_STAMP,
                    index % 11
                )
            })
            .collect::<Vec<_>>()
            .join(" ");
        for target in [
            purrdf_xsd::datatype::XSD_DATE,
            purrdf_xsd::datatype::XSD_G_YEAR_MONTH,
        ] {
            let query = format!(
                "SELECT ?bucket (COUNT(*) AS ?count) (SUM(?n/3) AS ?total) (AVG(?n/3) AS ?mean) (SAMPLE(?n) AS ?first) WHERE {{ VALUES (?time ?n) {{ {values} }} BIND(<{target}>(?time) AS ?bucket) }} GROUP BY ?bucket"
            );
            let literal = engine(true)
                .query_with_options_view(&*dataset, request(&query), QueryOptions::EMPTY)
                .unwrap();
            let allocation = purrdf_alloc_probe::CurrentThreadWindow::open();
            let shaped = engine(false)
                .query_with_options_view(&*dataset, request(&query), QueryOptions::EMPTY)
                .unwrap();
            let measured = allocation.close();
            assert_eq!(bytes(&shaped), bytes(&literal));
            let report = engine(false).explain_query(&dataset, &query, None).unwrap();
            assert!(
                report
                    .shapes()
                    .iter()
                    .any(|shape| shape.operator == Some("typed-bucketed-series"))
            );
            eprintln!(
                "bucket rows={count} datatype={target} allocations={measured:?} work={:?}",
                report.shapes()
            );
            group.throughput(Throughput::Elements(count as u64));
            for disabled in [false, true] {
                let native: NativeSparqlEngine = engine(disabled);
                group.bench_function(
                    format!(
                        "{}/{count}/{}",
                        if disabled { "literal" } else { "ordered" },
                        if target == purrdf_xsd::datatype::XSD_DATE {
                            "day"
                        } else {
                            "month"
                        }
                    ),
                    |b| {
                        b.iter(|| {
                            black_box(
                                native
                                    .query_with_options_view(
                                        &*dataset,
                                        request(black_box(&query)),
                                        QueryOptions::EMPTY,
                                    )
                                    .unwrap(),
                            )
                        });
                    },
                );
            }
        }
    }
    group.finish();
}

bench_group!(benches, dictionary, windows, buckets);
bench_main!(benches);
