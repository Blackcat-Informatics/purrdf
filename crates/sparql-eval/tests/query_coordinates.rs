// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact caller inputs at the real public query boundary, without host reads.

mod support;

use purrdf_core::DatasetView as _;
use purrdf_core::{
    RdfDatasetBuilder, SegmentedEvidence, SparqlRequest, SparqlResult, StopCause, TermValue,
    TrippedGovernor,
};
use purrdf_sparql_eval::{
    CancellationFlag, GovernedOutcome, NativeSparqlEngine, QueryCoordinates, QueryGovernors,
    QueryOptions,
};
use purrdf_testkit::harness::without_host_clock_or_entropy;
use purrdf_xsd::{XsdDatatype, XsdValue};

#[global_allocator]
static GLOBAL: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;

fn supplied_inputs_repeat_complete_results_without_host_reads() {
    let dataset = RdfDatasetBuilder::new().freeze().unwrap();
    let engine = NativeSparqlEngine::new();
    let XsdValue::DateTime(now) =
        purrdf_xsd::parse("1969-12-31T23:59:58.125Z", XsdDatatype::DateTime).unwrap()
    else {
        panic!("dateTime value")
    };
    let options = QueryOptions::new().with_now(&now).with_entropy_seed(7);
    let evaluate = || {
        engine
            .query_with_options_view(
                &dataset,
                SparqlRequest {
                    query: "SELECT (NOW() AS ?time) (RAND() AS ?random) (UUID() AS ?uuid) (STRUUID() AS ?text_uuid) WHERE {}",
                    base_iri: None,
                    substitutions: &[],
                },
                options,
            )
            .unwrap()
    };
    let first = without_host_clock_or_entropy(evaluate);
    let second = without_host_clock_or_entropy(evaluate);
    let SparqlResult::Solutions {
        variables, rows, ..
    } = first
    else {
        panic!("SELECT result")
    };
    let SparqlResult::Solutions {
        variables: repeated_variables,
        rows: repeated_rows,
        ..
    } = second
    else {
        panic!("repeated SELECT result")
    };
    assert_eq!(variables, ["time", "random", "uuid", "text_uuid"]);
    assert_eq!(variables, repeated_variables);
    assert_eq!(rows, repeated_rows);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].len(), 4);
    assert_eq!(
        rows[0][0],
        Some(TermValue::typed_literal(
            now.canonical_lexical(),
            XsdDatatype::DateTime.iri(),
        ))
    );
    assert!(rows[0][1..].iter().all(Option::is_some));
}

/// Sampled defaults are sufficient to replay results; identity also binds program and source.
fn sampled_coordinates_replay_and_bind_query_and_snapshot() {
    let engine = NativeSparqlEngine::new();
    let mut dataset = RdfDatasetBuilder::new().freeze().unwrap();
    let query = "SELECT (NOW() AS ?time) (RAND() AS ?random) (UUID() AS ?uuid) WHERE {}";
    let request = SparqlRequest {
        query,
        base_iri: None,
        substitutions: &[],
    };
    let sampled = engine
        .query_governed_view(
            &dataset,
            request,
            QueryOptions::EMPTY,
            &QueryGovernors::METERED,
        )
        .unwrap();
    let QueryCoordinates::Started(actual) = sampled.coordinates().clone() else {
        panic!("sampled context")
    };
    assert!(actual.now().is_none() && actual.entropy_seed().is_none());
    assert!(actual.execution_identity().is_some());
    let options = QueryOptions::new()
        .with_now(actual.effective_now())
        .with_entropy_seed(actual.effective_entropy_seed());
    let replay = without_host_clock_or_entropy(|| {
        engine.query_governed_view(&dataset, request, options, &QueryGovernors::METERED)
    })
    .unwrap();
    let QueryCoordinates::Started(replayed) = replay.coordinates() else {
        panic!("replayed context")
    };
    assert_eq!(actual.execution_identity(), replayed.execution_identity());
    let SparqlResult::Solutions { rows: expected, .. } = sampled.into_complete().unwrap() else {
        panic!("SELECT")
    };
    let SparqlResult::Solutions { rows, .. } = replay.into_complete().unwrap() else {
        panic!("SELECT")
    };
    assert_eq!(rows, expected);
    let changed_query = engine
        .query_governed_view(
            &dataset,
            SparqlRequest {
                query: "SELECT (NOW() AS ?time) (RAND() AS ?random) WHERE {}",
                ..request
            },
            options,
            &QueryGovernors::METERED,
        )
        .unwrap();
    let QueryCoordinates::Started(changed) = changed_query.coordinates() else {
        panic!("changed query")
    };
    assert_ne!(actual.execution_identity(), changed.execution_identity());
    assert_eq!(actual.snapshot_identity(), changed.snapshot_identity());
    engine
        .update_with_options(
            &mut dataset,
            SparqlRequest {
                query: "INSERT DATA { <https://example.org/s> <https://example.org/p> 1 }",
                ..request
            },
            options,
        )
        .unwrap();
    let changed_source = engine
        .query_governed_view(&dataset, request, options, &QueryGovernors::METERED)
        .unwrap();
    let QueryCoordinates::Started(changed) = changed_source.coordinates() else {
        panic!("changed source")
    };
    assert_ne!(actual.snapshot_identity(), changed.snapshot_identity());
    assert_ne!(actual.execution_identity(), changed.execution_identity());
}

