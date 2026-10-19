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
    CancellationFlag, GovernedOutcome, NativeSparqlEngine, QueryGovernors, QueryOptions,
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

purrdf_testkit::harness_main!(
    supplied_inputs_repeat_complete_results_without_host_reads,
    supplied_large_fraction_and_timezone_survive_repeated_now,
    prepared_executions_select_fresh_inputs_without_changing_the_plan,
    bounded_coordinate_owners_price_real_allocations_and_release_after_last_output,
);
