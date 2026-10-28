// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact coordinates cross every exported query wrapper; results own their values.

use std::ffi::CStr;
use std::ptr::{null, null_mut};

use super::*;
use crate::error::{purrdf_error_free, purrdf_error_message};

const NOW: &str = "2020-01-02T03:04:05.12345678901234567890123456789+05:30";
const OTHER_NOW: &str = "-0001-02-03T04:05:06.98765432109876543210987654321-04:00";

struct Answer {
    rows: *mut PurrdfRowCursor,
    buffer: *mut PurrdfBuffer,
}

impl Drop for Answer {
    fn drop(&mut self) {
        unsafe {
            purrdf_rowcursor_free(self.rows);
            purrdf_buffer_free(self.buffer);
        }
    }
}

impl Answer {
    fn values(mut self) -> Vec<String> {
        unsafe {
            if !self.buffer.is_null() {
                let text = take_buffer_text(std::mem::replace(&mut self.buffer, null_mut()));
                let document = purrdf_lex::json::read(&text).expect("results JSON");
                return ["clock", "random", "uuid", "text"]
                    .map(|name| {
                        document
                            .pointer(&format!("/results/bindings/0/{name}/value"))
                            .and_then(purrdf_lex::json::Value::as_str)
                            .expect("bound coordinate")
                            .to_owned()
                    })
                    .to_vec();
            }
            assert_eq!(purrdf_rowcursor_next(self.rows), PurrdfStatus::Ok as i32);
            let values = (0..4)
                .map(|column| {
                    let mut term = PurrdfTermView::empty();
                    let mut bound = 0;
                    assert_eq!(
                        purrdf_rowcursor_term(self.rows, column, &raw mut term, &raw mut bound),
                        0
                    );
                    assert_eq!(bound, 1);
                    term.lexical.as_str().expect("term UTF-8").to_owned()
                })
                .collect();
            assert_eq!(
                purrdf_rowcursor_next(self.rows),
                PurrdfStatus::CursorExhausted as i32
            );
            values
        }
    }
}