fn supplied_large_fraction_and_timezone_survive_repeated_now() {
    let dataset = RdfDatasetBuilder::new().freeze().unwrap();
    let engine = NativeSparqlEngine::new();
    let lexical = format!("2024-02-29T12:34:56.{}+05:30", "7".repeat(4096));
    let XsdValue::DateTime(now) = purrdf_xsd::parse(&lexical, XsdDatatype::DateTime).unwrap()
    else {
        panic!("dateTime value")
    };
    let expected = TermValue::typed_literal(now.canonical_lexical(), XsdDatatype::DateTime.iri());
    let result = without_host_clock_or_entropy(|| {
        engine
            .query_with_options_view(
                &dataset,
                SparqlRequest {
                    query: "SELECT (NOW() AS ?a) ?b WHERE {
                        { SELECT (NOW() AS ?b) WHERE {} }
                        FILTER EXISTS {
                            BIND(NOW() AS ?inside)
                            FILTER(?inside = ?b)
                        }
                    }",
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions::new().with_now(&now).with_entropy_seed(7),
            )
            .unwrap()
    });
    let SparqlResult::Solutions {
        variables, rows, ..
    } = result
    else {
        panic!("SELECT result")
    };
    assert_eq!(variables, ["a", "b"]);
    assert_eq!(rows, [vec![Some(expected.clone()), Some(expected)]]);
}

fn prepared_executions_select_fresh_inputs_without_changing_the_plan() {
    let dataset = RdfDatasetBuilder::new().freeze().unwrap();
    let engine = NativeSparqlEngine::new();
    let XsdValue::DateTime(first_time) =
        purrdf_xsd::parse("1969-12-31T23:59:58.125Z", XsdDatatype::DateTime).unwrap()
    else {
        panic!("first dateTime value")
    };
    let XsdValue::DateTime(second_time) =
        purrdf_xsd::parse("2024-02-29T12:34:56.75+05:30", XsdDatatype::DateTime).unwrap()
    else {
        panic!("second dateTime value")
    };
    let prepared = without_host_clock_or_entropy(|| {
        engine
            .prepare_query(
                "SELECT (NOW() AS ?time) (RAND() AS ?random) (UUID() AS ?uuid) (STRUUID() AS ?text_uuid) WHERE {}",
                None,
            )
            .unwrap()
    });
    let execute = |now, seed, governed| {
        let result = without_host_clock_or_entropy(|| {
            if governed {
                let outcome = engine
                    .query_prepared_governed_view(
                        &dataset,
                        &prepared,
                        &[],
                        QueryOptions::new().with_now(now).with_entropy_seed(seed),
                        &QueryGovernors::METERED,
                    )
                    .unwrap();
                let GovernedOutcome::Complete { result, .. } = outcome else {
                    panic!("complete governed prepared result")
                };
                return result;
            }
            engine
                .query_prepared_view(
                    &dataset,
                    &prepared,
                    &[],
                    QueryOptions::new().with_now(now).with_entropy_seed(seed),
                )
                .unwrap()
        });
        let SparqlResult::Solutions {
            variables, rows, ..
        } = result
        else {
            panic!("prepared SELECT result")
        };
        assert_eq!(variables, ["time", "random", "uuid", "text_uuid"]);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].len(), 4);
        assert!(rows[0].iter().all(Option::is_some));
        rows.into_iter().next().unwrap()
    };
    let ordinary = execute(&first_time, 7, false);
    for governed in [false, true] {
        let first = execute(&first_time, 7, governed);
        assert_eq!(first, ordinary);
        assert_eq!(first, execute(&first_time, 7, governed));
        let changed_time = execute(&second_time, 7, governed);
        assert_eq!(
            changed_time[0],
            Some(TermValue::typed_literal(
                second_time.canonical_lexical(),
                XsdDatatype::DateTime.iri(),
            ))
        );
        assert_ne!(first[0], changed_time[0]);
        assert_eq!(first[1..], changed_time[1..]);
        let changed_seed = execute(&first_time, 8, governed);
        assert_eq!(first[0], changed_seed[0]);
        for column in 1..4 {
            assert_ne!(first[column], changed_seed[column]);
        }
        assert_eq!(first, execute(&first_time, 7, governed));
    }
}

