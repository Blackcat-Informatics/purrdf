// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Governed UPDATE owns exact coordinates and publishes only complete requests.

use std::ffi::CStr;
use std::ptr::{null, null_mut};

use super::*;
use crate::error::purrdf_error_free;

const NOW: &str = "2020-01-02T03:04:05.12345678901234567890123456789+05:30";
const BCE: &str = "-0001-02-03T04:05:06.98765432109876543210987654321-04:00";
const REQUEST: &CStr = c"INSERT { <urn:first> <urn:clock> ?clock; <urn:random> ?random } WHERE { BIND(NOW() AS ?clock) BIND(RAND() AS ?random) }; INSERT { <urn:second> <urn:clock> ?clock; <urn:random> ?random } WHERE { BIND(NOW() AS ?clock) BIND(RAND() AS ?random) }";

fn apply(
    route: usize,
    dataset: *mut PurrdfDataset,
    request: &CStr,
    now: Option<&CStr>,
    seed: Option<&u64>,
    graph: i32,
) -> i32 {
    let governors = initialized_governors();
    let now = now.map_or(null(), CStr::as_ptr);
    let seed = seed.map_or(null(), std::ptr::from_ref);
    let mut outcome = -1;
    let mut evidence = PurrdfGovernorEvidence::EMPTY;
    evidence.consumed.fuel = 987;
    let sentinel = evidence;
    let before = unsafe { (*dataset).0.clone() };
    let mut error = null_mut();
    let status = unsafe {
        match route {
            0 => purrdf_update_governed(
                dataset,
                request.as_ptr(),
                null(),
                null(),
                graph,
                &raw const governors,
                now,
                seed,
                &mut crate::receipt::Output::default().0,
                &raw mut outcome,
                &raw mut evidence,
                &raw mut error,
            ),
            1 => purrdf_update_governed_xpath_regex(
                dataset,
                request.as_ptr(),
                null(),
                null(),
                c"xpath-3.1-2017-03-21".as_ptr(),
                graph,
                &raw const governors,
                now,
                seed,
                &mut crate::receipt::Output::default().0,
                &raw mut outcome,
                &raw mut evidence,
                &raw mut error,
            ),
            2 => purrdf_update_governed_with_geo(
                null(),
                dataset,
                request.as_ptr(),
                null(),
                null(),
                c"xpath-3.1-2017-03-21".as_ptr(),
                graph,
                &raw const governors,
                now,
                seed,
                &mut crate::receipt::Output::default().0,
                &raw mut outcome,
                &raw mut evidence,
                &raw mut error,
            ),
            _ => unreachable!("three exported UPDATE wrappers"),
        }
    };
    if status == PurrdfStatus::Ok as i32 {
        assert!(error.is_null());
        assert_eq!(outcome, PurrdfUpdateOutcomeKind::Applied as i32);
        assert_ne!(evidence, sentinel);
    } else {
        assert!(!error.is_null());
        assert_eq!(outcome, -1);
        assert_eq!(evidence, sentinel);
        assert!(unsafe { std::ptr::eq(before.as_ref(), (*dataset).0.as_ref()) });
    }
    unsafe { purrdf_error_free(error) };
    status
}

fn inserted(route: usize, time: Option<&str>, seed: Option<u64>) -> Vec<(String, String)> {
    let dataset = query_dataset();
    {
        let time = time.map(|text| CString::new(text).expect("time C string"));
        let seed = seed.map(Box::new);
        assert_eq!(
            apply(
                route,
                dataset,
                REQUEST,
                time.as_deref(),
                seed.as_deref(),
                crate::graph::PURRDF_GRAPH_EXISTENCE_IMPLICIT
            ),
            PurrdfStatus::Ok as i32
        );
    }
    // Caller-owned coordinate storage has gone before inserted values are inspected.
    let mut buffer = null_mut();
    let mut error = null_mut();
    assert_eq!(
        unsafe {
            purrdf_query_json(dataset,
        c"SELECT ?clock ?random WHERE { ?s <urn:clock> ?clock; <urn:random> ?random } ORDER BY ?s".as_ptr(),
        null(), null(), null(), null(), null(), &mut crate::receipt::Output::default().0, &raw mut buffer, &raw mut error)
        },
        0
    );
    assert!(error.is_null());
    unsafe { purrdf_dataset_free(dataset) };
    let text = take_buffer_text(buffer);
    let document = purrdf_lex::json::read(&text).expect("results JSON");
    (0..2)
        .map(|row| {
            let value = |name| {
                document
                    .pointer(&format!("/results/bindings/{row}/{name}/value"))
                    .and_then(purrdf_lex::json::Value::as_str)
                    .expect("inserted value")
                    .to_owned()
            };
            (value("clock"), value("random"))
        })
        .collect()
}

#[test]
fn every_update_wrapper_owns_exact_time_and_a_progressing_stream() {
    for route in 0..3 {
        for (time, seed) in [(NOW, 0), (BCE, u64::MAX)] {
            let values = inserted(route, Some(time), Some(seed));
            assert_eq!(values, inserted(route, Some(time), Some(seed)));
            assert_eq!(values[0].0, time);
            assert_eq!(values[1].0, time);
            assert_ne!(values[0].1, values[1].1);
        }
        let seeded = inserted(route, None, Some(0));
        assert_ne!(seeded[0].0, NOW);
        assert_eq!(seeded[0].1, inserted(route, Some(NOW), Some(0))[0].1);
        let timed = inserted(route, Some(NOW), None);
        assert_eq!(timed[0].0, NOW);
        assert_eq!(timed[1].0, NOW);
        assert_ne!(timed[0].1, inserted(route, Some(NOW), None)[0].1);
        let defaults = inserted(route, None, None);
        assert_ne!(defaults[0].0, NOW);
    }
}

#[test]
fn update_coordinate_refusals_and_late_errors_preserve_snapshot_and_outputs() {
    for route in 0..3 {
        let dataset = query_dataset();
        let graph = crate::graph::PURRDF_GRAPH_EXISTENCE_IMPLICIT;
        for (time, status) in [
            (c"not-a-time", PurrdfStatus::InvalidArgument),
            (c"\xff", PurrdfStatus::InvalidUtf8),
        ] {
            assert_eq!(
                apply(route, dataset, REQUEST, Some(time), Some(&0), graph),
                status as i32
            );
        }
        assert_eq!(
            apply(route, dataset, REQUEST, Some(c"\xff"), None, -1),
            PurrdfStatus::InvalidArgument as i32,
            "graph configuration precedes coordinates"
        );
        let late = c"INSERT DATA { <urn:before> <urn:p> <urn:o> }; CREATE GRAPH <urn:existing>; CREATE GRAPH <urn:existing>";
        assert_ne!(
            apply(
                route,
                dataset,
                late,
                Some(c"2020-01-02T03:04:05Z"),
                Some(&0),
                crate::graph::PURRDF_GRAPH_EXISTENCE_REMEMBER_EMPTY
            ),
            0
        );
        unsafe { purrdf_dataset_free(dataset) };
    }
}