/// Call a real exported wrapper; the dataset and query text die before result inspection.
fn execute(route: usize, now: Option<&CStr>, seed: Option<&u64>) -> Result<Answer, (i32, String)> {
    let dataset = entailment_dataset();
    let pattern = if route >= 9 {
        "?s a <http://example.org/Animal>"
    } else {
        ""
    };
    let query = CString::new(format!(
        "SELECT (NOW() AS ?clock) (RAND() AS ?random) (UUID() AS ?uuid) (STRUUID() AS ?text) WHERE {{ {pattern} }}"
    )).expect("query C string");
    let governors = initialized_governors();
    let now = now.map_or(null(), CStr::as_ptr);
    let entropy_seed = seed.map_or(null(), std::ptr::from_ref);
    let mut answer = Answer {
        rows: null_mut(),
        buffer: null_mut(),
    };
    let mut kind = 55;
    let mut outcome = -1;
    let mut partial = PurrdfPartialCertificate {
        kind: -1,
        positional_prefix: 0,
        barrier: PurrdfStr::empty(),
    };
    let mut evidence = MaybeUninit::uninit();
    let mut entailment_evidence = MaybeUninit::uninit();
    let mut report = null_mut();
    let mut error = null_mut();
    let status = unsafe {
        match route {
            0 => purrdf_query(
                dataset,
                query.as_ptr(),
                null(),
                now,
                entropy_seed,
                &mut crate::receipt::Output::default().0,
                &raw mut kind,
                &raw mut answer.rows,
                null_mut(),
                null_mut(),
                &raw mut error,
            ),
            1 => purrdf_query_xpath_regex(
                dataset,
                query.as_ptr(),
                null(),
                c"xpath-3.1-2017-03-21".as_ptr(),
                now,
                entropy_seed,
                &mut crate::receipt::Output::default().0,
                &raw mut kind,
                &raw mut answer.rows,
                null_mut(),
                null_mut(),
                &raw mut error,
            ),
            2 => purrdf_query_with_geo(
                null(),
                dataset,
                query.as_ptr(),
                null(),
                c"xpath-3.1-2017-03-21".as_ptr(),
                now,
                entropy_seed,
                &mut crate::receipt::Output::default().0,
                &raw mut kind,
                &raw mut answer.rows,
                null_mut(),
                null_mut(),
                &raw mut error,
            ),
            3 => purrdf_query_json(
                dataset,
                query.as_ptr(),
                null(),
                null(),
                null(),
                now,
                entropy_seed,
                &mut crate::receipt::Output::default().0,
                &raw mut answer.buffer,
                &raw mut error,
            ),
            4 => purrdf_query_json_xpath_regex(
                dataset,
                query.as_ptr(),
                null(),
                null(),
                null(),
                c"xpath-3.1-2017-03-21".as_ptr(),
                now,
                entropy_seed,
                &mut crate::receipt::Output::default().0,
                &raw mut answer.buffer,
                &raw mut error,
            ),
            5 => purrdf_query_json_with_geo(
                null(),
                dataset,
                query.as_ptr(),
                null(),
                null(),
                null(),
                c"xpath-3.1-2017-03-21".as_ptr(),
                now,
                entropy_seed,
                &mut crate::receipt::Output::default().0,
                &raw mut answer.buffer,
                &raw mut error,
            ),
            6 => purrdf_query_governed(
                dataset,
                query.as_ptr(),
                null(),
                null(),
                &raw const governors,
                now,
                entropy_seed,
                &mut crate::receipt::Output::default().0,
                &raw mut outcome,
                &raw mut kind,
                &raw mut answer.rows,
                null_mut(),
                null_mut(),
                evidence.as_mut_ptr(),
                &raw mut partial,
                &raw mut error,
            ),
            7 => purrdf_query_governed_xpath_regex(
                dataset,
                query.as_ptr(),
                null(),
                null(),
                c"xpath-3.1-2017-03-21".as_ptr(),
                &raw const governors,
                now,
                entropy_seed,
                &mut crate::receipt::Output::default().0,
                &raw mut outcome,
                &raw mut kind,
                &raw mut answer.rows,
                null_mut(),
                null_mut(),
                evidence.as_mut_ptr(),
                &raw mut partial,
                &raw mut error,
            ),
            8 => purrdf_query_governed_with_geo(
                null(),
                dataset,
                query.as_ptr(),
                null(),
                null(),
                c"xpath-3.1-2017-03-21".as_ptr(),
                &raw const governors,
                now,
                entropy_seed,
                &mut crate::receipt::Output::default().0,
                &raw mut outcome,
                &raw mut kind,
                &raw mut answer.rows,
                null_mut(),
                null_mut(),
                evidence.as_mut_ptr(),
                &raw mut partial,
                &raw mut error,
            ),
            9 => purrdf_query_entailment_governed(
                dataset,
                query.as_ptr(),
                null(),
                c"rdfs".as_ptr(),
                c"".as_ptr(),
                null(),
                null(),
                0,
                null(),
                0,
                null(),
                null(),
                null(),
                null(),
                &raw const governors,
                now,
                entropy_seed,
                &mut crate::receipt::Output::default().0,
                &raw mut outcome,
                &raw mut kind,
                &raw mut answer.rows,
                null_mut(),
                null_mut(),
                entailment_evidence.as_mut_ptr(),
                &raw mut partial,
                &raw mut report,
                &raw mut error,
            ),
            10 => purrdf_query_entailment_governed_xpath_regex(
                dataset,
                query.as_ptr(),
                null(),
                c"rdfs".as_ptr(),
                c"".as_ptr(),
                null(),
                null(),
                0,
                null(),
                0,
                null(),
                null(),
                null(),
                null(),
                c"xpath-3.1-2017-03-21".as_ptr(),
                &raw const governors,
                now,
                entropy_seed,
                &mut crate::receipt::Output::default().0,
                &raw mut outcome,
                &raw mut kind,
                &raw mut answer.rows,
                null_mut(),
                null_mut(),
                entailment_evidence.as_mut_ptr(),
                &raw mut partial,
                &raw mut report,
                &raw mut error,
            ),
            11 => purrdf_query_entailment_governed_with_geo(
                null(),
                dataset,
                query.as_ptr(),
                null(),
                c"rdfs".as_ptr(),
                c"".as_ptr(),
                null(),
                null(),
                0,
                null(),
                0,
                null(),
                null(),
                null(),
                null(),
                c"xpath-3.1-2017-03-21".as_ptr(),
                &raw const governors,
                now,
                entropy_seed,
                &mut crate::receipt::Output::default().0,
                &raw mut outcome,
                &raw mut kind,
                &raw mut answer.rows,
                null_mut(),
                null_mut(),
                entailment_evidence.as_mut_ptr(),
                &raw mut partial,
                &raw mut report,
                &raw mut error,
            ),
            _ => unreachable!("twelve query wrappers"),
        }
    };
    unsafe {
        purrdf_dataset_free(dataset);
    }
    if status != PurrdfStatus::Ok as i32 {
        assert!(!error.is_null());
        assert!(answer.rows.is_null() && answer.buffer.is_null() && report.is_null());
        if route >= 6 {
            assert_eq!(kind, KIND_NONE);
            assert_eq!(partial.kind, PurrdfPartialKind::None as i32);
        }
        let message = unsafe { CStr::from_ptr(purrdf_error_message(error)) }
            .to_str()
            .expect("error UTF-8")
            .to_owned();
        unsafe {
            purrdf_error_free(error);
        }
        return Err((status, message));
    }
    assert!(error.is_null());
    if route >= 6 {
        assert_eq!(outcome, 0);
        assert_eq!(partial.kind, PurrdfPartialKind::None as i32);
        if route >= 9 {
            assert_eq!(unsafe { entailment_evidence.assume_init() }.query_ran, 1);
            assert!(!report.is_null());
            assert!(take_buffer_text(report).contains("\nregime rdfs\n"));
        }
    }
    Ok(answer)
}