fn bounded_coordinate_owners_price_real_allocations_and_release_after_last_output() {
    let (image, _) = support::segmented::fixture();
    let source = support::segmented::open(&image, support::segmented::CEILING);
    let engine = NativeSparqlEngine::new();
    let prepared = engine
        .prepare_query("SELECT (NOW() AS ?time) WHERE {}", None)
        .unwrap();
    let lexical = format!("2024-02-29T12:34:56.{}+05:30", "7".repeat(4096));
    let XsdValue::DateTime(now) = purrdf_xsd::parse(&lexical, XsdDatatype::DateTime).unwrap()
    else {
        panic!("dateTime value")
    };
    let expected = TermValue::typed_literal(now.canonical_lexical(), XsdDatatype::DateTime.iri());
    // These reader buffers outlive results. Pinned blocks and original query
    // grants remain in the compared balance and must all be released.
    let owner_balance = |evidence: &SegmentedEvidence| {
        evidence.live_bytes() - evidence.unpinned_cache_bytes() - evidence.bookkeeping_bytes()
    };
    let before = owner_balance(&source.evidence());
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let found = without_host_clock_or_entropy(|| {
        engine
            .query_prepared_governed_fallible_view(
                &source,
                &prepared,
                &[],
                QueryOptions::new().with_now(&now).with_entropy_seed(7),
                &QueryGovernors::METERED,
            )
            .unwrap()
            .result
    });
    let measurement = window.close();
    assert!(
        u64::try_from(measurement.peak_working_bytes).unwrap() <= source.evidence().peak_bytes()
    );
    assert!(source.evidence().peak_bytes() <= support::segmented::CEILING);
    assert!(source.read_error().is_none());
    drop(now);
    drop(prepared);
    drop(engine);
    let (variables, rows) = found.solutions().unwrap();
    assert_eq!(variables, ["time"]);
    assert_eq!(rows, [vec![Some(expected.clone())]]);
    let original_live = source.evidence().live_bytes();
    let original_owners = owner_balance(&source.evidence());
    assert!(original_owners > before);
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let retained = found.clone();
    drop(found);
    let measurement = window.close();
    assert_eq!(measurement.allocations, 0);
    assert_eq!(measurement.requested_bytes, 0);
    assert_eq!(measurement.retained_bytes, 0);
    assert_eq!(measurement.peak_working_bytes, 0);
    assert_eq!(source.evidence().live_bytes(), original_live);
    assert_eq!(owner_balance(&source.evidence()), original_owners);
    assert_eq!(retained.solutions().unwrap().1, [vec![Some(expected)]]);
    drop(retained);
    assert_eq!(
        owner_balance(&source.evidence()),
        before,
        "the last output releases its grant after its engine and plan are gone: {:?}",
        source.evidence()
    );

    let engine = NativeSparqlEngine::new();
    let prepared = engine
        .prepare_query("SELECT (NOW() AS ?time) WHERE {}", None)
        .unwrap();
    let XsdValue::DateTime(now) = purrdf_xsd::parse(&lexical, XsdDatatype::DateTime).unwrap()
    else {
        panic!("dateTime value for cancellation")
    };
    let stop = std::sync::Arc::new(CancellationFlag::new());
    stop.cancel();
    let governors = QueryGovernors::METERED.with_stop_signal(stop);
    let refused = without_host_clock_or_entropy(|| {
        engine
            .query_prepared_governed_fallible_view(
                &source,
                &prepared,
                &[],
                QueryOptions::new().with_now(&now).with_entropy_seed(7),
                &governors,
            )
            .unwrap_err()
    });
    assert_eq!(
        refused.tripped(),
        Some(TrippedGovernor::Stopped {
            cause: StopCause::Cancelled,
        })
    );
    assert!(source.read_error().is_none());
    drop(refused);
    drop(prepared);
    drop(engine);
    assert_eq!(owner_balance(&source.evidence()), before);
}

