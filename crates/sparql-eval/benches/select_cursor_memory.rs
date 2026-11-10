// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Allocation-only comparison over thirteen original indexed source datasets.
//! Run each mode in its own process: `completed`, `cursor`, `completed-finite`
//! or `cursor-finite`, then optional rows per source. The finite pair includes
//! original coupled governor staging under a non-tripping fuel ceiling. Corpus construction and one identical warm-up precede the window.
//! Append `-join` to measure the same answer through two ordinary subqueries and
//! their keyed join, including retained factors and the streamed answer product.
//! Append `-path` instead for the transitive path over the same source edges.

use purrdf_alloc_probe::{CountingAllocator, WholeProcessWindow};
use purrdf_core::{CompositeDatasetView, RdfDatasetBuilder, SparqlResult, TermValue, ViewLimits};
use purrdf_sparql_eval::engine::{PreparedQuery, PreparedSelectCursor};
use purrdf_sparql_eval::{GovernedOutcome, NativeSparqlEngine, QueryGovernors, QueryOptions};

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

const SOURCES: usize = 13;
const QUERY: &str = "SELECT ?subject ?object WHERE { ?subject <http://example.org/p> ?object }";
const JOIN_QUERY: &str = "SELECT ?subject ?object WHERE { { SELECT ?subject ?object WHERE { ?subject <http://example.org/p> ?object } } { SELECT ?subject WHERE { ?subject <http://example.org/p> ?other } } }";
const PATH_QUERY: &str =
    "SELECT ?subject ?object WHERE { ?subject <http://example.org/p>+ ?object }";

fn corpus(rows: usize) -> CompositeDatasetView {
    let mut sources = Vec::with_capacity(SOURCES);
    for source in 0..SOURCES {
        let mut builder = RdfDatasetBuilder::new();
        let predicate = builder.intern_iri("http://example.org/p").unwrap();
        for row in 0..rows {
            let subject = builder
                .intern_iri(&format!("http://example.org/{source}/s{row}"))
                .unwrap();
            let object = builder
                .intern_iri(&format!("http://example.org/{source}/o{row}"))
                .unwrap();
            builder.push_quad(subject, predicate, object, None).unwrap();
        }
        sources.push(builder.freeze().unwrap());
    }
    CompositeDatasetView::new(sources, ViewLimits::default()).unwrap()
}

fn consume(checksum: &mut u64, cell: Option<&TermValue>) {
    let Some(TermValue::Iri(value)) = cell else {
        panic!("the measured query returns two bound IRI columns");
    };
    *checksum = purrdf_hash::fnv::fold(*checksum, value.as_bytes());
    *checksum = purrdf_hash::fnv::fold(*checksum, &[0xff]);
}

fn execute(
    mode: &str,
    dataset: &CompositeDatasetView,
    engine: &NativeSparqlEngine,
    prepared: &PreparedQuery,
) -> (
    usize,
    u64,
    Option<purrdf_sparql_eval::eval::CursorBufferingReport>,
) {
    let mode = mode
        .strip_suffix("-join")
        .or_else(|| mode.strip_suffix("-path"))
        .unwrap_or(mode);
    let mut checksum = purrdf_hash::fnv::BASIS;
    match mode {
        "completed" | "completed-finite" => {
            let result = if mode == "completed" {
                engine
                    .query_prepared_view(dataset, prepared, &[], &QueryOptions::EMPTY)
                    .unwrap()
            } else {
                let GovernedOutcome::Complete { result, .. } = engine
                    .query_prepared_governed_view(
                        dataset,
                        prepared,
                        &[],
                        &QueryOptions::EMPTY,
                        &QueryGovernors::UNBOUNDED.with_fuel(u64::MAX - 2),
                    )
                    .unwrap()
                else {
                    panic!("the finite measurement budget must not trip");
                };
                result
            };
            let SparqlResult::Solutions { rows, .. } = result else {
                panic!("the prepared query is SELECT");
            };
            for row in &rows {
                for cell in row {
                    consume(&mut checksum, cell.as_ref());
                }
            }
            (rows.len(), checksum, None)
        }
        "cursor" => consume_cursor(
            engine
                .open_select_cursor(dataset, prepared, &[], &QueryOptions::EMPTY)
                .unwrap(),
        ),
        "cursor-finite" => consume_cursor(
            engine
                .open_select_cursor_governed_view(
                    dataset,
                    prepared,
                    &[],
                    &QueryOptions::EMPTY,
                    &QueryGovernors::UNBOUNDED.with_fuel(u64::MAX - 2),
                )
                .unwrap(),
        ),
        _ => panic!("choose completed, cursor, completed-finite or cursor-finite"),
    }
}

fn consume_cursor<R>(
    mut cursor: PreparedSelectCursor<'_, '_, CompositeDatasetView, R>,
) -> (
    usize,
    u64,
    Option<purrdf_sparql_eval::eval::CursorBufferingReport>,
) {
    let mut checksum = purrdf_hash::fnv::BASIS;
    let mut count = 0;
    while let Some(row) = cursor.next_row().unwrap() {
        for cell in row.cells() {
            let value = cell.map(|term| {
                row.context()
                    .scratch
                    .try_value_of(row.context().dataset, term)
                    .unwrap()
            });
            consume(&mut checksum, value.as_ref());
        }
        count += 1;
    }
    let settled = cursor.settle().unwrap();
    assert!(matches!(
        settled.completion,
        purrdf_sparql_eval::eval::SelectCompletion::Complete
    ));
    (count, checksum, Some(cursor.buffering_report()))
}

fn main() {
    let mut arguments = std::env::args().skip(1);
    let mode = arguments
        .next()
        .expect("choose completed, cursor, completed-finite or cursor-finite");
    let execution = mode
        .strip_suffix("-join")
        .or_else(|| mode.strip_suffix("-path"))
        .unwrap_or(&mode);
    assert!(
        matches!(
            execution,
            "completed" | "cursor" | "completed-finite" | "cursor-finite"
        ),
        "choose completed, cursor, completed-finite or cursor-finite"
    );
    let rows: usize = arguments.next().map_or(4096, |value| {
        value.parse().expect("rows per source must be an integer")
    });
    assert!(rows > 0, "rows per source must be positive");
    assert!(
        arguments.next().is_none(),
        "unexpected measurement argument"
    );
    let dataset = corpus(rows);
    let engine = NativeSparqlEngine::new();
    let query = if mode.ends_with("-join") {
        JOIN_QUERY
    } else if mode.ends_with("-path") {
        PATH_QUERY
    } else {
        QUERY
    };
    let prepared = engine.prepare_query(query, None).unwrap();
    let expected = execute(&mode, &dataset, &engine, &prepared);
    assert_eq!(expected.0, SOURCES * rows);
    let window = WholeProcessWindow::open();
    let actual = execute(&mode, &dataset, &engine, &prepared);
    let measured = window.close();
    assert_eq!(actual, expected);
    println!(
        "mode={mode} sources={SOURCES} rows={} checksum={} allocations={} requested_bytes={} peak_working_bytes={} retained_bytes={} buffering={:?}",
        actual.0,
        actual.1,
        measured.allocations,
        measured.requested_bytes,
        measured.peak_working_bytes,
        measured.retained_bytes,
        actual.2,
    );
}