#[test]
fn every_exported_query_wrapper_forwards_supplied_coordinates() {
    let now = CString::new(NOW).expect("time C string");
    let expected = execute(0, Some(&now), Some(&0))
        .expect("ordinary result")
        .values();
    assert_eq!(expected[0], NOW);
    assert!(expected[2].starts_with("urn:uuid:"));
    for route in 0..12 {
        let owned_time = CString::new(NOW).expect("time C string");
        let seed = Box::new(0);
        let answer = execute(route, Some(&owned_time), Some(&seed)).expect("wrapper result");
        drop(owned_time);
        drop(seed);
        assert_eq!(answer.values(), expected, "route {route}");
    }
}

#[test]
fn coordinates_are_exact_independent_and_fresh_on_each_call() {
    let now = CString::new(NOW).expect("time C string");
    let other = CString::new(OTHER_NOW).expect("negative year C string");
    let first = execute(0, Some(&now), Some(&0)).expect("first").values();
    let changed = execute(0, Some(&other), Some(&0))
        .expect("different time")
        .values();
    assert_eq!(changed[0], OTHER_NOW);
    assert_eq!(changed[1..], first[1..]);
    let changed = execute(0, Some(&now), Some(&u64::MAX))
        .expect("different seed")
        .values();
    assert_eq!(changed[0], NOW);
    assert!(changed[1..].iter().zip(&first[1..]).all(|(a, b)| a != b));
    let omitted = execute(0, None, Some(&0)).expect("native clock").values();
    assert_ne!(omitted[0], NOW);
    assert_eq!(omitted[1..], first[1..]);
    assert_eq!(
        execute(0, Some(&now), None)
            .expect("native entropy")
            .values()[0],
        NOW
    );
    assert_ne!(
        execute(0, None, None).expect("native defaults").values()[0],
        NOW
    );
}

#[test]
fn coordinate_refusals_cross_c_with_governed_outputs_cleared() {
    let invalid_utf8 = c"\xff";
    for route in [0, 3, 6, 9] {
        for (time, expected) in [
            (c"invalid", PurrdfStatus::InvalidArgument),
            (invalid_utf8, PurrdfStatus::InvalidUtf8),
        ] {
            let Err((status, message)) = execute(route, Some(time), Some(&0)) else {
                panic!("bad time accepted");
            };
            assert_eq!(status, expected as i32);
            assert_ne!(message, "");
        }
        let now = CString::new(NOW).expect("valid time");
        assert_eq!(
            execute(route, Some(&now), Some(&0))
                .expect("recovery")
                .values()[0],
            NOW
        );
    }
}