/// UPDATE selects supplied inputs before host sampling and carries one seed
/// stream through every operation, on all three public native entry points.
fn supplied_update_coordinates_replay_and_preserve_atomic_publication() {
    use purrdf_sparql_eval::GovernedUpdateOutcome;
    use std::sync::Arc;

    let engine = NativeSparqlEngine::new();
    let lexical = format!("-0001-02-03T04:05:06.{}-04:00", "7".repeat(4096));
    let now = purrdf_xsd::temporal::parse_datetime(&lexical).unwrap();
    let options = QueryOptions::new().with_now(&now).with_entropy_seed(0);
    let update = ["a", "b"]
        .map(|subject| {
            format!(
                "INSERT {{ <http://example.org/{subject}> <http://example.org/time> ?t ; \
         <http://example.org/random> ?r ; <http://example.org/uuid> ?u ; \
         <http://example.org/str> ?v }} WHERE {{ BIND(NOW() AS ?t) \
         BIND(RAND() AS ?r) BIND(UUID() AS ?u) BIND(STRUUID() AS ?v) }}"
            )
        })
        .join("; ");
    let request = SparqlRequest {
        query: &update,
        base_iri: None,
        substitutions: &[],
    };
    let original = RdfDatasetBuilder::new().freeze().unwrap();
    let read = |dataset: &Arc<purrdf_core::RdfDataset>| {
        let result = engine
            .query_with_options_view(
                dataset,
                SparqlRequest {
                    query: "SELECT ?s ?t ?r ?u ?v WHERE { ?s <http://example.org/time> ?t ; \
             <http://example.org/random> ?r ; <http://example.org/uuid> ?u ; \
             <http://example.org/str> ?v } ORDER BY ?s",
                    ..request
                },
                options,
            )
            .unwrap();
        result.solutions().unwrap().1.to_vec()
    };
    let mut expected = None;
    let check_receipt = |outcome: GovernedUpdateOutcome| {
        assert!(outcome.is_applied());
        let QueryCoordinates::Started(inputs) = outcome.coordinates() else {
            panic!("the first WHERE constructed its context")
        };
        assert!(inputs.now().unwrap().same_representation(&now));
        assert_eq!(inputs.entropy_seed(), Some(0));
    };
    for lane in 0..3 {
        for _ in 0..2 {
            let mut dataset = Arc::clone(&original);
            without_host_clock_or_entropy(|| match lane {
                0 => engine
                    .update_with_options(&mut dataset, request, options)
                    .unwrap(),
                1 => check_receipt(
                    engine
                        .update_governed(&mut dataset, request, options, &QueryGovernors::METERED)
                        .unwrap(),
                ),
                _ => {
                    let mut handle: purrdf_core::DatasetHandle = Arc::clone(&dataset).into();
                    check_receipt(
                        engine
                            .update_governed_handle(
                                &mut handle,
                                request,
                                options,
                                &QueryGovernors::METERED,
                            )
                            .unwrap(),
                    );
                    // Materialize only for inspection; the update has already published.
                    let mut builder = RdfDatasetBuilder::new();
                    builder.push_dataset(handle.as_ref()).unwrap();
                    dataset = builder.freeze().unwrap();
                }
            });
            let rows = read(&dataset);
            assert_eq!(rows.len(), 2);
            let time = Some(TermValue::typed_literal(
                now.canonical_lexical(),
                XsdDatatype::DateTime.iri(),
            ));
            assert_eq!(rows[0][1], time);
            assert_eq!(rows[1][1], time);
            for column in 2..5 {
                assert!(rows[0][column].is_some());
                assert_ne!(
                    rows[0][column], rows[1][column],
                    "stream advances across operations"
                );
            }
            if let Some(expected) = &expected {
                assert_eq!(&rows, expected);
            } else {
                expected = Some(rows);
            }
        }
    }
    let failing = format!(
        "{update}; CREATE GRAPH <http://example.org/g>; CREATE GRAPH <http://example.org/g>"
    );
    for governed in [false, true] {
        let mut dataset = Arc::clone(&original);
        let failed = without_host_clock_or_entropy(|| {
            if governed {
                engine
                    .update_governed(
                        &mut dataset,
                        SparqlRequest {
                            query: &failing,
                            ..request
                        },
                        options,
                        &QueryGovernors::METERED,
                    )
                    .map(|_| ())
            } else {
                engine.update_with_options(
                    &mut dataset,
                    SparqlRequest {
                        query: &failing,
                        ..request
                    },
                    options,
                )
            }
        });
        assert!(failed.is_err());
        assert!(Arc::ptr_eq(&dataset, &original));
    }
    let cancelled = Arc::new(CancellationFlag::new());
    cancelled.cancel();
    let mut dataset = Arc::clone(&original);
    let stopped = without_host_clock_or_entropy(|| {
        engine.update_governed(
            &mut dataset,
            request,
            options,
            &QueryGovernors::METERED.with_stop_signal(cancelled),
        )
    })
    .unwrap();
    assert_eq!(stopped.coordinates(), &QueryCoordinates::NotStarted);
    assert!(matches!(
        stopped,
        GovernedUpdateOutcome::BudgetExhausted {
            tripped: TrippedGovernor::Stopped {
                cause: StopCause::Cancelled
            },
            ..
        }
    ));
    assert!(Arc::ptr_eq(&dataset, &original));
}

/// Receipt state follows actual WHERE construction, independently of mutation
/// or whether each optional input was supplied.
fn update_receipts_distinguish_contexts_and_independent_inputs() {
    let now = purrdf_xsd::temporal::parse_datetime("-0001-02-03T04:05:06.125-04:00").unwrap();
    for options in [
        QueryOptions::EMPTY,
        QueryOptions::EMPTY.with_now(&now),
        QueryOptions::EMPTY.with_entropy_seed(0),
    ] {
        for (update, started) in [
            (
                "INSERT DATA { <https://example.org/s> <https://example.org/p> 1 }",
                false,
            ),
            (
                "CREATE GRAPH <https://example.org/g>; CLEAR ALL; LOAD SILENT <https://example.org/missing>",
                false,
            ),
            (
                "INSERT { <https://example.org/s> <https://example.org/p> ?o } WHERE { ?s ?p ?o }",
                true,
            ),
            (
                "INSERT { <https://example.org/s> <https://example.org/p> 1 } WHERE {}",
                true,
            ),
        ] {
            let mut dataset = RdfDatasetBuilder::new().freeze().unwrap();
            let outcome = NativeSparqlEngine::new()
                .update_governed(
                    &mut dataset,
                    SparqlRequest {
                        query: update,
                        base_iri: None,
                        substitutions: &[],
                    },
                    options,
                    &QueryGovernors::METERED,
                )
                .unwrap();
            assert!(outcome.is_applied());
            match outcome.coordinates() {
                QueryCoordinates::NotStarted => assert!(!started),
                QueryCoordinates::Started(inputs) => {
                    assert!(started);
                    assert!(inputs.execution_identity().is_some());
                    assert!(inputs.snapshot_identity().is_some());
                    assert_eq!(inputs.entropy_seed(), options.entropy_seed);
                    assert_eq!(inputs.now().is_some(), options.now.is_some());
                    if let Some(actual) = inputs.now() {
                        assert!(actual.same_representation(&now));
                    }
                }
            }
        }
    }
}

/// UPDATE's resident coordinate owner survives every producer and is physically
/// released by the last receipt; it is not an external source admission claim.
fn update_receipts_release_the_resident_owner_after_the_last_clone() {
    let execute = || {
        let now = purrdf_xsd::temporal::parse_datetime(&format!(
            "-0001-02-03T04:05:06.{}-04:00",
            "7".repeat(4096)
        ))
        .unwrap();
        let mut dataset = RdfDatasetBuilder::new().freeze().unwrap();
        let engine = NativeSparqlEngine::new();
        let outcome = without_host_clock_or_entropy(|| engine.update_governed(
            &mut dataset,
            SparqlRequest {
                query: "INSERT { <https://example.org/s> <https://example.org/p> ?r } WHERE { BIND(RAND() AS ?r) }",
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions::EMPTY.with_now(&now).with_entropy_seed(0),
            &QueryGovernors::METERED,
        )).unwrap();
        let coordinates = outcome.coordinates().clone();
        let QueryCoordinates::Started(inputs) = &coordinates else {
            panic!("WHERE context")
        };
        assert!(inputs.now().unwrap().same_representation(&now));
        assert_eq!(inputs.entropy_seed(), Some(0));
        coordinates
    };
    // Initialize shared runtime machinery outside the measured lifecycle.
    drop(execute());
    for _ in 0..3 {
        let lifecycle = purrdf_alloc_probe::CurrentThreadWindow::open();
        let coordinates = execute();
        let held = lifecycle.close().retained_bytes;
        assert!(held > 0);
        let cloning = purrdf_alloc_probe::CurrentThreadWindow::open();
        let last = coordinates.clone();
        let measured = cloning.close();
        assert_eq!(measured.allocations, 0);
        assert_eq!(measured.requested_bytes, 0);
        drop(coordinates);
        let release = purrdf_alloc_probe::CurrentThreadWindow::open();
        drop(last);
        assert_eq!(release.close().retained_bytes, -held);
    }
}

/// A retained unused blank is observable by the native mint collision law.
fn snapshot_identity_binds_unused_blank_mint_reservations() {
    let empty = RdfDatasetBuilder::new().freeze().unwrap();
    let mut builder = RdfDatasetBuilder::new();
    builder
        .intern_blank("bnode1", purrdf_core::BlankScope::DEFAULT)
        .unwrap();
    let reserved = builder.freeze().unwrap();
    assert_eq!(empty.quad_count(), reserved.quad_count());
    let XsdValue::DateTime(now) =
        purrdf_xsd::parse("2024-01-01T00:00:00Z", XsdDatatype::DateTime).unwrap()
    else {
        panic!("dateTime")
    };
    let options = QueryOptions::EMPTY.with_now(&now).with_entropy_seed(0);
    let engine = NativeSparqlEngine::new();
    let run = |dataset: &std::sync::Arc<purrdf_core::RdfDataset>| {
        let outcome = engine
            .query_governed_view(
                dataset,
                SparqlRequest {
                    query: "SELECT (BNODE() AS ?blank) WHERE {}",
                    base_iri: None,
                    substitutions: &[],
                },
                options,
                &QueryGovernors::METERED,
            )
            .unwrap();
        let QueryCoordinates::Started(coordinates) = outcome.coordinates().clone() else {
            panic!("started context")
        };
        (
            coordinates,
            support::solutions(outcome.into_complete().unwrap()),
        )
    };
    let (ordinary, ordinary_result) = run(&empty);
    let (collision, collision_result) = run(&reserved);
    assert_ne!(ordinary_result, collision_result);
    assert_ne!(ordinary.snapshot_identity(), collision.snapshot_identity());
    assert_ne!(
        ordinary.execution_identity(),
        collision.execution_identity()
    );
}

purrdf_testkit::harness_main!(
    snapshot_identity_binds_unused_blank_mint_reservations,
    supplied_inputs_repeat_complete_results_without_host_reads,
    sampled_coordinates_replay_and_bind_query_and_snapshot,
    supplied_large_fraction_and_timezone_survive_repeated_now,
    prepared_executions_select_fresh_inputs_without_changing_the_plan,
    bounded_coordinate_owners_price_real_allocations_and_release_after_last_output,
    supplied_update_coordinates_replay_and_preserve_atomic_publication,
    update_receipts_distinguish_contexts_and_independent_inputs,
    update_receipts_release_the_resident_owner_after_the_last_clone,
);
